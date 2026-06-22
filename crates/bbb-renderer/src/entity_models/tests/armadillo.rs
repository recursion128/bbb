use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn adult_armadillo_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultArmadilloModel.createBodyLayer` (atlas 64×64): the root parents the body and
    // the four legs directly; the body parents the tail and head, and the head parents the head
    // cube and the two ear pivots.
    assert_eq!(ADULT_ARMADILLO_PARTS.len(), 5);

    // `body` (offset (0, 21, 4)): a `CubeDeformation(0.3)` shell over the bare 8×8×12 box.
    let body = &ADULT_ARMADILLO_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 21.0, 4.0]);
    assert_eq!(body.cubes.len(), 2);
    assert_eq!(body.cubes[0].min, [-4.3, -7.3, -10.3]);
    assert_eq!(body.cubes[0].size, [8.6, 8.6, 12.6]);
    assert_eq!(body.cubes[1].min, [-4.0, -7.0, -10.0]);
    assert_eq!(body.cubes[1].size, [8.0, 8.0, 12.0]);
    assert_eq!(body.children.len(), 2);

    // `tail`: the 1×6×1 plume, pitched down by 0.5061 rad.
    let tail = &body.children[0];
    assert_eq!(tail.pose.offset, [0.0, -3.0, 1.0]);
    assert_eq!(tail.pose.rotation, [0.5061, 0.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [1.0, 6.0, 1.0]);

    // `head` (offset (0, -2, -11)): a bare pivot parenting the head cube and the two ears.
    let head = &body.children[1];
    assert_eq!(head.pose.offset, [0.0, -2.0, -11.0]);
    assert!(head.cubes.is_empty());
    assert_eq!(head.children.len(), 3);

    // `head_cube`: the 3×5×2 snout, pitched up by -0.3927 rad.
    let head_cube = &head.children[0];
    assert_eq!(head_cube.pose.rotation, [-0.3927, 0.0, 0.0]);
    assert_eq!(head_cube.cubes[0].size, [3.0, 5.0, 2.0]);

    // The two ear pivots and their rotated 2×5×0 ear planes.
    let right_ear = &head.children[1];
    let left_ear = &head.children[2];
    assert_eq!(right_ear.pose.offset, [-1.0, -1.0, 0.0]);
    assert_eq!(
        right_ear.children[0].pose.rotation,
        [0.1886, -0.3864, -0.0718]
    );
    assert_eq!(right_ear.children[0].cubes[0].min, [-2.0, -3.0, 0.0]);
    assert_eq!(left_ear.pose.offset, [1.0, -2.0, 0.0]);
    assert_eq!(left_ear.children[0].pose.rotation, [0.1886, 0.3864, 0.0718]);
    assert_eq!(left_ear.children[0].cubes[0].size, [2.0, 5.0, 0.0]);

    // The four 2×3×2 legs at the corner pivots.
    assert_eq!(ADULT_ARMADILLO_PARTS[1].pose.offset, [-2.0, 21.0, 4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[2].pose.offset, [2.0, 21.0, 4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[3].pose.offset, [-2.0, 21.0, -4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[4].pose.offset, [2.0, 21.0, -4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[1].cubes[0].size, [2.0, 3.0, 2.0]);

    // Ten cubes (the shell-ball `cube` part is the deferred hiding-in-shell state).
    assert_eq!(count_cubes(&ADULT_ARMADILLO_PARTS), 10);
}

#[test]
fn baby_armadillo_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyArmadilloModel.createBodyLayer` (atlas 64×64): smaller geometry, the ears
    // parented to the head cube, and the front legs at swapped X origins.
    assert_eq!(BABY_ARMADILLO_PARTS.len(), 5);

    let body = &BABY_ARMADILLO_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 20.0, 0.5]);
    assert_eq!(body.cubes[0].min, [-2.8, -2.3, -3.8]);
    assert_eq!(body.cubes[0].size, [5.6, 4.6, 7.6]);
    assert_eq!(body.cubes[1].size, [5.0, 4.0, 6.0]);

    // `tail` pivot (offset (0, 0, 3.4)) parents the 1×1×4 stub pitched by -1.0472 rad.
    let tail = &body.children[0];
    assert_eq!(tail.pose.offset, [0.0, 0.0, 3.4]);
    assert_eq!(tail.children[0].pose.rotation, [-1.0472, 0.0, 0.0]);
    assert_eq!(tail.children[0].cubes[0].size, [1.0, 1.0, 4.0]);

    // `head` pivot parents the head cube (pitched up 0.7417649 rad) which parents the two ears.
    let head = &body.children[1];
    assert_eq!(head.pose.offset, [0.0, 0.0, -3.2]);
    let head_cube = &head.children[0];
    assert_eq!(head_cube.pose.rotation, [0.7417649, 0.0, 0.0]);
    assert_eq!(head_cube.cubes[0].size, [2.0, 2.0, 4.0]);
    assert_eq!(head_cube.children.len(), 2);
    assert_eq!(
        head_cube.children[0].pose.rotation,
        [-0.4363, -0.1134, 0.0524]
    );
    assert_eq!(
        head_cube.children[1].pose.rotation,
        [-0.4363, 0.1134, -0.0524]
    );
    assert_eq!(head_cube.children[0].cubes[0].size, [2.0, 3.0, 0.0]);

    // The front legs carry vanilla's swapped X origins (right at +1.5, left at -1.5).
    assert_eq!(BABY_ARMADILLO_PARTS[1].pose.offset, [-1.5, 22.0, 2.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[2].pose.offset, [1.5, 22.0, 2.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[3].pose.offset, [1.5, 22.0, -1.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[4].pose.offset, [-1.5, 22.0, -1.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[1].cubes[0].size, [2.0, 2.0, 2.0]);

    assert_eq!(count_cubes(&BABY_ARMADILLO_PARTS), 10);
}

#[test]
fn armadillo_mesh_selects_adult_or_baby_body_layer() {
    // Each rest pose has 10 cubes → 60 faces / 240 vertices / 360 indices; the soft head/ears/tail
    // carry the skin tint while the armored body/legs carry the shell tint.
    let adult = entity_model_mesh(&[EntityModelInstance::armadillo(
        70,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 60);
    assert_eq!(adult.vertices.len(), 240);
    assert_eq!(adult.indices.len(), 360);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARMADILLO_SHELL, 1.0)));
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARMADILLO_SKIN, 1.0)));

    let baby = entity_model_mesh(&[EntityModelInstance::armadillo(
        71,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);

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
