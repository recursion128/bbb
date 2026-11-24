use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use bbb_registry_codec::pack_filter::{IdentifierPattern, IdentifierPatternSource};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::{
    atlas::MAX_ATLAS_ANIMATION_FRAMES,
    image::{
        admit_png_snapshot, decode_admitted_png_snapshot_rgba,
        decode_png_resource_rgba_with_budget, read_png_resource_snapshot,
        verify_admitted_png_snapshot_dimensions, PngDecodeBudget,
    },
    resources::{
        validate_resource_path, PackResource, PackResourceStack, ResourceLocation,
        MAX_RESOURCE_PATH_BYTES,
    },
    rgba_len,
    sprites::{SpriteImage, SpriteSource, SpriteTransparency},
};

type AtlasEntries = IndexMap<ResourceLocation, AtlasTextureEntry>;
const MAX_ATLAS_FILTER_RULES: usize = 256;
const MAX_ATLAS_ENTRIES: usize = 32_768;
const ATLAS_ENTRY_LOGICAL_BYTES: u64 = 128;

pub(crate) fn load_atlas_texture_entries_with_budget(
    stack: &PackResourceStack,
    atlas_name: &str,
    budget: &mut PngDecodeBudget,
) -> Result<Vec<AtlasTextureEntry>> {
    let atlas_location = atlas_definition_location(atlas_name)?;
    let atlas_resources = stack.get_resource_stack(&atlas_location);
    if atlas_resources.is_empty() {
        bail!("missing atlas {atlas_name}");
    }
    let mut loader = AtlasTextureLoader {
        stack,
        budget,
        raw_texture_cache: HashMap::new(),
    };
    let mut entries = AtlasEntries::new();
    let mut filter_rules = 0usize;

    for resource in atlas_resources {
        let atlas: RawAtlas = match stack.read_json_resource(&resource, "atlas definition") {
            Ok(atlas) => atlas,
            Err(err) => {
                // Propagate budget exhaustion errors instead of treating them as malformed JSON.
                stack.ensure_resource_data_admitted().map_err(|_| err)?;
                continue;
            }
        };

        for source in atlas.sources {
            let source_type = ResourceLocation::parse(&source.source_type)
                .with_context(|| format!("invalid atlas source type {:?}", source.source_type))?
                .id();
            match source_type.as_str() {
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
                    filter_rules += 1;
                    if filter_rules > MAX_ATLAS_FILTER_RULES {
                        return Err(stack.reject_resource_data(format!(
                            "atlas {atlas_name} has more than {MAX_ATLAS_FILTER_RULES} filter rules"
                        )));
                    }
                    let pattern = source.required_pattern()?;
                    entries.retain(|id, _| !pattern.matches(id.namespace(), id.path()));
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
    entries.shift_remove(&ResourceLocation::parse(MISSING_TEXTURE_ID)?);
    Ok(
        std::iter::once(AtlasTextureEntry::Image(missing_texture_image()?))
            .chain(entries.into_values())
            .collect(),
    )
}

#[cfg(test)]
fn load_atlas_texture_entries(
    roots: &crate::PackRoots,
    atlas_name: &str,
) -> Result<Vec<AtlasTextureEntry>> {
    let mut budget = PngDecodeBudget::atlas();
    load_atlas_texture_entries_with_budget(&roots.resource_stack(), atlas_name, &mut budget)
}

const MISSING_TEXTURE_ID: &str = "minecraft:missingno";
const MISSING_TEXTURE_SIZE: u32 = 16;

fn missing_texture_image() -> Result<SpriteImage> {
    let mut rgba = Vec::with_capacity((MISSING_TEXTURE_SIZE * MISSING_TEXTURE_SIZE * 4) as usize);
    for y in 0..MISSING_TEXTURE_SIZE {
        for x in 0..MISSING_TEXTURE_SIZE {
            let magenta = (y < MISSING_TEXTURE_SIZE / 2) ^ (x < MISSING_TEXTURE_SIZE / 2);
            if magenta {
                rgba.extend_from_slice(&[248, 0, 248, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    SpriteImage::new(
        MISSING_TEXTURE_ID,
        MISSING_TEXTURE_SIZE,
        MISSING_TEXTURE_SIZE,
        rgba,
    )
}

fn atlas_definition_location(atlas_name: &str) -> Result<ResourceLocation> {
    let atlas_id = ResourceLocation::parse(atlas_name)?;
    ResourceLocation::new(
        atlas_id.namespace().to_string(),
        format!("atlases/{}.json", atlas_id.path()),
    )
}

struct AtlasTextureLoader<'a> {
    stack: &'a PackResourceStack,
    budget: &'a mut PngDecodeBudget,
    raw_texture_cache: HashMap<ResourceLocation, Arc<SpriteImage>>,
}

impl<'a> AtlasTextureLoader<'a> {
    fn admit_entry(&self, entries: &AtlasEntries, id: &ResourceLocation) -> Result<()> {
        if !entries.contains_key(id) {
            let next_count = u64::try_from(entries.len().saturating_add(1))
                .context("atlas entry count overflow")?;
            self.stack.admit_resource_collection_count(
                next_count,
                MAX_ATLAS_ENTRIES as u64,
                "atlas sprite entries",
            )?;
        }
        let id_bytes = u64::try_from(id.namespace().len())
            .ok()
            .and_then(|ns| ns.checked_add(1))
            .and_then(|ns| ns.checked_add(u64::try_from(id.path().len()).ok()?))
            .and_then(|bytes| bytes.checked_add(ATLAS_ENTRY_LOGICAL_BYTES))
            .ok_or_else(|| {
                self.stack
                    .reject_resource_data("atlas sprite entry logical size overflow")
            })?;
        self.stack
            .reserve_derived_resource_bytes(id_bytes, "atlas sprite entry")
    }

    fn append_directory_atlas_entries(
        &self,
        entries: &mut AtlasEntries,
        source_path: &str,
        prefix: &str,
    ) -> Result<()> {
        if prefix.len() > MAX_RESOURCE_PATH_BYTES {
            return Err(self.stack.reject_resource_data(format!(
                "atlas directory prefix length {} exceeds {MAX_RESOURCE_PATH_BYTES} byte limit",
                prefix.len()
            )));
        }
        let texture_prefix = resource_texture_prefix(source_path);
        for resource in self.stack.list_resources(&texture_prefix, ".png")? {
            let id = directory_sprite_id(&resource.location, source_path, prefix)?;
            self.admit_entry(entries, &id)?;
            let sprite_id = id.id();
            entries.insert(
                id,
                AtlasTextureEntry::File {
                    id: sprite_id,
                    resource: Box::new(resource),
                },
            );
        }
        Ok(())
    }

    fn append_single_entry(
        &self,
        entries: &mut AtlasEntries,
        resource: &ResourceLocation,
        sprite: &ResourceLocation,
    ) -> Result<()> {
        if let Some(resource) = self.texture_resource(resource)? {
            self.admit_entry(entries, sprite)?;
            entries.insert(
                sprite.clone(),
                AtlasTextureEntry::File {
                    id: sprite.id(),
                    resource: Box::new(resource),
                },
            );
        }
        Ok(())
    }

    fn append_paletted_permutation_entries(
        &mut self,
        entries: &mut AtlasEntries,
        textures: &[ResourceLocation],
        palette_key: &ResourceLocation,
        permutations: &BTreeMap<String, ResourceLocation>,
        separator: &str,
    ) -> Result<()> {
        if separator.len() > MAX_RESOURCE_PATH_BYTES {
            return Err(self.stack.reject_resource_data(format!(
                "atlas paletted_permutations separator length {} exceeds {MAX_RESOURCE_PATH_BYTES} byte limit",
                separator.len()
            )));
        }
        let planned_outputs = textures
            .len()
            .checked_mul(permutations.len())
            .ok_or_else(|| {
                self.stack
                    .reject_resource_data("atlas paletted_permutations output count overflow")
            })?;
        self.stack.admit_resource_collection_count(
            u64::try_from(planned_outputs).unwrap_or(u64::MAX),
            MAX_ATLAS_ENTRIES as u64,
            "atlas paletted_permutations outputs",
        )?;
        let key = self.required_raw_texture_image(palette_key)?;
        let key_indices = palette_indices(&key);
        let mut palettes = Vec::with_capacity(permutations.len());
        for (suffix, palette) in permutations {
            if suffix.len() > MAX_RESOURCE_PATH_BYTES {
                return Err(self.stack.reject_resource_data(format!(
                    "atlas paletted_permutations suffix length {} exceeds {MAX_RESOURCE_PATH_BYTES} byte limit",
                    suffix.len()
                )));
            }
            palettes.push((suffix.as_str(), self.required_raw_texture_image(palette)?));
        }

        for texture in textures {
            let base = self.required_raw_texture_image(texture)?;
            let outputs =
                apply_palette_permutations(&base, &key, &key_indices, &palettes, self.budget)?;
            for ((suffix, _), rgba) in palettes.iter().zip(outputs) {
                let permutation = texture.with_suffix(&format!("{separator}{suffix}"))?;
                self.admit_entry(entries, &permutation)?;
                let image = SpriteImage::new(permutation.id(), base.width, base.height, rgba)?;
                entries.insert(permutation, AtlasTextureEntry::Image(image));
            }
        }
        Ok(())
    }

    fn append_unstitch_entries(
        &mut self,
        entries: &mut AtlasEntries,
        resource: &ResourceLocation,
        regions: &[UnstitchRegion],
        divisor_x: f64,
        divisor_y: f64,
    ) -> Result<()> {
        self.stack.admit_resource_collection_count(
            u64::try_from(regions.len()).unwrap_or(u64::MAX),
            MAX_ATLAS_ENTRIES as u64,
            "atlas unstitch regions",
        )?;
        let pack_resource = self.required_texture_resource(resource)?;
        let (source_width, source_height, rgba) =
            read_raw_png(&pack_resource, self.stack, self.budget)?;
        for region in regions {
            self.admit_entry(entries, &region.sprite)?;
            entries.insert(
                region.sprite.clone(),
                AtlasTextureEntry::Image(unstitch_region(
                    source_width,
                    source_height,
                    &rgba,
                    region,
                    divisor_x,
                    divisor_y,
                    self.budget,
                )?),
            );
        }
        Ok(())
    }

    fn required_texture_resource(&self, location: &ResourceLocation) -> Result<PackResource> {
        self.texture_resource(location)?
            .ok_or_else(|| anyhow::anyhow!("missing atlas texture {}", location.id()))
    }

    fn texture_resource(&self, location: &ResourceLocation) -> Result<Option<PackResource>> {
        let resource = texture_resource_location(location)?;
        Ok(self.stack.get_resource(&resource))
    }

    fn required_raw_texture_image(
        &mut self,
        location: &ResourceLocation,
    ) -> Result<Arc<SpriteImage>> {
        if let Some(cached) = self.raw_texture_cache.get(location) {
            return Ok(Arc::clone(cached));
        }
        let resource = self.required_texture_resource(location)?;
        let (width, height, rgba) = read_raw_png(&resource, self.stack, self.budget)?;
        let image = Arc::new(SpriteImage::new(location.id(), width, height, rgba)?);
        self.raw_texture_cache
            .insert(location.clone(), Arc::clone(&image));
        Ok(image)
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
) -> Result<ResourceLocation> {
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
    pattern: Option<IdentifierPatternSource>,
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
        if !value.is_empty() {
            validate_resource_path(value)?;
        }
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
            .map_err(|error| anyhow::anyhow!("compile atlas filter: {error}"))
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AtlasTextureEntry {
    File {
        id: String,
        resource: Box<PackResource>,
    },
    Image(SpriteImage),
}

impl AtlasTextureEntry {
    #[cfg(test)]
    pub(crate) fn into_source(self) -> Result<SpriteSource> {
        let mut budget = PngDecodeBudget::single_image();
        self.into_source_with_budget(&mut budget, &PackResourceStack::default())
    }

    #[cfg(test)]
    pub(crate) fn into_source_with_budget(
        self,
        budget: &mut PngDecodeBudget,
        resources: &PackResourceStack,
    ) -> Result<SpriteSource> {
        match self {
            AtlasTextureEntry::File { id, resource } => {
                SpriteSource::from_pack_resource_with_budget(id, &resource, budget, resources)
            }
            AtlasTextureEntry::Image(image) => Ok(image.source()),
        }
    }

    #[cfg(test)]
    pub(crate) fn into_image(self) -> Result<SpriteImage> {
        let mut budget = PngDecodeBudget::single_image();
        self.into_image_with_budget(&mut budget, &PackResourceStack::default())
    }

    #[cfg(test)]
    pub(crate) fn into_image_with_budget(
        self,
        budget: &mut PngDecodeBudget,
        resources: &PackResourceStack,
    ) -> Result<SpriteImage> {
        match self {
            AtlasTextureEntry::File { id, resource } => {
                SpriteImage::from_pack_resource_with_budget(id, &resource, budget, resources)
            }
            AtlasTextureEntry::Image(image) => Ok(image),
        }
    }
}

pub(crate) fn decode_atlas_texture_sources(
    entries: Vec<AtlasTextureEntry>,
    budget: &mut PngDecodeBudget,
    resources: &PackResourceStack,
) -> Result<Vec<SpriteSource>> {
    use rayon::prelude::*;

    enum PreparedSource {
        Ready(SpriteSource),
        File {
            id: String,
            resource: Box<PackResource>,
            bytes: Vec<u8>,
        },
    }

    enum AdmittedSource {
        Ready(SpriteSource),
        File {
            id: String,
            resource: Box<PackResource>,
            bytes: Vec<u8>,
            header_dimensions: (u32, u32),
        },
    }

    enum VerifiedSource {
        Ready(SpriteSource),
        File {
            id: String,
            resource: Box<PackResource>,
            dimensions: (u32, u32),
        },
    }

    let prepare_source = |entry| match entry {
        AtlasTextureEntry::Image(image) => Ok(PreparedSource::Ready(image.source())),
        AtlasTextureEntry::File { id, resource } => {
            resources.validate_resource(&resource, &format!("sprite source {id}"))?;
            resources.validate_resource_metadata(&resource, &format!("sprite metadata {id}"))?;
            let bytes = read_png_resource_snapshot(&resource, "sprite source")?;
            Ok(PreparedSource::File {
                id,
                resource,
                bytes,
            })
        }
    };
    let prepared: Vec<Result<PreparedSource>> = if crate::can_use_rayon() {
        entries.into_par_iter().map(prepare_source).collect()
    } else {
        entries.into_iter().map(prepare_source).collect()
    };

    let mut admitted = Vec::with_capacity(prepared.len());
    let mut deferred_err = None;
    for item in prepared {
        match item {
            Err(err) => {
                deferred_err = Some(err);
                break;
            }
            Ok(PreparedSource::Ready(source)) => admitted.push(AdmittedSource::Ready(source)),
            Ok(PreparedSource::File {
                id,
                resource,
                bytes,
            }) => match admit_png_snapshot(&bytes, "sprite source", budget) {
                Ok(header_dimensions) => admitted.push(AdmittedSource::File {
                    id,
                    resource,
                    bytes,
                    header_dimensions,
                }),
                Err(err) => {
                    deferred_err = Some(err);
                    break;
                }
            },
        }
    }

    let verify_source = |entry| match entry {
        AdmittedSource::Ready(source) => Ok(VerifiedSource::Ready(source)),
        AdmittedSource::File {
            id,
            resource,
            bytes,
            header_dimensions,
        } => {
            let dimensions = verify_admitted_png_snapshot_dimensions(
                &bytes,
                header_dimensions,
                &resource.path,
                "sprite source",
            )?;
            Ok(VerifiedSource::File {
                id,
                resource,
                dimensions,
            })
        }
    };
    let verified: Vec<Result<VerifiedSource>> = if crate::can_use_rayon() {
        admitted.into_par_iter().map(verify_source).collect()
    } else {
        admitted.into_iter().map(verify_source).collect()
    };

    let mut sources = Vec::with_capacity(verified.len());
    for item in verified {
        match item? {
            VerifiedSource::Ready(source) => sources.push(source),
            VerifiedSource::File {
                id,
                resource,
                dimensions: (image_width, image_height),
            } => sources.push(SpriteSource::finish_from_pack_resource_dimensions(
                id,
                &resource,
                image_width,
                image_height,
                resources,
            )?),
        }
    }
    if let Some(err) = deferred_err {
        return Err(err);
    }
    Ok(sources)
}

pub(crate) fn decode_atlas_texture_images(
    entries: Vec<AtlasTextureEntry>,
    budget: &mut PngDecodeBudget,
    resources: &PackResourceStack,
) -> Result<Vec<SpriteImage>> {
    use rayon::prelude::*;

    enum PreparedImage {
        Ready(SpriteImage),
        File {
            id: String,
            resource: Box<PackResource>,
            bytes: Vec<u8>,
        },
    }

    enum AdmittedImage {
        Ready(SpriteImage),
        File {
            id: String,
            resource: Box<PackResource>,
            bytes: Vec<u8>,
            header_dimensions: (u32, u32),
        },
    }

    enum DecodedImage {
        Ready(SpriteImage),
        File {
            id: String,
            resource: Box<PackResource>,
            image_width: u32,
            image_height: u32,
            source_rgba: Vec<u8>,
            transparency: SpriteTransparency,
        },
    }

    let prepare_image = |entry| match entry {
        AtlasTextureEntry::Image(image) => Ok(PreparedImage::Ready(image)),
        AtlasTextureEntry::File { id, resource } => {
            resources.validate_resource(&resource, &format!("sprite image {id}"))?;
            resources.validate_resource_metadata(&resource, &format!("sprite metadata {id}"))?;
            let bytes = read_png_resource_snapshot(&resource, "sprite image")
                .with_context(|| format!("open sprite {}", resource.path.display()))?;
            Ok(PreparedImage::File {
                id,
                resource,
                bytes,
            })
        }
    };
    let prepared: Vec<Result<PreparedImage>> = if crate::can_use_rayon() {
        entries.into_par_iter().map(prepare_image).collect()
    } else {
        entries.into_iter().map(prepare_image).collect()
    };

    let mut admitted = Vec::with_capacity(prepared.len());
    let mut deferred_err = None;
    for item in prepared {
        match item {
            Err(err) => {
                deferred_err = Some(err);
                break;
            }
            Ok(PreparedImage::Ready(image)) => admitted.push(AdmittedImage::Ready(image)),
            Ok(PreparedImage::File {
                id,
                resource,
                bytes,
            }) => match admit_png_snapshot(&bytes, "sprite image", budget)
                .with_context(|| format!("open sprite {}", resource.path.display()))
            {
                Ok(header_dimensions) => admitted.push(AdmittedImage::File {
                    id,
                    resource,
                    bytes,
                    header_dimensions,
                }),
                Err(err) => {
                    deferred_err = Some(err);
                    break;
                }
            },
        }
    }

    let decode_image = |entry| match entry {
        AdmittedImage::Ready(image) => Ok(DecodedImage::Ready(image)),
        AdmittedImage::File {
            id,
            resource,
            bytes,
            header_dimensions,
        } => {
            let rgba = decode_admitted_png_snapshot_rgba(
                &bytes,
                header_dimensions,
                &resource.path,
                "sprite image",
            )
            .with_context(|| format!("open sprite {}", resource.path.display()))?;
            let (image_width, image_height) = rgba.dimensions();
            let source_rgba = rgba.into_raw();
            let transparency = SpriteTransparency::from_rgba(&source_rgba);
            Ok(DecodedImage::File {
                id,
                resource,
                image_width,
                image_height,
                source_rgba,
                transparency,
            })
        }
    };
    let decoded: Vec<Result<DecodedImage>> = if crate::can_use_rayon() {
        admitted.into_par_iter().map(decode_image).collect()
    } else {
        admitted.into_iter().map(decode_image).collect()
    };

    let mut images = Vec::with_capacity(decoded.len());
    let mut total_animation_frames = 0usize;
    for item in decoded {
        let image = match item? {
            DecodedImage::Ready(image) => image,
            DecodedImage::File {
                id,
                resource,
                image_width,
                image_height,
                source_rgba,
                transparency,
            } => SpriteImage::finish_from_pack_resource_rgba(
                id,
                &resource,
                image_width,
                image_height,
                source_rgba,
                transparency,
                resources,
                budget,
            )?,
        };
        if let Some(animation) = image.animation.as_ref() {
            total_animation_frames = total_animation_frames
                .checked_add(animation.frame_count as usize)
                .filter(|&count| count <= MAX_ATLAS_ANIMATION_FRAMES)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "atlas animation frames exceed the {MAX_ATLAS_ANIMATION_FRAMES} total frame limit"
                    )
                })?;
        }
        images.push(image);
    }
    if let Some(err) = deferred_err {
        return Err(err);
    }
    Ok(images)
}

fn read_raw_png(
    resource: &PackResource,
    resources: &PackResourceStack,
    budget: &mut PngDecodeBudget,
) -> Result<(u32, u32, Vec<u8>)> {
    resources.validate_resource(resource, "atlas source")?;
    let rgba = decode_png_resource_rgba_with_budget(resource, "atlas source", budget)?;
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
    budget: &mut PngDecodeBudget,
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

    budget.reserve_rgba(width, height, "unstitch output")?;
    let mut rgba = Vec::with_capacity(rgba_len(width, height)?);
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

fn palette_indices(key: &SpriteImage) -> HashMap<[u8; 3], usize> {
    key.rgba
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, pixel)| pixel[3] != 0)
        .map(|(index, pixel)| ([pixel[0], pixel[1], pixel[2]], index * 4))
        .collect()
}

fn apply_palette_permutations<P: Borrow<SpriteImage>>(
    base: &SpriteImage,
    key: &SpriteImage,
    key_indices: &HashMap<[u8; 3], usize>,
    palettes: &[(&str, P)],
    budget: &mut PngDecodeBudget,
) -> Result<Vec<Vec<u8>>> {
    let mut outputs = Vec::with_capacity(palettes.len());
    for (_, values) in palettes {
        let values = values.borrow();
        budget.reserve_rgba(base.width, base.height, "palette permutation output")?;
        if key.rgba.len() != values.rgba.len() {
            bail!(
                "palette mapping has different sizes: {} and {} pixels",
                key.rgba.len() / 4,
                values.rgba.len() / 4
            );
        }
        outputs.push(base.rgba.clone());
    }
    for (index, pixel) in base.rgba.as_chunks::<4>().0.iter().enumerate() {
        if pixel[3] == 0 {
            continue;
        }
        let Some(&color) = key_indices.get(&[pixel[0], pixel[1], pixel[2]]) else {
            continue;
        };
        let offset = index * 4;
        for ((_, values), output) in palettes.iter().zip(&mut outputs) {
            let values = values.borrow();
            output[offset..offset + 3].copy_from_slice(&values.rgba[color..color + 3]);
            output[offset + 3] =
                (u16::from(pixel[3]) * u16::from(values.rgba[color + 3]) / 255) as u8;
        }
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_palette_permutations, load_atlas_texture_entries,
        load_atlas_texture_entries_with_budget, palette_indices, AtlasTextureEntry,
    };
    use crate::image::PngDecodeBudget;
    use regex::Regex;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{PackRoots, MC_VERSION};
    use crate::{SpriteImage, SpriteMipmapStrategy, SpriteTextureMetadata};

    #[test]
    fn atlas_source_supported_types_match_vanilla_bootstrap_when_sources_are_available() {
        let Some(source) = local_vanilla_source(&[
            "net",
            "minecraft",
            "client",
            "renderer",
            "texture",
            "atlas",
            "SpriteSources.java",
        ]) else {
            return;
        };
        let source_types = BTreeSet::from([
            "minecraft:directory".to_string(),
            "minecraft:filter".to_string(),
            "minecraft:paletted_permutations".to_string(),
            "minecraft:single".to_string(),
            "minecraft:unstitch".to_string(),
        ]);

        assert_eq!(vanilla_bootstrap_ids(&source), source_types);

        let root = unique_temp_dir("atlas-vanilla-source-types");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        std::fs::create_dir_all(assets_dir.join("textures").join("block")).unwrap();
        std::fs::create_dir_all(assets_dir.join("textures").join("item")).unwrap();
        std::fs::create_dir_all(assets_dir.join("textures").join("pattern")).unwrap();
        std::fs::create_dir_all(assets_dir.join("textures").join("palette")).unwrap();
        std::fs::create_dir_all(assets_dir.join("textures").join("gui")).unwrap();
        std::fs::create_dir_all(assets_dir.join("atlases")).unwrap();
        write_file(&assets_dir.join("textures").join("block").join("stone.png"));
        write_file(&assets_dir.join("textures").join("item").join("test.png"));
        write_rgba_png(
            &assets_dir.join("textures").join("pattern").join("base.png"),
            1,
            1,
            &[10, 10, 10, 255],
        );
        write_rgba_png(
            &assets_dir.join("textures").join("palette").join("key.png"),
            1,
            1,
            &[10, 10, 10, 255],
        );
        write_rgba_png(
            &assets_dir.join("textures").join("palette").join("red.png"),
            1,
            1,
            &[120, 0, 0, 255],
        );
        write_rgba_png(
            &assets_dir.join("textures").join("gui").join("widgets.png"),
            2,
            2,
            &[
                10, 10, 40, 255, 20, 10, 40, 255, 10, 20, 40, 255, 20, 20, 40, 255,
            ],
        );
        write_json(
            &assets_dir.join("atlases").join("vanilla_source_types.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "source": "block",
                  "prefix": "block/"
                },
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:item/test"
                },
                {
                  "type": "minecraft:filter",
                  "pattern": {
                    "path": "^does_not_match$"
                  }
                },
                {
                  "type": "minecraft:paletted_permutations",
                  "textures": ["minecraft:pattern/base"],
                  "palette_key": "minecraft:palette/key",
                  "permutations": {
                    "red": "minecraft:palette/red"
                  }
                },
                {
                  "type": "minecraft:unstitch",
                  "resource": "minecraft:gui/widgets",
                  "divisor_x": 2.0,
                  "divisor_y": 2.0,
                  "regions": [
                    {
                      "sprite": "minecraft:gui/top_left",
                      "x": 0.0,
                      "y": 0.0,
                      "width": 1.0,
                      "height": 1.0
                    }
                  ]
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let entries = load_atlas_texture_entries(&roots, "vanilla_source_types").unwrap();
        let ids = entries.iter().map(entry_id).collect::<BTreeSet<_>>();
        assert_eq!(
            ids,
            BTreeSet::from([
                "minecraft:missingno",
                "minecraft:block/stone",
                "minecraft:item/test",
                "minecraft:pattern/base_red",
                "minecraft:gui/top_left",
            ])
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
        let mut budget = PngDecodeBudget::atlas();

        let indices = palette_indices(&key);
        let rgba = apply_palette_permutations(
            &base,
            &key,
            &indices,
            &[("value", same_pixel_count_different_shape)],
            &mut budget,
        )
        .unwrap();
        assert_eq!(rgba, [vec![0, 100, 0, 255]]);

        let wrong_pixel_count =
            SpriteImage::new("minecraft:palette/wrong", 1, 1, vec![100, 0, 0, 255]).unwrap();
        let err = apply_palette_permutations(
            &base,
            &key,
            &indices,
            &[("wrong", wrong_pixel_count)],
            &mut budget,
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
                    "path": "filtered(?=_stone)|\\Qentity/\\E"
                  }
                }
              ]
            }"#,
        )
        .unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let entries = load_atlas_texture_entries(&roots, "filtered").unwrap();
        let ids = entries.iter().map(entry_id).collect::<Vec<_>>();

        assert_eq!(ids, vec!["minecraft:missingno", "minecraft:block/stone"]);

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
            vec![
                "minecraft:missingno",
                "minecraft:block/stone",
                "minecraft:block/deepslate"
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_source_type_accepts_default_namespace() {
        let root = unique_temp_dir("atlas-source-default-namespace");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_file(&assets_dir.join("textures").join("block").join("stone.png"));
        write_json(
            &assets_dir.join("atlases").join("blocks.json"),
            r#"{"sources":[{"type":"single","resource":"minecraft:block/stone"}]}"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let entries = load_atlas_texture_entries(&roots, "blocks").unwrap();

        assert_eq!(
            entries.iter().map(entry_id).collect::<Vec<_>>(),
            vec!["minecraft:missingno", "minecraft:block/stone"]
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
            vec!["minecraft:missingno", "minecraft:block/stone"]
        );

        let particle_sources = roots.load_atlas_texture_sources("particles").unwrap();
        assert_eq!(particle_sources.len(), 2);
        assert_eq!(particle_sources[0].id, "minecraft:missingno");
        assert_eq!(particle_sources[1].id, "minecraft:particle/spark");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_definition_propagates_data_budget_exhaustion() {
        let root = unique_temp_dir("atlas-budget-error");
        let base = root.join("sources").join(MC_VERSION);
        let base_assets = base.join("assets").join("minecraft");
        write_json(
            &base_assets.join("atlases").join("blocks.json"),
            r#"{"sources":[{"type":"minecraft:single","resource":"minecraft:block/stone"}]}"#,
        );

        let budget = crate::resource_budget::ResourceDataGenerationBudget::new(8);
        let stack = crate::PackResourceStack::from_roots_with_data_budget([base], budget);
        let mut image_budget = PngDecodeBudget::atlas();
        let error = load_atlas_texture_entries_with_budget(&stack, "blocks", &mut image_budget)
            .unwrap_err();
        assert!(format!("{error:#}").contains("resource data"));

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
            vec!["minecraft:missingno", "minecraft:block/shared"]
        );
        let shared = entries
            .iter()
            .find(|entry| entry_id(entry) == "minecraft:block/shared")
            .unwrap();
        assert!(entry_path(shared).ends_with(&overlay_texture));

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
            vec!["minecraft:missingno", "minecraft:block/stone"]
        );
        let stone = entries
            .iter()
            .find(|entry| entry_id(entry) == "minecraft:block/stone")
            .unwrap();
        assert!(entry_path(stone).ends_with(&dirt));

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
            vec!["minecraft:missingno", "minecraft:block/stone"]
        );
        let stone = entries
            .iter()
            .find(|entry| entry_id(entry) == "minecraft:block/stone")
            .unwrap();
        assert!(entry_path(stone).ends_with(&overlay_texture));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_source_accepts_empty_source_path() {
        let root = unique_temp_dir("atlas-directory-empty-source");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_file(&assets_dir.join("textures").join("block").join("stone.png"));
        write_json(
            &assets_dir.join("atlases").join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "source": "",
                  "prefix": ""
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let entries = load_atlas_texture_entries(&roots, "blocks").unwrap();

        assert_eq!(
            entries.iter().map(entry_id).collect::<Vec<_>>(),
            vec!["minecraft:missingno", "minecraft:block/stone"]
        );

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
            .find(|entry| entry_id(entry) == "minecraft:block/stone")
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
            .find(|entry| entry_id(entry) == "minecraft:block/stone")
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
        assert_eq!(entries.len(), 2);
        let image = entries
            .into_iter()
            .find(|entry| entry_id(entry) == "minecraft:widget/center")
            .unwrap()
            .into_image()
            .unwrap();

        assert_eq!(image.id, "minecraft:widget/center");
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(
            image.rgba,
            vec![40, 40, 100, 255, 80, 40, 100, 255, 40, 80, 100, 255, 80, 80, 100, 255]
        );
        assert_eq!(image.texture_metadata, SpriteTextureMetadata::default());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parallel_atlas_decode_preserves_sprite_order_and_earlier_content_error_priority() {
        let root = unique_temp_dir("parallel-decode-order");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets/minecraft");
        std::fs::create_dir_all(assets_dir.join("textures/block")).unwrap();
        let valid_rgba = [
            10_u8, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        let a_path = assets_dir.join("textures/block/a.png");
        let b_path = assets_dir.join("textures/block/b.png");
        let c_path = assets_dir.join("textures/block/c.png");
        write_rgba_png(&a_path, 2, 2, &valid_rgba);
        write_rgba_png(&b_path, 2, 2, &valid_rgba);
        write_rgba_png(&c_path, 2, 2, &valid_rgba);
        write_json(
            &assets_dir.join("atlases/ordered.json"),
            r#"{"sources":[
                {"type":"minecraft:single","resource":"minecraft:block/a"},
                {"type":"minecraft:single","resource":"minecraft:block/b"},
                {"type":"minecraft:single","resource":"minecraft:block/c"}
            ]}"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let images = roots.load_atlas_texture_images("ordered").unwrap();
        let ids: Vec<&str> = images.iter().map(|img| img.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "minecraft:missingno",
                "minecraft:block/a",
                "minecraft:block/b",
                "minecraft:block/c",
            ]
        );

        // Replace `b.png` with valid IHDR+CRC framing but corrupted IDAT zlib data,
        // and set a generation budget that fits `a` + `b` (32 bytes) but rejects `c` (48 bytes).
        // Parallel decoding must still report `b`'s `ResourcePngContentInvalid` rather than `c`'s
        // later budget exhaustion error.
        let mut corrupt_b = std::fs::read(&b_path).unwrap();
        let mut offset = 8_usize;
        while offset + 12 <= corrupt_b.len() {
            let len =
                u32::from_be_bytes(corrupt_b[offset..offset + 4].try_into().unwrap()) as usize;
            let kind = &corrupt_b[offset + 4..offset + 8];
            if kind == b"IDAT" && len > 2 {
                corrupt_b[offset + 8] ^= 0xff;
                corrupt_b[offset + 9] ^= 0xff;
                let crc = crc32fast::hash(&corrupt_b[offset + 4..offset + 8 + len]);
                corrupt_b[offset + 8 + len..offset + 12 + len].copy_from_slice(&crc.to_be_bytes());
                break;
            }
            offset += 12 + len;
        }
        std::fs::write(&b_path, corrupt_b).unwrap();

        let selected = crate::SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let bounded = roots
            .with_client_resource_generation_image_limit(1, selected, 32)
            .unwrap();
        let err = bounded.load_atlas_texture_images("ordered").unwrap_err();
        assert!(
            err.downcast_ref::<crate::ResourcePngContentInvalid>()
                .is_some(),
            "expected earlier corrupt IDAT content error before later budget error, got: {err:#}"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_filter_source_rejects_excessive_filter_rules() {
        let root = unique_temp_dir("atlas-filter-rule-cap");
        let assets = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets/minecraft");
        let sources: Vec<serde_json::Value> = (0..=super::MAX_ATLAS_FILTER_RULES)
            .map(|i| {
                serde_json::json!({
                    "type": "minecraft:filter",
                    "pattern": { "namespace": "minecraft", "path": format!("block/{i}") }
                })
            })
            .collect();
        write_json(
            &assets.join("atlases/filtered.json"),
            &serde_json::to_string(&serde_json::json!({ "sources": sources })).unwrap(),
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let err = load_atlas_texture_entries(&roots, "filtered").unwrap_err();
        assert!(format!("{err:#}").contains("filter rules"), "{err:#}");
        assert!(roots.ensure_resource_data_admitted().is_err());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_sources_reject_oversized_prefix_and_separator_and_cache_palettes() {
        let root = unique_temp_dir("atlas-prefix-sep-cache");
        let assets = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets/minecraft");
        std::fs::create_dir_all(assets.join("textures/block")).unwrap();
        std::fs::create_dir_all(assets.join("textures/palettes")).unwrap();
        write_rgba_png(
            &assets.join("textures/block/stone.png"),
            1,
            1,
            &[1, 2, 3, 255],
        );
        write_rgba_png(
            &assets.join("textures/palettes/key.png"),
            1,
            1,
            &[1, 2, 3, 255],
        );
        write_rgba_png(
            &assets.join("textures/palettes/p.png"),
            1,
            1,
            &[9, 8, 7, 255],
        );

        let long_prefix = "a".repeat(super::MAX_RESOURCE_PATH_BYTES + 1);
        write_json(
            &assets.join("atlases/dir_bomb.json"),
            &serde_json::to_string(&serde_json::json!({
                "sources": [{
                    "type": "minecraft:directory",
                    "source": "block",
                    "prefix": long_prefix
                }]
            }))
            .unwrap(),
        );
        let roots = PackRoots::from_root(&root).unwrap();
        let err = load_atlas_texture_entries(&roots, "dir_bomb").unwrap_err();
        assert!(format!("{err:#}").contains("prefix length"), "{err:#}");

        let long_sep = "_".repeat(super::MAX_RESOURCE_PATH_BYTES + 1);
        write_json(
            &assets.join("atlases/sep_bomb.json"),
            &serde_json::to_string(&serde_json::json!({
                "sources": [{
                    "type": "minecraft:paletted_permutations",
                    "textures": ["minecraft:block/stone"],
                    "palette_key": "minecraft:palettes/key",
                    "permutations": { "a": "minecraft:palettes/p" },
                    "separator": long_sep
                }]
            }))
            .unwrap(),
        );
        let roots = PackRoots::from_root(&root).unwrap();
        let err = load_atlas_texture_entries(&roots, "sep_bomb").unwrap_err();
        assert!(format!("{err:#}").contains("separator length"), "{err:#}");

        // Multiple permutations sharing the same palette PNG decode it only once:
        // key (4B) + palette p (4B, cached!) + base stone (4B) + 3 outputs (12B) = 24B.
        write_json(
            &assets.join("atlases/cached_palettes.json"),
            &serde_json::to_string(&serde_json::json!({
                "sources": [{
                    "type": "minecraft:paletted_permutations",
                    "textures": ["minecraft:block/stone"],
                    "palette_key": "minecraft:palettes/key",
                    "permutations": {
                        "a": "minecraft:palettes/p",
                        "b": "minecraft:palettes/p",
                        "c": "minecraft:palettes/p"
                    }
                }]
            }))
            .unwrap(),
        );
        let base_roots = PackRoots::from_root(&root).unwrap();
        let selected = crate::SelectedClientPackStack::from_startup_dirs(&base_roots, []).unwrap();
        // Exact 24-byte image budget succeeds ONLY because `palettes/p` is cached instead of decoded 3 times (which would need 32B).
        let tight = base_roots
            .with_client_resource_generation_image_limit(1, selected, 24)
            .unwrap();
        let entries = tight.load_atlas_texture_images("cached_palettes").unwrap();
        assert_eq!(entries.len(), 4); // missingno + 3 permutations
        assert_eq!(
            tight.resource_image_memory_observation().charged_rgba_bytes,
            24
        );
        assert!(
            tight
                .resource_data_memory_observation()
                .derived_logical_bytes
                >= 3 * super::ATLAS_ENTRY_LOGICAL_BYTES
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    fn entry_id(entry: &AtlasTextureEntry) -> &str {
        match entry {
            AtlasTextureEntry::File { id, .. } => id,
            AtlasTextureEntry::Image(image) => &image.id,
        }
    }

    fn entry_path(entry: &AtlasTextureEntry) -> &std::path::Path {
        match entry {
            AtlasTextureEntry::File { resource, .. } => &resource.path,
            AtlasTextureEntry::Image(_) => panic!("expected file atlas texture entry"),
        }
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("bbb-client-assets-{label}-{nanos}"));
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

    fn local_vanilla_source(relative: &[&str]) -> Option<String> {
        let roots = PackRoots::from_environment().ok()?;
        let mut path = roots.sources_dir.clone();
        for segment in relative {
            path.push(segment);
        }
        path.is_file()
            .then(|| std::fs::read_to_string(path).ok())
            .flatten()
    }

    fn vanilla_bootstrap_ids(source: &str) -> BTreeSet<String> {
        Regex::new(r#"ID_MAPPER\.put\(\s*Identifier\.withDefaultNamespace\("([^"]+)"\)"#)
            .unwrap()
            .captures_iter(source)
            .map(|capture| format!("minecraft:{}", &capture[1]))
            .collect()
    }
}
