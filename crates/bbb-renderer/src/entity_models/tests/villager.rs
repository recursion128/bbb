use super::*;

#[test]
fn villager_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_VILLAGER_HAT[0],
        ModelCubeDesc {
            min: [-4.51, -10.51, -4.51],
            size: [9.02, 11.02, 9.02],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(
        ADULT_VILLAGER_JACKET[0],
        ModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(ADULT_VILLAGER_PARTS.len(), 5);
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HEAD.as_slice(),
        ADULT_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HAT.as_slice(),
        ADULT_VILLAGER_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_NOSE.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_BODY.as_slice(),
        ADULT_VILLAGER_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_JACKET.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        ADULT_VILLAGER_ARMS.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[3],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[4],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
}

#[test]
fn villager_adult_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        139,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875003, 64.00094, -0.46875006]);
    assert_close3(max, [0.46875003, 66.02301, 0.46875003]);

    let wandering_trader_mesh = entity_model_mesh(&[EntityModelInstance::wandering_trader(
        141,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(wandering_trader_mesh.opaque_faces, mesh.opaque_faces);
    assert_eq!(wandering_trader_mesh.vertices, mesh.vertices);
    assert_eq!(wandering_trader_mesh.indices, mesh.indices);
}

#[test]
fn villager_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_VILLAGER_RIGHT_HAND,
        [
            ModelCubeDesc {
                min: [-1.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
            ModelCubeDesc {
                min: [5.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
        ]
    );
    assert_eq!(
        BABY_VILLAGER_BB_MAIN[0],
        ModelCubeDesc {
            min: [-2.7, -8.2, -1.7],
            size: [4.4, 6.4, 3.4],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(BABY_VILLAGER_PARTS.len(), 6);
    assert_part_tree(
        &BABY_VILLAGER_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_VILLAGER_ARMS_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[0],
        [-3.0, 1.4025, -0.9599],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_RIGHT_HAND.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[1],
        [0.0, 0.9024, -1.8175],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_MIDDLE_ARM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[1],
        [-1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[2],
        [1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part_tree(
        &BABY_VILLAGER_PARTS[3],
        [0.0, 16.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HEAD.as_slice(),
        BABY_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[0],
        [0.0, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -4.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[2],
        [0.0, -2.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[4],
        [0.0, 18.75, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BODY.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[5],
        [0.5, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BB_MAIN.as_slice(),
    );
}

#[test]
fn villager_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.43750003, 64.001, -0.37500003]);
    assert_close3(max, [0.43750003, 65.01975, 0.37500003]);
}

#[test]
fn villager_and_wandering_trader_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Villager { baby: false }.model_key(),
        "villager"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/wandering_trader/wandering_trader.png",
            size: [64, 64],
        })
    );
}
