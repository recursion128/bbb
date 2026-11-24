use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_protocol::environment_audio::{AmbientSounds, BackgroundMusic};
use bbb_protocol::environment_particles::{AmbientParticle, ParticleOptionValue, ParticleOptions};
use bbb_protocol::NbtSummaryEntry;
use bbb_registry_codec::registry_codec::environment_source::audio::{
    ambient_sounds_from_canonical, background_music_from_canonical,
};
use bbb_registry_codec::registry_codec::environment_source::particles::{
    ambient_particles_from_canonical, particle_from_canonical,
};
use bbb_server_types::environment::{BedAvailability, BedRule, TriState};

use super::{
    compile_segments, error, field, integer, required, sample_segments, string, Easing,
    EnvironmentAttributeError, EnvironmentAttributeSources, Nbt, Result, Segment,
};

const BED: &str = "minecraft:gameplay/bed_rule";

#[derive(Debug, Clone)]
pub struct DimensionBedEnvironment {
    layer: Layer<BedRule>,
}

#[derive(Debug, Clone)]
pub struct DimensionEyeblossomEnvironment {
    layer: Layer<TriState>,
}

impl DimensionEyeblossomEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            layer: Layer::compile(
                sources,
                dimension_type_id,
                "minecraft:gameplay/eyeblossom_open",
            )?,
        })
    }

    pub fn sample(
        &self,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<TriState> {
        self.layer.sample(biome_id, &clock_ticks).copied()
    }
}

#[derive(Debug, Clone)]
pub struct DimensionBackgroundMusicEnvironment {
    layer: Layer<BackgroundMusic>,
}

impl DimensionBackgroundMusicEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            layer: Layer::compile(
                sources,
                dimension_type_id,
                "minecraft:audio/background_music",
            )?,
        })
    }

    pub fn sample(
        &self,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<&BackgroundMusic> {
        self.layer.sample(biome_id, &clock_ticks)
    }
}

#[derive(Debug, Clone)]
pub struct DimensionAmbientSoundsEnvironment {
    layer: Layer<AmbientSounds>,
}

impl DimensionAmbientSoundsEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            layer: Layer::compile(sources, dimension_type_id, "minecraft:audio/ambient_sounds")?,
        })
    }

    pub fn sample(
        &self,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<&AmbientSounds> {
        self.layer.sample(biome_id, &clock_ticks)
    }
}

#[derive(Debug, Clone)]
pub struct DimensionParticleEnvironment {
    layer: Layer<ParticleOptions>,
}

impl DimensionParticleEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            layer: Layer::compile(
                sources,
                dimension_type_id,
                "minecraft:visual/default_dripstone_particle",
            )?,
        })
    }

    pub fn sample(
        &self,
        biome_id: &str,
        clock_ticks: &impl Fn(&str) -> Option<i64>,
    ) -> Option<&ParticleOptions> {
        self.layer.sample(biome_id, clock_ticks)
    }
}

#[derive(Debug, Clone)]
pub struct DimensionAmbientParticlesEnvironment {
    layer: Layer<Vec<AmbientParticle>>,
}

impl DimensionAmbientParticlesEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            layer: Layer::compile(
                sources,
                dimension_type_id,
                "minecraft:visual/ambient_particles",
            )?,
        })
    }

    pub fn sample(
        &self,
        biome_id: &str,
        clock_ticks: &impl Fn(&str) -> Option<i64>,
    ) -> Option<&[AmbientParticle]> {
        self.layer.sample(biome_id, clock_ticks).map(Vec::as_slice)
    }
}

pub(super) trait DiscreteValue: Clone + Debug {
    fn default_value() -> Self;
    fn from_canonical(
        value: &Nbt,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self>;
}

impl DiscreteValue for ParticleOptions {
    // EnvironmentAttributes.java:87-92.
    fn default_value() -> Self {
        Self {
            resource_id: "minecraft:dripping_dripstone_water".into(),
            fields: Vec::new(),
            option: ParticleOptionValue::Empty,
        }
    }

    fn from_canonical(
        value: &Nbt,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        particle_from_canonical(value, lookup).map_err(|failure| error(failure.to_string()))
    }
}

impl DiscreteValue for Vec<AmbientParticle> {
    fn default_value() -> Self {
        Vec::new()
    }

    fn from_canonical(
        value: &Nbt,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        ambient_particles_from_canonical(value, lookup)
            .map_err(|failure| error(failure.to_string()))
    }
}

impl DiscreteValue for BackgroundMusic {
    fn default_value() -> Self {
        Self::default()
    }

    fn from_canonical(
        value: &Nbt,
        _lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        background_music_from_canonical(value)
            .ok_or_else(|| error("invalid canonical background music"))
    }
}

impl DiscreteValue for AmbientSounds {
    fn default_value() -> Self {
        Self::default()
    }

    fn from_canonical(
        value: &Nbt,
        _lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        ambient_sounds_from_canonical(value)
            .ok_or_else(|| error("invalid canonical ambient sounds"))
    }
}

impl DiscreteValue for TriState {
    fn default_value() -> Self {
        Self::Default
    }

    fn from_canonical(
        value: &Nbt,
        _lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        match value {
            Nbt::Byte(_) => super::patrol::boolean(value).map(Self::from_bool),
            Nbt::String(value) => Self::from_serialized_name(
                &value
                    .try_to_string()
                    .map_err(|_| error("invalid tri state"))?,
            )
            .ok_or_else(|| error(format!("invalid tri state {value}"))),
            _ => Err(error("expected tri state")),
        }
    }
}

impl DiscreteValue for BedRule {
    // EnvironmentAttributes.java:114-119; BedRule.CAN_SLEEP_WHEN_DARK.
    fn default_value() -> Self {
        Self {
            can_sleep: BedAvailability::WhenDark,
            can_set_spawn: BedAvailability::Always,
            explodes: false,
            error_message: Some(Nbt::Compound(vec![NbtSummaryEntry {
                name: "translate".into(),
                value: Nbt::String("block.minecraft.bed.no_sleep".into()),
            }])),
        }
    }
    fn from_canonical(
        value: &Nbt,
        _lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        Ok(Self {
            can_sleep: availability(required(value, "can_sleep")?)?,
            can_set_spawn: availability(required(value, "can_set_spawn")?)?,
            explodes: field(value, "explodes")
                .map(super::patrol::boolean)
                .transpose()?
                .unwrap_or(false),
            error_message: field(value, "error_message").cloned(),
        })
    }
}

pub(crate) fn rule_from_source(
    value: bbb_registry_codec::RegistryElementValueRef<'_>,
    source: bbb_registry_codec::RegistryElementSource,
) -> Result<BedRule> {
    let value = bbb_registry_codec::registry_codec::environment_source::canonical_bed_rule_source(
        value,
        source,
        Default::default(),
    )
    .map_err(|e| error(e.to_string()))?;
    BedRule::from_canonical(&value, None)
}

fn availability(value: &Nbt) -> Result<BedAvailability> {
    let name = string(value)?;
    BedAvailability::from_serialized_name(&name)
        .ok_or_else(|| error(format!("invalid canonical bed availability {name}")))
}

#[derive(Debug, Clone)]
struct Track<V: DiscreteValue> {
    clock: String,
    period: Option<i32>,
    easing: Easing,
    segments: Vec<Segment<V>>,
}
impl<V: DiscreteValue> Track<V> {
    fn compile(
        timeline: &Nbt,
        track: &Nbt,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        let clock = string(required(timeline, "clock")?)?;
        let period = field(timeline, "period_ticks").map(integer).transpose()?;
        let Nbt::List(frames) = required(track, "keyframes")? else {
            return Err(error("expected keyframes"));
        };
        let frames = frames
            .iter()
            .map(|frame| {
                Ok((
                    integer(required(frame, "ticks")?)?,
                    V::from_canonical(required(frame, "value")?, lookup)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let segments = compile_segments(&frames, period)?;
        Ok(Self {
            clock,
            period,
            easing: Easing::compile(field(track, "ease"))?,
            segments,
        })
    }
    // KeyframeTrackSampler.java:50-77.
    fn sample(&self, ticks: i64) -> &V {
        sample_segments(
            &self.segments,
            self.period,
            ticks,
            |value| value,
            |a, b, alpha| {
                if self.easing.apply(alpha) >= 1.0 {
                    b
                } else {
                    a
                }
            },
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct Layer<V: DiscreteValue> {
    base: V,
    biomes: BTreeMap<String, Option<V>>,
    has_biome_layer: bool,
    tracks: Vec<Track<V>>,
}
impl<V: DiscreteValue> Layer<V> {
    pub(super) fn compile(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
        attribute: &str,
    ) -> Result<Self> {
        let lookup = FrozenDatapackRegistryLookup::from_startup(sources.startup);
        let dimension = sources.dimension_source(dimension_type_id)?;
        let mut base = V::default_value();
        if let Some(value) =
            field(&dimension.value, "attributes").and_then(|map| field(map, attribute))
        {
            base = V::from_canonical(value, Some(&lookup))?;
        }
        let biomes = sources
            .biomes
            .iter()
            .map(|(id, attributes)| {
                Ok((
                    (*id).to_owned(),
                    field(attributes, attribute)
                        .map(|value| V::from_canonical(value, Some(&lookup)))
                        .transpose()?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let mut tracks = Vec::new();
        for id in &dimension.timeline_ids {
            let timeline = &sources.timelines[id];
            if let Some(track) = field(timeline, "tracks").and_then(|map| field(map, attribute)) {
                tracks.push(Track::compile(timeline, track, Some(&lookup))?);
            }
        }
        let has_biome_layer = biomes.values().any(Option::is_some);
        Ok(Self {
            base,
            biomes,
            has_biome_layer,
            tracks,
        })
    }
    pub(super) fn sample(
        &self,
        biome_id: &str,
        clocks: &impl Fn(&str) -> Option<i64>,
    ) -> Option<&V> {
        let mut value = &self.base;
        if self.has_biome_layer {
            if let Some(argument) = self.biomes.get(biome_id)? {
                value = argument;
            }
        }
        for track in &self.tracks {
            value = track.sample(clocks(&track.clock)?);
        }
        Some(value)
    }
}

impl DimensionBedEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            layer: Layer::compile(sources, dimension_type_id, BED)?,
        })
    }
    pub fn sample(
        &self,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<BedRule> {
        self.sample_ref(biome_id, clock_ticks).cloned()
    }

    pub fn sample_ref(
        &self,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<&BedRule> {
        self.layer.sample(biome_id, &clock_ticks)
    }
}

#[cfg(test)]
mod tests;
