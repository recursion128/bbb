use super::*;

// The eight spider leg bind poses, mirrored from the model file (right/left pairs hind-to-front), so
// the leg-swing pose-math test can exercise `spider_leg_swing_pose` without the deleted `SPIDER_PARTS`
// const tree. Paired with `spider_leg_swing_roles()` by index.
const SPIDER_LEG_SPLAY: f32 = 0.58119464;
fn spider_leg_bind_poses() -> [PartPose; 8] {
    use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};
    [
        PartPose {
            offset: [-4.0, 15.0, 2.0],
            rotation: [0.0, FRAC_PI_4, -FRAC_PI_4],
        },
        PartPose {
            offset: [4.0, 15.0, 2.0],
            rotation: [0.0, -FRAC_PI_4, FRAC_PI_4],
        },
        PartPose {
            offset: [-4.0, 15.0, 1.0],
            rotation: [0.0, FRAC_PI_8, -SPIDER_LEG_SPLAY],
        },
        PartPose {
            offset: [4.0, 15.0, 1.0],
            rotation: [0.0, -FRAC_PI_8, SPIDER_LEG_SPLAY],
        },
        PartPose {
            offset: [-4.0, 15.0, 0.0],
            rotation: [0.0, -FRAC_PI_8, -SPIDER_LEG_SPLAY],
        },
        PartPose {
            offset: [4.0, 15.0, 0.0],
            rotation: [0.0, FRAC_PI_8, SPIDER_LEG_SPLAY],
        },
        PartPose {
            offset: [-4.0, 15.0, -1.0],
            rotation: [0.0, -FRAC_PI_4, -FRAC_PI_4],
        },
        PartPose {
            offset: [4.0, 15.0, -1.0],
            rotation: [0.0, FRAC_PI_4, FRAC_PI_4],
        },
    ]
}

#[test]
fn spider_textured_layer_passes_match_vanilla_renderer_model_layers() {
    // The vestigial `parts` slices are nulled; both passes read the unified `SpiderModel` tree.
    let spider = spider_textured_layer_passes(false);
    assert_eq!(spider.len(), 2);
    assert_eq!(
        spider[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(spider[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(spider[0].texture, SPIDER_TEXTURE_REF);
    assert_eq!(spider[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(spider[0].order, 0);
    assert_eq!(spider[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(spider[1].render_type.vanilla_name(), "eyes");
    assert_eq!(spider[1].texture, SPIDER_EYES_TEXTURE_REF);
    assert_eq!(spider[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(spider[1].order, 1);

    let cave = spider_textured_layer_passes(true);
    assert_eq!(cave.len(), 2);
    assert_eq!(
        cave[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(cave[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(cave[0].texture, CAVE_SPIDER_TEXTURE_REF);
    assert_eq!(cave[0].order, 0);
    assert_eq!(cave[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(cave[1].render_type.vanilla_name(), "eyes");
    assert_eq!(cave[1].texture, SPIDER_EYES_TEXTURE_REF);
    assert_eq!(cave[1].order, 1);
}

#[test]
fn spider_model_layers_and_eyes_texture_match_vanilla() {
    assert_eq!(ModelLayer::SPIDER, ModelLayer::SPIDER);
    assert_eq!(ModelLayer::CAVE_SPIDER, ModelLayer::CAVE_SPIDER);
    assert_eq!(
        SPIDER_EYES_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/spider/spider_eyes.png",
            size: [64, 32],
        }
    );
}

#[test]
fn entity_texture_atlas_stitches_official_spider_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 96);
    assert_eq!(layout.entries.len(), 3);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/spider/spider.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0 / 3.0]);
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/spider/cave_spider.png"
    );
    assert_close2(layout.entries[1].uv.min, [0.0, 1.0 / 3.0]);
    assert_close2(layout.entries[1].uv.max, [1.0, 2.0 / 3.0]);
    assert_eq!(
        layout.entries[2].texture.path,
        "textures/entity/spider/spider_eyes.png"
    );
    assert_close2(layout.entries[2].uv.min, [0.0, 2.0 / 3.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let cave_start = rgba_offset(layout.width, 32, 0, "cave spider atlas row").unwrap();
    assert_eq!(&rgba[cave_start..cave_start + 4], &[1; 4]);
    let eyes_start = rgba_offset(layout.width, 64, 0, "spider eyes atlas row").unwrap();
    assert_eq!(&rgba[eyes_start..eyes_start + 4], &[2; 4]);
}

#[test]
fn spider_textured_mesh_uses_vanilla_uvs_tints_and_cave_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();

    let spider_instance = EntityModelInstance::spider(912, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let spider_meshes = entity_model_textured_meshes(&[spider_instance], &atlas);
    assert_spider_textured_output(&spider_meshes, spider_instance);
    let spider = &spider_meshes.cutout;
    assert_eq!(spider.cutout_faces, 66);
    assert_eq!(spider.vertices.len(), 264);
    assert_eq!(spider.indices.len(), 396);
    assert_close2(spider.vertices[0].uv, [48.0 / 64.0, 4.0 / 96.0]);
    assert!(spider
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(spider);
    assert_close3(min, [-1.0282283, 64.0193, -0.9375]);
    assert_close3(max, [1.0282283, 64.8135, 0.7696068]);

    let cave_instance = EntityModelInstance::cave_spider(913, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let cave_meshes = entity_model_textured_meshes(&[cave_instance], &atlas);
    assert_spider_textured_output(&cave_meshes, cave_instance);
    let cave = &cave_meshes.cutout;
    assert_eq!(cave.cutout_faces, 66);
    assert_eq!(cave.vertices.len(), 264);
    assert_eq!(cave.indices.len(), 396);
    assert_close2(cave.vertices[0].uv, [48.0 / 64.0, 36.0 / 96.0]);
    assert!(cave
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(cave);
    assert_close3(min, [-0.71976, 64.01351, -0.65625]);
    assert_close3(max, [0.71976, 64.56945, 0.5387248]);
}

#[test]
fn spider_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();
    for base in [
        EntityModelInstance::spider(914, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::cave_spider(915, [0.0, 64.0, 0.0], 0.0),
    ] {
        let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
        let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_spider_textured_output(&resting, base);
        assert_spider_textured_output(&yawed, yawed_instance);
        assert_spider_textured_output(&pitched, pitched_instance);
        assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
        assert_eq!(resting.eyes.vertices.len(), yawed.eyes.vertices.len());
        assert_ne!(
            resting.cutout.vertices, yawed.cutout.vertices,
            "{:?}",
            base.kind
        );
        assert_ne!(
            resting.eyes.vertices, yawed.eyes.vertices,
            "{:?} eyes",
            base.kind
        );
        assert_ne!(
            yawed.cutout.vertices, pitched.cutout.vertices,
            "{:?}",
            base.kind
        );
        assert_ne!(
            yawed.eyes.vertices, pitched.eyes.vertices,
            "{:?} eyes",
            base.kind
        );
    }
}

#[test]
fn spider_textured_mesh_swings_legs_not_head_or_body() {
    // The real spider render path (texture-backed) swings the same legs. With the head at
    // rest, a walking spider changes only the leg region (verts 72..) and a standing one
    // is byte-identical however far the swing has advanced.
    const STILL: &[&str] = &["head", "body0", "body1"];
    const LEGS: &[&str] = &[
        "right_hind_leg",
        "left_hind_leg",
        "right_middle_hind_leg",
        "left_middle_hind_leg",
        "right_middle_front_leg",
        "left_middle_front_leg",
        "right_front_leg",
        "left_front_leg",
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();
    for (base, model) in [
        (
            EntityModelInstance::spider(916, [0.0, 64.0, 0.0], 0.0),
            ModelLayer::SPIDER.bake(),
        ),
        (
            EntityModelInstance::cave_spider(917, [0.0, 64.0, 0.0], 0.0),
            ModelLayer::CAVE_SPIDER.bake(),
        ),
    ] {
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_spider_textured_output(&resting, base);
        assert_spider_textured_output(&still, still_instance);
        assert_spider_textured_output(&walking, walking_instance);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{:?} is inert",
            base.kind
        );
        assert_eq!(
            resting.eyes.vertices, still.eyes.vertices,
            "{:?} eyes are inert",
            base.kind
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{:?}",
            base.kind
        );
        assert_eq!(resting.eyes.vertices.len(), walking.eyes.vertices.len());
        assert_eq!(
            parts_vertices(&resting.cutout.vertices, &model, STILL),
            parts_vertices(&walking.cutout.vertices, &model, STILL),
            "{:?} head and body stay put",
            base.kind
        );
        assert_ne!(
            parts_vertices(&resting.cutout.vertices, &model, LEGS),
            parts_vertices(&walking.cutout.vertices, &model, LEGS),
            "{:?} the legs swing",
            base.kind
        );
        assert_eq!(
            parts_vertices(&resting.eyes.vertices, &model, STILL),
            parts_vertices(&walking.eyes.vertices, &model, STILL),
            "{:?} eyes head and body stay put",
            base.kind
        );
        assert_ne!(
            parts_vertices(&resting.eyes.vertices, &model, LEGS),
            parts_vertices(&walking.eyes.vertices, &model, LEGS),
            "{:?} eyes legs swing",
            base.kind
        );
    }
}

#[test]
fn spider_leg_swing_pose_matches_vanilla_formula() {
    // Vanilla SpiderModel.setupAnim, with animationPos = walkAnimationPos * 0.6662:
    //   swing(phase) = -(cos(animationPos*2 + phase) * 0.4) * speed   (added to yRot)
    //   step(phase)  =  |sin(animationPos + phase) * 0.4| * speed      (added to zRot)
    //   right legs add both (`+=`); left legs subtract both (`-=`). The eight legs come
    //   in right/left pairs from hind to front with phases 0, π, π/2, 3π/2, now addressed
    //   by name in `spider_leg_swing_roles`.
    use std::f32::consts::{FRAC_PI_2, PI};
    assert_eq!(
        spider_leg_swing_roles(),
        [
            ("right_hind_leg", 0.0, 1.0),
            ("left_hind_leg", 0.0, -1.0),
            ("right_middle_hind_leg", PI, 1.0),
            ("left_middle_hind_leg", PI, -1.0),
            ("right_middle_front_leg", FRAC_PI_2, 1.0),
            ("left_middle_front_leg", FRAC_PI_2, -1.0),
            ("right_front_leg", PI * 1.5, 1.0),
            ("left_front_leg", PI * 1.5, -1.0),
        ]
    );

    let pos = 1.5_f32;
    let speed = 0.5_f32;
    let anim = pos * 0.6662;
    let bind = spider_leg_bind_poses();
    for (i, (_name, phase, side_sign)) in spider_leg_swing_roles().into_iter().enumerate() {
        let base = bind[i];
        let posed = spider_leg_swing_pose(base, phase, side_sign, pos, speed);
        let swing = -((anim * 2.0 + phase).cos() * 0.4) * speed;
        let step = (anim + phase).sin().abs() * 0.4 * speed;
        assert!(
            (posed.rotation[1] - (base.rotation[1] + side_sign * swing)).abs() < 1e-6,
            "leg {i} yRot",
        );
        assert!(
            (posed.rotation[2] - (base.rotation[2] + side_sign * step)).abs() < 1e-6,
            "leg {i} zRot",
        );
        // xRot and the offset are untouched; only the swing/step accumulate.
        assert_eq!(posed.rotation[0], base.rotation[0], "leg {i} xRot");
        assert_eq!(posed.offset, base.offset, "leg {i} offset");
    }

    // At rest (speed == 0) every leg holds its body-layer splay exactly.
    for (i, (_name, phase, side_sign)) in spider_leg_swing_roles().into_iter().enumerate() {
        let base = bind[i];
        assert_eq!(
            spider_leg_swing_pose(base, phase, side_sign, pos, 0.0),
            base,
            "leg {i} at rest"
        );
    }

    // The right/left hind pair mirror each other about the body-layer splay.
    let right_hind = spider_leg_swing_pose(bind[0], 0.0, 1.0, pos, speed);
    let left_hind = spider_leg_swing_pose(bind[1], 0.0, -1.0, pos, speed);
    let swing = -((anim * 2.0).cos() * 0.4) * speed;
    assert!((right_hind.rotation[1] - (bind[0].rotation[1] + swing)).abs() < 1e-6);
    assert!((left_hind.rotation[1] - (bind[1].rotation[1] - swing)).abs() < 1e-6);
}

#[test]
fn batched_spider_eyes_and_cow_keep_their_distinct_poses() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(128, 96) else {
        return;
    };
    let textures = [
        (COW_TEMPERATE_TEXTURE_REF, [240, 160, 80, 255]),
        (SPIDER_TEXTURE_REF, [100, 100, 100, 255]),
        (SPIDER_EYES_TEXTURE_REF, [240, 40, 60, 255]),
    ]
    .map(|(texture, rgba)| {
        EntityModelTextureImage::new(
            texture,
            rgba.repeat((texture.size[0] * texture.size[1]) as usize),
        )
    });
    renderer.upload_entity_model_textures(&textures).unwrap();
    let camera = crate::RendererCameraFrame {
        pose: Some(crate::CameraPose {
            position: [0.0, 65.0, -4.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }),
        ..Default::default()
    };
    let cow = EntityModelInstance::cow(1, [-1.75, 64.0, 2.0], 20.0, false);
    let spider = EntityModelInstance::spider(2, [1.75, 64.0, 2.0], -30.0).with(|s| {
        (s.walk_animation_pos, s.walk_animation_speed) = (1.0, 0.8);
    });
    let mut render = |entities| {
        renderer.replace_scene_frame(camera, entities);
        renderer.render_offscreen_frame().unwrap().rgba
    };
    let empty = render(vec![]);
    let cow_only = render(vec![cow]);
    let spider_only = render(vec![spider]);
    render(vec![]);
    let both = render(vec![cow, spider]);
    assert_ne!(cow_only, empty);
    assert_ne!(spider_only, empty);
    for (((&bg, &a), &b), &actual) in empty
        .as_chunks::<4>()
        .0
        .iter()
        .zip(cow_only.as_chunks::<4>().0)
        .zip(spider_only.as_chunks::<4>().0)
        .zip(both.as_chunks::<4>().0)
    {
        assert!(a == bg || b == bg);
        assert_eq!(actual, if a == bg { b } else { a });
    }
}

#[test]
fn spider_eyes_textured_mesh_uses_parent_model_geometry_and_eyes_render_type() {
    let (atlas, _) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();

    let meshes = entity_model_textured_meshes(
        &[
            EntityModelInstance::spider(912, [0.0, 64.0, 0.0], 0.0),
            EntityModelInstance::cave_spider(913, [0.0, 64.0, 0.0], 0.0),
        ],
        &atlas,
    );

    assert_eq!(meshes.cutout.cutout_faces, 132);
    assert_eq!(meshes.eyes.cutout_faces, 132);
    assert_eq!(meshes.cutout.vertices.len(), 528);
    assert_eq!(meshes.eyes.vertices.len(), 528);
    assert_close2(meshes.eyes.vertices[0].uv, [48.0 / 64.0, 68.0 / 96.0]);
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert_eq!(
        textured_mesh_extents(&meshes.eyes),
        textured_mesh_extents(&meshes.cutout)
    );
}

#[test]
fn spider_eyes_missing_texture_preserves_other_geometry() {
    let images: Vec<_> = spider_texture_images()
        .into_iter()
        .filter(|image| image.texture != SPIDER_EYES_TEXTURE_REF)
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    for instance in [
        EntityModelInstance::spider(912, [0.0, 64.0, 0.0], 0.0).with(|s| {
            s.light_coords = (4_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.45;
            s.has_red_overlay = true;
        }),
        EntityModelInstance::cave_spider(913, [0.0, 64.0, 0.0], 0.0).with(|s| {
            s.light_coords = (7_u32 << 4) | (9_u32 << 20);
            s.white_overlay_progress = 0.35;
            s.has_red_overlay = true;
        }),
    ] {
        let meshes = entity_model_textured_meshes(&[instance], &atlas);

        assert_spider_textured_output(&meshes, instance);
        assert!(!meshes.cutout.vertices.is_empty(), "{:?}", instance.kind);
        assert!(meshes.eyes.vertices.is_empty(), "{:?}", instance.kind);
        assert!(meshes.eyes.indices.is_empty(), "{:?}", instance.kind);
    }
}

fn assert_spider_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());

    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.eyes.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}
