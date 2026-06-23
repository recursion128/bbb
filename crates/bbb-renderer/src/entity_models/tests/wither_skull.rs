use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn wither_skull_geometry_matches_vanilla_26_1_skull_layer() {
    // Vanilla `WitherSkullRenderer.createSkullLayer` (atlas 64×64): one `head` part at ZERO with a
    // single 8×8×8 box (`addBox(-4, -8, -4, 8, 8, 8)`).
    assert_eq!(WITHER_SKULL_PARTS.len(), 1);
    let head = &WITHER_SKULL_PARTS[0];
    assert_eq!(head.pose.offset, [0.0, 0.0, 0.0]);
    assert!(head.children.is_empty());
    assert_eq!(head.cubes.len(), 1);
    assert_eq!(head.cubes[0].min, [-4.0, -8.0, -4.0]);
    assert_eq!(head.cubes[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(count_cubes(&WITHER_SKULL_PARTS), 1);
}

#[test]
fn wither_skull_mesh_uses_vanilla_skull_layer_geometry() {
    // 1 cube → 6 faces / 24 vertices / 36 indices, one dark tint.
    let skull = entity_model_mesh(&[EntityModelInstance::wither_skull(
        820,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(skull.opaque_faces, 6);
    assert_eq!(skull.vertices.len(), 24);
    assert_eq!(skull.indices.len(), 36);
    assert!(skull
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_SKULL_GRAY, 1.0)));
}

#[test]
fn wither_skull_mesh_matches_on_both_render_paths() {
    // The wither skull is a colored-only entity (the wither textures are deferred), so the
    // texture-skipping colored runtime path emits the exact same mesh as the full path.
    let instances = [EntityModelInstance::wither_skull(
        821,
        [0.0, 64.0, 0.0],
        0.0,
    )];
    let full = entity_model_mesh(&instances);
    let colored = entity_model_colored_runtime_mesh(&instances);
    assert_eq!(full.vertices, colored.vertices);
    assert_eq!(full.indices, colored.indices);
}

#[test]
fn wither_skull_orients_along_flight() {
    // `WitherSkullRenderer` flips the skull (`scale(-1, -1, 1)`) and `SkullModel.setupAnim` turns it by
    // the flight `yRot`/`xRot`, so changing either the yaw (`body_rot`) or the pitch (`head_pitch`)
    // re-poses the whole skull.
    let base = EntityModelInstance::wither_skull(822, [0.0, 64.0, 0.0], 0.0);
    let yawed = EntityModelInstance::wither_skull(822, [0.0, 64.0, 0.0], 90.0);
    let pitched = base.with_head_look(0.0, 45.0);

    let base_mesh = entity_model_mesh(&[base]);
    let yawed_mesh = entity_model_mesh(&[yawed]);
    let pitched_mesh = entity_model_mesh(&[pitched]);
    assert_eq!(base_mesh.vertices.len(), yawed_mesh.vertices.len());
    assert_ne!(
        base_mesh.vertices, yawed_mesh.vertices,
        "the yaw orients the skull"
    );
    assert_ne!(
        base_mesh.vertices, pitched_mesh.vertices,
        "the pitch orients the skull"
    );
}

#[test]
fn wither_skull_exposes_stable_model_key() {
    assert_eq!(EntityModelKind::WitherSkull.model_key(), "wither_skull");
}
