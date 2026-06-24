use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn sheep_adult_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`.
    assert_eq!(
        ADULT_SHEEP_HEAD[0],
        ModelCube::new(
            [-3.0, -4.0, -6.0],
            [6.0, 6.0, 8.0],
            SHEEP_WOOL,
            [6.0, 6.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(ADULT_SHEEP_BODY[0].size, [8.0, 16.0, 6.0]);
    assert_eq!(ADULT_SHEEP_BODY[0].tex, [28.0, 8.0]);
    assert_eq!(ADULT_SHEEP_LEG[0].size, [4.0, 12.0, 4.0]);
    assert_eq!(ADULT_SHEEP_LEG[0].tex, [0.0, 16.0]);
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
    // The wool cubes inflate the colored geometry while the textured `uv_size` keeps the base box.
    assert_eq!(
        ADULT_SHEEP_WOOL_HEAD[0],
        ModelCube::new(
            [-3.6, -4.6, -4.6],
            [7.2, 7.2, 7.2],
            SHEEP_WOOL,
            [6.0, 6.0, 6.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(
        ADULT_SHEEP_WOOL_BODY[0],
        ModelCube::new(
            [-5.75, -11.75, -8.75],
            [11.5, 19.5, 9.5],
            SHEEP_WOOL,
            [8.0, 16.0, 6.0],
            [28.0, 8.0],
            false,
        )
    );
    assert_eq!(
        ADULT_SHEEP_WOOL_LEG[0],
        ModelCube::new(
            [-2.5, -0.5, -2.5],
            [5.0, 7.0, 5.0],
            SHEEP_WOOL,
            [4.0, 6.0, 4.0],
            [0.0, 16.0],
            false,
        )
    );
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
        ModelCube::new(
            [-3.0, -2.0, -4.5],
            [6.0, 4.0, 9.0],
            SHEEP_WOOL,
            [6.0, 4.0, 9.0],
            [0.0, 10.0],
            false,
        )
    );
    assert_eq!(BABY_SHEEP_HEAD[0].size, [5.0, 5.0, 5.0]);
    // The baby legs share one geometry but distinct per-corner UV origins.
    assert_eq!(BABY_SHEEP_RIGHT_HIND_LEG[0].tex, [0.0, 23.0]);
    assert_eq!(BABY_SHEEP_LEFT_HIND_LEG[0].tex, [24.0, 12.0]);
    assert_eq!(BABY_SHEEP_RIGHT_FRONT_LEG[0].tex, [8.0, 23.0]);
    assert_eq!(BABY_SHEEP_LEFT_FRONT_LEG[0].tex, [24.0, 5.0]);
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
    assert_eq!(adult_red[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (adult_red[0].collector_order, adult_red[0].submit_sequence),
        (0, 0)
    );
    assert_eq!(adult_red[1].model_layer, MODEL_LAYER_SHEEP_WOOL);
    assert_eq!(adult_red[1].texture, SHEEP_WOOL_TEXTURE_REF);
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
    // The textured UV sources now live on the unified cubes (`uv_size`/`tex`/`mirror`).
    assert_eq!(ADULT_SHEEP_HEAD[0].uv_size, [6.0, 6.0, 8.0]);
    assert_eq!(ADULT_SHEEP_HEAD[0].tex, [0.0, 0.0]);
    // The wool head/body inflate the colored geometry while the UV box stays the base size.
    assert_eq!(ADULT_SHEEP_WOOL_HEAD[0].size, [7.2, 7.2, 7.2]);
    assert_eq!(ADULT_SHEEP_WOOL_HEAD[0].uv_size, [6.0, 6.0, 6.0]);
    assert_eq!(ADULT_SHEEP_WOOL_BODY[0].size, [11.5, 19.5, 9.5]);
    assert_eq!(ADULT_SHEEP_WOOL_BODY[0].uv_size, [8.0, 16.0, 6.0]);
    assert_eq!(ADULT_SHEEP_WOOL_BODY[0].tex, [28.0, 8.0]);
    assert_eq!(BABY_SHEEP_LEFT_FRONT_LEG[0].uv_size, [2.0, 5.0, 2.0]);
    assert_eq!(BABY_SHEEP_LEFT_FRONT_LEG[0].tex, [24.0, 5.0]);
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
            false,
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

    // An invisible sheep renders nothing: the unified `render_state.invisible` skips the whole
    // model in both paths, so no body and no wool/undercoat layer is emitted.
    let invisible_red = entity_model_textured_mesh(
        &[EntityModelInstance::sheep_render_state(
            305,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            SheepWoolColor::Red,
            true,
            false,
            0.0,
        )],
        &atlas,
    );
    assert_eq!(invisible_red.cutout_faces, 0);
    assert!(invisible_red.vertices.is_empty());
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
    // Not eating: vanilla folds in the head-look pitch, which is not yet
    // projected, so the resting angle is 0.0.
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
        false,
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
        true,
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
    let looking = sheep_head_pose(adult_head, false, SheepHeadEatPose::NONE, 40.0, -18.0);
    assert_eq!(looking.offset, adult_head.offset);
    assert!((looking.rotation[0] - (-18.0_f32).to_radians()).abs() < 1e-6);
    assert!((looking.rotation[1] - 40.0_f32.to_radians()).abs() < 1e-6);
    assert_eq!(looking.rotation[2], adult_head.rotation[2]);

    // Eating wins the pitch (eat angle), but the look yaw still applies.
    let eating_and_looking = sheep_head_pose(
        adult_head,
        false,
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
        sheep_head_pose(adult_head, false, SheepHeadEatPose::NONE, 0.0, 0.0),
        adult_head
    );
}

#[test]
fn sheep_textured_mesh_applies_eating_head_pose_to_all_layers() {
    let (atlas, _) = build_entity_model_texture_atlas(&sheep_texture_images()).unwrap();
    let resting = entity_model_textured_mesh(
        &[EntityModelInstance::sheep_wool(
            401,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            SheepWoolColor::Red,
        )],
        &atlas,
    );
    // Tick 20: full head dip (positionScale 1.0) plus the plateau oscillation.
    let eating = entity_model_textured_mesh(
        &[EntityModelInstance::sheep_eating(
            401,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            SheepWoolColor::Red,
            20,
            0.0,
        )],
        &atlas,
    );

    // Base, wool, and undercoat each contribute 144 vertices; the head cube is
    // the first 24 vertices of each pass.
    assert_eq!(resting.vertices.len(), 432);
    assert_eq!(eating.vertices.len(), 432);
    for pass_start in [0usize, 144, 288] {
        let head = pass_start..pass_start + 24;
        let body_and_legs = pass_start + 24..pass_start + 144;
        assert_ne!(
            resting.vertices[head.clone()],
            eating.vertices[head.clone()],
            "head animates in pass at {pass_start}"
        );
        assert_eq!(
            resting.vertices[body_and_legs.clone()],
            eating.vertices[body_and_legs],
            "body and legs stay put in pass at {pass_start}"
        );
        // The root transform flips Y, so a larger model head.y dips the head down.
        assert!(
            average_textured_y(&eating.vertices[head.clone()])
                < average_textured_y(&resting.vertices[head]),
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
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

    assert_eq!(
        resting.vertices, still.vertices,
        "a standing sheep is inert"
    );
    assert_eq!(resting.vertices.len(), walking.vertices.len());
    // Base, wool, and undercoat each contribute 144 vertices; the head cube is the
    // first 24, the body and legs the remaining 120.
    for pass_start in [0usize, 144, 288] {
        let head = pass_start..pass_start + 24;
        let body_and_legs = pass_start + 24..pass_start + 144;
        assert_eq!(
            resting.vertices[head.clone()],
            walking.vertices[head],
            "head is unaffected by the leg swing in pass at {pass_start}"
        );
        assert_ne!(
            resting.vertices[body_and_legs.clone()],
            walking.vertices[body_and_legs],
            "legs swing in pass at {pass_start}"
        );
    }
}

#[test]
fn sheep_colored_mesh_swings_legs_when_walking() {
    // The colored sheep render (base + wool layers) swings its legs from the same
    // limb-swing projection: a standing sheep is inert, a walking sheep lifts its
    // feet (a shorter model) and splays its legs forward/back (a deeper footprint).
    for baby in [false, true] {
        let base = EntityModelInstance::sheep_wool(
            404,
            [0.0, 64.0, 0.0],
            0.0,
            baby,
            false,
            SheepWoolColor::Red,
        );
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "baby={baby}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "baby={baby}: walking differs"
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "baby={baby}: a walking sheep's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "baby={baby}: a walking sheep's legs should splay along Z"
        );
    }
}

#[test]
fn sheep_colored_mesh_applies_eating_head_pose() {
    let resting = entity_model_mesh(&[EntityModelInstance::sheep_wool(
        402,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
    )]);
    let eating = entity_model_mesh(&[EntityModelInstance::sheep_eating(
        402,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
        20,
        0.0,
    )]);

    assert_eq!(resting.vertices.len(), eating.vertices.len());
    assert_ne!(resting.vertices, eating.vertices);
    // The base body pass is emitted first; its head cube is the first 24 vertices.
    assert_ne!(resting.vertices[0..24], eating.vertices[0..24]);
    assert_eq!(resting.vertices[24..144], eating.vertices[24..144]);
}

#[test]
fn sheep_colored_mesh_applies_head_look_to_head_only() {
    let base = EntityModelInstance::sheep_wool(
        403,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        SheepWoolColor::Red,
    );
    let resting = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(60.0, 0.0)]);
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -30.0)]);

    // Head look turns the head cube (first 24 vertices) and leaves the body and
    // legs untouched.
    assert_ne!(resting.vertices[0..24], yawed.vertices[0..24]);
    assert_eq!(resting.vertices[24..144], yawed.vertices[24..144]);
    assert_ne!(resting.vertices[0..24], pitched.vertices[0..24]);
    assert_eq!(resting.vertices[24..144], pitched.vertices[24..144]);
    // Yaw and pitch are distinct head rotations.
    assert_ne!(yawed.vertices[0..24], pitched.vertices[0..24]);
}

fn average_textured_y(vertices: &[EntityModelTexturedVertex]) -> f32 {
    vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .sum::<f32>()
        / vertices.len() as f32
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
