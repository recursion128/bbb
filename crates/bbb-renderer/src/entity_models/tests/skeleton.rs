use super::*;

use crate::entity_models::geometry::{ModelCubeDesc, ModelPartDesc};
use crate::entity_models::model::ModelCube;

/// An inline `HumanoidModel` body-layer fixture for the desc-level swing reference helpers (the
/// skeleton now builds a named tree, so it has no `*_PARTS` desc const). Head, body, arms at `±5`,
/// legs at `±2` — the vanilla `SkeletonModel.createBodyLayer` layout, head first.
const SKELETON_BONE_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: SKELETON_BONE,
}];
const fn humanoid_fixture_part(offset: [f32; 3]) -> ModelPartDesc {
    ModelPartDesc {
        pose: PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_BONE_CUBE,
        children: &[],
    }
}
const SKELETON_HUMANOID_FIXTURE: [ModelPartDesc; 6] = [
    humanoid_fixture_part([0.0, 0.0, 0.0]),
    humanoid_fixture_part([0.0, 0.0, 0.0]),
    humanoid_fixture_part([-5.0, 2.0, 0.0]),
    humanoid_fixture_part([5.0, 2.0, 0.0]),
    humanoid_fixture_part([-2.0, 12.0, 0.0]),
    humanoid_fixture_part([2.0, 12.0, 0.0]),
];

#[test]
fn skeleton_model_parts_match_vanilla_26_1_body_layer() {
    // The skeleton builds a named-children tree (`head` -> `hat`, `body`, the arms/legs), so the
    // head look resolves the `head` child by name; the geometry is asserted on the per-part unified
    // cube consts (colored tint + textured uv/tex/mirror).
    assert_eq!(
        SKELETON_HAT[0],
        ModelCube::new(
            [-4.5, -8.5, -4.5],
            [9.0, 9.0, 9.0],
            SKELETON_BONE,
            [8.0, 8.0, 8.0],
            [32.0, 0.0],
            false,
        )
    );
    assert_eq!(SKELETON_HEAD[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(SKELETON_BODY[0].size, [8.0, 12.0, 4.0]);
    assert_eq!(SKELETON_RIGHT_ARM[0].size, [2.0, 12.0, 2.0]);
    assert!(!SKELETON_RIGHT_ARM[0].mirror);
    assert!(SKELETON_LEFT_ARM[0].mirror);
    assert_eq!(SKELETON_RIGHT_LEG[0].size, [2.0, 12.0, 2.0]);
    assert!(!SKELETON_RIGHT_LEG[0].mirror);
    assert!(SKELETON_LEFT_LEG[0].mirror);
}

#[test]
fn skeleton_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::skeleton(115, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    // The always-on HumanoidModel idle arm bob rolls the resting arms by zRot ±0.1 at
    // ageInTicks 0, which widens the X extent from the bare ±0.375 bind half-width to
    // ±0.43708366 (Y/Z are unchanged — a zRot roll only moves the arm in the XY plane).
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.43708366, 64.001, -0.28125]);
    assert_close3(max, [0.43708366, 66.03225, 0.28125]);
}

#[test]
fn skeleton_texture_refs_match_vanilla_renderers() {
    assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
    assert_eq!(
        EntityModelKind::Skeleton.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/skeleton.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/stray.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/parched.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/wither_skeleton.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: false }
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/bogged.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
        .vanilla_layer_texture_refs(),
        &[EntityModelTextureRef {
            path: "textures/entity/skeleton/stray_overlay.png",
            size: [64, 32],
        }]
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: false }
        }
        .vanilla_layer_texture_refs(),
        &[EntityModelTextureRef {
            path: "textures/entity/skeleton/bogged_overlay.png",
            size: [64, 32],
        }]
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: true }
        }
        .vanilla_layer_texture_refs(),
        &[BOGGED_OVERLAY_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
        .vanilla_layer_texture_refs(),
        &[]
    );
}

#[test]
fn skeleton_textured_layer_passes_match_vanilla_renderer_model_layers() {
    // Both the base body and the clothing overlay come from unified model trees, so every layer-pass
    // `parts` field is vestigial (`&[]`).
    let base = skeleton_textured_layer_passes(None);
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].kind, EntityModelLayerKind::SkeletonBase);
    assert_eq!(base[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(base[0].model_layer, MODEL_LAYER_SKELETON);
    assert_eq!(base[0].texture, SKELETON_TEXTURE_REF);
    assert!(base[0].parts.is_empty());
    assert_eq!(base[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(base[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((base[0].collector_order, base[0].submit_sequence), (0, 0));

    let stray = skeleton_textured_layer_passes(Some(SkeletonModelFamily::Stray));
    assert_eq!(stray.len(), 2);
    assert_eq!(stray[0].model_layer, MODEL_LAYER_STRAY);
    assert_eq!(stray[0].texture, STRAY_TEXTURE_REF);
    assert!(stray[0].parts.is_empty());
    assert_eq!(stray[1].kind, EntityModelLayerKind::SkeletonClothing);
    assert_eq!(stray[1].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(stray[1].model_layer, MODEL_LAYER_STRAY_OUTER_LAYER);
    assert_eq!(stray[1].texture, STRAY_OVERLAY_TEXTURE_REF);
    assert!(stray[1].parts.is_empty());
    assert_eq!(stray[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(stray[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((stray[1].collector_order, stray[1].submit_sequence), (1, 1));

    let parched = skeleton_textured_layer_passes(Some(SkeletonModelFamily::Parched));
    assert_eq!(parched.len(), 1);
    assert_eq!(parched[0].model_layer, MODEL_LAYER_PARCHED);
    assert_eq!(parched[0].texture, PARCHED_TEXTURE_REF);
    assert!(parched[0].parts.is_empty());

    let wither = skeleton_textured_layer_passes(Some(SkeletonModelFamily::WitherSkeleton));
    assert_eq!(wither.len(), 1);
    assert_eq!(wither[0].model_layer, MODEL_LAYER_WITHER_SKELETON);
    assert_eq!(wither[0].texture, WITHER_SKELETON_TEXTURE_REF);
    assert!(wither[0].parts.is_empty());

    let bogged =
        skeleton_textured_layer_passes(Some(SkeletonModelFamily::Bogged { sheared: false }));
    assert_eq!(bogged.len(), 2);
    assert_eq!(bogged[0].model_layer, MODEL_LAYER_BOGGED);
    assert_eq!(bogged[0].texture, BOGGED_TEXTURE_REF);
    assert!(bogged[0].parts.is_empty());
    assert_eq!(bogged[1].kind, EntityModelLayerKind::SkeletonClothing);
    assert_eq!(bogged[1].model_layer, MODEL_LAYER_BOGGED_OUTER_LAYER);
    assert_eq!(bogged[1].texture, BOGGED_OVERLAY_TEXTURE_REF);
    assert!(bogged[1].parts.is_empty());
    assert_eq!(
        (bogged[1].collector_order, bogged[1].submit_sequence),
        (1, 1)
    );

    let sheared_bogged =
        skeleton_textured_layer_passes(Some(SkeletonModelFamily::Bogged { sheared: true }));
    assert_eq!(sheared_bogged.len(), 2);
    assert!(sheared_bogged[0].parts.is_empty());
    assert!(sheared_bogged[1].parts.is_empty());
}

#[test]
fn skeleton_variant_parts_match_vanilla_26_1_body_layers() {
    // The parched/bogged variants build named-children trees too: parched lists the body first with
    // an empty hat child; bogged parents the hat (+ mushrooms, hidden when sheared) under the head.
    // The geometry is asserted on the per-part unified cube consts.
    assert_eq!(PARCHED_BODY[0].size, [8.0, 12.0, 4.0]);
    assert_eq!(PARCHED_BODY[1].size, [8.0, 1.0, 4.0]);
    // The inflated overlay cube keeps the base box as uv_size.
    assert_eq!(PARCHED_BODY[2].size, [8.05, 12.05, 4.05]);
    assert_eq!(PARCHED_BODY[2].uv_size, [8.0, 12.0, 4.0]);
    assert_eq!(PARCHED_HEAD[1].size, [8.4, 8.4, 8.4]);
    assert_eq!(PARCHED_HEAD[1].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PARCHED_RIGHT_ARM[1].size, [3.0, 12.0, 3.0]);
    assert_eq!(PARCHED_LEFT_ARM[1].size, [3.0, 12.0, 3.0]);
    assert_eq!(PARCHED_RIGHT_LEG[1].size, [3.0, 12.0, 3.0]);

    // The bogged mushroom planes (flat 6x4x0 quads) and the bone body cubes.
    assert_eq!(
        BOGGED_RED_MUSHROOM_PLANE[0],
        ModelCube::new(
            [-3.0, -3.0, 0.0],
            [6.0, 4.0, 0.0],
            BOGGED_RED_MUSHROOM_COLOR,
            [6.0, 4.0, 0.0],
            [50.0, 16.0],
            false,
        )
    );
    assert_eq!(BOGGED_BROWN_MUSHROOM_PLANE[0].size, [6.0, 4.0, 0.0]);
    assert_eq!(BOGGED_BROWN_TOP_MUSHROOM_PLANE[0].size, [6.0, 4.0, 0.0]);
    assert_eq!(BOGGED_HEAD[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(BOGGED_BODY[0].size, [8.0, 12.0, 4.0]);
    assert!(BOGGED_LEFT_ARM[0].mirror);
    assert!(BOGGED_LEFT_LEG[0].mirror);
}

#[test]
fn skeleton_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_SKELETON, "minecraft:skeleton#main");
    assert_eq!(MODEL_LAYER_STRAY, "minecraft:stray#main");
    assert_eq!(MODEL_LAYER_PARCHED, "minecraft:parched#main");
    assert_eq!(
        MODEL_LAYER_WITHER_SKELETON,
        "minecraft:wither_skeleton#main"
    );
    assert_eq!(MODEL_LAYER_BOGGED, "minecraft:bogged#main");
    assert_eq!(MODEL_LAYER_STRAY_OUTER_LAYER, "minecraft:stray#outer");
    assert_eq!(MODEL_LAYER_BOGGED_OUTER_LAYER, "minecraft:bogged#outer");
    // The base skeleton-family UVs are now carried on the unified cubes' `.tex` field. The left
    // arm/leg mirror the right's texOffs.
    assert_eq!(SKELETON_RIGHT_ARM[0].tex, [40.0, 16.0]);
    assert!(!SKELETON_RIGHT_ARM[0].mirror);
    assert_eq!(SKELETON_LEFT_ARM[0].tex, [40.0, 16.0]);
    assert!(SKELETON_LEFT_ARM[0].mirror);
    assert_eq!(SKELETON_RIGHT_LEG[0].tex, [0.0, 16.0]);
    assert!(SKELETON_LEFT_LEG[0].mirror);
    assert_eq!(SKELETON_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(SKELETON_HAT[0].tex, [32.0, 0.0]);

    assert_eq!(PARCHED_BODY[1].tex, [28.0, 0.0]);
    assert_eq!(PARCHED_BODY[2].tex, [16.0, 48.0]);
    assert_eq!(PARCHED_BODY[2].uv_size, [8.0, 12.0, 4.0]);
    assert_eq!(PARCHED_HEAD[1].tex, [0.0, 32.0]);
    assert_eq!(PARCHED_RIGHT_ARM[1].tex, [42.0, 33.0]);
    assert_eq!(PARCHED_LEFT_ARM[0].tex, [56.0, 16.0]);
    assert_eq!(PARCHED_LEFT_ARM[1].tex, [40.0, 48.0]);
    assert_eq!(PARCHED_RIGHT_LEG[1].tex, [0.0, 49.0]);
    assert_eq!(PARCHED_LEFT_LEG[1].tex, [4.0, 49.0]);

    assert_eq!(BOGGED_RED_MUSHROOM_PLANE[0].tex, [50.0, 16.0]);
    assert_eq!(BOGGED_BROWN_MUSHROOM_PLANE[0].tex, [50.0, 22.0]);
    assert_eq!(BOGGED_BROWN_TOP_MUSHROOM_PLANE[0].tex, [50.0, 28.0]);
    assert_eq!(BOGGED_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(BOGGED_HAT[0].tex, [32.0, 0.0]);

    // The stray/bogged clothing overlay is now a textured-only named-children `ModelCube` tree; the
    // geometry/UV are asserted on the per-part cube consts and the shared limb pose consts.
    assert_eq!(
        STRAY_OUTER_HEAD,
        ModelCube::new(
            [-4.25, -8.25, -4.25],
            [8.5, 8.5, 8.5],
            [0.0, 0.0, 0.0, 0.0],
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(
        STRAY_OUTER_HAT,
        ModelCube::new(
            [-4.75, -8.75, -4.75],
            [9.5, 9.5, 9.5],
            [0.0, 0.0, 0.0, 0.0],
            [8.0, 8.0, 8.0],
            [32.0, 0.0],
            false,
        )
    );
    assert_eq!(STRAY_OUTER_BODY.min, [-4.25, -0.25, -2.25]);
    assert_eq!(STRAY_OUTER_BODY.size, [8.5, 12.5, 4.5]);
    assert_eq!(STRAY_OUTER_RIGHT_ARM.tex, [40.0, 16.0]);
    assert_eq!(STRAY_OUTER_RIGHT_ARM.size, [4.5, 12.5, 4.5]);
    assert!(STRAY_OUTER_LEFT_ARM.mirror);
    assert_eq!(STRAY_OUTER_RIGHT_LEG.tex, [0.0, 16.0]);
    assert_eq!(CLOTHING_RIGHT_LEG_POSE.offset, [-1.9, 12.0, 0.0]);

    assert_eq!(
        BOGGED_OUTER_HEAD,
        ModelCube::new(
            [-4.2, -8.2, -4.2],
            [8.4, 8.4, 8.4],
            [0.0, 0.0, 0.0, 0.0],
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(BOGGED_OUTER_HAT.min, [-4.7, -8.7, -4.7]);
    assert_eq!(BOGGED_OUTER_HAT.size, [9.4, 9.4, 9.4]);
    assert_eq!(BOGGED_OUTER_BODY.min, [-4.2, -0.2, -2.2]);
    assert_eq!(BOGGED_OUTER_BODY.size, [8.4, 12.4, 4.4]);
    assert_eq!(BOGGED_OUTER_RIGHT_ARM.min, [-3.2, -2.2, -2.2]);
    assert_eq!(BOGGED_OUTER_RIGHT_ARM.size, [4.4, 12.4, 4.4]);
    assert!(BOGGED_OUTER_LEFT_ARM.mirror);
    assert_eq!(CLOTHING_LEFT_LEG_POSE.offset, [1.9, 12.0, 0.0]);
}

#[test]
fn entity_texture_atlas_stitches_official_skeleton_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 256);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/skeleton/skeleton.png",
            "textures/entity/skeleton/stray.png",
            "textures/entity/skeleton/stray_overlay.png",
            "textures/entity/skeleton/parched.png",
            "textures/entity/skeleton/wither_skeleton.png",
            "textures/entity/skeleton/bogged.png",
            "textures/entity/skeleton/bogged_overlay.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 32.0 / 256.0]);
    assert_close2(layout.entries[2].uv.min, [0.0, 64.0 / 256.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 96.0 / 256.0]);
    assert_close2(layout.entries[3].uv.min, [0.0, 96.0 / 256.0]);
    assert_close2(layout.entries[3].uv.max, [1.0, 160.0 / 256.0]);
    assert_close2(layout.entries[6].uv.min, [0.0, 224.0 / 256.0]);
    assert_close2(layout.entries[6].uv.max, [1.0, 1.0]);
    assert!(entity_model_texture_refs().contains(&SKELETON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRAY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRAY_OVERLAY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&PARCHED_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_SKELETON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BOGGED_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BOGGED_OVERLAY_TEXTURE_REF));
    let stray_overlay_first_pixel =
        rgba_offset(layout.width, 64, 0, "stray overlay atlas row").unwrap();
    assert_eq!(
        &rgba[stray_overlay_first_pixel..stray_overlay_first_pixel + 4],
        &[2; 4]
    );
    let parched_first_pixel = rgba_offset(layout.width, 96, 0, "parched atlas row").unwrap();
    assert_eq!(&rgba[parched_first_pixel..parched_first_pixel + 4], &[3; 4]);
}

#[test]
fn skeleton_variant_meshes_use_vanilla_body_layer_geometry() {
    let skeleton = entity_model_mesh(&[EntityModelInstance::skeleton(51, [0.0, 64.0, 0.0], 0.0)]);
    let stray = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        128,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Stray,
    )]);
    assert_eq!(stray.vertices, skeleton.vertices);
    assert_eq!(stray.indices, skeleton.indices);

    let wither = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        146,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::WitherSkeleton,
    )]);
    assert_eq!(wither.opaque_faces, 42);
    assert_eq!(wither.vertices.len(), 168);
    assert_eq!(wither.indices.len(), 252);
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_SKELETON_DARK, 0.78)));
    let (wither_min, wither_max) = mesh_extents(&wither);
    assert_close3(wither_min, [-0.52450037, 64.0012, -0.33750004]);
    assert_close3(wither_max, [0.52450037, 66.4387, 0.33750004]);

    let parched = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        97,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Parched,
    )]);
    assert_eq!(parched.opaque_faces, 78);
    assert_eq!(parched.vertices.len(), 312);
    assert_eq!(parched.indices.len(), 468);
    let (parched_min, parched_max) = mesh_extents(&parched);
    assert_close3(parched_min, [-0.50238097, 64.001, -0.26250002]);
    assert_close3(parched_max, [0.50238097, 66.0135, 0.26250002]);

    let bogged = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        16,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Bogged { sheared: false },
    )]);
    assert_eq!(bogged.opaque_faces, 78);
    assert_eq!(bogged.vertices.len(), 312);
    assert_eq!(bogged.indices.len(), 468);
    assert!(bogged
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(BOGGED_RED_MUSHROOM_COLOR, 0.78)));
    let (bogged_min, bogged_max) = mesh_extents(&bogged);
    assert_close3(bogged_min, [-0.43708366, 64.001, -0.5]);
    assert_close3(bogged_max, [0.43708366, 66.1885, 0.32008255]);

    let sheared_bogged = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        17,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Bogged { sheared: true },
    )]);
    assert_eq!(sheared_bogged.opaque_faces, 42);
    assert_eq!(sheared_bogged.vertices.len(), 168);
    assert_eq!(sheared_bogged.indices.len(), 252);
    assert_same_geometry(&sheared_bogged, &skeleton);
}

#[test]
fn skeleton_textured_mesh_uses_vanilla_uvs_tints_and_variant_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    let skeleton = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton(51, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(skeleton.cutout_faces, 42);
    assert_eq!(skeleton.vertices.len(), 168);
    assert_eq!(skeleton.indices.len(), 252);
    assert_close2(skeleton.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(skeleton
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // Same idle-bob X widening as the colored base skeleton above (±0.375 -> ±0.43708366).
    let (min, max) = textured_mesh_extents(&skeleton);
    assert_close3(min, [-0.43708366, 64.001, -0.28125]);
    assert_close3(max, [0.43708366, 66.03225, 0.28125]);

    let stray = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            128,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Stray,
        )],
        &atlas,
    );
    assert_eq!(stray.cutout_faces, 84);
    assert_eq!(stray.vertices.len(), 336);
    assert_close2(stray.vertices[0].uv, [16.0 / 64.0, 32.0 / 256.0]);
    assert_close2(stray.vertices[168].uv, [16.0 / 64.0, 64.0 / 256.0]);
    let (stray_min, stray_max) = textured_mesh_extents(&stray);
    assert_close3(stray_min, [-0.578566, 63.985374, -0.296875]);
    assert_close3(stray_max, [0.578566, 66.047875, 0.296875]);

    let wither = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            146,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::WitherSkeleton,
        )],
        &atlas,
    );
    assert_eq!(wither.cutout_faces, 42);
    assert_close2(wither.vertices[0].uv, [16.0 / 64.0, 160.0 / 256.0]);
    let (wither_min, wither_max) = textured_mesh_extents(&wither);
    assert_close3(wither_min, [-0.52450037, 64.0012, -0.33750004]);
    assert_close3(wither_max, [0.52450037, 66.4387, 0.33750004]);

    let parched = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            97,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Parched,
        )],
        &atlas,
    );
    assert_eq!(parched.cutout_faces, 78);
    assert_eq!(parched.vertices.len(), 312);
    assert_close2(parched.vertices[0].uv, [28.0 / 64.0, 112.0 / 256.0]);
    let (parched_min, parched_max) = textured_mesh_extents(&parched);
    assert_close3(parched_min, [-0.50238097, 64.001, -0.26250002]);
    assert_close3(parched_max, [0.50238097, 66.0135, 0.26250002]);

    let bogged = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            16,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Bogged { sheared: false },
        )],
        &atlas,
    );
    assert_eq!(bogged.cutout_faces, 120);
    assert_eq!(bogged.vertices.len(), 480);
    assert_close2(bogged.vertices[48].uv, [56.0 / 64.0, 208.0 / 256.0]);
    assert_close2(bogged.vertices[312].uv, [16.0 / 64.0, 224.0 / 256.0]);
    let (bogged_min, bogged_max) = textured_mesh_extents(&bogged);
    assert_close3(bogged_min, [-0.57514465, 63.9885, -0.5]);
    assert_close3(bogged_max, [0.57514465, 66.1885, 0.32008255]);

    let sheared_bogged = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            17,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Bogged { sheared: true },
        )],
        &atlas,
    );
    assert_eq!(sheared_bogged.cutout_faces, 84);
    assert_eq!(sheared_bogged.vertices.len(), 336);
    assert_close2(
        sheared_bogged.vertices[168].uv,
        [16.0 / 64.0, 224.0 / 256.0],
    );
    let (sheared_bogged_min, sheared_bogged_max) = textured_mesh_extents(&sheared_bogged);
    assert_close3(sheared_bogged_min, [-0.57514465, 63.9885, -0.29375]);
    assert_close3(sheared_bogged_max, [0.57514465, 66.04475, 0.29375]);
}

#[test]
fn skeleton_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    // Skeleton head is part 0: head look turns it without changing vertex count.
    let base = EntityModelInstance::skeleton(714, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    assert_ne!(yawed.vertices, pitched.vertices);

    // Parched lists the body first (head at index 1); head look turns the head
    // and leaves the leading body cube untouched.
    let parched = EntityModelInstance::skeleton_variant(
        715,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Parched,
    );
    let parched_resting = entity_model_textured_mesh(&[parched], &atlas);
    let parched_looking =
        entity_model_textured_mesh(&[parched.with_head_look(45.0, -20.0)], &atlas);
    assert_ne!(parched_resting.vertices, parched_looking.vertices);
    assert_eq!(
        parched_resting.vertices[0..24],
        parched_looking.vertices[0..24]
    );
}

#[test]
fn humanoid_limb_swing_parts_assign_vanilla_skeleton_leg_phases_by_side() {
    use std::borrow::Cow;

    // SkeletonModel extends HumanoidModel, so the legs swing via the inherited
    // HumanoidModel.setupAnim: rightLeg.xRot = cos(pos * 0.6662) * 1.4 * speed (in
    // phase), leftLeg.xRot = cos(pos * 0.6662 + π) * 1.4 * speed (out of phase).
    // SKELETON_HUMANOID_FIXTURE lists rightLeg (offset x = -2) at index 4 and leftLeg (x = +2)
    // at index 5. With pos = 0, speed = 1: rightLeg = 1.4, leftLeg = -1.4.
    let posed = humanoid_limb_swing_parts(
        Cow::Borrowed(&SKELETON_HUMANOID_FIXTURE),
        HUMANOID_LEG_PART_INDICES,
        0.0,
        1.0,
    );
    assert!(
        (posed[4].pose.rotation[0] - 1.4).abs() < 1e-5,
        "right leg in phase: {}",
        posed[4].pose.rotation[0]
    );
    assert!(
        (posed[5].pose.rotation[0] + 1.4).abs() < 1e-5,
        "left leg out of phase: {}",
        posed[5].pose.rotation[0]
    );
    // humanoid_limb_swing_parts only swings the legs; the arms (indices 2, 3) are
    // posed separately by humanoid_arm_swing_parts, so this helper leaves them at rest.
    assert_eq!(
        posed[2].pose.rotation,
        SKELETON_HUMANOID_FIXTURE[2].pose.rotation
    );
    assert_eq!(
        posed[3].pose.rotation,
        SKELETON_HUMANOID_FIXTURE[3].pose.rotation
    );

    // A general (pos, speed) reproduces cos(pos * 0.6662 [+ π]) * 1.4 * speed,
    // including the 0.6662 frequency factor.
    let posed = humanoid_limb_swing_parts(
        Cow::Borrowed(&SKELETON_HUMANOID_FIXTURE),
        HUMANOID_LEG_PART_INDICES,
        1.5,
        0.5,
    );
    let phase = 1.5_f32 * 0.6662;
    assert!((posed[4].pose.rotation[0] - phase.cos() * 1.4 * 0.5).abs() < 1e-5);
    assert!(
        (posed[5].pose.rotation[0] - (phase + std::f32::consts::PI).cos() * 1.4 * 0.5).abs() < 1e-5
    );
}

#[test]
fn skeleton_family_swings_its_legs_when_walking() {
    // SkeletonModel extends HumanoidModel and inherits its leg swing unchanged
    // (only the arms are overridden, when aiming, which is deferred). A standing
    // skeleton is inert; a walking one lifts its feet (a shorter model) and splays
    // its legs forward/back (a deeper footprint), for every family variant. This
    // exercises the colored path (the texture-backed render is checked separately).
    let instances: [(&str, EntityModelInstance); 6] = [
        (
            "skeleton",
            EntityModelInstance::skeleton(70, [0.0, 64.0, 0.0], 0.0),
        ),
        (
            "stray",
            EntityModelInstance::skeleton_variant(
                71,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Stray,
            ),
        ),
        (
            "wither_skeleton",
            EntityModelInstance::skeleton_variant(
                72,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::WitherSkeleton,
            ),
        ),
        (
            "parched",
            EntityModelInstance::skeleton_variant(
                73,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Parched,
            ),
        ),
        (
            "bogged",
            EntityModelInstance::skeleton_variant(
                74,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: false },
            ),
        ),
        (
            "bogged_sheared",
            EntityModelInstance::skeleton_variant(
                75,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: true },
            ),
        ),
    ];
    for (name, base) in instances {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
            "{name}: a walking skeleton's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.1,
            "{name}: a walking skeleton's legs should splay along Z"
        );
    }
}

#[test]
fn skeleton_textured_mesh_swings_legs_when_walking() {
    // The real skeleton render path (texture-backed) consumes the projected limb
    // swing via the inherited HumanoidModel.setupAnim leg rotation, on both the
    // body layer and the Stray/Bogged clothing overlay (its layer SkeletonModel
    // runs the same setupAnim). A standing skeleton is byte-identical however far
    // the swing position has advanced; a walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let instances: [(&str, EntityModelInstance); 6] = [
        (
            "skeleton",
            EntityModelInstance::skeleton(80, [0.0, 64.0, 0.0], 0.0),
        ),
        (
            "stray",
            EntityModelInstance::skeleton_variant(
                81,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Stray,
            ),
        ),
        (
            "wither_skeleton",
            EntityModelInstance::skeleton_variant(
                82,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::WitherSkeleton,
            ),
        ),
        (
            "parched",
            EntityModelInstance::skeleton_variant(
                83,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Parched,
            ),
        ),
        (
            "bogged",
            EntityModelInstance::skeleton_variant(
                84,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: false },
            ),
        ),
        (
            "bogged_sheared",
            EntityModelInstance::skeleton_variant(
                85,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: true },
            ),
        ),
    ];
    for (name, base) in instances {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(
            resting.vertices, still.vertices,
            "{name}: a standing skeleton is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{name}: a walking skeleton differs"
        );

        let (rest_min, rest_max) = textured_mesh_extents(&resting);
        let (walk_min, walk_max) = textured_mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
            "{name}: a walking skeleton's feet should lift off the ground"
        );
    }
}

#[test]
fn humanoid_arm_swing_parts_assign_vanilla_skeleton_arm_phases_by_side() {
    use std::borrow::Cow;

    // SkeletonModel extends HumanoidModel and, in its default (non-aiming, non-melee)
    // state, inherits the HumanoidModel.setupAnim arm swing:
    //   rightArm.xRot = cos(pos * 0.6662 + π) * 2.0 * speed * 0.5
    //   leftArm.xRot  = cos(pos * 0.6662)     * 2.0 * speed * 0.5
    // plus the always-on idle bob (AnimationUtils.bobModelPart). SKELETON_HUMANOID_FIXTURE lists
    // rightArm (offset x = -5) at index 2 and leftArm (x = +5) at index 3. At ageInTicks = 0
    // the bob's xRot term is sin(0) * 0.05 = 0, so the xRot is pure swing; with pos = 0,
    // speed = 1: rightArm = -1.0, leftArm = +1.0 — the opposite phase to the same-side leg.
    let posed = humanoid_arm_swing_parts(
        Cow::Borrowed(&SKELETON_HUMANOID_FIXTURE),
        HUMANOID_ARM_PART_INDICES,
        0.0,
        1.0,
        0.0,
    );
    assert!(
        (posed[2].pose.rotation[0] + 1.0).abs() < 1e-5,
        "right arm out of phase: {}",
        posed[2].pose.rotation[0]
    );
    assert!(
        (posed[3].pose.rotation[0] - 1.0).abs() < 1e-5,
        "left arm in phase: {}",
        posed[3].pose.rotation[0]
    );
    // The idle bob's zRot baseline rides on at every age: at ageInTicks = 0 it is
    // scale * (cos(0) * 0.05 + 0.05) = ±0.1 (right arm +, left arm -), accumulated onto the
    // arm's rest zRot. The swing leaves zRot untouched, so this isolates the bob baseline.
    assert!(
        (posed[2].pose.rotation[2] - (SKELETON_HUMANOID_FIXTURE[2].pose.rotation[2] + 0.1)).abs()
            < 1e-5,
        "right arm idle-bob zRot baseline: {}",
        posed[2].pose.rotation[2]
    );
    assert!(
        (posed[3].pose.rotation[2] - (SKELETON_HUMANOID_FIXTURE[3].pose.rotation[2] - 0.1)).abs()
            < 1e-5,
        "left arm idle-bob zRot baseline: {}",
        posed[3].pose.rotation[2]
    );
    // humanoid_arm_swing_parts only poses the arms; the legs (indices 4, 5) are posed
    // separately, so this helper leaves them at rest.
    assert_eq!(
        posed[4].pose.rotation,
        SKELETON_HUMANOID_FIXTURE[4].pose.rotation
    );
    assert_eq!(
        posed[5].pose.rotation,
        SKELETON_HUMANOID_FIXTURE[5].pose.rotation
    );

    // A general (pos, speed) reproduces cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5,
    // including the 0.6662 frequency factor and the 0.5 amplitude scale (ageInTicks = 0
    // keeps the bob's xRot term zero).
    let posed = humanoid_arm_swing_parts(
        Cow::Borrowed(&SKELETON_HUMANOID_FIXTURE),
        HUMANOID_ARM_PART_INDICES,
        1.5,
        0.5,
        0.0,
    );
    let phase = 1.5_f32 * 0.6662;
    assert!(
        (posed[2].pose.rotation[0] - (phase + std::f32::consts::PI).cos() * 2.0 * 0.5 * 0.5).abs()
            < 1e-5
    );
    assert!((posed[3].pose.rotation[0] - phase.cos() * 2.0 * 0.5 * 0.5).abs() < 1e-5);

    // Even at rest (speed = 0) the always-on idle bob re-poses the arms, so the result is
    // owned, not borrowed. At a nonzero age the bob accumulates both terms onto each arm:
    //   xRot += scale * sin(age * 0.067) * 0.05
    //   zRot += scale * (cos(age * 0.09) * 0.05 + 0.05)
    // with scale +1 for the right arm (x < 0) and -1 for the left.
    let age = 31.4_f32;
    let resting = humanoid_arm_swing_parts(
        Cow::Borrowed(&SKELETON_HUMANOID_FIXTURE),
        HUMANOID_ARM_PART_INDICES,
        3.0,
        0.0,
        age,
    );
    assert!(matches!(resting, Cow::Owned(_)));
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;
    assert!(
        (resting[2].pose.rotation[0] - (SKELETON_HUMANOID_FIXTURE[2].pose.rotation[0] + bob_x))
            .abs()
            < 1e-5,
        "right arm idle-bob xRot: {}",
        resting[2].pose.rotation[0]
    );
    assert!(
        (resting[2].pose.rotation[2] - (SKELETON_HUMANOID_FIXTURE[2].pose.rotation[2] + bob_z))
            .abs()
            < 1e-5,
        "right arm idle-bob zRot: {}",
        resting[2].pose.rotation[2]
    );
    assert!(
        (resting[3].pose.rotation[0] - (SKELETON_HUMANOID_FIXTURE[3].pose.rotation[0] - bob_x))
            .abs()
            < 1e-5,
        "left arm idle-bob xRot mirrored: {}",
        resting[3].pose.rotation[0]
    );
    assert!(
        (resting[3].pose.rotation[2] - (SKELETON_HUMANOID_FIXTURE[3].pose.rotation[2] - bob_z))
            .abs()
            < 1e-5,
        "left arm idle-bob zRot mirrored: {}",
        resting[3].pose.rotation[2]
    );
}

#[test]
fn skeleton_family_swings_its_arms_when_walking() {
    // SkeletonModel inherits the HumanoidModel arm counter-swing in its default state
    // (the arms are overridden only in the deferred melee/aiming poses). In the colored
    // body layer the parts emit head(0)+hat(1)+body(2), then right_arm(3), left_arm(4),
    // right_leg(5), left_leg(6) as 24-vertex blocks, so the head/body occupy vertices
    // [0, 72), the arms [72, 120) and the legs [120, 168). A standing skeleton is inert;
    // a walking one swings both arms and legs while the head and body stay put.
    let z_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let families: [(&str, EntityModelInstance); 6] = [
        (
            "skeleton",
            EntityModelInstance::skeleton(90, [0.0, 64.0, 0.0], 0.0),
        ),
        (
            "stray",
            EntityModelInstance::skeleton_variant(
                91,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Stray,
            ),
        ),
        (
            "wither_skeleton",
            EntityModelInstance::skeleton_variant(
                92,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::WitherSkeleton,
            ),
        ),
        (
            "parched",
            EntityModelInstance::skeleton_variant(
                93,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Parched,
            ),
        ),
        (
            "bogged",
            EntityModelInstance::skeleton_variant(
                94,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: false },
            ),
        ),
        (
            "bogged_sheared",
            EntityModelInstance::skeleton_variant(
                95,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: true },
            ),
        ),
    ];
    for (name, base) in families {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        // The skeleton/wither/bogged-sheared families share the plain body geometry whose
        // first three blocks are head/hat/body and whose arms/legs sit at known offsets;
        // assert the per-block split there. The overlaid/extra-cube variants only need to
        // show the arms move on top of the leg swing, which the splay check below covers.
        if matches!(name, "skeleton" | "wither_skeleton" | "bogged_sheared") {
            assert_eq!(
                rest.vertices[0..72],
                walking.vertices[0..72],
                "{name}: head and body never swing"
            );
            assert_ne!(
                rest.vertices[72..120],
                walking.vertices[72..120],
                "{name}: arms swing"
            );
            assert_ne!(
                rest.vertices[120..168],
                walking.vertices[120..168],
                "{name}: legs swing"
            );
            let rest_arm_z = z_extent(&rest.vertices[72..120]);
            let walk_arm_z = z_extent(&walking.vertices[72..120]);
            assert!(
                walk_arm_z > rest_arm_z + 0.1,
                "{name}: a forward/back arm swing deepens the arm Z footprint: {rest_arm_z} -> {walk_arm_z}"
            );
        } else {
            assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");
        }
    }
}

#[test]
fn skeleton_textured_mesh_swings_arms_when_walking() {
    // The texture-backed skeleton render path runs the same inherited HumanoidModel
    // arm swing. The plain body layer emits the parts in the same order as the colored
    // path, so the arms occupy textured vertices [72, 120). A standing skeleton is
    // byte-identical however far the swing has advanced; a walking one swings its arms.
    let z_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let base = EntityModelInstance::skeleton(96, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

    assert_eq!(
        resting.vertices, still.vertices,
        "a standing skeleton is inert"
    );
    assert_eq!(
        resting.vertices[0..72],
        walking.vertices[0..72],
        "head and body never swing"
    );
    assert_ne!(
        resting.vertices[72..120],
        walking.vertices[72..120],
        "arms swing"
    );
    assert_ne!(
        resting.vertices[120..168],
        walking.vertices[120..168],
        "legs swing"
    );
    let rest_arm_z = z_extent(&resting.vertices[72..120]);
    let walk_arm_z = z_extent(&walking.vertices[72..120]);
    assert!(
        walk_arm_z > rest_arm_z + 0.1,
        "the textured arms splay along Z when walking: {rest_arm_z} -> {walk_arm_z}"
    );
}

#[test]
fn skeleton_textured_arms_idle_bob_as_age_advances() {
    // The restructured textured humanoid path applies the inherited HumanoidModel idle arm
    // bob every frame, so a standing skeleton's arms ([72, 120)) move with ageInTicks while
    // the head and body ([0, 72)) and the legs ([120, 168)) stay byte-identical across ages.
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let base = EntityModelInstance::skeleton(97, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_textured_mesh(&[base], &atlas);
    let later = entity_model_textured_mesh(&[base.with_age_in_ticks(27.3)], &atlas);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_eq!(
        early.vertices[0..72],
        later.vertices[0..72],
        "head and body do not bob"
    );
    assert_ne!(
        early.vertices[72..120],
        later.vertices[72..120],
        "the arms idle-bob with ageInTicks"
    );
    assert_eq!(
        early.vertices[120..168],
        later.vertices[120..168],
        "the legs do not bob"
    );
}

fn skeleton_texture_images() -> Vec<EntityModelTextureImage> {
    skeleton_entity_texture_refs()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, texture)| EntityModelTextureImage {
            texture,
            rgba: vec![
                u8::try_from(index).unwrap();
                usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap()
            ],
        })
        .collect()
}
