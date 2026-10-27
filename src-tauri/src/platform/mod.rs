//! Platform boundary. Everything that knows about the real home directory, PATH, environment or
//! operating system lives behind `HostContext`, so discovery and mutation can be tested against
//! isolated temporary directories without touching the developer's machine.

pub mod fs;
pub mod process;

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Os {
    Macos,
    Windows,
    Linux,
    Other,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::Macos
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::Other
        }
    }
    pub fn exe_suffixes(self) -> &'static [&'static str] {
        match self {
            // Only real executables. `.cmd`/`.bat` shims are reported but never executed by probes.
            Os::Windows => &[".exe"],
            _ => &[""],
        }
    }
}

/// Environment variables Nexus is allowed to observe. Values of names marked secret are never
/// copied; only their presence is recorded.
pub const OBSERVED_ENV: &[(&str, bool)] = &[
    ("PATH", false),
    ("SHELL", false),
    ("CODEX_HOME", false),
    ("CLAUDE_CONFIG_DIR", false),
    ("ANTHROPIC_MODEL", false),
    ("ANTHROPIC_DEFAULT_MODEL", false),
    ("ANTHROPIC_API_KEY", true),
    ("ANTHROPIC_AUTH_TOKEN", true),
    ("ANTHROPIC_BASE_URL", false),
    ("CLAUDE_CODE_USE_BEDROCK", false),
    ("CLAUDE_CODE_USE_VERTEX", false),
    ("OPENAI_API_KEY", true),
    ("OPENAI_BASE_URL", false),
    ("GEMINI_MODEL", false),
    ("GEMINI_API_KEY", true),
    ("GOOGLE_API_KEY", true),
    ("GOOGLE_CLOUD_PROJECT", false),
    ("GOOGLE_GENAI_USE_VERTEXAI", false),
    ("OPENCODE_CONFIG", false),
    ("OPENCODE_CONFIG_DIR", false),
    ("OPENCODE_CONFIG_CONTENT", true),
    ("XDG_CONFIG_HOME", false),
    ("NPM_CONFIG_PREFIX", false),
    ("HOMEBREW_PREFIX", false),
    ("APPDATA", false),
    ("LOCALAPPDATA", false),
    ("USERPROFILE", false),
    ("ProgramFiles", false),
];

#[derive(Clone, Debug)]
pub struct EnvValue {
    pub present: bool,
    /// `None` for secret-bearing variables even when present.
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HostContext {
    pub os: Os,
    pub arch: &'static str,
    pub home: PathBuf,
    /// Nexus-owned application data (SQLite, encrypted backups). Never inside an Agent directory.
    pub data_dir: PathBuf,
    pub env: BTreeMap<String, EnvValue>,
    /// Extra well-known executable directories searched in addition to PATH, because a GUI app's
    /// PATH differs from the interactive shell's and Nexus never sources shell rc files.
    pub extra_bin_dirs: Vec<PathBuf>,
}

impl HostContext {
    pub fn detect(data_dir: PathBuf) -> Self {
        let home = home_dir().unwrap_or_else(|| PathBuf::from("/nonexistent-home"));
        let mut env = BTreeMap::new();
        for (name, secret) in OBSERVED_ENV {
            if let Ok(value) = std::env::var(name) {
                env.insert(
                    name.to_string(),
                    EnvValue {
                        present: true,
                        value: if *secret { None } else { Some(value) },
                    },
                );
            }
        }
        let os = Os::current();
        let extra_bin_dirs = default_bin_dirs(os, &home, &env);
        HostContext {
            os,
            arch: std::env::consts::ARCH,
            home,
            data_dir,
            env,
            extra_bin_dirs,
        }
    }

    /// Fully isolated context for tests: nothing comes from the real environment.
    #[cfg(test)]
    pub fn isolated(home: &Path, data_dir: &Path) -> Self {
        HostContext {
            os: Os::current(),
            arch: std::env::consts::ARCH,
            home: home.to_path_buf(),
            data_dir: data_dir.to_path_buf(),
            env: BTreeMap::new(),
            extra_bin_dirs: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn set_env(&mut self, name: &str, value: &str) {
        let secret = OBSERVED_ENV.iter().any(|(n, s)| *n == name && *s);
        self.env.insert(
            name.to_string(),
            EnvValue {
                present: true,
                value: if secret {
                    None
                } else {
                    Some(value.to_string())
                },
            },
        );
    }

    pub fn env_value(&self, name: &str) -> Option<&str> {
        self.env.get(name).and_then(|v| v.value.as_deref())
    }
    pub fn env_present(&self, name: &str) -> bool {
        self.env.get(name).map(|v| v.present).unwrap_or(false)
    }

    pub fn path_dirs(&self) -> Vec<PathBuf> {
        let sep = if self.os == Os::Windows { ';' } else { ':' };
        self.env_value("PATH")
            .map(|p| {
                p.split(sep)
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// PATH first (in order), then the well-known directories. Deduplicated, absolute only.
    pub fn search_dirs(&self) -> Vec<(PathBuf, bool)> {
        let mut out: Vec<(PathBuf, bool)> = Vec::new();
        for dir in self.path_dirs() {
            if !out.iter().any(|(d, _)| d == &dir) {
                out.push((dir, true));
            }
        }
        for dir in &self.extra_bin_dirs {
            if !out.iter().any(|(d, _)| d == dir) {
                out.push((dir.clone(), false));
            }
        }
        out
    }

    /// Replaces the home prefix with `~` for display. Paths cross IPC only in this form.
    pub fn display_path(&self, path: &Path) -> String {
        // Canonicalized paths may reach the home through a symlink (e.g. /var → /private/var).
        let canonical_home = std::fs::canonicalize(&self.home).ok();
        let stripped = path.strip_prefix(&self.home).or_else(|e| {
            canonical_home
                .as_deref()
                .map(|h| path.strip_prefix(h))
                .unwrap_or(Err(e))
        });
        match stripped {
            Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
            Ok(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
            Err(_) => path.display().to_string(),
        }
    }

    pub fn xdg_config_home(&self) -> PathBuf {
        match self.env_value("XDG_CONFIG_HOME") {
            Some(v) if Path::new(v).is_absolute() => PathBuf::from(v),
            _ => self.home.join(".config"),
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let var = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let var = std::env::var_os("HOME");
    var.map(PathBuf::from).filter(|p| p.is_absolute())
}

fn default_bin_dirs(os: Os, home: &Path, env: &BTreeMap<String, EnvValue>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    match os {
        Os::Macos | Os::Linux | Os::Other => {
            dirs.push(home.join(".local/bin"));
            dirs.push(home.join(".npm-global/bin"));
            dirs.push(home.join(".opencode/bin"));
            dirs.push(home.join(".bun/bin"));
            dirs.push(PathBuf::from("/opt/homebrew/bin"));
            dirs.push(PathBuf::from("/usr/local/bin"));
            dirs.push(PathBuf::from("/usr/bin"));
            dirs.push(PathBuf::from("/bin"));
            if let Some(prefix) = env.get("NPM_CONFIG_PREFIX").and_then(|v| v.value.clone()) {
                dirs.push(PathBuf::from(prefix).join("bin"));
            }
        }
        Os::Windows => {
            if let Some(appdata) = env.get("APPDATA").and_then(|v| v.value.clone()) {
                dirs.push(PathBuf::from(appdata).join("npm"));
            }
            dirs.push(home.join(".local").join("bin"));
            if let Some(local) = env.get("LOCALAPPDATA").and_then(|v| v.value.clone()) {
                dirs.push(
                    PathBuf::from(&local)
                        .join("Microsoft")
                        .join("WinGet")
                        .join("Links"),
                );
            }
        }
    }
    dirs.retain(|d| d.is_absolute());
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_hides_home_prefix() {
        let ctx = HostContext::isolated(Path::new("/Users/someone"), Path::new("/tmp/data"));
        assert_eq!(
            ctx.display_path(Path::new("/Users/someone/.codex/config.toml")),
            format!("~{}.codex/config.toml", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(
            ctx.display_path(Path::new("/opt/homebrew/bin/codex")),
            "/opt/homebrew/bin/codex"
        );
    }

    #[test]
    fn secret_env_values_are_never_captured() {
        let mut ctx = HostContext::isolated(Path::new("/h"), Path::new("/d"));
        ctx.set_env("ANTHROPIC_API_KEY", "sk-ant-secret");
        ctx.set_env("GEMINI_MODEL", "gemini-x");
        assert!(ctx.env_present("ANTHROPIC_API_KEY"));
        assert_eq!(ctx.env_value("ANTHROPIC_API_KEY"), None);
        assert_eq!(ctx.env_value("GEMINI_MODEL"), Some("gemini-x"));
    }

    #[test]
    fn relative_path_entries_are_ignored() {
        let mut ctx = HostContext::isolated(Path::new("/h"), Path::new("/d"));
        ctx.set_env(
            "PATH",
            if cfg!(windows) {
                ".;C:\\bin"
            } else {
                ".:/usr/bin::bin"
            },
        );
        let dirs = ctx.path_dirs();
        assert_eq!(dirs.len(), 1);
    }
}
