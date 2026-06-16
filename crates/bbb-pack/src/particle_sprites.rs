use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    particle_definitions::ParticleDefinitionCatalog, resources::ResourceLocation, roots::PackRoots,
    sprites::SpriteImage,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParticleSpriteCatalog {
    sprites: BTreeMap<String, SpriteImage>,
}

impl ParticleSpriteCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let sprites = roots
            .load_particle_texture_images()?
            .into_iter()
            .map(|sprite| (sprite.id.clone(), sprite))
            .collect();
        Ok(Self { sprites })
    }

    pub fn sprite(&self, sprite_id: &str) -> Option<&SpriteImage> {
        let sprite_id = ResourceLocation::parse(sprite_id).ok()?.id();
        self.sprites.get(&sprite_id)
    }

    pub fn sprites(&self) -> &BTreeMap<String, SpriteImage> {
        &self.sprites
    }

    pub fn missing_sprites_for_definitions(
        &self,
        definitions: &ParticleDefinitionCatalog,
    ) -> Vec<String> {
        definitions
            .unique_sprite_ids()
            .into_iter()
            .filter(|sprite_id| !self.sprites.contains_key(sprite_id))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn particle_sprite_catalog_loads_particle_atlas_images() {
        let root = unique_temp_dir("particle-sprite-catalog");
        let assets_dir = assets_dir(&root);
        write_test_png(
            &assets_dir
                .join("textures")
                .join("particle")
                .join("generic_0.png"),
            8,
            8,
        );
        write_test_png(
            &assets_dir
                .join("textures")
                .join("particle")
                .join("spark.png"),
            16,
            16,
        );
        write_json(
            &assets_dir.join("atlases").join("particles.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "",
                  "source": "particle"
                }
              ]
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_particle_sprite_catalog()
            .unwrap();

        assert_eq!(catalog.len(), 3);
        assert_eq!(
            catalog
                .sprite("minecraft:missingno")
                .map(|sprite| (sprite.width, sprite.height)),
            Some((16, 16))
        );
        assert_eq!(
            catalog
                .sprite("minecraft:generic_0")
                .map(|sprite| (sprite.width, sprite.height)),
            Some((8, 8))
        );
        assert_eq!(
            catalog
                .sprite("spark")
                .map(|sprite| (sprite.width, sprite.height)),
            Some((16, 16))
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn particle_sprite_catalog_reports_missing_definition_sprites() {
        let root = unique_temp_dir("particle-sprite-missing");
        let assets_dir = assets_dir(&root);
        write_test_png(
            &assets_dir
                .join("textures")
                .join("particle")
                .join("generic_0.png"),
            8,
            8,
        );
        write_json(
            &assets_dir.join("atlases").join("particles.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "",
                  "source": "particle"
                }
              ]
            }"#,
        );
        write_json(
            &assets_dir.join("particles").join("cloud.json"),
            r#"{
              "textures": [
                "minecraft:generic_0",
                "minecraft:missing_particle"
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let definitions = roots.load_particle_definition_catalog().unwrap();
        let sprites = roots.load_particle_sprite_catalog().unwrap();

        assert_eq!(
            sprites.missing_sprites_for_definitions(&definitions),
            vec!["minecraft:missing_particle".to_string()]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_particle_sprite_catalog() {
        let roots = PackRoots::discover().unwrap();
        let definitions = roots.load_particle_definition_catalog().unwrap();
        let sprites = roots.load_particle_sprite_catalog().unwrap();

        assert_eq!(sprites.len(), 254);
        assert!(sprites.sprite("minecraft:generic_0").is_some());
        assert!(sprites
            .missing_sprites_for_definitions(&definitions)
            .is_empty());
    }

    fn assets_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(crate::MC_VERSION)
            .join("assets")
            .join("minecraft")
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut image = image::RgbaImage::new(width, height);
        for (index, pixel) in image.pixels_mut().enumerate() {
            let shade = (index % 255) as u8;
            *pixel = image::Rgba([shade, 255 - shade, 64, 255]);
        }
        image.save(path).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
