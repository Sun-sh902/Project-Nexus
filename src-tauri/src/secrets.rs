//! Nexus-managed secrets live only in the OS credential store (macOS Keychain, Windows Credential
//! Manager). There is no plaintext fallback: an unavailable store is a visible, recoverable error.
//! Values never cross IPC; the frontend sees references and presence only.

use crate::core::{NexusError, NexusResult};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;

pub const SERVICE: &str = "dev.projectnexus.desktop";

pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> NexusResult<Option<Vec<u8>>>;
    fn set(&self, account: &str, secret: &[u8]) -> NexusResult<()>;
    fn delete(&self, account: &str) -> NexusResult<()>;
}

pub struct OsSecretStore;

fn map_err(e: keyring::Error) -> NexusError {
    match e {
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => NexusError::new("credential.store_unavailable").retryable(),
        keyring::Error::NoEntry => NexusError::new("credential.not_found"),
        _ => NexusError::new("credential.store_error"),
    }
}

fn validate_account(account: &str) -> NexusResult<()> {
    if account.is_empty() || account.len() > 128 || !account.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')) {
        return Err(NexusError::new("credential.invalid_reference"));
    }
    Ok(())
}

impl SecretStore for OsSecretStore {
    fn get(&self, account: &str) -> NexusResult<Option<Vec<u8>>> {
        validate_account(account)?;
        let entry = keyring::Entry::new(SERVICE, account).map_err(map_err)?;
        match entry.get_secret() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(map_err(e)),
        }
    }
    fn set(&self, account: &str, secret: &[u8]) -> NexusResult<()> {
        validate_account(account)?;
        keyring::Entry::new(SERVICE, account).map_err(map_err)?.set_secret(secret).map_err(map_err)
    }
    fn delete(&self, account: &str) -> NexusResult<()> {
        validate_account(account)?;
        match keyring::Entry::new(SERVICE, account).map_err(map_err)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(map_err(e)),
        }
    }
}

/// In-memory store for tests. `fail` simulates a locked/unavailable credential store.
#[cfg(test)]
#[derive(Default)]
pub struct MemorySecretStore {
    pub items: Mutex<HashMap<String, Vec<u8>>>,
    pub fail: Mutex<bool>,
}

#[cfg(test)]
impl SecretStore for MemorySecretStore {
    fn get(&self, account: &str) -> NexusResult<Option<Vec<u8>>> {
        validate_account(account)?;
        if *self.fail.lock().unwrap() {
            return Err(NexusError::new("credential.store_unavailable").retryable());
        }
        Ok(self.items.lock().unwrap().get(account).cloned())
    }
    fn set(&self, account: &str, secret: &[u8]) -> NexusResult<()> {
        validate_account(account)?;
        if *self.fail.lock().unwrap() {
            return Err(NexusError::new("credential.store_unavailable").retryable());
        }
        self.items.lock().unwrap().insert(account.to_string(), secret.to_vec());
        Ok(())
    }
    fn delete(&self, account: &str) -> NexusResult<()> {
        validate_account(account)?;
        self.items.lock().unwrap().remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_are_validated() {
        let store = MemorySecretStore::default();
        assert_eq!(store.set("../evil", b"x").unwrap_err().code, "credential.invalid_reference");
        assert_eq!(store.set("", b"x").unwrap_err().code, "credential.invalid_reference");
        store.set("provider:abc", b"k").unwrap();
        assert_eq!(store.get("provider:abc").unwrap().as_deref(), Some(&b"k"[..]));
    }

    #[test]
    fn locked_store_has_no_fallback() {
        let store = MemorySecretStore::default();
        *store.fail.lock().unwrap() = true;
        assert_eq!(store.set("provider:abc", b"k").unwrap_err().code, "credential.store_unavailable");
        assert!(store.items.lock().unwrap().is_empty());
    }
}
