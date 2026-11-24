use super::*;

use crate::{codec::Encoder, data_component_types::*};

mod block_state_collection_domain;

fn encode_delimited_patch(out: &mut Encoder, patch: &DataComponentPatchSummary) -> Result<()> {
    encode_data_component_patch_summary_with_framing(
        out,
        patch,
        ComponentValueFraming::LengthPrefixed,
    )
}

fn template_with_patch(
    item_id: i32,
    count: i32,
    component_patch: DataComponentPatchSummary,
) -> ItemStackTemplateSummary {
    ItemStackTemplateSummary {
        item_id,
        count,
        component_patch: component_patch.into(),
    }
}

fn plain_template(item_id: i32, count: i32) -> ItemStackTemplateSummary {
    template_with_patch(item_id, count, DataComponentPatchSummary::default())
}

fn nested_use_remainder_template(depth: usize) -> ItemStackTemplateSummary {
    let mut template = plain_template(42, 1);
    for _ in 1..depth {
        template = template_with_patch(
            42,
            1,
            DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID],
                use_remainder: Some(Box::new(template)),
                ..DataComponentPatchSummary::default()
            },
        );
    }
    template
}

fn assert_invalid_data_component_identifier(type_id: i32, write_value: impl FnOnce(&mut Encoder)) {
    let payload = single_data_component_payload(type_id, write_value);
    let mut decoder = Decoder::new(&payload);
    let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
    assert!(
        err.to_string().contains("invalid resource location"),
        "component {type_id} produced unexpected error: {err}"
    );
}

fn single_data_component_payload(type_id: i32, write_value: impl FnOnce(&mut Encoder)) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(type_id);
    write_value(&mut payload);
    payload.into_inner()
}

fn nbt_string_root(value: &str) -> Vec<u8> {
    let mut out = vec![8];
    write_mutf8(&mut out, value);
    out
}

fn plain_runs(text: &str) -> Vec<StyledTextRun> {
    vec![StyledTextRun {
        text: text.to_string(),
        style: Default::default(),
    }]
}

fn empty_nbt_compound_root() -> Vec<u8> {
    vec![10, 0]
}

fn write_item_stack_template(payload: &mut Encoder, item_id: i32, count: i32) {
    write_item_stack_template_with_patch(payload, item_id, count, |payload| {
        payload.write_var_i32(0);
        payload.write_var_i32(0);
    });
}

fn write_nested_use_remainder_template(payload: &mut Encoder, depth: usize) {
    assert!(depth > 0);
    write_item_stack_template_with_patch(payload, 1, 1, |payload| {
        if depth == 1 {
            payload.write_var_i32(0);
            payload.write_var_i32(0);
        } else {
            payload.write_var_i32(1);
            payload.write_var_i32(0);
            payload.write_var_i32(
                crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
            );
            write_nested_use_remainder_template(payload, depth - 1);
        }
    });
}

fn write_cursor_item_with_nested_use_remainder(payload: &mut Encoder, depth: usize) {
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload
        .write_var_i32(crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID);
    write_nested_use_remainder_template(payload, depth);
}

fn run_with_deep_item_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(test)
        .expect("spawn deep item-template test thread")
        .join()
        .expect("deep item-template test thread");
}

fn write_item_stack_template_with_patch(
    payload: &mut Encoder,
    item_id: i32,
    count: i32,
    write_patch: impl FnOnce(&mut Encoder),
) {
    payload.write_var_i32(item_id);
    payload.write_var_i32(count);
    write_patch(payload);
}

fn write_holder_set_tag(payload: &mut Encoder, tag: &str) {
    payload.write_var_i32(0);
    payload.write_string(tag);
}

fn write_direct_sound_event(payload: &mut Encoder, id: &str, fixed_range: Option<f32>) {
    payload.write_var_i32(0);
    payload.write_string(id);
    match fixed_range {
        Some(range) => {
            payload.write_bool(true);
            payload.write_f32(range);
        }
        None => payload.write_bool(false),
    }
}

fn write_mutf8(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

#[test]
fn recursive_item_component_descriptors_round_trip_ordinary_nested_patches() {
    let nested_patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
        max_stack_size: Some(64),
        ..DataComponentPatchSummary::default()
    };
    let remainder = template_with_patch(42, 1, nested_patch);
    let projectile = plain_template(50, 2);
    let bundle_item = plain_template(52, 3);
    let first_container_item = plain_template(53, 4);
    let last_container_item = plain_template(54, 5);
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
            VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
            VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        ],
        use_remainder: Some(Box::new(remainder)),
        charged_projectiles_items: vec![projectile],
        bundle_contents_items: vec![bundle_item],
        container: ContainerContentsSummary {
            slots: vec![
                ItemContainerSlotSummary {
                    slot: 1,
                    item: first_container_item,
                },
                ItemContainerSlotSummary {
                    slot: 3,
                    item: last_container_item,
                },
            ],
            slot_count: Some(5),
        },
        ..DataComponentPatchSummary::default()
    };

    let mut encoded = Encoder::new();
    encode_data_component_patch_summary_with_framing(
        &mut encoded,
        &patch,
        ComponentValueFraming::LengthPrefixed,
    )
    .unwrap();
    let encoded = encoded.into_inner();
    let mut decoder = Decoder::new(&encoded);
    let added = decoder.read_len().unwrap();
    let removed = decoder.read_len().unwrap();
    assert_eq!((added, removed), (4, 0));
    let mut decoded = DataComponentPatchSummary::default();
    for expected_type_id in &patch.added_type_ids {
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(type_id, *expected_type_id);
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder = Decoder::new(
            decoder
                .read_exact(value_len, "recursive component value")
                .unwrap(),
        );
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(
            value_decoder.is_empty(),
            "nested patches must use ordinary, non-delimited framing"
        );
        decoded.added_type_ids.push(type_id);
    }
    assert!(decoder.is_empty());
    assert_eq!(decoded, patch);
    assert_eq!(decoded.container.slot_count, Some(5));
    let mut report = serde_json::to_value(&patch).unwrap();
    assert_eq!(report["bundle_contents_item_count"], 1);
    report["bundle_contents_item_count"] = serde_json::json!(99);
    assert_eq!(
        serde_json::from_value::<DataComponentPatchSummary>(report).unwrap(),
        patch
    );
    assert!(
        serde_json::to_value(DataComponentPatchSummary::default()).unwrap()
            ["bundle_contents_item_count"]
            .is_null()
    );
}

#[test]
fn recursive_item_component_encoder_enforces_tree_and_projection_invariants() {
    let make_remainder_patch = |template| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID],
        use_remainder: Some(Box::new(template)),
        ..DataComponentPatchSummary::default()
    };
    let valid_negative = make_remainder_patch(plain_template(42, -1));
    encode_delimited_patch(&mut Encoder::new(), &valid_negative).unwrap();
    let exact_boundary = make_remainder_patch(nested_use_remainder_template(
        MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
    ));
    encode_delimited_patch(&mut Encoder::new(), &exact_boundary).unwrap();

    let invalid_templates = [
        plain_template(42, 0),
        plain_template(crate::item_types::VANILLA_ITEM_AIR_ID, 1),
        plain_template(i32::MAX, 1),
        nested_use_remainder_template(MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 1),
    ];
    for template in invalid_templates {
        let mut out = Encoder::new();
        let error = encode_delimited_patch(&mut out, &make_remainder_patch(template)).unwrap_err();
        assert!(
            error.to_string().contains("item stack template")
                || error.to_string().contains("item holder")
        );
    }

    let invalid_inner = make_remainder_patch(template_with_patch(
        42,
        1,
        DataComponentPatchSummary {
            added_type_ids: vec![110],
            ..DataComponentPatchSummary::default()
        },
    ));
    let error = encode_delimited_patch(&mut Encoder::new(), &invalid_inner).unwrap_err();
    assert!(error
        .to_string()
        .contains("unsupported data component type id 110"));

    let oversized_projectiles = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID],
        charged_projectiles_items: vec![plain_template(42, 1); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
        ..DataComponentPatchSummary::default()
    };
    let error = encode_delimited_patch(&mut Encoder::new(), &oversized_projectiles).unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(actual, max)
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));

    let item = plain_template(42, 1);
    let invalid_container = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID],
        container: ContainerContentsSummary {
            slots: vec![
                ItemContainerSlotSummary {
                    slot: 2,
                    item: item.clone(),
                },
                ItemContainerSlotSummary { slot: 1, item },
            ],
            slot_count: Some(3),
        },
        ..DataComponentPatchSummary::default()
    };
    assert!(
        encode_delimited_patch(&mut Encoder::new(), &invalid_container)
            .unwrap_err()
            .to_string()
            .contains("strictly increasing")
    );

    let oversized_container = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID],
        container: ContainerContentsSummary {
            slots: Vec::new(),
            slot_count: Some(MAX_CONTAINER_ITEMS + 1),
        },
        ..DataComponentPatchSummary::default()
    };
    let error = encode_delimited_patch(&mut Encoder::new(), &oversized_container).unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(actual, max)
            if actual == MAX_CONTAINER_ITEMS + 1 && max == MAX_CONTAINER_ITEMS
    ));
}

#[test]
fn container_component_round_trips_all_empty_trailing_slots() {
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID],
        container: ContainerContentsSummary {
            slots: Vec::new(),
            slot_count: Some(3),
        },
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    encode_data_component_patch_summary_with_framing(
        &mut encoded,
        &patch,
        ComponentValueFraming::LengthPrefixed,
    )
    .unwrap();
    assert_eq!(
        encoded.into_inner(),
        [
            1, 0, // patch counts
            75, 4, // component id and delimited value length
            3, 0, 0, 0, // three absent optional templates
        ]
    );
}

#[test]
fn container_summary_json_keeps_legacy_projection_keys_and_slot_span() {
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
        ],
        container: ContainerContentsSummary {
            slots: vec![ItemContainerSlotSummary {
                slot: 1,
                item: plain_template(42, 1),
            }],
            slot_count: Some(4),
        },
        firework_explosion: Some(FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            colors: vec![0x11_22_33],
            fade_colors: vec![0x44_55_66],
            has_trail: true,
            has_twinkle: false,
        }),
        ..DataComponentPatchSummary::default()
    };
    let value = serde_json::to_value(&patch).unwrap();
    assert_eq!(value["added"], 2);
    assert_eq!(value["container_items"].as_array().unwrap().len(), 1);
    assert_eq!(value["container_slots"][0]["slot"], 1);
    assert_eq!(value["container_item_count"], 1);
    assert_eq!(value["container_slot_count"], 4);
    assert_eq!(value["firework_explosion_shape"], "star");
    assert_eq!(
        value["firework_explosion_colors"],
        serde_json::json!([0x11_22_33])
    );
    assert_eq!(
        value["firework_explosion_fade_colors"],
        serde_json::json!([0x44_55_66])
    );
    assert_eq!(value["firework_explosion_has_trail"], true);
    assert_eq!(value["firework_explosion_has_twinkle"], false);
    assert!(value.get("firework_explosion").is_none());
    assert_eq!(
        serde_json::from_value::<DataComponentPatchSummary>(value.clone()).unwrap(),
        patch
    );
    let mut mismatched = value;
    mismatched["added"] = serde_json::json!(3);
    assert!(serde_json::from_value::<DataComponentPatchSummary>(mismatched).is_err());
}

#[test]
fn encodes_foundational_data_component_patch_with_delimited_values() {
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        ],
        removed_type_ids: vec![
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
        ],
        max_stack_size: Some(64),
        max_damage: Some(300),
        damage: Some(7),
        unbreakable: true,
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    encode_data_component_patch_summary_with_framing(
        &mut encoded,
        &patch,
        ComponentValueFraming::LengthPrefixed,
    )
    .unwrap();
    let encoded = encoded.into_inner();
    assert_eq!(
        encoded,
        [
            4, 2, // positive and negative counts
            3, 1, 7, // damage: type, byte length, VarInt value
            4, 0, // unbreakable: type, zero-byte Unit value
            1, 1, 64, // max_stack_size
            2, 2, 0xac, 0x02, // max_damage = 300
            0, 109, // removed types
        ]
    );

    let mut decoder = Decoder::new(&encoded);
    assert_eq!(decoder.read_len().unwrap(), 4);
    assert_eq!(decoder.read_len().unwrap(), 2);
    let mut decoded = DataComponentPatchSummary::default();
    for expected_type_id in [
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    ] {
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(type_id, expected_type_id);
        let value_len = decoder.read_len().unwrap();
        let value = decoder
            .read_exact(value_len, "delimited component value")
            .unwrap();
        let mut value_decoder = Decoder::new(value);
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
    }
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 109);
    assert!(decoder.is_empty());
    assert_eq!(decoded.max_stack_size, Some(64));
    assert_eq!(decoded.max_damage, Some(300));
    assert_eq!(decoded.damage, Some(7));
    assert!(decoded.unbreakable);
}

#[test]
fn component_patch_encoder_rejects_noncanonical_summaries() {
    let valid_max_stack_size = || DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
        max_stack_size: Some(64),
        ..DataComponentPatchSummary::default()
    };
    let mut cases = Vec::new();

    let mut duplicate_added = valid_max_stack_size();
    duplicate_added.added_type_ids.push(1);
    cases.push((duplicate_added, "duplicate added"));

    let mut overlapping = valid_max_stack_size();
    overlapping
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
    cases.push((overlapping, "overlapping removed"));

    let duplicate_removed = DataComponentPatchSummary {
        removed_type_ids: vec![
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        ],
        ..DataComponentPatchSummary::default()
    };
    cases.push((duplicate_removed, "duplicate or overlapping removed"));

    let incomplete_custom_name = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
        custom_name: Some("missing exact tree".to_string()),
        ..DataComponentPatchSummary::default()
    };
    cases.push((incomplete_custom_name, "missing component nbt"));

    let missing_value = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };
    cases.push((missing_value, "missing max_stack_size"));

    let missing_custom_data = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };
    cases.push((missing_custom_data, "missing custom_data"));

    let scalar_custom_data = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
        custom_data: Some(NbtSummaryValue::Int(42)),
        ..DataComponentPatchSummary::default()
    };
    cases.push((scalar_custom_data, "must be a non-null compound tag"));

    let missing_entity_data_tag = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID],
        entity_data_entity_type_id: Some(crate::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        ..DataComponentPatchSummary::default()
    };
    cases.push((missing_entity_data_tag, "missing entity_data"));

    let missing_bucket_entity_data = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };
    cases.push((missing_bucket_entity_data, "missing bucket_entity_data"));

    let stale_block_entity_derived_value = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID],
        block_entity_data_type_id: Some(
            crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
        ),
        block_entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
        block_entity_spawn_entity_type: Some("minecraft:pig".to_string()),
        ..DataComponentPatchSummary::default()
    };
    cases.push((
        stale_block_entity_derived_value,
        "dangling summarized values",
    ));

    let missing_profile = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };
    cases.push((missing_profile, "missing profile"));

    let stale_profile_textures = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID],
        profile: Some(ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some("Alex".to_string()),
            properties: Vec::new(),
            profile_textures: Some(ProfileTexturesSummary {
                skin: None,
                cape: None,
                elytra: None,
            }),
            skin_patch: PlayerSkinPatchSummary::default(),
        }),
        ..DataComponentPatchSummary::default()
    };
    cases.push((stale_profile_textures, "dangling summarized values"));

    let dangling_value = DataComponentPatchSummary {
        damage: Some(1),
        ..DataComponentPatchSummary::default()
    };
    cases.push((dangling_value, "dangling summarized values"));

    let missing_unit = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };
    cases.push((missing_unit, "missing unbreakable"));

    let unknown_removed = DataComponentPatchSummary {
        removed_type_ids: vec![110],
        ..DataComponentPatchSummary::default()
    };
    cases.push((unknown_removed, "unsupported data component type id 110"));

    for (patch, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = encode_data_component_patch_summary_with_framing(
            &mut encoded,
            &patch,
            ComponentValueFraming::LengthPrefixed,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
    }

    let oversized = DataComponentPatchSummary {
        removed_type_ids: vec![
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID;
            MAX_DATA_COMPONENT_PATCH_ENTRIES + 1
        ],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    let error = encode_data_component_patch_summary_with_framing(
        &mut encoded,
        &oversized,
        ComponentValueFraming::LengthPrefixed,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(actual, max)
            if actual == MAX_DATA_COMPONENT_PATCH_ENTRIES + 1
                && max == MAX_DATA_COMPONENT_PATCH_ENTRIES
    ));
}

#[test]
fn fixed_registry_network_holder_sets_reject_unknown_direct_ids() {
    for (registry, invalid_id, expected) in [
        (
            NetworkHolderSetRegistry::Item,
            crate::item_types::VANILLA_ITEM_TYPES.len() as i32,
            "invalid item holder id",
        ),
        (
            NetworkHolderSetRegistry::MobEffect,
            crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32,
            "invalid mob effect holder id",
        ),
        (
            NetworkHolderSetRegistry::EntityType,
            crate::entity_types::VANILLA_ENTITY_TYPES.len() as i32,
            "invalid entity type holder id",
        ),
    ] {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(invalid_id);
        let bytes = payload.into_inner();
        let Err(error) = decode_holder_set_summary(&mut Decoder::new(&bytes), registry) else {
            panic!("unknown direct holder unexpectedly decoded for {registry:?}");
        };
        assert!(error.to_string().contains(expected), "{error}");
    }

    for registry in [
        NetworkHolderSetRegistry::Item,
        NetworkHolderSetRegistry::MobEffect,
        NetworkHolderSetRegistry::EntityType,
    ] {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(-1);
        let bytes = payload.into_inner();
        assert!(decode_holder_set_summary(&mut Decoder::new(&bytes), registry).is_err());
    }
}

#[test]
fn decodes_supported_data_component_patch_values() {
    let mut payload = Encoder::new();
    payload.write_var_i32(17);
    payload.write_var_i32(2);
    payload.write_var_i32(1);
    payload.write_var_i32(64);
    payload.write_var_i32(2);
    payload.write_var_i32(432);
    payload.write_var_i32(3);
    payload.write_var_i32(431);
    payload.write_var_i32(4);
    payload.write_var_i32(6);
    payload.write_bytes(&nbt_string_root("Named"));
    payload.write_var_i32(10);
    payload.write_string("minecraft:diamond_sword");
    payload.write_var_i32(19);
    payload.write_var_i32(5);
    payload.write_var_i32(20);
    payload.write_var_i32(21);
    payload.write_bool(true);
    payload.write_var_i32(22);
    payload.write_bytes(&empty_nbt_compound_root());
    payload.write_var_i32(26);
    payload.write_f32(1.5);
    payload.write_bool(true);
    payload.write_string("minecraft:ender_pearl");
    payload.write_var_i32(31);
    payload.write_var_i32(15);
    payload.write_var_i32(34);
    payload.write_var_i32(35);
    payload.write_string("minecraft:rare");
    payload.write_var_i32(63);
    payload.write_var_i32(3);
    payload.write_var_i32(71);
    payload.write_string("minecraft:block.note_block.harp");
    payload.write_var_i32(79);
    payload.write_bytes(
        &crate::nbt_summary::encode_nbt_summary_bounded(
            Some(&NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "loot_table".into(),
                value: NbtSummaryValue::String("minecraft:empty".into()),
            }])),
            i32::MAX as usize,
            crate::nbt_summary::MAX_NBT_DEPTH,
        )
        .unwrap(),
    );
    payload.write_var_i32(13);
    payload.write_var_i32(12);

    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);
    let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
    assert_eq!(
        patch,
        DataComponentPatchSummary {
            added_type_ids: vec![1, 2, 3, 4, 6, 10, 19, 20, 21, 22, 26, 31, 34, 35, 63, 71, 79,],
            removed_type_ids: vec![13, 12],
            max_stack_size: Some(64),
            max_damage: Some(432),
            damage: Some(431),
            repair_cost: Some(5),
            creative_slot_lock: true,
            unbreakable: true,
            custom_name: Some("Named".to_string()),
            custom_name_styled: Some(plain_runs("Named")),
            custom_name_nbt: Some(NbtSummaryValue::String("Named".into())),
            item_model: Some("minecraft:diamond_sword".to_string()),
            enchantment_glint_override: Some(true),
            intangible_projectile: true,
            use_cooldown_seconds_bits: Some(1.5_f32.to_bits()),
            use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
            enchantable_value: Some(15),
            tooltip_style: Some("minecraft:rare".to_string()),
            note_block_sound: Some("minecraft:block.note_block.harp".to_string()),
            ominous_bottle_amplifier: Some(3),
            container_loot: true,
            container_loot_table: Some("minecraft:empty".to_string()),
            container_loot_seed: Some(0),
            codec_backed_nbt: BTreeMap::from([(
                79,
                NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "loot_table".into(),
                    value: NbtSummaryValue::String("minecraft:empty".into()),
                }]),
            )]),
            ..DataComponentPatchSummary::default()
        }
    );
    assert!(decoder.is_empty());
}

#[test]
fn mob_effect_java_equals_ignores_queued_hidden_effect_but_struct_eq_retains_it() {
    let base = MobEffectInstanceSummary {
        effect_id: 1,
        amplifier: 2,
        duration: 120,
        ambient: true,
        show_particles: false,
        show_icon: true,
        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
            amplifier: 3,
            duration: 40,
            ambient: false,
            show_particles: true,
            show_icon: false,
            hidden_effect: None,
        })),
    };
    let without_hidden = MobEffectInstanceSummary {
        hidden_effect: None,
        ..base.clone()
    };
    assert_ne!(base, without_hidden);
    assert!(base.java_equals(&without_hidden));
    for changed in [
        MobEffectInstanceSummary {
            effect_id: 2,
            ..base.clone()
        },
        MobEffectInstanceSummary {
            amplifier: 3,
            ..base.clone()
        },
        MobEffectInstanceSummary {
            duration: 121,
            ..base.clone()
        },
        MobEffectInstanceSummary {
            ambient: false,
            ..base.clone()
        },
        MobEffectInstanceSummary {
            show_particles: true,
            ..base.clone()
        },
        MobEffectInstanceSummary {
            show_icon: false,
            ..base.clone()
        },
    ] {
        assert!(!base.java_equals(&changed));
    }
}

#[test]
fn rejects_unknown_consume_effect_type_without_consuming_payload() {
    let mut payload = Encoder::new();
    payload.write_var_i32(5);
    payload.write_var_i32(123);
    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);

    let error = decode_consume_effect_summary(&mut decoder).unwrap_err();
    assert!(error
        .to_string()
        .contains("invalid consume effect type id 5"));
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
}

#[test]
fn decodes_item_rarity_out_of_bounds_as_common() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(0);

    payload.write_var_i32(12);
    payload.write_var_i32(99);

    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);
    let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

    assert_eq!(
        patch,
        DataComponentPatchSummary {
            added_type_ids: vec![12],
            removed_type_ids: Vec::new(),
            rarity: Some(ItemRaritySummary::Common),
            ..DataComponentPatchSummary::default()
        }
    );
    assert!(decoder.is_empty());
}

#[test]
fn enchantment_maps_reject_negative_registry_holders() {
    for type_id in [13, 42] {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(type_id);
        payload.write_var_i32(1);
        payload.write_var_i32(-1);

        let error = decode_data_component_patch_summary(&mut Decoder::new(&payload.into_inner()))
            .unwrap_err();
        assert_eq!(error.to_string(), "negative length -1");
    }
}

#[test]
fn enchantment_maps_apply_last_duplicate_value_in_first_key_order() {
    for type_id in [13, 42] {
        let mut payload = Encoder::new();
        payload.write_var_i32(3);
        payload.write_var_i32(7);
        payload.write_var_i32(-1);
        payload.write_var_i32(8);
        payload.write_var_i32(2);
        payload.write_var_i32(7);
        payload.write_var_i32(3);

        let bytes = payload.into_inner();
        let mut decoder = Decoder::new(&bytes);
        let mut summary = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut decoder, &mut summary)
            .unwrap();
        assert!(decoder.is_empty());
        let expected = vec![
            ItemEnchantmentSummary {
                holder_id: 7,
                level: 3,
            },
            ItemEnchantmentSummary {
                holder_id: 8,
                level: 2,
            },
        ];
        if type_id == 13 {
            assert_eq!(summary.enchantments, expected);
        } else {
            assert_eq!(summary.stored_enchantments, expected);
        }
    }
}

#[test]
fn enchantment_maps_reject_network_levels_outside_constructor_range() {
    for type_id in [13, 42] {
        for level in [-1, 256] {
            let mut payload = Encoder::new();
            payload.write_var_i32(1);
            payload.write_var_i32(0);
            payload.write_var_i32(level);

            let bytes = payload.into_inner();
            let mut summary = DataComponentPatchSummary::default();
            let error = data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut Decoder::new(&bytes), &mut summary)
                .unwrap_err();
            assert!(error
                .to_string()
                .contains("is outside network range 0..=255"));
        }
    }
}

#[test]
fn decodes_map_post_processing_out_of_bounds_as_lock() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(0);

    payload.write_var_i32(48);
    payload.write_var_i32(99);

    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);
    let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

    assert_eq!(
        patch.map_post_processing,
        Some(MapPostProcessingSummary::Lock)
    );
    assert!(decoder.is_empty());
}

#[test]
fn decodes_empty_bees_data_component_presence() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(0);

    payload.write_var_i32(77);
    payload.write_var_i32(0);

    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);
    let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
    assert_eq!(
        patch,
        DataComponentPatchSummary {
            added_type_ids: vec![77],
            removed_type_ids: Vec::new(),
            bees_present: true,
            ..DataComponentPatchSummary::default()
        }
    );
    assert!(decoder.is_empty());
}

#[test]
fn bees_typed_entity_data_strips_id_and_rejects_invalid_values() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(77);
    payload.write_var_i32(1);
    payload.write_var_i32(crate::entity_types::VANILLA_ENTITY_TYPE_BEE_ID);
    payload.write_bytes(&[
        10, // compound
        8, 0, 2, b'i', b'd', 0, 13, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':',
        b'b', b'e', b'e', 0,
    ]);
    payload.write_var_i32(40);
    payload.write_var_i32(2400);

    let payload = payload.into_inner();
    let patch = decode_data_component_patch_summary(&mut Decoder::new(&payload)).unwrap();
    assert_eq!(
        patch.bees,
        vec![BeeOccupantSummary {
            entity_type_id: crate::entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
            entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
            ticks_in_hive: 40,
            min_ticks_in_hive: 2400,
        }]
    );

    for (entity_type_id, tag, expected_error) in [
        (
            crate::entity_types::VANILLA_ENTITY_TYPES.len() as i32,
            vec![10, 0],
            "invalid vanilla entity type holder",
        ),
        (
            crate::entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
            vec![3, 0, 0, 0, 42],
            "must be a non-null compound tag",
        ),
    ] {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(77);
        payload.write_var_i32(1);
        payload.write_var_i32(entity_type_id);
        payload.write_bytes(&tag);
        let payload = payload.into_inner();
        let error = decode_data_component_patch_summary(&mut Decoder::new(&payload)).unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }
}

#[test]
fn rejects_invalid_identifier_data_component_values() {
    assert_invalid_data_component_identifier(10, |payload| {
        payload.write_string("minecraft:DiamondSword");
    });
    assert_invalid_data_component_identifier(35, |payload| {
        payload.write_string("minecraft:Tooltip");
    });
    assert_invalid_data_component_identifier(71, |payload| {
        payload.write_string("minecraft:NoteBlock");
    });
    assert_invalid_data_component_identifier(26, |payload| {
        payload.write_f32(1.0);
        payload.write_bool(true);
        payload.write_string("minecraft:EnderPearl");
    });
    assert_invalid_data_component_identifier(65, |payload| {
        write_holder_set_tag(payload, "minecraft:NoItemRequired");
    });
    assert_invalid_data_component_identifier(67, |payload| {
        payload.write_bool(true);
        payload.write_string("minecraft:Overworld");
        payload.write_i64(0);
    });
    assert_invalid_data_component_identifier(32, |payload| {
        payload.write_var_i32(5);
        write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
        payload.write_bool(true);
        payload.write_string("minecraft:Diamond");
    });
    assert_invalid_data_component_identifier(32, |payload| {
        payload.write_var_i32(5);
        write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("minecraft:Misc/Pumpkinblur");
    });
    assert_invalid_data_component_identifier(80, |payload| {
        payload.write_var_i32(0);
        payload.write_string("minecraft:Block.NoteBlock.Harp");
    });
    assert_invalid_data_component_identifier(102, |payload| {
        payload.write_var_i32(0);
        payload.write_var_i32(16);
        payload.write_var_i32(16);
        payload.write_string("minecraft:Kebab");
    });
}

#[test]
fn rejects_unknown_data_component_type_without_consuming_payload_guesswork() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(0);
    payload.write_var_i32(110);

    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);
    let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
    assert!(err
        .to_string()
        .contains("unsupported data component type id 110"));
}

#[test]
fn item_stack_template_requires_a_non_empty_builtin_item_holder() {
    let mut air = Encoder::new();
    write_item_stack_template(&mut air, crate::item_types::VANILLA_ITEM_AIR_ID, 1);
    let error = decode_item_stack_template(&mut Decoder::new(&air.into_inner())).unwrap_err();
    assert!(error
        .to_string()
        .contains("item stack template holder must not be minecraft:air"));

    let mut out_of_range = Encoder::new();
    let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
    write_item_stack_template(&mut out_of_range, invalid_item_id, 1);
    let error =
        decode_item_stack_template(&mut Decoder::new(&out_of_range.into_inner())).unwrap_err();
    assert!(error
        .to_string()
        .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
}

#[test]
fn item_stack_template_network_count_matches_vanilla_non_zero_rule() {
    let mut negative = Encoder::new();
    write_item_stack_template(&mut negative, 1, -1);
    let decoded = decode_item_stack_template(&mut Decoder::new(&negative.into_inner())).unwrap();
    assert_eq!(decoded.count, -1);

    let mut zero = Encoder::new();
    write_item_stack_template(&mut zero, 1, 0);
    let error = decode_item_stack_template(&mut Decoder::new(&zero.into_inner())).unwrap_err();
    assert_eq!(error.to_string(), "invalid item stack template count 0");
}

#[test]
fn item_stack_template_nesting_enforces_exact_boundary() {
    run_with_deep_item_stack(|| {
        let mut accepted = Encoder::new();
        write_cursor_item_with_nested_use_remainder(
            &mut accepted,
            MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
        );
        crate::packets::decode_play_clientbound(
            crate::ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
            &accepted.into_inner(),
        )
        .unwrap();

        let mut rejected = Encoder::new();
        write_cursor_item_with_nested_use_remainder(
            &mut rejected,
            MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 1,
        );
        let error = crate::packets::decode_play_clientbound(
            crate::ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
            &rejected.into_inner(),
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "item stack template nesting exceeds 32");
    });
}

/// Reverse guard for `MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH`: raising the
/// semantic limit or increasing debug-build frames must fail before a
/// production-sized 2 MiB Tokio worker stack loses its reserved headroom.
#[test]
fn full_depth_item_stack_template_stays_within_stack_budget() {
    const STACK_BUDGET_BYTES: usize = 3 * 1024 * 1024 / 2;
    run_with_deep_item_stack(|| {
        let mut payload = Encoder::new();
        write_cursor_item_with_nested_use_remainder(
            &mut payload,
            MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
        );
        let caller_marker = 0u8;
        let caller_address = std::ptr::from_ref(&caller_marker) as usize;
        ITEM_STACK_TEMPLATE_STACK_EXTREMES.with(|cell| cell.set((caller_address, caller_address)));
        crate::packets::decode_play_clientbound(
            crate::ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
            &payload.into_inner(),
        )
        .unwrap();
        let (top, bottom) = ITEM_STACK_TEMPLATE_STACK_EXTREMES.with(std::cell::Cell::get);
        let used = top - bottom;
        assert!(
            used <= STACK_BUDGET_BYTES,
            "depth-{MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH} item stack template used {used} \
             bytes of stack, over the {STACK_BUDGET_BYTES}-byte budget"
        );
    });
}

#[test]
fn mob_effect_component_values_reject_out_of_range_builtin_holders() {
    let invalid_effect_id = crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32;

    let mut instance = Encoder::new();
    instance.write_var_i32(invalid_effect_id);
    let error = decode_mob_effect_instance(&mut Decoder::new(&instance.into_inner())).unwrap_err();
    assert!(error.to_string().contains(&format!(
        "invalid vanilla mob effect holder id {invalid_effect_id}"
    )));

    let mut stew = Encoder::new();
    stew.write_var_i32(1);
    stew.write_var_i32(invalid_effect_id);
    let error = decode_suspicious_stew_effects(&mut Decoder::new(&stew.into_inner())).unwrap_err();
    assert!(error.to_string().contains(&format!(
        "invalid vanilla mob effect holder id {invalid_effect_id}"
    )));
}

#[test]
fn item_attribute_modifiers_reject_out_of_range_builtin_holders() {
    let invalid_attribute_id = crate::attribute_types::VANILLA_ATTRIBUTE_TYPES.len() as i32;
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(invalid_attribute_id);

    let error = decode_attribute_modifiers(
        &mut Decoder::new(&payload.into_inner()),
        crate::content_registry::ContentRegistry::vanilla(),
    )
    .unwrap_err();
    assert!(error.to_string().contains(&format!(
        "invalid vanilla attribute holder id {invalid_attribute_id}"
    )));
}

#[test]
fn potion_contents_rejects_holders_outside_the_builtin_registry() {
    let invalid_potion_id = crate::potion_types::VANILLA_POTIONS.len() as i32;
    let mut payload = Encoder::new();
    payload.write_bool(true);
    payload.write_var_i32(invalid_potion_id);

    let error = data_component_descriptor_or_error(
        crate::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    )
    .unwrap()
    .decode(
        &mut Decoder::new(&payload.into_inner()),
        &mut DataComponentPatchSummary::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains(&format!(
        "invalid vanilla potion holder id {invalid_potion_id}"
    )));
}

#[test]
fn sparse_data_component_patch_keeps_plain_item_stacks_compact_and_unallocated() {
    assert!(std::mem::size_of::<crate::packets::ItemStackSummary>() <= 24);
    assert_eq!(
        std::mem::size_of::<ItemStackTemplateSummary>(),
        2 * std::mem::size_of::<i32>() + std::mem::size_of::<SparseDataComponentPatchSummary>()
    );
    assert_eq!(
        std::mem::size_of::<crate::packets::AdvancementIconSummary>(),
        2 * std::mem::size_of::<i32>() + std::mem::size_of::<SparseDataComponentPatchSummary>()
    );

    let empty = crate::packets::ItemStackSummary::empty();
    assert!(!empty.component_patch.is_allocated());
    assert_eq!(empty.component_patch, DataComponentPatchSummary::default());

    let mut touched = crate::packets::ItemStackSummary::empty();
    assert_eq!(touched.component_patch.damage, None);
    let _ = &mut *touched.component_patch;
    assert!(touched.component_patch.is_allocated());
    assert_eq!(touched, empty);

    let json = serde_json::to_value(&touched).unwrap();
    let restored: crate::packets::ItemStackSummary = serde_json::from_value(json).unwrap();
    assert!(!restored.component_patch.is_allocated());
    assert_eq!(restored, empty);
}

#[test]
fn banner_patterns_rejects_negative_holder_id() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(i32::MIN);
    let err = data_component_descriptor_or_error(
        crate::data_component_types::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
    )
    .unwrap()
    .decode(
        &mut Decoder::new(&payload.into_inner()),
        &mut DataComponentPatchSummary::default(),
    )
    .unwrap_err();
    assert!(matches!(err, ProtocolError::NegativeLength(i32::MIN)));
}

#[test]
fn patch_decode_validates_removed_ids_and_canonicalizes_overlaps() {
    // Unknown removed type id is rejected.
    let mut bad = Encoder::new();
    bad.write_var_i32(0);
    bad.write_var_i32(1);
    bad.write_var_i32(999_999);
    assert!(decode_data_component_patch_summary(&mut Decoder::new(&bad.into_inner())).is_err());

    // Overlapping added + removed follows Vanilla last-wins map semantics and stays encodable.
    let mut overlap = Encoder::new();
    overlap.write_var_i32(2);
    overlap.write_var_i32(2);
    overlap.write_var_i32(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    overlap.write_var_i32(5);
    overlap.write_var_i32(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    overlap.write_var_i32(9);
    overlap.write_var_i32(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    overlap.write_var_i32(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    let patch = decode_data_component_patch_summary(&mut Decoder::new(&overlap.into_inner()))
        .expect("overlapping wire patch should canonicalize cleanly");
    assert!(patch.added_type_ids.is_empty());
    assert_eq!(
        patch.removed_type_ids,
        vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID]
    );
    assert_eq!(patch.damage, None);
    validate_data_component_patch_summary(&patch).expect("canonicalized patch must be encodable");
}
