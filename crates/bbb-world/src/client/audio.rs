use std::collections::BTreeMap;

use bbb_protocol::packets::{
    SoundEntityEvent as ProtocolSoundEntityEvent, SoundEvent as ProtocolSoundEvent,
    SoundEventHolder as ProtocolSoundEventHolder, StopSound as ProtocolStopSound,
    Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientAudioState {
    #[serde(default)]
    pub last_sound: Option<SoundEventState>,
    #[serde(default)]
    pub last_sound_entity: Option<SoundEntityEventState>,
    #[serde(default)]
    pub last_stop_sound: Option<StopSoundEventState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEventState {
    pub sound: SoundHolderState,
    pub source: String,
    pub position: ProtocolVec3d,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEntityEventState {
    pub sound: SoundHolderState,
    pub source: String,
    pub entity_id: i32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopSoundEventState {
    pub source: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundHolderState {
    pub kind: String,
    pub registry_id: Option<i32>,
    pub location: Option<String>,
    pub fixed_range: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldBlockSoundProfile {
    pub hit_sound: String,
    pub volume: f32,
    pub pitch: f32,
}

impl WorldStore {
    pub fn set_default_block_sound_profiles(
        &mut self,
        profiles: BTreeMap<String, WorldBlockSoundProfile>,
    ) {
        self.default_block_sound_profiles = profiles
            .into_iter()
            .filter(|(block_name, profile)| {
                !block_name.is_empty()
                    && !profile.hit_sound.is_empty()
                    && profile.volume.is_finite()
                    && profile.pitch.is_finite()
            })
            .collect();
    }

    pub fn apply_sound_event(&mut self, packet: ProtocolSoundEvent) -> SoundEventState {
        self.counters.sound_packets += 1;
        let state = SoundEventState {
            sound: sound_holder_state(packet.sound),
            source: packet.source.as_str().to_string(),
            position: packet.position,
            volume: packet.volume,
            pitch: packet.pitch,
            seed: packet.seed,
        };
        self.client_audio.last_sound = Some(state.clone());
        state
    }

    pub fn apply_sound_entity_event(
        &mut self,
        packet: ProtocolSoundEntityEvent,
    ) -> Option<SoundEntityEventState> {
        self.counters.sound_entity_packets += 1;
        let Some(is_silent) = self.entities.is_silent(packet.entity_id) else {
            self.counters.sound_entity_events_ignored += 1;
            return None;
        };
        if is_silent {
            self.counters.sound_entity_events_ignored += 1;
            return None;
        }
        let state = SoundEntityEventState {
            sound: sound_holder_state(packet.sound),
            source: packet.source.as_str().to_string(),
            entity_id: packet.entity_id,
            volume: packet.volume,
            pitch: packet.pitch,
            seed: packet.seed,
        };
        self.client_audio.last_sound_entity = Some(state.clone());
        self.counters.sound_entity_events_applied += 1;
        Some(state)
    }

    pub fn apply_stop_sound(&mut self, packet: ProtocolStopSound) -> StopSoundEventState {
        self.counters.stop_sound_packets += 1;
        let state = StopSoundEventState {
            source: packet.source.map(|source| source.as_str().to_string()),
            name: packet.name,
        };
        self.client_audio.last_stop_sound = Some(state.clone());
        state
    }

    pub fn client_audio(&self) -> &ClientAudioState {
        &self.client_audio
    }

    pub fn last_sound(&self) -> Option<&SoundEventState> {
        self.client_audio.last_sound.as_ref()
    }

    pub fn last_sound_entity(&self) -> Option<&SoundEntityEventState> {
        self.client_audio.last_sound_entity.as_ref()
    }

    pub fn last_stop_sound(&self) -> Option<&StopSoundEventState> {
        self.client_audio.last_stop_sound.as_ref()
    }

    pub fn local_block_hit_sound(&self, pos: crate::BlockPos) -> Option<SoundEventState> {
        let block = self.probe_block(pos)?;
        let block_name = block.block_name.as_deref()?;
        let profile = self.default_block_sound_profiles.get(block_name)?;
        Some(SoundEventState {
            sound: SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some(profile.hit_sound.clone()),
                fixed_range: None,
            },
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume: (profile.volume + 1.0) / 8.0,
            pitch: profile.pitch * 0.5,
            seed: 0,
        })
    }
}

fn sound_holder_state(sound: ProtocolSoundEventHolder) -> SoundHolderState {
    match sound {
        ProtocolSoundEventHolder::Reference { registry_id } => SoundHolderState {
            kind: "reference".to_string(),
            registry_id: Some(registry_id),
            location: None,
            fixed_range: None,
        },
        ProtocolSoundEventHolder::Direct {
            location,
            fixed_range,
        } => SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(location),
            fixed_range,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{AddEntity, SoundSource, StopSound};
    use uuid::Uuid;

    #[test]
    fn tracks_last_sound_events_and_counters() {
        let mut store = WorldStore::new();

        let sound = store.apply_sound_event(ProtocolSoundEvent {
            sound: ProtocolSoundEventHolder::Reference { registry_id: 41 },
            source: SoundSource::Blocks,
            position: ProtocolVec3d {
                x: 2.5,
                y: -1.0,
                z: 0.0,
            },
            volume: 0.75,
            pitch: 1.25,
            seed: 123456789,
        });
        let expected_sound = SoundEventState {
            sound: SoundHolderState {
                kind: "reference".to_string(),
                registry_id: Some(41),
                location: None,
                fixed_range: None,
            },
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: 2.5,
                y: -1.0,
                z: 0.0,
            },
            volume: 0.75,
            pitch: 1.25,
            seed: 123456789,
        };
        assert_eq!(sound, expected_sound);
        assert_eq!(store.last_sound(), Some(&expected_sound));

        store.apply_add_entity(protocol_add_entity(123));
        let entity_sound = store
            .apply_sound_entity_event(ProtocolSoundEntityEvent {
                sound: ProtocolSoundEventHolder::Direct {
                    location: "minecraft:entity.cat.ambient".to_string(),
                    fixed_range: Some(32.0),
                },
                source: SoundSource::Neutral,
                entity_id: 123,
                volume: 1.0,
                pitch: 0.5,
                seed: -9,
            })
            .unwrap();
        let expected_entity_sound = SoundEntityEventState {
            sound: SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some("minecraft:entity.cat.ambient".to_string()),
                fixed_range: Some(32.0),
            },
            source: "neutral".to_string(),
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        };
        assert_eq!(entity_sound, expected_entity_sound);
        assert_eq!(store.last_sound_entity(), Some(&expected_entity_sound));

        assert!(store
            .apply_sound_entity_event(ProtocolSoundEntityEvent {
                sound: ProtocolSoundEventHolder::Reference { registry_id: 99 },
                source: SoundSource::Hostile,
                entity_id: 404,
                volume: 0.2,
                pitch: 1.8,
                seed: 7,
            })
            .is_none());
        assert_eq!(
            store.last_sound_entity().map(|state| state.entity_id),
            Some(123)
        );

        assert!(
            store.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 123,
                values: vec![bbb_protocol::packets::EntityDataValue {
                    data_id: 4,
                    serializer_id: 8,
                    value: bbb_protocol::packets::EntityDataValueKind::Boolean(true),
                }],
            })
        );
        assert!(store
            .apply_sound_entity_event(ProtocolSoundEntityEvent {
                sound: ProtocolSoundEventHolder::Direct {
                    location: "minecraft:entity.sheep.ambient".to_string(),
                    fixed_range: None,
                },
                source: SoundSource::Neutral,
                entity_id: 123,
                volume: 1.0,
                pitch: 1.0,
                seed: 3,
            })
            .is_none());
        assert_eq!(
            store.last_sound_entity().map(|state| state.entity_id),
            Some(123)
        );

        let stop_sound = store.apply_stop_sound(StopSound {
            source: Some(SoundSource::Music),
            name: Some("minecraft:music.menu".to_string()),
        });
        let expected_stop_sound = StopSoundEventState {
            source: Some("music".to_string()),
            name: Some("minecraft:music.menu".to_string()),
        };
        assert_eq!(stop_sound, expected_stop_sound);
        assert_eq!(store.last_stop_sound(), Some(&expected_stop_sound));
        assert_eq!(store.counters().sound_packets, 1);
        assert_eq!(store.counters().sound_entity_packets, 3);
        assert_eq!(store.counters().sound_entity_events_applied, 1);
        assert_eq!(store.counters().sound_entity_events_ignored, 2);
        assert_eq!(store.counters().stop_sound_packets, 1);
    }

    fn protocol_add_entity(id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id: 7,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }
}
