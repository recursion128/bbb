use super::*;
use crate::entity_models::model_layers::goat_model;

#[test]
fn goat_texture_refs_match_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&GOAT_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GOAT_BABY_TEXTURE_REF));
}

#[test]
fn goat_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = goat_textured_layer_passes(false);
    assert_eq!(adult.len(), 1);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].texture, GOAT_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult[0].order, 0);

    let baby = goat_textured_layer_passes(true);
    assert_eq!(baby.len(), 1);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby[0].texture, GOAT_BABY_TEXTURE_REF);
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(baby[0].order, 0);
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
    let adult =
        EntityModelInstance::goat(401, [0.0, 64.0, 0.0], 0.0, false, true, true).with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_goat_textured_output(&adult_meshes);
    assert_goat_vertex_state(&adult_meshes, adult);
    let adult_mesh = &adult_meshes.cutout;
    assert_eq!(adult_mesh.cutout_faces, 72);
    assert_eq!(adult_mesh.vertices.len(), 288);
    assert_eq!(adult_mesh.indices.len(), 432);
    assert_eq!(adult_mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);

    let adult_left_horn_only_instance =
        EntityModelInstance::goat(402, [0.0, 64.0, 0.0], 0.0, false, true, false).with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let adult_left_horn_only_meshes =
        entity_model_textured_meshes(&[adult_left_horn_only_instance], &atlas);
    assert_goat_textured_output(&adult_left_horn_only_meshes);
    assert_goat_vertex_state(&adult_left_horn_only_meshes, adult_left_horn_only_instance);
    let adult_left_horn_only = &adult_left_horn_only_meshes.cutout;
    assert_eq!(adult_left_horn_only.cutout_faces, 66);
    assert_eq!(adult_left_horn_only.vertices.len(), 264);

    let adult_no_horns_instance =
        EntityModelInstance::goat(403, [0.0, 64.0, 0.0], 0.0, false, false, false).with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let adult_no_horns_meshes = entity_model_textured_meshes(&[adult_no_horns_instance], &atlas);
    assert_goat_textured_output(&adult_no_horns_meshes);
    assert_goat_vertex_state(&adult_no_horns_meshes, adult_no_horns_instance);
    let adult_no_horns = &adult_no_horns_meshes.cutout;
    assert_eq!(adult_no_horns.cutout_faces, 60);
    assert_eq!(adult_no_horns.vertices.len(), 240);

    let baby = EntityModelInstance::goat(404, [0.0, 64.0, 0.0], 0.0, true, true, true).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_goat_textured_output(&baby_meshes);
    assert_goat_vertex_state(&baby_meshes, baby);
    let baby_mesh = &baby_meshes.cutout;
    assert_eq!(baby_mesh.cutout_faces, 72);

    let baby_no_horns_instance =
        EntityModelInstance::goat(405, [0.0, 64.0, 0.0], 0.0, true, false, false).with(|s| {
            s.light_coords = (6_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let baby_no_horns_meshes = entity_model_textured_meshes(&[baby_no_horns_instance], &atlas);
    assert_goat_textured_output(&baby_no_horns_meshes);
    assert_goat_vertex_state(&baby_no_horns_meshes, baby_no_horns_instance);
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
        let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
        let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_goat_textured_output(&resting);
        assert_goat_textured_output(&yawed);
        assert_goat_textured_output(&pitched);
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
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_goat_textured_output(&resting);
        assert_goat_textured_output(&still);
        assert_goat_textured_output(&walking);

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

fn assert_goat_textured_output(meshes: &EntityModelTexturedMeshes) {
    assert_goat_folded_meshes_are_cutout_only(meshes);
}

fn assert_goat_vertex_state(meshes: &EntityModelTexturedMeshes, instance: EntityModelInstance) {
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
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
    [GOAT_TEXTURE_REF, GOAT_BABY_TEXTURE_REF]
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
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0))
        .with(|s| s.kind.goat_mut().ramming_x_head_rot = tilt);
    let mut model = goat_model(false, true, true, &ramming);
    let head_pitch = model.child_mut("head").pose.rotation[0];
    assert!(
        (head_pitch - tilt).abs() < 1.0e-6,
        "the ram tilt overwrites the look pitch: {head_pitch}"
    );

    // A resting goat (no ram) keeps its head-look pitch.
    let mut resting = goat_model(
        false,
        true,
        true,
        &EntityModelInstance::goat(410, [0.0, 64.0, 0.0], 0.0, false, true, true)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0)),
    );
    let resting_pitch = resting.child_mut("head").pose.rotation[0];
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
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0))]);
    assert_ne!(
        ramming_mesh.vertices, resting_mesh.vertices,
        "the ramming head tilt changes the mesh"
    );
}
