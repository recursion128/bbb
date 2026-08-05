use super::*;

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::packets::{
    AddEntity, BlockUpdate, CommonPlayerSpawnInfo, EntityDataValue, EntityDataValueKind,
    GameEvent as ProtocolGameEvent, PlayLogin, SetEntityData, Vec3d,
};
use bbb_world::{
    vanilla_block_state_id, vanilla_entity_metadata_entity_accessors, ChunkColumn, ChunkPos,
    ChunkSection, ChunkState, LightData, LocalPlayerPoseState, PaletteDomain, PaletteKind,
    PalettedContainerData,
};
use uuid::Uuid;

const LOCAL_PLAYER_ID: i32 = 7;

fn world_with_local_player() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: LOCAL_PLAYER_ID,
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
    });
    world.apply_add_entity(AddEntity {
        id: LOCAL_PLAYER_ID,
        uuid: Uuid::from_u128(LOCAL_PLAYER_ID as u128),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: Vec3d::default(),
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|_| ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    bbb_world::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })
            .collect(),
        block_entities: Vec::new(),
        light: LightData::default(),
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    world
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

fn set_block(world: &mut WorldStore, x: i32, y: i32, z: i32, name: &str) {
    let properties = if name == "minecraft:water" {
        std::collections::BTreeMap::from([("level".to_string(), "0".to_string())])
    } else {
        std::collections::BTreeMap::new()
    };
    let state = vanilla_block_state_id(name, &properties)
        .unwrap_or_else(|| panic!("block state for {name}"));
    assert!(world.apply_block_update(BlockUpdate {
        pos: bbb_protocol::packets::BlockPos { x, y, z },
        block_state_id: state,
    }));
}

fn set_local_player_on_fire(world: &mut WorldStore) {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: LOCAL_PLAYER_ID,
        values: vec![EntityDataValue {
            data_id: accessors.shared_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.on_fire_mask as i8),
        }],
    }));
}

fn set_local_player_sleeping(world: &mut WorldStore) {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: LOCAL_PLAYER_ID,
        values: vec![EntityDataValue {
            data_id: accessors.pose_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_SLEEPING_ID),
        }],
    }));
}

#[test]
fn screen_effect_overlays_gate_on_first_person_sleeping_and_spectator() {
    let mut world = world_with_local_player();
    set_local_player_on_fire(&mut world);
    let textures = TerrainTextureState::default();

    let quads = screen_effect_overlay_quads(&world, &textures, true, 0);
    assert_eq!(quads.len(), 2, "on-fire first person shows two fire quads");

    assert!(
        screen_effect_overlay_quads(&world, &textures, false, 0).is_empty(),
        "third person suppresses every overlay"
    );

    let mut sleeping = world.clone();
    set_local_player_sleeping(&mut sleeping);
    assert!(
        screen_effect_overlay_quads(&sleeping, &textures, true, 0).is_empty(),
        "a sleeping camera player suppresses every overlay"
    );

    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(
        screen_effect_overlay_quads(&world, &textures, true, 0).is_empty(),
        "a spectator (vanilla noPhysics/isSpectator gates) suppresses every overlay"
    );
}

#[test]
fn fire_overlay_quads_match_the_vanilla_side_transforms() {
    let mut world = world_with_local_player();
    set_local_player_on_fire(&mut world);
    let textures = TerrainTextureState::default();

    let quads = screen_effect_overlay_quads(&world, &textures, true, 0);
    assert_eq!(quads.len(), 2);
    for quad in &quads {
        assert_eq!(quad.texture, ScreenEffectTexture::TerrainAtlas);
        assert_eq!(quad.color, [1.0, 1.0, 1.0, 0.9]);
    }
    // side -1: translate(+0.24, -0.3, 0), then rotY(-10 deg) on (+-0.5, +-0.5, -0.5).
    let expected_first = [
        [-0.165_579_8, -0.8, -0.579_228],
        [0.819_228, -0.8, -0.405_579_8],
        [0.819_228, 0.2, -0.405_579_8],
        [-0.165_579_8, 0.2, -0.579_228],
    ];
    // side +1 mirrors it across x.
    let expected_second = [
        [-0.819_228, -0.8, -0.405_579_8],
        [0.165_579_8, -0.8, -0.579_228],
        [0.165_579_8, 0.2, -0.579_228],
        [-0.819_228, 0.2, -0.405_579_8],
    ];
    for (actual, expected) in quads[0].positions.iter().zip(expected_first) {
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a - e).abs() < 1e-5,
                "first fire quad {actual:?} vs {expected:?}"
            );
        }
    }
    for (actual, expected) in quads[1].positions.iter().zip(expected_second) {
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a - e).abs() < 1e-5,
                "second fire quad {actual:?} vs {expected:?}"
            );
        }
    }
}

#[test]
fn underwater_overlay_pans_uvs_with_head_rotation() {
    let mut world = world_with_local_player();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        },
        y_rot: 90.0,
        x_rot: 45.0,
        ..LocalPlayerPoseState::default()
    });
    set_block(&mut world, 0, 1, 0, "minecraft:water");
    let textures = TerrainTextureState::default();

    let quads = screen_effect_overlay_quads(&world, &textures, true, 0);
    assert_eq!(quads.len(), 1);
    let quad = &quads[0];
    assert_eq!(quad.texture, ScreenEffectTexture::Underwater);
    // Full skylight at the eye drives the vanilla brightness curve to 1.0;
    // the overlay alpha is the fixed 0.1.
    assert_eq!(quad.color, [1.0, 1.0, 1.0, 0.1]);
    let uo = -90.0 / 64.0;
    let vo = 45.0 / 64.0;
    let expected = [
        [4.0 + uo, 4.0 + vo],
        [uo, 4.0 + vo],
        [uo, vo],
        [4.0 + uo, vo],
    ];
    for (actual, expected) in quad.uvs.iter().zip(expected) {
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-6, "water uvs {actual:?} vs {expected:?}");
        }
    }
    assert_eq!(
        quad.positions,
        [
            [-1.0, -1.0, -0.5],
            [1.0, -1.0, -0.5],
            [1.0, 1.0, -0.5],
            [-1.0, 1.0, -0.5],
        ]
    );
}

#[test]
fn view_blocking_overlay_flips_the_particle_sprite_and_orders_before_water_and_fire() {
    let mut world = world_with_local_player();
    set_block(&mut world, 0, 1, 0, "minecraft:stone");
    set_local_player_on_fire(&mut world);
    let textures = TerrainTextureState::default();

    let quads = screen_effect_overlay_quads(&world, &textures, true, 0);
    assert_eq!(
        quads.len(),
        3,
        "block overlay then two fire quads (stone displaced the water column)"
    );
    let block = &quads[0];
    assert_eq!(block.texture, ScreenEffectTexture::TerrainAtlas);
    assert_eq!(block.color, [0.1, 0.1, 0.1, 1.0]);
    // Vanilla `renderTex` flips u across the quad; the default texture state
    // resolves the sprite to the unit rect.
    assert_eq!(block.uvs, [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]]);
    assert_eq!(block.positions[0], [-1.0, -1.0, -0.5]);
    for quad in &quads[1..] {
        assert_eq!(quad.color, [1.0, 1.0, 1.0, 0.9]);
    }
}
