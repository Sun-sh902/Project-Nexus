//! Trusted, compiled lifecycle recipes (Architecture §11, Security §5). A recipe binds an Agent
//! to one package identity and mechanism. Commands are built here from fixed argv templates plus
//! a version string validated against a strict grammar; nothing comes from the frontend,
//! Manifests or AI output. Script installers (curl | sh) are deliberately not implemented.

use crate::core::{NexusError, NexusResult};
use crate::discovery::{find_candidates, probe_policy};
use crate::domain::*;
use crate::platform::process::{base_env, run, ProcessSpec};
use crate::platform::{HostContext, Os};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Op {
    Install,
    Update,
    Uninstall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Mechanism {
    Npm { package: &'static str },
    HomebrewCask { token: &'static str },
    HomebrewFormula { name: &'static str },
    /// The Agent's own documented updater, run against a verified native installation.
    AgentSelfUpdate { args: &'static [&'static str] },
}

impl Mechanism {
    pub fn source(self) -> InstallSource {
        match self {
            Mechanism::Npm { .. } => InstallSource::Npm,
            Mechanism::HomebrewCask { .. } | Mechanism::HomebrewFormula { .. } => InstallSource::Homebrew,
            Mechanism::AgentSelfUpdate { .. } => InstallSource::NativeInstaller,
        }
    }
    pub fn platforms(self) -> &'static [Os] {
        match self {
            Mechanism::Npm { .. } => &[Os::Macos, Os::Windows, Os::Linux],
            Mechanism::HomebrewCask { .. } => &[Os::Macos],
            Mechanism::HomebrewFormula { .. } => &[Os::Macos, Os::Linux],
            Mechanism::AgentSelfUpdate { .. } => &[Os::Macos, Os::Linux, Os::Windows],
        }
    }
    pub fn package_label(self) -> String {
        match self {
            Mechanism::Npm { package } => format!("npm:{package}"),
            Mechanism::HomebrewCask { token } => format!("brew cask:{token}"),
            Mechanism::HomebrewFormula { name } => format!("brew:{name}"),
            Mechanism::AgentSelfUpdate { .. } => "self-update".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: &'static str,
    pub agent: AgentId,
    pub mechanism: Mechanism,
    pub ops: &'static [Op],
    pub evidence: &'static str,
    /// Package lifecycle scripts may run (disclosed in every plan).
    pub runs_package_scripts: bool,
}

/// Strict version grammar for pinned installs: `1.2.3` with optional `-prerelease`.
pub fn validate_version(v: &str) -> NexusResult<()> {
    let ok = v.len() <= 64
        && !v.starts_with('-')
        && v.split(['-', '+']).next().map(|core| {
            let parts: Vec<&str> = core.split('.').collect();
            parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        }).unwrap_or(false)
        && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    if ok { Ok(()) } else { Err(NexusError::new("lifecycle.invalid_version")) }
}

#[derive(Clone, Debug)]
pub struct Toolchain {
    pub npm: Option<PathBuf>,
    pub node_dir: Option<PathBuf>,
    pub brew: Option<PathBuf>,
}

/// Locates package managers. npm must resolve to a file owned by the user or root and not be
/// writable by others; Homebrew is accepted only at its documented prefixes.
pub fn toolchain(ctx: &HostContext) -> Toolchain {
    let npm = find_candidates(ctx, &["npm"]).into_iter().min_by_key(|c| c.path_rank.unwrap_or(usize::MAX)).filter(|c| owner_ok(&c.resolved)).map(|c| c.path);
    let node_dir = find_candidates(ctx, &["node"]).into_iter().min_by_key(|c| c.path_rank.unwrap_or(usize::MAX)).and_then(|c| c.path.parent().map(PathBuf::from));
    let brew = if ctx.os == Os::Macos {
        ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"].iter().map(PathBuf::from).find(|p| p.is_file() && owner_ok(p))
    } else {
        None
    };
    Toolchain { npm, node_dir, brew }
}

fn owner_ok(p: &std::path::Path) -> bool {
    match probe_policy_ignoring_scripts(p) {
        Ok(()) => true,
        Err(_) => false,
    }
}

fn probe_policy_ignoring_scripts(p: &std::path::Path) -> Result<(), &'static str> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(p).map_err(|_| "missing")?;
        let uid = unsafe { libc::getuid() };
        if meta.uid() != uid && meta.uid() != 0 {
            return Err("owner");
        }
        if meta.mode() & 0o022 != 0 {
            return Err("writable");
        }
    }
    let _ = p;
    Ok(())
}

pub fn available(recipe: &Recipe, ctx: &HostContext, tools: &Toolchain) -> bool {
    recipe.mechanism.platforms().contains(&ctx.os)
        && match recipe.mechanism {
            Mechanism::Npm { .. } => tools.npm.is_some() && tools.node_dir.is_some(),
            Mechanism::HomebrewCask { .. } | Mechanism::HomebrewFormula { .. } => tools.brew.is_some(),
            Mechanism::AgentSelfUpdate { .. } => true,
        }
}

fn child_env(ctx: &HostContext, tools: &Toolchain, extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = &tools.node_dir {
        dirs.push(d.clone());
    }
    if let Some(n) = tools.npm.as_ref().and_then(|n| n.parent()) {
        dirs.push(n.to_path_buf());
    }
    if let Some(b) = tools.brew.as_ref().and_then(|b| b.parent()) {
        dirs.push(b.to_path_buf());
    }
    dirs.extend(["/usr/bin", "/bin", "/usr/sbin", "/sbin"].iter().map(PathBuf::from));
    dirs.dedup();
    let mut env = base_env(&ctx.home, &dirs);
    for (k, v) in extra {
        env.push((k.to_string(), v.to_string()));
    }
    env
}

const BREW_ENV: &[(&str, &str)] = &[("HOMEBREW_NO_AUTO_UPDATE", "1"), ("HOMEBREW_NO_ANALYTICS", "1"), ("HOMEBREW_NO_ENV_HINTS", "1")];

/// Resolves the exact version a plan will install, *before* confirmation. Never `latest` after.
pub fn resolve_version(ctx: &HostContext, tools: &Toolchain, recipe: &Recipe) -> NexusResult<Option<String>> {
    let (program, args, extra): (PathBuf, Vec<String>, &[(&str, &str)]) = match recipe.mechanism {
        Mechanism::Npm { package } => (tools.npm.clone().ok_or_else(|| NexusError::new("lifecycle.tool_missing").with("tool", "npm"))?, vec!["view".into(), format!("{package}@latest"), "version".into(), "--json".into()], &[]),
        Mechanism::HomebrewCask { token } => (tools.brew.clone().ok_or_else(|| NexusError::new("lifecycle.tool_missing").with("tool", "brew"))?, vec!["info".into(), "--json=v2".into(), "--cask".into(), token.into()], BREW_ENV),
        Mechanism::HomebrewFormula { name } => (tools.brew.clone().ok_or_else(|| NexusError::new("lifecycle.tool_missing").with("tool", "brew"))?, vec!["info".into(), "--json=v2".into(), "--formula".into(), name.into()], BREW_ENV),
        Mechanism::AgentSelfUpdate { .. } => return Ok(None),
    };
    let spec = ProcessSpec { program, args, env: child_env(ctx, tools, extra), cwd: ctx.data_dir.clone(), timeout: Duration::from_secs(60), max_output: 256 * 1024 };
    let out = run(&spec)?;
    if !out.success() {
        return Err(NexusError::new("lifecycle.version_resolution_failed").retryable());
    }
    let version = parse_resolved_version(recipe.mechanism, &out.stdout).ok_or_else(|| NexusError::new("lifecycle.version_resolution_failed"))?;
    validate_version(&version)?;
    Ok(Some(version))
}

pub fn parse_resolved_version(mechanism: Mechanism, stdout: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    match mechanism {
        Mechanism::Npm { .. } => v.as_str().map(String::from),
        Mechanism::HomebrewCask { .. } => v.pointer("/casks/0/version")?.as_str().map(|s| s.split(',').next().unwrap_or(s).to_string()),
        Mechanism::HomebrewFormula { .. } => v.pointer("/formulae/0/versions/stable")?.as_str().map(String::from),
        Mechanism::AgentSelfUpdate { .. } => None,
    }
}

/// Builds the exact process for an operation. `agent_exe` is required for self-update.
pub fn build_spec(ctx: &HostContext, tools: &Toolchain, recipe: &Recipe, op: Op, version: Option<&str>, agent_exe: Option<&std::path::Path>) -> NexusResult<ProcessSpec> {
    if !recipe.ops.contains(&op) {
        return Err(NexusError::new("lifecycle.unsupported_operation"));
    }
    if let Some(v) = version {
        validate_version(v)?;
    }
    let timeout = Duration::from_secs(15 * 60);
    let (program, args, extra): (PathBuf, Vec<String>, &[(&str, &str)]) = match (recipe.mechanism, op) {
        (Mechanism::Npm { package }, Op::Install | Op::Update) => {
            let v = version.ok_or_else(|| NexusError::new("lifecycle.version_required"))?;
            (tools.npm.clone().ok_or_else(|| NexusError::new("lifecycle.tool_missing"))?, vec!["install".into(), "--global".into(), "--no-fund".into(), "--no-audit".into(), format!("{package}@{v}")], &[])
        }
        (Mechanism::Npm { package }, Op::Uninstall) => (tools.npm.clone().ok_or_else(|| NexusError::new("lifecycle.tool_missing"))?, vec!["uninstall".into(), "--global".into(), package.into()], &[]),
        (Mechanism::HomebrewCask { token }, op) => (
            tools.brew.clone().ok_or_else(|| NexusError::new("lifecycle.tool_missing"))?,
            vec![match op { Op::Install => "install", Op::Update => "upgrade", Op::Uninstall => "uninstall" }.into(), "--cask".into(), token.into()],
            BREW_ENV,
        ),
        (Mechanism::HomebrewFormula { name }, op) => (
            tools.brew.clone().ok_or_else(|| NexusError::new("lifecycle.tool_missing"))?,
            vec![match op { Op::Install => "install", Op::Update => "upgrade", Op::Uninstall => "uninstall" }.into(), "--formula".into(), name.into()],
            BREW_ENV,
        ),
        (Mechanism::AgentSelfUpdate { args }, Op::Update) => {
            let exe = agent_exe.ok_or_else(|| NexusError::new("lifecycle.target_missing"))?;
            probe_policy(ctx, exe).map_err(|r| NexusError::new("lifecycle.untrusted_executable").with("reason", r))?;
            (exe.to_path_buf(), args.iter().map(|s| s.to_string()).collect(), &[])
        }
        _ => return Err(NexusError::new("lifecycle.unsupported_operation")),
    };
    Ok(ProcessSpec { program, args, env: child_env(ctx, tools, extra), cwd: ctx.home.clone(), timeout, max_output: 16 * 1024 })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NPM: Recipe = Recipe { id: "gemini.npm", agent: AgentId::Gemini, mechanism: Mechanism::Npm { package: "@google/gemini-cli" }, ops: &[Op::Install, Op::Update, Op::Uninstall], evidence: "", runs_package_scripts: true };

    #[test]
    fn version_grammar_rejects_injection() {
        for ok in ["0.64.0", "2.1.311", "1.0.0-preview.1"] {
            assert!(validate_version(ok).is_ok(), "{ok}");
        }
        for bad in ["latest", "1.0", "-1.0.0", "1.0.0 --registry=http://evil", "1.0.0;rm", "^1.0.0", "1.0.0\n", "1.0.0@x"] {
            assert!(validate_version(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn npm_install_is_pinned_and_argv_is_fixed() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = HostContext::isolated(dir.path(), dir.path());
        let tools = Toolchain { npm: Some(PathBuf::from("/usr/bin/true")), node_dir: Some(PathBuf::from("/usr/bin")), brew: None };
        let spec = build_spec(&ctx, &tools, &NPM, Op::Install, Some("0.64.0"), None).unwrap();
        assert_eq!(spec.args.last().unwrap(), "@google/gemini-cli@0.64.0");
        assert!(spec.env.iter().all(|(k, _)| k != "NPM_TOKEN"));
        assert_eq!(build_spec(&ctx, &tools, &NPM, Op::Install, None, None).unwrap_err().code, "lifecycle.version_required");
        assert_eq!(build_spec(&ctx, &tools, &NPM, Op::Install, Some("latest"), None).unwrap_err().code, "lifecycle.invalid_version");
    }

    #[test]
    fn unsupported_operations_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = HostContext::isolated(dir.path(), dir.path());
        let tools = Toolchain { npm: None, node_dir: None, brew: None };
        let r = Recipe { id: "x", agent: AgentId::Claude, mechanism: Mechanism::AgentSelfUpdate { args: &["update"] }, ops: &[Op::Update], evidence: "", runs_package_scripts: false };
        assert_eq!(build_spec(&ctx, &tools, &r, Op::Install, None, None).unwrap_err().code, "lifecycle.unsupported_operation");
        assert_eq!(build_spec(&ctx, &tools, &r, Op::Update, None, None).unwrap_err().code, "lifecycle.target_missing");
    }

    #[test]
    fn resolved_version_parsing() {
        assert_eq!(parse_resolved_version(NPM.mechanism, "\"0.65.1\"\n").as_deref(), Some("0.65.1"));
        assert_eq!(parse_resolved_version(Mechanism::HomebrewCask { token: "codex" }, "{\"casks\":[{\"version\":\"0.164.0\"}]}").as_deref(), Some("0.164.0"));
        assert_eq!(parse_resolved_version(Mechanism::HomebrewFormula { name: "gemini-cli" }, "{\"formulae\":[{\"versions\":{\"stable\":\"0.64.0\"}}]}").as_deref(), Some("0.64.0"));
        assert_eq!(parse_resolved_version(NPM.mechanism, "<html>"), None);
    }
}
