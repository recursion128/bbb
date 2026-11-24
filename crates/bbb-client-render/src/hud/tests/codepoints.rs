use super::*;

#[test]
fn surrogate_obfuscated_glyph_reaches_the_frame_and_reloads_with_the_font_book() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.update_camera();
    let missing = HudAsciiGlyph {
        advance: 5.0,
        width: 4,
        height: 4,
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [0.5, 1.0],
        },
        ..Default::default()
    };
    let mut map = HudFontGlyphMap::new();
    map.set_missing_glyph(missing);
    map.insert_codepoint_first_wins(
        0xd800,
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.5, 0.0],
                max: [1.0, 1.0],
            },
            ..missing
        },
    );
    let pixels = [255, 0, 0, 255, 0, 255, 0, 255];
    renderer
        .upload_hud_font_atlas(2, 1, &pixels, HudFontGlyphBook::from(map))
        .unwrap();
    let mut screen = hud_inventory_screen_with_entity_previews(vec![]);
    screen.width = 320;
    screen.height = 240;
    screen.text_labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 10,
        y: 10,
        width: 100,
        text: "A".into(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: vec![HudStyledTextRun::plain("A")],
    });
    renderer.set_hud_inventory_screen(Some(screen.clone()));
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(11, 11),
        [255, 0, 0, 255]
    );
    screen.text_labels[0].runs[0].style.obfuscated = true;
    renderer.set_hud_inventory_screen(Some(screen));
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(11, 11),
        [0, 255, 0, 255]
    );
    let mut reloaded = HudFontGlyphMap::new();
    reloaded.set_missing_glyph(missing);
    renderer
        .upload_hud_font_atlas(2, 1, &pixels, HudFontGlyphBook::from(reloaded))
        .unwrap();
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(11, 11),
        [255, 0, 0, 255]
    );
}
