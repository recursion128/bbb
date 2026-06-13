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
                    "from": [0.8, 0, 8],
                    "to": [15.2, 16, 8],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "faces": {
                        "north": { "texture": "#cross" },
                        "south": { "texture": "#cross" }
                    }
                },
                {
                    "from": [8, 0, 0.8],
                    "to": [8, 16, 15.2],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
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

    assert_eq!(
        render_model.shape,
        BlockModelShape::Cross {
            shade: false,
            light_emission: 0,
        }
    );
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
fn block_model_catalog_preserves_emissive_cross_layers() {
    let root = unique_temp_dir("block-model-cross-emissive");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_flower.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/test_flower" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("test_flower.json"),
        r##"{
            "textures": {
                "cross": "minecraft:block/test_flower",
                "cross_emissive": "minecraft:block/test_flower_emissive"
            },
            "elements": [
                {
                    "from": [0.8, 0, 8],
                    "to": [15.2, 16, 8],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "faces": {
                        "north": { "texture": "#cross" },
                        "south": { "texture": "#cross" }
                    }
                },
                {
                    "from": [8, 0, 0.8],
                    "to": [8, 16, 15.2],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "faces": {
                        "west": { "texture": "#cross" },
                        "east": { "texture": "#cross" }
                    }
                },
                {
                    "from": [0.8, 0, 8],
                    "to": [15.2, 16, 8],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "light_emission": 15,
                    "faces": {
                        "north": { "texture": "#cross_emissive" },
                        "south": { "texture": "#cross_emissive" }
                    }
                },
                {
                    "from": [8, 0, 0.8],
                    "to": [8, 16, 15.2],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "light_emission": 15,
                    "faces": {
                        "west": { "texture": "#cross_emissive" },
                        "east": { "texture": "#cross_emissive" }
                    }
                }
            ]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:test_flower", &BTreeMap::new())
        .unwrap();
    let BlockModelShape::Crosses(crosses) = render_model.shape else {
        panic!("emissive cross should preserve base and emissive layers");
    };

    assert_eq!(crosses.len(), 2);
    assert!(!crosses[0].shade);
    assert_eq!(crosses[0].light_emission, 0);
    assert_eq!(
        crosses[0].face_textures[BlockModelFace::North.index()].as_deref(),
        Some("minecraft:block/test_flower")
    );
    assert!(!crosses[1].shade);
    assert_eq!(crosses[1].light_emission, 15);
    assert_eq!(
        crosses[1].face_textures[BlockModelFace::East.index()].as_deref(),
        Some("minecraft:block/test_flower_emissive")
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
                "shade": false,
                "faces": {
                    "down":  { "uv": [0, 0, 16, 16], "texture": "#bottom", "cullface": "down" },
                    "up":    { "uv": [0, 0, 16, 16], "rotation": 90, "texture": "#top" },
                    "north": { "uv": [0, 8, 16, 16], "texture": "#side", "cullface": "south" },
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
    assert_eq!(model_box.face_uv_rotations[BlockModelFace::Up.index()], 1);
    assert!(!model_box.face_shade[BlockModelFace::North.index()]);
    assert_eq!(
        model_box.face_cull[BlockModelFace::North.index()],
        Some(BlockModelFace::South)
    );
    assert_eq!(model_box.face_cull[BlockModelFace::Up.index()], None);
    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:block/oak_planks"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_uses_vanilla_default_face_uvs() {
    let root = unique_temp_dir("block-model-default-face-uvs");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("cobblestone_wall.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/cobblestone_wall_side" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("cobblestone_wall_side.json"),
        r##"{
            "textures": {
                "particle": "#wall",
                "wall": "minecraft:block/cobblestone"
            },
            "elements": [{
                "from": [5, 0, 0],
                "to": [11, 14, 8],
                "faces": {
                    "down":  { "texture": "#wall", "cullface": "down" },
                    "up":    { "texture": "#wall" },
                    "north": { "texture": "#wall", "cullface": "north" },
                    "west":  { "texture": "#wall" },
                    "east":  { "texture": "#wall" }
                }
            }]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:cobblestone_wall", &BTreeMap::new())
        .unwrap();
    let BlockModelShape::Box(model_box) = render_model.shape else {
        panic!("cobblestone_wall_side should resolve to a box model");
    };

    assert_eq!(
        model_box.face_uvs[BlockModelFace::Down.index()],
        [5, 8, 11, 16]
    );
    assert_eq!(
        model_box.face_uvs[BlockModelFace::Up.index()],
        [5, 0, 11, 8]
    );
    assert_eq!(
        model_box.face_uvs[BlockModelFace::North.index()],
        [5, 2, 11, 16]
    );
    assert_eq!(
        model_box.face_uvs[BlockModelFace::West.index()],
        [0, 2, 8, 16]
    );
    assert_eq!(
        model_box.face_uvs[BlockModelFace::East.index()],
        [8, 2, 16, 16]
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_keeps_rotated_elements_custom() {
    let root = unique_temp_dir("block-model-rotated-element");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("lever.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/lever" }
            }
        }"##,
    );
    write_json(
        &asset_root.join("models").join("block").join("lever.json"),
        r##"{
            "textures": { "texture": "minecraft:block/oak_planks" },
            "elements": [{
                "from": [7, 0, 7],
                "to": [9, 10, 9],
                "rotation": { "origin": [8, 1, 8], "axis": "x", "angle": 45 },
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

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:lever", &BTreeMap::new())
        .unwrap();

    assert_eq!(render_model.shape, BlockModelShape::Custom);
    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:block/oak_planks"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_preserves_full_cube_face_uv_rotation_as_box() {
    let root = unique_temp_dir("block-model-full-cube-face-rotation");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("oak_log.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/oak_log_horizontal" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("oak_log_horizontal.json"),
        r##"{
            "textures": {
                "end": "minecraft:block/oak_log_top",
                "side": "minecraft:block/oak_log"
            },
            "elements": [{
                "from": [0, 0, 0],
                "to": [16, 16, 16],
                "faces": {
                    "down":  { "texture": "#end", "cullface": "down" },
                    "up":    { "texture": "#end", "rotation": 180, "cullface": "up" },
                    "north": { "texture": "#side", "cullface": "north" },
                    "south": { "texture": "#side", "cullface": "south" },
                    "west":  { "texture": "#side", "cullface": "west" },
                    "east":  { "texture": "#side", "cullface": "east" }
                }
            }]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:oak_log", &BTreeMap::new())
        .unwrap();
    let BlockModelShape::Box(model_box) = render_model.shape else {
        panic!("full cube with face rotation should preserve box metadata");
    };

    assert_eq!(model_box.from, [0, 0, 0]);
    assert_eq!(model_box.to, [16, 16, 16]);
    assert_eq!(model_box.face_uv_rotations[BlockModelFace::Up.index()], 2);

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
    let BlockModelShape::Box(log_box) = &log_model.shape else {
        panic!("oak_log x-axis should preserve full-cube face UV rotation as a box");
    };
    assert_eq!(log_box.from, [0, 0, 0]);
    assert_eq!(log_box.to, [16, 16, 16]);
    assert_eq!(log_box.face_uv_rotations[BlockModelFace::East.index()], 2);
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
    assert_eq!(
        flower.shape,
        BlockModelShape::Cross {
            shade: false,
            light_emission: 0,
        }
    );
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
