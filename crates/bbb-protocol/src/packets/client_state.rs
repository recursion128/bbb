use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
};

use super::{chunks, decode_optional_vec3d, read_resource_key, BlockPos, Vec3d};

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
        cooldown_group: read_resource_key(decoder)?,
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
mod tests {
    use super::super::{decode_play_clientbound, BlockPos, PlayClientbound};
    use super::*;
    use crate::{
        codec::{Decoder, Encoder},
        ids,
    };
    use uuid::Uuid;

    #[test]
    fn decodes_boss_event_operations() {
        let id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);

        let payload = boss_event_payload(id, 0, |payload| {
            payload.write_bytes(&nbt_string_root("Raid"));
            payload.write_f32(0.75);
            payload.write_var_i32(5);
            payload.write_var_i32(3);
            payload.write_u8(BOSS_EVENT_FLAG_DARKEN_SCREEN | BOSS_EVENT_FLAG_CREATE_WORLD_FOG);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::Add {
                    name: "Raid".to_string(),
                    progress: 0.75,
                    color: BossBarColor::Purple,
                    overlay: BossBarOverlay::Notched12,
                    flags: BossEventFlags {
                        darken_screen: true,
                        play_music: false,
                        create_world_fog: true,
                    },
                },
            })
        );

        let payload = boss_event_payload(id, 1, |_| {});
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::Remove,
            })
        );

        let payload = boss_event_payload(id, 2, |payload| {
            payload.write_f32(0.25);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateProgress { progress: 0.25 },
            })
        );

        let payload = boss_event_payload(id, 3, |payload| {
            payload.write_bytes(&nbt_string_root("Dragon"));
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateName {
                    name: "Dragon".to_string(),
                },
            })
        );

        let payload = boss_event_payload(id, 4, |payload| {
            payload.write_var_i32(6);
            payload.write_var_i32(4);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateStyle {
                    color: BossBarColor::White,
                    overlay: BossBarOverlay::Notched20,
                },
            })
        );

        let payload = boss_event_payload(id, 5, |payload| {
            payload.write_u8(BOSS_EVENT_FLAG_PLAY_MUSIC);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateProperties {
                    flags: BossEventFlags {
                        darken_screen: false,
                        play_music: true,
                        create_world_fog: false,
                    },
                },
            })
        );
    }

    #[test]
    fn decodes_change_difficulty_with_wrapped_ids() {
        let payload = change_difficulty_payload(2, true);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Normal,
                locked: true,
            })
        );

        let payload = change_difficulty_payload(5, false);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Easy,
                locked: false,
            })
        );

        let payload = change_difficulty_payload(-1, false);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Hard,
                locked: false,
            })
        );
    }

    #[test]
    fn decodes_cooldown_packet_wire_order() {
        let mut payload = Encoder::new();
        payload.write_string("minecraft:ender_pearl");
        payload.write_var_i32(40);
        let payload = payload.into_inner();

        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_COOLDOWN, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::Cooldown(Cooldown {
                cooldown_group: "minecraft:ender_pearl".to_string(),
                duration: 40,
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:ender_pearl");
        assert_eq!(decoder.read_var_i32().unwrap(), 40);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_damage_event_without_source_position_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(7);
        payload.write_var_i32(0);
        payload.write_var_i32(35);
        payload.write_bool(false);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_DAMAGE_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::DamageEvent(DamageEvent {
                entity_id: 123,
                source_type_id: 7,
                source_cause_id: -1,
                source_direct_id: 34,
                source_position: None,
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(decoder.read_var_i32().unwrap(), 35);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_damage_event_with_source_position_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(456);
        payload.write_var_i32(12);
        payload.write_var_i32(79);
        payload.write_var_i32(0);
        payload.write_bool(true);
        payload.write_f64(1.25);
        payload.write_f64(-2.5);
        payload.write_f64(64.0);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_DAMAGE_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::DamageEvent(DamageEvent {
                entity_id: 456,
                source_type_id: 12,
                source_cause_id: 78,
                source_direct_id: -1,
                source_position: Some(Vec3d {
                    x: 1.25,
                    y: -2.5,
                    z: 64.0,
                }),
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 456);
        assert_eq!(decoder.read_var_i32().unwrap(), 12);
        assert_eq!(decoder.read_var_i32().unwrap(), 79);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_f64().unwrap(), 1.25);
        assert_eq!(decoder.read_f64().unwrap(), -2.5);
        assert_eq!(decoder.read_f64().unwrap(), 64.0);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_update_mob_effect_packet_wire_order_and_flags() {
        let flags = MOB_EFFECT_FLAG_AMBIENT | MOB_EFFECT_FLAG_SHOW_ICON | MOB_EFFECT_FLAG_BLEND;
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(5);
        payload.write_var_i32(2);
        payload.write_var_i32(600);
        payload.write_u8(flags);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::UpdateMobEffect(UpdateMobEffect {
                entity_id: 123,
                effect_id: 5,
                amplifier: 2,
                duration_ticks: 600,
                flags: MobEffectFlags {
                    raw: flags,
                    ambient: true,
                    visible: false,
                    show_icon: true,
                    blend: true,
                },
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_var_i32().unwrap(), 600);
        assert_eq!(decoder.read_u8().unwrap(), flags);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_remove_mob_effect_packet_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(5);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::RemoveMobEffect(RemoveMobEffect {
                entity_id: 123,
                effect_id: 5,
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_player_health() {
        let mut payload = Encoder::new();
        payload.write_f32(0.0);
        payload.write_var_i32(17);
        payload.write_f32(1.5);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_HEALTH, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetHealth(PlayerHealth {
                health: 0.0,
                food: 17,
                saturation: 1.5,
            })
        );
    }

    #[test]
    fn decodes_player_experience() {
        let mut payload = Encoder::new();
        payload.write_f32(0.625);
        payload.write_var_i32(12);
        payload.write_var_i32(345);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_EXPERIENCE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetExperience(PlayerExperience {
                progress: 0.625,
                level: 12,
                total: 345,
            })
        );
    }

    #[test]
    fn decodes_held_slot() {
        let mut payload = Encoder::new();
        payload.write_var_i32(6);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_HELD_SLOT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 6 })
        );
    }

    #[test]
    fn decodes_game_event_and_set_time() {
        let mut payload = Encoder::new();
        payload.write_u8(7);
        payload.write_f32(0.75);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_GAME_EVENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::GameEvent(GameEvent {
                event_id: 7,
                param: 0.75,
            })
        );

        let mut payload = Encoder::new();
        payload.write_i64(12345);
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i64(6000);
        payload.write_f32(0.25);
        payload.write_f32(1.0);
        payload.write_var_i32(1);
        payload.write_var_i64(18000);
        payload.write_f32(0.5);
        payload.write_f32(0.0);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_TIME, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetTime(PlayTime {
                game_time: 12345,
                clock_updates: vec![
                    ClockUpdate {
                        clock_id: 0,
                        total_ticks: 6000,
                        partial_tick: 0.25,
                        rate: 1.0,
                    },
                    ClockUpdate {
                        clock_id: 1,
                        total_ticks: 18000,
                        partial_tick: 0.5,
                        rate: 0.0,
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_title_camera_and_ticking_packets() {
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT,
            &nbt_string_root("Action"),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetActionBarText(SetActionBarText {
                content: "Action".to_string(),
            })
        );

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_TITLE_TEXT,
            &nbt_string_root("Title"),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetTitleText(SetTitleText {
                content: "Title".to_string(),
            })
        );

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT,
            &nbt_string_root("Subtitle"),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetSubtitleText(SetSubtitleText {
                content: "Subtitle".to_string(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_i32(10);
        payload.write_i32(70);
        payload.write_i32(-5);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_TITLES_ANIMATION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetTitlesAnimation(SetTitlesAnimation {
                fade_in: 10,
                stay: 70,
                fade_out: -5,
            })
        );

        let mut payload = Encoder::new();
        payload.write_f32(0.25);
        payload.write_bool(true);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TICKING_STATE, &payload.into_inner())
                .unwrap();
        let PlayClientbound::TickingState(ticking_state) = packet else {
            panic!("wrong packet");
        };
        assert_eq!(
            ticking_state,
            TickingState {
                tick_rate: 0.25,
                frozen: true,
            }
        );
        assert_eq!(ticking_state.clamped_tick_rate(), 1.0);
        assert_eq!(
            TickingState {
                tick_rate: 2.5,
                frozen: false,
            }
            .clamped_tick_rate(),
            2.5
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(40);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TICKING_STEP, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TickingStep(TickingStep { tick_steps: 40 })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(12345);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_CAMERA, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetCamera(SetCamera { camera_id: 12345 })
        );
    }

    #[test]
    fn decodes_player_abilities_spawn_distance_and_system_chat() {
        let mut payload = Encoder::new();
        payload.write_u8(0b0000_1101);
        payload.write_f32(0.05);
        payload.write_f32(0.1);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_ABILITIES,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerAbilities(PlayerAbilities {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("minecraft:overworld");
        payload.write_i64(encode_block_pos(-5, 70, 12));
        payload.write_f32(90.0);
        payload.write_f32(-10.0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetDefaultSpawnPosition(SetDefaultSpawnPosition {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                yaw: 90.0,
                pitch: -10.0,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(12);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetSimulationDistance(SetSimulationDistance { distance: 12 })
        );

        let mut payload = nbt_string_root("Server restarting");
        payload.push(1);
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SYSTEM_CHAT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SystemChat(SystemChat {
                content: "Server restarting".to_string(),
                overlay: true,
            })
        );
    }

    fn boss_event_payload(
        id: Uuid,
        operation: i32,
        write_body: impl FnOnce(&mut Encoder),
    ) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_uuid(id);
        payload.write_var_i32(operation);
        write_body(&mut payload);
        payload.into_inner()
    }

    fn change_difficulty_payload(id: i32, locked: bool) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_var_i32(id);
        payload.write_bool(locked);
        payload.into_inner()
    }

    fn encode_block_pos(x: i32, y: i32, z: i32) -> i64 {
        (((x as i64) & 0x3ffffff) << 38) | (((z as i64) & 0x3ffffff) << 12) | ((y as i64) & 0xfff)
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }
}
