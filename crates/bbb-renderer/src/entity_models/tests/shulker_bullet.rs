use super::*;

#[test]
fn shulker_bullet_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ShulkerBulletModel.createBodyLayer` (atlas 64×32): one `main` part at ZERO with three
    // interlocking slabs — `texOffs(0, 0)` 8×8×2, `texOffs(0, 10)` 2×8×8, `texOffs(20, 0)` 8×2×8.
    assert_eq!(SHULKER_BULLET_CUBES.len(), 3);

    assert_eq!(SHULKER_BULLET_CUBES[0].min, [-4.0, -4.0, -1.0]);
    assert_eq!(SHULKER_BULLET_CUBES[0].size, [8.0, 8.0, 2.0]);
    assert_eq!(SHULKER_BULLET_CUBES[0].tex, [0.0, 0.0]);
    assert_eq!(SHULKER_BULLET_CUBES[1].min, [-1.0, -4.0, -4.0]);
    assert_eq!(SHULKER_BULLET_CUBES[1].size, [2.0, 8.0, 8.0]);
    assert_eq!(SHULKER_BULLET_CUBES[1].tex, [0.0, 10.0]);
    assert_eq!(SHULKER_BULLET_CUBES[2].min, [-4.0, -1.0, -4.0]);
    assert_eq!(SHULKER_BULLET_CUBES[2].size, [8.0, 2.0, 8.0]);
    assert_eq!(SHULKER_BULLET_CUBES[2].tex, [20.0, 0.0]);
}

#[test]
fn shulker_bullet_mesh_uses_vanilla_body_layer_geometry() {
    // 3 cubes → 18 faces / 72 vertices / 108 indices; the slabs carry their single tint.
    let bullet = entity_model_mesh(&[EntityModelInstance::shulker_bullet(
        1130,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(bullet.opaque_faces, 18);
    assert_eq!(bullet.vertices.len(), 72);
    assert_eq!(bullet.indices.len(), 108);
    assert!(bullet
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SHULKER_BULLET_COLOR, 1.0)));
}

#[test]
fn shulker_bullet_orients_by_facing() {
    // `ShulkerBulletModel.setupAnim` rotates `main` by the bullet's yaw/pitch, so changing either the
    // yaw (`body_rot`) or the pitch (`head_pitch`) re-poses the whole model.
    let base = EntityModelInstance::shulker_bullet(1131, [0.0, 64.0, 0.0], 0.0);
    let yawed = EntityModelInstance::shulker_bullet(1131, [0.0, 64.0, 0.0], 90.0);
    let pitched = base.with_head_look(0.0, 45.0);

    let base_mesh = entity_model_mesh(&[base]);
    let yawed_mesh = entity_model_mesh(&[yawed]);
    let pitched_mesh = entity_model_mesh(&[pitched]);
    assert_eq!(base_mesh.vertices.len(), yawed_mesh.vertices.len());
    assert_ne!(
        base_mesh.vertices, yawed_mesh.vertices,
        "the yaw orients the bullet"
    );
    assert_ne!(
        base_mesh.vertices, pitched_mesh.vertices,
        "the pitch orients the bullet"
    );
}

#[test]
fn shulker_bullet_tumbles_with_age() {
    // Vanilla `ShulkerBulletRenderer.submit` spins the bullet by an `ageInTicks`-driven tumble
    // (`Ry(sin(t·0.1)·180°) · Rx(cos(t·0.1)·180°) · Rz(sin(t·0.15)·360°)`), so the whole model
    // re-poses as it flies, and the tumble advances with the (partial-tick-lerped) age.
    let base = EntityModelInstance::shulker_bullet(1132, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]); // age 0
    let aged = entity_model_mesh(&[base.with_age_in_ticks(10.0)]);
    let aged_later = entity_model_mesh(&[base.with_age_in_ticks(13.0)]);
    assert_eq!(rest.vertices.len(), aged.vertices.len());
    assert_ne!(
        rest.vertices, aged.vertices,
        "the bullet tumbles as it ages"
    );
    assert_ne!(
        aged.vertices, aged_later.vertices,
        "the tumble advances with the age"
    );
}

#[test]
fn shulker_bullet_textured_render_matches_vanilla_renderer() {
    assert_eq!(
        shulker_bullet_textured_layer_passes()[0].texture,
        SHULKER_BULLET_TEXTURE_REF
    );
    assert_eq!(
        EntityModelKind::ShulkerBullet.vanilla_texture_ref(),
        Some(SHULKER_BULLET_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&SHULKER_BULLET_TEXTURE_REF));
    assert_eq!(
        shulker_bullet_entity_texture_refs(),
        &[SHULKER_BULLET_TEXTURE_REF]
    );

    let len = usize::try_from(
        SHULKER_BULLET_TEXTURE_REF.size[0] * SHULKER_BULLET_TEXTURE_REF.size[1] * 4,
    )
    .unwrap();
    let images = vec![EntityModelTextureImage::new(
        SHULKER_BULLET_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::shulker_bullet(1130, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.submissions.len(), 2);
    assert_eq!(
        meshes.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        meshes.submissions[1].render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(meshes.submissions[0].texture, SHULKER_BULLET_TEXTURE_REF);
    assert_eq!(meshes.submissions[1].texture, SHULKER_BULLET_TEXTURE_REF);
    assert_eq!(meshes.submissions[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(meshes.submissions[1].tint, [1.0, 1.0, 1.0, 38.0 / 255.0]);
    assert_eq!(meshes.submissions[0].collector_order, 0);
    assert_eq!(meshes.submissions[1].collector_order, 1);
    assert_eq!(meshes.submissions[0].submit_sequence, 0);
    assert_eq!(meshes.submissions[1].submit_sequence, 1);
    let base_transform = shulker_bullet_model_root_transform(instance);
    assert_eq!(meshes.submissions[0].transform, base_transform);
    assert_eq!(
        meshes.submissions[1].transform,
        base_transform * Mat4::from_scale(Vec3::splat(1.5))
    );
    let mesh = &meshes.cutout;
    assert_eq!(mesh.vertices.len(), 72);
    assert_eq!(mesh.indices.len(), 108);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert_eq!(meshes.translucent.vertices.len(), 72);
    assert_eq!(meshes.translucent.indices.len(), 108);
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 38.0 / 255.0]));

    let (base_min, base_max) = textured_mesh_extents(&meshes.cutout);
    let (shell_min, shell_max) = textured_mesh_extents(&meshes.translucent);
    let center = |min: [f32; 3], max: [f32; 3]| {
        [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ]
    };
    let size = |min: [f32; 3], max: [f32; 3]| [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    assert_close3(center(shell_min, shell_max), center(base_min, base_max));
    let base_size = size(base_min, base_max);
    assert_close3(
        size(shell_min, shell_max),
        [base_size[0] * 1.5, base_size[1] * 1.5, base_size[2] * 1.5],
    );
}
