use super::*;

#[test]
fn sheep_wool_color_table_matches_vanilla_color_lerper() {
    for entry in bbb_render_types::VANILLA_DYE_COLORS {
        let color = entry.color;
        let id = color.vanilla_id();
        let [red, green, blue] = [
            ((entry.sheep_wool_color >> 16) & 0xff) as u8,
            ((entry.sheep_wool_color >> 8) & 0xff) as u8,
            (entry.sheep_wool_color & 0xff) as u8,
        ];
        assert_eq!(SheepWoolColor::from_vanilla_id(id), color);
        assert_eq!(color.vanilla_id(), id);
        assert_eq!(
            sheep_wool_layer_color(color),
            [
                f32::from(red) / 255.0,
                f32::from(green) / 255.0,
                f32::from(blue) / 255.0,
                1.0
            ]
        );
    }
    assert_eq!(SheepWoolColor::from_vanilla_id(99), SheepWoolColor::White);
    assert_eq!(
        sheep_jeb_wool_layer_color(0.0),
        sheep_wool_layer_color(SheepWoolColor::White)
    );
    assert_eq!(
        sheep_jeb_wool_layer_color(25.0),
        sheep_wool_layer_color(SheepWoolColor::Orange)
    );
    assert_eq!(
        sheep_jeb_wool_layer_color(12.5),
        [208.0 / 255.0, 163.0 / 255.0, 125.0 / 255.0, 1.0]
    );
    assert_eq!(
        sheep_jeb_wool_layer_color(37.5),
        [167.0 / 255.0, 77.0 / 255.0, 81.0 / 255.0, 1.0]
    );
}

#[test]
fn sheep_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        SHEEP_WOOL_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/sheep/sheep_wool.png",
            size: [64, 32],
        }
    );
    assert_eq!(
        SHEEP_WOOL_BABY_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/sheep/sheep_wool_baby.png",
            size: [64, 32],
        }
    );
    assert_eq!(
        SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/sheep/sheep_wool_undercoat.png",
            size: [64, 32],
        }
    );
}

#[test]
fn sheep_textured_layer_passes_match_vanilla_renderer_layers() {
    let adult_red = sheep_textured_layer_passes(false, false, SheepWoolColor::Red, false, 0.0);
    assert_eq!(adult_red.len(), 3);
    assert!(adult_red
        .iter()
        .all(|pass| pass.render_type == EntityModelLayerRenderType::EntityCutout));
    assert!(adult_red
        .iter()
        .all(|pass| pass.render_type.vanilla_name() == "entityCutout"));
    assert_eq!(adult_red[0].texture, SHEEP_TEXTURE_REF);
    assert_eq!(adult_red[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult_red[0].order, 0);
    assert_eq!(adult_red[1].texture, SHEEP_WOOL_TEXTURE_REF);
    assert_eq!(
        adult_red[1].tint,
        sheep_wool_layer_color(SheepWoolColor::Red)
    );
    assert_eq!(adult_red[1].order, 0);
    assert_eq!(adult_red[2].texture, SHEEP_WOOL_UNDERCOAT_TEXTURE_REF);
    assert_eq!(
        adult_red[2].tint,
        sheep_wool_layer_color(SheepWoolColor::Red)
    );
    assert_eq!(adult_red[2].order, 1);

    let sheared_red = sheep_textured_layer_passes(false, true, SheepWoolColor::Red, false, 0.0);
    assert_eq!(
        sheared_red
            .iter()
            .map(|pass| pass.texture)
            .collect::<Vec<_>>(),
        vec![SHEEP_TEXTURE_REF, SHEEP_WOOL_UNDERCOAT_TEXTURE_REF]
    );
    let sheared_white = sheep_textured_layer_passes(false, true, SheepWoolColor::White, false, 0.0);
    assert_eq!(sheared_white.len(), 1);

    let adult_jeb_white =
        sheep_textured_layer_passes(false, false, SheepWoolColor::White, true, 12.5);
    assert_eq!(
        adult_jeb_white
            .iter()
            .map(|pass| pass.texture)
            .collect::<Vec<_>>(),
        vec![
            SHEEP_TEXTURE_REF,
            SHEEP_WOOL_TEXTURE_REF,
            SHEEP_WOOL_UNDERCOAT_TEXTURE_REF
        ]
    );
    assert_eq!(adult_jeb_white[1].tint, sheep_jeb_wool_layer_color(12.5));
    assert_eq!(adult_jeb_white[2].tint, sheep_jeb_wool_layer_color(12.5));

    let baby_black = sheep_textured_layer_passes(true, false, SheepWoolColor::Black, false, 0.0);
    assert_eq!(
        baby_black
            .iter()
            .map(|pass| (pass.texture, pass.order))
            .collect::<Vec<_>>(),
        vec![
            (SHEEP_BABY_TEXTURE_REF, 0),
            (SHEEP_WOOL_BABY_TEXTURE_REF, 1)
        ]
    );
    let sheared_baby_black =
        sheep_textured_layer_passes(true, true, SheepWoolColor::Black, false, 0.0);
    assert_eq!(sheared_baby_black.len(), 1);
    let jeb_baby_white =
        sheep_textured_layer_passes(true, false, SheepWoolColor::White, true, 25.0);
    assert_eq!(
        jeb_baby_white
            .iter()
            .map(|pass| pass.texture)
            .collect::<Vec<_>>(),
        vec![SHEEP_BABY_TEXTURE_REF, SHEEP_WOOL_BABY_TEXTURE_REF]
    );
    assert_eq!(jeb_baby_white[1].tint, sheep_jeb_wool_layer_color(25.0));
}

#[test]
fn entity_texture_atlas_stitches_official_sheep_png_slots() {
    let images = [
        SHEEP_TEXTURE_REF,
        SHEEP_BABY_TEXTURE_REF,
        SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
        SHEEP_WOOL_TEXTURE_REF,
        SHEEP_WOOL_BABY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect::<Vec<_>>();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 160);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/sheep/sheep.png",
            "textures/entity/sheep/sheep_baby.png",
            "textures/entity/sheep/sheep_wool_undercoat.png",
            "textures/entity/sheep/sheep_wool.png",
            "textures/entity/sheep/sheep_wool_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.2]);
    assert_close2(layout.entries[2].uv.min, [0.0, 0.4]);
    assert_close2(layout.entries[2].uv.max, [1.0, 0.6]);
    assert_close2(layout.entries[3].uv.min, [0.0, 0.6]);
    assert_close2(layout.entries[3].uv.max, [1.0, 0.8]);
    let undercoat_first_pixel = rgba_offset(layout.width, 64, 0, "test").unwrap();
    assert_eq!(
        &rgba[undercoat_first_pixel..undercoat_first_pixel + 4],
        &[2; 4]
    );
    let wool_first_pixel = rgba_offset(layout.width, 96, 0, "test").unwrap();
    assert_eq!(&rgba[wool_first_pixel..wool_first_pixel + 4], &[3; 4]);
}

#[test]
fn sheep_textured_mesh_uses_vanilla_uvs_tints_and_layer_visibility() {
    let (atlas, _) = build_entity_model_texture_atlas(&sheep_texture_images()).unwrap();
    let red_instance = sheep_submission_probe(EntityModelInstance::sheep_wool(
        301,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
    ));
    let meshes = entity_model_textured_meshes(&[red_instance], &atlas);
    assert_sheep_textured_output(&meshes, red_instance);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 108);
    assert_eq!(mesh.vertices.len(), 432);
    assert_eq!(mesh.indices.len(), 648);
    assert_close2(mesh.vertices[0].uv, [14.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        mesh.vertices[144].tint,
        sheep_wool_layer_color(SheepWoolColor::Red)
    );
    assert_close2(mesh.vertices[144].uv, [12.0 / 64.0, 0.6]);
    assert_eq!(
        mesh.vertices[288].tint,
        sheep_wool_layer_color(SheepWoolColor::Red)
    );
    assert_close2(mesh.vertices[288].uv, [14.0 / 64.0, 0.4]);
    let red_base_overlay = red_instance.render_state.overlay_coords();
    let red_wool_overlay = [0.0, red_base_overlay[1]];
    assert_ne!(red_base_overlay, red_wool_overlay);
    assert_eq!(
        mesh.vertices[0].light,
        red_instance.render_state.shader_light()
    );
    assert_eq!(
        mesh.vertices[144].light,
        red_instance.render_state.shader_light()
    );
    assert_eq!(
        mesh.vertices[288].light,
        red_instance.render_state.shader_light()
    );
    assert_eq!(mesh.vertices[0].overlay, red_base_overlay);
    assert_eq!(mesh.vertices[144].overlay, red_wool_overlay);
    assert_eq!(mesh.vertices[288].overlay, red_wool_overlay);

    let sheared_instance = sheep_submission_probe(EntityModelInstance::sheep_wool(
        302,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        SheepWoolColor::Red,
    ));
    let sheared_meshes = entity_model_textured_meshes(&[sheared_instance], &atlas);
    assert_sheep_textured_output(&sheared_meshes, sheared_instance);
    let sheared = &sheared_meshes.cutout;
    assert_eq!(sheared.cutout_faces, 72);
    assert_eq!(sheared.vertices.len(), 288);

    let sheared_baby_instance = sheep_submission_probe(EntityModelInstance::sheep_wool(
        303,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        SheepWoolColor::Black,
    ));
    let sheared_baby_meshes = entity_model_textured_meshes(&[sheared_baby_instance], &atlas);
    assert_sheep_textured_output(&sheared_baby_meshes, sheared_baby_instance);
    let sheared_baby = &sheared_baby_meshes.cutout;
    assert_eq!(sheared_baby.cutout_faces, 36);
    assert!(sheared_baby
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let jeb_white_instance = sheep_submission_probe(EntityModelInstance::sheep_render_state(
        304,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::White,
        false,
        true,
        12.5,
    ));
    let jeb_white_meshes = entity_model_textured_meshes(&[jeb_white_instance], &atlas);
    assert_sheep_textured_output(&jeb_white_meshes, jeb_white_instance);
    let jeb_white = &jeb_white_meshes.cutout;
    assert_eq!(jeb_white.cutout_faces, 108);
    assert_eq!(jeb_white.vertices.len(), 432);
    assert_eq!(
        jeb_white.vertices[144].tint,
        sheep_jeb_wool_layer_color(12.5)
    );
    assert_eq!(
        jeb_white.vertices[288].tint,
        sheep_jeb_wool_layer_color(12.5)
    );

    // An invisible sheep renders nothing: the unified `render_state.invisible` skips the whole
    // model in both paths, so no body and no wool/undercoat layer is emitted.
    let invisible_red_instance = EntityModelInstance::sheep_render_state(
        305,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
        true,
        false,
        0.0,
    );
    let invisible_red = entity_model_textured_meshes(&[invisible_red_instance], &atlas);
    assert_sheep_folded_meshes_are_cutout_only(&invisible_red);

    assert_eq!(invisible_red.cutout.cutout_faces, 0);
    assert!(invisible_red.cutout.vertices.is_empty());

    // Vanilla `LivingEntityRenderer.getRenderType`: when an invisible sheep is still visible to this
    // client (`!isInvisibleToPlayer`), only the base body uses
    // `entityTranslucentCullItemTarget` with the `0x26ffffff` force-transparent alpha. Sheep wool /
    // undercoat layers still see `state.isInvisible` and do not submit.
    let self_visible_invisible = invisible_red_instance.with(|s| {
        s.invisible_to_player = false;
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.has_red_overlay = true;
    });
    let self_visible = entity_model_textured_meshes(&[self_visible_invisible], &atlas);

    assert!(self_visible.cutout.vertices.is_empty());
    assert!(self_visible.translucent.vertices.is_empty());
    assert_eq!(self_visible.item_entity_translucent_cull.cutout_faces, 36);
    assert_eq!(
        self_visible.item_entity_translucent_cull.vertices.len(),
        144
    );
    assert!(self_visible
        .item_entity_translucent_cull
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 38.0 / 255.0]
            && vertex.light == self_visible_invisible.render_state.shader_light()
            && vertex.overlay == self_visible_invisible.render_state.overlay_coords()));

    // Vanilla `SheepWoolLayer`: an invisible sheep that appears glowing submits outline
    // metadata for the base body and unsheared wool. `SheepWoolUndercoatLayer` still gates on
    // `!state.isInvisible`, so the adult red undercoat is not submitted.
    let glowing_invisible = invisible_red_instance
        .with_outline_color(0xff55_ff55)
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let glowing = entity_model_textured_meshes(&[glowing_invisible], &atlas);

    assert!(glowing.cutout.vertices.is_empty());
    assert!(glowing.translucent.vertices.is_empty());
    assert!(glowing.eyes.vertices.is_empty());
    assert_eq!(glowing.outline.cutout_faces, 72);
    assert_eq!(glowing.outline.vertices.len(), 288);
    assert_eq!(glowing.outline.indices.len(), 432);
    let outline_tint = [0x55 as f32 / 255.0, 1.0, 0x55 as f32 / 255.0, 1.0];
    let base_overlay = glowing_invisible.render_state.overlay_coords();
    for (range, overlay) in [(0..144, base_overlay), (144..288, [0.0, base_overlay[1]])] {
        assert!(glowing.outline.vertices[range].iter().all(|vertex| {
            vertex.tint == outline_tint
                && vertex.light == glowing_invisible.render_state.shader_light()
                && vertex.overlay == overlay
        }));
    }
}

#[test]
fn sheep_wool_missing_textures_preserve_other_geometry() {
    // Vanilla adds undercoat before wool, but `SubmitNodeCollector.order` sorts adult wool with the
    // base order and adult undercoat after it; missing atlas data suppresses only folded layer geometry.
    let images = [SHEEP_TEXTURE_REF, SHEEP_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect::<Vec<_>>();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    for instance in [
        sheep_submission_probe(EntityModelInstance::sheep_wool(
            306,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            SheepWoolColor::Red,
        )),
        sheep_submission_probe(EntityModelInstance::sheep_wool(
            307,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            false,
            SheepWoolColor::Black,
        )),
    ] {
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert_sheep_folded_meshes_are_cutout_only(&meshes);
        let EntityModelKind::Sheep {
            baby,
            sheared,
            wool_color,
            jeb,
            age_ticks,
        } = instance.kind
        else {
            panic!("expected sheep instance");
        };
        let passes = sheep_textured_layer_passes(baby, sheared, wool_color, jeb, age_ticks);

        assert!(
            passes.len() > 1,
            "the case must include a missing wool layer"
        );

        let base_overlay = instance.render_state.overlay_coords();
        let zero_white_overlay = [0.0, base_overlay[1]];

        assert!(
            !meshes.cutout.vertices.is_empty(),
            "missing sheep wool textures keep the base mesh"
        );
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0; 4]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()));
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.overlay != zero_white_overlay));
    }
}

#[test]
fn sheep_head_eat_position_scale_matches_vanilla_curve() {
    // Vanilla Sheep.getHeadEatPositionScale(partialTick): ramp up over ticks
    // 40..36, plateau at 1.0 over 36..4, ramp down over 4..0.
    let cases: [((i32, f32), f32); 12] = [
        ((0, 0.0), 0.0),
        ((0, 0.5), 0.0),
        ((1, 0.0), 0.25),
        ((3, 0.0), 0.75),
        ((3, 0.5), 0.625),
        ((4, 0.0), 1.0),
        ((20, 0.5), 1.0),
        ((36, 0.0), 1.0),
        ((37, 0.0), 0.75),
        ((39, 0.0), 0.25),
        ((40, 0.0), 0.0),
        ((40, 0.5), 0.125),
    ];
    for ((tick, partial), expected) in cases {
        assert_eq!(
            SheepHeadEatPose::from_eat_tick(tick, partial).position_scale,
            expected,
            "position scale tick={tick} partial={partial}"
        );
    }
}

#[test]
fn sheep_head_eat_angle_scale_matches_vanilla_curve() {
    // Vanilla Sheep.getHeadEatAngleScale(partialTick).
    let plateau = std::f32::consts::PI / 5.0;
    // Not eating: vanilla folds in the head-look pitch. The eat-pose scalar
    // only carries the eating branches, so the resting eat angle is 0.0 and
    // `sheep_head_pose` applies the separate head-look pitch.
    assert_eq!(SheepHeadEatPose::from_eat_tick(0, 0.0).angle_scale, 0.0);
    // The ramp-in/ramp-out and short ticks hold the constant plateau angle.
    for tick in [1, 2, 3, 4, 37, 38, 39, 40] {
        assert_eq!(
            SheepHeadEatPose::from_eat_tick(tick, 0.0).angle_scale,
            plateau,
            "plateau angle tick={tick}"
        );
    }
    // Ticks 5..=36 oscillate around the plateau with amplitude 0.21991149.
    for tick in 5..=36 {
        let angle = SheepHeadEatPose::from_eat_tick(tick, 0.0).angle_scale;
        assert!(
            (angle - plateau).abs() <= 0.21991149 + 1e-6,
            "amplitude tick={tick}"
        );
    }
    // Spot value pinned to the vanilla constants 0.21991149 and 28.7.
    let scale = (20.0_f32 - 4.0 - 0.0) / 32.0;
    let expected = plateau + 0.21991149 * (scale * 28.7).sin();
    assert!((SheepHeadEatPose::from_eat_tick(20, 0.0).angle_scale - expected).abs() < 1e-6);
}

#[test]
fn sheep_head_pose_matches_vanilla_setup_anim() {
    // Vanilla QuadrupedModel.setupAnim then SheepModel/SheepFurModel.setupAnim:
    //   head.xRot = xRot * PI/180; head.yRot = yRot * PI/180   (super)
    //   head.y += headEatPositionScale * 9.0 * ageScale         (sheep)
    //   head.xRot = headEatAngleScale                           (sheep, overrides pitch)
    // The head bind poses are the vanilla `SheepModel`/`BabySheepModel` head offsets.
    let adult_head = PartPose {
        offset: [0.0, 6.0, -8.0],
        rotation: [0.0, 0.0, 0.0],
    };
    // Eating overrides head.xRot with the eat angle; head.yRot stays at the look
    // yaw (0 here). ageScale = 1.0, so the head drops the full 9.0 model units.
    let adult_eaten = sheep_head_pose(
        adult_head,
        1.0,
        SheepHeadEatPose {
            position_scale: 1.0,
            angle_scale: 0.5,
        },
        0.0,
        0.0,
    );
    assert_eq!(
        adult_eaten.offset,
        [
            adult_head.offset[0],
            adult_head.offset[1] + 9.0,
            adult_head.offset[2]
        ]
    );
    assert_eq!(adult_eaten.rotation, [0.5, 0.0, adult_head.rotation[2]]);

    // BabySheepModel extends SheepModel; LivingEntity.getAgeScale = 0.5 for a
    // baby, so the head drops by 9.0 * 0.5 = 4.5.
    let baby_head = PartPose {
        offset: [0.0, 15.5, -2.5],
        rotation: [0.0, 0.0, 0.0],
    };
    let baby_eaten = sheep_head_pose(
        baby_head,
        0.5,
        SheepHeadEatPose {
            position_scale: 1.0,
            angle_scale: 0.3,
        },
        0.0,
        0.0,
    );
    assert_eq!(
        baby_eaten.offset,
        [
            baby_head.offset[0],
            baby_head.offset[1] + 4.5,
            baby_head.offset[2]
        ]
    );
    assert_eq!(baby_eaten.rotation, [0.3, 0.0, baby_head.rotation[2]]);

    // Not eating with a head-look turn: head.yRot = yRot*PI/180 (QuadrupedModel
    // super) and head.xRot = getXRot*PI/180 (Sheep.getHeadEatAngleScale's
    // non-eating branch). No vertical dip while at rest.
    let looking = sheep_head_pose(adult_head, 1.0, SheepHeadEatPose::NONE, 40.0, -18.0);
    assert_eq!(looking.offset, adult_head.offset);
    assert!((looking.rotation[0] - (-18.0_f32).to_radians()).abs() < 1e-6);
    assert!((looking.rotation[1] - 40.0_f32.to_radians()).abs() < 1e-6);
    assert_eq!(looking.rotation[2], adult_head.rotation[2]);

    // Eating wins the pitch (eat angle), but the look yaw still applies.
    let eating_and_looking = sheep_head_pose(
        adult_head,
        1.0,
        SheepHeadEatPose {
            position_scale: 1.0,
            angle_scale: 0.5,
        },
        40.0,
        -18.0,
    );
    assert_eq!(eating_and_looking.rotation[0], 0.5);
    assert!((eating_and_looking.rotation[1] - 40.0_f32.to_radians()).abs() < 1e-6);

    // Fully at rest (no eat, no look) leaves the head part untouched.
    assert!(sheep_head_at_rest(SheepHeadEatPose::NONE, 0.0, 0.0));
    assert!(!sheep_head_at_rest(SheepHeadEatPose::NONE, 1.0, 0.0));
    assert!(!sheep_head_at_rest(SheepHeadEatPose::NONE, 0.0, 1.0));
    assert_eq!(
        sheep_head_pose(adult_head, 1.0, SheepHeadEatPose::NONE, 0.0, 0.0),
        adult_head
    );
}

#[test]
fn sheep_textured_mesh_applies_eating_head_pose_to_all_layers() {
    let (atlas, _) = build_entity_model_texture_atlas(&sheep_texture_images()).unwrap();
    let resting_instance = EntityModelInstance::sheep_wool(
        401,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
    );
    let resting = entity_model_textured_meshes(&[resting_instance], &atlas);
    assert_sheep_textured_output(&resting, resting_instance);
    // Tick 20: full head dip (positionScale 1.0) plus the plateau oscillation.
    let eating_instance = EntityModelInstance::sheep_eating(
        401,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
        20,
        0.0,
    );
    let eating = entity_model_textured_meshes(&[eating_instance], &atlas);
    assert_sheep_textured_output(&eating, eating_instance);

    // Base, wool, and undercoat each contribute 144 vertices; the head cube is
    // the first 24 vertices of each pass.
    assert_eq!(resting.cutout.vertices.len(), 432);
    assert_eq!(eating.cutout.vertices.len(), 432);
    for pass_start in [0usize, 144, 288] {
        let head = pass_start..pass_start + 24;
        let body_and_legs = pass_start + 24..pass_start + 144;
        assert_ne!(
            resting.cutout.vertices[head.clone()],
            eating.cutout.vertices[head.clone()],
            "head animates in pass at {pass_start}"
        );
        assert_eq!(
            resting.cutout.vertices[body_and_legs.clone()],
            eating.cutout.vertices[body_and_legs],
            "body and legs stay put in pass at {pass_start}"
        );
        // The root transform flips Y, so a larger model head.y dips the head down.
        assert!(
            average_textured_y(&eating.cutout.vertices[head.clone()])
                < average_textured_y(&resting.cutout.vertices[head]),
            "head dips downward in pass at {pass_start}"
        );
    }
}

#[test]
fn sheep_textured_mesh_swings_legs_when_walking() {
    // Vanilla SheepModel.setupAnim runs super.setupAnim (the QuadrupedModel leg
    // swing) before the eat-grass head pose, so a walking sheep swings its legs in
    // every layer (base, wool, undercoat) while the head — at rest here — is
    // untouched.
    let (atlas, _) = build_entity_model_texture_atlas(&sheep_texture_images()).unwrap();
    let base = EntityModelInstance::sheep_wool(
        403,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
    );
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_sheep_textured_output(&resting, base);
    assert_sheep_textured_output(&still, still_instance);
    assert_sheep_textured_output(&walking, walking_instance);

    assert_eq!(
        resting.cutout.vertices, still.cutout.vertices,
        "a standing sheep is inert"
    );
    assert_eq!(resting.cutout.vertices.len(), walking.cutout.vertices.len());
    // Base, wool, and undercoat each contribute 144 vertices; the head cube is the
    // first 24, the body and legs the remaining 120.
    for pass_start in [0usize, 144, 288] {
        let head = pass_start..pass_start + 24;
        let body_and_legs = pass_start + 24..pass_start + 144;
        assert_eq!(
            resting.cutout.vertices[head.clone()],
            walking.cutout.vertices[head],
            "head is unaffected by the leg swing in pass at {pass_start}"
        );
        assert_ne!(
            resting.cutout.vertices[body_and_legs.clone()],
            walking.cutout.vertices[body_and_legs],
            "legs swing in pass at {pass_start}"
        );
    }
}

fn assert_sheep_textured_output(meshes: &EntityModelTexturedMeshes, instance: EntityModelInstance) {
    assert_sheep_folded_meshes_are_cutout_only(meshes);
    let EntityModelKind::Sheep {
        baby,
        sheared,
        wool_color,
        jeb,
        age_ticks,
    } = instance.kind
    else {
        panic!("expected sheep instance");
    };
    let passes = sheep_textured_layer_passes(baby, sheared, wool_color, jeb, age_ticks);

    let base_overlay = instance.render_state.overlay_coords();
    let zero_white_overlay = [0.0, base_overlay[1]];
    let has_zero_white_overlay = passes.iter().any(|pass| {
        matches!(
            pass.kind,
            EntityModelLayerKind::SheepWool | EntityModelLayerKind::ZeroWhiteOverlay
        )
    });

    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
    if has_zero_white_overlay && zero_white_overlay != base_overlay {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.overlay == base_overlay));
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.overlay == zero_white_overlay));
        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.overlay == base_overlay || vertex.overlay == zero_white_overlay
        }));
    } else {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.overlay == base_overlay));
    }
}

fn sheep_submission_probe(instance: EntityModelInstance) -> EntityModelInstance {
    instance.with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
}

fn assert_sheep_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.outline.vertices.is_empty());
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

fn average_textured_y(vertices: &[EntityModelTexturedVertex]) -> f32 {
    vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .sum::<f32>()
        / vertices.len() as f32
}

fn sheep_texture_images() -> Vec<EntityModelTextureImage> {
    [
        SHEEP_TEXTURE_REF,
        SHEEP_BABY_TEXTURE_REF,
        SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
        SHEEP_WOOL_TEXTURE_REF,
        SHEEP_WOOL_BABY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}
