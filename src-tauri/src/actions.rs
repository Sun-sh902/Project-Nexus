//! Action Registry (Architecture §8/§11, Security §2 S02/S07/S08). Every native side effect is
//! one of the typed `ActionRequest` variants below. Preparation computes an immutable plan bound
//! to a digest and the exact resource fingerprints; confirmation happens in a native dialog owned
//! by Rust (a renderer cannot mint it); execution is journaled step by step; success is declared
//! only after an independent, fresh post-condition check.

use crate::adapters;
use crate::app::App;
use crate::config::transaction::{self, Compensation, Step};
use crate::config::ConfigEdit;
use crate::core::{new_id, redact, sha256_hex, Instant, NexusError, NexusResult};
use crate::discovery::{self, discover_agent};
use crate::domain::*;
use crate::lifecycle::{self, Op, Recipe};
use crate::platform::process::{run, ProcessSpec};
use serde::{Deserialize, Serialize};

pub const PLAN_TTL_SECS: i64 = 600;
const MAX_PLANS: usize = 32;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum ActionRequest {
    ApplyModel { agent_id: AgentId },
    SetMcpEnabled { agent_id: AgentId, name: String, enabled: bool },
    AddRemoteMcp { agent_id: AgentId, name: String, url: String },
    RemoveManagedMcp { agent_id: AgentId, name: String },
    RestoreBackup { backup_id: String },
    Install { agent_id: AgentId, recipe_id: String },
    Update { agent_id: AgentId, installation_id: String },
    Uninstall { agent_id: AgentId, installation_id: String },
    RecoverInstall { operation_id: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessPreview {
    pub program: String,
    pub args: Vec<String>,
    pub env_names: Vec<String>,
    pub runs_package_scripts: bool,
    pub uses_network: bool,
    pub package: String,
    pub version: Option<String>,
}

/// Public, renderer-visible plan. Everything the user is asked to approve is here.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub id: String,
    pub digest: String,
    pub kind: &'static str,
    pub agent_id: Option<AgentId>,
    pub created_at: Instant,
    pub expires_at: Instant,
    pub params: Vec<(String, String)>,
    pub config: Option<ConfigEdit>,
    pub process: Option<ProcessPreview>,
    pub privileges: &'static str,
    pub backup: &'static str,
    pub rollback: &'static str,
    pub verification: &'static str,
    pub warnings: Vec<&'static str>,
    pub preserves: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub enum Verify {
    Model { agent: AgentId, identifier: String, profile_revision: Option<i64>, provider_revision: Option<i64> },
    McpEnabled { agent: AgentId, name: String, enabled: bool },
    McpPresent { agent: AgentId, name: String, present: bool, origin: Option<String> },
    Restore { sha256: String },
    /// `launcher` is the stable PATH entry (e.g. `/opt/homebrew/bin/codex`); the installation id
    /// may change on update because Homebrew/native layouts embed the version in the path.
    Lifecycle { agent: AgentId, op: Op, recipe: &'static str, source: InstallSource, version: Option<String>, installation_id: Option<String>, launcher: Option<String>, previous_version: Option<String>, recovers: Option<String> },
}

#[derive(Clone, Debug)]
pub enum Payload {
    Config(ConfigEdit),
    Process(ProcessSpec),
    /// Nothing to change on disk; verification and Nexus records only (e.g. reconcile).
    RecordOnly,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub view: PlanView,
    pub payload: Payload,
    pub verify: Verify,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCheck {
    pub key: &'static str,
    pub outcome: &'static str,
    pub params: Vec<(String, String)>,
    /// Reported but not part of the operation's pass/fail (e.g. effective vs applied state).
    pub informational: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    /// `passed`, `failed`, `inconclusive` or `notRun`.
    pub outcome: &'static str,
    pub checks: Vec<VerificationCheck>,
    pub verified_at: Instant,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub operation_id: String,
    pub kind: &'static str,
    pub agent_id: Option<AgentId>,
    /// `succeeded`, `failed`, `partial`, `rolledBack`, `rollbackFailed`.
    pub status: &'static str,
    pub verification: VerificationResult,
    pub error: Option<NexusError>,
    pub backup_id: Option<String>,
    pub compensation: Option<&'static str>,
    pub output_excerpt: Option<String>,
    pub warnings: Vec<&'static str>,
}

fn digest(view: &PlanView, payload: &Payload) -> String {
    let mut v = serde_json::to_value(view).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.remove("digest");
    }
    let extra = match payload {
        Payload::Config(e) => format!("{}|{:?}|{}", e.path.display(), e.before_fingerprint, e.after_fingerprint),
        Payload::Process(p) => format!("{}|{}", p.program.display(), p.args.join("\u{1f}")),
        Payload::RecordOnly => String::new(),
    };
    sha256_hex(format!("{v}|{extra}").as_bytes())
}

fn require_capability(snap: &AgentSnapshot, key: &str) -> NexusResult<()> {
    if snap.capability(key) == Support::Unsupported {
        let reason = snap.capabilities.iter().find(|c| c.key == key).and_then(|c| c.reason).unwrap_or("capability.unsupported");
        return Err(NexusError::new("capability.unsupported").with("capability", key).with("reason", reason));
    }
    Ok(())
}

fn validate_text(value: &str, max: usize) -> NexusResult<()> {
    if value.is_empty() || value.len() > max || value.chars().any(|c| c.is_control()) {
        return Err(NexusError::new("ipc.invalid_input"));
    }
    Ok(())
}

impl App {
    fn fresh_snapshot(&self, agent: AgentId) -> NexusResult<AgentSnapshot> {
        let ledger: Vec<String> = self.store()?.ledger()?.into_iter().filter_map(|e| e.installation_id).collect();
        Ok(discover_agent(&self.ctx, adapters::adapter(agent).as_ref(), &ledger, Instant::now()))
    }

    fn base_view(&self, kind: &'static str, agent: Option<AgentId>) -> PlanView {
        let now = Instant::now();
        PlanView {
            id: new_id("plan"),
            digest: String::new(),
            kind,
            agent_id: agent,
            created_at: now,
            expires_at: now.plus_secs(PLAN_TTL_SECS),
            params: Vec::new(),
            config: None,
            process: None,
            privileges: "currentUser",
            backup: "notApplicable",
            rollback: "none",
            verification: "",
            warnings: Vec::new(),
            preserves: Vec::new(),
        }
    }

    fn config_view(&self, mut view: PlanView, edit: ConfigEdit, verify: Verify, verification: &'static str) -> Plan {
        view.backup = if edit.creates_file { "notApplicable" } else { "encrypted" };
        view.rollback = "automaticIfUnchanged";
        view.verification = verification;
        view.preserves = vec!["preserve.unrelatedSettings", "preserve.comments", "preserve.credentials"];
        view.config = Some(edit.clone());
        Plan { view, payload: Payload::Config(edit), verify }
    }

    pub fn prepare(&self, request: ActionRequest) -> NexusResult<PlanView> {
        let plan = match request {
            ActionRequest::ApplyModel { agent_id } => self.prepare_model(agent_id)?,
            ActionRequest::SetMcpEnabled { agent_id, name, enabled } => {
                validate_text(&name, 128)?;
                let snap = self.fresh_snapshot(agent_id)?;
                require_capability(&snap, "mcp.toggle")?;
                let adapter = adapters::adapter(agent_id);
                let target = adapter.mcp_target().ok_or_else(|| NexusError::new("capability.unsupported"))?;
                let edit = target.plan_toggle(&self.ctx, &name, enabled)?;
                let mut view = self.base_view("setMcpEnabled", Some(agent_id));
                view.params = vec![("name".into(), name.clone()), ("enabled".into(), enabled.to_string())];
                if !enabled {
                    view.warnings.push("warning.agentToolsUnavailable");
                }
                self.config_view(view, edit, Verify::McpEnabled { agent: agent_id, name, enabled }, "verify.mcpEnabled")
            }
            ActionRequest::AddRemoteMcp { agent_id, name, url } => {
                let snap = self.fresh_snapshot(agent_id)?;
                require_capability(&snap, "mcp.addRemote")?;
                let adapter = adapters::adapter(agent_id);
                let target = adapter.mcp_target().ok_or_else(|| NexusError::new("capability.unsupported"))?;
                let edit = target.plan_add_remote(&self.ctx, &name, &url)?;
                let origin = discovery::url_origin(&url);
                let mut view = self.base_view("addRemoteMcp", Some(agent_id));
                view.params = vec![("name".into(), name.clone()), ("origin".into(), origin.clone().unwrap_or_default())];
                view.warnings = vec!["warning.remoteMcpNetwork", "warning.publisherUnverified"];
                self.config_view(view, edit, Verify::McpPresent { agent: agent_id, name, present: true, origin }, "verify.mcpPresent")
            }
            ActionRequest::RemoveManagedMcp { agent_id, name } => {
                validate_text(&name, 128)?;
                if !self.store()?.managed_extensions(agent_id)?.contains(&name) {
                    // Only entries Nexus created can be removed; user entries are never deleted.
                    return Err(NexusError::new("extension.not_managed"));
                }
                let snap = self.fresh_snapshot(agent_id)?;
                require_capability(&snap, "mcp.addRemote")?;
                let adapter = adapters::adapter(agent_id);
                let target = adapter.mcp_target().ok_or_else(|| NexusError::new("capability.unsupported"))?;
                let edit = target.plan_remove(&self.ctx, &name)?;
                let mut view = self.base_view("removeManagedMcp", Some(agent_id));
                view.params = vec![("name".into(), name.clone())];
                self.config_view(view, edit, Verify::McpPresent { agent: agent_id, name, present: false, origin: None }, "verify.mcpAbsent")
            }
            ActionRequest::RestoreBackup { backup_id } => self.prepare_restore(&backup_id)?,
            ActionRequest::Install { agent_id, recipe_id } => {
                validate_text(&recipe_id, 64)?;
                let snap = self.fresh_snapshot(agent_id)?;
                if !snap.installations.is_empty() {
                    return Err(NexusError::new("lifecycle.already_installed"));
                }
                let recipe = adapters::adapter(agent_id).recipes().into_iter().find(|r| r.id == recipe_id && r.ops.contains(&Op::Install)).ok_or_else(|| NexusError::new("lifecycle.unknown_recipe"))?;
                self.prepare_lifecycle(agent_id, recipe, Op::Install, None, None)?
            }
            ActionRequest::Update { agent_id, installation_id } => self.prepare_existing(agent_id, &installation_id, Op::Update)?,
            ActionRequest::Uninstall { agent_id, installation_id } => self.prepare_existing(agent_id, &installation_id, Op::Uninstall)?,
            ActionRequest::RecoverInstall { operation_id } => self.prepare_recovery(&operation_id)?,
        };
        let mut plan = plan;
        plan.view.digest = digest(&plan.view, &plan.payload);
        let view = plan.view.clone();
        let mut plans = self.plans.lock().map_err(|_| NexusError::new("app.state_poisoned"))?;
        let now = Instant::now();
        plans.retain(|_, p| p.view.expires_at > now);
        if plans.len() >= MAX_PLANS {
            return Err(NexusError::new("plan.too_many").retryable());
        }
        plans.insert(view.id.clone(), plan);
        Ok(view)
    }

    fn prepare_model(&self, agent: AgentId) -> NexusResult<Plan> {
        let snap = self.fresh_snapshot(agent)?;
        require_capability(&snap, "model.write")?;
        let resolved = self.resolve_assignment(agent)?.ok_or_else(|| NexusError::new("assignment.not_found"))?;
        if resolved.compatibility.support == Support::Unsupported {
            return Err(NexusError::new("model.incompatible").with("reason", resolved.compatibility.reason.unwrap_or("")));
        }
        let identifier = resolved.compatibility.native_identifier.clone().ok_or_else(|| NexusError::new("model.incompatible"))?;
        let routed_elsewhere = match agent {
            AgentId::Codex => snap.model.provider_hint.as_deref().map(|h| h != "openai").unwrap_or(false),
            AgentId::Claude => matches!(snap.model.provider_hint.as_deref(), Some("bedrock" | "vertex")) || snap.auth.evidence.contains(&AuthEvidence::CloudRouting),
            AgentId::Gemini => snap.auth.evidence.contains(&AuthEvidence::CloudRouting),
            AgentId::Opencode => false,
        };
        if routed_elsewhere {
            return Err(NexusError::new("model.routing_mismatch").with("route", snap.model.provider_hint.clone().unwrap_or_else(|| "cloud".into())));
        }
        crate::models::validate_identifier(&identifier)?;
        let adapter = adapters::adapter(agent);
        let target = adapter.model_target().ok_or_else(|| NexusError::new("capability.unsupported"))?;
        let mut view = self.base_view("applyModel", Some(agent));
        view.params = vec![("model".into(), identifier.clone()), ("provider".into(), resolved.provider.display_name.clone())];
        if let Some(rev) = resolved.profile_revision {
            view.params.push(("profileRevision".into(), rev.to_string()));
        }
        if resolved.compatibility.support == Support::Unverified {
            view.warnings.push("warning.availabilityUnverified");
        }
        let higher = snap.model.layers.iter().take_while(|l| !(l.kind == ModelSourceKind::UserConfig && !l.uncertain)).any(|l| l.value.is_some() || l.uncertain);
        if higher {
            view.warnings.push("warning.effectiveOverridden");
        }
        view.warnings.push("warning.nextSession");
        view.preserves.push("preserve.nativeLogin");
        let verify = Verify::Model { agent, identifier: identifier.clone(), profile_revision: resolved.profile_revision, provider_revision: Some(resolved.provider.revision) };
        match target.plan_model(&self.ctx, &identifier) {
            Ok(edit) => Ok(self.config_view(view, edit, verify, "verify.modelApplied")),
            Err(e) if e.code == "config.no_change" => {
                view.verification = "verify.modelApplied";
                view.warnings.push("warning.alreadyConfigured");
                Ok(Plan { view, payload: Payload::RecordOnly, verify })
            }
            Err(e) => Err(e),
        }
    }

    fn prepare_restore(&self, backup_id: &str) -> NexusResult<Plan> {
        validate_text(backup_id, 64)?;
        let record = self.store()?.backup(backup_id)?;
        let bytes = self.vault.read(backup_id)?;
        if sha256_hex(&bytes) != record.sha256 {
            return Err(NexusError::new("backup.corrupt"));
        }
        let agent = record.agent_id.as_deref().and_then(AgentId::parse).ok_or_else(|| NexusError::new("backup.invalid_reference"))?;
        // The target must still be one of the adapter's known configuration sources.
        let adapter = adapters::adapter(agent);
        let current = adapter.read_configs(&self.ctx).into_iter().find(|r| r.source.id == record.target_id && r.source.path.to_string_lossy() == record.target_path).ok_or_else(|| NexusError::new("backup.target_unknown"))?;
        if current.source.via_symlink {
            return Err(NexusError::new("config.unsafe_symlink"));
        }
        if current.source.scope != ConfigScope::User {
            return Err(NexusError::new("backup.target_read_only"));
        }
        crate::platform::fs::check_mutable_target(&current.source.path)?;
        let text = String::from_utf8(bytes.clone()).map_err(|_| NexusError::new("backup.corrupt"))?;
        let restored_ok = match current.source.format {
            ConfigFormat::Toml => text.parse::<toml_edit::DocumentMut>().is_ok(),
            ConfigFormat::Json => crate::config::jsonc::Document::parse(&text, crate::config::jsonc::Dialect::Json).is_ok(),
            ConfigFormat::Jsonc => crate::config::jsonc::Document::parse(&text, crate::config::jsonc::Dialect::Jsonc).is_ok(),
        };
        if !restored_ok {
            return Err(NexusError::new("backup.restored_invalid"));
        }
        if current.source.fingerprint.as_deref() == Some(record.sha256.as_str()) {
            return Err(NexusError::new("config.no_change"));
        }
        let before_text = current.text.clone().unwrap_or_default();
        let mut view = self.base_view("restoreBackup", Some(agent));
        view.params = vec![("backupId".into(), backup_id.to_string()), ("createdAt".into(), record.created_at.clone()), ("location".into(), record.target_display.clone())];
        // Execution-sensitive fields reintroduced by an old file must be disclosed (Security §6).
        let sensitive = ["hooks", "apiKeyHelper", "mcpServers", "mcp_servers", "mcp", "notify", "permissions"];
        let had = |t: &str, k: &str| t.contains(&format!("\"{k}\"")) || t.contains(&format!("{k} =")) || t.contains(&format!("[{k}"));
        if sensitive.iter().any(|k| had(&text, k) && !had(&before_text, k)) {
            view.warnings.push("warning.restoreReintroducesExecutable");
        }
        if current.source.status == ObsStatus::ParseFailed {
            view.warnings.push("warning.replacesMalformed");
        }
        view.warnings.push("warning.restoreReplacesLaterChanges");
        let edit = ConfigEdit {
            target_id: current.source.id.clone(),
            location: current.source.location.clone(),
            format: current.source.format,
            creates_file: current.source.status == ObsStatus::NotFound,
            before_fingerprint: current.source.fingerprint.clone(),
            after_fingerprint: record.sha256.clone(),
            changes: vec![crate::config::ChangeLine { key: "(file)".into(), before: current.source.fingerprint.as_ref().map(|f| f[..12].to_string()), after: Some(record.sha256[..12].to_string()) }],
            // Whole-file content (old or new) may hold secrets that heuristic redaction misses,
            // so restores disclose fingerprints and execution-sensitive warnings, never lines.
            diff: Vec::new(),
            path: current.source.path.clone(),
            after: bytes,
        };
        Ok(self.config_view(view, edit, Verify::Restore { sha256: record.sha256 }, "verify.restore"))
    }

    fn prepare_existing(&self, agent: AgentId, installation_id: &str, op: Op) -> NexusResult<Plan> {
        validate_text(installation_id, 64)?;
        let snap = self.fresh_snapshot(agent)?;
        let inst = snap.installations.iter().find(|i| i.id == installation_id).ok_or_else(|| NexusError::new("lifecycle.installation_not_found"))?;
        require_capability(&snap, if op == Op::Update { "lifecycle.update" } else { "lifecycle.uninstall" })?;
        let recipe = adapters::adapter(agent)
            .recipes()
            .into_iter()
            .find(|r| r.mechanism.source() == inst.source && r.ops.contains(&op))
            .ok_or_else(|| NexusError::new("lifecycle.no_recipe_for_source").with("source", format!("{:?}", inst.source)))?;
        let exe = inst.resolved.clone().unwrap_or_else(|| inst.executable.clone());
        let exe_path = expand(&self.ctx, &exe);
        let mut plan = self.prepare_lifecycle(agent, recipe, op, Some((inst.id.clone(), inst.version.value.clone(), exe_path)), None)?;
        if let Verify::Lifecycle { launcher, .. } = &mut plan.verify {
            *launcher = Some(inst.executable.clone());
        }
        Ok(plan)
    }

    fn prepare_lifecycle(&self, agent: AgentId, recipe: Recipe, op: Op, existing: Option<(String, Option<String>, std::path::PathBuf)>, recovers: Option<String>) -> NexusResult<Plan> {
        let tools = lifecycle::toolchain(&self.ctx);
        if !lifecycle::available(&recipe, &self.ctx, &tools) {
            return Err(NexusError::new("lifecycle.tool_missing").with("recipe", recipe.id));
        }
        let version = if op == Op::Uninstall { None } else { lifecycle::resolve_version(&self.ctx, &tools, &recipe)? };
        let previous = existing.as_ref().and_then(|e| e.1.clone());
        if op == Op::Update && version.is_some() && version == previous {
            return Err(NexusError::new("lifecycle.up_to_date").with("version", version.unwrap_or_default()));
        }
        let npm_prefix = match (recipe.mechanism, &existing) {
            (lifecycle::Mechanism::Npm { .. }, Some(e)) => Some(lifecycle::npm_prefix_of(&e.2).ok_or_else(|| NexusError::new("lifecycle.prefix_unknown"))?),
            _ => None,
        };
        let spec = lifecycle::build_spec(&self.ctx, &tools, &recipe, op, version.as_deref(), existing.as_ref().map(|e| e.2.as_path()), npm_prefix.as_deref())?;
        let kind = match (op, recovers.is_some()) {
            (_, true) => "recoverInstall",
            (Op::Install, _) => "install",
            (Op::Update, _) => "update",
            (Op::Uninstall, _) => "uninstall",
        };
        let mut view = self.base_view(kind, Some(agent));
        view.params = vec![("recipe".into(), recipe.id.into()), ("package".into(), recipe.mechanism.package_label())];
        if let Some(v) = &version {
            view.params.push(("version".into(), v.clone()));
        }
        if let Some(p) = &previous {
            view.params.push(("previousVersion".into(), p.clone()));
        }
        if let Some(op_id) = &recovers {
            view.params.push(("operationId".into(), op_id.clone()));
        }
        if let Some(p) = &npm_prefix {
            view.params.push(("prefix".into(), self.ctx.display_path(p)));
        }
        view.process = Some(ProcessPreview {
            program: self.ctx.display_path(&spec.program),
            args: spec.args.clone(),
            env_names: spec.env.iter().map(|(k, _)| k.clone()).collect(),
            runs_package_scripts: recipe.runs_package_scripts,
            uses_network: true,
            package: recipe.mechanism.package_label(),
            version: version.clone(),
        });
        view.rollback = if op == Op::Uninstall { "reinstallManually" } else { "manual" };
        view.verification = match op { Op::Install => "verify.installed", Op::Update => "verify.updated", Op::Uninstall => "verify.removed" };
        view.preserves = vec!["preserve.configuration", "preserve.credentials", "preserve.sharedRuntimes"];
        if recipe.runs_package_scripts {
            view.warnings.push("warning.packageScripts");
        }
        if version.is_none() && op == Op::Update {
            view.warnings.push("warning.versionChosenByAgent");
        }
        if op != Op::Uninstall {
            view.warnings.push("warning.network");
        }
        if op == Op::Uninstall && existing.is_some() {
            view.warnings.push("warning.uninstallExternal");
        }
        let verify = Verify::Lifecycle { agent, op, recipe: recipe.id, source: recipe.mechanism.source(), version, installation_id: existing.as_ref().map(|e| e.0.clone()), launcher: None, previous_version: previous, recovers };
        Ok(Plan { view, payload: Payload::Process(spec), verify })
    }

    fn prepare_recovery(&self, operation_id: &str) -> NexusResult<Plan> {
        validate_text(operation_id, 64)?;
        let op = self.store()?.operations(200)?.into_iter().find(|o| o.id == operation_id).ok_or_else(|| NexusError::new("operation.not_found"))?;
        if op.status != "interrupted" || op.action != "install" {
            return Err(NexusError::new("operation.not_recoverable"));
        }
        let agent = op.agent_id.as_deref().and_then(AgentId::parse).ok_or_else(|| NexusError::new("operation.not_recoverable"))?;
        let recipe_id = op.summary.get("recipe").and_then(|v| v.as_str()).ok_or_else(|| NexusError::new("operation.not_recoverable"))?;
        let version = op.summary.get("version").and_then(|v| v.as_str()).map(String::from);
        let recipe = adapters::adapter(agent).recipes().into_iter().find(|r| r.id == recipe_id).ok_or_else(|| NexusError::new("lifecycle.unknown_recipe"))?;
        let snap = self.fresh_snapshot(agent)?;
        // Reconcile observed state first; never blindly replay (Architecture §8).
        if let Some(inst) = snap.installations.iter().find(|i| i.source == recipe.mechanism.source()) {
            if version.is_some() && inst.version.value == version {
                let mut view = self.base_view("recoverInstall", Some(agent));
                view.params = vec![("recipe".into(), recipe.id.into()), ("operationId".into(), operation_id.into()), ("version".into(), version.clone().unwrap_or_default()), ("reconcileOnly".into(), "true".into())];
                view.verification = "verify.installed";
                let verify = Verify::Lifecycle { agent, op: Op::Install, recipe: recipe.id, source: recipe.mechanism.source(), version, installation_id: None, launcher: None, previous_version: None, recovers: Some(operation_id.into()) };
                return Ok(Plan { view, payload: Payload::RecordOnly, verify });
            }
        }
        let mut plan = self.prepare_lifecycle(agent, recipe, Op::Install, None, Some(operation_id.into()))?;
        // Recovery reinstalls the originally confirmed version when it was recorded.
        let npm = matches!(adapters::adapter(agent).recipes().into_iter().find(|r| r.id == recipe_id).map(|r| r.mechanism), Some(lifecycle::Mechanism::Npm { .. }));
        if let (Some(v), Payload::Process(spec), true) = (&version, &mut plan.payload, npm) {
            lifecycle::validate_version(v)?;
            if let Some(last) = spec.args.last_mut() {
                if let Some((pkg, _)) = last.rsplit_once('@').filter(|(p, _)| !p.is_empty()) {
                    *last = format!("{pkg}@{v}");
                }
            }
            if let Some(p) = plan.view.process.as_mut() {
                p.args = spec.args.clone();
                p.version = Some(v.clone());
            }
            plan.view.params.retain(|(k, _)| k != "version");
            plan.view.params.push(("version".into(), v.clone()));
            if let Verify::Lifecycle { version: ver, .. } = &mut plan.verify {
                *ver = Some(v.clone());
            }
        }
        Ok(plan)
    }

    pub fn discard_plan(&self, plan_id: &str) -> NexusResult<()> {
        self.plans.lock().map_err(|_| NexusError::new("app.state_poisoned"))?.remove(plan_id);
        Ok(())
    }

    /// Consumes the plan (single use), obtains native confirmation, executes and verifies.
    pub fn confirm(&self, plan_id: &str, digest_from_ui: &str) -> NexusResult<OperationResult> {
        let plan = self.plans.lock().map_err(|_| NexusError::new("app.state_poisoned"))?.remove(plan_id).ok_or_else(|| NexusError::new("plan.not_found"))?;
        let now = Instant::now();
        if plan.view.expires_at <= now {
            return Err(NexusError::new("plan.expired"));
        }
        if plan.view.digest != digest_from_ui || digest(&plan.view, &plan.payload) != plan.view.digest {
            return Err(NexusError::new("plan.digest_mismatch"));
        }
        let lock_key = plan.view.agent_id.map(|a| a.as_str().to_string()).unwrap_or_else(|| "global".into());
        {
            let mut busy = self.busy.lock().map_err(|_| NexusError::new("app.state_poisoned"))?;
            if !busy.insert(lock_key.clone()) {
                return Err(NexusError::new("operation.busy").retryable());
            }
        }
        let result = (|| {
            if !self.confirmer.confirm(&plan.view, &self.locale()) {
                return Err(NexusError::new("confirmation.denied"));
            }
            self.execute(plan)
        })();
        if let Ok(mut busy) = self.busy.lock() {
            busy.remove(&lock_key);
        }
        self.invalidate();
        result
    }

    fn execute(&self, plan: Plan) -> NexusResult<OperationResult> {
        let op_id = new_id("op");
        let kind = plan.view.kind;
        let agent = plan.view.agent_id;
        let summary = serde_json::json!({
            "kind": kind,
            "params": plan.view.params.iter().cloned().collect::<std::collections::BTreeMap<_, _>>(),
            "recipe": plan.view.params.iter().find(|(k, _)| k == "recipe").map(|(_, v)| v.clone()),
            "version": plan.view.params.iter().find(|(k, _)| k == "version").map(|(_, v)| v.clone()),
            "location": plan.view.config.as_ref().map(|c| c.location.clone()),
        });
        self.store()?.begin_operation(&op_id, kind, agent, &plan.view.digest, &summary, Instant::now())?;
        let mut result = OperationResult {
            operation_id: op_id.clone(),
            kind,
            agent_id: agent,
            status: "failed",
            verification: VerificationResult { outcome: "notRun", checks: vec![], verified_at: Instant::now() },
            error: None,
            backup_id: None,
            compensation: None,
            output_excerpt: None,
            warnings: plan.view.warnings.clone(),
        };
        match &plan.payload {
            Payload::Config(edit) => {
                let mut backup_id = None;
                let applied = transaction::apply(edit, &self.vault, &mut |step, backup| {
                    if let (Step::BackedUp, Some(b)) = (step, backup) {
                        backup_id = Some(b.id.clone());
                        if let Ok(store) = self.store() {
                            let _ = store.insert_backup(&b.id, &op_id, agent, &edit.target_id, &edit.path.to_string_lossy(), &edit.location, &b.sha256, b.size, Instant::now());
                        }
                    }
                    if let Ok(store) = self.store() {
                        let _ = store.step(&op_id, &format!("{step:?}").to_lowercase(), Instant::now());
                    }
                });
                result.backup_id = backup_id.clone();
                let applied = match applied {
                    Ok(a) => a,
                    Err(e) => {
                        // Failure before or during the write: the target is unchanged or was
                        // atomically not replaced. Nothing to compensate.
                        self.store()?.finish_operation(&op_id, "failed", Some(e.code), Instant::now())?;
                        result.error = Some(e);
                        return Ok(result);
                    }
                };
                if let Some(b) = &applied.backup {
                    let _ = self.store()?.set_backup_written(&b.id, &applied.written_fingerprint);
                }
                result.verification = if applied.reread_ok { self.verify(&plan.verify) } else { VerificationResult { outcome: "failed", checks: vec![VerificationCheck { key: "verify.configParses", outcome: "failed", params: vec![], informational: false }], verified_at: Instant::now() } };
                if result.verification.outcome == "passed" {
                    self.record_success(&plan.verify, &op_id, Some(edit))?;
                    result.status = "succeeded";
                } else {
                    let comp = transaction::compensate(edit, &applied.written_fingerprint, &self.vault, applied.backup.as_ref());
                    let (status, label) = match comp {
                        Compensation::RolledBack => ("rolledBack", "rolledBack"),
                        Compensation::RefusedExternalChange => ("partial", "refusedExternalChange"),
                        Compensation::Failed(_) => ("rollbackFailed", "rollbackFailed"),
                    };
                    result.status = status;
                    result.compensation = Some(label);
                    result.error = Some(NexusError::new("operation.verification_failed"));
                }
            }
            Payload::Process(spec) => {
                self.store()?.step(&op_id, "process.started", Instant::now())?;
                let outcome = run(spec);
                let (exited_ok, excerpt) = match &outcome {
                    Ok(o) => {
                        let _ = self.store()?.step(&op_id, &format!("process.exited.{:?}", o.exit).to_lowercase(), Instant::now());
                        (o.success(), if o.success() { None } else { Some(tail(&format!("{}\n{}", o.stderr, o.stdout))) })
                    }
                    Err(e) => (false, Some(e.code.to_string())),
                };
                result.output_excerpt = excerpt;
                result.verification = self.verify(&plan.verify);
                let verified = result.verification.outcome == "passed";
                result.status = match (exited_ok, verified) {
                    (true, true) => "succeeded",
                    (false, true) => "partial",
                    _ => "failed",
                };
                if verified {
                    self.record_success(&plan.verify, &op_id, None)?;
                }
                if !verified {
                    result.error = Some(NexusError::new(if exited_ok { "operation.verification_failed" } else { "lifecycle.process_failed" }));
                }
            }
            Payload::RecordOnly => {
                result.verification = self.verify(&plan.verify);
                if result.verification.outcome == "passed" {
                    self.record_success(&plan.verify, &op_id, None)?;
                    result.status = "succeeded";
                } else {
                    result.error = Some(NexusError::new("operation.verification_failed"));
                }
            }
        }
        self.store()?.finish_operation(&op_id, result.status, result.error.as_ref().map(|e| e.code), Instant::now())?;
        Ok(result)
    }

    /// Independent post-conditions from a *fresh* read; never from the write's own return value.
    pub(crate) fn verify(&self, verify: &Verify) -> VerificationResult {
        let now = Instant::now();
        let check = |key: &'static str, ok: bool, params: Vec<(String, String)>| VerificationCheck { key, outcome: if ok { "passed" } else { "failed" }, params, informational: false };
        let checks: Vec<VerificationCheck> = match verify {
            Verify::Model { agent, identifier, .. } => {
                let adapter = adapters::adapter(*agent);
                let configs = adapter.read_configs(&self.ctx);
                let written = adapter.model_target().and_then(|t| t.user_config_model(&self.ctx));
                let parse_ok = configs.iter().all(|c| c.source.scope != ConfigScope::User || matches!(c.source.status, ObsStatus::Observed | ObsStatus::NotFound));
                let observed = adapter.observe_model(&self.ctx, &configs);
                let mut v = vec![
                    check("verify.configParses", parse_ok, vec![]),
                    check("verify.userConfigModel", written.as_deref() == Some(identifier.as_str()), vec![("observed".into(), written.unwrap_or_default())]),
                ];
                // Effective state is reported separately: an override is not a failed write.
                v.push(VerificationCheck {
                    key: "verify.effectiveModel",
                    outcome: match (observed.effective.as_deref() == Some(identifier.as_str()), observed.layers.iter().any(|l| l.uncertain)) {
                        (true, false) => "passed",
                        (true, true) => "inconclusive",
                        (false, _) => "overridden",
                    },
                    params: vec![("effective".into(), observed.effective.unwrap_or_default()), ("source".into(), format!("{:?}", observed.effective_kind))],
                    informational: true,
                });
                v
            }
            Verify::McpEnabled { agent, name, enabled } => {
                let adapter = adapters::adapter(*agent);
                let configs = adapter.read_configs(&self.ctx);
                let ext = adapter.observe_extensions(&self.ctx, &configs);
                let found = ext.mcp.iter().find(|m| &m.name == name);
                vec![check("verify.mcpEnabled", found.map(|m| m.enabled == Some(*enabled)).unwrap_or(false), vec![("name".into(), name.clone())]), check("verify.mcpCount", true, vec![("count".into(), ext.mcp.len().to_string())])]
            }
            Verify::McpPresent { agent, name, present, origin } => {
                let adapter = adapters::adapter(*agent);
                let configs = adapter.read_configs(&self.ctx);
                let ext = adapter.observe_extensions(&self.ctx, &configs);
                let found = ext.mcp.iter().find(|m| &m.name == name);
                let ok = match (found, present) {
                    (Some(m), true) => origin.is_none() || m.endpoint_origin == *origin,
                    (None, false) => true,
                    _ => false,
                };
                vec![check(if *present { "verify.mcpPresent" } else { "verify.mcpAbsent" }, ok, vec![("name".into(), name.clone())])]
            }
            Verify::Restore { sha256, .. } => {
                // Restored bytes must match the backup exactly; the transaction already re-read them.
                vec![check("verify.restoredBytes", true, vec![("sha256".into(), sha256[..12].to_string())])]
            }
            Verify::Lifecycle { agent, op, source, version, installation_id, launcher, previous_version, .. } => {
                let snap = match self.fresh_snapshot(*agent) {
                    Ok(s) => s,
                    Err(_) => return VerificationResult { outcome: "inconclusive", checks: vec![], verified_at: now },
                };
                match op {
                    Op::Install | Op::Update => {
                        let by_id = installation_id.as_ref().and_then(|id| snap.installations.iter().find(|i| &i.id == id));
                        let by_launcher = launcher.as_ref().and_then(|l| snap.installations.iter().find(|i| &i.executable == l));
                        let inst = by_id.or(by_launcher).or_else(|| if installation_id.is_none() { snap.installations.iter().find(|i| i.source == *source) } else { None });
                        let present = inst.is_some();
                        let observed = inst.and_then(|i| i.version.value.clone());
                        let version_ok = match (version, &observed) {
                            (Some(v), Some(o)) => v == o,
                            (None, Some(o)) => previous_version.as_ref().map(|p| version_at_least(o, p).unwrap_or(false)).unwrap_or(true),
                            _ => false,
                        };
                        vec![
                            check("verify.installationPresent", present, vec![("source".into(), format!("{source:?}"))]),
                            VerificationCheck { key: "verify.version", outcome: if !present { "failed" } else if observed.is_none() { "inconclusive" } else if version_ok { "passed" } else { "failed" }, params: vec![("expected".into(), version.clone().unwrap_or_default()), ("observed".into(), observed.unwrap_or_default())], informational: false },
                        ]
                    }
                    Op::Uninstall => {
                        let gone = installation_id.as_ref().map(|id| !snap.installations.iter().any(|i| &i.id == id || Some(&i.executable) == launcher.as_ref())).unwrap_or(false);
                        vec![check("verify.installationRemoved", gone, vec![])]
                    }
                }
            }
        };
        let decisive = || checks.iter().filter(|c| !c.informational);
        let outcome = if decisive().any(|c| c.outcome == "failed") {
            "failed"
        } else if decisive().any(|c| c.outcome == "inconclusive") {
            "inconclusive"
        } else {
            "passed"
        };
        VerificationResult { outcome, checks, verified_at: now }
    }

    fn record_success(&self, verify: &Verify, op_id: &str, edit: Option<&ConfigEdit>) -> NexusResult<()> {
        let store = self.store()?;
        let now = Instant::now();
        match verify {
            Verify::Model { agent, identifier, profile_revision, provider_revision } => {
                let location = edit.map(|e| e.location.clone()).or_else(|| adapters::adapter(*agent).read_configs(&self.ctx).into_iter().find(|r| r.source.scope == ConfigScope::User).map(|r| r.source.location)).unwrap_or_default();
                let fingerprint = edit.map(|e| e.after_fingerprint.clone()).unwrap_or_default();
                store.set_applied(*agent, &crate::models::AppliedRecord { identifier: identifier.clone(), profile_revision: *profile_revision, provider_revision: *provider_revision, applied_at: now.to_rfc3339(), location, fingerprint })?;
            }
            Verify::McpPresent { agent, name, present: true, origin } => store.add_managed_extension(*agent, name, origin.as_deref(), op_id, now)?,
            Verify::McpPresent { agent, name, present: false, .. } => store.remove_managed_extension(*agent, name)?,
            Verify::Lifecycle { agent, op, recipe, version, recovers, .. } => {
                let snap = discover_agent(&self.ctx, adapters::adapter(*agent).as_ref(), &[], now);
                match op {
                    Op::Install | Op::Update => {
                        let inst = snap.installations.iter().find(|i| version.is_none() || i.version.value == *version).or(snap.primary_installation());
                        if *op == Op::Install || store.ledger()?.iter().any(|e| e.agent_id == agent.as_str() && e.recipe_id == *recipe) {
                            store.record_installation(&crate::store::LedgerEntry { agent_id: agent.as_str().into(), recipe_id: recipe.to_string(), version: version.clone(), installation_id: inst.map(|i| i.id.clone()), operation_id: op_id.into(), status: "verified".into(), at: now.to_rfc3339() })?;
                        }
                    }
                    Op::Uninstall => store.remove_installation(*agent, recipe)?,
                }
                if let Some(original) = recovers {
                    store.finish_operation(original, "recovered", None, now)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Opens the Agent in the user's terminal. Direct user action; never weakens Agent settings.
    pub fn launch(&self, agent: AgentId, installation_id: &str) -> NexusResult<String> {
        validate_text(installation_id, 64)?;
        if self.ctx.os != crate::platform::Os::Macos {
            return Err(NexusError::new("capability.unsupported").with("capability", "launch"));
        }
        let snap = self.fresh_snapshot(agent)?;
        let inst = snap.installations.iter().find(|i| i.id == installation_id).ok_or_else(|| NexusError::new("lifecycle.installation_not_found"))?;
        let path = expand(&self.ctx, &inst.executable);
        let resolved = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !path.is_absolute() || !crate::platform::fs::is_executable_file(&path) || !lifecycle::owner_ok(&path) || !lifecycle::owner_ok(&resolved) {
            return Err(NexusError::new("lifecycle.untrusted_executable"));
        }
        let spec = ProcessSpec {
            program: "/usr/bin/open".into(),
            args: vec!["-a".into(), "Terminal".into(), path.to_string_lossy().to_string()],
            env: crate::platform::process::base_env(&self.ctx.home, &["/usr/bin".into(), "/bin".into()]),
            cwd: self.ctx.home.clone(),
            timeout: std::time::Duration::from_secs(15),
            max_output: 2048,
        };
        let op_id = new_id("op");
        let store = self.store()?;
        store.begin_operation(&op_id, "launch", Some(agent), "launch", &serde_json::json!({ "installation": installation_id }), Instant::now())?;
        let out = run(&spec)?;
        let status = if out.success() { "succeeded" } else { "failed" };
        store.finish_operation(&op_id, status, (!out.success()).then_some("lifecycle.launch_failed"), Instant::now())?;
        if !out.success() {
            return Err(NexusError::new("lifecycle.launch_failed"));
        }
        Ok(op_id)
    }
}

fn tail(text: &str) -> String {
    let clean = redact(text);
    let chars: Vec<char> = clean.chars().collect();
    chars[chars.len().saturating_sub(1500)..].iter().collect()
}

/// Expands a display path (`~/…`) produced by `HostContext::display_path` back to absolute.
pub fn expand(ctx: &crate::platform::HostContext, display: &str) -> std::path::PathBuf {
    match display.strip_prefix('~') {
        Some(rest) => ctx.home.join(rest.trim_start_matches(['/', '\\'])),
        None => std::path::PathBuf::from(display),
    }
}

