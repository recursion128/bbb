//! `ExperienceOrb`: the entity a broken ore leaves behind.
//!
//! This is the third entity this crate simulates, and the first one a player
//! *consumes* rather than picks up into an inventory. Three things make it
//! worth its own module rather than a variant of [`super::item`]:
//!
//!
//! # What this module deliberately does not model
//!
//! * **`moveTowardsClosestSpace`** (`:124-130`). An orb spawned inside a
//!   block stays there rather than being nudged out.
//! * **`unstuckIfPossible`** (`:78-84`) needs `Level.findFreePosition`,
//!   which has no owner here.
//! * **`applyEffectsFromBlocks`** (`:134`), the `InsideBlockEffectApplier`
//!   half.
//! * **`hurtServer`** (`:244-256`) is owned by [`super::damage`].

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::NbtSummaryEntry;

use crate::entity::level::fluid_state_tag_membership;
use crate::entity::save::{optional_int, optional_short};
use crate::entity::store::EntityStore;
use crate::entity::{apply_gravity, Entity, EntityData, MoverType};
use bbb_server_types::experience::{experience_value, LIFETIME};
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::geometry::{visit_block_collisions_with_scratch, Aabb, Vec3Ops};
use std::cell::Cell;

/// `ExperienceOrb.getDefaultGravity()` (`ExperienceOrb.java:97-99`).
pub const DEFAULT_GRAVITY: f64 = 0.03;

/// `ExperienceOrb.DEFAULT_HEALTH` (`ExperienceOrb.java:37`), which is both the
/// field initialiser (`:42`) and the `readAdditionalSaveData` default (`:268`).
pub const DEFAULT_HEALTH: i32 = 5;

/// `ExperienceOrb.DEFAULT_COUNT` (`ExperienceOrb.java:40`), the `Count` a save
/// with none falls back to (`:271`).
pub const DEFAULT_COUNT: i32 = 1;

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> ExperienceOrbData {
    ExperienceOrbData::from_saved(
        i32::from(optional_short(entries, "Health").unwrap_or(DEFAULT_HEALTH as i16)),
        i32::from(optional_short(entries, "Age").unwrap_or(0)),
        i32::from(optional_short(entries, "Value").unwrap_or(0)),
        optional_int(entries, "Count")
            .filter(|count| *count > 0)
            .unwrap_or(DEFAULT_COUNT),
    )
}

/// `ExperienceOrb.ENTITY_SCAN_PERIOD` (`ExperienceOrb.java:33`): the tick
/// stride `scanForMerges` runs on (`:119`).
const ENTITY_SCAN_PERIOD: i32 = 20;

/// `ExperienceOrb.MAX_FOLLOW_DIST` (`ExperienceOrb.java:34`), the radius
/// `followNearbyPlayer` looks in (`:154`) and the divisor its pull strength is
/// scaled by (`:169`).
const MAX_FOLLOW_DIST: f64 = 8.0;

/// `this.followingPlayer.distanceToSqr(this) > 64.0` (`ExperienceOrb.java:153`):
/// the squared distance at which the orb lets go of the player it was chasing.
/// Vanilla writes the square out rather than squaring `MAX_FOLLOW_DIST`.
const FOLLOW_RELEASE_DISTANCE_SQR: f64 = 64.0;

/// `power * power * 0.1` (`ExperienceOrb.java:170`): the pull applied to the
/// delta once the direction is normalised.
const FOLLOW_PULL: f64 = 0.1;

/// `ExperienceOrb.ORB_GROUPS_PER_AREA` (`ExperienceOrb.java:35`): the modulus
/// two orbs' entity ids have to agree on before they may merge (`:220`), and
/// the bound of the draw `tryMergeToExisting` makes (`:203`).
pub const ORB_GROUPS_PER_AREA: i32 = 40;

/// `ExperienceOrb.ORB_MERGE_DISTANCE` (`ExperienceOrb.java:36`), how far
/// `scanForMerges` inflates the bounding box (`:181`).
const ORB_MERGE_DISTANCE: f64 = 0.5;

/// `AABB.ofSize(pos, 1.0, 1.0, 1.0)` (`ExperienceOrb.java:202`): the box
/// `tryMergeToExisting` looks for an existing orb in.
const AWARD_MERGE_BOX_SIZE: f64 = 1.0;

/// The drag a moving orb keeps (`ExperienceOrb.java:135`), before the block's
/// own friction is folded in at `:137`.
const DRAG: f32 = 0.98;

/// `-fallSpeed * 0.4` (`ExperienceOrb.java:142`): the bounce a landing orb
/// keeps.
const LANDED_VERTICAL_SCALE: f64 = 0.4;

/// `ExperienceOrb.getBlockPosBelowThatAffectsMyMovement()`
/// (`ExperienceOrb.java:175-177`): `getOnPos(0.999999F)`, the same whole-block
/// reach `ItemEntity` uses rather than `Entity`'s own half block.
pub const BLOCK_POS_BELOW_OFFSET: f32 = 0.999_999;

/// `player.takeXpDelay = 2` (`Player.java:278`): the ticks a session waits
/// between two orbs.
pub const TAKE_XP_DELAY: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceOrbData {
    /// `:41`, the counter `:145-148` retires the orb on.
    age: i32,
    /// `:42`, only ever moved by `hurtServer`, which has no owner here.
    health: i32,
    /// `:43`. How many *awards* this one entity stands for: `playerTouch`
    /// hands over `getValue()` and decrements this, so an orb with a count of
    /// three pays three times (`:280-288`).
    count: i32,
    /// The synched `DATA_VALUE` (`:31`), what one touch is worth.
    value: i32,
    /// `:44`. Transient in vanilla too -- it is not in `addAdditionalSaveData`
    /// -- so a reloaded orb picks a player again on its next tick.
    following_player: Option<uuid::Uuid>,
}

impl ExperienceOrbData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.following_player = previous.following_player;
    }

    /// The field initialisers plus `setValue` (`ExperienceOrb.java:41-43`,
    /// `:71`).
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self {
            age: 0,
            health: DEFAULT_HEALTH,
            count: DEFAULT_COUNT,
            value,
            following_player: None,
        }
    }

    /// The four fields `readAdditionalSaveData` fills (`ExperienceOrb.java:267-272`).
    #[must_use]
    pub const fn from_saved(health: i32, age: i32, value: i32, count: i32) -> Self {
        Self {
            age,
            health,
            count,
            value,
            following_player: None,
        }
    }

    #[must_use]
    pub const fn age(&self) -> i32 {
        self.age
    }

    #[must_use]
    pub const fn health(&self) -> i32 {
        self.health
    }

    pub fn set_health(&mut self, health: i32) {
        self.health = health;
    }

    #[must_use]
    pub const fn count(&self) -> i32 {
        self.count
    }

    /// `ExperienceOrb.getValue()` (`ExperienceOrb.java:313-315`).
    #[must_use]
    pub const fn value(&self) -> i32 {
        self.value
    }
}

fn data(entity: &Entity) -> &ExperienceOrbData {
    match entity.data() {
        EntityData::ExperienceOrb(orb) => orb,
        other => unreachable!("an experience orb carries orb data, not {other:?}"),
    }
}

fn data_mut(entity: &mut Entity) -> &mut ExperienceOrbData {
    match &mut entity.data {
        EntityData::ExperienceOrb(orb) => orb,
        other => unreachable!("an experience orb carries orb data, not {other:?}"),
    }
}

fn no_collision(level: &LevelCtx<'_, '_>, box_to_test: Aabb) -> Option<bool> {
    let unavailable = Cell::new(false);
    let mut colliding = false;
    let _ = visit_block_collisions_with_scratch(
        box_to_test,
        |pos| {
            level.is_moving_piston_at(pos).unwrap_or_else(|| {
                unavailable.set(true);
                false
            })
        },
        |pos, out| {
            let start = out.len();
            if level.push_block_collision_boxes_empty(pos, out).is_none() {
                out.truncate(start);
                unavailable.set(true);
            }
        },
        |_, shape| {
            if shape.intersects(box_to_test) {
                colliding = true;
            }
            true
        },
    );
    (!unavailable.get()).then_some(!colliding)
}

///
/// The store is threaded in for the same reason [`super::item`]'s tick takes
/// one: `scanForMerges` (`:179-185`) reads and writes *other* orbs.
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    // `:103-105` is the client interpolation and the client-only first-tick
    // return; the server always takes the `else` at `:106`.
    // `:107`, `Entity.tick` over `baseTick`.
    if !crate::entity::damage::base_tick_from_environment(entity, store, level) {
        return false;
    }
    // `:108`.
    let Some(clear) = no_collision(level, entity.bounding_box) else {
        return false;
    };
    let colliding = !clear;
    // `:109-113`.
    let Some(in_water) = eye_water_contact(entity, level) else {
        return false;
    };
    let lava_membership = fluid_state_tag_membership(
        level,
        block_state_fluid_state(level.block_state(entity.block_position())),
        "minecraft:lava",
    );
    if in_water {
        set_underwater_movement(entity);
    } else if !colliding {
        apply_gravity(entity);
    }
    // `:115-117`: four entity-random floats after both fluid memberships are
    // known, so an unavailable generation cannot consume the stream.
    if lava_membership {
        entity.set_delta_movement(Vec3d {
            x: f64::from((store.random().next_float() - store.random().next_float()) * 0.2),
            y: 0.2,
            z: f64::from((store.random().next_float() - store.random().next_float()) * 0.2),
        });
    }

    // `:119-121`.
    if entity.tick_count % ENTITY_SCAN_PERIOD == 1 {
        scan_for_merges(entity, store);
    }

    // `:123`.
    follow_nearby_player(entity, level);
    // `:124-130`.
    if data(entity).following_player.is_none() && colliding {
        let Some(next_free) = no_collision(level, entity.bounding_box.moved(entity.delta_movement))
        else {
            return false;
        };
        if !next_free {
            if super::item::move_towards_closest_space(entity, store, level).is_none() {
                return false;
            }
            entity.set_needs_sync(true);
        }
    }

    // `:132-133`.
    let fall_speed = entity.delta_movement.y;
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
    // `:134`.
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
        return false;
    }

    // `:135-138`. The block position is the orb's own override, a whole block
    // below the feet.
    let mut friction = DRAG;
    if entity.on_ground() {
        friction = level.block_friction(entity.block_pos_below_that_affects_my_movement()) * DRAG;
    }
    // `:140`. One `Vec3.scale`, so all three components take the same widened
    // float.
    let friction = f64::from(friction);
    entity.delta_movement = entity.delta_movement.scale(friction);
    // `:141-143`.
    if entity.vertical_collision_below() && fall_speed < -entity.gravity() {
        entity.delta_movement.y = -fall_speed * LANDED_VERTICAL_SCALE;
    }

    // `:145-148`.
    let orb = data_mut(entity);
    orb.age += 1;
    if orb.age >= LIFETIME {
        entity.discard();
    }
    true
}

fn eye_water_contact(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<bool> {
    let eye_height = entity.eye_height();
    let eye_y = entity.position().y + f64::from(eye_height);
    let pos = BlockPos {
        x: entity.position().x.floor() as i32,
        y: eye_y.floor() as i32,
        z: entity.position().z.floor() as i32,
    };
    let state_id = level.block_state(pos);
    if vanilla_block_state_block_type_id(state_id).is_none() {
        level.content_registry().block_by_state_id(state_id)?;
        return Some(false);
    }
    let fluid = block_state_fluid_state(state_id);
    if !fluid_state_tag_membership(level, fluid, "minecraft:water") {
        return Some(false);
    }
    let above_id = level.block_state(BlockPos {
        y: pos.y + 1,
        ..pos
    });
    if vanilla_block_state_block_type_id(above_id).is_none() {
        level.content_registry().block_by_state_id(above_id)?;
    }
    let height = if block_state_fluid_state(above_id).family() == fluid.family() {
        1.0
    } else {
        f64::from(fluid.own_height())
    };
    Some(eye_y >= f64::from(pos.y) && eye_y <= f64::from(pos.y) + height)
}

fn set_underwater_movement(entity: &mut Entity) {
    let movement = entity.delta_movement();
    entity.set_delta_movement(Vec3d {
        x: movement.x * f64::from(0.99_f32),
        y: (movement.y + f64::from(5.0e-4_f32)).min(f64::from(0.06_f32)),
        z: movement.z * f64::from(0.99_f32),
    });
}

fn follow_nearby_player(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    let position = entity.position();
    // `:153`. A player the level no longer holds reads as gone, which is the
    // same branch a spectator takes.
    let held = data(entity)
        .following_player
        .and_then(|uuid| level.player(uuid))
        .filter(|player| {
            !player.spectator
                && player.position.distance_to_sqr(position) <= FOLLOW_RELEASE_DISTANCE_SQR
        });
    let following = match held {
        Some(player) => Some(player),
        None => level
            .nearest_player(position, MAX_FOLLOW_DIST)
            .filter(|player| !player.spectator && !player.dead_or_dying),
    };
    data_mut(entity).following_player = following.as_ref().map(|player| player.uuid);
    // `:162-171`.
    let Some(player) = following else {
        return;
    };
    let delta = Vec3d {
        x: player.position.x - position.x,
        y: player.position.y + player.eye_height / 2.0 - position.y,
        z: player.position.z - position.z,
    };
    let length = delta.length_sqr();
    let power = 1.0 - length.sqrt() / MAX_FOLLOW_DIST;
    entity.delta_movement = entity
        .delta_movement
        .plus(delta.normalize().scale(power * power * FOLLOW_PULL));
}

fn scan_for_merges(entity: &mut Entity, store: &mut EntityStore) {
    let query =
        entity
            .bounding_box
            .inflate(ORB_MERGE_DISTANCE, ORB_MERGE_DISTANCE, ORB_MERGE_DISTANCE);
    let id = entity.id();
    let value = data(entity).value;
    // `EntityGetter.getEntities(EntityTypeTest, AABB, Predicate)`, whose own
    // test is the other entity's bounding box against the query box.
    let candidates: Vec<i32> = store
        .ids_in_box_sections(query)
        .into_iter()
        .filter(|other| {
            store.get(*other).is_some_and(|other| {
                other.bounding_box().intersects(query)
                    && matches!(other.data(), EntityData::ExperienceOrb(_))
                    // `:216`, `:220`.
                    && other.id() != id
                    && can_merge(other, id, value)
            })
        })
        .collect();
    for other_id in candidates {
        // `:224-226`.
        let Some((other_count, other_age)) = store.get(other_id).map(|other| {
            let other_data = data(other);
            (other_data.count, other_data.age)
        }) else {
            continue;
        };
        store.remove(other_id, crate::entity::RemovalReason::Discarded);
        let orb = data_mut(entity);
        orb.count += other_count;
        orb.age = orb.age.min(other_age);
    }
}

///
/// The modulus is taken on the *signed* difference, which in Java keeps the
/// sign of the dividend; Rust's `%` does the same, so the transcription is
/// literal.
fn can_merge(orb: &Entity, id: i32, value: i32) -> bool {
    orb.is_alive() && (orb.id() - id) % ORB_GROUPS_PER_AREA == 0 && data(orb).value == value
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OrbTouch {
    /// The value the session is owed, before mending spends any of it. Zero
    /// when the touch did not happen at all.
    pub awarded: i32,
    /// Whether the touch happened, which is what arms `takeXpDelay`.
    pub taken: bool,
}

///
/// `player.take(this, 1)` (`:279`) is the pickup animation packet, which the
/// caller sends because only it can see the tracking sessions -- the same
/// split [`super::item`]'s touch uses. `repairPlayerItems` (`:280`) and
/// `giveExperiencePoints` (`:282`) are the caller's too: both read the
/// session's inventory, which this mechanism has no reach into.
pub fn player_touch(entity: &mut Entity, take_xp_delay: i32) -> OrbTouch {
    // `:277`.
    if !entity.is_alive() || take_xp_delay != 0 {
        return OrbTouch::default();
    }
    let orb = data_mut(entity);
    let awarded = orb.value;
    // `:285-288`.
    orb.count -= 1;
    let empty = orb.count == 0;
    if empty {
        entity.discard();
    }
    OrbTouch {
        awarded,
        taken: true,
    }
}

///
///
/// The draws are vanilla's, in vanilla's order: one `nextInt(40)` per orb from
/// the *level* generator for the merge bucket (`:203`), then, only for an orb
/// that is actually constructed, the rotation and three movement components
/// from the entity's own generator (`:55-58`).
pub fn award(
    store: &mut EntityStore,
    level_random: &crate::random::GameRandom,
    position: Vec3d,
    amount: i32,
) -> Vec<i32> {
    let mut spawned = Vec::new();
    let mut amount = amount;
    // Maximum single-orb value from `ExperienceOrb.getExperienceValue` (`ExperienceOrb.java:346-349`).
    const MAX_ORB_VALUE: i32 = 2477;
    const BATCH_THRESHOLD: i32 = MAX_ORB_VALUE * ORB_GROUPS_PER_AREA;
    if amount > BATCH_THRESHOLD {
        let full_orbs = amount / MAX_ORB_VALUE;
        amount %= MAX_ORB_VALUE;
        let half = AWARD_MERGE_BOX_SIZE / 2.0;
        let box_to_test = Aabb {
            min_x: position.x - half,
            min_y: position.y - half,
            min_z: position.z - half,
            max_x: position.x + half,
            max_y: position.y + half,
            max_z: position.z + half,
        };
        let existing = store.ids_in_box_sections(box_to_test).into_iter().find(|other| {
            store.get(*other).is_some_and(|other| {
                other.is_alive()
                    && other.bounding_box().intersects(box_to_test)
                    && matches!(other.data(), EntityData::ExperienceOrb(orb) if orb.value == MAX_ORB_VALUE)
            })
        });
        if let Some(found) = existing {
            if let Some(orb) = store.get_mut(found) {
                let orb = data_mut(orb);
                orb.count = orb.count.saturating_add(full_orbs);
                orb.age = 0;
            }
        } else if let Some(id) = spawn_experience_orb(store, position, MAX_ORB_VALUE) {
            if full_orbs > 1 {
                if let Some(orb) = store.get_mut(id) {
                    let orb = data_mut(orb);
                    orb.count = orb.count.saturating_add(full_orbs - 1);
                }
            }
            spawned.push(id);
        }
    }
    // `:192-198`.
    while amount > 0 {
        let new_count = experience_value(amount);
        amount -= new_count;
        if try_merge_to_existing(store, level_random, position, new_count) {
            continue;
        }
        if let Some(id) = spawn_experience_orb(store, position, new_count) {
            spawned.push(id);
        }
    }
    spawned
}

fn try_merge_to_existing(
    store: &mut EntityStore,
    level_random: &crate::random::GameRandom,
    position: Vec3d,
    value: i32,
) -> bool {
    // `:202`, `AABB.ofSize` centred on the award position.
    let half = AWARD_MERGE_BOX_SIZE / 2.0;
    let box_to_test = Aabb {
        min_x: position.x - half,
        min_y: position.y - half,
        min_z: position.z - half,
        max_x: position.x + half,
        max_y: position.y + half,
        max_z: position.z + half,
    };
    // `:203`. The draw happens whether or not any orb is found, so it is made
    // before the search.
    let id = level_random.next_int(ORB_GROUPS_PER_AREA);
    // `:204-206`, first match in the level's own iteration order.
    let found = store
        .ids_in_box_sections(box_to_test)
        .into_iter()
        .find(|other| {
            store.get(*other).is_some_and(|other| {
                other.bounding_box().intersects(box_to_test)
                    && matches!(other.data(), EntityData::ExperienceOrb(_))
                    && can_merge(other, id, value)
            })
        });
    let Some(found) = found else {
        return false;
    };
    // `:207-209`.
    let Some(orb) = store.get_mut(found) else {
        return false;
    };
    let orb = data_mut(orb);
    orb.count += 1;
    orb.age = 0;
    true
}

pub fn spawn_experience_orb(store: &mut EntityStore, position: Vec3d, value: i32) -> Option<i32> {
    let uuid = store.next_uuid();
    // `:55`.
    let y_rot = store.random().next_float() * 360.0;
    // `:56-58`.
    let delta_x = (store.random().next_double() * 0.2 - 0.1) * 2.0;
    let delta_y = store.random().next_double() * 0.2 * 2.0;
    let delta_z = (store.random().next_double() * 0.2 - 0.1) * 2.0;
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        position,
        EntityData::ExperienceOrb(ExperienceOrbData::new(value)),
        |entity| {
            entity.set_rot(y_rot, 0.0);
            // `:65`. `:66-68` is `unstuckIfPossible`, which has no owner here.
            entity.set_delta_movement(Vec3d {
                x: delta_x,
                y: delta_y,
                z: delta_z,
            });
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::store::EntityIdCounter;

    #[test]
    fn scan_for_merges_removes_absorbed_orb_from_store_immediately() {
        let ids = EntityIdCounter::new();
        let mut store = EntityStore::new(1, ids.clone());
        let pos = Vec3d {
            x: 0.5,
            y: 64.5,
            z: 0.5,
        };
        let first_id = spawn_experience_orb(&mut store, pos, 5).expect("first orb");
        // Advance entity ids so `(second_id - first_id) % ORB_GROUPS_PER_AREA == 0`.
        for _ in 1..ORB_GROUPS_PER_AREA {
            let _ = ids.next_id();
        }
        let second_id = spawn_experience_orb(&mut store, pos, 5).expect("second orb");
        assert_eq!((second_id - first_id) % ORB_GROUPS_PER_AREA, 0);
        let mut first = store.detach(first_id).expect("detached first");
        scan_for_merges(&mut first, &mut store);
        assert!(
            store.get(second_id).is_none(),
            "absorbed orb must be removed from store immediately"
        );
        assert_eq!(data(&first).count(), 2);
        // Discarded orb cannot be picked up again.
        first.discard();
        assert_eq!(player_touch(&mut first, 0), OrbTouch::default());
        store.attach(first);
    }
}
