use serde::{Deserialize, Serialize};

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

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_INITIALIZE_BORDER,
            &payload.into_inner(),
        )
        .unwrap();
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
    }

    #[test]
    fn decodes_set_border_center_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(-7.75);
        payload.write_f64(42.125);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_CENTER,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderCenter(SetBorderCenter {
                new_center_x: -7.75,
                new_center_z: 42.125,
            })
        );
    }

    #[test]
    fn decodes_set_border_lerp_size_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(512.0);
        payload.write_f64(128.0);
        payload.write_var_i64(6000);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderLerpSize(SetBorderLerpSize {
                old_size: 512.0,
                new_size: 128.0,
                lerp_time: 6000,
            })
        );
    }

    #[test]
    fn decodes_set_border_size_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(2048.0);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_SIZE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderSize(SetBorderSize { size: 2048.0 })
        );
    }

    #[test]
    fn decodes_set_border_warning_delay_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(30);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderWarningDelay(SetBorderWarningDelay { warning_delay: 30 })
        );
    }

    #[test]
    fn decodes_set_border_warning_distance_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(8);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderWarningDistance(SetBorderWarningDistance {
                warning_blocks: 8
            })
        );
    }
}
