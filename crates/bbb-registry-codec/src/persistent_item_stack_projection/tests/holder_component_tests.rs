//! `minecraft:break_sound`, `minecraft:provides_trim_material`,
//! `minecraft:repairable`, `minecraft:damage_resistant`,
//! `minecraft:piercing_weapon`, `minecraft:lock` and
//! `minecraft:lodestone_tracker`: the batch whose persistent codecs name a
//! registry holder, a holder set, an `ItemPredicate` or a `GlobalPos`.
//!
//! Every round trip is NBT -> summary -> NBT through the published holder ids,
//! because each of these save forms writes an element's *identifier* rather
//! than the wire id it arrived as.

use bbb_facts::item_types::{VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_STONE_ID};
use bbb_facts::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::packets::{LodestoneTargetSummary, SoundEventSummary};

use super::holder::DAMAGE_TYPE_REGISTRY;
use super::*;

const ITEM_BREAK_SOUND: &str = "minecraft:entity.item.break";
const FIRE_TAG: &str = "minecraft:is_fire";

fn one_component(name: &str, value: NbtSummaryValue) -> NbtSummaryValue {
    saved_stack_with_components("minecraft:stone", vec![entry(name, value)])
}

fn round_trip(
    saved: &NbtSummaryValue,
    resolver: &PublishingResolver,
) -> bbb_protocol::packets::DataComponentPatchSummary {
    let summary = crate::read_item_stack(saved, resolver)
        .expect("saved component projects to the wire shape");
    assert_eq!(
        item_stack_summary_to_persistent_nbt_with_resolver(&summary, resolver).unwrap(),
        saved.clone()
    );
    summary.component_patch.into_inner()
}

/// `minecraft:break_sound` is `persistent(SoundEvent.CODEC)`
/// and `SoundEvent.CODEC` is a
/// `RegistryFileCodec`, so a named holder saves as its
/// identifier.
#[test]
fn break_sound_round_trips_through_its_named_holder() {
    let resolver = publishing(&[]);
    let saved = one_component(
        "minecraft:break_sound",
        NbtSummaryValue::String(ITEM_BREAK_SOUND.to_string().into()),
    );
    let patch = round_trip(&saved, &resolver);
    assert_eq!(
        patch.break_sound,
        Some(SoundEventSummary {
            registry_id: vanilla_sound_event_registry_id(ITEM_BREAK_SOUND),
            sound_id: None,
            fixed_range_bits: None,
        })
    );

    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved.clone()
    );
}

/// The direct alternative of the same codec writes `{sound_id, range?}`
/// which is what a save holds for a sound no
/// registry names.
#[test]
fn direct_break_sounds_round_trip_with_and_without_a_range() {
    let resolver = publishing(&[]);
    for value in [
        NbtSummaryValue::Compound(vec![entry(
            "sound_id",
            NbtSummaryValue::String("example:custom".to_string().into()),
        )]),
        NbtSummaryValue::Compound(vec![
            entry(
                "sound_id",
                NbtSummaryValue::String("example:custom".to_string().into()),
            ),
            entry("range", NbtSummaryValue::Float(8.0_f32.to_bits())),
        ]),
    ] {
        let saved = one_component("minecraft:break_sound", value);
        let patch = round_trip(&saved, &resolver);
        assert_eq!(
            patch
                .break_sound
                .as_ref()
                .and_then(|sound| sound.sound_id.as_deref()),
            Some("example:custom")
        );
    }
}

/// `ProvidingWeapon.CODEC`'s counterpart: `minecraft:provides_trim_material` is
/// `persistent(TrimMaterial.CODEC)`, another
/// `RegistryFileCodec` over a datapack registry.
#[test]
fn provides_trim_material_round_trips_through_the_published_material() {
    let resolver = publishing(&[("minecraft:trim_material", "minecraft:copper", 0)]);
    let saved = one_component(
        "minecraft:provides_trim_material",
        NbtSummaryValue::String("minecraft:copper".to_string().into()),
    );
    let patch = round_trip(&saved, &resolver);
    assert_eq!(patch.provides_trim_material_id, Some(0));
    assert!(patch.provides_trim_material_direct.is_none());

    // The publication is the only source of the identifier in either
    // direction: `minecraft:trim_material` is a datapack registry.
    assert!(crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_err());
    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    assert!(item_stack_summary_to_persistent_nbt(&summary).is_err());
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

/// A description whose hover and click payloads need the item stack and dialog
/// owners rather than plain text.
fn owned_description(text: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("text", string(text)),
        entry(
            "hover_event",
            NbtSummaryValue::Compound(vec![
                entry("action", string("show_item")),
                entry("id", string("minecraft:stone")),
                entry("count", NbtSummaryValue::Int(2)),
            ]),
        ),
        entry(
            "click_event",
            NbtSummaryValue::Compound(vec![
                entry("action", string("show_dialog")),
                entry("dialog", string("example:dialog")),
            ]),
        ),
    ])
}

fn written_again(
    summary: &ItemStackSummary,
    resolver: &PublishingResolver,
) -> (NbtSummaryValue, ItemStackSummary) {
    let written = item_stack_summary_to_persistent_nbt_with_resolver(summary, resolver).unwrap();
    let saved = written.clone();
    let reread = crate::read_item_stack(&saved, resolver).unwrap();
    (written, reread)
}

/// An inline material is its `DIRECT_CODEC` record,
/// so it projects and saves with its owned description instead of failing.
#[test]
fn inline_provides_trim_material_round_trips_an_owned_description() {
    let resolver = publishing(&[("minecraft:dialog", "example:dialog", 3)]);
    let saved = one_component(
        "minecraft:provides_trim_material",
        NbtSummaryValue::Compound(vec![
            entry("asset_name", string("copper")),
            entry(
                "override_armor_assets",
                NbtSummaryValue::Compound(vec![entry("minecraft:iron", string("copper_darker"))]),
            ),
            entry("description", owned_description("Copper")),
        ]),
    );
    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    let material = summary
        .component_patch
        .provides_trim_material_direct
        .as_deref()
        .unwrap();
    assert_eq!(summary.component_patch.provides_trim_material_id, None);
    assert_eq!(material.asset_name, "copper");
    assert_eq!(
        material.override_armor_assets["minecraft:iron"],
        "copper_darker"
    );
    assert_eq!(material.description, "Copper");
    let (written, reread) = written_again(&summary, &resolver);
    assert_eq!(reread, summary);
    assert_eq!(written_again(&reread, &resolver).0, written);

    let mut patch = summary.component_patch.clone();
    patch.provides_trim_material_id = Some(0);
    assert!(item_stack_summary_to_persistent_nbt_with_resolver(
        &stack(VANILLA_ITEM_STONE_ID, 1, patch.into_inner()),
        &resolver
    )
    .is_err());
    let mut patch = summary.component_patch;
    patch
        .provides_trim_material_direct
        .as_mut()
        .unwrap()
        .description_nbt = None;
    let error = item_stack_summary_to_persistent_nbt_with_resolver(
        &stack(VANILLA_ITEM_STONE_ID, 1, patch.into_inner()),
        &resolver,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("lacks its Component NBT"),
        "{error}"
    );
}

/// `ArmorTrim` saves both holders through `RegistryFileCodec`
/// so either may be an inline record.
#[test]
fn inline_trim_material_and_pattern_round_trip_owned_descriptions() {
    let resolver = publishing(&[
        ("minecraft:dialog", "example:dialog", 3),
        ("minecraft:trim_pattern", "minecraft:bolt", 1),
    ]);
    let material = NbtSummaryValue::Compound(vec![
        entry("asset_name", string("copper")),
        entry("description", owned_description("Copper")),
    ]);
    let pattern = NbtSummaryValue::Compound(vec![
        entry("asset_id", string("example:wave")),
        entry("description", owned_description("Wave")),
        entry("decal", NbtSummaryValue::Byte(1)),
    ]);
    for (material, pattern) in [
        (material.clone(), pattern.clone()),
        (material.clone(), string("minecraft:bolt")),
    ] {
        let saved = one_component(
            "minecraft:trim",
            NbtSummaryValue::Compound(vec![
                entry("material", material),
                entry("pattern", pattern.clone()),
            ]),
        );
        let summary = crate::read_item_stack(&saved, &resolver).unwrap();
        let patch = &summary.component_patch;
        assert_eq!(
            patch
                .armor_trim_material_direct
                .as_ref()
                .unwrap()
                .asset_name,
            "copper"
        );
        if let NbtSummaryValue::String(_) = pattern {
            assert_eq!(patch.armor_trim_pattern_id, Some(1));
        } else {
            let direct = patch.armor_trim_pattern_direct.as_ref().unwrap();
            assert_eq!(
                (direct.asset_id.as_str(), direct.decal),
                ("example:wave", true)
            );
        }
        let (written, reread) = written_again(&summary, &resolver);
        assert_eq!(reread, summary);
        assert_eq!(written_again(&reread, &resolver).0, written);
    }
}

/// `minecraft:repairable` is `persistent(Repairable.CODEC)`
/// one required `items` holder set over the
/// built-in `Registries.ITEM`.
///
/// `HolderSetCodec` writes a declared tag as `"#id"` and a
/// direct set through `ExtraCodecs.compactListCodec`, i.e. the bare element for
/// one holder.
#[test]
fn repairable_round_trips_every_holder_set_shape() {
    let resolver = publishing(&[]);
    for items in [
        NbtSummaryValue::String("minecraft:stone".to_string().into()),
        NbtSummaryValue::List(vec![
            NbtSummaryValue::String("minecraft:diamond".to_string().into()),
            NbtSummaryValue::String("minecraft:stone".to_string().into()),
        ]),
        NbtSummaryValue::String("#minecraft:stone_tool_materials".to_string().into()),
    ] {
        let saved = one_component(
            "minecraft:repairable",
            NbtSummaryValue::Compound(vec![entry("items", items)]),
        );
        let patch = round_trip(&saved, &resolver);
        assert!(
            patch.repairable_item_tag.is_some() || !patch.repairable_item_registry_ids.is_empty()
        );
    }

    let single = crate::read_item_stack(
        &one_component(
            "minecraft:repairable",
            NbtSummaryValue::Compound(vec![entry(
                "items",
                NbtSummaryValue::String("minecraft:stone".to_string().into()),
            )]),
        ),
        &resolver,
    )
    .unwrap();
    assert_eq!(
        single.component_patch.repairable_item_registry_ids,
        [VANILLA_ITEM_STONE_ID]
    );

    let listed = crate::read_item_stack(
        &one_component(
            "minecraft:repairable",
            NbtSummaryValue::Compound(vec![entry(
                "items",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("minecraft:diamond".to_string().into()),
                    NbtSummaryValue::String("minecraft:stone".to_string().into()),
                ]),
            )]),
        ),
        &resolver,
    )
    .unwrap();
    assert_eq!(
        listed.component_patch.repairable_item_registry_ids,
        [VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_STONE_ID]
    );

    assert_eq!(
        item_stack_summary_to_persistent_nbt(&listed).unwrap(),
        one_component(
            "minecraft:repairable",
            NbtSummaryValue::Compound(vec![entry(
                "items",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("minecraft:diamond".to_string().into()),
                    NbtSummaryValue::String("minecraft:stone".to_string().into()),
                ]),
            )]),
        )
    );

    let empty = one_component(
        "minecraft:repairable",
        NbtSummaryValue::Compound(vec![entry("items", NbtSummaryValue::List(vec![]))]),
    );
    let patch = round_trip(&empty, &resolver);
    assert!(patch.repairable_item_registry_ids.is_empty());
    assert!(patch.repairable_item_tag.is_none());
    assert!(patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID));
    assert_empty_holder_set_wire(
        stack(VANILLA_ITEM_STONE_ID, 1, patch),
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
    );
}

/// `minecraft:damage_resistant` is `persistent(DamageResistant.CODEC)`
/// the same holder-set shape over the
/// `minecraft:damage_type` datapack registry.
#[test]
fn damage_resistant_round_trips_through_the_published_damage_types() {
    let resolver = publishing(&[
        (DAMAGE_TYPE_REGISTRY, "minecraft:in_fire", 0),
        (DAMAGE_TYPE_REGISTRY, "minecraft:generic", 1),
    ]);
    for types in [
        NbtSummaryValue::String("minecraft:in_fire".to_string().into()),
        NbtSummaryValue::List(vec![
            NbtSummaryValue::String("minecraft:in_fire".to_string().into()),
            NbtSummaryValue::String("minecraft:generic".to_string().into()),
        ]),
        NbtSummaryValue::String(format!("#{FIRE_TAG}").into()),
    ] {
        let saved = one_component(
            "minecraft:damage_resistant",
            NbtSummaryValue::Compound(vec![entry("types", types)]),
        );
        round_trip(&saved, &resolver);
    }

    let listed = crate::read_item_stack(
        &one_component(
            "minecraft:damage_resistant",
            NbtSummaryValue::Compound(vec![entry(
                "types",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("minecraft:in_fire".to_string().into()),
                    NbtSummaryValue::String("minecraft:generic".to_string().into()),
                ]),
            )]),
        ),
        &resolver,
    )
    .unwrap();
    assert_eq!(
        listed.component_patch.damage_resistant_type_registry_ids,
        [0, 1]
    );
    assert_eq!(
        item_stack_summary_to_persistent_nbt_with_resolver(&listed, &resolver).unwrap(),
        one_component(
            "minecraft:damage_resistant",
            NbtSummaryValue::Compound(vec![entry(
                "types",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("minecraft:in_fire".to_string().into()),
                    NbtSummaryValue::String("minecraft:generic".to_string().into()),
                ]),
            )]),
        )
    );

    let empty = one_component(
        "minecraft:damage_resistant",
        NbtSummaryValue::Compound(vec![entry("types", NbtSummaryValue::List(vec![]))]),
    );
    let patch = round_trip(&empty, &resolver);
    assert!(patch.damage_resistant_type_registry_ids.is_empty());
    assert!(patch.damage_resistant_type_tag.is_none());
    assert!(patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID));
    assert_empty_holder_set_wire(
        stack(VANILLA_ITEM_STONE_ID, 1, patch),
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
    );
}

fn assert_empty_holder_set_wire(summary: ItemStackSummary, type_id: i32) {
    let (packet_id, bytes) =
        bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
            &bbb_protocol::packets::inventory::ContainerSetSlot {
                container_id: 0,
                state_id: 1,
                slot: 0,
                item: summary,
            },
        )
        .unwrap();
    let bbb_protocol::packets::play_clientbound::PlayClientbound::ContainerSetSlot(slot) =
        bbb_protocol::packets::play_clientbound::decode_play_clientbound(packet_id, &bytes)
            .unwrap()
    else {
        panic!("expected ContainerSetSlot");
    };
    assert!(slot.item.component_patch.added_type_ids.contains(&type_id));
    match type_id {
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => {
            assert!(slot
                .item
                .component_patch
                .repairable_item_registry_ids
                .is_empty());
            assert!(slot.item.component_patch.repairable_item_tag.is_none());
        }
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => {
            assert!(slot
                .item
                .component_patch
                .damage_resistant_type_registry_ids
                .is_empty());
            assert!(slot
                .item
                .component_patch
                .damage_resistant_type_tag
                .is_none());
        }
        _ => unreachable!(),
    }
}

/// `minecraft:piercing_weapon` is `persistent(PiercingWeapon.CODEC)`
/// four `optionalFieldOf` fields whose defaults
/// a save omits.
#[test]
fn piercing_weapon_round_trips_and_drops_codec_defaults() {
    let resolver = publishing(&[]);
    let empty = one_component(
        "minecraft:piercing_weapon",
        NbtSummaryValue::Compound(Vec::new()),
    );
    let patch = round_trip(&empty, &resolver);
    let weapon = patch.piercing_weapon.as_ref().unwrap();
    assert!(weapon.deals_knockback);
    assert!(!weapon.dismounts);
    assert!(weapon.sound.is_none() && weapon.hit_sound.is_none());

    let written = one_component(
        "minecraft:piercing_weapon",
        NbtSummaryValue::Compound(vec![
            entry("deals_knockback", NbtSummaryValue::Byte(0)),
            entry("dismounts", NbtSummaryValue::Byte(1)),
            entry(
                "sound",
                NbtSummaryValue::String(ITEM_BREAK_SOUND.to_string().into()),
            ),
            entry(
                "hit_sound",
                NbtSummaryValue::Compound(vec![entry(
                    "sound_id",
                    NbtSummaryValue::String("example:hit".to_string().into()),
                )]),
            ),
        ]),
    );
    let patch = round_trip(&written, &resolver);
    let weapon = patch.piercing_weapon.as_ref().unwrap();
    assert!(!weapon.deals_knockback);
    assert!(weapon.dismounts);

    // A save that spells a default out is normalized away while encoding, which
    // is `optionalFieldOf` dropping a field equal to its default.
    let spelled_out = one_component(
        "minecraft:piercing_weapon",
        NbtSummaryValue::Compound(vec![
            entry("deals_knockback", NbtSummaryValue::Byte(1)),
            entry("dismounts", NbtSummaryValue::Byte(0)),
        ]),
    );
    let summary = crate::read_item_stack(&spelled_out, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        empty
    );
}

/// `minecraft:lock` is `persistent(LockCode.CODEC)` only
/// and `LockCode.CODEC` is
/// `ItemPredicate.CODEC.xmap`, so the save form is the
/// `ItemPredicate` map itself.
#[test]
fn lock_round_trips_through_the_item_predicate_codec() {
    let resolver = publishing(&[]);
    let saved = one_component(
        "minecraft:lock",
        NbtSummaryValue::Compound(vec![
            entry(
                "items",
                NbtSummaryValue::String("minecraft:stone".to_string().into()),
            ),
            entry("count", NbtSummaryValue::Int(3)),
        ]),
    );
    let patch = round_trip(&saved, &resolver);
    let predicate = patch.lock_predicate.as_ref().unwrap();
    assert_eq!(predicate.count_min, Some(3));
    assert_eq!(predicate.count_max, Some(3));
    assert_eq!(
        predicate.items.as_ref().unwrap().registry_ids,
        [VANILLA_ITEM_STONE_ID]
    );

    // An unknown item is refused rather than written out.
    let unknown = one_component(
        "minecraft:lock",
        NbtSummaryValue::Compound(vec![entry(
            "items",
            NbtSummaryValue::String("example:missing".to_string().into()),
        )]),
    );
    assert!(crate::read_item_stack(&unknown, &resolver).is_err());
}

/// `minecraft:lodestone_tracker` is `persistent(LodestoneTracker.CODEC)`
/// an optional `GlobalPos` and a `tracked`
/// flag defaulting to `true`.
///
/// `GlobalPos.CODEC` is `{dimension, pos}` whose
/// position `NbtOps.createIntList` writes as an `IntArrayTag`
/// unlike the wire's packed `long`.
#[test]
fn lodestone_tracker_round_trips_its_global_pos() {
    let resolver = publishing(&[]);
    let saved = one_component(
        "minecraft:lodestone_tracker",
        NbtSummaryValue::Compound(vec![
            entry(
                "target",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "dimension",
                        NbtSummaryValue::String("minecraft:overworld".to_string().into()),
                    ),
                    entry("pos", NbtSummaryValue::IntArray(vec![100, 64, -200])),
                ]),
            ),
            entry("tracked", NbtSummaryValue::Byte(0)),
        ]),
    );
    let patch = round_trip(&saved, &resolver);
    assert_eq!(
        patch.lodestone_target,
        Some(LodestoneTargetSummary {
            dimension: "minecraft:overworld".to_string(),
            pos: bbb_protocol::packets::chunks::BlockPos {
                x: 100,
                y: 64,
                z: -200,
            },
        })
    );
    assert_eq!(patch.lodestone_tracked, Some(false));

    // `tracked` defaults to true, so spelling it out is normalized away.
    let tracked = one_component(
        "minecraft:lodestone_tracker",
        NbtSummaryValue::Compound(vec![entry("tracked", NbtSummaryValue::Byte(1))]),
    );
    let summary = crate::read_item_stack(&tracked, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(summary.component_patch.lodestone_tracked, Some(true));
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        one_component(
            "minecraft:lodestone_tracker",
            NbtSummaryValue::Compound(Vec::new())
        )
    );

    // A position that is not three ints is not the int stream the codec reads.
    let short = one_component(
        "minecraft:lodestone_tracker",
        NbtSummaryValue::Compound(vec![entry(
            "target",
            NbtSummaryValue::Compound(vec![
                entry(
                    "dimension",
                    NbtSummaryValue::String("minecraft:overworld".to_string().into()),
                ),
                entry("pos", NbtSummaryValue::IntArray(vec![100, 64])),
            ]),
        )]),
    );
    let error = crate::read_item_stack(&short, &crate::UnavailableHolderResolver).unwrap_err();
    assert!(
        error.to_string().contains("minecraft:lodestone_tracker"),
        "{error}"
    );
}
