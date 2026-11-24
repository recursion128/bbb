//! `RandomSequences.TYPE` (`RandomSequences.java:26-28`) on disk.
//!
//! `SavedDataStorage.getDataFile` (`SavedDataStorage.java:56-63`) resolves the
//! type id `minecraft:random_sequences` against the *level* data folder the
//! server builds once (`MinecraftServer.java:330`, `:343`), so the file is
//! `<world>/data/minecraft/random_sequences.dat` — one per save, not one per
//! dimension, because `MinecraftServer.getRandomSequence`
//! (`MinecraftServer.java:1758-1760`) hands every level the same map.
//!
//! `SavedDataStorage.encodeUnchecked` (`SavedDataStorage.java:198-204`) puts
//! the codec's output under `data` beside `NbtUtils.addCurrentDataVersion`, and
//! `tryWrite` (`:206-215`) gzips it, which makes the document
//!
//! ```text
//! { DataVersion: <int>, data: {
//!     salt: <int>,
//!     include_world_seed: <byte>,    // omitted when true
//!     include_sequence_id: <byte>,   // omitted when true
//!     sequences: { "<id>": { source: [L; <seedLo>, <seedHi>] }, .. },
//! } }
//! ```
//!
//! from `RandomSequences.CODEC` (`RandomSequences.java:17-25`) over
//! `RandomSequence.CODEC` (`RandomSequence.java:12-14`) over
//! `Xoroshiro128PlusPlus.CODEC` (`Xoroshiro128PlusPlus.java:10-11`), whose
//! `Codec.LONG_STREAM` of fixed size two `NbtOps` writes as a long array. The
//! two `optionalFieldOf(name, true)` flags are dropped on encode when they hold
//! their default, which is why a save an official server wrote carries neither
//! unless `/random reset` moved them.
//!
//! A save that never wrote the file is not an error: `SavedDataStorage.get`
//! answers null for a missing file (`SavedDataStorage.java:86-102`) and
//! `computeIfAbsent` builds a fresh `RandomSequences`; a sequence that does not
//! decode is dropped and restarts from its seed.

use std::collections::BTreeMap;

use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};

use bbb_server_types::random_sequences_saved::RandomSequencesSaved;

use super::schema_support::{bool_or, compound_or_empty, field, int_or, require_java_26_1_root};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

const SCHEMA: &str = "random_sequences";

/// The `random_sequences` saved data one save carries.
///
/// A missing file is a fresh `RandomSequences`, exactly as
/// `savedDataStorage.computeIfAbsent(RandomSequences.TYPE)`
/// (`MinecraftServer.java:343`) builds one.
pub fn load_java_26_1_random_sequences(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<RandomSequencesSaved> {
    let path = paths.random_sequences_dat();
    let Some((document, _)) = crate::level_file::read_optional_gzip_compound(&path, limits)? else {
        return Ok(RandomSequencesSaved::default());
    };
    let entries = require_java_26_1_root(&document, SCHEMA)?;
    let data = compound_or_empty(entries, "data");
    // `RandomSequences.CODEC` (`RandomSequences.java:17-25`).
    let salt = int_or(data, "salt", 0);
    let include_world_seed = bool_or(data, "include_world_seed", true);
    let include_sequence_id = bool_or(data, "include_sequence_id", true);
    let mut decoded = BTreeMap::new();
    for entry in compound_or_empty(data, "sequences") {
        let Some(key) = entry
            .name
            .try_to_string()
            .ok()
            .and_then(|raw| normalize_resource_location(raw).ok())
        else {
            continue;
        };
        let NbtSummaryValue::Compound(fields) = &entry.value else {
            continue;
        };
        // `Util.fixedSize(seed, 2)` (`Xoroshiro128PlusPlus.java:10-11`).
        let Some(NbtSummaryValue::LongArray(state)) = field(fields, "source") else {
            continue;
        };
        let [seed_lo, seed_hi] = state[..] else {
            continue;
        };
        decoded.entry(key).or_insert([seed_lo, seed_hi]);
    }
    Ok(RandomSequencesSaved {
        salt,
        include_world_seed,
        include_sequence_id,
        sequences: decoded,
    })
}

/// The `data/minecraft/random_sequences.dat` document one save is written with,
/// the inverse of [`load_java_26_1_random_sequences`].
pub fn java_26_1_random_sequences_document(sequences: &RandomSequencesSaved) -> NamedCompound {
    let mut data = vec![NbtSummaryEntry {
        name: "salt".into(),
        value: NbtSummaryValue::Int(sequences.salt),
    }];
    // `optionalFieldOf(name, true)` drops the field when it holds the default.
    if !sequences.include_world_seed {
        data.push(NbtSummaryEntry {
            name: "include_world_seed".into(),
            value: NbtSummaryValue::Byte(0),
        });
    }
    if !sequences.include_sequence_id {
        data.push(NbtSummaryEntry {
            name: "include_sequence_id".into(),
            value: NbtSummaryValue::Byte(0),
        });
    }
    data.push(NbtSummaryEntry {
        name: "sequences".into(),
        value: NbtSummaryValue::Compound(
            sequences
                .sequences
                .iter()
                .map(|(key, &[seed_lo, seed_hi])| NbtSummaryEntry {
                    name: key.as_str().into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "source".into(),
                        value: NbtSummaryValue::LongArray(vec![seed_lo, seed_hi]),
                    }]),
                })
                .collect(),
        ),
    });
    NamedCompound::new(
        "",
        vec![
            NbtSummaryEntry {
                name: "DataVersion".into(),
                value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
            },
            NbtSummaryEntry {
                name: "data".into(),
                value: NbtSummaryValue::Compound(data),
            },
        ],
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::level_file::encode_gzip_named_compound;
    use super::*;

    fn scratch(name: &str) -> WorldSavePaths {
        let root = std::env::temp_dir().join(format!(
            "bbb-random-sequences-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let paths = WorldSavePaths::new(&root).expect("the scratch root is a usable save path");
        fs::create_dir_all(paths.saved_data_dir().join("minecraft"))
            .expect("the data folder is creatable");
        paths
    }

    fn write(paths: &WorldSavePaths, document: &NamedCompound) {
        let limits = WorldSaveLimits::default();
        let path = paths.random_sequences_dat();
        fs::write(
            &path,
            encode_gzip_named_compound(&path, document, limits).expect("the document encodes"),
        )
        .expect("the file is writable");
    }

    fn data_fields(document: &NamedCompound) -> &[NbtSummaryEntry] {
        let entry = document
            .entries()
            .iter()
            .find(|entry| entry.name == "data")
            .expect("the document carries data");
        let NbtSummaryValue::Compound(fields) = &entry.value else {
            panic!("data must be a compound");
        };
        fields
    }

    /// A save that never wrote the file starts from the field initialisers
    /// (`RandomSequences.java:29-32`), the way `computeIfAbsent` does.
    #[test]
    fn a_save_without_the_file_starts_from_a_fresh_map() {
        let paths = scratch("absent");
        let loaded = load_java_26_1_random_sequences(&paths, WorldSaveLimits::default())
            .expect("a missing file is not an error");
        assert_eq!(loaded, RandomSequencesSaved::default());
        fs::remove_dir_all(paths.root()).expect("the scratch root is removable");
    }

    /// The empty document an official server writes for a map nothing has drawn
    /// from yet is `{salt:0,sequences:{}}` — both flags omitted because both
    /// hold their default.
    #[test]
    fn the_default_document_omits_both_flags() {
        let document = java_26_1_random_sequences_document(&RandomSequencesSaved::default());
        let fields = data_fields(&document);
        assert_eq!(
            fields.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
            vec!["salt", "sequences"]
        );
        assert_eq!(fields[0].value, NbtSummaryValue::Int(0));
        assert_eq!(fields[1].value, NbtSummaryValue::Compound(Vec::new()));
    }

    /// `/random reset` settings are written, and read back, as the two bytes
    /// the codec then has to see.
    #[test]
    fn non_default_flags_are_written_and_read_back() {
        let paths = scratch("flags");
        let sequences = RandomSequencesSaved {
            salt: 7,
            include_world_seed: false,
            include_sequence_id: false,
            sequences: BTreeMap::from([("minecraft:blocks/stone".to_string(), [1, 2])]),
        };
        let document = java_26_1_random_sequences_document(&sequences);
        let fields = data_fields(&document);
        assert_eq!(
            fields.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
            vec![
                "salt",
                "include_world_seed",
                "include_sequence_id",
                "sequences"
            ]
        );
        write(&paths, &document);
        assert_eq!(
            load_java_26_1_random_sequences(&paths, WorldSaveLimits::default())
                .expect("the document reads back"),
            sequences
        );
        fs::remove_dir_all(paths.root()).expect("the scratch root is removable");
    }

    #[test]
    fn sequence_state_pairs_round_trip_on_disk() {
        let paths = scratch("pairs");
        let sequences = RandomSequencesSaved {
            sequences: BTreeMap::from([
                (
                    "minecraft:blocks/glowstone".to_string(),
                    [i64::MIN, i64::MAX],
                ),
                ("minecraft:blocks/stone".to_string(), [0, 0]),
            ]),
            ..RandomSequencesSaved::default()
        };
        write(&paths, &java_26_1_random_sequences_document(&sequences));
        let reloaded = load_java_26_1_random_sequences(&paths, WorldSaveLimits::default())
            .expect("the document reads back");
        assert_eq!(reloaded, sequences);
        fs::remove_dir_all(paths.root()).expect("the scratch root is removable");
    }

    /// A key written without its namespace is the same sequence: `Identifier`
    /// defaults it to `minecraft`.
    #[test]
    fn a_bare_key_resolves_under_the_default_namespace() {
        let paths = scratch("bare");
        let document = NamedCompound::new(
            "",
            vec![
                NbtSummaryEntry {
                    name: "DataVersion".into(),
                    value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
                },
                NbtSummaryEntry {
                    name: "data".into(),
                    value: NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "salt".into(),
                            value: NbtSummaryValue::Int(0),
                        },
                        NbtSummaryEntry {
                            name: "sequences".into(),
                            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                                name: "blocks/stone".into(),
                                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                                    name: "source".into(),
                                    value: NbtSummaryValue::LongArray(vec![1, 2]),
                                }]),
                            }]),
                        },
                    ]),
                },
            ],
        );
        write(&paths, &document);
        let loaded = load_java_26_1_random_sequences(&paths, WorldSaveLimits::default())
            .expect("a bare key is a resource location");
        assert_eq!(
            loaded
                .sequences
                .iter()
                .map(|(key, state)| (key.to_string(), state))
                .collect::<Vec<_>>(),
            vec![("minecraft:blocks/stone".to_string(), &[1, 2])]
        );
        fs::remove_dir_all(paths.root()).expect("the scratch root is removable");
    }
}
