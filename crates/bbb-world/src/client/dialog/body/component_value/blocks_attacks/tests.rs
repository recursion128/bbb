use std::collections::BTreeMap;

use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    packets::inventory::{
        DamageTypeHolderSetSummary, DataComponentPatchSummary, SoundEventSummary,
    },
    sound_event_types::vanilla_sound_event_registry_id,
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::{RegistryContentState, RegistryPacketEntry, RegistryTagState};

use super::super::{
    apply_item_patch_value, decode_supported_component_value, DialogDataComponentValueState,
};
use super::*;

#[test]
fn decodes_blocks_attacks_defaults() {
    let decoded = decode(&compound([]), None).unwrap();
    let Some(DialogDataComponentValueState::BlocksAttacks { blocks_attacks }) = decoded else {
        panic!("expected blocks-attacks component");
    };
    assert_eq!(blocks_attacks.block_delay_seconds_bits, 0.0f32.to_bits());
    assert_eq!(blocks_attacks.disable_cooldown_scale_bits, 1.0f32.to_bits());
    assert_eq!(blocks_attacks.damage_reductions.len(), 1);
    assert_eq!(
        blocks_attacks.damage_reductions[0],
        DialogBlocksAttacksDamageReductionState {
            horizontal_blocking_angle_bits: 90.0f32.to_bits(),
            damage_types: None,
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        }
    );
    assert_eq!(
        blocks_attacks.item_damage,
        DialogBlocksAttacksItemDamageState {
            threshold_bits: 1.0f32.to_bits(),
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        }
    );
    assert_eq!(blocks_attacks.bypassed_by, None);
    assert_eq!(blocks_attacks.block_sound, None);
    assert_eq!(blocks_attacks.disabled_sound, None);
}

#[test]
fn decodes_blocks_attacks_nested_records_holders_and_sounds() {
    let registries = damage_type_registries();
    let nan_bits = f32::NAN.to_bits();
    let decoded = decode(
        &compound([
            ("block_delay_seconds", float(0.25)),
            ("disable_cooldown_scale", NbtSummaryValue::Int(2)),
            (
                "damage_reductions",
                NbtSummaryValue::List(vec![
                    compound([
                        ("horizontal_blocking_angle", float(45.0)),
                        ("type", string("#example:fire")),
                        ("base", float(-2.0)),
                        ("factor", NbtSummaryValue::Float(nan_bits)),
                    ]),
                    compound([
                        (
                            "type",
                            NbtSummaryValue::List(vec![string("in_fire"), string("explosion")]),
                        ),
                        ("base", float(2.5)),
                        ("factor", float(-0.25)),
                    ]),
                ]),
            ),
            (
                "item_damage",
                compound([
                    ("threshold", float(3.0)),
                    ("base", NbtSummaryValue::Float(f32::INFINITY.to_bits())),
                    ("factor", float(-1.0)),
                ]),
            ),
            ("bypassed_by", string("explosion")),
            ("block_sound", string("item.shield.block")),
            ("disabled_sound", direct_sound("example:disable", 8.0)),
        ]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::BlocksAttacks { blocks_attacks }) = decoded else {
        panic!("expected blocks-attacks component");
    };
    assert_eq!(
        f32::from_bits(blocks_attacks.block_delay_seconds_bits),
        0.25
    );
    assert_eq!(blocks_attacks.damage_reductions.len(), 2);
    assert_eq!(
        blocks_attacks.damage_reductions[0].horizontal_blocking_angle_bits,
        45.0f32.to_bits()
    );
    assert_eq!(blocks_attacks.damage_reductions[0].factor_bits, nan_bits);
    let named_types = blocks_attacks.damage_reductions[0]
        .damage_types
        .as_ref()
        .unwrap();
    assert_eq!(named_types.holder_set_kind, "named");
    assert_eq!(named_types.tag.as_deref(), Some("example:fire"));
    assert_eq!(named_types.entries[0].holder_id, 0);
    let direct_types = blocks_attacks.damage_reductions[1]
        .damage_types
        .as_ref()
        .unwrap();
    assert_eq!(
        direct_types
            .entries
            .iter()
            .map(|entry| entry.holder_id)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(
        blocks_attacks.item_damage.base_bits,
        f32::INFINITY.to_bits()
    );
    assert_eq!(
        blocks_attacks.bypassed_by.as_ref().unwrap().entries[0].holder_id,
        1
    );
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        &DialogDataComponentValueState::BlocksAttacks {
            blocks_attacks: blocks_attacks.clone(),
        },
        &mut patch,
    ));
    let summary = patch.blocks_attacks.unwrap();
    assert_eq!(summary.block_delay_seconds_bits, 0.25f32.to_bits());
    assert_eq!(summary.disable_cooldown_scale_bits, 2.0f32.to_bits());
    assert_eq!(summary.damage_reductions.len(), 2);
    assert_eq!(
        summary.damage_reductions[0].damage_types,
        Some(DamageTypeHolderSetSummary {
            registry_ids: vec![0],
            tag: Some("example:fire".to_string()),
        })
    );
    assert_eq!(
        summary.damage_reductions[1]
            .damage_types
            .as_ref()
            .unwrap()
            .registry_ids,
        [0, 1]
    );
    assert_eq!(summary.damage_reductions[0].factor_bits, nan_bits);
    assert_eq!(summary.item_damage.base_bits, f32::INFINITY.to_bits());
    assert_eq!(
        summary.bypassed_by,
        Some(DamageTypeHolderSetSummary {
            registry_ids: vec![1],
            tag: None,
        })
    );
    assert_eq!(
        summary.block_sound,
        Some(SoundEventSummary {
            registry_id: Some(
                vanilla_sound_event_registry_id("minecraft:item.shield.block").unwrap(),
            ),
            sound_id: None,
            fixed_range_bits: None,
        })
    );
    assert_eq!(
        summary.disable_sound,
        Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:disable".to_string()),
            fixed_range_bits: Some(8.0f32.to_bits()),
        })
    );

    let mut invalid = blocks_attacks.clone();
    invalid.damage_reductions[0].damage_types = Some(DialogRegistryHolderSetState {
        holder_set_kind: "named".to_string(),
        tag: None,
        entries: Vec::new(),
    });
    assert!(!apply_item_patch_value(
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        &DialogDataComponentValueState::BlocksAttacks {
            blocks_attacks: invalid,
        },
        &mut DataComponentPatchSummary::default(),
    ));
    assert!(matches!(
        blocks_attacks.block_sound,
        Some(DialogSoundEventHolderState::Named { registry_id, resource_id })
            if registry_id == vanilla_sound_event_registry_id("minecraft:item.shield.block").unwrap()
                && resource_id == "minecraft:item.shield.block"
    ));
    assert!(matches!(
        blocks_attacks.disabled_sound,
        Some(DialogSoundEventHolderState::Direct {
            sound_id,
            fixed_range_bits: Some(bits),
        }) if sound_id == "example:disable" && bits == 8.0f32.to_bits()
    ));
}

#[test]
fn decodes_explicit_empty_blocks_attacks_collections_without_registry_context() {
    let decoded = decode(
        &compound([
            ("damage_reductions", NbtSummaryValue::List(Vec::new())),
            ("bypassed_by", NbtSummaryValue::List(Vec::new())),
        ]),
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::BlocksAttacks { blocks_attacks }) = decoded else {
        panic!("expected blocks-attacks component");
    };
    assert!(blocks_attacks.damage_reductions.is_empty());
    assert!(blocks_attacks.bypassed_by.unwrap().entries.is_empty());
}

#[test]
fn rejects_malformed_blocks_attacks_records_and_ranges() {
    let registries = damage_type_registries();
    for (value, context, expected) in [
        (NbtSummaryValue::Int(0), None, "is not a compound"),
        (
            compound([("block_delay_seconds", float(-0.1))]),
            None,
            "block_delay_seconds -0.1 is outside",
        ),
        (
            compound([(
                "disable_cooldown_scale",
                NbtSummaryValue::Float(f32::NAN.to_bits()),
            )]),
            None,
            "disable_cooldown_scale NaN is outside",
        ),
        (
            compound([("damage_reductions", NbtSummaryValue::Int(0))]),
            None,
            "damage_reductions is not a list",
        ),
        (
            compound([(
                "damage_reductions",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            )]),
            None,
            "damage_reductions[0] is not a compound",
        ),
        (
            compound([(
                "damage_reductions",
                NbtSummaryValue::List(vec![compound([
                    ("base", float(0.0)),
                    ("factor", float(1.0)),
                    ("horizontal_blocking_angle", float(0.0)),
                ])]),
            )]),
            None,
            "horizontal_blocking_angle 0 is outside",
        ),
        (
            compound([(
                "damage_reductions",
                NbtSummaryValue::List(vec![compound([("factor", float(1.0))])]),
            )]),
            None,
            "missing field base",
        ),
        (
            compound([("item_damage", NbtSummaryValue::Int(0))]),
            None,
            "item_damage is not a compound",
        ),
        (
            compound([(
                "item_damage",
                compound([
                    ("threshold", float(-0.0)),
                    ("base", float(0.0)),
                    ("factor", float(1.0)),
                ]),
            )]),
            None,
            "threshold -0 is outside",
        ),
        (
            compound([(
                "item_damage",
                compound([("threshold", float(1.0)), ("factor", float(1.0))]),
            )]),
            None,
            "missing field base",
        ),
        (
            compound([("bypassed_by", string("explosion"))]),
            None,
            "without registry context",
        ),
        (
            compound([("bypassed_by", string("missing"))]),
            Some(&registries),
            "missing minecraft:damage_type resource",
        ),
        (
            compound([("block_sound", string("example:missing"))]),
            None,
            "missing minecraft:sound_event resource",
        ),
    ] {
        let error = decode(&value, context).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn damage_type_registries() -> RegistrySet {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(DAMAGE_TYPE_REGISTRY);
    contents.entries = vec![
        RegistryPacketEntry::stub("minecraft:in_fire"),
        RegistryPacketEntry::stub("minecraft:explosion"),
    ];
    registries
        .contents
        .insert(DAMAGE_TYPE_REGISTRY.to_string(), contents);
    registries.tags.insert(
        DAMAGE_TYPE_REGISTRY.to_string(),
        RegistryTagState {
            registry: DAMAGE_TYPE_REGISTRY.to_string(),
            tags: BTreeMap::from([("example:fire".to_string(), vec![0])]),
        },
    );
    registries
}

fn decode(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        "minecraft:blocks_attacks",
        value,
        registries,
    )
}

fn direct_sound(sound_id: &str, range: f32) -> NbtSummaryValue {
    compound([("sound_id", string(sound_id)), ("range", float(range))])
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

fn compound(entries: impl IntoIterator<Item = (&'static str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.to_string(),
                value,
            })
            .collect(),
    )
}
