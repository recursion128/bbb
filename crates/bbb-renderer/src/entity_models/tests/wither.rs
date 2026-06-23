use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn wither_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WitherBossModel.createBodyLayer(CubeDeformation.NONE)` (atlas 64×64): six sibling
    // root parts — shoulders, ribcage (spine + three ribs), tail, center head, two side heads.
    assert_eq!(WITHER_PARTS.len(), 6);

    // `shoulders` (20×3×3) at ZERO.
    let shoulders = &WITHER_PARTS[0];
    assert_eq!(shoulders.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(shoulders.cubes[0].min, [-10.0, 3.9, -0.5]);
    assert_eq!(shoulders.cubes[0].size, [20.0, 3.0, 3.0]);

    // `ribcage` (offset (-2, 6.9, -0.5), pitched 0.20420352 rad): the spine plus three rib bars.
    let ribcage = &WITHER_PARTS[1];
    assert_eq!(ribcage.pose.offset, [-2.0, 6.9, -0.5]);
    assert_eq!(ribcage.pose.rotation, [0.204_203_52, 0.0, 0.0]);
    assert_eq!(ribcage.cubes.len(), 4);
    assert_eq!(ribcage.cubes[0].size, [3.0, 10.0, 3.0]);
    assert_eq!(ribcage.cubes[1].min, [-4.0, 1.5, 0.5]);
    assert_eq!(ribcage.cubes[2].min, [-4.0, 4.0, 0.5]);
    assert_eq!(ribcage.cubes[3].min, [-4.0, 6.5, 0.5]);
    assert_eq!(ribcage.cubes[1].size, [11.0, 2.0, 2.0]);

    // `tail` (3×6×3) at the bind position derived from the ribcage bind pitch.
    let tail = &WITHER_PARTS[2];
    let ribcage_bind_xrot = 0.20420352_f32;
    let expected_tail_y = 6.9 + ribcage_bind_xrot.cos() * 10.0;
    let expected_tail_z = -0.5 + ribcage_bind_xrot.sin() * 10.0;
    assert!((tail.pose.offset[1] - expected_tail_y).abs() < 1.0e-4);
    assert!((tail.pose.offset[2] - expected_tail_z).abs() < 1.0e-4);
    assert_eq!(tail.pose.rotation, [0.832_522_03, 0.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [3.0, 6.0, 3.0]);

    // `center_head` (8×8×8) at ZERO; the two 6×6×6 side heads at their pivots.
    let center_head = &WITHER_PARTS[3];
    assert_eq!(center_head.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(center_head.cubes[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(WITHER_PARTS[4].pose.offset, [-8.0, 4.0, 0.0]);
    assert_eq!(WITHER_PARTS[5].pose.offset, [10.0, 4.0, 0.0]);
    assert_eq!(WITHER_PARTS[4].cubes[0].size, [6.0, 6.0, 6.0]);

    // Nine cubes total.
    assert_eq!(count_cubes(&WITHER_PARTS), 9);
}

#[test]
fn wither_mesh_uses_vanilla_body_layer_geometry() {
    // 9 cubes → 54 faces / 216 vertices / 324 indices; the body carries the body tint and the three
    // heads carry the head tint.
    let wither = entity_model_mesh(&[EntityModelInstance::wither(1450, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(wither.opaque_faces, 54);
    assert_eq!(wither.vertices.len(), 216);
    assert_eq!(wither.indices.len(), 324);
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_BODY, 1.0)));
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_HEAD, 1.0)));
}

#[test]
fn wither_center_head_follows_look_angles() {
    // Vanilla `WitherBossModel.setupAnim` sets `centerHead.yRot/xRot` from the net head look. The
    // center head is part 3 (vertices [144, 168)): one cube each for shoulders (1), ribcage (4), tail
    // (1) precede it (24·6 = 144). A non-zero look re-poses only those vertices; the shoulders /
    // ribcage / tail and the two side heads (which track the separate `DATA_TARGET_*` heads) stay at
    // bind.
    assert_eq!(WITHER_CENTER_HEAD_PART_INDEX, 3);
    let base = EntityModelInstance::wither(1451, [0.0, 64.0, 0.0], 0.0);
    let looking = base.with_head_look(35.0, -20.0);
    let rest = entity_model_mesh(&[base]);
    let turned = entity_model_mesh(&[looking]);
    assert_ne!(
        rest.vertices[144..168],
        turned.vertices[144..168],
        "the center head turns with the look angles"
    );
    assert_eq!(
        rest.vertices[..144],
        turned.vertices[..144],
        "the shoulders, ribcage, and tail are unmoved by the look (their shared age breathes both)"
    );
    assert_eq!(
        rest.vertices[168..],
        turned.vertices[168..],
        "the two side heads stay at bind"
    );
}

#[test]
fn wither_breathing_poses_match_vanilla_setup_anim() {
    use std::f32::consts::PI;
    // Vanilla `WitherBossModel.setupAnim`:
    //   anim         = cos(ageInTicks * 0.1)
    //   ribcage.xRot = (0.065 + 0.05 * anim) * PI
    //   tail.setPos(-2, 6.9 + cos(ribcage.xRot) * 10, -0.5 + sin(ribcage.xRot) * 10)
    //   tail.xRot    = (0.265 + 0.1 * anim) * PI
    let age = 10.0_f32;
    let anim = (age * 0.1).cos();
    let ribcage_x_rot = (0.065 + 0.05 * anim) * PI;
    let (ribcage, tail) = wither_breathing_poses(age);
    assert_eq!(ribcage.offset, [-2.0, 6.9, -0.5]);
    assert!((ribcage.rotation[0] - ribcage_x_rot).abs() < 1.0e-6);
    assert_eq!([ribcage.rotation[1], ribcage.rotation[2]], [0.0, 0.0]);
    assert!((tail.offset[0] - (-2.0)).abs() < 1.0e-6);
    assert!((tail.offset[1] - (6.9 + ribcage_x_rot.cos() * 10.0)).abs() < 1.0e-5);
    assert!((tail.offset[2] - (-0.5 + ribcage_x_rot.sin() * 10.0)).abs() < 1.0e-5);
    assert!((tail.rotation[0] - (0.265 + 0.1 * anim) * PI).abs() < 1.0e-6);
    assert_eq!([tail.rotation[1], tail.rotation[2]], [0.0, 0.0]);

    // `anim == 0` (when ageInTicks * 0.1 == PI/2) collapses the sway onto the baked rest poses, so
    // the breathing oscillates symmetrically about the layer pose.
    let (rib_rest, tail_rest) = wither_breathing_poses(5.0 * PI);
    let rib_bind = WITHER_PARTS[WITHER_RIBCAGE_PART_INDEX].pose;
    let tail_bind = WITHER_PARTS[WITHER_TAIL_PART_INDEX].pose;
    assert!((rib_rest.rotation[0] - rib_bind.rotation[0]).abs() < 1.0e-5);
    assert!((tail_rest.offset[1] - tail_bind.offset[1]).abs() < 1.0e-4);
    assert!((tail_rest.offset[2] - tail_bind.offset[2]).abs() < 1.0e-4);
    assert!((tail_rest.rotation[0] - tail_bind.rotation[0]).abs() < 1.0e-5);
}

#[test]
fn wither_ribcage_and_tail_breathe_with_age() {
    // The ribcage (cubes [24, 120)) and tail (cubes [120, 144)) sway off `ageInTicks`; the shoulders
    // (cubes [0, 24)) and the three heads (cubes [144, 216)) carry no breathing. Two distinct ages,
    // with the look at rest, re-pose only the ribcage and tail.
    let young = EntityModelInstance::wither(1460, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(3.0);
    let old = EntityModelInstance::wither(1460, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(11.0);
    let young_mesh = entity_model_mesh(&[young]);
    let old_mesh = entity_model_mesh(&[old]);
    assert_ne!(
        young_mesh.vertices[24..120],
        old_mesh.vertices[24..120],
        "the ribcage breathes with age"
    );
    assert_ne!(
        young_mesh.vertices[120..144],
        old_mesh.vertices[120..144],
        "the tail breathes with age"
    );
    assert_eq!(
        young_mesh.vertices[..24],
        old_mesh.vertices[..24],
        "the shoulders never breathe"
    );
    assert_eq!(
        young_mesh.vertices[144..],
        old_mesh.vertices[144..],
        "the three heads never breathe"
    );
}
