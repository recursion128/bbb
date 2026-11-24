//! Compiled packed-color environment layers.

use bbb_facts::color::{add_rgb, alpha_blend, blend_to_gray, multiply, srgb_lerp, subtract_rgb};
use bbb_protocol::environment_attribute_types::{
    VanillaEnvironmentAttribute, VanillaEnvironmentAttributeType,
};
use bbb_registry_codec::registry_codec::environment_source::color::color_from_canonical;
use std::collections::BTreeMap;

use super::{
    compile_segments, error, field, float, integer, lerp, required, sample_segments, string,
    Easing, EnvironmentAttributeError, EnvironmentAttributeSources, Nbt, Result, Segment,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Modifier {
    Override,
    Add,
    Subtract,
    Multiply,
    AlphaBlend,
    BlendToGray,
}

impl Modifier {
    fn parse(name: &str) -> Result<Self> {
        Ok(match name {
            "override" => Self::Override,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "alpha_blend" => Self::AlphaBlend,
            "blend_to_gray" => Self::BlendToGray,
            _ => return Err(error(format!("unknown color modifier {name}"))),
        })
    }

    fn argument(self, value: &Nbt) -> Result<Argument> {
        if self == Self::BlendToGray {
            Ok(Argument::Gray {
                brightness: float(required(value, "brightness")?)?,
                factor: float(required(value, "factor")?)?,
            })
        } else {
            color_from_canonical(value)
                .map(Argument::Color)
                .ok_or_else(|| error("expected canonical color"))
        }
    }

    // ColorModifier.java:13-45.
    fn apply(self, subject: u32, argument: Argument) -> u32 {
        match (self, argument) {
            (Self::Override, Argument::Color(color)) => color,
            (Self::Add, Argument::Color(color)) => add_rgb(subject, color),
            (Self::Subtract, Argument::Color(color)) => subtract_rgb(subject, color),
            (Self::Multiply, Argument::Color(color)) => multiply(subject, color),
            (Self::AlphaBlend, Argument::Color(color)) => alpha_blend(subject, color),
            (Self::BlendToGray, Argument::Gray { brightness, factor }) => {
                blend_to_gray(subject, brightness, factor)
            }
            _ => unreachable!("compiled color modifier argument type"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Argument {
    Color(u32),
    Gray { brightness: f32, factor: f32 },
}

impl Argument {
    // ColorModifier.java:24-25,44-45,58-59.
    fn interpolate(self, to: Self, alpha: f32) -> Self {
        match (self, to) {
            (Self::Color(from), Self::Color(to)) => Self::Color(srgb_lerp(alpha, from, to)),
            (
                Self::Gray { brightness, factor },
                Self::Gray {
                    brightness: to_brightness,
                    factor: to_factor,
                },
            ) => Self::Gray {
                brightness: lerp(alpha, brightness, to_brightness),
                factor: lerp(alpha, factor, to_factor),
            },
            _ => unreachable!("compiled color keyframe argument type"),
        }
    }
}

fn modifier(value: &Nbt) -> Result<(Modifier, Argument)> {
    let (modifier, argument) = if let Some(name) = field(value, "modifier") {
        (
            Modifier::parse(&string(name)?)?,
            required(value, "argument")?,
        )
    } else {
        (Modifier::Override, value)
    };
    Ok((modifier, modifier.argument(argument)?))
}

#[derive(Debug, Clone, PartialEq)]
struct Track {
    clock: String,
    modifier: Modifier,
    period: Option<i32>,
    easing: Easing,
    segments: Vec<Segment<Argument>>,
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
        let Nbt::List(frames) = required(track, "keyframes")? else {
            return Err(error("expected keyframe list"));
        };
        let frames = frames
            .iter()
            .map(|frame| {
                Ok((
                    integer(required(frame, "ticks")?)?,
                    modifier.argument(required(frame, "value")?)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            clock,
            modifier,
            period,
            easing: Easing::compile(field(track, "ease"))?,
            segments: compile_segments(&frames, period)?,
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

#[derive(Debug, Clone, PartialEq)]
pub struct DimensionColorEnvironment {
    base: u32,
    biomes: BTreeMap<String, Option<(Modifier, Argument)>>,
    tracks: Vec<Track>,
    has_biome_layer: bool,
}

impl DimensionColorEnvironment {
    pub fn from_attribute(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
        attribute: &'static VanillaEnvironmentAttribute,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        if !matches!(
            attribute.value_type,
            VanillaEnvironmentAttributeType::RgbColor | VanillaEnvironmentAttributeType::ArgbColor
        ) {
            return Err(error(format!(
                "{} is not a color attribute",
                attribute.resource_id
            )));
        }
        let mut base = attribute.color_default.ok_or_else(|| {
            error(format!(
                "missing color default for {}",
                attribute.resource_id
            ))
        })?;
        let attribute = attribute.resource_id;
        let dimension = sources.dimension_source(dimension_type_id)?;
        if let Some(value) =
            field(&dimension.value, "attributes").and_then(|map| field(map, attribute))
        {
            let (modifier, argument) = modifier(value)?;
            base = modifier.apply(base, argument);
        }
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
        })
    }

    pub fn varies_by_biome(&self) -> bool {
        self.has_biome_layer
    }

    // EnvironmentAttributeSystem.java:62-96.
    pub fn sample(
        &self,
        biome_id: Option<&str>,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<u32> {
        let mut value = self.base;
        if self.has_biome_layer {
            if let Some((modifier, argument)) = self.biomes.get(biome_id?)? {
                value = modifier.apply(value, *argument);
            }
        }
        for track in &self.tracks {
            value = track
                .modifier
                .apply(value, track.sample(clock_ticks(&track.clock)?));
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests;
