//! Compiled, first-party Agent capability adapters (ADR 001). The base trait covers detection
//! and observation; mutation capabilities (`ModelTarget`, `McpTarget`) are separate optional
//! traits so an Agent never claims an operation it does not support. Paths, formats and
//! precedence are adapter evidence — verified against official docs on 2026-10-27 (see
//! docs/BACKEND_IMPLEMENTATION_PROGRESS.md §Evidence).

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod opencode;

use crate::config::ConfigEdit;
use crate::core::{Instant, NexusResult};
use crate::discovery::{self, ReadConfig};
use crate::domain::*;
use crate::lifecycle::Recipe;
use crate::models::ProviderKind;
use crate::platform::{HostContext, Os};
use std::path::Path;

pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> AgentId;
    fn name(&self) -> &'static str;
    fn vendor(&self) -> &'static str;
    fn executable_names(&self) -> &'static [&'static str];
    fn platform_support(&self, os: Os) -> Support;

    fn classify_source(&self, ctx: &HostContext, resolved: &Path) -> InstallSource {
        discovery::classify_source(ctx, resolved)
    }
    /// Version from metadata (package manifest, install layout) without executing anything.
    /// `None` means "fall back to the reviewed --version probe".
    fn metadata_version(
        &self,
        ctx: &HostContext,
        exe: &Path,
        resolved: &Path,
        source: InstallSource,
        now: Instant,
    ) -> Option<Observed<String>>;

    fn read_configs(&self, ctx: &HostContext) -> Vec<ReadConfig>;
    fn observe_model(&self, ctx: &HostContext, configs: &[ReadConfig]) -> ModelObservation;
    fn observe_auth(&self, ctx: &HostContext, configs: &[ReadConfig]) -> AuthObservation;
    fn observe_extensions(&self, ctx: &HostContext, configs: &[ReadConfig])
        -> ExtensionObservation;
    /// Variables that change this Agent's behaviour, found in Nexus's environment or (statically)
    /// in shell startup files.
    fn override_variables(&self) -> &'static [(&'static str, &'static str)];
    fn env_overrides(&self, ctx: &HostContext) -> Vec<EnvOverride> {
        let vars = self.override_variables();
        let mut out: Vec<EnvOverride> = vars
            .iter()
            .filter(|(name, _)| ctx.env_present(name))
            .map(|(name, affects)| EnvOverride {
                variable: name.to_string(),
                origin: "process".into(),
                location: None,
                affects: affects.to_string(),
            })
            .collect();
        let names: Vec<&str> = vars.iter().map(|(n, _)| *n).collect();
        for (name, file) in discovery::scan_shell_profiles(ctx, &names) {
            let affects = vars
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, a)| a.to_string())
                .unwrap_or_default();
            out.push(EnvOverride {
                variable: name,
                origin: "shellProfile".into(),
                location: Some(file),
                affects,
            });
        }
        out
    }
    fn runtime_requirement(&self, source: Option<InstallSource>) -> Option<RuntimeRequirement>;
    fn capabilities(&self, os: Os, snapshot: &AgentSnapshot) -> Vec<Capability>;

    fn model_target(&self) -> Option<&dyn ModelTarget> {
        None
    }
    fn mcp_target(&self) -> Option<&dyn McpTarget> {
        None
    }
    /// Compiled lifecycle recipes applicable to this Agent (may be empty).
    fn recipes(&self) -> Vec<Recipe>;
}

pub trait ModelTarget: Send + Sync {
    /// Which Provider kinds this Agent can route to *without Nexus handing it a secret*.
    fn provider_support(&self, kind: ProviderKind) -> Support;
    /// Agent-specific model identifier for a Provider-scoped model id.
    fn native_identifier(&self, kind: ProviderKind, model_id: &str) -> String {
        let _ = kind;
        model_id.to_string()
    }
    /// Plans the minimal user-config change that selects `identifier`.
    fn plan_model(&self, ctx: &HostContext, identifier: &str) -> NexusResult<ConfigEdit>;
    /// The value the user-scope config resolves to after a write (for verification).
    fn user_config_model(&self, ctx: &HostContext) -> Option<String>;
}

pub trait McpTarget: Send + Sync {
    fn plan_toggle(&self, ctx: &HostContext, name: &str, enabled: bool) -> NexusResult<ConfigEdit>;
    fn plan_add_remote(&self, ctx: &HostContext, name: &str, url: &str) -> NexusResult<ConfigEdit>;
    fn plan_remove(&self, ctx: &HostContext, name: &str) -> NexusResult<ConfigEdit>;
}

pub fn registry() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(codex::Codex),
        Box::new(claude::Claude),
        Box::new(gemini::Gemini),
        Box::new(opencode::OpenCode),
    ]
}

pub fn adapter(id: AgentId) -> Box<dyn AgentAdapter> {
    registry()
        .into_iter()
        .find(|a| a.id() == id)
        .expect("every AgentId has a compiled adapter")
}

pub fn cap(key: &'static str, support: Support, reason: Option<&'static str>) -> Capability {
    Capability {
        key,
        support,
        reason,
    }
}

/// macOS behaviour is verified on this machine; Windows paths are implemented from official docs
/// but not executed, so mutation capabilities there are reported as unverified.
pub fn platform_support_level(os: Os) -> Support {
    match os {
        Os::Macos => Support::Supported,
        Os::Windows | Os::Linux => Support::Unverified,
        Os::Other => Support::Unsupported,
    }
}

/// MCP names Nexus accepts for new bindings: conservative, config-key safe for TOML and JSON.
pub fn validate_mcp_name(name: &str) -> NexusResult<()> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || name.starts_with('-')
    {
        return Err(crate::core::NexusError::new("extension.invalid_name"));
    }
    Ok(())
}

/// Remote MCP endpoints must be HTTPS with a public-looking host and no embedded credentials.
/// Loopback HTTP is allowed only for explicit local endpoints (Security §7).
pub fn validate_mcp_url(url: &str) -> NexusResult<()> {
    let err = || crate::core::NexusError::new("extension.invalid_url");
    if url.len() > 2048 || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(err());
    }
    let (scheme, rest) = url.split_once("://").ok_or_else(err)?;
    let authority = rest.split(['/', '?', '#']).next().ok_or_else(err)?;
    if authority.contains('@') || authority.is_empty() {
        return Err(err());
    }
    let host = if authority.starts_with('[') {
        authority
            .split(']')
            .next()
            .unwrap_or("")
            .trim_start_matches('[')
    } else {
        authority.split(':').next().unwrap_or("")
    };
    let host = host.to_ascii_lowercase();
    let loopback = host == "localhost" || host == "127.0.0.1" || host == "::1";
    match scheme {
        "https" => {}
        "http" if loopback => {}
        _ => return Err(err()),
    }
    // Link-local / cloud metadata endpoints are never valid MCP servers.
    if host.starts_with("169.254.")
        || host == "metadata.google.internal"
        || host.starts_with("fe80:")
    {
        return Err(err());
    }
    let v4_blocked = |v4: std::net::Ipv4Addr| {
        v4.is_private()
            || v4.is_loopback()
            || v4.is_unspecified()
            || v4.is_link_local()
            || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
    };
    let ip_blocked = host.parse::<std::net::IpAddr>().map(|ip| match ip {
        std::net::IpAddr::V4(v4) => v4_blocked(v4),
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                || v6.to_ipv4_mapped().map(v4_blocked).unwrap_or(false)
        }
    });
    if !loopback
        && (*ip_blocked.as_ref().unwrap_or(&false) || (ip_blocked.is_err() && !host.contains('.')))
    {
        return Err(err());
    }
    Ok(())
}

pub fn find_read<'a>(configs: &'a [ReadConfig], id: &str) -> Option<&'a ReadConfig> {
    configs.iter().find(|c| c.source.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_url_policy() {
        for ok in [
            "https://mcp.example.com/sse",
            "http://localhost:3000/mcp",
            "http://127.0.0.1:8080",
            "https://mcp.example.com:8443/x?y=1",
        ] {
            assert!(validate_mcp_url(ok).is_ok(), "{ok}");
        }
        for bad in [
            "http://mcp.example.com",
            "https://user:pass@mcp.example.com",
            "https://169.254.169.254/latest",
            "https://10.0.0.5/mcp",
            "https://[fe80::1]/",
            "https://192.168.1.2",
            "file:///etc/passwd",
            "https://intranet/mcp",
            "https://example.com/\nx",
            "javascript:alert(1)",
            "https://[fd00::1]/mcp",
            "https://[::ffff:169.254.169.254]/",
            "https://100.64.1.2/mcp",
        ] {
            assert!(validate_mcp_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn mcp_name_policy() {
        assert!(validate_mcp_name("docs-server_1").is_ok());
        for bad in [
            "",
            "-flag",
            "a b",
            "a.b",
            "a\"b",
            "x]\n[evil",
            &"a".repeat(65),
        ] {
            assert!(validate_mcp_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn every_agent_has_an_adapter() {
        for id in AgentId::ALL {
            assert_eq!(adapter(id).id(), id);
        }
    }
}
