//! Protected exact-byte backups (Security §6). Backups may contain pre-existing Agent secrets, so
//! they are encrypted with AES-256-GCM using a key held in the OS credential store, written 0600
//! into the Nexus data directory, and verified by decryption before the first write proceeds.
//! If the key store is unavailable the mutation is blocked; there is no plaintext fallback.

use crate::core::{new_id, sha256_hex, NexusError, NexusResult};
use crate::secrets::SecretStore;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use std::path::PathBuf;
use std::sync::Arc;

const KEY_ACCOUNT: &str = "backup-encryption-key-v1";
const MAGIC: &[u8; 4] = b"NXB1";

#[derive(Clone, Debug)]
pub struct BackupRef {
    pub id: String,
    pub sha256: String,
    pub size: usize,
}

pub struct BackupVault {
    pub dir: PathBuf,
    secrets: Arc<dyn SecretStore>,
}

impl BackupVault {
    pub fn new(dir: PathBuf, secrets: Arc<dyn SecretStore>) -> Self {
        BackupVault { dir, secrets }
    }

    fn cipher(&self) -> NexusResult<Aes256Gcm> {
        let key = match self.secrets.get(KEY_ACCOUNT).map_err(|e| NexusError::new("backup.secure_store_unavailable").with("cause", e.code))? {
            Some(k) if k.len() == 32 => k,
            Some(_) => return Err(NexusError::new("backup.key_invalid")),
            None => {
                let mut k = vec![0u8; 32];
                getrandom::fill(&mut k).map_err(|_| NexusError::new("backup.key_invalid"))?;
                self.secrets.set(KEY_ACCOUNT, &k).map_err(|e| NexusError::new("backup.secure_store_unavailable").with("cause", e.code))?;
                k
            }
        };
        Aes256Gcm::new_from_slice(&key).map_err(|_| NexusError::new("backup.key_invalid"))
    }

    fn file(&self, id: &str) -> NexusResult<PathBuf> {
        if !id.starts_with("bak_") || id.len() != 36 || !id[4..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(NexusError::new("backup.invalid_reference"));
        }
        Ok(self.dir.join(format!("{id}.nxb")))
    }

    /// Encrypts, writes and verifies a backup. Returns only after the backup round-trips.
    pub fn protect(&self, bytes: &[u8]) -> NexusResult<BackupRef> {
        crate::platform::fs::ensure_private_dir(&self.dir).map_err(|_| NexusError::new("backup.write_failed"))?;
        let cipher = self.cipher()?;
        let id = new_id("bak");
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).map_err(|_| NexusError::new("backup.write_failed"))?;
        let sealed = cipher
            .encrypt(Nonce::from_slice(&nonce), Payload { msg: bytes, aad: id.as_bytes() })
            .map_err(|_| NexusError::new("backup.write_failed"))?;
        let mut out = Vec::with_capacity(4 + 12 + sealed.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&sealed);
        let path = self.file(&id)?;
        crate::platform::fs::write_private(&path, &out).map_err(|_| NexusError::new("backup.write_failed"))?;
        let reference = BackupRef { sha256: sha256_hex(bytes), size: bytes.len(), id };
        if self.read(&reference.id)? != bytes {
            let _ = std::fs::remove_file(&path);
            return Err(NexusError::new("backup.verification_failed"));
        }
        Ok(reference)
    }

    pub fn read(&self, id: &str) -> NexusResult<Vec<u8>> {
        let path = self.file(id)?;
        let data = std::fs::read(&path).map_err(|_| NexusError::new("backup.missing"))?;
        if data.len() < 16 || &data[..4] != MAGIC {
            return Err(NexusError::new("backup.corrupt"));
        }
        self.cipher()?
            .decrypt(Nonce::from_slice(&data[4..16]), Payload { msg: &data[16..], aad: id.as_bytes() })
            .map_err(|_| NexusError::new("backup.corrupt"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemorySecretStore;

    #[test]
    fn backups_are_encrypted_and_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let vault = BackupVault::new(dir.path().join("backups"), Arc::new(MemorySecretStore::default()));
        let secret_bearing = b"{\"apiKey\": \"sk-CANARY-should-not-appear-in-plaintext\"}";
        let r = vault.protect(secret_bearing).unwrap();
        assert_eq!(vault.read(&r.id).unwrap(), secret_bearing);
        let raw = std::fs::read(dir.path().join("backups").join(format!("{}.nxb", r.id))).unwrap();
        assert!(!raw.windows(6).any(|w| w == b"CANARY"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join("backups").join(format!("{}.nxb", r.id))).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn unavailable_key_store_blocks_backup() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(MemorySecretStore::default());
        *store.fail.lock().unwrap() = true;
        let vault = BackupVault::new(dir.path().to_path_buf(), store);
        assert_eq!(vault.protect(b"x").unwrap_err().code, "backup.secure_store_unavailable");
    }

    #[test]
    fn tampered_or_swapped_backups_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let vault = BackupVault::new(dir.path().to_path_buf(), Arc::new(MemorySecretStore::default()));
        let a = vault.protect(b"aaa").unwrap();
        let b = vault.protect(b"bbb").unwrap();
        let pa = dir.path().join(format!("{}.nxb", a.id));
        let pb = dir.path().join(format!("{}.nxb", b.id));
        std::fs::copy(&pb, &pa).unwrap(); // AAD binds ciphertext to its id
        assert_eq!(vault.read(&a.id).unwrap_err().code, "backup.corrupt");
        assert_eq!(vault.read("../../etc/passwd").unwrap_err().code, "backup.invalid_reference");
    }
}
