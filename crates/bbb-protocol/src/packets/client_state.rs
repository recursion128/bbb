use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
};

use super::{
    chunks, decode_optional_vec3d, read_resource_key, read_resource_location, BlockPos, Vec3d,
};

const MAX_CLOCK_UPDATES: usize = 4096;
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
        name: String,
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
        name: String,
    },
    UpdateStyle {
        color: BossBarColor,
        overlay: BossBarOverlay,
    },
    UpdateProperties {
        flags: BossEventFlags,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

impl BossBarColor {
    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Pink,
            1 => Self::Blue,
            2 => Self::Red,
            3 => Self::Green,
            4 => Self::Yellow,
            5 => Self::Purple,
            6 => Self::White,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid boss bar color ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossBarOverlay {
    Progress,
    Notched6,
    Notched10,
    Notched12,
    Notched20,
}

impl BossBarOverlay {
    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Progress,
            1 => Self::Notched6,
            2 => Self::Notched10,
            3 => Self::Notched12,
            4 => Self::Notched20,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid boss bar overlay ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BossEventFlags {
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}

impl BossEventFlags {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    fn from_id(id: i32) -> Self {
        match id.rem_euclid(4) {
            0 => Self::Peaceful,
            1 => Self::Easy,
            2 => Self::Normal,
            3 => Self::Hard,
            _ => unreachable!("rem_euclid(4) is always in 0..4"),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetDefaultSpawnPosition {
    pub dimension: String,
    pub pos: BlockPos,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatFormatting {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Obfuscated,
    Bold,
    Strikethrough,
    Underline,
    Italic,
    Reset,
}

impl ChatFormatting {
    pub(super) fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Black,
            1 => Self::DarkBlue,
            2 => Self::DarkGreen,
            3 => Self::DarkAqua,
            4 => Self::DarkRed,
            5 => Self::DarkPurple,
            6 => Self::Gold,
            7 => Self::Gray,
            8 => Self::DarkGray,
            9 => Self::Blue,
            10 => Self::Green,
            11 => Self::Aqua,
            12 => Self::Red,
            13 => Self::LightPurple,
            14 => Self::Yellow,
            15 => Self::White,
            16 => Self::Obfuscated,
            17 => Self::Bold,
            18 => Self::Strikethrough,
            19 => Self::Underline,
            20 => Self::Italic,
            21 => Self::Reset,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid chat formatting ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSimulationDistance {
    pub distance: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetActionBarText {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTitleText {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSubtitleText {
    pub content: String,
}

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
    pub content: String,
    pub overlay: bool,
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

pub(super) fn decode_boss_event(decoder: &mut Decoder<'_>) -> Result<BossEvent> {
    let id = decoder.read_uuid()?;
    let operation = match decoder.read_var_i32()? {
        0 => BossEventOperation::Add {
            name: decode_component_summary_from_decoder(decoder)?,
            progress: decoder.read_f32()?,
            color: BossBarColor::from_ordinal(decoder.read_var_i32()?)?,
            overlay: BossBarOverlay::from_ordinal(decoder.read_var_i32()?)?,
            flags: BossEventFlags::from_bits(decoder.read_u8()?),
        },
        1 => BossEventOperation::Remove,
        2 => BossEventOperation::UpdateProgress {
            progress: decoder.read_f32()?,
        },
        3 => BossEventOperation::UpdateName {
            name: decode_component_summary_from_decoder(decoder)?,
        },
        4 => BossEventOperation::UpdateStyle {
            color: BossBarColor::from_ordinal(decoder.read_var_i32()?)?,
            overlay: BossBarOverlay::from_ordinal(decoder.read_var_i32()?)?,
        },
        5 => BossEventOperation::UpdateProperties {
            flags: BossEventFlags::from_bits(decoder.read_u8()?),
        },
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid boss event operation ordinal {other}"
            )))
        }
    };

    Ok(BossEvent { id, operation })
}

pub(super) fn decode_change_difficulty(decoder: &mut Decoder<'_>) -> Result<ChangeDifficulty> {
    Ok(ChangeDifficulty {
        difficulty: Difficulty::from_id(decoder.read_var_i32()?),
        locked: decoder.read_bool()?,
    })
}

pub(super) fn decode_cooldown(decoder: &mut Decoder<'_>) -> Result<Cooldown> {
    Ok(Cooldown {
        cooldown_group: read_resource_location(decoder)?,
        duration: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_damage_event(decoder: &mut Decoder<'_>) -> Result<DamageEvent> {
    Ok(DamageEvent {
        entity_id: decoder.read_var_i32()?,
        source_type_id: decoder.read_var_i32()?,
        source_cause_id: decoder.read_var_i32()? - 1,
        source_direct_id: decoder.read_var_i32()? - 1,
        source_position: decode_optional_vec3d(decoder)?,
    })
}

pub(super) fn decode_game_event(decoder: &mut Decoder<'_>) -> Result<GameEvent> {
    Ok(GameEvent {
        event_id: decoder.read_u8()?,
        param: decoder.read_f32()?,
    })
}

pub(super) fn decode_player_abilities(decoder: &mut Decoder<'_>) -> Result<PlayerAbilities> {
    let flags = decoder.read_u8()?;
    Ok(PlayerAbilities {
        invulnerable: flags & 0x01 != 0,
        flying: flags & 0x02 != 0,
        can_fly: flags & 0x04 != 0,
        instabuild: flags & 0x08 != 0,
        flying_speed: decoder.read_f32()?,
        walking_speed: decoder.read_f32()?,
    })
}

pub(super) fn decode_remove_mob_effect(decoder: &mut Decoder<'_>) -> Result<RemoveMobEffect> {
    Ok(RemoveMobEffect {
        entity_id: decoder.read_var_i32()?,
        effect_id: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_set_action_bar_text(decoder: &mut Decoder<'_>) -> Result<SetActionBarText> {
    Ok(SetActionBarText {
        content: decode_component_summary_from_decoder(decoder)?,
    })
}

pub(super) fn decode_clear_titles(decoder: &mut Decoder<'_>) -> Result<ClearTitles> {
    Ok(ClearTitles {
        reset_times: decoder.read_bool()?,
    })
}

pub(super) fn decode_set_camera(decoder: &mut Decoder<'_>) -> Result<SetCamera> {
    Ok(SetCamera {
        camera_id: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_player_health(decoder: &mut Decoder<'_>) -> Result<PlayerHealth> {
    Ok(PlayerHealth {
        health: decoder.read_f32()?,
        food: decoder.read_var_i32()?,
        saturation: decoder.read_f32()?,
    })
}

pub(super) fn decode_set_held_slot(decoder: &mut Decoder<'_>) -> Result<SetHeldSlot> {
    Ok(SetHeldSlot {
        slot: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_default_spawn_position(
    decoder: &mut Decoder<'_>,
) -> Result<SetDefaultSpawnPosition> {
    Ok(SetDefaultSpawnPosition {
        dimension: read_resource_key(decoder)?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
        yaw: decoder.read_f32()?,
        pitch: decoder.read_f32()?,
    })
}

pub(super) fn decode_player_experience(decoder: &mut Decoder<'_>) -> Result<PlayerExperience> {
    Ok(PlayerExperience {
        progress: decoder.read_f32()?,
        level: decoder.read_var_i32()?,
        total: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_set_simulation_distance(
    decoder: &mut Decoder<'_>,
) -> Result<SetSimulationDistance> {
    Ok(SetSimulationDistance {
        distance: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_set_subtitle_text(decoder: &mut Decoder<'_>) -> Result<SetSubtitleText> {
    Ok(SetSubtitleText {
        content: decode_component_summary_from_decoder(decoder)?,
    })
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
    let mut clock_updates = Vec::with_capacity(clock_count);
    for _ in 0..clock_count {
        clock_updates.push(ClockUpdate {
            clock_id: decoder.read_var_i32()?,
            total_ticks: decoder.read_var_i64()?,
            partial_tick: decoder.read_f32()?,
            rate: decoder.read_f32()?,
        });
    }
    Ok(PlayTime {
        game_time,
        clock_updates,
    })
}

pub(super) fn decode_set_title_text(decoder: &mut Decoder<'_>) -> Result<SetTitleText> {
    Ok(SetTitleText {
        content: decode_component_summary_from_decoder(decoder)?,
    })
}

pub(super) fn decode_set_titles_animation(decoder: &mut Decoder<'_>) -> Result<SetTitlesAnimation> {
    Ok(SetTitlesAnimation {
        fade_in: decoder.read_i32()?,
        stay: decoder.read_i32()?,
        fade_out: decoder.read_i32()?,
    })
}

pub(super) fn decode_system_chat(decoder: &mut Decoder<'_>) -> Result<SystemChat> {
    Ok(SystemChat {
        content: decode_component_summary_from_decoder(decoder)?,
        overlay: decoder.read_bool()?,
    })
}

pub(super) fn decode_ticking_state(decoder: &mut Decoder<'_>) -> Result<TickingState> {
    Ok(TickingState {
        tick_rate: decoder.read_f32()?,
        frozen: decoder.read_bool()?,
    })
}

pub(super) fn decode_ticking_step(decoder: &mut Decoder<'_>) -> Result<TickingStep> {
    Ok(TickingStep {
        tick_steps: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_update_mob_effect(decoder: &mut Decoder<'_>) -> Result<UpdateMobEffect> {
    Ok(UpdateMobEffect {
        entity_id: decoder.read_var_i32()?,
        effect_id: decoder.read_var_i32()?,
        amplifier: decoder.read_var_i32()?,
        duration_ticks: decoder.read_var_i32()?,
        flags: MobEffectFlags::from_bits(decoder.read_u8()?),
    })
}

#[cfg(test)]
mod tests;
