//! Compiled dimension environment-attribute layers.

use crate::startup::DatapackStartup;
use bbb_facts::environment_attribute_types::{
    VanillaEnvironmentAttribute, VanillaEnvironmentAttributeRange, VanillaEnvironmentScalarValue,
};
use bbb_protocol::nbt_summary::value_field as field;
use bbb_protocol::NbtSummaryValue as Nbt;
use bbb_registry_codec::registry_codec::{
    canonical_biome_environment_attributes_with_content_registry,
    canonical_direct_dimension_type_with_content_registry,
    canonical_direct_timeline_with_content_registry,
};
use std::collections::BTreeMap;

const TIMELINE: &str = "minecraft:timeline";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentAttributeError(pub String);
impl std::fmt::Display for EnvironmentAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for EnvironmentAttributeError {}
type Result<T> = std::result::Result<T, EnvironmentAttributeError>;
fn error(message: impl Into<String>) -> EnvironmentAttributeError {
    EnvironmentAttributeError(message.into())
}

fn required<'a>(value: &'a Nbt, key: &str) -> Result<&'a Nbt> {
    field(value, key).ok_or_else(|| error(format!("missing {key}")))
}
fn string(value: &Nbt) -> Result<String> {
    if let Nbt::String(value) = value {
        value
            .try_to_string()
            .map_err(|_| error("expected Unicode text"))
    } else {
        Err(error("expected string"))
    }
}
fn timeline_ids(dimension: &Nbt, startup: &DatapackStartup) -> Result<Vec<String>> {
    match field(dimension, "timelines") {
        None => Ok(Vec::new()),
        Some(value @ Nbt::String(_)) => {
            let id = string(value)?;
            Ok(match id.strip_prefix('#') {
                Some(tag) => startup
                    .tags
                    .registry(TIMELINE)
                    .and_then(|registry| registry.tag(tag))
                    .unwrap_or_default()
                    .to_vec(),
                None => vec![id],
            })
        }
        Some(Nbt::List(values)) => values.iter().map(string).collect(),
        Some(_) => Err(error("invalid timeline holders")),
    }
}

pub struct EnvironmentAttributeSources<'a> {
    startup: &'a DatapackStartup,
    dimensions: BTreeMap<&'a str, DimensionSource>,
    biomes: Vec<(&'a str, Nbt)>,
    timelines: BTreeMap<String, Nbt>,
}

struct DimensionSource {
    value: Nbt,
    timeline_ids: Vec<String>,
}

impl<'a> EnvironmentAttributeSources<'a> {
    pub fn new(startup: &'a DatapackStartup) -> Result<Self> {
        let dimensions = startup
            .registries
            .registry("minecraft:dimension_type")
            .ok_or_else(|| error("missing dimension registry"))?
            .elements
            .iter()
            .map(|element| {
                let source = element
                    .decoded_direct_source()
                    .map_err(|e| error(e.to_string()))?;
                let value = canonical_direct_dimension_type_with_content_registry(
                    &source,
                    &startup.content_registry,
                )
                .map_err(|e| error(e.to_string()))?;
                let timeline_ids = timeline_ids(&value, startup)?;
                Ok((
                    element.id.as_str(),
                    DimensionSource {
                        value,
                        timeline_ids,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let biomes = startup
            .registries
            .registry("minecraft:worldgen/biome")
            .ok_or_else(|| error("missing biome registry"))?
            .elements
            .iter()
            .map(|element| {
                let source = element
                    .decoded_direct_source()
                    .map_err(|e| error(format!("{}: {e}", element.id)))?;
                let value = canonical_biome_environment_attributes_with_content_registry(
                    &source,
                    &startup.content_registry,
                )
                .map_err(|e| error(format!("{}: {e}", element.id)))?;
                Ok((element.id.as_str(), value))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut timelines = BTreeMap::new();
        for dimension in dimensions.values() {
            for id in &dimension.timeline_ids {
                if !timelines.contains_key(id) {
                    let element = startup
                        .registries
                        .element(TIMELINE, id)
                        .ok_or_else(|| error(format!("missing timeline {id}")))?;
                    let source = element
                        .decoded_direct_source()
                        .map_err(|e| error(format!("{id}: {e}")))?;
                    let value = canonical_direct_timeline_with_content_registry(
                        &source,
                        &startup.content_registry,
                    )
                    .map_err(|e| error(format!("{id}: {e}")))?;
                    timelines.insert(id.clone(), value);
                }
            }
        }
        Ok(Self {
            startup,
            dimensions,
            biomes,
            timelines,
        })
    }

    pub fn dimension(&self, id: &str) -> Result<&Nbt> {
        Ok(&self.dimension_source(id)?.value)
    }

    fn dimension_source(&self, id: &str) -> Result<&DimensionSource> {
        self.dimensions
            .get(id)
            .ok_or_else(|| error(format!("missing dimension {id}")))
    }
}

fn float(value: &Nbt) -> Result<f32> {
    if let Nbt::Float(value) = value {
        Ok(f32::from_bits(*value))
    } else {
        Err(error("expected float"))
    }
}
fn integer(value: &Nbt) -> Result<i32> {
    if let Nbt::Int(value) = value {
        Ok(*value)
    } else {
        Err(error("expected integer"))
    }
}
fn lerp(alpha: f32, from: f32, to: f32) -> f32 {
    from + alpha * (to - from)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Modifier {
    Override,
    Add,
    Subtract,
    Multiply,
    Minimum,
    Maximum,
    AlphaBlend,
}
impl Modifier {
    fn parse(name: &str) -> Result<Self> {
        Ok(match name {
            "override" => Self::Override,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "minimum" => Self::Minimum,
            "maximum" => Self::Maximum,
            "alpha_blend" => Self::AlphaBlend,
            _ => return Err(error(format!("unknown float modifier {name}"))),
        })
    }
    fn argument(self, value: &Nbt) -> Result<Argument> {
        if self == Self::AlphaBlend && matches!(value, Nbt::Compound(_)) {
            Ok(Argument {
                value: float(required(value, "value")?)?,
                alpha: field(value, "alpha").map(float).transpose()?.unwrap_or(1.0),
            })
        } else {
            Ok(Argument {
                value: float(value)?,
                alpha: 1.0,
            })
        }
    }
    // `FloatModifier.java:9-34`.
    fn apply(self, subject: f32, arg: Argument) -> f32 {
        match self {
            Self::Override => arg.value,
            Self::Add => subject + arg.value,
            Self::Subtract => subject - arg.value,
            Self::Multiply => subject * arg.value,
            Self::Minimum => {
                if subject.is_nan() || arg.value.is_nan() {
                    f32::NAN
                } else {
                    subject.min(arg.value)
                }
            }
            Self::Maximum => {
                if subject.is_nan() || arg.value.is_nan() {
                    f32::NAN
                } else {
                    subject.max(arg.value)
                }
            }
            Self::AlphaBlend => lerp(arg.alpha, subject, arg.value),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Argument {
    value: f32,
    alpha: f32,
}
impl Argument {
    fn interpolate(self, to: Self, alpha: f32) -> Self {
        Self {
            value: lerp(alpha, self.value, to.value),
            alpha: lerp(alpha, self.alpha, to.alpha),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
struct Track {
    clock: String,
    modifier: Modifier,
    period: Option<i32>,
    easing: Easing,
    segments: Vec<Segment<Argument>>,
}
#[derive(Debug, Clone, PartialEq)]
struct Segment<A> {
    from: i32,
    to: i32,
    a: A,
    b: A,
}
impl Track {
    fn compile(timeline: &Nbt, track: &Nbt) -> Result<Self> {
        let clock = string(required(timeline, "clock")?)?;
        let period = field(timeline, "period_ticks").map(integer).transpose()?;
        let modifier = field(track, "modifier")
            .map(string)
            .transpose()?
            .unwrap_or_else(|| "override".into());
        let modifier = Modifier::parse(&modifier)?;
        let easing = Easing::compile(field(track, "ease"))?;
        let Nbt::List(frames) = required(track, "keyframes")? else {
            return Err(error("expected keyframe list"));
        };
        let frames = frames
            .iter()
            .map(|f| {
                Ok((
                    integer(required(f, "ticks")?)?,
                    modifier.argument(required(f, "value")?)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let segments = compile_segments(&frames, period)?;
        Ok(Self {
            clock,
            modifier,
            period,
            easing,
            segments,
        })
    }
    fn sample(&self, ticks: i64) -> Argument {
        sample_segments(
            &self.segments,
            self.period,
            ticks,
            |value| *value,
            |a, b, alpha| a.interpolate(b, self.easing.apply(alpha)),
        )
    }
}

// KeyframeTrackSampler.java:19-46.
fn compile_segments<A: Clone>(frames: &[(i32, A)], period: Option<i32>) -> Result<Vec<Segment<A>>> {
    let (first_tick, first) = frames.first().ok_or_else(|| error("empty keyframes"))?;
    let (last_tick, last) = frames.last().unwrap();
    let mut segments = Vec::new();
    if frames.len() == 1 {
        segments.push(Segment {
            from: 0,
            to: 0,
            a: first.clone(),
            b: first.clone(),
        });
    } else {
        if let Some(period) = period {
            segments.push(Segment {
                from: last_tick.wrapping_sub(period),
                to: *first_tick,
                a: last.clone(),
                b: first.clone(),
            });
        }
        segments.extend(frames.windows(2).map(|pair| Segment {
            from: pair[0].0,
            to: pair[1].0,
            a: pair[0].1.clone(),
            b: pair[1].1.clone(),
        }));
        if let Some(period) = period {
            segments.push(Segment {
                from: *last_tick,
                to: first_tick.wrapping_add(period),
                a: last.clone(),
                b: first.clone(),
            });
        }
    }
    Ok(segments)
}

// KeyframeTrackSampler.java:50-77.
fn sample_segments<'a, A, V>(
    segments: &'a [Segment<A>],
    period: Option<i32>,
    ticks: i64,
    value: impl Fn(&'a A) -> V,
    interpolate: impl FnOnce(V, V, f32) -> V,
) -> V {
    let ticks = period.map_or(ticks, |period| ticks.rem_euclid(i64::from(period)));
    let segment = segments
        .iter()
        .find(|s| ticks < i64::from(s.to))
        .unwrap_or_else(|| segments.last().unwrap());
    if ticks <= i64::from(segment.from) {
        return value(&segment.a);
    }
    if ticks >= i64::from(segment.to) {
        return value(&segment.b);
    }
    let alpha =
        (ticks - i64::from(segment.from)) as f32 / segment.to.wrapping_sub(segment.from) as f32;
    interpolate(value(&segment.a), value(&segment.b), alpha)
}

/// One float attribute with ordered dimension, biome and timeline layers.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionFloatAttribute {
    base: f32,
    biomes: BTreeMap<String, Option<(Modifier, Argument)>>,
    tracks: Vec<Track>,
    has_biome_layer: bool,
    range: VanillaEnvironmentAttributeRange,
}

fn float_modifier(value: &Nbt) -> Result<(Modifier, Argument)> {
    if let Some(name) = field(value, "modifier") {
        let modifier = Modifier::parse(&string(name)?)?;
        Ok((modifier, modifier.argument(required(value, "argument")?)?))
    } else {
        Ok((Modifier::Override, Modifier::Override.argument(value)?))
    }
}

impl DimensionFloatAttribute {
    pub fn from_attribute(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
        attribute: &'static VanillaEnvironmentAttribute,
    ) -> Result<Self> {
        let Some(VanillaEnvironmentScalarValue::FloatBits(default)) = attribute.scalar_default
        else {
            return Err(error(format!(
                "{} is not a float attribute",
                attribute.resource_id
            )));
        };
        let range = attribute.value_range;
        let attribute = attribute.resource_id;
        let dimension = sources.dimension_source(dimension_type_id)?;
        let mut base = f32::from_bits(default);
        if let Some(value) =
            field(&dimension.value, "attributes").and_then(|map| field(map, attribute))
        {
            let (modifier, argument) = float_modifier(value)?;
            base = modifier.apply(base, argument);
        }
        let biomes = sources
            .biomes
            .iter()
            .map(|(id, attributes)| {
                Ok((
                    (*id).to_owned(),
                    field(attributes, attribute)
                        .map(float_modifier)
                        .transpose()?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let mut tracks = Vec::new();
        for id in &dimension.timeline_ids {
            let timeline = &sources.timelines[id];
            if let Some(track) = field(timeline, "tracks").and_then(|map| field(map, attribute)) {
                tracks.push(Track::compile(timeline, track)?);
            }
        }
        let has_biome_layer = biomes.values().any(Option::is_some);
        Ok(Self {
            base,
            biomes,
            tracks,
            has_biome_layer,
            range,
        })
    }

    pub fn varies_by_biome(&self) -> bool {
        self.has_biome_layer
    }

    // EnvironmentAttributeSystem.java:62-96.
    /// `None` skips the positional biome layer for a dimension-only sample.
    pub fn sample_unclamped(
        &self,
        biome_id: Option<&str>,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<f32> {
        let mut value = self.base;
        if self.has_biome_layer {
            if let Some(biome_id) = biome_id {
                if let Some((modifier, argument)) = self.biomes.get(biome_id)? {
                    value = modifier.apply(value, *argument);
                }
            }
        }
        for track in &self.tracks {
            value = track
                .modifier
                .apply(value, track.sample(clock_ticks(&track.clock)?));
        }
        Some(value)
    }

    pub fn sanitize(&self, value: f32) -> f32 {
        self.range.sanitize_float(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Easing {
    Simple(String),
    Bezier([f32; 4]),
}
impl Easing {
    fn compile(value: Option<&Nbt>) -> Result<Self> {
        match value {
            None => Ok(Self::Simple("linear".into())),
            Some(value @ Nbt::String(_)) => Ok(Self::Simple(string(value)?)),
            Some(value) => {
                let Nbt::List(values) = required(value, "cubic_bezier")? else {
                    return Err(error("invalid easing"));
                };
                Ok(Self::Bezier([
                    float(&values[0])?,
                    float(&values[1])?,
                    float(&values[2])?,
                    float(&values[3])?,
                ]))
            }
        }
    }
    fn apply(&self, x: f32) -> f32 {
        match self {
            Self::Simple(name) => simple_easing(name, x).unwrap(),
            Self::Bezier([x1, y1, x2, y2]) => sample_cubic_bezier(x, *x1, *y1, *x2, *y2),
        }
    }
}

// `EasingType.java:13-44`; `Ease.java:4-169`.
fn simple_easing(name: &str, x: f32) -> Option<f32> {
    Some(match name {
        "constant" => 0.0,
        "linear" => x,
        "in_back" => x * x * (2.70158 * x - 1.70158),
        "in_bounce" => 1.0 - out_bounce(1.0 - x),
        "in_circ" => -(1.0 - x * x).sqrt() + 1.0,
        "in_cubic" => x * x * x,
        "in_elastic" => in_elastic(x),
        "in_expo" => {
            if x == 0.0 {
                0.0
            } else {
                2.0_f64.powf(10.0 * f64::from(x) - 10.0) as f32
            }
        }
        "in_quad" => x * x,
        "in_quart" => (x * x) * (x * x),
        "in_quint" => (x * x) * (x * x) * x,
        "in_sine" => 1.0 - table_trig(x * std::f32::consts::FRAC_PI_2, true),
        "in_out_back" => in_out_back(x),
        "in_out_bounce" => {
            if x < 0.5 {
                (1.0 - out_bounce(1.0 - 2.0 * x)) / 2.0
            } else {
                (1.0 + out_bounce(2.0 * x - 1.0)) / 2.0
            }
        }
        "in_out_circ" => {
            if x < 0.5 {
                ((1.0 - (1.0 - (2.0 * f64::from(x)).powi(2)).sqrt()) / 2.0) as f32
            } else {
                (((1.0 - (-2.0 * f64::from(x) + 2.0).powi(2)).sqrt() + 1.0) / 2.0) as f32
            }
        }
        "in_out_cubic" => {
            if x < 0.5 {
                4.0 * (x * x * x)
            } else {
                (1.0 - (-2.0 * f64::from(x) + 2.0).powi(3) / 2.0) as f32
            }
        }
        "in_out_elastic" => in_out_elastic(x),
        "in_out_expo" => {
            if x < 0.5 {
                if x == 0.0 {
                    0.0
                } else {
                    (2.0_f64.powf(20.0 * f64::from(x) - 10.0) / 2.0) as f32
                }
            } else if x == 1.0 {
                1.0
            } else {
                ((2.0 - 2.0_f64.powf(-20.0 * f64::from(x) + 10.0)) / 2.0) as f32
            }
        }
        "in_out_quad" => {
            if x < 0.5 {
                2.0 * (x * x)
            } else {
                (1.0 - (-2.0 * f64::from(x) + 2.0).powi(2) / 2.0) as f32
            }
        }
        "in_out_quart" => {
            if x < 0.5 {
                8.0 * ((x * x) * (x * x))
            } else {
                (1.0 - (-2.0 * f64::from(x) + 2.0).powi(4) / 2.0) as f32
            }
        }
        "in_out_quint" => {
            if x < 0.5 {
                16.0 * x * x * x * x * x
            } else {
                (1.0 - (-2.0 * f64::from(x) + 2.0).powi(5) / 2.0) as f32
            }
        }
        "in_out_sine" => -(table_trig(std::f32::consts::PI * x, true) - 1.0) / 2.0,
        "out_back" => 1.0 + 2.70158 * (x - 1.0).powi(3) + 1.70158 * (x - 1.0).powi(2),
        "out_bounce" => out_bounce(x),
        "out_circ" => (1.0 - (x - 1.0).powi(2)).sqrt(),
        "out_cubic" => 1.0 - (1.0 - x).powi(3),
        "out_elastic" => out_elastic(x),
        "out_expo" => {
            if x == 1.0 {
                1.0
            } else {
                1.0 - (2.0_f64.powf(-10.0 * f64::from(x)) as f32)
            }
        }
        "out_quad" => 1.0 - (1.0 - x).powi(2),
        "out_quart" => 1.0 - (1.0 - x).powi(4),
        "out_quint" => 1.0 - (1.0 - f64::from(x)).powi(5) as f32,
        "out_sine" => table_trig(x * std::f32::consts::FRAC_PI_2, false),
        _ => return None,
    })
}

fn out_bounce(x: f32) -> f32 {
    if x < 0.363_636_37 {
        7.5625 * (x * x)
    } else if x < 0.727_272_75 {
        7.5625 * (x - 0.545_454_56).powi(2) + 0.75
    } else if f64::from(x) < 0.9090909090909091 {
        7.5625 * (x - 0.818_181_8).powi(2) + 0.9375
    } else {
        7.5625 * (x - 0.954_545_44).powi(2) + 0.984_375
    }
}

fn in_elastic(x: f32) -> f32 {
    if x == 0.0 || x == 1.0 {
        return x;
    }
    let angle = (f64::from(x) * 10.0 - 10.75) * f64::from((std::f64::consts::TAU / 3.0) as f32);
    (-2.0_f64.powf(10.0 * f64::from(x) - 10.0) * angle.sin()) as f32
}
fn out_elastic(x: f32) -> f32 {
    if x == 0.0 || x == 1.0 {
        return x;
    }
    let angle = (f64::from(x) * 10.0 - 0.75) * f64::from((std::f64::consts::TAU / 3.0) as f32);
    (2.0_f64.powf(-10.0 * f64::from(x)) * angle.sin() + 1.0) as f32
}
fn in_out_elastic(x: f32) -> f32 {
    if x == 0.0 || x == 1.0 {
        return x;
    }
    let sin = ((20.0 * f64::from(x) - 11.125) * f64::from(std::f32::consts::PI) * 4.0 / 9.0).sin();
    if x < 0.5 {
        (-(2.0_f64.powf(20.0 * f64::from(x) - 10.0) * sin) / 2.0) as f32
    } else {
        (2.0_f64.powf(-20.0 * f64::from(x) + 10.0) * sin / 2.0 + 1.0) as f32
    }
}

// `Mth.java:30-38,49-55`.
fn table_trig(x: f32, cosine: bool) -> f32 {
    let index = ((f64::from(x) * 10430.378350470453 + if cosine { 16384.0 } else { 0.0 }) as i64
        & 65535) as usize;
    (index as f64 / 10430.378350470453).sin() as f32
}

fn in_out_back(x: f32) -> f32 {
    if x < 0.5 {
        4.0 * x * x * (7.189_819 * x - 2.594_909_4) / 2.0
    } else {
        let delta = 2.0 * x - 2.0;
        (delta * delta * (3.594_909_4 * delta + 2.594_909_4) + 2.0) / 2.0
    }
}

// `EasingType.java:76-95,112-120`.
fn sample_cubic_bezier(alpha: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let x_curve = cubic_curve(x1, x2);
    let y_curve = cubic_curve(y1, y2);
    let mut parameter = alpha;
    for _ in 0..4 {
        let gradient = cubic_gradient(x_curve, parameter);
        if gradient < 1.0e-5 {
            break;
        }
        parameter -= (cubic_sample(x_curve, parameter) - alpha) / gradient;
    }
    cubic_sample(y_curve, parameter)
}

fn cubic_curve(first: f32, second: f32) -> [f32; 3] {
    [
        3.0 * first - 3.0 * second + 1.0,
        -6.0 * first + 3.0 * second,
        3.0 * first,
    ]
}

fn cubic_sample(curve: [f32; 3], value: f32) -> f32 {
    ((curve[0] * value + curve[1]) * value + curve[2]) * value
}

fn cubic_gradient(curve: [f32; 3], value: f32) -> f32 {
    (3.0 * curve[0] * value + 2.0 * curve[1]) * value + curve[2]
}

#[cfg(test)]
mod tests;

mod patrol;
pub use patrol::DimensionBooleanEnvironment;

mod moon;
pub use moon::DimensionMoonEnvironment;

mod villager_activity;
pub use villager_activity::DimensionVillagerActivityEnvironment;

pub mod color;

pub mod discrete;
pub use discrete::{DimensionBedEnvironment, DimensionEyeblossomEnvironment};

#[cfg(test)]
mod float_biome_tests;
