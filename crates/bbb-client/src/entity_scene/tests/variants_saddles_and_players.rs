use super::*;

struct ReadyFarmAnimalPresentationLookup {
    presentation: FarmAnimalVariantPresentation,
    ready_asset_id: &'static str,
    texture: EntityDynamicTexture,
}

struct ReadyArmorTrimLookup {
    asset_id: &'static str,
    texture: EntityDynamicTexture,
    missing_texture: Option<EntityDynamicTexture>,
}

impl EntityVariantRegistryLookup for ReadyArmorTrimLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        if asset_id == self.asset_id {
            Some(self.texture)
        } else if asset_id == "minecraft:missingno" {
            self.missing_texture
        } else {
            None
        }
    }
}

#[test]
fn armor_trim_crosses_native_boundary_only_after_texture_is_ready() {
    let trim = WorldArmorTrimSprite {
        sprite_id: "example:trims/entity/humanoid/spire_opal".to_string(),
        decal: true,
    };
    let texture = EntityDynamicTexture { handle: 42_4300 };
    let ready = ReadyArmorTrimLookup {
        asset_id: "example:trims/entity/humanoid/spire_opal",
        texture,
        missing_texture: None,
    };
    let missing_texture = EntityDynamicTexture { handle: 42_4301 };
    let missing = ReadyArmorTrimLookup {
        asset_id: "example:some_other_sprite",
        texture,
        missing_texture: Some(missing_texture),
    };

    assert_eq!(armor_trim(Some(&trim), None), None);
    assert_eq!(
        armor_trim(Some(&trim), Some(&ready)),
        Some(EntityArmorTrim {
            texture,
            decal: true,
        })
    );
    assert_eq!(
        armor_trim(Some(&trim), Some(&missing)),
        Some(EntityArmorTrim {
            texture: missing_texture,
            decal: true,
        }),
        "vanilla's atlas lookup preserves the trim layer with the ready missing sprite"
    );
}

impl EntityVariantRegistryLookup for ReadyFarmAnimalPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn farm_animal_variant_presentation(
        &self,
        _registry: &str,
        _holder_id: i32,
    ) -> Option<Cow<'_, FarmAnimalVariantPresentation>> {
        Some(Cow::Borrowed(&self.presentation))
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        (asset_id == self.ready_asset_id).then_some(self.texture)
    }
}

#[test]
fn ready_farm_animal_presentation_switches_model_and_age_texture_atomically() {
    let texture = EntityDynamicTexture { handle: 42_4242 };
    let mut lookup = ReadyFarmAnimalPresentationLookup {
        presentation: FarmAnimalVariantPresentation {
            model: FarmAnimalVariantModel::Cold,
            adult_asset_id: "example:entity/cow/highland".to_string(),
            adult_texture_path: "example:textures/entity/cow/highland.png".to_string(),
            baby_asset_id: "example:entity/cow/highland_baby".to_string(),
            baby_texture_path: "example:textures/entity/cow/highland_baby.png".to_string(),
        },
        ready_asset_id: "example:entity/cow/highland",
        texture,
    };
    let values = [protocol_cow_variant_data(0)];
    let mut adult_kind = EntityModelKind::Cow {
        variant: CowModelVariant::Temperate,
        baby: false,
    };
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut adult_kind,
            VANILLA_ENTITY_TYPE_COW_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        adult_kind,
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: false,
        }
    );

    let mut baby_kind = EntityModelKind::Cow {
        variant: CowModelVariant::Warm,
        baby: true,
    };
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut baby_kind,
            VANILLA_ENTITY_TYPE_COW_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "an adult upload must not expose the baby presentation"
    );
    assert_eq!(
        baby_kind,
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: true,
        },
        "unready presentation must retain the complete built-in fallback"
    );

    lookup.ready_asset_id = "example:entity/cow/highland_baby";
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut baby_kind,
            VANILLA_ENTITY_TYPE_COW_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        baby_kind,
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true,
        }
    );
}

struct ReadyCatPresentationLookup {
    presentation: CatVariantPresentation,
    ready_asset_id: &'static str,
    texture: EntityDynamicTexture,
    revision: EntityDataRegistryRevision,
}

impl EntityVariantRegistryLookup for ReadyCatPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        self.revision
    }

    fn cat_variant_presentation(&self, _holder_id: i32) -> Option<Cow<'_, CatVariantPresentation>> {
        Some(Cow::Borrowed(&self.presentation))
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        (asset_id == self.ready_asset_id).then_some(self.texture)
    }
}

#[test]
fn ready_cat_presentation_selects_the_age_specific_network_texture() {
    let texture = EntityDynamicTexture { handle: 73_001 };
    let mut lookup = ReadyCatPresentationLookup {
        presentation: CatVariantPresentation {
            adult_asset_id: "example:entity/cat/lynx".to_string(),
            adult_texture_path: "example:textures/entity/cat/lynx.png".to_string(),
            baby_asset_id: "example:entity/cat/lynx_baby".to_string(),
            baby_texture_path: "example:textures/entity/cat/lynx_baby.png".to_string(),
        },
        ready_asset_id: "example:entity/cat/lynx",
        texture,
        revision: EntityDataRegistryRevision::default(),
    };
    let values = [protocol_cat_variant_data(0)];
    let mut adult_kind = EntityModelKind::Feline {
        cat: true,
        baby: false,
        cat_variant: CatModelVariant::Black,
        collar: None,
    };
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut adult_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );

    let mut baby_kind = EntityModelKind::Feline {
        cat: true,
        baby: true,
        cat_variant: CatModelVariant::Black,
        collar: None,
    };
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut baby_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "an adult upload must not expose the baby presentation"
    );
    lookup.ready_asset_id = "example:entity/cat/lynx_baby";
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut baby_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut baby_kind,
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "the cat holder never changes the ocelot texture"
    );

    lookup.revision.session = 1;
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut adult_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "a new registry generation must not reinterpret an old holder ordinal"
    );
}

struct ReadyWolfPresentationLookup {
    presentation: WolfVariantPresentation,
    ready_asset_id: &'static str,
    texture: EntityDynamicTexture,
}

impl EntityVariantRegistryLookup for ReadyWolfPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn wolf_variant_presentation(
        &self,
        _holder_id: i32,
    ) -> Option<Cow<'_, WolfVariantPresentation>> {
        Some(Cow::Borrowed(&self.presentation))
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        (asset_id == self.ready_asset_id).then_some(self.texture)
    }
}

#[test]
fn ready_wolf_presentation_selects_vanilla_age_and_state_texture() {
    let texture = EntityDynamicTexture { handle: 74_001 };
    let mut lookup = ReadyWolfPresentationLookup {
        presentation: WolfVariantPresentation {
            adult: wolf_test_asset_presentation("adult"),
            baby: wolf_test_asset_presentation("baby"),
        },
        ready_asset_id: "example:adult/wild",
        texture,
    };
    let values = [protocol_wolf_variant_data(0)];
    for (baby, tame, angry, expected_asset_id) in [
        (false, false, false, "example:adult/wild"),
        (false, true, false, "example:adult/tame"),
        (false, false, true, "example:adult/angry"),
        (true, false, false, "example:baby/wild"),
        (true, true, false, "example:baby/tame"),
        (true, false, true, "example:baby/angry"),
        (true, true, true, "example:baby/tame"),
    ] {
        lookup.ready_asset_id = expected_asset_id;
        let mut kind = EntityModelKind::Wolf {
            baby,
            tame,
            angry,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        };
        assert_eq!(
            apply_ready_entity_variant_presentation(
                &mut kind,
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &values,
                Some(&lookup),
            ),
            Some(texture),
            "wrong selection for baby={baby}, tame={tame}, angry={angry}"
        );
    }
}

fn wolf_test_asset_presentation(age: &str) -> WolfVariantAssetPresentation {
    WolfVariantAssetPresentation {
        wild_asset_id: format!("example:{age}/wild"),
        wild_texture_path: format!("example:textures/{age}/wild.png"),
        tame_asset_id: format!("example:{age}/tame"),
        tame_texture_path: format!("example:textures/{age}/tame.png"),
        angry_asset_id: format!("example:{age}/angry"),
        angry_texture_path: format!("example:textures/{age}/angry.png"),
    }
}

struct ReadyZombieNautilusPresentationLookup {
    ready: bool,
    texture: EntityDynamicTexture,
}

impl EntityVariantRegistryLookup for ReadyZombieNautilusPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn zombie_nautilus_variant_presentation(
        &self,
        _holder_id: i32,
    ) -> Option<Cow<'_, ZombieNautilusVariantPresentation>> {
        Some(Cow::Owned(ZombieNautilusVariantPresentation {
            model: ZombieNautilusVariantModelType::Warm,
            asset_id: "example:entity/nautilus/coral".to_string(),
            texture_path: "example:textures/entity/nautilus/coral.png".to_string(),
        }))
    }

    fn ready_texture(&self, _asset_id: &str) -> Option<EntityDynamicTexture> {
        self.ready.then_some(self.texture)
    }
}

#[test]
fn zombie_nautilus_network_model_waits_for_its_matching_texture() {
    let texture = EntityDynamicTexture { handle: 75_001 };
    let mut lookup = ReadyZombieNautilusPresentationLookup {
        ready: false,
        texture,
    };
    let values = [protocol_zombie_nautilus_variant_data(0)];
    let mut kind = EntityModelKind::ZombieNautilus {
        variant: ZombieNautilusModelVariant::Temperate,
    };

    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut kind,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &values,
            Some(&lookup),
        ),
        None
    );
    assert_eq!(
        kind,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate,
        },
        "unready custom texture must retain the complete built-in presentation"
    );

    lookup.ready = true;
    assert_eq!(
        apply_ready_entity_variant_presentation(
            &mut kind,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        kind,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm,
        }
    );
}

#[test]
fn entity_model_instances_project_pig_saddle_render_state() {
    const SADDLE_ITEM_ID: i32 = 740;
    let trim_texture = EntityDynamicTexture { handle: 42_4313 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/pig_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    // `minecraft:trim_material` and `minecraft:trim_pattern` are
                    // loaded from data files, so
                    // holder ids are sorted-resource-id indices.
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        25,
        &[(SADDLE_ITEM_ID, "saddle", "minecraft:saddle")],
    );
    world.apply_add_entity(protocol_add_entity(
        110,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [1.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(110)));
    let saddled = state(&world, 110);
    assert!(saddled.kind.pig().saddle_equipment_asset.is_some());
    assert!(saddled.kind.pig().saddle_foil);
    assert_eq!(
        saddled.kind.pig().saddle_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );
}

#[test]
fn entity_model_instances_project_snow_golem_pumpkin_render_state() {
    let (pumpkin_data_id, _pumpkin_default, _pumpkin_mask) = snow_golem_pumpkin_accessor();
    let pumpkin = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .snow_golem()
            .pumpkin
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        111,
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        112,
        VANILLA_ENTITY_TYPE_COW_ID,
        [3.0, 64.0, -3.0],
    ));

    assert!(
        pumpkin(&world, 111),
        "SnowGolem.DATA_PUMPKIN_ID defaults to the generated pumpkin mask"
    );
    assert!(!pumpkin(&world, 112));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 111,
        values: vec![EntityDataValue {
            data_id: pumpkin_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(0),
        }],
    }));
    assert!(!pumpkin(&world, 111));
}

#[test]
fn entity_model_instances_project_equine_saddle_and_ridden_render_state() {
    const SADDLE_ITEM_ID: i32 = 741;
    let trim_texture = EntityDynamicTexture { handle: 42_4313 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/horse_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let equine_saddle = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.kind.equine().saddle_equipment_asset.is_some(),
            render_state.kind.equine().saddle_ridden,
            render_state.kind.equine().saddle_trim,
            render_state.kind.equine().saddle_foil,
        )
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        26,
        &[(SADDLE_ITEM_ID, "saddle", "minecraft:saddle")],
    );
    world.apply_add_entity(protocol_add_entity(
        111,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        112,
        VANILLA_ENTITY_TYPE_COW_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(111)));
    assert_eq!(
        equine_saddle(&world, 111),
        (
            true,
            false,
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            true,
        )
    );

    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 111,
        passenger_ids: vec![112],
    }));
    assert_eq!(
        equine_saddle(&world, 111),
        (
            true,
            true,
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            true,
        )
    );
}

#[test]
fn entity_model_instances_project_strider_saddle_and_ridden_render_state() {
    const SADDLE_ITEM_ID: i32 = 742;
    let trim_texture = EntityDynamicTexture { handle: 42_4314 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/strider_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let strider_state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        27,
        &[(SADDLE_ITEM_ID, "saddle", "minecraft:saddle")],
    );
    world.apply_add_entity(protocol_add_entity(
        113,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        114,
        VANILLA_ENTITY_TYPE_COW_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(113)));
    let saddled = strider_state(&world, 113);
    assert!(saddled.kind.strider().saddle_equipment_asset.is_some());
    assert!(!saddled.kind.strider().ridden);
    assert!(saddled.kind.strider().saddle_foil);
    assert_eq!(
        saddled.kind.strider().saddle_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );

    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 113,
        passenger_ids: vec![114],
    }));
    let ridden = strider_state(&world, 113);
    assert!(ridden.kind.strider().saddle_equipment_asset.is_some());
    assert!(ridden.kind.strider().ridden);
    assert!(ridden.kind.strider().saddle_foil);
    assert_eq!(
        ridden.kind.strider().saddle_trim,
        saddled.kind.strider().saddle_trim
    );
}

#[test]
fn entity_model_instances_project_camel_saddle_and_ridden_render_state() {
    const SADDLE_ITEM_ID: i32 = 743;
    let camel_trim_texture = EntityDynamicTexture { handle: 42_4315 };
    let camel_trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/camel_saddle/sentry_iron",
        texture: camel_trim_texture,
        missing_texture: None,
    };
    let husk_trim_texture = EntityDynamicTexture { handle: 42_4316 };
    let husk_trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/camel_husk_saddle/sentry_iron",
        texture: husk_trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let camel_saddle = |world: &WorldStore, id: i32, lookup: &ReadyArmorTrimLookup| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        28,
        &[(SADDLE_ITEM_ID, "saddle", "minecraft:saddle")],
    );
    world.apply_add_entity(protocol_add_entity(
        115,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        116,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        117,
        VANILLA_ENTITY_TYPE_COW_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(115)));
    let saddled = camel_saddle(&world, 115, &camel_trim_lookup);
    assert!(saddled.kind.camel().saddle_equipment_asset.is_some());
    assert!(!saddled.kind.camel().saddle_ridden);
    assert!(saddled.kind.camel().saddle_foil);
    assert_eq!(
        saddled.kind.camel().saddle_trim,
        Some(EntityArmorTrim {
            texture: camel_trim_texture,
            decal: false,
        })
    );

    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 115,
        passenger_ids: vec![117],
    }));
    let ridden = camel_saddle(&world, 115, &camel_trim_lookup);
    assert!(ridden.kind.camel().saddle_equipment_asset.is_some());
    assert!(ridden.kind.camel().saddle_ridden);
    assert!(ridden.kind.camel().saddle_foil);
    assert_eq!(
        ridden.kind.camel().saddle_trim,
        saddled.kind.camel().saddle_trim
    );

    assert!(world.apply_set_equipment(saddle(116)));
    let husk = camel_saddle(&world, 116, &husk_trim_lookup);
    assert!(husk.kind.camel().saddle_equipment_asset.is_some());
    assert!(!husk.kind.camel().saddle_ridden);
    assert!(husk.kind.camel().saddle_foil);
    assert_eq!(
        husk.kind.camel().saddle_trim,
        Some(EntityArmorTrim {
            texture: husk_trim_texture,
            decal: false,
        })
    );
}

#[test]
fn entity_model_instances_project_nautilus_saddle_render_state() {
    const SADDLE_ITEM_ID: i32 = 744;
    let trim_texture = EntityDynamicTexture { handle: 42_4317 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/nautilus_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let nautilus_saddle = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        29,
        &[(SADDLE_ITEM_ID, "saddle", "minecraft:saddle")],
    );
    world.apply_add_entity(protocol_add_entity(
        118,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        119,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(118)));
    let living = nautilus_saddle(&world, 118);
    assert!(living.kind.nautilus().saddle_equipment_asset.is_some());
    assert!(living.kind.nautilus().saddle_foil);
    assert_eq!(
        living.kind.nautilus().saddle_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );

    assert!(world.apply_set_equipment(saddle(119)));
    let zombie = nautilus_saddle(&world, 119);
    assert!(zombie.kind.nautilus().saddle_equipment_asset.is_some());
    assert!(zombie.kind.nautilus().saddle_foil);
    assert_eq!(
        zombie.kind.nautilus().saddle_trim,
        living.kind.nautilus().saddle_trim
    );
}

#[test]
fn entity_model_instances_project_nautilus_body_armor_render_state() {
    const IRON_NAUTILUS_ARMOR_ITEM_ID: i32 = 747;
    const NETHERITE_NAUTILUS_ARMOR_ITEM_ID: i32 = 748;
    let trim_texture = EntityDynamicTexture { handle: 42_4308 };
    let missing_texture = EntityDynamicTexture { handle: 42_4307 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/nautilus_body/sentry_iron_darker",
        texture: trim_texture,
        missing_texture: Some(missing_texture),
    };

    let body_armor = |entity_id: i32, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let nautilus_body_armor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.kind.nautilus().body_equipment_asset,
            render_state.kind.nautilus().body_armor_trim,
            render_state.kind.nautilus().body_armor_foil,
        )
    };
    let nautilus_body_item_present = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
        .kind
        .nautilus()
        .body_item_present
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        30,
        &[
            (IRON_NAUTILUS_ARMOR_ITEM_ID, "body", "minecraft:iron"),
            (
                NETHERITE_NAUTILUS_ARMOR_ITEM_ID,
                "body",
                "minecraft:netherite",
            ),
        ],
    );
    world.apply_add_entity(protocol_add_entity(
        123,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        124,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        125,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 125,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_armor(123, IRON_NAUTILUS_ARMOR_ITEM_ID)));
    assert!(world.apply_set_equipment(body_armor(124, NETHERITE_NAUTILUS_ARMOR_ITEM_ID)));
    assert!(world.apply_set_equipment(body_armor(125, IRON_NAUTILUS_ARMOR_ITEM_ID)));
    assert!(nautilus_body_item_present(&world, 123));
    assert!(nautilus_body_item_present(&world, 124));
    assert!(
        nautilus_body_item_present(&world, 125),
        "native preserves the non-empty BODY stack independently of baby layer admission"
    );

    assert_eq!(
        nautilus_body_armor(&world, 123),
        (
            Some(EntityEquipmentAssetId::new(30, 0)),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            true,
        )
    );
    assert_eq!(
        nautilus_body_armor(&world, 124),
        (
            Some(EntityEquipmentAssetId::new(30, 1)),
            Some(EntityArmorTrim {
                texture: missing_texture,
                decal: false,
            }),
            true,
        ),
        "the absent netherite trim sprite keeps the layer through vanilla's missing sprite"
    );
    assert_eq!(
        nautilus_body_armor(&world, 125),
        (None, None, false),
        "baby living nautilus skip the body armor equipment layer"
    );
}

#[test]
fn entity_model_instances_project_horse_body_armor_render_state() {
    const LEATHER_HORSE_ARMOR_ITEM_ID: i32 = 749;
    const NETHERITE_HORSE_ARMOR_ITEM_ID: i32 = 750;
    const LEATHER_DYE: i32 = 0x0033_66CC;
    let trim_texture = EntityDynamicTexture { handle: 42_4309 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/horse_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let body_armor = |entity_id: i32, item_id: i32, dyed_color: Option<i32>| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    dyed_color,
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let horse_body_armor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.kind.equine().body_equipment_asset,
            render_state.kind.equine().body_armor_trim,
            render_state.kind.equine().body_armor_dye,
            render_state.kind.equine().body_armor_foil,
        )
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        31,
        &[
            (LEATHER_HORSE_ARMOR_ITEM_ID, "body", "minecraft:leather"),
            (NETHERITE_HORSE_ARMOR_ITEM_ID, "body", "minecraft:netherite"),
        ],
    );
    world.apply_add_entity(protocol_add_entity(
        126,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        127,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        128,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        [3.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        129,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        [4.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 129,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_HORSE_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_armor(
        126,
        LEATHER_HORSE_ARMOR_ITEM_ID,
        Some(LEATHER_DYE),
    )));
    assert!(world.apply_set_equipment(body_armor(127, NETHERITE_HORSE_ARMOR_ITEM_ID, None,)));
    assert!(world.apply_set_equipment(body_armor(128, NETHERITE_HORSE_ARMOR_ITEM_ID, None,)));
    assert!(world.apply_set_equipment(body_armor(
        129,
        LEATHER_HORSE_ARMOR_ITEM_ID,
        Some(LEATHER_DYE),
    )));

    assert_eq!(
        horse_body_armor(&world, 126),
        (
            Some(EntityEquipmentAssetId::new(31, 0)),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            Some(LEATHER_DYE as u32),
            true,
        )
    );
    assert_eq!(
        horse_body_armor(&world, 127),
        (
            Some(EntityEquipmentAssetId::new(31, 1)),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            None,
            true,
        )
    );
    assert_eq!(
        horse_body_armor(&world, 128),
        (None, None, None, false),
        "skeleton horses are not in vanilla CAN_WEAR_HORSE_ARMOR"
    );
    assert_eq!(
        horse_body_armor(&world, 129),
        (None, None, None, false),
        "baby horses skip the body armor equipment layer"
    );
}

#[test]
fn entity_model_instances_project_wolf_body_armor_render_state() {
    const WOLF_ARMOR_ITEM_ID: i32 = 751;
    const WOLF_ARMOR_DYE: i32 = 0x0033_66CC;
    let trim_texture = EntityDynamicTexture { handle: 42_4310 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/wolf_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let body_armor = |entity_id: i32, damage: i32, enchantment_glint_override: Option<bool>| {
        SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: ItemStackSummary {
                    item_id: Some(WOLF_ARMOR_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
                        dyed_color: Some(WOLF_ARMOR_DYE),
                        damage: Some(damage),
                        enchantments: vec![ItemEnchantmentSummary {
                            holder_id: 12,
                            level: 1,
                        }],
                        enchantment_glint_override,
                        armor_trim_material_id: Some(5), // iron in loader order
                        armor_trim_pattern_id: Some(8),  // sentry in loader order
                        ..Default::default()
                    }.into(),
                },
            }],
        }
    };
    let wolf_body_armor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.kind.wolf().body_equipment_asset,
            render_state.kind.wolf().body_armor_trim,
            render_state.kind.wolf().body_armor_dye,
            render_state.kind.wolf().body_armor_crackiness,
            render_state.kind.wolf().body_armor_foil,
        )
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        32,
        &[(WOLF_ARMOR_ITEM_ID, "body", "minecraft:armadillo_scute")],
    );
    world.set_default_item_component_codec_values(std::collections::BTreeMap::from([(
        WOLF_ARMOR_ITEM_ID,
        std::collections::BTreeMap::from([
            (
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                serde_json::json!({"slot": "body", "asset_id": "minecraft:armadillo_scute"}),
            ),
            (
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                serde_json::json!(1),
            ),
            (
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                serde_json::json!(64),
            ),
            (
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                serde_json::json!(0),
            ),
        ]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    world.apply_add_entity(protocol_add_entity(
        130,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        131,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        132,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 131,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_armor(130, 24, None)));
    assert!(world.apply_set_equipment(body_armor(131, 44, None)));
    assert!(world.apply_set_equipment(body_armor(132, 24, Some(false))));

    assert_eq!(
        wolf_body_armor(&world, 130),
        (
            Some(EntityEquipmentAssetId::new(32, 0)),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            Some(WOLF_ARMOR_DYE as u32),
            WolfArmorCrackiness::Medium,
            true
        )
    );
    assert_eq!(
        wolf_body_armor(&world, 132),
        (
            Some(EntityEquipmentAssetId::new(32, 0)),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            Some(WOLF_ARMOR_DYE as u32),
            WolfArmorCrackiness::Medium,
            false
        ),
        "enchantment_glint_override=false wins over non-empty enchantments"
    );
    assert_eq!(
        wolf_body_armor(&world, 131),
        (None, None, None, WolfArmorCrackiness::None, false),
        "baby wolves skip the adult-only WolfArmorLayer"
    );
}

#[test]
fn entity_model_instances_project_llama_body_decor_render_state() {
    const WHITE_CARPET_ITEM_ID: i32 = 745;
    const BLACK_CARPET_ITEM_ID: i32 = 746;
    let trim_texture = EntityDynamicTexture { handle: 42_4311 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/llama_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let body_item = |entity_id: i32, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5), // iron in loader order
                    armor_trim_pattern_id: Some(8),  // sentry in loader order
                    ..Default::default()
                }
                .into(),
            },
        }],
    };
    let llama_body_decor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.kind.llama().body_equipment_asset,
            render_state.kind.llama().body_decor_trim,
            render_state.kind.llama().body_decor_foil,
        )
    };

    let mut world = WorldStore::new();
    install_equipment_defaults(
        &mut world,
        33,
        &[
            (WHITE_CARPET_ITEM_ID, "body", "minecraft:white_carpet"),
            (BLACK_CARPET_ITEM_ID, "body", "minecraft:black_carpet"),
        ],
    );
    world.apply_add_entity(protocol_add_entity(
        120,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        121,
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        122,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 122,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_item(120, WHITE_CARPET_ITEM_ID)));
    assert!(world.apply_set_equipment(body_item(121, BLACK_CARPET_ITEM_ID)));
    assert!(world.apply_set_equipment(body_item(122, WHITE_CARPET_ITEM_ID)));

    let expected_trim = Some(EntityArmorTrim {
        texture: trim_texture,
        decal: false,
    });
    assert_eq!(
        llama_body_decor(&world, 120),
        (
            Some(EntityEquipmentAssetId::new(33, 0)),
            expected_trim,
            true
        )
    );
    assert_eq!(
        llama_body_decor(&world, 121),
        (
            Some(EntityEquipmentAssetId::new(33, 1)),
            expected_trim,
            true
        ),
        "an explicit carpet wins over the trader-llama empty-stack fallback"
    );
    assert_eq!(
        llama_body_decor(&world, 122),
        (None, None, false),
        "baby llamas ignore body items; renderer handles trader baby fallback separately"
    );
}

#[test]
fn entity_model_instances_project_happy_ghast_harness_and_ropes_render_state() {
    const WHITE_HARNESS_ITEM_ID: i32 = 747;
    let trim_texture = EntityDynamicTexture { handle: 42_4312 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/happy_ghast_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };
    let harness = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(WHITE_HARNESS_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(5),
                    armor_trim_pattern_id: Some(8),
                    ..Default::default()
                }
                .into(),
            },
        }],
    };

    let mut world = WorldStore::new();
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:harnesses".to_string(),
                entries: vec![WHITE_HARNESS_ITEM_ID],
            }],
        }],
    });
    install_equipment_defaults(
        &mut world,
        34,
        &[(WHITE_HARNESS_ITEM_ID, "body", "minecraft:white_harness")],
    );
    world.apply_add_entity(protocol_add_entity(
        126,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        128,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(harness(126)));
    let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 126,
        values: vec![EntityDataValue {
            data_id: accessors.is_leash_holder_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 126,
        passenger_ids: vec![128],
    }));

    let instances = entity_model_instances_from_world_with_registry_at_partial_tick(
        &world,
        None,
        0.0,
        Some(&trim_lookup),
    );
    {
        let (entity_id, baby, ridden) = (126, false, true);
        let instance = instances
            .iter()
            .find(|instance| instance.entity_id == entity_id)
            .unwrap();
        assert_eq!(instance.kind, EntityModelKind::HappyGhast { baby });
        assert!(instance.render_state.kind.happy_ghast().body_item);
        assert_eq!(
            instance
                .render_state
                .kind
                .happy_ghast()
                .body_equipment_asset,
            Some(EntityEquipmentAssetId::new(34, 0))
        );
        assert_eq!(
            instance.render_state.kind.happy_ghast().harness_trim,
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            })
        );
        assert!(instance.render_state.kind.happy_ghast().harness_foil);
        assert!(
            instance
                .render_state
                .kind
                .happy_ghast()
                .body_item_is_harness
        );
        assert!(instance.render_state.kind.happy_ghast().is_leash_holder);
        assert_eq!(instance.render_state.kind.happy_ghast().ridden, ridden);
    }
}

#[test]
fn entity_model_instances_project_armor_stand_flags_and_pose_without_pose_lerp() {
    let mut world = WorldStore::new();
    let accessors = armor_stand_metadata_accessors();
    world.apply_add_entity(protocol_add_entity(
        5,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        [1.0, 64.0, -2.0],
    ));
    let mut pose = armor_stand_default_model_pose();
    pose.body = [0.0, 15.0, 0.0];
    pose.left_arm = [-30.0, 0.0, -20.0];
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 5,
        values: vec![
            protocol_byte_data(
                accessors.client_flags_data_id,
                accessors.small_mask
                    | accessors.show_arms_mask
                    | accessors.no_baseplate_mask
                    | accessors.marker_mask,
            ),
            protocol_rotations_data(accessors.body_pose_data_id, pose.body),
            protocol_rotations_data(accessors.left_arm_pose_data_id, pose.left_arm),
        ],
    }));

    let start_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        start_instances[0].kind, instances[0].kind,
        "vanilla copies each synced ArmorStand Rotations pose directly at every partial tick"
    );

    assert_eq!(
        instances,
        vec![EntityModelInstance::armor_stand_with_marker(
            5,
            [1.0, 64.0, -2.0],
            0.0,
            true,
            true,
            true,
            false,
            pose,
        )
        .with(|s| s.kind = EntityKindRenderState::ArmorStand(Default::default()))
        .with(|s| s.age_scale = 0.5),]
    );
    assert_eq!(
        instances[0].render_state.age_in_ticks, 0.0,
        "ArmorStandModel and its layers do not consume the generic age field"
    );
}

#[test]
fn entity_model_instances_project_armor_stand_hit_wiggle() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        5,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        [1.0, 64.0, -2.0],
    ));

    fn armor_stand_wiggle(world: &WorldStore, partial_tick: f32) -> f32 {
        entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
            .into_iter()
            .find(|instance| instance.entity_id == 5)
            .unwrap()
            .render_state
            .kind
            .armor_stand()
            .wiggle
    }

    // Rest projects the first non-wobbling value: vanilla only applies the
    // setup rotation while `ArmorStandRenderState.wiggle < ArmorStand.WOBBLE_TIME`.
    assert_eq!(armor_stand_wiggle(&world, 0.0), 5.0);

    assert!(
        world.apply_entity_event(bbb_protocol::packets::EntityEvent {
            entity_id: 5,
            event_id: 32,
        })
    );
    assert_eq!(armor_stand_wiggle(&world, 0.0), 0.0);
    assert_eq!(armor_stand_wiggle(&world, 0.5), 0.5);

    world.advance_entity_client_animations(4);
    assert_eq!(armor_stand_wiggle(&world, 0.75), 4.75);

    world.advance_entity_client_animations(1);
    assert_eq!(armor_stand_wiggle(&world, 0.0), 5.0);
}

#[test]
fn entity_model_instances_project_avatar_model_part_visibility_from_world() {
    let mut world = WorldStore::new();
    let player = protocol_add_entity(1550, VANILLA_ENTITY_TYPE_PLAYER_ID, [1.0, 64.0, -2.0]);
    let player_uuid = player.uuid;
    world.apply_add_entity(player);
    world.apply_add_entity(protocol_add_entity(
        830,
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        [3.0, 64.0, -2.0],
    ));
    let player_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK
            | PlayerModelPartVisibility::JACKET_MASK
            | PlayerModelPartVisibility::RIGHT_PANTS_MASK,
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 1550,
        values: vec![protocol_byte_data(
            AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
            player_parts.vanilla_mask() as i8,
        )],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![
                EntityModelInstance::player_with_skin(
                    1550,
                    [1.0, 64.0, -2.0],
                    0.0,
                    EntityPlayerSkin::Default(default_player_skin_for_profile_id(
                        player_uuid.as_u128(),
                    )),
                    player_parts,
                )
                .with(|s| s.kind.player_mut().fall_flying_time_in_ticks = 1.0),
                EntityModelInstance::player_with_skin(
                    830,
                    [3.0, 64.0, -2.0],
                    0.0,
                    EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimAlex),
                    PlayerModelPartVisibility::from_vanilla_mask(
                        PlayerModelPartVisibility::ALL_MASK,
                    ),
                )
                .with(|s| s.kind = EntityKindRenderState::Player(Default::default())),
            ],
            1.0,
        )
    );
}

#[test]
fn mannequin_projects_data_profile_skin_patch_and_default_alex() {
    use bbb_protocol::packets::{
        EntityDataValue, EntityDataValueKind, PlayerSkinPatchData, ResolvableProfileData,
        ResolvableProfileKind,
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        831,
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        [2.0, 64.0, 0.0],
    ));
    let wide_steve_uuid = uuid::Uuid::from_u128(1);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 831,
        values: vec![EntityDataValue {
            data_id: 17,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_RESOLVABLE_PROFILE_ID,
            value: EntityDataValueKind::ResolvableProfile(ResolvableProfileData {
                profile: ResolvableProfileKind::Partial {
                    name: None,
                    id: Some(wide_steve_uuid),
                    properties: Vec::new(),
                },
                skin_patch: PlayerSkinPatchData {
                    body: Some("minecraft:entity/player/wide/steve".to_string()),
                    cape: None,
                    elytra: None,
                    slim_model: None,
                },
            }),
        }],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances[0].kind,
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: PlayerModelPartVisibility::from_vanilla_mask(
                PlayerModelPartVisibility::ALL_MASK,
            ),
        }
    );
}

#[test]
fn entity_model_instances_use_uuid_default_skin_without_player_info() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_uuid(
        1551,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        Uuid::nil(),
        [2.0, 64.0, -2.0],
    ));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1551)
        .unwrap();

    assert_eq!(
        instance.kind,
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimAlex),
            parts: PlayerModelPartVisibility::from_vanilla_mask(0),
        }
    );
}

#[test]
fn entity_model_instances_use_player_info_profile_skin_for_players() {
    const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";
    const SKIN_URL: &str = "https://textures.minecraft.net/texture/skinhash";
    const CAPE_URL: &str = "https://textures.minecraft.net/texture/capehash";
    const ELYTRA_URL: &str = "https://textures.minecraft.net/texture/elytrahash";

    let mut world = WorldStore::new();
    let player = protocol_add_entity(1552, VANILLA_ENTITY_TYPE_PLAYER_ID, [1.0, 64.0, -2.0]);
    let profile_id = player.uuid;
    // Vanilla's local lookup accepts an insecure packed property; remote
    // player-info lookups require a Services-authenticated signature.
    world.set_local_player_profile_id(profile_id);
    world.apply_add_entity(player);
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![PlayerInfoEntry {
            profile_id,
            profile: Some(GameProfile {
                uuid: profile_id,
                name: "Alex".into(),
                properties: vec![GameProfileProperty {
                    name: "textures".into(),
                    value: SLIM_TEXTURES_PROPERTY.to_string(),
                    signature: Some("signature".to_string()),
                }],
            }),
            listed: true,
            latency: 0,
            game_mode: GameType::Survival,
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
    let runtime = NativeItemRuntime::empty_for_test();

    let pending = entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1552)
        .unwrap();
    assert!(matches!(
        pending.kind,
        EntityModelKind::Player {
            skin: EntityPlayerSkin::ProfiledDefault(_),
            ..
        }
    ));
    assert_eq!(pending.render_state.kind.player().cape_texture, None);
    assert_eq!(pending.render_state.kind.player().elytra_texture, None);

    runtime.mark_profile_skin_resolved(SKIN_URL, 71);
    runtime.mark_profile_texture_resolved(
        bbb_client_render::item_model::DynamicPlayerTextureKind::Cape,
        CAPE_URL,
        72,
    );
    runtime.mark_profile_texture_resolved(
        bbb_client_render::item_model::DynamicPlayerTextureKind::Elytra,
        ELYTRA_URL,
        73,
    );
    let instance = entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1552)
        .unwrap();

    let EntityModelKind::Player { skin, parts } = instance.kind else {
        panic!("player entity should use the player model");
    };
    let EntityPlayerSkin::Dynamic(skin) = skin else {
        panic!("player info textures property should produce a dynamic player skin");
    };
    assert_eq!(skin.model, EntityPlayerSkinModel::Slim);
    assert_eq!(skin.status, EntityDynamicPlayerSkinStatus::Ready);
    assert_ne!(skin.handle, 0);
    assert_eq!(parts, PlayerModelPartVisibility::from_vanilla_mask(0));
    let cape = instance
        .render_state
        .kind
        .player()
        .cape_texture
        .expect("profile cape texture");
    let elytra = instance
        .render_state
        .kind
        .player()
        .elytra_texture
        .expect("profile elytra texture");
    assert_eq!(cape.kind, EntityDynamicPlayerTextureKind::Cape);
    assert_eq!(elytra.kind, EntityDynamicPlayerTextureKind::Elytra);
    assert_ne!(cape.handle, 0);
    assert_ne!(elytra.handle, 0);
    assert_ne!(cape.handle, elytra.handle);
}

#[test]
fn entity_model_instances_forward_player_extra_ears_from_world_source() {
    let mut world = WorldStore::new();
    let deadmau5_uuid = Uuid::from_u128(0xCCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC);
    let mixed_case_uuid = Uuid::from_u128(0xDDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD);
    world.apply_add_entity(protocol_add_entity_with_uuid(
        1554,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        deadmau5_uuid,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity_with_uuid(
        1555,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        mixed_case_uuid,
        [2.0, 64.0, -2.0],
    ));
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![
            PlayerInfoEntry {
                profile_id: deadmau5_uuid,
                profile: Some(GameProfile {
                    uuid: deadmau5_uuid,
                    name: "deadmau5".into(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: GameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            },
            PlayerInfoEntry {
                profile_id: mixed_case_uuid,
                profile: Some(GameProfile {
                    uuid: mixed_case_uuid,
                    name: "Deadmau5".into(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: GameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            },
        ],
    });

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let deadmau5 = instances
        .iter()
        .find(|instance| instance.entity_id == 1554)
        .expect("deadmau5 player instance");
    let mixed_case = instances
        .iter()
        .find(|instance| instance.entity_id == 1555)
        .expect("mixed-case player instance");

    assert!(deadmau5.render_state.kind.player().show_extra_ears);
    assert!(!mixed_case.render_state.kind.player().show_extra_ears);
}

#[test]
fn entity_model_instances_forward_player_cape_cloak_state() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        1553,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 1553,
        position: Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: true,
    }));
    world.advance_entity_client_animations(1);
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 1553,
        position: Vec3d {
            x: 0.0,
            y: 65.0,
            z: 1.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: true,
    }));
    world.advance_entity_client_animations(1);

    let source = world
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 1553)
        .unwrap();
    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1553)
        .unwrap();

    assert_eq!(
        (
            instance.render_state.kind.player().cape_flap,
            instance.render_state.kind.player().cape_lean,
            instance.render_state.kind.player().cape_lean2,
        ),
        (
            source.render.kind.player().cape_flap,
            source.render.kind.player().cape_lean,
            source.render.kind.player().cape_lean2,
        )
    );
    assert_eq!(instance.render_state.kind.player().cape_flap, -6.0);
    assert_eq!(instance.render_state.kind.player().cape_lean, 74.25);
    assert_eq!(instance.render_state.kind.player().cape_lean2, 0.0);
}

#[test]
fn entity_model_instances_forward_player_shoulder_parrots_from_world_source() {
    // Vanilla Player shoulder parrots use generated `OPTIONAL_UNSIGNED_INT` accessors. World keeps
    // the raw `Parrot.Variant` ids; native maps them to renderer `ParrotModelVariant`s.

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        1556,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 1556,
        values: vec![
            protocol_optional_unsigned_int_data(
                VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
                Some(4),
            ),
            protocol_optional_unsigned_int_data(
                VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
                Some(1),
            ),
        ],
    }));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1556)
        .unwrap();

    assert_eq!(
        instance.render_state.kind.player().left_shoulder_parrot,
        Some(ParrotModelVariant::Gray)
    );
    assert_eq!(
        instance.render_state.kind.player().right_shoulder_parrot,
        Some(ParrotModelVariant::Blue)
    );
}

#[test]
fn entity_model_instances_project_chest_equipment_layers() {
    const CHESTPLATE_ID: i32 = 0;
    const ELYTRA_ID: i32 = 1;
    let trim_texture = EntityDynamicTexture { handle: 42_4312 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/wings/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:diamond_chestplate",
                "minecraft:elytra"
            ],
            "protocol_ids": {
                "minecraft:diamond_chestplate": CHESTPLATE_ID,
                "minecraft:elytra": ELYTRA_ID
            },
            "default_component_codec_values": {
                "minecraft:diamond_chestplate": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "chest", "asset_id": "minecraft:diamond"}
                },
                "minecraft:elytra": {
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string(): {"slot": "chest", "asset_id": "minecraft:elytra"}
                }
            }
        }))
        .unwrap();
    let equipment_assets: bbb_client_assets::EquipmentAssetCatalog =
            serde_json::from_value(serde_json::json!({
                "assets": {
                    "minecraft:diamond": {
                        "layers": {
                            "humanoid": [
                                {
                                    "texture": "minecraft:diamond",
                                    "texture_location": "minecraft:textures/entity/equipment/humanoid/diamond.png",
                                    "use_player_texture": false
                                }
                            ]
                        }
                    },
                    "minecraft:elytra": {
                        "layers": {
                            "wings": [
                                {
                                    "texture": "minecraft:elytra",
                                    "texture_location": "minecraft:textures/entity/equipment/wings/elytra.png",
                                    "use_player_texture": true
                                }
                            ]
                        }
                    }
                }
            }))
            .unwrap();
    let runtime =
        NativeItemRuntime::for_test_with_registry_and_equipment_assets(registry, equipment_assets);

    let mut world = WorldStore::new();
    crate::resource_reload::apply_item_runtime_world_defaults(&runtime, &mut world).unwrap();
    world.rebuild_item_component_defaults().unwrap();
    world.apply_add_entity(protocol_add_entity(
        1553,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        1554,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [3.0, 64.0, -2.0],
    ));
    let equip_with_patch = |entity_id: i32,
                            item_id: Option<i32>,
                            count: i32,
                            component_patch: DataComponentPatchSummary|
     -> SetEquipment {
        SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Chest,
                item: ItemStackSummary {
                    item_id,
                    count,
                    component_patch: component_patch.into(),
                },
            }],
        }
    };
    let equip = |entity_id: i32, item_id: Option<i32>, count: i32| {
        equip_with_patch(
            entity_id,
            item_id,
            count,
            DataComponentPatchSummary::default(),
        )
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            Some(&runtime),
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let trimmed_foil = DataComponentPatchSummary {
        enchantments: vec![ItemEnchantmentSummary {
            holder_id: 12,
            level: 1,
        }],
        armor_trim_material_id: Some(5), // iron in loader order
        armor_trim_pattern_id: Some(8),  // sentry in loader order
        ..Default::default()
    };
    assert!(world.apply_set_equipment(equip_with_patch(
        1553,
        Some(ELYTRA_ID),
        1,
        trimmed_foil.clone(),
    )));
    let with_elytra = state(&world, 1553);
    assert_eq!(
        with_elytra.chest_equipment_asset,
        Some(EntityEquipmentAssetId::new(0, 1))
    );
    assert_eq!(
        with_elytra.chest_wings_equipment_asset,
        Some(EntityEquipmentAssetId::new(0, 1))
    );
    assert!(
        with_elytra.chest_armor_foil,
        "foil is stack state; the empty HUMANOID layer list suppresses its submission"
    );
    assert!(with_elytra.chest_wings_foil);
    assert_eq!(
        with_elytra.chest_wings_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );
    assert!(world.apply_set_equipment(equip_with_patch(
        1553,
        Some(CHESTPLATE_ID),
        1,
        DataComponentPatchSummary {
            enchantments: vec![ItemEnchantmentSummary {
                holder_id: 12,
                level: 1,
            }],
            ..Default::default()
        }
    )));
    let with_chestplate = state(&world, 1553);
    assert_eq!(
        with_chestplate.chest_equipment_asset,
        Some(EntityEquipmentAssetId::new(0, 0))
    );
    assert_eq!(
        with_chestplate.chest_wings_equipment_asset,
        Some(EntityEquipmentAssetId::new(0, 0))
    );
    assert!(with_chestplate.chest_armor_foil);
    assert!(
        with_chestplate.chest_wings_foil,
        "foil stays paired with the effective asset; the empty WINGS layer list suppresses it"
    );
    assert_eq!(with_chestplate.chest_wings_trim, None);

    assert!(world.apply_set_equipment(equip(1553, None, 0)));
    let empty_chest = state(&world, 1553);
    assert_eq!(empty_chest.chest_equipment_asset, None);
    assert_eq!(empty_chest.chest_wings_equipment_asset, None);
    assert!(!empty_chest.chest_armor_foil);
    assert!(!empty_chest.chest_wings_foil);
    assert_eq!(empty_chest.chest_wings_trim, None);

    assert!(world.apply_set_equipment(equip_with_patch(1554, Some(ELYTRA_ID), 1, trimmed_foil,)));
    let zombie = state(&world, 1554);
    assert_eq!(
        zombie.chest_equipment_asset,
        Some(EntityEquipmentAssetId::new(0, 1))
    );
    assert_eq!(
        zombie.chest_wings_equipment_asset,
        Some(EntityEquipmentAssetId::new(0, 1))
    );
    assert!(zombie.chest_armor_foil);
    assert!(zombie.chest_wings_foil);
    assert_eq!(
        zombie.chest_wings_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );
}

#[test]
fn entity_model_instances_project_slime_and_magma_cube_size() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        117,
        VANILLA_ENTITY_TYPE_SLIME_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        80,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 117,
        values: vec![slime_size_data(VANILLA_ENTITY_TYPE_SLIME_ID, 4)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 80,
        values: vec![slime_size_data(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, 3)],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![
                EntityModelInstance::slime(117, [1.0, 64.0, -2.0], 0.0, 4)
                    .with(|s| s.kind = EntityKindRenderState::Slime(Default::default())),
                EntityModelInstance::magma_cube(80, [3.0, 64.0, -2.0], 0.0, 3)
                    .with(|s| s.kind = EntityKindRenderState::Slime(Default::default())),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_do_not_drop_pick_target_entity_types() {
    let mut world = WorldStore::new();
    for (index, entity_type_id) in VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS
        .iter()
        .copied()
        .enumerate()
    {
        world.apply_add_entity(protocol_add_entity(
            1000 + index as i32,
            entity_type_id,
            [index as f64 * 2.0, 64.0, 0.0],
        ));
    }

    let sources = world.entity_model_sources_at_partial_tick(1.0);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        sources.len(),
        VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS.len()
    );
    assert_eq!(instances.len(), sources.len());
}

#[test]
fn painting_without_ready_variant_or_atlas_suppresses_raw_fallback_geometry() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:painting_variant",
        0,
        vec![RegistryPacketEntry::stub("minecraft:kebab")],
    );
    let mut spawn = protocol_add_entity(9300, VANILLA_ENTITY_TYPE_PAINTING_ID, [1.0, 64.0, -2.0]);
    spawn.data = 3;
    world.apply_add_entity(spawn);
    let data_id =
        bbb_client_world::vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PAINTING_ID,
        )
        .expect("generated Painting variant accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 9300,
        values: vec![EntityDataValue {
            data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
            value: bbb_protocol::packets::EntityDataValueKind::PaintingVariant(
                bbb_protocol::packets::PaintingVariantData {
                    registry: Some(
                        bbb_protocol::packets::EntityDataRegistryReference::unresolved(0),
                    ),
                    direct: None,
                },
            ),
        }],
    }));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 9300)
        .expect("not-ready painting presentation source");
    assert_eq!(instance.kind, EntityModelKind::NoRender);
}

#[test]
fn primed_tnt_uses_block_attachment_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TNT_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn falling_block_uses_block_attachment_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn firework_rocket_uses_item_billboard_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn item_entity_uses_item_model_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ITEM_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn ominous_item_spawner_uses_item_cluster_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn lightning_bolt_uses_weather_target_renderer_not_entity_placeholder() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, &[]),
        EntityModelKind::NoRender
    );
}
