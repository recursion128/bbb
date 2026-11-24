use super::*;

fn assert_rgba_close(actual: [u8; 4], expected: [u8; 4], tolerance: u8) {
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(a, e)| a.abs_diff(e) <= tolerance),
        "pixel {actual:?}, expected {expected:?} within {tolerance}"
    );
}

#[test]
fn gui_fill_gpu_preserves_zero_and_one_over_255_alpha_and_translucent_blending() {
    let Some(renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let surface = PhysicalSize::new(32, 20);
    for (x, alpha) in [(2, 0.0), (10, 1.0 / 255.0), (18, 0.5)] {
        push_hud_fill_rect(
            &mut vertices,
            &mut commands,
            surface,
            x,
            2,
            6,
            6,
            [1.0, 1.0, 1.0, alpha],
        );
    }
    let (frame, _) =
        render_commands_over(&renderer, &vertices, &commands, wgpu::Color::TRANSPARENT);
    assert_eq!(frame.pixel(4, 4), [0, 0, 0, 0]);
    assert_eq!(frame.pixel(12, 4), [1, 1, 1, 1]);
    assert_rgba_close(frame.pixel(20, 4), [128; 4], 1);
    let (frame, _) = render_commands(&renderer, &vertices, &commands);
    assert_eq!(frame.pixel(4, 4), [51, 102, 153, 255]);
    assert_rgba_close(frame.pixel(20, 4), [153, 179, 204, 255], 1);
}

#[test]
fn gui_fill_gpu_gradient_interpolates_top_middle_and_bottom() {
    let Some(renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_hud_fill_gradient(
        &mut vertices,
        &mut commands,
        PhysicalSize::new(32, 20),
        absolute_hud_rect(2.0, 2.0, 28, 16),
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    );
    let (frame, counts) = render_commands(&renderer, &vertices, &commands);
    assert_eq!(counts, (1, 1));
    for x in [3, 16, 28] {
        for y in [2, 9, 17] {
            let blue = ((y as f32 + 0.5 - 2.0) / 16.0 * 255.0).round() as u8;
            assert_rgba_close(frame.pixel(x, y), [255 - blue, 0, blue, 255], 1);
        }
    }
    assert_eq!(frame.pixel(1, 9), [51, 102, 153, 255]);
}

#[test]
fn gui_fill_gpu_restores_bindings_across_sprites_highlight_and_fill() {
    let Some(renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let surface = PhysicalSize::new(32, 20);
    let uv = HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };
    push_hud_draw_with_uv_and_tint(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        surface,
        absolute_hud_rect(0.0, 2.0, 28, 10),
        uv,
        [1.0, 0.0, 0.0, 1.0],
    );
    push_hud_fill(
        &mut vertices,
        &mut commands,
        surface,
        absolute_hud_rect(4.0, 2.0, 20, 10),
        [0.0, 1.0, 0.0, 0.5],
    );
    push_hud_text_highlight(
        &mut vertices,
        &mut commands,
        surface,
        absolute_hud_rect(8.0, 2.0, 12, 10),
    );
    push_hud_fill(
        &mut vertices,
        &mut commands,
        surface,
        absolute_hud_rect(12.0, 2.0, 12, 10),
        [0.0, 0.0, 1.0, 0.5],
    );
    push_hud_draw_with_uv_and_tint(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        surface,
        absolute_hud_rect(16.0, 2.0, 12, 10),
        uv,
        [1.0, 1.0, 0.0, 1.0],
    );
    let (frame, counts) = render_commands(&renderer, &vertices, &commands);
    assert_eq!(counts, (6, 6));
    for (x, expected) in [
        (2, [255, 0, 0, 255]),
        (6, [128, 128, 0, 255]),
        (10, [127, 127, 255, 255]),
        (14, [64, 64, 255, 255]),
        (18, [255, 255, 0, 255]),
        (26, [255, 255, 0, 255]),
    ] {
        assert_rgba_close(frame.pixel(x, 4), expected, 2);
    }
}

#[test]
fn gui_fill_gpu_empty_scissor_suppresses_fill_and_restores_partial_and_full_clips() {
    let Some(renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let surface = PhysicalSize::new(32, 20);
    commands.push(HudDrawCommand::Scissor {
        x: 40,
        y: 0,
        width: 2,
        height: 2,
    });
    push_hud_fill_rect(
        &mut vertices,
        &mut commands,
        surface,
        0,
        0,
        32,
        20,
        [1.0, 0.0, 0.0, 1.0],
    );
    commands.push(HudDrawCommand::Scissor {
        x: 3,
        y: 3,
        width: 3,
        height: 3,
    });
    push_hud_fill_rect(
        &mut vertices,
        &mut commands,
        surface,
        0,
        0,
        32,
        20,
        [0.0, 1.0, 0.0, 1.0],
    );
    commands.push(HudDrawCommand::Scissor {
        x: 0,
        y: 0,
        width: 32,
        height: 20,
    });
    push_hud_fill_rect(
        &mut vertices,
        &mut commands,
        surface,
        20,
        4,
        2,
        2,
        [0.0, 0.0, 1.0, 1.0],
    );
    let (frame, counts) = render_commands(&renderer, &vertices, &commands);
    assert_eq!(counts, (2, 1));
    assert_eq!(frame.pixel(2, 3), [51, 102, 153, 255]);
    assert_eq!(frame.pixel(3, 3), [0, 255, 0, 255]);
    assert_eq!(frame.pixel(5, 5), [0, 255, 0, 255]);
    assert_eq!(frame.pixel(6, 3), [51, 102, 153, 255]);
    assert_eq!(frame.pixel(20, 4), [0, 0, 255, 255]);
}

#[test]
fn gui_fill_gpu_selected_fragment_changes_fill_but_preserves_font_effects_and_sprites() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(1, 1, &[0, 255, 0, 255], glyphs(6.0))
        .unwrap();
    let shaders = crate::core_shaders::AdaptedShaderPair {
        vertex: "#version 450\nlayout(location=0) in vec2 Position; layout(location=2) in vec4 Color; layout(location=0) out vec4 vertexColor; void main() { gl_Position=vec4(Position,0.0,1.0); vertexColor=Color; }".into(),
        fragment: "#version 450\nlayout(location=0) in vec4 vertexColor; layout(location=0) out vec4 fragColor; void main() { if (vertexColor.a==0.0) discard; fragColor=vec4(1.0,0.0,0.0,vertexColor.a); }".into(),
    };
    renderer.pipelines.hud_gui = create_hud_gui_pipelines(
        &renderer.device,
        renderer.config.format,
        renderer.gui_projection_extent(),
        Some(&shaders),
    )
    .unwrap();
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let surface = PhysicalSize::new(32, 20);
    let uv = HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };
    push_hud_fill_rect(
        &mut vertices,
        &mut commands,
        surface,
        2,
        2,
        4,
        4,
        [0.0, 0.0, 1.0, 1.0],
    );
    let geometry = HudStyledTextGeometry {
        text_bounds: None,
        glyph_areas: Vec::new(),
        empty_areas: Vec::new(),
        glyph_quads: vec![(
            HudGlyphQuad {
                corners: [[8.0, 2.0], [8.0, 6.0], [12.0, 6.0], [12.0, 2.0]],
                uv,
                shadow: false,
            },
            HUD_TINT_WHITE,
        )],
        effect_rects: vec![
            (
                HudEffectRect {
                    kind: bbb_render_types::HudEffectKind::Underline,
                    x0: 8.0,
                    y0: 8.0,
                    x1: 12.0,
                    y1: 10.0,
                },
                HUD_TINT_WHITE,
            ),
            (
                HudEffectRect {
                    kind: bbb_render_types::HudEffectKind::Strikethrough,
                    x0: 8.0,
                    y0: 12.0,
                    x1: 12.0,
                    y1: 14.0,
                },
                [1.0, 0.0, 1.0, 1.0],
            ),
        ],
    };
    push_hud_styled_text_geometry(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        renderer.pack.hud_font_atlas.as_ref().unwrap(),
        surface,
        &geometry,
    );
    push_hud_draw_with_uv_and_tint(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        surface,
        absolute_hud_rect(16.0, 2.0, 4, 4),
        uv,
        [0.0, 0.0, 1.0, 1.0],
    );
    push_hud_fill_rect(
        &mut vertices,
        &mut commands,
        surface,
        24,
        2,
        4,
        4,
        [0.0, 1.0, 0.0, 1.0],
    );
    let (frame, _) = render_commands(&renderer, &vertices, &commands);
    assert_eq!(frame.pixel(3, 3), [255, 0, 0, 255]);
    assert_eq!(frame.pixel(9, 3), [0, 255, 0, 255]);
    assert_eq!(frame.pixel(9, 8), [255; 4]);
    assert_eq!(frame.pixel(9, 12), [255, 0, 255, 255]);
    assert_eq!(frame.pixel(17, 3), [0, 0, 255, 255]);
    assert_eq!(frame.pixel(25, 3), [255, 0, 0, 255]);
}

#[test]
fn gui_atlas_grows_to_fit_a_sprite_wider_than_its_default_width() {
    let Some(mut renderer) = Renderer::new_offscreen(4, 4) else {
        return;
    };
    renderer
        .add_test_gui_sprite("hud/wide", 1024, 16, &[255; 1024 * 16 * 4])
        .unwrap();
    renderer
        .add_test_gui_sprite("hud/small", 2, 2, &[255; 2 * 2 * 4])
        .unwrap();
    let wide = renderer.gui_sprite("hud/wide").expect("wide sprite");
    assert_eq!((wide.width, wide.height), (1024, 16));
    assert!(renderer.gui_sprite("hud/small").is_some());
}

#[test]
fn tile_sprite_drawn_at_its_png_size_still_tiles_by_its_metadata() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer
        .add_test_gui_sprite_scaled(
            "tooltip/background",
            32,
            32,
            &[255; 32 * 32 * 4],
            HudGuiSpriteScaling::Tile {
                width: 8,
                height: 8,
            },
        )
        .unwrap();
    let sprite = renderer.gui_sprite("tooltip/background").unwrap();
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_hud_gui_sprite(
        &mut vertices,
        &mut commands,
        sprite,
        PhysicalSize::new(32, 32),
        absolute_hud_rect(0.0, 0.0, 32, 32),
    );
    assert_eq!(vertices.len(), 16 * 6);
}
