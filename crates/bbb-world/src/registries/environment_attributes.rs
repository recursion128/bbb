use std::collections::BTreeSet;
use std::fmt;
use std::sync::{Arc, OnceLock};

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use super::environment_attribute_catalog::{
    vanilla_environment_attribute, VanillaEnvironmentAttribute, VanillaEnvironmentAttributeRange,
    VanillaEnvironmentAttributeType,
};
use super::{decode_persistent_sound_event, normalize_codec_identifier, PersistentSoundEvent};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientParticleOptions {
    pub resource_id: String,
    pub fields: Vec<NbtSummaryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientAmbientParticle {
    pub particle: ClientParticleOptions,
    pub probability_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientSoundEventHolder {
    Reference {
        resource_id: String,
    },
    Direct {
        sound_id: String,
        range_bits: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientLoopSoundState {
    pub sound: ClientSoundEventHolder,
    pub gain: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMusic {
    pub sound: ClientSoundEventHolder,
    pub min_delay: i32,
    pub max_delay: i32,
    pub replace_current_music: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientBackgroundMusic {
    pub default: Option<ClientMusic>,
    pub creative: Option<ClientMusic>,
    pub underwater: Option<ClientMusic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientAmbientMood {
    pub sound: ClientSoundEventHolder,
    pub tick_delay: i32,
    pub block_search_extent: i32,
    pub offset_bits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientAmbientAddition {
    pub sound: ClientSoundEventHolder,
    pub tick_chance_bits: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientAmbientSounds {
    pub loop_sound: Option<ClientSoundEventHolder>,
    pub mood: Option<ClientAmbientMood>,
    pub additions: Vec<ClientAmbientAddition>,
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
    pub default_dripstone_particle: Option<ClientParticleOptions>,
    pub ambient_particles: Option<Vec<ClientAmbientParticle>>,
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
    Particle(ClientParticleOptions),
    AmbientParticles(Vec<ClientAmbientParticle>),
    BackgroundMusic(ClientBackgroundMusic),
    AmbientSounds(ClientAmbientSounds),
    FilteredNonSyncable,
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
            apply_environment_argument(
                attributes,
                entry.field,
                entry.modifier,
                entry.argument.clone(),
            )?;
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
                    entry.argument.clone(),
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn same_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.entries, &other.entries)
    }

    pub(crate) fn validate_references(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        for entry in self.entries.iter() {
            validate_timeline_argument_references(&entry.argument, registry_access)?;
        }
        Ok(())
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
                apply_float_modifier(value, entry.modifier, entry.argument.clone(), entry.field)
                    .map(SpatialEnvironmentValue::Float)
            }
            SpatialEnvironmentValue::Rgb(value) => {
                apply_color_modifier(value, entry.modifier, entry.argument.clone(), entry.field)
                    .map(SpatialEnvironmentValue::Rgb)
            }
            SpatialEnvironmentValue::Argb(value) => {
                apply_color_modifier(value, entry.modifier, entry.argument.clone(), entry.field)
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
    if weighted_sources.is_empty() {
        return Ok(());
    }
    let base = attributes.clone();
    for descriptor in super::environment_attribute_catalog::VANILLA_ENVIRONMENT_ATTRIBUTES
        .iter()
        .filter(|descriptor| {
            descriptor.syncable
                && descriptor.positional
                && descriptor.spatially_interpolated
                && weighted_sources
                    .iter()
                    .any(|(source, _)| source.contains(descriptor.resource_id))
        })
    {
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

#[derive(Debug, Clone, Copy)]
enum AttributeKind {
    Float(FloatRange),
    Angle,
    Boolean,
    Rgb,
    Argb,
    MoonPhase,
    Particle,
    AmbientParticles,
    BackgroundMusic,
    AmbientSounds,
    TriState,
    Activity,
    BedRule,
}

#[derive(Debug, Clone, Copy)]
enum FloatRange {
    Any,
    NonNegative,
    Unit,
    SkyLight,
}

pub(crate) fn decode_client_environment_attributes(
    entries: &[NbtSummaryEntry],
) -> Result<ClientEnvironmentAttributes, EnvironmentAttributeProjectionError> {
    let attribute_map = decode_client_environment_attribute_map(entries, false)?;
    let mut attributes = ClientEnvironmentAttributes::default();
    attribute_map.apply_to(&mut attributes)?;
    Ok(attributes)
}

pub(crate) fn decode_client_environment_attribute_map(
    entries: &[NbtSummaryEntry],
    positional_only: bool,
) -> Result<ClientEnvironmentAttributeMap, EnvironmentAttributeProjectionError> {
    let mut decoded = Vec::with_capacity(entries.len());
    let mut seen = BTreeSet::new();
    for entry in entries {
        let Some(resource_id) = normalize_codec_identifier(&entry.name) else {
            return Err(error(
                "attributes",
                "unknown environment attribute resource id",
            ));
        };
        let Some(descriptor) = vanilla_environment_attribute(&resource_id) else {
            return Err(error(
                "attributes",
                "unknown environment attribute resource id",
            ));
        };
        if positional_only && !descriptor.positional {
            return Err(error(descriptor.resource_id, "attribute is not positional"));
        }
        if !seen.insert(descriptor.resource_id) {
            return Err(error("attributes", "duplicate environment attribute"));
        }
        let (modifier, argument) = decode_map_entry(
            descriptor.resource_id,
            attribute_kind(descriptor),
            &entry.value,
        )?;
        if descriptor.syncable {
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
) -> Result<EnvironmentTrackArgument, EnvironmentAttributeProjectionError> {
    let Some((field, kind, _)) = known_attribute(attribute) else {
        return Err(error("tracks", "unknown environment attribute"));
    };
    decode_modifier_argument(field, kind, modifier, value)
}

pub(super) fn is_syncable_environment_attribute(attribute: &str) -> bool {
    known_attribute(attribute).is_some_and(|(_, _, syncable)| syncable)
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
            Some(AttributeKind::Angle),
            EnvironmentTrackArgument::Float(from),
            EnvironmentTrackArgument::Float(to),
        ) => {
            let delta = wrap_degrees(*to - *from);
            if delta.abs() >= 90.0 {
                EnvironmentTrackArgument::Float(*to)
            } else {
                EnvironmentTrackArgument::Float(*from + alpha * delta)
            }
        }
        (
            Some(AttributeKind::Float(_)),
            EnvironmentTrackArgument::Float(from),
            EnvironmentTrackArgument::Float(to),
        ) => EnvironmentTrackArgument::Float(*from + alpha * (*to - *from)),
        (
            Some(AttributeKind::Float(_) | AttributeKind::Angle),
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
            Some(AttributeKind::Rgb | AttributeKind::Argb),
            EnvironmentTrackArgument::Rgb(from),
            EnvironmentTrackArgument::Rgb(to),
        ) => EnvironmentTrackArgument::Rgb(srgb_lerp(alpha, *from, *to)),
        (
            Some(AttributeKind::Rgb | AttributeKind::Argb),
            EnvironmentTrackArgument::Argb(from),
            EnvironmentTrackArgument::Argb(to),
        ) => EnvironmentTrackArgument::Argb(srgb_lerp(alpha, *from, *to)),
        (
            Some(AttributeKind::Rgb | AttributeKind::Argb),
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
    argument: EnvironmentTrackArgument,
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

pub(super) fn validate_client_environment_references(
    attributes: &ClientEnvironmentAttributes,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    let validate_sound = |sound: &ClientSoundEventHolder| {
        if let ClientSoundEventHolder::Reference { resource_id } = sound {
            registry_access
                .require_entry_by_resource_id("minecraft:sound_event", resource_id)
                .map_err(|error| error.to_string())?;
        }
        Ok::<_, String>(())
    };
    if let Some(background) = &attributes.background_music {
        for music in [
            background.default.as_ref(),
            background.creative.as_ref(),
            background.underwater.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_sound(&music.sound)?;
        }
    }
    if let Some(ambient) = &attributes.ambient_sounds {
        if let Some(sound) = &ambient.loop_sound {
            validate_sound(sound)?;
        }
        if let Some(mood) = &ambient.mood {
            validate_sound(&mood.sound)?;
        }
        for addition in &ambient.additions {
            validate_sound(&addition.sound)?;
        }
    }
    Ok(())
}

pub(super) fn validate_timeline_argument_references(
    argument: &EnvironmentTrackArgument,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    let mut attributes = ClientEnvironmentAttributes::default();
    match argument {
        EnvironmentTrackArgument::BackgroundMusic(value) => {
            attributes.background_music = Some(value.clone())
        }
        EnvironmentTrackArgument::AmbientSounds(value) => {
            attributes.ambient_sounds = Some(value.clone())
        }
        _ => return Ok(()),
    }
    validate_client_environment_references(&attributes, registry_access)
}

fn decode_map_entry(
    field: &'static str,
    kind: AttributeKind,
    value: &NbtSummaryValue,
) -> Result<(EnvironmentModifier, EnvironmentTrackArgument), EnvironmentAttributeProjectionError> {
    if let NbtSummaryValue::Compound(fields) = value {
        if let Some(modifier_value) = field_value(fields, "modifier") {
            let NbtSummaryValue::String(modifier_name) = modifier_value else {
                return Err(error(field, "modifier must be a string"));
            };
            let modifier = EnvironmentModifier::from_codec_name(modifier_name)
                .ok_or_else(|| error(field, "unknown attribute modifier"))?;
            let argument = field_value(fields, "argument")
                .ok_or_else(|| error(field, "modifier entry is missing argument"))?;
            return decode_modifier_argument(field, kind, modifier, argument)
                .map(|argument| (modifier, argument));
        }
    }
    decode_modifier_argument(field, kind, EnvironmentModifier::Override, value)
        .map(|argument| (EnvironmentModifier::Override, argument))
}

fn decode_modifier_argument(
    field: &'static str,
    kind: AttributeKind,
    modifier: EnvironmentModifier,
    value: &NbtSummaryValue,
) -> Result<EnvironmentTrackArgument, EnvironmentAttributeProjectionError> {
    use EnvironmentModifier as M;
    match (kind, modifier) {
        (AttributeKind::Float(range), M::Override) => parse_float(value, range)
            .map(EnvironmentTrackArgument::Float)
            .ok_or_else(|| error(field, float_range_reason(range))),
        (AttributeKind::Float(_), M::AlphaBlend) | (AttributeKind::Angle, M::AlphaBlend) => {
            parse_float_with_alpha(value)
                .map(|(value, alpha)| EnvironmentTrackArgument::FloatWithAlpha { value, alpha })
                .ok_or_else(|| error(field, "expected float or {value, alpha} argument"))
        }
        (
            AttributeKind::Float(_) | AttributeKind::Angle,
            M::Add | M::Subtract | M::Multiply | M::Minimum | M::Maximum,
        )
        | (AttributeKind::Angle, M::Override) => numeric_f32(value)
            .map(EnvironmentTrackArgument::Float)
            .ok_or_else(|| error(field, "expected float")),
        (
            AttributeKind::Boolean,
            M::Override | M::And | M::Nand | M::Or | M::Nor | M::Xor | M::Xnor,
        ) => bool_value(value)
            .map(EnvironmentTrackArgument::Boolean)
            .ok_or_else(|| error(field, "expected boolean")),
        (AttributeKind::Rgb | AttributeKind::Argb, M::AlphaBlend) => parse_argb_u32(value)
            .map(EnvironmentTrackArgument::Argb)
            .ok_or_else(|| error(field, "expected #AARRGGBB string or ARGB int")),
        (AttributeKind::Rgb, M::Override | M::Add | M::Subtract | M::Multiply) => {
            parse_rgb_u32(value)
                .map(EnvironmentTrackArgument::Rgb)
                .ok_or_else(|| error(field, "expected #RRGGBB string or RGB int"))
        }
        (AttributeKind::Argb, M::Override) => parse_argb_u32(value)
            .map(EnvironmentTrackArgument::Argb)
            .ok_or_else(|| error(field, "expected #AARRGGBB string or ARGB int")),
        (AttributeKind::Argb, M::Add | M::Subtract | M::Multiply) => parse_argb_or_rgb_u32(value)
            .map(EnvironmentTrackArgument::Argb)
            .ok_or_else(|| error(field, "expected RGB or ARGB color")),
        (AttributeKind::Rgb | AttributeKind::Argb, M::BlendToGray) => {
            let (brightness, factor) = parse_blend_to_gray(value)
                .ok_or_else(|| error(field, "expected unit brightness and factor"))?;
            Ok(EnvironmentTrackArgument::BlendToGray { brightness, factor })
        }
        (AttributeKind::MoonPhase, M::Override) => parse_moon_phase(value)
            .map(EnvironmentTrackArgument::MoonPhase)
            .ok_or_else(|| error(field, "unknown moon phase")),
        (AttributeKind::Particle, M::Override) => parse_particle(value)
            .map(EnvironmentTrackArgument::Particle)
            .ok_or_else(|| error(field, "invalid particle options")),
        (AttributeKind::AmbientParticles, M::Override) => parse_ambient_particles(value)
            .map(EnvironmentTrackArgument::AmbientParticles)
            .ok_or_else(|| error(field, "invalid ambient particle list")),
        (AttributeKind::BackgroundMusic, M::Override) => parse_background_music(value)
            .map(EnvironmentTrackArgument::BackgroundMusic)
            .ok_or_else(|| error(field, "invalid background music")),
        (AttributeKind::AmbientSounds, M::Override) => parse_ambient_sounds(value)
            .map(EnvironmentTrackArgument::AmbientSounds)
            .ok_or_else(|| error(field, "invalid ambient sounds")),
        (AttributeKind::TriState, M::Override) => parse_tri_state(value)
            .then_some(EnvironmentTrackArgument::FilteredNonSyncable)
            .ok_or_else(|| error(field, "expected boolean or tri-state name")),
        (AttributeKind::Activity, M::Override) => parse_activity(value)
            .then_some(EnvironmentTrackArgument::FilteredNonSyncable)
            .ok_or_else(|| error(field, "unknown activity")),
        (AttributeKind::BedRule, M::Override) => parse_bed_rule(value)
            .then_some(EnvironmentTrackArgument::FilteredNonSyncable)
            .ok_or_else(|| error(field, "invalid bed rule")),
        _ => Err(error(field, "modifier is not allowed for attribute type")),
    }
}

fn apply_environment_argument(
    attributes: &mut ClientEnvironmentAttributes,
    field: &'static str,
    modifier: EnvironmentModifier,
    argument: EnvironmentTrackArgument,
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
            attributes.moon_phase = Some(value);
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
            attributes.default_dripstone_particle = Some(value);
        }
        "minecraft:visual/ambient_particles" => {
            let EnvironmentTrackArgument::AmbientParticles(value) = argument else {
                return Err(error(field, "ambient particles argument type mismatch"));
            };
            attributes.ambient_particles = Some(value);
        }
        "minecraft:audio/background_music" => {
            let EnvironmentTrackArgument::BackgroundMusic(value) = argument else {
                return Err(error(field, "background music argument type mismatch"));
            };
            attributes.background_music = Some(value);
        }
        "minecraft:audio/music_volume" => apply_float!(attributes.music_volume, 1.0),
        "minecraft:audio/ambient_sounds" => {
            let EnvironmentTrackArgument::AmbientSounds(value) = argument else {
                return Err(error(field, "ambient sounds argument type mismatch"));
            };
            attributes.ambient_sounds = Some(value);
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
    argument: EnvironmentTrackArgument,
    field: &'static str,
) -> Result<f32, EnvironmentAttributeProjectionError> {
    let value = match (modifier, argument) {
        (
            EnvironmentModifier::AlphaBlend,
            EnvironmentTrackArgument::FloatWithAlpha { value, alpha },
        ) => base + alpha * (value - base),
        (EnvironmentModifier::Override, EnvironmentTrackArgument::Float(value)) => value,
        (EnvironmentModifier::Add, EnvironmentTrackArgument::Float(value)) => base + value,
        (EnvironmentModifier::Subtract, EnvironmentTrackArgument::Float(value)) => base - value,
        (EnvironmentModifier::Multiply, EnvironmentTrackArgument::Float(value)) => base * value,
        (EnvironmentModifier::Minimum, EnvironmentTrackArgument::Float(value)) => {
            java_float_min(base, value)
        }
        (EnvironmentModifier::Maximum, EnvironmentTrackArgument::Float(value)) => {
            java_float_max(base, value)
        }
        _ => return Err(error(field, "float modifier argument type mismatch")),
    };
    Ok(value)
}

fn apply_boolean_modifier(
    base: bool,
    modifier: EnvironmentModifier,
    argument: EnvironmentTrackArgument,
    field: &'static str,
) -> Result<bool, EnvironmentAttributeProjectionError> {
    let EnvironmentTrackArgument::Boolean(argument) = argument else {
        return Err(error(field, "boolean modifier argument type mismatch"));
    };
    Ok(match modifier {
        EnvironmentModifier::Override => argument,
        EnvironmentModifier::And => argument && base,
        EnvironmentModifier::Nand => !argument || !base,
        EnvironmentModifier::Or => argument || base,
        EnvironmentModifier::Nor => !argument && !base,
        EnvironmentModifier::Xor => argument ^ base,
        EnvironmentModifier::Xnor => argument == base,
        _ => return Err(error(field, "modifier is not allowed for boolean")),
    })
}

fn apply_color_modifier(
    base: u32,
    modifier: EnvironmentModifier,
    argument: EnvironmentTrackArgument,
    field: &'static str,
) -> Result<u32, EnvironmentAttributeProjectionError> {
    Ok(match (modifier, argument) {
        (EnvironmentModifier::Override, EnvironmentTrackArgument::Rgb(value)) => value,
        (EnvironmentModifier::Override, EnvironmentTrackArgument::Argb(value)) => value,
        (EnvironmentModifier::AlphaBlend, EnvironmentTrackArgument::Argb(value)) => {
            alpha_blend(base, value)
        }
        (EnvironmentModifier::Add, EnvironmentTrackArgument::Rgb(value))
        | (EnvironmentModifier::Add, EnvironmentTrackArgument::Argb(value)) => add_rgb(base, value),
        (EnvironmentModifier::Subtract, EnvironmentTrackArgument::Rgb(value))
        | (EnvironmentModifier::Subtract, EnvironmentTrackArgument::Argb(value)) => {
            subtract_rgb(base, value)
        }
        (EnvironmentModifier::Multiply, EnvironmentTrackArgument::Rgb(value))
        | (EnvironmentModifier::Multiply, EnvironmentTrackArgument::Argb(value)) => {
            multiply_argb(base, value)
        }
        (
            EnvironmentModifier::BlendToGray,
            EnvironmentTrackArgument::BlendToGray { brightness, factor },
        ) => {
            let gray = greyscale(base);
            srgb_lerp(factor, base, scale_rgb(gray, brightness))
        }
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
            VanillaEnvironmentAttributeType::Float,
            SpatialEnvironmentValue::Float(from),
            SpatialEnvironmentValue::Float(to),
        ) => SpatialEnvironmentValue::Float(from + alpha * (to - from)),
        (
            VanillaEnvironmentAttributeType::AngleDegrees,
            SpatialEnvironmentValue::Float(from),
            SpatialEnvironmentValue::Float(to),
        ) => {
            let delta = wrap_degrees(to - from);
            SpatialEnvironmentValue::Float(if delta.abs() >= 90.0 {
                to
            } else {
                from + alpha * delta
            })
        }
        (
            VanillaEnvironmentAttributeType::RgbColor,
            SpatialEnvironmentValue::Rgb(from),
            SpatialEnvironmentValue::Rgb(to),
        ) => SpatialEnvironmentValue::Rgb(srgb_lerp(alpha, from, to)),
        (
            VanillaEnvironmentAttributeType::ArgbColor,
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
    parse_rgb_u32(value).map(argb_to_rgb_bytes)
}

fn parse_rgb_u32(value: &NbtSummaryValue) -> Option<u32> {
    let rgb = match value {
        NbtSummaryValue::String(value) => parse_hex(value, 6)?,
        value => return parse_rgb_codec_u32(value),
    };
    Some(0xff00_0000 | rgb)
}

fn parse_argb_u32(value: &NbtSummaryValue) -> Option<u32> {
    match value {
        NbtSummaryValue::String(value) => Some(parse_hex(value, 8)?),
        value => parse_argb_codec_u32(value),
    }
}

fn parse_argb_or_rgb_u32(value: &NbtSummaryValue) -> Option<u32> {
    match value {
        NbtSummaryValue::String(value) => Some(parse_hex(value, 8)?),
        NbtSummaryValue::List(values) if values.len() == 3 => parse_rgb_codec_u32(value),
        value => parse_argb_codec_u32(value),
    }
}

fn parse_rgb_codec_u32(value: &NbtSummaryValue) -> Option<u32> {
    let rgb = match value {
        NbtSummaryValue::List(values) if values.len() == 3 => color(
            255,
            float_color_channel(&values[0])?,
            float_color_channel(&values[1])?,
            float_color_channel(&values[2])?,
        ),
        value => 0xff00_0000 | (numeric_i64(value)? as u32 & 0x00ff_ffff),
    };
    Some(rgb)
}

fn parse_argb_codec_u32(value: &NbtSummaryValue) -> Option<u32> {
    match value {
        NbtSummaryValue::List(values) if values.len() == 4 => Some(color(
            float_color_channel(&values[3])?,
            float_color_channel(&values[0])?,
            float_color_channel(&values[1])?,
            float_color_channel(&values[2])?,
        )),
        value => Some(numeric_i64(value)? as u32),
    }
}

fn float_color_channel(value: &NbtSummaryValue) -> Option<u32> {
    Some((numeric_f32(value)? * 255.0).floor() as i32 as u32 & 0xff)
}

fn parse_hex(value: &str, digits: usize) -> Option<u32> {
    let hex = value.strip_prefix('#')?;
    (hex.len() == digits)
        .then(|| u32::from_str_radix(hex, 16).ok())
        .flatten()
}

fn parse_float(value: &NbtSummaryValue, range: FloatRange) -> Option<f32> {
    numeric_f32(value).filter(|value| float_in_range(*value, range))
}

fn parse_float_with_alpha(value: &NbtSummaryValue) -> Option<(f32, f32)> {
    if let Some(value) = numeric_f32(value) {
        return Some((value, 1.0));
    }
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let value = numeric_f32(field_value(fields, "value")?)?;
    let alpha = field_value(fields, "alpha")
        .map(numeric_f32)
        .unwrap_or(Some(1.0))
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))?;
    Some((value, alpha))
}

fn parse_tri_state(value: &NbtSummaryValue) -> bool {
    bool_value(value).is_some()
        || matches!(
            value,
            NbtSummaryValue::String(value) if matches!(value.as_str(), "true" | "false" | "default")
        )
}

fn parse_activity(value: &NbtSummaryValue) -> bool {
    let NbtSummaryValue::String(value) = value else {
        return false;
    };
    matches!(
        value.as_str(),
        "minecraft:core"
            | "minecraft:idle"
            | "minecraft:work"
            | "minecraft:play"
            | "minecraft:rest"
            | "minecraft:meet"
            | "minecraft:panic"
            | "minecraft:raid"
            | "minecraft:pre_raid"
            | "minecraft:hide"
            | "minecraft:fight"
            | "minecraft:celebrate"
            | "minecraft:admire_item"
            | "minecraft:avoid"
            | "minecraft:ride"
            | "minecraft:play_dead"
            | "minecraft:long_jump"
            | "minecraft:ram"
            | "minecraft:tongue"
            | "minecraft:swim"
            | "minecraft:lay_spawn"
            | "minecraft:sniff"
            | "minecraft:investigate"
            | "minecraft:roar"
            | "minecraft:emerge"
            | "minecraft:dig"
    ) || normalize_codec_identifier(value).is_some_and(|normalized| {
        normalized != *value && parse_activity(&NbtSummaryValue::String(normalized))
    })
}

fn parse_bed_rule(value: &NbtSummaryValue) -> bool {
    let NbtSummaryValue::Compound(fields) = value else {
        return false;
    };
    let rule = |name| {
        matches!(
            field_value(fields, name),
            Some(NbtSummaryValue::String(value))
                if matches!(value.as_str(), "always" | "when_dark" | "never")
        )
    };
    if !rule("can_sleep") || !rule("can_set_spawn") {
        return false;
    }
    if field_value(fields, "explodes").is_some_and(|value| bool_value(value).is_none()) {
        return false;
    }
    field_value(fields, "error_message").is_none_or(|value| {
        bbb_protocol::validate_component_nbt(value, "bed rule error message").is_ok()
    })
}

fn parse_blend_to_gray(value: &NbtSummaryValue) -> Option<(f32, f32)> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let brightness = parse_float(field_value(fields, "brightness")?, FloatRange::Unit)?;
    let factor = parse_float(field_value(fields, "factor")?, FloatRange::Unit)?;
    Some((brightness, factor))
}

fn parse_moon_phase(value: &NbtSummaryValue) -> Option<ClientMoonPhase> {
    let NbtSummaryValue::String(value) = value else {
        return None;
    };
    Some(match value.as_str() {
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

fn parse_particle(value: &NbtSummaryValue) -> Option<ClientParticleOptions> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let NbtSummaryValue::String(resource_id) = field_value(fields, "type")? else {
        return None;
    };
    let resource_id = normalize_codec_identifier(resource_id)?;
    bbb_protocol::particle_types::vanilla_particle_type_id_for_resource_id(&resource_id)?;
    validate_particle_fields(&resource_id, fields)?;
    Some(ClientParticleOptions {
        resource_id,
        fields: fields.clone(),
    })
}

fn validate_particle_fields(resource_id: &str, fields: &[NbtSummaryEntry]) -> Option<()> {
    let numeric = |name| numeric_f32(field_value(fields, name)?).map(drop);
    let color = |name| parse_rgb_codec_u32(field_value(fields, name)?).map(drop);
    let argb = |name| parse_argb_codec_u32(field_value(fields, name)?).map(drop);
    match resource_id {
        "minecraft:block"
        | "minecraft:block_marker"
        | "minecraft:falling_dust"
        | "minecraft:dust_pillar"
        | "minecraft:block_crumble" => {
            validate_particle_block_state(field_value(fields, "block_state")?)
        }
        "minecraft:dragon_breath" => field_value(fields, "power")
            .map(numeric_f32)
            .transpose_option()
            .map(drop),
        "minecraft:dust" => {
            color("color")?;
            validate_particle_scale(field_value(fields, "scale")?)
        }
        "minecraft:dust_color_transition" => {
            color("from_color")?;
            color("to_color")?;
            validate_particle_scale(field_value(fields, "scale")?)
        }
        "minecraft:effect" | "minecraft:instant_effect" => {
            if field_value(fields, "color").is_some() {
                color("color")?;
            }
            if field_value(fields, "power").is_some() {
                numeric("power")?;
            }
            Some(())
        }
        "minecraft:entity_effect" | "minecraft:tinted_leaves" | "minecraft:flash" => argb("color"),
        "minecraft:sculk_charge" => numeric("roll"),
        "minecraft:item" => validate_particle_item(field_value(fields, "item")?),
        "minecraft:vibration" => validate_particle_vibration(fields),
        "minecraft:trail" => validate_particle_trail(fields),
        "minecraft:shriek" => numeric_i32(field_value(fields, "delay")?).map(drop),
        _ => Some(()),
    }
}

pub(crate) fn validate_command_particle_fields(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
) -> bool {
    validate_particle_fields(resource_id, fields).is_some()
}

fn validate_particle_scale(value: &NbtSummaryValue) -> Option<()> {
    numeric_f32(value)
        .filter(|value| (0.01..=4.0).contains(value))
        .map(drop)
}

fn validate_particle_block_state(value: &NbtSummaryValue) -> Option<()> {
    use std::collections::BTreeMap;

    let (name, properties) = match value {
        NbtSummaryValue::String(name) => (normalize_codec_identifier(name)?, BTreeMap::new()),
        NbtSummaryValue::Compound(fields) => {
            let NbtSummaryValue::String(name) = field_value(fields, "Name")? else {
                return None;
            };
            let mut properties = BTreeMap::new();
            if let Some(NbtSummaryValue::Compound(entries)) = field_value(fields, "Properties") {
                for entry in entries {
                    let NbtSummaryValue::String(value) = &entry.value else {
                        return None;
                    };
                    properties.insert(entry.name.clone(), value.clone());
                }
            } else if field_value(fields, "Properties").is_some() {
                return None;
            }
            (normalize_codec_identifier(name)?, properties)
        }
        _ => return None,
    };
    crate::vanilla_block_state_id(&name, &properties).map(drop)
}

fn validate_particle_item(value: &NbtSummaryValue) -> Option<()> {
    let item_exists = |resource_id: &str| {
        let resource_id = normalize_codec_identifier(resource_id)?;
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(&resource_id)
            .filter(|item| item.id != bbb_protocol::item_types::VANILLA_ITEM_AIR_ID)
            .map(drop)
    };
    match value {
        NbtSummaryValue::String(resource_id) => item_exists(resource_id),
        NbtSummaryValue::Compound(fields) => {
            let NbtSummaryValue::String(resource_id) = field_value(fields, "id")? else {
                return None;
            };
            item_exists(resource_id)?;
            if let Some(count) = field_value(fields, "count") {
                numeric_i32(count).filter(|count| (1..=99).contains(count))?;
            }
            if field_value(fields, "components")
                .is_some_and(|value| !matches!(value, NbtSummaryValue::Compound(_)))
            {
                return None;
            }
            Some(())
        }
        _ => None,
    }
}

fn validate_particle_vibration(fields: &[NbtSummaryEntry]) -> Option<()> {
    numeric_i32(field_value(fields, "arrival_in_ticks")?)?;
    let NbtSummaryValue::Compound(destination) = field_value(fields, "destination")? else {
        return None;
    };
    let NbtSummaryValue::String(kind) = field_value(destination, "type")? else {
        return None;
    };
    if normalize_codec_identifier(kind)? != "minecraft:block" {
        return None;
    }
    match field_value(destination, "pos")? {
        NbtSummaryValue::IntArray(values) if values.len() == 3 => Some(()),
        NbtSummaryValue::List(values)
            if values.len() == 3 && values.iter().all(|value| numeric_i32(value).is_some()) =>
        {
            Some(())
        }
        _ => None,
    }
}

fn validate_particle_trail(fields: &[NbtSummaryEntry]) -> Option<()> {
    let NbtSummaryValue::List(target) = field_value(fields, "target")? else {
        return None;
    };
    if target.len() != 3 || !target.iter().all(|value| numeric_f64(value).is_some()) {
        return None;
    }
    parse_rgb_codec_u32(field_value(fields, "color")?)?;
    numeric_i32(field_value(fields, "duration")?)
        .filter(|duration| *duration > 0)
        .map(drop)
}

fn parse_ambient_particles(value: &NbtSummaryValue) -> Option<Vec<ClientAmbientParticle>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    values
        .iter()
        .map(|value| {
            let NbtSummaryValue::Compound(fields) = value else {
                return None;
            };
            let particle = parse_particle(field_value(fields, "particle")?)?;
            let probability = parse_float(field_value(fields, "probability")?, FloatRange::Unit)?;
            Some(ClientAmbientParticle {
                particle,
                probability_bits: probability.to_bits(),
            })
        })
        .collect()
}

fn parse_background_music(value: &NbtSummaryValue) -> Option<ClientBackgroundMusic> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    Some(ClientBackgroundMusic {
        default: optional_compound(fields, "default", parse_music)?,
        creative: optional_compound(fields, "creative", parse_music)?,
        underwater: optional_compound(fields, "underwater", parse_music)?,
    })
}

fn parse_music(fields: &[NbtSummaryEntry]) -> Option<ClientMusic> {
    Some(ClientMusic {
        sound: parse_sound_holder(field_value(fields, "sound")?)?,
        min_delay: numeric_i32(field_value(fields, "min_delay")?).filter(|value| *value >= 0)?,
        max_delay: numeric_i32(field_value(fields, "max_delay")?).filter(|value| *value >= 0)?,
        replace_current_music: field_value(fields, "replace_current_music")
            .map(bool_value)
            .unwrap_or(Some(false))?,
    })
}

fn parse_ambient_sounds(value: &NbtSummaryValue) -> Option<ClientAmbientSounds> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let additions = match field_value(fields, "additions") {
        None => Vec::new(),
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .map(|value| {
                let NbtSummaryValue::Compound(fields) = value else {
                    return None;
                };
                parse_ambient_addition(fields)
            })
            .collect::<Option<Vec<_>>>()?,
        Some(NbtSummaryValue::Compound(fields)) => vec![parse_ambient_addition(fields)?],
        Some(_) => return None,
    };
    Some(ClientAmbientSounds {
        loop_sound: field_value(fields, "loop")
            .map(parse_sound_holder)
            .transpose_option()?,
        mood: optional_compound(fields, "mood", parse_ambient_mood)?,
        additions,
    })
}

fn parse_ambient_mood(fields: &[NbtSummaryEntry]) -> Option<ClientAmbientMood> {
    let offset = numeric_f64(field_value(fields, "offset")?)?;
    Some(ClientAmbientMood {
        sound: parse_sound_holder(field_value(fields, "sound")?)?,
        tick_delay: numeric_i32(field_value(fields, "tick_delay")?)?,
        block_search_extent: numeric_i32(field_value(fields, "block_search_extent")?)?,
        offset_bits: offset.to_bits(),
    })
}

fn parse_ambient_addition(fields: &[NbtSummaryEntry]) -> Option<ClientAmbientAddition> {
    let tick_chance = numeric_f64(field_value(fields, "tick_chance")?)?;
    Some(ClientAmbientAddition {
        sound: parse_sound_holder(field_value(fields, "sound")?)?,
        tick_chance_bits: tick_chance.to_bits(),
    })
}

fn parse_sound_holder(value: &NbtSummaryValue) -> Option<ClientSoundEventHolder> {
    if let NbtSummaryValue::String(raw) = value {
        return Some(ClientSoundEventHolder::Reference {
            resource_id: normalize_codec_identifier(raw)?,
        });
    }
    Some(match decode_persistent_sound_event(value, None).ok()? {
        PersistentSoundEvent::Named { .. } => return None,
        PersistentSoundEvent::Direct {
            resource_id,
            fixed_range,
        } => ClientSoundEventHolder::Direct {
            sound_id: resource_id,
            range_bits: fixed_range.map(f32::to_bits),
        },
    })
}

fn optional_compound<T>(
    fields: &[NbtSummaryEntry],
    name: &str,
    decode: impl FnOnce(&[NbtSummaryEntry]) -> Option<T>,
) -> Option<Option<T>> {
    match field_value(fields, name) {
        None => Some(None),
        Some(NbtSummaryValue::Compound(fields)) => decode(fields).map(Some),
        Some(_) => None,
    }
}

trait OptionTranspose<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}

impl<T> OptionTranspose<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(Some(value)) => Some(Some(value)),
            Some(None) => None,
        }
    }
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
        .then(|| (descriptor.resource_id, attribute_kind(descriptor)))
}

fn known_attribute(name: &str) -> Option<(&'static str, AttributeKind, bool)> {
    let descriptor = vanilla_environment_attribute(name)?;
    Some((
        descriptor.resource_id,
        attribute_kind(descriptor),
        descriptor.syncable,
    ))
}

fn attribute_kind(descriptor: &VanillaEnvironmentAttribute) -> AttributeKind {
    match descriptor.value_type {
        VanillaEnvironmentAttributeType::Boolean => AttributeKind::Boolean,
        VanillaEnvironmentAttributeType::TriState => AttributeKind::TriState,
        VanillaEnvironmentAttributeType::Float => {
            AttributeKind::Float(match descriptor.value_range {
                VanillaEnvironmentAttributeRange::Any => FloatRange::Any,
                VanillaEnvironmentAttributeRange::NonNegativeFloat => FloatRange::NonNegative,
                VanillaEnvironmentAttributeRange::UnitFloat => FloatRange::Unit,
                VanillaEnvironmentAttributeRange::SkyLight => FloatRange::SkyLight,
            })
        }
        VanillaEnvironmentAttributeType::AngleDegrees => AttributeKind::Angle,
        VanillaEnvironmentAttributeType::RgbColor => AttributeKind::Rgb,
        VanillaEnvironmentAttributeType::ArgbColor => AttributeKind::Argb,
        VanillaEnvironmentAttributeType::MoonPhase => AttributeKind::MoonPhase,
        VanillaEnvironmentAttributeType::Activity => AttributeKind::Activity,
        VanillaEnvironmentAttributeType::BedRule => AttributeKind::BedRule,
        VanillaEnvironmentAttributeType::Particle => AttributeKind::Particle,
        VanillaEnvironmentAttributeType::AmbientParticles => AttributeKind::AmbientParticles,
        VanillaEnvironmentAttributeType::BackgroundMusic => AttributeKind::BackgroundMusic,
        VanillaEnvironmentAttributeType::AmbientSounds => AttributeKind::AmbientSounds,
    }
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

fn java_float_min(left: f32, right: f32) -> f32 {
    if left.is_nan() {
        left
    } else if right.is_nan() {
        right
    } else {
        left.min(right)
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

fn float_in_range(value: f32, range: FloatRange) -> bool {
    match range {
        FloatRange::Any => true,
        FloatRange::NonNegative => value >= 0.0,
        FloatRange::Unit => (0.0..=1.0).contains(&value),
        FloatRange::SkyLight => (0.0..=15.0).contains(&value),
    }
}

fn float_range_reason(range: FloatRange) -> &'static str {
    match range {
        FloatRange::Any => "expected float",
        FloatRange::NonNegative => "float must be non-negative",
        FloatRange::Unit => "float must be within 0..=1",
        FloatRange::SkyLight => "float must be within 0..=15",
    }
}

fn field_value<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn bool_value(value: &NbtSummaryValue) -> Option<bool> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value != 0,
        NbtSummaryValue::Short(value) => *value != 0,
        NbtSummaryValue::Int(value) => *value != 0,
        NbtSummaryValue::Long(value) => *value != 0,
        _ => return None,
    })
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

fn numeric_i64(value: &NbtSummaryValue) -> Option<i64> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i64::from(*value)),
        NbtSummaryValue::Short(value) => Some(i64::from(*value)),
        NbtSummaryValue::Int(value) => Some(i64::from(*value)),
        NbtSummaryValue::Long(value) => Some(*value),
        _ => None,
    }
}

fn numeric_f32(value: &NbtSummaryValue) -> Option<f32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

fn numeric_f64(value: &NbtSummaryValue) -> Option<f64> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f64::from(*value),
        NbtSummaryValue::Short(value) => f64::from(*value),
        NbtSummaryValue::Int(value) => f64::from(*value),
        NbtSummaryValue::Long(value) => *value as f64,
        NbtSummaryValue::Float(bits) => f64::from(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits),
        _ => return None,
    })
}

fn error(field: &'static str, reason: &'static str) -> EnvironmentAttributeProjectionError {
    EnvironmentAttributeProjectionError { field, reason }
}

fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn channel(color: u32, shift: u32) -> u32 {
    color >> shift & 0xff
}

fn color(alpha: u32, red: u32, green: u32, blue: u32) -> u32 {
    (alpha & 0xff) << 24 | (red & 0xff) << 16 | (green & 0xff) << 8 | blue & 0xff
}

fn add_rgb(left: u32, right: u32) -> u32 {
    color(
        channel(left, 24),
        (channel(left, 16) + channel(right, 16)).min(255),
        (channel(left, 8) + channel(right, 8)).min(255),
        (channel(left, 0) + channel(right, 0)).min(255),
    )
}

fn subtract_rgb(left: u32, right: u32) -> u32 {
    color(
        channel(left, 24),
        channel(left, 16).saturating_sub(channel(right, 16)),
        channel(left, 8).saturating_sub(channel(right, 8)),
        channel(left, 0).saturating_sub(channel(right, 0)),
    )
}

fn multiply_argb(left: u32, right: u32) -> u32 {
    if left == u32::MAX {
        return right;
    }
    if right == u32::MAX {
        return left;
    }
    color(
        channel(left, 24) * channel(right, 24) / 255,
        channel(left, 16) * channel(right, 16) / 255,
        channel(left, 8) * channel(right, 8) / 255,
        channel(left, 0) * channel(right, 0) / 255,
    )
}

fn greyscale(value: u32) -> u32 {
    let gray = (channel(value, 16) as f32 * 0.3
        + channel(value, 8) as f32 * 0.59
        + channel(value, 0) as f32 * 0.11) as u32;
    color(channel(value, 24), gray, gray, gray)
}

fn scale_rgb(value: u32, scale: f32) -> u32 {
    color(
        channel(value, 24),
        (channel(value, 16) as f32 * scale).clamp(0.0, 255.0) as u32,
        (channel(value, 8) as f32 * scale).clamp(0.0, 255.0) as u32,
        (channel(value, 0) as f32 * scale).clamp(0.0, 255.0) as u32,
    )
}

fn alpha_blend(destination: u32, source: u32) -> u32 {
    let destination_alpha = channel(destination, 24);
    let source_alpha = channel(source, 24);
    if source_alpha == 255 {
        return source;
    }
    if source_alpha == 0 {
        return destination;
    }
    let alpha = source_alpha + destination_alpha * (255 - source_alpha) / 255;
    let blend_channel = |shift| {
        (channel(source, shift) * source_alpha
            + channel(destination, shift) * (alpha - source_alpha))
            / alpha
    };
    color(alpha, blend_channel(16), blend_channel(8), blend_channel(0))
}

fn srgb_lerp(alpha: f32, from: u32, to: u32) -> u32 {
    let lerp = |shift| {
        (channel(from, shift) as f32
            + alpha * (channel(to, shift) as f32 - channel(from, shift) as f32))
            .floor() as u32
    };
    color(lerp(24), lerp(16), lerp(8), lerp(0))
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
    use crate::{RegistryPacketEntry, WorldStore};

    use super::*;

    #[test]
    fn decodes_direct_visual_environment_attributes() {
        let entries = vec![
            entry(
                "minecraft:visual/fog_color",
                NbtSummaryValue::String("#123456".to_string()),
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
                NbtSummaryValue::Long(0x203040),
            ),
            entry(
                "minecraft:visual/sky_color",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(0.5_f32.to_bits()),
                    NbtSummaryValue::Float(0.25_f32.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                ]),
            ),
            entry(
                "minecraft:visual/sunrise_sunset_color",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(1.0_f32.to_bits()),
                    NbtSummaryValue::Float(0.5_f32.to_bits()),
                    NbtSummaryValue::Float(0.25_f32.to_bits()),
                    NbtSummaryValue::Float(0.125_f32.to_bits()),
                ]),
            ),
        ];

        let projection = decode_client_environment_attributes(&entries).unwrap();
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
                NbtSummaryValue::String("#102030".to_string()),
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

        let projection = decode_client_environment_attributes(&entries).unwrap();
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
                    NbtSummaryValue::String("minecraft:dripping_dripstone_lava".to_string()),
                )]),
            ),
            entry(
                "minecraft:audio/background_music",
                compound(vec![entry(
                    "default",
                    compound(vec![
                        entry(
                            "sound",
                            NbtSummaryValue::String("minecraft:music.game".to_string()),
                        ),
                        entry("min_delay", NbtSummaryValue::Int(10)),
                        entry("max_delay", NbtSummaryValue::Int(20)),
                    ]),
                )]),
            ),
        ];

        let projection = decode_client_environment_attributes(&entries).unwrap();
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
    fn preserves_custom_sound_references_for_frozen_registry_validation() {
        let entries = vec![entry(
            "minecraft:audio/ambient_sounds",
            compound(vec![entry(
                "loop",
                NbtSummaryValue::String("example:ambient_loop".to_string()),
            )]),
        )];
        let projection = decode_client_environment_attributes(&entries).unwrap();
        assert_eq!(
            projection
                .ambient_sounds
                .as_ref()
                .and_then(|ambient| ambient.loop_sound.as_ref()),
            Some(&ClientSoundEventHolder::Reference {
                resource_id: "example:ambient_loop".to_string(),
            })
        );

        let missing = WorldStore::default();
        assert!(validate_client_environment_references(
            &projection,
            missing.frozen_registry_access()
        )
        .is_err());

        let mut synchronized = WorldStore::default();
        synchronized.record_registry_entries(
            "minecraft:sound_event",
            0,
            vec![RegistryPacketEntry::stub("example:ambient_loop")],
        );
        validate_client_environment_references(&projection, synchronized.frozen_registry_access())
            .unwrap();
    }

    #[test]
    fn rejects_disallowed_modifiers_and_invalid_ranges() {
        let invalid_modifier = vec![modifier(
            "minecraft:visual/moon_phase",
            "add",
            NbtSummaryValue::String("full_moon".to_string()),
        )];
        assert_eq!(
            decode_client_environment_attributes(&invalid_modifier).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:visual/moon_phase",
                reason: "modifier is not allowed for attribute type",
            }
        );

        let invalid = vec![entry(
            "minecraft:visual/sky_light_factor",
            NbtSummaryValue::Double(1.5_f64.to_bits()),
        )];
        assert_eq!(
            decode_client_environment_attributes(&invalid).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:visual/sky_light_factor",
                reason: "float must be within 0..=1",
            }
        );

        let rgb_string_for_argb_modifier = vec![modifier(
            "minecraft:visual/cloud_color",
            "add",
            NbtSummaryValue::String("#123456".to_string()),
        )];
        assert!(decode_client_environment_attributes(&rgb_string_for_argb_modifier).is_err());
    }

    #[test]
    fn validates_non_syncable_values_before_filtering_them() {
        let valid = vec![
            entry("gameplay/can_start_raid", NbtSummaryValue::Byte(1)),
            entry(
                "minecraft:gameplay/villager_activity",
                NbtSummaryValue::String("idle".to_string()),
            ),
            entry(
                "minecraft:visual/sky_light_factor",
                NbtSummaryValue::Float(0.5_f32.to_bits()),
            ),
        ];
        let projection = decode_client_environment_attributes(&valid).unwrap();
        assert_eq!(projection.sky_light_factor, Some(0.5));

        let invalid = vec![entry(
            "minecraft:gameplay/villager_activity",
            NbtSummaryValue::String("example:unknown".to_string()),
        )];
        assert_eq!(
            decode_client_environment_attributes(&invalid).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:gameplay/villager_activity",
                reason: "unknown activity",
            }
        );
    }

    #[test]
    fn environment_attribute_map_preserves_modifier_for_later_base() {
        let entries = vec![modifier(
            "minecraft:visual/water_fog_end_distance",
            "multiply",
            NbtSummaryValue::Float(0.85_f32.to_bits()),
        )];
        let attribute_map = decode_client_environment_attribute_map(&entries, true).unwrap();
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
            entry("gameplay/can_start_raid", NbtSummaryValue::Byte(1)),
            entry(
                "minecraft:visual/sky_light_factor",
                NbtSummaryValue::Float(0.5_f32.to_bits()),
            ),
        ];
        let attribute_map = decode_client_environment_attribute_map(&entries, true).unwrap();
        assert_eq!(attribute_map.entries.len(), 1);

        let non_positional = vec![entry(
            "minecraft:gameplay/sky_light_level",
            NbtSummaryValue::Float(12.0_f32.to_bits()),
        )];
        assert_eq!(
            decode_client_environment_attribute_map(&non_positional, true).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:gameplay/sky_light_level",
                reason: "attribute is not positional",
            }
        );

        let malformed_filtered = vec![entry(
            "minecraft:gameplay/villager_activity",
            NbtSummaryValue::String("example:unknown".to_string()),
        )];
        assert_eq!(
            decode_client_environment_attribute_map(&malformed_filtered, true).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:gameplay/villager_activity",
                reason: "unknown activity",
            }
        );
    }

    #[test]
    fn rejects_duplicate_attribute_ids_after_namespace_normalization() {
        let duplicate = vec![
            entry(
                "visual/fog_color",
                NbtSummaryValue::String("#123456".to_string()),
            ),
            entry(
                "minecraft:visual/fog_color",
                NbtSummaryValue::String("#654321".to_string()),
            ),
        ];
        assert_eq!(
            decode_client_environment_attributes(&duplicate).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "attributes",
                reason: "duplicate environment attribute",
            }
        );
    }

    #[test]
    fn validates_particle_option_map_codecs() {
        let dust = vec![entry(
            "minecraft:visual/default_dripstone_particle",
            compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:dust".to_string()),
                ),
                entry("color", NbtSummaryValue::Int(0x12_34_56)),
                entry("scale", NbtSummaryValue::Float(0.5_f32.to_bits())),
            ]),
        )];
        let projection = decode_client_environment_attributes(&dust).unwrap();
        assert_eq!(
            projection.default_dripstone_particle.unwrap().resource_id,
            "minecraft:dust"
        );

        let missing_scale = vec![entry(
            "minecraft:visual/default_dripstone_particle",
            compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:dust".to_string()),
                ),
                entry("color", NbtSummaryValue::Int(0x12_34_56)),
            ]),
        )];
        assert!(decode_client_environment_attributes(&missing_scale).is_err());

        let air_item = vec![entry(
            "minecraft:visual/default_dripstone_particle",
            compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:item".to_string()),
                ),
                entry("item", NbtSummaryValue::String("minecraft:air".to_string())),
            ]),
        )];
        assert!(decode_client_environment_attributes(&air_item).is_err());
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

    fn modifier(name: &str, operation: &str, argument: NbtSummaryValue) -> NbtSummaryEntry {
        entry(
            name,
            compound(vec![
                entry("modifier", NbtSummaryValue::String(operation.to_string())),
                entry("argument", argument),
            ]),
        )
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }
}
