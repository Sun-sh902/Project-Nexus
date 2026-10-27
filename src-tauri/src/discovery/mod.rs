//! Read-only discovery. No function here writes, installs, authenticates, sources shell files or
//! starts MCP servers. The only process execution is the reviewed `--version` probe, gated by
//! `probe_policy`, with a cleared environment, 5 s timeout and 4 KiB output bound.

pub mod system;

use crate::adapters::AgentAdapter;
use crate::core::{sha256_hex, Instant};
use crate::domain::*;
use crate::platform::fs::{is_executable_file, read_bounded, ReadFailure, MAX_CONFIG_BYTES};
use crate::platform::process::{base_env, run, ProcessSpec};
use crate::platform::{HostContext, Os};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const SNAPSHOT_TTL_SECS: i64 = 600;

#[derive(Clone, Debug)]
pub struct Candidate {
    pub path: PathBuf,
    pub resolved: PathBuf,
    pub path_rank: Option<usize>,
}

/// Finds executables named `names` in PATH order, then in well-known directories. Candidates that
/// resolve to the same file are merged (first PATH occurrence wins).
pub fn find_candidates(ctx: &HostContext, names: &[&str]) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let mut rank = 0usize;
    for (dir, on_path) in ctx.search_dirs() {
        for name in names {
            for suffix in ctx
                .os
                .exe_suffixes()
                .iter()
                .chain(if ctx.os == Os::Windows {
                    [".cmd"].iter()
                } else {
                    [].iter()
                })
            {
                let path = dir.join(format!("{name}{suffix}"));
                let Ok(meta) = std::fs::metadata(&path) else {
                    continue;
                };
                if !meta.is_file() {
                    continue;
                }
                let resolved = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if out.iter().any(|c| c.resolved == resolved) {
                    continue;
                }
                out.push(Candidate {
                    path,
                    resolved,
                    path_rank: if on_path { Some(rank) } else { None },
                });
            }
        }
        if on_path {
            rank += 1;
        }
    }
    out
}

pub fn classify_source(ctx: &HostContext, resolved: &Path) -> InstallSource {
    let s = resolved.to_string_lossy().replace('\\', "/");
    if s.contains("/Caskroom/") || s.contains("/Cellar/") {
        InstallSource::Homebrew
    } else if s.contains("/.pnpm/")
        || s.contains("/pnpm/global/")
        || s.contains("/.bun/")
        || s.contains("/.yarn/")
        || s.contains("/yarn/global/")
    {
        // Other Node package managers: Nexus has no vetted recipe, so never treat them as npm.
        InstallSource::Unknown
    } else if s.contains("/lib/node_modules/") || (cfg!(windows) && s.contains("/node_modules/")) {
        InstallSource::Npm
    } else if s.contains("/Microsoft/WinGet/") {
        InstallSource::WinGet
    } else if under(ctx, resolved, ".local/share/claude/versions") {
        InstallSource::NativeInstaller
    } else if under(ctx, resolved, ".opencode") {
        InstallSource::ScriptInstaller
    } else {
        InstallSource::Unknown
    }
}

/// `resolved` is canonical; compare against both the literal and canonical home (macOS tempdirs
/// and some managed homes are reached through symlinks such as /var → /private/var).
fn under(ctx: &HostContext, resolved: &Path, rel: &str) -> bool {
    resolved.starts_with(ctx.home.join(rel))
        || std::fs::canonicalize(&ctx.home)
            .map(|h| resolved.starts_with(h.join(rel)))
            .unwrap_or(false)
}

/// Probe trust policy (Security §3 "Poisoned PATH / version probe"): only absolute regular
/// executables, owned by the current user or root, not group/world-writable, outside temporary
/// directories, and never script wrappers (which would execute an interpreter and its imports).
pub fn probe_policy(ctx: &HostContext, resolved: &Path) -> Result<(), &'static str> {
    if !resolved.is_absolute() || !is_executable_file(resolved) {
        return Err("probe.not_executable");
    }
    let s = resolved.to_string_lossy();
    if s.starts_with("/tmp/")
        || s.starts_with("/private/tmp/")
        || s.starts_with("/var/folders/")
        || s.contains("/Temp/")
    {
        return Err("probe.untrusted_location");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(resolved).map_err(|_| "probe.not_executable")?;
        let uid = unsafe { libc::getuid() };
        if meta.uid() != uid && meta.uid() != 0 {
            return Err("probe.unexpected_owner");
        }
        if meta.mode() & 0o022 != 0 {
            return Err("probe.writable_by_others");
        }
    }
    if is_script(resolved) {
        return Err("probe.script_wrapper");
    }
    let _ = ctx;
    Ok(())
}

pub fn is_script(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 2];
    f.read_exact(&mut head)
        .map(|_| &head == b"#!")
        .unwrap_or(false)
}

pub fn version_probe(ctx: &HostContext, resolved: &Path, now: Instant) -> Observed<String> {
    let location = Some(ctx.display_path(resolved));
    if let Err(_reason) = probe_policy(ctx, resolved) {
        return Observed::missing(
            ObsStatus::Unsafe,
            EvidenceSource::VersionProbe,
            location,
            now,
        );
    }
    let dirs: Vec<PathBuf> = ["/usr/bin", "/bin"].iter().map(PathBuf::from).collect();
    let spec = ProcessSpec {
        program: resolved.to_path_buf(),
        args: vec!["--version".into()],
        env: base_env(&ctx.home, &dirs),
        cwd: ctx.data_dir.clone(),
        timeout: Duration::from_secs(5),
        max_output: 4096,
    };
    match run(&spec) {
        Ok(out) if out.exit == crate::platform::process::ExitKind::TimedOut => Observed::missing(
            ObsStatus::Timeout,
            EvidenceSource::VersionProbe,
            location,
            now,
        ),
        Ok(out) if out.success() => {
            match extract_version(&out.stdout).or_else(|| extract_version(&out.stderr)) {
                Some(v) => Observed::ok(v, EvidenceSource::VersionProbe, location, now),
                None => Observed::missing(
                    ObsStatus::ParseFailed,
                    EvidenceSource::VersionProbe,
                    location,
                    now,
                ),
            }
        }
        Ok(_) => Observed::missing(
            ObsStatus::ParseFailed,
            EvidenceSource::VersionProbe,
            location,
            now,
        ),
        Err(_) => Observed::missing(
            ObsStatus::AccessDenied,
            EvidenceSource::VersionProbe,
            location,
            now,
        ),
    }
}

/// Reads `version` from an npm package manifest, walking up from the resolved entry point to
/// the package root whose `name` equals `package`.
pub fn npm_package_version(
    ctx: &HostContext,
    resolved: &Path,
    package: &str,
    now: Instant,
) -> Option<Observed<String>> {
    let mut dir = resolved.parent();
    for _ in 0..6 {
        let d = dir?;
        let manifest = d.join("package.json");
        if let Ok(read) = read_bounded(&manifest, 1024 * 1024) {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&read.bytes) {
                if v.get("name").and_then(|n| n.as_str()) == Some(package) {
                    let location = Some(ctx.display_path(&manifest));
                    return Some(
                        match v
                            .get("version")
                            .and_then(|n| n.as_str())
                            .and_then(extract_version)
                        {
                            Some(ver) => {
                                Observed::ok(ver, EvidenceSource::PackageManifest, location, now)
                            }
                            None => Observed::missing(
                                ObsStatus::ParseFailed,
                                EvidenceSource::PackageManifest,
                                location,
                                now,
                            ),
                        },
                    );
                }
            }
        }
        dir = d.parent();
    }
    None
}

/// Version from a Homebrew keg path such as `/opt/homebrew/Caskroom/codex/0.164.0/codex`.
pub fn homebrew_path_version(
    ctx: &HostContext,
    resolved: &Path,
    now: Instant,
) -> Option<Observed<String>> {
    let parts: Vec<String> = resolved
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let idx = parts
        .iter()
        .position(|p| p == "Caskroom" || p == "Cellar")?;
    let version = parts.get(idx + 2)?;
    extract_version(version).map(|v| {
        Observed::ok(
            v,
            EvidenceSource::InstallLayout,
            Some(ctx.display_path(resolved)),
            now,
        )
    })
}

pub struct ReadConfig {
    pub source: ConfigSource,
    pub text: Option<String>,
}

pub fn read_config(
    ctx: &HostContext,
    id: &str,
    path: PathBuf,
    scope: ConfigScope,
    format: ConfigFormat,
    tolerant: bool,
) -> ReadConfig {
    let location = ctx.display_path(&path);
    let mut source = ConfigSource {
        id: id.to_string(),
        scope,
        format,
        location,
        status: ObsStatus::NotFound,
        fingerprint: None,
        via_symlink: false,
        tolerant,
        path: path.clone(),
    };
    match read_bounded(&path, MAX_CONFIG_BYTES) {
        Ok(read) => {
            source.fingerprint = Some(read.fingerprint);
            source.via_symlink = read.via_symlink;
            match String::from_utf8(read.bytes) {
                Ok(text) => {
                    let parsed = match format {
                        ConfigFormat::Toml => text.parse::<toml_edit::DocumentMut>().is_ok(),
                        ConfigFormat::Json => crate::config::jsonc::Document::parse(
                            &text,
                            crate::config::jsonc::Dialect::Json,
                        )
                        .is_ok(),
                        ConfigFormat::Jsonc => crate::config::jsonc::Document::parse(
                            &text,
                            crate::config::jsonc::Dialect::Jsonc,
                        )
                        .is_ok(),
                    };
                    source.status = if parsed {
                        ObsStatus::Observed
                    } else {
                        ObsStatus::ParseFailed
                    };
                    ReadConfig {
                        source,
                        text: Some(text),
                    }
                }
                Err(_) => {
                    source.status = ObsStatus::ParseFailed;
                    ReadConfig { source, text: None }
                }
            }
        }
        Err(ReadFailure::NotFound) => ReadConfig { source, text: None },
        Err(ReadFailure::AccessDenied) => {
            source.status = ObsStatus::AccessDenied;
            ReadConfig { source, text: None }
        }
        Err(_) => {
            source.status = ObsStatus::Unsafe;
            ReadConfig { source, text: None }
        }
    }
}

/// Parses a read config into JSON for observation (TOML is converted). `None` when unparseable.
pub fn config_value(read: &ReadConfig) -> Option<serde_json::Value> {
    let text = read.text.as_ref()?;
    if read.source.status != ObsStatus::Observed {
        return None;
    }
    match read.source.format {
        ConfigFormat::Toml => toml_to_json(text),
        ConfigFormat::Json => {
            crate::config::jsonc::Document::parse(text, crate::config::jsonc::Dialect::Json)
                .ok()
                .map(|d| d.value())
        }
        ConfigFormat::Jsonc => {
            crate::config::jsonc::Document::parse(text, crate::config::jsonc::Dialect::Jsonc)
                .ok()
                .map(|d| d.value())
        }
    }
}

pub fn toml_to_json(text: &str) -> Option<serde_json::Value> {
    fn conv(item: &toml_edit::Item) -> serde_json::Value {
        use serde_json::Value as J;
        match item {
            toml_edit::Item::Value(v) => conv_value(v),
            toml_edit::Item::Table(t) => {
                J::Object(t.iter().map(|(k, v)| (k.to_string(), conv(v))).collect())
            }
            toml_edit::Item::ArrayOfTables(a) => J::Array(
                a.iter()
                    .map(|t| J::Object(t.iter().map(|(k, v)| (k.to_string(), conv(v))).collect()))
                    .collect(),
            ),
            toml_edit::Item::None => J::Null,
        }
    }
    fn conv_value(v: &toml_edit::Value) -> serde_json::Value {
        use serde_json::Value as J;
        match v {
            toml_edit::Value::String(s) => J::String(s.value().clone()),
            toml_edit::Value::Integer(i) => J::from(*i.value()),
            toml_edit::Value::Float(f) => serde_json::Number::from_f64(*f.value())
                .map(J::Number)
                .unwrap_or(J::Null),
            toml_edit::Value::Boolean(b) => J::Bool(*b.value()),
            toml_edit::Value::Datetime(d) => J::String(d.value().to_string()),
            toml_edit::Value::Array(a) => J::Array(a.iter().map(conv_value).collect()),
            toml_edit::Value::InlineTable(t) => J::Object(
                t.iter()
                    .map(|(k, v)| (k.to_string(), conv_value(v)))
                    .collect(),
            ),
        }
    }
    let doc = text.parse::<toml_edit::DocumentMut>().ok()?;
    Some(conv(doc.as_item()))
}

/// Statically scans shell startup files for exported variables that override Agent behaviour.
/// Files are read as text, never sourced or executed. Only variable names are recorded.
pub fn scan_shell_profiles(ctx: &HostContext, names: &[&str]) -> Vec<(String, String)> {
    let files = [
        ".zshrc",
        ".zprofile",
        ".zshenv",
        ".bash_profile",
        ".bashrc",
        ".profile",
        ".config/fish/config.fish",
    ];
    let mut found = Vec::new();
    for rel in files {
        let path = ctx.home.join(rel);
        let Ok(read) = read_bounded(&path, 256 * 1024) else {
            continue;
        };
        let text = String::from_utf8_lossy(&read.bytes);
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            for name in names {
                let exported = line
                    .strip_prefix("export ")
                    .map(|l| l.trim_start())
                    .unwrap_or(line);
                let fish = line
                    .strip_prefix("set -gx ")
                    .or_else(|| line.strip_prefix("set -x "))
                    .or_else(|| line.strip_prefix("set -Ux "));
                let hit = exported
                    .strip_prefix(name)
                    .map(|r| r.starts_with('='))
                    .unwrap_or(false)
                    || fish
                        .map(|f| {
                            f.trim_start()
                                .strip_prefix(name)
                                .map(|r| r.starts_with(' '))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false);
                if hit
                    && !found
                        .iter()
                        .any(|(n, f): &(String, String)| n == name && *f == ctx.display_path(&path))
                {
                    found.push((name.to_string(), ctx.display_path(&path)));
                }
            }
        }
    }
    found
}

pub fn skills_in(ctx: &HostContext, dir: &Path) -> (Vec<Skill>, ObsStatus) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (Vec::new(), ObsStatus::NotFound)
        }
        Err(_) => return (Vec::new(), ObsStatus::AccessDenied),
    };
    let mut skills = Vec::new();
    for entry in entries.flatten().take(500) {
        let path = entry.path();
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        // A skill is a directory with a SKILL.md. Hidden folders and containers (e.g. synced
        // collections) are not skills and are not reported as invalid ones.
        if !meta.is_dir() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let manifest = path.join("SKILL.md");
        let (name, description, valid) = match read_bounded(&manifest, 256 * 1024) {
            Ok(read) => parse_skill_frontmatter(&String::from_utf8_lossy(&read.bytes)),
            Err(ReadFailure::NotFound) => continue,
            Err(_) => (None, None, false),
        };
        let has_scripts = std::fs::read_dir(&path)
            .map(|it| {
                it.flatten().take(200).any(|e| {
                    let p = e.path();
                    let n = e.file_name().to_string_lossy().to_lowercase();
                    n == "scripts"
                        || n.ends_with(".sh")
                        || n.ends_with(".py")
                        || n.ends_with(".js")
                        || n.ends_with(".ps1")
                        || (p.is_file() && is_executable_file(&p))
                })
            })
            .unwrap_or(false);
        let dir_name = entry.file_name().to_string_lossy().to_string();
        skills.push(Skill {
            name: crate::core::redact(&name.unwrap_or(dir_name))
                .chars()
                .take(80)
                .collect(),
            description: description.map(|d| crate::core::redact(&d).chars().take(240).collect()),
            location: ctx.display_path(&path),
            has_scripts,
            valid,
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    (skills, ObsStatus::Observed)
}

/// Extracts `name` and `description` from YAML frontmatter. Content is untrusted display text.
pub fn parse_skill_frontmatter(text: &str) -> (Option<String>, Option<String>, bool) {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = body.strip_prefix("---") else {
        return (None, None, false);
    };
    let Some(end) = rest.find("\n---") else {
        return (None, None, false);
    };
    let mut name = None;
    let mut description: Option<String> = None;
    let mut block = false;
    for line in rest[..end].lines() {
        if block {
            if line.starts_with(' ') || line.starts_with('\t') {
                let d = description.get_or_insert_with(String::new);
                if !d.is_empty() {
                    d.push(' ');
                }
                d.push_str(line.trim());
                continue;
            }
            block = false;
        }
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(v) = line.strip_prefix("description:") {
            let v = v.trim();
            // YAML block scalars (`>`, `|`, `>-`, `|-`) continue on indented lines.
            if v.is_empty() || v.starts_with('>') || v.starts_with('|') {
                block = true;
                description = Some(String::new());
            } else {
                description = Some(v.trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    let valid = name.as_deref().map(|n| !n.is_empty()).unwrap_or(false)
        && description
            .as_deref()
            .map(|d| !d.is_empty())
            .unwrap_or(false);
    (name, description, valid)
}

/// Resolves an MCP stdio command for display/health without executing it.
pub fn command_resolvable(ctx: &HostContext, command: &str) -> Option<bool> {
    let p = Path::new(command);
    if p.is_absolute() {
        return Some(p.is_file());
    }
    if command.contains('/') || command.contains('\\') {
        return None; // relative to an unknown working directory
    }
    Some(ctx.search_dirs().iter().any(|(dir, _)| {
        ctx.os
            .exe_suffixes()
            .iter()
            .chain([".cmd"].iter())
            .any(|s| dir.join(format!("{command}{s}")).is_file())
    }))
}

pub fn url_origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?; // drop userinfo
    if host.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{host}"))
}

pub fn installation_id(resolved: &Path) -> String {
    format!(
        "inst_{}",
        &sha256_hex(resolved.to_string_lossy().as_bytes())[..16]
    )
}

/// Full discovery for one Agent. `managed_ids` are installation ids recorded in Nexus's ledger.
pub fn discover_agent(
    ctx: &HostContext,
    adapter: &dyn AgentAdapter,
    managed_ids: &[String],
    now: Instant,
) -> AgentSnapshot {
    let mut installations = Vec::new();
    for cand in find_candidates(ctx, adapter.executable_names()) {
        let source = adapter.classify_source(ctx, &cand.resolved);
        let wrapper = cand
            .path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("cmd"))
            .unwrap_or(false);
        let version = adapter
            .metadata_version(ctx, &cand.path, &cand.resolved, source, now)
            .unwrap_or_else(|| {
                if wrapper || is_script(&cand.resolved) {
                    Observed::missing(
                        ObsStatus::NotChecked,
                        EvidenceSource::VersionProbe,
                        Some(ctx.display_path(&cand.resolved)),
                        now,
                    )
                } else {
                    version_probe(ctx, &cand.resolved, now)
                }
            });
        let id = installation_id(&cand.resolved);
        installations.push(Installation {
            nexus_managed: managed_ids.contains(&id),
            id,
            executable: ctx.display_path(&cand.path),
            resolved: (cand.resolved != cand.path).then(|| ctx.display_path(&cand.resolved)),
            source,
            version,
            path_rank: cand.path_rank,
            executable_ok: is_executable_file(&cand.resolved) || wrapper,
            wrapper,
        });
    }
    let primary = installations
        .iter()
        .enumerate()
        .filter(|(_, i)| i.path_rank.is_some())
        .min_by_key(|(_, i)| i.path_rank)
        .map(|(n, _)| n)
        .or(if installations.is_empty() {
            None
        } else {
            Some(0)
        });
    let configs = adapter.read_configs(ctx);
    let model = adapter.observe_model(ctx, &configs);
    let auth = adapter.observe_auth(ctx, &configs);
    let extensions = adapter.observe_extensions(ctx, &configs);
    let env_overrides = adapter.env_overrides(ctx);
    let primary_source = primary.map(|i| installations[i].source);
    let mut snapshot = AgentSnapshot {
        snapshot_id: crate::core::new_id("snap"),
        agent_id: adapter.id(),
        name: adapter.name(),
        vendor: adapter.vendor(),
        observed_at: now,
        platform: adapter.platform_support(ctx.os),
        installations,
        primary,
        config_sources: configs.into_iter().map(|c| c.source).collect(),
        model,
        auth,
        extensions,
        env_overrides,
        capabilities: Vec::new(),
        runtime_requirement: adapter.runtime_requirement(primary_source),
    };
    snapshot.capabilities = adapter.capabilities(ctx.os, &snapshot);
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_frontmatter_parsing() {
        let (n, d, ok) =
            parse_skill_frontmatter("---\nname: review\ndescription: \"Reviews code\"\n---\nBody");
        assert_eq!(
            (n.as_deref(), d.as_deref(), ok),
            (Some("review"), Some("Reviews code"), true)
        );
        assert!(!parse_skill_frontmatter("no frontmatter").2);
        assert!(!parse_skill_frontmatter("---\nname: x\n---").2);
        let (_, d, ok) = parse_skill_frontmatter(
            "---\nname: x\ndescription: >\n  Folded\n  text\nlicense: MIT\n---\n",
        );
        assert_eq!((d.as_deref(), ok), (Some("Folded text"), true));
    }

    #[test]
    fn containers_and_hidden_folders_are_not_skills() {
        let home = tempfile::tempdir().unwrap();
        let ctx = HostContext::isolated(home.path(), home.path());
        let skills = home.path().join("skills");
        std::fs::create_dir_all(skills.join(".system")).unwrap();
        std::fs::create_dir_all(skills.join("synced/abc")).unwrap();
        std::fs::create_dir_all(skills.join("good")).unwrap();
        std::fs::write(
            skills.join("good/SKILL.md"),
            "---\nname: good\ndescription: d\n---\n",
        )
        .unwrap();
        std::fs::create_dir_all(skills.join("bad")).unwrap();
        std::fs::write(skills.join("bad/SKILL.md"), "no frontmatter").unwrap();
        let (found, _) = skills_in(&ctx, &skills);
        assert_eq!(
            found
                .iter()
                .map(|s| (s.name.as_str(), s.valid))
                .collect::<Vec<_>>(),
            vec![("bad", false), ("good", true)]
        );
    }

    #[test]
    fn url_origin_strips_credentials_and_paths() {
        assert_eq!(
            url_origin("https://user:tok@mcp.example.com:8443/path?key=secret").as_deref(),
            Some("https://mcp.example.com:8443")
        );
        assert_eq!(url_origin("file:///etc/passwd"), None);
        assert_eq!(url_origin("not a url"), None);
    }

    #[cfg(unix)]
    #[test]
    fn candidates_follow_path_order_and_merge_symlinks() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let a = home.path().join("a");
        let b = home.path().join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let real = b.join("tool");
        std::fs::write(&real, "\x7fELF").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::os::unix::fs::symlink(&real, a.join("tool")).unwrap();
        let mut ctx = HostContext::isolated(home.path(), home.path());
        ctx.set_env("PATH", &format!("{}:{}", a.display(), b.display()));
        let found = find_candidates(&ctx, &["tool"]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, a.join("tool"));
        assert_eq!(found[0].path_rank, Some(0));
    }

    #[cfg(unix)]
    #[test]
    fn probe_policy_rejects_scripts_and_writable_binaries() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let ctx = HostContext::isolated(home.path(), home.path());
        let script = home.path().join("wrapper");
        std::fs::write(&script, "#!/usr/bin/env node\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        // tempdir lives under /var/folders or /tmp on macOS → untrusted location wins first.
        assert!(probe_policy(&ctx, &script).is_err());
        assert_eq!(
            version_probe(&ctx, &script, Instant(0)).status,
            ObsStatus::Unsafe
        );
        let writable = home.path().join("bin");
        std::fs::write(&writable, "\x7fELF").unwrap();
        std::fs::set_permissions(&writable, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(probe_policy(&ctx, &writable).is_err());
    }

    #[test]
    fn shell_profile_scan_reports_names_not_values() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join(".zshrc"), "# export GEMINI_MODEL=ignored\nexport ANTHROPIC_MODEL=claude-secret-choice\nexport OTHER=1\n").unwrap();
        std::fs::create_dir_all(home.path().join(".config/fish")).unwrap();
        std::fs::write(
            home.path().join(".config/fish/config.fish"),
            "set -gx GEMINI_MODEL foo\n",
        )
        .unwrap();
        let ctx = HostContext::isolated(home.path(), home.path());
        let found = scan_shell_profiles(&ctx, &["ANTHROPIC_MODEL", "GEMINI_MODEL"]);
        assert_eq!(found.len(), 2);
        assert!(found
            .iter()
            .all(|(_, f)| !f.contains("claude-secret-choice")));
        assert!(found
            .iter()
            .any(|(n, f)| n == "ANTHROPIC_MODEL" && f.ends_with(".zshrc")));
    }

    #[test]
    fn toml_conversion_keeps_tables() {
        let v = toml_to_json(
            "model = \"m\"\n[mcp_servers.docs]\nurl = \"https://x\"\nenabled = false\n",
        )
        .unwrap();
        assert_eq!(
            v["mcp_servers"]["docs"]["enabled"],
            serde_json::json!(false)
        );
    }
}
