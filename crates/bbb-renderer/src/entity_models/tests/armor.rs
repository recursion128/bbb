use super::*;

// Build an atlas covering the zombie base texture plus the iron equipment-asset textures (humanoid +
// leggings), enough to render an iron-clad zombie.
fn iron_armor_atlas() -> EntityModelTextureAtlasLayout {
    let mut refs: Vec<EntityModelTextureRef> = zombie_entity_texture_refs().to_vec();
    refs.push(ARMOR_IRON_HUMANOID_TEXTURE_REF);
    refs.push(ARMOR_IRON_LEGGINGS_TEXTURE_REF);
    let images: Vec<EntityModelTextureImage> = refs
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    build_entity_model_texture_atlas(&images).unwrap().0
}

#[test]
fn armor_slot_textures_match_vanilla_layer_types() {
    // Vanilla `HumanoidArmorLayer.usesInnerModel`: only the LEGS slot reads the `humanoid_leggings`
    // texture; the head / chest / feet slots read the `humanoid` texture.
    assert_eq!(
        armor_slot_texture(EntityArmorMaterial::Iron, HumanoidArmorSlot::Head),
        ARMOR_IRON_HUMANOID_TEXTURE_REF
    );
    assert_eq!(
        armor_slot_texture(EntityArmorMaterial::Iron, HumanoidArmorSlot::Chest),
        ARMOR_IRON_HUMANOID_TEXTURE_REF
    );
    assert_eq!(
        armor_slot_texture(EntityArmorMaterial::Iron, HumanoidArmorSlot::Feet),
        ARMOR_IRON_HUMANOID_TEXTURE_REF
    );
    assert_eq!(
        armor_slot_texture(EntityArmorMaterial::Iron, HumanoidArmorSlot::Legs),
        ARMOR_IRON_LEGGINGS_TEXTURE_REF
    );
    assert_eq!(
        armor_slot_texture(EntityArmorMaterial::Diamond, HumanoidArmorSlot::Legs),
        ARMOR_DIAMOND_LEGGINGS_TEXTURE_REF
    );
    // Every equipment texture is stitched into the shared atlas.
    assert!(entity_model_texture_refs().contains(&ARMOR_IRON_HUMANOID_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF));
}

#[test]
fn armor_slot_part_subsets_match_vanilla_retain_exact_parts() {
    // Vanilla `HumanoidModel.ADULT_ARMOR_PARTS_PER_SLOT`.
    assert_eq!(HumanoidArmorSlot::Head.part_names(), &["head"]);
    assert_eq!(
        HumanoidArmorSlot::Chest.part_names(),
        &["body", "right_arm", "left_arm"]
    );
    assert_eq!(
        HumanoidArmorSlot::Legs.part_names(),
        &["body", "right_leg", "left_leg"]
    );
    assert_eq!(
        HumanoidArmorSlot::Feet.part_names(),
        &["right_leg", "left_leg"]
    );
    assert!(HumanoidArmorSlot::Legs.uses_inner_model());
    assert!(!HumanoidArmorSlot::Chest.uses_inner_model());
}

#[test]
fn armored_zombie_emits_inflated_armor_pieces() {
    let atlas = iron_armor_atlas();

    // A bare zombie wears no armor: its cutout is just the body.
    let bare = entity_model_textured_meshes(
        &[EntityModelInstance::zombie(
            70,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    let bare_cutout = bare.cutout.vertices.len();

    // A fully iron-clad adult zombie adds the four armor pieces into the cutout pass: helmet
    // (head + hat = 2 cubes), chestplate (body + 2 arms = 3), leggings (body + 2 legs = 3), boots
    // (2 legs = 2) — 10 cubes → 240 vertices.
    let armored = entity_model_textured_meshes(
        &[
            EntityModelInstance::zombie(71, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_armor(Some(EntityArmorMaterial::Iron))
                .with_chest_armor(Some(EntityArmorMaterial::Iron))
                .with_legs_armor(Some(EntityArmorMaterial::Iron))
                .with_feet_armor(Some(EntityArmorMaterial::Iron)),
        ],
        &atlas,
    );
    assert_eq!(
        armored.cutout.vertices.len() - bare_cutout,
        240,
        "four iron armor pieces add 10 cubes of cutout geometry"
    );

    // The armor is inflated (`CubeDeformation 1.0` / `0.5`), so it floats just outside the body: the
    // armored cutout reaches wider in X than the bare body.
    let bare_x_max = bare
        .cutout
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::MIN, f32::max);
    let armored_x_max = armored
        .cutout
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::MIN, f32::max);
    assert!(
        armored_x_max > bare_x_max,
        "inflated armor extends beyond the body ({armored_x_max} vs {bare_x_max})"
    );
}

#[test]
fn single_armor_slot_emits_only_its_pieces() {
    let atlas = iron_armor_atlas();
    let bare = entity_model_textured_meshes(
        &[EntityModelInstance::zombie(
            72,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    // Just a helmet: head + hat = 2 cubes → 48 vertices.
    let helmet = entity_model_textured_meshes(
        &[
            EntityModelInstance::zombie(73, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_armor(Some(EntityArmorMaterial::Iron)),
        ],
        &atlas,
    );
    assert_eq!(
        helmet.cutout.vertices.len() - bare.cutout.vertices.len(),
        48
    );
}

#[test]
fn standard_humanoid_wearers_all_drape_armor() {
    let atlas = iron_armor_atlas();
    let full_iron = |instance: EntityModelInstance| {
        instance
            .with_head_armor(Some(EntityArmorMaterial::Iron))
            .with_chest_armor(Some(EntityArmorMaterial::Iron))
            .with_legs_armor(Some(EntityArmorMaterial::Iron))
            .with_feet_armor(Some(EntityArmorMaterial::Iron))
    };
    // Each standard-`HumanoidModel` armor wearer (zombie family, skeleton family, player) drapes the
    // same 10-cube / 240-vertex armor set; the armor delta is independent of the base body texture.
    let wearers = [
        EntityModelInstance::skeleton(80, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::zombie_variant(
            81,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            false,
        ),
        EntityModelInstance::zombie_variant(
            82,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Husk,
            false,
        ),
        EntityModelInstance::player(83, [0.0, 64.0, 0.0], 0.0, false),
    ];
    for instance in wearers {
        let bare = entity_model_textured_meshes(&[instance], &atlas);
        let armored = entity_model_textured_meshes(&[full_iron(instance)], &atlas);
        assert_eq!(
            armored.cutout.vertices.len() - bare.cutout.vertices.len(),
            240,
            "{:?} drapes the four armor pieces",
            instance.kind
        );
    }

    // A baby husk wears a distinct baby armor mesh (deferred), so it drapes nothing.
    let baby_husk = EntityModelInstance::zombie_variant(
        84,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Husk,
        true,
    );
    let bare = entity_model_textured_meshes(&[baby_husk], &atlas);
    let armored = entity_model_textured_meshes(&[full_iron(baby_husk)], &atlas);
    assert_eq!(bare.cutout.vertices.len(), armored.cutout.vertices.len());
}

#[test]
fn baby_zombie_armor_is_deferred() {
    let atlas = iron_armor_atlas();
    // The baby zombie wears a distinct baby armor mesh, deferred for now — its cutout is unchanged
    // whether or not armor is equipped.
    let bare = entity_model_textured_meshes(
        &[EntityModelInstance::zombie(74, [0.0, 64.0, 0.0], 0.0, true)],
        &atlas,
    );
    let armored = entity_model_textured_meshes(
        &[EntityModelInstance::zombie(75, [0.0, 64.0, 0.0], 0.0, true)
            .with_chest_armor(Some(EntityArmorMaterial::Iron))],
        &atlas,
    );
    assert_eq!(bare.cutout.vertices.len(), armored.cutout.vertices.len());
}
