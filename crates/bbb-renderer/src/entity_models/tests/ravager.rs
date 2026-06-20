use super::*;

#[test]
fn ravager_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(RAVAGER_PARTS.len(), 6);
    assert_part_tree(
        &RAVAGER_PARTS[0],
        [0.0, -7.0, 5.5],
        [0.0, 0.0, 0.0],
        RAVAGER_NECK.as_slice(),
        RAVAGER_NECK_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &RAVAGER_NECK_CHILDREN[0],
        [0.0, 16.0, -17.0],
        [0.0, 0.0, 0.0],
        RAVAGER_HEAD.as_slice(),
        RAVAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[0],
        [-10.0, -14.0, -8.0],
        [1.0995574, 0.0, 0.0],
        RAVAGER_HORN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[1],
        [8.0, -14.0, -8.0],
        [1.0995574, 0.0, 0.0],
        RAVAGER_HORN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[2],
        [0.0, -2.0, 2.0],
        [0.0, 0.0, 0.0],
        RAVAGER_MOUTH.as_slice(),
    );
    assert_part(
        &RAVAGER_PARTS[1],
        [0.0, 1.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        RAVAGER_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &RAVAGER_PARTS[2],
            [-8.0, -13.0, 18.0],
            RAVAGER_HIND_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[3],
            [8.0, -13.0, 18.0],
            RAVAGER_HIND_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[4],
            [-8.0, -13.0, -5.0],
            RAVAGER_FRONT_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[5],
            [8.0, -13.0, -5.0],
            RAVAGER_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }
}

#[test]
fn ravager_mesh_uses_vanilla_body_layer_geometry() {
    let ravager = entity_model_mesh(&[EntityModelInstance::ravager(224, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(ravager.opaque_faces, 72);
    assert_eq!(ravager.vertices.len(), 288);
    assert_eq!(ravager.indices.len(), 432);
    assert!(ravager
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(RAVAGER_GRAY, 0.78)));

    let (min, max) = mesh_extents(&ravager);
    assert!(max[1] - min[1] > 2.0);
    assert!(max[2] - min[2] > 2.0);
}

#[test]
fn ravager_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Ravager;
    assert_eq!(kind.model_key(), "ravager");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/illager/ravager.png",
            size: [128, 128],
        })
    );
}
