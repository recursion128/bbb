//! Enderman relocation and carried-block actions.

pub(crate) mod blocks;

use crate::entity::level::fluid_state_tag_membership;
use crate::entity::living::random_teleport::{block_position, blocks_motion};
use crate::entity::{Entity, EntityStore};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::fluid::block_state_fluid_state;

pub(crate) fn random_teleport(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<bool> {
    if !body.is_alive() || !body.living().is_some_and(|living| living.health() > 0.0) {
        return Some(false);
    }
    let destination = random_destination(level.level_random(), body.position());
    teleport(body, store, level, destination)
}

// EnderMan.java:256-261.
pub(crate) fn random_destination(random: &GameRandom, position: Vec3d) -> Vec3d {
    Vec3d {
        x: position.x + (random.next_double() - 0.5) * 64.0,
        y: position.y + f64::from(random.next_int(64) - 32),
        z: position.z + (random.next_double() - 0.5) * 64.0,
    }
}

pub(crate) fn teleport(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    destination: Vec3d,
) -> Option<bool> {
    if !support_is_dry(level, destination)? {
        return Some(false);
    }
    let origin = body.position();
    let success =
        crate::entity::living::random_teleport::attempt(body, store, level, destination, true)?;
    if success {
        emit_teleport(body, store, level, origin);
    }
    Some(success)
}

fn support_is_dry(level: &LevelCtx<'_, '_>, destination: Vec3d) -> Option<bool> {
    let mut pos = block_position(destination);
    while pos.y > level.min_y() && !blocks_motion(level, pos)? {
        pos.y -= 1;
    }
    let support = blocks_motion(level, pos)?;
    let fluid = block_state_fluid_state(level.block_state_checked(pos)?);
    let wet = fluid_state_tag_membership(level, fluid, "minecraft:water");
    Some(support && !wet)
}

fn emit_teleport(
    body: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    origin: Vec3d,
) {
    level.game_event_at(
        crate::level_event::VanillaGameEvent::TELEPORT,
        origin,
        Some(body.id()),
    );
    if !body.is_silent() {
        for position in [body.old_position(), body.position()] {
            store.play_entity_sound(crate::entity::store::PendingEntitySound {
                position,
                resource_id: "minecraft:entity.enderman.teleport".to_owned(),
                source: crate::entity::living::living_sound_source(body),
                volume: 1.0,
                pitch: 1.0,
            });
        }
    }
}

#[cfg(test)]
mod tests;
