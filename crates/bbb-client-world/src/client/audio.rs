use std::collections::{BTreeMap, VecDeque};

use bbb_facts::entity_sound_sources::{
    vanilla_entity_sound_source_kind, VanillaEntitySoundSourceKind,
};
use bbb_registry_codec::legacy_random::LegacyRandomSource;
use bbb_registry_codec::random_source::RandomSource;

use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
    block_sound_facts_26_1::{vanilla_block_sound_group_26_1, vanilla_block_sound_type_26_1},
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
use crate::client::local_player_fluid::local_player_bubble_column_drag_down;
use crate::entities::{
    is_vanilla_abstract_nautilus_type, is_vanilla_minecart_type,
    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type,
    vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type,
    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
    vanilla_entity_metadata_goat_accessors_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
    vanilla_entity_metadata_type_death_sound_policy,
    vanilla_entity_metadata_type_hurt_sound_policy,
    vanilla_entity_metadata_type_sound_volume_policy, CatSoundVariantIdentity,
    ChickenSoundVariantIdentity, CowSoundVariantIdentity, EntityClientMovementSoundKind,
    EntityClientMovementSoundState, LivingEntityItemParticleState, PigSoundVariantIdentity,
    VanillaEntityDeathSoundPolicy, VanillaEntityHurtSoundPolicy, VanillaEntitySoundCondition,
    VanillaEntitySoundVariantFamily, VanillaEntitySoundVolumePolicy, WolfSoundVariantIdentity,
    VANILLA_ARMADILLO_STATE_IDLE_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID, VANILLA_RABBIT_VARIANT_EVIL_ID,
};
use crate::level::rounded_game_event_param;
use crate::registries::{DamageEffects, RegistrySoundKind, DAMAGE_TYPE_REGISTRY};
use crate::{block_name_is_air, block_name_is_rail, BlockPos, WorldStore};

mod level_event_sounds;
mod sound_holders;
use level_event_sounds::*;
use sound_holders::*;

mod ambient_and_tickable;
mod living_sounds;
mod movement_and_level_events;

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
const HUD_BUBBLE_POP_SOUND_EVENT: &str = "minecraft:ui.hud.bubble_pop";
const TOTEM_USE_SOUND_EVENT: &str = "minecraft:item.totem.use";
const GUARDIAN_ATTACK_SOUND_EVENT: &str = "minecraft:entity.guardian.attack";
const SNIFFER_DIGGING_SOUND_EVENT: &str = "minecraft:entity.sniffer.digging";
const MINECART_RIDING_SOUND_EVENT: &str = "minecraft:entity.minecart.riding";
const MINECART_INSIDE_SOUND_EVENT: &str = "minecraft:entity.minecart.inside";
const MINECART_INSIDE_UNDERWATER_SOUND_EVENT: &str = "minecraft:entity.minecart.inside.underwater";
const ELYTRA_FLYING_SOUND_EVENT: &str = "minecraft:item.elytra.flying";
const HAPPY_GHAST_RIDING_SOUND_EVENT: &str = "minecraft:entity.happy_ghast.riding";
const NAUTILUS_RIDING_SOUND_EVENT: &str = "minecraft:entity.nautilus.riding";
const BEE_LOOP_SOUND_EVENT: &str = "minecraft:entity.bee.loop";
const BEE_AGGRESSIVE_LOOP_SOUND_EVENT: &str = "minecraft:entity.bee.loop_aggressive";
const EVOKER_FANGS_ATTACK_SOUND_EVENT: &str = "minecraft:entity.evoker_fangs.attack";
const ZOMBIE_VILLAGER_CURE_SOUND_EVENT: &str = "minecraft:entity.zombie_villager.cure";
const ARMADILLO_PEEK_SOUND_EVENT: &str = "minecraft:entity.armadillo.peek";
const ARMOR_STAND_HIT_SOUND_EVENT: &str = "minecraft:entity.armor_stand.hit";
const PLAYER_DEATH_SOUND_EVENT: &str = "minecraft:entity.player.death";
const ITEM_PICKUP_SOUND_EVENT: &str = "minecraft:entity.item.pickup";
const EXPERIENCE_ORB_PICKUP_SOUND_EVENT: &str = "minecraft:entity.experience_orb.pickup";
const BLAZE_BURN_SOUND_EVENT: &str = "minecraft:entity.blaze.burn";
const WARDEN_HEARTBEAT_SOUND_EVENT: &str = "minecraft:entity.warden.heartbeat";
const ENDER_DRAGON_FLAP_SOUND_EVENT: &str = "minecraft:entity.ender_dragon.flap";
const ENDER_DRAGON_GROWL_SOUND_EVENT: &str = "minecraft:entity.ender_dragon.growl";
const LIGHTNING_BOLT_THUNDER_SOUND_EVENT: &str = "minecraft:entity.lightning_bolt.thunder";
const LIGHTNING_BOLT_IMPACT_SOUND_EVENT: &str = "minecraft:entity.lightning_bolt.impact";
const GUARDIAN_FLOP_SOUND_EVENT: &str = "minecraft:entity.guardian.flop";
const ELDER_GUARDIAN_FLOP_SOUND_EVENT: &str = "minecraft:entity.elder_guardian.flop";
const HONEY_BLOCK_SLIDE_SOUND_EVENT: &str = "minecraft:block.honey_block.slide";
const PLAYER_BIG_FALL_SOUND_EVENT: &str = "minecraft:entity.player.big_fall";
const PLAYER_SMALL_FALL_SOUND_EVENT: &str = "minecraft:entity.player.small_fall";
const GLOBAL_LEVEL_EVENT_SOUND_DISTANCE: f64 = 2.0;
const VANILLA_VEC3_NORMALIZE_EPSILON: f64 = 1.0e-5;
const SCULK_SHRIEKER_TOP_Y: f64 = 0.5;
/// Fixed local seed for `LevelEventSoundRandomState::default()`. Vanilla seeds
/// the client `level.random` from a wall-clock `RandomSupport.generateUniqueSeed`;
/// a fixed seed keeps the derived level-event pitch stream deterministic.
const DEFAULT_LEVEL_EVENT_SOUND_SEED: i64 = 0;
/// Fixed local seed for the client sound-seed generator. Vanilla derives sound
/// seeds from `RandomSource.nextLong()` (`Level.soundSeedGenerator` /
/// `ClientLevel.random`), itself seeded from wall-clock unique seeds; a distinct
/// fixed seed keeps `WorldStore` serializable and deterministic.
const SOUND_SEED_RANDOM_INITIAL_SEED: i64 = 0x0005_EED5_01DA_0D10;
const MAX_PENDING_LOCAL_INTERACTION_SOUND_ACTIONS: usize = 8;
const MAX_PENDING_LOCAL_PLAYER_AMBIENT_AUDIO_TICKS: usize = 16;

const fn default_true() -> bool {
    true
}

/// `level.random`: a client-local `LegacyRandomSource`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelEventSoundRandomState(LegacyRandomSource);

impl LevelEventSoundRandomState {
    pub fn with_seed(seed: i64) -> Self {
        Self(LegacyRandomSource::with_seed(seed))
    }

    pub fn next_float(&mut self) -> f32 {
        self.0.next_float()
    }

    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        self.0.next_int(bound)
    }

    pub fn next_double(&mut self) -> f64 {
        self.0.next_double()
    }

    pub fn next_boolean(&mut self) -> bool {
        self.0.next_bits(1) != 0
    }

    pub fn next_long(&mut self) -> i64 {
        self.0.next_long()
    }

    pub fn next_gaussian(&mut self) -> f64 {
        self.0.next_gaussian()
    }
}

impl RandomSource for LevelEventSoundRandomState {
    fn next_int(&mut self, bound: i32) -> i32 {
        Self::next_int_bound(self, bound)
    }

    fn next_long(&mut self) -> i64 {
        Self::next_long(self)
    }

    fn next_boolean(&mut self) -> bool {
        Self::next_boolean(self)
    }

    fn next_float(&mut self) -> f32 {
        Self::next_float(self)
    }

    fn next_double(&mut self) -> f64 {
        Self::next_double(self)
    }

    fn next_gaussian(&mut self) -> f64 {
        Self::next_gaussian(self)
    }
}

fn legacy_random_draw<T>(seed: &mut u64, draw: impl FnOnce(&mut LegacyRandomSource) -> T) -> T {
    let mut random = LegacyRandomSource::from_state(*seed);
    let value = draw(&mut random);
    *seed = random.state();
    value
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
    pub(crate) fn with_seed(seed: i64) -> Self {
        Self {
            seed: LegacyRandomSource::with_seed(seed).state(),
        }
    }

    /// Next `RandomSource.nextLong()` sound seed.
    pub fn next_long(&mut self) -> i64 {
        legacy_random_draw(&mut self.seed, LegacyRandomSource::next_long)
    }

    /// Next `RandomSource.nextFloat()` draw on the same LCG stream.
    pub(crate) fn next_float(&mut self) -> f32 {
        legacy_random_draw(&mut self.seed, LegacyRandomSource::next_float)
    }
}

impl Default for SoundSeedRandomState {
    fn default() -> Self {
        Self::with_seed(SOUND_SEED_RANDOM_INITIAL_SEED)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct JukeboxSongPosIndex(pub(crate) rustc_hash::FxHashMap<crate::BlockPos, usize>);

impl PartialEq for JukeboxSongPosIndex {
    fn eq(&self, _other: &Self) -> bool {
        true
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
    #[serde(skip)]
    pub(crate) playing_jukebox_song_index: JukeboxSongPosIndex,
    #[serde(default)]
    pub last_jukebox_event: Option<JukeboxLevelEventState>,
    #[serde(default)]
    pub sound_seed_random: SoundSeedRandomState,
    /// Vanilla `Player.wasUnderwater`, retained beside the audio transition
    /// queue because it exists solely to drive LocalPlayer ambient sounds.
    #[serde(default)]
    pub(crate) local_player_was_underwater: bool,
    /// Vanilla `BubbleColumnAmbientSoundHandler` edge state.
    #[serde(default)]
    pub(crate) local_player_was_in_bubble_column: bool,
    /// The handler suppresses a bubble-column entry sound on its first tick.
    #[serde(default)]
    pub(crate) local_player_ambient_audio_started: bool,
    #[serde(default)]
    pub(crate) active_tickable_entity_sounds: BTreeMap<i32, ActiveTickableEntitySound>,
    #[serde(default)]
    pub(crate) local_player_was_fall_flying: bool,
    #[serde(default)]
    pub(crate) local_player_elytra_sound: Option<ActiveElytraSound>,
    #[serde(default)]
    pub(crate) local_player_riding_sounds: Option<ActiveLocalPlayerRidingSounds>,
    #[serde(skip)]
    pub(crate) pending_local_interaction_sound_actions: VecDeque<LocalInteractionSoundAction>,
    #[serde(skip)]
    pub(crate) pending_local_player_ambient_audio_ticks: VecDeque<LocalPlayerAmbientAudioTickState>,
    #[serde(skip)]
    pub(crate) local_player_ambient_audio_tick_pending_completion: bool,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPlayerAmbientAudioTickState {
    /// Position where `Player.updateIsUnderwater` ran before movement.
    pub position: ProtocolVec3d,
    /// Position observed by ambient handlers after movement.
    pub bubble_column_position: ProtocolVec3d,
    pub underwater: bool,
    pub underwater_transition: Option<LocalPlayerUnderwaterAudioTransition>,
    pub bubble_column_transition: Option<LocalPlayerBubbleColumnAudioTransition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPlayerUnderwaterAudioTransition {
    Enter { loop_seed: i64 },
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPlayerBubbleColumnAudioTransition {
    Upwards,
    Whirlpool,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct ActiveElytraSound {
    pub seed: i64,
    pub time: u32,
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ActiveLocalPlayerRidingSounds {
    Minecart {
        vehicle_id: i32,
        dry_seed: i64,
        underwater_seed: i64,
    },
    HappyGhast {
        vehicle_id: i32,
        seed: i64,
    },
    Nautilus {
        vehicle_id: i32,
        seed: i64,
    },
}

impl ActiveLocalPlayerRidingSounds {
    pub(crate) fn vehicle_id(self) -> i32 {
        match self {
            Self::Minecart { vehicle_id, .. }
            | Self::HappyGhast { vehicle_id, .. }
            | Self::Nautilus { vehicle_id, .. } => vehicle_id,
        }
    }
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

impl WorldStore {
    /// Advances Vanilla `Player.updateIsUnderwater` at the pre-movement point
    /// of one LocalPlayer tick and records the ordered audio observation for
    /// the native ambient-sound handlers.
    pub fn begin_local_player_ambient_audio_tick(&mut self) -> bool {
        if self
            .client_audio
            .local_player_ambient_audio_tick_pending_completion
        {
            self.complete_local_player_ambient_audio_tick();
        }
        let Some(pose) = self.local_player.pose else {
            return false;
        };
        let underwater = self.local_player_eye_in_water();
        let was_underwater = self.client_audio.local_player_was_underwater;
        self.client_audio.local_player_was_underwater = underwater;

        let underwater_transition =
            if self.local_player_is_spectator() || underwater == was_underwater {
                None
            } else if underwater {
                Some(LocalPlayerUnderwaterAudioTransition::Enter {
                    loop_seed: self.next_sound_seed(),
                })
            } else {
                Some(LocalPlayerUnderwaterAudioTransition::Exit)
            };

        if self
            .client_audio
            .pending_local_player_ambient_audio_ticks
            .len()
            >= MAX_PENDING_LOCAL_PLAYER_AMBIENT_AUDIO_TICKS
        {
            self.client_audio
                .pending_local_player_ambient_audio_ticks
                .pop_front();
        }
        self.client_audio
            .pending_local_player_ambient_audio_ticks
            .push_back(LocalPlayerAmbientAudioTickState {
                position: pose.position,
                bubble_column_position: pose.position,
                underwater,
                underwater_transition,
                bubble_column_transition: None,
            });
        self.client_audio
            .local_player_ambient_audio_tick_pending_completion = true;
        true
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

    pub fn apply_stop_sound(&mut self, packet: ProtocolStopSound) -> StopSoundEventState {
        self.counters.stop_sound_packets += 1;
        let state = StopSoundEventState {
            source: packet.source.map(|source| source.as_str().to_string()),
            name: packet.name,
        };
        self.client_audio.last_stop_sound = Some(state.clone());
        state
    }

    pub(crate) fn local_player_damage_hurt_sound(
        &mut self,
        effects: Option<DamageEffects>,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let position = self.local_player_pose()?.position;
        let state = SoundEventState {
            sound: direct_sound_holder(effects?.sound_resource_id()),
            source: SoundSource::Players.as_str().to_string(),
            position,
            volume: 1.0,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
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
            position: transform.position,
            volume,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn game_event_positioned_sound(
        &mut self,
        event: ProtocolGameEvent,
    ) -> Option<SoundEventState> {
        let pose = self.local_player_pose()?;
        let feet_position = pose.position;
        let eye_position = ProtocolVec3d {
            x: feet_position.x,
            y: feet_position.y + self.local_player_eye_height(pose),
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

    /// Resolves a client-predicted interaction into its local sound event.
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
                let instrument = self.local_instrument_use_state(self.local_item_in_hand(hand)?)?;
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
            LocalInteractionSoundAction::BundleInsert => {
                let position = self.local_player_pose()?.position;
                let mut state = local_player_sound_state(
                    position,
                    "minecraft:item.bundle.insert",
                    0.8,
                    0.8 + random.next_float() * 0.4,
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
            LocalInteractionSoundAction::BundleInsertFail => {
                let position = self.local_player_pose()?.position;
                let mut state = local_player_sound_state(
                    position,
                    "minecraft:item.bundle.insert_fail",
                    1.0,
                    1.0,
                    SoundSource::Players.as_str(),
                );
                state.seed = self.next_sound_seed();
                return Some(LocalInteractionSoundEffect::Positioned(
                    self.record_positioned_sound(state),
                ));
            }
            LocalInteractionSoundAction::KineticWeaponUse { hand } => {
                let stack = self.local_item_in_hand(hand)?;
                if !self.item_stack_has_effective_component(
                    stack,
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                ) {
                    return None;
                }
                let sound_summary = self
                    .items
                    .component_source(
                        stack.item_id,
                        &stack.component_patch,
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                    )
                    .kinetic_weapon
                    .as_ref()
                    .and_then(|weapon| weapon.sound.as_ref())?;
                let sound = sound_event_summary_holder(sound_summary)?;
                let position = self.local_player_pose()?.position;
                SoundEventState {
                    sound,
                    source: SoundSource::Players.as_str().to_string(),
                    position,
                    volume: 1.0,
                    pitch: 1.0,
                    seed: 0,
                    distance_delay: false,
                }
            }
            LocalInteractionSoundAction::PiercingWeaponUse { hand } => {
                let stack = self.local_item_in_hand(hand)?;
                if !self.item_stack_has_effective_component(
                    stack,
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                ) {
                    return None;
                }
                let sound_summary = self
                    .items
                    .component_source(
                        stack.item_id,
                        &stack.component_patch,
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                    )
                    .piercing_weapon
                    .as_ref()
                    .and_then(|weapon| weapon.sound.as_ref())?;
                let sound = sound_event_summary_holder(sound_summary)?;
                let position = self.local_player_pose()?.position;
                SoundEventState {
                    sound,
                    source: SoundSource::Players.as_str().to_string(),
                    position,
                    volume: 1.0,
                    pitch: 1.0,
                    seed: 0,
                    distance_delay: false,
                }
            }
            LocalInteractionSoundAction::BlockDestroy {
                pos,
                block_state_id,
            } => self.level_event_block_break_sound_state(ProtocolLevelEvent {
                event_type: 2001,
                pos,
                data: block_state_id,
                global: false,
            })?,
            LocalInteractionSoundAction::BlockPositioned {
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
            LocalInteractionSoundAction::BlockToggleSound {
                pos,
                sound_resource_id,
            } => block_sound_state(
                pos,
                sound_resource_id,
                1.0,
                random.next_float() * 0.1 + 0.9,
                SoundSource::Blocks.as_str(),
            ),
            LocalInteractionSoundAction::EntityPositioned {
                entity_id,
                sound_resource_id,
                volume_bits,
                pitch_bits,
            } => {
                let position = self.entities.transform(entity_id)?.position;
                SoundEventState {
                    sound: direct_sound_holder(sound_resource_id),
                    source: SoundSource::Players.as_str().to_string(),
                    position,
                    volume: f32::from_bits(volume_bits),
                    pitch: f32::from_bits(pitch_bits),
                    seed: 0,
                    distance_delay: false,
                }
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
            LocalInteractionSoundAction::Consume { hand } => {
                return self
                    .local_consume_sound(hand, random)
                    .map(LocalInteractionSoundEffect::Positioned);
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
                    position: Some(position),
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
                        position,
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
                let group = vanilla_block_sound_group_26_1(block_type_id)?;
                let profile = vanilla_block_sound_type_26_1(group.sound_type)?;
                block_sound_state(
                    pos,
                    profile.place_sound,
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

    pub(crate) fn local_consume_sound(
        &mut self,
        hand: bbb_protocol::packets::InteractionHand,
        random: &mut LevelEventSoundRandomState,
    ) -> Option<SoundEventState> {
        let stack = self.local_item_in_hand(hand)?;
        let component = self.items.component_source(
            stack.item_id,
            &stack.component_patch,
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        );
        let sound = sound_event_summary_holder(component.consumable_sound.as_ref()?)?;
        let (volume, pitch) = if component.consumable?.is_drink() {
            (0.5, 0.9 + random.next_float() * 0.1)
        } else {
            (
                if random.next_boolean() { 0.5 } else { 1.0 },
                1.0 + (random.next_float() - random.next_float()) * 0.2,
            )
        };
        let state = SoundEventState {
            sound,
            source: SoundSource::Players.as_str().to_string(),
            position: self.local_player_pose()?.position,
            volume,
            pitch,
            seed: self.next_sound_seed(),
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn vault_level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let sound = match event.event_type {
            VAULT_ACTIVATE_LEVEL_EVENT
                if self.block_entity_type_id_at(event.pos)
                    == Some(VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID) =>
            {
                "minecraft:block.vault.activate"
            }
            VAULT_DEACTIVATE_LEVEL_EVENT => "minecraft:block.vault.deactivate",
            _ => return None,
        };
        Some(block_sound_state_with_distance_delay(
            event.pos,
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
            position: global_level_event_sound_position(event.pos, camera_position),
            volume: sound.volume,
            pitch: sound.pitch,
            seed: 0,
            distance_delay: false,
        })
    }

    pub fn record_positioned_sound(&mut self, state: SoundEventState) -> SoundEventState {
        self.client_audio.last_sound = Some(state.clone());
        state
    }

    /// Vanilla `Gui.playAirBubblePoppedSound` calls
    /// `LocalPlayer.playSound`, which emits an immediate positioned sound at
    /// the player's feet through `ClientLevel.playLocalSound`.
    pub fn hud_air_bubble_pop_sound(
        &mut self,
        position: ProtocolVec3d,
        volume: f32,
        pitch: f32,
    ) -> SoundEventState {
        let seed = self.next_sound_seed();
        self.record_positioned_sound(SoundEventState {
            sound: direct_sound_holder(HUD_BUBBLE_POP_SOUND_EVENT),
            source: SoundSource::Players.as_str().to_string(),
            position,
            volume,
            pitch,
            seed,
            distance_delay: false,
        })
    }
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
    use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_TYPES;
    use bbb_protocol::packets::{
        AddEntity, BlockPos as ProtocolBlockPos, EntityDataValue, EntityDataValueKind,
        EquipmentSlotUpdate, ItemStackSummary, LevelEvent, PlayTime, RegistryTags, RemoveEntities,
        SetEntityData, SetEquipment, SoundSource, StopSound, TagNetworkPayload,
        UpdateEnabledFeatures, UpdateTags,
    };
    use uuid::Uuid;

    mod fixtures;
    mod level_events;
    mod living_sounds;
    mod sound_events;
    use fixtures::*;

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
    fn silent_minecart_and_bee_suppress_tickable_sounds() {
        let mut store = WorldStore::new();
        let mut minecart = protocol_add_entity_of_type(70, VANILLA_ENTITY_TYPE_MINECART_ID);
        minecart.delta_movement = ProtocolVec3d {
            x: 0.3,
            y: 0.0,
            z: 0.4,
        };
        store.apply_add_entity(minecart);
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 70,
            values: vec![entity_silent_data(true)],
        }));
        assert!(store
            .post_add_entity_sound_for_entity(70, VANILLA_ENTITY_TYPE_MINECART_ID)
            .is_none());
        store.advance_tickable_entity_sounds(4, 4);
        assert!(store.tickable_entity_sounds().is_empty());

        let mut bee = protocol_add_entity_of_type(71, VANILLA_ENTITY_TYPE_BEE_ID);
        bee.delta_movement = ProtocolVec3d {
            x: 0.5,
            y: 0.0,
            z: 0.0,
        };
        store.apply_add_entity(bee);
        store
            .post_add_entity_sound_for_entity(71, VANILLA_ENTITY_TYPE_BEE_ID)
            .unwrap();
        assert_eq!(store.tickable_entity_sounds().len(), 1);
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 71,
            values: vec![entity_silent_data(true)],
        }));
        assert!(store.tickable_entity_sounds().is_empty());

        // Clearing `is_silent` restores both looping instances on tick.
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 70,
            values: vec![entity_silent_data(false)],
        }));
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 71,
            values: vec![entity_silent_data(false)],
        }));
        assert_eq!(store.tickable_entity_sounds().len(), 2);
    }

    #[test]
    fn local_player_elytra_and_riding_tickable_sounds_match_vanilla() {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.local_player_id = Some(1);
        store.apply_add_entity(protocol_add_entity_of_type(
            1,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        ));
        store.local_player.pose = Some(crate::LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 4.0,
                y: 8.0,
                z: 4.0,
            },
            delta_movement: ProtocolVec3d {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            ..Default::default()
        });

        let base_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
        let fall_flying_flag = |active: bool| EntityDataValue {
            data_id: base_accessors.shared_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(if active {
                crate::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING
            } else {
                0
            }),
        };

        assert!(store.apply_set_entity_data(SetEntityData {
            id: 1,
            values: vec![fall_flying_flag(true)],
        }));
        store.advance_tickable_entity_sounds(10, 10);
        let elytra = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|s| s.event.sound.location.as_deref() == Some(ELYTRA_FLYING_SOUND_EVENT))
            .expect("elytra sound should be active during delay window");
        assert_close(elytra.event.volume, 0.0);
        assert_close(elytra.event.pitch, 1.0);

        store.advance_tickable_entity_sounds(20, 20);
        let elytra = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|s| s.event.sound.location.as_deref() == Some(ELYTRA_FLYING_SOUND_EVENT))
            .unwrap();
        // time=30 -> speed_sqr=4.0 -> raw volume=1.0 -> ramp (30-20)/20 = 0.5
        assert_close(elytra.event.volume, 0.5);
        assert_close(elytra.event.pitch, 1.0);

        store.advance_tickable_entity_sounds(10, 10);
        let elytra = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|s| s.event.sound.location.as_deref() == Some(ELYTRA_FLYING_SOUND_EVENT))
            .unwrap();
        // time=40 -> full volume=1.0 -> pitch = 1.0 + (1.0 - 0.8) = 1.2
        assert_close(elytra.event.volume, 1.0);
        assert_close(elytra.event.pitch, 1.2);

        assert!(store.apply_set_entity_data(SetEntityData {
            id: 1,
            values: vec![fall_flying_flag(false)],
        }));
        store.advance_tickable_entity_sounds(1, 1);
        assert!(store
            .tickable_entity_sounds()
            .iter()
            .all(|s| s.event.sound.location.as_deref() != Some(ELYTRA_FLYING_SOUND_EVENT)));

        // Riding minecart emits both dry and underwater inside sounds with Attenuation.NONE.
        let mut cart = protocol_add_entity_of_type(20, VANILLA_ENTITY_TYPE_MINECART_ID);
        cart.position = ProtocolVec3d {
            x: 4.0,
            y: 8.0,
            z: 4.0,
        };
        cart.delta_movement = ProtocolVec3d {
            x: 0.4,
            y: 0.0,
            z: 0.0,
        };
        store.apply_add_entity(cart);
        store.local_player_vehicle_id = Some(20);
        store.advance_tickable_entity_sounds(1, 1);
        let sounds = store.tickable_entity_sounds();
        let dry = sounds
            .iter()
            .find(|s| s.event.sound.location.as_deref() == Some(MINECART_INSIDE_SOUND_EVENT))
            .unwrap();
        let wet = sounds
            .iter()
            .find(|s| {
                s.event.sound.location.as_deref() == Some(MINECART_INSIDE_UNDERWATER_SOUND_EVENT)
            })
            .unwrap();
        assert!(!dry.event.linear_attenuation);
        assert!(!wet.event.linear_attenuation);
        assert_close(dry.event.volume, 0.3);
        assert_close(wet.event.volume, 0.0);

        // Happy ghast riding uses 5.0 amplifier and 3D speed when dry.
        let mut ghast = protocol_add_entity_of_type(21, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID);
        ghast.position = ProtocolVec3d {
            x: 4.0,
            y: 8.0,
            z: 4.0,
        };
        ghast.delta_movement = ProtocolVec3d {
            x: 0.0,
            y: 0.2,
            z: 0.0,
        };
        store.apply_add_entity(ghast);
        store.local_player_vehicle_id = Some(21);
        store.advance_tickable_entity_sounds(1, 1);
        let ghast_sound = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|s| s.event.sound.location.as_deref() == Some(HAPPY_GHAST_RIDING_SOUND_EVENT))
            .unwrap();
        assert!(!ghast_sound.event.linear_attenuation);
        assert_close(ghast_sound.event.volume, 1.0);

        // Nautilus riding requires the nautilus to be underwater.
        let mut nautilus = protocol_add_entity_of_type(22, VANILLA_ENTITY_TYPE_NAUTILUS_ID);
        nautilus.position = ProtocolVec3d {
            x: 4.0,
            y: 8.0,
            z: 4.0,
        };
        nautilus.delta_movement = ProtocolVec3d {
            x: 0.0,
            y: 0.2,
            z: 0.0,
        };
        store.apply_add_entity(nautilus);
        store.local_player_vehicle_id = Some(22);
        store.advance_tickable_entity_sounds(1, 1);
        let nautilus_dry = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|s| s.event.sound.location.as_deref() == Some(NAUTILUS_RIDING_SOUND_EVENT))
            .unwrap();
        assert_close(nautilus_dry.event.volume, 0.0);

        let water_state_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
        store.insert_decoded_chunk(single_block_state_chunk(water_state_id));
        let nautilus_wet = store
            .tickable_entity_sounds()
            .into_iter()
            .find(|s| s.event.sound.location.as_deref() == Some(NAUTILUS_RIDING_SOUND_EVENT))
            .unwrap();
        assert_close(nautilus_wet.event.volume, 1.0);

        // Silent vehicle suppresses riding sounds.
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 22,
            values: vec![entity_silent_data(true)],
        }));
        assert!(store
            .tickable_entity_sounds()
            .iter()
            .all(|s| s.event.sound.location.as_deref() != Some(NAUTILUS_RIDING_SOUND_EVENT)));
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
            (DamageEffects::Hurt, "minecraft:entity.player.hurt"),
            (DamageEffects::Thorns, "minecraft:entity.player.hurt"),
            (
                DamageEffects::Drowning,
                "minecraft:entity.player.hurt_drown",
            ),
            (
                DamageEffects::Burning,
                "minecraft:entity.player.hurt_on_fire",
            ),
            (
                DamageEffects::Poking,
                "minecraft:entity.player.hurt_sweet_berry_bush",
            ),
            (
                DamageEffects::Freezing,
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
                .living_damage_hurt_sound_for_entity(entity_id, 0, DamageEffects::Hurt, || {
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
    fn dynamic_damage_audio_policies_read_canonical_metadata_and_fluid_state() {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        let assert_sound =
            |store: &mut WorldStore, entity_id: i32, expected: &str, expected_volume: f32| {
                let sound = store
                    .living_damage_hurt_sound_for_entity(entity_id, 0, DamageEffects::Hurt, || 0.5)
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
            .living_damage_hurt_sound_for_entity(210, 0, DamageEffects::Hurt, || 0.5)
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
            .living_damage_hurt_sound_for_entity(91, 0, DamageEffects::Hurt, || 0.5)
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
            .living_damage_hurt_sound_for_entity(91, 0, DamageEffects::Hurt, || 0.5)
            .unwrap();
        assert_eq!(
            bypassed.sound.location.as_deref(),
            Some("minecraft:entity.wolf.hurt")
        );
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
            ($identity:ty, $registry:expr, $entity_type_id:expr, $accessors:path, $serializer_id:expr, $serializer:expr, $volume:expr) => {
                let accessors = $accessors($entity_type_id).expect("sound-variant accessors");
                for (_, identity) in <$identity>::builtin_variants() {
                    // Sound-variant registries are loaded from data files, so the
                    // wire holder id is the sorted-resource-id index rather than the
                    // bootstrap index `builtin_variants` counts in. Ask the fallback
                    // registry for it instead of assuming either order.
                    let registry_id = store
                        .registry_holder_id_by_resource_id($registry, identity.resource_id())
                        .expect("vanilla fallback holder id");
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
                                DamageEffects::Hurt,
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
            "minecraft:cat_sound_variant",
            VANILLA_ENTITY_TYPE_CAT_ID,
            vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::CatSoundVariant,
            1.0
        );
        assert_family!(
            ChickenSoundVariantIdentity,
            "minecraft:chicken_sound_variant",
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::ChickenSoundVariant,
            1.0
        );
        assert_family!(
            CowSoundVariantIdentity,
            "minecraft:cow_sound_variant",
            VANILLA_ENTITY_TYPE_COW_ID,
            vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::CowSoundVariant,
            0.4
        );
        assert_family!(
            PigSoundVariantIdentity,
            "minecraft:pig_sound_variant",
            VANILLA_ENTITY_TYPE_PIG_ID,
            vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
            VANILLA_ENTITY_DATA_SERIALIZER_PIG_SOUND_VARIANT_ID,
            EntityDataRegistryHolder::PigSoundVariant,
            1.0
        );
        assert_family!(
            WolfSoundVariantIdentity,
            "minecraft:wolf_sound_variant",
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
            .living_damage_hurt_sound_for_entity(4_200, 0, DamageEffects::Hurt, || samples
                .next()
                .unwrap())
            .is_some());

        store.reset_registry_session();
        let mut stale_samples = [0.75, 0.25].into_iter();
        assert!(store
            .living_damage_hurt_sound_for_entity(4_200, 0, DamageEffects::Hurt, || {
                stale_samples.next().unwrap()
            })
            .is_none());

        let mut retained_samples = [0.75, 0.25].into_iter();
        assert!(generation_a
            .living_damage_hurt_sound_for_entity(4_200, 0, DamageEffects::Hurt, || {
                retained_samples.next().unwrap()
            })
            .is_some());
    }

    #[test]
    fn synchronized_sound_variant_registry_uses_server_order_and_typed_payload() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            EntityDataRegistryHolder::CatSoundVariant.registry(),
            0,
            vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
                "example:custom",
                cat_sound_variant_json(),
            )],
        );
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
        for (id, baby) in [(2_000, false), (2_001, true)] {
            store.apply_add_entity(protocol_add_entity_of_type(id, VANILLA_ENTITY_TYPE_CAT_ID));
            let mut values = vec![cat_sound_variant_entity_data(0)];
            if baby {
                values.push(baby_entity_data(VANILLA_ENTITY_TYPE_CAT_ID));
            }
            assert!(store.apply_set_entity_data(SetEntityData { id, values }));
            let hurt = store
                .living_damage_hurt_sound_for_entity(id, 0, DamageEffects::Hurt, || 0.5)
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
            EntityDataRegistryHolder::CatSoundVariant.registry(),
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
            EntityDataRegistryHolder::CatSoundVariant.registry(),
            0,
            vec![crate::RegistryPacketEntry::with_raw_data(
                "example:malformed",
                vec![10, 0],
            )],
        );
        store.begin_registry_configuration();
        assert!(store.finish_registry_configuration().is_err());
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
    fn hud_air_bubble_pop_sound_is_player_positioned_and_advances_the_sound_seed() {
        let mut store = WorldStore::new();
        let position = vec3(1.25, 64.0, -3.5);

        let first = store.hud_air_bubble_pop_sound(position, 1.0, 1.3);
        let second = store.hud_air_bubble_pop_sound(position, 0.5, 1.0);

        assert_eq!(
            first.sound.location.as_deref(),
            Some("minecraft:ui.hud.bubble_pop")
        );
        assert_eq!(first.source, "player");
        assert_eq!(first.position, position);
        assert_close(first.volume, 1.0);
        assert_close(first.pitch, 1.3);
        assert!(!first.distance_delay);
        assert_ne!(first.seed, second.seed);
        assert_eq!(store.last_sound(), Some(&second));
        assert_eq!(store.counters().sound_packets, 0);
    }

    #[test]
    fn entity_client_movement_sounds_use_canonical_seed_and_vanilla_categories() {
        let mut store = WorldStore::new();
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
        assert_ne!(blaze.seed, guardian.seed);
        assert_ne!(guardian.seed, elder_guardian.seed);
        assert_eq!(store.last_sound(), Some(&elder_guardian));
        assert_eq!(store.counters().sound_packets, 0);
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
