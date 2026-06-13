use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::PackRoots;

mod blockstates;
mod raw_model;
mod rotation;
mod shape;

use blockstates::RawBlockstate;
use raw_model::{RawBlockElement, RawBlockModel};
use rotation::{apply_variant_rotation, rotate_model_shape};
use shape::{classify_model_shape, combine_model_shapes};

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{BlockModelFace, BlockModelShape};
    use crate::{PackRoots, MC_VERSION};

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
