use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
fn jigsaw_priority_tooltip_hit_testing_matches_priority_fields() {
    let layout = hud_jigsaw_block_editor_layout(winit::dpi::PhysicalSize::new(320, 240));
    assert_eq!(
        jigsaw_priority_tooltip_key(layout, (layout.fields[4].x + 1, 161)).map(|(key, _)| key),
        Some("jigsaw_block.selection_priority.tooltip")
    );
    assert_eq!(
        jigsaw_priority_tooltip_key(layout, (layout.fields[5].x + 1, 161)).map(|(key, _)| key),
        Some("jigsaw_block.placement_priority.tooltip")
    );
    assert!(jigsaw_priority_tooltip_key(layout, (layout.fields[0].x + 1, 21)).is_none());
}

use bbb_pack::{
    BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, EquipmentAssetCatalog,
    GrassColorModifier, ItemRegistryCatalog, RegistryDependentItemComponentInitializer,
};
use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID;
use bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID,
};
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_TYPES;
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPE_ANVIL_ID, VANILLA_MENU_TYPE_BEACON_ID, VANILLA_MENU_TYPE_BREWING_STAND_ID,
    VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID, VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
    VANILLA_MENU_TYPE_CRAFTING_ID, VANILLA_MENU_TYPE_ENCHANTMENT_ID, VANILLA_MENU_TYPE_FURNACE_ID,
    VANILLA_MENU_TYPE_GENERIC_3X3_ID, VANILLA_MENU_TYPE_GENERIC_9X3_ID,
    VANILLA_MENU_TYPE_GENERIC_9X6_ID, VANILLA_MENU_TYPE_GRINDSTONE_ID, VANILLA_MENU_TYPE_HOPPER_ID,
    VANILLA_MENU_TYPE_LECTERN_ID, VANILLA_MENU_TYPE_MERCHANT_ID, VANILLA_MENU_TYPE_SHULKER_BOX_ID,
    VANILLA_MENU_TYPE_SMITHING_ID, VANILLA_MENU_TYPE_STONECUTTER_ID,
};
use bbb_protocol::packets::ClockUpdate as ProtocolClockUpdate;
use bbb_protocol::packets::{
    AdvancementCriterionProgressSummary, AdvancementDisplaySummary, AdvancementFrameType,
    AdvancementIconSummary, AdvancementProgressSummary, AdvancementSummary, AwardStats,
    BlockPos as ProtocolBlockPos, BlockUpdate as ProtocolBlockUpdate, BossBarColor, BossBarOverlay,
    BossEvent as ProtocolBossEvent, BossEventFlags as ProtocolBossEventFlags,
    BossEventOperation as ProtocolBossEventOperation, CommonPlayerSpawnInfo,
    DataComponentPatchSummary, DialogHolder, EntityEvent as ProtocolEntityEvent,
    GameEvent as ProtocolGameEvent, InitializeBorder as ProtocolInitializeBorder, InteractionHand,
    MerchantOffer, MerchantOffers, MobEffectFlags, OpenBook, OpenSignEditor, PlayLogin, PlayTime,
    RemoveMobEffect, SetBorderLerpSize as ProtocolSetBorderLerpSize,
    SetCursorItem as ProtocolSetCursorItem, SetPlayerInventory as ProtocolSetPlayerInventory,
    ShowDialog, StatUpdate, UpdateAdvancements, UpdateMobEffect, WrittenBookContentSummary,
};
use bbb_protocol::recipe_wire_types::{
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID, VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
    VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID, VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
    VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID, VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID,
    VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID, VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
    VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
};

use bbb_protocol::stat_types::{
    VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_CUSTOM_ID,
    VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID, VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
    VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_world::{
    BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, HeightmapData, LightData,
    LocalPlayerPoseState, PaletteDomain, PaletteKind, PalettedContainerData, RegistryPacketEntry,
    WorldBlockDestroyProfile, WorldDimension, TRIM_MATERIAL_REGISTRY, TRIM_PATTERN_REGISTRY,
};
use tokio::sync::mpsc;
use winit::{
    event::{ElementState, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

const TOOLTIP_TEST_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TOOLTIP_TEST_AQUA: [f32; 4] = [85.0 / 255.0, 1.0, 1.0, 1.0];
const TOOLTIP_TEST_BLUE: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 1.0, 1.0];
const TOOLTIP_TEST_DARK_PURPLE: [f32; 4] = [170.0 / 255.0, 0.0, 170.0 / 255.0, 1.0];
const TOOLTIP_TEST_GRAY: [f32; 4] = [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0];
const TOOLTIP_TEST_DARK_GRAY: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];
const TOOLTIP_TEST_DARK_GREEN: [f32; 4] = [0.0, 170.0 / 255.0, 0.0, 1.0];
const TOOLTIP_TEST_RED: [f32; 4] = [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];
const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
const OAK_SIGN_ROTATION_0_BLOCK_STATE_ID: i32 = 5336;
const BAMBOO_HANGING_SIGN_ATTACHED_ROTATION_0_BLOCK_STATE_ID: i32 = 6612;
const TEST_LIGHT_ARRAY_BYTES: usize = 2048;

#[test]
fn localized_chat_format_preserves_outer_and_argument_styles() {
    let outer = bbb_protocol::ComponentStyle {
        color: Some(0xaaaaaa),
        ..bbb_protocol::ComponentStyle::default()
    };
    let argument = HudStyledTextRun {
        text: "Alice".to_string(),
        style: HudTextStyle {
            bold: true,
            ..HudTextStyle::default()
        },
        color: Some(0x55ffff),
    };
    let runs = format_localized_chat_runs(
        "<%1$s> %2$s %%",
        &outer,
        &[
            vec![argument.clone()],
            vec![HudStyledTextRun::plain("hello")],
        ],
    );
    assert_eq!(runs[0].text, "<");
    assert_eq!(runs[0].color, Some(0xaaaaaa));
    assert_eq!(runs[1], argument);
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<String>(),
        "<Alice> hello %"
    );
}

#[test]
fn rich_chat_projection_retains_click_hover_and_revalidates_canonical_target() {
    use bbb_protocol::{
        ComponentClickEvent, ComponentHoverEvent, NbtSummaryEntry, NbtSummaryValue,
    };

    let component =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("click me".to_string()),
            },
            NbtSummaryEntry {
                name: "font".to_string(),
                value: NbtSummaryValue::String("example:compact".to_string()),
            },
            NbtSummaryEntry {
                name: "click_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("suggest_command".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "command".to_string(),
                        value: NbtSummaryValue::String("/seed".to_string()),
                    },
                ]),
            },
            NbtSummaryEntry {
                name: "hover_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("show_text".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "value".to_string(),
                        value: NbtSummaryValue::String("details".to_string()),
                    },
                ]),
            },
        ]))
        .unwrap();
    let mut world = WorldStore::new();
    world.apply_system_chat(bbb_protocol::packets::SystemChat {
        content: Box::new(component),
        overlay: false,
    });

    let messages = hud_chat_messages_from_world(&world, None);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].interactions.len(), 1);
    assert_eq!(messages[0].interactions[0].tooltip_runs[0].text, "details");
    assert_eq!(
        hud_chat_component_click_event(
            &world,
            None,
            messages[0].display_sequence,
            messages[0].interactions[0].target_index,
        ),
        Some(ComponentClickEvent::SuggestCommand {
            command: "/seed".to_string(),
        })
    );
    assert!(matches!(
        world.client_chat().messages[0].styled_content[0]
            .style
            .hover_event,
        Some(ComponentHoverEvent::ShowText { .. })
    ));
    assert_eq!(
        world.client_chat().messages[0].styled_content[0]
            .style
            .font
            .as_deref(),
        Some("example:compact")
    );
}

#[test]
fn chat_narration_text_uses_styled_content_without_duplicate_projection() {
    let narration = bbb_world::ChatNarrationRequest {
        sequence: 1,
        kind: bbb_world::ChatNarrationKind::SystemChat,
        translation_key: None,
        arguments: Vec::new(),
        styled_content: vec![
            bbb_protocol::StyledTextRun {
                text: "hello ".to_string(),
                style: bbb_protocol::ComponentStyle::default(),
            },
            bbb_protocol::StyledTextRun {
                text: "world".to_string(),
                style: bbb_protocol::ComponentStyle {
                    bold: Some(true),
                    ..bbb_protocol::ComponentStyle::default()
                },
            },
        ],
        content: "fallback".to_string(),
    };
    assert_eq!(chat_narration_text(&narration, None), "hello world");
}

#[test]
fn chat_render_cache_rebuilds_only_for_display_revision_or_level() {
    let mut world = WorldStore::default();
    let mut runtime = HudRenderRuntime::default();
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Replace(messages) if messages.is_empty()
    ));
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Unchanged
    ));

    world.push_client_system_chat_message("hello");
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Replace(messages)
            if messages.len() == 1 && messages[0].runs.iter().any(|run| run.text == "hello")
    ));
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Unchanged
    ));

    world.clear_client_chat_display_messages();
    assert!(matches!(
        runtime.chat_messages_update(&world, None),
        RendererFrameUpdate::Replace(messages) if messages.is_empty()
    ));
}

#[test]
fn non_entity_render_observation_keeps_recent_and_peak_phase_costs() {
    let mut scratch = NonEntityRenderExtractionScratch::default();
    scratch.record_frame(
        Duration::from_nanos(5),
        Duration::from_nanos(7),
        Duration::from_nanos(11),
        Duration::from_nanos(13),
        Duration::from_nanos(17),
    );
    scratch.record_frame(
        Duration::from_nanos(3),
        Duration::from_nanos(19),
        Duration::from_nanos(2),
        Duration::from_nanos(23),
        Duration::from_nanos(1),
    );

    let observation = scratch.observation();
    assert_eq!(observation.hud_nanos, 3);
    assert_eq!(observation.hud_peak_nanos, 5);
    assert_eq!(observation.item_nanos, 19);
    assert_eq!(observation.item_peak_nanos, 19);
    assert_eq!(observation.block_entity_nanos, 2);
    assert_eq!(observation.block_entity_peak_nanos, 11);
    assert_eq!(observation.world_debug_nanos, 23);
    assert_eq!(observation.world_debug_peak_nanos, 23);
    assert_eq!(observation.apply_nanos, 1);
    assert_eq!(observation.apply_peak_nanos, 17);
}

#[test]
fn non_entity_stable_world_cache_requires_exact_arc_and_resource_epoch() {
    let world = Arc::new(WorldStore::new());
    let cloned_world = Arc::new((*world).clone());
    let mut scratch = NonEntityRenderExtractionScratch::default();

    assert!(!scratch.begin_stable_world_frame(&world));
    assert!(scratch.begin_stable_world_frame(&world));
    assert!(!scratch.begin_stable_world_frame(&cloned_world));
    assert!(scratch.begin_stable_world_frame(&cloned_world));
    scratch.invalidate_resources();
    assert!(!scratch.begin_stable_world_frame(&cloned_world));

    let observation = scratch.observation();
    assert_eq!(observation.cache_hits, 6);
    assert_eq!(observation.cache_misses, 9);
    assert_eq!(observation.rebuilt_owners, 9);
}

#[test]
fn map_texture_cache_invalidates_only_on_level_generation_edge() {
    let mut scratch = NonEntityRenderExtractionScratch::default();

    assert!(!scratch.synchronize_map_texture_level(4));
    assert!(!scratch.synchronize_map_texture_level(4));
    assert!(scratch.synchronize_map_texture_level(5));
    assert!(!scratch.synchronize_map_texture_level(5));
}

#[test]
fn item_registry_key_cache_follows_registry_access_revision() {
    let mut world = WorldStore::new();
    let mut scratch = NonEntityRenderExtractionScratch::default();

    let first = scratch.item_registry_keys(&world);
    let second = scratch.item_registry_keys(&world);
    assert_eq!(first.trim_materials, second.trim_materials);
    assert_eq!(first.trim_patterns, second.trim_patterns);
    assert_eq!(first.enchantments, second.enchantments);
    assert_eq!(first.attributes, second.attributes);
    assert_eq!(scratch.observation().cache_misses, 4);
    assert_eq!(scratch.observation().cache_hits, 4);

    world.reset_registry_session();
    let _ = scratch.item_registry_keys(&world);
    assert_eq!(scratch.observation().cache_misses, 8);
    assert_eq!(scratch.observation().cache_hits, 4);
}

/// A hover-name tooltip line: rarity colour run, italic when custom-named
/// (vanilla `ItemStack.getStyledHoverName`).
fn tooltip_name_line(
    text: &str,
    tint: [f32; 4],
    color: u32,
    italic: bool,
) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![bbb_renderer::HudStyledTextRun {
            text: text.to_string(),
            style: bbb_renderer::HudTextStyle {
                italic,
                ..Default::default()
            },
            color: Some(color),
        }],
    }
}

/// A lore tooltip line carrying vanilla `ItemLore.LORE_STYLE`
/// (DARK_PURPLE + italic).
fn tooltip_lore_line(text: &str) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEST_DARK_PURPLE,
        runs: vec![bbb_renderer::HudStyledTextRun {
            text: text.to_string(),
            style: bbb_renderer::HudTextStyle {
                italic: true,
                ..Default::default()
            },
            color: Some(0xAA_00_AA),
        }],
    }
}

fn tooltip_plain_line(text: &str, tint: [f32; 4]) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![bbb_renderer::HudStyledTextRun::plain(text)],
    }
}

fn tooltip_enchantment_line(description: &str, level: &str, color: u32) -> HudInventoryTooltipLine {
    let run = |text: &str| bbb_renderer::HudStyledTextRun {
        text: text.to_string(),
        style: bbb_renderer::HudTextStyle::default(),
        color: Some(color),
    };
    HudInventoryTooltipLine {
        text: format!("{description} {level}"),
        tint: TOOLTIP_TEST_GRAY,
        runs: vec![run(description), run(" "), run(level)],
    }
}

fn tooltip_operator_warning_line(text: &str, bold: bool) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEST_RED,
        runs: vec![bbb_renderer::HudStyledTextRun {
            text: text.to_string(),
            style: bbb_renderer::HudTextStyle {
                bold,
                ..Default::default()
            },
            color: Some(0xFF_55_55),
        }],
    }
}

#[test]
fn camera_pose_uses_local_player_eye_height() {
    let mut world = WorldStore::new();
    let standing_pose = LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        y_rot: 45.0,
        x_rot: -10.0,
        ..LocalPlayerPoseState::default()
    };
    world.set_local_player_pose(standing_pose);
    let pose = camera_pose_from_world(&world).unwrap();

    assert_eq!(pose.position, [1.0, 2.0, 3.0]);
    assert_eq!(pose.y_rot, 45.0);
    assert_eq!(pose.x_rot, -10.0);
    assert_eq!(pose.eye_height, CameraPose::STANDING_EYE_HEIGHT);

    let sneaking_pose = LocalPlayerPoseState {
        sneaking: true,
        ..standing_pose
    };
    world.set_local_player_pose(sneaking_pose);
    let pose = camera_pose_from_world(&world).unwrap();
    assert_eq!(pose.eye_height, sneaking_pose.eye_height() as f32);
}

#[test]
fn camera_hurt_effect_follows_the_active_living_camera_entity() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 42, yaw: 45.0 }));

    let local = camera_hurt_effect_from_world(&world, 0.5, 0.25).unwrap();
    assert_eq!(local.hurt_time, 9.5);
    assert_eq!(local.hurt_dir, 45.0);
    assert_eq!(local.damage_tilt_strength, 0.25);

    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CHICKEN_ID));
    assert!(world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 100, yaw: 90.0 }));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    let remote = camera_hurt_effect_from_world(&world, 0.25, 1.0).unwrap();
    assert_eq!(remote.hurt_time, 9.75);
    assert_eq!(remote.hurt_dir, 0.0);

    world.apply_add_entity(test_add_entity(101, VANILLA_ENTITY_TYPE_ARROW_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 101 }));
    assert_eq!(camera_hurt_effect_from_world(&world, 0.25, 1.0), None);
}

#[test]
fn particle_light_block_pos_uses_block_pos_containing_floor() {
    assert_eq!(
        particle_light_block_pos([1.99, 64.0, -0.01]),
        BlockPos { x: 1, y: 64, z: -1 }
    );
}

#[test]
fn particle_scope_context_tracks_local_spyglass_use() {
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        serde_json::from_value::<ItemRegistryCatalog>(serde_json::json!({
            "resource_ids": ["minecraft:spyglass", "minecraft:stone"],
            "protocol_ids": {"minecraft:spyglass": 0, "minecraft:stone": 1}
        }))
        .unwrap(),
        EquipmentAssetCatalog::default(),
    );
    let mut world = WorldStore::new();
    let pose = LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        ..LocalPlayerPoseState::default()
    };
    world.set_local_player_pose(pose);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(0, 1),
    });
    world.set_local_using_item(true);

    let context =
        particle_local_player_scope_context(&world, Some(&runtime), camera_pose_from_world(&world))
            .unwrap();
    assert_eq!(
        context.eye_position,
        camera_eye_position(camera_pose_from_world(&world).unwrap()).map(f64::from)
    );
    assert!(context.first_person);
    assert!(context.scoping);

    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(1, 1),
    });
    assert_eq!(
        particle_local_player_scope_context(&world, Some(&runtime), camera_pose_from_world(&world)),
        None
    );
    assert_eq!(
        particle_local_player_scope_context(&world, None, camera_pose_from_world(&world)),
        None
    );
}

#[test]
fn particle_player_motion_contexts_track_local_and_remote_players() {
    let mut world = WorldStore::new();
    assert!(particle_player_motion_contexts(&world).is_empty());

    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: -0.1,
            y: 0.25,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    // Non-player entities are never nearest-player candidates.
    world.apply_add_entity(test_add_entity(7, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 42,
        uuid: uuid::Uuid::from_u128(42),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: -0.3,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    assert_eq!(
        particle_player_motion_contexts(&world),
        vec![
            ParticlePlayerMotionContext {
                position: [1.0, 2.0, 3.0],
                delta_movement: [-0.1, 0.25, 0.5],
            },
            ParticlePlayerMotionContext {
                position: [4.0, 5.0, 6.0],
                delta_movement: [0.0, -0.3, 0.0],
            },
        ]
    );

    // Vanilla `EntitySelector.NO_SPECTATORS` drops spectator remote players.
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![bbb_protocol::packets::PlayerInfoEntry {
            profile_id: uuid::Uuid::from_u128(42),
            profile: Some(bbb_protocol::packets::GameProfile {
                uuid: uuid::Uuid::from_u128(42),
                name: "RemoteSpectator".to_string(),
                properties: Vec::new(),
            }),
            listed: true,
            latency: 0,
            game_mode: bbb_protocol::packets::GameType::Spectator,
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }],
    });
    assert_eq!(
        particle_player_motion_contexts(&world),
        vec![ParticlePlayerMotionContext {
            position: [1.0, 2.0, 3.0],
            delta_movement: [-0.1, 0.25, 0.5],
        }]
    );

    // ... and the spectator local player as well.
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(particle_player_motion_contexts(&world).is_empty());
}

#[test]
fn hud_player_list_projection_uses_scores_rich_names_and_vanilla_visibility() {
    use bbb_protocol::packets::{
        ChatFormatting, ObjectiveRenderType, PlayerTeamMethod, PlayerTeamParameters,
        ScoreboardDisplaySlot, SetDisplayObjective, SetObjective, SetObjectiveMethod,
        SetObjectiveParameters, SetPlayerTeam, SetScore, TeamCollisionRule, TeamVisibility,
    };
    use bbb_renderer::HudPlayerListScore;

    let mut world = WorldStore::new();
    let entries = [
        (
            1u128,
            "alpha",
            0,
            bbb_protocol::packets::GameType::Survival,
            40,
        ),
        (
            2,
            "Zulu",
            5,
            bbb_protocol::packets::GameType::Spectator,
            900,
        ),
        (3, "beta", 5, bbb_protocol::packets::GameType::Creative, 120),
    ];
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateListed,
            bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
            bbb_protocol::packets::PlayerInfoAction::UpdateLatency,
            bbb_protocol::packets::PlayerInfoAction::UpdateListOrder,
        ],
        entries: entries
            .into_iter()
            .map(|(id, name, list_order, game_mode, latency)| {
                bbb_protocol::packets::PlayerInfoEntry {
                    profile_id: uuid::Uuid::from_u128(id),
                    profile: Some(bbb_protocol::packets::GameProfile {
                        uuid: uuid::Uuid::from_u128(id),
                        name: name.to_string(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency,
                    game_mode,
                    display_name: None,
                    show_hat: false,
                    list_order,
                    chat_session: None,
                }
            })
            .collect(),
    });
    world.apply_tab_list(bbb_protocol::packets::TabList {
        header: Some("Welcome".to_string()),
        footer: Some("Online".to_string()),
    });
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "aqua".to_string(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Aqua"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Aqua,
            player_prefix: bbb_protocol::RichTextComponent::plain("["),
            player_suffix: bbb_protocol::RichTextComponent::plain("]"),
        }),
        players: vec!["beta".to_string()],
    }));
    assert!(world.apply_set_objective(SetObjective {
        objective_name: "tab".to_string(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Health"),
            render_type: ObjectiveRenderType::Integer,
            number_format: None,
        }),
    }));
    assert!(world.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::List,
        objective_name: Some("tab".to_string()),
    }));
    assert!(world.apply_set_score(SetScore {
        owner: "beta".to_string(),
        objective_name: "tab".to_string(),
        score: 7,
        display: None,
        number_format: None,
    }));
    let mut input = ClientInputState::new(true);
    assert_eq!(hud_player_list_from_world(&input, &world, None, 0), None);
    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    let list = hud_player_list_from_world(&input, &world, None, 42).expect("Tab list");
    assert_eq!(list.header.as_deref(), Some("Welcome"));
    assert_eq!(list.footer.as_deref(), Some("Online"));
    assert_eq!(
        list.entries
            .iter()
            .map(|entry| {
                entry
                    .name_runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>(),
        vec!["[beta]", "Zulu", "alpha"]
    );
    assert!(!list.entries[0].spectator);
    assert!(list.entries[1].spectator);
    assert_eq!(list.entries[0].latency, 120);
    assert_eq!(list.entries[0].profile_id, 3);
    assert_eq!(list.entries[0].name_runs[0].color, Some(0x55FFFF));
    assert!(list.entries[1].name_runs.iter().all(|run| run.style.italic));
    let HudPlayerListScore::Integer(score_runs) =
        list.entries[0].score.as_ref().expect("integer score")
    else {
        panic!("expected integer player-list score");
    };
    assert_eq!(score_runs[0].text, "7");
    assert_eq!(score_runs[0].color, Some(0xFFFF55));
    let HudPlayerListScore::Integer(missing_score_runs) = list.entries[2]
        .score
        .as_ref()
        .expect("missing score formats zero")
    else {
        panic!("expected integer player-list score");
    };
    assert_eq!(missing_score_runs[0].text, "0");
    assert_eq!(list.tick_count, 42);

    assert!(world.apply_set_objective(SetObjective {
        objective_name: "tab".to_string(),
        method: SetObjectiveMethod::Change.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Health"),
            render_type: ObjectiveRenderType::Hearts,
            number_format: None,
        }),
    }));
    let hearts = hud_player_list_from_world(&input, &world, None, 43).expect("heart list");
    assert_eq!(
        hearts.entries[0].score,
        Some(HudPlayerListScore::Hearts {
            value: 7,
            hp_text: "3.5hp".to_string(),
            compact_text: "3.5".to_string(),
        })
    );
    assert_eq!(
        hearts.entries[2].score,
        Some(HudPlayerListScore::Hearts {
            value: 0,
            hp_text: "0.0hp".to_string(),
            compact_text: "0.0".to_string(),
        })
    );

    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert_eq!(hud_player_list_from_world(&input, &world, None, 44), None);
}

#[test]
fn hud_spectator_projection_substitutes_root_and_player_pages() {
    let mut world = WorldStore::new();
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    let profile_id = uuid::Uuid::from_u128(0x1234);
    world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
        actions: vec![
            bbb_protocol::packets::PlayerInfoAction::AddPlayer,
            bbb_protocol::packets::PlayerInfoAction::UpdateListed,
            bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![bbb_protocol::packets::PlayerInfoEntry {
            profile_id,
            profile: Some(bbb_protocol::packets::GameProfile {
                uuid: profile_id,
                name: "Target".to_string(),
                properties: Vec::new(),
            }),
            listed: true,
            latency: 0,
            game_mode: bbb_protocol::packets::GameType::Survival,
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    assert!(input.handle_spectator_hotbar_action(&world, &mut counters, &None));

    let HudHotbarMode::Spectator(Some(root)) =
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now())
    else {
        panic!("spectator root hotbar");
    };
    assert!(root.alpha > 0.99);
    assert_eq!(root.selected_slot, None);
    assert_eq!(
        root.prompt_runs[0].text,
        "Press a key to select a command, and again to use it."
    );
    assert!(matches!(
        root.slots[0].as_ref().unwrap(),
        bbb_renderer::HudSpectatorGuiSlot {
            icon: bbb_renderer::HudSpectatorIcon::TeleportToPlayer,
            enabled: true,
            ..
        }
    ));
    assert!(matches!(
        root.slots[1].as_ref().unwrap(),
        bbb_renderer::HudSpectatorGuiSlot {
            icon: bbb_renderer::HudSpectatorIcon::TeleportToTeam,
            enabled: false,
            ..
        }
    ));

    input.handle_spectator_hotbar_selected(&world, &mut counters, &None, 0);
    input.handle_spectator_hotbar_selected(&world, &mut counters, &None, 0);
    let HudHotbarMode::Spectator(Some(players)) =
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now())
    else {
        panic!("spectator player hotbar");
    };
    assert_eq!(players.selected_slot, None);
    assert_eq!(
        players.prompt_runs[0].text,
        "Select a player to teleport to"
    );
    let player = players.slots[0].as_ref().unwrap();
    assert_eq!(player.name_runs[0].text, "Target");
    assert!(matches!(
        player.icon,
        bbb_renderer::HudSpectatorIcon::Player {
            skin: bbb_renderer::EntityPlayerSkin::ProfiledDefault(_)
        }
    ));
}

#[test]
fn hud_spectator_projection_preserves_ordinary_and_hidden_hotbar_modes() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Ordinary
    ));
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(matches!(
        hud_hotbar_mode_from_world(&input, &world, None, Instant::now()),
        HudHotbarMode::Hidden
    ));
}

#[test]
fn hud_scoreboard_projection_localizes_rich_team_and_score_runs() {
    use bbb_protocol::packets::{
        ChatFormatting, NumberFormatSummary, ObjectiveRenderType, PlayerTeamMethod,
        PlayerTeamParameters, ScoreboardDisplaySlot, SetDisplayObjective, SetObjective,
        SetObjectiveMethod, SetObjectiveParameters, SetPlayerTeam, SetScore, TeamCollisionRule,
        TeamVisibility,
    };

    let styled = |text: &str, color: Option<&str>, bold: bool| {
        let mut entries = vec![NbtSummaryEntry {
            name: "text".to_string(),
            value: NbtSummaryValue::String(text.to_string()),
        }];
        if let Some(color) = color {
            entries.push(NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String(color.to_string()),
            });
        }
        if bold {
            entries.push(NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            });
        }
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(entries))
            .unwrap()
    };
    let mut world = WorldStore::new();
    assert!(world.apply_set_objective(SetObjective {
        objective_name: "kills".to_string(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: styled("Kills", None, true),
            render_type: ObjectiveRenderType::Integer,
            number_format: None,
        }),
    }));
    assert!(world.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::Sidebar,
        objective_name: Some("kills".to_string()),
    }));
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "red".to_string(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Red"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Red,
            player_prefix: bbb_protocol::RichTextComponent::plain("["),
            player_suffix: bbb_protocol::RichTextComponent::plain("]"),
        }),
        players: vec!["Alice".to_string()],
    }));
    assert!(world.apply_set_score(SetScore {
        owner: "Alice".to_string(),
        objective_name: "kills".to_string(),
        score: 9,
        display: Some(styled("Ace", Some("aqua"), false)),
        number_format: Some(NumberFormatSummary::Styled(
            bbb_protocol::ComponentStyle::default(),
        )),
    }));

    let mut input = ClientInputState::new(true);
    let sidebar = hud_scoreboard_sidebar_from_world(&input, &world, None)
        .expect("scoreboard sidebar projection");
    assert_eq!(sidebar.title_runs[0].text, "Kills");
    assert!(sidebar.title_runs[0].style.bold);
    assert_eq!(
        sidebar.entries[0]
            .name_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["[", "Ace", "]"]
    );
    assert_eq!(sidebar.entries[0].name_runs[0].color, Some(0xFF5555));
    assert_eq!(sidebar.entries[0].name_runs[1].color, Some(0x55FFFF));
    assert_eq!(sidebar.entries[0].name_runs[2].color, Some(0xFF5555));
    assert_eq!(sidebar.entries[0].score_runs[0].text, "9");
    assert_eq!(sidebar.entries[0].score_runs[0].color, None);

    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert_eq!(
        hud_scoreboard_sidebar_from_world(&input, &world, None),
        None
    );
}

#[test]
fn particle_entity_target_contexts_track_world_entity_positions() {
    let mut world = WorldStore::new();
    assert!(particle_entity_target_contexts(&world).is_empty());

    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.apply_add_entity(test_add_entity(77, VANILLA_ENTITY_TYPE_PLAYER_ID));

    let contexts = particle_entity_target_contexts(&world);

    assert_eq!(contexts.len(), 2);
    assert!(contexts.contains(&ParticleEntityTargetContext {
        entity_id: 42,
        position: [1.0, 2.0, 3.0],
    }));
    assert!(contexts.contains(&ParticleEntityTargetContext {
        entity_id: 77,
        position: [1.0, 2.0, 3.0],
    }));
}

#[test]
fn particle_sound_event_state_preserves_positioned_sound_metadata() {
    let state = particle_sound_event_state(ParticleSoundEvent {
        sound_event_id: "minecraft:block.pointed_dripstone.drip_water".to_string(),
        source: "block".to_string(),
        position: [1.25, 2.5, -3.75],
        volume: 0.65,
        pitch: 1.0,
        seed: 12345,
        distance_delay: false,
    });

    assert_eq!(state.sound.kind, "direct");
    assert_eq!(
        state.sound.location.as_deref(),
        Some("minecraft:block.pointed_dripstone.drip_water")
    );
    assert_eq!(state.sound.registry_id, None);
    assert_eq!(state.source, "block");
    assert_eq!(state.position.x, 1.25);
    assert_eq!(state.position.y, 2.5);
    assert_eq!(state.position.z, -3.75);
    assert_eq!(state.volume, 0.65);
    assert_eq!(state.pitch, 1.0);
    assert_eq!(state.seed, 12345);
    assert!(!state.distance_delay);
}

#[test]
fn primed_tnt_smoke_particle_batch_matches_vanilla_client_tick_spawn() {
    let batch = primed_tnt_smoke_particle_batch(
        vec![bbb_world::PrimedTntSmokeParticleState {
            entity_id: 42,
            position: bbb_world::EntityVec3 {
                x: 3.0,
                y: 64.0,
                z: -5.0,
            },
        }],
        1,
    );

    assert_eq!(batch.commands.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, VANILLA_PARTICLE_TYPE_SMOKE_ID);
    assert_eq!(command.particle_id, "minecraft:smoke");
    assert_eq!(command.position, [3.0, 64.5, -5.0]);
    assert_eq!(command.velocity, [0.0, 0.0, 0.0]);
    assert!(!command.override_limiter);
    assert!(!command.always_show);
    assert_eq!(command.raw_options_len, 0);
}

#[test]
fn primed_tnt_smoke_particle_batch_emits_once_per_advanced_tick() {
    let batch = primed_tnt_smoke_particle_batch(
        vec![bbb_world::PrimedTntSmokeParticleState {
            entity_id: 42,
            position: bbb_world::EntityVec3 {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
        }],
        3,
    );

    assert_eq!(batch.commands.len(), 3);
    assert!(batch
        .commands
        .iter()
        .all(|command| command.position == [0.0, 10.5, 0.0]));
}

#[test]
fn entity_client_tick_particle_batch_maps_ravager_and_fangs_particles() {
    let batch = entity_client_tick_particle_batch(
        vec![bbb_world::RavagerStunParticleState {
            entity_id: 76,
            position: bbb_world::EntityVec3 {
                x: 1.25,
                y: 65.9,
                z: -0.5,
            },
        }],
        vec![bbb_world::EvokerFangsCritParticleState {
            entity_id: 78,
            position: bbb_world::EntityVec3 {
                x: 0.9,
                y: 65.4,
                z: -1.8,
            },
            velocity: bbb_world::EntityVec3 {
                x: -0.1,
                y: 0.45,
                z: 0.2,
            },
        }],
    );

    assert_eq!(batch.commands.len(), 2);
    let stun = &batch.commands[0];
    assert_eq!(
        stun.particle_type_id,
        VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID
    );
    assert_eq!(stun.particle_id, "minecraft:entity_effect");
    assert_eq!(stun.position, [1.25, 65.9, -0.5]);
    assert_eq!(stun.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(
        stun.option_color,
        Some([0.49803922, 0.5137255, 0.57254905, 1.0])
    );

    let crit = &batch.commands[1];
    assert_eq!(crit.particle_type_id, VANILLA_PARTICLE_TYPE_CRIT_ID);
    assert_eq!(crit.particle_id, "minecraft:crit");
    assert_eq!(crit.position, [0.9, 65.4, -1.8]);
    assert_eq!(crit.velocity, [-0.1, 0.45, 0.2]);
    assert_eq!(crit.option_color, None);
}

#[test]
fn particle_light_for_world_samples_chunk_light_or_full_bright_fallback() {
    let missing = WorldStore::new();
    assert_eq!(
        particle_light_for_world(&missing, [0.5, 1.0, 0.5]),
        [1.0, 1.0]
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        42,
        9,
    ));

    assert_eq!(
        particle_light_for_world(&world, [0.5, 1.25, 0.5]),
        [0.0, 9.0 / 15.0]
    );
}

#[test]
fn entity_animation_partial_tick_tracks_time_since_last_client_tick() {
    let now = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();

    assert_eq!(ticks.entity_partial_tick(), 0.0);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, now),
        0
    );
    assert_eq!(ticks.entity_partial_tick(), 0.0);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, now + Duration::from_millis(25)),
        0
    );
    assert_eq!(ticks.entity_partial_tick(), 0.5);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, now + Duration::from_millis(75)),
        1
    );
    assert_eq!(ticks.entity_partial_tick(), 0.5);
}

#[test]
fn entity_client_tick_observation_reports_recent_work_and_running_peak() {
    let mut counters = NetCounters::default();
    record_entity_client_tick_observation(
        &mut counters,
        Duration::from_micros(20),
        EntityClientTickWork {
            scanned_entities: 500,
            mutated_entities: 498,
            preparation_duration: Duration::from_micros(5),
            schedule_duration: Duration::from_micros(4),
            projectile_duration: Duration::from_micros(3),
            interpolation_duration: Duration::from_micros(2),
            block_effect_duration: Duration::from_micros(6),
            block_effect_entities: 42,
            block_effect_blocks_visited: 84,
            block_effect_callbacks_applied: 21,
            animation_duration: Duration::from_micros(1),
        },
    );
    record_entity_client_tick_observation(
        &mut counters,
        Duration::from_micros(10),
        EntityClientTickWork {
            scanned_entities: 100,
            mutated_entities: 100,
            preparation_duration: Duration::from_micros(4),
            schedule_duration: Duration::from_micros(3),
            projectile_duration: Duration::from_micros(2),
            interpolation_duration: Duration::from_micros(1),
            block_effect_duration: Duration::from_micros(4),
            block_effect_entities: 11,
            block_effect_blocks_visited: 22,
            block_effect_callbacks_applied: 7,
            animation_duration: Duration::from_nanos(500),
        },
    );

    assert_eq!(counters.simulation_entity_client_tick_nanos, 10_000);
    assert_eq!(counters.simulation_entity_client_tick_peak_nanos, 20_000);
    assert_eq!(
        counters.simulation_entity_client_tick_preparation_nanos,
        4_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_preparation_peak_nanos,
        5_000
    );
    assert_eq!(counters.simulation_entity_client_tick_schedule_nanos, 3_000);
    assert_eq!(
        counters.simulation_entity_client_tick_schedule_peak_nanos,
        4_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_projectile_nanos,
        2_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_projectile_peak_nanos,
        3_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_interpolation_nanos,
        1_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_interpolation_peak_nanos,
        2_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_nanos,
        4_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_peak_nanos,
        6_000
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_entities,
        11
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_blocks_visited,
        22
    );
    assert_eq!(
        counters.simulation_entity_client_tick_block_effect_callbacks_applied,
        7
    );
    assert_eq!(counters.simulation_entity_client_tick_animation_nanos, 500);
    assert_eq!(
        counters.simulation_entity_client_tick_animation_peak_nanos,
        1_000
    );
    assert_eq!(counters.simulation_entity_client_tick_scanned_entities, 100);
    assert_eq!(counters.simulation_entity_client_tick_mutated_entities, 100);
}

#[test]
fn lightmap_tick_state_matches_vanilla_block_light_flicker_formula() {
    let mut lightmap = LightmapTickState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);

    let first_delta = (expected_random.next_float() - expected_random.next_float())
        * expected_random.next_float()
        * expected_random.next_float()
        * 0.1;
    let first_flicker = first_delta * 0.9;
    let first_factor = lightmap.advance(1);
    assert!((first_factor - (VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR + first_flicker)).abs() < 1e-6);

    let second_delta = (expected_random.next_float() - expected_random.next_float())
        * expected_random.next_float()
        * expected_random.next_float()
        * 0.1;
    let second_flicker = (first_flicker + second_delta) * 0.9;
    let second_factor = lightmap.advance(1);
    assert!(
        (second_factor - (VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR + second_flicker)).abs() < 1e-6
    );
}

#[test]
fn lightmap_environment_uses_vanilla_dimension_attributes() {
    let overworld = world_with_dimension(0, "minecraft:overworld");
    let overworld_environment = lightmap_environment_for_world(&overworld, 0.75, 1.25);
    assert_eq!(overworld_environment.sky_factor, 1.0);
    assert_eq!(overworld_environment.ambient_light, 0.0);
    assert_close3(
        overworld_environment.sky_light_color,
        VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    );
    assert_close3(overworld_environment.ambient_color, [10.0 / 255.0; 3]);
    assert_eq!(overworld_environment.brightness_factor, 0.75);
    assert_eq!(overworld_environment.block_factor, 1.25);
    assert_eq!(overworld_environment.level_lighting, LevelLighting::Default);

    let nether = world_with_dimension(1, "minecraft:the_nether");
    let nether_environment = lightmap_environment_for_world(&nether, 0.5, 1.4);
    assert_eq!(nether_environment.sky_factor, 0.0);
    assert_eq!(nether_environment.ambient_light, 0.1);
    assert_close3(
        nether_environment.sky_light_color,
        [122.0 / 255.0, 122.0 / 255.0, 1.0],
    );
    assert_close3(
        nether_environment.ambient_color,
        [48.0 / 255.0, 40.0 / 255.0, 33.0 / 255.0],
    );
    assert_eq!(nether_environment.level_lighting, LevelLighting::Nether);

    let end = world_with_dimension(2, "minecraft:the_end");
    let end_environment = lightmap_environment_for_world(&end, 0.5, 1.4);
    assert_eq!(end_environment.sky_factor, 0.0);
    assert_eq!(end_environment.ambient_light, 0.25);
    assert_close3(
        end_environment.sky_light_color,
        [172.0 / 255.0, 96.0 / 255.0, 205.0 / 255.0],
    );
    assert_close3(
        end_environment.ambient_color,
        [63.0 / 255.0, 71.0 / 255.0, 63.0 / 255.0],
    );
    assert_eq!(end_environment.level_lighting, LevelLighting::Default);
}

#[test]
fn lightmap_environment_uses_overworld_day_timeline_sky_light_attributes() {
    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut overworld, 18_000);
    let night_environment = lightmap_environment_for_world(&overworld, 0.5, 1.4);
    assert_eq!(night_environment.sky_factor, 0.24);
    assert_close3(
        night_environment.sky_light_color,
        [122.0 / 255.0, 122.0 / 255.0, 1.0],
    );

    set_world_day_time(&mut overworld, 6_000);
    let noon_environment = lightmap_environment_for_world(&overworld, 0.5, 1.4);
    assert_eq!(noon_environment.sky_factor, 1.0);
    assert_close3(
        noon_environment.sky_light_color,
        VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    );

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    set_world_day_time(&mut nether, 18_000);
    let nether_environment = lightmap_environment_for_world(&nether, 0.5, 1.4);
    assert_eq!(nether_environment.sky_factor, 0.0);
    assert_close3(
        nether_environment.sky_light_color,
        [122.0 / 255.0, 122.0 / 255.0, 1.0],
    );
}

#[test]
fn lightmap_environment_applies_overworld_weather_layers_after_timeline() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    set_world_weather(&mut world, 1.0, 0.0);
    let rain_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert!((rain_environment.sky_factor - 0.7625).abs() < 1e-6);
    assert_close3(
        rain_environment.sky_light_color,
        [213.0 / 255.0, 213.0 / 255.0, 1.0],
    );

    set_world_weather(&mut world, 1.0, 1.0);
    let thunder_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert!((thunder_environment.sky_factor - 0.5992187).abs() < 1e-6);
    assert_close3(
        thunder_environment.sky_light_color,
        [185.0 / 255.0, 185.0 / 255.0, 1.0],
    );

    set_world_weather(&mut world, 0.0, 1.0);
    let dry_thunder_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert_eq!(dry_thunder_environment.sky_factor, 1.0);
    assert_close3(
        dry_thunder_environment.sky_light_color,
        VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    );
}

#[test]
fn entity_shadow_sky_darken_uses_gameplay_timeline_and_weather_layers() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    assert_eq!(entity_shadow_sky_darken(&world), 0);

    set_world_day_time(&mut world, 18_000);
    assert_eq!(entity_shadow_sky_darken(&world), 11);

    set_world_day_time(&mut world, 6_000);
    set_world_weather(&mut world, 1.0, 0.0);
    assert_eq!(entity_shadow_sky_darken(&world), 3);
    set_world_weather(&mut world, 1.0, 1.0);
    assert_eq!(entity_shadow_sky_darken(&world), 5);
    set_world_weather(&mut world, 0.0, 1.0);
    assert_eq!(entity_shadow_sky_darken(&world), 0);
}

#[test]
fn lightmap_tick_state_smooths_rain_fog_multiplier_like_vanilla_atmospheric_fog() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        0,
        15,
    ));
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_precipitation(0, true),
    ]));
    set_world_weather(&mut world, 1.0, 0.0);

    let mut lightmap = LightmapTickState::with_seed(0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);
    assert!((lightmap.rain_fog_multiplier() - 0.2).abs() < 1e-6);

    lightmap.advance_rain_fog_for_world(1, &world, &textures);
    assert!((lightmap.rain_fog_multiplier() - 0.36).abs() < 1e-6);

    set_world_weather(&mut world, 0.0, 0.0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);
    assert!((lightmap.rain_fog_multiplier() - 0.288).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_gates_rain_fog_multiplier_by_camera_sky_light() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        0,
        8,
    ));
    assert_eq!(
        camera_block_position(&world),
        Some(BlockPos { x: 0, y: 1, z: 0 })
    );
    assert_eq!(
        world
            .sample_block_light(BlockPos { x: 0, y: 1, z: 0 })
            .unwrap()
            .sky,
        8
    );
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_precipitation(0, true),
    ]));
    set_world_weather(&mut world, 1.0, 0.0);

    let mut lightmap = LightmapTickState::with_seed(0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);

    assert_eq!(lightmap.rain_fog_multiplier(), 0.0);
}

#[test]
fn lightmap_tick_state_halves_rain_fog_target_when_camera_biome_has_no_precipitation() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_sky_light(
        world.dimension(),
        42,
        15,
    ));
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_precipitation(42, false),
    ]));
    set_world_weather(&mut world, 1.0, 0.0);

    let mut lightmap = LightmapTickState::with_seed(0);
    lightmap.advance_rain_fog_for_world(1, &world, &textures);

    assert!((lightmap.rain_fog_multiplier() - 0.1).abs() < 1e-6);
}

#[test]
fn lightmap_environment_applies_client_sky_flash_layer_after_timeline() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    let baseline_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    assert!(baseline_environment.sky_factor < 1.0);

    world.set_sky_flash_time(2);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, false);
    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.sky_factor, 1.0);
    assert_close3(
        environment.sky_light_color,
        baseline_environment.sky_light_color,
    );
}

#[test]
fn lightmap_environment_hides_client_sky_flash_when_option_is_enabled() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    let baseline_environment = lightmap_environment_for_world(&world, 0.5, 1.4);
    world.set_sky_flash_time(2);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, true);

    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - baseline_environment.sky_factor).abs() < 1e-6);
}

#[test]
fn renderer_frame_sky_flash_environment_extracts_after_client_level_tick() {
    let source = include_str!("../runtime.rs");
    let sky_flash_tick = source
        .find("world.advance_sky_flash_time(advanced_ticks);")
        .expect("pump should advance sky flash before extracting renderer environments");
    let lightmap_extract = source
        .find("let lightmap_environment = lightmap_ticks.environment_for_world(world);")
        .expect("pump should extract the lightmap environment");
    let clear_extract = source
        .find("let clear_color = clear_color_for_world_at_camera_with_water_vision(")
        .expect("pump should extract the clear color");
    let fog_extract = source
        .find("let fog_environment = if input.debug_fog_enabled()")
        .expect("pump should extract the fog environment");
    let sky_extract = source
        .find("let sky_environment = sky_environment_for_world_at_camera(")
        .expect("pump should extract the sky environment");
    let cloud_extract = source
        .find("let cloud_environment = cloud_environment_for_world(world);")
        .expect("pump should extract the cloud environment");

    for extraction in [
        lightmap_extract,
        clear_extract,
        fog_extract,
        sky_extract,
        cloud_extract,
    ] {
        assert!(
            sky_flash_tick < extraction,
            "vanilla `Minecraft.tick` runs `ClientLevel.tick` before `GameRenderer.extract`"
        );
    }

    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    world.set_sky_flash_time(1);
    world.advance_sky_flash_time(1);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, false);
    lightmap.advance_for_world(0, &world);

    let baseline = lightmap_environment_for_world(&world, 0.5, 1.4);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - baseline.sky_factor).abs() < 1e-6);
}

#[test]
fn renderer_frame_hud_extracts_after_input_and_use_item_tick() {
    let source = include_str!("../runtime.rs");
    let input_advance = source
        .find("advance_player_input_for_client_ticks(")
        .expect("pump should advance player input before HUD extraction");
    let destroy_advance = source
        .find("advance_destroying_block_at_partial_tick(")
        .expect("pump should advance destroy input before HUD extraction");
    let use_advance = source
        .find("advance_using_item_at_partial_tick(")
        .expect("pump should advance use-item input before HUD extraction");
    let using_item_tick = source
        .find("world.advance_local_using_item_ticks(advanced_ticks);")
        .expect("pump should advance local use-item ticks before HUD extraction");
    let hud_extract = source
        .find("let local_player = world.local_player();")
        .expect("pump should extract HUD state from the local player");
    let selected_slot_extract = source
        .find("let hud_selected_slot = local_player.selected_hotbar_slot;")
        .expect("pump should extract the selected hotbar slot");
    let hotbar_mode_extract = source
        .find("hud_hotbar_mode_from_world(input, world, item_runtime, render_started_at);")
        .expect("pump should extract the spectator/ordinary hotbar mode");
    let hotbar_icons_extract = source
        .find("let hud_hotbar_item_icons = if ordinary_hotbar {")
        .expect("pump should extract hotbar item icons");
    let debug_options_screen_extract = source
        .find(
            "let hud_debug_options_screen = hud_debug_options_screen(input, world, surface_size);",
        )
        .expect("pump should extract debug options screen HUD state");
    let pause_screen_extract = source
        .find("let hud_pause_screen = if hud_debug_options_screen.is_some()")
        .expect("pump should extract pause screen HUD state");
    let stats_screen_extract = source
        .find("let hud_stats_screen = if hud_debug_options_screen.is_some()")
        .expect("pump should extract stats screen HUD state");
    let sign_editor_extract = source
        .find("let hud_sign_editor_screen = if hud_debug_options_screen.is_some()")
        .expect("pump should extract sign editor HUD state");
    let inventory_screen_extract = source
        .find("hud_inventory_screen_with_local_state(")
        .expect("pump should extract inventory screen HUD state");

    for advance in [input_advance, destroy_advance, use_advance, using_item_tick] {
        assert!(
            advance < hud_extract,
            "vanilla `Minecraft.tick` handles keybinds before `GameRenderer.extractGui`"
        );
    }
    for extraction in [
        selected_slot_extract,
        hotbar_mode_extract,
        hotbar_icons_extract,
        debug_options_screen_extract,
        pause_screen_extract,
        stats_screen_extract,
        sign_editor_extract,
        inventory_screen_extract,
    ] {
        assert!(
            hud_extract < extraction,
            "HUD frame fields should read one post-input local player snapshot"
        );
    }
}

#[test]
fn hud_text_timers_tick_before_projection_like_gui_tick() {
    // Vanilla `Minecraft.tick` runs `Gui.tick` (overlayMessageTime-- /
    // titleTime--, Gui.java:1152-1166) once per client tick — outside the
    // tick-rate manager's freeze gate — before `GameRenderer.extractGui`
    // reads the countdowns with the frame partial tick.
    let source = include_str!("../runtime.rs");
    let gui_tick = source
        .find("world.advance_hud_text_ticks(advanced_ticks);")
        .expect("pump should tick the HUD text timers like Gui.tick");
    let action_bar_extract = source
        .find("hud_action_bar_text_from_world(world, item_runtime, entity_partial_tick);")
        .expect("pump should project the action bar state");
    let title_extract = source
        .find("hud_title_text_from_world(world, item_runtime, entity_partial_tick);")
        .expect("pump should project the title state");
    assert!(
        gui_tick < action_bar_extract && action_bar_extract < title_extract,
        "vanilla `Minecraft.tick` runs `Gui.tick` before `GameRenderer.extractGui`"
    );
}

#[test]
fn animate_player_wake_sleep_overlay_alpha_matches_vanilla_boundaries() {
    use bbb_renderer::HudSleepOverlay;

    assert_eq!(hud_sleep_overlay_from_counter(-1), None);
    assert_eq!(hud_sleep_overlay_from_counter(0), None);
    assert_eq!(
        hud_sleep_overlay_from_counter(1),
        Some(HudSleepOverlay { alpha: 2 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(50),
        Some(HudSleepOverlay { alpha: 110 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(99),
        Some(HudSleepOverlay { alpha: 217 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(100),
        Some(HudSleepOverlay { alpha: 220 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(101),
        Some(HudSleepOverlay { alpha: 198 })
    );
    assert_eq!(
        hud_sleep_overlay_from_counter(109),
        Some(HudSleepOverlay { alpha: 22 })
    );
    assert_eq!(hud_sleep_overlay_from_counter(110), None);
}

#[test]
fn animate_player_wake_sleep_overlay_frame_wiring_is_hide_gui_independent() {
    let runtime = include_str!("../runtime.rs");
    let extraction = runtime
        .find("let hud_sleep_overlay = hud_sleep_overlay_from_world(world);")
        .expect("runtime extracts canonical sleep overlay");
    let hide_gui = runtime
        .find("let hud_advancement_toasts = if input.hide_gui()")
        .expect("ordinary hide-GUI branch remains later");
    let frame = extraction
        + runtime[extraction..]
            .find("hud_sleep_overlay,")
            .expect("renderer frame carries the overlay");
    assert!(
        extraction < hide_gui && hide_gui < frame,
        "sleep overlay extraction stays outside hide-GUI and still reaches the later frame"
    );

    let frame_source = include_str!("render_extract.rs");
    let field = frame_source
        .find("pub(crate) hud_sleep_overlay: Option<HudSleepOverlay>")
        .expect("renderer frame owns the projected overlay");
    let setter = frame_source
        .find("renderer.set_hud_sleep_overlay(frame.hud_sleep_overlay);")
        .expect("frame apply forwards the overlay");
    assert!(field < setter);
}

#[test]
fn chat_deletion_timer_uses_raw_gui_ticks() {
    // Vanilla `Minecraft.tick` advances `Gui.tick` even while the level tick
    // rate is frozen. Deleted-chat replacement therefore follows raw client
    // ticks beside the other GUI timers, not running level ticks.
    let source = include_str!("../runtime.rs");
    let running_tick = source
        .find("let running_ticks = client_animation_ticks.last_running_level_ticks;")
        .expect("the completed client-level batch should expose its running tick count");
    let hud_tick = source
        .find("world.advance_hud_text_ticks(advanced_ticks);")
        .expect("pump should tick GUI text with raw ticks");
    let chat_tick = source
        .find("world.advance_chat_gui_ticks(advanced_ticks);")
        .expect("pump should tick deleted-chat replacement with raw ticks");
    assert!(running_tick < hud_tick && hud_tick < chat_tick);
}

#[test]
fn camera_fov_tick_state_eases_player_abilities_modifier_like_vanilla() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.2,
    });
    let mut fov = CameraFovTickState::default();

    assert!((fov.fov_degrees(1.0) - 70.0).abs() < 1.0e-6);
    fov.advance_for_world(1, &world);
    assert!((fov.fov_degrees(0.0) - 70.0).abs() < 1.0e-6);
    assert!((fov.fov_degrees(1.0) - 61.25).abs() < 1.0e-6);

    fov.advance_for_world(1, &world);
    assert!((fov.fov_degrees(1.0) - 56.875).abs() < 1.0e-6);

    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CREEPER_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    fov.advance_for_world(1, &world);
    assert!((fov.fov_degrees(1.0) - 63.4375).abs() < 1.0e-6);
}

#[test]
fn camera_spinning_effect_ticks_and_interpolates_nausea_like_vanilla() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_NAUSEA_ID,
        260,
        true,
    )));
    let mut spinning = CameraSpinningEffectTickState::default();

    assert_eq!(spinning.effect_for_world(&world, 1.0), None);
    world.advance_client_level_entity_ticks_with_observation(1);
    spinning.advance_for_world(1, &world);

    assert_eq!(spinning.effect_for_world(&world, 0.0), None);
    let effect = spinning.effect_for_world(&world, 0.5).unwrap();
    assert!((effect.intensity - 1.0 / 300.0).abs() < 1e-6);
    assert_eq!(effect.angle_degrees, 10.5);
    let effect = spinning.effect_for_world(&world, 1.0).unwrap();
    assert!((effect.intensity - 1.0 / 150.0).abs() < 1e-6);
    assert_eq!(effect.angle_degrees, 14.0);
}

#[test]
fn camera_spinning_effect_stops_without_resetting_phase_after_nausea_removal() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NAUSEA_ID, 260,)));
    let mut spinning = CameraSpinningEffectTickState::default();
    spinning.advance_for_world(2, &world);
    assert_eq!(
        spinning
            .effect_for_world(&world, 1.0)
            .unwrap()
            .angle_degrees,
        21.0
    );

    assert!(world.apply_remove_mob_effect(RemoveMobEffect {
        entity_id: 42,
        effect_id: VANILLA_MOB_EFFECT_NAUSEA_ID,
    }));
    spinning.advance_for_world(3, &world);
    assert_eq!(spinning.effect_for_world(&world, 1.0), None);

    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NAUSEA_ID, 260,)));
    spinning.advance_for_world(1, &world);
    assert_eq!(
        spinning
            .effect_for_world(&world, 0.0)
            .unwrap()
            .angle_degrees,
        21.0
    );
}

#[test]
fn hud_mob_effect_projection_uses_catalog_category_flags_level_and_duration() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    let mut speed = mob_effect(42, VANILLA_MOB_EFFECT_SPEED_ID, 1_220);
    speed.amplifier = 1;
    speed.flags = MobEffectFlags {
        raw: 0b0011,
        ambient: true,
        visible: true,
        show_icon: false,
        blend: false,
    };
    assert!(world.apply_update_mob_effect(speed));
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });
    let mut blindness = mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1);
    blindness.amplifier = 10;
    assert!(world.apply_update_mob_effect(blindness));

    let effects = hud_mob_effects_from_world(&world, None);
    assert_eq!(effects.len(), 2);
    let speed = effects
        .iter()
        .find(|effect| effect.effect_id == VANILLA_MOB_EFFECT_SPEED_ID)
        .expect("speed effect");
    assert!(speed.beneficial);
    assert_eq!(
        speed.color,
        vanilla_mob_effect_type(VANILLA_MOB_EFFECT_SPEED_ID)
            .unwrap()
            .color
    );
    assert_eq!(speed.amplifier, 1);
    assert_eq!(speed.duration_ticks, 1_220);
    assert!(speed.ambient);
    assert!(!speed.show_icon);
    assert_eq!(speed.display_name, "Speed II");
    assert_eq!(speed.duration_text, "02:02");

    let blindness = effects
        .iter()
        .find(|effect| effect.effect_id == VANILLA_MOB_EFFECT_BLINDNESS_ID)
        .expect("blindness effect");
    assert!(!blindness.beneficial);
    assert_eq!(blindness.display_name, "Blindness");
    assert_eq!(blindness.duration_text, "∞");
}

#[test]
fn hud_mob_effect_projection_requires_local_player_and_formats_vanilla_time_ranges() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CHICKEN_ID));
    assert!(world.apply_update_mob_effect(mob_effect(100, VANILLA_MOB_EFFECT_SPEED_ID, 200,)));
    assert!(hud_mob_effects_from_world(&world, None).is_empty());

    assert_eq!(hud_mob_effect_duration_text(None, 20, 20.0), "00:01");
    assert_eq!(hud_mob_effect_duration_text(None, 72_020, 20.0), "01:00:01");
    assert_eq!(hud_mob_effect_duration_text(None, 20, 0.0), "00:01");
    assert_eq!(
        humanize_resource_path("hero_of_the_village"),
        "Hero Of The Village"
    );
}

#[test]
fn hud_action_bar_and_title_projection_matches_world_state() {
    let mut world = WorldStore::new();
    assert_eq!(hud_action_bar_text_from_world(&world, None, 0.25), None);
    assert_eq!(hud_title_text_from_world(&world, None, 0.25), None);

    world.apply_action_bar_text(bbb_protocol::packets::SetActionBarText::plain(
        "Action ready",
    ));
    world.apply_titles_animation(bbb_protocol::packets::SetTitlesAnimation {
        fade_in: 5,
        stay: 40,
        fade_out: 15,
    });
    world.apply_title_text(bbb_protocol::packets::SetTitleText::plain("Quest complete"));
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText::plain(
        "Return to camp",
    ));

    let action_bar =
        hud_action_bar_text_from_world(&world, None, 0.25).expect("action bar projected");
    assert_eq!(
        action_bar.runs,
        vec![bbb_renderer::HudComponentTextRun::plain("Action ready")]
    );
    assert_eq!(action_bar.remaining_ticks, 60);
    assert_eq!(action_bar.partial_tick, 0.25);
    // Both packet paths are vanilla `setOverlayMessage(component, false)`;
    // only the jukebox now-playing path animates.
    assert!(!action_bar.animate_color);

    let title = hud_title_text_from_world(&world, None, 0.25).expect("title projected");
    assert_eq!(
        title.title_runs,
        vec![bbb_renderer::HudComponentTextRun::plain("Quest complete")]
    );
    assert_eq!(
        title.subtitle_runs,
        vec![bbb_renderer::HudComponentTextRun::plain("Return to camp")]
    );
    assert_eq!(title.remaining_ticks, 60);
    assert_eq!((title.fade_in, title.stay, title.fade_out), (5, 40, 15));
    assert_eq!(title.partial_tick, 0.25);

    // Post-tick countdowns flow through; expired timers stop projecting.
    world.advance_hud_text_ticks(59);
    assert_eq!(
        hud_action_bar_text_from_world(&world, None, 0.0)
            .expect("last action bar tick")
            .remaining_ticks,
        1
    );
    assert_eq!(
        hud_title_text_from_world(&world, None, 0.0)
            .expect("last title tick")
            .remaining_ticks,
        1
    );
    world.advance_hud_text_ticks(1);
    assert_eq!(hud_action_bar_text_from_world(&world, None, 0.0), None);
    assert_eq!(hud_title_text_from_world(&world, None, 0.0), None);

    // A re-set title without a subtitle projects an empty subtitle line.
    world.apply_title_text(bbb_protocol::packets::SetTitleText::plain("Solo"));
    let solo = hud_title_text_from_world(&world, None, 0.5).expect("title without subtitle");
    assert!(solo.subtitle_runs.is_empty());
    assert_eq!(solo.remaining_ticks, 60);

    // A subtitle without an active title never projects (vanilla draws the
    // subtitle only inside the title branch, Gui.java:364).
    world.apply_clear_titles(bbb_protocol::packets::ClearTitles { reset_times: false });
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText::plain("Orphan"));
    assert_eq!(hud_title_text_from_world(&world, None, 0.0), None);
}

#[test]
fn hud_action_bar_and_title_projection_localizes_style_and_font_without_interactions() {
    let component =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".to_string(),
                value: NbtSummaryValue::String("missing.action".to_string()),
            },
            NbtSummaryEntry {
                name: "fallback".to_string(),
                value: NbtSummaryValue::String("Ready, %s".to_string()),
            },
            NbtSummaryEntry {
                name: "with".to_string(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::String("Alex".to_string())]),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "italic".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "underlined".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "strikethrough".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "obfuscated".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("#123456".to_string()),
            },
            NbtSummaryEntry {
                name: "font".to_string(),
                value: NbtSummaryValue::String("example:title".to_string()),
            },
            NbtSummaryEntry {
                name: "click_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("run_command".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "command".to_string(),
                        value: NbtSummaryValue::String("/spawn".to_string()),
                    },
                ]),
            },
            NbtSummaryEntry {
                name: "hover_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("show_text".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "value".to_string(),
                        value: NbtSummaryValue::String("Details".to_string()),
                    },
                ]),
            },
        ]))
        .unwrap();
    let expected = bbb_renderer::HudComponentTextRun {
        visual: HudStyledTextRun {
            text: "Ready, Alex".to_string(),
            style: HudTextStyle {
                bold: true,
                italic: true,
                underlined: true,
                strikethrough: true,
                obfuscated: true,
            },
            color: Some(0x12_34_56),
        },
        font: Some("example:title".to_string()),
    };
    let mut world = WorldStore::new();
    world.apply_system_chat(bbb_protocol::packets::SystemChat {
        content: Box::new(component.clone()),
        overlay: true,
    });
    world.apply_title_text(bbb_protocol::packets::SetTitleText {
        content: component.clone(),
    });
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText { content: component });

    assert_eq!(
        hud_action_bar_text_from_world(&world, None, 0.0)
            .expect("overlay SystemChat projected")
            .runs,
        vec![expected.clone()]
    );
    let title = hud_title_text_from_world(&world, None, 0.0).expect("title projected");
    assert_eq!(title.title_runs, vec![expected.clone()]);
    assert_eq!(title.subtitle_runs, vec![expected]);
}

#[test]
fn hud_debug_overlay_projects_version_and_camera_position_lines() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(57);
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);
    assert_eq!(
        hud_debug_overlay(
            &input,
            &world,
            None,
            surface_size,
            &fps_sampler,
            VANILLA_UNLIMITED_FRAMERATE_LIMIT,
            true,
            &HudDebugNetworkSampler::default(),
            &HudDebugTpsSampler::default(),
            &NetCounters::default(),
        ),
        None
    );

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [10.25, 64.0, -5.75],
            y_rot: 90.0,
            x_rot: 15.0,
            eye_height: 1.62,
        }),
        surface_size,
        &fps_sampler,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("debug overlay should project when F3 is visible");

    assert_eq!(
        overlay.left_lines[0],
        format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-native)")
    );
    assert_eq!(overlay.left_lines[1], "57 fps T: inf vsync");
    assert!(overlay
        .left_lines
        .contains(&"XYZ: 10.250 / 64.00000 / -5.750".to_string()));
    assert!(overlay.left_lines.contains(&"Block: 10 64 -6".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Chunk: 0 4 -1 [0 31 in r.0.-1.mca]".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Facing: west (Towards negative X) (90.0 / 15.0)".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Section-relative: 10 00 10".to_string()));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Mem: ")));
    assert!(overlay
        .right_lines
        .contains(&"Display: 320x240 (wgpu)".to_string()));
    assert!(overlay
        .right_lines
        .contains(&"Filtering: Nearest".to_string()));
    assert_eq!(
        overlay.debug_crosshair,
        Some(HudDebugCrosshair {
            x_rot_degrees: 15.0,
            y_rot_degrees: 90.0,
            gui_scale: 1,
        })
    );
}

#[test]
fn hud_debug_overlay_projects_vanilla_default_profile_entries() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_custom_payload(bbb_protocol::packets::CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: bbb_protocol::packets::CustomPayloadBody::Brand {
            brand: "vanilla".to_string(),
        },
    });
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(60);
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [10.25, 64.0, -5.75],
            y_rot: 90.0,
            x_rot: 15.0,
            eye_height: 1.62,
        }),
        surface_size,
        &fps_sampler,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("default debug profile should project while F3 is visible");

    // Vanilla DebugScreenEntries.PROFILES projects these default entries into the overlay.
    assert_eq!(
        overlay.left_lines[0],
        format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-native)")
    );
    assert_eq!(overlay.left_lines[1], "60 fps T: inf vsync");
    assert!(overlay
        .left_lines
        .contains(&"\"vanilla\" server, 0 tx, 0 rx".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"XYZ: 10.250 / 64.00000 / -5.750".to_string()));
    assert!(overlay.left_lines.contains(&"Block: 10 64 -6".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Chunk: 0 4 -1 [0 31 in r.0.-1.mca]".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Section-relative: 10 00 10".to_string()));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Mem: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Allocation rate: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Allocated: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("Java: ")));
    assert!(overlay
        .right_lines
        .iter()
        .any(|line| line.starts_with("CPU: ")));
    assert!(overlay
        .right_lines
        .contains(&"Display: 320x240 (wgpu)".to_string()));
    assert!(overlay.right_lines.contains(&"B: 2".to_string()));
    assert!(overlay
        .right_lines
        .contains(&"Filtering: Nearest".to_string()));
    assert_eq!(
        overlay.debug_crosshair,
        Some(HudDebugCrosshair {
            x_rot_degrees: 15.0,
            y_rot_degrees: 90.0,
            gui_scale: 1,
        })
    );
}

#[test]
fn hud_debug_overlay_projects_performance_profile_fps_when_overlay_hidden() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.load_debug_screen_profile(crate::debug_entries::DebugScreenProfile::Performance);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(144);

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &fps_sampler,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        false,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("performance profile FPS is always-on");

    assert_eq!(overlay.left_lines, vec!["144 fps T: inf".to_string()]);
    assert!(overlay.right_lines.is_empty());
    assert_eq!(overlay.debug_crosshair, None);
}

#[test]
fn hud_debug_overlay_projects_performance_profile_gpu_utilization_when_overlay_visible() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.load_debug_screen_profile(crate::debug_entries::DebugScreenProfile::Performance);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(144);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &fps_sampler,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        false,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("performance profile overlay should include enabled entries");

    assert!(overlay.left_lines.contains(&"144 fps T: inf".to_string()));
    assert!(overlay.right_lines.contains(&"GPU: 0%".to_string()));
}

#[test]
fn hud_debug_overlay_projects_custom_day_count_entry_from_world_day_clock() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    set_world_day_time(&mut world, 48_123);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::DayCount,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        false,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom day-count entry should show while always-on");

    assert_eq!(overlay.left_lines, vec!["Day #2".to_string()]);
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_light_levels_from_camera_feet_block() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut chunk = empty_lightmap_test_chunk_with_biome(world.dimension(), 42);
    let mut sky = vec![0; TEST_LIGHT_ARRAY_BYTES];
    let mut block = vec![0; TEST_LIGHT_ARRAY_BYTES];
    let nibble_index = section_block_index(0, 1, 0);
    set_test_light_nibble(&mut sky, nibble_index, 4);
    set_test_light_nibble(&mut block, nibble_index, 13);
    let light_section_index = 0 - (world.dimension().min_section_y() - 1);
    let light_mask = single_bit_mask(usize::try_from(light_section_index).unwrap());
    chunk.light = LightData {
        sky_y_mask: light_mask.clone(),
        block_y_mask: light_mask,
        empty_sky_y_mask: Vec::new(),
        empty_block_y_mask: Vec::new(),
        sky_updates: vec![sky],
        block_updates: vec![block],
    };
    world.insert_decoded_chunk(chunk);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LightLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.25, 1.0, 0.25],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom light entry should show with loaded camera block");

    assert_eq!(
        overlay.left_lines,
        vec!["Client Light: 13 (4 sky, 13 block)".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_heightmaps_from_camera_feet_block() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut chunk = empty_lightmap_test_chunk(world.dimension());
    chunk.heightmaps = vec![
        test_heightmap(1, world.dimension(), &[(2, 3, 9)]),
        test_heightmap(4, world.dimension(), &[(2, 3, 7)]),
    ];
    world.insert_decoded_chunk(chunk);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::Heightmap,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [2.25, 1.0, 3.25],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom heightmap entry should show with loaded camera chunk");

    assert_eq!(
        overlay.left_lines,
        vec![
            "CH S: 9 M: 7 ML: ??".to_string(),
            "SH S: ?? O: ?? M: ?? ML: ??".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_biome_from_camera_feet_block() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:plains"),
            RegistryPacketEntry::stub("minecraft:cherry_grove"),
        ],
    );
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 1));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::Biome,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.25, 1.0, 0.25],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom biome entry should show with loaded camera block");

    assert_eq!(
        overlay.left_lines,
        vec!["Biome: minecraft:cherry_grove".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_suppresses_custom_local_difficulty_without_integrated_server() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LocalDifficulty,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    );

    assert_eq!(overlay, None);
}

#[test]
fn hud_debug_overlay_suppresses_custom_entity_spawn_counts_without_integrated_server() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::EntitySpawnCounts,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    );

    assert_eq!(overlay, None);
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_block_state() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("open".to_string(), "true".to_string()),
    ]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:barrel", &properties)
        .expect("vanilla barrel block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtBlockState,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom looking-at block-state entry should show with a target block");

    assert_eq!(
        overlay.left_lines,
        vec![
            "Targeted Block: 0, 1, 0".to_string(),
            "minecraft:barrel".to_string(),
            "facing: north".to_string(),
            "open: true".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_block_tags() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let stone_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:stone")
        .expect("vanilla block registry contains stone");
    let barrel_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:barrel")
        .expect("vanilla block registry contains barrel");
    let chest_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:chest")
        .expect("vanilla block registry contains chest");
    apply_block_tags(
        &mut world,
        vec![
            ("minecraft:guarded_by_piglins", vec![barrel_id]),
            ("minecraft:mineable/axe", vec![barrel_id, chest_id]),
            ("minecraft:mineable/pickaxe", vec![stone_id]),
        ],
    );
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("open".to_string(), "false".to_string()),
    ]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:barrel", &properties)
        .expect("vanilla barrel block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtBlockTags,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom looking-at block-tags entry should show target tags");

    assert_eq!(
        overlay.left_lines,
        vec![
            "#minecraft:guarded_by_piglins".to_string(),
            "#minecraft:mineable/axe".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_fluid_state() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([("level".to_string(), "1".to_string())]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:water", &properties)
        .expect("vanilla flowing water block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtFluidState,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom looking-at fluid-state entry should show target fluid");

    assert_eq!(
        overlay.left_lines,
        vec![
            "Targeted Fluid: 0, 1, 0".to_string(),
            "minecraft:flowing_water".to_string(),
            "falling: false".to_string(),
            "level: 7".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_fluid_tags() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.record_registry_entries(
        "minecraft:fluid",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:water"),
            RegistryPacketEntry::stub("minecraft:flowing_water"),
            RegistryPacketEntry::stub("minecraft:lava"),
        ],
    );
    apply_fluid_tags(
        &mut world,
        vec![
            ("minecraft:flowing_only", vec![1]),
            ("minecraft:lava", vec![2]),
            ("minecraft:water", vec![0, 1]),
        ],
    );
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let target = BlockPos { x: 0, y: 1, z: 0 };
    let properties = BTreeMap::from([("level".to_string(), "1".to_string())]);
    let block_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:water", &properties)
        .expect("vanilla flowing water block state");
    set_lightmap_test_block(&mut world, target, block_state_id);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtFluidTags,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom looking-at fluid-tags entry should show target tags");

    assert_eq!(
        overlay.left_lines,
        vec![
            "#minecraft:flowing_only".to_string(),
            "#minecraft:water".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_entity() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut entries = (0..=VANILLA_ENTITY_TYPE_ZOMBIE_ID)
        .map(|id| RegistryPacketEntry::stub(format!("minecraft:test_entity_{id}")))
        .collect::<Vec<_>>();
    entries[VANILLA_ENTITY_TYPE_ZOMBIE_ID as usize] =
        RegistryPacketEntry::stub("minecraft:debug_target_zombie");
    world.record_registry_entries("minecraft:entity_type", 0, entries);
    let mut target = test_add_entity(77, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    target.position = Vec3d {
        x: 0.5,
        y: 0.0,
        z: 0.0,
    };
    world.apply_add_entity(target);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtEntity,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom looking-at entity entry should show target entity");

    assert_eq!(
        overlay.left_lines,
        vec![
            "Targeted Entity".to_string(),
            "minecraft:debug_target_zombie".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_entity_type_name_uses_world_registry_source_rules() {
    let mut world = WorldStore::new();
    assert_eq!(
        hud_debug_entity_type_name(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID).as_deref(),
        Some("minecraft:zombie")
    );

    world.record_registry_entries(
        "minecraft:entity_type",
        0,
        vec![RegistryPacketEntry::stub("example:server_first")],
    );
    assert_eq!(
        hud_debug_entity_type_name(&world, 0).as_deref(),
        Some("example:server_first")
    );
    assert_eq!(
        hud_debug_entity_type_name(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        None
    );
}

#[test]
fn hud_debug_overlay_projects_custom_looking_at_entity_tags() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_entity_type_tags(
        &mut world,
        vec![
            ("minecraft:arrows", vec![VANILLA_ENTITY_TYPE_ARROW_ID]),
            ("minecraft:hostile", vec![VANILLA_ENTITY_TYPE_ZOMBIE_ID]),
            ("minecraft:zombies", vec![VANILLA_ENTITY_TYPE_ZOMBIE_ID]),
        ],
    );
    let mut target = test_add_entity(77, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    target.position = Vec3d {
        x: 0.5,
        y: 0.0,
        z: 0.0,
    };
    world.apply_add_entity(target);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::LookingAtEntityTags,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("custom looking-at entity-tags entry should show target tags");

    assert_eq!(
        overlay.left_lines,
        vec![
            "#minecraft:hostile".to_string(),
            "#minecraft:zombies".to_string(),
        ]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_chunk_render_stats_under_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkRenderStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let renderer_counters = bbb_renderer::RendererCounters {
        visible_sections: 7,
        uploaded_sections: 10,
        queued_sections: 3,
        ..Default::default()
    };

    let overlay = hud_debug_overlay_at_partial_tick(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        1.0,
        None,
        &TerrainTextureState::default(),
        &renderer_counters,
        12,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
        &AudioCounters::default(),
        None,
    )
    .expect("chunk render stats should be allowed under reduced debug info");

    assert_eq!(
        overlay.left_lines,
        vec!["C: 7/10 (s) D: 12, pC: 003, aB: 00".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_suppresses_custom_chunk_generation_stats_without_integrated_server() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkGenerationStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    );

    assert_eq!(overlay, None);
}

#[test]
fn hud_debug_overlay_projects_custom_entity_render_stats_under_reduced_debug_info() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    world.apply_simulation_distance(bbb_protocol::packets::SetSimulationDistance { distance: 12 });
    world.apply_add_entity(test_add_entity(7, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));
    world.apply_add_entity(test_add_entity(8, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::EntityRenderStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("entity render stats should be allowed under reduced debug info");

    assert_eq!(overlay.left_lines, vec!["E: 2/2, SD: 12".to_string()]);
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_particle_render_stats() {
    let world = world_with_dimension(0, "minecraft:overworld");
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ParticleRenderStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let renderer_counters = bbb_renderer::RendererCounters {
        active_particle_instances: 23,
        ..Default::default()
    };

    let overlay = hud_debug_overlay_at_partial_tick(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        1.0,
        None,
        &TerrainTextureState::default(),
        &renderer_counters,
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
        &AudioCounters::default(),
        None,
    )
    .expect("particle render stats should show outside reduced debug info");

    assert_eq!(overlay.left_lines, vec!["P: 23".to_string()]);
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_chunk_source_stats_under_reduced_debug_info() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    world.apply_set_chunk_cache_radius(bbb_protocol::packets::SetChunkCacheRadius { radius: 2 });
    let mut first_chunk = empty_lightmap_test_chunk(world.dimension());
    first_chunk.pos = ChunkPos { x: 0, z: 0 };
    world.insert_decoded_chunk(first_chunk);
    let mut second_chunk = empty_lightmap_test_chunk(world.dimension());
    second_chunk.pos = ChunkPos { x: 1, z: 0 };
    world.insert_decoded_chunk(second_chunk);
    world.apply_add_entity(test_add_entity(7, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkSourceStats,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("chunk source stats should be allowed under reduced debug info");

    assert_eq!(
        overlay.left_lines,
        vec!["Chunks[C] W: 121, 2 E: 1,0,2".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_sound_cache_under_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::SoundCache,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let audio_counters = AudioCounters {
        sound_cache_buffers: 7,
        sound_cache_bytes: 1_048_577,
        ..AudioCounters::default()
    };

    assert_eq!(hud_debug_sound_cache_bytes_to_mebibytes(0), 0);
    assert_eq!(hud_debug_sound_cache_bytes_to_mebibytes(1), 1);
    assert_eq!(hud_debug_sound_cache_bytes_to_mebibytes(2_097_152), 2);

    let overlay = hud_debug_overlay_at_partial_tick(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        1.0,
        None,
        &TerrainTextureState::default(),
        &bbb_renderer::RendererCounters::default(),
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
        &audio_counters,
        None,
    )
    .expect("sound cache should be allowed under reduced debug info");

    assert_eq!(
        overlay.left_lines,
        vec!["Sound cache: 7 buffers, 2 MiB".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_custom_sound_mood() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::SoundMood,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let audio_counters = AudioCounters {
        sound_static_channels_used: 3,
        sound_static_channels_capacity: 32,
        sound_streaming_channels_used: 1,
        sound_streaming_channels_capacity: 8,
        sound_mood_percent: 42,
        ..AudioCounters::default()
    };

    let overlay = hud_debug_overlay_at_partial_tick(
        &input,
        &world,
        Some(CameraPose {
            position: [0.5, 0.0, -2.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        1.0,
        None,
        &TerrainTextureState::default(),
        &bbb_renderer::RendererCounters::default(),
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
        &audio_counters,
        None,
    )
    .expect("custom sound mood entry should show while always-on");

    assert_eq!(
        overlay.left_lines,
        vec!["Sounds: 3/32 + 1/8 (Mood 42%)".to_string()]
    );
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_suppresses_post_effect_without_current_effect() {
    let world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::PostEffect,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    );

    assert!(
        overlay.is_none(),
        "post-effect entry should not render until a current post effect exists"
    );
}

#[test]
fn hud_debug_post_effect_tracks_vanilla_camera_entity_families() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    for (entity_id, entity_type_id, expected) in [
        (100, VANILLA_ENTITY_TYPE_CREEPER_ID, "minecraft:creeper"),
        (101, VANILLA_ENTITY_TYPE_SPIDER_ID, "minecraft:spider"),
        (102, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, "minecraft:spider"),
        (103, VANILLA_ENTITY_TYPE_ENDERMAN_ID, "minecraft:invert"),
    ] {
        world.apply_add_entity(test_add_entity(entity_id, entity_type_id));
        assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera {
            camera_id: entity_id,
        }));
        assert_eq!(current_post_effect_id_from_world(&world), Some(expected));
        assert_eq!(
            hud_debug_post_effect_line(Some(expected)).as_deref(),
            Some(format!("Post: {expected}").as_str())
        );
    }

    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::PostEffect,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("current enderman camera post effect should render");
    assert_eq!(overlay.left_lines, vec!["Post: minecraft:invert"]);

    world.apply_add_entity(test_add_entity(104, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 104 }));
    assert_eq!(current_post_effect_id_from_world(&world), None);
}

#[test]
fn hud_debug_post_effect_is_filtered_by_reduced_debug_info() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_CREEPER_ID));
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::PostEffect,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    assert!(hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .is_none());
}

#[test]
fn hud_debug_overlay_filters_default_entries_in_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(60);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let overlay = hud_debug_overlay(
        &input,
        &world,
        Some(CameraPose {
            position: [10.25, 64.0, -5.75],
            y_rot: 90.0,
            x_rot: 15.0,
            eye_height: 1.62,
        }),
        winit::dpi::PhysicalSize::new(320, 240),
        &fps_sampler,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("reduced-debug still projects allowed default entries");

    assert!(overlay
        .left_lines
        .contains(&format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-native)")));
    assert!(overlay
        .left_lines
        .contains(&"60 fps T: inf vsync".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Section-relative: 10 00 10".to_string()));
    assert!(!overlay
        .left_lines
        .iter()
        .any(|line| line.starts_with("XYZ: ")));
    assert!(!overlay
        .left_lines
        .iter()
        .any(|line| line.starts_with("Block: ")));
    assert!(!overlay
        .left_lines
        .iter()
        .any(|line| line.starts_with("Facing: ")));
    assert_eq!(overlay.debug_crosshair, None);
}

#[test]
fn hud_debug_overlay_projects_custom_detailed_memory_under_reduced_debug_info() {
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::DetailedMemory,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("detailed memory is allowed under reduced-debug info");

    assert_eq!(overlay.left_lines, Vec::<String>::new());
    assert_eq!(overlay.right_lines.len(), 2);
    assert!(overlay.right_lines[0].starts_with("Memory (heap): "));
    assert!(overlay.right_lines[1].starts_with("Memory (non-heap): "));
}

#[test]
fn hud_debug_fps_sampler_reports_completed_one_second_windows() {
    let start = Instant::now();
    let mut sampler = HudDebugFpsSampler::default();

    sampler.record_frame(start);
    assert!(sampler.frame_time_nanos().is_empty());
    sampler.record_frame(start + Duration::from_millis(500));
    assert_eq!(sampler.fps(), 0);
    assert_eq!(sampler.frame_time_nanos(), vec![500_000_000]);

    sampler.record_frame(start + Duration::from_secs(1));
    assert_eq!(sampler.fps(), 3);
    assert_eq!(sampler.frame_time_nanos(), vec![500_000_000, 500_000_000]);

    sampler.record_frame(start + Duration::from_millis(1200));
    assert_eq!(sampler.fps(), 3);

    sampler.record_frame(start + Duration::from_secs(2));
    assert_eq!(sampler.fps(), 2);
}

#[test]
fn hud_debug_fps_sampler_keeps_vanilla_sample_capacity() {
    let start = Instant::now();
    let mut sampler = HudDebugFpsSampler::default();
    for frame in 0..=HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY + 2 {
        sampler.record_frame(start + Duration::from_millis(frame as u64));
    }

    let samples = sampler.frame_time_nanos();
    assert_eq!(samples.len(), HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY);
    assert_eq!(samples[0], 1_000_000);
    assert_eq!(samples[HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY - 1], 1_000_000);
}

#[test]
fn hud_debug_network_sampler_logs_bandwidth_and_debug_ping_ticks() {
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
        None,
        None
    ));
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(4);
    let net_commands = Some(tx);
    let mut sampler = HudDebugNetworkSampler::default();

    sampler.record_received_packet(12);
    sampler.record_received_packet(8);
    sampler.advance_tick(&input, &net_counters, &net_commands, start, 1_000);
    assert_eq!(sampler.bandwidth_bytes_per_tick(), vec![20]);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::PingRequest(1_000));

    sampler.record_pong_response(940, 1_000);
    assert_eq!(sampler.ping_millis(), vec![60]);

    sampler.record_received_packet(5);
    sampler.advance_tick(
        &input,
        &net_counters,
        &net_commands,
        start + Duration::from_millis(49),
        1_049,
    );
    assert_eq!(sampler.bandwidth_bytes_per_tick(), vec![20]);
    assert!(rx.try_recv().is_err());

    sampler.advance_tick(
        &input,
        &net_counters,
        &net_commands,
        start + Duration::from_millis(50),
        1_050,
    );
    assert_eq!(sampler.bandwidth_bytes_per_tick(), vec![20, 5]);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::PingRequest(1_050));
}

#[test]
fn hud_debug_tps_sampler_records_tick_time_samples_and_syncs_subscription() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(4);
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: vec![0]
        }
    );
    assert_eq!(
        world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0]
    );
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(rx.try_recv().is_err());

    sampler.record_net_event(&NetEvent::Play(
        bbb_protocol::packets::PlayClientbound::DebugSample(bbb_protocol::packets::DebugSample {
            sample: vec![70_000_000, 30_000_000, 10_000_000, 5_000_000],
            sample_type: bbb_protocol::packets::RemoteDebugSampleType::TickTime,
        }),
    ));
    assert_eq!(
        sampler.samples(),
        vec![HudDebugTpsSample {
            full_tick_nanos: 70_000_000,
            tick_server_method_nanos: 30_000_000,
            scheduled_tasks_nanos: 10_000_000,
            idle_nanos: 5_000_000,
        }]
    );

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
        None,
        None
    ));
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: Vec::new()
        }
    );
    assert!(world.requested_debug_subscriptions().is_empty());
}

#[test]
fn remote_debug_renderer_requests_share_one_world_owned_subscription_set() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    input.set_remote_debug_renderers(
        [
            crate::remote_debug::RemoteDebugRenderer::Bees,
            crate::remote_debug::RemoteDebugRenderer::GameEventListeners,
        ]
        .into_iter()
        .collect(),
    );
    let mut net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(4);
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    let expected = vec![1, 4, 7, 13, 15];
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: expected.clone(),
        }
    );
    assert_eq!(
        world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        expected
    );

    net_counters.connected = false;
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(world.requested_debug_subscriptions().is_empty());
    assert!(world.client_debug_game().values_by_subscription.is_empty());
}

#[test]
fn remote_debug_subscription_retries_after_backpressure_and_listener_clear() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    input.set_remote_debug_renderers(
        [crate::remote_debug::RemoteDebugRenderer::Bees]
            .into_iter()
            .collect(),
    );
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetCommand::DebugSubscriptionRequest {
        subscription_ids: Vec::new(),
    })
    .unwrap();
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(world.requested_debug_subscriptions().is_empty());
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest { subscription_ids } if subscription_ids.is_empty()
    ));

    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    let expected = vec![1, 4, 7];
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: expected.clone(),
        }
    );
    assert_eq!(
        world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        expected
    );

    world.clear_debug_subscriber();
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::DebugSubscriptionRequest {
            subscription_ids: vec![1, 4, 7],
        }
    );
}

#[test]
fn remote_debug_subscription_purges_at_current_pre_increment_game_time() {
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    input.set_remote_debug_renderers(
        [crate::remote_debug::RemoteDebugRenderer::NeighborUpdates]
            .into_iter()
            .collect(),
    );
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let (tx, mut rx) = mpsc::channel(2);
    let net_commands = Some(tx);
    let mut sampler = HudDebugTpsSampler::default();
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    let _ = rx.try_recv().unwrap();
    world.apply_debug_event(bbb_protocol::packets::DebugEvent {
        event: bbb_protocol::packets::DebugSubscriptionEvent {
            subscription_id: 14,
            value: bbb_protocol::packets::DebugSubscriptionValue::NeighborUpdate(
                ProtocolBlockPos { x: 1, y: 2, z: 3 },
            ),
        },
    });
    assert_eq!(world.debug_subscription_values(14).unwrap().events.len(), 1);

    world.apply_world_time(PlayTime {
        game_time: 200,
        clock_updates: Vec::new(),
    });
    sampler.sync_subscriptions_and_tick(&input, &mut world, &net_counters, &net_commands);
    assert!(world
        .debug_subscription_values(14)
        .unwrap()
        .events
        .is_empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn hud_debug_overlay_projects_network_charts_for_connected_f3_3() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let net_counters = NetCounters {
        connected: true,
        state: Some("Play".to_string()),
        ..NetCounters::default()
    };
    let mut network_sampler = HudDebugNetworkSampler::default();
    network_sampler.record_pong_response(900, 1_000);
    network_sampler.record_received_packet(42);
    network_sampler.advance_tick(&input, &net_counters, &None, Instant::now(), 1_000);

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &network_sampler,
        &HudDebugTpsSampler::default(),
        &net_counters,
    )
    .expect("F3+3 should force the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"[F3+3] Network visible; [F3+4] Lightmap hidden".to_string()));
    assert_eq!(overlay.fps_chart, None);
    assert_eq!(
        overlay.network_charts,
        Some(HudDebugNetworkCharts {
            ping_millis: vec![100],
            bandwidth_bytes_per_tick: vec![42],
            show_bandwidth: true,
        })
    );
}

#[test]
fn hud_debug_overlay_projects_tps_chart_for_f3_2_remote_samples() {
    let mut world = WorldStore::new();
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let fps_sampler = hud_debug_fps_sampler_with_frame_times(&[16_000_000]);
    let mut tps_sampler = HudDebugTpsSampler::default();
    tps_sampler.record_debug_sample(&[50_000_000, 20_000_000, 5_000_000, 10_000_000]);

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &fps_sampler,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &tps_sampler,
        &NetCounters::default(),
    )
    .expect("F3+2 should force the debug overlay visible");

    assert_eq!(
        overlay.fps_chart,
        Some(HudDebugFrameTimeChart {
            frame_time_nanos: vec![16_000_000],
            configured_framerate_limit: None,
        })
    );
    assert_eq!(
        overlay.tps_chart,
        Some(HudDebugTpsChart {
            samples: vec![HudDebugTpsSample {
                full_tick_nanos: 50_000_000,
                tick_server_method_nanos: 20_000_000,
                scheduled_tasks_nanos: 5_000_000,
                idle_nanos: 10_000_000,
            }],
            milliseconds_per_tick: 100.0,
        })
    );
    assert_eq!(overlay.network_charts, None);
}

#[test]
fn hud_debug_fps_line_matches_vanilla_shape_without_configured_cap() {
    assert_eq!(
        hud_debug_fps_line(144, VANILLA_UNLIMITED_FRAMERATE_LIMIT, true),
        "144 fps T: inf vsync"
    );
    assert_eq!(hud_debug_fps_line(60, 120, false), "60 fps T: 120");
    assert_eq!(hud_debug_configured_framerate_limit(120), Some(120));
    assert_eq!(
        hud_debug_configured_framerate_limit(VANILLA_UNLIMITED_FRAMERATE_LIMIT),
        None
    );
}

#[test]
fn hud_debug_overlay_projects_configured_framerate_limit_into_fps_chart() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    let fps_sampler = hud_debug_fps_sampler_with_reported_fps(60);

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &fps_sampler,
        120,
        false,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("F3+2 should force the debug overlay visible");

    assert_eq!(overlay.left_lines[1], "60 fps T: 120");
    assert_eq!(
        overlay.fps_chart,
        Some(HudDebugFrameTimeChart {
            frame_time_nanos: fps_sampler.frame_time_nanos(),
            configured_framerate_limit: Some(120),
        })
    );
}

#[test]
fn hud_debug_overlay_formats_memory_lines_like_vanilla_debug_entries() {
    assert_eq!(
        hud_debug_memory_lines(HudDebugMemorySnapshot {
            used_mib: 32,
            max_mib: 256,
            allocated_mib: 128,
            allocation_rate_mib_per_s: 7,
        }),
        vec![
            "Mem: 12% 032/256MiB".to_string(),
            "Allocation rate: 007MiB/s".to_string(),
            "Allocated: 50% 128MiB".to_string(),
        ]
    );
}

#[test]
fn hud_debug_overlay_formats_detailed_memory_lines_like_vanilla_debug_entry() {
    assert_eq!(
        hud_debug_detailed_memory_lines(HudDebugDetailedMemorySnapshot {
            heap: HudDebugMemoryUsageSnapshot {
                init_mib: 1,
                used_mib: 32,
                committed_mib: 128,
                max_mib: 256,
            },
            non_heap: HudDebugMemoryUsageSnapshot {
                init_mib: 0,
                used_mib: 8,
                committed_mib: 16,
                max_mib: 0,
            },
        }),
        vec![
            "Memory (heap): i=001MiB u=032MiB c=128MiB m=256MiB".to_string(),
            "Memory (non-heap): i=000MiB u=008MiB c=016MiB m=000MiB".to_string(),
        ]
    );
}

#[test]
fn hud_debug_overlay_projects_tps_server_brand_and_freeze_status() {
    assert_eq!(hud_debug_tps_line(&WorldStore::new()), None);

    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_custom_payload(bbb_protocol::packets::CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: bbb_protocol::packets::CustomPayloadBody::Brand {
            brand: "vanilla".to_string(),
        },
    });
    assert_eq!(
        hud_debug_tps_line(&world),
        Some("\"vanilla\" server, 0 tx, 0 rx".to_string())
    );

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });
    assert_eq!(
        hud_debug_tps_line(&world),
        Some("\"vanilla\" server (frozen), 0 tx, 0 rx".to_string())
    );

    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 2 });
    let expected = "\"vanilla\" server (frozen - stepping), 0 tx, 0 rx".to_string();
    assert_eq!(hud_debug_tps_line(&world), Some(expected.clone()));

    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("F3 should show the debug overlay");

    assert!(overlay.left_lines.contains(&expected));
}

#[test]
fn hud_debug_overlay_help_lines_reflect_chart_toggle_state() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let fps_sampler = hud_debug_fps_sampler_with_frame_times(&[16_000_000, 33_000_000]);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &fps_sampler,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("chart toggle should force the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"Debug charts: [F3+1] Profiler hidden; [F3+2] FPS visible;".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"[F3+3] Network hidden; [F3+4] Lightmap hidden".to_string()));
    assert_eq!(
        overlay.fps_chart,
        Some(HudDebugFrameTimeChart {
            frame_time_nanos: fps_sampler.frame_time_nanos(),
            configured_framerate_limit: None,
        })
    );
    assert!(!overlay.show_lightmap_preview);
}

#[test]
fn hud_debug_overlay_projects_profiler_toggle_without_fake_chart_data() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit1),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("F3+1 should force the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"Debug charts: [F3+1] Profiler visible; [F3+2] FPS hidden;".to_string()));
    assert_eq!(overlay.profiler_chart, None);
    assert_eq!(overlay.fps_chart, None);
    assert_eq!(overlay.network_charts, None);
}

#[test]
fn hud_debug_overlay_merges_native_profiler_chart() {
    let chart = HudDebugProfilerChart {
        current_node_name: "root".to_string(),
        current_global_percentage: 100.0,
        slices: vec![bbb_renderer::HudDebugProfilerSlice {
            name: "render".to_string(),
            percentage: 75.0,
            global_percentage: 75.0,
        }],
    };

    let chart_only =
        hud_debug_overlay_with_profiler_chart(None, Some(chart.clone())).expect("chart overlay");
    assert_eq!(chart_only.profiler_chart, Some(chart.clone()));
    assert!(chart_only.left_lines.is_empty());

    let existing = HudDebugOverlay {
        left_lines: vec!["Minecraft 26.1".to_string()],
        ..HudDebugOverlay::default()
    };
    let merged = hud_debug_overlay_with_profiler_chart(Some(existing), Some(chart.clone()))
        .expect("merged overlay");
    assert_eq!(merged.left_lines, vec!["Minecraft 26.1".to_string()]);
    assert_eq!(merged.profiler_chart, Some(chart));
}

#[test]
fn hud_debug_overlay_projects_game_mode_switcher_state() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let player_id = world
        .local_player_id()
        .expect("test world has a local player");
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: player_id,
        event_id: 26,
    }));
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("game mode switcher should project even while the F3 overlay is hidden");
    let switcher = overlay
        .game_mode_switcher
        .expect("game mode switcher render state should be present");

    assert_eq!(
        switcher.selected,
        bbb_renderer::HudGameModeSwitcherMode::Creative
    );
    assert_eq!(switcher.title, "Creative Mode");
    assert_eq!(switcher.help_text, "Select next: F4");
    assert_eq!((switcher.background_x, switcher.background_y), (98, 62));
    assert_eq!(
        switcher.slots.iter().map(|slot| slot.x).collect::<Vec<_>>(),
        vec![101, 132, 163, 194]
    );
    assert!(switcher.slots[0].selected);
    assert!(switcher.slots[1..].iter().all(|slot| !slot.selected));
    assert!(overlay.left_lines.is_empty());
    assert!(overlay.right_lines.is_empty());
}

#[test]
fn hud_debug_overlay_projects_game_mode_switcher_item_icons() {
    let root = unique_runtime_temp_dir("game-mode-switcher-icons");
    write_runtime_game_mode_switcher_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let player_id = world
        .local_player_id()
        .expect("test world has a local player");
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: player_id,
        event_id: 26,
    }));
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay_at_partial_tick(
        &input,
        &world,
        None,
        1.0,
        Some(&item_runtime),
        &terrain_textures,
        &bbb_renderer::RendererCounters::default(),
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
        &AudioCounters::default(),
        None,
    )
    .expect("game mode switcher should project with runtime icons");
    let switcher = overlay.game_mode_switcher.expect("switcher render state");

    assert_eq!(
        DEBUG_GAME_MODE_SWITCHER_MODES
            .iter()
            .copied()
            .map(hud_game_mode_switcher_icon_resource_id)
            .collect::<Vec<_>>(),
        vec![
            "minecraft:grass_block",
            "minecraft:iron_sword",
            "minecraft:map",
            "minecraft:ender_eye",
        ]
    );
    for (slot, mode) in switcher.slots.iter().zip(DEBUG_GAME_MODE_SWITCHER_MODES) {
        let resource_id = hud_game_mode_switcher_icon_resource_id(mode);
        let item_id = item_runtime
            .item_protocol_id(resource_id)
            .expect("test item is registered");
        let stack = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        };
        let expected_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
        let icon = slot.icon.as_ref().expect("slot has item icon");

        assert_eq!(slot.mode, mode);
        assert_eq!(
            icon.layers[0].uv,
            HudUvRect {
                min: expected_uv.min,
                max: expected_uv.max,
            }
        );
        assert_eq!(icon.count_label, None);
        assert_eq!(icon.durability_bar, None);
        assert_eq!(icon.cooldown_progress, None);
        assert!(slot.block_model.is_none());
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_debug_overlay_projects_lightmap_preview_toggle_state() {
    let world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Digit4),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("lightmap toggle should force the debug overlay visible");

    assert!(overlay.show_lightmap_preview);
    assert!(overlay
        .left_lines
        .contains(&"[F3+3] Network hidden; [F3+4] Lightmap visible".to_string()));
}

fn hud_debug_fps_sampler_with_reported_fps(fps: u32) -> HudDebugFpsSampler {
    let start = Instant::now();
    let mut sampler = HudDebugFpsSampler::default();
    let frames = fps.max(1);
    for frame in 0..frames {
        let nanos = if frames == 1 {
            1_000_000_000
        } else {
            u64::from(frame) * 1_000_000_000 / u64::from(frames - 1)
        };
        sampler.record_frame(start + Duration::from_nanos(nanos));
    }
    sampler
}

fn hud_debug_fps_sampler_with_frame_times(frame_times: &[u64]) -> HudDebugFpsSampler {
    let mut sampler = HudDebugFpsSampler::default();
    let mut now = Instant::now();
    sampler.record_frame(now);
    for frame_time in frame_times {
        now += Duration::from_nanos(*frame_time);
        sampler.record_frame(now);
    }
    sampler
}

#[test]
fn f3_a_requests_terrain_reload_without_toggling_overlay_on_release() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut terrain_upload = TerrainUploadState::default();
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut world),
        Some(&mut terrain_upload)
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));

    assert!(terrain_upload.reload_all_chunks_requested());
    assert_eq!(world.client_chat().messages.len(), 1);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Reloading all chunks"
    );
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_p_toggles_pause_on_lost_focus_without_world() {
    let mut input = ClientInputState::new(true);
    assert!(input.debug_pause_on_lost_focus());

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyP),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));

    assert!(!input.debug_pause_on_lost_focus());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_p_emits_pause_on_lost_focus_debug_feedback_with_world() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyP),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyP),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));

    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: Pause on lost focus: disabled",
            "[Debug]: Pause on lost focus: enabled",
        ]
    );
    assert!(input.debug_pause_on_lost_focus());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn hud_debug_overlay_help_lines_reflect_status_toggle_state() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    let mut input = ClientInputState::new(true);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    for code in [KeyCode::KeyB, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyP] {
        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(code),
            ElementState::Pressed,
            Some(&mut world),
            None
        ));
    }
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    assert!(!input.debug_overlay_visible());
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));

    let overlay = hud_debug_overlay(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(320, 240),
        &HudDebugFpsSampler::default(),
        VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        true,
        &HudDebugNetworkSampler::default(),
        &HudDebugTpsSampler::default(),
        &NetCounters::default(),
    )
    .expect("plain F3 should make the debug overlay visible");

    assert!(overlay
        .left_lines
        .contains(&"Debug toggles: [F3+B] Hitboxes visible; [F3+G] Chunks visible".to_string()));
    assert!(overlay.left_lines.contains(
        &"Debug options: [F3+H] Tooltips enabled; [F3+P] Focus pause disabled".to_string()
    ));
    assert!(overlay
        .left_lines
        .contains(&"Debug actions: [F3+A] Reload chunks; [F3+C] Copy location".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Debug actions: [F3+D] Clear chat; [F3+S] Dump textures".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Debug actions: [F3+T] Reload packs; [F3+V] Version".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"Game mode: [F3+N] Spectator; [F3+F4] Switcher".to_string()));
    assert!(overlay
        .left_lines
        .contains(&"To edit: press [F3+F6]".to_string()));
}

#[test]
fn debug_entity_scene_outline_follows_f3_b_hitbox_toggle() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(77, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let mut input = ClientInputState::new(true);

    assert_eq!(debug_entity_scene_outline(&input, &world, 1.0), None);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let outline = debug_entity_scene_outline(&input, &world, 1.0)
        .expect("F3+B should enable entity AABB debug outlines");
    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 2);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.points.len(), 1);
    assert!(outline.text_labels.is_empty());
}

#[test]
fn debug_entity_scene_outline_includes_missing_server_label_when_startup_flag_is_enabled() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(77, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let mut input = ClientInputState::new(true);
    input.set_debug_show_local_server_entity_hit_boxes(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let outline = debug_entity_scene_outline(&input, &world, 1.0)
        .expect("F3+B should enable entity AABB debug outlines");
    assert_eq!(outline.text_labels.len(), 1);
    assert_eq!(outline.text_labels[0].text, "Missing Server Entity");
    assert_eq!(
        debug_selection_text_billboards(Some(&outline)),
        [DebugTextBillboard::centered(
            outline.text_labels[0].position,
            "Missing Server Entity",
            [255, 0, 0, 255],
        )]
    );
}

#[test]
fn debug_chunk_border_outline_follows_f3_g_chunk_border_toggle() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut input = ClientInputState::new(true);
    let camera_pose = Some(CameraPose {
        position: [31.8, 42.0, -0.2],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    assert_eq!(
        debug_chunk_border_outline(&input, &world, camera_pose),
        None
    );

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyG),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));

    let outline = debug_chunk_border_outline(&input, &world, camera_pose)
        .expect("F3+G should enable chunk-border debug outlines");
    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 1);
    assert_eq!(outline.colored_boxes[0].min, [16.0, 32.0, -16.0]);
    assert_eq!(outline.colored_boxes[0].max, [32.0, 48.0, 0.0]);
    assert_eq!(outline.colored_boxes[0].color, CHUNK_BORDER_MAJOR_COLOR);
    assert_eq!(
        outline.colored_boxes[0].line_width,
        CHUNK_BORDER_THIN_LINE_WIDTH
    );
    assert!(outline.colored_boxes[0].always_on_top);
    assert_eq!(outline.lines.len(), 920);
    assert_eq!(outline.lines[0].from, [0.0, -64.0, -32.0]);
    assert_eq!(outline.lines[0].to, [0.0, 320.0, -32.0]);
    assert_eq!(outline.lines[0].color, CHUNK_BORDER_NEIGHBOR_COLOR);
    assert_eq!(outline.lines[0].width, CHUNK_BORDER_THICK_LINE_WIDTH);
    assert_eq!(outline.lines[16].from, [18.0, -64.0, -16.0]);
    assert_eq!(outline.lines[16].color, CHUNK_BORDER_YELLOW_COLOR);
    assert_eq!(outline.lines[16].width, CHUNK_BORDER_THIN_LINE_WIDTH);
    assert_eq!(outline.lines[18].from, [20.0, -64.0, -16.0]);
    assert_eq!(outline.lines[18].color, CHUNK_BORDER_CELL_COLOR);
    assert_eq!(outline.lines[44].from, [16.0, -64.0, -16.0]);
    assert_eq!(outline.lines[44].to, [16.0, -64.0, 0.0]);
    assert_eq!(outline.lines[44].color, CHUNK_BORDER_CELL_COLOR);
    assert_eq!(outline.lines[816].from, [16.0, -64.0, -16.0]);
    assert_eq!(outline.lines[816].to, [16.0, 320.0, -16.0]);
    assert_eq!(outline.lines[816].color, CHUNK_BORDER_MAJOR_COLOR);
    assert_eq!(outline.lines[816].width, CHUNK_BORDER_THICK_LINE_WIDTH);
    assert_eq!(outline.lines[920 - 1].from, [32.0, 320.0, -16.0]);
    assert_eq!(outline.lines[920 - 1].to, [16.0, 320.0, -16.0]);
    assert_eq!(outline.lines[920 - 1].color, CHUNK_BORDER_MAJOR_COLOR);
    assert!(outline.points.is_empty());
    assert_eq!(debug_chunk_border_outline(&input, &world, None), None);
}

#[test]
fn debug_heightmap_boxes_follow_custom_entry_and_generated_descriptor_order() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let min_y = world.dimension().min_y as f32;
    let mut chunk = empty_lightmap_test_chunk(world.dimension());
    chunk.pos = ChunkPos { x: -1, z: -1 };
    chunk.heightmaps = VANILLA_HEIGHTMAP_TYPES
        .iter()
        .map(|entry| test_heightmap(entry.id, world.dimension(), &[]))
        .collect();
    world.insert_decoded_chunk(chunk);
    let camera_pose = Some(CameraPose {
        position: [-0.2, 42.0, -0.2],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let mut input = ClientInputState::new(true);

    assert!(debug_heightmap_boxes(&input, &world, camera_pose).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeHeightmap,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let boxes = debug_heightmap_boxes(&input, &world, camera_pose);
    assert_eq!(boxes.len(), VANILLA_HEIGHTMAP_TYPES.len() * 16 * 16);
    for (entry_index, entry) in VANILLA_HEIGHTMAP_TYPES.iter().enumerate() {
        let first = &boxes[entry_index * 16 * 16];
        let y = min_y + entry.id as f32 * DEBUG_HEIGHTMAP_BOX_HEIGHT;
        assert_eq!(first.min, [-15.75, y, -15.75]);
        assert_eq!(first.max, [-15.25, y + DEBUG_HEIGHTMAP_BOX_HEIGHT, -15.25]);
        assert_eq!(first.color, entry.debug_color_rgba);
    }
    assert_eq!(boxes[1].min[2], -14.75);
    assert_eq!(boxes[16].min[0], -14.75);
}

#[test]
fn debug_heightmap_boxes_respect_reduced_debug_and_camera_gates() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeHeightmap,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    assert!(debug_heightmap_boxes(
        &input,
        &world,
        Some(CameraPose {
            position: [0.0, 42.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        })
    )
    .is_empty());
    assert!(debug_heightmap_boxes(&input, &world, None).is_empty());
}

#[test]
fn debug_water_surfaces_follow_player_scan_order_and_vanilla_fluid_height() {
    assert_eq!(DEBUG_WATER_SCAN_RADIUS, 10);
    assert_eq!(DEBUG_WATER_BOX_INSET, 0.01);
    assert_eq!(DEBUG_WATER_BOX_COLOR, [0, 255, 0, 38]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 0, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let flowing_water_level_3 = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:water",
            &BTreeMap::from([("level".to_string(), "3".to_string())]),
        )
        .expect("vanilla level-3 flowing water state");
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 1, y: 0, z: 0 },
        flowing_water_level_3,
    );
    let lava = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:lava",
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .expect("vanilla source lava state");
    set_lightmap_test_block(&mut world, BlockPos { x: 2, y: 0, z: 0 }, lava);

    let mut input = ClientInputState::new(true);
    assert!(debug_water_samples(&input, &world).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeWaterLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let samples = debug_water_samples(&input, &world);
    let boxes = debug_water_boxes(&samples);
    assert_eq!(boxes.len(), 3, "lava is not in FluidTags.WATER");
    assert_eq!(
        boxes[0],
        DebugFilledBox {
            min: [0.01, 0.01, 0.01],
            max: [0.99, 1.0, 0.99],
            color: DEBUG_WATER_BOX_COLOR,
        },
        "same water above makes the lower source column full height"
    );
    assert_eq!(boxes[1].min, [1.01, 0.01, 0.01]);
    assert!((boxes[1].max[1] - 5.0 / 9.0).abs() < 1e-6);
    assert_eq!(boxes[2].min, [0.01, 1.01, 0.01]);
    assert!((boxes[2].max[1] - (1.0 + 8.0 / 9.0)).abs() < 1e-6);

    let labels = debug_water_amount_billboards(&samples);
    assert_eq!(labels.len(), 3);
    assert_eq!(
        labels[0],
        DebugTextBillboard::centered([0.5, 1.0, 0.5], "8", [0, 0, 0, 255])
    );
    assert_eq!(labels[1].text, "5");
    assert_eq!(labels[1].position[0], 1.5);
    assert!((labels[1].position[1] - 5.0 / 9.0).abs() < 1e-6);
    assert_eq!(labels[2].text, "8");
    assert!((labels[2].position[1] - (1.0 + 8.0 / 9.0)).abs() < 1e-6);
    assert!(labels.iter().all(|label| {
        label.scale == DebugTextBillboard::DEFAULT_SCALE && label.adjust_left.is_none()
    }));
}

#[test]
fn debug_water_boxes_respect_reduced_debug_and_player_gates() {
    let mut world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 0, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeWaterLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    assert!(debug_water_samples(&input, &world).is_empty());

    let mut no_player = world_with_dimension(0, "minecraft:overworld");
    no_player.insert_decoded_chunk(empty_lightmap_test_chunk(no_player.dimension()));
    assert!(debug_water_samples(&input, &no_player).is_empty());
}

#[test]
fn debug_light_billboards_match_vanilla_scan_filters_positions_and_colors() {
    assert_eq!(DEBUG_LIGHT_SCAN_RADIUS, 10);
    assert_eq!(DEBUG_LIGHT_SECTION_LABEL_SCALE, 4.8);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.25, 0.0, 0.25], 0.0, 0.0));
    let mut chunk = empty_lightmap_test_chunk(world.dimension());
    let mut sky = vec![0xff; TEST_LIGHT_ARRAY_BYTES];
    let mut block = vec![0; TEST_LIGHT_ARRAY_BYTES];
    let nibble_index = section_block_index(0, 1, 0);
    set_test_light_nibble(&mut sky, nibble_index, 4);
    set_test_light_nibble(&mut block, nibble_index, 13);
    let light_section_index = 0 - (world.dimension().min_section_y() - 1);
    let light_mask = single_bit_mask(usize::try_from(light_section_index).unwrap());
    chunk.light = LightData {
        sky_y_mask: light_mask.clone(),
        block_y_mask: light_mask,
        empty_sky_y_mask: Vec::new(),
        empty_block_y_mask: Vec::new(),
        sky_updates: vec![sky],
        block_updates: vec![block],
    };
    world.insert_decoded_chunk(chunk);
    let camera_pose = camera_pose_from_world(&world);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeBlockLightLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let block_only = debug_light_level_billboards(&input, &world, camera_pose);
    let section_labels = block_only
        .iter()
        .filter(|billboard| billboard.scale == DEBUG_LIGHT_SECTION_LABEL_SCALE)
        .collect::<Vec<_>>();
    assert_eq!(section_labels.len(), 8);
    assert!(section_labels.iter().all(|billboard| {
        billboard.text == "2" && billboard.color == DEBUG_LIGHT_SECTION_LABEL_COLOR
    }));
    assert_eq!(section_labels[0].position, [-8.0, -8.0, -8.0]);
    let block_label = block_only
        .iter()
        .find(|billboard| billboard.text == "13")
        .expect("non-zero block light should be labeled");
    assert_eq!(block_label.position, [0.5, 1.5, 0.5]);
    assert_eq!(block_label.color, [243, 221, 0, 255]);
    assert_eq!(block_label.scale, DebugTextBillboard::DEFAULT_SCALE);
    assert!(!block_only.iter().any(|billboard| billboard.text == "4"));

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightLevels,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let both = debug_light_level_billboards(&input, &world, camera_pose);
    assert_eq!(both.len(), 10);
    let sky_label = both
        .iter()
        .find(|billboard| billboard.text == "4")
        .expect("non-full sky light should be labeled");
    assert_eq!(sky_label.position, [0.5, 1.25, 0.5]);
    assert_eq!(sky_label.color, [0, 68, 255, 255]);
    let sky_index = both
        .iter()
        .position(|billboard| billboard.text == "4")
        .unwrap();
    let block_index = both
        .iter()
        .position(|billboard| billboard.text == "13")
        .unwrap();
    assert_eq!(block_index, sky_index + 1);

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    reduced.set_local_player_pose(local_player_pose([0.25, 0.0, 0.25], 0.0, 0.0));
    assert!(
        debug_light_level_billboards(&input, &reduced, camera_pose_from_world(&reduced)).is_empty()
    );
}

#[test]
fn debug_solid_face_rects_match_vanilla_scan_faces_inflate_and_gates() {
    assert_eq!(DEBUG_SOLID_FACE_SCAN_RADIUS, 6);
    assert_eq!(DEBUG_SOLID_FACE_INFLATE, 0.002);
    assert_eq!(DEBUG_SOLID_FACE_COLOR, [255, 0, 0, 128]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);
    let camera_pose = camera_pose_from_world(&world);
    let mut input = ClientInputState::new(true);
    assert!(debug_solid_face_rects(&input, &world, camera_pose).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSolidFaces,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let rects = debug_solid_face_rects(&input, &world, camera_pose);
    assert_eq!(rects.len(), 6);
    assert_eq!(
        rects[0],
        DebugFilledRect {
            corners: [
                [-0.002, -0.002, -0.002],
                [-0.002, -0.002, 1.002],
                [-0.002, 1.002, 1.002],
                [-0.002, 1.002, -0.002],
            ],
            color: DEBUG_SOLID_FACE_COLOR,
        }
    );
    assert_eq!(
        rects[5].corners,
        [
            [-0.002, 1.002, -0.002],
            [-0.002, 1.002, 1.002],
            [1.002, 1.002, 1.002],
            [1.002, 1.002, -0.002],
        ]
    );
    assert!(rects
        .iter()
        .all(|rect| rect.color == DEBUG_SOLID_FACE_COLOR));

    let scaffolding = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:scaffolding",
            &BTreeMap::from([
                ("bottom".to_owned(), "false".to_owned()),
                ("distance".to_owned(), "0".to_owned()),
                ("waterlogged".to_owned(), "false".to_owned()),
            ]),
        )
        .expect("vanilla stable scaffolding state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, scaffolding);
    let scaffolding_rects = debug_solid_face_rects(&input, &world, camera_pose);
    assert_eq!(scaffolding_rects.len(), 7);
    assert!(scaffolding_rects.iter().all(|rect| {
        rect.color == DEBUG_SOLID_FACE_COLOR && rect.corners.iter().all(|corner| corner[1] > 0.99)
    }));

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    reduced.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    reduced.insert_decoded_chunk(empty_lightmap_test_chunk(reduced.dimension()));
    set_lightmap_test_block(&mut reduced, BlockPos { x: 0, y: 0, z: 0 }, stone);
    assert!(debug_solid_face_rects(&input, &reduced, camera_pose_from_world(&reduced)).is_empty());
    assert!(debug_solid_face_rects(&input, &world, None).is_empty());
}

#[test]
fn debug_sky_light_sections_match_vanilla_shapes_colors_and_refresh_cache() {
    assert_eq!(debug_light_sections::RADIUS, 10);
    assert_eq!(
        debug_light_sections::REFRESH_INTERVAL,
        Duration::from_millis(500)
    );
    assert_eq!(
        debug_light_sections::LIGHT_AND_BLOCKS_FACE_COLOR,
        [255, 255, 0, 63]
    );
    assert_eq!(debug_light_sections::LIGHT_ONLY_FACE_COLOR, [63, 31, 0, 31]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    let air = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:air", &BTreeMap::new())
        .expect("vanilla air state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let mut input = ClientInputState::new(true);
    let mut cache = DebugSkyLightSectionCache::default();
    let start = Instant::now();
    assert_eq!(
        cache.gizmos(&input, &world, start),
        (Vec::new(), Vec::new())
    );
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightSections,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let (rects, lines) = cache.gizmos(&input, &world, start);
    // One LIGHT_AND_DATA voxel contributes 6 faces/12 edges. Its 26-neighbour
    // LIGHT_ONLY shell makes the light shape a 3x3x3 cube: 54 boundary-cell
    // faces and 12 merged outer edges.
    assert_eq!(rects.len(), 60);
    assert_eq!(lines.len(), 24);
    assert!(rects[..6]
        .iter()
        .all(|rect| rect.color == debug_light_sections::LIGHT_AND_BLOCKS_FACE_COLOR));
    assert!(rects[6..]
        .iter()
        .all(|rect| rect.color == debug_light_sections::LIGHT_ONLY_FACE_COLOR));
    assert!(lines[..12]
        .iter()
        .all(|line| line.color == debug_light_sections::LIGHT_AND_BLOCKS_LINE_COLOR));
    assert!(lines[12..]
        .iter()
        .all(|line| line.color == debug_light_sections::LIGHT_ONLY_LINE_COLOR));
    assert!(lines
        .iter()
        .all(|line| line.width == debug_light_sections::LINE_WIDTH));

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightSections,
        crate::debug_entries::DebugScreenEntryStatus::Never,
    );
    assert_eq!(
        cache.gizmos(&input, &world, start + Duration::from_millis(100)),
        (Vec::new(), Vec::new())
    );
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeSkyLightSections,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let restarted_at = start + Duration::from_millis(100);
    assert_eq!(
        cache.gizmos(&input, &world, restarted_at),
        (rects.clone(), lines.clone())
    );

    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, air);
    assert_eq!(
        cache.gizmos(
            &input,
            &world,
            restarted_at + debug_light_sections::REFRESH_INTERVAL
        ),
        (rects, lines)
    );
    assert_eq!(
        cache.gizmos(
            &input,
            &world,
            restarted_at + debug_light_sections::REFRESH_INTERVAL + Duration::from_nanos(1)
        ),
        (Vec::new(), Vec::new())
    );
}

#[test]
fn debug_collision_lines_project_vanilla_white_stroke_cuboids_and_gates() {
    assert_eq!(DEBUG_COLLISION_QUERY_RADIUS, 6.0);
    assert_eq!(DEBUG_COLLISION_COLOR, [255; 4]);
    assert_eq!(DebugLine::DEFAULT_WIDTH, 2.5);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let now = Instant::now();
    let mut cache = DebugCollisionCache::default();
    let mut input = ClientInputState::new(true);
    assert!(cache.lines(&input, &world, now).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let lines = cache.lines(&input, &world, now);
    assert_eq!(lines.len(), 12);
    assert_eq!(
        lines[0],
        DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [255; 4], 2.5,)
    );
    assert!(lines
        .iter()
        .all(|line| line.color == [255; 4] && line.width == 2.5));

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    reduced.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    reduced.insert_decoded_chunk(empty_lightmap_test_chunk(reduced.dimension()));
    set_lightmap_test_block(&mut reduced, BlockPos { x: 0, y: 0, z: 0 }, stone);
    assert!(cache.lines(&input, &reduced, now).is_empty());
}

#[test]
fn debug_collision_cache_matches_vanilla_strict_refresh_interval_and_resets() {
    assert_eq!(DEBUG_COLLISION_REFRESH_INTERVAL, Duration::from_millis(100));

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let start = Instant::now();
    let mut cache = DebugCollisionCache::default();
    assert_eq!(cache.lines(&input, &world, start).len(), 12);

    set_lightmap_test_block(&mut world, BlockPos { x: 1, y: 0, z: 0 }, stone);
    assert_eq!(
        cache
            .lines(&input, &world, start + DEBUG_COLLISION_REFRESH_INTERVAL)
            .len(),
        12,
        "vanilla refreshes only after more than 100 ms"
    );
    let refreshed_at = start + DEBUG_COLLISION_REFRESH_INTERVAL + Duration::from_nanos(1);
    assert_eq!(cache.lines(&input, &world, refreshed_at).len(), 24);

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::Never,
    );
    assert!(cache.lines(&input, &world, refreshed_at).is_empty());
    set_lightmap_test_block(&mut world, BlockPos { x: 2, y: 0, z: 0 }, stone);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    assert_eq!(
        cache.lines(&input, &world, refreshed_at).len(),
        36,
        "re-enabling creates a fresh renderer cache and refreshes immediately"
    );
}

#[test]
fn debug_supporting_block_lines_project_local_player_and_legacy_highlights() {
    assert_eq!(DEBUG_SUPPORTING_BLOCK_OUTLINE_ALPHA, 102);
    assert_eq!(DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET, 0.02);
    assert_eq!(DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET, 0.04);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut pose = local_player_pose([0.5, 1.0, 0.5], 0.0, 0.0);
    pose.on_ground = true;
    world.set_local_player_pose(pose);
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

    let now = Instant::now();
    let mut cache = DebugSupportingBlockCache::default();
    let mut input = ClientInputState::new(true);
    assert!(cache.lines(&input, &world, now).is_empty());
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let lines = cache.lines(&input, &world, now);
    assert_eq!(lines.len(), 24);
    assert_eq!(
        lines[0],
        DebugLine::new(
            [-0.04, -0.04, -0.04],
            [1.04, -0.04, -0.04],
            [255, 0, 0, 102],
            2.5,
        )
    );
    assert_eq!(
        lines[12],
        DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [255, 0, 0, 255], 2.5,)
    );

    pose.on_ground = false;
    world.set_local_player_pose(pose);
    assert!(cache.lines(&input, &world, now).is_empty());

    let mut reduced =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 16, true);
    pose.on_ground = true;
    reduced.set_local_player_pose(pose);
    reduced.insert_decoded_chunk(empty_lightmap_test_chunk(reduced.dimension()));
    set_lightmap_test_block(&mut reduced, BlockPos { x: 0, y: 0, z: 0 }, stone);
    assert!(cache.lines(&input, &reduced, now).is_empty());
}

#[test]
fn debug_supporting_block_lines_use_cyan_for_legacy_effect_position() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let mut pose = local_player_pose([0.5, 1.25, 0.5], 0.0, 0.0);
    pose.on_ground = true;
    world.set_local_player_pose(pose);
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let piston_head = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:piston_head",
            &BTreeMap::from([
                ("facing".to_string(), "down".to_string()),
                ("short".to_string(), "false".to_string()),
                ("type".to_string(), "normal".to_string()),
            ]),
        )
        .expect("vanilla downward piston head state");
    set_lightmap_test_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, piston_head);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let lines = DebugSupportingBlockCache::default().lines(&input, &world, Instant::now());
    let cyan_outline = lines
        .iter()
        .filter(|line| line.color == [0, 255, 255, 102])
        .collect::<Vec<_>>();
    assert_eq!(cyan_outline.len(), 12);
    assert_eq!(cyan_outline[0].from, [-0.08, 0.92, -0.08]);
    assert_eq!(cyan_outline[0].to, [1.08, 0.92, -0.08]);
    assert!(lines
        .iter()
        .all(|line| line.width == DebugLine::DEFAULT_WIDTH));
}

#[test]
fn debug_supporting_block_entity_bias_matches_vanilla_java_hash() {
    assert_eq!((52.0_f64 + 0.132_453_657).to_string(), "52.132453657");
    assert_eq!(java_string_hash_code("52.132453657"), 1_431_313_029);
    assert!((debug_supporting_block_entity_bias(52) - 0.000_58).abs() < f64::EPSILON);

    assert_eq!((41.0_f64 + 0.132_453_657).to_string(), "41.132453657");
    assert_eq!(java_string_hash_code("41.132453657"), -1_195_785_627);
    assert!((debug_supporting_block_entity_bias(41) + 0.012_54).abs() < f64::EPSILON);
}

#[test]
fn debug_supporting_block_cache_keeps_membership_but_reads_live_support_state() {
    assert_eq!(
        DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL,
        Duration::from_millis(100)
    );
    assert_eq!(DEBUG_SUPPORTING_BLOCK_QUERY_RADIUS, 16.0);
    assert_eq!(DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR, [0, 255, 0, 255]);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 1.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let stone = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
        .expect("vanilla stone state");
    let air = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:air", &BTreeMap::new())
        .expect("vanilla air state");
    add_remote_supporting_block_test_entity(&mut world, 52, 2, 1.0, stone);

    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    let start = Instant::now();
    let mut cache = DebugSupportingBlockCache::default();
    let first = cache.lines(&input, &world, start);
    assert_eq!(first.len(), 24);
    assert_eq!(first[0].color, [0, 255, 0, 102]);
    assert!((first[0].from[0] - 1.958_84).abs() < 1.0e-6);

    add_remote_supporting_block_test_entity(&mut world, 53, 4, 1.0, stone);
    assert_eq!(
        cache
            .lines(
                &input,
                &world,
                start + DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL,
            )
            .len(),
        24,
        "vanilla refreshes membership only after more than 100 ms"
    );
    let refreshed_at = start + DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL + Duration::from_nanos(1);
    assert_eq!(cache.lines(&input, &world, refreshed_at).len(), 48);

    set_lightmap_test_block(&mut world, BlockPos { x: 2, y: 0, z: 0 }, air);
    for _ in 0..2 {
        assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
            id: 52,
            has_position: false,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground: true,
        }));
    }
    assert_eq!(
        cache
            .lines(&input, &world, refreshed_at + Duration::from_millis(1))
            .len(),
        24,
        "cached members still project their live supporting-block state each frame"
    );

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::Never,
    );
    assert!(cache.lines(&input, &world, refreshed_at).is_empty());
    add_remote_supporting_block_test_entity(&mut world, 54, 6, 1.0, stone);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );
    assert_eq!(
        cache.lines(&input, &world, refreshed_at).len(),
        48,
        "re-enabling creates a fresh membership cache and refreshes immediately"
    );
}

#[test]
fn debug_supporting_block_remote_legacy_effect_uses_biased_cyan_outline() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 1.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let piston_head = world
        .registries()
        .block_state_id_by_name_and_properties(
            "minecraft:piston_head",
            &BTreeMap::from([
                ("facing".to_string(), "down".to_string()),
                ("short".to_string(), "false".to_string()),
                ("type".to_string(), "normal".to_string()),
            ]),
        )
        .expect("vanilla downward piston head state");
    add_remote_supporting_block_test_entity(&mut world, 41, 2, 1.25, piston_head);
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    let lines = DebugSupportingBlockCache::default().lines(&input, &world, Instant::now());
    let cyan_outline = lines
        .iter()
        .filter(|line| line.color == [0, 255, 255, 102])
        .collect::<Vec<_>>();
    assert_eq!(cyan_outline.len(), 12);
    assert!((cyan_outline[0].from[0] - 1.945_08).abs() < 1.0e-6);
    assert!((cyan_outline[0].from[1] - 0.945_08).abs() < 1.0e-6);
    assert!(lines
        .iter()
        .any(|line| line.color == DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR));
}

#[test]
fn hud_boss_bar_projection_keeps_packet_add_order_and_maps_style_names() {
    let mut world = WorldStore::new();
    assert!(hud_boss_bars_from_world(&world).is_empty());

    // Insert the higher UUID first: Vanilla's LinkedHashMap and the world
    // both preserve packet-add order rather than sorting by UUID.
    let dragon = uuid::Uuid::from_u128(7);
    let wither = uuid::Uuid::from_u128(2);
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: dragon,
        operation: bbb_protocol::packets::BossEventOperation::Add {
            name: styled_boss_component("Ender Dragon"),
            progress: 0.75,
            color: bbb_protocol::packets::BossBarColor::Purple,
            overlay: bbb_protocol::packets::BossBarOverlay::Progress,
            flags: bbb_protocol::packets::BossEventFlags {
                darken_screen: true,
                play_music: false,
                create_world_fog: true,
            },
        },
    });
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: wither,
        operation: bbb_protocol::packets::BossEventOperation::Add {
            name: Box::new(bbb_protocol::RichTextComponent::plain("Wither")),
            progress: 0.5,
            color: bbb_protocol::packets::BossBarColor::Red,
            overlay: bbb_protocol::packets::BossBarOverlay::Notched10,
            flags: bbb_protocol::packets::BossEventFlags {
                darken_screen: false,
                play_music: false,
                create_world_fog: false,
            },
        },
    });

    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].name_runs.len(), 1);
    assert_eq!(bars[0].name_runs[0].text, "Ender Dragon");
    assert!(bars[0].name_runs[0].style.bold);
    assert_eq!(bars[0].name_runs[0].color, Some(0xFF_55_55));
    assert_eq!(bars[0].progress, 0.75);
    assert_eq!(bars[0].color, bbb_renderer::HudBossBarColor::Purple);
    assert_eq!(bars[0].overlay, bbb_renderer::HudBossBarOverlay::Progress);
    assert_eq!(
        bars[1].name_runs,
        vec![bbb_renderer::HudStyledTextRun::plain("Wither")]
    );
    assert_eq!(bars[1].color, bbb_renderer::HudBossBarColor::Red);
    assert_eq!(bars[1].overlay, bbb_renderer::HudBossBarOverlay::Notched10);

    // Style updates re-project; removing a bar drops it from the list. The
    // darken/fog flags never ride the bar draw (they stay behind the world's
    // `boss_overlay_should_*` queries).
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: wither,
        operation: bbb_protocol::packets::BossEventOperation::UpdateStyle {
            color: bbb_protocol::packets::BossBarColor::Yellow,
            overlay: bbb_protocol::packets::BossBarOverlay::Notched20,
        },
    });
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars[1].color, bbb_renderer::HudBossBarColor::Yellow);
    assert_eq!(bars[1].overlay, bbb_renderer::HudBossBarOverlay::Notched20);
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: wither,
        operation: bbb_protocol::packets::BossEventOperation::Remove,
    });
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].progress, 0.75);
}

#[test]
fn hud_boss_bar_runtime_interpolates_progress_for_exactly_one_hundred_milliseconds() {
    let id = uuid::Uuid::from_u128(7);
    let start = Instant::now();
    let mut world = WorldStore::new();
    add_boss_bar_with_id_and_progress(&mut world, id, 0.0);
    let mut runtime = HudRenderRuntime::default();

    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(&world, 1, start, start, false)),
        Some(vec![0.0])
    );
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 1.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(&world, 1, start, start, false)),
        Some(vec![0.0])
    );
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            1,
            start,
            start + Duration::from_millis(50),
            true,
        )),
        Some(vec![0.5])
    );
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            1,
            start,
            start + Duration::from_millis(100),
            true,
        )),
        Some(vec![1.0])
    );
    assert!(matches!(
        runtime.boss_bars_update(&world, 1, start, start + Duration::from_millis(101), true,),
        RendererFrameUpdate::Unchanged
    ));
}

#[test]
fn hud_boss_bar_runtime_samples_interrupted_and_equal_target_updates() {
    let id = uuid::Uuid::from_u128(8);
    let start = Instant::now();
    let mut world = WorldStore::new();
    add_boss_bar_with_id_and_progress(&mut world, id, 0.0);
    let mut runtime = HudRenderRuntime::default();
    let _ = runtime.boss_bars_update(&world, 1, start, start, false);

    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 1.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            1,
            start,
            start + Duration::from_millis(50),
            false,
        )),
        Some(vec![0.5])
    );

    // Even an equal-valued packet restarts `LerpingBossEvent` from the
    // currently visible value rather than being coalesced away.
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 1.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            1,
            start + Duration::from_millis(50),
            start + Duration::from_millis(100),
            false,
        )),
        Some(vec![0.75])
    );

    // An interrupted update likewise samples the active curve at the packet
    // time, then begins the replacement curve from that visible value.
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::UpdateProgress { progress: 0.0 },
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            1,
            start + Duration::from_millis(100),
            start + Duration::from_millis(150),
            false,
        )),
        Some(vec![0.375])
    );
}

#[test]
fn hud_boss_bar_runtime_jumps_on_add_replacement_remove_and_connection_reset() {
    let id = uuid::Uuid::from_u128(9);
    let start = Instant::now();
    let mut world = WorldStore::new();
    add_boss_bar_with_id_and_progress(&mut world, id, 0.25);
    let mut runtime = HudRenderRuntime::default();
    let _ = runtime.boss_bars_update(&world, 4, start, start, false);

    add_boss_bar_with_id_and_progress(&mut world, id, 0.8);
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            4,
            start + Duration::from_millis(10),
            start + Duration::from_millis(10),
            false,
        )),
        Some(vec![0.8])
    );
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            5,
            start + Duration::from_millis(20),
            start + Duration::from_millis(20),
            true,
        )),
        Some(vec![0.8])
    );

    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::Remove,
    }));
    assert_eq!(
        boss_bar_progresses(runtime.boss_bars_update(
            &world,
            5,
            start + Duration::from_millis(30),
            start + Duration::from_millis(30),
            false,
        )),
        Some(Vec::new())
    );
}

#[test]
fn hud_boss_bar_projection_covers_every_vanilla_color_and_overlay() {
    // The world stores vanilla `getName` strings and the projection re-parses
    // them (`HudBossBarColor::from_name` / `HudBossBarOverlay::from_name`):
    // every protocol color x overlay combination must survive, or bars would
    // silently vanish on a name mismatch.
    let colors = [
        bbb_protocol::packets::BossBarColor::Pink,
        bbb_protocol::packets::BossBarColor::Blue,
        bbb_protocol::packets::BossBarColor::Red,
        bbb_protocol::packets::BossBarColor::Green,
        bbb_protocol::packets::BossBarColor::Yellow,
        bbb_protocol::packets::BossBarColor::Purple,
        bbb_protocol::packets::BossBarColor::White,
    ];
    let overlays = [
        bbb_protocol::packets::BossBarOverlay::Progress,
        bbb_protocol::packets::BossBarOverlay::Notched6,
        bbb_protocol::packets::BossBarOverlay::Notched10,
        bbb_protocol::packets::BossBarOverlay::Notched12,
        bbb_protocol::packets::BossBarOverlay::Notched20,
    ];
    let mut world = WorldStore::new();
    let mut id = 0u128;
    for color in colors {
        for overlay in overlays {
            id += 1;
            world.apply_boss_event(bbb_protocol::packets::BossEvent {
                id: uuid::Uuid::from_u128(id),
                operation: bbb_protocol::packets::BossEventOperation::Add {
                    name: Box::new(bbb_protocol::RichTextComponent::plain(format!("boss {id}"))),
                    progress: 1.0,
                    color,
                    overlay,
                    flags: bbb_protocol::packets::BossEventFlags {
                        darken_screen: false,
                        play_music: false,
                        create_world_fog: false,
                    },
                },
            });
        }
    }
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars.len(), colors.len() * overlays.len());
}

#[test]
fn renderer_frame_item_and_entity_projections_extract_after_tick_advances() {
    let source = include_str!("../runtime.rs");
    let entity_tick = source
        .find("let advanced_ticks = advance_entity_client_animations(")
        .expect("pump should advance entity client animations before render extraction");
    let partial_tick = source
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick();")
        .expect("pump should compute the render partial tick");
    let client_time = source
        .find("let running_ticks = client_animation_ticks.last_running_level_ticks;")
        .expect("pump should complete the client-level tick before item model extraction");
    let cooldown_tick = source
        .find("world.advance_item_cooldowns(advanced_ticks);")
        .expect("pump should advance item cooldowns before item model extraction");
    let input_advance = source
        .find("advance_player_input_for_client_ticks(")
        .expect("pump should advance input before held item extraction");
    let using_item_tick = source
        .find("world.advance_local_using_item_ticks(advanced_ticks);")
        .expect("pump should advance local use-item ticks before held item extraction");
    let item_age_extract = source
        .find("let item_model_age_ticks = world")
        .expect("pump should compute dropped item model age");
    let shader_time_extract = source
        .find("let shader_game_time_ticks = world")
        .expect("pump should compute shader GameTime from world time");
    let dropped_models = source
        .find("let dropped_item_models = dropped_item_models(")
        .expect("pump should extract dropped item models");
    let billboards = source
        .find("let item_entity_billboards = item_entity_billboards_from_world(")
        .expect("pump should extract item entity billboards");
    let entity_instances = source
        .find("let mut entity_instances =\n        visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick(")
        .expect("pump should extract entity model instances");
    let held_models = source
        .find("let held_item_models =")
        .expect("pump should extract held item models");
    let chest_instances = source
        .find("entity_instances.extend(chest_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract chest block-entity model instances");
    let chest_lid_tick = source
        .find("world.advance_chest_lid_ticks(running_ticks);")
        .expect("pump should advance chest lid ticks");
    // Vanilla `ClientLevel.tickBlockEntities` runs the chest lid ticker before
    // render extraction reads the lerped openness; the chest instances join the
    // single entity-model submission stream after held-item baking (chests have
    // no hands to bake).
    assert!(chest_lid_tick < chest_instances);
    assert!(held_models < chest_instances);
    // The bell shake ticker (`BellBlockEntity.clientTick`) likewise runs before
    // render extraction reads `ticks + partialTicks`; bed and bell instances
    // join the same stream after held-item baking.
    let bell_shake_tick = source
        .find("world.advance_bell_shake_ticks(running_ticks);")
        .expect("pump should advance bell shake ticks");
    let bed_instances = source
        .find("entity_instances.extend(bed_model_instances_from_world(world));")
        .expect("pump should extract bed block-entity model instances");
    let bell_instances = source
        .find("entity_instances.extend(bell_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract bell block-entity model instances");
    assert!(bell_shake_tick < bell_instances);
    assert!(held_models < bed_instances);
    assert!(held_models < bell_instances);
    // The shulker box lid ticker (`ShulkerBoxBlockEntity.tick`) and the pot
    // wobble clock likewise run before render extraction reads their lerped
    // progress; both instance streams join after held-item baking.
    let shulker_box_lid_tick = source
        .find("world.advance_shulker_box_lid_ticks(running_ticks);")
        .expect("pump should advance shulker box lid ticks");
    let pot_wobble_tick = source
        .find("world.advance_decorated_pot_wobble_ticks(running_ticks);")
        .expect("pump should advance decorated pot wobble ticks");
    let shulker_box_instances = source
        .find("entity_instances.extend(shulker_box_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract shulker box block-entity model instances");
    let decorated_pot_instances = source
        .find("entity_instances.extend(decorated_pot_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract decorated pot block-entity model instances");
    assert!(shulker_box_lid_tick < shulker_box_instances);
    assert!(pot_wobble_tick < decorated_pot_instances);
    assert!(held_models < shulker_box_instances);
    assert!(held_models < decorated_pot_instances);
    // The enchanting-table book ticker
    // (`EnchantingTableBlockEntity.bookAnimationTick`) runs before render
    // extraction reads the lerped flip/open/rot; both book streams join after
    // held-item baking.
    let enchanting_book_tick = source
        .find("world.advance_enchanting_table_book_ticks(running_ticks);")
        .expect("pump should advance enchanting table book ticks");
    let enchanting_book_instances = source
        .find("enchanting_table_book_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract enchanting table book block-entity model instances");
    let lectern_book_instances = source
        .find("entity_instances.extend(lectern_book_model_instances_from_world(world));")
        .expect("pump should extract lectern book block-entity model instances");
    assert!(enchanting_book_tick < enchanting_book_instances);
    assert!(held_models < enchanting_book_instances);
    assert!(held_models < lectern_book_instances);
    // The conduit ticker (`ConduitBlockEntity.clientTick`) advances the
    // active rotation and shape/hunting state before render extraction expands
    // the block entity into shell/cage/wind/eye model instances. The eye uses
    // the same frame camera pose snapshot as the first-person and level
    // renderer paths.
    let conduit_tick = source
        .find("world.advance_conduit_ticks(running_ticks);")
        .expect("pump should advance conduit ticks");
    let camera_pose = source
        .find(
            "let camera_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick);",
        )
        .expect("pump should extract the frame camera pose");
    let conduit_instances = source
        .find("entity_instances.extend(conduit_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract conduit block-entity model instances");
    assert!(conduit_tick < conduit_instances);
    assert!(camera_pose < conduit_instances);
    assert!(held_models < conduit_instances);
    // End gateways run their client block-entity beam ticker before the
    // renderer reads age/cooldown for the cube+beam model instance.
    let end_gateway_tick = source
        .find("world.advance_end_gateway_ticks(running_ticks);")
        .expect("pump should advance end gateway ticks");
    let end_portal_instances = source
        .find("entity_instances.extend(end_portal_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract end portal/gateway block-entity model instances");
    assert!(conduit_tick < end_gateway_tick);
    assert!(end_gateway_tick < end_portal_instances);
    assert!(held_models < end_portal_instances);
    // Powered dragon and piglin skull/head block entities tick their animation
    // clock before the renderer reads the skull model animation position; the
    // model instances join the same entity stream after held-item baking.
    let skull_tick = source
        .find("world.advance_skull_block_ticks(running_ticks);")
        .expect("pump should advance skull block-entity animation ticks");
    let skull_instances = source
        .find("entity_instances.extend(skull_model_instances_from_world_at_partial_tick(")
        .expect("pump should extract skull block-entity model instances");
    assert!(end_gateway_tick < skull_tick);
    assert!(skull_tick < skull_instances);
    assert!(held_models < skull_instances);
    // Ordinary spawners run `BaseSpawner.clientTick` before the renderer reads
    // the display entity spin/scale source; the display entity itself reuses
    // the shared entity-model stream.
    let spawner_tick = source
        .find("world.advance_spawner_block_ticks(running_ticks);")
        .expect("pump should advance ordinary spawner ticks");
    let spawner_instances = source
        .find(
            "entity_instances.extend(spawner_display_entity_instances_from_world_at_partial_tick(",
        )
        .expect("pump should extract spawner display entity model instances");
    assert!(skull_tick < spawner_tick);
    assert!(spawner_tick < spawner_instances);
    assert!(skull_instances < spawner_instances);
    assert!(held_models < spawner_instances);
    let moving_piston_tick = source
        .find("world.advance_moving_piston_ticks(running_ticks);")
        .expect("pump should advance moving piston block-entity ticks");
    let entity_block_meshes = source
        .find("let entity_block_models = entity_block_models(")
        .expect("pump should extract entity block item models");
    assert!(spawner_tick < moving_piston_tick);
    assert!(moving_piston_tick < entity_block_meshes);
    let shader_time_frame = source
        .find("shader_game_time_ticks,")
        .expect("pump should pass shader GameTime through RendererFrame");
    assert!(shader_time_extract < shader_time_frame);
    let item_frame_models = source
        .find("let item_frame_models = item_frame_models_with_map_cache(")
        .expect("pump should extract item frame models");
    for advance in [
        entity_tick,
        partial_tick,
        client_time,
        cooldown_tick,
        input_advance,
        using_item_tick,
    ] {
        assert!(
            advance < item_age_extract,
            "vanilla `Minecraft.tick` advances gameplay/entity state before `LevelRenderer.extractLevel`"
        );
        assert!(
            advance < shader_time_extract,
            "vanilla GlobalSettingsUniform.GameTime uses post-tick world time plus the frame partial tick"
        );
    }
    for extraction in [
        dropped_models,
        billboards,
        entity_instances,
        held_models,
        spawner_instances,
        item_frame_models,
        entity_block_meshes,
    ] {
        assert!(
            item_age_extract < extraction,
            "item/entity RendererFrame fields should read the post-tick world snapshot"
        );
    }
}

#[test]
fn renderer_frame_commits_the_same_current_post_effect_used_by_hud() {
    let runtime_source = include_str!("../runtime.rs");
    let extraction = runtime_source
        .find("let current_post_effect_id = current_post_effect_id_from_world(world);")
        .expect("pump should extract the current post effect once per frame");
    let hud_projection = runtime_source
        .find("            current_post_effect_id,\n")
        .expect("HUD should consume the same-frame current post effect");
    let frame_projection = runtime_source
        .find("            current_post_effect_id,\n            shader_game_time_ticks,")
        .expect("RendererFrame should carry the same current post effect");
    assert!(extraction < hud_projection && hud_projection < frame_projection);

    let frame_source = include_str!("render_extract.rs");
    assert!(frame_source.contains("pub(crate) current_post_effect_id: Option<&'a str>"));
    assert!(
        frame_source.contains("renderer.set_current_post_effect_id(frame.current_post_effect_id);")
    );
}

#[test]
fn renderer_frame_block_destroy_overlays_extract_after_destroy_tick() {
    let source = include_str!("../runtime.rs");
    let destroy_tick = source
        .find("advance_block_destruction_render_ticks(world, running_ticks);")
        .expect("pump should advance block-destroy render ticks");
    let block_destroy_stages = source
        .find("let block_destroy_stages = block_destroy_stages_from_world(world);")
        .expect("pump should select the highest block-destroy stage once");
    let block_entity_crumbling = source
        .find("apply_block_entity_crumbling(")
        .expect("pump should annotate eligible block-entity models");
    let block_destroy_extract = source
        .find("RendererFrameUpdate::Replace(block_destroy_overlays_from_stages(")
        .expect("pump should extract block-destroy overlays");

    assert!(
        destroy_tick < block_destroy_stages
            && block_destroy_stages < block_entity_crumbling
            && block_entity_crumbling < block_destroy_extract,
        "vanilla `LevelRenderer.extractBlockDestroyAnimation` reads one post-client-tick stage projection for block and block-entity geometry"
    );
}

#[test]
fn renderer_frame_outlines_extract_after_input_camera_and_partial_tick() {
    let source = include_str!("../runtime.rs");
    let input_advance = source
        .find("advance_player_input_for_client_ticks(")
        .expect("pump should advance player input before outline extraction");
    let using_item_tick = source
        .find("world.advance_local_using_item_ticks(advanced_ticks);")
        .expect("pump should advance local use-item ticks before outline extraction");
    let entity_tick = source
        .find("let advanced_ticks = advance_entity_client_animations(")
        .expect("pump should advance entity client animations before outline extraction");
    let partial_tick = source
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick();")
        .expect("pump should compute partial tick before outline extraction");
    let camera_pose = source
        .find(
            "let camera_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick);",
        )
        .expect("pump should extract camera pose before outlines");
    let selection_outline = source
        .find("let selection_outline = selection_outline_from_camera(")
        .expect("pump should extract selection outline");
    let entity_scene_outline = source
        .find("let entity_scene_outline =")
        .expect("pump should extract entity scene outline");
    let entity_target_outline = source
        .find("let entity_target_outline =")
        .expect("pump should extract entity target outline");

    for advance in [input_advance, using_item_tick, entity_tick, partial_tick] {
        assert!(
            advance < camera_pose,
            "vanilla picks and outlines use post-input camera/entity state before render extract"
        );
    }
    for outline in [
        selection_outline,
        entity_scene_outline,
        entity_target_outline,
    ] {
        assert!(
            camera_pose < outline,
            "outline RendererFrame fields should read one camera pose snapshot"
        );
    }
}

#[test]
fn renderer_frame_commits_debug_gizmos_from_one_water_sample_snapshot() {
    let runtime_source = include_str!("../runtime.rs");
    let camera_pose = runtime_source
        .find(
            "let camera_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick);",
        )
        .expect("pump should extract one camera pose");
    let samples = runtime_source
        .find("let debug_water_samples = debug_water_samples(input, world);")
        .expect("pump should extract water samples once");
    let boxes = runtime_source
        .find("let mut debug_filled_boxes = debug_water_boxes(&debug_water_samples);")
        .expect("pump should project debug water boxes");
    let heightmap_extend = runtime_source
        .find("debug_filled_boxes.extend(debug_heightmap_boxes(input, world, camera_pose));")
        .expect("heightmap boxes should follow water boxes");
    let game_test_extract = runtime_source
        .find("let game_test_highlight_gizmos = game_test_highlight_gizmos(world);")
        .expect("GameTest highlights should project from canonical marker state");
    let game_test_boxes = runtime_source
        .find("debug_filled_boxes.extend(game_test_highlight_gizmos.filled_boxes);")
        .expect("GameTest boxes should follow ordinary debug boxes");
    let solid_faces = runtime_source
        .find("let mut debug_filled_rects = debug_solid_face_rects(input, world, camera_pose);")
        .expect("pump should project solid-face rectangles");
    let text = runtime_source
        .find(
            "let mut debug_text_billboards = debug_water_amount_billboards(&debug_water_samples);",
        )
        .expect("pump should project water amount labels from the same samples");
    let light_text = runtime_source
        .find("debug_text_billboards.extend(debug_light_level_billboards(input, world, camera_pose));")
        .expect("light debug labels should follow water labels");
    let selection_text = runtime_source
        .find("debug_text_billboards.extend(debug_selection_text_billboards(")
        .expect("entity debug labels should use the shared 3D text surface");
    let game_test_text = runtime_source
        .find("debug_text_billboards.extend(game_test_highlight_gizmos.text_billboards);")
        .expect("GameTest labels should follow ordinary debug labels");
    let lines = runtime_source
        .find("let mut debug_lines = debug_collision_cache.lines(input, world, now);")
        .expect("pump should project cached collision debug lines");
    let support_lines = runtime_source
        .find("debug_lines.extend(debug_supporting_block_cache.lines(input, world, now));")
        .expect("supporting-block lines should follow collision lines");
    let sky_light_sections = runtime_source
        .find("debug_sky_light_section_cache.gizmos(input, world, now);")
        .expect("pump should read one cached sky-light-section snapshot");
    let sky_light_rects = runtime_source
        .find("debug_filled_rects.extend(debug_sky_light_section_rects);")
        .expect("sky-light section faces should follow solid faces");
    let sky_light_lines = runtime_source
        .find("debug_lines.extend(debug_sky_light_section_lines);")
        .expect("sky-light section edges should follow supporting-block lines");
    let frame = runtime_source
        .find("            debug_filled_boxes,\n")
        .expect("RendererFrame should carry debug filled boxes");
    assert!(
        camera_pose < game_test_extract
            && game_test_extract < samples
            && samples < boxes
            && boxes < heightmap_extend
            && heightmap_extend < game_test_boxes
            && game_test_boxes < solid_faces
            && solid_faces < text
            && text < light_text
            && light_text < selection_text
            && selection_text < game_test_text
            && game_test_text < lines
            && lines < support_lines
            && support_lines < sky_light_sections
            && sky_light_sections < sky_light_rects
            && sky_light_rects < sky_light_lines
            && sky_light_lines < frame
    );

    let frame_source = include_str!("render_extract.rs");
    assert!(frame_source.contains("pub(crate) debug_filled_boxes: Vec<DebugFilledBox>"));
    assert!(frame_source.contains("renderer.set_debug_filled_boxes(frame.debug_filled_boxes);"));
    assert!(frame_source.contains("pub(crate) debug_filled_rects: Vec<DebugFilledRect>"));
    assert!(frame_source.contains("renderer.set_debug_filled_rects(frame.debug_filled_rects);"));
    assert!(frame_source.contains("pub(crate) debug_text_billboards: Vec<DebugTextBillboard>"));
    assert!(
        frame_source.contains("renderer.set_debug_text_billboards(frame.debug_text_billboards);")
    );
    assert!(frame_source.contains("pub(crate) debug_lines: Vec<DebugLine>"));
    assert!(frame_source.contains("renderer.set_debug_lines(frame.debug_lines);"));
}

#[test]
fn renderer_frame_cloud_frame_extracts_after_client_time_camera_and_partial_tick() {
    let source = include_str!("../runtime.rs");
    let entity_tick = source
        .find("let advanced_ticks = advance_entity_client_animations(")
        .expect("pump should advance entity client animations before cloud frame extraction");
    let partial_tick = source
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick();")
        .expect("pump should compute the render partial tick before cloud frame extraction");
    let client_time = source
        .find("let running_ticks = client_animation_ticks.last_running_level_ticks;")
        .expect("pump should complete the client-level tick before cloud frame extraction");
    let camera_pose = source
        .find(
            "let camera_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick);",
        )
        .expect("pump should extract camera pose before cloud frame");
    let cloud_frame = source
        .find("let cloud_frame = cloud_frame_for_world(")
        .expect("pump should extract cloud frame");

    for advance in [entity_tick, partial_tick, client_time] {
        assert!(
            advance < cloud_frame,
            "vanilla cloud rendering samples post-tick game time with the frame partial tick"
        );
    }
    assert!(
        camera_pose < cloud_frame,
        "cloud frame should read the same frame camera pose used by level rendering"
    );
}

#[test]
fn renderer_frame_weather_extracts_after_client_time_camera_and_partial_tick() {
    let source = include_str!("../runtime.rs");
    let entity_tick = source
        .find("let advanced_ticks = advance_entity_client_animations(")
        .expect("pump should advance entity client animations before weather extraction");
    let partial_tick = source
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick();")
        .expect("pump should compute the render partial tick before weather extraction");
    let client_time = source
        .find("let running_ticks = client_animation_ticks.last_running_level_ticks;")
        .expect("pump should complete the client-level tick before weather extraction");
    let camera_pose = source
        .find(
            "let camera_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick);",
        )
        .expect("pump should extract camera pose before weather");
    let weather = source
        .find("let weather_render_state =")
        .expect("pump should extract weather render state");

    for advance in [entity_tick, partial_tick, client_time] {
        assert!(
            advance < weather,
            "vanilla weather extraction samples post-tick level time with the frame partial tick"
        );
    }
    assert!(
        camera_pose < weather,
        "weather render state should read the frame camera pose used by LevelRenderer.extractLevel"
    );
}

#[test]
fn particle_lights_refresh_after_particle_tick_and_frame_extract_inputs() {
    let source = include_str!("../runtime.rs");
    let input_advance = source
        .find("advance_player_input_for_client_ticks(")
        .expect("pump should advance player input before particle tick");
    let destroy_advance = source
        .find("advance_destroying_block_at_partial_tick(")
        .expect("pump should advance destroy input before particle tick");
    let local_player_action_particles = source
        .find("submit_local_player_action_particles(")
        .expect("pump should submit local player-action particles before particle tick");
    let use_advance = source
        .find("advance_using_item_at_partial_tick(")
        .expect("pump should advance use-item input before particle tick");
    let using_item_tick = source
        .find("world.advance_local_using_item_ticks(advanced_ticks);")
        .expect("pump should advance local use-item ticks before particle tick");
    let particle_camera_pose = source
        .find(
            "let particle_camera_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick);",
        )
        .expect("pump should sample particle sound camera before particle tick");
    let particle_scope_context = source
        .find("let particle_scope_context =")
        .expect("pump should sample local scoping state before particle tick");
    let particle_sound_camera_position = source
        .find("let particle_sound_camera_position =")
        .expect("pump should convert particle sound camera before particle tick");
    let particle_player_motion_contexts = source
        .find("let particle_player_motion_contexts =")
        .expect("pump should sample nearest-player candidate motion state before particle tick");
    let particle_entity_target_contexts = source
        .find("let particle_entity_target_contexts =")
        .expect("pump should sample entity target state before particle tick");
    let explosion_tracker = source
        .find("particle_runtime.tick_explosion_particles(")
        .expect("pump should tick queued explosion effects before particle tick");
    let primed_tnt_smoke = source
        .find("submit_primed_tnt_smoke_particles(particle_batches, world, advanced_ticks);")
        .expect("pump should emit PrimedTnt client smoke before particle tick");
    let entity_client_tick_effects = source
        .find("submit_entity_client_tick_effects(")
        .expect("pump should emit ordered entity client tick effects before particle tick");
    let ominous_item_spawner_particles = source
        .find(
            "submit_ominous_item_spawner_particles(particle_batches, world, &mut particle_events);",
        )
        .expect("pump should emit OminousItemSpawner client particles before particle tick");
    let particle_tick = source
        .find("renderer.advance_particles_with_world_and_particle_contexts_and_sound_camera(")
        .expect("pump should advance particles");
    let particle_sound_drain = source
        .find("let particle_sound_events = renderer.drain_particle_sound_events();")
        .expect("pump should drain particle sound events after particle tick");
    let particle_sound_emit = source
        .find("emit_particle_sound_events(&mut audio_events, particle_sound_events);")
        .expect("pump should emit particle sound events");
    let camera_pose = source
        .find(
            "let camera_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick);",
        )
        .expect("pump should bind frame camera pose before particle light refresh");
    let block_destroy_extract = source
        .find("RendererFrameUpdate::Replace(block_destroy_overlays_from_stages(")
        .expect("pump should extract block-destroy overlays before particle light refresh");
    let particle_light_refresh = source
        .find("renderer.refresh_particle_lights(")
        .expect("pump should refresh particle light");
    let frame_commit = source
        .find("apply_renderer_frame(")
        .expect("pump should commit the extracted renderer frame");

    for advance in [input_advance, destroy_advance, use_advance, using_item_tick] {
        assert!(
            advance < particle_tick,
            "vanilla `Minecraft.tick` handles gameplay input before `ParticleEngine.tick`"
        );
    }
    assert!(
        destroy_advance < local_player_action_particles
            && local_player_action_particles < particle_tick,
        "held-destroy effects should submit after input advances and before ParticleEngine.tick"
    );
    assert!(
        using_item_tick < particle_scope_context && particle_scope_context < particle_tick,
        "SpellParticle.tick samples post-input local scoping state during particle tick"
    );
    assert!(
        using_item_tick < particle_camera_pose
            && particle_camera_pose < particle_sound_camera_position
            && particle_sound_camera_position < particle_tick,
        "scheduled particle sounds choose far variants from the particle-tick camera"
    );
    assert!(
        using_item_tick < particle_player_motion_contexts
            && particle_player_motion_contexts < particle_tick,
        "player-coupled particles sample post-input player motion candidates during particle tick"
    );
    assert!(
        particle_entity_target_contexts < particle_tick,
        "entity-target particles sample world entity positions during particle tick"
    );
    assert!(
        particle_sound_camera_position < explosion_tracker && explosion_tracker < particle_tick,
        "ClientExplosionTracker should use the tick camera and submit before ParticleEngine.tick"
    );
    assert!(
        primed_tnt_smoke < particle_tick,
        "vanilla `PrimedTnt.tick` emits smoke before ParticleEngine.tick advances particles"
    );
    assert!(
        primed_tnt_smoke < entity_client_tick_effects && entity_client_tick_effects < particle_tick,
        "entity client-tick effects should be submitted before ParticleEngine.tick advances particles"
    );
    assert!(
        entity_client_tick_effects < ominous_item_spawner_particles
            && ominous_item_spawner_particles < particle_tick,
        "OminousItemSpawner client particles should be submitted before ParticleEngine.tick advances particles"
    );
    assert!(
        particle_tick < particle_sound_drain && particle_sound_drain < particle_sound_emit,
        "particle-local sounds should drain after particle tick and emit through audio sink"
    );
    assert!(
        particle_sound_emit < particle_light_refresh,
        "particle-local sounds should emit before render light extraction"
    );
    assert!(
        particle_tick < particle_light_refresh,
        "particle lights should sample positions after particle tick"
    );
    for extraction in [camera_pose, block_destroy_extract] {
        assert!(
            extraction < particle_light_refresh,
            "vanilla `ParticleEngine.extract` samples light during level render extraction"
        );
    }
    assert!(
        particle_light_refresh < frame_commit,
        "particle light refresh should finish before the frame can be rendered"
    );
}

#[test]
fn clear_color_applies_client_sky_flash_color_layer() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    let baseline = clear_color_for_world(&world, false);

    world.set_sky_flash_time(2);
    let flashed = clear_color_for_world(&world, false);
    let expected = clear_color_with_sky_flash(baseline);

    assert_clear_color_close(flashed, expected);
    assert_ne!(flashed, baseline);
}

#[test]
fn clear_color_hides_client_sky_flash_when_option_is_enabled() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    let baseline = clear_color_for_world(&world, false);

    world.set_sky_flash_time(2);
    let hidden = clear_color_for_world(&world, true);

    assert_eq!(hidden, baseline);
}

#[test]
fn clear_color_uses_vanilla_dimension_fog_and_sky_color_without_camera_biome() {
    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut overworld, 6_000);
    assert_clear_color_close(
        clear_color_for_world(&overworld, false),
        clear_color_for_day_time(6_000, 0.0, 0.0),
    );

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    set_world_day_time(&mut nether, 6_000);
    assert_clear_color_close(
        clear_color_for_world(&nether, false),
        clear_color_for_day_time_with_environment_colors(
            6_000,
            0.0,
            0.0,
            None,
            Some([0, 0, 0]),
            VanillaLightmapDimensionKind::Nether,
        ),
    );

    let mut end = world_with_dimension(2, "minecraft:the_end");
    set_world_day_time(&mut end, 6_000);
    assert_clear_color_close(
        clear_color_for_world(&end, false),
        clear_color_for_day_time_with_environment_colors(
            6_000,
            0.0,
            0.0,
            Some(VANILLA_END_FOG_COLOR),
            Some([0, 0, 0]),
            VanillaLightmapDimensionKind::End,
        ),
    );
}

#[test]
fn lightmap_dimension_kind_prefers_registry_identity_over_bootstrap_holder_id() {
    let world = world_with_dimension(0, "minecraft:overworld");
    let mut level = world.level_info().unwrap().clone();

    level.dimension_type_id = 0;
    level.dimension_type_name = Some("minecraft:the_nether".to_string());
    assert_eq!(
        vanilla_lightmap_dimension_kind(&level),
        VanillaLightmapDimensionKind::Nether
    );

    level.dimension_type_name = Some("example:custom".to_string());
    assert_eq!(
        vanilla_lightmap_dimension_kind(&level),
        VanillaLightmapDimensionKind::Other
    );

    level.dimension_type_id = 1;
    level.dimension_type_name = None;
    assert_eq!(
        vanilla_lightmap_dimension_kind(&level),
        VanillaLightmapDimensionKind::Nether
    );
}

#[test]
fn custom_dimension_direct_environment_attributes_drive_lightmap_and_fog() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));

    let lightmap = lightmap_environment_attributes_for_world(&world);
    assert_eq!(lightmap.sky_factor, 0.25);
    assert_close3(
        lightmap.block_light_tint,
        [
            0x20 as f32 / 255.0,
            0x40 as f32 / 255.0,
            0x60 as f32 / 255.0,
        ],
    );
    assert_close3(
        lightmap.sky_light_color,
        [
            0x40 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ],
    );
    assert_close3(
        lightmap.ambient_color,
        [
            0x30 as f32 / 255.0,
            0x50 as f32 / 255.0,
            0x70 as f32 / 255.0,
        ],
    );
    assert_close3(
        lightmap.night_vision_color,
        [
            0x50 as f32 / 255.0,
            0x70 as f32 / 255.0,
            0x90 as f32 / 255.0,
        ],
    );

    let atmospheric = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_eq!(atmospheric.environmental_start, 12.0);
    assert_eq!(atmospheric.environmental_end, 144.0);
    assert_eq!(atmospheric.sky_end, 96.0);
    assert_eq!(atmospheric.cloud_end, 192.0);
    assert_fog_environment_close(
        atmospheric,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_for_day_time_with_environment_colors(
                6_000,
                0.0,
                0.0,
                Some([0x12, 0x34, 0x56]),
                Some([0x34, 0x56, 0x78]),
                VanillaLightmapDimensionKind::Other,
            )),
            12.0,
            144.0,
            12,
            96.0,
            192.0,
        ),
    );

    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let water = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.5,
        0.0,
        false,
    );
    assert_fog_environment_close(
        water,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_from_argb(apply_fog_brightening(
                rgb_u8_to_argb([0x56, 0x78, 0x9a]),
                0.5,
            ))),
            -4.0,
            24.0,
            12,
            24.0,
            24.0,
        ),
    );
}

#[test]
fn custom_dimension_timeline_layers_drive_lightmap_from_session_registries() {
    let world = world_with_custom_dimension_timeline();

    let sampled = world.sampled_client_environment_attributes().unwrap();
    assert_eq!(sampled.sky_light_factor, Some(0.125));
    let lightmap = lightmap_environment_attributes_for_world(&world);
    assert_eq!(lightmap.sky_factor, 0.125);
}

#[test]
fn biome_positional_environment_values_override_custom_dimension_base() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[
                (
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#a0b0c0"),
                ),
                (
                    "minecraft:visual/fog_color",
                    TestEnvironmentAttributeValue::String("#908070"),
                ),
                (
                    "minecraft:visual/water_fog_color",
                    TestEnvironmentAttributeValue::String("#605040"),
                ),
                (
                    "minecraft:visual/water_fog_end_distance",
                    TestEnvironmentAttributeValue::Multiply(0.5),
                ),
                (
                    "minecraft:visual/block_light_tint",
                    TestEnvironmentAttributeValue::String("#102040"),
                ),
                (
                    "minecraft:visual/cloud_color",
                    TestEnvironmentAttributeValue::String("#80506070"),
                ),
                (
                    "minecraft:visual/sun_angle",
                    TestEnvironmentAttributeValue::Float(45.0),
                ),
            ],
        )],
    );

    let colors = camera_environment_colors(&world, camera_pose_from_world(&world));
    assert_eq!(colors.sky_color, Some([0xa0, 0xb0, 0xc0]));
    assert_eq!(colors.fog_color, Some([0x90, 0x80, 0x70]));
    assert_eq!(colors.water_fog_color, Some([0x60, 0x50, 0x40]));
    assert_eq!(colors.water_fog_end_distance, Some(24.0));
    assert_close3(
        lightmap_environment_attributes_for_world(&world).block_light_tint,
        [
            0x10 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ],
    );
    assert_close4(
        cloud_environment_for_world(&world).color,
        [
            0x50 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x70 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ],
    );
    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert!((sky.sun_angle_radians - 45.0_f32.to_radians()).abs() < 1e-6);
}

#[test]
fn clear_color_samples_camera_biome_fog_and_sky_color_attributes() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[
                (
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#102030"),
                ),
                (
                    "minecraft:visual/fog_color",
                    TestEnvironmentAttributeValue::String("#804020"),
                ),
            ],
        )],
    );

    let clear = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    let expected = clear_color_for_day_time_with_environment_colors(
        6_000,
        0.0,
        0.0,
        Some([0x80, 0x40, 0x20]),
        Some([0x10, 0x20, 0x30]),
        VanillaLightmapDimensionKind::Overworld,
    );

    assert_clear_color_close(clear, expected);
    assert_ne!(clear, clear_color_for_world(&world, false));
}

#[test]
fn sky_environment_samples_camera_biome_sky_color_for_sky_disc() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/sky_color",
                TestEnvironmentAttributeValue::String("#102030"),
            )],
        )],
    );

    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);

    assert_close3(
        [sky.color[0], sky.color[1], sky.color[2]],
        [
            0x10 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x30 as f32 / 255.0,
        ],
    );
    assert_eq!(sky.color[3], 1.0);
}

#[test]
fn sky_environment_applies_client_sky_flash_layer_and_dimension_gate() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_sky_flash_time(2);

    let flashed =
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    let hidden = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), true);
    let expected = rgb24(argb_srgb_lerp(
        VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
        rgb_u8_to_argb(VANILLA_OVERWORLD_SKY_COLOR),
        VANILLA_SKY_FLASH_SKY_COLOR,
    ));

    assert_close3(
        [flashed.color[0], flashed.color[1], flashed.color[2]],
        expected,
    );
    assert_ne!(flashed, hidden);

    let nether = world_with_dimension(1, "minecraft:the_nether");
    assert_eq!(
        sky_environment_for_world_at_camera(&nether, camera_pose_from_world(&nether), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn sky_environment_projects_end_skybox_state() {
    let end = world_with_dimension(2, "minecraft:the_end");

    let sky = sky_environment_for_world_at_camera(&end, camera_pose_from_world(&end), false);

    assert!(sky.end_sky_visible());
    assert!(!sky.is_visible());
    assert_eq!(sky.sunrise_sunset_color[3], 0.0);
}

#[test]
fn sky_environment_suppresses_every_sky_pass_for_boss_world_fog() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    add_boss_bar(&mut world, false, true);

    assert_eq!(
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn sky_environment_projects_sunrise_sunset_render_state() {
    let day_time = 71;
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, day_time);
    set_world_weather(&mut world, 0.25, 0.5);

    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    let sunrise_color = apply_weather_sunrise_sunset_color_layers(
        sample_periodic_argb_keyframes(
            day_time,
            &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
        0.25,
        0.5,
    );

    assert_close4(sky.sunrise_sunset_color, rgba32(sunrise_color));
    assert!(sky.sunrise_sunset_color[3] > 0.0);
    assert!((sky.sun_angle_radians - overworld_sun_angle(day_time).to_radians()).abs() < 1e-6);
    assert!((sky.moon_angle_radians - overworld_moon_angle(day_time).to_radians()).abs() < 1e-6);
    assert!((sky.rain_brightness - 0.75).abs() < 1e-6);
    assert_eq!(sky.moon_phase, SkyMoonPhase::FullMoon);
    assert!((sky.star_angle_radians - overworld_star_angle(day_time).to_radians()).abs() < 1e-6);
    let star_brightness = apply_weather_star_brightness_layers(
        sample_periodic_float_keyframes(
            day_time,
            &VANILLA_OVERWORLD_STAR_BRIGHTNESS_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
        0.25,
        0.5,
    );
    assert!((sky.star_brightness - star_brightness).abs() < 1e-6);
}

#[test]
fn sky_environment_projects_vanilla_moon_phase_cycle() {
    assert_eq!(overworld_moon_phase(0), SkyMoonPhase::FullMoon);
    assert_eq!(overworld_moon_phase(24_000), SkyMoonPhase::WaningGibbous);
    assert_eq!(overworld_moon_phase(48_000), SkyMoonPhase::ThirdQuarter);
    assert_eq!(overworld_moon_phase(72_000), SkyMoonPhase::WaningCrescent);
    assert_eq!(overworld_moon_phase(96_000), SkyMoonPhase::NewMoon);
    assert_eq!(overworld_moon_phase(120_000), SkyMoonPhase::WaxingCrescent);
    assert_eq!(overworld_moon_phase(144_000), SkyMoonPhase::FirstQuarter);
    assert_eq!(overworld_moon_phase(168_000), SkyMoonPhase::WaxingGibbous);
    assert_eq!(overworld_moon_phase(192_000), SkyMoonPhase::FullMoon);
}

#[test]
fn sky_and_cloud_environment_use_dynamic_dimension_attributes() {
    let world = world_with_dynamic_sky_environment();

    let sky = sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert_close4(
        sky.sunrise_sunset_color,
        [
            0x10 as f32 / 255.0,
            0x20 as f32 / 255.0,
            0x30 as f32 / 255.0,
            0x40 as f32 / 255.0,
        ],
    );
    assert!((sky.sun_angle_radians - 30.0_f32.to_radians()).abs() < 1e-6);
    assert!((sky.moon_angle_radians - 60.0_f32.to_radians()).abs() < 1e-6);
    assert!((sky.star_angle_radians - 90.0_f32.to_radians()).abs() < 1e-6);
    assert_eq!(sky.moon_phase, SkyMoonPhase::NewMoon);
    assert_eq!(sky.star_brightness, 0.75);

    let clouds = cloud_environment_for_world(&world);
    assert_close4(
        clouds.color,
        [
            0x50 as f32 / 255.0,
            0x60 as f32 / 255.0,
            0x70 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ],
    );
    assert_eq!(clouds.height, 144.5);
}

#[test]
fn sky_environment_projects_vanilla_star_brightness_with_weather_layers() {
    let day_time = 13_228;
    let mut clear = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut clear, day_time);
    let clear_sky =
        sky_environment_for_world_at_camera(&clear, camera_pose_from_world(&clear), false);

    assert!((clear_sky.star_brightness - 0.5).abs() < 1e-6);

    let mut storm = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut storm, day_time);
    set_world_weather(&mut storm, 0.75, 0.25);
    let storm_sky =
        sky_environment_for_world_at_camera(&storm, camera_pose_from_world(&storm), false);

    let expected = apply_weather_star_brightness_layers(0.5, 0.75, 0.25);
    assert!((storm_sky.star_brightness - expected).abs() < 1e-6);
}

#[test]
fn cloud_environment_projects_vanilla_overworld_defaults_and_dimension_gate() {
    let overworld = world_with_dimension(0, "minecraft:overworld");
    let nether = world_with_dimension(1, "minecraft:the_nether");
    let end = world_with_dimension(2, "minecraft:the_end");

    let clouds = cloud_environment_for_world(&overworld);

    assert_eq!(clouds.color, VANILLA_DEFAULT_CLOUD_COLOR);
    assert_eq!(clouds.height, VANILLA_DEFAULT_CLOUD_HEIGHT);
    assert!(clouds.is_visible());
    assert_eq!(
        cloud_environment_for_world(&nether),
        CloudEnvironment::disabled()
    );
    assert_eq!(
        cloud_environment_for_world(&end),
        CloudEnvironment::disabled()
    );
}

#[test]
fn cloud_environment_applies_overworld_day_timeline_cloud_color() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);

    let clouds = cloud_environment_for_world(&world);

    assert_close4(
        clouds.color,
        [20.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0, 1.0],
    );
    assert_ne!(clouds.color, VANILLA_DEFAULT_CLOUD_COLOR);
}

#[test]
fn cloud_environment_applies_vanilla_weather_cloud_color_layers() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    set_world_weather(&mut world, 1.0, 0.0);
    assert_close4(
        cloud_environment_for_world(&world).color,
        [126.0 / 255.0, 126.0 / 255.0, 126.0 / 255.0, 1.0],
    );

    set_world_weather(&mut world, 1.0, 1.0);
    assert_close4(
        cloud_environment_for_world(&world).color,
        [30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 1.0],
    );
}

#[test]
fn cloud_frame_projects_world_game_time_and_camera_eye_position() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 1234);
    world.set_local_player_pose(local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0));
    let camera_pose = camera_pose_from_world(&world);
    let frame = cloud_frame_for_world(&world, camera_pose, 0.25);

    assert_eq!(frame.camera_position, [10.0, 65.62, -5.0]);
    assert_eq!(frame.game_time, 1234);
    assert_eq!(frame.partial_tick, 0.25);
}

#[test]
fn weather_render_state_projects_overworld_rain_columns_from_world_weather() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    set_world_day_time(&mut world, 96);
    set_world_weather(&mut world, 0.5, 0.0);
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(42, 0.8, true),
        ]));

    let state = weather_render_state_for_world(
        &world,
        &terrain_textures,
        camera_pose_from_world(&world),
        0.25,
    );

    assert_eq!(state.frame.radius, VANILLA_WEATHER_RENDER_RADIUS);
    assert_eq!(state.frame.intensity, 0.5);
    assert_eq!(state.frame.camera_position, [0.5, 5.62, 0.5]);
    assert_eq!(
        state.rain_columns.len() + state.snow_columns.len(),
        (VANILLA_WEATHER_RENDER_RADIUS * 2 + 1).pow(2) as usize
    );
    let center = state
        .rain_columns
        .iter()
        .find(|column| column.x == 0 && column.z == 0)
        .expect("center column uses the loaded precipitating biome");
    assert_eq!(center.bottom_y, -5);
    assert_eq!(center.top_y, 15);
    assert_eq!(center.u_offset, 0.0);
    assert!(center.v_offset.is_finite());
}

#[test]
fn weather_render_state_uses_motion_blocking_heightmap_for_column_bounds() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    set_world_day_time(&mut world, 96);
    set_world_weather(&mut world, 1.0, 0.0);
    let mut chunk = empty_lightmap_test_chunk_with_biome(world.dimension(), 42);
    chunk.heightmaps = vec![test_motion_blocking_heightmap(
        world.dimension(),
        &[(0, 0, 8)],
    )];
    world.insert_decoded_chunk(chunk);
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(42, 0.8, true),
        ]));

    let state = weather_render_state_for_world(
        &world,
        &terrain_textures,
        camera_pose_from_world(&world),
        0.25,
    );

    let center = state
        .rain_columns
        .iter()
        .find(|column| column.x == 0 && column.z == 0)
        .expect("center column uses the loaded precipitating biome");
    assert_eq!(center.bottom_y, 8);
    assert_eq!(center.top_y, 15);
}

#[test]
fn weather_precipitation_uses_cold_biome_for_snow_and_brightens_snow_light() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 7));
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(7, 0.0, true),
        ]));

    assert_eq!(
        weather_precipitation_at(&world, &terrain_textures, BlockPos { x: 0, y: 4, z: 0 }, 63,),
        Some(WeatherPrecipitation::Snow)
    );

    let column = snow_weather_column(0, 0, 1, 6, TerrainLight { block: 1, sky: 7 }, 96, 0.25);
    assert_eq!(column.light, [4.0 / 15.0, 9.0 / 15.0]);
    assert!(column.u_offset.is_finite());
    assert!(column.v_offset.is_finite());
}

#[test]
fn weather_precipitation_uses_temperature_noise_and_frozen_modifier() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 7));
    let terrain_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_weather(7, 0.149, true),
        ]));

    assert_eq!(
        weather_precipitation_at(
            &world,
            &terrain_textures,
            BlockPos {
                x: -512,
                y: 81,
                z: -511,
            },
            63,
        ),
        Some(WeatherPrecipitation::Rain)
    );

    let frozen_textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
            biome_profile_with_temperature_modifier(7, 0.0, BiomeTemperatureModifier::Frozen),
        ]));
    assert_eq!(
        weather_precipitation_at(&world, &frozen_textures, BlockPos { x: 0, y: 64, z: 0 }, 63,),
        Some(WeatherPrecipitation::Rain)
    );
}

#[test]
fn weather_render_state_is_empty_without_rain_or_weather_dimension() {
    let mut dry = world_with_dimension(0, "minecraft:overworld");
    dry.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    let terrain_textures = TerrainTextureState::default();
    assert!(weather_render_state_for_world(
        &dry,
        &terrain_textures,
        camera_pose_from_world(&dry),
        0.0,
    )
    .is_empty());

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    nether.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    set_world_weather(&mut nether, 1.0, 0.0);
    assert!(weather_render_state_for_world(
        &nether,
        &terrain_textures,
        camera_pose_from_world(&nether),
        0.0,
    )
    .is_empty());
}

#[test]
fn weather_render_state_projects_lightning_bolts_without_rain() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    let mut lightning = test_add_entity(77, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID);
    lightning.uuid = uuid::Uuid::from_u128(0x1234_5678_9abc_def0);
    lightning.position = bbb_protocol::packets::Vec3d {
        x: 8.0,
        y: 65.0,
        z: -3.0,
    };
    world.apply_add_entity(lightning);

    let state = weather_render_state_for_world(
        &world,
        &TerrainTextureState::default(),
        camera_pose_from_world(&world),
        0.0,
    );

    assert!(!state.is_empty());
    assert_eq!(state.rain_column_count(), 0);
    assert_eq!(state.snow_column_count(), 0);
    assert_eq!(state.lightning_bolt_count(), 1);
    assert_eq!(state.lightning_bolts[0].position, [8.0, 65.0, -3.0]);
    assert_eq!(
        state.lightning_bolts[0].seed,
        lightning_bolt_seed(uuid::Uuid::from_u128(0x1234_5678_9abc_def0))
    );
}

fn apply_static_border(world: &mut WorldStore, center: (f64, f64), size: f64) {
    world.apply_initialize_border(ProtocolInitializeBorder {
        new_center_x: center.0,
        new_center_z: center.1,
        old_size: size,
        new_size: size,
        lerp_time: 0,
        new_absolute_max_size: 29_999_984,
        warning_blocks: 5,
        warning_time: 15,
    });
}

#[test]
fn world_border_render_state_matches_vanilla_extract_alpha_and_tint() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_static_border(&mut world, (0.0, 0.0), 64.0);
    // Camera eye at x = 24: 8 blocks from the east border edge (+32).
    world.set_local_player_pose(local_player_pose([24.0, 64.0, 0.0], 0.0, 0.0));

    let state =
        world_border_render_state_for_world(&world, camera_pose_from_world(&world), 2, 0.0, 4_500);

    assert_eq!(state.min_x, -32.0);
    assert_eq!(state.max_x, 32.0);
    assert_eq!(state.min_z, -32.0);
    assert_eq!(state.max_z, 32.0);
    // renderDistance = renderDistanceChunks * 16 (LevelRenderer.java:583,744).
    assert_eq!(state.render_distance, 32.0);
    // alpha = clamp((1 - distanceToBorder / renderDistance)^4, 0, 1)
    // (WorldBorderRenderer.java:117-119) = (1 - 8/32)^4.
    assert_eq!(state.alpha, 0.75_f64.powi(4));
    // Stationary border tint (BorderStatus.java:6).
    assert_eq!(state.tint, 2_138_367);
    // depthFar = max(renderDistanceBlocks * 4, 128 chunks * 16)
    // (Camera.java:91-92, Options.java:166-171) = max(128, 2048).
    assert_eq!(state.depth_far, 2_048.0);
    // offset = (millis % 3000) / 3000 (WorldBorderRenderer.java:134).
    assert_eq!(state.texture_offset, 0.5);
    assert_eq!(state.camera_position[0], 24.0);
    assert_eq!(state.camera_position[1], f64::from(64.0_f32 + 1.62_f32));
    assert_eq!(state.camera_position[2], 0.0);
}

#[test]
fn world_border_render_state_is_invisible_away_from_the_border() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_static_border(&mut world, (0.0, 0.0), 400.0);

    // Camera deep inside the border (further than renderDistance from every
    // edge): the first extract clause fails (WorldBorderRenderer.java:107-112).
    world.set_local_player_pose(local_player_pose([0.0, 64.0, 0.0], 0.0, 0.0));
    let inside =
        world_border_render_state_for_world(&world, camera_pose_from_world(&world), 2, 0.0, 0);
    assert_eq!(inside.alpha, 0.0);

    // Camera further than renderDistance outside the border: the second
    // clause fails (WorldBorderRenderer.java:113-116).
    world.set_local_player_pose(local_player_pose([250.0, 64.0, 0.0], 0.0, 0.0));
    let outside =
        world_border_render_state_for_world(&world, camera_pose_from_world(&world), 2, 0.0, 0);
    assert_eq!(outside.alpha, 0.0);

    // No camera pose: nothing to extract.
    assert_eq!(
        world_border_render_state_for_world(&world, None, 2, 0.0, 0).alpha,
        0.0
    );
}

#[test]
fn world_border_render_state_interpolates_lerping_bounds_and_uses_shrinking_tint() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_static_border(&mut world, (0.0, 0.0), 100.0);
    world.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
        old_size: 100.0,
        new_size: 50.0,
        lerp_time: 10,
    });
    // One border tick: previousSize = 100, size = lerp(1/10, 100, 50) = 95
    // (WorldBorder.java:397-400,431-441).
    world.advance_world_border(1);
    world.set_local_player_pose(local_player_pose([45.0, 64.0, 0.0], 0.0, 0.0));

    let state =
        world_border_render_state_for_world(&world, camera_pose_from_world(&world), 2, 0.5, 0);

    // Bounds interpolate previousSize -> size at the frame partial tick
    // (WorldBorder.java:353-386): lerp(0.5, 100, 95) / 2 = 48.75.
    assert_eq!(state.min_x, -48.75);
    assert_eq!(state.max_x, 48.75);
    // Shrinking border tint (BorderStatus.java:5).
    assert_eq!(state.tint, 16_724_016);
    // getDistanceToBorder uses the partial-tick-0 bounds (WorldBorder.java:104-112):
    // east distance = lerp(0, 100, 95) / 2 - 45 = 5.
    assert_eq!(state.alpha, (1.0 - 5.0 / 32.0_f64).powi(4));
}

#[test]
fn renderer_frame_world_border_extracts_after_border_tick_and_weather() {
    let source = include_str!("../runtime.rs");
    let client_level_tick = source
        .find("let running_ticks = client_animation_ticks.last_running_level_ticks;")
        .expect("pump should complete the ordered client-level tick batch");
    let weather_extract = source
        .find("let weather_render_state =")
        .expect("pump should extract the weather render state");
    let border_extract = source
        .find("let world_border_render_state = world_border_render_state_for_world(")
        .expect("pump should extract the world border render state");

    // Vanilla LevelRenderer extraction runs worldBorderRenderer.extract in the
    // "border" profiler section after the weather extraction
    // (LevelRenderer.java:573-585).
    assert!(
        client_level_tick < border_extract && weather_extract < border_extract,
        "world border render state extracts after the client tick and weather extraction"
    );
}

#[test]
fn clear_color_mixes_sunrise_sunset_color_when_camera_faces_sun() {
    let day_time = 71;
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, day_time);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 270.0, 0.0));

    let clear = clear_color_for_world_at_camera_with_render_distance(
        &world,
        camera_pose_from_world(&world),
        12,
        false,
    );

    let fog_color = argb_multiply(
        rgb_u8_to_argb(VANILLA_OVERWORLD_FOG_COLOR),
        sample_periodic_argb_keyframes(
            day_time,
            &VANILLA_OVERWORLD_FOG_COLOR_MULTIPLIER_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
    );
    let sky_color = argb_multiply(
        rgb_u8_to_argb(VANILLA_OVERWORLD_SKY_COLOR),
        sample_periodic_argb_keyframes(
            day_time,
            &VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
    );
    let sunrise_color = sample_periodic_argb_keyframes(
        day_time,
        &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
    );
    let looking_at_sun = camera_forward_vector(camera_pose_from_world(&world).unwrap())[0];
    let expected = atmospheric_clear_color(
        argb_srgb_lerp(
            looking_at_sun * argb_alpha(sunrise_color) as f32 / 255.0,
            fog_color,
            argb_opaque(sunrise_color),
        ),
        sky_color,
        12,
    );
    let baseline = clear_color_for_day_time(day_time, 0.0, 0.0);

    assert_clear_color_close(clear, expected);
    assert_ne!(clear, baseline);
}

#[test]
fn clear_color_skips_sunrise_sunset_when_not_facing_sun_or_render_distance_is_low() {
    let day_time = 71;
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, day_time);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 90.0, 0.0));

    let away = clear_color_for_world_at_camera_with_render_distance(
        &world,
        camera_pose_from_world(&world),
        12,
        false,
    );
    let away_baseline = clear_color_for_day_time(day_time, 0.0, 0.0);

    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 270.0, 0.0));
    let low_render_distance = clear_color_for_world_at_camera_with_render_distance(
        &world,
        camera_pose_from_world(&world),
        3,
        false,
    );
    let low_render_distance_baseline = clear_color_for_day_time_with_environment_colors_and_camera(
        day_time,
        0.0,
        0.0,
        Some(VANILLA_OVERWORLD_FOG_COLOR),
        Some(VANILLA_OVERWORLD_SKY_COLOR),
        VanillaLightmapDimensionKind::Overworld,
        None,
        3,
    );

    assert_clear_color_close(away, away_baseline);
    assert_clear_color_close(low_render_distance, low_render_distance_baseline);
}

#[test]
fn clear_color_samples_camera_biome_water_fog_when_eye_is_in_water() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/water_fog_color",
                TestEnvironmentAttributeValue::String("#022044"),
            )],
        )],
    );

    let clear = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);

    assert_eq!(
        clear,
        clear_color_from_argb(rgb_u8_to_argb([0x02, 0x20, 0x44]))
    );

    world.set_sky_flash_time(2);
    let flashed = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert_eq!(flashed, clear);
}

#[test]
fn clear_color_brightens_water_fog_with_vanilla_water_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/water_fog_color",
                TestEnvironmentAttributeValue::String("#022044"),
            )],
        )],
    );

    let clear = clear_color_for_world_at_camera_with_water_vision(
        &world,
        camera_pose_from_world(&world),
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        0.5,
        false,
    );

    assert_eq!(
        clear,
        clear_color_from_argb(rgb_u8_to_argb([0x04, 0x4b, 0xa1]))
    );
}

#[test]
fn fog_environment_uses_vanilla_render_distance_range_and_dimension_fog_distances() {
    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut overworld, 6_000);
    let fog = fog_environment_for_world_at_camera(
        &overworld,
        camera_pose_from_world(&overworld),
        12,
        0.0,
        0.0,
        false,
    );
    assert_fog_environment_close(
        fog,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_for_world(&overworld, false)),
            VANILLA_DEFAULT_FOG_START_DISTANCE,
            VANILLA_DEFAULT_FOG_END_DISTANCE,
            12,
            VANILLA_DEFAULT_SKY_FOG_END_DISTANCE.min(12.0 * 16.0),
            VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE,
        ),
    );

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    set_world_day_time(&mut nether, 6_000);
    let fog = fog_environment_for_world_at_camera(
        &nether,
        camera_pose_from_world(&nether),
        20,
        0.0,
        0.0,
        false,
    );
    assert_eq!(fog.environmental_start, VANILLA_NETHER_FOG_START_DISTANCE);
    assert_eq!(fog.environmental_end, VANILLA_NETHER_FOG_END_DISTANCE);
    assert_eq!(fog.render_distance_start, 288.0);
    assert_eq!(fog.render_distance_end, 320.0);
    assert_eq!(fog.sky_end, 320.0);
    assert_eq!(fog.cloud_end, VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE);
}

#[test]
fn fog_environment_applies_vanilla_rain_fog_distance_multiplier() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.5,
        false,
    );

    assert_eq!(fog.environmental_start, -80.0);
    assert_eq!(fog.environmental_end, 896.0);
    assert_eq!(
        fog.sky_end,
        VANILLA_DEFAULT_SKY_FOG_END_DISTANCE.min(12.0 * 16.0)
    );
    assert_eq!(fog.cloud_end, VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE);
}

#[test]
fn fog_environment_clamps_atmospheric_distance_for_boss_world_fog() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    add_boss_bar(&mut world, false, true);

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );

    assert_eq!(fog.environmental_start, VANILLA_DEFAULT_FOG_START_DISTANCE);
    assert_eq!(fog.environmental_end, VANILLA_NETHER_FOG_END_DISTANCE);
    assert_eq!(fog.sky_end, VANILLA_NETHER_FOG_END_DISTANCE);
    assert_eq!(fog.cloud_end, VANILLA_NETHER_FOG_END_DISTANCE);
}

#[test]
fn fog_environment_uses_water_fog_distances_when_eye_is_in_water() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/water_fog_color",
                TestEnvironmentAttributeValue::String("#022044"),
            )],
        )],
    );

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.5,
        0.0,
        false,
    );

    assert_fog_environment_close(
        fog,
        FogEnvironment::world_with_visibility_ends(
            clear_color_to_fog_color(clear_color_from_argb(rgb_u8_to_argb([0x04, 0x4b, 0xa1]))),
            VANILLA_DEFAULT_WATER_FOG_START_DISTANCE,
            VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.5,
            12,
            VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.5,
            VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.5,
        ),
    );
}

#[test]
fn fog_environment_applies_biome_water_fog_end_distance_modifier() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    insert_uniform_biome_neighbors(&mut world, 42);
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    install_biome_environment_profiles(
        &mut world,
        &[(
            42,
            &[(
                "minecraft:visual/water_fog_end_distance",
                TestEnvironmentAttributeValue::Multiply(0.85),
            )],
        )],
    );

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.5,
        0.0,
        false,
    );
    let expected_end = VANILLA_DEFAULT_WATER_FOG_END_DISTANCE * 0.85 * 0.5;

    assert!((fog.environmental_start - VANILLA_DEFAULT_WATER_FOG_START_DISTANCE).abs() < 1e-6);
    assert!((fog.environmental_end - expected_end).abs() < 1e-5);
    assert!((fog.sky_end - expected_end).abs() < 1e-5);
    assert!((fog.cloud_end - expected_end).abs() < 1e-5);
}

#[test]
fn blindness_overrides_atmospheric_fog_and_darkens_its_color() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    attach_lightmap_local_player(&mut world, 42);
    let baseline = clear_color_for_world(&world, false);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, 10,)));

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );

    // lerp(10 / 20, 12 * 16, 5) = 98.5.
    assert_eq!(fog.environmental_start, 24.625);
    assert_eq!(fog.environmental_end, 98.5);
    assert!((fog.sky_end - 78.8).abs() < 1e-5);
    assert!((fog.cloud_end - 78.8).abs() < 1e-5);
    // Blindness duration 10 contributes darkness 0.5, then FogRenderer
    // multiplies the base color by square(1 - darkness) = 0.25.
    assert_eq!(
        fog.color[0],
        ((baseline.r * 255.0) as i32 / 4) as f32 / 255.0
    );
    assert_eq!(
        fog.color[1],
        ((baseline.g * 255.0) as i32 / 4) as f32 / 255.0
    );
    assert_eq!(
        fog.color[2],
        ((baseline.b * 255.0) as i32 / 4) as f32 / 255.0
    );
}

#[test]
fn blindness_takes_fog_precedence_over_darkness_and_blocks_sky() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_DARKNESS_ID, 260,)));
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1,)));

    let fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );

    assert_eq!(fog.environmental_start, 1.25);
    assert_eq!(fog.environmental_end, 5.0);
    assert_eq!(fog.sky_end, 4.0);
    assert_eq!(fog.cloud_end, 4.0);
    assert_eq!(
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn darkness_fog_uses_canonical_blend_factor() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_DARKNESS_ID,
        260,
        true,
    )));

    let initial = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_eq!(initial.environmental_end, 192.0);

    world.advance_client_level_entity_ticks_with_observation(11);
    let blended = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    let expected_end = 192.0 + 0.5 * (15.0 - 192.0);
    assert!((blended.environmental_start - expected_end * 0.75).abs() < 1e-5);
    assert!((blended.environmental_end - expected_end).abs() < 1e-5);
    assert!((blended.sky_end - expected_end).abs() < 1e-5);
    assert!((blended.cloud_end - expected_end).abs() < 1e-5);
}

#[test]
fn camera_effect_fog_and_sky_follow_the_active_camera_entity() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.apply_add_entity(test_add_entity(100, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.apply_update_mob_effect(mob_effect(100, VANILLA_MOB_EFFECT_BLINDNESS_ID, -1,)));

    let local_fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_ne!(local_fog.environmental_end, 5.0);

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 100 }));
    let remote_fog = fog_environment_for_world_at_camera(
        &world,
        camera_pose_from_world(&world),
        12,
        0.0,
        0.0,
        false,
    );
    assert_eq!(remote_fog.environmental_end, 5.0);
    assert_eq!(
        sky_environment_for_world_at_camera(&world, camera_pose_from_world(&world), false),
        SkyEnvironment::disabled()
    );
}

#[test]
fn night_vision_brightens_atmospheric_fog_unless_darkness_is_present() {
    let mut world = world_with_custom_dimension_environment();
    attach_lightmap_local_player(&mut world, 42);
    let baseline = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 240,)));

    let night_vision =
        clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert!(night_vision.r >= baseline.r);
    assert!(night_vision.g >= baseline.g);
    assert!(night_vision.b >= baseline.b);
    assert_ne!(night_vision, baseline);
    assert_eq!(night_vision.r.max(night_vision.g).max(night_vision.b), 1.0);

    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_DARKNESS_ID,
        260,
        true,
    )));
    let darkness = clear_color_for_world_at_camera(&world, camera_pose_from_world(&world), false);
    assert_clear_color_close(darkness, baseline);
}

#[test]
fn camera_biome_sky_color_uses_vanilla_gaussian_spatial_weights() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose(
        [8.0, 32.0 - f64::from(CameraPose::STANDING_EYE_HEIGHT), 8.0],
        0.0,
        0.0,
    ));
    insert_split_x_biome_neighborhood(&mut world, 10, 20);
    install_biome_environment_profiles(
        &mut world,
        &[
            (
                10,
                &[(
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#000000"),
                )],
            ),
            (
                20,
                &[(
                    "minecraft:visual/sky_color",
                    TestEnvironmentAttributeValue::String("#64c8fa"),
                )],
            ),
        ],
    );
    assert_eq!(
        world
            .sampled_client_environment_attributes_at(
                [0.0, 32.0, 8.0],
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap()
            .sky_color,
        Some([0, 0, 0])
    );
    assert_eq!(
        world
            .sampled_client_environment_attributes_at(
                [12.0, 32.0, 8.0],
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap()
            .sky_color,
        Some([100, 200, 250])
    );

    let sky_color = camera_environment_colors(&world, camera_pose_from_world(&world))
        .sky_color
        .unwrap();

    assert_eq!(sky_color, [50, 100, 125]);
}

#[test]
fn camera_biome_fog_and_water_fog_use_vanilla_gaussian_spatial_weights() {
    let mut world = world_with_custom_dimension_environment();
    world.set_local_player_pose(local_player_pose(
        [8.0, 32.0 - f64::from(CameraPose::STANDING_EYE_HEIGHT), 8.0],
        0.0,
        0.0,
    ));
    insert_split_x_biome_neighborhood(&mut world, 10, 20);
    install_biome_environment_profiles(
        &mut world,
        &[
            (
                10,
                &[
                    (
                        "minecraft:visual/fog_color",
                        TestEnvironmentAttributeValue::String("#0a141e"),
                    ),
                    (
                        "minecraft:visual/water_fog_color",
                        TestEnvironmentAttributeValue::String("#010203"),
                    ),
                ],
            ),
            (
                20,
                &[
                    (
                        "minecraft:visual/fog_color",
                        TestEnvironmentAttributeValue::String("#6edce6"),
                    ),
                    (
                        "minecraft:visual/water_fog_color",
                        TestEnvironmentAttributeValue::String("#65cacb"),
                    ),
                ],
            ),
        ],
    );

    let camera_pose = camera_pose_from_world(&world);
    let colors = camera_environment_colors(&world, camera_pose);
    let fog_color = colors.fog_color.unwrap();
    let water_fog_color = colors.water_fog_color.unwrap();

    assert_eq!(fog_color, [60, 120, 130]);
    assert_eq!(water_fog_color, [51, 102, 103]);
}

#[test]
fn lightmap_tick_state_applies_end_flash_sky_factor_from_end_clock() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_end_clock_time(&mut world, 1_486);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - 1.0).abs() < 1e-6);
    assert_close3(
        environment.sky_light_color,
        [172.0 / 255.0, 96.0 / 255.0, 205.0 / 255.0],
    );
    assert_close3(
        environment.ambient_color,
        [63.0 / 255.0, 71.0 / 255.0, 63.0 / 255.0],
    );
}

#[test]
fn lightmap_tick_state_uses_locally_advanced_end_clock() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    world.apply_world_time(PlayTime {
        game_time: 100,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 1,
            total_ticks: 1_485,
            partial_tick: 0.75,
            rate: 0.5,
        }],
    });
    world.advance_client_time(1);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - 1.0).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_does_not_use_overworld_clock_for_end_flash() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_day_time(&mut world, 1_486);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.sky_factor, 0.0);
}

#[test]
fn lightmap_tick_state_hides_end_flash_when_option_is_enabled() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_end_clock_time(&mut world, 1_486);
    let mut lightmap =
        LightmapTickState::with_brightness_factor_and_hide_lightning_flash(0.5, true);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.sky_factor, 0.0);
    assert_close3(
        environment.sky_light_color,
        [172.0 / 255.0, 96.0 / 255.0, 205.0 / 255.0],
    );
}

#[test]
fn lightmap_tick_state_divides_end_flash_sky_factor_for_boss_world_fog() {
    let mut world = world_with_dimension(2, "minecraft:the_end");
    set_world_end_clock_time(&mut world, 1_486);
    add_boss_bar(&mut world, false, true);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);

    assert!((environment.sky_factor - (1.0 / 3.0)).abs() < 1e-6);
    assert_eq!(environment.boss_overlay_world_darkening, 0.0);
}

#[test]
fn lightmap_tick_state_projects_boss_overlay_world_darkening() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    add_boss_bar(&mut world, true, false);
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.5);

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.boss_overlay_world_darkening - 0.05).abs() < 1e-6);

    lightmap.advance_for_world(19, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.boss_overlay_world_darkening, 1.0);

    clear_boss_overlay_properties(&mut world);
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.boss_overlay_world_darkening - 0.9875).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_environment_preserves_gamma_and_flicker() {
    let world = world_with_dimension(1, "minecraft:the_nether");
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);
    let block_factor = lightmap.advance(1);
    let environment = lightmap.environment_for_world(&world);

    assert_eq!(environment.brightness_factor, 0.8);
    assert!((environment.block_factor - block_factor).abs() < 1e-6);
    assert_eq!(environment.sky_factor, 0.0);
    assert_close3(
        environment.ambient_color,
        [48.0 / 255.0, 40.0 / 255.0, 33.0 / 255.0],
    );
}

#[test]
fn lightmap_environment_projects_local_player_night_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 240,)));

    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.7,
        1.4,
        0,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );
    assert_eq!(environment.night_vision_factor, 1.0);
    assert_eq!(environment.brightness_factor, 0.7);

    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 40,)));
    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.7,
        1.4,
        0,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );
    let expected = 0.7
        + ((40.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.2).sin() * 0.3;
    assert!((environment.night_vision_factor - expected).abs() < 1e-6);
}

#[test]
fn lightmap_environment_projects_local_player_darkness() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_DARKNESS_ID, 260,)));

    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.8,
        1.4,
        10,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );

    assert_eq!(environment.brightness_factor, 0.0);
    let expected_darkness =
        (((10.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.025).cos()
            * 0.45)
            .max(0.0);
    assert!((environment.darkness_scale - expected_darkness).abs() < 1e-6);
    assert_eq!(environment.night_vision_factor, 0.0);
}

#[test]
fn lightmap_tick_state_blends_local_player_darkness_like_vanilla() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect_with_blend(
        42,
        VANILLA_MOB_EFFECT_DARKNESS_ID,
        260,
        true,
    )));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.8);
    assert_eq!(environment.darkness_scale, 0.0);

    world.advance_client_level_entity_ticks_with_observation(1);
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    let first_tick_factor = 1.0 / 22.0;
    assert!((environment.brightness_factor - (0.8 - first_tick_factor)).abs() < 1e-6);
    assert!((environment.darkness_scale - 0.45 * first_tick_factor).abs() < 1e-6);

    world.advance_client_level_entity_ticks_with_observation(21);
    lightmap.advance_for_world(21, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.0);
    let expected_darkness =
        (((22.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.025).cos()
            * 0.45)
            .max(0.0);
    assert!((environment.darkness_scale - expected_darkness).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_applies_and_removes_non_blending_darkness_immediately() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_DARKNESS_ID, 260,)));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.0);
    assert!(environment.darkness_scale > 0.0);

    assert!(world.apply_remove_mob_effect(RemoveMobEffect {
        entity_id: 42,
        effect_id: VANILLA_MOB_EFFECT_DARKNESS_ID,
    }));
    lightmap.advance_for_world(0, &world);
    let environment = lightmap.environment_for_world(&world);
    assert_eq!(environment.brightness_factor, 0.8);
    assert_eq!(environment.darkness_scale, 0.0);
}

#[test]
fn lightmap_tick_state_advances_night_vision_duration_before_sampling() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 201,)));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    assert_eq!(
        lightmap.environment_for_world(&world).night_vision_factor,
        1.0
    );

    world.advance_client_level_entity_ticks_with_observation(1);
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    let expected = 0.7
        + ((200.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.2).sin() * 0.3;
    assert!((environment.night_vision_factor - expected).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_uses_conduit_water_vision_without_night_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    assert!(world.apply_update_mob_effect(mob_effect(
        42,
        VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        260,
    )));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    lightmap.advance_for_world(0, &world);
    assert_eq!(
        lightmap.environment_for_world(&world).night_vision_factor,
        0.0
    );

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - 0.006).abs() < 1e-6);

    lightmap.advance_for_world(99, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - 0.6).abs() < 1e-6);
}

#[test]
fn lightmap_tick_state_prefers_night_vision_over_conduit_water_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    assert!(world.apply_update_mob_effect(mob_effect(
        42,
        VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        260,
    )));
    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 240,)));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);

    world.advance_client_level_entity_ticks_with_observation(100);
    lightmap.advance_for_world(100, &world);
    let expected = 0.7
        + ((140.0 - VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK) * std::f32::consts::PI * 0.2).sin() * 0.3;
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - expected).abs() < 1e-6);
    assert!((environment.night_vision_factor - 0.6).abs() > 0.01);
}

#[test]
fn lightmap_tick_state_water_vision_fades_out_when_eye_leaves_water() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    assert!(world.apply_update_mob_effect(mob_effect(
        42,
        VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        260,
    )));
    let mut lightmap = LightmapTickState::with_seed_and_brightness(0, 0.8);
    lightmap.advance_for_world(100, &world);
    assert!((lightmap.environment_for_world(&world).night_vision_factor - 0.6).abs() < 1e-6);

    world.set_local_player_pose(local_player_pose([0.5, 4.0, 0.5], 0.0, 0.0));
    lightmap.advance_for_world(1, &world);
    assert_eq!(
        lightmap.environment_for_world(&world).night_vision_factor,
        0.0
    );

    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    assert!((environment.night_vision_factor - 0.546).abs() < 1e-6);
}

#[test]
fn lightmap_environment_without_local_player_effects_keeps_base_factors() {
    let world = world_with_dimension(0, "minecraft:overworld");
    let environment = lightmap_environment_for_world_at_tick(
        &world,
        0.8,
        1.35,
        10,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    );

    assert_eq!(environment.brightness_factor, 0.8);
    assert_eq!(environment.block_factor, 1.35);
    assert_eq!(environment.darkness_scale, 0.0);
    assert_eq!(environment.night_vision_factor, 0.0);
}

#[test]
fn server_container_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput {
            forward: true,
            ..bbb_protocol::packets::PlayerInput::default()
        })
    );

    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

fn world_with_dimension(dimension_type_id: i32, dimension: &str) -> WorldStore {
    world_with_dimension_height(dimension_type_id, dimension, 16)
}

fn world_with_custom_dimension_environment() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension::default());
    world.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:custom",
            custom_dimension_environment_nbt(),
        )],
    );
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["example:level".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "example:level".to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    set_world_day_time(&mut world, 6_000);
    world
}

fn world_with_dynamic_sky_environment() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension::default());
    world.record_registry_entries(
        bbb_world::DIMENSION_TYPE_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:custom_sky",
            dynamic_sky_dimension_environment_nbt(),
        )],
    );
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    world
}

fn world_with_custom_dimension_timeline() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension::default());
    world.record_registry_entries(
        "minecraft:world_clock",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:clock",
            vec![10, 0],
        )],
    );
    world.record_registry_entries(
        "minecraft:timeline",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:lightmap",
            custom_lightmap_timeline_nbt(),
        )],
    );
    let mut dimension = custom_dimension_environment_nbt();
    assert_eq!(dimension.pop(), Some(0));
    dimension.push(8);
    test_nbt_utf(&mut dimension, "timelines");
    test_nbt_utf(&mut dimension, "example:lightmap");
    dimension.push(0);
    world.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:custom",
            dimension,
        )],
    );
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["example:level".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "example:level".to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    set_world_day_time(&mut world, 50);
    world
}

fn custom_dimension_environment_nbt() -> Vec<u8> {
    let mut out = vec![10, 10];
    test_nbt_utf(&mut out, "attributes");
    for (name, value) in [
        ("minecraft:visual/fog_color", "#123456"),
        ("minecraft:visual/sky_color", "#345678"),
        ("minecraft:visual/water_fog_color", "#56789a"),
        ("minecraft:visual/block_light_tint", "#204060"),
        ("minecraft:visual/sky_light_color", "#406080"),
        ("minecraft:visual/night_vision_color", "#507090"),
        ("minecraft:visual/ambient_light_color", "#305070"),
    ] {
        out.push(8);
        test_nbt_utf(&mut out, name);
        test_nbt_utf(&mut out, value);
    }
    for (name, value) in [
        ("minecraft:visual/fog_start_distance", 12.0_f32),
        ("minecraft:visual/fog_end_distance", 144.0),
        ("minecraft:visual/sky_fog_end_distance", 96.0),
        ("minecraft:visual/cloud_fog_end_distance", 192.0),
        ("minecraft:visual/water_fog_start_distance", -4.0),
        ("minecraft:visual/water_fog_end_distance", 48.0),
        ("minecraft:visual/sky_light_factor", 0.25),
    ] {
        out.push(5);
        test_nbt_utf(&mut out, name);
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    out.push(0);
    out.push(0);
    out
}

fn dynamic_sky_dimension_environment_nbt() -> Vec<u8> {
    let mut out = vec![10, 10];
    test_nbt_utf(&mut out, "attributes");
    for (name, value) in [
        ("minecraft:visual/sunrise_sunset_color", "#40102030"),
        ("minecraft:visual/cloud_color", "#80506070"),
        ("minecraft:visual/moon_phase", "new_moon"),
    ] {
        out.push(8);
        test_nbt_utf(&mut out, name);
        test_nbt_utf(&mut out, value);
    }
    for (name, value) in [
        ("minecraft:visual/cloud_height", 144.5_f32),
        ("minecraft:visual/sun_angle", 30.0),
        ("minecraft:visual/moon_angle", 60.0),
        ("minecraft:visual/star_angle", 90.0),
        ("minecraft:visual/star_brightness", 0.75),
    ] {
        out.push(5);
        test_nbt_utf(&mut out, name);
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    out.push(0);
    out.push(0);
    out
}

fn custom_lightmap_timeline_nbt() -> Vec<u8> {
    let mut out = vec![10];
    out.push(8);
    test_nbt_utf(&mut out, "clock");
    test_nbt_utf(&mut out, "example:clock");
    out.push(10);
    test_nbt_utf(&mut out, "tracks");
    out.push(10);
    test_nbt_utf(&mut out, "minecraft:visual/sky_light_factor");
    out.push(8);
    test_nbt_utf(&mut out, "modifier");
    test_nbt_utf(&mut out, "multiply");
    out.push(9);
    test_nbt_utf(&mut out, "keyframes");
    out.push(10);
    out.extend_from_slice(&1_i32.to_be_bytes());
    out.push(3);
    test_nbt_utf(&mut out, "ticks");
    out.extend_from_slice(&0_i32.to_be_bytes());
    out.push(5);
    test_nbt_utf(&mut out, "value");
    out.extend_from_slice(&0.5_f32.to_bits().to_be_bytes());
    out.push(0);
    out.push(0);
    out.push(0);
    out.push(0);
    out
}

fn test_nbt_utf(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn test_nbt_byte(out: &mut Vec<u8>, name: &str, value: i8) {
    out.push(1);
    test_nbt_utf(out, name);
    out.push(value as u8);
}

fn test_nbt_float(out: &mut Vec<u8>, name: &str, value: f32) {
    out.push(5);
    test_nbt_utf(out, name);
    out.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn test_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    test_nbt_utf(out, name);
    test_nbt_utf(out, value);
}

fn world_with_dimension_last_death_location(
    dimension_type_id: i32,
    dimension: &str,
    last_death_location: Option<(&str, [i32; 3])>,
) -> WorldStore {
    let mut world = world_with_dimension_height(dimension_type_id, dimension, 16);
    let level = world
        .level_info()
        .expect("test world has level info")
        .clone();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec![
            "minecraft:overworld".to_string(),
            "minecraft:the_nether".to_string(),
            "minecraft:the_end".to_string(),
        ],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: level.dimension_type_id,
            dimension: level.dimension,
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: level.is_debug,
            is_flat: level.is_flat,
            last_death_location: last_death_location.map(|(dimension, pos)| {
                bbb_protocol::packets::GlobalPos {
                    dimension: dimension.to_string(),
                    pos: ProtocolBlockPos {
                        x: pos[0],
                        y: pos[1],
                        z: pos[2],
                    },
                }
            }),
            portal_cooldown: 0,
            sea_level: level.sea_level,
        },
        enforces_secure_chat: true,
    });
    world
}

fn world_with_dimension_height(dimension_type_id: i32, dimension: &str, height: i32) -> WorldStore {
    world_with_dimension_height_and_reduced_debug_info(dimension_type_id, dimension, height, false)
}

fn world_with_dimension_height_and_reduced_debug_info(
    dimension_type_id: i32,
    dimension: &str,
    height: i32,
    reduced_debug_info: bool,
) -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension { min_y: 0, height });
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec![
            "minecraft:overworld".to_string(),
            "minecraft:the_nether".to_string(),
            "minecraft:the_end".to_string(),
        ],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id,
            dimension: dimension.to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    world
}

fn set_world_day_time(world: &mut WorldStore, day_time: i64) {
    world.apply_world_time(PlayTime {
        game_time: day_time,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 0,
            total_ticks: day_time,
            partial_tick: 0.0,
            rate: 1.0,
        }],
    });
}

fn set_default_spawn(world: &mut WorldStore, dimension: &str, pos: [i32; 3]) {
    world.apply_default_spawn_position(bbb_protocol::packets::SetDefaultSpawnPosition {
        dimension: dimension.to_string(),
        pos: ProtocolBlockPos {
            x: pos[0],
            y: pos[1],
            z: pos[2],
        },
        yaw: 0.0,
        pitch: 0.0,
    });
}

fn set_world_end_clock_time(world: &mut WorldStore, total_ticks: i64) {
    world.apply_world_time(PlayTime {
        game_time: total_ticks,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 1,
            total_ticks,
            partial_tick: 0.0,
            rate: 1.0,
        }],
    });
}

fn set_world_weather(world: &mut WorldStore, rain_level: f32, thunder_level: f32) {
    world.apply_game_event(ProtocolGameEvent {
        event_id: 7,
        param: rain_level,
    });
    world.apply_game_event(ProtocolGameEvent {
        event_id: 8,
        param: thunder_level,
    });
}

fn add_boss_bar(world: &mut WorldStore, darken_screen: bool, create_world_fog: bool) {
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id: uuid::Uuid::from_u128(1),
        operation: ProtocolBossEventOperation::Add {
            name: Box::new(bbb_protocol::RichTextComponent::plain("Boss")),
            progress: 1.0,
            color: BossBarColor::Purple,
            overlay: BossBarOverlay::Progress,
            flags: ProtocolBossEventFlags {
                darken_screen,
                play_music: false,
                create_world_fog,
            },
        },
    }));
}

fn styled_boss_component(text: &str) -> Box<bbb_protocol::RichTextComponent> {
    Box::new(
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String(text.to_string()),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("red".to_string()),
            },
        ]))
        .unwrap(),
    )
}

fn add_boss_bar_with_id_and_progress(world: &mut WorldStore, id: uuid::Uuid, progress: f32) {
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id,
        operation: ProtocolBossEventOperation::Add {
            name: Box::new(bbb_protocol::RichTextComponent::plain("Boss")),
            progress,
            color: BossBarColor::Purple,
            overlay: BossBarOverlay::Progress,
            flags: ProtocolBossEventFlags {
                darken_screen: false,
                play_music: false,
                create_world_fog: false,
            },
        },
    }));
}

fn boss_bar_progresses(
    update: RendererFrameUpdate<Vec<bbb_renderer::HudBossBar>>,
) -> Option<Vec<f32>> {
    match update {
        RendererFrameUpdate::Unchanged => None,
        RendererFrameUpdate::Replace(bars) => {
            Some(bars.into_iter().map(|bar| bar.progress).collect())
        }
    }
}

fn clear_boss_overlay_properties(world: &mut WorldStore) {
    assert!(world.apply_boss_event(ProtocolBossEvent {
        id: uuid::Uuid::from_u128(1),
        operation: ProtocolBossEventOperation::UpdateProperties {
            flags: ProtocolBossEventFlags {
                darken_screen: false,
                play_music: false,
                create_world_fog: false,
            },
        },
    }));
}

fn attach_lightmap_local_player(world: &mut WorldStore, id: i32) {
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id,
        uuid: uuid::Uuid::from_u128(id as u128),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
}

fn mob_effect(entity_id: i32, effect_id: i32, duration_ticks: i32) -> UpdateMobEffect {
    mob_effect_with_blend(entity_id, effect_id, duration_ticks, false)
}

fn mob_effect_with_blend(
    entity_id: i32,
    effect_id: i32,
    duration_ticks: i32,
    blend: bool,
) -> UpdateMobEffect {
    let raw = if blend { 0b1110 } else { 0b0110 };
    UpdateMobEffect {
        entity_id,
        effect_id,
        amplifier: 0,
        duration_ticks,
        flags: MobEffectFlags {
            raw,
            ambient: false,
            visible: true,
            show_icon: true,
            blend,
        },
    }
}

fn empty_lightmap_test_chunk(dimension: WorldDimension) -> ChunkColumn {
    empty_lightmap_test_chunk_with_biome(dimension, 0)
}

fn empty_lightmap_test_chunk_with_biome(dimension: WorldDimension, biome_id: i32) -> ChunkColumn {
    empty_lightmap_test_chunk_with_biomes(
        dimension,
        single_value_container(PaletteDomain::Biomes, 64, biome_id),
    )
}

fn empty_lightmap_test_chunk_with_sky_light(
    dimension: WorldDimension,
    biome_id: i32,
    sky_light: u8,
) -> ChunkColumn {
    let mut chunk = empty_lightmap_test_chunk_with_biome(dimension, biome_id);
    let mut sky = vec![0; TEST_LIGHT_ARRAY_BYTES];
    set_test_light_nibble(&mut sky, section_block_index(0, 1, 0), sky_light);
    let light_section_index = 0 - (dimension.min_section_y() - 1);
    chunk.light = LightData {
        sky_y_mask: single_bit_mask(usize::try_from(light_section_index).unwrap()),
        block_y_mask: Vec::new(),
        empty_sky_y_mask: Vec::new(),
        empty_block_y_mask: Vec::new(),
        sky_updates: vec![sky],
        block_updates: Vec::new(),
    };
    chunk
}

fn empty_lightmap_test_chunk_with_biomes(
    dimension: WorldDimension,
    biomes: PalettedContainerData,
) -> ChunkColumn {
    let section_count = usize::try_from(dimension.height.div_euclid(16)).unwrap();
    ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..section_count)
            .map(|_| ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
                biomes: biomes.clone(),
            })
            .collect(),
        block_entities: Vec::new(),
        light: LightData::default(),
    }
}

fn insert_uniform_biome_neighbors(world: &mut WorldStore, biome_id: i32) {
    for chunk_x in -1..=0 {
        for chunk_z in -1..=0 {
            if chunk_x == 0 && chunk_z == 0 {
                continue;
            }
            let mut chunk = empty_lightmap_test_chunk_with_biome(world.dimension(), biome_id);
            chunk.pos = ChunkPos {
                x: chunk_x,
                z: chunk_z,
            };
            world.insert_decoded_chunk(chunk);
        }
    }
}

fn insert_split_x_biome_neighborhood(
    world: &mut WorldStore,
    left_biome_id: i32,
    right_biome_id: i32,
) {
    for chunk_x in -1..=1 {
        for chunk_z in -1..=1 {
            let biomes = match chunk_x.cmp(&0) {
                std::cmp::Ordering::Less => {
                    single_value_container(PaletteDomain::Biomes, 64, left_biome_id)
                }
                std::cmp::Ordering::Equal => split_x_biome_container(left_biome_id, right_biome_id),
                std::cmp::Ordering::Greater => {
                    single_value_container(PaletteDomain::Biomes, 64, right_biome_id)
                }
            };
            let mut chunk = empty_lightmap_test_chunk_with_biomes(world.dimension(), biomes);
            chunk.pos = ChunkPos {
                x: chunk_x,
                z: chunk_z,
            };
            world.insert_decoded_chunk(chunk);
        }
    }
}

fn split_x_biome_container(left_biome_id: i32, right_biome_id: i32) -> PalettedContainerData {
    let mut packed = 0u64;
    for y in 0..4 {
        for z in 0..4 {
            for x in 0..4 {
                if x >= 2 {
                    let index = ((y as usize) << 4) | ((z as usize) << 2) | x as usize;
                    packed |= 1 << index;
                }
            }
        }
    }
    PalettedContainerData {
        domain: PaletteDomain::Biomes,
        bits_per_entry: 1,
        palette_kind: PaletteKind::Local,
        palette_global_ids: vec![left_biome_id, right_biome_id],
        packed_data: vec![packed as i64],
        entry_count: 64,
    }
}

#[derive(Clone, Copy)]
enum TestEnvironmentAttributeValue<'a> {
    String(&'a str),
    Float(f32),
    Multiply(f32),
}

fn install_biome_environment_profiles(
    world: &mut WorldStore,
    profiles: &[(i32, &[(&str, TestEnvironmentAttributeValue<'_>)])],
) {
    let max_id = profiles
        .iter()
        .map(|(id, _)| *id)
        .max()
        .expect("at least one biome environment profile");
    let max_id = usize::try_from(max_id).expect("biome holder id must be non-negative");
    let mut entries = (0..=max_id)
        .map(|id| RegistryPacketEntry::stub(format!("minecraft:test_biome_{id}")))
        .collect::<Vec<_>>();
    for (id, attributes) in profiles {
        let id = usize::try_from(*id).expect("biome holder id must be non-negative");
        entries[id] = RegistryPacketEntry::with_raw_data(
            format!("minecraft:test_biome_{id}"),
            biome_environment_nbt(attributes),
        );
    }
    world.begin_registry_configuration();
    world.record_registry_entries("minecraft:worldgen/biome", 0, entries);
    for data in minimal_non_empty_synchronized_registry_data() {
        if data.registry != "minecraft:worldgen/biome" {
            world.record_registry_data(data);
        }
    }
    world.finish_registry_configuration().unwrap();
}

fn biome_environment_nbt(attributes: &[(&str, TestEnvironmentAttributeValue<'_>)]) -> Vec<u8> {
    let mut out = vec![10];
    test_nbt_byte(&mut out, "has_precipitation", 1);
    test_nbt_float(&mut out, "temperature", 0.8);
    test_nbt_float(&mut out, "downfall", 0.4);
    out.push(10);
    test_nbt_utf(&mut out, "effects");
    test_nbt_string(&mut out, "water_color", "#3f76e4");
    out.push(0);
    out.push(10);
    test_nbt_utf(&mut out, "attributes");
    for (name, value) in attributes {
        match value {
            TestEnvironmentAttributeValue::String(value) => {
                test_nbt_string(&mut out, name, value);
            }
            TestEnvironmentAttributeValue::Float(value) => {
                test_nbt_float(&mut out, name, *value);
            }
            TestEnvironmentAttributeValue::Multiply(argument) => {
                out.push(10);
                test_nbt_utf(&mut out, name);
                test_nbt_float(&mut out, "argument", *argument);
                test_nbt_string(&mut out, "modifier", "multiply");
                out.push(0);
            }
        }
    }
    out.push(0);
    out.push(0);
    out
}

fn biome_profile(id: i32) -> BiomeColorProfile {
    BiomeColorProfile {
        id,
        name: format!("minecraft:test_biome_{id}"),
        temperature: 0.8,
        temperature_modifier: BiomeTemperatureModifier::None,
        downfall: 0.4,
        has_precipitation: true,
        grass_color: None,
        foliage_color: None,
        dry_foliage_color: None,
        water_color: None,
        grass_color_modifier: GrassColorModifier::None,
    }
}

fn biome_profile_with_precipitation(id: i32, has_precipitation: bool) -> BiomeColorProfile {
    BiomeColorProfile {
        has_precipitation,
        ..biome_profile(id)
    }
}

fn biome_profile_with_weather(
    id: i32,
    temperature: f32,
    has_precipitation: bool,
) -> BiomeColorProfile {
    BiomeColorProfile {
        temperature,
        has_precipitation,
        ..biome_profile(id)
    }
}

fn biome_profile_with_temperature_modifier(
    id: i32,
    temperature: f32,
    temperature_modifier: BiomeTemperatureModifier,
) -> BiomeColorProfile {
    BiomeColorProfile {
        temperature,
        temperature_modifier,
        ..biome_profile(id)
    }
}

fn single_value_container(
    domain: PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn test_motion_blocking_heightmap(
    dimension: WorldDimension,
    entries: &[(u8, u8, i32)],
) -> HeightmapData {
    test_heightmap(4, dimension, entries)
}

fn test_heightmap(
    kind_id: i32,
    dimension: WorldDimension,
    entries: &[(u8, u8, i32)],
) -> HeightmapData {
    let bits = test_heightmap_bits_for_dimension(dimension);
    let mut values = vec![0u64; 16 * 16];
    for &(local_x, local_z, first_available) in entries {
        let index = usize::from(local_x) + usize::from(local_z) * 16;
        values[index] = u64::try_from(first_available - dimension.min_y).unwrap();
    }
    HeightmapData {
        kind_id,
        data: pack_test_fixed_values(&values, bits)
            .into_iter()
            .map(|value| value as i64)
            .collect(),
    }
}

fn test_heightmap_bits_for_dimension(dimension: WorldDimension) -> usize {
    let value = u64::try_from(dimension.height).unwrap() + 1;
    (u64::BITS - (value - 1).leading_zeros()).max(1) as usize
}

fn pack_test_fixed_values(values: &[u64], bits_per_entry: usize) -> Vec<u64> {
    let values_per_long = 64 / bits_per_entry;
    let mut packed = vec![0; values.len().div_ceil(values_per_long)];
    let mask = (1u64 << bits_per_entry) - 1;
    for (index, value) in values.iter().copied().enumerate() {
        let cell_index = index / values_per_long;
        let bit_index = (index - cell_index * values_per_long) * bits_per_entry;
        packed[cell_index] |= (value & mask) << bit_index;
    }
    packed
}

fn set_lightmap_test_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        block_state_id,
    }));
}

fn section_block_index(x: u8, y: u8, z: u8) -> usize {
    ((y as usize) << 8) | ((z as usize) << 4) | x as usize
}

fn single_bit_mask(bit: usize) -> Vec<i64> {
    let mut words = vec![0; bit / 64 + 1];
    words[bit / 64] = (1u64 << (bit % 64)) as i64;
    words
}

fn set_test_light_nibble(layer: &mut [u8], nibble_index: usize, value: u8) {
    let byte = layer.get_mut(nibble_index / 2).unwrap();
    let shift = (nibble_index % 2) * 4;
    *byte = (*byte & !(0x0f << shift)) | ((value & 0x0f) << shift);
}

fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-6);
    }
}

fn assert_close4(actual: [f32; 4], expected: [f32; 4]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-6);
    }
}

fn assert_clear_color_close(actual: ClearColor, expected: ClearColor) {
    assert!((actual.r - expected.r).abs() < 1e-6);
    assert!((actual.g - expected.g).abs() < 1e-6);
    assert!((actual.b - expected.b).abs() < 1e-6);
    assert!((actual.a - expected.a).abs() < 1e-6);
}

fn clear_color_to_fog_color(clear: ClearColor) -> [f32; 4] {
    [
        clear.r as f32,
        clear.g as f32,
        clear.b as f32,
        clear.a as f32,
    ]
}

fn assert_fog_environment_close(actual: FogEnvironment, expected: FogEnvironment) {
    for (actual, expected) in actual.color.iter().zip(expected.color.iter()) {
        assert!(
            (*actual - *expected).abs() < 1e-6,
            "fog color component {actual} != {expected}"
        );
    }
    assert!((actual.environmental_start - expected.environmental_start).abs() < 1e-6);
    assert!((actual.environmental_end - expected.environmental_end).abs() < 1e-6);
    assert!((actual.render_distance_start - expected.render_distance_start).abs() < 1e-6);
    assert!((actual.render_distance_end - expected.render_distance_end).abs() < 1e-6);
    assert!((actual.sky_end - expected.sky_end).abs() < 1e-6);
    assert!((actual.cloud_end - expected.cloud_end).abs() < 1e-6);
}

#[test]
fn server_container_open_releases_held_mouse_actions() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(local_player_pose([0.0, 64.0, 0.0], 30.0, -10.0));
    let mut counters = NetCounters::default();

    crate::input::handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
    );
    world.set_local_destroying_block(bbb_world::BlockPos { x: 1, y: 2, z: 3 });

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(bbb_protocol::packets::PlayerAction {
            action: bbb_protocol::packets::PlayerActionKind::AbortDestroyBlock,
            pos: bbb_protocol::packets::BlockPos { x: 1, y: 2, z: 3 },
            direction: bbb_protocol::packets::Direction::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(bbb_protocol::packets::PlayerAction {
            action: bbb_protocol::packets::PlayerActionKind::ReleaseUseItem,
            pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
            direction: bbb_protocol::packets::Direction::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_dialog_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let sign_pos = BlockPos { x: 1, y: 2, z: 3 };
    set_lightmap_test_block(&mut world, sign_pos, OAK_SIGN_ROTATION_0_BLOCK_STATE_ID);
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        is_front_text: true,
    });
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn hud_sign_editor_screen_projects_standing_sign_preview() {
    let input = ClientInputState::new(true);
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let pos = BlockPos { x: 1, y: 2, z: 3 };
    set_lightmap_test_block(&mut world, pos, OAK_SIGN_ROTATION_0_BLOCK_STATE_ID);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        is_front_text: true,
    });

    let screen = hud_sign_editor_screen(&input, &world, winit::dpi::PhysicalSize::new(320, 240))
        .expect("sign editor screen");
    assert_eq!(screen.title, "Edit Sign Message");
    assert_eq!(
        screen.kind,
        HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        }
    );
    let preview = screen.sign_preview.expect("standing sign PIP preview");
    assert_eq!(preview.lighting, GuiItemLightingEntry::ItemsFlat);
    assert_eq!(preview.rect.width, 96);
    assert_eq!(preview.rect.height, 102);
    assert_eq!(screen.lines, std::array::from_fn(|_| String::new()));
    assert_eq!(screen.cursor, 0);
    assert_eq!(screen.selection, 0);
}

#[test]
fn hud_sign_editor_screen_projects_hanging_sign_background_state() {
    let input = ClientInputState::new(true);
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let pos = BlockPos { x: 1, y: 2, z: 3 };
    set_lightmap_test_block(
        &mut world,
        pos,
        BAMBOO_HANGING_SIGN_ATTACHED_ROTATION_0_BLOCK_STATE_ID,
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        is_front_text: true,
    });

    let screen = hud_sign_editor_screen(&input, &world, winit::dpi::PhysicalSize::new(320, 240))
        .expect("hanging sign editor screen");
    assert_eq!(screen.title, "Edit Hanging Sign Message");
    assert_eq!(
        screen.kind,
        HudSignEditorKind::Hanging {
            wood: SignModelWood::Bamboo,
        }
    );
    assert!(screen.sign_preview.is_none());
}

#[test]
fn hud_pause_screen_projects_no_menu_title() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);

    assert!(hud_pause_screen(&input, surface).is_none());
    input.open_debug_pause_screen_without_menu();

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert_eq!(screen.title, "Game Paused");
    assert!(!screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.options_enabled);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.player_reporting_enabled);
    assert!(!screen.disconnect_hovered);
    assert!(screen.disconnect_enabled);
}

#[test]
fn hud_pause_screen_projects_menu_title() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert_eq!(screen.title, "Game Menu");
    assert!(screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.options_enabled);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.player_reporting_enabled);
    assert!(!screen.disconnect_hovered);
    assert!(screen.disconnect_enabled);
}

#[test]
fn hud_pause_screen_projects_return_to_game_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 78.0
        )))
    );

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(!screen.options_hovered);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_advancements_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 102.0
        )))
    );

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(screen.advancements_hovered);
    assert!(!screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(!screen.options_hovered);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_stats_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            170.0, 102.0
        )))
    );

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.return_to_game_hovered);
    assert!(!screen.advancements_hovered);
    assert!(screen.stats_hovered);
    assert!(!screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(!screen.options_hovered);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_send_feedback_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 126.0
        )))
    );

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(screen.send_feedback_hovered);
    assert!(!screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_report_bugs_hover() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            170.0, 126.0
        )))
    );

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.send_feedback_hovered);
    assert!(screen.report_bugs_hovered);
    assert!(screen.report_bugs_enabled);
    assert!(!screen.options_hovered);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_disabled_options_row_shell() {
    let mut input = ClientInputState::new(true);
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 150.0
        )))
    );

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(!screen.options_hovered);
    assert!(!screen.options_enabled);
    assert!(!screen.player_reporting_hovered);
    assert!(!screen.player_reporting_enabled);
    assert!(!screen.disconnect_hovered);
}

#[test]
fn hud_pause_screen_projects_disconnect_hover_and_disabled_after_click() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(
        input.handle_debug_pause_screen_cursor_moved(Some(winit::dpi::PhysicalPosition::new(
            68.0, 198.0
        )))
    );

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(screen.show_pause_menu);
    assert!(screen.disconnect_hovered);
    assert!(screen.disconnect_enabled);

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(68.0, 198.0)),
        surface,
    ));

    let screen = hud_pause_screen(&input, surface).expect("pause screen");
    assert!(!screen.disconnect_hovered);
    assert!(!screen.disconnect_enabled);
}

#[test]
fn hud_stats_screen_waits_for_response_then_projects_general_item_and_mob_rows() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    let item_runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let stone_block_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:stone")
        .unwrap();

    let terrain_textures = TerrainTextureState::default();
    assert!(hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .is_none());
    assert!(world.open_stats_screen());
    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(winit::dpi::PhysicalPosition::new(70.0, 223.0)),
        surface,
    ));

    let screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .expect("stats screen");
    assert_eq!(screen.title, "Stats");
    assert_eq!(screen.loading_text, "Downloading statistics...");
    assert!(screen.is_loading);
    assert!(screen.rows.is_empty());
    assert_eq!(screen.scroll_amount, 0);
    assert_eq!(screen.max_scroll_amount, 0);
    assert_eq!(screen.visible_rows, 0);
    assert!(screen.done_hovered);

    world.apply_award_stats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                value_id: 6,
                amount: 12_345,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 2,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                value_id: stone_block_id,
                amount: 9,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 1,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:snowball",
                )
                .unwrap()
                .id,
                amount: 7,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:apple",
                )
                .unwrap()
                .id,
                amount: 2,
            },
        ],
    });
    let screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        winit::dpi::PhysicalSize::new(320, 1_200),
    )
    .expect("stats screen");
    assert!(!screen.is_loading);
    assert_eq!(screen.total_rows, 77);
    assert_eq!(screen.visible_rows, 77);
    assert!(screen
        .rows
        .iter()
        .any(|row| { row.label == "minecraft:walk_one_cm" && row.value == "123.45 m" }));

    let mut counters = NetCounters::default();
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 30.0)),
        surface,
    ));
    let item_screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .expect("items stats tab");
    assert_eq!(item_screen.active_tab, 1);
    assert!(item_screen.items_tab_enabled);
    assert_eq!(item_screen.row_height, 22);
    assert_eq!(item_screen.item_rows.len(), 3);
    assert!(item_screen.item_rows.iter().any(|row| {
        row.resource_id == "minecraft:snowball"
            && row.values[0].is_none()
            && row.values[1].as_deref() == Some("0")
            && row.values[2].as_deref() == Some("7")
    }));
    assert!(item_screen.item_rows.iter().any(|row| {
        row.resource_id == "minecraft:stone"
            && row.values[0].as_deref() == Some("9")
            && row.values[1].as_deref() == Some("0")
            && row.icon.is_some()
    }));
    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(winit::dpi::PhysicalPosition::new(25.0, 72.0)),
        surface,
    ));
    let hovered_item_screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();

    assert_eq!(
        hovered_item_screen
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some(hovered_item_screen.item_rows[0].label.as_str())
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 50.0)),
        surface,
    ));
    let item_screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(item_screen.item_sort_column, Some(2));
    assert_eq!(item_screen.item_sort_order, -1);
    assert_eq!(item_screen.item_rows[0].resource_id, "minecraft:snowball");
    assert_eq!(
        item_screen
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some("Crafted")
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 50.0)),
        surface,
    ));
    let item_screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(item_screen.item_sort_order, 1);
    assert_eq!(item_screen.item_rows[0].resource_id, "minecraft:stone");

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 50.0)),
        surface,
    ));
    let item_screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(item_screen.item_sort_column, None);
    assert_eq!(item_screen.item_sort_order, 0);
    assert_eq!(item_screen.item_rows[0].resource_id, "minecraft:stone");

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(253.0, 30.0)),
        surface,
    ));
    let screen = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .expect("mobs stats tab");
    assert_eq!(screen.active_tab, 2);
    assert!(screen.mobs_tab_enabled);
    assert_eq!(screen.row_height, 36);
    assert_eq!(screen.rows.len(), 1);
    assert_eq!(screen.rows[0].label, "minecraft:zombie");
    assert_eq!(screen.rows[0].value, "You killed 2 minecraft:zombie");
    assert_eq!(
        screen.rows[0].secondary_value.as_deref(),
        Some("minecraft:zombie killed you 1 time(s)")
    );
}

#[test]
fn stats_screen_narration_matches_tabs_general_items_and_mobs_widgets() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    let item_runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let terrain_textures = TerrainTextureState::default();
    assert!(world.open_stats_screen());

    let loading = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(
        stats_screen_narration_frame(&input, &loading, Some(&item_runtime), surface)
            .all_text_for_test(),
        "Stats. Use mouse cursor or Tab button to select element"
    );

    world.apply_award_stats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                value_id: 6,
                amount: 12_345,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:apple",
                )
                .unwrap()
                .id,
                amount: 2,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 2,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 1,
            },
        ],
    });
    let general = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(
        stats_screen_narration_frame(&input, &general, Some(&item_runtime), surface)
            .all_text_for_test(),
        "Stats. General tab. Screen element 1 out of 3. Selected tab 1 out of 3. Press Tab to navigate to the next element. Press Ctrl and Tab to switch between tabs"
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 50.0)),
        surface,
    ));
    let general = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    let row = &general.rows[0];
    assert_eq!(
        stats_screen_narration_frame(&input, &general, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Selected: {} {}. Screen element 2 out of 3. Selected list row 1 out of {}. Press Tab to navigate to the next element. Press the up and down buttons to move to another entry",
            row.label, row.value, general.total_rows
        )
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 30.0)),
        surface,
    ));
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 50.0)),
        surface,
    ));
    let items = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(items.active_tab, 1);
    assert_eq!(
        stats_screen_narration_frame(&input, &items, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Crafted button. Screen element 2 out of 3. Selected list row 1 out of {}. Selected row element 3 out of 6. Crafted. Press Tab to navigate to the next element. Press Enter to activate",
            items.total_rows + 1
        )
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(25.0, 72.0)),
        surface,
    ));
    let items = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(
        stats_screen_narration_frame(&input, &items, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Item: {}. Screen element 2 out of 3. Selected list row 2 out of {}. Press Tab to navigate to the next element",
            items.item_rows[0].label,
            items.total_rows + 1
        )
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(253.0, 30.0)),
        surface,
    ));
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 50.0)),
        surface,
    ));
    let mobs = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    let mob = &mobs.rows[0];
    assert_eq!(
        stats_screen_narration_frame(&input, &mobs, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Selected: {}. {}. Screen element 2 out of 3. Press Tab to navigate to the next element. Press the up and down buttons to move to another entry",
            mob.value,
            mob.secondary_value.as_deref().unwrap()
        )
    );

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    let done = hud_stats_screen(
        &mut input,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(
        stats_screen_narration_frame(&input, &done, Some(&item_runtime), surface)
            .all_text_for_test(),
        "Stats. Done button. Screen element 3 out of 3. Press Tab to navigate to the next element. Press Enter to activate"
    );
}

#[test]
fn hud_stats_screen_reports_empty_tab_tooltip_after_response() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    assert!(world.open_stats_screen());
    world.apply_award_stats(AwardStats { stats: Vec::new() });
    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(winit::dpi::PhysicalPosition::new(160.0, 30.0)),
        surface,
    ));

    let screen = hud_stats_screen(
        &mut input,
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();

    assert!(!screen.items_tab_enabled);
    assert_eq!(
        screen.tooltip.as_ref().map(|tooltip| tooltip.text.as_str()),
        Some("No statistics found")
    );
    assert_eq!(
        stats_screen_narration_frame(&input, &screen, None, surface).all_text_for_test(),
        "Stats. Items tab. Screen element 1 out of 3. Selected tab 2 out of 3. No statistics found"
    );
}

#[test]
fn command_suggestion_narration_localizes_vanilla_payload_shapes() {
    assert_eq!(
        localized_command_suggestion_narration(
            None,
            &CommandSuggestionNarrationState {
                position: 2,
                total: 4,
                text: "stone".to_string(),
                tooltip: None,
                tooltip_translation_key: None,
            },
        ),
        "Selected suggestion 2 out of 4: stone"
    );
    assert_eq!(
        localized_command_suggestion_narration(
            None,
            &CommandSuggestionNarrationState {
                position: 3,
                total: 5,
                text: "stick".to_string(),
                tooltip: Some("minecraft:stick".to_string()),
                tooltip_translation_key: None,
            },
        ),
        "Selected suggestion 3 out of 5: stick (minecraft:stick)"
    );
    assert_eq!(
        format_localized_string_arguments("Selected %s (%s), 100%%", &["value %s", "tip"]),
        "Selected value %s (tip), 100%"
    );

    let root = unique_runtime_temp_dir("command-suggestion-language");
    write_runtime_bow_item_assets(&root);
    write_runtime_json(
        &runtime_assets_dir(&root).join("lang").join("en_us.json"),
        r#"{"entity.minecraft.zombie":"Localized Zombie"}"#,
    );
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    assert_eq!(
        localized_command_suggestion_narration(
            Some(&item_runtime),
            &CommandSuggestionNarrationState {
                position: 1,
                total: 2,
                text: "minecraft:zombie".to_string(),
                tooltip: Some("minecraft:zombie".to_string()),
                tooltip_translation_key: Some("entity.minecraft.zombie".to_string()),
            },
        ),
        "Selected suggestion 1 out of 2: minecraft:zombie (Localized Zombie)"
    );
    let tooltip_runs = hud_command_suggestion_tooltip_runs(
        CommandSuggestionHudTooltip {
            text: "minecraft:zombie".to_string(),
            translation_key: Some("entity.minecraft.zombie".to_string()),
            runs: Vec::new(),
            x: 12,
            y: 24,
        },
        Some(&item_runtime),
    );
    assert_eq!(tooltip_runs.len(), 1);
    assert_eq!(tooltip_runs[0].text, "Localized Zombie");
    assert_eq!(tooltip_runs[0].style, HudTextStyle::default());
    assert_eq!(tooltip_runs[0].color, None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn command_suggestion_narration_speaks_once_per_state_change_and_resets() {
    #[derive(Default)]
    struct RecordingNarrator(Vec<String>);

    impl SystemNarrator for RecordingNarrator {
        fn say_system_now(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_chat_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_system_chat_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_system_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }
    }

    let narration = CommandSuggestionNarrationState {
        position: 1,
        total: 2,
        text: "stone".to_string(),
        tooltip: None,
        tooltip_translation_key: None,
    };
    let mut last = None;
    let mut narrator = RecordingNarrator::default();
    narrate_command_suggestion_change(Some(&narration), &mut last, None, Some(&mut narrator));
    narrate_command_suggestion_change(Some(&narration), &mut last, None, Some(&mut narrator));
    narrate_command_suggestion_change(None, &mut last, None, Some(&mut narrator));
    narrate_command_suggestion_change(Some(&narration), &mut last, None, Some(&mut narrator));

    assert_eq!(
        narrator.0,
        [
            "Selected suggestion 1 out of 2: stone",
            "Selected suggestion 1 out of 2: stone"
        ]
    );
}

#[test]
fn stats_screen_pixel_projection_scrolls_item_header_as_first_entry() {
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    assert_eq!(stats_screen_viewport_height(surface, 14), 161);
    assert_eq!(stats_screen_scroller_height(161, 280, 119), 92);

    assert_eq!(stats_screen_first_data_row(0, 7, 14), 0);
    assert_eq!(stats_screen_first_data_row_offset(0, 0, 7, 14), -5);
    assert_eq!(stats_screen_projected_row_count(20, 161, -5, 14), 12);

    assert_eq!(stats_screen_first_data_row(1, 11, 22), 0);
    assert_eq!(stats_screen_first_data_row_offset(1, 0, 11, 22), 13);
    assert_eq!(stats_screen_first_data_row(1, 30, 22), 0);
    assert_eq!(stats_screen_first_data_row_offset(1, 0, 30, 22), -6);
    assert_eq!(stats_screen_first_data_row(1, 44, 22), 0);
    assert_eq!(stats_screen_first_data_row_offset(1, 0, 44, 22), -20);
    assert_eq!(stats_screen_first_data_row(1, 46, 22), 1);
    assert_eq!(stats_screen_first_data_row_offset(1, 1, 46, 22), 0);
}

#[test]
fn hud_stats_screen_clamps_pixel_scroll_to_projected_content() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    assert!(world.open_stats_screen());
    world.apply_award_stats(AwardStats { stats: Vec::new() });

    let screen = hud_stats_screen(
        &mut input,
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(screen.total_rows, 77);
    assert_eq!(screen.max_scroll_amount, 921);
    assert_eq!(screen.scroller_height, 32);

    assert!(input.handle_stats_screen_mouse_wheel(
        &world,
        winit::event::MouseScrollDelta::LineDelta(0.0, -1.0),
    ));
    let screen = hud_stats_screen(
        &mut input,
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(screen.scroll_amount, 7);
    assert_eq!(screen.scroll_row, 0);
    assert_eq!(screen.visible_rows, 12);

    assert!(input.handle_stats_screen_mouse_wheel(
        &world,
        winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(
            0.0, -10_000.0,
        )),
    ));
    let screen = hud_stats_screen(
        &mut input,
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    )
    .unwrap();
    assert_eq!(screen.scroll_amount, 921);
    assert_eq!(screen.scroll_row, 65);
    assert_eq!(screen.rows.len(), 12);
}

#[test]
fn low_disk_space_toast_uses_ten_second_visible_window_and_square_slides() {
    let start = std::time::Instant::now();
    let mut state = SystemToastAnimationState::default();
    assert_eq!(state.update(0, start), None);
    assert_eq!(
        state.update(1, start),
        Some(SystemToastAnimationFrame {
            visible_portion: 0.0,
            visibility_sound: Some("minecraft:ui.toast.in"),
        })
    );
    assert_eq!(
        state
            .update(1, start + Duration::from_millis(300))
            .map(|frame| frame.visible_portion),
        Some(0.25)
    );
    assert_eq!(
        state
            .update(1, start + Duration::from_millis(600))
            .map(|frame| frame.visible_portion),
        Some(1.0)
    );
    assert_eq!(
        state
            .update(1, start + Duration::from_millis(10_599))
            .map(|frame| frame.visible_portion),
        Some(1.0)
    );
    assert_eq!(
        state.update(1, start + Duration::from_millis(10_600)),
        Some(SystemToastAnimationFrame {
            visible_portion: 1.0,
            visibility_sound: Some("minecraft:ui.toast.out"),
        })
    );
    assert_eq!(
        state
            .update(1, start + Duration::from_millis(10_900))
            .map(|frame| frame.visible_portion),
        Some(0.75)
    );
    assert_eq!(state.update(1, start + Duration::from_millis(11_201)), None);

    assert_eq!(
        state.update(2, start + Duration::from_millis(12_000)),
        Some(SystemToastAnimationFrame {
            visible_portion: 0.0,
            visibility_sound: Some("minecraft:ui.toast.in"),
        })
    );
}

#[test]
fn low_disk_space_warning_projects_one_server_driven_system_toast() {
    let start = std::time::Instant::now();
    let mut world = WorldStore::new();
    let mut state = HudRenderRuntime::default();

    assert_eq!(
        hud_system_toast(&mut state, &world, None, start),
        (None, None)
    );

    world.apply_low_disk_space_warning();
    let (toast, sound) = hud_system_toast(&mut state, &world, None, start);
    assert_eq!(
        toast,
        Some(HudSystemToast {
            title: "Low disk space!".to_string(),
            message: "Might not be able to save the world.".to_string(),
            multiline: false,
            visible_portion: 0.0,
        })
    );
    assert_eq!(
        sound.and_then(|sound| sound.sound.location),
        Some("minecraft:ui.toast.in".to_string())
    );
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start + Duration::from_millis(600),)
            .0
            .map(|toast| toast.visible_portion),
        Some(1.0)
    );

    world.apply_low_disk_space_warning();
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(10_500),
        )
        .0
        .map(|toast| toast.visible_portion),
        Some(1.0)
    );
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(20_499),
        )
        .0
        .map(|toast| toast.visible_portion),
        Some(1.0)
    );
}

#[test]
fn advancement_toast_runtime_admits_five_fifo_slots_and_matches_vanilla_timing() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(&mut world, 6, AdvancementFrameType::Challenge);
    let start = Instant::now();
    let mut state = AdvancementToastRuntime::default();

    let admission_sounds = state.update(&world, start);
    assert_eq!(state.visible.len(), 5);
    assert_eq!(state.queued.len(), 1);
    assert_eq!(
        state
            .visible
            .iter()
            .map(|toast| toast.slot)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(state.visible[0].event.advancement_id, "test:toast_0");
    assert_eq!(state.queued[0].advancement_id, "test:toast_5");
    assert_eq!(
        sound_locations(&admission_sounds),
        vec!["minecraft:ui.toast.challenge_complete"]
    );

    let transition_sounds = state.update(&world, start);
    assert_eq!(
        sound_locations(&transition_sounds),
        vec!["minecraft:ui.toast.in"]
    );
    assert!(state
        .visible
        .iter()
        .all(|toast| toast.visible_portion == 0.0));

    assert!(state
        .update(&world, start + Duration::from_millis(300))
        .is_empty());
    assert!(state
        .visible
        .iter()
        .all(|toast| (toast.visible_portion - 0.25).abs() < 1.0e-6));
    state.update(&world, start + Duration::from_millis(600));
    assert!(state
        .visible
        .iter()
        .all(|toast| toast.visible_portion == 1.0 && toast.fully_visible_ms == 0));

    assert!(state
        .update(&world, start + Duration::from_millis(5_599))
        .is_empty());
    assert_eq!(state.visible[0].fully_visible_ms, 4_999);
    assert_eq!(
        sound_locations(&state.update(&world, start + Duration::from_millis(5_600))),
        vec!["minecraft:ui.toast.out"]
    );
    state.update(&world, start + Duration::from_millis(5_900));
    assert!((state.visible[0].visible_portion - 0.75).abs() < 1.0e-6);

    let next_admission = state.update(&world, start + Duration::from_millis(6_201));
    assert_eq!(state.visible.len(), 1);
    assert_eq!(state.visible[0].event.advancement_id, "test:toast_5");
    assert_eq!(state.visible[0].slot, 0);
    assert_eq!(
        sound_locations(&next_admission),
        vec!["minecraft:ui.toast.challenge_complete"]
    );
}

#[test]
fn advancement_toast_runtime_counts_retained_window_lag_and_consumes_once() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(
        &mut world,
        bbb_world::MAX_ADVANCEMENT_TOAST_EVENTS + 1,
        AdvancementFrameType::Task,
    );
    let start = Instant::now();
    let mut state = AdvancementToastRuntime::default();
    state.update(&world, start);
    assert_eq!(state.dropped_events, 1);
    assert_eq!(
        state.next_sequence,
        u64::try_from(bbb_world::MAX_ADVANCEMENT_TOAST_EVENTS + 1).unwrap()
    );
    assert_eq!(state.visible.len(), 5);
    assert_eq!(
        state.queued.len(),
        bbb_world::MAX_ADVANCEMENT_TOAST_EVENTS - bbb_renderer::HUD_ADVANCEMENT_TOAST_SLOTS
    );

    state.update(&world, start);
    assert_eq!(state.dropped_events, 1);
    assert_eq!(state.visible.len(), 5);
    assert_eq!(
        state.queued.len(),
        bbb_world::MAX_ADVANCEMENT_TOAST_EVENTS - bbb_renderer::HUD_ADVANCEMENT_TOAST_SLOTS
    );
}

#[test]
fn advancement_toast_runtime_clears_visible_and_queued_state_for_new_level() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(&mut world, 6, AdvancementFrameType::Goal);
    let mut state = AdvancementToastRuntime::default();
    state.update(&world, Instant::now());
    assert_eq!(state.visible.len(), 5);
    assert_eq!(state.queued.len(), 1);

    world.clear_client_level();
    assert!(state.update(&world, Instant::now()).is_empty());
    assert!(state.visible.is_empty());
    assert!(state.queued.is_empty());
    assert_eq!(state.next_sequence, 0);
}

#[test]
fn advancement_toast_and_screen_preserve_rich_title_runs() {
    let rich_title =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Styled toast".to_string()),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("red".to_string()),
            },
        ]))
        .unwrap();
    let mut world = WorldStore::new();
    let mut advancement = runtime_displayed_advancement("test:styled", None);
    let display = advancement.display.as_mut().unwrap();
    display.title = rich_title.clone();
    display.show_toast = true;
    advancement.requirements = vec![vec!["done".to_string()]];
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![advancement],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    world.apply_update_advancements(UpdateAdvancements {
        reset: false,
        added: Vec::new(),
        removed: Vec::new(),
        progress: vec![AdvancementProgressSummary {
            id: "test:styled".to_string(),
            criteria: vec![AdvancementCriterionProgressSummary {
                name: "done".to_string(),
                obtained_epoch_millis: Some(1),
            }],
        }],
        show_advancements: true,
    });

    let mut runtime = AdvancementToastRuntime::default();
    runtime.update(&world, Instant::now());
    let item_runtime = recipe_book_ghost_item_runtime();
    let projected = runtime.project(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
    );
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].title_runs[0].text, "Styled toast");
    assert!(projected[0].title_runs[0].style.bold);
    assert_eq!(projected[0].title_runs[0].color, Some(0xFF_55_55));

    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("test:styled".to_string())
    );
    let screen = hud_advancements_screen(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        0.0,
        winit::dpi::PhysicalSize::new(800, 600),
        None,
        ItemModelKeybindContext::default(),
        0.0,
    );
    let title = screen
        .text_labels
        .iter()
        .find(|label| label.text == "Styled toast")
        .unwrap();
    assert!(title.runs[0].style.bold);
    assert_eq!(title.runs[0].color, Some(0xFF_55_55));
}

fn enqueue_runtime_advancement_toasts(
    world: &mut WorldStore,
    count: usize,
    frame_type: AdvancementFrameType,
) {
    let mut added = Vec::with_capacity(count);
    let mut progress = Vec::with_capacity(count);
    for index in 0..count {
        let id = format!("test:toast_{index}");
        let mut advancement = runtime_displayed_advancement(&id, None);
        let display = advancement.display.as_mut().unwrap();
        display.title = bbb_protocol::RichTextComponent::plain(format!("Toast {index}"));
        display.show_toast = true;
        display.frame_type = frame_type;
        advancement.requirements = vec![vec!["done".to_string()]];
        added.push(advancement);
        progress.push(AdvancementProgressSummary {
            id,
            criteria: vec![AdvancementCriterionProgressSummary {
                name: "done".to_string(),
                obtained_epoch_millis: Some(1),
            }],
        });
    }
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added,
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    world.apply_update_advancements(UpdateAdvancements {
        reset: false,
        added: Vec::new(),
        removed: Vec::new(),
        progress,
        show_advancements: true,
    });
}

fn sound_locations(sounds: &[LocalSoundEventState]) -> Vec<&str> {
    sounds
        .iter()
        .filter_map(|sound| sound.sound.location.as_deref())
        .collect()
}

#[test]
fn insecure_chat_warning_projects_multiline_toast_before_low_disk_queue() {
    let start = std::time::Instant::now();
    let mut world = world_with_custom_dimension_environment();
    let mut state = HudRenderRuntime::default();
    assert_eq!(world.insecure_chat_warning_count(), 1);

    let (toast, sound) = hud_system_toast(&mut state, &world, None, start);
    assert_eq!(
        toast,
        Some(HudSystemToast {
            title: "Chat messages can't be verified".to_string(),
            message: "Messages sent on this server may be modified and might not reflect the original message".to_string(),
            multiline: true,
            visible_portion: 0.0,
        })
    );
    assert_eq!(
        sound.and_then(|sound| sound.sound.location),
        Some("minecraft:ui.toast.in".to_string())
    );

    world.apply_low_disk_space_warning();
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start + Duration::from_millis(600))
            .0
            .unwrap()
            .title,
        "Chat messages can't be verified"
    );
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(10_600),
        )
        .1
        .and_then(|sound| sound.sound.location),
        Some("minecraft:ui.toast.out".to_string())
    );

    let (toast, sound) = hud_system_toast(
        &mut state,
        &world,
        None,
        start + Duration::from_millis(11_201),
    );
    assert_eq!(toast.unwrap().title, "Low disk space!");
    assert_eq!(
        sound.and_then(|sound| sound.sound.location),
        Some("minecraft:ui.toast.in".to_string())
    );
}

#[test]
fn vanilla_stat_formatters_preserve_thresholds_and_grouping() {
    assert_eq!(format_vanilla_stat_value("DEFAULT", 1_234_567), "1,234,567");
    assert_eq!(format_vanilla_stat_value("DIVIDE_BY_TEN", 123), "12.30");
    assert_eq!(format_vanilla_stat_value("DISTANCE", 50), "50 cm");
    assert_eq!(format_vanilla_stat_value("DISTANCE", 51), "0.51 m");
    assert_eq!(format_vanilla_stat_value("DISTANCE", 50_001), "0.50 km");
    assert_eq!(format_vanilla_stat_value("TIME", 20), "1.0 s");
    assert_eq!(format_vanilla_stat_value("TIME", 601), "0.50 min");
}

#[test]
fn hud_debug_options_screen_projects_visible_rows_and_suppresses_pause() {
    let mut input = ClientInputState::new(true);
    let world = WorldStore::new();
    input.open_debug_pause_screen_without_menu();
    input.open_debug_options_screen();
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::Biome,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    assert!(hud_pause_screen(&input, winit::dpi::PhysicalSize::new(420, 240)).is_none());
    let screen = hud_debug_options_screen(&input, &world, winit::dpi::PhysicalSize::new(420, 240))
        .expect("debug options screen");

    assert_eq!(screen.title, "Debug Options");
    assert_eq!(screen.total_rows, 47);
    assert_eq!(screen.visible_rows, 7);
    assert!(screen.default_profile_active);
    assert_eq!(
        screen.rows[0],
        HudDebugOptionsRow::Category {
            label: "Debug Screen Text".to_string()
        }
    );
    assert_eq!(
        screen.rows[1],
        HudDebugOptionsRow::Entry {
            path: "biome".to_string(),
            status: HudDebugOptionsEntryStatus::AlwaysOn,
            hovered_status: None,
            allowed: true,
        }
    );
}

#[test]
fn hud_debug_options_screen_projects_not_allowed_tooltip_under_reduced_debug_info() {
    let mut input = ClientInputState::new(true);
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("biome"));
    let tooltip_x = 37;
    let tooltip_y = 83;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(tooltip_x),
            f64::from(tooltip_y)
        )),
        surface,
    ));

    let screen = hud_debug_options_screen(&input, &world, surface).expect("debug options screen");

    assert_eq!(
        screen.tooltip.as_ref().map(|tooltip| tooltip.text.as_str()),
        Some("Not visible when debug info is reduced")
    );
    assert_eq!(
        screen
            .tooltip
            .as_ref()
            .map(|tooltip| (tooltip.x, tooltip.y)),
        Some((tooltip_x, tooltip_y))
    );
    assert_eq!(
        screen.rows[1],
        HudDebugOptionsRow::Entry {
            path: "biome".to_string(),
            status: HudDebugOptionsEntryStatus::Never,
            hovered_status: None,
            allowed: false,
        }
    );
}

#[test]
fn book_screen_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: written_book_stack(vec!["First page"]),
    });
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
    });
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn renderer_camera_pose_follows_active_camera_entity() {
    let mut world = WorldStore::new();
    world.set_local_player_pose(local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0));
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 123,
        uuid: uuid::Uuid::from_u128(123),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d::default(),
        x_rot: -15.0,
        y_rot: 30.0,
        y_head_rot: 30.0,
        data: 0,
    });

    assert_eq!(
        camera_pose_from_world(&world),
        Some(CameraPose {
            position: [10.0, 64.0, -5.0],
            y_rot: 90.0,
            x_rot: -10.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        })
    );

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 123 }));
    assert_eq!(
        camera_pose_from_world(&world),
        Some(CameraPose {
            position: [1.0, 2.0, 3.0],
            y_rot: 30.0,
            x_rot: -15.0,
            eye_height: 0.2751,
        })
    );

    assert_eq!(
        world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
            entity_ids: vec![123],
        }),
        1
    );
    assert_eq!(
        camera_pose_from_world(&world),
        Some(CameraPose {
            position: [10.0, 64.0, -5.0],
            y_rot: 90.0,
            x_rot: -10.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        })
    );
}

#[test]
fn audio_scene_command_tracks_listener_and_entity_positions() {
    let mut world = WorldStore::new();
    let standing_pose = local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0);
    world.set_local_player_pose(standing_pose);
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 123,
        uuid: uuid::Uuid::from_u128(123),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let command = audio_scene_command_from_world(&world);

    assert_eq!(
        command.listener,
        Some(AudioListenerState {
            position: [10.0, 64.0 + standing_pose.eye_height(), -5.0],
            y_rot: 90.0,
            x_rot: -10.0,
        })
    );
    assert_eq!(
        command.entities,
        vec![EntitySoundPosition {
            entity_id: 123,
            position: [1.0, 2.0, 3.0],
        }]
    );

    let sneaking_pose = LocalPlayerPoseState {
        sneaking: true,
        ..local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0)
    };
    world.set_local_player_pose(sneaking_pose);
    let command = audio_scene_command_from_world(&world);
    assert_eq!(
        command.listener,
        Some(AudioListenerState {
            position: [10.0, 64.0 + sneaking_pose.eye_height(), -5.0],
            y_rot: 90.0,
            x_rot: -10.0,
        })
    );

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 123 }));
    let command = audio_scene_command_from_world(&world);
    assert_eq!(
        command.listener,
        Some(AudioListenerState {
            position: [1.0, 2.0 + f64::from(0.2751_f32), 3.0],
            y_rot: 0.0,
            x_rot: 0.0,
        })
    );

    assert_eq!(
        world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
            entity_ids: vec![123],
        }),
        1
    );
    let command = audio_scene_command_from_world(&world);
    assert!(command.entities.is_empty());
    assert_eq!(
        command.listener,
        Some(AudioListenerState {
            position: [10.0, 64.0 + sneaking_pose.eye_height(), -5.0],
            y_rot: 90.0,
            x_rot: -10.0,
        })
    );
}

#[test]
fn hud_inventory_screen_projects_open_local_inventory_layout() {
    let mut world = WorldStore::new();
    assert_eq!(hud_inventory_screen(&world, None, Some(36), 0.0), None);

    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: bbb_protocol::packets::ItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, None, Some(36), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Inventory,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookButton,
                104,
                61,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(36));
    assert_eq!(screen.tooltip, None);
    assert_eq!(screen.slots.len(), 46);
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    assert_eq!(hotbar.x, 8);
    assert_eq!(hotbar.y, 142);
    assert!(hotbar.icon.is_none());
    let offhand = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!(offhand.x, 77);
    assert_eq!(offhand.y, 62);
}

#[test]
fn hud_inventory_screen_projects_local_player_entity_preview() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((10, 80)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.entity.entity_id, 42);
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 26,
            y: 8,
            width: 49,
            height: 70,
        }
    );
    assert_eq!(preview.scissor, None);
    assert_eq!(preview.scale, 30.0);
    assert!(preview.depth_isolated);
    assert_close3(preview.translation, [0.0, 0.9625, 0.0]);

    let x_angle = ((50.5_f32 - 10.0) / 40.0).atan();
    let y_angle = ((43.0_f32 - 80.0) / 40.0).atan();
    let expected_yaw = x_angle * 20.0;
    let expected_pitch = y_angle * 20.0;
    let expected_camera = quaternion_x(expected_pitch.to_radians());
    assert!((preview.entity.render_state.body_rot - (180.0 + expected_yaw)).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_yaw - expected_yaw).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_pitch + expected_pitch).abs() < 1.0e-6);
    assert_close4(
        preview.rotation,
        quaternion_mul([0.0, 0.0, 1.0, 0.0], expected_camera),
    );
    assert_eq!(preview.override_camera_rotation, Some(expected_camera));
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert_eq!(preview.entity.render_state.outline_color, 0);
    assert!(!preview.entity.render_state.appears_glowing);
}

#[test]
fn hotbar_item_icons_use_using_item_model_for_selected_slot_only() {
    let root = unique_runtime_temp_dir("hotbar-using-item");
    write_runtime_bow_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let using_uv = item_runtime
        .icon_for_stack_with_bundle_selected_item_and_using_item(&stack, None, true)
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, using_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));
    world.set_local_using_item(true);

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: using_uv.min,
            max: using_uv.max,
        }
    );
    assert_eq!(
        icons[1].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_selected_item_condition_for_selected_slot_only() {
    let root = unique_runtime_temp_dir("hotbar-selected-condition");
    write_runtime_selected_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let selected_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_selected(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            true,
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, selected_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(1));

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        icons[1].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: selected_uv.min,
            max: selected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_uses_selected_item_condition_for_local_selected_slot_only() {
    let root = unique_runtime_temp_dir("inventory-selected-condition");
    write_runtime_selected_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let selected_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_selected(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            true,
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, selected_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(1));
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    let non_selected = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    let selected = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
    assert_eq!(
        non_selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: selected_uv.min,
            max: selected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_container_screen_uses_selected_item_condition_for_server_opened_hotbar_slot_only() {
    let root = unique_runtime_temp_dir("container-selected-condition");
    write_runtime_selected_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let selected_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_selected(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            true,
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, selected_uv);

    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 63];
    items[54] = stack.clone();
    items[55] = stack;
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 1,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.set_local_selected_hotbar_slot(1));

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    let non_selected = screen.slots.iter().find(|slot| slot.slot_id == 54).unwrap();
    let selected = screen.slots.iter().find(|slot| slot.slot_id == 55).unwrap();
    assert_eq!(
        non_selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: selected_uv.min,
            max: selected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_item_icons_use_carried_item_condition_only_when_marked_carried() {
    let root = unique_runtime_temp_dir("hud-carried-condition");
    write_runtime_carried_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let carried_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            true,
            false,
            false,
            ItemModelKeybindContext::default(),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, carried_uv);

    let world = WorldStore::new();
    let ordinary_icon = hud_item_icon_for_stack(
        &world,
        Some(&item_runtime),
        &stack,
        None,
        false,
        false,
        false,
        false,
        false,
        ItemModelKeybindContext::default(),
        0,
        0.0,
    )
    .unwrap();
    let carried_icon = hud_item_icon_for_stack(
        &world,
        Some(&item_runtime),
        &stack,
        None,
        false,
        false,
        true,
        false,
        false,
        ItemModelKeybindContext::default(),
        0,
        0.0,
    )
    .unwrap();

    assert_eq!(
        ordinary_icon.layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        carried_icon.layers[0].uv,
        HudUvRect {
            min: carried_uv.min,
            max: carried_uv.max,
        }
    );
    assert_eq!(ordinary_icon.foil, HudItemFoil::None);

    let mut foiled_stack = stack.clone();
    foiled_stack.component_patch.enchantment_glint_override = Some(true);
    let foiled_icon = hud_item_icon_for_stack(
        &world,
        Some(&item_runtime),
        &foiled_stack,
        None,
        false,
        false,
        false,
        false,
        false,
        ItemModelKeybindContext::default(),
        0,
        0.0,
    )
    .unwrap();
    assert_eq!(foiled_icon.foil, HudItemFoil::Standard);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_item_foil_for_stack_projects_special_clock_and_compass_glint() {
    let root = unique_runtime_temp_dir("hud-special-foil");
    write_runtime_special_foil_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let foiled_stack = |resource_id: &str| {
        let mut stack = item_stack(item_runtime.item_protocol_id(resource_id).unwrap(), 1);
        stack.component_patch.enchantment_glint_override = Some(true);
        stack
    };

    assert_eq!(
        hud_item_foil_for_stack(&item_runtime, &foiled_stack("minecraft:clock")),
        HudItemFoil::Special
    );
    assert_eq!(
        hud_item_foil_for_stack(&item_runtime, &foiled_stack("minecraft:compass")),
        HudItemFoil::Special
    );
    assert_eq!(
        hud_item_foil_for_stack(&item_runtime, &foiled_stack("minecraft:spyglass")),
        HudItemFoil::Standard
    );
    assert_eq!(
        hud_item_foil_for_stack(
            &item_runtime,
            &item_stack(item_runtime.item_protocol_id("minecraft:clock").unwrap(), 1),
        ),
        HudItemFoil::None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_cursor_item_as_carried_floating_item() {
    let root = unique_runtime_temp_dir("hud-cursor-carried-condition");
    write_runtime_carried_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 3);
    let normal_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let carried_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            true,
            false,
            false,
            ItemModelKeybindContext::default(),
        )
        .unwrap()
        .layers[0]
        .uv;

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_cursor_item(ProtocolSetCursorItem {
        item: stack.clone(),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((40, 60)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    assert_eq!(
        hotbar.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(screen.floating_items.len(), 1);
    let cursor = &screen.floating_items[0];
    assert_eq!((cursor.x, cursor.y), (32, 52));
    assert_eq!(
        cursor.icon.layers[0].uv,
        HudUvRect {
            min: carried_uv.min,
            max: carried_uv.max,
        }
    );
    assert_eq!(cursor.icon.count_label, Some(HudItemCountLabel::new("3")));
    assert!(cursor.block_model.is_none());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_quick_craft_cursor_remainder() {
    let root = unique_runtime_temp_dir("hud-quick-craft-cursor-remainder");
    write_runtime_carried_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 11);
    let carried_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            true,
            false,
            false,
            ItemModelKeybindContext::default(),
        )
        .unwrap()
        .layers[0]
        .uv;

    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    world.apply_set_cursor_item(ProtocolSetCursorItem { item: stack });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((40, 60)),
            quick_craft_button_num: Some(0),
            quick_craft_slots: vec![9, 10, 11],
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.floating_items.len(), 1);
    let cursor = &screen.floating_items[0];
    assert_eq!((cursor.x, cursor.y), (32, 52));
    assert_eq!(
        cursor.icon.layers[0].uv,
        HudUvRect {
            min: carried_uv.min,
            max: carried_uv.max,
        }
    );
    assert_eq!(cursor.icon.count_label, Some(HudItemCountLabel::new("2")));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_local_use_ticks_into_use_duration_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-use-duration-range-dispatch");
    write_runtime_bow_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let expected_uv = item_runtime
        .icon_for_stack_with_context_and_use_context(
            &stack,
            None,
            true,
            item_runtime.item_model_use_context_for_stack(&stack, 13),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .layers[0]
        .uv;
    let initial_using_uv = item_runtime
        .icon_for_stack_with_context_and_use_context(
            &stack,
            None,
            true,
            item_runtime.item_model_use_context_for_stack(&stack, 0),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(expected_uv, initial_using_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));
    world.set_local_using_item(true);
    world.advance_local_using_item_ticks(13);

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: expected_uv.min,
            max: expected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_apply_synced_custom_crossbow_charge_time_effect() {
    let root = unique_runtime_temp_dir("hotbar-crossbow-custom-charge-time-range-dispatch");
    write_runtime_crossbow_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut stack = item_stack(0, 1);
    stack.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 0,
        level: 2,
    }];
    let default_uv = item_runtime
        .icon_for_stack_with_context_and_use_context(
            &stack,
            None,
            true,
            item_runtime.item_model_use_context_for_stack(&stack, 10),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .layers[0]
        .uv;
    let expected_uv = item_runtime
        .icon_for_stack_with_context_and_use_context(
            &stack,
            None,
            true,
            item_runtime.item_model_use_context_for_stack_with_crossbow_charge_duration(
                &stack,
                10,
                Some(15),
            ),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(default_uv, expected_uv);

    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![bbb_protocol::packets::RegistryDataEntry {
            id: "example:patient_loading".to_string(),
            raw_data: Some(crossbow_charge_time_enchantment_nbt(0.75)),
        }],
        raw_payload_len: 0,
    });
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));
    world.set_local_using_item(true);
    world.advance_local_using_item_ticks(10);

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: expected_uv.min,
            max: expected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_local_player_main_hand_owner_context() {
    let root = unique_runtime_temp_dir("hotbar-main-hand");
    write_runtime_main_hand_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let right_uv = item_runtime
        .icon_for_stack_with_owner_main_hand(&stack, Some(false))
        .unwrap()
        .layers[0]
        .uv;
    let left_uv = item_runtime
        .icon_for_stack_with_owner_main_hand(&stack, Some(true))
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, right_uv);
    assert_ne!(fallback_uv, left_uv);
    assert_ne!(right_uv, left_uv);

    let mut no_owner_world = WorldStore::new();
    no_owner_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_owner_icons = hotbar_item_icons(&no_owner_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_owner_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let right_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        right_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: right_uv.min,
            max: right_uv.max,
        }
    );

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 42,
            values: vec![test_humanoid_arm_data(15, 0)],
        })
    );
    let left_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        left_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: left_uv.min,
            max: left_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_world_dimension_select_context() {
    let root = unique_runtime_temp_dir("hotbar-context-dimension");
    write_runtime_context_dimension_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let overworld_uv = item_runtime
        .icon_for_stack_with_context(
            &stack,
            None,
            false,
            0.0,
            None,
            None,
            None,
            Some("minecraft:overworld"),
        )
        .unwrap()
        .layers[0]
        .uv;
    let nether_uv = item_runtime
        .icon_for_stack_with_context(
            &stack,
            None,
            false,
            0.0,
            None,
            None,
            None,
            Some("minecraft:the_nether"),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, overworld_uv);
    assert_ne!(fallback_uv, nether_uv);
    assert_ne!(overworld_uv, nether_uv);

    let mut no_level_world = WorldStore::new();
    no_level_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_level_icons = hotbar_item_icons(&no_level_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_level_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    overworld.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let overworld_icons = hotbar_item_icons(&overworld, Some(&item_runtime), 0.0);
    assert_eq!(
        overworld_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: overworld_uv.min,
            max: overworld_uv.max,
        }
    );

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    nether.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let nether_icons = hotbar_item_icons(&nether, Some(&item_runtime), 0.0);
    assert_eq!(
        nether_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: nether_uv.min,
            max: nether_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_local_player_context_entity_type_select() {
    let root = unique_runtime_temp_dir("hotbar-context-entity-type");
    write_runtime_context_entity_type_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let player_uv = item_runtime
        .icon_for_stack_with_context(
            &stack,
            None,
            false,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
        )
        .unwrap()
        .layers[0]
        .uv;
    let cow_uv = item_runtime
        .icon_for_stack_with_context(
            &stack,
            None,
            false,
            0.0,
            None,
            None,
            Some("minecraft:cow"),
            None,
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, player_uv);
    assert_ne!(fallback_uv, cow_uv);
    assert_ne!(player_uv, cow_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: player_uv.min,
            max: player_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_local_player_view_entity_condition() {
    let root = unique_runtime_temp_dir("hotbar-view-entity-condition");
    write_runtime_view_entity_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let view_entity_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            false,
            true,
            false,
            ItemModelKeybindContext::default(),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, view_entity_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: view_entity_uv.min,
            max: view_entity_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_extended_view_condition_when_shift_is_down() {
    let root = unique_runtime_temp_dir("hotbar-extended-view-condition");
    write_runtime_extended_view_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let extended_view_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            false,
            true,
            true,
            ItemModelKeybindContext::default(),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, extended_view_uv);
    let ground_shift_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Ground,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            false,
            true,
            true,
            ItemModelKeybindContext::default(),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_eq!(fallback_uv, ground_shift_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let inactive_icons =
        hotbar_item_icons_with_extended_view(&world, Some(&item_runtime), 0.0, false);
    assert_eq!(
        inactive_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );
    let active_icons = hotbar_item_icons_with_extended_view(&world, Some(&item_runtime), 0.0, true);
    assert_eq!(
        active_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: extended_view_uv.min,
            max: extended_view_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_keybind_down_condition_when_default_key_is_down() {
    let root = unique_runtime_temp_dir("hotbar-keybind-down-condition");
    write_runtime_keybind_down_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let keybind_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            false,
            true,
            false,
            ItemModelKeybindContext {
                quick_actions: true,
                ..ItemModelKeybindContext::default()
            },
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, keybind_uv);
    let unrelated_key_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            false,
            false,
            true,
            false,
            ItemModelKeybindContext {
                forward: true,
                ..ItemModelKeybindContext::default()
            },
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_eq!(fallback_uv, unrelated_key_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let inactive_icons = hotbar_item_icons_with_input_context(
        &world,
        Some(&item_runtime),
        0.0,
        false,
        ItemModelKeybindContext::default(),
    );
    assert_eq!(
        inactive_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );
    let active_icons = hotbar_item_icons_with_input_context(
        &world,
        Some(&item_runtime),
        0.0,
        false,
        ItemModelKeybindContext {
            quick_actions: true,
            ..ItemModelKeybindContext::default()
        },
    );
    assert_eq!(
        active_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: keybind_uv.min,
            max: keybind_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_fishing_rod_cast_condition_for_selected_local_rod() {
    let root = unique_runtime_temp_dir("hotbar-fishing-rod-cast-condition");
    write_runtime_fishing_rod_cast_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let cast_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            None,
            None,
            None,
            true,
            false,
            true,
            false,
            ItemModelKeybindContext::default(),
            true,
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, cast_uv);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));

    let no_bobber_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_bobber_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        data: 42,
        ..test_add_entity(700, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: cast_uv.min,
            max: cast_uv.max,
        }
    );
    assert_eq!(
        icons[1].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_vanilla_zero_partial_cooldown_model_property() {
    let root = unique_runtime_temp_dir("hotbar-cooldown-range-dispatch");
    write_runtime_cooldown_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut stack = item_stack(0, 1);
    stack.component_patch.use_cooldown_group = Some("minecraft:ender_pearl".to_string());
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let active_uv = item_runtime
        .icon_for_stack_with_context(&stack, None, false, 0.75, None, None, None, None)
        .unwrap()
        .layers[0]
        .uv;
    assert_eq!(
        item_runtime
            .icon_for_stack_with_context(&stack, None, false, 0.7, None, None, None, None)
            .unwrap()
            .layers[0]
            .uv,
        fallback_uv
    );
    assert_ne!(fallback_uv, active_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let inactive_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        inactive_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    world.apply_cooldown(bbb_protocol::packets::Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    });
    world.advance_item_cooldowns(5);
    let active_icons = hotbar_item_icons(&world, Some(&item_runtime), 1.0);
    assert_eq!(
        active_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: active_uv.min,
            max: active_uv.max,
        }
    );
    assert_eq!(
        active_icons[0].as_ref().unwrap().cooldown_progress,
        Some(0.7)
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_world_time_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-time-range-dispatch");
    write_runtime_time_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let selected_at_time = |stack: &bbb_protocol::packets::ItemStackSummary, day_time| {
        item_runtime
            .icon_for_stack_with_context_and_use_context_and_time_context(
                stack,
                None,
                false,
                bbb_item_model::ItemModelUseContext::inactive(),
                bbb_pack::BlockModelDisplayContext::Gui,
                0.0,
                None,
                None,
                Some("minecraft:player"),
                Some("minecraft:overworld"),
                Some(bbb_item_model::ItemModelTimeContext {
                    game_time: day_time,
                    day_time,
                    sun_angle_degrees: None,
                    moon_phase_index: None,
                }),
                None,
            )
            .unwrap()
            .layers[0]
            .uv
    };
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let night_uv = selected_at_time(&stack, 18_000);
    assert_ne!(fallback_uv, night_uv);

    let mut no_time_world = WorldStore::new();
    no_time_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_time_icons = hotbar_item_icons(&no_time_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_time_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: night_uv.min,
            max: night_uv.max,
        }
    );

    let wobbled_stack = item_stack(1, 1);
    let wobbled_fallback_uv = item_runtime.icon_for_stack(&wobbled_stack).unwrap().layers[0].uv;
    let wobbled_uv = selected_at_time(&wobbled_stack, 18_000);
    assert_ne!(wobbled_uv, wobbled_fallback_uv);

    let random_stack = item_stack(2, 1);
    let random_fallback_uv = item_runtime.icon_for_stack(&random_stack).unwrap().layers[0].uv;
    let random_uv = selected_at_time(&random_stack, 18_000);
    assert_ne!(random_uv, random_fallback_uv);

    let moon_phase_stack = item_stack(3, 1);
    let moon_phase_fallback_uv = item_runtime
        .icon_for_stack(&moon_phase_stack)
        .unwrap()
        .layers[0]
        .uv;
    let new_moon_uv = selected_at_time(&moon_phase_stack, 96_000);
    assert_ne!(new_moon_uv, moon_phase_fallback_uv);

    let mut moon_phase_world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut moon_phase_world, 96_000);
    moon_phase_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: moon_phase_stack,
    });
    let moon_phase_icons = hotbar_item_icons(&moon_phase_world, Some(&item_runtime), 0.0);
    assert_eq!(
        moon_phase_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: new_moon_uv.min,
            max: new_moon_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_spawn_compass_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-spawn-compass-range-dispatch");
    write_runtime_spawn_compass_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let east_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_and_time_context(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            Some("minecraft:overworld"),
            None,
            Some(bbb_item_model::ItemModelCompassContext {
                game_time: 18_000,
                level_dimension: "minecraft:overworld",
                owner_position: [0.5, 64.0, 0.5],
                owner_y_rot_degrees: 0.0,
                spawn: Some(bbb_item_model::ItemModelCompassTarget {
                    dimension: "minecraft:overworld",
                    pos: [10, 64, 0],
                }),
                recovery: None,
            }),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, east_uv);

    let wobbled_stack = item_stack(1, 1);
    let wobbled_fallback_uv = item_runtime.icon_for_stack(&wobbled_stack).unwrap().layers[0].uv;
    let wobbled_east_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_and_time_context(
            &wobbled_stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            Some("minecraft:overworld"),
            None,
            Some(bbb_item_model::ItemModelCompassContext {
                game_time: 18_000,
                level_dimension: "minecraft:overworld",
                owner_position: [0.5, 64.0, 0.5],
                owner_y_rot_degrees: 0.0,
                spawn: Some(bbb_item_model::ItemModelCompassTarget {
                    dimension: "minecraft:overworld",
                    pos: [10, 64, 0],
                }),
                recovery: None,
            }),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(wobbled_fallback_uv, wobbled_east_uv);

    let invalid_spin_stack = item_stack(2, 1);
    let invalid_spin_fallback_uv = item_runtime
        .icon_for_stack(&invalid_spin_stack)
        .unwrap()
        .layers[0]
        .uv;
    let mut invalid_spin_world = world_with_dimension(0, "minecraft:overworld");
    invalid_spin_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    set_default_spawn(&mut invalid_spin_world, "minecraft:the_nether", [10, 64, 0]);
    invalid_spin_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: invalid_spin_stack,
    });
    let invalid_spin_icons = hotbar_item_icons(&invalid_spin_world, Some(&item_runtime), 0.0);
    assert_ne!(
        invalid_spin_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: invalid_spin_fallback_uv.min,
            max: invalid_spin_fallback_uv.max,
        }
    );

    let no_target_spin_stack = item_stack(3, 1);
    let no_target_spin_fallback_uv = item_runtime
        .icon_for_stack(&no_target_spin_stack)
        .unwrap()
        .layers[0]
        .uv;
    let mut no_target_spin_world = world_with_dimension(0, "minecraft:overworld");
    no_target_spin_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    no_target_spin_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: no_target_spin_stack,
    });
    let no_target_spin_icons = hotbar_item_icons(&no_target_spin_world, Some(&item_runtime), 0.0);
    assert_ne!(
        no_target_spin_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: no_target_spin_fallback_uv.min,
            max: no_target_spin_fallback_uv.max,
        }
    );

    let mut no_pose_world = world_with_dimension(0, "minecraft:overworld");
    set_default_spawn(&mut no_pose_world, "minecraft:overworld", [10, 64, 0]);
    no_pose_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_pose_icons = hotbar_item_icons(&no_pose_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_pose_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    set_default_spawn(&mut world, "minecraft:overworld", [10, 64, 0]);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: east_uv.min,
            max: east_uv.max,
        }
    );

    let mut wrong_dimension_world = world_with_dimension(0, "minecraft:overworld");
    wrong_dimension_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    set_default_spawn(
        &mut wrong_dimension_world,
        "minecraft:the_nether",
        [10, 64, 0],
    );
    wrong_dimension_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let wrong_dimension_icons = hotbar_item_icons(&wrong_dimension_world, Some(&item_runtime), 0.0);
    assert_eq!(
        wrong_dimension_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_lodestone_compass_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-lodestone-compass-range-dispatch");
    write_runtime_lodestone_compass_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let lodestone_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![67],
            lodestone_target: Some(bbb_protocol::packets::LodestoneTargetSummary {
                dimension: "minecraft:overworld".to_string(),
                pos: ProtocolBlockPos { x: 10, y: 64, z: 0 },
            }),
            ..DataComponentPatchSummary::default()
        },
    };
    let fallback_uv = item_runtime
        .icon_for_stack(&lodestone_stack)
        .unwrap()
        .layers[0]
        .uv;
    let east_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_and_time_context(
            &lodestone_stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            Some("minecraft:overworld"),
            None,
            Some(bbb_item_model::ItemModelCompassContext {
                game_time: 18_000,
                level_dimension: "minecraft:overworld",
                owner_position: [0.5, 64.0, 0.5],
                owner_y_rot_degrees: 0.0,
                spawn: None,
                recovery: None,
            }),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, east_uv);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: lodestone_stack.clone(),
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: east_uv.min,
            max: east_uv.max,
        }
    );

    let mut missing_component_world = world_with_dimension(0, "minecraft:overworld");
    missing_component_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    missing_component_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(0, 1),
    });
    let missing_component_icons =
        hotbar_item_icons(&missing_component_world, Some(&item_runtime), 0.0);
    assert_ne!(
        missing_component_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut wrong_dimension_stack = lodestone_stack;
    wrong_dimension_stack
        .component_patch
        .lodestone_target
        .as_mut()
        .unwrap()
        .dimension = "minecraft:the_nether".to_string();
    let mut wrong_dimension_world = world_with_dimension(0, "minecraft:overworld");
    wrong_dimension_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    wrong_dimension_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: wrong_dimension_stack,
    });
    let wrong_dimension_icons = hotbar_item_icons(&wrong_dimension_world, Some(&item_runtime), 0.0);
    assert_eq!(
        wrong_dimension_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_recovery_compass_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-recovery-compass-range-dispatch");
    write_runtime_recovery_compass_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime.icon_for_stack(&stack).unwrap().layers[0].uv;
    let east_uv = item_runtime
        .icon_for_stack_with_context_and_use_context_and_time_context(
            &stack,
            None,
            false,
            bbb_item_model::ItemModelUseContext::inactive(),
            bbb_pack::BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            Some("minecraft:player"),
            Some("minecraft:overworld"),
            None,
            Some(bbb_item_model::ItemModelCompassContext {
                game_time: 18_000,
                level_dimension: "minecraft:overworld",
                owner_position: [0.5, 64.0, 0.5],
                owner_y_rot_degrees: 0.0,
                spawn: None,
                recovery: Some(bbb_item_model::ItemModelCompassTarget {
                    dimension: "minecraft:overworld",
                    pos: [10, 64, 0],
                }),
            }),
        )
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, east_uv);

    let mut no_pose_world = world_with_dimension_last_death_location(
        0,
        "minecraft:overworld",
        Some(("minecraft:overworld", [10, 64, 0])),
    );
    no_pose_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_pose_icons = hotbar_item_icons(&no_pose_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_pose_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension_last_death_location(
        0,
        "minecraft:overworld",
        Some(("minecraft:overworld", [10, 64, 0])),
    );
    world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: east_uv.min,
            max: east_uv.max,
        }
    );

    let mut missing_recovery_world =
        world_with_dimension_last_death_location(0, "minecraft:overworld", None);
    missing_recovery_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    missing_recovery_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let missing_recovery_icons =
        hotbar_item_icons(&missing_recovery_world, Some(&item_runtime), 0.0);
    assert_eq!(
        missing_recovery_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut wrong_dimension_world = world_with_dimension_last_death_location(
        0,
        "minecraft:overworld",
        Some(("minecraft:the_nether", [10, 64, 0])),
    );
    wrong_dimension_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    wrong_dimension_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let wrong_dimension_icons = hotbar_item_icons(&wrong_dimension_world, Some(&item_runtime), 0.0);
    assert_eq!(
        wrong_dimension_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn dropped_item_models_use_world_trim_material_select_context() {
    let root = unique_runtime_temp_dir("dropped-trim-material-select");
    write_runtime_trim_material_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    record_trim_material_registry(&mut world);
    world.apply_add_entity(test_add_entity(700, 71));
    let mut stack = item_stack(0, 1);
    stack.component_patch.armor_trim_material_id = Some(0);
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 700,
            values: vec![test_item_stack_data(8, stack)],
        })
    );
    let terrain_textures = crate::terrain_runtime::TerrainTextureState::default();
    let fallback = crate::item_models::dropped_item_models(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        0.0,
        None,
        None,
        None,
        None,
    );
    let trim_material_keys = world_trim_material_keys(&world).unwrap();
    let trimmed = crate::item_models::dropped_item_models(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        0.0,
        Some(&trim_material_keys),
        None,
        None,
        None,
    );

    assert!(fallback.flat_meshes.is_empty());
    assert!(trimmed.flat_meshes.is_empty());
    assert_eq!(fallback.residency.assets.len(), 1);
    assert_eq!(trimmed.residency.assets.len(), 1);
    assert_eq!(fallback.handled_entity_ids, trimmed.handled_entity_ids);
    assert_ne!(fallback.residency.assets, trimmed.residency.assets);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_frame_models_use_world_trim_material_select_context() {
    let root = unique_runtime_temp_dir("item-frame-trim-material-select");
    write_runtime_trim_material_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    record_trim_material_registry(&mut world);
    world.apply_add_entity(test_add_entity(701, 73));
    let mut stack = item_stack(0, 1);
    stack.component_patch.armor_trim_material_id = Some(0);
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 701,
            values: vec![test_item_stack_data(9, stack)],
        })
    );
    let terrain_textures = crate::terrain_runtime::TerrainTextureState::default();
    let fallback = crate::item_frames::item_frame_models(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        None,
        None,
        None,
        None,
    );
    let trim_material_keys = world_trim_material_keys(&world).unwrap();
    let trimmed = crate::item_frames::item_frame_models(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        Some(&trim_material_keys),
        None,
        None,
        None,
    );

    assert!(fallback.flat_meshes.is_empty());
    assert!(trimmed.flat_meshes.is_empty());
    let fallback_item_assets = fallback
        .residency
        .assets
        .iter()
        .filter(|asset| asset.atlas == bbb_renderer::ItemModelAtlasKind::Items)
        .collect::<Vec<_>>();
    let trimmed_item_assets = trimmed
        .residency
        .assets
        .iter()
        .filter(|asset| asset.atlas == bbb_renderer::ItemModelAtlasKind::Items)
        .collect::<Vec<_>>();
    assert_eq!(fallback_item_assets.len(), 1);
    assert_eq!(trimmed_item_assets.len(), 1);
    assert_ne!(fallback_item_assets, trimmed_item_assets);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_hovered_item_tooltip_name() {
    let root = unique_runtime_temp_dir("inventory-tooltip");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: item_stack(0, 1),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();

    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            lines: vec![tooltip_name_line(
                "Test Combo",
                TOOLTIP_TEST_WHITE,
                0xFF_FF_FF,
                false
            )],
        })
    );

    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:trade_rebalance".to_string()],
    });
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(
        screen.tooltip.as_ref().unwrap().lines,
        vec![
            tooltip_name_line("Test Combo", TOOLTIP_TEST_WHITE, 0xFF_FF_FF, false),
            tooltip_plain_line("Disabled item", TOOLTIP_TEST_RED),
        ]
    );
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });

    let mut damaged_stack = item_stack(0, 1);
    damaged_stack.component_patch.max_damage = Some(20);
    damaged_stack.component_patch.damage = Some(3);
    damaged_stack.component_patch.added_type_ids = vec![2, 3];
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: damaged_stack,
    });
    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(36),
        InventoryHudLocalState {
            advanced_item_tooltips: true,
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();
    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            lines: vec![
                tooltip_name_line("Test Combo", TOOLTIP_TEST_WHITE, 0xFF_FF_FF, false),
                tooltip_plain_line("Durability: 17 / 20", TOOLTIP_TEST_WHITE),
                tooltip_plain_line("minecraft:test_combo", TOOLTIP_TEST_DARK_GRAY),
                tooltip_plain_line("14 component(s)", TOOLTIP_TEST_DARK_GRAY),
            ],
        })
    );

    let mut custom_stack = item_stack(0, 1);
    custom_stack.component_patch.custom_name = Some("Custom Combo".to_string());
    custom_stack.component_patch.rarity = Some(bbb_protocol::packets::ItemRaritySummary::Rare);
    custom_stack.component_patch.lore = vec!["First lore".to_string(), "Second lore".to_string()];
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: custom_stack,
    });
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();

    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            lines: vec![
                tooltip_name_line("Custom Combo", TOOLTIP_TEST_AQUA, 0x55_FF_FF, true),
                tooltip_lore_line("First lore"),
                tooltip_lore_line("Second lore"),
            ],
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_tooltip_uses_local_player_base_attributes_for_default_weapon() {
    let root = unique_runtime_temp_dir("inventory-default-attribute-tooltip");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(
        world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
            entity_id: 42,
            attributes: vec![
                bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                    base: 2.5,
                    modifiers: Vec::new(),
                },
                bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
                    base: 5.0,
                    modifiers: Vec::new(),
                },
            ],
        })
    );
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: item_stack(1, 1),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            lines: vec![
                tooltip_name_line("Test Sword", TOOLTIP_TEST_WHITE, 0xFF_FF_FF, false),
                tooltip_plain_line("", TOOLTIP_TEST_WHITE),
                tooltip_plain_line("When in Main Hand:", TOOLTIP_TEST_GRAY),
                tooltip_plain_line(" 7.5 Attack Damage", TOOLTIP_TEST_DARK_GREEN),
                tooltip_plain_line(" 2.6 Attack Speed", TOOLTIP_TEST_DARK_GREEN),
            ],
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_tooltip_follows_synced_enchantment_holder_order_and_effects() {
    let root = unique_runtime_temp_dir("inventory-enchantment-attribute-tooltip");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "example:decoy".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "example:efficiency".to_string(),
                raw_data: Some(efficiency_enchantment_attribute_nbt()),
            },
        ],
        raw_payload_len: 0,
    });
    let mut stack = item_stack(0, 1);
    stack.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 1,
        level: 1,
    }];
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack,
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            lines: vec![
                tooltip_name_line("Test Combo", TOOLTIP_TEST_AQUA, 0x55_FF_FF, false),
                tooltip_enchantment_line("Synced Efficiency", "I", 0x12_34_56),
                tooltip_plain_line("", TOOLTIP_TEST_WHITE),
                tooltip_plain_line("When in Main Hand:", TOOLTIP_TEST_GRAY),
                tooltip_plain_line("+2 Mining Efficiency", TOOLTIP_TEST_BLUE),
            ],
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_tooltip_gates_operator_nbt_warning_on_local_permission() {
    let root = unique_runtime_temp_dir("inventory-operator-nbt-warning");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    let command_block = bbb_protocol::packets::ItemStackSummary {
        item_id: Some(2),
        count: 1,
        component_patch: DataComponentPatchSummary {
            block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID),
            ..DataComponentPatchSummary::default()
        },
    };
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: command_block.clone(),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(screen.tooltip.as_ref().unwrap().lines.len(), 1);

    assert!(
        world.apply_entity_event(bbb_protocol::packets::EntityEvent {
            entity_id: 42,
            event_id: 26,
        })
    );
    let warning_lines = vec![
        tooltip_operator_warning_line("Operator data", true),
        tooltip_operator_warning_line("Can alter commands", false),
        tooltip_operator_warning_line("Use with care", false),
    ];
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    let mut expected_lines = vec![tooltip_name_line(
        "Command Block",
        TOOLTIP_TEST_WHITE,
        0xFF_FF_FF,
        false,
    )];
    expected_lines.extend(warning_lines.clone());
    assert_eq!(screen.tooltip.as_ref().unwrap().lines, expected_lines);

    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: bbb_protocol::packets::ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                tooltip_hide_tooltip: true,
                ..command_block.component_patch.clone()
            },
            ..command_block
        },
    });
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(screen.tooltip.as_ref().unwrap().lines, warning_lines);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_generic_container_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        title: "Large Chest".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 90],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(89), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 222);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::GenericContainer,
                0,
                0,
                176,
                125,
                [0.0, 0.0],
                [176.0 / 256.0, 125.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::GenericContainer,
                0,
                125,
                176,
                96,
                [0.0, 126.0 / 256.0],
                [176.0 / 256.0, 222.0 / 256.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(89));
    assert_eq!(screen.slots.len(), 90);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (8, 18));
    let player_inventory = screen.slots.iter().find(|slot| slot.slot_id == 54).unwrap();
    assert_eq!((player_inventory.x, player_inventory.y), (8, 139));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 89).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 197));
}

#[test]
fn hud_inventory_screen_projects_generic_3x3_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: "Dispenser".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 45],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(44), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Dispenser,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(44));
    assert_eq!(screen.slots.len(), 45);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (62, 17));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 44).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_crafter_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: "Crafter".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Crafter,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CrafterUnpoweredRedstone,
                97,
                35,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(45));
    assert_eq!(screen.slots.len(), 46);
    let first_grid = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_grid.x, first_grid.y), (26, 17));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((result.x, result.y), (134, 35));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 44).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_crafter_state_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: "Crafter".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 8,
        value: 1,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 9,
        value: 1,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Crafter,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CrafterDisabledSlot,
                25,
                16,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CrafterDisabledSlot,
                61,
                52,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CrafterPoweredRedstone,
                97,
                35,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_crafting_table_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CraftingTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookButton,
                5,
                34,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(45));
    assert_eq!(screen.slots.len(), 46);
    let result = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((result.x, result.y), (124, 35));
    let first_grid = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((first_grid.x, first_grid.y), (30, 17));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_highlights_recipe_book_button() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((5, 34)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.background_layers.last().map(|layer| layer.texture),
        Some(HudInventoryBackgroundTexture::RecipeBookButtonHighlighted)
    );
}

#[test]
fn hud_inventory_screen_projects_recipe_book_overlay_for_crafting_table() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

    assert_eq!(screen.width, 320);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBook,
                0,
                0,
                147,
                166,
                [1.0 / 256.0, 1.0 / 256.0],
                [148.0 / 256.0, 167.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CraftingTable,
                149,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookTabSelected,
                -32,
                3,
                35,
                27,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::WidgetTextField,
                25,
                13,
                81,
                14,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookButton,
                154,
                34,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookFilterDisabled,
                110,
                12,
                26,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(45));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((result.x, result.y), (273, 35));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (301, 142));
}

#[test]
fn hud_inventory_screen_projects_narrow_recipe_book_over_main_gui() {
    let world = open_recipe_book_crafting_table_world();

    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &TerrainTextureState::default(),
        Some(45),
        InventoryHudLocalState::default(),
        winit::dpi::PhysicalSize::new(378, 720),
        0.0,
    )
    .unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(|layer| layer.texture == HudInventoryBackgroundTexture::CraftingTable)
            .map(|layer| layer.x),
        Some(0)
    );
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(|layer| layer.texture == HudInventoryBackgroundTexture::RecipeBook)
            .map(|layer| layer.x),
        Some(14)
    );
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(|layer| layer.texture == HudInventoryBackgroundTexture::RecipeBookButton)
            .map(|layer| layer.x),
        Some(5)
    );
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(|layer| layer.texture == HudInventoryBackgroundTexture::WidgetTextField)
            .map(|layer| layer.x),
        Some(39)
    );
    let result = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((result.x, result.y), (124, 35));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_crafting_table_ghost_recipe_slots() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShaped,
            raw_body: Vec::new(),
            crafting: Some(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shaped {
                    width: 2,
                    height: 1,
                    ingredients: vec![
                        stonecutter_item_display(1),
                        stonecutter_item_stack_display(2, 3),
                    ],
                    result: stonecutter_item_stack_display(0, 2),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: Vec::new(),
                        item_stack: None,
                        tag: None,
                    },
                },
            ),
            furnace: None,
        },
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::BeforeGhostItem)
            .copied()
            .collect::<Vec<_>>(),
        vec![
            HudInventoryFillLayer {
                x: 269,
                y: 31,
                width: 24,
                height: 24,
                tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
                stage: HudInventoryFillStage::BeforeGhostItem,
            },
            HudInventoryFillLayer {
                x: 179,
                y: 35,
                width: 16,
                height: 16,
                tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
                stage: HudInventoryFillStage::BeforeGhostItem,
            },
            HudInventoryFillLayer {
                x: 197,
                y: 35,
                width: 16,
                height: 16,
                tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
                stage: HudInventoryFillStage::BeforeGhostItem,
            },
        ]
    );
    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::AfterGhostItem)
            .map(|layer| (layer.x, layer.y, layer.width, layer.height, layer.tint))
            .collect::<Vec<_>>(),
        vec![
            (273, 35, 16, 16, RECIPE_BOOK_GHOST_POST_ITEM_TINT),
            (179, 35, 16, 16, RECIPE_BOOK_GHOST_POST_ITEM_TINT),
            (197, 35, 16, 16, RECIPE_BOOK_GHOST_POST_ITEM_TINT),
        ]
    );
    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (
                item.x,
                item.y,
                item.draw_decorations,
                item.icon.count_label.clone()
            ))
            .collect::<Vec<_>>(),
        vec![
            (273, 35, true, Some(HudItemCountLabel::new("2"))),
            (179, 35, false, None),
            (197, 35, false, None),
        ]
    );

    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 99,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![stonecutter_item_display(1)],
                    result: stonecutter_item_display(0),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: Vec::new(),
                        item_stack: None,
                        tag: None,
                    },
                },
            ),
            furnace: None,
        },
    });
    let stale = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    assert_eq!(stale.fill_layers, screen.fill_layers);
    assert_eq!(stale.ghost_items, screen.ghost_items);
    assert_eq!(world.counters().ghost_recipe_packets, 2);
    assert_eq!(world.counters().ghost_recipe_updates_applied, 1);
    assert_eq!(world.counters().ghost_recipe_updates_ignored, 1);
}

#[test]
fn hud_inventory_screen_projects_furnace_ghost_recipe_slots() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_furnace_world();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: furnace_recipe_display(2, 1, 0),
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::BeforeGhostItem)
            .map(|layer| (layer.x, layer.y, layer.width, layer.height))
            .collect::<Vec<_>>(),
        vec![(265, 35, 16, 16), (205, 17, 16, 16), (205, 53, 16, 16)]
    );
    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(265, 35, true), (205, 17, false), (205, 53, false)]
    );

    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 1,
        item: item_stack(1, 1),
    });
    let fuel_occupied = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    assert_eq!(
        fuel_occupied
            .ghost_items
            .iter()
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(265, 35, true), (205, 17, false)]
    );
}

#[test]
fn hud_inventory_screen_resolves_tag_ghost_recipe_ingredients() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![2])]);
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_tag("minecraft:planks")],
                    result: stonecutter_item_display(0),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: Vec::new(),
                        item_stack: None,
                        tag: None,
                    },
                },
            ),
            furnace: None,
        },
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (
                item.x,
                item.y,
                item.draw_decorations,
                item.icon.count_label.clone()
            ))
            .collect::<Vec<_>>(),
        vec![(273, 35, true, None), (179, 17, false, None)]
    );
}

#[test]
fn hud_inventory_screen_cycles_tag_ghost_recipe_ingredients() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![1, 2])]);
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_tag("minecraft:planks")],
                    result: stonecutter_item_display(0),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: Vec::new(),
                        item_stack: None,
                        tag: None,
                    },
                },
            ),
            furnace: None,
        },
    });

    let first_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let first_icon = first_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("first tag ghost ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let second_icon = second_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("second tag ghost ingredient");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_reads_replaced_and_removed_tag_ghost_members() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_tag("minecraft:planks")],
                    result: stonecutter_item_display(0),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: Vec::new(),
                        item_stack: None,
                        tag: None,
                    },
                },
            ),
            furnace: None,
        },
    });

    let ghost_icon = |world: &WorldStore| {
        hud_inventory_screen(world, Some(&item_runtime), None, 0.0)
            .unwrap()
            .ghost_items
            .into_iter()
            .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
            .map(|item| item.icon)
    };

    apply_item_tags(&mut world, vec![("minecraft:planks", vec![1, 2])]);
    let first = ghost_icon(&world).expect("first ordered tag member");

    apply_item_tags(&mut world, vec![("minecraft:planks", vec![2, 1])]);
    let reordered = ghost_icon(&world).expect("replacement tag member");
    assert_ne!(reordered, first);

    apply_item_tags(&mut world, Vec::new());
    assert_eq!(ghost_icon(&world), None);
}

#[test]
fn hud_inventory_screen_cycles_composite_ghost_recipe_ingredients() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_composite(vec![
                        slot_display_with_remainder(
                            stonecutter_item_display(1),
                            stonecutter_item_display(0),
                        ),
                        stonecutter_item_display(2),
                    ])],
                    result: stonecutter_item_display(0),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: Vec::new(),
                        item_stack: None,
                        tag: None,
                    },
                },
            ),
            furnace: None,
        },
    });

    let first_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let first_icon = first_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("first composite ghost ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let second_icon = second_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("second composite ghost ingredient");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_projects_local_inventory_ghost_result_without_big_slot_fill() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = WorldStore::new();
    world.open_local_inventory();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 0,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![stonecutter_item_display(1)],
                    result: stonecutter_item_display(0),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: Vec::new(),
                        item_stack: None,
                        tag: None,
                    },
                },
            ),
            furnace: None,
        },
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::BeforeGhostItem)
            .map(|layer| (layer.x, layer.y, layer.width, layer.height))
            .collect::<Vec<_>>(),
        vec![(154, 28, 16, 16), (98, 18, 16, 16)]
    );
    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(154, 28, true), (98, 18, false)]
    );
}

#[test]
fn hud_inventory_screen_projects_recipe_book_search_box_text() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_search: RecipeBookSearchHudState {
                text: "axe".to_string(),
                cursor: 2,
                selection: 1,
                focused: true,
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::WidgetTextFieldHighlighted,
                25,
                13,
                81,
                14,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    let label = screen.text_labels.last().unwrap();
    assert_eq!(label.x, 29);
    assert_eq!(label.y, 16);
    assert_eq!(label.width, 73);
    assert_eq!(label.text, "axe");
    assert_eq!(label.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(label.background, None);
    assert!(!label.shadow);
    assert!(label.runs.is_empty());
    let input = label.input.unwrap();
    assert_eq!(input.cursor, 2);
    assert_eq!(input.selection, 1);
    assert_eq!(input.scroll_to, 1);
    assert_eq!(input.max_length, 50);
    assert_eq!(input.cursor_tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(input.selection_tint, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn hud_inventory_screen_projects_selected_recipe_book_tab() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![shapeless_crafting_recipe_book_entry(20, 2, None, 120)],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookTabSelected,
                -32,
                30,
                35,
                27,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_animates_highlighted_recipe_book_tab() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = shapeless_crafting_recipe_book_entry(20, 2, None, 120);
    entry.flags |= 0b10;
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });
    world.advance_recipe_book_tab_animation(7);

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.5,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookTabSelected,
                -32,
                29,
                35,
                30,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));

    let primary_icon = screen
        .floating_items
        .iter()
        .find(|item| item.x == -29)
        .expect("selected recipe book tab primary icon");
    assert_eq!(primary_icon.y, 34);
    assert_eq!(primary_icon.scale, 1.0);
    assert!((primary_icon.scale_y - 1.1).abs() <= 0.000001);

    let secondary_icon = screen
        .floating_items
        .iter()
        .find(|item| item.x == -18)
        .expect("selected recipe book tab secondary icon");
    assert_eq!(secondary_icon.y, 34);
    assert_eq!(secondary_icon.scale, 1.0);
    assert!((secondary_icon.scale_y - 1.1).abs() <= 0.000001);
}

#[test]
fn hud_inventory_screen_projects_crafting_recipe_book_buttons() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![bbb_protocol::packets::RecipeBookAddEntry {
            contents: bbb_protocol::packets::RecipeDisplayEntry {
                id: bbb_protocol::packets::RecipeDisplayId { index: 42 },
                display: bbb_protocol::packets::RecipeDisplaySummary {
                    display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
                    raw_body: Vec::new(),
                    crafting: Some(
                        bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                            ingredients: Vec::new(),
                            result: bbb_protocol::packets::SlotDisplaySummary {
                                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                                raw_payload: Vec::new(),
                                item_stack: Some(item_stack(99, 1)),
                                tag: None,
                            },
                            crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                                raw_payload: Vec::new(),
                                item_stack: None,
                                tag: None,
                            },
                        },
                    ),
                    furnace: None,
                },
                group: None,
                category_id: VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
                crafting_requirements: None,
            },
            flags: 0,
            notification: false,
            highlight: false,
        }],
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotUncraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_projects_furnace_recipe_book_buttons() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_furnace_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![furnace_recipe_book_entry(42, 4, None, 1)],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                furnace: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.width, 320);
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotUncraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    assert!(screen
        .floating_items
        .iter()
        .any(|item| (item.x, item.y) == (15, 35)));
}

#[test]
fn hud_inventory_screen_counts_furnace_slots_for_recipe_book_craftability() {
    let mut world = open_recipe_book_furnace_world();
    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 1,
        item: item_stack(2, 1),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![furnace_recipe_book_entry_with_requirements(
            42,
            4,
            None,
            1,
            vec![vec![2]],
        )],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                furnace: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotCraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_draws_same_result_recipe_book_multi_recipe_offset_icons() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, Some(7), 1),
            shapeless_crafting_recipe_book_entry(21, 2, Some(7), 1),
        ],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    let item_positions: Vec<(i32, i32)> = screen
        .floating_items
        .iter()
        .map(|item| (item.x, item.y))
        .collect();
    assert!(item_positions.contains(&(16, 36)));
    assert!(item_positions.contains(&(14, 34)));
    assert!(!item_positions.contains(&(15, 35)));
}

#[test]
fn hud_inventory_screen_cycles_recipe_book_multi_recipe_icon() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, Some(7), 1),
            shapeless_crafting_recipe_book_entry(21, 2, Some(7), 2),
        ],
    });
    let local_state = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        ..InventoryHudLocalState::default()
    };

    let first_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let first_icon = first_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (15, 35))
        .map(|item| item.icon.clone())
        .expect("first multi-recipe icon");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap();
    let second_icon = second_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (15, 35))
        .map(|item| item.icon.clone())
        .expect("second multi-recipe icon");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_projects_recipe_book_overlay_picker() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                20,
                2,
                Some(7),
                1,
                vec![stonecutter_item_display(1)],
            ),
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                21,
                2,
                Some(7),
                2,
                vec![stonecutter_item_display(2)],
            ),
        ],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            recipe_book_overlay: Some(RecipeBookOverlayHudState {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                tab_index: 1,
                page_index: 0,
                button_index: 0,
                x: 11,
                y: 31,
            }),
            cursor_position: Some((41, 37)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookOverlayRecipe,
                11,
                31,
                58,
                33,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookCraftingOverlayDisabled,
                15,
                36,
                24,
                24,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookCraftingOverlayDisabledHighlighted,
                40,
                36,
                24,
                24,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    let all_item_positions: Vec<(i32, i32, f32)> = screen
        .floating_items
        .iter()
        .map(|item| (item.x, item.y, item.scale))
        .collect();
    let item_positions: Vec<(i32, i32)> = screen
        .floating_items
        .iter()
        .filter(|item| (item.scale - RECIPE_BOOK_OVERLAY_ITEM_SCALE).abs() < 1e-6)
        .map(|item| {
            assert!(!item.draw_decorations);
            (item.x, item.y)
        })
        .collect();
    assert!(
        item_positions.contains(&(17, 38)),
        "overlay item positions: {item_positions:?}; all positions: {all_item_positions:?}"
    );
    assert!(
        item_positions.contains(&(42, 38)),
        "overlay item positions: {item_positions:?}; all positions: {all_item_positions:?}"
    );
}

#[test]
fn hud_inventory_screen_cycles_recipe_book_overlay_tag_ingredient_icon() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![1, 2])]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                20,
                2,
                Some(7),
                1,
                vec![slot_display_tag("minecraft:planks")],
            ),
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                21,
                2,
                Some(7),
                2,
                vec![slot_display_tag("minecraft:planks")],
            ),
        ],
    });
    let local_state = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        recipe_book_overlay: Some(RecipeBookOverlayHudState {
            book_type: bbb_protocol::packets::RecipeBookType::Crafting,
            tab_index: 1,
            page_index: 0,
            button_index: 0,
            x: 11,
            y: 31,
        }),
        ..InventoryHudLocalState::default()
    };

    let first_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let first_icon = first_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("first tag overlay ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap();
    let second_icon = second_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("second tag overlay ingredient");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_cycles_composite_recipe_book_overlay_ingredient_icon() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                20,
                2,
                Some(7),
                1,
                vec![slot_display_composite(vec![
                    slot_display_with_remainder(
                        stonecutter_item_display(1),
                        stonecutter_item_display(0),
                    ),
                    stonecutter_item_display(2),
                ])],
            ),
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                21,
                2,
                Some(7),
                2,
                vec![stonecutter_item_display(2)],
            ),
        ],
    });
    let local_state = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        recipe_book_overlay: Some(RecipeBookOverlayHudState {
            book_type: bbb_protocol::packets::RecipeBookType::Crafting,
            tab_index: 1,
            page_index: 0,
            button_index: 0,
            x: 11,
            y: 31,
        }),
        ..InventoryHudLocalState::default()
    };

    let first_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let first_icon = first_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("first composite overlay ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap();
    let second_icon = second_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("second composite overlay ingredient");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_filters_recipe_book_buttons_by_crafting_tab_category() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 120),
            shapeless_crafting_recipe_book_entry(21, 3, None, 121),
        ],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    let recipe_slots = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::RecipeBookSlotUncraftable
                    | HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable
            )
        })
        .count();
    assert_eq!(recipe_slots, 1);
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotUncraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_filters_recipe_book_buttons_by_search_text() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 0),
            shapeless_crafting_recipe_book_entry(21, 2, None, 1),
            shapeless_crafting_recipe_book_entry(22, 2, None, 2),
        ],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_search: RecipeBookSearchHudState {
                text: "stick".to_string(),
                cursor: 5,
                selection: 5,
                focused: true,
            },
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    let recipe_slots = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::RecipeBookSlotUncraftable
                    | HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable
            )
        })
        .count();
    assert_eq!(recipe_slots, 1);
    assert!(screen
        .floating_items
        .iter()
        .any(|item| (item.x, item.y) == (15, 35)));
}

#[test]
fn recipe_book_plain_search_uses_result_tooltip_text_only() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 0),
            shapeless_crafting_recipe_book_entry(21, 2, None, 1),
            shapeless_crafting_recipe_book_entry(22, 2, None, 2),
        ],
    });

    assert_eq!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "wooden"),
        vec![22]
    );
    assert!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "oak_planks").is_empty()
    );
}

#[test]
fn recipe_book_identifier_search_intersects_namespace_with_path_or_tooltip() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 0),
            shapeless_crafting_recipe_book_entry(21, 2, None, 1),
            shapeless_crafting_recipe_book_entry(22, 2, None, 2),
        ],
    });

    assert_eq!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "minecraft:oak_planks"),
        vec![22]
    );
    assert_eq!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "minecraft:wooden"),
        vec![22]
    );
    assert!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "bbb:wooden").is_empty()
    );
    assert!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "minecraft:2").is_empty()
    );
}

#[test]
fn hud_inventory_screen_marks_and_filters_craftable_recipe_book_buttons() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 10,
        item: item_stack(50, 1),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_requirements(
                20,
                2,
                None,
                120,
                vec![vec![50]],
            ),
            shapeless_crafting_recipe_book_entry_with_requirements(
                21,
                2,
                None,
                121,
                vec![vec![51]],
            ),
        ],
    });

    let all_recipes = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(all_recipes.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotCraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    assert!(all_recipes.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotUncraftable,
                36,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));

    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: true,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    let craftable_recipes = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    let recipe_slots = craftable_recipes
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::RecipeBookSlotCraftable
                    | HudInventoryBackgroundTexture::RecipeBookSlotUncraftable
                    | HudInventoryBackgroundTexture::RecipeBookSlotManyCraftable
                    | HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable
            )
        })
        .count();
    assert_eq!(recipe_slots, 1);
    assert!(craftable_recipes.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotCraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_marks_tagged_recipe_book_requirements_craftable() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 10,
        item: item_stack(50, 1),
    });
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![50])]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_requirement_summaries(
                20,
                2,
                None,
                120,
                vec![bbb_protocol::packets::IngredientSummary {
                    tag: Some("minecraft:planks".to_string()),
                    item_ids: Vec::new(),
                }],
            ),
        ],
    });
    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookSlotCraftable,
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_projects_recipe_book_page_controls_and_current_page() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| shapeless_crafting_recipe_book_entry(index, 2, None, 200 + index))
            .collect(),
    });

    let first_page = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        first_page
            .background_layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::RecipeBookSlotUncraftable
                        | HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable
                )
            })
            .count(),
        20
    );
    assert!(first_page
        .background_layers
        .iter()
        .any(|layer| { layer.texture == HudInventoryBackgroundTexture::RecipeBookPageForward }));
    assert!(!first_page
        .background_layers
        .iter()
        .any(|layer| { layer.texture == HudInventoryBackgroundTexture::RecipeBookPageBackward }));
    assert!(first_page
        .text_labels
        .iter()
        .any(|label| label.text == "1/2"));

    let second_page = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            recipe_book_pages: RecipeBookPageHudState {
                crafting: 1,
                ..RecipeBookPageHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        second_page
            .background_layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::RecipeBookSlotUncraftable
                        | HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable
                )
            })
            .count(),
        1
    );
    assert!(second_page
        .background_layers
        .iter()
        .any(|layer| { layer.texture == HudInventoryBackgroundTexture::RecipeBookPageBackward }));
    assert!(!second_page
        .background_layers
        .iter()
        .any(|layer| { layer.texture == HudInventoryBackgroundTexture::RecipeBookPageForward }));
    assert!(second_page
        .text_labels
        .iter()
        .any(|label| label.text == "2/2"));
}

#[test]
fn hud_inventory_screen_highlights_recipe_book_filter_button() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((110, 12)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.background_layers.last().map(|layer| layer.texture),
        Some(HudInventoryBackgroundTexture::RecipeBookFilterDisabledHighlighted)
    );
}

#[test]
fn hud_inventory_screen_uses_furnace_recipe_book_filter_sprite() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        furnace: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: true,
        },
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.width, 320);
    assert_eq!(
        screen.background_layers.last(),
        Some(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::RecipeBookFurnaceFilterEnabled,
            110,
            12,
            26,
            16,
            [0.0, 0.0],
            [1.0, 1.0],
        ))
    );
}

#[test]
fn hud_inventory_screen_projects_enchanting_table_layout_and_lapis_slot_layer() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 38],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(37), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableLapisSlot,
                35,
                47,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                60,
                14,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                60,
                33,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                60,
                52,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(37));
    assert_eq!(screen.slots.len(), 38);
    let item = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((item.x, item.y), (15, 47));
    let lapis = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((lapis.x, lapis.y), (35, 47));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_enchanting_table_enabled_option_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[1] = item_stack(42, 2);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 1,
        value: 12,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 12,
        total: 0,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlot,
            60,
            33,
            108,
            19,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::EnchantingTableLevel2,
            61,
            34,
            16,
            16,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert_eq!(
        screen.text_labels,
        vec![HudInventoryTextLabel {
            x: 154,
            y: 42,
            width: 12,
            text: "12".to_string(),
            tint: ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            runs: Vec::new(),
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_enchanting_table_disabled_cost_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[1] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 2,
        value: 30,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 40,
        total: 0,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
            60,
            52,
            108,
            19,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::EnchantingTableLevel3Disabled,
            61,
            53,
            16,
            16,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert_eq!(
        screen.text_labels,
        vec![HudInventoryTextLabel {
            x: 154,
            y: 61,
            width: 12,
            text: "30".to_string(),
            tint: ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            runs: Vec::new(),
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Anvil,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AnvilTextFieldDisabled,
                59,
                20,
                110,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(38));
    assert_eq!(screen.slots.len(), 39);
    let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((input.x, input.y), (27, 47));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((additional.x, additional.y), (76, 47));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (134, 47));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_beacon_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 37],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(36), 0.0).unwrap();

    assert_eq!(screen.width, 230);
    assert_eq!(screen.height, 219);
    assert_eq!(screen.background_layers.len(), 17);
    assert_eq!(
        screen.background_layers[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Beacon,
            0,
            0,
            230,
            219,
            [0.0, 0.0],
            [230.0 / 256.0, 219.0 / 256.0],
        )
    );
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButtonDisabled,
            53,
            22,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_SPEED_ID),
            55,
            24,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_REGENERATION_ID),
            146,
            49,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButtonDisabled,
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconConfirm,
            166,
            109,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButton,
            190,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconCancel,
            192,
            109,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert_eq!(screen.hovered_slot_id, Some(36));
    assert_eq!(screen.slots.len(), 37);
    let payment = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((payment.x, payment.y), (136, 110));
    let first_inventory = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((first_inventory.x, first_inventory.y), (36, 137));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (180, 195));
}

#[test]
fn hud_inventory_screen_projects_active_beacon_confirm_button() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 37];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: BEACON_PRIMARY_EFFECT_DATA_ID,
        value: 5,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButton,
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(!screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButtonDisabled,
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_local_beacon_effect_selection() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 37];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: BEACON_LEVELS_DATA_ID,
        value: 4,
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            beacon_effect_selection: Some((
                Some(VANILLA_MOB_EFFECT_STRENGTH_ID),
                Some(VANILLA_MOB_EFFECT_STRENGTH_ID),
            )),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.background_layers.len(), 19);
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButtonSelected,
            65,
            72,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_STRENGTH_ID),
            67,
            74,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButtonSelected,
            168,
            47,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_STRENGTH_ID),
            170,
            49,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BeaconButton,
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_anvil_text_field_and_error_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Anvil,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AnvilTextField,
                59,
                20,
                110,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AnvilError,
                99,
                45,
                28,
                21,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_rename_text_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            anvil_rename_text: Some("Sharp Pick".to_string()),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.text_labels,
        vec![HudInventoryTextLabel {
            x: 62,
            y: 24,
            width: 103,
            text: "Sharp Pick".to_string(),
            tint: ANVIL_RENAME_TEXT_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            runs: Vec::new(),
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_cost_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[2] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 7,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 8,
        total: 0,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.text_labels,
        vec![HudInventoryTextLabel {
            x: 126,
            y: ANVIL_COST_LABEL_Y,
            width: 40,
            text: "Cost: 7".to_string(),
            tint: ANVIL_COST_TEXT_COLOR,
            background: Some(HudInventoryTextBackground {
                x: 124,
                y: ANVIL_COST_BACKGROUND_Y,
                width: 44,
                height: ANVIL_COST_BACKGROUND_HEIGHT,
                tint: ANVIL_COST_BACKGROUND_TINT,
            }),
            input: None,
            scrolling: None,
            shadow: false,
            runs: Vec::new(),
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_too_expensive_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: ANVIL_TOO_EXPENSIVE_LEVEL_COST,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 100,
        total: 0,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.text_labels,
        vec![HudInventoryTextLabel {
            x: 84,
            y: ANVIL_COST_LABEL_Y,
            width: 82,
            text: "Too Expensive!".to_string(),
            tint: ANVIL_COST_ERROR_TEXT_COLOR,
            background: Some(HudInventoryTextBackground {
                x: 82,
                y: ANVIL_COST_BACKGROUND_Y,
                width: 86,
                height: ANVIL_COST_BACKGROUND_HEIGHT,
                tint: ANVIL_COST_BACKGROUND_TINT,
            }),
            input: None,
            scrolling: None,
            shadow: false,
            runs: Vec::new(),
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_brewing_stand_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: "Brewing Stand".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(40), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BrewingStand,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(40));
    assert_eq!(screen.slots.len(), 41);
    let bottle = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((bottle.x, bottle.y), (56, 51));
    let ingredient = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((ingredient.x, ingredient.y), (79, 17));
    let fuel = screen.slots.iter().find(|slot| slot.slot_id == 4).unwrap();
    assert_eq!((fuel.x, fuel.y), (17, 17));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 40).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_brewing_stand_progress_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: "Brewing Stand".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 200,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 1,
        value: 10,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BrewingStand,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BrewingStandFuelLength,
                60,
                44,
                9,
                4,
                [0.0, 0.0],
                [9.0 / 18.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BrewingStandBrewProgress,
                97,
                16,
                9,
                14,
                [0.0, 0.0],
                [1.0, 14.0 / 28.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BrewingStandBubbles,
                63,
                23,
                12,
                20,
                [0.0, 9.0 / 29.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_furnace_like_layouts() {
    for (menu_type_id, title, texture) in [
        (
            10,
            "Blast Furnace",
            HudInventoryBackgroundTexture::BlastFurnace,
        ),
        (14, "Furnace", HudInventoryBackgroundTexture::Furnace),
        (22, "Smoker", HudInventoryBackgroundTexture::Smoker),
    ] {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id,
            title: title.to_string(),
            title_styled: Vec::new(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    texture,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::RecipeBookButton,
                    20,
                    34,
                    20,
                    18,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(38));
        assert_eq!(screen.slots.len(), 39);
        let ingredient = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((ingredient.x, ingredient.y), (56, 17));
        let fuel = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((fuel.x, fuel.y), (56, 53));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((result.x, result.y), (116, 35));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }
}

#[test]
fn hud_inventory_screen_projects_furnace_progress_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 50,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 1,
        value: 200,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 2,
        value: 25,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 3,
        value: 100,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Furnace,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::FurnaceLitProgress,
                56,
                45,
                14,
                5,
                [0.0, 9.0 / 14.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::FurnaceBurnProgress,
                79,
                34,
                6,
                16,
                [0.0, 0.0],
                [6.0 / 24.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::RecipeBookButton,
                20,
                34,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_grindstone_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Grindstone,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(38));
    assert_eq!(screen.slots.len(), 39);
    let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((input.x, input.y), (49, 19));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((additional.x, additional.y), (49, 40));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (129, 34));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_grindstone_error_layer() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Grindstone,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::GrindstoneError,
                92,
                31,
                28,
                21,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_hopper_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: "Hopper".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(40), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 133);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Hopper,
            0,
            0,
            176,
            133,
            [0.0, 0.0],
            [176.0 / 256.0, 133.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(40));
    assert_eq!(screen.slots.len(), 41);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (44, 20));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 40).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 109));
}

#[test]
fn hud_inventory_screen_projects_mount_horse_layout() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 66));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, Some(16), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Horse,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountSlot,
                7,
                17,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountSaddleSlot,
                8,
                18,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountSlot,
                7,
                35,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountHorseArmorSlot,
                8,
                36,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountChestSlots,
                79,
                17,
                90,
                54,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(16));
    assert_eq!(screen.slots.len(), 53);
    let saddle = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((saddle.x, saddle.y), (8, 18));
    let last_mount_slot = screen.slots.iter().find(|slot| slot.slot_id == 16).unwrap();
    assert_eq!((last_mount_slot.x, last_mount_slot.y), (152, 54));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 52).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_mount_entity_preview() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_add_entity(test_add_entity(77, 66));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 77,
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((5, 90)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.entity.entity_id, 77);
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 26,
            y: 18,
            width: 52,
            height: 52,
        }
    );
    assert_eq!(preview.scissor, None);
    assert_eq!(preview.scale, 17.0);
    assert!(preview.depth_isolated);
    let bounds = world.probe_entity_pick_bounds(77).unwrap();
    assert_close3(
        preview.translation,
        [0.0, (bounds.max[1] - bounds.min[1]) / 2.0 + 0.25, 0.0],
    );

    let x_angle = ((52.0_f32 - 5.0) / 40.0).atan();
    let y_angle = ((44.0_f32 - 90.0) / 40.0).atan();
    let expected_yaw = x_angle * 20.0;
    let expected_pitch = y_angle * 20.0;
    let expected_camera = quaternion_x(expected_pitch.to_radians());
    assert!((preview.entity.render_state.body_rot - (180.0 + expected_yaw)).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_yaw - expected_yaw).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_pitch + expected_pitch).abs() < 1.0e-6);
    assert_close4(
        preview.rotation,
        quaternion_mul([0.0, 0.0, 1.0, 0.0], expected_camera),
    );
    assert_eq!(preview.override_camera_rotation, Some(expected_camera));
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert_eq!(preview.entity.render_state.outline_color, 0);
    assert!(!preview.entity.render_state.appears_glowing);
}

#[test]
fn hud_inventory_screen_projects_mount_nautilus_slot_placeholders() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 88));
    world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
        id: 42,
        values: vec![test_byte_data(18, 4)],
    });
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, Some(1), 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Nautilus,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountSlot,
                7,
                17,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountSaddleSlot,
                8,
                18,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountSlot,
                7,
                35,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountNautilusArmorSlot,
                8,
                36,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.slots.len(), 38);
    let armor = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((armor.x, armor.y), (8, 36));
}

#[test]
fn hud_inventory_screen_hides_inactive_mount_equipment_slot_layers() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 36));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Horse,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::MountChestSlots,
                79,
                17,
                54,
                54,
                [0.0, 0.0],
                [0.6, 1.0],
            ),
        ]
    );
    assert!(screen.slots.iter().all(|slot| slot.slot_id != 0));
    assert!(screen.slots.iter().all(|slot| slot.slot_id != 1));
    assert_eq!(screen.slots[0].slot_id, 2);
}

#[test]
fn hud_inventory_screen_uses_mount_llama_armor_slot_placeholder() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 78));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, Some(1), 0.0).unwrap();

    assert!(screen.background_layers.iter().any(|layer| layer.texture
        == HudInventoryBackgroundTexture::MountLlamaArmorSlot
        && (layer.x, layer.y, layer.width, layer.height) == (8, 36, 16, 16)));
}

#[test]
fn hud_inventory_screen_projects_lectern_book_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: "Lectern".to_string(),
        title_styled: Vec::new(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.width, 192);
    assert_eq!(screen.height, 192);
    assert!(screen.slots.is_empty());
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Book,
                0,
                0,
                192,
                192,
                [0.0, 0.0],
                [192.0 / 256.0, 192.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::PageBackward,
                43,
                157,
                23,
                13,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::PageForward,
                116,
                157,
                23,
                13,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_lectern_current_page_text() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: "Lectern".to_string(),
        title_styled: Vec::new(),
    });
    let mut book = item_stack(42, 1);
    book.component_patch.written_book = Some(bbb_protocol::packets::WrittenBookContentSummary {
        title: "Guide".to_string(),
        title_filter: None,
        author: "Alex".to_string(),
        generation: 0,
        pages: vec![
            "First page".to_string(),
            "Second page\nLine two".to_string(),
        ],
        page_filters: vec![None, None],
        resolved: true,
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![book],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.text_labels,
        vec![
            HudInventoryTextLabel {
                x: 88,
                y: BOOK_PAGE_INDICATOR_Y,
                width: 60,
                text: "Page 2 of 2".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                input: None,
                scrolling: None,
                shadow: false,
                runs: Vec::new(),
            },
            HudInventoryTextLabel {
                x: BOOK_PAGE_TEXT_X,
                y: BOOK_PAGE_TEXT_Y,
                width: BOOK_PAGE_TEXT_WIDTH,
                text: "Second page".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                input: None,
                scrolling: None,
                shadow: false,
                runs: Vec::new(),
            },
            HudInventoryTextLabel {
                x: BOOK_PAGE_TEXT_X,
                y: BOOK_PAGE_TEXT_Y + BOOK_PAGE_LINE_HEIGHT,
                width: BOOK_PAGE_TEXT_WIDTH,
                text: "Line two".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                input: None,
                scrolling: None,
                shadow: false,
                runs: Vec::new(),
            },
        ]
    );
}

#[test]
fn dedicated_hud_book_screen_replaces_inventory_projection() {
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: written_book_stack(vec!["First page", "Second page"]),
    });
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
    });

    assert!(hud_inventory_screen(&world, None, None, 0.0).is_none());
    let screen = hud_book_screen(&ClientInputState::default(), &world, None).unwrap();
    assert_eq!(screen.mode, bbb_renderer::HudBookMode::View);
    assert_eq!(screen.page, 0);
    assert_eq!(screen.page_count, 2);
    assert_eq!(screen.page_indicator, "Page 1 of 2");
    assert_eq!(
        screen.runs,
        vec![HudStyledTextRun {
            text: "First page".to_string(),
            style: HudTextStyle::default(),
            color: Some(0),
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_notice_dialog_text_body_and_hovered_action() {
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.to_string(),
        value,
    };
    let mut world = WorldStore::new();
    assert!(
        !hud_screen_blur_for_world(&world, true),
        "ordinary in-game inventory-style screens stay transparent"
    );
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:notice".to_string()),
                ),
                entry(
                    "title",
                    NbtSummaryValue::String("Native Notice".to_string()),
                ),
                entry(
                    "body",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "type",
                            NbtSummaryValue::String("minecraft:plain_message".to_string()),
                        ),
                        entry("contents", NbtSummaryValue::String("AlphaBeta".to_string())),
                        entry("width", NbtSummaryValue::Int(30)),
                    ]),
                ),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "label",
                            NbtSummaryValue::Compound(vec![
                                entry("text", NbtSummaryValue::String("Continue".to_string())),
                                entry("bold", NbtSummaryValue::Byte(1)),
                                entry("color", NbtSummaryValue::String("#55ffff".to_string())),
                            ]),
                        ),
                        entry("width", NbtSummaryValue::Int(100)),
                    ]),
                ),
            ]),
        },
    });
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);

    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((160, 220)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();

    assert!(hud_screen_blur_for_world(&world, true));
    assert!(!hud_screen_blur_for_world(&world, false));
    assert_eq!((screen.width, screen.height), (320, 240));
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::InworldMenuBackground,
                0,
                0,
                320,
                240,
                [0.0, 0.0],
                [10.0, 7.5],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::DialogWarningButton,
                193,
                7,
                20,
                20,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::WidgetButtonHighlighted,
                110,
                213,
                100,
                20,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(
        screen
            .text_labels
            .iter()
            .map(|label| (label.text.as_str(), label.x, label.y, label.width))
            .collect::<Vec<_>>(),
        vec![("Native Notice", 107, 13, 76), ("Continue", 132, 219, 56),]
    );
    assert_eq!(
        screen
            .scroll_area
            .as_ref()
            .unwrap()
            .text_labels
            .iter()
            .map(|label| (label.text.as_str(), label.x, label.y, label.width))
            .collect::<Vec<_>>(),
        vec![
            ("Alp", 151, 67, 18),
            ("haB", 151, 76, 18),
            ("eta", 151, 85, 18),
        ]
    );
    let action_label = screen.text_labels.last().unwrap();
    assert_eq!(action_label.runs.len(), 1);
    assert!(action_label.runs[0].style.bold);
    assert_eq!(action_label.runs[0].color, Some(0x55_FF_FF));

    let idle = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((0, 0)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        idle.background_layers[0].texture,
        HudInventoryBackgroundTexture::InworldMenuBackground
    );
    assert_eq!(
        idle.background_layers[1].texture,
        HudInventoryBackgroundTexture::DialogWarningButton
    );
    assert_eq!(
        idle.background_layers[2].texture,
        HudInventoryBackgroundTexture::WidgetButton
    );
    assert!(world.open_dialog_warning());
    assert!(hud_screen_blur_for_world(&world, true));
    let warning = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(warning.text_labels[0].text, "Note about custom screens");
}

#[test]
fn hud_inventory_screen_projects_dialog_item_body_structured_patches() {
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.to_string(),
        value,
    };
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:notice".to_string()),
                ),
                entry("title", NbtSummaryValue::String("Item Body".to_string())),
                entry(
                    "body",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "type",
                            NbtSummaryValue::String("minecraft:item".to_string()),
                        ),
                        entry(
                            "item",
                            NbtSummaryValue::Compound(vec![
                                entry("id", NbtSummaryValue::String("minecraft:stick".to_string())),
                                entry("count", NbtSummaryValue::Int(3)),
                            ]),
                        ),
                        entry(
                            "description",
                            NbtSummaryValue::Compound(vec![
                                entry("contents", NbtSummaryValue::String("Info".to_string())),
                                entry("width", NbtSummaryValue::Int(50)),
                            ]),
                        ),
                        entry("width", NbtSummaryValue::Int(32)),
                        entry("height", NbtSummaryValue::Int(48)),
                    ]),
                ),
            ]),
        },
    });
    let item_runtime = recipe_book_ghost_item_runtime();
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);

    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        None,
        InventoryHudLocalState {
            cursor_position: Some((120, 65)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();

    assert_eq!(screen.floating_items.len(), 1);
    let item = &screen.floating_items[0];
    assert_eq!((item.x, item.y), (118, 63));
    assert!(item.draw_decorations);
    assert_eq!(
        item.icon
            .count_label
            .as_ref()
            .map(|label| label.text.as_str()),
        Some("3")
    );
    assert!(item.block_model.is_none());
    assert!(screen
        .scroll_area
        .as_ref()
        .unwrap()
        .text_labels
        .iter()
        .any(|label| { label.text == "Info" && (label.x, label.y, label.width) == (165, 83, 24) }));
    let tooltip = screen.tooltip.as_ref().unwrap();
    assert_eq!((tooltip.x, tooltip.y), (120, 65));
    assert_eq!(tooltip.lines[0].text, "Stick");

    assert!(world.focus_dialog_body_item(0));
    let focused = hud_inventory_screen_with_local_state_for_surface(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        None,
        InventoryHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        focused
            .scroll_area
            .as_ref()
            .unwrap()
            .fill_layers
            .iter()
            .map(|layer| {
                (
                    layer.x,
                    layer.y,
                    layer.width,
                    layer.height,
                    layer.tint,
                    layer.stage,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                118,
                63,
                32,
                1,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
            (
                118,
                110,
                32,
                1,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
            (
                118,
                64,
                1,
                46,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
            (
                149,
                64,
                1,
                46,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
        ]
    );

    let mut patched_world = WorldStore::new();
    patched_world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:notice".to_string()),
                ),
                entry("title", NbtSummaryValue::String("Patched".to_string())),
                entry(
                    "body",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "type",
                            NbtSummaryValue::String("minecraft:item".to_string()),
                        ),
                        entry(
                            "item",
                            NbtSummaryValue::Compound(vec![
                                entry("id", NbtSummaryValue::String("minecraft:stick".to_string())),
                                entry(
                                    "components",
                                    NbtSummaryValue::Compound(vec![
                                        entry(
                                            "minecraft:custom_name",
                                            NbtSummaryValue::String("Named".to_string()),
                                        ),
                                        entry(
                                            "minecraft:suspicious_stew_effects",
                                            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(
                                                vec![
                                                    entry(
                                                        "id",
                                                        NbtSummaryValue::String(
                                                            "minecraft:speed".to_string(),
                                                        ),
                                                    ),
                                                    entry("duration", NbtSummaryValue::Int(160)),
                                                ],
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:food",
                                            NbtSummaryValue::Compound(vec![
                                                entry("nutrition", NbtSummaryValue::Int(4)),
                                                entry(
                                                    "saturation",
                                                    NbtSummaryValue::Float(0.6f32.to_bits()),
                                                ),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:use_cooldown",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "seconds",
                                                NbtSummaryValue::Float(1.25f32.to_bits()),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:break_sound",
                                            NbtSummaryValue::String(
                                                "minecraft:entity.item.break".to_string(),
                                            ),
                                        ),
                                        entry(
                                            "minecraft:equippable",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "slot",
                                                NbtSummaryValue::String("head".to_string()),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:blocks_attacks",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:consumable",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:death_protection",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:tool",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "rules",
                                                NbtSummaryValue::List(Vec::new()),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:potion_contents",
                                            NbtSummaryValue::String("minecraft:water".to_string()),
                                        ),
                                        entry(
                                            "minecraft:enchantments",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:stored_enchantments",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:attribute_modifiers",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:can_break",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:can_place_on",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:writable_book_content",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:written_book_content",
                                            NbtSummaryValue::Compound(vec![
                                                entry(
                                                    "title",
                                                    NbtSummaryValue::String(String::new()),
                                                ),
                                                entry(
                                                    "author",
                                                    NbtSummaryValue::String(String::new()),
                                                ),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:entity_data",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "id",
                                                NbtSummaryValue::String(
                                                    "minecraft:pig".to_string(),
                                                ),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:bucket_entity_data",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:block_entity_data",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "id",
                                                NbtSummaryValue::String(
                                                    "minecraft:chest".to_string(),
                                                ),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:use_remainder",
                                            NbtSummaryValue::String(
                                                "minecraft:glass_bottle".to_string(),
                                            ),
                                        ),
                                        entry(
                                            "minecraft:charged_projectiles",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:bundle_contents",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:container",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry("minecraft:bees", NbtSummaryValue::List(Vec::new())),
                                        entry(
                                            "minecraft:map_decorations",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:debug_stick_state",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:recipes",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:container_loot",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "loot_table",
                                                NbtSummaryValue::String(
                                                    "minecraft:empty".to_string(),
                                                ),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:pot_decorations",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:lock",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:instrument",
                                            NbtSummaryValue::Compound(vec![
                                                entry(
                                                    "sound_event",
                                                    NbtSummaryValue::String(
                                                        "minecraft:block.note_block.harp"
                                                            .to_string(),
                                                    ),
                                                ),
                                                entry("use_duration", NbtSummaryValue::Int(1)),
                                                entry("range", NbtSummaryValue::Int(1)),
                                                entry(
                                                    "description",
                                                    NbtSummaryValue::String("Horn".to_string()),
                                                ),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:provides_trim_material",
                                            NbtSummaryValue::Compound(vec![
                                                entry(
                                                    "asset_name",
                                                    NbtSummaryValue::String("custom".to_string()),
                                                ),
                                                entry(
                                                    "description",
                                                    NbtSummaryValue::String(
                                                        "Custom material".to_string(),
                                                    ),
                                                ),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:trim",
                                            NbtSummaryValue::Compound(vec![
                                                entry(
                                                    "material",
                                                    NbtSummaryValue::Compound(vec![
                                                        entry(
                                                            "asset_name",
                                                            NbtSummaryValue::String(
                                                                "custom".to_string(),
                                                            ),
                                                        ),
                                                        entry(
                                                            "description",
                                                            NbtSummaryValue::String(
                                                                "Custom material".to_string(),
                                                            ),
                                                        ),
                                                    ]),
                                                ),
                                                entry(
                                                    "pattern",
                                                    NbtSummaryValue::Compound(vec![
                                                        entry(
                                                            "asset_id",
                                                            NbtSummaryValue::String(
                                                                "example:pattern".to_string(),
                                                            ),
                                                        ),
                                                        entry(
                                                            "description",
                                                            NbtSummaryValue::String(
                                                                "Custom pattern".to_string(),
                                                            ),
                                                        ),
                                                    ]),
                                                ),
                                            ]),
                                        ),
                                    ]),
                                ),
                            ]),
                        ),
                    ]),
                ),
            ]),
        },
    });
    assert_eq!(patched_world.current_dialog().unwrap().semantic_error, None);
    let patched = hud_inventory_screen_with_local_state_for_surface(
        &patched_world,
        Some(&item_runtime),
        &terrain_textures,
        None,
        InventoryHudLocalState {
            cursor_position: Some((153, 64)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(patched.floating_items.len(), 1);
    assert_eq!(patched.floating_items[0].icon.count_label, None);
    assert_eq!(patched.tooltip.as_ref().unwrap().lines[0].text, "Named");
}

#[test]
fn hud_inventory_screen_resolves_referenced_dialog_in_session_holder_order() {
    fn network_notice_dialog(title: &str) -> Vec<u8> {
        fn write_string_tag(payload: &mut Vec<u8>, name: &str, value: &str) {
            payload.push(8);
            payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
            payload.extend_from_slice(value.as_bytes());
        }

        let mut payload = vec![10];
        write_string_tag(&mut payload, "type", "minecraft:notice");
        write_string_tag(&mut payload, "title", title);
        payload.push(0);
        payload
    }

    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:dialog",
        0,
        vec![
            RegistryPacketEntry::stub("example:decoy"),
            RegistryPacketEntry::with_raw_data(
                "example:reordered_notice",
                network_notice_dialog("Reordered Notice"),
            ),
        ],
    );
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 1 },
    });

    assert_eq!(
        world.current_dialog().unwrap().resource_id.as_deref(),
        Some("example:reordered_notice")
    );
    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState::default(),
        winit::dpi::PhysicalSize::new(320, 240),
        0.0,
    )
    .unwrap();
    assert_eq!(screen.text_labels[0].text, "Reordered Notice");
}

#[test]
fn hud_inventory_screen_hides_unresolved_dialog_instead_of_falling_through() {
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 99 },
    });

    assert!(hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState::default(),
        winit::dpi::PhysicalSize::new(320, 240),
        0.0,
    )
    .is_none());
}

#[test]
fn hud_inventory_screen_projects_empty_advancements_screen() {
    let mut world = WorldStore::new();
    assert!(world.open_advancements_screen());
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let local_state = InventoryHudLocalState {
        cursor_position: Some((400, 580)),
        ..Default::default()
    };
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let (done_button_x, done_button_y) = advancements_done_button_origin_for_surface(surface_size);

    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &terrain_textures,
        None,
        local_state,
        surface_size,
        0.0,
    )
    .unwrap();

    assert_eq!(screen.width, 800);
    assert_eq!(screen.height, 600);
    assert_eq!((window_x, window_y), (274, 230));
    assert_eq!((done_button_x, done_button_y), (300, 573));
    assert!(screen.slots.is_empty());
    assert!(screen.floating_items.is_empty());
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementsWindow,
                window_x,
                window_y,
                ADVANCEMENTS_WINDOW_WIDTH,
                ADVANCEMENTS_WINDOW_HEIGHT,
                [0.0, 0.0],
                [
                    ADVANCEMENTS_WINDOW_WIDTH as f32 / 256.0,
                    ADVANCEMENTS_WINDOW_HEIGHT as f32 / 256.0,
                ],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::WidgetButtonHighlighted,
                done_button_x,
                done_button_y,
                ADVANCEMENTS_DONE_BUTTON_WIDTH,
                ADVANCEMENTS_DONE_BUTTON_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(
        screen.fill_layers,
        vec![HudInventoryFillLayer {
            x: window_x + ADVANCEMENTS_WINDOW_INSIDE_X,
            y: window_y + ADVANCEMENTS_WINDOW_INSIDE_Y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
            tint: ADVANCEMENTS_EMPTY_BACKGROUND_TINT,
            stage: HudInventoryFillStage::BeforeGhostItem,
        }]
    );
    assert_eq!(
        screen.text_labels,
        vec![
            HudInventoryTextLabel {
                x: window_x + ADVANCEMENTS_WINDOW_TITLE_X,
                y: window_y + ADVANCEMENTS_WINDOW_TITLE_Y,
                width: hud_ascii_approx_text_width(ADVANCEMENTS_TITLE_TEXT).unwrap(),
                text: ADVANCEMENTS_TITLE_TEXT.to_string(),
                tint: ADVANCEMENTS_TITLE_TEXT_COLOR,
                background: None,
                input: None,
                scrolling: None,
                shadow: false,
                runs: Vec::new(),
            },
            advancements_centered_text_label(
                ADVANCEMENTS_EMPTY_TEXT,
                window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
                window_y + ADVANCEMENTS_EMPTY_TEXT_Y,
                ADVANCEMENTS_EMPTY_TEXT_COLOR,
            ),
            advancements_centered_text_label(
                ADVANCEMENTS_SAD_TEXT,
                window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
                window_y + ADVANCEMENTS_SAD_TEXT_Y,
                ADVANCEMENTS_EMPTY_TEXT_COLOR,
            ),
            advancements_centered_text_label(
                ADVANCEMENTS_DONE_TEXT,
                done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32 / 2,
                done_button_y + ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET,
                ADVANCEMENTS_DONE_TEXT_COLOR,
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_advancement_root_tabs() {
    let mut world = WorldStore::new();
    let mut selected_root = runtime_displayed_advancement("minecraft:y/root", None);
    selected_root.display.as_mut().unwrap().background =
        Some("minecraft:gui/advancements/backgrounds/stone".to_string());
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            selected_root,
            runtime_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:y/root".to_string())
    );
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let local_state = InventoryHudLocalState::default();
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let (done_button_x, done_button_y) = advancements_done_button_origin_for_surface(surface_size);

    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &terrain_textures,
        None,
        local_state,
        surface_size,
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.background_layers[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::AdvancementsWindow,
            window_x,
            window_y,
            ADVANCEMENTS_WINDOW_WIDTH,
            ADVANCEMENTS_WINDOW_HEIGHT,
            [0.0, 0.0],
            [
                ADVANCEMENTS_WINDOW_WIDTH as f32 / 256.0,
                ADVANCEMENTS_WINDOW_HEIGHT as f32 / 256.0,
            ],
        )
    );
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let background_tiles: Vec<_> = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::AdvancementBackground(
                    HudAdvancementBackgroundTexture::Stone
                )
            )
        })
        .collect();
    assert_eq!(background_tiles.len(), 128);
    assert_eq!(
        *background_tiles[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::AdvancementBackground(
                HudAdvancementBackgroundTexture::Stone,
            ),
            inside_x,
            inside_y,
            7,
            11,
            [9.0 / 16.0, 5.0 / 16.0],
            [1.0, 1.0],
        )
    );
    assert!(background_tiles.iter().all(|layer| {
        layer.x >= inside_x
            && layer.y >= inside_y
            && layer.x + layer.width as i32 <= inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32
            && layer.y + layer.height as i32 <= inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32
    }));
    let done_button_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::WidgetButton,
        done_button_x,
        done_button_y,
        ADVANCEMENTS_DONE_BUTTON_WIDTH,
        ADVANCEMENTS_DONE_BUTTON_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    let selected_tab_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementTab(HudAdvancementTabSprite::AboveLeftSelected),
        window_x,
        window_y - 28,
        28,
        32,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    let second_tab_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementTab(HudAdvancementTabSprite::AboveMiddle),
        window_x + 32,
        window_y - 28,
        28,
        32,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    let widget_frame_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementWidgetFrame(
            HudAdvancementWidgetFrameSprite::TaskUnobtained,
        ),
        inside_x + 106,
        inside_y + 43,
        26,
        26,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    assert!(screen.background_layers.contains(&done_button_layer));
    assert!(screen.background_layers.contains(&selected_tab_layer));
    assert!(screen.background_layers.contains(&second_tab_layer));
    assert!(screen.background_layers.contains(&widget_frame_layer));
    let first_background_index = screen
        .background_layers
        .iter()
        .position(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::AdvancementBackground(_)
            )
        })
        .unwrap();
    let widget_index = screen
        .background_layers
        .iter()
        .position(|layer| *layer == widget_frame_layer)
        .unwrap();
    assert!(first_background_index < widget_index);
    assert_eq!(
        screen.background_layers.len(),
        1 + background_tiles.len() + 1 + 2 + 1
    );
    assert!(screen.fill_layers.is_empty());
    assert!(screen.floating_items.is_empty());
    assert_eq!(
        screen.text_labels,
        vec![
            HudInventoryTextLabel {
                x: window_x + ADVANCEMENTS_WINDOW_TITLE_X,
                y: window_y + ADVANCEMENTS_WINDOW_TITLE_Y,
                width: hud_ascii_approx_text_width("minecraft:y/root").unwrap(),
                text: "minecraft:y/root".to_string(),
                tint: ADVANCEMENTS_TITLE_TEXT_COLOR,
                background: None,
                input: None,
                scrolling: None,
                shadow: false,
                runs: vec![HudStyledTextRun::plain("minecraft:y/root")],
            },
            advancements_centered_text_label(
                ADVANCEMENTS_DONE_TEXT,
                done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32 / 2,
                done_button_y + ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET,
                ADVANCEMENTS_DONE_TEXT_COLOR,
            ),
        ]
    );
}

#[test]
fn advancement_widget_connection_layers_match_vanilla_two_pass_lines() {
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let widgets = vec![
        advancement_widget_summary_for_test("minecraft:story/root", None, 0, 0),
        advancement_widget_summary_for_test(
            "minecraft:story/mine_stone",
            Some("minecraft:story/root"),
            56,
            27,
        ),
    ];

    let layers = advancements_widget_connection_layers(&widgets, None, window_x, window_y);

    assert_eq!(
        layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 88,
                inside_y + 41,
                18,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 88,
                inside_y + 42,
                19,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 88,
                inside_y + 43,
                18,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 68,
                41,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 69,
                41,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 70,
                41,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 43,
                1,
                26,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 106,
                inside_y + 43,
                1,
                26,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Foreground,
                ),
                inside_x + 88,
                inside_y + 42,
                18,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Foreground,
                ),
                inside_x + 105,
                inside_y + 69,
                40,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Foreground,
                ),
                inside_x + 105,
                inside_y + 43,
                1,
                26,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn advancement_widget_scroll_applies_local_delta_and_vanilla_clamp() {
    let widgets = vec![
        advancement_widget_summary_for_test("minecraft:story/root", None, 0, 0),
        advancement_widget_summary_for_test(
            "minecraft:story/tall_child",
            Some("minecraft:story/root"),
            0,
            135,
        ),
    ];

    assert_eq!(advancements_widget_scroll(&widgets, None), Some((103, -25)));
    assert_eq!(
        advancements_widget_scroll(&widgets, Some((64.0, -16.0))),
        Some((103, -41))
    );
    assert_eq!(
        advancements_widget_scroll(&widgets, Some((0.0, -100.0))),
        Some((103, -49))
    );
    assert_eq!(
        advancements_widget_scroll(&widgets, Some((0.0, 100.0))),
        Some((103, 0))
    );
}

#[test]
fn advancement_widget_frame_layer_clips_to_contents_with_uvs() {
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;

    let layer = clipped_advancement_widget_frame_layer(
        HudAdvancementWidgetFrameSprite::TaskUnobtained,
        inside_x - 5,
        inside_y + 2,
        inside_x,
        inside_y,
    )
    .unwrap();

    assert_eq!(
        layer,
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::AdvancementWidgetFrame(
                HudAdvancementWidgetFrameSprite::TaskUnobtained,
            ),
            inside_x,
            inside_y + 2,
            21,
            26,
            [5.0 / 26.0, 0.0],
            [1.0, 1.0],
        )
    );
}

#[test]
fn advancement_widget_icon_items_keep_partial_fake_item_with_content_scissor() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let world = WorldStore::new();
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let widgets = vec![
        advancement_widget_summary_for_test("minecraft:story/root", None, 0, 0),
        advancement_widget_summary_for_test("minecraft:story/far_child", None, 400, 0),
    ];

    let items = advancements_widget_icon_items(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        &widgets,
        Some((81.0, 0.0)),
        window_x,
        window_y,
        ItemModelKeybindContext::default(),
        0.0,
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].x, inside_x - 8);
    assert_eq!(items[0].y, inside_y + 48);
    assert_eq!(
        items[0].scissor,
        Some(HudInventoryItemScissor {
            x: inside_x,
            y: inside_y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
        })
    );
    assert!(!items[0].draw_decorations);
}

#[test]
fn hud_inventory_screen_projects_advancement_hover_tooltip() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = WorldStore::new();
    let mut root = runtime_displayed_advancement("minecraft:story/root", None);
    root.requirements = vec![vec!["has_log".to_string()], vec!["has_planks".to_string()]];
    let display = root.display.as_mut().unwrap();
    display.title = *styled_boss_component("Getting Wood");
    display.description = *styled_boss_component("Punch a tree");
    display.frame_type = AdvancementFrameType::Challenge;
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![root],
        removed: Vec::new(),
        progress: vec![AdvancementProgressSummary {
            id: "minecraft:story/root".to_string(),
            criteria: vec![
                AdvancementCriterionProgressSummary {
                    name: "has_log".to_string(),
                    obtained_epoch_millis: Some(1),
                },
                AdvancementCriterionProgressSummary {
                    name: "has_planks".to_string(),
                    obtained_epoch_millis: None,
                },
            ],
        }],
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let local_state = InventoryHudLocalState {
        cursor_position: Some((inside_x + 110, inside_y + 50)),
        advancement_hover_fade: ADVANCEMENTS_HOVER_MAX_FADE,
        ..Default::default()
    };

    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        None,
        local_state,
        surface_size,
        0.0,
    )
    .unwrap();

    assert!(screen.fill_layers.contains(&HudInventoryFillLayer {
        x: inside_x,
        y: inside_y,
        width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
        height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
        tint: ADVANCEMENTS_HOVER_FADE_TINT,
        stage: HudInventoryFillStage::Foreground,
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        matches!(
            layer.texture,
            HudInventoryBackgroundTexture::AdvancementHoverBox(HudAdvancementHoverBoxSprite::Title)
        )
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        matches!(
            layer.texture,
            HudInventoryBackgroundTexture::AdvancementHoverBox(
                HudAdvancementHoverBoxSprite::Obtained
            )
        )
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        matches!(
            layer.texture,
            HudInventoryBackgroundTexture::AdvancementHoverBox(
                HudAdvancementHoverBoxSprite::Unobtained
            )
        )
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        matches!(
            layer.texture,
            HudInventoryBackgroundTexture::AdvancementWidgetFrame(
                HudAdvancementWidgetFrameSprite::ChallengeUnobtained
            )
        )
    }));
    let title = screen
        .text_labels
        .iter()
        .find(|label| label.text == "Getting Wood")
        .unwrap();
    assert!(title.runs[0].style.bold);
    assert_eq!(title.runs[0].color, Some(0xFF_55_55));
    assert!(screen.text_labels.iter().any(|label| label.text == "1/2"));
    let description = screen
        .text_labels
        .iter()
        .find(|label| label.text == "Punch a tree")
        .unwrap();
    assert_eq!(
        description.tint,
        ADVANCEMENTS_HOVER_CHALLENGE_DESCRIPTION_TEXT_COLOR
    );
    assert!(description.runs[0].style.bold);
    assert_eq!(description.runs[0].color, Some(0xFF_55_55));
    assert_eq!(screen.foreground_items.len(), 1);
    assert_eq!(screen.foreground_items[0].x, inside_x + 111);
    assert_eq!(screen.foreground_items[0].y, inside_y + 48);
    assert_eq!(screen.foreground_items[0].scissor, None);
    assert!(!screen.foreground_items[0].draw_decorations);
}

fn advancements_centered_text_label(
    text: &str,
    center_x: i32,
    y: i32,
    tint: [f32; 4],
) -> HudInventoryTextLabel {
    let width = hud_ascii_approx_text_width(text).unwrap();
    HudInventoryTextLabel {
        x: center_x - i32::try_from(width).unwrap() / 2,
        y,
        width,
        text: text.to_string(),
        tint,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        runs: Vec::new(),
    }
}

fn runtime_displayed_advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
    AdvancementSummary {
        id: id.to_string(),
        parent: parent.map(str::to_string),
        display: Some(AdvancementDisplaySummary {
            title: bbb_protocol::RichTextComponent::plain(id),
            description: bbb_protocol::RichTextComponent::plain(""),
            icon: AdvancementIconSummary {
                item_id: 1,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
            frame_type: AdvancementFrameType::Task,
            show_toast: false,
            hidden: false,
            background: None,
            x: 0.0,
            y: 0.0,
        }),
        requirements: Vec::new(),
        sends_telemetry_event: false,
    }
}

fn advancement_widget_summary_for_test(
    id: &str,
    parent_id: Option<&str>,
    x: i32,
    y: i32,
) -> bbb_world::AdvancementWidgetSummary {
    bbb_world::AdvancementWidgetSummary {
        id: id.to_string(),
        parent_id: parent_id.map(str::to_string),
        title: bbb_protocol::RichTextComponent::plain(id),
        description: bbb_protocol::RichTextComponent::plain(""),
        icon: AdvancementIconSummary {
            item_id: 1,
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        },
        frame_type: AdvancementFrameType::Task,
        x,
        y,
        hidden: false,
        done: false,
        progress_done: 0,
        progress_total: 0,
    }
}

#[test]
fn hud_inventory_screen_projects_shulker_box_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: "Shulker Box".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 63],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(62), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 167);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::ShulkerBox,
            0,
            0,
            176,
            167,
            [0.0, 0.0],
            [176.0 / 256.0, 167.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(62));
    assert_eq!(screen.slots.len(), 63);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (8, 18));
    let last_container = screen.slots.iter().find(|slot| slot.slot_id == 26).unwrap();
    assert_eq!((last_container.x, last_container.y), (152, 54));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 62).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_loom_layout_and_empty_slot_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(39), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Loom,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomBannerSlot,
                13,
                26,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomDyeSlot,
                33,
                26,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomPatternSlot,
                23,
                45,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomScrollerDisabled,
                119,
                13,
                12,
                15,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(39));
    assert_eq!(screen.slots.len(), 40);
    let banner = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((banner.x, banner.y), (13, 26));
    let dye = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((dye.x, dye.y), (33, 26));
    let pattern = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((pattern.x, pattern.y), (23, 45));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((result.x, result.y), (143, 57));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 39).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_loom_pattern_grid_and_scroller() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            loom_pattern_scroll_row: Some(2),
            loom_selected_pattern_index: Some(10),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.background_layers.len(), 19);
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::LoomScroller,
            119,
            33,
            12,
            15,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::LoomPattern,
            60,
            13,
            14,
            14,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::LoomPatternSelected,
            88,
            13,
            14,
            14,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::LoomPattern,
            102,
            55,
            14,
            14,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_merchant_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

    assert_eq!(screen.width, 276);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Villager,
            0,
            0,
            276,
            166,
            [0.0, 0.0],
            [276.0 / 512.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(38));
    assert_eq!(screen.slots.len(), 39);
    assert!(screen.floating_items.is_empty());
    let payment_a = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((payment_a.x, payment_a.y), (136, 37));
    let payment_b = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((payment_b.x, payment_b.y), (162, 37));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (220, 37));
    let first_inventory = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((first_inventory.x, first_inventory.y), (108, 84));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (252, 142));
}

#[test]
fn hud_inventory_screen_projects_merchant_trade_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 8, Some(2))));
    assert!(world.set_local_merchant_selected_offer(2));
    assert!(world.scroll_local_merchant_offers(1));

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        Some(38),
        InventoryHudLocalState {
            cursor_position: Some((6, 19)),
            merchant_focused_trade_row: Some(1),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.width, 276);
    assert_eq!(screen.height, 166);
    assert!(screen.floating_items.is_empty());
    assert_eq!(
        screen.background_layers[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Villager,
            0,
            0,
            276,
            166,
            [0.0, 0.0],
            [276.0 / 512.0, 166.0 / 256.0],
        )
    );
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::WidgetButtonHighlighted,
            5,
            18,
            88,
            20,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::WidgetButtonHighlighted,
            5,
            38,
            88,
            20,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::WidgetButton,
            5,
            58,
            88,
            20,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerTradeArrow,
            60,
            22,
            10,
            9,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerTradeArrowOutOfStock,
            60,
            42,
            10,
            9,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .foreground_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerDiscountStrikethrough,
            17,
            31,
            9,
            2,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerScroller,
            94,
            131,
            6,
            27,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerOutOfStock,
            182,
            35,
            28,
            21,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerExperienceBarBackground,
            136,
            16,
            102,
            5,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerExperienceBarCurrent,
            136,
            16,
            63,
            5,
            [0.0, 0.0],
            [63.0 / 102.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_merchant_future_xp_result() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[2] = item_stack(101, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 8, None)));
    assert!(world.set_local_merchant_selected_offer(2));

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerExperienceBarResult,
            199,
            16,
            10,
            5,
            [63.0 / 102.0, 0.0],
            [73.0 / 102.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_merchant_component_cost_decorations_and_tooltips() {
    let root = unique_runtime_temp_dir("merchant-trade-presentation");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    let mut packet = merchant_offers(7, 1, None);
    let offer = &mut packet.offers[0];
    offer.buy_a = item_cost(1, 10);
    offer.buy_a.component_predicate.components.added = 2;
    offer.buy_a.component_predicate.components.added_type_ids = vec![
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
    ];
    offer.buy_a.component_predicate.components.custom_name = Some("Predicate Cost".to_string());
    offer.buy_a.component_predicate.components.item_model =
        Some("minecraft:test_combo".to_string());
    offer.buy_b = Some(item_cost(0, 2));
    offer.sell = item_stack(0, 3);
    offer.sell.component_patch.added = 1;
    offer
        .sell
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID);
    offer.sell.component_patch.custom_name = Some("Trade Result".to_string());
    offer.sell.component_patch.max_damage = Some(20);
    offer.sell.component_patch.damage = Some(5);
    offer.special_price_diff = -2;
    offer.price_multiplier = 0.0;
    offer.demand = 0;
    let result = offer.sell.clone();
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[2] = result;
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(packet.clone()));

    let screen_at = |cursor_position| {
        hud_inventory_screen_with_local_state(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            None,
            InventoryHudLocalState {
                cursor_position: Some(cursor_position),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap()
    };
    let cost_a_screen = screen_at((10, 19));

    assert_eq!(cost_a_screen.floating_items.len(), 5);
    assert_eq!(
        cost_a_screen
            .floating_items
            .iter()
            .take(3)
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(10, 19, false), (40, 19, true), (73, 19, true)]
    );
    assert!(!cost_a_screen.floating_items[0].icon.layers.is_empty());
    assert_eq!(
        cost_a_screen.floating_items[1]
            .icon
            .count_label
            .as_ref()
            .map(|label| label.text.as_str()),
        Some("2")
    );
    assert_eq!(
        cost_a_screen.floating_items[2]
            .icon
            .count_label
            .as_ref()
            .map(|label| label.text.as_str()),
        Some("3")
    );
    assert!(cost_a_screen.floating_items[2]
        .icon
        .durability_bar
        .is_some());
    assert!(cost_a_screen.foreground_items.is_empty());
    assert_eq!(
        cost_a_screen
            .floating_items
            .iter()
            .skip(3)
            .map(|item| {
                (
                    item.x,
                    item.y,
                    item.icon
                        .count_label
                        .as_ref()
                        .map(|label| label.text.as_str()),
                    item.icon.layers.is_empty(),
                    item.draw_decorations,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (10, 19, Some("10"), true, true),
            (24, 19, Some("8"), true, true),
        ]
    );
    assert_eq!(
        cost_a_screen.tooltip.as_ref().unwrap().lines[0].text,
        "Predicate Cost"
    );
    assert_eq!(
        screen_at((40, 19)).tooltip.unwrap().lines[0].text,
        "Test Combo"
    );
    assert_eq!(
        screen_at((73, 19)).tooltip.unwrap().lines[0].text,
        "Trade Result"
    );
    packet.offers[0].is_out_of_stock = true;
    packet.offers[0].uses = packet.offers[0].max_uses;
    assert!(world.apply_merchant_offers(packet));
    let deprecated = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((186, 35)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();
    assert!(!deprecated.background_layers.iter().any(|layer| {
        layer.texture == HudInventoryBackgroundTexture::VillagerExperienceBarResult
    }));
    let deprecated = deprecated.tooltip.unwrap();
    assert_eq!(
        (deprecated.slot_id, deprecated.x, deprecated.y),
        (u16::MAX, 186, 35)
    );
    assert_eq!(
        deprecated.lines,
        vec![HudInventoryTooltipLine {
            text: "Villagers restock up to two times per day.".to_string(),
            tint: TOOLTIP_TEST_WHITE,
            runs: Vec::new(),
        }]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn merchant_offer_cost_a_stack_uses_vanilla_modified_cost_count() {
    let mut world = WorldStore::new();
    world.set_default_item_max_stack_sizes(std::collections::BTreeMap::from([(42, 16)]));
    let mut buy_a = item_cost(42, 10);
    buy_a.component_predicate.components.added = 1;
    buy_a.component_predicate.components.added_type_ids = vec![
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
    ];
    buy_a.component_predicate.components.max_stack_size = Some(12);
    buy_a.component_predicate.components.item_model = Some("minecraft:emerald".to_string());
    let offer = bbb_world::MerchantOfferState {
        buy_a,
        sell: item_stack(99, 1),
        buy_b: None,
        is_out_of_stock: false,
        uses: 0,
        max_uses: 12,
        xp: 8,
        special_price_diff: 5,
        price_multiplier: 0.5,
        demand: 2,
    };

    let stack = merchant_offer_cost_a_stack(&world, &offer);
    assert_eq!(stack.item_id, Some(42));
    assert_eq!(stack.count, 12);
    assert_eq!(
        stack.component_patch.added_type_ids,
        vec![
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
        ]
    );
    assert_eq!(stack.component_patch.max_stack_size, Some(12));
    assert_eq!(
        stack.component_patch.item_model.as_deref(),
        Some("minecraft:emerald")
    );
}

#[test]
fn hud_inventory_screen_projects_smithing_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(39), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Smithing,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(39));
    assert_eq!(screen.slots.len(), 40);
    let template = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((template.x, template.y), (8, 48));
    let base = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((base.x, base.y), (26, 48));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((additional.x, additional.y), (44, 48));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((result.x, result.y), (98, 48));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 39).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_smithing_armor_stand_preview() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.entity.entity_id, -1);
    match preview.entity.kind {
        bbb_renderer::EntityModelKind::ArmorStand {
            small,
            marker,
            show_arms,
            show_base_plate,
            pose,
        } => {
            assert!(!small);
            assert!(!marker);
            assert!(show_arms);
            assert!(!show_base_plate);
            assert_eq!(pose, DEFAULT_ARMOR_STAND_MODEL_POSE);
        }
        other => panic!("expected smithing armor stand preview, got {other:?}"),
    }
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 121,
            y: 20,
            width: 40,
            height: 60,
        }
    );
    assert_eq!(preview.scissor, None);
    assert_close3(preview.translation, [0.0, 1.0, 0.0]);
    assert_close4(
        preview.rotation,
        quaternion_mul(quaternion_x(0.43633232), quaternion_z(std::f32::consts::PI)),
    );
    assert_eq!(preview.override_camera_rotation, None);
    assert_eq!(preview.scale, 25.0);
    assert!(preview.depth_isolated);
    assert!((preview.entity.render_state.body_rot - 210.0).abs() < 1.0e-6);
    assert_eq!(preview.entity.render_state.head_yaw, 0.0);
    assert_eq!(preview.entity.render_state.head_pitch, 25.0);
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert_eq!(preview.entity.render_state.outline_color, 0);
    assert!(!preview.entity.render_state.appears_glowing);
}

#[test]
fn hud_inventory_screen_projects_smithing_result_equipment_preview() {
    const HELMET_ID: i32 = 0;
    const CHESTPLATE_ID: i32 = 1;
    const LEGGINGS_ID: i32 = 2;
    const BOOTS_ID: i32 = 3;
    const ELYTRA_ID: i32 = 4;
    const SKELETON_SKULL_ID: i32 = 5;
    const CARVED_PUMPKIN_ID: i32 = 6;
    const IRON_SWORD_ID: i32 = 7;

    let runtime = smithing_preview_item_runtime();

    let mut enchanted_helmet = item_stack(HELMET_ID, 1);
    enchanted_helmet.component_patch.dyed_color = Some(0x12_34_56);
    enchanted_helmet.component_patch.enchantments.push(
        bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: 12,
            level: 1,
        },
    );
    let helmet_preview = smithing_preview_for_result_stack(enchanted_helmet, &runtime);
    let helmet_state = &helmet_preview.entity.render_state;
    assert_eq!(
        helmet_state.head_armor,
        Some(bbb_renderer::EntityArmorMaterial::Diamond)
    );
    assert_eq!(helmet_state.head_armor_dye, Some(0x12_34_56));
    assert!(helmet_state.head_armor_foil);
    assert_eq!(helmet_state.custom_head_skull, None);

    let chest_preview = smithing_preview_for_result_stack(item_stack(CHESTPLATE_ID, 1), &runtime);
    let chest_state = &chest_preview.entity.render_state;
    assert_eq!(
        chest_state.chest_armor,
        Some(bbb_renderer::EntityArmorMaterial::Diamond)
    );
    assert!(chest_state.chest_equipment_has_humanoid);
    assert!(!chest_state.chest_equipment_has_wings);
    assert_eq!(chest_state.chest_wings_layer, None);

    let legs_preview = smithing_preview_for_result_stack(item_stack(LEGGINGS_ID, 1), &runtime);
    assert_eq!(
        legs_preview.entity.render_state.legs_armor,
        Some(bbb_renderer::EntityArmorMaterial::Diamond)
    );

    let feet_preview = smithing_preview_for_result_stack(item_stack(BOOTS_ID, 1), &runtime);
    assert_eq!(
        feet_preview.entity.render_state.feet_armor,
        Some(bbb_renderer::EntityArmorMaterial::Diamond)
    );

    let elytra_preview = smithing_preview_for_result_stack(item_stack(ELYTRA_ID, 1), &runtime);
    let elytra_state = &elytra_preview.entity.render_state;
    assert_eq!(elytra_state.chest_armor, None);
    assert!(!elytra_state.chest_armor_foil);
    assert!(elytra_state.chest_equipment_has_wings);
    assert!(!elytra_state.chest_equipment_has_humanoid);
    assert_eq!(
        elytra_state.chest_wings_layer,
        Some(bbb_renderer::EntityEquipmentLayerTexture {
            texture: bbb_renderer::EntityModelTextureRef {
                path: "textures/entity/equipment/wings/elytra.png",
                size: [64, 32],
            },
            use_player_texture: true,
        })
    );

    let skull_preview =
        smithing_preview_for_result_stack(item_stack(SKELETON_SKULL_ID, 1), &runtime);
    let skull_state = &skull_preview.entity.render_state;
    assert_eq!(skull_state.head_armor, None);
    assert!(!skull_state.head_armor_foil);
    assert_eq!(
        skull_state.custom_head_skull,
        Some(bbb_renderer::EntityCustomHeadSkull::Skeleton)
    );
    assert!(skull_preview.item_layers.is_empty());

    let pumpkin_preview =
        smithing_preview_for_result_stack(item_stack(CARVED_PUMPKIN_ID, 1), &runtime);
    let pumpkin_state = &pumpkin_preview.entity.render_state;
    assert_eq!(pumpkin_state.head_armor, None);
    assert_eq!(pumpkin_state.custom_head_skull, None);
    assert_eq!(
        pumpkin_preview.item_layers,
        vec![HudEntityPreviewItemLayer {
            slot: HudEntityPreviewItemSlot::Head,
            display_context: HudEntityPreviewItemDisplayContext::Head,
            item_id: CARVED_PUMPKIN_ID,
            count: 1,
            foil: false,
            light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
            overlay: ITEM_MODEL_NO_OVERLAY,
            order: 0,
            submit_sequence: 2,
        }]
    );

    let mut sword = item_stack(IRON_SWORD_ID, 1);
    sword.component_patch.enchantment_glint_override = Some(true);
    let sword_preview = smithing_preview_for_result_stack(sword, &runtime);
    assert_eq!(
        sword_preview.item_layers,
        vec![HudEntityPreviewItemLayer {
            slot: HudEntityPreviewItemSlot::LeftHand,
            display_context: HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
            item_id: IRON_SWORD_ID,
            count: 1,
            foil: true,
            light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
            overlay: ITEM_MODEL_NO_OVERLAY,
            order: 0,
            submit_sequence: 1,
        }]
    );
}

fn smithing_preview_for_result_stack(
    stack: bbb_protocol::packets::ItemStackSummary,
    runtime: &NativeItemRuntime,
) -> HudEntityPreview {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 40];
    items[3] = stack;
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    hud_inventory_screen(&world, Some(runtime), None, 0.0)
        .unwrap()
        .entity_previews
        .into_iter()
        .next()
        .unwrap()
}

fn smithing_preview_item_runtime() -> NativeItemRuntime {
    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:diamond_helmet",
            "minecraft:diamond_chestplate",
            "minecraft:diamond_leggings",
            "minecraft:diamond_boots",
            "minecraft:elytra",
            "minecraft:skeleton_skull",
            "minecraft:carved_pumpkin",
            "minecraft:iron_sword"
        ],
        "protocol_ids": {
            "minecraft:diamond_helmet": 0,
            "minecraft:diamond_chestplate": 1,
            "minecraft:diamond_leggings": 2,
            "minecraft:diamond_boots": 3,
            "minecraft:elytra": 4,
            "minecraft:skeleton_skull": 5,
            "minecraft:carved_pumpkin": 6,
            "minecraft:iron_sword": 7
        },
        "default_equipment_slots": {
            "minecraft:diamond_helmet": "head",
            "minecraft:diamond_chestplate": "chest",
            "minecraft:diamond_leggings": "legs",
            "minecraft:diamond_boots": "feet",
            "minecraft:elytra": "chest",
            "minecraft:skeleton_skull": "head",
            "minecraft:carved_pumpkin": "head"
        },
        "humanoid_armor_assets": {
            "minecraft:diamond_helmet": "diamond",
            "minecraft:diamond_chestplate": "diamond",
            "minecraft:diamond_leggings": "diamond",
            "minecraft:diamond_boots": "diamond"
        },
        "equippable_assets": {
            "minecraft:diamond_helmet": "diamond",
            "minecraft:diamond_chestplate": "diamond",
            "minecraft:diamond_leggings": "diamond",
            "minecraft:diamond_boots": "diamond",
            "minecraft:elytra": "elytra"
        }
    }))
    .unwrap();
    let equipment_assets: bbb_pack::EquipmentAssetCatalog =
        serde_json::from_value(serde_json::json!({
            "assets": {
                "minecraft:diamond": {
                    "layers": {
                        "humanoid": [
                            {
                                "texture": "minecraft:diamond",
                                "texture_location": "minecraft:textures/entity/equipment/humanoid/diamond.png",
                                "use_player_texture": false
                            }
                        ],
                        "humanoid_leggings": [
                            {
                                "texture": "minecraft:diamond",
                                "texture_location": "minecraft:textures/entity/equipment/humanoid_leggings/diamond.png",
                                "use_player_texture": false
                            }
                        ]
                    }
                },
                "minecraft:elytra": {
                    "layers": {
                        "wings": [
                            {
                                "texture": "minecraft:elytra",
                                "texture_location": "minecraft:textures/entity/equipment/wings/elytra.png",
                                "use_player_texture": true
                            }
                        ]
                    }
                }
            }
        }))
        .unwrap();

    NativeItemRuntime::for_test_with_registry_and_equipment_assets(registry, equipment_assets)
}

#[test]
fn hud_inventory_screen_projects_smithing_error_layer() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Smithing,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::SmithingError,
                65,
                46,
                28,
                21,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_cartography_table_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::CartographyTable,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(38));
    assert_eq!(screen.slots.len(), 39);
    let map = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((map.x, map.y), (15, 15));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((additional.x, additional.y), (15, 52));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (145, 39));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_cartography_table_map_frame() {
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = map_stack(100, 1, 42, None);
    let world = cartography_table_world_with_items(items);

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTableMap,
                67,
                13,
                66,
                66,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_cartography_table_result_modes() {
    let cases = [
        (
            map_stack(
                100,
                1,
                42,
                Some(bbb_protocol::packets::MapPostProcessingSummary::Scale),
            ),
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTableScaledMap,
                67,
                13,
                66,
                66,
                [0.0, 0.0],
                [1.0, 1.0],
            )],
        ),
        (
            map_stack(100, 2, 42, None),
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                    83,
                    13,
                    50,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                    67,
                    29,
                    50,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ],
        ),
        (
            map_stack(
                100,
                1,
                42,
                Some(bbb_protocol::packets::MapPostProcessingSummary::Lock),
            ),
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableMap,
                    67,
                    13,
                    66,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableLocked,
                    118,
                    60,
                    10,
                    14,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ],
        ),
    ];

    for (result_stack, expected_layers) in cases {
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = map_stack(100, 1, 42, None);
        items[2] = result_stack;
        let world = cartography_table_world_with_items(items);

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(&screen.background_layers[1..], expected_layers.as_slice());
    }
}

#[test]
fn cartography_additional_item_resource_ids_match_vanilla_items() {
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:paper"),
        Some(CartographyAdditionalItem::Paper)
    );
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:map"),
        Some(CartographyAdditionalItem::EmptyMap)
    );
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:glass_pane"),
        Some(CartographyAdditionalItem::GlassPane)
    );
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:filled_map"),
        None
    );
}

#[test]
fn hud_inventory_screen_projects_stonecutter_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 38],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(37), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Stonecutter,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::StonecutterScrollerDisabled,
                119,
                15,
                12,
                15,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(37));
    assert_eq!(screen.slots.len(), 38);
    let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((input.x, input.y), (20, 33));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((result.x, result.y), (143, 33));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_stonecutter_recipe_buttons_and_scroller() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_update_recipes(bbb_protocol::packets::UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..16)
            .map(|index| stonecutter_recipe_display(42, 100 + index, 1))
            .collect(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 5,
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            stonecutter_recipe_scroll_row: Some(1),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.background_layers[1],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::StonecutterScroller,
            119,
            56,
            12,
            15,
            [0.0, 0.0],
            [1.0, 1.0],
        )
    );
    let recipe_layers: Vec<_> = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::StonecutterRecipe
                    | HudInventoryBackgroundTexture::StonecutterRecipeSelected
            )
        })
        .collect();
    assert_eq!(recipe_layers.len(), 12);
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::StonecutterRecipeSelected,
            68,
            15,
            16,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));

    let option_stacks = stonecutter_visible_recipe_option_stacks(&world, 1);
    assert_eq!(option_stacks.len(), 12);
    assert_eq!(
        option_stacks[0],
        StonecutterRecipeOptionStack {
            x: 52,
            y: 16,
            stack: item_stack(104, 1),
        }
    );
    assert_eq!(
        option_stacks[11],
        StonecutterRecipeOptionStack {
            x: 100,
            y: 52,
            stack: item_stack(115, 1),
        }
    );
}

#[test]
fn stonecutter_buttons_and_tooltips_resolve_all_builtin_slot_displays() {
    let root = unique_runtime_temp_dir("stonecutter-slot-displays");
    write_stonecutter_slot_display_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let world = stonecutter_world_with_builtin_slot_displays(1);

    let option_stacks = stonecutter_visible_recipe_option_stacks(&world, 0);
    assert_eq!(option_stacks.len(), 10);
    let stack_at = |position: i32| {
        let x = STONECUTTER_RECIPE_BUTTON_X
            + position % STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_WIDTH;
        let y = STONECUTTER_RECIPE_ITEM_Y
            + position / STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_HEIGHT;
        option_stacks
            .iter()
            .find(|option| (option.x, option.y) == (x, y))
            .map(|option| &option.stack)
    };

    assert_eq!(stack_at(0), None);
    for position in 1..=10 {
        assert_eq!(stack_at(position).unwrap().item_id, Some(1));
    }
    assert_eq!(stack_at(2).unwrap().component_patch.potion_id, Some(0));
    assert!(world.item_stack_has_effective_component(
        stack_at(3).unwrap(),
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
    ));
    assert_eq!(stack_at(5).unwrap().count, 3);
    assert_eq!(
        stack_at(7).unwrap().component_patch.dyed_color,
        Some(0xB0_2E_26)
    );
    assert_eq!(
        stack_at(8).unwrap().component_patch.armor_trim_material_id,
        Some(0)
    );
    assert_eq!(
        stack_at(8).unwrap().component_patch.armor_trim_pattern_id,
        Some(0)
    );

    for position in 0..=10 {
        let cursor = (
            STONECUTTER_RECIPE_BUTTON_X
                + position % STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_WIDTH,
            STONECUTTER_RECIPE_ITEM_Y
                + position / STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_HEIGHT,
        );
        let screen = hud_inventory_screen_with_local_state(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            None,
            InventoryHudLocalState {
                cursor_position: Some(cursor),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();
        assert_eq!(screen.floating_items.len(), 10);
        if position == 0 {
            assert_eq!(screen.tooltip, None);
        } else {
            let tooltip = screen.tooltip.unwrap();
            assert_eq!(tooltip.slot_id, u16::MAX);
            assert_eq!((tooltip.x, tooltip.y), cursor);
            assert_eq!(tooltip.lines[0].text, "Test Sword");
        }
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stonecutter_buttons_and_tooltips_follow_current_tag_and_trim_context() {
    let root = unique_runtime_temp_dir("stonecutter-slot-display-context");
    write_stonecutter_slot_display_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = stonecutter_world_with_builtin_slot_displays(1);

    set_stonecutter_item_tag(&mut world, vec![2]);
    let tag_option = stonecutter_visible_recipe_option_stacks(&world, 0)
        .into_iter()
        .find(|option| (option.x, option.y) == stonecutter_recipe_item_position(6))
        .unwrap();
    assert_eq!(tag_option.stack.item_id, Some(2));
    let tag_tooltip =
        stonecutter_recipe_tooltip_at(&world, &item_runtime, stonecutter_recipe_item_position(6));
    assert_eq!(tag_tooltip.lines[0].text, "Command Block");

    clear_stonecutter_item_tags(&mut world);
    assert!(stonecutter_visible_recipe_option_stacks(&world, 0)
        .into_iter()
        .all(|option| (option.x, option.y) != stonecutter_recipe_item_position(6)));
    let removed_tag_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some(stonecutter_recipe_item_position(6)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();
    assert_eq!(removed_tag_screen.tooltip, None);

    let initial_trim_tooltip =
        stonecutter_recipe_tooltip_at(&world, &item_runtime, stonecutter_recipe_item_position(8));
    assert!(
        initial_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Spiral"),
        "{:#?}",
        initial_trim_tooltip.lines
    );
    assert!(
        initial_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Copper"),
        "{:#?}",
        initial_trim_tooltip.lines
    );

    world.record_registry_entries(
        TRIM_MATERIAL_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:opal",
            stonecutter_trim_material_nbt("opal", "Opal"),
        )],
    );
    world.record_registry_entries(
        TRIM_PATTERN_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:wave",
            stonecutter_trim_pattern_nbt("example:wave", "Wave", false),
        )],
    );
    world.set_registry_dependent_item_component_initializers(BTreeMap::from([(
        3,
        vec![
            RegistryDependentItemComponentInitializer::ProvidesTrimMaterial {
                material: "example:opal".to_string(),
            },
        ],
    )]));
    world.rebuild_registry_dependent_item_defaults().unwrap();
    world.apply_update_recipes(stonecutter_builtin_update_recipes(2));

    let trim_option = stonecutter_visible_recipe_option_stacks(&world, 0)
        .into_iter()
        .find(|option| (option.x, option.y) == stonecutter_recipe_item_position(8))
        .unwrap();
    assert_eq!(
        trim_option.stack.component_patch.armor_trim_material_id,
        Some(1)
    );
    assert_eq!(
        trim_option.stack.component_patch.armor_trim_pattern_id,
        Some(1)
    );
    let changed_trim_tooltip =
        stonecutter_recipe_tooltip_at(&world, &item_runtime, stonecutter_recipe_item_position(8));
    assert!(
        changed_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Wave"),
        "{:#?}",
        changed_trim_tooltip.lines
    );
    assert!(
        changed_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Opal"),
        "{:#?}",
        changed_trim_tooltip.lines
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_item_count_label_follows_vanilla_stack_count_rule() {
    assert_eq!(
        hud_item_count_label_for_stack(&item_stack(42, 64)),
        Some(HudItemCountLabel::new("64"))
    );
    assert_eq!(hud_item_count_label_for_stack(&item_stack(42, 1)), None);
    assert_eq!(hud_item_count_label_for_stack(&item_stack(42, 0)), None);
    assert_eq!(
        hud_item_count_label_for_stack(&bbb_protocol::packets::ItemStackSummary::empty()),
        None
    );
}

#[test]
fn hud_item_cooldown_progress_uses_world_cooldown_group_state() {
    let mut world = WorldStore::new();
    world.apply_cooldown(bbb_protocol::packets::Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    });
    world.advance_item_cooldowns(5);
    let mut stack = item_stack(42, 1);
    stack.component_patch.use_cooldown_group = Some("minecraft:ender_pearl".to_string());

    assert_eq!(
        hud_item_cooldown_progress_for_stack(&world, None, &stack, 0.5),
        Some(0.725)
    );
    assert_eq!(
        hud_item_cooldown_progress_for_stack(&world, None, &stack, 1.5),
        Some(0.7)
    );

    stack.component_patch.use_cooldown_group = Some("minecraft:wind_charge".to_string());
    assert_eq!(
        hud_item_cooldown_progress_for_stack(&world, None, &stack, 0.0),
        None
    );
    assert_eq!(
        hud_item_cooldown_progress_for_stack(
            &world,
            None,
            &bbb_protocol::packets::ItemStackSummary::empty(),
            0.0
        ),
        None
    );
}

#[test]
fn item_cooldown_group_requires_runtime_for_default_item_group() {
    let stack = item_stack(42, 1);
    assert_eq!(item_cooldown_group(None, &stack), None);

    let mut explicit_group = stack;
    explicit_group.component_patch.use_cooldown_group = Some("minecraft:custom_group".to_string());
    assert_eq!(
        item_cooldown_group(None, &explicit_group),
        Some("minecraft:custom_group".to_string())
    );
}

#[test]
fn hud_item_durability_bar_follows_vanilla_damage_formula() {
    let world = WorldStore::new();
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 25, false)),
        Some(HudItemDurabilityBar::new(10, [127.0 / 255.0, 1.0, 0.0]))
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 100, false)),
        Some(HudItemDurabilityBar::new(0, [1.0, 0.0, 0.0]))
    );
}

#[test]
fn hud_item_durability_bar_requires_damageable_damaged_stack() {
    let world = WorldStore::new();
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 0, false)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, -5, false)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 25, true)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 0, 100, 25, false)),
        None
    );
    let mut missing_damage = item_stack(42, 1);
    missing_damage.component_patch.max_damage = Some(100);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &missing_damage),
        None
    );

    // No patch `max_damage` and an empty default-item-max-damage table (as
    // when the registry default table hasn't been populated, or the item has
    // no vanilla default): still None, since there is nothing to fall back to.
    let mut missing_max_damage = item_stack(42, 1);
    missing_max_damage.component_patch.damage = Some(25);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &missing_max_damage),
        None
    );

    let mut non_damageable = item_stack_with_damage(42, 1, 0, 25, false);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &non_damageable),
        None
    );
    non_damageable.component_patch.max_damage = Some(-1);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &non_damageable),
        None
    );
}

#[test]
fn hud_item_durability_bar_falls_back_to_default_item_max_damage_table() {
    let mut world = WorldStore::new();
    world.set_default_item_max_damage(std::collections::BTreeMap::from([(42, 100)]));

    // Vanilla protocol patches usually only carry `damage` for a damaged
    // stack, since `max_damage` is a registry default component. The HUD/
    // inventory durability bar must still show, matching the same
    // width/color formula as an explicit patch `max_damage`.
    let mut default_max_damage_only = item_stack(42, 1);
    default_max_damage_only.component_patch.damage = Some(25);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &default_max_damage_only),
        Some(HudItemDurabilityBar::new(10, [127.0 / 255.0, 1.0, 0.0]))
    );

    // An explicit patch `max_damage` still takes priority over the registry
    // default table (100): using the patched 50 (not 100) yields width 10,
    // not the width 12 that the default-table value would produce.
    let mut explicit_max_damage = item_stack(42, 1);
    explicit_max_damage.component_patch.max_damage = Some(50);
    explicit_max_damage.component_patch.damage = Some(10);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &explicit_max_damage),
        Some(HudItemDurabilityBar::new(
            10,
            vanilla_hsv_to_rgb_unit(0.8 / 3.0, 1.0, 1.0)
        ))
    );

    // `minecraft:unbreakable` suppresses the bar even when the registry
    // default table has a `max_damage` entry for the item.
    let mut unbreakable = item_stack(42, 1);
    unbreakable.component_patch.damage = Some(25);
    unbreakable.component_patch.unbreakable = true;
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &unbreakable),
        None
    );

    // An item with no registry default entry (e.g. a non-damageable item)
    // still falls back to None when the patch omits `max_damage`.
    let mut other_item = item_stack(7, 1);
    other_item.component_patch.damage = Some(25);
    assert_eq!(hud_item_durability_bar_for_stack(&world, &other_item), None);
}

#[test]
fn block_destroy_overlays_include_server_progress_and_keep_highest_per_position() {
    let mut world = WorldStore::new();
    let textures = destroy_stage_test_textures();
    let pos = bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 };
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos,
            progress: 2,
        })
    );
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 11,
            pos,
            progress: 7,
        })
    );
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 12,
            pos: bbb_protocol::packets::BlockPos { x: 1, y: 3, z: 4 },
            progress: 3,
        })
    );

    let stages = block_destroy_stages_from_world(&world);
    let overlays = block_destroy_overlays_from_stages(&world, &textures, &stages);

    assert_eq!(
        stages,
        vec![
            (BlockPos { x: 1, y: 3, z: 4 }, 3),
            (BlockPos { x: 2, y: 3, z: 4 }, 7)
        ]
    );
    assert_eq!(overlays.len(), 2);
    assert_eq!(overlays[0].pos, [1, 3, 4]);
    assert_eq!(overlays[0].uv, textures.destroy_stage_uv_rect(3).unwrap());
    assert_eq!(overlays[1].pos, [2, 3, 4]);
    assert_eq!(overlays[1].uv, textures.destroy_stage_uv_rect(7).unwrap());
}

#[test]
fn block_destroy_overlays_merge_local_stage_with_server_progress_per_position() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let pos = BlockPos { x: 0, y: 1, z: 3 };
    set_lightmap_test_block(&mut world, pos, 9);
    assert_eq!(
        world.probe_block(pos).unwrap().block_name.as_deref(),
        Some("minecraft:grass_block")
    );
    world.set_default_block_destroy_profiles(std::collections::BTreeMap::from([(
        "minecraft:grass_block".to_string(),
        WorldBlockDestroyProfile {
            destroy_time_tenths: Some(6),
            requires_correct_tool: false,
        },
    )]));
    world.set_local_destroying_block_hit(pos, bbb_protocol::packets::Direction::North);
    for _ in 0..10 {
        assert_eq!(world.advance_local_destroying_block_tick(), None);
    }
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        Some(5)
    );
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos: bbb_protocol::packets::BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            progress: 2,
        })
    );
    let textures = destroy_stage_test_textures();

    let stages = block_destroy_stages_from_world(&world);
    let overlays = block_destroy_overlays_from_stages(&world, &textures, &stages);

    assert_eq!(stages, vec![(pos, 5)]);
    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].pos, [0, 1, 3]);
    assert_eq!(overlays[0].uv, textures.destroy_stage_uv_rect(5).unwrap());
    // The overlay now carries the block's render shape (grass_block has no model in the test
    // textures, so it projects to the full-cube crack).
    assert_eq!(
        overlays[0].shape,
        bbb_renderer::terrain::TerrainRenderShape::Cube
    );
}

#[test]
fn block_destroy_overlays_skip_missing_destroy_stage_textures() {
    let mut world = WorldStore::new();
    let textures = TerrainTextureState::default();
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos: bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 },
            progress: 2,
        })
    );

    let stages = block_destroy_stages_from_world(&world);
    assert_eq!(stages, vec![(BlockPos { x: 2, y: 3, z: 4 }, 2)]);
    assert!(block_destroy_overlays_from_stages(&world, &textures, &stages).is_empty());
}

#[test]
fn block_entity_crumbling_acceptance_matches_vanilla_model_renderers() {
    let accepted = [
        bbb_renderer::EntityModelKind::Chest {
            texture: bbb_renderer::ChestModelTexture::Normal,
            half: bbb_renderer::ChestModelHalf::Single,
        },
        bbb_renderer::EntityModelKind::Sign {
            wood: bbb_renderer::SignModelWood::Oak,
            attachment: bbb_renderer::SignModelAttachment::Standing,
        },
        bbb_renderer::EntityModelKind::Bed {
            color: bbb_renderer::EntityDyeColor::Red,
            part: bbb_renderer::BedModelPart::Head,
        },
        bbb_renderer::EntityModelKind::Bell,
        bbb_renderer::EntityModelKind::ShulkerBox {
            color: None,
            facing: bbb_renderer::EntityAttachmentFace::Up,
        },
        bbb_renderer::EntityModelKind::Banner {
            wall: false,
            base_color: bbb_renderer::EntityDyeColor::White,
            layers: [None; 16],
        },
        bbb_renderer::EntityModelKind::EnchantingBook,
        bbb_renderer::EntityModelKind::LecternBook,
        bbb_renderer::EntityModelKind::SkullBlock {
            skull: bbb_renderer::EntityCustomHeadSkull::Skeleton,
            attachment: bbb_renderer::SkullBlockModelAttachment::Ground,
        },
        bbb_renderer::EntityModelKind::Conduit {
            part: bbb_renderer::ConduitModelPart::Shell,
        },
        bbb_renderer::EntityModelKind::Conduit {
            part: bbb_renderer::ConduitModelPart::Cage,
        },
    ];
    assert!(accepted
        .into_iter()
        .all(block_entity_model_accepts_crumbling));

    let rejected = [
        bbb_renderer::EntityModelKind::DecoratedPot {
            back: None,
            left: None,
            right: None,
            front: None,
        },
        bbb_renderer::EntityModelKind::EndPortalBlock {
            kind: bbb_renderer::EndPortalModelKind::EndPortal,
            faces: [
                bbb_renderer::EndPortalModelFace::Down,
                bbb_renderer::EndPortalModelFace::Up,
            ],
        },
        bbb_renderer::EntityModelKind::Conduit {
            part: bbb_renderer::ConduitModelPart::OuterWind { phase: 1 },
        },
        bbb_renderer::EntityModelKind::Conduit {
            part: bbb_renderer::ConduitModelPart::InnerWind { vertical: false },
        },
        bbb_renderer::EntityModelKind::Conduit {
            part: bbb_renderer::ConduitModelPart::Eye { open: true },
        },
    ];
    assert!(rejected
        .into_iter()
        .all(|kind| !block_entity_model_accepts_crumbling(kind)));
}

#[test]
fn block_entity_crumbling_uses_shared_highest_stage_and_clears_ineligible_models() {
    let textures = destroy_stage_test_textures();
    let pos = BlockPos { x: 2, y: 3, z: 4 };
    let stages = vec![(pos, 7)];
    let stale = Some(bbb_renderer::BlockEntityCrumbling {
        origin: [99.0; 3],
        uv: textures.destroy_stage_uv_rect(1).unwrap(),
    });
    let mut instances = vec![
        bbb_renderer::EntityModelInstance::chest(
            -1,
            [2.0, 3.0, 4.0],
            0.0,
            bbb_renderer::ChestModelTexture::Normal,
            bbb_renderer::ChestModelHalf::Single,
        ),
        bbb_renderer::EntityModelInstance::conduit(
            -1,
            [2.0, 3.0, 4.0],
            bbb_renderer::ConduitModelPart::Shell,
        ),
        bbb_renderer::EntityModelInstance::conduit(
            -1,
            [2.0, 3.0, 4.0],
            bbb_renderer::ConduitModelPart::Cage,
        ),
        bbb_renderer::EntityModelInstance::conduit(
            -1,
            [2.0, 3.0, 4.0],
            bbb_renderer::ConduitModelPart::OuterWind { phase: 1 },
        )
        .with_block_entity_crumbling(stale),
        bbb_renderer::EntityModelInstance::decorated_pot(
            -1,
            [2.0, 3.0, 4.0],
            0.0,
            None,
            None,
            None,
            None,
        )
        .with_block_entity_crumbling(stale),
        bbb_renderer::EntityModelInstance::bell(-1, [5.0, 6.0, 7.0])
            .with_block_entity_crumbling(stale),
    ];

    apply_block_entity_crumbling(&mut instances, &stages, &textures);

    let expected = Some(bbb_renderer::BlockEntityCrumbling {
        origin: [2.0, 3.0, 4.0],
        uv: textures.destroy_stage_uv_rect(7).unwrap(),
    });
    assert_eq!(instances[0].render_state.block_entity_crumbling, expected);
    assert_eq!(instances[1].render_state.block_entity_crumbling, expected);
    assert_eq!(instances[2].render_state.block_entity_crumbling, expected);
    for instance in &instances[3..] {
        assert_eq!(instance.render_state.block_entity_crumbling, None);
    }
}

#[test]
fn block_destroy_render_ticks_respect_frozen_world_ticking_state() {
    let mut world = WorldStore::new();
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos: bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 },
            progress: 2,
        })
    );
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });

    let running_ticks = world.consume_running_render_ticks(420);
    assert_eq!(
        advance_block_destruction_render_ticks(&mut world, running_ticks),
        0
    );
    assert_eq!(world.block_destructions().len(), 1);

    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 420 });
    let running_ticks = world.consume_running_render_ticks(420);
    assert_eq!(
        advance_block_destruction_render_ticks(&mut world, running_ticks),
        1
    );
    assert!(world.block_destructions().is_empty());
    assert_eq!(world.ticking().frozen_ticks_to_run, 0);
}

#[test]
fn entity_client_animations_advance_at_vanilla_tick_interval() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(123, 104));
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 123,
            values: vec![test_bool_data(18, true)],
        })
    );

    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_world::EntityPickBoundsState::from_base_size(
            1.4, 1.4, 0.0
        ))
    );

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(49),),
        0
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_world::EntityPickBoundsState::from_base_size(
            1.4, 1.4, 0.0
        ))
    );

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(50),),
        1
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(50),),
        0
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_world::EntityPickBoundsState::from_base_size(
            1.4, 1.4, 0.0
        ))
    );

    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(100),
        ),
        1
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_world::EntityPickBoundsState::from_base_size(
            1.4,
            1.4 * (1.0 + 1.0 / 6.0),
            0.0,
        ))
    );

    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(350),
        ),
        5
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_world::EntityPickBoundsState::from_base_size(
            1.4, 2.8, 0.0
        ))
    );
}

#[test]
fn entity_client_tick_cadence_tracks_normal_world_tick_rate() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });

    assert_eq!(
        world_client_tick_interval(&world),
        Duration::from_millis(100)
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(99),),
        0
    );
    assert!((ticks.entity_partial_tick() - 0.99).abs() < 0.000_001);
    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(100),
        ),
        1
    );
    assert_eq!(ticks.entity_partial_tick(), 0.0);

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 40.0,
        frozen: false,
    });
    assert_eq!(
        world_client_tick_interval(&world),
        Duration::from_millis(50)
    );
}

#[test]
fn entity_client_tick_cadence_preserves_residual_across_rate_changes_and_freeze() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(25),),
        0
    );
    assert_eq!(ticks.entity_partial_tick(), 0.5);

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(75),),
        1
    );
    assert_eq!(ticks.entity_partial_tick(), 0.0);

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 1.0,
        frozen: true,
    });
    assert_eq!(
        world_client_tick_interval(&world),
        Duration::from_millis(50)
    );
    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(125),
        ),
        1
    );
}

#[test]
fn entity_client_tick_batch_shares_the_frozen_step_budget_with_world_time() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(123, 104));
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });
    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 1 });

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    let advanced_ticks = advance_entity_client_animations(
        &mut world,
        &mut ticks,
        start + Duration::from_millis(151),
    );
    assert_eq!(advanced_ticks, 3);
    assert_eq!(
        world.probe_entity(123).unwrap().client_animations.age_ticks,
        1
    );
    assert_eq!(ticks.last_running_level_ticks, 1);
    assert_eq!(world.ticking().frozen_ticks_to_run, 0);
}

#[test]
fn entity_client_tick_catch_up_is_capped_at_ten_per_step() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    let delayed = start + Duration::from_millis(1_050);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, delayed),
        VANILLA_MAX_CLIENT_TICKS_PER_STEP
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, delayed),
        VANILLA_MAX_CLIENT_TICKS_PER_STEP
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, delayed),
        1
    );
    assert_eq!(ticks.entity_partial_tick(), 0.0);
}

#[test]
fn local_prediction_consumes_the_runtime_client_tick_batch() {
    let source = include_str!("../runtime.rs");
    assert!(source.contains(
        "advance_player_input_for_client_ticks(\n        input,\n        world,\n        net_counters,\n        net_commands,\n        now,\n        advanced_ticks,\n    );"
    ));
}

#[test]
fn client_tick_end_commands_follow_movement_and_require_active_level() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();

    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );
    queue_client_tick_end_commands(&world, &commands, 2);

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert_eq!(rx.try_recv().unwrap(), NetCommand::ClientTickEnd);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::ClientTickEnd);

    world.clear_client_level();
    queue_client_tick_end_commands(&world, &commands, 1);
    assert!(matches!(
        rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[test]
fn client_tick_end_is_queued_after_native_tick_commands_and_particle_effects() {
    let source = include_str!("../runtime.rs");
    let movement = source
        .find("    advance_player_input_for_client_ticks(\n        input,")
        .expect("pump should advance player movement");
    let use_item = source
        .find("    advance_using_item_at_partial_tick(\n        input,")
        .expect("pump should advance use-item input");
    let particles = source
        .find("    submit_ominous_item_spawner_particles(")
        .expect("simulation should emit particle effects");
    let tick_end = source
        .find("    queue_client_tick_end_commands(world, net_commands, advanced_ticks);")
        .expect("pump should queue native client tick-end commands");

    assert!(movement < tick_end);
    assert!(use_item < tick_end);
    assert!(particles < tick_end);
}

#[test]
fn native_event_loop_owns_live_world_without_a_shared_lock() {
    let source = include_str!("../main.rs");
    assert!(!source.contains(&["world", "_shared"].concat()));
    assert!(!source.contains(&["world lock", " poisoned"].concat()));
}

fn local_player_pose(position: [f64; 3], y_rot: f32, x_rot: f32) -> LocalPlayerPoseState {
    LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        y_rot,
        x_rot,
        ..LocalPlayerPoseState::default()
    }
}

fn destroy_stage_test_textures() -> TerrainTextureState {
    let images = (0..10)
        .map(|stage| {
            bbb_pack::SpriteImage::new(
                format!("minecraft:block/destroy_stage_{stage}"),
                1,
                1,
                vec![255, 255, 255, 255],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let atlas = bbb_pack::AtlasPacker::new(16, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    TerrainTextureState::from_layout(&atlas.layout, None, None, None)
}

fn test_add_entity(id: i32, entity_type_id: i32) -> bbb_protocol::packets::AddEntity {
    bbb_protocol::packets::AddEntity {
        id,
        uuid: uuid::Uuid::from_u128(id as u128),
        entity_type_id,
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn add_remote_supporting_block_test_entity(
    world: &mut WorldStore,
    entity_id: i32,
    block_x: i32,
    entity_y: f64,
    block_state_id: i32,
) {
    let mut packet = test_add_entity(
        entity_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    );
    packet.position = bbb_protocol::packets::Vec3d {
        x: f64::from(block_x) + 0.5,
        y: entity_y,
        z: 0.5,
    };
    world.apply_add_entity(packet);
    set_lightmap_test_block(
        world,
        BlockPos {
            x: block_x,
            y: 0,
            z: 0,
        },
        block_state_id,
    );
    assert!(
        world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
            id: entity_id,
            position: bbb_protocol::packets::Vec3d {
                x: f64::from(block_x) + 0.5,
                y: entity_y,
                z: 0.5,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
}

fn test_bool_data(data_id: u8, value: bool) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: bbb_protocol::packets::EntityDataValueKind::Boolean(value),
    }
}

fn test_byte_data(data_id: u8, value: i8) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: bbb_protocol::packets::EntityDataValueKind::Byte(value),
    }
}

fn test_humanoid_arm_data(data_id: u8, arm_id: i32) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
        value: bbb_protocol::packets::EntityDataValueKind::HumanoidArm(arm_id),
    }
}

fn test_item_stack_data(
    data_id: u8,
    item: bbb_protocol::packets::ItemStackSummary,
) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        value: bbb_protocol::packets::EntityDataValueKind::ItemStack(item),
    }
}

fn record_trim_material_registry(world: &mut WorldStore) {
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:trim_material".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:quartz".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:iron".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
}

fn efficiency_enchantment_attribute_nbt() -> Vec<u8> {
    let mut out = vec![10];
    out.push(10);
    write_enchantment_nbt_string(&mut out, "description");
    for (name, value) in [
        ("translate", "enchantment.example.synced_efficiency"),
        ("color", "#123456"),
    ] {
        out.push(8);
        write_enchantment_nbt_string(&mut out, name);
        write_enchantment_nbt_string(&mut out, value);
    }
    out.push(0);
    out.push(3);
    write_enchantment_nbt_string(&mut out, "max_level");
    out.extend(3i32.to_be_bytes());
    out.push(9);
    write_enchantment_nbt_string(&mut out, "slots");
    out.push(8);
    out.extend(1i32.to_be_bytes());
    write_enchantment_nbt_string(&mut out, "mainhand");

    out.push(10);
    write_enchantment_nbt_string(&mut out, "effects");
    out.push(9);
    write_enchantment_nbt_string(&mut out, "minecraft:attributes");
    out.push(10);
    out.extend(1i32.to_be_bytes());
    out.push(10);
    write_enchantment_nbt_string(&mut out, "amount");
    out.push(8);
    write_enchantment_nbt_string(&mut out, "type");
    write_enchantment_nbt_string(&mut out, "minecraft:levels_squared");
    out.push(5);
    write_enchantment_nbt_string(&mut out, "added");
    out.extend(1.0f32.to_be_bytes());
    out.push(0);
    for (name, value) in [
        ("attribute", "minecraft:mining_efficiency"),
        ("id", "minecraft:enchantment.efficiency"),
        ("operation", "add_value"),
    ] {
        out.push(8);
        write_enchantment_nbt_string(&mut out, name);
        write_enchantment_nbt_string(&mut out, value);
    }
    out.push(0);
    out.push(0);
    out.push(0);
    out
}

fn crossbow_charge_time_enchantment_nbt(seconds: f32) -> Vec<u8> {
    let mut out = vec![10];
    out.push(10);
    write_enchantment_nbt_string(&mut out, "effects");
    out.push(10);
    write_enchantment_nbt_string(&mut out, "minecraft:crossbow_charge_time");
    out.push(8);
    write_enchantment_nbt_string(&mut out, "type");
    write_enchantment_nbt_string(&mut out, "minecraft:set");
    out.push(5);
    write_enchantment_nbt_string(&mut out, "value");
    out.extend(seconds.to_be_bytes());
    out.extend([0, 0, 0]);
    out
}

fn write_enchantment_nbt_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.extend((bytes.len() as u16).to_be_bytes());
    out.extend(bytes);
}

fn merchant_offers(
    container_id: i32,
    offer_count: usize,
    out_of_stock_index: Option<usize>,
) -> MerchantOffers {
    MerchantOffers {
        container_id,
        offers: (0..offer_count)
            .map(|index| MerchantOffer {
                buy_a: item_cost(42 + index as i32, 3),
                sell: item_stack(99 + index as i32, 1),
                buy_b: (index % 2 == 0).then(|| item_cost(52 + index as i32, 2)),
                is_out_of_stock: out_of_stock_index == Some(index),
                uses: if out_of_stock_index == Some(index) {
                    12
                } else {
                    1
                },
                max_uses: 12,
                xp: 8,
                special_price_diff: -2,
                price_multiplier: 0.05,
                demand: 6,
            })
            .collect(),
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: true,
    }
}

fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

fn stonecutter_world_with_builtin_slot_displays(encoded_pattern_holder_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[0] = item_stack(3, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.set_furnace_fuel_item_ids(BTreeSet::from([1]));
    world.set_default_item_component_type_ids(BTreeMap::from([
        (1, BTreeSet::from([VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID])),
        (2, BTreeSet::from([VANILLA_DATA_COMPONENT_DYE_TYPE_ID])),
    ]));
    world.set_default_item_dye_color_ids(BTreeMap::from([(2, VANILLA_DYE_COLOR_RED_ID)]));
    set_stonecutter_item_tag(&mut world, vec![1]);
    world.record_registry_entries(
        TRIM_MATERIAL_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:copper",
            stonecutter_trim_material_nbt("copper", "Copper"),
        )],
    );
    world.record_registry_entries(
        TRIM_PATTERN_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:spiral",
            stonecutter_trim_pattern_nbt("example:spiral", "Spiral", true),
        )],
    );
    world.set_registry_dependent_item_component_initializers(BTreeMap::from([(
        3,
        vec![
            RegistryDependentItemComponentInitializer::ProvidesTrimMaterial {
                material: "example:copper".to_string(),
            },
        ],
    )]));
    world.rebuild_registry_dependent_item_defaults().unwrap();
    let packet = stonecutter_builtin_update_recipes(encoded_pattern_holder_id);
    for recipe in &packet.stonecutter_recipes {
        world
            .resolve_slot_display_first_stack(&recipe.option_display)
            .unwrap();
    }
    world.apply_update_recipes(packet);
    assert_eq!(world.recipes().stonecutter_recipes.len(), 11);
    world
}

fn stonecutter_builtin_update_recipes(
    encoded_pattern_holder_id: i32,
) -> bbb_protocol::packets::UpdateRecipes {
    let item = stonecutter_item_display(1);
    let item_stack = stonecutter_item_stack_display(1, 3);
    let displays = vec![
        slot_display_type(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
        slot_display_type(VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID),
        slot_display_unary(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID, item.clone()),
        slot_display_only_with_component(item.clone(), VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID),
        item.clone(),
        item_stack,
        slot_display_tag("example:choices"),
        slot_display_binary(
            VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
            stonecutter_item_display(2),
            item.clone(),
        ),
        slot_display_smithing_trim(
            item.clone(),
            stonecutter_item_display(3),
            encoded_pattern_holder_id,
        ),
        slot_display_with_remainder(item.clone(), stonecutter_item_display(2)),
        slot_display_composite(vec![
            slot_display_type(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID),
            item,
        ]),
    ];
    bbb_protocol::packets::UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: displays
            .into_iter()
            .map(
                |option_display| bbb_protocol::packets::StonecutterSelectableRecipeSummary {
                    input: bbb_protocol::packets::IngredientSummary {
                        tag: None,
                        item_ids: vec![3],
                    },
                    option_display,
                },
            )
            .collect(),
    }
}

fn stonecutter_recipe_item_position(position: i32) -> (i32, i32) {
    (
        STONECUTTER_RECIPE_BUTTON_X
            + position % STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_WIDTH,
        STONECUTTER_RECIPE_ITEM_Y
            + position / STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_HEIGHT,
    )
}

fn stonecutter_recipe_tooltip_at(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    cursor_position: (i32, i32),
) -> HudInventoryTooltip {
    hud_inventory_screen_with_local_state(
        world,
        Some(item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some(cursor_position),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap()
    .tooltip
    .unwrap()
}

fn set_stonecutter_item_tag(world: &mut WorldStore, entries: Vec<i32>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![bbb_protocol::packets::TagNetworkPayload {
                tag: "example:choices".to_string(),
                entries,
            }],
        }],
    });
}

fn clear_stonecutter_item_tags(world: &mut WorldStore) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: Vec::new(),
        }],
    });
}

fn slot_display_type(display_type_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(display_type_id);
    slot_display_summary(display_type_id, payload.into_inner())
}

fn slot_display_unary(
    display_type_id: i32,
    display: bbb_protocol::packets::SlotDisplaySummary,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(&display.raw_payload);
    slot_display_summary(display_type_id, payload.into_inner())
}

fn slot_display_binary(
    display_type_id: i32,
    first: bbb_protocol::packets::SlotDisplaySummary,
    second: bbb_protocol::packets::SlotDisplaySummary,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(display_type_id);
    payload.write_bytes(&first.raw_payload);
    payload.write_bytes(&second.raw_payload);
    slot_display_summary(display_type_id, payload.into_inner())
}

fn slot_display_only_with_component(
    source: bbb_protocol::packets::SlotDisplaySummary,
    component_type_id: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID);
    payload.write_bytes(&source.raw_payload);
    payload.write_var_i32(component_type_id);
    slot_display_summary(
        VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID,
        payload.into_inner(),
    )
}

fn slot_display_smithing_trim(
    base: bbb_protocol::packets::SlotDisplaySummary,
    material: bbb_protocol::packets::SlotDisplaySummary,
    encoded_pattern_holder_id: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID);
    payload.write_bytes(&base.raw_payload);
    payload.write_bytes(&material.raw_payload);
    payload.write_var_i32(encoded_pattern_holder_id);
    slot_display_summary(
        VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
        payload.into_inner(),
    )
}

fn slot_display_summary(
    display_type_id: i32,
    raw_payload: Vec<u8>,
) -> bbb_protocol::packets::SlotDisplaySummary {
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id,
        raw_payload,
        item_stack: None,
        tag: None,
    }
}

fn stonecutter_recipe_display(
    input_item_id: i32,
    result_item_id: i32,
    result_count: i32,
) -> bbb_protocol::packets::StonecutterSelectableRecipeSummary {
    bbb_protocol::packets::StonecutterSelectableRecipeSummary {
        input: bbb_protocol::packets::IngredientSummary {
            tag: None,
            item_ids: vec![input_item_id],
        },
        option_display: stonecutter_item_stack_display(result_item_id, result_count),
    }
}

fn stonecutter_item_display(item_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = Vec::new();
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 4);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, item_id);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload,
        item_stack: Some(item_stack(item_id, 1)),
        tag: None,
    }
}

fn stonecutter_item_stack_display(
    item_id: i32,
    count: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = Vec::new();
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 5);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, item_id);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, count);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 0);
    bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 0);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
        raw_payload,
        item_stack: Some(item_stack(item_id, count)),
        tag: None,
    }
}

fn slot_display_tag(tag: &str) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = bbb_protocol::codec::Encoder::new();
    raw_payload.write_var_i32(6);
    raw_payload.write_string(tag);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: Some(tag.to_string()),
    }
}

fn slot_display_composite(
    contents: Vec<bbb_protocol::packets::SlotDisplaySummary>,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = bbb_protocol::codec::Encoder::new();
    raw_payload.write_var_i32(10);
    raw_payload.write_var_i32(i32::try_from(contents.len()).unwrap());
    for content in &contents {
        raw_payload.write_bytes(&content.raw_payload);
    }
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: None,
    }
}

fn slot_display_with_remainder(
    input: bbb_protocol::packets::SlotDisplaySummary,
    remainder: bbb_protocol::packets::SlotDisplaySummary,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut raw_payload = bbb_protocol::codec::Encoder::new();
    raw_payload.write_var_i32(9);
    raw_payload.write_bytes(&input.raw_payload);
    raw_payload.write_bytes(&remainder.raw_payload);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: None,
    }
}

fn cartography_table_world_with_items(
    items: Vec<bbb_protocol::packets::ItemStackSummary>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world
}

fn map_stack(
    item_id: i32,
    count: i32,
    map_id: i32,
    post_processing: Option<bbb_protocol::packets::MapPostProcessingSummary>,
) -> bbb_protocol::packets::ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.map_id = Some(map_id);
    item.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID);
    if let Some(post_processing) = post_processing {
        item.component_patch.map_post_processing = Some(post_processing);
        item.component_patch.added_type_ids.push(48);
    }
    item
}

fn item_stack(item_id: i32, count: i32) -> bbb_protocol::packets::ItemStackSummary {
    bbb_protocol::packets::ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn open_recipe_book_crafting_table_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world
}

fn open_recipe_book_furnace_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        furnace: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world
}

fn recipe_book_ghost_item_runtime() -> NativeItemRuntime {
    let root = unique_runtime_temp_dir("recipe-book-ghost-items");
    let assets = runtime_assets_dir(&root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    for (model_id, rgba) in [
        ("crafting_table", [120, 80, 40, 255]),
        ("stick", [150, 95, 45, 255]),
        ("oak_planks", [190, 145, 80, 255]),
        ("iron_axe", [180, 180, 190, 255]),
        ("golden_sword", [240, 210, 80, 255]),
    ] {
        write_runtime_json(
            &assets.join("items").join(format!("{model_id}.json")),
            &format!(
                r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{model_id}"
                    }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, model_id, &rgba);
    }
    write_runtime_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
            "item.minecraft.crafting_table": "Crafting Table",
            "item.minecraft.stick": "Stick",
            "item.minecraft.oak_planks": "Wooden Boards",
            "item.minecraft.iron_axe": "Iron Axe",
            "item.minecraft.golden_sword": "Golden Sword"
        }"#,
    );
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &[
            "crafting_table",
            "stick",
            "oak_planks",
            "iron_axe",
            "golden_sword",
        ],
    );
    let runtime = NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    runtime
}

fn crafting_recipe_book_search_recipe_indices(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    search_text: &str,
) -> Vec<i32> {
    crafting_recipe_book_collections(
        world,
        RecipeBookCraftingGrid {
            width: 3,
            height: 3,
        },
        1,
        false,
        search_text,
        Some(item_runtime),
    )
    .iter()
    .filter_map(|collection| {
        collection
            .recipe_index_and_craftable_at_slot_select_index(0)
            .map(|(recipe_index, _)| recipe_index)
    })
    .collect()
}

fn shapeless_crafting_recipe_book_entry(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    shapeless_crafting_recipe_book_entry_with_requirements(
        id,
        category_id,
        group,
        result_item_id,
        Vec::new(),
    )
}

fn shapeless_crafting_recipe_book_entry_with_display_ingredients(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    display_ingredients: Vec<bbb_protocol::packets::SlotDisplaySummary>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    let mut entry = shapeless_crafting_recipe_book_entry(id, category_id, group, result_item_id);
    if let Some(bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
        ingredients,
        ..
    }) = entry.contents.display.crafting.as_mut()
    {
        *ingredients = display_ingredients;
    }
    entry
}

fn shapeless_crafting_recipe_book_entry_with_requirements(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    requirements: Vec<Vec<i32>>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    shapeless_crafting_recipe_book_entry_with_requirement_summaries(
        id,
        category_id,
        group,
        result_item_id,
        requirements
            .into_iter()
            .map(|item_ids| bbb_protocol::packets::IngredientSummary {
                tag: None,
                item_ids,
            })
            .collect(),
    )
}

fn shapeless_crafting_recipe_book_entry_with_requirement_summaries(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    requirements: Vec<bbb_protocol::packets::IngredientSummary>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
                raw_body: Vec::new(),
                crafting: Some(
                    bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                        ingredients: Vec::new(),
                        result: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                            raw_payload: Vec::new(),
                            item_stack: Some(item_stack(result_item_id, 1)),
                            tag: None,
                        },
                        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                            raw_payload: Vec::new(),
                            item_stack: None,
                            tag: None,
                        },
                    },
                ),
                furnace: None,
            },
            group,
            category_id,
            crafting_requirements: (!requirements.is_empty()).then_some(requirements),
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn furnace_recipe_book_entry(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    furnace_recipe_book_entry_with_requirements(id, category_id, group, result_item_id, Vec::new())
}

fn furnace_recipe_book_entry_with_requirements(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
    requirements: Vec<Vec<i32>>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Furnace,
                raw_body: Vec::new(),
                crafting: None,
                furnace: Some(furnace_recipe_display_body(2, 1, result_item_id)),
            },
            group,
            category_id,
            crafting_requirements: (!requirements.is_empty()).then(|| {
                requirements
                    .into_iter()
                    .map(|item_ids| bbb_protocol::packets::IngredientSummary {
                        tag: None,
                        item_ids,
                    })
                    .collect()
            }),
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn furnace_recipe_display(
    ingredient_item_id: i32,
    fuel_item_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeDisplaySummary {
    bbb_protocol::packets::RecipeDisplaySummary {
        display_type: bbb_protocol::packets::RecipeDisplayType::Furnace,
        raw_body: Vec::new(),
        crafting: None,
        furnace: Some(furnace_recipe_display_body(
            ingredient_item_id,
            fuel_item_id,
            result_item_id,
        )),
    }
}

fn furnace_recipe_display_body(
    ingredient_item_id: i32,
    fuel_item_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::FurnaceRecipeDisplaySummary {
    bbb_protocol::packets::FurnaceRecipeDisplaySummary {
        ingredient: stonecutter_item_display(ingredient_item_id),
        fuel: stonecutter_item_display(fuel_item_id),
        result: stonecutter_item_stack_display(result_item_id, 1),
        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
            raw_payload: vec![0],
            item_stack: None,
            tag: None,
        },
        duration: 200,
        experience_bits: 0.0_f32.to_bits(),
    }
}

fn apply_item_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

fn apply_block_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

fn apply_fluid_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:fluid".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

fn apply_entity_type_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:entity_type".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| bbb_protocol::packets::TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

fn written_book_stack(pages: Vec<&str>) -> bbb_protocol::packets::ItemStackSummary {
    let mut item = item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    item.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".to_string(),
        title_filter: None,
        author: "Alex".to_string(),
        generation: 0,
        pages,
        page_filters,
        resolved: true,
    });
    item
}

fn item_stack_with_damage(
    item_id: i32,
    count: i32,
    max_damage: i32,
    damage: i32,
    unbreakable: bool,
) -> bbb_protocol::packets::ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.max_damage = Some(max_damage);
    item.component_patch.damage = Some(damage);
    item.component_patch.unbreakable = unbreakable;
    item
}

fn write_stonecutter_slot_display_item_assets(root: &Path) {
    write_runtime_tooltip_item_assets(root);
    let assets = runtime_assets_dir(root);
    for (item, color) in [
        ("test_sword", [180, 180, 190, 255]),
        ("command_block", [180, 140, 100, 255]),
    ] {
        write_runtime_json(
            &assets.join("items").join(format!("{item}.json")),
            &format!(
                r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{item}"
                    }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, item, &color);
    }
}

fn stonecutter_trim_material_nbt(asset_name: &str, description: &str) -> Vec<u8> {
    let mut out = vec![10];
    write_stonecutter_nbt_string_field(&mut out, "asset_name", asset_name);
    write_stonecutter_trim_description(&mut out, description);
    out.push(0);
    out
}

fn stonecutter_trim_pattern_nbt(asset_id: &str, description: &str, decal: bool) -> Vec<u8> {
    let mut out = vec![10];
    write_stonecutter_nbt_string_field(&mut out, "asset_id", asset_id);
    write_stonecutter_trim_description(&mut out, description);
    out.push(1);
    write_stonecutter_nbt_string(&mut out, "decal");
    out.push(u8::from(decal));
    out.push(0);
    out
}

fn write_stonecutter_trim_description(out: &mut Vec<u8>, text: &str) {
    out.push(10);
    write_stonecutter_nbt_string(out, "description");
    write_stonecutter_nbt_string_field(out, "text", text);
    out.push(0);
}

fn write_stonecutter_nbt_string_field(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    write_stonecutter_nbt_string(out, name);
    write_stonecutter_nbt_string(out, value);
}

fn write_stonecutter_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn write_runtime_tooltip_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "block/",
                    "source": "block"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("models").join("item").join("test_combo.json"),
        r#"{
            "textures": {
                "layer0": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
            "item.minecraft.test_combo": "Test Combo",
            "item.minecraft.test_sword": "Test Sword",
            "block.minecraft.command_block": "Command Block",
            "item.disabled": "Disabled item",
            "item.durability": "Durability: %s / %s",
            "item.components": "%s component(s)",
            "enchantment.minecraft.efficiency": "Efficiency",
            "enchantment.example.synced_efficiency": "Synced Efficiency",
            "enchantment.level.1": "I",
            "enchantment.level.3": "III",
            "attribute.modifier.plus.0": "+%s %s",
            "attribute.modifier.equals.0": "%s %s",
            "attribute.name.attack_damage": "Attack Damage",
            "attribute.name.attack_speed": "Attack Speed",
            "attribute.name.mining_efficiency": "Mining Efficiency",
            "item.modifiers.mainhand": "When in Main Hand:",
            "item.op_warning.line1": "Operator data",
            "item.op_warning.line2": "Can alter commands",
            "item.op_warning.line3": "Use with care"
        }"#,
    );
    write_runtime_png(
        &assets.join("textures").join("item").join("test_combo.png"),
        &[80, 120, 160, 255],
    );
    crate::item_registry_test_fixture::write_item_registry_fixture_with_profiles(
        root.as_ref(),
        &["test_combo", "test_sword", "command_block"],
        &[
            ("command_block", "command_block"),
            ("test_sword", "iron_sword"),
        ],
    );
}

fn write_runtime_bow_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("bow.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "on_false": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/bow"
                },
                "on_true": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_duration",
                    "scale": 0.05,
                    "entries": [
                        {
                            "threshold": 0.65,
                            "model": {
                                "type": "minecraft:model",
                                "model": "minecraft:item/bow_pulling_1"
                            }
                        },
                        {
                            "threshold": 0.9,
                            "model": {
                                "type": "minecraft:model",
                                "model": "minecraft:item/bow_pulling_2"
                            }
                        }
                    ],
                    "fallback": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/bow_pulling_0"
                    }
                }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "bow", &[80, 120, 160, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bow_pulling_0", &[160, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bow_pulling_1", &[120, 160, 80, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "bow_pulling_2", &[160, 120, 80, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root.as_ref(), &["bow"]);
}

fn write_runtime_crossbow_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("crossbow.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:charge_type",
                "cases": [],
                "fallback": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/crossbow"
                    },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:crossbow/pull",
                        "entries": [
                            {
                                "threshold": 0.58,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/crossbow_pulling_1"
                                }
                            },
                            {
                                "threshold": 1.0,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/crossbow_pulling_2"
                                }
                            }
                        ],
                        "fallback": {
                            "type": "minecraft:model",
                            "model": "minecraft:item/crossbow_pulling_0"
                        }
                    }
                }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "crossbow_pulling_0", &[70, 100, 130, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "crossbow_pulling_1", &[100, 130, 70, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "crossbow_pulling_2", &[130, 70, 100, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root.as_ref(), &["crossbow"]);
}

fn write_runtime_main_hand_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("hand_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:main_hand",
                "cases": [
                    {
                        "when": "left",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_left" }
                    },
                    {
                        "when": "right",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_right" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/hand_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "hand_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "hand_selector_left", &[120, 40, 80, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "hand_selector_right", &[80, 120, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["hand_selector"],
    );
}

fn write_runtime_context_dimension_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("dimension_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:context_dimension",
                "cases": [
                    {
                        "when": "minecraft:overworld",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_overworld" }
                    },
                    {
                        "when": "minecraft:the_nether",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_nether" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "dimension_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "dimension_selector_overworld",
        &[80, 120, 40, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "dimension_selector_nether",
        &[120, 40, 80, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["dimension_selector"],
    );
}

fn write_runtime_context_entity_type_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("entity_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:context_entity_type",
                "cases": [
                    {
                        "when": "minecraft:player",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_player" }
                    },
                    {
                        "when": "minecraft:cow",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_cow" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/entity_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "entity_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "entity_selector_player",
        &[80, 120, 40, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "entity_selector_cow", &[120, 40, 80, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["entity_selector"],
    );
}

fn write_runtime_view_entity_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("view_entity_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:view_entity",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/view_entity_condition_view" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/view_entity_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "view_entity_condition",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "view_entity_condition_view",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["view_entity_condition"],
    );
}

fn write_runtime_extended_view_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("extended_view_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:extended_view",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/extended_view_condition_view" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/extended_view_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "extended_view_condition",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "extended_view_condition_view",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["extended_view_condition"],
    );
}

fn write_runtime_keybind_down_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("keybind_down_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:keybind_down",
                "keybind": "key.quickActions",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/keybind_down_condition_use" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/keybind_down_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "keybind_down_condition",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "keybind_down_condition_use",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["keybind_down_condition"],
    );
}

fn write_runtime_fishing_rod_cast_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("fishing_rod.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:fishing_rod/cast",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/fishing_rod_cast" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/fishing_rod" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "fishing_rod", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "fishing_rod_cast", &[120, 80, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(root.as_ref(), &["fishing_rod"]);
}

fn write_runtime_cooldown_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("cooldown_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:cooldown",
                "entries": [
                    {
                        "threshold": 0.725,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/cooldown_selector_active" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/cooldown_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "cooldown_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "cooldown_selector_active",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["cooldown_selector"],
    );
}

fn write_runtime_time_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "daytime",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 1.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_selector_evening" }
                    },
                    {
                        "threshold": 2.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_selector_night" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_selector_day" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_wobbled_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "daytime",
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_wobbled_stateful" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_wobbled_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_random_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "random",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 0.5,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_random_stateful" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_random_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("time_moon_phase_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:time",
                "source": "moon_phase",
                "wobble": false,
                "scale": 8.0,
                "entries": [
                    {
                        "threshold": 4.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/time_moon_phase_new" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/time_moon_phase_full" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "time_selector_day", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "time_selector_evening",
        &[120, 80, 40, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "time_selector_night", &[80, 40, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "time_wobbled_fallback",
        &[45, 75, 115, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "time_wobbled_stateful",
        &[160, 40, 100, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "time_random_fallback", &[50, 70, 110, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "time_random_stateful", &[180, 50, 90, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "time_moon_phase_full", &[40, 70, 130, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "time_moon_phase_new", &[130, 70, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &[
            "time_selector",
            "time_wobbled_selector",
            "time_random_selector",
            "time_moon_phase_selector",
        ],
    );
}

fn write_runtime_spawn_compass_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "spawn",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass_wobbled.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "spawn",
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.5,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_wobbled_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_wobbled_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass_invalid_spin.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "spawn",
                "scale": 1.0,
                "entries": [
                    {
                        "threshold": 0.1,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_invalid_spin" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_invalid_fallback" }
            }
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("spawn_compass_none_spin.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "none",
                "scale": 1.0,
                "entries": [
                    {
                        "threshold": 0.1,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_none_spin" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/spawn_compass_none_fallback" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_fallback",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(&assets, "spawn_compass_east", &[120, 80, 40, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_wobbled_fallback",
        &[45, 75, 115, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_wobbled_east",
        &[140, 90, 50, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_invalid_fallback",
        &[35, 55, 95, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_invalid_spin",
        &[190, 70, 80, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_none_fallback",
        &[30, 45, 90, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "spawn_compass_none_spin",
        &[185, 85, 105, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &[
            "spawn_compass",
            "spawn_compass_wobbled",
            "spawn_compass_invalid_spin",
            "spawn_compass_none_spin",
        ],
    );
}

fn write_runtime_lodestone_compass_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("lodestone_compass.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "lodestone",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/lodestone_compass_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/lodestone_compass_fallback" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "lodestone_compass_fallback",
        &[40, 120, 80, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "lodestone_compass_east",
        &[120, 40, 80, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["lodestone_compass"],
    );
}

fn write_runtime_recovery_compass_range_dispatch_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("recovery_compass.json"),
        r#"{
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:compass",
                "target": "recovery",
                "wobble": false,
                "scale": 4.0,
                "entries": [
                    {
                        "threshold": 3.0,
                        "model": { "type": "minecraft:model", "model": "minecraft:item/recovery_compass_east" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/recovery_compass_fallback" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "recovery_compass_fallback",
        &[40, 80, 120, 255],
    );
    write_flat_runtime_item_model_and_texture(
        &assets,
        "recovery_compass_east",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["recovery_compass"],
    );
}

fn write_runtime_selected_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("selected_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:selected",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/selected_condition_selected" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/selected_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "selected_condition", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "selected_condition_selected",
        &[120, 80, 40, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["selected_condition"],
    );
}

fn write_runtime_carried_condition_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("carried_condition.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:carried",
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/carried_condition_carried" },
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/carried_condition" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "carried_condition", &[40, 120, 80, 255]);
    write_flat_runtime_item_model_and_texture(
        &assets,
        "carried_condition_carried",
        &[120, 40, 80, 255],
    );
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["carried_condition"],
    );
}

fn write_runtime_special_foil_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    for item_id in ["clock", "compass", "spyglass"] {
        write_runtime_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                    "model": {{ "type": "minecraft:model", "model": "minecraft:item/{item_id}" }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, item_id, &[40, 80, 120, 255]);
    }
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    write_runtime_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("item")
            .join("compasses.json"),
        r#"{
            "replace": true,
            "values": ["minecraft:compass"]
        }"#,
    );
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["clock", "compass", "spyglass"],
    );
}

fn write_runtime_trim_material_select_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    write_runtime_json(
        &assets.join("items").join("trim_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:trim_material",
                "cases": [
                    {
                        "when": "minecraft:quartz",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/trim_selector_quartz" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/trim_selector" }
            }
        }"#,
    );
    write_flat_runtime_item_model_and_texture(&assets, "trim_selector", &[40, 80, 120, 255]);
    write_flat_runtime_item_model_and_texture(&assets, "trim_selector_quartz", &[120, 80, 40, 255]);
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["trim_selector"],
    );
}

fn write_flat_runtime_item_model_and_texture(assets: &Path, model_id: &str, rgba: &[u8]) {
    write_runtime_json(
        &assets
            .join("models")
            .join("item")
            .join(format!("{model_id}.json")),
        &format!(
            r#"{{
            "textures": {{
                "layer0": "minecraft:item/{model_id}"
            }}
        }}"#
        ),
    );
    write_runtime_png(
        &assets
            .join("textures")
            .join("item")
            .join(format!("{model_id}.png")),
        rgba,
    );
}

fn write_runtime_game_mode_switcher_item_assets(root: &Path) {
    let assets = runtime_assets_dir(root);
    write_runtime_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_runtime_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": []
        }"#,
    );
    for (item_id, rgba) in [
        ("grass_block", [80, 160, 80, 255]),
        ("iron_sword", [180, 180, 190, 255]),
        ("map", [210, 190, 120, 255]),
        ("ender_eye", [80, 170, 140, 255]),
    ] {
        write_runtime_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{item_id}"
                    }}
                }}"#
            ),
        );
        write_flat_runtime_item_model_and_texture(&assets, item_id, &rgba);
    }
    write_runtime_json(&assets.join("lang").join("en_us.json"), "{}");
    crate::item_registry_test_fixture::write_item_registry_fixture(
        root.as_ref(),
        &["grass_block", "iron_sword", "map", "ender_eye"],
    );
}

fn runtime_assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_pack::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

fn write_runtime_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn write_runtime_png(path: &Path, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::save_buffer(path, rgba, 1, 1, image::ColorType::Rgba8).unwrap();
}

fn unique_runtime_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-native-runtime-{label}-{nanos}"))
}
