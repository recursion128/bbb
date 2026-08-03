use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_pack::{PackRoots, PostEffectCatalog};
use winit::dpi::PhysicalSize;

use super::*;
use crate::item_models::{
    ItemModelAssetIdentity, ItemModelAssetRequest, ItemModelAssetSource, ItemModelAtlasKind,
    ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass,
    ItemModelInstanceRequest, ItemModelQuad, ItemModelResidencyFrame,
};
use crate::{
    cow_entity_texture_refs, ClearColor, EntityModelInstance, EntityModelTextureImage,
    HudInventoryScreen, Renderer,
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
fn production_post_effect_dispatch_has_one_id_owner_and_no_legacy_resolver() {
    let owner = include_str!("../post_effect_graph.rs");
    for id in [
        "minecraft:blur",
        "minecraft:creeper",
        "minecraft:spider",
        "minecraft:invert",
        "minecraft:entity_outline",
        "minecraft:transparency",
    ] {
        assert_eq!(
            owner.matches(id).count(),
            1,
            "{id} must have one canonical production declaration"
        );
    }

    let runtime = include_str!("runtime.rs");
    let render = include_str!("../render.rs");
    let camera_graph = include_str!("../camera_post_effect/graph.rs")
        .split_once("#[cfg(test)]")
        .expect("camera graph tests remain separate from production source")
        .0;
    for (path, source) in [
        ("post_effect_graph/runtime.rs", runtime),
        ("render.rs", render),
        ("camera_post_effect/graph.rs", camera_graph),
    ] {
        for legacy in [
            "REQUIRED_GRAPH_IDS",
            "contains_camera_graph",
            "enum CameraPostEffect",
            "CameraPostEffect::from_id",
        ] {
            assert!(
                !source.contains(legacy),
                "{path} restored legacy post-effect resolver {legacy}"
            );
        }
    }
    for raw_graph_id in [
        "\"minecraft:blur\"",
        "\"minecraft:creeper\"",
        "\"minecraft:spider\"",
        "\"minecraft:invert\"",
    ] {
        assert!(
            !runtime.contains(raw_graph_id)
                && !render.contains(raw_graph_id)
                && !camera_graph.contains(raw_graph_id),
            "production consumers restored raw graph dispatch for {raw_graph_id}"
        );
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
        wgpu::ImageDataLayout {
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
        wgpu::ImageDataLayout {
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
        assert_eq!(graph.targets["demo:full"].size, [4, 5]);
        assert_eq!(graph.targets["demo:fixed"].size, [2, 3]);
    }

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
    assert!(!generation.graphs["demo:targets"].targets["demo:full"]
        .needs_clear
        .get());
    assert!(!generation.graphs["demo:targets"].targets["demo:fixed"]
        .needs_clear
        .get());

    let resize = generation
        .prepare_resize(&device, 7, 8)
        .unwrap()
        .expect("size change prepares graph target replacements");
    generation.apply_resize(resize);
    let graph = &generation.graphs["demo:targets"];
    assert_eq!(graph.targets["demo:full"].size, [7, 8]);
    assert!(graph.targets["demo:full"].needs_clear.get());
    assert_eq!(graph.targets["demo:fixed"].size, [2, 3]);
    assert!(!graph.targets["demo:fixed"].needs_clear.get());

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
    let generation = PostEffectGraphGeneration::new(
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
        generation.graphs["demo:context"].targets["demo:persistent"]
            .needs_clear
            .get(),
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
    let catalog = PostEffectCatalog::load(&PackRoots::discover().unwrap()).unwrap();
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
    renderer
        .render_offscreen_frame()
        .expect("official transparency graph executes with all six color/depth owners");
    renderer.set_camera_post_effect(Some("minecraft:invert"), true);
    renderer
        .render_offscreen_frame()
        .expect("official camera graph executes after transparency");
    assert_eq!(renderer.counters.post_effect_draw_calls, 2);
}

#[test]
fn selected_generation_drives_live_camera_graph_instead_of_fixed_shader() {
    let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
        return;
    };
    let root = unique_temp_dir("post-effect-live-camera");
    write_live_catalog(&root, [1.0, 0.0, 0.0, 1.0]);
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
    renderer.set_camera_post_effect(Some("minecraft:invert"), true);

    let pixels = renderer.render_offscreen_frame().unwrap();
    let corner = pixels.pixel(2, 2);
    assert!(
        corner[0] > 240 && corner[1] < 8 && corner[2] < 8,
        "selected red override must replace the embedded invert shader, got {corner:?}"
    );
    assert_eq!(renderer.counters.post_effect_draw_calls, 2);
    renderer.resize(PhysicalSize::new(31, 23)).unwrap();
    assert_eq!(renderer.post_effect_graph_generation(), Some(0));
    let resized = renderer.render_offscreen_frame().unwrap();
    let resized_corner = resized.pixel(2, 2);
    assert!(
        resized_corner[0] > 240 && resized_corner[1] < 8 && resized_corner[2] < 8,
        "selected generation must retain its graph and shader after resize, got {resized_corner:?}"
    );
    assert_eq!(renderer.counters.post_effect_draw_calls, 2);
    renderer.set_camera_post_effect(Some("minecraft:entity_outline"), true);
    let unrelated = renderer.render_offscreen_frame().unwrap();
    let unrelated_corner = unrelated.pixel(2, 2);
    assert!(
        unrelated_corner[0] < 8 && unrelated_corner[1] < 8 && unrelated_corner[2] > 240,
        "non-camera graph ids must not escape the typed camera owner, got {unrelated_corner:?}"
    );
    assert_eq!(renderer.counters.post_effect_draw_calls, 0);
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
    renderer.set_hud_inventory_screen(Some(HudInventoryScreen {
        width: 24,
        height: 20,
        background_layers: Vec::new(),
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
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
        "selected blur override must replace the embedded six-pass blur, got {corner:?}"
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
    let cow_textures = cow_entity_texture_refs()
        .iter()
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
        main_pixels.rgba.chunks_exact(4).any(|pixel| {
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
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("bbb-post-effect-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    ))
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
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
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
    let _ = device.poll(wgpu::Maintain::Wait);
    receiver.recv().unwrap().unwrap();
    let mapped = slice.get_mapped_range();
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
    std::env::temp_dir().join(format!("bbb-renderer-{label}-{nanos}"))
}
