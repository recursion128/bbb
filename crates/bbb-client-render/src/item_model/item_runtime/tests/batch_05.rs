use super::*;

#[test]
fn native_item_runtime_uses_item_model_component_as_root_model() {
    let root = unique_temp_dir("item-runtime-item-model-component");
    write_item_model_component_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = |component_patch| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch,
    };

    // Vanilla `Item.Properties.finalizeInitializer` defaults ITEM_MODEL to
    // the item's own id, so an unpatched stack uses the item definition.
    assert_eq!(
        runtime
            .resolve_item_model(
                &stack(DataComponentPatchSummary::default().into()),
                &ItemModelContext::new(BlockModelDisplayContext::Gui)
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv,
        uv("model_component")
    );

    // `ItemModelResolver.appendItemLayers` reads the effective
    // DataComponents.ITEM_MODEL value for the root item model.
    assert_eq!(
        runtime
            .resolve_item_model(
                &stack(
                    patch_with(&[10], |p| p.item_model =
                        Some("minecraft:alternate_model_component".to_string()))
                    .into()
                ),
                &ItemModelContext::new(BlockModelDisplayContext::Gui)
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv,
        uv("alternate_model_component")
    );
    assert_eq!(
        runtime
            .item_particle_sprites_for_stack(&stack(
                patch_with(&[10], |p| p.item_model =
                    Some("minecraft:alternate_model_component".to_string()))
                .into()
            ))
            .map(|sprites| sprites.to_vec()),
        Some(vec![
            runtime.texture_index("minecraft:item/alternate_model_component")
        ])
    );

    // Removing ITEM_MODEL makes ItemStack.get(ITEM_MODEL) return null; the
    // vanilla resolver clears the output and appends no item layers.
    assert!(runtime
        .resolve_item_model(
            &stack(patch_removed(&[10]).into()),
            &ItemModelContext::new(BlockModelDisplayContext::Gui)
        )
        .and_then(|model| model.flat_icon())
        .is_none());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_preserves_selected_special_leaves_and_composed_transformations() {
    let root = unique_temp_dir("item-runtime-special-leaf");
    write_special_item_model_leaf_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = |selected: bool| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            custom_model_data_flags: vec![selected],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };

    let selected_stack = stack(true);
    let resolved = runtime
        .resolve_item_model(
            &selected_stack,
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .unwrap();
    assert_eq!(resolved.layers.len(), 2);
    let ResolvedItemModelLayer::Ordinary {
        icon,
        transformation,
        display_transform,
    } = &resolved.layers[0]
    else {
        panic!("first composite leaf should stay ordinary");
    };
    assert_eq!(icon.layers[0].uv, uv("special_ordinary"));
    assert_eq!(
        display_transform
            .expect("ordinary base model supplies GUI properties")
            .translation,
        [1.0 / 16.0, 0.0, 0.0]
    );
    assert_eq!(
        transformation.columns,
        [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0, 0.0],
            [0.0, 0.0, 2.0, 0.0],
            [2.0, 3.0, 4.0, 1.0],
        ]
    );
    let ResolvedItemModelLayer::Special {
        model,
        transformation,
        display_transform,
        ..
    } = &resolved.layers[1]
    else {
        panic!("selected condition leaf should stay special");
    };
    assert!(matches!(model, ItemSpecialModel::DecoratedPot));
    assert_eq!(
        display_transform
            .expect("special wrapper base model supplies GUI properties")
            .translation,
        [5.0 / 16.0, 0.0, 0.0]
    );
    assert_eq!(
        transformation.columns,
        [
            [3.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 3.0, 0.0],
            [7.0, 3.0, 4.0, 1.0],
        ]
    );

    // The flat view of the same leaves uses the special wrapper's base sprite.
    let compatibility = runtime
        .resolve_item_model(
            &selected_stack,
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(compatibility.layers.len(), 2);
    assert_eq!(compatibility.layers[0].uv, uv("special_ordinary"));
    assert_eq!(compatibility.layers[1].uv, uv("special_base"));

    let fallback = runtime
        .resolve_item_model(
            &stack(false),
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .unwrap();
    assert!(matches!(
        &fallback.layers[1],
        ResolvedItemModelLayer::Ordinary {
            icon,
            transformation,
            ..
        }
            if icon.layers[0].uv == uv("special_fallback")
                && transformation.columns[3] == [7.0, 3.0, 4.0, 1.0]
    ));

    let bundle_stack = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: patch_with(
            &[component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
            |p| p.bundle_contents_items = vec![plain_template_stack(2, 1)],
        )
        .into(),
    };
    let nested = runtime
        .resolve_item_model(
            &bundle_stack,
            &ItemModelContext {
                bundle_selected_item_index: Some(0),
                ..ItemModelContext::new(BlockModelDisplayContext::Gui)
            },
        )
        .unwrap();
    assert_eq!(
        nested.layers,
        vec![ResolvedItemModelLayer::Special {
            model: ItemSpecialModel::Trident,
            transformation: ItemModelLeafTransformation {
                columns: [
                    [4.0, 0.0, 0.0, 0.0],
                    [0.0, 4.0, 0.0, 0.0],
                    [0.0, 0.0, 4.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            },
            display_transform: Some(BlockModelDisplayTransform::default()),
            base_icon: nested.flat_icon(),
        }]
    );
    assert_eq!(
        runtime
            .resolve_item_model(
                &bundle_stack,
                &ItemModelContext {
                    bundle_selected_item_index: Some(0),
                    ..ItemModelContext::new(BlockModelDisplayContext::Gui)
                }
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv,
        uv("nested_special_base")
    );

    let decomposed = runtime
        .resolve_item_model(
            &ItemStackSummary {
                item_id: Some(3),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .unwrap();
    let [ResolvedItemModelLayer::Special { transformation, .. }] = decomposed.layers.as_slice()
    else {
        panic!("decomposed static special definition should remain a special leaf");
    };
    let expected = [
        [0.0, 2.0, 0.0, 0.0],
        [0.0, 0.0, 4.0, 0.0],
        [3.0, 0.0, 0.0, 0.0],
        [1.0, 2.0, 3.0, 1.0],
    ];
    for (actual, expected) in transformation
        .columns
        .iter()
        .flatten()
        .zip(expected.iter().flatten())
    {
        assert!(
            (actual - expected).abs() <= 1.0e-5,
            "decomposed T * left * scale * right mismatch: {actual} != {expected}"
        );
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_display_context_select() {
    let root = unique_temp_dir("item-runtime-display-context");
    write_display_context_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let rect = |model_id: &str| {
        let uv = uv(model_id);
        ItemSpriteRect {
            min: uv.min,
            max: uv.max,
        }
    };
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };

    // GUI/HUD item icons pass vanilla ItemDisplayContext.GUI.
    assert_eq!(
        runtime
            .resolve_item_model(
                &stack,
                &ItemModelContext::new(BlockModelDisplayContext::Gui)
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv,
        uv("display_gui")
    );

    // Non-living generated consumers pass their actual world contexts.
    assert_eq!(
        runtime
            .resolve_item_model(
                &stack,
                &ItemModelContext::new(BlockModelDisplayContext::Ground)
            )
            .and_then(|model| model.flat_icon())
            .map(|icon| runtime.generated_item_layers_for_icon(&icon))
            .unwrap_or_default()
            .into_iter()
            .next()
            .unwrap()
            .rect,
        rect("display_ground")
    );
    assert_eq!(
        runtime
            .resolve_item_model(
                &stack,
                &ItemModelContext::new(BlockModelDisplayContext::Fixed)
            )
            .and_then(|model| model.flat_icon())
            .map(|icon| runtime.generated_item_layers_for_icon(&icon))
            .unwrap_or_default()
            .into_iter()
            .next()
            .unwrap()
            .rect,
        rect("display_fixed")
    );

    // Entity-owned generated held items pass their hand display context.
    assert_eq!(
        runtime
            .resolve_item_model(
                &stack,
                &ItemModelContext::new(BlockModelDisplayContext::ThirdPersonRightHand)
            )
            .and_then(|model| model.flat_icon())
            .map(|icon| runtime.generated_item_layers_for_icon(&icon))
            .unwrap_or_default()
            .into_iter()
            .next()
            .unwrap()
            .rect,
        rect("display_thirdperson_right")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_trim_material_select() {
    let root = unique_temp_dir("item-runtime-trim-material");
    write_trim_material_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    // Trim-material registry keys by holder id (registration order).
    let trim_keys = [
        "minecraft:quartz".to_string(),
        "minecraft:iron".to_string(),
        "minecraft:diamond".to_string(),
    ];
    let trimmed = |material_id: Option<i32>| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            armor_trim_material_id: material_id,
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let selected = |stack: &ItemStackSummary| {
        runtime
            .resolve_item_model(
                stack,
                &ItemModelContext {
                    registry_keys: ItemModelRegistryKeys {
                        trim_materials: Some(&trim_keys),
                        ..Default::default()
                    },
                    ..ItemModelContext::new(BlockModelDisplayContext::Gui)
                },
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };

    // No trim component → no match → fallback (plain chestplate).
    assert_eq!(selected(&trimmed(None)), uv("iron_chestplate"));
    // Holder id 0 → "minecraft:quartz" → quartz trim model.
    assert_eq!(
        selected(&trimmed(Some(0))),
        uv("iron_chestplate_quartz_trim")
    );
    // Holder id 2 → "minecraft:diamond" → diamond trim model.
    assert_eq!(
        selected(&trimmed(Some(2))),
        uv("iron_chestplate_diamond_trim")
    );
    // Holder id 1 → "minecraft:iron" (no case) → fallback.
    assert_eq!(selected(&trimmed(Some(1))), uv("iron_chestplate"));
    // Without the registry keys (no world context) → fallback.
    assert_eq!(
        runtime
            .resolve_item_model(
                &trimmed(Some(0)),
                &ItemModelContext::new(BlockModelDisplayContext::Gui)
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv,
        uv("iron_chestplate")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_charge_type_select() {
    let root = unique_temp_dir("item-runtime-charge-type");
    write_charge_type_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    // A charged-projectiles template list with one entry of the given item id.
    let charged = |item_id: i32| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            charged_projectiles_items: vec![ItemStackTemplateSummary {
                item_id,
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let selected = |stack: &ItemStackSummary| {
        runtime
            .resolve_item_model(stack, &ItemModelContext::new(BlockModelDisplayContext::Gui))
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };

    // Empty crossbow → NONE → no matching case → fallback (plain crossbow).
    assert_eq!(
        selected(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        uv("crossbow")
    );
    // Charged with an arrow (item 2) → ARROW → "arrow" case.
    assert_eq!(selected(&charged(2)), uv("crossbow_arrow"));
    // Charged with a firework_rocket (item 1) → ROCKET → "rocket" case.
    assert_eq!(selected(&charged(1)), uv("crossbow_firework"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_use_tick_range_dispatch_properties() {
    let root = unique_temp_dir("item-runtime-use-tick-range-dispatch");
    write_use_tick_range_dispatch_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = |item_id: i32| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let selected = |stack: &ItemStackSummary, using_item: bool, elapsed_ticks: u32| {
        let use_context = if using_item {
            runtime.item_model_use_context_for_stack(stack, elapsed_ticks)
        } else {
            ItemModelUseContext::inactive()
        };
        runtime
            .resolve_item_model(
                stack,
                &ItemModelContext {
                    using_item,
                    use_context,
                    ..ItemModelContext::new(BlockModelDisplayContext::Gui)
                },
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };
    let selected_with_crossbow_charge_duration =
        |stack: &ItemStackSummary,
         using_item: bool,
         elapsed_ticks: u32,
         crossbow_charge_duration_ticks: i32| {
            let use_context = if using_item {
                runtime.item_model_use_context_for_stack_with_crossbow_charge_duration(
                    stack,
                    elapsed_ticks,
                    Some(crossbow_charge_duration_ticks),
                )
            } else {
                ItemModelUseContext::inactive()
            };
            runtime
                .resolve_item_model(
                    stack,
                    &ItemModelContext {
                        using_item,
                        use_context,
                        ..ItemModelContext::new(BlockModelDisplayContext::Gui)
                    },
                )
                .and_then(|model| model.flat_icon())
                .unwrap()
                .layers[0]
                .uv
        };

    let bow = stack(0);
    assert_eq!(selected(&bow, false, 20), uv("bow"));
    assert_eq!(selected(&bow, true, 0), uv("bow_pulling_0"));
    assert_eq!(selected(&bow, true, 13), uv("bow_pulling_1"));
    assert_eq!(selected(&bow, true, 18), uv("bow_pulling_2"));

    let crossbow = stack(1);
    assert_eq!(selected(&crossbow, false, 25), uv("crossbow"));
    assert_eq!(selected(&crossbow, true, 0), uv("crossbow_pulling_0"));
    assert_eq!(selected(&crossbow, true, 15), uv("crossbow_pulling_1"));
    assert_eq!(selected(&crossbow, true, 25), uv("crossbow_pulling_2"));
    let quick_charge_crossbow = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary {
            enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 1,
                level: 2,
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    // The caller-projected synchronized effect result is 15 ticks, so elapsed
    // 10 ticks crosses the 0.58 `crossbow/pull` threshold; without that world
    // projection, the same stack still uses the 25 tick default.
    assert_eq!(
        selected(&quick_charge_crossbow, true, 10),
        uv("crossbow_pulling_0")
    );
    assert_eq!(
        selected_with_crossbow_charge_duration(&quick_charge_crossbow, true, 10, 15),
        uv("crossbow_pulling_1")
    );
    let charged_crossbow = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary {
            charged_projectiles_items: vec![plain_template_stack(3, 1)],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(selected(&charged_crossbow, true, 25), uv("crossbow_arrow"));

    let brush = stack(4);
    assert_eq!(selected(&brush, false, 1), uv("brush"));
    assert_eq!(selected(&brush, true, 0), uv("brush"));
    assert_eq!(selected(&brush, true, 7), uv("brush_brushing_0"));
    assert_eq!(selected(&brush, true, 1), uv("brush_brushing_2"));

    let consumable_apple = ItemStackSummary {
        item_id: Some(5),
        count: 1,
        component_patch: DataComponentPatchSummary {
            consumable: Some(ConsumableSummary {
                consume_seconds: 0.8,
                animation: ItemUseAnimationSummary::Eat,
            }),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(selected(&consumable_apple, false, 0), uv("apple"));
    // Vanilla `Consumable.consumeTicks()` casts `consumeSeconds * 20.0F` to
    // int, so 0.8 seconds yields a 16 tick remaining-time source.
    assert_eq!(
        selected(&consumable_apple, true, 0),
        uv("apple_remaining_high")
    );
    assert_eq!(
        selected(&consumable_apple, true, 10),
        uv("apple_remaining_low")
    );

    let ender_eye = stack(6);
    assert_eq!(selected(&ender_eye, false, 0), uv("ender_eye"));
    // Vanilla 26.1 `EnderEyeItem.getUseDuration` returns 0, so even an
    // active use context has no remaining ticks for range-dispatch.
    assert_eq!(
        selected(&ender_eye, true, 0),
        uv("ender_eye_remaining_empty")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_main_hand_select_from_owner_context() {
    let root = unique_temp_dir("item-runtime-main-hand-select");
    write_main_hand_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let selected = |owner_main_hand_left| {
        runtime
            .resolve_item_model(
                &stack,
                &ItemModelContext {
                    owner_main_hand_left,
                    ..ItemModelContext::new(BlockModelDisplayContext::Gui)
                },
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };

    // Vanilla `MainHand.get` returns null when there is no living owner, so
    // no case matches and the fallback model is used.
    assert_eq!(selected(None), uv("hand_selector"));
    assert_eq!(selected(Some(false)), uv("hand_selector_right"));
    assert_eq!(selected(Some(true)), uv("hand_selector_left"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_stack_string_select_properties() {
    let root = unique_temp_dir("item-runtime-stack-string-select");
    write_stack_string_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |stack: &ItemStackSummary| {
        runtime
            .resolve_item_model(stack, &ItemModelContext::new(BlockModelDisplayContext::Gui))
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };

    let block_state_stack = |properties: BTreeMap<String, String>| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            block_state_properties: properties
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    // Vanilla `ItemBlockState.get` returns null without the component/property,
    // so no case matches and the fallback model is used.
    assert_eq!(
        selected(&block_state_stack(BTreeMap::new())),
        uv("beehive_empty")
    );
    assert_eq!(
        selected(&block_state_stack(BTreeMap::from([(
            "honey_level".to_string(),
            "5".to_string()
        )]))),
        uv("beehive_honey")
    );
    assert_eq!(
        selected(&block_state_stack(BTreeMap::from([(
            "wrong_property".to_string(),
            "5".to_string()
        )]))),
        uv("beehive_empty")
    );

    let custom_model_data_stack = |strings: Vec<&str>| ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary {
            custom_model_data_strings: strings.into_iter().map(Into::into).collect(),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    // Vanilla `CustomModelDataProperty.get` reads strings[index], not floats.
    assert_eq!(
        selected(&custom_model_data_stack(vec!["ignored", "blue"])),
        uv("cmd_blue")
    );
    assert_eq!(
        selected(&custom_model_data_stack(vec!["ignored", "lime"])),
        uv("cmd_green")
    );
    // Out-of-range index and absent component both produce no selected value.
    assert_eq!(
        selected(&custom_model_data_stack(vec!["blue"])),
        uv("cmd_plain")
    );
    assert_eq!(
        selected(&custom_model_data_stack(Vec::new())),
        uv("cmd_plain")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_custom_model_data_condition_flags() {
    let root = unique_temp_dir("item-runtime-custom-model-data-condition");
    write_custom_model_data_condition_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |flags: Vec<bool>, removed: bool| {
        runtime
            .resolve_item_model(
                &ItemStackSummary {
                    item_id: Some(0),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        custom_model_data_flags: flags,
                        removed_type_ids: if removed { vec![17] } else { Vec::new() },
                        ..DataComponentPatchSummary::default()
                    }
                    .into(),
                },
                &ItemModelContext::new(BlockModelDisplayContext::Gui),
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };

    // Vanilla conditional `CustomModelDataProperty.get` reads
    // `CustomModelData.flags[index] == true`, not floats or strings.
    assert_eq!(selected(Vec::new(), false), uv("cmd_flag_false"));
    assert_eq!(selected(vec![true], false), uv("cmd_flag_false"));
    assert_eq!(selected(vec![false, true], false), uv("cmd_flag_true"));
    assert_eq!(selected(vec![true, false], false), uv("cmd_flag_false"));
    assert_eq!(selected(vec![false, true], true), uv("cmd_flag_false"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_local_time_select_property() {
    let root = unique_temp_dir("item-runtime-local-time-select");
    write_local_time_select_fixture(&root);

    fn format_test_local_time(
        epoch_millis: i64,
        pattern: &str,
        locale: &str,
        time_zone: Option<&str>,
    ) -> Option<String> {
        match (epoch_millis, pattern, locale, time_zone) {
            (1_798_156_800_000, "MM-dd", "", Some("GMT")) => Some("12-25".to_string()),
            (1_798_329_600_000, "MM-dd", "", Some("GMT")) => Some("12-27".to_string()),
            _ => None,
        }
    }

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap())
        .unwrap()
        .with_local_time_formatter(format_test_local_time);
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let selected = || {
        runtime
            .resolve_item_model(
                &stack,
                &ItemModelContext::new(BlockModelDisplayContext::Gui),
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };

    runtime.set_local_time_epoch_millis_for_test(1_798_156_800_000);
    assert_eq!(selected(), uv("seasonal_chest_christmas"));

    runtime.set_local_time_epoch_millis_for_test(1_798_329_600_000);
    assert_eq!(selected(), uv("seasonal_chest_normal"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_component_select_values() {
    let h = FixtureRuntimeHarness::load(
        "item-runtime-component-select",
        write_component_select_fixture,
    );
    let rect = |model_id: &str| {
        let uv = h.uv(model_id);
        ItemSpriteRect {
            min: uv.min,
            max: uv.max,
        }
    };

    // Vanilla `COMMON_ITEM_COMPONENTS` gives every item `rarity=common`.
    h.assert_profiled_models(
        0,
        [
            (
                DataComponentPatchSummary::default(),
                "component_rarity_common",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID],
                    |p| p.rarity = Some(ItemRaritySummary::Rare),
                ),
                "component_rarity_rare",
            ),
            (
                DataComponentPatchSummary {
                    rarity: Some(ItemRaritySummary::Rare),
                    removed_type_ids: vec![12],
                    ..DataComponentPatchSummary::default()
                },
                "component_rarity_fallback",
            ),
        ],
    );

    // `max_stack_size` is another common default component; numeric cases
    // must match JSON numbers, not strings.
    h.assert_profiled_models(
        1,
        [
            (
                DataComponentPatchSummary::default(),
                "component_stack_size_64",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                    |p| p.max_stack_size = Some(16),
                ),
                "component_stack_size_16",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                    |p| {
                        p.max_stack_size = Some(16);
                        p.removed_type_ids = vec![1];
                    },
                ),
                "component_stack_size_fallback",
            ),
        ],
    );

    // `enchantment_glint_override` has no common default, so the unset
    // stack falls through while explicit true/false cases both match.
    h.assert_profiled_models(
        2,
        [
            (
                DataComponentPatchSummary::default(),
                "component_glint_fallback",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID],
                    |p| p.enchantment_glint_override = Some(true),
                ),
                "component_glint_true",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID],
                    |p| p.enchantment_glint_override = Some(false),
                ),
                "component_glint_false",
            ),
            (
                DataComponentPatchSummary {
                    enchantment_glint_override: Some(true),
                    removed_type_ids: vec![21],
                    ..DataComponentPatchSummary::default()
                },
                "component_glint_fallback",
            ),
        ],
    );

    // Damageable item defaults project through the item registry: damage=0
    // and max_damage=432 until overridden or removed.
    h.assert_profiled_models(
        3,
        [
            (DataComponentPatchSummary::default(), "component_damage_0"),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
                    |p| p.damage = Some(7),
                ),
                "component_damage_7",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
                    |p| {
                        p.damage = Some(7);
                        p.removed_type_ids = vec![3];
                    },
                ),
                "component_damage_fallback",
            ),
        ],
    );
    h.assert_profiled_models(
        4,
        [
            (
                DataComponentPatchSummary::default(),
                "component_max_damage_432",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID],
                    |p| p.max_damage = Some(99),
                ),
                "component_max_damage_99",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID],
                    |p| {
                        p.max_damage = Some(99);
                        p.removed_type_ids = vec![2];
                    },
                ),
                "component_max_damage_fallback",
            ),
        ],
    );

    // `Item.Properties.finalizeInitializer` defaults ITEM_MODEL to the
    // item's own id; an explicit item_model patch changes the root model and
    // the component value seen by `ComponentContents.get`.
    h.assert_profiled_models(
        5,
        [
            (
                DataComponentPatchSummary::default(),
                "component_item_model_default",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID],
                    |p| {
                        p.item_model =
                            Some("minecraft:item_model_component_selector_alt_root".to_string())
                    },
                ),
                "component_item_model_alt",
            ),
        ],
    );
    assert!(h
        .runtime
        .resolve_item_model(
            &ItemStackSummary {
                item_id: Some(5),
                count: 1,
                component_patch: patch_removed(&[10]).into(),
            },
            &h.base_context()
        )
        .and_then(|model| model.flat_icon())
        .is_none());

    // `DataComponents.MAP_ID` wraps an int (`MapId.CODEC`), so component
    // select cases match JSON numbers and removed id 46 suppresses it.
    h.assert_profiled_models(
        6,
        [
            (
                DataComponentPatchSummary::default(),
                "component_map_id_fallback",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID],
                    |p| p.map_id = Some(123),
                ),
                "component_map_id_123",
            ),
            (
                DataComponentPatchSummary {
                    map_id: Some(123),
                    removed_type_ids: vec![46],
                    ..DataComponentPatchSummary::default()
                },
                "component_map_id_fallback",
            ),
        ],
    );

    // `DyedItemColor.CODEC` and `MapItemColor.CODEC` expose their RGB ints
    // to `ComponentContents.get`, with no common default component.
    h.assert_profiled_models(
        7,
        [
            (
                DataComponentPatchSummary::default(),
                "component_dyed_color_fallback",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID],
                    |p| p.dyed_color = Some(0x12_34_56),
                ),
                "component_dyed_color_123456",
            ),
            (
                DataComponentPatchSummary {
                    dyed_color: Some(0x12_34_56),
                    removed_type_ids: vec![44],
                    ..DataComponentPatchSummary::default()
                },
                "component_dyed_color_fallback",
            ),
        ],
    );
    h.assert_profiled_models(
        8,
        [
            (
                DataComponentPatchSummary::default(),
                "component_map_color_fallback",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID],
                    |p| p.map_color = Some(0x45_67_89),
                ),
                "component_map_color_456789",
            ),
            (
                DataComponentPatchSummary {
                    map_color: Some(0x45_67_89),
                    removed_type_ids: vec![45],
                    ..DataComponentPatchSummary::default()
                },
                "component_map_color_fallback",
            ),
        ],
    );

    // `DataComponents.CUSTOM_NAME` uses ComponentSerialization.CODEC;
    // this pins JSON-string / {"text": "..."} literal cases and removed id
    // 6 suppression.
    h.assert_profiled_models(
        9,
        [
            (
                DataComponentPatchSummary::default(),
                "component_custom_name_fallback",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
                    |p| p.custom_name = Some("Named Selector".to_string()),
                ),
                "component_custom_name_named",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
                    |p| p.custom_name = Some("Object Selector".to_string()),
                ),
                "component_custom_name_object",
            ),
            (
                DataComponentPatchSummary {
                    custom_name: Some("Named Selector".to_string()),
                    removed_type_ids: vec![6],
                    ..DataComponentPatchSummary::default()
                },
                "component_custom_name_fallback",
            ),
        ],
    );

    // `DataComponents.ITEM_NAME` also uses ComponentSerialization.CODEC,
    // and `Item.Properties.finalizeInitializer` contributes the default
    // translatable item-name component. Literal strings must not match that
    // default translatable key.
    h.assert_profiled_models(
        10,
        [
            (
                DataComponentPatchSummary::default(),
                "component_item_name_default",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID],
                    |p| p.item_name = Some("Patched Item Name".to_string()),
                ),
                "component_item_name_literal",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID],
                    |p| p.item_name = Some("Object Item Name".to_string()),
                ),
                "component_item_name_object",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID],
                    |p| {
                        p.item_name =
                            Some("item.minecraft.item_name_component_selector".to_string())
                    },
                ),
                "component_item_name_literal_key",
            ),
            (patch_removed(&[9]), "component_item_name_fallback"),
        ],
    );
    h.assert_profiled_models(
        11,
        [
            (
                DataComponentPatchSummary::default(),
                "component_item_name_block_default",
            ),
            (patch_removed(&[9]), "component_item_name_fallback"),
        ],
    );

    for (item_id, model_id) in [
        (10, "component_item_name_default"),
        (11, "component_item_name_block_default"),
    ] {
        assert_eq!(
            h.runtime
                .resolve_item_model(
                    &ItemStackSummary {
                        item_id: Some(item_id),
                        count: 1,
                        component_patch: DataComponentPatchSummary::default().into(),
                    },
                    &ItemModelContext::new(BlockModelDisplayContext::Gui),
                )
                .and_then(|model| model.flat_icon())
                .unwrap()
                .layers[0]
                .uv,
            h.uv(model_id)
        );
    }

    // `DamageType.STREAM_CODEC` carries a registry holder. Prototype spear
    // defaults use the pack-derived key; explicit patches resolve through the
    // current session holder order, and removal suppresses the prototype.
    h.assert_profiled_models(
        12,
        [(
            DataComponentPatchSummary::default(),
            "component_damage_type_spear",
        )],
    );
    struct TestDamageTypeResolver;
    impl bbb_protocol::PersistentRegistryResolver for TestDamageTypeResolver {
        fn registry_available(&self, registry: &str) -> bool {
            registry == "minecraft:damage_type"
        }
        fn resource_membership(
            &self,
            registry: &str,
            resource_id: &str,
        ) -> bbb_protocol::PersistentRegistryMembership {
            if registry == "minecraft:damage_type"
                && matches!(resource_id, "minecraft:spear" | "example:custom")
            {
                bbb_protocol::PersistentRegistryMembership::Present
            } else if registry == "minecraft:damage_type" {
                bbb_protocol::PersistentRegistryMembership::Absent
            } else {
                bbb_protocol::PersistentRegistryMembership::Unavailable
            }
        }
        fn tag_members(
            &self,
            _registry: &str,
            _tag_id: &str,
        ) -> bbb_protocol::PersistentRegistryTagMembers {
            bbb_protocol::PersistentRegistryTagMembers::Unavailable
        }
    }
    impl bbb_registry_codec::PersistentItemStackHolderResolver for TestDamageTypeResolver {
        fn holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
            (registry == "minecraft:damage_type").then_some(match resource_id {
                "minecraft:spear" => Some(0),
                "example:custom" => Some(1),
                _ => None,
            })?
        }
        fn holder_resource_id(&self, registry: &str, holder_id: i32) -> Option<String> {
            (registry == "minecraft:damage_type").then(|| match holder_id {
                0 => Some("minecraft:spear".to_string()),
                1 => Some("example:custom".to_string()),
                _ => None,
            })?
        }
        fn tag_holder_ids(&self, _registry: &str, tag: &str) -> Result<Vec<i32>, String> {
            Err(format!("unexpected tag {tag}"))
        }
    }
    let resolver = TestDamageTypeResolver;
    let explicit = ItemStackSummary {
        item_id: Some(12),
        count: 1,
        component_patch: patch_with(&[8], |p| p.damage_type_id = Some(1)).into(),
    };
    assert_eq!(
        h.runtime
            .resolve_item_model(
                &explicit,
                &ItemModelContext {
                    registry_access: Some((
                        bbb_registry_codec::RegistryAccessRevision {
                            session: 1,
                            contents: 0,
                            tags: 0,
                        },
                        &resolver,
                    )),
                    ..h.base_context()
                }
            )
            .and_then(|model| model.flat_icon())
            .map(|icon| h.runtime.generated_item_layers_for_icon(&icon))
            .unwrap_or_default()
            .into_iter()
            .next()
            .unwrap()
            .rect,
        rect("component_damage_type_custom")
    );
    h.assert_profiled_models(
        12,
        [(patch_removed(&[8]), "component_damage_type_fallback")],
    );

    // Descriptor-backed cases retain full object/list values and float bit
    // identity instead of collapsing them through the former scalar enum.
    h.assert_profiled_models(
        13,
        [
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID],
                    |p| {
                        p.custom_model_data_floats = vec![0.25, 1.5].into();
                        p.custom_model_data_flags = vec![true, false];
                        p.custom_model_data_strings = vec!["first".into(), "second".into()];
                        p.custom_model_data_colors = vec![0x12_34_56];
                    },
                ),
                "component_custom_model_data_match",
            ),
            (
                patch_with(
                    &[component_id::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID],
                    |p| {
                        p.custom_model_data_floats = vec![0.25, 1.5].into();
                        p.custom_model_data_flags = vec![true, false];
                        p.custom_model_data_strings = vec!["first".into(), "different".into()];
                        p.custom_model_data_colors = vec![0x12_34_56];
                    },
                ),
                "component_custom_model_data_fallback",
            ),
            (
                patch_removed(&[component_id::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID]),
                "component_custom_model_data_fallback",
            ),
        ],
    );
    h.assert_profiled_models(
        14,
        [(
            patch_with(
                &[component_id::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
                |p| p.minimum_attack_charge_bits = Some(0.125_f32.to_bits()),
            ),
            "component_minimum_attack_charge_match",
        )],
    );

    let lore = bbb_registry_codec::decode_persistent_component_select_value_with_resolver(
        component_id::VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        &serde_json::json!([
            {"text": "Styled", "bold": true},
            "Plain"
        ]),
        None,
    )
    .unwrap()
    .unwrap();
    h.assert_profiled_models(15, [(lore, "component_lore_match")]);

    let mut catalog: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(item_registry_override_path(&h.root)).unwrap(),
    )
    .unwrap();
    for (prototype, model_id) in [
        (
            serde_json::json!("Patched Item Name"),
            "component_item_name_literal",
        ),
        (
            serde_json::json!({
                "translate": "example.prototype_name",
                "with": [{"text": "Argument", "bold": true}],
                "color": "gold",
                "extra": [{"text": " suffix", "italic": true}],
            }),
            "component_item_name_object",
        ),
    ] {
        catalog["default_component_codec_values"]["minecraft:item_name_component_selector"]
            [component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID.to_string()] = prototype;
        write_item_registry_override_value(&h.root, &catalog);
        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&h.root).unwrap()).unwrap();
        let icon = runtime
            .resolve_item_model(
                &ItemStackSummary {
                    item_id: Some(10),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
                &ItemModelContext::new(BlockModelDisplayContext::Gui),
            )
            .and_then(|model| model.flat_icon())
            .unwrap();
        assert_eq!(
            icon.layers[0].uv,
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
                .unwrap()
        );
    }
}

#[test]
fn native_item_runtime_resolves_value_aware_range_dispatch() {
    let root = unique_temp_dir("item-runtime-range-dispatch");
    write_value_aware_range_dispatch_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let defaults = build_component_defaults(BTreeMap::from([(
        3,
        BTreeMap::from([(
            component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            serde_json::json!([]),
        )]),
    )]));
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let damage_stack = |damage: Option<i32>, max_damage: Option<i32>| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            damage,
            max_damage,
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let selected = |stack: &ItemStackSummary| {
        runtime
            .resolve_item_model(
                stack,
                &ItemModelContext {
                    item_profiles: Some(defaults.as_view()),
                    ..ItemModelContext::new(BlockModelDisplayContext::Gui)
                },
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };

    // damage 50/100 = 0.5 lands exactly on the 0.5 threshold (vanilla
    // `lastIndexLessOrEqual` is inclusive), proving sort + boundary.
    assert_eq!(
        selected(&damage_stack(Some(50), Some(100))),
        uv("damage_half")
    );
    // damage 95/100 = 0.95 reaches the top entry.
    assert_eq!(
        selected(&damage_stack(Some(95), Some(100))),
        uv("damage_low")
    );
    // damage 40/100 = 0.4 precedes the first threshold (0.5) → fallback (-1).
    assert_eq!(
        selected(&damage_stack(Some(40), Some(100))),
        uv("damage_fallback")
    );
    // No max_damage → 0/0 = NaN → fallback.
    assert_eq!(selected(&damage_stack(None, None)), uv("damage_fallback"));

    let cmd_stack = |floats: Vec<f32>| ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: patch_with(
            &[component_id::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID],
            |p| p.custom_model_data_floats = floats.into(),
        )
        .into(),
    };
    // floats[1] = 0.5, scale 2.0 → 1.0 lands on the 1.0 threshold; floats[0]
    // is ignored (index 1), proving index handling, scale, and boundary.
    assert_eq!(selected(&cmd_stack(vec![9.0, 0.5])), uv("cmd_1"));
    // floats[1] = 2.0 * 2.0 = 4.0 reaches the 3.0 entry.
    assert_eq!(selected(&cmd_stack(vec![9.0, 2.0])), uv("cmd_3"));
    // Missing index 1 → 0.0 → the 0.0 entry.
    assert_eq!(selected(&cmd_stack(vec![9.0])), uv("cmd_0"));
    // No custom_model_data at all → 0.0 → the 0.0 entry.
    assert_eq!(selected(&cmd_stack(Vec::new())), uv("cmd_0"));

    let count_stack = |count: i32, max_stack_size: Option<i32>| ItemStackSummary {
        item_id: Some(2),
        count,
        component_patch: DataComponentPatchSummary {
            max_stack_size,
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    // Count.get defaults to normalized `count / maxStackSize`, using the
    // item prototype default when the component is absent.
    assert_eq!(selected(&count_stack(32, None)), uv("count_half"));
    assert_eq!(selected(&count_stack(64, None)), uv("count_full"));
    assert_eq!(selected(&count_stack(1, None)), uv("count_fallback"));
    // The max_stack_size component overrides the prototype default.
    assert_eq!(selected(&count_stack(8, Some(16))), uv("count_half"));

    let apple_template = |count: i32| ItemStackTemplateSummary {
        item_id: 4,
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let bundle_stack = |items: Vec<ItemStackTemplateSummary>| ItemStackSummary {
        item_id: Some(3),
        count: 1,
        component_patch: patch_with(
            &[component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
            |p| p.bundle_contents_items = items,
        )
        .into(),
    };
    // BundleFullness.get sums BundleContents weights; regular items weigh
    // `count / getMaxStackSize`.
    assert_eq!(
        selected(&bundle_stack(vec![apple_template(32)])),
        uv("bundle_half")
    );
    assert_eq!(
        selected(&bundle_stack(vec![apple_template(16)])),
        uv("bundle_fallback")
    );
    // A nested bundle weighs its contents plus the fixed 1/16 bundle item weight.
    assert_eq!(
        selected(&bundle_stack(vec![template_stack(
            3,
            1,
            patch_with(
                &[component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
                |p| p.bundle_contents_items = vec![apple_template(32)]
            )
        )])),
        uv("bundle_nested")
    );
    let mut nested_default = plain_template_stack(3, 1);
    assert_eq!(
        selected(&bundle_stack(vec![
            apple_template(32),
            nested_default.clone()
        ])),
        uv("bundle_nested")
    );
    nested_default.component_patch.removed_type_ids =
        vec![component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID];
    assert_eq!(
        selected(&bundle_stack(vec![apple_template(32), nested_default])),
        uv("bundle_half")
    );
    // A beehive-like stack with non-empty BEES component weighs as a full bundle.
    assert_eq!(
        selected(&bundle_stack(vec![template_stack(
            5,
            1,
            patch_with(&[component_id::VANILLA_DATA_COMPONENT_BEES_TYPE_ID,], |p| {
                p.bees_present = true;
                p.bees = bee_occupants(1);
            })
        )])),
        uv("bundle_full")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_selects_bundle_icon_from_local_selected_item() {
    let root = unique_temp_dir("item-runtime-bundle-selected");
    write_bundle_selected_item_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let normal_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/bundle"))
        .unwrap();
    let open_back_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/bundle_open_back"))
        .unwrap();
    let open_front_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/bundle_open_front"))
        .unwrap();
    let apple_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/apple"))
        .unwrap();
    let bundle_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: patch_with(
            &[component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
            |p| p.bundle_contents_items = vec![plain_template_stack(1, 1)],
        )
        .into(),
    };

    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/bundle"))
    );

    let default_icon = runtime
        .resolve_item_model(
            &bundle_stack,
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(default_icon.layers[0].uv, normal_uv);

    let unselected_icon = runtime
        .resolve_item_model(
            &bundle_stack,
            &ItemModelContext {
                bundle_selected_item_index: Some(-1),
                ..ItemModelContext::new(BlockModelDisplayContext::Gui)
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(unselected_icon.layers[0].uv, normal_uv);

    let selected_icon = runtime
        .resolve_item_model(
            &bundle_stack,
            &ItemModelContext {
                bundle_selected_item_index: Some(0),
                ..ItemModelContext::new(BlockModelDisplayContext::Gui)
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(
        selected_icon
            .layers
            .iter()
            .map(|layer| layer.uv)
            .collect::<Vec<_>>(),
        vec![open_back_uv, apple_uv, open_front_uv]
    );

    let selected_team_icon = runtime
        .resolve_item_model(
            &bundle_stack,
            &ItemModelContext {
                bundle_selected_item_index: Some(0),
                context_entity_type: Some("minecraft:player"),
                owner_team_color_rgb: Some(0xab_cd_ef),
                view_entity: true,
                registry_access: Some((
                    bbb_registry_codec::RegistryAccessRevision::default(),
                    &bbb_registry_codec::UnavailableHolderResolver,
                )),
                ..ItemModelContext::new(BlockModelDisplayContext::Gui)
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(selected_team_icon.layers[1].tint, rgb_i32_tint(0xab_cd_ef));

    let out_of_bounds_icon = runtime
        .resolve_item_model(
            &bundle_stack,
            &ItemModelContext {
                bundle_selected_item_index: Some(1),
                ..ItemModelContext::new(BlockModelDisplayContext::Gui)
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(out_of_bounds_icon.layers[0].uv, normal_uv);

    let no_contents_icon = runtime
        .resolve_item_model(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            &ItemModelContext {
                bundle_selected_item_index: Some(0),
                ..ItemModelContext::new(BlockModelDisplayContext::Gui)
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(no_contents_icon.layers[0].uv, normal_uv);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_loads_assets_with_empty_registry_override() {
    let root = unique_temp_dir("item-runtime-no-registry");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_override(&root, &[]);
    write_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword"
                }
            }"#,
    );
    write_json(
        &assets.join("models").join("item").join("test_sword.json"),
        r#"{
                "textures": {
                    "layer0": "minecraft:item/test_sword"
                }
            }"#,
    );
    write_test_rgba_png(
        &assets.join("textures").join("item").join("test_sword.png"),
        1,
        1,
        &[255, 0, 0, 255],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

    assert_eq!(runtime.item_definition_count(), 1);
    assert_eq!(runtime.item_registry_count(), 0);
    assert_eq!(runtime.texture_count(), 2);
    assert_eq!(runtime.icon_texture_count(), 1);
    assert!(!runtime.atlas_rgba().is_empty());
    assert_eq!(runtime.icon_texture_index_for_protocol_id(0), None);
    assert_eq!(runtime.icon_uv_for_protocol_id(0), None);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 sources"]
fn loads_local_vanilla_item_runtime_assets() {
    let roots = PackRoots::from_environment().unwrap();
    let runtime = NativeItemRuntime::load(&roots).unwrap();

    assert_eq!(runtime.item_definition_count(), 1506);
    assert_eq!(runtime.item_registry_count(), 1506);
    assert_eq!(runtime.texture_count(), 1576);
    assert_eq!(runtime.icon_texture_count(), 1506);
    assert_eq!(runtime.missing_model_count(), 0);
    assert_eq!(runtime.missing_texture_count(), 0);
    assert!(runtime.icon_uv_for_protocol_id(1).is_some());
    assert!(runtime.resolved_model_count() > runtime.item_definition_count());
    assert!(runtime.atlas_size().0 > 0);
    assert!(runtime.atlas_size().1 > 0);
    roots.ensure_resource_data_admitted().unwrap();
    let observation = roots.resource_data_memory_observation();
    eprintln!("vanilla item runtime resource data charge: {observation:?}");
    assert!(observation.derived_logical_bytes > 0);
    assert!(observation.charged_logical_bytes < observation.max_logical_bytes);
}

#[test]
fn item_runtime_projects_swing_animation_codec_defaults_by_protocol_id() {
    let runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let spear_id = runtime
        .item_protocol_id("minecraft:wooden_spear")
        .expect("wooden spear id");
    let sword_id = runtime.item_protocol_id("minecraft:diamond_sword").unwrap();
    let defaults = runtime.default_component_codec_values_by_protocol_id();
    let swing_id =
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID;

    assert_eq!(
        defaults[&spear_id][&swing_id],
        serde_json::json!({"type": "stab", "duration": 13})
    );
    assert_eq!(defaults[&sword_id][&swing_id], serde_json::json!({}));
}

#[test]
fn native_item_runtime_component_select_preserves_raw_json_block_state_keys() {
    let root = unique_temp_dir("item-runtime-raw-component-select");
    write_component_select_fixture(&root);
    write_json(
        &assets_dir(&root)
            .join("items")
            .join("map_id_component_selector.json"),
        r#"{"model":{"type":"minecraft:select","property":"minecraft:component","component":"minecraft:block_state","cases":[{"when":{"\ud801":"one","\ud800":"old","?":"question","\ufffd":"replacement","\ud800":"two"},"model":{"type":"minecraft:model","model":"minecraft:item/component_map_id_123"}}],"fallback":{"type":"minecraft:model","model":"minecraft:item/component_map_id_fallback"}}}"#,
    );
    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |properties| {
        let stack = ItemStackSummary {
            item_id: Some(6),
            count: 1,
            component_patch: patch_with(
                &[bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID],
                |p| p.block_state_properties = properties,
            )
            .into(),
        };
        runtime
            .resolve_item_model(
                &stack,
                &ItemModelContext::new(BlockModelDisplayContext::Gui),
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };
    let expected = bbb_protocol::packets::data_components::BlockStateProperties::from_entries([
        (vec![0xd801].into(), "one".into()),
        (vec![0xd800].into(), "two".into()),
        ("?".into(), "question".into()),
        ("�".into(), "replacement".into()),
    ]);
    assert_eq!(selected(expected.clone()), uv("component_map_id_123"));
    let mut wrong = expected;
    wrong.insert(vec![0xd801].into(), "two".into());
    assert_eq!(selected(wrong), uv("component_map_id_fallback"));
    std::fs::remove_dir_all(root).unwrap();
}
