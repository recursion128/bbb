use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn blaze_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla BlazeModel.createBodyLayer: an 8x8x8 head at PartPose.ZERO plus twelve rods, each the
    // shared 2x8x2 `rod` CubeListBuilder addBox(0, 0, 0, 2, 8, 2). The rod layer offsets are
    // placeholders (setupAnim overwrites them every frame), so only the cube extents are
    // layer-stable. Each unified cube carries the colored tint (`BLAZE_ORANGE`) and the textured UV.
    assert_eq!(BLAZE_ROD_COUNT, 12);

    // Head: texOffs(0, 0), 8x8x8.
    assert_eq!(
        BLAZE_HEAD_CUBE[0],
        ModelCube::new(
            [-4.0, -4.0, -4.0],
            [8.0, 8.0, 8.0],
            BLAZE_ORANGE,
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    // Vanilla reuses one texOffs(0, 16) `rod` builder for all twelve rods.
    assert_eq!(
        BLAZE_ROD_CUBE[0],
        ModelCube::new(
            [0.0, 0.0, 0.0],
            [2.0, 8.0, 2.0],
            BLAZE_ORANGE,
            [2.0, 8.0, 2.0],
            [0.0, 16.0],
            false,
        )
    );
    assert_eq!(MODEL_LAYER_BLAZE, "minecraft:blaze#main");
    assert_eq!(BLAZE_TEXTURE_REF.size, [64, 32]);
}

#[test]
fn blaze_layer_passes_match_vanilla_renderer() {
    let passes = blaze_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::BlazeBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].model_layer, MODEL_LAYER_BLAZE);
    assert_eq!(passes[0].texture, BLAZE_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
}

#[test]
fn blaze_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Blaze.model_key(), "blaze");
    assert_eq!(
        EntityModelKind::Blaze.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/blaze/blaze.png",
            size: [64, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&BLAZE_TEXTURE_REF));
    assert_eq!(blaze_entity_texture_refs(), &[BLAZE_TEXTURE_REF]);
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
fn blaze_mesh_uses_vanilla_body_layer_geometry() {
    // Head + 12 rods = 13 cubes => 13*24 = 312 verts, 13*6 = 78 faces, 13*36 = 468 indices.
    // The blaze has no MeshTransformer scaling (unit model root transform).
    let mesh = entity_model_mesh(&[EntityModelInstance::blaze(14, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(mesh.opaque_faces, 78);
    assert_eq!(mesh.vertices.len(), 312);
    assert_eq!(mesh.indices.len(), 468);
    // The 8px head spans the highest point (model y -4 => world 65.751) and the radius-9
    // ring-0 rods reach +-9..11px in x/z (the 180-degree body yaw flips z), while the tall
    // ring-2 rods (model y ~20px) set the lowest point.
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.5625, 64.25349, -0.6875]);
    assert_close3(max, [0.6875, 65.751, 0.5625]);
}

#[test]
fn blaze_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&blaze_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::blaze(14, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 78);
    assert_eq!(mesh.vertices.len(), 312);
    assert_eq!(mesh.indices.len(), 468);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.5625, 64.25349, -0.6875]);
    assert_close3(max, [0.6875, 65.751, 0.5625]);
}

#[test]
fn blaze_rods_orbit_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: the twelve rods orbit while the head holds its
    // rest pose. The head is part 0 (vertices [0, 24)); the rods follow [24, 312).
    let base = EntityModelInstance::blaze(14, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(31.4)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the rods orbit as ageInTicks advances"
    );
    assert_eq!(
        early.vertices[..24],
        later.vertices[..24],
        "the head does not depend on ageInTicks at a rest head look"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&blaze_texture_images()).unwrap();
    let early_t = entity_model_textured_mesh(&[base], &atlas);
    let later_t = entity_model_textured_mesh(&[base.with_age_in_ticks(31.4)], &atlas);
    assert_ne!(
        early_t.vertices, later_t.vertices,
        "the textured rods orbit too"
    );
    assert_eq!(
        early_t.vertices[..24],
        later_t.vertices[..24],
        "the textured head does not depend on ageInTicks at a rest head look"
    );
}

#[test]
fn blaze_head_follows_look_angles() {
    // Vanilla BlazeModel.setupAnim sets head.yRot/xRot from the net head look. The head is
    // part 0 (vertices [0, 24)); a non-zero look re-poses only those vertices.
    let base = EntityModelInstance::blaze(14, [0.0, 64.0, 0.0], 0.0);
    let looking = base.with_head_look(35.0, -20.0);
    let rest = entity_model_mesh(&[base]);
    let turned = entity_model_mesh(&[looking]);
    assert_ne!(
        rest.vertices[..24],
        turned.vertices[..24],
        "the head turns with the look angles"
    );
    assert_eq!(
        rest.vertices[24..],
        turned.vertices[24..],
        "the rods do not depend on the head look"
    );
}

fn blaze_texture_images() -> Vec<EntityModelTextureImage> {
    blaze_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
