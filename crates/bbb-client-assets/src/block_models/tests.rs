use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bbb_protocol::block_state_types::{vanilla_block_state_identities, VANILLA_BLOCK_STATE_COUNT};

use super::{
    BlockModelDisplayContext, BlockModelDisplayTransform, BlockModelFace, BlockModelGuiLight,
    BlockModelShape, MAX_BLOCKSTATE_DOCUMENTS, MAX_BLOCKSTATE_SELECTION_ENTRIES,
    MAX_CUBOID_MODEL_DOCUMENTS, MAX_CUBOID_MODEL_ELEMENTS, MAX_CUBOID_MODEL_FACES,
    MAX_CUBOID_MODEL_TEXTURE_SLOTS,
};
use crate::{PackResourceStack, PackRoots, MC_VERSION};

#[test]
fn model_semantic_counts_accept_exact_limits_and_reject_overflow() {
    let exact = PackResourceStack::default();
    for (count, maximum, label) in [
        (
            MAX_BLOCKSTATE_DOCUMENTS,
            MAX_BLOCKSTATE_DOCUMENTS,
            "blockstate documents",
        ),
        (
            MAX_BLOCKSTATE_SELECTION_ENTRIES,
            MAX_BLOCKSTATE_SELECTION_ENTRIES,
            "blockstate selection entries",
        ),
        (
            MAX_CUBOID_MODEL_DOCUMENTS,
            MAX_CUBOID_MODEL_DOCUMENTS,
            "model documents",
        ),
        (
            MAX_CUBOID_MODEL_ELEMENTS,
            MAX_CUBOID_MODEL_ELEMENTS,
            "model elements",
        ),
        (
            MAX_CUBOID_MODEL_FACES,
            MAX_CUBOID_MODEL_FACES,
            "model faces",
        ),
        (
            MAX_CUBOID_MODEL_TEXTURE_SLOTS,
            MAX_CUBOID_MODEL_TEXTURE_SLOTS,
            "model texture slots",
        ),
    ] {
        exact
            .admit_resource_collection_count(count, maximum, label)
            .unwrap();
    }

    for (maximum, label) in [
        (MAX_BLOCKSTATE_DOCUMENTS, "blockstate documents"),
        (
            MAX_BLOCKSTATE_SELECTION_ENTRIES,
            "blockstate selection entries",
        ),
        (MAX_CUBOID_MODEL_DOCUMENTS, "model documents"),
        (MAX_CUBOID_MODEL_ELEMENTS, "model elements"),
        (MAX_CUBOID_MODEL_FACES, "model faces"),
        (MAX_CUBOID_MODEL_TEXTURE_SLOTS, "model texture slots"),
    ] {
        let stack = PackResourceStack::default();
        let error = stack
            .admit_resource_collection_count(maximum + 1, maximum, label)
            .unwrap_err()
            .to_string();
        assert!(error.contains(label), "{error}");
        assert!(stack.ensure_resource_data_admitted().is_err());
    }
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
                "from": [0, 0, 0],
                "to": [16, 16, 16],
                "faces": {
                    "down": { "texture": "#down", "cullface": "down" },
                    "up": { "texture": "#up", "tintindex": 0, "cullface": "up" },
                    "north": { "texture": "#north", "tintindex": 0, "cullface": "north" },
                    "south": { "texture": "#south", "tintindex": 0, "cullface": "south" },
                    "west": { "texture": "#west", "tintindex": 0, "cullface": "west" },
                    "east": { "texture": "#east", "tintindex": 0, "cullface": "east" }
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
    assert!(render_model.use_ambient_occlusion);
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
    assert!(!textures.force_translucent(BlockModelFace::Up));

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
    assert!(snowy.face_textures.force_translucent(BlockModelFace::Up));
    assert!(!snowy.face_textures.force_translucent(BlockModelFace::North));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_uses_resource_pack_precedence_for_blockstates_and_models() {
    let root = unique_temp_dir("block-model-resource-pack");
    let base_assets = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    let pack = root.join("resource_pack");
    let pack_assets = pack.join("assets").join("minecraft");

    write_json(
        &base_assets
            .join("blockstates")
            .join("model_override_block.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/shared_model" }
            }
        }"##,
    );
    write_json(
        &base_assets
            .join("blockstates")
            .join("blockstate_override_block.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/base_model" }
            }
        }"##,
    );
    write_single_texture_model(
        &base_assets
            .join("models")
            .join("block")
            .join("shared_model.json"),
        "minecraft:block/base_model_texture",
    );
    write_single_texture_model(
        &base_assets
            .join("models")
            .join("block")
            .join("base_model.json"),
        "minecraft:block/base_blockstate_texture",
    );

    write_json(
        &pack_assets
            .join("blockstates")
            .join("blockstate_override_block.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/pack_only_model" }
            }
        }"##,
    );
    write_single_texture_model(
        &pack_assets
            .join("models")
            .join("block")
            .join("shared_model.json"),
        "minecraft:block/overlay_model_texture",
    );
    write_single_texture_model(
        &pack_assets
            .join("models")
            .join("block")
            .join("pack_only_model.json"),
        "minecraft:block/overlay_blockstate_texture",
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .with_resource_pack_dirs([pack])
        .load_block_model_catalog()
        .unwrap();
    let model_override = catalog
        .block_render_model("minecraft:model_override_block", &BTreeMap::new())
        .unwrap();
    let blockstate_override = catalog
        .block_render_model("minecraft:blockstate_override_block", &BTreeMap::new())
        .unwrap();

    assert_eq!(
        model_override.face_textures.get(BlockModelFace::North),
        "minecraft:block/overlay_model_texture"
    );
    assert_eq!(
        blockstate_override.face_textures.get(BlockModelFace::North),
        "minecraft:block/overlay_blockstate_texture"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_merges_blockstate_resource_stacks_by_matching_state() {
    let root = unique_temp_dir("block-model-blockstate-stack");
    let base_assets = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    let pack = root.join("resource_pack");
    let pack_assets = pack.join("assets").join("minecraft");

    write_json(
        &base_assets.join("blockstates").join("partial_stack.json"),
        r##"{
            "variants": {
                "facing=north": { "model": "minecraft:block/base_north" },
                "facing=south": { "model": "minecraft:block/base_south" }
            }
        }"##,
    );
    write_json(
        &pack_assets.join("blockstates").join("partial_stack.json"),
        r##"{
            "variants": {
                "facing=north": { "model": "minecraft:block/overlay_north" }
            }
        }"##,
    );
    write_single_texture_model(
        &base_assets
            .join("models")
            .join("block")
            .join("base_north.json"),
        "minecraft:block/base_north_texture",
    );
    write_single_texture_model(
        &base_assets
            .join("models")
            .join("block")
            .join("base_south.json"),
        "minecraft:block/base_south_texture",
    );
    write_single_texture_model(
        &pack_assets
            .join("models")
            .join("block")
            .join("overlay_north.json"),
        "minecraft:block/overlay_north_texture",
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .with_resource_pack_dirs([pack])
        .load_block_model_catalog()
        .unwrap();
    let mut properties = BTreeMap::new();

    properties.insert("facing".to_string(), "north".to_string());
    let north = catalog
        .block_render_model("minecraft:partial_stack", &properties)
        .unwrap();
    assert_eq!(
        north.face_textures.get(BlockModelFace::North),
        "minecraft:block/overlay_north_texture"
    );

    properties.insert("facing".to_string(), "south".to_string());
    let south = catalog
        .block_render_model("minecraft:partial_stack", &properties)
        .unwrap();
    assert_eq!(
        south.face_textures.get(BlockModelFace::North),
        "minecraft:block/base_south_texture"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_resolves_unprefixed_face_texture_slots() {
    let root = unique_temp_dir("block-model-unprefixed-face-texture");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("heavy_core.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/heavy_core" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("heavy_core.json"),
        r##"{
            "textures": {
                "all": "block/heavy_core",
                "particle": "block/heavy_core"
            },
            "elements": [{
                "from": [4, 0, 4],
                "to": [12, 8, 12],
                "faces": {
                    "down":  { "uv": [8, 0, 16, 8], "texture": "all" },
                    "up":    { "uv": [0, 0, 8, 8], "texture": "all" },
                    "north": { "uv": [0, 8, 8, 16], "texture": "all" },
                    "south": { "uv": [0, 8, 8, 16], "texture": "all" },
                    "west":  { "uv": [0, 8, 8, 16], "texture": "all" },
                    "east":  { "uv": [0, 8, 8, 16], "texture": "all" }
                }
            }]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:heavy_core", &BTreeMap::new())
        .unwrap();
    let BlockModelShape::Box(model_box) = render_model.shape else {
        panic!("heavy_core should resolve to a box model");
    };

    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:block/heavy_core"
    );
    assert_eq!(
        model_box.face_textures[BlockModelFace::North.index()].as_deref(),
        Some("minecraft:block/heavy_core")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_uses_vanilla_default_namespace_for_direct_texture_ids() {
    let root = unique_temp_dir("block-model-direct-texture-default-namespace");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/test_block" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("test_block.json"),
        r##"{
            "textures": {
                "all": "plain_texture",
                "glass": {
                    "sprite": "glass_texture",
                    "force_translucent": true
                }
            },
            "elements": [{
                "faces": {
                    "north": { "texture": "#all" },
                    "east": { "texture": "#glass" }
                }
            }]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:test_block", &BTreeMap::new())
        .unwrap();

    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:plain_texture"
    );
    assert_eq!(
        render_model.face_textures.get(BlockModelFace::East),
        "minecraft:glass_texture"
    );
    assert!(render_model
        .face_textures
        .force_translucent(BlockModelFace::East));
    assert!(!render_model
        .face_textures
        .force_translucent(BlockModelFace::North));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_uses_vanilla_default_namespace_for_model_ids() {
    let root = unique_temp_dir("block-model-default-namespace");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "": { "model": "root_child" }
            }
        }"##,
    );
    write_json(
        &asset_root.join("models").join("root_child.json"),
        r##"{
            "parent": "root_parent",
            "textures": { "all": "child_texture" }
        }"##,
    );
    write_json(
        &asset_root.join("models").join("root_parent.json"),
        r##"{
            "textures": { "all": "parent_texture" },
            "elements": [{
                "faces": {
                    "north": { "texture": "#all" }
                }
            }]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:test_block", &BTreeMap::new())
        .unwrap();

    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:child_texture"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_resolves_parent_ambient_occlusion() {
    let root = unique_temp_dir("block-model-ambient-occlusion");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "kind=inherit": { "model": "minecraft:block/child_inherit" },
                "kind=override": { "model": "minecraft:block/child_override" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("parent_model.json"),
        r##"{
            "ambientocclusion": false,
            "textures": { "particle": "#all" },
            "elements": [{
                "faces": {
                    "down": { "texture": "#all" },
                    "up": { "texture": "#all" },
                    "north": { "texture": "#all" },
                    "south": { "texture": "#all" },
                    "west": { "texture": "#all" },
                    "east": { "texture": "#all" }
                }
            }]
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("child_inherit.json"),
        r##"{
            "parent": "minecraft:block/parent_model",
            "textures": { "all": "minecraft:block/oak_planks" }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("child_override.json"),
        r##"{
            "parent": "minecraft:block/parent_model",
            "ambientocclusion": true,
            "textures": { "all": "minecraft:block/stone" }
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let mut properties = BTreeMap::new();
    properties.insert("kind".to_string(), "inherit".to_string());
    let inherited = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();
    properties.insert("kind".to_string(), "override".to_string());
    let overridden = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();

    assert!(!inherited.use_ambient_occlusion);
    assert!(overridden.use_ambient_occlusion);
    assert_eq!(
        inherited.face_textures.get(BlockModelFace::North),
        "minecraft:block/oak_planks"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_resolves_parent_gui_light() {
    let root = unique_temp_dir("block-model-gui-light");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "kind=inherit": { "model": "minecraft:block/child_inherit" },
                "kind=override": { "model": "minecraft:block/child_override" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("parent_model.json"),
        r##"{
            "gui_light": "front",
            "textures": { "particle": "#all" },
            "elements": [{
                "faces": {
                    "down": { "texture": "#all" },
                    "up": { "texture": "#all" },
                    "north": { "texture": "#all" },
                    "south": { "texture": "#all" },
                    "west": { "texture": "#all" },
                    "east": { "texture": "#all" }
                }
            }]
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("child_inherit.json"),
        r##"{
            "parent": "minecraft:block/parent_model",
            "textures": { "all": "minecraft:block/oak_planks" }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("child_override.json"),
        r##"{
            "parent": "minecraft:block/parent_model",
            "gui_light": "side",
            "textures": { "all": "minecraft:block/stone" }
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let inherited_resolved = catalog
        .resolve_model("minecraft:block/child_inherit")
        .unwrap();
    let overridden_resolved = catalog
        .resolve_model("minecraft:block/child_override")
        .unwrap();
    assert_eq!(inherited_resolved.gui_light(), BlockModelGuiLight::Front);
    assert_eq!(overridden_resolved.gui_light(), BlockModelGuiLight::Side);

    let mut properties = BTreeMap::new();
    properties.insert("kind".to_string(), "inherit".to_string());
    let inherited = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();
    properties.insert("kind".to_string(), "override".to_string());
    let overridden = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();

    assert_eq!(inherited.gui_light, BlockModelGuiLight::Front);
    assert_eq!(overridden.gui_light, BlockModelGuiLight::Side);
    assert!(!inherited.gui_light.light_like_block());
    assert!(overridden.gui_light.light_like_block());
    assert_eq!(
        inherited.face_textures.get(BlockModelFace::North),
        "minecraft:block/oak_planks"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_rejects_invalid_gui_light() {
    let root = unique_temp_dir("block-model-invalid-gui-light");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("invalid_model.json"),
        r##"{
            "gui_light": "diagonal"
        }"##,
    );

    let error = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap_err();
    let message = format!("{error:#}");

    assert!(message.contains("parse block model"));
    assert!(message.contains("diagonal"));
    assert!(message.contains("front"));
    assert!(message.contains("side"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_resolves_parent_display_transforms() {
    let root = unique_temp_dir("block-model-display-transforms");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "kind=inherit": { "model": "minecraft:block/child_inherit" },
                "kind=override": { "model": "minecraft:block/child_override" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("parent_model.json"),
        r##"{
            "display": {
                "thirdperson_righthand": {
                    "rotation": [ 10, 20, 30 ],
                    "translation": [ 16, -160, 96 ],
                    "scale": [ 5, -5, 0.5 ]
                },
                "ground": {
                    "translation": [ 0, 2, 0 ],
                    "scale": [ 0.5, 0.5, 0.5 ]
                }
            },
            "textures": { "particle": "#all" },
            "elements": [{
                "faces": {
                    "down": { "texture": "#all" },
                    "up": { "texture": "#all" },
                    "north": { "texture": "#all" },
                    "south": { "texture": "#all" },
                    "west": { "texture": "#all" },
                    "east": { "texture": "#all" }
                }
            }]
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("child_inherit.json"),
        r##"{
            "parent": "minecraft:block/parent_model",
            "textures": { "all": "minecraft:block/oak_planks" }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("child_override.json"),
        r##"{
            "parent": "minecraft:block/parent_model",
            "display": {
                "thirdperson_righthand": {
                    "translation": [ 0, 32, 0 ]
                },
                "gui": {
                    "rotation": [ 0, 180, 0 ],
                    "scale": [ 1.25, 1.25, 1.25 ]
                }
            },
            "textures": { "all": "minecraft:block/stone" }
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let inherited_resolved = catalog
        .resolve_model("minecraft:block/child_inherit")
        .unwrap();
    let overridden_resolved = catalog
        .resolve_model("minecraft:block/child_override")
        .unwrap();

    let inherited = inherited_resolved.display_transforms();
    let expected_parent_hand = BlockModelDisplayTransform {
        rotation: [10.0, 20.0, 30.0],
        translation: [1.0, -5.0, 5.0],
        scale: [4.0, -4.0, 0.5],
    };
    assert_eq!(
        inherited.get(BlockModelDisplayContext::ThirdPersonRightHand),
        expected_parent_hand
    );
    assert_eq!(
        inherited.get(BlockModelDisplayContext::ThirdPersonLeftHand),
        expected_parent_hand
    );
    assert_eq!(
        inherited.get(BlockModelDisplayContext::Ground),
        BlockModelDisplayTransform {
            rotation: [0.0; 3],
            translation: [0.0, 0.125, 0.0],
            scale: [0.5, 0.5, 0.5],
        }
    );
    assert_eq!(
        inherited.get(BlockModelDisplayContext::Gui),
        BlockModelDisplayTransform::default()
    );

    let overridden = overridden_resolved.display_transforms();
    let expected_child_hand = BlockModelDisplayTransform {
        rotation: [0.0; 3],
        translation: [0.0, 2.0, 0.0],
        scale: [1.0; 3],
    };
    assert_eq!(
        overridden.get(BlockModelDisplayContext::ThirdPersonRightHand),
        expected_child_hand
    );
    assert_eq!(
        overridden.get(BlockModelDisplayContext::ThirdPersonLeftHand),
        expected_child_hand
    );
    assert_eq!(
        overridden.get(BlockModelDisplayContext::Ground),
        inherited.get(BlockModelDisplayContext::Ground)
    );
    assert_eq!(
        overridden.get(BlockModelDisplayContext::Gui),
        BlockModelDisplayTransform {
            rotation: [0.0, 180.0, 0.0],
            translation: [0.0; 3],
            scale: [1.25, 1.25, 1.25],
        }
    );

    let mut properties = BTreeMap::new();
    properties.insert("kind".to_string(), "override".to_string());
    let render_model = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();
    assert_eq!(render_model.display_transforms, overridden);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_rejects_invalid_display_transform_vectors() {
    let root = unique_temp_dir("block-model-invalid-display");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("invalid_model.json"),
        r##"{
            "display": {
                "gui": {
                    "translation": [ 0, 1 ]
                }
            }
        }"##,
    );

    let error = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap_err();
    let message = format!("{error:#}");

    assert!(message.contains("parse block model"));
    assert!(message.contains("invalid length 2"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_selects_weighted_variants_from_seed() {
    let root = unique_temp_dir("block-model-weighted-variants");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "": [
                    { "model": "minecraft:block/red_model", "weight": 2 },
                    { "model": "minecraft:block/blue_model" }
                ]
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("cube_all.json"),
        r##"{
            "textures": { "particle": "#all" },
            "elements": [{
                "faces": {
                    "down": { "texture": "#all" },
                    "up": { "texture": "#all" },
                    "north": { "texture": "#all" },
                    "south": { "texture": "#all" },
                    "west": { "texture": "#all" },
                    "east": { "texture": "#all" }
                }
            }]
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("red_model.json"),
        r##"{
            "parent": "minecraft:block/cube_all",
            "textures": { "all": "minecraft:block/red" }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("blue_model.json"),
        r##"{
            "parent": "minecraft:block/cube_all",
            "textures": { "all": "minecraft:block/blue" }
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let unseeded = catalog
        .block_render_model("minecraft:test_block", &BTreeMap::new())
        .unwrap();
    let red = catalog
        .block_render_model_with_seed("minecraft:test_block", &BTreeMap::new(), Some(0))
        .unwrap();
    let blue = catalog
        .block_render_model_with_seed("minecraft:test_block", &BTreeMap::new(), Some(3))
        .unwrap();

    assert_eq!(
        unseeded.face_textures.get(BlockModelFace::North),
        "minecraft:block/red"
    );
    assert_eq!(
        red.face_textures.get(BlockModelFace::North),
        "minecraft:block/red"
    );
    assert_eq!(
        blue.face_textures.get(BlockModelFace::North),
        "minecraft:block/blue"
    );
    let material_models = catalog
        .block_material_models("minecraft:test_block", &BTreeMap::new())
        .unwrap();
    assert_eq!(material_models.len(), 2);
    assert_eq!(
        material_models[0].face_textures.get(BlockModelFace::North),
        "minecraft:block/red"
    );
    assert_eq!(
        material_models[1].face_textures.get(BlockModelFace::North),
        "minecraft:block/blue"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_material_flags_include_inactive_multipart_selectors() {
    let root = unique_temp_dir("block-model-multipart-material-flags");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "multipart": [
                { "when": { "lit": "false" }, "apply": { "model": "minecraft:block/solid" } },
                { "when": { "lit": "true" }, "apply": [
                    { "model": "minecraft:block/glow_a" },
                    { "model": "minecraft:block/glow_b" }
                ] }
            ]
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("cube_all.json"),
        r##"{
            "textures": { "particle": "#all" },
            "elements": [{
                "faces": {
                    "down": { "texture": "#all" }, "up": { "texture": "#all" },
                    "north": { "texture": "#all" }, "south": { "texture": "#all" },
                    "west": { "texture": "#all" }, "east": { "texture": "#all" }
                }
            }]
        }"##,
    );
    for (model, texture) in [
        ("solid", "stone"),
        ("glow_a", "glow_a"),
        ("glow_b", "glow_b"),
    ] {
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join(format!("{model}.json")),
            &format!(
                r##"{{
                    "parent": "minecraft:block/cube_all",
                    "textures": {{ "all": "minecraft:block/{texture}" }}
                }}"##
            ),
        );
    }

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let properties = BTreeMap::from([("lit".to_string(), "false".to_string())]);
    let rendered = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();
    assert_eq!(
        rendered.face_textures.get(BlockModelFace::North),
        "minecraft:block/stone"
    );
    let material_models = catalog
        .block_material_models("minecraft:test_block", &properties)
        .unwrap();
    assert_eq!(material_models.len(), 3);
    assert_eq!(
        material_models
            .iter()
            .map(|model| model.face_textures.get(BlockModelFace::North))
            .collect::<Vec<_>>(),
        [
            "minecraft:block/stone",
            "minecraft:block/glow_a",
            "minecraft:block/glow_b",
        ]
    );

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
                        "north": { "uv": [0, 0, 16, 16], "texture": "#cross" },
                        "south": { "uv": [0, 0, 16, 16], "texture": "#cross" }
                    }
                },
                {
                    "from": [8, 0, 0.8],
                    "to": [8, 16, 15.2],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "faces": {
                        "west": { "uv": [0, 0, 16, 16], "texture": "#cross" },
                        "east": { "uv": [0, 0, 16, 16], "texture": "#cross" }
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
                "cross_emissive": {
                    "sprite": "minecraft:block/test_flower_emissive",
                    "force_translucent": true
                }
            },
            "elements": [
                {
                    "from": [0.8, 0, 8],
                    "to": [15.2, 16, 8],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "faces": {
                        "north": { "uv": [0, 0, 16, 16], "texture": "#cross" },
                        "south": { "uv": [0, 0, 16, 16], "texture": "#cross" }
                    }
                },
                {
                    "from": [8, 0, 0.8],
                    "to": [8, 16, 15.2],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "faces": {
                        "west": { "uv": [0, 0, 16, 16], "texture": "#cross" },
                        "east": { "uv": [0, 0, 16, 16], "texture": "#cross" }
                    }
                },
                {
                    "from": [0.8, 0, 8],
                    "to": [15.2, 16, 8],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "light_emission": 15,
                    "faces": {
                        "north": { "uv": [0, 0, 16, 16], "texture": "#cross_emissive" },
                        "south": { "uv": [0, 0, 16, 16], "texture": "#cross_emissive" }
                    }
                },
                {
                    "from": [8, 0, 0.8],
                    "to": [8, 16, 15.2],
                    "rotation": { "origin": [8, 8, 8], "axis": "y", "angle": 45, "rescale": true },
                    "shade": false,
                    "light_emission": 15,
                    "faces": {
                        "west": { "uv": [0, 0, 16, 16], "texture": "#cross_emissive" },
                        "east": { "uv": [0, 0, 16, 16], "texture": "#cross_emissive" }
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
    assert!(crosses[1].face_force_translucent[BlockModelFace::East.index()]);
    assert!(!crosses[0].face_force_translucent[BlockModelFace::North.index()]);

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

    assert_eq!(render_model.shape, BlockModelShape::Boxes(Vec::new()));
    assert_eq!(
        render_model.particle_texture.as_deref(),
        Some("minecraft:block/water_still")
    );
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
                "top": {
                    "sprite": "minecraft:block/oak_planks",
                    "force_translucent": true
                }
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
    assert!(model_box.face_force_translucent[BlockModelFace::Up.index()]);
    assert!(!model_box.face_force_translucent[BlockModelFace::North.index()]);
    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:block/oak_planks"
    );
    assert!(render_model
        .face_textures
        .force_translucent(BlockModelFace::Up));

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
fn block_model_catalog_bakes_rotated_elements_to_quads() {
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

    let BlockModelShape::Quads(quads) = render_model.shape else {
        panic!("rotated lever element should bake to quads");
    };
    assert_eq!(quads.len(), 6);
    let north = quads
        .iter()
        .find(|quad| quad.face == BlockModelFace::North)
        .expect("north face quad exists");
    // A rotated element keeps `FaceInfo.NORTH` vertex order (`FaceBakery` skips
    // `recalculateWinding`), so vertex 0 is the rotated (max x, max y, min z) corner.
    for (corner, expected) in north.corners.iter().zip([
        [9.0, 8.071068, 13.656855],
        [9.0, 1.0, 6.5857863],
        [7.0, 1.0, 6.5857863],
        [7.0, 8.071068, 13.656855],
    ]) {
        assert_close(*corner, expected);
    }
    assert_close(north.normal, [0.0, 0.70710677, -0.70710677]);
    assert_eq!(north.texture.as_deref(), Some("minecraft:block/oak_planks"));
    assert_eq!(north.uvs[0], [7.0 / 16.0, 6.0 / 16.0]);
    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:block/oak_planks"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_bakes_flat_elements_to_quads() {
    let root = unique_temp_dir("block-model-flat-element");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("ladder.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/ladder" }
            }
        }"##,
    );
    write_json(
        &asset_root.join("models").join("block").join("ladder.json"),
        r##"{
            "textures": { "texture": "minecraft:block/ladder" },
            "elements": [{
                "from": [0, 0, 15.2],
                "to": [16, 16, 15.2],
                "faces": {
                    "north": { "uv": [0, 0, 16, 16], "texture": "#texture" },
                    "south": { "uv": [16, 0, 0, 16], "texture": "#texture" }
                }
            }]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let render_model = catalog
        .block_render_model("minecraft:ladder", &BTreeMap::new())
        .unwrap();

    let BlockModelShape::Quads(quads) = render_model.shape else {
        panic!("a flat ladder element should bake to quads, not fall back to a cube");
    };
    assert_eq!(quads.len(), 2);
    assert!(quads
        .iter()
        .all(|quad| quad.corners.iter().all(|corner| corner[2] == 15.2)));

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
fn block_model_catalog_applies_variant_uvlock_to_box_face_uvs() {
    let root = unique_temp_dir("block-model-uvlock");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "locked=false": { "model": "minecraft:block/test_model", "y": 90 },
                "locked=true": { "model": "minecraft:block/test_model", "y": 90, "uvlock": true }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("test_model.json"),
        r##"{
            "textures": {
                "particle": "minecraft:block/oak_planks",
                "top": "minecraft:block/oak_planks"
            },
            "elements": [{
                "from": [0, 0, 0],
                "to": [16, 16, 16],
                "faces": {
                    "up": { "uv": [0, 0, 8, 16], "texture": "#top" }
                }
            }]
        }"##,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let mut properties = BTreeMap::new();
    properties.insert("locked".to_string(), "false".to_string());
    let unlocked = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();
    properties.insert("locked".to_string(), "true".to_string());
    let locked = catalog
        .block_render_model("minecraft:test_block", &properties)
        .unwrap();

    let BlockModelShape::Box(unlocked_box) = unlocked.shape else {
        panic!("unlocked model should stay a box");
    };
    let BlockModelShape::Box(locked_box) = locked.shape else {
        panic!("uvlocked model should stay a box");
    };
    let up = BlockModelFace::Up.index();
    // FaceBakery: without uvlock the crop turns with the face; with uvlock the crop is mapped
    // back through `inverseFaceTransformation` and stays world-aligned.
    assert_eq!(unlocked_box.face_uvs[up], [0, 0, 8, 16]);
    assert_eq!(unlocked_box.face_uv_rotations[up], 1);
    assert_eq!(locked_box.face_uvs[up], [0, 0, 16, 8]);
    assert_eq!(locked_box.face_uv_rotations[up], 0);

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
    assert_eq!(boxes[2].from, [8, 6, 7]);
    assert!(!boxes[1].face_present[BlockModelFace::Down.index()]);
    assert_eq!(
        render_model.face_textures.get(BlockModelFace::North),
        "minecraft:block/oak_planks"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_preserves_empty_multipart_selection() {
    let root = unique_temp_dir("block-model-empty-multipart");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("cobblestone_wall.json"),
        r##"{
            "multipart": [
                {
                    "when": { "up": "true" },
                    "apply": { "model": "minecraft:block/template_wall_post" }
                },
                {
                    "when": { "north": "low" },
                    "apply": { "model": "minecraft:block/template_wall_side" }
                }
            ]
        }"##,
    );
    std::fs::create_dir_all(asset_root.join("models").join("block")).unwrap();

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let mut properties = BTreeMap::new();
    properties.insert("north".to_string(), "none".to_string());
    properties.insert("up".to_string(), "false".to_string());
    let render_model = catalog
        .block_render_model("minecraft:cobblestone_wall", &properties)
        .unwrap();
    let BlockModelShape::Boxes(boxes) = render_model.shape else {
        panic!("empty multipart selection should resolve to empty box geometry");
    };

    assert!(boxes.is_empty());

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
                "axis=z": { "model": "minecraft:block/oak_log", "x": 90 },
                "axis=z_rot": { "model": "minecraft:block/oak_log", "z": 90 }
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

    properties.insert("axis".to_string(), "z_rot".to_string());
    let z_rotated = catalog
        .block_face_textures("minecraft:oak_log", &properties)
        .unwrap();
    assert_eq!(
        z_rotated.get(BlockModelFace::East),
        "minecraft:block/oak_log_top"
    );
    assert_eq!(
        z_rotated.get(BlockModelFace::West),
        "minecraft:block/oak_log_top"
    );
    assert_eq!(
        z_rotated.get(BlockModelFace::North),
        "minecraft:block/oak_log"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 sources"]
fn loads_local_vanilla_block_model_catalog() {
    let roots = PackRoots::from_environment().unwrap();
    let catalog = roots.load_block_model_catalog().unwrap();
    assert!(catalog.len() > 1_000);

    let base_block_transforms = catalog
        .resolve_model("minecraft:block/block")
        .unwrap()
        .display_transforms();
    assert_eq!(
        base_block_transforms.get(BlockModelDisplayContext::Gui),
        BlockModelDisplayTransform {
            rotation: [30.0, 225.0, 0.0],
            translation: [0.0, 0.0, 0.0],
            scale: [0.625, 0.625, 0.625],
        }
    );
    assert_eq!(
        base_block_transforms.get(BlockModelDisplayContext::Ground),
        BlockModelDisplayTransform {
            rotation: [0.0, 0.0, 0.0],
            translation: [0.0, 0.1875, 0.0],
            scale: [0.25, 0.25, 0.25],
        }
    );
    assert_eq!(
        base_block_transforms.get(BlockModelDisplayContext::OnShelf),
        BlockModelDisplayTransform {
            rotation: [0.0, 180.0, 0.0],
            translation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    );
    assert_eq!(
        base_block_transforms.get(BlockModelDisplayContext::ThirdPersonLeftHand),
        base_block_transforms.get(BlockModelDisplayContext::ThirdPersonRightHand)
    );

    let orientable_transforms = catalog
        .resolve_model("minecraft:block/orientable_with_bottom")
        .unwrap()
        .display_transforms();
    assert_eq!(
        orientable_transforms.get(BlockModelDisplayContext::FirstPersonRightHand),
        BlockModelDisplayTransform {
            rotation: [0.0, 135.0, 0.0],
            translation: [0.0, 0.0, 0.0],
            scale: [0.4, 0.4, 0.4],
        }
    );
    assert_eq!(
        orientable_transforms.get(BlockModelDisplayContext::FirstPersonLeftHand),
        orientable_transforms.get(BlockModelDisplayContext::FirstPersonRightHand)
    );
    assert_eq!(
        orientable_transforms.get(BlockModelDisplayContext::Ground),
        base_block_transforms.get(BlockModelDisplayContext::Ground)
    );

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
    // `cube_column_horizontal` turns its up face 180 degrees so that `x: 90, y: 90` leaves the
    // log end upright on the east face.
    assert_eq!(log_box.face_uv_rotations[BlockModelFace::East.index()], 0);
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
    assert_eq!(fence_boxes[3].from, [7, 12, 7]);
    assert_eq!(fence_boxes[4].from, [7, 6, 7]);
    // The east arm (`y: 90, uvlock`) of `fence_side`'s top bar: FaceBakery maps the up crop
    // [7, 0, 9, 9] to world-aligned [7, 7, 16, 9] and the north end to the east face.
    let east_bar = &fence_boxes[3];
    assert_eq!(east_bar.to, [16, 15, 9]);
    assert_eq!(east_bar.face_uvs[BlockModelFace::Up.index()], [7, 7, 16, 9]);
    assert_eq!(east_bar.face_uv_rotations[BlockModelFace::Up.index()], 0);
    assert_eq!(
        east_bar.face_cull[BlockModelFace::East.index()],
        Some(BlockModelFace::East)
    );

    let mut wall = BTreeMap::new();
    wall.insert("east".to_string(), "none".to_string());
    wall.insert("north".to_string(), "none".to_string());
    wall.insert("south".to_string(), "none".to_string());
    wall.insert("up".to_string(), "false".to_string());
    wall.insert("waterlogged".to_string(), "false".to_string());
    wall.insert("west".to_string(), "none".to_string());
    let wall_model = catalog
        .block_render_model("minecraft:cobblestone_wall", &wall)
        .unwrap();
    let BlockModelShape::Boxes(wall_boxes) = wall_model.shape else {
        panic!("cobblestone_wall can legally resolve to no multipart boxes");
    };
    assert!(wall_boxes.is_empty());

    let mut lever = BTreeMap::new();
    lever.insert("face".to_string(), "floor".to_string());
    lever.insert("facing".to_string(), "north".to_string());
    lever.insert("powered".to_string(), "true".to_string());
    let lever_model = catalog
        .block_render_model("minecraft:lever", &lever)
        .unwrap();
    let BlockModelShape::Quads(lever_quads) = lever_model.shape else {
        panic!("official lever should bake rotated elements to quads");
    };
    assert!(!lever_model.use_ambient_occlusion);
    assert_eq!(lever_quads.len(), 11);
    assert!(lever_quads
        .iter()
        .any(|quad| quad.texture.as_deref() == Some("minecraft:block/lever")));

    let heavy_core = catalog
        .block_render_model("minecraft:heavy_core", &BTreeMap::new())
        .unwrap();
    let BlockModelShape::Box(heavy_core_box) = &heavy_core.shape else {
        panic!("heavy_core should resolve unprefixed face texture slots as a box");
    };
    assert_eq!(
        heavy_core.face_textures.get(BlockModelFace::North),
        "minecraft:block/heavy_core"
    );
    assert_eq!(
        heavy_core_box.face_textures[BlockModelFace::North.index()].as_deref(),
        Some("minecraft:block/heavy_core")
    );

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
    assert_eq!(water.shape, BlockModelShape::Boxes(Vec::new()));
    assert_eq!(
        water.face_textures.get(BlockModelFace::Up),
        "minecraft:block/water_still"
    );

    // `cube_all_inner_faces`: the inverted inner element bakes faces that point inwards.
    let spawner = catalog
        .block_render_model("minecraft:spawner", &BTreeMap::new())
        .unwrap();
    let BlockModelShape::Quads(spawner_quads) = &spawner.shape else {
        panic!("spawner should bake its inner faces as quads");
    };
    assert_eq!(spawner_quads.len(), 12);
    let inner_north = &spawner_quads[8];
    assert_eq!(inner_north.face, BlockModelFace::North);
    assert_close(inner_north.normal, [0.0, 0.0, 1.0]);

    // A cross turned onto its side is not the upright `Cross` fast path.
    let mut amethyst = BTreeMap::new();
    amethyst.insert("facing".to_string(), "north".to_string());
    amethyst.insert("waterlogged".to_string(), "false".to_string());
    let amethyst = catalog
        .block_render_model("minecraft:amethyst_cluster", &amethyst)
        .unwrap();
    assert!(matches!(amethyst.shape, BlockModelShape::Quads(ref quads) if quads.len() == 4));

    let fire_properties = ["east", "north", "south", "up", "west"]
        .into_iter()
        .map(|side| (side.to_string(), "false".to_string()))
        .chain([("age".to_string(), "0".to_string())])
        .collect::<BTreeMap<_, _>>();
    let fire = catalog
        .block_render_model("minecraft:fire", &fire_properties)
        .unwrap();
    assert!(matches!(fire.shape, BlockModelShape::Quads(ref quads) if !quads.is_empty()));
}

#[test]
#[ignore = "requires local vanilla 26.1 sources"]
fn resolves_all_local_vanilla_block_state_models() {
    let roots = PackRoots::from_environment().unwrap();
    let catalog = roots.load_block_model_catalog().unwrap();
    let states = vanilla_block_state_identities();
    assert_eq!(states.len(), VANILLA_BLOCK_STATE_COUNT as usize);

    let mut failures = Vec::new();
    for state in states {
        if catalog
            .block_render_model(&state.name, &state.properties)
            .is_none()
        {
            failures.push(format!("{}#{}", state.id, state.name));
        }
    }

    assert!(
        failures.is_empty(),
        "failed to resolve {} vanilla block state models: {}",
        failures.len(),
        failures
            .iter()
            .take(32)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[test]
fn blockstate_rejects_invalid_and_overflowing_variant_weights() {
    for (label, variants_json) in [
        (
            "zero-weight",
            r#"[{ "model": "minecraft:block/a", "weight": 0 }]"#,
        ),
        (
            "above-i32-max",
            r#"[{ "model": "minecraft:block/a", "weight": 2147483648 }]"#,
        ),
        (
            "sum-overflow",
            r#"[
                { "model": "minecraft:block/a", "weight": 1500000000 },
                { "model": "minecraft:block/b", "weight": 1500000000 }
            ]"#,
        ),
    ] {
        let root = unique_temp_dir(&format!("blockstate-weight-{label}"));
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_single_texture_model(
            &asset_root.join("models").join("block").join("a.json"),
            "minecraft:block/a",
        );
        write_single_texture_model(
            &asset_root.join("models").join("block").join("b.json"),
            "minecraft:block/b",
        );
        write_json(
            &asset_root.join("blockstates").join("bad.json"),
            &format!(r#"{{ "variants": {{ "": {variants_json} }} }}"#),
        );

        let result = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog();
        assert!(
            result.is_err(),
            "expected weight case {label} to be rejected"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    // Valid sum at i32::MAX must load and select deterministically without hanging.
    let root = unique_temp_dir("blockstate-weight-max-valid");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    for (name, texture) in [("a", "minecraft:block/a"), ("b", "minecraft:block/b")] {
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join(format!("{name}.json")),
            &format!(
                r##"{{
                    "textures": {{ "all": "{texture}" }},
                    "elements": [{{
                        "from": [0, 0, 0],
                        "to": [16, 8, 16],
                        "faces": {{
                            "north": {{ "texture": "#all" }}
                        }}
                    }}]
                }}"##
            ),
        );
    }
    write_json(
        &asset_root.join("blockstates").join("ok.json"),
        r#"{
            "variants": {
                "": [
                    { "model": "minecraft:block/a", "weight": 2147483646 },
                    { "model": "minecraft:block/b", "weight": 1 }
                ]
            }
        }"#,
    );
    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    for seed in [0_i64, 1, 42, -999_999] {
        assert!(catalog
            .block_render_model_with_seed("minecraft:ok", &BTreeMap::new(), Some(seed))
            .is_some());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn multipart_many_weighted_selectors_do_not_overflow_combination_or_seeded_resolution() {
    let root = unique_temp_dir("multipart-weighted-overflow");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    for (name, texture) in [("a", "minecraft:block/a"), ("b", "minecraft:block/b")] {
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join(format!("{name}.json")),
            &format!(
                r##"{{
                    "textures": {{ "all": "{texture}" }},
                    "elements": [{{
                        "from": [0, 0, 0],
                        "to": [16, 8, 16],
                        "faces": {{
                            "north": {{ "texture": "#all" }}
                        }}
                    }}]
                }}"##
            ),
        );
    }
    // 80 selectors * 2 weighted variants = 2^80 combinations (overflows usize).
    let selectors = (0..80)
        .map(|_| {
            r#"{ "apply": [
                { "model": "minecraft:block/a", "weight": 1 },
                { "model": "minecraft:block/b", "weight": 1 }
            ] }"#
        })
        .collect::<Vec<_>>()
        .join(",");
    write_json(
        &asset_root.join("blockstates").join("many_weighted.json"),
        &format!(r#"{{ "multipart": [{selectors}] }}"#),
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let props = BTreeMap::new();
    let choices = catalog
        .block_state_model_choices("minecraft:many_weighted", &props)
        .unwrap();
    assert_eq!(choices.combination_count(), usize::MAX);
    let comb = choices.combination(Some(123_456_789));
    assert!(catalog.block_state_model(&choices, comb).is_some());
    let seeded = catalog
        .block_render_model_with_seed("minecraft:many_weighted", &props, Some(123_456_789))
        .unwrap();
    let BlockModelShape::Boxes(boxes) = seeded.shape else {
        panic!("expected combined multipart boxes");
    };
    assert_eq!(boxes.len(), 80);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn model_parent_cycles_and_deep_chains_resolve_iteratively_to_none() {
    let root = unique_temp_dir("model-parent-cycles-and-depth");
    let asset_root = root
        .join("sources")
        .join(MC_VERSION)
        .join("assets")
        .join("minecraft");
    let block_dir = asset_root.join("models").join("block");
    write_json(
        &block_dir.join("cycle_a.json"),
        r#"{ "parent": "minecraft:block/cycle_b" }"#,
    );
    write_json(
        &block_dir.join("cycle_b.json"),
        r#"{ "parent": "minecraft:block/cycle_a" }"#,
    );
    write_single_texture_model(&block_dir.join("chain_0.json"), "minecraft:block/base");
    for i in 1..=300 {
        write_json(
            &block_dir.join(format!("chain_{i}.json")),
            &format!(r#"{{ "parent": "minecraft:block/chain_{}" }}"#, i - 1),
        );
    }

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    assert!(catalog.resolve_model("minecraft:block/cycle_a").is_none());
    assert!(catalog.resolve_model("minecraft:block/cycle_b").is_none());
    assert!(catalog.resolve_model("minecraft:block/chain_300").is_none());
    assert!(catalog.resolve_model("minecraft:block/chain_20").is_some());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_catalog_enforces_caps_caches_variants_and_reports_translucency() {
    // 1. Rejects > MAX_MULTIPART_SELECTORS_PER_BLOCKSTATE selectors.
    {
        let root = unique_temp_dir("multipart-selector-cap");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_single_texture_model(
            &asset_root.join("models").join("block").join("a.json"),
            "minecraft:block/a",
        );
        let selectors = (0..=super::blockstates::MAX_MULTIPART_SELECTORS_PER_BLOCKSTATE)
            .map(|_| r#"{ "apply": { "model": "minecraft:block/a" } }"#)
            .collect::<Vec<_>>()
            .join(",");
        write_json(
            &asset_root.join("blockstates").join("too_many.json"),
            &format!(r#"{{ "multipart": [{selectors}] }}"#),
        );
        assert!(PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    // 2. Rejects cuboid model with > MAX_ELEMENTS_PER_CUBOID_MODEL elements.
    {
        let root = unique_temp_dir("cuboid-element-cap");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        let elements = (0..=super::MAX_ELEMENTS_PER_CUBOID_MODEL)
            .map(|_| r##"{ "faces": { "north": { "texture": "#all" } } }"##)
            .collect::<Vec<_>>()
            .join(",");
        write_json(
            &asset_root.join("models").join("block").join("huge.json"),
            &format!(
                r#"{{ "textures": {{ "all": "minecraft:block/a" }}, "elements": [{elements}] }}"#
            ),
        );
        assert!(PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    // 3. Rejects multipart blockstate combining > MAX_COMBINED_ELEMENTS_PER_BLOCKSTATE elements.
    {
        let root = unique_temp_dir("multipart-combined-element-cap");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        let elements = (0..500)
            .map(|_| r##"{ "faces": { "north": { "texture": "#all" } } }"##)
            .collect::<Vec<_>>()
            .join(",");
        write_json(
            &asset_root.join("models").join("block").join("heavy.json"),
            &format!(
                r#"{{ "textures": {{ "all": "minecraft:block/a" }}, "elements": [{elements}] }}"#
            ),
        );
        // 5 * 500 = 2500 > 2048 combined element cap.
        let selectors = (0..5)
            .map(|_| r#"{ "apply": { "model": "minecraft:block/heavy" } }"#)
            .collect::<Vec<_>>()
            .join(",");
        write_json(
            &asset_root.join("blockstates").join("bomb.json"),
            &format!(r#"{{ "multipart": [{selectors}] }}"#),
        );
        assert!(PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    // 4. Caches rotated variants by normalized key and exposes fast translucency flag.
    {
        let root = unique_temp_dir("variant-cache-and-translucency");
        let asset_root = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &asset_root.join("models").join("block").join("opaque.json"),
            r##"{
                "textures": { "all": "minecraft:block/stone" },
                "elements": [{ "faces": { "north": { "texture": "#all" } } }]
            }"##,
        );
        write_json(
            &asset_root.join("models").join("block").join("glassy.json"),
            r##"{
                "textures": { "all": { "sprite": "minecraft:block/glass", "force_translucent": true } },
                "elements": [{ "faces": { "north": { "texture": "#all" } } }]
            }"##,
        );
        write_json(
            &asset_root.join("blockstates").join("opaque_block.json"),
            r#"{ "variants": { "": { "model": "minecraft:block/opaque", "y": 90 } } }"#,
        );
        write_json(
            &asset_root.join("blockstates").join("mixed_block.json"),
            r#"{
                "multipart": [
                    { "when": { "lit": "false" }, "apply": { "model": "minecraft:block/opaque" } },
                    { "when": { "lit": "true" }, "apply": { "model": "minecraft:block/glassy" } }
                ]
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap();
        let v1: super::RawModelVariant =
            serde_json::from_str(r#"{ "model": "block/opaque", "y": 90 }"#).unwrap();
        let v2: super::RawModelVariant =
            serde_json::from_str(r#"{ "model": "minecraft:block/opaque", "x": 360, "y": -270 }"#)
                .unwrap();
        let c1 = catalog.cached_rotated_variant(&v1).unwrap();
        let c2 = catalog.cached_rotated_variant(&v2).unwrap();
        assert!(std::sync::Arc::ptr_eq(&c1, &c2));

        let props = BTreeMap::from([("lit".to_string(), "false".to_string())]);
        assert_eq!(
            catalog.block_material_force_translucent("minecraft:opaque_block", &BTreeMap::new()),
            Some(false)
        );
        assert_eq!(
            catalog.block_material_force_translucent("minecraft:mixed_block", &props),
            Some(true)
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}

fn write_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn write_single_texture_model(path: &Path, texture: &str) {
    write_json(
        path,
        &format!(
            r##"{{
                "textures": {{ "all": "{texture}" }},
                "elements": [{{
                    "faces": {{
                        "north": {{ "texture": "#all" }}
                    }}
                }}]
            }}"##
        ),
    );
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bbb-client-assets-{name}-{}-{nonce}",
        std::process::id()
    ))
}

fn assert_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {actual} to be close to {expected}"
        );
    }
}
