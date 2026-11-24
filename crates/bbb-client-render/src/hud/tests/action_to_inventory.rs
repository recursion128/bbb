use super::*;
use bbb_protocol::game_types::GameType;

#[test]
fn hide_gui_drops_hotbar_item_residency_but_keeps_screen_and_toast_items() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let model = test_hud_block_item_model();
    renderer.set_hud_hotbar_block_item_models(vec![Some(model.clone())]);
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.slots.push(HudInventorySlot {
        slot_id: 0,
        x: 8,
        y: 8,
        icon: None,
        block_model: Some(model.clone()),
    });
    renderer.set_hud_inventory_screen(Some(screen));
    renderer.set_hud_advancement_toasts(vec![HudAdvancementToast {
        frame_label: "Advancement Made!".to_string(),
        frame_label_color: 0xFF_FF_00,
        title_runs: vec![HudStyledTextRun::plain("A")],
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        block_model: Some(model),
        slot: 0,
        visible_portion: 1.0,
        fully_visible_ms: 0,
    }]);

    let count = |renderer: &Renderer| {
        let mut count = 0;
        renderer
            .visit_hud_block_item_models_for_stratum(HudScreenStratum::All, |_, _, _| count += 1);
        count
    };
    assert_eq!(count(&renderer), 3);
    renderer.set_hide_gui(true);
    assert_eq!(
        count(&renderer),
        2,
        "only the ordinary hotbar item is hidden; Screen and Toast items remain"
    );
    renderer.set_hide_gui(false);
    assert_eq!(
        count(&renderer),
        3,
        "retained hotbar item returns without another setter"
    );
}

#[test]
fn advancement_toast_sanitizes_slot_text_icon_and_slide() {
    let toast = sanitize_hud_advancement_toast(HudAdvancementToast {
        frame_label: "Challenge\u{0} Complete!".to_string(),
        frame_label_color: 0x12FF_88FF,
        title_runs: vec![HudStyledTextRun {
            shadow_color: None,
            text: "Styled title".to_string(),
            style: HudTextStyle {
                bold: true,
                ..HudTextStyle::default()
            },
            color: Some(0xAB12_3456),
        }],
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        block_model: None,
        slot: 4,
        visible_portion: 1.5,
        fully_visible_ms: 1_600,
    })
    .unwrap();
    assert_eq!(toast.frame_label, "Challenge Complete!");
    assert_eq!(toast.frame_label_color, 0xFF_88_FF);
    assert_eq!(toast.title_runs[0].color, Some(0x12_34_56));
    assert_eq!(toast.visible_portion, 1.0);
    assert!(sanitize_hud_advancement_toast(HudAdvancementToast { slot: 5, ..toast }).is_none());
}

#[test]
fn advancement_toast_uses_vanilla_slot_and_right_edge_slide() {
    let toast = HudAdvancementToast {
        frame_label: "Goal Reached!".to_string(),
        frame_label_color: 0xFF_FF_00,
        title_runs: vec![HudStyledTextRun::plain("A goal")],
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        block_model: None,
        slot: 3,
        visible_portion: 0.25,
        fully_visible_ms: 0,
    };
    assert_eq!(
        hud_advancement_toast_rect(PhysicalSize::new(320, 240), &toast),
        absolute_hud_rect(280.0, 96.0, 160, 32)
    );
    assert_eq!(
        hud_advancement_toast_icon_rect(PhysicalSize::new(320, 240), &toast),
        absolute_hud_rect(288.0, 104.0, 16, 16)
    );
}

#[test]
fn hud_block_item_quads_submit_resident_scissored_base_and_glint_draws() {
    let solid = crate::item_models::ItemModelQuad {
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
    };
    let mut translucent = solid;
    translucent.translucent = true;
    let model = test_hud_block_item_model_with_quads(
        vec![solid, translucent],
        glam::Mat4::IDENTITY,
        GuiItemLightingEntry::Items3d,
        true,
    );

    let mut frame = ItemModelResidencyFrame::default();
    let scissor = crate::item_models::ItemModelScissorRect::new(4, 5, 6, 7);
    append_hud_block_item_model_residency(
        &mut frame,
        &model,
        glam::Mat4::IDENTITY,
        ItemModelDrawTarget::Gui,
        scissor,
    );

    assert_eq!(frame.assets.len(), 2);
    assert_eq!(
        frame
            .assets
            .iter()
            .map(|asset| (asset.atlas, asset.geometry_class))
            .collect::<Vec<_>>(),
        [
            (ItemModelAtlasKind::Blocks, ItemModelGeometryClass::Solid),
            (
                ItemModelAtlasKind::Blocks,
                ItemModelGeometryClass::Translucent,
            ),
        ]
    );
    assert!(frame
        .assets
        .iter()
        .all(|asset| matches!(asset.source, ItemModelAssetSource::Quads(_))));
    assert_eq!(frame.instances.len(), 4);
    assert_eq!(
        frame
            .instances
            .iter()
            .map(|instance| instance.phase)
            .collect::<Vec<_>>(),
        [
            ItemModelDrawPhase::Solid,
            ItemModelDrawPhase::Glint,
            ItemModelDrawPhase::Translucent,
            ItemModelDrawPhase::GlintTranslucent,
        ]
    );
    assert!(frame.instances.iter().all(|instance| {
        instance.target == ItemModelDrawTarget::Gui
            && instance.scissor == scissor
            && instance.light == ITEM_MODEL_FULL_BRIGHT_LIGHT
    }));
}

#[test]
fn hud_resident_special_item_submits_gui_geometry_with_leaf_transform() {
    let texture = crate::entity_model_texture_refs()
        .iter()
        .copied()
        .find(|texture| texture.path == "textures/entity/bell/bell_body.png")
        .expect("bell texture is part of the static entity catalog");
    let leaf_transform = glam::Mat4::from_translation(glam::Vec3::new(0.25, 0.5, 0.75));
    let gui_display = glam::Mat4::from_scale(glam::Vec3::splat(2.0));
    let placement = glam::Mat4::from_translation(glam::Vec3::new(4.0, 5.0, 6.0));
    let model = HudBlockItemModel {
        layers: Arc::from([HudResidentItemLayer::Special {
            model: crate::item_models::SpecialItemModel::Bell {
                texture: crate::item_models::SpecialItemTexture::Static(texture),
            },
            transform: gui_display * leaf_transform,
        }]),
        lighting: GuiItemLightingEntry::Items3d,
        foil: true,
    };
    let mut frame = ItemModelResidencyFrame::default();
    let scissor = crate::item_models::ItemModelScissorRect::new(4, 5, 6, 7);

    append_hud_block_item_model_residency(
        &mut frame,
        &model,
        placement,
        ItemModelDrawTarget::Gui,
        scissor,
    );

    assert!(frame.assets.is_empty());
    assert!(frame.instances.is_empty());
    assert_eq!(frame.special_items.len(), 1);
    let special = &frame.special_items[0];
    assert_eq!(special.target, ItemModelDrawTarget::Gui);
    assert_eq!(special.foil, ItemModelFoil::Standard);
    assert_eq!(special.scissor, scissor);
    assert_eq!(special.transform, placement * gui_display * leaf_transform);
    assert!(hud_block_item_model_is_renderable(&model));
}

/// End-to-end GPU proof of the HUD 3D block-item consumer, now through the shared offscreen
/// whole-frame harness: a block item's quad seated at hotbar slot 4 (via the real
/// [`gui_item_slot_placement`]) renders in `render()`'s GUI item pass under the GUI ortho
/// camera, and the frame readback shows the slot center as the block's atlas color while a far
/// corner stays the clear color. Skips (no assertion) when no GPU adapter is available, so it
/// never fails the suite on adapter-less machines.
#[test]
fn hud_block_item_renders_visible_pixels_in_its_slot() {
    use crate::camera::ClearColor;
    use crate::item_models::{GuiItemLightingEntry, HudBlockItemModel, ItemModelQuad};
    use glam::{Mat4, Vec3, Vec4};

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    // A 1x1 opaque-red blocks atlas: every UV samples red, so the block icon is unambiguously
    // visible against the blue clear color.
    renderer
        .update_terrain_texture_atlas(&[255, 0, 0, 255])
        .expect("1x1 atlas");
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    // Seat the GUI ortho camera for the item pass (production writes it whenever the camera
    // updates; a fresh renderer has not had one yet).
    renderer.update_camera();

    // One full-slot front-facing quad at hotbar slot 4, centered in the slot exactly as
    // vanilla's display transform centers the model (`gui_display = T(-0.5)`).
    let slot = 4;
    let gui_display = Mat4::from_translation(Vec3::splat(-0.5));
    let quad = ItemModelQuad {
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
    };
    let mut models: Vec<Option<HudBlockItemModel>> = vec![None; HUD_HOTBAR_SLOTS];
    models[slot] = Some(test_hud_block_item_model_with_quads(
        vec![quad],
        gui_display,
        GuiItemLightingEntry::Items3d,
        false,
    ));
    renderer.set_hud_hotbar_block_item_models(models);
    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame::default())
        .unwrap();
    assert!(renderer.counters().item_model_vertex_upload_bytes > 0);
    assert!(renderer.counters().item_model_index_upload_bytes > 0);

    // The slot-center pixel (framebuffer col,row from top-left) is where the placement seats
    // the model origin; pixel (0,0) is far from the bottom-centered hotbar, so it stays
    // background.
    let placement =
        gui_item_slot_placement(hotbar_item_hud_rect(PhysicalSize::new(WIDTH, HEIGHT), slot));
    let center = placement * gui_display * Vec4::new(0.5, 0.5, 0.5, 1.0);
    let center_px = center.x.round() as u32;
    let center_py = center.y.round() as u32;
    assert!(
        center_px < WIDTH && center_py < HEIGHT,
        "slot center in bounds"
    );

    let pixels = renderer.render_offscreen_frame().expect("offscreen frame");
    let center_pixel = pixels.pixel(center_px, center_py);
    let corner_pixel = pixels.pixel(0, 0);

    // The slot center shows the red block icon (R high, B low); the far corner stays blue
    // background.
    assert!(
        center_pixel[0] > 128 && center_pixel[2] < 128,
        "slot center should show the block item, got {center_pixel:?}"
    );
    assert!(
        corner_pixel[2] > 128 && corner_pixel[0] < 128,
        "corner should stay background, got {corner_pixel:?}"
    );

    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame::default())
        .unwrap();
    assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
    assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
    assert_eq!(renderer.counters().item_instance_upload_bytes, 0);

    renderer.set_hud_hotbar_block_item_models(Vec::new());
    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame::default())
        .unwrap();
    let empty = renderer.render_offscreen_frame().expect("removed GUI item");
    assert_eq!(
        empty.pixel(center_px, center_py),
        corner_pixel,
        "removing the item must restore the background without retaining its pixels"
    );
}

#[test]
fn hud_block_item_gpu_scissor_clips_one_draw_and_resets_for_the_next() {
    use crate::camera::ClearColor;
    use crate::item_models::{ItemModelQuad, ItemModelResidencyFrame};
    use glam::{Mat4, Vec3};

    const WIDTH: u32 = 40;
    const HEIGHT: u32 = 20;
    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        return;
    };
    renderer
        .update_terrain_texture_atlas(&[255, 0, 0, 255])
        .expect("1x1 red atlas");
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.update_camera();

    let quad = ItemModelQuad {
        corners: [
            [0.0, 0.0, 8.0],
            [16.0, 0.0, 8.0],
            [16.0, 16.0, 8.0],
            [0.0, 16.0, 8.0],
        ],
        uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        tint: [1.0; 4],
        normal: [0.0, 0.0, 1.0],
        shade: 1.0,
        translucent: false,
    };
    let model = || {
        test_hud_block_item_model_with_quads(
            vec![quad],
            Mat4::from_translation(Vec3::splat(-0.5)),
            GuiItemLightingEntry::Items3d,
            false,
        )
    };
    let floating_item = |x, scissor| HudInventoryItem {
        x,
        y: 0,
        scale: 1.0,
        scale_y: 1.0,
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        scissor,
        draw_decorations: false,
        block_model: Some(model()),
    };
    renderer.set_hud_inventory_screen(Some(HudInventoryScreen {
        width: WIDTH,
        height: HEIGHT,
        background_layers: Vec::new(),
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: vec![
            floating_item(
                0,
                Some(HudInventoryItemScissor {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                }),
            ),
            floating_item(16, None),
        ],
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
    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame::default())
        .unwrap();

    let pixels = renderer.render_offscreen_frame().expect("offscreen frame");
    let clipped_inside = pixels.pixel(4, 8);
    let clipped_outside = pixels.pixel(12, 8);
    let reset_inside = pixels.pixel(24, 8);
    assert!(
        clipped_inside[0] > 128 && clipped_inside[2] < 128,
        "inside clipped item should be red, got {clipped_inside:?}"
    );
    assert!(
        clipped_outside[2] > 128 && clipped_outside[0] < 128,
        "outside scissor should stay blue, got {clipped_outside:?}"
    );
    assert!(
        reset_inside[0] > 128 && reset_inside[2] < 128,
        "later unscissored item should see the restored full surface, got {reset_inside:?}"
    );
}

#[test]
fn command_suggestions_sanitize_row_budget_and_rich_tooltip() {
    let state = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_text: format!("{}\n", "x".repeat(300)),
        cursor: usize::MAX,
        selection: usize::MAX,
        suffix: Some("\nnext".to_string()),
        list_width: u32::MAX,
        list_height: u32::MAX,
        entries: (0..12)
            .map(|index| HudCommandSuggestionEntry {
                runs: vec![HudStyledTextRun::plain(format!("entry{index}"))],
                selected: index == 11,
            })
            .collect(),
        tooltip: Some(HudCommandSuggestionTooltip {
            runs: vec![HudStyledTextRun {
                shadow_color: None,
                text: "Rich\nhelp".to_string(),
                style: HudTextStyle {
                    bold: true,
                    underlined: true,
                    ..Default::default()
                },
                color: Some(0xFF_12_34_56),
            }],
            x: 12,
            y: 34,
        }),
        ..Default::default()
    });

    assert_eq!(state.input_text.chars().count(), 256);
    assert_eq!(state.cursor, 256);
    assert_eq!(state.selection, 256);
    assert_eq!(state.suffix.as_deref(), Some("next"));
    assert_eq!(state.list_width, 4096);
    assert_eq!(state.list_height, 120);
    assert_eq!(state.entries.len(), 10);
    let tooltip = state.tooltip.expect("valid tooltip");
    assert_eq!(tooltip.runs[0].text, "Richhelp");
    assert!(tooltip.runs[0].style.bold);
    assert!(tooltip.runs[0].style.underlined);
    assert_eq!(tooltip.runs[0].color, Some(0x12_34_56));
}

#[test]
fn chat_draft_input_uses_vanilla_gray_italic_style_only_for_chat_layout() {
    let chat = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_text: "draft".to_string(),
        draft: true,
        ..HudCommandSuggestions::default()
    });
    let runs = hud_command_input_runs(&chat);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "draft");
    assert!(runs[0].style.italic);
    assert_eq!(runs[0].color, Some(0xAAAAAA));

    let command_block = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_text: "draft".to_string(),
        draft: true,
        layout: HudCommandSuggestionsLayout::CommandBlock,
        ..HudCommandSuggestions::default()
    });
    assert!(!command_block.draft);
    assert!(hud_command_input_runs(&command_block).is_empty());
}

#[test]
fn command_block_suggestions_sanitize_utf16_and_seven_row_budget() {
    let state = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_text: "😀".repeat(20_000),
        cursor: usize::MAX,
        selection: usize::MAX,
        list_height: u32::MAX,
        entries: (0..12)
            .map(|index| HudCommandSuggestionEntry {
                runs: vec![HudStyledTextRun::plain(format!("entry{index}"))],
                selected: false,
            })
            .collect(),
        layout: HudCommandSuggestionsLayout::CommandBlock,
        ..Default::default()
    });

    assert_eq!(state.input_text.encode_utf16().count(), 32_500);
    assert_eq!(state.cursor, 16_250);
    assert_eq!(state.selection, 16_250);
    assert_eq!(state.list_height, 84);
    assert_eq!(state.entries.len(), 7);
}

#[test]
fn game_mode_switcher_offscreen_frame_draws_flat_item_icons_inside_slots() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.update_camera();
    renderer
        .upload_item_atlas(1, 1, &[0, 255, 0, 255])
        .expect("item atlas");
    renderer
        .update_item_atlas_patches(&[bbb_render_types::AtlasMipPatch {
            level: 0,
            rect: bbb_render_types::AtlasRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            rgba: vec![255, 0, 0, 255],
        }])
        .expect("animate item icon");
    let icon = HudItemIcon::single(HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    });
    let slot = |mode, x, icon| HudDebugGameModeSwitcherSlot {
        mode,
        x,
        y: 89,
        width: 26,
        height: 26,
        selected: false,
        icon,
        block_model: None,
    };
    renderer.set_hud_debug_overlay(Some(HudDebugOverlay {
        game_mode_switcher: Some(HudDebugGameModeSwitcher {
            selected: GameType::Creative,
            title: "Creative Mode".to_string(),
            help_runs: vec![HudStyledTextRun::plain("F4 Next")],
            background_x: 98,
            background_y: 62,
            background_width: 125,
            background_height: 75,
            slots: vec![
                slot(GameType::Creative, 101, None),
                slot(GameType::Survival, 132, Some(icon)),
                slot(GameType::Adventure, 163, None),
                slot(GameType::Spectator, 194, None),
            ],
        }),
        ..HudDebugOverlay::default()
    }));

    let pixels = renderer
        .render_offscreen_frame()
        .expect("game mode switcher icon frame");
    let icon_pixel = pixels.pixel(145, 102);
    assert!(
        icon_pixel[0] > 128 && icon_pixel[1] < 128 && icon_pixel[2] < 128,
        "flat item icon should draw red inside slot, got {icon_pixel:?}"
    );
}

#[test]
fn hud_debug_profiler_chart_layout_matches_vanilla_right_anchor() {
    assert_eq!(
        hud_debug_profiler_chart_layout(PhysicalSize::new(640, 360), 2, 1, 10),
        HudDebugProfilerChartLayout {
            left: 370,
            right: 630,
            chart_center_x: 500,
            chart_center_y: 260,
            text_start_y: 327,
            current_node_top: 198,
            bottom: 345,
        }
    );
    assert_eq!(
        hud_debug_profiler_bottom_offset(&HudDebugOverlay {
            profiler_chart: Some(HudDebugProfilerChart::default()),
            fps_chart: Some(HudDebugFrameTimeChart::default()),
            ..HudDebugOverlay::default()
        }),
        69
    );
}

#[test]
fn hud_debug_profiler_helpers_match_vanilla_result_field_rules() {
    assert_eq!(
        hud_debug_profiler_demangle_path("root\u{001e}tick"),
        "root.tick"
    );
    assert_eq!(hud_debug_profiler_percentage_text(12.345), "12.35%");
    assert_eq!(hud_debug_profiler_slice_steps(16.0), 5);
    assert_eq!(hud_debug_profiler_slice_argb("tick"), 0xFF66_44CC);
    assert_eq!(hud_argb_multiply(0xFF66_44CC, 0xFF80_8080), 0xFF33_2266);
}

#[test]
fn hud_debug_profiler_split_node_name_wraps_dot_segments() {
    let mut glyphs = HudFontGlyphMap::new();
    for ch in ['?', 'a', 'b', '.'] {
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
        hud_debug_profiler_split_node_name("a.b", 12, 12, &glyphs),
        vec!["a".to_string(), ".b".to_string()]
    );
}

#[test]
fn hud_debug_crosshair_axis_length_matches_vanilla_scale_projection() {
    let length = hud_debug_crosshair_axis_length(
        PhysicalSize::new(320, 240),
        HudDebugCrosshair {
            x_rot_degrees: 0.0,
            y_rot_degrees: 0.0,
            gui_scale: 1,
        },
    );
    let expected = 0.01 * (1.0 / 35.0_f32.to_radians().tan()) * 120.0;
    assert!((length - expected).abs() < 1e-6);
}

#[test]
fn hud_debug_crosshair_axes_apply_vanilla_scale_and_rotation_order() {
    let straight = hud_debug_crosshair_axis_vectors(HudDebugCrosshair {
        x_rot_degrees: 0.0,
        y_rot_degrees: 0.0,
        gui_scale: 1,
    });
    assert_close2(straight[0], [-1.0, -0.0]);
    assert_close2(straight[1], [0.0, -1.0]);
    assert_close2(straight[2], [0.0, -0.0]);

    let pitched = hud_debug_crosshair_axis_vectors(HudDebugCrosshair {
        x_rot_degrees: 90.0,
        y_rot_degrees: 0.0,
        gui_scale: 1,
    });
    assert_close2(pitched[1], [0.0, 0.0]);
    assert_close2(pitched[2], [0.0, -1.0]);
}

#[test]
fn hud_debug_crosshair_line_corners_expand_around_segment_centerline() {
    assert_eq!(
        hud_debug_crosshair_line_corners([10.0, 10.0], [14.0, 10.0], 4.0),
        Some([[10.0, 12.0], [10.0, 8.0], [14.0, 8.0], [14.0, 12.0]])
    );
    assert_eq!(
        hud_debug_crosshair_line_corners([10.0, 10.0], [10.0, 10.0], 4.0),
        None
    );
}

#[test]
fn hud_debug_lightmap_preview_rect_matches_vanilla_bottom_right() {
    let surface = PhysicalSize::new(320, 240);
    assert_eq!(
        hud_debug_lightmap_preview_rect(surface),
        absolute_hud_rect(254.0, 174.0, 64, 64)
    );
    assert_eq!(
        hud_debug_lightmap_preview_border_rect(surface),
        absolute_hud_rect(253.0, 173.0, 66, 66)
    );
}

#[test]
fn hud_debug_lightmap_preview_uv_matches_vanilla_flipped_blit() {
    assert_eq!(
        hud_debug_lightmap_preview_uv(),
        HudUvRect {
            min: [0.0, 1.0],
            max: [1.0, 0.0],
        }
    );
}

#[test]
fn hud_debug_fps_chart_width_matches_vanilla_capacity_and_half_screen_cap() {
    assert_eq!(
        hud_debug_chart_width(PhysicalSize::new(800, 240)),
        HUD_DEBUG_CHART_SAMPLE_CAPACITY as u32 + 2
    );
    assert_eq!(hud_debug_chart_width(PhysicalSize::new(320, 240)), 160);
}

#[test]
fn hud_debug_fps_chart_sample_height_matches_vanilla_millis_scale() {
    assert_eq!(hud_debug_fps_chart_sample_height(16_666_667), 30);
    assert_eq!(hud_debug_fps_chart_sample_height(33_333_333), 60);
    assert_eq!(hud_debug_fps_configured_framerate_height(120), 15);
    assert_eq!(hud_debug_fps_chart_display_string(16_666_667.0), "17 ms");
}

#[test]
fn hud_debug_fps_chart_sample_tint_matches_vanilla_threshold_colors() {
    assert_eq!(hud_debug_fps_chart_sample_tint(0), [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(
        hud_debug_fps_chart_sample_tint(28_000_000),
        [1.0, 1.0, 0.0, 1.0]
    );
    assert_eq!(
        hud_debug_fps_chart_sample_tint(56_000_000),
        [1.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn hud_debug_tps_chart_sample_rules_match_vanilla_mspt_scale() {
    assert_eq!(hud_debug_tps_chart_sample_height(25_000_000, 50.0), 30);
    assert_eq!(hud_debug_tps_chart_sample_height(50_000_000, 50.0), 60);
    assert_eq!(hud_debug_tps_chart_display_string(16_666_667.0), "17 ms");
    assert_eq!(hud_debug_tps_chart_tps_label(50.0), "20.0 TPS");
    assert_eq!(
        hud_debug_tps_chart_sample_tint(50_000_000, 50.0),
        [0.0, 1.0, 0.0, 1.0]
    );
    assert_eq!(
        hud_debug_tps_chart_sample_tint(56_250_000, 50.0),
        [1.0, 1.0, 0.0, 1.0]
    );
    assert_eq!(
        hud_debug_tps_chart_sample_tint(62_500_000, 50.0),
        [1.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn hud_debug_tps_chart_aggregation_and_other_time_match_vanilla_dimensions() {
    let sample = HudDebugTpsSample {
        full_tick_nanos: 70_000_000,
        tick_server_method_nanos: 30_000_000,
        scheduled_tasks_nanos: 10_000_000,
        idle_nanos: 5_000_000,
    };
    assert_eq!(hud_debug_tps_chart_aggregation_nanos(sample), 65_000_000);
    assert_eq!(hud_debug_tps_chart_other_nanos(sample), 25_000_000);

    let saturated = HudDebugTpsSample {
        full_tick_nanos: 20_000_000,
        tick_server_method_nanos: 30_000_000,
        scheduled_tasks_nanos: 10_000_000,
        idle_nanos: 5_000_000,
    };
    assert_eq!(hud_debug_tps_chart_other_nanos(saturated), 0);
}

#[test]
fn hud_debug_ping_chart_sample_rules_match_vanilla_thresholds() {
    assert_eq!(hud_debug_ping_chart_sample_height(250), 30);
    assert_eq!(hud_debug_ping_chart_sample_height(500), 60);
    assert_eq!(hud_debug_ping_chart_display_string(41.6), "42 ms");
    assert_eq!(hud_debug_ping_chart_sample_tint(0), [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(hud_debug_ping_chart_sample_tint(250), [1.0, 1.0, 0.0, 1.0]);
    assert_eq!(hud_debug_ping_chart_sample_tint(500), [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn hud_debug_bandwidth_chart_sample_rules_match_vanilla_thresholds() {
    assert_eq!(
        hud_debug_bandwidth_chart_display_string_internal(64.0),
        "64 B/s"
    );
    assert_eq!(
        hud_debug_bandwidth_chart_display_string_internal(1_024.0),
        "1.0 KiB/s"
    );
    assert_eq!(
        hud_debug_bandwidth_chart_display_string_internal(1_048_576.0),
        "1.0 MiB/s"
    );
    assert_eq!(hud_debug_bandwidth_chart_sample_height_internal(0.0), 0);
    assert_eq!(
        hud_debug_bandwidth_chart_sample_height_internal(1_048_576.0),
        60
    );
    assert_eq!(
        hud_debug_bandwidth_chart_sample_tint(0),
        [0.0, 1.0, 1.0, 1.0]
    );
    assert_eq!(
        hud_debug_bandwidth_chart_sample_tint(524_288),
        [1.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn hud_item_icon_draw_steps_match_vanilla_item_decoration_order() {
    // Vanilla `GuiGraphicsExtractor.itemDecorations` calls `itemBar`,
    // `itemCooldown`, then `itemCount`, after the item sprite itself has
    // already been submitted to the GUI item atlas.
    let icon = HudItemIcon {
        pop: 0.0,
        lighting: GuiItemLightingEntry::ItemsFlat,
        layers: vec![HudIconLayer::new(
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            HUD_TINT_WHITE,
        )],
        foil: HudItemFoil::Standard,
        count_label: Some(HudItemCountLabel::new("64")),
        durability_bar: Some(HudItemDurabilityBar::new(13, [1.0, 0.0, 0.0])),
        cooldown_progress: Some(0.5),
    };

    let mut steps = Vec::new();
    for_each_hud_item_icon_draw_step(&icon, true, true, |step| steps.push(step));
    assert_eq!(
        steps,
        vec![
            HudItemIconDrawStep::Layers,
            HudItemIconDrawStep::Glint,
            HudItemIconDrawStep::DurabilityBar,
            HudItemIconDrawStep::Cooldown,
            HudItemIconDrawStep::CountLabel,
        ]
    );

    let mut steps = Vec::new();
    for_each_hud_item_icon_draw_step(&icon, false, true, |step| steps.push(step));
    assert_eq!(
        steps,
        vec![
            HudItemIconDrawStep::DurabilityBar,
            HudItemIconDrawStep::Cooldown,
            HudItemIconDrawStep::CountLabel,
        ]
    );
}

#[test]
fn hud_special_foil_glint_uv_uses_gui_sheeted_decal_scale() {
    // Vanilla `ItemFeatureRenderer.computeFoilDecalPose` scales GUI SPECIAL foil poses by 0.5
    // before `SheetedDecalTextureGenerator` applies its 1/128 texture scale.
    assert_eq!(hud_item_special_foil_glint_uv([0.0, 0.0]), [0.0, -0.0]);
    assert_eq!(
        hud_item_special_foil_glint_uv([1.0, 0.0]),
        [2.0 / 128.0, -0.0]
    );
    assert_eq!(
        hud_item_special_foil_glint_uv([1.0, 1.0]),
        [2.0 / 128.0, -2.0 / 128.0]
    );
}

#[test]
fn hud_digit_text_width_uses_digit_advances_only() {
    let mut glyphs = [HudDigitGlyph::default(); 10];
    glyphs[4].advance = 6.0;
    glyphs[6].advance = 6.0;

    assert_eq!(hud_digit_text_width("64", &glyphs), Some(12));
    assert_eq!(hud_digit_text_width("1x", &glyphs), None);
    assert_eq!(hud_digit_text_width("", &glyphs), None);
}

#[test]
fn hud_font_text_width_uses_glyph_map_with_replacement_fallback() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance, bearing_top) in [('A', 6, 7.0), (' ', 4, 7.0), ('?', 5, 7.0), ('é', 7, 10.0)]
    {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: advance as f32,
                bearing_top,
                ..HudAsciiGlyph::default()
            },
        );
    }

    assert_eq!(hud_font_text_width("A A", &glyphs), Some(16));
    assert_eq!(hud_font_text_width("A\u{0007}", &glyphs), Some(11));
    // Mapped non-ASCII codepoints now resolve their own bitmap glyph.
    assert_eq!(hud_font_text_width("é", &glyphs), Some(7));
    assert_eq!(hud_font_glyph('é', &glyphs).bearing_top, 10.0);
    // This small fixture deliberately has no unihex provider, so CJK
    // degrades to the `?` replacement glyph.
    assert_eq!(hud_font_text_width("钻", &glyphs), Some(5));
    assert_eq!(hud_font_glyph('钻', &glyphs), hud_font_glyph('?', &glyphs));
    assert_eq!(hud_font_text_width("", &glyphs), Some(0));
}

#[test]
fn hud_font_text_width_styled_adds_bold_offset_per_glyph() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('a', 6), ('b', 6), ('?', 5)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: advance as f32,
                ..HudAsciiGlyph::default()
            },
        );
    }

    // Vanilla Font.width: bold adds getBoldOffset() (1) per glyph, so bold
    // "ab" is the plain width plus one pixel per character.
    let plain = hud_font_text_width("ab", &glyphs).unwrap();
    let bold = hud_font_text_width_styled(
        "ab",
        &glyphs,
        HudTextStyle {
            bold: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(plain, 12);
    assert_eq!(bold, plain + 2);
    // The default style keeps the existing plain-advance width unchanged.
    assert_eq!(
        hud_font_text_width_styled("ab", &glyphs, HudTextStyle::default()),
        hud_font_text_width("ab", &glyphs)
    );
    // Non-bold flags leave the width alone (obfuscated is equal-advance).
    assert_eq!(
        hud_font_text_width_styled(
            "ab",
            &glyphs,
            HudTextStyle {
                italic: true,
                underlined: true,
                strikethrough: true,
                obfuscated: true,
                ..Default::default()
            },
        ),
        Some(plain)
    );
}

#[test]
fn inventory_text_input_layout_scrolls_to_cursor_with_width_budget() {
    let glyphs = styled_test_glyphs();
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: 12,
        text: "aaaaa".to_string(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    };

    let layout = utf16_widget_input_layout(
        &label,
        HudInventoryTextInputDecoration {
            cursor: 5,
            selection: 5,
            scroll_to: 5,
            max_length: 50,
            cursor_visible: true,
            cursor_tint: HUD_TINT_WHITE,
        },
        &glyphs,
    );

    assert_eq!(layout.displayed_text, "aa");
    assert!(layout.cursor_on_screen);
    assert!(!layout.insert_cursor);
    assert_eq!(layout.cursor_x, 13.0);
    assert_eq!(layout.selection_rect, None);
}

#[test]
fn inventory_text_input_layout_highlights_visible_selection_prefix() {
    let glyphs = styled_test_glyphs();
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: 12,
        text: "aaaaa".to_string(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    };

    let layout = utf16_widget_input_layout(
        &label,
        HudInventoryTextInputDecoration {
            cursor: 5,
            selection: 0,
            scroll_to: 0,
            max_length: 50,
            cursor_visible: true,
            cursor_tint: HUD_TINT_WHITE,
        },
        &glyphs,
    );

    assert_eq!(layout.displayed_text, "aa");
    assert!(!layout.cursor_on_screen);
    assert_eq!(layout.selection_rect, Some((-1.0, 13)));
}

#[test]
fn action_bar_offscreen_pixels_follow_selected_rtl_visual_order() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let mut glyphs = HudFontGlyphMap::new();
    for (index, ch) in ['1', '2', '3', '-'].into_iter().enumerate() {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [index as f32 / 4.0, 0.0],
                    max: [(index + 1) as f32 / 4.0, 1.0],
                },
                width: 1,
                height: 1,
                advance: 1.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    renderer
        .upload_hud_font_atlas(
            4,
            1,
            &[
                255, 0, 0, 255, // 1
                0, 255, 0, 255, // 2
                0, 0, 255, 255, // 3
                255, 255, 0, 255, // -
            ],
            glyphs,
        )
        .expect("font atlas");
    renderer.set_default_text_right_to_left(true);
    renderer.set_hud_action_bar_text(Some(HudActionBarText {
        runs: vec![HudComponentTextRun::plain("123-")],
        remaining_ticks: 60,
        partial_tick: 0.0,
        animate_color: false,
    }));

    let pixels = renderer
        .render_offscreen_frame()
        .expect("offscreen bidi action-bar frame");
    let visual = (158..162).map(|x| pixels.pixel(x, 168)).collect::<Vec<_>>();
    assert!(
        visual[0][0] > 200 && visual[0][1] > 200,
        "hyphen must draw first as yellow, got {visual:?}"
    );
    assert!(
        visual[1][0] > 200 && visual[1][1] < 80,
        "one must follow as red, got {visual:?}"
    );
    assert!(
        visual[2][1] > 200 && visual[2][0] < 80,
        "two must follow as green, got {visual:?}"
    );
    assert!(
        visual[3][2] > 200 && visual[3][0] < 80,
        "three must finish as blue, got {visual:?}"
    );
}

#[test]
fn experience_level_projection_gates_on_a_positive_level() {
    // Vanilla `Gui` draws the level only when `experienceLevel > 0`.
    assert_eq!(hud_experience_level_projection(Some(30)), Some(30));
    assert_eq!(hud_experience_level_projection(Some(1)), Some(1));
    assert_eq!(hud_experience_level_projection(Some(0)), None);
    assert_eq!(hud_experience_level_projection(Some(-4)), None);
    assert_eq!(hud_experience_level_projection(None), None);
}

#[test]
fn experience_level_outline_passes_match_vanilla_offsets_and_colors() {
    // ContextualBarRenderer: four black axis-offset copies drawn
    // in `(+1,0),(-1,0),(0,+1),(0,-1)` order, then the `0x80FF20` green
    // center last, so the fill sits on top of the outline.
    const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    const GREEN: [f32; 4] = [128.0 / 255.0, 255.0 / 255.0, 32.0 / 255.0, 1.0];
    assert_eq!(
        HUD_EXPERIENCE_LEVEL_PASSES,
        [
            (1.0, 0.0, BLACK),
            (-1.0, 0.0, BLACK),
            (0.0, 1.0, BLACK),
            (0.0, -1.0, BLACK),
            (0.0, 0.0, GREEN),
        ]
    );
    assert!(HUD_EXPERIENCE_LEVEL_PASSES[..4]
        .iter()
        .all(|pass| pass.2 == BLACK));
    assert_eq!(HUD_EXPERIENCE_LEVEL_PASSES[4].2, GREEN);
}

#[test]
fn boss_bar_names_round_trip_the_vanilla_getname_vocabularies() {
    // Vanilla `BossEvent.BossBarColor`/`BossBarOverlay` getName strings
    // (122-127) — the same names the world stores.
    for color in HudBossBarColor::ALL {
        assert_eq!(HudBossBarColor::from_name(color.name()), Some(color));
    }
    assert_eq!(HudBossBarColor::from_name("magenta"), None);
    for overlay in [HudBossBarOverlay::Progress]
        .into_iter()
        .chain(HudBossBarOverlay::NOTCHED)
    {
        assert_eq!(HudBossBarOverlay::from_name(overlay.name()), Some(overlay));
    }
    assert_eq!(HudBossBarOverlay::from_name("notched_8"), None);

    // Notched sprite arrays index by `ordinal() - 1`
    // Progress has no notched sheet.
    assert_eq!(HudBossBarOverlay::Progress.notched_index(), None);
    assert_eq!(HudBossBarOverlay::Notched6.notched_index(), Some(0));
    assert_eq!(HudBossBarOverlay::Notched20.notched_index(), Some(3));
}

#[test]
fn boss_bar_draws_stack_rows_center_names_and_truncate() {
    let glyphs = styled_test_glyphs();
    let surface = PhysicalSize::new(320, 240);
    let bars = vec![
        HudBossBar {
            name_runs: vec![HudStyledTextRun::plain("ab")],
            progress: 1.0,
            color: HudBossBarColor::Purple,
            overlay: HudBossBarOverlay::Progress,
        };
        6
    ];
    let draws = hud_boss_bar_draws(&bars, &glyphs, surface);
    // guiHeight / 3 = 80: rows 12, 31, 50, 69 survive, bars 5-6 drop.
    assert_eq!(
        draws.iter().map(|draw| draw.y).collect::<Vec<_>>(),
        vec![12, 31, 50, 69]
    );
    // Name: centered `(guiWidth/2 - width/2, y - 9)` ("ab" is 12px), at
    // scale 1 in opaque white (vanilla colour -1 with drop shadow).
    assert_eq!(draws[0].name.origin, (154.0, 3.0));
    assert_eq!(draws[1].name.origin, (154.0, 22.0));
    assert_eq!(draws[0].name.scale, 1.0);
    assert_eq!(draws[0].name.tint, HUD_TINT_WHITE);
    // Full progress fills the whole sheet.
    assert_eq!(draws[0].layers.last().unwrap().width, 182);
    assert!(hud_boss_bar_draws(&[], &glyphs, surface).is_empty());
}

#[test]
fn cartography_map_preview_commands_preserve_duplicate_and_locked_strata() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        eprintln!("skipping offscreen renderer test without a compatible adapter");
        return;
    };
    renderer
        .add_test_gui_sprite(
            "container/cartography_table/duplicated_map",
            1,
            1,
            &[255; 4],
        )
        .unwrap();
    renderer
        .add_test_gui_sprite("container/cartography_table/map", 1, 1, &[255; 4])
        .unwrap();
    renderer
        .add_test_gui_sprite("container/cartography_table/locked", 1, 1, &[255; 4])
        .unwrap();

    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.background_layers = vec![
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite(
                "container/cartography_table/duplicated_map",
            ),
            x: 83,
            y: 13,
            width: 50,
            height: 66,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite(
                "container/cartography_table/duplicated_map",
            ),
            x: 67,
            y: 29,
            width: 50,
            height: 66,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
    ];
    screen.map_previews = vec![
        HudInventoryMapPreview {
            map_id: 7,
            x: 86,
            y: 16,
            scale: 0.34,
            after_background_layer: 0,
        },
        HudInventoryMapPreview {
            map_id: 7,
            x: 70,
            y: 32,
            scale: 0.34,
            after_background_layer: 1,
        },
    ];
    renderer.set_hud_inventory_screen(Some(screen));
    let draws = renderer.collect_hud_draws();
    assert!(matches!(draws.commands[0], HudDrawCommand::Sprite { .. }));
    assert!(matches!(
        draws.commands[1],
        HudDrawCommand::MapPreview { preview_index: 0 }
    ));
    assert!(matches!(draws.commands[2], HudDrawCommand::Sprite { .. }));
    assert!(matches!(
        draws.commands[3],
        HudDrawCommand::MapPreview { preview_index: 1 }
    ));

    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.background_layers = vec![
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("container/cartography_table/map"),
            x: 67,
            y: 13,
            width: 66,
            height: 66,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("container/cartography_table/locked"),
            x: 118,
            y: 60,
            width: 10,
            height: 14,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
    ];
    screen.map_previews = vec![HudInventoryMapPreview {
        map_id: 7,
        x: 71,
        y: 17,
        scale: 0.45,
        after_background_layer: 0,
    }];
    renderer.set_hud_inventory_screen(Some(screen));
    let draws = renderer.collect_hud_draws();
    assert!(matches!(draws.commands[0], HudDrawCommand::Sprite { .. }));
    assert!(matches!(
        draws.commands[1],
        HudDrawCommand::MapPreview { preview_index: 0 }
    ));
    assert!(matches!(draws.commands[2], HudDrawCommand::Sprite { .. }));
}

#[test]
fn cartography_map_preview_offscreen_draws_pixels_decorations_names_before_lock_overlay() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        eprintln!("skipping offscreen renderer test without a compatible adapter");
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer
        .add_test_gui_sprite("container/cartography_table/map", 1, 1, &[0, 0, 255, 255])
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "container/cartography_table/locked",
            1,
            1,
            &[0, 255, 0, 255],
        )
        .unwrap();

    let map_transform = Mat4::from_translation(Vec3::new(143.0, 54.0, 0.0))
        * Mat4::from_scale(Vec3::new(0.45, 0.45, 1.0));
    renderer.set_map_surfaces_with_gui(
        Vec::new(),
        Vec::new(),
        vec![crate::item_models::ItemFrameMapTexture {
            map_id: 7,
            owner_generation: 1,
            content_revision: 1,
            rgba: Arc::from([255, 0, 0, 255].repeat(128 * 128)),
            patch: None,
        }],
        Vec::new(),
        Vec::new(),
        vec![crate::item_models::HudMapPreviewSurface {
            preview_index: 0,
            surface: crate::item_models::bake_item_frame_map_surface(
                7,
                map_transform,
                crate::item_models::ITEM_MODEL_FULL_BRIGHT_LIGHT,
            ),
        }],
    );
    let decoration = crate::item_models::bake_hud_map_decoration_surface(
        1,
        -80,
        -80,
        0,
        0,
        map_transform,
        crate::item_models::ITEM_MODEL_FULL_BRIGHT_LIGHT,
        1,
    )
    .unwrap();
    renderer.set_map_decoration_surfaces_with_gui(
        Vec::new(),
        Vec::new(),
        vec![crate::item_models::ItemFrameMapDecorationTexture {
            sprite_id: decoration.submission.texture.sprite_id.to_string(),
            width: 1,
            height: 1,
            rgba: vec![255, 255, 0, 255],
        }],
        Vec::new(),
        Vec::new(),
        vec![crate::item_models::HudMapPreviewSurface {
            preview_index: 0,
            surface: decoration,
        }],
    );
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'A',
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            width: 5,
            height: 8,
            advance: 6.0,
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        },
    );
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], renderer.pack.hud_font_book.clone())
        .unwrap();
    let text = crate::item_models::bake_hud_map_text_surface(
        1,
        &[crate::WorldTextRun::plain("A")],
        40,
        40,
        1,
        map_transform,
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

    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.background_layers = vec![
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("container/cartography_table/map"),
            x: 67,
            y: 13,
            width: 66,
            height: 66,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("container/cartography_table/locked"),
            x: 118,
            y: 60,
            width: 10,
            height: 14,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
    ];
    screen.map_previews = vec![HudInventoryMapPreview {
        map_id: 7,
        x: 71,
        y: 17,
        scale: 0.45,
        after_background_layer: 0,
    }];
    renderer.set_hud_inventory_screen(Some(screen));
    renderer.update_camera();
    assert!(renderer
        .pack
        .map_surface_residency
        .dynamic_draws
        .iter()
        .any(|draw| draw.target == crate::item_models::MapSurfaceTarget::Gui(0)));
    assert!(renderer
        .pack
        .map_surface_residency
        .text_draws
        .iter()
        .any(|draw| draw.target == crate::item_models::MapSurfaceTarget::Gui(0)));
    assert!(renderer
        .collect_hud_draws()
        .commands
        .iter()
        .any(|command| matches!(command, HudDrawCommand::MapPreview { preview_index: 0 })));

    let pixels = renderer.render_offscreen_frame().unwrap();
    let map_pixel = pixels.pixel(148, 59);
    let locked_pixel = pixels.pixel(195, 103);
    assert!(
        map_pixel[0] > 128 && map_pixel[1] < 96 && map_pixel[2] < 96,
        "map pixels should cover the static frame, got {map_pixel:?}"
    );
    assert!(
        locked_pixel[1] > 128 && locked_pixel[0] < 96,
        "locked overlay must cover the map stack, got {locked_pixel:?}"
    );
    let mut yellow = 0usize;
    let mut white = 0usize;
    for y in 54..112 {
        for x in 143..201 {
            let pixel = pixels.pixel(x, y);
            if pixel[0] > 128 && pixel[1] > 128 && pixel[2] < 96 {
                yellow += 1;
            }
            if pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 180 {
                white += 1;
            }
        }
    }
    assert!(yellow > 0, "decoration sprite should cover map pixels");
    assert!(white > 0, "decoration name should cover map pixels");
}

#[test]
fn enchanting_book_pip_offscreen_pixels_precede_highlighted_option_stratum() {
    use crate::entity_models::EntityModelTextureImage;

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer
        .upload_gui_texture(
            "textures/gui/container/enchanting_table.png",
            1,
            1,
            &[0, 0, 255, 255],
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "container/enchanting_table/enchantment_slot_highlighted",
            1,
            1,
            &[255, 0, 0, 255],
        )
        .unwrap();
    let book_images = crate::entity_model_texture_refs()
        .iter()
        .filter(|texture| {
            texture
                .path
                .starts_with("textures/entity/enchantment/enchanting_table_book.png")
        })
        .map(|texture| {
            EntityModelTextureImage::new(
                *texture,
                [0, 255, 0, 255]
                    .repeat(usize::try_from(texture.size[0] * texture.size[1]).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    renderer.upload_entity_model_textures(&book_images).unwrap();

    let mut entity = EntityModelInstance::enchanting_book(-1, [-0.5, -0.75, -0.5], 0.0)
        .with_block_entity(|s| {
            s.book_progress = 0.0;
            s.book_open = 1.0;
            s.book_page_flip_1 = 0.1;
            s.book_page_flip_2 = 0.9;
            s.book_float_y = 0.0;
        });
    entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    let gui_base = glam::Quat::from_rotation_y(std::f32::consts::PI)
        * glam::Quat::from_rotation_x(25.0_f32.to_radians());
    let rotation = gui_base
        * glam::Quat::from_rotation_y(-90.0_f32.to_radians())
        * glam::Quat::from_rotation_x(std::f32::consts::PI)
        * glam::Quat::from_rotation_z(-80.0_f32.to_radians());
    let preview = HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: 14,
            y: 14,
            width: 38,
            height: 31,
        },
        scissor: None,
        translation: [0.0, 0.0375, 0.0],
        rotation: rotation.to_array(),
        override_camera_rotation: None,
        scale: 40.0,
        depth_isolated: true,
        after_background_layer: Some(0),
        item_layers: Vec::new(),
    };
    let mut screen = hud_inventory_screen_with_entity_previews(vec![preview]);
    screen.background_layers = vec![
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Texture(
                "textures/gui/container/enchanting_table.png",
            ),
            x: 0,
            y: 0,
            width: 176,
            height: 166,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [176.0 / 256.0, 166.0 / 256.0],
            },
        },
        HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite(
                "container/enchanting_table/enchantment_slot_highlighted",
            ),
            x: 60,
            y: 14,
            width: 108,
            height: 19,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
    ];
    renderer.set_hud_inventory_screen(Some(screen));

    let pixels = renderer.render_offscreen_frame().unwrap();
    let origin_x = (320 - 176) / 2;
    let origin_y = (240 - 166) / 2;
    let mut green_book_pixels = 0usize;
    for y in (origin_y + 14)..(origin_y + 45) {
        for x in (origin_x + 14)..(origin_x + 52) {
            let pixel = pixels.pixel(x, y);
            if pixel[1] > 48 && pixel[1] > pixel[0] && pixel[1] > pixel[2] {
                green_book_pixels += 1;
            }
        }
    }
    assert!(
        green_book_pixels > 0,
        "GUI book PIP should contribute green model pixels"
    );
    let option_pixel = pixels.pixel(origin_x + 61, origin_y + 15);
    assert!(
        option_pixel[0] > 192 && option_pixel[1] < 64 && option_pixel[2] < 64,
        "highlighted option stratum should remain above the PIP insertion, got {option_pixel:?}"
    );
}

#[test]
fn hud_entity_preview_blit_uv_is_identity_without_scissor() {
    let rect = HudEntityPreviewRect {
        x: 26,
        y: 8,
        width: 49,
        height: 70,
    };
    assert_eq!(
        hud_entity_preview_blit_uv(rect, rect),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }
    );
}

#[test]
fn hud_entity_preview_blit_uv_samples_scissored_sub_rect() {
    // rect 26..75 x 8..78 (49x70), visible sub-rect 30..40 x 20..32: the blit samples the
    // matching PIP texture fractions, with no vertical flip (wgpu row 0 is the texture top).
    let rect = HudEntityPreviewRect {
        x: 26,
        y: 8,
        width: 49,
        height: 70,
    };
    let visible = HudEntityPreviewRect {
        x: 30,
        y: 20,
        width: 10,
        height: 12,
    };
    let uv = hud_entity_preview_blit_uv(rect, visible);
    assert_eq!(uv.min, [4.0 / 49.0, 12.0 / 70.0]);
    assert_eq!(uv.max, [14.0 / 49.0, 24.0 / 70.0]);
}

#[test]
fn hud_inventory_beacon_offscreen_consumes_button_states_icons_and_payment_items() {
    let Some(mut renderer) = hud_inventory_beacon_test_renderer() else {
        return;
    };

    let pixels = renderer
        .render_offscreen_frame()
        .expect("Beacon inventory frame");
    for (position, expected) in [
        ((53, 22), [255, 0, 0]),
        ((65, 72), [0, 255, 0]),
        ((164, 107), [255, 255, 0]),
        ((190, 107), [255, 0, 255]),
        ((63, 32), [0, 255, 255]),
        ((75, 82), [255, 255, 255]),
        ((174, 117), [0, 255, 255]),
        ((200, 117), [255, 255, 255]),
    ] {
        let pixel = pixels.pixel(position.0, position.1);
        for channel in 0..3 {
            assert!(
                pixel[channel].abs_diff(expected[channel]) < 16,
                "Beacon pixel at {position:?} should be {expected:?}, got {pixel:?}"
            );
        }
    }

    for ((x, y), expected) in [
        ((28, 117), [255, 0, 0]),
        ((49, 117), [0, 255, 0]),
        ((71, 117), [0, 255, 255]),
        ((94, 117), [255, 0, 255]),
        ((116, 117), [255, 255, 255]),
    ] {
        let pixel = pixels.pixel(x, y);
        for channel in 0..3 {
            assert!(
                pixel[channel].abs_diff(expected[channel]) < 24,
                "Beacon payment pixel at ({x},{y}) should be {expected:?}, got {pixel:?}"
            );
        }
    }
}

#[test]
fn hud_mob_effect_icon_alpha_matches_vanilla_ending_window_edges() {
    let mut effect = test_hud_mob_effect(1, true, 1, 201, false, true);
    assert_eq!(hud_mob_effect_icon_alpha(&effect), 1.0);
    effect.duration_ticks = 200;
    assert!((hud_mob_effect_icon_alpha(&effect) - 0.5).abs() < 1e-6);
    effect.duration_ticks = 0;
    assert!((hud_mob_effect_icon_alpha(&effect) - 0.25).abs() < 1e-6);
    effect.duration_ticks = -1;
    assert_eq!(hud_mob_effect_icon_alpha(&effect), 1.0);
    effect.duration_ticks = 0;
    effect.ambient = true;
    assert_eq!(hud_mob_effect_icon_alpha(&effect), 1.0);
}

#[test]
fn dialog_screen_blur_marker_splits_before_and_after_screen_commands() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .add_test_gui_sprite("hud/crosshair", 2, 2, &[255, 0, 0, 255].repeat(4))
        .expect("crosshair");
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.fill_layers.push(HudInventoryFillLayer {
        x: 20,
        y: 20,
        width: 4,
        height: 4,
        tint: [0.0, 1.0, 0.0, 1.0],
        stage: HudInventoryFillStage::Foreground,
    });
    renderer.set_hud_inventory_screen(Some(screen));
    renderer.set_hud_screen_blur(true);

    let draws = renderer.collect_hud_draws();
    let blur = draws.screen_blur.expect("dialog blur marker");
    assert_eq!(blur.radius, 5.0);
    assert!(
        blur.first_base_command > 0,
        "in-world HUD commands stay before the screen blur"
    );
    assert_eq!(blur.first_overlay_command, 0);
    assert!(
        draws.commands.len() > draws.post_gui_item_start,
        "dialog foreground fill stays after the GUI-item split"
    );
    drop(draws);

    renderer.set_hud_screen_blur(false);
    assert!(renderer.collect_hud_draws().screen_blur.is_none());
    renderer.set_hud_screen_blur(true);
    renderer.set_menu_background_blurriness(0);
    assert!(renderer.collect_hud_draws().screen_blur.is_none());

    renderer.set_hud_screen_presence(true, true);
    renderer.set_hud_screen_blur(false);
    let draws = renderer.collect_hud_draws();
    assert!(draws.screen_blur.is_none());
    assert!(draws.commands.iter().any(|command| {
        let HudDrawCommand::Fill { start, end } = command else {
            return false;
        };
        let quad = &draws.vertices[*start as usize..*end as usize];
        quad.len() == 6
            && quad[0].position == [-1.0, 1.0]
            && quad[2].position == [1.0, -1.0]
            && quad[0].tint == HUD_PAUSE_BACKGROUND_TOP_TINT
            && quad[2].tint == HUD_PAUSE_BACKGROUND_BOTTOM_TINT
    }));
}

#[test]
fn dialog_screen_blur_strata_separate_hotbar_and_screen_block_items() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .update_terrain_texture_atlas(&[255, 255, 255, 255])
        .unwrap();
    renderer.update_camera();
    let rect = hotbar_item_hud_rect(PhysicalSize::new(320, 240), 0);
    let mut model = test_hud_block_item_model_with_quads(
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
        glam::Mat4::from_translation(glam::Vec3::splat(-0.5)),
        GuiItemLightingEntry::Items3d,
        false,
    );
    let mut after_model = model.clone();
    let HudResidentItemLayer::Geometry { tint, .. } = &mut Arc::make_mut(&mut model.layers)[0]
    else {
        unreachable!()
    };
    *tint = [1.0, 0.0, 0.0, 1.0];
    let HudResidentItemLayer::Geometry {
        tint, transform, ..
    } = &mut Arc::make_mut(&mut after_model.layers)[0]
    else {
        unreachable!()
    };
    *tint = [0.0, 1.0, 0.0, 1.0];
    *transform = glam::Mat4::from_translation(glam::Vec3::new(-0.5, -0.5, -1.0));
    renderer.set_hud_hotbar_block_item_models(vec![Some(model)]);
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.width = 320;
    screen.height = 240;
    screen.slots.push(HudInventorySlot {
        slot_id: 0,
        x: rect.x as i32,
        y: rect.y as i32,
        icon: None,
        block_model: Some(after_model),
    });
    renderer.set_hud_inventory_screen(Some(screen));
    renderer.set_hud_screen_blur(true);

    let mut residency = ItemModelResidencyFrame::default();
    renderer.append_hud_block_item_residency(&mut residency);
    assert_eq!(
        residency
            .instances
            .iter()
            .filter(|instance| instance.target == ItemModelDrawTarget::GuiBeforeBlur)
            .count(),
        1
    );
    assert_eq!(
        residency
            .instances
            .iter()
            .filter(|instance| instance.target == ItemModelDrawTarget::GuiAfterBlur)
            .count(),
        1
    );

    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame::default())
        .unwrap();
    let center = (
        (rect.x + rect.width as f32 / 2.0) as u32,
        (rect.y + rect.height as f32 / 2.0) as u32,
    );
    assert!(renderer.collect_hud_draws().screen_blur.is_some());
    let frame = renderer
        .render_offscreen_frame()
        .expect("blur strata frame");
    let pixel = frame.pixel(center.0, center.1);
    assert!(
        pixel[1] > 128 && pixel[0] < 8,
        "the farther after-blur item must not be occluded by before-blur depth: {pixel:?}"
    );

    renderer.set_hud_hotbar_mode(HudHotbarMode::Spectator(None));
    let mut residency = ItemModelResidencyFrame::default();
    renderer.append_hud_block_item_residency(&mut residency);
    assert_eq!(
        residency
            .instances
            .iter()
            .filter(|instance| instance.target == ItemModelDrawTarget::GuiBeforeBlur)
            .count(),
        0,
        "spectator hotbars must not retain ordinary 3D item models"
    );
    assert_eq!(
        residency
            .instances
            .iter()
            .filter(|instance| instance.target == ItemModelDrawTarget::GuiAfterBlur)
            .count(),
        1,
        "screen item residency is independent of the hotbar mode"
    );
}

#[test]
fn inventory_scroll_area_sanitizes_nested_layers_and_emits_centered_scissor() {
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.scroll_area = Some(HudInventoryScrollArea {
        scissor: HudInventoryItemScissor {
            x: 10,
            y: 20,
            width: 80,
            height: 60,
        },
        background_layers: vec![HudInventoryBackgroundLayer {
            texture: HudInventoryBackgroundTexture::Sprite("widget/button"),
            x: 12,
            y: 24,
            width: 40,
            height: 0,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        }],
        fill_layers: vec![HudInventoryFillLayer {
            x: 12,
            y: 24,
            width: 40,
            height: 12,
            tint: [2.0, 0.5, -1.0, 1.0],
            stage: HudInventoryFillStage::Foreground,
        }],
        text_labels: vec![HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 12,
            y: 24,
            width: 40,
            text: "Body".to_string(),
            tint: HUD_TINT_WHITE,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        }],
        text_areas: Vec::new(),
        scrollbar: Some(HudInventoryScrollbar {
            x: 84,
            y: 20,
            width: 6,
            height: 60,
            thumb_y: 30,
            thumb_height: 100,
        }),
    });
    let screen = sanitize_hud_inventory_screen(screen);
    let area = screen.scroll_area.as_ref().unwrap();
    assert!(area.background_layers.is_empty());
    assert_eq!(area.fill_layers[0].tint, [1.0, 0.5, 0.0, 1.0]);
    assert_eq!(area.text_labels[0].text, "Body");
    assert_eq!(area.scrollbar.unwrap().thumb_height, 60);

    let mut commands = Vec::new();
    assert!(push_hud_inventory_scissor(
        &mut commands,
        PhysicalSize::new(320, 240),
        &screen,
        area.scissor,
    ));
    push_hud_inventory_scissor_reset(&mut commands, PhysicalSize::new(320, 240));
    assert!(matches!(
        commands.as_slice(),
        [
            HudDrawCommand::Scissor {
                x: 82,
                y: 57,
                width: 80,
                height: 60,
            },
            HudDrawCommand::Scissor {
                x: 0,
                y: 0,
                width: 320,
                height: 240,
            }
        ]
    ));
}

#[test]
fn debug_options_scrollbar_rects_match_vanilla_selection_list_metrics() {
    let surface = PhysicalSize::new(420, 240);
    let (background, scroller) =
        hud_debug_options_scrollbar_rects(surface, 10, 47).expect("scrollbar");

    assert_eq!(background.x, 393);
    assert_eq!(background.y, 61);
    assert_eq!(background.width, 6);
    assert_eq!(background.height, 146);
    assert_eq!(scroller.x, 393);
    assert_eq!(scroller.y, 89);
    assert_eq!(scroller.width, 6);
    assert_eq!(scroller.height, 32);
    assert_eq!(hud_debug_options_scrollbar_rects(surface, 0, 3), None);
}

#[test]
fn debug_options_button_sprite_slot_matches_vanilla_widget_sprites() {
    assert_eq!(
        hud_debug_options_button_sprite_slot(true, false),
        HudDebugOptionsButtonSpriteSlot::Normal
    );
    assert_eq!(
        hud_debug_options_button_sprite_slot(true, true),
        HudDebugOptionsButtonSpriteSlot::Highlighted
    );
    assert_eq!(
        hud_debug_options_button_sprite_slot(false, false),
        HudDebugOptionsButtonSpriteSlot::Disabled
    );
    assert_eq!(
        hud_debug_options_button_sprite_slot(false, true),
        HudDebugOptionsButtonSpriteSlot::Disabled
    );
}

#[test]
fn debug_options_widget_text_styles_match_vanilla_focus_and_status_states() {
    let (hint, hint_runs) = hud_debug_options_search_display("", false, "Search...");
    assert_eq!(hint, "Search...");
    assert_eq!(hint_runs.len(), 1);
    assert!(hint_runs[0].style.italic);
    assert_eq!(hint_runs[0].color, Some(0xAAAAAA));
    assert_eq!(
        hud_debug_options_search_display("", true, "Search..."),
        (String::new(), Vec::new())
    );

    assert!(!hud_debug_options_entry_name_run("biome", true).style.italic);
    assert!(
        hud_debug_options_entry_name_run("biome", false)
            .style
            .italic
    );

    let selected = hud_debug_options_status_tint(HudDebugOptionsEntryStatus::AlwaysOn, true);
    assert_eq!(
        hud_debug_options_button_label_tint(false, selected, false),
        hud_argb_to_tint(0xFFDF5050)
    );
    assert_eq!(
        hud_debug_options_button_label_tint(false, selected, true),
        hud_argb_to_tint(0xFFA0A0A0)
    );
    assert_eq!(
        crate::hud::text_layout::hud_run_color_tint(
            &HudStyledTextRun {
                text: "kept".to_string(),
                style: HudTextStyle::default(),
                color: Some(0x55FF55),
                shadow_color: None,
            },
            hud_debug_options_button_label_tint(false, HUD_TINT_WHITE, true),
        ),
        hud_argb_to_tint(0xFF55FF55)
    );
    assert_eq!(
        hud_debug_options_status_tint(HudDebugOptionsEntryStatus::InOverlay, true),
        hud_argb_to_tint(0xFFFFFF55)
    );
    assert_eq!(
        hud_debug_options_status_tint(HudDebugOptionsEntryStatus::Never, false),
        hud_argb_to_tint(0xFFBABABA)
    );
}
