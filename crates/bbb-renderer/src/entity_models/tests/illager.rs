use super::*;

#[test]
fn illager_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ILLAGER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -10.0, -4.0],
            size: [8.0, 10.0, 8.0],
            color: ILLAGER_ROBE,
        }
    );
    assert_eq!(
        ILLAGER_HAT[0],
        ModelCubeDesc {
            min: [-4.45, -10.45, -4.45],
            size: [8.9, 12.9, 8.9],
            color: ILLAGER_HAT_COLOR,
        }
    );
    assert_eq!(
        ILLAGER_BODY[1],
        ModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            color: ILLAGER_ROBE,
        }
    );

    assert_eq!(ILLAGER_SHARED_CROSSED_PARTS.len(), 5);
    assert_part_tree(
        &ILLAGER_SHARED_CROSSED_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HEAD.as_slice(),
        ILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_CHILDREN[0],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &ILLAGER_SHARED_CROSSED_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_BODY.as_slice(),
    );
    assert_part_tree(
        &ILLAGER_SHARED_CROSSED_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        ILLAGER_CROSSED_ARMS.as_slice(),
        ILLAGER_CROSSED_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_CROSSED_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_LEFT_SHOULDER.as_slice(),
    );

    assert_eq!(ILLAGER_SHARED_UNCROSSED_PARTS.len(), 6);
    assert_part(
        &ILLAGER_SHARED_UNCROSSED_PARTS[4],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ILLAGER_SHARED_UNCROSSED_PARTS[5],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_LEFT_ARM.as_slice(),
    );

    assert_part_tree(
        &ILLAGER_ILLUSIONER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HEAD.as_slice(),
        ILLAGER_HEAD_WITH_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_WITH_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HAT.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_WITH_HAT_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_NOSE.as_slice(),
    );
}

#[test]
fn illager_model_meshes_use_vanilla_scaled_body_layer_geometry() {
    let evoker = entity_model_mesh(&[EntityModelInstance::illager(
        46,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Evoker,
    )]);
    assert_eq!(evoker.opaque_faces, 54);
    assert_eq!(evoker.vertices.len(), 216);
    assert_eq!(evoker.indices.len(), 324);
    let (evoker_min, evoker_max) = mesh_extents(&evoker);
    assert_close3(evoker_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(evoker_max, [0.46875003, 65.993126, 0.3839772]);

    let illusioner = entity_model_mesh(&[EntityModelInstance::illager(
        68,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Illusioner,
    )]);
    assert_eq!(illusioner.opaque_faces, 60);
    assert_eq!(illusioner.vertices.len(), 240);
    assert_eq!(illusioner.indices.len(), 360);
    let (illusioner_min, illusioner_max) = mesh_extents(&illusioner);
    assert_close3(illusioner_min, [-0.46875, 64.00094, -0.26074222]);
    assert_close3(illusioner_max, [0.46875003, 66.01949, 0.3839772]);

    let pillager = entity_model_mesh(&[EntityModelInstance::illager(
        103,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Pillager,
    )]);
    assert_eq!(pillager.opaque_faces, 48);
    assert_eq!(pillager.vertices.len(), 192);
    assert_eq!(pillager.indices.len(), 288);
    let (pillager_min, pillager_max) = mesh_extents(&pillager);
    assert_close3(pillager_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(pillager_max, [0.46875, 65.993126, 0.3515625]);

    let vindicator = entity_model_mesh(&[EntityModelInstance::illager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Vindicator,
    )]);
    assert_eq!(vindicator.vertices, evoker.vertices);
    assert_eq!(vindicator.indices, evoker.indices);
}

#[test]
fn illager_texture_refs_match_vanilla_renderers() {
    let cases = [
        (
            IllagerModelFamily::Evoker,
            "evoker",
            EntityModelTextureRef {
                path: "textures/entity/illager/evoker.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Illusioner,
            "illusioner",
            EntityModelTextureRef {
                path: "textures/entity/illager/illusioner.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Pillager,
            "pillager",
            EntityModelTextureRef {
                path: "textures/entity/illager/pillager.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Vindicator,
            "vindicator",
            EntityModelTextureRef {
                path: "textures/entity/illager/vindicator.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, model_key, texture) in cases {
        let kind = EntityModelKind::Illager { family };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}
