use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_client_assets::{PackRoots, PostEffectCatalog};
use winit::dpi::PhysicalSize;

use super::*;
use crate::item_models::{
    ItemModelAssetIdentity, ItemModelAssetRequest, ItemModelAssetSource, ItemModelAtlasKind,
    ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass,
    ItemModelInstanceRequest, ItemModelQuad, ItemModelResidencyFrame,
};
use crate::{
    ClearColor, EntityModelInstance, EntityModelTextureImage, HudInventoryFillLayer,
    HudInventoryFillStage, HudInventoryScreen, LevelFrameRoute, Renderer,
};

const TEST_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn vanilla_post_effect_identity_owner_is_complete_unique_and_camera_bounded() {
    let ids = VanillaPostEffect::REQUIRED.map(VanillaPostEffect::id);
    assert_eq!(
        ids,
        [
            "minecraft:blur",
            "minecraft:creeper",
            "minecraft:spider",
            "minecraft:invert",
            "minecraft:entity_outline",
            "minecraft:transparency",
        ]
    );
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        6
    );
    assert_eq!(
        VanillaPostEffect::camera_from_id(Some("minecraft:creeper")),
        Some(VanillaPostEffect::Creeper)
    );
    assert_eq!(
        VanillaPostEffect::camera_from_id(Some("minecraft:spider")),
        Some(VanillaPostEffect::Spider)
    );
    assert_eq!(
        VanillaPostEffect::camera_from_id(Some("minecraft:invert")),
        Some(VanillaPostEffect::Invert)
    );
    for non_camera in [
        None,
        Some("minecraft:blur"),
        Some("minecraft:entity_outline"),
        Some("minecraft:transparency"),
        Some("minecraft:unknown"),
    ] {
        assert_eq!(VanillaPostEffect::camera_from_id(non_camera), None);
    }
}

#[test]
fn post_effect_graph_executes_selected_custom_shader_pixels_in_pass_order() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let root = unique_temp_dir("post-effect-graph-pixels");
    write_two_pass_fixture(&root);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    let generation = PostEffectGraphGeneration::new(
        &device,
        &queue,
        TEST_FORMAT,
        4,
        4,
        &catalog.renderer_projection(),
    )
    .unwrap();
    assert_eq!(generation.generation(), 0);
    assert!(generation.contains_graph("demo:custom"));

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-post-effect-test-target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TEST_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    queue.write_texture(
        target.as_image_copy(),
        &[40, 80, 160, 255].repeat(16),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(4),
        },
        wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
    );
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let context = PostEffectExecutionContext {
        targets: BTreeMap::from([(
            "minecraft:main".to_string(),
            PostEffectExternalTarget {
                color: &view,
                depth: None,
                size: [4, 4],
            },
        )]),
        globals: PostEffectGlobals::default(),
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-post-effect-test-encoder"),
    });
    assert_eq!(
        generation
            .execute(&device, &queue, &mut encoder, "demo:custom", &context)
            .unwrap(),
        2
    );
    queue.submit(Some(encoder.finish()));
    let pixel = read_first_pixel(&device, &queue, &target, 4, 4);
    assert!(
        (29..=31).contains(&pixel[0])
            && (84..=86).contains(&pixel[1])
            && (41..=43).contains(&pixel[2])
            && pixel[3] >= 250,
        "custom target tint plus RGBA8 texture should produce [30,85,42,255], got {pixel:?}"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_effect_graph_preserves_one_pass_render_target_orientation() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let root = unique_temp_dir("post-effect-graph-orientation");
    write_text(
        &asset(&root, "demo", "post_effect/orientation.json"),
        r#"{
          "passes": [{
            "vertex_shader": "demo:core/fullscreen",
            "fragment_shader": "demo:post/blit",
            "inputs": [{"sampler_name":"In","target":"demo:source"}],
            "output": "minecraft:main"
          }]
        }"#,
    );
    write_screenquad(&root);
    write_text(
        &asset(&root, "demo", "shaders/post/blit.fsh"),
        r#"#version 330
uniform sampler2D InSampler;
in vec2 texCoord;
out vec4 fragColor;
void main() {
    fragColor = texture(InSampler, texCoord);
}
"#,
    );
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    let generation = PostEffectGraphGeneration::new(
        &device,
        &queue,
        TEST_FORMAT,
        4,
        4,
        &catalog.renderer_projection(),
    )
    .unwrap();
    let source = create_test_target(&device, "bbb-post-effect-orientation-source");
    let destination = create_test_target(&device, "bbb-post-effect-orientation-destination");
    let mut pixels = Vec::with_capacity(64);
    for row in 0..4 {
        pixels.extend_from_slice(&if row < 2 {
            [255, 0, 0, 255].repeat(4)
        } else {
            [0, 0, 255, 255].repeat(4)
        });
    }
    queue.write_texture(
        source.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(4),
        },
        wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
    );
    let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
    let destination_view = destination.create_view(&wgpu::TextureViewDescriptor::default());
    let context = PostEffectExecutionContext {
        targets: BTreeMap::from([
            (
                "demo:source".to_string(),
                PostEffectExternalTarget {
                    color: &source_view,
                    depth: None,
                    size: [4, 4],
                },
            ),
            (
                "minecraft:main".to_string(),
                PostEffectExternalTarget {
                    color: &destination_view,
                    depth: None,
                    size: [4, 4],
                },
            ),
        ]),
        globals: PostEffectGlobals::default(),
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-post-effect-orientation-encoder"),
    });
    generation
        .execute(&device, &queue, &mut encoder, "demo:orientation", &context)
        .unwrap();
    queue.submit(Some(encoder.finish()));

    let pixels = read_pixels(&device, &queue, &destination, 4, 4);
    assert_eq!(&pixels[..4], &[255, 0, 0, 255]);
    assert_eq!(&pixels[48..52], &[0, 0, 255, 255]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_effect_graph_resize_recreates_only_size_dependent_targets() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let root = unique_temp_dir("post-effect-graph-resize");
    write_text(
        &asset(&root, "demo", "post_effect/targets.json"),
        r#"{
          "targets": {
            "demo:full": {"persistent": true},
            "demo:fixed": {"width": 2, "height": 3, "persistent": true}
          }
        }"#,
    );
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    let mut generation = PostEffectGraphGeneration::new(
        &device,
        &queue,
        TEST_FORMAT,
        4,
        5,
        &catalog.renderer_projection(),
    )
    .unwrap();
    {
        let graph = &generation.graphs["demo:targets"];
        assert!(
            !graph.targets_materialized(),
            "internal post-effect targets should allocate lazily on first execute"
        );
    }

    let pre_execute_resize = generation
        .prepare_resize(&device, 6, 7)
        .unwrap()
        .expect("size change prepares resize plan even before materialization");
    generation.apply_resize(pre_execute_resize);
    assert!(
        !generation.graphs["demo:targets"].targets_materialized(),
        "resizing before first execute must not materialize unused graph targets"
    );

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-post-effect-resize-clear"),
    });
    assert_eq!(
        generation
            .execute(
                &device,
                &queue,
                &mut encoder,
                "demo:targets",
                &PostEffectExecutionContext::default(),
            )
            .unwrap(),
        0
    );
    queue.submit(Some(encoder.finish()));
    {
        let graph = &generation.graphs["demo:targets"];
        assert!(graph.targets_materialized());
        assert_eq!(graph.target_size("demo:full"), Some([6, 7]));
        assert_eq!(graph.target_size("demo:fixed"), Some([2, 3]));
        assert_eq!(graph.target_needs_clear("demo:full"), Some(false));
        assert_eq!(graph.target_needs_clear("demo:fixed"), Some(false));
    }

    let resize = generation
        .prepare_resize(&device, 7, 8)
        .unwrap()
        .expect("size change prepares graph target replacements");
    generation.apply_resize(resize);
    let graph = &generation.graphs["demo:targets"];
    assert_eq!(graph.target_size("demo:full"), Some([7, 8]));
    assert_eq!(graph.target_needs_clear("demo:full"), Some(true));
    assert_eq!(graph.target_size("demo:fixed"), Some([2, 3]));
    assert_eq!(graph.target_needs_clear("demo:fixed"), Some(false));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_effect_graph_rejects_context_before_mutating_persistent_targets() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let root = unique_temp_dir("post-effect-graph-context");
    write_text(
        &asset(&root, "demo", "post_effect/context.json"),
        r#"{
          "targets": {"demo:persistent": {"persistent": true}},
          "passes": [{
            "vertex_shader": "demo:core/fullscreen",
            "fragment_shader": "demo:post/solid",
            "output": "demo:missing"
          }]
        }"#,
    );
    write_screenquad(&root);
    write_text(
        &asset(&root, "demo", "shaders/post/solid.fsh"),
        "#version 330\nin vec2 texCoord;\nout vec4 fragColor;\nvoid main() { fragColor = vec4(texCoord * 0.0, 1.0, 1.0); }\n",
    );
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    let mut generation = PostEffectGraphGeneration::new(
        &device,
        &queue,
        TEST_FORMAT,
        4,
        4,
        &catalog.renderer_projection(),
    )
    .unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-post-effect-invalid-context-encoder"),
    });
    let error = generation
        .execute(
            &device,
            &queue,
            &mut encoder,
            "demo:context",
            &PostEffectExecutionContext::default(),
        )
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("external output target demo:missing is not allowed"),
        "{error:#}"
    );
    assert!(
        !generation.graphs["demo:context"].targets_materialized(),
        "failed execution must not materialize internal targets"
    );

    // Also verify an already-materialized persistent target stays untouched when
    // a subsequent execution fails validation.
    let output_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-post-effect-valid-output"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TEST_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut valid_context = PostEffectExecutionContext::default();
    valid_context.targets.insert(
        "demo:missing".to_string(),
        PostEffectExternalTarget {
            color: &output_view,
            depth: None,
            size: [4, 4],
        },
    );
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-post-effect-valid-context-encoder"),
    });
    assert_eq!(
        generation
            .execute(
                &device,
                &queue,
                &mut encoder,
                "demo:context",
                &valid_context
            )
            .unwrap(),
        1
    );
    queue.submit(Some(encoder.finish()));
    let resize = generation
        .prepare_resize(&device, 5, 5)
        .unwrap()
        .expect("resize marks persistent target for clear");
    generation.apply_resize(resize);
    assert_eq!(
        generation.graphs["demo:context"].target_needs_clear("demo:persistent"),
        Some(true)
    );
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-post-effect-invalid-context-encoder-2"),
    });
    assert!(generation
        .execute(
            &device,
            &queue,
            &mut encoder,
            "demo:context",
            &PostEffectExecutionContext::default(),
        )
        .is_err());
    assert_eq!(
        generation.graphs["demo:context"].target_needs_clear("demo:persistent"),
        Some(true),
        "failed execution must not advance persistent target clear state"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_effect_graph_reports_shader_compilation_failure_without_installing() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let root = unique_temp_dir("post-effect-graph-compile-error");
    write_text(
        &asset(&root, "demo", "post_effect/bad.json"),
        r#"{
          "passes": [{
            "vertex_shader": "demo:core/fullscreen",
            "fragment_shader": "demo:post/bad",
            "output": "minecraft:main"
          }]
        }"#,
    );
    write_screenquad(&root);
    write_text(
        &asset(&root, "demo", "shaders/post/bad.fsh"),
        "#version 330\nout vec4 fragColor;\nvoid main() { fragColor = ; }\n",
    );
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    let error = format!(
        "{:#}",
        PostEffectGraphGeneration::new(
            &device,
            &queue,
            TEST_FORMAT,
            4,
            4,
            &catalog.renderer_projection(),
        )
        .err()
        .expect("malformed GLSL must reject the staged generation")
    );
    assert!(error.contains("GLSL parse failed"), "{error}");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 sources and a GPU adapter"]
fn compiles_all_local_vanilla_post_effect_graphs() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let catalog = PostEffectCatalog::load(&PackRoots::from_environment().unwrap()).unwrap();
    let generation = PostEffectGraphGeneration::new(
        &device,
        &queue,
        TEST_FORMAT,
        64,
        64,
        &catalog.renderer_projection(),
    )
    .unwrap();
    for id in [
        "minecraft:blur",
        "minecraft:creeper",
        "minecraft:spider",
        "minecraft:invert",
        "minecraft:entity_outline",
        "minecraft:transparency",
    ] {
        assert!(generation.contains_graph(id), "{id}");
    }
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    assert!(
        renderer
            .post_effect_graphs
            .as_ref()
            .unwrap()
            .uses_vanilla_transparency_direct_path(),
        "the direct-depth path is gated by the exact Vanilla 26.1 graph and shader hashes"
    );
    renderer.set_level_frame_route(LevelFrameRoute::ImprovedTransparency);
    renderer.set_gpu_timing_enabled(true);
    #[cfg(feature = "lifecycle-soak")]
    {
        renderer.inject_lifecycle_soak_surface_error(crate::LifecycleSoakSurfaceError::Lost);
        assert!(matches!(
            renderer.render(None).unwrap(),
            crate::RenderOutcome::Skipped
        ));
        renderer.inject_lifecycle_soak_surface_error(crate::LifecycleSoakSurfaceError::Validation);
        assert!(renderer.render(None).is_err());
    }
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_pose(Some(crate::CameraPose {
        position: [0.0; 3],
        eye_height: 0.0,
        y_rot: 0.0,
        x_rot: 0.0,
    }));
    renderer.set_debug_lines(vec![crate::DebugLine::new(
        [-0.5, 0.0, 2.0],
        [0.5, 0.0, 2.0],
        [255, 0, 0, 128],
        3.0,
    )]);
    let direct = renderer.render_offscreen_frame().unwrap();
    assert!(direct
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|p| p[0].abs_diff(128) <= 1 && p[1] == 0 && p[2].abs_diff(127) <= 1));
    assert_eq!(direct.pixel(0, 0), [0, 0, 255, 255]);
    let crate::RenderOutcome::Presented {
        capture: Some(captured),
        ..
    } = renderer.render_with_capture(None, true).unwrap()
    else {
        panic!("expected captured transparency frame")
    };
    assert_eq!(direct.rgba, captured.unwrap().rgba);
    if renderer.gpu_timing.is_some() {
        assert!(renderer.take_gpu_timing_profile().is_some());
    }

    let skin = crate::EntityModelTextureRef {
        path: "textures/entity/player/wide/steve.png",
        size: [64, 64],
    };
    renderer
        .upload_entity_model_textures(&[EntityModelTextureImage::new(
            skin,
            [25, 220, 55, 255].repeat(64 * 64),
        )])
        .unwrap();
    let mut previous = direct.rgba;
    for rotation in [0.15, -0.65] {
        renderer.set_first_person_player_arms(vec![crate::FirstPersonPlayerArm {
            left: false,
            skin: crate::EntityPlayerSkin::Default(crate::EntityDefaultPlayerSkin::WideSteve),
            sleeve_visible: true,
            light: [1.0; 2],
            transform: glam::Mat4::from_translation(glam::Vec3::new(0.1, -0.15, 1.2))
                * glam::Mat4::from_rotation_z(rotation),
        }]);
        let hand = renderer.render_offscreen_frame().unwrap();
        let crate::RenderOutcome::Presented {
            capture: Some(captured),
            ..
        } = renderer.render_with_capture(None, true).unwrap()
        else {
            panic!("expected captured hand")
        };
        assert!(
            hand.rgba == captured.unwrap().rgba,
            "direct hand must match captured pixels"
        );
        assert!(
            hand.rgba != previous,
            "the arm pose must change visible pixels"
        );
        previous = hand.rgba;
    }
    renderer.set_first_person_player_arms(Vec::new());

    // The opaque line also writes the item target's depth before it becomes inactive.
    renderer.set_debug_lines(vec![crate::DebugLine::new(
        [-0.5, 0.0, 2.0],
        [0.5, 0.0, 2.0],
        [255, 0, 0, 255],
        3.0,
    )]);
    renderer.render(None).unwrap();
    renderer.set_debug_lines(Vec::new());
    let empty = renderer.render_offscreen_frame().unwrap();
    assert!(
        empty
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|&pixel| pixel == [0, 0, 255, 255]),
        "an inactive item-entity target must not retain the previous frame's red line"
    );
    let pack = unique_temp_dir("previously-omitted-transparency-input");
    write_live_catalog_with_overrides(&pack, &[]);
    write_text(
        &asset(&pack, "minecraft", "post_effect/transparency.json"),
        r#"{"targets":{"minecraft:comparison":{}},"passes":[{
            "vertex_shader":"minecraft:core/screenquad","fragment_shader":"minecraft:post/compare_depth",
            "inputs":[{"sampler_name":"In","target":"minecraft:item_entity"},
                      {"sampler_name":"MainDepth","target":"minecraft:main","use_depth_buffer":true},
                      {"sampler_name":"ItemDepth","target":"minecraft:item_entity","use_depth_buffer":true},
                      {"sampler_name":"ParticleDepth","target":"minecraft:particles","use_depth_buffer":true}],
            "output":"minecraft:comparison"
        },{
            "vertex_shader":"minecraft:core/screenquad","fragment_shader":"minecraft:post/blit",
            "inputs":[{"sampler_name":"In","target":"minecraft:comparison"}],"output":"minecraft:main"
        }]}"#,
    );
    write_text(
        &asset(&pack, "minecraft", "shaders/post/compare_depth.fsh"),
        r#"#version 330
uniform sampler2D InSampler;
uniform sampler2D MainDepthSampler;
uniform sampler2D ItemDepthSampler;
uniform sampler2D ParticleDepthSampler;
in vec2 texCoord;
out vec4 fragColor;
void main() {
    float mainDepth = texture(MainDepthSampler, texCoord).r;
    bool matchDepth = texture(ItemDepthSampler, texCoord).r == mainDepth
                   && texture(ParticleDepthSampler, texCoord).r == mainDepth;
    fragColor = matchDepth ? texture(InSampler, texCoord) : vec4(1.0);
}
"#,
    );
    let custom = PostEffectCatalog::load(&fixture_roots(&pack)).unwrap();
    renderer
        .reload_post_effect_graphs(&custom.renderer_projection())
        .unwrap();
    let sampled = renderer.render_offscreen_frame().unwrap();
    assert!(
        sampled
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|&pixel| pixel == [0; 4]),
        "the replacement graph must see clear item color and current opaque depth for both inactive layers"
    );
    std::fs::remove_dir_all(pack).unwrap();
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    renderer.set_level_frame_route(LevelFrameRoute::Regular);
    renderer.set_camera_post_effect(Some("minecraft:invert"), true);
    let invert = renderer
        .render_offscreen_frame()
        .expect("official invert graph");
    let corner = invert.pixel(4, 4);
    assert!(
        corner[0] > corner[2].saturating_add(50) && corner[1] > corner[2].saturating_add(50),
        "0.8 inverse of blue should be yellow-dominant, got {corner:?}"
    );
    assert_eq!(renderer.counters.post_effect_draw_calls, 2);

    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    });
    renderer.set_camera_post_effect(Some("minecraft:creeper"), true);
    let creeper = renderer
        .render_offscreen_frame()
        .expect("official creeper graph");
    let corner = creeper.pixel(4, 4);
    assert!(
        corner[1] > 180 && corner[0] < 64 && corner[2] < 64,
        "creeper color matrix and bits pass should retain green only, got {corner:?}"
    );
    assert_eq!(renderer.counters.post_effect_draw_calls, 2);

    renderer.set_clear_color(ClearColor {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_post_effect(Some("minecraft:spider"), true);
    let spider = renderer
        .render_offscreen_frame()
        .expect("official spider graph");
    let corner = spider.pixel(4, 4);
    assert!(
        corner[0] > corner[1].saturating_add(10) && corner[1].abs_diff(corner[2]) < 5,
        "spider's final [1, 0.8, 0.8, 1] blit should red-tint white, got {corner:?}"
    );
    assert_eq!(renderer.counters.post_effect_draw_calls, 10);
}

#[test]
fn selected_generation_drives_live_camera_graphs_before_ordinary_hud() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let root = unique_temp_dir("post-effect-live-camera");
    let effects = [
        ("creeper", [0.0, 1.0, 0.0, 1.0]),
        ("spider", [1.0, 0.0, 1.0, 1.0]),
        ("invert", [1.0, 1.0, 0.0, 1.0]),
    ];
    write_live_catalog_with_overrides(&root, &effects);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    assert!(
        !renderer
            .post_effect_graphs
            .as_ref()
            .unwrap()
            .uses_vanilla_transparency_direct_path(),
        "a resource-pack graph override must retain generic graph execution"
    );
    assert_eq!(renderer.post_effect_graph_generation(), Some(0));
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer
        .add_test_gui_sprite("hud/crosshair", 4, 4, &[255, 0, 0, 255].repeat(16))
        .expect("crosshair sentinel");
    let crosshair = (12, 10);

    renderer.set_camera_post_effect(Some("minecraft:invert"), false);
    let inactive = renderer.render_offscreen_frame().unwrap();
    assert_eq!(inactive.pixel(2, 2), [0, 0, 255, 255]);
    assert_eq!(renderer.counters.post_effect_draw_calls, 0);

    for (graph, color) in effects {
        renderer.set_camera_post_effect(Some(&format!("minecraft:{graph}")), true);
        let pixels = renderer.render_offscreen_frame().unwrap();
        let expected = color.map(|channel| (channel * 255.0) as u8);
        assert_eq!(
            pixels.pixel(2, 2),
            expected,
            "{graph} graph owns the main target"
        );
        assert_eq!(
            pixels.pixel(crosshair.0, crosshair.1),
            [255 - expected[0], expected[1], expected[2], 255],
            "the red crosshair inverts red after the {graph} graph"
        );
        assert_eq!(renderer.counters.post_effect_draw_calls, 2);
    }

    renderer.resize(PhysicalSize::new(31, 23)).unwrap();
    assert_eq!(renderer.post_effect_graph_generation(), Some(0));
    let resized = renderer.render_offscreen_frame().unwrap();
    assert_eq!(
        resized.pixel(2, 2),
        [255, 255, 0, 255],
        "selected generation must retain its graph and shader after resize"
    );
    assert_eq!(renderer.counters.post_effect_draw_calls, 2);
    for id in ["minecraft:entity_outline", "minecraft:unknown"] {
        renderer.set_camera_post_effect(Some(id), true);
        let unrelated = renderer.render_offscreen_frame().unwrap();
        assert_eq!(
            unrelated.pixel(2, 2),
            [0, 0, 255, 255],
            "{id} must not run as a camera effect"
        );
        assert_eq!(renderer.counters.post_effect_draw_calls, 0);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_generation_drives_live_transparency_graph() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let root = unique_temp_dir("post-effect-live-transparency");
    write_live_catalog_with_overrides(&root, &[("transparency", [1.0, 0.0, 1.0, 1.0])]);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    renderer.set_level_frame_route(LevelFrameRoute::ImprovedTransparency);
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });

    let pixels = renderer.render_offscreen_frame().unwrap();
    let corner = pixels.pixel(2, 2);
    assert!(
        corner[0] > 240 && corner[1] < 8 && corner[2] > 240,
        "selected transparency override must own the main composition, got {corner:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_generation_drives_live_menu_blur_graph_and_pass_count() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let root = unique_temp_dir("post-effect-live-blur");
    write_live_catalog_with_overrides(&root, &[("blur", [0.0, 1.0, 0.0, 1.0])]);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    // Keep the menu surface transparent to isolate the blur shader's output.
    renderer
        .upload_gui_texture(
            "textures/gui/inworld_menu_background.png",
            1,
            1,
            &[0; 4],
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    renderer.set_hud_inventory_screen(Some(HudInventoryScreen {
        width: 24,
        height: 20,
        background_layers: Vec::new(),
        foreground_layers: Vec::new(),
        fill_layers: vec![HudInventoryFillLayer {
            x: 11,
            y: 11,
            width: 4,
            height: 4,
            tint: [1.0, 0.0, 0.0, 1.0],
            stage: HudInventoryFillStage::Foreground,
        }],
        slots: Vec::new(),
        floating_items: Vec::new(),
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
    renderer.set_hud_screen_blur(true);

    let pixels = renderer.render_offscreen_frame().unwrap();
    let corner = pixels.pixel(2, 2);
    assert!(
        corner[0] < 8 && corner[1] > 240 && corner[2] < 8,
        "selected blur override must own the main target, got {corner:?}"
    );
    assert_eq!(
        pixels.pixel(12, 12),
        [255, 0, 0, 255],
        "screen strata draw sharp after the blur"
    );
    assert_eq!(
        renderer.counters.screen_blur_draw_calls, 2,
        "the existing counter reports selected graph passes, excluding presentation"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_generation_drives_live_entity_outline_graph_before_composite() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let root = unique_temp_dir("post-effect-live-outline");
    write_live_catalog_with_overrides(&root, &[("entity_outline", [1.0, 0.0, 0.0, 1.0])]);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    let cow_textures = crate::entity_model_texture_refs()
        .iter()
        .filter(|texture| texture.path.starts_with("textures/entity/cow/cow_"))
        .map(|texture| {
            EntityModelTextureImage::new(
                *texture,
                [255, 255, 255, 255]
                    .repeat(usize::try_from(texture.size[0] * texture.size[1]).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    renderer
        .upload_entity_model_textures(&cow_textures)
        .unwrap();
    renderer.set_entity_model_instances(vec![EntityModelInstance::cow(
        1,
        [0.0, 0.0, 0.0],
        0.0,
        false,
    )
    .with_outline_color(0xffff_ffff)]);

    let pixels = renderer.render_offscreen_frame().unwrap();
    let corner = pixels.pixel(2, 2);
    assert!(
        corner[0] > 240 && corner[1] < 8 && corner[2] < 8,
        "selected red outline graph must fill the outline target before composition, got {corner:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_effect_globals_uses_eye_position_and_vanilla_camera_offset_sign() {
    let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
        return;
    };
    renderer.set_camera_pose(Some(crate::CameraPose {
        position: [12.25, 64.5, -3.75],
        eye_height: 1.62,
        y_rot: 0.0,
        x_rot: 0.0,
    }));
    let globals = renderer.post_effect_globals();
    // Eye Y = 64.5 + 1.62 = 66.12 -> block 66, offset 66.0 - 66.12 = -0.12.
    assert_eq!(globals.camera_block_pos, [12, 66, -4]);
    assert!((globals.camera_offset[0] - (-0.25)).abs() < 1e-5);
    assert!((globals.camera_offset[1] - (-0.12)).abs() < 1e-4);
    assert!((globals.camera_offset[2] - (-0.25)).abs() < 1e-5);
}

#[test]
fn entity_outline_composites_after_improved_transparency_combine() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let root = unique_temp_dir("post-effect-outline-after-transparency");
    write_live_catalog_with_overrides(
        &root,
        &[
            ("entity_outline", [1.0, 0.0, 0.0, 1.0]),
            ("transparency", [0.0, 1.0, 0.0, 1.0]),
        ],
    );
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    renderer.set_level_frame_route(crate::LevelFrameRoute::ImprovedTransparency);
    let cow_textures = crate::entity_model_texture_refs()
        .iter()
        .filter(|texture| texture.path.starts_with("textures/entity/cow/cow_"))
        .map(|texture| {
            EntityModelTextureImage::new(
                *texture,
                [255, 255, 255, 255]
                    .repeat(usize::try_from(texture.size[0] * texture.size[1]).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    renderer
        .upload_entity_model_textures(&cow_textures)
        .unwrap();
    renderer.set_entity_model_instances(vec![EntityModelInstance::cow(
        1,
        [0.0, 0.0, 0.0],
        0.0,
        false,
    )
    .with_outline_color(0xffff_ffff)]);

    let pixels = renderer.render_offscreen_frame().unwrap();
    let corner = pixels.pixel(2, 2);
    assert!(
        corner[0] > 240 && corner[1] < 8 && corner[2] < 8,
        "entity outline must composite after transparency combine (expected red, got {corner:?})"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resident_item_model_main_and_entity_outline_copy_reach_both_gpu_targets() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let root = unique_temp_dir("resident-block-main-outline");
    write_live_catalog_with_overrides(&root, &[("entity_outline", [1.0, 0.0, 0.0, 1.0])]);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    renderer
        .reload_post_effect_graphs(&catalog.renderer_projection())
        .unwrap();
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer
        .update_terrain_texture_atlas(&[255, 255, 255, 255])
        .unwrap();
    renderer.update_camera();

    let identity = ItemModelAssetIdentity::new("minecraft:stone", "plain", 0);
    let asset = ItemModelAssetRequest {
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
    };
    let main = ItemModelInstanceRequest {
        identity,
        atlas: ItemModelAtlasKind::Blocks,
        geometry_class: ItemModelGeometryClass::Solid,
        transform: glam::Mat4::IDENTITY,
        tint: [0.0, 1.0, 0.0, 1.0],
        light: [1.0; 2],
        overlay: [0.0, 10.0],
        foil: ItemModelFoil::None,
        target: ItemModelDrawTarget::BlockFeature,
        phase: ItemModelDrawPhase::Solid,
        submit_order: 0,
        scissor: None,
    };
    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame {
            assets: vec![asset.clone()],
            instances: vec![main.clone()],
            special_items: Vec::new(),
        })
        .unwrap();

    let main_pixels = renderer.render_offscreen_frame().unwrap();
    assert!(
        main_pixels.rgba.as_chunks::<4>().0.iter().any(|pixel| {
            pixel[1] > 96
                && u16::from(pixel[1]) > u16::from(pixel[0]) + 32
                && u16::from(pixel[1]) > u16::from(pixel[2]) + 32
        }),
        "the ordinary BlockFeature instance must produce green main-target pixels"
    );

    let mut outline = main.clone();
    outline.tint = [1.0, 0.0, 0.0, 1.0];
    outline.target = ItemModelDrawTarget::EntityOutline;
    outline.phase = ItemModelDrawPhase::Outline;
    outline.submit_order = 1;
    renderer
        .set_item_model_residency_frame(ItemModelResidencyFrame {
            assets: vec![asset],
            instances: vec![main, outline],
            special_items: Vec::new(),
        })
        .unwrap();

    let outlined = renderer.render_offscreen_frame().unwrap();
    let corner = outlined.pixel(2, 2);
    assert!(
        corner[0] > 240 && corner[1] < 8 && corner[2] < 8,
        "the resident outline copy must activate and feed the selected red entity-outline graph, got {corner:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_generation_rollback_restores_exact_previous_owner_and_counters() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let red_root = unique_temp_dir("post-effect-live-red");
    let green_root = unique_temp_dir("post-effect-live-green");
    let bad_root = unique_temp_dir("post-effect-live-bad");
    write_live_catalog(&red_root, [1.0, 0.0, 0.0, 1.0]);
    write_live_catalog(&green_root, [0.0, 1.0, 0.0, 1.0]);
    write_live_catalog(&bad_root, [0.0, 0.0, 1.0, 1.0]);
    write_text(
        &asset(&bad_root, "minecraft", "shaders/post/invert_override.fsh"),
        "#version 330\nuniform sampler2D InSampler;\nout vec4 fragColor;\nvoid main() { fragColor = texture(InSampler, vec2(0.0)) + ; }\n",
    );
    let red = PostEffectCatalog::load(&fixture_roots(&red_root)).unwrap();
    let green = PostEffectCatalog::load(&fixture_roots(&green_root)).unwrap();
    let bad = PostEffectCatalog::load(&fixture_roots(&bad_root)).unwrap();
    renderer
        .reload_post_effect_graphs(&red.renderer_projection())
        .unwrap();
    renderer.counters.draw_calls = 123;

    let error = renderer
        .reload_post_effect_graphs(&bad.renderer_projection())
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("GLSL parse failed"),
        "staged compile error should retain shader context: {error:#}"
    );
    assert_eq!(renderer.post_effect_graph_generation(), Some(0));
    assert_eq!(renderer.counters.draw_calls, 123);

    let error = renderer
        .transactional_resource_reload(|renderer| -> anyhow::Result<()> {
            renderer.reload_post_effect_graphs(&green.renderer_projection())?;
            renderer.counters.draw_calls = 999;
            anyhow::bail!("later staged upload failed")
        })
        .unwrap_err();
    assert!(error.to_string().contains("later staged upload failed"));
    assert_eq!(renderer.counters.draw_calls, 123);
    renderer.set_camera_post_effect(Some("minecraft:invert"), true);
    let pixels = renderer.render_offscreen_frame().unwrap();
    let corner = pixels.pixel(2, 2);
    assert!(
        corner[0] > 240 && corner[1] < 8 && corner[2] < 8,
        "rollback must restore the red generation rather than retain green, got {corner:?}"
    );

    std::fs::remove_dir_all(red_root).unwrap();
    std::fs::remove_dir_all(green_root).unwrap();
    std::fs::remove_dir_all(bad_root).unwrap();
}

fn request_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("bbb-post-effect-test-device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        ..Default::default()
    }))
    .ok()
}

fn read_first_pixel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> [u8; 4] {
    read_pixels(device, queue, texture, width, height)[..4]
        .try_into()
        .unwrap()
}

fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    const PADDED_BYTES_PER_ROW: u32 = 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bbb-post-effect-test-readback"),
        size: u64::from(PADDED_BYTES_PER_ROW) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bbb-post-effect-test-readback-encoder"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PADDED_BYTES_PER_ROW),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    receiver.recv().unwrap().unwrap();
    let mapped = slice.get_mapped_range().expect("mapped GPU readback");
    let row_bytes = usize::try_from(width).unwrap() * 4;
    let pixels = mapped
        .chunks_exact(usize::try_from(PADDED_BYTES_PER_ROW).unwrap())
        .take(usize::try_from(height).unwrap())
        .flat_map(|row| row[..row_bytes].iter().copied())
        .collect();
    drop(mapped);
    readback.unmap();
    pixels
}

fn create_test_target(device: &wgpu::Device, label: &str) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TEST_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn write_two_pass_fixture(root: &Path) {
    write_text(
        &asset(root, "demo", "post_effect/custom.json"),
        r#"{
          "targets": {"demo:swap": {"clear_color": -16777216}},
          "passes": [
            {
              "vertex_shader": "demo:core/fullscreen",
              "fragment_shader": "demo:post/tint",
              "inputs": [
                {"sampler_name":"In","target":"minecraft:main"},
                {
                  "sampler_name":"Noise",
                  "location":"demo:noise",
                  "width":1,
                  "height":1,
                  "bilinear":true
                }
              ],
              "output": "demo:swap",
              "uniforms": {
                "Globals": [
                  {"name":"Tint","type":"vec4","value":[0.5,1.0,0.25,1.0]}
                ]
              }
            },
            {
              "vertex_shader": "demo:core/fullscreen",
              "fragment_shader": "demo:post/blit",
              "inputs": [{"sampler_name":"In","target":"demo:swap"}],
              "output": "minecraft:main"
            }
          ]
        }"#,
    );
    write_screenquad(root);
    write_png(
        &asset(root, "demo", "textures/effect/noise.png"),
        [10, 5, 2, 255],
    );
    write_text(
        &asset(root, "demo", "shaders/post/tint.fsh"),
        r#"#version 330
uniform sampler2D InSampler;
uniform sampler2D NoiseSampler;
layout(std140) uniform SamplerInfo {
    vec2 OutSize;
    vec2 InSize;
    vec2 NoiseSize;
};
layout(std140) uniform Globals {
    vec4 Tint;
};
in vec2 texCoord;
out vec4 fragColor;
void main() {
    vec4 tinted = texture(InSampler, texCoord) * Tint;
    fragColor = vec4(tinted.rgb + texture(NoiseSampler, texCoord).rgb, tinted.a);
}
"#,
    );
    write_text(
        &asset(root, "demo", "shaders/post/blit.fsh"),
        r#"#version 330
uniform sampler2D InSampler;
in vec2 texCoord;
out vec4 fragColor;
void main() {
    fragColor = texture(InSampler, texCoord);
}
"#,
    );
}

fn write_screenquad(root: &Path) {
    write_text(
        &asset(root, "demo", "shaders/core/fullscreen.vsh"),
        r#"#version 330
out vec2 texCoord;
void main() {
    vec2 uv = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    gl_Position = vec4(uv * vec2(2, 2) + vec2(-1, -1), 0, 1);
    texCoord = uv;
}
"#,
    );
}

fn write_live_catalog(root: &Path, invert_color: [f32; 4]) {
    write_live_catalog_with_overrides(root, &[("invert", invert_color)]);
}

fn write_live_catalog_with_overrides(root: &Path, overrides: &[(&str, [f32; 4])]) {
    for (graph, target) in [
        ("blur", "minecraft:main"),
        ("creeper", "minecraft:main"),
        ("spider", "minecraft:main"),
        ("invert", "minecraft:main"),
        ("entity_outline", "minecraft:entity_outline"),
        ("transparency", "minecraft:main"),
    ] {
        let first_fragment = if overrides.iter().any(|(name, _)| *name == graph) {
            format!("minecraft:post/{graph}_override")
        } else {
            "minecraft:post/blit".to_string()
        };
        write_text(
            &asset(root, "minecraft", &format!("post_effect/{graph}.json")),
            &format!(
                r#"{{
                  "targets": {{"minecraft:swap": {{}}}},
                  "passes": [
                    {{
                      "vertex_shader": "minecraft:core/screenquad",
                      "fragment_shader": "{first_fragment}",
                      "inputs": [{{"sampler_name":"In","target":"{target}"}}],
                      "output": "minecraft:swap"
                    }},
                    {{
                      "vertex_shader": "minecraft:core/screenquad",
                      "fragment_shader": "minecraft:post/blit",
                      "inputs": [{{"sampler_name":"In","target":"minecraft:swap"}}],
                      "output": "{target}"
                    }}
                  ]
                }}"#
            ),
        );
    }
    write_text(
        &asset(root, "minecraft", "shaders/core/screenquad.vsh"),
        r#"#version 330
out vec2 texCoord;
void main() {
    vec2 uv = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    gl_Position = vec4(uv * vec2(2, 2) + vec2(-1, -1), 0, 1);
    texCoord = uv;
}
"#,
    );
    write_text(
        &asset(root, "minecraft", "shaders/post/blit.fsh"),
        r#"#version 330
uniform sampler2D InSampler;
in vec2 texCoord;
out vec4 fragColor;
void main() {
    fragColor = texture(InSampler, texCoord);
}
"#,
    );
    for (graph, color) in overrides {
        write_text(
            &asset(
                root,
                "minecraft",
                &format!("shaders/post/{graph}_override.fsh"),
            ),
            &format!(
                r#"#version 330
uniform sampler2D InSampler;
in vec2 texCoord;
out vec4 fragColor;
void main() {{
    fragColor = texture(InSampler, texCoord) * 0.0
        + vec4({}, {}, {}, {});
}}
"#,
                color[0], color[1], color[2], color[3]
            ),
        );
    }
}

fn fixture_roots(root: &Path) -> PackRoots {
    let vanilla = root.join("empty-vanilla");
    PackRoots::from_paths(
        root.to_path_buf(),
        &vanilla,
        vanilla.join("assets/minecraft"),
    )
    .with_resource_pack_dirs([root.to_path_buf()])
}

fn asset(root: &Path, namespace: &str, path: &str) -> PathBuf {
    root.join("assets").join(namespace).join(path)
}

fn write_text(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn write_png(path: &Path, color: [u8; 4]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba(color))
        .save(path)
        .unwrap();
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-client-renderer-{label}-{nanos}"))
}

#[test]
fn post_effect_graph_caches_pass_bind_groups_across_frames_and_invalidates_on_resize() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let root = unique_temp_dir("post-effect-bind-group-cache");
    write_two_pass_fixture(&root);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    let mut generation = PostEffectGraphGeneration::new(
        &device,
        &queue,
        TEST_FORMAT,
        4,
        4,
        &catalog.renderer_projection(),
    )
    .unwrap();

    let main_target = create_test_target(&device, "bbb-post-effect-cache-main");
    let main_view = main_target.create_view(&wgpu::TextureViewDescriptor::default());
    let run_once = |gen: &PostEffectGraphGeneration, view: &wgpu::TextureView, size: [u32; 2]| {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bbb-post-effect-cache-test"),
        });
        let external = [(
            "minecraft:main",
            PostEffectExternalTarget {
                color: view,
                depth: None,
                size,
            },
        )];
        let passes = gen
            .execute_with_targets(
                &device,
                &queue,
                &mut encoder,
                "demo:custom",
                &external,
                PostEffectGlobals::default(),
            )
            .unwrap();
        assert_eq!(passes, 2);
        queue.submit(Some(encoder.finish()));
    };

    run_once(&generation, &main_view, [4, 4]);
    let pass0_first = generation
        .cached_pass_bind_group("demo:custom", 0)
        .expect("pass 0 bind group cached");
    let pass1_first = generation
        .cached_pass_bind_group("demo:custom", 1)
        .expect("pass 1 bind group cached");

    run_once(&generation, &main_view, [4, 4]);
    assert_eq!(
        generation.cached_pass_bind_group("demo:custom", 0),
        Some(pass0_first.clone()),
        "steady-state frame should reuse cached pass 0 bind group"
    );
    assert_eq!(
        generation.cached_pass_bind_group("demo:custom", 1),
        Some(pass1_first.clone()),
        "steady-state frame should reuse cached pass 1 bind group"
    );

    let resize = generation.prepare_resize(&device, 8, 8).unwrap().unwrap();
    generation.apply_resize(resize);
    assert_eq!(
        generation.cached_pass_bind_group("demo:custom", 0),
        None,
        "resize must invalidate cached pass bind groups"
    );
    assert_eq!(
        generation.cached_pass_bind_group("demo:custom", 1),
        None,
        "resize must invalidate cached pass bind groups"
    );

    let resized_target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-post-effect-cache-main-resized"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TEST_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let resized_view = resized_target.create_view(&wgpu::TextureViewDescriptor::default());
    run_once(&generation, &resized_view, [8, 8]);
    let pass0_resized = generation
        .cached_pass_bind_group("demo:custom", 0)
        .expect("pass 0 bind group recreated after resize");
    let pass1_resized = generation
        .cached_pass_bind_group("demo:custom", 1)
        .expect("pass 1 bind group recreated after resize");
    assert_ne!(pass0_resized, pass0_first);
    assert_ne!(pass1_resized, pass1_first);

    let _ = std::fs::remove_dir_all(&root);
}
