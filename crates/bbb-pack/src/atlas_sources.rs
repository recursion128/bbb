use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Deserialize;

use crate::{
    roots::PackRoots,
    sprites::{SpriteImage, SpriteSource},
};

pub(crate) fn load_atlas_texture_entries(
    roots: &PackRoots,
    atlas_name: &str,
) -> Result<Vec<AtlasTextureEntry>> {
    let path = roots.atlas_definition(atlas_name);
    let bytes = std::fs::read(&path).with_context(|| format!("read atlas {}", path.display()))?;
    let atlas: RawAtlas = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse atlas {}", path.display()))?;

    let loader = AtlasTextureLoader::new(roots);
    let mut entries = Vec::new();
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
                entries.push(AtlasTextureEntry::File {
                    id: sprite.id(),
                    path: loader.texture_path(&resource),
                });
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
            other => bail!("unsupported atlas source type {other:?} in atlas {atlas_name}"),
        }
    }
    Ok(entries)
}

struct AtlasTextureLoader<'a> {
    roots: &'a PackRoots,
}

impl<'a> AtlasTextureLoader<'a> {
    fn new(roots: &'a PackRoots) -> Self {
        Self { roots }
    }

    fn append_directory_atlas_entries(
        &self,
        entries: &mut Vec<AtlasTextureEntry>,
        source_path: &str,
        prefix: &str,
    ) -> Result<()> {
        for (namespace, assets_dir) in self.namespace_assets_dirs()? {
            let dir = assets_dir.join("textures").join(source_path);
            if !dir.is_dir() {
                continue;
            }
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
                    id: format!("{namespace}:{prefix}{relative}"),
                    path,
                });
            }
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

    fn namespace_assets_dirs(&self) -> Result<Vec<(String, PathBuf)>> {
        let assets_root = self.roots.sources_dir.join("assets");
        let mut dirs = Vec::new();
        for entry in std::fs::read_dir(&assets_root)
            .with_context(|| format!("read assets directory {}", assets_root.display()))?
        {
            let entry =
                entry.with_context(|| format!("read assets entry in {}", assets_root.display()))?;
            let path = entry.path();
            if !entry
                .file_type()
                .with_context(|| format!("read file type {}", path.display()))?
                .is_dir()
            {
                continue;
            }
            let namespace = entry
                .file_name()
                .into_string()
                .map_err(|name| anyhow::anyhow!("non-utf8 asset namespace {name:?}"))?;
            validate_resource_namespace(&namespace)?;
            dirs.push((namespace, path));
        }
        dirs.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(dirs)
    }

    fn namespace_assets_dir(&self, namespace: &str) -> PathBuf {
        self.roots.sources_dir.join("assets").join(namespace)
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
    pattern: Option<RawIdentifierPattern>,
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
}

#[derive(Debug, Deserialize)]
struct RawIdentifierPattern {
    namespace: Option<String>,
    path: Option<String>,
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
            .map_or(true, |pattern| pattern.is_match(&id.namespace))
            && self
                .path
                .as_ref()
                .map_or(true, |pattern| pattern.is_match(&id.path))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResourceLocation {
    namespace: String,
    path: String,
}

impl ResourceLocation {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        let (namespace, path) = value.split_once(':').unwrap_or(("minecraft", value));
        validate_resource_namespace(namespace)?;
        validate_resource_path(path)?;
        Ok(Self {
            namespace: namespace.to_string(),
            path: path.to_string(),
        })
    }

    pub(crate) fn id(&self) -> String {
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AtlasTextureEntry {
    File { id: String, path: PathBuf },
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
            AtlasTextureEntry::File { id, path } => SpriteSource::from_png_file(id, path),
            AtlasTextureEntry::Image(image) => Ok(image.source()),
        }
    }

    pub(crate) fn into_image(self) -> Result<SpriteImage> {
        match self {
            AtlasTextureEntry::File { id, path } => SpriteImage::from_png_file(id, path),
            AtlasTextureEntry::Image(image) => Ok(image),
        }
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
    let mut image = SpriteImage::new(id, base.width, base.height, rgba)?;
    image.texture_metadata = base.texture_metadata;
    Ok(image)
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
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{PackRoots, MC_VERSION};
    use crate::{SpriteImage, SpriteMipmapStrategy, SpriteTextureMetadata};

    #[test]
    fn paletted_permutations_follow_vanilla_palette_size_rules() {
        let mut base =
            SpriteImage::new("minecraft:pattern/base", 1, 1, vec![20, 20, 20, 255]).unwrap();
        base.texture_metadata = SpriteTextureMetadata {
            blur: false,
            clamp: false,
            mipmap_strategy: SpriteMipmapStrategy::StrictCutout,
            alpha_cutoff_bias: 0.25,
        };
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
        assert_eq!(image.texture_metadata, base.texture_metadata);

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

    fn entry_id(entry: &AtlasTextureEntry) -> &str {
        match entry {
            AtlasTextureEntry::File { id, .. } => id,
            AtlasTextureEntry::Image(image) => &image.id,
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
}
