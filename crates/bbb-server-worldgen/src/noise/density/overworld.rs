//! Runtime values shared by the Overworld density codecs
//! (`DensityFunctions.java:861-1365`, `CubicSpline.java:245-277`).

use std::sync::Arc;

use super::NodeId;
use crate::noise::NormalNoise;

use bbb_server_datapack::worldgen::density::{MappedKind, RarityValueMapper, ShiftKind};

pub fn apply_mapped(kind: MappedKind, input: f64) -> f64 {
    match kind {
        MappedKind::Abs => input.abs(),
        MappedKind::Square => input * input,
        MappedKind::Cube => input * input * input,
        MappedKind::HalfNegative if input <= 0.0 => input * 0.5,
        MappedKind::QuarterNegative if input <= 0.0 => input * 0.25,
        MappedKind::HalfNegative | MappedKind::QuarterNegative => input,
        MappedKind::Invert => 1.0 / input,
        MappedKind::Squeeze => {
            let clamped = input.clamp(-1.0, 1.0);
            clamped / 2.0 - clamped * clamped * clamped / 24.0
        }
    }
}

pub struct NoiseSampler {
    pub noise: Arc<NormalNoise>,
    pub xz_scale: f64,
    pub y_scale: f64,
}

impl NoiseSampler {
    pub fn sample(&self, x: i32, y: i32, z: i32) -> f64 {
        self.noise.get_value(
            f64::from(x) * self.xz_scale,
            f64::from(y) * self.y_scale,
            f64::from(z) * self.xz_scale,
        )
    }
}

pub struct ShiftSampler {
    pub kind: ShiftKind,
    pub noise: Arc<NormalNoise>,
}

impl ShiftSampler {
    pub fn sample(&self, x: i32, y: i32, z: i32) -> f64 {
        let (x, y, z) = match self.kind {
            // `DensityFunctions.Shift.compute` and `ShiftNoise.sample`
            // (`DensityFunctions.java:982-1000`, `DensityFunctions.java:1045-1065`).
            ShiftKind::Plain => (f64::from(x), f64::from(y), f64::from(z)),
            ShiftKind::A => (f64::from(x), 0.0, f64::from(z)),
            ShiftKind::B => (f64::from(z), f64::from(x), 0.0),
        };
        self.noise.get_value(x * 0.25, y * 0.25, z * 0.25) * 4.0
    }
}

pub struct ShiftedNoiseSampler {
    pub shift_x: NodeId,
    pub shift_y: NodeId,
    pub shift_z: NodeId,
    pub xz_scale: f64,
    pub y_scale: f64,
    pub noise: Arc<NormalNoise>,
}

impl ShiftedNoiseSampler {
    pub fn sample(&self, x: i32, y: i32, z: i32, mut shift: impl FnMut(NodeId) -> f64) -> f64 {
        let shifted_x = f64::from(x) * self.xz_scale + shift(self.shift_x);
        let shifted_y = f64::from(y) * self.y_scale + shift(self.shift_y);
        let shifted_z = f64::from(z) * self.xz_scale + shift(self.shift_z);
        self.noise.get_value(shifted_x, shifted_y, shifted_z)
    }
}

fn map_rarity(mapper: RarityValueMapper, value: f64) -> f64 {
    match mapper {
        RarityValueMapper::Type1 if value < -0.5 => 0.75,
        RarityValueMapper::Type1 if value < 0.0 => 1.0,
        RarityValueMapper::Type1 if value < 0.5 => 1.5,
        RarityValueMapper::Type1 => 2.0,
        RarityValueMapper::Type2 if value < -0.75 => 0.5,
        RarityValueMapper::Type2 if value < -0.5 => 0.75,
        RarityValueMapper::Type2 if value < 0.5 => 1.0,
        RarityValueMapper::Type2 if value < 0.75 => 2.0,
        RarityValueMapper::Type2 => 3.0,
    }
}

pub struct WeirdScaledSampler {
    pub input: NodeId,
    pub noise: Arc<NormalNoise>,
    pub rarity: RarityValueMapper,
}

impl WeirdScaledSampler {
    pub fn sample(&self, x: i32, y: i32, z: i32, input: f64) -> f64 {
        let rarity = map_rarity(self.rarity, input);
        rarity
            * self
                .noise
                .get_value(
                    f64::from(x) / rarity,
                    f64::from(y) / rarity,
                    f64::from(z) / rarity,
                )
                .abs()
    }
}

pub enum Spline {
    Constant(f32),
    Multipoint {
        coordinate: NodeId,
        points: Vec<SplinePoint>,
    },
}

pub struct SplinePoint {
    pub location: f32,
    pub value: Spline,
    pub derivative: f32,
}

impl Spline {
    /// `CubicSpline.Multipoint.apply` with the coordinate function evaluated by
    /// the caller (`CubicSpline.java`).
    pub fn evaluate(&self, coordinate_value: &mut dyn FnMut(NodeId) -> f64) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Multipoint { coordinate, points } => {
                let input = coordinate_value(*coordinate) as f32;
                let upper = points.partition_point(|point| input >= point.location);
                if upper == 0 {
                    return linear_extend(input, &points[0], coordinate_value);
                }
                if upper == points.len() {
                    return linear_extend(input, &points[points.len() - 1], coordinate_value);
                }
                let first = &points[upper - 1];
                let second = &points[upper];
                let span = second.location - first.location;
                let factor = (input - first.location) / span;
                let first_value = first.value.evaluate(coordinate_value);
                let second_value = second.value.evaluate(coordinate_value);
                let delta = second_value - first_value;
                let first_control = first.derivative * span - delta;
                let second_control = -second.derivative * span + delta;
                lerp(factor, first_value, second_value)
                    + factor * (1.0 - factor) * lerp(factor, first_control, second_control)
            }
        }
    }

    pub fn visit_coordinates(&self, visit: &mut dyn FnMut(NodeId)) {
        if let Self::Multipoint { coordinate, points } = self {
            visit(*coordinate);
            for point in points {
                point.value.visit_coordinates(visit);
            }
        }
    }
}

fn linear_extend(
    input: f32,
    point: &SplinePoint,
    coordinate_value: &mut dyn FnMut(NodeId) -> f64,
) -> f32 {
    let value = point.value.evaluate(coordinate_value);
    if point.derivative == 0.0 {
        value
    } else {
        value + point.derivative * (input - point.location)
    }
}

fn lerp(factor: f32, start: f32, end: f32) -> f32 {
    start + factor * (end - start)
}
