//! `AbstractArrow`: the state and tick the arrow, the spectral arrow and the
//! thrown trident share -- flight with block and entity hits, sticking in the
//! ground, despawn and the player pickup.

use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID, VANILLA_ABSTRACT_ARROW_FLAG_CRIT,
    VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS, VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID,
    VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID,
};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{no_block_collision_with_scratch, Aabb, Vec3Ops};
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::hurting_projectile::ProjectileState;
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_action::{
    ArrowDamage, ArrowRefusal, PendingArrowHit, PendingProjectileAction, PendingProjectileHitBlock,
    PendingProjectileHurt,
};
use crate::entity::projectile_hit::{
    aabb_contains, for_each_candidate_hit, ProjectileCandidate, ProjectileDeflection,
    ProjectileTarget,
};
use crate::entity::store::{section_walk_order_key, EntityStore, PendingEntitySound};
use crate::entity::throwable_projectile::{
    owner_vehicle, pending_owner, pending_target, rides_with_owner,
};
use crate::entity::{apply_gravity, Entity, EntityCommand, EntityData, NearbyPlayer};
use crate::level_event::{PendingHurtEntity, VanillaGameEvent};
use crate::world::simulation_level::LevelCtx;

/// `AbstractArrow.getDefaultGravity()` (`AbstractArrow.java:339-341`).
pub const DEFAULT_GRAVITY: f64 = 0.05;

/// `AbstractArrow.ARROW_BASE_DAMAGE` (`AbstractArrow.java:56`).
pub const DEFAULT_DAMAGE: f64 = 2.0;

/// `AbstractArrow.INERTIA` and `WATER_INERTIA` (`AbstractArrow.java:58-59`).
const INERTIA: f64 = 0.99;
const WATER_INERTIA: f64 = 0.6;

/// `AbstractArrow.tickDespawn` (`AbstractArrow.java:375-379`).
const DESPAWN_TICKS: i32 = 1200;

/// `AbstractArrow.SHAKE_TIME` (`AbstractArrow.java:57`).
const SHAKE_TIME: i32 = 7;

/// `ThrownTrident.onHitEntity`'s base damage (`ThrownTrident.java:121`).
const TRIDENT_DAMAGE: f32 = 8.0;

/// `Arrow.EXPOSED_POTION_DECAY_TIME` (`Arrow.java:20`).
const EXPOSED_POTION_DECAY_TIME: i32 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowPickup {
    Disallowed = 0,
    Allowed = 1,
    CreativeOnly = 2,
}

impl ArrowPickup {
    pub const fn from_byte(value: i8) -> Option<Self> {
        match value {
            0 => Some(Self::Disallowed),
            1 => Some(Self::Allowed),
            2 => Some(Self::CreativeOnly),
            _ => None,
        }
    }

    pub const fn to_byte(self) -> i8 {
        self as i8
    }
}

/// The `Projectile` and `AbstractArrow` fields every arrow subtype carries.
#[derive(Debug, Clone, PartialEq)]
pub struct AbstractArrowState {
    life: i32,
    in_block_state_id: Option<i32>,
    shake_time: i32,
    in_ground: bool,
    in_ground_time: i32,
    pickup: ArrowPickup,
    damage: f64,
    flags: i8,
    pierce_level: i8,
    sound_event_resource_id: String,
    item: RuntimeItemStack,
    weapon: Option<RuntimeItemStack>,
    pub(crate) projectile: ProjectileState,
    /// `piercingIgnoreEntityIds`, which is never saved.
    piercing_ignore: Vec<i32>,
    /// `piercedAndKilledEntities` (`AbstractArrow.java:78`) by entity id; the
    /// world appends the victims a piercing hit killed.
    pierced_and_killed: Vec<i32>,
}

impl AbstractArrowState {
    pub(crate) fn new(item: RuntimeItemStack, sound_event_resource_id: &str) -> Self {
        Self::from_saved(
            0,
            None,
            0,
            false,
            ArrowPickup::Disallowed,
            DEFAULT_DAMAGE,
            false,
            0,
            sound_event_resource_id.to_string(),
            item,
            None,
            ProjectileState::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_saved(
        life: i32,
        in_block_state_id: Option<i32>,
        shake_time: i32,
        in_ground: bool,
        pickup: ArrowPickup,
        damage: f64,
        crit: bool,
        pierce_level: i8,
        sound_event_resource_id: String,
        item: RuntimeItemStack,
        weapon: Option<RuntimeItemStack>,
        projectile: ProjectileState,
    ) -> Self {
        Self {
            life,
            in_block_state_id,
            shake_time,
            in_ground,
            in_ground_time: 0,
            pickup,
            damage,
            flags: if crit {
                VANILLA_ABSTRACT_ARROW_FLAG_CRIT
            } else {
                0
            },
            pierce_level,
            sound_event_resource_id,
            item,
            weapon,
            projectile,
            piercing_ignore: Vec::new(),
            pierced_and_killed: Vec::new(),
        }
    }

    pub(crate) fn replace_slot_item(&mut self, item: RuntimeItemStack) {
        self.item = item;
    }

    pub const fn life(&self) -> i32 {
        self.life
    }

    pub const fn in_block_state_id(&self) -> Option<i32> {
        self.in_block_state_id
    }

    pub const fn shake_time(&self) -> i32 {
        self.shake_time
    }

    pub const fn in_ground(&self) -> bool {
        self.in_ground
    }

    pub(crate) const fn in_ground_time(&self) -> i32 {
        self.in_ground_time
    }

    #[cfg(test)]
    pub(crate) fn restore_in_ground_time(&mut self, in_ground_time: i32) {
        self.in_ground_time = in_ground_time;
    }

    // AbstractArrow.java:593-597,616-629; ThrownTrident.java:67-93,195-198.
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.in_ground_time = previous.in_ground_time;
        self.piercing_ignore = previous.piercing_ignore.clone();
        self.pierced_and_killed = previous.pierced_and_killed.clone();
        self.set_no_physics(previous.no_physics());
    }

    pub const fn pickup(&self) -> ArrowPickup {
        self.pickup
    }

    pub const fn damage(&self) -> f64 {
        self.damage
    }

    pub const fn crit(&self) -> bool {
        self.flags & VANILLA_ABSTRACT_ARROW_FLAG_CRIT != 0
    }

    pub const fn no_physics(&self) -> bool {
        self.flags & VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS != 0
    }

    pub const fn pierce_level(&self) -> i8 {
        self.pierce_level
    }

    pub fn sound_event_resource_id(&self) -> &str {
        &self.sound_event_resource_id
    }

    pub const fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub const fn weapon(&self) -> Option<&RuntimeItemStack> {
        self.weapon.as_ref()
    }

    pub const fn projectile(&self) -> &ProjectileState {
        &self.projectile
    }

    pub fn set_pickup(&mut self, pickup: ArrowPickup) {
        self.pickup = pickup;
    }

    /// `AbstractArrow.setBaseDamage` (`AbstractArrow.java:675-677`).
    pub(crate) fn set_damage(&mut self, damage: f64) {
        self.damage = damage;
    }

    /// `AbstractArrow.firedFromWeapon` (`AbstractArrow.java:104-112`).
    pub(crate) fn set_weapon(&mut self, weapon: Option<RuntimeItemStack>) {
        self.weapon = weapon;
    }

    pub(crate) fn set_pierce_level(&mut self, pierce_level: i8) {
        self.pierce_level = pierce_level;
    }

    pub(crate) fn set_sound_event(&mut self, resource_id: &str) {
        self.sound_event_resource_id = resource_id.to_string();
    }

    pub fn set_crit(&mut self, crit: bool) {
        self.set_flag(VANILLA_ABSTRACT_ARROW_FLAG_CRIT, crit);
    }

    pub(crate) fn set_no_physics(&mut self, no_physics: bool) {
        self.set_flag(VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS, no_physics);
    }

    fn set_flag(&mut self, flag: i8, value: bool) {
        if value {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    pub(crate) fn set_in_ground(&mut self, in_ground: bool) {
        self.in_ground = in_ground;
        if !in_ground {
            self.in_ground_time = 0;
        }
    }
}

/// The `AbstractArrow` synched rows after the `Entity` rows.
pub(crate) fn synched_values(entity: &Entity, state: &AbstractArrowState) -> Vec<SynchedValue> {
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    };
    use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
    let row = |data_id: u8, serializer_id: i32, value: EntityDataValueKind, is_default: bool| {
        SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id,
                value,
            },
            is_default,
        }
    };
    let mut rows = crate::entity::living::entity_synched_values(entity);
    rows.push(row(
        VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        EntityDataValueKind::Byte(state.flags),
        state.flags == 0,
    ));
    rows.push(row(
        VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        EntityDataValueKind::Byte(state.pierce_level),
        state.pierce_level == 0,
    ));
    rows.push(row(
        VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        EntityDataValueKind::Boolean(state.in_ground),
        !state.in_ground,
    ));
    rows
}

/// Files a flying arrow subtype carrying `movement`.
pub(crate) fn spawn(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: Vec3d,
    movement: Vec3d,
    data: EntityData,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let shot = crate::entity::projectile_shoot::ProjectileShot::new(movement);
    store.restore_initialized(uuid, entity_type_id, position, data, |entity| {
        shot.install(entity)
    })
}

pub(crate) fn state(data: &EntityData) -> Option<&AbstractArrowState> {
    match data {
        EntityData::Arrow(data) => Some(data),
        EntityData::SpectralArrow(data) => Some(data),
        EntityData::Trident(data) => Some(data),
        _ => None,
    }
}

pub(crate) fn set_weapon(entity: &mut Entity, weapon: Option<RuntimeItemStack>) {
    if let Some(state) = state_mut(&mut entity.data) {
        state.set_weapon(weapon);
    }
}

pub(crate) fn set_item(entity: &mut Entity, item: RuntimeItemStack) {
    if let Some(state) = state_mut(&mut entity.data) {
        state.replace_slot_item(item);
    }
}

/// Records a victim a piercing hit killed and answers every victim so far.
pub(crate) fn add_pierced_and_killed(entity: &mut Entity, victim: Option<i32>) -> Vec<i32> {
    let Some(state) = state_mut(&mut entity.data) else {
        return victim.into_iter().collect();
    };
    if let Some(victim) = victim {
        state.pierced_and_killed.push(victim);
    }
    state.pierced_and_killed.clone()
}

pub(crate) fn state_mut(data: &mut EntityData) -> Option<&mut AbstractArrowState> {
    match data {
        EntityData::Arrow(data) => Some(data),
        EntityData::SpectralArrow(data) => Some(data),
        EntityData::Trident(data) => Some(data),
        _ => None,
    }
}

/// `AbstractArrow.tick` (`AbstractArrow.java:174-272`) and the subtype
/// overrides in front of it.
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    // `ThrownTrident.tick` (`ThrownTrident.java:63-93`).
    if matches!(entity.data(), EntityData::Trident(_))
        && !crate::entity::trident::return_to_owner(entity, store, level)
    {
        return true;
    }
    let physics = !state(entity.data()).expect("arrow state").no_physics();
    entity.set_no_physics(!physics);
    let movement = entity.delta_movement();
    let block_pos = entity.block_position();
    let block_state = level.block_state(block_pos);
    if physics && !block_state_is_air(block_state) {
        let position = entity.position();
        let local = Vec3d {
            x: position.x - f64::from(block_pos.x),
            y: position.y - f64::from(block_pos.y),
            z: position.z - f64::from(block_pos.z),
        };
        let inside = if let Some(mut boxes) =
            bbb_server_types::geometry::static_block_collision_box_iter(block_state)
        {
            boxes.any(|shape| aabb_contains(shape, local))
        } else {
            level
                .block_collision_boxes(block_pos)
                .iter()
                .any(|shape| aabb_contains(*shape, local))
        };
        if inside {
            entity.set_delta_movement(Vec3d::default());
            set_in_ground(entity, true);
        }
    }
    let arrow = state_mut(&mut entity.data).expect("arrow state");
    if arrow.shake_time > 0 {
        arrow.shake_time -= 1;
    }
    if entity.was_touching_water() || level.is_raining_at(block_pos) {
        entity.clear_fire();
    }
    let arrow = state(entity.data()).expect("arrow state");
    if arrow.in_ground && physics {
        if arrow.in_block_state_id != Some(block_state) && should_fall(entity, level) {
            start_falling(entity, store);
        } else {
            // `ThrownTrident.tickDespawn` (`ThrownTrident.java:213-218`).
            let kept = matches!(entity.data(), EntityData::Trident(data) if data.loyalty() > 0)
                && arrow.pickup == ArrowPickup::Allowed;
            if !kept {
                let arrow = state_mut(&mut entity.data).expect("arrow state");
                arrow.life += 1;
                if arrow.life >= DESPAWN_TICKS {
                    entity.discard();
                }
            }
        }
        state_mut(&mut entity.data)
            .expect("arrow state")
            .in_ground_time += 1;
        if entity.is_alive()
            && !crate::entity::apply_nonliving_inside_block_effects(
                entity,
                level,
                Some(&mut *store),
            )
        {
            return false;
        }
        if matches!(entity.data(), EntityData::Arrow(_)) {
            decay_exposed_potion(entity, store);
        }
        // `AbstractArrow.tick` (`AbstractArrow.java:216-218`).
        entity.set_shared_flag_on_fire(entity.remaining_fire_ticks() > 0);
        return true;
    }

    state_mut(&mut entity.data)
        .expect("arrow state")
        .in_ground_time = 0;
    if entity.was_touching_water() {
        let inertia = match entity.data() {
            // `ThrownTrident.WATER_INERTIA` (`ThrownTrident.java:33`).
            EntityData::Trident(_) => f64::from(0.99_f32),
            _ => f64::from(WATER_INERTIA as f32),
        };
        entity.set_delta_movement(entity.delta_movement().scale(inertia));
    }
    let movement_now = entity.delta_movement();
    let (y_rot, x_rot) = rotation_of(movement_now, physics);
    entity.set_rot(y_rot, x_rot);
    let arrow = state(entity.data()).expect("arrow state");
    if !arrow.projectile.left_owner {
        let left_owner = crate::entity::throwable_projectile::outside_owner_collision_range(
            entity,
            arrow.projectile.owner,
            store,
            level,
        );
        state_mut(&mut entity.data)
            .expect("arrow state")
            .projectile
            .left_owner = left_owner;
    }
    if physics {
        let from = entity.position();
        let to = from.plus(movement);
        let block_hit = crate::entity::projectile_hit::clip_including_border(level, from, to);
        if !step_move_and_hit(entity, store, level, block_hit, to) {
            return false;
        }
    } else {
        let from = entity.position();
        entity.set_pos(from.plus(movement));
        store.refile_detached(entity);
        if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
            return false;
        }
    }
    if !entity.was_touching_water() {
        entity.set_delta_movement(entity.delta_movement().scale(INERTIA));
    }
    if physics && !state(entity.data()).expect("arrow state").in_ground {
        apply_gravity(entity);
    }
    // `Projectile.tick` (`Projectile.java:99-108`).
    let arrow = state(entity.data()).expect("arrow state");
    if !arrow.projectile.has_been_shot {
        level.game_event_at(
            VanillaGameEvent::PROJECTILE_SHOOT,
            entity.position(),
            crate::entity::throwable_projectile::owner_entity_id(
                arrow.projectile.owner,
                store,
                level,
            ),
        );
        state_mut(&mut entity.data)
            .expect("arrow state")
            .projectile
            .has_been_shot = true;
    }
    crate::entity::damage::base_tick_from_environment(entity, store, level)
}

/// `Mth.atan2` of the motion, reversed for a no-physics arrow
/// (`AbstractArrow.java:242-251`).
fn rotation_of(movement: Vec3d, physics: bool) -> (f32, f32) {
    use crate::entity::projectile_hit::mth_atan2;
    let y_rot = if physics {
        mth_atan2(movement.x, movement.z)
    } else {
        mth_atan2(-movement.x, -movement.z)
    };
    let horizontal = movement.horizontal_distance_sqr().sqrt();
    let x_rot = mth_atan2(movement.y, horizontal);
    (
        (y_rot * 180.0 / std::f64::consts::PI) as f32,
        (x_rot * 180.0 / std::f64::consts::PI) as f32,
    )
}

/// `AbstractArrow.shouldFall` (`AbstractArrow.java:343-345`).
fn should_fall(entity: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    let position = entity.position();
    let query = Aabb {
        min_x: position.x,
        min_y: position.y,
        min_z: position.z,
        max_x: position.x,
        max_y: position.y,
        max_z: position.z,
    }
    .inflate(0.06, 0.06, 0.06);
    no_block_collision_with_scratch(
        query,
        |pos| level.is_moving_piston_at(pos),
        |pos, out| {
            level.push_block_collision_boxes(pos, out);
            Some(())
        },
    )
    .unwrap_or(false)
}

/// `AbstractArrow.startFalling` (`AbstractArrow.java:347-352`).
fn start_falling(entity: &mut Entity, store: &mut EntityStore) {
    set_in_ground(entity, false);
    let random = store.random();
    let movement = entity.delta_movement();
    let (x, y, z) = (
        f64::from(random.next_float() * 0.2),
        f64::from(random.next_float() * 0.2),
        f64::from(random.next_float() * 0.2),
    );
    entity.set_delta_movement(movement.multiply(x, y, z));
    state_mut(&mut entity.data).expect("arrow state").life = 0;
}

fn set_in_ground(entity: &mut Entity, in_ground: bool) {
    let arrow = state_mut(&mut entity.data).expect("arrow state");
    let changed = arrow.in_ground != in_ground;
    arrow.set_in_ground(in_ground);
    if changed {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID);
    }
}

/// `Arrow.tick`'s potion decay (`Arrow.java:84-87`): an arrow stuck for
/// thirty seconds loses its potion.
fn decay_exposed_potion(entity: &mut Entity, store: &mut EntityStore) {
    let EntityData::Arrow(data) = entity.data() else {
        unreachable!("potion decay requires an arrow");
    };
    if data.in_ground_time < EXPOSED_POTION_DECAY_TIME || data.effect_color() == -1 {
        return;
    }
    crate::entity::throwable_projectile::broadcast_entity_event(entity, store, 0);
    let EntityData::Arrow(data) = &mut entity.data else {
        unreachable!("potion decay requires an arrow");
    };
    if let Some(plain) = crate::entity::item::plain_stack("minecraft:arrow", 1) {
        data.replace_slot_item(plain);
    }
    data.set_effect_color(-1);
    entity
        .mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_ARROW_EFFECT_COLOR_DATA_ID);
}

/// `AbstractArrow.stepMoveAndHit` (`AbstractArrow.java:274-302`).
fn step_move_and_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    block_hit: Option<(Vec3d, BlockPos, bbb_protocol::packets::Direction, bool)>,
    miss_end: Vec3d,
) -> bool {
    let end = block_hit.map_or(miss_end, |(location, ..)| location);
    while entity.is_alive() {
        let from = entity.position();
        let hits = find_hit_entities(entity, store, level, from, end);
        let next = hits.first().map_or(end, |hit| hit.1);
        entity.set_pos(next);
        store.refile_detached(entity);
        if !crate::entity::apply_nonliving_inside_block_effects_between(
            entity,
            level,
            Some(&mut *store),
            from,
            next,
        ) {
            return false;
        }
        if entity
            .portal_process()
            .is_some_and(|portal| portal.is_inside_this_tick())
            && !crate::entity::handle_portal(entity, level)
        {
            return false;
        }
        if hits.is_empty() {
            match block_hit.filter(|_| entity.is_alive()) {
                Some((.., true)) => {
                    crate::entity::projectile_hit::bounce_off_world_border(entity, store);
                }
                Some((location, pos, direction, false)) => {
                    on_hit_block(entity, store, level, location, pos, direction);
                    entity.set_needs_sync(true);
                }
                None => {}
            }
            break;
        }
        if !entity.is_alive() || state(entity.data()).expect("arrow state").no_physics() {
            break;
        }
        let mut deflected = false;
        for (target, location, _) in hits {
            let deflection = deflection_of(entity, store, level, target);
            if deflection != ProjectileDeflection::None {
                crate::entity::hurting_projectile::apply_target_deflection(
                    entity, store, target, deflection,
                );
                deflected = true;
                break;
            }
            on_hit_entity(entity, store, level, target, location);
            if !entity.is_alive()
                || state(entity.data()).expect("arrow state").pierce_level <= 0
                || matches!(entity.data(), EntityData::Trident(_))
            {
                break;
            }
        }
        entity.set_needs_sync(true);
        if state(entity.data()).expect("arrow state").pierce_level <= 0 || deflected {
            break;
        }
    }
    true
}

fn deflection_of(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    target: ProjectileTarget,
) -> ProjectileDeflection {
    crate::entity::projectile_hit::target_deflection(entity, store, level, target)
        .unwrap_or(ProjectileDeflection::None)
}

/// `AbstractArrow.findHitEntities` (`AbstractArrow.java:586-600`) over
/// `canHitEntity` (`:471-475`); a trident finds the nearest one and nothing
/// once it dealt damage (`ThrownTrident.java:108-110`).
fn find_hit_entities(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    from: Vec3d,
    to: Vec3d,
) -> Vec<(ProjectileTarget, Vec3d, Vec3d)> {
    let trident = match entity.data() {
        EntityData::Trident(data) => {
            if data.dealt_damage() {
                return Vec::new();
            }
            true
        }
        _ => false,
    };
    let arrow = state(entity.data()).expect("arrow state");
    let owner = arrow.projectile.owner;
    let owner_vehicle = owner_vehicle(owner, arrow.projectile.left_owner, store, level);
    let owner_player = owner.and_then(|owner| level.player(owner));
    let mut hits = Vec::new();
    let mut nearest = None;
    for_each_candidate_hit(
        store,
        level,
        entity,
        from,
        to,
        |candidate, store, level| {
            candidate.is_alive_and_pickable_in_level(level)
                && !matches!((owner_player, candidate), (Some(owner), ProjectileCandidate::Player(target))
                    if !can_harm_player(store, level, owner, target))
                && !rides_with_owner(candidate, owner_vehicle, store)
                && !arrow
                    .piercing_ignore
                    .contains(&candidate_entity_id(candidate))
        },
        true,
        |target, location, position, section| {
            let key = (section_walk_order_key(section), target.entity_id());
            let hit = (target, location, position);
            if trident {
                let dd = from.distance_to_sqr(location);
                if nearest.as_ref().is_none_or(|(distance, previous, _)| {
                    dd.total_cmp(distance)
                        .then_with(|| key.cmp(previous))
                        .is_lt()
                }) {
                    nearest = Some((dd, key, hit));
                }
            } else {
                hits.push((key, hit));
            }
        },
    );
    if trident {
        return nearest.into_iter().map(|(_, _, hit)| hit).collect();
    }
    hits.sort_by(|(left_key, left), (right_key, right)| {
        from.distance_to_sqr(left.2)
            .total_cmp(&from.distance_to_sqr(right.2))
            .then_with(|| left_key.cmp(right_key))
    });
    hits.into_iter().map(|(_, hit)| hit).collect()
}

/// `ServerPlayer.canHarmPlayer` (`ServerPlayer.java:987-988`) over
/// `Player.canHarmPlayer` (`Player.java:727-735`).
fn can_harm_player(
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    owner: NearbyPlayer,
    target: &NearbyPlayer,
) -> bool {
    store.target_level_facts().pvp
        && match level.player_team(store, owner.uuid) {
            Some(team) if Some(team) == level.player_team(store, target.uuid) => {
                level.scoreboard_teams().allows_friendly_fire(team)
            }
            _ => true,
        }
}

fn candidate_entity_id(candidate: ProjectileCandidate<'_>) -> i32 {
    candidate.target().entity_id()
}

/// `AbstractArrow.onHitBlock` (`AbstractArrow.java:528-552`).
fn on_hit_block(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    location: Vec3d,
    pos: BlockPos,
    direction: bbb_protocol::packets::Direction,
) {
    state_mut(&mut entity.data)
        .expect("arrow state")
        .in_block_state_id = Some(level.block_state(pos));
    crate::block_behavior::projectile_hit::on_projectile_hit(
        level,
        store,
        entity,
        crate::block_behavior::projectile_hit::ProjectileBlockHit {
            location,
            pos,
            direction,
        },
    );
    let block_state = level.block_state(pos);
    // `ThrownTrident.getWeaponItem` is the trident itself.
    let arrow = state(entity.data()).expect("arrow state");
    let weapon = if matches!(entity.data(), EntityData::Trident(_)) {
        Some(&arrow.item)
    } else {
        arrow.weapon.as_ref()
    };
    if let Some(weapon) = weapon.filter(|weapon| !weapon.is_empty()) {
        let clamp = |value: f64, min: i32| {
            value.clamp(
                f64::from(min) + f64::from(1.0e-5_f32),
                f64::from(min) + 1.0 - f64::from(1.0e-5_f32),
            )
        };
        store.queue_command(EntityCommand::Projectile(
            PendingProjectileAction::HitBlockEffects(Box::new(PendingProjectileHitBlock {
                projectile_entity_id: entity.id(),
                weapon: weapon.clone(),
                owner: arrow.projectile.owner,
                position: Vec3d {
                    x: clamp(location.x, pos.x),
                    y: clamp(location.y, pos.y),
                    z: clamp(location.z, pos.z),
                },
                block_state_id: block_state,
            })),
        ));
    }
    level.game_event_with_state(
        VanillaGameEvent::PROJECTILE_LAND,
        pos,
        Some(entity.id()),
        Some(block_state),
    );
    let movement = entity.delta_movement();
    let back = Vec3d {
        x: signum_or_zero(movement.x),
        y: signum_or_zero(movement.y),
        z: signum_or_zero(movement.z),
    }
    .scale(f64::from(0.05_f32));
    entity.set_pos(Vec3d {
        x: location.x - back.x,
        y: location.y - back.y,
        z: location.z - back.z,
    });
    store.refile_detached(entity);
    entity.set_delta_movement(Vec3d::default());
    let pitch = arrow_sound_pitch(store);
    let arrow = state(entity.data()).expect("arrow state");
    play_sound(entity, store, &arrow.sound_event_resource_id, pitch);
    set_in_ground(entity, true);
    let default_ground_sound = if matches!(entity.data(), EntityData::Trident(_)) {
        "minecraft:item.trident.hit_ground"
    } else {
        "minecraft:entity.arrow.hit"
    };
    let arrow = state_mut(&mut entity.data).expect("arrow state");
    arrow.shake_time = SHAKE_TIME;
    arrow.set_crit(false);
    arrow.pierce_level = 0;
    arrow.sound_event_resource_id = default_ground_sound.to_string();
    arrow.piercing_ignore.clear();
    arrow.pierced_and_killed.clear();
    entity.mark_synched_dirty(VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID);
    entity.mark_synched_dirty(VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID);
}

/// `Math.signum`, which answers zero for zero.
fn signum_or_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value.signum()
    }
}

/// `1.2F / (random.nextFloat() * 0.2F + 0.9F)` (`AbstractArrow.java:496`, `:541`).
fn arrow_sound_pitch(store: &mut EntityStore) -> f32 {
    1.2 / (store.random().next_float() * 0.2 + 0.9)
}

fn play_sound(entity: &Entity, store: &mut EntityStore, resource_id: &str, pitch: f32) {
    if entity.is_silent() {
        return;
    }
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_string(),
        source: SoundSource::Neutral,
        volume: 1.0,
        pitch,
    });
}

/// `AbstractArrow.onHitEntity` (`AbstractArrow.java:419-505`) and
/// `ThrownTrident.onHitEntity` (`ThrownTrident.java:119-144`). The hurt and
/// what follows from its outcome are queued; the world runs them once it can
/// read whether the target took the hit.
fn on_hit_entity(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target: ProjectileTarget,
    location: Vec3d,
) {
    let owner = state(entity.data()).expect("arrow state").projectile.owner;
    // `Projectile.onHit` (`Projectile.java:295-297`) aims a redirectable target first.
    if let Ok(Some(action)) = crate::entity::hurting_projectile::projectile_deflection_action(
        target, owner, store, &*level,
    ) {
        store.queue_command(EntityCommand::Projectile(action));
    }
    level.game_event_at(
        VanillaGameEvent::PROJECTILE_LAND,
        location,
        Some(entity.id()),
    );
    let causing = pending_owner(owner, store, level);
    let enderman = matches!(target, ProjectileTarget::Stored { entity_id }
    if store.get(entity_id).is_some_and(|target| {
        target.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID
    }));
    if let EntityData::Trident(data) = &mut entity.data {
        data.mark_dealt_damage();
        let item = data.item().clone();
        queue(
            store,
            PendingArrowHit {
                hurt: hurt(entity, level, target, "minecraft:trident", causing),
                damage: ArrowDamage {
                    base: TRIDENT_DAMAGE,
                    speed: None,
                    crit: false,
                    weapon: Some(item),
                    movement: entity.delta_movement(),
                },
                arrow_entity_id: entity.id(),
                ignites: false,
                enderman,
                sticks: false,
                effects: Vec::new(),
                owner,
                hit_sound: None,
                discard_on_success: false,
                refusal: ArrowRefusal::Ignore,
                piercing: false,
                trident: true,
            },
        );
        reverse(entity, store);
        entity.set_delta_movement(entity.delta_movement().multiply(0.02, 0.2, 0.02));
        play_sound(entity, store, "minecraft:item.trident.hit", 1.0);
        return;
    }
    let arrow = state_mut(&mut entity.data).expect("arrow state");
    if arrow.pierce_level > 0 {
        if arrow.piercing_ignore.len() > arrow.pierce_level as usize {
            entity.discard();
            return;
        }
        let entity_id = crate::entity::ender_dragon::part_by_id(store, target.entity_id())
            .map_or(target.entity_id(), |(dragon_id, _, _)| dragon_id);
        arrow.piercing_ignore.push(entity_id);
    }
    let effects = match entity.data() {
        EntityData::Arrow(data) => crate::entity::thrown_potion::potion_mob_effects(data.item()),
        // `SpectralArrow.doPostHurtEffects` (`SpectralArrow.java:43-47`).
        EntityData::SpectralArrow(data) => {
            ActiveMobEffect::new("minecraft:glowing", data.duration(), 0)
                .into_iter()
                .collect()
        }
        _ => unreachable!("arrow effects require an arrow or spectral arrow"),
    };
    let arrow = state(entity.data()).expect("arrow state");
    let pickup = (arrow.pickup == ArrowPickup::Allowed).then(|| arrow.item.clone());
    let pitch = arrow_sound_pitch(store);
    let hit = PendingArrowHit {
        hurt: hurt(entity, level, target, "minecraft:arrow", causing),
        damage: ArrowDamage {
            base: arrow.damage as f32,
            speed: Some(entity.delta_movement().length_sqr().sqrt() as f32),
            crit: arrow.crit(),
            weapon: arrow.weapon.clone(),
            movement: entity.delta_movement(),
        },
        arrow_entity_id: entity.id(),
        ignites: entity.remaining_fire_ticks() > 0 && !enderman,
        enderman,
        sticks: arrow.pierce_level <= 0,
        effects,
        owner,
        hit_sound: Some((arrow.sound_event_resource_id.clone(), pitch)),
        discard_on_success: arrow.pierce_level <= 0,
        refusal: ArrowRefusal::Bounce { pickup },
        piercing: arrow.pierce_level > 0,
        trident: false,
    };
    queue(store, hit);
}

fn hurt(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    target: ProjectileTarget,
    damage_type: &'static str,
    causing: Option<PendingHurtEntity>,
) -> PendingProjectileHurt {
    PendingProjectileHurt {
        target: pending_target(target),
        damage_type,
        amount: 0.0,
        // `damageSources().arrow(this, owner != null ? owner : this)`.
        causing: causing.or(Some(PendingHurtEntity::Stored {
            entity_id: entity.id(),
        })),
        direct: Some(PendingHurtEntity::Stored {
            entity_id: entity.id(),
        }),
        direct_resource_id: entity.data().entity_resource_id(),
        registry_tags: level.capture_registry_tags(),
    }
}

fn queue(store: &mut EntityStore, hit: PendingArrowHit) {
    store.queue_command(EntityCommand::Projectile(
        PendingProjectileAction::ArrowHit(Box::new(hit)),
    ));
}

/// `ProjectileDeflection.REVERSE` (`ProjectileDeflection.java:11-17`).
pub(crate) fn reverse(entity: &mut Entity, store: &mut EntityStore) {
    let rotation = 170.0 + store.random().next_float() * 20.0;
    entity.set_delta_movement(entity.delta_movement().scale(-0.5));
    entity.set_rot(entity.y_rot() + rotation, entity.x_rot());
    entity.set_needs_sync(true);
}

/// `AbstractArrow.playerTouch` (`AbstractArrow.java:643-658`) and the
/// trident's owner rule (`ThrownTrident.java:187-191`): what a player
/// touching the arrow takes, if anything. `creative` stands for
/// `hasInfiniteMaterials`.
pub(crate) fn player_touch(entity: &Entity, player: Uuid, creative: bool) -> Option<ArrowTouch> {
    let state = state(entity.data())?;
    if !(state.in_ground || state.no_physics()) || state.shake_time > 0 {
        return None;
    }
    let trident = matches!(entity.data(), EntityData::Trident(_));
    let owned = state.projectile.owner == Some(player);
    if trident && !(owned || state.projectile.owner.is_none()) {
        return None;
    }
    match state.pickup {
        ArrowPickup::Allowed => Some(ArrowTouch::Take(state.item.clone())),
        ArrowPickup::CreativeOnly if creative => Some(ArrowTouch::Discard),
        // `ThrownTrident.tryPickup` (`ThrownTrident.java:172-174`).
        _ if trident && state.no_physics() && owned => Some(ArrowTouch::Take(state.item.clone())),
        _ => None,
    }
}

pub(crate) enum ArrowTouch {
    /// `player.getInventory().add(getPickupItem())`.
    Take(RuntimeItemStack),
    /// A creative player takes nothing but still collects the arrow.
    Discard,
}
