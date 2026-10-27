//! Opt-in check of the real OS credential store (macOS Keychain / Windows Credential Manager):
//! `cargo test keychain_selftest -- --ignored`. Uses a dedicated self-test service name and
//! deletes its item afterwards; it never touches Agent credentials or Nexus production entries.

use crate::backup::BackupVault;
use crate::core::{NexusError, NexusResult};
use crate::secrets::SecretStore;
use std::sync::Arc;

struct SelfTestStore;
const SERVICE: &str = "dev.projectnexus.desktop.selftest";

impl SecretStore for SelfTestStore {
    fn get(&self, account: &str) -> NexusResult<Option<Vec<u8>>> {
        match keyring::Entry::new(SERVICE, account)
            .map_err(|_| NexusError::new("credential.store_error"))?
            .get_secret()
        {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(NexusError::new("credential.store_unavailable")),
        }
    }
    fn set(&self, account: &str, secret: &[u8]) -> NexusResult<()> {
        keyring::Entry::new(SERVICE, account)
            .map_err(|_| NexusError::new("credential.store_error"))?
            .set_secret(secret)
            .map_err(|_| NexusError::new("credential.store_unavailable"))
    }
    fn delete(&self, account: &str) -> NexusResult<()> {
        match keyring::Entry::new(SERVICE, account)
            .map_err(|_| NexusError::new("credential.store_error"))?
            .delete_credential()
        {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(NexusError::new("credential.store_error")),
        }
    }
}

#[test]
#[ignore]
fn keychain_selftest() {
    let store = Arc::new(SelfTestStore);
    let dir = tempfile::tempdir().unwrap();
    let vault = BackupVault::new(dir.path().join("backups"), store.clone());
    let payload = b"{\"model\": \"opus\", \"note\": \"keychain self-test\"}";
    let r = vault
        .protect(payload)
        .expect("real Keychain must hold the backup key");
    assert_eq!(vault.read(&r.id).unwrap(), payload);
    // A second vault instance reuses the persisted key from the OS store.
    let again = BackupVault::new(dir.path().join("backups"), store.clone());
    assert_eq!(again.read(&r.id).unwrap(), payload);
    store.delete("backup-encryption-key-v1").unwrap();
    assert!(store.get("backup-encryption-key-v1").unwrap().is_none());
    // Without the key the backup cannot be decrypted, and a new key does not unlock it.
    assert_eq!(again.read(&r.id).unwrap_err().code, "backup.corrupt");
    store.delete("backup-encryption-key-v1").unwrap();
}
