use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};

#[test]
fn goat_model_parts_match_vanilla_26_1_body_layers() {
    // The unified cubes carry both render paths' geometry; the beard is a flat (size-0 width) quad.
    assert_eq!(
        ADULT_GOAT_HEAD[2],
        ModelCube::new(
            [-0.5, -3.0, -14.0],
            [0.0, 7.0, 5.0],
            GOAT_BEARD,
            [0.0, 7.0, 5.0],
            [23.0, 52.0],
            false,
        )
    );
    assert_eq!(ADULT_GOAT_NOSE_POSE.offset, [0.0, -8.0, -8.0]);
    assert_eq!(ADULT_GOAT_NOSE_POSE.rotation, [0.9599, 0.0, 0.0]);
    assert_eq!(ADULT_GOAT_LEFT_HORN[0].color, GOAT_HORN);
    assert_eq!(ADULT_GOAT_RIGHT_HORN[0].color, GOAT_HORN);
    assert_eq!(ADULT_GOAT_BODY[0].size, [9.0, 11.0, 16.0]);

    assert_eq!(BABY_GOAT_HORN_POSE.offset, [-1.5, -1.5, -1.0]);
    assert_eq!(BABY_GOAT_HORN_POSE.rotation, [-0.3926991, 0.0, 0.0]);
    assert_eq!(BABY_GOAT_RIGHT_EAR_POSE.rotation, [0.0, -0.5236, 0.0]);
    assert_eq!(BABY_GOAT_LEFT_EAR_POSE.rotation, [0.0, 0.5236, 0.0]);
    assert_eq!(BABY_GOAT_HEAD_MAIN_POSE.offset, [0.0, -1.3126, -1.1548]);
    assert_eq!(BABY_GOAT_RIGHT_HORN[0].color, GOAT_HORN);
    assert_eq!(BABY_GOAT_LEFT_HORN[0].color, GOAT_HORN);
}

#[test]
fn goat_meshes_use_vanilla_body_layers_and_horn_visibility() {
    let adult = entity_model_mesh(&[EntityModelInstance::goat(
        200,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
    )]);
    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let adult_left_horn_only = entity_model_mesh(&[EntityModelInstance::goat(
        201,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
    )]);
    assert_eq!(adult_left_horn_only.opaque_faces, 66);
    assert_eq!(adult_left_horn_only.vertices.len(), 264);
    assert_eq!(adult_left_horn_only.indices.len(), 396);

    let adult_no_horns = entity_model_mesh(&[EntityModelInstance::goat(
        202,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        false,
    )]);
    assert_eq!(adult_no_horns.opaque_faces, 60);
    assert!(!adult_no_horns
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::goat(
        203,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 72);
    assert_eq!(baby.vertices.len(), 288);
    assert_eq!(baby.indices.len(), 432);

    let baby_no_horns = entity_model_mesh(&[EntityModelInstance::goat(
        204,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        false,
    )]);
    assert_eq!(baby_no_horns.opaque_faces, 60);
    assert!(!baby_no_horns
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn goat_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            "goat",
            EntityModelTextureRef {
                path: "textures/entity/goat/goat.png",
                size: [64, 64],
            },
        ),
        (
            true,
            "goat_baby",
            EntityModelTextureRef {
                path: "textures/entity/goat/goat_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (baby, model_key, texture) in cases {
        let kind = EntityModelKind::Goat {
            baby,
            left_horn: false,
            right_horn: true,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }

    assert_eq!(
        goat_entity_texture_refs(),
        &[
            EntityModelTextureRef {
                path: "textures/entity/goat/goat.png",
                size: [64, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/goat/goat_baby.png",
                size: [64, 64],
            },
        ]
    );
    assert!(entity_model_texture_refs().contains(&GOAT_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GOAT_BABY_TEXTURE_REF));
}

#[test]
fn goat_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = goat_textured_layer_passes(false);
    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::GoatBase);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].model_layer, MODEL_LAYER_GOAT);
    assert_eq!(adult[0].texture, GOAT_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].order, adult[0].submit_sequence), (0, 0));

    let baby = goat_textured_layer_passes(true);
    assert_eq!(baby.len(), 1);
    assert_eq!(baby[0].kind, EntityModelLayerKind::GoatBase);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby[0].model_layer, MODEL_LAYER_GOAT_BABY);
    assert_eq!(baby[0].texture, GOAT_BABY_TEXTURE_REF);
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((baby[0].order, baby[0].submit_sequence), (0, 0));
}

#[test]
fn goat_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    // The textured UV sources now live on the unified cubes (`uv_size`/`tex`/`mirror`).
    assert_eq!(MODEL_LAYER_GOAT, "minecraft:goat#main");
    assert_eq!(MODEL_LAYER_GOAT_BABY, "minecraft:goat_baby#main");
    assert_eq!(ADULT_GOAT_HEAD[1].tex, [2.0, 61.0]);
    assert!(ADULT_GOAT_HEAD[1].mirror);
    assert_eq!(ADULT_GOAT_HEAD[2].tex, [23.0, 52.0]);
    assert!(!ADULT_GOAT_HEAD[2].mirror);
    assert_eq!(ADULT_GOAT_LEFT_HORN[0].tex, [12.0, 55.0]);
    assert_eq!(ADULT_GOAT_RIGHT_HORN[0].tex, [12.0, 55.0]);
    assert_eq!(ADULT_GOAT_NOSE[0].tex, [34.0, 46.0]);
    assert_eq!(ADULT_GOAT_LEFT_HIND_LEG[0].tex, [36.0, 29.0]);
    assert_eq!(ADULT_GOAT_RIGHT_HIND_LEG[0].tex, [49.0, 29.0]);
    assert_eq!(ADULT_GOAT_LEFT_FRONT_LEG[0].tex, [49.0, 2.0]);
    assert_eq!(ADULT_GOAT_RIGHT_FRONT_LEG[0].tex, [35.0, 2.0]);

    assert_eq!(BABY_GOAT_LEFT_HIND_LEG[0].tex, [29.0, 12.0]);
    assert_eq!(BABY_GOAT_RIGHT_HIND_LEG[0].tex, [21.0, 12.0]);
    assert_eq!(BABY_GOAT_RIGHT_FRONT_LEG[0].tex, [21.0, 5.0]);
    assert_eq!(BABY_GOAT_LEFT_FRONT_LEG[0].tex, [29.0, 5.0]);
    assert_eq!(BABY_GOAT_RIGHT_HORN[0].tex, [24.0, 0.0]);
    assert!(BABY_GOAT_RIGHT_HORN[0].mirror);
    assert!(BABY_GOAT_LEFT_HORN[0].mirror);
    assert_eq!(BABY_GOAT_RIGHT_EAR[0].tex, [0.0, 12.0]);
    assert!(BABY_GOAT_RIGHT_EAR[0].mirror);
    assert!(!BABY_GOAT_LEFT_EAR[0].mirror);
    assert_eq!(BABY_GOAT_HEAD_MAIN[0].tex, [0.0, 0.0]);
}

#[test]
fn entity_texture_atlas_stitches_official_goat_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/goat/goat.png",
            "textures/entity/goat/goat_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let baby_first_pixel = rgba_offset(layout.width, 64, 0, "goat baby atlas row").unwrap();
    assert_eq!(&rgba[baby_first_pixel..baby_first_pixel + 4], &[1; 4]);
}

#[test]
fn goat_textured_mesh_uses_vanilla_uvs_tints_and_horn_visibility() {
    let (atlas, _) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();
    let adult = EntityModelInstance::goat(401, [0.0, 64.0, 0.0], 0.0, false, true, true)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_goat_submissions_match_vanilla(&adult_meshes, adult);
    assert_goat_vertices_match_submission(&adult_meshes);
    let adult_mesh = &adult_meshes.cutout;
    assert_eq!(adult_mesh.cutout_faces, 72);
    assert_eq!(adult_mesh.vertices.len(), 288);
    assert_eq!(adult_mesh.indices.len(), 432);
    assert_close2(adult_mesh.vertices[0].uv, [6.0 / 64.0, 61.0 / 128.0]);
    assert_eq!(adult_mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    let (adult_textured_min, adult_textured_max) = textured_mesh_extents(adult_mesh);
    let (adult_colored_min, adult_colored_max) = mesh_extents(&entity_model_mesh(&[adult]));
    assert_close3(adult_textured_min, adult_colored_min);
    assert_close3(adult_textured_max, adult_colored_max);

    let adult_left_horn_only_instance =
        EntityModelInstance::goat(402, [0.0, 64.0, 0.0], 0.0, false, true, false)
            .with_light_coords((5_u32 << 4) | (11_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);
    let adult_left_horn_only_meshes =
        entity_model_textured_meshes(&[adult_left_horn_only_instance], &atlas);
    assert_goat_submissions_match_vanilla(
        &adult_left_horn_only_meshes,
        adult_left_horn_only_instance,
    );
    assert_goat_vertices_match_submission(&adult_left_horn_only_meshes);
    let adult_left_horn_only = &adult_left_horn_only_meshes.cutout;
    assert_eq!(adult_left_horn_only.cutout_faces, 66);
    assert_eq!(adult_left_horn_only.vertices.len(), 264);

    let adult_no_horns_instance =
        EntityModelInstance::goat(403, [0.0, 64.0, 0.0], 0.0, false, false, false)
            .with_light_coords((5_u32 << 4) | (11_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);
    let adult_no_horns_meshes = entity_model_textured_meshes(&[adult_no_horns_instance], &atlas);
    assert_goat_submissions_match_vanilla(&adult_no_horns_meshes, adult_no_horns_instance);
    assert_goat_vertices_match_submission(&adult_no_horns_meshes);
    let adult_no_horns = &adult_no_horns_meshes.cutout;
    assert_eq!(adult_no_horns.cutout_faces, 60);
    assert_eq!(adult_no_horns.vertices.len(), 240);

    let baby = EntityModelInstance::goat(404, [0.0, 64.0, 0.0], 0.0, true, true, true)
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_goat_submissions_match_vanilla(&baby_meshes, baby);
    assert_goat_vertices_match_submission(&baby_meshes);
    let baby_mesh = &baby_meshes.cutout;
    assert_eq!(baby_mesh.cutout_faces, 72);
    assert_close2(baby_mesh.vertices[0].uv, [33.0 / 64.0, 76.0 / 128.0]);
    let (baby_textured_min, baby_textured_max) = textured_mesh_extents(baby_mesh);
    let (baby_colored_min, baby_colored_max) = mesh_extents(&entity_model_mesh(&[baby]));
    assert_close3(baby_textured_min, baby_colored_min);
    assert_close3(baby_textured_max, baby_colored_max);

    let baby_no_horns_instance =
        EntityModelInstance::goat(405, [0.0, 64.0, 0.0], 0.0, true, false, false)
            .with_light_coords((6_u32 << 4) | (10_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);
    let baby_no_horns_meshes = entity_model_textured_meshes(&[baby_no_horns_instance], &atlas);
    assert_goat_submissions_match_vanilla(&baby_no_horns_meshes, baby_no_horns_instance);
    assert_goat_vertices_match_submission(&baby_no_horns_meshes);
    let baby_no_horns = &baby_no_horns_meshes.cutout;
    assert_eq!(baby_no_horns.cutout_faces, 60);
    assert!(baby_no_horns
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn goat_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();
    for base in [
        EntityModelInstance::goat(440, [0.0, 64.0, 0.0], 0.0, false, true, true),
        EntityModelInstance::goat(441, [0.0, 64.0, 0.0], 0.0, true, true, true),
    ] {
        let yawed_instance = base.with_head_look(45.0, 0.0);
        let pitched_instance = base.with_head_look(0.0, -20.0);
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_goat_submissions_match_vanilla(&resting, base);
        assert_goat_submissions_match_vanilla(&yawed, yawed_instance);
        assert_goat_submissions_match_vanilla(&pitched, pitched_instance);
        assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
        assert_ne!(
            resting.cutout.vertices, yawed.cutout.vertices,
            "{:?}",
            base.kind
        );
        assert_ne!(
            yawed.cutout.vertices, pitched.cutout.vertices,
            "{:?}",
            base.kind
        );
    }
}

#[test]
fn goat_swings_its_legs_when_walking() {
    // Vanilla `GoatModel extends QuadrupedModel`: `setupAnim` runs `super.setupAnim`
    // (the diagonal `QuadrupedModel` leg swing) before the horn visibility and the
    // ramming head tilt, so the four legs swing. A standing goat is inert; a walking
    // adult lifts its feet and splays its legs along Z; the baby's short legs swing
    // too but the motion stays inside its bounding box, so only the adult asserts the
    // extent change. The ramming head tilt is deferred. Colored path.
    for (name, base, adult_size) in [
        (
            "goat_adult",
            EntityModelInstance::goat(450, [0.0, 64.0, 0.0], 0.0, false, true, true),
            true,
        ),
        (
            "goat_baby",
            EntityModelInstance::goat(451, [0.0, 64.0, 0.0], 0.0, true, true, true),
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        if adult_size {
            let (rest_min, rest_max) = mesh_extents(&rest);
            let (walk_min, walk_max) = mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking goat's feet should lift off the ground"
            );
            assert!(
                (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
                "{name}: a walking goat's legs should splay along Z"
            );
        }
    }
}

#[test]
fn goat_textured_mesh_swings_legs_when_walking() {
    // The real goat render path (texture-backed) swings the same `QuadrupedModel`
    // legs on the shared visibility-filtered part array. A standing goat is
    // byte-identical however far the swing position has advanced; a walking adult
    // lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();
    for (name, base, adult_size) in [
        (
            "goat_adult",
            EntityModelInstance::goat(452, [0.0, 64.0, 0.0], 0.0, false, true, true),
            true,
        ),
        (
            "goat_baby",
            EntityModelInstance::goat(453, [0.0, 64.0, 0.0], 0.0, true, true, true),
            false,
        ),
    ] {
        let still_instance = base.with_walk_animation(2.5, 0.0);
        let walking_instance = base.with_walk_animation(0.0, 1.0);
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_goat_submissions_match_vanilla(&resting, base);
        assert_goat_submissions_match_vanilla(&still, still_instance);
        assert_goat_submissions_match_vanilla(&walking, walking_instance);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{name}: a standing goat is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{name}: a walking goat differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
            let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking goat's feet should lift off the ground"
            );
        }
    }
}

fn assert_goat_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_goat_folded_meshes_are_cutout_only(meshes);
    let baby = match instance.kind {
        EntityModelKind::Goat { baby, .. } => baby,
        _ => panic!("expected goat instance"),
    };
    let passes = goat_textured_layer_passes(baby);
    assert_eq!(meshes.submissions.len(), passes.len());
    for (submit, pass) in meshes.submissions.iter().copied().zip(passes) {
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type, pass.render_type);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, pass.texture);
        assert_eq!(submit.tint, pass.tint);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!(submit.light, instance.render_state.shader_light());
        assert_eq!(submit.overlay, instance.render_state.overlay_coords());
        assert_eq!(
            (submit.order, submit.submit_sequence),
            (pass.order, pass.submit_sequence)
        );
    }
}

fn assert_goat_vertices_match_submission(meshes: &EntityModelTexturedMeshes) {
    let submit = meshes.submissions[0];
    assert_ne!(submit.overlay, [0.0, 10.0]);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
}

fn assert_goat_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn goat_texture_images() -> Vec<EntityModelTextureImage> {
    goat_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn ramming_goat_tilts_its_head_down() {
    // Vanilla `GoatModel.setupAnim`: `if rammingXHeadRot != 0 { head.xRot = rammingXHeadRot }`, set after
    // the head look — so a ramming goat's head pitch is the ram tilt (overwriting the look pitch), and a
    // resting goat keeps its look pitch. The projected `goat_ramming_x_head_rot` already bakes in the
    // adult/baby max head pitch, so the renderer just SETs it.
    let tilt = 0.4_f32;
    let ramming = EntityModelInstance::goat(410, [0.0, 64.0, 0.0], 0.0, false, true, true)
        .with_head_look(0.0, -20.0)
        .with_goat_ramming_x_head_rot(tilt);
    let mut model = GoatModel::new(false, true, true);
    model.prepare(&ramming);
    let head_pitch = model.root_mut().child_mut("head").pose.rotation[0];
    assert!(
        (head_pitch - tilt).abs() < 1.0e-6,
        "the ram tilt overwrites the look pitch: {head_pitch}"
    );

    // A resting goat (no ram) keeps its head-look pitch.
    let mut resting = GoatModel::new(false, true, true);
    resting.prepare(
        &EntityModelInstance::goat(410, [0.0, 64.0, 0.0], 0.0, false, true, true)
            .with_head_look(0.0, -20.0),
    );
    let resting_pitch = resting.root_mut().child_mut("head").pose.rotation[0];
    assert!(
        (resting_pitch - (-20.0_f32).to_radians()).abs() < 1.0e-6,
        "a resting goat keeps its look pitch: {resting_pitch}"
    );

    // The ram tilt is visible in the rendered mesh (the head re-poses).
    let ramming_mesh = entity_model_mesh(&[ramming]);
    let resting_mesh = entity_model_mesh(&[EntityModelInstance::goat(
        410,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
    )
    .with_head_look(0.0, -20.0)]);
    assert_ne!(
        ramming_mesh.vertices, resting_mesh.vertices,
        "the ramming head tilt changes the mesh"
    );
}
