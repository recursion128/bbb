use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use image::ImageReader;
use regex::Regex;
use serde::Deserialize;

use crate::{
    resources::{validate_resource_path, PackResource, PackResourceStack, ResourceLocation},
    roots::PackRoots,
    sprites::{SpriteImage, SpriteSource},
};

pub(crate) fn load_atlas_texture_entries(
    roots: &PackRoots,
    atlas_name: &str,
) -> Result<Vec<AtlasTextureEntry>> {
    let atlas_location = atlas_definition_location(atlas_name)?;
    let stack = roots.resource_stack();
    let atlas_resources = stack.get_resource_stack(&atlas_location);
    if atlas_resources.is_empty() {
        bail!("missing atlas {atlas_name}");
    }
    let loader = AtlasTextureLoader::new(roots);
    let mut entries = Vec::new();

    for resource in atlas_resources {
        let bytes = match std::fs::read(&resource.path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let atlas: RawAtlas = match serde_json::from_slice(&bytes) {
            Ok(atlas) => atlas,
            Err(_) => continue,
        };

        for source in atlas.sources {
            match source.source_type.as_str() {
                "minecraft:directory" => {
                    let source_path = source.required_path("source")?;
                    let prefix = source.required_field("prefix")?;
                    loader.append_directory_atlas_entries(&mut entries, source_path, prefix)?;
                }
                "minecraft:single" => {
                    let resource = source.required_location("resource")?;
                    let sprite = source
                        .optional_location("sprite")?
                        .unwrap_or(resource.clone());
                    loader.append_single_entry(&mut entries, &resource, &sprite)?;
                }
                "minecraft:paletted_permutations" => {
                    let textures = source.required_locations("textures")?;
                    let palette_key = source.required_location("palette_key")?;
                    let permutations = source.required_permutations()?;
                    let separator = source.separator.as_deref().unwrap_or("_");
                    loader.append_paletted_permutation_entries(
                        &mut entries,
                        &textures,
                        &palette_key,
                        &permutations,
                        separator,
                    )?;
                }
                "minecraft:filter" => {
                    let pattern = source.required_pattern()?;
                    remove_matching_entries(&mut entries, &pattern)?;
                }
                "minecraft:unstitch" => {
                    let resource = source.required_location("resource")?;
                    let regions = source.required_unstitch_regions()?;
                    let divisor_x = source.divisor_x.unwrap_or(1.0);
                    let divisor_y = source.divisor_y.unwrap_or(1.0);
                    loader.append_unstitch_entries(
                        &mut entries,
                        &resource,
                        &regions,
                        divisor_x,
                        divisor_y,
                    )?;
                }
                other => bail!("unsupported atlas source type {other:?} in atlas {atlas_name}"),
            }
        }
    }
    Ok(entries)
}

fn atlas_definition_location(atlas_name: &str) -> Result<ResourceLocation> {
    let atlas_id = ResourceLocation::parse(atlas_name)?;
    ResourceLocation::new(
        atlas_id.namespace().to_string(),
        format!("atlases/{}.json", atlas_id.path()),
    )
}

struct AtlasTextureLoader {
    stack: PackResourceStack,
}

impl AtlasTextureLoader {
    fn new(roots: &PackRoots) -> Self {
        Self {
            stack: roots.resource_stack(),
        }
    }

    fn append_directory_atlas_entries(
        &self,
        entries: &mut Vec<AtlasTextureEntry>,
        source_path: &str,
        prefix: &str,
    ) -> Result<()> {
        let texture_prefix = resource_texture_prefix(source_path);
        for resource in self.stack.list_resources(&texture_prefix, ".png")? {
            let id = directory_sprite_id(&resource.location, source_path, prefix)?;
            append_or_replace_entry(
                entries,
                AtlasTextureEntry::File {
                    id,
                    path: resource.path,
                    metadata_path: resource.metadata_path,
                },
            );
        }
        Ok(())
    }

    fn append_single_entry(
        &self,
        entries: &mut Vec<AtlasTextureEntry>,
        resource: &ResourceLocation,
        sprite: &ResourceLocation,
    ) -> Result<()> {
        if let Some(resource) = self.texture_resource(resource)? {
            append_or_replace_entry(
                entries,
                AtlasTextureEntry::File {
                    id: sprite.id(),
                    path: resource.path,
                    metadata_path: resource.metadata_path,
                },
            );
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
        let key = self.required_raw_texture_image(palette_key)?;
        let mut palettes = Vec::with_capacity(permutations.len());
        for (suffix, palette) in permutations {
            palettes.push((suffix.as_str(), self.required_raw_texture_image(palette)?));
        }

        for texture in textures {
            let base = self.required_raw_texture_image(texture)?;
            for (suffix, palette) in &palettes {
                let permutation = texture.with_suffix(&format!("{separator}{suffix}"))?;
                append_or_replace_entry(
                    entries,
                    AtlasTextureEntry::Image(apply_palette_permutation(
                        permutation.id(),
                        &base,
                        &key,
                        palette,
                    )?),
                );
            }
        }
        Ok(())
    }

    fn append_unstitch_entries(
        &self,
        entries: &mut Vec<AtlasTextureEntry>,
        resource: &ResourceLocation,
        regions: &[UnstitchRegion],
        divisor_x: f64,
        divisor_y: f64,
    ) -> Result<()> {
        let (source_width, source_height, rgba) =
            read_raw_png(self.required_texture_path(resource)?)?;
        for region in regions {
            append_or_replace_entry(
                entries,
                AtlasTextureEntry::Image(unstitch_region(
                    source_width,
                    source_height,
                    &rgba,
                    region,
                    divisor_x,
                    divisor_y,
                )?),
            );
        }
        Ok(())
    }

    fn required_texture_path(&self, location: &ResourceLocation) -> Result<PathBuf> {
        Ok(self.required_texture_resource(location)?.path)
    }

    fn required_texture_resource(&self, location: &ResourceLocation) -> Result<PackResource> {
        self.texture_resource(location)?
            .ok_or_else(|| anyhow::anyhow!("missing atlas texture {}", location.id()))
    }

    fn texture_resource(&self, location: &ResourceLocation) -> Result<Option<PackResource>> {
        let resource = texture_resource_location(location)?;
        Ok(self.stack.get_resource(&resource))
    }

    fn required_raw_texture_image(&self, location: &ResourceLocation) -> Result<SpriteImage> {
        let resource = self.required_texture_resource(location)?;
        let (width, height, rgba) = read_raw_png(resource.path)?;
        SpriteImage::new(location.id(), width, height, rgba)
    }
}

fn texture_resource_location(location: &ResourceLocation) -> Result<ResourceLocation> {
    ResourceLocation::new(
        location.namespace().to_string(),
        format!("textures/{}.png", location.path()),
    )
}

fn resource_texture_prefix(source_path: &str) -> String {
    if source_path.is_empty() {
        "textures".to_string()
    } else {
        format!("textures/{source_path}")
    }
}

fn directory_sprite_id(
    resource: &ResourceLocation,
    source_path: &str,
    prefix: &str,
) -> Result<String> {
    let texture_prefix = resource_texture_prefix(source_path);
    let texture_prefix = format!("{texture_prefix}/");
    let suffix = resource
        .path()
        .strip_prefix(&texture_prefix)
        .ok_or_else(|| anyhow::anyhow!("texture {} is outside atlas source", resource.id()))?;
    let suffix = suffix
        .strip_suffix(".png")
        .ok_or_else(|| anyhow::anyhow!("texture {} is not a png", resource.id()))?;
    ResourceLocation::new(
        resource.namespace().to_string(),
        format!("{prefix}{suffix}"),
    )
    .map(|location| location.id())
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
    pattern: Option<RawIdentifierPattern>,
    permutations: Option<BTreeMap<String, String>>,
    regions: Option<Vec<RawUnstitchRegion>>,
    textures: Option<Vec<String>>,
    separator: Option<String>,
    divisor_x: Option<f64>,
    divisor_y: Option<f64>,
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

    fn required_path(&self, field: &str) -> Result<&str> {
        let value = match field {
            "source" => self.source.as_deref(),
            _ => bail!("unsupported required atlas path field {field:?}"),
        }
        .ok_or_else(|| anyhow::anyhow!("missing atlas source {field}"))?;
        validate_resource_path(value)?;
        Ok(value)
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

    fn required_pattern(&self) -> Result<IdentifierPattern> {
        self.pattern
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing atlas source pattern"))?
            .compile()
    }

    fn required_unstitch_regions(&self) -> Result<Vec<UnstitchRegion>> {
        let regions = self
            .regions
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing atlas source regions"))?;
        if regions.is_empty() {
            bail!("atlas source regions must not be empty");
        }
        regions.iter().map(RawUnstitchRegion::region).collect()
    }
}

#[derive(Debug, Deserialize)]
struct RawIdentifierPattern {
    namespace: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawUnstitchRegion {
    sprite: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone)]
struct UnstitchRegion {
    sprite: ResourceLocation,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl RawUnstitchRegion {
    fn region(&self) -> Result<UnstitchRegion> {
        Ok(UnstitchRegion {
            sprite: ResourceLocation::parse(&self.sprite)?,
            x: finite_number(self.x, "unstitch x")?,
            y: finite_number(self.y, "unstitch y")?,
            width: positive_finite_number(self.width, "unstitch width")?,
            height: positive_finite_number(self.height, "unstitch height")?,
        })
    }
}

#[derive(Debug)]
struct IdentifierPattern {
    namespace: Option<Regex>,
    path: Option<Regex>,
}

impl RawIdentifierPattern {
    fn compile(&self) -> Result<IdentifierPattern> {
        Ok(IdentifierPattern {
            namespace: compile_optional_regex(self.namespace.as_deref(), "namespace")?,
            path: compile_optional_regex(self.path.as_deref(), "path")?,
        })
    }
}

impl IdentifierPattern {
    fn matches(&self, id: &ResourceLocation) -> bool {
        self.namespace
            .as_ref()
            .map_or(true, |pattern| pattern.is_match(id.namespace()))
            && self
                .path
                .as_ref()
                .map_or(true, |pattern| pattern.is_match(id.path()))
    }
}

fn compile_optional_regex(pattern: Option<&str>, field: &str) -> Result<Option<Regex>> {
    pattern
        .map(|pattern| {
            Regex::new(pattern)
                .with_context(|| format!("compile atlas filter {field} regex {pattern:?}"))
        })
        .transpose()
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AtlasTextureEntry {
    File {
        id: String,
        path: PathBuf,
        metadata_path: Option<PathBuf>,
    },
    Image(SpriteImage),
}

impl AtlasTextureEntry {
    fn id(&self) -> &str {
        match self {
            AtlasTextureEntry::File { id, .. } => id,
            AtlasTextureEntry::Image(image) => &image.id,
        }
    }

    pub(crate) fn into_source(self) -> Result<SpriteSource> {
        match self {
            AtlasTextureEntry::File {
                id,
                path,
                metadata_path,
            } => SpriteSource::from_png_file_with_metadata_path(id, path, metadata_path.as_deref()),
            AtlasTextureEntry::Image(image) => Ok(image.source()),
        }
    }

    pub(crate) fn into_image(self) -> Result<SpriteImage> {
        match self {
            AtlasTextureEntry::File {
                id,
                path,
                metadata_path,
            } => SpriteImage::from_png_file_with_metadata_path(id, path, metadata_path.as_deref()),
            AtlasTextureEntry::Image(image) => Ok(image),
        }
    }
}

fn append_or_replace_entry(entries: &mut Vec<AtlasTextureEntry>, entry: AtlasTextureEntry) {
    let id = entry.id().to_string();
    if let Some(existing) = entries.iter_mut().find(|existing| existing.id() == id) {
        *existing = entry;
    } else {
        entries.push(entry);
    }
}

fn remove_matching_entries(
    entries: &mut Vec<AtlasTextureEntry>,
    pattern: &IdentifierPattern,
) -> Result<()> {
    let mut retained = Vec::with_capacity(entries.len());
    for entry in entries.drain(..) {
        let id = ResourceLocation::parse(entry.id())?;
        if !pattern.matches(&id) {
            retained.push(entry);
        }
    }
    *entries = retained;
    Ok(())
}

fn read_raw_png(path: impl AsRef<Path>) -> Result<(u32, u32, Vec<u8>)> {
    let path = path.as_ref();
    let reader = ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
    let reader = reader
        .with_guessed_format()
        .with_context(|| format!("guess image format {}", path.display()))?;
    if reader.format() != Some(image::ImageFormat::Png) {
        bail!("unstitch source {} is not a PNG", path.display());
    }
    let rgba = reader
        .decode()
        .with_context(|| format!("decode png {}", path.display()))?
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((width, height, rgba.into_raw()))
}

fn unstitch_region(
    source_width: u32,
    source_height: u32,
    source_rgba: &[u8],
    region: &UnstitchRegion,
    divisor_x: f64,
    divisor_y: f64,
) -> Result<SpriteImage> {
    let divisor_x = positive_finite_number(divisor_x, "unstitch divisor_x")?;
    let divisor_y = positive_finite_number(divisor_y, "unstitch divisor_y")?;
    let x_scale = f64::from(source_width) / divisor_x;
    let y_scale = f64::from(source_height) / divisor_y;
    let x = scaled_floor(region.x, x_scale, "unstitch x")?;
    let y = scaled_floor(region.y, y_scale, "unstitch y")?;
    let width = scaled_floor(region.width, x_scale, "unstitch width")?;
    let height = scaled_floor(region.height, y_scale, "unstitch height")?;
    if width == 0 || height == 0 {
        bail!(
            "unstitch region {} has zero-sized dimensions",
            region.sprite.id()
        );
    }
    let right = x
        .checked_add(width)
        .ok_or_else(|| anyhow::anyhow!("unstitch region width overflow"))?;
    let bottom = y
        .checked_add(height)
        .ok_or_else(|| anyhow::anyhow!("unstitch region height overflow"))?;
    if right > source_width || bottom > source_height {
        bail!(
            "unstitch region {} exceeds source bounds {}x{}",
            region.sprite.id(),
            source_width,
            source_height
        );
    }

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let source_stride = source_width as usize * 4;
    let row_len = width as usize * 4;
    for local_y in 0..height {
        let start = (y as usize + local_y as usize)
            .checked_mul(source_stride)
            .and_then(|row| row.checked_add(x as usize * 4))
            .ok_or_else(|| anyhow::anyhow!("unstitch region offset overflow"))?;
        rgba.extend_from_slice(&source_rgba[start..start + row_len]);
    }
    SpriteImage::new(region.sprite.id(), width, height, rgba)
}

fn finite_number(value: f64, label: &str) -> Result<f64> {
    if !value.is_finite() {
        bail!("{label} must be finite");
    }
    Ok(value)
}

fn positive_finite_number(value: f64, label: &str) -> Result<f64> {
    let value = finite_number(value, label)?;
    if value <= 0.0 {
        bail!("{label} must be positive");
    }
    Ok(value)
}

fn scaled_floor(value: f64, scale: f64, label: &str) -> Result<u32> {
    let scaled = finite_number(value * scale, label)?.floor();
    if scaled < 0.0 || scaled > f64::from(u32::MAX) {
        bail!("{label} is outside supported bounds");
    }
    Ok(scaled as u32)
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
    use super::{apply_palette_permutation, load_atlas_texture_entries, AtlasTextureEntry};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{PackRoots, MC_VERSION};
    use crate::{SpriteImage, SpriteMipmapStrategy, SpriteTextureMetadata};

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
        assert_eq!(image.texture_metadata, SpriteTextureMetadata::default());

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
    fn atlas_filter_removes_previous_entries_matching_identifier_pattern() {
        let root = unique_temp_dir("atlas-filter");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        std::fs::create_dir_all(assets_dir.join("textures").join("block")).unwrap();
        std::fs::create_dir_all(assets_dir.join("textures").join("entity")).unwrap();
        std::fs::create_dir_all(assets_dir.join("atlases")).unwrap();
        std::fs::write(
            assets_dir.join("textures").join("block").join("stone.png"),
            [],
        )
        .unwrap();
        std::fs::write(
            assets_dir
                .join("textures")
                .join("block")
                .join("filtered_stone.png"),
            [],
        )
        .unwrap();
        std::fs::write(
            assets_dir.join("textures").join("entity").join("bell.png"),
            [],
        )
        .unwrap();
        std::fs::write(
            assets_dir.join("atlases").join("filtered.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "source": "block",
                  "prefix": "block/"
                },
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:entity/bell"
                },
                {
                  "type": "minecraft:filter",
                  "pattern": {
                    "namespace": "minecraft",
                    "path": "filtered|entity/"
                  }
                }
              ]
            }"#,
        )
        .unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let entries = load_atlas_texture_entries(&roots, "filtered").unwrap();
        let ids = entries.iter().map(entry_id).collect::<Vec<_>>();

        assert_eq!(ids, vec!["minecraft:block/stone"]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_definition_stack_appends_sources_in_pack_order() {
        let root = unique_temp_dir("atlas-stack-order");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        write_file(
            &base
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
        );
        write_file(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("deepslate.png"),
        );
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{"sources":[{"type":"minecraft:single","resource":"minecraft:block/stone"}]}"#,
        );
        write_json(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{"sources":[{"type":"minecraft:single","resource":"minecraft:block/deepslate"}]}"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let entries = load_atlas_texture_entries(&roots, "blocks").unwrap();
        let ids = entries.iter().map(entry_id).collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec!["minecraft:block/stone", "minecraft:block/deepslate"]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_definition_stack_skips_invalid_definition_and_keeps_valid_sources() {
        let root = unique_temp_dir("atlas-stack-skip-invalid");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        let base_assets = base.join("assets").join("minecraft");
        let overlay_atlases = overlay.join("assets").join("minecraft").join("atlases");
        std::fs::create_dir_all(base_assets.join("textures").join("block")).unwrap();
        std::fs::create_dir_all(base_assets.join("textures").join("particle")).unwrap();
        write_rgba_png(
            &base_assets.join("textures").join("block").join("stone.png"),
            1,
            1,
            &[10, 20, 30, 255],
        );
        write_rgba_png(
            &base_assets
                .join("textures")
                .join("particle")
                .join("spark.png"),
            1,
            1,
            &[40, 50, 60, 255],
        );
        write_json(
            &base_assets.join("atlases").join("blocks.json"),
            r#"{"sources":[{"type":"minecraft:single","resource":"minecraft:block/stone"}]}"#,
        );
        write_json(
            &base_assets.join("atlases").join("particles.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "source": "particle",
                  "prefix": "particle/"
                }
              ]
            }"#,
        );
        write_json(&overlay_atlases.join("blocks.json"), r#"{"sources":["#);
        write_json(
            &overlay_atlases.join("particles.json"),
            r#"{"sources":{"type":"minecraft:single"}}"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);

        let block_entries = load_atlas_texture_entries(&roots, "blocks").unwrap();
        assert_eq!(
            block_entries.iter().map(entry_id).collect::<Vec<_>>(),
            vec!["minecraft:block/stone"]
        );

        let particle_sources = roots.load_atlas_texture_sources("particles").unwrap();
        assert_eq!(particle_sources.len(), 1);
        assert_eq!(particle_sources[0].id, "minecraft:particle/spark");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_duplicate_sprite_id_keeps_later_entry() {
        let root = unique_temp_dir("atlas-duplicate-sprite");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        write_file(
            &base
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
        );
        let overlay_texture = overlay
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("dirt.png");
        write_file(&overlay_texture);
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:block/stone",
                  "sprite": "minecraft:block/shared"
                }
              ]
            }"#,
        );
        write_json(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:block/dirt",
                  "sprite": "minecraft:block/shared"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let entries = load_atlas_texture_entries(&roots, "blocks").unwrap();

        assert_eq!(
            entries.iter().map(entry_id).collect::<Vec<_>>(),
            vec!["minecraft:block/shared"]
        );
        assert!(entry_path(&entries[0]).ends_with(&overlay_texture));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_filter_removes_current_entry_but_later_source_can_readd() {
        let root = unique_temp_dir("atlas-filter-readd");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_file(&assets_dir.join("textures").join("block").join("stone.png"));
        let dirt = assets_dir.join("textures").join("block").join("dirt.png");
        write_file(&dirt);
        write_json(
            &assets_dir.join("atlases").join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:block/stone"
                },
                {
                  "type": "minecraft:filter",
                  "pattern": {
                    "namespace": "minecraft",
                    "path": "block/stone"
                  }
                },
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:block/dirt",
                  "sprite": "minecraft:block/stone"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let entries = load_atlas_texture_entries(&roots, "blocks").unwrap();

        assert_eq!(
            entries.iter().map(entry_id).collect::<Vec<_>>(),
            vec!["minecraft:block/stone"]
        );
        assert!(entry_path(&entries[0]).ends_with(&dirt));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_source_uses_highest_precedence_texture_resource() {
        let root = unique_temp_dir("atlas-directory-precedence");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        write_file(
            &base
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
        );
        let overlay_texture = overlay
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        write_file(&overlay_texture);
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "source": "block",
                  "prefix": "block/"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let entries = load_atlas_texture_entries(&roots, "blocks").unwrap();

        assert_eq!(
            entries.iter().map(entry_id).collect::<Vec<_>>(),
            vec!["minecraft:block/stone"]
        );
        assert!(entry_path(&entries[0]).ends_with(&overlay_texture));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn single_source_uses_highest_precedence_texture_metadata() {
        let root = unique_temp_dir("atlas-single-metadata");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        let texture = base
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        write_rgba_png(&texture, 1, 1, &[10, 20, 30, 255]);
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{"sources":[{"type":"minecraft:single","resource":"minecraft:block/stone"}]}"#,
        );
        write_json(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png.mcmeta"),
            r#"{"texture":{"mipmap_strategy":"mean"}}"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let source = load_atlas_texture_entries(&roots, "blocks")
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .into_source()
            .unwrap();

        assert_eq!(
            source.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::Mean
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_source_uses_highest_precedence_texture_metadata() {
        let root = unique_temp_dir("atlas-directory-metadata");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        let texture = base
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        write_rgba_png(&texture, 1, 1, &[10, 20, 30, 255]);
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "source": "block",
                  "prefix": "block/"
                }
              ]
            }"#,
        );
        write_json(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png.mcmeta"),
            r#"{"texture":{"mipmap_strategy":"mean"}}"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let source = load_atlas_texture_entries(&roots, "blocks")
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .into_source()
            .unwrap();

        assert_eq!(
            source.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::Mean
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_texture_metadata_filter_clears_lower_priority_metadata() {
        let root = unique_temp_dir("atlas-metadata-filter");
        let base = root.join("sources").join(MC_VERSION);
        let filter = root.join("filter");
        let texture = base
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        write_rgba_png(&texture, 1, 1, &[10, 20, 30, 255]);
        write_json(
            &texture.with_file_name("stone.png.mcmeta"),
            r#"{"texture":{"mipmap_strategy":"mean"}}"#,
        );
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("atlases")
                .join("blocks.json"),
            r#"{"sources":[{"type":"minecraft:single","resource":"minecraft:block/stone"}]}"#,
        );
        write_json(
            &filter.join("pack.mcmeta"),
            r#"{
              "filter": {
                "block": [
                  {
                    "namespace": "minecraft",
                    "path": "textures/block/stone\\.png\\.mcmeta"
                  }
                ]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([filter]);
        let source = load_atlas_texture_entries(&roots, "blocks")
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .into_source()
            .unwrap();

        assert_eq!(
            source.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::Auto
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_filter_without_pattern_rejects_source() {
        let root = unique_temp_dir("atlas-filter-missing-pattern");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("atlases");
        std::fs::create_dir_all(&assets_dir).unwrap();
        std::fs::write(
            assets_dir.join("bad.json"),
            r#"{"sources":[{"type":"minecraft:filter"}]}"#,
        )
        .unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let err = load_atlas_texture_entries(&roots, "bad").unwrap_err();

        assert!(err.to_string().contains("missing atlas source pattern"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_unstitch_crops_regions_with_vanilla_divisors() {
        let root = unique_temp_dir("atlas-unstitch");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        std::fs::create_dir_all(assets_dir.join("textures").join("gui")).unwrap();
        std::fs::create_dir_all(assets_dir.join("atlases")).unwrap();
        write_rgba_png(
            &assets_dir.join("textures").join("gui").join("widgets.png"),
            4,
            4,
            &[
                0, 0, 100, 255, 40, 0, 100, 255, 80, 0, 100, 255, 120, 0, 100, 255, 0, 40, 100,
                255, 40, 40, 100, 255, 80, 40, 100, 255, 120, 40, 100, 255, 0, 80, 100, 255, 40,
                80, 100, 255, 80, 80, 100, 255, 120, 80, 100, 255, 0, 120, 100, 255, 40, 120, 100,
                255, 80, 120, 100, 255, 120, 120, 100, 255,
            ],
        );
        std::fs::write(
            assets_dir.join("atlases").join("unstitch.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:unstitch",
                  "resource": "minecraft:gui/widgets",
                  "divisor_x": 4.0,
                  "divisor_y": 4.0,
                  "regions": [
                    {
                      "sprite": "minecraft:widget/center",
                      "x": 1.0,
                      "y": 1.0,
                      "width": 2.0,
                      "height": 2.0
                    }
                  ]
                }
              ]
            }"#,
        )
        .unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let entries = load_atlas_texture_entries(&roots, "unstitch").unwrap();
        assert_eq!(entries.len(), 1);
        let image = entries.into_iter().next().unwrap().into_image().unwrap();

        assert_eq!(image.id, "minecraft:widget/center");
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(
            image.rgba,
            vec![40, 40, 100, 255, 80, 40, 100, 255, 40, 80, 100, 255, 80, 80, 100, 255]
        );
        assert_eq!(image.texture_metadata, SpriteTextureMetadata::default());

        std::fs::remove_dir_all(root).unwrap();
    }

    fn entry_id(entry: &AtlasTextureEntry) -> &str {
        match entry {
            AtlasTextureEntry::File { id, .. } => id,
            AtlasTextureEntry::Image(image) => &image.id,
        }
    }

    fn entry_path(entry: &AtlasTextureEntry) -> &Path {
        match entry {
            AtlasTextureEntry::File { path, .. } => path,
            AtlasTextureEntry::Image(_) => panic!("expected file atlas texture entry"),
        }
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("bbb-pack-{label}-{nanos}"));
        dir
    }

    fn write_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec()).unwrap();
        image.save(path).unwrap();
    }

    fn write_file(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, []).unwrap();
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
}
