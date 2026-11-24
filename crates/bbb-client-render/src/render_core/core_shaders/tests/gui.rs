use super::*;

const VERTEX: &str = r#"#version 330
layout(std140) uniform DynamicTransforms {
    mat4 ModelViewMat;
    vec4 ColorModulator;
    vec3 ModelOffset;
    mat4 TextureMat;
};
layout(std140) uniform Projection {
    mat4 ProjMat;
};
in vec3 Position;
in vec4 Color;
out vec4 vertexColor;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    vertexColor = Color;
}
"#;
const FRAGMENT: &str = r#"#version 330
layout(std140) uniform DynamicTransforms {
    mat4 ModelViewMat;
    vec4 ColorModulator;
    vec3 ModelOffset;
    mat4 TextureMat;
};
in vec4 vertexColor;
out vec4 fragColor;
void main() {
    vec4 color = vertexColor;
    if (color.a == 0.0) {
        discard;
    }
    fragColor = color * ColorModulator;
}
"#;

fn sources(
    vertex: Option<(&str, &str)>,
    fragment: Option<(&str, &str)>,
) -> CoreShaderSourceProjection {
    let sources = [
        (ShaderStage::Vertex, vertex),
        (ShaderStage::Fragment, fragment),
    ]
    .into_iter()
    .filter_map(|(stage, source)| {
        let (pack_id, source) = source?;
        let key = ShaderSourceKey {
            id: "minecraft:core/gui".into(),
            stage,
        };
        Some((
            key.clone(),
            ShaderSource {
                key,
                resource_id: format!(
                    "minecraft:shaders/core/gui.{}",
                    if stage == ShaderStage::Vertex {
                        "vsh"
                    } else {
                        "fsh"
                    }
                ),
                pack_id: pack_id.into(),
                source: source.into(),
                imports: Vec::new(),
            },
        ))
    })
    .collect();
    CoreShaderSourceProjection::new(1, Arc::new(sources))
}

fn validate(source: &str, stage: wgpu::naga::ShaderStage) -> wgpu::naga::Module {
    let mut frontend = wgpu::naga::front::glsl::Frontend::default();
    let module = frontend
        .parse(&stage.into(), source)
        .unwrap_or_else(|errors| panic!("{}", errors.emit_to_string(source)));
    wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("valid selected GUI shader");
    let bindings = module
        .global_variables
        .iter()
        .filter_map(|(_, variable)| variable.binding.as_ref().map(|binding| (variable, binding)))
        .collect::<Vec<_>>();
    assert_eq!(bindings.len(), 1, "GUI has only its size uniform");
    let (variable, binding) = bindings[0];
    assert_eq!((binding.group, binding.binding), (0, 0));
    assert_eq!(variable.space, wgpu::naga::AddressSpace::Uniform);
    let wgpu::naga::TypeInner::Struct { ref members, span } = module.types[variable.ty].inner
    else {
        panic!("GUI parameters must be a uniform block");
    };
    assert_eq!(span, 16);
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].name.as_deref(), Some("BbbGuiSize"));
    assert_eq!(members[0].offset, 0);
    assert_eq!(module.entry_points[0].name, "main");
    module
}

#[test]
fn gui_vanilla_pair_keeps_builtin_pipeline() {
    assert!(adapt_selected_gui_shaders(&sources(
        Some(("vanilla", VERTEX)),
        Some(("vanilla", FRAGMENT))
    ))
    .unwrap()
    .is_none());
}

#[test]
fn gui_selected_vertex_preserves_arithmetic_and_adapts_default_fragment() {
    let vertex = VERTEX
        .replace(
            "vec4(Position, 1.0)",
            "vec4(Position + ModelOffset + vec3(0.25, -0.5, 0.0), 1.0)",
        )
        .replace(
            "vertexColor = Color;",
            "vertexColor = TextureMat * Color * vec4(0.25, 0.5, 1.0, 0.75);",
        );
    let adapted = adapt_selected_gui_shaders(&sources(
        Some(("fixture", &vertex)),
        Some(("vanilla", FRAGMENT)),
    ))
    .unwrap()
    .unwrap();
    let module = validate(&adapted.vertex, wgpu::naga::ShaderStage::Vertex);
    validate(&adapted.fragment, wgpu::naga::ShaderStage::Fragment);
    let inputs = &module.entry_points[0].function.arguments;
    for (location, expected_size) in [
        (0, wgpu::naga::VectorSize::Bi),
        (2, wgpu::naga::VectorSize::Quad),
    ] {
        let argument = inputs.iter().find(|argument| matches!(argument.binding, Some(wgpu::naga::Binding::Location { location: value, .. }) if value == location)).expect("HudVertex input location");
        assert!(
            matches!(module.types[argument.ty].inner, wgpu::naga::TypeInner::Vector { size, .. } if size == expected_size)
        );
    }
    assert!(adapted
        .vertex
        .contains("Position + ModelOffset + vec3(0.25, -0.5, 0.0)"));
    assert!(adapted
        .vertex
        .contains("TextureMat * Color * vec4(0.25, 0.5, 1.0, 0.75)"));
    assert!(!adapted.vertex.contains("gl_Position.y = -"));
    assert!(!adapted.vertex.contains("BbbCamera"));
    assert!(adapted.fragment.contains("if (color.a == 0.0)"));
    assert!(adapted.fragment.contains("discard;"));
}

#[test]
fn gui_selected_fragment_preserves_override_alpha_and_default_vertex() {
    let fragment = FRAGMENT.replace(
        "color * ColorModulator",
        "vec4(1.0, 0.0, 0.5, color.a * 0.25) * ColorModulator",
    );
    let adapted = adapt_selected_gui_shaders(&sources(
        Some(("vanilla", VERTEX)),
        Some(("fixture", &fragment)),
    ))
    .unwrap()
    .unwrap();
    validate(&adapted.vertex, wgpu::naga::ShaderStage::Vertex);
    validate(&adapted.fragment, wgpu::naga::ShaderStage::Fragment);
    assert!(adapted
        .fragment
        .contains("vec4(1.0, 0.0, 0.5, color.a * 0.25)"));
    assert!(adapted.fragment.contains("discard;"));
    assert!(
        adapted
            .vertex
            .contains("#define Position vec3((BbbGuiPosition * vec2(1.0, -1.0) + vec2(1.0)) * BbbGuiSize.xy * 0.5, 0.0)")
    );
    assert!(adapted
        .vertex
        .contains("layout(location = 0) out vec4 vertexColor;"));
    assert!(adapted
        .fragment
        .contains("layout(location = 0) in vec4 vertexColor;"));
}

#[test]
fn gui_missing_stage_fails_with_resource_and_stage_context() {
    for (vertex, fragment, missing) in [
        (None, Some(("fixture", FRAGMENT)), "vertex"),
        (Some(("fixture", VERTEX)), None, "fragment"),
    ] {
        let error = adapt_selected_gui_shaders(&sources(vertex, fragment))
            .err()
            .expect("missing stage must fail")
            .to_string();
        assert!(error.contains("minecraft:core/gui"), "{error}");
        assert!(error.contains(missing), "{error}");
    }
}

#[test]
fn gui_missing_color_input_fails_in_selected_vertex_context() {
    let vertex = VERTEX.replace("in vec4 Color;", "");
    let error = adapt_selected_gui_shaders(&sources(
        Some(("fixture", &vertex)),
        Some(("vanilla", FRAGMENT)),
    ))
    .err()
    .expect("missing input must fail");
    let chain = format!("{error:#}");
    assert!(
        chain.contains("adapt selected minecraft:core/gui vertex shader"),
        "{chain}"
    );
    assert!(chain.contains("Color"), "{chain}");
}
