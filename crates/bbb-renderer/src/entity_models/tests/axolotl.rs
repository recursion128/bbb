use super::*;

#[test]
fn adult_axolotl_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultAxolotlModel.createBodyLayer` (atlas 64×64): the root holds the body, which
    // parents the head (parenting the three gills), the four leg planes, and the tail fin.

    // `body` (offset (0, 19.5, 5)): the 8×4×10 trunk plus a 0×5×9 dorsal fin.
    assert_eq!(ADULT_AXOLOTL_BODY_POSE.offset, [0.0, 19.5, 5.0]);
    assert_eq!(ADULT_AXOLOTL_BODY_CUBES.len(), 2);
    assert_eq!(ADULT_AXOLOTL_BODY_CUBES[0].min, [-4.0, -2.0, -9.0]);
    assert_eq!(ADULT_AXOLOTL_BODY_CUBES[0].size, [8.0, 4.0, 10.0]);
    assert_eq!(ADULT_AXOLOTL_BODY_CUBES[1].size, [0.0, 5.0, 9.0]);

    // `head` (offset (0, 0, -9)): the 8×5×5 skull, fudge-inflated, parenting three gill planes.
    assert_eq!(ADULT_AXOLOTL_HEAD_POSE.offset, [0.0, 0.0, -9.0]);
    assert_eq!(ADULT_AXOLOTL_HEAD_CUBES[0].min, [-4.001, -3.001, -5.001]);
    assert_eq!(ADULT_AXOLOTL_HEAD_CUBES[0].size, [8.002, 5.002, 5.002]);
    // top gills 8×3×0, the two side frills 3×7×0.
    assert_eq!(ADULT_AXOLOTL_TOP_GILLS_POSE.offset, [0.0, -3.0, -1.0]);
    assert_eq!(ADULT_AXOLOTL_TOP_GILLS_CUBES[0].size, [8.002, 3.002, 0.002]);
    assert_eq!(ADULT_AXOLOTL_LEFT_GILLS_POSE.offset, [-4.0, 0.0, -1.0]);
    assert_eq!(
        ADULT_AXOLOTL_LEFT_GILLS_CUBES[0].min,
        [-3.001, -5.001, -0.001]
    );
    assert_eq!(ADULT_AXOLOTL_RIGHT_GILLS_POSE.offset, [4.0, 0.0, -1.0]);
    assert_eq!(
        ADULT_AXOLOTL_RIGHT_GILLS_CUBES[0].min,
        [-0.001, -5.001, -0.001]
    );

    // The four 3×5×0 leg planes at the body corners (right legs use the -2 origin, left the -1).
    assert_eq!(ADULT_AXOLOTL_RIGHT_FRONT_LEG_POSE.offset, [-3.5, 1.0, -1.0]);
    assert_eq!(
        ADULT_AXOLOTL_RIGHT_LEG_CUBES[0].min,
        [-2.001, -0.001, -0.001]
    );
    assert_eq!(ADULT_AXOLOTL_LEFT_FRONT_LEG_POSE.offset, [3.5, 1.0, -1.0]);
    assert_eq!(
        ADULT_AXOLOTL_LEFT_LEG_CUBES[0].min,
        [-1.001, -0.001, -0.001]
    );
    assert_eq!(ADULT_AXOLOTL_RIGHT_HIND_LEG_POSE.offset, [-3.5, 1.0, -8.0]);
    assert_eq!(ADULT_AXOLOTL_LEFT_HIND_LEG_POSE.offset, [3.5, 1.0, -8.0]);

    // `tail` (offset (0, 0, 1)): the 0×5×12 fin plane.
    assert_eq!(ADULT_AXOLOTL_TAIL_POSE.offset, [0.0, 0.0, 1.0]);
    assert_eq!(ADULT_AXOLOTL_TAIL_CUBES[0].size, [0.0, 5.0, 12.0]);
}

#[test]
fn baby_axolotl_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyAxolotlModel.createBodyLayer` (atlas 32×32): a `root` bone at (0, 24, 0) wraps
    // the body, which parents the legs (one a doubly-rotated pivot), the tail, and the head.
    assert_eq!(BABY_AXOLOTL_ROOT_POSE.offset, [0.0, 24.0, 0.0]);

    assert_eq!(BABY_AXOLOTL_BODY_POSE.offset, [0.0, -1.25, 1.75]);
    assert_eq!(BABY_AXOLOTL_BODY_CUBES[0].min, [-2.0, -0.75, -2.75]);
    assert_eq!(BABY_AXOLOTL_BODY_CUBES[0].size, [4.0, 2.0, 6.0]);
    assert_eq!(BABY_AXOLOTL_BODY_CUBES[1].size, [0.0, 3.0, 5.0]);

    // `right_hind_leg` is a bare pivot rotated (yRot, zRot) = (π/2, π/2); its cube hangs off the
    // `right_leg_r1` child rotated (xRot, zRot) = (-π/2, π/2).
    assert_eq!(BABY_AXOLOTL_RIGHT_HIND_LEG_POSE.offset, [-2.0, 0.25, 1.75]);
    assert_eq!(
        BABY_AXOLOTL_RIGHT_HIND_LEG_POSE.rotation,
        [0.0, 1.5708, 1.5708]
    );
    assert_eq!(
        BABY_AXOLOTL_RIGHT_LEG_R1_POSE.rotation,
        [-1.5708, 0.0, 1.5708]
    );
    assert_eq!(BABY_AXOLOTL_RIGHT_HIND_LEG_CUBES[0].min, [0.0, 0.0, -0.5]);

    // `head` (offset (0, 0.25, -2.75)): the 6×3×4 skull parenting the three gill planes.
    assert_eq!(BABY_AXOLOTL_HEAD_POSE.offset, [0.0, 0.25, -2.75]);
    assert_eq!(BABY_AXOLOTL_HEAD_CUBES[0].size, [6.0, 3.0, 4.0]);
    assert_eq!(BABY_AXOLOTL_TOP_GILLS_POSE.offset, [0.0, -2.0, -2.0]);
    assert_eq!(BABY_AXOLOTL_TOP_GILLS_CUBES[0].size, [6.0, 3.0, 0.0]);
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

#[test]
fn axolotl_adult_body_turns_toward_the_look_yaw() {
    // Vanilla `AdultAxolotlModel.setupAnim` turns the whole body toward the look:
    // `body.yRot += yRot·π/180`, unconditionally and before the deferred procedural sways. The body
    // is the root part, so the yaw rotates every vertex about the body pivot.
    let adult_rest = entity_model_mesh(&[EntityModelInstance::axolotl(
        82,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let adult_yawed =
        entity_model_mesh(&[
            EntityModelInstance::axolotl(83, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(40.0, 0.0),
        ]);
    assert_eq!(adult_rest.vertices.len(), adult_yawed.vertices.len());
    assert_ne!(
        adult_rest.vertices, adult_yawed.vertices,
        "the adult body turns with the look yaw"
    );

    // The pitch feeds only the deferred swimming sways (line 77 uses `yRot` alone), so a pure-pitch
    // look leaves the body at rest.
    let adult_pitched =
        entity_model_mesh(&[
            EntityModelInstance::axolotl(84, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(0.0, -25.0),
        ]);
    assert_eq!(
        adult_rest.vertices, adult_pitched.vertices,
        "pitch alone does not turn the adult body"
    );

    // The baby model (entirely keyframe-driven, no `body.yRot += yRot`) ignores the look.
    let baby_rest = entity_model_mesh(&[EntityModelInstance::axolotl(
        85,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    let baby_looked =
        entity_model_mesh(&[
            EntityModelInstance::axolotl(86, [0.0, 64.0, 0.0], 0.0, true)
                .with_head_look(40.0, -25.0),
        ]);
    assert_eq!(
        baby_rest.vertices, baby_looked.vertices,
        "the baby axolotl ignores the look"
    );
}
