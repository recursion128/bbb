use super::*;

#[test]
fn phantom_parts_match_vanilla_26_1_body_layer() {
    // Vanilla PhantomModel.createBodyLayer: a body parenting the tail chain, the two wing
    // chains, and the head. texture 64x64. No CubeDeformation.
    assert_eq!(MODEL_LAYER_PHANTOM, "minecraft:phantom#main");

    // (pose, cube min, cube size)
    assert_eq!(PHANTOM_BODY_POSE.offset, [0.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_BODY_POSE.rotation, [-0.1, 0.0, 0.0]);
    assert_eq!(PHANTOM_BODY_CUBE.min, [-3.0, -2.0, -8.0]);
    assert_eq!(PHANTOM_BODY_CUBE.size, [5.0, 3.0, 9.0]);

    assert_eq!(PHANTOM_TAIL_BASE_POSE.offset, [0.0, -2.0, 1.0]);
    assert_eq!(PHANTOM_TAIL_BASE_CUBE.min, [-2.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_TAIL_BASE_CUBE.size, [3.0, 2.0, 6.0]);

    assert_eq!(PHANTOM_TAIL_TIP_POSE.offset, [0.0, 0.5, 6.0]);
    assert_eq!(PHANTOM_TAIL_TIP_CUBE.min, [-1.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_TAIL_TIP_CUBE.size, [1.0, 1.0, 6.0]);

    assert_eq!(PHANTOM_LEFT_WING_BASE_POSE.offset, [2.0, -2.0, -8.0]);
    assert_eq!(PHANTOM_LEFT_WING_BASE_POSE.rotation, [0.0, 0.0, 0.1]);
    assert_eq!(PHANTOM_LEFT_WING_BASE_CUBE.min, [0.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_LEFT_WING_BASE_CUBE.size, [6.0, 2.0, 9.0]);

    assert_eq!(PHANTOM_LEFT_WING_TIP_POSE.offset, [6.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_LEFT_WING_TIP_POSE.rotation, [0.0, 0.0, 0.1]);
    assert_eq!(PHANTOM_LEFT_WING_TIP_CUBE.min, [0.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_LEFT_WING_TIP_CUBE.size, [13.0, 1.0, 9.0]);

    // The right wing mirrors the left across x: negative-x boxes, opposite dihedral.
    assert_eq!(PHANTOM_RIGHT_WING_BASE_POSE.offset, [-3.0, -2.0, -8.0]);
    assert_eq!(PHANTOM_RIGHT_WING_BASE_POSE.rotation, [0.0, 0.0, -0.1]);
    assert_eq!(PHANTOM_RIGHT_WING_BASE_CUBE.min, [-6.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_RIGHT_WING_BASE_CUBE.size, [6.0, 2.0, 9.0]);

    assert_eq!(PHANTOM_RIGHT_WING_TIP_POSE.offset, [-6.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_RIGHT_WING_TIP_CUBE.min, [-13.0, 0.0, 0.0]);
    assert_eq!(PHANTOM_RIGHT_WING_TIP_CUBE.size, [13.0, 1.0, 9.0]);

    assert_eq!(PHANTOM_HEAD_POSE.offset, [0.0, 1.0, -7.0]);
    assert_eq!(PHANTOM_HEAD_POSE.rotation, [0.2, 0.0, 0.0]);
    assert_eq!(PHANTOM_HEAD_CUBE.min, [-4.0, -2.0, -5.0]);
    assert_eq!(PHANTOM_HEAD_CUBE.size, [7.0, 3.0, 5.0]);
}

#[test]
fn phantom_textured_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(PHANTOM_TEXTURE_REF.size, [64, 64]);
    // The nested tree: body parents the tail chain, both wing chains, and the head.
    assert_eq!(PHANTOM_TEXTURED_PARTS.len(), 1);
    let body = &PHANTOM_TEXTURED_PARTS[0];
    assert_eq!(body.pose, PHANTOM_BODY_POSE);
    assert_eq!(body.cubes[0], PHANTOM_BODY_TEXTURED_CUBE);
    assert_eq!(body.children.len(), 4);

    // texOffs + mirror sources for every cube.
    assert_eq!(PHANTOM_BODY_TEXTURED_CUBE.tex, [0.0, 8.0]);
    assert!(!PHANTOM_BODY_TEXTURED_CUBE.mirror);
    assert_eq!(PHANTOM_TAIL_BASE_TEXTURED_CUBE.tex, [3.0, 20.0]);
    assert_eq!(PHANTOM_TAIL_TIP_TEXTURED_CUBE.tex, [4.0, 29.0]);
    assert_eq!(PHANTOM_LEFT_WING_BASE_TEXTURED_CUBE.tex, [23.0, 12.0]);
    assert!(!PHANTOM_LEFT_WING_BASE_TEXTURED_CUBE.mirror);
    assert_eq!(PHANTOM_LEFT_WING_TIP_TEXTURED_CUBE.tex, [16.0, 24.0]);
    // Right wing reuses the left texOffs, mirrored.
    assert_eq!(PHANTOM_RIGHT_WING_BASE_TEXTURED_CUBE.tex, [23.0, 12.0]);
    assert!(PHANTOM_RIGHT_WING_BASE_TEXTURED_CUBE.mirror);
    assert_eq!(PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE.tex, [16.0, 24.0]);
    assert!(PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE.mirror);
    assert_eq!(PHANTOM_HEAD_TEXTURED_CUBE.tex, [0.0, 0.0]);
    // uv_size mirrors the cube size for every textured cube.
    assert_eq!(PHANTOM_BODY_TEXTURED_CUBE.uv_size, PHANTOM_BODY_CUBE.size);
    assert_eq!(
        PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE.uv_size,
        PHANTOM_RIGHT_WING_TIP_CUBE.size
    );
}

#[test]
fn phantom_layer_passes_match_vanilla_renderer() {
    let passes = phantom_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::PhantomBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_PHANTOM);
    assert_eq!(passes[0].texture, PHANTOM_TEXTURE_REF);
    assert_eq!(passes[0].parts, PHANTOM_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn phantom_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Phantom { size: 0 }.model_key(), "phantom");
    assert_eq!(EntityModelKind::Phantom { size: 5 }.model_key(), "phantom");
    assert_eq!(
        EntityModelKind::Phantom { size: 0 }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/phantom/phantom.png",
            size: [64, 64],
        })
    );
    assert!(entity_model_texture_refs().contains(&PHANTOM_TEXTURE_REF));
    assert_eq!(phantom_entity_texture_refs(), &[PHANTOM_TEXTURE_REF]);
}

#[test]
fn phantom_flap_time_matches_vanilla_unique_offset() {
    // Vanilla flapTime = getUniqueFlapTickOffset() + ageInTicks, getUniqueFlapTickOffset() =
    // id * 3 (Java int multiply).
    assert_eq!(phantom_flap_time(0, 0.0), 0.0);
    assert_eq!(phantom_flap_time(7, 0.0), 21.0);
    assert_eq!(phantom_flap_time(7, 4.5), 25.5);
    assert_eq!(phantom_flap_time(100, 1.0), 301.0);
}

#[test]
fn phantom_wing_and_tail_match_vanilla_setup_anim() {
    // Vanilla: anim = flapTime * 7.448451 * pi/180.
    //   wing zRot = cos(anim) * 16 deg ; right wing = -that.
    //   tail xRot = -(5 deg + cos(2*anim) * 5 deg).
    let deg = |d: f32| d.to_radians();
    for flap in [0.0f32, 21.0, 25.5, 137.0] {
        let anim = (flap * 7.448451).to_radians();
        let want_wing = anim.cos() * deg(16.0);
        let want_tail = -(deg(5.0) + (anim * 2.0).cos() * deg(5.0));
        assert!(
            (phantom_wing_z_rot(flap) - want_wing).abs() < 1e-7,
            "wing at flap {flap}"
        );
        assert!(
            (phantom_tail_x_rot(flap) - want_tail).abs() < 1e-7,
            "tail at flap {flap}"
        );
    }
    // The wing/tail never sit at the layer rest: at flap 0 the wing is cos(0)*16deg and the
    // tail carries the -10deg baseline (cos(0) term).
    assert!((phantom_wing_z_rot(0.0) - 16.0_f32.to_radians()).abs() < 1e-7);
    assert!((phantom_tail_x_rot(0.0) - (-10.0_f32.to_radians())).abs() < 1e-7);
}

#[test]
fn phantom_wing_pose_overwrites_rest_dihedral() {
    // setupAnim sets the wing zRot absolutely, overwriting the layer's +-0.1 rest dihedral.
    let posed = phantom_wing_pose(PHANTOM_LEFT_WING_BASE_POSE, 0.5);
    assert_eq!(posed.offset, PHANTOM_LEFT_WING_BASE_POSE.offset);
    assert_eq!(posed.rotation, [0.0, 0.0, 0.5]);
    // Tail pose sets xRot, preserving the zeroed yRot/zRot.
    let tail = phantom_tail_pose(PHANTOM_TAIL_BASE_POSE, -0.3);
    assert_eq!(tail.rotation, [-0.3, 0.0, 0.0]);
}

#[test]
fn phantom_mesh_uses_vanilla_body_layer_geometry() {
    // Body + tail(2) + wings(4) + head = 8 cubes => 8*24 = 192 verts, 48 faces, 288 indices.
    let mesh = entity_model_mesh(&[EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0)]);
    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);
    // The wingspan dominates the x extent (asymmetric: vanilla's left/right wing bases sit at
    // +2/-3 px), the flat body sets the ~0.45-block height, and the tail reaches back in z
    // (the 180-degree body yaw flips z).
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-1.3223567, 63.913067, -0.991909]);
    assert_close3(max, [1.2598567, 64.36279, 0.577472]);
}

#[test]
fn phantom_size_scales_the_mesh() {
    // Vanilla PhantomRenderer.scale: scale = 1 + 0.15 * size. A size-5 phantom is 1.75x the
    // base, so its mesh spans a wider extent than the size-0 phantom.
    let base = entity_model_mesh(&[EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0)]);
    let big = entity_model_mesh(&[EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 5)]);
    let (base_min, base_max) = mesh_extents(&base);
    let (big_min, big_max) = mesh_extents(&big);
    let base_span = base_max[0] - base_min[0];
    let big_span = big_max[0] - big_min[0];
    assert!(
        (big_span / base_span - 1.75).abs() < 1e-4,
        "size-5 phantom is 1.75x wide: {big_span} vs {base_span}"
    );
}

#[test]
fn phantom_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&phantom_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0)],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-1.3223567, 63.913067, -0.991909]);
    assert_close3(max, [1.2598567, 64.36279, 0.577472]);
}

#[test]
fn phantom_wings_flap_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: the wings and tail flap by flapTime while the body
    // and head hold still. The body is the first cube (vertices [0, 24)).
    let base = EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(11.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the wings flap as ageInTicks advances"
    );
    assert_eq!(
        early.vertices[..24],
        later.vertices[..24],
        "the body does not depend on ageInTicks"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&phantom_texture_images()).unwrap();
    let early_t = entity_model_textured_mesh(&[base], &atlas);
    let later_t = entity_model_textured_mesh(&[base.with_age_in_ticks(11.0)], &atlas);
    assert_ne!(
        early_t.vertices, later_t.vertices,
        "the textured wings flap too"
    );
}

#[test]
fn phantom_flap_phase_depends_on_entity_id() {
    // Vanilla's getUniqueFlapTickOffset = id * 3 gives each phantom a distinct flap phase, so
    // two phantoms with the same age but different ids render differently.
    let a = entity_model_mesh(&[EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0)]);
    let b = entity_model_mesh(&[EntityModelInstance::phantom(100, [0.0, 64.0, 0.0], 0.0, 0)]);
    assert_ne!(
        a.vertices, b.vertices,
        "different entity ids give different flap phases"
    );
}

fn phantom_texture_images() -> Vec<EntityModelTextureImage> {
    phantom_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
