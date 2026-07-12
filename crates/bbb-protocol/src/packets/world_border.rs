use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InitializeBorder {
    pub new_center_x: f64,
    pub new_center_z: f64,
    pub old_size: f64,
    pub new_size: f64,
    pub lerp_time: i64,
    pub new_absolute_max_size: i32,
    pub warning_blocks: i32,
    pub warning_time: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetBorderCenter {
    pub new_center_x: f64,
    pub new_center_z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetBorderLerpSize {
    pub old_size: f64,
    pub new_size: f64,
    pub lerp_time: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetBorderSize {
    pub size: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetBorderWarningDelay {
    pub warning_delay: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetBorderWarningDistance {
    pub warning_blocks: i32,
}

pub(super) fn decode_set_border_center(decoder: &mut Decoder<'_>) -> Result<SetBorderCenter> {
    let packet = SetBorderCenter {
        new_center_x: decoder.read_f64()?,
        new_center_z: decoder.read_f64()?,
    };
    finish_border_packet(decoder, packet, "set border center")
}

pub(super) fn decode_set_border_lerp_size(decoder: &mut Decoder<'_>) -> Result<SetBorderLerpSize> {
    let packet = SetBorderLerpSize {
        old_size: decoder.read_f64()?,
        new_size: decoder.read_f64()?,
        lerp_time: decoder.read_var_i64()?,
    };
    finish_border_packet(decoder, packet, "set border lerp size")
}

pub(super) fn decode_set_border_size(decoder: &mut Decoder<'_>) -> Result<SetBorderSize> {
    let packet = SetBorderSize {
        size: decoder.read_f64()?,
    };
    finish_border_packet(decoder, packet, "set border size")
}

pub(super) fn decode_set_border_warning_delay(
    decoder: &mut Decoder<'_>,
) -> Result<SetBorderWarningDelay> {
    let packet = SetBorderWarningDelay {
        warning_delay: decoder.read_var_i32()?,
    };
    finish_border_packet(decoder, packet, "set border warning delay")
}

pub(super) fn decode_set_border_warning_distance(
    decoder: &mut Decoder<'_>,
) -> Result<SetBorderWarningDistance> {
    let packet = SetBorderWarningDistance {
        warning_blocks: decoder.read_var_i32()?,
    };
    finish_border_packet(decoder, packet, "set border warning distance")
}

fn finish_border_packet<T>(decoder: &Decoder<'_>, packet: T, name: &str) -> Result<T> {
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {name} packet"
        )));
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::Encoder,
        ids,
        packets::{decode_play_clientbound, PlayClientbound},
    };

    #[test]
    fn decodes_initialize_border_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(12.5);
        payload.write_f64(-30.25);
        payload.write_f64(60000000.0);
        payload.write_f64(300.0);
        payload.write_var_i64(1200);
        payload.write_var_i32(29999984);
        payload.write_var_i32(5);
        payload.write_var_i32(15);

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_INITIALIZE_BORDER, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::InitializeBorder(InitializeBorder {
                new_center_x: 12.5,
                new_center_z: -30.25,
                old_size: 60000000.0,
                new_size: 300.0,
                lerp_time: 1200,
                new_absolute_max_size: 29999984,
                warning_blocks: 5,
                warning_time: 15,
            })
        );
        let mut trailing = payload;
        trailing.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_INITIALIZE_BORDER, &trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after initialize border packet")
        );
    }

    #[test]
    fn decodes_set_border_center_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(-7.75);
        payload.write_f64(42.125);

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_BORDER_CENTER, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderCenter(SetBorderCenter {
                new_center_x: -7.75,
                new_center_z: 42.125,
            })
        );
        assert_trailing_rejected(
            ids::play::CLIENTBOUND_SET_BORDER_CENTER,
            payload,
            "set border center",
        );
    }

    #[test]
    fn decodes_set_border_lerp_size_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(512.0);
        payload.write_f64(128.0);
        payload.write_var_i64(6000);

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderLerpSize(SetBorderLerpSize {
                old_size: 512.0,
                new_size: 128.0,
                lerp_time: 6000,
            })
        );
        assert_trailing_rejected(
            ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE,
            payload,
            "set border lerp size",
        );
    }

    #[test]
    fn decodes_set_border_size_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(2048.0);

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_BORDER_SIZE, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderSize(SetBorderSize { size: 2048.0 })
        );
        assert_trailing_rejected(
            ids::play::CLIENTBOUND_SET_BORDER_SIZE,
            payload,
            "set border size",
        );
    }

    #[test]
    fn decodes_set_border_warning_delay_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(30);

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY, &payload)
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderWarningDelay(SetBorderWarningDelay { warning_delay: 30 })
        );
        assert_trailing_rejected(
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY,
            payload,
            "set border warning delay",
        );
    }

    #[test]
    fn decodes_set_border_warning_distance_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(8);

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE, &payload)
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderWarningDistance(SetBorderWarningDistance {
                warning_blocks: 8
            })
        );
        assert_trailing_rejected(
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE,
            payload,
            "set border warning distance",
        );
    }

    fn assert_trailing_rejected(packet_id: i32, mut payload: Vec<u8>, packet_name: &str) {
        payload.push(0);
        assert!(decode_play_clientbound(packet_id, &payload)
            .unwrap_err()
            .to_string()
            .contains(&format!("trailing bytes after {packet_name} packet")));
    }
}
