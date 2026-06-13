use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

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

    pub fn atlases_dir(&self) -> PathBuf {
        self.assets_dir.join("atlases")
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
        SpriteImage::from_png_file(
            format!("minecraft:gui/sprites/{name}"),
            self.gui_sprite_texture(name),
        )
    }

    pub fn load_atlas_texture_sources(&self, atlas_name: &str) -> Result<Vec<SpriteSource>> {
        self.load_atlas_texture_entries(atlas_name)?
            .into_iter()
            .map(AtlasTextureEntry::into_source)
            .collect()
    }

    pub fn load_atlas_texture_images(&self, atlas_name: &str) -> Result<Vec<SpriteImage>> {
        self.load_atlas_texture_entries(atlas_name)?
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

    fn load_atlas_texture_entries(&self, atlas_name: &str) -> Result<Vec<AtlasTextureEntry>> {
        let path = self.atlas_definition(atlas_name);
        let bytes =
            std::fs::read(&path).with_context(|| format!("read atlas {}", path.display()))?;
        let atlas: RawAtlas = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse atlas {}", path.display()))?;

        let mut entries = Vec::new();
        for source in atlas.sources {
            match source.source_type.as_str() {
                "minecraft:directory" => {
                    let source_location = source.required_location("source")?;
                    let prefix = source.required_field("prefix")?;
                    self.append_directory_atlas_entries(&mut entries, &source_location, prefix)?;
                }
                "minecraft:single" => {
                    let resource = source.required_location("resource")?;
                    let sprite = source
                        .optional_location("sprite")?
                        .unwrap_or(resource.clone());
                    entries.push(AtlasTextureEntry::File {
                        id: sprite.id(),
                        path: self.texture_path(&resource),
                    });
                }
                "minecraft:paletted_permutations" => {
                    let textures = source.required_locations("textures")?;
                    let palette_key = source.required_location("palette_key")?;
                    let permutations = source.required_permutations()?;
                    let separator = source.separator.as_deref().unwrap_or("_");
                    self.append_paletted_permutation_entries(
                        &mut entries,
                        &textures,
                        &palette_key,
                        &permutations,
                        separator,
                    )?;
                }
                other => bail!("unsupported atlas source type {other:?} in atlas {atlas_name}"),
            }
        }
        Ok(entries)
    }

    fn append_directory_atlas_entries(
        &self,
        entries: &mut Vec<AtlasTextureEntry>,
        source: &ResourceLocation,
        prefix: &str,
    ) -> Result<()> {
        let dir = self.texture_dir(source);
        let mut files = Vec::new();
        collect_png_files(&dir, &mut files)
            .with_context(|| format!("read atlas texture directory {}", dir.display()))?;
        files.sort();

        for path in files {
            let relative = path
                .strip_prefix(&dir)
                .with_context(|| format!("strip texture directory {}", dir.display()))?;
            let relative = texture_id_suffix(relative)?;
            entries.push(AtlasTextureEntry::File {
                id: format!("{}:{}{}", source.namespace, prefix, relative),
                path,
            });
        }
        Ok(())
    }

    fn append_paletted_permutation_entries(
        &self,
        entries: &mut Vec<AtlasTextureEntry>,
        textures: &[ResourceLocation],
        palette_key: &ResourceLocation,
        permutations: &BTreeMap<String, ResourceLocation>,
        separator: &str,
    ) -> Result<()> {
        let key = SpriteImage::from_png_file(palette_key.id(), self.texture_path(palette_key))?;
        let mut palettes = Vec::with_capacity(permutations.len());
        for (suffix, palette) in permutations {
            palettes.push((
                suffix.as_str(),
                SpriteImage::from_png_file(palette.id(), self.texture_path(palette))?,
            ));
        }

        for texture in textures {
            let base = SpriteImage::from_png_file(texture.id(), self.texture_path(texture))?;
            for (suffix, palette) in &palettes {
                let permutation = texture.with_suffix(&format!("{separator}{suffix}"))?;
                entries.push(AtlasTextureEntry::Image(apply_palette_permutation(
                    permutation.id(),
                    &base,
                    &key,
                    palette,
                )?));
            }
        }
        Ok(())
    }

    fn namespace_assets_dir(&self, namespace: &str) -> PathBuf {
        self.sources_dir.join("assets").join(namespace)
    }

    fn texture_dir(&self, location: &ResourceLocation) -> PathBuf {
        self.namespace_assets_dir(&location.namespace)
            .join("textures")
            .join(&location.path)
    }

    fn texture_path(&self, location: &ResourceLocation) -> PathBuf {
        self.namespace_assets_dir(&location.namespace)
            .join("textures")
            .join(format!("{}.png", location.path))
    }
}

#[derive(Debug, Deserialize)]
struct RawAtlas {
    sources: Vec<RawAtlasSource>,
}

#[derive(Debug, Deserialize)]
struct RawAtlasSource {
    #[serde(rename = "type")]
    source_type: String,
    source: Option<String>,
    prefix: Option<String>,
    resource: Option<String>,
    sprite: Option<String>,
    palette_key: Option<String>,
    permutations: Option<BTreeMap<String, String>>,
    textures: Option<Vec<String>>,
    separator: Option<String>,
}

impl RawAtlasSource {
    fn required_field(&self, field: &str) -> Result<&str> {
        match field {
            "prefix" => self
                .prefix
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing atlas source prefix")),
            _ => bail!("unsupported required atlas field {field:?}"),
        }
    }

    fn required_location(&self, field: &str) -> Result<ResourceLocation> {
        self.optional_location(field)?
            .ok_or_else(|| anyhow::anyhow!("missing atlas source {field}"))
    }

    fn optional_location(&self, field: &str) -> Result<Option<ResourceLocation>> {
        let value = match field {
            "source" => self.source.as_deref(),
            "resource" => self.resource.as_deref(),
            "sprite" => self.sprite.as_deref(),
            "palette_key" => self.palette_key.as_deref(),
            _ => bail!("unsupported atlas location field {field:?}"),
        };
        value.map(ResourceLocation::parse).transpose()
    }

    fn required_locations(&self, field: &str) -> Result<Vec<ResourceLocation>> {
        let values = match field {
            "textures" => self.textures.as_deref(),
            _ => bail!("unsupported atlas locations field {field:?}"),
        }
        .ok_or_else(|| anyhow::anyhow!("missing atlas source {field}"))?;
        values
            .iter()
            .map(|value| ResourceLocation::parse(value))
            .collect()
    }

    fn required_permutations(&self) -> Result<BTreeMap<String, ResourceLocation>> {
        let permutations = self
            .permutations
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing atlas source permutations"))?;
        permutations
            .iter()
            .map(|(suffix, location)| Ok((suffix.clone(), ResourceLocation::parse(location)?)))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceLocation {
    namespace: String,
    path: String,
}

impl ResourceLocation {
    fn parse(value: &str) -> Result<Self> {
        let (namespace, path) = value.split_once(':').unwrap_or(("minecraft", value));
        validate_resource_namespace(namespace)?;
        validate_resource_path(path)?;
        Ok(Self {
            namespace: namespace.to_string(),
            path: path.to_string(),
        })
    }

    fn id(&self) -> String {
        format!("{}:{}", self.namespace, self.path)
    }

    fn with_suffix(&self, suffix: &str) -> Result<Self> {
        let path = format!("{}{}", self.path, suffix);
        validate_resource_path(&path)?;
        Ok(Self {
            namespace: self.namespace.clone(),
            path,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AtlasTextureEntry {
    File { id: String, path: PathBuf },
    Image(SpriteImage),
}

impl AtlasTextureEntry {
    fn into_source(self) -> Result<SpriteSource> {
        match self {
            AtlasTextureEntry::File { id, path } => SpriteSource::from_png_file(id, path),
            AtlasTextureEntry::Image(image) => Ok(image.source()),
        }
    }

    fn into_image(self) -> Result<SpriteImage> {
        match self {
            AtlasTextureEntry::File { id, path } => SpriteImage::from_png_file(id, path),
            AtlasTextureEntry::Image(image) => Ok(image),
        }
    }
}

fn collect_png_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read texture directory {}", dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("read texture directory entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("read file type {}", path.display()))?;
        if file_type.is_dir() {
            collect_png_files(&path, files)?;
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) == Some("png") {
            files.push(path);
        }
    }
    Ok(())
}

fn texture_id_suffix(path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let std::path::Component::Normal(part) = component else {
            bail!("invalid texture path component in {}", path.display());
        };
        let part = part
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("non-utf8 texture path {}", path.display()))?;
        parts.push(part.to_string());
    }
    let Some(last) = parts.last_mut() else {
        bail!("empty texture path");
    };
    let Some(stem) = last.strip_suffix(".png") else {
        bail!("texture path {} does not end with .png", path.display());
    };
    *last = stem.to_string();
    Ok(parts.join("/"))
}

fn validate_resource_namespace(namespace: &str) -> Result<()> {
    if namespace.is_empty() {
        bail!("resource namespace must not be empty");
    }
    if !namespace.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
    }) {
        bail!("invalid resource namespace {namespace:?}");
    }
    Ok(())
}

fn validate_resource_path(path: &str) -> Result<()> {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') {
        bail!("invalid resource path {path:?}");
    }
    for segment in path.split('/') {
        if segment.is_empty() || matches!(segment, "." | "..") {
            bail!("invalid resource path segment {segment:?} in {path:?}");
        }
    }
    Ok(())
}

fn apply_palette_permutation(
    id: String,
    base: &SpriteImage,
    key: &SpriteImage,
    values: &SpriteImage,
) -> Result<SpriteImage> {
    let palette = create_palette_mapping(key, values)?;
    let mut rgba = base.rgba.clone();
    for pixel in rgba.chunks_exact_mut(4) {
        let pixel_alpha = pixel[3];
        if pixel_alpha == 0 {
            continue;
        }

        let replacement = palette
            .get(&[pixel[0], pixel[1], pixel[2]])
            .copied()
            .unwrap_or([pixel[0], pixel[1], pixel[2], 255]);
        pixel[0] = replacement[0];
        pixel[1] = replacement[1];
        pixel[2] = replacement[2];
        pixel[3] = ((u16::from(pixel_alpha) * u16::from(replacement[3])) / 255) as u8;
    }
    SpriteImage::new(id, base.width, base.height, rgba)
}

fn create_palette_mapping(
    key: &SpriteImage,
    values: &SpriteImage,
) -> Result<HashMap<[u8; 3], [u8; 4]>> {
    if key.rgba.len() != values.rgba.len() {
        bail!(
            "palette mapping has different sizes: {} and {} pixels",
            key.rgba.len() / 4,
            values.rgba.len() / 4
        );
    }

    let mut palette = HashMap::new();
    for (key, value) in key.rgba.chunks_exact(4).zip(values.rgba.chunks_exact(4)) {
        if key[3] != 0 {
            palette.insert(
                [key[0], key[1], key[2]],
                [value[0], value[1], value[2], value[3]],
            );
        }
    }
    Ok(palette)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{colors::VANILLA_BIOME_ORDER, AtlasPacker, GrassColorModifier, SpriteSource};

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
            vec![SpriteSource::new("minecraft:pattern/base_ruby", 3, 1)]
        );
        let images = roots.load_atlas_texture_images("items").unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].id, "minecraft:pattern/base_ruby");
        assert_eq!(
            images[0].rgba,
            vec![100, 10, 20, 100, 20, 20, 20, 77, 10, 10, 10, 0]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn paletted_permutations_follow_vanilla_palette_size_rules() {
        let base = SpriteImage::new("minecraft:pattern/base", 1, 1, vec![20, 20, 20, 255]).unwrap();
        let key = SpriteImage::new(
            "minecraft:palette/key",
            2,
            1,
            vec![10, 10, 10, 255, 20, 20, 20, 255],
        )
        .unwrap();
        let same_pixel_count_different_shape = SpriteImage::new(
            "minecraft:palette/value",
            1,
            2,
            vec![100, 0, 0, 255, 0, 100, 0, 255],
        )
        .unwrap();

        let image = apply_palette_permutation(
            "minecraft:pattern/base_value".to_string(),
            &base,
            &key,
            &same_pixel_count_different_shape,
        )
        .unwrap();
        assert_eq!(image.rgba, vec![0, 100, 0, 255]);

        let wrong_pixel_count =
            SpriteImage::new("minecraft:palette/wrong", 1, 1, vec![100, 0, 0, 255]).unwrap();
        let err = apply_palette_permutation(
            "minecraft:pattern/base_wrong".to_string(),
            &base,
            &key,
            &wrong_pixel_count,
        )
        .unwrap_err();
        assert!(err.to_string().contains("different sizes"));
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
        assert_eq!(sources.len(), 1_121);
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
        let conduit = sources
            .iter()
            .find(|source| source.id == "minecraft:entity/conduit/base")
            .unwrap();
        assert_eq!((conduit.width, conduit.height), (32, 16));
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
    fn loads_local_vanilla_item_and_armor_trim_atlases() {
        let roots = PackRoots::discover().unwrap();
        let item_sources = roots.load_atlas_texture_sources("items").unwrap();
        assert_eq!(item_sources.len(), 856);
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
        assert_eq!(armor_sources.len(), 576);
        let sentry = armor_sources
            .iter()
            .find(|source| source.id == "minecraft:trims/entity/humanoid/sentry_diamond")
            .unwrap();
        assert_eq!((sentry.width, sentry.height), (64, 32));
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
