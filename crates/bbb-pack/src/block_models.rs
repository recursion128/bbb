use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::PackRoots;

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
