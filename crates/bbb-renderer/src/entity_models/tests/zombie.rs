use super::*;

#[test]
fn zombie_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_ZOMBIE_HAT[0],
        ModelCubeDesc {
            min: [-4.5, -8.5, -4.5],
            size: [9.0, 9.0, 9.0],
            color: ZOMBIE_GREEN,
        }
    );
    assert_eq!(ADULT_ZOMBIE_PARTS.len(), 6);
    assert_eq!(ADULT_ZOMBIE_PARTS[0].pose, PART_POSE_ZERO);
    assert_eq!(ADULT_ZOMBIE_PARTS[0].cubes, ADULT_ZOMBIE_HEAD.as_slice());
    assert_eq!(
        ADULT_ZOMBIE_PARTS[0].children,
        ADULT_ZOMBIE_HEAD_CHILDREN.as_slice()
    );
    assert_part(
        &ADULT_ZOMBIE_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_HAT.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_BODY.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_LEFT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[4],
        [-1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_LEG.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[5],
        [1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_LEG.as_slice(),
    );
}

#[test]
fn zombie_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::zombie(
        54,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.5, 64.001, -0.28125]);
    assert_close3(max, [0.5, 66.03225, 0.28125]);
}

#[test]
fn zombie_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_ZOMBIE_HEAD,
        [
            ModelCubeDesc {
                min: [-3.0, -6.25, -3.0],
                size: [6.0, 6.0, 6.0],
                color: ZOMBIE_GREEN,
            },
            ModelCubeDesc {
                min: [-3.25, -6.4, -3.25],
                size: [6.5, 6.5, 6.5],
                color: ZOMBIE_GREEN,
            },
        ]
    );
    assert_eq!(BABY_ZOMBIE_PARTS.len(), 6);
    assert_part(
        &BABY_ZOMBIE_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_BODY.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[1],
        [0.0, 15.25, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_HEAD.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[2],
        [-3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[3],
        [3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[4],
        [-1.0, 20.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_LEG.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[5],
        [1.0, 20.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_LEG.as_slice(),
    );
}

#[test]
fn zombie_villager_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_ZOMBIE_VILLAGER_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -10.0, -4.0],
                size: [8.0, 10.0, 8.0],
                color: ZOMBIE_VILLAGER_ROBE,
            },
            ModelCubeDesc {
                min: [-1.0, -3.0, -6.0],
                size: [2.0, 4.0, 2.0],
                color: ZOMBIE_VILLAGER_ROBE,
            },
        ]
    );
    assert_eq!(
        ADULT_ZOMBIE_VILLAGER_BODY[1],
        ModelCubeDesc {
            min: [-4.05, -0.05, -3.05],
            size: [8.1, 20.1, 6.1],
            color: ZOMBIE_VILLAGER_ROBE,
        }
    );
    assert_eq!(ADULT_ZOMBIE_VILLAGER_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_ZOMBIE_VILLAGER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_HEAD.as_slice(),
        ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_HAT.as_slice(),
        ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_BODY.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_LEFT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[4],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[5],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_LEG.as_slice(),
    );

    assert_eq!(
        BABY_ZOMBIE_VILLAGER_BODY[1],
        ModelCubeDesc {
            min: [-2.1, -2.85, -1.6],
            size: [4.2, 6.2, 3.2],
            color: ZOMBIE_VILLAGER_ROBE,
        }
    );
    assert_eq!(BABY_ZOMBIE_VILLAGER_PARTS.len(), 6);
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[0],
        [0.0, 18.75, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_BODY.as_slice(),
    );
    assert_part_tree(
        &BABY_ZOMBIE_VILLAGER_PARTS[1],
        [0.0, 16.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_HEAD.as_slice(),
        BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[0],
        [0.0, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_HAT.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -4.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[2],
        [0.0, -1.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[2],
        [-3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[3],
        [3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[4],
        [-1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[5],
        [1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_LEG.as_slice(),
    );
}

#[test]
fn zombie_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::zombie(55, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.203125]);
    assert_close3(max, [0.25, 64.947876, 0.203125]);
}

#[test]
fn zombie_variant_meshes_use_vanilla_body_layer_geometry() {
    let zombie = entity_model_mesh(&[EntityModelInstance::zombie(
        150,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let baby_zombie = entity_model_mesh(&[EntityModelInstance::zombie(
        150,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    let husk = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        67,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Husk,
        false,
    )]);
    assert_eq!(husk.opaque_faces, 42);
    assert_eq!(husk.vertices.len(), 168);
    assert_eq!(husk.indices.len(), 252);
    assert!(husk
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(HUSK_TAN, 0.78)));
    let (husk_min, husk_max) = mesh_extents(&husk);
    assert_close3(husk_min, [-0.53125, 64.00106, -0.29882815]);
    assert_close3(husk_max, [0.53125, 66.15926, 0.29882815]);

    let baby_husk = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        67,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Husk,
        true,
    )]);
    assert_same_geometry(&baby_husk, &baby_zombie);
    assert!(baby_husk
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(HUSK_TAN, 0.78)));

    let drowned = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        38,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        false,
    )]);
    assert_same_geometry(&drowned, &zombie);
    assert!(drowned
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DROWNED_BLUE, 0.78)));

    let baby_drowned = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        38,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        true,
    )]);
    assert_same_geometry(&baby_drowned, &baby_zombie);
    assert!(baby_drowned
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DROWNED_BLUE, 0.78)));

    let zombie_villager = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        153,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::ZombieVillager,
        false,
    )]);
    assert_eq!(zombie_villager.opaque_faces, 60);
    assert_eq!(zombie_villager.vertices.len(), 240);
    assert_eq!(zombie_villager.indices.len(), 360);
    assert!(zombie_villager
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIE_VILLAGER_ROBE, 0.78)));
    let (zombie_villager_min, zombie_villager_max) = mesh_extents(&zombie_villager);
    assert_close3(zombie_villager_min, [-0.50000006, 64.001, -0.50000006]);
    assert_close3(zombie_villager_max, [0.50000006, 66.15725, 0.50000006]);

    let baby_zombie_villager = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        153,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::ZombieVillager,
        true,
    )]);
    assert_eq!(baby_zombie_villager.opaque_faces, 60);
    assert_eq!(baby_zombie_villager.vertices.len(), 240);
    assert_eq!(baby_zombie_villager.indices.len(), 360);
    assert!(baby_zombie_villager
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIE_VILLAGER_ROBE, 0.78)));
    let (baby_zombie_villager_min, baby_zombie_villager_max) = mesh_extents(&baby_zombie_villager);
    assert_close3(baby_zombie_villager_min, [-0.43750003, 64.001, -0.37500003]);
    assert_close3(baby_zombie_villager_max, [0.43750003, 65.01975, 0.37500003]);
}
