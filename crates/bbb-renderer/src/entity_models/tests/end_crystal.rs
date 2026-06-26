use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn end_crystal_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `EndCrystalModel.createBodyLayer` (atlas 64×32): the base slab at ZERO plus the
    // concentric glass stack at offset (0, 24, 0) — outer (unscaled), inner (`withScale(0.875)`),
    // and the core (cumulative `0.875 · 0.765625`).
    assert_eq!(END_CRYSTAL_PARTS.len(), 4);

    // `base` (12×4×12) at the model origin.
    let base = &END_CRYSTAL_PARTS[0];
    assert_eq!(base.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(base.cubes[0].min, [-6.0, 0.0, -6.0]);
    assert_eq!(base.cubes[0].size, [12.0, 4.0, 12.0]);

    // `outer_glass`: the unscaled 8×8×8 cube at (0, 24, 0).
    let outer = &END_CRYSTAL_PARTS[1];
    assert_eq!(outer.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(outer.cubes[0].size, [8.0, 8.0, 8.0]);

    // `inner_glass`: the 8×8×8 cube baked at `withScale(0.875)` → a centred 7×7×7 box.
    let inner = &END_CRYSTAL_PARTS[2];
    assert_eq!(inner.pose.offset, [0.0, 24.0, 0.0]);
    let inner_scale = 0.875_f32;
    assert!((inner.cubes[0].size[0] - 8.0 * inner_scale).abs() < 1.0e-6);
    assert_eq!(inner.cubes[0].size, [7.0, 7.0, 7.0]);
    assert_eq!(inner.cubes[0].min, [-3.5, -3.5, -3.5]);

    // `cube`: the core 8×8×8 cube baked at the cumulative `0.875 · 0.765625` scale.
    let core = &END_CRYSTAL_PARTS[3];
    assert_eq!(core.pose.offset, [0.0, 24.0, 0.0]);
    let core_scale = 0.875_f32 * 0.765625_f32;
    assert!((core.cubes[0].size[0] - 8.0 * core_scale).abs() < 1.0e-6);
    assert!((core.cubes[0].min[0] - (-4.0 * core_scale)).abs() < 1.0e-6);
    assert_eq!(core.cubes[0].size, [5.359375, 5.359375, 5.359375]);

    // The three glass boxes share the same centre and shrink monotonically.
    assert!(outer.cubes[0].size[0] > inner.cubes[0].size[0]);
    assert!(inner.cubes[0].size[0] > core.cubes[0].size[0]);

    // Four cubes total.
    assert_eq!(count_cubes(&END_CRYSTAL_PARTS), 4);
}

#[test]
fn end_crystal_mesh_uses_vanilla_body_layer_geometry() {
    // 4 cubes → 24 faces / 96 vertices / 144 indices; the glass, core, and base carry their tints.
    let crystal =
        entity_model_mesh(&[EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(crystal.opaque_faces, 24);
    assert_eq!(crystal.vertices.len(), 96);
    assert_eq!(crystal.indices.len(), 144);
    assert!(crystal
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_GLASS, 1.0)));
    assert!(crystal
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_CORE, 1.0)));
    assert!(crystal
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_BASE, 1.0)));
}

#[test]
fn end_crystal_colored_runtime_skips_the_texture_backed_crystal() {
    let instances = [EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0)];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn end_crystal_textured_submit_matches_vanilla_renderer() {
    // Vanilla `EndCrystalRenderer.submit`: submit the `EndCrystalModel` with
    // `textures/entity/end_crystal/end_crystal.png`, the default `EntityModel` render type
    // (`entityCutout`), `order(0)`, and the renderer root transform `scale(2)·translate(0,-0.5,0)`.
    assert_eq!(
        EntityModelKind::EndCrystal.vanilla_texture_ref(),
        Some(END_CRYSTAL_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&END_CRYSTAL_TEXTURE_REF));
    assert_eq!(
        end_crystal_entity_texture_refs(),
        &[END_CRYSTAL_TEXTURE_REF]
    );

    let len =
        usize::try_from(END_CRYSTAL_TEXTURE_REF.size[0] * END_CRYSTAL_TEXTURE_REF.size[1] * 4)
            .unwrap();
    let images = vec![EntityModelTextureImage::new(
        END_CRYSTAL_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(30.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.texture, END_CRYSTAL_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.order, 0);
    assert_eq!(submit.submit_sequence, 0);
    assert_eq!(submit.transform, end_crystal_model_root_transform(instance));

    assert_eq!(meshes.cutout.vertices.len(), 96);
    assert_eq!(meshes.cutout.indices.len(), 144);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let colored = entity_model_mesh(&[instance]);
    let (colored_min, colored_max) = mesh_extents(&colored);
    let (textured_min, textured_max) = textured_mesh_extents(&meshes.cutout);
    assert_close3(textured_min, colored_min);
    assert_close3(textured_max, colored_max);
}

#[test]
fn end_crystal_hides_base_when_shows_bottom_false() {
    // Vanilla `EndCrystalModel.setupAnim`: `base.visible = showsBottom`. The default instance
    // shows the base (vanilla default `true`); clearing `showsBottom` drops the base slab
    // (`END_CRYSTAL_PARTS[0]`, one cube): 24→18 faces, 96→72 vertices, 144→108 indices, and the
    // base tint disappears while the glass/core stack is untouched.
    let shown = entity_model_mesh(&[EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(shown.opaque_faces, 24);

    let hidden = entity_model_mesh(
        &[EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0)
            .with_end_crystal_shows_bottom(false)],
    );
    assert_eq!(hidden.opaque_faces, 18);
    assert_eq!(hidden.vertices.len(), 72);
    assert_eq!(hidden.indices.len(), 108);
    assert!(!hidden
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_BASE, 1.0)));
    assert!(hidden
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_GLASS, 1.0)));
    assert!(hidden
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_CORE, 1.0)));
}

#[test]
fn end_crystal_textured_submit_hides_base_when_shows_bottom_false() {
    let len =
        usize::try_from(END_CRYSTAL_TEXTURE_REF.size[0] * END_CRYSTAL_TEXTURE_REF.size[1] * 4)
            .unwrap();
    let images = vec![EntityModelTextureImage::new(
        END_CRYSTAL_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let hidden = entity_model_textured_meshes(
        &[EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0)
            .with_end_crystal_shows_bottom(false)],
        &atlas,
    );
    assert_eq!(hidden.submissions.len(), 1);
    assert_eq!(
        hidden.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(hidden.cutout.vertices.len(), 72);
    assert_eq!(hidden.cutout.indices.len(), 108);
}

#[test]
fn end_crystal_bob_matches_vanilla_get_y() {
    // Vanilla `EndCrystalRenderer.getY`: hh = sin(t·0.2)/2 + 0.5; hh = (hh² + hh)·0.4; return hh − 1.4.
    // The glass bob is `getY(age)·16/2`.
    for age in [0.0_f32, 7.5, 30.0, 100.0] {
        let hh = (age * 0.2).sin() / 2.0 + 0.5;
        let hh = (hh * hh + hh) * 0.4;
        let expected = hh - 1.4;
        assert!((end_crystal_get_y(age) - expected).abs() < 1.0e-6);
        assert!((end_crystal_bob_y(age) - expected * 8.0).abs() < 1.0e-6);
    }
    // getY is always negative — the crystal hovers above its base.
    assert!(end_crystal_get_y(0.0) < 0.0);
}

#[test]
fn end_crystal_glass_spin_matches_vanilla_setup_anim() {
    use glam::Vec3;

    // At age 0 the spin is identity, so both quaternions are the π/3 tilt about the (sin45, 0, sin45)
    // diagonal. Rotating +Y by 60° about that axis gives, by Rodrigues, (-0.61237, 0.5, 0.61237).
    let (outer0, inner0) = end_crystal_glass_quaternions(0.0);
    let up = outer0 * Vec3::Y;
    assert!((up.x - (-0.61237)).abs() < 1.0e-4, "x was {}", up.x);
    assert!((up.y - 0.5).abs() < 1.0e-4, "y was {}", up.y);
    assert!((up.z - 0.61237).abs() < 1.0e-4, "z was {}", up.z);
    // Both shells share the tilt when the spin is zero.
    assert!((outer0 * Vec3::Y).abs_diff_eq(inner0 * Vec3::Y, 1.0e-6));

    // Advancing the age spins the shells, and the outer (`Ry·TILT`) and inner (`TILT·Ry`) orders
    // diverge — the order distinction is the vanilla detail this proves.
    let (outer, inner) = end_crystal_glass_quaternions(30.0);
    assert!(
        !(outer * Vec3::Y).abs_diff_eq(outer0 * Vec3::Y, 1.0e-3),
        "the outer glass spins with age"
    );
    assert!(
        !(outer * Vec3::Y).abs_diff_eq(inner * Vec3::Y, 1.0e-3),
        "the outer and inner spin orders differ"
    );
}

#[test]
fn end_crystal_spins_and_bobs_the_glass_with_age() {
    // The four cubes emit base [0, 24), outer glass [24, 48), inner glass [48, 72), core [72, 96).
    // The base holds across age; the whole glass stack spins (the always-on π/3 tilt plus the
    // age-driven Y spin) and bobs (`getY`), so its vertices move while the count is preserved.
    let rest = entity_model_mesh(&[EntityModelInstance::end_crystal(451, [0.0, 64.0, 0.0], 0.0)]);
    let later = entity_model_mesh(&[
        EntityModelInstance::end_crystal(452, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(30.0)
    ]);
    assert_eq!(rest.vertices.len(), 96);
    assert_eq!(later.vertices.len(), 96);
    assert_eq!(
        rest.vertices[..24],
        later.vertices[..24],
        "the base slab holds"
    );
    assert_ne!(
        rest.vertices[24..],
        later.vertices[24..],
        "the glass stack spins and bobs with age"
    );
}
