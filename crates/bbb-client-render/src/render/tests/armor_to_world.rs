use super::*;

#[test]
fn particle_instance_draw_samples_all_four_sprite_corners() {
    use crate::particles::{ParticleAtlasDrawRange, ParticleQuad, ParticleTextureAtlasKind};

    let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
        return;
    };
    let colors = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
    ];
    renderer
        .upload_particle_atlas(2, 2, &colors.concat(), vec![])
        .unwrap();
    let quad = ParticleQuad {
        corners: [
            [-1.0, -1.0, 0.5],
            [1.0, -1.0, 0.5],
            [1.0, 1.0, 0.5],
            [-1.0, 1.0, 0.5],
        ],
        uv: [0.0, 0.0, 1.0, 1.0],
        color: [1.0; 4],
        light: [1.0; 2],
    };
    let batch = &mut renderer.particle_frame.quads.opaque;
    batch.quads.extend([
        ParticleQuad {
            corners: [[0.0; 3]; 4],
            ..quad
        },
        quad,
    ]);
    batch.draws.push(ParticleAtlasDrawRange {
        texture_atlas: ParticleTextureAtlasKind::Particles,
        instances: 1..2,
    });
    let frame = renderer.render_offscreen_frame().unwrap();
    for ((x, y), color) in [(7, 8), (25, 8), (6, 23), (24, 23)].into_iter().zip(colors) {
        let actual = frame.pixel(x, y);
        for i in 0..3 {
            assert_eq!(
                actual[i] > 128,
                color[i] != 0,
                "pixel ({x}, {y}) = {actual:?}"
            );
        }
    }
}

#[test]
fn empty_item_and_particle_targets_do_not_claim_depth_work() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 12) else {
        return;
    };
    let mut particle_batches = crate::particles::ParticleBatches::default();
    assert!(!renderer.item_entity_target_has_depth_work());
    assert!(!renderer.particle_target_has_depth_work(&particle_batches));

    renderer
        .item_entity_billboards
        .push(crate::item_entities::ItemEntityBillboard::single(
            [0.0, 0.0, 0.0],
            crate::item_entities::ItemEntityUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        ));
    assert!(renderer.item_entity_target_has_depth_work());
    renderer.item_entity_billboards.clear();
    renderer.set_block_selection(Some(crate::selection::BlockSelectionRenderState {
        boxes: vec![crate::selection::SelectionBox {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        }],
        phase: crate::selection::BlockSelectionPhase::BeforeTranslucent,
        line_width: 2.5,
        high_contrast: false,
    }));
    assert!(renderer.item_entity_target_has_depth_work());

    particle_batches
        .translucent
        .quads
        .push(crate::particles::ParticleQuad {
            corners: [[0.0; 3]; 4],
            uv: [0.0; 4],
            color: [1.0, 1.0, 1.0, 1.0],
            light: [1.0, 1.0],
        });
    assert!(renderer.particle_target_has_depth_work(&particle_batches));
}

#[test]
fn frame_plan_selects_feature_attachments_and_transparency_combine() {
    let regular = FramePlan::new(
        crate::LevelFrameRoute::Regular,
        true,
        true,
        true,
        false,
        false,
    );
    assert!(!regular.improved_transparency);
    assert!(regular.update_lightmap);
    assert!(regular.item_entity_depth_work);
    assert!(regular.particle_depth_work);
    assert_eq!(regular.transparency_combine, FrameTransparencyCombine::None);

    let without_graphs = FramePlan::new(
        crate::LevelFrameRoute::ImprovedTransparency,
        false,
        true,
        true,
        false,
        false,
    );
    assert!(!without_graphs.improved_transparency);
    assert!(!without_graphs.update_lightmap);
    assert_eq!(
        without_graphs.transparency_combine,
        FrameTransparencyCombine::None
    );

    let generic_graph = FramePlan::new(
        crate::LevelFrameRoute::ImprovedTransparency,
        false,
        false,
        true,
        true,
        false,
    );
    assert!(generic_graph.improved_transparency);
    assert_eq!(
        generic_graph.transparency_combine,
        FrameTransparencyCombine::GenericGraph
    );

    let vanilla_direct = FramePlan::new(
        crate::LevelFrameRoute::ImprovedTransparency,
        false,
        true,
        false,
        true,
        true,
    );
    assert_eq!(
        vanilla_direct.transparency_combine,
        FrameTransparencyCombine::VanillaDirect
    );
}

#[test]
fn physical_scissor_from_gui_scales_by_integer_gui_scale_on_odd_framebuffers() {
    // Scale 4: an integer ratio.
    assert_eq!(
        physical_scissor_from_gui(4, PhysicalSize::new(1920, 1080), 120, 40, 16, 16,),
        Some((480, 160, 64, 64))
    );
    // Ceil layout width 427 must not change the physical scale of a GUI pixel.
    assert_eq!(
        physical_scissor_from_gui(3, PhysicalSize::new(1280, 720), 213, 120, 1, 1,),
        Some((639, 360, 3, 3))
    );
    assert_eq!(
        physical_scissor_from_gui(3, PhysicalSize::new(1280, 720), 0, 0, 427, 240,),
        Some((0, 0, 1280, 720))
    );
    assert_eq!(
        physical_scissor_from_gui(2, PhysicalSize::new(801, 603), 400, 301, 1, 1),
        Some((800, 602, 1, 1))
    );
    assert_eq!(
        physical_scissor_from_gui(2, PhysicalSize::new(801, 603), 300, 200, 1, 1),
        Some((600, 400, 2, 2))
    );
}

#[test]
fn physical_scissor_from_gui_clips_partially_outside_rects() {
    // A logical rect straddling the right GUI edge clips to the physical
    // attachment boundary: x=400..500 on a 480-wide GUI at scale 4 lands
    // at physical x=1600..1920 (the tail beyond 1920 is dropped).
    assert_eq!(
        physical_scissor_from_gui(4, PhysicalSize::new(1920, 1080), 400, 0, 100, 270,),
        Some((1600, 0, 320, 1080))
    );
    // A rect starting inside and ending past the bottom edge clips the
    // same way on y.
    assert_eq!(
        physical_scissor_from_gui(4, PhysicalSize::new(1920, 1080), 0, 260, 480, 20,),
        Some((0, 1040, 1920, 40))
    );
}

#[test]
fn physical_scissor_from_gui_fails_closed_for_fully_outside_rects() {
    // Entirely beyond the right GUI edge: no physical pixels, and the
    // callers must not issue a zero-area native scissor.
    assert_eq!(
        physical_scissor_from_gui(4, PhysicalSize::new(1920, 1080), 480, 0, 100, 270,),
        None
    );
    // Entirely below the GUI surface.
    assert_eq!(
        physical_scissor_from_gui(4, PhysicalSize::new(1920, 1080), 0, 270, 480, 100,),
        None
    );
    // Entirely outside on both axes at once.
    assert_eq!(
        physical_scissor_from_gui(3, PhysicalSize::new(1280, 720), 427, 240, 10, 10,),
        None
    );
}

#[cfg(feature = "lifecycle-soak")]
#[test]
fn lifecycle_soak_faults_use_the_real_reconfigure_and_skip_paths() {
    use crate::LifecycleSoakSurfaceError;

    let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer.config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    renderer
        .surface
        .configure(&renderer.device, &renderer.config);
    let clear = [17_u8, 34, 51, 113];
    let texel = [120_u8, 167, 255, 255];
    renderer.set_clear_color(crate::ClearColor {
        r: f64::from(clear[0]) / 255.0,
        g: f64::from(clear[1]) / 255.0,
        b: f64::from(clear[2]) / 255.0,
        a: f64::from(clear[3]) / 255.0,
    });
    renderer
        .add_test_gui_sprite("hud/hotbar", 182, 22, &texel.repeat(182 * 22))
        .expect("upload hotbar");
    renderer.set_hud_hotbar_mode(crate::HudHotbarMode::Ordinary);
    let screenshot_path = std::env::temp_dir().join(format!(
        "bbb-screenshot-retry-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    for error in [
        LifecycleSoakSurfaceError::Lost,
        LifecycleSoakSurfaceError::Outdated,
        LifecycleSoakSurfaceError::Timeout,
        LifecycleSoakSurfaceError::Occluded,
        LifecycleSoakSurfaceError::Suboptimal,
    ] {
        renderer.inject_lifecycle_soak_surface_error(error);
        assert!(matches!(
            renderer.render(Some(&screenshot_path)).unwrap(),
            crate::RenderOutcome::Skipped
        ));
        assert!(!screenshot_path.exists());
        assert_eq!(renderer.counters().screenshots_written, 0);
    }
    let counters = renderer.counters();
    assert_eq!(counters.surface_lost, 1);
    assert_eq!(counters.surface_outdated, 1);
    assert_eq!(counters.surface_timeouts, 1);
    assert_eq!(counters.surface_reconfigurations, 3);
    assert_eq!(counters.surface_frames_skipped, 5);
    assert_eq!(counters.frame_index, 0);

    let crate::RenderOutcome::Presented {
        screenshot: Some(Ok(image)),
        capture: Some(Ok(pixels)),
        ..
    } = renderer
        .render_with_capture(Some(&screenshot_path), true)
        .unwrap()
    else {
        panic!("retried screenshot was not completed");
    };
    assert_eq!(std::fs::read(&screenshot_path).unwrap(), image.bytes);
    let decoded = image::load_from_memory(&image.bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (32, 32));
    assert_eq!(decoded.to_rgba8().into_raw(), pixels.rgba);
    for (point, expected) in [((8, 8), clear), ((16, 30), texel)] {
        let actual = pixels.pixel(point.0, point.1);
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(&a, b)| a.abs_diff(b) <= 1),
            "RA-only surface screenshot/capture at {point:?}: {actual:?}"
        );
    }
    assert_eq!(renderer.counters().screenshots_written, 1);
    assert_eq!(renderer.counters().frame_index, 1);

    renderer.inject_lifecycle_soak_surface_error(LifecycleSoakSurfaceError::Validation);
    let error = renderer
        .render(None)
        .expect_err("surface validation must fail closed");
    assert!(error
        .to_string()
        .contains("surface acquisition validation error"));
    assert_eq!(renderer.counters().frame_index, 1);

    std::fs::remove_file(screenshot_path).unwrap();
}

#[test]
fn resident_item_model_entity_outline_draw_activates_the_shared_target() {
    use crate::item_models::{
        ItemModelAssetIdentity, ItemModelAssetRequest, ItemModelAssetSource, ItemModelAtlasKind,
        ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass,
        ItemModelInstanceRequest, ItemModelQuad, ItemModelResidencyFrame,
    };
    use std::sync::Arc;

    let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
        return;
    };
    let identity = ItemModelAssetIdentity::new("minecraft:carved_pumpkin", "facing=north", 0);
    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame {
            assets: vec![ItemModelAssetRequest {
                identity: identity.clone(),
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                source: ItemModelAssetSource::Quads(Arc::from([ItemModelQuad {
                    corners: [
                        [0.0, 0.0, 0.0],
                        [16.0, 0.0, 0.0],
                        [16.0, 16.0, 0.0],
                        [0.0, 16.0, 0.0],
                    ],
                    uvs: [[0.0, 0.0]; 4],
                    tint: [1.0; 4],
                    normal: [0.0, 0.0, 1.0],
                    shade: 1.0,
                    translucent: false,
                }])),
            }],
            instances: vec![ItemModelInstanceRequest {
                identity,
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                transform: glam::Mat4::IDENTITY,
                tint: [0.2, 0.4, 0.8, 1.0],
                light: [0.0; 2],
                overlay: [0.0; 2],
                foil: ItemModelFoil::None,
                target: ItemModelDrawTarget::EntityOutline,
                phase: ItemModelDrawPhase::Outline,
                submit_order: 0,
                scissor: None,
            }],
            special_items: Vec::new(),
        })
        .unwrap();

    assert!(renderer.pack.entity_model_texture_atlas.is_none());
    assert!(renderer.pack.entity_meshes[crate::renderer::EntityMeshLane::Outline].is_none());
    assert!(renderer.has_entity_outline_content());
}
