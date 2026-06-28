use super::*;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_protocol::packets::ClockUpdate as ProtocolClockUpdate;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, BlockUpdate as ProtocolBlockUpdate, CommonPlayerSpawnInfo,
    DialogHolder, GameEvent as ProtocolGameEvent, InteractionHand, MerchantOffer, MerchantOffers,
    MobEffectFlags, OpenBook, OpenSignEditor, PlayLogin, PlayTime, RemoveMobEffect,
    SetPlayerInventory as ProtocolSetPlayerInventory, ShowDialog, UpdateMobEffect,
    WrittenBookContentSummary,
};
use bbb_world::{
    BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState,
    PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
};
use tokio::sync::mpsc;
use winit::{
    event::{ElementState, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

const TOOLTIP_TEST_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TOOLTIP_TEST_AQUA: [f32; 4] = [85.0 / 255.0, 1.0, 1.0, 1.0];
const TOOLTIP_TEST_DARK_PURPLE: [f32; 4] = [170.0 / 255.0, 0.0, 170.0 / 255.0, 1.0];
const VANILLA_26_1_PLAYER_ENTITY_TYPE_ID: i32 = 155;
const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

fn tooltip_line(text: &str, tint: [f32; 4]) -> HudInventoryTooltipLine {
    HudInventoryTooltipLine {
        text: text.to_string(),
        tint,
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
fn entity_animation_partial_tick_tracks_time_since_last_client_tick() {
    let now = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();

    assert_eq!(ticks.entity_partial_tick(now), 1.0);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, now),
        0
    );
    assert_eq!(ticks.entity_partial_tick(now), 0.0);
    assert_eq!(
        ticks.entity_partial_tick(now + Duration::from_millis(25)),
        0.5
    );
    assert_eq!(
        ticks.entity_partial_tick(now + Duration::from_millis(75)),
        1.0
    );
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
    assert_close3(
        overworld_environment.sky_light_color,
        VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    );
    assert_close3(overworld_environment.ambient_color, [10.0 / 255.0; 3]);
    assert_eq!(overworld_environment.brightness_factor, 0.75);
    assert_eq!(overworld_environment.block_factor, 1.25);

    let nether = world_with_dimension(1, "minecraft:the_nether");
    let nether_environment = lightmap_environment_for_world(&nether, 0.5, 1.4);
    assert_eq!(nether_environment.sky_factor, 0.0);
    assert_close3(
        nether_environment.sky_light_color,
        [122.0 / 255.0, 122.0 / 255.0, 1.0],
    );
    assert_close3(
        nether_environment.ambient_color,
        [48.0 / 255.0, 40.0 / 255.0, 33.0 / 255.0],
    );

    let end = world_with_dimension(2, "minecraft:the_end");
    let end_environment = lightmap_environment_for_world(&end, 0.5, 1.4);
    assert_eq!(end_environment.sky_factor, 0.0);
    assert_close3(
        end_environment.sky_light_color,
        [172.0 / 255.0, 96.0 / 255.0, 205.0 / 255.0],
    );
    assert_close3(
        end_environment.ambient_color,
        [63.0 / 255.0, 71.0 / 255.0, 63.0 / 255.0],
    );
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

    lightmap.advance_for_world(1, &world);
    let environment = lightmap.environment_for_world(&world);
    let first_tick_factor = 1.0 / VANILLA_DARKNESS_BLEND_OUT_ADVANCE_TICKS as f32;
    assert!((environment.brightness_factor - (0.8 - first_tick_factor)).abs() < 1e-6);
    assert!((environment.darkness_scale - 0.45 * first_tick_factor).abs() < 1e-6);

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
        menu_type_id: 2,
        title: "Chest".to_string(),
    });
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

fn world_with_dimension(dimension_type_id: i32, dimension: &str) -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
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

fn attach_lightmap_local_player(world: &mut WorldStore, id: i32) {
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id,
        uuid: uuid::Uuid::from_u128(id as u128),
        entity_type_id: VANILLA_26_1_PLAYER_ENTITY_TYPE_ID,
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
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })
            .collect(),
        block_entities: Vec::new(),
        light: LightData::default(),
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

fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-6);
    }
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
        menu_type_id: 2,
        title: "Chest".to_string(),
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
        entity_type_id: 7,
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
        entity_type_id: 7,
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
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Inventory,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
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
            lines: vec![tooltip_line("Test Combo", TOOLTIP_TEST_WHITE)],
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
                tooltip_line("Custom Combo", TOOLTIP_TEST_AQUA),
                tooltip_line("First lore", TOOLTIP_TEST_DARK_PURPLE),
                tooltip_line("Second lore", TOOLTIP_TEST_DARK_PURPLE),
            ],
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_generic_container_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 5,
        title: "Large Chest".to_string(),
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
        menu_type_id: 6,
        title: "Dispenser".to_string(),
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
        menu_type_id: 7,
        title: "Crafter".to_string(),
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
        menu_type_id: 7,
        title: "Crafter".to_string(),
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
        menu_type_id: 12,
        title: "Crafting".to_string(),
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
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::CraftingTable,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
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
fn hud_inventory_screen_projects_enchanting_table_layout_and_lapis_slot_layer() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 13,
        title: "Enchanting Table".to_string(),
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
        menu_type_id: 13,
        title: "Enchanting Table".to_string(),
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
            shadow: false,
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_enchanting_table_disabled_cost_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 13,
        title: "Enchanting Table".to_string(),
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
            shadow: false,
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 8,
        title: "Anvil".to_string(),
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
        menu_type_id: 9,
        title: "Beacon".to_string(),
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
            HudInventoryBackgroundTexture::BeaconEffectSpeed,
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
            HudInventoryBackgroundTexture::BeaconEffectRegeneration,
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
        menu_type_id: 9,
        title: "Beacon".to_string(),
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
        menu_type_id: 9,
        title: "Beacon".to_string(),
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
                Some(BEACON_EFFECT_STRENGTH_ID),
                Some(BEACON_EFFECT_STRENGTH_ID),
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
            HudInventoryBackgroundTexture::BeaconEffectStrength,
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
            HudInventoryBackgroundTexture::BeaconEffectStrength,
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
        menu_type_id: 8,
        title: "Anvil".to_string(),
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
        menu_type_id: 8,
        title: "Anvil".to_string(),
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
            shadow: false,
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_cost_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 8,
        title: "Anvil".to_string(),
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
            shadow: false,
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_too_expensive_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 8,
        title: "Anvil".to_string(),
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
            shadow: false,
        }]
    );
}

#[test]
fn hud_inventory_screen_projects_brewing_stand_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 11,
        title: "Brewing Stand".to_string(),
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
        menu_type_id: 11,
        title: "Brewing Stand".to_string(),
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
                texture,
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
        menu_type_id: 14,
        title: "Furnace".to_string(),
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
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_grindstone_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 15,
        title: "Grindstone".to_string(),
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
        menu_type_id: 15,
        title: "Grindstone".to_string(),
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
        menu_type_id: 16,
        title: "Hopper".to_string(),
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
        menu_type_id: 17,
        title: "Lectern".to_string(),
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
        menu_type_id: 17,
        title: "Lectern".to_string(),
    });
    let mut book = item_stack(42, 1);
    book.component_patch.written_book = Some(bbb_protocol::packets::WrittenBookContentSummary {
        title: "Guide".to_string(),
        author: "Alex".to_string(),
        generation: 0,
        pages: vec![
            "First page".to_string(),
            "Second page\nLine two".to_string(),
        ],
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
                shadow: false,
            },
            HudInventoryTextLabel {
                x: BOOK_PAGE_TEXT_X,
                y: BOOK_PAGE_TEXT_Y,
                width: BOOK_PAGE_TEXT_WIDTH,
                text: "Second page".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                shadow: false,
            },
            HudInventoryTextLabel {
                x: BOOK_PAGE_TEXT_X,
                y: BOOK_PAGE_TEXT_Y + BOOK_PAGE_LINE_HEIGHT,
                width: BOOK_PAGE_TEXT_WIDTH,
                text: "Line two".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                shadow: false,
            },
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_current_book_screen() {
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: written_book_stack(vec!["First page", "Second page"]),
    });
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
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
    assert_eq!(
        screen.text_labels,
        vec![
            HudInventoryTextLabel {
                x: 88,
                y: BOOK_PAGE_INDICATOR_Y,
                width: 60,
                text: "Page 1 of 2".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                shadow: false,
            },
            HudInventoryTextLabel {
                x: BOOK_PAGE_TEXT_X,
                y: BOOK_PAGE_TEXT_Y,
                width: BOOK_PAGE_TEXT_WIDTH,
                text: "First page".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                shadow: false,
            },
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_shulker_box_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 20,
        title: "Shulker Box".to_string(),
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
        menu_type_id: 18,
        title: "Loom".to_string(),
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
        menu_type_id: 18,
        title: "Loom".to_string(),
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
        menu_type_id: 19,
        title: "Merchant".to_string(),
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
        menu_type_id: 19,
        title: "Merchant".to_string(),
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

    let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

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
    assert_eq!(
        screen.background_layers[1],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerTradeArrow,
            60,
            22,
            10,
            9,
            [0.0, 0.0],
            [1.0, 1.0],
        )
    );
    assert_eq!(
        screen.background_layers[2],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerTradeArrowOutOfStock,
            60,
            42,
            10,
            9,
            [0.0, 0.0],
            [1.0, 1.0],
        )
    );
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
fn merchant_offer_cost_a_stack_uses_vanilla_modified_cost_count() {
    let mut world = WorldStore::new();
    world.set_default_item_max_stack_sizes(std::collections::BTreeMap::from([(42, 16)]));
    let offer = bbb_world::MerchantOfferState {
        buy_a: item_cost(42, 10),
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

    assert_eq!(
        merchant_offer_cost_a_stack(&world, &offer),
        item_stack(42, 16)
    );
}

#[test]
fn hud_inventory_screen_projects_smithing_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 21,
        title: "Smithing".to_string(),
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
fn hud_inventory_screen_projects_smithing_error_layer() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 21,
        title: "Smithing".to_string(),
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
        menu_type_id: 23,
        title: "Cartography Table".to_string(),
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
        menu_type_id: 24,
        title: "Stonecutter".to_string(),
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
        menu_type_id: 24,
        title: "Stonecutter".to_string(),
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
fn stonecutter_slot_display_item_stack_projects_direct_item_displays() {
    assert_eq!(
        stonecutter_slot_display_item_stack(&stonecutter_item_display(77)),
        Some(item_stack(77, 1))
    );
    assert_eq!(
        stonecutter_slot_display_item_stack(&stonecutter_item_stack_display(78, 3)),
        Some(item_stack(78, 3))
    );
    assert_eq!(
        stonecutter_slot_display_item_stack(&bbb_protocol::packets::SlotDisplaySummary {
            display_type_id: 6,
            raw_payload: vec![6, 4, b't', b'e', b's', b't'],
            item_stack: None,
        }),
        None
    );
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
    assert_eq!(
        hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 25, false)),
        Some(HudItemDurabilityBar::new(10, [127.0 / 255.0, 1.0, 0.0]))
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 100, false)),
        Some(HudItemDurabilityBar::new(0, [1.0, 0.0, 0.0]))
    );
}

#[test]
fn hud_item_durability_bar_requires_damageable_damaged_stack() {
    assert_eq!(
        hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 0, false)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, -5, false)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 25, true)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 0, 100, 25, false)),
        None
    );
    let mut missing_damage = item_stack(42, 1);
    missing_damage.component_patch.max_damage = Some(100);
    assert_eq!(hud_item_durability_bar_for_stack(&missing_damage), None);

    let mut missing_max_damage = item_stack(42, 1);
    missing_max_damage.component_patch.damage = Some(25);
    assert_eq!(hud_item_durability_bar_for_stack(&missing_max_damage), None);

    let mut non_damageable = item_stack_with_damage(42, 1, 0, 25, false);
    assert_eq!(hud_item_durability_bar_for_stack(&non_damageable), None);
    non_damageable.component_patch.max_damage = Some(-1);
    assert_eq!(hud_item_durability_bar_for_stack(&non_damageable), None);
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

    let overlays = block_destroy_overlays_from_world(&world, &textures);

    assert_eq!(overlays.len(), 2);
    assert_eq!(overlays[0].pos, [1, 3, 4]);
    assert_eq!(overlays[0].uv, textures.destroy_stage_uv_rect(3).unwrap());
    assert_eq!(overlays[1].pos, [2, 3, 4]);
    assert_eq!(overlays[1].uv, textures.destroy_stage_uv_rect(7).unwrap());
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

    assert!(block_destroy_overlays_from_world(&world, &textures).is_empty());
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

    assert_eq!(advance_block_destruction_render_ticks(&mut world, 420), 0);
    assert_eq!(world.block_destructions().len(), 1);

    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 420 });
    assert_eq!(advance_block_destruction_render_ticks(&mut world, 420), 1);
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
fn publish_snapshot_includes_audio_runtime_counters() {
    let snapshot = bbb_control::shared_snapshot("test");
    let audio = AudioCounters {
        enabled: true,
        catalog_events: 1902,
        registry_entries: 1902,
        commands_submitted: 3,
        submit_failures: 1,
        last_submit_error: Some("failed to submit audio command".to_string()),
        ..AudioCounters::default()
    };
    let net = NetCounters::default();
    let world = WorldStore::new();

    assert!(publish_snapshot(
        &snapshot,
        RendererCounters::default(),
        &net,
        &audio,
        &world,
    ));

    assert_eq!(snapshot.read().unwrap().audio, audio);
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

fn test_bool_data(data_id: u8, value: bool) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: 8,
        value: bbb_protocol::packets::EntityDataValueKind::Boolean(value),
    }
}

fn test_byte_data(data_id: u8, value: i8) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: 0,
        value: bbb_protocol::packets::EntityDataValueKind::Byte(value),
    }
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
        display_type_id: 4,
        raw_payload,
        item_stack: Some(item_stack(item_id, 1)),
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
        display_type_id: 5,
        raw_payload,
        item_stack: Some(item_stack(item_id, count)),
    }
}

fn cartography_table_world_with_items(
    items: Vec<bbb_protocol::packets::ItemStackSummary>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 23,
        title: "Cartography Table".to_string(),
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
        .push(MAP_ID_DATA_COMPONENT_TYPE_ID);
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

fn written_book_stack(pages: Vec<&str>) -> bbb_protocol::packets::ItemStackSummary {
    let mut item = item_stack(42, 1);
    item.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".to_string(),
        author: "Alex".to_string(),
        generation: 0,
        pages: pages.into_iter().map(str::to_string).collect(),
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
            "item.minecraft.test_combo": "Test Combo"
        }"#,
    );
    write_runtime_png(
        &assets.join("textures").join("item").join("test_combo.png"),
        &[80, 120, 160, 255],
    );
    write_runtime_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
            public static final Item TEST_COMBO = registerItem("test_combo");
        }"#,
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
    write_runtime_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
            public static final Item BOW = registerItem("bow");
        }"#,
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
