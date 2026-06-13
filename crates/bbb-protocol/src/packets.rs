use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
};
use uuid::Uuid;

pub mod chunks;
pub mod client_audio;
pub mod client_common;
pub mod client_features;
pub mod client_state;
pub mod client_ui;
pub mod command_suggestions;
pub mod connection;
pub mod debug_game;
pub mod entities;
pub mod inventory;
pub mod maps;
pub mod movement;
pub mod play_clientbound;
pub mod player_actions;
pub mod player_info;
pub mod scoreboard;
pub mod server_presentation;
pub mod serverbound;
pub mod tags;
pub mod waypoints;
pub mod world_border;
pub mod world_effects;
pub use chunks::*;
pub use client_audio::*;
pub use client_common::*;
pub use client_features::*;
pub use client_state::*;
pub use client_ui::*;
pub use command_suggestions::*;
pub use connection::*;
pub use debug_game::*;
pub use entities::*;
pub use inventory::*;
pub use maps::*;
pub use movement::*;
pub use play_clientbound::*;
pub use player_actions::*;
pub use player_info::*;
pub use scoreboard::*;
pub use server_presentation::*;
pub use serverbound::*;
pub use tags::*;
pub use waypoints::*;
pub use world_border::*;
pub use world_effects::*;

const MAX_SERVER_ICON_BYTES: usize = 2 * 1024 * 1024;

fn decode_nullable_string(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_string(32767)?))
    } else {
        Ok(None)
    }
}

fn decode_optional_uuid(decoder: &mut Decoder<'_>) -> Result<Option<Uuid>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_uuid()?))
    } else {
        Ok(None)
    }
}

fn decode_optional_component_summary_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decode_component_summary_from_decoder(decoder)?))
    } else {
        Ok(None)
    }
}

fn decode_optional_byte_array(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<Option<Vec<u8>>> {
    if decoder.read_bool()? {
        Ok(Some(decode_byte_array(decoder, max_len, what)?))
    } else {
        Ok(None)
    }
}

fn decode_optional_trailing_number_format(decoder: &mut Decoder<'_>) -> Result<Option<Vec<u8>>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }

    let len = decoder.remaining_len();
    Ok(Some(
        decoder.read_exact(len, "number format payload")?.to_vec(),
    ))
}

fn decode_byte_array(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<Vec<u8>> {
    let len = decoder.read_len()?;
    if len > max_len {
        return Err(ProtocolError::PacketTooLarge(len, max_len));
    }
    Ok(decoder.read_exact(len, what)?.to_vec())
}

fn read_resource_key(decoder: &mut Decoder<'_>) -> Result<String> {
    decoder.read_string(32767)
}

fn decode_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    Ok(Vec3d {
        x: decoder.read_f64()?,
        y: decoder.read_f64()?,
        z: decoder.read_f64()?,
    })
}

fn decode_optional_vec3d(decoder: &mut Decoder<'_>) -> Result<Option<Vec3d>> {
    if decoder.read_bool()? {
        Ok(Some(decode_vec3d(decoder)?))
    } else {
        Ok(None)
    }
}

fn pack_move_flags(on_ground: bool, horizontal_collision: bool) -> u8 {
    let mut flags = 0;
    if on_ground {
        flags |= 1;
    }
    if horizontal_collision {
        flags |= 2;
    }
    flags
}

#[cfg(test)]
mod tests;
