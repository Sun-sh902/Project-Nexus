//! Read → Parse → Validate → Backup → Minimal merge → Atomic write → Re-read → Validate →
//! Rollback where practical. The semantic post-condition (e.g. "the Agent now resolves model X")
//! is checked by the caller with a fresh adapter observation; this module guarantees the byte-level
//! steps and the compensation rules.

use super::ConfigEdit;
use crate::backup::{BackupRef, BackupVault};
use crate::core::{NexusError, NexusResult};
use crate::platform::fs::{atomic_replace, check_mutable_target, read_bounded, MAX_CONFIG_BYTES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Prechecked,
    BackedUp,
    Written,
    Reread,
}

#[derive(Clone, Debug)]
pub struct Applied {
    pub backup: Option<BackupRef>,
    pub written_fingerprint: String,
    /// The re-read after the write matched the planned bytes. When false the file *was*
    /// changed but is unverified, so the caller must treat it as a verification failure.
    pub reread_ok: bool,
}

/// Applies an edit. `on_step` is called after each completed step so the caller can journal it
/// before the next side effect.
pub fn apply(edit: &ConfigEdit, vault: &BackupVault, on_step: &mut dyn FnMut(Step, Option<&BackupRef>)) -> NexusResult<Applied> {
    let exists = check_mutable_target(&edit.path)?;
    if exists != edit.before_fingerprint.is_some() {
        return Err(NexusError::new("config.concurrent_change"));
    }
    let current = if exists { Some(read_bounded(&edit.path, MAX_CONFIG_BYTES).map_err(|f| NexusError::new(f.code()))?) } else { None };
    if current.as_ref().map(|c| c.fingerprint.as_str()) != edit.before_fingerprint.as_deref() {
        return Err(NexusError::new("config.concurrent_change"));
    }
    on_step(Step::Prechecked, None);
    let backup = match &current {
        Some(read) => {
            let r = vault.protect(&read.bytes)?;
            on_step(Step::BackedUp, Some(&r));
            Some(r)
        }
        None => None,
    };
    let written = atomic_replace(&edit.path, &edit.after, edit.before_fingerprint.as_deref())?;
    on_step(Step::Written, backup.as_ref());
    let reread_ok = read_bounded(&edit.path, MAX_CONFIG_BYTES).map(|r| r.fingerprint == edit.after_fingerprint).unwrap_or(false) && written == edit.after_fingerprint;
    if reread_ok {
        on_step(Step::Reread, backup.as_ref());
    }
    Ok(Applied { backup, written_fingerprint: written, reread_ok })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Compensation {
    RolledBack,
    /// The file changed after Nexus wrote it; restoring would erase someone else's edit.
    RefusedExternalChange,
    Failed(&'static str),
}

/// Automatic compensation: restore the exact pre-edit bytes only if the file still holds exactly
/// what Nexus wrote. Never applies when an external edit intervened (Architecture §8).
pub fn compensate(edit: &ConfigEdit, applied_fingerprint: &str, vault: &BackupVault, backup: Option<&BackupRef>) -> Compensation {
    let current = match read_bounded(&edit.path, MAX_CONFIG_BYTES) {
        Ok(r) => r.fingerprint,
        Err(_) => return Compensation::Failed("fs.io_error"),
    };
    if current != applied_fingerprint {
        return Compensation::RefusedExternalChange;
    }
    match backup {
        Some(b) => match vault.read(&b.id) {
            Ok(bytes) if crate::core::sha256_hex(&bytes) == b.sha256 => match atomic_replace(&edit.path, &bytes, Some(applied_fingerprint)) {
                Ok(_) => Compensation::RolledBack,
                Err(e) => Compensation::Failed(e.code),
            },
            _ => Compensation::Failed("backup.corrupt"),
        },
        // Nexus created the file: removing it restores the prior (absent) state.
        None => match std::fs::remove_file(&edit.path) {
            Ok(()) => Compensation::RolledBack,
            Err(_) => Compensation::Failed("fs.write_failed"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::plan_json_set;
    use crate::discovery::read_config;
    use crate::domain::{ConfigFormat, ConfigScope};
    use crate::platform::HostContext;
    use crate::secrets::MemorySecretStore;
    use std::sync::Arc;

    fn setup(text: Option<&str>) -> (tempfile::TempDir, std::path::PathBuf, BackupVault, HostContext) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        if let Some(t) = text {
            std::fs::write(&path, t).unwrap();
        }
        let vault = BackupVault::new(dir.path().join("vault"), Arc::new(MemorySecretStore::default()));
        let ctx = HostContext::isolated(dir.path(), &dir.path().join("data"));
        (dir, path, vault, ctx)
    }

    #[test]
    fn apply_backs_up_writes_and_rolls_back() {
        let original = "{\n  \"theme\": \"dark\"\n}\n";
        let (_d, path, vault, ctx) = setup(Some(original));
        let read = read_config(&ctx, "s", path.clone(), ConfigScope::User, ConfigFormat::Json, false);
        let edit = plan_json_set(&read, &["model"], serde_json::json!("opus")).unwrap();
        let mut steps = Vec::new();
        let applied = apply(&edit, &vault, &mut |s, _| steps.push(s)).unwrap();
        assert_eq!(steps, vec![Step::Prechecked, Step::BackedUp, Step::Written, Step::Reread]);
        assert_eq!(vault.read(&applied.backup.as_ref().unwrap().id).unwrap(), original.as_bytes());
        assert!(std::fs::read_to_string(&path).unwrap().contains("\"model\": \"opus\""));
        assert_eq!(compensate(&edit, &applied.written_fingerprint, &vault, applied.backup.as_ref()), Compensation::RolledBack);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn external_edit_between_plan_and_apply_is_refused() {
        let (_d, path, vault, ctx) = setup(Some("{}"));
        let read = read_config(&ctx, "s", path.clone(), ConfigScope::User, ConfigFormat::Json, false);
        let edit = plan_json_set(&read, &["model"], serde_json::json!("opus")).unwrap();
        std::fs::write(&path, "{\"theme\": \"light\"}").unwrap();
        assert_eq!(apply(&edit, &vault, &mut |_, _| {}).unwrap_err().code, "config.concurrent_change");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"theme\": \"light\"}");
    }

    #[test]
    fn compensation_never_erases_later_external_edits() {
        let (_d, path, vault, ctx) = setup(Some("{}"));
        let read = read_config(&ctx, "s", path.clone(), ConfigScope::User, ConfigFormat::Json, false);
        let edit = plan_json_set(&read, &["model"], serde_json::json!("opus")).unwrap();
        let applied = apply(&edit, &vault, &mut |_, _| {}).unwrap();
        std::fs::write(&path, "{\"model\": \"user-changed\"}").unwrap();
        assert_eq!(compensate(&edit, &applied.written_fingerprint, &vault, applied.backup.as_ref()), Compensation::RefusedExternalChange);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"model\": \"user-changed\"}");
    }

    #[test]
    fn backup_failure_stops_before_any_write() {
        let (dir, path, _vault, ctx) = setup(Some("{}"));
        let store = Arc::new(MemorySecretStore::default());
        *store.fail.lock().unwrap() = true;
        let vault = BackupVault::new(dir.path().join("vault"), store);
        let read = read_config(&ctx, "s", path.clone(), ConfigScope::User, ConfigFormat::Json, false);
        let edit = plan_json_set(&read, &["model"], serde_json::json!("opus")).unwrap();
        assert_eq!(apply(&edit, &vault, &mut |_, _| {}).unwrap_err().code, "backup.secure_store_unavailable");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}");
    }

    #[test]
    fn created_files_are_removed_on_compensation() {
        let (_d, path, vault, ctx) = setup(None);
        let read = read_config(&ctx, "s", path.clone(), ConfigScope::User, ConfigFormat::Json, false);
        let edit = plan_json_set(&read, &["model"], serde_json::json!("opus")).unwrap();
        let applied = apply(&edit, &vault, &mut |_, _| {}).unwrap();
        assert!(applied.backup.is_none() && path.exists());
        assert_eq!(compensate(&edit, &applied.written_fingerprint, &vault, None), Compensation::RolledBack);
        assert!(!path.exists());
    }
}
