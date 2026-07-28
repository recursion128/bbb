use std::collections::{BTreeMap, VecDeque};

use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
    data_component_types::VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
    entity_types::*,
    game_event_types::{
        VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
        VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID,
        VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID,
    },
    item_types::{vanilla_item_default_break_sound_resource_id, VANILLA_ITEM_WOLF_ARMOR_ID},
    packets::{
        EntityDataRegistryHolder, EquipmentSlot, GameEvent as ProtocolGameEvent,
        LevelEvent as ProtocolLevelEvent, SoundEntityEvent as ProtocolSoundEntityEvent,
        SoundEvent as ProtocolSoundEvent, SoundEventHolder as ProtocolSoundEventHolder,
        SoundEventSummary, SoundSource, StopSound as ProtocolStopSound, Vec3d as ProtocolVec3d,
    },
};
use serde::{Deserialize, Serialize};

use crate::client::inventory::LocalInteractionSoundAction;
use crate::entities::{
    is_vanilla_minecart_type, vanilla_entity_metadata_armadillo_state_accessor_for_entity_type,
    vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type,
    vanilla_entity_metadata_eating_sound_policy,
    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
    vanilla_entity_metadata_goat_accessors_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
    vanilla_entity_metadata_type_death_sound_policy,
    vanilla_entity_metadata_type_hurt_sound_policy, vanilla_entity_metadata_type_sound_source_kind,
    vanilla_entity_metadata_type_sound_volume_policy, CatSoundVariantIdentity,
    ChickenSoundVariantIdentity, CowSoundVariantIdentity, EntityClientMovementSoundKind,
    EntityClientMovementSoundState, LivingEntityItemBreakState, PigSoundVariantIdentity,
    VanillaEntityDeathSoundPolicy, VanillaEntityEatingSoundPolicy, VanillaEntityEventKind,
    VanillaEntityHurtSoundPolicy, VanillaEntitySoundCondition, VanillaEntitySoundSourceKind,
    VanillaEntitySoundVariantFamily, VanillaEntitySoundVolumePolicy, WolfSoundVariantIdentity,
    VANILLA_ARMADILLO_STATE_IDLE_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID, VANILLA_RABBIT_VARIANT_EVIL_ID,
};
use crate::level::rounded_game_event_param;
use crate::registries::{
    DamageTypeEffects, RegistrySoundEvent, RegistrySoundKind, CAT_SOUND_VARIANT_REGISTRY,
    CHICKEN_SOUND_VARIANT_REGISTRY, COW_SOUND_VARIANT_REGISTRY, DAMAGE_TYPE_REGISTRY,
    PIG_SOUND_VARIANT_REGISTRY, WOLF_SOUND_VARIANT_REGISTRY,
};
use crate::{block_name_is_air, block_name_is_rail, BlockPos, WorldStore};

const JUKEBOX_PLAY_LEVEL_EVENT: i32 = 1010;
const JUKEBOX_STOP_LEVEL_EVENT: i32 = 1011;
const BLOCK_BREAK_LEVEL_EVENT: i32 = 2001;
const BRUSH_BLOCK_COMPLETE_LEVEL_EVENT: i32 = 3008;
const SCULK_SHRIEKER_LEVEL_EVENT: i32 = 3007;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
const VAULT_DEACTIVATE_LEVEL_EVENT: i32 = 3016;
const COBWEB_PLACE_LEVEL_EVENT: i32 = 3018;
const ARROW_HIT_PLAYER_SOUND_EVENT: &str = "minecraft:entity.arrow.hit_player";
const PUFFER_FISH_STING_SOUND_EVENT: &str = "minecraft:entity.puffer_fish.sting";
const ELDER_GUARDIAN_CURSE_SOUND_EVENT: &str = "minecraft:entity.elder_guardian.curse";
const TOTEM_USE_SOUND_EVENT: &str = "minecraft:item.totem.use";
const GUARDIAN_ATTACK_SOUND_EVENT: &str = "minecraft:entity.guardian.attack";
const SNIFFER_DIGGING_SOUND_EVENT: &str = "minecraft:entity.sniffer.digging";
const MINECART_RIDING_SOUND_EVENT: &str = "minecraft:entity.minecart.riding";
const BEE_LOOP_SOUND_EVENT: &str = "minecraft:entity.bee.loop";
const BEE_AGGRESSIVE_LOOP_SOUND_EVENT: &str = "minecraft:entity.bee.loop_aggressive";
const RAVAGER_ATTACK_SOUND_EVENT: &str = "minecraft:entity.ravager.attack";
const IRON_GOLEM_ATTACK_SOUND_EVENT: &str = "minecraft:entity.iron_golem.attack";
const HOGLIN_ATTACK_SOUND_EVENT: &str = "minecraft:entity.hoglin.attack";
const ZOGLIN_ATTACK_SOUND_EVENT: &str = "minecraft:entity.zoglin.attack";
const CREAKING_ATTACK_SOUND_EVENT: &str = "minecraft:entity.creaking.attack";
const CREAKING_SWAY_SOUND_EVENT: &str = "minecraft:entity.creaking.sway";
const HOSTILE_HURT_SOUND_EVENT: &str = "minecraft:entity.hostile.hurt";
const EVOKER_FANGS_ATTACK_SOUND_EVENT: &str = "minecraft:entity.evoker_fangs.attack";
const ZOMBIE_VILLAGER_CURE_SOUND_EVENT: &str = "minecraft:entity.zombie_villager.cure";
const ARMADILLO_PEEK_SOUND_EVENT: &str = "minecraft:entity.armadillo.peek";
const ARMOR_STAND_HIT_SOUND_EVENT: &str = "minecraft:entity.armor_stand.hit";
const PLAYER_HURT_SOUND_EVENT: &str = "minecraft:entity.player.hurt";
const PLAYER_HURT_DROWN_SOUND_EVENT: &str = "minecraft:entity.player.hurt_drown";
const PLAYER_HURT_ON_FIRE_SOUND_EVENT: &str = "minecraft:entity.player.hurt_on_fire";
const PLAYER_HURT_SWEET_BERRY_BUSH_SOUND_EVENT: &str =
    "minecraft:entity.player.hurt_sweet_berry_bush";
const PLAYER_HURT_FREEZE_SOUND_EVENT: &str = "minecraft:entity.player.hurt_freeze";
const PLAYER_DEATH_SOUND_EVENT: &str = "minecraft:entity.player.death";
const ITEM_PICKUP_SOUND_EVENT: &str = "minecraft:entity.item.pickup";
const EXPERIENCE_ORB_PICKUP_SOUND_EVENT: &str = "minecraft:entity.experience_orb.pickup";
const BLAZE_BURN_SOUND_EVENT: &str = "minecraft:entity.blaze.burn";
const WARDEN_HEARTBEAT_SOUND_EVENT: &str = "minecraft:entity.warden.heartbeat";
const ENDER_DRAGON_FLAP_SOUND_EVENT: &str = "minecraft:entity.ender_dragon.flap";
const ENDER_DRAGON_GROWL_SOUND_EVENT: &str = "minecraft:entity.ender_dragon.growl";
const GUARDIAN_FLOP_SOUND_EVENT: &str = "minecraft:entity.guardian.flop";
const ELDER_GUARDIAN_FLOP_SOUND_EVENT: &str = "minecraft:entity.elder_guardian.flop";
const SLIME_SQUISH_SOUND_EVENT: &str = "minecraft:entity.slime.squish";
const SLIME_SQUISH_SMALL_SOUND_EVENT: &str = "minecraft:entity.slime.squish_small";
const MAGMA_CUBE_SQUISH_SOUND_EVENT: &str = "minecraft:entity.magma_cube.squish";
const MAGMA_CUBE_SQUISH_SMALL_SOUND_EVENT: &str = "minecraft:entity.magma_cube.squish_small";
const CAMEL_DASH_READY_SOUND_EVENT: &str = "minecraft:entity.camel.dash_ready";
const CAMEL_HUSK_DASH_READY_SOUND_EVENT: &str = "minecraft:entity.camel_husk.dash_ready";
const CAMEL_STAND_SOUND_EVENT: &str = "minecraft:entity.camel.stand";
const CAMEL_HUSK_STAND_SOUND_EVENT: &str = "minecraft:entity.camel_husk.stand";
const NAUTILUS_DASH_READY_SOUND_EVENT: &str = "minecraft:entity.nautilus.dash_ready";
const NAUTILUS_DASH_READY_LAND_SOUND_EVENT: &str = "minecraft:entity.nautilus.dash_ready_land";
const ZOMBIE_NAUTILUS_DASH_READY_SOUND_EVENT: &str = "minecraft:entity.zombie_nautilus.dash_ready";
const ZOMBIE_NAUTILUS_DASH_READY_LAND_SOUND_EVENT: &str =
    "minecraft:entity.zombie_nautilus.dash_ready_land";
const BREEZE_DEFLECT_SOUND_EVENT: &str = "minecraft:entity.breeze.deflect";
const FIREWORK_ROCKET_LAUNCH_SOUND_EVENT: &str = "minecraft:entity.firework_rocket.launch";
const PLAYER_BIG_FALL_SOUND_EVENT: &str = "minecraft:entity.player.big_fall";
const PLAYER_SMALL_FALL_SOUND_EVENT: &str = "minecraft:entity.player.small_fall";
const TRIDENT_RETURN_SOUND_EVENT: &str = "minecraft:item.trident.return";
const GLOBAL_LEVEL_EVENT_SOUND_DISTANCE: f64 = 2.0;
const VANILLA_VEC3_NORMALIZE_EPSILON: f64 = 1.0e-5;
const SCULK_SHRIEKER_TOP_Y: f64 = 0.5;
const RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const RANDOM_INCREMENT: u64 = 11;
const RANDOM_MASK: u64 = (1_u64 << 48) - 1;
const RANDOM_FLOAT_MULTIPLIER: f32 = 5.960_464_5e-8;
const RANDOM_DOUBLE_DIVISOR: f64 = (1_u64 << 53) as f64;
/// Fixed local seed for `LevelEventSoundRandomState::default()`. Vanilla seeds
/// the client `level.random` from a wall-clock `RandomSupport.generateUniqueSeed`;
/// a fixed seed keeps the derived level-event pitch stream deterministic.
const DEFAULT_LEVEL_EVENT_SOUND_SEED: i64 = 0;
/// Fixed local seed for the client sound-seed generator. Vanilla derives sound
/// seeds from `RandomSource.nextLong()` (`Level.soundSeedGenerator` /
/// `ClientLevel.random`), itself seeded from wall-clock unique seeds; a distinct
/// fixed seed keeps `WorldStore` serializable and deterministic.
const SOUND_SEED_RANDOM_INITIAL_SEED: i64 = 0x5EED_50_1D_A0D_10;
const MAX_PENDING_LOCAL_INTERACTION_SOUND_ACTIONS: usize = 8;

const fn default_true() -> bool {
    true
}

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
        Self::with_seed(DEFAULT_LEVEL_EVENT_SOUND_SEED)
    }
}

/// Deterministic client-local sound-seed generator.
///
/// Vanilla derives per-sound seeds from a `RandomSource.nextLong()` chain
/// (`Level.soundSeedGenerator` / `ClientLevel.random`), itself seeded from a
/// wall-clock `RandomSupport.generateUniqueSeed()`. To keep `WorldStore`
/// serializable and deterministic (same packet stream -> same state), this
/// replaces the wall-clock seed with a fixed local seed while preserving the
/// `LegacyRandomSource.nextLong()` advancement, so successive draws stay unique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundSeedRandomState {
    seed: u64,
}

impl SoundSeedRandomState {
    fn with_seed(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
        }
    }

    /// Next `RandomSource.nextLong()` sound seed.
    pub fn next_long(&mut self) -> i64 {
        let high = (self.next_bits(32) as i32 as i64) << 32;
        let low = self.next_bits(32) as i32 as i64;
        high.wrapping_add(low)
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

impl Default for SoundSeedRandomState {
    fn default() -> Self {
        Self::with_seed(SOUND_SEED_RANDOM_INITIAL_SEED)
    }
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
    #[serde(default)]
    pub sound_seed_random: SoundSeedRandomState,
    #[serde(default)]
    pub(crate) active_tickable_entity_sounds: BTreeMap<i32, ActiveTickableEntitySound>,
    #[serde(skip)]
    pub(crate) pending_local_interaction_sound_actions: VecDeque<LocalInteractionSoundAction>,
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
    #[serde(default)]
    pub looping: bool,
    #[serde(default = "default_true")]
    pub linear_attenuation: bool,
    #[serde(default)]
    pub tickable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LocalInteractionSoundEffect {
    Local(LocalSoundEventState),
    Positioned(SoundEventState),
    Entity {
        state: SoundEntityEventState,
        position: Option<ProtocolVec3d>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickableEntitySoundState {
    pub event: SoundEntityEventState,
    pub position: ProtocolVec3d,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum ActiveTickableEntitySound {
    GuardianAttack { seed: i64 },
    SnifferDigging { seed: i64 },
    Minecart { seed: i64, volume: f32, pitch: f32 },
    Bee { seed: i64 },
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
    #[serde(default)]
    pub place_sound: String,
    pub hit_sound: String,
    pub volume: f32,
    pub pitch: f32,
}

impl WorldStore {
    pub(crate) fn queue_local_interaction_sound_action(
        &mut self,
        action: LocalInteractionSoundAction,
    ) {
        if self
            .client_audio
            .pending_local_interaction_sound_actions
            .len()
            >= MAX_PENDING_LOCAL_INTERACTION_SOUND_ACTIONS
        {
            self.client_audio
                .pending_local_interaction_sound_actions
                .pop_front();
        }
        self.client_audio
            .pending_local_interaction_sound_actions
            .push_back(action);
    }

    pub fn take_pending_local_interaction_sound_actions(
        &mut self,
    ) -> Vec<LocalInteractionSoundAction> {
        self.client_audio
            .pending_local_interaction_sound_actions
            .drain(..)
            .collect()
    }

    pub fn set_default_block_sound_profiles(
        &mut self,
        profiles: BTreeMap<String, WorldBlockSoundProfile>,
    ) {
        self.items.default_block_sound_profiles = profiles
            .into_iter()
            .filter(|(block_name, profile)| {
                !block_name.is_empty()
                    && !profile.hit_sound.is_empty()
                    && !profile.place_sound.is_empty()
                    && profile.volume.is_finite()
                    && profile.pitch.is_finite()
            })
            .collect::<BTreeMap<_, _>>()
            .into();
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
            looping: false,
            linear_attenuation: true,
            tickable: false,
        };
        self.client_audio.last_sound_entity = Some(state.clone());
        self.counters.sound_entity_events_applied += 1;
        Some(state)
    }

    pub fn guardian_attack_sound_for_entity(
        &mut self,
        entity_id: i32,
    ) -> Option<SoundEntityEventState> {
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let (_, attack_scale) = self.entities.guardian_attack_sound_state(entity_id)?;
        let seed = self.next_sound_seed();
        let state = SoundEntityEventState {
            sound: direct_sound_holder(GUARDIAN_ATTACK_SOUND_EVENT),
            source: SoundSource::Hostile.as_str().to_string(),
            entity_id,
            volume: attack_scale * attack_scale,
            pitch: 0.7 + 0.5 * attack_scale,
            seed,
            looping: true,
            linear_attenuation: false,
            tickable: true,
        };
        self.client_audio.active_tickable_entity_sounds.insert(
            entity_id,
            ActiveTickableEntitySound::GuardianAttack { seed },
        );
        self.client_audio.last_sound_entity = Some(state.clone());
        Some(state)
    }

    pub fn sniffer_digging_sound_for_entity(
        &mut self,
        entity_id: i32,
    ) -> Option<SoundEntityEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_SNIFFER_ID
            || self.entities.is_silent(entity_id)?
        {
            return None;
        }
        let seed = self.next_sound_seed();
        let state = SoundEntityEventState {
            sound: direct_sound_holder(SNIFFER_DIGGING_SOUND_EVENT),
            source: SoundSource::Neutral.as_str().to_string(),
            entity_id,
            volume: 1.0,
            pitch: 1.0,
            seed,
            looping: false,
            linear_attenuation: true,
            tickable: true,
        };
        self.client_audio.active_tickable_entity_sounds.insert(
            entity_id,
            ActiveTickableEntitySound::SnifferDigging { seed },
        );
        self.client_audio.last_sound_entity = Some(state.clone());
        Some(state)
    }

    /// Vanilla `ClientLevel.postAddEntitySoundInstance`: minecarts and bees
    /// immediately acquire a client-owned looping sound instance after their
    /// canonical entity has been added to the level.
    pub(crate) fn post_add_entity_sound_for_entity(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> Option<SoundEntityEventState> {
        let active = if is_vanilla_minecart_type(entity_type_id) {
            ActiveTickableEntitySound::Minecart {
                seed: self.next_sound_seed(),
                volume: 0.0,
                pitch: 0.0,
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_BEE_ID {
            ActiveTickableEntitySound::Bee {
                seed: self.next_sound_seed(),
            }
        } else {
            return None;
        };
        self.client_audio
            .active_tickable_entity_sounds
            .insert(entity_id, active);
        let state = SoundEntityEventState {
            sound: direct_sound_holder(if is_vanilla_minecart_type(entity_type_id) {
                MINECART_RIDING_SOUND_EVENT
            } else {
                BEE_LOOP_SOUND_EVENT
            }),
            source: SoundSource::Neutral.as_str().to_string(),
            entity_id,
            volume: 0.0,
            pitch: 0.0,
            seed: match active {
                ActiveTickableEntitySound::Minecart { seed, .. }
                | ActiveTickableEntitySound::Bee { seed } => seed,
                ActiveTickableEntitySound::GuardianAttack { .. }
                | ActiveTickableEntitySound::SnifferDigging { .. } => unreachable!(),
            },
            looping: true,
            linear_attenuation: true,
            tickable: true,
        };
        self.client_audio.last_sound_entity = Some(state.clone());
        Some(state)
    }

    /// Advances retained state whose vanilla sound instance mutates once per
    /// client tick. World time is advanced before callers project bee anger;
    /// minecart pitch itself advances only while the tick-rate manager runs.
    pub fn advance_tickable_entity_sounds(&mut self, client_ticks: u32, running_ticks: u32) {
        if client_ticks == 0 {
            return;
        }
        let minecart_updates = self
            .client_audio
            .active_tickable_entity_sounds
            .iter()
            .filter_map(|(&entity_id, active)| {
                let ActiveTickableEntitySound::Minecart { pitch, .. } = *active else {
                    return None;
                };
                let speed = self
                    .entities
                    .transform_state(entity_id)
                    .map(|transform| horizontal_speed(transform.delta_movement));
                let (next_volume, next_pitch) = match speed {
                    Some(speed)
                        if running_ticks > 0
                            && !self.minecart_sound_off_rail(entity_id)
                            && speed >= 0.01 =>
                    {
                        (
                            sound_lerp(speed.clamp(0.0, 0.5), 0.0, 0.7),
                            (pitch + 0.0025 * running_ticks as f32).clamp(0.0, 1.0),
                        )
                    }
                    _ => (0.0, 0.0),
                };
                Some((entity_id, next_volume, next_pitch))
            })
            .collect::<Vec<_>>();
        for (entity_id, volume, pitch) in minecart_updates {
            if let Some(ActiveTickableEntitySound::Minecart {
                volume: active_volume,
                pitch: active_pitch,
                ..
            }) = self
                .client_audio
                .active_tickable_entity_sounds
                .get_mut(&entity_id)
            {
                *active_volume = volume;
                *active_pitch = pitch;
            }
        }
    }

    pub fn tickable_entity_sounds(&self) -> Vec<TickableEntitySoundState> {
        self.client_audio
            .active_tickable_entity_sounds
            .iter()
            .filter_map(|(&entity_id, active)| self.tickable_entity_sound_for(entity_id, *active))
            .collect()
    }

    fn tickable_entity_sound_for(
        &self,
        entity_id: i32,
        active: ActiveTickableEntitySound,
    ) -> Option<TickableEntitySoundState> {
        let (position, sound, source, volume, pitch, seed, looping, attenuation) = match active {
            ActiveTickableEntitySound::GuardianAttack { seed } => {
                if self.entities.is_silent(entity_id)? {
                    return None;
                }
                let (position, attack_scale) =
                    self.entities.guardian_attack_sound_state(entity_id)?;
                (
                    position,
                    GUARDIAN_ATTACK_SOUND_EVENT,
                    SoundSource::Hostile,
                    attack_scale * attack_scale,
                    0.7 + 0.5 * attack_scale,
                    seed,
                    true,
                    false,
                )
            }
            ActiveTickableEntitySound::SnifferDigging { seed } => {
                if self.entities.is_silent(entity_id)? {
                    return None;
                }
                if !self.entities.sniffer_can_play_digging_sound(entity_id)? {
                    return None;
                }
                let transform = self.entities.transform_state(entity_id)?;
                (
                    transform.position,
                    SNIFFER_DIGGING_SOUND_EVENT,
                    SoundSource::Neutral,
                    1.0,
                    1.0,
                    seed,
                    false,
                    true,
                )
            }
            ActiveTickableEntitySound::Minecart {
                seed,
                volume,
                pitch,
            } => {
                let transform = self.entities.transform_state(entity_id)?;
                if !is_vanilla_minecart_type(transform.entity_type_id) {
                    return None;
                }
                (
                    transform.position,
                    MINECART_RIDING_SOUND_EVENT,
                    SoundSource::Neutral,
                    volume,
                    pitch,
                    seed,
                    true,
                    true,
                )
            }
            ActiveTickableEntitySound::Bee { seed } => {
                let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
                let bee = self.entities.bee_sound_state(entity_id, game_time)?;
                let speed = horizontal_speed(bee.transform.delta_movement);
                let (pitch, volume) = if speed >= 0.01 {
                    let (min_pitch, max_pitch) = if bee.is_baby { (1.1, 1.5) } else { (0.7, 1.1) };
                    (
                        sound_lerp(speed.clamp(min_pitch, max_pitch), min_pitch, max_pitch),
                        sound_lerp(speed.clamp(0.0, 0.5), 0.0, 1.2),
                    )
                } else {
                    (0.0, 0.0)
                };
                (
                    bee.transform.position,
                    if bee.angry {
                        BEE_AGGRESSIVE_LOOP_SOUND_EVENT
                    } else {
                        BEE_LOOP_SOUND_EVENT
                    },
                    SoundSource::Neutral,
                    volume,
                    pitch,
                    seed,
                    true,
                    true,
                )
            }
        };
        Some(TickableEntitySoundState {
            event: SoundEntityEventState {
                sound: direct_sound_holder(sound),
                source: source.as_str().to_string(),
                entity_id,
                volume,
                pitch,
                seed,
                looping,
                linear_attenuation: attenuation,
                tickable: true,
            },
            position: ProtocolVec3d {
                x: position.x,
                y: position.y,
                z: position.z,
            },
        })
    }

    fn minecart_sound_off_rail(&self, entity_id: i32) -> bool {
        if !self.is_feature_enabled("minecraft:minecart_improvements") {
            return false;
        }
        let Some(transform) = self.entities.transform_state(entity_id) else {
            return true;
        };
        let rail_below = BlockPos {
            x: transform.position.x.floor() as i32,
            y: (transform.position.y - 0.1 - 1.0e-5).floor() as i32,
            z: transform.position.z.floor() as i32,
        };
        let current_block = BlockPos {
            y: transform.position.y.floor() as i32,
            ..rail_below
        };
        ![rail_below, current_block].into_iter().any(|pos| {
            self.probe_block(pos)
                .and_then(|block| block.block_name)
                .as_deref()
                .is_some_and(block_name_is_rail)
        })
    }

    pub(crate) fn clear_tickable_entity_sounds(&mut self) {
        self.client_audio.active_tickable_entity_sounds.clear();
    }

    pub(crate) fn remove_tickable_entity_sounds(&mut self, entity_ids: &[i32]) {
        for entity_id in entity_ids {
            self.client_audio
                .active_tickable_entity_sounds
                .remove(entity_id);
        }
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

    /// Resolves vanilla `LivingEntity.breakItem` audio from the effective stack component.
    /// A synchronized patch wins, an explicit removal suppresses sound, and otherwise the
    /// source-generated item prototype default supplies the common/Shield/Wolf Armor event.
    pub fn living_entity_item_break_sound(
        &mut self,
        state: &LivingEntityItemBreakState,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if self.entities.is_silent(state.entity_id)? {
            return None;
        }
        let patch = &state.item_stack.component_patch;
        let sound = if let Some(summary) = patch.break_sound.as_ref() {
            sound_event_summary_holder(summary)?
        } else if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID)
        {
            return None;
        } else {
            direct_sound_holder(vanilla_item_default_break_sound_resource_id(
                state.item_stack.item_id?,
            )?)
        };
        let entity_type_id = self.probe_entity_transform(state.entity_id)?.entity_type_id;
        let sound = SoundEventState {
            sound,
            source: vanilla_entity_sound_source(entity_type_id)
                .as_str()
                .to_string(),
            position: ProtocolVec3d {
                x: state.position.x,
                y: state.position.y,
                z: state.position.z,
            },
            volume: 0.8,
            pitch: 0.8 + next_float().clamp(0.0, 1.0) * 0.4,
            seed: self.next_sound_seed(),
            distance_delay: false,
        };
        Some(self.record_positioned_sound(sound))
    }

    pub fn living_damage_hurt_sound_for_entity(
        &mut self,
        entity_id: i32,
        source_type_id: i32,
        effects: DamageTypeEffects,
        next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        self.living_damage_hurt_sound_for_entity_with_optional_effects(
            entity_id,
            source_type_id,
            Some(effects),
            next_float,
        )
    }

    pub(crate) fn local_player_damage_hurt_sound(
        &mut self,
        effects: Option<DamageTypeEffects>,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let position = self.local_player_pose()?.position;
        let state = SoundEventState {
            sound: direct_sound_holder(player_hurt_sound_event(effects?)),
            source: SoundSource::Players.as_str().to_string(),
            position,
            volume: 1.0,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub(crate) fn living_damage_hurt_sound_for_entity_with_optional_effects(
        &mut self,
        entity_id: i32,
        source_type_id: i32,
        effects: Option<DamageTypeEffects>,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let policy = vanilla_entity_metadata_type_hurt_sound_policy(transform.entity_type_id)?;
        let sound = match policy {
            VanillaEntityHurtSoundPolicy::NonLiving => return None,
            VanillaEntityHurtSoundPolicy::Fixed(sound) => direct_sound_holder(sound),
            VanillaEntityHurtSoundPolicy::Conditional {
                condition,
                when_true,
                when_false,
            } => direct_sound_holder(if self.living_sound_condition(entity_id, condition)? {
                when_true
            } else {
                when_false
            }),
            VanillaEntityHurtSoundPolicy::NautilusAgeWater {
                baby_underwater,
                baby_land,
                adult_underwater,
                adult_land,
            } => {
                let baby = self.entities.is_baby(entity_id)?;
                let underwater = self.living_sound_fluid_state(entity_id).1;
                direct_sound_holder(match (baby, underwater) {
                    (true, true) => baby_underwater,
                    (true, false) => baby_land,
                    (false, true) => adult_underwater,
                    (false, false) => adult_land,
                })
            }
            VanillaEntityHurtSoundPolicy::CopperGolemOxidation {
                unaffected,
                exposed,
                weathered,
                oxidized,
            } => {
                let accessors = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
                    transform.entity_type_id,
                )?;
                let state = self.entities.metadata_int_for_entity(
                    entity_id,
                    accessors.weather_state_data_id,
                    accessors.weather_state_default,
                )?;
                direct_sound_holder(match state {
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID => unaffected,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID => exposed,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => weathered,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => oxidized,
                    _ => return None,
                })
            }
            VanillaEntityHurtSoundPolicy::SoundVariant(family) => self.living_sound_variant(
                entity_id,
                transform.entity_type_id,
                family,
                RegistrySoundKind::Hurt,
            )?,
            VanillaEntityHurtSoundPolicy::WolfVariant { armor_damage } => {
                if self
                    .equipment_item(entity_id, EquipmentSlot::Body)
                    .and_then(|item| item.item_id)
                    == Some(VANILLA_ITEM_WOLF_ARMOR_ID)
                    && !self.registries().holder_is_in_tag(
                        DAMAGE_TYPE_REGISTRY,
                        "minecraft:bypasses_wolf_armor",
                        source_type_id,
                    )
                {
                    direct_sound_holder(armor_damage)
                } else {
                    self.living_damage_wolf_sound_variant(entity_id, transform.entity_type_id)?
                }
            }
            VanillaEntityHurtSoundPolicy::PlayerDamageType => {
                direct_sound_holder(player_hurt_sound_event(effects?))
            }
        };
        let volume = self.living_sound_volume(entity_id, transform.entity_type_id)?;
        let state = SoundEventState {
            sound,
            source: self
                .living_sound_source(entity_id, transform.entity_type_id)?
                .as_str()
                .to_string(),
            position: ProtocolVec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            volume,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    fn living_sound_variant(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        family: VanillaEntitySoundVariantFamily,
        kind: RegistrySoundKind,
    ) -> Option<SoundHolderState> {
        let baby = self.entities.is_baby(entity_id)?;
        match family {
            VanillaEntitySoundVariantFamily::Cat => {
                let accessors =
                    vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
                        entity_type_id,
                    )?;
                let builtin_sound_event: fn(CatSoundVariantIdentity, bool) -> &'static str =
                    match kind {
                        RegistrySoundKind::Hurt => {
                            CatSoundVariantIdentity::builtin_hurt_sound_event
                        }
                        RegistrySoundKind::Death => {
                            CatSoundVariantIdentity::builtin_death_sound_event
                        }
                    };
                self.sound_variant_sound(
                    entity_id,
                    CAT_SOUND_VARIANT_REGISTRY,
                    accessors.sound_variant_data_id,
                    EntityDataRegistryHolder::CatSoundVariant,
                    accessors.sound_variant_default_resource_id,
                    CatSoundVariantIdentity::for_resource_id,
                    builtin_sound_event,
                    baby,
                    kind,
                )
            }
            VanillaEntitySoundVariantFamily::Chicken => {
                let accessors =
                    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
                        VANILLA_ENTITY_TYPE_CHICKEN_ID,
                    )?;
                let builtin_sound_event: fn(ChickenSoundVariantIdentity, bool) -> &'static str =
                    match kind {
                        RegistrySoundKind::Hurt => {
                            ChickenSoundVariantIdentity::builtin_hurt_sound_event
                        }
                        RegistrySoundKind::Death => {
                            ChickenSoundVariantIdentity::builtin_death_sound_event
                        }
                    };
                self.sound_variant_sound(
                    entity_id,
                    CHICKEN_SOUND_VARIANT_REGISTRY,
                    accessors.sound_variant_data_id,
                    EntityDataRegistryHolder::ChickenSoundVariant,
                    accessors.sound_variant_default_resource_id,
                    ChickenSoundVariantIdentity::for_resource_id,
                    builtin_sound_event,
                    baby,
                    kind,
                )
            }
            VanillaEntitySoundVariantFamily::Cow => {
                let accessors =
                    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
                        VANILLA_ENTITY_TYPE_COW_ID,
                    )?;
                let builtin_sound_event: fn(CowSoundVariantIdentity, bool) -> &'static str =
                    match kind {
                        RegistrySoundKind::Hurt => {
                            CowSoundVariantIdentity::builtin_hurt_sound_event
                        }
                        RegistrySoundKind::Death => {
                            CowSoundVariantIdentity::builtin_death_sound_event
                        }
                    };
                self.sound_variant_sound(
                    entity_id,
                    COW_SOUND_VARIANT_REGISTRY,
                    accessors.sound_variant_data_id,
                    EntityDataRegistryHolder::CowSoundVariant,
                    accessors.sound_variant_default_resource_id,
                    CowSoundVariantIdentity::for_resource_id,
                    builtin_sound_event,
                    baby,
                    kind,
                )
            }
            VanillaEntitySoundVariantFamily::Pig => {
                let accessors =
                    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
                        VANILLA_ENTITY_TYPE_PIG_ID,
                    )?;
                let builtin_sound_event: fn(PigSoundVariantIdentity, bool) -> &'static str =
                    match kind {
                        RegistrySoundKind::Hurt => {
                            PigSoundVariantIdentity::builtin_hurt_sound_event
                        }
                        RegistrySoundKind::Death => {
                            PigSoundVariantIdentity::builtin_death_sound_event
                        }
                    };
                self.sound_variant_sound(
                    entity_id,
                    PIG_SOUND_VARIANT_REGISTRY,
                    accessors.sound_variant_data_id,
                    EntityDataRegistryHolder::PigSoundVariant,
                    accessors.sound_variant_default_resource_id,
                    PigSoundVariantIdentity::for_resource_id,
                    builtin_sound_event,
                    baby,
                    kind,
                )
            }
            VanillaEntitySoundVariantFamily::Wolf => {
                let accessors =
                    vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
                        entity_type_id,
                    )?;
                let builtin_sound_event: fn(WolfSoundVariantIdentity, bool) -> &'static str =
                    match kind {
                        RegistrySoundKind::Hurt => {
                            WolfSoundVariantIdentity::builtin_hurt_sound_event
                        }
                        RegistrySoundKind::Death => {
                            WolfSoundVariantIdentity::builtin_death_sound_event
                        }
                    };
                self.sound_variant_sound(
                    entity_id,
                    WOLF_SOUND_VARIANT_REGISTRY,
                    accessors.sound_variant_data_id,
                    EntityDataRegistryHolder::WolfSoundVariant,
                    accessors.sound_variant_default_resource_id,
                    WolfSoundVariantIdentity::for_resource_id,
                    builtin_sound_event,
                    baby,
                    kind,
                )
            }
        }
    }

    fn living_damage_wolf_sound_variant(
        &self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> Option<SoundHolderState> {
        self.living_sound_variant(
            entity_id,
            entity_type_id,
            VanillaEntitySoundVariantFamily::Wolf,
            RegistrySoundKind::Hurt,
        )
    }

    fn living_sound_volume(&self, entity_id: i32, entity_type_id: i32) -> Option<f32> {
        match vanilla_entity_metadata_type_sound_volume_policy(entity_type_id)? {
            VanillaEntitySoundVolumePolicy::NonLiving => None,
            VanillaEntitySoundVolumePolicy::Fixed(bits) => Some(f32::from_bits(bits)),
            VanillaEntitySoundVolumePolicy::SlimeSize => {
                let (data_id, default) =
                    vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)?;
                Some(
                    0.4 * self
                        .entities
                        .metadata_int_for_entity(entity_id, data_id, default)?
                        as f32,
                )
            }
            VanillaEntitySoundVolumePolicy::HappyGhastAge => {
                Some(if self.entities.is_baby(entity_id)? {
                    1.0
                } else {
                    4.0
                })
            }
        }
    }

    fn living_sound_condition(
        &self,
        entity_id: i32,
        condition: VanillaEntitySoundCondition,
    ) -> Option<bool> {
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        match condition {
            VanillaEntitySoundCondition::ArmadilloScared => {
                let (data_id, default) =
                    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
                        entity_type_id,
                    )?;
                Some(
                    self.entities
                        .metadata_int_for_entity(entity_id, data_id, default)?
                        != VANILLA_ARMADILLO_STATE_IDLE_ID,
                )
            }
            VanillaEntitySoundCondition::InWater => {
                Some(self.living_sound_fluid_state(entity_id).0)
            }
            VanillaEntitySoundCondition::UnderWater => {
                Some(self.living_sound_fluid_state(entity_id).1)
            }
            VanillaEntitySoundCondition::Baby => self.entities.is_baby(entity_id),
            VanillaEntitySoundCondition::GoatScreaming => {
                let accessors =
                    vanilla_entity_metadata_goat_accessors_for_entity_type(entity_type_id)?;
                self.entities.metadata_bool_for_entity(
                    entity_id,
                    accessors.screaming_data_id,
                    accessors.screaming_default,
                )
            }
            VanillaEntitySoundCondition::Tiny => {
                let (data_id, default) =
                    vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)?;
                Some(
                    self.entities
                        .metadata_int_for_entity(entity_id, data_id, default)?
                        <= 1,
                )
            }
            VanillaEntitySoundCondition::ShulkerClosed => {
                let accessors =
                    vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)?;
                Some(
                    self.entities.metadata_byte_for_entity(
                        entity_id,
                        accessors.peek_data_id,
                        accessors.peek_default,
                    )? == 0,
                )
            }
            VanillaEntitySoundCondition::CreakingHeartBound => {
                self.entities.creaking_is_heart_bound(entity_id)
            }
        }
    }

    fn living_sound_fluid_state(&self, entity_id: i32) -> (bool, bool) {
        let Some(input) = self.entities.fluid_contact_input(entity_id) else {
            return (false, false);
        };
        let contact = crate::client::entity_fluid_contact(
            self,
            input.aabb_min,
            input.aabb_max,
            input.position.y,
            input.eye_position,
        );
        let in_water = contact.in_water();
        (in_water, in_water && contact.eye_in_water)
    }

    fn living_sound_source(&self, entity_id: i32, entity_type_id: i32) -> Option<SoundSource> {
        if vanilla_entity_metadata_type_sound_source_kind(entity_type_id)
            != Some(VanillaEntitySoundSourceKind::Dynamic)
        {
            return Some(vanilla_entity_sound_source(entity_type_id));
        }
        let (data_id, default) =
            crate::entities::vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
                entity_type_id,
            )?;
        Some(
            if self
                .entities
                .metadata_int_for_entity(entity_id, data_id, default)?
                == VANILLA_RABBIT_VARIANT_EVIL_ID
            {
                SoundSource::Hostile
            } else {
                SoundSource::Neutral
            },
        )
    }

    pub fn entity_attack_event_sound_for_entity(
        &mut self,
        entity_id: i32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let sound = match transform.entity_type_id {
            // Vanilla `Ravager.handleEntityEvent(4)` and
            // `IronGolem.handleEntityEvent(4)` use explicit `playSound(sound, 1, 1)`.
            VANILLA_ENTITY_TYPE_RAVAGER_ID => RAVAGER_ATTACK_SOUND_EVENT,
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => IRON_GOLEM_ATTACK_SOUND_EVENT,
            _ => return None,
        };
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(sound),
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

    pub(crate) fn entity_event_voice_sound_for_entity(
        &mut self,
        entity_id: i32,
        event_kind: VanillaEntityEventKind,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let sound = match (transform.entity_type_id, event_kind) {
            (VANILLA_ENTITY_TYPE_HOGLIN_ID, VanillaEntityEventKind::HoglinAttack) => {
                HOGLIN_ATTACK_SOUND_EVENT
            }
            (VANILLA_ENTITY_TYPE_ZOGLIN_ID, VanillaEntityEventKind::ZoglinAttack) => {
                ZOGLIN_ATTACK_SOUND_EVENT
            }
            (VANILLA_ENTITY_TYPE_CREAKING_ID, VanillaEntityEventKind::CreakingAttack) => {
                CREAKING_ATTACK_SOUND_EVENT
            }
            (VANILLA_ENTITY_TYPE_CREAKING_ID, VanillaEntityEventKind::CreakingInvulnerable) => {
                if self.entities.creaking_is_heart_bound(entity_id)? {
                    CREAKING_SWAY_SOUND_EVENT
                } else {
                    HOSTILE_HURT_SOUND_EVENT
                }
            }
            _ => return None,
        };
        let pitch_base = if self.entities.is_baby(entity_id)? {
            1.5
        } else {
            1.0
        };
        let state = SoundEventState {
            sound: direct_sound_holder(sound),
            source: vanilla_entity_sound_source(transform.entity_type_id)
                .as_str()
                .to_string(),
            position: ProtocolVec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            volume: 1.0,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + pitch_base,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn evoker_fangs_attack_sound_for_entity(
        &mut self,
        entity_id: i32,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID {
            return None;
        }
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(EVOKER_FANGS_ATTACK_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: ProtocolVec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            volume: 1.0,
            pitch: next_float().clamp(0.0, 1.0) * 0.2 + 0.85,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn zombie_villager_cure_sound_for_entity(
        &mut self,
        entity_id: i32,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID {
            return None;
        }
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let pose = self.probe_entity_camera_pose(entity_id)?;
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(ZOMBIE_VILLAGER_CURE_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: ProtocolVec3d {
                x: pose.position.x,
                y: pose.position.y + f64::from(pose.eye_height),
                z: pose.position.z,
            },
            volume: 1.0 + next_float().clamp(0.0, 1.0),
            pitch: next_float().clamp(0.0, 1.0) * 0.7 + 0.3,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn armadillo_peek_sound_for_entity(&mut self, entity_id: i32) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ARMADILLO_ID {
            return None;
        }
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(ARMADILLO_PEEK_SOUND_EVENT),
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

    pub fn armor_stand_hit_sound_for_entity(&mut self, entity_id: i32) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            return None;
        }
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(ARMOR_STAND_HIT_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: ProtocolVec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            volume: 0.3,
            pitch: 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn living_death_sound_for_entity(
        &mut self,
        entity_id: i32,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let policy = vanilla_entity_metadata_type_death_sound_policy(transform.entity_type_id)?;
        let sound = match policy {
            VanillaEntityDeathSoundPolicy::NonLiving => return None,
            VanillaEntityDeathSoundPolicy::Fixed(sound) => direct_sound_holder(sound),
            VanillaEntityDeathSoundPolicy::Conditional {
                condition,
                when_true,
                when_false,
            } => direct_sound_holder(if self.living_sound_condition(entity_id, condition)? {
                when_true
            } else {
                when_false
            }),
            VanillaEntityDeathSoundPolicy::NautilusAgeWater {
                baby_underwater,
                baby_land,
                adult_underwater,
                adult_land,
            } => {
                let baby = self.entities.is_baby(entity_id)?;
                let underwater = self.living_sound_fluid_state(entity_id).1;
                direct_sound_holder(match (baby, underwater) {
                    (true, true) => baby_underwater,
                    (true, false) => baby_land,
                    (false, true) => adult_underwater,
                    (false, false) => adult_land,
                })
            }
            VanillaEntityDeathSoundPolicy::CopperGolemOxidation {
                unaffected,
                exposed,
                weathered,
                oxidized,
            } => {
                let accessors = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
                    transform.entity_type_id,
                )?;
                let state = self.entities.metadata_int_for_entity(
                    entity_id,
                    accessors.weather_state_data_id,
                    accessors.weather_state_default,
                )?;
                direct_sound_holder(match state {
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID => unaffected,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID => exposed,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => weathered,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => oxidized,
                    _ => return None,
                })
            }
            VanillaEntityDeathSoundPolicy::SoundVariant(family) => self.living_sound_variant(
                entity_id,
                transform.entity_type_id,
                family,
                RegistrySoundKind::Death,
            )?,
        };
        let volume = self.living_sound_volume(entity_id, transform.entity_type_id)?;
        let state = SoundEventState {
            sound,
            source: self
                .living_sound_source(entity_id, transform.entity_type_id)?
                .as_str()
                .to_string(),
            position: ProtocolVec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            volume,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub(crate) fn local_player_death_sound(
        &mut self,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let position = self.local_player_pose()?.position;
        let state = SoundEventState {
            sound: direct_sound_holder(PLAYER_DEATH_SOUND_EVENT),
            source: SoundSource::Players.as_str().to_string(),
            position,
            volume: 1.0,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    #[allow(clippy::too_many_arguments)]
    fn sound_variant_sound<T: Copy>(
        &self,
        entity_id: i32,
        registry: &str,
        data_id: u8,
        serializer: EntityDataRegistryHolder,
        default_resource_id: &str,
        for_resource_id: fn(&str) -> Option<T>,
        builtin_sound_event: fn(T, bool) -> &'static str,
        baby: bool,
        kind: RegistrySoundKind,
    ) -> Option<SoundHolderState> {
        let metadata_reference = self
            .entities
            .metadata_registry_reference_for_entity(entity_id, data_id, serializer)?;
        let metadata_identity = match metadata_reference.as_ref() {
            Some(reference) => Some(reference.identity()?),
            None => None,
        };
        if let Some(identity) = metadata_identity {
            let revision = self.registry_access_revision();
            let identity_revision = identity.revision();
            if identity.registry() != registry
                || identity_revision.session != revision.session
                || identity_revision.contents != revision.contents
                || identity_revision.tags != revision.tags
            {
                return None;
            }
        }
        if self.registry_content(registry).is_some() {
            let holder_id = metadata_reference
                .as_ref()
                .map(|reference| reference.holder_id())
                .or_else(|| {
                    self.registry_holder_id_by_resource_id(registry, default_resource_id)
                })?;
            let entry = self.registry_entry_by_holder_id(registry, holder_id)?;
            let sound = match kind {
                RegistrySoundKind::Hurt => {
                    self.synchronized_sound_variant_hurt_sound(registry, holder_id, baby)
                }
                RegistrySoundKind::Death => {
                    self.synchronized_sound_variant_death_sound(registry, holder_id, baby)
                }
            }
            .ok()?;
            if let Some(sound) = sound {
                return Some(registry_sound_holder(sound));
            }
            if entry.has_data || entry.known_pack_data().is_some() {
                return None;
            }
            let identity = for_resource_id(&entry.id)?;
            return Some(direct_sound_holder(builtin_sound_event(identity, baby)));
        }

        let resource_id = metadata_identity
            .map(|identity| identity.resource_id())
            .unwrap_or(default_resource_id);
        let identity = for_resource_id(resource_id)?;
        Some(direct_sound_holder(builtin_sound_event(identity, baby)))
    }

    pub fn take_item_entity_pickup_sound_with_random(
        &self,
        item_id: i32,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(item_id)?;
        let (sound, volume, pitch) =
            if transform.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
                (
                    EXPERIENCE_ORB_PICKUP_SOUND_EVENT,
                    0.1,
                    (next_float() - next_float()) * 0.35 + 0.9,
                )
            } else {
                (
                    ITEM_PICKUP_SOUND_EVENT,
                    0.2,
                    (next_float() - next_float()) * 1.4 + 2.0,
                )
            };
        Some(SoundEventState {
            sound: direct_sound_holder(sound),
            source: SoundSource::Players.as_str().to_string(),
            position: ProtocolVec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            volume,
            pitch,
            seed: 0,
            distance_delay: false,
        })
    }

    pub fn game_event_positioned_sound(
        &mut self,
        event: ProtocolGameEvent,
    ) -> Option<SoundEventState> {
        let pose = self.local_player_pose()?;
        let feet_position = pose.position;
        let eye_position = ProtocolVec3d {
            x: feet_position.x,
            y: feet_position.y + pose.eye_height(),
            z: feet_position.z,
        };
        let (sound, source, position, volume, pitch) = match event.event_id {
            VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID => (
                ARROW_HIT_PLAYER_SOUND_EVENT,
                SoundSource::Players,
                eye_position,
                0.18,
                0.45,
            ),
            VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID => (
                PUFFER_FISH_STING_SOUND_EVENT,
                SoundSource::Neutral,
                feet_position,
                1.0,
                1.0,
            ),
            VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID
                if rounded_game_event_param(event.param) == 1 =>
            {
                (
                    ELDER_GUARDIAN_CURSE_SOUND_EVENT,
                    SoundSource::Hostile,
                    feet_position,
                    1.0,
                    1.0,
                )
            }
            _ => return None,
        };
        let state = SoundEventState {
            sound: direct_sound_holder(sound),
            source: source.as_str().to_string(),
            position,
            volume,
            pitch,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn client_audio(&self) -> &ClientAudioState {
        &self.client_audio
    }

    /// Draws the next deterministic client-local sound seed, matching vanilla's
    /// `RandomSource.nextLong()` sound-seed derivation
    /// (`Level.soundSeedGenerator` / `ClientLevel.random`) with a fixed local
    /// seed instead of a wall-clock one.
    pub fn next_sound_seed(&mut self) -> i64 {
        self.client_audio.sound_seed_random.next_long()
    }

    /// Applies the ordered client-level random and sound-seed side effects of
    /// an item interaction sound.
    ///
    /// Vanilla flint-and-steel and instruments pass the local player as
    /// `except`, so `ClientLevel.playSeededSound` plays them immediately.
    /// Instrument playback is entity-bound; flint-and-steel is positioned.
    /// Spyglass start/stop and bundle removal call `LocalPlayer.playSound`;
    /// bottle filling passes the local player as `except`, and bucket pickup
    /// calls `LocalPlayer.playSound`, so these are immediate positioned sounds.
    /// Bucket emptying, evaporation, and client-predicted block placement pass
    /// the local player as `except` and stay block-positioned. Bundle
    /// content-drop, fishing rods, bow release, Riptide release, thrown
    /// projectiles, fire charges, lodestone compasses, both potion
    /// mud-conversion sounds, candle extinguish/addition, decorated-pot insert
    /// failure, and tripwire-hook state transitions pass `null`, so their
    /// local calls only advance the random streams. Copper-golem pose and
    /// spawn sounds also pass `null`.
    pub fn apply_local_interaction_sound_action(
        &mut self,
        action: LocalInteractionSoundAction,
        random: &mut LevelEventSoundRandomState,
    ) -> Option<LocalInteractionSoundEffect> {
        let mut state = match action {
            LocalInteractionSoundAction::FlintAndSteelUse { pos } => block_sound_state(
                pos,
                "minecraft:item.flintandsteel.use",
                1.0,
                random.next_float() * 0.4 + 0.8,
                SoundSource::Blocks.as_str(),
            ),
            LocalInteractionSoundAction::InstrumentUse { hand } => {
                let stack = self.local_item_in_hand(hand)?.clone();
                let instrument = self.local_instrument_use_state(&stack)?;
                let entity_id = self.local_player_id()?;
                let sound = sound_event_summary_holder(&instrument.sound_event)?;
                let seed = self.next_sound_seed();
                if self.entities.is_silent(entity_id).unwrap_or(false) {
                    return None;
                }
                let state = SoundEntityEventState {
                    sound,
                    source: SoundSource::Records.as_str().to_string(),
                    entity_id,
                    volume: instrument.range / 16.0,
                    pitch: 1.0,
                    seed,
                    looping: false,
                    linear_attenuation: true,
                    tickable: false,
                };
                self.client_audio.last_sound_entity = Some(state.clone());
                return Some(LocalInteractionSoundEffect::Entity {
                    state,
                    position: self.local_player_pose().map(|pose| pose.position),
                });
            }
            LocalInteractionSoundAction::LoomSelectPattern => {
                return Some(LocalInteractionSoundEffect::Local(self.record_local_sound(
                    ui_sound_state("minecraft:ui.loom.select_pattern", 0.25, 1.0),
                )));
            }
            LocalInteractionSoundAction::StonecutterSelectRecipe => {
                return Some(LocalInteractionSoundEffect::Local(self.record_local_sound(
                    ui_sound_state("minecraft:ui.stonecutter.select_recipe", 0.25, 1.0),
                )));
            }
            LocalInteractionSoundAction::CrafterSlotStateChanged { enabled } => {
                let position = self.local_player_pose()?.position;
                let mut state = local_player_sound_state(
                    position,
                    "minecraft:ui.button.click",
                    0.4,
                    if enabled { 1.0 } else { 0.75 },
                    SoundSource::Players.as_str(),
                );
                state.seed = self.next_sound_seed();
                return Some(LocalInteractionSoundEffect::Positioned(
                    self.record_positioned_sound(state),
                ));
            }
            LocalInteractionSoundAction::BundleRemoveOne => {
                let position = self.local_player_pose()?.position;
                let mut state = local_player_sound_state(
                    position,
                    "minecraft:item.bundle.remove_one",
                    0.8,
                    0.8 + random.next_float() * 0.4,
                    SoundSource::Players.as_str(),
                );
                state.seed = self.next_sound_seed();
                return Some(LocalInteractionSoundEffect::Positioned(
                    self.record_positioned_sound(state),
                ));
            }
            LocalInteractionSoundAction::SuppressedBundleDropContents => {
                let _discarded_drop_pitch = 0.8 + random.next_float() * 0.4;
                let _discarded_drop_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedFishingRodUse { retrieving } => {
                let _suppressed_volume = if retrieving { 1.0 } else { 0.5 };
                let _discarded_pitch = 0.4 / (random.next_float() * 0.4 + 0.8);
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SpyglassUse => local_player_sound_state(
                self.local_player_pose()?.position,
                "minecraft:item.spyglass.use",
                1.0,
                1.0,
                SoundSource::Players.as_str(),
            ),
            LocalInteractionSoundAction::SpyglassStopUsing => local_player_sound_state(
                self.local_player_pose()?.position,
                "minecraft:item.spyglass.stop_using",
                1.0,
                1.0,
                SoundSource::Players.as_str(),
            ),
            LocalInteractionSoundAction::SuppressedBowRelease { power_bits } => {
                let power = f32::from_bits(power_bits);
                let _discarded_pitch = 1.0 / (random.next_float() * 0.4 + 1.2) + power * 0.5;
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedTridentRiptide => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedProjectileThrow => {
                let _discarded_pitch = 0.4 / (random.next_float() * 0.4 + 0.8);
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedFireChargeUse => {
                let _discarded_pitch = (random.next_float() - random.next_float()) * 0.2 + 1.0;
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedLodestoneCompassUse => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedPotionMudConversion => {
                let _discarded_splash_seed = self.next_sound_seed();
                let _discarded_bottle_empty_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::BottleFill => local_player_sound_state(
                self.local_player_pose()?.position,
                "minecraft:item.bottle.fill",
                1.0,
                1.0,
                SoundSource::Neutral.as_str(),
            ),
            LocalInteractionSoundAction::BeehiveBottleFill => local_player_sound_state(
                self.local_player_pose()?.position,
                "minecraft:item.bottle.fill",
                1.0,
                1.0,
                SoundSource::Blocks.as_str(),
            ),
            LocalInteractionSoundAction::BucketFill { sound_resource_id } => {
                local_player_sound_state(
                    self.local_player_pose()?.position,
                    sound_resource_id,
                    1.0,
                    1.0,
                    SoundSource::Players.as_str(),
                )
            }
            LocalInteractionSoundAction::BucketEmpty {
                pos,
                sound_resource_id,
                neutral_source,
            } => block_sound_state(
                pos,
                sound_resource_id,
                1.0,
                1.0,
                if neutral_source {
                    SoundSource::Neutral.as_str()
                } else {
                    SoundSource::Blocks.as_str()
                },
            ),
            LocalInteractionSoundAction::BucketWaterEvaporate { pos } => block_sound_state(
                pos,
                "minecraft:block.fire.extinguish",
                0.5,
                2.6 + (random.next_float() - random.next_float()) * 0.8,
                SoundSource::Blocks.as_str(),
            ),
            LocalInteractionSoundAction::SuppressedCandleExtinguish => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedCakeAddCandle => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedDecoratedPotInsertFail => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedRespawnAnchorCharge => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedCopperGolemBecomeStatue => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedCopperGolemSpawn => {
                let _discarded_seed = self.next_sound_seed();
                return None;
            }
            LocalInteractionSoundAction::SuppressedAnimalEating {
                entity_id,
                repetitions,
            } => {
                let entity_type_id = self.entities.identity(entity_id)?.entity_type_id;
                let policy = vanilla_entity_metadata_eating_sound_policy(entity_type_id)?;
                for _ in 0..repetitions {
                    match policy {
                        VanillaEntityEatingSoundPolicy::None => {}
                        VanillaEntityEatingSoundPolicy::EntityVoicePitch => {
                            self.entities
                                .advance_local_eating_voice_pitch_random(entity_id)?;
                            if !self.entities.is_silent(entity_id)? {
                                let _discarded_seed = self.next_sound_seed();
                            }
                        }
                        VanillaEntityEatingSoundPolicy::EntityFixedPitch => {
                            if !self.entities.is_silent(entity_id)? {
                                let _discarded_seed = self.next_sound_seed();
                            }
                        }
                        VanillaEntityEatingSoundPolicy::LevelFixedPitch => {
                            let _discarded_seed = self.next_sound_seed();
                        }
                        VanillaEntityEatingSoundPolicy::LevelUniformPitch => {
                            let _discarded_pitch = 0.8 + random.next_float() * (1.2 - 0.8);
                            let _discarded_seed = self.next_sound_seed();
                        }
                    }
                }
                return None;
            }
            LocalInteractionSoundAction::SuppressedEquineEating { entity_id } => {
                if !self.entities.is_silent(entity_id)? {
                    self.entities
                        .advance_local_equine_eating_pitch_random(entity_id)?;
                    let _discarded_seed = self.next_sound_seed();
                }
                return None;
            }
            LocalInteractionSoundAction::AllayInteraction {
                entity_id,
                sound_resource_id,
            } => {
                let position = self.entities.transform(entity_id)?.position;
                let state = SoundEntityEventState {
                    sound: direct_sound_holder(sound_resource_id),
                    source: SoundSource::Neutral.as_str().to_string(),
                    entity_id,
                    volume: 2.0,
                    pitch: 1.0,
                    seed: self.next_sound_seed(),
                    looping: false,
                    linear_attenuation: true,
                    tickable: false,
                };
                self.client_audio.last_sound_entity = Some(state.clone());
                return Some(LocalInteractionSoundEffect::Entity {
                    state,
                    position: Some(ProtocolVec3d {
                        x: position.x,
                        y: position.y,
                        z: position.z,
                    }),
                });
            }
            LocalInteractionSoundAction::CreeperIgnition {
                entity_id,
                fire_charge,
            } => {
                let position = self.entities.transform(entity_id)?.position;
                let pitch = self
                    .entities
                    .next_local_interaction_random_float(entity_id)?
                    * 0.4
                    + 0.8;
                let seed = self.next_sound_seed();
                return Some(LocalInteractionSoundEffect::Positioned(
                    self.record_positioned_sound(SoundEventState {
                        sound: direct_sound_holder(if fire_charge {
                            "minecraft:item.firecharge.use"
                        } else {
                            "minecraft:item.flintandsteel.use"
                        }),
                        source: SoundSource::Hostile.as_str().to_string(),
                        position: ProtocolVec3d {
                            x: position.x,
                            y: position.y,
                            z: position.z,
                        },
                        volume: 1.0,
                        pitch,
                        seed,
                        distance_delay: false,
                    }),
                ));
            }
            LocalInteractionSoundAction::LocalPlayerMobInteraction { sound_resource_id } => {
                local_player_sound_state(
                    self.local_player_pose()?.position,
                    sound_resource_id,
                    1.0,
                    1.0,
                    SoundSource::Players.as_str(),
                )
            }
            LocalInteractionSoundAction::SuppressedMobInteraction {
                entity_id,
                silent,
                random_float_count,
                random_before_silent_check,
            } => {
                if random_before_silent_check || !silent {
                    for _ in 0..random_float_count {
                        let _discarded_random = self
                            .entities
                            .next_local_interaction_random_float(entity_id)?;
                    }
                }
                if !silent {
                    let _discarded_seed = self.next_sound_seed();
                }
                return None;
            }
            LocalInteractionSoundAction::BlockPlace {
                pos,
                sound_resource_id,
                volume_bits,
                pitch_bits,
            } => block_sound_state(
                pos,
                sound_resource_id,
                f32::from_bits(volume_bits),
                f32::from_bits(pitch_bits),
                SoundSource::Blocks.as_str(),
            ),
            LocalInteractionSoundAction::BlockPlaceFromProfile { pos, block_type_id } => {
                let block_name =
                    bbb_protocol::block_types::vanilla_block_type(block_type_id)?.resource_id;
                let profile = self.items.default_block_sound_profiles.get(block_name)?;
                block_sound_state(
                    pos,
                    &profile.place_sound,
                    (profile.volume + 1.0) / 2.0,
                    profile.pitch * 0.8,
                    SoundSource::Blocks.as_str(),
                )
            }
            LocalInteractionSoundAction::TripWireHookBlockPlace {
                pos,
                block_type_id,
                suppressed_state_sound_count,
                suppressed_detach_pitch,
            } => {
                for _ in 0..suppressed_state_sound_count {
                    if suppressed_detach_pitch {
                        let _discarded_detach_pitch = 1.2 / (random.next_float() * 0.2 + 0.9);
                    }
                    let _discarded_state_sound_seed = self.next_sound_seed();
                }
                let block_name =
                    bbb_protocol::block_types::vanilla_block_type(block_type_id)?.resource_id;
                let profile = self.items.default_block_sound_profiles.get(block_name)?;
                block_sound_state(
                    pos,
                    &profile.place_sound,
                    (profile.volume + 1.0) / 2.0,
                    profile.pitch * 0.8,
                    SoundSource::Blocks.as_str(),
                )
            }
        };
        state.seed = self.next_sound_seed();
        Some(LocalInteractionSoundEffect::Positioned(
            self.record_positioned_sound(state),
        ))
    }

    /// Resolves a narrow entity-movement sound into the same positioned sound
    /// state used by packet and client-local audio, consuming the canonical
    /// client sound seed at the movement effect's ordered position.
    pub(crate) fn record_entity_client_movement_sound(
        &mut self,
        state: EntityClientMovementSoundState,
    ) -> SoundEventState {
        let (sound, source, volume, pitch) = match state.kind {
            EntityClientMovementSoundKind::BoatPaddleLeft
            | EntityClientMovementSoundKind::BoatPaddleRight => {
                unreachable!("boat paddle sounds require post-move world resolution")
            }
            EntityClientMovementSoundKind::BlazeBurn { volume, pitch } => {
                (BLAZE_BURN_SOUND_EVENT, "hostile", volume, pitch)
            }
            EntityClientMovementSoundKind::WardenHeartbeat { pitch } => {
                (WARDEN_HEARTBEAT_SOUND_EVENT, "hostile", 5.0, pitch)
            }
            EntityClientMovementSoundKind::EnderDragonFlap { pitch } => {
                (ENDER_DRAGON_FLAP_SOUND_EVENT, "hostile", 5.0, pitch)
            }
            EntityClientMovementSoundKind::EnderDragonGrowl { pitch } => {
                (ENDER_DRAGON_GROWL_SOUND_EVENT, "hostile", 2.5, pitch)
            }
            EntityClientMovementSoundKind::GuardianFlop { elder } => (
                if elder {
                    ELDER_GUARDIAN_FLOP_SOUND_EVENT
                } else {
                    GUARDIAN_FLOP_SOUND_EVENT
                },
                "hostile",
                1.0,
                1.0,
            ),
            EntityClientMovementSoundKind::SlimeSquish {
                magma_cube,
                tiny,
                volume,
                pitch,
            } => (
                match (magma_cube, tiny) {
                    (false, false) => SLIME_SQUISH_SOUND_EVENT,
                    (false, true) => SLIME_SQUISH_SMALL_SOUND_EVENT,
                    (true, false) => MAGMA_CUBE_SQUISH_SOUND_EVENT,
                    (true, true) => MAGMA_CUBE_SQUISH_SMALL_SOUND_EVENT,
                },
                "hostile",
                volume,
                pitch,
            ),
            EntityClientMovementSoundKind::CamelDashReady { husk } => (
                if husk {
                    CAMEL_HUSK_DASH_READY_SOUND_EVENT
                } else {
                    CAMEL_DASH_READY_SOUND_EVENT
                },
                "neutral",
                1.0,
                1.0,
            ),
            EntityClientMovementSoundKind::CamelStand { husk, pitch } => (
                if husk {
                    CAMEL_HUSK_STAND_SOUND_EVENT
                } else {
                    CAMEL_STAND_SOUND_EVENT
                },
                "neutral",
                0.8,
                pitch,
            ),
            EntityClientMovementSoundKind::NautilusDashReady {
                zombie,
                underwater,
                pitch,
            } => (
                match (zombie, underwater) {
                    (false, true) => NAUTILUS_DASH_READY_SOUND_EVENT,
                    (false, false) => NAUTILUS_DASH_READY_LAND_SOUND_EVENT,
                    (true, true) => ZOMBIE_NAUTILUS_DASH_READY_SOUND_EVENT,
                    (true, false) => ZOMBIE_NAUTILUS_DASH_READY_LAND_SOUND_EVENT,
                },
                "neutral",
                1.0,
                pitch,
            ),
            EntityClientMovementSoundKind::BreezeDeflect => {
                (BREEZE_DEFLECT_SOUND_EVENT, "hostile", 1.0, 1.0)
            }
            EntityClientMovementSoundKind::FireworkRocketLaunch => {
                (FIREWORK_ROCKET_LAUNCH_SOUND_EVENT, "ambient", 3.0, 1.0)
            }
            EntityClientMovementSoundKind::PlayerPowderSnowFall { big } => (
                if big {
                    PLAYER_BIG_FALL_SOUND_EVENT
                } else {
                    PLAYER_SMALL_FALL_SOUND_EVENT
                },
                "player",
                1.0,
                1.0,
            ),
            EntityClientMovementSoundKind::TridentReturn => {
                (TRIDENT_RETURN_SOUND_EVENT, "neutral", 10.0, 1.0)
            }
        };
        let event = SoundEventState {
            sound: direct_sound_holder(sound),
            source: source.to_string(),
            position: ProtocolVec3d {
                x: state.position.x,
                y: state.position.y,
                z: state.position.z,
            },
            volume,
            pitch,
            seed: self.next_sound_seed(),
            distance_delay: false,
        };
        self.record_positioned_sound(event)
    }

    /// Resolves movement sounds whose exact client-visible result depends on
    /// post-move world state. Ordinary movement sounds keep the narrower
    /// infallible path above.
    pub fn record_entity_client_movement_sound_if_audible(
        &mut self,
        state: EntityClientMovementSoundState,
    ) -> Option<SoundEventState> {
        match state.kind {
            EntityClientMovementSoundKind::BoatPaddleLeft => {
                self.record_boat_paddle_sound(state.entity_id, 0)
            }
            EntityClientMovementSoundKind::BoatPaddleRight => {
                self.record_boat_paddle_sound(state.entity_id, 1)
            }
            _ => Some(self.record_entity_client_movement_sound(state)),
        }
    }

    fn record_boat_paddle_sound(&mut self, entity_id: i32, side: usize) -> Option<SoundEventState> {
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        if !crate::entities::is_vanilla_boat_type(entity_type_id)
            || self.entities.is_silent(entity_id).unwrap_or(false)
        {
            return None;
        }
        let transform = self.entities.transform(entity_id)?;
        let bounds = self.entities.pick_bounds(entity_id)?;
        let relative_min = bounds.min.map(f64::from);
        let relative_max = bounds.max.map(f64::from);
        let position = [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ];
        let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
        let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
        let sound = if crate::fluid::boat_water_status(self, aabb_min, aabb_max)
            .0
            .is_some()
        {
            "minecraft:entity.boat.paddle_water"
        } else if self.boat_ground_friction(position, relative_min, relative_max) > 0.0 {
            "minecraft:entity.boat.paddle_land"
        } else {
            return None;
        };
        let view = crate::entities::calculate_view_vector(transform.x_rot, transform.y_rot);
        let (dx, dz) = if side == 1 {
            (-view.z, view.x)
        } else {
            (view.z, -view.x)
        };
        let identity = self.entities.identity(entity_id)?;
        let age_ticks = self
            .entities
            .entity_age_ticks(entity_id)
            .unwrap_or_default();
        let salt = 0x626f_6174_5f70_646c_u64
            ^ u64::from(age_ticks)
            ^ (side as u64).wrapping_mul(0x9e37_79b9);
        let pitch = 0.8 + crate::entities::spawn_random_unit_float(identity.uuid, salt) * 0.4;
        let event = SoundEventState {
            sound: direct_sound_holder(sound),
            source: "neutral".to_string(),
            position: ProtocolVec3d {
                x: transform.position.x + dx,
                y: transform.position.y,
                z: transform.position.z + dz,
            },
            volume: 1.0,
            pitch,
            seed: self.next_sound_seed(),
            distance_delay: false,
        };
        Some(self.record_positioned_sound(event))
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
                    == Some(VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID) =>
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

    pub(crate) fn record_explosion_sound(
        &mut self,
        explosion: &crate::ExplosionEventState,
        random: &mut LevelEventSoundRandomState,
    ) -> SoundEventState {
        let pitch = (1.0 + (random.next_float() - random.next_float()) * 0.2) * 0.7;
        let state = SoundEventState {
            sound: sound_holder_state(explosion.explosion_sound.clone()),
            source: SoundSource::Blocks.as_str().to_string(),
            position: explosion.center,
            volume: 4.0,
            pitch,
            seed: random.next_long(),
            distance_delay: false,
        };
        self.record_positioned_sound(state)
    }

    fn level_event_block_break_sound_state(
        &self,
        event: ProtocolLevelEvent,
    ) -> Option<SoundEventState> {
        if event.event_type != BLOCK_BREAK_LEVEL_EVENT {
            return None;
        }
        let block_state = self.registries.block_state(event.data)?;
        if block_name_is_air(&block_state.name) {
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

fn horizontal_speed(delta: crate::EntityVec3) -> f32 {
    delta.x.hypot(delta.z) as f32
}

fn sound_lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
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

fn local_player_sound_state(
    position: ProtocolVec3d,
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
        position,
        volume,
        pitch,
        seed: 0,
        distance_delay: false,
    }
}

fn ui_sound_state(sound: &str, volume: f32, pitch: f32) -> LocalSoundEventState {
    LocalSoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(sound.to_string()),
            fixed_range: None,
        },
        source: SoundSource::Ui.as_str().to_string(),
        volume,
        pitch,
        // `SimpleSoundInstance.forUI` owns an unseeded random source rather
        // than advancing ClientLevel's shared sound seed.
        seed: 0,
    }
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
    match vanilla_entity_metadata_type_sound_source_kind(entity_type_id) {
        Some(VanillaEntitySoundSourceKind::Hostile) => SoundSource::Hostile,
        Some(VanillaEntitySoundSourceKind::Ambient) => SoundSource::Ambient,
        Some(VanillaEntitySoundSourceKind::Weather) => SoundSource::Weather,
        Some(VanillaEntitySoundSourceKind::Players) => SoundSource::Players,
        Some(VanillaEntitySoundSourceKind::Neutral | VanillaEntitySoundSourceKind::Dynamic)
        | None => SoundSource::Neutral,
    }
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
    advance_vault_activation_particle_randoms_with_connections(random, 0);
}

pub fn advance_vault_activation_particle_randoms_with_connections(
    random: &mut LevelEventSoundRandomState,
    connected_player_targets: usize,
) {
    for _ in 0..connected_player_targets {
        let particle_count = random.next_int_bound(4) + 2;
        for _ in 0..particle_count {
            let _ = random.next_float();
            let _ = random.next_float();
            let _ = random.next_float();
        }
    }
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

fn player_hurt_sound_event(effects: DamageTypeEffects) -> &'static str {
    match effects {
        DamageTypeEffects::Hurt | DamageTypeEffects::Thorns => PLAYER_HURT_SOUND_EVENT,
        DamageTypeEffects::Drowning => PLAYER_HURT_DROWN_SOUND_EVENT,
        DamageTypeEffects::Burning => PLAYER_HURT_ON_FIRE_SOUND_EVENT,
        DamageTypeEffects::Poking => PLAYER_HURT_SWEET_BERRY_BUSH_SOUND_EVENT,
        DamageTypeEffects::Freezing => PLAYER_HURT_FREEZE_SOUND_EVENT,
    }
}

fn sound_event_summary_holder(sound: &SoundEventSummary) -> Option<SoundHolderState> {
    if let Some(registry_id) = sound.registry_id {
        return Some(SoundHolderState {
            kind: "reference".to_string(),
            registry_id: Some(registry_id),
            location: None,
            fixed_range: None,
        });
    }
    Some(SoundHolderState {
        kind: "direct".to_string(),
        registry_id: None,
        location: Some(sound.sound_id.clone()?),
        fixed_range: sound.fixed_range_bits.map(f32::from_bits),
    })
}

fn registry_sound_holder(sound: RegistrySoundEvent) -> SoundHolderState {
    match sound {
        RegistrySoundEvent::Named { resource_id } => direct_sound_holder(&resource_id),
        RegistrySoundEvent::Direct {
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
    use crate::entities::vanilla_entity_metadata_entity_accessors;
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_ARMADILLO_STATE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_SOUND_VARIANT_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_PIG_SOUND_VARIANT_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_WEATHERING_COPPER_STATE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_WOLF_SOUND_VARIANT_ID,
    };
    use bbb_protocol::packets::{
        AddEntity, BlockPos as ProtocolBlockPos, EntityDataValue, EntityDataValueKind,
        EquipmentSlotUpdate, ItemStackSummary, LevelEvent, PlayTime, RegistryTags, RemoveEntities,
        SetEntityData, SetEquipment, SoundSource, StopSound, TagNetworkPayload,
        UpdateEnabledFeatures, UpdateTags,
    };
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn entity_silent_data(value: bool) -> EntityDataValue {
        let accessors = vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors must resolve");
        assert!(!accessors.silent_default);
        EntityDataValue {
            data_id: accessors.silent_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(value),
        }
    }

    #[test]
    fn sound_seed_random_is_deterministic_and_non_repeating() {
        // Two identically-initialized instances produce the same seed sequence
        // (determinism), and a single instance does not repeat within a window
        // (the uniqueness the old wall-clock `unique ^ nanos` seed provided).
        let mut a = SoundSeedRandomState::default();
        let mut b = SoundSeedRandomState::default();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1024 {
            let seed_a = a.next_long();
            let seed_b = b.next_long();
            assert_eq!(seed_a, seed_b, "identical seeds must yield identical draws");
            assert!(seen.insert(seed_a), "sound seed sequence must not repeat");
        }

        // `WorldStore` threads the same deterministic chain: fresh stores draw
        // matching seeds, and successive draws advance.
        let mut world_one = WorldStore::new();
        let mut world_two = WorldStore::new();
        for _ in 0..8 {
            assert_eq!(world_one.next_sound_seed(), world_two.next_sound_seed());
        }
        let first = world_one.next_sound_seed();
        let second = world_one.next_sound_seed();
        assert_ne!(first, second);
    }

    #[test]
    fn minecart_and_bee_tickable_sounds_match_client_instance_math_and_generation_isolation() {
        let mut store = WorldStore::new();
        let mut minecart = protocol_add_entity_of_type(70, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID);
        minecart.delta_movement = ProtocolVec3d {
            x: 0.3,
            y: 0.0,
            z: 0.4,
        };
        store.apply_add_entity(minecart);
        let initial = store
            .post_add_entity_sound_for_entity(70, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID)
            .unwrap();
        assert_eq!(
            initial.sound.location.as_deref(),
            Some(MINECART_RIDING_SOUND_EVENT)
        );
        assert_close(initial.volume, 0.0);
        assert_close(initial.pitch, 0.0);

        let generation_a = store.publication_snapshot();
        store.advance_tickable_entity_sounds(4, 4);
        let minecart = store.tickable_entity_sounds();
        assert_eq!(minecart.len(), 1);
        assert_close(minecart[0].event.volume, 0.35);
        assert_close(minecart[0].event.pitch, 0.01);
        assert_close(generation_a.tickable_entity_sounds()[0].event.pitch, 0.0);
        store.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec![
                "minecraft:vanilla".to_string(),
                "minecraft:minecart_improvements".to_string(),
            ],
        });
        store.advance_tickable_entity_sounds(1, 1);
        assert_close(store.tickable_entity_sounds()[0].event.volume, 0.0);
        assert_close(store.tickable_entity_sounds()[0].event.pitch, 0.0);
        store.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec!["minecraft:vanilla".to_string()],
        });
        store.advance_tickable_entity_sounds(1, 1);
        assert_close(store.tickable_entity_sounds()[0].event.volume, 0.35);
        assert_close(store.tickable_entity_sounds()[0].event.pitch, 0.0025);
        store.advance_tickable_entity_sounds(1, 0);
        assert_close(store.tickable_entity_sounds()[0].event.volume, 0.0);
        assert_close(store.tickable_entity_sounds()[0].event.pitch, 0.0);

        let mut bee = protocol_add_entity_of_type(71, VANILLA_ENTITY_TYPE_BEE_ID);
        bee.delta_movement = ProtocolVec3d {
            x: 0.8,
            y: 0.0,
            z: 0.0,
        };
        store.apply_add_entity(bee);
        let bee_accessors = crate::entities::vanilla_entity_metadata_bee_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_BEE_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 71,
            values: vec![
                EntityDataValue {
                    data_id: bee_accessors.anger_end_time_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                    value: EntityDataValueKind::Long(20),
                },
                baby_entity_data(VANILLA_ENTITY_TYPE_BEE_ID),
            ],
        }));
        store.apply_world_time(PlayTime {
            game_time: 10,
            clock_updates: Vec::new(),
        });
        store
            .post_add_entity_sound_for_entity(71, VANILLA_ENTITY_TYPE_BEE_ID)
            .unwrap();
        let bee = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|state| state.event.entity_id == 71)
            .unwrap();
        assert_eq!(
            bee.event.sound.location.as_deref(),
            Some(BEE_AGGRESSIVE_LOOP_SOUND_EVENT)
        );
        assert_close(bee.event.volume, 0.6);
        assert_close(bee.event.pitch, 1.54);

        store.advance_client_time(10);
        let bee = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|state| state.event.entity_id == 71)
            .unwrap();
        assert_eq!(
            bee.event.sound.location.as_deref(),
            Some(BEE_LOOP_SOUND_EVENT)
        );

        store.apply_add_entity(protocol_add_entity_of_type(
            71,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        ));
        assert!(store
            .tickable_entity_sounds()
            .iter()
            .all(|state| state.event.entity_id != 71));
        assert_eq!(
            store.apply_remove_entities(RemoveEntities {
                entity_ids: vec![70],
            }),
            1
        );
        assert!(store.tickable_entity_sounds().is_empty());
    }

    #[test]
    fn entity_sound_source_uses_generated_class_behavior() {
        for (entity_type_id, expected) in [
            (VANILLA_ENTITY_TYPE_PLAYER_ID, SoundSource::Players),
            (VANILLA_ENTITY_TYPE_ITEM_ID, SoundSource::Ambient),
            (VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID, SoundSource::Ambient),
            (VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, SoundSource::Weather),
            (VANILLA_ENTITY_TYPE_BLAZE_ID, SoundSource::Hostile),
            (VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, SoundSource::Hostile),
            (VANILLA_ENTITY_TYPE_HOGLIN_ID, SoundSource::Hostile),
            (VANILLA_ENTITY_TYPE_PARCHED_ID, SoundSource::Hostile),
            (VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, SoundSource::Neutral),
            // Rabbit is the one per-instance override. This type-only call keeps
            // the existing neutral fallback until a metadata-aware caller owns it.
            (VANILLA_ENTITY_TYPE_RABBIT_ID, SoundSource::Neutral),
        ] {
            assert_eq!(vanilla_entity_sound_source(entity_type_id), expected);
        }
        assert_eq!(vanilla_entity_sound_source(-1), SoundSource::Neutral);
        assert_eq!(vanilla_entity_sound_source(157), SoundSource::Neutral);
    }

    #[test]
    fn player_damage_effects_select_vanilla_hurt_sounds() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_of_type(
            90,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        ));
        let cases = [
            (DamageTypeEffects::Hurt, "minecraft:entity.player.hurt"),
            (DamageTypeEffects::Thorns, "minecraft:entity.player.hurt"),
            (
                DamageTypeEffects::Drowning,
                "minecraft:entity.player.hurt_drown",
            ),
            (
                DamageTypeEffects::Burning,
                "minecraft:entity.player.hurt_on_fire",
            ),
            (
                DamageTypeEffects::Poking,
                "minecraft:entity.player.hurt_sweet_berry_bush",
            ),
            (
                DamageTypeEffects::Freezing,
                "minecraft:entity.player.hurt_freeze",
            ),
        ];
        for (effects, expected_sound) in cases {
            let sound = store
                .living_damage_hurt_sound_for_entity(90, 0, effects, || 0.5)
                .unwrap();
            assert_eq!(sound.sound.location.as_deref(), Some(expected_sound));
            assert_eq!(sound.source, "player");
            assert_eq!(sound.volume, 1.0);
            assert_eq!(sound.pitch, 1.0);
        }
    }

    #[test]
    fn generated_damage_audio_policies_resolve_all_fixed_living_entity_types() {
        let mut store = WorldStore::new();
        let mut living_count = 0;
        for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
            if matches!(
                vanilla_entity_metadata_type_hurt_sound_policy(entry.id),
                Some(VanillaEntityHurtSoundPolicy::NonLiving)
            ) {
                continue;
            }
            living_count += 1;
            let entity_id = 10_000 + entry.id;
            store.apply_add_entity(protocol_add_entity_of_type(entity_id, entry.id));
            let mut samples = [0.75, 0.25].into_iter();
            let sound = store
                .living_damage_hurt_sound_for_entity(entity_id, 0, DamageTypeEffects::Hurt, || {
                    samples.next().unwrap()
                })
                .unwrap_or_else(|| panic!("{} damage policy did not resolve", entry.resource_id));
            assert!(sound.sound.location.is_some(), "{}", entry.resource_id);
            assert_close(sound.pitch, 1.1);

            let expected_volume = match vanilla_entity_metadata_type_sound_volume_policy(entry.id)
                .expect("every fixed type has a generated volume policy")
            {
                VanillaEntitySoundVolumePolicy::Fixed(bits) => f32::from_bits(bits),
                VanillaEntitySoundVolumePolicy::SlimeSize => 0.4,
                VanillaEntitySoundVolumePolicy::HappyGhastAge => 4.0,
                VanillaEntitySoundVolumePolicy::NonLiving => {
                    panic!("{} living row has non-living volume", entry.resource_id)
                }
            };
            assert_close(sound.volume, expected_volume);
        }
        assert_eq!(living_count, 92);
    }

    #[test]
    fn generated_death_audio_policies_resolve_all_living_entity_types() {
        let mut store = WorldStore::new();
        let mut living_count = 0;
        for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
            if matches!(
                vanilla_entity_metadata_type_death_sound_policy(entry.id),
                Some(VanillaEntityDeathSoundPolicy::NonLiving)
            ) {
                continue;
            }
            living_count += 1;
            let entity_id = 20_000 + entry.id;
            store.apply_add_entity(protocol_add_entity_of_type(entity_id, entry.id));
            let mut samples = [0.75, 0.25].into_iter();
            let sound = store
                .living_death_sound_for_entity(entity_id, || samples.next().unwrap())
                .unwrap_or_else(|| panic!("{} death policy did not resolve", entry.resource_id));
            assert!(sound.sound.location.is_some(), "{}", entry.resource_id);
            assert_close(sound.pitch, 1.1);
            assert!(
                samples.next().is_none(),
                "{} death pitch consumed fewer than two samples",
                entry.resource_id
            );

            let expected_volume = match vanilla_entity_metadata_type_sound_volume_policy(entry.id)
                .expect("every fixed type has a generated volume policy")
            {
                VanillaEntitySoundVolumePolicy::Fixed(bits) => f32::from_bits(bits),
                VanillaEntitySoundVolumePolicy::SlimeSize => 0.4,
                VanillaEntitySoundVolumePolicy::HappyGhastAge => 4.0,
                VanillaEntitySoundVolumePolicy::NonLiving => {
                    panic!("{} living row has non-living volume", entry.resource_id)
                }
            };
            assert_close(sound.volume, expected_volume);
        }
        assert_eq!(living_count, 92);
    }

    #[test]
    fn dynamic_damage_audio_policies_read_canonical_metadata_and_fluid_state() {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        let assert_sound = |store: &mut WorldStore,
                            entity_id: i32,
                            expected: &str,
                            expected_volume: f32| {
            let sound = store
                .living_damage_hurt_sound_for_entity(entity_id, 0, DamageTypeEffects::Hurt, || 0.5)
                .unwrap();
            assert_eq!(sound.sound.location.as_deref(), Some(expected));
            assert_close(sound.volume, expected_volume);
        };
        let assert_death =
            |store: &mut WorldStore, entity_id: i32, expected: &str, expected_volume: f32| {
                let sound = store
                    .living_death_sound_for_entity(entity_id, || 0.5)
                    .unwrap();
                assert_eq!(sound.sound.location.as_deref(), Some(expected));
                assert_close(sound.volume, expected_volume);
            };

        store.apply_add_entity(protocol_add_entity_of_type(
            201,
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        ));
        let (armadillo_data_id, _) =
            vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            )
            .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 201,
            values: vec![EntityDataValue {
                data_id: armadillo_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ARMADILLO_STATE_ID,
                value: EntityDataValueKind::Int(
                    crate::entities::VANILLA_ARMADILLO_STATE_ROLLING_ID,
                ),
            }],
        }));
        assert_sound(
            &mut store,
            201,
            "minecraft:entity.armadillo.hurt_reduced",
            1.0,
        );

        store.apply_add_entity(protocol_add_entity_of_type(
            202,
            VANILLA_ENTITY_TYPE_GOAT_ID,
        ));
        let goat =
            vanilla_entity_metadata_goat_accessors_for_entity_type(VANILLA_ENTITY_TYPE_GOAT_ID)
                .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 202,
            values: vec![EntityDataValue {
                data_id: goat.screaming_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert_sound(&mut store, 202, "minecraft:entity.goat.screaming.hurt", 1.0);
        assert_death(
            &mut store,
            202,
            "minecraft:entity.goat.screaming.death",
            1.0,
        );

        store.apply_add_entity(protocol_add_entity_of_type(
            203,
            VANILLA_ENTITY_TYPE_SLIME_ID,
        ));
        let (slime_data_id, _) = vanilla_entity_metadata_slime_size_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SLIME_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 203,
            values: vec![EntityDataValue {
                data_id: slime_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(4),
            }],
        }));
        assert_sound(&mut store, 203, "minecraft:entity.slime.hurt", 1.6);
        assert_death(&mut store, 203, "minecraft:entity.slime.death", 1.6);

        store.apply_add_entity(protocol_add_entity_of_type(
            204,
            VANILLA_ENTITY_TYPE_SHULKER_ID,
        ));
        let shulker = vanilla_entity_metadata_shulker_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_SHULKER_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 204,
            values: vec![EntityDataValue {
                data_id: shulker.peek_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(1),
            }],
        }));
        assert_sound(&mut store, 204, "minecraft:entity.shulker.hurt", 1.0);

        store.apply_add_entity(protocol_add_entity_of_type(
            205,
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        ));
        let copper = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 205,
            values: vec![EntityDataValue {
                data_id: copper.weather_state_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_WEATHERING_COPPER_STATE_ID,
                value: EntityDataValueKind::Int(VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID),
            }],
        }));
        assert_sound(
            &mut store,
            205,
            "minecraft:entity.copper_golem_oxidized.hurt",
            1.0,
        );
        assert_death(
            &mut store,
            205,
            "minecraft:entity.copper_golem_oxidized.death",
            1.0,
        );

        store.apply_add_entity(protocol_add_entity_of_type(
            206,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        ));
        let home_data_id =
            crate::entities::vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID,
            )
            .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 206,
            values: vec![EntityDataValue {
                data_id: home_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                    x: 1,
                    y: 2,
                    z: 3,
                })),
            }],
        }));
        assert_sound(&mut store, 206, "minecraft:entity.creaking.sway", 1.0);

        store.apply_add_entity(protocol_add_entity_of_type(
            207,
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        ));
        assert_sound(&mut store, 207, "minecraft:entity.happy_ghast.hurt", 4.0);
        assert_death(&mut store, 207, "minecraft:entity.happy_ghast.death", 4.0);
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 207,
            values: vec![baby_entity_data(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID)],
        }));
        assert_sound(&mut store, 207, "minecraft:entity.ghastling.hurt", 1.0);
        assert_death(&mut store, 207, "minecraft:entity.ghastling.death", 1.0);

        store.apply_add_entity(protocol_add_entity_of_type(
            210,
            VANILLA_ENTITY_TYPE_RABBIT_ID,
        ));
        let (rabbit_data_id, _) =
            crate::entities::vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_RABBIT_ID,
            )
            .unwrap();
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 210,
            values: vec![EntityDataValue {
                data_id: rabbit_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(VANILLA_RABBIT_VARIANT_EVIL_ID),
            }],
        }));
        let rabbit_sound = store
            .living_damage_hurt_sound_for_entity(210, 0, DamageTypeEffects::Hurt, || 0.5)
            .unwrap();
        assert_eq!(
            rabbit_sound.sound.location.as_deref(),
            Some("minecraft:entity.rabbit.hurt")
        );
        assert_eq!(rabbit_sound.source, "hostile");
        let rabbit_death = store.living_death_sound_for_entity(210, || 0.5).unwrap();
        assert_eq!(
            rabbit_death.sound.location.as_deref(),
            Some("minecraft:entity.rabbit.death")
        );
        assert_eq!(rabbit_death.source, "hostile");

        store.insert_decoded_chunk(single_block_state_chunk(vanilla_block_state_id(
            "minecraft:water",
            [("level", "0")],
        )));
        for (entity_id, entity_type_id, expected_hurt, expected_death) in [
            (
                208,
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                "minecraft:entity.drowned.hurt_water",
                "minecraft:entity.drowned.death_water",
            ),
            (
                209,
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                "minecraft:entity.zombie_nautilus.hurt",
                "minecraft:entity.zombie_nautilus.death",
            ),
        ] {
            let mut packet = protocol_add_entity_of_type(entity_id, entity_type_id);
            packet.position = ProtocolVec3d {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            };
            store.apply_add_entity(packet);
            assert_sound(&mut store, entity_id, expected_hurt, 1.0);
            assert_death(&mut store, entity_id, expected_death, 1.0);
        }

        let mut nautilus = protocol_add_entity_of_type(211, VANILLA_ENTITY_TYPE_NAUTILUS_ID);
        nautilus.position = ProtocolVec3d {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        };
        store.apply_add_entity(nautilus);
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 211,
            values: vec![baby_entity_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID)],
        }));
        assert_death(&mut store, 211, "minecraft:entity.baby_nautilus.death", 1.0);
    }

    #[test]
    fn wolf_body_armor_overrides_variant_hurt_unless_damage_bypasses_it() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_of_type(91, VANILLA_ENTITY_TYPE_WOLF_ID));
        assert!(store.apply_set_equipment(SetEquipment {
            entity_id: 91,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_WOLF_ARMOR_ID),
                    count: 1,
                    component_patch: Default::default(),
                },
            }],
        }));
        store.record_registry_entries(
            DAMAGE_TYPE_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::stub("minecraft:generic")],
        );

        let armored = store
            .living_damage_hurt_sound_for_entity(91, 0, DamageTypeEffects::Hurt, || 0.5)
            .unwrap();
        assert_eq!(
            armored.sound.location.as_deref(),
            Some("minecraft:item.wolf_armor.damage")
        );
        assert_eq!(armored.volume, 0.4);

        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: DAMAGE_TYPE_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:bypasses_wolf_armor".to_string(),
                    entries: vec![0],
                }],
            }],
        });
        let bypassed = store
            .living_damage_hurt_sound_for_entity(91, 0, DamageTypeEffects::Hurt, || 0.5)
            .unwrap();
        assert_eq!(
            bypassed.sound.location.as_deref(),
            Some("minecraft:entity.wolf.hurt")
        );
    }

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
    fn elder_guardian_game_event_sound_uses_listener_rounded_param() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(crate::LocalPlayerPoseState::default());

        for (param, expected) in [
            (0.499, false),
            (0.5, true),
            (1.499, true),
            (1.5, false),
            (f32::NAN, false),
        ] {
            let sound = store.game_event_positioned_sound(ProtocolGameEvent {
                event_id: VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
                param,
            });

            assert_eq!(sound.is_some(), expected, "param={param:?}");
        }
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
            looping: false,
            linear_attenuation: true,
            tickable: false,
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
                values: vec![entity_silent_data(true)],
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
    fn built_in_animal_sound_variants_drive_adult_and_baby_consumed_sounds() {
        let mut store = WorldStore::new();
        let mut entity_ids = 1_000..;
        macro_rules! assert_family {
            ($identity:ty, $entity_type_id:expr, $accessors:path, $serializer_id:expr, $serializer:expr, $volume:expr) => {
                let accessors = $accessors($entity_type_id).expect("sound-variant accessors");
                for (registry_id, identity) in <$identity>::builtin_variants() {
                    for baby in [false, true] {
                        let id = entity_ids.next().unwrap();
                        store.apply_add_entity(protocol_add_entity_of_type(id, $entity_type_id));
                        let mut values = vec![EntityDataValue {
                            data_id: accessors.sound_variant_data_id,
                            serializer_id: $serializer_id,
                            value: EntityDataValueKind::RegistryId {
                                serializer: $serializer,
                                value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(
                                    registry_id,
                                ),
                            },
                        }];
                        if baby {
                            values.push(baby_entity_data($entity_type_id));
                        }
                        assert!(store.apply_set_entity_data(SetEntityData { id, values }));

                        let mut hurt_samples = [0.75, 0.25].into_iter();
                        let hurt = store
                            .living_damage_hurt_sound_for_entity(
                                id,
                                0,
                                DamageTypeEffects::Hurt,
                                || hurt_samples.next().unwrap(),
                            )
                            .unwrap();
                        assert_eq!(
                            hurt.sound.location.as_deref(),
                            Some(identity.builtin_hurt_sound_event(baby))
                        );
                        assert_close(hurt.volume, $volume);
                        assert_close(hurt.pitch, 1.1);

                        let mut samples = [0.75, 0.25].into_iter();
                        let sound = store
                            .living_death_sound_for_entity(id, || samples.next().unwrap())
                            .unwrap();
                        assert_eq!(
                            sound.sound.location.as_deref(),
                            Some(identity.builtin_death_sound_event(baby))
                        );
                        assert_close(sound.volume, $volume);
                        assert_close(sound.pitch, 1.1);
                    }
                }
            };
        }

        assert_family!(
            CatSoundVariantIdentity,
            VANILLA_ENTITY_TYPE_CAT_ID,
            vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::CatSoundVariant,
            1.0
        );
        assert_family!(
            ChickenSoundVariantIdentity,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::ChickenSoundVariant,
            1.0
        );
        assert_family!(
            CowSoundVariantIdentity,
            VANILLA_ENTITY_TYPE_COW_ID,
            vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::CowSoundVariant,
            0.4
        );
        assert_family!(
            PigSoundVariantIdentity,
            VANILLA_ENTITY_TYPE_PIG_ID,
            vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_PIG_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::PigSoundVariant,
            1.0
        );
        assert_family!(
            WolfSoundVariantIdentity,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_WOLF_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::WolfSoundVariant,
            0.4
        );
    }

    #[test]
    fn animal_sound_variant_does_not_reinterpret_metadata_across_registry_sessions() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_of_type(
            4_200,
            VANILLA_ENTITY_TYPE_CAT_ID,
        ));
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 4_200,
            values: vec![cat_sound_variant_entity_data(0)],
        }));
        let mut generation_a = store.clone();

        let mut samples = [0.75, 0.25].into_iter();
        assert!(generation_a
            .living_damage_hurt_sound_for_entity(4_200, 0, DamageTypeEffects::Hurt, || samples
                .next()
                .unwrap())
            .is_some());

        store.reset_registry_session();
        let mut stale_samples = [0.75, 0.25].into_iter();
        assert!(store
            .living_damage_hurt_sound_for_entity(4_200, 0, DamageTypeEffects::Hurt, || {
                stale_samples.next().unwrap()
            })
            .is_none());

        let mut retained_samples = [0.75, 0.25].into_iter();
        assert!(generation_a
            .living_damage_hurt_sound_for_entity(4_200, 0, DamageTypeEffects::Hurt, || {
                retained_samples.next().unwrap()
            })
            .is_some());
    }

    #[test]
    fn synchronized_sound_variant_registry_uses_server_order_and_typed_payload() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            CAT_SOUND_VARIANT_REGISTRY,
            0,
            vec![
                crate::RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:custom",
                    cat_sound_variant_json(),
                ),
                crate::RegistryPacketEntry::stub("minecraft:classic"),
            ],
        );
        for (id, baby) in [(2_000, false), (2_001, true)] {
            store.apply_add_entity(protocol_add_entity_of_type(id, VANILLA_ENTITY_TYPE_CAT_ID));
            let mut values = vec![cat_sound_variant_entity_data(0)];
            if baby {
                values.push(baby_entity_data(VANILLA_ENTITY_TYPE_CAT_ID));
            }
            assert!(store.apply_set_entity_data(SetEntityData { id, values }));
            let hurt = store
                .living_damage_hurt_sound_for_entity(id, 0, DamageTypeEffects::Hurt, || 0.5)
                .unwrap();
            assert_eq!(
                hurt.sound.location.as_deref(),
                Some(if baby {
                    "example:entity.cat.baby_hurt"
                } else {
                    "example:entity.cat.adult_hurt"
                })
            );
            assert_eq!(hurt.sound.fixed_range, baby.then_some(12.0));
            let sound = store.living_death_sound_for_entity(id, || 0.5).unwrap();
            assert_eq!(
                sound.sound.location.as_deref(),
                Some(if baby {
                    "example:entity.cat.baby_death"
                } else {
                    "example:entity.cat.adult_death"
                })
            );
            assert_eq!(sound.sound.fixed_range, baby.then_some(24.0));
        }
    }

    #[test]
    fn synchronized_sound_variant_stubs_use_resource_fallback_in_server_order() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            CAT_SOUND_VARIANT_REGISTRY,
            0,
            vec![
                crate::RegistryPacketEntry::stub("minecraft:royal"),
                crate::RegistryPacketEntry::stub("minecraft:classic"),
            ],
        );
        store.apply_add_entity(protocol_add_entity_of_type(
            2_010,
            VANILLA_ENTITY_TYPE_CAT_ID,
        ));
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 2_010,
            values: vec![cat_sound_variant_entity_data(0)],
        }));
        let royal = store.living_death_sound_for_entity(2_010, || 0.5).unwrap();
        assert_eq!(
            royal.sound.location.as_deref(),
            Some("minecraft:entity.cat_royal.death")
        );

        store.apply_add_entity(protocol_add_entity_of_type(
            2_011,
            VANILLA_ENTITY_TYPE_CAT_ID,
        ));
        let default = store.living_death_sound_for_entity(2_011, || 0.5).unwrap();
        assert_eq!(
            default.sound.location.as_deref(),
            Some("minecraft:entity.cat.death")
        );
    }

    #[test]
    fn malformed_synchronized_sound_variant_payload_fails_closed() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            CAT_SOUND_VARIANT_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::with_raw_data(
                "example:malformed",
                vec![10, 0],
            )],
        );
        store.apply_add_entity(protocol_add_entity_of_type(
            2_020,
            VANILLA_ENTITY_TYPE_CAT_ID,
        ));
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 2_020,
            values: vec![cat_sound_variant_entity_data(0)],
        }));

        assert!(store
            .living_death_sound_for_entity(2_020, || {
                panic!("malformed synchronized SoundSet must not sample pitch")
            })
            .is_none());
    }

    #[test]
    fn level_event_2001_maps_block_state_id_to_vanilla_break_sound() {
        let mut store = WorldStore::new();
        store.set_default_block_sound_profiles(BTreeMap::from([(
            "minecraft:grass_block".to_string(),
            WorldBlockSoundProfile {
                break_sound: "minecraft:block.grass.break".to_string(),
                place_sound: "minecraft:block.grass.place".to_string(),
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
    fn entity_client_movement_sounds_use_canonical_seed_and_vanilla_categories() {
        let mut store = WorldStore::new();
        let firework = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
            entity_id: 7,
            kind: EntityClientMovementSoundKind::FireworkRocketLaunch,
            position: crate::EntityVec3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        });
        let blaze = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
            entity_id: 10,
            kind: EntityClientMovementSoundKind::BlazeBurn {
                volume: 1.75,
                pitch: 0.65,
            },
            position: crate::EntityVec3 {
                x: 8.5,
                y: 9.5,
                z: 10.5,
            },
        });
        let guardian = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
            entity_id: 11,
            kind: EntityClientMovementSoundKind::GuardianFlop { elder: false },
            position: crate::EntityVec3 {
                x: 11.0,
                y: 12.0,
                z: 13.0,
            },
        });
        let elder_guardian =
            store.record_entity_client_movement_sound(EntityClientMovementSoundState {
                entity_id: 12,
                kind: EntityClientMovementSoundKind::GuardianFlop { elder: true },
                position: crate::EntityVec3 {
                    x: 14.0,
                    y: 15.0,
                    z: 16.0,
                },
            });
        let camel_ready =
            store.record_entity_client_movement_sound(EntityClientMovementSoundState {
                entity_id: 13,
                kind: EntityClientMovementSoundKind::CamelDashReady { husk: false },
                position: crate::EntityVec3::default(),
            });
        let camel_husk_stand =
            store.record_entity_client_movement_sound(EntityClientMovementSoundState {
                entity_id: 14,
                kind: EntityClientMovementSoundKind::CamelStand {
                    husk: true,
                    pitch: 1.125,
                },
                position: crate::EntityVec3::default(),
            });
        let nautilus_land =
            store.record_entity_client_movement_sound(EntityClientMovementSoundState {
                entity_id: 15,
                kind: EntityClientMovementSoundKind::NautilusDashReady {
                    zombie: false,
                    underwater: false,
                    pitch: 0.925,
                },
                position: crate::EntityVec3::default(),
            });
        let zombie_nautilus_underwater =
            store.record_entity_client_movement_sound(EntityClientMovementSoundState {
                entity_id: 16,
                kind: EntityClientMovementSoundKind::NautilusDashReady {
                    zombie: true,
                    underwater: true,
                    pitch: 1.075,
                },
                position: crate::EntityVec3::default(),
            });
        let breeze = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
            entity_id: 9,
            kind: EntityClientMovementSoundKind::BreezeDeflect,
            position: crate::EntityVec3 {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
        });
        let trident = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
            entity_id: 8,
            kind: EntityClientMovementSoundKind::TridentReturn,
            position: crate::EntityVec3 {
                x: -1.0,
                y: 4.0,
                z: 0.5,
            },
        });

        assert_eq!(
            firework.sound.location.as_deref(),
            Some("minecraft:entity.firework_rocket.launch")
        );
        assert_eq!(firework.source, "ambient");
        assert_eq!(firework.volume, 3.0);
        assert_eq!(
            blaze.sound.location.as_deref(),
            Some("minecraft:entity.blaze.burn")
        );
        assert_eq!(blaze.source, "hostile");
        assert_eq!(blaze.volume, 1.75);
        assert_eq!(blaze.pitch, 0.65);
        assert_eq!(
            blaze.position,
            ProtocolVec3d {
                x: 8.5,
                y: 9.5,
                z: 10.5
            }
        );
        assert_eq!(
            guardian.sound.location.as_deref(),
            Some("minecraft:entity.guardian.flop")
        );
        assert_eq!(guardian.source, "hostile");
        assert_eq!(guardian.volume, 1.0);
        assert_eq!(guardian.pitch, 1.0);
        assert_eq!(
            elder_guardian.sound.location.as_deref(),
            Some("minecraft:entity.elder_guardian.flop")
        );
        assert_eq!(elder_guardian.source, "hostile");
        assert_eq!(
            camel_ready.sound.location.as_deref(),
            Some("minecraft:entity.camel.dash_ready")
        );
        assert_eq!(camel_ready.source, "neutral");
        assert_eq!(camel_ready.volume, 1.0);
        assert_eq!(camel_ready.pitch, 1.0);
        assert_eq!(
            camel_husk_stand.sound.location.as_deref(),
            Some("minecraft:entity.camel_husk.stand")
        );
        assert_eq!(camel_husk_stand.source, "neutral");
        assert_eq!(camel_husk_stand.volume, 0.8);
        assert_eq!(camel_husk_stand.pitch, 1.125);
        assert_eq!(
            nautilus_land.sound.location.as_deref(),
            Some("minecraft:entity.nautilus.dash_ready_land")
        );
        assert_eq!(nautilus_land.source, "neutral");
        assert_eq!(nautilus_land.volume, 1.0);
        assert_eq!(nautilus_land.pitch, 0.925);
        assert_eq!(
            zombie_nautilus_underwater.sound.location.as_deref(),
            Some("minecraft:entity.zombie_nautilus.dash_ready")
        );
        assert_eq!(zombie_nautilus_underwater.source, "neutral");
        assert_eq!(zombie_nautilus_underwater.pitch, 1.075);
        assert_eq!(
            breeze.sound.location.as_deref(),
            Some("minecraft:entity.breeze.deflect")
        );
        assert_eq!(breeze.source, "hostile");
        assert_eq!(breeze.volume, 1.0);
        assert_eq!(
            trident.sound.location.as_deref(),
            Some("minecraft:item.trident.return")
        );
        assert_eq!(trident.source, "neutral");
        assert_eq!(trident.volume, 10.0);
        assert_ne!(firework.seed, blaze.seed);
        assert_ne!(blaze.seed, guardian.seed);
        assert_ne!(guardian.seed, elder_guardian.seed);
        assert_ne!(elder_guardian.seed, camel_ready.seed);
        assert_ne!(camel_ready.seed, camel_husk_stand.seed);
        assert_ne!(camel_husk_stand.seed, breeze.seed);
        assert_ne!(breeze.seed, trident.seed);
        assert_eq!(store.last_sound(), Some(&trident));
        assert_eq!(store.counters().sound_packets, 0);
    }

    #[test]
    fn slime_entity_client_tick_sound_variants_use_vanilla_events() {
        let mut store = WorldStore::new();
        for (magma_cube, tiny, expected_event) in [
            (false, false, "minecraft:entity.slime.squish"),
            (false, true, "minecraft:entity.slime.squish_small"),
            (true, false, "minecraft:entity.magma_cube.squish"),
            (true, true, "minecraft:entity.magma_cube.squish_small"),
        ] {
            let sound = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
                entity_id: 17,
                kind: EntityClientMovementSoundKind::SlimeSquish {
                    magma_cube,
                    tiny,
                    volume: 1.6,
                    pitch: 1.25,
                },
                position: crate::EntityVec3::default(),
            });
            assert_eq!(sound.sound.location.as_deref(), Some(expected_event));
            assert_eq!(sound.source, "hostile");
            assert_eq!(sound.volume, 1.6);
            assert_eq!(sound.pitch, 1.25);
        }
    }

    #[test]
    fn player_powder_snow_fall_sounds_use_player_category_and_size_variants() {
        let mut store = WorldStore::new();
        for (big, expected_event) in [
            (false, "minecraft:entity.player.small_fall"),
            (true, "minecraft:entity.player.big_fall"),
        ] {
            let sound = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
                entity_id: 19,
                kind: EntityClientMovementSoundKind::PlayerPowderSnowFall { big },
                position: crate::EntityVec3 {
                    x: 4.5,
                    y: 65.9,
                    z: -5.5,
                },
            });
            assert_eq!(sound.sound.location.as_deref(), Some(expected_event));
            assert_eq!(sound.source, "player");
            assert_eq!(sound.volume, 1.0);
            assert_eq!(sound.pitch, 1.0);
            assert_eq!(
                sound.position,
                ProtocolVec3d {
                    x: 4.5,
                    y: 65.9,
                    z: -5.5,
                }
            );
        }
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
        protocol_add_entity_of_type(id, 7)
    }

    fn protocol_add_entity_of_type(id: i32, entity_type_id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn single_block_state_chunk(block_state_id: i32) -> crate::ChunkColumn {
        let single_value = |domain, entry_count, global_id| crate::PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: crate::PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        };
        crate::ChunkColumn {
            pos: crate::ChunkPos { x: 0, z: 0 },
            state: crate::ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![crate::ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 4096,
                block_states: single_value(crate::PaletteDomain::BlockStates, 4096, block_state_id),
                biomes: single_value(crate::PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: crate::LightData::default(),
        }
    }

    fn baby_entity_data(entity_type_id: i32) -> EntityDataValue {
        let (data_id, default) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
                .expect("generated nearest baby accessor must resolve");
        assert!(!default, "vanilla AgeableMob baby default must be false");
        EntityDataValue {
            data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }
    }

    fn cat_sound_variant_entity_data(registry_id: i32) -> EntityDataValue {
        let accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_CAT_ID,
        )
        .expect("generated Cat variant metadata accessors");
        EntityDataValue {
            data_id: accessors.sound_variant_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CatSoundVariant,
                value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(registry_id),
            },
        }
    }

    fn cat_sound_variant_json() -> Vec<u8> {
        br#"{
            "adult_sounds": {
                "ambient_sound": "minecraft:entity.cat.ambient",
                "stray_ambient_sound": "minecraft:entity.cat.ambient",
                "hiss_sound": "minecraft:entity.cat.ambient",
                "hurt_sound": {"sound_id": "example:entity.cat.adult_hurt"},
                "death_sound": {"sound_id": "example:entity.cat.adult_death"},
                "eat_sound": "minecraft:entity.cat.ambient",
                "beg_for_food_sound": "minecraft:entity.cat.ambient",
                "purr_sound": "minecraft:entity.cat.ambient",
                "purreow_sound": "minecraft:entity.cat.ambient"
            },
            "baby_sounds": {
                "ambient_sound": "minecraft:entity.baby_cat.ambient",
                "stray_ambient_sound": "minecraft:entity.baby_cat.ambient",
                "hiss_sound": "minecraft:entity.baby_cat.ambient",
                "hurt_sound": {
                    "sound_id": "example:entity.cat.baby_hurt",
                    "range": 12.0
                },
                "death_sound": {
                    "sound_id": "example:entity.cat.baby_death",
                    "range": 24.0
                },
                "eat_sound": "minecraft:entity.baby_cat.ambient",
                "beg_for_food_sound": "minecraft:entity.baby_cat.ambient",
                "purr_sound": "minecraft:entity.baby_cat.ambient",
                "purreow_sound": "minecraft:entity.baby_cat.ambient"
            }
        }"#
        .to_vec()
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
