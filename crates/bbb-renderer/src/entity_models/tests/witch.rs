use super::*;

#[test]
fn witch_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        WITCH_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -10.0, -4.0],
            size: [8.0, 10.0, 8.0],
            color: WITCH_ROBE,
        }
    );
    assert_eq!(
        WITCH_HAT_4[0],
        ModelCubeDesc {
            min: [-0.25, -0.25, -0.25],
            size: [1.5, 2.5, 1.5],
            color: WITCH_HAT_COLOR,
        }
    );
    assert_eq!(
        WITCH_MOLE[0],
        ModelCubeDesc {
            min: [0.25, 3.25, -6.5],
            size: [0.5, 0.5, 0.5],
            color: WITCH_ROBE,
        }
    );

    assert_eq!(WITCH_PARTS.len(), 5);
    assert_part_tree(
        &WITCH_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_HEAD.as_slice(),
        WITCH_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HEAD_CHILDREN[0],
        [-5.0, -10.03125, -5.0],
        [0.0, 0.0, 0.0],
        WITCH_HAT.as_slice(),
        WITCH_HAT_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HAT_CHILDREN[0],
        [1.75, -4.0, 2.0],
        [-0.05235988, 0.0, 0.02617994],
        WITCH_HAT_2.as_slice(),
        WITCH_HAT_2_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HAT_2_CHILDREN[0],
        [1.75, -4.0, 2.0],
        [-0.10471976, 0.0, 0.05235988],
        WITCH_HAT_3.as_slice(),
        WITCH_HAT_3_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_HAT_3_CHILDREN[0],
        [1.75, -2.0, 2.0],
        [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
        WITCH_HAT_4.as_slice(),
    );
    assert_part_tree(
        &WITCH_HEAD_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_NOSE.as_slice(),
        WITCH_NOSE_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_NOSE_CHILDREN[0],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_MOLE.as_slice(),
    );
    assert_part_tree(
        &WITCH_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_BODY.as_slice(),
        WITCH_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_JACKET.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        WITCH_ARMS.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[3],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_LEG.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[4],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_LEG.as_slice(),
    );
}

#[test]
fn witch_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 84);
    assert_eq!(mesh.vertices.len(), 336);
    assert_eq!(mesh.indices.len(), 504);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875, 64.00094, -0.29296878]);
    assert_close3(max, [0.46875003, 66.56483, 0.3839772]);
}

#[test]
fn witch_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Witch.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/witch/witch.png",
            size: [64, 128],
        })
    );
}
