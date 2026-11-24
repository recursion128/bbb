use super::*;

#[test]
fn native_item_runtime_loads_client_item_fixture_and_keeps_missingno_fallback() {
    let root = unique_temp_dir("item-runtime");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture_defaults(&root);
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "item.minecraft.test_combo": "Test Combo",
                "raw.a": "Raw %s %s",
                "raw.b": "Child",
                "raw.c": "Raw artist",
                "key.keyboard.space": "Space",
                "key.keyboard.q": "Q",
                "item.minecraft.test_sword_item": "Test Sword",
                "item.minecraft.disc_fragment_5": "Disc Fragment",
                "item.minecraft.disc_fragment_5.desc": "Music Disc - 5",
                "item.minecraft.painting": "Painting",
                "painting.dimensions": "%sx%s",
                "painting.minecraft.sunset.author": "Kristoffer Zetterstrand",
                "painting.minecraft.sunset.title": "sunset_dense",
                "painting.minecraft.wither.title": "Wither",
                "painting.random": "Random variant",
                "item.unbreakable": "Unbreakable",
                "item.intangible": "Intangible",
                "item.disabled": "Disabled item",
                "effect.minecraft.bad_omen": "Bad Omen",
                "effect.minecraft.night_vision": "Night Vision",
                "effect.none": "No Effects",
                "effect.minecraft.poison": "Poison",
                "effect.minecraft.speed": "Speed",
                "effect.minecraft.weakness": "Weakness",
                "enchantment.minecraft.efficiency": "Efficiency",
                "enchantment.level.3": "III",
                "potion.potency.1": "II",
                "potion.potency.3": "IV",
                "potion.withAmplifier": "%s %s",
                "potion.withDuration": "%s (%s)",
                "potion.whenDrank": "When Applied:",
                "attribute.modifier.plus.0": "+%s %s",
                "attribute.modifier.plus.2": "+%s%% %s",
                "attribute.modifier.take.0": "-%s %s",
                "attribute.modifier.take.2": "-%s%% %s",
                "attribute.modifier.equals.0": "%s %s",
                "item.modifiers.mainhand": "When in Main Hand:",
                "item.modifiers.offhand": "When in Off Hand:",
                "attribute.name.attack_damage": "Attack Damage",
                "attribute.name.attack_speed": "Attack Speed",
                "attribute.name.mining_efficiency": "Mining Efficiency",
                "attribute.name.knockback_resistance": "Knockback Resistance",
                "attribute.name.movement_speed": "Speed",
                "item.durability": "Durability: %s / %s",
                "item.components": "%s component(s)",
                "component.profile.dynamic": "Dynamic",
                "container.beehive.bees": "Bees: %s / %s",
                "item.dyed": "Dyed",
                "item.color": "Color: %s",
                "item.container.loot_table.unknown": "Unknown contents",
                "item.container.item_count": "%s x%s",
                "item.container.more_items": "and %s more...",
                "entity.minecraft.tropical_fish.predefined.0": "Anemone",
                "entity.minecraft.tropical_fish.type.kob": "Kob",
                "color.minecraft.white": "White",
                "color.minecraft.red": "Red",
                "instrument.minecraft.ponder_goat_horn": "Ponder",
                "block.minecraft.banner.stripe_bottom.red": "Red Base",
                "block.test.banner.custom.blue": "Custom Blue",
                "�.blue": "Replacement",
                "?.blue": "Question",
                "block.minecraft.banner.guster.black": "Black Guster",
                "enchantment.custom.glow": "Custom Glow",
                "enchantment.minecraft.binding_curse": "Curse of Binding",
                "enchantment.minecraft.mending": "Mending",
                "enchantment.minecraft.sharpness": "Sharpness",
                "enchantment.minecraft.thorns": "Thorns",
                "enchantment.level.1": "I",
                "enchantment.level.2": "II",
                "filled_map.id": "Id #%s",
                "filled_map.level": "(Level %s/%s)",
                "filled_map.locked": "Locked",
                "filled_map.scale": "Scaling at 1:%s",
                "filled_map.unknown": "Unknown Map",
                "item.minecraft.smithing_template.upgrade": "Upgrade: ",
                "item.minecraft.crossbow.projectile.single": "Projectile: %s",
                "item.minecraft.crossbow.projectile.multiple": "Projectile: %s x %s",
                "trim_material.minecraft.iron": "Iron Material",
                "trim_material.minecraft.diamond": "Diamond Material",
                "trim_pattern.minecraft.sentry": "Sentry Armor Trim",
                "trim_pattern.minecraft.spire": "Spire Armor Trim",
                "trim.custom.material": "Localized Opal",
                "trim.custom.pattern": "Localized Spiral",
                "item.minecraft.firework_rocket.flight": "Flight Duration:",
                "item.minecraft.firework_rocket.single_star": "%s",
                "item.minecraft.firework_rocket.multiple_stars": "%s x %s",
                "container.beehive.honey": "Honey: %s / %s",
                "item.minecraft.firework_star.shape.small_ball": "Small Ball",
                "item.minecraft.firework_star.shape.large_ball": "Large Ball",
                "item.minecraft.firework_star.shape.star": "Star-shaped",
                "item.minecraft.firework_star.shape.creeper": "Creeper-shaped",
                "item.minecraft.firework_star.shape.burst": "Burst",
                "item.minecraft.firework_star.red": "Red",
                "item.minecraft.firework_star.yellow": "Yellow",
                "item.minecraft.firework_star.custom_color": "Custom",
                "item.minecraft.firework_star.fade_to": "Fade to",
                "item.minecraft.firework_star.trail": "Trail",
                "item.minecraft.firework_star.flicker": "Twinkle",
                "book.byAuthor": "by %1$s",
                "book.generation.0": "Original",
                "book.generation.2": "Copy of a copy",
                "jukebox_song.minecraft.cat": "C418 - cat"
            }"#,
    );
    write_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
                "hand_animation_on_swap": false,
                "swap_animation_scale": 0.375,
                "model": {
                    "type": "minecraft:composite",
                    "models": [
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/test_sword",
                            "tints": [
                                { "type": "minecraft:constant", "value": 3368601 },
                                { "type": "minecraft:custom_model_data", "index": 1, "default": 16711935 }
                            ]
                        },
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/missing_model"
                        }
                    ]
                }
            }"#,
    );
    write_json(
        &assets.join("models").join("item").join("test_sword.json"),
        r##"{
                "textures": {
                    "layer0": "minecraft:item/test_sword",
                    "layer1": {
                        "sprite": "custom:item/missing_overlay",
                        "force_translucent": true
                    },
                    "layer2": "minecraft:item/test_overlay"
                }
            }"##,
    );
    write_test_rgba_png(
        &assets.join("textures").join("item").join("test_sword.png"),
        1,
        1,
        &[255, 0, 0, 255],
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("item")
            .join("test_overlay.png"),
        1,
        1,
        &[0, 255, 0, 127],
    );

    static REBOUND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    fn bound_key(id: &[u16]) -> Option<&'static str> {
        id.iter().copied().eq("key.jump".encode_utf16()).then(|| {
            if REBOUND.load(std::sync::atomic::Ordering::Relaxed) {
                "key.keyboard.q"
            } else {
                "key.keyboard.space"
            }
        })
    }
    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap())
        .unwrap()
        .with_bound_key_lookup(bound_key);
    let test_combo_stack = ItemStackSummary {
        item_id: runtime.item_protocol_id("minecraft:test_combo"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    assert!(!runtime.hand_animation_on_swap_for_stack(&test_combo_stack));
    assert_eq!(
        runtime.swap_animation_scale_for_stack(&test_combo_stack),
        0.375
    );

    assert_eq!(runtime.item_definition_count(), 1);
    assert_eq!(runtime.item_registry_count(), 4);
    assert_eq!(runtime.resolved_model_count(), 1);
    assert_eq!(runtime.missing_model_count(), 1);
    assert_eq!(runtime.missing_texture_count(), 1);
    assert_eq!(runtime.texture_count(), 3);
    assert_eq!(runtime.icon_texture_count(), 1);
    assert_ne!(
        runtime.texture_index("minecraft:item/test_sword"),
        runtime.texture_index(MISSING_TEXTURE_ID)
    );
    assert_eq!(
        runtime.texture_index("custom:item/missing_overlay"),
        runtime.texture_index(MISSING_TEXTURE_ID)
    );
    assert_eq!(
        runtime.texture_index("unknown:item/texture"),
        runtime.texture_index(MISSING_TEXTURE_ID)
    );
    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/test_sword"))
    );
    assert_eq!(
        runtime
            .default_item_particle_sprites()
            .iter()
            .map(|sprites| sprites.to_vec())
            .collect::<Vec<_>>(),
        vec![
            vec![
                runtime.texture_index("minecraft:item/test_sword"),
                runtime.texture_index(MISSING_TEXTURE_ID),
                runtime.texture_index("minecraft:item/test_overlay"),
            ],
            vec![runtime.texture_index(MISSING_TEXTURE_ID)],
            vec![runtime.texture_index(MISSING_TEXTURE_ID)],
            vec![runtime.texture_index(MISSING_TEXTURE_ID)],
        ]
    );
    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(1),
        Some(runtime.texture_index(MISSING_TEXTURE_ID))
    );
    let icon = runtime.icon_for_protocol_id(0).unwrap();
    assert_eq!(icon.layers.len(), 3);
    assert_eq!(icon.layers[0].tint, rgb_i32_tint(0x33_66_99));
    assert_eq!(icon.layers[1].tint, rgb_i32_tint(0xff_00_ff));
    assert_eq!(icon.layers[2].tint, ITEM_TINT_WHITE);
    let sprite_uvs = runtime.atlas_sprite_uvs();
    assert_eq!(sprite_uvs.len(), runtime.texture_count());
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == "minecraft:item/test_sword" && sprite.uv == icon.layers[0].uv));
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == "minecraft:item/test_overlay" && sprite.has_translucent));
    assert_eq!(
        icon.layers[1].uv,
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(MISSING_TEXTURE_ID))
            .unwrap()
    );
    assert_eq!(runtime.icon_uv_for_protocol_id(0), Some(icon.layers[0].uv));
    let generated_layers = runtime
        .resolve_item_model(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            &ItemModelContext::new(BlockModelDisplayContext::Ground),
        )
        .and_then(|model| model.flat_icon())
        .map(|icon| runtime.generated_item_layers_for_icon(&icon))
        .unwrap_or_default();
    assert_eq!(generated_layers.len(), 3);
    assert!(!generated_layers[0].translucent);
    assert!(generated_layers[1].translucent);
    assert!(generated_layers[2].translucent);

    let stack_icon = runtime
        .resolve_item_model(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    custom_model_data_colors: vec![0x00_00_00, 0x12_34_56],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .and_then(|model| model.flat_icon())
        .unwrap();
    assert_eq!(stack_icon.layers[0].tint, rgb_i32_tint(0x33_66_99));
    assert_eq!(stack_icon.layers[1].tint, rgb_i32_tint(0x12_34_56));

    std::fs::remove_dir_all(root).unwrap();
}
