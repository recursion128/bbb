use super::*;
use bbb_protocol::packets::NbtSummaryEntry;

#[test]
fn bundle_item_weight_treats_empty_bundle_component_as_nested_bundle() {
    // Vanilla `BundleContents.getWeight` checks for the presence of the
    // BUNDLE_CONTENTS component, not for non-empty contents. An empty nested
    // bundle therefore still weighs the fixed bundle-in-bundle `1/16`.
    let nested_empty_bundle = ItemStackTemplateSummary {
        item_id: 3,
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };

    let mut patch = DataComponentPatchSummary {
        bundle_contents_items: vec![nested_empty_bundle],
        ..DataComponentPatchSummary::default()
    };
    let weight = bundle_fullness_value(Some(&patch), None);
    assert!((weight - 1.0 / 16.0).abs() < f32::EPSILON);

    let codecs = std::collections::BTreeMap::from([(
        3,
        std::collections::BTreeMap::from([
            (
                component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
                serde_json::json!([]),
            ),
            (
                component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                serde_json::json!(1),
            ),
        ]),
    )]);
    let mut default_patch = DataComponentPatchSummary::default();
    for (type_id, value) in &codecs[&3] {
        let nbt = bbb_protocol::json_value_to_nbt_summary(value).unwrap();
        bbb_registry_codec::read_persistent_component_into_with_resolver(
            *type_id,
            &nbt,
            None,
            &mut default_patch,
        )
        .unwrap();
    }
    let item_defaults = bbb_render_types::ItemComponentDefaults::from_btree_maps(
        codecs,
        std::collections::BTreeMap::from([(3, default_patch)]),
    );
    patch.bundle_contents_items[0]
        .component_patch
        .added_type_ids
        .clear();
    assert_eq!(
        bundle_fullness_value(Some(&patch), Some(item_defaults.as_view())),
        1.0 / 16.0
    );
    patch.bundle_contents_items[0]
        .component_patch
        .removed_type_ids = vec![component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID];
    assert_eq!(
        bundle_fullness_value(Some(&patch), Some(item_defaults.as_view())),
        1.0
    );
}

#[test]
fn bundle_fullness_walks_deep_nested_bundles_without_recursion() {
    let mut nested = ItemStackTemplateSummary {
        item_id: 3,
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    for _ in 1..=bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        nested = ItemStackTemplateSummary {
            item_id: 3,
            count: 1,
            component_patch: DataComponentPatchSummary {
                bundle_contents_items: vec![nested],
                added_type_ids: vec![component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        };
    }
    let patch = DataComponentPatchSummary {
        bundle_contents_items: vec![nested],
        added_type_ids: vec![component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
        ..DataComponentPatchSummary::default()
    };

    assert_eq!(
        bundle_fullness_value(Some(&patch), None),
        (bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH as f32 + 1.0) / 16.0
    );
}

#[test]
fn consume_effect_family_component_presence_drives_item_models() {
    let consumable_id = component_id::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID;
    let death_protection_id = component_id::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID;

    let patch = DataComponentPatchSummary {
        added_type_ids: vec![consumable_id, death_protection_id],
        ..DataComponentPatchSummary::default()
    };
    assert!(item_stack_has_component_id(
        consumable_id,
        Some(&patch),
        None,
        None,
        false,
    ));
    assert!(item_stack_has_component_id(
        death_protection_id,
        Some(&patch),
        None,
        None,
        false,
    ));

    let catalog = bbb_client_assets::ItemRegistryCatalog::vanilla_26_1().unwrap();
    let default_values = BTreeMap::from([
        (
            consumable_id,
            catalog
                .default_component_codec_values("minecraft:apple")
                .unwrap()[&consumable_id]
                .clone(),
        ),
        (
            death_protection_id,
            catalog
                .default_component_codec_values("minecraft:totem_of_undying")
                .unwrap()[&death_protection_id]
                .clone(),
        ),
    ]);
    assert!(item_stack_has_component_id(
        consumable_id,
        None,
        Some(&default_values),
        None,
        false,
    ));

    let removed = DataComponentPatchSummary {
        removed_type_ids: vec![consumable_id, death_protection_id],
        ..patch
    };
    assert!(!item_stack_has_component_id(
        consumable_id,
        Some(&removed),
        Some(&default_values),
        None,
        false,
    ));
    assert!(!item_stack_has_component_id(
        death_protection_id,
        Some(&removed),
        Some(&default_values),
        None,
        false,
    ));
}

#[test]
fn exact_sound_event_matches_registry_and_direct_holders() {
    let parse = |sound_event| {
        let condition = component_condition(
            "minecraft:bundle_contents",
            serde_json::json!({
                "items": {"contains": [{"components": {"components": {
                    "minecraft:jukebox_playable": {
                        "sound_event": sound_event,
                        "description": "Test song",
                        "length_in_seconds": 3.5,
                        "comparator_output": 7
                    }
                }}}]}
            }),
        );
        let ItemModelCondition::Component(
            bbb_client_assets::ItemComponentPredicate::BundleContents(items),
        ) = condition
        else {
            panic!("bundle component condition");
        };
        let bbb_client_assets::ItemExactComponent::JukeboxPlayable(
            bbb_client_assets::ExactJukeboxPlayable::DirectSong { sound_event, .. },
        ) = &items.contains[0].components[0]
        else {
            panic!("direct jukebox component");
        };
        sound_event.clone()
    };
    let named = parse(serde_json::json!("minecraft:entity.cat.ambient"));
    assert_eq!(named.registry_id, Some(286));
    assert_eq!(named.sound_id, None);
    assert_eq!(named.fixed_range_bits, None);

    let direct = parse(serde_json::json!({"sound_id": "minecraft:test.song", "range": 16.0}));
    assert_eq!(direct.registry_id, None);
    assert_eq!(direct.sound_id.as_deref(), Some("minecraft:test.song"));
    assert_eq!(direct.fixed_range_bits, Some(16.0f32.to_bits()));
}

/// The `minecraft:component` condition a pack writes with this predicate.
fn component_condition(predicate: &str, value: serde_json::Value) -> ItemModelCondition {
    let definition = serde_json::json!({
        "model": {
            "type": "minecraft:condition",
            "property": "minecraft:component",
            "predicate": predicate,
            "value": value,
            "on_true": {"type": "minecraft:empty"},
            "on_false": {"type": "minecraft:empty"}
        }
    });
    let definition =
        bbb_client_assets::ClientItemDefinition::from_json_bytes(definition.to_string().as_bytes())
            .unwrap();
    let ItemModelDefinition::Condition { condition, .. } = definition.model else {
        panic!("condition definition");
    };
    condition
}

fn custom_data_condition_nbt(snbt: &str) -> Option<NbtSummaryValue> {
    match component_condition("minecraft:custom_data", serde_json::Value::from(snbt)) {
        ItemModelCondition::Component(bbb_client_assets::ItemComponentPredicate::CustomData(
            expected,
        )) => Some(expected),
        _ => None,
    }
}

#[test]
fn custom_data_predicate_accepts_snbt_compound_strings() {
    let expected = custom_data_condition_nbt(
        r#"{owner:"Alex",level:7,nested:{flag:true},lore:["two"],bytes:[B;1b,2b]}"#,
    )
    .unwrap();
    let actual = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "owner".into(),
            value: NbtSummaryValue::string("Alex"),
        },
        NbtSummaryEntry {
            name: "level".into(),
            value: NbtSummaryValue::Int(7),
        },
        NbtSummaryEntry {
            name: "nested".into(),
            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "flag".into(),
                value: NbtSummaryValue::Byte(1),
            }]),
        },
        NbtSummaryEntry {
            name: "lore".into(),
            value: NbtSummaryValue::List(vec![
                NbtSummaryValue::string("one"),
                NbtSummaryValue::string("two"),
            ]),
        },
        NbtSummaryEntry {
            name: "bytes".into(),
            value: NbtSummaryValue::ByteArray(vec![1, 2]),
        },
    ]);

    assert!(nbt_summary_matches(&expected, &actual, true));
}

#[test]
fn custom_data_predicate_snbt_uses_shared_grammar_in_item_matching() {
    let expected =
        custom_data_condition_nbt(r"{value:'\uD800',number:0b101,overwritten:0,overwritten:1}")
            .unwrap();
    let actual = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "value".into(),
            value: NbtSummaryValue::string_from_utf16(vec![0xd800]),
        },
        NbtSummaryEntry {
            name: "number".into(),
            value: NbtSummaryValue::Int(5),
        },
        NbtSummaryEntry {
            name: "overwritten".into(),
            value: NbtSummaryValue::Int(1),
        },
    ]);
    assert!(nbt_summary_matches(&expected, &actual, true));
    for invalid in [
        r"{value:'\q'}",
        "{value:1e999}",
        "{bad key:1}",
        "{value:1} trailing",
    ] {
        assert_eq!(custom_data_condition_nbt(invalid), None, "{invalid}");
    }
}

#[test]
fn negative_protocol_stack_size_and_max_damage_never_panic_or_divide_by_zero() {
    let patch = DataComponentPatchSummary {
        max_stack_size: Some(-10),
        max_damage: Some(-5),
        damage: Some(3),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(effective_item_max_stack_size(Some(&patch), Some(-1)), 1);
    assert_eq!(range_dispatch_count_value(5, -10, false), 1.0);
    assert_eq!(range_dispatch_count_value(5, 0, true), 1.0);
    assert_eq!(clamp_range_dispatch_damage(3, -5, false), 0.0);
    assert_eq!(clamp_range_dispatch_damage(3, -5, true), 0.0);
    assert_eq!(clamp_range_dispatch_damage(3, 0, true), 0.0);

    let bundle_patch = DataComponentPatchSummary {
        bundle_contents_items: vec![ItemStackTemplateSummary {
            item_id: 1,
            count: 2,
            component_patch: patch.into(),
        }],
        ..DataComponentPatchSummary::default()
    };
    let fullness = bundle_fullness_value(Some(&bundle_patch), None);
    assert!(fullness.is_finite() && fullness >= 0.0);
}

#[test]
fn composite_cuboid_item_shares_single_baked_arc_across_repeated_leaves() {
    use bbb_client_assets::{ClientItemDefinition, ItemCuboidModelCatalog, PackRoots, SpriteImage};
    use std::sync::Arc;

    let temp =
        std::env::temp_dir().join(format!("bbb-r21-01-composite-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    let models_dir = temp
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets/minecraft/models/item");
    std::fs::create_dir_all(&models_dir).unwrap();
    std::fs::write(
        models_dir.join("heavy.json"),
        r##"{
            "textures": { "all": "minecraft:block/stone" },
            "elements": [
                { "from": [0, 0, 0], "to": [16, 16, 16], "faces": {
                    "down": { "texture": "#all" },
                    "up": { "texture": "#all" },
                    "north": { "texture": "#all" },
                    "south": { "texture": "#all" },
                    "west": { "texture": "#all" },
                    "east": { "texture": "#all" }
                } }
            ]
        }"##,
    )
    .unwrap();

    let roots = PackRoots::from_root(&temp).unwrap();
    let cuboid_models = ItemCuboidModelCatalog::load(&roots).unwrap();
    let stone_sprite =
        SpriteImage::new("minecraft:block/stone", 16, 16, vec![255; 16 * 16 * 4]).unwrap();
    let textures = ItemTextureState::from_images(vec![stone_sprite]).unwrap();

    let leaves: Vec<serde_json::Value> = (0..256)
        .map(|_| serde_json::json!({ "type": "minecraft:model", "model": "minecraft:item/heavy" }))
        .collect();
    let def_json = serde_json::json!({
        "model": { "type": "minecraft:composite", "models": leaves }
    });
    let definition =
        ClientItemDefinition::from_json_bytes(def_json.to_string().as_bytes()).unwrap();
    let model_tints = HashMap::new();
    let mut cache = CuboidBakeCache::new(None);
    let icon_model = item_icon_model_for_definition_cached(
        &definition.model,
        &cuboid_models,
        &model_tints,
        None,
        &textures,
        &mut cache,
    )
    .expect("composite caches cleanly");

    assert_eq!(cache.unique_baked_geometry_count(), 1);
    let ItemIconModel::Composite(children) = &icon_model else {
        panic!("expected composite model");
    };
    assert_eq!(children.len(), 256);
    let first_cuboid = match &children[0] {
        ItemIconModel::Layers {
            cuboid: Some(parts),
            ..
        } => parts,
        other => panic!("expected cuboid leaf, got {other:?}"),
    };
    assert!(first_cuboid[0].ground_model_extents.is_some());
    for child in &children[1..] {
        let ItemIconModel::Layers {
            cuboid: Some(parts),
            ..
        } = child
        else {
            panic!("expected cuboid leaf");
        };
        assert!(
            Arc::ptr_eq(first_cuboid, parts),
            "repeated composite leaves must share the same Arc<[CuboidPart]>"
        );
    }
    let _ = std::fs::remove_dir_all(&temp);
}
