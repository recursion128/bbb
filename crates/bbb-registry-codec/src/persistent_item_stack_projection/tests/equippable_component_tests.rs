//! `minecraft:equippable`: `Equippable.CODEC`, whose
//! save shape differs from its network shape in three ways -- the slot is an
//! enum name, the two sounds default to a *named* holder that the save form
//! omits, and both `asset_id` and `camera_overlay` write a resource location
//! where the network writes an optional identifier.

use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
use bbb_facts::entity_types::{VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_PLAYER_ID};
use bbb_facts::equipment_slots::{
    VANILLA_EQUIPMENT_SLOT_BODY_ID, VANILLA_EQUIPMENT_SLOT_CHEST_ID,
    VANILLA_EQUIPMENT_SLOT_MAINHAND_ID,
};
use bbb_facts::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::packets::{EquippableSummary, HolderSetSummary, SoundEventSummary};

use super::*;

const DEFAULT_EQUIP_SOUND: &str = "minecraft:item.armor.equip_generic";
const DEFAULT_SHEARING_SOUND: &str = "minecraft:item.shears.snip";
const SHEARING_SOUND: &str = "minecraft:entity.sheep.shear";

fn named_sound(resource_id: &str) -> SoundEventSummary {
    SoundEventSummary {
        registry_id: vanilla_sound_event_registry_id(resource_id),
        sound_id: None,
        fixed_range_bits: None,
    }
}

/// The summary a full `Equippable.CODEC` save form decodes to. Everything the
/// save form omits holds its codec default.
fn equippable_summary(slot_id: i32) -> EquippableSummary {
    EquippableSummary {
        slot_id,
        equip_sound: named_sound(DEFAULT_EQUIP_SOUND),
        asset_id: None,
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: named_sound(DEFAULT_SHEARING_SOUND),
    }
}

fn summary_with(equippable: EquippableSummary) -> ItemStackSummary {
    stack(
        VANILLA_ITEM_DIAMOND_PICKAXE_ID,
        1,
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID],
            equippable: Some(equippable),
            ..DataComponentPatchSummary::default()
        },
    )
}

fn one_component(value: NbtSummaryValue) -> NbtSummaryValue {
    saved_stack_with_components(
        "minecraft:diamond_pickaxe",
        vec![entry("minecraft:equippable", value)],
    )
}

/// NBT -> summary -> NBT, plus the decode/encode fixpoint, through the
/// publication-free entry point: every holder this component writes is a
/// built-in catalog entry.
fn round_trip(saved: &NbtSummaryValue) -> ItemStackSummary {
    let summary = crate::read_item_stack(saved, &BuiltinResolver)
        .expect("the built-in sound-event and entity-type catalogs name every holder here");
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved.clone()
    );
    summary
}

/// `EquipmentSlot.CODEC` is `StringRepresentable.fromEnum`
/// so the slot crosses as its serialized name, and
/// `optionalFieldOf(name, default)` drops every field that holds its default
#[test]
fn a_minimal_save_form_carries_only_the_slot() {
    let saved = one_component(NbtSummaryValue::Compound(vec![entry(
        "slot",
        NbtSummaryValue::String("chest".to_string().into()),
    )]));
    let summary = round_trip(&saved);
    assert_eq!(
        summary.component_patch.equippable,
        Some(equippable_summary(VANILLA_EQUIPMENT_SLOT_CHEST_ID))
    );
    // The save form holds one field, so every other default really was omitted.
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved.clone()
    );
}

/// Every field of `Equippable` at once, including a direct (unnamed) sound that
/// must *not* be folded onto the named default: `optionalFieldOf` compares
/// `Holder` identity, and a `Holder.Direct` is never the
/// registry's own reference.
#[test]
fn a_full_record_round_trips_every_field() {
    let saved = one_component(NbtSummaryValue::Compound(vec![
        entry("slot", NbtSummaryValue::String("body".to_string().into())),
        entry(
            "equip_sound",
            NbtSummaryValue::Compound(vec![entry(
                "sound_id",
                NbtSummaryValue::String("example:custom".to_string().into()),
            )]),
        ),
        entry(
            "asset_id",
            NbtSummaryValue::String("minecraft:elytra".to_string().into()),
        ),
        entry(
            "camera_overlay",
            NbtSummaryValue::String("minecraft:misc/pumpkinblur".to_string().into()),
        ),
        entry(
            "allowed_entities",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("minecraft:llama".to_string().into()),
                NbtSummaryValue::String("minecraft:player".to_string().into()),
            ]),
        ),
        entry("dispensable", NbtSummaryValue::Byte(0)),
        entry("swappable", NbtSummaryValue::Byte(0)),
        entry("damage_on_hurt", NbtSummaryValue::Byte(0)),
        entry("equip_on_interact", NbtSummaryValue::Byte(1)),
        entry("can_be_sheared", NbtSummaryValue::Byte(1)),
        entry(
            "shearing_sound",
            NbtSummaryValue::String(SHEARING_SOUND.to_string().into()),
        ),
    ]));
    let summary = round_trip(&saved);
    assert_eq!(
        summary.component_patch.equippable,
        Some(EquippableSummary {
            slot_id: VANILLA_EQUIPMENT_SLOT_BODY_ID,
            equip_sound: SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:custom".to_string()),
                fixed_range_bits: None,
            },
            asset_id: Some("minecraft:elytra".to_string()),
            camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
            allowed_entities: Some(HolderSetSummary {
                registry_ids: vec![VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_PLAYER_ID],
                tag: None,
            }),
            dispensable: false,
            swappable: false,
            damage_on_hurt: false,
            equip_on_interact: true,
            can_be_sheared: true,
            shearing_sound: named_sound(SHEARING_SOUND),
        })
    );
}

/// The encode direction drops a named sound exactly while it names the codec's
/// default entry, and keeps every other holder spelling.
#[test]
fn only_the_codec_default_sounds_are_omitted() {
    for (sound, omitted) in [
        (named_sound(DEFAULT_EQUIP_SOUND), true),
        (named_sound("minecraft:item.armor.equip_iron"), false),
        (
            SoundEventSummary {
                registry_id: None,
                sound_id: Some(DEFAULT_EQUIP_SOUND.to_string()),
                fixed_range_bits: None,
            },
            false,
        ),
    ] {
        let mut equippable = equippable_summary(VANILLA_EQUIPMENT_SLOT_CHEST_ID);
        equippable.equip_sound = sound;
        let nbt = item_stack_summary_to_persistent_nbt(&summary_with(equippable)).unwrap();
        let fields = match projected_components(&nbt).unwrap() {
            NbtSummaryValue::Compound(components) => match &components[0].value {
                NbtSummaryValue::Compound(fields) => fields.clone(),
                other => panic!("equippable is not a compound: {other:?}"),
            },
            other => panic!("components is not a compound: {other:?}"),
        };
        assert_eq!(
            fields.iter().any(|field| field.name == "equip_sound"),
            !omitted
        );
    }
}

/// `allowed_entities` is `RegistryCodecs.homogeneousList(Registries.ENTITY_TYPE)`
/// so it compacts to a bare id for one holder and
/// writes `"#tag"` for a declared tag.
#[test]
fn allowed_entities_use_the_compact_holder_set_form() {
    let cases = [
        (
            NbtSummaryValue::String("minecraft:llama".to_string().into()),
            HolderSetSummary {
                registry_ids: vec![VANILLA_ENTITY_TYPE_LLAMA_ID],
                tag: None,
            },
        ),
        (
            NbtSummaryValue::String("#minecraft:can_equip_saddle".to_string().into()),
            HolderSetSummary {
                registry_ids: Vec::new(),
                tag: Some("minecraft:can_equip_saddle".to_string()),
            },
        ),
        (
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("minecraft:llama".to_string().into()),
                NbtSummaryValue::String("minecraft:player".to_string().into()),
            ]),
            HolderSetSummary {
                registry_ids: vec![VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_PLAYER_ID],
                tag: None,
            },
        ),
    ];
    for (value, expected) in cases {
        let saved = one_component(NbtSummaryValue::Compound(vec![
            entry("slot", NbtSummaryValue::String("saddle".to_string().into())),
            entry("allowed_entities", value),
        ]));
        let summary = round_trip(&saved);
        assert_eq!(
            summary
                .component_patch
                .equippable
                .as_ref()
                .unwrap()
                .allowed_entities,
            Some(expected)
        );
    }
}

/// Equipment sounds and entity types are built-in registries, so a record
/// crosses in both directions without a publication.
#[test]
fn built_in_holders_cross_without_a_publication() {
    let bare = one_component(NbtSummaryValue::Compound(vec![entry(
        "slot",
        NbtSummaryValue::String("mainhand".to_string().into()),
    )]));
    let summary = crate::read_item_stack(&bare, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        summary.component_patch.equippable.as_ref().unwrap().slot_id,
        VANILLA_EQUIPMENT_SLOT_MAINHAND_ID
    );
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        bare
    );

    for (what, value) in [
        (
            "a named equip_sound",
            NbtSummaryValue::String(DEFAULT_EQUIP_SOUND.to_string().into()),
        ),
        (
            "an allowed_entities holder set",
            NbtSummaryValue::String("minecraft:llama".to_string().into()),
        ),
    ] {
        let field = if what.contains("sound") {
            "equip_sound"
        } else {
            "allowed_entities"
        };
        let saved = one_component(NbtSummaryValue::Compound(vec![
            entry("slot", NbtSummaryValue::String("chest".to_string().into())),
            entry(field, value),
        ]));
        assert!(
            crate::read_item_stack(&saved, &BuiltinResolver).is_ok(),
            "{what} should cross with a publication"
        );
    }
}

/// An explicit default is not preserved: `optionalFieldOf(name, default)`
/// compares against the default while encoding (`Equippable`),
/// so the save form for a record that spells its default out is the
/// one that omits it.
#[test]
fn an_explicit_default_is_normalized_away() {
    let explicit = one_component(NbtSummaryValue::Compound(vec![
        entry("slot", NbtSummaryValue::String("chest".to_string().into())),
        entry(
            "equip_sound",
            NbtSummaryValue::String(DEFAULT_EQUIP_SOUND.to_string().into()),
        ),
        entry("dispensable", NbtSummaryValue::Byte(1)),
        entry(
            "shearing_sound",
            NbtSummaryValue::String(DEFAULT_SHEARING_SOUND.to_string().into()),
        ),
    ]));
    let normalized = one_component(NbtSummaryValue::Compound(vec![entry(
        "slot",
        NbtSummaryValue::String("chest".to_string().into()),
    )]));
    let summary = crate::read_item_stack(&explicit, &BuiltinResolver).unwrap();
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        normalized
    );
}
