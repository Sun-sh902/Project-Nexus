//! Bounded filesystem reads and the single verified write primitive used by configuration
//! mutation. No other module writes outside the Nexus data directory.

use crate::core::{new_id, sha256_hex, NexusError, NexusResult};
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadFailure {
    NotFound,
    AccessDenied,
    TooLarge,
    NotRegularFile,
    Io,
}

impl ReadFailure {
    pub fn code(self) -> &'static str {
        match self {
            ReadFailure::NotFound => "fs.not_found",
            ReadFailure::AccessDenied => "fs.access_denied",
            ReadFailure::TooLarge => "fs.too_large",
            ReadFailure::NotRegularFile => "fs.not_regular_file",
            ReadFailure::Io => "fs.io_error",
        }
    }
}

fn classify(err: &std::io::Error) -> ReadFailure {
    match err.kind() {
        std::io::ErrorKind::NotFound => ReadFailure::NotFound,
        std::io::ErrorKind::PermissionDenied => ReadFailure::AccessDenied,
        _ => ReadFailure::Io,
    }
}

#[derive(Clone, Debug)]
pub struct FileRead {
    pub bytes: Vec<u8>,
    pub fingerprint: String,
    /// The path itself is a symbolic link (the link was followed for reading only).
    pub via_symlink: bool,
}

/// Reads a regular file with a size bound. Follows symlinks for reading but reports them.
pub fn read_bounded(path: &Path, max: u64) -> Result<FileRead, ReadFailure> {
    let link_meta = fs::symlink_metadata(path).map_err(|e| classify(&e))?;
    let via_symlink = link_meta.file_type().is_symlink();
    let meta = fs::metadata(path).map_err(|e| classify(&e))?;
    if !meta.is_file() {
        return Err(ReadFailure::NotRegularFile);
    }
    if meta.len() > max {
        return Err(ReadFailure::TooLarge);
    }
    let mut file = File::open(path).map_err(|e| classify(&e))?;
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    (&mut file).take(max + 1).read_to_end(&mut bytes).map_err(|e| classify(&e))?;
    if bytes.len() as u64 > max {
        return Err(ReadFailure::TooLarge);
    }
    Ok(FileRead { fingerprint: sha256_hex(&bytes), bytes, via_symlink })
}

/// A path is safe to mutate only if it is a regular, non-symlinked file (or absent with a real,
/// non-symlinked parent directory), owned by the current user, and not group/world writable.
pub fn check_mutable_target(path: &Path) -> NexusResult<bool> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(NexusError::new("config.unsafe_path"));
    }
    let parent = path.parent().ok_or_else(|| NexusError::new("config.unsafe_path"))?;
    let parent_meta = fs::symlink_metadata(parent).map_err(|_| NexusError::new("config.parent_missing"))?;
    if parent_meta.file_type().is_symlink() || !parent_meta.is_dir() {
        return Err(NexusError::new("config.unsafe_symlink"));
    }
    check_owner(&parent_meta)?;
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(NexusError::new("config.unsafe_symlink"));
            }
            if !meta.is_file() {
                return Err(NexusError::new("config.not_regular_file"));
            }
            check_owner(&meta)?;
            check_hard_links(&meta)?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(NexusError::new("fs.access_denied")),
    }
}

#[cfg(unix)]
fn check_owner(meta: &fs::Metadata) -> NexusResult<()> {
    use std::os::unix::fs::MetadataExt;
    let uid = unsafe { libc::getuid() };
    if meta.uid() != uid {
        return Err(NexusError::new("config.unexpected_owner"));
    }
    if meta.mode() & 0o022 != 0 && !meta.is_dir() {
        return Err(NexusError::new("config.unsafe_permissions"));
    }
    Ok(())
}
#[cfg(not(unix))]
fn check_owner(_meta: &fs::Metadata) -> NexusResult<()> {
    // Windows ACL/ownership validation is not yet implemented; mutation remains allowed only for
    // paths under the user profile, which callers enforce. Recorded as outstanding in the progress doc.
    Ok(())
}

#[cfg(unix)]
fn check_hard_links(meta: &fs::Metadata) -> NexusResult<()> {
    use std::os::unix::fs::MetadataExt;
    if meta.nlink() > 1 {
        return Err(NexusError::new("config.unsafe_hard_link"));
    }
    Ok(())
}
#[cfg(not(unix))]
fn check_hard_links(_meta: &fs::Metadata) -> NexusResult<()> {
    Ok(())
}

/// Atomic replacement: securely created sibling temporary file → write → fsync → rename →
/// directory fsync. Preserves the original file's permission bits. Fails closed if the target's
/// current fingerprint no longer matches `expected_fingerprint` (concurrent external edit).
pub fn atomic_replace(path: &Path, bytes: &[u8], expected_fingerprint: Option<&str>) -> NexusResult<String> {
    let existed = check_mutable_target(path)?;
    let current = if existed {
        Some(read_bounded(path, MAX_CONFIG_BYTES).map_err(|f| NexusError::new(f.code()))?.fingerprint)
    } else {
        None
    };
    if current.as_deref() != expected_fingerprint {
        return Err(NexusError::new("config.concurrent_change"));
    }
    let parent = path.parent().ok_or_else(|| NexusError::new("config.unsafe_path"))?;
    let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| NexusError::new("config.unsafe_path"))?;
    let tmp: PathBuf = parent.join(format!(".{name}.{}.nexus-tmp", new_id("w")));
    let result = (|| -> NexusResult<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp).map_err(|_| NexusError::new("fs.write_failed"))?;
        file.write_all(bytes).map_err(|_| NexusError::new("fs.write_failed").retryable())?;
        file.sync_all().map_err(|_| NexusError::new("fs.write_failed").retryable())?;
        #[cfg(unix)]
        if existed {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(path).map_err(|_| NexusError::new("fs.write_failed"))?.permissions().mode();
            fs::set_permissions(&tmp, fs::Permissions::from_mode(mode & 0o7777)).map_err(|_| NexusError::new("fs.write_failed"))?;
        }
        drop(file);
        // Re-check immediately before replacement to narrow the race window.
        if existed {
            let again = read_bounded(path, MAX_CONFIG_BYTES).map_err(|f| NexusError::new(f.code()))?.fingerprint;
            if Some(again.as_str()) != expected_fingerprint {
                return Err(NexusError::new("config.concurrent_change"));
            }
        }
        if existed {
            fs::rename(&tmp, path).map_err(|_| NexusError::new("fs.write_failed"))?;
        } else {
            // Creation must never replace a file that appeared after planning: a hard link
            // fails if the target exists, unlike rename.
            fs::hard_link(&tmp, path).map_err(|e| if e.kind() == std::io::ErrorKind::AlreadyExists { NexusError::new("config.concurrent_change") } else { NexusError::new("fs.write_failed") })?;
            let _ = fs::remove_file(&tmp);
        }
        #[cfg(unix)]
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    Ok(sha256_hex(bytes))
}

/// Writes a file inside a Nexus-owned directory (backups, database sidecars) with 0600 perms.
pub fn write_private(path: &Path, bytes: &[u8]) -> NexusResult<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| NexusError::new("fs.write_failed"))?;
    file.write_all(bytes).map_err(|_| NexusError::new("fs.write_failed"))?;
    file.sync_all().map_err(|_| NexusError::new("fs.write_failed"))?;
    Ok(())
}

pub fn ensure_private_dir(path: &Path) -> NexusResult<()> {
    fs::create_dir_all(path).map_err(|_| NexusError::new("fs.write_failed"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| NexusError::new("fs.write_failed"))?;
    }
    Ok(())
}

/// True for a regular file with an executable bit (Unix) or `.exe` (Windows).
pub fn is_executable_file(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                path.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false)
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_replace_preserves_mode_and_detects_concurrent_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, b"{}").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        }
        let fp = read_bounded(&path, MAX_CONFIG_BYTES).unwrap().fingerprint;
        atomic_replace(&path, b"{\"a\":1}", Some(&fp)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{\"a\":1}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o640);
        }
        // Stale fingerprint → refused, file untouched.
        let err = atomic_replace(&path, b"{}", Some(&fp)).unwrap_err();
        assert_eq!(err.code, "config.concurrent_change");
        assert_eq!(fs::read(&path).unwrap(), b"{\"a\":1}");
        // No temp files left behind.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_targets_and_parents_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.json");
        fs::write(&real, b"{}").unwrap();
        let link = dir.path().join("link.json");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(check_mutable_target(&link).unwrap_err().code, "config.unsafe_symlink");
        let real_dir = dir.path().join("realdir");
        fs::create_dir(&real_dir).unwrap();
        let link_dir = dir.path().join("linkdir");
        std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();
        assert_eq!(check_mutable_target(&link_dir.join("x.json")).unwrap_err().code, "config.unsafe_symlink");
    }

    #[cfg(unix)]
    #[test]
    fn hard_linked_and_world_writable_targets_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.json");
        fs::write(&a, b"{}").unwrap();
        fs::hard_link(&a, dir.path().join("b.json")).unwrap();
        assert_eq!(check_mutable_target(&a).unwrap_err().code, "config.unsafe_hard_link");
        let c = dir.path().join("c.json");
        fs::write(&c, b"{}").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&c, fs::Permissions::from_mode(0o666)).unwrap();
        assert_eq!(check_mutable_target(&c).unwrap_err().code, "config.unsafe_permissions");
    }

    #[test]
    fn creation_never_replaces_a_file_that_appeared_meanwhile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.json");
        atomic_replace(&path, b"{}", None).unwrap();
        assert_eq!(atomic_replace(&path, b"{\"x\":1}", None).unwrap_err().code, "config.concurrent_change");
        assert_eq!(fs::read(&path).unwrap(), b"{}");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn traversal_paths_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a").join("..").join("x.json");
        assert_eq!(check_mutable_target(&p).unwrap_err().code, "config.unsafe_path");
        assert_eq!(check_mutable_target(Path::new("relative.json")).unwrap_err().code, "config.unsafe_path");
    }

    #[test]
    fn bounded_read_rejects_oversize() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big");
        fs::write(&p, vec![b'a'; 100]).unwrap();
        assert_eq!(read_bounded(&p, 10).unwrap_err(), ReadFailure::TooLarge);
        assert_eq!(read_bounded(&dir.path().join("missing"), 10).unwrap_err(), ReadFailure::NotFound);
    }
}
