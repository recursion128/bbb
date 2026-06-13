use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

mod atlas;
mod block_models;
mod colors;
mod sprites;

pub use atlas::{AtlasImage, AtlasLayout, AtlasPacker, AtlasRect, AtlasSprite};
pub use block_models::{
    BlockFaceTextures, BlockModelBox, BlockModelCatalog, BlockModelFace, BlockModelShape,
    BlockRenderModel,
};
pub use colors::{
    BiomeColorCatalog, BiomeColorProfile, ColorMapImage, GrassColorModifier, TerrainColorMaps,
};
pub use sprites::{SpriteImage, SpriteSource};

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

pub(crate) fn rgba_offset(width: u32, x: u32, y: u32) -> Result<usize> {
    let pixel = y
        .checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .ok_or_else(|| anyhow::anyhow!("RGBA offset overflow"))?;
    usize::try_from(pixel)
        .ok()
        .and_then(|pixel| pixel.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("RGBA offset overflow"))
}

pub(crate) fn rgba_len(width: u32, height: u32) -> Result<usize> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| anyhow::anyhow!("RGBA image size overflow"))?;
    usize::try_from(pixels)
        .ok()
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("RGBA image size overflow"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn atlas_rects_preserve_content_dimensions_inside_padding() {
        let layout = AtlasPacker::new(128, 2)
            .unwrap()
            .pack(&[
                SpriteSource::new("minecraft:block/stone", 16, 16),
                SpriteSource::new("pack:block/hd_overlay", 64, 32),
            ])
            .unwrap();

        assert_eq!(layout.width, 88);
        assert_eq!(layout.height, 36);
        assert_eq!(layout.padding, 2);

        let stone = &layout.sprites[0];
        assert_eq!(stone.source_width, 16);
        assert_eq!(stone.source_height, 16);
        assert_eq!(
            stone.padded,
            AtlasRect {
                x: 0,
                y: 0,
                width: 20,
                height: 20
            }
        );
        assert_eq!(
            stone.content,
            AtlasRect {
                x: 2,
                y: 2,
                width: 16,
                height: 16
            }
        );

        let overlay = &layout.sprites[1];
        assert_eq!(
            overlay.padded,
            AtlasRect {
                x: 20,
                y: 0,
                width: 68,
                height: 36
            }
        );
        assert_eq!(
            overlay.content,
            AtlasRect {
                x: 22,
                y: 2,
                width: 64,
                height: 32
            }
        );
    }

    #[test]
    fn atlas_packer_wraps_rows_for_mixed_resolution_sprites() {
        let layout = AtlasPacker::new(300, 1)
            .unwrap()
            .pack(&[
                SpriteSource::new("pack:block/large", 256, 256),
                SpriteSource::new("pack:block/medium", 64, 64),
                SpriteSource::new("minecraft:block/small", 16, 16),
            ])
            .unwrap();

        assert_eq!(layout.width, 258);
        assert_eq!(layout.height, 324);
        assert_eq!(
            layout.sprites[0].content,
            AtlasRect {
                x: 1,
                y: 1,
                width: 256,
                height: 256
            }
        );
        assert_eq!(
            layout.sprites[1].content,
            AtlasRect {
                x: 1,
                y: 259,
                width: 64,
                height: 64
            }
        );
        assert_eq!(
            layout.sprites[2].content,
            AtlasRect {
                x: 67,
                y: 259,
                width: 16,
                height: 16
            }
        );
    }

    #[test]
    fn atlas_packer_rejects_invalid_sprite_dimensions() {
        let zero = AtlasPacker::new(128, 1)
            .unwrap()
            .pack(&[SpriteSource::new("bad", 0, 16)]);
        assert!(zero.is_err());

        let too_wide = AtlasPacker::new(16, 1)
            .unwrap()
            .pack(&[SpriteSource::new("wide", 16, 16)]);
        assert!(too_wide.is_err());
    }

    #[test]
    fn atlas_stitcher_extends_sprite_edges_into_padding() {
        let image = SpriteImage::new(
            "test:quad",
            2,
            2,
            vec![10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255],
        )
        .unwrap();
        let atlas = AtlasPacker::new(8, 1).unwrap().stitch(&[image]).unwrap();

        assert_eq!(atlas.layout.width, 4);
        assert_eq!(atlas.layout.height, 4);
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 0, 0),
            [10, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 3, 0),
            [20, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 0, 3),
            [30, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 3, 3),
            [40, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 1, 1),
            [10, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 2, 2),
            [40, 0, 0, 255]
        );
    }

    #[test]
    fn sprite_source_reads_png_dimensions() {
        let dir = unique_temp_dir("png-dimensions");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sprite.png");
        write_test_png(&path, 7, 11);

        let source = SpriteSource::from_png_file("test:sprite", &path).unwrap();
        assert_eq!(source, SpriteSource::new("test:sprite", 7, 11));

        std::fs::remove_dir_all(dir).unwrap();
    }

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
    fn block_model_catalog_resolves_parent_texture_aliases_and_variants() {
        let root = unique_temp_dir("block-model-catalog");
        write_json(
            &root
                .join("sources")
                .join(MC_VERSION)
                .join("assets")
                .join("minecraft")
                .join("blockstates")
                .join("grass_block.json"),
            r##"{
                "variants": {
                    "snowy=false": { "model": "minecraft:block/grass_block" },
                    "snowy=true": { "model": "minecraft:block/grass_block_snow" }
                }
            }"##,
        );
        write_json(
            &root
                .join("sources")
                .join(MC_VERSION)
                .join("assets")
                .join("minecraft")
                .join("models")
                .join("block")
                .join("cube.json"),
            r##"{
                "elements": [{
                    "faces": {
                        "down": { "texture": "#down" },
                        "up": { "texture": "#up", "tintindex": 0 },
                        "north": { "texture": "#north", "tintindex": 0 },
                        "south": { "texture": "#south", "tintindex": 0 },
                        "west": { "texture": "#west", "tintindex": 0 },
                        "east": { "texture": "#east", "tintindex": 0 }
                    }
                }]
            }"##,
        );
        write_json(
            &root
                .join("sources")
                .join(MC_VERSION)
                .join("assets")
                .join("minecraft")
                .join("models")
                .join("block")
                .join("cube_bottom_top.json"),
            r##"{
                "parent": "block/cube",
                "textures": {
                    "particle": "#side",
                    "down": "#bottom",
                    "up": "#top",
                    "north": "#side",
                    "south": "#side",
                    "west": "#side",
                    "east": "#side"
                }
            }"##,
        );
        write_json(
            &root
                .join("sources")
                .join(MC_VERSION)
                .join("assets")
                .join("minecraft")
                .join("models")
                .join("block")
                .join("grass_block.json"),
            r##"{
                "parent": "minecraft:block/cube_bottom_top",
                "textures": {
                    "bottom": "block/dirt",
                    "top": "block/grass_block_top",
                    "side": "block/grass_block_side"
                }
            }"##,
        );
        write_json(
            &root
                .join("sources")
                .join(MC_VERSION)
                .join("assets")
                .join("minecraft")
                .join("models")
                .join("block")
                .join("grass_block_snow.json"),
            r##"{
                "parent": "minecraft:block/cube_bottom_top",
                "textures": {
                    "bottom": "block/dirt",
                    "top": { "force_translucent": true, "sprite": "block/snow" },
                    "side": "block/grass_block_snow"
                }
            }"##,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap();
        let mut properties = BTreeMap::new();
        properties.insert("snowy".to_string(), "false".to_string());
        let render_model = catalog
            .block_render_model("minecraft:grass_block", &properties)
            .unwrap();
        assert_eq!(render_model.shape, BlockModelShape::Cube);
        let textures = render_model.face_textures;

        assert_eq!(textures.get(BlockModelFace::Down), "minecraft:block/dirt");
        assert_eq!(
            textures.get(BlockModelFace::Up),
            "minecraft:block/grass_block_top"
        );
        assert_eq!(
            textures.get(BlockModelFace::North),
            "minecraft:block/grass_block_side"
        );
        assert_eq!(
            textures.get(BlockModelFace::East),
            "minecraft:block/grass_block_side"
        );
        assert_eq!(textures.tint_index(BlockModelFace::Down), None);
        assert_eq!(textures.tint_index(BlockModelFace::Up), Some(0));
        assert_eq!(textures.tint_index(BlockModelFace::North), Some(0));

        properties.insert("snowy".to_string(), "true".to_string());
        let snowy = catalog
            .block_render_model("minecraft:grass_block", &properties)
            .unwrap();
        assert_eq!(snowy.shape, BlockModelShape::Cube);
        assert_eq!(
            snowy.face_textures.get(BlockModelFace::Up),
            "minecraft:block/snow"
        );
        assert_eq!(snowy.face_textures.tint_index(BlockModelFace::Up), Some(0));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn block_model_catalog_classifies_cross_models() {
        let root = unique_temp_dir("block-model-cross");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &asset_root.join("blockstates").join("dandelion.json"),
            r##"{
                "variants": {
                    "": { "model": "minecraft:block/dandelion" }
                }
            }"##,
        );
        write_json(
            &asset_root.join("models").join("block").join("cross.json"),
            r##"{
                "textures": { "particle": "#cross" },
                "elements": [
                    {
                        "faces": {
                            "north": { "texture": "#cross" },
                            "south": { "texture": "#cross" }
                        }
                    },
                    {
                        "faces": {
                            "west": { "texture": "#cross" },
                            "east": { "texture": "#cross" }
                        }
                    }
                ]
            }"##,
        );
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join("dandelion.json"),
            r##"{
                "parent": "minecraft:block/cross",
                "textures": { "cross": "minecraft:block/dandelion" }
            }"##,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap();
        let properties = BTreeMap::new();
        let render_model = catalog
            .block_render_model("minecraft:dandelion", &properties)
            .unwrap();

        assert_eq!(render_model.shape, BlockModelShape::Cross);
        assert_eq!(
            render_model.face_textures.get(BlockModelFace::North),
            "minecraft:block/dandelion"
        );
        assert_eq!(
            render_model.face_textures.get(BlockModelFace::Up),
            "minecraft:block/dandelion"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn block_model_catalog_uses_particle_texture_for_elementless_models() {
        let root = unique_temp_dir("block-model-particle-only");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &asset_root.join("blockstates").join("water.json"),
            r##"{
                "variants": {
                    "": { "model": "minecraft:block/water" }
                }
            }"##,
        );
        write_json(
            &asset_root.join("models").join("block").join("water.json"),
            r##"{
                "textures": {
                    "particle": "block/water_still"
                }
            }"##,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap();
        let render_model = catalog
            .block_render_model("minecraft:water", &BTreeMap::new())
            .unwrap();

        assert_eq!(render_model.shape, BlockModelShape::Custom);
        assert_eq!(
            render_model.face_textures.get(BlockModelFace::Up),
            "minecraft:block/water_still"
        );
        assert_eq!(
            render_model.face_textures.get(BlockModelFace::North),
            "minecraft:block/water_still"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn block_model_catalog_extracts_single_box_geometry() {
        let root = unique_temp_dir("block-model-box");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &asset_root.join("blockstates").join("oak_slab.json"),
            r##"{
                "variants": {
                    "type=bottom": { "model": "minecraft:block/oak_slab" }
                }
            }"##,
        );
        write_json(
            &asset_root.join("models").join("block").join("slab.json"),
            r##"{
                "elements": [{
                    "from": [0, 0, 0],
                    "to": [16, 8, 16],
                    "faces": {
                        "down":  { "uv": [0, 0, 16, 16], "texture": "#bottom", "cullface": "down" },
                        "up":    { "uv": [0, 0, 16, 16], "texture": "#top" },
                        "north": { "uv": [0, 8, 16, 16], "texture": "#side", "cullface": "north" },
                        "south": { "uv": [0, 8, 16, 16], "texture": "#side", "cullface": "south" },
                        "west":  { "uv": [0, 8, 16, 16], "texture": "#side", "cullface": "west" },
                        "east":  { "uv": [0, 8, 16, 16], "texture": "#side", "cullface": "east" }
                    }
                }]
            }"##,
        );
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join("oak_slab.json"),
            r##"{
                "parent": "minecraft:block/slab",
                "textures": {
                    "bottom": "minecraft:block/oak_planks",
                    "side": "minecraft:block/oak_planks",
                    "top": "minecraft:block/oak_planks"
                }
            }"##,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap();
        let mut properties = BTreeMap::new();
        properties.insert("type".to_string(), "bottom".to_string());
        let render_model = catalog
            .block_render_model("minecraft:oak_slab", &properties)
            .unwrap();
        let BlockModelShape::Box(model_box) = render_model.shape else {
            panic!("oak_slab should resolve to a box model");
        };

        assert_eq!(model_box.from, [0, 0, 0]);
        assert_eq!(model_box.to, [16, 8, 16]);
        assert_eq!(
            model_box.face_uvs[BlockModelFace::North.index()],
            [0, 8, 16, 16]
        );
        assert!(model_box.face_cull[BlockModelFace::North.index()]);
        assert!(!model_box.face_cull[BlockModelFace::Up.index()]);
        assert_eq!(
            render_model.face_textures.get(BlockModelFace::North),
            "minecraft:block/oak_planks"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn block_model_catalog_combines_multipart_boxes() {
        let root = unique_temp_dir("block-model-multipart-boxes");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &asset_root.join("blockstates").join("oak_fence.json"),
            r##"{
                "multipart": [
                    { "apply": { "model": "minecraft:block/oak_fence_post" } },
                    {
                        "when": { "north": "true" },
                        "apply": { "model": "minecraft:block/oak_fence_side" }
                    },
                    {
                        "when": { "east": "true" },
                        "apply": { "model": "minecraft:block/oak_fence_side", "y": 90 }
                    }
                ]
            }"##,
        );
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join("oak_fence_post.json"),
            r##"{
                "textures": { "particle": "#texture", "texture": "minecraft:block/oak_planks" },
                "elements": [{
                    "from": [6, 0, 6],
                    "to": [10, 16, 10],
                    "faces": {
                        "down":  { "texture": "#texture" },
                        "up":    { "texture": "#texture" },
                        "north": { "texture": "#texture" },
                        "south": { "texture": "#texture" },
                        "west":  { "texture": "#texture" },
                        "east":  { "texture": "#texture" }
                    }
                }]
            }"##,
        );
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join("oak_fence_side.json"),
            r##"{
                "textures": { "particle": "#texture", "texture": "minecraft:block/oak_planks" },
                "elements": [{
                    "from": [7, 6, 0],
                    "to": [9, 15, 8],
                    "faces": {
                        "up":    { "texture": "#texture" },
                        "north": { "texture": "#texture" },
                        "south": { "texture": "#texture" },
                        "west":  { "texture": "#texture" },
                        "east":  { "texture": "#texture" }
                    }
                }]
            }"##,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap();
        let mut properties = BTreeMap::new();
        properties.insert("north".to_string(), "true".to_string());
        properties.insert("east".to_string(), "true".to_string());
        let render_model = catalog
            .block_render_model("minecraft:oak_fence", &properties)
            .unwrap();
        let BlockModelShape::Boxes(boxes) = render_model.shape else {
            panic!("oak_fence multipart should combine post and side boxes");
        };

        assert_eq!(boxes.len(), 3);
        assert_eq!(boxes[0].from, [6, 0, 6]);
        assert_eq!(boxes[1].from, [7, 6, 0]);
        assert_eq!(boxes[2].from, [0, 6, 7]);
        assert!(!boxes[1].face_present[BlockModelFace::Down.index()]);
        assert_eq!(
            render_model.face_textures.get(BlockModelFace::North),
            "minecraft:block/oak_planks"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn block_model_catalog_applies_blockstate_rotation_to_faces() {
        let root = unique_temp_dir("block-model-rotation");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &asset_root.join("blockstates").join("oak_log.json"),
            r##"{
                "variants": {
                    "axis=x": { "model": "minecraft:block/oak_log", "x": 90, "y": 90 },
                    "axis=y": { "model": "minecraft:block/oak_log" },
                    "axis=z": { "model": "minecraft:block/oak_log", "x": 90 }
                }
            }"##,
        );
        write_json(
            &asset_root.join("models").join("block").join("cube.json"),
            r##"{
                "elements": [{
                    "faces": {
                        "down": { "texture": "#down" },
                        "up": { "texture": "#up" },
                        "north": { "texture": "#north" },
                        "south": { "texture": "#south" },
                        "west": { "texture": "#west" },
                        "east": { "texture": "#east" }
                    }
                }]
            }"##,
        );
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join("cube_column.json"),
            r##"{
                "parent": "block/cube",
                "textures": {
                    "particle": "#side",
                    "down": "#end",
                    "up": "#end",
                    "north": "#side",
                    "south": "#side",
                    "west": "#side",
                    "east": "#side"
                }
            }"##,
        );
        write_json(
            &asset_root.join("models").join("block").join("oak_log.json"),
            r##"{
                "parent": "minecraft:block/cube_column",
                "textures": {
                    "end": "minecraft:block/oak_log_top",
                    "side": "minecraft:block/oak_log"
                }
            }"##,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap();
        let mut properties = BTreeMap::new();
        properties.insert("axis".to_string(), "y".to_string());
        let vertical = catalog
            .block_face_textures("minecraft:oak_log", &properties)
            .unwrap();
        assert_eq!(
            vertical.get(BlockModelFace::Down),
            "minecraft:block/oak_log_top"
        );
        assert_eq!(
            vertical.get(BlockModelFace::North),
            "minecraft:block/oak_log"
        );

        properties.insert("axis".to_string(), "x".to_string());
        let east_west = catalog
            .block_face_textures("minecraft:oak_log", &properties)
            .unwrap();
        assert_eq!(
            east_west.get(BlockModelFace::West),
            "minecraft:block/oak_log_top"
        );
        assert_eq!(
            east_west.get(BlockModelFace::East),
            "minecraft:block/oak_log_top"
        );
        assert_eq!(
            east_west.get(BlockModelFace::Down),
            "minecraft:block/oak_log"
        );

        properties.insert("axis".to_string(), "z".to_string());
        let north_south = catalog
            .block_face_textures("minecraft:oak_log", &properties)
            .unwrap();
        assert_eq!(
            north_south.get(BlockModelFace::North),
            "minecraft:block/oak_log_top"
        );
        assert_eq!(
            north_south.get(BlockModelFace::South),
            "minecraft:block/oak_log_top"
        );
        assert_eq!(
            north_south.get(BlockModelFace::Up),
            "minecraft:block/oak_log"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_block_texture_dimensions() {
        let roots = PackRoots::discover().unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert!(sources.len() > 1_000);
        let biome_colors = roots.load_biome_color_catalog().unwrap();
        assert_eq!(biome_colors.len(), colors::VANILLA_BIOME_ORDER.len());
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

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_block_model_catalog() {
        let roots = PackRoots::discover().unwrap();
        let catalog = roots.load_block_model_catalog().unwrap();
        assert!(catalog.len() > 1_000);

        let mut grass = BTreeMap::new();
        grass.insert("snowy".to_string(), "false".to_string());
        let grass_model = catalog
            .block_render_model("minecraft:grass_block", &grass)
            .unwrap();
        let BlockModelShape::Boxes(grass_boxes) = &grass_model.shape else {
            panic!("grass_block should preserve base and overlay boxes");
        };
        assert_eq!(grass_boxes.len(), 2);
        assert_eq!(
            grass_boxes[0].face_textures[BlockModelFace::North.index()].as_deref(),
            Some("minecraft:block/grass_block_side")
        );
        assert_eq!(
            grass_boxes[1].face_textures[BlockModelFace::North.index()].as_deref(),
            Some("minecraft:block/grass_block_side_overlay")
        );
        assert_eq!(
            grass_boxes[1].face_tint_indices[BlockModelFace::North.index()],
            Some(0)
        );
        assert_eq!(
            grass_model.face_textures.get(BlockModelFace::Down),
            "minecraft:block/dirt"
        );
        assert_eq!(
            grass_model.face_textures.get(BlockModelFace::Up),
            "minecraft:block/grass_block_top"
        );
        assert_eq!(
            grass_model.face_textures.tint_index(BlockModelFace::Down),
            None
        );
        assert_eq!(
            grass_model.face_textures.tint_index(BlockModelFace::Up),
            Some(0)
        );

        let mut log = BTreeMap::new();
        log.insert("axis".to_string(), "x".to_string());
        let log_model = catalog
            .block_render_model("minecraft:oak_log", &log)
            .unwrap();
        assert_eq!(log_model.shape, BlockModelShape::Cube);
        assert_eq!(
            log_model.face_textures.get(BlockModelFace::West),
            "minecraft:block/oak_log_top"
        );

        let mut slab = BTreeMap::new();
        slab.insert("type".to_string(), "bottom".to_string());
        let slab_model = catalog
            .block_render_model("minecraft:oak_slab", &slab)
            .unwrap();
        let BlockModelShape::Box(slab_box) = slab_model.shape else {
            panic!("oak_slab bottom should resolve to a box model");
        };
        assert_eq!(slab_box.from, [0, 0, 0]);
        assert_eq!(slab_box.to, [16, 8, 16]);
        assert_eq!(
            slab_box.face_uvs[BlockModelFace::North.index()],
            [0, 8, 16, 16]
        );

        let mut stairs = BTreeMap::new();
        stairs.insert("facing".to_string(), "east".to_string());
        stairs.insert("half".to_string(), "bottom".to_string());
        stairs.insert("shape".to_string(), "straight".to_string());
        let stairs_model = catalog
            .block_render_model("minecraft:oak_stairs", &stairs)
            .unwrap();
        let BlockModelShape::Boxes(stair_boxes) = stairs_model.shape else {
            panic!("oak_stairs straight should resolve to multi-box geometry");
        };
        assert_eq!(stair_boxes.len(), 2);
        assert!(!stair_boxes[1].face_present[BlockModelFace::Down.index()]);

        let mut fence = BTreeMap::new();
        fence.insert("north".to_string(), "true".to_string());
        fence.insert("east".to_string(), "true".to_string());
        let fence_model = catalog
            .block_render_model("minecraft:oak_fence", &fence)
            .unwrap();
        let BlockModelShape::Boxes(fence_boxes) = fence_model.shape else {
            panic!("oak_fence should combine matching multipart boxes");
        };
        assert_eq!(fence_boxes.len(), 5);
        assert_eq!(fence_boxes[3].from, [0, 12, 7]);
        assert_eq!(fence_boxes[4].from, [0, 6, 7]);

        let flower = catalog
            .block_render_model("minecraft:dandelion", &BTreeMap::new())
            .unwrap();
        assert_eq!(flower.shape, BlockModelShape::Cross);
        assert_eq!(
            flower.face_textures.get(BlockModelFace::North),
            "minecraft:block/dandelion"
        );

        let water = catalog
            .block_render_model("minecraft:water", &BTreeMap::new())
            .unwrap();
        assert_eq!(water.shape, BlockModelShape::Custom);
        assert_eq!(
            water.face_textures.get(BlockModelFace::Up),
            "minecraft:block/water_still"
        );
    }

    fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * width + x) * 4) as usize;
        rgba[offset..offset + 4].try_into().unwrap()
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
