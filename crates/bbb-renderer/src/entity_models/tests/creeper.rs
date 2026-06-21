use super::*;

#[test]
fn creeper_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        CREEPER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color: CREEPER_GREEN
        }
    );
    assert_eq!(
        CREEPER_BODY[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color: CREEPER_GREEN
        }
    );
    assert_eq!(
        CREEPER_LEG[0],
        ModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            color: CREEPER_GREEN
        }
    );

    assert_eq!(CREEPER_PARTS.len(), 6);
    assert_eq!(CREEPER_PARTS[0].pose.offset, [0.0, 6.0, 0.0]);
    assert_eq!(CREEPER_PARTS[0].cubes, CREEPER_HEAD.as_slice());
    assert_eq!(CREEPER_PARTS[1].pose.offset, [0.0, 6.0, 0.0]);
    assert_eq!(CREEPER_PARTS[1].cubes, CREEPER_BODY.as_slice());

    let leg_offsets = [
        [-2.0, 18.0, 4.0],
        [2.0, 18.0, 4.0],
        [-2.0, 18.0, -4.0],
        [2.0, 18.0, -4.0],
    ];
    for (part, expected_offset) in CREEPER_PARTS[2..].iter().zip(leg_offsets) {
        assert_eq!(part.pose.offset, expected_offset);
        assert_eq!(part.pose.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(part.cubes, CREEPER_LEG.as_slice());
        assert!(part.children.is_empty());
    }
}

#[test]
fn creeper_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::new(
        50,
        EntityModelKind::Creeper,
        [0.0, 64.0, 0.0],
        0.0,
    )]);

    assert_eq!(mesh.opaque_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn creeper_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Creeper.model_key(), "creeper");
    assert_eq!(
        EntityModelKind::Creeper.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/creeper/creeper.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_temperate.png",
            size: [64, 32],
        })
    );
}

#[test]
fn creeper_textured_layer_passes_match_vanilla_renderer_model_layer() {
    let passes = creeper_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::CreeperBase);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_CREEPER);
    assert_eq!(passes[0].texture, CREEPER_TEXTURE_REF);
    assert_eq!(passes[0].parts, CREEPER_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn creeper_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_CREEPER, "minecraft:creeper#main");
    assert_eq!(CREEPER_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        CREEPER_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        CREEPER_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv_size: [8.0, 12.0, 4.0],
            tex: [16.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        CREEPER_TEXTURED_LEG[0],
        TexturedModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            uv_size: [4.0, 6.0, 4.0],
            tex: [0.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(CREEPER_TEXTURED_PARTS[0].pose, CREEPER_PARTS[0].pose);
    assert_eq!(CREEPER_TEXTURED_PARTS[1].pose, CREEPER_PARTS[1].pose);
    assert_eq!(CREEPER_TEXTURED_PARTS[5].pose, CREEPER_PARTS[5].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_creeper_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 32);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/creeper/creeper.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn creeper_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::new(
            910,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0,
        )],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn creeper_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(199, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn creeper_swings_its_legs_when_walking() {
    // Vanilla `CreeperModel` is a custom `EntityModel` whose `setupAnim` leg swing is
    // exactly the `QuadrupedModel` formula (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`,
    // hind-right/front-left in phase, legs at [2, 3, 4, 5]). A standing creeper is
    // inert; a walking one lifts its feet and splays its legs along Z. The swelling
    // scale and powered charge layer are deferred. Colored path here, textured below.
    let base = EntityModelInstance::new(250, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(rest.vertices, still.vertices, "rest is inert");

    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_ne!(rest.vertices, walking.vertices, "walking differs");

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking creeper's feet should lift off the ground"
    );
    assert!(
        (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
        "a walking creeper's legs should splay along Z"
    );
}

#[test]
fn creeper_textured_mesh_swings_legs_when_walking() {
    // The real creeper render path (texture-backed) swings the same `QuadrupedModel`
    // legs. A standing creeper is byte-identical however far the swing has advanced;
    // a walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(251, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

    assert_eq!(
        resting.vertices, still.vertices,
        "a standing creeper is inert"
    );
    assert_eq!(
        resting.vertices.len(),
        walking.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_ne!(
        resting.vertices, walking.vertices,
        "a walking creeper differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting);
    let (walk_min, walk_max) = textured_mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking creeper's feet should lift off the ground"
    );
}

fn creeper_texture_images() -> Vec<EntityModelTextureImage> {
    creeper_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
