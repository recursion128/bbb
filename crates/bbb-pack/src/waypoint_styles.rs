use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    resources::{PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

const DEFAULT_NEAR_DISTANCE: u32 = 128;
const DEFAULT_FAR_DISTANCE: u32 = 332;
const MAX_DISTANCE: u32 = 60_000_000;
const ICON_LOCATION_PREFIX: &str = "hud/locator_bar_dot/";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaypointStyleCatalog {
    styles: BTreeMap<String, WaypointStyle>,
}

impl WaypointStyleCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut styles = BTreeMap::new();
        for resource in stack.list_resources("waypoint_style", ".json")? {
            let style_id = style_id_from_resource(&resource.location)?;
            let bytes = std::fs::read(&resource.path)
                .with_context(|| format!("read waypoint style {}", resource.path.display()))?;
            let style = WaypointStyle::from_json_bytes(&bytes)
                .with_context(|| format!("parse waypoint style {}", resource.path.display()))?;
            styles.insert(style_id, style);
        }
        Ok(Self { styles })
    }

    pub fn style(&self, style_id: &str) -> Option<&WaypointStyle> {
        let style_id = ResourceLocation::parse(style_id).ok()?.id();
        self.styles.get(&style_id)
    }

    pub fn style_or_missing(&self, style_id: &str) -> WaypointStyle {
        self.style(style_id)
            .cloned()
            .unwrap_or_else(WaypointStyle::missing)
    }

    pub fn styles(&self) -> &BTreeMap<String, WaypointStyle> {
        &self.styles
    }

    pub fn len(&self) -> usize {
        self.styles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaypointStyle {
    pub near_distance: u32,
    pub far_distance: u32,
    pub sprites: Vec<String>,
    pub sprite_locations: Vec<String>,
}

impl WaypointStyle {
    pub fn new(near_distance: u32, far_distance: u32, sprites: Vec<String>) -> Result<Self> {
        validate_distance("near_distance", near_distance)?;
        validate_distance("far_distance", far_distance)?;
        if sprites.is_empty() {
            bail!("waypoint style must have at least one sprite icon");
        }
        if near_distance == 0 {
            bail!("waypoint style near_distance must be greater than zero");
        }
        if near_distance >= far_distance {
            bail!("waypoint style far_distance must be greater than near_distance");
        }

        let sprites = sprites
            .into_iter()
            .map(|sprite| ResourceLocation::parse(&sprite).map(|location| location.id()))
            .collect::<Result<Vec<_>>>()?;
        let sprite_locations = sprites
            .iter()
            .map(|sprite| prefixed_sprite_location(sprite))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            near_distance,
            far_distance,
            sprites,
            sprite_locations,
        })
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let raw: RawWaypointStyle = serde_json::from_slice(bytes)?;
        raw.into_style()
    }

    pub fn missing() -> Self {
        Self {
            near_distance: 0,
            far_distance: 1,
            sprites: vec!["minecraft:missingno".to_string()],
            sprite_locations: vec!["minecraft:hud/locator_bar_dot/missingno".to_string()],
        }
    }

    pub fn sprite_location_for_distance(&self, distance: f32) -> &str {
        if distance < self.near_distance as f32 {
            return self.sprite_locations.first().expect("style has sprites");
        }
        if distance >= self.far_distance as f32 {
            return self.sprite_locations.last().expect("style has sprites");
        }
        if self.sprite_locations.len() == 1 {
            return self.sprite_locations.first().expect("style has sprites");
        }
        if self.sprite_locations.len() == 3 {
            return &self.sprite_locations[1];
        }

        let alpha = (distance - self.near_distance as f32)
            / (self.far_distance - self.near_distance) as f32;
        let index = 1 + (alpha * (self.sprite_locations.len() as f32 - 2.0)).floor() as usize;
        &self.sprite_locations[index]
    }
}

#[derive(Debug, Deserialize)]
struct RawWaypointStyle {
    #[serde(default = "default_near_distance")]
    near_distance: u32,
    #[serde(default = "default_far_distance")]
    far_distance: u32,
    sprites: Vec<String>,
}

impl RawWaypointStyle {
    fn into_style(self) -> Result<WaypointStyle> {
        WaypointStyle::new(self.near_distance, self.far_distance, self.sprites)
    }
}

fn default_near_distance() -> u32 {
    DEFAULT_NEAR_DISTANCE
}

fn default_far_distance() -> u32 {
    DEFAULT_FAR_DISTANCE
}

fn validate_distance(field: &str, distance: u32) -> Result<()> {
    if distance > MAX_DISTANCE {
        bail!("waypoint style {field} must be at most {MAX_DISTANCE}");
    }
    Ok(())
}

fn prefixed_sprite_location(sprite: &str) -> Result<String> {
    let sprite = ResourceLocation::parse(sprite)?;
    ResourceLocation::new(
        sprite.namespace().to_string(),
        format!("{ICON_LOCATION_PREFIX}{}", sprite.path()),
    )
    .map(|location| location.id())
}

fn style_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("waypoint_style/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "waypoint style resource {} is outside waypoint_style",
                location.id()
            )
        })?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn waypoint_style_catalog_loads_defaults_and_sprite_locations() {
        let root = unique_temp_dir("waypoint-style-defaults");
        write_json(
            &style_dir(&root).join("default.json"),
            r#"{
              "sprites": [
                "minecraft:default_0",
                "minecraft:default_1",
                "minecraft:default_2",
                "minecraft:default_3"
              ]
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_waypoint_style_catalog()
            .unwrap();
        let style = catalog.style("default").unwrap();

        assert_eq!(style.near_distance, DEFAULT_NEAR_DISTANCE);
        assert_eq!(style.far_distance, DEFAULT_FAR_DISTANCE);
        assert_eq!(
            style.sprite_locations,
            vec![
                "minecraft:hud/locator_bar_dot/default_0".to_string(),
                "minecraft:hud/locator_bar_dot/default_1".to_string(),
                "minecraft:hud/locator_bar_dot/default_2".to_string(),
                "minecraft:hud/locator_bar_dot/default_3".to_string(),
            ]
        );
        assert_eq!(
            style.sprite_location_for_distance(50.0),
            "minecraft:hud/locator_bar_dot/default_0"
        );
        assert_eq!(
            style.sprite_location_for_distance(200.0),
            "minecraft:hud/locator_bar_dot/default_1"
        );
        assert_eq!(
            style.sprite_location_for_distance(331.0),
            "minecraft:hud/locator_bar_dot/default_2"
        );
        assert_eq!(
            style.sprite_location_for_distance(332.0),
            "minecraft:hud/locator_bar_dot/default_3"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn waypoint_style_catalog_uses_resource_pack_overrides() {
        let root = unique_temp_dir("waypoint-style-pack");
        let pack = root.join("resource-pack");
        write_json(
            &style_dir(&root).join("bowtie.json"),
            r#"{
              "near_distance": 64,
              "sprites": [
                "minecraft:bowtie",
                "minecraft:default_0"
              ]
            }"#,
        );
        write_json(
            &pack
                .join("assets")
                .join("minecraft")
                .join("waypoint_style")
                .join("bowtie.json"),
            r#"{
              "near_distance": 32,
              "far_distance": 96,
              "sprites": [
                "custom:near",
                "custom:middle",
                "custom:far"
              ]
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack])
            .load_waypoint_style_catalog()
            .unwrap();
        let style = catalog.style("minecraft:bowtie").unwrap();

        assert_eq!(style.near_distance, 32);
        assert_eq!(style.far_distance, 96);
        assert_eq!(
            style.sprite_location_for_distance(80.0),
            "custom:hud/locator_bar_dot/middle"
        );
        assert_eq!(
            catalog.style_or_missing("missing").sprite_locations,
            vec!["minecraft:hud/locator_bar_dot/missingno".to_string()]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn waypoint_style_catalog_rejects_invalid_styles() {
        let empty =
            WaypointStyle::from_json_bytes(br#"{"near_distance":1,"far_distance":2,"sprites":[]}"#)
                .unwrap_err();
        assert!(empty.to_string().contains("at least one sprite"));

        let zero_near = WaypointStyle::from_json_bytes(
            br#"{"near_distance":0,"far_distance":2,"sprites":["minecraft:a"]}"#,
        )
        .unwrap_err();
        assert!(zero_near.to_string().contains("greater than zero"));

        let inverted = WaypointStyle::from_json_bytes(
            br#"{"near_distance":5,"far_distance":5,"sprites":["minecraft:a"]}"#,
        )
        .unwrap_err();
        assert!(inverted.to_string().contains("greater than near_distance"));
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_waypoint_styles() {
        let catalog = PackRoots::discover()
            .unwrap()
            .load_waypoint_style_catalog()
            .unwrap();
        assert_eq!(catalog.len(), 2);

        let default = catalog.style("minecraft:default").unwrap();
        assert_eq!(default.near_distance, 128);
        assert_eq!(default.far_distance, 332);
        assert_eq!(default.sprites.len(), 4);

        let bowtie = catalog.style("minecraft:bowtie").unwrap();
        assert_eq!(bowtie.near_distance, 64);
        assert_eq!(bowtie.far_distance, 332);
        assert_eq!(bowtie.sprites.len(), 5);
        assert_eq!(bowtie.sprites[0], "minecraft:bowtie");
    }

    fn style_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(crate::MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("waypoint_style")
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
