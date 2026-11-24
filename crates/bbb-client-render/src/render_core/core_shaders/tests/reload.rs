use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::bail;

use super::*;

fn resident_test_item_frame(
    quad: crate::ItemModelQuad,
    foil: bool,
) -> crate::ItemModelResidencyFrame {
    use crate::{
        ItemModelAssetIdentity, ItemModelAssetRequest, ItemModelAssetSource, ItemModelAtlasKind,
        ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass,
        ItemModelInstanceRequest,
    };
    let identity = ItemModelAssetIdentity::new(
        std::sync::Arc::<str>::from("bbb:test_item"),
        std::sync::Arc::<str>::from("plain"),
        0,
    );
    let foil = ItemModelFoil::from_has_foil(foil);
    let phases: &[ItemModelDrawPhase] = if foil.has_foil() {
        &[ItemModelDrawPhase::Solid, ItemModelDrawPhase::Glint]
    } else {
        &[ItemModelDrawPhase::Solid]
    };
    crate::ItemModelResidencyFrame {
        assets: vec![ItemModelAssetRequest {
            identity: identity.clone(),
            atlas: ItemModelAtlasKind::Blocks,
            geometry_class: ItemModelGeometryClass::Solid,
            source: ItemModelAssetSource::Quads(std::sync::Arc::from([quad])),
        }],
        instances: phases
            .iter()
            .enumerate()
            .map(|(order, phase)| ItemModelInstanceRequest {
                identity: identity.clone(),
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                transform: glam::Mat4::from_translation(glam::Vec3::new(-0.5, -0.5, 4.0)),
                tint: [1.0; 4],
                light: [1.0; 2],
                overlay: [0.0, 10.0],
                foil,
                target: ItemModelDrawTarget::World,
                phase: *phase,
                submit_order: order as u32,
                scissor: None,
            })
            .collect(),
        special_items: Vec::new(),
    }
}

#[test]
fn terrain_shader_selection_keeps_vanilla_embedded_and_adapts_overrides() {
    let vanilla = test_projection(30, "vec4(1.0)", "vanilla");
    assert!(has_terrain_shader_sources(&vanilla));
    assert!(adapt_selected_terrain_shaders(&vanilla, true)
        .expect("classify vanilla terrain shaders")
        .is_none());

    let selected = test_projection(31, "vec4(1.0)", "fixture");
    assert!(adapt_selected_terrain_shaders(&selected, true)
        .expect("adapt selected terrain shaders")
        .is_some());
}

#[test]
fn selected_core_shader_drives_terrain_pixels_and_failed_generation_rolls_back() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .expect("white terrain atlas");
    let vertex = |position| {
        crate::gpu::TerrainGpuVertex::pack(
            position,
            [0.5, 0.5],
            0x00F0_00F0,
            crate::terrain::TerrainTint::WHITE,
            1.0,
            1.0,
        )
    };
    renderer.show_test_terrain_mesh(crate::terrain::TerrainMesh {
        vertices: vec![
            vertex([-0.75, -0.75, 0.0]),
            vertex([0.75, -0.75, 0.0]),
            vertex([0.75, 0.75, 0.0]),
            vertex([-0.75, 0.75, 0.0]),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        source_sections: 1,
        opaque_faces: 1,
        ..crate::terrain::TerrainMesh::default()
    });

    let red = test_projection(31, "vec4(1.0, 0.0, 0.0, 1.0)", "fixture");
    renderer
        .transactional_resource_reload(|renderer| renderer.reload_core_shaders(&red, false))
        .expect("install red selected shader");
    assert_eq!(renderer.core_shader_generation(), Some(31));
    assert_red(renderer.render_offscreen_frame().unwrap().pixel(16, 16));

    let green = test_projection(32, "vec4(0.0, 1.0, 0.0, 1.0)", "fixture");
    renderer
        .transactional_resource_reload(|renderer| -> Result<()> {
            renderer.reload_core_shaders(&green, false)?;
            bail!("injected failure after core pipeline replacement")
        })
        .expect_err("later generation stage should fail");
    assert_eq!(renderer.core_shader_generation(), Some(31));
    assert_red(renderer.render_offscreen_frame().unwrap().pixel(16, 16));

    renderer
        .transactional_resource_reload(|renderer| renderer.reload_core_shaders(&green, false))
        .expect("install green selected shader");
    assert_eq!(renderer.core_shader_generation(), Some(32));
    let pixel = renderer.render_offscreen_frame().unwrap().pixel(16, 16);
    assert!(
        pixel[1] > 200 && pixel[0] < 20 && pixel[2] < 20,
        "selected green shader must own current terrain pixels, got {pixel:?}"
    );
}

fn test_projection(
    generation: u64,
    color: &str,
    terrain_pack_id: &str,
) -> CoreShaderSourceProjection {
    let vertex_key = ShaderSourceKey {
        id: TERRAIN_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let fragment_key = ShaderSourceKey {
        id: TERRAIN_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let entity_vertex_key = ShaderSourceKey {
        id: ENTITY_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let entity_fragment_key = ShaderSourceKey {
        id: ENTITY_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let item_vertex_key = ShaderSourceKey {
        id: ITEM_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let item_fragment_key = ShaderSourceKey {
        id: ITEM_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let particle_vertex_key = ShaderSourceKey {
        id: PARTICLE_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let particle_fragment_key = ShaderSourceKey {
        id: PARTICLE_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let position_color_vertex_key = ShaderSourceKey {
        id: POSITION_COLOR_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let position_color_fragment_key = ShaderSourceKey {
        id: POSITION_COLOR_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let glint_vertex_key = ShaderSourceKey {
        id: GLINT_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let glint_fragment_key = ShaderSourceKey {
        id: GLINT_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let end_portal_vertex_key = ShaderSourceKey {
        id: END_PORTAL_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let end_portal_fragment_key = ShaderSourceKey {
        id: END_PORTAL_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let outline_vertex_key = ShaderSourceKey {
        id: OUTLINE_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let outline_fragment_key = ShaderSourceKey {
        id: OUTLINE_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let beacon_beam_vertex_key = ShaderSourceKey {
        id: BEACON_BEAM_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let beacon_beam_fragment_key = ShaderSourceKey {
        id: BEACON_BEAM_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let lightning_vertex_key = ShaderSourceKey {
        id: LIGHTNING_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let lightning_fragment_key = ShaderSourceKey {
        id: LIGHTNING_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let lines_vertex_key = ShaderSourceKey {
        id: LINES_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let lines_fragment_key = ShaderSourceKey {
        id: LINES_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let text_vertex_key = ShaderSourceKey {
        id: TEXT_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let text_fragment_key = ShaderSourceKey {
        id: TEXT_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let text_see_through_vertex_key = ShaderSourceKey {
        id: TEXT_SEE_THROUGH_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let text_see_through_fragment_key = ShaderSourceKey {
        id: TEXT_SEE_THROUGH_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let text_background_vertex_key = ShaderSourceKey {
        id: TEXT_BACKGROUND_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let text_background_fragment_key = ShaderSourceKey {
        id: TEXT_BACKGROUND_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let cloud_vertex_key = ShaderSourceKey {
        id: CLOUD_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let cloud_fragment_key = ShaderSourceKey {
        id: CLOUD_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let screen_quad_vertex_key = ShaderSourceKey {
        id: SCREEN_QUAD_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let lightmap_fragment_key = ShaderSourceKey {
        id: LIGHTMAP_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let sky_vertex_key = ShaderSourceKey {
        id: SKY_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let sky_fragment_key = ShaderSourceKey {
        id: SKY_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let stars_vertex_key = ShaderSourceKey {
        id: STARS_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let stars_fragment_key = ShaderSourceKey {
        id: STARS_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let end_sky_vertex_key = ShaderSourceKey {
        id: POSITION_TEX_COLOR_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let end_sky_fragment_key = ShaderSourceKey {
        id: POSITION_TEX_COLOR_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let celestial_vertex_key = ShaderSourceKey {
        id: POSITION_TEX_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let celestial_fragment_key = ShaderSourceKey {
        id: POSITION_TEX_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let entity_special_keys = [
        POSITION_SHADER_ID,
        LEASH_SHADER_ID,
        WATER_MASK_SHADER_ID,
        ENTITY_SHADOW_SHADER_ID,
    ]
    .into_iter()
    .flat_map(|id| {
        [ShaderStage::Vertex, ShaderStage::Fragment].map(|stage| ShaderSourceKey {
            id: id.to_string(),
            stage,
        })
    })
    .collect::<Vec<_>>();
    let static_core_keys = [
        (CRUMBLING_SHADER_ID, ShaderStage::Vertex),
        (CRUMBLING_SHADER_ID, ShaderStage::Fragment),
        (WORLD_BORDER_SHADER_ID, ShaderStage::Vertex),
        (WORLD_BORDER_SHADER_ID, ShaderStage::Fragment),
        (BLOCK_SHADER_ID, ShaderStage::Vertex),
        (BLOCK_SHADER_ID, ShaderStage::Fragment),
        (BLIT_SCREEN_SHADER_ID, ShaderStage::Fragment),
    ]
    .into_iter()
    .map(|(id, stage)| ShaderSourceKey {
        id: id.to_string(),
        stage,
    })
    .collect::<Vec<_>>();
    let mut sources = BTreeMap::from([
            (
                vertex_key.clone(),
                ShaderSource {
                    key: vertex_key,
                    resource_id: "minecraft:shaders/core/terrain.vsh".to_string(),
                    pack_id: terrain_pack_id.to_string(),
                    source: r#"#version 330
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
void main() { gl_Position = vec4(Position, 1.0); }
"#
                    .to_string(),
                    imports: Vec::new(),
                },
            ),
            (
                fragment_key.clone(),
                ShaderSource {
                    key: fragment_key,
                    resource_id: "minecraft:shaders/core/terrain.fsh".to_string(),
                    pack_id: terrain_pack_id.to_string(),
                    source: format!(
                        "#version 330\nout vec4 fragColor;\nvoid main() {{ fragColor = {color} * vec4(vec3(ChunkVisibility), 1.0); }}\n"
                    ),
                    imports: Vec::new(),
                },
            ),
            (
                entity_vertex_key.clone(),
                ShaderSource {
                    key: entity_vertex_key,
                    resource_id: "minecraft:shaders/core/entity.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                entity_fragment_key.clone(),
                ShaderSource {
                    key: entity_fragment_key,
                    resource_id: "minecraft:shaders/core/entity.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                item_vertex_key.clone(),
                ShaderSource {
                    key: item_vertex_key,
                    resource_id: "minecraft:shaders/core/item.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                item_fragment_key.clone(),
                ShaderSource {
                    key: item_fragment_key,
                    resource_id: "minecraft:shaders/core/item.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                particle_vertex_key.clone(),
                ShaderSource {
                    key: particle_vertex_key,
                    resource_id: "minecraft:shaders/core/particle.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                particle_fragment_key.clone(),
                ShaderSource {
                    key: particle_fragment_key,
                    resource_id: "minecraft:shaders/core/particle.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                position_color_vertex_key.clone(),
                ShaderSource {
                    key: position_color_vertex_key,
                    resource_id: "minecraft:shaders/core/position_color.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                position_color_fragment_key.clone(),
                ShaderSource {
                    key: position_color_fragment_key,
                    resource_id: "minecraft:shaders/core/position_color.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                glint_vertex_key.clone(),
                ShaderSource {
                    key: glint_vertex_key,
                    resource_id: "minecraft:shaders/core/glint.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                glint_fragment_key.clone(),
                ShaderSource {
                    key: glint_fragment_key,
                    resource_id: "minecraft:shaders/core/glint.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                end_portal_vertex_key.clone(),
                ShaderSource {
                    key: end_portal_vertex_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_end_portal.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                end_portal_fragment_key.clone(),
                ShaderSource {
                    key: end_portal_fragment_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_end_portal.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                outline_vertex_key.clone(),
                ShaderSource {
                    key: outline_vertex_key,
                    resource_id: "minecraft:shaders/core/rendertype_outline.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                outline_fragment_key.clone(),
                ShaderSource {
                    key: outline_fragment_key,
                    resource_id: "minecraft:shaders/core/rendertype_outline.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                beacon_beam_vertex_key.clone(),
                ShaderSource {
                    key: beacon_beam_vertex_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_beacon_beam.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                beacon_beam_fragment_key.clone(),
                ShaderSource {
                    key: beacon_beam_fragment_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_beacon_beam.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                lightning_vertex_key.clone(),
                ShaderSource {
                    key: lightning_vertex_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_lightning.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                lightning_fragment_key.clone(),
                ShaderSource {
                    key: lightning_fragment_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_lightning.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                lines_vertex_key.clone(),
                ShaderSource {
                    key: lines_vertex_key,
                    resource_id: "minecraft:shaders/core/rendertype_lines.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                lines_fragment_key.clone(),
                ShaderSource {
                    key: lines_fragment_key,
                    resource_id: "minecraft:shaders/core/rendertype_lines.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                text_vertex_key.clone(),
                ShaderSource {
                    key: text_vertex_key,
                    resource_id: "minecraft:shaders/core/rendertype_text.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                text_fragment_key.clone(),
                ShaderSource {
                    key: text_fragment_key,
                    resource_id: "minecraft:shaders/core/rendertype_text.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                text_see_through_vertex_key.clone(),
                ShaderSource {
                    key: text_see_through_vertex_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_text_see_through.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                text_see_through_fragment_key.clone(),
                ShaderSource {
                    key: text_see_through_fragment_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_text_see_through.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                text_background_vertex_key.clone(),
                ShaderSource {
                    key: text_background_vertex_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_text_background.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                text_background_fragment_key.clone(),
                ShaderSource {
                    key: text_background_fragment_key,
                    resource_id:
                        "minecraft:shaders/core/rendertype_text_background.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                cloud_vertex_key.clone(),
                ShaderSource {
                    key: cloud_vertex_key,
                    resource_id: "minecraft:shaders/core/rendertype_clouds.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                cloud_fragment_key.clone(),
                ShaderSource {
                    key: cloud_fragment_key,
                    resource_id: "minecraft:shaders/core/rendertype_clouds.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                screen_quad_vertex_key.clone(),
                ShaderSource {
                    key: screen_quad_vertex_key,
                    resource_id: "minecraft:shaders/core/screenquad.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                lightmap_fragment_key.clone(),
                ShaderSource {
                    key: lightmap_fragment_key,
                    resource_id: "minecraft:shaders/core/lightmap.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                sky_vertex_key.clone(),
                ShaderSource {
                    key: sky_vertex_key,
                    resource_id: "minecraft:shaders/core/sky.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                sky_fragment_key.clone(),
                ShaderSource {
                    key: sky_fragment_key,
                    resource_id: "minecraft:shaders/core/sky.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                stars_vertex_key.clone(),
                ShaderSource {
                    key: stars_vertex_key,
                    resource_id: "minecraft:shaders/core/stars.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                stars_fragment_key.clone(),
                ShaderSource {
                    key: stars_fragment_key,
                    resource_id: "minecraft:shaders/core/stars.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                end_sky_vertex_key.clone(),
                ShaderSource {
                    key: end_sky_vertex_key,
                    resource_id: "minecraft:shaders/core/position_tex_color.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                end_sky_fragment_key.clone(),
                ShaderSource {
                    key: end_sky_fragment_key,
                    resource_id: "minecraft:shaders/core/position_tex_color.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                celestial_vertex_key.clone(),
                ShaderSource {
                    key: celestial_vertex_key,
                    resource_id: "minecraft:shaders/core/position_tex.vsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
            (
                celestial_fragment_key.clone(),
                ShaderSource {
                    key: celestial_fragment_key,
                    resource_id: "minecraft:shaders/core/position_tex.fsh".to_string(),
                    pack_id: "vanilla".to_string(),
                    source: String::new(),
                    imports: Vec::new(),
                },
            ),
    ]);
    for key in entity_special_keys {
        let extension = match key.stage {
            ShaderStage::Vertex => "vsh",
            ShaderStage::Fragment => "fsh",
        };
        let shader_path = key
            .id
            .strip_prefix("minecraft:core/")
            .expect("fixture shader namespace")
            .to_string();
        sources.insert(
            key.clone(),
            ShaderSource {
                key,
                resource_id: format!("minecraft:shaders/core/{shader_path}.{extension}"),
                pack_id: "vanilla".to_string(),
                source: String::new(),
                imports: Vec::new(),
            },
        );
    }
    for key in static_core_keys
        .into_iter()
        .chain(
            [ShaderStage::Vertex, ShaderStage::Fragment].map(|stage| ShaderSourceKey {
                id: "minecraft:core/gui".into(),
                stage,
            }),
        )
    {
        let extension = match key.stage {
            ShaderStage::Vertex => "vsh",
            ShaderStage::Fragment => "fsh",
        };
        let shader_path = key
            .id
            .strip_prefix("minecraft:core/")
            .expect("fixture shader namespace")
            .to_string();
        sources.insert(
            key.clone(),
            ShaderSource {
                key,
                resource_id: format!("minecraft:shaders/core/{shader_path}.{extension}"),
                pack_id: "vanilla".to_string(),
                source: String::new(),
                imports: Vec::new(),
            },
        );
    }
    CoreShaderSourceProjection::new(generation, Arc::new(sources))
}

fn assert_red(pixel: [u8; 4]) {
    assert!(
        pixel[0] > 200 && pixel[1] < 20 && pixel[2] < 20,
        "selected red shader must own current terrain pixels, got {pixel:?}"
    );
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn compiles_local_vanilla_terrain_pipeline_generation() {
    let catalog = bbb_client_assets::ShaderSourceCatalog::load(
        &bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots"),
    )
    .expect("local shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("compile local terrain pipelines");
    assert_eq!(
        renderer.core_shader_generation(),
        Some(catalog.generation())
    );
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_pack_override_and_filter_drive_or_reject_current_terrain_pipeline() {
    let root = unique_temp_dir("selected-core-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/terrain.fsh"),
        "#version 330\nin float sphericalVertexDistance;\nin float cylindricalVertexDistance;\nin vec4 vertexColor;\nin vec2 texCoord0;\nout vec4 fragColor;\nvoid main() { fragColor = vec4(1.0, 0.0, 0.0, 1.0); }\n",
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected override pack");
    let override_roots = base
        .clone()
        .with_client_resource_generation(51, selected)
        .expect("override generation");
    let override_catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&override_roots).expect("override catalog");
    assert_eq!(
        override_catalog
            .source_by_id(TERRAIN_SHADER_ID, ShaderStage::Fragment)
            .expect("selected fragment")
            .pack_id,
        "startup/0"
    );

    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&override_catalog.renderer_projection(), false)
        })
        .expect("selected override compiles");
    assert_eq!(renderer.core_shader_generation(), Some(51));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/terrain\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected filter pack");
    let filtered_roots = base
        .with_client_resource_generation(52, filtered)
        .expect("filtered generation");
    let filtered_catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots).expect("filtered catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering the required terrain stages must reject the generation");
    assert!(error.to_string().contains("missing minecraft:core/terrain"));
    assert_eq!(renderer.core_shader_generation(), Some(51));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_entity_shader_preloads_all_current_entity_surface_variants() {
    let root = unique_temp_dir("selected-entity-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    write_text(
        &overlay.join("assets/minecraft/shaders/core/entity.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
#ifdef PER_FACE_LIGHTING
in vec4 vertexPerFaceColorBack;
in vec4 vertexPerFaceColorFront;
#else
in vec4 vertexColor;
#endif
#ifndef EMISSIVE
in vec4 lightMapColor;
#endif
#ifndef NO_OVERLAY
in vec4 overlayColor;
#endif
in vec2 texCoord0;
#ifdef DISSOLVE
uniform sampler2D DissolveMaskSampler;
#endif
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + texCoord0.x;
#ifdef PER_FACE_LIGHTING
keep += vertexPerFaceColorBack.x + vertexPerFaceColorFront.x;
#else
keep += vertexColor.x;
#endif
#ifndef EMISSIVE
keep += lightMapColor.x;
#endif
#ifndef NO_OVERLAY
keep += overlayColor.x;
#endif
#ifdef DISSOLVE
keep += texture(DissolveMaskSampler, texCoord0).a;
#endif
fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
#ifndef NO_OVERLAY
fragColor.rgb = mix(overlayColor.rgb, fragColor.rgb, overlayColor.a);
#endif
}
"#,
    );
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected entity shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(61, selected)
        .expect("selected entity generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    assert_eq!(
        catalog
            .source_by_id(ENTITY_SHADER_ID, ShaderStage::Fragment)
            .expect("selected entity fragment")
            .pack_id,
        "startup/0"
    );

    let mut renderer = crate::Renderer::new_offscreen(24, 20)
        .expect("ignored shader pixel test requires a GPU adapter and device");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected entity shader variants compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(61));
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_pose(Some(crate::CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    }));
    let textures = crate::entity_model_texture_refs()
        .iter()
        .filter(|texture| texture.path.starts_with("textures/entity/cow/cow_"))
        .map(|texture| {
            crate::EntityModelTextureImage::new(
                *texture,
                [255, 255, 255, 255]
                    .repeat(usize::try_from(texture.size[0] * texture.size[1]).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    renderer
        .upload_entity_model_textures(&textures)
        .expect("cow textures");
    let cow = crate::EntityModelInstance::cow(1, [0.0, -0.75, 4.0], 0.0, false);
    renderer.set_entity_model_instances(vec![cow]);
    let pixels = renderer
        .render_offscreen_frame()
        .expect("selected entity frame");
    let max_green = pixels
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[1])
        .max()
        .unwrap_or_default();
    assert!(
        (0..20).any(|y| {
            (0..24).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
            })
        }),
        "selected green entity fragment must own at least one current entity pixel; max_green={max_green}, entity_draw_calls={}",
        renderer.counters().entity_scene_draw_calls
    );

    let (x, y) = (0..20)
        .flat_map(|y| (0..24).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let pixel = pixels.pixel(x, y);
            pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
        })
        .unwrap();
    for (instance, expected) in [
        (cow.with(|s| s.has_red_overlay = true), [77u8, 178, 0, 255]),
        (
            cow.with(|s| s.white_overlay_progress = 1.0),
            [192u8, 255, 192, 255],
        ),
    ] {
        renderer.set_entity_model_instances(vec![instance]);
        let overlay = renderer
            .render_offscreen_frame()
            .expect("selected overlay frame");
        let actual = overlay.pixel(x, y);
        assert!(
            actual.iter().zip(expected).all(|(actual, expected)| actual.abs_diff(expected) <= 1),
            "selected fragment must receive Vanilla overlay texels: actual={actual:?}, expected={expected:?}"
        );
    }
    renderer.set_entity_model_instances(vec![cow]);

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/entity\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected entity filter pack");
    let filtered_roots = base
        .clone()
        .with_client_resource_generation(62, filtered)
        .expect("filtered entity generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required entity stage must reject the generation");
    assert!(error.to_string().contains("minecraft:core/entity"));
    assert_eq!(renderer.core_shader_generation(), Some(61));
    let retained = renderer
        .render_offscreen_frame()
        .expect("retained selected entity frame");
    assert!(frame_has_green_entity_pixel(&retained));

    let default_stack = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [])
        .expect("default selected stack");
    let default_roots = base
        .with_client_resource_generation(63, default_stack)
        .expect("default entity generation");
    let default_catalog = bbb_client_assets::ShaderSourceCatalog::load(&default_roots)
        .expect("default shader catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
        })
        .expect("removing selected entity shader restores embedded generation");
    assert_eq!(renderer.core_shader_generation(), Some(63));
    let restored = renderer
        .render_offscreen_frame()
        .expect("restored embedded entity frame");
    assert!(!frame_has_green_entity_pixel(&restored));

    renderer.upload_entity_model_textures(&textures).unwrap();
    renderer.set_entity_model_instances(vec![cow]);
    let mut moving = cow;
    moving.position[0] += 0.1;
    renderer.set_entity_model_instances(vec![moving]);
    let embedded = renderer.render_offscreen_frame().unwrap();
    assert!(embedded
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|&pixel| pixel != [0, 0, 255, 255]));
    renderer
        .reload_core_shaders(&catalog.renderer_projection(), false)
        .unwrap();
    assert!(frame_has_green_entity_pixel(
        &renderer.render_offscreen_frame().unwrap()
    ));
    renderer
        .reload_core_shaders(&default_catalog.renderer_projection(), false)
        .unwrap();
    assert_eq!(
        renderer.render_offscreen_frame().unwrap().rgba,
        embedded.rgba
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_end_portal_shader_drives_hud_and_preloads_all_current_portal_surfaces() {
    let root = unique_temp_dir("selected-end-portal-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_end_portal.fsh"),
        r#"#version 330
in vec4 texProj0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
out vec4 fragColor;
void main() {
float keep = texProj0.x + sphericalVertexDistance + cylindricalVertexDistance;
fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected end-portal shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(71, selected)
        .expect("selected end-portal generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    assert_eq!(
        catalog
            .source_by_id(END_PORTAL_SHADER_ID, ShaderStage::Fragment)
            .expect("selected end-portal fragment")
            .pack_id,
        "startup/0"
    );

    let mut renderer = crate::Renderer::new_offscreen(32, 32)
        .expect("ignored shader integration test requires a GPU adapter and device");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected end-portal variants compile atomically");
    let sky = crate::EntityModelTextureRef {
        path: "textures/environment/end_sky.png",
        size: [128, 128],
    };
    let portal = crate::EntityModelTextureRef {
        path: "textures/entity/end_portal/end_portal.png",
        size: [256, 256],
    };
    renderer
        .upload_entity_model_textures(&[
            crate::EntityModelTextureImage::with_physical_size(
                sky,
                [1, 1],
                vec![255, 255, 255, 255],
            ),
            crate::EntityModelTextureImage::with_physical_size(
                portal,
                [1, 1],
                vec![255, 255, 255, 255],
            ),
        ])
        .expect("portal textures");
    renderer.set_hud_game_event_screen(Some(crate::HudGameEventScreen::Win(crate::HudWinScreen {
        generation: 71,
        scroll: 0.0,
        show_easter_egg_logo: false,
        lines: Arc::from([]),
    })));
    assert_eq!(renderer.core_shader_generation(), Some(71));
    let selected_frame = renderer
        .render_offscreen_frame()
        .expect("selected portal frame");
    assert!(frame_has_green_entity_pixel(&selected_frame));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_end_portal\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected end-portal filter pack");
    let filtered_roots = base
        .clone()
        .with_client_resource_generation(72, filtered)
        .expect("filtered end-portal generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required end-portal stage must reject the generation");
    assert!(error.to_string().contains("rendertype_end_portal"));
    assert_eq!(renderer.core_shader_generation(), Some(71));
    assert!(frame_has_green_entity_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("retained portal frame")
    ));

    let default_stack = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [])
        .expect("default selected stack");
    let default_roots = base
        .clone()
        .with_client_resource_generation(73, default_stack)
        .expect("default end-portal generation");
    let default_catalog = bbb_client_assets::ShaderSourceCatalog::load(&default_roots)
        .expect("default shader catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
        })
        .expect("removing selected end-portal shader restores embedded generation");
    assert_eq!(renderer.core_shader_generation(), Some(73));
    assert!(!frame_has_green_entity_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("restored portal frame")
    ));

    let official = root.join("official-fragment");
    write_pack_metadata(&official, None);
    let expanded_vertex = default_catalog
        .source_by_id(END_PORTAL_SHADER_ID, ShaderStage::Vertex)
        .expect("default expanded end-portal vertex")
        .source
        .clone();
    write_text(
        &official.join("assets/minecraft/shaders/core/rendertype_end_portal.vsh"),
        &expanded_vertex,
    );
    let official_selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [official])
            .expect("selected official vertex pack");
    let official_roots = base
        .with_client_resource_generation(74, official_selected)
        .expect("selected official end-portal generation");
    let official_catalog = bbb_client_assets::ShaderSourceCatalog::load(&official_roots)
        .expect("selected official-fragment catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&official_catalog.renderer_projection(), false)
        })
        .expect("official end-portal fragment compiles through the atlas projection");
    assert_eq!(renderer.core_shader_generation(), Some(74));
    renderer
        .render_offscreen_frame()
        .expect("official projected portal frame");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_outline_shader_preloads_all_current_surfaces_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-outline-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_outline.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
vec4 color = texture(Sampler0, texCoord0);
if (color.a == 0.0) { discard; }
fragColor = vec4(0.0, 1.0, 0.0, vertexColor.a);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected outline shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(81, selected)
        .expect("selected outline generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    assert_eq!(
        catalog
            .source_by_id(OUTLINE_SHADER_ID, ShaderStage::Fragment)
            .expect("selected outline fragment")
            .pack_id,
        "startup/0"
    );

    let mut renderer = crate::Renderer::new_offscreen(32, 32)
        .expect("ignored shader integration test requires a GPU adapter and device");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected entity/resident outline variants compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(81));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_outline\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected outline filter pack");
    let filtered_roots = base
        .clone()
        .with_client_resource_generation(82, filtered)
        .expect("filtered outline generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required outline stage must reject the generation");
    assert!(error.to_string().contains("rendertype_outline"));
    assert_eq!(renderer.core_shader_generation(), Some(81));

    let default_stack = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [])
        .expect("default selected stack");
    let default_roots = base
        .with_client_resource_generation(83, default_stack)
        .expect("default outline generation");
    let default_catalog = bbb_client_assets::ShaderSourceCatalog::load(&default_roots)
        .expect("default shader catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
        })
        .expect("removing selected outline shader restores embedded generation");
    assert_eq!(renderer.core_shader_generation(), Some(83));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_beacon_beam_shader_preloads_both_depth_variants_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-beacon-beam-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_beacon_beam.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
fragColor = vec4(0.0, 1.0, 0.0, vertexColor.a + texCoord0.x * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected beacon-beam shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(91, selected)
        .expect("selected beacon-beam generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected opaque/translucent beacon-beam variants compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(91));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_beacon_beam\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected beacon-beam filter pack");
    let filtered_roots = base
        .with_client_resource_generation(92, filtered)
        .expect("filtered beacon-beam generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required beacon-beam stage must reject the generation");
    assert!(error.to_string().contains("rendertype_beacon_beam"));
    assert_eq!(renderer.core_shader_generation(), Some(91));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_lightning_shader_preloads_weather_and_dragon_variants_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-lightning-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_lightning.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + vertexColor.a;
fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected lightning shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(101, selected)
        .expect("selected lightning generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected weather/dragon lightning variants compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(101));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_lightning\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected lightning filter pack");
    let filtered_roots = base
        .with_client_resource_generation(102, filtered)
        .expect("filtered lightning generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required lightning stage must reject the generation");
    assert!(error.to_string().contains("rendertype_lightning"));
    assert_eq!(renderer.core_shader_generation(), Some(101));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_line_shader_preloads_depth_variants_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-line-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_lines.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + vertexColor.a;
fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected line shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(111, selected)
        .expect("selected line generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected line depth variants compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(111));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_lines\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected line filter pack");
    let filtered_roots = base
        .with_client_resource_generation(112, filtered)
        .expect("filtered line generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required line stage must reject the generation");
    assert!(error.to_string().contains("rendertype_lines"));
    assert_eq!(renderer.core_shader_generation(), Some(111));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_text_shaders_preload_debug_display_and_see_through_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-text-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    let normal_fragment = r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + texCoord0.x;
fragColor = vec4(0.0, 1.0, 0.0, vertexColor.a) + vec4(keep * 0.0);
}
"#;
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_text.fsh"),
        normal_fragment,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_text_see_through.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
fragColor = vec4(0.0, 1.0, 0.0, vertexColor.a + texCoord0.x * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected text shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(121, selected)
        .expect("selected text generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected debug/display text variants compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(121));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_text(?:_see_through)?\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected text filter pack");
    let filtered_roots = base
        .with_client_resource_generation(122, filtered)
        .expect("filtered text generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required text stage must reject the generation");
    assert!(error.to_string().contains("rendertype_text"));
    assert_eq!(renderer.core_shader_generation(), Some(121));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_text_background_shader_preloads_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-text-background-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_text_background.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + vertexColor.a;
fragColor = vec4(0.0, 1.0, 0.0, vertexColor.a) + vec4(keep * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected text-background shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(131, selected)
        .expect("selected text-background generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected text-background pipeline compiles atomically");
    assert_eq!(renderer.core_shader_generation(), Some(131));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_text_background\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected text-background filter pack");
    let filtered_roots = base
        .with_client_resource_generation(132, filtered)
        .expect("filtered text-background generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering required text-background stages must reject the generation");
    assert!(error.to_string().contains("rendertype_text_background"));
    assert_eq!(renderer.core_shader_generation(), Some(131));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_cloud_shader_preloads_flat_and_fancy_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-cloud-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_clouds.fsh"),
        r#"#version 330
in float vertexDistance;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
fragColor = vec4(0.0, 1.0, 0.0, vertexColor.a + vertexDistance * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected cloud shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(141, selected)
        .expect("selected cloud generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected flat/fancy cloud pipelines compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(141));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/rendertype_clouds\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected cloud filter pack");
    let filtered_roots = base
        .with_client_resource_generation(142, filtered)
        .expect("filtered cloud generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering required cloud stages must reject the generation");
    assert!(error.to_string().contains("rendertype_clouds"));
    assert_eq!(renderer.core_shader_generation(), Some(141));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_lightmap_shader_preloads_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-lightmap-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/lightmap.fsh"),
        r#"#version 330
layout(std140) uniform LightmapInfo {
float SkyFactor;
float BlockFactor;
float NightVisionFactor;
float DarknessScale;
float BossOverlayWorldDarkeningFactor;
float BrightnessFactor;
vec3 BlockLightTint;
vec3 SkyLightColor;
vec3 AmbientColor;
vec3 NightVisionColor;
} lightmapInfo;
in vec2 texCoord;
out vec4 fragColor;
void main() {
fragColor = vec4(0.0, 1.0, texCoord.x * 0.0, 1.0 + lightmapInfo.SkyFactor * 0.0);
}
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected lightmap shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(151, selected)
        .expect("selected lightmap generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let mut renderer = crate::Renderer::new_offscreen(32, 32)
        .expect("ignored shader integration test requires a GPU adapter and device");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected lightmap pipeline compiles atomically");
    assert_eq!(renderer.core_shader_generation(), Some(151));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/(?:screenquad\\.vsh|lightmap\\.fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected lightmap filter pack");
    let filtered_roots = base
        .with_client_resource_generation(152, filtered)
        .expect("filtered lightmap generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering required lightmap stages must reject the generation");
    assert!(error.to_string().contains("screenquad") || error.to_string().contains("lightmap"));
    assert_eq!(renderer.core_shader_generation(), Some(151));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_sky_and_position_tex_color_families_preload_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-sky-family");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/sky.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
out vec4 fragColor;
void main() {
fragColor = vec4(0.0, 1.0, sphericalVertexDistance * 0.0, 1.0 + cylindricalVertexDistance * 0.0) * ColorModulator;
}
"#,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/stars.fsh"),
        "#version 330\nout vec4 fragColor;\nvoid main() { fragColor = ColorModulator; }\n",
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/position_tex_color.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in vec2 texCoord0;
in vec4 vertexColor;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * vertexColor * ColorModulator; }
"#,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/position_tex.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * ColorModulator; }
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected sky-family shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(161, selected)
        .expect("selected sky-family generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    for shader_id in [
        SKY_SHADER_ID,
        STARS_SHADER_ID,
        POSITION_TEX_COLOR_SHADER_ID,
        POSITION_TEX_SHADER_ID,
    ] {
        assert_eq!(
            catalog
                .source_by_id(shader_id, ShaderStage::Fragment)
                .expect("selected sky-family fragment")
                .pack_id,
            "startup/0"
        );
    }

    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect(
            "selected sky, stars, celestial, end-sky, HUD, and vignette pipelines compile atomically",
        );
    assert_eq!(renderer.core_shader_generation(), Some(161));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/(?:sky|stars|position_tex_color|position_tex)\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected sky-family filter pack");
    let filtered_roots = base
        .with_client_resource_generation(162, filtered)
        .expect("filtered sky-family generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering required sky-family stages must reject the generation");
    assert!([
        "core/sky",
        "core/stars",
        "position_tex_color",
        "position_tex"
    ]
    .iter()
    .any(|needle| error.to_string().contains(needle)));
    assert_eq!(renderer.core_shader_generation(), Some(161));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_entity_special_family_preloads_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-entity-special");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/position.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
out vec4 fragColor;
void main() { fragColor = vec4(0.0, 1.0, sphericalVertexDistance * 0.0, 1.0 + cylindricalVertexDistance * 0.0) * ColorModulator; }
"#,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_leash.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
flat in vec4 vertexColor;
out vec4 fragColor;
void main() { fragColor = vertexColor + vec4((sphericalVertexDistance + cylindricalVertexDistance) * 0.0); }
"#,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_water_mask.fsh"),
        "#version 330\nout vec4 fragColor;\nvoid main() { fragColor = ColorModulator; }\n",
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_entity_shadow.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, clamp(texCoord0, 0.0, 1.0)) * vertexColor + vec4((sphericalVertexDistance + cylindricalVertexDistance) * 0.0); }
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected entity-special shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(171, selected)
        .expect("selected entity-special generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected dragon-depth, leash, water-mask, and shadow compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(171));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/(?:position|rendertype_leash|rendertype_water_mask|rendertype_entity_shadow)\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected entity-special filter pack");
    let filtered_roots = base
        .with_client_resource_generation(172, filtered)
        .expect("filtered entity-special generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering required entity-special stages must reject the generation");
    assert!(error.to_string().contains("core/position"));
    assert_eq!(renderer.core_shader_generation(), Some(171));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_final_static_core_family_preloads_and_filter_rolls_back() {
    let root = unique_temp_dir("selected-final-static-core");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_crumbling.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * vertexColor + vec4((sphericalVertexDistance + cylindricalVertexDistance) * 0.0); }
"#,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/rendertype_world_border.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * ColorModulator; }
"#,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/block.fsh"),
        r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * vertexColor * ColorModulator + vec4((sphericalVertexDistance + cylindricalVertexDistance) * 0.0); }
"#,
    );
    write_text(
        &overlay.join("assets/minecraft/shaders/core/blit_screen.fsh"),
        r#"#version 330
uniform sampler2D InSampler;
in vec2 texCoord;
out vec4 fragColor;
void main() { fragColor = texture(InSampler, texCoord); }
"#,
    );
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected final-static shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(181, selected)
        .expect("selected final-static generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    for shader_id in [
        CRUMBLING_SHADER_ID,
        WORLD_BORDER_SHADER_ID,
        BLOCK_SHADER_ID,
        BLIT_SCREEN_SHADER_ID,
    ] {
        assert_eq!(
            catalog
                .source_by_id(shader_id, ShaderStage::Fragment)
                .expect("selected final-static fragment")
                .pack_id,
            "startup/0"
        );
    }

    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected crumbling, world-border, resident-block, and outline-blit pipelines compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(181));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/(?:rendertype_crumbling|rendertype_world_border|block|screenquad|blit_screen)\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected final-static filter pack");
    let filtered_roots = base
        .with_client_resource_generation(182, filtered)
        .expect("filtered final-static generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering required final-static stages must reject the generation");
    assert!([
        "rendertype_crumbling",
        "rendertype_world_border",
        "core/block",
        "blit_screen",
        "screenquad",
    ]
    .iter()
    .any(|needle| error.to_string().contains(needle)));
    assert_eq!(renderer.core_shader_generation(), Some(181));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_item_shader_drives_dynamic_and_preloads_resident_item_surfaces() {
    let root = unique_temp_dir("selected-item-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    write_text(
        &overlay.join("assets/minecraft/shaders/core/item.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + vertexColor.x + texCoord0.x;
fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
    );
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected item shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(71, selected)
        .expect("selected item generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    assert_eq!(
        catalog
            .source_by_id(ITEM_SHADER_ID, ShaderStage::Fragment)
            .expect("selected item fragment")
            .pack_id,
        "startup/0"
    );

    let mut renderer = crate::Renderer::new_offscreen(24, 20)
        .expect("ignored shader pixel test requires a GPU adapter and device");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected dynamic and resident item pipelines compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(71));
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    let camera = crate::CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    renderer.set_camera_pose(Some(camera));
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .expect("white item atlas");
    let item = crate::ItemModelQuad {
        corners: [
            [0.0, 0.0, 0.0],
            [0.0, 16.0, 0.0],
            [16.0, 16.0, 0.0],
            [16.0, 0.0, 0.0],
        ],
        uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        tint: [1.0; 4],
        normal: [0.0, 0.0, -1.0],
        shade: 1.0,
        translucent: false,
    };
    renderer
        .set_item_model_residency_frame(resident_test_item_frame(item, false))
        .expect("resident item frame");
    let pixels = renderer
        .render_offscreen_frame()
        .expect("selected item frame");
    assert!(frame_has_green_item_pixel(&pixels));

    renderer
        .set_item_model_residency_frame(crate::ItemModelResidencyFrame::default())
        .expect("clear resident items for isolated flame frame");
    let flame = crate::bake_entity_flame_surface(
        crate::EntityModelInstance::new(1, crate::EntityModelKind::NoRender, [0.0, -0.5, 4.0], 0.0)
            .with(|state| state.flame_dimensions = Some([1.0, 2.0])),
        camera,
        crate::terrain::TerrainUvRect::UNIT,
        crate::terrain::TerrainUvRect::UNIT,
    )
    .expect("visible entity flame");
    renderer.set_entity_flame_surfaces(vec![flame]);
    let pixels = renderer
        .render_offscreen_frame()
        .expect("selected item shader flame frame");
    assert_eq!(pixels.pixel(12, 10), [0, 255, 0, 255]);
    renderer.set_entity_flame_surfaces(Vec::new());
    let pixels = renderer
        .render_offscreen_frame()
        .expect("cleared flame frame");
    assert_eq!(pixels.pixel(12, 10), [0, 0, 255, 255]);
    renderer
        .set_item_model_residency_frame(resident_test_item_frame(item, false))
        .expect("restore resident item frame");

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/item\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected item filter pack");
    let filtered_roots = base
        .clone()
        .with_client_resource_generation(72, filtered)
        .expect("filtered item generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required item stage must reject the generation");
    assert!(error.to_string().contains("minecraft:core/item"));
    assert_eq!(renderer.core_shader_generation(), Some(71));
    assert!(frame_has_green_item_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("retained selected item frame")
    ));

    let default_stack = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [])
        .expect("default selected stack");
    let default_roots = base
        .with_client_resource_generation(73, default_stack)
        .expect("default item generation");
    let default_catalog = bbb_client_assets::ShaderSourceCatalog::load(&default_roots)
        .expect("default shader catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
        })
        .expect("removing selected item shader restores embedded generation");
    assert_eq!(renderer.core_shader_generation(), Some(73));
    assert!(!frame_has_green_item_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("restored embedded item frame")
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_glint_shader_drives_item_pixels_and_preloads_all_current_glint_surfaces() {
    let root = unique_temp_dir("selected-glint-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    write_text(
        &overlay.join("assets/minecraft/shaders/core/glint.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + texCoord0.x;
fragColor = vec4(0.0, 0.75, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
    );
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected glint shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(76, selected)
        .expect("selected glint generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    assert_eq!(
        catalog
            .source_by_id(GLINT_SHADER_ID, ShaderStage::Fragment)
            .expect("selected glint fragment")
            .pack_id,
        "startup/0"
    );

    let mut renderer = crate::Renderer::new_offscreen(24, 20)
        .expect("ignored shader integration test requires a GPU adapter and device");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("all five selected glint pipelines compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(76));
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_pose(Some(crate::CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    }));
    renderer
        .upload_terrain_texture_atlas(1, 1, &[0, 0, 0, 255])
        .expect("black item atlas");
    renderer
        .upload_item_glint_texture(1, 1, &[255; 4])
        .expect("white glint texture");
    let item = crate::ItemModelQuad {
        corners: [
            [0.0, 0.0, 0.0],
            [0.0, 16.0, 0.0],
            [16.0, 16.0, 0.0],
            [16.0, 0.0, 0.0],
        ],
        uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        tint: [1.0; 4],
        normal: [0.0, 0.0, -1.0],
        shade: 1.0,
        translucent: false,
    };
    renderer
        .set_item_model_residency_frame(resident_test_item_frame(item, true))
        .expect("resident foiled item frame");
    assert!(frame_has_green_glint_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("selected glint frame")
    ));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/glint\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected glint filter pack");
    let filtered_roots = base
        .clone()
        .with_client_resource_generation(77, filtered)
        .expect("filtered glint generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering required glint stages must reject the generation");
    assert!(error.to_string().contains("minecraft:core/glint"));
    assert_eq!(renderer.core_shader_generation(), Some(76));
    assert!(frame_has_green_glint_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("retained selected glint frame")
    ));

    let default_stack = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [])
        .expect("default selected stack");
    let default_roots = base
        .with_client_resource_generation(78, default_stack)
        .expect("default glint generation");
    let default_catalog = bbb_client_assets::ShaderSourceCatalog::load(&default_roots)
        .expect("default shader catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
        })
        .expect("removing selected glint shader restores embedded generation");
    assert_eq!(renderer.core_shader_generation(), Some(78));
    assert!(!frame_has_green_glint_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("restored embedded glint frame")
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_particle_shader_preloads_particle_and_weather_surfaces() {
    let root = unique_temp_dir("selected-particle-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    write_text(
        &overlay.join("assets/minecraft/shaders/core/particle.fsh"),
        r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec2 texCoord0;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
float keep = sphericalVertexDistance + cylindricalVertexDistance + texCoord0.x + vertexColor.x;
fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
    );
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected particle shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(81, selected)
        .expect("selected particle generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    assert_eq!(
        catalog
            .source_by_id(PARTICLE_SHADER_ID, ShaderStage::Fragment)
            .expect("selected particle fragment")
            .pack_id,
        "startup/0"
    );

    let Some(mut renderer) = crate::Renderer::new_offscreen(24, 20) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected opaque/translucent particle and weather pipelines compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(81));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/particle\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected particle filter pack");
    let filtered_roots = base
        .clone()
        .with_client_resource_generation(82, filtered)
        .expect("filtered particle generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required particle stage must reject the generation");
    assert!(error.to_string().contains("minecraft:core/particle"));
    assert_eq!(renderer.core_shader_generation(), Some(81));

    let default_stack = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [])
        .expect("default selected stack");
    let default_roots = base
        .with_client_resource_generation(83, default_stack)
        .expect("default particle generation");
    let default_catalog = bbb_client_assets::ShaderSourceCatalog::load(&default_roots)
        .expect("default shader catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
        })
        .expect("removing selected particle shader restores embedded generation");
    assert_eq!(renderer.core_shader_generation(), Some(83));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
fn selected_position_color_shader_drives_debug_and_preloads_sunrise_surface() {
    let root = unique_temp_dir("selected-position-color-shader");
    let overlay = root.join("overlay");
    write_pack_metadata(&overlay, None);
    let base = bbb_client_assets::PackRoots::from_environment().expect("local vanilla roots");
    write_text(
        &overlay.join("assets/minecraft/shaders/core/position_color.fsh"),
        r#"#version 330
in vec4 vertexColor;
out vec4 fragColor;
void main() {
fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(vertexColor.x * 0.0);
}
"#,
    );
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
            .expect("selected position-color shader pack");
    let roots = base
        .clone()
        .with_client_resource_generation(91, selected)
        .expect("selected position-color generation");
    let catalog =
        bbb_client_assets::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
    assert_eq!(
        catalog
            .source_by_id(POSITION_COLOR_SHADER_ID, ShaderStage::Fragment)
            .expect("selected position-color fragment")
            .pack_id,
        "startup/0"
    );

    let Some(mut renderer) = crate::Renderer::new_offscreen(24, 20) else {
        std::fs::remove_dir_all(root).unwrap();
        return;
    };
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&catalog.renderer_projection(), false)
        })
        .expect("selected debug-filled and sunrise pipelines compile atomically");
    assert_eq!(renderer.core_shader_generation(), Some(91));
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_debug_filled_boxes(vec![crate::DebugFilledBox {
        min: [-0.5, -0.5, 0.25],
        max: [0.5, 0.5, 0.75],
        color: [255; 4],
    }]);
    assert!(frame_has_green_position_color_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("selected position-color debug frame")
    ));

    let filter = root.join("filter");
    write_pack_metadata(
        &filter,
        Some(
            r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/position_color\\.(?:vsh|fsh)"}]}"#,
        ),
    );
    let filtered = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [filter])
        .expect("selected position-color filter pack");
    let filtered_roots = base
        .clone()
        .with_client_resource_generation(92, filtered)
        .expect("filtered position-color generation");
    let filtered_catalog = bbb_client_assets::ShaderSourceCatalog::load(&filtered_roots)
        .expect("filtered shader catalog");
    let error = renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
        })
        .expect_err("filtering a required position-color stage must reject the generation");
    assert!(error.to_string().contains("minecraft:core/position_color"));
    assert_eq!(renderer.core_shader_generation(), Some(91));
    assert!(frame_has_green_position_color_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("retained selected position-color frame")
    ));

    let default_stack = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&base, [])
        .expect("default selected stack");
    let default_roots = base
        .with_client_resource_generation(93, default_stack)
        .expect("default position-color generation");
    let default_catalog = bbb_client_assets::ShaderSourceCatalog::load(&default_roots)
        .expect("default shader catalog");
    renderer
        .transactional_resource_reload(|renderer| {
            renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
        })
        .expect("removing selected position-color shader restores embedded generation");
    assert_eq!(renderer.core_shader_generation(), Some(93));
    assert!(!frame_has_green_position_color_pixel(
        &renderer
            .render_offscreen_frame()
            .expect("restored embedded position-color frame")
    ));

    std::fs::remove_dir_all(root).unwrap();
}

fn frame_has_green_entity_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
    (0..pixels.height).any(|y| {
        (0..pixels.width).any(|x| {
            let pixel = pixels.pixel(x, y);
            pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
        })
    })
}

fn frame_has_green_item_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
    (0..pixels.height).any(|y| {
        (0..pixels.width).any(|x| {
            let pixel = pixels.pixel(x, y);
            pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
        })
    })
}

fn frame_has_green_position_color_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
    (0..pixels.height).any(|y| {
        (0..pixels.width).any(|x| {
            let pixel = pixels.pixel(x, y);
            pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
        })
    })
}

fn frame_has_green_glint_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
    (0..pixels.height).any(|y| {
        (0..pixels.width).any(|x| {
            let pixel = pixels.pixel(x, y);
            pixel[1] > 100 && pixel[0] < 20 && pixel[2] < 20
        })
    })
}

fn write_pack_metadata(root: &Path, suffix: Option<&str>) {
    write_text(
        &root.join("pack.mcmeta"),
        &format!(
            r#"{{"pack":{{"description":"test","min_format":[84,0],"max_format":[84,0]}}{}}}"#,
            suffix.unwrap_or_default()
        ),
    );
}

fn write_text(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-client-renderer-{label}-{nanos}"))
}
