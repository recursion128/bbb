use super::*;

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
    AttributeSnapshot as ProtocolAttributeSnapshot, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, ChatFormatting as ProtocolChatFormatting,
    CommonPlayerSpawnInfo as ProtocolSpawnInfo, DamageEvent as ProtocolDamageEvent,
    DataComponentPatchSummary, EntityAnimation as ProtocolEntityAnimation,
    EntityDataEnumSerializer, EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EntityEvent as ProtocolEntityEvent, EntityMove as ProtocolEntityMove,
    EntityPositionSync as ProtocolEntityPositionSync, EquipmentSlot, EquipmentSlotUpdate,
    FireworkExplosionShapeSummary, FireworkExplosionSummary, GameEvent as ProtocolGameEvent,
    GameProfile as ProtocolGameProfile, GameType as ProtocolGameType,
    HurtAnimation as ProtocolHurtAnimation, InteractionHand, ItemEnchantmentSummary,
    ItemStackSummary, ItemStackSummary as ProtocolItemStackSummary,
    LevelEvent as ProtocolLevelEvent, MinecartStep as ProtocolMinecartStep, MobEffectFlags,
    MoveMinecartAlongTrack as ProtocolMoveMinecartAlongTrack, MoveVehicle as ProtocolMoveVehicle,
    PlayLogin as ProtocolPlayLogin, PlayTime as ProtocolPlayTime,
    PlayerInfoAction as ProtocolPlayerInfoAction, PlayerInfoEntry as ProtocolPlayerInfoEntry,
    PlayerInfoUpdate as ProtocolPlayerInfoUpdate, PlayerTeamMethod as ProtocolPlayerTeamMethod,
    PlayerTeamParameters as ProtocolPlayerTeamParameters, RemoveEntities as ProtocolRemoveEntities,
    RemoveMobEffect as ProtocolRemoveMobEffect, RotateHead as ProtocolRotateHead,
    SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
    SetEntityMotion as ProtocolSetEntityMotion, SetEquipment as ProtocolSetEquipment,
    SetPassengers as ProtocolSetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
    SetPlayerTeam as ProtocolSetPlayerTeam, SwingAnimationSummary, SwingAnimationTypeSummary,
    TakeItemEntity as ProtocolTakeItemEntity, TeamCollisionRule as ProtocolTeamCollisionRule,
    TeamVisibility as ProtocolTeamVisibility, TeleportEntity as ProtocolTeleportEntity,
    UpdateAttributes as ProtocolUpdateAttributes, UpdateMobEffect, Vec3d as ProtocolVec3d,
    PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_X,
};

mod aquatic_and_beam_creature_animation;
mod combat_state_and_mob_flag_projection;
mod creature_pick_bounds_projection;
mod item_entity_and_pickup_projection;
mod lifecycle_and_queries;
mod movement_combat_and_swing_projection;
mod passenger_and_vehicle_movement;
mod player_pose_and_sleeping_projection;
mod small_mob_and_walk_animation;
mod specialized_pick_bounds_and_render_scale;
mod transient_events_and_animation_triggers;
mod vehicle_and_block_entity_projection;
mod visibility_equipment_and_mount_projection;

fn minecart_step(
    x: f64,
    y: f64,
    z: f64,
    xa: f64,
    ya: f64,
    za: f64,
    y_rot: f32,
    x_rot: f32,
    weight: f32,
) -> ProtocolMinecartStep {
    ProtocolMinecartStep {
        position: ProtocolVec3d { x, y, z },
        movement: ProtocolVec3d {
            x: xa,
            y: ya,
            z: za,
        },
        y_rot,
        x_rot,
        weight,
    }
}

const SHULKER_TYPE_ID: i32 = 112;
const SHULKER_ATTACH_FACE_DATA_ID: u8 = 16;
const SHULKER_PEEK_DATA_ID: u8 = 17;
const DIRECTION_DOWN: i32 = 0;
const DIRECTION_UP: i32 = 1;
const DIRECTION_NORTH: i32 = 2;
const DIRECTION_SOUTH: i32 = 3;
const DIRECTION_WEST: i32 = 4;
const DIRECTION_EAST: i32 = 5;

fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
    protocol_add_entity_with_type(id, 7)
}

fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
    protocol_add_entity_with_type_data(id, entity_type_id, 99)
}

fn protocol_add_entity_with_type_y_rot(
    id: i32,
    entity_type_id: i32,
    y_rot: f32,
) -> ProtocolAddEntity {
    ProtocolAddEntity {
        y_rot,
        ..protocol_add_entity_with_type(id, entity_type_id)
    }
}

fn protocol_add_entity_with_type_data(
    id: i32,
    entity_type_id: i32,
    data: i32,
) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
        entity_type_id,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data,
    }
}

fn default_entity_uuid() -> Uuid {
    Uuid::from_u128(0x12345678123456781234567812345678)
}

fn wind_charge_pick_bounds() -> EntityPickBoundsState {
    let half_width = 0.3125 / 2.0;
    EntityPickBoundsState {
        min: [-half_width, -0.15, -half_width],
        max: [half_width, -0.15 + 0.3125, half_width],
        pick_radius: 1.0,
    }
}

fn shulker_attach_face_data(direction_id: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: SHULKER_ATTACH_FACE_DATA_ID,
        serializer_id: 12,
        value: EntityDataValueKind::Direction(direction_id),
    }
}

fn shulker_peek_data(raw_peek: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: SHULKER_PEEK_DATA_ID,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(raw_peek),
    }
}

fn shulker_pick_bounds(
    attach_face_id: i32,
    client_peek_amount: f32,
    scale: f32,
) -> EntityPickBoundsState {
    let physical_peek = 0.5 - ((0.5 + client_peek_amount) * std::f32::consts::PI).sin() * 0.5;
    let half_size = scale * 0.5;
    let mut min = [-half_size, 0.0, -half_size];
    let mut max = [half_size, scale, half_size];
    let extension = physical_peek * scale;

    match opposite_direction_id(attach_face_id) {
        DIRECTION_DOWN => min[1] -= extension,
        DIRECTION_UP => max[1] += extension,
        DIRECTION_NORTH => min[2] -= extension,
        DIRECTION_SOUTH => max[2] += extension,
        DIRECTION_WEST => min[0] -= extension,
        DIRECTION_EAST => max[0] += extension,
        _ => unreachable!("unexpected vanilla direction id"),
    }

    EntityPickBoundsState {
        min,
        max,
        pick_radius: 0.0,
    }
}

fn opposite_direction_id(direction_id: i32) -> i32 {
    match direction_id {
        DIRECTION_DOWN => DIRECTION_UP,
        DIRECTION_UP => DIRECTION_DOWN,
        DIRECTION_NORTH => DIRECTION_SOUTH,
        DIRECTION_SOUTH => DIRECTION_NORTH,
        DIRECTION_WEST => DIRECTION_EAST,
        DIRECTION_EAST => DIRECTION_WEST,
        _ => unreachable!("unexpected vanilla direction id"),
    }
}

fn assert_pick_bounds_close(
    actual: Option<EntityPickBoundsState>,
    expected: EntityPickBoundsState,
) {
    const EPSILON: f32 = 0.000_01;

    let actual = actual.expect("entity should have pick bounds");
    for axis in 0..3 {
        assert!(
            (actual.min[axis] - expected.min[axis]).abs() <= EPSILON,
            "min[{axis}] expected {:?}, got {:?}",
            expected.min,
            actual.min,
        );
        assert!(
            (actual.max[axis] - expected.max[axis]).abs() <= EPSILON,
            "max[{axis}] expected {:?}, got {:?}",
            expected.max,
            actual.max,
        );
    }
    assert!(
        (actual.pick_radius - expected.pick_radius).abs() <= EPSILON,
        "pick_radius expected {}, got {}",
        expected.pick_radius,
        actual.pick_radius,
    );
}

fn assert_entity_vec3_close(actual: EntityVec3, expected: EntityVec3) {
    const EPSILON: f64 = 0.000_000_1;

    assert!(
        (actual.x - expected.x).abs() <= EPSILON,
        "x: expected {}, got {}",
        expected.x,
        actual.x
    );
    assert!(
        (actual.y - expected.y).abs() <= EPSILON,
        "y: expected {}, got {}",
        expected.y,
        actual.y
    );
    assert!(
        (actual.z - expected.z).abs() <= EPSILON,
        "z: expected {}, got {}",
        expected.z,
        actual.z
    );
}

fn assert_close3_f32(actual: [f32; 3], expected: [f32; 3]) {
    const EPSILON: f32 = 0.000_01;

    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= EPSILON,
            "axis {axis}: expected {:?}, got {:?}",
            expected,
            actual
        );
    }
}

fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, &string_props(props))
        .unwrap_or_else(|| panic!("vanilla 26.1 block state exists for {name}"))
        .id
}

fn string_props<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn pick_target(targets: &[EntityPickTargetState], entity_id: i32) -> &EntityPickTargetState {
    targets
        .iter()
        .find(|target| target.entity_id == entity_id)
        .unwrap_or_else(|| panic!("missing pick target {entity_id}"))
}

fn protocol_player_info_entry_with_mode(
    profile_id: Uuid,
    game_mode: ProtocolGameType,
) -> ProtocolPlayerInfoEntry {
    ProtocolPlayerInfoEntry {
        profile_id,
        profile: Some(ProtocolGameProfile {
            uuid: profile_id,
            name: "PickTarget".to_string(),
            properties: Vec::new(),
        }),
        listed: true,
        latency: 0,
        game_mode,
        display_name: None,
        show_hat: true,
        list_order: 0,
        chat_session: None,
    }
}

fn protocol_pose_data(data_id: u8, pose_id: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 20,
        value: EntityDataValueKind::Pose(pose_id),
    }
}

fn protocol_bool_data(data_id: u8, value: bool) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 8,
        value: EntityDataValueKind::Boolean(value),
    }
}

fn protocol_byte_data(data_id: u8, value: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(value),
    }
}

fn protocol_int_data(data_id: u8, value: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 1,
        value: EntityDataValueKind::Int(value),
    }
}

fn protocol_block_state_data(data_id: u8, value: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 14,
        value: EntityDataValueKind::BlockState(value),
    }
}

fn protocol_long_data(data_id: u8, value: i64) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 2,
        value: EntityDataValueKind::Long(value),
    }
}

fn protocol_optional_unsigned_int_data(data_id: u8, value: Option<i32>) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 19,
        value: EntityDataValueKind::OptionalUnsignedInt(value),
    }
}

fn protocol_optional_block_state_data(data_id: u8, value: Option<i32>) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 15,
        value: EntityDataValueKind::OptionalBlockState(value),
    }
}

fn protocol_float_data(data_id: u8, value: f32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 3,
        value: EntityDataValueKind::Float(value),
    }
}

fn protocol_enum_data(
    data_id: u8,
    serializer: EntityDataEnumSerializer,
    id: i32,
) -> ProtocolEntityDataValue {
    let serializer_id = match serializer {
        EntityDataEnumSerializer::SnifferState => 35,
        EntityDataEnumSerializer::ArmadilloState => 36,
        EntityDataEnumSerializer::CopperGolemState => 37,
        EntityDataEnumSerializer::WeatheringCopperState => 38,
    };

    ProtocolEntityDataValue {
        data_id,
        serializer_id,
        value: EntityDataValueKind::EnumId { serializer, id },
    }
}

fn protocol_humanoid_arm_data(data_id: u8, arm_id: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: 42,
        value: EntityDataValueKind::HumanoidArm(arm_id),
    }
}

fn protocol_play_login(player_id: i32) -> ProtocolPlayLogin {
    ProtocolPlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: ProtocolSpawnInfo {
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

fn item_stack_entity_data(item: ProtocolItemStackSummary) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: VANILLA_ITEM_ENTITY_STACK_DATA_ID,
        serializer_id: 7,
        value: EntityDataValueKind::ItemStack(item),
    }
}

fn living_entity_flags_data(flags: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id: 8,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(flags),
    }
}

/// A single decoded chunk of air at (0, 0), used to back block lookups (e.g. the
/// sleeping bed orientation) in entity-source tests.
fn empty_test_chunk() -> crate::ChunkColumn {
    crate::ChunkColumn {
        pos: crate::ChunkPos { x: 0, z: 0 },
        state: crate::ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![crate::ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(crate::PaletteDomain::BlockStates, 4096, 0),
            biomes: single_value_container(crate::PaletteDomain::Biomes, 64, 0),
        }],
        block_entities: Vec::new(),
        light: crate::LightData::default(),
    }
}

fn single_value_container(
    domain: crate::PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> crate::PalettedContainerData {
    crate::PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: crate::PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}
