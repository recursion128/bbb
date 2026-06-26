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
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );

    assert_eq!(passes[1].kind, EntityModelLayerKind::CopperGolemEyes);
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].model_layer, MODEL_LAYER_COPPER_GOLEM);
    assert_eq!(passes[1].texture, COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF);
    assert_eq!(passes[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(
        (passes[1].collector_order, passes[1].submit_sequence),
        (1, 1)
    );
}

#[test]
fn copper_golem_textures_are_in_entity_model_atlas() {
    for texture in copper_golem_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
    }
}
