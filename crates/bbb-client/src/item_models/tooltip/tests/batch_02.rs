use super::*;

#[test]
fn native_item_runtime_renders_client_item_fixture_tooltips() {
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

    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        Some(vec![name_line(
            "Test Combo",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let raw_name = NbtSummaryValue::Compound(vec![
        entry("translate", NbtSummaryValue::String("raw.a".into())),
        entry(
            "with",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Compound(vec![entry(
                    "translate",
                    NbtSummaryValue::String("raw.b".into()),
                )]),
                NbtSummaryValue::Compound(vec![entry(
                    "keybind",
                    NbtSummaryValue::String("key.jump".into()),
                )]),
            ]),
        ),
    ]);
    let raw_name_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            custom_name: Some("stale".into()),
            custom_name_nbt: Some(raw_name),
            lore: vec!["stale lore".into()],
            lore_nbt: vec![NbtSummaryValue::Compound(vec![entry(
                "translate",
                NbtSummaryValue::String("raw.b".into()),
            )])],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&raw_name_stack).unwrap()[0].text,
        "Raw Child Space"
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&raw_name_stack).unwrap()[1].text,
        "Child"
    );
    assert_eq!(
        runtime.styled_hover_name(&raw_name_stack).unwrap().text,
        "Raw Child Space"
    );
    REBOUND.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        runtime.styled_hover_name(&raw_name_stack).unwrap().text,
        "Raw Child Q"
    );
    REBOUND.store(false, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_options(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            true
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("minecraft:test_combo", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("13 component(s)", TOOLTIP_TEXT_DARK_GRAY),
        ])
    );
    let remapped_trim_material_keys = ["minecraft:diamond".to_string()];
    let remapped_trim_pattern_keys = ["minecraft:spire".to_string()];
    let remapped_trim_material_presentations = [None];
    let remapped_trim_pattern_presentations = [None];
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    armor_trim_material_id: Some(0),
                    armor_trim_pattern_id: Some(0),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                trim_material_keys: Some(&remapped_trim_material_keys),
                trim_material_presentations: Some(&remapped_trim_material_presentations),
                trim_pattern_keys: Some(&remapped_trim_pattern_keys),
                trim_pattern_presentations: Some(&remapped_trim_pattern_presentations),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Upgrade: ", TOOLTIP_TEXT_GRAY),
            colored_trim_tooltip_line("Spire Armor Trim", 0x6E_EC_D2),
            colored_trim_tooltip_line("Diamond Material", 0x6E_EC_D2),
        ])
    );
    let unresolved_trim_keys = [String::new()];
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    armor_trim_material_id: Some(0),
                    armor_trim_pattern_id: Some(0),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                trim_material_keys: Some(&unresolved_trim_keys),
                trim_material_presentations: Some(&remapped_trim_material_presentations),
                trim_pattern_keys: Some(&unresolved_trim_keys),
                trim_pattern_presentations: Some(&remapped_trim_pattern_presentations),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![name_line(
            "Test Combo",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    let translated_component = |key: &str, fallback: &str, color: Option<&str>| {
        let mut entries = vec![
            NbtSummaryEntry {
                name: "translate".into(),
                value: NbtSummaryValue::string(key),
            },
            NbtSummaryEntry {
                name: "fallback".into(),
                value: NbtSummaryValue::string(fallback),
            },
        ];
        if let Some(color) = color {
            entries.push(NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string(color),
            });
        }
        NbtSummaryValue::Compound(entries)
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                    asset_name: "opal".to_string(),
                    override_armor_assets: BTreeMap::new(),
                    description: "stale material".to_string(),
                    description_styled: None,
                    description_nbt: Some(Box::new(translated_component(
                        "trim.custom.material",
                        "Fallback material",
                        Some("#123456"),
                    ))),
                })),
                armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                    asset_id: "example:spiral".to_string(),
                    description: "stale pattern".to_string(),
                    description_styled: None,
                    description_nbt: Some(Box::new(translated_component(
                        "trim.custom.pattern",
                        "Fallback pattern",
                        None,
                    ))),
                    decal: true,
                })),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Upgrade: ", TOOLTIP_TEXT_GRAY),
            colored_trim_tooltip_line("Localized Spiral", 0x12_34_56),
            colored_trim_tooltip_line("Localized Opal", 0x12_34_56),
        ])
    );
    let custom_material_keys = ["example:opal".to_string()];
    let custom_pattern_keys = ["example:spiral".to_string()];
    let custom_material_presentations = [Some(TrimMaterialPresentation {
        asset_name: "opal".to_string(),
        override_armor_assets: BTreeMap::new(),
        description: "Fallback material".to_string(),
        description_styled: vec![StyledTextRun {
            text: "Fallback material".to_string(),
            style: ComponentStyle {
                color: Some(0x12_34_56),
                ..ComponentStyle::default()
            },
        }],
        description_nbt: Some(translated_component(
            "trim.custom.material",
            "Fallback material",
            Some("#123456"),
        )),
    })];
    let custom_pattern_presentations = [Some(TrimPatternPresentation {
        asset_id: "example:spiral".to_string(),
        description: "Fallback pattern".to_string(),
        description_styled: vec![StyledTextRun {
            text: "Fallback pattern".to_string(),
            style: ComponentStyle::default(),
        }],
        description_nbt: Some(translated_component(
            "trim.custom.pattern",
            "Fallback pattern",
            None,
        )),
        decal: true,
    })];
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    armor_trim_material_id: Some(0),
                    armor_trim_pattern_id: Some(0),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                trim_material_keys: Some(&custom_material_keys),
                trim_material_presentations: Some(&custom_material_presentations),
                trim_pattern_keys: Some(&custom_pattern_keys),
                trim_pattern_presentations: Some(&custom_pattern_presentations),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Upgrade: ", TOOLTIP_TEXT_GRAY),
            colored_trim_tooltip_line("Localized Spiral", 0x12_34_56),
            colored_trim_tooltip_line("Localized Opal", 0x12_34_56),
        ])
    );
    let synchronized_paintings = vec![Some(bbb_client_world::PaintingVariantPresentation {
        width: 4,
        height: 2,
        asset_id: "example:gallery/wide".to_string(),
        title: Some("Custom gallery".to_string()),
        title_styled: vec![bbb_protocol::StyledTextRun {
            text: "Custom gallery".to_string(),
            style: bbb_protocol::ComponentStyle {
                color: Some(0x12_34_56),
                ..bbb_protocol::ComponentStyle::default()
            },
        }],
        title_nbt: None,
        title_translation_key: None,
        author: Some("Pack author".to_string()),
        author_styled: vec![bbb_protocol::StyledTextRun {
            text: "Pack author".to_string(),
            style: bbb_protocol::ComponentStyle {
                color: Some(0x65_43_21),
                ..bbb_protocol::ComponentStyle::default()
            },
        }],
        author_nbt: None,
        author_translation_key: None,
    })];
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(2),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    painting_variant_id: Some(0),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                painting_variant_presentations: Some(&synchronized_paintings),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Painting", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            colored_tooltip_line("Custom gallery", TOOLTIP_TEXT_WHITE, 0x12_34_56),
            colored_tooltip_line("Pack author", TOOLTIP_TEXT_WHITE, 0x65_43_21),
            tooltip_line("4x2", TOOLTIP_TEXT_WHITE),
        ])
    );
    let mut raw_paintings = synchronized_paintings.clone();
    let raw_presentation = raw_paintings[0].as_mut().unwrap();
    raw_presentation.title_nbt = Some(NbtSummaryValue::Compound(vec![entry(
        "translate",
        NbtSummaryValue::String("raw.b".into()),
    )]));
    raw_presentation.author_nbt = Some(NbtSummaryValue::Compound(vec![entry(
        "translate",
        NbtSummaryValue::String("raw.c".into()),
    )]));
    let raw_painting_lines = runtime
        .tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(2),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    painting_variant_id: Some(0),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                painting_variant_presentations: Some(&raw_paintings),
                ..NativeItemTooltipOptions::default()
            },
        )
        .unwrap();
    assert_eq!(raw_painting_lines[1].text, "Child");
    assert_eq!(raw_painting_lines[2].text, "Raw artist");
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        Some(vec![
            name_line("Disc Fragment", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Music Disc - 5", TOOLTIP_TEXT_GRAY),
        ])
    );
    let painting_variant = bbb_protocol::packets::PaintingVariantSummary {
        width: 2,
        height: 1,
        asset_id: "minecraft:sunset".to_string(),
        title: Some("Sunset".to_string()),
        title_styled: Some(vec![bbb_protocol::StyledTextRun {
            text: "Sunset".to_string(),
            style: bbb_protocol::ComponentStyle {
                color: Some(0xFF_FF_55),
                ..bbb_protocol::ComponentStyle::default()
            },
        }]),
        title_nbt: None,
        author: Some("Artist".to_string()),
        author_styled: Some(vec![bbb_protocol::StyledTextRun {
            text: "Artist".to_string(),
            style: bbb_protocol::ComponentStyle {
                color: Some(0xAA_AA_AA),
                ..bbb_protocol::ComponentStyle::default()
            },
        }]),
        author_nbt: None,
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(2),
            count: 1,
            component_patch: DataComponentPatchSummary {
                painting_variant_direct: Some(painting_variant.clone()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Painting", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            colored_tooltip_line("Sunset", TOOLTIP_TEXT_WHITE, 0xFF_FF_55),
            colored_tooltip_line("Artist", TOOLTIP_TEXT_WHITE, 0xAA_AA_AA),
            tooltip_line("2x1", TOOLTIP_TEXT_WHITE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(2),
            count: 1,
            component_patch: DataComponentPatchSummary {
                painting_variant_direct: Some(painting_variant),
                tooltip_hidden_component_type_ids: vec![102],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![name_line(
            "Painting",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(2),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            NativeItemTooltipOptions {
                creative: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Painting", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Random variant", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(2),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    painting_variant_id: Some(10),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                creative: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Painting", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            colored_tooltip_line("sunset_dense", TOOLTIP_TEXT_WHITE, 0xFF_FF_55),
            colored_tooltip_line("Kristoffer Zetterstrand", TOOLTIP_TEXT_WHITE, 0xAA_AA_AA),
            tooltip_line("2x1", TOOLTIP_TEXT_WHITE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(2),
            count: 1,
            component_patch: DataComponentPatchSummary {
                painting_variant_id: Some(19),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Painting", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            colored_tooltip_line("Wither", TOOLTIP_TEXT_WHITE, 0xFF_FF_55),
            tooltip_line("2x2", TOOLTIP_TEXT_WHITE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(2),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    painting_variant_id: Some(999),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                creative: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![name_line(
            "Painting",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    tooltip_hide_tooltip: true,
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                creative: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Disc Fragment", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Music Disc - 5", TOOLTIP_TEXT_GRAY),
        ])
    );
    let hidden_tooltip_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            tooltip_hide_tooltip: true,
            lore: vec!["Hidden lore".to_string()],
            unbreakable: true,
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(runtime.tooltip_lines_for_stack(&hidden_tooltip_stack), None);
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &hidden_tooltip_stack,
            NativeItemTooltipOptions {
                creative: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![name_line(
            "Test Combo",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            NativeItemTooltipOptions {
                item_disabled: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Disabled item", TOOLTIP_TEXT_RED),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                tooltip_hidden_component_type_ids: vec![11, 13, 22, 44],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 13,
                    level: 1,
                }],
                dyed_color: Some(0x11_22_33),
                intangible_projectile: true,
                lore: vec!["Hidden lore".to_string()],
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_AQUA, 0x55_FF_FF, false),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
        ])
    );
    let stack_attribute_modifier = |attribute_id: i32,
                                    amount: f64,
                                    operation_id: i32,
                                    slot_id: i32,
                                    display_id: i32,
                                    display_text: Option<&str>| {
        AttributeModifierSummary {
            attribute_id,
            modifier_id: format!(
                "minecraft:test_{attribute_id}_{operation_id}_{slot_id}_{display_id}"
            ),
            amount_bits: amount.to_bits(),
            operation_id,
            slot_id,
            display_id,
            display_text: display_text.map(str::to_string),
            display_text_styled: (display_id == 2).then(|| {
                vec![StyledTextRun {
                    text: display_text.unwrap_or_default().to_string(),
                    style: ComponentStyle {
                        bold: Some(true),
                        color: Some(0xFF_AA_00),
                        ..ComponentStyle::default()
                    },
                }]
            }),
            display_text_nbt: (display_id == 2).then(|| {
                bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "text".into(),
                        value: bbb_protocol::NbtSummaryValue::string(
                            display_text.unwrap_or_default(),
                        ),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "bold".into(),
                        value: bbb_protocol::NbtSummaryValue::Byte(1),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "color".into(),
                        value: bbb_protocol::NbtSummaryValue::string("gold"),
                    },
                ])
            }),
        }
    };
    let attribute_modifier_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![16],
            attribute_modifiers: vec![
                stack_attribute_modifier(2, 3.0, 0, 1, 0, None),
                stack_attribute_modifier(4, -0.2, 2, 1, 0, None),
                stack_attribute_modifier(16, 1.0, 0, 1, 1, None),
                stack_attribute_modifier(2, 1.0, 0, 2, 2, Some("Heavy hit")),
                stack_attribute_modifier(16, 0.5, 0, 2, 0, None),
            ],
            intangible_projectile: true,
            lore: vec!["Before attributes".to_string()],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&attribute_modifier_stack),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("Before attributes"),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When in Main Hand:", TOOLTIP_TEXT_GRAY),
            tooltip_line("+3 Attack Damage", TOOLTIP_TEXT_BLUE),
            tooltip_line("-20% Attack Speed", TOOLTIP_TEXT_RED),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When in Off Hand:", TOOLTIP_TEXT_GRAY),
            NativeItemTooltipLine {
                text: "Heavy hit".to_string(),
                tint: TOOLTIP_TEXT_WHITE,
                runs: vec![HudStyledTextRun {
                    shadow_color: None,
                    text: "Heavy hit".to_string(),
                    style: HudTextStyle {
                        bold: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0xFF_AA_00),
                }
                .into()],
            },
            tooltip_line("+5 Knockback Resistance", TOOLTIP_TEXT_BLUE),
            tooltip_line("Intangible", TOOLTIP_TEXT_GRAY),
        ])
    );
    let attribute_keys = bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES
        .iter()
        .map(|attribute| attribute.resource_id.to_string())
        .collect::<Vec<_>>();
    let world = world_with_stack_component_defaults(&runtime);
    let attribute_options = NativeItemTooltipOptions {
        item_profiles: Some(world.item_profiles()),
        attribute_keys: Some(&attribute_keys),
        ..NativeItemTooltipOptions::default()
    };
    let default_sword_stack = ItemStackSummary {
        item_id: Some(3),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &default_sword_stack,
            NativeItemTooltipOptions {
                player_base_attack_damage: Some(1.0),
                player_base_attack_speed: Some(4.0),
                ..attribute_options
            },
        ),
        Some(vec![
            name_line("Test Sword", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When in Main Hand:", TOOLTIP_TEXT_GRAY),
            tooltip_line(" 6 Attack Damage", TOOLTIP_TEXT_DARK_GREEN),
            tooltip_line(" 1.6 Attack Speed", TOOLTIP_TEXT_DARK_GREEN),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![16],
                    attribute_modifiers: vec![stack_attribute_modifier(2, 2.0, 0, 1, 0, None)],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
                ..default_sword_stack.clone()
            },
            attribute_options
        ),
        Some(vec![
            name_line("Test Sword", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When in Main Hand:", TOOLTIP_TEXT_GRAY),
            tooltip_line("+2 Attack Damage", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&default_sword_stack),
        Some(vec![name_line(
            "Test Sword",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    let mut display_world = WorldStore::new();
    display_world.set_default_item_component_codec_values(BTreeMap::from([(
        3,
        BTreeMap::from([(
            component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
            serde_json::json!([
                {
                    "type": "minecraft:attack_damage", "id": "minecraft:hidden",
                    "amount": 5.0, "operation": "add_value", "slot": "mainhand",
                    "display": {"type": "hidden"}
                },
                {
                    "type": "minecraft:attack_speed", "id": "minecraft:override",
                    "amount": -2.4, "operation": "add_value", "slot": "mainhand",
                    "display": {"type": "override", "value": {
                        "text": "Heavy hit", "bold": true, "color": "gold"
                    }}
                }
            ]),
        )]),
    )]));
    display_world.rebuild_item_component_defaults().unwrap();
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &default_sword_stack,
            NativeItemTooltipOptions {
                item_profiles: Some(display_world.item_profiles()),
                ..attribute_options
            },
        ),
        Some(vec![
            name_line("Test Sword", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When in Main Hand:", TOOLTIP_TEXT_GRAY),
            NativeItemTooltipLine {
                text: "Heavy hit".to_string(),
                tint: TOOLTIP_TEXT_WHITE,
                runs: vec![HudStyledTextRun {
                    shadow_color: None,
                    text: "Heavy hit".to_string(),
                    style: HudTextStyle {
                        bold: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0xFF_AA_00),
                }
                .into()],
            },
        ])
    );
    let enchantment_attribute_modifiers = vec![bbb_client_world::EnchantmentAttributeModifier {
        attribute: "minecraft:mining_efficiency".to_string(),
        modifier_id: "minecraft:enchantment.efficiency/mainhand".to_string(),
        amount: 10.0,
        operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
        slot: "mainhand".to_string(),
    }];
    let efficiency_enchantment_keys = ["minecraft:efficiency".to_string()];
    let enchanted_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 0,
                level: 3,
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &enchanted_stack,
            NativeItemTooltipOptions {
                enchantment_keys: Some(&efficiency_enchantment_keys),
                enchantment_attribute_modifiers: Some(&enchantment_attribute_modifiers),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_AQUA, 0x55_FF_FF, false),
            enchantment_tooltip_line("Efficiency", "III"),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When in Main Hand:", TOOLTIP_TEXT_GRAY),
            tooltip_line("+10 Mining Efficiency", TOOLTIP_TEXT_BLUE),
        ])
    );
    let mut hidden_enchanted_stack = enchanted_stack;
    hidden_enchanted_stack
        .component_patch
        .tooltip_hidden_component_type_ids
        .push(16);
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &hidden_enchanted_stack,
            NativeItemTooltipOptions {
                enchantment_keys: Some(&efficiency_enchantment_keys),
                enchantment_attribute_modifiers: Some(&enchantment_attribute_modifiers),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_AQUA, 0x55_FF_FF, false),
            enchantment_tooltip_line("Efficiency", "III"),
        ])
    );
    for component_patch in [
        DataComponentPatchSummary {
            added_type_ids: vec![16],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            removed_type_ids: vec![16],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tooltip_hidden_component_type_ids: vec![16],
            ..DataComponentPatchSummary::default()
        },
    ] {
        assert_eq!(
            runtime.tooltip_lines_for_stack_with_context(
                &ItemStackSummary {
                    component_patch: component_patch.into(),
                    ..default_sword_stack.clone()
                },
                attribute_options
            ),
            Some(vec![name_line(
                "Test Sword",
                TOOLTIP_TEXT_WHITE,
                0xFF_FF_FF,
                false,
            )])
        );
    }
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                tooltip_hidden_component_type_ids: vec![16],
                ..attribute_modifier_stack
                    .component_patch
                    .clone()
                    .into_inner()
            }
            .into(),
            ..attribute_modifier_stack
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("Before attributes"),
            tooltip_line("Intangible", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                tropical_fish_pattern_id: Some(VANILLA_TROPICAL_FISH_PATTERN_STRIPEY_PACKED_ID),
                tropical_fish_base_color_id: Some(VANILLA_DYE_COLOR_ORANGE_ID),
                tropical_fish_pattern_color_id: Some(VANILLA_DYE_COLOR_GRAY_ID),
                // With no instrument registry on the session the holder id is
                // the loader-order index, where `minecraft:ponder_goat_horn`
                // sits at 4 rather than at its bootstrap index 0.
                instrument: bbb_protocol::packets::InstrumentComponentSummary {
                    registry_id: Some(4),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            italic_tooltip_line("Anemone", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
            tooltip_line("Ponder", TOOLTIP_TEXT_GRAY),
        ])
    );
    let instrument_keys = [
        "example:custom_horn".to_string(),
        "minecraft:ponder_goat_horn".to_string(),
    ];
    let instrument_presentations = [
        Some(bbb_client_world::InstrumentPresentation {
            sound_event: bbb_client_world::RegistrySoundEvent::Reference {
                resource_id: "example:custom_sound".to_string(),
            },
            use_duration: 4.0,
            range: 96.0,
            description: "Custom horn".to_string(),
            description_styled: vec![bbb_protocol::StyledTextRun {
                text: "Custom horn".to_string(),
                style: bbb_protocol::ComponentStyle {
                    italic: Some(true),
                    color: Some(0x55_FF_55),
                    ..bbb_protocol::ComponentStyle::default()
                },
            }],
            description_nbt: NbtSummaryValue::Compound(vec![
                entry("text", NbtSummaryValue::String("Custom horn".into())),
                entry("italic", NbtSummaryValue::Byte(1)),
                entry("color", NbtSummaryValue::String("#55ff55".into())),
            ]),
        }),
        None,
    ];
    let instrument_stack = |instrument_id| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            instrument: bbb_protocol::packets::InstrumentComponentSummary {
                registry_id: Some(instrument_id),
                ..Default::default()
            },
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let instrument_options = NativeItemTooltipOptions {
        instrument_keys: Some(&instrument_keys),
        instrument_presentations: Some(&instrument_presentations),
        ..NativeItemTooltipOptions::default()
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&instrument_stack(0), instrument_options),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            italic_tooltip_line("Custom horn", TOOLTIP_TEXT_GRAY, 0x55_FF_55),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&instrument_stack(1), instrument_options),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Ponder", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                tropical_fish_pattern_id: Some(VANILLA_TROPICAL_FISH_PATTERN_KOB_PACKED_ID),
                tropical_fish_base_color_id: Some(VANILLA_DYE_COLOR_WHITE_ID),
                tropical_fish_pattern_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            italic_tooltip_line("Kob", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
            italic_tooltip_line("White, Red", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                instrument: bbb_protocol::packets::InstrumentComponentSummary {
                    registry_id: Some(4),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Ponder", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                instrument: bbb_protocol::packets::InstrumentComponentSummary {
                    direct: Some(Box::new(bbb_protocol::packets::InstrumentSummary {
                        sound_event: bbb_protocol::packets::SoundEventSummary {
                            registry_id: Some(0),
                            sound_id: None,
                            fixed_range_bits: None,
                        },
                        use_duration_bits: 7.0_f32.to_bits(),
                        range_bits: 256.0_f32.to_bits(),
                        description: "Inline Horn".to_string(),
                        description_styled: Some(
                            vec![bbb_protocol::StyledTextRun {
                                text: "Inline Horn".to_string(),
                                style: bbb_protocol::ComponentStyle {
                                    italic: Some(true),
                                    color: Some(0xFF_AA_00),
                                    ..bbb_protocol::ComponentStyle::default()
                                },
                            }]
                            .into_boxed_slice()
                        ),
                        description_nbt: None,
                    })),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            italic_tooltip_line("Inline Horn", TOOLTIP_TEXT_GRAY, 0xFF_AA_00),
        ])
    );
    let map_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            map_id: Some(7),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&map_stack),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Unknown Map", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                map_id: Some(7),
                tooltip_hidden_component_type_ids: vec![46],
                ..DataComponentPatchSummary::default()
            }
            .into(),
            ..map_stack.clone()
        }),
        Some(vec![name_line(
            "Test Combo",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &map_stack,
            NativeItemTooltipOptions {
                map_data: Some(NativeItemMapTooltipData {
                    scale: 2,
                    locked: false,
                }),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Id #7", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![46, 48],
                    map_id: Some(7),
                    map_post_processing: Some(MapPostProcessingSummary::Scale),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
                ..map_stack.clone()
            },
            NativeItemTooltipOptions {
                advanced: true,
                map_data: Some(NativeItemMapTooltipData {
                    scale: 2,
                    locked: false,
                }),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Scaling at 1:8", TOOLTIP_TEXT_GRAY),
            tooltip_line("(Level 3/4)", TOOLTIP_TEXT_GRAY),
            tooltip_line("minecraft:test_combo", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("15 component(s)", TOOLTIP_TEXT_DARK_GRAY),
        ])
    );
    let damaged_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![2, 3],
            max_damage: Some(20),
            damage: Some(3),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&damaged_stack),
        Some(vec![name_line(
            "Test Combo",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_options(&damaged_stack, true),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Durability: 17 / 20", TOOLTIP_TEXT_WHITE),
            tooltip_line("minecraft:test_combo", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("15 component(s)", TOOLTIP_TEXT_DARK_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 0,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        None
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                custom_name: Some("Custom Pick".to_string()),
                item_name: Some("Renamed Item Name".to_string()),
                lore: vec!["First lore".to_string(), "Second lore".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Custom Pick", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, true),
            lore_line("First lore"),
            lore_line("Second lore"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                    title: "Book Title".into(),
                    title_filter: None,
                    author: "Alex".into(),
                    generation: 0,
                    pages: Vec::new(),
                    page_filters: Vec::new(),
                    resolved: true,
                }),
                item_name: Some("Ignored Item Name".to_string()),
                lore: vec!["Book lore".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Book Title", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("by Alex", TOOLTIP_TEXT_GRAY),
            tooltip_line("Original", TOOLTIP_TEXT_GRAY),
            lore_line("Book lore"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                    title: "Copy".into(),
                    title_filter: None,
                    author: "   ".into(),
                    generation: 2,
                    pages: Vec::new(),
                    page_filters: Vec::new(),
                    resolved: true,
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Copy", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Copy of a copy", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                charged_projectiles_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default().into(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default().into(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            custom_name: Some("Charged Custom".to_string()),
                            custom_name_styled: Some(vec![
                                bbb_protocol::StyledTextRun {
                                    text: "Charged".to_string(),
                                    style: bbb_protocol::ComponentStyle {
                                        color: Some(0xFF_AA_00),
                                        ..bbb_protocol::ComponentStyle::default()
                                    },
                                },
                                bbb_protocol::StyledTextRun {
                                    text: " Custom".to_string(),
                                    style: bbb_protocol::ComponentStyle::default(),
                                },
                            ]),
                            container_loot: true,
                            potion_contents: bbb_protocol::packets::PotionContentsSummary {
                                custom_effects: vec![MobEffectInstanceSummary {
                                    effect_id: VANILLA_MOB_EFFECT_POISON_ID,
                                    amplifier: 1,
                                    duration: 200,
                                    ambient: false,
                                    show_particles: true,
                                    show_icon: true,
                                    hidden_effect: None,
                                }],
                                ..Default::default()
                            },
                            unbreakable: true,
                            ..DataComponentPatchSummary::default()
                        }
                        .into(),
                    },
                ],
                fireworks_flight_duration: Some(1),
                lore: vec!["After charged projectiles".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            charged_projectile_header_line(
                "Projectile: 2 x [Test Combo]",
                vec![
                    HudStyledTextRun::plain("Projectile: "),
                    HudStyledTextRun::plain("2"),
                    HudStyledTextRun::plain(" x "),
                    display_name_bracket_run("["),
                    display_name_bracket_run("Test Combo"),
                    display_name_bracket_run("]"),
                ],
            ),
            charged_projectile_header_line(
                "Projectile: [Charged Custom]",
                vec![
                    HudStyledTextRun::plain("Projectile: "),
                    display_name_bracket_run("["),
                    HudStyledTextRun {
                        shadow_color: None,
                        text: "Charged".to_string(),
                        style: HudTextStyle {
                            italic: true,
                            ..HudTextStyle::default()
                        },
                        color: Some(0xFF_AA_00),
                    },
                    HudStyledTextRun {
                        shadow_color: None,
                        text: " Custom".to_string(),
                        style: HudTextStyle {
                            italic: true,
                            ..HudTextStyle::default()
                        },
                        color: Some(0xFF_FF_FF),
                    },
                    display_name_bracket_run("]"),
                ],
            ),
            charged_projectile_detail_line("Unknown contents", 0xAA_AA_AA),
            charged_projectile_detail_line("Poison II (00:10)", 0xFF_55_55),
            charged_projectile_detail_line("Unbreakable", 0x55_55_FF),
            tooltip_line("Flight Duration: 1", TOOLTIP_TEXT_GRAY),
            lore_line("After charged projectiles"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    custom_effects: vec![MobEffectInstanceSummary {
                        effect_id: VANILLA_MOB_EFFECT_POISON_ID,
                        amplifier: 1,
                        duration: 200,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: None,
                    }],
                    ..Default::default()
                },
                lore: vec!["After potion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Poison II (00:10)", TOOLTIP_TEXT_RED),
            lore_line("After potion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    potion_contents: bbb_protocol::packets::PotionContentsSummary {
                        custom_effects: vec![MobEffectInstanceSummary {
                            effect_id: VANILLA_MOB_EFFECT_POISON_ID,
                            amplifier: 1,
                            duration: 200,
                            ambient: false,
                            show_particles: true,
                            show_icon: true,
                            hidden_effect: None,
                        }],
                        ..Default::default()
                    },
                    lore: vec!["After slow tick potion".to_string()],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                tooltip_tick_rate: Some(10.0),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Poison II (00:20)", TOOLTIP_TEXT_RED),
            lore_line("After slow tick potion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    custom_effects: vec![MobEffectInstanceSummary {
                        effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
                        amplifier: 1,
                        duration: 1200,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: None,
                    }],
                    ..Default::default()
                },
                lore: vec!["After speed potion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Speed II (01:00)", TOOLTIP_TEXT_BLUE),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When Applied:", TOOLTIP_TEXT_DARK_PURPLE),
            tooltip_line("+40% Speed", TOOLTIP_TEXT_BLUE),
            lore_line("After speed potion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    custom_effects: vec![MobEffectInstanceSummary {
                        effect_id: VANILLA_MOB_EFFECT_WEAKNESS_ID,
                        amplifier: 0,
                        duration: 1800,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: None,
                    }],
                    ..Default::default()
                },
                lore: vec!["After weakness potion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Weakness (01:30)", TOOLTIP_TEXT_RED),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When Applied:", TOOLTIP_TEXT_DARK_PURPLE),
            tooltip_line("-4 Attack Damage", TOOLTIP_TEXT_RED),
            lore_line("After weakness potion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(4),
                    ..Default::default()
                },
                lore: vec!["After base potion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Night Vision (03:00)", TOOLTIP_TEXT_BLUE),
            lore_line("After base potion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(13),
                    ..Default::default()
                },
                lore: vec!["After base speed potion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Speed (03:00)", TOOLTIP_TEXT_BLUE),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("When Applied:", TOOLTIP_TEXT_DARK_PURPLE),
            tooltip_line("+20% Speed", TOOLTIP_TEXT_BLUE),
            lore_line("After base speed potion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(4),
                    custom_effects: vec![MobEffectInstanceSummary {
                        effect_id: VANILLA_MOB_EFFECT_POISON_ID,
                        amplifier: 0,
                        duration: 200,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: None,
                    }],
                    ..Default::default()
                },
                lore: vec!["After mixed potion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Night Vision (03:00)", TOOLTIP_TEXT_BLUE),
            tooltip_line("Poison (00:10)", TOOLTIP_TEXT_RED),
            lore_line("After mixed potion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(0),
                    ..Default::default()
                },
                lore: vec!["After empty potion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("No Effects", TOOLTIP_TEXT_GRAY),
            lore_line("After empty potion"),
        ])
    );
    let suspicious_stew_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            suspicious_stew_effects: vec![SuspiciousStewEffectSummary {
                effect_id: VANILLA_MOB_EFFECT_POISON_ID,
                duration: 160,
            }],
            lore: vec!["Before creative stew details".to_string()],
            unbreakable: true,
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&suspicious_stew_stack),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("Before creative stew details"),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &suspicious_stew_stack,
            NativeItemTooltipOptions {
                creative: true,
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("Before creative stew details"),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
            tooltip_line("Poison (00:08)", TOOLTIP_TEXT_RED),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &suspicious_stew_stack,
            NativeItemTooltipOptions {
                creative: true,
                tooltip_tick_rate: Some(10.0),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("Before creative stew details"),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
            tooltip_line("Poison (00:16)", TOOLTIP_TEXT_RED),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Component Item Name".to_string()),
                rarity: Some(ItemRaritySummary::Uncommon),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![name_line(
            "Component Item Name",
            TOOLTIP_TEXT_YELLOW,
            0xFF_FF_55,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Durable Item".to_string()),
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Durable Item", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                intangible_projectile: true,
                unbreakable: true,
                ominous_bottle_amplifier: Some(3),
                lore: vec!["Before unit components".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("Before unit components"),
            tooltip_line("Intangible", TOOLTIP_TEXT_GRAY),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
            tooltip_line("Bad Omen IV (01:40:00)", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    intangible_projectile: true,
                    unbreakable: true,
                    ominous_bottle_amplifier: Some(3),
                    lore: vec!["Before unit components".to_string()],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                tooltip_tick_rate: Some(10.0),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("Before unit components"),
            tooltip_line("Intangible", TOOLTIP_TEXT_GRAY),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
            tooltip_line("Bad Omen IV (03:20:00)", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                bees_present: true,
                bees: bee_occupants(2),
                lore: vec!["After bees".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Bees: 2 / 3", TOOLTIP_TEXT_GRAY),
            lore_line("After bees"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                bees_present: true,
                lore: vec!["After empty bees".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Bees: 0 / 3", TOOLTIP_TEXT_GRAY),
            lore_line("After empty bees"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                bees_present: true,
                bees: bee_occupants(1),
                container_loot: true,
                lore: vec!["After container loot".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Bees: 1 / 3", TOOLTIP_TEXT_GRAY),
            tooltip_line("Unknown contents", TOOLTIP_TEXT_WHITE),
            lore_line("After container loot"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                container: container_from_items(
                    (1..=6)
                        .map(|count| ItemStackTemplateSummary {
                            item_id: 0,
                            count,
                            component_patch: DataComponentPatchSummary::default().into(),
                        })
                        .collect(),
                    Some(6)
                ),
                lore: vec!["After container items".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            container_item_count_line(
                "Test Combo x1",
                vec![HudStyledTextRun::plain("Test Combo")],
                1
            ),
            container_item_count_line(
                "Test Combo x2",
                vec![HudStyledTextRun::plain("Test Combo")],
                2
            ),
            container_item_count_line(
                "Test Combo x3",
                vec![HudStyledTextRun::plain("Test Combo")],
                3
            ),
            container_item_count_line(
                "Test Combo x4",
                vec![HudStyledTextRun::plain("Test Combo")],
                4
            ),
            container_item_count_line(
                "Test Combo x5",
                vec![HudStyledTextRun::plain("Test Combo")],
                5
            ),
            italic_plain_tooltip_line("and 1 more...", TOOLTIP_TEXT_WHITE),
            lore_line("After container items"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                container: container_from_items(
                    vec![ItemStackTemplateSummary {
                        item_id: 0,
                        count: 2,
                        component_patch: DataComponentPatchSummary {
                            custom_name: Some("Styled Item".to_string()),
                            custom_name_styled: Some(vec![
                                bbb_protocol::StyledTextRun {
                                    text: "Styled".to_string(),
                                    style: bbb_protocol::ComponentStyle {
                                        color: Some(0x55_FF_FF),
                                        bold: Some(true),
                                        ..bbb_protocol::ComponentStyle::default()
                                    },
                                },
                                bbb_protocol::StyledTextRun {
                                    text: " Item".to_string(),
                                    style: bbb_protocol::ComponentStyle::default(),
                                },
                            ]),
                            ..DataComponentPatchSummary::default()
                        }
                        .into(),
                    }],
                    None
                ),
                lore: vec!["After styled container item".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            container_item_count_line(
                "Styled Item x2",
                vec![
                    HudStyledTextRun {
                        shadow_color: None,
                        text: "Styled".to_string(),
                        style: HudTextStyle {
                            bold: true,
                            ..HudTextStyle::default()
                        },
                        color: Some(0x55_FF_FF),
                    },
                    HudStyledTextRun::plain(" Item"),
                ],
                2,
            ),
            lore_line("After styled container item"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                // With no banner-pattern registry the holder id is the
                // loader-order index: `minecraft:stripe_bottom` answers to 31,
                // not to its bootstrap index 5.
                banner_pattern_layers: vec![
                    BannerPatternLayerSummary {
                        registry_id: Some(31),
                        asset_id: None,
                        translation_key: None,
                        color_id: VANILLA_DYE_COLOR_RED_ID,
                    },
                    BannerPatternLayerSummary {
                        registry_id: None,
                        asset_id: Some("test:banner/custom".to_string()),
                        translation_key: Some("block.test.banner.custom".into()),
                        color_id: VANILLA_DYE_COLOR_BLUE_ID,
                    },
                    BannerPatternLayerSummary {
                        registry_id: Some(31),
                        asset_id: None,
                        translation_key: None,
                        color_id: VANILLA_DYE_COLOR_RED_ID,
                    },
                    BannerPatternLayerSummary {
                        registry_id: Some(31),
                        asset_id: None,
                        translation_key: None,
                        color_id: VANILLA_DYE_COLOR_RED_ID,
                    },
                    BannerPatternLayerSummary {
                        registry_id: Some(31),
                        asset_id: None,
                        translation_key: None,
                        color_id: VANILLA_DYE_COLOR_RED_ID,
                    },
                    BannerPatternLayerSummary {
                        registry_id: Some(31),
                        asset_id: None,
                        translation_key: None,
                        color_id: VANILLA_DYE_COLOR_RED_ID,
                    },
                    BannerPatternLayerSummary {
                        registry_id: Some(42),
                        asset_id: None,
                        translation_key: None,
                        color_id: VANILLA_DYE_COLOR_BLACK_ID,
                    },
                ],
                lore: vec!["After banner patterns".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Red Base", TOOLTIP_TEXT_GRAY),
            tooltip_line("Custom Blue", TOOLTIP_TEXT_GRAY),
            tooltip_line("Red Base", TOOLTIP_TEXT_GRAY),
            tooltip_line("Red Base", TOOLTIP_TEXT_GRAY),
            tooltip_line("Red Base", TOOLTIP_TEXT_GRAY),
            tooltip_line("Red Base", TOOLTIP_TEXT_GRAY),
            lore_line("After banner patterns"),
        ])
    );

    let synchronized_banner_patterns = [
        Some(BannerPatternElement {
            asset_id: "example:custom".to_string(),
            translation_key: "block.test.banner.custom".into(),
        }),
        None,
    ];
    let synchronized_banner_keys = ["example:custom".to_string(), String::new()];
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    banner_pattern_layers: vec![
                        BannerPatternLayerSummary {
                            registry_id: Some(0),
                            asset_id: None,
                            translation_key: None,
                            color_id: VANILLA_DYE_COLOR_BLUE_ID,
                        },
                        BannerPatternLayerSummary {
                            registry_id: Some(1),
                            asset_id: None,
                            translation_key: None,
                            color_id: VANILLA_DYE_COLOR_RED_ID,
                        },
                        BannerPatternLayerSummary {
                            registry_id: None,
                            asset_id: Some("Invalid Asset".to_string()),
                            translation_key: Some("block.minecraft.banner.guster".into()),
                            color_id: VANILLA_DYE_COLOR_BLACK_ID,
                        },
                    ],
                    ..Default::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                banner_pattern_keys: Some(&synchronized_banner_keys),
                banner_pattern_presentations: Some(&synchronized_banner_patterns),
                ..Default::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Custom Blue", TOOLTIP_TEXT_GRAY),
        ])
    );
    let raw_banner_lines = runtime
        .tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                banner_pattern_layers: [vec![0xfffd], vec![b'?' as u16]]
                    .into_iter()
                    .map(|units| BannerPatternLayerSummary {
                        asset_id: Some("example:logo".into()),
                        translation_key: Some(units.into()),
                        color_id: VANILLA_DYE_COLOR_BLUE_ID,
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            }
            .into(),
        })
        .unwrap();
    assert_eq!(
        raw_banner_lines
            .iter()
            .skip(1)
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        ["Replacement", "Question"]
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                pot_decorations_item_ids: vec![0, 0, 0, 0],
                lore: vec!["After pot decorations".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Test Combo", TOOLTIP_TEXT_GRAY),
            tooltip_line("Test Combo", TOOLTIP_TEXT_GRAY),
            tooltip_line("Test Combo", TOOLTIP_TEXT_GRAY),
            tooltip_line("Test Combo", TOOLTIP_TEXT_GRAY),
            lore_line("After pot decorations"),
        ])
    );
    let enchantment_keys = vec![
        "custom:glow".to_string(),
        "minecraft:sharpness".to_string(),
        "minecraft:binding_curse".to_string(),
        "minecraft:mending".to_string(),
    ];
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 3,
                        level: 1,
                    }],
                    enchantments: vec![
                        bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 1,
                            level: 1,
                        },
                        bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 2,
                            level: 1,
                        },
                        bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 0,
                            level: 2,
                        },
                    ],
                    lore: vec!["After enchantments".to_string()],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            NativeItemTooltipOptions {
                enchantment_keys: Some(&enchantment_keys),
                ..NativeItemTooltipOptions::default()
            },
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_AQUA, 0x55_FF_FF, false),
            colored_tooltip_line("Mending", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
            colored_tooltip_line("Curse of Binding", TOOLTIP_TEXT_RED, 0xFF_55_55),
            enchantment_tooltip_line("Sharpness", "I"),
            enchantment_tooltip_line("Custom Glow", "II"),
            lore_line("After enchantments"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(0),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    description: "Test song".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test song".to_string(),
                            style: bbb_protocol::ComponentStyle {
                                bold: Some(true),
                                ..bbb_protocol::ComponentStyle::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                    description_nbt: None,
                    length_in_seconds_bits: 120.0f32.to_bits(),
                    comparator_output: 15,
                })),
                dyed_color: Some(0x33_44_55),
                lore: vec!["After jukebox".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            NativeItemTooltipLine {
                text: "Test song".to_string(),
                tint: TOOLTIP_TEXT_GRAY,
                runs: vec![HudStyledTextRun {
                    shadow_color: None,
                    text: "Test song".to_string(),
                    style: HudTextStyle {
                        bold: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0xAA_AA_AA),
                }
                .into()],
            },
            italic_tooltip_line("Dyed", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
            lore_line("After jukebox"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                jukebox_song_id: Some(1),
                lore: vec!["After jukebox holder".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("C418 - cat", TOOLTIP_TEXT_GRAY),
            lore_line("After jukebox holder"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                jukebox_song_id: Some(1),
                tooltip_hidden_component_type_ids: vec![64],
                lore: vec!["After hidden jukebox".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("After hidden jukebox"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                    asset_name: "test_material".to_string(),
                    override_armor_assets: BTreeMap::new(),
                    description: "Test material".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test material".to_string(),
                            style: bbb_protocol::ComponentStyle {
                                italic: Some(true),
                                color: Some(0x12_34_56),
                                ..bbb_protocol::ComponentStyle::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                    description_nbt: None,
                })),
                armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                    asset_id: "minecraft:test_pattern".to_string(),
                    description: "Test pattern".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test pattern".to_string(),
                            style: bbb_protocol::ComponentStyle {
                                bold: Some(true),
                                ..bbb_protocol::ComponentStyle::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                    description_nbt: None,
                    decal: false,
                })),
                dyed_color: Some(0x11_22_33),
                lore: vec!["After trim".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Upgrade: ", TOOLTIP_TEXT_GRAY),
            trim_tooltip_line(
                "Test pattern",
                HudStyledTextRun {
                    shadow_color: None,
                    text: "Test pattern".to_string(),
                    style: HudTextStyle {
                        bold: true,
                        italic: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0x12_34_56),
                }
            ),
            trim_tooltip_line(
                "Test material",
                HudStyledTextRun {
                    shadow_color: None,
                    text: "Test material".to_string(),
                    style: HudTextStyle {
                        italic: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0x12_34_56),
                }
            ),
            italic_tooltip_line("Dyed", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
            lore_line("After trim"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                // With no trim registries on the session the holder ids are
                // loader-order indices: `minecraft:iron` answers to 5 and
                // `minecraft:sentry` to 8, not to bootstrap 1 and 0.
                armor_trim_material_id: Some(5),
                armor_trim_pattern_id: Some(8),
                lore: vec!["After trim holder".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Upgrade: ", TOOLTIP_TEXT_GRAY),
            colored_trim_tooltip_line("Sentry Armor Trim", 0xEC_EC_EC),
            colored_trim_tooltip_line("Iron Material", 0xEC_EC_EC),
            lore_line("After trim holder"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                armor_trim_material_id: Some(5),
                armor_trim_pattern_id: Some(8),
                tooltip_hidden_component_type_ids: vec![56],
                lore: vec!["After hidden trim".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            lore_line("After hidden trim"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                profile: Some(ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::Partial,
                    uuid: None,
                    name: Some("Alex".to_string()),
                    properties: Vec::new(),
                    profile_textures: None,
                    skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary::default(),
                }),
                lore: vec!["After profile".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Dynamic", TOOLTIP_TEXT_GRAY),
            lore_line("After profile"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![44],
                dyed_color: Some(0x12_3A_BC),
                lore: vec!["After dyed".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            italic_tooltip_line("Dyed", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
            lore_line("After dyed"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_options(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![44],
                    dyed_color: Some(0x12_3A_BC),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
            true,
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Color: #123ABC", TOOLTIP_TEXT_GRAY),
            tooltip_line("minecraft:test_combo", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("14 component(s)", TOOLTIP_TEXT_DARK_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                lore: vec!["After fireworks".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Flight Duration: 2", TOOLTIP_TEXT_GRAY),
            lore_line("After fireworks"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                fireworks_explosions: vec![
                    FireworkExplosionSummary {
                        shape: FireworkExplosionShapeSummary::LargeBall,
                        colors: vec![11_743_532],
                        fade_colors: vec![14_602_026],
                        has_trail: true,
                        has_twinkle: false,
                    },
                    FireworkExplosionSummary {
                        shape: FireworkExplosionShapeSummary::LargeBall,
                        colors: vec![11_743_532],
                        fade_colors: vec![14_602_026],
                        has_trail: true,
                        has_twinkle: false,
                    },
                    FireworkExplosionSummary {
                        shape: FireworkExplosionShapeSummary::Burst,
                        colors: Vec::new(),
                        fade_colors: Vec::new(),
                        has_trail: false,
                        has_twinkle: true,
                    },
                ],
                lore: vec!["After grouped fireworks".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Flight Duration: 2", TOOLTIP_TEXT_GRAY),
            tooltip_line("2 x Large Ball", TOOLTIP_TEXT_GRAY),
            tooltip_line("  Red", TOOLTIP_TEXT_GRAY),
            tooltip_line("  Fade to Yellow", TOOLTIP_TEXT_GRAY),
            tooltip_line("  Trail", TOOLTIP_TEXT_GRAY),
            tooltip_line("Burst", TOOLTIP_TEXT_GRAY),
            tooltip_line("  Twinkle", TOOLTIP_TEXT_GRAY),
            lore_line("After grouped fireworks"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![76],
                block_state_properties: BTreeMap::from([
                    ("facing".into(), "north".into()),
                    ("honey_level".into(), "4".into()),
                ])
                .into(),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Honey: 4 / 5", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![68],
                firework_explosion: Some(bbb_protocol::packets::FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::Star,
                    colors: vec![11_743_532, 0x12_34_56],
                    fade_colors: vec![14_602_026],
                    has_trail: true,
                    has_twinkle: true,
                }),
                lore: vec!["After explosion".to_string()],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Star-shaped", TOOLTIP_TEXT_GRAY),
            tooltip_line("Red, Custom", TOOLTIP_TEXT_GRAY),
            tooltip_line("Fade to Yellow", TOOLTIP_TEXT_GRAY),
            tooltip_line("Trail", TOOLTIP_TEXT_GRAY),
            tooltip_line("Twinkle", TOOLTIP_TEXT_GRAY),
            lore_line("After explosion"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Enchanted Item".to_string()),
                // With no enchantment registry the holder id is the
                // loader-order index: `minecraft:thorns` answers to 39, not
                // to its bootstrap index 7.
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 39,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line("Enchanted Item", TOOLTIP_TEXT_AQUA, 0x55_FF_FF, false),
            enchantment_tooltip_line("Thorns", "I"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Rare Enchanted Item".to_string()),
                rarity: Some(ItemRaritySummary::Rare),
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 39,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            name_line(
                "Rare Enchanted Item",
                TOOLTIP_TEXT_LIGHT_PURPLE,
                0xFF_55_FF,
                false,
            ),
            enchantment_tooltip_line("Thorns", "I"),
        ])
    );
    // Styled component runs flow through: the custom name keeps its own keys
    // over the rarity wrapper (colour) while inheriting the custom-name
    // italic; an explicit `italic:0b` lore key beats the injected
    // `ItemLore.LORE_STYLE` and its colour override beats DARK_PURPLE.
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                custom_name: Some("Bold Red".to_string()),
                custom_name_styled: Some(vec![bbb_protocol::StyledTextRun {
                    text: "Bold Red".to_string(),
                    style: bbb_protocol::ComponentStyle {
                        bold: Some(true),
                        color: Some(0xFF_55_55),
                        ..Default::default()
                    },
                }]),
                lore: vec!["Upright gold".to_string()],
                lore_styled: vec![vec![bbb_protocol::StyledTextRun {
                    text: "Upright gold".to_string(),
                    style: bbb_protocol::ComponentStyle {
                        italic: Some(false),
                        color: Some(0xFF_AA_00),
                        ..Default::default()
                    },
                }]],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        }),
        Some(vec![
            NativeItemTooltipLine {
                text: "Bold Red".to_string(),
                tint: TOOLTIP_TEXT_WHITE,
                runs: vec![HudStyledTextRun {
                    shadow_color: None,
                    text: "Bold Red".to_string(),
                    style: HudTextStyle {
                        bold: true,
                        italic: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0xFF_55_55),
                }
                .into()],
            },
            NativeItemTooltipLine {
                text: "Upright gold".to_string(),
                tint: TOOLTIP_TEXT_DARK_PURPLE,
                runs: vec![HudStyledTextRun {
                    shadow_color: None,
                    text: "Upright gold".to_string(),
                    style: HudTextStyle::default(),
                    color: Some(0xFF_AA_00),
                }
                .into()],
            },
        ])
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_shows_smithing_template_append_tooltips() {
    let root = unique_temp_dir("item-runtime-smithing-template-tooltip");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        &root,
        &[
            "netherite_upgrade_smithing_template",
            "sentry_armor_trim_smithing_template",
        ],
    );
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "item.minecraft.netherite_upgrade_smithing_template": "Smithing Template",
                "item.minecraft.sentry_armor_trim_smithing_template": "Smithing Template",
                "item.minecraft.smithing_template": "Smithing Template",
                "item.minecraft.smithing_template.applies_to": "Applies to:",
                "item.minecraft.smithing_template.ingredients": "Ingredients:",
                "item.minecraft.smithing_template.netherite_upgrade.applies_to": "Diamond Equipment",
                "item.minecraft.smithing_template.netherite_upgrade.ingredients": "Netherite Ingot",
                "item.minecraft.smithing_template.armor_trim.applies_to": "Armor",
                "item.minecraft.smithing_template.armor_trim.ingredients": "Ingots & Crystals"
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        Some(vec![
            name_line("Smithing Template", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Smithing Template", TOOLTIP_TEXT_GRAY),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Applies to:", TOOLTIP_TEXT_GRAY),
            tooltip_line(" Diamond Equipment", TOOLTIP_TEXT_BLUE),
            tooltip_line("Ingredients:", TOOLTIP_TEXT_GRAY),
            tooltip_line(" Netherite Ingot", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }),
        Some(vec![
            name_line("Smithing Template", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Smithing Template", TOOLTIP_TEXT_GRAY),
            tooltip_line("", TOOLTIP_TEXT_WHITE),
            tooltip_line("Applies to:", TOOLTIP_TEXT_GRAY),
            tooltip_line(" Armor", TOOLTIP_TEXT_BLUE),
            tooltip_line("Ingredients:", TOOLTIP_TEXT_GRAY),
            tooltip_line(" Ingots & Crystals", TOOLTIP_TEXT_BLUE),
        ])
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn content_registry_custom_item_resolves_hover_name_and_tooltip() {
    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModItemRegistration};
    use std::sync::Arc;

    let root = unique_temp_dir("content-registry-tooltip");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_fixture(&root, &["stone"]);
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{"item.demo.relic": "Ancient Relic"}"#,
    );

    let mut runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let mut builder = ContentRegistryBuilder::new();
    builder.register_item(ModItemRegistration {
        resource_id: "demo:relic".to_string(),
        max_stack_size: 16,
        is_block_item: false,
    });
    let registry = Arc::new(builder.freeze_for_session().unwrap());
    let mod_item_id = registry.item_by_resource_id("demo:relic").unwrap().id;
    assert!(runtime.sync_content_registry(registry, None));

    let stack = ItemStackSummary {
        item_id: Some(mod_item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    assert_eq!(
        runtime.styled_hover_name(&stack).map(|name| name.text),
        Some("Ancient Relic".to_string())
    );
    let lines = runtime
        .tooltip_lines_for_stack(&stack)
        .expect("custom item tooltip lines");
    assert_eq!(lines[0].text, "Ancient Relic");

    std::fs::remove_dir_all(root).unwrap();
}
