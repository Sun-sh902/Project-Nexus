//! End-to-end scenarios through the application layer (`App`), using isolated fake homes,
//! fake executables and a fake npm. Nothing here touches the developer's real Agents, configs,
//! credentials or package managers. Covers the acceptance scenarios in the backend brief.

#![cfg(unix)]

use crate::actions::ActionRequest;
use crate::app::test_support::{app, Fixed};
use crate::app::App;
use crate::core::NexusError;
use crate::domain::*;
use crate::models::AssignmentMode;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::PathBuf;

struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    fn new() -> Self {
        Home { dir: tempfile::tempdir().unwrap() }
    }
    fn p(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn write(&self, rel: &str, text: &str) {
        let p = self.p(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    fn exe(&self, rel: &str, body: &str) {
        self.write(rel, body);
        fs::set_permissions(self.p(rel), fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn link(&self, target: &str, rel: &str) {
        let p = self.p(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        symlink(target, p).unwrap();
    }
    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.p(rel)).unwrap()
    }
    /// Node from a Homebrew-style keg so its version is read from the layout, not executed.
    fn node(&self, version: &str) {
        self.exe(&format!("opt/Cellar/node/{version}/bin/node"), "\x7fELF");
        self.link(&self.p(&format!("opt/Cellar/node/{version}/bin/node")).to_string_lossy(), "bin/node");
    }
    fn npm_package(&self, package: &str, bin: &str, version: &str, entry: &str) {
        let root = format!(".npm-global/lib/node_modules/{package}");
        self.write(&format!("{root}/package.json"), &format!("{{\"name\": \"{package}\", \"version\": \"{version}\"}}"));
        self.exe(&format!("{root}/{entry}"), "#!/usr/bin/env node\n");
        self.link(&format!("../lib/node_modules/{package}/{entry}"), &format!(".npm-global/bin/{bin}"));
    }
    fn gemini(&self, version: &str) {
        self.npm_package("@google/gemini-cli", "gemini", version, "bundle/gemini.js");
        self.write(".gemini/oauth_creds.json", "{\"access_token\": \"ya29.CANARY-TOKEN-VALUE-SHOULD-NEVER-LEAK\"}");
    }
    fn claude(&self, version: &str) {
        self.exe(&format!(".local/share/claude/versions/{version}"), "\x7fELF");
        self.link(&self.p(&format!(".local/share/claude/versions/{version}")).to_string_lossy(), ".local/bin/claude");
        self.write(".claude.json", "{\"oauthAccount\": {\"emailAddress\": \"private@example.com\"}, \"mcpServers\": {}}");
    }
    /// Fake npm: `view` prints a version; `install`/`uninstall` mutate the fake prefix.
    fn fake_npm(&self, view_version: &str, fail_install: bool) {
        let prefix = self.p(".npm-global");
        let script = format!(
            r#"#!/bin/sh
case "$1" in
  view) echo '"{view_version}"' ;;
  install)
    {fail}
    spec=$(eval echo \${{$#}})
    pkg=${{spec%@*}}; ver=${{spec##*@}}
    case "$pkg" in
      @google/gemini-cli) bin=gemini; entry=bundle/gemini.js ;;
      @openai/codex) bin=codex; entry=bin/codex.js ;;
      *) echo "unknown package" >&2; exit 1 ;;
    esac
    mkdir -p "{prefix}/lib/node_modules/$pkg/$(dirname $entry)" "{prefix}/bin"
    printf '{{"name": "%s", "version": "%s"}}' "$pkg" "$ver" > "{prefix}/lib/node_modules/$pkg/package.json"
    printf '#!/usr/bin/env node\n' > "{prefix}/lib/node_modules/$pkg/$entry"; chmod 755 "{prefix}/lib/node_modules/$pkg/$entry"
    ln -sf "../lib/node_modules/$pkg/$entry" "{prefix}/bin/$bin" ;;
  uninstall)
    [ "$3" = "--prefix" ] || {{ echo "refusing uninstall without --prefix" >&2; exit 1; }}
    pkg=$(eval echo \${{$#}})
    rm -rf "$4/lib/node_modules/$pkg"; rm -f "$4/bin/gemini" ;;
esac
"#,
            prefix = prefix.display(),
            fail = if fail_install { "echo 'npm ERR! network sk-ant-api03-CANARYCANARYCANARY0123456789' >&2; exit 1" } else { "" }
        );
        self.exe("bin/npm", &script);
    }
    fn app(&self, confirm: bool) -> App {
        let mut a = app(self.dir.path(), confirm);
        a.ctx.set_env("PATH", &format!("{}:{}:{}", self.p("bin").display(), self.p(".npm-global/bin").display(), self.p(".local/bin").display()));
        a
    }
}

fn report(o: &crate::app::Overview, id: AgentId) -> &crate::app::AgentReport {
    o.agents.iter().find(|a| a.snapshot.agent_id == id).unwrap()
}

fn confirm(app: &App, req: ActionRequest) -> Result<crate::actions::OperationResult, NexusError> {
    let plan = app.prepare(req)?;
    app.confirm(&plan.id, &plan.digest)
}

#[test]
fn scenario_1_no_agents_installed() {
    let home = Home::new();
    let app = home.app(true);
    let o = app.overview(true).unwrap();
    assert_eq!(o.agents.len(), 4);
    for r in &o.agents {
        assert_eq!(r.health.readiness, Readiness::Blocked, "{:?}", r.snapshot.agent_id);
        assert_eq!(r.findings[0].code, "notInstalled");
        assert!(r.snapshot.installations.is_empty());
    }
}

#[test]
fn scenario_2_and_3_installed_agents_with_versions_and_sources() {
    let home = Home::new();
    home.node("26.10.0");
    home.gemini("0.64.0");
    home.write(".gemini/settings.json", "{\"security\": {\"auth\": {\"selectedType\": \"oauth-personal\"}}}");
    home.claude("2.1.311");
    let o = home.app(true).overview(true).unwrap();
    let g = report(&o, AgentId::Gemini);
    assert_eq!(g.health.readiness, Readiness::Ready, "{:#?}", g.health.checks.iter().filter(|c| c.outcome != crate::health::Outcome::Pass).collect::<Vec<_>>());
    let gi = g.snapshot.primary_installation().unwrap();
    assert_eq!((gi.source, gi.version.value.as_deref(), gi.version.source), (InstallSource::Npm, Some("0.64.0"), EvidenceSource::PackageManifest));
    let c = report(&o, AgentId::Claude);
    let ci = c.snapshot.primary_installation().unwrap();
    assert_eq!((ci.source, ci.version.value.as_deref()), (InstallSource::NativeInstaller, Some("2.1.311")));
    assert!(matches!(c.health.readiness, Readiness::Ready | Readiness::Attention));
    // Confirmed absence is distinct from unknown evidence.
    assert_eq!(report(&o, AgentId::Codex).health.readiness, Readiness::Blocked);
    // Nothing secret crosses IPC.
    let json = serde_json::to_string(&o).unwrap();
    assert!(!json.contains("CANARY") && !json.contains("private@example.com"));
}

#[test]
fn scenario_4_invalid_configuration_blocks_with_manual_guidance() {
    let home = Home::new();
    home.node("26.10.0");
    home.npm_package("@openai/codex", "codex", "0.164.0", "bin/codex.js");
    home.write(".codex/config.toml", "model = \"gpt\"\n[broken\n");
    let o = home.app(true).overview(true).unwrap();
    let r = report(&o, AgentId::Codex);
    assert_eq!(r.health.readiness, Readiness::Blocked);
    let f = r.findings.iter().find(|f| f.code == "configMalformed").unwrap();
    assert_eq!(f.manual, Some("manual.fixConfig"));
    // A malformed file is read-only for ordinary edits.
    let app = home.app(true);
    app.set_assignment(AgentId::Codex, AssignmentMode::Direct, Some("observed.builtin.openai.gpt"), None).ok();
    assert!(app.prepare(ActionRequest::SetMcpEnabled { agent_id: AgentId::Codex, name: "x".into(), enabled: false }).is_err());
    assert_eq!(home.read(".codex/config.toml"), "model = \"gpt\"\n[broken\n");
}

#[test]
fn scenario_5_missing_prerequisite() {
    let home = Home::new();
    home.gemini("0.64.0");
    let o = home.app(true).overview(true).unwrap();
    let r = report(&o, AgentId::Gemini);
    assert_eq!(r.health.readiness, Readiness::Blocked);
    assert!(r.findings.iter().any(|f| f.code == "runtimeMissing" && f.manual == Some("manual.installRuntime")));
    let home2 = Home::new();
    home2.node("18.19.0");
    home2.gemini("0.64.0");
    let o = home2.app(true).overview(true).unwrap();
    assert!(report(&o, AgentId::Gemini).findings.iter().any(|f| f.code == "runtimeTooOld"));
}

#[test]
fn scenario_6_model_override_is_explained_with_its_source() {
    let home = Home::new();
    home.claude("2.1.311");
    home.write(".claude/settings.json", "{\n  \"theme\": \"dark\",\n  \"model\": \"opus\"\n}\n");
    home.write(".zshrc", "export ANTHROPIC_MODEL=sonnet\n");
    let mut app = home.app(true);
    app.ctx.set_env("ANTHROPIC_MODEL", "haiku");
    let o = app.overview(true).unwrap();
    let r = report(&o, AgentId::Claude);
    assert_eq!(r.snapshot.model.effective.as_deref(), Some("haiku"));
    assert_eq!(r.snapshot.model.effective_kind, ModelSourceKind::Environment);
    assert!(r.snapshot.model.layers.iter().any(|l| l.uncertain && l.location.contains(".zshrc")));
    let f = r.findings.iter().find(|f| f.code == "environmentOverride").unwrap();
    assert!(f.params.iter().any(|(k, v)| k == "variables" && v.contains("ANTHROPIC_MODEL")));
    // After assigning opus, the override is reported as displacing Nexus's applied choice.
    app.set_assignment(AgentId::Claude, AssignmentMode::Direct, Some("builtin.anthropic.opus"), None).unwrap();
    let res = confirm(&app, ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap();
    assert_eq!(res.status, "succeeded", "{res:#?}");
    assert!(res.warnings.contains(&"warning.effectiveOverridden"));
    assert!(res.verification.checks.iter().any(|c| c.key == "verify.effectiveModel" && c.outcome == "overridden"));
    let o = app.overview(true).unwrap();
    assert!(report(&o, AgentId::Claude).findings.iter().any(|f| f.code == "modelOverridden"));
}

#[test]
fn model_is_not_written_into_a_config_that_routes_elsewhere() {
    let home = Home::new();
    home.node("26.10.0");
    home.npm_package("@openai/codex", "codex", "0.164.0", "bin/codex.js");
    home.write(".codex/config.toml", "model_provider = \"ollama\"\nmodel = \"llama3\"\n");
    let app = home.app(true);
    app.create_model("builtin.openai", "gpt-x", None).unwrap();
    let id = app.models_view().unwrap().models.into_iter().find(|m| m.identifier == "gpt-x").unwrap().id;
    app.set_assignment(AgentId::Codex, AssignmentMode::Direct, Some(&id), None).unwrap();
    assert_eq!(app.prepare(ActionRequest::ApplyModel { agent_id: AgentId::Codex }).unwrap_err().code, "model.routing_mismatch");
    let mut app = home.app(true);
    home.write(".claude/settings.json", "{}");
    home.claude("2.1.311");
    app.ctx.set_env("CLAUDE_CODE_USE_BEDROCK", "1");
    app.set_assignment(AgentId::Claude, AssignmentMode::Direct, Some("builtin.anthropic.opus"), None).unwrap();
    assert_eq!(app.prepare(ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap_err().code, "model.routing_mismatch");
    assert_eq!(home.read(".claude/settings.json"), "{}");
}

#[test]
fn scenario_7_extension_conflict_repaired_by_trusted_toggle() {
    let home = Home::new();
    home.node("26.10.0");
    home.npm_package("@openai/codex", "codex", "0.164.0", "bin/codex.js");
    let original = "# personal settings\nmodel = \"gpt-x\"\n\n[mcp_servers.files]\ncommand = \"/nonexistent/mcp-files\"\nargs = [\"--root\", \"/\"]\nenv = { TOKEN = \"sk-abcdefghijklmnopqrstuvwxyz012345\" }\n\n[mcp_servers.docs]\nurl = \"https://docs.example.com/mcp\"\n";
    home.write(".codex/config.toml", original);
    let app = home.app(true);
    let o = app.overview(true).unwrap();
    let r = report(&o, AgentId::Codex);
    assert_eq!(r.health.readiness, Readiness::Attention);
    let f = r.findings.iter().find(|f| f.code == "mcpCommandMissing").unwrap();
    assert_eq!(f.repairs[0].action, "setMcpEnabled");
    let res = confirm(&app, ActionRequest::SetMcpEnabled { agent_id: AgentId::Codex, name: "files".into(), enabled: false }).unwrap();
    assert_eq!((res.status, res.verification.outcome), ("succeeded", "passed"));
    let after = home.read(".codex/config.toml");
    assert!(after.contains("# personal settings") && after.contains("[mcp_servers.docs]") && after.contains("TOKEN = \"sk-abc"));
    assert!(after.contains("enabled = false"));
    assert_eq!(report(&app.overview(true).unwrap(), AgentId::Codex).health.readiness, Readiness::Ready);
}

#[test]
fn scenario_8_confirmation_is_required_and_single_use() {
    let home = Home::new();
    home.claude("2.1.311");
    home.write(".claude/settings.json", "{\"theme\": \"dark\"}");
    let app = home.app(false);
    app.set_assignment(AgentId::Claude, AssignmentMode::Direct, Some("builtin.anthropic.sonnet"), None).unwrap();
    let plan = app.prepare(ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap();
    assert!(plan.config.as_ref().unwrap().diff.iter().any(|d| d.kind == '+' && d.text.contains("sonnet")));
    assert_eq!(app.confirm(&plan.id, &plan.digest).unwrap_err().code, "confirmation.denied");
    assert_eq!(home.read(".claude/settings.json"), "{\"theme\": \"dark\"}");
    // The plan was consumed: a replay cannot execute.
    assert_eq!(app.confirm(&plan.id, &plan.digest).unwrap_err().code, "plan.not_found");
    // A forged digest is refused.
    let plan = app.prepare(ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap();
    assert_eq!(app.confirm(&plan.id, "0".repeat(64).as_str()).unwrap_err().code, "plan.digest_mismatch");
}

#[test]
fn scenario_9_failures_report_recovery_information() {
    // (a) External edit between preview and confirmation → stale plan, file untouched.
    let home = Home::new();
    home.claude("2.1.311");
    home.write(".claude/settings.json", "{\"theme\": \"dark\"}");
    let app = home.app(true);
    app.set_assignment(AgentId::Claude, AssignmentMode::Direct, Some("builtin.anthropic.sonnet"), None).unwrap();
    let plan = app.prepare(ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap();
    home.write(".claude/settings.json", "{\"theme\": \"light\"}");
    let res = app.confirm(&plan.id, &plan.digest).unwrap();
    assert_eq!(res.status, "failed");
    assert_eq!(res.error.unwrap().code, "config.concurrent_change");
    assert_eq!(home.read(".claude/settings.json"), "{\"theme\": \"light\"}");
    let ops = app.store().unwrap().operations(5).unwrap();
    assert_eq!((ops[0].status.as_str(), ops[0].error_code.as_deref()), ("failed", Some("config.concurrent_change")));

    // (b) Installer failure: process output is redacted, verification fails, nothing claimed.
    let home = Home::new();
    home.node("26.10.0");
    home.fake_npm("0.65.0", true);
    let app = home.app(true);
    let res = confirm(&app, ActionRequest::Install { agent_id: AgentId::Gemini, recipe_id: "gemini.npm".into() }).unwrap();
    assert_eq!((res.status, res.verification.outcome), ("failed", "failed"));
    let excerpt = res.output_excerpt.unwrap();
    assert!(excerpt.contains("npm ERR!") && !excerpt.contains("CANARY"));
}

#[test]
fn scenario_10_apply_verify_drift_reapply_and_restore() {
    let home = Home::new();
    home.claude("2.1.311");
    let original = "{\n  \"theme\": \"dark\",\n  \"permissions\": {\"allow\": [\"Bash(npm test)\"]}\n}\n";
    home.write(".claude/settings.json", original);
    let app = home.app(true);
    app.set_assignment(AgentId::Claude, AssignmentMode::Direct, Some("builtin.anthropic.opus"), None).unwrap();
    let res = confirm(&app, ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap();
    assert_eq!((res.status, res.verification.outcome), ("succeeded", "passed"));
    let backup = res.backup_id.clone().expect("existing file is backed up before writing");
    let written = home.read(".claude/settings.json");
    assert!(written.contains("\"model\": \"opus\"") && written.contains("Bash(npm test)"));
    assert!(report(&app.overview(true).unwrap(), AgentId::Claude).findings.iter().all(|f| f.code != "assignmentPending"));

    // Drift: someone edits the model; Nexus explains and offers the trusted reapply.
    home.write(".claude/settings.json", &written.replace("opus", "haiku"));
    let o = app.overview(true).unwrap();
    let drift = report(&o, AgentId::Claude).findings.iter().find(|f| f.code == "assignmentDrift").cloned().unwrap();
    assert_eq!(drift.repairs[0].action, "applyModel");
    let res = confirm(&app, ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap();
    assert_eq!(res.status, "succeeded");
    assert!(home.read(".claude/settings.json").contains("\"model\": \"opus\""));

    // Corruption: restore the known-good backup through a newly confirmed plan.
    home.write(".claude/settings.json", "{ \"model\": \"opus\", ");
    let o = app.overview(true).unwrap();
    let r = report(&o, AgentId::Claude);
    let f = r.findings.iter().find(|f| f.code == "configMalformed").unwrap();
    assert_eq!(f.repairs[0].action, "restoreBackup");
    let plan = app.prepare(ActionRequest::RestoreBackup { backup_id: backup.clone() }).unwrap();
    assert!(plan.warnings.contains(&"warning.replacesMalformed"));
    // Restores never send old or new file content to the renderer.
    assert!(plan.config.as_ref().unwrap().diff.is_empty());
    let res = app.confirm(&plan.id, &plan.digest).unwrap();
    assert_eq!(res.status, "succeeded");
    assert_eq!(home.read(".claude/settings.json"), original);
}

#[test]
fn scenario_e_trusted_install_update_uninstall_with_verification() {
    let home = Home::new();
    home.node("26.10.0");
    home.fake_npm("0.65.0", false);
    let app = home.app(true);
    let plan = app.prepare(ActionRequest::Install { agent_id: AgentId::Gemini, recipe_id: "gemini.npm".into() }).unwrap();
    let p = plan.process.as_ref().unwrap();
    assert_eq!(p.args.last().unwrap(), "@google/gemini-cli@0.65.0");
    assert!(p.runs_package_scripts && plan.warnings.contains(&"warning.packageScripts"));
    let res = app.confirm(&plan.id, &plan.digest).unwrap();
    assert_eq!((res.status, res.verification.outcome), ("succeeded", "passed"), "{res:?}");
    let o = app.overview(true).unwrap();
    let inst = report(&o, AgentId::Gemini).snapshot.primary_installation().unwrap().clone();
    assert_eq!(inst.version.value.as_deref(), Some("0.65.0"));
    assert!(inst.nexus_managed);
    // Up to date → no update plan; newer version → pinned update.
    assert_eq!(app.prepare(ActionRequest::Update { agent_id: AgentId::Gemini, installation_id: inst.id.clone() }).unwrap_err().code, "lifecycle.up_to_date");
    home.fake_npm("0.66.0", false);
    let res = confirm(&app, ActionRequest::Update { agent_id: AgentId::Gemini, installation_id: inst.id.clone() }).unwrap();
    assert_eq!(res.status, "succeeded");
    // Uninstall preserves configuration.
    home.write(".gemini/settings.json", "{\"theme\": \"x\"}");
    let res = confirm(&app, ActionRequest::Uninstall { agent_id: AgentId::Gemini, installation_id: inst.id }).unwrap();
    assert_eq!((res.status, res.verification.outcome), ("succeeded", "passed"));
    assert_eq!(home.read(".gemini/settings.json"), "{\"theme\": \"x\"}");
    // Installing over an existing installation is refused.
    home.gemini("0.64.0");
    assert_eq!(app.prepare(ActionRequest::Install { agent_id: AgentId::Gemini, recipe_id: "gemini.npm".into() }).unwrap_err().code, "lifecycle.already_installed");
}

#[test]
fn update_verification_survives_version_in_path_layouts() {
    // Claude's native and Homebrew layouts embed the version in the resolved path, so the
    // installation id changes on update. Verification must follow the stable launcher.
    let home = Home::new();
    home.claude("2.1.400");
    let app = home.app(true);
    let launcher = report(&app.overview(true).unwrap(), AgentId::Claude).snapshot.primary_installation().unwrap().executable.clone();
    let verify = |launcher: Option<String>| crate::actions::Verify::Lifecycle {
        agent: AgentId::Claude, op: crate::lifecycle::Op::Update, recipe: "claude.native-update", source: InstallSource::NativeInstaller,
        version: None, installation_id: Some("inst_before_update".into()), launcher, previous_version: Some("2.1.311".into()), recovers: None,
    };
    assert_eq!(app.verify(&verify(Some(launcher))).outcome, "passed");
    assert_eq!(app.verify(&verify(None)).outcome, "failed");
}

#[test]
fn scenario_interrupted_install_is_reconciled_then_recovered() {
    let home = Home::new();
    home.node("26.10.0");
    home.fake_npm("0.65.0", false);
    let db = home.p("nexus.db");
    {
        let store = crate::store::Store::open(&db).unwrap();
        store.begin_operation("op_crashed", "install", Some(AgentId::Gemini), "d", &serde_json::json!({"recipe": "gemini.npm", "version": "0.65.0"}), crate::core::Instant::now()).unwrap();
        store.step("op_crashed", "process.started", crate::core::Instant::now()).unwrap();
    }
    let mut ctx = crate::platform::HostContext::isolated(home.dir.path(), &home.p("nexus-data"));
    ctx.set_env("PATH", &format!("{}:{}", home.p("bin").display(), home.p(".npm-global/bin").display()));
    let app = App::new(ctx, crate::store::Store::open(&db).unwrap(), std::sync::Arc::new(crate::secrets::MemorySecretStore::default()), Box::new(Fixed(true))).unwrap();
    let o = app.overview(true).unwrap();
    let f = report(&o, AgentId::Gemini).findings.iter().find(|f| f.code == "operationInterrupted").unwrap().clone();
    assert_eq!(f.repairs[0].action, "recoverInstall");
    let res = confirm(&app, ActionRequest::RecoverInstall { operation_id: "op_crashed".into() }).unwrap();
    assert_eq!(res.status, "succeeded");
    let ops = app.store().unwrap().operations(10).unwrap();
    assert_eq!(ops.iter().find(|o| o.id == "op_crashed").unwrap().status, "recovered");
    assert!(report(&app.overview(true).unwrap(), AgentId::Gemini).findings.iter().all(|f| f.code != "operationInterrupted"));
}

#[test]
fn scenario_f_remote_mcp_add_and_managed_removal_preserve_user_entries() {
    let home = Home::new();
    home.node("26.10.0");
    home.gemini("0.64.0");
    home.write(".gemini/settings.json", "{\n  // user comment\n  \"mcpServers\": {\n    \"mine\": {\"command\": \"node\", \"args\": [\"server.js\"]}\n  }\n}\n");
    let app = home.app(true);
    let res = confirm(&app, ActionRequest::AddRemoteMcp { agent_id: AgentId::Gemini, name: "docs".into(), url: "https://docs.example.com/mcp".into() }).unwrap();
    assert_eq!((res.status, res.verification.outcome), ("succeeded", "passed"));
    let text = home.read(".gemini/settings.json");
    assert!(text.contains("// user comment") && text.contains("\"mine\"") && text.contains("httpUrl"));
    // User-owned entries can never be removed through the managed-removal action.
    assert_eq!(app.prepare(ActionRequest::RemoveManagedMcp { agent_id: AgentId::Gemini, name: "mine".into() }).unwrap_err().code, "extension.not_managed");
    let res = confirm(&app, ActionRequest::RemoveManagedMcp { agent_id: AgentId::Gemini, name: "docs".into() }).unwrap();
    assert_eq!(res.status, "succeeded");
    let text = home.read(".gemini/settings.json");
    assert!(text.contains("\"mine\"") && !text.contains("docs.example.com"));
}

#[test]
fn security_ipc_inputs_are_strictly_typed() {
    for bad in [
        r#"{"kind":"applyModel","agentId":"codex","extra":"x"}"#,
        r#"{"kind":"runCommand","command":"rm -rf /"}"#,
        r#"{"kind":"install","agentId":"codex","recipeId":"x","url":"https://evil"}"#,
        r#"{"kind":"applyModel","agentId":"../../etc"}"#,
    ] {
        assert!(serde_json::from_str::<ActionRequest>(bad).is_err(), "{bad}");
    }
    assert!(serde_json::from_str::<ActionRequest>(r#"{"kind":"setMcpEnabled","agentId":"codex","name":"x","enabled":true}"#).is_ok());
}

#[test]
fn security_hostile_inputs_never_reach_configuration() {
    let home = Home::new();
    home.node("26.10.0");
    home.npm_package("@openai/codex", "codex", "0.164.0", "bin/codex.js");
    home.write(".codex/config.toml", "model = \"gpt\"\n");
    let app = home.app(true);
    for (name, url) in [("x]\n[evil", "https://a.example.com"), ("ok", "http://a.example.com"), ("ok", "https://169.254.169.254/"), ("ok", "https://u:p@a.example.com")] {
        assert!(app.prepare(ActionRequest::AddRemoteMcp { agent_id: AgentId::Codex, name: name.into(), url: url.into() }).is_err(), "{name} {url}");
    }
    assert!(app.create_model("builtin.openai", "--flag", None).is_err());
    assert!(app.create_model("builtin.openai", "a;rm -rf ~", None).is_err());
    assert!(app.prepare(ActionRequest::Install { agent_id: AgentId::Codex, recipe_id: "codex.npm".into() }).is_err());
    assert!(app.prepare(ActionRequest::Install { agent_id: AgentId::Gemini, recipe_id: "curl-pipe-sh".into() }).is_err());
    assert_eq!(home.read(".codex/config.toml"), "model = \"gpt\"\n");
}

#[test]
fn security_secrets_go_only_to_the_credential_store() {
    let home = Home::new();
    let app = home.app(true);
    let p = app.create_provider("Local LLM", crate::models::ProviderKind::OpenaiCompatible, Some("http://localhost:11434/v1"), crate::models::AuthMode::NexusSecret, None).unwrap();
    app.set_provider_secret(&p.id, "sk-local-CANARY-SECRET-0123456789abcdef".into()).unwrap();
    let view = serde_json::to_string(&app.models_view().unwrap()).unwrap();
    assert!(!view.contains("CANARY"));
    assert!(view.contains("\"secretPresent\":true"));
    let db = format!("{:?}", app.store().unwrap().providers().unwrap());
    assert!(!db.contains("CANARY"));
    assert!(app.create_provider("Bad", crate::models::ProviderKind::OpenaiCompatible, Some("http://public.example.com"), crate::models::AuthMode::NexusSecret, None).is_err());
    app.delete_provider(&p.id).unwrap();
    assert!(app.secrets.get(p.secret_ref.as_deref().unwrap()).unwrap().is_none());
}

#[test]
fn opencode_ambiguous_config_blocks_mutation_with_explanation() {
    let home = Home::new();
    home.write(".config/opencode/opencode.json", "{\"model\": \"anthropic/a\"}");
    home.write(".config/opencode/opencode.jsonc", "{\"model\": \"openai/b\"}");
    let app = home.app(true);
    let o = app.overview(true).unwrap();
    let r = report(&o, AgentId::Opencode);
    assert_eq!(r.snapshot.capability("model.write"), Support::Unsupported);
    app.set_assignment(AgentId::Opencode, AssignmentMode::Direct, Some("builtin.anthropic.sonnet"), None).unwrap();
    assert_eq!(app.prepare(ActionRequest::ApplyModel { agent_id: AgentId::Opencode }).unwrap_err().code, "capability.unsupported");
}

/// Read-only discovery of the real machine. Opt-in: `cargo test real_machine -- --ignored --nocapture`.
#[test]
#[ignore]
fn real_machine_discovery_smoke() {
    let data = tempfile::tempdir().unwrap();
    let ctx = crate::platform::HostContext::detect(data.path().to_path_buf());
    let app = App::new(ctx, crate::store::Store::open_in_memory().unwrap(), std::sync::Arc::new(crate::secrets::MemorySecretStore::default()), Box::new(Fixed(false))).unwrap();
    let started = std::time::Instant::now();
    let o = app.overview(true).unwrap();
    println!("scan took {:?}; os {:?} {:?} {}", started.elapsed(), o.system.os, o.system.os_version.value, o.system.arch);
    for t in &o.system.tools {
        println!("  tool {:<8} present={} version={:?} status={:?}", t.key, t.present, t.version.value, t.version.status);
    }
    for r in &o.agents {
        println!("{:?}: {:?}", r.snapshot.agent_id, r.health.readiness);
        for i in &r.snapshot.installations {
            println!("    install {} -> {:?} {:?} v={:?} ({:?}) rank={:?}", i.executable, i.resolved, i.source, i.version.value, i.version.source, i.path_rank);
        }
        println!("    model effective={:?} via {:?}", r.snapshot.model.effective, r.snapshot.model.effective_kind);
        println!("    mcp={} skills={} auth={:?}", r.snapshot.extensions.mcp.len(), r.snapshot.extensions.skills.len(), r.snapshot.auth.evidence);
        for f in &r.findings {
            println!("    finding {} [{}] repairs={:?} manual={:?}", f.code, f.severity, f.repairs.iter().map(|r| r.action).collect::<Vec<_>>(), f.manual);
        }
    }
    let json = serde_json::to_string(&o).unwrap();
    assert!(!json.contains("sk-") , "possible secret in overview");
}

/// Regenerates the live-mode UI fixture from a deterministic fake home:
/// `cargo test export_live_ui_fixture -- --ignored`. Output: tests/fixtures/live/*.json.
#[test]
#[ignore]
fn export_live_ui_fixture() {
    let home = Home::new();
    home.node("26.10.0");
    home.fake_npm("0.65.0", false);
    home.npm_package("@openai/codex", "codex", "0.164.0", "bin/codex.js");
    home.write(".codex/auth.json", "{}");
    home.write(".codex/config.toml", "model = \"gpt-6.1-terra\"\n\n[mcp_servers.files]\ncommand = \"/nonexistent/mcp-files\"\n\n[mcp_servers.docs]\nurl = \"https://docs.example.com/mcp\"\n");
    home.write(".codex/skills/review/SKILL.md", "---\nname: review\ndescription: Reviews diffs\n---\n");
    home.claude("2.1.311");
    home.write(".claude/settings.json", "{\n  \"theme\": \"dark\"\n}\n");
    home.write(".config/opencode/opencode.json", "{\"model\": \"anthropic/claude-sonnet-5-5\"}");
    home.write(".config/opencode/opencode.jsonc", "{}");
    home.p("bin/opencode");
    let app = home.app(true);
    app.set_assignment(AgentId::Claude, AssignmentMode::Direct, Some("builtin.anthropic.opus"), None).unwrap();
    confirm(&app, ActionRequest::ApplyModel { agent_id: AgentId::Claude }).unwrap();
    // Drift so Claude shows a reapply repair.
    home.write(".claude/settings.json", &home.read(".claude/settings.json").replace("opus", "haiku"));
    let overview = app.overview(true).unwrap();
    let models = app.models_view().unwrap();
    let plan = app.prepare(ActionRequest::SetMcpEnabled { agent_id: AgentId::Codex, name: "files".into(), enabled: false }).unwrap();
    let result = app.confirm(&plan.id, &plan.digest).unwrap();
    let after = app.overview(true).unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/live");
    fs::create_dir_all(&out).unwrap();
    let scrub = |v: String| v.replace(&home.dir.path().to_string_lossy().to_string(), "/Users/example");
    for (name, value) in [("overview", serde_json::to_string_pretty(&overview).unwrap()), ("models", serde_json::to_string_pretty(&models).unwrap()), ("plan", serde_json::to_string_pretty(&plan).unwrap()), ("result", serde_json::to_string_pretty(&result).unwrap()), ("overview-after", serde_json::to_string_pretty(&after).unwrap())] {
        fs::write(out.join(format!("{name}.json")), scrub(value)).unwrap();
    }
}
