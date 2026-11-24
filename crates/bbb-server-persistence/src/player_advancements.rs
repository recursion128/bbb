//! Directory loading for `players/advancements/<uuid>.json` documents.

use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::level_file::read_optional_bounded_file;
pub use bbb_server_types::advancements::{
    file, format_obtained_time, parse_obtained_time, player_advancements_dir,
    player_advancements_file, progress, requirements, AdvancementProgress, AdvancementRequirements,
    CriterionProgress, Java26PlayerAdvancements, PlayerAdvancementsParseError,
};
use bbb_server_types::save::{SaveError, SaveResult, WorldSaveLimits, WorldSavePaths};

/// Discovers the profile uuids present in `players/advancements/<uuid>.json`
/// without reading any file bodies.
pub fn discover_java_26_1_player_advancements(
    paths: &WorldSavePaths,
) -> SaveResult<BTreeSet<Uuid>> {
    let dir = player_advancements_dir(paths);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => return Err(io_error("listing player advancements", &dir, error)),
    };
    let mut uuids = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| io_error("listing player advancements", &dir, error))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| io_error("reading player advancements entry type", &path, error))?;
        if file_type.is_symlink() {
            return Err(SaveError::InvalidPath {
                path,
                detail: "managed player advancements directory must not contain symbolic links",
            });
        }
        let Some(uuid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|stem| {
                let uuid = Uuid::parse_str(stem).ok()?;
                (uuid.hyphenated().to_string() == stem).then_some(uuid)
            })
        else {
            continue;
        };
        if !file_type.is_file() {
            return Err(SaveError::InvalidPath {
                path,
                detail: "canonical player advancements path must be a regular file",
            });
        }
        uuids.insert(uuid);
    }
    Ok(uuids)
}

/// Reads and parses a single `players/advancements/<uuid>.json` document on
/// demand using default save budgets.
pub fn load_java_26_1_player_advancement(
    paths: &WorldSavePaths,
    uuid: Uuid,
) -> SaveResult<Option<Java26PlayerAdvancements>> {
    load_java_26_1_player_advancement_with_limits(paths, WorldSaveLimits::default(), uuid)
}

/// Reads and parses a single `players/advancements/<uuid>.json` document on
/// demand under `limits`.
pub fn load_java_26_1_player_advancement_with_limits(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    uuid: Uuid,
) -> SaveResult<Option<Java26PlayerAdvancements>> {
    let path = player_advancements_file(paths, uuid);
    let Some(bytes) = read_optional_bounded_file(
        &path,
        "reading player advancements",
        "player advancements JSON",
        limits.max_decompressed_nbt_bytes,
    )?
    else {
        return Ok(None);
    };
    let data =
        Java26PlayerAdvancements::parse_bytes(&bytes).map_err(|error| SaveError::InvalidNbt {
            path: Some(path.clone()),
            detail: error.to_string(),
        })?;
    Ok(Some(data))
}

/// Every `players/advancements/<uuid>.json` the save holds.
pub fn load_java_26_1_player_advancements(
    paths: &WorldSavePaths,
) -> SaveResult<BTreeMap<Uuid, Java26PlayerAdvancements>> {
    let uuids = discover_java_26_1_player_advancements(paths)?;
    let mut loaded = BTreeMap::new();
    for uuid in uuids {
        if let Some(data) = load_java_26_1_player_advancement(paths, uuid)? {
            loaded.insert(uuid, data);
        }
    }
    Ok(loaded)
}

fn io_error(operation: &'static str, path: &std::path::Path, source: std::io::Error) -> SaveError {
    SaveError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::JavaString;

    const OBTAINED_MILLIS: i64 = 1_786_000_000_000;

    fn scratch_world(name: &str) -> WorldSavePaths {
        let root = std::env::temp_dir().join(format!(
            "bbb-player-advancements-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch world root is creatable");
        WorldSavePaths::new(root).expect("the scratch world root is a usable save path")
    }

    #[test]
    fn the_loader_reads_every_document_the_directory_holds() {
        let paths = scratch_world("directory");
        assert!(load_java_26_1_player_advancements(&paths)
            .expect("a missing directory is not an error")
            .is_empty());

        let played = Uuid::from_u128(1);
        std::fs::create_dir_all(player_advancements_dir(&paths)).unwrap();
        std::fs::write(
            player_advancements_file(&paths, played),
            Java26PlayerAdvancements {
                progress: BTreeMap::from([(
                    "minecraft:story/root".to_string(),
                    AdvancementProgress::from_criteria(BTreeMap::from([(
                        JavaString::from("a"),
                        CriterionProgress::obtained(OBTAINED_MILLIS),
                    )])),
                )]),
            }
            .to_file_bytes(),
        )
        .unwrap();
        std::fs::write(player_advancements_dir(&paths).join("Steve.json"), b"{}").unwrap();

        let loaded = load_java_26_1_player_advancements(&paths).expect("the document is readable");
        assert_eq!(loaded.keys().collect::<Vec<_>>(), [&played]);
        assert_eq!(
            loaded[&played].progress["minecraft:story/root"]
                .criterion(&JavaString::from("a"))
                .and_then(CriterionProgress::obtained_epoch_millis),
            Some(OBTAINED_MILLIS)
        );

        std::fs::write(
            player_advancements_file(&paths, Uuid::from_u128(3)),
            Java26PlayerAdvancements::default().to_file_bytes(),
        )
        .unwrap();
        assert_eq!(
            load_java_26_1_player_advancements(&paths)
                .expect("both documents are readable")
                .len(),
            2
        );

        // Non-canonical UUID spellings are ignored, while a non-regular canonical UUID entry fails.
        let other = Uuid::from_u128(0xabc);
        std::fs::write(
            player_advancements_dir(&paths).join(format!(
                "{}.json",
                other.hyphenated().to_string().to_uppercase()
            )),
            b"{}",
        )
        .unwrap();
        assert!(!discover_java_26_1_player_advancements(&paths)
            .unwrap()
            .contains(&other));

        let tight = WorldSaveLimits {
            max_decompressed_nbt_bytes: 8,
            ..Default::default()
        };
        assert!(matches!(
            load_java_26_1_player_advancement_with_limits(&paths, tight, played),
            Err(SaveError::BudgetExceeded { .. })
        ));

        let dir_uuid = Uuid::from_u128(0xdef);
        std::fs::create_dir(player_advancements_file(&paths, dir_uuid)).unwrap();
        assert!(matches!(
            discover_java_26_1_player_advancements(&paths),
            Err(SaveError::InvalidPath { .. })
        ));
        let _ = std::fs::remove_dir_all(paths.root());
    }
}
