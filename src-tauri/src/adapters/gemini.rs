//! Gemini CLI. Evidence (geminicli.com/docs reference/configuration and get-started/installation,
//! 2026-10-27): layers default < system-defaults < user `~/.gemini/settings.json` < project <
//! system settings < environment < CLI args. Model key `model.name`; `GEMINI_MODEL` overrides.
//! MCP servers in `mcpServers.<name>` (`command`/`args`/`env`/`url`/`httpUrl`); individual
//! servers are disabled through `mcp.excluded`. Requires Node.js 20.0.0+. Install via npm
//! `@google/gemini-cli` or Homebrew `gemini-cli`.

use super::codex::{basename, finish_layers};
use super::*;
use crate::config::{plan_json_remove, plan_json_set, ConfigEdit};
use crate::core::NexusError;
use crate::discovery::{
    command_resolvable, config_value, homebrew_path_version, npm_package_version, read_config,
    skills_in, url_origin,
};
use crate::lifecycle::{Mechanism, Op, Recipe};
use std::path::PathBuf;

pub struct Gemini;

fn user_settings(ctx: &HostContext) -> ReadConfig {
    read_config(
        ctx,
        "gemini.user",
        ctx.home.join(".gemini").join("settings.json"),
        ConfigScope::User,
        ConfigFormat::Jsonc,
        false,
    )
}

fn system_dir(os: Os) -> Option<PathBuf> {
    match os {
        Os::Macos => Some(PathBuf::from("/Library/Application Support/GeminiCli")),
        Os::Windows => Some(PathBuf::from("C:\\ProgramData\\gemini-cli")),
        Os::Linux => Some(PathBuf::from("/etc/gemini-cli")),
        Os::Other => None,
    }
}

fn model_name(v: &serde_json::Value) -> Option<String> {
    match v.get("model")? {
        serde_json::Value::Object(o) => o.get("name").and_then(|n| n.as_str()).map(String::from),
        // Older releases used a top-level string; still observed, never rewritten.
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn excluded(v: &serde_json::Value) -> Vec<String> {
    v.get("mcp")
        .and_then(|m| m.get("excluded"))
        .and_then(|e| e.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
fn allowed(v: &serde_json::Value) -> Option<Vec<String>> {
    v.get("mcp")
        .and_then(|m| m.get("allowed"))
        .and_then(|e| e.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
}

impl AgentAdapter for Gemini {
    fn id(&self) -> AgentId {
        AgentId::Gemini
    }
    fn name(&self) -> &'static str {
        "Gemini CLI"
    }
    fn vendor(&self) -> &'static str {
        "Google"
    }
    fn executable_names(&self) -> &'static [&'static str] {
        &["gemini"]
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
            InstallSource::Homebrew => homebrew_path_version(ctx, resolved, now)
                .or_else(|| npm_package_version(ctx, resolved, "@google/gemini-cli", now)),
            _ => npm_package_version(ctx, resolved, "@google/gemini-cli", now),
        }
    }

    fn read_configs(&self, ctx: &HostContext) -> Vec<ReadConfig> {
        let mut v = vec![user_settings(ctx)];
        if let Some(dir) = system_dir(ctx.os) {
            v.push(read_config(
                ctx,
                "gemini.system",
                dir.join("settings.json"),
                ConfigScope::Managed,
                ConfigFormat::Jsonc,
                false,
            ));
            v.push(read_config(
                ctx,
                "gemini.systemDefaults",
                dir.join("system-defaults.json"),
                ConfigScope::Managed,
                ConfigFormat::Jsonc,
                false,
            ));
        }
        v
    }

    fn observe_model(&self, ctx: &HostContext, configs: &[ReadConfig]) -> ModelObservation {
        let mut layers = vec![ModelLayer {
            kind: ModelSourceKind::Environment,
            location: "GEMINI_MODEL".into(),
            value: ctx.env_value("GEMINI_MODEL").map(String::from),
            effective: false,
            uncertain: false,
        }];
        for (name, file) in crate::discovery::scan_shell_profiles(ctx, &["GEMINI_MODEL"]) {
            layers.push(ModelLayer {
                kind: ModelSourceKind::Environment,
                location: format!("{file} ({name})"),
                value: None,
                effective: false,
                uncertain: true,
            });
        }
        for (id, kind) in [
            ("gemini.system", ModelSourceKind::ManagedConfig),
            ("gemini.user", ModelSourceKind::UserConfig),
            ("gemini.systemDefaults", ModelSourceKind::ManagedConfig),
        ] {
            if let Some(read) = find_read(configs, id) {
                layers.push(ModelLayer {
                    kind,
                    location: read.source.location.clone(),
                    value: config_value(read).as_ref().and_then(model_name),
                    effective: false,
                    uncertain: false,
                });
            }
        }
        finish_layers(layers, Some("google".into()))
    }

    fn observe_auth(&self, ctx: &HostContext, configs: &[ReadConfig]) -> AuthObservation {
        let mut evidence = Vec::new();
        let mut locations = Vec::new();
        let creds = ctx.home.join(".gemini").join("oauth_creds.json");
        if creds.is_file() {
            evidence.push(AuthEvidence::AgentLoginPresent);
            locations.push(ctx.display_path(&creds));
        }
        if ctx.env_present("GEMINI_API_KEY") || ctx.env_present("GOOGLE_API_KEY") {
            evidence.push(AuthEvidence::ApiKeyEnvPresent);
        }
        let vertex_setting = find_read(configs, "gemini.user")
            .and_then(config_value)
            .and_then(|v| {
                v.pointer("/security/auth/selectedType")
                    .and_then(|s| s.as_str())
                    .map(|s| s == "vertex-ai")
            })
            .unwrap_or(false);
        if ctx.env_present("GOOGLE_GENAI_USE_VERTEXAI") || vertex_setting {
            evidence.push(AuthEvidence::CloudRouting);
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
        let (mcp, mcp_status) = match find_read(configs, "gemini.user") {
            Some(r) if r.source.status == ObsStatus::Observed => {
                let value = config_value(r).unwrap_or_default();
                let excluded = excluded(&value);
                let allowed = allowed(&value);
                let servers = value
                    .get("mcpServers")
                    .and_then(|m| m.as_object())
                    .cloned()
                    .unwrap_or_default();
                let list = servers
                    .iter()
                    .map(|(name, entry)| {
                        let command = entry.get("command").and_then(|c| c.as_str());
                        let http = entry.get("httpUrl").and_then(|c| c.as_str());
                        let sse = entry.get("url").and_then(|c| c.as_str());
                        // Documented precedence: httpUrl, then url, then command.
                        let transport = if http.is_some() {
                            McpTransport::Http
                        } else if sse.is_some() {
                            McpTransport::Sse
                        } else if command.is_some() {
                            McpTransport::Stdio
                        } else {
                            McpTransport::Unknown
                        };
                        let enabled = !excluded.contains(name)
                            && allowed.as_ref().map(|a| a.contains(name)).unwrap_or(true);
                        McpServer {
                            name: crate::core::clean_name(name),
                            transport,
                            enabled: Some(enabled),
                            command: if transport == McpTransport::Stdio {
                                command.map(basename)
                            } else {
                                None
                            },
                            command_resolvable: if transport == McpTransport::Stdio {
                                command.and_then(|c| command_resolvable(ctx, c))
                            } else {
                                None
                            },
                            endpoint_origin: http.or(sse).and_then(url_origin),
                            env_names: entry
                                .get("env")
                                .and_then(|e| e.as_object())
                                .map(|o| o.keys().cloned().collect())
                                .unwrap_or_default(),
                            scope: ConfigScope::User,
                            location: r.source.location.clone(),
                            nexus_managed: false,
                            can_toggle: allowed.is_none(),
                        }
                    })
                    .collect();
                (list, ObsStatus::Observed)
            }
            Some(r) => (Vec::new(), r.source.status),
            None => (Vec::new(), ObsStatus::NotChecked),
        };
        let (skills, skills_status) = skills_in(ctx, &ctx.home.join(".gemini").join("skills"));
        ExtensionObservation {
            mcp,
            mcp_status,
            skills,
            skills_status,
        }
    }

    fn override_variables(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("GEMINI_MODEL", "model"),
            ("GOOGLE_GENAI_USE_VERTEXAI", "provider"),
            ("GOOGLE_CLOUD_PROJECT", "provider"),
        ]
    }

    fn runtime_requirement(&self, _source: Option<InstallSource>) -> Option<RuntimeRequirement> {
        Some(RuntimeRequirement {
            tool: "node",
            minimum: Some("20.0.0"),
            evidence: "https://geminicli.com/docs/get-started/installation/",
        })
    }

    fn capabilities(&self, os: Os, snapshot: &AgentSnapshot) -> Vec<Capability> {
        let level = platform_support_level(os);
        let source = snapshot.primary_installation().map(|i| i.source);
        let managed = matches!(source, Some(InstallSource::Npm | InstallSource::Homebrew));
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
                if managed { level } else { Support::Unsupported },
                if managed {
                    None
                } else {
                    Some("capability.unknownInstaller")
                },
            ),
            cap(
                "lifecycle.uninstall",
                if managed { level } else { Support::Unsupported },
                if managed {
                    None
                } else {
                    Some("capability.unknownInstaller")
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
                id: "gemini.npm",
                agent: AgentId::Gemini,
                mechanism: Mechanism::Npm {
                    package: "@google/gemini-cli",
                },
                ops: &[Op::Install, Op::Update, Op::Uninstall],
                evidence: "https://geminicli.com/docs/get-started/installation/",
                runs_package_scripts: true,
            },
            Recipe {
                id: "gemini.brew",
                agent: AgentId::Gemini,
                mechanism: Mechanism::HomebrewFormula { name: "gemini-cli" },
                ops: &[Op::Install, Op::Update, Op::Uninstall],
                evidence: "https://geminicli.com/docs/get-started/installation/",
                runs_package_scripts: false,
            },
        ]
    }
}

impl ModelTarget for Gemini {
    fn provider_support(&self, kind: ProviderKind) -> Support {
        if kind == ProviderKind::Google {
            Support::Supported
        } else {
            Support::Unsupported
        }
    }
    fn plan_model(&self, ctx: &HostContext, identifier: &str) -> NexusResult<ConfigEdit> {
        plan_json_set(
            &user_settings(ctx),
            &["model", "name"],
            serde_json::Value::String(identifier.to_string()),
        )
    }
    fn user_config_model(&self, ctx: &HostContext) -> Option<String> {
        config_value(&user_settings(ctx))
            .as_ref()
            .and_then(model_name)
    }
}

impl McpTarget for Gemini {
    fn plan_toggle(&self, ctx: &HostContext, name: &str, enabled: bool) -> NexusResult<ConfigEdit> {
        let read = user_settings(ctx);
        let value =
            config_value(&read).ok_or_else(|| NexusError::new("config.malformed_read_only"))?;
        if value.get("mcpServers").and_then(|m| m.get(name)).is_none() {
            return Err(NexusError::new("config.entry_missing"));
        }
        if allowed(&value).is_some() {
            // An allow-list is a policy choice; Nexus does not rewrite it.
            return Err(NexusError::new("extension.allowlist_policy"));
        }
        let mut list = excluded(&value);
        let currently = !list.iter().any(|n| n == name);
        if currently == enabled {
            return Err(NexusError::new("config.no_change"));
        }
        if enabled {
            list.retain(|n| n != name);
        } else {
            list.push(name.to_string());
        }
        plan_json_set(&read, &["mcp", "excluded"], serde_json::json!(list))
    }
    fn plan_add_remote(&self, ctx: &HostContext, name: &str, url: &str) -> NexusResult<ConfigEdit> {
        validate_mcp_name(name)?;
        validate_mcp_url(url)?;
        let read = user_settings(ctx);
        if config_value(&read)
            .and_then(|v| v.get("mcpServers").and_then(|m| m.get(name)).cloned())
            .is_some()
        {
            return Err(NexusError::new("extension.name_conflict"));
        }
        plan_json_set(
            &read,
            &["mcpServers", name],
            serde_json::json!({ "httpUrl": url }),
        )
    }
    fn plan_remove(&self, ctx: &HostContext, name: &str) -> NexusResult<ConfigEdit> {
        plan_json_remove(&user_settings(ctx), &["mcpServers", name])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(settings: &str) -> (tempfile::TempDir, HostContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".gemini")).unwrap();
        std::fs::write(dir.path().join(".gemini/settings.json"), settings).unwrap();
        let ctx = HostContext::isolated(dir.path(), &dir.path().join("data"));
        (dir, ctx)
    }

    #[test]
    fn excluded_servers_are_disabled_and_toggle_round_trips() {
        let (_d, ctx) = setup("{\n  \"mcpServers\": {\n    \"a\": {\"command\": \"node\"},\n    \"b\": {\"httpUrl\": \"https://b.example.com/mcp\"}\n  },\n  \"mcp\": {\"excluded\": [\"a\"]},\n  \"security\": {\"auth\": {\"selectedType\": \"oauth-personal\"}}\n}\n");
        let configs = Gemini.read_configs(&ctx);
        let ext = Gemini.observe_extensions(&ctx, &configs);
        assert_eq!(
            ext.mcp.iter().find(|m| m.name == "a").unwrap().enabled,
            Some(false)
        );
        assert_eq!(
            ext.mcp.iter().find(|m| m.name == "b").unwrap().transport,
            McpTransport::Http
        );
        let edit = Gemini.plan_toggle(&ctx, "a", true).unwrap();
        let v = crate::config::jsonc::Document::parse(
            &String::from_utf8(edit.after).unwrap(),
            crate::config::jsonc::Dialect::Jsonc,
        )
        .unwrap()
        .value();
        assert_eq!(v["mcp"]["excluded"], serde_json::json!([]));
        assert_eq!(
            v["security"]["auth"]["selectedType"],
            serde_json::json!("oauth-personal")
        );
        let edit = Gemini.plan_toggle(&ctx, "b", false).unwrap();
        assert!(String::from_utf8(edit.after).unwrap().contains("\"b\""));
    }

    #[test]
    fn env_override_is_explained() {
        let (_d, mut ctx) = setup("{\"model\": {\"name\": \"gemini-a\"}}");
        ctx.set_env("GEMINI_MODEL", "gemini-b");
        let configs = Gemini.read_configs(&ctx);
        let m = Gemini.observe_model(&ctx, &configs);
        assert_eq!(m.effective.as_deref(), Some("gemini-b"));
        assert!(m
            .layers
            .iter()
            .any(|l| l.kind == ModelSourceKind::UserConfig
                && l.value.as_deref() == Some("gemini-a")
                && !l.effective));
    }

    #[test]
    fn legacy_string_model_is_read_but_not_rewritten() {
        let (_d, ctx) = setup("{\"model\": \"gemini-old\"}");
        assert_eq!(
            Gemini.user_config_model(&ctx).as_deref(),
            Some("gemini-old")
        );
        assert_eq!(
            Gemini.plan_model(&ctx, "gemini-new").unwrap_err().code,
            "config.unexpected_shape"
        );
    }

    #[test]
    fn allowlist_policy_blocks_toggle() {
        let (_d, ctx) = setup(
            "{\"mcpServers\": {\"a\": {\"command\": \"x\"}}, \"mcp\": {\"allowed\": [\"a\"]}}",
        );
        assert_eq!(
            Gemini.plan_toggle(&ctx, "a", false).unwrap_err().code,
            "extension.allowlist_policy"
        );
    }
}
