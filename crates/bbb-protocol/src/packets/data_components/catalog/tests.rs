use std::collections::{BTreeMap, HashSet};

use bbb_facts::vanilla_map_decoration_types;

use crate::{
    block_entity_types::{VANILLA_BLOCK_ENTITY_TYPES, VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID},
    builtin_enum_component_values::{
        VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_RED_ID, VANILLA_DYE_COLOR_VALUES,
    },
    codec::Encoder,
    entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    },
};

use super::*;

fn write_nested_adventure_mode_predicate(out: &mut Encoder, depth: usize) {
    out.write_var_i32(1);
    out.write_bool(false);
    out.write_bool(false);
    out.write_bool(false);
    if depth > 1 {
        out.write_var_i32(1);
        out.write_var_i32(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID);
        write_nested_adventure_mode_predicate(out, depth - 1);
    } else {
        out.write_var_i32(0);
    }
    out.write_var_i32(0);
}

fn nested_adventure_mode_predicate_summary(depth: usize) -> AdventureModePredicateDataSummary {
    let exact_components = (depth > 1).then(|| {
        Box::new(DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
            can_place_on_persistent: Some(nested_adventure_mode_predicate_summary(depth - 1)),
            ..DataComponentPatchSummary::default()
        })
    });
    AdventureModePredicateDataSummary {
        predicates: vec![BlockPredicateSummary {
            exact_components,
            ..BlockPredicateSummary::default()
        }],
    }
}

fn direct_instrument_patch() -> DataComponentPatchSummary {
    let styled = vec![StyledTextRun {
        text: "Custom horn".to_string(),
        style: Default::default(),
    }];
    DataComponentPatchSummary {
        instrument: InstrumentComponentSummary {
            registry_id: None,
            direct: Some(Box::new(InstrumentSummary {
                sound_event: SoundEventSummary {
                    registry_id: Some(7),
                    sound_id: None,
                    fixed_range_bits: None,
                },
                use_duration_bits: 3.25_f32.to_bits(),
                range_bits: 96.0_f32.to_bits(),
                description: "Custom horn".to_string(),
                description_styled: Some(styled.into_boxed_slice()),
                description_nbt: Some(Box::new(NbtSummaryValue::String("Custom horn".into()))),
            })),
        },
        ..DataComponentPatchSummary::default()
    }
}

fn banner_pattern_layers() -> Vec<BannerPatternLayerSummary> {
    vec![
        BannerPatternLayerSummary {
            registry_id: Some(4),
            asset_id: None,
            translation_key: None,
            color_id: VANILLA_DYE_COLOR_RED_ID,
        },
        BannerPatternLayerSummary {
            registry_id: None,
            asset_id: Some("minecraft:stripe_bottom".to_string()),
            translation_key: Some("block.minecraft.banner.stripe_bottom".into()),
            color_id: VANILLA_DYE_COLOR_BLUE_ID,
        },
    ]
}

fn direct_painting_variant() -> PaintingVariantSummary {
    PaintingVariantSummary {
        width: 4,
        height: 2,
        asset_id: "minecraft:gallery".to_string(),
        title: Some("Gallery".to_string()),
        title_styled: Some(vec![StyledTextRun {
            text: "Gallery".to_string(),
            style: Default::default(),
        }]),
        title_nbt: Some(Box::new(NbtSummaryValue::String("Gallery".into()))),
        author: None,
        author_styled: None,
        author_nbt: None,
    }
}

fn bee_occupant() -> BeeOccupantSummary {
    BeeOccupantSummary {
        entity_type_id: VANILLA_ENTITY_TYPE_BEE_ID,
        entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
        ticks_in_hive: i32::MIN,
        min_ticks_in_hive: i32::MAX,
    }
}

fn direct_jukebox_playable_patch() -> DataComponentPatchSummary {
    DataComponentPatchSummary {
        jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
            sound_event: SoundEventSummary {
                registry_id: Some(7),
                sound_id: None,
                fixed_range_bits: None,
            },
            description: "Custom song".to_string(),
            description_styled: Some(
                vec![StyledTextRun {
                    text: "Custom song".to_string(),
                    style: Default::default(),
                }]
                .into_boxed_slice(),
            ),
            description_nbt: Some(Box::new(NbtSummaryValue::String("Custom song".into()))),
            length_in_seconds_bits: 60.0_f32.to_bits(),
            comparator_output: 7,
        })),
        ..DataComponentPatchSummary::default()
    }
}

fn canonical_trim_description(nbt: &NbtSummaryValue) -> (String, Box<[StyledTextRun]>) {
    let encoded = encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
    let (runs, canonical_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(&encoded)).unwrap();
    assert_eq!(canonical_nbt.as_ref(), Some(nbt));
    (styled_runs_summary_text(&runs), runs.into_boxed_slice())
}

fn direct_trim_patch() -> DataComponentPatchSummary {
    let material_nbt = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("Ruby".into()),
        },
        NbtSummaryEntry {
            name: "color".into(),
            value: NbtSummaryValue::String("red".into()),
        },
    ]);
    let pattern_nbt = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("Spiral".into()),
        },
        NbtSummaryEntry {
            name: "italic".into(),
            value: NbtSummaryValue::Byte(1),
        },
    ]);
    let (material_description, material_styled) = canonical_trim_description(&material_nbt);
    let (pattern_description, pattern_styled) = canonical_trim_description(&pattern_nbt);
    DataComponentPatchSummary {
        armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
            asset_name: "ruby".to_string(),
            override_armor_assets: BTreeMap::from([
                ("minecraft:netherite".to_string(), "ruby_darker".to_string()),
                ("example:opal".to_string(), "ruby_opal".to_string()),
            ]),
            description: material_description,
            description_styled: Some(material_styled),
            description_nbt: Some(Box::new(material_nbt)),
        })),
        armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
            asset_id: "example:spiral".to_string(),
            description: pattern_description,
            description_styled: Some(pattern_styled),
            description_nbt: Some(Box::new(pattern_nbt)),
            decal: true,
        })),
        ..DataComponentPatchSummary::default()
    }
}

mod batch_01;
mod batch_02;
mod batch_03;
mod batch_04;
