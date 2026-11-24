//! Source-decoded density functions and their named registry references.

use super::json::{self, fail, At, Obj, Result};
use super::noise::NoiseHolder;
use super::{Holder, Refs};
use bbb_registry_codec::registry_codec::DimensionTypeScalars;
use bbb_registry_codec::RegistryJsonValue as Json;

pub const DENSITY_FUNCTION_REGISTRY: &str = "minecraft:worldgen/density_function";
pub const MAX_DENSITY_DEPTH: usize = 128;
pub type DensityHolder = Holder<DensityFunction>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryKind {
    Add,
    Mul,
    Min,
    Max,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedKind {
    Abs,
    Square,
    Cube,
    HalfNegative,
    QuarterNegative,
    Invert,
    Squeeze,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Cache2d,
    CacheAllInCell,
    CacheOnce,
    FlatCache,
    Interpolated,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftKind {
    Plain,
    A,
    B,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RarityValueMapper {
    Type1,
    Type2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DensityFunction {
    Constant(f64),
    Beardifier,
    BlendAlpha,
    BlendOffset,
    EndIslands,
    YClampedGradient {
        from_y: i32,
        to_y: i32,
        from_value: f64,
        to_value: f64,
    },
    Binary {
        kind: BinaryKind,
        first: DensityHolder,
        second: DensityHolder,
    },
    Mapped {
        kind: MappedKind,
        input: DensityHolder,
    },
    Marker {
        kind: MarkerKind,
        input: DensityHolder,
    },
    Clamp {
        input: Box<Self>,
        min: f64,
        max: f64,
    },
    RangeChoice {
        input: DensityHolder,
        min_inclusive: f64,
        max_exclusive: f64,
        when_in_range: DensityHolder,
        when_out_of_range: DensityHolder,
    },
    FindTopSurface {
        density: DensityHolder,
        upper_bound: DensityHolder,
        lower_bound: i32,
        cell_height: i32,
    },
    BlendDensity(DensityHolder),
    Noise {
        noise: NoiseHolder,
        xz_scale: f64,
        y_scale: f64,
    },
    Shift {
        kind: ShiftKind,
        noise: NoiseHolder,
    },
    ShiftedNoise {
        shift_x: DensityHolder,
        shift_y: DensityHolder,
        shift_z: DensityHolder,
        xz_scale: f64,
        y_scale: f64,
        noise: NoiseHolder,
    },
    Spline(Spline),
    WeirdScaled {
        input: DensityHolder,
        noise: NoiseHolder,
        rarity: RarityValueMapper,
    },
    OldBlendedNoise {
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear: f64,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub enum Spline {
    Constant(f32),
    Multipoint {
        coordinate: DensityHolder,
        points: Vec<SplinePoint>,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub struct SplinePoint {
    pub location: f32,
    pub value: Spline,
    pub derivative: f32,
}

impl DensityHolder {
    pub fn decode_density(value: &Json) -> Result<Self> {
        Self::decode_at(value, 0)
    }
    fn decode_at(value: &Json, depth: usize) -> Result<Self> {
        Self::decode(value, |v| DensityFunction::decode_at(v, depth))
    }
    pub(super) fn refs(&self, refs: &mut Refs) {
        match self {
            Self::Named(id) => refs.element(DENSITY_FUNCTION_REGISTRY, id),
            Self::Inline(value) => value.refs(refs),
        }
    }
}
fn density_number(value: &Json) -> Result<f64> {
    json::double_in(value, -1_000_000.0, 1_000_000.0)
}
fn density_y(value: &Json) -> Result<i32> {
    json::int_in(
        value,
        DimensionTypeScalars::MIN_Y * 2,
        DimensionTypeScalars::MAX_Y * 2,
    )
}
fn check_depth(depth: usize) -> Result<()> {
    if depth > MAX_DENSITY_DEPTH {
        fail("density graph exceeds 128 nested nodes")
    } else {
        Ok(())
    }
}
impl DensityFunction {
    pub fn decode(value: &Json) -> Result<Self> {
        Self::decode_at(value, 0)
    }
    fn decode_at(value: &Json, depth: usize) -> Result<Self> {
        check_depth(depth)?;
        if matches!(value, Json::Number(_)) {
            return Ok(Self::Constant(density_number(value)?));
        }
        let obj = Obj::of(value)?;
        let kind = obj.type_id("type")?;
        let holder = |v: &Json| DensityHolder::decode_at(v, depth + 1);
        Ok(match kind.as_str() {
            "minecraft:constant" => Self::Constant(obj.req("argument", density_number)?),
            "minecraft:beardifier" => Self::Beardifier,
            "minecraft:blend_alpha" => Self::BlendAlpha,
            "minecraft:blend_offset" => Self::BlendOffset,
            "minecraft:end_islands" => Self::EndIslands,
            "minecraft:y_clamped_gradient" => Self::YClampedGradient {
                from_y: obj.req("from_y", density_y)?,
                to_y: obj.req("to_y", density_y)?,
                from_value: obj.req("from_value", density_number)?,
                to_value: obj.req("to_value", density_number)?,
            },
            "minecraft:add" | "minecraft:mul" | "minecraft:min" | "minecraft:max" => Self::Binary {
                kind: match kind.as_str() {
                    "minecraft:add" => BinaryKind::Add,
                    "minecraft:mul" => BinaryKind::Mul,
                    "minecraft:min" => BinaryKind::Min,
                    _ => BinaryKind::Max,
                },
                first: obj.req("argument1", holder)?,
                second: obj.req("argument2", holder)?,
            },
            "minecraft:abs"
            | "minecraft:square"
            | "minecraft:cube"
            | "minecraft:half_negative"
            | "minecraft:quarter_negative"
            | "minecraft:invert"
            | "minecraft:squeeze" => Self::Mapped {
                kind: match kind.as_str() {
                    "minecraft:abs" => MappedKind::Abs,
                    "minecraft:square" => MappedKind::Square,
                    "minecraft:cube" => MappedKind::Cube,
                    "minecraft:half_negative" => MappedKind::HalfNegative,
                    "minecraft:quarter_negative" => MappedKind::QuarterNegative,
                    "minecraft:invert" => MappedKind::Invert,
                    _ => MappedKind::Squeeze,
                },
                input: obj.req("argument", holder)?,
            },
            "minecraft:cache_2d"
            | "minecraft:cache_all_in_cell"
            | "minecraft:cache_once"
            | "minecraft:flat_cache"
            | "minecraft:interpolated" => Self::Marker {
                kind: match kind.as_str() {
                    "minecraft:cache_2d" => MarkerKind::Cache2d,
                    "minecraft:cache_all_in_cell" => MarkerKind::CacheAllInCell,
                    "minecraft:cache_once" => MarkerKind::CacheOnce,
                    "minecraft:flat_cache" => MarkerKind::FlatCache,
                    _ => MarkerKind::Interpolated,
                },
                input: obj.req("argument", holder)?,
            },
            // `DensityFunctions.java:466-475`: clamp's input is direct, not a holder.
            "minecraft:clamp" => Self::Clamp {
                input: Box::new(obj.req("input", |v| Self::decode_at(v, depth + 1))?),
                min: obj.req("min", density_number)?,
                max: obj.req("max", density_number)?,
            },
            "minecraft:range_choice" => Self::RangeChoice {
                input: obj.req("input", holder)?,
                min_inclusive: obj.req("min_inclusive", density_number)?,
                max_exclusive: obj.req("max_exclusive", density_number)?,
                when_in_range: obj.req("when_in_range", holder)?,
                when_out_of_range: obj.req("when_out_of_range", holder)?,
            },
            "minecraft:find_top_surface" => Self::FindTopSurface {
                density: obj.req("density", holder)?,
                upper_bound: obj.req("upper_bound", holder)?,
                lower_bound: obj.req("lower_bound", density_y)?,
                cell_height: obj.req("cell_height", |v| json::int_in(v, 1, i32::MAX))?,
            },
            "minecraft:blend_density" => Self::BlendDensity(obj.req("argument", holder)?),
            "minecraft:noise" => Self::Noise {
                noise: obj.req("noise", NoiseHolder::decode_noise)?,
                xz_scale: obj.req("xz_scale", json::double)?,
                y_scale: obj.req("y_scale", json::double)?,
            },
            "minecraft:shift" | "minecraft:shift_a" | "minecraft:shift_b" => Self::Shift {
                kind: match kind.as_str() {
                    "minecraft:shift" => ShiftKind::Plain,
                    "minecraft:shift_a" => ShiftKind::A,
                    _ => ShiftKind::B,
                },
                noise: obj.req("argument", NoiseHolder::decode_noise)?,
            },
            "minecraft:shifted_noise" => Self::ShiftedNoise {
                shift_x: obj.req("shift_x", holder)?,
                shift_y: obj.req("shift_y", holder)?,
                shift_z: obj.req("shift_z", holder)?,
                xz_scale: obj.req("xz_scale", json::double)?,
                y_scale: obj.req("y_scale", json::double)?,
                noise: obj.req("noise", NoiseHolder::decode_noise)?,
            },
            "minecraft:spline" => {
                Self::Spline(obj.req("spline", |v| Spline::decode(v, depth + 1))?)
            }
            "minecraft:weird_scaled_sampler" => Self::WeirdScaled {
                input: obj.req("input", holder)?,
                noise: obj.req("noise", NoiseHolder::decode_noise)?,
                rarity: obj.req("rarity_value_mapper", |v| match json::string(v)?.as_str() {
                    "type_1" => Ok(RarityValueMapper::Type1),
                    "type_2" => Ok(RarityValueMapper::Type2),
                    _ => fail("unknown rarity mapper"),
                })?,
            },
            "minecraft:old_blended_noise" => Self::OldBlendedNoise {
                xz_scale: obj.req("xz_scale", |v| json::double_in(v, 0.001, 1000.0))?,
                y_scale: obj.req("y_scale", |v| json::double_in(v, 0.001, 1000.0))?,
                xz_factor: obj.req("xz_factor", |v| json::double_in(v, 0.001, 1000.0))?,
                y_factor: obj.req("y_factor", |v| json::double_in(v, 0.001, 1000.0))?,
                smear: obj.req("smear_scale_multiplier", |v| json::double_in(v, 1.0, 8.0))?,
            },
            _ => return fail(format!("unknown density function type {kind}")),
        })
    }
    pub(super) fn refs(&self, refs: &mut Refs) {
        match self {
            Self::Binary { first, second, .. } => {
                first.refs(refs);
                second.refs(refs);
            }
            Self::Mapped { input, .. } | Self::Marker { input, .. } | Self::BlendDensity(input) => {
                input.refs(refs)
            }
            Self::Clamp { input, .. } => input.refs(refs),
            Self::RangeChoice {
                input,
                when_in_range,
                when_out_of_range,
                ..
            } => {
                input.refs(refs);
                when_in_range.refs(refs);
                when_out_of_range.refs(refs);
            }
            Self::FindTopSurface {
                density,
                upper_bound,
                ..
            } => {
                density.refs(refs);
                upper_bound.refs(refs);
            }
            Self::Noise { noise, .. } | Self::Shift { noise, .. } => noise.refs(refs),
            Self::ShiftedNoise {
                shift_x,
                shift_y,
                shift_z,
                noise,
                ..
            } => {
                shift_x.refs(refs);
                shift_y.refs(refs);
                shift_z.refs(refs);
                noise.refs(refs);
            }
            Self::Spline(spline) => spline.refs(refs),
            Self::WeirdScaled { input, noise, .. } => {
                input.refs(refs);
                noise.refs(refs);
            }
            _ => {}
        }
    }
}
impl Spline {
    fn decode(value: &Json, depth: usize) -> Result<Self> {
        check_depth(depth)?;
        // `CubicSpline.java:29-34` and `CubicSpline.java:61-67` use FLOAT directly.
        if matches!(value, Json::Number(_)) {
            return Ok(Self::Constant(json::float(value)?));
        }
        let obj = Obj::of(value)?;
        let coordinate = obj.req("coordinate", |v| DensityHolder::decode_at(v, depth + 1))?;
        let points = obj.req("points", |v| {
            json::bounded_list(v, 1, 1024, |v| {
                let obj = Obj::of(v)?;
                Ok(SplinePoint {
                    location: obj.req("location", json::float)?,
                    value: obj.req("value", |v| Self::decode(v, depth + 1))?,
                    derivative: obj.req("derivative", json::float)?,
                })
            })
        })?;
        if points
            .windows(2)
            .any(|pair| pair[0].location >= pair[1].location)
        {
            return fail("spline locations must be strictly increasing").at("points");
        }
        Ok(Self::Multipoint { coordinate, points })
    }
    fn refs(&self, refs: &mut Refs) {
        if let Self::Multipoint { coordinate, points } = self {
            coordinate.refs(refs);
            for point in points {
                point.value.refs(refs);
            }
        }
    }
}
