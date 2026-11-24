use super::*;
use crate::{HudChatInputView, HudCommandSuggestions};
use bbb_render_types::{ShaderSourceKey, ShaderStage};

#[test]
fn gui_highlight_selected_shader_commits_and_rolls_back_with_resource_generation() {
    if std::env::var_os("BBB_VANILLA_ROOT").is_none() {
        return;
    }
    let Some(mut renderer) = Renderer::new_offscreen(32, 60) else {
        return;
    };
    let roots =
        bbb_client_assets::PackRoots::from_environment().expect("prepared Vanilla resources");
    let catalog = bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("shader catalog");
    let effects = bbb_client_assets::PostEffectCatalog::load(&roots).expect("post-effect catalog");
    let projection = |generation, stage, override_source: Option<&str>| {
        let mut sources = catalog.sources().clone();
        if let Some(source) = override_source {
            let fragment = sources
                .get_mut(&ShaderSourceKey {
                    id: "minecraft:core/gui".into(),
                    stage,
                })
                .unwrap();
            fragment.pack_id = "selected-gui-fixture".into();
            fragment.source = source.into();
        }
        PostEffectResourceProjection::new(
            generation,
            Arc::new(effects.graphs().clone()),
            Arc::new(sources.keys().cloned().collect()),
            CoreShaderSourceProjection::new(generation, Arc::new(sources)),
            Arc::new(effects.textures().clone()),
        )
    };
    let fragment = catalog
        .source_by_id("minecraft:core/gui", ShaderStage::Fragment)
        .unwrap();
    let red = fragment.source.replace(
        "fragColor = color * ColorModulator;",
        "fragColor = vec4(1.0, 0.0, 0.0, 1.0);",
    );
    assert_ne!(red, fragment.source);
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'A',
        HudAsciiGlyph {
            width: 2,
            height: 2,
            advance: 6.0,
            ..Default::default()
        },
    );
    let install_font =
        |renderer: &mut Renderer| renderer.upload_hud_font_atlas(1, 1, &[255; 4], glyphs.clone());
    renderer.set_hud_command_suggestions(Some(HudCommandSuggestions {
        input_view: Some(HudChatInputView {
            value: vec![65, 65],
            cursor: 0,
            selection: 1,
            display_start: 0,
        }),
        ..Default::default()
    }));
    let default = projection(41, ShaderStage::Fragment, None);
    renderer
        .transactional_shader_resource_reload(&default, false, install_font)
        .expect("default generation");
    let initial_frame = renderer.render_offscreen_frame().unwrap();
    let pixel = initial_frame.pixel(4, 48);
    let background = initial_frame.pixel(20, 47);
    assert!(
        pixel[0] < 40 && pixel[1] < 40 && pixel[2] > 180,
        "inverted white glyph is blue: {pixel:?}"
    );
    let selected = projection(42, ShaderStage::Fragment, Some(&red));
    renderer
        .transactional_shader_resource_reload(&selected, false, |_| -> Result<()> {
            anyhow::bail!("after shader swap")
        })
        .expect_err("later failure rolls back all GUI blend modes");
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(20, 47),
        background
    );
    assert_eq!(renderer.core_shader_generation(), Some(41));
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(4, 48),
        pixel
    );
    renderer
        .transactional_shader_resource_reload(&selected, false, install_font)
        .expect("selected gui generation");
    assert_eq!(renderer.core_shader_generation(), Some(42));
    let selected_frame = renderer.render_offscreen_frame().unwrap();
    let selected_pixel = selected_frame.pixel(4, 48);
    assert_eq!(
        selected_frame.pixel(20, 47),
        [255, 0, 0, 255],
        "chat background uses selected GUI fill"
    );
    assert!(
        selected_frame.pixel(10, 48)[..3].iter().all(|v| *v > 180),
        "unselected font glyph keeps its texture pipeline"
    );
    assert!(
        selected_pixel[..3].iter().all(|channel| *channel > 180),
        "both passes use selected gui shader: {selected_pixel:?}"
    );
    renderer
        .transactional_shader_resource_reload(
            &projection(43, ShaderStage::Fragment, Some("invalid GLSL")),
            false,
            install_font,
        )
        .expect_err("invalid gui stage rejects whole generation");
    assert_eq!(renderer.core_shader_generation(), Some(42));
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(20, 47),
        [255, 0, 0, 255]
    );
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(4, 48),
        selected_pixel
    );
    renderer
        .transactional_shader_resource_reload(
            &projection(44, ShaderStage::Fragment, None),
            false,
            install_font,
        )
        .expect("remove selected override");
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(20, 47),
        background
    );
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().pixel(4, 48),
        pixel
    );
    let vertex = catalog
        .source_by_id("minecraft:core/gui", ShaderStage::Vertex)
        .unwrap();
    let shifted = vertex.source.replace(
        "vec4(Position, 1.0)",
        "vec4(Position + vec3(1.0, 1.0, 0.0), 1.0)",
    );
    assert_ne!(shifted, vertex.source);
    renderer
        .transactional_shader_resource_reload(
            &projection(45, ShaderStage::Vertex, Some(&shifted)),
            false,
            install_font,
        )
        .expect("selected vertex pixel offset");
    let shifted_frame = renderer.render_offscreen_frame().unwrap();
    assert!(
        shifted_frame.pixel(4, 48)[..3].iter().all(|v| *v > 180),
        "one GUI pixel shift leaves first glyph column white"
    );
    assert_eq!(shifted_frame.pixel(5, 48), pixel);
    renderer.set_gui_viewport(winit::dpi::PhysicalSize::new(16, 30), 2);
    let resized = renderer.render_offscreen_frame().unwrap();
    assert!(
        resized.pixel(8, 36)[..3].iter().all(|v| *v > 180),
        "logical pixel offset follows resize"
    );
    assert_eq!(resized.pixel(10, 36), pixel);
    renderer
        .resize(winit::dpi::PhysicalSize::new(801, 603))
        .unwrap();
    renderer.set_gui_viewport(winit::dpi::PhysicalSize::new(401, 302), 2);
    let odd = renderer.render_offscreen_frame().unwrap();
    for x in [8, 9] {
        assert!(odd.pixel(x, 580)[..3].iter().all(|v| *v > 180));
    }
    for x in [10, 11] {
        assert_eq!(odd.pixel(x, 580), pixel);
    }
    let projection_color = fragment.source.replace(
        "fragColor = color * ColorModulator;",
        "fragColor = vec4(fract(vec2(1.0 / ProjMat[0].x, -1.0 / ProjMat[1].y) + vec2(0.125)), 0.0, 1.0);",
    );
    renderer
        .transactional_shader_resource_reload(
            &projection(46, ShaderStage::Fragment, Some(&projection_color)),
            false,
            install_font,
        )
        .expect("selected fragment reads the current GUI projection");
    let assert_projection_color = |renderer: &mut Renderer, expected: [u8; 4]| {
        let actual = renderer.render_offscreen_frame().unwrap().pixel(40, 580);
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(&a, e)| a.abs_diff(e) <= 1),
            "projection color: expected {expected:?}, got {actual:?}"
        );
    };
    assert_projection_color(&mut renderer, [96, 223, 0, 255]);
    renderer
        .resize(winit::dpi::PhysicalSize::new(802, 604))
        .unwrap();
    renderer.set_gui_viewport(winit::dpi::PhysicalSize::new(401, 302), 2);
    assert_projection_color(&mut renderer, [159, 32, 0, 255]);
    renderer.set_gui_viewport(winit::dpi::PhysicalSize::new(268, 202), 3);
    assert_projection_color(&mut renderer, [202, 202, 0, 255]);
    renderer
        .transactional_shader_resource_reload(
            &projection(
                47,
                ShaderStage::Fragment,
                Some(&projection_color.replace("0.125", "0.25")),
            ),
            false,
            install_font,
        )
        .expect("replacement GUI pipeline uses the current scaled extent");
    assert_projection_color(&mut renderer, [234, 234, 0, 255]);
}
