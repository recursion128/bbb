//! What a save directory has to look like before it is read, and the
//! resource budgets a read may not exceed.
//!
//! Split out of `world_storage.rs`.

use crate::atomic_file::io_error;
use bbb_server_types::save::{Java26LevelKey, SaveError, SaveResult, WorldSavePaths};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn validate_dimension_storage(
    paths: &WorldSavePaths,
    level_keys: &[Java26LevelKey],
) -> SaveResult<()> {
    validate_level_key_roots(paths, level_keys)?;
    reject_managed_symlinks(paths, level_keys)
}

fn validate_level_key_roots(
    paths: &WorldSavePaths,
    level_keys: &[Java26LevelKey],
) -> SaveResult<()> {
    let mut roots = BTreeSet::new();
    for key in level_keys {
        let relative = key.storage_relative_path();
        if !roots.insert(relative.clone()) {
            return Err(SaveError::InvalidPath {
                path: paths.root().join("dimensions").join(relative),
                detail: "level keys resolve to the same managed directory",
            });
        }
    }
    for root in &roots {
        if roots
            .iter()
            .any(|other| other != root && root.starts_with(other))
        {
            return Err(SaveError::InvalidPath {
                path: paths.root().join("dimensions").join(root),
                detail: "level directory is nested below another managed level directory",
            });
        }
    }
    Ok(())
}

pub fn reject_symlink_root(root: &Path) -> SaveResult<()> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(SaveError::InvalidPath {
            path: root.to_path_buf(),
            detail: "world root must not be a symbolic link",
        }),
        Ok(metadata) if !metadata.is_dir() => Err(SaveError::InvalidPath {
            path: root.to_path_buf(),
            detail: "world root must be a directory",
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error("reading world root metadata", root, source)),
    }
}

pub fn reject_global_managed_symlinks(paths: &WorldSavePaths) -> SaveResult<()> {
    let mut managed = vec![
        paths.level_dat(),
        paths.level_dat_old(),
        paths.session_lock(),
        paths.saved_data_dir(),
        paths.saved_data_dir().join("minecraft"),
        crate::wandering_trader_schema::wandering_trader_dat(paths),
        paths.maps_dir(),
        paths.root().join("players"),
        paths.player_data_dir(),
        paths.player_stats_dir(),
        crate::player_advancements::player_advancements_dir(paths),
        paths.root().join("dimensions"),
    ];
    reject_symlinks(managed.drain(..))
}

pub fn reject_managed_symlinks(
    paths: &WorldSavePaths,
    level_keys: &[Java26LevelKey],
) -> SaveResult<()> {
    reject_global_managed_symlinks(paths)?;
    let dimensions_root = paths.root().join("dimensions");
    let mut managed = Vec::new();
    for key in level_keys {
        let mut ancestor = dimensions_root.clone();
        for component in key.storage_relative_path().components() {
            ancestor.push(component);
            managed.push(ancestor.clone());
        }
        let dimension_root = paths.dimension_dir(key);
        managed.push(dimension_root.clone());
        managed.push(dimension_root.join("region"));
        managed.push(dimension_root.join("entities"));
        managed.push(dimension_root.join("poi"));
        managed.push(dimension_root.join("data"));
        managed.push(dimension_root.join("data").join("minecraft"));
    }
    reject_symlinks(managed)
}

fn reject_symlinks(paths: impl IntoIterator<Item = PathBuf>) -> SaveResult<()> {
    for path in paths {
        if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(SaveError::InvalidPath {
                path,
                detail: "managed world-save path must not be a symbolic link",
            });
        }
    }
    Ok(())
}
