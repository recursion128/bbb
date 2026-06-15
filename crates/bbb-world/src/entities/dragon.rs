use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{EntityPickBoundsState, EntityPickTargetState, EntityTransform, EntityVec3};

pub(crate) const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;
pub(crate) const ENDER_DRAGON_PHASE_DATA_ID: u8 = 16;
pub(crate) const ENDER_DRAGON_PHASE_HOVERING_ID: i32 = 10;

const DRAGON_FLIGHT_HISTORY_LEN: usize = 64;
const DRAGON_FLIGHT_HISTORY_MASK: i32 = 63;
const ENDER_DRAGON_SITTING_PHASE_IDS: &[i32] = &[5, 6, 7, ENDER_DRAGON_PHASE_HOVERING_ID];

const ENDER_DRAGON_PARTS: &[EnderDragonPartSpec] = &[
    EnderDragonPartSpec {
        id_offset: 1,
        width: 1.0,
        height: 1.0,
        offset: EnderDragonPartOffset::Head,
    },
    EnderDragonPartSpec {
        id_offset: 2,
        width: 3.0,
        height: 3.0,
        offset: EnderDragonPartOffset::Neck,
    },
    EnderDragonPartSpec {
        id_offset: 3,
        width: 5.0,
        height: 3.0,
        offset: EnderDragonPartOffset::Body,
    },
    EnderDragonPartSpec {
        id_offset: 4,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(0),
    },
    EnderDragonPartSpec {
        id_offset: 5,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(1),
    },
    EnderDragonPartSpec {
        id_offset: 6,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(2),
    },
    EnderDragonPartSpec {
        id_offset: 7,
        width: 4.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Wing(1.0),
    },
    EnderDragonPartSpec {
        id_offset: 8,
        width: 4.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Wing(-1.0),
    },
];

#[derive(Debug, Clone, Copy)]
struct EnderDragonPartSpec {
    id_offset: i32,
    width: f32,
    height: f32,
    offset: EnderDragonPartOffset,
}

#[derive(Debug, Clone, Copy)]
enum EnderDragonPartOffset {
    Head,
    Neck,
    Body,
    Tail(u8),
    Wing(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnderDragonAnimationState {
    #[serde(default = "default_ender_dragon_phase_id")]
    pub phase_id: i32,
    #[serde(default)]
    pub y_rot_a: f32,
    #[serde(default)]
    pub flight_history: DragonFlightHistoryState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragonFlightHistoryState {
    pub samples: [DragonFlightHistorySample; DRAGON_FLIGHT_HISTORY_LEN],
    pub head: i8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct DragonFlightHistorySample {
    pub y: f64,
    pub y_rot: f32,
}

#[derive(Deserialize)]
struct DragonFlightHistoryStateWire {
    #[serde(default)]
    samples: Vec<DragonFlightHistorySample>,
    #[serde(default = "default_flight_history_head")]
    head: i8,
}

impl Default for EnderDragonAnimationState {
    fn default() -> Self {
        Self {
            phase_id: default_ender_dragon_phase_id(),
            y_rot_a: 0.0,
            flight_history: DragonFlightHistoryState::default(),
        }
    }
}

impl Default for DragonFlightHistoryState {
    fn default() -> Self {
        Self {
            samples: [DragonFlightHistorySample::default(); DRAGON_FLIGHT_HISTORY_LEN],
            head: -1,
        }
    }
}

impl Serialize for DragonFlightHistoryState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("DragonFlightHistoryState", 2)?;
        state.serialize_field("samples", &self.samples.as_slice())?;
        state.serialize_field("head", &self.head)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for DragonFlightHistoryState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DragonFlightHistoryStateWire::deserialize(deserializer)?;
        let mut samples = [DragonFlightHistorySample::default(); DRAGON_FLIGHT_HISTORY_LEN];
        for (sample, value) in samples.iter_mut().zip(wire.samples) {
            *sample = value;
        }
        let head = if (-1..DRAGON_FLIGHT_HISTORY_LEN as i32).contains(&i32::from(wire.head)) {
            wire.head
        } else {
            default_flight_history_head()
        };
        Ok(Self { samples, head })
    }
}

fn default_flight_history_head() -> i8 {
    -1
}

fn default_ender_dragon_phase_id() -> i32 {
    ENDER_DRAGON_PHASE_HOVERING_ID
}

impl EnderDragonAnimationState {
    pub(crate) fn set_phase(&mut self, phase_id: i32) {
        self.phase_id = phase_id;
    }

    pub(crate) fn advance_client_tick(&mut self, transform: EntityTransform) {
        self.flight_history
            .record(transform.position.y, transform.y_rot);
    }

    fn is_sitting(self) -> bool {
        ENDER_DRAGON_SITTING_PHASE_IDS.contains(&self.phase_id)
    }
}

impl DragonFlightHistoryState {
    fn record(&mut self, y: f64, y_rot: f32) {
        let sample = DragonFlightHistorySample { y, y_rot };
        if self.head < 0 {
            self.samples.fill(sample);
        }

        let next = i32::from(self.head) + 1;
        self.head = if next == DRAGON_FLIGHT_HISTORY_LEN as i32 {
            0
        } else {
            next as i8
        };
        self.samples[self.head as usize] = sample;
    }

    fn get(self, delay: i32) -> DragonFlightHistorySample {
        if self.head < 0 {
            return DragonFlightHistorySample::default();
        }
        let index = (i32::from(self.head) - delay) & DRAGON_FLIGHT_HISTORY_MASK;
        self.samples[index as usize]
    }

    fn get_interpolated(self, delay: i32, partial_ticks: f32) -> DragonFlightHistorySample {
        let sample = self.get(delay);
        let sample_old = self.get(delay + 1);
        DragonFlightHistorySample {
            y: sample_old.y + (sample.y - sample_old.y) * f64::from(partial_ticks),
            y_rot: sample_old.y_rot + wrap_degrees(sample.y_rot - sample_old.y_rot) * partial_ticks,
        }
    }
}

pub(crate) fn ender_dragon_part_pick_targets_at_partial_tick(
    parent_id: i32,
    transform: EntityTransform,
    animation: Option<EnderDragonAnimationState>,
    partial_ticks: f32,
) -> Vec<EntityPickTargetState> {
    let animation = animation.unwrap_or_default();
    ENDER_DRAGON_PARTS
        .iter()
        .map(|part| EntityPickTargetState {
            entity_id: parent_id + part.id_offset,
            position: part_position(transform, part.offset, animation, partial_ticks),
            bounds: EntityPickBoundsState::from_base_size(part.width, part.height, 0.0),
        })
        .collect()
}

fn part_position(
    transform: EntityTransform,
    offset: EnderDragonPartOffset,
    animation: EnderDragonAnimationState,
    partial_ticks: f32,
) -> EntityVec3 {
    let yaw = f64::from(transform.y_rot).to_radians();
    let sin_yaw = yaw.sin();
    let cos_yaw = yaw.cos();
    let sample_0 = animation.flight_history.get_interpolated(0, partial_ticks);
    let sample_5 = animation.flight_history.get_interpolated(5, partial_ticks);
    let sample_10 = animation.flight_history.get_interpolated(10, partial_ticks);
    let tilt = (sample_5.y - sample_10.y) * 10.0 * std::f64::consts::PI / 180.0;
    let cos_tilt = tilt.cos();
    let sin_tilt = tilt.sin();
    let head_y_offset = if animation.is_sitting() {
        -1.0
    } else {
        sample_5.y - sample_0.y
    };
    let (x, y, z) = match offset {
        EnderDragonPartOffset::Head => head_part_offset(
            yaw,
            animation.y_rot_a,
            cos_tilt,
            sin_tilt,
            head_y_offset,
            6.5,
        ),
        EnderDragonPartOffset::Neck => head_part_offset(
            yaw,
            animation.y_rot_a,
            cos_tilt,
            sin_tilt,
            head_y_offset,
            5.5,
        ),
        EnderDragonPartOffset::Body => (sin_yaw * 0.5, 0.0, -cos_yaw * 0.5),
        EnderDragonPartOffset::Tail(index) => {
            let distance = f64::from(index + 1) * 2.0;
            let p1 = sample_5;
            let p0 = animation
                .flight_history
                .get_interpolated(12 + i32::from(index) * 2, partial_ticks);
            let tail_yaw = yaw + f64::from(wrap_degrees(p0.y_rot - p1.y_rot)).to_radians();
            let tail_sin = tail_yaw.sin();
            let tail_cos = tail_yaw.cos();
            (
                -(sin_yaw * 1.5 + tail_sin * distance) * cos_tilt,
                p0.y - p1.y - (distance + 1.5) * sin_tilt + 1.5,
                (cos_yaw * 1.5 + tail_cos * distance) * cos_tilt,
            )
        }
        EnderDragonPartOffset::Wing(side) => (cos_yaw * 4.5 * side, 2.0, sin_yaw * 4.5 * side),
    };
    EntityVec3 {
        x: transform.position.x + x,
        y: transform.position.y + y,
        z: transform.position.z + z,
    }
}

fn head_part_offset(
    yaw: f64,
    y_rot_a: f32,
    cos_tilt: f64,
    sin_tilt: f64,
    head_y_offset: f64,
    distance: f64,
) -> (f64, f64, f64) {
    let head_yaw = yaw - f64::from(y_rot_a) * 0.01;
    (
        head_yaw.sin() * distance * cos_tilt,
        head_y_offset + sin_tilt * distance,
        -head_yaw.cos() * distance * cos_tilt,
    )
}

fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragon_flight_history_interpolates_like_vanilla() {
        let mut history = DragonFlightHistoryState::default();
        history.record(64.0, 170.0);
        history.record(74.0, -170.0);

        assert_eq!(
            history.get_interpolated(0, 0.0),
            DragonFlightHistorySample {
                y: 64.0,
                y_rot: 170.0,
            }
        );
        assert_eq!(
            history.get_interpolated(0, 0.5),
            DragonFlightHistorySample {
                y: 69.0,
                y_rot: 180.0,
            }
        );
        assert_eq!(
            history.get_interpolated(0, 1.0),
            DragonFlightHistorySample {
                y: 74.0,
                y_rot: 190.0,
            }
        );
    }
}
