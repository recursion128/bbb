use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

mod generated_26_1;

pub(crate) use generated_26_1::VANILLA_BIOME_ORDER;

use crate::{
    image::decode_png_file_rgba, resources::ResourceLocation, rgba_len, rgba_offset, PackRoots,
};

fn parse_hex_rgb(color: &str) -> Result<[u8; 3]> {
    let color = color
        .strip_prefix('#')
        .ok_or_else(|| anyhow::anyhow!("RGB color {color:?} must start with #"))?;
    if color.len() != 6 {
        bail!("RGB color #{color} must have 6 hex digits");
    }
    let value =
        u32::from_str_radix(color, 16).with_context(|| format!("parse RGB color #{color}"))?;
    Ok([
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ])
}

fn raw_rgb_attribute(
    attributes: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<[u8; 3]>> {
    attributes
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(parse_hex_rgb)
        .transpose()
}

fn raw_float_attribute_modifier(
    attributes: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<FloatAttributeModifier>> {
    let Some(value) = attributes.get(key) else {
        return Ok(None);
    };
    if let Some(argument) = value.as_f64() {
        return Ok(Some(FloatAttributeModifier::override_value(finite_f32(
            argument, key,
        )?)));
    }
    let raw: RawFloatAttributeModifier = serde_json::from_value(value.clone())
        .with_context(|| format!("parse float environment attribute {key}"))?;
    Ok(Some(FloatAttributeModifier {
        modifier: raw.modifier,
        argument: finite_f32(f64::from(raw.argument), key)?,
    }))
}

fn finite_f32(value: f64, key: &str) -> Result<f32> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        bail!("float environment attribute {key} must be finite f32");
    }
    Ok(value as f32)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FloatAttributeModifier {
    pub modifier: FloatAttributeModifierKind,
    pub argument: f32,
}

impl FloatAttributeModifier {
    pub fn override_value(argument: f32) -> Self {
        Self {
            modifier: FloatAttributeModifierKind::Override,
            argument,
        }
    }

    pub fn apply(self, base: f32) -> f32 {
        match self.modifier {
            FloatAttributeModifierKind::Override => self.argument,
            FloatAttributeModifierKind::Add => base + self.argument,
            FloatAttributeModifierKind::Subtract => base - self.argument,
            FloatAttributeModifierKind::Multiply => base * self.argument,
            FloatAttributeModifierKind::Minimum => base.min(self.argument),
            FloatAttributeModifierKind::Maximum => base.max(self.argument),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloatAttributeModifierKind {
    Override,
    Add,
    Subtract,
    Multiply,
    Minimum,
    Maximum,
}

#[derive(Debug, Deserialize)]
struct RawFloatAttributeModifier {
    modifier: FloatAttributeModifierKind,
    argument: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorMapImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ColorMapImage {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self> {
        if width == 0 || height == 0 {
            bail!("colormap image must not be empty");
        }
        let expected = rgba_len(width, height)?;
        if rgba.len() != expected {
            bail!(
                "colormap image has {} RGBA bytes, expected {} for {}x{}",
                rgba.len(),
                expected,
                width,
                height
            );
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn from_png_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let rgba = decode_png_file_rgba(path, "colormap image")?;
        let (width, height) = rgba.dimensions();
        Self::new(width, height, rgba.into_raw())
    }

    pub fn sample_temperature_downfall(&self, temperature: f32, downfall: f32) -> [u8; 3] {
        let temperature = temperature.clamp(0.0, 1.0);
        let downfall = (downfall.clamp(0.0, 1.0) * temperature).clamp(0.0, 1.0);
        let x = ((1.0 - temperature) * (self.width - 1) as f32) as u32;
        let y = ((1.0 - downfall) * (self.height - 1) as f32) as u32;
        self.rgb_at(x, y)
    }

    pub fn rgb_at(&self, x: u32, y: u32) -> [u8; 3] {
        let x = x.min(self.width - 1);
        let y = y.min(self.height - 1);
        let offset = rgba_offset(self.width, x, y).expect("valid colormap offset");
        [
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainColorMaps {
    pub grass: ColorMapImage,
    pub foliage: ColorMapImage,
    pub dry_foliage: Option<ColorMapImage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeColorCatalog {
    pub profiles: BTreeMap<i32, BiomeColorProfile>,
}

impl BiomeColorCatalog {
    pub fn new(profiles: impl IntoIterator<Item = BiomeColorProfile>) -> Self {
        Self {
            profiles: profiles
                .into_iter()
                .map(|profile| (profile.id, profile))
                .collect(),
        }
    }

    pub fn load_vanilla_26_1(roots: &PackRoots) -> Result<Self> {
        let stack = roots.resource_stack();
        let mut profiles = Vec::new();
        for (id, stem) in VANILLA_BIOME_ORDER.iter().enumerate() {
            let location =
                ResourceLocation::new("minecraft", format!("worldgen/biome/{stem}.json"))?;
            let Some(resource) = stack.get_data_resource(&location) else {
                continue;
            };
            let raw = std::fs::read_to_string(&resource.path)
                .with_context(|| format!("read biome json {}", resource.path.display()))?;
            let raw: RawBiomeColorProfile = serde_json::from_str(&raw)
                .with_context(|| format!("parse biome json {}", resource.path.display()))?;
            profiles.push(BiomeColorProfile::from_raw(
                id as i32,
                format!("minecraft:{stem}"),
                raw,
            )?);
        }
        Ok(Self::new(profiles))
    }

    pub fn profile(&self, id: i32) -> Option<&BiomeColorProfile> {
        self.profiles.get(&id)
    }

    pub fn profile_by_name(&self, name: &str) -> Option<&BiomeColorProfile> {
        self.profiles.values().find(|profile| profile.name == name)
    }

    pub fn remapped_to_resource_order<'a>(
        &self,
        resource_ids: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        Self::new(
            resource_ids
                .into_iter()
                .enumerate()
                .filter_map(|(holder_id, resource_id)| {
                    let mut profile = self.profile_by_name(resource_id)?.clone();
                    profile.id = i32::try_from(holder_id).ok()?;
                    Some(profile)
                }),
        )
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeColorProfile {
    pub id: i32,
    pub name: String,
    pub temperature: f32,
    pub temperature_modifier: BiomeTemperatureModifier,
    pub downfall: f32,
    pub has_precipitation: bool,
    pub grass_color: Option<[u8; 3]>,
    pub foliage_color: Option<[u8; 3]>,
    pub dry_foliage_color: Option<[u8; 3]>,
    pub water_color: Option<[u8; 3]>,
    pub fog_color: Option<[u8; 3]>,
    pub sky_color: Option<[u8; 3]>,
    pub water_fog_color: Option<[u8; 3]>,
    pub water_fog_end_distance: Option<FloatAttributeModifier>,
    pub grass_color_modifier: GrassColorModifier,
}

impl BiomeColorProfile {
    fn from_raw(id: i32, name: String, raw: RawBiomeColorProfile) -> Result<Self> {
        Ok(Self {
            id,
            name,
            temperature: raw.temperature,
            temperature_modifier: raw.temperature_modifier.unwrap_or_default(),
            downfall: raw.downfall,
            has_precipitation: raw.has_precipitation,
            grass_color: raw
                .effects
                .grass_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            foliage_color: raw
                .effects
                .foliage_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            dry_foliage_color: raw
                .effects
                .dry_foliage_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            water_color: raw
                .effects
                .water_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            fog_color: raw_rgb_attribute(&raw.attributes, "minecraft:visual/fog_color")?,
            sky_color: raw_rgb_attribute(&raw.attributes, "minecraft:visual/sky_color")?,
            water_fog_color: raw_rgb_attribute(
                &raw.attributes,
                "minecraft:visual/water_fog_color",
            )?,
            water_fog_end_distance: raw_float_attribute_modifier(
                &raw.attributes,
                "minecraft:visual/water_fog_end_distance",
            )?,
            grass_color_modifier: raw.effects.grass_color_modifier.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiomeTemperatureModifier {
    #[default]
    None,
    Frozen,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrassColorModifier {
    #[default]
    None,
    DarkForest,
    Swamp,
}

#[derive(Debug, Deserialize)]
struct RawBiomeColorProfile {
    temperature: f32,
    temperature_modifier: Option<BiomeTemperatureModifier>,
    downfall: f32,
    has_precipitation: bool,
    #[serde(default)]
    attributes: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    effects: RawBiomeColorEffects,
}

#[derive(Debug, Default, Deserialize)]
struct RawBiomeColorEffects {
    grass_color: Option<String>,
    foliage_color: Option<String>,
    dry_foliage_color: Option<String>,
    water_color: Option<String>,
    grass_color_modifier: Option<GrassColorModifier>,
}

#[cfg(test)]
mod tests {
    use super::{
        BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, ColorMapImage,
        GrassColorModifier,
    };

    #[test]
    fn colormap_samples_temperature_downfall_coordinates() {
        let mut rgba = Vec::new();
        for y in 0u8..4 {
            for x in 0u8..4 {
                rgba.extend([x * 10, y * 20, x + y, 255]);
            }
        }
        let colormap = ColorMapImage::new(4, 4, rgba).unwrap();

        assert_eq!(colormap.sample_temperature_downfall(1.0, 1.0), [0, 0, 0]);
        assert_eq!(colormap.sample_temperature_downfall(0.5, 1.0), [10, 20, 2]);
        assert_eq!(colormap.sample_temperature_downfall(0.0, 1.0), [30, 60, 6]);
    }

    #[test]
    fn biome_catalog_remaps_profiles_by_resource_identity() {
        let source = BiomeColorCatalog::new([
            biome_profile(10, "minecraft:plains", [1, 2, 3]),
            biome_profile(20, "minecraft:desert", [4, 5, 6]),
        ]);
        let remapped = source.remapped_to_resource_order([
            "example:custom",
            "minecraft:desert",
            "minecraft:plains",
        ]);

        assert!(remapped.profile(0).is_none());
        assert_eq!(remapped.profile(1).unwrap().name, "minecraft:desert");
        assert_eq!(remapped.profile(1).unwrap().sky_color, Some([4, 5, 6]));
        assert_eq!(remapped.profile(2).unwrap().name, "minecraft:plains");
        assert_eq!(remapped.profile(2).unwrap().sky_color, Some([1, 2, 3]));
    }

    fn biome_profile(id: i32, name: &str, sky_color: [u8; 3]) -> BiomeColorProfile {
        BiomeColorProfile {
            id,
            name: name.to_string(),
            temperature: 0.5,
            temperature_modifier: BiomeTemperatureModifier::None,
            downfall: 0.5,
            has_precipitation: true,
            grass_color: None,
            foliage_color: None,
            dry_foliage_color: None,
            water_color: None,
            fog_color: None,
            sky_color: Some(sky_color),
            water_fog_color: None,
            water_fog_end_distance: None,
            grass_color_modifier: GrassColorModifier::None,
        }
    }
}
