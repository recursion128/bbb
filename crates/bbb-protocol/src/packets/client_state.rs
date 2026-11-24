use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::chat_formattings::ChatFormatting;
pub use crate::difficulties::Difficulty;
use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_boxed_rich_text_component_from_decoder_with_registry,
        decode_rich_text_component_from_decoder_with_registry,
    },
    content_registry::ContentRegistry,
    ids,
    mob_effect_types::vanilla_mob_effect_type,
    nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH},
    RichTextComponent,
};
pub use bbb_facts::boss_bar_styles::{BossBarColor, BossBarOverlay};
use bbb_facts::boss_event_operation_types::BossEventOperationType;

use super::{chunks, decode_optional_vec3d, read_resource_location, BlockPos, Vec3d};

const MAX_CLOCK_UPDATES: usize = 4096;
const COMPONENT_NBT_BYTES: usize = crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize;
const BOSS_EVENT_FLAG_DARKEN_SCREEN: u8 = 1;
const BOSS_EVENT_FLAG_PLAY_MUSIC: u8 = 2;
const BOSS_EVENT_FLAG_CREATE_WORLD_FOG: u8 = 4;
const MOB_EFFECT_FLAG_AMBIENT: u8 = 1;
const MOB_EFFECT_FLAG_VISIBLE: u8 = 2;
const MOB_EFFECT_FLAG_SHOW_ICON: u8 = 4;
const MOB_EFFECT_FLAG_BLEND: u8 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cooldown {
    pub cooldown_group: String,
    pub duration: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DamageEvent {
    pub entity_id: i32,
    pub source_type_id: i32,
    pub source_cause_id: i32,
    pub source_direct_id: i32,
    pub source_position: Option<Vec3d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateMobEffect {
    pub entity_id: i32,
    pub effect_id: i32,
    pub amplifier: i32,
    pub duration_ticks: i32,
    pub flags: MobEffectFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveMobEffect {
    pub entity_id: i32,
    pub effect_id: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobEffectFlags {
    pub raw: u8,
    pub ambient: bool,
    pub visible: bool,
    pub show_icon: bool,
    pub blend: bool,
}

impl MobEffectFlags {
    /// The byte `ClientboundUpdateMobEffectPacket`'s constructor folds the four
    /// booleans into (`ClientboundUpdateMobEffectPacket.java:30-47`). `raw` is
    /// what a decode read, so it is not consulted here: an encode has to answer
    /// for the booleans it was handed.
    pub fn to_bits(self) -> u8 {
        let mut raw = 0;
        if self.ambient {
            raw |= MOB_EFFECT_FLAG_AMBIENT;
        }
        if self.visible {
            raw |= MOB_EFFECT_FLAG_VISIBLE;
        }
        if self.show_icon {
            raw |= MOB_EFFECT_FLAG_SHOW_ICON;
        }
        if self.blend {
            raw |= MOB_EFFECT_FLAG_BLEND;
        }
        raw
    }

    fn from_bits(raw: u8) -> Self {
        Self {
            raw,
            ambient: raw & MOB_EFFECT_FLAG_AMBIENT != 0,
            visible: raw & MOB_EFFECT_FLAG_VISIBLE != 0,
            show_icon: raw & MOB_EFFECT_FLAG_SHOW_ICON != 0,
            blend: raw & MOB_EFFECT_FLAG_BLEND != 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossEvent {
    pub id: Uuid,
    pub operation: BossEventOperation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BossEventOperation {
    Add {
        name: Box<RichTextComponent>,
        progress: f32,
        color: BossBarColor,
        overlay: BossBarOverlay,
        flags: BossEventFlags,
    },
    Remove,
    UpdateProgress {
        progress: f32,
    },
    UpdateName {
        name: Box<RichTextComponent>,
    },
    UpdateStyle {
        color: BossBarColor,
        overlay: BossBarOverlay,
    },
    UpdateProperties {
        flags: BossEventFlags,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BossEventFlags {
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}

impl BossEventFlags {
    fn to_bits(self) -> u8 {
        let mut bits = 0;
        if self.darken_screen {
            bits |= BOSS_EVENT_FLAG_DARKEN_SCREEN;
        }
        if self.play_music {
            bits |= BOSS_EVENT_FLAG_PLAY_MUSIC;
        }
        if self.create_world_fog {
            bits |= BOSS_EVENT_FLAG_CREATE_WORLD_FOG;
        }
        bits
    }

    fn from_bits(bits: u8) -> Self {
        Self {
            darken_screen: bits & BOSS_EVENT_FLAG_DARKEN_SCREEN != 0,
            play_music: bits & BOSS_EVENT_FLAG_PLAY_MUSIC != 0,
            create_world_fog: bits & BOSS_EVENT_FLAG_CREATE_WORLD_FOG != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeDifficulty {
    pub difficulty: Difficulty,
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_id: u8,
    pub param: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayTime {
    pub game_time: i64,
    pub clock_updates: Vec<ClockUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClockUpdate {
    pub clock_id: i32,
    pub total_ticks: i64,
    pub partial_tick: f32,
    pub rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerAbilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instabuild: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

impl PlayerAbilities {
    /// The abilities a game mode grants, which is exactly what the wire form
    /// carries.
    ///
    /// `GameType.updatePlayerAbilities` (`GameType.java:62-80`) over the
    /// `Abilities` field defaults (`Abilities.java:7-20`). It runs on both
    /// sides of the connection — `ServerPlayerGameMode.setGameModeForPlayer`
    /// (`ServerPlayerGameMode.java:79-84`) and `MultiPlayerGameMode`
    /// (`MultiPlayerGameMode.java:91-106`) — so this is the single owner of
    /// the projection.
    ///
    /// `Abilities.mayBuild` is deliberately absent: `updatePlayerAbilities`
    /// sets it, but `ClientboundPlayerAbilitiesPacket` never writes it, so it
    /// is not part of this struct's contract.
    pub fn for_game_type(game_type: crate::game_types::GameType) -> Self {
        use crate::game_types::GameType;

        let (invulnerable, flying, can_fly, instabuild) = match game_type {
            GameType::Creative => (true, false, true, true),
            GameType::Spectator => (true, true, true, false),
            GameType::Survival | GameType::Adventure => (false, false, false, false),
        };
        Self {
            invulnerable,
            flying,
            can_fly,
            instabuild,
            flying_speed: DEFAULT_FLYING_SPEED,
            walking_speed: DEFAULT_WALKING_SPEED,
        }
    }
}

/// `Abilities.DEFAULT_FLYING_SPEED` (`Abilities.java:12`).
const DEFAULT_FLYING_SPEED: f32 = 0.05;
/// `Abilities.DEFAULT_WALKING_SPEED` (`Abilities.java:13`).
const DEFAULT_WALKING_SPEED: f32 = 0.1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetDefaultSpawnPosition {
    pub dimension: String,
    pub pos: BlockPos,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSimulationDistance {
    pub distance: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetActionBarText {
    pub content: RichTextComponent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTitleText {
    pub content: RichTextComponent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSubtitleText {
    pub content: RichTextComponent,
}

macro_rules! impl_plain_text_packet {
    ($packet:ty) => {
        impl $packet {
            pub fn plain(content: impl Into<String>) -> Self {
                Self {
                    content: RichTextComponent::plain(content),
                }
            }
        }
    };
}

impl_plain_text_packet!(SetActionBarText);
impl_plain_text_packet!(SetTitleText);
impl_plain_text_packet!(SetSubtitleText);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearTitles {
    pub reset_times: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTitlesAnimation {
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TickingState {
    pub tick_rate: f32,
    pub frozen: bool,
}

impl TickingState {
    pub fn clamped_tick_rate(&self) -> f32 {
        self.tick_rate.max(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickingStep {
    pub tick_steps: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCamera {
    pub camera_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemChat {
    pub content: Box<RichTextComponent>,
    pub overlay: bool,
}

impl SystemChat {
    /// Builds a semantic plain-Component fixture. Production wire decode keeps
    /// the exact Component NBT through the fields above.
    pub fn plain(content: impl Into<String>, overlay: bool) -> Self {
        Self {
            content: Box::new(RichTextComponent::plain(content)),
            overlay,
        }
    }

    /// Wraps an already-built Component, the form `CommandSourceStack.sendSuccess`
    /// and `sendFailure` reach the client with.
    pub fn from_component(content: RichTextComponent, overlay: bool) -> Self {
        Self {
            content: Box::new(content),
            overlay,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerHealth {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerExperience {
    pub progress: f32,
    pub level: i32,
    pub total: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetHeldSlot {
    pub slot: i32,
}

/// `ClientboundBossEventPacket.java:43-96`,
/// `ClientboundBossEventPacket.java:112-205`, and
/// `ClientboundBossEventPacket.java:214-318` define the operation codecs.
pub fn encode_play_clientbound_boss_event(event: &BossEvent) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_uuid(event.id);
    match &event.operation {
        BossEventOperation::Add {
            name,
            progress,
            color,
            overlay,
            flags,
        } => {
            encoder.write_var_i32(BossEventOperationType::Add as i32);
            encoder.write_bytes(&encode_nbt_summary_bounded(
                Some(name.component()),
                COMPONENT_NBT_BYTES,
                MAX_NBT_DEPTH,
            )?);
            encoder.write_f32(*progress);
            encoder.write_var_i32(color.ordinal());
            encoder.write_var_i32(overlay.ordinal());
            encoder.write_u8(flags.to_bits());
        }
        BossEventOperation::Remove => {
            encoder.write_var_i32(BossEventOperationType::Remove as i32);
        }
        BossEventOperation::UpdateProgress { progress } => {
            encoder.write_var_i32(BossEventOperationType::UpdateProgress as i32);
            encoder.write_f32(*progress);
        }
        BossEventOperation::UpdateName { name } => {
            encoder.write_var_i32(BossEventOperationType::UpdateName as i32);
            encoder.write_bytes(&encode_nbt_summary_bounded(
                Some(name.component()),
                COMPONENT_NBT_BYTES,
                MAX_NBT_DEPTH,
            )?);
        }
        BossEventOperation::UpdateStyle { color, overlay } => {
            encoder.write_var_i32(BossEventOperationType::UpdateStyle as i32);
            encoder.write_var_i32(color.ordinal());
            encoder.write_var_i32(overlay.ordinal());
        }
        BossEventOperation::UpdateProperties { flags } => {
            encoder.write_var_i32(BossEventOperationType::UpdateProperties as i32);
            encoder.write_u8(flags.to_bits());
        }
    }
    Ok((ids::play::CLIENTBOUND_BOSS_EVENT, encoder.into_inner()))
}

pub(super) fn decode_boss_event(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<BossEvent> {
    let id = decoder.read_uuid()?;
    let ordinal = decoder.read_var_i32()?;
    let operation_type = BossEventOperationType::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid boss event operation ordinal {ordinal}"))
    })?;
    let operation = match operation_type {
        BossEventOperationType::Add => BossEventOperation::Add {
            name: decode_boxed_rich_text_component_from_decoder_with_registry(decoder, registry)?,
            progress: decoder.read_f32()?,
            color: decode_boss_bar_color(decoder)?,
            overlay: decode_boss_bar_overlay(decoder)?,
            flags: BossEventFlags::from_bits(decoder.read_u8()?),
        },
        BossEventOperationType::Remove => BossEventOperation::Remove,
        BossEventOperationType::UpdateProgress => BossEventOperation::UpdateProgress {
            progress: decoder.read_f32()?,
        },
        BossEventOperationType::UpdateName => BossEventOperation::UpdateName {
            name: decode_boxed_rich_text_component_from_decoder_with_registry(decoder, registry)?,
        },
        BossEventOperationType::UpdateStyle => BossEventOperation::UpdateStyle {
            color: decode_boss_bar_color(decoder)?,
            overlay: decode_boss_bar_overlay(decoder)?,
        },
        BossEventOperationType::UpdateProperties => BossEventOperation::UpdateProperties {
            flags: BossEventFlags::from_bits(decoder.read_u8()?),
        },
    };

    let packet = BossEvent { id, operation };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after boss event packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_boss_bar_color(decoder: &mut Decoder<'_>) -> Result<BossBarColor> {
    let ordinal = decoder.read_var_i32()?;
    BossBarColor::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid boss bar color ordinal {ordinal}"))
    })
}

fn decode_boss_bar_overlay(decoder: &mut Decoder<'_>) -> Result<BossBarOverlay> {
    let ordinal = decoder.read_var_i32()?;
    BossBarOverlay::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid boss bar overlay ordinal {ordinal}"))
    })
}

pub(super) fn decode_change_difficulty(decoder: &mut Decoder<'_>) -> Result<ChangeDifficulty> {
    let update = ChangeDifficulty {
        difficulty: Difficulty::from_id(decoder.read_var_i32()?),
        locked: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after change difficulty packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_cooldown(decoder: &mut Decoder<'_>) -> Result<Cooldown> {
    let update = Cooldown {
        cooldown_group: read_resource_location(decoder)?,
        duration: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after cooldown packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_damage_event(decoder: &mut Decoder<'_>) -> Result<DamageEvent> {
    let entity_id = decoder.read_var_i32()?;
    let source_type_id = decoder.read_var_i32()?;
    let source_cause_id = decoder.read_var_i32()?.checked_sub(1).ok_or_else(|| {
        ProtocolError::InvalidData("damage event source_cause_id underflow".to_string())
    })?;
    let source_direct_id = decoder.read_var_i32()?.checked_sub(1).ok_or_else(|| {
        ProtocolError::InvalidData("damage event source_direct_id underflow".to_string())
    })?;
    let update = DamageEvent {
        entity_id,
        source_type_id,
        source_cause_id,
        source_direct_id,
        source_position: decode_optional_vec3d(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after damage event packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_game_event(decoder: &mut Decoder<'_>) -> Result<GameEvent> {
    let update = GameEvent {
        event_id: decoder.read_u8()?,
        param: decoder.read_f32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after game event packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_player_abilities(decoder: &mut Decoder<'_>) -> Result<PlayerAbilities> {
    let flags = decoder.read_u8()?;
    let update = PlayerAbilities {
        invulnerable: flags & 0x01 != 0,
        flying: flags & 0x02 != 0,
        can_fly: flags & 0x04 != 0,
        instabuild: flags & 0x08 != 0,
        flying_speed: decoder.read_f32()?,
        walking_speed: decoder.read_f32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after player abilities packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_remove_mob_effect(decoder: &mut Decoder<'_>) -> Result<RemoveMobEffect> {
    let packet = RemoveMobEffect {
        entity_id: decoder.read_var_i32()?,
        effect_id: decode_mob_effect_holder_id(decoder)?,
    };
    ensure_client_state_packet_consumed(decoder, "remove mob effect")?;
    Ok(packet)
}

/// `ClientboundSetActionBarTextPacket.STREAM_CODEC`
/// (`ClientboundSetActionBarTextPacket.java:11-13`).
pub fn encode_play_clientbound_set_action_bar_text(
    update: &SetActionBarText,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(update.content.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok((
        ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT,
        encoder.into_inner(),
    ))
}

pub(super) fn decode_set_action_bar_text_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SetActionBarText> {
    let update = SetActionBarText {
        content: decode_rich_text_component_from_decoder_with_registry(decoder, registry)?,
    };
    ensure_client_state_packet_consumed(decoder, "set action bar text")?;
    Ok(update)
}

/// `ClientboundClearTitlesPacket.write` (`ClientboundClearTitlesPacket.java:22-24`).
pub fn encode_play_clientbound_clear_titles(update: &ClearTitles) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bool(update.reset_times);
    Ok((ids::play::CLIENTBOUND_CLEAR_TITLES, encoder.into_inner()))
}

pub(super) fn decode_clear_titles(decoder: &mut Decoder<'_>) -> Result<ClearTitles> {
    let update = ClearTitles {
        reset_times: decoder.read_bool()?,
    };
    ensure_client_state_packet_consumed(decoder, "clear titles")?;
    Ok(update)
}

pub(super) fn decode_set_camera(decoder: &mut Decoder<'_>) -> Result<SetCamera> {
    let update = SetCamera {
        camera_id: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set camera packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_player_health(decoder: &mut Decoder<'_>) -> Result<PlayerHealth> {
    let update = PlayerHealth {
        health: decoder.read_f32()?,
        food: decoder.read_var_i32()?,
        saturation: decoder.read_f32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set health packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_set_held_slot(decoder: &mut Decoder<'_>) -> Result<SetHeldSlot> {
    let update = SetHeldSlot {
        slot: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set held slot packet".to_string(),
        ));
    }
    Ok(update)
}

/// `ClientboundSetDefaultSpawnPositionPacket.STREAM_CODEC`
/// (`ClientboundSetDefaultSpawnPositionPacket.java:10-12`) over
/// `LevelData.RespawnData.STREAM_CODEC` (`LevelData.java:44-52`):
/// `GlobalPos` then yaw then pitch.
pub fn encode_play_clientbound_set_default_spawn_position(
    update: &SetDefaultSpawnPosition,
) -> Result<(i32, Vec<u8>)> {
    let resource = super::normalize_resource_location(update.dimension.clone())?;
    let mut encoder = Encoder::new();
    encoder.write_string(&resource);
    encoder.write_i64(chunks::encode_block_pos(update.pos));
    encoder.write_f32(update.yaw);
    encoder.write_f32(update.pitch);
    Ok((
        ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION,
        encoder.into_inner(),
    ))
}

pub(super) fn decode_default_spawn_position(
    decoder: &mut Decoder<'_>,
) -> Result<SetDefaultSpawnPosition> {
    let update = SetDefaultSpawnPosition {
        dimension: read_resource_location(decoder)?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
        yaw: decoder.read_f32()?,
        pitch: decoder.read_f32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set default spawn position packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_player_experience(decoder: &mut Decoder<'_>) -> Result<PlayerExperience> {
    let update = PlayerExperience {
        progress: decoder.read_f32()?,
        level: decoder.read_var_i32()?,
        total: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set experience packet".to_string(),
        ));
    }
    Ok(update)
}

/// `ClientboundSetSimulationDistancePacket.write`
/// (`ClientboundSetSimulationDistancePacket.java:17-19`). Vanilla tracks this
/// separately from the view distance because the two bound different things:
/// `PlayerList.setSimulationDistance` (`PlayerList.java:810-812`) broadcasts it so
/// the client knows how far entities and block ticks are actually simulated,
/// while the chunk cache radius only says how far columns are streamed.
pub fn encode_play_clientbound_set_simulation_distance(
    update: SetSimulationDistance,
) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.distance);
    (
        ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE,
        encoder.into_inner(),
    )
}

pub(super) fn decode_set_simulation_distance(
    decoder: &mut Decoder<'_>,
) -> Result<SetSimulationDistance> {
    let update = SetSimulationDistance {
        distance: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set simulation distance packet".to_string(),
        ));
    }
    Ok(update)
}

/// `ClientboundSetSubtitleTextPacket.STREAM_CODEC`
/// (`ClientboundSetSubtitleTextPacket.java:11-13`).
pub fn encode_play_clientbound_set_subtitle_text(
    update: &SetSubtitleText,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(update.content.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok((
        ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT,
        encoder.into_inner(),
    ))
}

pub(super) fn decode_set_subtitle_text_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SetSubtitleText> {
    let update = SetSubtitleText {
        content: decode_rich_text_component_from_decoder_with_registry(decoder, registry)?,
    };
    ensure_client_state_packet_consumed(decoder, "set subtitle text")?;
    Ok(update)
}

pub(super) fn decode_play_time(decoder: &mut Decoder<'_>) -> Result<PlayTime> {
    let game_time = decoder.read_i64()?;
    let clock_count = decoder.read_len()?;
    if clock_count > MAX_CLOCK_UPDATES {
        return Err(ProtocolError::PacketTooLarge(
            clock_count,
            MAX_CLOCK_UPDATES,
        ));
    }
    let mut clock_updates = Vec::with_capacity(decoder.bounded_capacity(clock_count, 7));
    for _ in 0..clock_count {
        clock_updates.push(ClockUpdate {
            clock_id: decoder.read_var_i32()?,
            total_ticks: decoder.read_var_i64()?,
            partial_tick: decoder.read_f32()?,
            rate: decoder.read_f32()?,
        });
    }
    let update = PlayTime {
        game_time,
        clock_updates,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set time packet".to_string(),
        ));
    }
    Ok(update)
}

/// `ClientboundSetTitleTextPacket.STREAM_CODEC`
/// (`ClientboundSetTitleTextPacket.java:11-13`).
pub fn encode_play_clientbound_set_title_text(update: &SetTitleText) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(update.content.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok((ids::play::CLIENTBOUND_SET_TITLE_TEXT, encoder.into_inner()))
}

pub(super) fn decode_set_title_text_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SetTitleText> {
    let update = SetTitleText {
        content: decode_rich_text_component_from_decoder_with_registry(decoder, registry)?,
    };
    ensure_client_state_packet_consumed(decoder, "set title text")?;
    Ok(update)
}

/// `ClientboundSetTitlesAnimationPacket.write`
/// (`ClientboundSetTitlesAnimationPacket.java:28-32`): fadeIn, stay, fadeOut.
pub fn encode_play_clientbound_set_titles_animation(
    update: &SetTitlesAnimation,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_i32(update.fade_in);
    encoder.write_i32(update.stay);
    encoder.write_i32(update.fade_out);
    Ok((
        ids::play::CLIENTBOUND_SET_TITLES_ANIMATION,
        encoder.into_inner(),
    ))
}

pub(super) fn decode_set_titles_animation(decoder: &mut Decoder<'_>) -> Result<SetTitlesAnimation> {
    let update = SetTitlesAnimation {
        fade_in: decoder.read_i32()?,
        stay: decoder.read_i32()?,
        fade_out: decoder.read_i32()?,
    };
    ensure_client_state_packet_consumed(decoder, "set titles animation")?;
    Ok(update)
}

pub(super) fn decode_system_chat(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SystemChat> {
    let update = SystemChat {
        content: decode_boxed_rich_text_component_from_decoder_with_registry(decoder, registry)?,
        overlay: decoder.read_bool()?,
    };
    ensure_client_state_packet_consumed(decoder, "system chat")?;
    Ok(update)
}

pub(super) fn decode_ticking_state(decoder: &mut Decoder<'_>) -> Result<TickingState> {
    let update = TickingState {
        tick_rate: decoder.read_f32()?,
        frozen: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after ticking state packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_ticking_step(decoder: &mut Decoder<'_>) -> Result<TickingStep> {
    let update = TickingStep {
        tick_steps: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after ticking step packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_update_mob_effect(decoder: &mut Decoder<'_>) -> Result<UpdateMobEffect> {
    let update = UpdateMobEffect {
        entity_id: decoder.read_var_i32()?,
        effect_id: decode_mob_effect_holder_id(decoder)?,
        amplifier: decoder.read_var_i32()?,
        duration_ticks: decoder.read_var_i32()?,
        flags: MobEffectFlags::from_bits(decoder.read_u8()?),
    };
    ensure_client_state_packet_consumed(decoder, "update mob effect")?;
    Ok(update)
}

pub(super) fn decode_mob_effect_holder_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    validate_mob_effect_holder_id(decoder.read_var_i32()?)
}

pub(super) fn validate_mob_effect_holder_id(effect_id: i32) -> Result<i32> {
    if vanilla_mob_effect_type(effect_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "invalid vanilla mob effect holder id {effect_id}"
        )));
    }
    Ok(effect_id)
}

fn ensure_client_state_packet_consumed(decoder: &Decoder<'_>, packet: &str) -> Result<()> {
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {packet} packet"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
