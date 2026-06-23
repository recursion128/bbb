use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn wind_charge_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WindChargeModel.createBodyLayer` (atlas 64×32): the `bone` root (no cubes) parents the
    // `wind` shell (a fixed `yRot = -0.7854` ≈ -π/4, two boxes) and the `wind_charge` core box.
    assert_eq!(WIND_CHARGE_PARTS.len(), 1);
    let bone = &WIND_CHARGE_PARTS[0];
    assert_eq!(bone.pose.offset, [0.0, 0.0, 0.0]);
    assert!(bone.cubes.is_empty());
    assert_eq!(bone.children.len(), 2);

    // `wind`: the -π/4 bind rotation plus the `texOffs(15, 20)` 8×2×8 and `texOffs(0, 9)` 6×4×6 boxes.
    let wind = &bone.children[0];
    assert_eq!(wind.pose.rotation[0], 0.0);
    assert!((wind.pose.rotation[1] - (-std::f32::consts::FRAC_PI_4)).abs() < 1.0e-4);
    assert_eq!(wind.pose.rotation[2], 0.0);
    assert_eq!(wind.cubes.len(), 2);
    assert_eq!(wind.cubes[0].min, [-4.0, -1.0, -4.0]);
    assert_eq!(wind.cubes[0].size, [8.0, 2.0, 8.0]);
    assert_eq!(wind.cubes[1].min, [-3.0, -2.0, -3.0]);
    assert_eq!(wind.cubes[1].size, [6.0, 4.0, 6.0]);

    // `wind_charge`: the 4×4×4 core box at ZERO with no rotation.
    let core = &bone.children[1];
    assert_eq!(core.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(core.pose.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(core.cubes.len(), 1);
    assert_eq!(core.cubes[0].min, [-2.0, -2.0, -2.0]);
    assert_eq!(core.cubes[0].size, [4.0, 4.0, 4.0]);

    assert_eq!(count_cubes(&WIND_CHARGE_PARTS), 3);
}

#[test]
fn wind_charge_mesh_uses_vanilla_body_layer_geometry() {
    // 3 cubes → 18 faces / 72 vertices / 108 indices; the wind shell and the core carry their tints.
    let charge = entity_model_mesh(&[EntityModelInstance::wind_charge(180, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(charge.opaque_faces, 18);
    assert_eq!(charge.vertices.len(), 72);
    assert_eq!(charge.indices.len(), 108);
    assert!(charge
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WIND_CHARGE_WIND, 1.0)));
    assert!(charge
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WIND_CHARGE_CORE, 1.0)));
}

#[test]
fn wind_charge_spin_matches_vanilla_setup_anim() {
    // `wind.yRot = age·16°·π/180` (set, overwriting the -π/4 bind); `windCharge.yRot = -that`.
    let age = 5.0_f32;
    let expected = (age * 16.0).to_radians();
    assert!((wind_charge_spin_yrot(age) - expected).abs() < 1.0e-6);
    assert_eq!(wind_charge_spin_yrot(0.0), 0.0);
}

#[test]
fn wind_charge_counter_spins_shell_and_core_with_age() {
    // Vanilla `WindChargeModel.setupAnim` sets `wind.yRot = age·16°` (overwriting the -π/4 bind) and
    // `windCharge.yRot = -age·16°`. So at age 0 the shell sits axis-aligned: its 8×2×8 slab
    // half-extent is 0.25 block, so no vertex passes |x| = 0.30. At age 2.8125 the shell turns 45°,
    // mapping its corner (0.25, 0.25) to (0.354, 0) — reaching ~0.354 on X.
    let rest = entity_model_mesh(&[EntityModelInstance::wind_charge(181, [0.0, 0.0, 0.0], 0.0)]);
    assert!(
        !rest.vertices.iter().any(|v| v.position[0].abs() > 0.30),
        "at age 0 the spin is zero, so the shell sits axis-aligned (the -π/4 bind is overwritten)"
    );

    let age = 45.0_f32 / 16.0; // age·16° = 45°
    let spun = entity_model_mesh(&[
        EntityModelInstance::wind_charge(182, [0.0, 0.0, 0.0], 0.0).with_age_in_ticks(age)
    ]);
    assert!(
        spun.vertices.iter().any(|v| v.position[0].abs() > 0.30),
        "the age-driven spin swings the shell off-axis"
    );
    assert_eq!(rest.vertices.len(), spun.vertices.len());
    assert_ne!(
        rest.vertices, spun.vertices,
        "both the shell and the counter-spun core turn with age"
    );
}
