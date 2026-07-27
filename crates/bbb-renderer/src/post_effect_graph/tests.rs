use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_pack::{PackRoots, PostEffectCatalog};

use super::*;

const TEST_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn post_effect_graph_executes_selected_custom_shader_pixels_in_pass_order() {
    let Some((device, queue)) = request_test_device() else {
        return;
    };
    let root = unique_temp_dir("post-effect-graph-pixels");
    write_two_pass_fixture(&root);
    let catalog = PostEffectCatalog::load(&fixture_roots(&root)).unwrap();
    let mut generation =
        PostEffectGraphGeneration::new(&device, &queue, TEST_FORMAT, 4, 4, &catalog).unwrap();
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
        (20..=22).contains(&pixel[0])
            && (79..=81).contains(&pixel[1])
            && (39..=41).contains(&pixel[2])
            && pixel[3] >= 250,
        "custom target tint plus sRGB texture should produce [21,80,40,255], got {pixel:?}"
    );

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
    let mut generation =
        PostEffectGraphGeneration::new(&device, &queue, TEST_FORMAT, 4, 5, &catalog).unwrap();
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
    assert!(!generation.graphs["demo:targets"].targets["demo:full"].needs_clear);
    assert!(!generation.graphs["demo:targets"].targets["demo:fixed"].needs_clear);

    generation.resize(&device, 7, 8).unwrap();
    let graph = &generation.graphs["demo:targets"];
    assert_eq!(graph.targets["demo:full"].size, [7, 8]);
    assert!(graph.targets["demo:full"].needs_clear);
    assert_eq!(graph.targets["demo:fixed"].size, [2, 3]);
    assert!(!graph.targets["demo:fixed"].needs_clear);

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
        PostEffectGraphGeneration::new(&device, &queue, TEST_FORMAT, 4, 4, &catalog)
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
    let generation =
        PostEffectGraphGeneration::new(&device, &queue, TEST_FORMAT, 64, 64, &catalog).unwrap();
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
    let pixel = mapped[..4].try_into().unwrap();
    drop(mapped);
    readback.unmap();
    pixel
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

fn fixture_roots(root: &Path) -> PackRoots {
    PackRoots::from_paths(
        root.to_path_buf(),
        root.to_path_buf(),
        root.join("assets/minecraft"),
        None,
    )
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
