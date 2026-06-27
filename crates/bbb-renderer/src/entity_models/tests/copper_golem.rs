use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn copper_golem_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        COPPER_GOLEM_BODY[0],
        ModelCube::new(
            [-4.0, -6.0, -3.0],
            [8.0, 6.0, 6.0],
            COPPER_GOLEM_COPPER,
            [8.0, 6.0, 6.0],
            [0.0, 15.0],
            false,
        )
    );
    assert_eq!(COPPER_GOLEM_HEAD.len(), 4);
    assert_eq!(COPPER_GOLEM_HEAD[0].min, [-4.015, -5.015, -5.015]);
    assert_eq!(COPPER_GOLEM_HEAD[0].size, [8.03, 5.03, 10.03]);
    assert_eq!(COPPER_GOLEM_HEAD[0].uv_size, [8.0, 5.0, 10.0]);
    assert_eq!(COPPER_GOLEM_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(COPPER_GOLEM_HEAD[1].tex, [56.0, 0.0]);
    assert_eq!(COPPER_GOLEM_HEAD[2].min, [-0.985, -8.985, -0.985]);
    assert_eq!(COPPER_GOLEM_HEAD[2].uv_size, [2.0, 4.0, 2.0]);
    assert_eq!(COPPER_GOLEM_HEAD[2].tex, [37.0, 8.0]);
    assert_eq!(COPPER_GOLEM_HEAD[3].uv_size, [4.0, 4.0, 4.0]);
    assert_eq!(COPPER_GOLEM_HEAD[3].tex, [37.0, 0.0]);
    assert_eq!(COPPER_GOLEM_RIGHT_ARM[0].tex, [36.0, 16.0]);
    assert_eq!(COPPER_GOLEM_LEFT_ARM[0].tex, [50.0, 16.0]);
    assert_eq!(COPPER_GOLEM_RIGHT_LEG[0].tex, [0.0, 27.0]);
    assert_eq!(COPPER_GOLEM_LEFT_LEG[0].tex, [16.0, 27.0]);

    assert_eq!(COPPER_GOLEM_ROOT_POSE.offset, [0.0, 24.0, 0.0]);
    assert_eq!(COPPER_GOLEM_BODY_POSE.offset, [0.0, -5.0, 0.0]);
    assert_eq!(COPPER_GOLEM_HEAD_POSE.offset, [0.0, -6.0, 0.0]);
    assert_eq!(COPPER_GOLEM_RIGHT_ARM_POSE.offset, [-4.0, -6.0, 0.0]);
    assert_eq!(COPPER_GOLEM_LEFT_ARM_POSE.offset, [4.0, -6.0, 0.0]);
    assert_eq!(COPPER_GOLEM_LEG_POSE.offset, [0.0, -5.0, 0.0]);
}

#[test]
fn copper_golem_texture_ref_matches_vanilla_renderer_weathering() {
    let cases = [
        (
            CopperGolemWeathering::Unaffected,
            "textures/entity/copper_golem/copper_golem.png",
            "textures/entity/copper_golem/copper_golem_eyes.png",
        ),
        (
            CopperGolemWeathering::Exposed,
            "textures/entity/copper_golem/copper_golem_exposed.png",
            "textures/entity/copper_golem/copper_golem_eyes_exposed.png",
        ),
        (
            CopperGolemWeathering::Weathered,
            "textures/entity/copper_golem/copper_golem_weathered.png",
            "textures/entity/copper_golem/copper_golem_eyes_weathered.png",
        ),
        (
            CopperGolemWeathering::Oxidized,
            "textures/entity/copper_golem/copper_golem_oxidized.png",
            "textures/entity/copper_golem/copper_golem_eyes_oxidized.png",
        ),
    ];

    for (weathering, base_path, eyes_path) in cases {
        let kind = EntityModelKind::CopperGolem { weathering };
        assert_eq!(kind.model_key(), "copper_golem");
        assert_eq!(
            kind.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: base_path,
                size: [64, 64],
            })
        );
        assert_eq!(
            kind.vanilla_layer_texture_refs(),
            &[EntityModelTextureRef {
                path: eyes_path,
                size: [64, 64],
            }]
        );
    }
}

#[test]
fn copper_golem_weathering_from_vanilla_id_clamps_like_weather_state_codec() {
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(-1),
        CopperGolemWeathering::Unaffected
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(0),
        CopperGolemWeathering::Unaffected
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(1),
        CopperGolemWeathering::Exposed
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(2),
        CopperGolemWeathering::Weathered
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(3),
        CopperGolemWeathering::Oxidized
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(4),
        CopperGolemWeathering::Oxidized
    );
}

#[test]
fn copper_golem_textured_layer_passes_match_vanilla_renderer() {
    let passes = copper_golem_textured_layer_passes(CopperGolemWeathering::Weathered);

    assert_eq!(passes.len(), 2);
    assert_eq!(passes[0].kind, EntityModelLayerKind::CopperGolemBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].model_layer, MODEL_LAYER_COPPER_GOLEM);
    assert_eq!(passes[0].texture, COPPER_GOLEM_WEATHERED_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));

    assert_eq!(passes[1].kind, EntityModelLayerKind::CopperGolemEyes);
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].model_layer, MODEL_LAYER_COPPER_GOLEM);
    assert_eq!(passes[1].texture, COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF);
    assert_eq!(passes[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!((passes[1].order, passes[1].submit_sequence), (1, 1));
}

#[test]
fn copper_golem_textured_submissions_pin_living_emissive_metadata() {
    let images: Vec<EntityModelTextureImage> = copper_golem_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::new(
        910,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Weathered,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 2);

    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, COPPER_GOLEM_WEATHERED_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, entity_model_root_transform(instance));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert_ne!(base.overlay, [0.0, 10.0]);
    assert_eq!((base.order, base.submit_sequence), (0, 0));

    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.render_type.vanilla_name(), "eyes");
    assert_eq!(eyes.texture, COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF);
    assert_eq!(eyes.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes.transform, base.transform);
    assert_eq!(eyes.light, instance.render_state.shader_light());
    assert_eq!(
        eyes.overlay,
        [0.0, instance.render_state.overlay_coords()[1]]
    );
    assert_ne!(eyes.overlay, base.overlay);
    assert_ne!(eyes.overlay, [0.0, 10.0]);
    assert_eq!((eyes.order, eyes.submit_sequence), (1, 1));

    assert!(!meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.eyes.vertices.len(), meshes.cutout.vertices.len());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.light && vertex.overlay == base.overlay));
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.light == eyes.light && vertex.overlay == eyes.overlay));
}

#[test]
fn copper_golem_eyes_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `CopperGolemRenderer` records its LivingEntityEmissiveLayer eyes submit at order(1);
    // missing weathered eyes texture data suppresses only the folded emissive eyes geometry.
    let images: Vec<EntityModelTextureImage> = copper_golem_entity_texture_refs()
        .iter()
        .enumerate()
        .filter_map(|(index, texture)| {
            if *texture == COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF {
                return None;
            }
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            Some(EntityModelTextureImage::new(
                *texture,
                vec![index as u8; len],
            ))
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::new(
        911,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Weathered,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with_light_coords((2_u32 << 4) | (14_u32 << 20))
    .with_white_overlay_progress(0.7)
    .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, COPPER_GOLEM_WEATHERED_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, entity_model_root_transform(instance));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert_eq!((base.order, base.submit_sequence), (0, 0));

    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.render_type.vanilla_name(), "eyes");
    assert_eq!(eyes.texture, COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF);
    assert_eq!(eyes.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes.transform, base.transform);
    assert_eq!(eyes.light, instance.render_state.shader_light());
    assert_eq!(
        eyes.overlay,
        [0.0, instance.render_state.overlay_coords()[1]]
    );
    assert_ne!(eyes.overlay, base.overlay);
    assert_eq!((eyes.order, eyes.submit_sequence), (1, 1));

    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.eyes.indices.is_empty());
}

#[test]
fn copper_golem_textures_are_in_entity_model_atlas() {
    for texture in copper_golem_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
    }
}
