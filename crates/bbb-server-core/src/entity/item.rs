//! Dropped item movement, merging, pickup and persistent stack state.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::VANILLA_ITEM_ENTITY_ITEM_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::environment::FluidTag;
use crate::entity::store::EntityStore;
use crate::entity::{apply_gravity, Entity, EntityData, MoverType, RemovalReason};
use bbb_server_types::geometry::{no_block_collision_with_scratch, Aabb, Axis, BoxShape, Vec3Ops};

/// `ItemEntity.getDefaultGravity()` (`ItemEntity.java:112-114`).
pub const DEFAULT_GRAVITY: f64 = 0.04;

/// `ItemEntity.LIFETIME` (`ItemEntity.java:42`): the age at which an item on
/// the ground vanishes, five minutes at twenty ticks a second.
pub const LIFETIME: i32 = 6000;

/// `ItemEntity.INFINITE_PICKUP_DELAY` (`ItemEntity.java:43`): the one pickup
/// delay that never counts down, which is `Short.MAX_VALUE` so it survives the
/// narrowing `addAdditionalSaveData` performs (`:317`).
pub const INFINITE_PICKUP_DELAY: i32 = 32767;

/// `ItemEntity.INFINITE_LIFETIME` (`ItemEntity.java:44`): the one age that
/// never increments, `Short.MIN_VALUE` for the same reason.
pub const INFINITE_LIFETIME: i32 = -32768;

/// `ItemEntity.DEFAULT_HEALTH` (`ItemEntity.java:45`), which is both the field
/// initializer (`:50`) and the `readAdditionalSaveData` default (`:327`).
pub const DEFAULT_HEALTH: i32 = 5;

/// `ItemEntity.setDefaultPickUpDelay()` (`ItemEntity.java:398-400`), the delay
/// `Entity.spawnAtLocation` gives every drop (`Entity.java:2137`).
pub const DEFAULT_PICKUP_DELAY: i32 = 10;

/// `entity.setPickUpDelay(40)` (`LivingEntity.java:3379`), the delay a stack
/// dropped out of a player's inventory gets instead.
pub const DROP_PICKUP_DELAY: i32 = 40;

/// `ItemEntity.setExtendedLifetime()` (`ItemEntity.java:422-424`). Vanilla
/// writes it as its own literal rather than as `-LIFETIME`, so it is its own
/// constant here too.
pub const EXTENDED_LIFETIME_AGE: i32 = -6000;

/// The drag `ItemEntity.tick` applies to a moving item, in the two precisions
/// vanilla writes it in.
///
/// `friction` is a `float` local initialised to `0.98F` and multiplied by the
/// block's own `float` friction (`ItemEntity.java:152-155`), and it is only
/// widened when `Vec3.multiply(double, double, double)` takes it at `:157`;
/// the vertical argument of that same call is the `double` literal `0.98`. The
/// two are therefore *not* the same number -- `(double)0.98F` is
/// `0.9800000190734863` -- so a horizontal component and a vertical one drag at
/// measurably different rates, and the split is transcribed rather than tidied.
const HORIZONTAL_DRAG: f32 = 0.98;
const VERTICAL_DRAG: f64 = 0.98;

/// The vertical bounce a landed item keeps (`ItemEntity.java:160-162`): half
/// the downward speed, upward.
const LANDED_VERTICAL_SCALE: f64 = -0.5;

/// `!(this.getDeltaMovement().horizontalDistanceSqr() > 1.0E-5F)`
/// (`ItemEntity.java:147`), the speed below which a grounded item counts as at
/// rest.
///
/// The literal is a `float`, so the comparison widens `9.999999747378752E-6`
/// rather than `1.0E-5`.
const RESTING_HORIZONTAL_SPEED_SQR: f64 = 1.0e-5_f32 as f64;

/// `(this.tickCount + this.getId()) % 4 != 0` (`ItemEntity.java:147`): a
/// resting item still runs the whole move once every four ticks, staggered by
/// its network id so a pile of them does not all move on the same tick.
const RESTING_MOVE_STRIDE: i32 = 4;

/// `int rate = moved ? 2 : 40` (`ItemEntity.java:169`): how often an item looks
/// for neighbours to merge with, in ticks.
pub const MERGE_RATE_MOVED: i32 = 2;
pub const MERGE_RATE_STILL: i32 = 40;

/// `this.getBoundingBox().inflate(0.5, 0.0, 0.5)` (`ItemEntity.java:213`): the
/// search volume `mergeWithNeighbours` looks in, half a block wider
/// horizontally and not one bit taller.
const MERGE_SEARCH_INFLATION: [f64; 3] = [0.5, 0.0, 0.5];

/// `merge(toStack, fromStack, 64)` (`ItemEntity.java:255`): the ceiling the
/// entity-to-entity merge imposes on top of the stack's own maximum, which
/// only bites for an item whose `max_stack_size` component raises it above 64.
const MERGE_STACK_LIMIT: i32 = 64;

/// `ItemEntity.getBlockPosBelowThatAffectsMyMovement()`
/// (`ItemEntity.java:193-195`): `getOnPos(0.999999F)` rather than `Entity`'s
/// own `getOnPos(0.500001F)` (`Entity.java:988-990`), so an item reads the
/// friction of the block a whole block below its feet.
pub const BLOCK_POS_BELOW_OFFSET: f32 = 0.999_999;

#[derive(Debug, Clone)]
pub struct ItemEntityData {
    item: RuntimeItemStack,
    age: i32,
    pickup_delay: i32,
    health: i32,
    thrower: Option<crate::entity::entity_reference::EntityReference>,
    target: Option<Uuid>,
    bob_offs: f32,
}

impl PartialEq for ItemEntityData {
    fn eq(&self, other: &Self) -> bool {
        *self.item() == *other.item()
            && self.age == other.age
            && self.pickup_delay == other.pickup_delay
            && self.health == other.health
            && self.thrower == other.thrower
            && self.target == other.target
            && self.bob_offs == other.bob_offs
    }
}

impl ItemEntityData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.bob_offs = previous.bob_offs;
    }

    pub fn new(item: RuntimeItemStack, bob_offs: f32) -> Self {
        Self {
            item,
            age: 0,
            pickup_delay: 0,
            health: DEFAULT_HEALTH,
            thrower: None,
            target: None,
            bob_offs,
        }
    }

    pub fn with_saved_fields(
        mut self,
        health: i32,
        age: i32,
        pickup_delay: i32,
        thrower: Option<Uuid>,
        target: Option<Uuid>,
    ) -> Self {
        self.health = health;
        self.age = age;
        self.pickup_delay = pickup_delay;
        self.thrower = thrower.map(crate::entity::entity_reference::EntityReference::Uuid);
        self.target = target;
        self
    }

    pub fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub fn stack(&self) -> &RuntimeItemStack {
        &self.item
    }

    /// `ItemEntity.getAge()` (`ItemEntity.java:394-396`).
    pub const fn age(&self) -> i32 {
        self.age
    }

    pub const fn pickup_delay(&self) -> i32 {
        self.pickup_delay
    }

    pub const fn health(&self) -> i32 {
        self.health
    }

    pub fn set_health(&mut self, health: i32) {
        self.health = health;
    }

    /// The UUID behind `ItemEntity.thrower` (`ItemEntity.java:51`).
    pub const fn thrower(&self) -> Option<Uuid> {
        match self.thrower {
            Some(reference) => reference.uuid(),
            None => None,
        }
    }

    pub const fn thrower_reference(
        &self,
    ) -> Option<crate::entity::entity_reference::EntityReference> {
        self.thrower
    }

    pub(crate) fn set_thrower_reference(
        &mut self,
        reference: Option<crate::entity::entity_reference::EntityReference>,
    ) {
        self.thrower = reference;
    }

    /// `ItemEntity.target` (`ItemEntity.java:52`), the only player allowed to
    /// pick this stack up (`:344`). Written as `Owner` (`:319`).
    pub const fn target(&self) -> Option<Uuid> {
        self.target
    }

    /// `ItemEntity.bobOffs` (`ItemEntity.java:53`), the per-entity phase the
    /// client bobs and spins the stack with (`getSpin`, `:431-433`).
    pub const fn bob_offs(&self) -> f32 {
        self.bob_offs
    }

    pub fn set_item(&mut self, item: RuntimeItemStack) {
        self.item = item;
    }

    pub(crate) fn update_item(&mut self, item: RuntimeItemStack) {
        self.item = item;
    }

    pub(crate) fn update_item_count(&mut self, item: RuntimeItemStack) {
        self.item.set_count(item.count());
    }

    pub fn set_default_pickup_delay(&mut self) {
        self.pickup_delay = DEFAULT_PICKUP_DELAY;
    }

    pub fn set_no_pickup_delay(&mut self) {
        self.pickup_delay = 0;
    }

    pub fn set_never_pick_up(&mut self) {
        self.pickup_delay = INFINITE_PICKUP_DELAY;
    }

    pub fn set_pickup_delay(&mut self, ticks: i32) {
        self.pickup_delay = ticks;
    }

    pub fn set_unlimited_lifetime(&mut self) {
        self.age = INFINITE_LIFETIME;
    }

    pub fn set_extended_lifetime(&mut self) {
        self.age = EXTENDED_LIFETIME_AGE;
    }

    pub fn make_fake_item(&mut self) {
        self.set_never_pick_up();
        self.age = LIFETIME - 1;
    }

    pub fn set_target(&mut self, target: Option<Uuid>) {
        self.target = target;
    }

    pub fn set_thrower(&mut self, thrower: Option<Uuid>) {
        self.thrower = thrower.map(crate::entity::entity_reference::EntityReference::Uuid);
    }

    /// `ItemEntity.hasPickUpDelay()` (`ItemEntity.java:414-416`).
    pub const fn has_pickup_delay(&self) -> bool {
        self.pickup_delay > 0
    }
}

///
///
/// `None` only for a store that already knows the drawn UUID, which cannot
/// happen for a fresh draw and is reported rather than asserted so the caller
/// keeps one shape for every spawn.
pub fn spawn_item_entity(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |_| {})
}

/// The common constructor path for ordinary and default-delay item drops.
/// `initialize` represents the caller-owned fields Vanilla installs between
/// construction and `ServerLevel.addFreshEntity`.
fn spawn_item_entity_initialized<F>(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    initialize: F,
) -> Option<i32>
where
    F: FnOnce(&mut Entity),
{
    let uuid = store.next_uuid();
    let bob_offs = store.random().next_float() * std::f32::consts::PI * 2.0;
    let y_rot = store.random().next_float() * 360.0;
    let delta_x = store.random().next_double() * 0.2 - 0.1;
    let delta_z = store.random().next_double() * 0.2 - 0.1;
    restore_item_entity_initialized(
        store,
        uuid,
        position,
        item,
        bob_offs,
        y_rot,
        Vec3d {
            x: delta_x,
            y: 0.2,
            z: delta_z,
        },
        initialize,
    )
}

#[allow(clippy::too_many_arguments)]
fn restore_item_entity_initialized<F>(
    store: &mut EntityStore,
    uuid: Uuid,
    position: Vec3d,
    item: RuntimeItemStack,
    bob_offs: f32,
    y_rot: f32,
    delta: Vec3d,
    initialize: F,
) -> Option<i32>
where
    F: FnOnce(&mut Entity),
{
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        position,
        EntityData::Item(ItemEntityData::new(item, bob_offs)),
        |entity| {
            entity.set_rot(y_rot, 0.0);
            entity.set_delta_movement(delta);
            initialize(entity);
        },
    )
}

#[derive(Debug)]
pub(crate) struct PlannedContainerDrop {
    uuid: Uuid,
    position: Vec3d,
    item: RuntimeItemStack,
    bob_offs: f32,
    y_rot: f32,
    delta: Vec3d,
}

pub(crate) fn plan_container_drops(
    store: &mut EntityStore,
    position: Vec3d,
    slots: &[Option<RuntimeItemStack>],
) -> Vec<PlannedContainerDrop> {
    let size = f64::from(
        bbb_protocol::entity_types::vanilla_entity_type(VANILLA_ENTITY_TYPE_ITEM_ID)
            .expect("the item entity is a built-in type")
            .width,
    );
    let center_range = 1.0 - size;
    let half_size = size / 2.0;
    let mut planned = Vec::new();
    for stack in slots {
        let drop_position = Vec3d {
            x: position.x.floor() + store.random().next_double() * center_range + half_size,
            y: position.y.floor() + store.random().next_double() * center_range,
            z: position.z.floor() + store.random().next_double() * center_range + half_size,
        };
        let Some(mut original) = stack.clone() else {
            continue;
        };
        let mut remaining = original.count();
        while remaining > 0 {
            let count = remaining.min(store.random().next_int(21) + 10);
            let split = original.split(count);
            let item = split;
            remaining = original.count();
            let uuid = store.next_uuid();
            let bob_offs = store.random().next_float() * std::f32::consts::PI * 2.0;
            let y_rot = store.random().next_float() * 360.0;
            let triangle = |store: &mut EntityStore, mean: f64, deviation: f64| {
                mean + deviation * (store.random().next_double() - store.random().next_double())
            };
            planned.push(PlannedContainerDrop {
                uuid,
                position: drop_position,
                item,
                bob_offs,
                y_rot,
                delta: Vec3d {
                    x: triangle(store, 0.0, 0.114_850_001_711_398_36),
                    y: triangle(store, 0.2, 0.114_850_001_711_398_36),
                    z: triangle(store, 0.0, 0.114_850_001_711_398_36),
                },
            });
        }
    }
    planned
}

pub(crate) fn spawn_planned_container_drops(
    store: &mut EntityStore,
    planned: Vec<PlannedContainerDrop>,
) -> bool {
    let mut accepted = true;
    for drop in planned {
        let id = restore_item_entity_initialized(
            store,
            drop.uuid,
            drop.position,
            drop.item,
            drop.bob_offs,
            drop.y_rot,
            drop.delta,
            |_| {},
        );
        accepted &= id.is_some();
    }
    accepted
}

pub fn spawn_dispensed_item_entity(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    velocity: Vec3d,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        entity.set_delta_movement(velocity);
    })
}

///
/// Both callers set the delay unconditionally, so it is part of this call
/// rather than a flag on it; a spawn that wants another delay uses
/// [`spawn_item_entity`] and says so.
pub fn spawn_dropped_item_entity(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        data_mut(entity).set_default_pickup_delay();
    })
}

/// The constructor's own delta, optionally with the default pickup delay.
pub fn spawn_item_entity_with_pickup_delay(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    default_pickup_delay: bool,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        if default_pickup_delay {
            data_mut(entity).set_default_pickup_delay();
        }
    })
}

/// `spawnAtLocation` followed by `ItemEntity.setExtendedLifetime`.
pub(crate) fn spawn_extended_lifetime_item(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        let data = data_mut(entity);
        data.set_default_pickup_delay();
        data.set_extended_lifetime();
    })
}

pub(crate) fn spawn_thrown_item_entity(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    pickup_delay: i32,
    thrower: Uuid,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        let data = data_mut(entity);
        data.set_pickup_delay(pickup_delay);
        data.set_thrower(Some(thrower));
    })
}

pub fn spawn_advancement_reward_item(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    velocity: Vec3d,
    target: Uuid,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        entity.set_delta_movement(velocity);
        let data = data_mut(entity);
        data.set_no_pickup_delay();
        data.set_target(Some(target));
    })
}

pub fn spawn_fake_command_item(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    velocity: Vec3d,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        entity.set_delta_movement(velocity);
        data_mut(entity).make_fake_item();
    })
}

pub fn spawn_player_dropped_item_entity(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    velocity: Vec3d,
    thrown_from_hand: bool,
    thrower: Uuid,
) -> Option<i32> {
    spawn_item_entity_initialized(store, position, item, |entity| {
        entity.set_delta_movement(velocity);
        let data = data_mut(entity);
        data.set_pickup_delay(DROP_PICKUP_DELAY);
        if thrown_from_hand {
            data.set_thrower(Some(thrower));
        }
    })
}

///
/// The store is threaded in because `mergeWithNeighbours` (`:170-172`) is the
/// one thing in this crate's entity tick that reads and writes *other*
/// entities. The ticking entity itself is detached from the store for the
/// length of its tick (see [`EntityStore::detach`]), which is also what makes
/// vanilla's `other != this` filter (`:213`) unnecessary here.
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    // `:118-119`.
    if data(entity).item.is_empty() {
        entity.discard();
        return true;
    }
    // `:121`.
    if !crate::entity::damage::base_tick_from_environment(entity, store, level) {
        return false;
    }

    // `:122-124`.
    let item_data = data_mut(entity);
    if item_data.pickup_delay > 0 && item_data.pickup_delay != INFINITE_PICKUP_DELAY {
        item_data.pickup_delay -= 1;
    }

    // `:126-128`. `xo`/`yo`/`zo` are written here and read at `:166-168`
    // inside the same tick, so they are locals rather than entity state.
    let old_position = entity.position;
    let old_movement = entity.delta_movement;

    // `ItemEntity.tick` and `setFluidMovement` (`ItemEntity.java:130-136`, `:197-208`).
    let fluid_drag =
        if entity.is_in_water() && entity.fluid_height(FluidTag::Water) > 0.1_f32.into() {
            Some(0.99_f32)
        } else if entity.is_in_lava() && entity.fluid_height(FluidTag::Lava) > 0.1_f32.into() {
            Some(0.95_f32)
        } else {
            None
        };
    if let Some(drag) = fluid_drag {
        let movement = entity.delta_movement;
        entity.delta_movement = Vec3d {
            x: movement.x * f64::from(drag),
            y: movement.y
                + if movement.y < f64::from(0.06_f32) {
                    f64::from(5.0e-4_f32)
                } else {
                    0.0
                },
            z: movement.z * f64::from(drag),
        };
    } else {
        apply_gravity(entity);
    }

    // ItemEntity.java:141-143: embedded drops bypass collision until they escape.
    let Some(clear) = no_block_collision_with_scratch(
        entity.bounding_box().inflate(-1.0e-7, -1.0e-7, -1.0e-7),
        |pos| level.is_moving_piston_at(pos),
        |pos, out| {
            level.block_state_checked(pos)?;
            level.push_block_collision_boxes_for_entity(entity, pos, out)
        },
    ) else {
        return false;
    };
    entity.set_no_physics(!clear);
    if !clear && move_towards_closest_space(entity, store, level).is_none() {
        return false;
    }

    // `:147`.
    let horizontal_speed_sqr = entity.delta_movement.horizontal_distance_sqr();
    let resting = entity.on_ground
        && horizontal_speed_sqr.partial_cmp(&RESTING_HORIZONTAL_SPEED_SQR)
            != Some(std::cmp::Ordering::Greater)
        && (entity.tick_count + entity.id) % RESTING_MOVE_STRIDE != 0;
    if resting {
        // `:148`.
        if !crate::entity::apply_nonliving_inside_block_effects_for_last_movements(
            entity,
            level,
            Some(&mut *store),
        ) {
            return false;
        }
    } else {
        // `:150`.
        let delta = entity.delta_movement;
        if crate::entity::movement::move_entity_with_ground_contact(
            entity,
            store,
            level,
            MoverType::Self_,
            delta,
        )
        .is_err()
        {
            return false;
        }
        // `:151`.
        if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
            return false;
        }

        // `:152-155`. The block position is `ItemEntity`'s own override, a
        // whole block below the feet rather than half of one.
        let mut friction = HORIZONTAL_DRAG;
        if entity.on_ground {
            friction = level.block_friction(entity.block_pos_below_that_affects_my_movement())
                * HORIZONTAL_DRAG;
        }
        // `:157`.
        let friction = f64::from(friction);
        entity.delta_movement = entity
            .delta_movement
            .multiply(friction, VERTICAL_DRAG, friction);

        // `:158-163`.
        if entity.on_ground {
            let movement = entity.delta_movement;
            if movement.y < 0.0 {
                entity.delta_movement = movement.multiply(1.0, LANDED_VERTICAL_SCALE, 1.0);
            }
        }
    }

    let moved = floor(old_position.x) != floor(entity.position.x)
        || floor(old_position.y) != floor(entity.position.y)
        || floor(old_position.z) != floor(entity.position.z);
    let rate = if moved {
        MERGE_RATE_MOVED
    } else {
        MERGE_RATE_STILL
    };
    if entity.tick_count % rate == 0 && is_mergable(entity) {
        merge_with_neighbours(entity, store);
    }

    // `:174-176`.
    let item_data = data_mut(entity);
    if item_data.age != INFINITE_LIFETIME {
        item_data.age += 1;
    }

    if crate::entity::environment::update_fluid_interaction_in_level(entity, store, level).is_none()
    {
        return false;
    }
    // `:178-183`.
    if entity.is_in_water()
        || entity.is_in_lava()
        || entity.delta_movement.distance_to_sqr(old_movement) > 0.01
    {
        entity.set_needs_sync(true);
    }

    // `:186-188`.
    if data(entity).age >= LIFETIME {
        entity.discard();
    }
    true
}

fn floor(value: f64) -> i32 {
    value.floor() as i32
}

// Entity.java:2834-2862: choose the nearest non-full neighbour, excluding down.
pub(crate) fn move_towards_closest_space(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<()> {
    let bounds = entity.bounding_box();
    let center = Vec3d {
        x: entity.position.x,
        y: (bounds.min_y + bounds.max_y) * 0.5,
        z: entity.position.z,
    };
    let pos = BlockPos {
        x: floor(center.x),
        y: floor(center.y),
        z: floor(center.z),
    };
    let fraction = Vec3d {
        x: center.x - f64::from(pos.x),
        y: center.y - f64::from(pos.y),
        z: center.z - f64::from(pos.z),
    };
    let cube_bounds = Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 });
    let cube = BoxShape::from_box(cube_bounds)?;
    let mut closest = f64::MAX;
    let mut escape = (Axis::Y, 1.0);
    let mut scratch = Vec::new();
    for (direction, axis, sign) in [
        (Direction::North, Axis::Z, -1.0),
        (Direction::South, Axis::Z, 1.0),
        (Direction::West, Axis::X, -1.0),
        (Direction::East, Axis::X, 1.0),
        (Direction::Up, Axis::Y, 1.0),
    ] {
        let neighbour = bbb_server_types::directions::relative(pos, direction);
        scratch.clear();
        level.push_block_collision_boxes_empty(neighbour, &mut scratch)?;
        let full = if scratch.len() == 1 && scratch[0].is_unit_cube() {
            true
        } else {
            let shape = BoxShape::from_boxes(&scratch)?;
            cube.only_first(&shape)?.is_empty() && shape.only_first(&cube)?.is_empty()
        };
        let distance = if sign > 0.0 {
            1.0 - fraction.axis(axis)
        } else {
            fraction.axis(axis)
        };
        if !full && distance < closest {
            closest = distance;
            escape = (axis, sign);
        }
    }
    let speed = f64::from(store.random().next_float() * 0.2 + 0.1);
    entity.delta_movement = entity
        .delta_movement
        .scale(0.75)
        .with_axis(escape.0, escape.1 * speed);
    Some(())
}

///
/// A stack whose maximum size this crate cannot derive answers false, which is
/// the fail-closed direction: an unmergable item is left alone rather than
/// merged on a guess.
pub fn is_mergable(entity: &Entity) -> bool {
    let item_data = data(entity);
    entity.is_alive()
        && item_data.pickup_delay != INFINITE_PICKUP_DELAY
        && item_data.age != INFINITE_LIFETIME
        && item_data.age < LIFETIME
        && merge_facts(item_data.item()).is_some_and(|facts| facts.count() < facts.max_stack_size())
}

pub fn merge_with_neighbours(entity: &mut Entity, store: &mut EntityStore) {
    // `:211`. The caller already asked, and vanilla asks again here because
    // `mergeWithNeighbours` has a second call site (`teleport`, `:369-376`).
    if !is_mergable(entity) {
        return;
    }
    let search_box = entity.bounding_box.inflate(
        MERGE_SEARCH_INFLATION[0],
        MERGE_SEARCH_INFLATION[1],
        MERGE_SEARCH_INFLATION[2],
    );
    let candidates: Vec<i32> = store
        .ids_in_box_sections(search_box)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|other| {
                matches!(other.data, EntityData::Item(_))
                    && other.bounding_box.intersects(search_box)
                    && is_mergable(other)
            })
        })
        .collect();
    for other_id in candidates {
        // `:214`: the predicate ran while the list was being built, and an
        // earlier merge can have emptied or filled a later neighbour, so
        // vanilla asks each one again as it reaches it.
        let Some(other) = store.get(other_id) else {
            continue;
        };
        if !is_mergable(other) {
            continue;
        }
        try_to_merge(entity, store, other_id);
        // `:216-218`.
        if entity.removal_reason.is_some() {
            break;
        }
    }
}

fn try_to_merge(entity: &mut Entity, store: &mut EntityStore, other_id: i32) {
    let Some(other_entity) = store.get(other_id) else {
        return;
    };
    let this_data = data(entity);
    let other_data = data(other_entity);
    if this_data.target != other_data.target
        || !are_mergable(this_data.item(), other_data.item(), store.item_resolver())
    {
        return;
    }
    let this_count = stack_count(this_data.item());
    let other_count = stack_count(other_data.item());
    merge_entities(entity, store, other_id, other_count < this_count);
}

///
/// The size ceiling is read off the *second* stack, which is the one
/// `tryToMerge` passes as `other`; both stacks resolve to the same components
/// whenever the second test passes, so the two maxima agree in every case that
/// can merge.
fn are_mergable(
    this_stack: &RuntimeItemStack,
    other_stack: &RuntimeItemStack,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> bool {
    let (Some(this_facts), Some(other_facts)) = (merge_facts(this_stack), merge_facts(other_stack))
    else {
        return false;
    };
    if other_facts.count() + this_facts.count() > other_facts.max_stack_size() {
        return false;
    }
    this_stack
        .same_item_same_effective_components(other_stack, resolver)
        .unwrap_or(false)
}

///
/// `entity_is_target` picks which of the two is vanilla's `toItem`. The ticking
/// entity is the one held by reference and the other lives in the store, so the
/// two roles are spelled out rather than aliased.
///
/// `delta` is not clamped at zero, exactly as vanilla does not clamp it: for an
/// item whose `max_stack_size` component is above 64 the ceiling at `:255` can
/// be below the target's current count, and the transfer then runs backwards.
fn merge_entities(
    entity: &mut Entity,
    store: &mut EntityStore,
    other_id: i32,
    entity_is_target: bool,
) {
    let this_data = data(entity).clone();
    let Some(other_entity) = store.get(other_id) else {
        return;
    };
    let other_data = data(other_entity).clone();
    let (to_data, from_data) = if entity_is_target {
        (&this_data, &other_data)
    } else {
        (&other_data, &this_data)
    };
    let (Some(to_facts), Some(from_facts)) =
        (merge_facts(to_data.item()), merge_facts(from_data.item()))
    else {
        return;
    };
    // `:248`.
    let delta = (to_facts.max_stack_size().min(MERGE_STACK_LIMIT) - to_facts.count())
        .min(from_facts.count());
    // `:249-250`.
    let (Some(new_to_stack), Some(new_from_stack)) = (
        stack_with_count(to_data.item(), to_facts.count() + delta),
        stack_with_count(from_data.item(), from_facts.count() - delta),
    ) else {
        return;
    };
    // `:256`, `:261-262`.
    let new_to_pickup_delay = to_data.pickup_delay.max(from_data.pickup_delay);
    let new_to_age = to_data.age.min(from_data.age);
    let from_is_empty = new_from_stack.is_empty();

    if entity_is_target {
        let Some(source) = store.get_mut(other_id) else {
            return;
        };
        data_mut(source).update_item_count(new_from_stack);
        source.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
        if from_is_empty {
            store.remove(other_id, RemovalReason::Discarded);
        }
        let target = data_mut(entity);
        target.set_item(new_to_stack);
        target.pickup_delay = new_to_pickup_delay;
        target.age = new_to_age;
        entity.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
    } else {
        let Some(target_entity) = store.get_mut(other_id) else {
            return;
        };
        let target = data_mut(target_entity);
        target.set_item(new_to_stack);
        target.pickup_delay = new_to_pickup_delay;
        target.age = new_to_age;
        target_entity.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
        data_mut(entity).update_item_count(new_from_stack);
        if from_is_empty {
            entity.discard();
        } else {
            entity.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTouch {
    /// One of the two gates at `:344` refused, or the inventory took nothing
    /// at all. The entity is exactly as it was.
    Nothing,
    /// The inventory moved part of the stack and answered `false`, so `:345`
    /// onwards never ran: no take packet, no statistic, no discard. The entity
    /// keeps what would not fit.
    ///
    /// This is not a rounding of vanilla's answer; it is vanilla's answer. See
    /// [`player_touch`] for why `Inventory.add` reports success only for a
    /// stack it emptied.
    PartlyTaken,
    /// `:345-352` ran: the inventory emptied the stack.
    Taken {
        /// `orgCount` (`:343`), read before the inventory saw the stack, which
        /// is the number `Player.take` reports to every tracker (`:345`).
        org_count: i32,
    },
}

///
/// `add` is `player.getInventory().add(itemStack)` (`:344`). It is a callback
/// rather than an argument because vanilla hands the inventory the entity's
/// *live* stack: the inventory decides how much it takes and shrinks that
/// stack in place, so what it declines is what stays on the ground. Passing
/// the inventory itself would mean this module reaching into
/// [`crate::inventory`], which owns the slot search.
///
/// # `Inventory.add` answers true only for a stack it emptied
///
///
/// A failed inventory callback leaves the stack on the ground and records a
/// pickup refusal. The callback receives the complete persistent stack;
/// network publication does not decide whether its physical items can move.
pub fn player_touch<F>(entity: &mut Entity, player_uuid: Uuid, add: F) -> ItemTouch
where
    F: FnOnce(&mut RuntimeItemStack) -> bbb_server_types::Result<bool>,
{
    if !matches!(entity.data, EntityData::Item(_)) {
        return ItemTouch::Nothing;
    }
    // `:340` is the client-side cut; this crate only ever runs the server.
    let item_data = data(entity);
    // `:341-343`.
    let mut offered = item_data.item().clone();
    let org_count = stack_count(&offered);
    // `:344`, the two gates that run before the inventory is asked. A delay
    // still counting down refuses, and so does `INFINITE_PICKUP_DELAY`, which
    // never counts down at all.
    if item_data.pickup_delay != 0 {
        return ItemTouch::Nothing;
    }
    if item_data.target.is_some_and(|target| target != player_uuid) {
        return ItemTouch::Nothing;
    }
    let answered = match add(&mut offered) {
        Ok(answered) => answered,
        Err(_) => return ItemTouch::Nothing,
    };
    let remaining = stack_count(&offered);
    if remaining != org_count {
        // The inventory shrank the stack it was handed, and that stack is the
        // entity's own.
        data_mut(entity).update_item_count(offered);
        entity.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
    }
    if !answered {
        return if remaining == org_count {
            ItemTouch::Nothing
        } else {
            ItemTouch::PartlyTaken
        };
    }
    ItemTouch::Taken { org_count }
}

pub fn stack_count(stack: &RuntimeItemStack) -> i32 {
    stack.count()
}

pub(crate) fn stack_max_size(stack: &RuntimeItemStack) -> Option<i32> {
    stack.max_stack_size().ok()
}

pub fn stack_with_count(stack: &RuntimeItemStack, count: i32) -> Option<RuntimeItemStack> {
    Some(stack.with_count(count))
}

/// `ItemStack.EMPTY` in the persistent form the codec writes for it: no fields
/// at all.
pub fn empty_stack() -> Option<RuntimeItemStack> {
    Some(RuntimeItemStack::empty())
}

/// One nonempty stack of `item_resource_id`, which is what
/// `new ItemStack(ItemLike)` produces for a caller that has an item and a
/// count and nothing else.
///
/// `None` for a resource id outside the built-in 26.1 item registry or a count
/// the codec refuses.
pub fn plain_stack(item_resource_id: &str, count: i32) -> Option<RuntimeItemStack> {
    let item = bbb_protocol::item_types::vanilla_item_type_for_resource_id(item_resource_id)?;
    (1..=99).contains(&count).then(|| {
        RuntimeItemStack::from_summary(bbb_protocol::packets::ItemStackSummary {
            item_id: Some(item.id),
            count,
            component_patch: Default::default(),
        })
    })
}

/// `ItemStack.getMaxStackSize()` and `isSameItemSameComponents` for the merge
/// rules; `None` for a stack whose maximum is unreadable.
pub(crate) struct MergeFacts<'a> {
    stack: &'a RuntimeItemStack,
    max_stack_size: i32,
}

impl MergeFacts<'_> {
    pub(crate) fn count(&self) -> i32 {
        self.stack.count()
    }

    pub(crate) fn max_stack_size(&self) -> i32 {
        self.max_stack_size
    }
}

pub(crate) fn merge_facts(stack: &RuntimeItemStack) -> Option<MergeFacts<'_>> {
    Some(MergeFacts {
        stack,
        max_stack_size: stack.max_stack_size().ok()?,
    })
}

impl EntityData {
    ///
    /// `in_tag` answers that tag for one item type, because the tag generation
    /// belongs to the level and not to the stack.
    pub fn dampens_vibrations(&self, in_tag: impl Fn(i32) -> bool) -> bool {
        match self {
            Self::Item(data) => data.item().item_id().is_some_and(in_tag),
            Self::GeneralLiving(data) => data.entity_resource_id() == "minecraft:warden",
            _ => false,
        }
    }

    /// `ItemEntity.getOwner()` (`ItemEntity.java:89-91`): the entity the
    /// `thrower` reference names, which this crate keeps as its UUID. Only an
    /// `ItemEntity` answers it -- the readers reach it through an `instanceof`
    /// (`SculkShriekerBlockEntity.java:96`).
    #[must_use]
    pub const fn item_thrower(&self) -> Option<Uuid> {
        match self {
            Self::Item(data) => data.thrower(),
            _ => None,
        }
    }
}

pub fn set_thrower_reference(
    entity: &mut Entity,
    reference: Option<crate::entity::entity_reference::EntityReference>,
) -> Option<()> {
    if !matches!(entity.data(), EntityData::Item(_)) {
        return None;
    }
    data_mut(entity).set_thrower_reference(reference);
    Some(())
}

pub fn restore_taken_stack(entity: &mut Entity, stack: RuntimeItemStack) -> Option<()> {
    if !matches!(entity.data(), EntityData::Item(_)) {
        return None;
    }
    data_mut(entity).item = stack;
    Some(())
}

/// The subtype data of an entity this module's dispatch has already selected.
fn data(entity: &Entity) -> &ItemEntityData {
    match &entity.data {
        EntityData::Item(data) => data,
        other => unreachable!("item entity tick reached {other:?}"),
    }
}

fn data_mut(entity: &mut Entity) -> &mut ItemEntityData {
    match &mut entity.data {
        EntityData::Item(data) => data,
        other => unreachable!("item entity tick reached {other:?}"),
    }
}

///
/// The value is the stack's own codec document, so the read is a decode rather
/// than a translation and the write is the same document again.
pub(crate) type ItemResolver<'r> = &'r dyn bbb_registry_codec::PersistentItemStackHolderResolver;

/// `ItemStack.CODEC` encode of a non-empty stack; a stack the publication
/// cannot name is written as `ItemStack.EMPTY`'s empty compound.
pub(crate) fn write_saved_stack(
    stack: &RuntimeItemStack,
    items: ItemResolver<'_>,
) -> NbtSummaryValue {
    stack
        .write(items)
        .ok()
        .flatten()
        .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()))
}

pub fn read_saved_stack(
    value: &NbtSummaryValue,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Option<RuntimeItemStack> {
    RuntimeItemStack::try_read(value, items).ok()
}

impl Entity {
    pub(crate) fn set_dropped_item(&mut self, stack: RuntimeItemStack) {
        if let EntityData::Item(data) = &mut self.data {
            data.set_item(stack);
            self.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
        }
    }
}

pub(crate) fn throw_item(
    body: &Entity,
    store: &mut EntityStore,
    towards: Vec3d,
    thrown: bbb_server_types::item_stack::RuntimeItemStack,
) {
    throw_item_with_velocity(
        body,
        store,
        towards,
        thrown,
        Vec3d {
            x: 0.3,
            y: 0.3,
            z: 0.3,
        },
        0.3,
    );
}

// BehaviorUtils.java:94-104.
pub(crate) fn throw_item_with_velocity(
    body: &Entity,
    store: &mut EntityStore,
    towards: Vec3d,
    thrown: bbb_server_types::item_stack::RuntimeItemStack,
    velocity: Vec3d,
    eye_offset: f64,
) {
    let position = body.position();
    let eye_y = position.y + f64::from(body.eye_height()) - eye_offset;
    let Some(item_id) = crate::entity::item::spawn_thrown_item_entity(
        store,
        Vec3d {
            y: eye_y,
            ..position
        },
        thrown,
        10,
        body.uuid(),
    ) else {
        return;
    };
    let (dx, dy, dz) = (
        towards.x - position.x,
        towards.y - position.y,
        towards.z - position.z,
    );
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    if length >= 1.0e-5 {
        if let Some(item) = store.get_mut(item_id) {
            item.set_delta_movement(Vec3d {
                x: dx / length * velocity.x,
                y: dy / length * velocity.y,
                z: dz / length * velocity.z,
            });
        }
    }
}
