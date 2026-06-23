use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn tadpole_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `TadpoleModel.createBodyLayer` (atlas 16×16): two sibling root parts — a 3×2×3 body
    // box at offset (0, 22, -3) and a 0×2×7 tail fin plane at offset (0, 22, 0).
    assert_eq!(TADPOLE_PARTS.len(), 2);

    let body = &TADPOLE_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 22.0, -3.0]);
    assert!(body.children.is_empty());
    assert_eq!(body.cubes[0].min, [-1.5, -1.0, 0.0]);
    assert_eq!(body.cubes[0].size, [3.0, 2.0, 3.0]);

    let tail = &TADPOLE_PARTS[1];
    assert_eq!(tail.pose.offset, [0.0, 22.0, 0.0]);
    assert_eq!(tail.cubes[0].min, [0.0, -1.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [0.0, 2.0, 7.0]);

    // Two cubes total.
    assert_eq!(count_cubes(&TADPOLE_PARTS), 2);
}

#[test]
fn tadpole_mesh_uses_vanilla_body_layer_geometry() {
    // The body box contributes 6 faces; the tail is a zero-width plane (front/back quads only). The
    // body carries the body tint and the tail carries its own fin tint.
    let tadpole = entity_model_mesh(&[EntityModelInstance::tadpole(640, [0.0, 64.0, 0.0], 0.0)]);
    assert!(tadpole
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TADPOLE_BODY, 1.0)));
    assert!(tadpole
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TADPOLE_TAIL, 1.0)));
}

#[test]
fn tadpole_tail_sway_matches_vanilla_setup_anim() {
    // `tail.yRot = -amplitude * 0.25 * sin(0.3 * ageInTicks)`, amplitude 1.0 in water / 1.5 out (a
    // beached tadpole thrashes harder). At age 0 the sway is zero regardless of amplitude.
    assert_eq!(tadpole_tail_yrot(0.0, true), 0.0);
    assert_eq!(tadpole_tail_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (0.3 * age).sin();
    assert!((tadpole_tail_yrot(age, true) - (-1.0 * 0.25 * s)).abs() < 1.0e-6);
    assert!((tadpole_tail_yrot(age, false) - (-1.5 * 0.25 * s)).abs() < 1.0e-6);
    assert!(tadpole_tail_yrot(age, false).abs() > tadpole_tail_yrot(age, true).abs());
}

#[test]
fn tadpole_swims_its_tail_with_age() {
    // A still tadpole (age 0) is at bind; advancing the age sways the tail fin (part 1, vertices
    // [24, 48)) while the body box (part 0, [0, 24)) stays put.
    assert_eq!(TADPOLE_TAIL_PART_INDEX, 1);
    let base = EntityModelInstance::tadpole(641, [0.0, 64.0, 0.0], 0.0).with_in_water(true);
    let rest = entity_model_mesh(&[base]);
    let swaying = entity_model_mesh(&[base.with_age_in_ticks(5.0)]);
    assert_eq!(rest.vertices.len(), swaying.vertices.len());
    assert_eq!(
        rest.vertices[..24],
        swaying.vertices[..24],
        "the body stays put"
    );
    assert_ne!(
        rest.vertices[24..],
        swaying.vertices[24..],
        "the tail fin sways with the age"
    );

    // A beached tadpole thrashes harder, so its tail differs from the in-water sway at the same age.
    let beached = entity_model_mesh(&[base.with_in_water(false).with_age_in_ticks(5.0)]);
    assert_ne!(beached.vertices[24..], swaying.vertices[24..]);
}
