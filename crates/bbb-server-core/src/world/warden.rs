//! The physical sonic wave, through the ordinary player and stored hurt paths.

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID, VANILLA_ATTRIBUTE_SCALE_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;
use bbb_protocol::packets::{LevelParticles, ParticlePayload, SoundSource, Vec3d};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SONIC_BOOM_ID;
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::warden::sonic_boom::within_range;
use crate::entity::PendingEntitySound;
use crate::level_event::PendingHurtEntity;
use crate::world::{LevelId, ServerWorld, WorldEvent};

impl ServerWorld {
    // SonicBoom.java:68-90: the wave reaches through walls and pushes only after successful hurt.
    pub(in crate::world) fn apply_warden_sonic_boom(
        &mut self,
        dimension: &LevelId,
        source_entity_id: i32,
        target_entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(body) = self.level_by_id(dimension).entities.get(source_entity_id) else {
            return;
        };
        if body.entity_type_id() != VANILLA_ENTITY_TYPE_WARDEN_ID
            || !body.is_alive()
            || self.warden_can_target(dimension, body, target_entity_id) != Some(true)
        {
            return;
        }
        let body_position = body.position();
        // EntityType.java:1047-1055; LivingEntity.java:3663-3669; EntityDimensions.java:25-39.
        let scale = body
            .living()
            .expect("warden living state")
            .attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32;
        let source = Vec3d {
            y: body_position.y + f64::from(1.6_f32) * f64::from(scale),
            ..body_position
        };
        let silent = body.is_silent();
        let player_connection = self.connection_by_entity_id(target_entity_id);
        let (target_position, eye_position, resistance) = if let Some(connection) =
            player_connection
        {
            let player = &self.levels[connection];
            let Ok(eye_position) = player.eye_position() else {
                return;
            };
            (
                player.position(),
                eye_position,
                player
                    .attributes
                    .value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
                    .unwrap_or(0.0),
            )
        } else {
            let Some(target) = self.level_by_id(dimension).entities.get(target_entity_id) else {
                return;
            };
            (
                target.position(),
                Vec3d {
                    y: target.position().y + f64::from(target.eye_height()),
                    ..target.position()
                },
                target
                    .living()
                    .expect("targetable living state")
                    .attribute_value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID),
            )
        };
        if !within_range(body_position, target_position) {
            return;
        }
        let delta = Vec3d {
            x: eye_position.x - source.x,
            y: eye_position.y - source.y,
            z: eye_position.z - source.z,
        };
        let direction = delta.normalize();
        let steps = delta.length_sqr().sqrt().floor() as i32 + 7;
        let store = &mut self.level_mut_by_id(dimension).entities;
        for step in 1..steps {
            store.send_particles(LevelParticles {
                override_limiter: false,
                always_show: false,
                position: Vec3d {
                    x: source.x + direction.x * f64::from(step),
                    y: source.y + direction.y * f64::from(step),
                    z: source.z + direction.z * f64::from(step),
                },
                offset: Vec3d::default(),
                max_speed: 0.0,
                count: 1,
                particle: ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_SONIC_BOOM_ID,
                    raw_options: Vec::new(),
                },
            });
        }
        if !silent {
            store.play_entity_sound(PendingEntitySound {
                position: body_position,
                resource_id: "minecraft:entity.warden.sonic_boom".to_owned(),
                source: SoundSource::Hostile,
                volume: 3.0,
                pitch: 1.0,
            });
        }
        let Some(attacker) = self.pending_hurt_source_entity(
            dimension,
            PendingHurtEntity::Stored {
                entity_id: source_entity_id,
            },
        ) else {
            return;
        };
        let hurt = if let Some(connection) = player_connection {
            self.hurt_connected_player(
                connection,
                "minecraft:sonic_boom",
                10.0,
                Some(attacker.clone()),
                Some(attacker),
                None,
                events,
            )
        } else {
            self.hurt_stored_entity_by_command(
                *dimension,
                target_entity_id,
                "minecraft:sonic_boom",
                10.0,
                Some(attacker.clone()),
                Some(attacker),
                None,
                events,
            )
            .is_some_and(|outcome| outcome.succeeded)
        };
        if hurt {
            let horizontal = 2.5 * (1.0 - resistance);
            let vertical = 0.5 * (1.0 - resistance);
            let impulse = Vec3d {
                x: direction.x * horizontal,
                y: direction.y * vertical,
                z: direction.z * horizontal,
            };
            if let Some(connection) = player_connection {
                if let Some(player) = self.levels.player_mut(connection) {
                    player.push_impulse(impulse);
                }
            } else if let Some(target) = self
                .level_mut_by_id(dimension)
                .entities
                .get_mut(target_entity_id)
            {
                crate::entity::push(target, impulse.x, impulse.y, impulse.z);
            }
        }
    }
}
