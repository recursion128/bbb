use super::*;

#[test]
fn camel_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_CAMEL_TAIL[0],
        ModelCubeDesc {
            min: [-1.5, 0.0, 0.0],
            size: [3.0, 14.0, 0.0],
            color: CAMEL_TAN,
        }
    );
    assert_eq!(ADULT_CAMEL_PARTS.len(), 5);
    assert_part_tree(
        &ADULT_CAMEL_PARTS[0],
        [0.0, 4.0, 9.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_BODY.as_slice(),
        ADULT_CAMEL_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_BODY_CHILDREN[0],
        [0.0, -12.0, -10.0],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_HUMP.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_BODY_CHILDREN[1],
        [0.0, -9.0, 3.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_TAIL.as_slice(),
    );
    assert_part_tree(
        &ADULT_CAMEL_BODY_CHILDREN[2],
        [0.0, -3.0, -19.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_HEAD.as_slice(),
        ADULT_CAMEL_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_HEAD_CHILDREN[0],
        [2.5, -21.0, -9.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_LEFT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_HEAD_CHILDREN[1],
        [-2.5, -21.0, -9.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_RIGHT_EAR.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_CAMEL_PARTS[1],
            [4.9, 1.0, 9.5],
            ADULT_CAMEL_LEFT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_CAMEL_PARTS[2],
            [-4.9, 1.0, 9.5],
            ADULT_CAMEL_RIGHT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_CAMEL_PARTS[3],
            [4.9, 1.0, -10.5],
            ADULT_CAMEL_LEFT_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_CAMEL_PARTS[4],
            [-4.9, 1.0, -10.5],
            ADULT_CAMEL_RIGHT_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(
        BABY_CAMEL_TAIL[0],
        ModelCubeDesc {
            min: [-1.5, -0.5, 0.0],
            size: [3.0, 9.0, 0.0],
            color: CAMEL_TAN,
        }
    );
    assert_eq!(BABY_CAMEL_PARTS.len(), 5);
    assert_part_tree(
        &BABY_CAMEL_PARTS[0],
        [0.0, 7.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_BODY.as_slice(),
        BABY_CAMEL_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_CAMEL_BODY_CHILDREN[0],
        [0.0, -1.5, 8.05],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_TAIL.as_slice(),
    );
    assert_part_tree(
        &BABY_CAMEL_BODY_CHILDREN[1],
        [0.0, 1.0, -7.5],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_HEAD.as_slice(),
        BABY_CAMEL_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_CAMEL_HEAD_CHILDREN[0],
        [-2.5, -11.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &BABY_CAMEL_HEAD_CHILDREN[1],
        [2.5, -11.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_LEFT_EAR.as_slice(),
    );
    for (part, expected_offset) in [
        (&BABY_CAMEL_PARTS[1], [-3.0, 11.5, -5.5]),
        (&BABY_CAMEL_PARTS[2], [3.0, 11.5, -5.5]),
        (&BABY_CAMEL_PARTS[3], [3.0, 11.5, 5.5]),
        (&BABY_CAMEL_PARTS[4], [-3.0, 11.5, 5.5]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_CAMEL_LEG.as_slice(),
        );
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
}

#[test]
fn camel_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = camel_textured_layer_passes(CamelModelFamily::Camel, false);
    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::CamelBase);
    assert_eq!(adult[0].model_layer, MODEL_LAYER_CAMEL);
    assert_eq!(adult[0].texture, CAMEL_TEXTURE_REF);
    assert_eq!(adult[0].parts, ADULT_CAMEL_TEXTURED_PARTS.as_slice());
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].collector_order, adult[0].submit_sequence), (0, 0));

    let baby = camel_textured_layer_passes(CamelModelFamily::Camel, true);
    assert_eq!(baby[0].model_layer, MODEL_LAYER_CAMEL_BABY);
    assert_eq!(baby[0].texture, CAMEL_BABY_TEXTURE_REF);
    assert_eq!(baby[0].parts, BABY_CAMEL_TEXTURED_PARTS.as_slice());

    // The camel husk shares the adult mesh/layer; only the texture differs, and it is
    // never a baby (the husk renderer is adult-only), so the age flag must not change it.
    let husk = camel_textured_layer_passes(CamelModelFamily::CamelHusk, false);
    assert_eq!(husk[0].model_layer, MODEL_LAYER_CAMEL);
    assert_eq!(husk[0].texture, CAMEL_HUSK_TEXTURE_REF);
    assert_eq!(husk[0].parts, ADULT_CAMEL_TEXTURED_PARTS.as_slice());
    let husk_baby = camel_textured_layer_passes(CamelModelFamily::CamelHusk, true);
    assert_eq!(husk_baby[0].model_layer, MODEL_LAYER_CAMEL);
    assert_eq!(husk_baby[0].texture, CAMEL_HUSK_TEXTURE_REF);
    assert_eq!(husk_baby[0].parts, ADULT_CAMEL_TEXTURED_PARTS.as_slice());
}

#[test]
fn camel_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_CAMEL, "minecraft:camel#main");
    assert_eq!(MODEL_LAYER_CAMEL_BABY, "minecraft:camel_baby#main");

    // Adult `AdultCamelModel.createBodyMesh` (atlas 128×128): body, hump, the
    // zero-thickness tail plane, the three head cubes, the two ears, and four legs each
    // with a distinct `texOffs`.
    assert_eq!(
        ADULT_CAMEL_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-7.5, -12.0, -23.5],
            size: [15.0, 12.0, 27.0],
            uv_size: [15.0, 12.0, 27.0],
            tex: [0.0, 25.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_CAMEL_TEXTURED_HUMP[0].tex, [74.0, 0.0]);
    assert_eq!(
        ADULT_CAMEL_TEXTURED_TAIL[0],
        TexturedModelCubeDesc {
            min: [-1.5, 0.0, 0.0],
            size: [3.0, 14.0, 0.0],
            uv_size: [3.0, 14.0, 0.0],
            tex: [122.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_CAMEL_TEXTURED_HEAD[0].tex, [60.0, 24.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_HEAD[1].tex, [21.0, 0.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_HEAD[2].tex, [50.0, 0.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_LEFT_EAR[0].tex, [45.0, 0.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_RIGHT_EAR[0].tex, [67.0, 0.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_LEFT_HIND_LEG[0].tex, [58.0, 16.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_RIGHT_HIND_LEG[0].tex, [94.0, 16.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_LEFT_FRONT_LEG[0].tex, [0.0, 0.0]);
    assert_eq!(ADULT_CAMEL_TEXTURED_RIGHT_FRONT_LEG[0].tex, [0.0, 26.0]);

    // Adult part tree: body carries hump/tail/head, head carries the two ears.
    assert_eq!(ADULT_CAMEL_TEXTURED_PARTS.len(), 5);
    assert_eq!(
        ADULT_CAMEL_TEXTURED_PARTS[0].pose,
        ADULT_CAMEL_PARTS[0].pose
    );
    assert_eq!(ADULT_CAMEL_TEXTURED_PARTS[0].children.len(), 3);
    assert_eq!(ADULT_CAMEL_TEXTURED_BODY_CHILDREN[2].children.len(), 2);

    // Baby `BabyCamelModel.createBodyLayer` (atlas 64×64): four legs with distinct
    // `texOffs`, and the tail plane / head cubes at the baby offsets.
    assert_eq!(BABY_CAMEL_TEXTURED_BODY[0].tex, [0.0, 14.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_TAIL[0].size, [3.0, 9.0, 0.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_HEAD[0].tex, [20.0, 0.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_HEAD[1].tex, [0.0, 0.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_HEAD[2].tex, [0.0, 14.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_RIGHT_FRONT_LEG[0].tex, [36.0, 14.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_LEFT_FRONT_LEG[0].tex, [48.0, 14.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_LEFT_HIND_LEG[0].tex, [12.0, 38.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_RIGHT_HIND_LEG[0].tex, [0.0, 38.0]);
    assert_eq!(BABY_CAMEL_TEXTURED_PARTS.len(), 5);
    assert_eq!(BABY_CAMEL_TEXTURED_PARTS[0].children.len(), 2);
}

#[test]
fn camel_textured_mesh_matches_static_vanilla_pose() {
    // Vanilla `CamelModel.setupAnim` drives the limbs via baked `KeyframeAnimation`s plus a
    // direct head clamp. The keyframe animations are deferred, so the textured meshes carry the
    // full body-layer geometry (12 adult cubes / 11 baby cubes, 24 vertices each) and are inert
    // under walk animation (the head look is exercised separately below).
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

    let adult_mesh = entity_model_textured_mesh(&[adult], &atlas);
    let baby_mesh = entity_model_textured_mesh(&[baby], &atlas);
    let husk_mesh = entity_model_textured_mesh(&[husk], &atlas);
    assert_eq!(adult_mesh.vertices.len(), 288);
    assert_eq!(baby_mesh.vertices.len(), 264);
    // The husk reuses the adult mesh (adult-only renderer); only its sampled texels differ.
    assert_eq!(husk_mesh.vertices.len(), 288);
    assert_eq!(
        husk_mesh
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>(),
        adult_mesh
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>()
    );

    // The keyframe walk animation is still deferred, so walking is byte-identical to rest.
    let walking = entity_model_textured_mesh(&[adult.with_walk_animation(0.0, 1.0)], &atlas);
    assert_eq!(adult_mesh.vertices, walking.vertices);
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
    let rest_textured = entity_model_textured_mesh(&[rest], &atlas);
    let looked_textured = entity_model_textured_mesh(&[looked], &atlas);
    assert_eq!(rest_textured.vertices.len(), looked_textured.vertices.len());
    assert_eq!(
        rest_textured.vertices[..head.start],
        looked_textured.vertices[..head.start],
        "the body/hump/tail stay put"
    );
    assert_ne!(
        rest_textured.vertices[head.clone()],
        looked_textured.vertices[head.clone()],
        "the nested head subtree turns"
    );
    assert_eq!(
        rest_textured.vertices[head.end..],
        looked_textured.vertices[head.end..],
        "the legs stay put"
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

fn camel_texture_images() -> Vec<EntityModelTextureImage> {
    camel_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
