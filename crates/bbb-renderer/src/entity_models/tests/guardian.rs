use super::*;

use glam::Vec3;
use std::f32::consts::PI;

#[test]
fn guardian_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `GuardianModel.createBodyLayer` (atlas 64×64). The `head` part carries five body
    // cubes: the main 12×12×16 box, two 2×12×12 side plates, and the bottom/top 12×2×12 plates.
    assert_eq!(GUARDIAN_HEAD.len(), 5);
    assert_eq!(GUARDIAN_HEAD[0].min, [-6.0, 10.0, -8.0]);
    assert_eq!(GUARDIAN_HEAD[0].size, [12.0, 12.0, 16.0]);
    assert_eq!(GUARDIAN_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(GUARDIAN_HEAD[1].min, [-8.0, 10.0, -6.0]);
    assert_eq!(GUARDIAN_HEAD[1].size, [2.0, 12.0, 12.0]);
    assert_eq!(GUARDIAN_HEAD[1].tex, [0.0, 28.0]);
    assert!(!GUARDIAN_HEAD[1].mirror);
    assert_eq!(GUARDIAN_HEAD[2].min, [6.0, 10.0, -6.0]);
    // The right side plate is the mirrored twin of the left (same `texOffs(0,28)`, `mirror()`).
    assert_eq!(GUARDIAN_HEAD[2].tex, [0.0, 28.0]);
    assert!(GUARDIAN_HEAD[2].mirror);
    assert_eq!(GUARDIAN_HEAD[3].min, [-6.0, 8.0, -6.0]);
    assert_eq!(GUARDIAN_HEAD[3].size, [12.0, 2.0, 12.0]);
    assert_eq!(GUARDIAN_HEAD[3].tex, [16.0, 40.0]);
    assert_eq!(GUARDIAN_HEAD[4].min, [-6.0, 22.0, -6.0]);
    assert_eq!(GUARDIAN_HEAD[4].tex, [16.0, 40.0]);

    // The shared spike box `addBox(-1, -4.5, -1, 2, 9, 2)`, texOffs(0,0).
    assert_eq!(GUARDIAN_SPIKE[0].min, [-1.0, -4.5, -1.0]);
    assert_eq!(GUARDIAN_SPIKE[0].size, [2.0, 9.0, 2.0]);
    assert_eq!(GUARDIAN_SPIKE[0].tex, [0.0, 0.0]);

    // The eye `addBox(-1, 15, 0, 2, 2, 1)` at `offset(0, 0, -8.25)`, texOffs(8,0).
    assert_eq!(GUARDIAN_EYE_CUBE[0].min, [-1.0, 15.0, 0.0]);
    assert_eq!(GUARDIAN_EYE_CUBE[0].size, [2.0, 2.0, 1.0]);
    assert_eq!(GUARDIAN_EYE_CUBE[0].tex, [8.0, 0.0]);
    assert_eq!(GUARDIAN_EYE_POSE.offset, [0.0, 0.0, -8.25]);

    // The three-segment tail: tail0 (ZERO, texOffs(40,0)), tail1 at (-1.5, 0.5, 14) texOffs(0,54),
    // tail2 (two cubes) at (0.5, 0.5, 6) texOffs(41,32)/(25,19).
    assert_eq!(GUARDIAN_TAIL0[0].min, [-2.0, 14.0, 7.0]);
    assert_eq!(GUARDIAN_TAIL0[0].size, [4.0, 4.0, 8.0]);
    assert_eq!(GUARDIAN_TAIL0[0].tex, [40.0, 0.0]);
    assert_eq!(GUARDIAN_TAIL1[0].size, [3.0, 3.0, 7.0]);
    assert_eq!(GUARDIAN_TAIL1[0].tex, [0.0, 54.0]);
    assert_eq!(GUARDIAN_TAIL1_POSE.offset, [-1.5, 0.5, 14.0]);
    assert_eq!(GUARDIAN_TAIL2.len(), 2);
    assert_eq!(GUARDIAN_TAIL2[0].size, [2.0, 2.0, 6.0]);
    assert_eq!(GUARDIAN_TAIL2[0].tex, [41.0, 32.0]);
    assert_eq!(GUARDIAN_TAIL2[1].min, [1.0, 10.5, 3.0]);
    assert_eq!(GUARDIAN_TAIL2[1].size, [1.0, 9.0, 9.0]);
    assert_eq!(GUARDIAN_TAIL2[1].tex, [25.0, 19.0]);
    assert_eq!(GUARDIAN_TAIL2_POSE.offset, [0.5, 0.5, 6.0]);

    // The `SPIKE_*` tables transcribed verbatim from `GuardianModel`.
    assert_eq!(
        GUARDIAN_SPIKE_X_ROT,
        [1.75, 0.25, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5, 1.25, 0.75, 0.0, 0.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Y_ROT,
        [0.0, 0.0, 0.0, 0.0, 0.25, 1.75, 1.25, 0.75, 0.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Z_ROT,
        [0.0, 0.0, 0.25, 1.75, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.75, 1.25]
    );
    assert_eq!(
        GUARDIAN_SPIKE_X,
        [0.0, 0.0, 8.0, -8.0, -8.0, 8.0, 8.0, -8.0, 0.0, 0.0, 8.0, -8.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Y,
        [-8.0, -8.0, -8.0, -8.0, 0.0, 0.0, 0.0, 0.0, 8.0, 8.0, 8.0, 8.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Z,
        [8.0, -8.0, 0.0, 0.0, -8.0, -8.0, 8.0, 8.0, 8.0, -8.0, 0.0, 0.0]
    );
}

#[test]
fn guardian_spike_bind_pose_matches_vanilla_get_spike() {
    // Vanilla `createBodyLayer` places spike `i` at `getSpike{X,Y,Z}(i, 0, 0)` with rotation
    // `PI * SPIKE_{X,Y,Z}_ROT[i]`, where `getSpikeOffset(i, 0, 0) = 1 + cos(i) * 0.01` and the
    // Y base adds 16.
    for i in 0..12 {
        let factor = 1.0 + (i as f32).cos() * 0.01;
        let pose = guardian_spike_bind_pose(i);
        assert!((pose.offset[0] - GUARDIAN_SPIKE_X[i] * factor).abs() < 1.0e-6);
        assert!((pose.offset[1] - (16.0 + GUARDIAN_SPIKE_Y[i] * factor)).abs() < 1.0e-6);
        assert!((pose.offset[2] - GUARDIAN_SPIKE_Z[i] * factor).abs() < 1.0e-6);
        assert!((pose.rotation[0] - PI * GUARDIAN_SPIKE_X_ROT[i]).abs() < 1.0e-6);
        assert!((pose.rotation[1] - PI * GUARDIAN_SPIKE_Y_ROT[i]).abs() < 1.0e-6);
        assert!((pose.rotation[2] - PI * GUARDIAN_SPIKE_Z_ROT[i]).abs() < 1.0e-6);
    }
}

#[test]
fn guardian_mesh_uses_vanilla_body_layer_geometry() {
    // 5 head cubes + 12 spikes + 1 eye + 3 tail cubes (1 + 1 + 2) = 22 cubes → 132 faces /
    // 528 vertices.
    let guardian = entity_model_mesh(&[EntityModelInstance::guardian(
        990,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(guardian.opaque_faces, 132);
    assert_eq!(guardian.vertices.len(), 528);
    assert_eq!(guardian.indices.len(), 792);
    // The body uses the guardian body color; the eye uses its own pink tint.
    assert!(guardian
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GUARDIAN_BODY, 1.0)));
    assert!(guardian
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GUARDIAN_EYE, 1.0)));
}

#[test]
fn elder_guardian_is_the_guardian_mesh_scaled_up() {
    // The elder guardian is the same 22-cube mesh scaled 2.35× by `ELDER_GUARDIAN_SCALE`, so it
    // keeps the cube count but occupies a larger world-space extent.
    assert_eq!(GUARDIAN_ELDER_SCALE, 2.35);
    let guardian = entity_model_mesh(&[EntityModelInstance::guardian(
        991,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let elder = entity_model_mesh(&[EntityModelInstance::guardian(
        992,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(guardian.vertices.len(), elder.vertices.len());
    assert_ne!(guardian.vertices, elder.vertices, "the elder is scaled up");

    let (g_min, g_max) = mesh_extents(&guardian);
    let (e_min, e_max) = mesh_extents(&elder);
    let guardian_width = g_max[0] - g_min[0];
    let elder_width = e_max[0] - e_min[0];
    assert!(
        elder_width > guardian_width * 2.0,
        "the elder guardian is ~2.35× the guardian's size ({guardian_width} vs {elder_width})"
    );
}

#[test]
fn guardian_spikes_pulse_with_age() {
    // Vanilla `GuardianModel.setupAnim` pulses each spike's offset by
    // `1 + cos(ageInTicks · 1.5 + i) · 0.01`, so the spikes breathe in and out as the entity ages.
    // The pulse tracks the age phase, and `ageInTicks = 0` is the baked bind pose (`cos(i)`).
    let base = EntityModelInstance::guardian(995, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]); // ageInTicks = 0 (bind)
    let aged = entity_model_mesh(&[base.with_age_in_ticks(10.0)]);
    assert_eq!(rest.vertices.len(), aged.vertices.len());
    assert_ne!(
        rest.vertices, aged.vertices,
        "the spikes pulse in and out with the entity age"
    );

    // The pulse advances with the age phase (a later age gives a different spike pose again).
    let aged_later = entity_model_mesh(&[base.with_age_in_ticks(20.0)]);
    assert_ne!(aged.vertices, aged_later.vertices);

    // `ageInTicks = 0` reproduces the baked bind spike pose (the `createBodyLayer` phase `cos(i)`).
    let zero = entity_model_mesh(&[base.with_age_in_ticks(0.0)]);
    assert_eq!(
        zero.vertices, rest.vertices,
        "ageInTicks = 0 is the bind spike pose"
    );
}

#[test]
fn guardian_spikes_retract_with_the_withdrawal() {
    // Vanilla `getSpikeOffset = 1 + cos(age·1.5+i)·0.01 - withdrawal`, where
    // `withdrawal = (1 - spikesAnimation) · 0.55`. At `spikesAnimation = 1` the withdrawal is `0`
    // (fully extended = bind); a smaller value pulls the whole spike crown in toward the body by
    // exactly `withdrawal · SPIKE_{X,Y,Z}` along each axis, leaving the rotation untouched.
    for i in 0..GUARDIAN_SPIKE_X.len() {
        let extended = guardian_spike_pose(i, 0.0, 0.0);
        let withdrawn = guardian_spike_pose(i, 0.0, 0.55);
        assert!(
            (withdrawn.offset[0] - (extended.offset[0] - GUARDIAN_SPIKE_X[i] * 0.55)).abs()
                < 1.0e-4
        );
        assert!(
            (withdrawn.offset[1] - (extended.offset[1] - GUARDIAN_SPIKE_Y[i] * 0.55)).abs()
                < 1.0e-4
        );
        assert!(
            (withdrawn.offset[2] - (extended.offset[2] - GUARDIAN_SPIKE_Z[i] * 0.55)).abs()
                < 1.0e-4
        );
        assert_eq!(withdrawn.rotation, extended.rotation);
    }

    // Mesh: `spikesAnimation = 1.0` (withdrawal `0`) is the fully-extended bind pose (the default); a
    // swimming guardian (`spikesAnimation = 0.0`) retracts the spikes, changing the mesh.
    let base = EntityModelInstance::guardian(997, [0.0, 64.0, 0.0], 0.0, false);
    let extended = entity_model_mesh(&[base.with_guardian_spikes_animation(1.0)]);
    assert_eq!(
        extended.vertices,
        entity_model_mesh(&[base]).vertices,
        "spikesAnimation 1.0 is the fully-extended bind pose (the default)"
    );
    let retracted = entity_model_mesh(&[base.with_guardian_spikes_animation(0.0)]);
    assert_eq!(extended.vertices.len(), retracted.vertices.len());
    assert_ne!(
        extended.vertices, retracted.vertices,
        "a swimming guardian retracts its spikes"
    );
}

#[test]
fn guardian_whole_body_turns_with_the_look() {
    // Vanilla `GuardianModel.setupAnim` sets `head.yRot/xRot` from the plain look, and every part
    // (body shell, spikes, eye, tail) hangs off `head`, so the whole guardian rotates with the look.
    // A non-zero look re-poses every vertex (the cube count is unchanged — it is a rigid rotation).
    let base = EntityModelInstance::guardian(993, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(35.0, -20.0)]);
    assert_eq!(rest.vertices.len(), looking.vertices.len());
    assert_ne!(
        rest.vertices, looking.vertices,
        "the whole guardian turns with the look"
    );

    // The elder guardian (scaled mesh) turns the same way.
    let elder = EntityModelInstance::guardian(994, [0.0, 64.0, 0.0], 0.0, true);
    assert_ne!(
        entity_model_mesh(&[elder]).vertices,
        entity_model_mesh(&[elder.with_head_look(35.0, -20.0)]).vertices,
    );
}

#[test]
fn guardian_tail_sways_with_the_swim_accumulator() {
    // Vanilla `GuardianModel.setupAnim`: `float swim = state.tailAnimation; tailParts[i].yRot =
    // sin(swim) * π * {0.05, 0.1, 0.15}`. A non-zero tail animation (off the sin zero) re-poses the
    // three-segment tail off the bind pose without touching the cube count, and the elder shares it.
    for elder in [false, true] {
        let base = EntityModelInstance::guardian(996, [0.0, 64.0, 0.0], 0.0, elder);
        let rest = entity_model_mesh(&[base]); // tailAnimation = 0 → sin(0) = 0 → bind tail

        // `tailAnimation = 0` (and any multiple of π) keeps the tail at the bind pose.
        let zero = entity_model_mesh(&[base.with_guardian_tail_animation(0.0)]);
        assert_eq!(
            rest.vertices, zero.vertices,
            "tailAnimation = 0 holds the bind tail pose (elder = {elder})"
        );

        // A live sway phase (sin ≠ 0) bends the tail.
        let swaying = entity_model_mesh(&[base.with_guardian_tail_animation(1.0)]);
        assert_eq!(rest.vertices.len(), swaying.vertices.len());
        assert_ne!(
            rest.vertices, swaying.vertices,
            "a non-zero tail animation sways the tail off the bind pose (elder = {elder})"
        );

        // A different phase re-poses the tail again (the sway tracks the phase).
        let swaying_other = entity_model_mesh(&[base.with_guardian_tail_animation(2.5)]);
        assert_ne!(swaying.vertices, swaying_other.vertices);
    }
}

#[test]
fn guardian_tail_sway_is_independent_of_the_spike_pulse() {
    // The tail sway (`guardian_tail_animation`) and the spike pulse (`age_in_ticks`) come from
    // separate render-state channels and pose disjoint parts, so each drives a distinct mesh change.
    let base = EntityModelInstance::guardian(997, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);

    let only_tail = entity_model_mesh(&[base.with_guardian_tail_animation(1.0)]);
    let only_spikes = entity_model_mesh(&[base.with_age_in_ticks(10.0)]);
    let both = entity_model_mesh(&[base
        .with_guardian_tail_animation(1.0)
        .with_age_in_ticks(10.0)]);

    assert_ne!(
        rest.vertices, only_tail.vertices,
        "the tail sway moves the tail"
    );
    assert_ne!(
        rest.vertices, only_spikes.vertices,
        "the spike pulse moves the spikes"
    );
    assert_ne!(only_tail.vertices, both.vertices);
    assert_ne!(only_spikes.vertices, both.vertices);
}

#[test]
fn guardian_textured_render_matches_vanilla_renderer() {
    // The guardian and elder guardian share one mesh, differing only by texture (and the elder's 2.35
    // root scale).
    assert_eq!(
        guardian_textured_layer_passes(false)[0].texture,
        GUARDIAN_TEXTURE_REF
    );
    assert_eq!(
        guardian_textured_layer_passes(true)[0].texture,
        GUARDIAN_ELDER_TEXTURE_REF
    );
    assert_eq!(
        EntityModelKind::Guardian { elder: false }.vanilla_texture_ref(),
        Some(GUARDIAN_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Guardian { elder: true }.vanilla_texture_ref(),
        Some(GUARDIAN_ELDER_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&GUARDIAN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GUARDIAN_ELDER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GUARDIAN_BEAM_TEXTURE_REF));
    assert_eq!(
        guardian_entity_texture_refs(),
        &[
            GUARDIAN_TEXTURE_REF,
            GUARDIAN_ELDER_TEXTURE_REF,
            GUARDIAN_BEAM_TEXTURE_REF
        ]
    );

    let images: Vec<EntityModelTextureImage> = guardian_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for elder in [false, true] {
        let instance = EntityModelInstance::guardian(990, [0.0, 64.0, 0.0], 0.0, elder)
            .with_light_coords(if elder {
                (6_u32 << 4) | (10_u32 << 20)
            } else {
                (5_u32 << 4) | (11_u32 << 20)
            })
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());
        assert_eq!(meshes.submissions.len(), 1);
        let submit = meshes.submissions[0];
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(
            submit.texture,
            if elder {
                GUARDIAN_ELDER_TEXTURE_REF
            } else {
                GUARDIAN_TEXTURE_REF
            }
        );
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            submit.transform,
            mesh_transformer_scaled_model_root_transform(
                instance,
                if elder { GUARDIAN_ELDER_SCALE } else { 1.0 }
            )
        );
        assert_eq!(submit.light, instance.render_state.shader_light());
        assert_eq!(submit.overlay, instance.render_state.overlay_coords());
        assert_ne!(submit.overlay, [0.0, 10.0]);
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "elder={elder} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
    }
}

// Build an atlas covering the guardian base + beam textures, enough to render a beaming guardian.
fn guardian_beam_atlas() -> EntityModelTextureAtlasLayout {
    let images: Vec<EntityModelTextureImage> = guardian_entity_texture_refs()
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
        meshes.scroll.vertices.is_empty(),
        "a guardian with no active target emits no beam geometry"
    );
    assert!(
        meshes
            .submissions
            .iter()
            .all(|submit| submit.texture != GUARDIAN_BEAM_TEXTURE_REF),
        "a guardian with no active target emits no beam submission"
    );
}

#[test]
fn guardian_beam_submission_survives_missing_beam_texture_atlas_entry() {
    // The custom beam geometry enters through the same submission-first path as scrolled model
    // overlays: the vanilla `RenderTypes.entityCutout(guardian_beam.png)` submit is recorded before
    // atlas lookup, and missing texture data suppresses only the folded mesh output.
    let atlas = guardian_base_only_atlas();
    let instance = EntityModelInstance::guardian(501, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((2_u32 << 4) | (7_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_guardian_beam(Some(GuardianBeamRenderState {
            eye_to_target: [0.0, 8.0, 0.0],
            eye_height: 0.5,
            attack_time: 0.0,
            attack_scale: 1.0,
        }));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let beam_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == GUARDIAN_BEAM_TEXTURE_REF)
        .expect("beam submission is recorded even when the atlas lacks beam texture");
    assert_eq!(
        beam_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(beam_submit.tint, [1.0, 223.0 / 255.0, 64.0 / 255.0, 1.0]);
    assert_eq!(beam_submit.light, [1.0, 1.0]);
    assert_ne!(beam_submit.light, instance.render_state.shader_light());
    assert_eq!(beam_submit.overlay, [0.0, 10.0]);
    assert_ne!(beam_submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((beam_submit.order, beam_submit.submit_sequence), (0, 1));
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll.indices.is_empty());
}

#[test]
fn guardian_beam_emits_twisted_prism_with_scale_color() {
    // Vanilla `GuardianRenderer.renderBeam`: the beam is a 12-vertex twisted prism (two crossed
    // longitudinal strips + a top cap), folded into the scroll (tiled) pass — three quads → 18 indices.
    let atlas = guardian_beam_atlas();
    let instance = EntityModelInstance::guardian(501, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((2_u32 << 4) | (7_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_guardian_beam(Some(GuardianBeamRenderState {
            eye_to_target: [0.0, 8.0, 0.0],
            eye_height: 0.5,
            attack_time: 0.0,
            attack_scale: 1.0,
        }));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.scroll.vertices.len(), 12, "12 beam vertices");
    assert_eq!(meshes.scroll.indices.len(), 18, "3 quads → 18 indices");

    // Vanilla color ramp at `attackScale = 1` (`colorScale = 1`): red `64+191=255`, green `32+191=223`,
    // blue `128-64=64`, alpha opaque.
    let expected = [1.0, 223.0 / 255.0, 64.0 / 255.0, 1.0];
    assert!(meshes
        .scroll
        .vertices
        .iter()
        .all(|vertex| vertex.tint == expected));
    assert_eq!(meshes.submissions.len(), 2);
    let beam_submissions: Vec<_> = meshes
        .submissions
        .iter()
        .filter(|submit| submit.texture == GUARDIAN_BEAM_TEXTURE_REF)
        .collect();
    assert_eq!(beam_submissions.len(), 1);
    let beam_submit = beam_submissions[0];
    assert_eq!(
        beam_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(beam_submit.dynamic_player_skin, None);
    assert_eq!(beam_submit.tint, expected);
    assert_eq!(beam_submit.light, [1.0, 1.0]);
    assert_ne!(beam_submit.light, instance.render_state.shader_light());
    assert_eq!(beam_submit.overlay, [0.0, 10.0]);
    assert_ne!(beam_submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((beam_submit.order, beam_submit.submit_sequence), (0, 1));
    assert_close3(
        beam_submit
            .transform
            .transform_point3(Vec3::ZERO)
            .to_array(),
        [0.0, 64.5, 0.0],
    );
    assert_close3(
        beam_submit.transform.transform_vector3(Vec3::Y).to_array(),
        [0.0, 1.0, 0.0],
    );
    // Every beam vertex carries the beam texture's atlas sub-rect for the shader's vertical tiling.
    let rect_size = meshes.scroll.vertices[0].uv_rect_size;
    assert!(rect_size[0] > 0.0 && rect_size[1] > 0.0);
    // The longitudinal strips span the beam length in V (vanilla `maxV = minV + length * 2.5`,
    // `length = |beamVector| + 1`): the V extent is `(8 + 1) * 2.5 = 22.5` tiles.
    let v_values: Vec<f32> = meshes
        .scroll
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
                EntityModelInstance::guardian(502, [0.0, 64.0, 0.0], y_rot, false)
                    .with_guardian_beam(Some(GuardianBeamRenderState {
                        eye_to_target: delta,
                        eye_height: 0.5,
                        attack_time: 0.0,
                        attack_scale: 0.5,
                    })),
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
        .scroll
        .vertices
        .iter()
        .map(|v| v.position[0])
        .collect();
    let along_x_z: Vec<f32> = along_x
        .scroll
        .vertices
        .iter()
        .map(|v| v.position[2])
        .collect();
    assert!(extent(along_x_x) > 9.0, "beam along +X spans X");
    assert!(extent(along_x_z) < 1.0, "beam along +X is thin in Z");
    let along_z = beam_along([0.0, 0.0, 10.0], 2.7);
    let along_z_z: Vec<f32> = along_z
        .scroll
        .vertices
        .iter()
        .map(|v| v.position[2])
        .collect();
    let along_z_x: Vec<f32> = along_z
        .scroll
        .vertices
        .iter()
        .map(|v| v.position[0])
        .collect();
    assert!(extent(along_z_z) > 9.0, "beam along +Z spans Z");
    assert!(extent(along_z_x) < 1.0, "beam along +Z is thin in X");
}
