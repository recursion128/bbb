//! Zombies approach and stomp exposed turtle eggs.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_TURTLE_EGG_ID;
use bbb_protocol::item_types::VANILLA_ITEM_EGG_ID;
use bbb_protocol::packets::{
    BlockPos, ItemStackTemplateSummary, LevelParticles, ParticleOptions, SoundSource, Vec3d,
};
use bbb_protocol::particle_types::{VANILLA_PARTICLE_TYPE_ITEM_ID, VANILLA_PARTICLE_TYPE_POOF_ID};
use bbb_server_types::fluid::block_state_is_air;

use super::target::TargetView;
use super::MobAiContext;
use crate::entity::level::EntityBlockContext;
use crate::entity::{Entity, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;

fn is_egg(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.has_ticking_column_at(pos)
        && vanilla_block_state_block_type_id(level.block_state(pos))
            == Some(VANILLA_BLOCK_TURTLE_EGG_ID)
}

pub(super) fn valid_egg_target(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    is_egg(level, pos)
        && [1, 2].into_iter().all(|dy| {
            block_state_is_air(level.block_state(BlockPos {
                y: pos.y + dy,
                ..pos
            }))
        })
}

pub(super) fn tick_stomp_egg(
    context: &MobAiContext,
    targets: &mut TargetView<'_, '_, '_>,
    stomp_ticks: u16,
) {
    let body_pos = targets.body_entity().block_position();
    let nearby = [
        (0, 0, 0),
        (0, -1, 0),
        (-1, 0, 0),
        (1, 0, 0),
        (0, 0, -1),
        (0, 0, 1),
        (0, -2, 0),
    ]
    .into_iter()
    .map(|(dx, dy, dz)| BlockPos {
        x: body_pos.x + dx,
        y: body_pos.y + dy,
        z: body_pos.z + dz,
    })
    .find(|pos| is_egg(targets.level(), *pos));
    let Some(pos) = nearby else {
        return;
    };
    // Stomping changes vertical motion, not a normal jump.
    targets.set_body_delta_movement(Vec3d {
        y: if stomp_ticks.is_multiple_of(2) {
            -0.3
        } else {
            0.3
        },
        ..context.body.delta_movement
    });
    if let Some((entity, store, level)) = targets.world_mut() {
        stomp_egg(entity, store, level, pos, stomp_ticks);
    }
}

pub(super) fn stomp_egg(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    ticks: u16,
) {
    if !level.mob_griefing() || !is_egg(level, pos) {
        return;
    }
    let position = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.7,
        z: f64::from(pos.z) + 0.5,
    };
    if ticks.is_multiple_of(6) {
        let pitch = 0.9 + level.level_random().next_float() * 0.2;
        store.play_entity_sound(PendingEntitySound {
            position: Vec3d {
                y: f64::from(pos.y) + 0.5,
                ..position
            },
            resource_id: "minecraft:entity.zombie.destroy_egg".to_owned(),
            source: SoundSource::Hostile,
            volume: 0.5,
            pitch,
        });
    }
    let (particle, count, y) = if ticks > 60 {
        // RemoveBlockGoal: remove the whole block without drops.
        if !level.set_entity_block(
            EntityBlockContext::new(Some(&mut *store), Some(entity)),
            pos,
            crate::block_behavior::default_block_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
            ),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ) {
            return;
        }
        let pitch = 0.9 + level.level_random().next_float() * 0.2;
        level.play_block_sound_with_volume_pitch(
            pos,
            "minecraft:entity.turtle.egg_break",
            0.7,
            pitch,
        );
        (
            ParticleOptions::Simple.payload(VANILLA_PARTICLE_TYPE_POOF_ID),
            20,
            f64::from(pos.y),
        )
    } else if ticks > 0 {
        (
            ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
                item_id: VANILLA_ITEM_EGG_ID,
                count: 1,
                component_patch: Default::default(),
            }))
            .payload(VANILLA_PARTICLE_TYPE_ITEM_ID),
            3,
            position.y,
        )
    } else {
        return;
    };
    let Ok(particle) = particle else {
        return;
    };
    store.send_particles(LevelParticles {
        override_limiter: false,
        always_show: false,
        position: Vec3d { y, ..position },
        offset: Vec3d {
            x: 0.04,
            y: 0.04,
            z: 0.04,
        },
        max_speed: 0.15,
        count,
        particle,
    });
}
