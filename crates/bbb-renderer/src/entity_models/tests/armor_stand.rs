use super::*;

#[test]
fn armor_stand_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(ARMOR_STAND_PARTS.len(), 10);
    assert_part(
        &ARMOR_STAND_PARTS[0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_HEAD.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_BODY.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEFT_ARM.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[4],
        [-1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[5],
        [1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[6],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_RIGHT_BODY_STICK.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[7],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_LEFT_BODY_STICK.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[8],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_SHOULDER_STICK.as_slice(),
    );
    assert_part(
        &ARMOR_STAND_PARTS[9],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ARMOR_STAND_BASE_PLATE.as_slice(),
    );

    assert_eq!(SMALL_ARMOR_STAND_PARTS.len(), 10);
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[0],
        [0.0, 12.75, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_HEAD.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[1],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_BODY.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[2],
        [-2.5, 13.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[3],
        [2.5, 13.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEFT_ARM.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[4],
        [-0.95, 18.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[5],
        [0.95, 18.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEG.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[6],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_RIGHT_BODY_STICK.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[7],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_LEFT_BODY_STICK.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[8],
        [0.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_SHOULDER_STICK.as_slice(),
    );
    assert_part(
        &SMALL_ARMOR_STAND_PARTS[9],
        [0.0, 18.0, 0.0],
        [0.0, 0.0, 0.0],
        SMALL_ARMOR_STAND_BASE_PLATE.as_slice(),
    );
}

#[test]
fn armor_stand_mesh_uses_vanilla_visibility_and_pose_state() {
    let default = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )]);
    assert_eq!(default.opaque_faces, 48);
    assert_eq!(default.vertices.len(), 192);
    assert_eq!(default.indices.len(), 288);

    let arms_without_base = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )]);
    assert_eq!(arms_without_base.opaque_faces, 54);
    assert_eq!(arms_without_base.vertices.len(), 216);
    assert_eq!(arms_without_base.indices.len(), 324);

    let small = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )]);
    assert_eq!(small.opaque_faces, 48);
    assert_eq!(small.vertices.len(), 192);
    assert_eq!(small.indices.len(), 288);

    let mut pose = DEFAULT_ARMOR_STAND_MODEL_POSE;
    pose.head = [0.0, 45.0, 0.0];
    pose.body = [0.0, 0.0, 12.0];
    let posed = entity_model_mesh(&[EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        pose,
    )]);
    assert_eq!(posed.opaque_faces, default.opaque_faces);
    assert_ne!(posed.vertices, default.vertices);
}

#[test]
fn armor_stand_texture_refs_match_vanilla_renderer() {
    let adult = EntityModelKind::ArmorStand {
        small: false,
        show_arms: false,
        show_base_plate: true,
        pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
    };
    let small = EntityModelKind::ArmorStand {
        small: true,
        show_arms: false,
        show_base_plate: true,
        pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
    };

    assert_eq!(adult.model_key(), "armor_stand");
    assert_eq!(small.model_key(), "armor_stand_small");
    assert_eq!(
        adult.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/armorstand/armorstand.png",
            size: [64, 64],
        })
    );
    assert_eq!(small.vanilla_texture_ref(), adult.vanilla_texture_ref());
}
