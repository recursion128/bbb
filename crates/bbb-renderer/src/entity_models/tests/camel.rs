use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn camel_model_cubes_and_poses_match_vanilla_26_1_body_layers() {
    // Adult `AdultCamelModel.createBodyLayer`: body carries [hump, tail, head], head carries
    // [left_ear, right_ear], and the four legs hang off the root in the order
    // [left_hind, right_hind, left_front, right_front]. The tail is a zero-thickness plane. Each
    // unified cube carries the colored tint (`CAMEL_TAN`) and the textured UV.
    assert_eq!(
        ADULT_CAMEL_TAIL[0],
        ModelCube::new(
            [-1.5, 0.0, 0.0],
            [3.0, 14.0, 0.0],
            CAMEL_TAN,
            [3.0, 14.0, 0.0],
            [122.0, 0.0],
            false,
        )
    );
    assert_eq!(ADULT_CAMEL_BODY_POSE.offset, [0.0, 4.0, 9.5]);
    assert_eq!(ADULT_CAMEL_BODY[0].min, [-7.5, -12.0, -23.5]);
    assert_eq!(ADULT_CAMEL_HUMP_POSE.offset, [0.0, -12.0, -10.0]);
    assert_eq!(ADULT_CAMEL_TAIL_POSE.offset, [0.0, -9.0, 3.5]);
    assert_eq!(ADULT_CAMEL_HEAD_POSE.offset, [0.0, -3.0, -19.5]);
    assert_eq!(ADULT_CAMEL_HEAD.len(), 3);
    assert_eq!(ADULT_CAMEL_LEFT_EAR_POSE.offset, [2.5, -21.0, -9.5]);
    assert_eq!(ADULT_CAMEL_RIGHT_EAR_POSE.offset, [-2.5, -21.0, -9.5]);
    for (pose, offset, cube) in [
        (
            ADULT_CAMEL_LEFT_HIND_LEG_POSE,
            [4.9, 1.0, 9.5],
            ADULT_CAMEL_LEFT_HIND_LEG[0],
        ),
        (
            ADULT_CAMEL_RIGHT_HIND_LEG_POSE,
            [-4.9, 1.0, 9.5],
            ADULT_CAMEL_RIGHT_HIND_LEG[0],
        ),
        (
            ADULT_CAMEL_LEFT_FRONT_LEG_POSE,
            [4.9, 1.0, -10.5],
            ADULT_CAMEL_LEFT_FRONT_LEG[0],
        ),
        (
            ADULT_CAMEL_RIGHT_FRONT_LEG_POSE,
            [-4.9, 1.0, -10.5],
            ADULT_CAMEL_RIGHT_FRONT_LEG[0],
        ),
    ] {
        assert_eq!(pose.offset, offset);
        assert_eq!(cube.size, [5.0, 21.0, 5.0]);
    }

    // Baby `BabyCamelModel.createBodyLayer`: body carries [tail, head], head carries
    // [right_ear, left_ear], and the four legs hang off the root in the order
    // [right_front, left_front, left_hind, right_hind].
    assert_eq!(
        BABY_CAMEL_TAIL[0],
        ModelCube::new(
            [-1.5, -0.5, 0.0],
            [3.0, 9.0, 0.0],
            CAMEL_TAN,
            [3.0, 9.0, 0.0],
            [50.0, 38.0],
            false,
        )
    );
    assert_eq!(BABY_CAMEL_BODY_POSE.offset, [0.0, 7.0, 0.0]);
    assert_eq!(BABY_CAMEL_BODY[0].min, [-4.5, -4.0, -8.0]);
    assert_eq!(BABY_CAMEL_TAIL_POSE.offset, [0.0, -1.5, 8.05]);
    assert_eq!(BABY_CAMEL_HEAD_POSE.offset, [0.0, 1.0, -7.5]);
    assert_eq!(BABY_CAMEL_RIGHT_EAR_POSE.offset, [-2.5, -11.0, -4.0]);
    assert_eq!(BABY_CAMEL_LEFT_EAR_POSE.offset, [2.5, -11.0, -4.0]);
    for (pose, offset, cube) in [
        (
            BABY_CAMEL_RIGHT_FRONT_LEG_POSE,
            [-3.0, 11.5, -5.5],
            BABY_CAMEL_RIGHT_FRONT_LEG[0],
        ),
        (
            BABY_CAMEL_LEFT_FRONT_LEG_POSE,
            [3.0, 11.5, -5.5],
            BABY_CAMEL_LEFT_FRONT_LEG[0],
        ),
        (
            BABY_CAMEL_LEFT_HIND_LEG_POSE,
            [3.0, 11.5, 5.5],
            BABY_CAMEL_LEFT_HIND_LEG[0],
        ),
        (
            BABY_CAMEL_RIGHT_HIND_LEG_POSE,
            [-3.0, 11.5, 5.5],
            BABY_CAMEL_RIGHT_HIND_LEG[0],
        ),
    ] {
        assert_eq!(pose.offset, offset);
        assert_eq!(cube.size, [3.0, 13.0, 3.0]);
    }
}

#[test]
fn camel_meshes_use_vanilla_body_layer_geometry() {
    let adult = entity_model_mesh(&[EntityModelInstance::camel(
        180,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::Camel,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CAMEL_TAN, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::camel(
        181,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::Camel,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 66);
    assert_eq!(baby.vertices.len(), 264);
    assert_eq!(baby.indices.len(), 396);

    let husk = entity_model_mesh(&[EntityModelInstance::camel(
        182,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::CamelHusk,
        true,
    )]);
    assert_eq!(husk.opaque_faces, 72);
    assert_same_geometry(&husk, &adult);
    assert!(husk
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CAMEL_HUSK_BROWN, 0.78)));

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn camel_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            CamelModelFamily::Camel,
            false,
            "camel",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel.png",
                size: [128, 128],
            },
        ),
        (
            CamelModelFamily::Camel,
            true,
            "camel_baby",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel_baby.png",
                size: [64, 64],
            },
        ),
        (
            CamelModelFamily::CamelHusk,
            false,
            "camel_husk",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel_husk.png",
                size: [128, 128],
            },
        ),
        (
            CamelModelFamily::CamelHusk,
            true,
            "camel_husk",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel_husk.png",
                size: [128, 128],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Camel { family, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
    assert_eq!(
        CAMEL_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/camel_saddle/saddle.png",
            size: [128, 128],
        }
    );
    assert_eq!(
        CAMEL_HUSK_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/camel_husk_saddle/saddle.png",
            size: [128, 128],
        }
    );
    assert!(entity_model_texture_refs().contains(&CAMEL_SADDLE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&CAMEL_HUSK_SADDLE_TEXTURE_REF));
}

#[test]
fn camel_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = camel_textured_layer_passes(CamelModelFamily::Camel, false);
    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::CamelBase);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].model_layer, MODEL_LAYER_CAMEL);
    assert_eq!(adult[0].texture, CAMEL_TEXTURE_REF);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].order, adult[0].submit_sequence), (0, 0));

    let baby = camel_textured_layer_passes(CamelModelFamily::Camel, true);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby[0].model_layer, MODEL_LAYER_CAMEL_BABY);
    assert_eq!(baby[0].texture, CAMEL_BABY_TEXTURE_REF);

    // The camel husk shares the adult mesh/layer; only the texture differs, and it is
    // never a baby (the husk renderer is adult-only), so the age flag must not change it.
    let husk = camel_textured_layer_passes(CamelModelFamily::CamelHusk, false);
    assert_eq!(
        husk[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(husk[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(husk[0].model_layer, MODEL_LAYER_CAMEL);
    assert_eq!(husk[0].texture, CAMEL_HUSK_TEXTURE_REF);
    let husk_baby = camel_textured_layer_passes(CamelModelFamily::CamelHusk, true);
    assert_eq!(husk_baby[0].model_layer, MODEL_LAYER_CAMEL);
    assert_eq!(husk_baby[0].texture, CAMEL_HUSK_TEXTURE_REF);
}

#[test]
fn camel_cubes_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_CAMEL, "minecraft:camel#main");
    assert_eq!(MODEL_LAYER_CAMEL_BABY, "minecraft:camel_baby#main");

    // Adult `AdultCamelModel.createBodyMesh` (atlas 128×128): body, hump, the
    // zero-thickness tail plane, the three head cubes, the two ears, and four legs each
    // with a distinct `texOffs`. Each unified cube carries the colored tint and the textured UV;
    // `uv_size == size` and no cube mirrors.
    assert_eq!(
        ADULT_CAMEL_BODY[0],
        ModelCube::new(
            [-7.5, -12.0, -23.5],
            [15.0, 12.0, 27.0],
            CAMEL_TAN,
            [15.0, 12.0, 27.0],
            [0.0, 25.0],
            false,
        )
    );
    assert_eq!(ADULT_CAMEL_HUMP[0].tex, [74.0, 0.0]);
    assert_eq!(ADULT_CAMEL_TAIL[0].tex, [122.0, 0.0]);
    assert_eq!(ADULT_CAMEL_HEAD[0].tex, [60.0, 24.0]);
    assert_eq!(ADULT_CAMEL_HEAD[1].tex, [21.0, 0.0]);
    assert_eq!(ADULT_CAMEL_HEAD[2].tex, [50.0, 0.0]);
    assert_eq!(ADULT_CAMEL_LEFT_EAR[0].tex, [45.0, 0.0]);
    assert_eq!(ADULT_CAMEL_RIGHT_EAR[0].tex, [67.0, 0.0]);
    assert_eq!(ADULT_CAMEL_LEFT_HIND_LEG[0].tex, [58.0, 16.0]);
    assert_eq!(ADULT_CAMEL_RIGHT_HIND_LEG[0].tex, [94.0, 16.0]);
    assert_eq!(ADULT_CAMEL_LEFT_FRONT_LEG[0].tex, [0.0, 0.0]);
    assert_eq!(ADULT_CAMEL_RIGHT_FRONT_LEG[0].tex, [0.0, 26.0]);

    // Baby `BabyCamelModel.createBodyLayer` (atlas 64×64): four legs with distinct
    // `texOffs`, and the tail plane / head cubes at the baby offsets.
    assert_eq!(BABY_CAMEL_BODY[0].tex, [0.0, 14.0]);
    assert_eq!(BABY_CAMEL_TAIL[0].size, [3.0, 9.0, 0.0]);
    assert_eq!(BABY_CAMEL_HEAD[0].tex, [20.0, 0.0]);
    assert_eq!(BABY_CAMEL_HEAD[1].tex, [0.0, 0.0]);
    assert_eq!(BABY_CAMEL_HEAD[2].tex, [0.0, 14.0]);
    assert_eq!(BABY_CAMEL_RIGHT_FRONT_LEG[0].tex, [36.0, 14.0]);
    assert_eq!(BABY_CAMEL_LEFT_FRONT_LEG[0].tex, [48.0, 14.0]);
    assert_eq!(BABY_CAMEL_LEFT_HIND_LEG[0].tex, [12.0, 38.0]);
    assert_eq!(BABY_CAMEL_RIGHT_HIND_LEG[0].tex, [0.0, 38.0]);

    // No cube mirrors and `uv_size` equals the geometry size, for every camel cube.
    for cube in ADULT_CAMEL_BODY
        .iter()
        .chain(ADULT_CAMEL_HUMP.iter())
        .chain(ADULT_CAMEL_TAIL.iter())
        .chain(ADULT_CAMEL_HEAD.iter())
        .chain(ADULT_CAMEL_LEFT_EAR.iter())
        .chain(ADULT_CAMEL_RIGHT_EAR.iter())
        .chain(ADULT_CAMEL_LEFT_HIND_LEG.iter())
        .chain(BABY_CAMEL_BODY.iter())
        .chain(BABY_CAMEL_TAIL.iter())
        .chain(BABY_CAMEL_HEAD.iter())
        .chain(BABY_CAMEL_RIGHT_FRONT_LEG.iter())
    {
        assert_eq!(cube.uv_size, cube.size);
        assert!(!cube.mirror);
    }
}

#[test]
fn camel_saddle_model_parts_match_vanilla_layer_sources() {
    // Vanilla `CamelSaddleModel.createSaddleLayer()` starts from the adult camel body mesh and appends
    // saddle, reins, and bridle children. Inflated boxes use `CubeDeformation(0.05F)`, while the reins
    // planes and two mouth cubes use no deformation.
    assert_eq!(ADULT_CAMEL_SADDLE[0].min, [-4.55, -17.05, -15.55]);
    assert_eq!(ADULT_CAMEL_SADDLE[0].size, [9.1, 5.1, 11.1]);
    assert_eq!(ADULT_CAMEL_SADDLE[0].uv_size, [9.0, 5.0, 11.0]);
    assert_eq!(ADULT_CAMEL_SADDLE[0].tex, [74.0, 64.0]);
    assert_eq!(ADULT_CAMEL_SADDLE[1].tex, [92.0, 114.0]);
    assert_eq!(ADULT_CAMEL_SADDLE[2].tex, [0.0, 89.0]);

    assert_eq!(ADULT_CAMEL_REINS[0].min, [3.51, -18.0, -17.0]);
    assert_eq!(ADULT_CAMEL_REINS[0].size, [0.0, 7.0, 15.0]);
    assert_eq!(ADULT_CAMEL_REINS[1].size, [7.0, 7.0, 0.0]);
    assert_eq!(ADULT_CAMEL_REINS[1].tex, [84.0, 57.0]);
    assert_eq!(ADULT_CAMEL_REINS[2].tex, [98.0, 42.0]);

    assert_eq!(ADULT_CAMEL_BRIDLE[0].min, [-3.55, -7.05, -15.05]);
    assert_eq!(ADULT_CAMEL_BRIDLE[0].size, [7.1, 8.1, 19.1]);
    assert_eq!(ADULT_CAMEL_BRIDLE[1].tex, [21.0, 64.0]);
    assert_eq!(ADULT_CAMEL_BRIDLE[2].tex, [50.0, 64.0]);
    assert_eq!(ADULT_CAMEL_BRIDLE[3].size, [1.0, 2.0, 2.0]);
    assert!(!ADULT_CAMEL_BRIDLE[3].mirror);
    assert!(ADULT_CAMEL_BRIDLE[4].mirror);
}

#[test]
fn camel_textured_mesh_matches_static_vanilla_pose() {
    // Vanilla `CamelModel.setupAnim` drives the limbs via baked `KeyframeAnimation`s plus a
    // direct head clamp. The textured meshes carry the full body-layer geometry (12 adult cubes /
    // 11 baby cubes, 24 vertices each); the adult/husk walk is reproduced (exercised below), and the
    // head look is exercised separately.
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let adult =
        EntityModelInstance::camel(700, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let baby =
        EntityModelInstance::camel(701, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, true);
    let husk = EntityModelInstance::camel(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::CamelHusk,
        true,
    );

    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    let husk_meshes = entity_model_textured_meshes(&[husk], &atlas);
    assert_camel_submissions_match_vanilla(&adult_meshes, adult);
    assert_camel_submissions_match_vanilla(&baby_meshes, baby);
    assert_camel_submissions_match_vanilla(&husk_meshes, husk);
    assert_eq!(adult_meshes.cutout.vertices.len(), 288);
    assert_eq!(baby_meshes.cutout.vertices.len(), 264);
    // The husk reuses the adult mesh (adult-only renderer); only its sampled texels differ.
    assert_eq!(husk_meshes.cutout.vertices.len(), 288);
    assert_eq!(
        husk_meshes
            .cutout
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>(),
        adult_meshes
            .cutout
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>()
    );

    // The adult/husk walk is reproduced on the textured path: a still camel (walk speed 0) matches
    // the rest pose, while a walking camel (speed > 0) differs.
    let still_instance = adult.with_walk_animation(0.0, 0.0);
    let walking_instance = adult.with_walk_animation(5.0, 1.0);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_camel_submissions_match_vanilla(&still, still_instance);
    assert_camel_submissions_match_vanilla(&walking, walking_instance);
    assert_eq!(adult_meshes.cutout.vertices, still.cutout.vertices);
    assert_eq!(
        adult_meshes.cutout.vertices.len(),
        walking.cutout.vertices.len()
    );
    assert_ne!(adult_meshes.cutout.vertices, walking.cutout.vertices);
}

#[test]
fn camel_saddle_layer_renders_for_adult_camel_and_husk_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        CAMEL_TEXTURE_REF,
        CAMEL_BABY_TEXTURE_REF,
        CAMEL_HUSK_TEXTURE_REF,
        CAMEL_SADDLE_TEXTURE_REF,
        CAMEL_HUSK_SADDLE_TEXTURE_REF,
    ]))
    .unwrap();

    let adult =
        EntityModelInstance::camel(760, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false)
            .with_light_coords((5_u32 << 4) | (11_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult.with_camel_saddle(true);
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    assert_camel_submissions_match_vanilla(&bare_meshes, adult);
    assert_camel_submissions_match_vanilla(&saddled_meshes, saddled_instance);
    let bare = &bare_meshes.cutout;
    let saddled = &saddled_meshes.cutout;
    assert_eq!(saddled_meshes.submissions.len(), 2);
    let saddle_submit = saddled_meshes.submissions[1];
    assert_eq!(
        saddle_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        saddle_submit.render_type.vanilla_name(),
        "armorCutoutNoCull"
    );
    assert_eq!(saddle_submit.texture, CAMEL_SADDLE_TEXTURE_REF);
    assert_eq!(saddle_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((saddle_submit.order, saddle_submit.submit_sequence), (0, 1));
    assert_eq!(
        saddle_submit.transform,
        entity_model_root_transform(saddled_instance)
    );
    assert_ne!(saddled_instance.render_state.overlay_coords(), [0.0, 10.0]);
    assert_eq!(saddled.cutout_faces - bare.cutout_faces, 120);
    assert_eq!(saddled.vertices.len() - bare.vertices.len(), 480);

    let ridden_instance = adult.with_camel_saddle(true).with_camel_saddle_ridden(true);
    let ridden_meshes = entity_model_textured_meshes(&[ridden_instance], &atlas);
    assert_camel_submissions_match_vanilla(&ridden_meshes, ridden_instance);
    assert_eq!(ridden_meshes.cutout.cutout_faces - saddled.cutout_faces, 18);
    assert_eq!(
        ridden_meshes.cutout.vertices.len() - saddled.vertices.len(),
        72
    );

    let saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == CAMEL_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_saddle_vertex = saddled.vertices[bare.vertices.len()].uv;
    assert!(first_saddle_vertex[0] >= saddle_uv.min[0]);
    assert!(first_saddle_vertex[0] <= saddle_uv.max[0]);
    assert!(first_saddle_vertex[1] >= saddle_uv.min[1]);
    assert!(first_saddle_vertex[1] <= saddle_uv.max[1]);

    let baby =
        EntityModelInstance::camel(761, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, true)
            .with_light_coords((5_u32 << 4) | (11_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true)
            .with_camel_saddle(true)
            .with_camel_saddle_ridden(true);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_camel_submissions_match_vanilla(&baby_meshes, baby);
    assert_eq!(
        baby_meshes.cutout.vertices.len(),
        264,
        "vanilla supplies no baby model for the camel saddle layer"
    );

    let husk = EntityModelInstance::camel(
        762,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::CamelHusk,
        false,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_camel_saddle(true);
    let husk_meshes = entity_model_textured_meshes(&[husk], &atlas);
    assert_camel_submissions_match_vanilla(&husk_meshes, husk);
    let husk_mesh = &husk_meshes.cutout;
    assert_eq!(husk_meshes.submissions.len(), 2);
    let husk_saddle_submit = husk_meshes.submissions[1];
    assert_eq!(
        husk_saddle_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        husk_saddle_submit.render_type.vanilla_name(),
        "armorCutoutNoCull"
    );
    assert_eq!(husk_saddle_submit.texture, CAMEL_HUSK_SADDLE_TEXTURE_REF);
    assert_eq!(
        (husk_saddle_submit.order, husk_saddle_submit.submit_sequence),
        (0, 1)
    );
    assert_eq!(
        husk_saddle_submit.transform,
        entity_model_root_transform(husk)
    );
    assert_eq!(husk_mesh.cutout_faces - bare.cutout_faces, 120);
    let husk_saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == CAMEL_HUSK_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_husk_saddle_vertex = husk_mesh.vertices[bare.vertices.len()].uv;
    assert!(first_husk_saddle_vertex[0] >= husk_saddle_uv.min[0]);
    assert!(first_husk_saddle_vertex[0] <= husk_saddle_uv.max[0]);
    assert!(first_husk_saddle_vertex[1] >= husk_saddle_uv.min[1]);
    assert!(first_husk_saddle_vertex[1] <= husk_saddle_uv.max[1]);
}

/// The adult camel's depth-first emit order: body `[0, 24)`, hump `[24, 48)`, the zero-thickness
/// tail plane `[48, 72)`, the three head cubes and two ears `[72, 192)`, then the four legs
/// `[192, 288)`. The head sits nested under the body, so a head look turns only `[72, 192)`.
const ADULT_CAMEL_HEAD_VERTEX_RANGE: std::ops::Range<usize> = 72..192;

#[test]
fn camel_head_look_turns_only_the_nested_head_subtree() {
    // Vanilla `CamelModel.applyHeadRotation` drives `head.yRot/xRot` from the clamped look. The
    // head is `body.getChild("head")`, so the body, hump, tail, and legs stay put while the head
    // cubes and their ear children turn. This must hold on both the colored and textured paths.
    let head = ADULT_CAMEL_HEAD_VERTEX_RANGE;
    let rest =
        EntityModelInstance::camel(710, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let looked = rest.with_head_look(40.0, -20.0);

    let rest_colored = entity_model_mesh(&[rest]);
    let looked_colored = entity_model_mesh(&[looked]);
    assert_eq!(rest_colored.vertices.len(), looked_colored.vertices.len());
    assert_eq!(
        rest_colored.vertices[..head.start],
        looked_colored.vertices[..head.start],
        "the body/hump/tail stay put"
    );
    assert_ne!(
        rest_colored.vertices[head.clone()],
        looked_colored.vertices[head.clone()],
        "the nested head subtree turns"
    );
    assert_eq!(
        rest_colored.vertices[head.end..],
        looked_colored.vertices[head.end..],
        "the legs stay put"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let rest_textured = entity_model_textured_meshes(&[rest], &atlas);
    let looked_textured = entity_model_textured_meshes(&[looked], &atlas);
    assert_camel_submissions_match_vanilla(&rest_textured, rest);
    assert_camel_submissions_match_vanilla(&looked_textured, looked);
    assert_eq!(
        rest_textured.cutout.vertices.len(),
        looked_textured.cutout.vertices.len()
    );
    assert_eq!(
        rest_textured.cutout.vertices[..head.start],
        looked_textured.cutout.vertices[..head.start],
        "the body/hump/tail stay put"
    );
    assert_ne!(
        rest_textured.cutout.vertices[head.clone()],
        looked_textured.cutout.vertices[head.clone()],
        "the nested head subtree turns"
    );
    assert_eq!(
        rest_textured.cutout.vertices[head.end..],
        looked_textured.cutout.vertices[head.end..],
        "the legs stay put"
    );
}

#[test]
fn camel_walk_animation_matches_vanilla_definition() {
    // Vanilla `CamelAnimation.CAMEL_WALK`: 1.5 s looping, animating the root (whole-model roll), the
    // head, the four legs (rotation + position), the two ears, and the tail — nine bones, 51 keyframes.
    assert_eq!(CAMEL_WALK.length_seconds, 1.5);
    assert!(CAMEL_WALK.looping);
    assert_eq!(CAMEL_WALK.bones.len(), 9);
    let keyframes: usize = CAMEL_WALK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 51);

    // The root rolls the whole model: `degreeVec(0, 0, 2.5)` at t=0.
    let (_, root_rot) = sample_bone_offsets(&CAMEL_WALK, "root", 0.0, 1.0);
    assert!((root_rot[2] - 2.5_f32.to_radians()).abs() < 1.0e-4);

    // The front legs start a half-cycle apart: right `+22.5°`, left `-22.5°` at t=0.
    let (_, rfl_rot) = sample_bone_offsets(&CAMEL_WALK, "right_front_leg", 0.0, 1.0);
    let (_, lfl_rot) = sample_bone_offsets(&CAMEL_WALK, "left_front_leg", 0.0, 1.0);
    assert!((rfl_rot[0] - 22.5_f32.to_radians()).abs() < 1.0e-4);
    assert!((lfl_rot[0] - (-22.5_f32).to_radians()).abs() < 1.0e-4);
}

#[test]
fn camel_walk_moves_the_whole_model_and_composes_with_the_look() {
    // A still adult camel (walk speed 0) samples the cycle at amplitude 0, collapsing to the bind pose;
    // a walking camel samples CAMEL_WALK — and the `root` roll turns the entire model, so the body and
    // legs move too. The vertex count is preserved.
    let still = entity_model_mesh(&[EntityModelInstance::camel(
        720,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::Camel,
        false,
    )]);
    let walking = entity_model_mesh(&[EntityModelInstance::camel(
        721,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::Camel,
        false,
    )
    .with_walk_animation(5.0, 1.0)]);
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking camel rolls its whole body and swings its legs"
    );

    // The head walk pitch ADDS onto the clamped look, so a walking + looking camel differs from one
    // that only walks ONLY across the nested head subtree [72, 192); the body, tail, and legs share the
    // same walk (they don't depend on the head look).
    let head = ADULT_CAMEL_HEAD_VERTEX_RANGE;
    let walking_looking = entity_model_mesh(&[EntityModelInstance::camel(
        722,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::Camel,
        false,
    )
    .with_walk_animation(5.0, 1.0)
    .with_head_look(40.0, -20.0)]);
    assert_ne!(
        walking.vertices[head.clone()],
        walking_looking.vertices[head.clone()],
        "the look composes onto the walking head"
    );
    assert_eq!(
        walking.vertices[..head.start],
        walking_looking.vertices[..head.start],
        "the body/hump/tail share the same walk regardless of the look"
    );
    assert_eq!(
        walking.vertices[head.end..],
        walking_looking.vertices[head.end..],
        "the legs share the same walk regardless of the look"
    );
}

#[test]
fn camel_textured_walk_moves_the_whole_model_and_composes_with_the_look() {
    // The textured path reproduces the same CAMEL_WALK as the colored path: the `root` roll turns the
    // whole model and the head walk pitch ADDS onto the clamped look (only the nested head subtree
    // [72, 192) tracks the look; the body and legs share the walk).
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let head = ADULT_CAMEL_HEAD_VERTEX_RANGE;
    let still =
        EntityModelInstance::camel(730, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let walking = still.with_walk_animation(5.0, 1.0);
    let walking_looking = still
        .with_walk_animation(5.0, 1.0)
        .with_head_look(40.0, -20.0);

    let still_mesh = entity_model_textured_meshes(&[still], &atlas);
    let walking_mesh = entity_model_textured_meshes(&[walking], &atlas);
    assert_camel_submissions_match_vanilla(&still_mesh, still);
    assert_camel_submissions_match_vanilla(&walking_mesh, walking);
    assert_eq!(
        still_mesh.cutout.vertices.len(),
        walking_mesh.cutout.vertices.len()
    );
    assert_ne!(
        still_mesh.cutout.vertices, walking_mesh.cutout.vertices,
        "the walking camel rolls its whole body and swings its legs"
    );

    let walking_looking_mesh = entity_model_textured_meshes(&[walking_looking], &atlas);
    assert_camel_submissions_match_vanilla(&walking_looking_mesh, walking_looking);
    assert_ne!(
        walking_mesh.cutout.vertices[head.clone()],
        walking_looking_mesh.cutout.vertices[head.clone()],
        "the look composes onto the walking head"
    );
    assert_eq!(
        walking_mesh.cutout.vertices[..head.start],
        walking_looking_mesh.cutout.vertices[..head.start],
        "the body/hump/tail share the same walk regardless of the look"
    );
    assert_eq!(
        walking_mesh.cutout.vertices[head.end..],
        walking_looking_mesh.cutout.vertices[head.end..],
        "the legs share the same walk regardless of the look"
    );
}

#[test]
fn camel_baby_walk_animation_matches_vanilla_definition() {
    // Vanilla `CamelBabyAnimation.CAMEL_BABY_WALK`: 1.5 s looping, animating the root, the head
    // (rotation + position), the four legs (rotation + position), the two ears, the tail, and a `body`
    // y-dip the adult lacks — ten bones, 58 keyframes.
    assert_eq!(CAMEL_BABY_WALK.length_seconds, 1.5);
    assert!(CAMEL_BABY_WALK.looping);
    assert_eq!(CAMEL_BABY_WALK.bones.len(), 10);
    let keyframes: usize = CAMEL_BABY_WALK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 58);

    // The root rolls the whole model (`degreeVec(0, 0, 2.5)` at t=0) and the baby body dips
    // (`posVec(0, -0.6, 0)` → y negated to +0.6).
    let (_, root_rot) = sample_bone_offsets(&CAMEL_BABY_WALK, "root", 0.0, 1.0);
    assert!((root_rot[2] - 2.5_f32.to_radians()).abs() < 1.0e-4);
    let (body_pos, _) = sample_bone_offsets(&CAMEL_BABY_WALK, "body", 0.0, 1.0);
    assert!((body_pos[1] - 0.6).abs() < 1.0e-4);

    // The front legs start a half-cycle apart: right `-22.5°`, left `+22.5°` at t=0.
    let (_, rfl_rot) = sample_bone_offsets(&CAMEL_BABY_WALK, "right_front_leg", 0.0, 1.0);
    let (_, lfl_rot) = sample_bone_offsets(&CAMEL_BABY_WALK, "left_front_leg", 0.0, 1.0);
    assert!((rfl_rot[0] - (-22.5_f32).to_radians()).abs() < 1.0e-4);
    assert!((lfl_rot[0] - 22.5_f32.to_radians()).abs() < 1.0e-4);
}

/// The baby camel's depth-first emit order: body `[0, 24)`, the zero-thickness tail plane `[24, 48)`,
/// the three head cubes and two ears `[48, 168)`, then the four legs `[168, 264)`. The head sits
/// nested under the body, so a head look turns only `[48, 168)`.
const BABY_CAMEL_HEAD_VERTEX_RANGE: std::ops::Range<usize> = 48..168;

#[test]
fn camel_baby_walk_moves_the_model_and_composes_with_the_look() {
    // The baby camel hand-walks `CAMEL_BABY_WALK` on both paths: a still baby (walk speed 0) collapses
    // to the bind pose, a walking baby rolls/swings, and the head walk pitch ADDS onto the look (only
    // the nested head subtree [48, 168) tracks the look).
    let head = BABY_CAMEL_HEAD_VERTEX_RANGE;
    let still =
        EntityModelInstance::camel(740, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, true);
    let walking = still.with_walk_animation(5.0, 1.0);
    let walking_looking = still
        .with_walk_animation(5.0, 1.0)
        .with_head_look(40.0, -20.0);

    let still_colored = entity_model_mesh(&[still]);
    let walking_colored = entity_model_mesh(&[walking]);
    assert_eq!(still_colored.vertices.len(), walking_colored.vertices.len());
    assert_ne!(
        still_colored.vertices, walking_colored.vertices,
        "the walking baby camel rolls its whole body and swings its legs"
    );
    let walking_looking_colored = entity_model_mesh(&[walking_looking]);
    assert_ne!(
        walking_colored.vertices[head.clone()],
        walking_looking_colored.vertices[head.clone()],
        "the look composes onto the walking baby head"
    );
    assert_eq!(
        walking_colored.vertices[..head.start],
        walking_looking_colored.vertices[..head.start],
        "the body and tail share the same walk regardless of the look"
    );
    assert_eq!(
        walking_colored.vertices[head.end..],
        walking_looking_colored.vertices[head.end..],
        "the legs share the same walk regardless of the look"
    );

    // The textured path reproduces the same baby walk.
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let still_textured = entity_model_textured_meshes(&[still], &atlas);
    let walking_textured = entity_model_textured_meshes(&[walking], &atlas);
    assert_camel_submissions_match_vanilla(&still_textured, still);
    assert_camel_submissions_match_vanilla(&walking_textured, walking);
    assert_eq!(
        still_textured.cutout.vertices.len(),
        walking_textured.cutout.vertices.len()
    );
    assert_ne!(
        still_textured.cutout.vertices, walking_textured.cutout.vertices,
        "the textured baby camel walks too"
    );
}

#[test]
fn camel_head_look_clamps_to_vanilla_range() {
    // Vanilla `CamelModel.applyHeadRotation`: `yRot = clamp(yRot, -30, 30)`,
    // `xRot = clamp(xRot, -25, 45)`, in degrees. Inside the range the angle passes through.
    assert_eq!(camel_clamped_head_look(0.0, 0.0), (0.0, 0.0));
    assert_eq!(camel_clamped_head_look(12.0, 20.0), (12.0, 20.0));
    assert_eq!(camel_clamped_head_look(50.0, 60.0), (30.0, 45.0));
    assert_eq!(camel_clamped_head_look(-50.0, -60.0), (-30.0, -25.0));
}

#[test]
fn camel_sit_and_standup_animations_match_vanilla_definitions() {
    // Vanilla `CamelAnimation`: CAMEL_SIT (2.0 s), CAMEL_SIT_POSE (1.0 s), CAMEL_STANDUP (2.6 s), all
    // NOT looping, each animating seven bones (body, four legs, head, tail — the ears stay still).
    for definition in [&CAMEL_SIT, &CAMEL_SIT_POSE, &CAMEL_STANDUP] {
        assert!(!definition.looping);
        assert_eq!(definition.bones.len(), 7);
    }
    assert_eq!(CAMEL_SIT.length_seconds, 2.0);
    assert_eq!(CAMEL_SIT_POSE.length_seconds, 1.0);
    assert_eq!(CAMEL_STANDUP.length_seconds, 2.6);

    // CAMEL_SIT body pitch rolls back to 0° at the t=2.0 final frame and drops the body to the seated
    // y. Vanilla `posVec` negates y, so the sampled offset for `posVec(0, -19.9, 0)` is +19.9.
    let (body_pos, body_rot) = sample_bone_offsets(&CAMEL_SIT, "body", 2.0, 1.0);
    assert!(
        (body_rot[0] - 0.0_f32).abs() < 1.0e-4,
        "body rolls back to 0 at t=2.0"
    );
    assert!(
        (body_pos[1] - 19.9).abs() < 1.0e-3,
        "body drops to the seated y"
    );

    // CAMEL_SIT_POSE is a constant hold at the seated pose: the body stays at the seated y throughout.
    let (pose_pos, _) = sample_bone_offsets(&CAMEL_SIT_POSE, "body", 0.0, 1.0);
    assert!((pose_pos[1] - 19.9).abs() < 1.0e-3);
    let (pose_pos_end, _) = sample_bone_offsets(&CAMEL_SIT_POSE, "body", 1.0, 1.0);
    assert!((pose_pos_end[1] - 19.9).abs() < 1.0e-3);

    // CAMEL_STANDUP starts at the seated y and returns the body to bind (y 0) by t=2.6.
    let (standup_start, _) = sample_bone_offsets(&CAMEL_STANDUP, "body", 0.0, 1.0);
    assert!((standup_start[1] - 19.9).abs() < 1.0e-3);
    let (standup_end, _) = sample_bone_offsets(&CAMEL_STANDUP, "body", 2.6, 1.0);
    assert!((standup_end[1] - 0.0).abs() < 1.0e-3);
}

#[test]
fn camel_sitting_and_standing_re_pose_the_body_and_legs_vs_the_bind_pose() {
    // Vanilla `CamelModel.setupAnim` applies the sit/sit-pose/stand-up keyframes ADDITIVELY onto the
    // walk pose. A standing camel (all three sentinels -1) sits at the bind pose; a sitting-down camel,
    // a seated camel, and a standing-up camel each re-pose the body and legs differently. This must
    // hold on both the colored and textured paths.
    let bind =
        EntityModelInstance::camel(750, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let sitting_down = bind.with_camel_sit_seconds(1.0);
    let seated = bind.with_camel_sit_pose_seconds(0.5);
    let standing_up = bind.with_camel_standup_seconds(0.5);

    let bind_mesh = entity_model_mesh(&[bind]);
    let sitting_down_mesh = entity_model_mesh(&[sitting_down]);
    let seated_mesh = entity_model_mesh(&[seated]);
    let standing_up_mesh = entity_model_mesh(&[standing_up]);

    // Every pose preserves the vertex count.
    assert_eq!(bind_mesh.vertices.len(), sitting_down_mesh.vertices.len());
    assert_eq!(bind_mesh.vertices.len(), seated_mesh.vertices.len());
    assert_eq!(bind_mesh.vertices.len(), standing_up_mesh.vertices.len());

    // Each sit/stand pose differs from the bind pose, and the three differ from each other.
    assert_ne!(
        bind_mesh.vertices, sitting_down_mesh.vertices,
        "the sitting-down camel folds down off the bind pose"
    );
    assert_ne!(
        bind_mesh.vertices, seated_mesh.vertices,
        "the seated camel holds a folded pose off the bind pose"
    );
    assert_ne!(
        bind_mesh.vertices, standing_up_mesh.vertices,
        "the standing-up camel unfolds off the bind pose"
    );
    assert_ne!(
        sitting_down_mesh.vertices, standing_up_mesh.vertices,
        "sitting down and standing up pose the camel differently"
    );

    // The textured path reproduces the same sit/stand re-pose.
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let bind_textured = entity_model_textured_meshes(&[bind], &atlas);
    let seated_textured = entity_model_textured_meshes(&[seated], &atlas);
    assert_camel_submissions_match_vanilla(&bind_textured, bind);
    assert_camel_submissions_match_vanilla(&seated_textured, seated);
    assert_eq!(
        bind_textured.cutout.vertices.len(),
        seated_textured.cutout.vertices.len()
    );
    assert_ne!(
        bind_textured.cutout.vertices, seated_textured.cutout.vertices,
        "the seated camel re-poses on the textured path too"
    );
}

fn assert_camel_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_camel_folded_meshes_are_cutout_only(meshes);
    let (family, baby) = match instance.kind {
        EntityModelKind::Camel { family, baby } => (family, baby),
        _ => panic!("expected camel instance"),
    };
    let base_passes = camel_textured_layer_passes(family, baby);
    let mut expected = Vec::new();
    expected.extend(base_passes.iter().map(|pass| {
        (
            pass.render_type,
            pass.texture,
            pass.tint,
            pass.order,
            pass.submit_sequence,
        )
    }));
    if instance.render_state.camel_saddle {
        match (family, baby) {
            (CamelModelFamily::Camel, false) => expected.push((
                EntityModelLayerRenderType::ArmorCutoutNoCull,
                CAMEL_SADDLE_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
                0,
                1,
            )),
            (CamelModelFamily::CamelHusk, _) => expected.push((
                EntityModelLayerRenderType::ArmorCutoutNoCull,
                CAMEL_HUSK_SADDLE_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
                0,
                1,
            )),
            (CamelModelFamily::Camel, true) => {}
        }
    }

    assert_eq!(meshes.submissions.len(), expected.len());
    for (submit, (render_type, texture, tint, order, sequence)) in
        meshes.submissions.iter().zip(expected)
    {
        assert_eq!(submit.render_type, render_type);
        let expected_render_type_name = match render_type {
            EntityModelLayerRenderType::EntityCutout => "entityCutout",
            EntityModelLayerRenderType::ArmorCutoutNoCull => "armorCutoutNoCull",
            _ => panic!("unexpected camel render type"),
        };
        assert_eq!(submit.render_type.vanilla_name(), expected_render_type_name);
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, tint);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!((submit.order, submit.submit_sequence), (order, sequence));
        assert_eq!(submit.light, instance.render_state.shader_light());
        assert_eq!(
            submit.overlay,
            match render_type {
                EntityModelLayerRenderType::EntityCutout => instance.render_state.overlay_coords(),
                EntityModelLayerRenderType::ArmorCutoutNoCull => [0.0, 10.0],
                _ => panic!("unexpected camel render type"),
            }
        );
    }

    let base_vertex_count = match (family, baby) {
        (CamelModelFamily::Camel, true) => 264,
        _ => 288,
    };
    let base_submit = meshes.submissions[0];
    assert!(meshes.cutout.vertices[..base_vertex_count]
        .iter()
        .all(|vertex| vertex.light == base_submit.light && vertex.overlay == base_submit.overlay));
    if let Some(saddle_submit) = meshes.submissions.get(1) {
        assert!(meshes.cutout.vertices[base_vertex_count..]
            .iter()
            .all(|vertex| vertex.light == saddle_submit.light
                && vertex.overlay == saddle_submit.overlay));
    }
}

fn assert_camel_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn camel_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(camel_entity_texture_refs())
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn camel_dash_seconds_add_the_gallop_pose() {
    use crate::entity_models::model::EntityModel;

    // Vanilla `CamelModel.setupAnim` applies `CAMEL_DASH` ADDITIVELY over the walk pose. With the camel
    // at rest (no walk speed), the dash offsets are the only motion: the ears flare a constant ±67.5°
    // yaw and the body pitches up a constant 5° (both Linear, so the sample at any time is exact).
    let base =
        EntityModelInstance::camel(700, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);

    let mut resting = CamelModel::new(CamelModelFamily::Camel, false);
    resting.prepare(&base);
    let rest_left_ear_yaw = resting
        .root_mut()
        .child_mut("body")
        .child_mut("head")
        .child_mut("left_ear")
        .pose
        .rotation[1];
    let rest_body_pitch = resting.root_mut().child_mut("body").pose.rotation[0];

    let mut dashing = CamelModel::new(CamelModelFamily::Camel, false);
    dashing.prepare(&base.with_camel_dash_seconds(0.0));
    let dash_left_ear_yaw = dashing
        .root_mut()
        .child_mut("body")
        .child_mut("head")
        .child_mut("left_ear")
        .pose
        .rotation[1];
    let dash_body_pitch = dashing.root_mut().child_mut("body").pose.rotation[0];

    assert!(
        (dash_left_ear_yaw - rest_left_ear_yaw - (-67.5_f32).to_radians()).abs() < 1.0e-5,
        "dash flares the left ear by -67.5° yaw: {rest_left_ear_yaw} -> {dash_left_ear_yaw}"
    );
    assert!(
        (dash_body_pitch - rest_body_pitch - 5.0_f32.to_radians()).abs() < 1.0e-5,
        "dash pitches the body up 5°: {rest_body_pitch} -> {dash_body_pitch}"
    );
}
