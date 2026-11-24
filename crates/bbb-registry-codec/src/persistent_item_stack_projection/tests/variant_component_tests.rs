//! The `Holder<...>` variant components and `minecraft:painting/variant`.
//!
//! Every one persists through `RegistryFixedCodec`, which writes the named
//! element's identifier, while the network carries the session-published holder
//! id. Villager type uses its fixed bootstrap catalog; the other variants use
//! a publication and refuse holders it cannot name.

use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
};
use bbb_protocol::packets::PaintingVariantSummary;

use super::variant_components::variant_registry;
use super::*;

/// The registry ids the fourteen variant components belong to, taken from the
/// generated component table rather than repeated by hand.
const VARIANT_RESOURCE_IDS: &[&str] = &[
    "minecraft:villager/variant",
    "minecraft:wolf/variant",
    "minecraft:wolf/sound_variant",
    "minecraft:pig/variant",
    "minecraft:pig/sound_variant",
    "minecraft:cow/variant",
    "minecraft:cow/sound_variant",
    "minecraft:chicken/variant",
    "minecraft:chicken/sound_variant",
    "minecraft:zombie_nautilus/variant",
    "minecraft:frog/variant",
    "minecraft:painting/variant",
    "minecraft:cat/variant",
    "minecraft:cat/sound_variant",
];

fn variant_type_ids() -> Vec<(i32, &'static str, &'static str)> {
    VARIANT_RESOURCE_IDS
        .iter()
        .map(|resource_id| {
            let component =
                bbb_facts::data_component_types::vanilla_data_component_type_for_resource_id(
                    resource_id,
                )
                .expect("variant components are catalogued");
            (
                component.type_id,
                component.resource_id,
                variant_registry(component.type_id).expect("variant components own a registry"),
            )
        })
        .collect()
}

/// Each component's save form is the identifier of the holder id it arrived as,
/// and the projected summary is wire-encodable.
#[test]
fn every_variant_round_trips_through_its_holder_catalog() {
    for (type_id, resource_id, registry_id) in variant_type_ids() {
        let named = if type_id == VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID {
            "minecraft:savanna"
        } else {
            "example:variant"
        };
        let resolver = publishing(&[(registry_id, "example:variant", 3)]);
        let saved = saved_stack_with_components(
            "minecraft:stone",
            vec![entry(
                resource_id,
                NbtSummaryValue::String(named.to_string().into()),
            )],
        );
        let summary = crate::read_item_stack(&saved, &resolver)
            .unwrap_or_else(|error| panic!("{resource_id}: {error}"));
        assert_eq!(
            item_stack_summary_to_persistent_nbt_with_resolver(&summary, &resolver).unwrap(),
            saved.clone(),
            "{resource_id}"
        );
        bbb_protocol::packets::validate_data_component_patch_summary(&summary.component_patch)
            .unwrap_or_else(|error| panic!("{resource_id}: {error}"));
        // The component really did land in the summary under `type_id`.
        assert!(
            summary.component_patch.added_type_ids.contains(&type_id),
            "{resource_id} did not reach {type_id}"
        );
        let (packet_id, bytes) =
            bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
                &bbb_protocol::packets::inventory::ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 0,
                    item: summary.clone(),
                },
            )
            .unwrap();
        let bbb_protocol::packets::play_clientbound::PlayClientbound::ContainerSetSlot(slot) =
            bbb_protocol::packets::play_clientbound::decode_play_clientbound(packet_id, &bytes)
                .unwrap()
        else {
            panic!("{resource_id}: slot packet");
        };
        assert_eq!(
            slot.item.component_patch, summary.component_patch,
            "{resource_id}"
        );
    }
}

#[test]
fn villager_variant_uses_the_fixed_registry_in_both_directions() {
    let type_id = VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID;
    let resource_id = "minecraft:villager/variant";
    let registry_id = "minecraft:villager_type";
    let absent = publishing(&[]);
    for (name, id) in [
        ("minecraft:desert", 0),
        ("minecraft:savanna", 3),
        ("minecraft:taiga", 6),
    ] {
        let saved = saved_stack_with_components(
            "minecraft:stone",
            vec![entry(resource_id, NbtSummaryValue::String(name.into()))],
        );
        let summary = crate::read_item_stack(&saved, &absent).unwrap();
        assert_eq!(summary.component_patch.villager_variant_id, Some(id));
        assert_eq!(
            item_stack_summary_to_persistent_nbt_with_resolver(&summary, &absent).unwrap(),
            saved
        );
    }
    for name in ["minecraft:unknown", "example:variant"] {
        let saved = saved_stack_with_components(
            "minecraft:stone",
            vec![entry(resource_id, NbtSummaryValue::String(name.into()))],
        );
        let error = crate::read_item_stack(&saved, &absent).unwrap_err();
        assert!(error.to_string().contains(name), "{error}");
    }
    for holder_id in [-1, 7, i32::MAX] {
        let patch = DataComponentPatchSummary {
            added_type_ids: vec![type_id],
            villager_variant_id: Some(holder_id),
            ..Default::default()
        };
        assert_eq!(
            item_stack_summary_to_persistent_nbt_with_resolver(
                &stack(VANILLA_ITEM_STONE_ID, 1, patch),
                &absent
            ),
            Err(
                ItemStackPersistentProjectionError::UnresolvedRegistryHolderName {
                    registry_id,
                    holder_id,
                }
            )
        );
    }
}

#[test]
fn direct_painting_variant_keeps_canonical_components_on_slot_wire_but_cannot_save() {
    let title = NbtSummaryValue::string_from_utf16(vec![0xd800]);
    let direct = PaintingVariantSummary {
        width: 20,
        height: 0,
        asset_id: "minecraft:aztec".into(),
        title: Some("�".into()),
        title_styled: Some(bbb_protocol::component::styled_component_runs_from_nbt_value(&title)),
        title_nbt: Some(Box::new(title.clone())),
        author: None,
        author_styled: None,
        author_nbt: None,
    };
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID],
        painting_variant_direct: Some(direct),
        ..Default::default()
    };
    let mut copied = added_patch(&[VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID]);
    bbb_protocol::packets::data_components::fields::copy_component_value(
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        &patch,
        &mut copied,
    )
    .unwrap();
    let summary = stack(VANILLA_ITEM_STONE_ID, 1, copied);
    let (packet_id, bytes) =
        bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
            &bbb_protocol::packets::inventory::ContainerSetSlot {
                container_id: 0,
                state_id: 1,
                slot: 0,
                item: summary.clone(),
            },
        )
        .unwrap();
    let bbb_protocol::packets::play_clientbound::PlayClientbound::ContainerSetSlot(slot) =
        bbb_protocol::packets::play_clientbound::decode_play_clientbound(packet_id, &bytes)
            .unwrap()
    else {
        panic!("slot packet");
    };
    let decoded = slot
        .item
        .component_patch
        .painting_variant_direct
        .as_ref()
        .unwrap();
    assert_eq!((decoded.width, decoded.height), (20, 0));
    assert_eq!(decoded.title_nbt.as_deref(), Some(&title));
    assert!(item_stack_summary_to_persistent_nbt(&summary).is_err());
}
