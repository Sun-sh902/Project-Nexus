//! Structured Diagnostics (Architecture §7). Deterministic mapping from Health checks (plus the
//! operation journal) to findings: what was observed, what was expected, how sure Nexus is
//! (`established` vs `possible`), the user impact, and which *registered* repair actions are
//! eligible. A finding without a safe automated repair carries manual guidance instead.

use crate::core::Instant;
use crate::domain::*;
use crate::health::{AgentHealth, Outcome, Requirement};
use crate::store::{BackupRecord, OperationRecord};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairOption {
    /// Action Registry kind passed back to `prepare_action`.
    pub action: &'static str,
    pub params: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticFinding {
    /// Stable identity: `<agent>:<code>`.
    pub id: String,
    /// Translation family `diagnostics:findings.<code>`.
    pub code: String,
    pub agent_id: AgentId,
    pub check_id: Option<&'static str>,
    pub severity: &'static str,
    pub cause: &'static str,
    pub params: Vec<(String, String)>,
    pub evidence: Vec<String>,
    pub observed_at: Instant,
    pub stale: bool,
    pub repairs: Vec<RepairOption>,
    pub manual: Option<&'static str>,
}

pub struct DiagnosticInputs<'a> {
    pub snapshot: &'a AgentSnapshot,
    pub health: &'a AgentHealth,
    pub backups: &'a [BackupRecord],
    pub interrupted: &'a [OperationRecord],
    pub install_recipe_available: bool,
}

pub fn diagnose(input: &DiagnosticInputs) -> Vec<DiagnosticFinding> {
    let snap = input.snapshot;
    let mut out = Vec::new();
    let mut push = |code: &str, check: Option<&'static str>, severity: &'static str, cause: &'static str, params: Vec<(String, String)>, evidence: Vec<String>, repairs: Vec<RepairOption>, manual: Option<&'static str>| {
        out.push(DiagnosticFinding {
            id: format!("{}:{code}", snap.agent_id.as_str()),
            code: code.to_string(),
            agent_id: snap.agent_id,
            check_id: check,
            severity,
            cause,
            params,
            evidence,
            observed_at: snap.observed_at,
            stale: input.health.stale,
            repairs,
            manual,
        });
    };

    // Interrupted Nexus-owned operations come first: they explain partial state.
    for op in input.interrupted {
        let recover = op.action == "install";
        push(
            "operationInterrupted",
            None,
            "warning",
            "established",
            vec![("action".into(), op.action.clone()), ("startedAt".into(), op.started_at.clone()), ("steps".into(), op.steps.join(" → "))],
            vec![],
            if recover { vec![RepairOption { action: "recoverInstall", params: vec![("operationId".into(), op.id.clone())] }] } else { vec![] },
            if recover { None } else { Some("manual.reviewInterrupted") },
        );
    }

    for check in &input.health.checks {
        let failing = check.outcome == Outcome::Fail || (check.outcome == Outcome::Unknown && check.requirement == Requirement::Required);
        if !failing {
            continue;
        }
        let severity = if check.requirement == Requirement::Required && check.outcome == Outcome::Fail { "blocking" } else if check.outcome == Outcome::Unknown { "unknown" } else { "warning" };
        let params = check.params.clone();
        let evidence = check.evidence.clone();
        match (check.id, check.detail) {
            (_, Some("stale")) => push("evidenceStale", Some(check.id), "unknown", "established", vec![], vec![], vec![], Some("manual.refresh")),
            ("platform.supported", _) => push("platformUnsupported", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.platform")),
            ("install.present", _) => push(
                "notInstalled",
                Some(check.id),
                severity,
                "established",
                params,
                evidence,
                if input.install_recipe_available { vec![RepairOption { action: "install", params: vec![] }] } else { vec![] },
                if input.install_recipe_available { None } else { Some("manual.installOfficial") },
            ),
            ("install.executable", _) => push("notExecutable", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.reinstall")),
            ("install.version", d) => push("versionUnknown", Some(check.id), severity, "possible", with(params, "reason", d), evidence, vec![], Some("manual.versionProbe")),
            ("install.unique", _) => push("duplicateInstallations", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.duplicates")),
            ("runtime.prerequisite", d) => push(if d == Some("tooOld") { "runtimeTooOld" } else { "runtimeMissing" }, Some(check.id), severity, if check.outcome == Outcome::Fail { "established" } else { "possible" }, params, evidence, vec![], Some("manual.installRuntime")),
            ("config.valid" | "config.validTolerated", Some("parseFailed")) => {
                let location = check.params.iter().find(|(k, _)| k == "location").map(|(_, v)| v.clone());
                let restorable: Vec<RepairOption> = location
                    .as_ref()
                    .and_then(|loc| input.backups.iter().find(|b| &b.target_display == loc))
                    .map(|b| vec![RepairOption { action: "restoreBackup", params: vec![("backupId".into(), b.id.clone())] }])
                    .unwrap_or_default();
                let manual = if restorable.is_empty() { Some("manual.fixConfig") } else { None };
                push("configMalformed", Some(check.id), severity, "established", params, evidence, restorable, manual);
            }
            ("config.valid" | "config.validTolerated", _) => push("configUnreadable", Some(check.id), severity, "possible", params, evidence, vec![], Some("manual.permissions")),
            ("config.unambiguous", _) => push("configAmbiguous", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.mergeConfig")),
            ("model.compatible", _) => push("modelIncompatible", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.chooseCompatible")),
            ("model.applied", _) => push("assignmentPending", Some(check.id), severity, "established", params, evidence, vec![RepairOption { action: "applyModel", params: vec![] }], None),
            ("model.drift", Some("drift")) => push("assignmentDrift", Some(check.id), severity, "established", params, evidence, vec![RepairOption { action: "applyModel", params: vec![] }], None),
            ("model.drift", _) => push("modelOverridden", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.unsetOverride")),
            ("env.overrides", _) => push("environmentOverride", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.unsetOverride")),
            ("extensions.mcp", Some("commandMissing")) => {
                let names: Vec<String> = snap.extensions.mcp.iter().filter(|m| m.enabled != Some(false) && m.command_resolvable == Some(false)).map(|m| m.name.clone()).collect();
                let toggleable = snap.extensions.mcp.iter().filter(|m| names.contains(&m.name) && m.can_toggle).map(|m| RepairOption { action: "setMcpEnabled", params: vec![("name".into(), m.name.clone()), ("enabled".into(), "false".into())] }).collect::<Vec<_>>();
                let manual = if toggleable.is_empty() { Some("manual.fixMcpCommand") } else { None };
                push("mcpCommandMissing", Some(check.id), severity, "possible", params, evidence, toggleable, manual);
            }
            ("extensions.skills", _) => push("skillInvalid", Some(check.id), severity, "established", params, evidence, vec![], Some("manual.fixSkill")),
            (id, _) => push("checkFailed", Some(id), severity, "possible", vec![("check".into(), id.to_string())], evidence, vec![], None),
        }
    }
    out
}

fn with(mut params: Vec<(String, String)>, key: &str, value: Option<&str>) -> Vec<(String, String)> {
    if let Some(v) = value {
        params.push((key.into(), v.into()));
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::fixtures::*;
    use crate::health::{evaluate, AssignmentContext};

    const T: Instant = Instant(1_800_000_000);

    #[test]
    fn missing_agent_offers_install_only_when_a_recipe_exists() {
        let s = snapshot(AgentId::Gemini, vec![], T);
        let h = evaluate(&s, &system(T, None), &AssignmentContext::default(), T);
        let f = diagnose(&DiagnosticInputs { snapshot: &s, health: &h, backups: &[], interrupted: &[], install_recipe_available: true });
        assert_eq!(f[0].code, "notInstalled");
        assert_eq!(f[0].repairs[0].action, "install");
        let f = diagnose(&DiagnosticInputs { snapshot: &s, health: &h, backups: &[], interrupted: &[], install_recipe_available: false });
        assert!(f[0].repairs.is_empty() && f[0].manual.is_some());
    }

    #[test]
    fn malformed_config_offers_restore_only_with_matching_backup() {
        let mut s = snapshot(AgentId::Codex, vec![installation(Some("1.0.0"), T)], T);
        s.config_sources[0].status = ObsStatus::ParseFailed;
        let h = evaluate(&s, &system(T, None), &AssignmentContext::default(), T);
        let none = diagnose(&DiagnosticInputs { snapshot: &s, health: &h, backups: &[], interrupted: &[], install_recipe_available: false });
        let f = none.iter().find(|f| f.code == "configMalformed").unwrap();
        assert!(f.repairs.is_empty() && f.manual == Some("manual.fixConfig"));
        let b = BackupRecord { id: "bak_1".into(), operation_id: "op".into(), agent_id: Some("codex".into()), target_id: "u".into(), target_display: "~/.agent/settings.json".into(), sha256: "x".into(), size: 2, created_at: "t".into(), target_path: "/x".into(), written_fingerprint: None };
        let some = diagnose(&DiagnosticInputs { snapshot: &s, health: &h, backups: &[b], interrupted: &[], install_recipe_available: false });
        assert_eq!(some.iter().find(|f| f.code == "configMalformed").unwrap().repairs[0].action, "restoreBackup");
    }

    #[test]
    fn interrupted_install_is_recoverable_finding() {
        let s = snapshot(AgentId::Gemini, vec![], T);
        let h = evaluate(&s, &system(T, None), &AssignmentContext::default(), T);
        let op = OperationRecord { id: "op_9".into(), action: "install".into(), agent_id: Some("gemini".into()), status: "interrupted".into(), started_at: "t".into(), finished_at: None, error_code: None, summary: serde_json::json!({}), steps: vec!["started".into()] };
        let f = diagnose(&DiagnosticInputs { snapshot: &s, health: &h, backups: &[], interrupted: &[op], install_recipe_available: true });
        assert_eq!(f[0].code, "operationInterrupted");
        assert_eq!(f[0].repairs[0].action, "recoverInstall");
    }

    #[test]
    fn healthy_agent_has_no_findings() {
        let s = snapshot(AgentId::Codex, vec![installation(Some("1.0.0"), T)], T);
        let h = evaluate(&s, &system(T, None), &AssignmentContext::default(), T);
        assert!(diagnose(&DiagnosticInputs { snapshot: &s, health: &h, backups: &[], interrupted: &[], install_recipe_available: false }).is_empty());
    }
}
