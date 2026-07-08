use super::*;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_pack::{
    BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, EquipmentAssetCatalog,
    FloatAttributeModifier, FloatAttributeModifierKind, GrassColorModifier, ItemRegistryCatalog,
};
use bbb_protocol::packets::ClockUpdate as ProtocolClockUpdate;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, BlockUpdate as ProtocolBlockUpdate, BossBarColor, BossBarOverlay,
    BossEvent as ProtocolBossEvent, BossEventFlags as ProtocolBossEventFlags,
    BossEventOperation as ProtocolBossEventOperation, CommonPlayerSpawnInfo,
    DataComponentPatchSummary, DialogHolder, GameEvent as ProtocolGameEvent,
    InitializeBorder as ProtocolInitializeBorder, InteractionHand, MerchantOffer, MerchantOffers,
    MobEffectFlags, OpenBook, OpenSignEditor, PlayLogin, PlayTime, RemoveMobEffect,
    SetBorderLerpSize as ProtocolSetBorderLerpSize, SetCursorItem as ProtocolSetCursorItem,
    SetPlayerInventory as ProtocolSetPlayerInventory, ShowDialog, UpdateMobEffect,
    WrittenBookContentSummary,
};
use bbb_world::{
    BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, HeightmapData, LightData,
    LocalPlayerPoseState, PaletteDomain, PaletteKind, PalettedContainerData,
    WorldBlockDestroyProfile, WorldDimension,
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
const VANILLA_26_1_FISHING_BOBBER_ENTITY_TYPE_ID: i32 = 156;
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
    world.apply_add_entity(test_add_entity(
        7,
        VANILLA_26_1_FISHING_BOBBER_ENTITY_TYPE_ID,
    ));
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 42,
        uuid: uuid::Uuid::from_u128(42),
        entity_type_id: VANILLA_26_1_PLAYER_ENTITY_TYPE_ID,
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
fn particle_entity_target_contexts_track_world_entity_positions() {
    let mut world = WorldStore::new();
    assert!(particle_entity_target_contexts(&world).is_empty());

    world.apply_add_entity(test_add_entity(42, VANILLA_26_1_PLAYER_ENTITY_TYPE_ID));
    world.apply_add_entity(test_add_entity(77, VANILLA_26_1_PLAYER_ENTITY_TYPE_ID));

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
    assert_eq!(command.particle_type_id, SMOKE_PARTICLE_TYPE_ID);
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
    assert_eq!(stun.particle_type_id, ENTITY_EFFECT_PARTICLE_TYPE_ID);
    assert_eq!(stun.particle_id, "minecraft:entity_effect");
    assert_eq!(stun.position, [1.25, 65.9, -0.5]);
    assert_eq!(stun.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(
        stun.option_color,
        Some([0.49803922, 0.5137255, 0.57254905, 1.0])
    );

    let crit = &batch.commands[1];
    assert_eq!(crit.particle_type_id, CRIT_PARTICLE_TYPE_ID);
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
    assert_eq!(overworld_environment.level_lighting, LevelLighting::Default);

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
    assert_eq!(nether_environment.level_lighting, LevelLighting::Nether);

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
        .find("let fog_environment = fog_environment_for_world_at_camera(")
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
        .find("advance_player_input(input, world, net_counters, net_commands, now);")
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
    let hotbar_icons_extract = source
        .find("let hud_hotbar_item_icons = hotbar_item_icons_with_input_context(")
        .expect("pump should extract hotbar item icons");
    let sign_editor_extract = source
        .find("let hud_sign_editor_screen = hud_sign_editor_screen(input, world);")
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
        hotbar_icons_extract,
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
        .find(
            "let hud_action_bar_text = hud_action_bar_text_from_world(world, entity_partial_tick);",
        )
        .expect("pump should project the action bar state");
    let title_extract = source
        .find("let hud_title_text = hud_title_text_from_world(world, entity_partial_tick);")
        .expect("pump should project the title state");
    assert!(
        gui_tick < action_bar_extract && action_bar_extract < title_extract,
        "vanilla `Minecraft.tick` runs `Gui.tick` before `GameRenderer.extractGui`"
    );
}

#[test]
fn hud_action_bar_and_title_projection_matches_world_state() {
    let mut world = WorldStore::new();
    assert_eq!(hud_action_bar_text_from_world(&world, 0.25), None);
    assert_eq!(hud_title_text_from_world(&world, 0.25), None);

    world.apply_action_bar_text(bbb_protocol::packets::SetActionBarText {
        content: "Action ready".to_string(),
    });
    world.apply_titles_animation(bbb_protocol::packets::SetTitlesAnimation {
        fade_in: 5,
        stay: 40,
        fade_out: 15,
    });
    world.apply_title_text(bbb_protocol::packets::SetTitleText {
        content: "Quest complete".to_string(),
    });
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText {
        content: "Return to camp".to_string(),
    });

    let action_bar = hud_action_bar_text_from_world(&world, 0.25).expect("action bar projected");
    assert_eq!(
        action_bar.runs,
        vec![bbb_renderer::HudStyledTextRun::plain("Action ready")]
    );
    assert_eq!(action_bar.remaining_ticks, 60);
    assert_eq!(action_bar.partial_tick, 0.25);
    // Both packet paths are vanilla `setOverlayMessage(component, false)`;
    // only the jukebox now-playing path animates.
    assert!(!action_bar.animate_color);

    let title = hud_title_text_from_world(&world, 0.25).expect("title projected");
    assert_eq!(
        title.title_runs,
        vec![bbb_renderer::HudStyledTextRun::plain("Quest complete")]
    );
    assert_eq!(
        title.subtitle_runs,
        vec![bbb_renderer::HudStyledTextRun::plain("Return to camp")]
    );
    assert_eq!(title.remaining_ticks, 60);
    assert_eq!((title.fade_in, title.stay, title.fade_out), (5, 40, 15));
    assert_eq!(title.partial_tick, 0.25);

    // Post-tick countdowns flow through; expired timers stop projecting.
    world.advance_hud_text_ticks(59);
    assert_eq!(
        hud_action_bar_text_from_world(&world, 0.0)
            .expect("last action bar tick")
            .remaining_ticks,
        1
    );
    assert_eq!(
        hud_title_text_from_world(&world, 0.0)
            .expect("last title tick")
            .remaining_ticks,
        1
    );
    world.advance_hud_text_ticks(1);
    assert_eq!(hud_action_bar_text_from_world(&world, 0.0), None);
    assert_eq!(hud_title_text_from_world(&world, 0.0), None);

    // A re-set title without a subtitle projects an empty subtitle line.
    world.apply_title_text(bbb_protocol::packets::SetTitleText {
        content: "Solo".to_string(),
    });
    let solo = hud_title_text_from_world(&world, 0.5).expect("title without subtitle");
    assert!(solo.subtitle_runs.is_empty());
    assert_eq!(solo.remaining_ticks, 60);

    // A subtitle without an active title never projects (vanilla draws the
    // subtitle only inside the title branch, Gui.java:364).
    world.apply_clear_titles(bbb_protocol::packets::ClearTitles { reset_times: false });
    world.apply_subtitle_text(bbb_protocol::packets::SetSubtitleText {
        content: "Orphan".to_string(),
    });
    assert_eq!(hud_title_text_from_world(&world, 0.0), None);
}

#[test]
fn hud_boss_bar_projection_orders_by_uuid_and_maps_style_names() {
    let mut world = WorldStore::new();
    assert!(hud_boss_bars_from_world(&world).is_empty());

    // Insert the higher UUID first: the world keys bars in a BTreeMap, so
    // the projection orders by UUID (deterministic across frames; vanilla's
    // LinkedHashMap packet-arrival order is not tracked).
    let dragon = uuid::Uuid::from_u128(7);
    let wither = uuid::Uuid::from_u128(2);
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: dragon,
        operation: bbb_protocol::packets::BossEventOperation::Add {
            name: "Ender Dragon".to_string(),
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
            name: "Wither".to_string(),
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
    assert_eq!(
        bars[0].name_runs,
        vec![bbb_renderer::HudStyledTextRun::plain("Wither")]
    );
    assert_eq!(bars[0].progress, 0.5);
    assert_eq!(bars[0].color, bbb_renderer::HudBossBarColor::Red);
    assert_eq!(bars[0].overlay, bbb_renderer::HudBossBarOverlay::Notched10);
    assert_eq!(
        bars[1].name_runs,
        vec![bbb_renderer::HudStyledTextRun::plain("Ender Dragon")]
    );
    assert_eq!(bars[1].color, bbb_renderer::HudBossBarColor::Purple);
    assert_eq!(bars[1].overlay, bbb_renderer::HudBossBarOverlay::Progress);

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
    assert_eq!(bars[0].color, bbb_renderer::HudBossBarColor::Yellow);
    assert_eq!(bars[0].overlay, bbb_renderer::HudBossBarOverlay::Notched20);
    world.apply_boss_event(bbb_protocol::packets::BossEvent {
        id: wither,
        operation: bbb_protocol::packets::BossEventOperation::Remove,
    });
    let bars = hud_boss_bars_from_world(&world);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].progress, 0.75);
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
                    name: format!("boss {id}"),
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
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick(now);")
        .expect("pump should compute the render partial tick");
    let client_time = source
        .find("world.advance_client_time(running_ticks);")
        .expect("pump should advance client time before item model extraction");
    let cooldown_tick = source
        .find("world.advance_item_cooldowns(advanced_ticks);")
        .expect("pump should advance item cooldowns before item model extraction");
    let input_advance = source
        .find("advance_player_input(input, world, net_counters, net_commands, now);")
        .expect("pump should advance input before held item extraction");
    let using_item_tick = source
        .find("world.advance_local_using_item_ticks(advanced_ticks);")
        .expect("pump should advance local use-item ticks before held item extraction");
    let item_age_extract = source
        .find("let item_model_age_ticks = world")
        .expect("pump should compute dropped item model age");
    let dropped_models = source
        .find("let dropped_item_models = dropped_item_models(")
        .expect("pump should extract dropped item models");
    let billboards = source
        .find("let item_entity_billboards = item_entity_billboards_from_world(")
        .expect("pump should extract item entity billboards");
    let entity_instances = source
        .find("let mut entity_instances =\n        entity_model_instances_from_world_at_partial_tick(")
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
        .find("let camera_pose = camera_pose_from_world(world);")
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
    let item_frame_models = source
        .find("let item_frame_models = item_frame_models(")
        .expect("pump should extract item frame models");
    let entity_block_meshes = source
        .find("let entity_block_meshes =")
        .expect("pump should extract entity block item models");

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
fn renderer_frame_block_destroy_overlays_extract_after_destroy_tick() {
    let source = include_str!("../runtime.rs");
    let destroy_tick = source
        .find("advance_block_destruction_render_ticks(world, running_ticks);")
        .expect("pump should advance block-destroy render ticks");
    let block_destroy_extract = source
        .find("let block_destroy_overlays = block_destroy_overlays_from_world(")
        .expect("pump should extract block-destroy overlays");

    assert!(
        destroy_tick < block_destroy_extract,
        "vanilla `LevelRenderer.extractBlockDestroyAnimation` reads post-client-tick block-breaking state"
    );
}

#[test]
fn renderer_frame_outlines_extract_after_input_camera_and_partial_tick() {
    let source = include_str!("../runtime.rs");
    let input_advance = source
        .find("advance_player_input(input, world, net_counters, net_commands, now);")
        .expect("pump should advance player input before outline extraction");
    let using_item_tick = source
        .find("world.advance_local_using_item_ticks(advanced_ticks);")
        .expect("pump should advance local use-item ticks before outline extraction");
    let entity_tick = source
        .find("let advanced_ticks = advance_entity_client_animations(")
        .expect("pump should advance entity client animations before outline extraction");
    let partial_tick = source
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick(now);")
        .expect("pump should compute partial tick before outline extraction");
    let camera_pose = source
        .find("let camera_pose = camera_pose_from_world(world);")
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
fn renderer_frame_cloud_frame_extracts_after_client_time_camera_and_partial_tick() {
    let source = include_str!("../runtime.rs");
    let entity_tick = source
        .find("let advanced_ticks = advance_entity_client_animations(")
        .expect("pump should advance entity client animations before cloud frame extraction");
    let partial_tick = source
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick(now);")
        .expect("pump should compute the render partial tick before cloud frame extraction");
    let client_time = source
        .find("world.advance_client_time(running_ticks);")
        .expect("pump should advance client time before cloud frame extraction");
    let camera_pose = source
        .find("let camera_pose = camera_pose_from_world(world);")
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
        .find("let entity_partial_tick = client_animation_ticks.entity_partial_tick(now);")
        .expect("pump should compute the render partial tick before weather extraction");
    let client_time = source
        .find("world.advance_client_time(running_ticks);")
        .expect("pump should advance client time before weather extraction");
    let camera_pose = source
        .find("let camera_pose = camera_pose_from_world(world);")
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
        .find("advance_player_input(input, world, net_counters, net_commands, now);")
        .expect("pump should advance player input before particle tick");
    let destroy_advance = source
        .find("advance_destroying_block_at_partial_tick(")
        .expect("pump should advance destroy input before particle tick");
    let use_advance = source
        .find("advance_using_item_at_partial_tick(")
        .expect("pump should advance use-item input before particle tick");
    let using_item_tick = source
        .find("world.advance_local_using_item_ticks(advanced_ticks);")
        .expect("pump should advance local use-item ticks before particle tick");
    let particle_camera_pose = source
        .find("let particle_camera_pose = camera_pose_from_world(world);")
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
    let primed_tnt_smoke = source
        .find("submit_primed_tnt_smoke_particles(renderer, world, advanced_ticks);")
        .expect("pump should emit PrimedTnt client smoke before particle tick");
    let entity_client_tick_particles = source
        .find("submit_entity_client_tick_particles(renderer, world, &mut particle_events);")
        .expect("pump should emit entity client tick particles before particle tick");
    let ominous_item_spawner_particles = source
        .find("submit_ominous_item_spawner_particles(renderer, world, &mut particle_events);")
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
        .find("let camera_pose = camera_pose_from_world(world);")
        .expect("pump should bind frame camera pose before particle light refresh");
    let block_destroy_extract = source
        .find("let block_destroy_overlays = block_destroy_overlays_from_world(")
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
        primed_tnt_smoke < particle_tick,
        "vanilla `PrimedTnt.tick` emits smoke before ParticleEngine.tick advances particles"
    );
    assert!(
        primed_tnt_smoke < entity_client_tick_particles && entity_client_tick_particles < particle_tick,
        "entity client-tick particles should be submitted before ParticleEngine.tick advances particles"
    );
    assert!(
        entity_client_tick_particles < ominous_item_spawner_particles
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
fn clear_color_samples_camera_biome_fog_and_sky_color_attributes() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_environment(
            42,
            Some([0x10, 0x20, 0x30]),
            Some([0x80, 0x40, 0x20]),
            None,
        ),
    ]));

    let clear =
        clear_color_for_world_at_camera(&world, &textures, camera_pose_from_world(&world), false);
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
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_environment(42, Some([0x10, 0x20, 0x30]), None, None),
    ]));

    let sky = sky_environment_for_world_at_camera(
        &world,
        &textures,
        camera_pose_from_world(&world),
        false,
    );

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
    let textures = TerrainTextureState::default();

    let flashed = sky_environment_for_world_at_camera(
        &world,
        &textures,
        camera_pose_from_world(&world),
        false,
    );
    let hidden = sky_environment_for_world_at_camera(
        &world,
        &textures,
        camera_pose_from_world(&world),
        true,
    );
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
        sky_environment_for_world_at_camera(
            &nether,
            &textures,
            camera_pose_from_world(&nether),
            false
        ),
        SkyEnvironment::disabled()
    );
}

#[test]
fn sky_environment_projects_end_skybox_state() {
    let end = world_with_dimension(2, "minecraft:the_end");

    let sky = sky_environment_for_world_at_camera(
        &end,
        &TerrainTextureState::default(),
        camera_pose_from_world(&end),
        false,
    );

    assert!(sky.end_sky_visible());
    assert!(!sky.is_visible());
    assert_eq!(sky.sunrise_sunset_color[3], 0.0);
}

#[test]
fn sky_environment_projects_sunrise_sunset_render_state() {
    let day_time = 71;
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, day_time);
    set_world_weather(&mut world, 0.25, 0.5);

    let sky = sky_environment_for_world_at_camera(
        &world,
        &TerrainTextureState::default(),
        camera_pose_from_world(&world),
        false,
    );
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
fn sky_environment_projects_vanilla_star_brightness_with_weather_layers() {
    let day_time = 13_228;
    let mut clear = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut clear, day_time);
    let clear_sky = sky_environment_for_world_at_camera(
        &clear,
        &TerrainTextureState::default(),
        camera_pose_from_world(&clear),
        false,
    );

    assert!((clear_sky.star_brightness - 0.5).abs() < 1e-6);

    let mut storm = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut storm, day_time);
    set_world_weather(&mut storm, 0.75, 0.25);
    let storm_sky = sky_environment_for_world_at_camera(
        &storm,
        &TerrainTextureState::default(),
        camera_pose_from_world(&storm),
        false,
    );

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
    let border_tick = source
        .find("world.advance_world_border(running_ticks);")
        .expect("pump should tick the world border");
    let client_time = source
        .find("world.advance_client_time(running_ticks);")
        .expect("pump should advance the client clock");
    let weather_extract = source
        .find("let weather_render_state =")
        .expect("pump should extract the weather render state");
    let border_extract = source
        .find("let world_border_render_state = world_border_render_state_for_world(")
        .expect("pump should extract the world border render state");

    // Vanilla ClientLevel.tick runs getWorldBorder().tick() right before
    // tickTime() (ClientLevel.java:276-281).
    assert!(
        border_tick < client_time,
        "world border ticks before the client clock, like vanilla ClientLevel.tick"
    );
    // Vanilla LevelRenderer extraction runs worldBorderRenderer.extract in the
    // "border" profiler section after the weather extraction
    // (LevelRenderer.java:573-585).
    assert!(
        client_time < border_extract && weather_extract < border_extract,
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
        &TerrainTextureState::default(),
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
        &TerrainTextureState::default(),
        camera_pose_from_world(&world),
        12,
        false,
    );
    let away_baseline = clear_color_for_day_time(day_time, 0.0, 0.0);

    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 270.0, 0.0));
    let low_render_distance = clear_color_for_world_at_camera_with_render_distance(
        &world,
        &TerrainTextureState::default(),
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
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_environment(42, None, None, Some([0x02, 0x20, 0x44])),
    ]));

    let clear =
        clear_color_for_world_at_camera(&world, &textures, camera_pose_from_world(&world), false);

    assert_eq!(
        clear,
        clear_color_from_argb(rgb_u8_to_argb([0x02, 0x20, 0x44]))
    );

    world.set_sky_flash_time(2);
    let flashed =
        clear_color_for_world_at_camera(&world, &textures, camera_pose_from_world(&world), false);
    assert_eq!(flashed, clear);
}

#[test]
fn clear_color_brightens_water_fog_with_vanilla_water_vision() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 6_000);
    world.set_local_player_pose(local_player_pose([0.5, 0.0, 0.5], 0.0, 0.0));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biome(world.dimension(), 42));
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_environment(42, None, None, Some([0x02, 0x20, 0x44])),
    ]));

    let clear = clear_color_for_world_at_camera_with_water_vision(
        &world,
        &textures,
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
        &TerrainTextureState::default(),
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
        &TerrainTextureState::default(),
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
        &TerrainTextureState::default(),
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
        &TerrainTextureState::default(),
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
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_environment(42, None, None, Some([0x02, 0x20, 0x44])),
    ]));

    let fog = fog_environment_for_world_at_camera(
        &world,
        &textures,
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
    set_lightmap_test_block(
        &mut world,
        BlockPos { x: 0, y: 1, z: 0 },
        SOURCE_WATER_BLOCK_STATE_ID,
    );
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_water_fog_end_distance(
            42,
            FloatAttributeModifier {
                modifier: FloatAttributeModifierKind::Multiply,
                argument: 0.85,
            },
        ),
    ]));

    let fog = fog_environment_for_world_at_camera(
        &world,
        &textures,
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
fn camera_biome_sky_color_uses_vanilla_gaussian_spatial_weights() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 64);
    world.set_local_player_pose(local_player_pose(
        [8.0, 32.0 - f64::from(CameraPose::STANDING_EYE_HEIGHT), 8.0],
        0.0,
        0.0,
    ));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biomes(
        world.dimension(),
        split_x_biome_container(10, 20),
    ));
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile(10, [0, 0, 0]),
        biome_profile(20, [100, 200, 250]),
    ]));

    let sky_color =
        camera_biome_sky_color(&world, &textures, camera_pose_from_world(&world)).unwrap();

    assert_eq!(sky_color, [50, 100, 125]);
}

#[test]
fn camera_biome_fog_and_water_fog_use_vanilla_gaussian_spatial_weights() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 64);
    world.set_local_player_pose(local_player_pose(
        [8.0, 32.0 - f64::from(CameraPose::STANDING_EYE_HEIGHT), 8.0],
        0.0,
        0.0,
    ));
    world.insert_decoded_chunk(empty_lightmap_test_chunk_with_biomes(
        world.dimension(),
        split_x_biome_container(10, 20),
    ));
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        biome_profile_with_environment(10, None, Some([10, 20, 30]), Some([1, 2, 3])),
        biome_profile_with_environment(20, None, Some([110, 220, 230]), Some([101, 202, 203])),
    ]));

    let camera_pose = camera_pose_from_world(&world);
    let fog_color = camera_biome_fog_color(&world, &textures, camera_pose).unwrap();
    let water_fog_color = camera_biome_water_fog_color(&world, &textures, camera_pose).unwrap();

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
            name: "Boss".to_string(),
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

fn biome_profile(id: i32, sky_color: [u8; 3]) -> BiomeColorProfile {
    biome_profile_with_environment(id, Some(sky_color), None, None)
}

fn biome_profile_with_environment(
    id: i32,
    sky_color: Option<[u8; 3]>,
    fog_color: Option<[u8; 3]>,
    water_fog_color: Option<[u8; 3]>,
) -> BiomeColorProfile {
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
        fog_color,
        sky_color,
        water_fog_color,
        water_fog_end_distance: None,
        grass_color_modifier: GrassColorModifier::None,
    }
}

fn biome_profile_with_water_fog_end_distance(
    id: i32,
    water_fog_end_distance: FloatAttributeModifier,
) -> BiomeColorProfile {
    BiomeColorProfile {
        water_fog_end_distance: Some(water_fog_end_distance),
        ..biome_profile_with_environment(id, None, None, None)
    }
}

fn biome_profile_with_precipitation(id: i32, has_precipitation: bool) -> BiomeColorProfile {
    BiomeColorProfile {
        has_precipitation,
        ..biome_profile_with_environment(id, None, None, None)
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
        ..biome_profile_with_environment(id, None, None, None)
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
        ..biome_profile_with_environment(id, None, None, None)
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
    let bits = test_heightmap_bits_for_dimension(dimension);
    let mut values = vec![0u64; 16 * 16];
    for &(local_x, local_z, first_available) in entries {
        let index = usize::from(local_x) + usize::from(local_z) * 16;
        values[index] = u64::try_from(first_available - dimension.min_y).unwrap();
    }
    HeightmapData {
        kind_id: 4,
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
        assert!((*actual - *expected).abs() < 1e-6);
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
        menu_type_id: 2,
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

    let screen = hud_sign_editor_screen(&input, &world).expect("sign editor screen");
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

    let screen = hud_sign_editor_screen(&input, &world).expect("hanging sign editor screen");
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
    world.apply_add_entity(test_add_entity(42, VANILLA_26_1_PLAYER_ENTITY_TYPE_ID));
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
        menu_type_id: 2,
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
fn hotbar_item_icons_apply_quick_charge_to_crossbow_pull_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-crossbow-quick-charge-range-dispatch");
    write_runtime_crossbow_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut stack = item_stack(0, 1);
    stack.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 1,
        level: 2,
    }];
    let enchantment_keys = vec![
        "minecraft:power".to_string(),
        "minecraft:quick_charge".to_string(),
    ];
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
            item_runtime.item_model_use_context_for_stack_with_enchantment_keys(
                &stack,
                10,
                Some(&enchantment_keys),
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
    record_enchantment_registry(&mut world);
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
    world.apply_add_entity(test_add_entity(42, VANILLA_26_1_PLAYER_ENTITY_TYPE_ID));
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
        ..test_add_entity(700, VANILLA_26_1_FISHING_BOBBER_ENTITY_TYPE_ID)
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
    );

    assert_eq!(fallback.flat_meshes.len(), 1);
    assert_eq!(trimmed.flat_meshes.len(), 1);
    assert_eq!(fallback.handled_entity_ids, trimmed.handled_entity_ids);
    assert_ne!(fallback.flat_meshes[0], trimmed.flat_meshes[0]);

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
    );
    let trim_material_keys = world_trim_material_keys(&world).unwrap();
    let trimmed = crate::item_frames::item_frame_models(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        Some(&trim_material_keys),
        None,
        None,
    );

    assert_eq!(fallback.flat_meshes.len(), 1);
    assert_eq!(trimmed.flat_meshes.len(), 1);
    assert_ne!(fallback.flat_meshes[0], trimmed.flat_meshes[0]);

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
fn hud_inventory_screen_projects_generic_container_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 5,
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
        menu_type_id: 6,
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
        menu_type_id: 7,
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
        menu_type_id: 7,
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
        menu_type_id: 12,
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
        menu_type_id: 12,
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
        menu_type_id: 12,
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
                        display_type_id: 0,
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
                        display_type_id: 0,
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
    assert!(stale.fill_layers.is_empty());
    assert!(stale.ghost_items.is_empty());
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
                        display_type_id: 0,
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
                        display_type_id: 0,
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
        menu_type_id: 12,
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
        menu_type_id: 12,
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
}

#[test]
fn hud_inventory_screen_projects_crafting_recipe_book_buttons() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 12,
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
                                display_type_id: 5,
                                raw_payload: Vec::new(),
                                item_stack: Some(item_stack(99, 1)),
                                tag: None,
                            },
                            crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                                display_type_id: 0,
                                raw_payload: Vec::new(),
                                item_stack: None,
                                tag: None,
                            },
                        },
                    ),
                    furnace: None,
                },
                group: None,
                category_id: 3,
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
            shapeless_crafting_recipe_book_entry(20, 2, Some(7), 1),
            shapeless_crafting_recipe_book_entry(21, 2, Some(7), 2),
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
    let item_positions: Vec<(i32, i32)> = screen
        .floating_items
        .iter()
        .map(|item| (item.x, item.y))
        .collect();
    assert!(item_positions.contains(&(19, 40)));
    assert!(item_positions.contains(&(44, 40)));
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
        menu_type_id: 12,
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
        menu_type_id: 14,
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
        menu_type_id: 13,
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
        menu_type_id: 13,
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
        menu_type_id: 13,
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
        menu_type_id: 8,
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
        menu_type_id: 9,
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
        menu_type_id: 9,
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
        menu_type_id: 8,
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
        menu_type_id: 8,
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
        menu_type_id: 8,
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
        menu_type_id: 11,
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
        menu_type_id: 11,
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
        menu_type_id: 14,
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
        menu_type_id: 15,
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
        menu_type_id: 15,
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
        menu_type_id: 16,
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
        menu_type_id: 17,
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
        menu_type_id: 17,
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
                shadow: false,
                runs: Vec::new(),
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
                input: None,
                shadow: false,
                runs: Vec::new(),
            },
            HudInventoryTextLabel {
                x: BOOK_PAGE_TEXT_X,
                y: BOOK_PAGE_TEXT_Y,
                width: BOOK_PAGE_TEXT_WIDTH,
                text: "First page".to_string(),
                tint: BOOK_TEXT_COLOR,
                background: None,
                input: None,
                shadow: false,
                runs: Vec::new(),
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
        menu_type_id: 18,
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
        menu_type_id: 18,
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
        menu_type_id: 19,
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
        menu_type_id: 19,
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
        menu_type_id: 21,
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
        menu_type_id: 21,
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
        menu_type_id: 21,
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
        menu_type_id: 23,
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
        menu_type_id: 24,
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
        menu_type_id: 24,
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
            tag: Some("minecraft:test".to_string()),
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

    let overlays = block_destroy_overlays_from_world(&world, &textures);

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

    let overlays = block_destroy_overlays_from_world(&world, &textures);

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

    assert!(publish_snapshot(
        &snapshot,
        RendererCounters::default(),
        &net,
        &audio,
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

fn test_humanoid_arm_data(data_id: u8, arm_id: i32) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: 42,
        value: bbb_protocol::packets::EntityDataValueKind::HumanoidArm(arm_id),
    }
}

fn test_item_stack_data(
    data_id: u8,
    item: bbb_protocol::packets::ItemStackSummary,
) -> bbb_protocol::packets::EntityDataValue {
    bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: 7,
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

fn record_enchantment_registry(world: &mut WorldStore) {
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:power".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:quick_charge".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
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
        display_type_id: 5,
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
        display_type_id: 6,
        raw_payload: raw_payload.into_inner(),
        item_stack: None,
        tag: Some(tag.to_string()),
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

fn open_recipe_book_crafting_table_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: 12,
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
        menu_type_id: 14,
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
            public static final Item CRAFTING_TABLE = registerItem("crafting_table");
            public static final Item STICK = registerItem("stick");
            public static final Item OAK_PLANKS = registerItem("oak_planks");
        }"#,
    );
    let runtime = NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    runtime
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
                            display_type_id: 5,
                            raw_payload: Vec::new(),
                            item_stack: Some(item_stack(result_item_id, 1)),
                            tag: None,
                        },
                        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: 0,
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
            display_type_id: 0,
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
            public static final Item CROSSBOW = registerItem("crossbow");
        }"#,
    );
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
            public static final Item HAND_SELECTOR = registerItem("hand_selector");
        }"#,
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
            public static final Item DIMENSION_SELECTOR = registerItem("dimension_selector");
        }"#,
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
            public static final Item ENTITY_SELECTOR = registerItem("entity_selector");
        }"#,
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
            public static final Item VIEW_ENTITY_CONDITION = registerItem("view_entity_condition");
        }"#,
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
            public static final Item EXTENDED_VIEW_CONDITION = registerItem("extended_view_condition");
        }"#,
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
            public static final Item KEYBIND_DOWN_CONDITION = registerItem("keybind_down_condition");
        }"#,
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
            public static final Item FISHING_ROD = registerItem("fishing_rod");
        }"#,
    );
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
            public static final Item COOLDOWN_SELECTOR = registerItem("cooldown_selector");
        }"#,
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
            public static final Item TIME_SELECTOR = registerItem("time_selector");
            public static final Item TIME_WOBBLED_SELECTOR = registerItem("time_wobbled_selector");
            public static final Item TIME_RANDOM_SELECTOR = registerItem("time_random_selector");
            public static final Item TIME_MOON_PHASE_SELECTOR = registerItem("time_moon_phase_selector");
        }"#,
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
            public static final Item SPAWN_COMPASS = registerItem("spawn_compass");
            public static final Item SPAWN_COMPASS_WOBBLED = registerItem("spawn_compass_wobbled");
            public static final Item SPAWN_COMPASS_INVALID_SPIN = registerItem("spawn_compass_invalid_spin");
            public static final Item SPAWN_COMPASS_NONE_SPIN = registerItem("spawn_compass_none_spin");
        }"#,
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
            public static final Item LODESTONE_COMPASS = registerItem("lodestone_compass");
        }"#,
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
            public static final Item RECOVERY_COMPASS = registerItem("recovery_compass");
        }"#,
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
            public static final Item SELECTED_CONDITION = registerItem("selected_condition");
        }"#,
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
            public static final Item CARRIED_CONDITION = registerItem("carried_condition");
        }"#,
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
            public static final Item CLOCK = registerItem("clock");
            public static final Item COMPASS = registerItem("compass");
            public static final Item SPYGLASS = registerItem("spyglass");
        }"#,
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
            public static final Item TRIM_SELECTOR = registerItem("trim_selector");
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
