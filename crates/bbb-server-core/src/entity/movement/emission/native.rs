use super::{
    body_movement_effects, BodyMovementEffect, BodyMovementEmissionState, BodyMovementFacts,
};
use crate::entity::{Entity, EntityStore, MovementEmission};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::{BlockPos, Vec3d};

pub(in crate::entity::movement) fn apply(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    emission: MovementEmission,
    delta: Vec3d,
    pos: BlockPos,
    state: i32,
) {
    let support = entity.get_on_pos();
    let facts = BodyMovementFacts {
        support,
        support_state: level.block_state(support),
        on_ground: entity.on_ground(),
        crouching: entity.is_crouching(),
        on_rails: entity.is_on_rails(),
        swimming: entity.is_swimming(),
        in_water: entity.is_in_water(),
    };
    let mut counters = BodyMovementEmissionState {
        move_dist: entity.move_dist,
        fly_dist: entity.fly_dist,
        next_step: entity.next_step,
    };
    let effects = body_movement_effects(
        &mut counters,
        facts,
        |state| Some(super::super::is_state_climbable(level, state)),
        emission,
        delta,
        pos,
        state,
    );
    let fly_dist_before = entity.fly_dist;
    entity.move_dist = counters.move_dist;
    entity.fly_dist = counters.fly_dist;
    entity.next_step = counters.next_step;
    for effect in effects.into_iter().flatten() {
        match effect {
            BodyMovementEffect::StepSound { pos, state } => {
                super::sounds::play_step(entity, store, level, pos, state);
                super::sounds::play_amethyst_step(entity, store, level, state);
            }
            BodyMovementEffect::SwimSound => super::sounds::play_swim(entity, store),
            BodyMovementEffect::GameEvent(event, state) => {
                level.game_event_at_with_state(event, entity.position(), Some(entity.id()), state)
            }
        }
    }
    if bbb_server_types::fluid::block_state_is_air(facts.support_state)
        && is_flapping(entity, fly_dist_before)
    {
        super::sounds::play_flap(entity, store);
        if emission.emits_events() {
            level.game_event_at(
                crate::level_event::VanillaGameEvent::FLAP,
                entity.position(),
                Some(entity.id()),
            );
        }
    }
}

/// `Entity.processFlappingMovement` (`Entity.java:974-980`) with the
/// `isFlapping` overrides. Parrot and chicken flap every half block of
/// `flyDist` (an airborne `flapSpeed` of 1) instead of keeping `nextFlap`.
/// Vex moves with `noPhysics` and never reaches the movement emission.
fn is_flapping(entity: &Entity, fly_dist_before: f32) -> bool {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_BEE_ID,
        VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
        VANILLA_ENTITY_TYPE_PHANTOM_ID,
    };
    let tick = entity.tick_count();
    match entity.entity_type_id() {
        // Parrot.java:360-362, Chicken.java:141-143.
        VANILLA_ENTITY_TYPE_PARROT_ID | VANILLA_ENTITY_TYPE_CHICKEN_ID => {
            (entity.fly_dist * 2.0).floor() > (fly_dist_before * 2.0).floor()
        }
        // Allay.java:364-366.
        VANILLA_ENTITY_TYPE_ALLAY_ID => !entity.on_ground(),
        // Bee.java:100,621-623.
        VANILLA_ENTITY_TYPE_BEE_ID => !entity.on_ground() && tick % 2 == 0,
        // Phantom.java:46,60-62,105-107.
        VANILLA_ENTITY_TYPE_PHANTOM_ID => entity.id().wrapping_mul(3).wrapping_add(tick) % 25 == 0,
        // Bat.java:50-52.
        VANILLA_ENTITY_TYPE_BAT_ID => {
            tick % 10 == 0
                && matches!(
                    entity.data(),
                    crate::entity::EntityData::GeneralLiving(data)
                        if data.bat_retained().is_some_and(|bat| !bat.is_resting())
                )
        }
        _ => false,
    }
}
