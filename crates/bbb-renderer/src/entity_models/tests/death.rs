use super::*;

#[test]
fn death_fall_factor_matches_vanilla_setup_rotations() {
    // Vanilla LivingEntityRenderer.setupRotations:
    //   fall = (deathTime - 1) / 20 * 1.6; fall = sqrt(fall); if fall > 1 fall = 1.
    // state.deathTime is always >= 1 while dying (entity.deathTime >= 1 + partial).
    assert_eq!(death_fall_factor(1.0), 0.0);
    assert!((death_fall_factor(11.0) - (0.8_f32).sqrt()).abs() < 1e-6);
    // deathTime 20: (19/20)*1.6 = 1.52, sqrt > 1, clamped to 1.0.
    assert_eq!(death_fall_factor(20.0), 1.0);
    // Beyond the clamp point it stays at 1.0.
    assert_eq!(death_fall_factor(40.0), 1.0);
    // The factor is monotonic across the unclamped range.
    assert!(death_fall_factor(5.0) < death_fall_factor(10.0));
    assert!(death_fall_factor(10.0) < death_fall_factor(15.0));
}

#[test]
fn entity_flip_degrees_match_vanilla_renderer_overrides() {
    // SpiderRenderer (and the cave spider that extends it), EndermiteRenderer,
    // and SilverfishRenderer override getFlipDegrees to 180; other living
    // renderers keep the base 90.
    assert_eq!(entity_flip_degrees(EntityModelKind::Spider), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::CaveSpider), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::Endermite), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::Silverfish), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::Creeper), 90.0);
    assert_eq!(
        entity_flip_degrees(EntityModelKind::Zombie { baby: false }),
        90.0
    );
    assert_eq!(entity_flip_degrees(EntityModelKind::Ravager), 90.0);
}

#[test]
fn death_flip_is_inert_until_the_fall_factor_grows() {
    // deathTime 0 (alive) and deathTime 1 (fall factor 0) leave the model upright,
    // so the mesh is byte-identical to the resting render.
    let base = EntityModelInstance::cow(900, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let alive = entity_model_mesh(&[base.with_death_time(0.0)]);
    let just_died = entity_model_mesh(&[base.with_death_time(1.0)]);

    assert_eq!(resting.vertices, alive.vertices);
    assert_eq!(resting.vertices, just_died.vertices);
}

#[test]
fn death_flip_tips_the_model_over_about_z() {
    // A fully tipped cow (deathTime 20, fall factor 1, 90-degree flip) is rotated
    // about the Z axis: the upright body's tall Y span lies down into the X span
    // while the Z (depth) span is preserved by a Z-axis rotation.
    let base = EntityModelInstance::cow(901, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let dead = entity_model_mesh(&[base.with_death_time(20.0)]);

    assert_eq!(resting.vertices.len(), dead.vertices.len());
    assert_ne!(resting.vertices, dead.vertices);

    let (resting_min, resting_max) = mesh_extents(&resting);
    let (dead_min, dead_max) = mesh_extents(&dead);
    let span = |min: [f32; 3], max: [f32; 3], axis: usize| max[axis] - min[axis];

    // The Z (front-to-back) span is unchanged by a rotation about Z.
    assert!(
        (span(dead_min, dead_max, 2) - span(resting_min, resting_max, 2)).abs() < 1e-3,
        "death flip preserves the Z span"
    );
    // The tall upright Y span collapses; the horizontal X span grows to take it.
    assert!(
        span(dead_min, dead_max, 1) < span(resting_min, resting_max, 1),
        "death flip shrinks the upright Y span"
    );
    assert!(
        span(dead_min, dead_max, 0) > span(resting_min, resting_max, 0),
        "death flip widens the X span"
    );
}

#[test]
fn death_flip_progresses_with_the_fall_factor() {
    // A partially fallen entity deviates from rest by less than a fully fallen one,
    // tying the rendered tilt to the monotonic fall factor.
    let base = EntityModelInstance::zombie(902, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let early = entity_model_mesh(&[base.with_death_time(5.0)]);
    let late = entity_model_mesh(&[base.with_death_time(20.0)]);

    let max_dev = |mesh: &EntityModelMesh| {
        mesh.vertices
            .iter()
            .zip(resting.vertices.iter())
            .map(|(a, b)| {
                let dx = a.position[0] - b.position[0];
                let dy = a.position[1] - b.position[1];
                let dz = a.position[2] - b.position[2];
                (dx * dx + dy * dy + dz * dz).sqrt()
            })
            .fold(0.0_f32, f32::max)
    };

    let early_dev = max_dev(&early);
    let late_dev = max_dev(&late);
    assert!(early_dev > 0.0, "an early-death zombie has begun to tip");
    assert!(
        late_dev > early_dev,
        "a fully fallen zombie has tipped further than an early-death one"
    );
}
