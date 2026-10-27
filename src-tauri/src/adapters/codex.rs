//! OpenAI Codex CLI. Evidence (2026-10-27): user config `~/.codex/config.toml` (TOML), keys
//! `model`, `[mcp_servers.<name>]` with `command`/`args`/`env`/`url`/`enabled`; system config
//! `/etc/codex/config.toml` below user config; install via npm `@openai/codex`, Homebrew
//! `--cask codex`, or the standalone script (disabled in Nexus). `CODEX_HOME` relocates the
//! config directory (observed on this machine's layout; treated as an override).

use super::*;
use crate::config::{plan_toml_edit, ConfigEdit};
use crate::core::NexusError;
use crate::discovery::{
    command_resolvable, config_value, homebrew_path_version, npm_package_version, read_config,
    skills_in, url_origin,
};
use crate::lifecycle::{Mechanism, Op, Recipe};
use std::path::PathBuf;

pub struct Codex;

fn home(ctx: &HostContext) -> PathBuf {
    match ctx.env_value("CODEX_HOME") {
        Some(v) if Path::new(v).is_absolute() => PathBuf::from(v),
        _ => ctx.home.join(".codex"),
    }
}

fn user_config(ctx: &HostContext) -> ReadConfig {
    read_config(
        ctx,
        "codex.user",
        home(ctx).join("config.toml"),
        ConfigScope::User,
        ConfigFormat::Toml,
        false,
    )
}

impl AgentAdapter for Codex {
    fn id(&self) -> AgentId {
        AgentId::Codex
    }
    fn name(&self) -> &'static str {
        "OpenAI Codex"
    }
    fn vendor(&self) -> &'static str {
        "OpenAI"
    }
    fn executable_names(&self) -> &'static [&'static str] {
        &["codex"]
    }
    fn platform_support(&self, os: Os) -> Support {
        platform_support_level(os)
    }

    fn metadata_version(
        &self,
        ctx: &HostContext,
        _exe: &Path,
        resolved: &Path,
        source: InstallSource,
        now: Instant,
    ) -> Option<Observed<String>> {
        match source {
            InstallSource::Npm => npm_package_version(ctx, resolved, "@openai/codex", now),
            InstallSource::Homebrew => homebrew_path_version(ctx, resolved, now),
            _ => None,
        }
    }

    fn read_configs(&self, ctx: &HostContext) -> Vec<ReadConfig> {
        let mut v = vec![user_config(ctx)];
        if ctx.os != Os::Windows {
            v.push(read_config(
                ctx,
                "codex.system",
                PathBuf::from("/etc/codex/config.toml"),
                ConfigScope::Managed,
                ConfigFormat::Toml,
                false,
            ));
        }
        v
    }

    fn observe_model(&self, ctx: &HostContext, configs: &[ReadConfig]) -> ModelObservation {
        let mut layers = Vec::new();
        // A default `profile` can select `[profiles.<name>].model`; Codex documentation differs
        // across versions on how it is chosen, so its effect is reported as uncertain.
        if let Some(read) = find_read(configs, "codex.user") {
            if let Some(profile) = config_value(read)
                .and_then(|v| v.get("profile").and_then(|p| p.as_str()).map(String::from))
            {
                layers.push(ModelLayer {
                    kind: ModelSourceKind::UserConfig,
                    location: format!(
                        "{} (profile = {})",
                        read.source.location,
                        crate::core::redact(&profile)
                    ),
                    value: None,
                    effective: false,
                    uncertain: true,
                });
            }
        }
        for (id, kind) in [
            ("codex.user", ModelSourceKind::UserConfig),
            ("codex.system", ModelSourceKind::ManagedConfig),
        ] {
            if let Some(read) = find_read(configs, id) {
                let value = config_value(read)
                    .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from));
                layers.push(ModelLayer {
                    kind,
                    location: read.source.location.clone(),
                    value,
                    effective: false,
                    uncertain: false,
                });
            }
        }
        let provider_hint = find_read(configs, "codex.user")
            .and_then(config_value)
            .and_then(|v| {
                v.get("model_provider")
                    .and_then(|m| m.as_str())
                    .map(String::from)
            });
        let _ = ctx;
        finish_layers(layers, provider_hint)
    }

    fn observe_auth(&self, ctx: &HostContext, _configs: &[ReadConfig]) -> AuthObservation {
        let mut evidence = Vec::new();
        let mut locations = Vec::new();
        let auth = home(ctx).join("auth.json");
        if auth.is_file() {
            evidence.push(AuthEvidence::AgentLoginPresent);
            locations.push(ctx.display_path(&auth));
        }
        if ctx.env_present("OPENAI_API_KEY") {
            evidence.push(AuthEvidence::ApiKeyEnvPresent);
        }
        AuthObservation {
            evidence,
            locations,
        }
    }

    fn observe_extensions(
        &self,
        ctx: &HostContext,
        configs: &[ReadConfig],
    ) -> ExtensionObservation {
        let read = find_read(configs, "codex.user");
        let (mcp, mcp_status) = match read {
            Some(r) if r.source.status == ObsStatus::Observed => {
                let value = config_value(r).unwrap_or_default();
                let servers = value
                    .get("mcp_servers")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                let list = servers
                    .iter()
                    .map(|(name, entry)| {
                        let command = entry.get("command").and_then(|c| c.as_str());
                        let url = entry.get("url").and_then(|c| c.as_str());
                        McpServer {
                            name: crate::core::clean_name(name),
                            transport: if command.is_some() {
                                McpTransport::Stdio
                            } else if url.is_some() {
                                McpTransport::Http
                            } else {
                                McpTransport::Unknown
                            },
                            enabled: Some(
                                entry
                                    .get("enabled")
                                    .and_then(|e| e.as_bool())
                                    .unwrap_or(true),
                            ),
                            command: command.map(basename),
                            command_resolvable: command.and_then(|c| command_resolvable(ctx, c)),
                            endpoint_origin: url.and_then(url_origin),
                            env_names: entry
                                .get("env")
                                .and_then(|e| e.as_object())
                                .map(|o| o.keys().cloned().collect())
                                .unwrap_or_default(),
                            scope: ConfigScope::User,
                            location: r.source.location.clone(),
                            nexus_managed: false,
                            can_toggle: true,
                        }
                    })
                    .collect();
                (list, ObsStatus::Observed)
            }
            Some(r) => (Vec::new(), r.source.status),
            None => (Vec::new(), ObsStatus::NotChecked),
        };
        let (skills, skills_status) = skills_in(ctx, &home(ctx).join("skills"));
        ExtensionObservation {
            mcp,
            mcp_status,
            skills,
            skills_status,
        }
    }

    fn override_variables(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("CODEX_HOME", "configLocation"),
            ("OPENAI_BASE_URL", "provider"),
        ]
    }

    fn runtime_requirement(&self, source: Option<InstallSource>) -> Option<RuntimeRequirement> {
        // The npm package's launcher is a Node script; standalone/Homebrew builds are native.
        (source == Some(InstallSource::Npm)).then_some(RuntimeRequirement {
            tool: "node",
            minimum: None,
            evidence: "https://github.com/openai/codex",
        })
    }

    fn capabilities(&self, os: Os, snapshot: &AgentSnapshot) -> Vec<Capability> {
        let level = platform_support_level(os);
        let source = snapshot.primary_installation().map(|i| i.source);
        vec![
            cap("discovery", level, None),
            cap("health", level, None),
            cap("model.write", level, None),
            cap("mcp.read", level, None),
            cap("mcp.toggle", level, None),
            cap("mcp.addRemote", level, None),
            cap("skills.read", level, None),
            cap(
                "skills.write",
                Support::Unsupported,
                Some("capability.skillsWriteDeferred"),
            ),
            cap("lifecycle.install", level, None),
            cap(
                "lifecycle.update",
                if matches!(source, Some(InstallSource::Npm | InstallSource::Homebrew)) {
                    level
                } else {
                    Support::Unsupported
                },
                if matches!(source, Some(InstallSource::Npm | InstallSource::Homebrew)) {
                    None
                } else {
                    Some("capability.scriptInstallerManaged")
                },
            ),
            cap(
                "lifecycle.uninstall",
                if matches!(source, Some(InstallSource::Npm | InstallSource::Homebrew)) {
                    level
                } else {
                    Support::Unsupported
                },
                if matches!(source, Some(InstallSource::Npm | InstallSource::Homebrew)) {
                    None
                } else {
                    Some("capability.scriptInstallerManaged")
                },
            ),
            cap(
                "launch",
                if os == Os::Macos {
                    Support::Supported
                } else {
                    Support::Unsupported
                },
                if os == Os::Macos {
                    None
                } else {
                    Some("capability.launchMacOnly")
                },
            ),
            cap(
                "online.validation",
                Support::Unsupported,
                Some("capability.onlineDeferred"),
            ),
        ]
    }

    fn model_target(&self) -> Option<&dyn ModelTarget> {
        Some(self)
    }
    fn mcp_target(&self) -> Option<&dyn McpTarget> {
        Some(self)
    }

    fn recipes(&self) -> Vec<Recipe> {
        vec![
            Recipe {
                id: "codex.npm",
                agent: AgentId::Codex,
                mechanism: Mechanism::Npm {
                    package: "@openai/codex",
                },
                ops: &[Op::Install, Op::Update, Op::Uninstall],
                evidence: "https://github.com/openai/codex",
                runs_package_scripts: true,
            },
            Recipe {
                id: "codex.brew",
                agent: AgentId::Codex,
                mechanism: Mechanism::HomebrewCask { token: "codex" },
                ops: &[Op::Install, Op::Update, Op::Uninstall],
                evidence: "https://github.com/openai/codex",
                runs_package_scripts: false,
            },
        ]
    }
}

impl ModelTarget for Codex {
    fn provider_support(&self, kind: ProviderKind) -> Support {
        // Custom `[model_providers]` need an env_key secret handoff Nexus does not automate.
        if kind == ProviderKind::Openai {
            Support::Supported
        } else {
            Support::Unsupported
        }
    }
    fn plan_model(&self, ctx: &HostContext, identifier: &str) -> NexusResult<ConfigEdit> {
        let read = user_config(ctx);
        plan_toml_edit(&read, "model", |doc| {
            let before = doc.get("model").and_then(|v| v.as_str()).map(String::from);
            if doc.get("model").map(|v| !v.is_str()).unwrap_or(false) {
                return Err(NexusError::new("config.unexpected_shape").with("key", "model"));
            }
            match doc.get_mut("model").and_then(|i| i.as_value_mut()) {
                // Replace the value but keep its surrounding comment/whitespace decor.
                Some(v) => {
                    let decor = v.decor().clone();
                    *v = toml_edit::Value::from(identifier);
                    *v.decor_mut() = decor;
                }
                // Root key-values render before any [table] header, so this stays top-level.
                None => {
                    doc.insert("model", toml_edit::value(identifier));
                }
            }
            Ok((before, Some(identifier.to_string())))
        })
    }
    fn user_config_model(&self, ctx: &HostContext) -> Option<String> {
        config_value(&user_config(ctx))
            .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))
    }
}

fn mcp_table<'a>(
    doc: &'a mut toml_edit::DocumentMut,
    name: &str,
) -> NexusResult<&'a mut toml_edit::Table> {
    doc.get_mut("mcp_servers")
        .and_then(|t| t.as_table_mut())
        .and_then(|t| t.get_mut(name))
        .and_then(|t| t.as_table_mut())
        .ok_or_else(|| NexusError::new("config.entry_missing"))
}

impl McpTarget for Codex {
    fn plan_toggle(&self, ctx: &HostContext, name: &str, enabled: bool) -> NexusResult<ConfigEdit> {
        let read = user_config(ctx);
        plan_toml_edit(&read, &format!("mcp_servers.{name}.enabled"), |doc| {
            let table = mcp_table(doc, name)?;
            let before = table
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if before == enabled {
                return Err(NexusError::new("config.no_change"));
            }
            table.insert("enabled", toml_edit::value(enabled));
            Ok((Some(before.to_string()), Some(enabled.to_string())))
        })
    }
    fn plan_add_remote(&self, ctx: &HostContext, name: &str, url: &str) -> NexusResult<ConfigEdit> {
        validate_mcp_name(name)?;
        validate_mcp_url(url)?;
        let read = user_config(ctx);
        plan_toml_edit(&read, &format!("mcp_servers.{name}"), |doc| {
            if doc.get("mcp_servers").and_then(|t| t.get(name)).is_some() {
                return Err(NexusError::new("extension.name_conflict"));
            }
            if doc
                .get("mcp_servers")
                .map(|t| !t.is_table_like())
                .unwrap_or(false)
            {
                return Err(NexusError::new("config.unexpected_shape"));
            }
            if doc.get("mcp_servers").is_none() {
                let mut t = toml_edit::Table::new();
                t.set_implicit(true);
                doc.insert("mcp_servers", toml_edit::Item::Table(t));
            }
            let mut entry = toml_edit::Table::new();
            entry.insert("url", toml_edit::value(url));
            entry.insert("enabled", toml_edit::value(true));
            doc["mcp_servers"]
                .as_table_mut()
                .ok_or_else(|| NexusError::new("config.unexpected_shape"))?
                .insert(name, toml_edit::Item::Table(entry));
            Ok((None, url_origin(url)))
        })
    }
    fn plan_remove(&self, ctx: &HostContext, name: &str) -> NexusResult<ConfigEdit> {
        let read = user_config(ctx);
        plan_toml_edit(&read, &format!("mcp_servers.{name}"), |doc| {
            let servers = doc
                .get_mut("mcp_servers")
                .and_then(|t| t.as_table_mut())
                .ok_or_else(|| NexusError::new("config.entry_missing"))?;
            let removed = servers
                .remove(name)
                .ok_or_else(|| NexusError::new("config.entry_missing"))?;
            let origin = removed
                .get("url")
                .and_then(|u| u.as_str())
                .and_then(url_origin);
            Ok((origin, None))
        })
    }
}

pub fn basename(command: &str) -> String {
    let name = command.rsplit(['/', '\\']).next().unwrap_or(command);
    crate::core::redact(name).chars().take(64).collect()
}

/// Marks the highest-precedence layer with a value as effective. Layers are given highest first.
pub fn finish_layers(
    mut layers: Vec<ModelLayer>,
    provider_hint: Option<String>,
) -> ModelObservation {
    let mut effective = None;
    let mut effective_kind = ModelSourceKind::AgentDefault;
    for layer in layers.iter_mut() {
        if effective.is_none() && layer.value.is_some() && !layer.uncertain {
            layer.effective = true;
            effective = layer.value.clone();
            effective_kind = layer.kind;
        }
    }
    ModelObservation {
        effective,
        effective_kind,
        layers,
        provider_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with(config: Option<&str>) -> (tempfile::TempDir, HostContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".codex")).unwrap();
        if let Some(c) = config {
            std::fs::write(dir.path().join(".codex/config.toml"), c).unwrap();
        }
        let ctx = HostContext::isolated(dir.path(), &dir.path().join("data"));
        (dir, ctx)
    }

    #[test]
    fn model_and_mcp_observation() {
        let (_d, ctx) = ctx_with(Some("model = \"gpt-x\"\n[mcp_servers.docs]\nurl = \"https://mcp.example.com/a?token=SECRET\"\n[mcp_servers.local]\ncommand = \"/nonexistent/bin/server\"\nargs = [\"--key\", \"sk-abcdefghijklmnopqrstuvwxyz123456\"]\nenabled = false\n"));
        let configs = Codex.read_configs(&ctx);
        let model = Codex.observe_model(&ctx, &configs);
        assert_eq!(model.effective.as_deref(), Some("gpt-x"));
        assert_eq!(model.effective_kind, ModelSourceKind::UserConfig);
        let ext = Codex.observe_extensions(&ctx, &configs);
        let docs = ext.mcp.iter().find(|m| m.name == "docs").unwrap();
        assert_eq!(
            docs.endpoint_origin.as_deref(),
            Some("https://mcp.example.com")
        );
        let local = ext.mcp.iter().find(|m| m.name == "local").unwrap();
        assert_eq!(local.enabled, Some(false));
        assert_eq!(local.command_resolvable, Some(false));
        let json = serde_json::to_string(&ext).unwrap();
        assert!(!json.contains("SECRET") && !json.contains("sk-abc"));
    }

    #[test]
    fn active_profile_makes_effective_model_uncertain() {
        let (_d, ctx) = ctx_with(Some(
            "profile = \"fast\"\nmodel = \"gpt-x\"\n[profiles.fast]\nmodel = \"gpt-mini\"\n",
        ));
        let configs = Codex.read_configs(&ctx);
        let m = Codex.observe_model(&ctx, &configs);
        assert!(m.layers[0].uncertain);
        assert_eq!(m.effective.as_deref(), Some("gpt-x"));
        assert!(m
            .layers
            .iter()
            .any(|l| l.kind == ModelSourceKind::UserConfig && l.value.as_deref() == Some("gpt-x")));
    }

    #[test]
    fn model_write_preserves_comments_and_tables() {
        let original = "# my codex\nmodel = \"old\" # chosen\napproval_policy = \"never\"\n\n[projects.\"/x\"]\ntrust_level = \"trusted\"\n";
        let (_d, ctx) = ctx_with(Some(original));
        let edit = Codex.plan_model(&ctx, "gpt-new").unwrap();
        let out = String::from_utf8(edit.after).unwrap();
        assert_eq!(out, original.replace("\"old\"", "\"gpt-new\""));
        // Insertion when absent stays at the top level, before any table.
        let (_d2, ctx2) = ctx_with(Some("approval_policy = \"never\"\n\n[tui]\nx = 1\n"));
        let out = String::from_utf8(Codex.plan_model(&ctx2, "gpt-new").unwrap().after).unwrap();
        let v: toml_edit::DocumentMut = out.parse().unwrap();
        assert_eq!(v["model"].as_str(), Some("gpt-new"));
        assert!(v["tui"].get("model").is_none());
    }

    #[test]
    fn mcp_toggle_add_remove() {
        let (_d, ctx) = ctx_with(Some(
            "[mcp_servers.docs]\nurl = \"https://mcp.example.com\"\n",
        ));
        let edit = Codex.plan_toggle(&ctx, "docs", false).unwrap();
        assert!(String::from_utf8(edit.after)
            .unwrap()
            .contains("enabled = false"));
        assert_eq!(
            Codex.plan_toggle(&ctx, "docs", true).unwrap_err().code,
            "config.no_change"
        );
        assert_eq!(
            Codex.plan_toggle(&ctx, "missing", true).unwrap_err().code,
            "config.entry_missing"
        );
        assert_eq!(
            Codex
                .plan_add_remote(&ctx, "docs", "https://x.example.com")
                .unwrap_err()
                .code,
            "extension.name_conflict"
        );
        let add = Codex
            .plan_add_remote(&ctx, "new-one", "https://new.example.com/mcp")
            .unwrap();
        let v: toml_edit::DocumentMut = String::from_utf8(add.after).unwrap().parse().unwrap();
        assert_eq!(
            v["mcp_servers"]["new-one"]["url"].as_str(),
            Some("https://new.example.com/mcp")
        );
        assert_eq!(
            v["mcp_servers"]["docs"]["url"].as_str(),
            Some("https://mcp.example.com")
        );
        let rm = Codex.plan_remove(&ctx, "docs").unwrap();
        assert!(!String::from_utf8(rm.after).unwrap().contains("docs"));
    }

    #[test]
    fn malformed_config_is_observed_but_never_edited() {
        let (_d, ctx) = ctx_with(Some("model = \n"));
        let configs = Codex.read_configs(&ctx);
        assert_eq!(configs[0].source.status, ObsStatus::ParseFailed);
        assert_eq!(
            Codex.plan_model(&ctx, "x").unwrap_err().code,
            "config.malformed_read_only"
        );
    }
}
