use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    atlas_sources::{load_atlas_texture_entries, AtlasTextureEntry},
    block_models::BlockModelCatalog,
    colors::{BiomeColorCatalog, ColorMapImage, TerrainColorMaps},
    item_cuboid_models::ItemCuboidModelCatalog,
    item_models::ItemModelCatalog,
    item_registry::ItemRegistryCatalog,
    language::{LanguageCatalog, DEFAULT_LANGUAGE_CODE},
    metadata::PackMetadataCatalog,
    particle_definitions::ParticleDefinitionCatalog,
    particle_sprites::ParticleSpriteCatalog,
    resources::{PackResourceStack, ResourceLocation},
    sounds::SoundCatalog,
    sprites::{SpriteImage, SpriteSource},
    tags::TagCatalog,
    waypoint_styles::WaypointStyleCatalog,
};

pub const MC_VERSION: &str = "26.1";
pub const DEFAULT_MC_CODE_ROOT: &str = "/Users/zhangguyu/Work/mc-code";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRoots {
    pub mc_code_root: PathBuf,
    pub sources_dir: PathBuf,
    pub assets_dir: PathBuf,
    #[serde(default)]
    pub resource_pack_dirs: Vec<PathBuf>,
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
            resource_pack_dirs: Vec::new(),
        })
    }

    pub fn with_resource_pack_dirs(
        mut self,
        dirs: impl IntoIterator<Item = impl Into<PathBuf>>,
    ) -> Self {
        self.resource_pack_dirs = dirs.into_iter().map(Into::into).collect();
        self
    }

    pub fn resource_stack(&self) -> PackResourceStack {
        let mut roots = Vec::with_capacity(1 + self.resource_pack_dirs.len());
        roots.push(self.sources_dir.clone());
        roots.extend(self.resource_pack_dirs.iter().cloned());
        PackResourceStack::from_roots(roots)
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

    pub fn atlases_dir(&self) -> PathBuf {
        self.assets_dir.join("atlases")
    }

    pub fn sounds_definition(&self) -> PathBuf {
        self.assets_dir.join("sounds.json")
    }

    pub fn sounds_dir(&self) -> PathBuf {
        self.assets_dir.join("sounds")
    }

    pub fn atlas_definition(&self, name: &str) -> PathBuf {
        self.atlases_dir().join(format!("{name}.json"))
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
        self.load_atlas_texture_image("gui", &ResourceLocation::parse(name)?.id())
    }

    pub fn load_atlas_texture_image(
        &self,
        atlas_name: &str,
        sprite_id: &str,
    ) -> Result<SpriteImage> {
        let sprite_id = ResourceLocation::parse(sprite_id)?.id();
        self.load_atlas_texture_images(atlas_name)?
            .into_iter()
            .find(|image| image.id == sprite_id)
            .ok_or_else(|| anyhow::anyhow!("missing sprite {sprite_id} in atlas {atlas_name}"))
    }

    pub fn load_atlas_texture_sources(&self, atlas_name: &str) -> Result<Vec<SpriteSource>> {
        load_atlas_texture_entries(self, atlas_name)?
            .into_iter()
            .map(AtlasTextureEntry::into_source)
            .collect()
    }

    pub fn load_atlas_texture_images(&self, atlas_name: &str) -> Result<Vec<SpriteImage>> {
        load_atlas_texture_entries(self, atlas_name)?
            .into_iter()
            .map(AtlasTextureEntry::into_image)
            .collect()
    }

    pub fn load_block_texture_sources(&self) -> Result<Vec<SpriteSource>> {
        self.load_atlas_texture_sources("blocks")
    }

    pub fn load_block_texture_images(&self) -> Result<Vec<SpriteImage>> {
        self.load_atlas_texture_images("blocks")
    }

    pub fn load_particle_texture_sources(&self) -> Result<Vec<SpriteSource>> {
        self.load_atlas_texture_sources("particles")
    }

    pub fn load_particle_texture_images(&self) -> Result<Vec<SpriteImage>> {
        self.load_atlas_texture_images("particles")
    }

    pub fn load_block_model_catalog(&self) -> Result<BlockModelCatalog> {
        BlockModelCatalog::load(self)
    }

    pub fn load_item_model_catalog(&self) -> Result<ItemModelCatalog> {
        ItemModelCatalog::load(self)
    }

    pub fn load_item_cuboid_model_catalog(&self) -> Result<ItemCuboidModelCatalog> {
        ItemCuboidModelCatalog::load(self)
    }

    pub fn load_item_registry_catalog(&self) -> Result<ItemRegistryCatalog> {
        ItemRegistryCatalog::load(self)
    }

    pub fn load_waypoint_style_catalog(&self) -> Result<WaypointStyleCatalog> {
        WaypointStyleCatalog::load(self)
    }

    pub fn load_particle_definition_catalog(&self) -> Result<ParticleDefinitionCatalog> {
        ParticleDefinitionCatalog::load(self)
    }

    pub fn load_particle_sprite_catalog(&self) -> Result<ParticleSpriteCatalog> {
        ParticleSpriteCatalog::load(self)
    }

    pub fn load_terrain_colormaps(&self) -> Result<TerrainColorMaps> {
        let stack = self.resource_stack();
        Ok(TerrainColorMaps {
            grass: load_terrain_colormap(&stack, "grass")?,
            foliage: load_terrain_colormap(&stack, "foliage")?,
            dry_foliage: Some(load_terrain_colormap(&stack, "dry_foliage")?),
        })
    }

    pub fn load_biome_color_catalog(&self) -> Result<BiomeColorCatalog> {
        BiomeColorCatalog::load_vanilla_26_1(self)
    }

    pub fn load_sound_catalog(&self) -> Result<SoundCatalog> {
        SoundCatalog::load_resource_stack(&self.resource_stack())
    }

    pub fn load_language_catalog(&self, language_code: &str) -> Result<LanguageCatalog> {
        if language_code == DEFAULT_LANGUAGE_CODE {
            self.load_language_stack([DEFAULT_LANGUAGE_CODE])
        } else {
            self.load_language_stack([DEFAULT_LANGUAGE_CODE, language_code])
        }
    }

    pub fn load_client_language_catalog(
        &self,
        selected_language_code: &str,
    ) -> Result<LanguageCatalog> {
        let metadata = self.load_pack_metadata_catalog();
        self.load_language_stack(metadata.language_stack(selected_language_code))
    }

    pub fn load_pack_metadata_catalog(&self) -> PackMetadataCatalog {
        PackMetadataCatalog::load_resource_stack(&self.resource_stack())
    }

    fn load_language_stack(
        &self,
        language_codes: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<LanguageCatalog> {
        let language_codes = language_codes
            .into_iter()
            .map(|code| code.as_ref().to_string())
            .collect::<Vec<_>>();
        let mut catalog =
            LanguageCatalog::load_resource_stack(&self.resource_stack(), &language_codes)?;
        let deprecated_path = self.assets_dir.join("lang").join("deprecated.json");
        if deprecated_path.is_file() {
            // Vanilla loads deprecated translations from the default resource, not overlays.
            if let Ok(bytes) = std::fs::read(&deprecated_path) {
                let _ = catalog.apply_deprecated_json_bytes(&bytes);
            }
        }
        Ok(catalog)
    }

    pub fn load_tag_catalog(&self, registry_path: &str) -> Result<TagCatalog> {
        TagCatalog::load_resource_stack(&self.resource_stack(), registry_path)
    }
}

fn load_terrain_colormap(stack: &PackResourceStack, name: &str) -> Result<ColorMapImage> {
    let location = ResourceLocation::parse(&format!("textures/colormap/{name}.png"))?;
    let resource = stack
        .get_resource(&location)
        .ok_or_else(|| anyhow::anyhow!("missing terrain colormap {}", location.id()))?;
    ColorMapImage::from_png_file(resource.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{
        colors::VANILLA_BIOME_ORDER, AtlasPacker, GrassColorModifier, SpriteGuiScaling,
        SpriteMipmapStrategy, SpriteNineSliceBorder, SpriteSource,
    };

    #[test]
    fn pack_roots_loads_block_texture_sources_from_vanilla_atlas() {
        let root = unique_temp_dir("pack-roots");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        let block_dir = assets_dir.join("textures").join("block");
        write_test_png(&block_dir.join("z_stone.png"), 16, 16);
        write_test_png(&block_dir.join("a_hd_overlay.png"), 64, 32);
        write_test_png(&block_dir.join("sub").join("deepslate.png"), 8, 8);
        std::fs::write(block_dir.join("a_hd_overlay.png.mcmeta"), "{}").unwrap();
        write_test_png(
            &assets_dir
                .join("textures")
                .join("entity")
                .join("conduit")
                .join("base.png"),
            32,
            16,
        );
        write_test_png(
            &assets_dir
                .join("textures")
                .join("entity")
                .join("bell")
                .join("bell_body.png"),
            32,
            32,
        );
        write_test_png(
            &assets_dir
                .join("textures")
                .join("entity")
                .join("enchantment")
                .join("enchanting_table_book.png"),
            48,
            16,
        );
        write_json(
            &assets_dir.join("atlases").join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "block/",
                  "source": "block"
                },
                {
                  "type": "minecraft:directory",
                  "prefix": "entity/conduit/",
                  "source": "entity/conduit"
                },
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:entity/bell/bell_body"
                },
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:entity/enchantment/enchanting_table_book",
                  "sprite": "minecraft:custom/book"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert_eq!(
            sources,
            vec![
                SpriteSource::new("minecraft:missingno", 16, 16),
                SpriteSource::new("minecraft:block/a_hd_overlay", 64, 32),
                SpriteSource::new("minecraft:block/sub/deepslate", 8, 8),
                SpriteSource::new("minecraft:block/z_stone", 16, 16),
                SpriteSource::new("minecraft:entity/conduit/base", 32, 16),
                SpriteSource::new("minecraft:entity/bell/bell_body", 32, 32),
                SpriteSource::new("minecraft:custom/book", 48, 16),
            ]
        );
        let images = roots.load_block_texture_images().unwrap();
        assert_eq!(images.len(), sources.len());
        assert_eq!(images.last().unwrap().id, "minecraft:custom/book");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_directory_atlas_sources_scan_all_namespaces() {
        let root = unique_temp_dir("pack-roots-directory-namespaces");
        let sources_dir = root.join("sources").join(MC_VERSION);
        let assets_root = sources_dir.join("assets");
        let minecraft_assets = assets_root.join("minecraft");
        let example_assets = assets_root.join("example");
        write_test_png(
            &example_assets
                .join("textures")
                .join("block")
                .join("gem.png"),
            16,
            16,
        );
        write_test_png(
            &example_assets
                .join("textures")
                .join("block")
                .join("sub")
                .join("ore.png"),
            8,
            8,
        );
        write_test_png(
            &minecraft_assets
                .join("textures")
                .join("block")
                .join("stone.png"),
            16,
            16,
        );
        write_json(
            &minecraft_assets.join("atlases").join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "block/",
                  "source": "block"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert_eq!(
            sources,
            vec![
                SpriteSource::new("minecraft:missingno", 16, 16),
                SpriteSource::new("example:block/gem", 16, 16),
                SpriteSource::new("example:block/sub/ore", 8, 8),
                SpriteSource::new("minecraft:block/stone", 16, 16),
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_gui_sprite_images_from_vanilla_atlas() {
        let root = unique_temp_dir("gui-atlas");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_test_png(
            &assets_dir
                .join("textures")
                .join("gui")
                .join("sprites")
                .join("hud")
                .join("crosshair.png"),
            15,
            15,
        );
        write_test_png(
            &assets_dir
                .join("textures")
                .join("mob_effect")
                .join("speed.png"),
            18,
            18,
        );
        write_json(
            &assets_dir.join("atlases").join("gui.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "",
                  "source": "gui/sprites"
                },
                {
                  "type": "minecraft:directory",
                  "prefix": "mob_effect/",
                  "source": "mob_effect"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let images = roots.load_atlas_texture_images("gui").unwrap();
        let ids = images
            .iter()
            .map(|image| image.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "minecraft:missingno",
                "minecraft:hud/crosshair",
                "minecraft:mob_effect/speed"
            ]
        );
        let crosshair = roots.load_gui_sprite_image("hud/crosshair").unwrap();
        assert_eq!(crosshair.id, "minecraft:hud/crosshair");
        assert_eq!((crosshair.width, crosshair.height), (15, 15));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_paletted_permutation_atlas_images() {
        let root = unique_temp_dir("paletted-permutations");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_test_rgba_png(
            &assets_dir.join("textures").join("palette").join("key.png"),
            2,
            1,
            &[10, 10, 10, 255, 20, 20, 20, 0],
        );
        write_test_rgba_png(
            &assets_dir.join("textures").join("palette").join("ruby.png"),
            2,
            1,
            &[100, 10, 20, 128, 200, 200, 200, 255],
        );
        write_test_rgba_png(
            &assets_dir.join("textures").join("pattern").join("base.png"),
            3,
            1,
            &[10, 10, 10, 200, 20, 20, 20, 77, 10, 10, 10, 0],
        );
        write_json(
            &assets_dir.join("atlases").join("items.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:paletted_permutations",
                  "palette_key": "minecraft:palette/key",
                  "permutations": {
                    "ruby": "minecraft:palette/ruby"
                  },
                  "textures": [
                    "minecraft:pattern/base"
                  ]
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let sources = roots.load_atlas_texture_sources("items").unwrap();
        assert_eq!(
            sources,
            vec![
                SpriteSource::new("minecraft:missingno", 16, 16),
                SpriteSource::new("minecraft:pattern/base_ruby", 3, 1)
            ]
        );
        let images = roots.load_atlas_texture_images("items").unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].id, "minecraft:missingno");
        assert_eq!(images[1].id, "minecraft:pattern/base_ruby");
        assert_eq!(
            images[1].rgba,
            vec![100, 10, 20, 100, 20, 20, 20, 77, 10, 10, 10, 0]
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
    fn pack_roots_terrain_colormaps_use_resource_pack_precedence() {
        let root = unique_temp_dir("terrain-colormap-pack-precedence");
        let base_colormap_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("colormap");
        let pack = root.join("pack");
        let pack_colormap_dir = pack
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("colormap");
        write_test_rgba_png(&base_colormap_dir.join("grass.png"), 1, 1, &[1, 2, 3, 255]);
        write_test_rgba_png(
            &base_colormap_dir.join("foliage.png"),
            1,
            1,
            &[40, 50, 60, 255],
        );
        write_test_rgba_png(
            &base_colormap_dir.join("dry_foliage.png"),
            1,
            1,
            &[70, 80, 90, 255],
        );
        write_test_rgba_png(
            &pack_colormap_dir.join("grass.png"),
            1,
            1,
            &[10, 20, 30, 255],
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack]);
        let colormaps = roots.load_terrain_colormaps().unwrap();

        assert_eq!(colormaps.grass.rgb_at(0, 0), [10, 20, 30]);
        assert_eq!(colormaps.foliage.rgb_at(0, 0), [40, 50, 60]);
        assert_eq!(
            colormaps
                .dry_foliage
                .as_ref()
                .map(|colormap| colormap.rgb_at(0, 0)),
            Some([70, 80, 90])
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
    fn pack_roots_biome_color_catalog_uses_data_pack_precedence() {
        let root = unique_temp_dir("biome-color-pack-precedence");
        let base_biome_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome");
        let pack = root.join("pack");
        let pack_biome_dir = pack
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome");
        write_json(
            &base_biome_dir.join("plains.json"),
            r##"{
              "temperature": 0.8,
              "downfall": 0.4,
              "effects": {
                "water_color": "#123456"
              }
            }"##,
        );
        write_json(
            &base_biome_dir.join("swamp.json"),
            r##"{
              "temperature": 0.8,
              "downfall": 0.9,
              "effects": {
                "water_color": "#617b64"
              }
            }"##,
        );
        write_json(
            &pack_biome_dir.join("plains.json"),
            r##"{
              "temperature": 0.7,
              "downfall": 0.3,
              "effects": {
                "water_color": "#abcdef",
                "grass_color": "#010203"
              }
            }"##,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack]);
        let catalog = roots.load_biome_color_catalog().unwrap();
        let plains = catalog.profile(1).unwrap();
        let swamp = catalog.profile(6).unwrap();

        assert_eq!(plains.temperature, 0.7);
        assert_eq!(plains.downfall, 0.3);
        assert_eq!(plains.water_color, Some([0xab, 0xcd, 0xef]));
        assert_eq!(plains.grass_color, Some([1, 2, 3]));
        assert_eq!(swamp.water_color, Some([0x61, 0x7b, 0x64]));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_all_local_vanilla_atlases() {
        let roots = PackRoots::discover().unwrap();
        let expected = [
            ("armor_trims", 577),
            ("banner_patterns", 45),
            ("beds", 17),
            ("blocks", 1_122),
            ("celestials", 11),
            ("chests", 23),
            ("decorated_pot", 26),
            ("gui", 487),
            ("items", 857),
            ("map_decorations", 36),
            ("paintings", 53),
            ("particles", 255),
            ("shield_patterns", 46),
            ("shulker_boxes", 19),
            ("signs", 25),
        ];

        for (atlas_name, expected_count) in expected {
            let sources = roots.load_atlas_texture_sources(atlas_name).unwrap();
            assert_eq!(
                sources.len(),
                expected_count,
                "unexpected sprite count for {atlas_name} atlas"
            );
            assert!(
                sources
                    .iter()
                    .all(|source| source.width > 0 && source.height > 0),
                "atlas {atlas_name} contains an empty sprite"
            );
        }
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_block_texture_dimensions() {
        let roots = PackRoots::discover().unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert_eq!(sources.len(), 1_122);
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
        assert_eq!(
            stone.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::Auto
        );

        let torchflower = sources
            .iter()
            .find(|source| source.id == "minecraft:block/torchflower")
            .unwrap();
        assert_eq!(
            torchflower.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::StrictCutout
        );
        let glass = sources
            .iter()
            .find(|source| source.id == "minecraft:block/glass")
            .unwrap();
        assert_eq!(
            glass.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::Mean
        );
        let oak_leaves = sources
            .iter()
            .find(|source| source.id == "minecraft:block/oak_leaves")
            .unwrap();
        assert_eq!(
            oak_leaves.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::DarkCutout
        );

        let water = sources
            .iter()
            .find(|source| source.id == "minecraft:block/water_still")
            .unwrap();
        assert_eq!((water.width, water.height), (16, 16));
        let water_animation = water.animation.as_ref().unwrap();
        assert_eq!(water_animation.frame_count, 32);
        assert_eq!(water_animation.default_frame_time, 2);
        assert_eq!(water_animation.frames.len(), 32);
        let water_image = roots
            .load_atlas_texture_image("blocks", "minecraft:block/water_still")
            .unwrap();
        assert_eq!(water_image.animation_frames_rgba.len(), 32);
        assert_eq!(water_image.frame_rgba(31).unwrap().len(), 16 * 16 * 4);
        let water_flow = sources
            .iter()
            .find(|source| source.id == "minecraft:block/water_flow")
            .unwrap();
        assert_eq!((water_flow.width, water_flow.height), (32, 32));
        let lava = sources
            .iter()
            .find(|source| source.id == "minecraft:block/lava_still")
            .unwrap();
        assert_eq!((lava.width, lava.height), (16, 16));
        let lava_animation = lava.animation.as_ref().unwrap();
        assert_eq!(lava_animation.frame_count, 20);
        assert_eq!(lava_animation.default_frame_time, 2);
        assert_eq!(lava_animation.frames.len(), 38);
        assert_eq!(lava_animation.frames[20].index, 18);
        let fire = sources
            .iter()
            .find(|source| source.id == "minecraft:block/fire_0")
            .unwrap();
        assert_eq!((fire.width, fire.height), (16, 16));
        let conduit = sources
            .iter()
            .find(|source| source.id == "minecraft:entity/conduit/base")
            .unwrap();
        assert_eq!((conduit.width, conduit.height), (32, 16));
        let conduit_wind = sources
            .iter()
            .find(|source| source.id == "minecraft:entity/conduit/wind")
            .unwrap();
        assert_eq!((conduit_wind.width, conduit_wind.height), (64, 32));
        let bell = sources
            .iter()
            .find(|source| source.id == "minecraft:entity/bell/bell_body")
            .unwrap();
        assert_eq!((bell.width, bell.height), (32, 32));

        let layout = AtlasPacker::new(4096, 1)
            .unwrap()
            .pack(&sources[..64])
            .unwrap();
        assert!(layout.width <= 4096);
        assert_eq!(layout.sprites.len(), 64);
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn stitches_local_vanilla_block_texture_mip_atlas() {
        let roots = PackRoots::discover().unwrap();
        let images = roots.load_block_texture_images().unwrap();
        assert_eq!(images.len(), 1_121);

        let packer = AtlasPacker::new(4096, 1).unwrap();
        let atlas = packer.stitch_mips_with_max_level(&images, 4).unwrap();
        assert_eq!(atlas.levels.len(), atlas.mip_level() as usize + 1);
        assert!(atlas.mip_level() > 0);
        assert!(atlas.mip_level() <= 4);
        for (level, mip) in atlas.levels.iter().enumerate() {
            assert_eq!(mip.level, level as u32);
            assert_eq!(mip.width, atlas.layout.width >> level);
            assert_eq!(mip.height, atlas.layout.height >> level);
            assert_eq!(mip.rgba.len(), (mip.width * mip.height * 4) as usize);
        }

        let animation_atlas = packer
            .stitch_animation_frame_mips_with_max_level(&images, 2, 4)
            .unwrap();
        assert_eq!(animation_atlas.mip_level(), atlas.mip_level());
        assert_eq!(animation_atlas.levels.len(), atlas.levels.len());
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_item_and_armor_trim_atlases() {
        let roots = PackRoots::discover().unwrap();
        let item_sources = roots.load_atlas_texture_sources("items").unwrap();
        assert_eq!(item_sources.len(), 857);
        let helmet_trim = item_sources
            .iter()
            .find(|source| source.id == "minecraft:trims/items/helmet_trim_diamond")
            .unwrap();
        assert_eq!((helmet_trim.width, helmet_trim.height), (16, 16));
        let apple = item_sources
            .iter()
            .find(|source| source.id == "minecraft:item/apple")
            .unwrap();
        assert_eq!((apple.width, apple.height), (16, 16));

        let armor_sources = roots.load_atlas_texture_sources("armor_trims").unwrap();
        assert_eq!(armor_sources.len(), 577);
        let sentry = armor_sources
            .iter()
            .find(|source| source.id == "minecraft:trims/entity/humanoid/sentry_diamond")
            .unwrap();
        assert_eq!((sentry.width, sentry.height), (64, 32));
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_gui_atlas() {
        let roots = PackRoots::discover().unwrap();
        let gui_sources = roots.load_atlas_texture_sources("gui").unwrap();
        assert_eq!(gui_sources.len(), 487);
        let crosshair = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/crosshair")
            .unwrap();
        assert_eq!((crosshair.width, crosshair.height), (15, 15));
        let hotbar = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/hotbar")
            .unwrap();
        assert_eq!((hotbar.width, hotbar.height), (182, 22));
        let speed = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:mob_effect/speed")
            .unwrap();
        assert_eq!((speed.width, speed.height), (18, 18));
        let locator_arrow = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/locator_bar_arrow_down")
            .unwrap();
        assert_eq!((locator_arrow.width, locator_arrow.height), (7, 5));
        let locator_animation = locator_arrow.animation.as_ref().unwrap();
        assert_eq!(locator_animation.frame_count, 2);
        assert_eq!(locator_animation.frames[0].time, 10);
        assert_eq!(locator_animation.frames[1].time, 4);

        let crosshair_image = roots.load_gui_sprite_image("hud/crosshair").unwrap();
        assert_eq!(crosshair_image.id, "minecraft:hud/crosshair");
        assert_eq!((crosshair_image.width, crosshair_image.height), (15, 15));

        let button = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:widget/button")
            .unwrap();
        assert_eq!(
            button.gui_metadata.scaling,
            SpriteGuiScaling::NineSlice {
                width: 200,
                height: 20,
                border: SpriteNineSliceBorder::uniform(3),
                stretch_inner: false,
            }
        );
        let locator_background = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/locator_bar_background")
            .unwrap();
        assert_eq!(
            locator_background.gui_metadata.scaling,
            SpriteGuiScaling::NineSlice {
                width: 12,
                height: 5,
                border: SpriteNineSliceBorder {
                    left: 5,
                    right: 5,
                    top: 1,
                    bottom: 1,
                },
                stretch_inner: false,
            }
        );
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        write_test_rgba_png(
            path,
            width,
            height,
            &[1, 2, 3, 255].repeat((width * height) as usize),
        );
    }

    fn write_test_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec()).unwrap();
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
