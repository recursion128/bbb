//! `minecraft:glider`, `minecraft:kinetic_weapon` and
//! `minecraft:blocks_attacks`: the unit component and the two records whose
//! persistent codecs nest records of their own.
//!
//! `minecraft:glider` is `persistent(Unit.CODEC)`, so its save form is the
//! empty compound and nothing else. The two records drop every field that
//! holds its `optionalFieldOf` default, and `minecraft:blocks_attacks` has
//! defaults that are not the zero value: one `damage_reductions` entry and an
//! all-default `item_damage`.

use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID;
use bbb_protocol::packets::{
    BlocksAttacksSummary, DamageReductionSummary, HolderSetSummary, ItemDamageFunctionSummary,
    KineticWeaponConditionSummary, KineticWeaponSummary,
};

use super::holder::DAMAGE_TYPE_REGISTRY;
use super::*;

const ITEM_BREAK_SOUND: &str = "minecraft:entity.item.break";

fn one_component(resource_id: &str, value: NbtSummaryValue) -> NbtSummaryValue {
    saved_stack_with_components("minecraft:stone", vec![entry(resource_id, value)])
}

fn empty(type_resource_id: &str) -> NbtSummaryValue {
    let _ = type_resource_id;
    NbtSummaryValue::Compound(Vec::new())
}

/// Decodes a save form, asserts the summary re-encodes to exactly that form,
/// and hands the summary back.
fn round_trip_with<R: PersistentItemStackHolderResolver>(
    saved: &NbtSummaryValue,
    resolver: &R,
) -> ItemStackSummary {
    let summary = crate::read_item_stack(saved, resolver)
        .unwrap_or_else(|error| panic!("{saved:?}: {error}"));
    assert_eq!(
        item_stack_summary_to_persistent_nbt_with_resolver(&summary, resolver).unwrap(),
        saved.clone()
    );
    summary
}

fn kinetic_weapon(summary: &ItemStackSummary) -> KineticWeaponSummary {
    summary
        .component_patch
        .kinetic_weapon
        .clone()
        .expect("the summary carries a kinetic weapon")
}

fn blocks_attacks(summary: &ItemStackSummary) -> BlocksAttacksSummary {
    summary
        .component_patch
        .blocks_attacks
        .clone()
        .expect("the summary carries blocks attacks")
}

/// `minecraft:glider` is `persistent(Unit.CODEC)`
/// and `Unit.CODEC` is `MapCodec.unitCodec`, whose only save
/// form is the empty compound.
#[test]
fn glider_round_trips_as_the_empty_unit_compound() {
    let saved = one_component("minecraft:glider", NbtSummaryValue::Compound(Vec::new()));
    let summary = round_trip_with(&saved, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.added_type_ids,
        [VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID]
    );
    assert_eq!(summary.component_patch.added_type_ids.len(), 1);

    // The declaration is the whole of the value, so a summary that names the
    // component writes the same empty compound back.
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved.clone()
    );
}

/// `KineticWeapon.CODEC`: an all-default weapon is
/// `{}`, and every field away from its default is written.
#[test]
fn kinetic_weapon_round_trips_and_drops_its_codec_defaults() {
    let defaults = one_component("minecraft:kinetic_weapon", empty("kinetic_weapon"));
    let summary = round_trip_with(&defaults, &BuiltinResolver);
    let weapon = kinetic_weapon(&summary);
    assert_eq!(weapon.contact_cooldown_ticks, 10);
    assert_eq!(weapon.delay_ticks, 0);
    assert!(weapon.dismount_conditions.is_none());
    assert!(weapon.knockback_conditions.is_none());
    assert!(weapon.damage_conditions.is_none());
    assert_eq!(weapon.forward_movement_bits, 0.0f32.to_bits());
    assert_eq!(weapon.damage_multiplier_bits, 1.0f32.to_bits());
    assert!(weapon.sound.is_none() && weapon.hit_sound.is_none());

    let written = one_component(
        "minecraft:kinetic_weapon",
        NbtSummaryValue::Compound(vec![
            entry("contact_cooldown_ticks", NbtSummaryValue::Int(4)),
            entry("delay_ticks", NbtSummaryValue::Int(7)),
            entry(
                "damage_conditions",
                NbtSummaryValue::Compound(vec![
                    entry("max_duration_ticks", NbtSummaryValue::Int(20)),
                    entry("min_speed", NbtSummaryValue::Float(0.5f32.to_bits())),
                ]),
            ),
            entry(
                "forward_movement",
                NbtSummaryValue::Float(0.25f32.to_bits()),
            ),
            entry(
                "sound",
                NbtSummaryValue::String(ITEM_BREAK_SOUND.to_string().into()),
            ),
        ]),
    );
    let summary = round_trip_with(&written, &BuiltinResolver);
    let weapon = kinetic_weapon(&summary);
    assert_eq!(weapon.contact_cooldown_ticks, 4);
    assert_eq!(weapon.delay_ticks, 7);
    assert_eq!(
        weapon.damage_conditions,
        Some(KineticWeaponConditionSummary {
            max_duration_ticks: 20,
            min_speed_bits: 0.5f32.to_bits(),
            min_relative_speed_bits: 0.0f32.to_bits(),
        })
    );
    assert_eq!(weapon.forward_movement_bits, 0.25f32.to_bits());
    // The two non-condition defaults are still dropped, so `{}` and a
    // spelled-out `1.0` multiplier normalize to the same save form.
    assert_eq!(weapon.damage_multiplier_bits, 1.0f32.to_bits());
    assert!(weapon.knockback_conditions.is_none());

    let spelled_out = one_component(
        "minecraft:kinetic_weapon",
        NbtSummaryValue::Compound(vec![
            entry("contact_cooldown_ticks", NbtSummaryValue::Int(10)),
            entry("delay_ticks", NbtSummaryValue::Int(0)),
            entry(
                "damage_multiplier",
                NbtSummaryValue::Float(1.0f32.to_bits()),
            ),
        ]),
    );
    let summary = crate::read_item_stack(&spelled_out, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        defaults
    );
}

/// `BlocksAttacks.CODEC`: its two list/record
/// defaults are not zero values, so an empty `{}` decodes to the one
/// block-everything reduction and the `1.0`/`0.0`/`1.0` item damage.
#[test]
fn blocks_attacks_round_trips_its_reductions_and_nested_defaults() {
    let defaults = one_component("minecraft:blocks_attacks", empty("blocks_attacks"));
    let summary = round_trip_with(&defaults, &BuiltinResolver);
    let attacks = blocks_attacks(&summary);
    assert_eq!(attacks.block_delay_seconds_bits, 0.0f32.to_bits());
    assert_eq!(attacks.disable_cooldown_scale_bits, 1.0f32.to_bits());
    assert_eq!(
        attacks.damage_reductions,
        [DamageReductionSummary {
            horizontal_blocking_angle_bits: 90.0f32.to_bits(),
            damage_types: None,
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        }]
    );
    assert_eq!(
        attacks.item_damage,
        ItemDamageFunctionSummary {
            threshold_bits: 1.0f32.to_bits(),
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        }
    );
    assert!(attacks.bypassed_by.is_none());
    assert!(attacks.block_sound.is_none() && attacks.disable_sound.is_none());

    // A save that spells the defaults out normalizes to `{}` again, and a
    // reduction list that is not the default one is written whole.
    let spelled_out = one_component(
        "minecraft:blocks_attacks",
        NbtSummaryValue::Compound(vec![
            entry(
                "damage_reductions",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry(
                        "horizontal_blocking_angle",
                        NbtSummaryValue::Float(90.0f32.to_bits()),
                    ),
                    entry("base", NbtSummaryValue::Float(0.0f32.to_bits())),
                    entry("factor", NbtSummaryValue::Float(1.0f32.to_bits())),
                ])]),
            ),
            entry(
                "item_damage",
                NbtSummaryValue::Compound(vec![
                    entry("threshold", NbtSummaryValue::Float(1.0f32.to_bits())),
                    entry("base", NbtSummaryValue::Float(0.0f32.to_bits())),
                    entry("factor", NbtSummaryValue::Float(1.0f32.to_bits())),
                ]),
            ),
        ]),
    );
    let summary = crate::read_item_stack(&spelled_out, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        defaults
    );

    let written = one_component(
        "minecraft:blocks_attacks",
        NbtSummaryValue::Compound(vec![
            entry(
                "block_delay_seconds",
                NbtSummaryValue::Float(0.25f32.to_bits()),
            ),
            entry(
                "disable_cooldown_scale",
                NbtSummaryValue::Float(2.0f32.to_bits()),
            ),
            entry(
                "damage_reductions",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry(
                        "horizontal_blocking_angle",
                        NbtSummaryValue::Float(180.0f32.to_bits()),
                    ),
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:in_fire".to_string().into()),
                    ),
                    entry("base", NbtSummaryValue::Float(2.0f32.to_bits())),
                    entry("factor", NbtSummaryValue::Float(0.5f32.to_bits())),
                ])]),
            ),
            entry(
                "bypassed_by",
                NbtSummaryValue::String("#minecraft:is_fire".to_string().into()),
            ),
            entry(
                "disabled_sound",
                NbtSummaryValue::String(ITEM_BREAK_SOUND.to_string().into()),
            ),
        ]),
    );
    let resolver = publishing(&[(DAMAGE_TYPE_REGISTRY, "minecraft:in_fire", 3)]);
    let summary = round_trip_with(&written, &resolver);
    let attacks = blocks_attacks(&summary);
    assert_eq!(attacks.block_delay_seconds_bits, 0.25f32.to_bits());
    assert_eq!(attacks.disable_cooldown_scale_bits, 2.0f32.to_bits());
    assert_eq!(
        attacks.damage_reductions,
        [DamageReductionSummary {
            horizontal_blocking_angle_bits: 180.0f32.to_bits(),
            damage_types: Some(HolderSetSummary {
                registry_ids: vec![3],
                tag: None,
            }),
            base_bits: 2.0f32.to_bits(),
            factor_bits: 0.5f32.to_bits(),
        }]
    );
    assert_eq!(
        attacks.bypassed_by,
        Some(HolderSetSummary {
            registry_ids: vec![],
            tag: Some("minecraft:is_fire".to_string()),
        })
    );
    assert!(attacks.block_sound.is_none());
    assert_eq!(
        attacks
            .disable_sound
            .as_ref()
            .and_then(|sound| sound.registry_id),
        Some(
            bbb_facts::sound_event_types::vanilla_sound_event_registry_id(ITEM_BREAK_SOUND)
                .expect("the item break sound is a built-in sound event")
        )
    );
}
