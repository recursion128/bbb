use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_protocol::{
    entity_types::*,
    packets::{
        LevelEvent as ProtocolLevelEvent, SoundEntityEvent as ProtocolSoundEntityEvent,
        SoundEvent as ProtocolSoundEvent, SoundEventHolder as ProtocolSoundEventHolder,
        SoundSource, StopSound as ProtocolStopSound, Vec3d as ProtocolVec3d,
    },
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const JUKEBOX_PLAY_LEVEL_EVENT: i32 = 1010;
const JUKEBOX_STOP_LEVEL_EVENT: i32 = 1011;
const BLOCK_BREAK_LEVEL_EVENT: i32 = 2001;
const BRUSH_BLOCK_COMPLETE_LEVEL_EVENT: i32 = 3008;
const SCULK_SHRIEKER_LEVEL_EVENT: i32 = 3007;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
const VAULT_DEACTIVATE_LEVEL_EVENT: i32 = 3016;
const COBWEB_PLACE_LEVEL_EVENT: i32 = 3018;
const TOTEM_USE_SOUND_EVENT: &str = "minecraft:item.totem.use";
// Vanilla 26.1 BlockEntityType registry order in BlockEntityType.java.
const VANILLA_VAULT_BLOCK_ENTITY_TYPE_ID: i32 = 45;
const GLOBAL_LEVEL_EVENT_SOUND_DISTANCE: f64 = 2.0;
const VANILLA_VEC3_NORMALIZE_EPSILON: f64 = 1.0e-5;
const SCULK_SHRIEKER_TOP_Y: f64 = 0.5;
const RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const RANDOM_INCREMENT: u64 = 11;
const RANDOM_MASK: u64 = (1_u64 << 48) - 1;
const RANDOM_FLOAT_MULTIPLIER: f32 = 5.960_464_5e-8;
const RANDOM_DOUBLE_DIVISOR: f64 = (1_u64 << 53) as f64;
const SEED_UNIQUIFIER_INITIAL: u64 = 8_682_522_807_148_012;
const SEED_UNIQUIFIER_MULTIPLIER: u64 = 1_181_783_497_276_652_981;

static SEED_UNIQUIFIER: AtomicU64 = AtomicU64::new(SEED_UNIQUIFIER_INITIAL);

#[derive(Debug, Clone)]
pub struct LevelEventSoundRandomState {
    seed: u64,
    next_gaussian: Option<f64>,
}

impl LevelEventSoundRandomState {
    pub fn with_seed(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
            next_gaussian: None,
        }
    }

    pub fn next_float(&mut self) -> f32 {
        (self.next_bits(24) as f32) * RANDOM_FLOAT_MULTIPLIER
    }

    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return ((i64::from(bound) * i64::from(self.next_bits(31))) >> 31) as i32;
        }

        loop {
            let sample = self.next_bits(31) as i32;
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo;
            }
        }
    }

    pub fn next_double(&mut self) -> f64 {
        let high = (self.next_bits(26) as u64) << 27;
        let low = self.next_bits(27) as u64;
        (high + low) as f64 / RANDOM_DOUBLE_DIVISOR
    }

    pub fn next_long(&mut self) -> i64 {
        let high = (self.next_bits(32) as i32 as i64) << 32;
        let low = self.next_bits(32) as i32 as i64;
        high.wrapping_add(low)
    }

    pub fn next_gaussian(&mut self) -> f64 {
        if let Some(value) = self.next_gaussian.take() {
            return value;
        }

        loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let multiplier = (-2.0 * s.ln() / s).sqrt();
                self.next_gaussian = Some(v2 * multiplier);
                return v1 * multiplier;
            }
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(RANDOM_MULTIPLIER)
            .wrapping_add(RANDOM_INCREMENT)
            & RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

impl Default for LevelEventSoundRandomState {
    fn default() -> Self {
        Self::with_seed(generate_unique_seed())
    }
}

fn generate_unique_seed() -> i64 {
    let unique = SEED_UNIQUIFIER
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            Some(current.wrapping_mul(SEED_UNIQUIFIER_MULTIPLIER))
        })
        .map(|previous| previous.wrapping_mul(SEED_UNIQUIFIER_MULTIPLIER))
        .unwrap_or_else(|current| current.wrapping_mul(SEED_UNIQUIFIER_MULTIPLIER));
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    (unique ^ nanos) as i64
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientAudioState {
    #[serde(default)]
    pub last_sound: Option<SoundEventState>,
    #[serde(default)]
    pub last_local_sound: Option<LocalSoundEventState>,
    #[serde(default)]
    pub last_sound_entity: Option<SoundEntityEventState>,
    #[serde(default)]
    pub last_stop_sound: Option<StopSoundEventState>,
    #[serde(default)]
    pub playing_jukebox_songs: Vec<JukeboxSongState>,
    #[serde(default)]
    pub last_jukebox_event: Option<JukeboxLevelEventState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEventState {
    pub sound: SoundHolderState,
    pub source: String,
    pub position: ProtocolVec3d,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
    #[serde(default)]
    pub distance_delay: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalSoundEventState {
    pub sound: SoundHolderState,
    pub source: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct JukeboxSongState {
    pub pos: crate::BlockPos,
    pub song_registry_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct JukeboxLevelEventState {
    pub action: JukeboxLevelEventAction,
    pub pos: crate::BlockPos,
    pub song_registry_id: Option<i32>,
    pub stopped_existing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JukeboxLevelEventAction {
    Start,
    Stop,
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
        self.items.default_block_sound_profiles = profiles
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
            distance_delay: false,
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

    pub fn totem_use_sound_for_entity(&mut self, entity_id: i32) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(TOTEM_USE_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: ProtocolVec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            volume: 1.0,
            pitch: 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn client_audio(&self) -> &ClientAudioState {
        &self.client_audio
    }

    pub fn last_sound(&self) -> Option<&SoundEventState> {
        self.client_audio.last_sound.as_ref()
    }

    pub fn last_local_sound(&self) -> Option<&LocalSoundEventState> {
        self.client_audio.last_local_sound.as_ref()
    }

    pub fn last_sound_entity(&self) -> Option<&SoundEntityEventState> {
        self.client_audio.last_sound_entity.as_ref()
    }

    pub fn last_stop_sound(&self) -> Option<&StopSoundEventState> {
        self.client_audio.last_stop_sound.as_ref()
    }

    pub fn playing_jukebox_songs(&self) -> &[JukeboxSongState] {
        &self.client_audio.playing_jukebox_songs
    }

    pub fn last_jukebox_event(&self) -> Option<&JukeboxLevelEventState> {
        self.client_audio.last_jukebox_event.as_ref()
    }

    pub(crate) fn record_jukebox_level_event(
        &mut self,
        event: ProtocolLevelEvent,
    ) -> Option<JukeboxLevelEventState> {
        let pos = crate::protocol_block_pos(event.pos);
        let state = match event.event_type {
            JUKEBOX_PLAY_LEVEL_EVENT if event.data >= 0 => {
                let stopped_existing =
                    remove_jukebox_song(&mut self.client_audio.playing_jukebox_songs, pos);
                self.client_audio
                    .playing_jukebox_songs
                    .push(JukeboxSongState {
                        pos,
                        song_registry_id: event.data,
                    });
                sort_jukebox_songs(&mut self.client_audio.playing_jukebox_songs);
                JukeboxLevelEventState {
                    action: JukeboxLevelEventAction::Start,
                    pos,
                    song_registry_id: Some(event.data),
                    stopped_existing,
                }
            }
            JUKEBOX_STOP_LEVEL_EVENT => {
                let stopped_existing =
                    remove_jukebox_song(&mut self.client_audio.playing_jukebox_songs, pos);
                JukeboxLevelEventState {
                    action: JukeboxLevelEventAction::Stop,
                    pos,
                    song_registry_id: None,
                    stopped_existing,
                }
            }
            _ => return None,
        };
        self.client_audio.last_jukebox_event = Some(state);
        Some(state)
    }

    pub fn local_block_hit_sound(&self, pos: crate::BlockPos) -> Option<SoundEventState> {
        let block = self.probe_block(pos)?;
        let block_name = block.block_name.as_deref()?;
        let profile = self.items.default_block_sound_profiles.get(block_name)?;
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
        if let Some(state) = self.level_event_brush_complete_sound_state(event) {
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
        if let Some(sound) =
            random_distance_delayed_level_event_sound(event.event_type, event.data, &mut next_float)
        {
            return Some(block_sound_state_with_distance_delay(
                crate::protocol_block_pos(event.pos),
                sound.event_id,
                sound.volume,
                sound.pitch,
                sound.source,
                true,
            ));
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

    pub fn cobweb_place_level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if event.event_type != COBWEB_PLACE_LEVEL_EVENT {
            return None;
        }
        Some(block_sound_state_with_distance_delay(
            crate::protocol_block_pos(event.pos),
            "minecraft:block.cobweb.place",
            1.0,
            triangle_pitch(1.0, 0.2, &mut next_float),
            "block",
            true,
        ))
    }

    pub fn sculk_shrieker_level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if event.event_type != SCULK_SHRIEKER_LEVEL_EVENT {
            return None;
        }
        let pos = crate::protocol_block_pos(event.pos);
        if self
            .probe_block(pos)
            .and_then(|probe| probe.block_properties.get("waterlogged").cloned())
            .is_some_and(|waterlogged| waterlogged == "true")
        {
            return None;
        }
        Some(SoundEventState {
            sound: direct_sound_holder("minecraft:block.sculk_shrieker.shriek"),
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + SCULK_SHRIEKER_TOP_Y,
                z: f64::from(pos.z) + 0.5,
            },
            volume: 2.0,
            pitch: ranged_pitch(0.6, 0.4, &mut next_float),
            seed: 0,
            distance_delay: false,
        })
    }

    pub fn vault_level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let sound = match event.event_type {
            VAULT_ACTIVATE_LEVEL_EVENT
                if self.block_entity_type_id_at(crate::protocol_block_pos(event.pos))
                    == Some(VANILLA_VAULT_BLOCK_ENTITY_TYPE_ID) =>
            {
                "minecraft:block.vault.activate"
            }
            VAULT_DEACTIVATE_LEVEL_EVENT => "minecraft:block.vault.deactivate",
            _ => return None,
        };
        Some(block_sound_state_with_distance_delay(
            crate::protocol_block_pos(event.pos),
            sound,
            1.0,
            triangle_pitch(1.0, 0.2, &mut next_float),
            "block",
            true,
        ))
    }

    pub fn global_level_event_sound(
        &self,
        event: ProtocolLevelEvent,
        camera_position: ProtocolVec3d,
    ) -> Option<SoundEventState> {
        if !event.global {
            return None;
        }
        let sound = global_level_event_sound(event.event_type)?;
        Some(SoundEventState {
            sound: direct_sound_holder(sound.event_id),
            source: sound.source.to_string(),
            position: global_level_event_sound_position(
                crate::protocol_block_pos(event.pos),
                camera_position,
            ),
            volume: sound.volume,
            pitch: sound.pitch,
            seed: 0,
            distance_delay: false,
        })
    }

    pub fn level_event_local_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<LocalSoundEventState> {
        let sound = local_level_event_sound(event.event_type, &mut next_float)?;
        Some(LocalSoundEventState {
            sound: direct_sound_holder(sound.event_id),
            source: sound.source.to_string(),
            volume: sound.volume,
            pitch: sound.pitch,
            seed: 0,
        })
    }

    pub fn record_local_sound(&mut self, state: LocalSoundEventState) -> LocalSoundEventState {
        self.client_audio.last_local_sound = Some(state.clone());
        state
    }

    pub fn record_positioned_sound(&mut self, state: SoundEventState) -> SoundEventState {
        self.client_audio.last_sound = Some(state.clone());
        state
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
        let profile = self
            .items
            .default_block_sound_profiles
            .get(&block_state.name)?;
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

    fn level_event_brush_complete_sound_state(
        &self,
        event: ProtocolLevelEvent,
    ) -> Option<SoundEventState> {
        if event.event_type != BRUSH_BLOCK_COMPLETE_LEVEL_EVENT {
            return None;
        }
        let block_state = self.registries.block_state(event.data)?;
        let sound = match block_state.name.as_str() {
            // Vanilla 26.1 Blocks.SUSPICIOUS_* constructs BrushableBlock with the
            // same brush sound for regular and completed brush events.
            "minecraft:suspicious_sand" => "minecraft:item.brush.brushing.sand",
            "minecraft:suspicious_gravel" => "minecraft:item.brush.brushing.gravel",
            _ => return None,
        };
        Some(block_sound_state(
            crate::protocol_block_pos(event.pos),
            sound,
            1.0,
            1.0,
            "player",
        ))
    }
}

fn remove_jukebox_song(songs: &mut Vec<JukeboxSongState>, pos: crate::BlockPos) -> bool {
    let before = songs.len();
    songs.retain(|song| song.pos != pos);
    songs.len() != before
}

fn sort_jukebox_songs(songs: &mut [JukeboxSongState]) {
    songs.sort_by_key(|song| (song.pos.x, song.pos.y, song.pos.z));
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
        3003 => FixedLevelEventSound {
            event_id: "minecraft:item.honeycomb.wax_on",
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
        2002 | 2007 => FixedLevelEventSound {
            event_id: "minecraft:entity.splash_potion.break",
            source: "neutral",
            volume: 1.0,
            pitch: ranged_pitch(0.9, 0.1, next_float),
        },
        2006 if data == 1 => FixedLevelEventSound {
            event_id: "minecraft:entity.dragon_fireball.explode",
            source: "hostile",
            volume: 1.0,
            pitch: ranged_pitch(0.9, 0.1, next_float),
        },
        3000 => FixedLevelEventSound {
            event_id: "minecraft:block.end_gateway.spawn",
            source: "block",
            volume: 10.0,
            pitch: triangle_pitch(1.0, 0.2, next_float) * 0.7,
        },
        3001 => FixedLevelEventSound {
            event_id: "minecraft:entity.ender_dragon.growl",
            source: "hostile",
            volume: 64.0,
            pitch: ranged_pitch(0.8, 0.3, next_float),
        },
        3006 => sculk_charge_sound(data, next_float)?,
        _ => return None,
    };
    Some(sound)
}

fn random_distance_delayed_level_event_sound(
    event_type: i32,
    data: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        3012 => trial_spawner_triangle("minecraft:block.trial_spawner.spawn_mob", 1.0, next_float),
        3013 | 3019 => trial_spawner_triangle(
            "minecraft:block.trial_spawner.detect_player",
            1.0,
            next_float,
        ),
        3014 => trial_spawner_triangle("minecraft:block.trial_spawner.eject_item", 1.0, next_float),
        3020 => trial_spawner_triangle(
            "minecraft:block.trial_spawner.ominous_activate",
            if data == 0 { 0.3 } else { 1.0 },
            next_float,
        ),
        3021 => trial_spawner_triangle("minecraft:block.trial_spawner.spawn_item", 1.0, next_float),
        _ => return None,
    };
    Some(sound)
}

fn local_level_event_sound(
    event_type: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        1032 => FixedLevelEventSound {
            event_id: "minecraft:block.portal.travel",
            source: "ambient",
            volume: 0.25,
            pitch: ranged_pitch(0.8, 0.4, next_float),
        },
        _ => return None,
    };
    Some(sound)
}

fn global_level_event_sound(event_type: i32) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        1023 => FixedLevelEventSound {
            event_id: "minecraft:entity.wither.spawn",
            source: "hostile",
            volume: 1.0,
            pitch: 1.0,
        },
        1028 => FixedLevelEventSound {
            event_id: "minecraft:entity.ender_dragon.death",
            source: "hostile",
            volume: 5.0,
            pitch: 1.0,
        },
        1038 => FixedLevelEventSound {
            event_id: "minecraft:block.end_portal.spawn",
            source: "hostile",
            volume: 1.0,
            pitch: 1.0,
        },
        _ => return None,
    };
    Some(sound)
}

fn global_level_event_sound_position(
    pos: crate::BlockPos,
    camera_position: ProtocolVec3d,
) -> ProtocolVec3d {
    let event_center = ProtocolVec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    };
    let delta = ProtocolVec3d {
        x: event_center.x - camera_position.x,
        y: event_center.y - camera_position.y,
        z: event_center.z - camera_position.z,
    };
    let distance = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
    if distance < VANILLA_VEC3_NORMALIZE_EPSILON {
        return camera_position;
    }

    ProtocolVec3d {
        x: camera_position.x + delta.x / distance * GLOBAL_LEVEL_EVENT_SOUND_DISTANCE,
        y: camera_position.y + delta.y / distance * GLOBAL_LEVEL_EVENT_SOUND_DISTANCE,
        z: camera_position.z + delta.z / distance * GLOBAL_LEVEL_EVENT_SOUND_DISTANCE,
    }
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

fn trial_spawner_triangle(
    event_id: &'static str,
    volume: f32,
    next_float: &mut impl FnMut() -> f32,
) -> FixedLevelEventSound {
    FixedLevelEventSound {
        event_id,
        source: "block",
        volume,
        pitch: triangle_pitch(1.0, 0.2, next_float),
    }
}

fn sculk_charge_sound(
    data: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let count = data >> 6;
    if count <= 0 {
        return Some(FixedLevelEventSound {
            event_id: "minecraft:block.sculk.charge",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        });
    }

    let count = count as f32;
    if next_float().clamp(0.0, 1.0) >= 0.3 + count * 0.1 {
        return None;
    }

    Some(FixedLevelEventSound {
        event_id: "minecraft:block.sculk.charge",
        source: "block",
        volume: 0.15 + 0.02 * count * count * next_float().clamp(0.0, 1.0),
        pitch: 0.4 + 0.3 * count * next_float().clamp(0.0, 1.0),
    })
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
    block_sound_state_with_distance_delay(pos, sound, volume, pitch, source, false)
}

fn block_sound_state_with_distance_delay(
    pos: crate::BlockPos,
    sound: &str,
    volume: f32,
    pitch: f32,
    source: &str,
    distance_delay: bool,
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
        distance_delay,
    }
}

fn vanilla_entity_sound_source(entity_type_id: i32) -> SoundSource {
    if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
        return SoundSource::Players;
    }
    if is_vanilla_hostile_sound_source_type(entity_type_id) {
        return SoundSource::Hostile;
    }
    match entity_type_id {
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID | VANILLA_ENTITY_TYPE_ITEM_ID => SoundSource::Ambient,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => SoundSource::Weather,
        _ => SoundSource::Neutral,
    }
}

fn is_vanilla_hostile_sound_source_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BLAZE_ID
            | VANILLA_ENTITY_TYPE_BOGGED_ID
            | VANILLA_ENTITY_TYPE_BREEZE_ID
            | VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID
            | VANILLA_ENTITY_TYPE_CREAKING_ID
            | VANILLA_ENTITY_TYPE_CREEPER_ID
            | VANILLA_ENTITY_TYPE_DROWNED_ID
            | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
            | VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
            | VANILLA_ENTITY_TYPE_ENDERMAN_ID
            | VANILLA_ENTITY_TYPE_ENDERMITE_ID
            | VANILLA_ENTITY_TYPE_EVOKER_ID
            | VANILLA_ENTITY_TYPE_GHAST_ID
            | VANILLA_ENTITY_TYPE_GIANT_ID
            | VANILLA_ENTITY_TYPE_GUARDIAN_ID
            | VANILLA_ENTITY_TYPE_HOGLIN_ID
            | VANILLA_ENTITY_TYPE_HUSK_ID
            | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
            | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
            | VANILLA_ENTITY_TYPE_PHANTOM_ID
            | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            | VANILLA_ENTITY_TYPE_PIGLIN_ID
            | VANILLA_ENTITY_TYPE_PILLAGER_ID
            | VANILLA_ENTITY_TYPE_RAVAGER_ID
            | VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID
            | VANILLA_ENTITY_TYPE_SHULKER_ID
            | VANILLA_ENTITY_TYPE_SILVERFISH_ID
            | VANILLA_ENTITY_TYPE_SKELETON_ID
            | VANILLA_ENTITY_TYPE_SLIME_ID
            | VANILLA_ENTITY_TYPE_SPIDER_ID
            | VANILLA_ENTITY_TYPE_STRAY_ID
            | VANILLA_ENTITY_TYPE_VEX_ID
            | VANILLA_ENTITY_TYPE_VINDICATOR_ID
            | VANILLA_ENTITY_TYPE_WARDEN_ID
            | VANILLA_ENTITY_TYPE_WITCH_ID
            | VANILLA_ENTITY_TYPE_WITHER_ID
            | VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID
            | VANILLA_ENTITY_TYPE_ZOGLIN_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
    )
}

pub fn advance_cobweb_place_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..10 {
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

pub fn advance_vault_activation_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

pub fn advance_vault_deactivation_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

fn direct_sound_holder(sound: &str) -> SoundHolderState {
    SoundHolderState {
        kind: "direct".to_string(),
        registry_id: None,
        location: Some(sound.to_string()),
        fixed_range: None,
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
    fn level_event_random_state_matches_java_float_and_double_samples() {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        assert_close(random.next_float(), 0.730_967_76);
        assert_close(random.next_float(), 0.831_441);
        assert_close_f64(random.next_double(), 0.240_536_415_671_485_87);
        assert_close_f64(random.next_double(), 0.637_417_425_350_108_3);

        let mut random = LevelEventSoundRandomState::with_seed(0);
        assert_eq!(random.next_long(), -4_962_768_465_676_381_896);
        assert_eq!(random.next_long(), 4_437_113_781_045_784_766);

        let mut random = LevelEventSoundRandomState::with_seed(0);
        assert_eq!(random.next_int_bound(3), 0);
        assert_eq!(random.next_int_bound(3), 1);
        assert_eq!(random.next_int_bound(4), 0);
        assert_eq!(random.next_int_bound(5), 2);

        let mut random = LevelEventSoundRandomState::with_seed(0);
        assert_close_f64(random.next_gaussian(), 0.802_533_063_739_030_5);
        assert_close_f64(random.next_gaussian(), -0.901_546_088_417_512_2);
        assert_close_f64(random.next_gaussian(), 2.080_920_790_428_163);
    }

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
            distance_delay: false,
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
    fn level_event_3008_maps_brushable_block_state_to_completion_sound() {
        let store = WorldStore::new();
        let suspicious_sand =
            vanilla_block_state_id("minecraft:suspicious_sand", [("dusted", "0")]);
        let suspicious_gravel =
            vanilla_block_state_id("minecraft:suspicious_gravel", [("dusted", "3")]);
        let stone = vanilla_block_state_id("minecraft:stone", []);

        let sand_sound = store
            .level_event_sound(LevelEvent {
                event_type: 3008,
                pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
                data: suspicious_sand,
                global: false,
            })
            .unwrap();
        assert_eq!(
            sand_sound.sound.location.as_deref(),
            Some("minecraft:item.brush.brushing.sand")
        );
        assert_eq!(sand_sound.source, "player");
        assert_eq!(sand_sound.position, vec3(2.5, 3.5, -3.5));
        assert_close(sand_sound.volume, 1.0);
        assert_close(sand_sound.pitch, 1.0);

        let gravel_sound = store
            .level_event_sound(LevelEvent {
                event_type: 3008,
                pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                data: suspicious_gravel,
                global: false,
            })
            .unwrap();
        assert_eq!(
            gravel_sound.sound.location.as_deref(),
            Some("minecraft:item.brush.brushing.gravel")
        );
        assert_eq!(gravel_sound.source, "player");
        assert_eq!(gravel_sound.position, vec3(-0.5, 70.5, 4.5));

        assert!(store
            .level_event_sound(LevelEvent {
                event_type: 3008,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                data: stone,
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
            (3003, "minecraft:item.honeycomb.wax_on", "block", 1.0, 1.0),
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
    fn global_level_event_sound_maps_vanilla_camera_relative_audio() {
        let store = WorldStore::new();
        let camera_position = vec3(0.5, 64.5, 0.5);

        for (event_type, event_id, volume) in [
            (1023, "minecraft:entity.wither.spawn", 1.0),
            (1028, "minecraft:entity.ender_dragon.death", 5.0),
            (1038, "minecraft:block.end_portal.spawn", 1.0),
        ] {
            let sound = store
                .global_level_event_sound(
                    LevelEvent {
                        event_type,
                        pos: ProtocolBlockPos { x: 0, y: 64, z: 10 },
                        data: 0,
                        global: true,
                    },
                    camera_position,
                )
                .unwrap();
            assert_eq!(sound.sound.location.as_deref(), Some(event_id));
            assert_eq!(sound.source, "hostile");
            assert_eq!(sound.position, vec3(0.5, 64.5, 2.5));
            assert_close(sound.volume, volume);
            assert_close(sound.pitch, 1.0);
            assert_eq!(sound.seed, 0);
        }

        assert!(store
            .global_level_event_sound(
                LevelEvent {
                    event_type: 1023,
                    pos: ProtocolBlockPos { x: 0, y: 64, z: 10 },
                    data: 0,
                    global: false,
                },
                camera_position,
            )
            .is_none());
        assert!(store
            .global_level_event_sound(
                LevelEvent {
                    event_type: 1001,
                    pos: ProtocolBlockPos { x: 0, y: 64, z: 10 },
                    data: 0,
                    global: true,
                },
                camera_position,
            )
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

        let splash_potion_break = random_level_event_sound(&store, 2002, 0x3366cc, &[0.5]);
        assert_eq!(
            splash_potion_break.sound.location.as_deref(),
            Some("minecraft:entity.splash_potion.break")
        );
        assert_eq!(splash_potion_break.source, "neutral");
        assert_close(splash_potion_break.volume, 1.0);
        assert_close(splash_potion_break.pitch, 0.95);

        let instant_effect_potion_break = random_level_event_sound(&store, 2007, 0x3366cc, &[0.25]);
        assert_eq!(
            instant_effect_potion_break.sound.location.as_deref(),
            Some("minecraft:entity.splash_potion.break")
        );
        assert_eq!(instant_effect_potion_break.source, "neutral");
        assert_close(instant_effect_potion_break.volume, 1.0);
        assert_close(instant_effect_potion_break.pitch, 0.925);

        let dragon_fireball = random_level_event_sound(&store, 2006, 1, &[0.75]);
        assert_eq!(
            dragon_fireball.sound.location.as_deref(),
            Some("minecraft:entity.dragon_fireball.explode")
        );
        assert_eq!(dragon_fireball.source, "hostile");
        assert_close(dragon_fireball.volume, 1.0);
        assert_close(dragon_fireball.pitch, 0.975);

        assert!(store
            .level_event_sound_with_random(
                LevelEvent {
                    event_type: 2006,
                    pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                    data: 0,
                    global: false,
                },
                || panic!("2006 without data=1 has no sound")
            )
            .is_none());

        for (event_type, data, event_id, volume, expected_pitch) in [
            (3012, 0, "minecraft:block.trial_spawner.spawn_mob", 1.0, 1.1),
            (
                3013,
                0,
                "minecraft:block.trial_spawner.detect_player",
                1.0,
                1.1,
            ),
            (
                3014,
                0,
                "minecraft:block.trial_spawner.eject_item",
                1.0,
                1.1,
            ),
            (
                3019,
                0,
                "minecraft:block.trial_spawner.detect_player",
                1.0,
                1.1,
            ),
            (
                3020,
                0,
                "minecraft:block.trial_spawner.ominous_activate",
                0.3,
                1.1,
            ),
            (
                3020,
                1,
                "minecraft:block.trial_spawner.ominous_activate",
                1.0,
                1.1,
            ),
            (
                3021,
                0,
                "minecraft:block.trial_spawner.spawn_item",
                1.0,
                1.1,
            ),
        ] {
            let sound = random_level_event_sound(&store, event_type, data, &[0.75, 0.25]);
            assert_eq!(sound.sound.location.as_deref(), Some(event_id));
            assert_eq!(sound.source, "block");
            assert_close(sound.volume, volume);
            assert_close(sound.pitch, expected_pitch);
            assert!(sound.distance_delay);
        }

        let end_gateway_spawn = random_level_event_sound(&store, 3000, 0, &[0.75, 0.25]);
        assert_eq!(
            end_gateway_spawn.sound.location.as_deref(),
            Some("minecraft:block.end_gateway.spawn")
        );
        assert_eq!(end_gateway_spawn.source, "block");
        assert_close(end_gateway_spawn.volume, 10.0);
        assert_close(end_gateway_spawn.pitch, 0.77);

        let dragon_growl = random_level_event_sound(&store, 3001, 0, &[0.5]);
        assert_eq!(
            dragon_growl.sound.location.as_deref(),
            Some("minecraft:entity.ender_dragon.growl")
        );
        assert_eq!(dragon_growl.source, "hostile");
        assert_close(dragon_growl.volume, 64.0);
        assert_close(dragon_growl.pitch, 0.95);

        let sculk_charge_pop = random_level_event_sound(&store, 3006, 0, &[]);
        assert_eq!(
            sculk_charge_pop.sound.location.as_deref(),
            Some("minecraft:block.sculk.charge")
        );
        assert_eq!(sculk_charge_pop.source, "block");
        assert_close(sculk_charge_pop.volume, 1.0);
        assert_close(sculk_charge_pop.pitch, 1.0);

        let sculk_charge = random_level_event_sound(&store, 3006, 2 << 6, &[0.25, 0.5, 0.25]);
        assert_eq!(
            sculk_charge.sound.location.as_deref(),
            Some("minecraft:block.sculk.charge")
        );
        assert_eq!(sculk_charge.source, "block");
        assert_close(sculk_charge.volume, 0.19);
        assert_close(sculk_charge.pitch, 0.55);

        let mut missed_samples = [0.5].into_iter();
        assert!(store
            .level_event_sound_with_random(
                LevelEvent {
                    event_type: 3006,
                    pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                    data: 1 << 6,
                    global: false,
                },
                || missed_samples.next().unwrap(),
            )
            .is_none());

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

    #[test]
    fn cobweb_place_level_event_sound_uses_post_particle_random_pitch_and_distance_delay() {
        let store = WorldStore::new();
        let event = LevelEvent {
            event_type: 3018,
            pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
            data: 0,
            global: false,
        };
        let mut random = LevelEventSoundRandomState::with_seed(0);
        advance_cobweb_place_particle_randoms(&mut random);

        let sound = store
            .cobweb_place_level_event_sound_with_random(event, || random.next_float())
            .unwrap();

        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.cobweb.place")
        );
        assert_eq!(sound.source, "block");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 2.5,
                y: 64.5,
                z: -4.5,
            }
        );
        assert_close(sound.volume, 1.0);
        assert_close(sound.pitch, 1.013_698_2);
        assert_eq!(sound.seed, 0);
        assert!(sound.distance_delay);

        assert!(store
            .level_event_sound_with_random(event, || panic!("3018 sound requires particle order"))
            .is_none());
    }

    #[test]
    fn level_event_local_sound_with_random_maps_portal_travel_ambience() {
        let mut store = WorldStore::new();
        let sound = store
            .level_event_local_sound_with_random(
                LevelEvent {
                    event_type: 1032,
                    pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                    data: 0,
                    global: false,
                },
                || 0.5,
            )
            .unwrap();

        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.portal.travel")
        );
        assert_eq!(sound.source, "ambient");
        assert_close(sound.volume, 0.25);
        assert_close(sound.pitch, 1.0);
        assert_eq!(sound.seed, 0);
        assert_eq!(store.last_local_sound(), None);

        let recorded = store.record_local_sound(sound);
        assert_eq!(store.last_local_sound(), Some(&recorded));
    }

    #[test]
    fn sculk_shrieker_level_event_sound_uses_top_y_and_random_pitch() {
        let store = WorldStore::new();
        let sound = store
            .sculk_shrieker_level_event_sound_with_random(
                LevelEvent {
                    event_type: 3007,
                    pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
                    data: 0,
                    global: false,
                },
                || 0.25,
            )
            .unwrap();

        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.sculk_shrieker.shriek")
        );
        assert_eq!(sound.source, "block");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 2.5,
                y: 64.5,
                z: -4.5,
            }
        );
        assert_close(sound.volume, 2.0);
        assert_close(sound.pitch, 0.7);
        assert_eq!(sound.seed, 0);
        assert!(!sound.distance_delay);

        assert!(store
            .sculk_shrieker_level_event_sound_with_random(
                LevelEvent {
                    event_type: 3008,
                    pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
                    data: 0,
                    global: false,
                },
                || panic!("non-3007 should not sample random"),
            )
            .is_none());
    }

    #[test]
    fn vault_level_event_sound_uses_distance_delay_and_block_entity_gate() {
        let store = WorldStore::new();
        let deactivate = store
            .vault_level_event_sound_with_random(
                LevelEvent {
                    event_type: 3016,
                    pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                    data: 0,
                    global: false,
                },
                {
                    let mut samples = [0.75, 0.25].into_iter();
                    move || samples.next().unwrap()
                },
            )
            .unwrap();
        assert_eq!(
            deactivate.sound.location.as_deref(),
            Some("minecraft:block.vault.deactivate")
        );
        assert_eq!(deactivate.source, "block");
        assert_eq!(deactivate.position, vec3(-0.5, 70.5, 4.5));
        assert_close(deactivate.volume, 1.0);
        assert_close(deactivate.pitch, 1.1);
        assert!(deactivate.distance_delay);

        assert!(store
            .vault_level_event_sound_with_random(
                LevelEvent {
                    event_type: 3015,
                    pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                    data: 0,
                    global: false,
                },
                || panic!("3015 without a vault block entity has no sound")
            )
            .is_none());
    }

    #[test]
    fn record_positioned_sound_updates_last_sound_without_counting_sound_packet() {
        let mut store = WorldStore::new();

        let sound = SoundEventState {
            sound: SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some("minecraft:block.dispenser.fail".to_string()),
                fixed_range: None,
            },
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: 1.5,
                y: 2.5,
                z: -3.5,
            },
            volume: 1.0,
            pitch: 1.2,
            seed: 0,
            distance_delay: false,
        };

        let recorded = store.record_positioned_sound(sound);

        assert_eq!(store.last_sound(), Some(&recorded));
        assert_eq!(store.counters().sound_packets, 0);
    }

    #[test]
    fn level_event_1010_and_1011_track_jukebox_song_state() {
        let mut store = WorldStore::new();
        let pos = ProtocolBlockPos { x: 4, y: 65, z: -7 };

        store.apply_level_event(LevelEvent {
            event_type: 1010,
            pos,
            data: 12,
            global: false,
        });

        assert_eq!(
            store.playing_jukebox_songs(),
            &[JukeboxSongState {
                pos: crate::BlockPos { x: 4, y: 65, z: -7 },
                song_registry_id: 12,
            }]
        );
        assert_eq!(
            store.last_jukebox_event(),
            Some(&JukeboxLevelEventState {
                action: JukeboxLevelEventAction::Start,
                pos: crate::BlockPos { x: 4, y: 65, z: -7 },
                song_registry_id: Some(12),
                stopped_existing: false,
            })
        );

        store.apply_level_event(LevelEvent {
            event_type: 1010,
            pos,
            data: 15,
            global: false,
        });

        assert_eq!(
            store.playing_jukebox_songs(),
            &[JukeboxSongState {
                pos: crate::BlockPos { x: 4, y: 65, z: -7 },
                song_registry_id: 15,
            }]
        );
        assert_eq!(
            store.last_jukebox_event().unwrap().action,
            JukeboxLevelEventAction::Start
        );
        assert!(store.last_jukebox_event().unwrap().stopped_existing);

        store.apply_level_event(LevelEvent {
            event_type: 1011,
            pos,
            data: 0,
            global: false,
        });

        assert!(store.playing_jukebox_songs().is_empty());
        assert_eq!(
            store.last_jukebox_event(),
            Some(&JukeboxLevelEventState {
                action: JukeboxLevelEventAction::Stop,
                pos: crate::BlockPos { x: 4, y: 65, z: -7 },
                song_registry_id: None,
                stopped_existing: true,
            })
        );
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

    fn assert_close_f64(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
        let properties = props
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        crate::registries::BlockStateRegistry::vanilla_26_1()
            .find_by_name_and_properties(name, &properties)
            .unwrap()
            .id
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
