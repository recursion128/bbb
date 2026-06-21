use super::*;

#[test]
fn auto_spin_is_inert_until_the_entity_spins() {
    // No auto-spin (None) leaves the model upright: the mesh is byte-identical to
    // the resting render, so the riptide branch never fires for a still entity.
    let base = EntityModelInstance::cow(910, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let not_spinning = entity_model_mesh(&[base.with_auto_spin_age_ticks(None)]);

    assert_eq!(resting.vertices, not_spinning.vertices);
}

#[test]
fn auto_spin_tips_and_spins_the_model() {
    // Vanilla LivingEntityRenderer.setupRotations riptide branch:
    //   Axis.XP.rotationDegrees(-90 - xRot) then Axis.YP.rotationDegrees(ageInTicks * -75).
    // The leading Rx(-90) lays the upright model down, and the trailing Ry advances
    // with ageInTicks, so different ages render different spin frames.
    let base = EntityModelInstance::cow(911, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let spin_start = entity_model_mesh(&[base.with_auto_spin_age_ticks(Some(0.0))]);
    let spin_later = entity_model_mesh(&[base.with_auto_spin_age_ticks(Some(2.0))]);

    assert_eq!(resting.vertices.len(), spin_start.vertices.len());
    // The Rx(-90) tip-over alone already differs from the upright resting render.
    assert_ne!(resting.vertices, spin_start.vertices);
    // Advancing ageInTicks rotates the trailing Ry, so the spin frame changes.
    assert_ne!(spin_start.vertices, spin_later.vertices);

    // The leading Rx(-90) lays the model onto its spin axis: it maps local +Z to
    // +Y and local +Y to -Z (Ry(0) at age 0 is identity), so the resting Y/Z
    // extents swap exactly. The outer Ry(180 - bodyRot) preserves the Y span and
    // only mirrors the Z span, leaving both extents' magnitudes unchanged.
    let (resting_min, resting_max) = mesh_extents(&resting);
    let (spin_min, spin_max) = mesh_extents(&spin_start);
    let span = |min: [f32; 3], max: [f32; 3], axis: usize| max[axis] - min[axis];
    assert!(
        (span(spin_min, spin_max, 1) - span(resting_min, resting_max, 2)).abs() < 1e-3,
        "the riptide tip-over rotates the resting depth (Z) span up into the Y span"
    );
    assert!(
        (span(spin_min, spin_max, 2) - span(resting_min, resting_max, 1)).abs() < 1e-3,
        "the riptide tip-over lays the resting upright (Y) span down into the Z span"
    );
}

#[test]
fn death_flip_takes_precedence_over_auto_spin() {
    // Vanilla setupRotations is an else-if chain: `if deathTime > 0 ... else if
    // isAutoSpinAttack ...`, so a dying spinner tips over via the death flip and
    // ignores the riptide branch entirely.
    let base = EntityModelInstance::cow(912, [0.0, 64.0, 0.0], 0.0, false);
    let death_only = entity_model_mesh(&[base.with_death_time(20.0)]);
    let death_and_spin = entity_model_mesh(&[base
        .with_death_time(20.0)
        .with_auto_spin_age_ticks(Some(5.0))]);
    let spin_only = entity_model_mesh(&[base.with_auto_spin_age_ticks(Some(5.0))]);

    // The spin is suppressed while dying: the dying spinner matches the death-only flip.
    assert_eq!(death_only.vertices, death_and_spin.vertices);
    // And the death flip is a genuinely different pose from the riptide spin.
    assert_ne!(spin_only.vertices, death_and_spin.vertices);
}
