use super::*;

#[test]
fn spider_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        SPIDER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -4.0, -8.0],
            size: [8.0, 8.0, 8.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_BODY_0[0],
        ModelCubeDesc {
            min: [-3.0, -3.0, -3.0],
            size: [6.0, 6.0, 6.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_BODY_1[0],
        ModelCubeDesc {
            min: [-5.0, -4.0, -6.0],
            size: [10.0, 8.0, 12.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_RIGHT_LEG[0],
        ModelCubeDesc {
            min: [-15.0, -1.0, -1.0],
            size: [16.0, 2.0, 2.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_LEFT_LEG[0],
        ModelCubeDesc {
            min: [-1.0, -1.0, -1.0],
            size: [16.0, 2.0, 2.0],
            color: SPIDER_DARK,
        }
    );

    assert_eq!(SPIDER_PARTS.len(), 11);
    assert_part(
        &SPIDER_PARTS[0],
        [0.0, 15.0, -3.0],
        [0.0, 0.0, 0.0],
        SPIDER_HEAD.as_slice(),
    );
    assert_part(
        &SPIDER_PARTS[1],
        [0.0, 15.0, 0.0],
        [0.0, 0.0, 0.0],
        SPIDER_BODY_0.as_slice(),
    );
    assert_part(
        &SPIDER_PARTS[2],
        [0.0, 15.0, 9.0],
        [0.0, 0.0, 0.0],
        SPIDER_BODY_1.as_slice(),
    );

    let leg_specs = [
        (
            [-4.0, 15.0, 2.0],
            [
                0.0,
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, 2.0],
            [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_LEFT_LEG.as_slice(),
        ),
        (
            [-4.0, 15.0, 1.0],
            [0.0, std::f32::consts::FRAC_PI_8, -0.58119464],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, 1.0],
            [0.0, -std::f32::consts::FRAC_PI_8, 0.58119464],
            SPIDER_LEFT_LEG.as_slice(),
        ),
        (
            [-4.0, 15.0, 0.0],
            [0.0, -std::f32::consts::FRAC_PI_8, -0.58119464],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, 0.0],
            [0.0, std::f32::consts::FRAC_PI_8, 0.58119464],
            SPIDER_LEFT_LEG.as_slice(),
        ),
        (
            [-4.0, 15.0, -1.0],
            [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, -1.0],
            [
                0.0,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_LEFT_LEG.as_slice(),
        ),
    ];
    for (part, (offset, rotation, cubes)) in SPIDER_PARTS[3..].iter().zip(leg_specs) {
        assert_part(part, offset, rotation, cubes);
    }
}

#[test]
fn spider_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::spider(124, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-1.0282283, 64.0193, -0.9375]);
    assert_close3(max, [1.0282283, 64.8135, 0.7696068]);
}

#[test]
fn cave_spider_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cave_spider(22, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.71976, 64.01351, -0.65625]);
    assert_close3(max, [0.71976, 64.56945, 0.5387248]);
}

#[test]
fn spider_texture_refs_match_vanilla_renderers() {
    assert_eq!(EntityModelKind::Spider.model_key(), "spider");
    assert_eq!(
        EntityModelKind::Spider.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/spider/spider.png",
            size: [64, 32],
        })
    );
    assert_eq!(EntityModelKind::CaveSpider.model_key(), "cave_spider");
    assert_eq!(
        EntityModelKind::CaveSpider.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/spider/cave_spider.png",
            size: [64, 32],
        })
    );
}
