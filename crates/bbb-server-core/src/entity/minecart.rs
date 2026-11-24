//! `AbstractMinecart` runtime and `NewMinecartBehavior` track-interpolation steps.

use crate::block_behavior::{is_base_rail_block, string_property};
use crate::connection::ClientboundPlayPacket;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_types::{VANILLA_BLOCK_ACTIVATOR_RAIL_ID, VANILLA_BLOCK_POWERED_RAIL_ID};
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
    VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID, VANILLA_FURNACE_MINECART_FUEL_DATA_ID,
    VANILLA_VEHICLE_DAMAGE_DATA_ID, VANILLA_VEHICLE_HURT_DIR_DATA_ID,
    VANILLA_VEHICLE_HURT_TIME_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
    VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
    VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
};
use bbb_protocol::nbt_summary::compound_field as field;
use bbb_protocol::packets::{
    BlockPos, EntityDataValue, EntityDataValueKind, EntityEvent, SoundSource, Vec3d,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::item::plain_stack;
use crate::entity::level::{block_state_tag_membership, builtin_registry_tag_membership};
use crate::entity::living::SynchedValue;
use crate::entity::save::EntitySaveError;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{
    check_below_world, handle_portal, move_entity, Entity, EntityData, MoverType, RemovalReason,
};

pub const DEFAULT_GRAVITY: f64 = 0.04;
pub const WATER_GRAVITY: f64 = 0.005;
pub const DEFAULT_DISPLAY_OFFSET: i32 = 6;
pub const DAMAGE_TO_DESTROY: f32 = 40.0;
const FURNACE_FUEL_TICKS_PER_ITEM: i32 = 3_600;
const FURNACE_MAX_FUEL_TICKS: i32 = 32_000;

/// Pending `NewMinecartBehavior.lerpSteps` (`NewMinecartBehavior.java:36`)
/// drained by `ServerEntity.handleMinecartPosRot` (`ServerEntity.java:226-255`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MinecartTrackInterpolation {
    pub(crate) lerp_steps: Vec<bbb_protocol::packets::MinecartStep>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinecartData {
    hurt_time: i32,
    hurt_dir: i32,
    damage: f32,
    custom_display_block: Option<i32>,
    display_offset: i32,
    default_display_offset: i32,
    on_rails: bool,
    flipped: bool,
    first_tick: bool,
}

impl MinecartData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.hurt_time = previous.hurt_time;
        self.hurt_dir = previous.hurt_dir;
        self.damage = previous.damage;
        self.on_rails = previous.on_rails;
    }

    /// `VehicleEntity.java:80-84` and `AbstractMinecart.java:156-160`.
    pub const fn new() -> Self {
        Self::with_default_display_offset(DEFAULT_DISPLAY_OFFSET)
    }

    pub(crate) const fn with_default_display_offset(default_display_offset: i32) -> Self {
        Self {
            hurt_time: 0,
            hurt_dir: 1,
            damage: 0.0,
            custom_display_block: None,
            display_offset: default_display_offset,
            default_display_offset,
            on_rails: false,
            flipped: false,
            first_tick: true,
        }
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

    pub const fn custom_display_block(&self) -> Option<i32> {
        self.custom_display_block
    }

    pub const fn display_offset(&self) -> i32 {
        self.display_offset
    }

    pub const fn default_display_offset(&self) -> i32 {
        self.default_display_offset
    }

    pub const fn on_rails(&self) -> bool {
        self.on_rails
    }

    pub const fn is_flipped(&self) -> bool {
        self.flipped
    }

    pub const fn is_first_tick(&self) -> bool {
        self.first_tick
    }
}

impl Default for MinecartData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FurnaceMinecartData {
    minecart: MinecartData,
    fuel: i32,
    push: Vec3d,
    has_fuel: bool,
}

impl FurnaceMinecartData {
    pub const fn new() -> Self {
        Self {
            minecart: MinecartData::new(),
            fuel: 0,
            push: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            has_fuel: false,
        }
    }

    pub const fn minecart(&self) -> &MinecartData {
        &self.minecart
    }

    pub(crate) fn minecart_mut(&mut self) -> &mut MinecartData {
        &mut self.minecart
    }

    pub const fn fuel(&self) -> i32 {
        self.fuel
    }

    pub const fn push(&self) -> Vec3d {
        self.push
    }

    pub const fn has_fuel(&self) -> bool {
        self.has_fuel
    }

    fn from_saved(minecart: MinecartData, fuel: i32, push: Vec3d) -> Self {
        Self {
            minecart,
            fuel,
            push,
            // The constructor defines false; the first server tick derives it.
            has_fuel: false,
        }
    }
}

impl Default for FurnaceMinecartData {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn is_minecart(data: &EntityData) -> bool {
    matches!(
        data,
        EntityData::Minecart(_)
            | EntityData::FurnaceMinecart(_)
            | EntityData::ChestMinecart(_)
            | EntityData::HopperMinecart(_)
            | EntityData::CommandBlockMinecart(_)
            | EntityData::SpawnerMinecart(_)
            | EntityData::TntMinecart(_)
    )
}

/// `VehicleEntity.ignoreExplosion` (`VehicleEntity.java:66-67`).
pub const fn ignore_explosion(indirect_source_is_mob: bool, mob_griefing: bool) -> bool {
    indirect_source_is_mob && !mob_griefing
}

fn data_of(entity: &Entity) -> MinecartData {
    match entity.data() {
        EntityData::Minecart(data) => *data,
        EntityData::FurnaceMinecart(data) => data.minecart,
        EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => *data.minecart(),
        EntityData::CommandBlockMinecart(data) => *data.minecart(),
        EntityData::SpawnerMinecart(data) => *data.minecart(),
        EntityData::TntMinecart(data) => *data.minecart(),
        _ => unreachable!("minecart dispatch received {:?}", entity.data()),
    }
}

pub(crate) fn first_tick(entity: &Entity) -> Option<bool> {
    is_minecart(entity.data()).then(|| data_of(entity).is_first_tick())
}

pub(crate) fn is_flipped(entity: &Entity) -> bool {
    is_minecart(entity.data()) && data_of(entity).is_flipped()
}

fn store_data(entity: &mut Entity, updated: MinecartData) {
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
            VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
            updated.custom_display_block != current.custom_display_block,
        ),
        (
            VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
            updated.display_offset != current.display_offset,
        ),
    ] {
        if changed {
            entity.mark_synched_dirty(data_id);
        }
    }
    match &mut entity.data {
        EntityData::Minecart(data) => *data = updated,
        EntityData::FurnaceMinecart(data) => data.minecart = updated,
        EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => {
            *data.minecart_mut() = updated;
        }
        EntityData::CommandBlockMinecart(data) => *data.minecart_mut() = updated,
        EntityData::SpawnerMinecart(data) => *data.minecart_mut() = updated,
        EntityData::TntMinecart(data) => *data.minecart_mut() = updated,
        _ => unreachable!("minecart dispatch received a non-minecart"),
    }
}

pub fn spawn_minecart(store: &mut EntityStore, position: Vec3d, y_rot: f32) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        position,
        EntityData::Minecart(MinecartData::new()),
        |entity| entity.set_rot(y_rot, 0.0),
    )
}

pub fn spawn_furnace_minecart(store: &mut EntityStore, position: Vec3d, y_rot: f32) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
        position,
        EntityData::FurnaceMinecart(FurnaceMinecartData::new()),
        |entity| entity.set_rot(y_rot, 0.0),
    )
}

pub fn spawn_command_block_minecart(
    store: &mut EntityStore,
    position: Vec3d,
    y_rot: f32,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        position,
        EntityData::CommandBlockMinecart(crate::entity::CommandBlockMinecartData::new()),
        |entity| entity.set_rot(y_rot, 0.0),
    )
}

pub fn spawn_tnt_minecart(store: &mut EntityStore, position: Vec3d, y_rot: f32) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        position,
        EntityData::TntMinecart(crate::entity::TntMinecartData::new()),
        |entity| entity.set_rot(y_rot, 0.0),
    )
}

pub fn spawn_spawner_minecart(store: &mut EntityStore, position: Vec3d, y_rot: f32) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
        position,
        EntityData::SpawnerMinecart(crate::entity::SpawnerMinecartData::new()),
        |entity| entity.set_rot(y_rot, 0.0),
    )
}

pub fn spawner_minecart_data_mut(
    entity: &mut Entity,
) -> Option<&mut crate::entity::SpawnerMinecartData> {
    match &mut entity.data {
        EntityData::SpawnerMinecart(data) => Some(data),
        _ => None,
    }
}

pub fn add_furnace_minecart_fuel(
    entity: &mut Entity,
    interacting_position: Vec3d,
    item_resource_id: &str,
) -> bool {
    add_furnace_minecart_fuel_if_member(
        entity,
        interacting_position,
        builtin_registry_tag_membership(
            "minecraft:item",
            "minecraft:furnace_minecart_fuel",
            item_resource_id,
        ),
    )
}

#[cfg(test)]
pub(crate) fn add_furnace_minecart_fuel_in_level(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    interacting_position: Vec3d,
    item_resource_id: &str,
) -> bool {
    add_furnace_minecart_fuel_if_member(
        entity,
        interacting_position,
        level.registry_tag_membership(
            "minecraft:item",
            "minecraft:furnace_minecart_fuel",
            item_resource_id,
        ),
    )
}

pub(crate) fn add_furnace_minecart_fuel_if_member(
    entity: &mut Entity,
    interacting_position: Vec3d,
    membership: bool,
) -> bool {
    let position = entity.position();
    let EntityData::FurnaceMinecart(data) = &mut entity.data else {
        return false;
    };
    if !membership || data.fuel + FURNACE_FUEL_TICKS_PER_ITEM > FURNACE_MAX_FUEL_TICKS {
        return false;
    }
    data.fuel += FURNACE_FUEL_TICKS_PER_ITEM;
    data.push = Vec3d {
        x: position.x - interacting_position.x,
        y: 0.0,
        z: position.z - interacting_position.z,
    };
    true
}

pub fn set_custom_display_block(entity: &mut Entity, state: Option<i32>) {
    let mut data = data_of(entity);
    data.custom_display_block = state;
    store_data(entity, data);
}

pub fn set_display_offset(entity: &mut Entity, offset: i32) {
    let mut data = data_of(entity);
    data.display_offset = offset;
    store_data(entity, data);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RailShape {
    NorthSouth,
    EastWest,
    AscendingEast,
    AscendingWest,
    AscendingNorth,
    AscendingSouth,
    SouthEast,
    SouthWest,
    NorthWest,
    NorthEast,
}

impl RailShape {
    fn from_state(block_state_id: i32) -> Option<Self> {
        match string_property(block_state_id, "shape")?.as_str() {
            "north_south" => Some(Self::NorthSouth),
            "east_west" => Some(Self::EastWest),
            "ascending_east" => Some(Self::AscendingEast),
            "ascending_west" => Some(Self::AscendingWest),
            "ascending_north" => Some(Self::AscendingNorth),
            "ascending_south" => Some(Self::AscendingSouth),
            "south_east" => Some(Self::SouthEast),
            "south_west" => Some(Self::SouthWest),
            "north_west" => Some(Self::NorthWest),
            "north_east" => Some(Self::NorthEast),
            _ => None,
        }
    }

    /// `AbstractMinecart.EXITS` (`AbstractMinecart.java:63-95`).
    const fn exits(self) -> ((i32, i32, i32), (i32, i32, i32)) {
        match self {
            Self::NorthSouth => ((0, 0, -1), (0, 0, 1)),
            Self::EastWest => ((-1, 0, 0), (1, 0, 0)),
            Self::AscendingEast => ((-1, -1, 0), (1, 0, 0)),
            Self::AscendingWest => ((-1, 0, 0), (1, -1, 0)),
            Self::AscendingNorth => ((0, 0, -1), (0, -1, 1)),
            Self::AscendingSouth => ((0, -1, -1), (0, 0, 1)),
            Self::SouthEast => ((0, 0, 1), (1, 0, 0)),
            Self::SouthWest => ((0, 0, 1), (-1, 0, 0)),
            Self::NorthWest => ((0, 0, -1), (-1, 0, 0)),
            Self::NorthEast => ((0, 0, -1), (1, 0, 0)),
        }
    }
}

fn offset(pos: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: pos.x.wrapping_add(x),
        y: pos.y.wrapping_add(y),
        z: pos.z.wrapping_add(z),
    }
}

fn current_rail_pos(entity: &Entity, level: &LevelCtx<'_, '_>) -> BlockPos {
    let position = entity.position();
    let pos = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    let below = offset(pos, 0, -1, 0);
    if is_in_rails_tag(level, level.block_state(below)) {
        below
    } else {
        pos
    }
}

fn block_type(block_state_id: i32) -> Option<i32> {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
}

fn is_in_rails_tag(level: &LevelCtx<'_, '_>, block_state_id: i32) -> bool {
    block_state_tag_membership(level, block_state_id, "minecraft:rails")
}

fn is_rail_state(level: &LevelCtx<'_, '_>, block_state_id: i32) -> bool {
    is_in_rails_tag(level, block_state_id) && is_base_rail_block(block_state_id)
}

fn get_pos(level: &LevelCtx<'_, '_>, x: f64, y: f64, z: f64) -> Option<Vec3d> {
    let xt = x.floor() as i32;
    let mut yt = y.floor() as i32;
    let zt = z.floor() as i32;
    if is_in_rails_tag(
        level,
        level.block_state(BlockPos {
            x: xt,
            y: yt - 1,
            z: zt,
        }),
    ) {
        yt -= 1;
    }
    let state = level.block_state(BlockPos {
        x: xt,
        y: yt,
        z: zt,
    });
    if !is_rail_state(level, state) {
        return None;
    }
    let shape = RailShape::from_state(state)?;
    let (a, b) = shape.exits();
    let x0 = f64::from(xt) + 0.5 + f64::from(a.0) * 0.5;
    let y0 = f64::from(yt) + 0.0625 + f64::from(a.1) * 0.5;
    let z0 = f64::from(zt) + 0.5 + f64::from(a.2) * 0.5;
    let x1 = f64::from(xt) + 0.5 + f64::from(b.0) * 0.5;
    let y1 = f64::from(yt) + 0.0625 + f64::from(b.1) * 0.5;
    let z1 = f64::from(zt) + 0.5 + f64::from(b.2) * 0.5;
    let xd = x1 - x0;
    let yd = (y1 - y0) * 2.0;
    let zd = z1 - z0;
    let progress = if xd == 0.0 {
        z - f64::from(zt)
    } else if zd == 0.0 {
        x - f64::from(xt)
    } else {
        ((x - x0) * xd + (z - z0) * zd) * 2.0
    };
    let mut rail_y = y0 + yd * progress;
    if yd < 0.0 {
        rail_y += 1.0;
    } else if yd > 0.0 {
        rail_y += 0.5;
    }
    Some(Vec3d {
        x: x0 + xd * progress,
        y: rail_y,
        z: z0 + zd * progress,
    })
}

fn move_along_track(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let pos = current_rail_pos(entity, level);
    let state = level.block_state(pos);
    if !is_rail_state(level, state) {
        return true;
    }
    let Some(shape) = RailShape::from_state(state) else {
        return true;
    };
    let old_position = entity.position();
    let old_rail_pos = get_pos(level, old_position.x, old_position.y, old_position.z);
    entity.reset_fall_distance();
    let mut y = f64::from(pos.y);
    let powered_rail = block_type(state) == Some(VANILLA_BLOCK_POWERED_RAIL_ID);
    let powered = powered_rail
        && bbb_server_types::block_queries::bool_property(state, "powered").unwrap_or(false);
    let halted = powered_rail && !powered;
    let slide = if entity.was_touching_water() {
        0.0078125 * 0.2
    } else {
        0.0078125
    };
    let movement = entity.delta_movement();
    entity.set_delta_movement(match shape {
        RailShape::AscendingEast => Vec3d {
            x: movement.x - slide,
            ..movement
        },
        RailShape::AscendingWest => Vec3d {
            x: movement.x + slide,
            ..movement
        },
        RailShape::AscendingNorth => Vec3d {
            z: movement.z + slide,
            ..movement
        },
        RailShape::AscendingSouth => Vec3d {
            z: movement.z - slide,
            ..movement
        },
        _ => movement,
    });
    if matches!(
        shape,
        RailShape::AscendingEast
            | RailShape::AscendingWest
            | RailShape::AscendingNorth
            | RailShape::AscendingSouth
    ) {
        y += 1.0;
    }
    let (a, b) = shape.exits();
    let mut xd = f64::from(b.0 - a.0);
    let mut zd = f64::from(b.2 - a.2);
    let length = (xd * xd + zd * zd).sqrt();
    let movement = entity.delta_movement();
    if movement.x * xd + movement.z * zd < 0.0 {
        xd = -xd;
        zd = -zd;
    }
    let speed = movement.horizontal_distance_sqr().sqrt().min(2.0);
    entity.set_delta_movement(Vec3d {
        x: speed * xd / length,
        y: movement.y,
        z: speed * zd / length,
    });
    let mut halted = halted;
    // `OldMinecartBehavior.java:161-176`: a player rider nudges a near-stopped cart.
    let intent = store
        .passengers(entity.id())
        .first()
        .and_then(|id| store.player(*id))
        .map(crate::world::PlayerState::last_client_move_intent);
    if let Some(intent) = intent.filter(|intent| intent.length_sqr() > 0.0) {
        if entity.delta_movement().horizontal_distance_sqr() < 0.01 {
            let movement = entity.delta_movement();
            entity.set_delta_movement(Vec3d {
                x: movement.x + intent.x * 0.001,
                z: movement.z + intent.z * 0.001,
                ..movement
            });
            halted = false;
        }
    }
    if halted {
        let speed = entity.delta_movement().horizontal_distance_sqr().sqrt();
        if speed < 0.03 {
            entity.set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        } else {
            entity.set_delta_movement(entity.delta_movement().multiply(0.5, 0.0, 0.5));
        }
    }
    let x0 = f64::from(pos.x) + 0.5 + f64::from(a.0) * 0.5;
    let z0 = f64::from(pos.z) + 0.5 + f64::from(a.2) * 0.5;
    let x1 = f64::from(pos.x) + 0.5 + f64::from(b.0) * 0.5;
    let z1 = f64::from(pos.z) + 0.5 + f64::from(b.2) * 0.5;
    let rail_xd = x1 - x0;
    let rail_zd = z1 - z0;
    let progress = if rail_xd == 0.0 {
        old_position.z - f64::from(pos.z)
    } else if rail_zd == 0.0 {
        old_position.x - f64::from(pos.x)
    } else {
        ((old_position.x - x0) * rail_xd + (old_position.z - z0) * rail_zd) * 2.0
    };
    entity.set_pos(Vec3d {
        x: x0 + rail_xd * progress,
        y,
        z: z0 + rail_zd * progress,
    });
    store.refile_detached(entity);
    let ridden = !store.passengers(entity.id()).is_empty();
    // `OldMinecartBehavior.java:207-210`.
    let scale = if ridden { 0.75 } else { 1.0 };
    let movement = entity.delta_movement();
    let max_speed = max_speed(entity);
    move_entity(
        entity,
        store,
        level,
        MoverType::Self_,
        Vec3d {
            x: (scale * movement.x).clamp(-max_speed, max_speed),
            y: 0.0,
            z: (scale * movement.z).clamp(-max_speed, max_speed),
        },
    );
    let current = entity.position();
    if !crate::entity::apply_nonliving_inside_block_effects_between(
        entity,
        level,
        Some(&mut *store),
        current,
        current,
    ) {
        return false;
    }
    entity.clear_movement_this_tick();
    let now = entity.position();
    if a.1 != 0 && now.x.floor() as i32 - pos.x == a.0 && now.z.floor() as i32 - pos.z == a.2 {
        entity.set_pos(Vec3d {
            y: now.y + f64::from(a.1),
            ..now
        });
        store.refile_detached(entity);
    } else if b.1 != 0 && now.x.floor() as i32 - pos.x == b.0 && now.z.floor() as i32 - pos.z == b.2
    {
        entity.set_pos(Vec3d {
            y: now.y + f64::from(b.1),
            ..now
        });
        store.refile_detached(entity);
    }
    let slowed = apply_natural_slowdown(entity, ridden);
    entity.set_delta_movement(slowed);
    if let (Some(old), Some(new)) = (
        old_rail_pos,
        get_pos(
            level,
            entity.position().x,
            entity.position().y,
            entity.position().z,
        ),
    ) {
        let slope_speed = (old.y - new.y) * 0.05;
        let movement = entity.delta_movement();
        let horizontal = movement.horizontal_distance_sqr().sqrt();
        if horizontal > 0.0 {
            let factor = (horizontal + slope_speed) / horizontal;
            entity.set_delta_movement(movement.multiply(factor, 1.0, factor));
        }
        let position = entity.position();
        entity.set_pos(Vec3d {
            y: new.y,
            ..position
        });
        store.refile_detached(entity);
    }
    let now = entity.position();
    let xn = now.x.floor() as i32;
    let zn = now.z.floor() as i32;
    if xn != pos.x || zn != pos.z {
        let movement = entity.delta_movement();
        let horizontal = movement.horizontal_distance_sqr().sqrt();
        entity.set_delta_movement(Vec3d {
            x: horizontal * f64::from(xn - pos.x),
            y: movement.y,
            z: horizontal * f64::from(zn - pos.z),
        });
    }
    if powered {
        let movement = entity.delta_movement();
        let speed = movement.horizontal_distance_sqr().sqrt();
        if speed > 0.01 {
            entity.set_delta_movement(Vec3d {
                x: movement.x + movement.x / speed * 0.06,
                y: movement.y,
                z: movement.z + movement.z / speed * 0.06,
            });
        } else {
            let mut updated = movement;
            let content_registry = level.content_registry();
            if shape == RailShape::EastWest {
                if crate::redstone::is_redstone_conductor_with_registry(
                    level.block_state(offset(pos, -1, 0, 0)),
                    content_registry,
                ) {
                    updated.x = 0.02;
                } else if crate::redstone::is_redstone_conductor_with_registry(
                    level.block_state(offset(pos, 1, 0, 0)),
                    content_registry,
                ) {
                    updated.x = -0.02;
                }
            } else if shape == RailShape::NorthSouth {
                if crate::redstone::is_redstone_conductor_with_registry(
                    level.block_state(offset(pos, 0, 0, -1)),
                    content_registry,
                ) {
                    updated.z = 0.02;
                } else if crate::redstone::is_redstone_conductor_with_registry(
                    level.block_state(offset(pos, 0, 0, 1)),
                    content_registry,
                ) {
                    updated.z = -0.02;
                }
            }
            entity.set_delta_movement(updated);
        }
    }
    true
}

fn max_speed(entity: &Entity) -> f64 {
    let base = if entity.was_touching_water() {
        0.2
    } else {
        0.4
    };
    if matches!(entity.data(), EntityData::FurnaceMinecart(_)) {
        base * if entity.was_touching_water() {
            0.75
        } else {
            0.5
        }
    } else {
        base
    }
}

fn apply_natural_slowdown(entity: &mut Entity, ridden: bool) -> Vec3d {
    let movement = entity.delta_movement();
    if let EntityData::FurnaceMinecart(mut data) = *entity.data() {
        let mut slowed = if data.push.horizontal_distance_sqr() > 1.0e-7 {
            if data.push.horizontal_distance_sqr() > 1.0e-4
                && movement.horizontal_distance_sqr() > 0.001
            {
                let dot = data.push.x * movement.x + data.push.z * movement.z;
                let projected = movement.multiply(
                    dot / movement.horizontal_distance_sqr(),
                    0.0,
                    dot / movement.horizontal_distance_sqr(),
                );
                data.push = projected.normalize().scale(data.push.length_sqr().sqrt());
            }
            movement.multiply(0.8, 0.0, 0.8).plus(data.push)
        } else {
            movement.multiply(0.98, 0.0, 0.98)
        };
        if entity.was_touching_water() {
            slowed = slowed.scale(0.1);
        }
        entity.data = EntityData::FurnaceMinecart(data);
        return slowed.multiply(
            0.96 * if entity.was_touching_water() {
                0.95
            } else {
                1.0
            },
            0.0,
            0.96 * if entity.was_touching_water() {
                0.95
            } else {
                1.0
            },
        );
    }
    if let Some(container) = crate::entity::minecart_container::minecart_container(entity.data()) {
        container.apply_natural_slowdown(movement, entity.was_touching_water())
    } else {
        let water_scale = if entity.was_touching_water() {
            0.95
        } else {
            1.0
        };
        // `OldMinecartBehavior.getSlowdownFactor` (`OldMinecartBehavior.java:412-414`).
        let factor = if ridden { 0.997 } else { 0.96 } * water_scale;
        movement.multiply(factor, 0.0, factor)
    }
}

fn come_off_track(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Result<Option<f64>, ()> {
    let max_speed = max_speed(entity);
    let movement = entity.delta_movement();
    entity.set_delta_movement(Vec3d {
        x: movement.x.clamp(-max_speed, max_speed),
        y: movement.y,
        z: movement.z.clamp(-max_speed, max_speed),
    });
    if entity.on_ground() {
        entity.set_delta_movement(entity.delta_movement().multiply(0.5, 0.5, 0.5));
    }
    let fall_distance = move_entity(
        entity,
        store,
        level,
        MoverType::Self_,
        entity.delta_movement(),
    );
    let current = entity.position();
    if !crate::entity::apply_nonliving_inside_block_effects_between(
        entity,
        level,
        Some(&mut *store),
        current,
        current,
    ) {
        return Err(());
    }
    entity.clear_movement_this_tick();
    if !entity.on_ground() {
        entity.set_delta_movement(entity.delta_movement().multiply(0.95, 0.95, 0.95));
    }
    Ok(fall_distance)
}

fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let old_position = entity.position();
    let old_y_rot = entity.y_rot();
    let mut data = data_of(entity);
    if data.hurt_time > 0 {
        data.hurt_time -= 1;
    }
    if data.damage > 0.0 {
        data.damage -= 1.0;
    }
    store_data(entity, data);
    check_below_world(entity, level);
    if !handle_portal(entity, level) {
        return false;
    }
    if entity.removal_reason().is_some() {
        return true;
    }
    let rail_pos = current_rail_pos(entity, level);
    let rail_state = level.block_state(rail_pos);
    let on_rails = is_rail_state(level, rail_state);
    let gravity = entity.gravity();
    if gravity != 0.0 {
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            y: movement.y - gravity,
            ..movement
        });
    }
    data = data_of(entity);
    data.on_rails = on_rails;
    store_data(entity, data);
    let initial_horizontal_speed_sqr = entity.delta_movement().horizontal_distance_sqr();
    let fall_distance = if on_rails {
        if !move_along_track(entity, store, level) {
            return false;
        }
        if block_type(rail_state) == Some(VANILLA_BLOCK_ACTIVATOR_RAIL_ID) {
            let powered = bbb_server_types::block_queries::bool_property(rail_state, "powered")
                .unwrap_or(false);
            activate_minecart(entity, store, level, powered);
        }
        None
    } else {
        let Ok(fall_distance) = come_off_track(entity, store, level) else {
            return false;
        };
        fall_distance
    };
    if let Some(fall_distance) = fall_distance {
        let decision = match entity.data() {
            EntityData::TntMinecart(data) => {
                data.fall_decision(level.tnt_explodes(), fall_distance)
            }
            _ => crate::entity::TntMinecartExplosionDecision::None,
        };
        apply_tnt_explosion_decision(entity, store, decision);
        if !entity.is_alive() {
            return true;
        }
    }
    let current = entity.position();
    if !crate::entity::apply_nonliving_inside_block_effects_between(
        entity,
        level,
        Some(&mut *store),
        current,
        current,
    ) {
        return false;
    }
    entity.clear_movement_this_tick();
    let x_diff = old_position.x - entity.position().x;
    let z_diff = old_position.z - entity.position().z;
    if x_diff * x_diff + z_diff * z_diff > 0.001 {
        let mut y_rot = (z_diff.atan2(x_diff) * 180.0 / std::f64::consts::PI) as f32;
        if data_of(entity).flipped {
            y_rot += 180.0;
        }
        entity.set_rot(y_rot, 0.0);
    }
    let rotation_diff = wrap_degrees(entity.y_rot() - old_y_rot);
    if !(-170.0..170.0).contains(&rotation_diff) {
        entity.set_rot(entity.y_rot() + 180.0, 0.0);
        data = data_of(entity);
        data.flipped = !data.flipped;
        store_data(entity, data);
    }
    entity.set_rot(entity.y_rot() % 360.0, entity.x_rot() % 360.0);
    if level.minecart_improvements_enabled() {
        record_minecart_interpolation_step(
            entity,
            old_position,
            initial_horizontal_speed_sqr,
            max_speed(entity),
        );
    }
    if crate::entity::push::minecart_push_and_pickup(entity, store, level).is_none() {
        return false;
    }
    if crate::entity::environment::update_fluid_interaction_in_level(entity, store, level).is_none()
    {
        return false;
    }
    data = data_of(entity);
    data.first_tick = false;
    store_data(entity, data);
    true
}

/// Records one `MinecartStep` (`NewMinecartBehavior.java:267-293`) when
/// `minecraft:minecart_improvements` is active.
fn record_minecart_interpolation_step(
    entity: &mut Entity,
    old_position: Vec3d,
    initial_horizontal_speed_sqr: f64,
    max_speed: f64,
) {
    entity.enable_minecart_track_interpolation();
    let step_position = entity.position();
    let step_delta = Vec3d {
        x: step_position.x - old_position.x,
        y: step_position.y - old_position.y,
        z: step_position.z - old_position.z,
    };
    let step_length = step_delta.length_sqr().sqrt();
    if step_length > 1.0e-5 {
        let on_rails = data_of(entity).on_rails;
        let flipped = data_of(entity).flipped;
        if step_delta.horizontal_distance_sqr() > 1.0e-5 {
            let mut y_rot =
                180.0 - (step_delta.z.atan2(step_delta.x) * 180.0 / std::f64::consts::PI) as f32;
            let mut x_rot = if entity.on_ground() && !on_rails {
                0.0
            } else {
                90.0 - (step_delta
                    .horizontal_distance_sqr()
                    .sqrt()
                    .atan2(step_delta.y)
                    * 180.0
                    / std::f64::consts::PI) as f32
            };
            if flipped {
                y_rot += 180.0;
                x_rot = -x_rot;
            }
            entity.set_rot(y_rot % 360.0, x_rot % 360.0);
        } else if !on_rails {
            let x_rot = if entity.on_ground() {
                0.0
            } else {
                entity.x_rot() + 0.2 * wrap_degrees(0.0 - entity.x_rot())
            };
            entity.set_rot(entity.y_rot(), x_rot);
        }
        entity.push_minecart_lerp_step(bbb_protocol::packets::MinecartStep {
            position: step_position,
            movement: entity.delta_movement(),
            y_rot: entity.y_rot(),
            x_rot: entity.x_rot(),
            weight: step_length.min(max_speed) as f32,
        });
    } else if initial_horizontal_speed_sqr > 0.0 {
        entity.push_minecart_lerp_step(bbb_protocol::packets::MinecartStep {
            position: step_position,
            movement: entity.delta_movement(),
            y_rot: entity.y_rot(),
            x_rot: entity.x_rot(),
            weight: 1.0,
        });
    }
}

pub(crate) fn tick_stored(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if !tick(entity, store, level) {
        return false;
    }
    if !entity.is_alive() {
        return true;
    }
    match *entity.data() {
        EntityData::FurnaceMinecart(mut furnace) => {
            if furnace.fuel > 0 {
                furnace.fuel -= 1;
            }
            if furnace.fuel <= 0 {
                furnace.push = Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                };
            }
            let has_fuel = furnace.fuel > 0;
            if furnace.has_fuel != has_fuel {
                entity.mark_synched_dirty(VANILLA_FURNACE_MINECART_FUEL_DATA_ID);
            }
            furnace.has_fuel = has_fuel;
            entity.data = EntityData::FurnaceMinecart(furnace);
        }
        EntityData::TntMinecart(_) => tick_tnt_minecart(entity, store, level),
        EntityData::SpawnerMinecart(_) => tick_spawner_minecart(entity, store, level),
        EntityData::HopperMinecart(_) => {
            crate::entity::minecart_hopper::try_consume_items(entity, store);
        }
        _ => {}
    }
    true
}

fn tick_spawner_minecart(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let block_pos = entity.block_position();
    let (required_player_range, entity_id, entity_uuid) = match entity.data() {
        EntityData::SpawnerMinecart(data) => {
            (data.required_player_range(), entity.id(), entity.uuid())
        }
        _ => return,
    };
    let centre = Vec3d {
        x: f64::from(block_pos.x) + 0.5,
        y: f64::from(block_pos.y) + 0.5,
        z: f64::from(block_pos.z) + 0.5,
    };
    let near_alive_player = level.has_nearby_alive_player(centre, f64::from(required_player_range));
    let spawner_enabled = level.spawner_blocks_work();
    let decision = {
        let EntityData::SpawnerMinecart(data) = &mut entity.data else {
            return;
        };
        data.server_tick_with_registry(
            block_pos,
            near_alive_player,
            spawner_enabled,
            level.level_random(),
            level.content_registry(),
        )
    };
    apply_spawner_tick_decision(entity_id, entity_uuid, decision, store);
}

pub fn apply_spawner_tick_decision(
    minecart_entity_id: i32,
    minecart_uuid: uuid::Uuid,
    decision: crate::entity::SpawnerTickDecision,
    store: &mut EntityStore,
) {
    let broadcast_event = match &decision {
        crate::entity::SpawnerTickDecision::Countdown {
            broadcast_event, ..
        }
        | crate::entity::SpawnerTickDecision::Finished {
            broadcast_event, ..
        } => *broadcast_event,
        crate::entity::SpawnerTickDecision::Dormant
        | crate::entity::SpawnerTickDecision::Attempt(_) => None,
    };
    if let Some(event_id) = broadcast_event {
        store.broadcast_to_tracking(
            minecart_entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: minecart_entity_id,
                event_id: event_id as i8,
            }),
        );
    }
    if let crate::entity::SpawnerTickDecision::Attempt(attempt) = decision {
        store.queue_command(crate::entity::EntityCommand::SpawnerMinecart(
            crate::entity::PendingSpawnerMinecartAttempt {
                minecart_entity_id,
                minecart_uuid,
                attempt,
            },
        ));
    }
}

pub fn resolve_spawner_minecart_attempt(
    store: &mut EntityStore,
    pending: &crate::entity::PendingSpawnerMinecartAttempt,
    result: Result<(), crate::entity::SpawnerRefusal>,
    random: &GameRandom,
) -> crate::entity::SpawnerTickDecision {
    let plugins = std::sync::Arc::clone(store.plugins());
    let Some(entity) = store.get_mut(pending.minecart_entity_id) else {
        return crate::entity::SpawnerTickDecision::Dormant;
    };
    if entity.uuid() != pending.minecart_uuid {
        return crate::entity::SpawnerTickDecision::Dormant;
    }
    let EntityData::SpawnerMinecart(data) = &mut entity.data else {
        return crate::entity::SpawnerTickDecision::Dormant;
    };
    data.resolve_attempt_with_registry(
        pending.attempt.request.spawner_position,
        result,
        random,
        plugins.content_registry(),
    )
}

fn activate_minecart(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    powered: bool,
) {
    if matches!(entity.data(), EntityData::Minecart(_)) {
        if powered {
            // `Minecart.activateMinecart` (Minecart.java:49-53).
            if !store.passengers(entity.id()).is_empty() {
                store.eject_body_passengers_reverse(entity);
            }
            let mut data = data_of(entity);
            if data.hurt_time == 0 {
                data.hurt_dir = -data.hurt_dir;
                data.hurt_time = 10;
                data.damage = 50.0;
                store_data(entity, data);
                entity.mark_hurt();
            }
        }
        return;
    }
    if let EntityData::HopperMinecart(data) = &mut entity.data {
        data.set_enabled(!powered);
        return;
    }
    let tick_count = entity.tick_count();
    let pending = match &mut entity.data {
        EntityData::CommandBlockMinecart(data) => {
            match data.activate_minecart(tick_count, powered) {
                crate::entity::ActivatorRailDecision::NoExecution => None,
                crate::entity::ActivatorRailDecision::Execute(snapshot) => {
                    Some(crate::entity::PendingCommandMinecartExecution {
                        entity_id: entity.id(),
                        entity_uuid: entity.uuid(),
                        position: entity.position(),
                        y_rot: entity.y_rot(),
                        x_rot: entity.x_rot(),
                        command_blocks_work: level.command_blocks_work(),
                        snapshot,
                    })
                }
            }
        }
        _ => None,
    };
    if let Some(pending) = pending {
        store.queue_command(crate::entity::EntityCommand::CommandMinecart(pending));
        return;
    }
    let silent = entity.is_silent();
    let effects = match &mut entity.data {
        EntityData::TntMinecart(data) => data.activator_rail(powered, level.tnt_explodes(), silent),
        _ => None,
    };
    if let Some(effects) = effects {
        apply_tnt_prime_effects(entity, store, effects);
    }
}

fn tick_tnt_minecart(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    if !entity.is_alive() {
        return;
    }
    let speed_squared = entity.delta_movement().horizontal_distance_sqr();
    let effects = match &mut entity.data {
        EntityData::TntMinecart(data) => data.tick_fuse(level.tnt_explodes(), speed_squared),
        _ => return,
    };
    apply_tnt_explosion_decision(entity, store, effects.explosion);
    if !entity.is_alive() {
        return;
    }
    if entity.horizontal_collision() {
        let speed_squared = entity.delta_movement().horizontal_distance_sqr();
        let decision = match entity.data() {
            EntityData::TntMinecart(data) => {
                data.horizontal_collision_decision(level.tnt_explodes(), speed_squared)
            }
            _ => return,
        };
        apply_tnt_explosion_decision(entity, store, decision);
    }
}

fn apply_tnt_prime_effects(
    entity: &Entity,
    store: &mut EntityStore,
    effects: crate::entity::TntMinecartPrimeEffects,
) {
    let crate::entity::TntMinecartPrimeEffects::Primed { event, play_sound } = effects else {
        return;
    };
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: event,
        }),
    );
    if play_sound {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.tnt.primed".to_string(),
            source: SoundSource::Blocks,
            volume: 1.0,
            pitch: 1.0,
        });
    }
}

fn apply_tnt_explosion_decision(
    entity: &mut Entity,
    store: &mut EntityStore,
    decision: crate::entity::TntMinecartExplosionDecision,
) {
    let crate::entity::TntMinecartExplosionDecision::Explode {
        speed_squared,
        ignition_source,
    } = decision
    else {
        if decision == crate::entity::TntMinecartExplosionDecision::Discard {
            entity.discard();
        }
        return;
    };
    let data = match entity.data() {
        EntityData::TntMinecart(data) => data.clone(),
        _ => return,
    };
    let indirect =
        ignition_source.and_then(crate::entity::TntMinecartIgnitionSource::into_causing_entity);
    let direct = crate::entity::damage::explosion_source_entity(entity);
    let radius = data.explosion_radius(speed_squared, store.random().next_double());
    store.queue_command(crate::entity::EntityCommand::Explosion(PendingExplosion {
        source: Some(ExplosionSource {
            direct,
            indirect: indirect.clone(),
            causing: indirect,
            in_water: entity.was_touching_water(),
            wind_charge: false,
        }),
        center: entity.position(),
        radius,
        fire: false,
        interaction: ExplosionInteraction::Tnt,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::TntMinecart {
            primed: data.is_primed(),
        },
        presentation: ExplosionPresentation::Default,
    }));
    entity.discard();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinecartHurtSource {
    pub creative_player: bool,
    pub bypasses_invulnerability: bool,
    pub is_fire: bool,
    pub fire_immune: bool,
    pub is_fall: bool,
    pub fall_damage_immune: bool,
    pub causing_entity_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinecartHurtEffects {
    pub applied: bool,
    pub succeeded: bool,
    pub emits_entity_damage: bool,
    pub emits_entity_die: bool,
    pub game_event_source_id: Option<i32>,
    pub drops: Vec<RuntimeItemStack>,
}

impl MinecartHurtEffects {
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

pub fn hurt_server(
    entity: &mut Entity,
    damage: f32,
    source: MinecartHurtSource,
    entity_drops: bool,
) -> MinecartHurtEffects {
    if entity.removal_reason().is_some() {
        return MinecartHurtEffects::refused(true);
    }
    if entity.invulnerable() && !source.bypasses_invulnerability && !source.creative_player
        || source.is_fire && source.fire_immune
        || source.is_fall && source.fall_damage_immune
    {
        return MinecartHurtEffects::refused(false);
    }
    let mut data = data_of(entity);
    data.hurt_dir = -data.hurt_dir;
    data.hurt_time = 10;
    data.damage += damage * 10.0;
    store_data(entity, data);
    entity.mark_hurt();
    let mut drops = Vec::new();
    let mut destroyed = false;
    if source.creative_player {
        entity.discard();
    } else if data.damage > DAMAGE_TO_DESTROY {
        destroyed = true;
        entity.set_removed(RemovalReason::Killed);
        if entity_drops {
            let resource_id = match entity.data() {
                EntityData::FurnaceMinecart(_) => "minecraft:furnace_minecart",
                EntityData::TntMinecart(_) => "minecraft:tnt_minecart",
                data => crate::entity::minecart_container::minecart_container(data)
                    .map_or("minecraft:minecart", |container| {
                        container.kind().vehicle_item_resource_id()
                    }),
            };
            drops.extend(plain_stack(resource_id, 1));
        }
    }
    MinecartHurtEffects {
        applied: true,
        succeeded: true,
        emits_entity_damage: true,
        emits_entity_die: destroyed,
        game_event_source_id: source.causing_entity_id,
        drops,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn hurt_tnt_server(
    entity: &mut Entity,
    store: &mut EntityStore,
    damage: f32,
    source: MinecartHurtSource,
    tnt_source: crate::entity::TntMinecartDamageSource,
    burning_arrow_speed_squared: Option<f64>,
    tnt_explodes: bool,
    entity_drops: bool,
) -> MinecartHurtEffects {
    if let Some(speed_squared) = burning_arrow_speed_squared {
        let decision = match entity.data() {
            EntityData::TntMinecart(data) => data.burning_arrow_decision(
                tnt_explodes,
                speed_squared,
                tnt_source.ignition_source.clone(),
            ),
            _ => crate::entity::TntMinecartExplosionDecision::None,
        };
        apply_tnt_explosion_decision(entity, store, decision);
    }
    if entity.removal_reason().is_some() {
        return MinecartHurtEffects::refused(true);
    }
    if entity.invulnerable() && !source.bypasses_invulnerability && !source.creative_player
        || source.is_fire && source.fire_immune
        || source.is_fall && source.fall_damage_immune
    {
        return MinecartHurtEffects::refused(false);
    }

    let mut minecart = data_of(entity);
    minecart.hurt_dir = -minecart.hurt_dir;
    minecart.hurt_time = 10;
    minecart.damage += damage * 10.0;
    store_data(entity, minecart);
    entity.mark_hurt();

    let should_source_destroy = tnt_source.ignites_tnt();
    let mut drops = Vec::new();
    let mut destroyed = false;
    if (source.creative_player || minecart.damage <= DAMAGE_TO_DESTROY) && !should_source_destroy {
        if source.creative_player {
            entity.discard();
        }
    } else {
        let speed_squared = entity.delta_movement().horizontal_distance_sqr();
        let needs_fuse_draws = matches!(entity.data(), EntityData::TntMinecart(data) if !data.is_primed())
            && (should_source_destroy
                || speed_squared >= crate::entity::minecart_tnt::MIN_COLLISION_SPEED_SQUARED);
        let (first_fuse_draw, second_fuse_draw) = if needs_fuse_draws {
            (store.random().next_int(20), store.random().next_int(20))
        } else {
            (0, 0)
        };
        let silent = entity.is_silent();
        let decision = match &mut entity.data {
            EntityData::TntMinecart(data) => data.destroy_decision(
                tnt_explodes,
                tnt_source,
                speed_squared,
                first_fuse_draw,
                second_fuse_draw,
                silent,
            ),
            _ => unreachable!("TNT minecart damage received a non-TNT minecart"),
        };
        match decision {
            crate::entity::TntMinecartDestroyDecision::DropTntMinecart => {
                destroyed = true;
                entity.set_removed(RemovalReason::Killed);
                if entity_drops {
                    drops.extend(plain_stack("minecraft:tnt_minecart", 1));
                }
            }
            crate::entity::TntMinecartDestroyDecision::ArmShortFuse { prime } => {
                apply_tnt_prime_effects(entity, store, prime);
            }
            crate::entity::TntMinecartDestroyDecision::Keep => {}
        }
    }

    MinecartHurtEffects {
        applied: true,
        succeeded: true,
        emits_entity_damage: true,
        emits_entity_die: destroyed,
        game_event_source_id: source.causing_entity_id,
        drops,
    }
}

pub fn synched_values(entity: &Entity, data: &MinecartData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    rows.extend([
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_VEHICLE_HURT_TIME_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(data.hurt_time),
            },
            is_default: data.hurt_time == 0,
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_VEHICLE_HURT_DIR_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(data.hurt_dir),
            },
            is_default: data.hurt_dir == 1,
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_VEHICLE_DAMAGE_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(data.damage),
            },
            is_default: data.damage == 0.0,
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                value: EntityDataValueKind::OptionalBlockState(data.custom_display_block),
            },
            is_default: data.custom_display_block.is_none(),
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(data.display_offset),
            },
            is_default: data.display_offset == data.default_display_offset,
        },
    ]);
    rows
}

pub(crate) fn furnace_synched_values(
    entity: &Entity,
    data: &FurnaceMinecartData,
) -> Vec<SynchedValue> {
    let mut rows = synched_values(entity, &data.minecart);
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_FURNACE_MINECART_FUEL_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(data.has_fuel),
        },
        is_default: !data.has_fuel,
    });
    rows
}

fn block_state_value(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue, EntitySaveError> {
    crate::entity::save::block_state_value_with_registry(
        block_state_id,
        "DisplayState",
        content_registry,
    )
}

pub fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &MinecartData,
) -> Result<(), EntitySaveError> {
    write_save_data_with_registry(
        entries,
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
    )
}

pub(crate) fn write_save_data_with_registry(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &MinecartData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(), EntitySaveError> {
    if let Some(state) = data.custom_display_block {
        entries.push(NbtSummaryEntry {
            name: "DisplayState".into(),
            value: block_state_value(state, content_registry)?,
        });
    }
    if data.display_offset != data.default_display_offset {
        entries.push(NbtSummaryEntry {
            name: "DisplayOffset".into(),
            value: NbtSummaryValue::Int(data.display_offset),
        });
    }
    entries.extend([
        NbtSummaryEntry {
            name: "FlippedRotation".into(),
            value: NbtSummaryValue::Byte(i8::from(data.flipped)),
        },
        NbtSummaryEntry {
            name: "HasTicked".into(),
            value: NbtSummaryValue::Byte(i8::from(data.first_tick)),
        },
    ]);
    Ok(())
}

pub(crate) fn write_furnace_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &FurnaceMinecartData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(), EntitySaveError> {
    write_save_data_with_registry(entries, &data.minecart, content_registry)?;
    entries.extend([
        NbtSummaryEntry {
            name: "PushX".into(),
            value: NbtSummaryValue::Double(data.push.x.to_bits()),
        },
        NbtSummaryEntry {
            name: "PushZ".into(),
            value: NbtSummaryValue::Double(data.push.z.to_bits()),
        },
        NbtSummaryEntry {
            name: "Fuel".into(),
            value: NbtSummaryValue::Short(data.fuel as i16),
        },
    ]);
    Ok(())
}

pub fn read_save_data(entries: &[NbtSummaryEntry]) -> MinecartData {
    read_save_data_with_default_offset(entries, DEFAULT_DISPLAY_OFFSET)
}

pub(crate) fn read_save_data_with_registry(
    entries: &[NbtSummaryEntry],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> MinecartData {
    read_save_data_with_default_offset_and_registry(
        entries,
        DEFAULT_DISPLAY_OFFSET,
        content_registry,
    )
}

pub(crate) fn read_furnace_save_data(
    entries: &[NbtSummaryEntry],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> FurnaceMinecartData {
    use crate::entity::save::{optional_double, optional_short};
    FurnaceMinecartData::from_saved(
        read_save_data_with_registry(entries, content_registry),
        i32::from(optional_short(entries, "Fuel").unwrap_or(0)),
        Vec3d {
            x: optional_double(entries, "PushX").unwrap_or(0.0),
            y: 0.0,
            z: optional_double(entries, "PushZ").unwrap_or(0.0),
        },
    )
}

pub(crate) fn read_save_data_with_default_offset(
    entries: &[NbtSummaryEntry],
    default_display_offset: i32,
) -> MinecartData {
    read_save_data_with_default_offset_and_registry(
        entries,
        default_display_offset,
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
    )
}

pub(crate) fn read_save_data_with_default_offset_and_registry(
    entries: &[NbtSummaryEntry],
    default_display_offset: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> MinecartData {
    use crate::entity::save::{optional_bool, optional_int};
    let mut data = MinecartData::with_default_display_offset(default_display_offset);
    data.custom_display_block = field(entries, "DisplayState").and_then(|value| {
        crate::entity::save::read_block_state_from_value_with_registry(value, content_registry)
    });
    data.display_offset = optional_int(entries, "DisplayOffset").unwrap_or(default_display_offset);
    data.flipped = optional_bool(entries, "FlippedRotation").unwrap_or(false);
    data.first_tick = optional_bool(entries, "HasTicked").unwrap_or(false);
    data
}
