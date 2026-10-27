//! OpenCode. Evidence (docs.opencode.ai config / mcp-servers / intro, 2026-10-27): global config
//! `~/.config/opencode/opencode.json` (JSON or JSONC), files are merged with later sources
//! overriding: global < `OPENCODE_CONFIG` custom file < project < `OPENCODE_CONFIG_CONTENT`.
//! `model` is `provider/model`. MCP servers under `mcp.<name>` with `type` local (`command`
//! array) or remote (`url`), and `enabled`. OpenCode 2 reads V1-shaped config and translates it
//! in memory; Nexus writes only V1-shaped keys and never mixes V2 field names.
//! Install via npm `opencode-ai`; the curl script and Homebrew tap are not automated by Nexus.

use super::codex::{basename, finish_layers};
use super::*;
use crate::config::{plan_json_remove, plan_json_set, ConfigEdit};
use crate::core::NexusError;
use crate::discovery::{command_resolvable, config_value, npm_package_version, read_config, skills_in, url_origin};
use crate::lifecycle::{Mechanism, Op, Recipe};
use std::path::PathBuf;

pub struct OpenCode;

fn global_dir(ctx: &HostContext) -> PathBuf {
    ctx.xdg_config_home().join("opencode")
}

fn global_reads(ctx: &HostContext) -> Vec<ReadConfig> {
    let dir = global_dir(ctx);
    vec![
        read_config(ctx, "opencode.global.json", dir.join("opencode.json"), ConfigScope::User, ConfigFormat::Jsonc, false),
        read_config(ctx, "opencode.global.jsonc", dir.join("opencode.jsonc"), ConfigScope::User, ConfigFormat::Jsonc, false),
    ]
}

/// The single global file Nexus may edit. Two coexisting global files make the effective merge
/// order unverifiable, so mutation is refused until the user resolves it.
fn writable_global(ctx: &HostContext) -> NexusResult<ReadConfig> {
    let mut reads = global_reads(ctx);
    let existing: Vec<usize> = reads.iter().enumerate().filter(|(_, r)| r.source.status != ObsStatus::NotFound).map(|(i, _)| i).collect();
    match existing.as_slice() {
        [] => Ok(reads.remove(0)),
        [one] => Ok(reads.remove(*one)),
        _ => Err(NexusError::new("config.ambiguous_source")),
    }
}

fn provider_prefix(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::Anthropic => Some("anthropic"),
        ProviderKind::Openai => Some("openai"),
        ProviderKind::Google => Some("google"),
        _ => None,
    }
}

impl AgentAdapter for OpenCode {
    fn id(&self) -> AgentId { AgentId::Opencode }
    fn name(&self) -> &'static str { "OpenCode" }
    fn vendor(&self) -> &'static str { "OpenCode" }
    fn executable_names(&self) -> &'static [&'static str] { &["opencode"] }
    fn platform_support(&self, os: Os) -> Support { platform_support_level(os) }

    fn metadata_version(&self, ctx: &HostContext, _exe: &Path, resolved: &Path, source: InstallSource, now: Instant) -> Option<Observed<String>> {
        match source {
            InstallSource::Npm => npm_package_version(ctx, resolved, "opencode-ai", now),
            _ => None,
        }
    }

    fn read_configs(&self, ctx: &HostContext) -> Vec<ReadConfig> {
        let mut v = global_reads(ctx);
        if let Some(custom) = ctx.env_value("OPENCODE_CONFIG").filter(|p| Path::new(p).is_absolute()) {
            v.push(read_config(ctx, "opencode.custom", PathBuf::from(custom), ConfigScope::User, ConfigFormat::Jsonc, false));
        }
        v
    }

    fn observe_model(&self, ctx: &HostContext, configs: &[ReadConfig]) -> ModelObservation {
        let mut layers = Vec::new();
        if ctx.env_present("OPENCODE_CONFIG_CONTENT") {
            layers.push(ModelLayer { kind: ModelSourceKind::Environment, location: "OPENCODE_CONFIG_CONTENT".into(), value: None, effective: false, uncertain: true });
        }
        if let Some(read) = find_read(configs, "opencode.custom") {
            layers.push(ModelLayer { kind: ModelSourceKind::UserConfig, location: read.source.location.clone(), value: config_value(read).and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from)), effective: false, uncertain: false });
        }
        let globals: Vec<&ReadConfig> = ["opencode.global.jsonc", "opencode.global.json"].iter().filter_map(|id| find_read(configs, id)).filter(|r| r.source.status != ObsStatus::NotFound).collect();
        let values: Vec<Option<String>> = globals.iter().map(|r| config_value(r).and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))).collect();
        let conflicting = values.iter().flatten().collect::<std::collections::BTreeSet<_>>().len() > 1;
        for (read, value) in globals.iter().zip(values) {
            layers.push(ModelLayer { kind: ModelSourceKind::UserConfig, location: read.source.location.clone(), value, effective: false, uncertain: conflicting });
        }
        let obs = finish_layers(layers, None);
        let hint = obs.effective.as_ref().and_then(|m| m.split_once('/').map(|(p, _)| p.to_string()));
        ModelObservation { provider_hint: hint, ..obs }
    }

    fn observe_auth(&self, ctx: &HostContext, _configs: &[ReadConfig]) -> AuthObservation {
        let data_home = match ctx.env_value("XDG_DATA_HOME") {
            Some(v) if Path::new(v).is_absolute() => PathBuf::from(v),
            _ => ctx.home.join(".local").join("share"),
        };
        let auth = data_home.join("opencode").join("auth.json");
        let mut evidence = Vec::new();
        let mut locations = Vec::new();
        if auth.is_file() {
            evidence.push(AuthEvidence::AgentLoginPresent);
            locations.push(ctx.display_path(&auth));
        }
        if ctx.env_present("ANTHROPIC_API_KEY") || ctx.env_present("OPENAI_API_KEY") || ctx.env_present("GEMINI_API_KEY") {
            evidence.push(AuthEvidence::ApiKeyEnvPresent);
        }
        AuthObservation { evidence, locations }
    }

    fn observe_extensions(&self, ctx: &HostContext, configs: &[ReadConfig]) -> ExtensionObservation {
        let mut mcp = Vec::new();
        let mut status = ObsStatus::NotFound;
        let single_global = configs.iter().filter(|r| r.source.id.starts_with("opencode.global") && r.source.status != ObsStatus::NotFound).count() <= 1;
        for read in configs {
            match read.source.status {
                ObsStatus::Observed => status = ObsStatus::Observed,
                ObsStatus::NotFound => continue,
                other => {
                    if status != ObsStatus::Observed {
                        status = other;
                    }
                    continue;
                }
            }
            let servers = config_value(read).and_then(|v| v.get("mcp").and_then(|m| m.as_object()).cloned()).unwrap_or_default();
            for (name, entry) in servers {
                let command = entry.get("command").and_then(|c| c.as_array()).and_then(|a| a.first()).and_then(|c| c.as_str());
                let url = entry.get("url").and_then(|c| c.as_str());
                let transport = match entry.get("type").and_then(|t| t.as_str()) {
                    Some("local") => McpTransport::Stdio,
                    Some("remote") => McpTransport::Http,
                    _ => McpTransport::Unknown,
                };
                mcp.push(McpServer {
                    name: crate::core::redact(&name),
                    transport,
                    enabled: Some(entry.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true)),
                    command: command.map(basename),
                    command_resolvable: command.and_then(|c| command_resolvable(ctx, c)),
                    endpoint_origin: url.and_then(url_origin),
                    env_names: entry.get("environment").and_then(|e| e.as_object()).map(|o| o.keys().cloned().collect()).unwrap_or_default(),
                    scope: ConfigScope::User,
                    location: read.source.location.clone(),
                    nexus_managed: false,
                    can_toggle: single_global && read.source.id.starts_with("opencode.global"),
                });
            }
        }
        let (skills, skills_status) = skills_in(ctx, &global_dir(ctx).join("skills"));
        ExtensionObservation { mcp, mcp_status: status, skills, skills_status }
    }

    fn override_variables(&self) -> &'static [(&'static str, &'static str)] {
        &[("OPENCODE_CONFIG", "configLocation"), ("OPENCODE_CONFIG_CONTENT", "model"), ("OPENCODE_CONFIG_DIR", "configLocation")]
    }

    fn runtime_requirement(&self, source: Option<InstallSource>) -> Option<RuntimeRequirement> {
        (source == Some(InstallSource::Npm)).then_some(RuntimeRequirement { tool: "node", minimum: None, evidence: "https://docs.opencode.ai/docs/" })
    }

    fn capabilities(&self, os: Os, snapshot: &AgentSnapshot) -> Vec<Capability> {
        let level = platform_support_level(os);
        let npm = snapshot.primary_installation().map(|i| i.source) == Some(InstallSource::Npm);
        let ambiguous = snapshot.config_sources.iter().filter(|c| c.id.starts_with("opencode.global") && c.status != ObsStatus::NotFound).count() > 1;
        let write = if ambiguous { Support::Unsupported } else { level };
        let write_reason = if ambiguous { Some("capability.opencodeAmbiguousConfig") } else { None };
        vec![
            cap("discovery", level, None),
            cap("health", level, None),
            cap("model.write", write, write_reason),
            cap("mcp.read", level, None),
            cap("mcp.toggle", write, write_reason),
            cap("mcp.addRemote", write, write_reason),
            cap("skills.read", level, None),
            cap("skills.write", Support::Unsupported, Some("capability.skillsWriteDeferred")),
            cap("lifecycle.install", level, None),
            cap("lifecycle.update", if npm { level } else { Support::Unsupported }, if npm { None } else { Some("capability.scriptInstallerManaged") }),
            cap("lifecycle.uninstall", if npm { level } else { Support::Unsupported }, if npm { None } else { Some("capability.scriptInstallerManaged") }),
            cap("launch", if os == Os::Macos { Support::Supported } else { Support::Unsupported }, if os == Os::Macos { None } else { Some("capability.launchMacOnly") }),
            cap("online.validation", Support::Unsupported, Some("capability.onlineDeferred")),
        ]
    }

    fn model_target(&self) -> Option<&dyn ModelTarget> { Some(self) }
    fn mcp_target(&self) -> Option<&dyn McpTarget> { Some(self) }

    fn recipes(&self) -> Vec<Recipe> {
        vec![Recipe { id: "opencode.npm", agent: AgentId::Opencode, mechanism: Mechanism::Npm { package: "opencode-ai" }, ops: &[Op::Install, Op::Update, Op::Uninstall], evidence: "https://docs.opencode.ai/docs/", runs_package_scripts: true }]
    }
}

impl ModelTarget for OpenCode {
    fn provider_support(&self, kind: ProviderKind) -> Support {
        // Credentials stay in OpenCode's own auth store; custom providers need a secret handoff.
        if provider_prefix(kind).is_some() { Support::Supported } else { Support::Unsupported }
    }
    fn native_identifier(&self, kind: ProviderKind, model_id: &str) -> String {
        match provider_prefix(kind) {
            Some(prefix) if !model_id.starts_with(&format!("{prefix}/")) => format!("{prefix}/{model_id}"),
            _ => model_id.to_string(),
        }
    }
    fn plan_model(&self, ctx: &HostContext, identifier: &str) -> NexusResult<ConfigEdit> {
        plan_json_set(&writable_global(ctx)?, &["model"], serde_json::Value::String(identifier.to_string()))
    }
    fn user_config_model(&self, ctx: &HostContext) -> Option<String> {
        writable_global(ctx).ok().and_then(|r| config_value(&r)).and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))
    }
}

impl McpTarget for OpenCode {
    fn plan_toggle(&self, ctx: &HostContext, name: &str, enabled: bool) -> NexusResult<ConfigEdit> {
        let read = writable_global(ctx)?;
        let value = config_value(&read).ok_or_else(|| NexusError::new("config.malformed_read_only"))?;
        let entry = value.get("mcp").and_then(|m| m.get(name)).ok_or_else(|| NexusError::new("config.entry_missing"))?;
        if entry.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true) == enabled {
            return Err(NexusError::new("config.no_change"));
        }
        plan_json_set(&read, &["mcp", name, "enabled"], serde_json::Value::Bool(enabled))
    }
    fn plan_add_remote(&self, ctx: &HostContext, name: &str, url: &str) -> NexusResult<ConfigEdit> {
        validate_mcp_name(name)?;
        validate_mcp_url(url)?;
        let read = writable_global(ctx)?;
        if config_value(&read).and_then(|v| v.get("mcp").and_then(|m| m.get(name)).cloned()).is_some() {
            return Err(NexusError::new("extension.name_conflict"));
        }
        plan_json_set(&read, &["mcp", name], serde_json::json!({ "type": "remote", "url": url, "enabled": true }))
    }
    fn plan_remove(&self, ctx: &HostContext, name: &str) -> NexusResult<ConfigEdit> {
        plan_json_remove(&writable_global(ctx)?, &["mcp", name])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(json: Option<&str>, jsonc: Option<&str>) -> (tempfile::TempDir, HostContext) {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join(".config/opencode");
        std::fs::create_dir_all(&cfg).unwrap();
        if let Some(t) = json {
            std::fs::write(cfg.join("opencode.json"), t).unwrap();
        }
        if let Some(t) = jsonc {
            std::fs::write(cfg.join("opencode.jsonc"), t).unwrap();
        }
        let ctx = HostContext::isolated(dir.path(), &dir.path().join("data"));
        (dir, ctx)
    }

    #[test]
    fn duplicate_global_files_block_mutation_and_mark_conflict() {
        let (_d, ctx) = setup(Some("{\"model\": \"anthropic/a\"}"), Some("{\"model\": \"openai/b\"}"));
        assert_eq!(OpenCode.plan_model(&ctx, "anthropic/c").unwrap_err().code, "config.ambiguous_source");
        let configs = OpenCode.read_configs(&ctx);
        let m = OpenCode.observe_model(&ctx, &configs);
        assert_eq!(m.effective, None);
        assert!(m.layers.iter().all(|l| l.uncertain));
    }

    #[test]
    fn jsonc_comments_survive_model_and_mcp_edits() {
        let original = "{\n  // keep me\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"mcp\": {\n    \"docs\": { \"type\": \"remote\", \"url\": \"https://docs.example.com/mcp\" }\n  }\n}\n";
        let (_d, ctx) = setup(None, Some(original));
        let edit = OpenCode.plan_model(&ctx, "anthropic/claude-x").unwrap();
        assert!(String::from_utf8(edit.after).unwrap().contains("// keep me"));
        let edit = OpenCode.plan_toggle(&ctx, "docs", false).unwrap();
        let out = String::from_utf8(edit.after).unwrap();
        assert!(out.contains("// keep me") && out.contains("\"enabled\": false"));
        assert_eq!(OpenCode.native_identifier(ProviderKind::Anthropic, "claude-x"), "anthropic/claude-x");
        assert_eq!(OpenCode.native_identifier(ProviderKind::Anthropic, "anthropic/claude-x"), "anthropic/claude-x");
    }

    #[test]
    fn local_command_arrays_are_observed_by_basename_only() {
        let (_d, ctx) = setup(Some("{\"mcp\": {\"fs\": {\"type\": \"local\", \"command\": [\"/opt/tools/mcp-fs\", \"--token\", \"sk-abcdefghijklmnopqrstuvwxyz0123\"], \"environment\": {\"KEY\": \"v\"}}}}"), None);
        let configs = OpenCode.read_configs(&ctx);
        let ext = OpenCode.observe_extensions(&ctx, &configs);
        assert_eq!(ext.mcp[0].command.as_deref(), Some("mcp-fs"));
        assert_eq!(ext.mcp[0].command_resolvable, Some(false));
        assert!(!serde_json::to_string(&ext).unwrap().contains("sk-"));
    }
}
