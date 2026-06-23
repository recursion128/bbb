use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn shulker_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ShulkerModel.createBodyLayer` (atlas 64×64): three sibling root parts — the 16×12×16
    // lid and the 16×8×16 base (both at offset (0, 24, 0)), and the 6×6×6 head at offset (0, 12, 0).
    assert_eq!(SHULKER_PARTS.len(), 3);

    let lid = &SHULKER_PARTS[0];
    assert_eq!(lid.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(lid.cubes[0].min, [-8.0, -16.0, -8.0]);
    assert_eq!(lid.cubes[0].size, [16.0, 12.0, 16.0]);

    let base = &SHULKER_PARTS[1];
    assert_eq!(base.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(base.cubes[0].min, [-8.0, -8.0, -8.0]);
    assert_eq!(base.cubes[0].size, [16.0, 8.0, 16.0]);

    let head = &SHULKER_PARTS[2];
    assert_eq!(head.pose.offset, [0.0, 12.0, 0.0]);
    assert_eq!(head.cubes[0].min, [-3.0, 0.0, -3.0]);
    assert_eq!(head.cubes[0].size, [6.0, 6.0, 6.0]);

    // Three cubes total.
    assert_eq!(count_cubes(&SHULKER_PARTS), 3);
}

#[test]
fn shulker_lid_pose_matches_vanilla_setup_anim() {
    use std::f32::consts::PI;

    // Closed (peek 0): `bs = 0.5π`, `sin(bs) = 1` → `lid.y = 16 + 8 = 24` (the bind offset), and
    // `peek ≤ 0.3` keeps `lid.yRot = 0`. The closed pose equals the bind pose.
    let (lid_y, lid_yrot) = shulker_lid_pose(0.0, 17.0);
    assert!((lid_y - 24.0).abs() < 1.0e-5);
    assert_eq!(lid_yrot, 0.0);

    // Fully open (peek 1): `bs = 1.5π`, `sin(bs) = -1` → `lid.y = 16 - 8 = 8` (the bob `extra` is
    // `sin(age·0.1)·0.7`, which is `0` at age 0). `q = -1 + (-1) = -2`, so
    // `lid.yRot = (-2)⁴ · π · 0.125 = 16 · π · 0.125 = 2π`.
    let (lid_y, lid_yrot) = shulker_lid_pose(1.0, 0.0);
    assert!((lid_y - 8.0).abs() < 1.0e-5);
    assert!((lid_yrot - 2.0 * PI).abs() < 1.0e-5);

    // The `lid.yRot` twist switches on strictly above `peek = 0.3` (vanilla `peekAmount > 0.3F`).
    assert_eq!(shulker_lid_pose(0.3, 0.0).1, 0.0);
    assert_ne!(shulker_lid_pose(0.4, 0.0).1, 0.0);

    // The open-lid bob `sin(age·0.1)·0.7` only applies past half-open (`bs > π`, i.e. `peek > 0.5`):
    // at `peek = 0.6` the lid Y moves with age; at `peek = 0.4` it does not.
    assert_ne!(shulker_lid_pose(0.6, 0.0).0, shulker_lid_pose(0.6, 15.0).0);
    assert_eq!(shulker_lid_pose(0.4, 0.0).0, shulker_lid_pose(0.4, 15.0).0);
}

#[test]
fn shulker_lid_opens_with_projected_peek() {
    // A closed shulker (peek 0) equals the bind-pose mesh; opening the lid (peek > 0) re-poses
    // the lid only, so the mesh differs while keeping the same 3-cube vertex count.
    let closed = EntityModelInstance::shulker(1121, [0.0, 64.0, 0.0], 0.0);
    let closed_mesh = entity_model_mesh(&[closed]);
    let default_mesh = entity_model_mesh(&[closed.with_shulker_peek(0.0)]);
    assert_eq!(closed_mesh.vertices, default_mesh.vertices);

    let open_mesh = entity_model_mesh(&[closed.with_shulker_peek(1.0)]);
    assert_eq!(closed_mesh.vertices.len(), open_mesh.vertices.len());
    assert_ne!(
        closed_mesh.vertices, open_mesh.vertices,
        "opening the lid re-poses the shulker lid"
    );
}

#[test]
fn shulker_head_tracks_look_angles() {
    // Vanilla `ShulkerModel.setupAnim` poses the head (part 2, emitted last → vertices 48..72)
    // with `head.xRot = xRot` and `head.yRot = (yHeadRot − 180 − yBodyRot) = head_yaw − 180`. The
    // lid (0..24) and base (24..48) never move with the look. A 60° look tilts the head off the
    // axis-aligned rest (head_yaw 0 → yRot −180, which is axis-aligned for the symmetric head cube).
    let base = EntityModelInstance::shulker(1130, [0.0, 64.0, 0.0], 0.0);
    let base_mesh = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(60.0, 0.0)]);
    assert_eq!(base_mesh.vertices.len(), 72);
    assert_eq!(looking.vertices.len(), 72);
    // The lid and base are untouched by the head look.
    assert_eq!(base_mesh.vertices[..48], looking.vertices[..48]);
    // The head re-poses with the yaw.
    assert_ne!(base_mesh.vertices[48..], looking.vertices[48..]);

    // The pitch maps to the head's xRot (a different axis than the yaw), so it re-poses the head
    // distinctly and still leaves the lid/base untouched.
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -25.0)]);
    assert_eq!(base_mesh.vertices[..48], pitched.vertices[..48]);
    assert_ne!(pitched.vertices[48..], looking.vertices[48..]);
}

#[test]
fn shulker_mesh_uses_vanilla_body_layer_geometry() {
    // 3 cubes → 18 faces / 72 vertices / 108 indices; the shell carries the shell tint and the head
    // carries its own yellow tint.
    let shulker = entity_model_mesh(&[EntityModelInstance::shulker(1120, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(shulker.opaque_faces, 18);
    assert_eq!(shulker.vertices.len(), 72);
    assert_eq!(shulker.indices.len(), 108);
    assert!(shulker
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SHULKER_SHELL, 1.0)));
    assert!(shulker
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SHULKER_HEAD, 1.0)));
}
