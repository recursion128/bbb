use crate::world::simulation_level::LevelCtx;
use rustc_hash::FxHashSet;

use crate::block_behavior::InsideEntity;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::{VanillaGameEvent, LEVEL_EVENT_PARTICLES_DESTROY_BLOCK};
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id, vanilla_block_state_property,
};
use bbb_protocol::block_types::{
    self as block_types, vanilla_block_type, VanillaBlockRedstoneSignalOverride,
    VANILLA_BLOCK_AIR_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{axis_step_order, Aabb, Axis, Vec3Ops};

use crate::entity::environment::{self, KnownContact};
use crate::entity::{damage, living, Entity, EntityStore};

pub(crate) mod berry;
pub(crate) mod body_contact;
mod continuation;
pub(crate) use continuation::{
    start_nonliving as start_nonliving_inside_pausing, NonlivingInsidePause,
};
pub(crate) mod effects;
pub(crate) mod extinguish;
mod honey;
mod immediate;
pub(crate) mod lava;
pub(crate) mod sculk;
pub(crate) mod sculk_player;
pub(crate) mod step;
use effects::{
    BlockEffectCollector as StepBasedCollector, InsideBlockEffect as DeferredEffect,
    InsideBlockEffectType as EffectType,
};
pub(crate) use honey::slide_effects as honey_slide_effects;
pub(crate) use step::ground_body;

const INSIDE_EPSILON: f64 = 1.0e-5_f32 as f64;
const MAX_MOVEMENT_ITERATIONS: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityMovement {
    pub from: Vec3d,
    pub to: Vec3d,
    pub axis_dependent_original: Option<Vec3d>,
}

impl EntityMovement {
    pub const fn direct(from: Vec3d, to: Vec3d) -> Self {
        Self {
            from,
            to,
            axis_dependent_original: None,
        }
    }

    pub const fn axis_dependent(from: Vec3d, to: Vec3d, original: Vec3d) -> Self {
        Self {
            from,
            to,
            axis_dependent_original: Some(original),
        }
    }
}

impl Entity {
    /// `Entity.java`, `Vex.java`, `ShulkerBullet.java`.
    pub(crate) fn is_affected_by_blocks(&self) -> bool {
        use bbb_protocol::entity_types::{
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, VANILLA_ENTITY_TYPE_VEX_ID,
        };
        self.removal_reason().is_none()
            && (!self.no_physics
                || matches!(
                    self.entity_type_id(),
                    VANILLA_ENTITY_TYPE_VEX_ID | VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID
                ))
    }

    /// `Entity.addMovementThisTick` (`Entity.java`).
    pub(crate) fn record_movement(&mut self, movement: EntityMovement) {
        body_contact::record_body_movement(&mut self.movement_this_tick, movement);
    }

    /// `Entity.applyEffectsFromBlocks`' movement snapshot (`Entity.java`).
    pub(crate) fn take_effect_movements(&mut self) -> Vec<EntityMovement> {
        body_contact::take_body_effect_movements(
            &mut self.movement_this_tick,
            &mut self.final_movements_this_tick,
            self.old_position,
            self.position,
        );
        self.final_movements_this_tick.clone()
    }

    /// `Entity.clearMovementThisTick` (`Entity.java`).
    pub(crate) fn clear_movement_this_tick(&mut self) {
        self.movement_this_tick.clear();
    }

    fn last_effect_movements(&self) -> Vec<EntityMovement> {
        self.final_movements_this_tick.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LivingInsideBlockCallback {
    SweetBerryBush {
        state: i32,
    },
    Damage {
        damage_type: &'static str,
        amount: f32,
    },
    LavaDamage {
        amount: f32,
    },
    WitherRose {
        duration: i32,
    },
    BeeEyeblossom {
        state: i32,
        duration: i32,
    },
    HoneySlideEffects,
    ExtinguishSound,
    PortalContact {
        kind: crate::entity::portal::PortalKind,
        pos: BlockPos,
    },
    SculkShriekerStep {
        pos: BlockPos,
    },
    SculkSensorStep {
        state: i32,
        pos: BlockPos,
    },
}

/// `Entity.applyEffectsFromBlocks` (`Entity.java`) for non-living
/// call sites.
pub(crate) fn apply_nonliving_inside_block_effects(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut EntityStore>,
) -> bool {
    apply_living_inside_block_effects(entity, level, store, |entity, level, store, callback| {
        nonliving_inside_block_callback(entity, level, store, callback)
    })
}

pub(crate) fn apply_nonliving_inside_block_effects_for_last_movements(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut EntityStore>,
) -> bool {
    apply_inside_block_effects_for_last_movements(
        entity,
        level,
        store,
        |entity, level, store, callback| {
            nonliving_inside_block_callback(entity, level, store, callback)
        },
    )
}

pub(crate) fn apply_nonliving_inside_block_effects_between(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut EntityStore>,
    from: Vec3d,
    to: Vec3d,
) -> bool {
    apply_inside_block_effects_between(
        entity,
        level,
        store,
        from,
        to,
        |entity, level, store, callback| {
            nonliving_inside_block_callback(entity, level, store, callback)
        },
    )
}

fn nonliving_inside_block_callback(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    mut store: Option<&mut EntityStore>,
    callback: LivingInsideBlockCallback,
) -> bool {
    if entity.living().is_some() {
        return store.is_some_and(|store| {
            living::apply_detached_inside_callback(entity, store, level, callback)
        });
    }
    match callback {
        LivingInsideBlockCallback::SweetBerryBush { .. } => true,
        LivingInsideBlockCallback::SculkShriekerStep { pos } => {
            sculk_player::step_on_shrieker(entity, store.as_deref_mut(), level, pos).is_some()
        }
        LivingInsideBlockCallback::SculkSensorStep { state, pos } => {
            sculk::step_on_sensor(entity, store.as_deref_mut(), level, state, pos).is_some()
        }
        LivingInsideBlockCallback::ExtinguishSound => store
            .is_some_and(|store| extinguish::play_entity_extinguish_sound(entity, store).is_some()),
        LivingInsideBlockCallback::PortalContact { kind, pos } => {
            level.record_portal_contact(entity, kind, pos);
            true
        }
        LivingInsideBlockCallback::WitherRose { duration } => store
            .is_some_and(|store| living::apply_wither_rose_effect(entity, store, level, duration)),
        LivingInsideBlockCallback::BeeEyeblossom { state, duration } => {
            store.is_some_and(|store| {
                living::apply_eyeblossom_effect(entity, store, level, state, duration)
            })
        }
        LivingInsideBlockCallback::HoneySlideEffects => {
            store.is_some_and(|store| honey_slide_effects(entity, store, level).is_some())
        }
        LivingInsideBlockCallback::Damage {
            damage_type,
            amount,
        } => store.is_some_and(|store| {
            damage::hurt_detached_from_environment(entity, store, level, damage_type, amount)
                .is_some()
        }),
        LivingInsideBlockCallback::LavaDamage { amount } => store.is_some_and(|store| {
            lava::apply_lava_hurt(
                entity,
                store,
                level,
                amount,
                &mut damage::hurt_detached_from_environment,
            )
        }),
    }
}

/// `Entity.applyEffectsFromBlocks` (`Entity.java`) and
/// `Entity.checkInsideBlocks` (`Entity.java`).
pub(crate) fn apply_living_inside_block_effects<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut crate::entity::EntityStore>,
    callback: F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    let movements = entity.take_effect_movements();
    apply_inside_block_effects_for_movements(entity, level, store, movements, callback)
}

pub(crate) fn apply_inside_block_effects_for_last_movements<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut crate::entity::EntityStore>,
    callback: F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    let movements = entity.last_effect_movements();
    apply_inside_block_effects_for_movements(entity, level, store, movements, callback)
}

pub(crate) fn apply_inside_block_effects_between<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut crate::entity::EntityStore>,
    from: Vec3d,
    to: Vec3d,
    callback: F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    apply_inside_block_effects_for_movements(
        entity,
        level,
        store,
        vec![EntityMovement::direct(from, to)],
        callback,
    )
}

fn apply_inside_block_effects_for_movements<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut crate::entity::EntityStore>,
    movements: Vec<EntityMovement>,
    mut callback: F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    continuation::InsideEffectsFrame::new(entity, movements).run(
        entity,
        level,
        store,
        &mut callback,
    )
}

fn apply_deferred_effect<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    mut store: Option<&mut EntityStore>,
    effect: DeferredEffect,
    callback: &mut F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    match effect {
        DeferredEffect::Type(effect) => apply_effect_type(entity, level, effect),
        DeferredEffect::Damage {
            damage_type,
            amount,
        } => callback(
            entity,
            level,
            store.as_deref_mut(),
            LivingInsideBlockCallback::Damage {
                damage_type,
                amount,
            },
        ),
        DeferredEffect::LavaDamage { amount } => callback(
            entity,
            level,
            store.as_deref_mut(),
            LivingInsideBlockCallback::LavaDamage { amount },
        ),
        DeferredEffect::LowerCauldron { pos, state } => {
            let Ok(on_fire) = entity.is_on_fire_in(level) else {
                return false;
            };
            if on_fire
                && crate::block_behavior::projectile_hit::entity_may_interact(
                    entity,
                    store.as_deref(),
                    level,
                    pos,
                )
            {
                crate::block_behavior::cauldron::lower_fill_level_on_fire(level, pos, state);
            }
            true
        }
        DeferredEffect::DestroyPowderSnow(pos) => {
            let Ok(on_fire) = entity.is_on_fire_in(level) else {
                return false;
            };
            // `PowderSnowBlock.java:88-92`; a projectile answers for its
            // owner (`Projectile.java:361-363`).
            if on_fire
                && level.mob_griefing()
                && crate::block_behavior::projectile_hit::entity_may_interact(
                    entity,
                    store.as_deref(),
                    level,
                    pos,
                )
            {
                let state = level.block_state(pos);
                let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID)
                    .expect("built-in air has a default state");
                level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, pos, state);
                if level.set_entity_block(
                    crate::entity::level::EntityBlockContext::new(store, Some(entity)),
                    pos,
                    air,
                    SET_BLOCK_UPDATE_ALL,
                ) {
                    level.game_event_with_state(
                        VanillaGameEvent::BLOCK_DESTROY,
                        pos,
                        None,
                        Some(state),
                    );
                }
            }
            true
        }
    }
}

fn apply_effect_type(entity: &mut Entity, level: &LevelCtx<'_, '_>, effect: EffectType) -> bool {
    match effect {
        EffectType::Freeze => {
            entity.set_is_in_powder_snow(true);
            match environment::can_freeze(entity, level) {
                KnownContact::Unknown => return false,
                KnownContact::Known(true) => entity.set_ticks_frozen(
                    (entity.ticks_frozen() + 1).min(environment::TICKS_REQUIRED_TO_FREEZE),
                ),
                KnownContact::Known(false) => {}
            }
        }
        EffectType::ClearFreeze => entity.set_ticks_frozen(0),
        EffectType::FireIgnite => {
            let Ok(fire_immune) = entity.fire_immune_in(level) else {
                return false;
            };
            if !fire_immune {
                if entity.remaining_fire_ticks() < 0 {
                    entity.set_remaining_fire_ticks(entity.remaining_fire_ticks() + 1);
                }
                if entity.remaining_fire_ticks() >= 0 {
                    crate::entity::living::ignite_living_for_seconds(entity, 8.0);
                    entity.set_ticks_frozen(0);
                }
            }
        }
        EffectType::LavaIgnite => {
            let Ok(fire_immune) = entity.fire_immune_in(level) else {
                return false;
            };
            if !fire_immune {
                crate::entity::living::ignite_living_for_seconds(entity, 15.0);
                entity.set_ticks_frozen(0);
            }
        }
        EffectType::Extinguish => entity.clear_fire(),
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn apply_block_inside<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    mut store: Option<&mut crate::entity::EntityStore>,
    pos: BlockPos,
    state: i32,
    precise: bool,
    collector: &mut StepBasedCollector,
    callback: &mut F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    let Some(block_type_id) = vanilla_block_state_block_type_id(state) else {
        return false;
    };
    let environment_kind = effects::environment_block_kind(block_type_id);
    collector.collect_block(environment_kind, pos, state);
    match immediate::apply(entity, level, store.as_deref_mut(), state, pos, callback) {
        Some(true) => return true,
        Some(false) => {}
        None => return false,
    }
    match block_type_id {
        block_types::VANILLA_BLOCK_HOPPER_ID => {
            if matches!(entity.data(), crate::entity::EntityData::Item(_)) {
                if let Some(store) = store.as_deref_mut() {
                    store.queue_command(crate::entity::EntityCommand::HopperItemInside {
                        pos,
                        entity_id: entity.id(),
                    });
                }
            }
        }
        block_types::VANILLA_BLOCK_NETHER_PORTAL_ID => {
            // `NetherPortalBlock.entityInside` (`NetherPortalBlock.java`).
            return callback(
                entity,
                level,
                store.as_deref_mut(),
                LivingInsideBlockCallback::PortalContact {
                    kind: crate::entity::portal::PortalKind::Nether,
                    pos,
                },
            );
        }
        block_types::VANILLA_BLOCK_END_PORTAL_ID => {
            // `EndPortalBlock.entityInside` (`EndPortalBlock.java`).
            return callback(
                entity,
                level,
                store.as_deref_mut(),
                LivingInsideBlockCallback::PortalContact {
                    kind: crate::entity::portal::PortalKind::End,
                    pos,
                },
            );
        }
        block_types::VANILLA_BLOCK_END_GATEWAY_ID => {
            // ThrownEnderpearl.java:213-217.
            if let crate::entity::EntityData::EnderPearl(pearl) = entity.data() {
                if let (Some(player), Some(store)) =
                    (pearl.projectile().owner(), store.as_deref_mut())
                {
                    store.queue_command(crate::entity::EntityCommand::PlayerEnteredBlock {
                        player,
                        state,
                    });
                }
            }
            // EndGatewayBlock.java.
            return callback(
                entity,
                level,
                store.as_deref_mut(),
                LivingInsideBlockCallback::PortalContact {
                    kind: crate::entity::portal::PortalKind::EndGateway,
                    pos,
                },
            );
        }
        block_types::VANILLA_BLOCK_DETECTOR_RAIL_ID | block_types::VANILLA_BLOCK_TRIPWIRE_ID => {
            // `DetectorRailBlock.entityInside` / `TripWireBlock.entityInside`.
            return level.entity_inside_block(
                crate::entity::level::EntityBlockContext::new(store.as_deref_mut(), Some(entity)),
                pos,
                state,
                visiting_entity(entity),
            );
        }
        id if vanilla_block_type(id).is_some_and(|block| {
            matches!(
                block.is_signal_source_override,
                Some(
                    VanillaBlockRedstoneSignalOverride::BasePressurePlateBlock
                        | VanillaBlockRedstoneSignalOverride::ButtonBlock
                )
            )
        }) =>
        {
            // `BasePressurePlateBlock.entityInside` / `ButtonBlock.entityInside`.
            return level.entity_inside_block(
                crate::entity::level::EntityBlockContext::new(store.as_deref_mut(), Some(entity)),
                pos,
                state,
                visiting_entity(entity),
            );
        }
        block_types::VANILLA_BLOCK_BIG_DRIPLEAF_ID => {
            crate::block_behavior::big_dripleaf::entity_inside(
                level,
                pos,
                state,
                entity.on_ground(),
                entity.position().y,
            );
        }
        block_types::VANILLA_BLOCK_POWDER_SNOW_ID => {
            // `PowderSnowBlock.java:65`: anything but a living body is always
            // slowed; a living one only once its in-block state is the snow.
            let in_block_state = level.block_state(entity.block_position());
            let Some(in_block_type_id) = vanilla_block_state_block_type_id(in_block_state) else {
                return false;
            };
            if entity.living().is_none()
                || in_block_type_id == block_types::VANILLA_BLOCK_POWDER_SNOW_ID
            {
                entity.make_stuck_in_block(Vec3d {
                    x: f64::from(0.9_f32),
                    y: 1.5,
                    z: f64::from(0.9_f32),
                });
            }
        }
        block_types::VANILLA_BLOCK_BUBBLE_COLUMN_ID if precise => {
            // `BubbleColumnBlock.entityInside` (`BubbleColumnBlock.java:53-64`).
            let Some(drag_down) = bubble_column_drags_down(state) else {
                return false;
            };
            let above = BlockPos {
                y: pos.y + 1,
                ..pos
            };
            let above_state = level.block_state(above);
            if vanilla_block_state_block_type_id(above_state).is_none() {
                return false;
            }
            let Some(above_fluid) = environment::fluid_state_at(level.chunks(), above) else {
                return false;
            };
            let above_empty = if let Some(boxes) =
                bbb_server_types::geometry::static_block_collision_box_iter(above_state)
            {
                boxes.len() == 0
            } else {
                level.block_collision_boxes(above).is_empty()
            };
            if above_empty && above_fluid.is_empty() {
                on_above_bubble_column(entity, level, store, drag_down, pos);
            } else {
                on_inside_bubble_column(entity, drag_down);
            }
        }
        _ => {}
    }
    true
}

fn bubble_column_drags_down(state: i32) -> Option<bool> {
    vanilla_block_state_property(state, "drag").map(|value| value == "true")
}

enum BubbleColumnBody {
    Generic,
    Projectile,
    Boat,
    Unaffected,
}

fn bubble_column_body(entity: &Entity) -> BubbleColumnBody {
    use crate::entity::EntityData;
    match entity.data() {
        data if crate::entity::boat::is_boat(data) => BubbleColumnBody::Boat,
        // `ThrownEnderpearl.java:230-237` restores the base behaviour.
        EntityData::EnderPearl(_) => BubbleColumnBody::Generic,
        // `AbstractArrow.java:398-409`.
        data if crate::entity::abstract_arrow::state(data)
            .is_some_and(|arrow| arrow.in_ground()) =>
        {
            BubbleColumnBody::Unaffected
        }
        _ if entity.is_projectile_class() => BubbleColumnBody::Projectile,
        _ => BubbleColumnBody::Generic,
    }
}

/// `Entity.onAboveBubbleColumn` (`Entity.java:2771-2801`) with the
/// `Projectile` (`Projectile.java:162-166`) and `AbstractBoat`
/// (`AbstractBoat.java:154-172`) overrides.
fn on_above_bubble_column(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut EntityStore>,
    drag_down: bool,
    pos: BlockPos,
) {
    let movement = entity.delta_movement;
    let y = match bubble_column_body(entity) {
        BubbleColumnBody::Unaffected => return,
        BubbleColumnBody::Boat => {
            crate::entity::boat::on_above_bubble_column(entity, drag_down);
            if !entity.is_under_water() && level.level_random().next_int(100) == 0 {
                let source = store.and_then(|store| store.passengers(entity.id()).first().copied());
                level.game_event_at(VanillaGameEvent::SPLASH, entity.position(), source);
            }
            return;
        }
        BubbleColumnBody::Projectile => movement.y + if drag_down { -0.03 } else { 0.1 },
        BubbleColumnBody::Generic if drag_down => (movement.y - 0.03).max(-0.9),
        BubbleColumnBody::Generic => (movement.y + 0.1).min(1.8),
    };
    entity.set_delta_movement(Vec3d { y, ..movement });
    if let Some(store) = store {
        for particles in bubble_column_particles(level.level_random(), pos) {
            store.send_particles(particles);
        }
    }
}

/// `Entity.onInsideBubbleColumn` (`Entity.java:2803-2817`) with the
/// `Projectile` override (`Projectile.java:169-173`).
fn on_inside_bubble_column(entity: &mut Entity, drag_down: bool) {
    let movement = entity.delta_movement;
    let y = match bubble_column_body(entity) {
        BubbleColumnBody::Unaffected => return,
        BubbleColumnBody::Projectile => movement.y + if drag_down { -0.03 } else { 0.06 },
        BubbleColumnBody::Generic | BubbleColumnBody::Boat if drag_down => {
            (movement.y - 0.03).max(-0.3)
        }
        BubbleColumnBody::Generic | BubbleColumnBody::Boat => (movement.y + 0.06).min(0.7),
    };
    entity.set_delta_movement(Vec3d { y, ..movement });
    entity.reset_fall_distance();
}

/// `Entity.sendBubbleColumnParticles` (`Entity.java:2788-2801`).
pub(crate) fn bubble_column_particles(
    random: &crate::random::GameRandom,
    pos: BlockPos,
) -> Vec<bbb_protocol::packets::LevelParticles> {
    use bbb_protocol::packets::{LevelParticles, ParticlePayload};
    use bbb_protocol::particle_types::{
        VANILLA_PARTICLE_TYPE_BUBBLE_ID, VANILLA_PARTICLE_TYPE_SPLASH_ID,
    };
    let particle = |particle_type_id, offset_y, speed: f32| LevelParticles {
        override_limiter: false,
        always_show: false,
        position: Vec3d {
            x: f64::from(pos.x) + random.next_double(),
            y: f64::from(pos.y + 1),
            z: f64::from(pos.z) + random.next_double(),
        },
        offset: Vec3d {
            x: 0.0,
            y: offset_y,
            z: 0.0,
        },
        max_speed: speed,
        count: 1,
        particle: ParticlePayload {
            particle_type_id,
            raw_options: Vec::new(),
        },
    };
    let mut packets = Vec::with_capacity(4);
    for _ in 0..2 {
        packets.push(particle(VANILLA_PARTICLE_TYPE_SPLASH_ID, 0.0, 1.0));
        packets.push(particle(VANILLA_PARTICLE_TYPE_BUBBLE_ID, 0.01, 0.2));
    }
    packets
}

/// The entity the walk is running, as the block seam reads it: its own tick
/// holds it, so the level's occupancy snapshot cannot answer for it.
fn visiting_entity(entity: &Entity) -> InsideEntity {
    InsideEntity {
        source: Some(entity.id()),
        bounds: entity.bounding_box(),
        minecart: crate::entity::minecart::is_minecart(entity.data()),
        living: entity.living().is_some(),
        ignores_block_triggers: entity.is_ignoring_block_triggers(),
    }
}

fn living_is_alive(entity: &Entity) -> bool {
    entity.removal_reason().is_none() && entity.living().is_none_or(|living| living.health() > 0.0)
}

pub(crate) fn for_each_block_intersected(
    from: Vec3d,
    to: Vec3d,
    target_box: Aabb,
    mut visitor: impl FnMut(BlockPos, i32) -> bool,
) {
    let travel = subtract(to, from);
    if travel.length_sqr() < INSIDE_EPSILON.powi(2) {
        for_each_between_box_corners(target_box, travel, |pos| visitor(pos, 0));
        return;
    }

    let mut visited = FxHashSet::default();
    let keep_going =
        for_each_between_box_corners(target_box.moved(travel.scale(-1.0)), travel, |pos| {
            if !visitor(pos, 0) {
                return false;
            }
            visited.insert((pos.x, pos.y, pos.z));
            true
        });
    if !keep_going {
        return;
    }
    let Some(iterations) =
        add_collisions_along_travel(&mut visited, travel, target_box, &mut visitor)
    else {
        return;
    };
    for_each_between_box_corners(target_box, travel, |pos| {
        !visited.insert((pos.x, pos.y, pos.z)) || visitor(pos, iterations + 1)
    });
}

fn add_collisions_along_travel(
    visited: &mut FxHashSet<(i32, i32, i32)>,
    travel: Vec3d,
    target_box: Aabb,
    visitor: &mut impl FnMut(BlockPos, i32) -> bool,
) -> Option<i32> {
    let size = Vec3d {
        x: target_box.max_x - target_box.min_x,
        y: target_box.max_y - target_box.min_y,
        z: target_box.max_z - target_box.min_z,
    };
    let corner_direction = furthest_corner(travel);
    let center = target_box.center();
    let to_corner = Vec3d {
        x: center.x + size.x * 0.5 * f64::from(corner_direction.0),
        y: center.y + size.y * 0.5 * f64::from(corner_direction.1),
        z: center.z + size.z * 0.5 * f64::from(corner_direction.2),
    };
    let from_corner = subtract(to_corner, travel);
    let mut block = (
        from_corner.x.floor() as i32,
        from_corner.y.floor() as i32,
        from_corner.z.floor() as i32,
    );
    let sign = (sign(travel.x), sign(travel.y), sign(travel.z));
    let delta = (
        step_delta(sign.0, travel.x),
        step_delta(sign.1, travel.y),
        step_delta(sign.2, travel.z),
    );
    let mut next = (
        first_step(delta.0, sign.0, from_corner.x),
        first_step(delta.1, sign.1, from_corner.y),
        first_step(delta.2, sign.2, from_corner.z),
    );
    if !from_corner.x.is_finite()
        || !from_corner.y.is_finite()
        || !from_corner.z.is_finite()
        || !to_corner.x.is_finite()
        || !to_corner.y.is_finite()
        || !to_corner.z.is_finite()
        || !size.x.is_finite()
        || !size.y.is_finite()
        || !size.z.is_finite()
    {
        return Some(0);
    }
    const MAX_TRAVEL_DDA_STEPS: usize = 16_384;
    let mut steps = 0usize;
    let mut iterations = 0;

    while next.0 <= 1.0 || next.1 <= 1.0 || next.2 <= 1.0 {
        if steps >= MAX_TRAVEL_DDA_STEPS {
            break;
        }
        steps += 1;
        if next.0 < next.1 {
            if next.0 < next.2 {
                let Some(next_x) = block.0.checked_add(sign.0) else {
                    break;
                };
                let advanced = next.0 + delta.0;
                if !advanced.is_finite() || advanced <= next.0 {
                    break;
                }
                block.0 = next_x;
                next.0 = advanced;
            } else {
                let Some(next_z) = block.2.checked_add(sign.2) else {
                    break;
                };
                let advanced = next.2 + delta.2;
                if !advanced.is_finite() || advanced <= next.2 {
                    break;
                }
                block.2 = next_z;
                next.2 = advanced;
            }
        } else if next.1 < next.2 {
            let Some(next_y) = block.1.checked_add(sign.1) else {
                break;
            };
            let advanced = next.1 + delta.1;
            if !advanced.is_finite() || advanced <= next.1 {
                break;
            }
            block.1 = next_y;
            next.1 = advanced;
        } else {
            let Some(next_z) = block.2.checked_add(sign.2) else {
                break;
            };
            let advanced = next.2 + delta.2;
            if !advanced.is_finite() || advanced <= next.2 {
                break;
            }
            block.2 = next_z;
            next.2 = advanced;
        }
        let block_box = Aabb::of_block(BlockPos {
            x: block.0,
            y: block.1,
            z: block.2,
        });
        let Some(hit) = crate::entity::projectile_hit::aabb_clip(block_box, from_corner, to_corner)
        else {
            continue;
        };
        iterations += 1;
        let max_x = f64::from(block.0) + 1.0 - INSIDE_EPSILON;
        let max_y = f64::from(block.1) + 1.0 - INSIDE_EPSILON;
        let max_z = f64::from(block.2) + 1.0 - INSIDE_EPSILON;
        let hit = Vec3d {
            x: hit.x.clamp(f64::from(block.0) + INSIDE_EPSILON, max_x),
            y: hit.y.clamp(f64::from(block.1) + INSIDE_EPSILON, max_y),
            z: hit.z.clamp(f64::from(block.2) + INSIDE_EPSILON, max_z),
        };
        let opposite = (
            (hit.x - size.x * f64::from(corner_direction.0)).floor() as i32,
            (hit.y - size.y * f64::from(corner_direction.1)).floor() as i32,
            (hit.z - size.z * f64::from(corner_direction.2)).floor() as i32,
        );
        if !for_each_between_corners(block, opposite, travel, |pos| {
            !visited.insert((pos.x, pos.y, pos.z)) || visitor(pos, iterations)
        }) {
            return None;
        }
    }
    Some(iterations)
}

fn for_each_between_box_corners(
    box_: Aabb,
    direction: Vec3d,
    visitor: impl FnMut(BlockPos) -> bool,
) -> bool {
    for_each_between_corners(
        (
            box_.min_x.floor() as i32,
            box_.min_y.floor() as i32,
            box_.min_z.floor() as i32,
        ),
        (
            box_.max_x.floor() as i32,
            box_.max_y.floor() as i32,
            box_.max_z.floor() as i32,
        ),
        direction,
        visitor,
    )
}

fn for_each_between_corners(
    first: (i32, i32, i32),
    second: (i32, i32, i32),
    direction: Vec3d,
    mut visitor: impl FnMut(BlockPos) -> bool,
) -> bool {
    let min = (
        first.0.min(second.0),
        first.1.min(second.1),
        first.2.min(second.2),
    );
    let max = (
        first.0.max(second.0),
        first.1.max(second.1),
        first.2.max(second.2),
    );
    let ranges = (max.0 - min.0, max.1 - min.1, max.2 - min.2);
    let start = (
        if direction.x >= 0.0 { min.0 } else { max.0 },
        if direction.y >= 0.0 { min.1 } else { max.1 },
        if direction.z >= 0.0 { min.2 } else { max.2 },
    );
    let axes = axis_step_order(direction);
    let sign0 = axis_sign(direction, axes[0]);
    let sign1 = axis_sign(direction, axes[1]);
    let sign2 = axis_sign(direction, axes[2]);
    for first_index in 0..=axis_component_i32(ranges, axes[0]) {
        for second_index in 0..=axis_component_i32(ranges, axes[1]) {
            for third_index in 0..=axis_component_i32(ranges, axes[2]) {
                let mut offset = (0, 0, 0);
                add_axis_offset(&mut offset, axes[0], first_index * sign0);
                add_axis_offset(&mut offset, axes[1], second_index * sign1);
                add_axis_offset(&mut offset, axes[2], third_index * sign2);
                if !visitor(BlockPos {
                    x: start.0 + offset.0,
                    y: start.1 + offset.1,
                    z: start.2 + offset.2,
                }) {
                    return false;
                }
            }
        }
    }
    true
}

pub(crate) fn collided_along_vector(entity_box: Aabb, travel: Vec3d, shape: Aabb) -> bool {
    let center = entity_box.center();
    let to = center.plus(travel);
    let inflated = shape.inflate(
        (entity_box.max_x - entity_box.min_x) * 0.5 - 1.0e-7,
        (entity_box.max_y - entity_box.min_y) * 0.5 - 1.0e-7,
        (entity_box.max_z - entity_box.min_z) * 0.5 - 1.0e-7,
    );
    contains(inflated, center)
        || contains(inflated, to)
        || crate::entity::projectile_hit::aabb_clip(inflated, center, to).is_some()
}

fn contains(box_: Aabb, point: Vec3d) -> bool {
    point.x >= box_.min_x
        && point.x < box_.max_x
        && point.y >= box_.min_y
        && point.y < box_.max_y
        && point.z >= box_.min_z
        && point.z < box_.max_z
}

fn furthest_corner(direction: Vec3d) -> (i32, i32, i32) {
    let x_sign = if direction.x >= 0.0 { 1 } else { -1 };
    let y_sign = if direction.y >= 0.0 { 1 } else { -1 };
    let z_sign = if direction.z >= 0.0 { 1 } else { -1 };
    if direction.x.abs() <= direction.y.abs() && direction.x.abs() <= direction.z.abs() {
        (-x_sign, -z_sign, y_sign)
    } else if direction.y.abs() <= direction.z.abs() {
        (z_sign, -y_sign, -x_sign)
    } else {
        (-y_sign, x_sign, -z_sign)
    }
}

fn component(vector: Vec3d, axis: Axis) -> f64 {
    match axis {
        Axis::X => vector.x,
        Axis::Y => vector.y,
        Axis::Z => vector.z,
    }
}

fn with_component_added(mut vector: Vec3d, axis: Axis, amount: f64) -> Vec3d {
    match axis {
        Axis::X => vector.x += amount,
        Axis::Y => vector.y += amount,
        Axis::Z => vector.z += amount,
    }
    vector
}

fn axis_component_i32(vector: (i32, i32, i32), axis: Axis) -> i32 {
    match axis {
        Axis::X => vector.0,
        Axis::Y => vector.1,
        Axis::Z => vector.2,
    }
}

fn axis_sign(vector: Vec3d, axis: Axis) -> i32 {
    if component(vector, axis) >= 0.0 {
        1
    } else {
        -1
    }
}

fn add_axis_offset(vector: &mut (i32, i32, i32), axis: Axis, amount: i32) {
    match axis {
        Axis::X => vector.0 += amount,
        Axis::Y => vector.1 += amount,
        Axis::Z => vector.2 += amount,
    }
}

fn step_delta(sign: i32, movement: f64) -> f64 {
    if sign == 0 {
        f64::MAX
    } else {
        f64::from(sign) / movement
    }
}

fn first_step(delta: f64, sign: i32, coordinate: f64) -> f64 {
    let fraction = coordinate - coordinate.floor();
    delta * if sign > 0 { 1.0 - fraction } else { fraction }
}

fn sign(value: f64) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

fn subtract(left: Vec3d, right: Vec3d) -> Vec3d {
    Vec3d {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn distance_to_sqr(left: Vec3d, right: Vec3d) -> f64 {
    subtract(left, right).length_sqr()
}

#[cfg(test)]
mod tests;
