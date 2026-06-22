use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

#[test]
fn minecart_parts_match_vanilla_26_1_body_layer() {
    // Vanilla MinecartModel.createBodyLayer(): a 20x16x2 floor panel laid flat plus four
    // 16x8x2 wall panels boxed in. No setupAnim, so the cart is static.
    assert_eq!(MINECART_PARTS.len(), 5);

    // bottom: texOffs(0, 10), addBox(-10, -8, -1, 20, 16, 2), rotated 90deg on X and offset y 4.
    assert_eq!(MINECART_PARTS[0].pose.offset, [0.0, 4.0, 0.0]);
    assert_eq!(MINECART_PARTS[0].pose.rotation, [FRAC_PI_2, 0.0, 0.0]);
    assert_eq!(MINECART_PARTS[0].cubes[0].min, [-10.0, -8.0, -1.0]);
    assert_eq!(MINECART_PARTS[0].cubes[0].size, [20.0, 16.0, 2.0]);

    // The four walls share one 16x8x2 box, rotated to face out of each side.
    let wall_rotations = [
        [0.0, PI * 1.5, 0.0],
        [0.0, FRAC_PI_2, 0.0],
        [0.0, PI, 0.0],
        [0.0, 0.0, 0.0],
    ];
    let wall_offsets = [
        [-9.0, 4.0, 0.0],
        [9.0, 4.0, 0.0],
        [0.0, 4.0, -7.0],
        [0.0, 4.0, 7.0],
    ];
    for index in 0..4 {
        let part = &MINECART_PARTS[index + 1];
        assert_eq!(part.pose.offset, wall_offsets[index], "wall {index} offset");
        assert_eq!(
            part.pose.rotation, wall_rotations[index],
            "wall {index} rot"
        );
        assert_eq!(part.cubes[0].min, [-8.0, -9.0, -1.0]);
        assert_eq!(part.cubes[0].size, [16.0, 8.0, 2.0]);
    }
}

#[test]
fn minecart_textured_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_MINECART, "minecraft:minecart#main");
    assert_eq!(MINECART_TEXTURE_REF.size, [64, 32]);
    assert_eq!(MINECART_TEXTURED_PARTS.len(), 5);

    // The floor samples texOffs(0, 10); every wall samples texOffs(0, 0). None are mirrored.
    assert_eq!(MINECART_TEXTURED_PARTS[0].cubes[0].tex, [0.0, 10.0]);
    assert_eq!(
        MINECART_TEXTURED_PARTS[0].cubes[0].uv_size,
        [20.0, 16.0, 2.0]
    );
    assert!(!MINECART_TEXTURED_PARTS[0].cubes[0].mirror);
    for index in 0..4 {
        let part = &MINECART_TEXTURED_PARTS[index + 1];
        assert_eq!(part.cubes[0].tex, [0.0, 0.0], "wall {index} texOffs");
        assert_eq!(part.cubes[0].uv_size, [16.0, 8.0, 2.0]);
        assert!(!part.cubes[0].mirror);
    }
    // The textured poses mirror the colored poses exactly.
    for index in 0..5 {
        assert_eq!(
            MINECART_TEXTURED_PARTS[index].pose.offset,
            MINECART_PARTS[index].pose.offset
        );
        assert_eq!(
            MINECART_TEXTURED_PARTS[index].pose.rotation,
            MINECART_PARTS[index].pose.rotation
        );
    }
}

#[test]
fn minecart_layer_passes_match_vanilla_renderer() {
    let passes = minecart_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::MinecartBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_MINECART);
    assert_eq!(passes[0].texture, MINECART_TEXTURE_REF);
    assert_eq!(passes[0].parts, MINECART_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn minecart_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Minecart.model_key(), "minecart");
    assert_eq!(
        EntityModelKind::Minecart.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/minecart/minecart.png",
            size: [64, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&MINECART_TEXTURE_REF));
    assert_eq!(minecart_entity_texture_refs(), &[MINECART_TEXTURE_REF]);
}

#[test]
fn minecart_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)]);
    // Five cubes => 30 faces, 120 verts, 180 indices.
    assert_eq!(mesh.opaque_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);
}

#[test]
fn minecart_textured_mesh_matches_colored_geometry_and_vanilla_uvs() {
    let (atlas, _) = build_entity_model_texture_atlas(&minecart_texture_images()).unwrap();
    let textured = entity_model_textured_mesh(
        &[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(textured.cutout_faces, 30);
    assert_eq!(textured.vertices.len(), 120);
    assert_eq!(textured.indices.len(), 180);
    assert!(textured
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // The textured cart shares the colored cart's geometry exactly (same parts and transform).
    let colored = entity_model_mesh(&[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)]);
    let (cmin, cmax) = mesh_extents(&colored);
    let (tmin, tmax) = textured_mesh_extents(&textured);
    assert_close3(tmin, cmin);
    assert_close3(tmax, cmax);
}

fn minecart_texture_images() -> Vec<EntityModelTextureImage> {
    minecart_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
