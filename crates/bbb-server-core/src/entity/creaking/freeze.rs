use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use bbb_protocol::packets::{SoundSource, Vec3d};

use crate::entity::ai::{MemoryModuleType, MemoryValue};
use crate::entity::level::NearbyPlayer;
use crate::entity::store::PendingEntitySound;
use crate::entity::{Entity, EntityStore};

/// `SoundEvents.CREAKING_ACTIVATE` (`Creaking.java:494`).
const CREAKING_ACTIVATE: &str = "minecraft:entity.creaking.activate";
/// `SoundEvents.CREAKING_DEACTIVATE` (`Creaking.java:501`).
const CREAKING_DEACTIVATE: &str = "minecraft:entity.creaking.deactivate";
/// `SoundEvents.CREAKING_FREEZE` (`Creaking.java:232`).
const CREAKING_FREEZE: &str = "minecraft:entity.creaking.freeze";
/// `SoundEvents.CREAKING_UNFREEZE` (`Creaking.java:229`).
const CREAKING_UNFREEZE: &str = "minecraft:entity.creaking.unfreeze";

/// `Creaking.ACTIVATION_RANGE_SQ` (`Creaking.java:67`), the squared distance a
/// staring player must be inside for `activate` (`:475`).
const ACTIVATION_RANGE_SQ: f64 = 144.0;
/// `LivingEntity.isLookingAtMe`'s cone size as `Creaking.checkCanMove` passes
/// it (`Creaking.java:470`).
const GAZE_CONE: f64 = 0.5;

///
/// The order is the source's: the two animation decrements, the decision
/// (`checkCanMove`, which may activate or deactivate), and only when the answer
/// differs from the row's current value the `ENTITY_ACTION` event, the
/// `stopInPlace` + freeze sound or the unfreeze sound, and the `CAN_MOVE` write.
///
/// Answers false when the level refuses a write, the way the other entity
/// hooks do.
pub(crate) fn before_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_CREAKING_ID {
        return true;
    }
    // The two animation decrements (`:215-216`, `:219-220`) run at the head of
    // the tick body, before this hook (`tick_runtime`).
    let Some(now_can_move) = check_can_move(entity, store, level) else {
        return false;
    };
    let can_move = entity.creaking_can_move();
    if now_can_move != can_move {
        // `:227` `gameEvent(GameEvent.ENTITY_ACTION)`, before either sound.
        level.game_event_at(
            VanillaGameEvent::ENTITY_ACTION,
            entity.position(),
            Some(entity.id()),
        );
        if now_can_move {
            // `:229` `makeSound(SoundEvents.CREAKING_UNFREEZE)`.
            play_body_sound(entity, store, CREAKING_UNFREEZE);
        } else {
            // `:231-232`: `stopInPlace()` then the freeze sound.
            stop_in_place(entity, store);
            play_body_sound(entity, store, CREAKING_FREEZE);
        }
        // `:236` `entityData.set(CAN_MOVE, nowCanMove)`.
        entity.set_creaking_can_move(now_can_move);
    }
    true
}

///
/// The player list is the `NEAREST_PLAYERS` memory the `PlayerSensor` wrote,
/// so this reads it once per tick and never scans the level itself; each id is
/// resolved through the level, which is where a session player lives
/// (`LevelCtx::player_by_entity_id`).
fn check_can_move(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<bool> {
    // `this.brain.getMemory(NEAREST_PLAYERS).orElse(List.of())` (`:455`).
    let players: Vec<i32> = match store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.brain_memory(MemoryModuleType::NearestPlayers))
    {
        Some(MemoryValue::EntityList(ids)) => ids.clone(),
        _ => Vec::new(),
    };
    let active = entity.creaking_is_active();
    if players.is_empty() {
        // `:457-462`.
        if active {
            deactivate(entity, store, level);
        }
        return Some(true);
    }

    let mut has_potential_target = false;
    for player_id in players {
        // `Level.getEntity(id)`: a player this level no longer holds is not a
        // target, which is the branch a logged-out session takes.
        let Some(player) = level.player_by_entity_id(player_id) else {
            continue;
        };
        // `this.canAttack(player) && !this.isAlliedTo(player)` (`:467`).
        if !can_attack_player(entity, store, &player) || is_allied_to(entity, store, level, &player)
        {
            continue;
        }
        has_potential_target = true;
        // `!active || PLAYER_NOT_WEARING_DISGUISE_ITEM.test(player)` (`:469`).
        if (!active || crate::entity::living::gaze::not_wearing_disguise(level, &player))
            && is_looking_at_me(entity, level, &player)?
        {
            if active {
                return Some(false);
            }
            // `player.distanceToSqr(this) < 144.0` (`:475`).
            if player_distance_to_sqr(entity, &player) < ACTIVATION_RANGE_SQ {
                activate(entity, store, level, player);
                return Some(false);
            }
        }
    }

    // `:483-485`.
    if !has_potential_target && active {
        deactivate(entity, store, level);
    }
    Some(true)
}

/// `Creaking.java:467`; `LivingEntity.java:927-936`; `Player.java:723-724`:
/// `target.canBeSeenAsEnemy()` is `!isInvulnerable() && !abilities.invulnerable && canBeSeenByAnyone()`,
/// whose own terms are `!isSpectator()` and `isAlive()`.
fn can_attack_player(entity: &Entity, store: &EntityStore, player: &NearbyPlayer) -> bool {
    let peaceful = store.difficulty() == Some(bbb_protocol::difficulties::Difficulty::Peaceful);
    if peaceful {
        return false;
    }
    let _ = entity;
    !player.invulnerable
        && !player.abilities_invulnerable
        && !player.spectator
        && !player.dead_or_dying
}

fn is_allied_to(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    player: &NearbyPlayer,
) -> bool {
    crate::entity::level::ScoreboardTeams::allied(
        level.scoreboard_teams().entity_team(entity.uuid()),
        level.player_team(store, player.uuid),
    )
}

fn is_looking_at_me(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    player: &NearbyPlayer,
) -> Option<bool> {
    let feet_y = entity.position().y;
    let eye_y = feet_y + f64::from(entity.eye_height());
    crate::entity::living::gaze::is_looking_at_me(
        level,
        entity.position(),
        &[
            eye_y,
            feet_y + 0.5 * body_scale(entity),
            (eye_y + feet_y) / 2.0,
        ],
        player,
        GAZE_CONE,
        false,
        true,
    )
}

fn body_scale(entity: &Entity) -> f64 {
    entity.living().map_or(1.0, |living| {
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
    })
}

fn player_distance_to_sqr(entity: &Entity, player: &NearbyPlayer) -> f64 {
    let dx = entity.position().x - player.position.x;
    let dy = entity.position().y - player.position.y;
    let dz = entity.position().z - player.position.z;
    dx * dx + dy * dy + dz * dz
}

fn activate(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    player: NearbyPlayer,
) {
    // `getBrain().setMemory(ATTACK_TARGET, player)`.
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.set_brain_memory(
            MemoryModuleType::AttackTarget,
            MemoryValue::EntityRef(player.entity_id),
        );
    }
    level.game_event_at(
        VanillaGameEvent::ENTITY_ACTION,
        entity.position(),
        Some(entity.id()),
    );
    play_body_sound(entity, store, CREAKING_ACTIVATE);
    // `setIsActive(true)` (`:495`).
    entity.set_creaking_active(true);
}

fn deactivate(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    // `getBrain().eraseMemory(ATTACK_TARGET)`.
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.erase_brain_memory(MemoryModuleType::AttackTarget);
    }
    level.game_event_at(
        VanillaGameEvent::ENTITY_ACTION,
        entity.position(),
        Some(entity.id()),
    );
    play_body_sound(entity, store, CREAKING_DEACTIVATE);
    // `setIsActive(false)` (`:502`).
    entity.set_creaking_active(false);
}

///
/// `resetAngularLeashMomentum` (`:446`) is the leash's own angular momentum,
/// which this crate's leash owner does not carry.
fn stop_in_place(entity: &mut Entity, store: &mut EntityStore) {
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.stop_navigation();
    }
    if let Some(living) = entity.living_mut() {
        living.xxa = 0.0;
        living.yya = 0.0;
        living.set_speed(0.0);
    }
    entity.set_delta_movement(Vec3d::default());
}

fn play_body_sound(entity: &Entity, store: &mut EntityStore, resource_id: &'static str) {
    if entity.is_silent() {
        return;
    }
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_owned(),
        source: SoundSource::Hostile,
        volume: 1.0,
        pitch: 1.0,
    });
}
