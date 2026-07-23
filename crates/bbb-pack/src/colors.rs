use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

mod generated_26_1;

pub(crate) use generated_26_1::VANILLA_BIOME_ORDER;

use crate::{
    image::{decode_png_file_rgba, decode_png_file_rgba_with_budget},
    resources::{PackResourceStack, ResourceLocation},
    rgba_len, rgba_offset, PackRoots,
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

    pub(crate) fn from_png_file_with_resource_stack(
        path: impl AsRef<Path>,
        resources: &PackResourceStack,
    ) -> Result<Self> {
        let path = path.as_ref();
        let mut budget = resources.single_image_png_decode_budget();
        let rgba = decode_png_file_rgba_with_budget(path, "colormap image", &mut budget)?;
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
            let raw: RawBiomeColorProfile = stack
                .read_json_resource(&resource, "biome color profile")
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
            biome_profile(10, "minecraft:plains"),
            biome_profile(20, "minecraft:desert"),
        ]);
        let remapped = source.remapped_to_resource_order([
            "example:custom",
            "minecraft:desert",
            "minecraft:plains",
        ]);

        assert!(remapped.profile(0).is_none());
        assert_eq!(remapped.profile(1).unwrap().name, "minecraft:desert");
        assert_eq!(remapped.profile(1).unwrap().id, 1);
        assert_eq!(remapped.profile(2).unwrap().name, "minecraft:plains");
        assert_eq!(remapped.profile(2).unwrap().id, 2);
    }

    fn biome_profile(id: i32, name: &str) -> BiomeColorProfile {
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
            grass_color_modifier: GrassColorModifier::None,
        }
    }
}
