use super::*;

#[test]
fn enderman_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ENDERMAN_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_HAT[0],
        ModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_BODY[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_ARM[0],
        ModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 30.0, 2.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_LEG[0],
        ModelCubeDesc {
            min: [-1.0, 0.0, -1.0],
            size: [2.0, 30.0, 2.0],
            color: ENDERMAN_DARK,
        }
    );

    assert_eq!(ENDERMAN_PARTS.len(), 6);
    assert_part_tree(
        &ENDERMAN_PARTS[0],
        [0.0, -13.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_HEAD.as_slice(),
        ENDERMAN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ENDERMAN_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_HAT.as_slice(),
    );
    assert_part(
        &ENDERMAN_PARTS[1],
        [0.0, -14.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_BODY.as_slice(),
    );

    let limb_specs = [
        ([-5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
        ([5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
        ([-2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
        ([2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
    ];
    for (part, (offset, cubes)) in ENDERMAN_PARTS[2..].iter().zip(limb_specs) {
        assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
    }
}

#[test]
fn enderman_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::enderman(141, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.375, 63.9385, -0.25]);
    assert_close3(max, [0.375, 66.8135, 0.25]);
}

#[test]
fn enderman_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
    assert_eq!(
        EntityModelKind::Enderman.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/enderman/enderman.png",
            size: [64, 32],
        })
    );
}
