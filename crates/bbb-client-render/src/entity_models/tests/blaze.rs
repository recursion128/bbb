use super::*;

#[test]
fn blaze_layer_passes_match_vanilla_renderer() {
    let passes = blaze_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, BLAZE_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn blaze_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&BLAZE_TEXTURE_REF));
}

#[test]
fn blaze_rod_offset_matches_vanilla_setup_anim() {
    // Vanilla BlazeModel.setupAnim positions each rod from ageInTicks. Three rings of four:
    //   ring 0 (0..4):  radius 9, y = -2 + cos((2i + age) * 0.25), base angle = age*pi*-0.1
    //   ring 1 (4..8):  radius 7, y =  2 + cos((2i + age) * 0.25), base angle = pi/4 + age*pi*0.03
    //   ring 2 (8..12): radius 5, y = 11 + cos((1.5i + age) * 0.5), base angle = 0.47123894 + age*pi*-0.05
    // Each rod's angle adds (index % 4) * pi/2 onto its ring base; x = cos(angle)*r, z = sin(angle)*r.
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
    let vanilla = |index: usize, age: f32| -> [f32; 3] {
        let i = index as f32;
        let (radius, y, base) = if index < 4 {
            (9.0, -2.0 + ((2.0 * i + age) * 0.25).cos(), age * PI * -0.1)
        } else if index < 8 {
            (
                7.0,
                2.0 + ((2.0 * i + age) * 0.25).cos(),
                FRAC_PI_4 + age * PI * 0.03,
            )
        } else {
            (
                5.0,
                11.0 + ((1.5 * i + age) * 0.5).cos(),
                0.47123894 + age * PI * -0.05,
            )
        };
        let angle = base + (index % 4) as f32 * FRAC_PI_2;
        [angle.cos() * radius, y, angle.sin() * radius]
    };
    for age in [0.0f32, 7.5, 31.4, 100.0] {
        for index in 0..BLAZE_ROD_COUNT {
            let got = blaze_rod_offset(index, age);
            let want = vanilla(index, age);
            for axis in 0..3 {
                assert!(
                    (got[axis] - want[axis]).abs() < 1e-6,
                    "rod {index} axis {axis} at age {age}: {got:?} vs {want:?}"
                );
            }
        }
    }
    // At age 0 the first rod of ring 0 sits at (radius 9, 0) with y = -2 + cos(0) = -1.
    assert!((blaze_rod_offset(0, 0.0)[0] - 9.0).abs() < 1e-6);
    assert!((blaze_rod_offset(0, 0.0)[1] - (-1.0)).abs() < 1e-6);
    assert!(blaze_rod_offset(0, 0.0)[2].abs() < 1e-6);
}

#[test]
fn blaze_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&blaze_texture_images()).unwrap();
    let instance = EntityModelInstance::blaze(14, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (15_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 78);
    assert_eq!(meshes.cutout.vertices.len(), 312);
    assert_eq!(meshes.cutout.indices.len(), 468);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert_eq!(instance.render_state.shader_light(), [1.0, 9.0 / 15.0]);
    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(&meshes.cutout);
    assert_close3(min, [-0.5625, 64.25349, -0.6875]);
    assert_close3(max, [0.6875, 65.751, 0.5625]);
}

#[test]
fn blaze_rods_orbit_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: the twelve rods orbit while the head holds its
    // rest pose. The head is the fourth child of the baked root (vertices [72, 96)).
    let base = EntityModelInstance::blaze(14, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with(|s| s.age_in_ticks = 31.4)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the rods orbit as ageInTicks advances"
    );
    assert_eq!(
        early.vertices[72..96],
        later.vertices[72..96],
        "the head does not depend on ageInTicks at a rest head look"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&blaze_texture_images()).unwrap();
    let early_t = entity_model_textured_meshes(&[base], &atlas);

    let later_instance = base.with(|s| s.age_in_ticks = 31.4);
    let later_t = entity_model_textured_meshes(&[later_instance], &atlas);

    assert_ne!(
        early_t.cutout.vertices, later_t.cutout.vertices,
        "the textured rods orbit too"
    );
    assert_eq!(
        early_t.cutout.vertices[72..96],
        later_t.cutout.vertices[72..96],
        "the textured head does not depend on ageInTicks at a rest head look"
    );
}

fn blaze_texture_images() -> Vec<EntityModelTextureImage> {
    [BLAZE_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
