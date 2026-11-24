use super::*;

pub(super) fn assert_close2(actual: [f32; 2], expected: [f32; 2]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-6);
    }
}

pub(super) fn test_hud_block_item_identity() -> crate::item_models::ItemModelAssetIdentity {
    crate::item_models::ItemModelAssetIdentity::new("minecraft:test_block", "default", 0)
}

pub(super) fn test_hud_block_item_model_with_quads(
    quads: Vec<crate::item_models::ItemModelQuad>,
    transform: glam::Mat4,
    lighting: GuiItemLightingEntry,
    foil: bool,
) -> HudBlockItemModel {
    let quads: Arc<[crate::item_models::ItemModelQuad]> = quads.into();
    let identity = test_hud_block_item_identity();
    let layers = [
        ItemModelGeometryClass::Solid,
        ItemModelGeometryClass::Translucent,
    ]
    .into_iter()
    .filter(|geometry_class| {
        let translucent = *geometry_class == ItemModelGeometryClass::Translucent;
        quads.iter().any(|quad| quad.translucent == translucent)
    })
    .map(|geometry_class| HudResidentItemLayer::Geometry {
        atlas: ItemModelAtlasKind::Blocks,
        identity: identity.clone(),
        geometry_class,
        source: ItemModelAssetSource::Quads(Arc::clone(&quads)),
        transform,
        tint: [1.0; 4],
    })
    .collect::<Vec<_>>();
    HudBlockItemModel {
        layers: layers.into(),
        lighting,
        foil,
    }
}

pub(super) fn test_hud_block_item_model() -> HudBlockItemModel {
    test_hud_block_item_model_with_quads(
        vec![crate::item_models::ItemModelQuad {
            corners: [
                [0.0, 0.0, 8.0],
                [16.0, 0.0, 8.0],
                [16.0, 16.0, 8.0],
                [0.0, 16.0, 8.0],
            ],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            tint: [1.0, 1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        }],
        glam::Mat4::IDENTITY,
        GuiItemLightingEntry::Items3d,
        false,
    )
}

pub(super) fn styled_test_glyphs() -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    for ch in ['a', 'b'] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: 6.0,
                width: 5,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs
}

pub(super) fn component_test_font_book() -> HudFontGlyphBook {
    let glyph = |advance, width, min_x| HudAsciiGlyph {
        uv: HudUvRect {
            min: [min_x, 0.0],
            max: [min_x + 0.05, 1.0],
        },
        advance: advance as f32,
        width,
        height: 8,
        ..HudAsciiGlyph::default()
    };
    let mut default = HudFontGlyphMap::new();
    default.insert_first_wins('a', glyph(6.0, 5, 0.1));
    default.insert_first_wins('b', glyph(6.0, 5, 0.2));
    let mut custom = HudFontGlyphMap::new();
    custom.insert_first_wins('a', glyph(10.0, 7, 0.6));
    custom.insert_first_wins('b', glyph(10.0, 7, 0.7));
    HudFontGlyphBook::from_fonts(
        "minecraft:default",
        std::collections::BTreeMap::from([
            ("example:wide".to_string(), custom),
            ("minecraft:default".to_string(), default),
        ]),
    )
}

/// Distinct-uv glyph map for obfuscated observability: three advance-6
/// glyphs ('a'/'b'/'c') plus one advance-4 glyph ('d').
pub(super) fn obfuscated_test_glyphs() -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, uv_min, advance) in [
        ('a', 0.1, 6u32),
        ('b', 0.2, 6),
        ('c', 0.3, 6),
        ('d', 0.4, 4),
    ] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [uv_min, 0.0],
                    max: [uv_min + 0.05, 0.1],
                },
                advance: advance as f32,
                width: 5,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs
}

pub(super) fn hud_entity_preview_for_test() -> HudEntityPreview {
    HudEntityPreview {
        entity: EntityModelInstance::chicken(90, [0.0, 64.0, 0.0], 180.0, false)
            .with(|s| s.light_coords = 0)
            .with_outline_color(0xff00_ff00),
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: 26,
            y: 8,
            width: 49,
            height: 70,
        },
        scissor: Some(HudEntityPreviewRect {
            x: 30,
            y: 20,
            width: 10,
            height: 12,
        }),
        translation: [0.0, 0.875, 0.0],
        rotation: [0.0, 0.0, 1.0, 0.0],
        override_camera_rotation: Some([0.125, 0.0, 0.0, 0.992_156_74]),
        scale: 30.0,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: vec![HudEntityPreviewItemLayer {
            slot: HudEntityPreviewItemSlot::LeftHand,
            display_context: HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
            item_id: 12,
            count: 1,
            foil: true,
            light_coords: 0,
            overlay: [1.0, 2.0],
            order: 0,
            submit_sequence: 1,
        }],
    }
}

pub(super) fn hud_inventory_screen_with_entity_previews(
    entity_previews: Vec<HudEntityPreview>,
) -> HudInventoryScreen {
    HudInventoryScreen {
        width: 176,
        height: 166,
        background_layers: Vec::new(),
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews,
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels: Vec::new(),
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}

pub(super) fn hud_inventory_beacon_test_renderer() -> Option<Renderer> {
    const WIDTH: u32 = 230;
    const HEIGHT: u32 = 219;

    let mut renderer = Renderer::new_offscreen(WIDTH, HEIGHT)?;
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.25,
        a: 1.0,
    });
    renderer.update_camera();
    renderer
        .upload_gui_texture(
            "textures/gui/container/beacon.png",
            1,
            1,
            &[0, 0, 0, 0],
            crate::StandaloneTextureSampling::default(),
        )
        .expect("transparent Beacon background");
    renderer
        .add_test_gui_sprite("container/beacon/button_disabled", 1, 1, &[255, 0, 0, 255])
        .expect("disabled Beacon button");
    renderer
        .add_test_gui_sprite("container/beacon/button_selected", 1, 1, &[0, 255, 0, 255])
        .expect("selected Beacon button");
    renderer
        .add_test_gui_sprite(
            "container/beacon/button_highlighted",
            1,
            1,
            &[255, 255, 0, 255],
        )
        .expect("highlighted Beacon button");
    renderer
        .add_test_gui_sprite("container/beacon/button", 1, 1, &[255, 0, 255, 255])
        .expect("normal Beacon button");
    renderer
        .add_test_gui_sprite("container/beacon/confirm", 1, 1, &[0, 255, 255, 255])
        .expect("Beacon confirm icon");
    renderer
        .add_test_gui_sprite("container/beacon/cancel", 1, 1, &[255, 255, 255, 255])
        .expect("Beacon cancel icon");
    // Effect ids 1 and 5 (`slowness`, `instant_health`).
    renderer
        .add_test_gui_sprite("mob_effect/slowness", 1, 1, &[0, 255, 255, 255])
        .expect("Beacon effect icon");
    renderer
        .add_test_gui_sprite("mob_effect/instant_health", 1, 1, &[255, 255, 255, 255])
        .expect("Beacon effect icon");

    let item_colors = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 255, 255, 255],
        [255, 0, 255, 255],
        [255, 255, 255, 255],
    ];
    let item_atlas = item_colors.into_iter().flatten().collect::<Vec<_>>();
    renderer
        .upload_item_atlas(5, 1, &item_atlas)
        .expect("Beacon payment item atlas");

    let background_layer =
        |texture: HudInventoryBackgroundTexture, x, y, width, height| HudInventoryBackgroundLayer {
            texture,
            x,
            y,
            width,
            height,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        };
    let floating_items = [(20, 109), (41, 109), (63, 109), (86, 109), (108, 109)]
        .into_iter()
        .enumerate()
        .map(|(index, (x, y))| HudInventoryItem {
            x,
            y,
            scale: 1.0,
            scale_y: 1.0,
            icon: HudItemIcon::single(HudUvRect {
                min: [index as f32 / 5.0, 0.0],
                max: [(index + 1) as f32 / 5.0, 1.0],
            }),
            scissor: None,
            draw_decorations: false,
            block_model: None,
        })
        .collect();
    renderer.set_hud_inventory_screen(Some(HudInventoryScreen {
        width: WIDTH,
        height: HEIGHT,
        background_layers: vec![
            background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/beacon.png"),
                0,
                0,
                WIDTH,
                HEIGHT,
            ),
            background_layer(
                HudInventoryBackgroundTexture::Sprite("container/beacon/button_disabled"),
                53,
                22,
                22,
                22,
            ),
            background_layer(HudInventoryBackgroundTexture::MobEffect(1), 55, 24, 18, 18),
            background_layer(
                HudInventoryBackgroundTexture::Sprite("container/beacon/button_selected"),
                65,
                72,
                22,
                22,
            ),
            background_layer(HudInventoryBackgroundTexture::MobEffect(5), 67, 74, 18, 18),
            background_layer(
                HudInventoryBackgroundTexture::Sprite("container/beacon/button_highlighted"),
                164,
                107,
                22,
                22,
            ),
            background_layer(
                HudInventoryBackgroundTexture::Sprite("container/beacon/confirm"),
                166,
                109,
                18,
                18,
            ),
            background_layer(
                HudInventoryBackgroundTexture::Sprite("container/beacon/button"),
                190,
                107,
                22,
                22,
            ),
            background_layer(
                HudInventoryBackgroundTexture::Sprite("container/beacon/cancel"),
                192,
                109,
                18,
                18,
            ),
        ],
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items,
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels: Vec::new(),
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }));
    Some(renderer)
}

pub(super) fn test_hud_mob_effect(
    effect_id: i32,
    beneficial: bool,
    color: u32,
    duration_ticks: i32,
    ambient: bool,
    show_icon: bool,
) -> HudMobEffect {
    HudMobEffect {
        effect_id,
        beneficial,
        color,
        amplifier: 0,
        duration_ticks,
        ambient,
        show_icon,
        display_name: format!("Effect {effect_id}"),
        duration_text: "00:10".to_string(),
    }
}

pub(super) fn hud_mob_effect_test_glyphs() -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    for character in ' '..='~' {
        glyphs.insert_first_wins(
            character,
            HudAsciiGlyph {
                advance: 6.0,
                width: 5,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs
}
