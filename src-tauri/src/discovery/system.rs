//! System facts, kept separate from per-Agent requirements. A missing tool is only a problem when
//! an applicable Agent capability requires it; that decision belongs to Health.

use super::{find_candidates, homebrew_path_version, is_script, npm_package_version, version_probe};
use crate::core::Instant;
use crate::domain::*;
use crate::platform::{HostContext, Os};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolObservation {
    pub key: &'static str,
    pub name: &'static str,
    pub present: bool,
    pub location: Option<String>,
    pub version: Observed<String>,
    pub candidates: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathEntry {
    pub location: String,
    pub exists: bool,
    pub on_path: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    pub observed_at: Instant,
    pub os: Os,
    pub os_version: Observed<String>,
    pub arch: &'static str,
    pub shell: Option<String>,
    pub tools: Vec<ToolObservation>,
    pub path_entries: Vec<PathEntry>,
    pub data_dir: String,
    pub data_dir_writable: bool,
    /// PATH Nexus inherited from the desktop session; differs from interactive shells on macOS.
    pub gui_path_note: bool,
}

pub fn discover_system(ctx: &HostContext, now: Instant) -> SystemSnapshot {
    let tools = vec![
        tool(ctx, "node", "Node.js", &["node"], now),
        tool(ctx, "npm", "npm", &["npm"], now),
        tool(ctx, "git", "Git", &["git"], now),
        tool(ctx, "python", "Python", &["python3", "python"], now),
        if ctx.os == Os::Windows { tool(ctx, "winget", "WinGet", &["winget"], now) } else { tool(ctx, "homebrew", "Homebrew", &["brew"], now) },
    ];
    let path_entries = ctx
        .search_dirs()
        .into_iter()
        .map(|(dir, on_path)| PathEntry { location: ctx.display_path(&dir), exists: dir.is_dir(), on_path })
        .collect();
    SystemSnapshot {
        observed_at: now,
        os: ctx.os,
        os_version: os_version(ctx, now),
        arch: ctx.arch,
        shell: ctx.env_value("SHELL").and_then(|s| Path::new(s).file_name().map(|n| n.to_string_lossy().to_string())),
        tools,
        path_entries,
        data_dir: ctx.display_path(&ctx.data_dir),
        data_dir_writable: crate::platform::fs::ensure_private_dir(&ctx.data_dir).is_ok(),
        gui_path_note: ctx.os == Os::Macos,
    }
}

fn tool(ctx: &HostContext, key: &'static str, name: &'static str, exe: &[&str], now: Instant) -> ToolObservation {
    let mut candidates: Vec<_> = find_candidates(ctx, exe).into_iter().filter(|c| !is_unprovisioned_shim(ctx, &c.resolved)).collect();
    candidates.sort_by_key(|c| c.path_rank.unwrap_or(usize::MAX));
    let Some(first) = candidates.first() else {
        return ToolObservation { key, name, present: false, location: None, version: Observed::missing(ObsStatus::NotFound, EvidenceSource::FileMetadata, None, now), candidates: 0 };
    };
    let version = match key {
        "npm" => npm_package_version(ctx, &first.resolved, "npm", now),
        _ => homebrew_path_version(ctx, &first.resolved, now).filter(|_| key == "node"),
    }
    .unwrap_or_else(|| {
        if is_script(&first.resolved) {
            Observed::missing(ObsStatus::NotChecked, EvidenceSource::VersionProbe, Some(ctx.display_path(&first.resolved)), now)
        } else {
            version_probe(ctx, &first.resolved, now)
        }
    });
    ToolObservation { key, name, present: true, location: Some(ctx.display_path(&first.path)), version, candidates: candidates.len() }
}

/// On macOS `/usr/bin/git` and `/usr/bin/python3` are stubs that open an install dialog when the
/// Command Line Tools are absent. Never execute them unless the tools are provisioned.
fn is_unprovisioned_shim(ctx: &HostContext, resolved: &Path) -> bool {
    if ctx.os != Os::Macos {
        return false;
    }
    let stubs = [PathBuf::from("/usr/bin/git"), PathBuf::from("/usr/bin/python3")];
    stubs.iter().any(|s| s == resolved)
        && !Path::new("/Library/Developer/CommandLineTools/usr/bin").is_dir()
        && !Path::new("/Applications/Xcode.app").is_dir()
}

fn os_version(ctx: &HostContext, now: Instant) -> Observed<String> {
    match ctx.os {
        Os::Macos => {
            let plist = Path::new("/System/Library/CoreServices/SystemVersion.plist");
            match crate::platform::fs::read_bounded(plist, 64 * 1024) {
                Ok(read) => {
                    let text = String::from_utf8_lossy(&read.bytes);
                    match plist_string(&text, "ProductVersion") {
                        Some(v) => Observed::ok(v, EvidenceSource::PlatformApi, None, now),
                        None => Observed::missing(ObsStatus::ParseFailed, EvidenceSource::PlatformApi, None, now),
                    }
                }
                Err(_) => Observed::missing(ObsStatus::AccessDenied, EvidenceSource::PlatformApi, None, now),
            }
        }
        // Windows build detection (registry/RtlGetVersion) is not implemented; reported honestly.
        _ => Observed::missing(ObsStatus::NotChecked, EvidenceSource::PlatformApi, None, now),
    }
}

fn plist_string(text: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let after = &text[text.find(&marker)? + marker.len()..];
    let start = after.find("<string>")? + "<string>".len();
    let end = after[start..].find("</string>")?;
    extract_version(&after[start..start + end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_product_version() {
        let text = "<dict><key>ProductName</key><string>macOS</string><key>ProductVersion</key><string>27.2</string></dict>";
        assert_eq!(plist_string(text, "ProductVersion").as_deref(), Some("27.2"));
        assert_eq!(plist_string(text, "Missing"), None);
    }

    #[test]
    fn empty_environment_reports_missing_tools_not_failures() {
        let home = tempfile::tempdir().unwrap();
        let data = home.path().join("data");
        let ctx = HostContext::isolated(home.path(), &data);
        let snap = discover_system(&ctx, Instant(100));
        assert!(snap.tools.iter().all(|t| !t.present && t.version.status == ObsStatus::NotFound));
        assert!(snap.data_dir_writable);
    }
}
