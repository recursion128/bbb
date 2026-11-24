use super::*;
use crate::{HudAsciiGlyph, HudChatInputView, HudCommandSuggestions, HudFontGlyphMap, HudUvRect};

fn viewport(renderer: &mut Renderer, width: u32, height: u32) {
    renderer.resize(PhysicalSize::new(width, height)).unwrap();
    renderer.set_gui_viewport(PhysicalSize::new(width.div_ceil(2), height.div_ceil(2)), 2);
}

#[test]
fn gui_projection_whole_frame_glyph_and_highlight_keep_two_physical_pixels_per_gui_pixel() {
    let Some(mut renderer) = Renderer::new_offscreen(801, 603) else {
        return;
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(' ', HudAsciiGlyph::empty(296.0));
    glyphs.insert_first_wins(
        'A',
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.0; 2],
                max: [1.0; 2],
            },
            width: 2,
            height: 2,
            advance: 3.0,
            ..Default::default()
        },
    );
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], glyphs)
        .unwrap();
    for (width, height) in [(801, 603), (803, 605), (804, 606), (800, 600)] {
        viewport(&mut renderer, width, height);
        let mut input = HudCommandSuggestions {
            input_view: Some(HudChatInputView {
                value: vec![32, 65],
                cursor: 2,
                selection: 2,
                display_start: 0,
            }),
            ..Default::default()
        };
        renderer.set_hud_command_suggestions(Some(input.clone()));
        let plain = renderer.render_offscreen_frame().unwrap();
        let y = (height.div_ceil(2) - 12) * 2;
        for py in y..y + 4 {
            for px in 600..604 {
                assert!(
                    plain.pixel(px, py)[..3].iter().all(|v| *v > 180),
                    "glyph ({px},{py}) at {width}x{height}: {:?}",
                    plain.pixel(px, py)
                );
            }
        }
        for (x, y) in [(599, y), (604, y), (600, y - 1), (600, y + 4)] {
            assert!(
                plain.pixel(x, y)[0] < 100,
                "outside glyph at {width}x{height} ({x},{y}): {:?}",
                plain.pixel(x, y)
            );
        }
        input.input_view.as_mut().unwrap().cursor = 1;
        renderer.set_hud_command_suggestions(Some(input));
        let selected = renderer.render_offscreen_frame().unwrap();
        for py in y..y + 4 {
            for px in 600..604 {
                let pixel = selected.pixel(px, py);
                assert!(
                    pixel[0] < 40 && pixel[1] < 40 && pixel[2] > 180,
                    "highlight glyph ({px},{py}) at {width}x{height}: {pixel:?}"
                );
            }
        }
    }
}

fn map_screen() -> crate::HudInventoryScreen {
    crate::HudInventoryScreen {
        width: 1,
        height: 1,
        background_layers: vec![crate::HudInventoryBackgroundLayer {
            texture: crate::HudInventoryBackgroundTexture::Sprite(
                "container/cartography_table/map",
            ),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            uv: HudUvRect {
                min: [0.0; 2],
                max: [1.0; 2],
            },
        }],
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: vec![crate::HudInventoryMapPreview {
            map_id: 7,
            x: 0,
            y: 0,
            scale: 1.0,
            after_background_layer: 0,
        }],
        banner_pattern_previews: Vec::new(),
        text_labels: Vec::new(),
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}

#[test]
fn gui_projection_map_and_text_consume_actual_gui_item_camera_across_odd_and_equal_ceil_resize() {
    let Some(mut renderer) = Renderer::new_offscreen(801, 603) else {
        return;
    };
    renderer
        .add_test_gui_sprite("container/cartography_table/map", 1, 1, &[0, 0, 255, 255])
        .unwrap();
    renderer.set_hud_inventory_screen(Some(map_screen()));
    let surface = crate::item_models::bake_item_frame_map_surface(
        7,
        glam::Mat4::from_translation(glam::Vec3::new(300.0, 200.0, 0.0))
            * glam::Mat4::from_scale(glam::Vec3::new(1.0 / 128.0, 1.0 / 128.0, 1.0)),
        crate::item_models::ITEM_MODEL_FULL_BRIGHT_LIGHT,
    );
    renderer.set_map_surfaces_with_gui(
        Vec::new(),
        Vec::new(),
        vec![crate::item_models::ItemFrameMapTexture {
            map_id: 7,
            owner_generation: 1,
            content_revision: 1,
            rgba: std::sync::Arc::from([255, 0, 0, 255].repeat(128 * 128)),
            patch: None,
        }],
        Vec::new(),
        Vec::new(),
        vec![crate::item_models::HudMapPreviewSurface {
            preview_index: 0,
            surface,
        }],
    );
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'A',
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.0; 2],
                max: [1.0; 2],
            },
            width: 3,
            height: 3,
            advance: 3.0,
            ..Default::default()
        },
    );
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], renderer.pack.hud_font_book.clone())
        .unwrap();
    let text = crate::item_models::bake_hud_map_text_surface(
        1,
        &[crate::WorldTextRun::plain("A")],
        0,
        0,
        0,
        glam::Mat4::from_translation(glam::Vec3::new(247.0, 132.0, 0.0)),
        crate::item_models::ITEM_MODEL_FULL_BRIGHT_LIGHT,
        0,
        &crate::HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([("minecraft:default".into(), glyphs)]),
        ),
        0,
    )
    .unwrap();
    renderer.set_map_text_surfaces_with_gui(
        Vec::new(),
        Vec::new(),
        vec![crate::item_models::HudMapPreviewSurface {
            preview_index: 0,
            surface: text,
        }],
    );
    for (width, height) in [(801, 603), (802, 604), (803, 605), (800, 600)] {
        viewport(&mut renderer, width, height);
        let frame = renderer.render_offscreen_frame().unwrap();
        for y in 399..405 {
            for x in 619..625 {
                let pixel = frame.pixel(x, y);
                let white = pixel[..3].iter().all(|v| *v > 180);
                assert_eq!(
                    white,
                    (620..624).contains(&x) && (400..404).contains(&y),
                    "map text ({x},{y}) at {width}x{height}: {pixel:?}"
                );
            }
        }
        for y in 399..403 {
            for x in 599..603 {
                let pixel = frame.pixel(x, y);
                let red = pixel[0] > 128 && pixel[1] < 60 && pixel[2] < 60;
                assert_eq!(
                    red,
                    (600..602).contains(&x) && (400..402).contains(&y),
                    "map ({x},{y}) at {width}x{height}: {pixel:?}"
                );
            }
        }
    }
}

#[test]
fn unchanged_hud_frames_reuse_retained_geometry_until_mutated_or_frame_varying() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'A',
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.0; 2],
                max: [1.0; 2],
            },
            width: 5,
            height: 7,
            advance: 6.0,
            ..Default::default()
        },
    );
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], glyphs)
        .unwrap();
    renderer
        .add_test_gui_sprite("hud/hotbar", 182, 22, &vec![40; 182 * 22 * 4])
        .unwrap();
    renderer
        .add_test_gui_sprite("hud/hotbar_selection", 24, 23, &vec![220; 24 * 23 * 4])
        .unwrap();
    renderer.set_hud_food(Some(20));
    renderer.set_hud_selected_slot(1);

    let first = renderer.render_offscreen_frame().unwrap();
    assert_eq!(renderer.hud_geometry.rebuild_count, 1);
    assert!(!renderer.hud_geometry.frame_varying);

    // Re-applying identical HUD values on a non-tick frame reuses the retained buffer and commands.
    renderer.set_hud_food(Some(20));
    renderer.set_hud_selected_slot(1);
    let second = renderer.render_offscreen_frame().unwrap();
    assert_eq!(renderer.hud_geometry.rebuild_count, 1);
    assert_eq!(first.rgba, second.rgba);

    // Mutating HUD state invalidates and rebuilds once.
    renderer.set_hud_selected_slot(4);
    let third = renderer.render_offscreen_frame().unwrap();
    assert_eq!(renderer.hud_geometry.rebuild_count, 2);
    assert_ne!(first.rgba, third.rgba);

    // Frame-varying obfuscated text opts out of cross-frame reuse while active.
    let mut obf = crate::HudStyledTextRun::plain("A");
    obf.style.obfuscated = true;
    renderer.set_hud_selected_item_name(Some(crate::HudSelectedItemName {
        runs: vec![obf.into()],
        highlight_ticks: 40,
        raised: false,
    }));
    renderer.render_offscreen_frame().unwrap();
    assert_eq!(renderer.hud_geometry.rebuild_count, 3);
    assert!(renderer.hud_geometry.frame_varying);
    renderer.render_offscreen_frame().unwrap();
    assert_eq!(renderer.hud_geometry.rebuild_count, 4);
}
