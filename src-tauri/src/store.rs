//! Local persistence (SQLite). Holds Nexus intent and history only: Providers (non-secret),
//! Models, Profile revisions, Assignments, operation journal, backup index, Health history and
//! the ownership ledger. External configuration remains the observed truth; secrets live only in
//! the OS credential store, referenced here by opaque name.

use crate::core::{Instant, NexusError, NexusResult};
use crate::domain::AgentId;
use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 1;

const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    r#"
CREATE TABLE providers (
  id TEXT PRIMARY KEY, display_name TEXT NOT NULL, kind TEXT NOT NULL, base_url TEXT,
  auth_mode TEXT NOT NULL, secret_ref TEXT, env_var TEXT, revision INTEGER NOT NULL,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE models (
  id TEXT PRIMARY KEY, provider_id TEXT NOT NULL, identifier TEXT NOT NULL, display_name TEXT,
  source TEXT NOT NULL, source_detail TEXT, capabilities_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL, UNIQUE(provider_id, identifier));
CREATE TABLE model_profiles (
  id TEXT NOT NULL, revision INTEGER NOT NULL, name TEXT NOT NULL, model_id TEXT NOT NULL,
  reasoning TEXT, created_at TEXT NOT NULL, PRIMARY KEY (id, revision));
CREATE TABLE assignments (
  agent_id TEXT PRIMARY KEY, mode TEXT NOT NULL, model_id TEXT, profile_id TEXT,
  desired_revision INTEGER, applied_identifier TEXT, applied_profile_revision INTEGER,
  applied_provider_revision INTEGER, applied_at TEXT, applied_location TEXT,
  applied_fingerprint TEXT, updated_at TEXT NOT NULL);
CREATE TABLE operations (
  id TEXT PRIMARY KEY, action TEXT NOT NULL, agent_id TEXT, status TEXT NOT NULL,
  plan_digest TEXT NOT NULL, summary_json TEXT NOT NULL, started_at TEXT NOT NULL,
  finished_at TEXT, error_code TEXT);
CREATE TABLE operation_steps (
  operation_id TEXT NOT NULL, seq INTEGER NOT NULL, step TEXT NOT NULL, at TEXT NOT NULL,
  PRIMARY KEY (operation_id, seq));
CREATE TABLE backups (
  id TEXT PRIMARY KEY, operation_id TEXT NOT NULL, agent_id TEXT, target_id TEXT NOT NULL,
  target_path TEXT NOT NULL, target_display TEXT NOT NULL, sha256 TEXT NOT NULL,
  size INTEGER NOT NULL, written_fingerprint TEXT, created_at TEXT NOT NULL);
CREATE TABLE health_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL, readiness TEXT NOT NULL,
  rule_set TEXT NOT NULL, issue_codes TEXT NOT NULL, evaluated_at TEXT NOT NULL);
CREATE TABLE managed_extensions (
  agent_id TEXT NOT NULL, name TEXT NOT NULL, origin TEXT, created_at TEXT NOT NULL,
  operation_id TEXT NOT NULL, PRIMARY KEY (agent_id, name));
CREATE TABLE nexus_installations (
  agent_id TEXT NOT NULL, recipe_id TEXT NOT NULL, version TEXT, installation_id TEXT,
  operation_id TEXT NOT NULL, status TEXT NOT NULL, at TEXT NOT NULL,
  PRIMARY KEY (agent_id, recipe_id));
"#,
)];

pub struct Store {
    conn: Connection,
}

fn db_err(_: rusqlite::Error) -> NexusError {
    NexusError::new("store.io_error")
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: String,
    pub action: String,
    pub agent_id: Option<String>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error_code: Option<String>,
    pub summary: serde_json::Value,
    pub steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub operation_id: String,
    pub agent_id: Option<String>,
    pub target_id: String,
    pub target_display: String,
    pub sha256: String,
    pub size: i64,
    pub created_at: String,
    #[serde(skip)]
    pub target_path: String,
    /// Fingerprint Nexus wrote after this backup (lets a later restore detect intervening edits).
    #[serde(skip)]
    #[allow(dead_code)]
    pub written_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthHistoryRow {
    pub agent_id: String,
    pub readiness: String,
    pub issue_codes: Vec<String>,
    pub evaluated_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub agent_id: String,
    pub recipe_id: String,
    pub version: Option<String>,
    pub installation_id: Option<String>,
    pub operation_id: String,
    pub status: String,
    pub at: String,
}

impl Store {
    pub fn open(path: &Path) -> NexusResult<Store> {
        let conn = Connection::open(path).map_err(|_| NexusError::new("store.open_failed"))?;
        Self::init(conn)
    }
    #[cfg(test)]
    pub fn open_in_memory() -> NexusResult<Store> {
        Self::init(Connection::open_in_memory().map_err(db_err)?)
    }
    fn init(mut conn: Connection) -> NexusResult<Store> {
        conn.pragma_update(None, "journal_mode", "WAL").map_err(db_err)?;
        conn.pragma_update(None, "foreign_keys", "ON").map_err(db_err)?;
        conn.pragma_update(None, "synchronous", "FULL").map_err(db_err)?;
        let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(db_err)?;
        if current > SCHEMA_VERSION {
            // Written by a newer Nexus: never downgrade or write into an unknown schema.
            return Err(NexusError::new("store.newer_schema").with("found", current.to_string()));
        }
        for (version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
            let tx = conn.transaction().map_err(db_err)?;
            tx.execute_batch(sql).map_err(|_| NexusError::new("store.migration_failed").with("version", version.to_string()))?;
            tx.pragma_update(None, "user_version", version).map_err(db_err)?;
            tx.commit().map_err(db_err)?;
        }
        Ok(Store { conn })
    }

    // ---- Providers -------------------------------------------------------------------------

    pub fn providers(&self) -> NexusResult<Vec<ProviderProfile>> {
        let mut stmt = self.conn.prepare("SELECT id, display_name, kind, base_url, auth_mode, secret_ref, env_var, revision FROM providers ORDER BY created_at").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ProviderProfile {
                    id: r.get(0)?,
                    display_name: r.get(1)?,
                    kind: ProviderKind::parse(&r.get::<_, String>(2)?).unwrap_or(ProviderKind::OpenaiCompatible),
                    base_url: r.get(3)?,
                    auth_mode: match r.get::<_, String>(4)?.as_str() {
                        "nexusSecret" => AuthMode::NexusSecret,
                        "environment" => AuthMode::Environment,
                        _ => AuthMode::AgentLogin,
                    },
                    secret_ref: r.get(5)?,
                    secret_present: None,
                    env_var: r.get(6)?,
                    revision: r.get(7)?,
                    builtin: false,
                    connection: "notTested",
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn insert_provider(&self, p: &ProviderProfile, now: Instant) -> NexusResult<()> {
        let auth = match p.auth_mode { AuthMode::AgentLogin => "agentLogin", AuthMode::NexusSecret => "nexusSecret", AuthMode::Environment => "environment" };
        self.conn
            .execute(
                "INSERT INTO providers (id, display_name, kind, base_url, auth_mode, secret_ref, env_var, revision, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
                params![p.id, p.display_name, p.kind.as_str(), p.base_url, auth, p.secret_ref, p.env_var, p.revision, now.to_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Deleting a Provider still referenced by a Model is refused (Architecture §9).
    pub fn delete_provider(&self, id: &str) -> NexusResult<Option<String>> {
        let refs: i64 = self.conn.query_row("SELECT COUNT(*) FROM models WHERE provider_id = ?1", [id], |r| r.get(0)).map_err(db_err)?;
        if refs > 0 {
            return Err(NexusError::new("provider.in_use").with("models", refs.to_string()));
        }
        let secret: Option<Option<String>> = self.conn.query_row("SELECT secret_ref FROM providers WHERE id = ?1", [id], |r| r.get(0)).optional().map_err(db_err)?;
        let Some(secret) = secret else { return Err(NexusError::new("provider.not_found")) };
        self.conn.execute("DELETE FROM providers WHERE id = ?1", [id]).map_err(db_err)?;
        Ok(secret)
    }

    // ---- Models ----------------------------------------------------------------------------

    pub fn models(&self) -> NexusResult<Vec<ModelDefinition>> {
        let mut stmt = self.conn.prepare("SELECT id, provider_id, identifier, display_name, source, source_detail, capabilities_json FROM models ORDER BY created_at").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                let caps: String = r.get(6)?;
                let claims: Vec<(String, String, String)> = serde_json::from_str(&caps).unwrap_or_default();
                Ok(ModelDefinition {
                    id: r.get(0)?,
                    provider_id: r.get(1)?,
                    identifier: r.get(2)?,
                    display_name: r.get(3)?,
                    source: r.get(4)?,
                    source_detail: r.get(5)?,
                    capabilities: claims.into_iter().map(|(key, state, source)| CapabilityClaim { key, state, source }).collect(),
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn insert_model(&self, m: &ModelDefinition, now: Instant) -> NexusResult<()> {
        let caps: Vec<(String, String, String)> = m.capabilities.iter().map(|c| (c.key.clone(), c.state.clone(), c.source.clone())).collect();
        self.conn
            .execute(
                "INSERT INTO models (id, provider_id, identifier, display_name, source, source_detail, capabilities_json, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![m.id, m.provider_id, m.identifier, m.display_name, m.source, m.source_detail, serde_json::to_string(&caps).unwrap_or_default(), now.to_rfc3339()],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => NexusError::new("model.duplicate"),
                other => db_err(other),
            })?;
        Ok(())
    }

    pub fn delete_model(&self, id: &str) -> NexusResult<()> {
        let refs: i64 = self
            .conn
            .query_row("SELECT (SELECT COUNT(*) FROM model_profiles WHERE model_id = ?1) + (SELECT COUNT(*) FROM assignments WHERE model_id = ?1)", [id], |r| r.get(0))
            .map_err(db_err)?;
        if refs > 0 {
            return Err(NexusError::new("model.in_use"));
        }
        if self.conn.execute("DELETE FROM models WHERE id = ?1", [id]).map_err(db_err)? == 0 {
            return Err(NexusError::new("model.not_found"));
        }
        Ok(())
    }

    // ---- Profiles (revisioned) -------------------------------------------------------------

    pub fn profiles(&self) -> NexusResult<Vec<ModelProfile>> {
        let mut stmt = self
            .conn
            .prepare("SELECT p.id, p.name, p.revision, p.model_id, p.reasoning FROM model_profiles p JOIN (SELECT id, MAX(revision) r FROM model_profiles GROUP BY id) l ON p.id = l.id AND p.revision = l.r ORDER BY p.created_at")
            .map_err(db_err)?;
        let rows = stmt.query_map([], |r| Ok(ModelProfile { id: r.get(0)?, name: r.get(1)?, revision: r.get(2)?, model_id: r.get(3)?, reasoning: r.get(4)? })).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    #[cfg(test)]
    pub fn profile_revision(&self, id: &str, revision: i64) -> NexusResult<Option<ModelProfile>> {
        self.conn
            .query_row("SELECT id, name, revision, model_id, reasoning FROM model_profiles WHERE id = ?1 AND revision = ?2", params![id, revision], |r| {
                Ok(ModelProfile { id: r.get(0)?, name: r.get(1)?, revision: r.get(2)?, model_id: r.get(3)?, reasoning: r.get(4)? })
            })
            .optional()
            .map_err(db_err)
    }

    /// Editing a Profile creates a new revision; applied revisions stay pinned (Architecture §9).
    pub fn save_profile(&self, id: &str, name: &str, model_id: &str, reasoning: Option<&str>, now: Instant) -> NexusResult<i64> {
        let next: i64 = self.conn.query_row("SELECT COALESCE(MAX(revision), 0) + 1 FROM model_profiles WHERE id = ?1", [id], |r| r.get(0)).map_err(db_err)?;
        self.conn
            .execute("INSERT INTO model_profiles (id, revision, name, model_id, reasoning, created_at) VALUES (?1,?2,?3,?4,?5,?6)", params![id, next, name, model_id, reasoning, now.to_rfc3339()])
            .map_err(db_err)?;
        Ok(next)
    }

    // ---- Assignments -----------------------------------------------------------------------

    pub fn assignments(&self) -> NexusResult<Vec<AgentModelAssignment>> {
        let mut stmt = self
            .conn
            .prepare("SELECT agent_id, mode, model_id, profile_id, desired_revision, applied_identifier, applied_profile_revision, applied_provider_revision, applied_at, applied_location, applied_fingerprint, updated_at FROM assignments")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                let applied_identifier: Option<String> = r.get(5)?;
                Ok((r.get::<_, String>(0)?, AgentModelAssignment {
                    agent_id: AgentId::Codex,
                    mode: match r.get::<_, String>(1)?.as_str() { "direct" => AssignmentMode::Direct, "profile" => AssignmentMode::Profile, _ => AssignmentMode::Native },
                    model_id: r.get(2)?,
                    profile_id: r.get(3)?,
                    desired_revision: r.get(4)?,
                    applied: match applied_identifier {
                        Some(identifier) => Some(AppliedRecord {
                            identifier,
                            profile_revision: r.get(6)?,
                            provider_revision: r.get(7)?,
                            applied_at: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
                            location: r.get::<_, Option<String>>(9)?.unwrap_or_default(),
                            fingerprint: r.get::<_, Option<String>>(10)?.unwrap_or_default(),
                        }),
                        None => None,
                    },
                    updated_at: r.get(11)?,
                }))
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (agent, mut a) = row.map_err(db_err)?;
            if let Some(id) = AgentId::parse(&agent) {
                a.agent_id = id;
                out.push(a);
            }
        }
        Ok(out)
    }

    pub fn assignment(&self, agent: AgentId) -> NexusResult<Option<AgentModelAssignment>> {
        Ok(self.assignments()?.into_iter().find(|a| a.agent_id == agent))
    }

    /// Records desired intent only; the applied record is untouched until a verified apply.
    pub fn set_desired(&self, agent: AgentId, mode: AssignmentMode, model_id: Option<&str>, profile_id: Option<&str>, desired_revision: Option<i64>, now: Instant) -> NexusResult<()> {
        let mode_s = match mode { AssignmentMode::Native => "native", AssignmentMode::Direct => "direct", AssignmentMode::Profile => "profile" };
        self.conn
            .execute(
                "INSERT INTO assignments (agent_id, mode, model_id, profile_id, desired_revision, updated_at) VALUES (?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(agent_id) DO UPDATE SET mode=excluded.mode, model_id=excluded.model_id, profile_id=excluded.profile_id, desired_revision=excluded.desired_revision, updated_at=excluded.updated_at",
                params![agent.as_str(), mode_s, model_id, profile_id, desired_revision, now.to_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_applied(&self, agent: AgentId, rec: &AppliedRecord) -> NexusResult<()> {
        let n = self
            .conn
            .execute(
                "UPDATE assignments SET applied_identifier=?2, applied_profile_revision=?3, applied_provider_revision=?4, applied_at=?5, applied_location=?6, applied_fingerprint=?7 WHERE agent_id=?1",
                params![agent.as_str(), rec.identifier, rec.profile_revision, rec.provider_revision, rec.applied_at, rec.location, rec.fingerprint],
            )
            .map_err(db_err)?;
        if n == 0 {
            return Err(NexusError::new("assignment.not_found"));
        }
        Ok(())
    }

    pub fn clear_assignment(&self, agent: AgentId) -> NexusResult<()> {
        self.conn.execute("DELETE FROM assignments WHERE agent_id = ?1", [agent.as_str()]).map_err(db_err)?;
        Ok(())
    }

    // ---- Operation journal -----------------------------------------------------------------

    pub fn begin_operation(&self, id: &str, action: &str, agent: Option<AgentId>, digest: &str, summary: &serde_json::Value, now: Instant) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT INTO operations (id, action, agent_id, status, plan_digest, summary_json, started_at) VALUES (?1,?2,?3,'executing',?4,?5,?6)",
                params![id, action, agent.map(|a| a.as_str()), digest, summary.to_string(), now.to_rfc3339()],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => NexusError::new("operation.replayed"),
                other => db_err(other),
            })?;
        Ok(())
    }

    /// Journals a step *before* the next side effect so a crash leaves an exact record.
    pub fn step(&self, op: &str, step: &str, now: Instant) -> NexusResult<()> {
        self.conn
            .execute("INSERT INTO operation_steps (operation_id, seq, step, at) VALUES (?1, (SELECT COALESCE(MAX(seq),0)+1 FROM operation_steps WHERE operation_id=?1), ?2, ?3)", params![op, step, now.to_rfc3339()])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn finish_operation(&self, op: &str, status: &str, error: Option<&str>, now: Instant) -> NexusResult<()> {
        self.conn.execute("UPDATE operations SET status=?2, error_code=?3, finished_at=?4 WHERE id=?1", params![op, status, error, now.to_rfc3339()]).map_err(db_err)?;
        Ok(())
    }

    pub fn operations(&self, limit: i64) -> NexusResult<Vec<OperationRecord>> {
        let mut stmt = self.conn.prepare("SELECT id, action, agent_id, status, started_at, finished_at, error_code, summary_json FROM operations ORDER BY started_at DESC, rowid DESC LIMIT ?1").map_err(db_err)?;
        let rows = stmt
            .query_map([limit], |r| {
                Ok(OperationRecord {
                    id: r.get(0)?,
                    action: r.get(1)?,
                    agent_id: r.get(2)?,
                    status: r.get(3)?,
                    started_at: r.get(4)?,
                    finished_at: r.get(5)?,
                    error_code: r.get(6)?,
                    summary: serde_json::from_str(&r.get::<_, String>(7)?).unwrap_or_default(),
                    steps: Vec::new(),
                })
            })
            .map_err(db_err)?;
        let mut ops: Vec<OperationRecord> = rows.collect::<Result<_, _>>().map_err(db_err)?;
        for op in ops.iter_mut() {
            let mut s = self.conn.prepare("SELECT step FROM operation_steps WHERE operation_id = ?1 ORDER BY seq").map_err(db_err)?;
            op.steps = s.query_map([&op.id], |r| r.get(0)).map_err(db_err)?.collect::<Result<_, _>>().map_err(db_err)?;
        }
        Ok(ops)
    }

    /// On startup, any operation still `executing` was interrupted (crash, kill, power loss).
    /// It is marked `interrupted`, never replayed automatically.
    pub fn reconcile_interrupted(&self, now: Instant) -> NexusResult<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT id FROM operations WHERE status = 'executing'").map_err(db_err)?;
        let ids: Vec<String> = stmt.query_map([], |r| r.get(0)).map_err(db_err)?.collect::<Result<_, _>>().map_err(db_err)?;
        for id in &ids {
            self.finish_operation(id, "interrupted", Some("operation.interrupted"), now)?;
        }
        Ok(ids)
    }

    // ---- Backups ---------------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_backup(&self, id: &str, op: &str, agent: Option<AgentId>, target_id: &str, target_path: &str, display: &str, sha: &str, size: usize, now: Instant) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT INTO backups (id, operation_id, agent_id, target_id, target_path, target_display, sha256, size, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![id, op, agent.map(|a| a.as_str()), target_id, target_path, display, sha, size as i64, now.to_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_backup_written(&self, id: &str, fingerprint: &str) -> NexusResult<()> {
        self.conn.execute("UPDATE backups SET written_fingerprint = ?2 WHERE id = ?1", params![id, fingerprint]).map_err(db_err)?;
        Ok(())
    }

    pub fn backups(&self, agent: Option<AgentId>) -> NexusResult<Vec<BackupRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, operation_id, agent_id, target_id, target_display, sha256, size, created_at, target_path, written_fingerprint FROM backups WHERE (?1 IS NULL OR agent_id = ?1) ORDER BY created_at DESC, rowid DESC LIMIT 50")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([agent.map(|a| a.as_str())], |r| {
                Ok(BackupRecord {
                    id: r.get(0)?,
                    operation_id: r.get(1)?,
                    agent_id: r.get(2)?,
                    target_id: r.get(3)?,
                    target_display: r.get(4)?,
                    sha256: r.get(5)?,
                    size: r.get(6)?,
                    created_at: r.get(7)?,
                    target_path: r.get(8)?,
                    written_fingerprint: r.get(9)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn backup(&self, id: &str) -> NexusResult<BackupRecord> {
        self.backups(None)?.into_iter().find(|b| b.id == id).ok_or_else(|| NexusError::new("backup.missing"))
    }

    // ---- Health history --------------------------------------------------------------------

    pub fn record_health(&self, agent: AgentId, readiness: &str, rule_set: &str, codes: &[String], at: Instant) -> NexusResult<()> {
        let tx = self.conn.unchecked_transaction().map_err(db_err)?;
        tx.execute("INSERT INTO health_history (agent_id, readiness, rule_set, issue_codes, evaluated_at) VALUES (?1,?2,?3,?4,?5)", params![agent.as_str(), readiness, rule_set, codes.join(","), at.to_rfc3339()])
            .map_err(db_err)?;
        // Bounded local history (Security §6 retention): keep the latest 200 rows per Agent.
        tx.execute("DELETE FROM health_history WHERE agent_id = ?1 AND id NOT IN (SELECT id FROM health_history WHERE agent_id = ?1 ORDER BY id DESC LIMIT 200)", [agent.as_str()]).map_err(db_err)?;
        tx.commit().map_err(db_err)
    }

    pub fn health_history(&self, agent: AgentId, limit: i64) -> NexusResult<Vec<HealthHistoryRow>> {
        let mut stmt = self.conn.prepare("SELECT agent_id, readiness, issue_codes, evaluated_at FROM health_history WHERE agent_id = ?1 ORDER BY id DESC LIMIT ?2").map_err(db_err)?;
        let rows = stmt
            .query_map(params![agent.as_str(), limit], |r| {
                let codes: String = r.get(2)?;
                Ok(HealthHistoryRow { agent_id: r.get(0)?, readiness: r.get(1)?, issue_codes: codes.split(',').filter(|s| !s.is_empty()).map(String::from).collect(), evaluated_at: r.get(3)? })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    // ---- Nexus-managed extensions and installations ---------------------------------------

    pub fn add_managed_extension(&self, agent: AgentId, name: &str, origin: Option<&str>, op: &str, now: Instant) -> NexusResult<()> {
        self.conn
            .execute("INSERT OR REPLACE INTO managed_extensions (agent_id, name, origin, created_at, operation_id) VALUES (?1,?2,?3,?4,?5)", params![agent.as_str(), name, origin, now.to_rfc3339(), op])
            .map_err(db_err)?;
        Ok(())
    }
    pub fn remove_managed_extension(&self, agent: AgentId, name: &str) -> NexusResult<()> {
        self.conn.execute("DELETE FROM managed_extensions WHERE agent_id = ?1 AND name = ?2", params![agent.as_str(), name]).map_err(db_err)?;
        Ok(())
    }
    pub fn managed_extensions(&self, agent: AgentId) -> NexusResult<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM managed_extensions WHERE agent_id = ?1").map_err(db_err)?;
        let rows = stmt.query_map([agent.as_str()], |r| r.get(0)).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn record_installation(&self, e: &LedgerEntry) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO nexus_installations (agent_id, recipe_id, version, installation_id, operation_id, status, at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![e.agent_id, e.recipe_id, e.version, e.installation_id, e.operation_id, e.status, e.at],
            )
            .map_err(db_err)?;
        Ok(())
    }
    pub fn remove_installation(&self, agent: AgentId, recipe: &str) -> NexusResult<()> {
        self.conn.execute("DELETE FROM nexus_installations WHERE agent_id = ?1 AND recipe_id = ?2", params![agent.as_str(), recipe]).map_err(db_err)?;
        Ok(())
    }
    pub fn ledger(&self) -> NexusResult<Vec<LedgerEntry>> {
        let mut stmt = self.conn.prepare("SELECT agent_id, recipe_id, version, installation_id, operation_id, status, at FROM nexus_installations").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| Ok(LedgerEntry { agent_id: r.get(0)?, recipe_id: r.get(1)?, version: r.get(2)?, installation_id: r.get(3)?, operation_id: r.get(4)?, status: r.get(5)?, at: r.get(6)? }))
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_once_and_newer_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nexus.db");
        Store::open(&path).unwrap();
        Store::open(&path).unwrap(); // idempotent
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);
        assert_eq!(Store::open(&path).err().unwrap().code, "store.newer_schema");
    }

    #[test]
    fn profile_edits_create_revisions_and_references_block_deletes() {
        let s = Store::open_in_memory().unwrap();
        let now = Instant(1000);
        let m = ModelDefinition { id: "m1".into(), provider_id: "builtin.openai".into(), identifier: "gpt-x".into(), display_name: None, source: "manual".into(), source_detail: None, capabilities: vec![] };
        s.insert_model(&m, now).unwrap();
        assert_eq!(s.insert_model(&m, now).unwrap_err().code, "model.duplicate");
        assert_eq!(s.save_profile("p", "Fast", "m1", None, now).unwrap(), 1);
        assert_eq!(s.save_profile("p", "Fast", "m1", Some("high"), now).unwrap(), 2);
        assert_eq!(s.profiles().unwrap()[0].revision, 2);
        assert!(s.profile_revision("p", 1).unwrap().is_some());
        assert_eq!(s.delete_model("m1").unwrap_err().code, "model.in_use");
    }

    #[test]
    fn interrupted_operations_are_reconciled_not_replayed() {
        let s = Store::open_in_memory().unwrap();
        s.begin_operation("op_1", "lifecycle.install", Some(AgentId::Gemini), "d", &serde_json::json!({}), Instant(1)).unwrap();
        s.step("op_1", "started", Instant(1)).unwrap();
        assert_eq!(s.begin_operation("op_1", "x", None, "d", &serde_json::json!({}), Instant(1)).unwrap_err().code, "operation.replayed");
        assert_eq!(s.reconcile_interrupted(Instant(2)).unwrap(), vec!["op_1".to_string()]);
        let ops = s.operations(10).unwrap();
        assert_eq!(ops[0].status, "interrupted");
        assert_eq!(ops[0].steps, vec!["started".to_string()]);
    }

    #[test]
    fn applied_record_is_separate_from_desired_intent() {
        let s = Store::open_in_memory().unwrap();
        s.set_desired(AgentId::Claude, AssignmentMode::Direct, Some("m"), None, None, Instant(1)).unwrap();
        assert!(s.assignment(AgentId::Claude).unwrap().unwrap().applied.is_none());
        s.set_applied(AgentId::Claude, &AppliedRecord { identifier: "opus".into(), profile_revision: None, provider_revision: Some(1), applied_at: "t".into(), location: "~/x".into(), fingerprint: "f".into() }).unwrap();
        s.set_desired(AgentId::Claude, AssignmentMode::Direct, Some("m2"), None, None, Instant(2)).unwrap();
        let a = s.assignment(AgentId::Claude).unwrap().unwrap();
        assert_eq!(a.model_id.as_deref(), Some("m2"));
        assert_eq!(a.applied.unwrap().identifier, "opus");
    }

    #[test]
    fn schema_has_no_secret_columns() {
        for (_, sql) in MIGRATIONS {
            let lower = sql.to_lowercase();
            assert!(!lower.contains("api_key") && !lower.contains("token") && !lower.contains("password") && !lower.contains("secret_value"));
        }
    }
}
