use bbb_protocol::environment_audio::{
    AmbientSounds as ClientAmbientSounds, BackgroundMusic as ClientBackgroundMusic,
    SoundEventHolderValue as ClientSoundEventHolder,
};
use bbb_protocol::environment_particles::{AmbientParticle, ParticleOptions};
use bbb_protocol::nbt_summary::{codec_float_value, integer_bool_value};
use bbb_registry_codec::level_based_value::java_float_min;
use bbb_registry_codec::registry_codec::environment_source::audio::{
    ambient_sounds_from_canonical as parse_ambient_sounds,
    background_music_from_canonical as parse_background_music,
};
use bbb_registry_codec::registry_codec::environment_source::color::color_from_canonical;
use bbb_render_types::color::{
    add_rgb, alpha_blend, blend_to_gray, channel, color, multiply as multiply_argb, srgb_lerp,
    subtract_rgb,
};
use std::fmt;
use std::sync::{Arc, OnceLock};

use bbb_protocol::environment_attribute_types::{
    vanilla_environment_attribute, VanillaEnvironmentAttribute,
    VanillaEnvironmentAttributeType as AttributeKind, VANILLA_ENVIRONMENT_ATTRIBUTES,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use super::environment_particle_options::{parse_ambient_particles, parse_particle};
use crate::FrozenRegistryAccess;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientMoonPhase {
    FullMoon,
    WaningGibbous,
    ThirdQuarter,
    WaningCrescent,
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientLoopSoundState {
    pub sound: ClientSoundEventHolder,
    pub gain: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientEnvironmentAttributes {
    pub fog_color: Option<[u8; 3]>,
    pub fog_start_distance: Option<f32>,
    pub fog_end_distance: Option<f32>,
    pub sky_fog_end_distance: Option<f32>,
    pub cloud_fog_end_distance: Option<f32>,
    pub water_fog_color: Option<[u8; 3]>,
    pub water_fog_start_distance: Option<f32>,
    pub water_fog_end_distance: Option<f32>,
    pub sky_color: Option<[u8; 3]>,
    pub sunrise_sunset_color: Option<[u8; 4]>,
    pub cloud_color: Option<[u8; 4]>,
    pub cloud_height: Option<f32>,
    pub sun_angle_degrees: Option<f32>,
    pub moon_angle_degrees: Option<f32>,
    pub star_angle_degrees: Option<f32>,
    pub moon_phase: Option<ClientMoonPhase>,
    pub star_brightness: Option<f32>,
    pub block_light_tint: Option<[u8; 3]>,
    pub sky_light_color: Option<[u8; 3]>,
    pub sky_light_factor: Option<f32>,
    pub night_vision_color: Option<[u8; 3]>,
    pub ambient_light_color: Option<[u8; 3]>,
    pub default_dripstone_particle: Option<ParticleOptions>,
    pub ambient_particles: Option<Vec<AmbientParticle>>,
    pub background_music: Option<ClientBackgroundMusic>,
    pub music_volume: Option<f32>,
    pub ambient_sounds: Option<ClientAmbientSounds>,
    pub firefly_bush_sounds: Option<bool>,
    pub sky_light_level: Option<f32>,
    pub water_evaporates: Option<bool>,
    pub fast_lava: Option<bool>,
    pub piglins_zombify: Option<bool>,
    pub creaking_active: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EnvironmentAttributeProjectionError {
    pub(crate) field: &'static str,
    pub(crate) reason: &'static str,
}

impl fmt::Display for EnvironmentAttributeProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EnvironmentModifier {
    Override,
    AlphaBlend,
    Add,
    Subtract,
    Multiply,
    BlendToGray,
    Minimum,
    Maximum,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum EnvironmentTrackArgument {
    Float(f32),
    FloatWithAlpha { value: f32, alpha: f32 },
    Boolean(bool),
    Rgb(u32),
    Argb(u32),
    BlendToGray { brightness: f32, factor: f32 },
    MoonPhase(ClientMoonPhase),
    // Vanilla's baked attribute tracks retain object references; sampling a scalar track does
    // not copy storage sized for the largest audio/particle payload. Keep the same property in
    // the tagged Rust projection so interpolated float/color tracks stay register-sized.
    Particle(Arc<ParticleOptions>),
    AmbientParticles(Arc<[AmbientParticle]>),
    BackgroundMusic(Arc<ClientBackgroundMusic>),
    AmbientSounds(Arc<ClientAmbientSounds>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClientEnvironmentAttributeMap {
    entries: Arc<[ClientEnvironmentAttributeMapEntry]>,
}

#[derive(Debug, Clone, PartialEq)]
struct ClientEnvironmentAttributeMapEntry {
    field: &'static str,
    modifier: EnvironmentModifier,
    argument: EnvironmentTrackArgument,
}

impl Default for ClientEnvironmentAttributeMap {
    fn default() -> Self {
        static EMPTY: OnceLock<Arc<[ClientEnvironmentAttributeMapEntry]>> = OnceLock::new();
        Self {
            entries: Arc::clone(EMPTY.get_or_init(|| Arc::from([]))),
        }
    }
}

impl ClientEnvironmentAttributeMap {
    pub(crate) fn apply_to(
        &self,
        attributes: &mut ClientEnvironmentAttributes,
    ) -> Result<(), EnvironmentAttributeProjectionError> {
        for entry in self.entries.iter() {
            apply_environment_argument(attributes, entry.field, entry.modifier, &entry.argument)?;
        }
        Ok(())
    }

    pub(crate) fn apply_non_spatial_to(
        &self,
        attributes: &mut ClientEnvironmentAttributes,
    ) -> Result<(), EnvironmentAttributeProjectionError> {
        for entry in self.entries.iter() {
            if vanilla_environment_attribute(entry.field)
                .is_some_and(|descriptor| !descriptor.spatially_interpolated)
            {
                apply_environment_argument(
                    attributes,
                    entry.field,
                    entry.modifier,
                    &entry.argument,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn apply_attribute_to(
        &self,
        attribute: &str,
        attributes: &mut ClientEnvironmentAttributes,
    ) -> Result<(), EnvironmentAttributeProjectionError> {
        for entry in self.entries.iter().filter(|entry| entry.field == attribute) {
            apply_environment_argument(attributes, entry.field, entry.modifier, &entry.argument)?;
        }
        Ok(())
    }

    pub(crate) fn same_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.entries, &other.entries)
    }

    fn contains(&self, field: &str) -> bool {
        self.entries.iter().any(|entry| entry.field == field)
    }

    fn spatial_value(
        &self,
        descriptor: &VanillaEnvironmentAttribute,
        base: &ClientEnvironmentAttributes,
    ) -> Result<SpatialEnvironmentValue, EnvironmentAttributeProjectionError> {
        let base_value = spatial_environment_value(base, descriptor.resource_id)
            .ok_or_else(|| error("attributes", "attribute has no spatial value"))?;
        let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.field == descriptor.resource_id)
        else {
            return Ok(base_value);
        };
        match base_value {
            SpatialEnvironmentValue::Float(value) => {
                apply_float_modifier(value, entry.modifier, &entry.argument, entry.field)
                    .map(SpatialEnvironmentValue::Float)
            }
            SpatialEnvironmentValue::Rgb(value) => {
                apply_color_modifier(value, entry.modifier, &entry.argument, entry.field)
                    .map(SpatialEnvironmentValue::Rgb)
            }
            SpatialEnvironmentValue::Argb(value) => {
                apply_color_modifier(value, entry.modifier, &entry.argument, entry.field)
                    .map(SpatialEnvironmentValue::Argb)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SpatialEnvironmentValue {
    Float(f32),
    Rgb(u32),
    Argb(u32),
}

pub(crate) fn apply_spatial_environment_attribute_maps(
    attributes: &mut ClientEnvironmentAttributes,
    weighted_sources: &[(ClientEnvironmentAttributeMap, f64)],
) -> Result<(), EnvironmentAttributeProjectionError> {
    apply_filtered_spatial_environment_attribute_maps(attributes, weighted_sources, None)
}

pub(crate) fn apply_selected_spatial_environment_attribute_maps(
    attributes: &mut ClientEnvironmentAttributes,
    weighted_sources: &[(ClientEnvironmentAttributeMap, f64)],
    selected_attributes: &[&str],
) -> Result<(), EnvironmentAttributeProjectionError> {
    apply_filtered_spatial_environment_attribute_maps(
        attributes,
        weighted_sources,
        Some(selected_attributes),
    )
}

fn apply_filtered_spatial_environment_attribute_maps(
    attributes: &mut ClientEnvironmentAttributes,
    weighted_sources: &[(ClientEnvironmentAttributeMap, f64)],
    selected_attributes: Option<&[&str]>,
) -> Result<(), EnvironmentAttributeProjectionError> {
    if weighted_sources.is_empty() {
        return Ok(());
    }
    let base = attributes.clone();
    for descriptor in VANILLA_ENVIRONMENT_ATTRIBUTES.iter().filter(|descriptor| {
        descriptor.syncable
            && descriptor.positional
            && descriptor.spatially_interpolated
            && selected_attributes.is_none_or(|selected| selected.contains(&descriptor.resource_id))
            && weighted_sources
                .iter()
                .any(|(source, _)| source.contains(descriptor.resource_id))
    }) {
        let mut result = None;
        let mut accumulated_weight = 0.0;
        for (source, source_weight) in weighted_sources {
            let source_value = source.spatial_value(descriptor, &base)?;
            accumulated_weight += source_weight;
            result = Some(match result {
                None => source_value,
                Some(result) => lerp_spatial_environment_value(
                    descriptor,
                    (*source_weight / accumulated_weight) as f32,
                    result,
                    source_value,
                )?,
            });
        }
        if let Some(result) = result {
            set_spatial_environment_value(attributes, descriptor.resource_id, result)?;
        }
    }
    Ok(())
}

pub(crate) fn decode_client_environment_attributes(
    entries: &[NbtSummaryEntry],
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<ClientEnvironmentAttributes, EnvironmentAttributeProjectionError> {
    let attribute_map = decode_client_environment_attribute_map(entries, false, registry_access)?;
    let mut attributes = ClientEnvironmentAttributes::default();
    attribute_map.apply_to(&mut attributes)?;
    Ok(attributes)
}

pub(crate) fn decode_client_environment_attribute_map(
    entries: &[NbtSummaryEntry],
    positional_only: bool,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<ClientEnvironmentAttributeMap, EnvironmentAttributeProjectionError> {
    let mut decoded = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(descriptor) = entry
            .name
            .with_utf8(vanilla_environment_attribute)
            .flatten()
        else {
            return Err(error(
                "attributes",
                "unknown environment attribute resource id",
            ));
        };
        if positional_only && !descriptor.positional {
            return Err(error(descriptor.resource_id, "attribute is not positional"));
        }
        if descriptor.syncable {
            let (modifier, argument) = decode_map_entry(
                descriptor.resource_id,
                descriptor.value_type,
                &entry.value,
                registry_access,
            )?;
            decoded.push(ClientEnvironmentAttributeMapEntry {
                field: descriptor.resource_id,
                modifier,
                argument,
            });
        }
    }
    Ok(ClientEnvironmentAttributeMap {
        entries: decoded.into(),
    })
}

pub(super) fn decode_timeline_argument(
    attribute: &str,
    modifier: EnvironmentModifier,
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<EnvironmentTrackArgument, EnvironmentAttributeProjectionError> {
    let Some((field, kind)) = syncable_attribute(attribute) else {
        return Err(error(
            "tracks",
            "unknown or non-syncable environment attribute",
        ));
    };
    decode_modifier_argument(field, kind, modifier, value, registry_access)
}

pub(super) fn interpolate_timeline_argument(
    attribute: &str,
    from: &EnvironmentTrackArgument,
    to: &EnvironmentTrackArgument,
    alpha: f32,
) -> EnvironmentTrackArgument {
    let kind = syncable_attribute(attribute).map(|(_, kind)| kind);
    match (kind, from, to) {
        (
            Some(AttributeKind::Float | AttributeKind::AngleDegrees),
            EnvironmentTrackArgument::Float(from),
            EnvironmentTrackArgument::Float(to),
        ) => EnvironmentTrackArgument::Float(*from + alpha * (*to - *from)),
        (
            Some(AttributeKind::Float | AttributeKind::AngleDegrees),
            EnvironmentTrackArgument::FloatWithAlpha {
                value: from_value,
                alpha: from_alpha,
            },
            EnvironmentTrackArgument::FloatWithAlpha {
                value: to_value,
                alpha: to_alpha,
            },
        ) => EnvironmentTrackArgument::FloatWithAlpha {
            value: *from_value + alpha * (*to_value - *from_value),
            alpha: *from_alpha + alpha * (*to_alpha - *from_alpha),
        },
        (
            Some(AttributeKind::RgbColor | AttributeKind::ArgbColor),
            EnvironmentTrackArgument::Rgb(from),
            EnvironmentTrackArgument::Rgb(to),
        ) => EnvironmentTrackArgument::Rgb(srgb_lerp(alpha, *from, *to)),
        (
            Some(AttributeKind::RgbColor | AttributeKind::ArgbColor),
            EnvironmentTrackArgument::Argb(from),
            EnvironmentTrackArgument::Argb(to),
        ) => EnvironmentTrackArgument::Argb(srgb_lerp(alpha, *from, *to)),
        (
            Some(AttributeKind::RgbColor | AttributeKind::ArgbColor),
            EnvironmentTrackArgument::BlendToGray {
                brightness: from_brightness,
                factor: from_factor,
            },
            EnvironmentTrackArgument::BlendToGray {
                brightness: to_brightness,
                factor: to_factor,
            },
        ) => EnvironmentTrackArgument::BlendToGray {
            brightness: *from_brightness + alpha * (*to_brightness - *from_brightness),
            factor: *from_factor + alpha * (*to_factor - *from_factor),
        },
        _ => from.clone(),
    }
}

pub(super) fn apply_timeline_argument(
    attributes: &mut ClientEnvironmentAttributes,
    attribute: &str,
    modifier: EnvironmentModifier,
    argument: &EnvironmentTrackArgument,
) -> Result<(), EnvironmentAttributeProjectionError> {
    let Some((field, _)) = syncable_attribute(attribute) else {
        return Err(error(
            "tracks",
            "unknown or non-syncable environment attribute",
        ));
    };
    apply_environment_argument(attributes, field, modifier, argument)?;
    Ok(())
}

fn decode_map_entry(
    field: &'static str,
    kind: AttributeKind,
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(EnvironmentModifier, EnvironmentTrackArgument), EnvironmentAttributeProjectionError> {
    if let NbtSummaryValue::Compound(fields) = value {
        if let Some(modifier_value) = field_value(fields, "modifier") {
            let NbtSummaryValue::String(modifier_name) = modifier_value else {
                return Err(error(field, "modifier must be a string"));
            };
            let modifier = modifier_name
                .with_utf8(EnvironmentModifier::from_codec_name)
                .flatten()
                .ok_or_else(|| error(field, "unknown attribute modifier"))?;
            let argument = field_value(fields, "argument")
                .ok_or_else(|| error(field, "modifier entry is missing argument"))?;
            return decode_modifier_argument(field, kind, modifier, argument, registry_access)
                .map(|argument| (modifier, argument));
        }
    }
    decode_modifier_argument(
        field,
        kind,
        EnvironmentModifier::Override,
        value,
        registry_access,
    )
    .map(|argument| (EnvironmentModifier::Override, argument))
}

fn decode_modifier_argument(
    field: &'static str,
    kind: AttributeKind,
    modifier: EnvironmentModifier,
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<EnvironmentTrackArgument, EnvironmentAttributeProjectionError> {
    use EnvironmentModifier as M;
    match (kind, modifier) {
        (AttributeKind::Float | AttributeKind::AngleDegrees, M::AlphaBlend) => {
            parse_float_with_alpha(value)
                .map(|(value, alpha)| EnvironmentTrackArgument::FloatWithAlpha { value, alpha })
                .ok_or_else(|| error(field, "expected float or {value, alpha} argument"))
        }
        (
            AttributeKind::Float | AttributeKind::AngleDegrees,
            M::Override | M::Add | M::Subtract | M::Multiply | M::Minimum | M::Maximum,
        ) => codec_float_value(value)
            .map(EnvironmentTrackArgument::Float)
            .ok_or_else(|| error(field, "expected float")),
        (
            AttributeKind::Boolean,
            M::Override | M::And | M::Nand | M::Or | M::Nor | M::Xor | M::Xnor,
        ) => integer_bool_value(value)
            .map(EnvironmentTrackArgument::Boolean)
            .ok_or_else(|| error(field, "expected boolean")),
        (AttributeKind::RgbColor | AttributeKind::ArgbColor, M::AlphaBlend) => {
            color_from_canonical(value)
                .map(EnvironmentTrackArgument::Argb)
                .ok_or_else(|| error(field, "expected #AARRGGBB string or ARGB int"))
        }
        (AttributeKind::RgbColor, M::Override | M::Add | M::Subtract | M::Multiply) => {
            color_from_canonical(value)
                .map(|color| EnvironmentTrackArgument::Rgb(color | 0xff00_0000))
                .ok_or_else(|| error(field, "expected #RRGGBB string or RGB int"))
        }
        (AttributeKind::ArgbColor, M::Override) => color_from_canonical(value)
            .map(EnvironmentTrackArgument::Argb)
            .ok_or_else(|| error(field, "expected #AARRGGBB string or ARGB int")),
        (AttributeKind::ArgbColor, M::Add | M::Subtract | M::Multiply) => {
            color_from_canonical(value)
                .map(EnvironmentTrackArgument::Argb)
                .ok_or_else(|| error(field, "expected RGB or ARGB color"))
        }
        (AttributeKind::RgbColor | AttributeKind::ArgbColor, M::BlendToGray) => {
            let (brightness, factor) = parse_blend_to_gray(value)
                .ok_or_else(|| error(field, "expected unit brightness and factor"))?;
            Ok(EnvironmentTrackArgument::BlendToGray { brightness, factor })
        }
        (AttributeKind::MoonPhase, M::Override) => parse_moon_phase(value)
            .map(EnvironmentTrackArgument::MoonPhase)
            .ok_or_else(|| error(field, "unknown moon phase")),
        (AttributeKind::Particle, M::Override) => parse_particle(value, registry_access)
            .map(Arc::new)
            .map(EnvironmentTrackArgument::Particle)
            .ok_or_else(|| error(field, "invalid particle options")),
        (AttributeKind::AmbientParticles, M::Override) => {
            parse_ambient_particles(value, registry_access)
                .map(Arc::from)
                .map(EnvironmentTrackArgument::AmbientParticles)
                .ok_or_else(|| error(field, "invalid ambient particle list"))
        }
        (AttributeKind::BackgroundMusic, M::Override) => parse_background_music(value)
            .map(Arc::new)
            .map(EnvironmentTrackArgument::BackgroundMusic)
            .ok_or_else(|| error(field, "invalid background music")),
        (AttributeKind::AmbientSounds, M::Override) => parse_ambient_sounds(value)
            .map(Arc::new)
            .map(EnvironmentTrackArgument::AmbientSounds)
            .ok_or_else(|| error(field, "invalid ambient sounds")),
        _ => Err(error(field, "modifier is not allowed for attribute type")),
    }
}

fn apply_environment_argument(
    attributes: &mut ClientEnvironmentAttributes,
    field: &'static str,
    modifier: EnvironmentModifier,
    argument: &EnvironmentTrackArgument,
) -> Result<(), EnvironmentAttributeProjectionError> {
    macro_rules! apply_float {
        ($slot:expr, $default:expr) => {{
            let base = $slot.unwrap_or($default);
            $slot = Some(apply_float_modifier(base, modifier, argument, field)?);
        }};
    }
    macro_rules! apply_bool {
        ($slot:expr, $default:expr) => {{
            let base = $slot.unwrap_or($default);
            $slot = Some(apply_boolean_modifier(base, modifier, argument, field)?);
        }};
    }
    match field {
        "minecraft:visual/fog_color" => {
            let base = attributes
                .fog_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff00_0000);
            attributes.fog_color = Some(argb_to_rgb_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/fog_start_distance" => apply_float!(attributes.fog_start_distance, 0.0),
        "minecraft:visual/fog_end_distance" => apply_float!(attributes.fog_end_distance, 1024.0),
        "minecraft:visual/sky_fog_end_distance" => {
            apply_float!(attributes.sky_fog_end_distance, 512.0)
        }
        "minecraft:visual/cloud_fog_end_distance" => {
            apply_float!(attributes.cloud_fog_end_distance, 2048.0)
        }
        "minecraft:visual/water_fog_color" => {
            let base = attributes
                .water_fog_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff05_0533);
            attributes.water_fog_color = Some(argb_to_rgb_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/water_fog_start_distance" => {
            apply_float!(attributes.water_fog_start_distance, -8.0)
        }
        "minecraft:visual/water_fog_end_distance" => {
            apply_float!(attributes.water_fog_end_distance, 96.0)
        }
        "minecraft:visual/sky_color" => {
            let base = attributes
                .sky_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff00_0000);
            attributes.sky_color = Some(argb_to_rgb_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/sunrise_sunset_color" => {
            let base = attributes
                .sunrise_sunset_color
                .map(rgba_bytes_to_argb)
                .unwrap_or(0);
            attributes.sunrise_sunset_color = Some(argb_to_rgba_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/cloud_color" => {
            let base = attributes.cloud_color.map(rgba_bytes_to_argb).unwrap_or(0);
            attributes.cloud_color = Some(argb_to_rgba_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/cloud_height" => apply_float!(attributes.cloud_height, 192.33),
        "minecraft:visual/sun_angle" => apply_float!(attributes.sun_angle_degrees, 0.0),
        "minecraft:visual/moon_angle" => apply_float!(attributes.moon_angle_degrees, 0.0),
        "minecraft:visual/star_angle" => apply_float!(attributes.star_angle_degrees, 0.0),
        "minecraft:visual/moon_phase" => {
            let EnvironmentTrackArgument::MoonPhase(value) = argument else {
                return Err(error(field, "moon phase argument type mismatch"));
            };
            if modifier != EnvironmentModifier::Override {
                return Err(error(field, "modifier is not allowed for moon phase"));
            }
            attributes.moon_phase = Some(*value);
        }
        "minecraft:visual/star_brightness" => apply_float!(attributes.star_brightness, 0.0),
        "minecraft:visual/block_light_tint" => {
            let base = attributes
                .block_light_tint
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xffff_d88c);
            attributes.block_light_tint = Some(argb_to_rgb_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/sky_light_color" => {
            let base = attributes
                .sky_light_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xffff_ffff);
            attributes.sky_light_color = Some(argb_to_rgb_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/sky_light_factor" => apply_float!(attributes.sky_light_factor, 1.0),
        "minecraft:visual/night_vision_color" => {
            let base = attributes
                .night_vision_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff99_9999);
            attributes.night_vision_color = Some(argb_to_rgb_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/ambient_light_color" => {
            let base = attributes
                .ambient_light_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff00_0000);
            attributes.ambient_light_color = Some(argb_to_rgb_bytes(apply_color_modifier(
                base, modifier, argument, field,
            )?));
        }
        "minecraft:visual/default_dripstone_particle" => {
            let EnvironmentTrackArgument::Particle(value) = argument else {
                return Err(error(field, "particle argument type mismatch"));
            };
            attributes.default_dripstone_particle = Some(value.as_ref().clone());
        }
        "minecraft:visual/ambient_particles" => {
            let EnvironmentTrackArgument::AmbientParticles(value) = argument else {
                return Err(error(field, "ambient particles argument type mismatch"));
            };
            attributes.ambient_particles = Some(value.to_vec());
        }
        "minecraft:audio/background_music" => {
            let EnvironmentTrackArgument::BackgroundMusic(value) = argument else {
                return Err(error(field, "background music argument type mismatch"));
            };
            attributes.background_music = Some(value.as_ref().clone());
        }
        "minecraft:audio/music_volume" => apply_float!(attributes.music_volume, 1.0),
        "minecraft:audio/ambient_sounds" => {
            let EnvironmentTrackArgument::AmbientSounds(value) = argument else {
                return Err(error(field, "ambient sounds argument type mismatch"));
            };
            attributes.ambient_sounds = Some(value.as_ref().clone());
        }
        "minecraft:audio/firefly_bush_sounds" => {
            apply_bool!(attributes.firefly_bush_sounds, false)
        }
        "minecraft:gameplay/sky_light_level" => apply_float!(attributes.sky_light_level, 15.0),
        "minecraft:gameplay/water_evaporates" => apply_bool!(attributes.water_evaporates, false),
        "minecraft:gameplay/fast_lava" => apply_bool!(attributes.fast_lava, false),
        "minecraft:gameplay/piglins_zombify" => apply_bool!(attributes.piglins_zombify, true),
        "minecraft:gameplay/creaking_active" => apply_bool!(attributes.creaking_active, false),
        _ => return Err(error("attributes", "unsupported syncable attribute")),
    }
    Ok(())
}

fn apply_float_modifier(
    base: f32,
    modifier: EnvironmentModifier,
    argument: &EnvironmentTrackArgument,
    field: &'static str,
) -> Result<f32, EnvironmentAttributeProjectionError> {
    let value = match (modifier, argument) {
        (
            EnvironmentModifier::AlphaBlend,
            EnvironmentTrackArgument::FloatWithAlpha { value, alpha },
        ) => base + *alpha * (*value - base),
        (EnvironmentModifier::Override, EnvironmentTrackArgument::Float(value)) => *value,
        (EnvironmentModifier::Add, EnvironmentTrackArgument::Float(value)) => base + *value,
        (EnvironmentModifier::Subtract, EnvironmentTrackArgument::Float(value)) => base - *value,
        (EnvironmentModifier::Multiply, EnvironmentTrackArgument::Float(value)) => base * *value,
        (EnvironmentModifier::Minimum, EnvironmentTrackArgument::Float(value)) => {
            java_float_min(base, *value)
        }
        (EnvironmentModifier::Maximum, EnvironmentTrackArgument::Float(value)) => {
            java_float_max(base, *value)
        }
        _ => return Err(error(field, "float modifier argument type mismatch")),
    };
    Ok(value)
}

fn apply_boolean_modifier(
    base: bool,
    modifier: EnvironmentModifier,
    argument: &EnvironmentTrackArgument,
    field: &'static str,
) -> Result<bool, EnvironmentAttributeProjectionError> {
    let EnvironmentTrackArgument::Boolean(argument) = argument else {
        return Err(error(field, "boolean modifier argument type mismatch"));
    };
    Ok(match modifier {
        EnvironmentModifier::Override => *argument,
        EnvironmentModifier::And => *argument && base,
        EnvironmentModifier::Nand => !*argument || !base,
        EnvironmentModifier::Or => *argument || base,
        EnvironmentModifier::Nor => !*argument && !base,
        EnvironmentModifier::Xor => *argument ^ base,
        EnvironmentModifier::Xnor => *argument == base,
        _ => return Err(error(field, "modifier is not allowed for boolean")),
    })
}

fn apply_color_modifier(
    base: u32,
    modifier: EnvironmentModifier,
    argument: &EnvironmentTrackArgument,
    field: &'static str,
) -> Result<u32, EnvironmentAttributeProjectionError> {
    Ok(match (modifier, argument) {
        (EnvironmentModifier::Override, EnvironmentTrackArgument::Rgb(value)) => *value,
        (EnvironmentModifier::Override, EnvironmentTrackArgument::Argb(value)) => *value,
        (EnvironmentModifier::AlphaBlend, EnvironmentTrackArgument::Argb(value)) => {
            alpha_blend(base, *value)
        }
        (EnvironmentModifier::Add, EnvironmentTrackArgument::Rgb(value))
        | (EnvironmentModifier::Add, EnvironmentTrackArgument::Argb(value)) => {
            add_rgb(base, *value)
        }
        (EnvironmentModifier::Subtract, EnvironmentTrackArgument::Rgb(value))
        | (EnvironmentModifier::Subtract, EnvironmentTrackArgument::Argb(value)) => {
            subtract_rgb(base, *value)
        }
        (EnvironmentModifier::Multiply, EnvironmentTrackArgument::Rgb(value))
        | (EnvironmentModifier::Multiply, EnvironmentTrackArgument::Argb(value)) => {
            multiply_argb(base, *value)
        }
        (
            EnvironmentModifier::BlendToGray,
            EnvironmentTrackArgument::BlendToGray { brightness, factor },
        ) => blend_to_gray(base, *brightness, *factor),
        _ => return Err(error(field, "color modifier argument type mismatch")),
    })
}

fn spatial_environment_value(
    attributes: &ClientEnvironmentAttributes,
    field: &str,
) -> Option<SpatialEnvironmentValue> {
    Some(match field {
        "minecraft:visual/fog_color" => SpatialEnvironmentValue::Rgb(
            attributes
                .fog_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff00_0000),
        ),
        "minecraft:visual/fog_start_distance" => {
            SpatialEnvironmentValue::Float(attributes.fog_start_distance.unwrap_or(0.0))
        }
        "minecraft:visual/fog_end_distance" => {
            SpatialEnvironmentValue::Float(attributes.fog_end_distance.unwrap_or(1024.0))
        }
        "minecraft:visual/sky_fog_end_distance" => {
            SpatialEnvironmentValue::Float(attributes.sky_fog_end_distance.unwrap_or(512.0))
        }
        "minecraft:visual/cloud_fog_end_distance" => {
            SpatialEnvironmentValue::Float(attributes.cloud_fog_end_distance.unwrap_or(2048.0))
        }
        "minecraft:visual/water_fog_color" => SpatialEnvironmentValue::Rgb(
            attributes
                .water_fog_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff05_0533),
        ),
        "minecraft:visual/water_fog_start_distance" => {
            SpatialEnvironmentValue::Float(attributes.water_fog_start_distance.unwrap_or(-8.0))
        }
        "minecraft:visual/water_fog_end_distance" => {
            SpatialEnvironmentValue::Float(attributes.water_fog_end_distance.unwrap_or(96.0))
        }
        "minecraft:visual/sky_color" => SpatialEnvironmentValue::Rgb(
            attributes
                .sky_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff00_0000),
        ),
        "minecraft:visual/sunrise_sunset_color" => SpatialEnvironmentValue::Argb(
            attributes
                .sunrise_sunset_color
                .map(rgba_bytes_to_argb)
                .unwrap_or(0),
        ),
        "minecraft:visual/cloud_color" => SpatialEnvironmentValue::Argb(
            attributes.cloud_color.map(rgba_bytes_to_argb).unwrap_or(0),
        ),
        "minecraft:visual/cloud_height" => {
            SpatialEnvironmentValue::Float(attributes.cloud_height.unwrap_or(192.33))
        }
        "minecraft:visual/sun_angle" => {
            SpatialEnvironmentValue::Float(attributes.sun_angle_degrees.unwrap_or(0.0))
        }
        "minecraft:visual/moon_angle" => {
            SpatialEnvironmentValue::Float(attributes.moon_angle_degrees.unwrap_or(0.0))
        }
        "minecraft:visual/star_angle" => {
            SpatialEnvironmentValue::Float(attributes.star_angle_degrees.unwrap_or(0.0))
        }
        "minecraft:visual/star_brightness" => {
            SpatialEnvironmentValue::Float(attributes.star_brightness.unwrap_or(0.0))
        }
        "minecraft:visual/block_light_tint" => SpatialEnvironmentValue::Rgb(
            attributes
                .block_light_tint
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xffff_d88c),
        ),
        "minecraft:visual/sky_light_color" => SpatialEnvironmentValue::Rgb(
            attributes
                .sky_light_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xffff_ffff),
        ),
        "minecraft:visual/sky_light_factor" => {
            SpatialEnvironmentValue::Float(attributes.sky_light_factor.unwrap_or(1.0))
        }
        "minecraft:visual/night_vision_color" => SpatialEnvironmentValue::Rgb(
            attributes
                .night_vision_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff99_9999),
        ),
        "minecraft:visual/ambient_light_color" => SpatialEnvironmentValue::Rgb(
            attributes
                .ambient_light_color
                .map(rgb_bytes_to_argb)
                .unwrap_or(0xff00_0000),
        ),
        _ => return None,
    })
}

fn set_spatial_environment_value(
    attributes: &mut ClientEnvironmentAttributes,
    field: &'static str,
    value: SpatialEnvironmentValue,
) -> Result<(), EnvironmentAttributeProjectionError> {
    match (field, value) {
        ("minecraft:visual/fog_color", SpatialEnvironmentValue::Rgb(value)) => {
            attributes.fog_color = Some(argb_to_rgb_bytes(value))
        }
        ("minecraft:visual/fog_start_distance", SpatialEnvironmentValue::Float(value)) => {
            attributes.fog_start_distance = Some(value)
        }
        ("minecraft:visual/fog_end_distance", SpatialEnvironmentValue::Float(value)) => {
            attributes.fog_end_distance = Some(value)
        }
        ("minecraft:visual/sky_fog_end_distance", SpatialEnvironmentValue::Float(value)) => {
            attributes.sky_fog_end_distance = Some(value)
        }
        ("minecraft:visual/cloud_fog_end_distance", SpatialEnvironmentValue::Float(value)) => {
            attributes.cloud_fog_end_distance = Some(value)
        }
        ("minecraft:visual/water_fog_color", SpatialEnvironmentValue::Rgb(value)) => {
            attributes.water_fog_color = Some(argb_to_rgb_bytes(value))
        }
        ("minecraft:visual/water_fog_start_distance", SpatialEnvironmentValue::Float(value)) => {
            attributes.water_fog_start_distance = Some(value)
        }
        ("minecraft:visual/water_fog_end_distance", SpatialEnvironmentValue::Float(value)) => {
            attributes.water_fog_end_distance = Some(value)
        }
        ("minecraft:visual/sky_color", SpatialEnvironmentValue::Rgb(value)) => {
            attributes.sky_color = Some(argb_to_rgb_bytes(value))
        }
        ("minecraft:visual/sunrise_sunset_color", SpatialEnvironmentValue::Argb(value)) => {
            attributes.sunrise_sunset_color = Some(argb_to_rgba_bytes(value))
        }
        ("minecraft:visual/cloud_color", SpatialEnvironmentValue::Argb(value)) => {
            attributes.cloud_color = Some(argb_to_rgba_bytes(value))
        }
        ("minecraft:visual/cloud_height", SpatialEnvironmentValue::Float(value)) => {
            attributes.cloud_height = Some(value)
        }
        ("minecraft:visual/sun_angle", SpatialEnvironmentValue::Float(value)) => {
            attributes.sun_angle_degrees = Some(value)
        }
        ("minecraft:visual/moon_angle", SpatialEnvironmentValue::Float(value)) => {
            attributes.moon_angle_degrees = Some(value)
        }
        ("minecraft:visual/star_angle", SpatialEnvironmentValue::Float(value)) => {
            attributes.star_angle_degrees = Some(value)
        }
        ("minecraft:visual/star_brightness", SpatialEnvironmentValue::Float(value)) => {
            attributes.star_brightness = Some(value)
        }
        ("minecraft:visual/block_light_tint", SpatialEnvironmentValue::Rgb(value)) => {
            attributes.block_light_tint = Some(argb_to_rgb_bytes(value))
        }
        ("minecraft:visual/sky_light_color", SpatialEnvironmentValue::Rgb(value)) => {
            attributes.sky_light_color = Some(argb_to_rgb_bytes(value))
        }
        ("minecraft:visual/sky_light_factor", SpatialEnvironmentValue::Float(value)) => {
            attributes.sky_light_factor = Some(value)
        }
        ("minecraft:visual/night_vision_color", SpatialEnvironmentValue::Rgb(value)) => {
            attributes.night_vision_color = Some(argb_to_rgb_bytes(value))
        }
        ("minecraft:visual/ambient_light_color", SpatialEnvironmentValue::Rgb(value)) => {
            attributes.ambient_light_color = Some(argb_to_rgb_bytes(value))
        }
        _ => return Err(error(field, "spatial attribute value type mismatch")),
    }
    Ok(())
}

fn lerp_spatial_environment_value(
    descriptor: &VanillaEnvironmentAttribute,
    alpha: f32,
    from: SpatialEnvironmentValue,
    to: SpatialEnvironmentValue,
) -> Result<SpatialEnvironmentValue, EnvironmentAttributeProjectionError> {
    Ok(match (descriptor.value_type, from, to) {
        (
            AttributeKind::Float | AttributeKind::AngleDegrees,
            SpatialEnvironmentValue::Float(from),
            SpatialEnvironmentValue::Float(to),
        ) => SpatialEnvironmentValue::Float(from + alpha * (to - from)),
        (
            AttributeKind::RgbColor,
            SpatialEnvironmentValue::Rgb(from),
            SpatialEnvironmentValue::Rgb(to),
        ) => SpatialEnvironmentValue::Rgb(srgb_lerp(alpha, from, to)),
        (
            AttributeKind::ArgbColor,
            SpatialEnvironmentValue::Argb(from),
            SpatialEnvironmentValue::Argb(to),
        ) => SpatialEnvironmentValue::Argb(srgb_lerp(alpha, from, to)),
        _ => {
            return Err(error(
                descriptor.resource_id,
                "spatial attribute value type mismatch",
            ))
        }
    })
}

pub(crate) fn parse_rgb(value: &NbtSummaryValue) -> Option<[u8; 3]> {
    color_from_canonical(value).map(|color| argb_to_rgb_bytes(color | 0xff00_0000))
}

fn parse_float_with_alpha(value: &NbtSummaryValue) -> Option<(f32, f32)> {
    if let Some(value) = codec_float_value(value) {
        return Some((value, 1.0));
    }
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let value = codec_float_value(field_value(fields, "value")?)?;
    let alpha = codec_float_value(field_value(fields, "alpha")?)?;
    Some((value, alpha))
}

fn parse_blend_to_gray(value: &NbtSummaryValue) -> Option<(f32, f32)> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let brightness = codec_float_value(field_value(fields, "brightness")?)?;
    let factor = codec_float_value(field_value(fields, "factor")?)?;
    Some((brightness, factor))
}

fn parse_moon_phase(value: &NbtSummaryValue) -> Option<ClientMoonPhase> {
    let NbtSummaryValue::String(value) = value else {
        return None;
    };
    Some(match value.try_to_string().ok()?.as_str() {
        "full_moon" => ClientMoonPhase::FullMoon,
        "waning_gibbous" => ClientMoonPhase::WaningGibbous,
        "third_quarter" => ClientMoonPhase::ThirdQuarter,
        "waning_crescent" => ClientMoonPhase::WaningCrescent,
        "new_moon" => ClientMoonPhase::NewMoon,
        "waxing_crescent" => ClientMoonPhase::WaxingCrescent,
        "first_quarter" => ClientMoonPhase::FirstQuarter,
        "waxing_gibbous" => ClientMoonPhase::WaxingGibbous,
        _ => return None,
    })
}

impl EnvironmentModifier {
    pub(super) fn from_codec_name(value: &str) -> Option<Self> {
        Some(match value {
            "override" => Self::Override,
            "alpha_blend" => Self::AlphaBlend,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "blend_to_gray" => Self::BlendToGray,
            "minimum" => Self::Minimum,
            "maximum" => Self::Maximum,
            "and" => Self::And,
            "nand" => Self::Nand,
            "or" => Self::Or,
            "nor" => Self::Nor,
            "xor" => Self::Xor,
            "xnor" => Self::Xnor,
            _ => return None,
        })
    }
}

fn syncable_attribute(name: &str) -> Option<(&'static str, AttributeKind)> {
    let descriptor = vanilla_environment_attribute(name)?;
    descriptor
        .syncable
        .then_some((descriptor.resource_id, descriptor.value_type))
}

pub(super) fn sanitize_environment_attributes(attributes: &mut ClientEnvironmentAttributes) {
    attributes.fog_end_distance = attributes.fog_end_distance.map(sanitize_non_negative);
    attributes.sky_fog_end_distance = attributes.sky_fog_end_distance.map(sanitize_non_negative);
    attributes.cloud_fog_end_distance =
        attributes.cloud_fog_end_distance.map(sanitize_non_negative);
    attributes.water_fog_end_distance =
        attributes.water_fog_end_distance.map(sanitize_non_negative);
    attributes.star_brightness = attributes
        .star_brightness
        .map(|value| value.clamp(0.0, 1.0));
    attributes.sky_light_factor = attributes
        .sky_light_factor
        .map(|value| value.clamp(0.0, 1.0));
    attributes.music_volume = attributes.music_volume.map(|value| value.clamp(0.0, 1.0));
    attributes.sky_light_level = attributes
        .sky_light_level
        .map(|value| value.clamp(0.0, 15.0));
}

fn sanitize_non_negative(value: f32) -> f32 {
    if value < 0.0 {
        0.0
    } else {
        value
    }
}

fn java_float_max(left: f32, right: f32) -> f32 {
    if left.is_nan() {
        left
    } else if right.is_nan() {
        right
    } else {
        left.max(right)
    }
}

pub(super) fn field_value<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn error(field: &'static str, reason: &'static str) -> EnvironmentAttributeProjectionError {
    EnvironmentAttributeProjectionError { field, reason }
}

fn rgb_bytes_to_argb([red, green, blue]: [u8; 3]) -> u32 {
    color(255, u32::from(red), u32::from(green), u32::from(blue))
}

fn rgba_bytes_to_argb([red, green, blue, alpha]: [u8; 4]) -> u32 {
    color(
        u32::from(alpha),
        u32::from(red),
        u32::from(green),
        u32::from(blue),
    )
}

fn argb_to_rgb_bytes(value: u32) -> [u8; 3] {
    [
        channel(value, 16) as u8,
        channel(value, 8) as u8,
        channel(value, 0) as u8,
    ]
}

fn argb_to_rgba_bytes(value: u32) -> [u8; 4] {
    [
        channel(value, 16) as u8,
        channel(value, 8) as u8,
        channel(value, 0) as u8,
        channel(value, 24) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use crate::RegistrySet;

    use super::*;

    fn decode_attributes(
        entries: &[NbtSummaryEntry],
    ) -> Result<ClientEnvironmentAttributes, EnvironmentAttributeProjectionError> {
        let registries = RegistrySet::vanilla_26_1();
        decode_client_environment_attributes(entries, registries.frozen_access())
    }

    fn decode_attribute_map(
        entries: &[NbtSummaryEntry],
        positional_only: bool,
    ) -> Result<ClientEnvironmentAttributeMap, EnvironmentAttributeProjectionError> {
        let registries = RegistrySet::vanilla_26_1();
        decode_client_environment_attribute_map(
            entries,
            positional_only,
            registries.frozen_access(),
        )
    }

    #[test]
    fn decodes_direct_visual_environment_attributes() {
        let entries = vec![
            entry(
                "minecraft:visual/fog_color",
                NbtSummaryValue::string("#123456"),
            ),
            entry(
                "minecraft:visual/fog_start_distance",
                NbtSummaryValue::Double((-4.0_f64).to_bits()),
            ),
            entry(
                "minecraft:visual/fog_end_distance",
                NbtSummaryValue::Double(192.0_f64.to_bits()),
            ),
            entry(
                "minecraft:visual/sky_light_factor",
                NbtSummaryValue::Double(0.25_f64.to_bits()),
            ),
            entry(
                "minecraft:visual/ambient_light_color",
                NbtSummaryValue::Int(0x203040),
            ),
            entry("minecraft:visual/sky_color", NbtSummaryValue::Int(0x7f3f00)),
            entry(
                "minecraft:visual/sunrise_sunset_color",
                NbtSummaryValue::string("#1fff7f3f"),
            ),
        ];

        let projection = decode_attributes(&entries).unwrap();
        assert_eq!(projection.fog_color, Some([0x12, 0x34, 0x56]));
        assert_eq!(projection.fog_start_distance, Some(-4.0));
        assert_eq!(projection.fog_end_distance, Some(192.0));
        assert_eq!(projection.sky_light_factor, Some(0.25));
        assert_eq!(projection.ambient_light_color, Some([0x20, 0x30, 0x40]));
        assert_eq!(projection.sky_color, Some([127, 63, 0]));
        assert_eq!(projection.sunrise_sunset_color, Some([255, 127, 63, 31]));
    }

    #[test]
    fn applies_float_boolean_and_color_modifiers() {
        let entries = vec![
            modifier(
                "minecraft:visual/fog_end_distance",
                "multiply",
                NbtSummaryValue::Float(0.25_f32.to_bits()),
            ),
            modifier(
                "minecraft:visual/fog_color",
                "add",
                NbtSummaryValue::string("#102030"),
            ),
            modifier(
                "minecraft:gameplay/piglins_zombify",
                "and",
                NbtSummaryValue::Byte(0),
            ),
            modifier(
                "minecraft:visual/sky_light_factor",
                "multiply",
                NbtSummaryValue::Float(2.0_f32.to_bits()),
            ),
        ];

        let projection = decode_attributes(&entries).unwrap();
        assert_eq!(projection.fog_end_distance, Some(256.0));
        assert_eq!(projection.fog_color, Some([0x10, 0x20, 0x30]));
        assert_eq!(projection.piglins_zombify, Some(false));
        assert_eq!(projection.sky_light_factor, Some(2.0));
    }

    #[test]
    fn decodes_structured_syncable_values() {
        let entries = vec![
            entry(
                "minecraft:visual/default_dripstone_particle",
                compound(vec![entry(
                    "type",
                    NbtSummaryValue::string("minecraft:dripping_dripstone_lava"),
                )]),
            ),
            entry(
                "minecraft:audio/background_music",
                compound(vec![entry(
                    "default",
                    compound(vec![
                        entry("sound", NbtSummaryValue::string("minecraft:music.game")),
                        entry("min_delay", NbtSummaryValue::Int(10)),
                        entry("max_delay", NbtSummaryValue::Int(20)),
                    ]),
                )]),
            ),
        ];

        let projection = decode_attributes(&entries).unwrap();
        assert_eq!(
            projection
                .default_dripstone_particle
                .as_ref()
                .map(|particle| particle.resource_id.as_str()),
            Some("minecraft:dripping_dripstone_lava")
        );
        assert_eq!(
            projection
                .background_music
                .as_ref()
                .and_then(|music| music.default.as_ref())
                .map(|music| (music.min_delay, music.max_delay)),
            Some((10, 20))
        );
    }

    #[test]
    fn preserves_custom_sound_references() {
        let entries = vec![entry(
            "minecraft:audio/ambient_sounds",
            compound(vec![entry(
                "loop",
                NbtSummaryValue::string("example:ambient_loop"),
            )]),
        )];
        let projection = decode_attributes(&entries).unwrap();
        assert_eq!(
            projection
                .ambient_sounds
                .as_ref()
                .and_then(|ambient| ambient.loop_sound.as_ref()),
            Some(&ClientSoundEventHolder::Reference {
                resource_id: "example:ambient_loop".to_string(),
            })
        );
    }

    #[test]
    fn rejects_disallowed_modifiers_and_invalid_color_sources() {
        let invalid_modifier = vec![modifier(
            "minecraft:visual/moon_phase",
            "add",
            NbtSummaryValue::string("full_moon"),
        )];
        assert_eq!(
            decode_attributes(&invalid_modifier).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:visual/moon_phase",
                reason: "modifier is not allowed for attribute type",
            }
        );

        let rgb_string_for_argb_modifier = NbtSummaryValue::string("#123456");
        assert!(
            bbb_registry_codec::registry_codec::environment_source::color::color_from_source(
                bbb_registry_codec::registry_codec::RegistryElementValueRef::Nbt(
                    &rgb_string_for_argb_modifier,
                ),
                bbb_registry_codec::RegistryElementSource::NetworkNbt,
                bbb_registry_codec::PersistentNbtDecodeLimits::default(),
                vanilla_environment_attribute("minecraft:visual/cloud_color").unwrap(),
            )
            .is_err()
        );
    }

    #[test]
    fn environment_attribute_map_preserves_modifier_for_later_base() {
        let entries = vec![modifier(
            "minecraft:visual/water_fog_end_distance",
            "multiply",
            NbtSummaryValue::Float(0.85_f32.to_bits()),
        )];
        let attribute_map = decode_attribute_map(&entries, true).unwrap();
        let mut attributes = ClientEnvironmentAttributes {
            water_fog_end_distance: Some(200.0),
            ..ClientEnvironmentAttributes::default()
        };

        attribute_map.apply_to(&mut attributes).unwrap();

        assert!((attributes.water_fog_end_distance.unwrap() - 170.0).abs() < 0.0001);
    }

    #[test]
    fn environment_attribute_map_enforces_positional_and_network_filtering() {
        let entries = vec![
            entry(
                "minecraft:gameplay/can_start_raid",
                NbtSummaryValue::Byte(1),
            ),
            entry(
                "minecraft:visual/sky_light_factor",
                NbtSummaryValue::Float(0.5_f32.to_bits()),
            ),
        ];
        let attribute_map = decode_attribute_map(&entries, true).unwrap();
        assert_eq!(attribute_map.entries.len(), 1);

        let non_positional = vec![entry(
            "minecraft:gameplay/sky_light_level",
            NbtSummaryValue::Float(12.0_f32.to_bits()),
        )];
        assert_eq!(
            decode_attribute_map(&non_positional, true).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:gameplay/sky_light_level",
                reason: "attribute is not positional",
            }
        );
    }

    #[test]
    fn validates_particle_option_map_codecs() {
        let dust = vec![entry(
            "minecraft:visual/default_dripstone_particle",
            compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:dust")),
                entry("color", NbtSummaryValue::Int(0x12_34_56)),
                entry("scale", NbtSummaryValue::Float(0.5_f32.to_bits())),
            ]),
        )];
        let projection = decode_attributes(&dust).unwrap();
        assert_eq!(
            projection.default_dripstone_particle.unwrap().resource_id,
            "minecraft:dust"
        );

        let missing_scale = vec![entry(
            "minecraft:visual/default_dripstone_particle",
            compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:dust")),
                entry("color", NbtSummaryValue::Int(0x12_34_56)),
            ]),
        )];
        assert!(decode_attributes(&missing_scale).is_err());

        let air_item = vec![entry(
            "minecraft:visual/default_dripstone_particle",
            compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:item")),
                entry("item", NbtSummaryValue::string("minecraft:air")),
            ]),
        )];
        assert!(decode_attributes(&air_item).is_err());
    }

    #[test]
    fn interpolates_angle_alpha_blend_arguments_linearly() {
        let from = EnvironmentTrackArgument::FloatWithAlpha {
            value: 10.0,
            alpha: 0.25,
        };
        let to = EnvironmentTrackArgument::FloatWithAlpha {
            value: 30.0,
            alpha: 0.75,
        };
        assert_eq!(
            interpolate_timeline_argument("minecraft:visual/sun_angle", &from, &to, 0.5),
            EnvironmentTrackArgument::FloatWithAlpha {
                value: 20.0,
                alpha: 0.5,
            }
        );
    }

    #[test]
    fn sun_and_moon_angles_interpolate_linearly_across_full_day_cycle() {
        let sun_from = EnvironmentTrackArgument::Float(0.0);
        let sun_to = EnvironmentTrackArgument::Float(360.0);
        assert_eq!(
            interpolate_timeline_argument("minecraft:visual/sun_angle", &sun_from, &sun_to, 0.25),
            EnvironmentTrackArgument::Float(90.0)
        );
        assert_eq!(
            interpolate_timeline_argument("minecraft:visual/sun_angle", &sun_from, &sun_to, 0.5),
            EnvironmentTrackArgument::Float(180.0)
        );
        let moon_from = EnvironmentTrackArgument::Float(180.0);
        let moon_to = EnvironmentTrackArgument::Float(540.0);
        assert_eq!(
            interpolate_timeline_argument("minecraft:visual/moon_angle", &moon_from, &moon_to, 0.5),
            EnvironmentTrackArgument::Float(360.0)
        );
    }

    fn modifier(name: &str, operation: &str, argument: NbtSummaryValue) -> NbtSummaryEntry {
        entry(
            name,
            compound(vec![
                entry("modifier", NbtSummaryValue::string(operation)),
                entry("argument", argument),
            ]),
        )
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }
}
