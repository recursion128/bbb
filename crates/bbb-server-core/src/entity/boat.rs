//! `AbstractBoat` runtime for the ten built-in boat and raft types.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_LILY_PAD_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID, VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID,
    VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID, VANILLA_VEHICLE_DAMAGE_DATA_ID,
    VANILLA_VEHICLE_HURT_DIR_DATA_ID, VANILLA_VEHICLE_HURT_TIME_DATA_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type, EntityTypeCatalogEntry, VANILLA_ENTITY_TYPES,
    VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID, VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
    VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID, VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID,
    VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID,
    VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID,
};
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::NbtSummaryEntry;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::item::plain_stack;
use crate::entity::leash::LeashData;
use crate::entity::level::fluid_state_tag_membership;
use crate::entity::living::SynchedValue;
use crate::entity::save::EntitySaveError;
use crate::entity::store::EntityStore;
use crate::entity::{move_entity, Entity, EntityData, MoverType, RemovalReason};
use crate::fluid::above;
use bbb_server_types::fluid::{block_state_fluid_state, FluidState};
use bbb_server_types::geometry::{no_block_collision_with_scratch, Aabb, Vec3Ops};

/// `AbstractBoat.getDefaultGravity()` (`AbstractBoat.java:525-526`).
pub const DEFAULT_GRAVITY: f64 = bbb_facts::movement::BOAT_DEFAULT_GRAVITY;

/// `AbstractBoat.BUBBLE_TIME` (`:60`).
pub const BUBBLE_TIME: i32 = 60;

/// `AbstractBoat.TIME_TO_EJECT` (`:57`).
pub const TIME_TO_EJECT: f32 = 60.0;

/// The damage a hit has to push `DATA_ID_DAMAGE` past before the vehicle
/// breaks (`VehicleEntity.java:50`).
pub const DAMAGE_TO_DESTROY: f32 = 40.0;

/// `AbstractBoat.Status` (`:786-792`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoatStatus {
    InWater,
    UnderWater,
    UnderFlowingWater,
    OnLand,
    InAir,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoatData {
    entity_type: &'static EntityTypeCatalogEntry,
    hurt_time: i32,
    hurt_dir: i32,
    damage: f32,
    paddle_left: bool,
    paddle_right: bool,
    /// `AbstractBoat.paddlePositions` (`:73`).
    paddle_positions: [f32; 2],
    bubble_time: i32,
    out_of_control_ticks: f32,
    delta_rotation: f32,
    water_level: f64,
    land_friction: f32,
    status: Option<BoatStatus>,
    old_status: Option<BoatStatus>,
    last_yd: f64,
    is_above_bubble_column: bool,
    bubble_column_direction_is_down: bool,
    /// `AbstractBoat.leashData` (`:79`).
    leash_data: Option<LeashData>,
}

impl BoatData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        let leash_data = self.leash_data.take();
        *self = *previous;
        self.leash_data = leash_data;
    }

    pub const fn new() -> Self {
        Self::with_catalog(&VANILLA_ENTITY_TYPES[VANILLA_ENTITY_TYPE_OAK_BOAT_ID as usize])
    }

    pub(crate) fn for_entity_type(entity_type_id: i32) -> Option<Self> {
        if !is_boat_type(entity_type_id) {
            return None;
        }
        Some(Self::with_catalog(vanilla_entity_type(entity_type_id)?))
    }

    pub(super) const fn with_catalog(entity_type: &'static EntityTypeCatalogEntry) -> Self {
        Self {
            entity_type,
            hurt_time: 0,
            hurt_dir: 1,
            damage: 0.0,
            paddle_left: false,
            paddle_right: false,
            paddle_positions: [0.0; 2],
            bubble_time: 0,
            out_of_control_ticks: 0.0,
            delta_rotation: 0.0,
            water_level: 0.0,
            land_friction: 0.0,
            status: None,
            old_status: None,
            last_yd: 0.0,
            is_above_bubble_column: false,
            bubble_column_direction_is_down: false,
            leash_data: None,
        }
    }

    pub(super) const fn entity_resource_id(&self) -> &'static str {
        self.entity_type.resource_id
    }

    /// `AbstractBoat.getLeashData` / `setLeashData` (`:349-357`).
    pub(crate) const fn leash_data(&self) -> Option<&LeashData> {
        self.leash_data.as_ref()
    }

    pub(crate) const fn leash_data_mut(&mut self) -> Option<&mut LeashData> {
        self.leash_data.as_mut()
    }

    pub(crate) const fn set_leash_data(&mut self, leash_data: Option<LeashData>) {
        self.leash_data = leash_data;
    }

    pub const fn hurt_time(&self) -> i32 {
        self.hurt_time
    }

    pub const fn hurt_dir(&self) -> i32 {
        self.hurt_dir
    }

    pub const fn damage(&self) -> f32 {
        self.damage
    }

    pub const fn bubble_time(&self) -> i32 {
        self.bubble_time
    }

    pub const fn paddle_state(&self) -> (bool, bool) {
        (self.paddle_left, self.paddle_right)
    }

    pub const fn status(&self) -> Option<BoatStatus> {
        self.status
    }

    pub const fn out_of_control_ticks(&self) -> f32 {
        self.out_of_control_ticks
    }

    pub const fn water_level(&self) -> f64 {
        self.water_level
    }

    pub const fn land_friction(&self) -> f32 {
        self.land_friction
    }

    pub const fn last_yd(&self) -> f64 {
        self.last_yd
    }

    pub(crate) const fn delta_rotation(&self) -> f32 {
        self.delta_rotation
    }

    /// `AbstractBoat.isUnderWater()` (`:756-758`).
    pub const fn is_under_water(&self) -> bool {
        matches!(
            self.status,
            Some(BoatStatus::UnderWater | BoatStatus::UnderFlowingWater)
        )
    }
}

impl Default for BoatData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn boat(data: &EntityData) -> Option<&BoatData> {
    match data {
        EntityData::Boat(boat) => Some(boat),
        EntityData::ChestBoat(data) => Some(data.boat()),
        _ => None,
    }
}

pub(crate) fn boat_mut(data: &mut EntityData) -> Option<&mut BoatData> {
    match data {
        EntityData::Boat(boat) => Some(boat),
        EntityData::ChestBoat(data) => Some(data.boat_mut()),
        _ => None,
    }
}

pub fn is_boat(data: &EntityData) -> bool {
    boat(data).is_some()
}

#[cfg(test)]
pub(crate) fn set_delta_rotation_for_test(entity: &mut Entity, delta_rotation: f32) -> bool {
    let Some(data) = boat_mut(&mut entity.data) else {
        return false;
    };
    data.delta_rotation = delta_rotation;
    true
}

/// `VehicleEntity.ignoreExplosion` (`VehicleEntity.java:66-67`).
pub const fn ignore_explosion(indirect_source_is_mob: bool, mob_griefing: bool) -> bool {
    indirect_source_is_mob && !mob_griefing
}

fn data_of(entity: &Entity) -> BoatData {
    match boat(entity.data()) {
        Some(data) => *data,
        None => unreachable!("boat dispatch received {:?}", entity.data()),
    }
}

fn store_data(entity: &mut Entity, updated: BoatData) {
    let current = data_of(entity);
    for (data_id, changed) in [
        (
            VANILLA_VEHICLE_HURT_TIME_DATA_ID,
            updated.hurt_time != current.hurt_time,
        ),
        (
            VANILLA_VEHICLE_HURT_DIR_DATA_ID,
            updated.hurt_dir != current.hurt_dir,
        ),
        (
            VANILLA_VEHICLE_DAMAGE_DATA_ID,
            updated.damage != current.damage,
        ),
        (
            VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID,
            updated.paddle_left != current.paddle_left,
        ),
        (
            VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID,
            updated.paddle_right != current.paddle_right,
        ),
        (
            VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID,
            updated.bubble_time != current.bubble_time,
        ),
    ] {
        if changed {
            entity.mark_synched_dirty(data_id);
        }
    }
    if let Some(data) = boat_mut(&mut entity.data) {
        *data = updated;
    }
}

pub(super) const fn is_boat_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID
            | VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID
            | VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID
            | VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID
            | VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID
            | VANILLA_ENTITY_TYPE_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID
            | VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID
    )
}

pub fn spawn_boat(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: Vec3d,
    y_rot: f32,
) -> Option<i32> {
    let data = EntityData::Boat(BoatData::for_entity_type(entity_type_id)?);
    let uuid = store.next_uuid();
    store.restore_initialized(uuid, entity_type_id, position, data, |entity| {
        entity.set_rot(y_rot, 0.0);
    })
}

fn water_at(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<FluidState> {
    let state = block_state_fluid_state(level.block_state(pos));
    fluid_state_tag_membership(level, state, "minecraft:water").then_some(state)
}

fn water_height(level: &LevelCtx<'_, '_>, state: FluidState, pos: BlockPos) -> f32 {
    bbb_facts::fluid::fluid_height_f32(
        state.to_facts_sample(),
        block_state_fluid_state(level.block_state(above(pos))).family() == state.family(),
    )
}

fn no_collision(level: &LevelCtx<'_, '_>, box_to_test: Aabb) -> bool {
    no_block_collision_with_scratch(
        box_to_test,
        |pos| Some(level.is_moving_piston_at(pos).unwrap_or(true)),
        |pos, out| {
            level.push_block_collision_boxes(pos, out);
            Some(())
        },
    )
    .unwrap_or(true)
}

/// `AbstractBoat.getWaterLevelAbove()` (`:392-414`).
fn water_level_above(data: &BoatData, entity: &Entity, level: &LevelCtx<'_, '_>) -> f32 {
    bbb_facts::fluid::boat_water_level_above(entity.bounding_box(), data.last_yd, |[x, y, z]| {
        let pos = BlockPos { x, y, z };
        water_at(level, pos).map(|state| water_height(level, state, pos))
    })
}

fn to_physics_status(status: BoatStatus) -> bbb_facts::movement::BoatPhysicsStatus {
    match status {
        BoatStatus::InWater => bbb_facts::movement::BoatPhysicsStatus::InWater,
        BoatStatus::UnderWater => bbb_facts::movement::BoatPhysicsStatus::UnderWater,
        BoatStatus::UnderFlowingWater => bbb_facts::movement::BoatPhysicsStatus::UnderFlowingWater,
        BoatStatus::OnLand => bbb_facts::movement::BoatPhysicsStatus::OnLand,
        BoatStatus::InAir => bbb_facts::movement::BoatPhysicsStatus::InAir,
    }
}

/// `AbstractBoat.getGroundFriction()` (`:428-455`).
///
/// The trailing `friction / count` is Java float division, so an empty sample
/// answers NaN and the caller's `> 0.0F` test then chooses `IN_AIR`.
fn ground_friction(entity: &Entity, level: &LevelCtx<'_, '_>) -> f32 {
    use bbb_facts::collision::CollisionBounds;

    let mut scratch = Vec::new();
    bbb_facts::movement::boat_ground_friction(entity.bounding_box(), |[x, y, z], intersects| {
        let pos = BlockPos { x, y, z };
        let state = level.block_state(pos);
        if vanilla_block_state_block_type_id(state) == Some(VANILLA_BLOCK_LILY_PAD_ID) {
            return None;
        }
        let hit = if let Some(mut boxes) =
            bbb_server_types::geometry::static_block_collision_box_iter(state)
        {
            boxes.any(|shape| intersects(shape.to_collision_box()))
        } else {
            scratch.clear();
            level.push_block_collision_boxes(pos, &mut scratch);
            scratch
                .iter()
                .any(|shape| intersects(shape.to_collision_box()))
        };
        hit.then(|| level.block_friction(pos))
    })
}

/// `AbstractBoat.getStatus()` (`:374-388`).
fn status_of(data: &mut BoatData, entity: &Entity, level: &LevelCtx<'_, '_>) -> BoatStatus {
    let (water_status, water_level) =
        bbb_facts::fluid::boat_water_status(entity.bounding_box(), |[x, y, z]| {
            let pos = BlockPos { x, y, z };
            let state = water_at(level, pos)?;
            Some((water_height(level, state, pos), state.is_source()))
        });
    data.water_level = water_level;
    if let Some(status) = water_status {
        return match status {
            bbb_facts::fluid::BoatWaterStatus::InWater => BoatStatus::InWater,
            bbb_facts::fluid::BoatWaterStatus::UnderWater => BoatStatus::UnderWater,
            bbb_facts::fluid::BoatWaterStatus::UnderFlowingWater => BoatStatus::UnderFlowingWater,
        };
    }
    let friction = ground_friction(entity, level);
    if friction > 0.0 {
        data.land_friction = friction;
        BoatStatus::OnLand
    } else {
        BoatStatus::InAir
    }
}

/// `AbstractBoat.floatBoat()` (`:529-569`).
fn float_boat(
    data: &mut BoatData,
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    surface_water_level: Option<f32>,
) {
    let bb_height = entity.bb_height();
    if data.old_status == Some(BoatStatus::InAir)
        && data.status != Some(BoatStatus::InAir)
        && data.status != Some(BoatStatus::OnLand)
    {
        // `:532-541`: the boat snaps to the surface it just entered.
        let water_level = surface_water_level
            .expect("the live water-tag preflight completed before boat movement");
        data.water_level = entity.position().y + f64::from(bb_height);
        let target_y = f64::from(water_level - bb_height) + 0.101;
        let lifted = entity.bounding_box().moved(Vec3d {
            x: 0.0,
            y: target_y - entity.position().y,
            z: 0.0,
        });
        if no_collision(level, lifted) {
            let position = entity.position();
            entity.set_pos(Vec3d {
                y: target_y,
                ..position
            });
            entity.set_delta_movement(entity.delta_movement().multiply(1.0, 0.0, 1.0));
            data.last_yd = 0.0;
        }
        data.status = Some(BoatStatus::InWater);
        return;
    }
    let movement = entity.delta_movement();
    let (next_delta, next_rotation, _) = bbb_facts::movement::boat_float_step(
        [movement.x, movement.y, movement.z],
        data.delta_rotation,
        bbb_facts::movement::BoatFloatInput {
            status: data.status.map(to_physics_status),
            position_y: entity.position().y,
            bb_height: f64::from(bb_height),
            gravity: entity.gravity(),
            water_level: data.water_level,
            land_friction: data.land_friction,
            has_controlling_player: false,
        },
    );
    entity.set_delta_movement(Vec3d {
        x: next_delta[0],
        y: next_delta[1],
        z: next_delta[2],
    });
    data.delta_rotation = next_rotation;
}

/// `AbstractBoat.setPaddleState` (`:340-343`).
pub(crate) fn set_paddle_state(entity: &mut Entity, left: bool, right: bool) {
    let mut data = data_of(entity);
    data.paddle_left = left;
    data.paddle_right = right;
    store_data(entity, data);
}

/// `AbstractBoat.tickBubbleColumn()` (`:295-330`), server half.
fn tick_bubble_column(entity: &mut Entity, has_external_player: bool) -> bool {
    let mut data = data_of(entity);
    let mut eject_passengers = false;
    // `:309-311`.
    if !data.is_above_bubble_column {
        data.bubble_time = 0;
    }
    // `:313-314`.
    if data.bubble_time > 0 {
        data.bubble_time -= 1;
        // `:317-318`.
        let elapsed = BUBBLE_TIME - data.bubble_time - 1;
        if elapsed > 0 && data.bubble_time == 0 {
            data.bubble_time = 0;
            let movement = entity.delta_movement();
            if data.bubble_column_direction_is_down {
                // `:320-321`.
                entity.set_delta_movement(movement.plus(Vec3d {
                    x: 0.0,
                    y: -0.7,
                    z: 0.0,
                }));
                eject_passengers = true;
            } else {
                // `:323`: any Player passenger selects the stronger launch.
                entity.set_delta_movement(Vec3d {
                    y: if has_external_player { 2.7 } else { 0.6 },
                    ..movement
                });
            }
        }
        data.is_above_bubble_column = false;
    }
    store_data(entity, data);
    eject_passengers
}

pub fn on_above_bubble_column(entity: &mut Entity, drag_down: bool) {
    let mut data = data_of(entity);
    data.is_above_bubble_column = true;
    data.bubble_column_direction_is_down = drag_down;
    if data.bubble_time == 0 {
        data.bubble_time = BUBBLE_TIME;
    }
    store_data(entity, data);
}

#[cfg(test)]
pub(crate) fn set_bubble_countdown_for_test(
    entity: &mut Entity,
    bubble_time: i32,
    drag_down: bool,
) {
    let mut data = data_of(entity);
    data.bubble_time = bubble_time;
    data.is_above_bubble_column = true;
    data.bubble_column_direction_is_down = drag_down;
    store_data(entity, data);
}

#[cfg(test)]
pub(crate) fn set_out_of_control_ticks_for_test(entity: &mut Entity, ticks: f32) {
    let mut data = data_of(entity);
    data.out_of_control_ticks = ticks;
    store_data(entity, data);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoatEjectionOutcome {
    None,
    TimeoutBeforeMove,
    BubbleAfterMove,
    TimeoutBeforeAndBubbleAfter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoatTickOutcome {
    pub(crate) ejection: BoatEjectionOutcome,
}

/// `AbstractBoat.tick()` (`:208-268`), server side.
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    external_player_controls: bool,
    has_external_player: bool,
) -> Option<BoatTickOutcome> {
    let mut data = data_of(entity);
    let mut next = data;
    // `:209-210`.
    next.old_status = next.status;
    next.status = Some(status_of(&mut next, entity, level));
    let entering_water = next.old_status == Some(BoatStatus::InAir)
        && next.status != Some(BoatStatus::InAir)
        && next.status != Some(BoatStatus::OnLand);
    let surface_water_level = entering_water.then(|| water_level_above(&next, entity, level));
    data = next;
    // `:211-215`.
    if data.is_under_water() {
        data.out_of_control_ticks += 1.0;
    } else {
        data.out_of_control_ticks = 0.0;
    }
    // `:217-219`: ejection precedes the authority branch, so its tick resumes
    // as an unoccupied, server-authoritative boat.
    let out_of_control_eject = data.out_of_control_ticks >= TIME_TO_EJECT;
    if out_of_control_eject {
        store.eject_body_passengers_reverse(entity);
    }
    // `:221-223`.
    if data.hurt_time > 0 {
        data.hurt_time -= 1;
    }
    // `:225-227`.
    if data.damage > 0.0 {
        data.damage -= 1.0;
    }
    store_data(entity, data);
    if !crate::entity::damage::base_tick_from_environment(entity, store, level) {
        return None;
    }
    if !crate::entity::leash::tick_leash(entity, store, level) {
        return None;
    }
    if external_player_controls && !out_of_control_eject {
        entity.set_delta_movement(Vec3d::default());
    } else {
        set_paddle_state(entity, false, false);
        let mut data = data_of(entity);
        float_boat(&mut data, entity, level, surface_water_level);
        store.refile_detached(entity);
        store_data(entity, data);
        let delta = entity.delta_movement();
        move_entity(entity, store, level, MoverType::Self_, delta);
    }
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store))
        || !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store))
    {
        return None;
    }
    let bubble_eject = tick_bubble_column(entity, has_external_player && !out_of_control_eject);
    play_paddle_sounds(entity, store, level);
    crate::entity::push::boat_push_and_pickup(entity, store, level, external_player_controls)?;
    let ejection = match (out_of_control_eject, bubble_eject) {
        (false, false) => BoatEjectionOutcome::None,
        (true, false) => BoatEjectionOutcome::TimeoutBeforeMove,
        (false, true) => BoatEjectionOutcome::BubbleAfterMove,
        (true, true) => BoatEjectionOutcome::TimeoutBeforeAndBubbleAfter,
    };
    Some(BoatTickOutcome { ejection })
}

/// `AbstractBoat.java:251-271`.
fn play_paddle_sounds(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    use std::f32::consts::PI;
    let mut data = data_of(entity);
    for (side, paddling) in [data.paddle_left, data.paddle_right]
        .into_iter()
        .enumerate()
    {
        if !paddling {
            data.paddle_positions[side] = 0.0;
            continue;
        }
        let position = data.paddle_positions[side];
        if !entity.is_silent()
            && position % (PI * 2.0) <= PI / 4.0
            && (position + PI / 8.0) % (PI * 2.0) >= PI / 4.0
        {
            // `getPaddleSound()` (`:332-338`) re-reads the status.
            let sound = match status_of(&mut data, entity, level) {
                BoatStatus::InWater | BoatStatus::UnderWater | BoatStatus::UnderFlowingWater => {
                    Some("minecraft:entity.boat.paddle_water")
                }
                BoatStatus::OnLand => Some("minecraft:entity.boat.paddle_land"),
                BoatStatus::InAir => None,
            };
            if let Some(sound) = sound {
                // `getViewVector(1.0F)` with the boat's zero pitch.
                let y_rot = f64::from(-entity.y_rot() * (PI / 180.0));
                let view_x = f64::from(bbb_server_types::geometry::mth_sin(y_rot));
                let view_z = f64::from(bbb_server_types::geometry::mth_cos(y_rot));
                let (dx, dz) = if side == 1 {
                    (-view_z, view_x)
                } else {
                    (view_z, -view_x)
                };
                let pitch = 0.8 + 0.4 * store.random().next_float();
                store.play_entity_sound(crate::entity::store::PendingEntitySound {
                    position: Vec3d {
                        x: entity.position().x + dx,
                        y: entity.position().y,
                        z: entity.position().z + dz,
                    },
                    resource_id: sound.to_string(),
                    source: bbb_protocol::packets::SoundSource::Neutral,
                    volume: 1.0,
                    pitch,
                });
            }
        }
        data.paddle_positions[side] = position + PI / 8.0;
    }
    store_data(entity, data);
}

pub(crate) fn check_fall_damage(entity: &mut Entity, level: &LevelCtx<'_, '_>, ya: f64) {
    let mut data = data_of(entity);
    // `:707`.
    data.last_yd = entity.delta_movement().y;
    store_data(entity, data);
    // `:708-714`: `isPassenger` is always false here.
    if entity.on_ground() {
        entity.reset_fall_distance();
        return;
    }
    let block = entity.block_position();
    let below = BlockPos {
        y: block.y - 1,
        ..block
    };
    if water_at(level, below).is_none() && ya < 0.0 {
        entity.fall_distance -= f64::from(ya as f32);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoatHurtSource {
    pub creative_player: bool,
    pub bypasses_invulnerability: bool,
    pub is_fire: bool,
    pub fire_immune: bool,
    pub is_fall: bool,
    pub fall_damage_immune: bool,
    pub causing_entity_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoatHurtEffects {
    pub applied: bool,
    pub succeeded: bool,
    pub emits_entity_damage: bool,
    pub emits_entity_die: bool,
    pub game_event_source_id: Option<i32>,
    pub drops: Vec<RuntimeItemStack>,
}

impl BoatHurtEffects {
    fn refused(succeeded: bool) -> Self {
        Self {
            applied: false,
            succeeded,
            emits_entity_damage: false,
            emits_entity_die: false,
            game_event_source_id: None,
            drops: Vec::new(),
        }
    }
}

fn drop_item(entity: &Entity) -> Option<RuntimeItemStack> {
    plain_stack(entity.data().entity_resource_id(), 1)
}

pub fn hurt_server(
    entity: &mut Entity,
    damage: f32,
    source: BoatHurtSource,
    entity_drops: bool,
) -> BoatHurtEffects {
    // `:35-38`: a removed vehicle answers true having touched nothing.
    if entity.removal_reason().is_some() {
        return BoatHurtEffects::refused(true);
    }
    if entity.invulnerable() && !source.bypasses_invulnerability && !source.creative_player
        || source.is_fire && source.fire_immune
        || source.is_fall && source.fall_damage_immune
    {
        return BoatHurtEffects::refused(false);
    }
    let mut data = data_of(entity);
    // `:44-45`.
    data.hurt_dir = -data.hurt_dir;
    data.hurt_time = 10;
    store_data(entity, data);
    // `:46`.
    entity.mark_hurt();
    // `:47`.
    data.damage += damage * 10.0;
    store_data(entity, data);
    // `:49`.
    let creative_player = source.creative_player;
    let mut drops = Vec::new();
    let mut destroyed = false;
    if creative_player
        || data.damage.partial_cmp(&DAMAGE_TO_DESTROY) != Some(std::cmp::Ordering::Greater)
    {
        // `:50-53`.
        if creative_player {
            entity.discard();
        }
    } else {
        // `:55` over `destroy(ServerLevel, Item)` (`:70-77`).
        destroyed = true;
        entity.set_removed(RemovalReason::Killed);
        if entity_drops {
            drops.extend(drop_item(entity));
        }
    }
    BoatHurtEffects {
        applied: true,
        succeeded: true,
        emits_entity_damage: true,
        emits_entity_die: destroyed,
        game_event_source_id: source.causing_entity_id,
        drops,
    }
}

pub fn synched_values(entity: &Entity, data: &BoatData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_VEHICLE_HURT_TIME_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(data.hurt_time),
        },
        is_default: data.hurt_time == 0,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_VEHICLE_HURT_DIR_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(data.hurt_dir),
        },
        is_default: data.hurt_dir == 1,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_VEHICLE_DAMAGE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(data.damage),
        },
        is_default: data.damage == 0.0,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(data.paddle_left),
        },
        is_default: !data.paddle_left,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(data.paddle_right),
        },
        is_default: !data.paddle_right,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(data.bubble_time),
        },
        is_default: data.bubble_time == 0,
    });
    rows
}

pub fn read_save_data(entries: &[NbtSummaryEntry], mut data: BoatData) -> BoatData {
    data.leash_data = crate::entity::leash::read_leash_data(entries);
    data
}

pub fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &BoatData,
) -> Result<(), EntitySaveError> {
    crate::entity::leash::write_leash_data(entries, data.leash_data.as_ref())
}
