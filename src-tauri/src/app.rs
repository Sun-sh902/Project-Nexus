//! Application use cases: assembles discovery, Health, Diagnostics and Models into the typed
//! results IPC returns. Owns process-wide state (store, plans, locks, cached overview).

use crate::actions::{Plan, PlanView};
use crate::adapters;
use crate::backup::BackupVault;
use crate::core::{Instant, NexusError, NexusResult};
use crate::diagnostics::{diagnose, DiagnosticFinding, DiagnosticInputs};
use crate::discovery::discover_agent;
use crate::discovery::system::{discover_system, SystemSnapshot};
use crate::domain::*;
use crate::health::{evaluate, AgentHealth, AssignmentContext};
use crate::lifecycle;
use crate::models::*;
use crate::platform::HostContext;
use crate::secrets::SecretStore;
use crate::store::{BackupRecord, OperationRecord, Store};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};

pub trait Confirmer: Send + Sync {
    fn confirm(&self, plan: &PlanView, locale: &str) -> bool;
}

pub struct App {
    pub ctx: HostContext,
    store: Mutex<Store>,
    pub secrets: Arc<dyn SecretStore>,
    pub vault: BackupVault,
    pub plans: Mutex<HashMap<String, Plan>>,
    pub busy: Mutex<HashSet<String>>,
    pub confirmer: Box<dyn Confirmer>,
    cache: Mutex<Option<Overview>>,
    locale: Mutex<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleOption {
    pub recipe_id: &'static str,
    pub op: lifecycle::Op,
    pub package: String,
    pub available: bool,
    pub installation_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReport {
    pub snapshot: AgentSnapshot,
    pub health: AgentHealth,
    pub findings: Vec<DiagnosticFinding>,
    pub assignment: Option<ResolvedAssignment>,
    pub lifecycle: Vec<LifecycleOption>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub contract_version: u8,
    pub generated_at: Instant,
    pub system: SystemSnapshot,
    pub agents: Vec<AgentReport>,
    pub recent_operations: Vec<OperationRecord>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAssignment {
    pub assignment: AgentModelAssignment,
    pub provider: ProviderProfile,
    pub model: ModelDefinition,
    pub profile: Option<ModelProfile>,
    pub profile_revision: Option<i64>,
    pub compatibility: Compatibility,
    pub pending: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsView {
    pub providers: Vec<ProviderProfile>,
    pub models: Vec<ModelDefinition>,
    pub profiles: Vec<ModelProfile>,
    pub assignments: Vec<ResolvedAssignment>,
    pub matrix: Vec<(String, Vec<Compatibility>)>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHistory {
    pub backups: Vec<BackupRecord>,
    pub operations: Vec<OperationRecord>,
    pub health: Vec<crate::store::HealthHistoryRow>,
}

impl App {
    pub fn new(
        ctx: HostContext,
        store: Store,
        secrets: Arc<dyn SecretStore>,
        confirmer: Box<dyn Confirmer>,
    ) -> NexusResult<App> {
        crate::platform::fs::ensure_private_dir(&ctx.data_dir)?;
        let vault = BackupVault::new(ctx.data_dir.join("backups"), secrets.clone());
        // Anything still "executing" was interrupted by a crash or kill: mark, never replay.
        store.reconcile_interrupted(Instant::now())?;
        Ok(App {
            ctx,
            store: Mutex::new(store),
            secrets,
            vault,
            plans: Mutex::new(HashMap::new()),
            busy: Mutex::new(HashSet::new()),
            confirmer,
            cache: Mutex::new(None),
            locale: Mutex::new("en-US".into()),
        })
    }

    pub fn store(&self) -> NexusResult<MutexGuard<'_, Store>> {
        self.store
            .lock()
            .map_err(|_| NexusError::new("app.state_poisoned"))
    }
    pub fn locale(&self) -> String {
        self.locale
            .lock()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "en-US".into())
    }
    pub fn set_locale(&self, locale: &str) {
        if let Ok(mut l) = self.locale.lock() {
            *l = if locale == "zh-CN" {
                "zh-CN".into()
            } else {
                "en-US".into()
            };
        }
    }
    pub fn invalidate(&self) {
        if let Ok(mut c) = self.cache.lock() {
            *c = None;
        }
    }

    pub fn all_providers(&self) -> NexusResult<Vec<ProviderProfile>> {
        let mut out = builtin_providers();
        for mut p in self.store()?.providers()? {
            if let Some(r) = &p.secret_ref {
                // Presence only. A locked store is reported as unknown, never as absent.
                p.secret_present = self.secrets.get(r).ok().map(|v| v.is_some());
            }
            out.push(p);
        }
        Ok(out)
    }

    pub fn all_models(&self, observed: &[ModelDefinition]) -> NexusResult<Vec<ModelDefinition>> {
        let mut out = builtin_models();
        out.extend(self.store()?.models()?);
        for m in observed {
            if !out
                .iter()
                .any(|x| x.provider_id == m.provider_id && x.identifier == m.identifier)
            {
                out.push(m.clone());
            }
        }
        Ok(out)
    }

    /// Model identifiers currently configured in Agent configs become `observed` catalog
    /// entries (with provenance), so users can adopt what they already use.
    pub fn observed_models(&self, snapshots: &[AgentSnapshot]) -> Vec<ModelDefinition> {
        let mut out = Vec::new();
        for snap in snapshots {
            for layer in snap
                .model
                .layers
                .iter()
                .filter(|l| l.kind == ModelSourceKind::UserConfig)
            {
                let Some(value) = &layer.value else { continue };
                if validate_identifier(value).is_err() {
                    continue;
                }
                let (provider, identifier) = match snap.agent_id {
                    AgentId::Claude if snap.model.provider_hint.as_deref() == Some("anthropic") => {
                        ("builtin.anthropic", value.clone())
                    }
                    AgentId::Codex
                        if matches!(snap.model.provider_hint.as_deref(), None | Some("openai")) =>
                    {
                        ("builtin.openai", value.clone())
                    }
                    AgentId::Gemini => ("builtin.google", value.clone()),
                    AgentId::Opencode => match value.split_once('/') {
                        Some(("anthropic", m)) => ("builtin.anthropic", m.to_string()),
                        Some(("openai", m)) => ("builtin.openai", m.to_string()),
                        Some(("google", m)) => ("builtin.google", m.to_string()),
                        _ => continue,
                    },
                    _ => continue,
                };
                if out.iter().any(|m: &ModelDefinition| {
                    m.provider_id == provider && m.identifier == identifier
                }) {
                    continue;
                }
                out.push(ModelDefinition {
                    id: format!("observed.{provider}.{identifier}"),
                    provider_id: provider.into(),
                    identifier,
                    display_name: None,
                    source: "observed".into(),
                    source_detail: Some(layer.location.clone()),
                    capabilities: Vec::new(),
                });
            }
        }
        out
    }

    pub fn resolve_assignment(&self, agent: AgentId) -> NexusResult<Option<ResolvedAssignment>> {
        let Some(assignment) = self.store()?.assignment(agent)? else {
            return Ok(None);
        };
        let (model_id, profile, profile_revision) = match assignment.mode {
            AssignmentMode::Native => return Ok(None),
            AssignmentMode::Direct => (assignment.model_id.clone(), None, None),
            AssignmentMode::Profile => {
                let pid = assignment
                    .profile_id
                    .clone()
                    .ok_or_else(|| NexusError::new("assignment.invalid"))?;
                let profile = self
                    .store()?
                    .profiles()?
                    .into_iter()
                    .find(|p| p.id == pid)
                    .ok_or_else(|| NexusError::new("profile.not_found"))?;
                let rev = profile.revision;
                (Some(profile.model_id.clone()), Some(profile), Some(rev))
            }
        };
        let model_id = model_id.ok_or_else(|| NexusError::new("assignment.invalid"))?;
        let models = self.all_models(&[])?;
        let model = models
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| NexusError::new("model.not_found"))?;
        let provider = self
            .all_providers()?
            .into_iter()
            .find(|p| p.id == model.provider_id)
            .ok_or_else(|| NexusError::new("provider.not_found"))?;
        let compatibility = compatibility(agent, &provider, &model);
        let pending = match &assignment.applied {
            None => true,
            Some(a) => {
                Some(&a.identifier) != compatibility.native_identifier.as_ref()
                    || (assignment.mode == AssignmentMode::Profile
                        && a.profile_revision != profile_revision)
            }
        };
        Ok(Some(ResolvedAssignment {
            assignment,
            provider,
            model,
            profile,
            profile_revision,
            compatibility,
            pending,
        }))
    }

    fn report(
        &self,
        snap: AgentSnapshot,
        system: &SystemSnapshot,
        now: Instant,
        ops: &[OperationRecord],
    ) -> NexusResult<AgentReport> {
        let resolved = self.resolve_assignment(snap.agent_id).unwrap_or(None);
        let ctx = match &resolved {
            Some(r) => AssignmentContext {
                desired_identifier: r
                    .compatibility
                    .native_identifier
                    .clone()
                    .or_else(|| Some(r.model.identifier.clone())),
                compatible: Some(r.compatibility.support),
                applied_identifier: r.assignment.applied.as_ref().map(|a| a.identifier.clone()),
                desired_matches_applied: !r.pending,
            },
            None => AssignmentContext::default(),
        };
        let health = evaluate(&snap, system, &ctx, now);
        let backups = self.store()?.backups(Some(snap.agent_id))?;
        let interrupted: Vec<OperationRecord> = ops
            .iter()
            .filter(|o| {
                o.status == "interrupted" && o.agent_id.as_deref() == Some(snap.agent_id.as_str())
            })
            .cloned()
            .collect();
        let tools = lifecycle::toolchain(&self.ctx);
        let recipes = adapters::adapter(snap.agent_id).recipes();
        let mut options = Vec::new();
        if snap.installations.is_empty() {
            for r in recipes.iter().filter(|r| {
                r.ops.contains(&lifecycle::Op::Install)
                    && r.mechanism.platforms().contains(&self.ctx.os)
            }) {
                options.push(LifecycleOption {
                    recipe_id: r.id,
                    op: lifecycle::Op::Install,
                    package: r.mechanism.package_label(),
                    available: lifecycle::available(r, &self.ctx, &tools),
                    installation_id: None,
                });
            }
        }
        for inst in &snap.installations {
            for op in [lifecycle::Op::Update, lifecycle::Op::Uninstall] {
                if let Some(r) = recipes
                    .iter()
                    .find(|r| r.mechanism.source() == inst.source && r.ops.contains(&op))
                {
                    options.push(LifecycleOption {
                        recipe_id: r.id,
                        op,
                        package: r.mechanism.package_label(),
                        available: lifecycle::available(r, &self.ctx, &tools),
                        installation_id: Some(inst.id.clone()),
                    });
                }
            }
        }
        let install_available = options
            .iter()
            .any(|o| o.op == lifecycle::Op::Install && o.available);
        let findings = diagnose(&DiagnosticInputs {
            snapshot: &snap,
            health: &health,
            backups: &backups,
            interrupted: &interrupted,
            install_recipe_available: install_available,
        });
        Ok(AgentReport {
            snapshot: snap,
            health,
            findings,
            assignment: resolved,
            lifecycle: options,
        })
    }

    /// Full read-only scan. Agents are discovered in parallel with bounded per-probe timeouts.
    pub fn overview(&self, refresh: bool) -> NexusResult<Overview> {
        if !refresh {
            if let Some(cached) = self
                .cache
                .lock()
                .map_err(|_| NexusError::new("app.state_poisoned"))?
                .clone()
            {
                return Ok(cached);
            }
        }
        let now = Instant::now();
        let ledger: Vec<String> = self
            .store()?
            .ledger()?
            .into_iter()
            .filter_map(|e| e.installation_id)
            .collect();
        let ctx = &self.ctx;
        let (system, snapshots) = std::thread::scope(|scope| {
            let sys = scope.spawn(|| discover_system(ctx, now));
            let handles: Vec<_> = adapters::registry()
                .into_iter()
                .map(|a| {
                    let ledger = &ledger;
                    scope.spawn(move || discover_agent(ctx, a.as_ref(), ledger, now))
                })
                .collect();
            let snaps: Vec<AgentSnapshot> =
                handles.into_iter().filter_map(|h| h.join().ok()).collect();
            (sys.join().expect("system discovery thread"), snaps)
        });
        let ops = self.store()?.operations(50)?;
        let mut agents = Vec::new();
        for snap in snapshots {
            let report = self.report(snap, &system, now, &ops)?;
            let codes: Vec<String> = report.findings.iter().map(|f| f.code.clone()).collect();
            let readiness = serde_json::to_value(report.health.readiness)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            self.store()?.record_health(
                report.snapshot.agent_id,
                &readiness,
                crate::health::RULE_SET,
                &codes,
                now,
            )?;
            agents.push(report);
        }
        let overview = Overview {
            contract_version: 1,
            generated_at: now,
            system,
            agents,
            recent_operations: ops.into_iter().take(10).collect(),
        };
        *self
            .cache
            .lock()
            .map_err(|_| NexusError::new("app.state_poisoned"))? = Some(overview.clone());
        Ok(overview)
    }

    pub fn agent_history(&self, agent: AgentId) -> NexusResult<AgentHistory> {
        let store = self.store()?;
        Ok(AgentHistory {
            backups: store.backups(Some(agent))?,
            operations: store
                .operations(100)?
                .into_iter()
                .filter(|o| o.agent_id.as_deref() == Some(agent.as_str()))
                .take(20)
                .collect(),
            health: store.health_history(agent, 20)?,
        })
    }

    pub fn models_view(&self) -> NexusResult<ModelsView> {
        let overview = self.overview(false)?;
        let snapshots: Vec<AgentSnapshot> =
            overview.agents.iter().map(|a| a.snapshot.clone()).collect();
        let observed = self.observed_models(&snapshots);
        let providers = self.all_providers()?;
        let models = self.all_models(&observed)?;
        let profiles = self.store()?.profiles()?;
        let mut assignments = Vec::new();
        for agent in AgentId::ALL {
            if let Ok(Some(r)) = self.resolve_assignment(agent) {
                assignments.push(r);
            }
        }
        let matrix = models
            .iter()
            .filter_map(|m| {
                providers.iter().find(|p| p.id == m.provider_id).map(|p| {
                    (
                        m.id.clone(),
                        AgentId::ALL
                            .iter()
                            .map(|a| compatibility(*a, p, m))
                            .collect(),
                    )
                })
            })
            .collect();
        Ok(ModelsView {
            providers,
            models,
            profiles,
            assignments,
            matrix,
        })
    }

    // ---- Model domain mutations (Nexus records only; Agent configs change via actions) ----

    pub fn create_provider(
        &self,
        name: &str,
        kind: ProviderKind,
        base_url: Option<&str>,
        auth: AuthMode,
        env_var: Option<&str>,
    ) -> NexusResult<ProviderProfile> {
        let display_name = validate_display_name(name)?;
        let base_url = match (kind, base_url) {
            (ProviderKind::OpenaiCompatible, Some(u)) => Some(validate_base_url(u)?),
            (ProviderKind::OpenaiCompatible, None) => {
                return Err(NexusError::new("provider.invalid_base_url"))
            }
            (_, Some(_)) => return Err(NexusError::new("provider.fixed_origin")),
            (k, None) => k.default_origin().map(String::from),
        };
        if auth == AuthMode::Environment {
            validate_env_var(env_var.ok_or_else(|| NexusError::new("provider.invalid_env_var"))?)?;
        }
        let id = crate::core::new_id("prov");
        let p = ProviderProfile {
            secret_ref: (auth == AuthMode::NexusSecret).then(|| format!("provider:{id}")),
            id,
            display_name,
            kind,
            base_url,
            auth_mode: auth,
            secret_present: Some(false),
            env_var: if auth == AuthMode::Environment {
                env_var.map(String::from)
            } else {
                None
            },
            revision: 1,
            builtin: false,
            connection: "notTested",
        };
        self.store()?.insert_provider(&p, Instant::now())?;
        self.invalidate();
        Ok(p)
    }

    /// Stores a secret in the OS credential store. The value is never returned or persisted
    /// elsewhere; the caller's buffer is dropped when this returns.
    pub fn set_provider_secret(&self, provider_id: &str, secret: String) -> NexusResult<()> {
        let p = self
            .store()?
            .providers()?
            .into_iter()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| NexusError::new("provider.not_found"))?;
        let reference = p
            .secret_ref
            .ok_or_else(|| NexusError::new("provider.no_secret_reference"))?;
        let trimmed = secret.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 || trimmed.chars().any(|c| c.is_control()) {
            return Err(NexusError::new("credential.invalid_value"));
        }
        self.secrets.set(&reference, trimmed.as_bytes())?;
        self.invalidate();
        Ok(())
    }

    pub fn delete_provider(&self, provider_id: &str) -> NexusResult<()> {
        if provider_id.starts_with("builtin.") {
            return Err(NexusError::new("provider.builtin_read_only"));
        }
        let secret = self.store()?.delete_provider(provider_id)?;
        if let Some(reference) = secret {
            self.secrets.delete(&reference)?;
        }
        self.invalidate();
        Ok(())
    }

    pub fn create_model(
        &self,
        provider_id: &str,
        identifier: &str,
        display_name: Option<&str>,
    ) -> NexusResult<ModelDefinition> {
        validate_identifier(identifier)?;
        if !self.all_providers()?.iter().any(|p| p.id == provider_id) {
            return Err(NexusError::new("provider.not_found"));
        }
        let m = ModelDefinition {
            id: crate::core::new_id("model"),
            provider_id: provider_id.into(),
            identifier: identifier.into(),
            display_name: display_name.map(validate_display_name).transpose()?,
            source: "manual".into(),
            source_detail: None,
            capabilities: Vec::new(),
        };
        self.store()?.insert_model(&m, Instant::now())?;
        self.invalidate();
        Ok(m)
    }

    pub fn delete_model(&self, id: &str) -> NexusResult<()> {
        self.store()?.delete_model(id)?;
        self.invalidate();
        Ok(())
    }

    /// Ensures a model id refers to a stored or builtin definition, importing an observed one.
    fn materialize_model(&self, model_id: &str) -> NexusResult<()> {
        if self.all_models(&[])?.iter().any(|m| m.id == model_id) {
            return Ok(());
        }
        let overview = self.overview(false)?;
        let snaps: Vec<AgentSnapshot> =
            overview.agents.iter().map(|a| a.snapshot.clone()).collect();
        let observed = self
            .observed_models(&snaps)
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| NexusError::new("model.not_found"))?;
        self.store()?.insert_model(&observed, Instant::now())
    }

    pub fn save_profile(
        &self,
        id: Option<&str>,
        name: &str,
        model_id: &str,
        reasoning: Option<&str>,
    ) -> NexusResult<ModelProfile> {
        let name = validate_display_name(name)?;
        if let Some(r) = reasoning {
            if !matches!(r, "low" | "medium" | "high") {
                return Err(NexusError::new("profile.invalid_reasoning"));
            }
        }
        self.materialize_model(model_id)?;
        let id = match id {
            Some(existing) => {
                if !self.store()?.profiles()?.iter().any(|p| p.id == existing) {
                    return Err(NexusError::new("profile.not_found"));
                }
                existing.to_string()
            }
            None => crate::core::new_id("profile"),
        };
        let revision =
            self.store()?
                .save_profile(&id, &name, model_id, reasoning, Instant::now())?;
        self.invalidate();
        Ok(ModelProfile {
            id,
            name,
            revision,
            model_id: model_id.into(),
            reasoning: reasoning.map(String::from),
        })
    }

    /// Records desired intent. The Agent configuration is unchanged until an applyModel plan is
    /// previewed, confirmed and verified.
    pub fn set_assignment(
        &self,
        agent: AgentId,
        mode: AssignmentMode,
        model_id: Option<&str>,
        profile_id: Option<&str>,
    ) -> NexusResult<Option<ResolvedAssignment>> {
        let now = Instant::now();
        match mode {
            AssignmentMode::Native => {
                self.store()?.clear_assignment(agent)?;
            }
            AssignmentMode::Direct => {
                let m = model_id.ok_or_else(|| NexusError::new("assignment.invalid"))?;
                self.materialize_model(m)?;
                self.store()?
                    .set_desired(agent, mode, Some(m), None, None, now)?;
            }
            AssignmentMode::Profile => {
                let pid = profile_id.ok_or_else(|| NexusError::new("assignment.invalid"))?;
                let p = self
                    .store()?
                    .profiles()?
                    .into_iter()
                    .find(|p| p.id == pid)
                    .ok_or_else(|| NexusError::new("profile.not_found"))?;
                self.store()?
                    .set_desired(agent, mode, None, Some(pid), Some(p.revision), now)?;
            }
        }
        self.invalidate();
        self.resolve_assignment(agent)
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use crate::secrets::MemorySecretStore;

    pub struct Fixed(pub bool);
    impl Confirmer for Fixed {
        fn confirm(&self, _plan: &PlanView, _locale: &str) -> bool {
            self.0
        }
    }

    pub fn app(home: &std::path::Path, confirm: bool) -> App {
        let ctx = HostContext::isolated(home, &home.join("nexus-data"));
        App::new(
            ctx,
            Store::open_in_memory().unwrap(),
            Arc::new(MemorySecretStore::default()),
            Box::new(Fixed(confirm)),
        )
        .unwrap()
    }
}
