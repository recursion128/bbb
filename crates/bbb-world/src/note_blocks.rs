use std::collections::BTreeMap;

use bbb_protocol::packets::Vec3d as ProtocolVec3d;
use serde::{Deserialize, Serialize};

use crate::{
    BlockPos, LevelEventSoundRandomState, PlayApplyEffects, SoundEventState, SoundHolderState,
    WorldStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoteBlockParticleState {
    pub position: ProtocolVec3d,
    pub velocity: ProtocolVec3d,
}

enum NoteInstrumentSound {
    Tunable,
    Fixed(&'static str),
    Custom,
}

fn note_instrument_sound(instrument: &str) -> Option<NoteInstrumentSound> {
    let sound = match instrument {
        "harp" | "basedrum" | "snare" | "hat" | "bass" | "flute" | "bell" | "guitar" | "chime"
        | "xylophone" | "iron_xylophone" | "cow_bell" | "didgeridoo" | "bit" | "banjo"
        | "pling" | "trumpet" | "trumpet_exposed" | "trumpet_oxidized" | "trumpet_weathered" => {
            NoteInstrumentSound::Tunable
        }
        "zombie" => NoteInstrumentSound::Fixed("imitate.zombie"),
        "skeleton" => NoteInstrumentSound::Fixed("imitate.skeleton"),
        "creeper" => NoteInstrumentSound::Fixed("imitate.creeper"),
        "dragon" => NoteInstrumentSound::Fixed("imitate.ender_dragon"),
        "wither_skeleton" => NoteInstrumentSound::Fixed("imitate.wither_skeleton"),
        "piglin" => NoteInstrumentSound::Fixed("imitate.piglin"),
        "custom_head" => NoteInstrumentSound::Custom,
        _ => return None,
    };
    Some(sound)
}

impl WorldStore {
    /// Mirrors `NoteBlock.triggerEvent`: `b0`/`b1` are ignored; the current
    /// block state's instrument and note choose the particle, sound and pitch.
    pub(crate) fn trigger_note_block_event(
        &mut self,
        pos: BlockPos,
        properties: &BTreeMap<String, String>,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) -> bool {
        let Some((instrument_name, instrument)) =
            properties.get("instrument").and_then(|instrument| {
                note_instrument_sound(instrument).map(|kind| (instrument, kind))
            })
        else {
            return false;
        };

        let (sound, pitch) = match instrument {
            NoteInstrumentSound::Tunable => {
                let Some(note) = properties
                    .get("note")
                    .and_then(|note| note.parse::<u8>().ok())
                    .filter(|note| *note <= 24)
                else {
                    return false;
                };
                effects.note_block_particle(
                    self,
                    NoteBlockParticleState {
                        position: ProtocolVec3d {
                            x: f64::from(pos.x) + 0.5,
                            y: f64::from(pos.y) + 1.2,
                            z: f64::from(pos.z) + 0.5,
                        },
                        velocity: ProtocolVec3d {
                            x: f64::from(note) / 24.0,
                            y: 0.0,
                            z: 0.0,
                        },
                    },
                );
                (
                    format!("minecraft:block.note_block.{instrument_name}"),
                    2.0_f32.powf((f32::from(note) - 12.0) / 12.0),
                )
            }
            NoteInstrumentSound::Fixed(sound) => {
                (format!("minecraft:block.note_block.{sound}"), 1.0)
            }
            NoteInstrumentSound::Custom => {
                let above = BlockPos {
                    x: pos.x,
                    y: pos.y.saturating_add(1),
                    z: pos.z,
                };
                let Some(sound) = self.skull_note_block_sound_at(above) else {
                    return false;
                };
                (sound.to_string(), 1.0)
            }
        };

        let state = self.record_positioned_sound(SoundEventState {
            sound: SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some(sound),
                fixed_range: None,
            },
            source: "record".to_string(),
            position: ProtocolVec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume: 3.0,
            pitch,
            seed: random.next_long(),
            distance_delay: false,
        });
        effects.positioned_sound(&state);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::{
        block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID,
        block_types::{VANILLA_BLOCK_NOTE_BLOCK_ID, VANILLA_BLOCK_STONE_ID},
        packets::{
            BlockEntityData, BlockEvent, BlockPos as ProtocolBlockPos,
            BlockUpdate as ProtocolBlockUpdate,
        },
    };

    #[derive(Default)]
    struct RecordingEffects {
        order: Vec<&'static str>,
        particles: Vec<NoteBlockParticleState>,
        sounds: Vec<SoundEventState>,
    }

    impl PlayApplyEffects for RecordingEffects {
        fn positioned_sound(&mut self, state: &SoundEventState) {
            self.order.push("sound");
            self.sounds.push(state.clone());
        }

        fn note_block_particle(&mut self, _world: &WorldStore, state: NoteBlockParticleState) {
            self.order.push("particle");
            self.particles.push(state);
        }
    }

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_note_block(world: &mut WorldStore, pos: BlockPos, instrument: &str, note: u8) {
        let properties = BTreeMap::from([
            ("instrument".to_string(), instrument.to_string()),
            ("note".to_string(), note.to_string()),
            ("powered".to_string(), "false".to_string()),
        ]);
        let state_id = world
            .registries
            .block_state_id_by_name_and_properties("minecraft:note_block", &properties)
            .unwrap();
        assert!(world.apply_block_update(ProtocolBlockUpdate {
            pos: protocol_pos(pos),
            block_state_id: state_id,
        }));
    }

    fn protocol_pos(pos: BlockPos) -> ProtocolBlockPos {
        ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        }
    }

    fn event(pos: BlockPos, block_id: i32) -> BlockEvent {
        BlockEvent {
            pos: protocol_pos(pos),
            b0: 239,
            b1: 173,
            block_id,
        }
    }

    fn string_nbt(key: &str, value: &str) -> Vec<u8> {
        let mut nbt = vec![10, 8];
        nbt.extend_from_slice(&(key.len() as u16).to_be_bytes());
        nbt.extend_from_slice(key.as_bytes());
        nbt.extend_from_slice(&(value.len() as u16).to_be_bytes());
        nbt.extend_from_slice(value.as_bytes());
        nbt.push(0);
        nbt
    }

    #[test]
    fn tunable_note_uses_current_state_and_emits_particle_before_sound() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_note_block(&mut world, pos, "harp", 24);
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let expected_seed = expected_random.next_long();
        let mut effects = RecordingEffects::default();

        // Packet holder mismatch is intentional: Level.blockEvent ignores it
        // after decoding and dispatches through the current note block state.
        assert!(world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_STONE_ID),
            &mut random,
            &mut effects,
        ));

        assert_eq!(effects.order, ["particle", "sound"]);
        assert_eq!(
            effects.particles,
            [NoteBlockParticleState {
                position: ProtocolVec3d {
                    x: 3.5,
                    y: 5.2,
                    z: 5.5,
                },
                velocity: ProtocolVec3d {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            }]
        );
        let sound = &effects.sounds[0];
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.note_block.harp")
        );
        assert_eq!(sound.source, "record");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 3.5,
                y: 4.5,
                z: 5.5
            }
        );
        assert_eq!(sound.volume, 3.0);
        assert_eq!(sound.pitch, 2.0);
        assert_eq!(sound.seed, expected_seed);
        assert_eq!(world.counters.block_events_applied, 1);
        assert_eq!(world.counters.block_events_ignored, 0);
    }

    #[test]
    fn mob_head_instrument_has_fixed_pitch_and_no_particle() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        set_note_block(&mut world, pos, "dragon", 7);
        let mut effects = RecordingEffects::default();

        assert!(world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID),
            &mut LevelEventSoundRandomState::with_seed(4),
            &mut effects,
        ));

        assert!(effects.particles.is_empty());
        assert_eq!(effects.order, ["sound"]);
        assert_eq!(effects.sounds[0].pitch, 1.0);
        assert_eq!(
            effects.sounds[0].sound.location.as_deref(),
            Some("minecraft:block.note_block.imitate.ender_dragon")
        );
    }

    #[test]
    fn custom_head_requires_typed_skull_sound_without_consuming_random_on_miss() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 6, y: 6, z: 6 };
        set_note_block(&mut world, pos, "custom_head", 12);
        let mut random = LevelEventSoundRandomState::with_seed(9);
        let mut effects = RecordingEffects::default();
        assert!(!world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID),
            &mut random,
            &mut effects,
        ));
        let mut expected_random = LevelEventSoundRandomState::with_seed(9);
        assert_eq!(random.next_long(), expected_random.next_long());
        assert!(effects.order.is_empty());

        let head_pos = BlockPos {
            y: pos.y + 1,
            ..pos
        };
        let head_properties = BTreeMap::from([
            ("powered".to_string(), "false".to_string()),
            ("rotation".to_string(), "0".to_string()),
        ]);
        let head_state_id = world
            .registries
            .block_state_id_by_name_and_properties("minecraft:player_head", &head_properties)
            .unwrap();
        assert!(world.apply_block_update(ProtocolBlockUpdate {
            pos: protocol_pos(head_pos),
            block_state_id: head_state_id,
        }));
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: protocol_pos(head_pos),
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID,
                raw_nbt: string_nbt("note_block_sound", "bbb:custom/chime"),
            })
            .unwrap());
        assert!(world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID),
            &mut random,
            &mut effects,
        ));
        assert_eq!(
            effects.sounds[0].sound.location.as_deref(),
            Some("bbb:custom/chime")
        );
    }

    #[test]
    fn invalid_holder_is_rejected_before_recording_and_non_triggering_state_is_tracked() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 1, z: 1 };
        assert!(!world.apply_block_event(event(pos, i32::MAX)));
        assert!(world.block_events().is_empty());
        assert_eq!(world.counters.block_events_received, 1);
        assert_eq!(world.counters.block_events_ignored, 1);

        assert!(!world.apply_block_event(event(pos, VANILLA_BLOCK_STONE_ID)));
        assert_eq!(world.block_events().len(), 1);
        assert_eq!(world.counters.block_events_ignored, 2);
        assert_eq!(world.counters.block_events_applied, 0);
    }
}
