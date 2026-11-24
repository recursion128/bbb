//! `FishingHook`: the `minecraft:fishing_bobber` a fishing rod throws.

use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_types::VANILLA_BLOCK_LILY_PAD_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_FISHING_HOOK_BITING_DATA_ID, VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::EntityEvent;
use bbb_protocol::packets::{
    BlockPos, EntityDataValue, EntityDataValueKind, LevelParticles, ParticlePayload, SoundSource,
    Vec3d,
};
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_BUBBLE_ID, VANILLA_PARTICLE_TYPE_FISHING_ID,
    VANILLA_PARTICLE_TYPE_SPLASH_ID,
};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::experience_orb::spawn_experience_orb;
use crate::entity::hurting_projectile;
use crate::entity::item::spawn_item_entity;
use crate::entity::level::{fluid_state_tag_membership, NearbyPlayer};
use crate::entity::living::SynchedValue;
use crate::entity::projectile_action::PendingProjectileAction;
use crate::entity::projectile_hit::{
    hit_result_on_move_vector, mth_atan2, HitResult, ProjectileCandidate,
};
use crate::entity::store::{EntityBodyReadView, EntityStore, PendingEntitySound};
use crate::entity::{move_entity, Entity, EntityData, MoverType};
use crate::fluid::above;
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::{LootBiomeAt, LootTool};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

/// `Items.FISHING_ROD` (`Items.java:1430`) as a built-in item type id.
const FISHING_ROD_ITEM_ID: i32 = bbb_protocol::item_types::VANILLA_ITEM_FISHING_ROD_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishHookState {
    Flying,
    HookedInEntity,
    Bobbing,
}

/// The `FishingHook` and `Projectile` fields this slice carries.
///
#[derive(Debug, Clone, PartialEq)]
pub struct FishingBobberData {
    owner: Option<uuid::Uuid>,
    has_been_shot: bool,
    biting: bool,
    out_of_water_time: i32,
    life: i32,
    nibble: i32,
    time_until_lured: i32,
    time_until_hooked: i32,
    fish_angle: f32,
    open_water: bool,
    hooked_entity_id: Option<i32>,
    current_state: FishHookState,
    luck: i32,
    lure_speed: i32,
}

impl FishingBobberData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        let owner = self.owner;
        *self = previous.clone();
        self.owner = owner;
    }

    pub fn new(owner: uuid::Uuid, luck: i32, lure_speed: i32) -> Self {
        Self {
            owner: Some(owner),
            has_been_shot: false,
            biting: false,
            out_of_water_time: 0,
            life: 0,
            nibble: 0,
            time_until_lured: 0,
            time_until_hooked: 0,
            fish_angle: 0.0,
            open_water: true,
            hooked_entity_id: None,
            current_state: FishHookState::Flying,
            luck: luck.max(0),
            lure_speed: lure_speed.max(0),
        }
    }

    pub const fn owner(&self) -> Option<uuid::Uuid> {
        self.owner
    }

    pub const fn has_been_shot(&self) -> bool {
        self.has_been_shot
    }

    pub(crate) fn set_owner(&mut self, owner: Option<uuid::Uuid>) {
        self.owner = owner;
    }

    pub const fn current_state(&self) -> FishHookState {
        self.current_state
    }

    pub const fn biting(&self) -> bool {
        self.biting
    }

    pub const fn nibble(&self) -> i32 {
        self.nibble
    }

    pub const fn time_until_lured(&self) -> i32 {
        self.time_until_lured
    }

    pub const fn time_until_hooked(&self) -> i32 {
        self.time_until_hooked
    }

    pub const fn hooked_entity_id(&self) -> Option<i32> {
        self.hooked_entity_id
    }

    pub const fn open_water(&self) -> bool {
        self.open_water
    }

    /// The `DATA_HOOKED_ENTITY` int (`FishingHook.java:291`): `0` for none.
    const fn data_hooked_entity(&self) -> i32 {
        match self.hooked_entity_id {
            Some(id) => id + 1,
            None => 0,
        }
    }
}

/// `new FishingHook(player, level, luck, lureSpeed)` (`FishingHook.java:81-106`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_fishing_bobber(
    store: &mut EntityStore,
    owner: uuid::Uuid,
    owner_position: Vec3d,
    owner_eye_y: f64,
    owner_y_rot: f32,
    owner_x_rot: f32,
    luck: i32,
    lure_speed: i32,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let y_cos =
        mth_cos(-f64::from(owner_y_rot) * (std::f64::consts::PI / 180.0) - std::f64::consts::PI);
    let y_sin =
        mth_sin(-f64::from(owner_y_rot) * (std::f64::consts::PI / 180.0) - std::f64::consts::PI);
    let x_cos = -mth_cos(-f64::from(owner_x_rot) * (std::f64::consts::PI / 180.0));
    let x_sin = mth_sin(-f64::from(owner_x_rot) * (std::f64::consts::PI / 180.0));
    let x1 = owner_position.x - f64::from(y_sin) * 0.3;
    let y1 = owner_eye_y;
    let z1 = owner_position.z - f64::from(y_cos) * 0.3;
    let mut movement = Vec3d {
        x: f64::from(-y_sin),
        y: f64::from((-(x_sin / x_cos)).clamp(-5.0, 5.0)),
        z: f64::from(-y_cos),
    };
    let dist = movement.length_sqr().sqrt();
    movement = movement.multiply(
        0.6 / dist + triangle(store, 0.5, 0.0103365),
        0.6 / dist + triangle(store, 0.5, 0.0103365),
        0.6 / dist + triangle(store, 0.5, 0.0103365),
    );
    let y_rot = mth_atan2(movement.x, movement.z) as f32 * 180.0 / std::f32::consts::PI;
    let x_rot = mth_atan2(movement.y, movement.horizontal_distance_sqr().sqrt()) as f32 * 180.0
        / std::f32::consts::PI;
    let id = store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        Vec3d {
            x: x1,
            y: y1,
            z: z1,
        },
        EntityData::FishingBobber(FishingBobberData::new(owner, luck, lure_speed)),
        |entity| {
            entity.set_delta_movement(movement);
            entity.set_rot(y_rot, x_rot);
        },
    )?;
    Some(id)
}

fn triangle(store: &mut EntityStore, mean: f64, spread: f64) -> f64 {
    mean + spread * (store.random().next_double() - store.random().next_double())
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if !matches!(entity.data(), EntityData::FishingBobber(_)) {
        return true;
    }

    if !fishing_data(entity).has_been_shot {
        let owner_id = fishing_data(entity)
            .owner
            .and_then(|owner| level.player(owner).map(|player| player.entity_id));
        level.game_event_at(
            VanillaGameEvent::PROJECTILE_SHOOT,
            entity.position(),
            owner_id,
        );
        fishing_data_mut(entity).has_been_shot = true;
    }
    if !crate::entity::damage::base_tick_from_environment(entity, store, level) {
        return false;
    }
    if entity.removal_reason().is_some() {
        return true;
    }

    let owner = fishing_data(entity)
        .owner
        .and_then(|owner| level.player(owner));
    let Some(owner) = owner else {
        entity.discard();
        return true;
    };
    if should_stop_fishing(entity, &owner) {
        return true;
    }

    if entity.on_ground() {
        fishing_data_mut(entity).life += 1;
        if fishing_data(entity).life >= 1200 {
            entity.discard();
            return true;
        }
    } else {
        fishing_data_mut(entity).life = 0;
    }

    let block_pos = entity.block_position();
    let fluid = block_state_fluid_state(level.block_state(block_pos));
    let water_membership = fluid_state_tag_membership(level, fluid, "minecraft:water");
    let liquid_height = if water_membership {
        water_fluid_height(level, fluid, block_pos)
    } else {
        0.0
    };
    let is_in_water = liquid_height > 0.0;
    let collision = if fishing_data(entity).current_state == FishHookState::Flying
        && fishing_data(entity).hooked_entity_id.is_none()
        && !is_in_water
    {
        let owner = fishing_data(entity).owner;
        let Some(hit) = hit_result_on_move_vector(entity, store, &*level, |candidate, _, level| {
            hook_can_hit_entity(candidate, level, owner)
        }) else {
            return false;
        };
        let redirect_action = match hit {
            HitResult::Entity {
                target,
                deflection: crate::entity::projectile_hit::ProjectileDeflection::None,
                ..
            } => {
                let Ok(action) = hurting_projectile::projectile_deflection_action(
                    target,
                    fishing_data(entity).owner,
                    store,
                    &*level,
                ) else {
                    return false;
                };
                action
            }
            _ => None,
        };
        Some((hit, redirect_action))
    } else {
        None
    };
    let open_water = if fishing_data(entity).current_state == FishHookState::Bobbing
        && (fishing_data(entity).nibble > 0 || fishing_data(entity).time_until_hooked > 0)
        && fishing_data(entity).open_water
        && fishing_data(entity).out_of_water_time < 10
    {
        Some(calculate_open_water(level, block_pos))
    } else if fishing_data(entity).current_state == FishHookState::Bobbing
        && (fishing_data(entity).nibble > 0 || fishing_data(entity).time_until_hooked > 0)
    {
        Some(false)
    } else {
        None
    };

    let mut returned = false;
    if fishing_data(entity).current_state == FishHookState::Flying {
        if fishing_data(entity).hooked_entity_id.is_some() {
            entity.set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
            fishing_data_mut(entity).current_state = FishHookState::HookedInEntity;
            returned = true;
        } else if is_in_water {
            entity.set_delta_movement(entity.delta_movement().multiply(0.3, 0.2, 0.3));
            fishing_data_mut(entity).current_state = FishHookState::Bobbing;
            returned = true;
        } else {
            let (hit, redirect_action) =
                collision.expect("the flying dry hook resolved collision before mutation");
            apply_collision(entity, store, level, hit, redirect_action);
        }
    } else if fishing_data(entity).current_state == FishHookState::HookedInEntity {
        if let Some(hooked_id) = fishing_data(entity).hooked_entity_id {
            // `Entity.canInteractWithLevel` (`FishingHook.java:191`) lets go of a dying mob.
            let hooked_position = store
                .get(hooked_id)
                .filter(|hooked| hooked.is_alive_with_health())
                .or_else(|| {
                    store
                        .player(hooked_id)
                        .filter(|player| player.is_alive() && !player.is_spectator())
                        .map(crate::world::PlayerState::body)
                })
                .map(|hooked| Vec3d {
                    y: hooked.position().y
                        + (hooked.bounding_box().max_y - hooked.bounding_box().min_y) * 0.8,
                    ..hooked.position()
                })
                .or_else(|| {
                    let (dragon_id, _, bounds) =
                        crate::entity::ender_dragon::part_by_id(store, hooked_id)?;
                    let dragon = store.get(dragon_id)?;
                    if !dragon.is_alive()
                        || !store
                            .section_visibility(dragon.filed_section())
                            .is_accessible()
                    {
                        return None;
                    }
                    Some(Vec3d {
                        y: bounds.min_y + (bounds.max_y - bounds.min_y) * 0.8,
                        ..bounds.center()
                    })
                });
            if let Some(position) = hooked_position {
                entity.set_pos(position);
            } else {
                set_hooked_entity(entity, None);
                fishing_data_mut(entity).current_state = FishHookState::Flying;
            }
        }
        returned = true;
    } else if fishing_data(entity).current_state == FishHookState::Bobbing {
        bobbing_physics(
            entity,
            store,
            level,
            block_pos,
            liquid_height,
            is_in_water,
            open_water,
        );
    }

    if returned {
        return true;
    }

    if !water_membership && !entity.on_ground() && fishing_data(entity).hooked_entity_id.is_none() {
        entity.set_delta_movement(entity.delta_movement().plus(Vec3d {
            x: 0.0,
            y: -0.03,
            z: 0.0,
        }));
    }

    let delta = entity.delta_movement();
    move_entity(entity, store, level, MoverType::Self_, delta);
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
        return false;
    }
    update_rotation(entity);
    if fishing_data(entity).current_state == FishHookState::Flying
        && (entity.on_ground() || entity.horizontal_collision())
    {
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    }
    entity.set_delta_movement(entity.delta_movement().scale(0.92));
    // `reapplyPosition` (`:246`) only re-derives the bounding box from the
    // position, which `set_pos`/`move_entity` already keep in sync.
    true
}

fn should_stop_fishing(entity: &mut Entity, owner: &NearbyPlayer) -> bool {
    if can_continue_fishing(entity, owner) {
        return false;
    }
    entity.discard();
    true
}

fn can_continue_fishing(entity: &Entity, owner: &NearbyPlayer) -> bool {
    let can_interact = !owner.dead_or_dying && !owner.spectator;
    let main = owner.main_hand_item_id == Some(FISHING_ROD_ITEM_ID);
    let off = owner.off_hand_item_id == Some(FISHING_ROD_ITEM_ID);
    can_interact && (main || off) && entity.position().distance_to_sqr(owner.position) <= 1024.0
}

fn apply_collision(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    hit: HitResult,
    redirect_action: Option<PendingProjectileAction>,
) {
    match hit {
        HitResult::Entity {
            target,
            location,
            deflection,
            ..
        } => {
            if hurting_projectile::apply_target_deflection(entity, store, target, deflection) {
                return;
            }
            if let Some(action) = redirect_action {
                store.queue_command(crate::entity::EntityCommand::Projectile(action));
            }
            set_hooked_entity(entity, Some(target.entity_id()));
            level.game_event_at(
                VanillaGameEvent::PROJECTILE_LAND,
                location,
                Some(entity.id()),
            );
        }
        HitResult::Block {
            world_border: true, ..
        } => crate::entity::projectile_hit::bounce_off_world_border(entity, store),
        HitResult::Block {
            location,
            pos,
            direction,
            ..
        } => {
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
            let distance = hit_result_distance_to(location, entity.position());
            entity.set_delta_movement(entity.delta_movement().normalize().scale(distance));
            let state = level.block_state(pos);
            level.game_event_with_state(
                VanillaGameEvent::PROJECTILE_LAND,
                pos,
                Some(entity.id()),
                Some(state),
            );
        }
        HitResult::Miss => {}
    }
}

fn hit_result_distance_to(location: Vec3d, entity_position: Vec3d) -> f64 {
    location.distance_to_sqr(entity_position)
}

/// `FishingHook.canHitEntity` (`FishingHook.java:271-273`). The caster is
/// never a candidate: the hook keeps no `leftOwner` latch.
fn hook_can_hit_entity(
    candidate: ProjectileCandidate<'_>,
    level: &LevelCtx<'_, '_>,
    owner: Option<uuid::Uuid>,
) -> bool {
    match candidate {
        ProjectileCandidate::Stored(entity) if !entity.is_alive() => false,
        ProjectileCandidate::Stored(entity) if matches!(entity.data(), EntityData::Item(_)) => true,
        ProjectileCandidate::Stored(_) => candidate.is_alive_and_pickable_in_level(level),
        ProjectileCandidate::Player(player) => {
            Some(player.uuid) != owner && candidate.is_alive_and_pickable_in_level(level)
        }
    }
}

/// `FishingHook.setHookedEntity` (`FishingHook.java:289-292`).
fn set_hooked_entity(entity: &mut Entity, hooked: Option<i32>) {
    if fishing_data(entity).hooked_entity_id != hooked {
        fishing_data_mut(entity).hooked_entity_id = hooked;
        entity.mark_synched_dirty(VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID);
    }
}

fn update_rotation(entity: &mut Entity) {
    let movement = entity.delta_movement();
    let sd = movement.horizontal_distance_sqr().sqrt();
    let x_rot = lerp_rotation(
        entity.x_rot(),
        mth_atan2(movement.y, sd) as f32 * 180.0 / std::f32::consts::PI,
    );
    let y_rot = lerp_rotation(
        entity.y_rot(),
        mth_atan2(movement.x, movement.z) as f32 * 180.0 / std::f32::consts::PI,
    );
    entity.set_rot(y_rot, x_rot);
}

fn lerp_rotation(rot_o: f32, rot: f32) -> f32 {
    if !rot.is_finite() {
        return 0.0;
    }
    if !rot_o.is_finite() {
        return rot;
    }
    let delta = ((f64::from(rot) - f64::from(rot_o) + 180.0).rem_euclid(360.0) - 180.0) as f32;
    (rot - delta) + 0.2 * delta
}

#[allow(clippy::too_many_arguments)]
fn bobbing_physics(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    block_pos: BlockPos,
    liquid_height: f32,
    is_in_water: bool,
    open_water: Option<bool>,
) {
    let movement = entity.delta_movement();
    let mut force =
        entity.position().y + movement.y - f64::from(block_pos.y) - f64::from(liquid_height);
    if force.abs() < 0.01 {
        force += force.signum() * 0.1;
    }
    entity.set_delta_movement(Vec3d {
        x: movement.x * 0.9,
        y: movement.y - force * f64::from(store.random().next_float()) * 0.2,
        z: movement.z * 0.9,
    });
    if fishing_data(entity).nibble <= 0 && fishing_data(entity).time_until_hooked <= 0 {
        fishing_data_mut(entity).open_water = true;
    } else if let Some(open_water) = open_water {
        fishing_data_mut(entity).open_water = fishing_data(entity).open_water
            && fishing_data(entity).out_of_water_time < 10
            && open_water;
    }

    if is_in_water {
        fishing_data_mut(entity).out_of_water_time =
            (fishing_data(entity).out_of_water_time - 1).max(0);
        if fishing_data(entity).biting {
            let bump = -0.1
                * f64::from(store.random().next_float())
                * f64::from(store.random().next_float());
            entity.set_delta_movement(entity.delta_movement().plus(Vec3d {
                x: 0.0,
                y: bump,
                z: 0.0,
            }));
        }
        catching_fish(entity, store, level, block_pos);
    } else {
        fishing_data_mut(entity).out_of_water_time =
            (fishing_data(entity).out_of_water_time + 1).min(10);
    }
}

fn catching_fish(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    block_pos: BlockPos,
) {
    let mut fishing_speed = 1;
    let above_pos = above(block_pos);
    if store.random().next_float() < 0.25 && level.is_raining_at(above_pos) {
        fishing_speed += 1;
    }
    if store.random().next_float() < 0.5 && !level.can_see_sky(above_pos) {
        fishing_speed -= 1;
    }

    if fishing_data(entity).nibble > 0 {
        fishing_data_mut(entity).nibble -= 1;
        if fishing_data(entity).nibble <= 0 {
            fishing_data_mut(entity).time_until_lured = 0;
            fishing_data_mut(entity).time_until_hooked = 0;
            set_biting(entity, store.random(), false);
        }
    } else if fishing_data(entity).time_until_hooked > 0 {
        fishing_data_mut(entity).time_until_hooked -= fishing_speed;
        if fishing_data(entity).time_until_hooked > 0 {
            fishing_data_mut(entity).fish_angle += triangle(store, 0.0, 9.188) as f32;
            let angle = fishing_data(entity).fish_angle * (std::f32::consts::PI / 180.0);
            let angle_sin = mth_sin(f64::from(angle));
            let angle_cos = mth_cos(f64::from(angle));
            let fish_x = entity.position().x
                + f64::from(angle_sin) * f64::from(fishing_data(entity).time_until_hooked) * 0.1;
            let fish_y = entity.position().y.floor() + 1.0;
            let fish_z = entity.position().z
                + f64::from(angle_cos) * f64::from(fishing_data(entity).time_until_hooked) * 0.1;
            if splash_block_is_water(level, fish_x, fish_y, fish_z) {
                if store.random().next_float() < 0.15 {
                    send_particles(
                        store,
                        VANILLA_PARTICLE_TYPE_BUBBLE_ID,
                        fish_x,
                        fish_y - 0.1,
                        fish_z,
                        1,
                        f64::from(angle_sin),
                        0.1,
                        f64::from(angle_cos),
                        0.0,
                    );
                }
                let particle_x_movement = angle_sin * 0.04;
                let particle_z_movement = angle_cos * 0.04;
                send_particles(
                    store,
                    VANILLA_PARTICLE_TYPE_FISHING_ID,
                    fish_x,
                    fish_y,
                    fish_z,
                    0,
                    f64::from(particle_z_movement),
                    0.01,
                    f64::from(-particle_x_movement),
                    1.0,
                );
                send_particles(
                    store,
                    VANILLA_PARTICLE_TYPE_FISHING_ID,
                    fish_x,
                    fish_y,
                    fish_z,
                    0,
                    f64::from(-particle_z_movement),
                    0.01,
                    f64::from(particle_x_movement),
                    1.0,
                );
            }
        } else {
            let pitch = 1.0 + (store.random().next_float() - store.random().next_float()) * 0.4;
            play_sound(
                entity,
                store,
                "minecraft:entity.fishing_bobber.splash",
                0.25,
                pitch,
            );
            let y = entity.position().y + 0.5;
            let width = f64::from(entity_width(entity));
            let count = (1.0 + width * 20.0) as i32;
            send_particles(
                store,
                VANILLA_PARTICLE_TYPE_BUBBLE_ID,
                entity.position().x,
                y,
                entity.position().z,
                count,
                width,
                0.0,
                width,
                0.2,
            );
            send_particles(
                store,
                VANILLA_PARTICLE_TYPE_FISHING_ID,
                entity.position().x,
                y,
                entity.position().z,
                count,
                width,
                0.0,
                width,
                0.2,
            );
            fishing_data_mut(entity).nibble = mth_next_int(store, 20, 40);
            set_biting(entity, store.random(), true);
        }
    } else if fishing_data(entity).time_until_lured > 0 {
        fishing_data_mut(entity).time_until_lured -= fishing_speed;
        let mut tease_chance = 0.15_f32;
        if fishing_data(entity).time_until_lured < 20 {
            tease_chance += (20 - fishing_data(entity).time_until_lured) as f32 * 0.05;
        } else if fishing_data(entity).time_until_lured < 40 {
            tease_chance += (40 - fishing_data(entity).time_until_lured) as f32 * 0.02;
        } else if fishing_data(entity).time_until_lured < 60 {
            tease_chance += (60 - fishing_data(entity).time_until_lured) as f32 * 0.01;
        }
        if store.random().next_float() < tease_chance {
            let angle = mth_next_float(store, 0.0, 360.0) * (std::f32::consts::PI / 180.0);
            let dist = mth_next_float(store, 25.0, 60.0);
            let fish_x =
                entity.position().x + f64::from(mth_sin(f64::from(angle))) * f64::from(dist) * 0.1;
            let fish_y = entity.position().y.floor() + 1.0;
            let fish_z =
                entity.position().z + f64::from(mth_cos(f64::from(angle))) * f64::from(dist) * 0.1;
            if splash_block_is_water(level, fish_x, fish_y, fish_z) {
                let count = 2 + store.random().next_int(2);
                send_particles(
                    store,
                    VANILLA_PARTICLE_TYPE_SPLASH_ID,
                    fish_x,
                    fish_y,
                    fish_z,
                    count,
                    0.1,
                    0.0,
                    0.1,
                    0.0,
                );
            }
        }
        if fishing_data(entity).time_until_lured <= 0 {
            fishing_data_mut(entity).fish_angle = mth_next_float(store, 0.0, 360.0);
            fishing_data_mut(entity).time_until_hooked = mth_next_int(store, 20, 80);
        }
    } else {
        fishing_data_mut(entity).time_until_lured = mth_next_int(store, 100, 600);
        let data = fishing_data_mut(entity);
        data.time_until_lured -= data.lure_speed;
    }
}

fn set_biting(entity: &mut Entity, random: &GameRandom, value: bool) {
    fishing_data_mut(entity).biting = value;
    entity.mark_synched_dirty(VANILLA_FISHING_HOOK_BITING_DATA_ID);
    if value {
        let delta = entity.delta_movement();
        let kick = -0.4 * f64::from(mth_next_float_raw(random, 0.6, 1.0));
        entity.set_delta_movement(Vec3d {
            x: delta.x,
            y: kick,
            z: delta.z,
        });
    }
    entity.set_needs_sync(true);
}

fn fishing_data(entity: &Entity) -> &FishingBobberData {
    let EntityData::FishingBobber(data) = entity.data() else {
        unreachable!("fishing bobber data");
    };
    data
}

fn fishing_data_mut(entity: &mut Entity) -> &mut FishingBobberData {
    let EntityData::FishingBobber(data) = &mut entity.data else {
        unreachable!("fishing bobber data");
    };
    data
}

fn entity_width(entity: &Entity) -> f32 {
    (entity.bounding_box().max_x - entity.bounding_box().min_x) as f32
}

fn splash_block_is_water(level: &LevelCtx<'_, '_>, fish_x: f64, fish_y: f64, fish_z: f64) -> bool {
    let pos = BlockPos {
        x: fish_x.floor() as i32,
        y: (fish_y - 1.0).floor() as i32,
        z: fish_z.floor() as i32,
    };
    // `Blocks.WATER` is the water liquid block; a source or flowing water
    // block-state answers `is(Blocks.WATER)` because both are `WATER` states.
    block_state_fluid_state(level.block_state(pos)).family() == Some(FluidFamily::Water)
        && crate::fluid::liquid_block_family(level.block_state(pos)) == Some(FluidFamily::Water)
}

fn calculate_open_water(level: &LevelCtx<'_, '_>, block_pos: BlockPos) -> bool {
    let mut previous = OpenWaterType::Invalid;
    for y in -1..=2 {
        let layer = open_water_type_for_area(
            level,
            BlockPos {
                x: block_pos.x - 2,
                y: block_pos.y + y,
                z: block_pos.z - 2,
            },
            BlockPos {
                x: block_pos.x + 2,
                y: block_pos.y + y,
                z: block_pos.z + 2,
            },
        );
        match layer {
            OpenWaterType::AboveWater => {
                if previous == OpenWaterType::Invalid {
                    return false;
                }
            }
            OpenWaterType::InsideWater => {
                if previous == OpenWaterType::AboveWater {
                    return false;
                }
            }
            OpenWaterType::Invalid => return false,
        }
        previous = layer;
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenWaterType {
    AboveWater,
    InsideWater,
    Invalid,
}

fn open_water_type_for_area(
    level: &LevelCtx<'_, '_>,
    from: BlockPos,
    to: BlockPos,
) -> OpenWaterType {
    let mut result: Option<OpenWaterType> = None;
    for x in from.x..=to.x {
        for y in from.y..=to.y {
            for z in from.z..=to.z {
                let cell = open_water_type_for_block(level, BlockPos { x, y, z });
                result = Some(match result {
                    None => cell,
                    Some(previous) if previous == cell => previous,
                    Some(_) => OpenWaterType::Invalid,
                });
            }
        }
    }
    result.unwrap_or(OpenWaterType::Invalid)
}

fn open_water_type_for_block(level: &LevelCtx<'_, '_>, pos: BlockPos) -> OpenWaterType {
    let state = level.block_state(pos);
    if !bbb_server_types::fluid::block_state_is_air(state) && !is_lily_pad(state) {
        let fluid = block_state_fluid_state(state);
        let empty_collision = level.block_collision_boxes(pos).is_empty();
        if fluid_state_tag_membership(level, fluid, "minecraft:water")
            && fluid.is_source()
            && empty_collision
        {
            OpenWaterType::InsideWater
        } else {
            OpenWaterType::Invalid
        }
    } else {
        OpenWaterType::AboveWater
    }
}

fn is_lily_pad(block_state_id: i32) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
        == Some(VANILLA_BLOCK_LILY_PAD_ID)
}

fn water_fluid_height(
    level: &LevelCtx<'_, '_>,
    state: bbb_server_types::fluid::FluidState,
    pos: BlockPos,
) -> f32 {
    if block_state_fluid_state(level.block_state(above(pos))).family() == state.family() {
        1.0
    } else {
        state.own_height()
    }
}

/// One `ServerLevel.sendParticles` broadcast (`FishingHook.java` particle
/// calls) queued on the store for the tracking fan-out.
#[allow(clippy::too_many_arguments)]
fn send_particles(
    store: &mut EntityStore,
    particle_type_id: i32,
    x: f64,
    y: f64,
    z: f64,
    count: i32,
    off_x: f64,
    off_y: f64,
    off_z: f64,
    speed: f64,
) {
    store.send_particles(LevelParticles {
        override_limiter: false,
        always_show: false,
        position: Vec3d { x, y, z },
        offset: Vec3d {
            x: off_x,
            y: off_y,
            z: off_z,
        },
        max_speed: speed as f32,
        count,
        particle: ParticlePayload {
            particle_type_id,
            // Bubble, fishing and splash are `SimpleParticleType`s, which carry
            // no options on the wire.
            raw_options: Vec::new(),
        },
    });
}

fn play_sound(
    entity: &Entity,
    store: &mut EntityStore,
    resource_id: &str,
    volume: f32,
    pitch: f32,
) {
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_string(),
        source: SoundSource::Neutral,
        volume,
        pitch,
    });
}

fn mth_next_int(store: &mut EntityStore, min_inclusive: i32, max_inclusive: i32) -> i32 {
    if min_inclusive >= max_inclusive {
        min_inclusive
    } else {
        store.random().next_int(max_inclusive - min_inclusive + 1) + min_inclusive
    }
}

fn mth_next_float(store: &mut EntityStore, min: f32, max: f32) -> f32 {
    if min >= max {
        min
    } else {
        store.random().next_float() * (max - min) + min
    }
}

fn mth_next_float_raw(random: &GameRandom, min: f32, max: f32) -> f32 {
    if min >= max {
        min
    } else {
        random.next_float() * (max - min) + min
    }
}

/// What `FishingHook.retrieve` (`FishingHook.java:436-482`) did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RetrieveOutcome {
    pub rod_damage: i32,
    /// Whether the owner could still fish, so the pull or the catch ran.
    pub retrieved: bool,
    /// The entity the hook pulled toward the owner.
    pub pulled_entity_id: Option<i32>,
    /// The stacks a bite drew from `gameplay/fishing`.
    pub caught: Vec<RuntimeItemStack>,
    /// How many of them are `#minecraft:fishes`, the `FISH_CAUGHT` award.
    pub fish_caught: i32,
}

/// `FishingHook.retrieve` (`FishingHook.java:436-482`).
///
/// The hook is passed detached so its hooked entity can be mutated through the
/// store; a caller re-files it with [`EntityStore::attach`], which finalizes
/// the `discard` this makes.
pub(crate) fn retrieve(
    hook: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    owner: &NearbyPlayer,
    rod: &LootTool,
    biome_at: &LootBiomeAt<'_>,
) -> RetrieveOutcome {
    let EntityData::FishingBobber(mut data) = hook.data.clone() else {
        return RetrieveOutcome::default();
    };
    if should_stop_fishing(hook, owner) {
        hook.data = EntityData::FishingBobber(data);
        return RetrieveOutcome::default();
    }
    let mut outcome = RetrieveOutcome {
        retrieved: true,
        ..RetrieveOutcome::default()
    };
    if let Some(hooked_id) = data.hooked_entity_id {
        let is_item = pull_entity(hook, store, owner, hooked_id);
        store.broadcast_to_tracking(
            hook.id(),
            crate::connection::ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: hook.id(),
                event_id: 31,
            }),
        );
        outcome.pulled_entity_id = Some(hooked_id);
        outcome.rod_damage = if is_item { 3 } else { 5 };
    } else if data.nibble > 0 {
        // `:446-453`.
        let luck = data.luck as f32 + owner.luck;
        let drawn = level.fishing_loot_drops(
            EntityBodyReadView::new(store, Some(hook), None),
            hook,
            rod,
            luck,
            biome_at,
        );
        let origin = hook.position();
        for stack in drawn {
            let stack = RuntimeItemStack::from_summary(stack);
            if stack.is_empty() {
                continue;
            }
            // `:456-468`.
            let d = Vec3d {
                x: owner.position.x - origin.x,
                y: owner.position.y - origin.y,
                z: owner.position.z - origin.z,
            };
            if let Some(id) = spawn_item_entity(store, origin, stack.clone()) {
                if let Some(item) = store.get_mut(id) {
                    item.set_delta_movement(Vec3d {
                        x: d.x * 0.1,
                        y: d.y * 0.1 + d.length_sqr().sqrt().sqrt() * 0.08,
                        z: d.z * 0.1,
                    });
                }
            }
            let value = store.random().next_int(6) + 1;
            let _ = spawn_experience_orb(
                store,
                Vec3d {
                    x: owner.position.x,
                    y: owner.position.y + 0.5,
                    z: owner.position.z + 0.5,
                },
                value,
            );
            let fish = stack
                .item_id()
                .and_then(vanilla_item_type)
                .is_some_and(|item| {
                    level.registry_tag_membership(
                        "minecraft:item",
                        "minecraft:fishes",
                        item.resource_id,
                    )
                });
            if fish {
                outcome.fish_caught += 1;
            }
            outcome.caught.push(stack);
        }
        outcome.rod_damage = 1;
    }
    if hook.on_ground() {
        outcome.rod_damage = 2;
    }
    hook.discard();
    data.hooked_entity_id = None;
    hook.data = EntityData::FishingBobber(data);
    outcome
}

/// `FishingHook.pullEntity` (`FishingHook.java:493-499`); answers whether the
/// pulled entity is an item.
fn pull_entity(
    hook: &Entity,
    store: &mut EntityStore,
    owner: &NearbyPlayer,
    hooked_id: i32,
) -> bool {
    let Some(hooked) = store.get_mut(hooked_id) else {
        return false;
    };
    let is_item = matches!(hooked.data(), EntityData::Item(_));
    let delta = Vec3d {
        x: owner.position.x - hook.position().x,
        y: owner.position.y - hook.position().y,
        z: owner.position.z - hook.position().z,
    }
    .scale(0.1);
    hooked.set_delta_movement(hooked.delta_movement().plus(delta));
    hooked.set_needs_sync(true);
    is_item
}

/// The hook `owner` has out, `Player.fishing`.
pub(crate) fn owned_bobber(store: &EntityStore, owner: uuid::Uuid) -> Option<i32> {
    store.owned_bobber_id(owner)
}

pub fn synched_values(entity: &Entity, data: &FishingBobberData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    let hooked = data.data_hooked_entity();
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(hooked),
        },
        is_default: hooked == 0,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_FISHING_HOOK_BITING_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(data.biting),
        },
        is_default: !data.biting,
    });
    rows
}
