use super::*;

#[test]
fn sheep_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_SHEEP_HEAD[0],
        ModelCubeDesc {
            min: [-3.0, -4.0, -6.0],
            size: [6.0, 6.0, 8.0],
            color: SHEEP_WOOL,
        }
    );
    assert_eq!(ADULT_SHEEP_PARTS.len(), 6);
    assert_part(
        &ADULT_SHEEP_PARTS[0],
        [0.0, 6.0, -8.0],
        [0.0, 0.0, 0.0],
        ADULT_SHEEP_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_SHEEP_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_SHEEP_BODY.as_slice(),
    );
    for (part, expected_offset) in ADULT_SHEEP_PARTS[2..].iter().zip([
        [-3.0, 12.0, 7.0],
        [3.0, 12.0, 7.0],
        [-3.0, 12.0, -5.0],
        [3.0, 12.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_SHEEP_LEG.as_slice(),
        );
    }
}

#[test]
fn sheep_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::sheep_wool(
        94,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        SheepWoolColor::White,
    )]);

    assert_eq!(mesh.opaque_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.3125, 64.001, -0.5625]);
    assert_close3(max, [0.3125, 65.376, 0.875]);
}

#[test]
fn sheep_wool_layer_parts_match_vanilla_26_1_fur_layer() {
    assert_eq!(
        ADULT_SHEEP_WOOL_HEAD[0],
        ModelCubeDesc {
            min: [-3.6, -4.6, -4.6],
            size: [7.2, 7.2, 7.2],
            color: SHEEP_WOOL,
        }
    );
    assert_eq!(
        ADULT_SHEEP_WOOL_BODY[0],
        ModelCubeDesc {
            min: [-5.75, -11.75, -8.75],
            size: [11.5, 19.5, 9.5],
            color: SHEEP_WOOL,
        }
    );
    assert_eq!(
        ADULT_SHEEP_WOOL_LEG[0],
        ModelCubeDesc {
            min: [-2.5, -0.5, -2.5],
            size: [5.0, 7.0, 5.0],
            color: SHEEP_WOOL,
        }
    );
    assert_eq!(ADULT_SHEEP_WOOL_PARTS.len(), 6);
    assert_part(
        &ADULT_SHEEP_WOOL_PARTS[0],
        [0.0, 6.0, -8.0],
        [0.0, 0.0, 0.0],
        ADULT_SHEEP_WOOL_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_SHEEP_WOOL_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_SHEEP_WOOL_BODY.as_slice(),
    );
    for (part, expected_offset) in ADULT_SHEEP_WOOL_PARTS[2..].iter().zip([
        [-3.0, 12.0, 7.0],
        [3.0, 12.0, 7.0],
        [-3.0, 12.0, -5.0],
        [3.0, 12.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_SHEEP_WOOL_LEG.as_slice(),
        );
    }
}

#[test]
fn sheep_wool_color_table_matches_vanilla_color_lerper() {
    let cases: [(u8, SheepWoolColor, [u8; 3]); 16] = [
        (0, SheepWoolColor::White, [230, 230, 230]),
        (1, SheepWoolColor::Orange, [186, 96, 21]),
        (2, SheepWoolColor::Magenta, [149, 58, 141]),
        (3, SheepWoolColor::LightBlue, [43, 134, 163]),
        (4, SheepWoolColor::Yellow, [190, 162, 45]),
        (5, SheepWoolColor::Lime, [96, 149, 23]),
        (6, SheepWoolColor::Pink, [182, 104, 127]),
        (7, SheepWoolColor::Gray, [53, 59, 61]),
        (8, SheepWoolColor::LightGray, [117, 117, 113]),
        (9, SheepWoolColor::Cyan, [16, 117, 117]),
        (10, SheepWoolColor::Purple, [102, 37, 138]),
        (11, SheepWoolColor::Blue, [45, 51, 127]),
        (12, SheepWoolColor::Brown, [98, 63, 37]),
        (13, SheepWoolColor::Green, [70, 93, 16]),
        (14, SheepWoolColor::Red, [132, 34, 28]),
        (15, SheepWoolColor::Black, [21, 21, 24]),
    ];

    for (id, color, [red, green, blue]) in cases {
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
fn sheep_wool_layer_mesh_applies_vanilla_visibility_and_color() {
    let unsheared_white =
        entity_model_mesh(&[EntityModelInstance::sheep(96, [0.0, 64.0, 0.0], 0.0, false)]);
    assert_eq!(unsheared_white.opaque_faces, 72);
    assert_eq!(unsheared_white.vertices.len(), 288);
    assert_eq!(unsheared_white.indices.len(), 432);
    assert!(unsheared_white
        .vertices
        .iter()
        .any(|vertex| vertex.color
            == shade_color(sheep_wool_layer_color(SheepWoolColor::White), 1.0)));

    let unsheared_red = entity_model_mesh(&[EntityModelInstance::sheep_wool(
        97,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
    )]);
    assert_eq!(unsheared_red.opaque_faces, 108);
    assert_eq!(unsheared_red.vertices.len(), 432);
    assert_eq!(unsheared_red.indices.len(), 648);
    assert!(unsheared_red.vertices.iter().any(
        |vertex| vertex.color == shade_color(sheep_wool_layer_color(SheepWoolColor::Red), 1.0)
    ));

    let sheared_red = entity_model_mesh(&[EntityModelInstance::sheep_wool(
        98,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        SheepWoolColor::Red,
    )]);
    assert_eq!(sheared_red.opaque_faces, 72);
    assert_eq!(sheared_red.vertices.len(), 288);
    assert_eq!(sheared_red.indices.len(), 432);
    let (min, max) = mesh_extents(&sheared_red);
    assert_close3(min, [-0.3125, 64.001, -0.5625]);
    assert_close3(max, [0.3125, 65.376, 0.875]);

    let sheared_red_baby = entity_model_mesh(&[EntityModelInstance::sheep_wool(
        99,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        SheepWoolColor::Red,
    )]);
    assert_eq!(sheared_red_baby.opaque_faces, 36);
    assert!(!sheared_red_baby.vertices.iter().any(
        |vertex| vertex.color == shade_color(sheep_wool_layer_color(SheepWoolColor::Red), 1.0)
    ));
}

#[test]
fn sheep_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_SHEEP_BODY[0],
        ModelCubeDesc {
            min: [-3.0, -2.0, -4.5],
            size: [6.0, 4.0, 9.0],
            color: SHEEP_WOOL,
        }
    );
    assert_eq!(BABY_SHEEP_PARTS.len(), 6);
    assert_part(
        &BABY_SHEEP_PARTS[0],
        [0.0, 17.0, 0.5],
        [0.0, 0.0, 0.0],
        BABY_SHEEP_BODY.as_slice(),
    );
    assert_part(
        &BABY_SHEEP_PARTS[1],
        [0.0, 15.5, -2.5],
        [0.0, 0.0, 0.0],
        BABY_SHEEP_HEAD.as_slice(),
    );
    for (part, expected_offset) in BABY_SHEEP_PARTS[2..].iter().zip([
        [-2.0, 19.0, 3.0],
        [2.0, 19.0, 3.0],
        [-2.0, 19.0, -2.0],
        [2.0, 19.0, -2.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_SHEEP_LEG.as_slice(),
        );
    }
}

#[test]
fn sheep_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::sheep_wool(
        95,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        SheepWoolColor::White,
    )]);

    assert_eq!(mesh.opaque_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.1875, 64.001, -0.3125]);
    assert_close3(max, [0.1875, 64.8135, 0.375]);
}

#[test]
fn sheep_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
        .model_key(),
        "sheep"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
        .model_key(),
        "sheep_red"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: true,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
        .model_key(),
        "sheep_red_sheared"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: true,
            sheared: true,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
        .model_key(),
        "sheep_baby_sheared"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/sheep/sheep.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/sheep/sheep_baby.png",
            size: [64, 32],
        })
    );
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
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
        .vanilla_layer_texture_refs(),
        &[SHEEP_WOOL_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
        .vanilla_layer_texture_refs(),
        &[SHEEP_WOOL_UNDERCOAT_TEXTURE_REF, SHEEP_WOOL_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: true,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
        .vanilla_layer_texture_refs(),
        &[SHEEP_WOOL_UNDERCOAT_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::Black,
            jeb: false,
            age_ticks: 0.0,
        }
        .vanilla_layer_texture_refs(),
        &[SHEEP_WOOL_BABY_TEXTURE_REF]
    );
    assert!(EntityModelKind::Sheep {
        baby: true,
        sheared: true,
        wool_color: SheepWoolColor::Black,
        jeb: false,
        age_ticks: 0.0,
    }
    .vanilla_layer_texture_refs()
    .is_empty());
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: true,
            age_ticks: 12.5,
        }
        .model_key(),
        "sheep_jeb"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: true,
            age_ticks: 12.5,
        }
        .vanilla_layer_texture_refs(),
        &[SHEEP_WOOL_UNDERCOAT_TEXTURE_REF, SHEEP_WOOL_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: false,
            sheared: true,
            wool_color: SheepWoolColor::White,
            jeb: true,
            age_ticks: 25.0,
        }
        .vanilla_layer_texture_refs(),
        &[SHEEP_WOOL_UNDERCOAT_TEXTURE_REF]
    );
}

#[test]
fn sheep_textured_layer_passes_match_vanilla_renderer_layers() {
    let adult_red = sheep_textured_layer_passes(false, false, SheepWoolColor::Red, false, 0.0);
    assert_eq!(
        adult_red.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![
            EntityModelLayerKind::SheepBase,
            EntityModelLayerKind::SheepWool,
            EntityModelLayerKind::SheepWoolUndercoat,
        ]
    );
    assert_eq!(adult_red[0].model_layer, MODEL_LAYER_SHEEP);
    assert_eq!(adult_red[0].texture, SHEEP_TEXTURE_REF);
    assert_eq!(adult_red[0].parts, ADULT_SHEEP_TEXTURED_PARTS.as_slice());
    assert_eq!(adult_red[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (adult_red[0].collector_order, adult_red[0].submit_sequence),
        (0, 0)
    );
    assert_eq!(adult_red[1].model_layer, MODEL_LAYER_SHEEP_WOOL);
    assert_eq!(adult_red[1].texture, SHEEP_WOOL_TEXTURE_REF);
    assert_eq!(
        adult_red[1].parts,
        ADULT_SHEEP_WOOL_TEXTURED_PARTS.as_slice()
    );
    assert_eq!(
        adult_red[1].tint,
        sheep_wool_layer_color(SheepWoolColor::Red)
    );
    assert_eq!(
        (adult_red[1].collector_order, adult_red[1].submit_sequence),
        (0, 2)
    );
    assert_eq!(adult_red[2].model_layer, MODEL_LAYER_SHEEP_WOOL_UNDERCOAT);
    assert_eq!(adult_red[2].texture, SHEEP_WOOL_UNDERCOAT_TEXTURE_REF);
    assert_eq!(adult_red[2].parts, ADULT_SHEEP_TEXTURED_PARTS.as_slice());
    assert_eq!(
        adult_red[2].tint,
        sheep_wool_layer_color(SheepWoolColor::Red)
    );
    assert_eq!(
        (adult_red[2].collector_order, adult_red[2].submit_sequence),
        (1, 1)
    );

    let sheared_red = sheep_textured_layer_passes(false, true, SheepWoolColor::Red, false, 0.0);
    assert_eq!(
        sheared_red.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![
            EntityModelLayerKind::SheepBase,
            EntityModelLayerKind::SheepWoolUndercoat,
        ]
    );
    let sheared_white = sheep_textured_layer_passes(false, true, SheepWoolColor::White, false, 0.0);
    assert_eq!(sheared_white.len(), 1);
    assert_eq!(sheared_white[0].kind, EntityModelLayerKind::SheepBase);

    let adult_jeb_white =
        sheep_textured_layer_passes(false, false, SheepWoolColor::White, true, 12.5);
    assert_eq!(
        adult_jeb_white
            .iter()
            .map(|pass| pass.kind)
            .collect::<Vec<_>>(),
        vec![
            EntityModelLayerKind::SheepBase,
            EntityModelLayerKind::SheepWool,
            EntityModelLayerKind::SheepWoolUndercoat,
        ]
    );
    assert_eq!(adult_jeb_white[1].tint, sheep_jeb_wool_layer_color(12.5));
    assert_eq!(adult_jeb_white[2].tint, sheep_jeb_wool_layer_color(12.5));

    let baby_black = sheep_textured_layer_passes(true, false, SheepWoolColor::Black, false, 0.0);
    assert_eq!(
        baby_black
            .iter()
            .map(|pass| (
                pass.kind,
                pass.model_layer,
                pass.texture,
                pass.collector_order
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                EntityModelLayerKind::SheepBase,
                MODEL_LAYER_SHEEP_BABY,
                SHEEP_BABY_TEXTURE_REF,
                0,
            ),
            (
                EntityModelLayerKind::SheepWool,
                MODEL_LAYER_SHEEP_BABY_WOOL,
                SHEEP_WOOL_BABY_TEXTURE_REF,
                1,
            ),
        ]
    );
    assert_eq!(baby_black[1].parts, BABY_SHEEP_TEXTURED_PARTS.as_slice());
    let sheared_baby_black =
        sheep_textured_layer_passes(true, true, SheepWoolColor::Black, false, 0.0);
    assert_eq!(sheared_baby_black.len(), 1);
    let jeb_baby_white =
        sheep_textured_layer_passes(true, false, SheepWoolColor::White, true, 25.0);
    assert_eq!(
        jeb_baby_white
            .iter()
            .map(|pass| pass.kind)
            .collect::<Vec<_>>(),
        vec![
            EntityModelLayerKind::SheepBase,
            EntityModelLayerKind::SheepWool
        ]
    );
    assert_eq!(jeb_baby_white[1].tint, sheep_jeb_wool_layer_color(25.0));
}

#[test]
fn sheep_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_SHEEP, "minecraft:sheep#main");
    assert_eq!(MODEL_LAYER_SHEEP_BABY, "minecraft:sheep_baby#main");
    assert_eq!(MODEL_LAYER_SHEEP_WOOL, "minecraft:sheep#wool");
    assert_eq!(MODEL_LAYER_SHEEP_BABY_WOOL, "minecraft:sheep_baby#wool");
    assert_eq!(
        MODEL_LAYER_SHEEP_WOOL_UNDERCOAT,
        "minecraft:sheep#wool_undercoat"
    );
    assert_eq!(
        ADULT_SHEEP_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-3.0, -4.0, -6.0],
            size: [6.0, 6.0, 8.0],
            uv_size: [6.0, 6.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_SHEEP_WOOL_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-3.6, -4.6, -4.6],
            size: [7.2, 7.2, 7.2],
            uv_size: [6.0, 6.0, 6.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_SHEEP_WOOL_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-5.75, -11.75, -8.75],
            size: [11.5, 19.5, 9.5],
            uv_size: [8.0, 16.0, 6.0],
            tex: [28.0, 8.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_SHEEP_TEXTURED_LEFT_FRONT_LEG[0],
        TexturedModelCubeDesc {
            min: [-1.0, 0.0, -1.0],
            size: [2.0, 5.0, 2.0],
            uv_size: [2.0, 5.0, 2.0],
            tex: [24.0, 5.0],
            mirror: false,
        }
    );
}

#[test]
fn entity_texture_atlas_stitches_official_sheep_png_slots() {
    let images = sheep_entity_texture_refs()
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
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::sheep_wool(
            301,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            SheepWoolColor::Red,
        )],
        &atlas,
    );

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

    let sheared = entity_model_textured_mesh(
        &[EntityModelInstance::sheep_wool(
            302,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            SheepWoolColor::Red,
        )],
        &atlas,
    );
    assert_eq!(sheared.cutout_faces, 72);
    assert_eq!(sheared.vertices.len(), 288);

    let sheared_baby = entity_model_textured_mesh(
        &[EntityModelInstance::sheep_wool(
            303,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            true,
            SheepWoolColor::Black,
        )],
        &atlas,
    );
    assert_eq!(sheared_baby.cutout_faces, 36);
    assert!(sheared_baby
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let jeb_white = entity_model_textured_mesh(
        &[EntityModelInstance::sheep_render_state(
            304,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            SheepWoolColor::White,
            true,
            12.5,
        )],
        &atlas,
    );
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
}

fn sheep_texture_images() -> Vec<EntityModelTextureImage> {
    sheep_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
