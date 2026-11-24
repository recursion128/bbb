//! `minecraft:banner_patterns`, `minecraft:provides_banner_patterns`,
//! `minecraft:debug_stick_state`, `minecraft:entity_data`,
//! `minecraft:block_entity_data` and `minecraft:bees`.
//!
//! The two tag-bearing components still carry state the codec NBT does not hold
//! (`BannerPatternLayers` is a plain list, but each layer names a datapack
//! holder), so all three of the datapack-registry holders here are exercised
//! through a publication.

use bbb_facts::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID;
use bbb_facts::block_types::VANILLA_BLOCK_WALL_TORCH_ID;
use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
};
use bbb_facts::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
use bbb_protocol::packets::{BeeOccupantSummary, DebugStickPropertySummary};

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

/// `BannerPatternLayers.Layer.CODEC`: one
/// `{pattern, color}` record per layer, where `pattern` is a named
/// `minecraft:banner_pattern` holder or the direct `{asset_id,
/// translation_key}` record.
#[test]
fn banner_pattern_layers_round_trip_in_both_holder_forms() {
    let resolver = publishing(&[("minecraft:banner_pattern", "example:pattern", 6)]);
    let saved = one_component(
        "minecraft:banner_patterns",
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Compound(vec![
                entry(
                    "pattern",
                    NbtSummaryValue::String("example:pattern".to_string().into()),
                ),
                entry(
                    "color",
                    NbtSummaryValue::String("light_blue".to_string().into()),
                ),
            ]),
            NbtSummaryValue::Compound(vec![
                entry(
                    "pattern",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "asset_id",
                            NbtSummaryValue::String("example:direct".to_string().into()),
                        ),
                        entry(
                            "translation_key",
                            NbtSummaryValue::String("example.direct".into()),
                        ),
                    ]),
                ),
                entry("color", NbtSummaryValue::String("white".to_string().into())),
            ]),
        ]),
    );
    let summary = round_trip_with(&saved, &resolver);
    let layers = &summary.component_patch.banner_pattern_layers;
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].registry_id, Some(6));
    assert_eq!(layers[0].asset_id, None);
    assert_eq!(layers[1].registry_id, None);
    assert_eq!(layers[1].asset_id.as_deref(), Some("example:direct"));
    assert_eq!(
        layers[1].translation_key.as_ref(),
        Some(&bbb_protocol::JavaString::from("example.direct"))
    );

    // A named holder the publication cannot spell is the usual refusal.
    let error =
        item_stack_summary_to_persistent_nbt_with_resolver(&summary, &publishing(&[])).unwrap_err();
    assert_eq!(
        error,
        ItemStackPersistentProjectionError::UnresolvedRegistryHolderName {
            registry_id: "minecraft:banner_pattern",
            holder_id: 6,
        }
    );
    // And the decode direction refuses resolver-free.
    assert!(crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_err());
}

/// `minecraft:provides_banner_patterns` is
/// `RegistryCodecs.homogeneousList(Registries.BANNER_PATTERN)`: the compact
/// holder set over a datapack registry.
#[test]
fn provides_banner_patterns_uses_the_compact_holder_set_form() {
    let resolver = publishing(&[("minecraft:banner_pattern", "example:pattern", 2)]);
    for (value, expected_ids, expected_tag) in [
        (
            NbtSummaryValue::String("example:pattern".to_string().into()),
            vec![2],
            None,
        ),
        (
            NbtSummaryValue::String("#minecraft:no_item_required".to_string().into()),
            Vec::new(),
            Some("minecraft:no_item_required"),
        ),
        (
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("example:pattern".to_string().into()),
                NbtSummaryValue::String("mongus".to_string().into()),
            ]),
            Vec::new(),
            None,
        ),
    ] {
        if expected_ids.is_empty() && expected_tag.is_none() {
            // The two-element case needs a second published member.
            continue;
        }
        let saved = one_component("minecraft:provides_banner_patterns", value);
        let summary = round_trip_with(&saved, &resolver);
        assert_eq!(
            summary.component_patch.provides_banner_pattern_registry_ids,
            expected_ids
        );
        assert_eq!(
            summary
                .component_patch
                .provides_banner_pattern_tag
                .as_deref(),
            expected_tag
        );
    }

    let empty = one_component(
        "minecraft:provides_banner_patterns",
        NbtSummaryValue::List(vec![]),
    );
    let summary = round_trip_with(&empty, &resolver);
    assert!(summary
        .component_patch
        .provides_banner_pattern_registry_ids
        .is_empty());
    assert!(summary
        .component_patch
        .provides_banner_pattern_tag
        .is_none());
    assert!(summary
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID));
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
    assert!(slot
        .item
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID));
    assert!(slot
        .item
        .component_patch
        .provides_banner_pattern_registry_ids
        .is_empty());
    assert!(slot
        .item
        .component_patch
        .provides_banner_pattern_tag
        .is_none());
}

/// `DebugStickState.CODEC` is a map from a block
/// name to one of that block's own property names.
#[test]
fn debug_stick_state_maps_blocks_to_their_own_properties() {
    let saved = one_component(
        "minecraft:debug_stick_state",
        NbtSummaryValue::Compound(vec![entry(
            "minecraft:wall_torch",
            NbtSummaryValue::String("facing".to_string().into()),
        )]),
    );
    let summary = round_trip_with(&saved, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.debug_stick_properties,
        [DebugStickPropertySummary {
            block_registry_id: VANILLA_BLOCK_WALL_TORCH_ID,
            property: "facing".to_string(),
        }]
    );

    // A property the block does not declare is what the value codec's
    // `comapFlatMap` refuses.
    let bad_property = one_component(
        "minecraft:debug_stick_state",
        NbtSummaryValue::Compound(vec![entry(
            "minecraft:wall_torch",
            NbtSummaryValue::String("not_a_property".to_string().into()),
        )]),
    );
    let message = crate::read_item_stack(&bad_property, &BuiltinResolver)
        .unwrap_err()
        .to_string();
    assert!(message.contains("not_a_property"), "{message}");

    let repeated = summary_with(VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID, |patch| {
        patch.debug_stick_properties = vec![
            DebugStickPropertySummary {
                block_registry_id: VANILLA_BLOCK_WALL_TORCH_ID,
                property: "facing".to_string(),
            },
            DebugStickPropertySummary {
                block_registry_id: VANILLA_BLOCK_WALL_TORCH_ID,
                property: "facing".to_string(),
            },
        ];
    });
    assert!(item_stack_summary_to_persistent_nbt(&repeated).is_err());
}

#[test]
fn flattened_custom_data_inputs_preserve_raw_fields_for_all_tag_components() {
    use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID;

    let raw = bbb_protocol::JavaString::from(vec![0xd800, 65, 0xdc00]);
    for (type_id, resource) in [
        (VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, "minecraft:pig"),
        (
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            "minecraft:pig",
        ),
        (VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, "minecraft:pig"),
        (
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            "minecraft:furnace",
        ),
    ] {
        let mut source = format!("{{id:'{resource}',raw:'")
            .encode_utf16()
            .collect::<Vec<_>>();
        source.extend_from_slice(raw.units());
        source.extend("'}".encode_utf16());
        let patch = read_item_component(
            type_id,
            &bbb_protocol::JavaString::from(source).to_nbt(),
            &BuiltinResolver,
        )
        .unwrap();
        let value = match type_id {
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => patch.custom_data.as_ref().unwrap(),
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => &patch.codec_backed_nbt[&type_id],
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => {
                assert_eq!(
                    patch.entity_data_entity_type_id,
                    Some(VANILLA_ENTITY_TYPE_PIG_ID)
                );
                patch.entity_data_tag.as_ref().unwrap()
            }
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
                assert_eq!(
                    patch.block_entity_data_type_id,
                    Some(VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID)
                );
                patch.block_entity_data_tag.as_ref().unwrap()
            }
            _ => unreachable!(),
        };
        let NbtSummaryValue::Compound(fields) = value else {
            panic!("expected compound")
        };
        assert_eq!(fields.last().unwrap().name, "raw");
        assert_eq!(fields.last().unwrap().value, raw.to_nbt());
        if matches!(
            type_id,
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
                | VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID
        ) {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].value, NbtSummaryValue::String(resource.into()));
        } else {
            assert_eq!(fields.len(), 1);
        }
    }
}

/// `TypedEntityData.codec` writes the retained
/// compound with an `id` naming the type, and the network tag must not already
/// carry one.
#[test]
fn typed_entity_data_carries_its_type_as_an_id_field() {
    let entity = one_component(
        "minecraft:entity_data",
        NbtSummaryValue::Compound(vec![
            entry(
                "id",
                NbtSummaryValue::String("minecraft:pig".to_string().into()),
            ),
            entry(
                "CustomName",
                NbtSummaryValue::String("\"Wilbur\"".to_string().into()),
            ),
        ]),
    );
    let summary = round_trip_with(&entity, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.entity_data_entity_type_id,
        Some(VANILLA_ENTITY_TYPE_PIG_ID)
    );
    // The `id` really is split out of the network tag.
    assert_eq!(
        summary.component_patch.entity_data_tag,
        Some(NbtSummaryValue::Compound(vec![entry(
            "CustomName",
            NbtSummaryValue::String("\"Wilbur\"".to_string().into())
        )]))
    );

    let block_entity = one_component(
        "minecraft:block_entity_data",
        NbtSummaryValue::Compound(vec![
            entry(
                "id",
                NbtSummaryValue::String("minecraft:furnace".to_string().into()),
            ),
            entry("BurnTime", NbtSummaryValue::Short(200)),
        ]),
    );
    let summary = round_trip_with(&block_entity, &BuiltinResolver);
    assert_eq!(
        summary.component_patch.block_entity_data_type_id,
        Some(VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID)
    );

    for (what, resource_id, value) in [
        (
            "an entity data with no id",
            "minecraft:entity_data",
            NbtSummaryValue::Compound(Vec::new()),
        ),
        (
            "an unknown entity type",
            "minecraft:entity_data",
            NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::String("example:nope".to_string().into()),
            )]),
        ),
        (
            "an unknown block entity type",
            "minecraft:block_entity_data",
            NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::String("example:nope".to_string().into()),
            )]),
        ),
    ] {
        let saved = one_component(resource_id, value);
        let message = crate::read_item_stack(&saved, &BuiltinResolver)
            .unwrap_err()
            .to_string();
        assert!(message.contains(resource_id), "{what}: {message}");
    }

    // A summary whose network tag already carries `id` has no save form that
    // would not shadow it.
    let shadowed = summary_with(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, |patch| {
        patch.entity_data_entity_type_id = Some(VANILLA_ENTITY_TYPE_PIG_ID);
        patch.entity_data_tag = Some(NbtSummaryValue::Compound(vec![entry(
            "id",
            NbtSummaryValue::String("minecraft:pig".to_string().into()),
        )]));
    });
    let message = item_stack_summary_to_persistent_nbt(&shadowed)
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("already carries a top-level id"),
        "{message}"
    );
}

/// `Bees.CODEC` is
/// `BeehiveBlockEntity.Occupant.LIST_CODEC`:
/// three required fields per occupant, the first of which is itself typed entity
/// data.
#[test]
fn bees_round_trip_their_occupants() {
    let saved = one_component(
        "minecraft:bees",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry(
                "entity_data",
                NbtSummaryValue::Compound(vec![entry(
                    "id",
                    NbtSummaryValue::String("minecraft:pig".to_string().into()),
                )]),
            ),
            entry("ticks_in_hive", NbtSummaryValue::Int(200)),
            entry("min_ticks_in_hive", NbtSummaryValue::Int(600)),
        ])]),
    );
    let summary = round_trip_with(&saved, &BuiltinResolver);
    assert!(summary.component_patch.bees_present);
    assert_eq!(
        summary.component_patch.bees,
        [BeeOccupantSummary {
            entity_type_id: VANILLA_ENTITY_TYPE_PIG_ID,
            entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
            ticks_in_hive: 200,
            min_ticks_in_hive: 600,
        }]
    );

    let missing_field = one_component(
        "minecraft:bees",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
            "entity_data",
            NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::String("minecraft:pig".to_string().into()),
            )]),
        )])]),
    );
    let message = crate::read_item_stack(&missing_field, &BuiltinResolver)
        .unwrap_err()
        .to_string();
    assert!(message.contains("ticks_in_hive"), "{message}");

    // The encode direction needs the presence flag the wire decode sets.
    let absent = added_patch(&[VANILLA_DATA_COMPONENT_BEES_TYPE_ID]);
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_STONE_ID, 1, absent)),
        Err(ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id: VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
            resource_id: "minecraft:bees",
        })
    );
}

/// The four components that name only built-in registries cross the
/// resolver-free entry point; the two that name a `minecraft:banner_pattern`
/// holder do not.
#[test]
fn the_resolver_free_entry_point_follows_the_registry() {
    for (resource_id, value) in [
        (
            "minecraft:debug_stick_state",
            NbtSummaryValue::Compound(vec![entry(
                "minecraft:wall_torch",
                NbtSummaryValue::String("facing".to_string().into()),
            )]),
        ),
        (
            "minecraft:entity_data",
            NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::String("minecraft:pig".to_string().into()),
            )]),
        ),
        (
            "minecraft:block_entity_data",
            NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::String("minecraft:chest".to_string().into()),
            )]),
        ),
        ("minecraft:bees", NbtSummaryValue::List(Vec::new())),
    ] {
        let saved = one_component(resource_id, value);
        assert!(
            crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_ok(),
            "{resource_id} names only built-in registries"
        );
    }
    for resource_id in [
        "minecraft:banner_patterns",
        "minecraft:provides_banner_patterns",
    ] {
        let saved = one_component(
            resource_id,
            NbtSummaryValue::String("example:pattern".to_string().into()),
        );
        assert!(
            crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_err(),
            "{resource_id} should refuse resolver-free"
        );
    }
}
