use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::vanilla_block_note_instrument,
    block_types::vanilla_block_type_for_resource_id,
    note_block_instruments::{
        vanilla_note_block_instrument_for_name, NoteBlockInstrumentCatalogEntry,
        NoteBlockInstrumentType,
    },
    packets::Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, LevelEventSoundRandomState, PlayApplyEffects, WorldStore};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoteBlockParticleState {
    pub position: ProtocolVec3d,
    pub velocity: ProtocolVec3d,
}

/// The `NoteBlockInstrument` constant an `instrument` property value names
/// (`NoteBlockInstrument.java:9-35`).
///
/// The instrument's three facts — its serialised name, its `SoundEvent` and its
/// `Type` — are one generated catalog,
/// [`bbb_protocol::note_block_instruments`], which the server's own note-block
/// code reads as well. `None` is a name 26.1 has no constant for.
///
/// This side reads only the `Type`: the `SoundEvent` belongs to the server, for
/// the reason [`WorldStore::trigger_note_block_event`] spells out.
fn note_instrument(instrument: &str) -> Option<&'static NoteBlockInstrumentCatalogEntry> {
    vanilla_note_block_instrument_for_name(instrument)
}

/// `NoteBlockInstrument.worksAboveNoteBlock` (`NoteBlockInstrument.java:64-66`).
fn note_instrument_works_above(instrument: &str) -> Option<bool> {
    Some(note_instrument(instrument)?.instrument_type != NoteBlockInstrumentType::BaseBlock)
}

impl WorldStore {
    pub(crate) fn note_block_instrument_for_placement(
        &self,
        pos: BlockPos,
    ) -> Option<&'static str> {
        let above = self.block_view(BlockPos {
            y: pos.y + 1,
            ..pos
        })?;
        let above_instrument = block_note_instrument(&above)?;
        if note_instrument_works_above(above_instrument)? {
            return Some(above_instrument);
        }

        let below = self.block_view(BlockPos {
            y: pos.y - 1,
            ..pos
        })?;
        let below_instrument = block_note_instrument(&below)?;
        if note_instrument_works_above(below_instrument)? {
            Some("harp")
        } else {
            Some(below_instrument)
        }
    }

    /// Mirrors `NoteBlock.triggerEvent` (`NoteBlock.java:148-175`), which
    /// `Level.blockEvent` (`Level.java:902-904`) dispatches at `pos` off the
    /// current block state — so `b0`/`b1` are ignored and the state's own
    /// instrument and note decide everything.
    ///
    /// # Why this side plays no sound
    ///
    /// `triggerEvent` ends in `level.playSeededSound(null, ...)`
    /// (`NoteBlock.java:171-173`), and the `except` entity it passes is `null`.
    /// On a client that call lands in `ClientLevel.playSeededSound`
    /// (`ClientLevel.java:562-576`), whose entire body is
    /// `if (except == this.minecraft.player) { this.playSound(...); }` — and
    /// `null` is never the local player, so nothing is played. The note a
    /// player hears is the `ClientboundSoundPacket` that
    /// `ServerLevel.playSeededSound` (`ServerLevel.java:1023-1045`) broadcast
    /// while running the *server's* copy of `triggerEvent`, which this client
    /// plays through its ordinary sound-packet path. Synthesising one here
    /// would play every note twice.
    ///
    /// The particle is the mirror image of that: the server's
    /// `Level.addParticle` body is empty (`Level.java:492-493`), while
    /// `ClientLevel.addParticle` (`ClientLevel.java:756-758`) really does add
    /// one, so `NoteBlock.java:154` is this side's alone.
    ///
    /// What is left here, then, is the particle, the boolean, and one draw off
    /// the level random.
    pub(crate) fn trigger_note_block_event(
        &mut self,
        pos: BlockPos,
        properties: &BTreeMap<String, String>,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) -> bool {
        let Some(instrument) = properties
            .get("instrument")
            .and_then(|name| note_instrument(name))
        else {
            return false;
        };

        match instrument.instrument_type {
            // `NoteBlockInstrument.isTunable` (`NoteBlockInstrument.java:56-58`),
            // the `if` at `NoteBlock.java:151-154`. The pitch this branch
            // computes (`:153`) is only ever an argument of the muted
            // `playSeededSound`, so the particle is all that survives.
            NoteBlockInstrumentType::BaseBlock => {
                let Some(note) = properties
                    .get("note")
                    .and_then(|note| note.parse::<u8>().ok())
                    .filter(|note| *note <= 24)
                else {
                    return false;
                };
                // `NoteBlock.java:154`.
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
            }
            // The `else` at `NoteBlock.java:155-157` sets a fixed pitch and
            // adds no particle, and that pitch dies with the sound.
            NoteBlockInstrumentType::MobHead => {}
            // `NoteBlockInstrument.hasCustomSound`
            // (`NoteBlockInstrument.java:60-62`): the skull above supplies the
            // sound id (`NoteBlock.java:177-179`), and `triggerEvent` answers
            // false outright when there is none (`:160-164`). That `return`
            // sits *before* the `playSeededSound` call, so a miss must not draw
            // the seed below either.
            NoteBlockInstrumentType::Custom => {
                let above = BlockPos {
                    x: pos.x,
                    y: pos.y.saturating_add(1),
                    z: pos.z,
                };
                if self.skull_note_block_sound_at(above).is_none() {
                    return false;
                }
            }
        }

        // `level.getRandom().nextLong()` (`NoteBlock.java:172`) is an argument
        // of the `playSeededSound` call, so Java evaluates it before the call
        // and the draw happens whether or not `ClientLevel.playSeededSound`
        // goes on to play anything. Keeping it keeps this client's level random
        // in step with vanilla's for every later draw off the same generator.
        random.next_long();
        // `NoteBlock.java:174`.
        true
    }
}

fn block_note_instrument(block: &crate::BlockView) -> Option<&'static str> {
    let block_type = block
        .block_name
        .and_then(vanilla_block_type_for_resource_id)?;
    vanilla_block_note_instrument(block_type.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, SoundEventState, WorldDimension,
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
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })],
            block_entities: std::sync::Arc::default(),
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
    fn tunable_note_uses_current_state_and_emits_only_the_particle() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_note_block(&mut world, pos, "harp", 24);
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let mut effects = RecordingEffects::default();

        // Packet holder mismatch is intentional: Level.blockEvent ignores it
        // after decoding and dispatches through the current note block state
        // (`Level.java:902-904`).
        assert!(world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_STONE_ID),
            &mut random,
            &mut effects,
        ));

        // `NoteBlock.java:154` is the only visible effect a client gets out of
        // this block event: `NoteBlock.java:171-173` hands `null` as `except`,
        // and `ClientLevel.playSeededSound` (`ClientLevel.java:562-576`) plays
        // only when `except == this.minecraft.player`. The note the player
        // hears rides in on the server's own `ClientboundSoundPacket`
        // (`ServerLevel.java:1023-1045`) instead.
        assert_eq!(effects.order, ["particle"]);
        assert!(effects.sounds.is_empty());
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
        // `level.getRandom().nextLong()` (`NoteBlock.java:172`) is evaluated as
        // an argument before the muted call, so the level random still advances
        // by exactly one draw.
        expected_random.next_long();
        assert_eq!(random.next_long(), expected_random.next_long());
        assert_eq!(world.counters.block_events_applied, 1);
        assert_eq!(world.counters.block_events_ignored, 0);
    }

    #[test]
    fn mob_head_instrument_adds_no_particle_and_still_draws_the_seed() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        set_note_block(&mut world, pos, "dragon", 7);
        let mut random = LevelEventSoundRandomState::with_seed(4);
        let mut expected_random = LevelEventSoundRandomState::with_seed(4);
        let mut effects = RecordingEffects::default();

        assert!(world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID),
            &mut random,
            &mut effects,
        ));

        // A `MOB_HEAD` instrument takes the `else` at `NoteBlock.java:155-157`,
        // which only sets the pitch of a sound this side never plays.
        assert!(effects.order.is_empty());
        assert!(effects.particles.is_empty());
        assert!(effects.sounds.is_empty());
        expected_random.next_long();
        assert_eq!(random.next_long(), expected_random.next_long());
    }

    #[test]
    fn custom_head_requires_typed_skull_sound_without_consuming_random_on_miss() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 6, y: 6, z: 6 };
        set_note_block(&mut world, pos, "custom_head", 12);
        let mut random = LevelEventSoundRandomState::with_seed(9);
        let mut expected_random = LevelEventSoundRandomState::with_seed(9);
        let mut effects = RecordingEffects::default();
        assert!(!world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID),
            &mut random,
            &mut effects,
        ));
        // `NoteBlock.java:162-164` returns before the `playSeededSound` call
        // whose argument list holds the `nextLong()` draw, so a miss leaves the
        // level random untouched.
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
        // A head block alone is still a miss: `getCustomSoundId`
        // (`NoteBlock.java:177-179`) needs a `SkullBlockEntity` whose
        // `getNoteBlockSound()` is non-null.
        assert!(!world.apply_block_event_with_effects(
            event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID),
            &mut random,
            &mut effects,
        ));
        assert_eq!(random.next_long(), expected_random.next_long());
        assert!(effects.order.is_empty());

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
        // The skull's sound id decides the boolean and nothing else on this
        // side: `NoteBlock.java:166` only feeds the muted `playSeededSound`.
        assert!(effects.order.is_empty());
        assert!(effects.sounds.is_empty());
        // The hit reaches `:171-173`, so this run does draw the seed.
        expected_random.next_long();
        assert_eq!(random.next_long(), expected_random.next_long());
    }

    /// Every instrument this client can be handed, with the
    /// `worksAboveNoteBlock` answer it produced before the
    /// `NoteBlockInstrument` facts became a generated catalog.
    ///
    /// The expectations are literal on purpose: they are the old hand-written
    /// table's output, so a derived value that drifts from it is a behaviour
    /// change and fails here. Only the `Type`-derived column is here, because
    /// the `Type` is the only catalog field this client still reads — the
    /// instrument's `SoundEvent` (`NoteBlockInstrument.java:9-35`) belongs to
    /// the server, and `bbb-protocol`'s
    /// `every_instrument_sound_is_in_the_generated_sound_event_catalog` already
    /// guards the sound column at its owner.
    const INSTRUMENT_WORKS_ABOVE: &[(&str, bool)] = &[
        ("harp", false),
        ("basedrum", false),
        ("snare", false),
        ("hat", false),
        ("bass", false),
        ("flute", false),
        ("bell", false),
        ("guitar", false),
        ("chime", false),
        ("xylophone", false),
        ("iron_xylophone", false),
        ("cow_bell", false),
        ("didgeridoo", false),
        ("bit", false),
        ("banjo", false),
        ("pling", false),
        ("trumpet", false),
        ("trumpet_exposed", false),
        ("trumpet_oxidized", false),
        ("trumpet_weathered", false),
        ("zombie", true),
        ("skeleton", true),
        ("creeper", true),
        ("dragon", true),
        ("wither_skeleton", true),
        ("piglin", true),
        ("custom_head", true),
    ];

    #[test]
    fn every_instrument_keeps_its_works_above_answer_and_plays_nothing_client_side() {
        assert_eq!(
            INSTRUMENT_WORKS_ABOVE.len(),
            27,
            "one row per 26.1 instrument"
        );
        for &(instrument, works_above) in INSTRUMENT_WORKS_ABOVE {
            assert_eq!(
                note_instrument_works_above(instrument),
                Some(works_above),
                "{instrument}"
            );

            let mut world = world_with_air_chunk();
            let pos = BlockPos { x: 2, y: 3, z: 4 };
            set_note_block(&mut world, pos, instrument, 12);
            let mut effects = RecordingEffects::default();
            let triggered = world.apply_block_event_with_effects(
                event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID),
                &mut LevelEventSoundRandomState::with_seed(0),
                &mut effects,
            );

            // `CUSTOM_HEAD` (`NoteBlockInstrument.java:35`) is the one
            // instrument that answers false with nothing above it
            // (`NoteBlock.java:160-164`); the dedicated test above covers both
            // of its legs in full.
            assert_eq!(triggered, instrument != "custom_head", "{instrument}");
            // No instrument makes this side play anything: `except` is `null`
            // at `NoteBlock.java:172` and `ClientLevel.playSeededSound`
            // (`ClientLevel.java:562-576`) plays only for the local player.
            assert!(effects.sounds.is_empty(), "{instrument}");
            // The particle is the tunable branch's alone (`NoteBlock.java:154`),
            // and `isTunable` is the complement of `worksAboveNoteBlock`
            // because both read the same `Type` (`NoteBlockInstrument.java:56-58`,
            // `:64-66`).
            assert_eq!(
                effects.order,
                if works_above {
                    Vec::<&'static str>::new()
                } else {
                    vec!["particle"]
                },
                "{instrument}"
            );
        }
    }

    #[test]
    fn a_name_that_is_not_an_instrument_has_no_catalog_entry_and_triggers_nothing() {
        assert_eq!(note_instrument_works_above("not_an_instrument"), None);
        assert!(note_instrument("not_an_instrument").is_none());

        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 8, y: 9, z: 10 };
        let mut random = LevelEventSoundRandomState::with_seed(1);
        let mut expected_random = LevelEventSoundRandomState::with_seed(1);
        let mut effects = RecordingEffects::default();
        let properties = BTreeMap::from([
            ("instrument".to_string(), "not_an_instrument".to_string()),
            ("note".to_string(), "3".to_string()),
        ]);
        // `state.getValue(INSTRUMENT)` (`NoteBlock.java:149`) can only ever
        // yield a real constant, so this is the closed side of that lookup:
        // no particle, no draw, no trigger.
        assert!(!world.trigger_note_block_event(pos, &properties, &mut random, &mut effects));
        assert!(effects.order.is_empty());
        assert_eq!(random.next_long(), expected_random.next_long());
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
