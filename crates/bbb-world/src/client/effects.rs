use bbb_protocol::packets::{
    Explosion as ProtocolExplosion, LevelParticles as ProtocolLevelParticles,
    Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::entities::{
    vanilla_living_entity_type, EntityPickBoundsState, EntityVec3, VANILLA_ENTITY_TYPE_RAVAGER_ID,
};
use crate::{LocalPlayerPoseState, WorldStore};

const LOCAL_PLAYER_BODY_WIDTH: f64 = 0.6;
const RAVAGER_ROAR_TARGET_INFLATE: f64 = 4.0;
const RAVAGER_ROAR_KNOCKBACK_HORIZONTAL_SCALE: f64 = 4.0;
const RAVAGER_ROAR_KNOCKBACK_VERTICAL: f64 = 0.2;
const RAVAGER_ROAR_MIN_HORIZONTAL_DISTANCE_SQUARED: f64 = 0.001;

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

    pub(crate) fn apply_ravager_roar_knockback(&mut self, ravager_id: i32) -> bool {
        let Some(local_player_id) = self.local_player_id else {
            return false;
        };
        if local_player_id == ravager_id || self.local_player_is_dead() {
            return false;
        }

        let Some(local_identity) = self.entities.identity(local_player_id) else {
            return false;
        };
        if local_identity.entity_type_id == VANILLA_ENTITY_TYPE_RAVAGER_ID
            || !vanilla_living_entity_type(local_identity.entity_type_id)
        {
            return false;
        }

        let Some(ravager_identity) = self.entities.identity(ravager_id) else {
            return false;
        };
        if ravager_identity.entity_type_id != VANILLA_ENTITY_TYPE_RAVAGER_ID {
            return false;
        }

        let Some(mut pose) = self.local_player.pose else {
            return false;
        };
        let Some(ravager_transform) = self.entities.transform(ravager_id) else {
            return false;
        };
        let Some(ravager_bounds) = self.entities.pick_bounds(ravager_id) else {
            return false;
        };
        let ravager_box = entity_pick_bounds_aabb(ravager_transform.position, ravager_bounds)
            .inflate(RAVAGER_ROAR_TARGET_INFLATE);
        if !ravager_box.intersects(local_player_pose_aabb(pose)) {
            return false;
        }

        let xd = pose.position.x - ravager_transform.position.x;
        let zd = pose.position.z - ravager_transform.position.z;
        let dd = (xd * xd + zd * zd).max(RAVAGER_ROAR_MIN_HORIZONTAL_DISTANCE_SQUARED);
        if !(xd.is_finite() && zd.is_finite() && dd.is_finite()) {
            return false;
        }
        pose.delta_movement.x += xd / dd * RAVAGER_ROAR_KNOCKBACK_HORIZONTAL_SCALE;
        pose.delta_movement.y += RAVAGER_ROAR_KNOCKBACK_VERTICAL;
        pose.delta_movement.z += zd / dd * RAVAGER_ROAR_KNOCKBACK_HORIZONTAL_SCALE;
        self.local_player.pose = Some(pose);
        true
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

#[derive(Debug, Clone, Copy)]
struct WorldAabb {
    min: [f64; 3],
    max: [f64; 3],
}

impl WorldAabb {
    fn inflate(self, value: f64) -> Self {
        Self {
            min: [
                self.min[0] - value,
                self.min[1] - value,
                self.min[2] - value,
            ],
            max: [
                self.max[0] + value,
                self.max[1] + value,
                self.max[2] + value,
            ],
        }
    }

    fn intersects(self, other: Self) -> bool {
        self.min[0] < other.max[0]
            && self.max[0] > other.min[0]
            && self.min[1] < other.max[1]
            && self.max[1] > other.min[1]
            && self.min[2] < other.max[2]
            && self.max[2] > other.min[2]
    }
}

fn entity_pick_bounds_aabb(position: EntityVec3, bounds: EntityPickBoundsState) -> WorldAabb {
    WorldAabb {
        min: [
            position.x + f64::from(bounds.min[0]),
            position.y + f64::from(bounds.min[1]),
            position.z + f64::from(bounds.min[2]),
        ],
        max: [
            position.x + f64::from(bounds.max[0]),
            position.y + f64::from(bounds.max[1]),
            position.z + f64::from(bounds.max[2]),
        ],
    }
}

fn local_player_pose_aabb(pose: LocalPlayerPoseState) -> WorldAabb {
    let half_width = LOCAL_PLAYER_BODY_WIDTH * 0.5;
    WorldAabb {
        min: [
            pose.position.x - half_width,
            pose.position.y,
            pose.position.z - half_width,
        ],
        max: [
            pose.position.x + half_width,
            pose.position.y + pose.body_height(),
            pose.position.z + half_width,
        ],
    }
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
