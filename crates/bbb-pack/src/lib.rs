use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use image::ImageReader;
use serde::{Deserialize, Serialize};

mod atlas;
mod colors;

pub use atlas::{AtlasImage, AtlasLayout, AtlasPacker, AtlasRect, AtlasSprite};
pub use colors::{
    BiomeColorCatalog, BiomeColorProfile, ColorMapImage, GrassColorModifier, TerrainColorMaps,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockModelFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl BlockModelFace {
    pub const ALL: [Self; 6] = [
        Self::Down,
        Self::Up,
        Self::North,
        Self::South,
        Self::West,
        Self::East,
    ];

    pub fn index(self) -> usize {
        match self {
            Self::Down => 0,
            Self::Up => 1,
            Self::North => 2,
            Self::South => 3,
            Self::West => 4,
            Self::East => 5,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Up => "up",
            Self::North => "north",
            Self::South => "south",
            Self::West => "west",
            Self::East => "east",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "down" => Some(Self::Down),
            "up" => Some(Self::Up),
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
        }
    }

    fn from_vector(vector: (i32, i32, i32)) -> Option<Self> {
        match vector {
            (0, -1, 0) => Some(Self::Down),
            (0, 1, 0) => Some(Self::Up),
            (0, 0, -1) => Some(Self::North),
            (0, 0, 1) => Some(Self::South),
            (-1, 0, 0) => Some(Self::West),
            (1, 0, 0) => Some(Self::East),
            _ => None,
        }
    }

    fn vector(self) -> (i32, i32, i32) {
        match self {
            Self::Down => (0, -1, 0),
            Self::Up => (0, 1, 0),
            Self::North => (0, 0, -1),
            Self::South => (0, 0, 1),
            Self::West => (-1, 0, 0),
            Self::East => (1, 0, 0),
        }
    }

    fn rotate(self, x_degrees: i32, y_degrees: i32) -> Self {
        let mut vector = self.vector();
        for _ in 0..quarter_turns(x_degrees) {
            vector = rotate_x_quarter(vector);
        }
        for _ in 0..quarter_turns(y_degrees) {
            vector = rotate_y_quarter(vector);
        }
        Self::from_vector(vector).unwrap_or(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockFaceTextures {
    pub textures: [String; 6],
    pub tint_indices: [Option<i32>; 6],
}

impl BlockFaceTextures {
    pub fn get(&self, face: BlockModelFace) -> &str {
        &self.textures[face.index()]
    }

    pub fn tint_index(&self, face: BlockModelFace) -> Option<i32> {
        self.tint_indices[face.index()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockModelShape {
    Cube,
    Cross,
    Box(BlockModelBox),
    Boxes(Vec<BlockModelBox>),
    Custom,
}

impl Default for BlockModelShape {
    fn default() -> Self {
        Self::Custom
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRenderModel {
    pub face_textures: BlockFaceTextures,
    pub shape: BlockModelShape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockModelBox {
    pub from: [u8; 3],
    pub to: [u8; 3],
    pub face_present: [bool; 6],
    pub face_uvs: [[u8; 4]; 6],
    pub face_cull: [bool; 6],
    pub face_tint_indices: [Option<i32>; 6],
    pub face_textures: [Option<String>; 6],
}

#[derive(Debug, Clone)]
pub struct BlockModelCatalog {
    blockstates: HashMap<String, RawBlockstate>,
    models: HashMap<String, RawBlockModel>,
}

impl BlockModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let mut blockstates = HashMap::new();
        let blockstates_dir = roots.blockstates_dir();
        for entry in std::fs::read_dir(&blockstates_dir)
            .with_context(|| format!("read blockstate directory {}", blockstates_dir.display()))?
        {
            let entry = entry.with_context(|| {
                format!("read blockstate entry in {}", blockstates_dir.display())
            })?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let id = format!("minecraft:{stem}");
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read blockstate {}", path.display()))?;
            let blockstate = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse blockstate {}", path.display()))?;
            blockstates.insert(id, blockstate);
        }

        let mut models = HashMap::new();
        let models_dir = roots.block_models_dir();
        for entry in std::fs::read_dir(&models_dir)
            .with_context(|| format!("read block model directory {}", models_dir.display()))?
        {
            let entry = entry
                .with_context(|| format!("read block model entry in {}", models_dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let id = format!("minecraft:block/{stem}");
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read block model {}", path.display()))?;
            let model = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse block model {}", path.display()))?;
            models.insert(id, model);
        }

        Ok(Self {
            blockstates,
            models,
        })
    }

    pub fn len(&self) -> usize {
        self.blockstates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blockstates.is_empty()
    }

    pub fn block_face_textures(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<BlockFaceTextures> {
        self.block_render_model(block_name, properties)
            .map(|model| model.face_textures)
    }

    pub fn block_render_model(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<BlockRenderModel> {
        let blockstate = self.blockstates.get(&normalize_block_id(block_name))?;
        let variants = blockstate.select_variants(properties)?;
        let mut face_textures = None;
        let mut shapes = Vec::with_capacity(variants.len());
        for variant in variants {
            let model =
                self.resolve_model(&normalize_model_id(&variant.model), &mut HashSet::new())?;
            let local = model.face_textures()?;
            face_textures
                .get_or_insert_with(|| apply_variant_rotation(local, variant.x, variant.y));
            shapes.push(rotate_model_shape(model.shape, variant.x, variant.y));
        }

        Some(BlockRenderModel {
            face_textures: face_textures?,
            shape: combine_model_shapes(shapes),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawBlockstate {
    #[serde(default)]
    variants: BTreeMap<String, RawBlockstateVariant>,
    #[serde(default)]
    multipart: Vec<RawMultipartCase>,
}

impl RawBlockstate {
    fn select_variants(
        &self,
        properties: &BTreeMap<String, String>,
    ) -> Option<Vec<RawModelVariant>> {
        let mut best_variant = None;
        let mut best_score = 0usize;
        for (key, variant) in &self.variants {
            let Some(score) = variant_key_match_score(key, properties) else {
                continue;
            };
            if best_variant.is_none() || score >= best_score {
                best_score = score;
                best_variant = variant.first_model();
            }
        }
        if best_variant.is_some() {
            return best_variant.map(|variant| vec![variant]);
        }

        let variants: Vec<_> = self
            .multipart
            .iter()
            .filter(|case| case.matches(properties))
            .filter_map(|case| case.apply.first_model())
            .collect();
        (!variants.is_empty()).then_some(variants)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawBlockstateVariant {
    One(RawModelVariant),
    Many(Vec<RawModelVariant>),
}

impl RawBlockstateVariant {
    fn first_model(&self) -> Option<RawModelVariant> {
        match self {
            Self::One(model) => Some(model.clone()),
            Self::Many(models) => models.first().cloned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawMultipartCase {
    #[serde(default)]
    when: Option<serde_json::Value>,
    apply: RawBlockstateVariant,
}

impl RawMultipartCase {
    fn matches(&self, properties: &BTreeMap<String, String>) -> bool {
        self.when
            .as_ref()
            .map(|when| multipart_condition_matches(when, properties))
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawModelVariant {
    model: String,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawBlockModel {
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    textures: BTreeMap<String, RawTextureReference>,
    #[serde(default)]
    elements: Vec<RawBlockElement>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawTextureReference {
    String(String),
    Object {
        #[serde(default)]
        sprite: Option<String>,
    },
}

impl RawTextureReference {
    fn texture_id(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            Self::Object { sprite } => sprite.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawBlockElement {
    #[serde(default)]
    from: Option<[f32; 3]>,
    #[serde(default)]
    to: Option<[f32; 3]>,
    #[serde(default)]
    faces: BTreeMap<String, RawBlockModelFace>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawBlockModelFace {
    texture: String,
    #[serde(default)]
    uv: Option<[f32; 4]>,
    #[serde(default)]
    cullface: Option<String>,
    #[serde(default)]
    tintindex: Option<i32>,
}

#[derive(Debug, Clone, Default)]
struct ResolvedBlockModel {
    textures: BTreeMap<String, String>,
    faces: [Option<ResolvedModelFace>; 6],
    shape: BlockModelShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedModelFace {
    texture: String,
    tint_index: Option<i32>,
}

impl ResolvedBlockModel {
    fn face_textures(&self) -> Option<BlockFaceTextures> {
        let resolved_faces: [Option<String>; 6] = std::array::from_fn(|index| {
            self.faces[index]
                .as_ref()
                .and_then(|face| resolve_texture_alias(&self.textures, &face.texture))
        });
        let fallback = self
            .textures
            .get("particle")
            .and_then(|texture| resolve_texture_alias(&self.textures, texture))
            .or_else(|| resolved_faces.iter().find_map(Clone::clone))?;
        Some(BlockFaceTextures {
            textures: std::array::from_fn(|index| {
                resolved_faces[index]
                    .clone()
                    .unwrap_or_else(|| fallback.clone())
            }),
            tint_indices: std::array::from_fn(|index| {
                self.faces[index].as_ref().and_then(|face| face.tint_index)
            }),
        })
    }
}

impl BlockModelCatalog {
    fn resolve_model(
        &self,
        model_id: &str,
        seen: &mut HashSet<String>,
    ) -> Option<ResolvedBlockModel> {
        let model_id = normalize_model_id(model_id);
        if !seen.insert(model_id.clone()) {
            return None;
        }
        let raw = self.models.get(&model_id)?;
        let mut resolved = raw
            .parent
            .as_deref()
            .and_then(|parent| self.resolve_model(parent, seen))
            .unwrap_or_default();

        for (key, value) in &raw.textures {
            if let Some(value) = value.texture_id() {
                resolved.textures.insert(key.clone(), value.to_string());
            }
        }

        if !raw.elements.is_empty() {
            resolved.faces = std::array::from_fn(|_| None);
            resolved.shape = classify_model_shape(&raw.elements, &resolved.textures);
            for element in &raw.elements {
                for (face_name, face) in &element.faces {
                    let Some(face_kind) = BlockModelFace::from_name(face_name) else {
                        continue;
                    };
                    let slot = &mut resolved.faces[face_kind.index()];
                    if slot.is_none() {
                        *slot = Some(ResolvedModelFace {
                            texture: face.texture.clone(),
                            tint_index: face.tintindex,
                        });
                    }
                }
            }
        }

        seen.remove(&model_id);
        Some(resolved)
    }
}

fn classify_model_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, String>,
) -> BlockModelShape {
    let mut face_counts = [0usize; 6];
    let mut total_faces = 0usize;
    for element in elements {
        for face_name in element.faces.keys() {
            let Some(face) = BlockModelFace::from_name(face_name) else {
                continue;
            };
            face_counts[face.index()] += 1;
            total_faces += 1;
        }
    }

    if elements.len() > 1 {
        if let Some(model_boxes) = multi_box_shape(elements, textures) {
            return BlockModelShape::Boxes(model_boxes);
        }
    }

    if elements.iter().any(is_full_cube_element) {
        return BlockModelShape::Cube;
    }

    let has_cross_faces = total_faces == 4
        && face_counts[BlockModelFace::North.index()] == 1
        && face_counts[BlockModelFace::South.index()] == 1
        && face_counts[BlockModelFace::West.index()] == 1
        && face_counts[BlockModelFace::East.index()] == 1
        && face_counts[BlockModelFace::Down.index()] == 0
        && face_counts[BlockModelFace::Up.index()] == 0;
    if has_cross_faces {
        return BlockModelShape::Cross;
    }

    if let Some(model_box) = single_box_shape(elements, textures) {
        return BlockModelShape::Box(model_box);
    }
    if let Some(model_boxes) = multi_box_shape(elements, textures) {
        return BlockModelShape::Boxes(model_boxes);
    }

    let has_cube_faces = face_counts.iter().all(|count| *count > 0);
    if has_cube_faces {
        return BlockModelShape::Cube;
    }

    BlockModelShape::Custom
}

fn is_full_cube_element(element: &RawBlockElement) -> bool {
    let Some(from) = element.from.and_then(quantize_vec3_0_16) else {
        return false;
    };
    let Some(to) = element.to.and_then(quantize_vec3_0_16) else {
        return false;
    };
    from == [0, 0, 0]
        && to == [16, 16, 16]
        && BlockModelFace::ALL
            .iter()
            .all(|face| element.faces.contains_key(face.name()))
}

fn single_box_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, String>,
) -> Option<BlockModelBox> {
    let [element] = elements else {
        return None;
    };
    element_box_shape(element, textures)
}

fn multi_box_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, String>,
) -> Option<Vec<BlockModelBox>> {
    if elements.len() <= 1 {
        return None;
    }
    elements
        .iter()
        .map(|element| element_box_shape(element, textures))
        .collect()
}

fn element_box_shape(
    element: &RawBlockElement,
    textures: &BTreeMap<String, String>,
) -> Option<BlockModelBox> {
    let from = element.from.and_then(quantize_vec3_0_16)?;
    let to = element.to.and_then(quantize_vec3_0_16)?;
    if from[0] >= to[0] || from[1] >= to[1] || from[2] >= to[2] {
        return None;
    }

    let mut face_present = [false; 6];
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    let mut face_cull = [false; 6];
    let mut face_tint_indices = [None; 6];
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    for (face_name, raw_face) in &element.faces {
        let face = BlockModelFace::from_name(face_name)?;
        face_present[face.index()] = true;
        face_uvs[face.index()] = raw_face
            .uv
            .and_then(quantize_uv_0_16)
            .unwrap_or([0, 0, 16, 16]);
        face_cull[face.index()] = raw_face
            .cullface
            .as_deref()
            .and_then(BlockModelFace::from_name)
            .is_some();
        face_tint_indices[face.index()] = raw_face.tintindex;
        face_textures[face.index()] = resolve_texture_alias(textures, &raw_face.texture);
    }

    Some(BlockModelBox {
        from,
        to,
        face_present,
        face_uvs,
        face_cull,
        face_tint_indices,
        face_textures,
    })
}

fn quantize_vec3_0_16(values: [f32; 3]) -> Option<[u8; 3]> {
    Some([
        quantize_0_16(values[0])?,
        quantize_0_16(values[1])?,
        quantize_0_16(values[2])?,
    ])
}

fn quantize_uv_0_16(values: [f32; 4]) -> Option<[u8; 4]> {
    Some([
        quantize_0_16(values[0])?,
        quantize_0_16(values[1])?,
        quantize_0_16(values[2])?,
        quantize_0_16(values[3])?,
    ])
}

fn quantize_0_16(value: f32) -> Option<u8> {
    if !(0.0..=16.0).contains(&value) {
        return None;
    }
    let rounded = value.round();
    if (value - rounded).abs() > f32::EPSILON {
        return None;
    }
    Some(rounded as u8)
}

fn apply_variant_rotation(
    local: BlockFaceTextures,
    x_degrees: i32,
    y_degrees: i32,
) -> BlockFaceTextures {
    let mut rotated: [Option<String>; 6] = std::array::from_fn(|_| None);
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        rotated[target.index()] = Some(local.textures[face.index()].clone());
    }
    BlockFaceTextures {
        textures: std::array::from_fn(|index| {
            rotated[index]
                .clone()
                .unwrap_or_else(|| local.textures[index].clone())
        }),
        tint_indices: rotate_face_values(local.tint_indices, x_degrees, y_degrees),
    }
}

fn rotate_model_shape(shape: BlockModelShape, x_degrees: i32, y_degrees: i32) -> BlockModelShape {
    match shape {
        BlockModelShape::Box(model_box) => {
            BlockModelShape::Box(rotate_model_box(model_box, x_degrees, y_degrees))
        }
        BlockModelShape::Boxes(model_boxes) => BlockModelShape::Boxes(
            model_boxes
                .into_iter()
                .map(|model_box| rotate_model_box(model_box, x_degrees, y_degrees))
                .collect(),
        ),
        BlockModelShape::Cube | BlockModelShape::Cross | BlockModelShape::Custom => shape,
    }
}

fn rotate_model_box(model_box: BlockModelBox, x_degrees: i32, y_degrees: i32) -> BlockModelBox {
    let mut min = [u8::MAX; 3];
    let mut max = [u8::MIN; 3];
    for x in [model_box.from[0], model_box.to[0]] {
        for y in [model_box.from[1], model_box.to[1]] {
            for z in [model_box.from[2], model_box.to[2]] {
                let rotated = rotate_model_point([x, y, z], x_degrees, y_degrees);
                for axis in 0..3 {
                    min[axis] = min[axis].min(rotated[axis]);
                    max[axis] = max[axis].max(rotated[axis]);
                }
            }
        }
    }

    let mut face_present = [false; 6];
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    let mut face_cull = [false; 6];
    let mut face_tint_indices = [None; 6];
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        face_present[target.index()] = model_box.face_present[face.index()];
        face_uvs[target.index()] = model_box.face_uvs[face.index()];
        face_cull[target.index()] = model_box.face_cull[face.index()];
        face_tint_indices[target.index()] = model_box.face_tint_indices[face.index()];
        face_textures[target.index()] = model_box.face_textures[face.index()].clone();
    }

    BlockModelBox {
        from: min,
        to: max,
        face_present,
        face_uvs,
        face_cull,
        face_tint_indices,
        face_textures,
    }
}

fn rotate_face_values<T: Copy>(values: [T; 6], x_degrees: i32, y_degrees: i32) -> [T; 6] {
    let mut rotated = values;
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        rotated[target.index()] = values[face.index()];
    }
    rotated
}

fn rotate_model_point(point: [u8; 3], x_degrees: i32, y_degrees: i32) -> [u8; 3] {
    let mut vector = (
        point[0] as i32 - 8,
        point[1] as i32 - 8,
        point[2] as i32 - 8,
    );
    for _ in 0..quarter_turns(x_degrees) {
        vector = rotate_x_quarter(vector);
    }
    for _ in 0..quarter_turns(y_degrees) {
        vector = rotate_y_quarter(vector);
    }
    [
        (vector.0 + 8).clamp(0, 16) as u8,
        (vector.1 + 8).clamp(0, 16) as u8,
        (vector.2 + 8).clamp(0, 16) as u8,
    ]
}

fn combine_model_shapes(shapes: Vec<BlockModelShape>) -> BlockModelShape {
    let mut shapes = shapes.into_iter();
    let Some(first) = shapes.next() else {
        return BlockModelShape::Custom;
    };
    let Some(second) = shapes.next() else {
        return first;
    };

    let mut boxes = Vec::new();
    for shape in std::iter::once(first)
        .chain(std::iter::once(second))
        .chain(shapes)
    {
        match shape {
            BlockModelShape::Box(model_box) => boxes.push(model_box),
            BlockModelShape::Boxes(model_boxes) => boxes.extend(model_boxes),
            BlockModelShape::Cube => return BlockModelShape::Cube,
            BlockModelShape::Cross | BlockModelShape::Custom => return BlockModelShape::Custom,
        }
    }

    match boxes.len() {
        0 => BlockModelShape::Custom,
        1 => BlockModelShape::Box(boxes[0].clone()),
        _ => BlockModelShape::Boxes(boxes),
    }
}

fn variant_key_match_score(key: &str, properties: &BTreeMap<String, String>) -> Option<usize> {
    if key.is_empty() {
        return Some(0);
    }

    let mut score = 0;
    for assignment in key.split(',') {
        let (name, value) = assignment.split_once('=')?;
        if properties.get(name)? != value {
            return None;
        }
        score += 1;
    }
    Some(score)
}

fn multipart_condition_matches(
    condition: &serde_json::Value,
    properties: &BTreeMap<String, String>,
) -> bool {
    let Some(object) = condition.as_object() else {
        return false;
    };

    for (key, value) in object {
        match key.as_str() {
            "OR" => {
                let Some(items) = value.as_array() else {
                    return false;
                };
                if !items
                    .iter()
                    .any(|item| multipart_condition_matches(item, properties))
                {
                    return false;
                }
            }
            "AND" => {
                let Some(items) = value.as_array() else {
                    return false;
                };
                if !items
                    .iter()
                    .all(|item| multipart_condition_matches(item, properties))
                {
                    return false;
                }
            }
            property => {
                let Some(actual) = properties.get(property) else {
                    return false;
                };
                if !condition_value_matches(value, actual) {
                    return false;
                }
            }
        }
    }

    true
}

fn condition_value_matches(expected: &serde_json::Value, actual: &str) -> bool {
    match expected {
        serde_json::Value::String(value) => value.split('|').any(|candidate| candidate == actual),
        serde_json::Value::Bool(value) => actual == value.to_string(),
        serde_json::Value::Number(value) => actual == value.to_string(),
        _ => false,
    }
}

fn resolve_texture_alias(textures: &BTreeMap<String, String>, texture: &str) -> Option<String> {
    let mut current = texture;
    let mut seen = HashSet::new();
    loop {
        let Some(alias) = current.strip_prefix('#') else {
            return Some(normalize_texture_id(current));
        };
        if !seen.insert(alias.to_string()) {
            return None;
        }
        current = textures.get(alias)?;
    }
}

fn normalize_block_id(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else {
        format!("minecraft:{id}")
    }
}

fn normalize_model_id(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else if id.contains('/') {
        format!("minecraft:{id}")
    } else {
        format!("minecraft:block/{id}")
    }
}

fn normalize_texture_id(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else if id.contains('/') {
        format!("minecraft:{id}")
    } else {
        format!("minecraft:block/{id}")
    }
}

fn quarter_turns(degrees: i32) -> usize {
    degrees.rem_euclid(360).div_euclid(90) as usize
}

fn rotate_x_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (x, -z, y)
}

fn rotate_y_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (z, y, -x)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteSource {
    pub id: String,
    pub width: u32,
    pub height: u32,
}

impl SpriteSource {
    pub fn new(id: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id: id.into(),
            width,
            height,
        }
    }

    pub fn from_png_file(id: impl Into<String>, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let reader =
            ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
        let reader = reader
            .with_guessed_format()
            .with_context(|| format!("guess image format {}", path.display()))?;
        let format = reader
            .format()
            .ok_or_else(|| anyhow::anyhow!("missing image format for {}", path.display()))?;
        if format != image::ImageFormat::Png {
            bail!("sprite source {} is not a PNG", path.display());
        }
        let (width, height) = reader
            .into_dimensions()
            .with_context(|| format!("read png dimensions {}", path.display()))?;
        Ok(Self::new(id, width, height))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteImage {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl SpriteImage {
    pub fn new(id: impl Into<String>, width: u32, height: u32, rgba: Vec<u8>) -> Result<Self> {
        let expected = rgba_len(width, height)?;
        if rgba.len() != expected {
            bail!(
                "sprite image has {} RGBA bytes, expected {} for {}x{}",
                rgba.len(),
                expected,
                width,
                height
            );
        }
        Ok(Self {
            id: id.into(),
            width,
            height,
            rgba,
        })
    }

    pub fn from_png_file(id: impl Into<String>, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let reader =
            ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
        let reader = reader
            .with_guessed_format()
            .with_context(|| format!("guess image format {}", path.display()))?;
        let format = reader
            .format()
            .ok_or_else(|| anyhow::anyhow!("missing image format for {}", path.display()))?;
        if format != image::ImageFormat::Png {
            bail!("sprite image {} is not a PNG", path.display());
        }
        let rgba = reader
            .decode()
            .with_context(|| format!("decode png {}", path.display()))?
            .into_rgba8();
        let (width, height) = rgba.dimensions();
        Self::new(id, width, height, rgba.into_raw())
    }

    fn source(&self) -> SpriteSource {
        SpriteSource::new(self.id.clone(), self.width, self.height)
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
