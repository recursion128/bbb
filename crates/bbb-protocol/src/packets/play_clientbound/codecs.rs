use crate::{
    codec::{Decoder, ProtocolError, Result},
    packets::{chunks, decode_vec3d, read_resource_key, PlayerPositionUpdate},
};

use super::types::*;

const MAX_AWARD_STATS: usize = 8192;

pub(super) fn decode_award_stats(decoder: &mut Decoder<'_>) -> Result<AwardStats> {
    let count = decoder.read_len()?;
    if count > MAX_AWARD_STATS {
        return Err(ProtocolError::PacketTooLarge(count, MAX_AWARD_STATS));
    }

    let mut stats = Vec::with_capacity(count);
    for _ in 0..count {
        stats.push(StatUpdate {
            stat_type_id: decoder.read_var_i32()?,
            value_id: decoder.read_var_i32()?,
            amount: decoder.read_var_i32()?,
        });
    }
    Ok(AwardStats { stats })
}

pub(super) fn decode_play_login(decoder: &mut Decoder<'_>) -> Result<PlayLogin> {
    let player_id = decoder.read_i32()?;
    let hardcore = decoder.read_bool()?;
    let level_count = decoder.read_len()?;
    let mut levels = Vec::with_capacity(level_count);
    for _ in 0..level_count {
        levels.push(read_resource_key(decoder)?);
    }
    Ok(PlayLogin {
        player_id,
        hardcore,
        levels,
        max_players: decoder.read_var_i32()?,
        chunk_radius: decoder.read_var_i32()?,
        simulation_distance: decoder.read_var_i32()?,
        reduced_debug_info: decoder.read_bool()?,
        show_death_screen: decoder.read_bool()?,
        do_limited_crafting: decoder.read_bool()?,
        common_spawn_info: decode_common_spawn_info(decoder)?,
        enforces_secure_chat: decoder.read_bool()?,
    })
}

pub(super) fn decode_respawn(decoder: &mut Decoder<'_>) -> Result<Respawn> {
    Ok(Respawn {
        common_spawn_info: decode_common_spawn_info(decoder)?,
        data_to_keep: decoder.read_i8()?,
    })
}

pub(super) fn decode_player_position(decoder: &mut Decoder<'_>) -> Result<PlayerPositionUpdate> {
    let id = decoder.read_var_i32()?;
    let position = decode_vec3d(decoder)?;
    let delta_movement = decode_vec3d(decoder)?;
    let y_rot = decoder.read_f32()?;
    let x_rot = decoder.read_f32()?;
    let relatives_mask = decoder.read_i32()?;
    Ok(PlayerPositionUpdate {
        id,
        position,
        delta_movement,
        y_rot,
        x_rot,
        relatives_mask,
    })
}

fn decode_common_spawn_info(decoder: &mut Decoder<'_>) -> Result<CommonPlayerSpawnInfo> {
    Ok(CommonPlayerSpawnInfo {
        dimension_type_id: decoder.read_var_i32()?,
        dimension: read_resource_key(decoder)?,
        seed: decoder.read_i64()?,
        game_type: decoder.read_i8()?,
        previous_game_type: decoder.read_i8()?,
        is_debug: decoder.read_bool()?,
        is_flat: decoder.read_bool()?,
        last_death_location: decode_optional_global_pos(decoder)?,
        portal_cooldown: decoder.read_var_i32()?,
        sea_level: decoder.read_var_i32()?,
    })
}

fn decode_optional_global_pos(decoder: &mut Decoder<'_>) -> Result<Option<GlobalPos>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    Ok(Some(GlobalPos {
        dimension: read_resource_key(decoder)?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
    }))
}
