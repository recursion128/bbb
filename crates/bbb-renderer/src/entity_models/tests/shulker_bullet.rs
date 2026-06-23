use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn shulker_bullet_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ShulkerBulletModel.createBodyLayer` (atlas 64×32): one `main` part at ZERO with three
    // interlocking slabs — `texOffs(0, 0)` 8×8×2, `texOffs(0, 10)` 2×8×8, `texOffs(20, 0)` 8×2×8.
    assert_eq!(SHULKER_BULLET_PARTS.len(), 1);
    let main = &SHULKER_BULLET_PARTS[0];
    assert_eq!(main.pose.offset, [0.0, 0.0, 0.0]);
    assert!(main.children.is_empty());
    assert_eq!(main.cubes.len(), 3);

    assert_eq!(main.cubes[0].min, [-4.0, -4.0, -1.0]);
    assert_eq!(main.cubes[0].size, [8.0, 8.0, 2.0]);
    assert_eq!(main.cubes[1].min, [-1.0, -4.0, -4.0]);
    assert_eq!(main.cubes[1].size, [2.0, 8.0, 8.0]);
    assert_eq!(main.cubes[2].min, [-4.0, -1.0, -4.0]);
    assert_eq!(main.cubes[2].size, [8.0, 2.0, 8.0]);

    assert_eq!(count_cubes(&SHULKER_BULLET_PARTS), 3);
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
