use std::collections::BTreeMap;

use bbb_protocol::packets::{
    LevelEvent as ProtocolLevelEvent, SoundEntityEvent as ProtocolSoundEntityEvent,
    SoundEvent as ProtocolSoundEvent, SoundEventHolder as ProtocolSoundEventHolder,
    StopSound as ProtocolStopSound, Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const BLOCK_BREAK_LEVEL_EVENT: i32 = 2001;

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
    #[serde(default)]
    pub break_sound: String,
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
        Some(block_sound_state(
            pos,
            &profile.hit_sound,
            (profile.volume + 1.0) / 8.0,
            profile.pitch * 0.5,
            "block",
        ))
    }

    pub fn level_event_block_break_sound(
        &self,
        event: ProtocolLevelEvent,
    ) -> Option<SoundEventState> {
        self.level_event_block_break_sound_state(event)
    }

    pub fn level_event_sound(&self, event: ProtocolLevelEvent) -> Option<SoundEventState> {
        if let Some(state) = self.level_event_block_break_sound_state(event) {
            return Some(state);
        }
        let sound = fixed_level_event_sound(event.event_type)?;
        Some(block_sound_state(
            crate::protocol_block_pos(event.pos),
            sound.event_id,
            sound.volume,
            sound.pitch,
            sound.source,
        ))
    }

    pub fn level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if let Some(state) = self.level_event_sound(event) {
            return Some(state);
        }
        let sound = random_level_event_sound(event.event_type, event.data, &mut next_float)?;
        Some(block_sound_state(
            crate::protocol_block_pos(event.pos),
            sound.event_id,
            sound.volume,
            sound.pitch,
            sound.source,
        ))
    }

    fn level_event_block_break_sound_state(
        &self,
        event: ProtocolLevelEvent,
    ) -> Option<SoundEventState> {
        if event.event_type != BLOCK_BREAK_LEVEL_EVENT {
            return None;
        }
        let block_state = self.registries.block_state(event.data)?;
        if is_air_block_name(&block_state.name) {
            return None;
        }
        let profile = self.default_block_sound_profiles.get(&block_state.name)?;
        if profile.break_sound.is_empty() {
            return None;
        }
        Some(block_sound_state(
            crate::protocol_block_pos(event.pos),
            &profile.break_sound,
            (profile.volume + 1.0) / 2.0,
            profile.pitch * 0.8,
            "block",
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FixedLevelEventSound {
    event_id: &'static str,
    source: &'static str,
    volume: f32,
    pitch: f32,
}

fn fixed_level_event_sound(event_type: i32) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        1000 => FixedLevelEventSound {
            event_id: "minecraft:block.dispenser.dispense",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1001 => FixedLevelEventSound {
            event_id: "minecraft:block.dispenser.fail",
            source: "block",
            volume: 1.0,
            pitch: 1.2,
        },
        1002 => FixedLevelEventSound {
            event_id: "minecraft:block.dispenser.launch",
            source: "block",
            volume: 1.0,
            pitch: 1.2,
        },
        1004 => FixedLevelEventSound {
            event_id: "minecraft:entity.firework_rocket.shoot",
            source: "neutral",
            volume: 1.0,
            pitch: 1.2,
        },
        1033 => FixedLevelEventSound {
            event_id: "minecraft:block.chorus_flower.grow",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1034 => FixedLevelEventSound {
            event_id: "minecraft:block.chorus_flower.death",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1035 => FixedLevelEventSound {
            event_id: "minecraft:block.brewing_stand.brew",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1049 => FixedLevelEventSound {
            event_id: "minecraft:block.crafter.craft",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1050 => FixedLevelEventSound {
            event_id: "minecraft:block.crafter.fail",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1503 => FixedLevelEventSound {
            event_id: "minecraft:block.end_portal_frame.fill",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1505 => FixedLevelEventSound {
            event_id: "minecraft:item.bone_meal.use",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        _ => return None,
    };
    Some(sound)
}

fn random_level_event_sound(
    event_type: i32,
    data: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        1009 if data == 0 => FixedLevelEventSound {
            event_id: "minecraft:block.fire.extinguish",
            source: "block",
            volume: 0.5,
            pitch: triangle_pitch(2.6, 0.8, next_float),
        },
        1009 if data == 1 => FixedLevelEventSound {
            event_id: "minecraft:entity.generic.extinguish_fire",
            source: "block",
            volume: 0.7,
            pitch: triangle_pitch(1.6, 0.4, next_float),
        },
        1015 => hostile_triangle("minecraft:entity.ghast.warn", 10.0, next_float),
        1016 => hostile_triangle("minecraft:entity.ghast.shoot", 10.0, next_float),
        1017 => hostile_triangle("minecraft:entity.ender_dragon.shoot", 10.0, next_float),
        1018 => hostile_triangle("minecraft:entity.blaze.shoot", 2.0, next_float),
        1019 => hostile_triangle(
            "minecraft:entity.zombie.attack_wooden_door",
            2.0,
            next_float,
        ),
        1020 => hostile_triangle("minecraft:entity.zombie.attack_iron_door", 2.0, next_float),
        1021 => hostile_triangle("minecraft:entity.zombie.break_wooden_door", 2.0, next_float),
        1022 => hostile_triangle("minecraft:entity.wither.break_block", 2.0, next_float),
        1024 => hostile_triangle("minecraft:entity.wither.shoot", 2.0, next_float),
        1025 => FixedLevelEventSound {
            event_id: "minecraft:entity.bat.takeoff",
            source: "neutral",
            volume: 0.05,
            pitch: triangle_pitch(1.0, 0.2, next_float),
        },
        1026 => hostile_triangle("minecraft:entity.zombie.infect", 2.0, next_float),
        1027 => hostile_triangle(
            "minecraft:entity.zombie_villager.converted",
            2.0,
            next_float,
        ),
        1029 => block_ranged("minecraft:block.anvil.destroy", 1.0, next_float),
        1030 => block_ranged("minecraft:block.anvil.use", 1.0, next_float),
        1031 => block_ranged("minecraft:block.anvil.land", 0.3, next_float),
        1039 => FixedLevelEventSound {
            event_id: "minecraft:entity.phantom.bite",
            source: "hostile",
            volume: 0.3,
            pitch: ranged_pitch(0.9, 0.1, next_float),
        },
        1040 => hostile_triangle(
            "minecraft:entity.zombie.converted_to_drowned",
            2.0,
            next_float,
        ),
        1041 => hostile_triangle("minecraft:entity.husk.converted_to_zombie", 2.0, next_float),
        1042 => block_ranged("minecraft:block.grindstone.use", 1.0, next_float),
        1043 => block_ranged("minecraft:item.book.page_turn", 1.0, next_float),
        1044 => block_ranged("minecraft:block.smithing_table.use", 1.0, next_float),
        1045 => block_ranged("minecraft:block.pointed_dripstone.land", 2.0, next_float),
        1046 => block_ranged(
            "minecraft:block.pointed_dripstone.drip_lava_into_cauldron",
            2.0,
            next_float,
        ),
        1047 => block_ranged(
            "minecraft:block.pointed_dripstone.drip_water_into_cauldron",
            2.0,
            next_float,
        ),
        1048 => hostile_triangle(
            "minecraft:entity.skeleton.converted_to_stray",
            2.0,
            next_float,
        ),
        1051 => FixedLevelEventSound {
            event_id: "minecraft:entity.wind_charge.throw",
            source: "block",
            volume: 0.5,
            pitch: 0.4 / (next_float().clamp(0.0, 1.0) * 0.4 + 0.8),
        },
        1501 => FixedLevelEventSound {
            event_id: "minecraft:block.lava.extinguish",
            source: "block",
            volume: 0.5,
            pitch: triangle_pitch(2.6, 0.8, next_float),
        },
        1502 => FixedLevelEventSound {
            event_id: "minecraft:block.redstone_torch.burnout",
            source: "block",
            volume: 0.5,
            pitch: triangle_pitch(2.6, 0.8, next_float),
        },
        _ => return None,
    };
    Some(sound)
}

fn hostile_triangle(
    event_id: &'static str,
    volume: f32,
    next_float: &mut impl FnMut() -> f32,
) -> FixedLevelEventSound {
    FixedLevelEventSound {
        event_id,
        source: "hostile",
        volume,
        pitch: triangle_pitch(1.0, 0.2, next_float),
    }
}

fn block_ranged(
    event_id: &'static str,
    volume: f32,
    next_float: &mut impl FnMut() -> f32,
) -> FixedLevelEventSound {
    FixedLevelEventSound {
        event_id,
        source: "block",
        volume,
        pitch: ranged_pitch(0.9, 0.1, next_float),
    }
}

fn triangle_pitch(mean: f32, spread: f32, next_float: &mut impl FnMut() -> f32) -> f32 {
    mean + (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * spread
}

fn ranged_pitch(min: f32, range: f32, next_float: &mut impl FnMut() -> f32) -> f32 {
    next_float().clamp(0.0, 1.0) * range + min
}

fn block_sound_state(
    pos: crate::BlockPos,
    sound: &str,
    volume: f32,
    pitch: f32,
    source: &str,
) -> SoundEventState {
    SoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(sound.to_string()),
            fixed_range: None,
        },
        source: source.to_string(),
        position: ProtocolVec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
        volume,
        pitch,
        seed: 0,
    }
}

fn is_air_block_name(name: &str) -> bool {
    matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
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
    use bbb_protocol::packets::{
        AddEntity, BlockPos as ProtocolBlockPos, LevelEvent, SoundSource, StopSound,
    };
    use std::collections::BTreeMap;
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

    #[test]
    fn level_event_2001_maps_block_state_id_to_vanilla_break_sound() {
        let mut store = WorldStore::new();
        store.set_default_block_sound_profiles(BTreeMap::from([(
            "minecraft:grass_block".to_string(),
            WorldBlockSoundProfile {
                break_sound: "minecraft:block.grass.break".to_string(),
                hit_sound: "minecraft:block.grass.hit".to_string(),
                volume: 0.8,
                pitch: 1.2,
            },
        )]));

        let sound = store
            .level_event_block_break_sound(LevelEvent {
                event_type: 2001,
                pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
                data: 9,
                global: false,
            })
            .unwrap();

        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.grass.break")
        );
        assert_eq!(sound.source, "block");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 2.5,
                y: 3.5,
                z: -3.5,
            }
        );
        assert_close(sound.volume, 0.9);
        assert_close(sound.pitch, 0.96);
        assert_eq!(sound.seed, 0);
        assert_eq!(
            store.level_event_sound(LevelEvent {
                event_type: 2001,
                pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
                data: 9,
                global: false,
            }),
            Some(sound)
        );

        assert!(store
            .level_event_block_break_sound(LevelEvent {
                event_type: 1001,
                pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
                data: 9,
                global: false,
            })
            .is_none());
        assert!(store
            .level_event_block_break_sound(LevelEvent {
                event_type: 2001,
                pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
                data: 0,
                global: false,
            })
            .is_none());
    }

    #[test]
    fn level_event_sound_maps_fixed_vanilla_level_event_audio() {
        let store = WorldStore::new();

        for (event_type, event_id, source, volume, pitch) in [
            (
                1000,
                "minecraft:block.dispenser.dispense",
                "block",
                1.0,
                1.0,
            ),
            (1001, "minecraft:block.dispenser.fail", "block", 1.0, 1.2),
            (1002, "minecraft:block.dispenser.launch", "block", 1.0, 1.2),
            (
                1004,
                "minecraft:entity.firework_rocket.shoot",
                "neutral",
                1.0,
                1.2,
            ),
            (
                1033,
                "minecraft:block.chorus_flower.grow",
                "block",
                1.0,
                1.0,
            ),
            (
                1034,
                "minecraft:block.chorus_flower.death",
                "block",
                1.0,
                1.0,
            ),
            (
                1035,
                "minecraft:block.brewing_stand.brew",
                "block",
                1.0,
                1.0,
            ),
            (1049, "minecraft:block.crafter.craft", "block", 1.0, 1.0),
            (1050, "minecraft:block.crafter.fail", "block", 1.0, 1.0),
            (
                1503,
                "minecraft:block.end_portal_frame.fill",
                "block",
                1.0,
                1.0,
            ),
            (1505, "minecraft:item.bone_meal.use", "block", 1.0, 1.0),
        ] {
            let sound = store
                .level_event_sound(LevelEvent {
                    event_type,
                    pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                    data: 7,
                    global: false,
                })
                .unwrap();
            assert_eq!(sound.sound.location.as_deref(), Some(event_id));
            assert_eq!(sound.source, source);
            assert_eq!(sound.position, vec3(-0.5, 70.5, 4.5));
            assert_close(sound.volume, volume);
            assert_close(sound.pitch, pitch);
        }

        assert!(store
            .level_event_sound(LevelEvent {
                event_type: 1015,
                pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                data: 0,
                global: false,
            })
            .is_none());
        assert!(store
            .level_event_sound(LevelEvent {
                event_type: 1501,
                pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                data: 0,
                global: false,
            })
            .is_none());
    }

    #[test]
    fn level_event_sound_with_random_maps_randomized_vanilla_audio() {
        let store = WorldStore::new();

        let fire_extinguish = random_level_event_sound(&store, 1009, 0, &[0.25, 0.75]);
        assert_eq!(
            fire_extinguish.sound.location.as_deref(),
            Some("minecraft:block.fire.extinguish")
        );
        assert_eq!(fire_extinguish.source, "block");
        assert_close(fire_extinguish.volume, 0.5);
        assert_close(fire_extinguish.pitch, 2.2);

        let generic_extinguish = random_level_event_sound(&store, 1009, 1, &[0.25, 0.75]);
        assert_eq!(
            generic_extinguish.sound.location.as_deref(),
            Some("minecraft:entity.generic.extinguish_fire")
        );
        assert_close(generic_extinguish.volume, 0.7);
        assert_close(generic_extinguish.pitch, 1.4);

        let ghast_warn = random_level_event_sound(&store, 1015, 0, &[0.75, 0.25]);
        assert_eq!(
            ghast_warn.sound.location.as_deref(),
            Some("minecraft:entity.ghast.warn")
        );
        assert_eq!(ghast_warn.source, "hostile");
        assert_close(ghast_warn.volume, 10.0);
        assert_close(ghast_warn.pitch, 1.1);

        let anvil_destroy = random_level_event_sound(&store, 1029, 0, &[0.5]);
        assert_eq!(
            anvil_destroy.sound.location.as_deref(),
            Some("minecraft:block.anvil.destroy")
        );
        assert_close(anvil_destroy.volume, 1.0);
        assert_close(anvil_destroy.pitch, 0.95);

        let wind_charge = random_level_event_sound(&store, 1051, 0, &[0.5]);
        assert_eq!(
            wind_charge.sound.location.as_deref(),
            Some("minecraft:entity.wind_charge.throw")
        );
        assert_eq!(wind_charge.source, "block");
        assert_close(wind_charge.volume, 0.5);
        assert_close(wind_charge.pitch, 0.4);

        assert!(store
            .level_event_sound_with_random(
                LevelEvent {
                    event_type: 1032,
                    pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                    data: 0,
                    global: false,
                },
                || 0.5,
            )
            .is_none());
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

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn random_level_event_sound(
        store: &WorldStore,
        event_type: i32,
        data: i32,
        samples: &[f32],
    ) -> SoundEventState {
        let mut samples = samples.iter().copied();
        store
            .level_event_sound_with_random(
                LevelEvent {
                    event_type,
                    pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                    data,
                    global: false,
                },
                || samples.next().unwrap(),
            )
            .unwrap()
    }
}
