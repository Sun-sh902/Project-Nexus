//! Claude Code. Evidence (code.claude.com/docs/en/settings and /setup, 2026-10-27):
//! user settings `~/.claude/settings.json` (strict JSON; a broken file is reported but the session
//! can continue without it), `CLAUDE_CONFIG_DIR` relocates it, managed settings outrank user
//! settings, `ANTHROPIC_MODEL` overrides the `model` key from any file and
//! `ANTHROPIC_DEFAULT_MODEL` applies only when no file sets `model`. `~/.claude.json` is
//! Claude-owned state holding user-scope MCP servers — Nexus reads it but never writes it.
//! Install: native script (disabled in Nexus), Homebrew cask `claude-code`, WinGet
//! `Anthropic.ClaudeCode`, npm `@anthropic-ai/claude-code`. Native installs self-update with
//! `claude update`.

use super::codex::{basename, finish_layers};
use super::*;
use crate::config::{plan_json_set, ConfigEdit};
use crate::discovery::{
    command_resolvable, config_value, homebrew_path_version, npm_package_version, read_config,
    skills_in, url_origin,
};
use crate::lifecycle::{Mechanism, Op, Recipe};
use std::path::PathBuf;

pub struct Claude;

fn config_dir(ctx: &HostContext) -> PathBuf {
    match ctx.env_value("CLAUDE_CONFIG_DIR") {
        Some(v) if Path::new(v).is_absolute() => PathBuf::from(v),
        _ => ctx.home.join(".claude"),
    }
}

fn state_file(ctx: &HostContext) -> PathBuf {
    match ctx.env_value("CLAUDE_CONFIG_DIR") {
        Some(v) if Path::new(v).is_absolute() => PathBuf::from(v).join(".claude.json"),
        _ => ctx.home.join(".claude.json"),
    }
}

fn managed_path(os: Os) -> Option<PathBuf> {
    match os {
        Os::Macos => Some(PathBuf::from(
            "/Library/Application Support/ClaudeCode/managed-settings.json",
        )),
        Os::Windows => Some(PathBuf::from(
            "C:\\Program Files\\ClaudeCode\\managed-settings.json",
        )),
        Os::Linux => Some(PathBuf::from("/etc/claude-code/managed-settings.json")),
        Os::Other => None,
    }
}

fn user_settings(ctx: &HostContext) -> ReadConfig {
    read_config(
        ctx,
        "claude.user",
        config_dir(ctx).join("settings.json"),
        ConfigScope::User,
        ConfigFormat::Json,
        true,
    )
}

impl AgentAdapter for Claude {
    fn id(&self) -> AgentId {
        AgentId::Claude
    }
    fn name(&self) -> &'static str {
        "Claude Code"
    }
    fn vendor(&self) -> &'static str {
        "Anthropic"
    }
    fn executable_names(&self) -> &'static [&'static str] {
        &["claude"]
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
            // Native layout: ~/.local/bin/claude → ~/.local/share/claude/versions/<version>
            InstallSource::NativeInstaller => resolved
                .file_name()
                .and_then(|n| extract_version(&n.to_string_lossy()))
                .map(|v| {
                    Observed::ok(
                        v,
                        EvidenceSource::InstallLayout,
                        Some(ctx.display_path(resolved)),
                        now,
                    )
                }),
            InstallSource::Npm => {
                npm_package_version(ctx, resolved, "@anthropic-ai/claude-code", now)
            }
            InstallSource::Homebrew => homebrew_path_version(ctx, resolved, now),
            _ => None,
        }
    }

    fn read_configs(&self, ctx: &HostContext) -> Vec<ReadConfig> {
        let mut v = vec![
            user_settings(ctx),
            read_config(
                ctx,
                "claude.state",
                state_file(ctx),
                ConfigScope::AgentState,
                ConfigFormat::Json,
                true,
            ),
        ];
        if let Some(p) = managed_path(ctx.os) {
            v.push(read_config(
                ctx,
                "claude.managed",
                p,
                ConfigScope::Managed,
                ConfigFormat::Json,
                false,
            ));
        }
        v
    }

    fn observe_model(&self, ctx: &HostContext, configs: &[ReadConfig]) -> ModelObservation {
        let user = find_read(configs, "claude.user").and_then(config_value);
        let managed = find_read(configs, "claude.managed").and_then(config_value);
        let loc = |id: &str| {
            find_read(configs, id)
                .map(|r| r.source.location.clone())
                .unwrap_or_default()
        };
        let get = |v: &Option<serde_json::Value>, path: &[&str]| -> Option<String> {
            let mut cur = v.as_ref()?;
            for p in path {
                cur = cur.get(*p)?;
            }
            cur.as_str().map(String::from)
        };
        let mut layers = vec![ModelLayer {
            kind: ModelSourceKind::Environment,
            location: "ANTHROPIC_MODEL".into(),
            value: ctx.env_value("ANTHROPIC_MODEL").map(String::from),
            effective: false,
            uncertain: false,
        }];
        for (name, file) in crate::discovery::scan_shell_profiles(ctx, &["ANTHROPIC_MODEL"]) {
            layers.push(ModelLayer {
                kind: ModelSourceKind::Environment,
                location: format!("{file} ({name})"),
                value: None,
                effective: false,
                uncertain: true,
            });
        }
        layers.push(ModelLayer {
            kind: ModelSourceKind::ManagedConfig,
            location: loc("claude.managed"),
            value: get(&managed, &["model"]),
            effective: false,
            uncertain: false,
        });
        layers.push(ModelLayer {
            kind: ModelSourceKind::ConfigEnvBlock,
            location: format!("{} (env.ANTHROPIC_MODEL)", loc("claude.user")),
            value: get(&user, &["env", "ANTHROPIC_MODEL"]),
            effective: false,
            uncertain: false,
        });
        layers.push(ModelLayer {
            kind: ModelSourceKind::UserConfig,
            location: loc("claude.user"),
            value: get(&user, &["model"]),
            effective: false,
            uncertain: false,
        });
        layers.push(ModelLayer {
            kind: ModelSourceKind::Environment,
            location: "ANTHROPIC_DEFAULT_MODEL".into(),
            value: ctx.env_value("ANTHROPIC_DEFAULT_MODEL").map(String::from),
            effective: false,
            uncertain: false,
        });
        let hint = if ctx.env_value("CLAUDE_CODE_USE_BEDROCK").is_some() {
            Some("bedrock".into())
        } else if ctx.env_value("CLAUDE_CODE_USE_VERTEX").is_some() {
            Some("vertex".into())
        } else {
            Some("anthropic".into())
        };
        finish_layers(layers, hint)
    }

    fn observe_auth(&self, ctx: &HostContext, configs: &[ReadConfig]) -> AuthObservation {
        let mut evidence = Vec::new();
        let mut locations = Vec::new();
        // Presence of the account record only; the credential itself lives in the OS store.
        if let Some(state) = find_read(configs, "claude.state").and_then(config_value) {
            if state
                .get("oauthAccount")
                .map(|v| v.is_object())
                .unwrap_or(false)
            {
                evidence.push(AuthEvidence::AgentLoginPresent);
                locations.push(ctx.display_path(&state_file(ctx)));
            }
        }
        if ctx.os == Os::Macos {
            evidence.push(AuthEvidence::OsStoreUnverified);
        }
        if ctx.env_present("ANTHROPIC_API_KEY") || ctx.env_present("ANTHROPIC_AUTH_TOKEN") {
            evidence.push(AuthEvidence::ApiKeyEnvPresent);
        }
        if ctx.env_present("CLAUDE_CODE_USE_BEDROCK") || ctx.env_present("CLAUDE_CODE_USE_VERTEX") {
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
        let (mcp, mcp_status) = match find_read(configs, "claude.state") {
            Some(r) if r.source.status == ObsStatus::Observed => {
                let servers = config_value(r)
                    .and_then(|v| v.get("mcpServers").and_then(|m| m.as_object()).cloned())
                    .unwrap_or_default();
                let list = servers
                    .iter()
                    .map(|(name, entry)| {
                        let command = entry.get("command").and_then(|c| c.as_str());
                        let url = entry.get("url").and_then(|c| c.as_str());
                        let transport = match entry.get("type").and_then(|t| t.as_str()) {
                            Some("http") => McpTransport::Http,
                            Some("sse") => McpTransport::Sse,
                            Some("stdio") => McpTransport::Stdio,
                            _ if command.is_some() => McpTransport::Stdio,
                            _ => McpTransport::Unknown,
                        };
                        McpServer {
                            name: crate::core::clean_name(name),
                            transport,
                            enabled: None,
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
                            can_toggle: false,
                        }
                    })
                    .collect();
                (list, ObsStatus::Observed)
            }
            Some(r) => (Vec::new(), r.source.status),
            None => (Vec::new(), ObsStatus::NotChecked),
        };
        let (skills, skills_status) = skills_in(ctx, &config_dir(ctx).join("skills"));
        ExtensionObservation {
            mcp,
            mcp_status,
            skills,
            skills_status,
        }
    }

    fn override_variables(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("ANTHROPIC_MODEL", "model"),
            ("ANTHROPIC_BASE_URL", "provider"),
            ("CLAUDE_CONFIG_DIR", "configLocation"),
            ("CLAUDE_CODE_USE_BEDROCK", "provider"),
            ("CLAUDE_CODE_USE_VERTEX", "provider"),
        ]
    }

    fn runtime_requirement(&self, _source: Option<InstallSource>) -> Option<RuntimeRequirement> {
        // Every documented install method ships a native binary that does not use Node at runtime.
        None
    }

    fn capabilities(&self, os: Os, snapshot: &AgentSnapshot) -> Vec<Capability> {
        let level = platform_support_level(os);
        let source = snapshot.primary_installation().map(|i| i.source);
        let managed_update = matches!(
            source,
            Some(InstallSource::Npm | InstallSource::Homebrew | InstallSource::NativeInstaller)
        );
        let managed_uninstall =
            matches!(source, Some(InstallSource::Npm | InstallSource::Homebrew));
        vec![
            cap("discovery", level, None),
            cap("health", level, None),
            cap("model.write", level, None),
            cap("mcp.read", level, None),
            cap(
                "mcp.toggle",
                Support::Unsupported,
                Some("capability.claudeMcpAgentOwned"),
            ),
            cap(
                "mcp.addRemote",
                Support::Unsupported,
                Some("capability.claudeMcpAgentOwned"),
            ),
            cap("skills.read", level, None),
            cap(
                "skills.write",
                Support::Unsupported,
                Some("capability.skillsWriteDeferred"),
            ),
            cap(
                "lifecycle.install",
                if os == Os::Windows {
                    Support::Unsupported
                } else {
                    level
                },
                if os == Os::Windows {
                    Some("capability.wingetUnverified")
                } else {
                    None
                },
            ),
            cap(
                "lifecycle.update",
                if managed_update {
                    level
                } else {
                    Support::Unsupported
                },
                if managed_update {
                    None
                } else {
                    Some("capability.unknownInstaller")
                },
            ),
            cap(
                "lifecycle.uninstall",
                if managed_uninstall {
                    level
                } else {
                    Support::Unsupported
                },
                if managed_uninstall {
                    None
                } else {
                    Some("capability.nativeUninstallManual")
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

    fn recipes(&self) -> Vec<Recipe> {
        vec![
            Recipe {
                id: "claude.brew",
                agent: AgentId::Claude,
                mechanism: Mechanism::HomebrewCask {
                    token: "claude-code",
                },
                ops: &[Op::Install, Op::Update, Op::Uninstall],
                evidence: "https://code.claude.com/docs/en/setup",
                runs_package_scripts: false,
            },
            Recipe {
                id: "claude.npm",
                agent: AgentId::Claude,
                mechanism: Mechanism::Npm {
                    package: "@anthropic-ai/claude-code",
                },
                ops: &[Op::Install, Op::Update, Op::Uninstall],
                evidence: "https://code.claude.com/docs/en/setup",
                runs_package_scripts: true,
            },
            Recipe {
                id: "claude.native-update",
                agent: AgentId::Claude,
                mechanism: Mechanism::AgentSelfUpdate { args: &["update"] },
                ops: &[Op::Update],
                evidence: "https://code.claude.com/docs/en/setup",
                runs_package_scripts: false,
            },
        ]
    }
}

impl ModelTarget for Claude {
    fn provider_support(&self, kind: ProviderKind) -> Support {
        if kind == ProviderKind::Anthropic {
            Support::Supported
        } else {
            Support::Unsupported
        }
    }
    fn plan_model(&self, ctx: &HostContext, identifier: &str) -> NexusResult<ConfigEdit> {
        plan_json_set(
            &user_settings(ctx),
            &["model"],
            serde_json::Value::String(identifier.to_string()),
        )
    }
    fn user_config_model(&self, ctx: &HostContext) -> Option<String> {
        config_value(&user_settings(ctx))
            .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(settings: Option<&str>, state: Option<&str>) -> (tempfile::TempDir, HostContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        if let Some(s) = settings {
            std::fs::write(dir.path().join(".claude/settings.json"), s).unwrap();
        }
        if let Some(s) = state {
            std::fs::write(dir.path().join(".claude.json"), s).unwrap();
        }
        let ctx = HostContext::isolated(dir.path(), &dir.path().join("data"));
        (dir, ctx)
    }

    #[test]
    fn env_variable_overrides_settings_model() {
        let (_d, mut ctx) = setup(Some("{\"model\": \"opus\"}"), None);
        ctx.set_env("ANTHROPIC_MODEL", "sonnet");
        let configs = Claude.read_configs(&ctx);
        let m = Claude.observe_model(&ctx, &configs);
        assert_eq!(m.effective.as_deref(), Some("sonnet"));
        assert_eq!(m.effective_kind, ModelSourceKind::Environment);
        let user = m
            .layers
            .iter()
            .find(|l| l.kind == ModelSourceKind::UserConfig)
            .unwrap();
        assert_eq!(user.value.as_deref(), Some("opus"));
        assert!(!user.effective);
    }

    #[test]
    fn default_model_env_only_applies_when_no_file_sets_model() {
        let (_d, mut ctx) = setup(Some("{\"theme\": \"dark\"}"), None);
        ctx.set_env("ANTHROPIC_DEFAULT_MODEL", "haiku");
        let configs = Claude.read_configs(&ctx);
        assert_eq!(
            Claude.observe_model(&ctx, &configs).effective.as_deref(),
            Some("haiku")
        );
        let (_d, mut ctx) = setup(Some("{\"model\": \"opus\"}"), None);
        ctx.set_env("ANTHROPIC_DEFAULT_MODEL", "haiku");
        let configs = Claude.read_configs(&ctx);
        assert_eq!(
            Claude.observe_model(&ctx, &configs).effective.as_deref(),
            Some("opus")
        );
    }

    #[test]
    fn settings_env_block_outranks_model_key() {
        let (_d, ctx) = setup(
            Some("{\"model\": \"opus\", \"env\": {\"ANTHROPIC_MODEL\": \"haiku\"}}"),
            None,
        );
        let configs = Claude.read_configs(&ctx);
        let m = Claude.observe_model(&ctx, &configs);
        assert_eq!(
            (m.effective.as_deref(), m.effective_kind),
            (Some("haiku"), ModelSourceKind::ConfigEnvBlock)
        );
    }

    #[test]
    fn auth_presence_without_reading_credentials() {
        let (_d, mut ctx) = setup(None, Some("{\"oauthAccount\": {\"emailAddress\": \"x@y\"}, \"mcpServers\": {\"fs\": {\"type\": \"stdio\", \"command\": \"npx\", \"args\": [\"-y\", \"server\"], \"env\": {\"TOKEN\": \"sk-secretsecretsecretsecret1234\"}}}}"));
        ctx.set_env("ANTHROPIC_API_KEY", "sk-ant-should-not-be-read");
        let configs = Claude.read_configs(&ctx);
        let auth = Claude.observe_auth(&ctx, &configs);
        assert!(auth.evidence.contains(&AuthEvidence::AgentLoginPresent));
        assert!(auth.evidence.contains(&AuthEvidence::ApiKeyEnvPresent));
        let ext = Claude.observe_extensions(&ctx, &configs);
        assert_eq!(ext.mcp[0].env_names, vec!["TOKEN".to_string()]);
        assert!(!ext.mcp[0].can_toggle);
        let json = serde_json::to_string(&(auth, ext)).unwrap();
        assert!(!json.contains("sk-") && !json.contains("x@y"));
    }

    #[test]
    fn model_write_keeps_strict_json() {
        let (_d, ctx) = setup(Some("{\n  \"theme\": \"dark\"\n}\n"), None);
        let edit = Claude.plan_model(&ctx, "opus").unwrap();
        let out = String::from_utf8(edit.after).unwrap();
        crate::config::jsonc::Document::parse(&out, crate::config::jsonc::Dialect::Json).unwrap();
        assert!(out.contains("\"theme\": \"dark\""));
    }
}
