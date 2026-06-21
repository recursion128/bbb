use super::*;

#[test]
fn wolf_textured_mesh_uses_vanilla_uvs_and_collar_tint() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            305,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            false,
            Some(EntityDyeColor::Blue),
        )],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_close2(mesh.vertices[0].uv, [10.0 / 64.0, 32.0 / 256.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[144].uv, [4.0 / 64.0, 52.0 / 256.0]);
    assert_close2(mesh.vertices[264].uv, [10.0 / 64.0, 192.0 / 256.0]);
    assert_eq!(
        mesh.vertices[264].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );

    let untamed_with_collar_metadata = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            306,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            false,
            false,
            Some(EntityDyeColor::Red),
        )],
        &atlas,
    );
    assert_eq!(untamed_with_collar_metadata.cutout_faces, 66);
    assert!(untamed_with_collar_metadata
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let invisible_tame = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            307,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            true,
            Some(EntityDyeColor::Blue),
        )],
        &atlas,
    );
    assert_eq!(invisible_tame.cutout_faces, 66);
    assert!(invisible_tame
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn wolf_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_WOLF_REAL_HEAD,
        [
            ModelCubeDesc {
                min: [-2.0, -3.0, -2.0],
                size: [6.0, 6.0, 4.0],
                color: WOLF_GRAY,
            },
            ModelCubeDesc {
                min: [-2.0, -5.0, 0.0],
                size: [2.0, 2.0, 1.0],
                color: WOLF_GRAY,
            },
            ModelCubeDesc {
                min: [2.0, -5.0, 0.0],
                size: [2.0, 2.0, 1.0],
                color: WOLF_GRAY,
            },
            ModelCubeDesc {
                min: [-0.5, -0.001, -5.0],
                size: [3.0, 3.0, 4.0],
                color: WOLF_GRAY,
            },
        ]
    );
    assert_eq!(ADULT_WOLF_PARTS.len(), 8);
    assert_part_tree(
        &ADULT_WOLF_PARTS[0],
        [-1.0, 13.5, -7.0],
        [0.0, 0.0, 0.0],
        &[],
        ADULT_WOLF_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_WOLF_REAL_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_PARTS[1],
        [0.0, 14.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_WOLF_BODY.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_PARTS[2],
        [-1.0, 14.0, -3.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_WOLF_UPPER_BODY.as_slice(),
    );
    for (part, expected_offset) in ADULT_WOLF_PARTS[3..7].iter().zip([
        [-2.5, 16.0, 7.0],
        [0.5, 16.0, 7.0],
        [-2.5, 16.0, -4.0],
        [0.5, 16.0, -4.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_WOLF_LEG.as_slice(),
        );
    }
    assert_part_tree(
        &ADULT_WOLF_PARTS[7],
        [-1.0, 12.0, 8.0],
        [0.62831855, 0.0, 0.0],
        &[],
        ADULT_WOLF_TAIL_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_TAIL_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_WOLF_REAL_TAIL.as_slice(),
    );

    assert_eq!(
        BABY_WOLF_HEAD[0],
        ModelCubeDesc {
            min: [-3.015, -3.275, -3.025],
            size: [6.05, 5.05, 5.05],
            color: WOLF_GRAY,
        }
    );
    assert_eq!(BABY_WOLF_PARTS.len(), 7);
    assert_part_tree(
        &BABY_WOLF_PARTS[0],
        [0.0, 18.25, -4.0],
        [0.0, 0.0, 0.0],
        BABY_WOLF_HEAD.as_slice(),
        BABY_WOLF_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_WOLF_HEAD_CHILDREN[0],
        [-2.0, -4.25, -0.5],
        [0.0, 0.0, 0.0],
        BABY_WOLF_EAR.as_slice(),
    );
    assert_part(
        &BABY_WOLF_HEAD_CHILDREN[1],
        [2.0, -4.25, -0.5],
        [0.0, 0.0, 0.0],
        BABY_WOLF_EAR.as_slice(),
    );
    assert_part(
        &BABY_WOLF_PARTS[1],
        [0.0, 19.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_WOLF_BODY.as_slice(),
    );
    for (part, expected_offset) in BABY_WOLF_PARTS[2..6].iter().zip([
        [-1.5, 21.0, 3.0],
        [1.5, 21.0, 3.0],
        [-1.5, 21.0, -3.0],
        [1.5, 21.0, -3.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_WOLF_LEG.as_slice(),
        );
    }
    assert_part_tree(
        &BABY_WOLF_PARTS[6],
        [0.0, 19.0, 3.0],
        [-0.5236, 0.0, 0.0],
        &[],
        BABY_WOLF_TAIL_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_WOLF_TAIL_CHILDREN[0],
        [0.0, -0.6, 0.2],
        [-3.1, 0.0, 0.0],
        BABY_WOLF_TAIL_R1.as_slice(),
    );
}

#[test]
fn wolf_meshes_use_vanilla_body_layer_geometry() {
    let adult = entity_model_mesh(&[EntityModelInstance::wolf(148, [0.0, 64.0, 0.0], 0.0, false)]);

    assert_eq!(adult.opaque_faces, 66);
    assert_eq!(adult.vertices.len(), 264);
    assert_eq!(adult.indices.len(), 396);
    let (adult_min, adult_max) = mesh_extents(&adult);
    assert_close3(adult_min, [-0.25, 64.001, -0.8444562]);
    assert_close3(adult_max, [0.25000006, 64.96975, 0.75]);

    let baby = entity_model_mesh(&[EntityModelInstance::wolf(149, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_eq!(baby.indices.len(), 360);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert_close3(baby_min, [-0.1884375, 63.995087, -0.28114623]);
    assert_close3(baby_max, [0.18968754, 64.6885, 0.5625]);
}

#[test]
fn wolf_texture_refs_match_vanilla_renderer_pale_variant_assets() {
    let cases = [
        (
            false,
            false,
            false,
            "wolf",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf.png",
                size: [64, 32],
            },
        ),
        (
            false,
            true,
            false,
            "wolf_tame",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_tame.png",
                size: [64, 32],
            },
        ),
        (
            false,
            false,
            true,
            "wolf_angry",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_angry.png",
                size: [64, 32],
            },
        ),
        (
            true,
            false,
            false,
            "wolf_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            true,
            false,
            "wolf_tame_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_tame_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            false,
            true,
            "wolf_angry_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_angry_baby.png",
                size: [32, 32],
            },
        ),
    ];
    for (baby, tame, angry, model_key, texture) in cases {
        let kind = EntityModelKind::Wolf {
            baby,
            tame,
            angry,
            invisible: false,
            collar_color: None,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }

    assert_eq!(
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            invisible: false,
            collar_color: Some(EntityDyeColor::Red),
        }
        .vanilla_layer_texture_refs(),
        &[WOLF_COLLAR_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::Wolf {
            baby: true,
            tame: true,
            angry: false,
            invisible: false,
            collar_color: Some(EntityDyeColor::Red),
        }
        .vanilla_layer_texture_refs(),
        &[WOLF_BABY_COLLAR_TEXTURE_REF]
    );
    assert!(EntityModelKind::Wolf {
        baby: false,
        tame: false,
        angry: false,
        invisible: false,
        collar_color: None,
    }
    .vanilla_layer_texture_refs()
    .is_empty());
    assert!(EntityModelKind::Wolf {
        baby: false,
        tame: false,
        angry: false,
        invisible: false,
        collar_color: Some(EntityDyeColor::Red),
    }
    .vanilla_layer_texture_refs()
    .is_empty());
    assert!(EntityModelKind::Wolf {
        baby: false,
        tame: true,
        angry: false,
        invisible: true,
        collar_color: Some(EntityDyeColor::Red),
    }
    .vanilla_layer_texture_refs()
    .is_empty());
}

#[test]
fn wolf_textured_layer_passes_match_vanilla_renderer_layers() {
    let wild = wolf_textured_layer_passes(false, false, false, false, None);
    assert_eq!(
        wild.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![EntityModelLayerKind::WolfBase]
    );
    assert_eq!(wild[0].model_layer, MODEL_LAYER_WOLF);
    assert_eq!(wild[0].texture, WOLF_TEXTURE_REF);
    assert_eq!(wild[0].parts, ADULT_WOLF_TEXTURED_PARTS.as_slice());
    assert_eq!(wild[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((wild[0].collector_order, wild[0].submit_sequence), (0, 0));

    let tame_blue =
        wolf_textured_layer_passes(false, true, false, false, Some(EntityDyeColor::Blue));
    assert_eq!(
        tame_blue.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![
            EntityModelLayerKind::WolfBase,
            EntityModelLayerKind::WolfCollar
        ]
    );
    assert_eq!(tame_blue[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_blue[1].model_layer, MODEL_LAYER_WOLF);
    assert_eq!(tame_blue[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(tame_blue[1].parts, ADULT_WOLF_TEXTURED_PARTS.as_slice());
    assert_eq!(
        tame_blue[1].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
    assert_eq!(
        (tame_blue[1].collector_order, tame_blue[1].submit_sequence),
        (1, 1)
    );

    let invisible_tame =
        wolf_textured_layer_passes(false, true, false, true, Some(EntityDyeColor::Blue));
    assert_eq!(
        invisible_tame
            .iter()
            .map(|pass| pass.kind)
            .collect::<Vec<_>>(),
        vec![EntityModelLayerKind::WolfBase]
    );
    assert_eq!(invisible_tame[0].texture, WOLF_TAME_TEXTURE_REF);

    let angry = wolf_textured_layer_passes(false, false, true, false, None);
    assert_eq!(angry[0].texture, WOLF_ANGRY_TEXTURE_REF);
    assert_eq!(angry.len(), 1);

    let tame_angry =
        wolf_textured_layer_passes(false, true, true, false, Some(EntityDyeColor::Red));
    assert_eq!(tame_angry[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_angry.len(), 2);

    let baby_tame = wolf_textured_layer_passes(true, true, false, false, Some(EntityDyeColor::Red));
    assert_eq!(baby_tame[0].model_layer, MODEL_LAYER_WOLF_BABY);
    assert_eq!(baby_tame[0].texture, WOLF_TAME_BABY_TEXTURE_REF);
    assert_eq!(baby_tame[0].parts, BABY_WOLF_TEXTURED_PARTS.as_slice());
    assert_eq!(baby_tame[1].texture, WOLF_BABY_COLLAR_TEXTURE_REF);
    assert_eq!(baby_tame[1].parts, BABY_WOLF_TEXTURED_PARTS.as_slice());
}

#[test]
fn wolf_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_WOLF, "minecraft:wolf#main");
    assert_eq!(MODEL_LAYER_WOLF_BABY, "minecraft:wolf_baby#main");
    assert_eq!(
        ADULT_WOLF_TEXTURED_REAL_HEAD[0],
        TexturedModelCubeDesc {
            min: [-2.0, -3.0, -2.0],
            size: [6.0, 6.0, 4.0],
            uv_size: [6.0, 6.0, 4.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_WOLF_TEXTURED_RIGHT_LEG[0],
        TexturedModelCubeDesc {
            min: [0.0, 0.0, -1.0],
            size: [2.0, 8.0, 2.0],
            uv_size: [2.0, 8.0, 2.0],
            tex: [0.0, 18.0],
            mirror: true,
        }
    );
    assert_eq!(
        BABY_WOLF_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-3.015, -3.275, -3.025],
            size: [6.05, 5.05, 5.05],
            uv_size: [6.0, 5.0, 5.0],
            tex: [0.0, 12.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_WOLF_TEXTURED_LEFT_EAR[0],
        TexturedModelCubeDesc {
            min: [-1.0, -1.0, -0.5],
            size: [2.0, 2.0, 1.0],
            uv_size: [2.0, 2.0, 1.0],
            tex: [20.0, 5.0],
            mirror: false,
        }
    );
}

#[test]
fn wolf_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for base in [
        EntityModelInstance::wolf(480, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::wolf(481, [0.0, 64.0, 0.0], 0.0, true),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "{:?}", base.kind);
        assert_ne!(yawed.vertices, pitched.vertices, "{:?}", base.kind);
    }
}

#[test]
fn wolf_swings_its_legs_when_walking() {
    // Vanilla `WolfModel.setupAnim` (adult and baby) swings the four legs with the
    // `QuadrupedModel` diagonal phase `cos(pos * 0.6662 [+ π]) * 1.4 * speed` in its
    // non-sitting branch. A standing wolf is inert; a walking one splays its legs along
    // Z. The adult (with its tall legs) also lifts its feet; the baby's short legs swing
    // inside the head/body bounding box, so only the Z splay shows. The tail wag, the
    // water-shake body roll, and the sitting pose are deferred. Colored path.
    for (base, adult_size) in [
        (
            EntityModelInstance::wolf(148, [0.0, 64.0, 0.0], 0.0, false),
            true,
        ),
        (
            EntityModelInstance::wolf(149, [0.0, 64.0, 0.0], 0.0, true),
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(
            rest.vertices, still.vertices,
            "{:?} rest is inert",
            base.kind
        );

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{:?} legs should splay along Z",
            base.kind
        );
        if adult_size {
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "an adult wolf's feet should lift off the ground"
            );
        }
    }
}

#[test]
fn wolf_textured_mesh_swings_its_legs_when_walking() {
    // The real wolf render path (texture-backed) swings the same legs. A standing wolf is
    // byte-identical however far the swing has advanced; a walking one differs, splays
    // along Z, and (for the adult) lifts its feet, while keeping the vertex count.
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for (base, adult_size) in [
        (
            EntityModelInstance::wolf(482, [0.0, 64.0, 0.0], 0.0, false),
            true,
        ),
        (
            EntityModelInstance::wolf(483, [0.0, 64.0, 0.0], 0.0, true),
            false,
        ),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(resting.vertices, still.vertices, "{:?} is inert", base.kind);
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{:?} leg swing keeps the vertex count",
            base.kind
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );

        let (rest_min, rest_max) = textured_mesh_extents(&resting);
        let (walk_min, walk_max) = textured_mesh_extents(&walking);
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{:?} legs should splay along Z",
            base.kind
        );
        if adult_size {
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "an adult wolf's feet should lift off the ground (textured)"
            );
        }
    }
}

fn wolf_texture_images() -> Vec<EntityModelTextureImage> {
    wolf_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn wolf_tail_swing_pose_wags_with_the_quadruped_amplitude() {
    // Vanilla WolfModel.setupAnim (non-angry branch): tail.yRot = cos(pos * 0.6662) *
    // 1.4 * speed (the same QuadrupedModel amplitude as the legs, no phase offset). The
    // base tail pose carries the layer's resting xRot droop (0.62831855 for the adult);
    // the wag sets only yRot and preserves the other axes (vanilla overwrites xRot with
    // the deferred tailAngle).
    let base = ADULT_WOLF_PARTS[7].pose;
    assert!(
        (base.rotation[0] - 0.62831855).abs() < 1e-6,
        "adult tail rests with the layer xRot droop: {}",
        base.rotation[0]
    );
    let tail = wolf_tail_swing_pose(base, 0.0, 1.0);
    assert!(
        (tail.rotation[1] - 1.4).abs() < 1e-6,
        "tail wags to cos(0) * 1.4 * 1 = 1.4: {}",
        tail.rotation[1]
    );
    assert_eq!(tail.rotation[0], base.rotation[0], "xRot droop preserved");
    assert_eq!(tail.rotation[2], base.rotation[2], "zRot preserved");
    assert_eq!(tail.offset, base.offset, "offset preserved");

    // A general (pos, speed) reproduces cos(pos * 0.6662) * 1.4 * speed.
    let phase = 2.0_f32 * 0.6662;
    let tail = wolf_tail_swing_pose(base, 2.0, 0.5);
    assert!((tail.rotation[1] - phase.cos() * 1.4 * 0.5).abs() < 1e-6);

    // At rest (speed = 0) the wag is zero, so the tail keeps its resting pose.
    let tail = wolf_tail_swing_pose(base, 3.0, 0.0);
    assert_eq!(tail.rotation[1], 0.0);
}

#[test]
fn wolf_wags_its_tail_when_walking() {
    // The non-angry wolf wags its tail side to side (a yRot sweep) in step with the gait.
    // In the colored body layer the parts emit the head subtree and body/mane in the
    // leading blocks, then the four legs, then the tail child cube last: for the adult
    // (264 verts) the tail occupies vertices [240, 264) and the legs [144, 240); the baby
    // (240 verts) lists the tail at [216, 240) and the legs [120, 216). A yRot wag sweeps
    // the tail sideways, deepening the tail region's X footprint.
    let x_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[0]);
            hi = hi.max(vertex.position[0]);
        }
        hi - lo
    };
    for (baby, tail) in [(false, 240..264), (true, 216..240)] {
        let base = EntityModelInstance::wolf(150, [0.0, 64.0, 0.0], 0.0, baby);
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices[tail.clone()],
            walking.vertices[tail.clone()],
            "baby={baby}: the tail wags when walking"
        );
        let rest_tail_x = x_extent(&rest.vertices[tail.clone()]);
        let walk_tail_x = x_extent(&walking.vertices[tail.clone()]);
        assert!(
            walk_tail_x > rest_tail_x + 0.1,
            "baby={baby}: a yRot tail wag deepens the tail X footprint: {rest_tail_x} -> {walk_tail_x}"
        );
    }
}

#[test]
fn wolf_textured_mesh_wags_its_tail_when_walking() {
    // The texture-backed wolf base layer runs the same tail wag, emitting the parts in the
    // same order, so the adult tail occupies textured vertices [240, 264). A standing wolf
    // is byte-identical; a walking one wags its tail sideways.
    let x_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[0]);
            hi = hi.max(vertex.position[0]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let base = EntityModelInstance::wolf(151, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);
    assert_ne!(
        resting.vertices[240..264],
        walking.vertices[240..264],
        "the tail wags when walking"
    );
    let rest_tail_x = x_extent(&resting.vertices[240..264]);
    let walk_tail_x = x_extent(&walking.vertices[240..264]);
    assert!(
        walk_tail_x > rest_tail_x + 0.1,
        "the textured tail wags sideways when walking: {rest_tail_x} -> {walk_tail_x}"
    );
}
