use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn adult_axolotl_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultAxolotlModel.createBodyLayer` (atlas 64×64): the root holds the body, which
    // parents the head (parenting the three gills), the four leg planes, and the tail fin.
    assert_eq!(ADULT_AXOLOTL_PARTS.len(), 1);

    // `body` (offset (0, 19.5, 5)): the 8×4×10 trunk plus a 0×5×9 dorsal fin.
    let body = &ADULT_AXOLOTL_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 19.5, 5.0]);
    assert_eq!(body.cubes.len(), 2);
    assert_eq!(body.cubes[0].min, [-4.0, -2.0, -9.0]);
    assert_eq!(body.cubes[0].size, [8.0, 4.0, 10.0]);
    assert_eq!(body.cubes[1].size, [0.0, 5.0, 9.0]);
    assert_eq!(body.children.len(), 6);

    // `head` (offset (0, 0, -9)): the 8×5×5 skull, fudge-inflated, parenting three gill planes.
    let head = &body.children[0];
    assert_eq!(head.pose.offset, [0.0, 0.0, -9.0]);
    assert_eq!(head.cubes[0].min, [-4.001, -3.001, -5.001]);
    assert_eq!(head.cubes[0].size, [8.002, 5.002, 5.002]);
    assert_eq!(head.children.len(), 3);
    // top gills 8×3×0, the two side frills 3×7×0.
    assert_eq!(head.children[0].pose.offset, [0.0, -3.0, -1.0]);
    assert_eq!(head.children[0].cubes[0].size, [8.002, 3.002, 0.002]);
    assert_eq!(head.children[1].pose.offset, [-4.0, 0.0, -1.0]);
    assert_eq!(head.children[1].cubes[0].min, [-3.001, -5.001, -0.001]);
    assert_eq!(head.children[2].pose.offset, [4.0, 0.0, -1.0]);
    assert_eq!(head.children[2].cubes[0].min, [-0.001, -5.001, -0.001]);

    // The four 3×5×0 leg planes at the body corners (right legs use the -2 origin, left the -1).
    assert_eq!(body.children[1].pose.offset, [-3.5, 1.0, -1.0]);
    assert_eq!(body.children[1].cubes[0].min, [-2.001, -0.001, -0.001]);
    assert_eq!(body.children[2].pose.offset, [3.5, 1.0, -1.0]);
    assert_eq!(body.children[2].cubes[0].min, [-1.001, -0.001, -0.001]);
    assert_eq!(body.children[3].pose.offset, [-3.5, 1.0, -8.0]);
    assert_eq!(body.children[4].pose.offset, [3.5, 1.0, -8.0]);

    // `tail` (offset (0, 0, 1)): the 0×5×12 fin plane.
    assert_eq!(body.children[5].pose.offset, [0.0, 0.0, 1.0]);
    assert_eq!(body.children[5].cubes[0].size, [0.0, 5.0, 12.0]);

    // Eleven cubes.
    assert_eq!(count_cubes(&ADULT_AXOLOTL_PARTS), 11);
}

#[test]
fn baby_axolotl_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyAxolotlModel.createBodyLayer` (atlas 32×32): a `root` bone at (0, 24, 0) wraps
    // the body, which parents the legs (one a doubly-rotated pivot), the tail, and the head.
    assert_eq!(BABY_AXOLOTL_PARTS.len(), 1);
    let root = &BABY_AXOLOTL_PARTS[0];
    assert_eq!(root.pose.offset, [0.0, 24.0, 0.0]);
    assert!(root.cubes.is_empty());

    let body = &root.children[0];
    assert_eq!(body.pose.offset, [0.0, -1.25, 1.75]);
    assert_eq!(body.cubes[0].min, [-2.0, -0.75, -2.75]);
    assert_eq!(body.cubes[0].size, [4.0, 2.0, 6.0]);
    assert_eq!(body.cubes[1].size, [0.0, 3.0, 5.0]);
    assert_eq!(body.children.len(), 6);

    // `right_hind_leg` is a bare pivot rotated (yRot, zRot) = (π/2, π/2); its cube hangs off the
    // `right_leg_r1` child rotated (xRot, zRot) = (-π/2, π/2).
    let right_hind = &body.children[1];
    assert_eq!(right_hind.pose.offset, [-2.0, 0.25, 1.75]);
    assert_eq!(right_hind.pose.rotation, [0.0, 1.5708, 1.5708]);
    assert!(right_hind.cubes.is_empty());
    assert_eq!(right_hind.children[0].pose.rotation, [-1.5708, 0.0, 1.5708]);
    assert_eq!(right_hind.children[0].cubes[0].min, [0.0, 0.0, -0.5]);

    // `head` (offset (0, 0.25, -2.75)): the 6×3×4 skull parenting the three gill planes.
    let head = &body.children[5];
    assert_eq!(head.pose.offset, [0.0, 0.25, -2.75]);
    assert_eq!(head.cubes[0].size, [6.0, 3.0, 4.0]);
    assert_eq!(head.children.len(), 3);
    assert_eq!(head.children[2].pose.offset, [0.0, -2.0, -2.0]);
    assert_eq!(head.children[2].cubes[0].size, [6.0, 3.0, 0.0]);

    assert_eq!(count_cubes(&BABY_AXOLOTL_PARTS), 11);
}

#[test]
fn axolotl_mesh_selects_adult_or_baby_body_layer() {
    // Each rest pose has 11 cubes, but several are zero-thickness fins, so face counts vary; the
    // body carries the body tint and the gills carry the gill tint.
    let adult = entity_model_mesh(&[EntityModelInstance::axolotl(
        80,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(AXOLOTL_BODY, 1.0)));
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(AXOLOTL_GILLS, 1.0)));

    let baby = entity_model_mesh(&[EntityModelInstance::axolotl(
        81,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(AXOLOTL_GILLS, 1.0)));

    // The baby layer is geometrically smaller than the adult, so its mesh is more compact.
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_span = adult_max[2] - adult_min[2];
    let baby_span = baby_max[2] - baby_min[2];
    assert!(
        baby_span < adult_span,
        "baby z-span {baby_span} should be smaller than adult {adult_span}"
    );
}
