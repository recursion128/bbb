use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn witch_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`. The hat-tip (`hat4`) and mole keep the base UV box against their
    // inflated geometry.
    assert_eq!(
        WITCH_HEAD[0],
        ModelCube::new(
            [-4.0, -10.0, -4.0],
            [8.0, 10.0, 8.0],
            WITCH_ROBE,
            [8.0, 10.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(
        WITCH_HAT_4[0],
        ModelCube::new(
            [-0.25, -0.25, -0.25],
            [1.5, 2.5, 1.5],
            WITCH_HAT_COLOR,
            [1.0, 2.0, 1.0],
            [0.0, 95.0],
            false,
        )
    );
    assert_eq!(
        WITCH_MOLE[0],
        ModelCube::new(
            [0.25, 3.25, -6.5],
            [0.5, 0.5, 0.5],
            WITCH_ROBE,
            [1.0, 1.0, 1.0],
            [0.0, 0.0],
            false,
        )
    );
    // The nested hat chain (`hat` -> `hat2` -> `hat3` -> `hat4`) and the nose/mole carry their bind
    // poses; the head look, leg swing, and nose bob resolve their parts by name.
    assert_eq!(WITCH_HAT_POSE.offset, [-5.0, -10.03125, -5.0]);
    assert_eq!(WITCH_HAT_2_POSE.rotation, [-0.05235988, 0.0, 0.02617994]);
    assert_eq!(WITCH_HAT_3_POSE.rotation, [-0.10471976, 0.0, 0.05235988]);
    assert_eq!(
        WITCH_HAT_4_POSE.rotation,
        [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976]
    );
    assert_eq!(WITCH_NOSE_POSE.offset, [0.0, -2.0, 0.0]);
    assert_eq!(WITCH_MOLE_POSE.offset, [0.0, -2.0, 0.0]);
    assert_eq!(WITCH_ARMS[1].tex, [44.0, 22.0]);
    assert!(WITCH_ARMS[1].mirror);
    assert!(!WITCH_RIGHT_LEG[0].mirror);
    assert!(WITCH_LEFT_LEG[0].mirror);
}

#[test]
fn witch_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 84);
    assert_eq!(mesh.vertices.len(), 336);
    assert_eq!(mesh.indices.len(), 504);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875, 64.00094, -0.29296878]);
    assert_close3(max, [0.46875003, 66.56483, 0.3839772]);
}

#[test]
fn witch_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Witch.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/witch/witch.png",
            size: [64, 128],
        })
    );
}

#[test]
fn witch_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = witch_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::WitchBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_WITCH);
    assert_eq!(passes[0].texture, WITCH_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn witch_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_WITCH, "minecraft:witch#main");
    assert_eq!(WITCH_TEXTURE_REF.size, [64, 128]);
    // The unified cubes carry the textured UV sources (`uv_size`/`texOffs`/`mirror`) merged into the
    // colored geometry.
    assert_eq!(WITCH_HEAD[0].uv_size, [8.0, 10.0, 8.0]);
    assert_eq!(WITCH_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(WITCH_HAT_4[0].uv_size, [1.0, 2.0, 1.0]);
    assert_eq!(WITCH_HAT_4[0].tex, [0.0, 95.0]);
    assert_eq!(WITCH_MOLE[0].uv_size, [1.0, 1.0, 1.0]);
    assert_eq!(WITCH_MOLE[0].tex, [0.0, 0.0]);
    assert_eq!(WITCH_LEFT_LEG[0].uv_size, [4.0, 12.0, 4.0]);
    assert_eq!(WITCH_LEFT_LEG[0].tex, [0.0, 22.0]);
    assert!(WITCH_LEFT_LEG[0].mirror);
    assert_eq!(WITCH_JACKET[0].uv_size, [8.0, 20.0, 6.0]);
    assert_eq!(WITCH_JACKET[0].tex, [0.0, 38.0]);
}

#[test]
fn witch_texture_atlas_stitches_official_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/witch/witch.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn witch_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 84);
    assert_eq!(mesh.vertices.len(), 336);
    assert_eq!(mesh.indices.len(), 504);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.46875, 64.00094, -0.29296878]);
    assert_close3(max, [0.46875003, 66.56483, 0.3839772]);
}

#[test]
fn witch_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let base = EntityModelInstance::witch(67, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);

    // Witch head is part 0; head look turns it without changing vertex count.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn witch_swings_its_legs_when_walking() {
    // `WitchModel.setupAnim` swings the legs `cos(pos * 0.6662 [+ π]) * 1.4 * speed *
    // 0.5` (half amplitude, legs at [3, 4]). A standing witch is inert; a walking one
    // lifts its feet and splays its legs along Z. The nose bob/hold pose and combined
    // `arms` part are deferred. Colored path here; textured below.
    let base = EntityModelInstance::witch(310, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(rest.vertices, still.vertices, "rest is inert");

    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_ne!(rest.vertices, walking.vertices, "walking differs");

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking witch's feet should lift off the ground"
    );
    assert!(
        (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
        "a walking witch's legs should splay along Z"
    );
}

#[test]
fn witch_textured_mesh_swings_legs_when_walking() {
    // The real witch render path (texture-backed) swings the same half-amplitude
    // legs. A standing witch is byte-identical however far the swing has advanced; a
    // walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let base = EntityModelInstance::witch(311, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

    assert_eq!(
        resting.vertices, still.vertices,
        "a standing witch is inert"
    );
    assert_eq!(
        resting.vertices.len(),
        walking.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_ne!(
        resting.vertices, walking.vertices,
        "a walking witch differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting);
    let (walk_min, walk_max) = textured_mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking witch's feet should lift off the ground"
    );
}

#[test]
fn witch_nose_bob_pose_matches_vanilla_formula() {
    // Vanilla `WitchModel.setupAnim`:
    //   speed = 0.01 * (entityId % 10);
    //   nose.xRot = sin(ageInTicks * speed) * 4.5°;
    //   nose.zRot = cos(ageInTicks * speed) * 2.5°.
    // Both are SET absolutely (the base nose pose carries rotation [0, 0, 0]); the yRot and
    // the offset are preserved. The nose is the head's `nose` child (after the hat chain).
    let base = WITCH_NOSE_POSE;
    assert_eq!(base.offset, [0.0, -2.0, 0.0]);
    assert_eq!(base.rotation, [0.0, 0.0, 0.0]);

    // entityId 66 → speed 0.06; ageInTicks 40 reproduces the sin/cos amplitudes exactly.
    let (entity_id, age) = (66_i32, 40.0_f32);
    let speed = 0.01 * (entity_id % 10) as f32;
    let phase = age * speed;
    let posed = witch_nose_bob_pose(base, age, entity_id);
    assert!((posed.rotation[0] - phase.sin() * 4.5_f32.to_radians()).abs() < 1e-7);
    assert!((posed.rotation[2] - phase.cos() * 2.5_f32.to_radians()).abs() < 1e-7);
    assert_eq!(posed.rotation[1], base.rotation[1]);
    assert_eq!(posed.offset, base.offset);

    // At age 0 the bob is not neutral: zRot = cos(0) * 2.5° = 2.5°, xRot = sin(0) = 0.
    let at_zero = witch_nose_bob_pose(base, 0.0, entity_id);
    assert_eq!(at_zero.rotation[0], 0.0);
    assert!((at_zero.rotation[2] - 2.5_f32.to_radians()).abs() < 1e-7);

    // entityId % 10 == 0 freezes the bob (speed 0): the nose holds the age-0 pose forever.
    let frozen = witch_nose_bob_pose(base, age, 310);
    assert_eq!(frozen.rotation[0], 0.0);
    assert!((frozen.rotation[2] - 2.5_f32.to_radians()).abs() < 1e-7);
}

#[test]
fn witch_holding_item_pins_the_nose_to_the_drinking_pose() {
    // Vanilla `WitchModel.setupAnim` applies the idle nose bob first, then `isHoldingItem` moves the nose
    // to `(0, 1, -1.5)` and forces `xRot = -0.9`. The visible mesh must differ from the idle bob.
    let base = EntityModelInstance::witch(312, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(40.0);
    let idle = entity_model_mesh(&[base]);
    let holding = entity_model_mesh(&[base.with_witch_holding_item(true)]);
    assert_eq!(idle.vertices.len(), holding.vertices.len());
    assert_ne!(
        idle.vertices, holding.vertices,
        "holding a main-hand item re-poses the nose"
    );
}

#[test]
fn witch_colored_mesh_bobs_its_nose_as_age_advances() {
    // Vanilla runs `WitchModel.setupAnim` every frame, bobbing the nose from `ageInTicks`
    // (`speed = 0.01 * (entityId % 10)`). Advancing `ageInTicks` re-poses only the nose
    // subtree, so the mesh changes while the age-independent legs hold still. entityId 313
    // → speed 0.03 (nonzero, so the bob actually moves). Colored path here; textured below.
    let base = EntityModelInstance::witch(313, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(31.4)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the nose bobs as ageInTicks advances"
    );
    // The left leg is the final part and carries no age term, so it is byte-identical.
    let leg_tail = early.vertices.len() - 24;
    assert_eq!(
        early.vertices[leg_tail..],
        later.vertices[leg_tail..],
        "the legs do not depend on ageInTicks"
    );
}

#[test]
fn witch_textured_mesh_bobs_its_nose_as_age_advances() {
    // The texture-backed render path bobs the same nose. entityId 313 → speed 0.03.
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let base = EntityModelInstance::witch(313, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_textured_mesh(&[base], &atlas);
    let later = entity_model_textured_mesh(&[base.with_age_in_ticks(31.4)], &atlas);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the nose bobs as ageInTicks advances"
    );
    let leg_tail = early.vertices.len() - 24;
    assert_eq!(
        early.vertices[leg_tail..],
        later.vertices[leg_tail..],
        "the legs do not depend on ageInTicks"
    );
}

fn witch_texture_images() -> Vec<EntityModelTextureImage> {
    witch_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
