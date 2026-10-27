//! Immutable observation types shared by Discovery, Health, Diagnostics and IPC.
//! Every observation records its source, time and an explicit failure class; "unknown" is a
//! first-class value, never silently converted to success or absence.

use crate::core::Instant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentId {
    Codex,
    Claude,
    Gemini,
    Opencode,
}

impl AgentId {
    pub const ALL: [AgentId; 4] = [AgentId::Codex, AgentId::Claude, AgentId::Gemini, AgentId::Opencode];
    pub fn as_str(self) -> &'static str {
        match self {
            AgentId::Codex => "codex",
            AgentId::Claude => "claude",
            AgentId::Gemini => "gemini",
            AgentId::Opencode => "opencode",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        AgentId::ALL.into_iter().find(|a| a.as_str() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Readiness {
    Ready,
    Attention,
    Blocked,
    Unknown,
    Unsupported,
}

/// Why an observation has no value. Kept distinct per Architecture §5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ObsStatus {
    Observed,
    NotFound,
    AccessDenied,
    Timeout,
    ParseFailed,
    NotChecked,
    Unsafe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceSource {
    FileMetadata,
    InstallLayout,
    PackageManifest,
    VersionProbe,
    ConfigFile,
    Environment,
    ShellProfile,
    PlatformApi,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Observed<T> {
    pub value: Option<T>,
    pub status: ObsStatus,
    pub source: EvidenceSource,
    /// Display-safe location (home abbreviated), never a secret.
    pub location: Option<String>,
    pub observed_at: Instant,
}

impl<T> Observed<T> {
    pub fn ok(value: T, source: EvidenceSource, location: Option<String>, at: Instant) -> Self {
        Observed { value: Some(value), status: ObsStatus::Observed, source, location, observed_at: at }
    }
    pub fn missing(status: ObsStatus, source: EvidenceSource, location: Option<String>, at: Instant) -> Self {
        Observed { value: None, status, source, location, observed_at: at }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallSource {
    NativeInstaller,
    Homebrew,
    Npm,
    ScriptInstaller,
    WinGet,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    /// Stable opaque id derived from the canonical executable path.
    pub id: String,
    pub executable: String,
    pub resolved: Option<String>,
    pub source: InstallSource,
    pub version: Observed<String>,
    /// Position in PATH order (0 = would run first) or `None` if found only in a known directory.
    pub path_rank: Option<usize>,
    pub executable_ok: bool,
    /// Nexus installed it (from the operation ledger). External installations are adopted, not owned.
    pub nexus_managed: bool,
    /// Windows `.cmd` shim or other wrapper that probes do not execute.
    pub wrapper: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigFormat {
    Toml,
    Json,
    Jsonc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigScope {
    User,
    Managed,
    AgentState,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSource {
    pub id: String,
    pub scope: ConfigScope,
    pub format: ConfigFormat,
    pub location: String,
    pub status: ObsStatus,
    pub fingerprint: Option<String>,
    pub via_symlink: bool,
    /// Agent tolerates a broken file (continues without it) rather than refusing to start.
    pub tolerant: bool,
    #[serde(skip)]
    pub path: std::path::PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelSourceKind {
    /// Agent's own default: nothing selects a model explicitly.
    AgentDefault,
    UserConfig,
    ManagedConfig,
    Environment,
    ConfigEnvBlock,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelLayer {
    pub kind: ModelSourceKind,
    pub location: String,
    pub value: Option<String>,
    /// Highest-precedence layer that sets a value is effective.
    pub effective: bool,
    /// Only observable in shells Nexus did not launch (static profile scan); never authoritative.
    pub uncertain: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelObservation {
    pub effective: Option<String>,
    pub effective_kind: ModelSourceKind,
    /// All layers in precedence order, highest first.
    pub layers: Vec<ModelLayer>,
    pub provider_hint: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthEvidence {
    /// Agent-owned login artifact exists (contents never read).
    AgentLoginPresent,
    /// An API-key variable is present in Nexus's environment (value never read).
    ApiKeyEnvPresent,
    /// Third-party cloud routing selected (Bedrock/Vertex) — credentials owned by that cloud CLI.
    CloudRouting,
    /// Credentials may live in the OS credential store; Nexus does not read Agent-owned items.
    OsStoreUnverified,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthObservation {
    pub evidence: Vec<AuthEvidence>,
    pub locations: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub name: String,
    pub transport: McpTransport,
    pub enabled: Option<bool>,
    /// Command basename only (arguments and env values are never exposed).
    pub command: Option<String>,
    pub command_resolvable: Option<bool>,
    /// Origin (scheme://host) only; paths/queries may carry tokens.
    pub endpoint_origin: Option<String>,
    pub env_names: Vec<String>,
    pub scope: ConfigScope,
    pub location: String,
    pub nexus_managed: bool,
    pub can_toggle: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    pub description: Option<String>,
    pub location: String,
    pub has_scripts: bool,
    pub valid: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionObservation {
    pub mcp: Vec<McpServer>,
    pub mcp_status: ObsStatus,
    pub skills: Vec<Skill>,
    pub skills_status: ObsStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvOverride {
    pub variable: String,
    /// `process` = visible to Nexus and to Agents Nexus launches; `shellProfile` = statically
    /// found in a shell startup file (may or may not be active in the user's terminal).
    pub origin: String,
    pub location: Option<String>,
    pub affects: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Support {
    Supported,
    Unverified,
    Unsupported,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub key: &'static str,
    pub support: Support,
    /// Translation key explaining the limitation.
    pub reason: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSnapshot {
    pub snapshot_id: String,
    pub agent_id: AgentId,
    pub name: &'static str,
    pub vendor: &'static str,
    pub observed_at: Instant,
    pub platform: Support,
    pub installations: Vec<Installation>,
    /// Index of the installation that would run first from PATH.
    pub primary: Option<usize>,
    pub config_sources: Vec<ConfigSource>,
    pub model: ModelObservation,
    pub auth: AuthObservation,
    pub extensions: ExtensionObservation,
    pub env_overrides: Vec<EnvOverride>,
    pub capabilities: Vec<Capability>,
    pub runtime_requirement: Option<RuntimeRequirement>,
}

impl AgentSnapshot {
    pub fn primary_installation(&self) -> Option<&Installation> {
        self.primary.and_then(|i| self.installations.get(i))
    }
    pub fn capability(&self, key: &str) -> Support {
        self.capabilities.iter().find(|c| c.key == key).map(|c| c.support).unwrap_or(Support::Unsupported)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRequirement {
    pub tool: &'static str,
    pub minimum: Option<&'static str>,
    /// Official documentation the requirement comes from.
    pub evidence: &'static str,
}

/// Compares dotted numeric versions, ignoring pre-release suffixes. `None` if unparseable.
pub fn version_at_least(version: &str, minimum: &str) -> Option<bool> {
    let parse = |s: &str| -> Option<Vec<u64>> {
        let core = s.trim().trim_start_matches('v').split(['-', '+', ' ']).next()?;
        core.split('.').map(|p| p.parse::<u64>().ok()).collect()
    };
    let (mut a, mut b) = (parse(version)?, parse(minimum)?);
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    Some(a >= b)
}

/// Extracts the first `x.y.z` token from probe output such as `codex-cli 0.164.0`.
pub fn extract_version(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',')
        .map(|t| t.trim_start_matches('v'))
        .find(|t| {
            let core = t.split(['-', '+']).next().unwrap_or("");
            let parts: Vec<&str> = core.split('.').collect();
            parts.len() >= 2 && parts.len() <= 4 && parts.iter().all(|p| !p.is_empty() && p.len() <= 9 && p.chars().all(|c| c.is_ascii_digit()))
        })
        .map(|t| t.chars().take(64).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_extraction_from_real_probe_shapes() {
        assert_eq!(extract_version("codex-cli 0.164.0\n").as_deref(), Some("0.164.0"));
        assert_eq!(extract_version("opencode v2.1.4").as_deref(), Some("2.1.4"));
        assert_eq!(extract_version("2.1.311 (Claude Code)").as_deref(), Some("2.1.311"));
        assert_eq!(extract_version("0.64.0-preview.1").as_deref(), Some("0.64.0-preview.1"));
        assert_eq!(extract_version("no version here"), None);
        assert_eq!(extract_version("error 404"), None);
    }

    #[test]
    fn version_comparison() {
        assert_eq!(version_at_least("v26.10.0", "20.0.0"), Some(true));
        assert_eq!(version_at_least("18.19.1", "20.0.0"), Some(false));
        assert_eq!(version_at_least("20", "20.0.0"), Some(true));
        assert_eq!(version_at_least("x.y", "1.0"), None);
    }
}
