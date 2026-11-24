use bbb_facts::environment_attribute_types::{
    VanillaEnvironmentAttribute, VanillaEnvironmentScalarValue,
};
use std::collections::BTreeMap;

use super::{
    compile_segments, error, field, integer, required, sample_segments, string, Easing,
    EnvironmentAttributeError, EnvironmentAttributeSources, Nbt, Result, Segment,
};

/// One boolean environment attribute of a dimension type: its dimension,
/// biome and timeline layers.
#[derive(Debug, Clone)]
pub struct DimensionBooleanEnvironment {
    base: bool,
    biomes: BTreeMap<String, Option<(Modifier, bool)>>,
    tracks: Vec<Track>,
    has_biome_layer: bool,
}

#[derive(Debug, Clone, Copy)]
enum Modifier {
    Override,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
}

impl Modifier {
    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "override" => Self::Override,
            "and" => Self::And,
            "nand" => Self::Nand,
            "or" => Self::Or,
            "nor" => Self::Nor,
            "xor" => Self::Xor,
            "xnor" => Self::Xnor,
            _ => return Err(error(format!("unknown boolean modifier {value}"))),
        })
    }
    // BooleanModifier.java:15-24.
    fn apply(self, base: bool, argument: bool) -> bool {
        match self {
            Self::Override => argument,
            Self::And => base && argument,
            Self::Nand => !(base && argument),
            Self::Or => base || argument,
            Self::Nor => !(base || argument),
            Self::Xor => base ^ argument,
            Self::Xnor => base == argument,
        }
    }
}

pub(super) fn boolean(value: &Nbt) -> Result<bool> {
    if let Nbt::Byte(value) = value {
        Ok(*value != 0)
    } else {
        Err(error("expected boolean"))
    }
}

fn modifier(value: &Nbt) -> Result<(Modifier, bool)> {
    if let Some(name) = field(value, "modifier") {
        Ok((
            Modifier::parse(&string(name)?)?,
            boolean(required(value, "argument")?)?,
        ))
    } else {
        Ok((Modifier::Override, boolean(value)?))
    }
}

#[derive(Debug, Clone)]
struct Track {
    clock: String,
    period: Option<i32>,
    modifier: Modifier,
    segments: Vec<Segment<bool>>,
    easing: Easing,
}

impl Track {
    fn compile(timeline: &Nbt, track: &Nbt) -> Result<Self> {
        let clock = string(required(timeline, "clock")?)?;
        let period = field(timeline, "period_ticks").map(integer).transpose()?;
        let modifier = field(track, "modifier")
            .map(string)
            .transpose()?
            .unwrap_or_else(|| "override".into());
        let Nbt::List(frames) = required(track, "keyframes")? else {
            return Err(error("expected keyframes"));
        };
        let frames = frames
            .iter()
            .map(|frame| {
                Ok((
                    integer(required(frame, "ticks")?)?,
                    boolean(required(frame, "value")?)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let segments = compile_segments(&frames, period)?;
        Ok(Self {
            clock,
            period,
            modifier: Modifier::parse(&modifier)?,
            segments,
            easing: Easing::compile(field(track, "ease"))?,
        })
    }

    fn apply(&self, base: bool, clock_ticks: impl Fn(&str) -> Option<i64>) -> Option<bool> {
        Some(
            self.modifier
                .apply(base, self.sample(clock_ticks(&self.clock)?)),
        )
    }

    // KeyframeTrackSampler.java:50-66; AttributeType.java:39-51; LerpFunction.java:26-28.
    fn sample(&self, ticks: i64) -> bool {
        sample_segments(
            &self.segments,
            self.period,
            ticks,
            |value| *value,
            |a, b, alpha| {
                // BooleanModifier.java:32-34: non-override arguments use constant interpolation.
                if matches!(self.modifier, Modifier::Override) && self.easing.apply(alpha) >= 1.0 {
                    b
                } else {
                    a
                }
            },
        )
    }
}

fn compile_dimension_boolean(
    sources: &EnvironmentAttributeSources<'_>,
    dimension_type_id: &str,
    attribute: &str,
    default: bool,
) -> Result<(bool, Vec<Track>)> {
    let dimension = sources.dimension_source(dimension_type_id)?;
    let mut base = default;
    if let Some(value) = field(&dimension.value, "attributes").and_then(|map| field(map, attribute))
    {
        let (modifier, argument) = modifier(value)?;
        base = modifier.apply(base, argument);
    }
    let mut tracks = Vec::new();
    for id in &dimension.timeline_ids {
        let timeline = &sources.timelines[id];
        if let Some(track) = field(timeline, "tracks").and_then(|map| field(map, attribute)) {
            tracks.push(Track::compile(timeline, track)?);
        }
    }
    Ok((base, tracks))
}

impl DimensionBooleanEnvironment {
    pub fn from_attribute(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
        attribute: &'static VanillaEnvironmentAttribute,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        let Some(VanillaEnvironmentScalarValue::Boolean(default)) = attribute.scalar_default else {
            return Err(error(format!(
                "{} is not a boolean attribute",
                attribute.resource_id
            )));
        };
        let attribute = attribute.resource_id;
        let (base, tracks) =
            compile_dimension_boolean(sources, dimension_type_id, attribute, default)?;
        let biomes = sources
            .biomes
            .iter()
            .map(|(id, attributes)| {
                Ok((
                    (*id).to_owned(),
                    field(attributes, attribute).map(modifier).transpose()?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let has_biome_layer = biomes.values().any(Option::is_some);
        Ok(Self {
            base,
            biomes,
            tracks,
            has_biome_layer,
        })
    }

    /// Whether any biome modifies the attribute, so a sample needs the biome.
    pub fn varies_by_biome(&self) -> bool {
        self.has_biome_layer
    }

    // EnvironmentAttributeSystem.java:62-96.
    pub fn sample(
        &self,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<bool> {
        let mut value = self.base;
        if self.has_biome_layer {
            if let Some((modifier, argument)) = self.biomes.get(biome_id)? {
                value = modifier.apply(value, *argument);
            }
        }
        self.sample_tracks(value, clock_ticks)
    }

    pub fn sample_dimension(&self, clock_ticks: impl Fn(&str) -> Option<i64>) -> Option<bool> {
        self.sample_tracks(self.base, clock_ticks)
    }

    fn sample_tracks(
        &self,
        mut value: bool,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<bool> {
        for track in &self.tracks {
            value = track.apply(value, &clock_ticks)?;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests;
