use super::*;

#[test]
fn nautilus_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `NautilusModel.createBodyMesh` (atlas 128×128): one cubeless `root` pivot parenting the
    // `shell` and the `body` (the body parenting the three mouths).
    assert_eq!(NAUTILUS_ROOT_POSE.offset, [0.0, 29.0, -6.0]);

    // `shell` (offset (0, -13, 5)): the 14×10×16 dome, the 14×8×20 whorl, and a 14×8×0 fin plane.
    assert_eq!(NAUTILUS_SHELL_POSE.offset, [0.0, -13.0, 5.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES.len(), 3);
    assert_eq!(NAUTILUS_SHELL_CUBES[0].min, [-7.0, -10.0, -7.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES[0].size, [14.0, 10.0, 16.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES[1].size, [14.0, 8.0, 20.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES[2].size, [14.0, 8.0, 0.0]);

    // `body` (offset (0, -8.5, 12.3)): the 10×8×14 trunk and a 10×8×0 fin plane, parenting the mouths.
    assert_eq!(NAUTILUS_BODY_POSE.offset, [0.0, -8.5, 12.3]);
    assert_eq!(NAUTILUS_BODY_CUBES.len(), 2);
    assert_eq!(NAUTILUS_BODY_CUBES[0].min, [-5.0, -4.51, -3.0]);
    assert_eq!(NAUTILUS_BODY_CUBES[0].size, [10.0, 8.0, 14.0]);

    // The three mouths; upper/lower deflated by the vanilla `CubeDeformation(-0.001)`, inner undeformed.
    assert_eq!(NAUTILUS_UPPER_MOUTH_POSE.offset, [0.0, -2.51, 7.0]);
    assert_eq!(NAUTILUS_UPPER_MOUTH_CUBES[0].min, [-4.999, -1.999, 0.001]);
    assert_eq!(NAUTILUS_UPPER_MOUTH_CUBES[0].size, [9.998, 3.998, 3.998]);
    assert_eq!(NAUTILUS_INNER_MOUTH_POSE.offset, [0.0, -0.51, 7.5]);
    assert_eq!(NAUTILUS_INNER_MOUTH_CUBES[0].min, [-3.0, -2.0, -0.5]);
    assert_eq!(NAUTILUS_INNER_MOUTH_CUBES[0].size, [6.0, 4.0, 4.0]);
    assert_eq!(NAUTILUS_LOWER_MOUTH_POSE.offset, [0.0, 1.49, 7.0]);
    assert_eq!(NAUTILUS_LOWER_MOUTH_CUBES[0].min, [-4.999, -1.979, 0.001]);
}

#[test]
fn nautilus_saddle_geometry_matches_vanilla_26_1_layer() {
    // Vanilla `NautilusSaddleModel.createSaddleLayer()` starts from `NautilusModel.createBodyMesh`,
    // replaces the adult shell with a single `CubeDeformation(0.2F)` dome, and preserves the body and
    // mouth subtrees. `SimpleEquipmentLayer` supplies no baby model.
    assert_eq!(NAUTILUS_SADDLE_SHELL_CUBES.len(), 1);
    assert_eq!(NAUTILUS_SADDLE_SHELL_CUBES[0].min, [-7.2, -10.2, -7.2]);
    assert_eq!(NAUTILUS_SADDLE_SHELL_CUBES[0].size, [14.4, 10.4, 16.4]);
    assert_eq!(NAUTILUS_SADDLE_SHELL_CUBES[0].uv_size, [14.0, 10.0, 16.0]);
    assert_eq!(NAUTILUS_SADDLE_SHELL_CUBES[0].tex, [0.0, 0.0]);
}

#[test]
fn nautilus_body_armor_geometry_matches_vanilla_26_1_layer() {
    // Vanilla `NautilusArmorModel.createBodyLayer()` starts from `NautilusModel.createBodyMesh`,
    // replaces the adult shell with the three shell cubes, and inflates only the dome and whorl by
    // `CubeDeformation(0.01F)`. `SimpleEquipmentLayer` supplies no baby model.
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES.len(), 3);
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES[0].min, [-7.01, -10.01, -7.01]);
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES[0].size, [14.02, 10.02, 16.02]);
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES[0].uv_size, [14.0, 10.0, 16.0]);
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES[0].tex, [0.0, 0.0]);
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES[1].min, [-7.01, -0.01, -7.01]);
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES[1].size, [14.02, 8.02, 20.02]);
    assert_eq!(NAUTILUS_ARMOR_SHELL_CUBES[1].uv_size, [14.0, 8.0, 20.0]);
    assert_eq!(
        NAUTILUS_ARMOR_SHELL_CUBES[2].min,
        NAUTILUS_SHELL_CUBES[2].min
    );
    assert_eq!(
        NAUTILUS_ARMOR_SHELL_CUBES[2].size,
        NAUTILUS_SHELL_CUBES[2].size
    );
}

#[test]
fn nautilus_mesh_uses_vanilla_body_layer_geometry() {
    // 8 cubes → 48 faces / 192 vertices / 288 indices, two tones: tan shell, pale body/mouths.
    let nautilus = entity_model_mesh(&[EntityModelInstance::nautilus(
        300,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(nautilus.opaque_faces, 48);
    assert_eq!(nautilus.vertices.len(), 192);
    assert_eq!(nautilus.indices.len(), 288);
    assert!(nautilus
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_SHELL, 1.0)));
    assert!(nautilus
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_BODY, 1.0)));
}

#[test]
fn nautilus_colored_runtime_skips_the_texture_backed_nautilus() {
    // The nautilus now carries vanilla texture UVs, so it renders through the textured path. The
    // texture-skipping colored runtime path emits nothing for it (adult or baby), while the full path
    // still emits the colored fallback geometry.
    let instances = [
        EntityModelInstance::nautilus(301, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::nautilus(311, [4.0, 64.0, 0.0], 0.0, true),
    ];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn nautilus_body_look_turns_the_body_and_mouths_not_the_shell() {
    // Vanilla `NautilusModel.applyBodyRotation` steers the `body` (not the shell) by the look. The shell
    // is the first child (3 cubes → vertices `[0, 72)`); the body and its three mouths `[72, 192)` turn.
    let rest = EntityModelInstance::nautilus(302, [0.0, 64.0, 0.0], 0.0, false);
    let looked = rest.with_head_look(8.0, -8.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_eq!(
        rest_mesh.vertices[..72],
        looked_mesh.vertices[..72],
        "the shell stays put"
    );
    assert_ne!(
        rest_mesh.vertices[72..],
        looked_mesh.vertices[72..],
        "the body and its mouths turn with the look"
    );
}

#[test]
fn nautilus_body_look_is_clamped_to_ten_degrees() {
    // Vanilla `applyBodyRotation` clamps the look to ±10° before steering the body, so two looks past
    // the clamp render identically while a look inside the clamp differs.
    let clamped_50 =
        entity_model_mesh(&[
            EntityModelInstance::nautilus(303, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(50.0, 50.0),
        ]);
    let clamped_90 =
        entity_model_mesh(&[
            EntityModelInstance::nautilus(304, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(90.0, 90.0),
        ]);
    assert_eq!(
        clamped_50.vertices, clamped_90.vertices,
        "looks past ±10° clamp to the same body rotation"
    );
    let within_5 =
        entity_model_mesh(&[
            EntityModelInstance::nautilus(305, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(5.0, 5.0),
        ]);
    assert_ne!(
        within_5.vertices, clamped_50.vertices,
        "a look inside ±10° turns the body less than the clamp"
    );
}

#[test]
fn baby_nautilus_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `NautilusModel.createBabyBodyLayer` (atlas 64×64): the same cubeless `root → shell + body
    // → three mouths` structure as the adult, in smaller hatchling proportions.
    assert_eq!(BABY_NAUTILUS_ROOT_POSE.offset, [-0.5, 28.0, -0.5]);

    // `shell` (offset (3, -8, -2)): the 7×4×7 dome, the 7×4×9 whorl, and a 7×4×0 fin plane.
    assert_eq!(BABY_NAUTILUS_SHELL_POSE.offset, [3.0, -8.0, -2.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES.len(), 3);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[0].min, [-6.0, -4.0, -1.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[0].size, [7.0, 4.0, 7.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[1].size, [7.0, 4.0, 9.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[2].size, [7.0, 4.0, 0.0]);

    // `body` (offset (0.5, -5, 3)): the 5×4×7 trunk and a 5×4×0 fin plane, parenting the mouths.
    assert_eq!(BABY_NAUTILUS_BODY_POSE.offset, [0.5, -5.0, 3.0]);
    assert_eq!(BABY_NAUTILUS_BODY_CUBES.len(), 2);
    assert_eq!(BABY_NAUTILUS_BODY_CUBES[0].min, [-2.5, -3.01, -1.0]);
    assert_eq!(BABY_NAUTILUS_BODY_CUBES[0].size, [5.0, 4.0, 7.0]);

    // The three mouths; upper/lower deflated by the vanilla `CubeDeformation(-0.001)`, inner undeformed.
    assert_eq!(BABY_NAUTILUS_UPPER_MOUTH_POSE.offset, [0.0, -2.01, 3.9]);
    assert_eq!(
        BABY_NAUTILUS_UPPER_MOUTH_CUBES[0].min,
        [-2.499, -0.999, 0.001]
    );
    assert_eq!(
        BABY_NAUTILUS_UPPER_MOUTH_CUBES[0].size,
        [4.998, 1.998, 1.998]
    );
    assert_eq!(BABY_NAUTILUS_INNER_MOUTH_POSE.offset, [0.0, -1.01, 4.9]);
    assert_eq!(BABY_NAUTILUS_INNER_MOUTH_CUBES[0].min, [-1.5, -1.0, -1.0]);
    assert_eq!(BABY_NAUTILUS_INNER_MOUTH_CUBES[0].size, [3.0, 2.0, 2.0]);
    assert_eq!(BABY_NAUTILUS_LOWER_MOUTH_POSE.offset, [0.0, -0.01, 3.9]);
    assert_eq!(
        BABY_NAUTILUS_LOWER_MOUTH_CUBES[0].min,
        [-2.499, -0.999, 0.001]
    );
}

#[test]
fn baby_nautilus_mesh_uses_vanilla_body_layer_geometry() {
    // 8 cubes → 48 faces / 192 vertices / 288 indices, two tones: tan shell, pale body/mouths.
    let baby = entity_model_mesh(&[EntityModelInstance::nautilus(
        310,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 48);
    assert_eq!(baby.vertices.len(), 192);
    assert_eq!(baby.indices.len(), 288);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_SHELL, 1.0)));
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_BODY, 1.0)));
}

#[test]
fn baby_nautilus_is_smaller_than_the_adult() {
    // The hatchling `createBabyBodyLayer` is a compacted version of the adult mesh, so its bounds are
    // tighter on every axis (the shell shrinks from 14-wide to 7-wide, etc.).
    let adult = entity_model_mesh(&[EntityModelInstance::nautilus(
        312,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let baby = entity_model_mesh(&[EntityModelInstance::nautilus(
        312,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    for axis in 0..3 {
        let adult_span = adult_max[axis] - adult_min[axis];
        let baby_span = baby_max[axis] - baby_min[axis];
        assert!(
            baby_span < adult_span,
            "baby axis {axis} span {baby_span} should be smaller than adult {adult_span}"
        );
    }
}

#[test]
fn baby_nautilus_body_look_turns_the_body_and_mouths_not_the_shell() {
    // The baby shares the adult's hierarchy (shell is the first child, 3 cubes → vertices `[0, 72)`), so
    // the clamped body look turns the body and its mouths `[72, 192)` while the shell holds.
    let rest = EntityModelInstance::nautilus(313, [0.0, 64.0, 0.0], 0.0, true);
    let looked = rest.with_head_look(8.0, -8.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(
        rest_mesh.vertices[..72],
        looked_mesh.vertices[..72],
        "the shell stays put"
    );
    assert_ne!(
        rest_mesh.vertices[72..],
        looked_mesh.vertices[72..],
        "the body and its mouths turn with the look"
    );
}

#[test]
fn nautilus_exposes_stable_model_key() {
    assert_eq!(
        EntityModelKind::Nautilus { baby: false }.model_key(),
        "nautilus"
    );
    assert_eq!(
        EntityModelKind::Nautilus { baby: true }.model_key(),
        "nautilus_baby"
    );
}

#[test]
fn nautilus_textured_render_matches_vanilla_renderer() {
    let adult_pass = nautilus_textured_layer_passes(false)[0];
    assert_eq!(adult_pass.kind, EntityModelLayerKind::NautilusBase);
    assert_eq!(adult_pass.model_layer, MODEL_LAYER_NAUTILUS);
    assert_eq!(adult_pass.texture, NAUTILUS_TEXTURE_REF);
    assert_eq!(
        adult_pass.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult_pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult_pass.visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult_pass.order, adult_pass.submit_sequence), (0, 0));

    let baby_pass = nautilus_textured_layer_passes(true)[0];
    assert_eq!(baby_pass.kind, EntityModelLayerKind::NautilusBase);
    assert_eq!(baby_pass.model_layer, MODEL_LAYER_NAUTILUS_BABY);
    assert_eq!(baby_pass.texture, NAUTILUS_BABY_TEXTURE_REF);
    assert_eq!((baby_pass.order, baby_pass.submit_sequence), (0, 0));
    assert_eq!(
        EntityModelKind::Nautilus { baby: false }.vanilla_texture_ref(),
        Some(NAUTILUS_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Nautilus { baby: true }.vanilla_texture_ref(),
        Some(NAUTILUS_BABY_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&NAUTILUS_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&NAUTILUS_BABY_TEXTURE_REF));
    assert_eq!(
        NAUTILUS_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/nautilus_saddle/saddle.png",
            size: [128, 128],
        }
    );
    assert_eq!(
        NAUTILUS_BODY_IRON_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/nautilus_body/iron.png",
            size: [128, 128],
        }
    );
    assert_eq!(
        nautilus_body_armor_texture_ref(EntityArmorMaterial::Copper),
        Some(NAUTILUS_BODY_COPPER_TEXTURE_REF)
    );
    assert_eq!(
        nautilus_body_armor_texture_ref(EntityArmorMaterial::Iron),
        Some(NAUTILUS_BODY_IRON_TEXTURE_REF)
    );
    assert_eq!(
        nautilus_body_armor_texture_ref(EntityArmorMaterial::Gold),
        Some(NAUTILUS_BODY_GOLD_TEXTURE_REF)
    );
    assert_eq!(
        nautilus_body_armor_texture_ref(EntityArmorMaterial::Diamond),
        Some(NAUTILUS_BODY_DIAMOND_TEXTURE_REF)
    );
    assert_eq!(
        nautilus_body_armor_texture_ref(EntityArmorMaterial::Netherite),
        Some(NAUTILUS_BODY_NETHERITE_TEXTURE_REF)
    );
    assert_eq!(
        nautilus_body_armor_texture_ref(EntityArmorMaterial::Chainmail),
        None
    );
    assert!(entity_model_texture_refs().contains(&NAUTILUS_SADDLE_TEXTURE_REF));
    for texture in [
        NAUTILUS_BODY_COPPER_TEXTURE_REF,
        NAUTILUS_BODY_IRON_TEXTURE_REF,
        NAUTILUS_BODY_GOLD_TEXTURE_REF,
        NAUTILUS_BODY_DIAMOND_TEXTURE_REF,
        NAUTILUS_BODY_NETHERITE_TEXTURE_REF,
    ] {
        assert!(entity_model_texture_refs().contains(&texture));
    }
    assert_eq!(
        nautilus_entity_texture_refs(),
        &[NAUTILUS_TEXTURE_REF, NAUTILUS_BABY_TEXTURE_REF]
    );

    let images: Vec<EntityModelTextureImage> = nautilus_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        let instance = EntityModelInstance::nautilus(900, [0.0, 64.0, 0.0], 0.0, baby)
            .with_light_coords((6_u32 << 4) | (12_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert_nautilus_submissions_match_vanilla(&meshes, instance);
        assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
        let mesh = &meshes.cutout;
        assert!(
            !mesh.vertices.is_empty(),
            "baby={baby} emits textured geometry"
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
fn nautilus_saddle_layer_renders_for_adult_living_and_zombie_only() {
    let images: Vec<EntityModelTextureImage> = [
        NAUTILUS_TEXTURE_REF,
        NAUTILUS_BABY_TEXTURE_REF,
        ZOMBIE_NAUTILUS_TEXTURE_REF,
        ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF,
        NAUTILUS_SADDLE_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![(index * 40) as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == NAUTILUS_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;

    let adult = EntityModelInstance::nautilus(920, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult.with_nautilus_saddle(true);
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&bare_meshes, adult);
    assert_nautilus_submissions_match_vanilla(&saddled_meshes, saddled_instance);
    let bare = &bare_meshes.cutout;
    let saddled = &saddled_meshes.cutout;
    assert_eq!(saddled_meshes.submissions.len(), 2);
    let saddle_submit = saddled_meshes.submissions[1];
    assert_eq!(
        saddle_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        saddle_submit.render_type.vanilla_name(),
        "armorCutoutNoCull"
    );
    assert_eq!(saddle_submit.texture, NAUTILUS_SADDLE_TEXTURE_REF);
    assert_eq!(saddle_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((saddle_submit.order, saddle_submit.submit_sequence), (0, 1));
    assert_eq!(
        saddle_submit.transform,
        entity_model_root_transform(saddled_instance)
    );
    assert_eq!(saddle_submit.light, adult.render_state.shader_light());
    assert_eq!(saddle_submit.overlay, [0.0, 10.0]);
    // Saddle layer: one inflated shell cube plus the adult body/mouth subtree = 6 cubes.
    assert_eq!(saddled.cutout_faces - bare.cutout_faces, 36);
    assert_eq!(saddled.vertices.len() - bare.vertices.len(), 144);
    assert_eq!(
        saddled.vertices[bare.vertices.len()].light,
        adult.render_state.shader_light()
    );
    assert_eq!(saddled.vertices[bare.vertices.len()].overlay, [0.0, 10.0]);
    let first_saddle_vertex = saddled.vertices[bare.vertices.len()].uv;
    assert!(first_saddle_vertex[0] >= saddle_uv.min[0]);
    assert!(first_saddle_vertex[0] <= saddle_uv.max[0]);
    assert!(first_saddle_vertex[1] >= saddle_uv.min[1]);
    assert!(first_saddle_vertex[1] <= saddle_uv.max[1]);

    let baby = EntityModelInstance::nautilus(921, [0.0, 64.0, 0.0], 0.0, true);
    let baby_saddled_instance = baby.with_nautilus_saddle(true);
    let baby_bare = entity_model_textured_meshes(&[baby], &atlas);
    let baby_saddled = entity_model_textured_meshes(&[baby_saddled_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&baby_bare, baby);
    assert_nautilus_submissions_match_vanilla(&baby_saddled, baby_saddled_instance);
    assert_eq!(
        baby_saddled.cutout.vertices.len(),
        baby_bare.cutout.vertices.len(),
        "vanilla supplies no baby nautilus saddle model"
    );

    let zombie = EntityModelInstance::new(
        922,
        EntityModelKind::ZombieNautilus { coral: false },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let zombie_saddled_instance = zombie.with_nautilus_saddle(true);
    let zombie_bare = entity_model_textured_meshes(&[zombie], &atlas);
    let zombie_saddled = entity_model_textured_meshes(&[zombie_saddled_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&zombie_bare, zombie);
    assert_nautilus_submissions_match_vanilla(&zombie_saddled, zombie_saddled_instance);
    assert_eq!(
        zombie_saddled.cutout.cutout_faces - zombie_bare.cutout.cutout_faces,
        36
    );
    assert_eq!(
        zombie_saddled.cutout.vertices.len() - zombie_bare.cutout.vertices.len(),
        144
    );

    let warm = EntityModelInstance::new(
        923,
        EntityModelKind::ZombieNautilus { coral: true },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let warm_saddled_instance = warm.with_nautilus_saddle(true);
    let warm_bare = entity_model_textured_meshes(&[warm], &atlas);
    let warm_saddled = entity_model_textured_meshes(&[warm_saddled_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&warm_bare, warm);
    assert_nautilus_submissions_match_vanilla(&warm_saddled, warm_saddled_instance);
    assert_eq!(
        warm_saddled.cutout.cutout_faces - warm_bare.cutout.cutout_faces,
        36
    );
    assert_eq!(
        warm_saddled.cutout.vertices.len() - warm_bare.cutout.vertices.len(),
        144
    );
}

#[test]
fn nautilus_saddle_submission_survives_missing_saddle_texture_atlas_entry() {
    // Vanilla `NautilusRenderer` records `SimpleEquipmentLayer(NAUTILUS_SADDLE)` after the base body
    // at the default collector order, forcing `OverlayTexture.NO_OVERLAY`.
    let images: Vec<EntityModelTextureImage> = [NAUTILUS_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 40) as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::nautilus(924, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_nautilus_saddle(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, NAUTILUS_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, entity_model_root_transform(instance));
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert_eq!(
        meshes.cutout.vertices.len(),
        192,
        "missing nautilus_saddle/saddle.png suppresses only folded saddle geometry"
    );
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.light && vertex.overlay == base.overlay));

    let saddle = meshes.submissions[1];
    assert_eq!(
        saddle.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(saddle.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(saddle.texture, NAUTILUS_SADDLE_TEXTURE_REF);
    assert_eq!(saddle.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(saddle.transform, base.transform);
    assert_eq!((saddle.order, saddle.submit_sequence), (0, 1));
    assert_eq!(saddle.light, base.light);
    assert_eq!(saddle.overlay, [0.0, 10.0]);
    assert_nautilus_folded_meshes_are_cutout_only(&meshes);
}

#[test]
fn nautilus_body_armor_layer_renders_for_adult_living_and_zombie_only() {
    let images: Vec<EntityModelTextureImage> = [
        NAUTILUS_TEXTURE_REF,
        NAUTILUS_BABY_TEXTURE_REF,
        ZOMBIE_NAUTILUS_TEXTURE_REF,
        ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF,
        NAUTILUS_BODY_IRON_TEXTURE_REF,
        NAUTILUS_BODY_NETHERITE_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![(index * 35) as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let iron_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == NAUTILUS_BODY_IRON_TEXTURE_REF)
        .unwrap()
        .uv;

    let adult = EntityModelInstance::nautilus(930, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let armored_instance = adult.with_nautilus_body_armor(Some(EntityArmorMaterial::Iron));
    let armored_meshes = entity_model_textured_meshes(&[armored_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&bare_meshes, adult);
    assert_nautilus_submissions_match_vanilla(&armored_meshes, armored_instance);
    let bare = &bare_meshes.cutout;
    let armored = &armored_meshes.cutout;
    assert_eq!(armored_meshes.submissions.len(), 2);
    let armor_submit = armored_meshes.submissions[1];
    assert_eq!(
        armor_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(armor_submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(armor_submit.texture, NAUTILUS_BODY_IRON_TEXTURE_REF);
    assert_eq!(armor_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((armor_submit.order, armor_submit.submit_sequence), (0, 1));
    assert_eq!(
        armor_submit.transform,
        entity_model_root_transform(armored_instance)
    );
    assert_eq!(armor_submit.light, adult.render_state.shader_light());
    assert_eq!(armor_submit.overlay, [0.0, 10.0]);
    // Armor layer: adult shell/body/mouth subtree = 8 cubes.
    assert_eq!(armored.cutout_faces - bare.cutout_faces, 48);
    assert_eq!(armored.vertices.len() - bare.vertices.len(), 192);
    assert_eq!(
        armored.vertices[bare.vertices.len()].light,
        adult.render_state.shader_light()
    );
    assert_eq!(armored.vertices[bare.vertices.len()].overlay, [0.0, 10.0]);
    let first_armor_vertex = armored.vertices[bare.vertices.len()].uv;
    assert!(first_armor_vertex[0] >= iron_uv.min[0]);
    assert!(first_armor_vertex[0] <= iron_uv.max[0]);
    assert!(first_armor_vertex[1] >= iron_uv.min[1]);
    assert!(first_armor_vertex[1] <= iron_uv.max[1]);

    let invalid_material_instance =
        adult.with_nautilus_body_armor(Some(EntityArmorMaterial::Chainmail));
    let invalid_material = entity_model_textured_meshes(&[invalid_material_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&invalid_material, invalid_material_instance);
    assert_eq!(
        invalid_material.cutout.vertices.len(),
        bare.vertices.len(),
        "vanilla 26.1 has no chainmail nautilus_body equipment texture"
    );

    let baby = EntityModelInstance::nautilus(931, [0.0, 64.0, 0.0], 0.0, true);
    let baby_armored_instance = baby.with_nautilus_body_armor(Some(EntityArmorMaterial::Iron));
    let baby_bare = entity_model_textured_meshes(&[baby], &atlas);
    let baby_armored = entity_model_textured_meshes(&[baby_armored_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&baby_bare, baby);
    assert_nautilus_submissions_match_vanilla(&baby_armored, baby_armored_instance);
    assert_eq!(
        baby_armored.cutout.vertices.len(),
        baby_bare.cutout.vertices.len(),
        "vanilla supplies no baby nautilus armor model"
    );

    let zombie = EntityModelInstance::new(
        932,
        EntityModelKind::ZombieNautilus { coral: false },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let zombie_armored_instance =
        zombie.with_nautilus_body_armor(Some(EntityArmorMaterial::Netherite));
    let zombie_bare = entity_model_textured_meshes(&[zombie], &atlas);
    let zombie_armored = entity_model_textured_meshes(&[zombie_armored_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&zombie_bare, zombie);
    assert_nautilus_submissions_match_vanilla(&zombie_armored, zombie_armored_instance);
    assert_eq!(
        zombie_armored.cutout.cutout_faces - zombie_bare.cutout.cutout_faces,
        48
    );
    assert_eq!(
        zombie_armored.cutout.vertices.len() - zombie_bare.cutout.vertices.len(),
        192
    );

    let warm = EntityModelInstance::new(
        933,
        EntityModelKind::ZombieNautilus { coral: true },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let warm_armored_instance = warm.with_nautilus_body_armor(Some(EntityArmorMaterial::Netherite));
    let warm_bare = entity_model_textured_meshes(&[warm], &atlas);
    let warm_armored = entity_model_textured_meshes(&[warm_armored_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&warm_bare, warm);
    assert_nautilus_submissions_match_vanilla(&warm_armored, warm_armored_instance);
    assert_eq!(
        warm_bare.cutout.cutout_faces - zombie_bare.cutout.cutout_faces,
        48,
        "warm zombie nautilus normally adds the coral cluster"
    );
    assert_eq!(
        warm_armored.cutout.cutout_faces, zombie_armored.cutout.cutout_faces,
        "ZombieNautilusCoralModel hides corals while body armor is present"
    );
    assert_eq!(
        warm_armored.cutout.vertices.len(),
        zombie_armored.cutout.vertices.len()
    );
}

#[test]
fn nautilus_body_armor_submission_survives_missing_armor_texture_atlas_entry() {
    // Vanilla `NautilusRenderer` records `SimpleEquipmentLayer(NAUTILUS_BODY)` before the saddle
    // layer, at the default collector order, and forces `OverlayTexture.NO_OVERLAY`.
    let images: Vec<EntityModelTextureImage> = [NAUTILUS_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 35) as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::nautilus(934, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_nautilus_body_armor(Some(EntityArmorMaterial::Iron));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, NAUTILUS_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, entity_model_root_transform(instance));
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert_eq!(
        meshes.cutout.vertices.len(),
        192,
        "missing nautilus_body/iron.png suppresses only folded armor geometry"
    );
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.light && vertex.overlay == base.overlay));

    let armor = meshes.submissions[1];
    assert_eq!(
        armor.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(armor.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(armor.texture, NAUTILUS_BODY_IRON_TEXTURE_REF);
    assert_eq!(armor.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(armor.transform, base.transform);
    assert_eq!((armor.order, armor.submit_sequence), (0, 1));
    assert_eq!(armor.light, base.light);
    assert_eq!(armor.overlay, [0.0, 10.0]);
    assert_nautilus_folded_meshes_are_cutout_only(&meshes);
}

#[test]
fn zombie_nautilus_uses_its_own_texture_over_the_shared_adult_body() {
    // Vanilla `ZombieNautilusRenderer` `NORMAL` variant: the shared adult `NautilusModel` body
    // (`ModelLayers.ZOMBIE_NAUTILUS` bakes to `NautilusModel.createBodyLayer()`) textured by
    // `zombie_nautilus.png` — same geometry as the living adult nautilus, only the texture differs.
    assert_eq!(
        EntityModelKind::ZombieNautilus { coral: false }.model_key(),
        "zombie_nautilus"
    );
    assert_eq!(
        EntityModelKind::ZombieNautilus { coral: false }.vanilla_texture_ref(),
        Some(ZOMBIE_NAUTILUS_TEXTURE_REF)
    );
    let passes = zombie_nautilus_textured_layer_passes(false);
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::ZombieNautilusBase);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_ZOMBIE_NAUTILUS);
    assert_eq!(passes[0].texture, ZOMBIE_NAUTILUS_TEXTURE_REF);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    assert!(entity_model_texture_refs().contains(&ZOMBIE_NAUTILUS_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [NAUTILUS_TEXTURE_REF, ZOMBIE_NAUTILUS_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 30) as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let living_instance = EntityModelInstance::nautilus(900, [0.0, 64.0, 0.0], 0.0, false);
    let zombie_instance = EntityModelInstance::new(
        901,
        EntityModelKind::ZombieNautilus { coral: false },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let living = entity_model_textured_meshes(&[living_instance], &atlas);
    let zombie = entity_model_textured_meshes(&[zombie_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&living, living_instance);
    assert_nautilus_submissions_match_vanilla(&zombie, zombie_instance);
    assert!(!zombie.cutout.vertices.is_empty());
    assert!(zombie
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // Same shared adult-nautilus geometry: identical vertex positions...
    assert_eq!(living.cutout.vertices.len(), zombie.cutout.vertices.len());
    assert!(living
        .cutout
        .vertices
        .iter()
        .zip(&zombie.cutout.vertices)
        .all(|(a, b)| a.position == b.position));
    // ...but the zombie samples a different atlas region (its own texture).
    assert!(living
        .cutout
        .vertices
        .iter()
        .zip(&zombie.cutout.vertices)
        .any(|(a, b)| a.uv != b.uv));
}

#[test]
fn zombie_nautilus_warm_variant_adds_the_coral_cluster() {
    // Vanilla `ZombieNautilusRenderer` `WARM` variant: the `ZombieNautilusCoralModel` — the adult body
    // PLUS the `corals` subtree — over `zombie_nautilus_coral.png`. The corals are eight textured-only
    // cross-plane cubes (yellow ×2, pink + pink_second, blue ×2, red ×2), so the WARM mesh carries
    // exactly 8×6 = 48 more faces than the plain (`coral: false`) zombie nautilus body.
    assert_eq!(
        EntityModelKind::ZombieNautilus { coral: true }.model_key(),
        "zombie_nautilus_coral"
    );
    assert_eq!(
        EntityModelKind::ZombieNautilus { coral: true }.vanilla_texture_ref(),
        Some(ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF)
    );
    let coral_pass = zombie_nautilus_textured_layer_passes(true)[0];
    assert_eq!(coral_pass.kind, EntityModelLayerKind::ZombieNautilusBase);
    assert_eq!(coral_pass.model_layer, MODEL_LAYER_ZOMBIE_NAUTILUS_CORAL);
    assert_eq!(coral_pass.texture, ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF);
    assert_eq!(coral_pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!((coral_pass.order, coral_pass.submit_sequence), (0, 0));
    assert!(entity_model_texture_refs().contains(&ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [
        ZOMBIE_NAUTILUS_TEXTURE_REF,
        ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![(index * 30) as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let plain_instance = EntityModelInstance::new(
        902,
        EntityModelKind::ZombieNautilus { coral: false },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let warm_instance = EntityModelInstance::new(
        903,
        EntityModelKind::ZombieNautilus { coral: true },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let plain = entity_model_textured_meshes(&[plain_instance], &atlas);
    let warm = entity_model_textured_meshes(&[warm_instance], &atlas);
    assert_nautilus_submissions_match_vanilla(&plain, plain_instance);
    assert_nautilus_submissions_match_vanilla(&warm, warm_instance);
    assert!(!warm.cutout.vertices.is_empty());
    assert!(warm
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // The WARM mesh is the plain body plus the eight coral cross-plane cubes (48 faces, 192 vertices).
    assert_eq!(warm.cutout.cutout_faces, plain.cutout.cutout_faces + 48);
    assert_eq!(
        warm.cutout.vertices.len(),
        plain.cutout.vertices.len() + 48 * 4
    );
}

fn assert_nautilus_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_nautilus_folded_meshes_are_cutout_only(meshes);
    let base_passes = match instance.kind {
        EntityModelKind::Nautilus { baby } => nautilus_textured_layer_passes(baby),
        EntityModelKind::ZombieNautilus { coral } => zombie_nautilus_textured_layer_passes(coral),
        _ => panic!("expected nautilus instance"),
    };
    let mut expected = Vec::new();
    expected.extend(base_passes.iter().map(|pass| {
        (
            pass.render_type,
            pass.texture,
            pass.tint,
            pass.order,
            pass.submit_sequence,
        )
    }));

    let equipment_layers_supported = matches!(
        instance.kind,
        EntityModelKind::Nautilus { baby: false } | EntityModelKind::ZombieNautilus { .. }
    );
    let body_armor_texture = instance
        .render_state
        .nautilus_body_armor
        .and_then(nautilus_body_armor_texture_ref);
    if equipment_layers_supported {
        if let Some(texture) = body_armor_texture {
            expected.push((
                EntityModelLayerRenderType::ArmorCutoutNoCull,
                texture,
                [1.0, 1.0, 1.0, 1.0],
                0,
                1,
            ));
        }
        if instance.render_state.nautilus_saddle {
            expected.push((
                EntityModelLayerRenderType::ArmorCutoutNoCull,
                NAUTILUS_SADDLE_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
                0,
                1 + u32::from(body_armor_texture.is_some()),
            ));
        }
    }

    assert_eq!(meshes.submissions.len(), expected.len());
    for (submit, (render_type, texture, tint, order, sequence)) in
        meshes.submissions.iter().zip(expected)
    {
        assert_eq!(submit.render_type, render_type);
        let expected_render_type_name = match render_type {
            EntityModelLayerRenderType::EntityCutout => "entityCutout",
            EntityModelLayerRenderType::ArmorCutoutNoCull => "armorCutoutNoCull",
            _ => panic!("unexpected nautilus render type"),
        };
        assert_eq!(submit.render_type.vanilla_name(), expected_render_type_name);
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, tint);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!(submit.light, instance.render_state.shader_light());
        if render_type == EntityModelLayerRenderType::ArmorCutoutNoCull {
            assert_eq!(submit.overlay, [0.0, 10.0]);
        } else {
            assert_eq!(submit.overlay, instance.render_state.overlay_coords());
        }
        assert_eq!((submit.order, submit.submit_sequence), (order, sequence));
    }
}

fn assert_nautilus_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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
