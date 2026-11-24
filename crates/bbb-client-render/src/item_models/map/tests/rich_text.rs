use super::*;
use bbb_render_types::{HudTextStyle, HudUvRect};

fn glyph(advance: f32, u: f32) -> HudAsciiGlyph {
    HudAsciiGlyph {
        width: 4,
        height: 8,
        advance,
        uv: HudUvRect {
            min: [u, 0.2],
            max: [u + 0.05, 0.8],
        },
        ..Default::default()
    }
}

fn fonts() -> HudFontGlyphBook {
    let mut default = HudFontGlyphMap::new();
    let mut alternate = HudFontGlyphMap::new();
    for ch in ['A', 'B', '1', '2', '-'] {
        default.insert_first_wins(ch, glyph(2.25, 0.1));
        alternate.insert_first_wins(ch, glyph(3.25, 0.6));
    }
    default.insert_first_wins(' ', HudAsciiGlyph::empty(2.5));
    HudFontGlyphBook::from_fonts(
        "default",
        BTreeMap::from([("default".into(), default), ("alternate".into(), alternate)]),
    )
}

fn bake(runs: &[WorldTextRun], fonts: &HudFontGlyphBook, seed: u64) -> ItemFrameMapTextSurface {
    bake_item_frame_map_text_surface(
        1,
        runs,
        0,
        0,
        3,
        Mat4::IDENTITY,
        [0.25, 0.75],
        7,
        fonts,
        seed,
    )
    .unwrap()
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

fn local(surface: &ItemFrameMapTextSurface, index: usize, unit: Vec3) -> Vec3 {
    surface.submission.transform.inverse().transform_point3(
        surface.glyphs[index]
            .transform
            .transform_point3(Vec3::new(unit.x, 1.0 - unit.y, unit.z)),
    )
}

#[test]
fn mixed_fonts_bold_measure_one_logical_sum_and_center_with_width_clamp() {
    let fonts = fonts();
    let runs = [
        WorldTextRun::plain("A"),
        WorldTextRun {
            text: "B".into(),
            font: Some("alternate".into()),
            style: HudTextStyle {
                bold: true,
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    let surface = bake(&runs, &fonts, 0);
    assert_eq!(surface.submission.width, 7.0); // ceil(2.25 + 3.25 + 1), not 3 + 5.
    near(surface.submission.scale, 2.0 / 3.0);
    near(surface.submission.transform.w_axis.x, 61.666668);
    near(local(&surface, 1, Vec3::Y).x, 2.15);
    assert_eq!(surface.glyphs[1].uv.min[0], 0.6);
    let long = bake(&[WorldTextRun::plain("A".repeat(20))], &fonts, 0);
    assert_eq!(long.submission.width, 45.0);
    near(long.submission.scale, 5.0 / 9.0);
    near(long.submission.transform.w_axis.x, 51.5);
    near(long.submission.transform.w_axis.y, 68.0);
}

#[test]
fn colored_italic_bold_quads_have_independent_corners_and_target_depth() {
    let fonts = fonts();
    let runs = [
        WorldTextRun {
            text: "A".into(),
            color: Some(0xff8000),
            style: HudTextStyle {
                bold: true,
                italic: true,
                ..Default::default()
            },
            ..Default::default()
        },
        WorldTextRun {
            text: "B".into(),
            color: Some(0x0040ff),
            ..Default::default()
        },
    ];
    let world = bake(&runs, &fonts, 0);
    let first = bake_first_person_map_text_surface(
        1,
        &runs,
        0,
        0,
        3,
        Mat4::IDENTITY,
        [0.25, 0.75],
        7,
        &fonts,
        0,
    )
    .unwrap();
    let gui = bake_hud_map_text_surface(
        1,
        &runs,
        0,
        0,
        3,
        Mat4::IDENTITY,
        [0.25, 0.75],
        7,
        &fonts,
        0,
    )
    .unwrap();
    for surface in [&world, &first, &gui] {
        assert_eq!(surface.glyphs.len(), 3, "no implicit shadow quads");
        let top = local(surface, 0, Vec3::Y);
        let bottom = local(surface, 0, Vec3::ZERO);
        near(top.x, 0.9);
        near(top.y, -0.1);
        near(bottom.x, -1.1);
        near(bottom.y, 8.1);
        near(local(surface, 1, Vec3::Y).x, 1.9);
        near(local(surface, 2, Vec3::Y).x, 3.25);
        assert_eq!(surface.glyphs[0].tint, [1.0, 128.0 / 255.0, 0.0, 1.0]);
        assert_eq!(surface.glyphs[2].tint, [0.0, 64.0 / 255.0, 1.0, 1.0]);
        assert_eq!(surface.glyphs[0].uv.min, [0.1, 0.2]);
        assert_eq!(surface.glyphs[0].uv.max, [0.15, 0.8]);
    }
    near(local(&world, 1, Vec3::ZERO).z, 0.001);
    near(local(&first, 1, Vec3::ZERO).z, 0.001);
    near(local(&gui, 1, Vec3::ZERO).z, 0.0);
}

#[test]
fn empty_glyph_keeps_pen_and_effects_after_all_visible_glyphs() {
    let runs = [WorldTextRun {
        text: " A".into(),
        color: Some(0x00ff00),
        style: HudTextStyle {
            underlined: true,
            strikethrough: true,
            ..Default::default()
        },
        ..Default::default()
    }];
    let surface = bake(&runs, &fonts(), 0);
    assert_eq!(surface.glyphs.len(), 5);
    assert!(!surface.glyphs[0].solid);
    near(local(&surface, 0, Vec3::Y).x, 2.5);
    for (index, x0, x1, y0, y1) in [
        (1, -1.0, 2.5, 3.5, 4.5),
        (2, -1.0, 2.5, 8.0, 9.0),
        (3, 2.5, 4.75, 3.5, 4.5),
        (4, 2.5, 4.75, 8.0, 9.0),
    ] {
        assert!(surface.glyphs[index].solid);
        let top = local(&surface, index, Vec3::Y);
        let bottom_right = local(&surface, index, Vec3::X);
        near(top.x, x0);
        near(top.y, y0);
        near(bottom_right.x, x1);
        near(bottom_right.y, y1);
        near(top.z, 0.01);
        assert_eq!(surface.glyphs[index].uv.min, [0.0; 2]);
    }
}

#[test]
fn decoration_run_that_only_changes_its_event_expands_its_own_first_glyph() {
    // Identical glyphs and identical visible style; only the event triple
    // differs, which vanilla `Style` equality counts, so
    // `SubStringSource.substring` cuts a second
    // part and its first glyph draws at `position == 0`.
    let underlined = |event_id| WorldTextRun {
        text: "A".into(),
        color: Some(0x00ff00),
        style: HudTextStyle {
            underlined: true,
            ..Default::default()
        },
        event_id,
        ..Default::default()
    };
    // Effects follow every glyph quad, so the two bars are the last two.
    let bars = |runs: [WorldTextRun; 2]| {
        let surface = bake(&runs, &fonts(), 0);
        assert_eq!(surface.glyphs.len(), 4);
        [2, 3].map(|index| {
            (
                local(&surface, index, Vec3::Y).x,
                local(&surface, index, Vec3::X).x,
            )
        })
    };
    // One part: only the first glyph gets `effectX0 = x - 1`.
    let [first, second] = bars([underlined(None), underlined(None)]);
    near(first.0, -1.0);
    near(first.1, 2.25);
    near(second.0, 2.25);
    near(second.1, 4.5);
    // A different event restarts it, so the second run's bar reaches one pixel
    // left of its own pen.
    let [first, second] = bars([underlined(None), underlined(Some(0))]);
    near(first.0, -1.0);
    near(first.1, 2.25);
    near(second.0, 1.25);
    near(second.1, 4.5);
}

#[test]
fn rtl_visual_order_preserves_component_font_and_color() {
    let mut fonts = fonts();
    fonts.set_default_right_to_left(true);
    let surface = bake(
        &[
            WorldTextRun {
                text: "12".into(),
                color: Some(0xff0000),
                ..Default::default()
            },
            WorldTextRun {
                text: "-".into(),
                font: Some("alternate".into()),
                color: Some(0x00ff00),
                ..Default::default()
            },
        ],
        &fonts,
        0,
    );
    assert_eq!(surface.submission.text, "12-");
    assert_eq!(surface.submission.width, 8.0);
    assert_eq!(
        surface
            .glyphs
            .iter()
            .map(|g| g.uv.min[0])
            .collect::<Vec<_>>(),
        [0.6, 0.1, 0.1]
    );
    assert_eq!(surface.glyphs[0].tint, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(surface.glyphs[1].tint, [1.0, 0.0, 0.0, 1.0]);
    near(local(&surface, 1, Vec3::Y).x, 3.25);
}

#[test]
fn obfuscation_uses_selected_font_ceil_bucket_and_replacement_metrics() {
    let mut default = HudFontGlyphMap::new();
    default.insert_first_wins('A', glyph(2.5, 0.9));
    let mut alternate = HudFontGlyphMap::new();
    alternate.insert_first_wins('A', glyph(2.1, 0.1));
    alternate.insert_first_wins('B', glyph(2.9, 0.2));
    alternate.insert_first_wins('C', glyph(4.0, 0.3));
    let fonts = HudFontGlyphBook::from_fonts(
        "default",
        BTreeMap::from([("default".into(), default), ("alternate".into(), alternate)]),
    );
    let runs = [WorldTextRun {
        text: "AA".into(),
        font: Some("alternate".into()),
        style: HudTextStyle {
            obfuscated: true,
            underlined: true,
            ..Default::default()
        },
        ..Default::default()
    }];
    let mut choices = std::collections::BTreeSet::new();
    for seed in 0..32 {
        let surface = bake(&runs, &fonts, seed);
        assert_eq!(surface, bake(&runs, &fonts, seed));
        assert_eq!(surface.submission.width, 5.0);
        let mut advance = 0.0;
        for index in 0..2 {
            let u = surface.glyphs[index].uv.min[0];
            let width = if u == 0.1 {
                2.1
            } else {
                assert_eq!(u, 0.2);
                2.9
            };
            choices.insert((u * 10.0) as u32);
            near(local(&surface, index, Vec3::Y).x, advance);
            near(local(&surface, 2 + index, Vec3::X).x, advance + width);
            advance += width;
        }
    }
    assert_eq!(
        choices.len(),
        2,
        "seeds must exercise both fractional advances"
    );
}

#[test]
fn gui_rich_text_upload_and_pixels_preserve_tint_and_solid_effects() {
    let Some(mut renderer) = Renderer::new_offscreen(128, 128) else {
        return;
    };
    for label in ["A", ""] {
        let runs = [WorldTextRun {
            text: label.into(),
            color: Some(0xff0000),
            shadow_color: Some(0x800000ff),
            style: HudTextStyle {
                underlined: true,
                ..Default::default()
            },
            ..Default::default()
        }];
        let pose = Mat4::from_translation(Vec3::new(-170.0, -194.0, 0.0))
            * Mat4::from_scale(Vec3::splat(3.0));
        let surface =
            bake_hud_map_text_surface(1, &runs, 0, 0, 0, pose, [0.25, 0.75], 0, &fonts(), 0)
                .unwrap();
        if !surface.glyphs.is_empty() {
            near(local(&surface, 2, Vec3::ZERO).z, 0.01);
        }
        let projection =
            glam::camera::rh::proj::directx::orthographic(0.0, 128.0, 128.0, 0.0, -1.0, 1.0);
        renderer.queue.write_buffer(
            renderer.camera_buffers.gui().buffer,
            renderer.camera_buffers.gui().offset,
            bytemuck::cast_slice(&projection.to_cols_array()),
        );
        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rich-map-readback"),
            size: wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renderer.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        for alpha in [255, 0] {
            renderer
                .upload_hud_font_atlas(
                    1,
                    1,
                    &[255, 255, 255, alpha],
                    renderer.pack.hud_font_book.clone(),
                )
                .unwrap();
            renderer.set_map_text_surfaces_with_gui(
                Vec::new(),
                Vec::new(),
                vec![HudMapPreviewSurface {
                    preview_index: 2,
                    surface: surface.clone(),
                }],
            );
            let residency = &renderer.pack.map_surface_residency;
            assert_eq!(residency.text_instances.len(), surface.glyphs.len() + 1);
            assert_eq!(residency.text_draws[0].target, MapSurfaceTarget::Gui(2));
            assert_eq!(
                residency.text_upload_bytes,
                (surface.glyphs.len() + 1) as u64
                    * std::mem::size_of::<MapSurfaceGpuInstance>() as u64
            );
            for (instance, glyph) in residency
                .text_instances
                .iter()
                .zip(std::iter::once(&surface.background).chain(&surface.glyphs))
            {
                assert_eq!(instance.tint, glyph.tint);
                assert_eq!(instance.light, [0.25, 0.75, f32::from(glyph.solid), 0.0]);
                assert_eq!(instance.model, glyph.transform.to_cols_array_2d());
                assert_eq!(
                    instance.uv_rect,
                    [
                        glyph.uv.min[0],
                        glyph.uv.min[1],
                        glyph.uv.max[0],
                        glyph.uv.max[1]
                    ]
                );
            }
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("rich-map-gui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&residency.gui_pipeline);
                pass.set_bind_group(
                    0,
                    &renderer
                        .pack
                        .item_frame_map_text_font_atlas
                        .as_ref()
                        .unwrap()
                        .gui_bind_group,
                    &[],
                );
                pass.set_vertex_buffer(0, residency.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, residency.text_buffer().unwrap().slice(..));
                pass.set_index_buffer(residency.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(
                    0..residency.index_count,
                    0,
                    0..residency.text_instances.len() as u32,
                );
            }
            let pending = renderer
                .prepare_screenshot_copy(&mut encoder, &target)
                .unwrap();
            renderer.queue.submit(Some(encoder.finish()));
            let pixels = renderer.read_screenshot_pixels(pending).unwrap();
            if !label.is_empty() {
                assert_eq!(
                    pixels.pixel(22, 16),
                    if alpha == 255 {
                        [255, 0, 0, 255]
                    } else {
                        [0, 127, 0, 255]
                    },
                    "textured glyph respects alpha and red tint"
                );
                assert_eq!(
                    pixels.pixel(28, 16),
                    if alpha == 255 {
                        [0, 127, 128, 255]
                    } else {
                        [0, 255, 0, 255]
                    },
                    "half-transparent blue shadow beyond the main glyph"
                );
                assert_eq!(
                    pixels.pixel(22, 29),
                    [0, 127, 128, 255],
                    "solid shadow survives transparent atlas"
                );
                assert_eq!(
                    pixels.pixel(22, 27),
                    [255, 0, 0, 255],
                    "solid underline survives transparent font atlas"
                );
            }
            assert_eq!(pixels.pixel(30, 27), [0, 255, 0, 255]);
            assert_eq!(
                pixels.pixel(20, 8),
                [0, 127, 0, 255],
                "black background blends over green"
            );
        }
    }
}

#[test]
fn player_marker_label_is_first_person_only() {
    let fonts = fonts();
    let runs = [WorldTextRun::plain("A")];
    assert!(bake_item_frame_map_text_surface(
        0,
        &runs,
        0,
        0,
        0,
        Mat4::IDENTITY,
        [1.0; 2],
        0,
        &fonts,
        0
    )
    .is_none());
    assert!(
        bake_hud_map_text_surface(0, &runs, 0, 0, 0, Mat4::IDENTITY, [1.0; 2], 0, &fonts, 0)
            .is_none()
    );
    assert!(bake_first_person_map_text_surface(
        0,
        &runs,
        0,
        0,
        0,
        Mat4::IDENTITY,
        [1.0; 2],
        0,
        &fonts,
        0
    )
    .is_some());
}

#[test]
fn world_and_first_person_text_faces_match_base_map_with_back_culling() {
    let Some(mut renderer) = Renderer::new_offscreen(128, 128) else {
        return;
    };
    renderer.render_offscreen_frame().unwrap();
    let projection = Mat4::from_scale(Vec3::new(1.0 / 64.0, -1.0 / 64.0, 0.5))
        * Mat4::from_translation(Vec3::new(-64.0, -64.0, 1.0));
    for binding in [
        renderer.camera_buffers.world(),
        renderer.camera_buffers.hud3d(),
    ] {
        renderer.queue.write_buffer(
            binding.buffer,
            binding.offset,
            bytemuck::bytes_of(&crate::camera::CameraUniform::identity()),
        );
        renderer.queue.write_buffer(
            binding.buffer,
            binding.offset,
            bytemuck::cast_slice(&projection.to_cols_array()),
        );
    }
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rich-map-world-readback"),
        size: wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: renderer.config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rich-map-world-depth"),
        size: wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let pose =
        Mat4::from_translation(Vec3::new(-170.0, -194.0, 0.0)) * Mat4::from_scale(Vec3::splat(3.0));
    for label in ["A", ""] {
        let runs = [WorldTextRun {
            text: label.into(),
            color: Some(0xff0000),
            shadow_color: Some(0x800000ff),
            style: HudTextStyle {
                underlined: true,
                ..Default::default()
            },
            ..Default::default()
        }];
        for map_target in [MapSurfaceTarget::World, MapSurfaceTarget::FirstPerson] {
            let bake_text = if map_target == MapSurfaceTarget::World {
                bake_item_frame_map_text_surface
            } else {
                bake_first_person_map_text_surface
            };
            let text = bake_text(1, &runs, 0, 0, 0, pose, [1.0; 2], 0, &fonts(), 0).unwrap();
            let base = bake_item_frame_map_surface(1, pose, [1.0; 2]);
            renderer.set_map_surfaces(
                vec![map_texture(1, [0, 255, 0, 255].repeat(128 * 128))],
                Vec::new(),
                if map_target == MapSurfaceTarget::World {
                    vec![base.clone()]
                } else {
                    Vec::new()
                },
                if map_target == MapSurfaceTarget::FirstPerson {
                    vec![base]
                } else {
                    Vec::new()
                },
            );
            for alpha in [255, 0] {
                renderer
                    .upload_hud_font_atlas(
                        1,
                        1,
                        &[255, 255, 255, alpha],
                        renderer.pack.hud_font_book.clone(),
                    )
                    .unwrap();
                renderer.set_map_text_surfaces(
                    if map_target == MapSurfaceTarget::World {
                        vec![text.clone()]
                    } else {
                        Vec::new()
                    },
                    if map_target == MapSurfaceTarget::FirstPerson {
                        vec![text.clone()]
                    } else {
                        Vec::new()
                    },
                );
                let residency = &renderer.pack.map_surface_residency;
                assert_eq!(residency.text_draws[0].target, map_target);
                let view = target.create_view(&wgpu::TextureViewDescriptor::default());
                let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("rich-map-world-culling"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
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
                    pass.set_pipeline(&residency.pipeline);
                    pass.set_bind_group(1, &renderer.lightmap.sample_bind_group, &[]);
                    pass.set_vertex_buffer(0, residency.vertex_buffer.slice(..));
                    pass.set_index_buffer(
                        residency.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.set_vertex_buffer(1, residency.dynamic_buffer().unwrap().slice(..));
                    let atlas = renderer.pack.item_frame_map_atlas.as_ref().unwrap();
                    pass.set_bind_group(
                        0,
                        atlas
                            .page_bind_group(residency.dynamic_draws[0].page, map_target)
                            .unwrap(),
                        &[],
                    );
                    pass.draw_indexed(0..residency.index_count, 0, 0..1);
                    let font = renderer
                        .pack
                        .item_frame_map_text_font_atlas
                        .as_ref()
                        .unwrap();
                    pass.set_bind_group(
                        0,
                        if map_target == MapSurfaceTarget::World {
                            &font.bind_group
                        } else {
                            &font.hud3d_bind_group
                        },
                        &[],
                    );
                    pass.set_vertex_buffer(1, residency.text_buffer().unwrap().slice(..));
                    pass.draw_indexed(
                        0..residency.index_count,
                        0,
                        0..residency.text_instances.len() as u32,
                    );
                }
                let pending = renderer
                    .prepare_screenshot_copy(&mut encoder, &target)
                    .unwrap();
                renderer.queue.submit(Some(encoder.finish()));
                let pixels = renderer.read_screenshot_pixels(pending).unwrap();
                let base_pixel = pixels.pixel(30, 27);
                assert!(
                    base_pixel[1] > 200 && base_pixel[0] < 10,
                    "{map_target:?}: base map must face camera: {base_pixel:?}"
                );
                assert_black_background(pixels.pixel(20, 8), base_pixel, map_target);
                if !label.is_empty() {
                    let glyph_pixel = pixels.pixel(22, 16);
                    if alpha == 255 {
                        assert!(
                            glyph_pixel[0] > 200 && glyph_pixel[1] < 10,
                            "{map_target:?}: glyph must face same side as base: {glyph_pixel:?}"
                        );
                    } else {
                        assert_black_background(glyph_pixel, base_pixel, map_target);
                    }
                    let shadow_pixel = pixels.pixel(28, 16);
                    if alpha == 255 {
                        assert!(
                            shadow_pixel[2] > 90 && shadow_pixel[1] < 150 && shadow_pixel[0] < 10,
                            "{map_target:?}: colored half-transparent shadow: {shadow_pixel:?}"
                        );
                    } else {
                        assert_eq!(shadow_pixel, base_pixel);
                    }
                    let effect_shadow = pixels.pixel(22, 29);
                    assert!(
                        effect_shadow[2] > 90 && effect_shadow[1] < 150 && effect_shadow[0] < 10,
                        "{map_target:?}: effect shadow: {effect_shadow:?}"
                    );
                    let effect_pixel = pixels.pixel(22, 27);
                    assert!(
                        effect_pixel[0] > 200 && effect_pixel[1] < 10,
                        "{map_target:?}: solid underline must face same side as base: {effect_pixel:?}"
                    );
                }
            }
        }
    }
}

fn assert_black_background(actual: [u8; 4], base: [u8; 4], target: MapSurfaceTarget) {
    for channel in 0..3 {
        let expected = (f32::from(base[channel]) * (127.0 / 255.0)).round() as i16;
        assert!(
            (i16::from(actual[channel]) - expected).abs() <= 1,
            "{target:?}: half-transparent black must blend over {base:?}, got {actual:?}"
        );
    }
    assert_eq!(actual[3], 255);
}

#[test]
fn explicit_shadow_obeys_all_map_target_depths_and_selected_offsets() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'A',
        HudAsciiGlyph {
            shadow_offset: 0.5,
            bold_offset: 0.5,
            ..glyph(3.25, 0.1)
        },
    );
    glyphs.insert_first_wins(
        ' ',
        HudAsciiGlyph {
            shadow_offset: 0.5,
            ..HudAsciiGlyph::empty(2.5)
        },
    );
    let fonts = HudFontGlyphBook::from(glyphs);
    for explicit in [None, Some(0), Some(0x8000ff00), Some(0x0000ff00)] {
        let runs = [WorldTextRun {
            text: "A ".into(),
            color: Some(0xff0000),
            shadow_color: explicit,
            style: HudTextStyle {
                bold: true,
                underlined: true,
                ..Default::default()
            },
            ..Default::default()
        }];
        for (flat, baker) in [
            (
                false,
                bake_item_frame_map_text_surface as fn(_, _, _, _, _, _, _, _, _, _) -> _,
            ),
            (false, bake_first_person_map_text_surface),
            (true, bake_hud_map_text_surface),
        ] {
            let surface: ItemFrameMapTextSurface =
                baker(1, &runs, 0, 0, 0, Mat4::IDENTITY, [1.0; 2], 0, &fonts, 0).unwrap();
            let shadow = explicit.is_some_and(|color| color != 0);
            let glyph_count = if shadow { 4 } else { 2 };
            assert_eq!(
                surface.glyphs.len(),
                glyph_count + if shadow { 4 } else { 2 }
            );
            let main = if shadow { 2 } else { 0 };
            near(
                local(&surface, main, Vec3::Y).z,
                if shadow && !flat { 0.03 } else { 0.0 },
            );
            near(
                local(&surface, main + 1, Vec3::Y).z,
                if flat {
                    0.0
                } else if shadow {
                    0.031
                } else {
                    0.001
                },
            );
            near(
                local(&surface, main + 1, Vec3::Y).x - local(&surface, main, Vec3::Y).x,
                0.5,
            );
            assert_eq!(surface.background.tint, [0.0, 0.0, 0.0, 128.0 / 255.0]);
            if shadow {
                assert_eq!(
                    surface.glyphs[0].tint,
                    [
                        0.0,
                        1.0,
                        0.0,
                        if explicit == Some(0x8000ff00) {
                            128.0 / 255.0
                        } else {
                            0.0
                        }
                    ]
                );
                near(
                    local(&surface, 0, Vec3::Y).x - local(&surface, main, Vec3::Y).x,
                    0.5,
                );
                near(local(&surface, glyph_count, Vec3::Y).z, 0.01);
                near(local(&surface, glyph_count + 1, Vec3::Y).z, 0.04);
                near(
                    local(&surface, glyph_count, Vec3::Y).x
                        - local(&surface, glyph_count + 1, Vec3::Y).x,
                    0.5,
                );
            }
        }
    }
}
