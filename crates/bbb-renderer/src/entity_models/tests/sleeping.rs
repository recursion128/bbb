use super::*;

fn sleeping(yaw_angle: f32, bed_offset: [f32; 2]) -> SleepingPose {
    SleepingPose {
        yaw_angle,
        bed_offset,
    }
}

#[test]
fn sleeping_is_inert_when_none() {
    // No sleeping pose (None) leaves the model standing: the mesh is byte-identical
    // to the resting render, so the sleeping branch never fires for an awake entity.
    let base = EntityModelInstance::cow(930, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let awake = entity_model_mesh(&[base.with_sleeping(None)]);

    assert_eq!(resting.vertices, awake.vertices);
}

#[test]
fn sleeping_lays_the_model_down() {
    // Vanilla setupRotations sleeping branch: the `180 - bodyRot` yaw is skipped and
    // the model is laid on its side via Ry(angle) * Rz(getFlipDegrees) * Ry(270). The
    // Rz flip changes the upright Y extent, which a pure body yaw never would.
    let base = EntityModelInstance::cow(931, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let asleep = entity_model_mesh(&[base.with_sleeping(Some(sleeping(0.0, [0.0, 0.0])))]);

    assert_eq!(resting.vertices.len(), asleep.vertices.len());
    assert_ne!(resting.vertices, asleep.vertices);

    // A pure body yaw rotates about the vertical axis and so preserves the multiset
    // of vertex heights; the sleeping Rz(getFlipDegrees) flip lays the model on its
    // side and redistributes those heights, proving it is more than a yaw.
    let mut resting_y: Vec<f32> = resting.vertices.iter().map(|v| v.position[1]).collect();
    let mut asleep_y: Vec<f32> = asleep.vertices.iter().map(|v| v.position[1]).collect();
    resting_y.sort_by(f32::total_cmp);
    asleep_y.sort_by(f32::total_cmp);
    let max_height_shift = resting_y
        .iter()
        .zip(&asleep_y)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_height_shift > 0.3,
        "the sleeping flip redistributes vertex heights (it is not a mere body yaw)"
    );
}

#[test]
fn sleeping_bed_offset_translates_the_model_in_world_space() {
    // Vanilla submit() bed head offset is applied before the entity scale, so it is a
    // pure world-space translate: shifting the offset shifts every vertex by exactly
    // the same world delta.
    let base = EntityModelInstance::cow(932, [0.0, 64.0, 0.0], 0.0, false);
    let centered = entity_model_mesh(&[base.with_sleeping(Some(sleeping(0.0, [0.0, 0.0])))]);
    let shifted = entity_model_mesh(&[base.with_sleeping(Some(sleeping(0.0, [2.0, 0.0])))]);

    let (centered_min, centered_max) = mesh_extents(&centered);
    let (shifted_min, shifted_max) = mesh_extents(&shifted);
    // The whole model slides +2 along world X and nowhere else.
    assert!((shifted_min[0] - centered_min[0] - 2.0).abs() < 1e-3);
    assert!((shifted_max[0] - centered_max[0] - 2.0).abs() < 1e-3);
    assert!((shifted_min[2] - centered_min[2]).abs() < 1e-3);
    assert!((shifted_max[1] - centered_max[1]).abs() < 1e-3);
}

#[test]
fn sleeping_yaw_rotates_about_vertical() {
    // The bed-direction angle rotates the laid-down model about the vertical axis, so
    // different bed orientations render different frames.
    let base = EntityModelInstance::cow(933, [0.0, 64.0, 0.0], 0.0, false);
    let north = entity_model_mesh(&[base.with_sleeping(Some(sleeping(270.0, [0.0, 0.0])))]);
    let east = entity_model_mesh(&[base.with_sleeping(Some(sleeping(180.0, [0.0, 0.0])))]);

    assert_eq!(north.vertices.len(), east.vertices.len());
    assert_ne!(north.vertices, east.vertices);
}

#[test]
fn sleeping_precedence_matches_vanilla_else_if_chain() {
    // Vanilla else-if chain: death > auto-spin > sleeping > upside-down, while the
    // `180 - bodyRot` yaw is skipped whenever the entity is sleeping.
    let base = EntityModelInstance::cow(934, [0.0, 64.0, 0.0], 0.0, false);
    let asleep = entity_model_mesh(&[base.with_sleeping(Some(sleeping(0.0, [0.0, 0.0])))]);

    // Sleeping takes precedence over the upside-down flip: the upside-down height is
    // ignored while sleeping.
    let asleep_and_upside = entity_model_mesh(&[base
        .with_sleeping(Some(sleeping(0.0, [0.0, 0.0])))
        .with_upside_down_height(Some(1.4))]);
    assert_eq!(asleep.vertices, asleep_and_upside.vertices);

    // Death takes precedence over the sleeping branch, but the sleeping yaw skip
    // still applies, so a dying sleeper matches neither death-only nor sleeping-only.
    let death_only = entity_model_mesh(&[base.with_death_time(20.0)]);
    let death_and_sleep = entity_model_mesh(&[base
        .with_death_time(20.0)
        .with_sleeping(Some(sleeping(0.0, [0.0, 0.0])))]);
    assert_ne!(death_and_sleep.vertices, death_only.vertices);
    assert_ne!(death_and_sleep.vertices, asleep.vertices);
}
