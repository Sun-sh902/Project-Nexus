//! Provider → Model → Model Profile → Agent Assignment (ADR 004). Four separate records; no
//! universal option bag. Model identifiers are opaque provider-native strings. Capabilities are
//! `unknown` unless a source asserts them, and user-declared metadata is never "verified".

use crate::adapters::{self, ModelTarget};
use crate::core::{NexusError, NexusResult};
use crate::domain::{AgentId, Support};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    Anthropic,
    Openai,
    Google,
    Openrouter,
    OpenaiCompatible,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Openai => "openai",
            ProviderKind::Google => "google",
            ProviderKind::Openrouter => "openrouter",
            ProviderKind::OpenaiCompatible => "openaiCompatible",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [ProviderKind::Anthropic, ProviderKind::Openai, ProviderKind::Google, ProviderKind::Openrouter, ProviderKind::OpenaiCompatible].into_iter().find(|k| k.as_str() == s)
    }
    pub fn default_origin(self) -> Option<&'static str> {
        match self {
            ProviderKind::Anthropic => Some("https://api.anthropic.com"),
            ProviderKind::Openai => Some("https://api.openai.com"),
            ProviderKind::Google => Some("https://generativelanguage.googleapis.com"),
            ProviderKind::Openrouter => Some("https://openrouter.ai"),
            ProviderKind::OpenaiCompatible => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthMode {
    /// The Agent's own login (ChatGPT / Claude account / Google sign-in). Nexus never touches it.
    AgentLogin,
    /// A Nexus-managed secret in the OS credential store.
    NexusSecret,
    /// User-managed environment variable (name only).
    Environment,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub display_name: String,
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub auth_mode: AuthMode,
    /// Opaque credential-store reference; the secret itself never leaves Rust.
    pub secret_ref: Option<String>,
    pub secret_present: Option<bool>,
    pub env_var: Option<String>,
    pub revision: i64,
    pub builtin: bool,
    pub connection: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityClaim {
    pub key: String,
    /// `supported` only when the source asserts it; manual claims are `declared`.
    pub state: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDefinition {
    pub id: String,
    pub provider_id: String,
    /// Opaque provider-native identifier (the ModelReference value).
    pub identifier: String,
    pub display_name: Option<String>,
    /// `builtin` (reviewed seed), `observed` (found in an Agent config), `manual` (user-entered).
    pub source: String,
    pub source_detail: Option<String>,
    pub capabilities: Vec<CapabilityClaim>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub revision: i64,
    pub model_id: String,
    pub reasoning: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentMode {
    Native,
    Direct,
    Profile,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedRecord {
    pub identifier: String,
    pub profile_revision: Option<i64>,
    pub provider_revision: Option<i64>,
    pub applied_at: String,
    pub location: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelAssignment {
    pub agent_id: AgentId,
    pub mode: AssignmentMode,
    pub model_id: Option<String>,
    pub profile_id: Option<String>,
    pub desired_revision: Option<i64>,
    pub applied: Option<AppliedRecord>,
    pub updated_at: String,
}

/// Built-in Providers that route through an Agent's own login. Nothing is stored for them.
pub fn builtin_providers() -> Vec<ProviderProfile> {
    [("builtin.anthropic", "Anthropic", ProviderKind::Anthropic), ("builtin.openai", "OpenAI", ProviderKind::Openai), ("builtin.google", "Google", ProviderKind::Google)]
        .into_iter()
        .map(|(id, name, kind)| ProviderProfile {
            id: id.into(),
            display_name: name.into(),
            kind,
            base_url: kind.default_origin().map(String::from),
            auth_mode: AuthMode::AgentLogin,
            secret_ref: None,
            secret_present: None,
            env_var: None,
            revision: 1,
            builtin: true,
            connection: "notTested",
        })
        .collect()
}

/// Reviewed seed metadata. Only Claude Code's documented model aliases are seeded; concrete model
/// names change too often to hard-code and are discovered from Agent configs or entered manually.
pub fn builtin_models() -> Vec<ModelDefinition> {
    ["default", "sonnet", "opus", "haiku"]
        .into_iter()
        .map(|alias| ModelDefinition {
            id: format!("builtin.anthropic.{alias}"),
            provider_id: "builtin.anthropic".into(),
            identifier: alias.into(),
            display_name: None,
            source: "builtin".into(),
            source_detail: Some("https://code.claude.com/docs/en/model-config (alias, 2026-10-27)".into()),
            capabilities: Vec::new(),
        })
        .collect()
}

pub fn validate_identifier(identifier: &str) -> NexusResult<()> {
    let ok = !identifier.is_empty()
        && identifier.len() <= 200
        && !identifier.starts_with('-')
        && identifier.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@' | '[' | ']'));
    if ok { Ok(()) } else { Err(NexusError::new("model.invalid_identifier")) }
}

pub fn validate_display_name(name: &str) -> NexusResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 80 || trimmed.chars().any(|c| c.is_control()) {
        return Err(NexusError::new("model.invalid_name"));
    }
    Ok(trimmed.to_string())
}

/// Custom provider endpoints: HTTPS only, or loopback HTTP for local servers (Security §7).
pub fn validate_base_url(url: &str) -> NexusResult<String> {
    adapters::validate_mcp_url(url).map_err(|_| NexusError::new("provider.invalid_base_url"))?;
    Ok(url.trim_end_matches('/').to_string())
}

pub fn validate_env_var(name: &str) -> NexusResult<()> {
    if name.is_empty() || name.len() > 64 || !name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') || name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(NexusError::new("provider.invalid_env_var"));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub agent_id: AgentId,
    pub support: Support,
    pub reason: Option<&'static str>,
    /// Identifier as written into the Agent's configuration.
    pub native_identifier: Option<String>,
}

pub fn compatibility(agent: AgentId, provider: &ProviderProfile, model: &ModelDefinition) -> Compatibility {
    let adapter = adapters::adapter(agent);
    let Some(target): Option<&dyn ModelTarget> = adapter.model_target() else {
        return Compatibility { agent_id: agent, support: Support::Unsupported, reason: Some("compat.noModelTarget"), native_identifier: None };
    };
    let support = target.provider_support(provider.kind);
    if support == Support::Unsupported {
        return Compatibility { agent_id: agent, support, reason: Some(if provider.builtin { "compat.providerNotRoutable" } else { "compat.customProviderHandoff" }), native_identifier: None };
    }
    if !provider.builtin && provider.auth_mode != AuthMode::AgentLogin {
        return Compatibility { agent_id: agent, support: Support::Unsupported, reason: Some("compat.customProviderHandoff"), native_identifier: None };
    }
    let native = target.native_identifier(provider.kind, &model.identifier);
    // Manual or observed models are applyable but their availability is not verified.
    let support = if model.source == "builtin" { Support::Supported } else { Support::Unverified };
    Compatibility { agent_id: agent, support, reason: (support == Support::Unverified).then_some("compat.availabilityUnverified"), native_identifier: Some(native) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(provider: &str, id: &str, source: &str) -> ModelDefinition {
        ModelDefinition { id: format!("m.{id}"), provider_id: provider.into(), identifier: id.into(), display_name: None, source: source.into(), source_detail: None, capabilities: vec![] }
    }

    #[test]
    fn provider_routing_is_agent_specific() {
        let anthropic = builtin_providers().into_iter().find(|p| p.kind == ProviderKind::Anthropic).unwrap();
        let sonnet = model("builtin.anthropic", "sonnet", "builtin");
        assert_eq!(compatibility(AgentId::Claude, &anthropic, &sonnet).support, Support::Supported);
        assert_eq!(compatibility(AgentId::Codex, &anthropic, &sonnet).support, Support::Unsupported);
        assert_eq!(compatibility(AgentId::Gemini, &anthropic, &sonnet).support, Support::Unsupported);
        let oc = compatibility(AgentId::Opencode, &anthropic, &sonnet);
        assert_eq!(oc.native_identifier.as_deref(), Some("anthropic/sonnet"));
    }

    #[test]
    fn manual_models_are_never_verified() {
        let openai = builtin_providers().into_iter().find(|p| p.kind == ProviderKind::Openai).unwrap();
        let c = compatibility(AgentId::Codex, &openai, &model("builtin.openai", "my-model", "manual"));
        assert_eq!((c.support, c.reason), (Support::Unverified, Some("compat.availabilityUnverified")));
    }

    #[test]
    fn custom_providers_requiring_secret_handoff_are_unsupported() {
        let custom = ProviderProfile { id: "p1".into(), display_name: "Local".into(), kind: ProviderKind::OpenaiCompatible, base_url: Some("http://localhost:11434".into()), auth_mode: AuthMode::NexusSecret, secret_ref: Some("provider:p1".into()), secret_present: Some(true), env_var: None, revision: 1, builtin: false, connection: "notTested" };
        for agent in AgentId::ALL {
            assert_eq!(compatibility(agent, &custom, &model("p1", "llama", "manual")).support, Support::Unsupported);
        }
    }

    #[test]
    fn identifier_validation_rejects_flags_and_control() {
        for bad in ["", "-rf", "a b", "a\nb", "x;y", "$(id)"] {
            assert!(validate_identifier(bad).is_err(), "{bad}");
        }
        for good in ["claude-opus-5-5", "anthropic/claude-x", "models/gemini-3-flash", "gpt-6.1-sol", "opus[1m]"] {
            assert!(validate_identifier(good).is_ok(), "{good}");
        }
        assert!(validate_base_url("http://evil.example.com").is_err());
        assert!(validate_base_url("http://localhost:11434/v1/").unwrap().ends_with("/v1"));
    }
}
