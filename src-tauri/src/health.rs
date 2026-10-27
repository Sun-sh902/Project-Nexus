//! Deterministic, evidence-based Agent Health (Architecture §6, ADR 003). A pure function of
//! (snapshot, system snapshot, assignment context, evaluation time). No I/O, no AI, no network.
//! Aggregation order is normative: unsupported → blocked → unknown → attention → ready.

use crate::core::Instant;
use crate::discovery::system::SystemSnapshot;
use crate::discovery::SNAPSHOT_TTL_SECS;
use crate::domain::*;
use serde::Serialize;

pub const RULE_SET: &str = "nexus-readiness-1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Requirement {
    Required,
    Advisory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Pass,
    Fail,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    /// Stable check identifier; also the translation key family `health:checks.<id>`.
    pub id: &'static str,
    pub category: &'static str,
    pub requirement: Requirement,
    pub outcome: Outcome,
    /// Refines the explanation (`health:checks.<id>.<detail>`), e.g. `parseFailed`.
    pub detail: Option<&'static str>,
    pub params: Vec<(String, String)>,
    pub evidence: Vec<String>,
    pub source: Option<EvidenceSource>,
    pub evaluated_at: Instant,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub required: usize,
    pub required_passed: usize,
    pub advisory: usize,
    pub advisory_passed: usize,
    pub unknown: usize,
    /// Areas never evaluated by routine Health (explicit, not silently passed).
    pub not_checked: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHealth {
    pub agent_id: AgentId,
    pub readiness: Readiness,
    pub checks: Vec<HealthCheck>,
    pub coverage: Coverage,
    pub rule_set: &'static str,
    pub snapshot_id: String,
    pub evaluated_at: Instant,
    pub valid_until: Instant,
    pub stale: bool,
}

/// Nexus intent relevant to Health, already resolved by the application layer.
#[derive(Clone, Debug, Default)]
pub struct AssignmentContext {
    /// Identifier the desired assignment resolves to for this Agent.
    pub desired_identifier: Option<String>,
    pub compatible: Option<Support>,
    pub applied_identifier: Option<String>,
    pub desired_matches_applied: bool,
}

struct Builder {
    checks: Vec<HealthCheck>,
    at: Instant,
}

impl Builder {
    fn push(&mut self, id: &'static str, category: &'static str, requirement: Requirement, outcome: Outcome, detail: Option<&'static str>) -> &mut HealthCheck {
        self.checks.push(HealthCheck { id, category, requirement, outcome, detail, params: Vec::new(), evidence: Vec::new(), source: None, evaluated_at: self.at });
        self.checks.last_mut().unwrap()
    }
}

fn status_detail(s: ObsStatus) -> &'static str {
    match s {
        ObsStatus::Observed => "observed",
        ObsStatus::NotFound => "notFound",
        ObsStatus::AccessDenied => "accessDenied",
        ObsStatus::Timeout => "timeout",
        ObsStatus::ParseFailed => "parseFailed",
        ObsStatus::NotChecked => "notChecked",
        ObsStatus::Unsafe => "unsafe",
    }
}

pub fn evaluate(snap: &AgentSnapshot, system: &SystemSnapshot, assignment: &AssignmentContext, now: Instant) -> AgentHealth {
    let mut b = Builder { checks: Vec::new(), at: now };
    let stale = now.0 > snap.observed_at.0 + SNAPSHOT_TTL_SECS || now.0 > system.observed_at.0 + SNAPSHOT_TTL_SECS;

    // Platform
    match snap.platform {
        Support::Unsupported => {
            b.push("platform.supported", "platform", Requirement::Required, Outcome::Fail, None);
        }
        Support::Unverified => {
            b.push("platform.supported", "platform", Requirement::Required, Outcome::Pass, Some("unverified"));
        }
        Support::Supported => {
            b.push("platform.supported", "platform", Requirement::Required, Outcome::Pass, None);
        }
    }

    // Installation
    let primary = snap.primary_installation();
    {
        let c = b.push("install.present", "installation", Requirement::Required, if primary.is_some() { Outcome::Pass } else { Outcome::Fail }, None);
        c.source = Some(EvidenceSource::FileMetadata);
        if let Some(p) = primary {
            c.evidence.push(p.executable.clone());
            c.params.push(("location".into(), p.executable.clone()));
        }
    }
    match primary {
        None => {
            b.push("install.executable", "installation", Requirement::Required, Outcome::NotApplicable, Some("requiresInstall"));
            b.push("install.version", "installation", Requirement::Required, Outcome::NotApplicable, Some("requiresInstall"));
        }
        Some(p) => {
            let c = b.push("install.executable", "installation", Requirement::Required, if p.executable_ok { Outcome::Pass } else { Outcome::Fail }, None);
            c.evidence.push(p.resolved.clone().unwrap_or_else(|| p.executable.clone()));
            let (outcome, detail) = match p.version.status {
                ObsStatus::Observed => (Outcome::Pass, None),
                other => (Outcome::Unknown, Some(status_detail(other))),
            };
            let c = b.push("install.version", "installation", Requirement::Required, outcome, detail);
            c.source = Some(p.version.source);
            if let Some(v) = &p.version.value {
                c.params.push(("version".into(), v.clone()));
            }
            c.evidence.extend(p.version.location.clone());
        }
    }
    {
        let n = snap.installations.len();
        let c = b.push("install.unique", "installation", Requirement::Advisory, if n > 1 { Outcome::Fail } else if n == 1 { Outcome::Pass } else { Outcome::NotApplicable }, None);
        c.params.push(("count".into(), n.to_string()));
        c.evidence = snap.installations.iter().map(|i| i.executable.clone()).collect();
    }

    // Runtime prerequisites
    match (&snap.runtime_requirement, primary) {
        (Some(req), Some(_)) => {
            let tool = system.tools.iter().find(|t| t.key == req.tool);
            let (outcome, detail) = match tool {
                None => (Outcome::Unknown, Some("notChecked")),
                Some(t) if !t.present => (Outcome::Fail, Some("missing")),
                Some(t) => match (&t.version.value, req.minimum) {
                    (_, None) => (Outcome::Pass, None),
                    (Some(v), Some(min)) => match version_at_least(v, min) {
                        Some(true) => (Outcome::Pass, None),
                        Some(false) => (Outcome::Fail, Some("tooOld")),
                        None => (Outcome::Unknown, Some("parseFailed")),
                    },
                    (None, Some(_)) => (Outcome::Unknown, Some(status_detail(t.version.status))),
                },
            };
            let c = b.push("runtime.prerequisite", "runtime", Requirement::Required, outcome, detail);
            c.params.push(("tool".into(), req.tool.to_string()));
            if let Some(min) = req.minimum {
                c.params.push(("minimum".into(), min.to_string()));
            }
            if let Some(v) = tool.and_then(|t| t.version.value.clone()) {
                c.params.push(("version".into(), v));
            }
            c.evidence.push(req.evidence.to_string());
        }
        (Some(_), None) => {
            b.push("runtime.prerequisite", "runtime", Requirement::Required, Outcome::NotApplicable, Some("requiresInstall"));
        }
        (None, _) => {
            b.push("runtime.prerequisite", "runtime", Requirement::Required, Outcome::NotApplicable, Some("noRequirement"));
        }
    }

    // Configuration validity: Agents that refuse to start on a broken file → required.
    for (id, tolerant) in [("config.valid", false), ("config.validTolerated", true)] {
        let relevant: Vec<&ConfigSource> = snap.config_sources.iter().filter(|c| c.tolerant == tolerant && c.status != ObsStatus::NotFound).collect();
        let broken: Vec<&&ConfigSource> = relevant.iter().filter(|c| c.status == ObsStatus::ParseFailed).collect();
        let unreadable: Vec<&&ConfigSource> = relevant.iter().filter(|c| matches!(c.status, ObsStatus::AccessDenied | ObsStatus::Unsafe | ObsStatus::Timeout)).collect();
        let (outcome, detail) = if !broken.is_empty() {
            (Outcome::Fail, Some("parseFailed"))
        } else if !unreadable.is_empty() {
            (Outcome::Unknown, Some("accessDenied"))
        } else if relevant.is_empty() {
            (Outcome::NotApplicable, Some("noFile"))
        } else {
            (Outcome::Pass, None)
        };
        if tolerant && relevant.is_empty() && snap.config_sources.iter().all(|c| !c.tolerant) {
            continue;
        }
        let c = b.push(id, "configuration", if tolerant { Requirement::Advisory } else { Requirement::Required }, outcome, detail);
        c.source = Some(EvidenceSource::ConfigFile);
        c.evidence = broken.iter().chain(unreadable.iter()).map(|s| s.location.clone()).collect();
        if c.evidence.is_empty() {
            c.evidence = relevant.iter().map(|s| s.location.clone()).collect();
        }
        if let Some(first) = broken.first() {
            c.params.push(("location".into(), first.location.clone()));
        }
    }
    if snap.agent_id == AgentId::Opencode {
        let globals: Vec<&ConfigSource> = snap.config_sources.iter().filter(|c| c.id.starts_with("opencode.global") && c.status != ObsStatus::NotFound).collect();
        let c = b.push("config.unambiguous", "configuration", Requirement::Advisory, if globals.len() > 1 { Outcome::Fail } else { Outcome::Pass }, None);
        c.evidence = globals.iter().map(|g| g.location.clone()).collect();
    }

    // Authentication evidence: presence only; absence never proves the Agent cannot work.
    {
        let positive = snap.auth.evidence.iter().any(|e| *e != AuthEvidence::OsStoreUnverified);
        let os_store = snap.auth.evidence.contains(&AuthEvidence::OsStoreUnverified);
        let (outcome, detail) = if primary.is_none() {
            (Outcome::NotApplicable, Some("requiresInstall"))
        } else if positive {
            (Outcome::Pass, None)
        } else if os_store {
            (Outcome::Unknown, Some("osStore"))
        } else {
            (Outcome::Unknown, Some("noEvidence"))
        };
        let c = b.push("auth.evidence", "authentication", Requirement::Advisory, outcome, detail);
        c.evidence = snap.auth.locations.clone();
        c.params.push(("kinds".into(), snap.auth.evidence.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>().join(",")));
    }

    // Effective model
    {
        let ambiguous = snap.model.layers.iter().any(|l| l.uncertain);
        let (outcome, detail) = match (&snap.model.effective, ambiguous) {
            (_, true) => (Outcome::Unknown, Some("ambiguous")),
            (Some(_), false) => (Outcome::Pass, None),
            (None, false) => (Outcome::Pass, Some("agentDefault")),
        };
        let c = b.push("model.effective", "model", Requirement::Advisory, outcome, detail);
        if let Some(m) = &snap.model.effective {
            c.params.push(("model".into(), m.clone()));
        }
        c.params.push(("source".into(), format!("{:?}", snap.model.effective_kind)));
        c.evidence = snap.model.layers.iter().filter(|l| l.effective || l.uncertain).map(|l| l.location.clone()).collect();
    }

    // Nexus assignment (only when the user asked Nexus to manage the model)
    match &assignment.desired_identifier {
        None => {
            b.push("model.compatible", "model", Requirement::Required, Outcome::NotApplicable, Some("noAssignment"));
        }
        Some(desired) => {
            let outcome = match assignment.compatible {
                Some(Support::Unsupported) => Outcome::Fail,
                Some(_) => Outcome::Pass,
                None => Outcome::Unknown,
            };
            let c = b.push("model.compatible", "model", Requirement::Required, outcome, None);
            c.params.push(("model".into(), desired.clone()));
            let user_value = snap.model.layers.iter().find(|l| l.kind == ModelSourceKind::UserConfig).and_then(|l| l.value.clone());
            match &assignment.applied_identifier {
                None => {
                    let c = b.push("model.applied", "model", Requirement::Advisory, Outcome::Fail, Some("pending"));
                    c.params.push(("model".into(), desired.clone()));
                }
                Some(applied) if !assignment.desired_matches_applied => {
                    let c = b.push("model.applied", "model", Requirement::Advisory, Outcome::Fail, Some("pending"));
                    c.params.push(("model".into(), desired.clone()));
                    c.params.push(("applied".into(), applied.clone()));
                }
                Some(applied) => {
                    b.push("model.applied", "model", Requirement::Advisory, Outcome::Pass, None).params.push(("model".into(), applied.clone()));
                    // Drift: the editable config no longer holds what Nexus applied.
                    let (outcome, detail) = if user_value.as_deref() != Some(applied.as_str()) {
                        (Outcome::Fail, Some("drift"))
                    } else if snap.model.effective.as_deref() != Some(applied.as_str()) && snap.model.effective_kind != ModelSourceKind::UserConfig {
                        (Outcome::Fail, Some("overridden"))
                    } else if snap.model.layers.iter().any(|l| l.uncertain) {
                        (Outcome::Unknown, Some("possiblyOverridden"))
                    } else {
                        (Outcome::Pass, None)
                    };
                    let c = b.push("model.drift", "model", Requirement::Advisory, outcome, detail);
                    c.params.push(("applied".into(), applied.clone()));
                    if let Some(v) = user_value {
                        c.params.push(("current".into(), v));
                    }
                    if let Some(e) = &snap.model.effective {
                        c.params.push(("effective".into(), e.clone()));
                    }
                    c.params.push(("source".into(), format!("{:?}", snap.model.effective_kind)));
                    c.evidence = snap.model.layers.iter().filter(|l| l.effective || l.uncertain || l.kind == ModelSourceKind::UserConfig).map(|l| l.location.clone()).collect();
                }
            }
        }
    }

    // Environment overrides active in Nexus's own environment (Agents Nexus launches inherit them).
    {
        let process: Vec<&EnvOverride> = snap.env_overrides.iter().filter(|o| o.origin == "process").collect();
        let profile: Vec<&EnvOverride> = snap.env_overrides.iter().filter(|o| o.origin == "shellProfile").collect();
        let (outcome, detail) = if !process.is_empty() {
            (Outcome::Fail, Some("process"))
        } else if !profile.is_empty() {
            (Outcome::Unknown, Some("shellProfile"))
        } else {
            (Outcome::Pass, None)
        };
        let c = b.push("env.overrides", "environment", Requirement::Advisory, outcome, detail);
        c.params.push(("variables".into(), process.iter().chain(profile.iter()).map(|o| o.variable.clone()).collect::<Vec<_>>().join(", ")));
        c.evidence = profile.iter().filter_map(|o| o.location.clone()).collect();
        c.source = Some(if process.is_empty() { EvidenceSource::ShellProfile } else { EvidenceSource::Environment });
    }

    // Extensions
    {
        let missing: Vec<&McpServer> = snap.extensions.mcp.iter().filter(|m| m.enabled != Some(false) && m.command_resolvable == Some(false)).collect();
        let (outcome, detail) = if matches!(snap.extensions.mcp_status, ObsStatus::ParseFailed | ObsStatus::AccessDenied) {
            (Outcome::Unknown, Some(status_detail(snap.extensions.mcp_status)))
        } else if !missing.is_empty() {
            (Outcome::Fail, Some("commandMissing"))
        } else if snap.extensions.mcp.is_empty() {
            (Outcome::NotApplicable, Some("none"))
        } else {
            (Outcome::Pass, None)
        };
        let c = b.push("extensions.mcp", "extensions", Requirement::Advisory, outcome, detail);
        c.params.push(("servers".into(), missing.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(", ")));
        c.params.push(("count".into(), snap.extensions.mcp.len().to_string()));
        c.evidence = missing.iter().map(|m| m.location.clone()).collect();
    }
    {
        let invalid: Vec<&Skill> = snap.extensions.skills.iter().filter(|s| !s.valid).collect();
        let (outcome, detail) = if !invalid.is_empty() {
            (Outcome::Fail, Some("invalid"))
        } else if snap.extensions.skills.is_empty() {
            (Outcome::NotApplicable, Some("none"))
        } else {
            (Outcome::Pass, None)
        };
        let c = b.push("extensions.skills", "extensions", Requirement::Advisory, outcome, detail);
        c.params.push(("skills".into(), invalid.iter().map(|s| s.name.clone()).collect::<Vec<_>>().join(", ")));
        c.params.push(("count".into(), snap.extensions.skills.len().to_string()));
        c.evidence = invalid.iter().map(|s| s.location.clone()).collect();
    }

    let mut checks = b.checks;
    if stale {
        // Stale evidence can never become fresh truth: required results degrade to unknown.
        for c in checks.iter_mut().filter(|c| c.requirement == Requirement::Required && c.outcome != Outcome::NotApplicable) {
            c.outcome = Outcome::Unknown;
            c.detail = Some("stale");
        }
    }
    let readiness = aggregate(&checks);
    let coverage = Coverage {
        required: checks.iter().filter(|c| c.requirement == Requirement::Required && c.outcome != Outcome::NotApplicable).count(),
        required_passed: checks.iter().filter(|c| c.requirement == Requirement::Required && c.outcome == Outcome::Pass).count(),
        advisory: checks.iter().filter(|c| c.requirement == Requirement::Advisory && c.outcome != Outcome::NotApplicable).count(),
        advisory_passed: checks.iter().filter(|c| c.requirement == Requirement::Advisory && c.outcome == Outcome::Pass).count(),
        unknown: checks.iter().filter(|c| c.outcome == Outcome::Unknown).count(),
        not_checked: vec!["onlineService", "credentialValidity", "projectScope"],
    };
    AgentHealth {
        agent_id: snap.agent_id,
        readiness,
        checks,
        coverage,
        rule_set: RULE_SET,
        snapshot_id: snap.snapshot_id.clone(),
        evaluated_at: now,
        valid_until: snap.observed_at.plus_secs(SNAPSHOT_TTL_SECS),
        stale,
    }
}

pub fn aggregate(checks: &[HealthCheck]) -> Readiness {
    if checks.iter().any(|c| c.id == "platform.supported" && c.outcome == Outcome::Fail) {
        return Readiness::Unsupported;
    }
    if checks.iter().any(|c| c.requirement == Requirement::Required && c.outcome == Outcome::Fail) {
        return Readiness::Blocked;
    }
    if checks.iter().any(|c| c.requirement == Requirement::Required && c.outcome == Outcome::Unknown) {
        return Readiness::Unknown;
    }
    if checks.iter().any(|c| c.requirement == Requirement::Advisory && c.outcome == Outcome::Fail) {
        return Readiness::Attention;
    }
    Readiness::Ready
}

#[cfg(test)]
pub mod fixtures {
    use super::*;
    use crate::discovery::system::ToolObservation;
    use crate::platform::Os;

    pub fn system(at: Instant, node: Option<&str>) -> SystemSnapshot {
        SystemSnapshot {
            observed_at: at,
            os: Os::Macos,
            os_version: Observed::ok("27.2".into(), EvidenceSource::PlatformApi, None, at),
            arch: "aarch64",
            shell: None,
            tools: vec![ToolObservation {
                key: "node",
                name: "Node.js",
                present: node.is_some(),
                location: node.map(|_| "/opt/homebrew/bin/node".into()),
                version: match node {
                    Some(v) => Observed::ok(v.into(), EvidenceSource::VersionProbe, None, at),
                    None => Observed::missing(ObsStatus::NotFound, EvidenceSource::FileMetadata, None, at),
                },
                candidates: node.is_some() as usize,
            }],
            path_entries: vec![],
            data_dir: "~/data".into(),
            data_dir_writable: true,
            gui_path_note: true,
        }
    }

    pub fn installation(version: Option<&str>, at: Instant) -> Installation {
        Installation {
            id: "inst_1".into(),
            executable: "~/.local/bin/agent".into(),
            resolved: None,
            source: InstallSource::Npm,
            version: match version {
                Some(v) => Observed::ok(v.into(), EvidenceSource::PackageManifest, None, at),
                None => Observed::missing(ObsStatus::Timeout, EvidenceSource::VersionProbe, None, at),
            },
            path_rank: Some(0),
            executable_ok: true,
            nexus_managed: false,
            wrapper: false,
        }
    }

    pub fn snapshot(agent: AgentId, installs: Vec<Installation>, at: Instant) -> AgentSnapshot {
        AgentSnapshot {
            snapshot_id: "snap_x".into(),
            agent_id: agent,
            name: "Agent",
            vendor: "Vendor",
            observed_at: at,
            platform: Support::Supported,
            primary: if installs.is_empty() { None } else { Some(0) },
            installations: installs,
            config_sources: vec![ConfigSource { id: "u".into(), scope: ConfigScope::User, format: ConfigFormat::Json, location: "~/.agent/settings.json".into(), status: ObsStatus::Observed, fingerprint: Some("f".into()), via_symlink: false, tolerant: false, path: Default::default() }],
            model: ModelObservation { effective: None, effective_kind: ModelSourceKind::AgentDefault, layers: vec![], provider_hint: None },
            auth: AuthObservation { evidence: vec![AuthEvidence::AgentLoginPresent], locations: vec![] },
            extensions: ExtensionObservation { mcp: vec![], mcp_status: ObsStatus::Observed, skills: vec![], skills_status: ObsStatus::NotFound },
            env_overrides: vec![],
            capabilities: vec![],
            runtime_requirement: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    const T: Instant = Instant(1_800_000_000);

    fn eval(s: &AgentSnapshot) -> AgentHealth {
        evaluate(s, &system(T, Some("26.10.0")), &AssignmentContext::default(), T)
    }

    #[test]
    fn healthy_installation_is_ready_with_explicit_coverage() {
        let h = eval(&snapshot(AgentId::Codex, vec![installation(Some("0.164.0"), T)], T));
        assert_eq!(h.readiness, Readiness::Ready);
        assert!(h.coverage.not_checked.contains(&"onlineService"));
        assert!(h.coverage.required > 0 && h.coverage.required == h.coverage.required_passed);
    }

    #[test]
    fn missing_agent_is_blocked_not_unsupported() {
        assert_eq!(eval(&snapshot(AgentId::Gemini, vec![], T)).readiness, Readiness::Blocked);
    }

    #[test]
    fn unsupported_platform_wins() {
        let mut s = snapshot(AgentId::Codex, vec![], T);
        s.platform = Support::Unsupported;
        assert_eq!(eval(&s).readiness, Readiness::Unsupported);
    }

    #[test]
    fn version_timeout_is_unknown_not_ready() {
        assert_eq!(eval(&snapshot(AgentId::Codex, vec![installation(None, T)], T)).readiness, Readiness::Unknown);
    }

    #[test]
    fn duplicate_installations_need_attention() {
        let mut second = installation(Some("1.0.0"), T);
        second.id = "inst_2".into();
        second.path_rank = Some(3);
        let h = eval(&snapshot(AgentId::Codex, vec![installation(Some("1.0.0"), T), second], T));
        assert_eq!(h.readiness, Readiness::Attention);
    }

    #[test]
    fn stale_evidence_never_stays_ready_or_blocked() {
        let s = snapshot(AgentId::Codex, vec![installation(Some("1.0.0"), T)], T);
        let later = T.plus_secs(SNAPSHOT_TTL_SECS + 1);
        let h = evaluate(&s, &system(T, Some("26.0.0")), &AssignmentContext::default(), later);
        assert!(h.stale);
        assert_eq!(h.readiness, Readiness::Unknown);
        let missing = snapshot(AgentId::Codex, vec![], T);
        assert_eq!(evaluate(&missing, &system(T, None), &AssignmentContext::default(), later).readiness, Readiness::Unknown);
    }

    #[test]
    fn runtime_prerequisite_blocks_when_missing_or_too_old() {
        let mut s = snapshot(AgentId::Gemini, vec![installation(Some("0.64.0"), T)], T);
        s.runtime_requirement = Some(RuntimeRequirement { tool: "node", minimum: Some("20.0.0"), evidence: "doc" });
        assert_eq!(evaluate(&s, &system(T, None), &AssignmentContext::default(), T).readiness, Readiness::Blocked);
        let h = evaluate(&s, &system(T, Some("18.19.0")), &AssignmentContext::default(), T);
        assert_eq!(h.readiness, Readiness::Blocked);
        assert_eq!(h.checks.iter().find(|c| c.id == "runtime.prerequisite").unwrap().detail, Some("tooOld"));
        assert_eq!(evaluate(&s, &system(T, Some("20.1.0")), &AssignmentContext::default(), T).readiness, Readiness::Ready);
    }

    #[test]
    fn malformed_config_blocks_unless_agent_tolerates_it() {
        let mut s = snapshot(AgentId::Codex, vec![installation(Some("1.0.0"), T)], T);
        s.config_sources[0].status = ObsStatus::ParseFailed;
        assert_eq!(eval(&s).readiness, Readiness::Blocked);
        s.config_sources[0].tolerant = true;
        assert_eq!(eval(&s).readiness, Readiness::Attention);
    }

    #[test]
    fn missing_auth_evidence_is_never_a_failure() {
        let mut s = snapshot(AgentId::Claude, vec![installation(Some("2.1.0"), T)], T);
        s.auth.evidence.clear();
        let h = eval(&s);
        assert_eq!(h.readiness, Readiness::Ready);
        assert_eq!(h.checks.iter().find(|c| c.id == "auth.evidence").unwrap().outcome, Outcome::Unknown);
    }

    #[test]
    fn model_drift_and_env_override_are_distinguished() {
        let mut s = snapshot(AgentId::Claude, vec![installation(Some("2.1.0"), T)], T);
        s.model.layers = vec![
            ModelLayer { kind: ModelSourceKind::Environment, location: "ANTHROPIC_MODEL".into(), value: Some("sonnet".into()), effective: true, uncertain: false },
            ModelLayer { kind: ModelSourceKind::UserConfig, location: "~/.claude/settings.json".into(), value: Some("opus".into()), effective: false, uncertain: false },
        ];
        s.model.effective = Some("sonnet".into());
        s.model.effective_kind = ModelSourceKind::Environment;
        let ctx = AssignmentContext { desired_identifier: Some("opus".into()), compatible: Some(Support::Supported), applied_identifier: Some("opus".into()), desired_matches_applied: true };
        let h = evaluate(&s, &system(T, None), &ctx, T);
        assert_eq!(h.checks.iter().find(|c| c.id == "model.drift").unwrap().detail, Some("overridden"));
        assert_eq!(h.readiness, Readiness::Attention);
        s.model.layers[1].value = Some("haiku".into());
        let h = evaluate(&s, &system(T, None), &ctx, T);
        assert_eq!(h.checks.iter().find(|c| c.id == "model.drift").unwrap().detail, Some("drift"));
    }

    #[test]
    fn incompatible_assignment_blocks() {
        let s = snapshot(AgentId::Gemini, vec![installation(Some("1.0.0"), T)], T);
        let ctx = AssignmentContext { desired_identifier: Some("opus".into()), compatible: Some(Support::Unsupported), applied_identifier: None, desired_matches_applied: false };
        assert_eq!(evaluate(&s, &system(T, Some("26.0.0")), &ctx, T).readiness, Readiness::Blocked);
    }

    #[test]
    fn unresolvable_mcp_command_needs_attention() {
        let mut s = snapshot(AgentId::Codex, vec![installation(Some("1.0.0"), T)], T);
        s.extensions.mcp.push(McpServer { name: "fs".into(), transport: McpTransport::Stdio, enabled: Some(true), command: Some("mcp-fs".into()), command_resolvable: Some(false), endpoint_origin: None, env_names: vec![], scope: ConfigScope::User, location: "~/.codex/config.toml".into(), nexus_managed: false, can_toggle: true });
        assert_eq!(eval(&s).readiness, Readiness::Attention);
        s.extensions.mcp[0].enabled = Some(false);
        assert_eq!(eval(&s).readiness, Readiness::Ready);
    }

    #[test]
    fn evaluation_is_deterministic() {
        let s = snapshot(AgentId::Codex, vec![installation(Some("1.0.0"), T)], T);
        let a = serde_json::to_string(&eval(&s)).unwrap();
        let b = serde_json::to_string(&eval(&s)).unwrap();
        assert_eq!(a, b);
    }
}
