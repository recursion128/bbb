use bbb_protocol::packets::{
    Explosion as ProtocolExplosion, LevelParticles as ProtocolLevelParticles,
    Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientEffectsState {
    #[serde(default)]
    pub last_explosion: Option<ExplosionEventState>,
    #[serde(default)]
    pub last_level_particles: Option<LevelParticlesEventState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplosionEventState {
    pub center: ProtocolVec3d,
    pub radius: f32,
    pub block_count: i32,
    pub player_knockback: Option<ProtocolVec3d>,
    pub raw_effect_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelParticlesEventState {
    pub override_limiter: bool,
    pub always_show: bool,
    pub position: ProtocolVec3d,
    pub offset: ProtocolVec3d,
    pub max_speed: f32,
    pub count: i32,
    pub particle_type_id: i32,
    pub raw_options_len: usize,
}

impl WorldStore {
    pub fn apply_explosion(&mut self, packet: ProtocolExplosion) -> ExplosionEventState {
        self.counters.explosion_packets += 1;
        if let (Some(knockback), Some(pose)) =
            (packet.player_knockback, self.local_player.pose.as_mut())
        {
            if vec3_is_finite(knockback) {
                pose.delta_movement.x += knockback.x;
                pose.delta_movement.y += knockback.y;
                pose.delta_movement.z += knockback.z;
            }
        }
        let state = ExplosionEventState {
            center: packet.center,
            radius: packet.radius,
            block_count: packet.block_count,
            player_knockback: packet.player_knockback,
            raw_effect_payload_len: packet.raw_effect_payload.len(),
        };
        self.client_effects.last_explosion = Some(state.clone());
        state
    }

    pub fn apply_level_particles(
        &mut self,
        packet: ProtocolLevelParticles,
    ) -> LevelParticlesEventState {
        self.counters.level_particles_packets += 1;
        let state = LevelParticlesEventState {
            override_limiter: packet.override_limiter,
            always_show: packet.always_show,
            position: packet.position,
            offset: packet.offset,
            max_speed: packet.max_speed,
            count: packet.count,
            particle_type_id: packet.particle.particle_type_id,
            raw_options_len: packet.particle.raw_options.len(),
        };
        self.client_effects.last_level_particles = Some(state.clone());
        state
    }

    pub fn client_effects(&self) -> &ClientEffectsState {
        &self.client_effects
    }

    pub fn last_explosion(&self) -> Option<&ExplosionEventState> {
        self.client_effects.last_explosion.as_ref()
    }

    pub fn last_level_particles(&self) -> Option<&LevelParticlesEventState> {
        self.client_effects.last_level_particles.as_ref()
    }
}

fn vec3_is_finite(vec: ProtocolVec3d) -> bool {
    vec.x.is_finite() && vec.y.is_finite() && vec.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalPlayerPoseState;
    use bbb_protocol::packets::ParticlePayload;

    #[test]
    fn tracks_last_world_effect_events_and_counters() {
        let mut store = WorldStore::new();

        let explosion = store.apply_explosion(ProtocolExplosion {
            center: ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            radius: 4.5,
            block_count: 7,
            player_knockback: Some(ProtocolVec3d {
                x: 0.25,
                y: -0.5,
                z: 1.5,
            }),
            raw_effect_payload: vec![0x2d, 0x2a, 0x01, 0x00],
        });
        let expected_explosion = ExplosionEventState {
            center: ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            radius: 4.5,
            block_count: 7,
            player_knockback: Some(ProtocolVec3d {
                x: 0.25,
                y: -0.5,
                z: 1.5,
            }),
            raw_effect_payload_len: 4,
        };
        assert_eq!(explosion, expected_explosion);
        assert_eq!(store.last_explosion(), Some(&expected_explosion));

        let level_particles = store.apply_level_particles(ProtocolLevelParticles {
            override_limiter: true,
            always_show: false,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: ProtocolVec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle: ParticlePayload {
                particle_type_id: 45,
                raw_options: vec![0xaa, 0xbb],
            },
        });
        let expected_level_particles = LevelParticlesEventState {
            override_limiter: true,
            always_show: false,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: ProtocolVec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle_type_id: 45,
            raw_options_len: 2,
        };
        assert_eq!(level_particles, expected_level_particles);
        assert_eq!(
            store.last_level_particles(),
            Some(&expected_level_particles)
        );

        let counters = store.counters();
        assert_eq!(counters.explosion_packets, 1);
        assert_eq!(counters.level_particles_packets, 1);
    }

    #[test]
    fn explosion_knockback_adds_to_local_player_delta_movement() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            delta_movement: vec3(0.5, -0.25, 1.0),
            ..LocalPlayerPoseState::default()
        });

        store.apply_explosion(explosion_with_knockback(Some(vec3(0.25, 0.5, -1.5))));

        assert_eq!(
            store.local_player_pose().unwrap().delta_movement,
            vec3(0.75, 0.25, -0.5)
        );
    }

    #[test]
    fn explosion_without_finite_knockback_does_not_change_local_player_delta_movement() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            delta_movement: vec3(0.5, -0.25, 1.0),
            ..LocalPlayerPoseState::default()
        });

        store.apply_explosion(explosion_with_knockback(None));
        assert_eq!(
            store.local_player_pose().unwrap().delta_movement,
            vec3(0.5, -0.25, 1.0)
        );

        store.apply_explosion(explosion_with_knockback(Some(vec3(f64::NAN, 1.0, 1.0))));
        assert_eq!(
            store.local_player_pose().unwrap().delta_movement,
            vec3(0.5, -0.25, 1.0)
        );
    }

    fn explosion_with_knockback(player_knockback: Option<ProtocolVec3d>) -> ProtocolExplosion {
        ProtocolExplosion {
            center: vec3(1.0, 2.0, 3.0),
            radius: 4.5,
            block_count: 7,
            player_knockback,
            raw_effect_payload: Vec::new(),
        }
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }
}
