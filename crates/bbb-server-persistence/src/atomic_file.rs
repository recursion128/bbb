use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use bbb_server_types::save::{SaveError, SaveResult};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

pub struct StagedFile {
    path: PathBuf,
    published: bool,
}

impl StagedFile {
    pub fn write(parent: &Path, stem: &str, bytes: &[u8]) -> SaveResult<Self> {
        fs::create_dir_all(parent)
            .map_err(|source| io_error("creating directory", parent, source))?;
        let mut last_error = None;
        for _ in 0..16 {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".bbb-{stem}-{}-{id}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    // Arm the drop guard immediately so write/sync errors remove the
                    // temporary file after the open handle closes.
                    let staged = Self {
                        path,
                        published: false,
                    };
                    let write_result = file
                        .write_all(bytes)
                        .map_err(|source| io_error("writing staged file", &staged.path, source))
                        .and_then(|()| {
                            file.sync_all().map_err(|source| {
                                io_error("syncing staged file", &staged.path, source)
                            })
                        });
                    drop(file);
                    write_result?;
                    return Ok(staged);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    last_error = Some(error);
                }
                Err(source) => return Err(io_error("creating staged file", &path, source)),
            }
        }
        Err(io_error(
            "creating unique staged file",
            parent,
            last_error.unwrap_or_else(|| std::io::Error::other("temporary-name retries exhausted")),
        ))
    }

    pub fn publish(mut self, current: &Path) -> SaveResult<()> {
        fs::rename(&self.path, current).map_err(|source| SaveError::AtomicPublish {
            path: current.to_path_buf(),
            detail: source.to_string(),
        })?;
        self.published = true;
        sync_parent_dir(current);
        Ok(())
    }

    pub fn publish_with_backup(self, current: &Path, backup: &Path) -> SaveResult<()> {
        if current.parent() != backup.parent() {
            return Err(SaveError::AtomicPublish {
                path: current.to_path_buf(),
                detail: "current file and backup must share a directory".to_string(),
            });
        }
        let rotated = match fs::rename(current, backup) {
            Ok(()) => true,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => false,
            Err(source) => {
                return Err(io_error("rotating current file to backup", current, source));
            }
        };
        if let Err(error) = self.publish(current) {
            if rotated {
                let _ = fs::rename(backup, current);
            }
            return Err(error);
        }
        Ok(())
    }
}

pub(crate) fn sync_parent_dir(path: &Path) {
    #[cfg(unix)]
    {
        if let Some(parent) = path.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Writes `bytes` to a sibling temp file and renames it over `path`,
/// keeping the previous file at `backup` when one is given
/// (`Util.safeReplaceFile`, `Util.java:854-873`).
pub fn write_atomic(path: &Path, backup: Option<&Path>, bytes: &[u8]) -> SaveResult<()> {
    let parent = path.parent().ok_or_else(|| SaveError::AtomicPublish {
        path: path.to_path_buf(),
        detail: "file has no parent directory".to_string(),
    })?;
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("save");
    let staged = StagedFile::write(parent, stem, bytes)?;
    match backup {
        Some(backup) => staged.publish_with_backup(path, backup),
        None => staged.publish(path),
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) fn open_file_matches_path(file: &File, path: &Path) -> bool {
    let Ok(open_metadata) = file.metadata() else {
        return false;
    };
    let Ok(path_link_metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if path_link_metadata.file_type().is_symlink() || !path_link_metadata.is_file() {
        return false;
    }
    let Ok(path_metadata) = fs::metadata(path) else {
        return false;
    };
    same_file(&open_metadata, &path_metadata)
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
        && left.created().ok() == right.created().ok()
}

pub fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> SaveError {
    SaveError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "bbb-server-world-{name}-{}-{}",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn staged_file_publishes_with_one_vanilla_style_backup() {
        let dir = temp_dir("atomic-publish");
        fs::create_dir_all(&dir).unwrap();
        let current = dir.join("level.dat");
        let backup = dir.join("level.dat_old");
        fs::write(&current, b"old-current").unwrap();
        fs::write(&backup, b"stale-backup").unwrap();

        StagedFile::write(&dir, "level", b"new-current")
            .unwrap()
            .publish_with_backup(&current, &backup)
            .unwrap();

        assert_eq!(fs::read(&current).unwrap(), b"new-current");
        assert_eq!(fs::read(&backup).unwrap(), b"old-current");
        assert_eq!(
            fs::read_dir(&dir)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(".bbb-"))
                .count(),
            0
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
