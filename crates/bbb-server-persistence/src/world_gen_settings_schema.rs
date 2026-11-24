//! Saved world generator settings and retained generator payloads.

use std::collections::BTreeMap;

use bbb_protocol::NbtSummaryValue;
use bbb_server_types::save::{
    default_overworld_level_stem, parse_level_stem, Java26BuiltinDimension, Java26LevelKey,
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths,
};
pub use bbb_server_types::save::{Java26LevelStem, Java26WorldGenSettings};

use super::level_file::read_gzip_compound;
use super::schema_support::{bool_or, compound_or_empty, field, long_or, require_java_26_1_root};

const SCHEMA: &str = "world_gen_settings";

pub fn load_java_26_1_world_gen_settings(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Java26WorldGenSettings> {
    let (document, decompressed_bytes) =
        read_gzip_compound(&paths.world_gen_settings_dat(), limits)?;
    parse_java_26_1_world_gen_settings(document, decompressed_bytes)
}

pub fn parse_java_26_1_world_gen_settings(
    document: NamedCompound,
    decompressed_bytes: usize,
) -> SaveResult<Java26WorldGenSettings> {
    let entries = require_java_26_1_root(&document, SCHEMA)?;
    let data = compound_or_empty(entries, "data");
    let seed = long_or(data, "seed", 0);
    let generate_structures = bool_or(data, "generate_structures", true);
    let generate_bonus_chest = bool_or(data, "bonus_chest", false);
    let legacy_custom_options = match field(data, "legacy_custom_options") {
        Some(NbtSummaryValue::String(value)) => Some(value.clone()),
        _ => None,
    };
    let mut dimensions = BTreeMap::new();
    for stem_entry in compound_or_empty(data, "dimensions") {
        let Some(key) = stem_entry
            .name
            .try_to_string()
            .ok()
            .and_then(|raw| Java26LevelKey::parse(&raw).ok())
        else {
            continue;
        };
        if let Some(stem) = parse_level_stem(stem_entry) {
            dimensions.entry(key).or_insert(stem);
        }
    }
    // LevelStorageSource.java:153-164: unreadable worldgen settings use startup defaults.
    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    if let std::collections::btree_map::Entry::Vacant(e) = dimensions.entry(overworld) {
        e.insert(default_overworld_level_stem()?);
    }
    Ok(Java26WorldGenSettings {
        seed,
        generate_structures,
        generate_bonus_chest,
        legacy_custom_options,
        dimensions,
        document,
        decompressed_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        decode_named_saved_nbt_summary, encode_named_nbt_summary_java_string, NbtSummaryEntry,
    };
    use bbb_server_types::save::JAVA_26_1_DATA_VERSION;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn stem(dimension_type: NbtSummaryValue, generator_type: &str) -> NbtSummaryValue {
        compound(vec![
            entry("type", dimension_type),
            entry(
                "generator",
                compound(vec![
                    entry("type", NbtSummaryValue::string(generator_type)),
                    entry("settings", NbtSummaryValue::string("minecraft:overworld")),
                ]),
            ),
        ])
    }

    fn valid_document() -> NamedCompound {
        NamedCompound::new(
            "",
            vec![
                entry(
                    "data",
                    compound(vec![
                        entry("seed", NbtSummaryValue::Long(42)),
                        entry("generate_structures", NbtSummaryValue::Byte(1)),
                        entry("bonus_chest", NbtSummaryValue::Byte(0)),
                        entry(
                            "dimensions",
                            compound(vec![
                                entry(
                                    "minecraft:overworld",
                                    stem(
                                        NbtSummaryValue::string("minecraft:overworld"),
                                        "minecraft:noise",
                                    ),
                                ),
                                entry(
                                    "minecraft:the_nether",
                                    stem(
                                        NbtSummaryValue::string("minecraft:the_nether"),
                                        "minecraft:noise",
                                    ),
                                ),
                                entry(
                                    "minecraft:the_end",
                                    stem(
                                        NbtSummaryValue::string("minecraft:the_end"),
                                        "minecraft:noise",
                                    ),
                                ),
                            ]),
                        ),
                    ]),
                ),
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            ],
        )
    }

    fn parse(document: NamedCompound) -> SaveResult<Java26WorldGenSettings> {
        parse_java_26_1_world_gen_settings(document, 0)
    }

    fn key(raw: &str) -> Java26LevelKey {
        Java26LevelKey::parse(raw).unwrap()
    }

    #[test]
    fn retained_document_round_trips_through_summary_encoding_unchanged() {
        let document = valid_document();
        let settings = parse(document.clone()).unwrap();
        assert_eq!(settings.document.to_summary(), document.to_summary());

        // The same summary encode/decode used by
        // `level_file::encode_gzip_named_compound` must reproduce the retained
        // document exactly, so the writer can emit it verbatim.
        let bytes = encode_named_nbt_summary_java_string(
            settings.document.root_name(),
            &settings.document.to_summary(),
            WorldSaveLimits::default().max_decompressed_nbt_bytes,
        )
        .unwrap();
        let (root_name, summary) = decode_named_saved_nbt_summary(&bytes).unwrap().unwrap();
        assert_eq!(&root_name, document.root_name());
        assert_eq!(summary, document.to_summary());
    }

    #[test]
    fn writer_canonical_official_snapshot_resolves_all_builtin_stems() {
        let settings = parse(valid_document()).unwrap();
        assert_eq!(settings.seed, 42);
        assert!(settings.generate_structures);
        assert!(!settings.generate_bonus_chest);
        assert_eq!(settings.legacy_custom_options, None);
        assert_eq!(settings.dimensions.len(), 3);
        let overworld = &settings.dimensions[&key("minecraft:overworld")];
        assert_eq!(overworld.dimension_type, "minecraft:overworld");
        assert_eq!(overworld.generator, "minecraft:noise");
        assert!(matches!(
            overworld.raw_generator,
            NbtSummaryValue::Compound(_)
        ));
    }

    #[test]
    fn defaulted_options_and_flat_debug_generators_accept() {
        let mut document = valid_document();
        {
            let NbtSummaryValue::Compound(data) = &mut document.entries_mut()[0].value else {
                unreachable!();
            };
            data.remove(2); // bonus_chest
            data.remove(1); // generate_structures; defaults apply
            data.push(entry(
                "legacy_custom_options",
                NbtSummaryValue::string("legacy"),
            ));
            let NbtSummaryValue::Compound(dimensions) = &mut data[1].value else {
                unreachable!();
            };
            dimensions.truncate(1); // overworld only; nether/end fall back to the built-in base
            dimensions[0].value = stem(
                NbtSummaryValue::string("minecraft:overworld_caves"),
                "minecraft:flat",
            );
        }
        let NbtSummaryValue::Compound(data) = &mut document.entries_mut()[0].value else {
            unreachable!();
        };
        data.retain(|entry| entry.name != "seed");
        let settings = parse(document).unwrap();
        assert_eq!(settings.seed, 0);
        assert!(settings.generate_structures);
        assert!(!settings.generate_bonus_chest);
        assert_eq!(
            settings.legacy_custom_options.as_ref(),
            Some(&bbb_protocol::JavaString::from("legacy"))
        );
        assert_eq!(settings.dimensions.len(), 1);
        let overworld = &settings.dimensions[&key("minecraft:overworld")];
        assert_eq!(overworld.dimension_type, "minecraft:overworld_caves");
        assert_eq!(overworld.generator, "minecraft:flat");
    }

    fn dimensions_mut(document: &mut NamedCompound) -> &mut Vec<NbtSummaryEntry> {
        let NbtSummaryValue::Compound(data) = &mut document.entries_mut()[0].value else {
            unreachable!();
        };
        let dimensions = data.iter_mut().find(|e| e.name == "dimensions").unwrap();
        let NbtSummaryValue::Compound(entries) = &mut dimensions.value else {
            unreachable!();
        };
        entries
    }

    #[test]
    fn custom_stems_and_dimension_type_ids_are_accepted() {
        let mut document = valid_document();
        dimensions_mut(&mut document).push(entry(
            "example:moon",
            stem(
                NbtSummaryValue::string("example:moon_type"),
                "minecraft:noise",
            ),
        ));
        dimensions_mut(&mut document).remove(0);
        dimensions_mut(&mut document).push(entry("bad stem", NbtSummaryValue::Int(0)));
        let original = document.clone();
        let settings = parse(document).unwrap();
        assert_eq!(settings.seed, 42);
        assert!(settings
            .dimensions
            .contains_key(&key("minecraft:overworld")));
        assert_eq!(settings.document, original);
        assert_eq!(
            settings.dimensions[&key("example:moon")].dimension_type,
            "example:moon_type"
        );
    }
}
