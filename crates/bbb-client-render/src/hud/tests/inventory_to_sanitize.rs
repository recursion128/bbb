use super::*;
use bbb_protocol::game_types::GameType;

#[test]
fn recipe_toast_sanitizes_text_and_uses_vanilla_layout() {
    let toast = sanitize_hud_recipe_toast(HudRecipeToast {
        title: "New\u{0} Recipe(s) Unlocked!".to_string(),
        description: "Check your recipe book".to_string(),
        category_icon: None,
        category_block_model: None,
        unlocked_icon: None,
        unlocked_block_model: None,
        slot: 2,
        visible_portion: 0.25,
    })
    .unwrap();
    assert_eq!(toast.title, "New Recipe(s) Unlocked!");
    assert_eq!(toast.visible_portion, 0.25);
    assert_eq!(
        hud_recipe_toast_rect(PhysicalSize::new(320, 240), &toast),
        absolute_hud_rect(280.0, 64.0, 160, 32)
    );
    assert_eq!(
        hud_recipe_toast_category_icon_rect(PhysicalSize::new(320, 240), &toast),
        absolute_hud_rect(282.0, 66.0, 10, 10)
    );
    assert_eq!(
        hud_recipe_toast_unlocked_icon_rect(PhysicalSize::new(320, 240), &toast),
        absolute_hud_rect(288.0, 72.0, 16, 16)
    );
    assert!(sanitize_hud_recipe_toast(HudRecipeToast { slot: 5, ..toast }).is_none());
}

#[test]
fn locator_style_selects_vanilla_distance_buckets() {
    let style = HudLocatorStyle {
        near_distance: 128,
        far_distance: 332,
        sprite_ids: (0..4).map(|index| format!("dot/{index}")).collect(),
    };
    assert_eq!(style.sprite_id_for_distance(127.99), Some("dot/0"));
    assert_eq!(style.sprite_id_for_distance(128.0), Some("dot/1"));
    assert_eq!(style.sprite_id_for_distance(230.0), Some("dot/2"));
    assert_eq!(style.sprite_id_for_distance(332.0), Some("dot/3"));
    assert_eq!(
        HudLocatorStyle {
            near_distance: 64,
            far_distance: 65,
            sprite_ids: vec!["bowtie".to_string()],
        }
        .sprite_id_for_distance(f32::INFINITY),
        Some("bowtie")
    );
}

#[test]
fn sanitize_hud_debug_overlay_strips_control_chars_and_drops_empty_overlay() {
    assert_eq!(sanitize_hud_debug_overlay(HudDebugOverlay::default()), None);

    let overlay = sanitize_hud_debug_overlay(HudDebugOverlay {
        left_lines: vec!["A\u{0007}B".to_string(), "".to_string()],
        right_lines: vec!["Right".to_string()],
        debug_crosshair: None,
        game_mode_switcher: None,
        profiler_chart: None,
        fps_chart: None,
        tps_chart: None,
        network_charts: None,
        show_lightmap_preview: false,
    })
    .expect("non-empty debug overlay survives sanitize");

    assert_eq!(overlay.left_lines, vec!["AB".to_string(), "".to_string()]);
    assert_eq!(overlay.right_lines, vec!["Right".to_string()]);
    assert!(!overlay.show_lightmap_preview);

    let preview_only = sanitize_hud_debug_overlay(HudDebugOverlay {
        show_lightmap_preview: true,
        ..HudDebugOverlay::default()
    })
    .expect("lightmap preview survives without text lines");
    assert!(preview_only.show_lightmap_preview);
}

#[test]
fn sanitize_hud_debug_overlay_keeps_game_mode_switcher_without_text_lines() {
    let overlay = sanitize_hud_debug_overlay(HudDebugOverlay {
        game_mode_switcher: Some(HudDebugGameModeSwitcher {
            selected: GameType::Creative,
            title: "Creative\u{0007} Mode".to_string(),
            help_runs: vec![HudStyledTextRun::plain("F4 Next")],
            background_x: 98,
            background_y: 62,
            background_width: 125,
            background_height: 75,
            slots: vec![
                HudDebugGameModeSwitcherSlot {
                    mode: GameType::Creative,
                    x: 101,
                    y: 89,
                    width: 26,
                    height: 26,
                    selected: true,
                    icon: None,
                    block_model: None,
                },
                HudDebugGameModeSwitcherSlot {
                    mode: GameType::Survival,
                    x: 132,
                    y: 89,
                    width: 26,
                    height: 26,
                    selected: false,
                    icon: None,
                    block_model: None,
                },
                HudDebugGameModeSwitcherSlot {
                    mode: GameType::Adventure,
                    x: 163,
                    y: 89,
                    width: 26,
                    height: 26,
                    selected: false,
                    icon: None,
                    block_model: None,
                },
                HudDebugGameModeSwitcherSlot {
                    mode: GameType::Spectator,
                    x: 194,
                    y: 89,
                    width: 26,
                    height: 26,
                    selected: false,
                    icon: None,
                    block_model: None,
                },
            ],
        }),
        ..HudDebugOverlay::default()
    })
    .expect("game mode switcher survives without text lines");
    let switcher = overlay
        .game_mode_switcher
        .expect("switcher should remain after sanitize");

    assert_eq!(switcher.title, "Creative Mode");
    assert_eq!(switcher.slots.len(), 4);
}

#[test]
fn sanitize_hud_debug_overlay_sanitizes_game_mode_switcher_icons() {
    let valid_icon = HudItemIcon::single(HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    });
    let invalid_icon = HudItemIcon {
        lighting: GuiItemLightingEntry::Items3d,
        ..valid_icon.clone()
    };
    let empty_block_model = test_hud_block_item_model_with_quads(
        Vec::new(),
        glam::Mat4::IDENTITY,
        GuiItemLightingEntry::Items3d,
        false,
    );
    let slot = |mode, x, icon, block_model| HudDebugGameModeSwitcherSlot {
        mode,
        x,
        y: 89,
        width: 26,
        height: 26,
        selected: mode == GameType::Creative,
        icon,
        block_model,
    };
    let overlay = sanitize_hud_debug_overlay(HudDebugOverlay {
        game_mode_switcher: Some(HudDebugGameModeSwitcher {
            selected: GameType::Creative,
            title: "Creative Mode".to_string(),
            help_runs: vec![HudStyledTextRun::plain("F4 Next")],
            background_x: 98,
            background_y: 62,
            background_width: 125,
            background_height: 75,
            slots: vec![
                slot(
                    GameType::Creative,
                    101,
                    Some(valid_icon),
                    Some(empty_block_model),
                ),
                slot(GameType::Survival, 132, Some(invalid_icon), None),
                slot(GameType::Adventure, 163, None, None),
                slot(GameType::Spectator, 194, None, None),
            ],
        }),
        ..HudDebugOverlay::default()
    })
    .expect("game mode switcher with sanitized icons survives");
    let slots = overlay.game_mode_switcher.unwrap().slots;

    assert!(slots[0].icon.is_some());
    assert!(slots[0].block_model.is_none());
    assert!(slots[1].icon.is_none());
}

#[test]
fn jigsaw_block_editor_layout_matches_vanilla_absolute_geometry() {
    let compact = hud_jigsaw_block_editor_layout(PhysicalSize::new(320, 240));
    assert_eq!(
        compact.fields[0],
        HudCommandBlockEditorRect {
            x: 7,
            y: 20,
            width: 300,
            height: 20,
        }
    );
    assert_eq!(
        compact.fields.map(|rect| rect.y),
        [20, 55, 90, 125, 160, 160]
    );
    assert_eq!(compact.fields[4].x, 7);
    assert_eq!(compact.fields[5].x, 110);
    assert_eq!(compact.joint.x, 214);
    assert_eq!(compact.levels.x, 6);
    assert_eq!(compact.keep_jigsaws.x, 110);
    assert_eq!(compact.generate.x, 214);
    assert_eq!(compact.done.x, 6);
    assert_eq!(compact.cancel.x, 164);

    let wide = hud_jigsaw_block_editor_layout(PhysicalSize::new(854, 480));
    assert_eq!(wide.fields[0].x, 274);
    assert_eq!(wide.fields[5].x, 377);
    assert_eq!(wide.joint.x, 481);
    assert_eq!(wide.levels.x, 273);
    assert_eq!(wide.done.x, 273);
    assert_eq!(wide.cancel.x, 431);
}

#[test]
fn jigsaw_block_editor_sanitizes_field_slider_and_tooltip_bounds() {
    let field = || HudTextField {
        text: "😀".repeat(200),
        cursor: usize::MAX,
        selection: usize::MAX,
        scroll_to: usize::MAX,
        max_length: usize::MAX,
        ..HudTextField::default()
    };
    let screen = sanitize_hud_jigsaw_block_editor_screen(HudJigsawBlockEditorScreen {
        labels: std::array::from_fn(|_| "label".to_string()),
        joint_label: "joint".to_string(),
        fields: std::array::from_fn(|_| field()),
        joint_visible: true,
        joint: "rollable".to_string(),
        levels: "Levels: 0".to_string(),
        slider_value_bits: f64::NAN.to_bits(),
        keep_jigsaws: "Keep Jigsaws: ON".to_string(),
        generate: "Generate".to_string(),
        done: "Done".to_string(),
        cancel: "Cancel".to_string(),
        controls_enabled: true,
        joint_hovered: false,
        levels_hovered: false,
        keep_jigsaws_hovered: false,
        generate_hovered: false,
        done_hovered: false,
        cancel_hovered: false,
        tooltip: Some(HudDebugOptionsTooltip {
            text: "first\nsecond\u{7}".to_string(),
            x: 1,
            y: 2,
        }),
    })
    .unwrap();
    assert_eq!(screen.fields[0].text.encode_utf16().count(), 128);
    assert_eq!(screen.fields[3].text.encode_utf16().count(), 256);
    assert_eq!(screen.fields[4].text.encode_utf16().count(), 2);
    assert_eq!(screen.slider_value_bits, 0.0_f64.to_bits());
    assert_eq!(screen.tooltip.unwrap().text, "first\nsecond");
}

#[test]
fn sanitize_hud_debug_overlay_keeps_valid_crosshair_and_drops_invalid() {
    let crosshair_only = sanitize_hud_debug_overlay(HudDebugOverlay {
        debug_crosshair: Some(HudDebugCrosshair {
            x_rot_degrees: 15.0,
            y_rot_degrees: 90.0,
            gui_scale: 0,
        }),
        ..HudDebugOverlay::default()
    })
    .expect("3d crosshair survives without text lines");

    assert_eq!(
        crosshair_only.debug_crosshair,
        Some(HudDebugCrosshair {
            x_rot_degrees: 15.0,
            y_rot_degrees: 90.0,
            gui_scale: 1,
        })
    );

    assert_eq!(
        sanitize_hud_debug_overlay(HudDebugOverlay {
            debug_crosshair: Some(HudDebugCrosshair {
                x_rot_degrees: f32::NAN,
                y_rot_degrees: 0.0,
                gui_scale: 1,
            }),
            ..HudDebugOverlay::default()
        }),
        None
    );
}

#[test]
fn sanitize_hud_item_icon_preserves_layer_order_and_clamps_tint() {
    let first = HudIconLayer::new(
        HudUvRect {
            min: [0.0, 0.0],
            max: [0.25, 0.25],
        },
        [-1.0, 0.25, 1.5, 1.0],
    );
    let second = HudIconLayer::new(
        HudUvRect {
            min: [0.25, 0.25],
            max: [0.5, 0.5],
        },
        [0.75, 0.5, 0.25, 0.0],
    );
    let icon = sanitize_hud_item_icon(HudItemIcon {
        pop: 0.0,
        lighting: GuiItemLightingEntry::ItemsFlat,
        layers: vec![first, second],
        foil: HudItemFoil::Standard,
        count_label: Some(HudItemCountLabel::new("64")),
        durability_bar: Some(HudItemDurabilityBar::new(99, [-1.0, 0.5, 1.5])),
        cooldown_progress: Some(1.5),
    })
    .expect("valid icon layers should remain");

    assert_eq!(icon.layers.len(), 2);
    assert_eq!(icon.foil, HudItemFoil::Standard);
    assert_eq!(icon.count_label, Some(HudItemCountLabel::new("64")));
    assert_eq!(
        icon.durability_bar,
        Some(HudItemDurabilityBar::new(13, [0.0, 0.5, 1.0]))
    );
    assert_eq!(icon.cooldown_progress, Some(1.0));
    assert_eq!(icon.layers[0].uv.min, [0.0, 0.0]);
    assert_eq!(icon.layers[0].uv.max, [0.25, 0.25]);
    assert_eq!(icon.layers[0].tint, [0.0, 0.25, 1.0, 1.0]);
    assert_eq!(icon.layers[1].uv.min, [0.25, 0.25]);
    assert_eq!(icon.layers[1].uv.max, [0.5, 0.5]);
    assert_eq!(icon.layers[1].tint, [0.75, 0.5, 0.25, 0.0]);
}

#[test]
fn player_list_plain_wrapping_matches_projection_contract() {
    let glyphs = styled_test_glyphs();
    assert_eq!(hud_wrap_plain_text("abab", 12, &glyphs), ["ab", "ab"]);
    assert_eq!(hud_wrap_plain_text("ab\nab", 12, &glyphs), ["ab", "ab"]);
}

#[test]
fn plain_text_cursor_for_width_uses_loaded_glyph_advances() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('i', 2), ('w', 7), ('x', 6)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: advance as f32,
                width: advance,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
    }

    assert_eq!(hud_plain_text_cursor_for_width("iwx", 0, &glyphs), Some(0));
    assert_eq!(hud_plain_text_cursor_for_width("iwx", 2, &glyphs), Some(1));
    assert_eq!(hud_plain_text_cursor_for_width("iwx", 8, &glyphs), Some(1));
    assert_eq!(hud_plain_text_cursor_for_width("iwx", 9, &glyphs), Some(2));
    assert_eq!(hud_plain_text_cursor_for_width("iwx", 15, &glyphs), Some(3));
    assert_eq!(
        hud_plain_text_cursor_for_width_from("iwx", 1, 7, &glyphs),
        Some(2)
    );
    assert_eq!(
        hud_plain_text_cursor_for_width("iwx", 15, &HudFontGlyphMap::new()),
        None
    );
}

#[test]
fn multiline_text_wrap_preserves_break_boundaries_and_trailing_empty_line() {
    let mut glyphs = HudFontGlyphMap::new();
    for ch in ['a', 'b', 'c', 'd', 'x', ' '] {
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
    assert_eq!(
        hud_multiline_lines("ab cd\nx\n", 12, &glyphs),
        [
            HudMultilineLine { begin: 0, end: 2 },
            HudMultilineLine { begin: 3, end: 5 },
            HudMultilineLine { begin: 6, end: 7 },
            HudMultilineLine { begin: 8, end: 8 },
        ]
    );

    let area = sanitize_hud_inventory_text_area(HudInventoryTextArea {
        needs_cursor_position: false,
        x: 0,
        y: 0,
        width: 44,
        height: 17,
        text: "a\t\n😀b".to_string(),
        cursor: usize::MAX,
        selection: usize::MAX,
        scroll_amount: -5,
        max_length: 4,
        cursor_visible: true,
        text_tint: [1.0; 4],
        cursor_tint: [1.0; 4],
        counter_tint: [0.5; 4],
        shadow: true,
    })
    .unwrap();
    assert_eq!(area.text, "a\t\n😀b");
    assert_eq!(area.cursor, 6);
    assert_eq!(area.selection, 6);
    assert_eq!(area.scroll_amount, 0);
}

#[test]
fn plain_text_display_start_for_width_matches_text_input_layout() {
    let glyphs = styled_test_glyphs();

    assert_eq!(
        hud_plain_text_display_start_for_width("aaaaa", 5, 12, &glyphs),
        Some(3)
    );
    assert_eq!(
        hud_plain_text_cursor_for_width_from("aaaaa", 3, 12, &glyphs),
        Some(5)
    );
    assert_eq!(
        hud_plain_text_display_start_for_width("aaaaa", 5, 12, &HudFontGlyphMap::new()),
        None
    );
}

#[test]
fn logical_text_geometry_keeps_editable_neutral_lines_in_buffer_order() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, uv_x) in [('1', 0.1), ('2', 0.2), ('3', 0.3), ('-', 0.4)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [uv_x, 0.0],
                    max: [uv_x + 0.05, 1.0],
                },
                width: 5,
                height: 8,
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs.set_default_right_to_left(true);

    let geometry = hud_logical_styled_text_geometry(
        &[HudStyledTextRun::plain("123-")],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(
        geometry
            .glyph_quads
            .iter()
            .map(|(quad, _)| quad.uv.min[0])
            .collect::<Vec<_>>(),
        vec![0.1, 0.2, 0.3, 0.4]
    );
}

#[test]
fn inventory_text_label_component_signed_width_keeps_visible_prefix_geometry() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'a',
        HudAsciiGlyph {
            advance: 10.0,
            width: 2,
            height: 2,
            bearing_top: HUD_FONT_BASELINE as f32,
            uv: HudUvRect {
                min: [0.7, 0.0],
                max: [0.8, 1.0],
            },
            ..Default::default()
        },
    );
    glyphs.insert_first_wins('n', HudAsciiGlyph::empty(-20.0));
    glyphs.insert_first_wins('z', HudAsciiGlyph::empty(-10.0));
    let fonts = HudFontGlyphBook::from(glyphs);
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], fonts.clone())
        .unwrap();
    for (text, width) in [("an", -10), ("az", 0)] {
        let runs = vec![HudComponentTextRun::plain(text)];
        assert_eq!(hud_component_text_width(&runs, &fonts), width);
        let label = HudInventoryTextLabel {
            component_runs: runs,
            x: 80,
            y: 16,
            width: 0,
            text: String::new(),
            tint: HUD_TINT_WHITE,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        };
        let sanitized = sanitize_hud_inventory_text_label(label.clone()).unwrap();
        assert_eq!(sanitized.width, 1);
        assert_eq!(sanitized.text, text);
        let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
        screen.text_labels.push(label);
        renderer.set_hud_inventory_screen(Some(screen));
        let draws = renderer.collect_hud_draws();
        assert!(draws
            .vertices
            .iter()
            .any(|vertex| (vertex.uv[0] - 0.7).abs() < 1.0e-6));
    }
}

#[test]
fn inventory_text_label_font_selects_matching_book_glyphs_for_draw() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let glyph = |uv_x: f32, advance: u32| HudAsciiGlyph {
        uv: HudUvRect {
            min: [uv_x, 0.0],
            max: [uv_x + 0.05, 1.0],
        },
        width: 1,
        height: 1,
        advance: advance as f32,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut default = HudFontGlyphMap::new();
    default.insert_first_wins('a', glyph(0.1, 2));
    let mut alt = HudFontGlyphMap::new();
    alt.insert_first_wins('a', glyph(0.7, 7));
    let fonts = HudFontGlyphBook::from_fonts(
        "minecraft:default",
        BTreeMap::from([
            ("minecraft:default".to_string(), default),
            ("minecraft:alt".to_string(), alt),
        ]),
    );
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], fonts)
        .unwrap();
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.text_labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 80,
        y: 16,
        width: 86,
        text: "a".to_string(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: Some("minecraft:alt".to_string()),
        runs: Vec::new(),
    });
    renderer.set_hud_inventory_screen(Some(screen.clone()));

    assert_eq!(
        renderer
            .hud_font_book_ref()
            .glyphs(Some("minecraft:alt"))
            .plain_text_width("a"),
        7
    );
    let draws = renderer.collect_hud_draws();
    assert!(draws
        .vertices
        .iter()
        .any(|vertex| (vertex.uv[0] - 0.7).abs() < 1.0e-6));

    screen.text_labels[0].font = None;
    screen.text_labels[0].text.clear();
    screen.text_labels[0].component_runs = vec![HudComponentTextRun {
        visual: HudStyledTextRun {
            text: "a".into(),
            color: Some(0x12ff3456),
            shadow_color: Some(0x80123456),
            style: HudTextStyle {
                bold: true,
                ..Default::default()
            },
        },
        font: Some("minecraft:alt".into()),
    }];
    let sanitized = sanitize_hud_inventory_text_label(screen.text_labels[0].clone()).unwrap();
    assert_eq!(sanitized.text, "a");
    assert_eq!(sanitized.component_runs[0].visual.color, Some(0xff3456));
    assert_eq!(
        sanitized.component_runs[0].visual.shadow_color,
        Some(0x80123456)
    );
    assert!(sanitized.component_runs[0].visual.style.bold);
    assert_eq!(
        sanitized.component_runs[0].font.as_deref(),
        Some("minecraft:alt")
    );
    renderer.set_hud_inventory_screen(Some(screen));
    let draws = renderer.collect_hud_draws();
    assert!(draws
        .vertices
        .iter()
        .any(|vertex| (vertex.uv[0] - 0.7).abs() < 1.0e-6));
}

#[test]
fn rich_component_inventory_tooltip_uses_custom_font_fallback_and_style_geometry() {
    let fonts = component_test_font_book();
    let runs = vec![
        HudComponentTextRun {
            visual: HudStyledTextRun {
                shadow_color: None,
                text: "a".to_string(),
                style: HudTextStyle {
                    italic: true,
                    underlined: true,
                    strikethrough: true,
                    ..HudTextStyle::default()
                },
                color: Some(0x1_12_34_56),
            },
            font: Some("example:wide".to_string()),
        },
        HudComponentTextRun {
            visual: HudStyledTextRun::plain("a"),
            font: Some("example:missing".to_string()),
        },
    ];

    let sanitized = sanitize_hud_inventory_tooltip_line(HudInventoryTooltipLine {
        text: "aa".to_string(),
        tint: [2.0, -1.0, 0.5, 1.0],
        runs,
    })
    .expect("valid tooltip line");
    assert_eq!(sanitized.tint, [1.0, 0.0, 0.5, 1.0]);
    assert_eq!(sanitized.runs[0].font.as_deref(), Some("example:wide"));
    assert_eq!(sanitized.runs[0].visual.color, Some(0x12_34_56));
    assert_eq!(sanitized.runs[1].font.as_deref(), Some("example:missing"));
    assert_eq!(hud_component_runs_width(&sanitized.runs, &fonts), Some(16));

    let geometry = hud_component_text_geometry(
        &sanitized.runs,
        &fonts,
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 2);
    assert_eq!(geometry.glyph_quads[0].0.uv.min[0], 0.6);
    assert!(
        geometry.glyph_quads[0].0.corners[0][0] > geometry.glyph_quads[0].0.corners[1][0],
        "italic custom glyph must preserve the vanilla shear"
    );
    assert_eq!(
        geometry.glyph_quads[1].0.uv.min[0], 0.1,
        "missing tooltip font must fall back to minecraft:default"
    );
    assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 10.0);
    assert_eq!(geometry.effect_rects.len(), 2);
}

#[test]
fn rich_component_space_provider_advances_without_bitmap_quad() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins('\u{200c}', HudAsciiGlyph::empty(9.0));
    let fonts = HudFontGlyphBook::from(glyphs);
    let runs = [HudComponentTextRun {
        visual: HudStyledTextRun {
            shadow_color: None,
            text: "\u{200c}".to_string(),
            style: HudTextStyle {
                underlined: true,
                ..HudTextStyle::default()
            },
            color: None,
        },
        font: None,
    }];

    assert_eq!(hud_component_runs_width(&runs, &fonts), Some(9));
    let geometry = hud_component_text_geometry(
        &runs,
        &fonts,
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert!(
        geometry.glyph_quads.is_empty(),
        "space provider glyphs must not emit bitmap quads"
    );
    assert_eq!(geometry.effect_rects.len(), 1);
    assert_eq!(
        geometry.effect_rects[0].0.kind,
        bbb_render_types::HudEffectKind::Underline
    );
    assert_eq!(
        (geometry.effect_rects[0].0.x0, geometry.effect_rects[0].0.x1),
        (-1.0, 9.0)
    );
}

#[test]
fn rich_component_tooltip_sanitization_bounds_lines_fonts_text_and_tints() {
    let line = || HudRichTooltipLine {
        runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun {
                shadow_color: None,
                text: format!("\u{0007}{}", "x".repeat(100)),
                style: HudTextStyle::default(),
                color: Some(0x1_12_34_56),
            },
            font: Some(format!("example:\u{0007}{}", "f".repeat(300))),
        }],
        tint: [f32::NAN, 0.0, 0.0, 1.0],
    };
    let tooltip = sanitize_hud_rich_tooltip(HudRichTooltip {
        style: Some(format!("example:\u{0007}{}", "s".repeat(300))),
        lines: (0..65).map(|_| line()).collect(),
        advanced_lines: Some((0..65).map(|_| line()).collect()),
        wrap_text: false,
    })
    .expect("bounded tooltip remains non-empty");

    let style = tooltip.style.as_deref().expect("sanitized style");
    assert!(style.chars().count() <= 256);
    assert!(!style.chars().any(char::is_control));
    assert_eq!(tooltip.lines.len(), 64);
    assert!(tooltip
        .advanced_lines
        .as_ref()
        .is_some_and(|lines| lines.len() <= 64));
    let all_lines = tooltip
        .lines
        .iter()
        .chain(tooltip.advanced_lines.as_deref().unwrap_or_default().iter());
    let total_utf16 = all_lines
        .clone()
        .flat_map(|line| line.runs.iter())
        .map(|run| run.visual.text.encode_utf16().count())
        .sum::<usize>();
    assert!(total_utf16 <= 8_192);
    assert!(all_lines.clone().all(|line| line.tint == HUD_TINT_WHITE));
    assert!(all_lines.flat_map(|line| line.runs.iter()).all(|run| {
        run.visual.color == Some(0x12_34_56)
            && run.font.as_ref().is_some_and(|font| {
                font.chars().count() <= 256 && !font.chars().any(char::is_control)
            })
    }));
}

#[test]
fn offscreen_chat_custom_font_selects_its_atlas_region() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    let glyph = |min_x| HudAsciiGlyph {
        uv: HudUvRect {
            min: [min_x, 0.0],
            max: [min_x + 0.5, 1.0],
        },
        advance: 6.0,
        width: 4,
        height: 8,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut default = HudFontGlyphMap::new();
    default.insert_first_wins('a', glyph(0.0));
    let mut custom = HudFontGlyphMap::new();
    custom.insert_first_wins('a', glyph(0.5));
    renderer
        .upload_hud_font_atlas(
            2,
            1,
            &[255, 0, 0, 255, 0, 255, 0, 255],
            HudFontGlyphBook::from_fonts(
                "minecraft:default",
                BTreeMap::from([
                    ("example:green".to_string(), custom),
                    ("minecraft:default".to_string(), default),
                ]),
            ),
        )
        .expect("font atlas");
    renderer.set_hud_chat_messages(vec![HudChatMessage {
        display_sequence: 1,
        added_tick: 0,
        runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun::plain("a"),
            font: Some("example:green".to_string()),
        }],
        interactions: Vec::new(),
        tag: None,
    }]);
    renderer.set_hud_chat_context(HudChatContext::default());

    let pixels = renderer
        .render_offscreen_frame()
        .expect("offscreen chat frame");
    let glyph_pixel = pixels.pixel(5, 193);
    assert!(
        glyph_pixel[1] > glyph_pixel[0] && glyph_pixel[1] > glyph_pixel[2],
        "custom-font chat glyph should sample the green atlas half, got {glyph_pixel:?}"
    );
}

#[test]
fn sanitize_hud_inventory_screen_keeps_slot_positions_and_sanitizes_icons() {
    let screen = sanitize_hud_inventory_screen(HudInventoryScreen {
        width: 0,
        height: 700,
        background_layers: vec![
            HudInventoryBackgroundLayer {
                texture: HudInventoryBackgroundTexture::Texture(
                    "textures/gui/container/generic_54.png",
                ),
                x: 0,
                y: 0,
                width: 176,
                height: 125,
                uv: HudUvRect {
                    min: [-1.0, 0.0],
                    max: [0.75, 2.0],
                },
            },
            HudInventoryBackgroundLayer {
                texture: HudInventoryBackgroundTexture::Texture(
                    "textures/gui/container/inventory.png",
                ),
                x: 0,
                y: 0,
                width: 0,
                height: 96,
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
            },
        ],
        foreground_layers: Vec::new(),
        fill_layers: vec![
            HudInventoryFillLayer {
                x: 8,
                y: 84,
                width: 16,
                height: 16,
                tint: [2.0, 0.5, -1.0, 0.5],
                stage: HudInventoryFillStage::BeforeGhostItem,
            },
            HudInventoryFillLayer {
                x: 26,
                y: 84,
                width: 0,
                height: 16,
                tint: [1.0, 1.0, 1.0, 1.0],
                stage: HudInventoryFillStage::AfterGhostItem,
            },
        ],
        slots: vec![
            HudInventorySlot {
                slot_id: 5,
                x: 8,
                y: 84,
                icon: Some(HudItemIcon {
                    pop: 0.0,
                    lighting: GuiItemLightingEntry::ItemsFlat,
                    layers: vec![HudIconLayer::new(
                        HudUvRect {
                            min: [1.25, 0.75],
                            max: [-0.5, 0.25],
                        },
                        [1.5, 0.25, -1.0, 1.0],
                    )],
                    foil: HudItemFoil::Standard,
                    count_label: Some(HudItemCountLabel::new("64")),
                    durability_bar: Some(HudItemDurabilityBar::new(99, [-1.0, 0.5, 1.5])),
                    cooldown_progress: Some(1.5),
                }),
                block_model: None,
            },
            HudInventorySlot {
                slot_id: 6,
                x: 26,
                y: 84,
                icon: Some(HudItemIcon {
                    pop: 0.0,
                    lighting: GuiItemLightingEntry::ItemsFlat,
                    layers: vec![HudIconLayer::new(
                        HudUvRect {
                            min: [0.0, f32::NAN],
                            max: [1.0, 1.0],
                        },
                        [1.0, 1.0, 1.0, 1.0],
                    )],
                    foil: HudItemFoil::Standard,
                    count_label: Some(HudItemCountLabel::new("bad")),
                    durability_bar: None,
                    cooldown_progress: Some(f32::INFINITY),
                }),
                block_model: None,
            },
            HudInventorySlot {
                slot_id: 7,
                x: 44,
                y: 84,
                icon: None,
                block_model: None,
            },
        ],
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels: vec![
            HudInventoryTextLabel {
                component_runs: Vec::new(),
                x: 62,
                y: 24,
                width: 103,
                text: "Name\u{0007}".to_string(),
                tint: [1.25, 0.5, -1.0, 1.0],
                background: Some(HudInventoryTextBackground {
                    x: 60,
                    y: 22,
                    width: 120,
                    height: 12,
                    tint: [0.0, 0.0, 0.0, 1.5],
                }),
                input: None,
                scrolling: None,
                shadow: false,
                font: None,
                runs: Vec::new(),
            },
            HudInventoryTextLabel {
                component_runs: Vec::new(),
                x: 10,
                y: 10,
                width: 0,
                text: "ignored".to_string(),
                tint: HUD_TINT_WHITE,
                background: None,
                input: None,
                scrolling: None,
                shadow: true,
                font: None,
                runs: Vec::new(),
            },
        ],
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: Some(7),
        tooltip: Some(HudInventoryTooltip {
            slot_id: 5,
            x: 8,
            y: 84,
            style: None,
            lines: vec![
                HudInventoryTooltipLine {
                    text: "Diamond Sword".to_string(),
                    tint: [1.5, 1.0, 0.5, 1.0],
                    runs: Vec::new(),
                },
                HudInventoryTooltipLine {
                    text: String::new(),
                    tint: HUD_TINT_WHITE,
                    runs: Vec::new(),
                },
                HudInventoryTooltipLine {
                    text: "Attack\u{0007}Damage".to_string(),
                    tint: [0.25, 0.5, 0.75, 2.0],
                    runs: Vec::new(),
                },
            ],
        }),
    });

    assert_eq!(screen.width, 1);
    assert_eq!(screen.height, 512);
    assert_eq!(screen.background_layers.len(), 1);
    assert_eq!(
        screen.background_layers[0].texture,
        HudInventoryBackgroundTexture::Texture("textures/gui/container/generic_54.png")
    );
    assert_eq!(screen.background_layers[0].uv.min, [0.0, 0.0]);
    assert_eq!(screen.background_layers[0].uv.max, [0.75, 1.0]);
    assert_eq!(
        screen.fill_layers,
        vec![HudInventoryFillLayer {
            x: 8,
            y: 84,
            width: 16,
            height: 16,
            tint: [1.0, 0.5, 0.0, 0.5],
            stage: HudInventoryFillStage::BeforeGhostItem,
        }]
    );
    assert_eq!(screen.hovered_slot_id, Some(7));
    assert_eq!(screen.slots.len(), 3);
    assert_eq!(screen.slots[0].slot_id, 5);
    assert_eq!(screen.slots[0].x, 8);
    assert_eq!(screen.slots[0].y, 84);
    assert_eq!(
        screen.slots[0].icon,
        Some(HudItemIcon {
            pop: 0.0,
            lighting: GuiItemLightingEntry::ItemsFlat,
            layers: vec![HudIconLayer::new(
                HudUvRect {
                    min: [0.0, 0.25],
                    max: [1.0, 0.75],
                },
                [1.0, 0.25, 0.0, 1.0],
            )],
            foil: HudItemFoil::Standard,
            count_label: Some(HudItemCountLabel::new("64")),
            durability_bar: Some(HudItemDurabilityBar::new(13, [0.0, 0.5, 1.0])),
            cooldown_progress: Some(1.0),
        })
    );
    assert_eq!(screen.slots[1].slot_id, 6);
    assert_eq!(screen.slots[1].icon, None);
    assert_eq!(screen.slots[2].slot_id, 7);
    assert_eq!(screen.slots[2].icon, None);
    assert_eq!(
        screen.text_labels,
        vec![HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 62,
            y: 24,
            width: 103,
            text: "Name".to_string(),
            tint: [1.0, 0.5, 0.0, 1.0],
            background: Some(HudInventoryTextBackground {
                x: 60,
                y: 22,
                width: 120,
                height: 12,
                tint: [0.0, 0.0, 0.0, 1.0],
            }),
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            // Plain labels (empty runs in) synthesize one default-style
            // run from the sanitized text.
            runs: vec![HudStyledTextRun::plain("Name")],
        }]
    );
    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 5,
            x: 8,
            y: 84,
            style: None,
            lines: vec![
                HudInventoryTooltipLine {
                    text: "Diamond Sword".to_string(),
                    tint: [1.0, 1.0, 0.5, 1.0],
                    runs: vec![HudComponentTextRun::plain("Diamond Sword")],
                },
                HudInventoryTooltipLine {
                    text: String::new(),
                    tint: HUD_TINT_WHITE,
                    runs: Vec::new(),
                },
                HudInventoryTooltipLine {
                    text: "AttackDamage".to_string(),
                    tint: [0.25, 0.5, 0.75, 1.0],
                    runs: vec![HudComponentTextRun::plain("AttackDamage")],
                },
            ],
        })
    );
}

#[test]
fn sanitize_hud_inventory_text_label_keeps_empty_text_input_decoration() {
    let label = sanitize_hud_inventory_text_label(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 29,
        y: 16,
        width: 73,
        text: String::new(),
        tint: [1.2, 1.0, -1.0, 1.0],
        background: None,
        input: Some(HudInventoryTextInputDecoration {
            cursor: 99,
            selection: 98,
            scroll_to: 97,
            max_length: 2_000,
            cursor_visible: true,
            cursor_tint: [2.0, 0.5, -1.0, 1.0],
        }),
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    })
    .unwrap();

    assert_eq!(label.text, "");
    assert_eq!(label.tint, [1.0, 1.0, 0.0, 1.0]);
    assert!(label.runs.is_empty());
    assert_eq!(
        label.input,
        Some(HudInventoryTextInputDecoration {
            cursor: 0,
            selection: 0,
            scroll_to: 0,
            max_length: 2_000,
            cursor_visible: true,
            cursor_tint: [1.0, 0.5, 0.0, 1.0],
        })
    );
}

#[test]
fn sanitize_hud_inventory_background_keeps_repeat_uv_outside_unit_range() {
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.background_layers = vec![
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Texture(
                "textures/gui/inworld_menu_background.png",
            ),
            x: -144,
            y: -44,
            width: 800,
            height: 600,
            uv: HudUvRect {
                min: [25.0, 18.75],
                max: [0.0, 0.0],
            },
        },
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("widget/button"),
            x: 0,
            y: 0,
            width: 200,
            height: 20,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [25.0, 18.75],
            },
        },
    ];

    let screen = sanitize_hud_inventory_screen(screen);

    assert_eq!(screen.background_layers[0].uv.min, [0.0, 0.0]);
    assert_eq!(screen.background_layers[0].uv.max, [25.0, 18.75]);
    assert_eq!(screen.background_layers[1].uv.max, [1.0, 1.0]);
}

#[test]
fn loom_pattern_thumbnails_interleave_static_and_dynamic_entity_atlases() {
    use crate::entity_models::{EntityDynamicTexture, EntityModelTextureImage};
    use crate::DynamicEntityTextureImage;

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .add_test_gui_sprite("container/loom/pattern", 1, 1, &[255, 0, 0, 255])
        .unwrap();
    let pattern = crate::BannerPatternKind::StripeTop;
    let pattern_texture = banner_pattern_texture_ref(pattern);
    renderer
        .upload_entity_model_textures(&[EntityModelTextureImage::new(
            pattern_texture,
            [0, 0, 255, 255].repeat(
                usize::try_from(pattern_texture.size[0] * pattern_texture.size[1]).unwrap(),
            ),
        )])
        .unwrap();
    renderer
        .upload_dynamic_entity_texture(DynamicEntityTextureImage {
            handle: 81_261,
            size: [64, 64],
            standalone_sampling: None,
            rgba: [255, 255, 0, 255].repeat(64 * 64),
        })
        .unwrap();

    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.background_layers = vec![
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("container/loom/pattern"),
            x: 60,
            y: 13,
            width: 14,
            height: 14,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("container/loom/pattern"),
            x: 74,
            y: 13,
            width: 14,
            height: 14,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
    ];
    let thumbnail = |texture, x, after_background_layer| HudInventoryBannerPatternPreview {
        texture: Some(texture),
        x,
        y: 15,
        width: 5,
        height: 10,
        uv: HudUvRect {
            min: [0.0, 1.0 / 64.0],
            max: [21.0 / 64.0, 41.0 / 64.0],
        },
        after_background_layer,
    };
    screen.banner_pattern_previews = vec![
        thumbnail(BannerPatternTexture::Builtin(pattern), 64, 0),
        thumbnail(
            BannerPatternTexture::Runtime(EntityDynamicTexture { handle: 81_261 }),
            78,
            1,
        ),
    ];
    renderer.set_hud_inventory_screen(Some(screen));

    let draws = renderer.collect_hud_draws();
    assert_eq!(draws.commands.len(), 6);
    assert!(matches!(draws.commands[0], HudDrawCommand::Sprite { .. }));
    assert!(matches!(draws.commands[1], HudDrawCommand::Fill { .. }));
    assert!(matches!(
        draws.commands[2],
        HudDrawCommand::BannerPattern {
            atlas: HudBannerPatternAtlas::StaticEntity,
            ..
        }
    ));
    assert!(matches!(draws.commands[3], HudDrawCommand::Sprite { .. }));
    assert!(matches!(draws.commands[4], HudDrawCommand::Fill { .. }));
    assert!(matches!(
        draws.commands[5],
        HudDrawCommand::BannerPattern {
            atlas: HudBannerPatternAtlas::DynamicEntity,
            ..
        }
    ));

    let pixels = renderer.render_offscreen_frame().unwrap();
    let origin_x = (320 - 176) / 2;
    let origin_y = (240 - 166) / 2;
    let static_pixel = pixels.pixel(origin_x + 66, origin_y + 20);
    assert!(
        static_pixel[2] > 192 && static_pixel[0] < 64 && static_pixel[1] < 64,
        "static banner atlas should paint the first thumbnail blue, got {static_pixel:?}"
    );
    let dynamic_pixel = pixels.pixel(origin_x + 80, origin_y + 20);
    assert!(
        dynamic_pixel[0] > 192 && dynamic_pixel[1] > 192 && dynamic_pixel[2] < 64,
        "dynamic banner atlas should paint the second thumbnail yellow, got {dynamic_pixel:?}"
    );
}

#[test]
fn sanitize_hud_inventory_screen_keeps_sanitized_floating_items() {
    let screen = sanitize_hud_inventory_screen(HudInventoryScreen {
        width: 176,
        height: 166,
        background_layers: Vec::new(),
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: vec![
            HudInventoryItem {
                x: 33,
                y: 19,
                scale: 2.0,
                scale_y: 20.0,
                icon: HudItemIcon {
                    pop: 0.0,
                    lighting: GuiItemLightingEntry::ItemsFlat,
                    layers: vec![HudIconLayer::new(
                        HudUvRect {
                            min: [1.25, 0.75],
                            max: [-0.5, 0.25],
                        },
                        [1.25, 0.5, -1.0, 1.0],
                    )],
                    foil: HudItemFoil::Standard,
                    count_label: Some(HudItemCountLabel::new("12")),
                    durability_bar: Some(HudItemDurabilityBar::new(99, [0.25, 2.0, -1.0])),
                    cooldown_progress: Some(1.5),
                },
                scissor: Some(HudInventoryItemScissor {
                    x: 9,
                    y: 18,
                    width: 900,
                    height: 0,
                }),
                draw_decorations: true,
                block_model: None,
            },
            HudInventoryItem {
                x: 51,
                y: 19,
                scale: 1.0,
                scale_y: 1.0,
                icon: HudItemIcon {
                    pop: 0.0,
                    lighting: GuiItemLightingEntry::ItemsFlat,
                    layers: vec![HudIconLayer::new(
                        HudUvRect {
                            min: [0.0, f32::NAN],
                            max: [1.0, 1.0],
                        },
                        [1.0, 1.0, 1.0, 1.0],
                    )],
                    foil: HudItemFoil::Standard,
                    count_label: Some(HudItemCountLabel::new("64")),
                    durability_bar: None,
                    cooldown_progress: None,
                },
                scissor: None,
                draw_decorations: true,
                block_model: None,
            },
        ],
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
        text_labels: Vec::new(),
    });

    assert_eq!(screen.floating_items.len(), 1);
    assert_eq!(screen.floating_items[0].x, 33);
    assert_eq!(screen.floating_items[0].y, 19);
    assert_eq!(screen.floating_items[0].scale, 2.0);
    assert_eq!(screen.floating_items[0].scale_y, 16.0);
    assert_eq!(screen.floating_items[0].scissor, None);
    assert!(screen.floating_items[0].draw_decorations);
    assert_eq!(
        screen.floating_items[0].icon,
        HudItemIcon {
            pop: 0.0,
            lighting: GuiItemLightingEntry::ItemsFlat,
            layers: vec![HudIconLayer::new(
                HudUvRect {
                    min: [0.0, 0.25],
                    max: [1.0, 0.75],
                },
                [1.0, 0.5, 0.0, 1.0],
            )],
            foil: HudItemFoil::Standard,
            count_label: Some(HudItemCountLabel::new("12")),
            durability_bar: Some(HudItemDurabilityBar::new(13, [0.25, 1.0, 0.0])),
            cooldown_progress: Some(1.0),
        }
    );
}

#[test]
fn loom_result_banner_pip_renders_flag_before_pattern_button_stratum() {
    use crate::entity_models::EntityModelTextureImage;

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/gui/container/loom.png",
            1,
            1,
            &[0, 0, 255, 255],
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite("container/loom/pattern", 1, 1, &[255, 0, 0, 255])
        .unwrap();
    let banner_images = crate::entity_model_texture_refs()
        .iter()
        .filter(|texture| texture.path.starts_with("textures/entity/banner"))
        .map(|texture| {
            EntityModelTextureImage::new(
                *texture,
                [0, 255, 0, 255]
                    .repeat(usize::try_from(texture.size[0] * texture.size[1]).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    renderer
        .upload_entity_model_textures(&banner_images)
        .unwrap();

    let preview = HudEntityPreview {
        entity: EntityModelInstance::banner_flag(-1, EntityDyeColor::White, [None; 16]),
        lighting: GuiItemLightingEntry::ItemsFlat,
        rect: HudEntityPreviewRect {
            x: 141,
            y: 8,
            width: 20,
            height: 40,
        },
        scissor: None,
        translation: [0.0, 0.25, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        override_camera_rotation: None,
        scale: 16.0,
        depth_isolated: true,
        after_background_layer: Some(0),
        item_layers: Vec::new(),
    };
    let mut screen = hud_inventory_screen_with_entity_previews(vec![preview]);
    screen.background_layers = vec![
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Texture("textures/gui/container/loom.png"),
            x: 0,
            y: 0,
            width: 176,
            height: 166,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("container/loom/pattern"),
            x: 145,
            y: 20,
            width: 10,
            height: 10,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
    ];
    renderer.set_hud_inventory_screen(Some(screen));

    let pixels = renderer.render_offscreen_frame().unwrap();
    let draws = renderer.collect_hud_draws();
    assert!(matches!(draws.commands[0], HudDrawCommand::Sprite { .. }));
    assert!(matches!(
        draws.commands[1],
        HudDrawCommand::EntityPreviewBlit {
            target_index: 0,
            ..
        }
    ));
    assert!(matches!(draws.commands[2], HudDrawCommand::Sprite { .. }));

    let origin_x = (320 - 176) / 2;
    let origin_y = (240 - 166) / 2;
    let mut green_flag_pixels = 0usize;
    for y in (origin_y + 8)..(origin_y + 48) {
        for x in (origin_x + 141)..(origin_x + 161) {
            let pixel = pixels.pixel(x, y);
            if pixel[1] > 64 && pixel[1] > pixel[0] && pixel[1] > pixel[2] {
                green_flag_pixels += 1;
            }
        }
    }
    assert!(
        green_flag_pixels > 0,
        "Loom result PIP should contribute flag-model pixels"
    );
    let overlay = pixels.pixel(origin_x + 146, origin_y + 21);
    assert!(
        overlay[0] > 192 && overlay[1] < 64 && overlay[2] < 64,
        "later pattern button should cover the PIP, got {overlay:?}"
    );
}

#[test]
fn preedit_overlay_sanitizes_bounds_and_draws_focused_underline_and_caret() {
    let sanitized = sanitize_hud_inventory_preedit_overlay(HudInventoryPreeditOverlay {
        text: "a\u{0007}bc".to_string(),
        focused_start: 2,
        focused_end: usize::MAX,
        caret: Some(usize::MAX),
        caret_visible: true,
        text_x: 20,
        text_y: 30,
        background_x: 15,
        background_y: 25,
        background_width: u32::MAX,
        background_height: u32::MAX,
    })
    .unwrap();
    assert_eq!(sanitized.text, "abc");
    assert_eq!((sanitized.focused_start, sanitized.focused_end), (2, 3));
    assert_eq!(sanitized.caret, Some(3));
    assert_eq!(
        (sanitized.background_width, sanitized.background_height),
        (2048, 512)
    );

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let glyph = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        width: 1,
        height: 1,
        advance: 6.0,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    for ch in "abc".chars() {
        glyphs.insert_first_wins(ch, glyph);
    }
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .unwrap();
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.preedit_overlay = Some(HudInventoryPreeditOverlay {
        text: "abc".to_string(),
        focused_start: 1,
        focused_end: 2,
        caret: Some(2),
        caret_visible: true,
        text_x: 20,
        text_y: 30,
        background_x: 15,
        background_y: 25,
        background_width: 28,
        background_height: 19,
    });
    renderer.set_hud_inventory_screen(Some(screen));

    let draws = renderer.collect_hud_draws();
    assert_eq!(
        draws
            .commands
            .iter()
            .filter(|command| matches!(command, HudDrawCommand::Sprite { .. }))
            .count(),
        2,
        "glyphs and focused underline retain font texture semantics"
    );
    assert_eq!(
        draws
            .commands
            .iter()
            .filter_map(|command| match command {
                HudDrawCommand::Sprite { start, end, .. } => Some(end - start),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [18, 6]
    );
    assert!(
        matches!(draws.commands.last(), Some(HudDrawCommand::Fill { .. })),
        "caret is an untextured fill"
    );
    assert_eq!(draws.vertices.len(), 5 * 6);
}

#[test]
fn multiline_text_area_draws_with_nested_scissor_and_restores_parent() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let glyph = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        width: 1,
        height: 1,
        advance: 6.0,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    for ch in "a /0123456789_".chars() {
        glyphs.insert_first_wins(ch, glyph);
    }
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .unwrap();

    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.scroll_area = Some(HudInventoryScrollArea {
        scissor: HudInventoryItemScissor {
            x: 2,
            y: 30,
            width: 30,
            height: 40,
        },
        background_layers: Vec::new(),
        fill_layers: Vec::new(),
        text_labels: Vec::new(),
        text_areas: vec![HudInventoryTextArea {
            needs_cursor_position: false,
            x: 2,
            y: 30,
            width: 20,
            height: 17,
            text: "aaaa aaaa aaaa".to_string(),
            cursor: 14,
            selection: 5,
            scroll_amount: i32::MAX,
            max_length: 32,
            cursor_visible: true,
            text_tint: HUD_TINT_WHITE,
            cursor_tint: HUD_TINT_WHITE,
            counter_tint: [0.5, 0.5, 0.5, 1.0],
            shadow: true,
        }],
        scrollbar: None,
    });
    renderer.set_hud_inventory_screen(Some(screen));

    let draws = renderer.collect_hud_draws();
    let scissors = draws
        .commands
        .iter()
        .filter_map(|command| match command {
            HudDrawCommand::Scissor {
                x,
                y,
                width,
                height,
            } => Some((*x, *y, *width, *height)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(scissors.ends_with(&[
        (74, 67, 30, 40),
        (75, 68, 18, 15),
        (74, 67, 30, 40),
        (0, 0, 320, 240),
    ]));
    assert!(!draws.vertices.is_empty());
}

#[test]
fn sanitize_entity_preview_allows_items_flat_only_for_gui_owned_models() {
    let base = hud_entity_preview_for_test();
    assert!(sanitize_hud_entity_preview(HudEntityPreview {
        lighting: GuiItemLightingEntry::ItemsFlat,
        ..base.clone()
    })
    .is_none());

    let gui_sign = HudEntityPreview {
        entity: EntityModelInstance::sign(
            -1,
            [0.0, 0.0, 0.0],
            0.0,
            SignModelWood::Oak,
            SignModelAttachment::Standing,
        ),
        lighting: GuiItemLightingEntry::ItemsFlat,
        rect: HudEntityPreviewRect {
            x: 0,
            y: 0,
            width: 96,
            height: 102,
        },
        scissor: None,
        translation: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        override_camera_rotation: None,
        scale: 62.500_004,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: Vec::new(),
    };
    let sanitized = sanitize_hud_entity_preview(gui_sign).expect("gui sign preview");
    assert_eq!(sanitized.lighting, GuiItemLightingEntry::ItemsFlat);
    assert_eq!(
        sanitized.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );

    let gui_banner = HudEntityPreview {
        entity: EntityModelInstance::banner_flag(-1, EntityDyeColor::Red, [None; 16]),
        lighting: GuiItemLightingEntry::ItemsFlat,
        rect: HudEntityPreviewRect {
            x: 141,
            y: 8,
            width: 20,
            height: 40,
        },
        scissor: None,
        translation: [0.0, 0.25, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        override_camera_rotation: None,
        scale: 16.0,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: Vec::new(),
    };
    assert!(sanitize_hud_entity_preview(gui_banner.clone()).is_some());
    assert!(sanitize_hud_entity_preview(HudEntityPreview {
        entity: EntityModelInstance::banner(
            -1,
            [0.0, 0.0, 0.0],
            0.0,
            false,
            EntityDyeColor::Red,
            [None; 16],
        ),
        ..gui_banner
    })
    .is_none());
}

#[test]
fn sanitize_debug_options_screen_keeps_empty_search_and_clamps_visible_rows() {
    let screen = sanitize_hud_debug_options_screen(HudDebugOptionsScreen {
        title: "Debug\nOptions".to_string(),
        warning: "Warning".to_string(),
        search: HudTextField {
            text: "abc".to_string(),
            cursor: 99,
            selection: 1,
            max_length: 32,
            focused: true,
            cursor_visible: true,
            ..HudTextField::default()
        },
        search_hint: "Search...".to_string(),
        never_label: "OFF".to_string(),
        overlay_label: "In Overlay".to_string(),
        always_label: "Always".to_string(),
        tooltip: Some(HudDebugOptionsTooltip {
            text: "No\nReduced".to_string(),
            x: 12,
            y: 34,
        }),
        rows: vec![
            HudDebugOptionsRow::Category {
                label: "Debug Screen Text".to_string(),
            },
            HudDebugOptionsRow::Entry {
                path: "biome".to_string(),
                status: HudDebugOptionsEntryStatus::AlwaysOn,
                hovered_status: Some(HudDebugOptionsEntryStatus::AlwaysOn),
                focused_status: None,
                allowed: true,
            },
            HudDebugOptionsRow::Entry {
                path: "fps".to_string(),
                status: HudDebugOptionsEntryStatus::InOverlay,
                hovered_status: None,
                focused_status: Some(HudDebugOptionsEntryStatus::Never),
                allowed: false,
            },
        ],
        scroll_row: 999,
        total_rows: 3,
        visible_rows: 2,
        default_profile_active: false,
        default_profile_hovered: true,
        default_profile_focused: false,
        default_profile_label: "Default profile".to_string(),
        performance_profile_active: true,
        performance_profile_hovered: false,
        performance_profile_focused: true,
        performance_profile_label: "Performance profile".to_string(),
        done_hovered: true,
        done_focused: false,
        done_label: "Done".to_string(),
    })
    .expect("debug options screen");

    assert_eq!(screen.title, "DebugOptions");
    assert_eq!(screen.search.text, "abc");
    assert_eq!(screen.search.cursor, 3);
    assert_eq!(screen.search.selection, 1);
    assert!(screen.search.cursor_visible);
    assert_eq!(screen.rows.len(), 2);
    assert_eq!(screen.scroll_row, 3);
    assert_eq!(
        screen.tooltip.as_ref().map(|tooltip| tooltip.text.as_str()),
        Some("NoReduced")
    );
    assert!(!screen.default_profile_active);
    assert!(screen.default_profile_hovered);
    assert!(screen.performance_profile_active);
    assert!(!screen.performance_profile_hovered);
    assert!(screen.done_hovered);

    let wide_search = format!("{}a", "\u{1f600}".repeat(16));
    let wide = sanitize_hud_debug_options_screen(HudDebugOptionsScreen {
        title: "Debug Options".to_string(),
        warning: "Warning".to_string(),
        search: HudTextField {
            text: wide_search,
            cursor: 99,
            selection: 99,
            max_length: 32,
            focused: false,
            cursor_visible: true,
            ..HudTextField::default()
        },
        search_hint: "Search...".to_string(),
        never_label: "OFF".to_string(),
        overlay_label: "In Overlay".to_string(),
        always_label: "Always".to_string(),
        tooltip: None,
        rows: Vec::new(),
        scroll_row: 0,
        total_rows: 0,
        visible_rows: 0,
        default_profile_active: true,
        default_profile_hovered: false,
        default_profile_focused: false,
        default_profile_label: "Default profile".to_string(),
        performance_profile_active: true,
        performance_profile_hovered: false,
        performance_profile_focused: false,
        performance_profile_label: "Performance profile".to_string(),
        done_hovered: false,
        done_focused: false,
        done_label: "Done".to_string(),
    })
    .expect("wide debug options screen");
    assert_eq!(wide.search.text, "\u{1f600}".repeat(16));
    assert_eq!(wide.search.text.encode_utf16().count(), 32);
    assert_eq!(wide.search.cursor, 32);
    assert_eq!(wide.search.selection, 32);

    assert!(sanitize_hud_debug_options_screen(HudDebugOptionsScreen {
        title: "\n".to_string(),
        warning: "Warning".to_string(),
        search: HudTextField {
            text: String::new(),
            cursor: 0,
            selection: 0,
            max_length: 32,
            focused: false,
            cursor_visible: false,
            ..HudTextField::default()
        },
        search_hint: "Search...".to_string(),
        never_label: "OFF".to_string(),
        overlay_label: "In Overlay".to_string(),
        always_label: "Always".to_string(),
        tooltip: None,
        rows: Vec::new(),
        scroll_row: 0,
        total_rows: 0,
        visible_rows: 0,
        default_profile_active: true,
        default_profile_hovered: false,
        default_profile_focused: false,
        default_profile_label: "Default profile".to_string(),
        performance_profile_active: true,
        performance_profile_hovered: false,
        performance_profile_focused: false,
        performance_profile_label: "Performance profile".to_string(),
        done_hovered: false,
        done_focused: false,
        done_label: "Done".to_string(),
    })
    .is_none());
}

#[test]
fn pause_screen_title_origin_matches_vanilla_y_positions() {
    let glyphs = styled_test_glyphs();
    let no_menu = HudPauseScreen {
        title: "ab".to_string(),
        labels: HudPauseScreenLabels::default(),
        show_pause_menu: false,
        feedback_subscreen: false,
        return_to_game_hovered: false,
        advancements_hovered: false,
        stats_hovered: false,
        send_feedback_hovered: false,
        report_bugs_hovered: false,
        report_bugs_enabled: true,
        custom_dialog_label: None,
        custom_dialog_hovered: false,
        custom_dialog_tooltip: None,
        feedback_back_hovered: false,
        options_hovered: false,
        options_enabled: false,
        options_tooltip: None,
        disconnect_hovered: false,
        disconnect_enabled: true,
    };
    let menu = HudPauseScreen {
        title: "ab".to_string(),
        labels: HudPauseScreenLabels::default(),
        show_pause_menu: true,
        feedback_subscreen: false,
        return_to_game_hovered: false,
        advancements_hovered: false,
        stats_hovered: false,
        send_feedback_hovered: false,
        report_bugs_hovered: false,
        report_bugs_enabled: true,
        custom_dialog_label: None,
        custom_dialog_hovered: false,
        custom_dialog_tooltip: None,
        feedback_back_hovered: false,
        options_hovered: false,
        options_enabled: false,
        options_tooltip: None,
        disconnect_hovered: false,
        disconnect_enabled: true,
    };
    let feedback = HudPauseScreen {
        feedback_subscreen: true,
        ..menu.clone()
    };

    assert_eq!(
        hud_pause_screen_title_origin(&no_menu, &glyphs, PhysicalSize::new(100, 50)),
        (44.0, 10.0)
    );
    assert_eq!(
        hud_pause_screen_title_origin(&menu, &glyphs, PhysicalSize::new(100, 50)),
        (44.0, 40.0)
    );
    assert_eq!(
        hud_pause_screen_title_origin(&feedback, &glyphs, PhysicalSize::new(100, 50)),
        (44.0, 12.0)
    );
    assert!(!hud_pause_screen_draws_background(&no_menu));
    assert!(hud_pause_screen_draws_background(&menu));
    assert!(hud_pause_screen_draws_background(&feedback));
}

#[test]
fn pause_screen_menu_background_matches_vanilla_transparent_gradient() {
    let vertices = hud_pause_background_vertices(PhysicalSize::new(320, 240));

    assert_eq!(vertices[0].position, [-1.0, 1.0]);
    assert_eq!(vertices[1].position, [1.0, 1.0]);
    assert_eq!(vertices[2].position, [1.0, -1.0]);
    assert_eq!(vertices[3].position, [-1.0, 1.0]);
    assert_eq!(vertices[4].position, [1.0, -1.0]);
    assert_eq!(vertices[5].position, [-1.0, -1.0]);
    assert_eq!(vertices[0].tint, HUD_PAUSE_BACKGROUND_TOP_TINT);
    assert_eq!(vertices[1].tint, HUD_PAUSE_BACKGROUND_TOP_TINT);
    assert_eq!(vertices[3].tint, HUD_PAUSE_BACKGROUND_TOP_TINT);
    assert_eq!(vertices[2].tint, HUD_PAUSE_BACKGROUND_BOTTOM_TINT);
    assert_eq!(vertices[4].tint, HUD_PAUSE_BACKGROUND_BOTTOM_TINT);
    assert_eq!(vertices[5].tint, HUD_PAUSE_BACKGROUND_BOTTOM_TINT);
}

#[test]
fn pause_screen_button_rects_match_vanilla_grid_and_header_footer_layouts() {
    assert_eq!(
        hud_pause_return_to_game_button_rect(PhysicalSize::new(320, 240)),
        (58, 68, 204, 20)
    );
    assert_eq!(
        hud_pause_return_to_game_button_rect(PhysicalSize::new(854, 480)),
        (325, 128, 204, 20)
    );
    assert_eq!(
        hud_pause_advancements_button_rect(PhysicalSize::new(320, 240)),
        (58, 92, 98, 20)
    );
    assert_eq!(
        hud_pause_advancements_button_rect(PhysicalSize::new(854, 480)),
        (325, 152, 98, 20)
    );
    assert_eq!(
        hud_pause_stats_button_rect(PhysicalSize::new(320, 240)),
        (164, 92, 98, 20)
    );
    assert_eq!(
        hud_pause_stats_button_rect(PhysicalSize::new(854, 480)),
        (431, 152, 98, 20)
    );
    assert_eq!(
        hud_pause_send_feedback_button_rect(PhysicalSize::new(320, 240)),
        (58, 116, 98, 20)
    );
    assert_eq!(
        hud_pause_send_feedback_button_rect(PhysicalSize::new(854, 480)),
        (325, 176, 98, 20)
    );
    assert_eq!(
        hud_pause_report_bugs_button_rect(PhysicalSize::new(320, 240)),
        (164, 116, 98, 20)
    );
    assert_eq!(
        hud_pause_report_bugs_button_rect(PhysicalSize::new(854, 480)),
        (431, 176, 98, 20)
    );
    assert_eq!(
        hud_pause_options_button_rect(PhysicalSize::new(320, 240)),
        (58, 140, 98, 20)
    );
    assert_eq!(
        hud_pause_options_button_rect(PhysicalSize::new(854, 480)),
        (325, 200, 98, 20)
    );
    assert_eq!(
        hud_pause_disconnect_button_rect(PhysicalSize::new(320, 240)),
        (58, 164, 204, 20)
    );
    assert_eq!(
        hud_pause_disconnect_button_rect(PhysicalSize::new(854, 480)),
        (325, 224, 204, 20)
    );
    assert_eq!(
        hud_pause_feedback_send_feedback_button_rect(PhysicalSize::new(320, 240)),
        (58, 67, 98, 20)
    );
    assert_eq!(
        hud_pause_feedback_report_bugs_button_rect(PhysicalSize::new(320, 240)),
        (164, 67, 98, 20)
    );
    assert_eq!(
        hud_pause_feedback_back_button_rect(PhysicalSize::new(320, 240)),
        (60, 213, 200, 20)
    );
    assert_eq!(
        hud_pause_feedback_send_feedback_button_rect(PhysicalSize::new(320, 100)),
        (58, 47, 98, 20)
    );
    assert_eq!(
        hud_pause_feedback_back_button_rect(PhysicalSize::new(320, 100)),
        (60, 73, 200, 20)
    );
    assert_eq!(
        hud_stats_done_button_rect(PhysicalSize::new(320, 240)),
        (60, 213, 200, 20)
    );
    assert_eq!(
        hud_stats_done_button_rect(PhysicalSize::new(854, 480)),
        (327, 453, 200, 20)
    );
}
