use super::*;

use crate::entity_models::model_layers::guardian_model;

use glam::Vec3;

#[test]
fn guardian_spike_pose_at_age_zero_matches_the_baked_bind_pose() {
    for i in 0..12 {
        let pose = guardian_spike_pose(i, 0.0, 0.0);
        let bind = layer_pose(ModelLayer::GUARDIAN, &format!("spike{i}"));
        for axis in 0..3 {
            assert!((pose.offset[axis] - bind.offset[axis]).abs() < 1.0e-5);
            assert!((pose.rotation[axis] - bind.rotation[axis]).abs() < 1.0e-6);
        }
    }
}

#[test]
fn guardian_eye_tracks_target_with_vanilla_horizontal_and_vertical_formula() {
    let base = EntityModelInstance::guardian(995, [0.0, 64.0, 0.0], 0.0, false);
    let eye_offset = |tracking: GuardianEyeTrackingRenderState| {
        let model = guardian_model(
            false,
            &base.with(|s| s.kind.guardian_mut().eye_tracking = Some(tracking)),
        );
        model
            .try_child("head")
            .unwrap()
            .try_child("eye")
            .expect("guardian eye child")
            .pose
            .offset
    };

    // Facing +Z and tracking a target east (+X) makes the rotated delta dot positive: Vanilla
    // applies `sqrt(abs(dot)) * 2 * signum(dot) = +2`. A target above keeps bind y = 0.
    assert_eq!(
        eye_offset(GuardianEyeTrackingRenderState {
            eye_position: [0.0, 65.0, 0.0],
            look_direction: [0.0, 0.0, 1.0],
            look_at_position: [1.0, 66.0, 0.0],
        }),
        [2.0, 0.0, -8.25]
    );

    // The mirrored west target flips the signed offset; a target at or below eye height moves the
    // eye down exactly one model unit.
    assert_eq!(
        eye_offset(GuardianEyeTrackingRenderState {
            eye_position: [0.0, 65.0, 0.0],
            look_direction: [0.0, 0.0, 1.0],
            look_at_position: [-1.0, 64.0, 0.0],
        }),
        [-2.0, 1.0, -8.25]
    );
}

#[test]
fn guardian_textured_render_matches_vanilla_renderer() {
    // The guardian and elder guardian share one mesh, differing only by texture (and the elder's 2.35
    // root scale).
    let guardian_passes = guardian_textured_layer_passes(false);
    assert_eq!(guardian_passes.len(), 2);
    assert_eq!(
        guardian_passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        guardian_passes[0].render_type.vanilla_name(),
        "entityCutout"
    );
    assert_eq!(guardian_passes[0].texture, GUARDIAN_TEXTURE_REF);
    assert_eq!(guardian_passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(guardian_passes[0].order, 0);
    assert_eq!(
        guardian_passes[1].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        guardian_passes[1].render_type.vanilla_name(),
        "entityCutout"
    );
    assert_eq!(guardian_passes[1].texture, GUARDIAN_BEAM_TEXTURE_REF);
    assert_eq!(guardian_passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(guardian_passes[1].order, 0);

    let elder_passes = guardian_textured_layer_passes(true);
    assert_eq!(elder_passes.len(), 2);
    assert_eq!(elder_passes[0].texture, GUARDIAN_ELDER_TEXTURE_REF);
    assert_eq!(elder_passes[1], guardian_passes[1]);
    assert!(entity_model_texture_refs().contains(&GUARDIAN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GUARDIAN_ELDER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GUARDIAN_BEAM_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [
        GUARDIAN_TEXTURE_REF,
        GUARDIAN_ELDER_TEXTURE_REF,
        GUARDIAN_BEAM_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for elder in [false, true] {
        let instance = EntityModelInstance::guardian(990, [0.0, 64.0, 0.0], 0.0, elder).with(|s| {
            s.light_coords = if elder {
                (6_u32 << 4) | (10_u32 << 20)
            } else {
                (5_u32 << 4) | (11_u32 << 20)
            };
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());

        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "elder={elder} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        assert!(mesh.vertices.iter().all(|vertex| vertex.light
            == instance.render_state.shader_light()
            && vertex.overlay == instance.render_state.overlay_coords()));
    }
}

#[test]
fn elder_guardian_particle_mesh_uses_vanilla_special_group_submission() {
    let atlas = guardian_beam_atlas();
    let transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)) * Mat4::from_rotation_x(0.25);
    let instance = ElderGuardianParticleRenderInstance {
        transform,
        tint: [1.0, 1.0, 1.0, 0.42],
    };

    let meshes = elder_guardian_particle_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(
        !meshes.translucent.vertices.is_empty(),
        "elder guardian particle emits textured translucent geometry"
    );

    assert!(meshes.translucent.vertices.iter().all(|vertex| {
        vertex.tint == instance.tint
            && vertex.light == ENTITY_VERTEX_FULL_BRIGHT_LIGHT
            && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
    }));
}

// Build an atlas covering the guardian base + beam textures, enough to render a beaming guardian.
fn guardian_beam_atlas() -> EntityModelTextureAtlasLayout {
    let images: Vec<EntityModelTextureImage> = [
        GUARDIAN_TEXTURE_REF,
        GUARDIAN_ELDER_TEXTURE_REF,
        GUARDIAN_BEAM_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn guardian_base_only_atlas() -> EntityModelTextureAtlasLayout {
    let images = [GUARDIAN_TEXTURE_REF, GUARDIAN_ELDER_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect::<Vec<_>>();
    build_entity_model_texture_atlas(&images).unwrap().0
}

#[test]
fn guardian_without_active_target_emits_no_beam() {
    // Vanilla `GuardianRenderer.submit`: with `attackTargetPosition == null` (no active attack target),
    // the beam is skipped entirely.
    let atlas = guardian_beam_atlas();
    let meshes = entity_model_textured_meshes(
        &[EntityModelInstance::guardian(
            500,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    assert!(
        meshes.guardian_beam.vertices.is_empty(),
        "a guardian with no active target emits no beam geometry"
    );
}

#[test]
fn guardian_beam_missing_beam_texture_texture_preserves_other_geometry() {
    let atlas = guardian_base_only_atlas();
    let instance = EntityModelInstance::guardian(501, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (2_u32 << 4) | (7_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.guardian_mut().beam = Some(GuardianBeamRenderState {
            eye_to_target: [0.0, 8.0, 0.0],
            eye_height: 0.5,
            attack_time: 0.0,
            attack_scale: 1.0,
        });
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());

    assert!(meshes.guardian_beam.vertices.is_empty());
    assert!(meshes.guardian_beam.indices.is_empty());
    assert!(meshes.opaque_beam_draws.is_empty());
}

#[test]
fn guardian_beam_emits_twisted_prism_with_scale_color() {
    // Vanilla `GuardianRenderer.renderBeam`: the beam is a 12-vertex twisted prism (two crossed
    // longitudinal strips + a top cap), folded into the scroll (tiled) pass — three quads → 18 indices.
    let atlas = guardian_beam_atlas();
    let instance = EntityModelInstance::guardian(501, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (2_u32 << 4) | (7_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.guardian_mut().beam = Some(GuardianBeamRenderState {
            eye_to_target: [0.0, 8.0, 0.0],
            eye_height: 0.5,
            attack_time: 0.0,
            attack_scale: 1.0,
        });
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.guardian_beam.vertices.len(), 12, "12 beam vertices");
    assert_eq!(
        meshes.guardian_beam.indices.len(),
        18,
        "3 quads → 18 indices"
    );
    assert_eq!(meshes.opaque_beam_draws.len(), 1);
    assert_eq!(
        (
            meshes.opaque_beam_draws[0].render_type,
            meshes.opaque_beam_draws[0].index_start,
            meshes.opaque_beam_draws[0].index_count
        ),
        (EntityModelLayerRenderType::EntityCutout, 0, 18)
    );

    // Vanilla color ramp at `attackScale = 1` (`colorScale = 1`): red `64+191=255`, green `32+191=223`,
    // blue `128-64=64`, alpha opaque.
    let expected = [1.0, 223.0 / 255.0, 64.0 / 255.0, 1.0];
    assert!(meshes
        .guardian_beam
        .vertices
        .iter()
        .all(|vertex| vertex.tint == expected && vertex.normal == [0.0, 1.0, 0.0]));

    // Every beam vertex carries the beam texture's atlas sub-rect for the shader's vertical tiling.
    let rect_size = meshes.guardian_beam.vertices[0].uv_rect_size;
    assert!(rect_size[0] > 0.0 && rect_size[1] > 0.0);
    // The longitudinal strips span the beam length in V (vanilla `maxV = minV + length * 2.5`,
    // `length = |beamVector| + 1`): the V extent is `(8 + 1) * 2.5 = 22.5` tiles.
    let v_values: Vec<f32> = meshes
        .guardian_beam
        .vertices
        .iter()
        .map(|vertex| vertex.local_uv[1])
        .collect();
    let v_span = v_values.iter().cloned().fold(f32::MIN, f32::max)
        - v_values.iter().cloned().fold(f32::MAX, f32::min);
    assert!(
        (v_span - 22.5).abs() < 0.01,
        "beam V tiles over length * 2.5 = 22.5, got {v_span}"
    );
}

#[test]
fn guardian_beam_orients_along_world_target_vector() {
    // The beam is built in a world-aligned frame and orients its local +Y onto the world `eye_to_target`
    // vector, so a beam fired straight along +X extends in world +X (and one along +Z extends in +Z),
    // independent of the guardian's body yaw.
    let atlas = guardian_beam_atlas();
    let beam_along = |delta: [f32; 3], y_rot: f32| {
        entity_model_textured_meshes(
            &[
                EntityModelInstance::guardian(502, [0.0, 64.0, 0.0], y_rot, false).with(|s| {
                    s.kind.guardian_mut().beam = Some(GuardianBeamRenderState {
                        eye_to_target: delta,
                        eye_height: 0.5,
                        attack_time: 0.0,
                        attack_scale: 0.5,
                    })
                }),
            ],
            &atlas,
        )
    };
    let extent = |positions: Vec<f32>| {
        positions.iter().cloned().fold(f32::MIN, f32::max)
            - positions.iter().cloned().fold(f32::MAX, f32::min)
    };
    // Length = 10 + 1 = 11; a beam along +X reaches ~11 in X but stays thin (radius ≤ 0.282) in Z, and
    // vice versa — regardless of the body yaw, which the beam ignores.
    let along_x = beam_along([10.0, 0.0, 0.0], 1.3);
    let along_x_x: Vec<f32> = along_x
        .guardian_beam
        .vertices
        .iter()
        .map(|v| v.position[0])
        .collect();
    let along_x_z: Vec<f32> = along_x
        .guardian_beam
        .vertices
        .iter()
        .map(|v| v.position[2])
        .collect();
    assert!(extent(along_x_x) > 9.0, "beam along +X spans X");
    assert!(extent(along_x_z) < 1.0, "beam along +X is thin in Z");
    let along_z = beam_along([0.0, 0.0, 10.0], 2.7);
    let along_z_z: Vec<f32> = along_z
        .guardian_beam
        .vertices
        .iter()
        .map(|v| v.position[2])
        .collect();
    let along_z_x: Vec<f32> = along_z
        .guardian_beam
        .vertices
        .iter()
        .map(|v| v.position[0])
        .collect();
    assert!(extent(along_z_z) > 9.0, "beam along +Z spans Z");
    assert!(extent(along_z_x) < 1.0, "beam along +Z is thin in X");
}
