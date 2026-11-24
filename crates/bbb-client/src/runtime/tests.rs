use super::hud_debug::*;
use super::*;
use bbb_client_render::{HudSystemToast, HudToast};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod camera_lightmap;
mod component_localization;
mod hotbar_gui;
mod hud_advancements;
mod hud_blocks;
mod hud_debug;
mod hud_frame;
mod hud_menus;
mod hud_overlays;
mod hud_screens;
mod legacy_codes_hud;
mod lightmap_fog;
mod support;
mod weather_sky;
use support::*;

use bbb_client_assets::{
    BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier,
};
use bbb_client_render::{HudComponentTextRun, HudDebugOptionsEntryStatus, HudDebugOptionsRow};
use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID;
use bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID,
};
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_TYPES;
use bbb_protocol::item_types::{VANILLA_ITEM_BOW_ID, VANILLA_ITEM_SPYGLASS_ID};
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPES, VANILLA_MENU_TYPE_ANVIL_ID, VANILLA_MENU_TYPE_BEACON_ID,
    VANILLA_MENU_TYPE_BLAST_FURNACE_ID, VANILLA_MENU_TYPE_BREWING_STAND_ID,
    VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID, VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
    VANILLA_MENU_TYPE_CRAFTING_ID, VANILLA_MENU_TYPE_ENCHANTMENT_ID, VANILLA_MENU_TYPE_FURNACE_ID,
    VANILLA_MENU_TYPE_GENERIC_3X3_ID, VANILLA_MENU_TYPE_GENERIC_9X3_ID,
    VANILLA_MENU_TYPE_GENERIC_9X6_ID, VANILLA_MENU_TYPE_GRINDSTONE_ID, VANILLA_MENU_TYPE_HOPPER_ID,
    VANILLA_MENU_TYPE_LECTERN_ID, VANILLA_MENU_TYPE_LOOM_ID, VANILLA_MENU_TYPE_MERCHANT_ID,
    VANILLA_MENU_TYPE_SHULKER_BOX_ID, VANILLA_MENU_TYPE_SMITHING_ID, VANILLA_MENU_TYPE_SMOKER_ID,
    VANILLA_MENU_TYPE_STONECUTTER_ID,
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
    PlayerCombatKill, RemoveMobEffect, SetBorderLerpSize as ProtocolSetBorderLerpSize,
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

use bbb_client_world::{
    BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, CreativeInventoryTabType,
    HeightmapData, LightData, LocalPlayerPoseState, PaletteDomain, PaletteKind,
    PalettedContainerData, RegistryPacketEntry, WorldDimension, TRIM_MATERIAL_REGISTRY,
    TRIM_PATTERN_REGISTRY,
};
use bbb_protocol::stat_types::{
    VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_CUSTOM_ID,
    VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID, VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
    VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
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
        runs: vec![bbb_client_render::HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: bbb_client_render::HudTextStyle {
                italic,
                ..Default::default()
            },
            color: Some(color),
        }
        .into()],
    }
}

/// A lore tooltip line carrying vanilla `ItemLore.LORE_STYLE`
/// (DARK_PURPLE + italic).
fn tooltip_lore_line(text: &str) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEST_DARK_PURPLE,
        runs: vec![bbb_client_render::HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: bbb_client_render::HudTextStyle {
                italic: true,
                ..Default::default()
            },
            color: Some(0xAA_00_AA),
        }
        .into()],
    }
}

fn tooltip_plain_line(text: &str, tint: [f32; 4]) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![bbb_client_render::HudStyledTextRun::plain(text).into()],
    }
}

fn tooltip_enchantment_line(description: &str, level: &str, color: u32) -> HudInventoryTooltipLine {
    let run = |text: &str| bbb_client_render::HudStyledTextRun {
        shadow_color: None,
        text: text.to_string(),
        style: bbb_client_render::HudTextStyle::default(),
        color: Some(color),
    };
    HudInventoryTooltipLine {
        text: format!("{description} {level}"),
        tint: TOOLTIP_TEST_GRAY,
        runs: vec![run(description).into(), run(" ").into(), run(level).into()],
    }
}

fn tooltip_operator_warning_line(text: &str, bold: bool) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEST_RED,
        runs: vec![bbb_client_render::HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: bbb_client_render::HudTextStyle {
                bold,
                ..Default::default()
            },
            color: Some(0xFF_55_55),
        }
        .into()],
    }
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
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
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
        bbb_client_world::DIMENSION_TYPE_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:custom_sky",
            dynamic_sky_dimension_environment_nbt(),
        )],
    );
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
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
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
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
    let mut out = complete_dimension_type_nbt();
    out.push(10);
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
    let mut out = complete_dimension_type_nbt();
    out.push(10);
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

fn complete_dimension_type_nbt() -> Vec<u8> {
    let mut out = vec![10];
    for (name, value) in [
        ("has_skylight", true),
        ("has_ceiling", false),
        ("has_ender_dragon_fight", false),
    ] {
        out.push(1);
        test_nbt_utf(&mut out, name);
        out.push(u8::from(value));
    }
    out.push(6);
    test_nbt_utf(&mut out, "coordinate_scale");
    out.extend_from_slice(&1.0_f64.to_be_bytes());
    for (name, value) in [
        ("min_y", -64_i32),
        ("height", 384),
        ("logical_height", 384),
        ("monster_spawn_light_level", 0),
        ("monster_spawn_block_light_limit", 0),
    ] {
        out.push(3);
        test_nbt_utf(&mut out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }
    out.push(8);
    test_nbt_utf(&mut out, "infiniburn");
    test_nbt_utf(&mut out, "#minecraft:infiniburn_overworld");
    out.push(5);
    test_nbt_utf(&mut out, "ambient_light");
    out.extend_from_slice(&0.0_f32.to_be_bytes());
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
                name: "text".into(),
                value: NbtSummaryValue::string(text),
            },
            NbtSummaryEntry {
                name: "bold".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("red"),
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
    update: RendererFrameUpdate<Vec<bbb_client_render::HudBossBar>>,
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
    let mut sky = [0; TEST_LIGHT_ARRAY_BYTES];
    set_test_light_nibble(&mut sky, section_block_index(0, 1, 0), sky_light);
    let light_section_index = 0 - (dimension.min_section_y() - 1);
    chunk.light = LightData::default();
    chunk
        .light
        .set_sky_packed(usize::try_from(light_section_index).unwrap(), sky);
    chunk
}

fn weather_effect_test_world(block_state_id: i32) -> WorldStore {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_weather(&mut world, 1.0, 0.0);
    install_biome_environment_profiles(&mut world, &[(42, &[])]);
    world.record_registry_entries(
        "minecraft:sound_event",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:weather.rain"),
            RegistryPacketEntry::stub("minecraft:weather.rain.above"),
        ],
    );
    let mut chunk = empty_lightmap_test_chunk_with_biome(world.dimension(), 42);
    let height_entries = (0..16)
        .flat_map(|z| (0..16).map(move |x| (x, z, 8)))
        .collect::<Vec<_>>();
    chunk.heightmaps = vec![test_motion_blocking_heightmap(
        world.dimension(),
        &height_entries,
    )];
    for section in &mut chunk.sections {
        let section = std::sync::Arc::make_mut(section);
        section.block_states =
            single_value_container(PaletteDomain::BlockStates, 4096, block_state_id);
        section.non_empty_block_count = i16::from(block_state_id != 0) * 4096;
    }
    world.insert_decoded_chunk(chunk);
    world
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
            .map(|_| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
                    biomes: biomes.clone(),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::default(),
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
        .map(|id| {
            RegistryPacketEntry::with_raw_data(
                format!("minecraft:test_biome_{id}"),
                biome_environment_nbt(&[]),
            )
        })
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
        advancement.requirements = vec![vec!["done".into()]];
        added.push(advancement);
        progress.push(AdvancementProgressSummary {
            id,
            criteria: vec![AdvancementCriterionProgressSummary {
                name: "done".into(),
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

fn runtime_tutorial_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.replace_services_public_keys(Vec::new(), vec![vec![1]]);
    world.apply_login(&PlayLogin {
        player_id: 7,
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
            seed: 0,
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

fn enqueue_runtime_recipe_toast(
    world: &mut WorldStore,
    recipe_id: i32,
    category_item_id: i32,
    result_item_id: i32,
) {
    let mut entry = furnace_recipe_book_entry(
        recipe_id,
        VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
        None,
        result_item_id,
    );
    entry
        .contents
        .display
        .furnace
        .as_mut()
        .expect("furnace recipe")
        .crafting_station = stonecutter_item_display(category_item_id);
    entry.flags = 1;
    entry.notification = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: false,
        entries: vec![entry],
    });
}

fn sound_locations(sounds: &[LocalSoundEventState]) -> Vec<&str> {
    sounds
        .iter()
        .filter_map(|sound| sound.sound.location.as_deref())
        .collect()
}

fn beacon_test_world(payment: bool, levels: i16) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 37];
    if payment {
        items[0] = item_stack(42, 1);
    }
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: BEACON_LEVELS_DATA_ID,
        value: levels,
    });
    world
}

fn beacon_screen_with_local_state(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    local_state: InventoryHudLocalState,
) -> HudInventoryScreen {
    hud_inventory_screen_with_local_state(
        world,
        item_runtime,
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap()
}

fn beacon_button_texture_at(
    screen: &HudInventoryScreen,
    x: i32,
    y: i32,
) -> Option<HudInventoryBackgroundTexture> {
    screen
        .background_layers
        .iter()
        .find(|layer| {
            layer.x == x
                && layer.y == y
                && layer.width == BEACON_EFFECT_BUTTON_SIZE
                && layer.height == BEACON_EFFECT_BUTTON_SIZE
        })
        .map(|layer| layer.texture)
}

fn advancements_centered_text_label(
    text: &str,
    center_x: i32,
    y: i32,
    tint: [f32; 4],
) -> HudInventoryTextLabel {
    let width = hud_ascii_approx_text_width(text).unwrap();
    HudInventoryTextLabel {
        component_runs: vec![HudComponentTextRun::plain(text)],
        x: center_x - i32::try_from(width).unwrap() / 2,
        y,
        width,
        text: text.to_string(),
        tint,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
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
                component_patch: DataComponentPatchSummary::default().into(),
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
) -> bbb_client_world::AdvancementWidgetSummary {
    bbb_client_world::AdvancementWidgetSummary {
        id: id.to_string(),
        parent_id: parent_id.map(str::to_string),
        title: bbb_protocol::RichTextComponent::plain(id),
        description: bbb_protocol::RichTextComponent::plain(""),
        icon: AdvancementIconSummary {
            item_id: 1,
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
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

fn smithing_preview_for_result_stack(
    stack: bbb_protocol::packets::ItemStackSummary,
    runtime: &NativeItemRuntime,
) -> HudEntityPreview {
    let mut world = WorldStore::new();
    crate::resource_reload::apply_item_runtime_world_defaults(runtime, &mut world).unwrap();
    world.rebuild_item_component_defaults().unwrap();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
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
    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
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
            "default_component_codec_values": {
                "minecraft:diamond_helmet": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "head", "asset_id": "minecraft:diamond"}
                },
                "minecraft:diamond_chestplate": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "chest", "asset_id": "minecraft:diamond"}
                },
                "minecraft:diamond_leggings": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "legs", "asset_id": "minecraft:diamond"}
                },
                "minecraft:diamond_boots": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "feet", "asset_id": "minecraft:diamond"}
                },
                "minecraft:elytra": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "chest", "asset_id": "minecraft:elytra"}
                },
                "minecraft:skeleton_skull": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "head"}
                },
                "minecraft:carved_pumpkin": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "head"}
                }
            }
        }))
        .unwrap();
    let equipment_assets: bbb_client_assets::EquipmentAssetCatalog =
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

fn offhand_test_login() -> PlayLogin {
    PlayLogin {
        player_id: 7,
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
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    }
}
