use super::*;

#[test]
fn iron_golem_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        IRON_GOLEM_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -12.0, -5.5],
                size: [8.0, 10.0, 8.0],
                color: IRON_GOLEM_STONE,
            },
            ModelCubeDesc {
                min: [-1.0, -5.0, -7.5],
                size: [2.0, 4.0, 2.0],
                color: IRON_GOLEM_STONE,
            },
        ]
    );
    assert_eq!(
        IRON_GOLEM_BODY,
        [
            ModelCubeDesc {
                min: [-9.0, -2.0, -6.0],
                size: [18.0, 12.0, 11.0],
                color: IRON_GOLEM_STONE,
            },
            ModelCubeDesc {
                min: [-5.0, 9.5, -3.5],
                size: [10.0, 6.0, 7.0],
                color: IRON_GOLEM_STONE,
            },
        ]
    );
    assert_eq!(
        IRON_GOLEM_RIGHT_ARM[0],
        ModelCubeDesc {
            min: [-13.0, -2.5, -3.0],
            size: [4.0, 30.0, 6.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(
        IRON_GOLEM_LEFT_ARM[0],
        ModelCubeDesc {
            min: [9.0, -2.5, -3.0],
            size: [4.0, 30.0, 6.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(
        IRON_GOLEM_RIGHT_LEG[0],
        ModelCubeDesc {
            min: [-3.5, -3.0, -3.0],
            size: [6.0, 16.0, 5.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(IRON_GOLEM_LEFT_LEG, IRON_GOLEM_RIGHT_LEG);

    assert_eq!(IRON_GOLEM_PARTS.len(), 6);
    let part_specs = [
        ([0.0, -7.0, -2.0], IRON_GOLEM_HEAD.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_BODY.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_RIGHT_ARM.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_LEFT_ARM.as_slice()),
        ([-4.0, 11.0, 0.0], IRON_GOLEM_RIGHT_LEG.as_slice()),
        ([5.0, 11.0, 0.0], IRON_GOLEM_LEFT_LEG.as_slice()),
    ];
    for (part, (offset, cubes)) in IRON_GOLEM_PARTS.iter().zip(part_specs) {
        assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
    }
}

#[test]
fn iron_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.8125, 64.001, -0.3125]);
    assert_close3(max, [0.8125, 66.6885, 0.59375]);
}

#[test]
fn iron_golem_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
    assert_eq!(
        EntityModelKind::IronGolem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/iron_golem/iron_golem.png",
            size: [128, 128],
        })
    );
}

#[test]
fn snow_golem_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        SNOW_GOLEM_HEAD[0],
        ModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_ARM[0],
        ModelCubeDesc {
            min: [-0.5, 0.5, -0.5],
            size: [11.0, 1.0, 1.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_UPPER_BODY[0],
        ModelCubeDesc {
            min: [-4.5, -9.5, -4.5],
            size: [9.0, 9.0, 9.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_LOWER_BODY[0],
        ModelCubeDesc {
            min: [-5.5, -11.5, -5.5],
            size: [11.0, 11.0, 11.0],
            color: SNOW_GOLEM_WHITE,
        }
    );

    assert_eq!(SNOW_GOLEM_PARTS.len(), 5);
    let part_specs = [
        ([0.0, 4.0, 0.0], [0.0, 0.0, 0.0], SNOW_GOLEM_HEAD.as_slice()),
        ([5.0, 6.0, 1.0], [0.0, 0.0, 1.0], SNOW_GOLEM_ARM.as_slice()),
        (
            [-5.0, 6.0, -1.0],
            [0.0, std::f32::consts::PI, -1.0],
            SNOW_GOLEM_ARM.as_slice(),
        ),
        (
            [0.0, 13.0, 0.0],
            [0.0, 0.0, 0.0],
            SNOW_GOLEM_UPPER_BODY.as_slice(),
        ),
        (
            [0.0, 24.0, 0.0],
            [0.0, 0.0, 0.0],
            SNOW_GOLEM_LOWER_BODY.as_slice(),
        ),
    ];
    for (part, (offset, rotation, cubes)) in SNOW_GOLEM_PARTS.iter().zip(part_specs) {
        assert_part(part, offset, rotation, cubes);
    }
}

#[test]
fn snow_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.6407774, 64.03225, -0.34375]);
    assert_close3(max, [0.6407774, 65.71975, 0.34375]);
}

#[test]
fn snow_golem_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(
        EntityModelKind::SnowGolem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/snow_golem/snow_golem.png",
            size: [64, 64],
        })
    );
}
