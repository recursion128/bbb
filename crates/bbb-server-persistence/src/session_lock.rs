//! Vanilla-compatible exclusive world-directory ownership.

use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};

use super::atomic_file::{io_error, open_file_matches_path};
use bbb_server_types::save::{SaveError, SaveResult, WorldSavePaths};

const VANILLA_LOCK_MARKER: &[u8] = "☃".as_bytes();

#[derive(Debug)]
pub struct WorldSessionLock {
    file: Option<File>,
    path: PathBuf,
}

impl WorldSessionLock {
    pub fn acquire(paths: &WorldSavePaths) -> SaveResult<Self> {
        fs::create_dir_all(paths.root())
            .map_err(|source| io_error("creating world directory", paths.root(), source))?;
        let path = paths.session_lock();
        reject_existing_lock_symlink(&path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let mut file = options
            .open(&path)
            .map_err(|source| io_error("opening world session lock", &path, source))?;
        if !open_file_matches_path(&file, &path) {
            return Err(SaveError::InvalidPath {
                path,
                detail: "world session lock changed or became a symbolic link while opening",
            });
        }
        file.try_lock().map_err(|source| SaveError::Io {
            operation: "acquiring world session lock",
            path: path.clone(),
            source: source.into(),
        })?;
        file.seek(SeekFrom::Start(0))
            .map_err(|source| io_error("seeking world session lock", &path, source))?;
        file.write_all(VANILLA_LOCK_MARKER)
            .map_err(|source| io_error("writing world session lock", &path, source))?;
        file.set_len(VANILLA_LOCK_MARKER.len() as u64)
            .map_err(|source| io_error("truncating world session lock", &path, source))?;
        file.sync_all()
            .map_err(|source| io_error("syncing world session lock", &path, source))?;
        Ok(Self {
            file: Some(file),
            path,
        })
    }

    pub fn is_valid(&self) -> bool {
        let Some(file) = &self.file else {
            return false;
        };
        open_file_matches_path(file, &self.path)
    }

    pub fn release(mut self) -> SaveResult<()> {
        let Some(file) = self.file.take() else {
            return Ok(());
        };
        file.unlock().map_err(|source| SaveError::Io {
            operation: "releasing world session lock",
            path: self.path.clone(),
            source,
        })
    }
}

fn reject_existing_lock_symlink(path: &std::path::Path) -> SaveResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(SaveError::InvalidPath {
            path: path.to_path_buf(),
            detail: "world session lock must not be a symbolic link",
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(
            "reading world session lock metadata",
            path,
            source,
        )),
    }
}

impl Drop for WorldSessionLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT_DIR: AtomicU64 = AtomicU64::new(1);

    fn paths() -> WorldSavePaths {
        WorldSavePaths::new(std::env::temp_dir().join(format!(
            "bbb-client-world-session-lock-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        )))
        .unwrap()
    }

    #[test]
    fn writes_vanilla_marker_and_excludes_a_second_owner() {
        let paths = paths();
        let lock = WorldSessionLock::acquire(&paths).unwrap();
        assert!(lock.is_valid());
        assert!(matches!(
            WorldSessionLock::acquire(&paths),
            Err(SaveError::Io {
                operation: "acquiring world session lock",
                ..
            })
        ));
        lock.release().unwrap();
        assert_eq!(fs::read(paths.session_lock()).unwrap(), VANILLA_LOCK_MARKER);
        let reacquired = WorldSessionLock::acquire(&paths).unwrap();
        assert!(reacquired.is_valid());
        reacquired.release().unwrap();
        fs::remove_dir_all(paths.root()).unwrap();
    }
}
