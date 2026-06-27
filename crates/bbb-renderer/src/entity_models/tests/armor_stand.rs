use super::*;

#[test]
fn armor_stand_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(ARMOR_STAND_PARTS.len(), 10);
    assert_part(
        &ARMOR_STAND_PARTS[0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_HEAD.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_BODY.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEFT_ARM.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[4],
        [-1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[5],
        [1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[6],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_RIGHT_BODY_STICK.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[7],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEFT_BODY_STICK.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[8],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_SHOULDER_STICK.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[9],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_BASE_PLATE.as_slice(),
    );

    assert_eq!(SMALL_ARMOR_STAND_PARTS.len(), 10);
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[0],
        [0.0, 12.75, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_HEAD.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[1],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_BODY.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[2],
        [-2.5, 13.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[3],
        [2.5, 13.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEFT_ARM.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[4],
        [-0.95, 18.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[5],
        [0.95, 18.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[6],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_RIGHT_BODY_STICK.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[7],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEFT_BODY_STICK.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[8],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_SHOULDER_STICK.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[9],
        [0.0, 18.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_BASE_PLATE.as_slice(),
    );
}

#[test]
fn armor_stand_mesh_uses_vanilla_visibility_and_pose_state() {
    let default = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )]);
    assert_eq!(default.opaque_faces, 48);
    assert_eq!(default.vertices.len(), 192);
    assert_eq!(default.indices.len(), 288);

    let arms_without_base = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )]);
    assert_eq!(arms_without_base.opaque_faces, 54);
    assert_eq!(arms_without_base.vertices.len(), 216);
    assert_eq!(arms_without_base.indices.len(), 324);

    let small = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )]);
    assert_eq!(small.opaque_faces, 48);
    assert_eq!(small.vertices.len(), 192);
    assert_eq!(small.indices.len(), 288);

    let mut pose = DEFAULT_ARMOR_STAND_MODEL_POSE;
    pose.head = [0.0, 45.0, 0.0];
    pose.body = [0.0, 0.0, 12.0];
    let posed = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        pose,
    )]);
    assert_eq!(posed.opaque_faces, default.opaque_faces);
    assert_ne!(posed.vertices, default.vertices);
}

#[test]
fn armor_stand_texture_refs_match_vanilla_renderer() {
    let adult = EntityModelKind::ArmorStand {
        small: false,
        marker: false,
        show_arms: false,
        show_base_plate: true,
        pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
    };
    let small = EntityModelKind::ArmorStand {
        small: true,
        marker: false,
        show_arms: false,
        show_base_plate: true,
        pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
    };

    assert_eq!(adult.model_key(), "armor_stand");
    assert_eq!(small.model_key(), "armor_stand_small");
    assert_eq!(
        adult.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/armorstand/armorstand.png",
            size: [64, 64],
        })
    );
    assert_eq!(small.vanilla_texture_ref(), adult.vanilla_texture_ref());
    assert!(entity_model_texture_refs().contains(&ARMOR_STAND_TEXTURE_REF));
    assert_eq!(
        armor_stand_entity_texture_refs(),
        &[ARMOR_STAND_TEXTURE_REF]
    );
}

#[test]
fn armor_stand_textured_part_uvs_match_vanilla_model_layer() {
    // Vanilla ArmorStandModel.createBodyLayer texOffs + box per part (texture 64x64). The small
    // layer is the same mesh scaled by BABY_TRANSFORMER, so its UVs equal the full model's.
    assert_eq!(ARMOR_STAND_PART_UVS.len(), 10);
    let expected: [([f32; 2], [f32; 3], bool); 10] = [
        ([0.0, 0.0], [2.0, 7.0, 2.0], false),
        ([0.0, 26.0], [12.0, 3.0, 3.0], false),
        ([24.0, 0.0], [2.0, 12.0, 2.0], false),
        ([32.0, 16.0], [2.0, 12.0, 2.0], true),
        ([8.0, 0.0], [2.0, 11.0, 2.0], false),
        ([40.0, 16.0], [2.0, 11.0, 2.0], true),
        ([16.0, 0.0], [2.0, 7.0, 2.0], false),
        ([48.0, 16.0], [2.0, 7.0, 2.0], false),
        ([0.0, 48.0], [8.0, 2.0, 2.0], false),
        ([0.0, 32.0], [12.0, 1.0, 12.0], false),
    ];
    for (index, (tex, uv_size, mirror)) in expected.iter().enumerate() {
        let uv = ARMOR_STAND_PART_UVS[index];
        assert_eq!(uv.tex, *tex, "part {index} texOffs");
        assert_eq!(uv.uv_size, *uv_size, "part {index} uv_size");
        assert_eq!(uv.mirror, *mirror, "part {index} mirror");
    }
    // The full model's textured cube reuses the colored geometry verbatim.
    let head = armor_stand_textured_cube(&ARMOR_STAND_PARTS[0], ARMOR_STAND_PART_UVS[0]);
    assert_eq!(head.min, ARMOR_STAND_HEAD[0].min);
    assert_eq!(head.size, ARMOR_STAND_HEAD[0].size);
    assert_eq!(head.uv_size, [2.0, 7.0, 2.0]);
    // The small model scales the geometry but keeps the full-model UV source.
    let small_head =
        armor_stand_textured_cube(&SMALL_ARMOR_STAND_PARTS[0], ARMOR_STAND_PART_UVS[0]);
    assert_eq!(small_head.min, SMALL_ARMOR_STAND_HEAD[0].min);
    assert_eq!(small_head.size, SMALL_ARMOR_STAND_HEAD[0].size);
    assert_eq!(small_head.uv_size, [2.0, 7.0, 2.0]);
}

#[test]
fn armor_stand_textured_mesh_matches_colored_geometry_and_visibility() {
    let (atlas, _) = build_entity_model_texture_atlas(&armor_stand_texture_images()).unwrap();
    for (small, show_arms, show_base_plate) in [
        (false, false, true),
        (false, true, false),
        (true, true, true),
    ] {
        let mut pose = DEFAULT_ARMOR_STAND_MODEL_POSE;
        pose.head = [0.0, 45.0, 0.0];
        pose.body = [0.0, 0.0, 12.0];
        let instance = EntityModelInstance::armor_stand(
            5,
            [0.0, 64.0, 0.0],
            0.0,
            small,
            show_arms,
            show_base_plate,
            pose,
        );
        let instances = [instance];
        let colored = entity_model_mesh(&instances);
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert!(textured_meshes.translucent.vertices.is_empty());
        assert!(textured_meshes.eyes.vertices.is_empty());
        assert_eq!(textured_meshes.submissions.len(), 1);
        let submit = textured_meshes.submissions[0];
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, ARMOR_STAND_TEXTURE_REF);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        let textured = &textured_meshes.cutout;

        // The textured armor stand shares the colored geometry exactly: same cube count and bounds.
        assert_eq!(textured.cutout_faces, colored.opaque_faces);
        assert_eq!(textured.vertices.len(), colored.vertices.len());
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        let (cmin, cmax) = mesh_extents(&colored);
        let (tmin, tmax) = textured_mesh_extents(&textured);
        assert_close3(tmin, cmin);
        assert_close3(tmax, cmax);
    }
}

#[test]
fn armor_stand_marker_render_type_follows_vanilla_visibility_branch() {
    let (atlas, _) = build_entity_model_texture_atlas(&armor_stand_texture_images()).unwrap();
    let visible_marker = EntityModelInstance::armor_stand_with_marker(
        9,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);

    let visible = entity_model_textured_meshes(&[visible_marker], &atlas);
    assert_eq!(visible.submissions.len(), 1);
    let submit = visible.submissions[0];
    assert_eq!(submit.texture, ARMOR_STAND_TEXTURE_REF);
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        submit.transform,
        entity_model_root_transform(visible_marker)
    );
    assert_eq!(submit.light, visible_marker.render_state.shader_light());
    assert_eq!(submit.overlay, visible_marker.render_state.overlay_coords());
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert!(!visible.cutout.vertices.is_empty());
    assert!(visible
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
    assert!(visible.translucent.vertices.is_empty());

    let hidden_invisible = visible_marker.with_invisible(true);
    let hidden = entity_model_textured_meshes(&[hidden_invisible], &atlas);
    assert!(hidden.submissions.is_empty());
    assert!(hidden.cutout.vertices.is_empty());
    assert!(hidden.translucent.vertices.is_empty());

    let self_visible_invisible = hidden_invisible.with_invisible_to_player(false);
    let self_visible = entity_model_textured_meshes(&[self_visible_invisible], &atlas);
    assert_eq!(self_visible.submissions.len(), 1);
    let submit = self_visible.submissions[0];
    assert_eq!(submit.texture, ARMOR_STAND_TEXTURE_REF);
    // Vanilla `ArmorStandRenderer.getRenderType`: marker force-transparent uses
    // `RenderTypes.entityTranslucent(texture, false)`, not the generic living
    // `entityTranslucentCullItemTarget`.
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(submit.render_type.vanilla_name(), "entityTranslucent");
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 38.0 / 255.0]);
    assert_eq!(
        submit.transform,
        entity_model_root_transform(self_visible_invisible)
    );
    assert_eq!(
        submit.light,
        self_visible_invisible.render_state.shader_light()
    );
    assert_eq!(
        submit.overlay,
        self_visible_invisible.render_state.overlay_coords()
    );
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert!(self_visible.cutout.vertices.is_empty());
    assert!(!self_visible.translucent.vertices.is_empty());
    assert!(self_visible
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
}

fn armor_stand_texture_images() -> Vec<EntityModelTextureImage> {
    armor_stand_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
