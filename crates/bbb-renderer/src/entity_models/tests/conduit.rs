use super::*;

use crate::entity_models::colored::conduit_model_root_transform;
use crate::entity_models::model::{EntityModel, ModelCube};
use crate::entity_models::model_layers::{
    conduit_entity_texture_refs, ConduitModel, CONDUIT_BASE_TEXTURE_REF, CONDUIT_CAGE_CUBE,
    CONDUIT_CAGE_TEXTURE_REF, CONDUIT_CLOSED_EYE_TEXTURE_REF, CONDUIT_EYE_CUBE,
    CONDUIT_OPEN_EYE_TEXTURE_REF, CONDUIT_SHELL_CUBE, CONDUIT_WIND_CUBE, CONDUIT_WIND_TEXTURE_REF,
    CONDUIT_WIND_VERTICAL_TEXTURE_REF, MODEL_LAYER_CONDUIT_CAGE, MODEL_LAYER_CONDUIT_EYE,
    MODEL_LAYER_CONDUIT_SHELL, MODEL_LAYER_CONDUIT_WIND,
};
use glam::Vec3;

fn conduit_instance(part: ConduitModelPart) -> EntityModelInstance {
    EntityModelInstance::conduit(-1, [2.0, 3.0, 4.0], part)
        .with_conduit_anim_time(0.0)
        .with_conduit_active_rotation(0.0)
}

#[test]
fn conduit_cubes_match_vanilla_26_1_model_layers() {
    // Vanilla `ConduitRenderer.createEyeLayer`: the 8x8 zero-depth eye plane is inflated by
    // CubeDeformation(0.01), while the UV box stays 8x8x0 on the 16x16 eye atlas.
    assert_eq!(CONDUIT_EYE_CUBE.min, [-4.01, -4.01, -0.01]);
    assert_eq!(CONDUIT_EYE_CUBE.size, [8.02, 8.02, 0.02]);
    assert_eq!(CONDUIT_EYE_CUBE.uv_size, [8.0, 8.0, 0.0]);
    assert_eq!(CONDUIT_EYE_CUBE.tex, [0.0, 0.0]);
    assert_eq!(
        CONDUIT_WIND_CUBE,
        ModelCube::new(
            [-8.0, -8.0, -8.0],
            [16.0, 16.0, 16.0],
            CONDUIT_WIND_CUBE.color,
            [16.0, 16.0, 16.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(CONDUIT_SHELL_CUBE.min, [-3.0, -3.0, -3.0]);
    assert_eq!(CONDUIT_SHELL_CUBE.size, [6.0, 6.0, 6.0]);
    assert_eq!(CONDUIT_CAGE_CUBE.min, [-4.0, -4.0, -4.0]);
    assert_eq!(CONDUIT_CAGE_CUBE.size, [8.0, 8.0, 8.0]);

    let model = ConduitModel::new(ConduitModelPart::Eye { open: true });
    assert!(model.root().try_child("missing").is_none());
}

#[test]
fn conduit_model_keys_and_texture_refs_match_vanilla_selection() {
    let shell = EntityModelKind::Conduit {
        part: ConduitModelPart::Shell,
    };
    assert_eq!(shell.model_key(), "conduit");
    assert_eq!(shell.vanilla_texture_ref(), Some(CONDUIT_BASE_TEXTURE_REF));
    assert_eq!(
        EntityModelKind::Conduit {
            part: ConduitModelPart::Cage,
        }
        .vanilla_texture_ref(),
        Some(CONDUIT_CAGE_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Conduit {
            part: ConduitModelPart::OuterWind { phase: 1 },
        }
        .vanilla_texture_ref(),
        Some(CONDUIT_WIND_VERTICAL_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Conduit {
            part: ConduitModelPart::OuterWind { phase: 2 },
        }
        .vanilla_texture_ref(),
        Some(CONDUIT_WIND_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Conduit {
            part: ConduitModelPart::Eye { open: true },
        }
        .vanilla_texture_ref(),
        Some(CONDUIT_OPEN_EYE_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Conduit {
            part: ConduitModelPart::Eye { open: false },
        }
        .vanilla_texture_ref(),
        Some(CONDUIT_CLOSED_EYE_TEXTURE_REF)
    );
    assert_eq!(conduit_entity_texture_refs().len(), 6);
    for texture in conduit_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
    }
}

#[test]
fn conduit_layer_passes_match_vanilla_renderer() {
    let shell = conduit_textured_layer_passes(ConduitModelPart::Shell);
    assert_eq!(shell.len(), 1);
    assert_eq!(shell[0].kind, EntityModelLayerKind::ConduitShell);
    assert_eq!(
        shell[0].render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(shell[0].model_layer, MODEL_LAYER_CONDUIT_SHELL);
    assert_eq!(shell[0].texture, CONDUIT_BASE_TEXTURE_REF);

    let cage = conduit_textured_layer_passes(ConduitModelPart::Cage);
    assert_eq!(cage[0].kind, EntityModelLayerKind::ConduitCage);
    assert_eq!(
        cage[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(cage[0].model_layer, MODEL_LAYER_CONDUIT_CAGE);
    assert_eq!(cage[0].texture, CONDUIT_CAGE_TEXTURE_REF);

    let vertical_wind = conduit_textured_layer_passes(ConduitModelPart::OuterWind { phase: 1 });
    assert_eq!(vertical_wind[0].kind, EntityModelLayerKind::ConduitWind);
    assert_eq!(vertical_wind[0].model_layer, MODEL_LAYER_CONDUIT_WIND);
    assert_eq!(vertical_wind[0].texture, CONDUIT_WIND_VERTICAL_TEXTURE_REF);

    let eye = conduit_textured_layer_passes(ConduitModelPart::Eye { open: false });
    assert_eq!(eye[0].kind, EntityModelLayerKind::ConduitEye);
    assert_eq!(eye[0].model_layer, MODEL_LAYER_CONDUIT_EYE);
    assert_eq!(eye[0].texture, CONDUIT_CLOSED_EYE_TEXTURE_REF);
}

#[test]
fn conduit_transforms_match_vanilla_submit_positions() {
    let shell = conduit_model_root_transform(
        conduit_instance(ConduitModelPart::Shell),
        ConduitModelPart::Shell,
    );
    assert!((shell.transform_point3(Vec3::ZERO) - Vec3::new(2.5, 3.5, 4.5)).length() < 1.0e-6);

    let cage = conduit_model_root_transform(
        conduit_instance(ConduitModelPart::Cage),
        ConduitModelPart::Cage,
    );
    // At animTime 0, vanilla bob is (0.5^2 + 0.5), so y = 0.3 + 0.75 * 0.2.
    assert!((cage.transform_point3(Vec3::ZERO) - Vec3::new(2.5, 3.45, 4.5)).length() < 1.0e-6);

    let wind = conduit_model_root_transform(
        conduit_instance(ConduitModelPart::OuterWind { phase: 1 }),
        ConduitModelPart::OuterWind { phase: 1 },
    );
    assert!((wind.transform_point3(Vec3::Y) - Vec3::new(2.5, 3.5, 5.5)).length() < 1.0e-6);

    let inner = conduit_model_root_transform(
        conduit_instance(ConduitModelPart::InnerWind { vertical: false }),
        ConduitModelPart::InnerWind { vertical: false },
    );
    assert!((inner.transform_point3(Vec3::X) - Vec3::new(1.625, 3.5, 4.5)).length() < 1.0e-6);
}

#[test]
fn conduit_textured_mesh_submits_shell_and_active_parts() {
    let images: Vec<EntityModelTextureImage> = conduit_entity_texture_refs()
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![9; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instances = [
        conduit_instance(ConduitModelPart::Shell).with_light_coords((4_u32 << 4) | (12_u32 << 20)),
        conduit_instance(ConduitModelPart::Cage).with_light_coords((5_u32 << 4) | (11_u32 << 20)),
        conduit_instance(ConduitModelPart::OuterWind { phase: 1 }),
        conduit_instance(ConduitModelPart::InnerWind { vertical: false }),
        conduit_instance(ConduitModelPart::Eye { open: true }),
    ];
    let meshes = entity_model_textured_meshes(&instances, &atlas);

    assert_eq!(meshes.submissions.len(), 5);
    assert_eq!(
        meshes.submissions[0].render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(meshes.submissions[0].texture, CONDUIT_BASE_TEXTURE_REF);
    assert_eq!(meshes.submissions[1].texture, CONDUIT_CAGE_TEXTURE_REF);
    assert_eq!(
        meshes.submissions[2].texture,
        CONDUIT_WIND_VERTICAL_TEXTURE_REF
    );
    assert_eq!(meshes.submissions[3].texture, CONDUIT_WIND_TEXTURE_REF);
    assert_eq!(meshes.submissions[4].texture, CONDUIT_OPEN_EYE_TEXTURE_REF);
    assert_eq!(
        meshes.submissions[0].light,
        instances[0].render_state.shader_light()
    );
    assert_eq!(
        meshes.submissions[1].light,
        instances[1].render_state.shader_light()
    );

    // Shell uses culling `entitySolid`; the active parts use non-culling `entityCutout`.
    assert_eq!(meshes.cutout_cull.cutout_faces, 6);
    assert_eq!(meshes.cutout.cutout_faces, 24);
}
