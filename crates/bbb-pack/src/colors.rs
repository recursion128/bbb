use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use image::ImageReader;
use serde::{Deserialize, Serialize};

use crate::{resources::ResourceLocation, rgba_len, rgba_offset, PackRoots};

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
        let reader =
            ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
        let reader = reader
            .with_guessed_format()
            .with_context(|| format!("guess image format {}", path.display()))?;
        let format = reader
            .format()
            .ok_or_else(|| anyhow::anyhow!("missing image format for {}", path.display()))?;
        if format != image::ImageFormat::Png {
            bail!("colormap image {} is not a PNG", path.display());
        }
        let rgba = reader
            .decode()
            .with_context(|| format!("decode png {}", path.display()))?
            .into_rgba8();
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

pub(crate) const VANILLA_BIOME_ORDER: &[&str] = &[
    "the_void",
    "plains",
    "sunflower_plains",
    "snowy_plains",
    "ice_spikes",
    "desert",
    "swamp",
    "mangrove_swamp",
    "forest",
    "flower_forest",
    "birch_forest",
    "dark_forest",
    "pale_garden",
    "old_growth_birch_forest",
    "old_growth_pine_taiga",
    "old_growth_spruce_taiga",
    "taiga",
    "snowy_taiga",
    "savanna",
    "savanna_plateau",
    "windswept_hills",
    "windswept_gravelly_hills",
    "windswept_forest",
    "windswept_savanna",
    "jungle",
    "sparse_jungle",
    "bamboo_jungle",
    "badlands",
    "eroded_badlands",
    "wooded_badlands",
    "meadow",
    "cherry_grove",
    "grove",
    "snowy_slopes",
    "frozen_peaks",
    "jagged_peaks",
    "stony_peaks",
    "river",
    "frozen_river",
    "beach",
    "snowy_beach",
    "stony_shore",
    "warm_ocean",
    "lukewarm_ocean",
    "deep_lukewarm_ocean",
    "ocean",
    "deep_ocean",
    "cold_ocean",
    "deep_cold_ocean",
    "frozen_ocean",
    "deep_frozen_ocean",
    "mushroom_fields",
    "dripstone_caves",
    "lush_caves",
    "deep_dark",
    "nether_wastes",
    "warped_forest",
    "crimson_forest",
    "soul_sand_valley",
    "basalt_deltas",
    "the_end",
    "end_highlands",
    "end_midlands",
    "small_end_islands",
    "end_barrens",
];

#[cfg(test)]
mod tests {
    use super::ColorMapImage;

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
}
