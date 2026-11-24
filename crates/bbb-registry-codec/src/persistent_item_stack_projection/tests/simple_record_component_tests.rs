//! `minecraft:attack_range`, `minecraft:swing_animation`,
//! `minecraft:use_effects`, `minecraft:death_protection`,
//! `minecraft:damage_type`, `minecraft:instrument` and
//! `minecraft:jukebox_playable`.
//!
//! Their save forms differ from the wire forms in one way each: the three
//! records drop every field that holds its `optionalFieldOf` default, and the
//! three holders write an identifier where the wire carries a raw id.

use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID, VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
};
use bbb_facts::item_component_value_types::SwingAnimationTypeSummary;
use bbb_protocol::packets::{
    AttackRangeSummary, JukeboxSongSummary, SwingAnimationSummary, UseEffectsSummary,
};

use super::simple_record_components::{INSTRUMENT_REGISTRY, JUKEBOX_SONG_REGISTRY};
use super::*;

fn one_component(resource_id: &str, value: NbtSummaryValue) -> NbtSummaryValue {
    saved_stack_with_components("minecraft:stone", vec![entry(resource_id, value)])
}

fn summary_with(
    type_id: i32,
    edit: impl FnOnce(&mut DataComponentPatchSummary),
) -> ItemStackSummary {
    let mut patch = DataComponentPatchSummary {
        added_type_ids: vec![type_id],
        ..DataComponentPatchSummary::default()
    };
    edit(&mut patch);
    stack(VANILLA_ITEM_STONE_ID, 1, patch)
}

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

/// `AttackRange.CODEC`: six `optionalFieldOf`
/// floats, so an all-default range is `{}` and a range that sets every field
/// writes all six.
#[test]
fn attack_range_drops_its_defaults_and_keeps_every_other_field() {
    let defaults = one_component(
        "minecraft:attack_range",
        NbtSummaryValue::Compound(Vec::new()),
    );
    let summary = round_trip_with(&defaults, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.attack_range,
        Some(AttackRangeSummary {
            min_reach: 0.0,
            max_reach: 3.0,
            min_creative_reach: 0.0,
            max_creative_reach: 5.0,
            hitbox_margin: 0.3,
            mob_factor: 1.0,
        })
    );

    let full = one_component(
        "minecraft:attack_range",
        NbtSummaryValue::Compound(vec![
            entry("min_reach", NbtSummaryValue::Float(0.5_f32.to_bits())),
            entry("max_reach", NbtSummaryValue::Float(4.0_f32.to_bits())),
            entry(
                "min_creative_reach",
                NbtSummaryValue::Float(1.0_f32.to_bits()),
            ),
            entry(
                "max_creative_reach",
                NbtSummaryValue::Float(6.0_f32.to_bits()),
            ),
            entry("hitbox_margin", NbtSummaryValue::Float(0.5_f32.to_bits())),
            entry("mob_factor", NbtSummaryValue::Float(2.0_f32.to_bits())),
        ]),
    );
    let summary = round_trip_with(&full, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.attack_range,
        Some(AttackRangeSummary {
            min_reach: 0.5,
            max_reach: 4.0,
            min_creative_reach: 1.0,
            max_creative_reach: 6.0,
            hitbox_margin: 0.5,
            mob_factor: 2.0,
        })
    );
}

/// `SwingAnimation.CODEC`: an enum-name `type`
/// defaulting to `whack` beside `ExtraCodecs.POSITIVE_INT` `duration`
/// defaulting to `6`.
#[test]
fn swing_animation_uses_the_enum_name_and_a_positive_duration() {
    let defaults = one_component(
        "minecraft:swing_animation",
        NbtSummaryValue::Compound(Vec::new()),
    );
    let summary = round_trip_with(&defaults, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.swing_animation,
        Some(SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::Whack,
            duration: 6,
        })
    );

    let stab = one_component(
        "minecraft:swing_animation",
        NbtSummaryValue::Compound(vec![
            entry("type", NbtSummaryValue::String("stab".to_string().into())),
            entry("duration", NbtSummaryValue::Int(12)),
        ]),
    );
    let summary = round_trip_with(&stab, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.swing_animation,
        Some(SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::Stab,
            duration: 12,
        })
    );

    for (what, value) in [
        (
            "an unknown type name",
            NbtSummaryValue::Compound(vec![entry(
                "type",
                NbtSummaryValue::String("swing".to_string().into()),
            )]),
        ),
        (
            "a non-positive duration",
            NbtSummaryValue::Compound(vec![entry("duration", NbtSummaryValue::Int(0))]),
        ),
    ] {
        let saved = one_component("minecraft:swing_animation", value);
        let message = crate::read_item_stack(&saved, &BuiltinResolver)
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("minecraft:swing_animation"),
            "{what}: {message}"
        );
    }
}

/// `UseEffects.CODEC`: `can_sprint` defaults `false`,
/// `interact_vibrations` defaults `true` and `speed_multiplier` defaults `0.2`
/// inside `0..=1`.
#[test]
fn use_effects_round_trips_its_three_defaults() {
    let defaults = one_component(
        "minecraft:use_effects",
        NbtSummaryValue::Compound(Vec::new()),
    );
    let summary = round_trip_with(&defaults, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.use_effects,
        Some(UseEffectsSummary {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier: 0.2,
        })
    );

    let changed = one_component(
        "minecraft:use_effects",
        NbtSummaryValue::Compound(vec![
            entry("can_sprint", NbtSummaryValue::Byte(1)),
            entry("interact_vibrations", NbtSummaryValue::Byte(0)),
            entry(
                "speed_multiplier",
                NbtSummaryValue::Float(0.9_f32.to_bits()),
            ),
        ]),
    );
    let summary = round_trip_with(&changed, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.use_effects,
        Some(UseEffectsSummary {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier: 0.9,
        })
    );

    let out_of_range = one_component(
        "minecraft:use_effects",
        NbtSummaryValue::Compound(vec![entry(
            "speed_multiplier",
            NbtSummaryValue::Float(1.5_f32.to_bits()),
        )]),
    );
    let message = crate::read_item_stack(&out_of_range, &BuiltinResolver)
        .unwrap_err()
        .to_string();
    assert!(message.contains("outside 0..=1"), "{message}");
}

/// `DeathProtection.CODEC` is one optional
/// `death_effects` list of `ConsumeEffect`s, so no effects is `{}` and each
/// entry is the same effect compound `minecraft:consumable` writes.
#[test]
fn death_protection_carries_the_consume_effect_list() {
    let empty = one_component(
        "minecraft:death_protection",
        NbtSummaryValue::Compound(Vec::new()),
    );
    let summary = round_trip_with(&empty, &BuiltinResolver);
    assert!(summary.component_patch.death_protection_effects.is_empty());
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        empty
    );

    let totem = one_component(
        "minecraft:death_protection",
        NbtSummaryValue::Compound(vec![entry(
            "death_effects",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:apply_effects".to_string().into()),
                ),
                entry(
                    "effects",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry(
                            "id",
                            NbtSummaryValue::String("minecraft:absorption".to_string().into()),
                        ),
                        entry("amplifier", NbtSummaryValue::Byte(1)),
                        entry("duration", NbtSummaryValue::Int(100)),
                        entry("show_icon", NbtSummaryValue::Byte(1)),
                    ])]),
                ),
            ])]),
        )]),
    );
    let summary = round_trip_with(&totem, &BuiltinResolver);
    assert_eq!(summary.component_patch.death_protection_effects.len(), 1);
}

/// The three holder components write the identifier of the holder id the wire
/// carried; each of their registries is answered by the publication, and the
/// two datapack ones skip the static 26.1 re-check.
#[test]
fn the_holder_components_round_trip_through_the_publication() {
    for (resource_id, registry_id) in [
        ("minecraft:damage_type", "minecraft:damage_type"),
        ("minecraft:instrument", INSTRUMENT_REGISTRY),
        ("minecraft:jukebox_playable", JUKEBOX_SONG_REGISTRY),
    ] {
        let resolver = publishing(&[(registry_id, "example:entry", 4)]);
        let saved = one_component(
            resource_id,
            NbtSummaryValue::String("example:entry".to_string().into()),
        );
        let summary = round_trip_with(&saved, &resolver);
        assert!(
            summary.component_patch.added_type_ids.contains(
                &bbb_facts::data_component_types::vanilla_data_component_type_for_resource_id(
                    resource_id
                )
                .unwrap()
                .type_id
            ),
            "{resource_id}"
        );

        // Without the publication the decode direction names the registry.
        let error = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap_err();
        assert!(
            error.to_string().contains(registry_id),
            "{resource_id}: {error}"
        );
    }
}

/// `Instrument.CODEC` is a `RegistryFileCodec`, so an
/// inline record saves with its owned description; `JukeboxPlayable` keeps a
/// named-only save form.
#[test]
fn inline_instruments_round_trip_and_inline_songs_are_refused_by_name() {
    let resolver = publishing(&[("minecraft:dialog", "example:dialog", 3)]);
    let description = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::String("Horn".to_string().into())),
        entry(
            "hover_event",
            NbtSummaryValue::Compound(vec![
                entry(
                    "action",
                    NbtSummaryValue::String("show_item".to_string().into()),
                ),
                entry(
                    "id",
                    NbtSummaryValue::String("minecraft:goat_horn".to_string().into()),
                ),
            ]),
        ),
        entry(
            "click_event",
            NbtSummaryValue::Compound(vec![
                entry(
                    "action",
                    NbtSummaryValue::String("show_dialog".to_string().into()),
                ),
                entry(
                    "dialog",
                    NbtSummaryValue::String("example:dialog".to_string().into()),
                ),
            ]),
        ),
    ]);
    let saved = one_component(
        "minecraft:instrument",
        NbtSummaryValue::Compound(vec![
            entry(
                "sound_event",
                NbtSummaryValue::String("minecraft:item.goat_horn.sound.0".to_string().into()),
            ),
            entry("use_duration", NbtSummaryValue::Float(7.0_f32.to_bits())),
            entry("range", NbtSummaryValue::Float(256.0_f32.to_bits())),
            entry("description", description),
        ]),
    );
    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    let patch = &summary.component_patch;
    let instrument = patch.instrument.direct.as_deref().unwrap();
    assert_eq!(patch.instrument.registry_id, None);
    assert_eq!(instrument.use_duration_bits, 7.0_f32.to_bits());
    assert_eq!(instrument.range_bits, 256.0_f32.to_bits());
    assert_eq!(instrument.description, "Horn");
    let written = item_stack_summary_to_persistent_nbt_with_resolver(&summary, &resolver).unwrap();
    let reread = crate::read_item_stack(&written.clone(), &resolver).unwrap();
    assert_eq!(reread, summary);
    assert_eq!(
        item_stack_summary_to_persistent_nbt_with_resolver(&reread, &resolver).unwrap(),
        written
    );

    let sound = instrument.sound_event.clone();
    let non_positive = summary_with(VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID, |patch| {
        let mut direct = instrument.clone();
        direct.use_duration_bits = 0.0_f32.to_bits();
        patch.instrument.direct = Some(Box::new(direct));
    });
    let message = item_stack_summary_to_persistent_nbt_with_resolver(&non_positive, &resolver)
        .unwrap_err()
        .to_string();
    assert!(message.contains("use_duration"), "{message}");

    let song = summary_with(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, |patch| {
        patch.jukebox_direct_song = Some(Box::new(JukeboxSongSummary {
            sound_event: sound,
            description: "a song".to_string(),
            description_styled: None,
            description_nbt: None,
            length_in_seconds_bits: 10.0_f32.to_bits(),
            comparator_output: 1,
        }));
    });
    let message = item_stack_summary_to_persistent_nbt(&song)
        .unwrap_err()
        .to_string();
    assert!(message.contains("inline jukebox song"), "{message}");
}

/// Every one of these components refuses on the resolver-free entry point when
/// its validator needs a registry answer, and the two that name no registry
/// still cross it -- the guard is per component, not per module.
#[test]
fn the_resolver_free_entry_point_is_per_component() {
    for (resource_id, value) in [
        (
            "minecraft:attack_range",
            NbtSummaryValue::Compound(Vec::new()),
        ),
        (
            "minecraft:swing_animation",
            NbtSummaryValue::Compound(Vec::new()),
        ),
        (
            "minecraft:use_effects",
            NbtSummaryValue::Compound(Vec::new()),
        ),
        (
            "minecraft:death_protection",
            NbtSummaryValue::Compound(Vec::new()),
        ),
    ] {
        let saved = one_component(resource_id, value);
        assert!(
            crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_ok(),
            "{resource_id} should need no publication"
        );
    }
    for resource_id in [
        "minecraft:damage_type",
        "minecraft:instrument",
        "minecraft:jukebox_playable",
    ] {
        let saved = one_component(
            resource_id,
            NbtSummaryValue::String("minecraft:in_fire".to_string().into()),
        );
        assert!(
            crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_err(),
            "{resource_id} should refuse resolver-free"
        );
    }
}
