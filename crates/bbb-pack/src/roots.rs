use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    block_models::BlockModelCatalog,
    colors::{BiomeColorCatalog, ColorMapImage, TerrainColorMaps},
    sprites::{SpriteImage, SpriteSource},
};

pub const MC_VERSION: &str = "26.1";
pub const DEFAULT_MC_CODE_ROOT: &str = "/Users/zhangguyu/Work/mc-code";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRoots {
    pub mc_code_root: PathBuf,
    pub sources_dir: PathBuf,
    pub assets_dir: PathBuf,
}

impl PackRoots {
    pub fn discover() -> Result<Self> {
        let root = std::env::var_os("BBB_MC_CODE_ROOT")
            .or_else(|| std::env::var_os("MC_CODE_ROOT"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_MC_CODE_ROOT));
        Self::from_root(root)
    }

    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self> {
        let mc_code_root = root.into();
        let sources_dir = mc_code_root.join("sources").join(MC_VERSION);
        let assets_dir = sources_dir.join("assets").join("minecraft");
        if !sources_dir.is_dir() {
            bail!("missing vanilla source directory {}", sources_dir.display());
        }
        Ok(Self {
            mc_code_root,
            sources_dir,
            assets_dir,
        })
    }

    pub fn vanilla_source(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.sources_dir.join(relative)
    }

    pub fn block_textures_dir(&self) -> PathBuf {
        self.assets_dir.join("textures").join("block")
    }

    pub fn blockstates_dir(&self) -> PathBuf {
        self.assets_dir.join("blockstates")
    }

    pub fn block_models_dir(&self) -> PathBuf {
        self.assets_dir.join("models").join("block")
    }

    pub fn biomes_dir(&self) -> PathBuf {
        self.sources_dir
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome")
    }

    pub fn colormap_texture(&self, name: &str) -> PathBuf {
        self.assets_dir
            .join("textures")
            .join("colormap")
            .join(format!("{name}.png"))
    }

    pub fn gui_sprite_texture(&self, name: &str) -> PathBuf {
        self.assets_dir
            .join("textures")
            .join("gui")
            .join("sprites")
            .join(format!("{name}.png"))
    }

    pub fn load_gui_sprite_image(&self, name: &str) -> Result<SpriteImage> {
        SpriteImage::from_png_file(
            format!("minecraft:gui/sprites/{name}"),
            self.gui_sprite_texture(name),
        )
    }

    pub fn load_block_texture_sources(&self) -> Result<Vec<SpriteSource>> {
        let dir = self.block_textures_dir();
        let mut sources = Vec::new();
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("read block texture directory {}", dir.display()))?
        {
            let entry =
                entry.with_context(|| format!("read block texture entry in {}", dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("png") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            sources.push(SpriteSource::from_png_file(
                format!("minecraft:block/{stem}"),
                &path,
            )?);
        }
        sources.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(sources)
    }

    pub fn load_block_texture_images(&self) -> Result<Vec<SpriteImage>> {
        let dir = self.block_textures_dir();
        let mut images = Vec::new();
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("read block texture directory {}", dir.display()))?
        {
            let entry =
                entry.with_context(|| format!("read block texture entry in {}", dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("png") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            images.push(SpriteImage::from_png_file(
                format!("minecraft:block/{stem}"),
                &path,
            )?);
        }
        images.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(images)
    }

    pub fn load_block_model_catalog(&self) -> Result<BlockModelCatalog> {
        BlockModelCatalog::load(self)
    }

    pub fn load_terrain_colormaps(&self) -> Result<TerrainColorMaps> {
        Ok(TerrainColorMaps {
            grass: ColorMapImage::from_png_file(self.colormap_texture("grass"))?,
            foliage: ColorMapImage::from_png_file(self.colormap_texture("foliage"))?,
            dry_foliage: Some(ColorMapImage::from_png_file(
                self.colormap_texture("dry_foliage"),
            )?),
        })
    }

    pub fn load_biome_color_catalog(&self) -> Result<BiomeColorCatalog> {
        BiomeColorCatalog::load_vanilla_26_1(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{colors::VANILLA_BIOME_ORDER, AtlasPacker, GrassColorModifier, SpriteSource};

    #[test]
    fn pack_roots_loads_sorted_block_texture_sources() {
        let root = unique_temp_dir("pack-roots");
        let block_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block");
        std::fs::create_dir_all(&block_dir).unwrap();
        write_test_png(&block_dir.join("z_stone.png"), 16, 16);
        write_test_png(&block_dir.join("a_hd_overlay.png"), 64, 32);
        std::fs::write(block_dir.join("a_hd_overlay.png.mcmeta"), "{}").unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert_eq!(
            sources,
            vec![
                SpriteSource::new("minecraft:block/a_hd_overlay", 64, 32),
                SpriteSource::new("minecraft:block/z_stone", 16, 16),
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_terrain_colormaps() {
        let root = unique_temp_dir("terrain-colormaps");
        let colormap_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("colormap");
        write_test_png(&colormap_dir.join("grass.png"), 4, 4);
        write_test_png(&colormap_dir.join("foliage.png"), 4, 4);
        write_test_png(&colormap_dir.join("dry_foliage.png"), 4, 4);

        let roots = PackRoots::from_root(&root).unwrap();
        let colormaps = roots.load_terrain_colormaps().unwrap();
        assert_eq!((colormaps.grass.width, colormaps.grass.height), (4, 4));
        assert_eq!((colormaps.foliage.width, colormaps.foliage.height), (4, 4));
        assert_eq!(
            colormaps
                .dry_foliage
                .as_ref()
                .map(|colormap| (colormap.width, colormap.height)),
            Some((4, 4))
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_biome_color_catalog_by_vanilla_id() {
        let root = unique_temp_dir("biome-color-catalog");
        let biome_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome");
        write_json(
            &biome_dir.join("plains.json"),
            r##"{
              "temperature": 0.8,
              "downfall": 0.4,
              "effects": {
                "water_color": "#123456"
              }
            }"##,
        );
        write_json(
            &biome_dir.join("swamp.json"),
            r##"{
              "temperature": 0.8,
              "downfall": 0.9,
              "effects": {
                "dry_foliage_color": "#7b5334",
                "foliage_color": "#6a7039",
                "grass_color_modifier": "swamp",
                "water_color": "#617b64"
              }
            }"##,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_biome_color_catalog().unwrap();
        let plains = catalog.profile(1).unwrap();
        assert_eq!(plains.name, "minecraft:plains");
        assert_eq!(plains.temperature, 0.8);
        assert_eq!(plains.downfall, 0.4);
        assert_eq!(plains.water_color, Some([0x12, 0x34, 0x56]));

        let swamp = catalog.profile(6).unwrap();
        assert_eq!(swamp.name, "minecraft:swamp");
        assert_eq!(swamp.foliage_color, Some([0x6a, 0x70, 0x39]));
        assert_eq!(swamp.dry_foliage_color, Some([0x7b, 0x53, 0x34]));
        assert_eq!(swamp.water_color, Some([0x61, 0x7b, 0x64]));
        assert_eq!(swamp.grass_color_modifier, GrassColorModifier::Swamp);
        assert!(catalog.profile(0).is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_block_texture_dimensions() {
        let roots = PackRoots::discover().unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert!(sources.len() > 1_000);
        let biome_colors = roots.load_biome_color_catalog().unwrap();
        assert_eq!(biome_colors.len(), VANILLA_BIOME_ORDER.len());
        let plains = biome_colors.profile(1).unwrap();
        assert_eq!(plains.name, "minecraft:plains");
        assert_eq!(plains.water_color, Some([0x3f, 0x76, 0xe4]));
        let swamp = biome_colors.profile(6).unwrap();
        assert_eq!(swamp.name, "minecraft:swamp");
        assert_eq!(swamp.grass_color_modifier, GrassColorModifier::Swamp);
        assert_eq!(swamp.foliage_color, Some([0x6a, 0x70, 0x39]));
        let colormaps = roots.load_terrain_colormaps().unwrap();
        assert_eq!((colormaps.grass.width, colormaps.grass.height), (256, 256));
        assert_eq!(
            (colormaps.foliage.width, colormaps.foliage.height),
            (256, 256)
        );
        assert_eq!(
            colormaps
                .dry_foliage
                .as_ref()
                .map(|colormap| (colormap.width, colormap.height)),
            Some((256, 256))
        );

        let stone = sources
            .iter()
            .find(|source| source.id == "minecraft:block/stone")
            .unwrap();
        assert_eq!((stone.width, stone.height), (16, 16));

        let water = sources
            .iter()
            .find(|source| source.id == "minecraft:block/water_still")
            .unwrap();
        assert_eq!(water.width, 16);
        assert!(water.height >= 16);

        let layout = AtlasPacker::new(4096, 1)
            .unwrap()
            .pack(&sources[..64])
            .unwrap();
        assert!(layout.width <= 4096);
        assert_eq!(layout.sprites.len(), 64);
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([1, 2, 3, 255]));
        image.save(path).unwrap();
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{name}-{}-{nonce}", std::process::id()))
    }
}
