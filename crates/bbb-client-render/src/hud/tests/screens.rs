use super::*;

#[test]
fn minecart_command_block_editor_layout_matches_vanilla_absolute_geometry() {
    let compact = hud_minecart_command_block_editor_layout(PhysicalSize::new(320, 240));
    assert_eq!(
        compact.command_input,
        HudCommandBlockEditorRect {
            x: 10,
            y: 50,
            width: 300,
            height: 20,
        }
    );
    assert_eq!(compact.previous_output.x, 10);
    assert_eq!(compact.previous_output.y, 150);
    assert_eq!(compact.track_output.x, 290);
    assert_eq!(compact.track_output.y, 150);
    assert_eq!(
        compact.done,
        HudCommandBlockEditorRect {
            x: 6,
            y: 192,
            width: 150,
            height: 20,
        }
    );
    assert_eq!(compact.cancel.x, 164);

    let wide = hud_minecart_command_block_editor_layout(PhysicalSize::new(854, 480));
    assert_eq!(wide.command_input.x, 277);
    assert_eq!(wide.previous_output.x, 277);
    assert_eq!(wide.previous_output.y, 150);
    assert_eq!(wide.track_output.x, 557);
    assert_eq!(wide.track_output.y, 150);
    assert_eq!(wide.done.y, 252);
    assert_eq!(wide.cancel.x, 431);
}

#[test]
fn command_suggestions_offscreen_frame_draws_input_list_and_scroll_markers() {
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
    for ch in "?/seedcmd01next".chars() {
        glyphs.insert_first_wins(ch, glyph);
    }
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .expect("test font atlas");
    renderer.set_hud_command_suggestions(Some(HudCommandSuggestions {
        preedit: None,
        input_view: None,
        input_text: "/se".to_string(),
        draft: false,
        syntax: None,
        cursor: 3,
        selection: 3,
        cursor_visible: true,
        suffix: Some("ed".to_string()),
        list_x: 10,
        list_y: 50,
        list_width: 60,
        list_height: 24,
        entries: vec![
            HudCommandSuggestionEntry {
                runs: vec![HudStyledTextRun::plain("seed")],
                selected: true,
            },
            HudCommandSuggestionEntry {
                runs: vec![HudStyledTextRun::plain("cmd01")],
                selected: false,
            },
        ],
        has_previous: true,
        has_next: true,
        tooltip: None,
        layout: HudCommandSuggestionsLayout::Chat,
    }));

    let pixels = renderer
        .render_offscreen_frame()
        .expect("command suggestion frame");
    let outside = pixels.pixel(5, 45);
    assert!(outside[2] > 128 && outside[0] < 32 && outside[1] < 32);
    let list_background = pixels.pixel(65, 60);
    assert!(
        list_background[2] < 160,
        "list background should darken the blue frame, got {list_background:?}"
    );
    let input_background = pixels.pixel(300, 232);
    assert!(
        input_background[2] >= 124 && input_background[2] <= 130,
        "input background should half-darken the blue frame, got {input_background:?}"
    );
    let previous_marker = pixels.pixel(10, 49);
    let next_marker = pixels.pixel(10, 74);
    for marker in [previous_marker, next_marker] {
        assert!(
            marker[0] > 200 && marker[1] > 200 && marker[2] > 200,
            "scroll marker should be white, got {marker:?}"
        );
    }
}

/// `CommandSuggestions.SuggestionsList` measures each row with
/// `font.width(suggestion.getText())` and
/// draws it through `graphics.text(font, String, …)`.
/// Both faces read the one
/// `FormattedText.of` run list, so the codes cannot contribute an advance to
/// the width and still reach the glyph pass.
#[test]
fn command_suggestion_row_width_and_glyphs_come_from_the_resolved_run_list() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;
    const LIST_X: i32 = 10;
    const LIST_Y: i32 = 50;
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
    for ch in "sedplain\u{a7}c".chars() {
        glyphs.insert_first_wins(ch, glyph);
    }

    // "§cseed§rplain": a colour code then a reset restores the edit box's own
    // style, which is what `Style.EMPTY` does for `FormattedText.of`.
    let runs = hud_coded_string_runs("\u{a7}cseed\u{a7}rplain");
    assert_eq!(
        runs,
        vec![
            HudStyledTextRun {
                shadow_color: None,
                text: "seed".to_string(),
                style: HudTextStyle::default(),
                color: Some(0xFF_5555),
            },
            HudStyledTextRun::plain("plain"),
        ]
    );
    // Nine glyphs at 6px: the two codes and the reset contributed nothing.
    let width = hud_coded_string_width(&runs, &glyphs);
    assert_eq!(width, 54);
    assert_eq!(
        hud_coded_string_width(&runs, &glyphs),
        hud_raw_string_width("seedplain", &glyphs)
    );
    // The raw shape would have charged both code characters their advance.
    let raw = "\u{a7}cseed\u{a7}rplain";
    assert_ne!(
        hud_coded_string_width(&runs, &glyphs),
        hud_raw_string_width(raw, &glyphs)
    );

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
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .expect("test font atlas");
    renderer.set_hud_command_suggestions(Some(HudCommandSuggestions {
        input_text: String::new(),
        list_x: LIST_X,
        list_y: LIST_Y,
        list_width: width as u32 + 1,
        list_height: HUD_COMMAND_LINE_HEIGHT as u32,
        entries: vec![HudCommandSuggestionEntry {
            runs: runs.clone(),
            selected: true,
        }],
        ..Default::default()
    }));

    let pixels = renderer
        .render_offscreen_frame()
        .expect("command suggestion frame");
    // The one row: the list fill everywhere except the glyph quads and their
    // one-pixel shadows, so the lit columns are the drawn glyph sequence.
    let background = pixels.pixel(LIST_X as u32 + width as u32 - 6, LIST_Y as u32 + 6);
    let lit = (LIST_X as u32..LIST_X as u32 + width as u32 + 1)
        .filter(|x| {
            (0..HUD_COMMAND_LINE_HEIGHT as u32)
                .any(|dy| pixels.pixel(*x, LIST_Y as u32 + dy) != background)
        })
        .collect::<Vec<_>>();
    let expected = (0..9)
        .flat_map(|index| {
            let x = LIST_X as u32 + 1 + index * 6;
            [x, x + 1]
        })
        .collect::<Vec<_>>();
    assert_eq!(lit, expected);
    // The first run keeps the code's colour on the glyphs it resolved to.
    let red = (0..HUD_COMMAND_LINE_HEIGHT as u32)
        .map(|dy| pixels.pixel(LIST_X as u32 + 1, LIST_Y as u32 + dy))
        .find(|pixel| *pixel != background)
        .expect("first glyph pixel");
    assert_eq!(red, [255, 85, 85, 255]);
    // The reset run falls back to the row tint.
    let plain = (0..HUD_COMMAND_LINE_HEIGHT as u32)
        .map(|dy| pixels.pixel(LIST_X as u32 + 1 + 4 * 6, LIST_Y as u32 + dy))
        .find(|pixel| *pixel != background)
        .expect("reset-run glyph pixel");
    assert_eq!(plain, [255, 255, 0, 255]);
}

/// `StatsScreen` draws the general row's `statDisplay` as a
/// Component and its `getValueText()` through the `String` overload
/// and right-aligns the value with
/// `font.width(msg)`. Both this client's row
/// label and its value are localised projections, so the shared draw helper
/// reads one resolved list for the width and the glyphs.
#[test]
fn stats_row_name_and_value_draw_the_resolved_runs_that_measured_them() {
    use crate::camera::ClearColor;

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
    for ch in "LablV\u{a7}c".chars() {
        glyphs.insert_first_wins(ch, glyph);
    }
    // "§cVal": the pair is consumed, so three glyphs of six pixels each.
    assert_eq!(
        hud_coded_string_width(&hud_coded_string_runs("\u{a7}cVal"), &glyphs),
        18
    );
    assert_eq!(hud_raw_string_width("\u{a7}cVal", &glyphs), 30);

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
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
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .expect("test font atlas");
    renderer.set_hud_stats_screen(Some(HudStatsScreen {
        title: "Stats".to_string(),
        loading_text: "Loading".to_string(),
        is_loading: false,
        general_tab: "General".to_string(),
        items_tab: "Items".to_string(),
        mobs_tab: "Mobs".to_string(),
        active_tab: 0,
        items_tab_enabled: true,
        mobs_tab_enabled: true,
        rows: vec![HudStatsRow {
            label: "\u{a7}cLabl".to_string(),
            value: "\u{a7}cVal".to_string(),
            secondary_value: None,
        }],
        item_rows: Vec::new(),
        item_column_labels: std::array::from_fn(|_| "Col".to_string()),
        item_no_value: "-".to_string(),
        item_sort_column: None,
        item_sort_order: 0,
        item_hovered_column: None,
        item_focused_column: None,
        scroll_row: 0,
        scroll_amount: 0,
        max_scroll_amount: 0,
        scroller_height: 0,
        total_rows: 1,
        visible_rows: 6,
        row_height: 14,
        tooltip: None,
        done: "Done".to_string(),
        done_hovered: false,
        done_focused: false,
    }));

    let pixels = renderer.render_offscreen_frame().expect("stats frame");
    // The label starts at the list's left inset and the value ends four pixels
    // before the list's right edge, both measured from the resolved list alone.
    let list_x = 160 - HUD_STATS_LIST_WIDTH / 2;
    let row_y = (HUD_STATS_LIST_TOP + HUD_STATS_LIST_ENTRY_PADDING) as u32;
    let value_x = list_x + HUD_STATS_LIST_WIDTH - 18 - 4;
    let glyph_columns = |left: i32, count: i32| {
        (0..count)
            .map(|index| (left + index * 6) as u32)
            .collect::<Vec<_>>()
    };
    let red_columns = |left: i32, count: i32| {
        glyph_columns(left, count)
            .into_iter()
            .filter(|x| (0..9).any(|dy| pixels.pixel(*x, row_y + dy) == [255, 85, 85, 255]))
            .collect::<Vec<_>>()
    };
    assert_eq!(red_columns(list_x + 2, 4), glyph_columns(list_x + 2, 4));
    assert_eq!(red_columns(value_x, 3), glyph_columns(value_x, 3));
    // The raw face would have charged the code two glyphs and started the value
    // twelve pixels further left.
    assert!(
        red_columns(value_x - 12, 2).is_empty(),
        "no glyphs where the raw width would have placed the value"
    );
    assert_eq!(
        hud_coded_string_runs("\u{a7}cVal"),
        vec![HudStyledTextRun {
            text: "Val".to_string(),
            color: Some(0xFF_5555),
            ..HudStyledTextRun::default()
        }]
    );
}

/// `SocialInteractionsScreen` builds the panel's search box as an
/// `EditBox`: its value reaches the glyph pass through
/// `FormattedCharSequence.forward`, while the hint at
/// is a `Component`. The one call site therefore resolves each input
/// on its own face, so a `§` typed into the box keeps its own glyph and advance
/// while the same pair in the hint shapes the run instead.
#[test]
fn social_search_box_value_keeps_its_section_glyph_and_hint_consumes_codes() {
    use crate::camera::ClearColor;

    let glyph = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        width: 2,
        height: 2,
        advance: 6.0,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    for ch in "SearchA\u{a7}c".chars() {
        glyphs.insert_first_wins(ch, glyph);
    }
    // The raw face charges the `§` and `c` glyphs the glyph pass draws.
    assert_eq!(hud_raw_string_width("\u{a7}cA", &glyphs), 18);
    assert_eq!(
        hud_coded_string_width(&hud_coded_string_runs("\u{a7}cA"), &glyphs),
        6
    );

    let surface = PhysicalSize::new(320, 240);
    let layout = hud_social_interactions_layout(surface, 0, 0, false);
    let search = layout.search;
    let draw = |renderer: &mut Renderer, screen: HudSocialInteractionsMainScreen| {
        renderer.set_hud_screen_blur(false);
        renderer
            .set_hud_social_interactions_screen(Some(HudSocialInteractionsScreen::Main(screen)));
    };
    let base = |search_text: String, search_hint: String| HudSocialInteractionsMainScreen {
        generation: 1,
        title: String::new(),
        server_label: String::new(),
        page: HudSocialInteractionsPage::All,
        tab_labels: [String::new(), String::new(), String::new()],
        search: HudTextField {
            text: search_text,
            max_length: 16,
            focused: true,
            ..HudTextField::default()
        },
        search_hint,
        rows: Vec::new(),
        scroll_amount: 0,
        empty_text: None,
        blocking_help_label: String::new(),
        done_label: String::new(),
        focus: HudSocialInteractionsFocus::Search,
        hovered: None,
        tooltip: None,
    };

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
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
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .expect("test font atlas");
    let text_x = search.x + 4;
    let text_y = search.y + 4;
    let glyph_columns = |left: i32, count: i32| {
        (0..count)
            .flat_map(|index| {
                let x = (left + index * 6) as u32;
                [x, x + 1]
            })
            .collect::<Vec<_>>()
    };
    let white = |pixels: &crate::screenshot::ScreenshotPixels, columns: Vec<u32>| {
        columns
            .into_iter()
            .filter(|x| {
                (0..2).any(|dy| pixels.pixel(*x, text_y as u32 + dy) == [255, 255, 255, 255])
            })
            .collect::<Vec<_>>()
    };

    draw(
        &mut renderer,
        base("\u{a7}cA".to_string(), "Search".to_string()),
    );
    let pixels = renderer
        .render_offscreen_frame()
        .expect("social search value frame");
    // Three raw glyphs: `§`, `c` and `A` at the edit box's own advances.
    assert_eq!(
        white(&pixels, glyph_columns(text_x, 3)),
        glyph_columns(text_x, 3),
        "the edit box value keeps the `§` glyph"
    );
    assert!(
        white(&pixels, glyph_columns(text_x + 18, 2)).is_empty(),
        "the raw value ends after its third glyph"
    );

    draw(
        &mut renderer,
        base(String::new(), "\u{a7}cSearch".to_string()),
    );
    let pixels = renderer
        .render_offscreen_frame()
        .expect("social search hint frame");
    let red = |columns: Vec<u32>| {
        columns
            .into_iter()
            .filter(|x| (0..2).any(|dy| pixels.pixel(*x, text_y as u32 + dy) == [255, 85, 85, 255]))
            .collect::<Vec<_>>()
    };
    // The hint consumes the pair and draws the six styled glyphs it resolved to.
    assert_eq!(
        red(glyph_columns(text_x, 6)),
        glyph_columns(text_x, 6),
        "the hint's `§c` shapes the run instead of reaching the glyph pass"
    );
    assert!(
        red(glyph_columns(text_x + 36, 2)).is_empty(),
        "the consumed pair contributes no advance or glyph"
    );
}

#[test]
fn stats_screen_block_items_submit_scissored_resident_rows() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let model = test_hud_block_item_model_with_quads(
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
    );
    renderer.set_hud_stats_screen(Some(HudStatsScreen {
        title: "Stats".to_string(),
        loading_text: "Loading".to_string(),
        is_loading: false,
        general_tab: "General".to_string(),
        items_tab: "Items".to_string(),
        mobs_tab: "Mobs".to_string(),
        active_tab: 1,
        items_tab_enabled: true,
        mobs_tab_enabled: false,
        rows: Vec::new(),
        item_rows: vec![HudStatsItemRow {
            item_id: 1,
            resource_id: "minecraft:stone".to_string(),
            label: "Stone".to_string(),
            values: std::array::from_fn(|_| Some("0".to_string())),
            icon: None,
            block_model: Some(model),
        }],
        item_column_labels: std::array::from_fn(|index| format!("Column {index}")),
        item_no_value: "-".to_string(),
        item_sort_column: None,
        item_sort_order: 0,
        item_hovered_column: None,
        item_focused_column: None,
        scroll_row: 0,
        scroll_amount: 0,
        max_scroll_amount: 0,
        scroller_height: 0,
        total_rows: 1,
        visible_rows: 6,
        row_height: 22,
        tooltip: None,
        done: "Done".to_string(),
        done_hovered: false,
        done_focused: false,
    }));

    let mut residency = ItemModelResidencyFrame::default();
    renderer.append_hud_block_item_residency(&mut residency);
    assert_eq!(residency.instances.len(), 1);
    assert_eq!(residency.instances[0].target, ItemModelDrawTarget::Gui);
    assert!(residency.instances[0].scissor.is_some());
}

/// End-to-end GPU proof of the entity-preview PIP consumer: bakes the vanilla smithing-screen
/// armor-stand preview through the production `bake_hud_entity_preview_pip_geometry` path,
/// renders it into an isolated 40x60 PIP color+depth target under the real
/// [`CameraUniform::hud_entity_preview_pip`] camera (private depth cleared to 1.0 — the
/// `depth_isolated` contract: no world depth is attached), then blits the PIP texture into a
/// HUD frame through the real HUD sprite pipeline at the preview's screen rect. Asserts the
/// projected first-face anchor shows the entity texture, the in-rect empty corner keeps the
/// HUD background (transparent PIP pixels do not overwrite the frame), and pixels outside the
/// rect stay background. Skips (no assertion) when no GPU adapter is available.
#[test]
fn hud_entity_preview_pip_renders_and_blits_isolated_entity_pixels() {
    use wgpu::util::DeviceExt;

    use crate::camera::{CameraUniform, LightmapEnvironment};
    use crate::entity_models::{
        bake_hud_entity_preview_pip_geometry, build_entity_model_texture_atlas,
        create_entity_model_textured_pipeline, EntityModelTextureImage, EntityModelTextureRef,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    };
    use crate::gpu::{create_camera_buffer, create_depth_target, create_terrain_bind_group_layout};
    use crate::lightmap::{
        create_lightmap_bind_group_layout, create_lightmap_gpu,
        create_lightmap_sample_bind_group_layout,
    };
    use glam::{Quat, Vec4};

    const FRAME_WIDTH: u32 = 320;
    const FRAME_HEIGHT: u32 = 240;
    const PIP_WIDTH: u32 = 40;
    const PIP_HEIGHT: u32 = 60;
    // Non-sRGB frame so readback bytes are the shader's output verbatim; `320 * 4 = 1280` is a
    // multiple of 256, so the copy needs no row padding.
    const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    })) else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("bbb-hud-entity-preview-pip-test-device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        ..Default::default()
    })) else {
        return;
    };

    // Entity atlas with the armor-stand texture filled opaque green, so every textured face
    // samples an unambiguous non-background colour.
    let green = |texture: EntityModelTextureRef| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        let mut rgba = vec![0u8; len];
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&[0, 255, 0, 255]);
        }
        EntityModelTextureImage::new(texture, rgba)
    };
    let images: Vec<EntityModelTextureImage> = crate::entity_model_texture_refs()
        .iter()
        .filter(|texture| texture.path.starts_with("textures/entity/armorstand/"))
        .copied()
        .map(green)
        .collect();
    let (atlas_layout, atlas_rgba) =
        build_entity_model_texture_atlas(&images).expect("entity atlas");

    // The vanilla smithing-screen preview: armor stand at the origin, bodyRot 210, head pitch
    // 25, rect 121..161 x 20..80 (40x60), scale 25, translation (0, 1, 0), rotation
    // rotX(0.43633232) * rotZ(PI) — the exact `hud_smithing_entity_preview` producer values.
    let entity = EntityModelInstance::armor_stand(
        -1,
        [0.0, 0.0, 0.0],
        210.0,
        false,
        true,
        false,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 25.0));
    let rotation = Quat::from_rotation_x(0.43633232) * Quat::from_rotation_z(std::f32::consts::PI);
    let geometry = bake_hud_entity_preview_pip_geometry(&entity, &atlas_layout, None, None);
    assert!(
        !geometry.textured_indices.is_empty(),
        "armor stand preview bakes textured PIP geometry"
    );
    assert!(
        !geometry.textured_draws.is_empty(),
        "armor stand preview records PIP draw ranges"
    );
    let camera_uniform = CameraUniform::hud_entity_preview_pip(
        PIP_WIDTH as f32,
        PIP_HEIGHT as f32,
        25.0,
        [0.0, 1.0, 0.0],
        rotation.to_array(),
    );

    // Anchor: the first textured face's model-space center, projected through the same PIP
    // view-projection the shader uses — its own (green) face covers that pixel no matter which
    // equally-green part wins the depth test.
    let face_center = geometry
        .first_textured_face_center()
        .expect("baked geometry has a first face");
    let clip = camera_uniform.view_proj() * Vec4::from((face_center, 1.0));
    let ndc = clip.truncate() / clip.w;
    let anchor_px = ((ndc.x * 0.5 + 0.5) * PIP_WIDTH as f32).round() as u32;
    let anchor_py = ((0.5 - ndc.y * 0.5) * PIP_HEIGHT as f32).round() as u32;
    assert!(
        anchor_px < PIP_WIDTH && anchor_py < PIP_HEIGHT,
        "anchor projects into the PIP texture, got ({anchor_px},{anchor_py})"
    );

    // Entity atlas + PIP camera bind group (terrain layout: camera @0, texture @1, sampler @2).
    let bind_group_layout = create_terrain_bind_group_layout(&device);
    let camera_buffer = create_camera_buffer(&device);
    queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
    let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pip-test-atlas"),
        size: wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &atlas_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &atlas_rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_layout.width * 4),
            rows_per_image: Some(atlas_layout.height),
        },
        wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
    );
    let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("pip-test-atlas-sampler"),
        ..Default::default()
    });
    let entity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pip-test-entity-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&atlas_sampler),
            },
        ],
    });
    let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
    let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
    let lightmap = create_lightmap_gpu(
        &device,
        &queue,
        &lightmap_bind_group_layout,
        &lightmap_sample_bind_group_layout,
        LightmapEnvironment::default(),
    );
    // The armor stand's buckets are all cutout-family; one no-cull cutout pipeline covers the
    // whole concatenated index stream for the sentinel.
    let entity_pipeline = create_entity_model_textured_pipeline(
        &device,
        COLOR_FORMAT,
        &bind_group_layout,
        &lightmap_sample_bind_group_layout,
        None,
    )
    .expect("embedded entity pipeline");
    let entity_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("pip-test-entity-vertices"),
        contents: geometry.textured_vertex_bytes(),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let entity_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("pip-test-entity-indices"),
        contents: bytemuck::cast_slice(&geometry.textured_indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Isolated PIP target: private color texture (sampled by the blit) + private depth.
    let pip_color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pip-test-color"),
        size: wgpu::Extent3d {
            width: PIP_WIDTH,
            height: PIP_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let pip_color_view = pip_color.create_view(&wgpu::TextureViewDescriptor::default());
    let pip_depth = create_depth_target(&device, PIP_WIDTH, PIP_HEIGHT);

    // HUD frame + blit resources: the production HUD sprite pipeline and vertex path.
    let hud_bind_group_layout = create_hud_bind_group_layout(&device);
    let hud_pipeline = create_hud_pipeline(
        &device,
        COLOR_FORMAT,
        &hud_bind_group_layout,
        None,
        wgpu::BlendState::ALPHA_BLENDING,
    )
    .expect("embedded HUD pipeline");
    let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("pip-test-blit-sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pip-test-blit-bind-group"),
        layout: &hud_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&pip_color_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&blit_sampler),
            },
        ],
    });
    // The preview blit rect at the smithing screen position, through the production screen
    // layout mapping (screen 176x166 centered; preview rect origin 121,20).
    let surface_size = PhysicalSize::new(FRAME_WIDTH, FRAME_HEIGHT);
    let blit_rect =
        inventory_background_hud_rect(surface_size, 176, 166, 121, 20, PIP_WIDTH, PIP_HEIGHT);
    let rect = HudEntityPreviewRect {
        x: 121,
        y: 20,
        width: PIP_WIDTH,
        height: PIP_HEIGHT,
    };
    let blit_vertices_data = hud_quad_vertices(
        surface_size,
        blit_rect,
        hud_entity_preview_blit_uv(rect, rect),
        [1.0, 1.0, 1.0, 1.0],
    );
    let blit_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("pip-test-blit-vertices"),
        contents: bytemuck::cast_slice(&blit_vertices_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let frame_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pip-test-frame"),
        size: wgpu::Extent3d {
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let frame_view = frame_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bytes_per_row = FRAME_WIDTH * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pip-test-readback"),
        size: (bytes_per_row * FRAME_HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    // PIP pass: isolated color cleared to transparent + private depth cleared to 1.0 (the
    // depth_isolated contract — the frame's depth is never attached here).
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pip-test-pip-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &pip_color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &pip_depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&entity_pipeline);
        pass.set_bind_group(0, &entity_bind_group, &[]);
        pass.set_bind_group(1, &lightmap.sample_bind_group, &[]);
        pass.set_vertex_buffer(0, entity_vertices.slice(..));
        pass.set_index_buffer(entity_indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..geometry.textured_indices.len() as u32, 0, 0..1);
    }
    // HUD pass: blue background, then the PIP blit quad through the HUD sprite pipeline.
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pip-test-hud-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&hud_pipeline);
        pass.set_bind_group(0, &blit_bind_group, &[]);
        pass.set_vertex_buffer(0, blit_vertices.slice(..));
        pass.draw(0..blit_vertices_data.len() as u32, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &frame_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(FRAME_HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let data = slice.get_mapped_range().expect("mapped GPU readback");
    let pixel = |x: u32, y: u32| -> [u8; 4] {
        let o = (y * bytes_per_row + x * 4) as usize;
        [data[o], data[o + 1], data[o + 2], data[o + 3]]
    };
    // The screen origin is ((320-176)/2, (240-166)/2) = (72, 37); the preview rect lands at
    // (193, 57)..(233, 117) in frame pixels.
    let rect_origin_x = 72 + 121;
    let rect_origin_y = 37 + 20;
    let anchor_pixel = pixel(rect_origin_x + anchor_px, rect_origin_y + anchor_py);
    let in_rect_empty_pixel = pixel(rect_origin_x + 1, rect_origin_y + 1);
    let outside_pixel = pixel(0, 0);

    // The projected anchor shows the green entity texture through the blit.
    assert!(
        anchor_pixel[1] > 64
            && anchor_pixel[1] > anchor_pixel[2]
            && anchor_pixel[0] < anchor_pixel[1],
        "preview anchor should show the green entity, got {anchor_pixel:?}"
    );
    // Transparent PIP pixels inside the rect keep the HUD background (the blit alpha-blends
    // the isolated texture; it does not stamp the whole rect).
    assert!(
        in_rect_empty_pixel[2] > 128 && in_rect_empty_pixel[1] < 64,
        "empty preview pixel should keep the HUD background, got {in_rect_empty_pixel:?}"
    );
    // Pixels outside the preview rect stay background.
    assert!(
        outside_pixel[2] > 128 && outside_pixel[1] < 64,
        "outside pixel should stay background, got {outside_pixel:?}"
    );

    drop(data);
    readback.unmap();
}

#[test]
fn inventory_tooltip_uses_the_exact_custom_style_sprite_pair() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    let glyph = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        width: 1,
        height: 1,
        advance: 1.0,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins('A', glyph);
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .unwrap();

    let scaling = HudNineSliceScaling {
        sprite_width: 3,
        sprite_height: 3,
        border_left: 1,
        border_top: 1,
        border_right: 1,
        border_bottom: 1,
        stretch_inner: true,
    };
    renderer
        .add_test_gui_sprite_scaled(
            "tooltip/background",
            3,
            3,
            &[255, 0, 0, 255].repeat(9),
            HudGuiSpriteScaling::NineSlice(scaling),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite_scaled(
            "tooltip/frame",
            3,
            3,
            &[0, 0, 0, 0].repeat(9),
            HudGuiSpriteScaling::NineSlice(scaling),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "example:tooltip/rare_background",
            3,
            3,
            &[0, 255, 0, 255].repeat(9),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite_scaled(
            "example:tooltip/rare_frame",
            3,
            3,
            &[0, 0, 0, 0].repeat(9),
            HudGuiSpriteScaling::Tile {
                width: 3,
                height: 3,
            },
        )
        .unwrap();

    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.tooltip = Some(HudInventoryTooltip {
        slot_id: 0,
        x: 20,
        y: 20,
        style: Some("example:rare".to_string()),
        lines: vec![HudInventoryTooltipLine {
            text: "A".to_string(),
            tint: HUD_TINT_WHITE,
            runs: Vec::new(),
        }],
    });
    let background = hud_inventory_tooltip_background_hud_rect(
        PhysicalSize::new(320, 240),
        screen.width,
        screen.height,
        20,
        20,
        1,
        8,
    );
    renderer.set_hud_inventory_screen(Some(screen));

    let pixels = renderer.render_offscreen_frame().unwrap();
    let sample = pixels.pixel(background.x as u32, background.y as u32);
    assert!(
        sample[1] > 128 && sample[0] < 128 && sample[2] < 128,
        "custom tooltip background should be green instead of default red, got {sample:?}"
    );
}

#[test]
fn inventory_bundle_tooltip_renders_visual_grid_and_sanitizes_bounds() {
    let sanitized = sanitize_hud_bundle_tooltip(HudBundleTooltip {
        items: vec![
            HudItemIcon::single(HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            });
            20
        ],
        hidden_item_count: 5,
        selected_index: Some(15),
        selected_item_name: Some(HudInventoryTooltipLine {
            text: "Stone".to_string(),
            tint: HUD_TINT_WHITE,
            runs: Vec::new(),
        }),
        selected_item_tooltip_style: None,
        occupancy: f32::NAN,
        empty_description: "Can hold a mixed stack of items".to_string(),
        progress_label: Some("Full".to_string()),
    });
    assert_eq!(sanitized.items.len(), 11);
    assert_eq!(sanitized.selected_index, None);
    assert_eq!(sanitized.selected_item_name, None);
    assert_eq!(sanitized.occupancy, 0.0);

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'B',
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            advance: 6.0,
            width: 6,
            height: 8,
            ..Default::default()
        },
    );
    renderer
        .upload_hud_font_atlas(8, 8, &vec![0xff; 8 * 8 * 4], glyphs)
        .unwrap();
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.tooltip = Some(HudInventoryTooltip {
        slot_id: 0,
        x: 20,
        y: 20,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text: "B".to_string(),
            tint: HUD_TINT_WHITE,
            runs: Vec::new(),
        }],
    });
    renderer.set_hud_inventory_screen_with_bundle_tooltip(
        Some(screen),
        Some(HudBundleTooltip {
            items: vec![
                HudItemIcon::single(HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                });
                3
            ],
            hidden_item_count: 0,
            selected_index: Some(0),
            selected_item_name: Some(HudInventoryTooltipLine {
                text: "B".to_string(),
                tint: HUD_TINT_WHITE,
                runs: Vec::new(),
            }),
            selected_item_tooltip_style: None,
            occupancy: 0.5,
            empty_description: String::new(),
            progress_label: None,
        }),
    );
    let pixels = renderer.render_offscreen_frame().unwrap();
    assert!(pixels
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|px| px[3] > 0 && px[0..3] != [0, 0, 0]));
}

#[test]
fn scrolling_widget_text_layout_matches_vanilla_period_and_centering() {
    let bounds = HudInventoryScrollingText {
        x: 12,
        y: 30,
        width: 96,
        height: 20,
    };
    assert_eq!(
        hud_scrolling_text_layout(bounds, 50, 0.0),
        HudScrollingTextLayout {
            x: 35,
            y: 36,
            clipped: false,
        }
    );

    // Four overflow pixels use vanilla's minimum three-second period.
    assert_eq!(
        hud_scrolling_text_layout(bounds, 100, 0.0),
        HudScrollingTextLayout {
            x: 8,
            y: 36,
            clipped: true,
        }
    );
    assert_eq!(
        hud_scrolling_text_layout(bounds, 100, 1.5),
        HudScrollingTextLayout {
            x: 12,
            y: 36,
            clipped: true,
        }
    );

    // Twenty overflow pixels use a ten-second period instead of the floor.
    assert_eq!(hud_scrolling_text_layout(bounds, 116, 0.0).x, -8);
    assert_eq!(hud_scrolling_text_layout(bounds, 116, 5.0).x, 12);
    assert_eq!(
        hud_scrolling_text_layout(bounds, 116, f64::NAN),
        hud_scrolling_text_layout(bounds, 116, 0.0)
    );
}

#[test]
fn sign_editor_done_layout_and_preedit_projection_match_vanilla_geometry() {
    let surface = PhysicalSize::new(320, 240);
    assert_eq!(
        hud_sign_editor_done_button_rect(surface),
        HudCommandBlockEditorRect {
            x: 60,
            y: 204,
            width: 200,
            height: 20,
        }
    );
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('a', 5), ('b', 6), ('c', 6), ('?', 6)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: advance as f32,
                ..HudAsciiGlyph::default()
            },
        );
    }
    let screen = HudSignEditorScreen {
        kind: HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        },
        sign_preview: None,
        title: "Edit Sign Message".to_string(),
        lines: [
            "ab".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ],
        line: 0,
        cursor: 1,
        selection: 1,
        cursor_visible: true,
        text_tint: [0.0, 0.0, 0.0, 1.0],
        done: "Done".to_string(),
        done_hovered: true,
        preedit: Some(HudImePreedit {
            text: "c".to_string(),
            focused_start: 0,
            focused_end: 1,
            caret: Some(1),
            caret_visible: true,
        }),
    };
    let projection = hud_sign_editor_preedit_projection(&screen, surface, &glyphs).unwrap();
    assert_eq!(
        (projection.overlay.text_x, projection.overlay.text_y),
        (160, 84)
    );
    assert_eq!(
        (
            projection.overlay.background_x,
            projection.overlay.background_y,
            projection.overlay.background_width,
            projection.overlay.background_height,
        ),
        (155, 79, 16, 19)
    );
    assert_eq!(projection.ime_position, PhysicalPosition::new(158, 68));
    assert_eq!(projection.ime_size, PhysicalSize::new(10, 27));
}
