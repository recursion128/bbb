use super::*;

use glam::Mat4;

use crate::entity_models::catalog::horse_markings_texture_ref;
use crate::entity_models::colored::{
    entity_model_root_transform, mesh_transformer_scaled_model_root_transform, HORSE_SCALE,
};

#[test]
fn horse_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_HORSE_BODY[0],
        ModelCubeDesc {
            min: [-5.05, -8.05, -17.05],
            size: [10.1, 10.1, 22.1],
            color: HORSE_BROWN,
        }
    );
    assert_eq!(
        ADULT_HORSE_EAR[0],
        ModelCubeDesc {
            min: [0.551, -12.999, 4.001],
            size: [1.998, 2.998, 0.998],
            color: HORSE_BROWN,
        }
    );
    assert_eq!(ADULT_HORSE_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_HORSE_PARTS[0],
        [0.0, 11.0, 5.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_BODY.as_slice(),
        ADULT_HORSE_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_BODY_CHILDREN[0],
        [0.0, -5.0, 2.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_TAIL.as_slice(),
    );
    assert_part_tree(
        &ADULT_HORSE_PARTS[1],
        [0.0, 4.0, -12.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_NECK.as_slice(),
        ADULT_HORSE_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_HORSE_HEAD_PARTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_HEAD.as_slice(),
        ADULT_HORSE_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_CHILDREN[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_PARTS_CHILDREN[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_MANE.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_PARTS_CHILDREN[2],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_UPPER_MOUTH.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[2],
        [4.0, 14.0, 7.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_LEFT_HIND_LEG.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[3],
        [-4.0, 14.0, 7.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_RIGHT_HIND_LEG.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[4],
        [4.0, 14.0, -10.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_LEFT_FRONT_LEG.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[5],
        [-4.0, 14.0, -10.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_RIGHT_FRONT_LEG.as_slice(),
    );

    assert_eq!(
        BABY_HORSE_HEAD[0],
        ModelCubeDesc {
            min: [-3.0, -3.9484, -6.705],
            size: [6.0, 4.0, 9.0],
            color: HORSE_BROWN,
        }
    );
    assert_eq!(BABY_HORSE_PARTS.len(), 6);
    assert_part_tree(
        &BABY_HORSE_PARTS[0],
        [0.0, 12.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_HORSE_BODY.as_slice(),
        BABY_HORSE_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_HORSE_BODY_CHILDREN[0],
        [0.0, -1.0, 7.0],
        [-0.7418, 0.0, 0.0],
        BABY_HORSE_TAIL.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &BABY_HORSE_PARTS[1],
            [2.4, 16.0, 5.4],
            BABY_HORSE_LEFT_HIND_LEG.as_slice(),
        ),
        (
            &BABY_HORSE_PARTS[2],
            [-2.4, 16.0, 5.4],
            BABY_HORSE_RIGHT_HIND_LEG.as_slice(),
        ),
        (
            &BABY_HORSE_PARTS[3],
            [2.4, 16.0, -5.4],
            BABY_HORSE_LEFT_FRONT_LEG.as_slice(),
        ),
        (
            &BABY_HORSE_PARTS[4],
            [-2.4, 16.0, -5.4],
            BABY_HORSE_RIGHT_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }
    assert_part_tree(
        &BABY_HORSE_PARTS[5],
        [0.0, 10.0, -6.0],
        [0.6109, 0.0, 0.0],
        BABY_HORSE_NECK.as_slice(),
        BABY_HORSE_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &BABY_HORSE_HEAD_PARTS_CHILDREN[0],
        [0.0, -6.0516, -0.2951],
        [0.0, 0.0, 0.0],
        BABY_HORSE_HEAD.as_slice(),
        BABY_HORSE_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_HORSE_HEAD_CHILDREN[0],
        [2.0, -4.2484, 1.9451],
        [0.0, 0.0, 0.2618],
        BABY_HORSE_LEFT_EAR.as_slice(),
    );
    assert_part(
        &BABY_HORSE_HEAD_CHILDREN[1],
        [-2.0, -4.2484, 1.645],
        [0.0, 0.0, -0.2618],
        BABY_HORSE_RIGHT_EAR.as_slice(),
    );
}

#[test]
fn horse_body_armor_geometry_matches_vanilla_26_1_layer() {
    // Vanilla `ModelLayers.HORSE_ARMOR` / `UNDEAD_HORSE_ARMOR` use
    // `AbstractEquineModel.createBodyMesh(CubeDeformation(0.1F))`. The body keeps its hardcoded
    // `0.05F` deformation, the neck has no deformation parameter, and the ears keep `-0.001F`.
    assert_eq!(ADULT_HORSE_ARMOR_PARTS_TEXTURED.len(), 6);
    assert_eq!(
        ADULT_HORSE_ARMOR_PARTS_TEXTURED[0].cubes,
        ADULT_HORSE_BODY_TEXTURED.as_slice()
    );
    assert_eq!(
        ADULT_HORSE_ARMOR_PARTS_TEXTURED[1].cubes,
        ADULT_HORSE_NECK_TEXTURED.as_slice()
    );
    assert_eq!(
        ADULT_HORSE_ARMOR_HEAD_CHILDREN_TEXTURED[0].cubes,
        ADULT_HORSE_EAR_TEXTURED.as_slice()
    );
    assert_eq!(
        ADULT_HORSE_ARMOR_HEAD_CHILDREN_TEXTURED[1].cubes,
        ADULT_HORSE_RIGHT_EAR_TEXTURED.as_slice()
    );

    assert_close3(ADULT_HORSE_ARMOR_HEAD_TEXTURED[0].min, [-3.1, -11.1, -2.1]);
    assert_close3(ADULT_HORSE_ARMOR_HEAD_TEXTURED[0].size, [6.2, 5.2, 7.2]);
    assert_eq!(ADULT_HORSE_ARMOR_HEAD_TEXTURED[0].uv_size, [6.0, 5.0, 7.0]);
    assert_close3(ADULT_HORSE_ARMOR_MANE_TEXTURED[0].min, [-1.1, -11.1, 4.91]);
    assert_close3(ADULT_HORSE_ARMOR_MANE_TEXTURED[0].size, [2.2, 16.2, 2.2]);
    assert_close3(
        ADULT_HORSE_ARMOR_UPPER_MOUTH_TEXTURED[0].min,
        [-2.1, -11.1, -7.1],
    );
    assert_close3(
        ADULT_HORSE_ARMOR_UPPER_MOUTH_TEXTURED[0].size,
        [4.2, 5.2, 5.2],
    );
    assert_close3(
        ADULT_HORSE_ARMOR_LEFT_HIND_LEG_TEXTURED[0].min,
        [-3.1, -1.11, -1.1],
    );
    assert_close3(
        ADULT_HORSE_ARMOR_LEFT_HIND_LEG_TEXTURED[0].size,
        [4.2, 11.2, 4.2],
    );
    assert_close3(ADULT_HORSE_ARMOR_TAIL_TEXTURED[0].min, [-1.6, -0.1, -0.1]);
    assert_close3(ADULT_HORSE_ARMOR_TAIL_TEXTURED[0].size, [3.2, 14.2, 4.2]);
    assert_eq!(ADULT_HORSE_ARMOR_TAIL_TEXTURED[0].uv_size, [3.0, 14.0, 4.0]);
}

#[test]
fn horse_meshes_use_vanilla_body_layer_geometry() {
    let adult = entity_model_mesh(&[EntityModelInstance::horse(
        150,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    let (adult_min, adult_max) = mesh_extents(&adult);
    assert_close3(adult_min, [-0.34718758, 64.001785, -1.200657]);
    assert_close3(adult_max, [0.34718758, 66.32189, 1.6198997]);

    let baby = entity_model_mesh(&[EntityModelInstance::horse(151, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_eq!(baby.indices.len(), 360);
    // Vanilla runs `setupAnim` every frame, so a standing baby horse's tail sits at the
    // overridden `getTailXRotOffset() + π/6 = −π/2 + π/6 = −1.0472`, not the layer's
    // `−0.7418`. The steeper tail tucks the tail box in (less reach back, less reach down),
    // shifting the back/forward extents from the un-posed layer box.
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert_close3(baby_min, [-0.25000003, 64.001, -0.7374399]);
    assert_close3(baby_max, [0.25000003, 65.636024, 1.0663916]);
}

#[test]
fn horse_texture_refs_match_vanilla_renderer_defaults() {
    assert_eq!(
        EntityModelKind::Horse {
            baby: false,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
        .model_key(),
        "horse"
    );
    assert_eq!(
        EntityModelKind::Horse {
            baby: false,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/horse/horse_white.png",
            size: [64, 64],
        })
    );
    // Each of the seven coats maps to its `horse_<color>(_baby).png`; the model key stays "horse".
    for (variant, adult_path, baby_path) in [
        (
            HorseColorVariant::Creamy,
            "textures/entity/horse/horse_creamy.png",
            "textures/entity/horse/horse_creamy_baby.png",
        ),
        (
            HorseColorVariant::Black,
            "textures/entity/horse/horse_black.png",
            "textures/entity/horse/horse_black_baby.png",
        ),
        (
            HorseColorVariant::DarkBrown,
            "textures/entity/horse/horse_darkbrown.png",
            "textures/entity/horse/horse_darkbrown_baby.png",
        ),
    ] {
        assert_eq!(
            EntityModelKind::Horse {
                baby: false,
                variant,
                markings: HorseMarkings::None
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: adult_path,
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Horse {
                baby: true,
                variant,
                markings: HorseMarkings::None
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: baby_path,
                size: [64, 64],
            })
        );
    }
    assert_eq!(
        EntityModelKind::Horse {
            baby: true,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
        .model_key(),
        "horse_baby"
    );
    assert_eq!(
        EntityModelKind::Horse {
            baby: true,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/horse/horse_white_baby.png",
            size: [64, 64],
        })
    );
}

#[test]
fn horse_body_armor_texture_layers_match_vanilla_equipment_assets() {
    let leather = horse_body_armor_texture_layers(EntityArmorMaterial::Leather).unwrap();
    assert_eq!(leather.len(), 2);
    assert_eq!(leather[0].texture, HORSE_BODY_LEATHER_TEXTURE_REF);
    assert!(leather[0].dyeable);
    assert_eq!(leather[1].texture, HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF);
    assert!(!leather[1].dyeable);

    for (material, texture) in [
        (EntityArmorMaterial::Copper, HORSE_BODY_COPPER_TEXTURE_REF),
        (EntityArmorMaterial::Iron, HORSE_BODY_IRON_TEXTURE_REF),
        (EntityArmorMaterial::Gold, HORSE_BODY_GOLD_TEXTURE_REF),
        (EntityArmorMaterial::Diamond, HORSE_BODY_DIAMOND_TEXTURE_REF),
        (
            EntityArmorMaterial::Netherite,
            HORSE_BODY_NETHERITE_TEXTURE_REF,
        ),
    ] {
        let layers = horse_body_armor_texture_layers(material).unwrap();
        assert_eq!(
            layers,
            &[HorseBodyArmorTextureLayer {
                texture,
                dyeable: false
            }]
        );
    }
    assert_eq!(
        horse_body_armor_texture_layers(EntityArmorMaterial::Chainmail),
        None
    );
    assert_eq!(
        horse_body_armor_texture_layers(EntityArmorMaterial::TurtleScute),
        None
    );
}

#[test]
fn donkey_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_DONKEY_CHEST[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 8.0, 3.0],
            color: DONKEY_GRAY,
        }
    );
    assert_eq!(
        ADULT_DONKEY_EAR[0],
        ModelCubeDesc {
            min: [-1.0, -7.0, 0.0],
            size: [2.0, 7.0, 1.0],
            color: DONKEY_GRAY,
        }
    );
    assert_eq!(ADULT_DONKEY_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_DONKEY_PARTS[0],
        [0.0, 11.0, 5.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_BODY.as_slice(),
        ADULT_HORSE_BODY_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_DONKEY_PARTS_WITH_CHEST[0],
        [0.0, 11.0, 5.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_BODY.as_slice(),
        ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[0],
        [0.0, -5.0, 2.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_TAIL.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[1],
        [6.0, -8.0, 0.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        ADULT_DONKEY_CHEST.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[2],
        [-6.0, -8.0, 0.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        ADULT_DONKEY_CHEST.as_slice(),
    );
    assert_part_tree(
        &ADULT_DONKEY_PARTS[1],
        [0.0, 4.0, -12.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_NECK.as_slice(),
        ADULT_DONKEY_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_DONKEY_HEAD_PARTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_HEAD.as_slice(),
        ADULT_DONKEY_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_HEAD_CHILDREN[0],
        [1.25, -10.0, 4.0],
        [0.2617994, 0.0, 0.2617994],
        ADULT_DONKEY_EAR.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_HEAD_CHILDREN[1],
        [-1.25, -10.0, 4.0],
        [0.2617994, 0.0, -0.2617994],
        ADULT_DONKEY_EAR.as_slice(),
    );

    assert_eq!(BABY_DONKEY_PARTS.len(), 1);
    assert_part_tree(
        &BABY_DONKEY_PARTS[0],
        [1.0, 14.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_DONKEY_BODY.as_slice(),
        BABY_DONKEY_BODY_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &BABY_DONKEY_BODY_CHILDREN[0],
        [0.0, -1.5, 6.5],
        [0.0, 0.0, 0.0],
        &[],
        BABY_DONKEY_TAIL_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_TAIL_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-0.7418, 0.0, 0.0],
        BABY_DONKEY_TAIL_R1.as_slice(),
    );
    for (part, expected_offset) in [
        (&BABY_DONKEY_BODY_CHILDREN[1], [2.25, 3.5, 5.25]),
        (&BABY_DONKEY_BODY_CHILDREN[2], [-2.4, 3.5, 5.4]),
        (&BABY_DONKEY_BODY_CHILDREN[3], [2.4, 3.5, -5.3]),
        (&BABY_DONKEY_BODY_CHILDREN[4], [-2.4, 3.5, -5.4]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_DONKEY_LEG.as_slice(),
        );
    }
    assert_part_tree(
        &BABY_DONKEY_BODY_CHILDREN[5],
        [0.0, -3.0, -5.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_DONKEY_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_PARTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.3927, 0.0, 0.0],
        BABY_DONKEY_NECK_R1.as_slice(),
    );
    assert_part_tree(
        &BABY_DONKEY_HEAD_PARTS_CHILDREN[1],
        [0.0, -5.0, -3.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_DONKEY_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_CHILDREN[0],
        [0.0, -1.0, 1.0],
        [0.3927, 0.0, 0.0],
        BABY_DONKEY_HEAD_R1.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_CHILDREN[1],
        [2.0, -3.5, -1.0],
        [0.48, 0.0, 0.48],
        BABY_DONKEY_EAR.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_CHILDREN[2],
        [-2.0, -3.5, -1.0],
        [0.48, 0.0, -0.48],
        BABY_DONKEY_EAR.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_BODY_CHILDREN[6],
        [-1.0, 10.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
    );
    assert_part(
        &BABY_DONKEY_BODY_CHILDREN[7],
        [-1.0, 10.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
    );
}

#[test]
fn donkey_meshes_use_vanilla_body_layer_geometry_and_chest_visibility() {
    let adult = entity_model_mesh(&[EntityModelInstance::donkey(
        160,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DONKEY_GRAY, 0.78)));

    let with_chest = entity_model_mesh(&[EntityModelInstance::donkey(
        161,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        true,
    )]);
    assert_eq!(with_chest.opaque_faces, 84);
    assert_eq!(with_chest.vertices.len(), 336);
    assert_eq!(with_chest.indices.len(), 504);

    let mule = entity_model_mesh(&[EntityModelInstance::donkey(
        162,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        false,
        false,
    )]);
    assert_eq!(mule.opaque_faces, 72);
    assert!(mule
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(MULE_BROWN, 0.78)));
    let (donkey_min, donkey_max) = mesh_extents(&adult);
    let (mule_min, mule_max) = mesh_extents(&mule);
    assert!(mule_max[1] > donkey_max[1]);
    assert!(mule_min[2] < donkey_min[2]);

    let baby_without_chest = entity_model_mesh(&[EntityModelInstance::donkey(
        163,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        false,
    )]);
    let baby_with_chest = entity_model_mesh(&[EntityModelInstance::donkey(
        164,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        true,
    )]);
    assert_eq!(baby_without_chest.opaque_faces, 60);
    assert_eq!(baby_without_chest.vertices.len(), 240);
    assert_eq!(baby_without_chest.indices.len(), 360);
    assert_same_geometry(&baby_with_chest, &baby_without_chest);
}

#[test]
fn donkey_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            DonkeyModelFamily::Donkey,
            false,
            "donkey",
            EntityModelTextureRef {
                path: "textures/entity/horse/donkey.png",
                size: [64, 64],
            },
        ),
        (
            DonkeyModelFamily::Donkey,
            true,
            "donkey_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/donkey_baby.png",
                size: [64, 64],
            },
        ),
        (
            DonkeyModelFamily::Mule,
            false,
            "mule",
            EntityModelTextureRef {
                path: "textures/entity/horse/mule.png",
                size: [64, 64],
            },
        ),
        (
            DonkeyModelFamily::Mule,
            true,
            "mule_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/mule_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Donkey {
            family,
            baby,
            has_chest: true,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn undead_horse_meshes_use_unscaled_vanilla_horse_layers() {
    let skeleton_adult = entity_model_mesh(&[EntityModelInstance::undead_horse(
        170,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    )]);
    assert_eq!(skeleton_adult.opaque_faces, 72);
    assert_eq!(skeleton_adult.vertices.len(), 288);
    assert_eq!(skeleton_adult.indices.len(), 432);
    assert!(skeleton_adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SKELETON_HORSE_BONE, 0.78)));
    let (skeleton_min, skeleton_max) = mesh_extents(&skeleton_adult);
    assert_close3(skeleton_min, [-0.31562507, 64.001625, -1.0915062]);
    assert_close3(skeleton_max, [0.31562507, 66.11081, 1.4726361]);

    let base_horse_adult = entity_model_mesh(&[EntityModelInstance::horse(
        171,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let (horse_min, horse_max) = mesh_extents(&base_horse_adult);
    assert!(horse_max[1] > skeleton_max[1]);
    assert!(horse_min[2] < skeleton_min[2]);

    let zombie_baby = entity_model_mesh(&[EntityModelInstance::undead_horse(
        172,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        true,
    )]);
    let base_horse_baby =
        entity_model_mesh(&[EntityModelInstance::horse(173, [0.0, 64.0, 0.0], 0.0, true)]);
    assert_eq!(zombie_baby.opaque_faces, 60);
    assert_same_geometry(&zombie_baby, &base_horse_baby);
    assert!(zombie_baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIE_HORSE_GREEN, 0.78)));
}

#[test]
fn undead_horse_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            UndeadHorseModelFamily::Skeleton,
            false,
            "skeleton_horse",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_skeleton.png",
                size: [64, 64],
            },
        ),
        (
            UndeadHorseModelFamily::Skeleton,
            true,
            "skeleton_horse_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_skeleton_baby.png",
                size: [64, 64],
            },
        ),
        (
            UndeadHorseModelFamily::Zombie,
            false,
            "zombie_horse",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_zombie.png",
                size: [64, 64],
            },
        ),
        (
            UndeadHorseModelFamily::Zombie,
            true,
            "zombie_horse_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_zombie_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::UndeadHorse { family, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn equine_swings_its_legs_when_walking() {
    // Vanilla `AbstractEquineModel.setupAnim` swings the four legs with the equine gait
    // (front amplitude 0.8, hind 0.5), applies the head look/bob to the neck, and lifts the
    // tail with the gait. A standing equine with a level head is inert (the adult tail rest
    // equals the layer pose); a walking one differs. Covers horse (adult + the re-parented
    // baby layout), donkey/mule (adult + with-chest), and the undead horses.
    for base in [
        EntityModelInstance::horse(150, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::horse(151, [0.0, 64.0, 0.0], 0.0, true),
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            false,
        ),
        EntityModelInstance::donkey(
            37,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            true,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            false,
            false,
        ),
        EntityModelInstance::undead_horse(
            116,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            false,
        ),
        EntityModelInstance::undead_horse(
            151,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Zombie,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(
            rest.vertices, still.vertices,
            "{:?} rest is inert",
            base.kind
        );

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices.len(),
            walking.vertices.len(),
            "{:?}",
            base.kind
        );
        assert_ne!(
            rest.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );
    }
}

#[test]
fn adult_equine_swings_its_legs_and_keeps_its_body_still() {
    // The adult horse/donkey/mule/undead-horse layers list the body first (its cube is
    // the first 24 vertices) and the four single-cube legs last (the final 96 vertices).
    // A walking adult equine swings those legs while the body cube stays put. (The neck
    // bobs too — checked by `adult_horse_turns_and_bobs_its_neck`; the re-parented baby
    // horse layout lists its head last, so these contiguous checks are adult-only.)
    for base in [
        EntityModelInstance::horse(150, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            false,
        ),
        EntityModelInstance::donkey(
            37,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            true,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            false,
            false,
        ),
        EntityModelInstance::undead_horse(
            116,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        let leg_start = rest.vertices.len() - 96;
        assert_eq!(
            rest.vertices[0..24],
            walking.vertices[0..24],
            "{:?} the body cube stays put",
            base.kind
        );
        assert_ne!(
            rest.vertices[leg_start..],
            walking.vertices[leg_start..],
            "{:?} the four legs swing",
            base.kind
        );
    }
}

#[test]
fn adult_horse_turns_and_bobs_its_neck() {
    // Adult horse layer (288 verts): the body and its tail child occupy blocks [0, 2) =
    // vertices [0, 48); the neck (`head_parts`) and its head/mane/upper_mouth/ear children
    // occupy blocks [2, 8) = vertices [48, 192); the four legs occupy blocks [8, 12) =
    // vertices [192, 288). The vanilla `AbstractEquineModel.setupAnim` head look turns and
    // tilts the neck subtree, and the walk bob also moves it, while neither touches the
    // body; the legs move only when walking.
    let base = EntityModelInstance::horse(160, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);

    // Standing, head yawed (30° clamps to the equine ±20° limit, still a turn): only the
    // neck subtree moves.
    let yawed = entity_model_mesh(&[base.with_head_look(30.0, 0.0)]);
    assert_eq!(
        rest.vertices[0..48],
        yawed.vertices[0..48],
        "body/tail stay put when looking"
    );
    assert_ne!(
        rest.vertices[48..192],
        yawed.vertices[48..192],
        "the neck turns"
    );
    assert_eq!(
        rest.vertices[192..288],
        yawed.vertices[192..288],
        "legs stay put when standing"
    );

    // Standing, head pitched: the neck tilts, the legs stay put.
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -25.0)]);
    assert_ne!(
        rest.vertices[48..192],
        pitched.vertices[48..192],
        "the neck tilts"
    );
    assert_eq!(
        rest.vertices[192..288],
        pitched.vertices[192..288],
        "legs stay put when standing"
    );

    // Walking with a level head: the body cube stays put (block 0 = vertices [0, 24)), but
    // the tail lifts with the gait (`tail.xRot += speed * 0.75`, block 1 = vertices
    // [24, 48)), the neck bobs (speed 1 > 0.2), and the legs swing.
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(
        rest.vertices[0..24],
        walking.vertices[0..24],
        "the body cube stays put when walking"
    );
    assert_ne!(
        rest.vertices[24..48],
        walking.vertices[24..48],
        "the tail lifts when walking"
    );
    assert_ne!(
        rest.vertices[48..192],
        walking.vertices[48..192],
        "the neck bobs when walking"
    );
    assert_ne!(
        rest.vertices[192..288],
        walking.vertices[192..288],
        "the legs swing when walking"
    );
}

#[test]
fn equine_head_look_pose_clamps_yaw_and_tilts_pitch() {
    use std::f32::consts::FRAC_PI_6;

    // ADULT_HORSE_PARTS[1] is the neck (`head_parts`); its rest xRot is the layer's π/6
    // tilt, onto which the look pitch (and walk bob) add.
    let base = ADULT_HORSE_PARTS[1].pose;
    assert!((base.rotation[0] - FRAC_PI_6).abs() < 1e-6);

    // Yaw clamps to ±20° then converts to radians; pitch adds onto the π/6 neck tilt.
    let look = equine_head_look_pose(base, 45.0, -25.0, 0.0, 0.0);
    assert!(
        (look.rotation[1] - 20.0_f32.to_radians()).abs() < 1e-6,
        "yaw clamps to +20: {}",
        look.rotation[1]
    );
    assert!((look.rotation[0] - (FRAC_PI_6 + (-25.0_f32).to_radians())).abs() < 1e-6);
    let look = equine_head_look_pose(base, -50.0, 0.0, 0.0, 0.0);
    assert!(
        (look.rotation[1] - (-20.0_f32).to_radians()).abs() < 1e-6,
        "yaw clamps to -20: {}",
        look.rotation[1]
    );
    // Within ±20° the yaw passes through unchanged.
    let look = equine_head_look_pose(base, 12.0, 0.0, 0.0, 0.0);
    assert!((look.rotation[1] - 12.0_f32.to_radians()).abs() < 1e-6);

    // The walk bob adds cos(pos * 0.8) * 0.15 * speed onto the pitch when speed > 0.2.
    let look = equine_head_look_pose(base, 0.0, 0.0, 0.0, 1.0);
    assert!(
        (look.rotation[0] - (FRAC_PI_6 + 0.15)).abs() < 1e-6,
        "bob at pos 0, speed 1: {}",
        look.rotation[0]
    );
    // A slow gait (speed <= 0.2) adds no bob.
    let look = equine_head_look_pose(base, 0.0, 0.0, 0.0, 0.2);
    assert!(
        (look.rotation[0] - FRAC_PI_6).abs() < 1e-6,
        "no bob at speed 0.2"
    );
    // A general (pos, speed) bob.
    let pos = 2.0_f32;
    let speed = 0.5_f32;
    let look = equine_head_look_pose(base, 0.0, 0.0, pos, speed);
    assert!((look.rotation[0] - (FRAC_PI_6 + (pos * 0.8).cos() * 0.15 * speed)).abs() < 1e-6);

    // zRot and offset are preserved.
    assert_eq!(look.rotation[2], base.rotation[2]);
    assert_eq!(look.offset, base.offset);
}

#[test]
fn equine_tail_swing_pose_matches_vanilla_formula() {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_6};

    // Vanilla `AbstractEquineModel.setupAnim` tail animation (default branch):
    //   tail.xRot = getTailXRotOffset() + π/6 + speed * 0.75
    //   tail.y   += speed * ageScale
    //   tail.z   += speed * 2 * ageScale
    // The adult horse tail rest pose carries the layer's π/6 xRot and offset [0, -5, 2].
    let base = ADULT_HORSE_BODY_CHILDREN[0].pose;
    assert_eq!(base.offset, [0.0, -5.0, 2.0]);
    assert!((base.rotation[0] - FRAC_PI_6).abs() < 1e-6);

    // Adult (offset 0, ageScale 1), standing: the pose equals the layer rest pose exactly.
    let rest = equine_tail_swing_pose(base, 0.0, 0.0, 1.0);
    assert_eq!(rest, base);

    // Adult, walking (speed 1): the tail lifts (+0.75 xRot) and shifts back/up.
    let walking = equine_tail_swing_pose(base, 0.0, 1.0, 1.0);
    assert!((walking.rotation[0] - (FRAC_PI_6 + 0.75)).abs() < 1e-6);
    assert!((walking.offset[1] - (-5.0 + 1.0)).abs() < 1e-6);
    assert!((walking.offset[2] - (2.0 + 2.0)).abs() < 1e-6);
    assert_eq!(walking.offset[0], base.offset[0]);
    assert_eq!(walking.rotation[1], base.rotation[1]);
    assert_eq!(walking.rotation[2], base.rotation[2]);

    // A general (offset, speed, ageScale) sample.
    let speed = 0.4_f32;
    let sample = equine_tail_swing_pose(base, -FRAC_PI_2, speed, 0.5);
    assert!((sample.rotation[0] - (-FRAC_PI_2 + FRAC_PI_6 + speed * 0.75)).abs() < 1e-6);
    assert!((sample.offset[1] - (-5.0 + speed * 0.5)).abs() < 1e-6);
    assert!((sample.offset[2] - (2.0 + speed * 2.0 * 0.5)).abs() < 1e-6);

    // Baby horse: getTailXRotOffset = −π/2 overrides the layer's −0.7418 rest angle even
    // when standing, and ageScale = 0.5 halves the walk translation.
    let baby_base = BABY_HORSE_BODY_CHILDREN[0].pose;
    assert!((baby_base.rotation[0] - (-0.7418)).abs() < 1e-4);
    let baby_rest = equine_tail_swing_pose(baby_base, -FRAC_PI_2, 0.0, 0.5);
    assert!(
        (baby_rest.rotation[0] - (-FRAC_PI_2 + FRAC_PI_6)).abs() < 1e-6,
        "baby tail rest overridden to −π/2 + π/6: {}",
        baby_rest.rotation[0]
    );
    assert_ne!(
        baby_rest.rotation[0], baby_base.rotation[0],
        "the override differs from the baked layer rest angle"
    );
    let baby_walking = equine_tail_swing_pose(baby_base, -FRAC_PI_2, 1.0, 0.5);
    assert!((baby_walking.rotation[0] - (-FRAC_PI_2 + FRAC_PI_6 + 0.75)).abs() < 1e-6);
    assert!((baby_walking.offset[1] - (baby_base.offset[1] + 0.5)).abs() < 1e-6);
    assert!((baby_walking.offset[2] - (baby_base.offset[2] + 1.0)).abs() < 1e-6);
}

#[test]
fn adult_equine_swings_its_tail_when_walking() {
    // Every adult equine layer lists the body cube first (block 0 = vertices [0, 24)) and
    // its tail child next (block 1 = vertices [24, 48)). A walking adult equine lifts the
    // tail (`tail.xRot += speed * 0.75`, plus a back/up shift) while the body cube stays
    // put. Covers the colored horse path and the uniform-color donkey/mule and undead-horse
    // paths (all share `emit_equine_posed`).
    for base in [
        EntityModelInstance::horse(150, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            false,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            false,
            false,
        ),
        EntityModelInstance::undead_horse(
            116,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices.len(),
            walking.vertices.len(),
            "{:?}",
            base.kind
        );
        assert_eq!(
            rest.vertices[0..24],
            walking.vertices[0..24],
            "{:?} the body cube stays put",
            base.kind
        );
        assert_ne!(
            rest.vertices[24..48],
            walking.vertices[24..48],
            "{:?} the tail lifts with the gait",
            base.kind
        );
    }
}

#[test]
fn baby_horse_swings_and_overrides_its_tail() {
    // `BabyHorseModel` inherits `AbstractEquineModel.setupAnim`, which both lifts the tail
    // with the gait and overrides its rest angle (`getTailXRotOffset() + π/6 = −1.0472`,
    // vs the layer's baked `−0.7418`). The baby body cube is block 0 ([0, 24)); the tail is
    // block 1 ([24, 48)). A walking baby horse lifts the tail while its body cube stays put;
    // the overridden standing rest angle is checked by `horse_meshes_use_vanilla_body_layer_geometry`
    // and `equine_tail_swing_pose_matches_vanilla_formula`. Covers the baby skeleton horse too.
    for base in [
        EntityModelInstance::horse(151, [0.0, 64.0, 0.0], 0.0, true),
        EntityModelInstance::undead_horse(
            152,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            true,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices[0..24],
            walking.vertices[0..24],
            "{:?} the baby body cube stays put",
            base.kind
        );
        assert_ne!(
            rest.vertices[24..48],
            walking.vertices[24..48],
            "{:?} the baby tail lifts when walking",
            base.kind
        );
    }
}

#[test]
fn baby_donkey_leg_swing_is_deferred() {
    // The baby donkey/mule layer re-parents its legs under the body
    // (`BabyDonkeyModel.createBabyLayer`) and overrides `setupAnim` (forcing `xRot = -30°`),
    // unlike the top-level adult layout, so its leg swing, head look, and tail lift are all
    // deferred: a walking baby donkey is unchanged for now.
    for base in [
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            true,
            false,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            true,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices, walking.vertices,
            "{:?} baby leg swing deferred",
            base.kind
        );
    }
}

#[test]
fn equine_leg_swing_pose_matches_vanilla_gait() {
    // Vanilla AbstractEquineModel.setupAnim (non-standing, land): with legAnim =
    // cos(pos*0.6662 + π) * speed, leftHind = -0.5*legAnim, rightHind = +0.5*legAnim,
    // leftFront = +0.8*legAnim, rightFront = -0.8*legAnim. ADULT_HORSE_PARTS lists
    // left_hind [2], right_hind [3], left_front [4], right_front [5].
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let leg_anim = (pos * 0.6662 + std::f32::consts::PI).cos() * speed;
    let left_hind = equine_leg_swing_pose(ADULT_HORSE_PARTS[2].pose, pos, speed, false);
    let right_hind = equine_leg_swing_pose(ADULT_HORSE_PARTS[3].pose, pos, speed, false);
    let left_front = equine_leg_swing_pose(ADULT_HORSE_PARTS[4].pose, pos, speed, false);
    let right_front = equine_leg_swing_pose(ADULT_HORSE_PARTS[5].pose, pos, speed, false);
    assert!(
        (left_hind.rotation[0] - (-0.5 * leg_anim)).abs() < 1e-6,
        "left hind"
    );
    assert!(
        (right_hind.rotation[0] - (0.5 * leg_anim)).abs() < 1e-6,
        "right hind"
    );
    assert!(
        (left_front.rotation[0] - (0.8 * leg_anim)).abs() < 1e-6,
        "left front"
    );
    assert!(
        (right_front.rotation[0] - (-0.8 * leg_anim)).abs() < 1e-6,
        "right front"
    );

    // Only xRot changes; offset and yRot/zRot are preserved.
    for (posed, index) in [(left_hind, 2), (right_front, 5)] {
        let base = ADULT_HORSE_PARTS[index].pose;
        assert_eq!(posed.offset, base.offset);
        assert_eq!(posed.rotation[1], base.rotation[1]);
        assert_eq!(posed.rotation[2], base.rotation[2]);
    }

    // At rest (speed 0) every leg holds its body-layer pose.
    assert_eq!(
        equine_leg_swing_pose(ADULT_HORSE_PARTS[4].pose, pos, 0.0, false),
        ADULT_HORSE_PARTS[4].pose
    );
}

#[test]
fn equine_leg_swing_pose_slows_the_paddle_in_water() {
    // Vanilla `AbstractEquineModel.setupAnim` scales the swing frequency by
    // `waterMultiplier = isInWater ? 0.2 : 1.0`: `legAnim = cos(waterMultiplier·pos·0.6662 + π)·speed`.
    // The same projected `walk_animation_pos`/`speed` therefore drive a slower in-water paddle.
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let water_leg_anim = (0.2 * pos * 0.6662 + std::f32::consts::PI).cos() * speed;

    let land_front = equine_leg_swing_pose(ADULT_HORSE_PARTS[4].pose, pos, speed, false);
    let water_front = equine_leg_swing_pose(ADULT_HORSE_PARTS[4].pose, pos, speed, true);
    assert!(
        (water_front.rotation[0] - (0.8 * water_leg_anim)).abs() < 1e-6,
        "in-water front leg uses the 0.2 frequency multiplier"
    );
    assert_ne!(
        land_front.rotation[0], water_front.rotation[0],
        "the in-water paddle differs from the land gait"
    );

    // At rest the multiplier is irrelevant — both hold the body-layer pose.
    assert_eq!(
        equine_leg_swing_pose(ADULT_HORSE_PARTS[4].pose, pos, 0.0, true),
        ADULT_HORSE_PARTS[4].pose
    );
}

fn undead_horse_texture_images() -> Vec<EntityModelTextureImage> {
    undead_horse_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn horse_texture_images() -> Vec<EntityModelTextureImage> {
    horse_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn horse_textured_mesh_matches_vanilla_horse_geometry() {
    // Vanilla `HorseRenderer` renders the shared `HorseModel` / `BabyHorseModel` geometry on the
    // textured path with a per-coat texture: 12 adult cubes (72 faces / 288 vertices, at the 1.1
    // `livingHorseScale`) and 10 baby cubes (60 faces / 240 vertices, unscaled). The textured body
    // occupies exactly the same space as the colored fallback (which applies the same scale), and the
    // coat variant only changes the sampled atlas region — positions stay identical.
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();

    let white_adult_instance = EntityModelInstance::horse(160, [0.0, 64.0, 0.0], 0.0, false);
    let white_adult_meshes = entity_model_textured_meshes(&[white_adult_instance], &atlas);
    assert_equine_submissions_match_vanilla(&white_adult_meshes, white_adult_instance);
    let white_adult = &white_adult_meshes.cutout;
    assert_eq!(white_adult.cutout_faces, 72);
    assert_eq!(white_adult.vertices.len(), 288);
    let colored_adult = entity_model_mesh(&[EntityModelInstance::horse(
        161,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let (tex_min, tex_max) = textured_mesh_extents(&white_adult);
    let (col_min, col_max) = mesh_extents(&colored_adult);
    assert_close3(tex_min, col_min);
    assert_close3(tex_max, col_max);

    // Coat variant → same geometry, different atlas sub-rect.
    let black_adult_instance = EntityModelInstance::horse_with_variant(
        162,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::Black,
    );
    let black_adult_meshes = entity_model_textured_meshes(&[black_adult_instance], &atlas);
    assert_equine_submissions_match_vanilla(&black_adult_meshes, black_adult_instance);
    let black_adult = &black_adult_meshes.cutout;
    let white_positions: Vec<_> = white_adult.vertices.iter().map(|v| v.position).collect();
    let black_positions: Vec<_> = black_adult.vertices.iter().map(|v| v.position).collect();
    assert_eq!(white_positions, black_positions);
    let white_uvs: Vec<_> = white_adult.vertices.iter().map(|v| v.uv).collect();
    let black_uvs: Vec<_> = black_adult.vertices.iter().map(|v| v.uv).collect();
    assert_ne!(white_uvs, black_uvs);

    // The unscaled baby layer occupies the same space as its colored baby fallback.
    let baby_instance = EntityModelInstance::horse_with_variant(
        163,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        HorseColorVariant::Gray,
    );
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_equine_submissions_match_vanilla(&baby_meshes, baby_instance);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.cutout_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    let colored_baby =
        entity_model_mesh(&[EntityModelInstance::horse(164, [0.0, 64.0, 0.0], 0.0, true)]);
    let (baby_min, baby_max) = textured_mesh_extents(&baby);
    let (colored_baby_min, colored_baby_max) = mesh_extents(&colored_baby);
    assert_close3(baby_min, colored_baby_min);
    assert_close3(baby_max, colored_baby_max);
}

#[test]
fn horse_colored_runtime_skips_the_texture_backed_horse() {
    // The living horse now carries vanilla coat UVs, so it renders through the textured path. The
    // texture-skipping colored runtime emits nothing for it, while the full colored path still emits
    // the brown fallback geometry.
    let instances = [
        EntityModelInstance::horse(165, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::horse(166, [4.0, 64.0, 0.0], 0.0, true),
    ];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn horse_textured_swings_legs_when_walking() {
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();
    let base = EntityModelInstance::horse(167, [0.0, 64.0, 0.0], 0.0, false);
    let still_instance = base.with_walk_animation(0.0, 0.0);
    let walking_instance = base.with_walk_animation(5.0, 1.0);
    let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_equine_submissions_match_vanilla(&still_meshes, still_instance);
    assert_equine_submissions_match_vanilla(&walking_meshes, walking_instance);
    let still = &still_meshes.cutout;
    let walking = &walking_meshes.cutout;
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking horse re-poses on the textured path"
    );
}

#[test]
fn horse_markings_overlay_layers_a_translucent_white_copy() {
    // Vanilla `HorseMarkingLayer` draws the white markings as a translucent overlay of the SAME posed
    // `HorseModel` on top of the base coat, but only when the coat carries markings (`Markings.NONE` →
    // `INVISIBLE_TEXTURE`, no overlay). The overlay rides the identical pose, so the body's cutout base
    // and the markings' translucent overlay have the same vertex count and positions.
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();

    // A plain (no-markings) horse: only the cutout base, no translucent overlay.
    let plain_instance = EntityModelInstance::horse_with_variant(
        170,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
    );
    let plain = entity_model_textured_meshes(&[plain_instance], &atlas);
    assert_equine_submissions_match_vanilla(&plain, plain_instance);
    assert_eq!(plain.cutout.vertices.len(), 288);
    assert!(plain.translucent.vertices.is_empty());
    assert_eq!(plain.submissions.len(), 1);
    assert_eq!(
        plain.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(plain.submissions[0].texture, HORSE_WHITE_TEXTURE_REF);
    assert_eq!(plain.submissions[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (
            plain.submissions[0].order,
            plain.submissions[0].submit_sequence
        ),
        (0, 0)
    );
    assert_eq!(
        plain.submissions[0].transform,
        mesh_transformer_scaled_model_root_transform(plain_instance, 1.1)
    );

    // A marked horse: the same cutout base PLUS a translucent overlay of identical geometry.
    let marked_instance = EntityModelInstance::horse_full(
        171,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
        HorseMarkings::WhiteDots,
    );
    let marked = entity_model_textured_meshes(&[marked_instance], &atlas);
    assert_equine_submissions_match_vanilla(&marked, marked_instance);
    assert_eq!(marked.cutout.vertices.len(), 288);
    assert_eq!(marked.translucent.cutout_faces, 72);
    assert_eq!(marked.translucent.vertices.len(), 288);
    assert_eq!(marked.submissions.len(), 2);
    let base_submit = marked.submissions[0];
    let markings_submit = marked.submissions[1];
    assert_eq!(
        base_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(base_submit.texture, HORSE_WHITE_TEXTURE_REF);
    assert_eq!(base_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((base_submit.order, base_submit.submit_sequence), (0, 0));
    assert_eq!(
        base_submit.transform,
        mesh_transformer_scaled_model_root_transform(marked_instance, 1.1)
    );
    assert_eq!(
        markings_submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(
        markings_submit.texture,
        HORSE_MARKINGS_WHITEDOTS_TEXTURE_REF
    );
    assert_eq!(markings_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (markings_submit.order, markings_submit.submit_sequence),
        (1, 1)
    );
    assert_eq!(markings_submit.transform, base_submit.transform);
    let base_positions: Vec<_> = marked.cutout.vertices.iter().map(|v| v.position).collect();
    let overlay_positions: Vec<_> = marked
        .translucent
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    assert_eq!(base_positions, overlay_positions);
    // The overlay samples the markings atlas region, not the coat's.
    let base_uvs: Vec<_> = marked.cutout.vertices.iter().map(|v| v.uv).collect();
    let overlay_uvs: Vec<_> = marked.translucent.vertices.iter().map(|v| v.uv).collect();
    assert_ne!(base_uvs, overlay_uvs);
}

#[test]
fn equine_saddle_model_parts_match_vanilla_layer_sources() {
    // Vanilla `EquineSaddleModel.createSaddleLayer()` adds the saddle body cube, bridle mouth/wrap
    // cubes, and two ridden-only zero-width line planes on top of the normal adult equine body mesh.
    assert_eq!(ADULT_EQUINE_SADDLE_BODY_TEXTURED[0].min, [-5.5, -8.5, -9.5]);
    assert_eq!(
        ADULT_EQUINE_SADDLE_BODY_TEXTURED[0].size,
        [11.0, 10.0, 10.0]
    );
    assert_eq!(
        ADULT_EQUINE_SADDLE_BODY_TEXTURED[0].uv_size,
        [10.0, 9.0, 9.0]
    );
    assert_eq!(ADULT_EQUINE_SADDLE_BODY_TEXTURED[0].tex, [26.0, 0.0]);
    assert_eq!(
        ADULT_EQUINE_SADDLE_HEAD_TEXTURED[0].min,
        [-3.22, -11.22, -2.12]
    );
    assert_eq!(
        ADULT_EQUINE_SADDLE_MOUTH_WRAP_TEXTURED[0].size,
        [4.4, 5.4, 2.4]
    );
    assert_eq!(
        ADULT_EQUINE_SADDLE_LEFT_LINE_TEXTURED[0].size,
        [0.0, 3.0, 16.0]
    );
    assert_eq!(
        ADULT_HORSE_SADDLE_RIDDEN_HEAD_PARTS_CHILDREN_TEXTURED[5]
            .pose
            .rotation,
        [-std::f32::consts::FRAC_PI_6, 0.0, 0.0]
    );
    assert_eq!(ADULT_HORSE_SADDLE_HEAD_PARTS_CHILDREN_TEXTURED.len(), 7);
    assert_eq!(
        ADULT_HORSE_SADDLE_RIDDEN_HEAD_PARTS_CHILDREN_TEXTURED.len(),
        9
    );
    // Donkey/mule saddle layers run the donkey mesh transformer after adding the saddle, so their
    // saddle tree keeps the two chest cubes even though `EquineSaddleModel` has no chest visibility
    // flag of its own.
    assert_eq!(ADULT_DONKEY_SADDLE_BODY_CHILDREN_TEXTURED.len(), 4);
}

#[test]
fn equine_saddle_layer_renders_for_adult_horses_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        HORSE_WHITE_TEXTURE_REF,
        HORSE_SADDLE_TEXTURE_REF,
        HORSE_WHITE_BABY_TEXTURE_REF,
    ]))
    .unwrap();

    let base = EntityModelInstance::horse(172, [0.0, 64.0, 0.0], 0.0, false);
    let bare_meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_equine_submissions_match_vanilla(&bare_meshes, base);
    let bare = &bare_meshes.cutout;
    let saddled_instance = base.with_equine_saddle(true);
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    assert_equine_submissions_match_vanilla(&saddled_meshes, saddled_instance);
    let saddled = &saddled_meshes.cutout;
    assert_eq!(saddled.cutout_faces - bare.cutout_faces, 102);
    assert_eq!(saddled.vertices.len() - bare.vertices.len(), 408);
    assert_eq!(saddled_meshes.submissions.len(), 2);
    assert_eq!(
        saddled_meshes.submissions[1].render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        saddled_meshes.submissions[1].texture,
        HORSE_SADDLE_TEXTURE_REF
    );
    assert_eq!(saddled_meshes.submissions[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (
            saddled_meshes.submissions[1].order,
            saddled_meshes.submissions[1].submit_sequence
        ),
        (2, 2)
    );
    assert_eq!(
        saddled_meshes.submissions[1].transform,
        saddled_meshes.submissions[0].transform
    );
    assert!(
        saddled.vertices[288].uv[1] >= 64.0 / 192.0,
        "the overlay samples the horse_saddle atlas region"
    );
    let (bare_min, bare_max) = textured_mesh_extents(&bare);
    let (saddle_min, saddle_max) = textured_mesh_extents(&saddled);
    assert!(saddle_min[0] < bare_min[0]);
    assert!(saddle_max[0] > bare_max[0]);

    let ridden_instance = base
        .with_equine_saddle(true)
        .with_equine_saddle_ridden(true);
    let ridden_meshes = entity_model_textured_meshes(&[ridden_instance], &atlas);
    assert_equine_submissions_match_vanilla(&ridden_meshes, ridden_instance);
    let ridden = &ridden_meshes.cutout;
    assert_eq!(ridden.cutout_faces - saddled.cutout_faces, 12);
    assert_eq!(ridden.vertices.len() - saddled.vertices.len(), 48);

    let baby_instance = EntityModelInstance::horse(173, [0.0, 64.0, 0.0], 0.0, true)
        .with_equine_saddle(true)
        .with_equine_saddle_ridden(true);
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_equine_submissions_match_vanilla(&baby_meshes, baby_instance);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.cutout_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
}

#[test]
fn horse_body_armor_layer_renders_for_adult_horse_and_zombie_horse_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        HORSE_WHITE_TEXTURE_REF,
        HORSE_WHITE_BABY_TEXTURE_REF,
        ZOMBIE_HORSE_TEXTURE_REF,
        SKELETON_HORSE_TEXTURE_REF,
        HORSE_BODY_IRON_TEXTURE_REF,
        HORSE_BODY_LEATHER_TEXTURE_REF,
        HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF,
        HORSE_BODY_NETHERITE_TEXTURE_REF,
    ]))
    .unwrap();

    let horse = EntityModelInstance::horse(182, [0.0, 64.0, 0.0], 0.0, false);
    let bare_meshes = entity_model_textured_meshes(&[horse], &atlas);
    assert_equine_submissions_match_vanilla(&bare_meshes, horse);
    let bare = &bare_meshes.cutout;
    let iron_meshes = entity_model_textured_meshes(
        &[horse.with_equine_body_armor(Some(EntityArmorMaterial::Iron))],
        &atlas,
    );
    assert_equine_submissions_match_vanilla(
        &iron_meshes,
        horse.with_equine_body_armor(Some(EntityArmorMaterial::Iron)),
    );
    let iron = &iron_meshes.cutout;
    assert_eq!(iron.cutout_faces - bare.cutout_faces, 72);
    assert_eq!(iron.vertices.len() - bare.vertices.len(), 288);
    assert_eq!(iron_meshes.submissions.len(), 2);
    assert_eq!(
        iron_meshes.submissions[1].render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        iron_meshes.submissions[1].texture,
        HORSE_BODY_IRON_TEXTURE_REF
    );
    assert_eq!(iron_meshes.submissions[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (
            iron_meshes.submissions[1].order,
            iron_meshes.submissions[1].submit_sequence
        ),
        (2, 2)
    );
    assert_eq!(
        iron_meshes.submissions[1].transform,
        iron_meshes.submissions[0].transform
    );
    let iron_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == HORSE_BODY_IRON_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_armor_vertex = iron.vertices[bare.vertices.len()].uv;
    assert!(first_armor_vertex[0] >= iron_uv.min[0]);
    assert!(first_armor_vertex[0] <= iron_uv.max[0]);
    assert!(first_armor_vertex[1] >= iron_uv.min[1]);
    assert!(first_armor_vertex[1] <= iron_uv.max[1]);

    let dye = 0x0033_66CC;
    let dyed_tint = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xCC as f32 / 255.0,
        1.0,
    ];
    let leather_meshes = entity_model_textured_meshes(
        &[horse
            .with_equine_body_armor(Some(EntityArmorMaterial::Leather))
            .with_equine_body_armor_dye(Some(dye))],
        &atlas,
    );
    assert_equine_submissions_match_vanilla(
        &leather_meshes,
        horse
            .with_equine_body_armor(Some(EntityArmorMaterial::Leather))
            .with_equine_body_armor_dye(Some(dye)),
    );
    let leather = &leather_meshes.cutout;
    assert_eq!(leather.cutout_faces - bare.cutout_faces, 144);
    assert_eq!(leather.vertices.len() - bare.vertices.len(), 576);
    assert_eq!(leather_meshes.submissions.len(), 3);
    assert_eq!(
        leather_meshes.submissions[1].render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        leather_meshes.submissions[1].texture,
        HORSE_BODY_LEATHER_TEXTURE_REF
    );
    assert_eq!(leather_meshes.submissions[1].tint, dyed_tint);
    assert_eq!(
        (
            leather_meshes.submissions[1].order,
            leather_meshes.submissions[1].submit_sequence
        ),
        (2, 2)
    );
    assert_eq!(
        leather_meshes.submissions[2].texture,
        HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF
    );
    assert_eq!(
        (
            leather_meshes.submissions[2].order,
            leather_meshes.submissions[2].submit_sequence
        ),
        (3, 3)
    );
    assert_eq!(leather_meshes.submissions[2].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        leather_meshes.submissions[2].transform,
        leather_meshes.submissions[0].transform
    );
    assert!(
        leather.vertices[bare.vertices.len()..bare.vertices.len() + 288]
            .iter()
            .all(|vertex| vertex.tint == dyed_tint)
    );
    assert!(leather.vertices[bare.vertices.len() + 288..]
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let baby = EntityModelInstance::horse(183, [0.0, 64.0, 0.0], 0.0, true);
    let baby_bare_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_equine_submissions_match_vanilla(&baby_bare_meshes, baby);
    let baby_bare = &baby_bare_meshes.cutout;
    let baby_armored_instance = baby.with_equine_body_armor(Some(EntityArmorMaterial::Iron));
    let baby_armored_meshes = entity_model_textured_meshes(&[baby_armored_instance], &atlas);
    assert_equine_submissions_match_vanilla(&baby_armored_meshes, baby_armored_instance);
    let baby_armored = &baby_armored_meshes.cutout;
    assert_eq!(baby_armored.vertices.len(), baby_bare.vertices.len());

    let zombie = EntityModelInstance::undead_horse(
        184,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        false,
    );
    let zombie_bare_meshes = entity_model_textured_meshes(&[zombie], &atlas);
    assert_equine_submissions_match_vanilla(&zombie_bare_meshes, zombie);
    let zombie_bare = &zombie_bare_meshes.cutout;
    let zombie_armored_instance =
        zombie.with_equine_body_armor(Some(EntityArmorMaterial::Netherite));
    let zombie_armored_meshes = entity_model_textured_meshes(&[zombie_armored_instance], &atlas);
    assert_equine_submissions_match_vanilla(&zombie_armored_meshes, zombie_armored_instance);
    let zombie_armored = &zombie_armored_meshes.cutout;
    assert_eq!(zombie_armored.cutout_faces - zombie_bare.cutout_faces, 72);
    assert_eq!(
        zombie_armored.vertices.len() - zombie_bare.vertices.len(),
        288
    );

    let skeleton = EntityModelInstance::undead_horse(
        185,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    );
    let skeleton_bare_meshes = entity_model_textured_meshes(&[skeleton], &atlas);
    assert_equine_submissions_match_vanilla(&skeleton_bare_meshes, skeleton);
    let skeleton_bare = &skeleton_bare_meshes.cutout;
    let skeleton_armored_instance =
        skeleton.with_equine_body_armor(Some(EntityArmorMaterial::Netherite));
    let skeleton_armored_meshes =
        entity_model_textured_meshes(&[skeleton_armored_instance], &atlas);
    assert_equine_submissions_match_vanilla(&skeleton_armored_meshes, skeleton_armored_instance);
    let skeleton_armored = &skeleton_armored_meshes.cutout;
    assert_eq!(
        skeleton_armored.vertices.len(),
        skeleton_bare.vertices.len(),
        "EntityTypeTags.CAN_WEAR_HORSE_ARMOR excludes skeleton horses"
    );

    let invalid_material_instance =
        horse.with_equine_body_armor(Some(EntityArmorMaterial::Chainmail));
    let invalid_material_meshes =
        entity_model_textured_meshes(&[invalid_material_instance], &atlas);
    assert_equine_submissions_match_vanilla(&invalid_material_meshes, invalid_material_instance);
    let invalid_material = &invalid_material_meshes.cutout;
    assert_eq!(
        invalid_material.vertices.len(),
        bare.vertices.len(),
        "vanilla 26.1 has no chainmail horse_body equipment texture"
    );
}

#[test]
fn equine_saddle_layer_uses_family_specific_models_and_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        DONKEY_TEXTURE_REF,
        DONKEY_SADDLE_TEXTURE_REF,
        MULE_TEXTURE_REF,
        MULE_SADDLE_TEXTURE_REF,
        SKELETON_HORSE_TEXTURE_REF,
        SKELETON_HORSE_SADDLE_TEXTURE_REF,
        ZOMBIE_HORSE_TEXTURE_REF,
        ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
    ]))
    .unwrap();

    let donkey_instance = EntityModelInstance::donkey(
        174,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        false,
    )
    .with_equine_saddle(true);
    let donkey_meshes = entity_model_textured_meshes(&[donkey_instance], &atlas);
    assert_equine_submissions_match_vanilla(&donkey_meshes, donkey_instance);
    let donkey = &donkey_meshes.cutout;
    assert_eq!(donkey.cutout_faces, 72 + 114);
    assert_eq!(donkey.vertices.len(), 288 + 456);
    assert_eq!(donkey_meshes.submissions.len(), 2);
    assert_eq!(
        donkey_meshes.submissions[1].render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        donkey_meshes.submissions[1].texture,
        DONKEY_SADDLE_TEXTURE_REF
    );
    assert_eq!(
        (
            donkey_meshes.submissions[1].order,
            donkey_meshes.submissions[1].submit_sequence
        ),
        (0, 1)
    );

    let mule_instance = EntityModelInstance::donkey(
        175,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        false,
        false,
    )
    .with_equine_saddle(true);
    let mule_meshes = entity_model_textured_meshes(&[mule_instance], &atlas);
    assert_equine_submissions_match_vanilla(&mule_meshes, mule_instance);
    let mule = &mule_meshes.cutout;
    assert_eq!(mule.cutout_faces, 72 + 114);
    let (_, donkey_max) = textured_mesh_extents(&donkey);
    let (_, mule_max) = textured_mesh_extents(&mule);
    assert!(
        mule_max[1] > donkey_max[1],
        "the mule saddle layer uses the larger 0.92 layer scale"
    );

    let skeleton_instance = EntityModelInstance::undead_horse(
        176,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    )
    .with_equine_saddle(true);
    let skeleton_meshes = entity_model_textured_meshes(&[skeleton_instance], &atlas);
    assert_equine_submissions_match_vanilla(&skeleton_meshes, skeleton_instance);
    let skeleton = &skeleton_meshes.cutout;
    let zombie_instance = EntityModelInstance::undead_horse(
        177,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        false,
    )
    .with_equine_saddle(true);
    let zombie_meshes = entity_model_textured_meshes(&[zombie_instance], &atlas);
    assert_equine_submissions_match_vanilla(&zombie_meshes, zombie_instance);
    let zombie = &zombie_meshes.cutout;
    assert_eq!(skeleton.cutout_faces, 72 + 102);
    assert_eq!(zombie.cutout_faces, 72 + 102);
    assert_eq!(zombie_meshes.submissions.len(), 2);
    assert_eq!(
        zombie_meshes.submissions[1].render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        zombie_meshes.submissions[1].texture,
        ZOMBIE_HORSE_SADDLE_TEXTURE_REF
    );
    assert_eq!(
        (
            zombie_meshes.submissions[1].order,
            zombie_meshes.submissions[1].submit_sequence
        ),
        (0, 1)
    );
    let skeleton_overlay_uv = skeleton.vertices[288].uv;
    let zombie_overlay_uv = zombie.vertices[288].uv;
    assert_ne!(
        skeleton_overlay_uv, zombie_overlay_uv,
        "skeleton and zombie horse saddles sample different equipment-layer atlas regions"
    );
}

fn donkey_texture_images() -> Vec<EntityModelTextureImage> {
    donkey_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn donkey_textured_mesh_matches_vanilla_adult_geometry() {
    // Vanilla `DonkeyModel` / `MuleModel` reuse `AbstractEquineModel.createBodyMesh` + `modifyMesh`
    // (donkey ears + side chests), so the adult donkey/mule render on the textured path: 12 cubes
    // (72 faces / 288 vertices) without chest, 14 (84 / 336) with. The textured body occupies the same
    // space as the colored fallback (same 0.87 donkey scale), and the mule shares the geometry at the
    // larger 0.92 scale. The baby donkey/mule stays colored (deferred), exercised separately.
    let (atlas, _) = build_entity_model_texture_atlas(&donkey_texture_images()).unwrap();

    let donkey_instance = EntityModelInstance::donkey(
        160,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        false,
    );
    let donkey_meshes = entity_model_textured_meshes(&[donkey_instance], &atlas);
    assert_equine_submissions_match_vanilla(&donkey_meshes, donkey_instance);
    let donkey = &donkey_meshes.cutout;
    assert_eq!(donkey.cutout_faces, 72);
    assert_eq!(donkey.vertices.len(), 288);
    let colored = entity_model_mesh(&[EntityModelInstance::donkey(
        161,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        false,
    )]);
    let (t_min, t_max) = textured_mesh_extents(&donkey);
    let (c_min, c_max) = mesh_extents(&colored);
    assert_close3(t_min, c_min);
    assert_close3(t_max, c_max);

    // The two side chests add 12 faces (2 boxes × 6).
    let with_chest_instance = EntityModelInstance::donkey(
        162,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        true,
    );
    let with_chest_meshes = entity_model_textured_meshes(&[with_chest_instance], &atlas);
    assert_equine_submissions_match_vanilla(&with_chest_meshes, with_chest_instance);
    let with_chest = &with_chest_meshes.cutout;
    assert_eq!(with_chest.cutout_faces, 84);
    assert_eq!(with_chest.vertices.len(), 336);

    // The mule shares the geometry at the larger 0.92 scale (vs donkey 0.87) and a different texture.
    let mule_instance = EntityModelInstance::donkey(
        163,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        false,
        false,
    );
    let mule_meshes = entity_model_textured_meshes(&[mule_instance], &atlas);
    assert_equine_submissions_match_vanilla(&mule_meshes, mule_instance);
    let mule = &mule_meshes.cutout;
    assert_eq!(mule.cutout_faces, 72);
    let (_, mule_max) = textured_mesh_extents(&mule);
    assert!(
        mule_max[1] > t_max[1],
        "the mule renders at the larger 0.92 scale"
    );
}

#[test]
fn donkey_textured_baby_matches_vanilla_baby_geometry() {
    // Vanilla `BabyDonkeyModel.createBabyLayer()` is a distinct re-parented mesh (10 cubes, 60 faces /
    // 240 vertices), emitted STATIC (its `setupAnim` forces `xRot = -30°`, so no equine posing). The
    // textured baby occupies the same space as its colored baby fallback (both unscaled), and the empty
    // chest children make `hasChest` immaterial.
    let (atlas, _) = build_entity_model_texture_atlas(&donkey_texture_images()).unwrap();
    let baby_instance = EntityModelInstance::donkey(
        165,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        false,
    );
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_equine_submissions_match_vanilla(&baby_meshes, baby_instance);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.cutout_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    let colored = entity_model_mesh(&[EntityModelInstance::donkey(
        166,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        false,
    )]);
    let (t_min, t_max) = textured_mesh_extents(&baby);
    let (c_min, c_max) = mesh_extents(&colored);
    assert_close3(t_min, c_min);
    assert_close3(t_max, c_max);

    // `hasChest` does not change the baby (its chest children are empty).
    let baby_chest_instance = EntityModelInstance::donkey(
        167,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        true,
        true,
    );
    let baby_chest_meshes = entity_model_textured_meshes(&[baby_chest_instance], &atlas);
    assert_equine_submissions_match_vanilla(&baby_chest_meshes, baby_chest_instance);
    let baby_chest = &baby_chest_meshes.cutout;
    assert_eq!(baby_chest.cutout_faces, 60);
}

#[test]
fn donkey_colored_runtime_skips_the_texture_backed_donkey() {
    // The donkey/mule (adult AND baby) now render through the textured path, so the runtime colored
    // mesh emits nothing for them; the full colored path still emits the fallback geometry.
    let instances = [
        EntityModelInstance::donkey(
            170,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            false,
        ),
        EntityModelInstance::donkey(
            171,
            [4.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            true,
            true,
        ),
    ];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn undead_horse_textured_mesh_matches_vanilla_horse_geometry() {
    // Vanilla `UndeadHorseRenderer extends HorseRenderer`, so the skeleton/zombie horses render the
    // shared `HorseModel` / `BabyHorseModel` geometry on the textured path: 12 adult cubes (72 faces /
    // 288 vertices) and 10 baby cubes (60 faces / 240 vertices). The textured body occupies exactly the
    // same space as the colored fallback — the adult extents match the colored mesh's vanilla-pinned
    // bounds (mirroring the left legs reorders vertices but keeps the bounding box) — so only the
    // texture, not a per-cube color, differs.
    let (atlas, _) = build_entity_model_texture_atlas(&undead_horse_texture_images()).unwrap();

    let skeleton_adult_instance = EntityModelInstance::undead_horse(
        170,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    );
    let skeleton_adult_meshes = entity_model_textured_meshes(&[skeleton_adult_instance], &atlas);
    assert_equine_submissions_match_vanilla(&skeleton_adult_meshes, skeleton_adult_instance);
    let skeleton_adult = &skeleton_adult_meshes.cutout;
    assert_eq!(skeleton_adult.cutout_faces, 72);
    assert_eq!(skeleton_adult.vertices.len(), 288);
    assert_eq!(skeleton_adult.indices.len(), 432);
    let (adult_min, adult_max) = textured_mesh_extents(&skeleton_adult);
    assert_close3(adult_min, [-0.31562507, 64.001625, -1.0915062]);
    assert_close3(adult_max, [0.31562507, 66.11081, 1.4726361]);

    // Same geometry, different family → identical vertex positions but a different atlas sub-rect
    // (proving the per-family texture is routed through the emit via `vanilla_texture_ref`).
    let zombie_adult_instance = EntityModelInstance::undead_horse(
        171,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        false,
    );
    let zombie_adult_meshes = entity_model_textured_meshes(&[zombie_adult_instance], &atlas);
    assert_equine_submissions_match_vanilla(&zombie_adult_meshes, zombie_adult_instance);
    let zombie_adult = &zombie_adult_meshes.cutout;
    let skeleton_positions: Vec<_> = skeleton_adult.vertices.iter().map(|v| v.position).collect();
    let zombie_positions: Vec<_> = zombie_adult.vertices.iter().map(|v| v.position).collect();
    assert_eq!(skeleton_positions, zombie_positions);
    let skeleton_uvs: Vec<_> = skeleton_adult.vertices.iter().map(|v| v.uv).collect();
    let zombie_uvs: Vec<_> = zombie_adult.vertices.iter().map(|v| v.uv).collect();
    assert_ne!(skeleton_uvs, zombie_uvs);

    // The baby re-parented layout (`BabyHorseModel.createBabyLayer`) renders on the textured path with
    // the same bounds as its colored fallback.
    let zombie_baby_instance = EntityModelInstance::undead_horse(
        172,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        true,
    );
    let zombie_baby_meshes = entity_model_textured_meshes(&[zombie_baby_instance], &atlas);
    assert_equine_submissions_match_vanilla(&zombie_baby_meshes, zombie_baby_instance);
    let zombie_baby = &zombie_baby_meshes.cutout;
    assert_eq!(zombie_baby.cutout_faces, 60);
    assert_eq!(zombie_baby.vertices.len(), 240);
    let colored_baby = entity_model_mesh(&[EntityModelInstance::undead_horse(
        173,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        true,
    )]);
    let (baby_min, baby_max) = textured_mesh_extents(&zombie_baby);
    let (colored_baby_min, colored_baby_max) = mesh_extents(&colored_baby);
    assert_close3(baby_min, colored_baby_min);
    assert_close3(baby_max, colored_baby_max);
}

#[test]
fn undead_horse_colored_runtime_skips_the_texture_backed_horse() {
    // The skeleton/zombie horse now carries vanilla texture UVs, so it renders through the textured
    // path. The texture-skipping colored runtime emits nothing for it (adult or baby), while the full
    // colored path still emits the fallback geometry.
    let instances = [
        EntityModelInstance::undead_horse(
            180,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            false,
        ),
        EntityModelInstance::undead_horse(
            181,
            [4.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Zombie,
            true,
        ),
    ];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn undead_horse_textured_swings_legs_when_walking() {
    // The undead horse reuses `HorseModel.setupAnim`, so the textured path swings the legs and lifts
    // the tail with the gait: a still horse matches the rest pose, a walking one differs.
    let (atlas, _) = build_entity_model_texture_atlas(&undead_horse_texture_images()).unwrap();
    let base = EntityModelInstance::undead_horse(
        190,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    );
    let still_instance = base.with_walk_animation(0.0, 0.0);
    let walking_instance = base.with_walk_animation(5.0, 1.0);
    let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_equine_submissions_match_vanilla(&still_meshes, still_instance);
    assert_equine_submissions_match_vanilla(&walking_meshes, walking_instance);
    let still = &still_meshes.cutout;
    let walking = &walking_meshes.cutout;
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking undead horse re-poses on the textured path"
    );
}

#[derive(Clone, Copy)]
struct ExpectedEquineSubmission {
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    tint: [f32; 4],
    transform: Mat4,
    order: i32,
    submit_sequence: u32,
}

fn assert_equine_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    let mut expected = Vec::new();
    match instance.kind {
        EntityModelKind::Horse { baby, markings, .. } => {
            let transform = if baby {
                entity_model_root_transform(instance)
            } else {
                mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE)
            };
            push_expected_equine_submit(
                &mut expected,
                EntityModelLayerRenderType::EntityCutout,
                instance.kind.vanilla_texture_ref().unwrap(),
                [1.0, 1.0, 1.0, 1.0],
                transform,
                0,
                0,
            );
            if let Some(texture) = horse_markings_texture_ref(markings, baby) {
                push_expected_equine_submit(
                    &mut expected,
                    EntityModelLayerRenderType::EntityTranslucent,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                    transform,
                    1,
                    1,
                );
            }
            let body_layer_count = if baby {
                0
            } else {
                push_expected_equine_body_armor_submits(&mut expected, instance, transform, 2, 2)
            };
            if instance.render_state.equine_saddle && !baby {
                push_expected_equine_submit(
                    &mut expected,
                    EntityModelLayerRenderType::ArmorCutoutNoCull,
                    HORSE_SADDLE_TEXTURE_REF,
                    [1.0, 1.0, 1.0, 1.0],
                    transform,
                    2,
                    2 + body_layer_count as u32,
                );
            }
        }
        EntityModelKind::Donkey { family, baby, .. } => {
            let transform = if baby {
                entity_model_root_transform(instance)
            } else {
                let scale = match family {
                    DonkeyModelFamily::Donkey => 0.87,
                    DonkeyModelFamily::Mule => 0.92,
                };
                mesh_transformer_scaled_model_root_transform(instance, scale)
            };
            push_expected_equine_submit(
                &mut expected,
                EntityModelLayerRenderType::EntityCutout,
                instance.kind.vanilla_texture_ref().unwrap(),
                [1.0, 1.0, 1.0, 1.0],
                transform,
                0,
                0,
            );
            if instance.render_state.equine_saddle && !baby {
                let texture = match family {
                    DonkeyModelFamily::Donkey => DONKEY_SADDLE_TEXTURE_REF,
                    DonkeyModelFamily::Mule => MULE_SADDLE_TEXTURE_REF,
                };
                push_expected_equine_submit(
                    &mut expected,
                    EntityModelLayerRenderType::ArmorCutoutNoCull,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                    transform,
                    0,
                    1,
                );
            }
        }
        EntityModelKind::UndeadHorse { family, baby } => {
            let transform = entity_model_root_transform(instance);
            push_expected_equine_submit(
                &mut expected,
                EntityModelLayerRenderType::EntityCutout,
                instance.kind.vanilla_texture_ref().unwrap(),
                [1.0, 1.0, 1.0, 1.0],
                transform,
                0,
                0,
            );
            let body_layer_count = if family == UndeadHorseModelFamily::Zombie && !baby {
                push_expected_equine_body_armor_submits(&mut expected, instance, transform, 0, 1)
            } else {
                0
            };
            if instance.render_state.equine_saddle && !baby {
                let texture = match family {
                    UndeadHorseModelFamily::Skeleton => SKELETON_HORSE_SADDLE_TEXTURE_REF,
                    UndeadHorseModelFamily::Zombie => ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
                };
                push_expected_equine_submit(
                    &mut expected,
                    EntityModelLayerRenderType::ArmorCutoutNoCull,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                    transform,
                    0,
                    1 + body_layer_count as u32,
                );
            }
        }
        _ => panic!("expected equine instance"),
    }

    assert_equine_folded_meshes_match_expected_buckets(meshes, &expected);
    assert_eq!(meshes.submissions.len(), expected.len());
    for (submit, expected) in meshes.submissions.iter().zip(expected.iter()) {
        assert_eq!(submit.render_type, expected.render_type);
        assert_eq!(
            submit.render_type.vanilla_name(),
            expected.render_type.vanilla_name()
        );
        assert_eq!(submit.texture, expected.texture);
        assert_eq!(submit.tint, expected.tint);
        assert_eq!(submit.transform, expected.transform);
        assert_eq!(
            (submit.order, submit.submit_sequence),
            (expected.order, expected.submit_sequence)
        );
    }
}

fn push_expected_equine_body_armor_submits(
    expected: &mut Vec<ExpectedEquineSubmission>,
    instance: EntityModelInstance,
    transform: Mat4,
    order: i32,
    first_submit_sequence: u32,
) -> usize {
    let Some(material) = instance.render_state.equine_body_armor else {
        return 0;
    };
    let Some(layers) = horse_body_armor_texture_layers(material) else {
        return 0;
    };
    for (layer_index, layer) in layers.iter().enumerate() {
        let tint = if layer.dyeable {
            expected_leather_horse_body_armor_tint(instance.render_state.equine_body_armor_dye)
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        push_expected_equine_submit(
            expected,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            layer.texture,
            tint,
            transform,
            order + layer_index as i32,
            first_submit_sequence + layer_index as u32,
        );
    }
    layers.len()
}

fn expected_leather_horse_body_armor_tint(dye: Option<u32>) -> [f32; 4] {
    let color = dye.unwrap_or(0x00A0_6540);
    [
        ((color >> 16) & 0xFF) as f32 / 255.0,
        ((color >> 8) & 0xFF) as f32 / 255.0,
        (color & 0xFF) as f32 / 255.0,
        1.0,
    ]
}

fn push_expected_equine_submit(
    expected: &mut Vec<ExpectedEquineSubmission>,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    tint: [f32; 4],
    transform: Mat4,
    order: i32,
    submit_sequence: u32,
) {
    expected.push(ExpectedEquineSubmission {
        render_type,
        texture,
        tint,
        transform,
        order,
        submit_sequence,
    });
}

fn assert_equine_folded_meshes_match_expected_buckets(
    meshes: &EntityModelTexturedMeshes,
    expected: &[ExpectedEquineSubmission],
) {
    assert!(!meshes.cutout.vertices.is_empty());
    if expected
        .iter()
        .any(|submit| submit.render_type == EntityModelLayerRenderType::EntityTranslucent)
    {
        assert!(!meshes.translucent.vertices.is_empty());
    } else {
        assert!(meshes.translucent.vertices.is_empty());
    }
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}
