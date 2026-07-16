use super::*;

pub(super) fn villager_model_data(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> VillagerModelData {
    let Some(data_id) =
        bbb_world::vanilla_entity_metadata_villager_data_accessor_for_entity_type(entity_type_id)
    else {
        return VillagerModelData::DEFAULT;
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::VillagerData {
                villager_type,
                profession,
                level,
            } => Some((*villager_type, *profession, *level)),
            _ => None,
        })
        .map(|(villager_type, profession, level)| {
            VillagerModelData::new(
                resolve_villager_type(villager_type, registries),
                resolve_villager_profession(profession, registries),
                level,
            )
        })
        .unwrap_or(VillagerModelData::DEFAULT)
}

pub(super) fn resolve_villager_type(
    registry_id: i32,
    _registries: Option<&dyn EntityVariantRegistryLookup>,
) -> VillagerModelType {
    VillagerModelType::for_vanilla_registry_id(registry_id).unwrap_or(VillagerModelType::Plains)
}

pub(super) fn resolve_villager_profession(
    registry_id: i32,
    _registries: Option<&dyn EntityVariantRegistryLookup>,
) -> VillagerModelProfession {
    VillagerModelProfession::for_vanilla_registry_id(registry_id)
        .unwrap_or(VillagerModelProfession::None)
}

/// Vanilla `RabbitRenderer` picks `AdultRabbitModel` for an adult and `BabyRabbitModel` for a baby; both
/// render through the dedicated [`EntityModelKind::Rabbit`] (`baby` selecting the body layout),
/// textured by the `Rabbit.Variant` colour (`DATA_TYPE_ID`, 18) with the `Toast` custom-name override
/// (`RabbitRenderer.checkMagicName(entity, "Toast")`).
pub(super) fn rabbit_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let entity_accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    let variant_id = vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default();
    EntityModelKind::Rabbit {
        baby: ageable_baby(values),
        variant: RabbitModelVariant::from_id(variant_id),
        toast: entity_data_optional_component(values, entity_accessors.custom_name_data_id)
            .is_some_and(|name| name == "Toast"),
    }
}

/// Vanilla `Shulker.getColor()` reads `DATA_COLOR_ID` (18, byte): `0..=15` map to the dye,
/// everything else (the default byte 16) is `null`, which `ShulkerRenderer.getTextureLocation`
/// renders with the uncolored `shulker.png`.
pub(super) fn shulker_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let Some(accessors) = vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)
    else {
        return EntityModelKind::Shulker { color: None };
    };
    let color_id = entity_data_byte(values, accessors.color_data_id, accessors.color_default);
    let color = (0..=15)
        .contains(&color_id)
        .then(|| EntityDyeColor::from_vanilla_id(color_id as i32));
    EntityModelKind::Shulker { color }
}

/// Vanilla `ParrotRenderer.getVariantTexture` selects the parrot colour from `Parrot.getVariant()`
/// (the synced `DATA_VARIANT_ID` int, mapped through `Parrot.Variant.byId`). Renders through the
/// dedicated [`EntityModelKind::Parrot`] (`variant` selecting the texture).
pub(super) fn parrot_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let variant = vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default();
    EntityModelKind::Parrot {
        variant: ParrotModelVariant::from_id(variant),
    }
}

/// Vanilla `PandaRenderer` (an `AgeableMobRenderer`) picks `PandaModel` for an adult and `BabyPandaModel`
/// for a baby; both render through the dedicated [`EntityModelKind::Panda`] (`baby` selecting the
/// layout, `variant` selecting the gene-driven texture). The displayed gene is
/// `Panda.Gene.getVariantFromGenes(mainGene, hiddenGene)` off the two synced gene bytes.
pub(super) fn panda_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let Some(accessors) = vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id)
    else {
        return EntityModelKind::Panda {
            baby: ageable_baby(values),
            variant: PandaModelVariant::Normal,
        };
    };
    let main_gene = entity_data_byte(
        values,
        accessors.main_gene_data_id,
        accessors.main_gene_default,
    ) as i32;
    let hidden_gene = entity_data_byte(
        values,
        accessors.hidden_gene_data_id,
        accessors.hidden_gene_default,
    ) as i32;
    EntityModelKind::Panda {
        baby: ageable_baby(values),
        variant: PandaModelVariant::from_genes(main_gene, hidden_gene),
    }
}

/// Vanilla `CatRenderer` / `OcelotRenderer` (both `AgeableMobRenderer`s) pick `AdultCatModel` /
/// `AdultOcelotModel` (the shared `AdultFelineModel` mesh, the cat scaled 0.8) for an adult and the
/// flatter `BabyFelineModel` mesh (unscaled for both breeds) for a baby. Both render through the
/// dedicated [`EntityModelKind::Feline`] (`cat` selecting the breed/scale, `baby` selecting the
/// layout). For cats the `cat_variant` is decoded from `DATA_VARIANT_ID` (20, `Holder<CatVariant>`);
/// the ocelot has no breed, so it carries the default (ignored when `!cat`). `collar` mirrors vanilla
/// `CatRenderer` (`isTame() ? getCollarColor() : null`): the dyed collar of a tame cat only.
pub(super) fn feline_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    cat: bool,
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    let tame =
        vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type(entity_type_id)
            .is_some_and(|(data_id, default)| {
                entity_data_byte(values, data_id, default) & TAMABLE_ANIMAL_TAME_FLAG != 0
            });
    EntityModelKind::Feline {
        cat,
        baby: ageable_baby(values),
        cat_variant: if cat {
            cat_model_variant(entity_type_id, values, registries)
        } else {
            CatModelVariant::Black
        },
        collar: (cat && tame).then(|| {
            EntityDyeColor::from_vanilla_id(entity_data_int(
                values,
                CAT_COLLAR_COLOR_DATA_ID,
                CAT_DEFAULT_COLLAR_COLOR_ID,
            ))
        }),
    }
}

pub(super) fn cat_model_variant(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> CatModelVariant {
    let Some(accessors) =
        vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity_type_id)
    else {
        return CatModelVariant::Black;
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == accessors.variant_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CatVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registries) = registries {
                cat_variant_from_registry_id(registries, id).unwrap_or_else(|| {
                    cat_default_model_variant(accessors.variant_default_resource_id)
                })
            } else {
                CatModelVariant::for_vanilla_registry_id(id).unwrap_or_else(|| {
                    cat_default_model_variant(accessors.variant_default_resource_id)
                })
            }
        })
        .unwrap_or_else(|| cat_default_model_variant(accessors.variant_default_resource_id))
}

fn cat_default_model_variant(default_resource_id: &str) -> CatModelVariant {
    CatModelVariant::for_resource_id(default_resource_id)
        .expect("generated vanilla cat default variant must map to a model")
}

pub(super) fn cat_variant_from_registry_id(
    registries: &dyn EntityVariantRegistryLookup,
    registry_id: i32,
) -> Option<CatModelVariant> {
    registries
        .resource_id_by_holder_id("minecraft:cat_variant", registry_id)
        .and_then(CatModelVariant::for_resource_id)
}

/// Vanilla `FoxRenderer` (an `AgeableMobRenderer`) picks `AdultFoxModel` for an adult and `BabyFoxModel`
/// for a baby; both render through the dedicated [`EntityModelKind::Fox`] (`baby` selecting the layout).
pub(super) fn fox_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let variant = vanilla_entity_metadata_fox_variant_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default();
    EntityModelKind::Fox {
        baby: ageable_baby(values),
        variant: FoxModelVariant::from_id(variant),
    }
}

/// Vanilla `NautilusRenderer` (an `AgeableMobRenderer`) picks `NautilusModel.createBodyMesh` for an
/// adult and the smaller `createBabyBodyLayer` for a baby; both render through the dedicated
/// [`EntityModelKind::Nautilus`] (`baby` selecting the layout). The zombie nautilus reuses the same
/// adult body — see [`zombie_nautilus_model_kind`].
pub(super) fn nautilus_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    EntityModelKind::Nautilus {
        baby: ageable_baby(values),
    }
}

/// Vanilla `ZombieNautilusRenderer` (a plain `MobRenderer`, so never a baby), selected by the synced
/// `ZombieNautilusVariant` holder: `NORMAL`/`TEMPERATE` renders the living adult `NautilusModel` body
/// over `zombie_nautilus.png`, `WARM` renders the `ZombieNautilusCoralModel` (the same body plus the
/// `corals` subtree) over `zombie_nautilus_coral.png`. The saddle equipment layer is driven by
/// render state; the body armor equipment layer defers.
pub(super) fn zombie_nautilus_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    EntityModelKind::ZombieNautilus {
        variant: zombie_nautilus_model_variant(values, registries),
    }
}

/// Vanilla `ZombieNautilus.DATA_VARIANT_ID` carries a `Holder<ZombieNautilusVariant>` after the
/// inherited TamableAnimal and AbstractNautilus accessors. Resolve its generated wire index and
/// virtual default through the metadata schema, and resolve holder identity through the synchronized
/// session registry when available because even a two-entry registry can be reordered. Fall back to
/// the generated vanilla order only when there is no session registry. The registry element's
/// `modelAndTexture` payload remains a separate RegistryAccess codec concern.
pub(super) fn zombie_nautilus_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> ZombieNautilusModelVariant {
    let (variant_data_id, variant_default_resource_id) =
        vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        )
        .expect("generated ZombieNautilus variant metadata accessor must resolve");
    values
        .iter()
        .rev()
        .find(|value| value.data_id == variant_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::ZombieNautilusVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registries) = registries {
                zombie_nautilus_variant_from_registry_id(registries, id).unwrap_or_else(|| {
                    zombie_nautilus_default_model_variant(variant_default_resource_id)
                })
            } else {
                ZombieNautilusModelVariant::for_vanilla_registry_id(id).unwrap_or_else(|| {
                    zombie_nautilus_default_model_variant(variant_default_resource_id)
                })
            }
        })
        .unwrap_or_else(|| zombie_nautilus_default_model_variant(variant_default_resource_id))
}

fn zombie_nautilus_default_model_variant(default_resource_id: &str) -> ZombieNautilusModelVariant {
    ZombieNautilusModelVariant::for_resource_id(default_resource_id)
        .expect("generated vanilla zombie nautilus default variant must map to a model")
}

pub(super) fn zombie_nautilus_variant_from_registry_id(
    registries: &dyn EntityVariantRegistryLookup,
    registry_id: i32,
) -> Option<ZombieNautilusModelVariant> {
    registries
        .resource_id_by_holder_id("minecraft:zombie_nautilus_variant", registry_id)
        .and_then(ZombieNautilusModelVariant::for_resource_id)
}

pub(super) fn boat(family: BoatModelFamily, chest: bool) -> EntityModelKind {
    EntityModelKind::Boat { family, chest }
}

pub(super) fn chicken_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    EntityModelKind::Chicken {
        variant: chicken_model_variant(entity_type_id, values, registries),
        baby: ageable_baby(values),
    }
}

pub(super) fn pig_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    EntityModelKind::Pig {
        variant: pig_model_variant(entity_type_id, values, registries),
        baby: ageable_baby(values),
    }
}

pub(super) fn cow_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    EntityModelKind::Cow {
        variant: cow_model_variant(entity_type_id, values, registries),
        baby: ageable_baby(values),
    }
}

/// Applies a synchronized farm-animal presentation only when its selected adult/baby texture is
/// already resident. Until then `kind` retains the holder-identity built-in fallback, keeping the
/// geometry and texture switch atomic.
pub(super) fn apply_ready_farm_animal_variant_presentation(
    kind: &mut EntityModelKind,
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<EntityDynamicTexture> {
    let registries = registries?;
    let accessors =
        vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(entity_type_id)?;
    let (registry, holder_id, baby) = match kind {
        EntityModelKind::Chicken { baby, .. }
            if entity_type_id == VANILLA_ENTITY_TYPE_CHICKEN_ID =>
        {
            (
                CHICKEN_VARIANT_REGISTRY,
                entity_data_registry_holder_id(
                    values,
                    accessors.variant_data_id,
                    EntityDataRegistryHolder::ChickenVariant,
                )?,
                *baby,
            )
        }
        EntityModelKind::Cow { baby, .. } if entity_type_id == VANILLA_ENTITY_TYPE_COW_ID => (
            COW_VARIANT_REGISTRY,
            entity_data_registry_holder_id(
                values,
                accessors.variant_data_id,
                EntityDataRegistryHolder::CowVariant,
            )?,
            *baby,
        ),
        EntityModelKind::Pig { baby, .. } if entity_type_id == VANILLA_ENTITY_TYPE_PIG_ID => (
            PIG_VARIANT_REGISTRY,
            entity_data_registry_holder_id(
                values,
                accessors.variant_data_id,
                EntityDataRegistryHolder::PigVariant,
            )?,
            *baby,
        ),
        _ => return None,
    };
    let presentation = registries.farm_animal_variant_presentation(registry, holder_id)?;
    let asset_id = if baby {
        &presentation.baby_asset_id
    } else {
        &presentation.adult_asset_id
    };
    let texture = registries.ready_texture(asset_id)?;

    match (kind, presentation.model) {
        (EntityModelKind::Chicken { variant, .. }, FarmAnimalVariantModel::Normal) => {
            *variant = ChickenModelVariant::Temperate;
        }
        (EntityModelKind::Chicken { variant, .. }, FarmAnimalVariantModel::Cold) => {
            *variant = ChickenModelVariant::Cold;
        }
        (EntityModelKind::Cow { variant, .. }, FarmAnimalVariantModel::Normal) => {
            *variant = CowModelVariant::Temperate;
        }
        (EntityModelKind::Cow { variant, .. }, FarmAnimalVariantModel::Warm) => {
            *variant = CowModelVariant::Warm;
        }
        (EntityModelKind::Cow { variant, .. }, FarmAnimalVariantModel::Cold) => {
            *variant = CowModelVariant::Cold;
        }
        (EntityModelKind::Pig { variant, .. }, FarmAnimalVariantModel::Normal) => {
            *variant = PigModelVariant::Temperate;
        }
        (EntityModelKind::Pig { variant, .. }, FarmAnimalVariantModel::Cold) => {
            *variant = PigModelVariant::Cold;
        }
        // Strict world projection rejects `warm` for chicken/pig. Fail closed if another lookup
        // implementation violates that family constraint.
        _ => return None,
    }
    Some(texture)
}

/// Selects the synchronized cat `NETWORK_CODEC` texture only after that exact age-specific asset is
/// resident. Until then the entity keeps the holder-identity built-in texture chosen by `kind`.
pub(super) fn ready_cat_variant_texture(
    kind: &EntityModelKind,
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<EntityDynamicTexture> {
    let EntityModelKind::Feline {
        cat: true, baby, ..
    } = kind
    else {
        return None;
    };
    if entity_type_id != VANILLA_ENTITY_TYPE_CAT_ID {
        return None;
    }
    let accessors =
        vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity_type_id)?;
    let holder_id = entity_data_registry_holder_id(
        values,
        accessors.variant_data_id,
        EntityDataRegistryHolder::CatVariant,
    )?;
    let registries = registries?;
    let presentation = registries.cat_variant_presentation(holder_id)?;
    let asset_id = if *baby {
        &presentation.baby_asset_id
    } else {
        &presentation.adult_asset_id
    };
    registries.ready_texture(asset_id)
}

/// Selects the synchronized frog `NETWORK_CODEC` texture only after its resource is resident.
/// Until then the entity keeps the holder-identity built-in texture chosen by `kind`.
pub(super) fn ready_frog_variant_texture(
    kind: &EntityModelKind,
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<EntityDynamicTexture> {
    if !matches!(kind, EntityModelKind::Frog { .. })
        || entity_type_id != VANILLA_ENTITY_TYPE_FROG_ID
    {
        return None;
    }
    let (variant_data_id, _) =
        vanilla_entity_metadata_frog_variant_accessor_for_entity_type(entity_type_id)?;
    let holder_id = entity_data_registry_holder_id(
        values,
        variant_data_id,
        EntityDataRegistryHolder::FrogVariant,
    )?;
    let registries = registries?;
    let presentation = registries.frog_variant_presentation(holder_id)?;
    registries.ready_texture(&presentation.asset_id)
}

/// Selects the synchronized wolf `NETWORK_CODEC` texture for the current age and state only after
/// that exact resource is resident. Vanilla gives tame priority over angry for the same age group.
pub(super) fn ready_wolf_variant_texture(
    kind: &EntityModelKind,
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<EntityDynamicTexture> {
    let EntityModelKind::Wolf {
        baby, tame, angry, ..
    } = kind
    else {
        return None;
    };
    if entity_type_id != VANILLA_ENTITY_TYPE_WOLF_ID {
        return None;
    }
    let accessors =
        vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity_type_id)?;
    let holder_id = entity_data_registry_holder_id(
        values,
        accessors.variant_data_id,
        EntityDataRegistryHolder::WolfVariant,
    )?;
    let registries = registries?;
    let presentation = registries.wolf_variant_presentation(holder_id)?;
    let assets = if *baby {
        &presentation.baby
    } else {
        &presentation.adult
    };
    let asset_id = if *tame {
        &assets.tame_asset_id
    } else if *angry {
        &assets.angry_asset_id
    } else {
        &assets.wild_asset_id
    };
    registries.ready_texture(asset_id)
}

/// Applies the synchronized zombie-nautilus model selector and texture only after the texture is
/// resident, keeping the geometry/texture transition atomic.
pub(super) fn apply_ready_zombie_nautilus_variant_presentation(
    kind: &mut EntityModelKind,
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<EntityDynamicTexture> {
    let EntityModelKind::ZombieNautilus { variant } = kind else {
        return None;
    };
    if entity_type_id != VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID {
        return None;
    }
    let (variant_data_id, _) =
        vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(entity_type_id)?;
    let holder_id = entity_data_registry_holder_id(
        values,
        variant_data_id,
        EntityDataRegistryHolder::ZombieNautilusVariant,
    )?;
    let registries = registries?;
    let presentation = registries.zombie_nautilus_variant_presentation(holder_id)?;
    let texture = registries.ready_texture(&presentation.asset_id)?;
    *variant = match presentation.model {
        ZombieNautilusVariantModelType::Normal => ZombieNautilusModelVariant::Temperate,
        ZombieNautilusVariantModelType::Warm => ZombieNautilusModelVariant::Warm,
    };
    Some(texture)
}

fn entity_data_registry_holder_id(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    expected_serializer: EntityDataRegistryHolder,
) -> Option<i32> {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId { serializer, id }
                if *serializer == expected_serializer =>
            {
                Some(*id)
            }
            _ => None,
        })
}

/// Vanilla `MushroomCowRenderer` (an `AgeableMobRenderer`) renders the mooshroom with the shared
/// `CowModel` / `BabyCowModel` body (`ModelLayers.MOOSHROOM` bakes to the temperate `cowBodyLayer`,
/// `MOOSHROOM_BABY` to `BabyCowModel.createBodyLayer()`), so it maps to the dedicated
/// [`EntityModelKind::Mooshroom`] (`baby` selecting the layout) — the real cow body instead of the
/// generic quadruped stand-in. The red/brown body texture and adult-only mushroom block-model layer are
/// projected from the synced `MushroomCow.DATA_TYPE` (index 18).
pub(super) fn mooshroom_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let variant =
        vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(entity_type_id)
            .map(|(data_id, default)| entity_data_int(values, data_id, default))
            .unwrap_or_default();
    EntityModelKind::Mooshroom {
        baby: ageable_baby(values),
        variant: MooshroomVariant::from_vanilla_id(variant),
    }
}

pub(super) fn sheep_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    age_ticks: f32,
) -> EntityModelKind {
    let entity_accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    let wool = vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_byte(values, data_id, default) as u8)
        .unwrap_or_default();
    EntityModelKind::Sheep {
        baby: ageable_baby(values),
        sheared: wool & SHEEP_WOOL_SHEARED_FLAG as u8 != 0,
        wool_color: SheepWoolColor::from_vanilla_id(wool & SHEEP_WOOL_COLOR_MASK),
        jeb: entity_data_optional_component(values, entity_accessors.custom_name_data_id)
            .is_some_and(|name| name == "jeb_"),
        age_ticks,
    }
}

pub(super) fn player_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let fallback = if entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID {
        MANNEQUIN_DEFAULT_MODEL_CUSTOMIZATION
    } else {
        AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION
    };
    let mask = entity_data_byte(values, AVATAR_MODEL_CUSTOMIZATION_DATA_ID, fallback) as u8;
    EntityModelKind::Player {
        skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        parts: PlayerModelPartVisibility::from_vanilla_mask(mask),
    }
}

pub(super) fn apply_player_profile_skin(
    kind: &mut EntityModelKind,
    source: &EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let EntityModelKind::Player { skin, .. } = kind else {
        return;
    };
    if source.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
        return;
    }
    *skin = EntityPlayerSkin::Default(default_player_skin_for_profile_id(source.uuid.as_u128()));
    let Some(item_runtime) = item_runtime else {
        return;
    };
    let Some(info) = world.player_info_entry(source.uuid) else {
        return;
    };
    *skin = item_runtime.player_skin_for_profile(&player_info_profile_resolvable(&info.profile));
}

pub(super) fn player_profile_texture(
    source: &EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    kind: EntityDynamicPlayerTextureKind,
) -> Option<EntityDynamicPlayerTexture> {
    if source.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
        return None;
    }
    let item_runtime = item_runtime?;
    let info = world.player_info_entry(source.uuid)?;
    item_runtime
        .player_profile_texture_for_profile(&player_info_profile_resolvable(&info.profile), kind)
}

pub(super) fn chest_equipment_layers(
    source: &EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> (Option<EntityEquipmentLayerTexture>, bool, bool) {
    let Some(item_runtime) = item_runtime else {
        return (None, false, false);
    };
    let Some(stack) = world.equipment_item(source.entity_id, EquipmentSlot::Chest) else {
        return (None, false, false);
    };
    if !item_stack_non_empty(&stack) {
        return (None, false, false);
    }
    let Some(item_id) = stack.item_id else {
        return (None, false, false);
    };
    (
        item_runtime.item_equipment_wings_layer(item_id),
        item_runtime.item_equipment_asset_has_wings_layer(item_id),
        item_runtime.item_equipment_asset_has_humanoid_layer(item_id),
    )
}

pub(super) fn player_info_profile_resolvable(
    profile: &bbb_world::PlayerInfoProfileState,
) -> ResolvableProfileSummary {
    let properties: Vec<_> = profile
        .properties
        .iter()
        .map(|property| GameProfilePropertySummary {
            name: property.name.clone(),
            value: property.value.clone(),
            signature: property.signature.clone(),
        })
        .collect();
    let profile_textures = decode_profile_textures_from_properties(
        properties
            .iter()
            .map(|property| (property.name.as_str(), property.value.as_str())),
    );
    ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::GameProfile,
        uuid: Some(profile.uuid),
        name: Some(profile.name.clone()),
        properties,
        profile_textures,
        skin_patch: PlayerSkinPatchSummary::default(),
    }
}

pub(super) fn wolf_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    let accessors = vanilla_entity_metadata_wolf_accessors_for_entity_type(entity_type_id)
        .expect("generated Wolf metadata accessors must resolve");
    let tame =
        vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type(entity_type_id)
            .is_some_and(|(data_id, default)| {
                entity_data_byte(values, data_id, default) & TAMABLE_ANIMAL_TAME_FLAG != 0
            });
    EntityModelKind::Wolf {
        baby: ageable_baby(values),
        tame,
        angry: wolf_is_angry(entity_type_id, values, game_time),
        collar_color: tame.then(|| {
            EntityDyeColor::from_vanilla_id(entity_data_int(
                values,
                accessors.collar_color_data_id,
                accessors.collar_color_default,
            ))
        }),
        variant: wolf_model_variant(entity_type_id, values, registries),
    }
}

/// Vanilla `WolfRenderer`: resolve the synced `Wolf.DATA_VARIANT_ID` registry holder to the renderer
/// variant. Mirrors [`cat_model_variant`] — prefer the dynamic `wolf_variant` registry order the
/// server sent, falling back to the static vanilla registration order, and to `Pale` (the vanilla
/// `WolfVariants.DEFAULT`) when no holder is present.
pub(super) fn wolf_model_variant(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> WolfModelVariant {
    let Some(accessors) =
        vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity_type_id)
    else {
        return WolfModelVariant::Pale;
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == accessors.variant_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::WolfVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registries) = registries {
                wolf_variant_from_registry_id(registries, id).unwrap_or_else(|| {
                    wolf_default_model_variant(accessors.variant_default_resource_id)
                })
            } else {
                WolfModelVariant::for_vanilla_registry_id(id).unwrap_or_else(|| {
                    wolf_default_model_variant(accessors.variant_default_resource_id)
                })
            }
        })
        .unwrap_or_else(|| wolf_default_model_variant(accessors.variant_default_resource_id))
}

fn wolf_default_model_variant(default_resource_id: &str) -> WolfModelVariant {
    WolfModelVariant::for_resource_id(default_resource_id)
        .expect("generated vanilla wolf default variant must map to a model")
}

pub(super) fn wolf_variant_from_registry_id(
    registries: &dyn EntityVariantRegistryLookup,
    registry_id: i32,
) -> Option<WolfModelVariant> {
    registries
        .resource_id_by_holder_id("minecraft:wolf_variant", registry_id)
        .and_then(WolfModelVariant::for_resource_id)
}

pub(super) fn entity_invisible(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    (entity_data_byte(
        values,
        accessors.shared_flags_data_id,
        accessors.shared_flags_default,
    ) & accessors.invisible_mask)
        != 0
}

pub(super) fn wolf_is_angry(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> bool {
    let Some(accessors) = vanilla_entity_metadata_wolf_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    let end_time = entity_data_long(
        values,
        accessors.anger_end_time_data_id,
        accessors.anger_end_time_default,
    );
    end_time > 0 && end_time - game_time > 0
}

/// Vanilla `BeeRenderState.isAngry` (`Bee.isAngry()`, the `NeutralMob` anger): the synced
/// `DATA_ANGER_END_TIME` is in the future (`endTime > 0 && endTime - gameTime > 0`). An angry
/// bee skips `BeeModel.bobUpAndDown`. Gated to the bee; every other entity is calm.
pub(super) fn bee_is_angry(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> bool {
    if entity_type_id != VANILLA_ENTITY_TYPE_BEE_ID {
        return false;
    }
    let end_time = entity_data_long(
        values,
        BEE_ANGER_END_TIME_DATA_ID,
        BEE_DEFAULT_ANGER_END_TIME,
    );
    end_time > 0 && end_time - game_time > 0
}

/// Vanilla `BeeRenderState.hasNectar` (`Bee.hasNectar()`, the synced `DATA_FLAGS_ID & FLAG_HAS_NECTAR`):
/// bit 8 of the bee flags byte. Drives the `BeeRenderer.getTextureLocation` nectar texture swap.
/// Gated to the bee; every other entity reports no nectar.
pub(super) fn bee_has_nectar(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_bee_flags_accessor_for_entity_type(entity_type_id).is_some_and(
        |(data_id, default)| entity_data_byte(values, data_id, default) & BEE_FLAG_HAS_NECTAR != 0,
    )
}

/// The three projected camel sit/stand elapsed-seconds values, each `-1.0` when its
/// `AnimationState` is stopped (so the renderer applies no keyframe).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CamelSitState {
    /// Vanilla `Camel.sitAnimationState` elapsed seconds (`CAMEL_SIT`, 2.0 s).
    pub(super) sit_seconds: f32,
    /// Vanilla `Camel.sitPoseAnimationState` elapsed seconds (`CAMEL_SIT_POSE`, 1.0 s).
    pub(super) sit_pose_seconds: f32,
    /// Vanilla `Camel.sitUpAnimationState` elapsed seconds (`CAMEL_STANDUP`, 2.6 s).
    pub(super) standup_seconds: f32,
}

impl CamelSitState {
    const STOPPED: Self = Self {
        sit_seconds: -1.0,
        sit_pose_seconds: -1.0,
        standup_seconds: -1.0,
    };
}

/// Vanilla `Camel.setupAnimationStates()` (client tick) projected purely from the synced
/// `LAST_POSE_CHANGE_TICK` (`Long`) and the world game time — no client-side
/// accumulator is needed because the camel's sit/stand timing is a deterministic function of
/// those two values. Mirrors `Camel.getPoseTime()` (`gameTime - |lastPoseChangeTick|`) and the
/// `isCamelSitting`/`isCamelVisuallySitting`/`isVisuallySittingDown`/`isInPoseTransition`
/// predicates, returning each active animation's `(ageInTicks - startTick)` elapsed as raw
/// seconds (the renderer clamps the non-looping tables to their final frame):
///   - `sit` and `standup` start at the pose-change tick, so their elapsed is `getPoseTime`;
///   - `sitPose` starts when the 40-tick sit-down window ends, so its elapsed is
///     `getPoseTime - 40`.
/// The `dash` (rising-edge driven) and `idle` (client random-timer driven) animations are projected
/// by the world client-animation state. Non-camel entities return [`CamelSitState::STOPPED`].
pub(super) fn camel_sit_state(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> CamelSitState {
    let Some((last_pose_change_tick_data_id, last_pose_change_tick_default)) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )
    else {
        return CamelSitState::STOPPED;
    };
    let last_pose_change_tick = entity_data_long(
        values,
        last_pose_change_tick_data_id,
        last_pose_change_tick_default,
    );
    // Vanilla `Camel.getPoseTime()` and `isCamelSitting()`.
    let pose_time = game_time - last_pose_change_tick.abs();
    let is_sitting = last_pose_change_tick < 0;
    // Vanilla `Camel.isCamelVisuallySitting()`.
    let is_visually_sitting = (pose_time < 0) != is_sitting;
    // Vanilla `Camel.isVisuallySittingDown()`.
    let is_visually_sitting_down =
        is_sitting && pose_time >= 0 && pose_time < CAMEL_SITDOWN_DURATION_TICKS;
    // Vanilla `Camel.isInPoseTransition()`.
    let transition_length = if is_sitting {
        CAMEL_SITDOWN_DURATION_TICKS
    } else {
        CAMEL_STANDUP_DURATION_TICKS
    };
    let is_in_pose_transition = pose_time < transition_length;

    let ticks_to_seconds = |ticks: i64| ticks as f32 / 20.0;
    if is_visually_sitting {
        // `sitUp`/`dash` are stopped; `sit` plays during the sit-down window, then `sitPose`.
        if is_visually_sitting_down {
            CamelSitState {
                sit_seconds: ticks_to_seconds(pose_time),
                sit_pose_seconds: -1.0,
                standup_seconds: -1.0,
            }
        } else {
            CamelSitState {
                sit_seconds: -1.0,
                // `sitPose` starts when the 40-tick sit-down window ends.
                sit_pose_seconds: ticks_to_seconds(pose_time - CAMEL_SITDOWN_DURATION_TICKS),
                standup_seconds: -1.0,
            }
        }
    } else if is_in_pose_transition && pose_time >= 0 {
        // Not visually sitting: `standup` plays during the stand-up transition.
        CamelSitState {
            sit_seconds: -1.0,
            sit_pose_seconds: -1.0,
            standup_seconds: ticks_to_seconds(pose_time),
        }
    } else {
        CamelSitState::STOPPED
    }
}

/// Vanilla `WolfRenderState.tailAngle = Wolf.getTailAngle()`: the wolf tail `xRot`. An angry
/// wolf raises it to the constant `1.5393804`; a tame wolf droops it with damage,
/// `(0.55 - damageRatio * 0.4) * π` where `damageRatio = (maxHealth - health) / maxHealth`
/// and tamed wolves have the constant `maxHealth = 40` (`Wolf.setTame` sets the base value);
/// an untamed wolf returns the `π/5` default. Non-wolf entities return the `π/5` default,
/// which matches the wolf-tail render-state default and so leaves every other model
/// untouched.
pub(super) fn wolf_tail_angle(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> f32 {
    const WILD_TAIL_ANGLE: f32 = std::f32::consts::PI / 5.0;
    if entity_type_id != VANILLA_ENTITY_TYPE_WOLF_ID {
        return WILD_TAIL_ANGLE;
    }
    if wolf_is_angry(entity_type_id, values, game_time) {
        // `Wolf.getTailAngle()` angry branch returns this exact constant.
        return 1.5393804;
    }
    let tame =
        vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type(entity_type_id)
            .is_some_and(|(data_id, default)| {
                entity_data_byte(values, data_id, default) & TAMABLE_ANIMAL_TAME_FLAG != 0
            });
    if !tame {
        return WILD_TAIL_ANGLE;
    }
    const TAME_MAX_HEALTH: f32 = 40.0;
    let health = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .map(|accessors| entity_data_float(values, accessors.health_data_id, TAME_MAX_HEALTH))
        .unwrap_or(TAME_MAX_HEALTH)
        .max(0.0);
    let damage_ratio = (TAME_MAX_HEALTH - health) / TAME_MAX_HEALTH;
    (0.55 - damage_ratio * 0.4) * std::f32::consts::PI
}

/// Vanilla `WolfRenderState.isSitting = Wolf.isInSittingPose()`: the `TamableAnimal`
/// `DATA_FLAGS_ID` sitting bit. Only the wolf model renders a sitting pose, so non-wolf
/// entities (and other tamables that are not yet modelled) report `false`.
pub(super) fn wolf_sitting(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
        && vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type(entity_type_id)
            .is_some_and(|(data_id, default)| {
                entity_data_byte(values, data_id, default) & TAMABLE_ANIMAL_SITTING_FLAG != 0
            })
}

/// Vanilla `ParrotModel.getPose == SITTING` (`Parrot.isInSittingPose()`): the `TamableAnimal`
/// `DATA_FLAGS_ID` sitting bit (id 18, the same byte the wolf uses for `isSitting`). Only the
/// parrot renders the `prepare(SITTING)` perch pose, so non-parrot entities report `false`.
pub(super) fn parrot_sitting(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PARROT_ID
        && vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type(entity_type_id)
            .is_some_and(|(data_id, default)| {
                entity_data_byte(values, data_id, default) & TAMABLE_ANIMAL_SITTING_FLAG != 0
            })
}

/// Vanilla `Raider.isCelebrating()` (the synced `IS_CELEBRATING` boolean): drives the evoker and
/// vindicator `CELEBRATING` victory-dance arm pose. Generated metadata supplies Raider membership,
/// wire index and typed default; generated class behavior resolves the nearest `getArmPose`, where
/// the illusioner override omits celebrating and the pillager never returns it. The renderer also
/// suppresses the projection while casting / aggressive, which take priority in vanilla.
pub(super) fn illager_celebrating(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_type_illager_arm_pose_uses_celebrating(entity_type_id)
        && bbb_world::vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type(
            entity_type_id,
        )
        .is_some_and(|(data_id, default)| entity_data_bool(values, data_id, default))
}

/// Vanilla `Piglin.isDancing()` (the synced `DATA_IS_DANCING` boolean): drives the piglin's
/// `DANCING` arm pose (the soul-campfire celebration — swaying ears, raised arms, bobbing head/body).
/// The generated accessor owns regular-Piglin membership, wire index and typed default; PiglinBrute
/// and ZombifiedPiglin therefore cannot read unrelated metadata as this flag.
pub(super) fn piglin_is_dancing(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    bbb_world::vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `PandaRenderState.isUnhappy = Panda.getUnhappyCounter() > 0` (the synced
/// `UNHAPPY_COUNTER` int): the panda shakes its head and paddles its front legs.
pub(super) fn panda_is_unhappy(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id).is_some_and(
        |accessors| {
            entity_data_int(
                values,
                accessors.unhappy_counter_data_id,
                accessors.unhappy_counter_default,
            ) > 0
        },
    )
}

/// Vanilla `PandaRenderState.isSneezing = Panda.isSneezing()` (the synced `DATA_ID_FLAGS` byte, id 23,
/// bit `0x02`): the panda dips its head into a sneeze. Gated to the panda type.
pub(super) fn panda_is_sneezing(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_panda_flags_accessor_for_entity_type(entity_type_id).is_some_and(
        |(data_id, default)| entity_data_byte(values, data_id, default) & PANDA_SNEEZING_FLAG != 0,
    )
}

/// Vanilla `PandaRenderState.sneezeTime = Panda.getSneezeCounter()` (the synced `SNEEZE_COUNTER`
/// int): the 0..20 ramp that drives the sneeze head dip. `0` for a non-panda or a panda not sneezing.
pub(super) fn panda_sneeze_time(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> i32 {
    vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            entity_data_int(
                values,
                accessors.sneeze_counter_data_id,
                accessors.sneeze_counter_default,
            )
        })
        .unwrap_or_default()
}

/// Vanilla `PandaRenderState.isEating = Panda.isEating()` (the synced `EAT_COUNTER` int, `> 0`).
/// The held-item layer uses this only to bob the item while the sitting gate is active.
pub(super) fn panda_is_eating(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id).is_some_and(
        |accessors| {
            entity_data_int(
                values,
                accessors.eat_counter_data_id,
                accessors.eat_counter_default,
            ) > 0
        },
    )
}

/// Vanilla `PandaRenderState.isSitting = Panda.isSitting()` (the synced `DATA_ID_FLAGS` byte, id 23, bit
/// `0x08`). `PandaHoldsItemLayer` renders only in this state.
pub(super) fn panda_is_sitting(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_panda_flags_accessor_for_entity_type(entity_type_id).is_some_and(
        |(data_id, default)| entity_data_byte(values, data_id, default) & PANDA_SITTING_FLAG != 0,
    )
}

/// Vanilla `PandaRenderState.isScared = Panda.isScared()` = `isWorried() && level.isThundering()`.
/// `isWorried()` reads the displayed gene from main/hidden genes; `Level.isThundering()` gates weather-capable
/// dimensions and checks `getThunderLevel(1.0F) > 0.9`, where `getThunderLevel` multiplies thunder by rain.
pub(super) fn panda_is_scared(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    world: &WorldStore,
) -> bool {
    let Some(accessors) = vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    if !world_is_thundering(world) {
        return false;
    }
    let main_gene = entity_data_byte(
        values,
        accessors.main_gene_data_id,
        accessors.main_gene_default,
    ) as i32;
    let hidden_gene = entity_data_byte(
        values,
        accessors.hidden_gene_data_id,
        accessors.hidden_gene_default,
    ) as i32;
    PandaModelVariant::from_genes(main_gene, hidden_gene) == PandaModelVariant::Worried
}

pub(super) fn world_is_thundering(world: &WorldStore) -> bool {
    if !world_can_have_weather(world) {
        return false;
    }
    let weather = world.weather();
    weather.rain_level.clamp(0.0, 1.0) * weather.thunder_level.clamp(0.0, 1.0) > 0.9
}

pub(super) fn world_can_have_weather(world: &WorldStore) -> bool {
    let Some(level) = world.level_info() else {
        return true;
    };
    let dimension = level.dimension.as_str();
    let dimension_type = level.dimension_type_name.as_deref();
    !matches!(
        (level.dimension_type_id, dimension, dimension_type),
        (1, _, _)
            | (2, _, _)
            | (_, "minecraft:the_nether", _)
            | (_, "minecraft:the_end", _)
            | (_, _, Some("minecraft:the_nether"))
            | (_, _, Some("minecraft:the_end"))
    )
}

/// Vanilla `TurtleRenderState.hasEgg = !isBaby() && Turtle.hasEgg()` (the synced `HAS_EGG`
/// boolean). Only the adult turtle renders the `egg_belly` overlay shell, so the projection is
/// gated by the generated Turtle accessor bundle and excludes babies (matching
/// `extractRenderState`).
pub(super) fn turtle_has_egg(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    let Some(accessors) = vanilla_entity_metadata_turtle_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    !ageable_baby(values)
        && entity_data_bool(values, accessors.has_egg_data_id, accessors.has_egg_default)
}

/// Vanilla `TurtleRenderState.isLayingEgg = Turtle.isLayingEgg()` (the synced `LAYING_EGG`
/// boolean). The egg-laying front-leg amplitude lives in the shared `TurtleModel`, so — unlike
/// `hasEgg` — babies are NOT excluded; the generated accessor bundle owns the type gate.
pub(super) fn turtle_laying_egg(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    let Some(accessors) = vanilla_entity_metadata_turtle_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    entity_data_bool(
        values,
        accessors.laying_egg_data_id,
        accessors.laying_egg_default,
    )
}

/// Vanilla `EndCrystalRenderState.showsBottom = EndCrystal.showsBottom()` (the synced
/// `DATA_SHOW_BOTTOM` boolean. Gated to the end-crystal type; a crystal
/// without the synced value keeps the vanilla `true` default (the bottom slab is shown).
pub(super) fn end_crystal_shows_bottom(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    let Some(accessors) =
        bbb_world::vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity_type_id)
    else {
        return true;
    };
    entity_data_bool(
        values,
        accessors.show_bottom_data_id,
        accessors.show_bottom_default,
    )
}

pub(super) fn donkey_model_kind(
    entity_type_id: i32,
    family: DonkeyModelFamily,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    EntityModelKind::Donkey {
        family,
        baby: ageable_baby(values),
        has_chest: chested_horse_has_chest(entity_type_id, values),
    }
}

pub(super) fn undead_horse_model_kind(
    family: UndeadHorseModelFamily,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    EntityModelKind::UndeadHorse {
        family,
        baby: ageable_baby(values),
    }
}

/// The living horse's coat color from synced `Horse.DATA_ID_TYPE_VARIANT`: vanilla
/// `Variant.byId(typeVariant & 0xFF)`, where `byId` is `ByIdMap.continuous(WRAP)` so an out-of-range
/// id wraps modulo the seven colors. The markings nibble (`(typeVariant & 0xFF00) >> 8`) is the
/// deferred `HorseMarkingLayer`.
pub(super) fn horse_color_variant(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> HorseColorVariant {
    let color = horse_type_variant(entity_type_id, values) & HORSE_COLOR_MASK;
    match color.rem_euclid(7) {
        0 => HorseColorVariant::White,
        1 => HorseColorVariant::Creamy,
        2 => HorseColorVariant::Chestnut,
        3 => HorseColorVariant::Brown,
        4 => HorseColorVariant::Black,
        5 => HorseColorVariant::Gray,
        _ => HorseColorVariant::DarkBrown,
    }
}

/// The living horse's white markings from synced `Horse.DATA_ID_TYPE_VARIANT`: vanilla
/// `Markings.byId((typeVariant & 0xFF00) >> 8)`, where `byId` is `ByIdMap.continuous(WRAP)` so an
/// out-of-range nibble wraps modulo the five markings. `Markings.NONE` draws no overlay.
pub(super) fn horse_markings(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> HorseMarkings {
    let markings =
        (horse_type_variant(entity_type_id, values) & HORSE_MARKINGS_MASK) >> HORSE_MARKINGS_SHIFT;
    match markings.rem_euclid(5) {
        0 => HorseMarkings::None,
        1 => HorseMarkings::White,
        2 => HorseMarkings::WhiteField,
        3 => HorseMarkings::WhiteDots,
        _ => HorseMarkings::BlackDots,
    }
}

fn horse_type_variant(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> i32 {
    vanilla_entity_metadata_horse_variant_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default()
}

pub(super) fn llama_model_kind(
    entity_type_id: i32,
    family: LlamaModelFamily,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let baby = ageable_baby(values);
    let variant = vanilla_entity_metadata_llama_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            entity_data_int(values, accessors.variant_data_id, accessors.variant_default)
        })
        .unwrap_or_default();
    EntityModelKind::Llama {
        family,
        variant: LlamaVariant::from_vanilla_id(variant),
        baby,
        has_chest: !baby && chested_horse_has_chest(entity_type_id, values),
    }
}

pub(super) fn goat_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let accessors = vanilla_entity_metadata_goat_accessors_for_entity_type(entity_type_id);
    EntityModelKind::Goat {
        baby: ageable_baby(values),
        left_horn: accessors.is_none_or(|accessors| {
            entity_data_bool(
                values,
                accessors.left_horn_data_id,
                accessors.left_horn_default,
            )
        }),
        right_horn: accessors.is_none_or(|accessors| {
            entity_data_bool(
                values,
                accessors.right_horn_data_id,
                accessors.right_horn_default,
            )
        }),
    }
}

/// Vanilla `Goat.getRammingXHeadRot()`: `lowerHeadTick / 20 · maxRammingXHeadRot · π/180`, where the max
/// head pitch is `52.5°` for a baby goat and `30°` for an adult. The world projects the `0..=20` ram
/// counter (advanced from entity events 58/59); `GoatModel.setupAnim` SETs `head.xRot` to this while
/// non-zero, overwriting the head-look pitch during a ram.
pub(super) fn goat_ramming_x_head_rot(lower_head_tick: i32, baby: bool) -> f32 {
    let max_degrees = if baby { 52.5 } else { 30.0 };
    lower_head_tick as f32 / 20.0 * max_degrees * std::f32::consts::PI / 180.0
}

pub(super) fn placeholder(
    name: &'static str,
    width: f32,
    height: f32,
    depth: f32,
) -> EntityModelKind {
    EntityModelKind::Placeholder {
        name,
        bounds: bbb_renderer::EntityModelBounds {
            width,
            height,
            depth,
        },
    }
}

/// Bounds-only placeholder for entities whose dedicated renderer is deferred,
/// but whose vanilla `EntityType.sized(width, height)` box is source-verified.
pub(super) fn entity_type_bounds_placeholder(
    name: &'static str,
    width: f32,
    height: f32,
) -> EntityModelKind {
    placeholder(name, width, height, width)
}

pub(super) fn armor_stand_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(entity_type_id)
        .expect("generated ArmorStand metadata accessor bundle");
    let flags = entity_data_byte(
        values,
        accessors.client_flags_data_id,
        accessors.client_flags_default,
    );
    EntityModelKind::ArmorStand {
        small: flags & ARMOR_STAND_CLIENT_FLAG_SMALL != 0,
        marker: flags & ARMOR_STAND_CLIENT_FLAG_MARKER != 0,
        show_arms: flags & ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS != 0,
        show_base_plate: flags & ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE == 0,
        pose: armor_stand_pose(values, accessors),
    }
}

pub(super) fn armor_stand_pose(
    values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaArmorStandMetadataAccessors,
) -> ArmorStandModelPose {
    ArmorStandModelPose {
        head: entity_data_rotations(
            values,
            accessors.head_pose_data_id,
            accessors.head_pose_default,
        ),
        body: entity_data_rotations(
            values,
            accessors.body_pose_data_id,
            accessors.body_pose_default,
        ),
        left_arm: entity_data_rotations(
            values,
            accessors.left_arm_pose_data_id,
            accessors.left_arm_pose_default,
        ),
        right_arm: entity_data_rotations(
            values,
            accessors.right_arm_pose_data_id,
            accessors.right_arm_pose_default,
        ),
        left_leg: entity_data_rotations(
            values,
            accessors.left_leg_pose_data_id,
            accessors.left_leg_pose_default,
        ),
        right_leg: entity_data_rotations(
            values,
            accessors.right_leg_pose_data_id,
            accessors.right_leg_pose_default,
        ),
    }
}

pub(super) fn slime_size(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> i32 {
    vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default()
}

pub(super) fn phantom_size(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> i32 {
    vanilla_entity_metadata_phantom_size_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default()
}

pub(super) fn pufferfish_puff_state(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> i32 {
    vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default()
}

pub(super) fn salmon_model_size(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> SalmonModelSize {
    let variant = vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(values, data_id, default))
        .unwrap_or_default();
    SalmonModelSize::from_vanilla_id(variant)
}

pub(super) fn tropical_fish_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let packed =
        vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(entity_type_id)
            .map(|(data_id, default)| entity_data_int(values, data_id, default))
            .unwrap_or_default();
    EntityModelKind::TropicalFish {
        shape: tropical_fish_shape(packed),
        base_color: tropical_fish_base_color(packed),
        pattern: tropical_fish_pattern(packed),
        pattern_color: tropical_fish_pattern_color(packed),
    }
}

fn tropical_fish_shape(packed: i32) -> TropicalFishModelShape {
    TropicalFishModelShape::from_vanilla_pattern_id(packed & TROPICAL_FISH_PATTERN_MASK)
}

/// Vanilla `TropicalFish.getBaseColor(packedVariant) = DyeColor.byId(packedVariant >> 16 & 0xFF)`,
/// projected into the renderer body tint (`TropicalFishRenderer.getModelTint = state.baseColor`).
fn tropical_fish_base_color(packed: i32) -> EntityDyeColor {
    EntityDyeColor::from_vanilla_id(
        (packed >> TROPICAL_FISH_BASE_COLOR_SHIFT) & TROPICAL_FISH_BASE_COLOR_MASK,
    )
}

/// Vanilla `TropicalFish.getPattern(packedVariant) = Pattern.byId(packedVariant & 0xFFFF)`, the
/// `TropicalFishPatternLayer` overlay selector.
fn tropical_fish_pattern(packed: i32) -> TropicalFishPattern {
    TropicalFishPattern::from_vanilla_pattern_id(packed & TROPICAL_FISH_PATTERN_MASK)
}

/// Vanilla `TropicalFish.getPatternColor(packedVariant) = DyeColor.byId(packedVariant >> 24 &
/// 0xFF)`, the `TropicalFishPatternLayer` tint (`state.patternColor`).
fn tropical_fish_pattern_color(packed: i32) -> EntityDyeColor {
    EntityDyeColor::from_vanilla_id(
        (packed >> TROPICAL_FISH_PATTERN_COLOR_SHIFT) & TROPICAL_FISH_PATTERN_COLOR_MASK,
    )
}

pub(super) fn ageable_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, AGEABLE_MOB_BABY_DATA_ID, AGEABLE_MOB_DEFAULT_BABY)
}

/// Vanilla `ArmadilloModel.setupAnim` `isHidingInShell` swap, projected from the synced
/// `Armadillo.ARMADILLO_STATE` (the `ArmadilloState` enum; SCARED uses the generated ordinal).
/// Only the steady SCARED state is server-derivable: it hides the body in the shell for every
/// `inStateTicks`, whereas ROLLING/UNROLLING gate the hide on the un-synced `inStateTicks`, so
/// they stay deferred (treated as not rolled up). Defaults to IDLE (not rolled up).
pub(super) fn armadillo_rolled_up(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    let Some((data_id, default)) =
        vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(entity_type_id)
    else {
        return false;
    };
    values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::ArmadilloState,
                id,
            } => Some(*id),
            _ => None,
        })
        .unwrap_or(default)
        == ARMADILLO_STATE_SCARED_ID
}

pub(super) fn chicken_model_variant(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> ChickenModelVariant {
    let Some(accessors) =
        vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(entity_type_id)
    else {
        return ChickenModelVariant::Temperate;
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == accessors.variant_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::ChickenVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registries) = registries {
                chicken_variant_from_registry_id(registries, id).unwrap_or_else(|| {
                    chicken_default_model_variant(accessors.variant_default_resource_id)
                })
            } else {
                ChickenModelVariant::for_vanilla_registry_id(id).unwrap_or_else(|| {
                    chicken_default_model_variant(accessors.variant_default_resource_id)
                })
            }
        })
        .unwrap_or_else(|| chicken_default_model_variant(accessors.variant_default_resource_id))
}

fn chicken_default_model_variant(default_resource_id: &str) -> ChickenModelVariant {
    ChickenModelVariant::for_resource_id(default_resource_id)
        .expect("generated vanilla chicken default variant must map to a model")
}

pub(super) fn chicken_variant_from_registry_id(
    registries: &dyn EntityVariantRegistryLookup,
    registry_id: i32,
) -> Option<ChickenModelVariant> {
    registries
        .resource_id_by_holder_id("minecraft:chicken_variant", registry_id)
        .and_then(ChickenModelVariant::for_resource_id)
}

pub(super) fn frog_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    EntityModelKind::Frog {
        variant: frog_model_variant(values, registries),
    }
}

pub(super) fn frog_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> FrogModelVariant {
    let (variant_data_id, variant_default_resource_id) =
        vanilla_entity_metadata_frog_variant_accessor_for_entity_type(VANILLA_ENTITY_TYPE_FROG_ID)
            .expect("generated Frog variant metadata accessor must resolve");
    values
        .iter()
        .rev()
        .find(|value| value.data_id == variant_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::FrogVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registries) = registries {
                frog_variant_from_registry_id(registries, id)
                    .unwrap_or_else(|| frog_default_model_variant(variant_default_resource_id))
            } else {
                FrogModelVariant::for_vanilla_registry_id(id)
                    .unwrap_or_else(|| frog_default_model_variant(variant_default_resource_id))
            }
        })
        .unwrap_or_else(|| frog_default_model_variant(variant_default_resource_id))
}

fn frog_default_model_variant(default_resource_id: &str) -> FrogModelVariant {
    FrogModelVariant::for_resource_id(default_resource_id)
        .expect("generated vanilla frog default variant must map to a model")
}

pub(super) fn frog_variant_from_registry_id(
    registries: &dyn EntityVariantRegistryLookup,
    registry_id: i32,
) -> Option<FrogModelVariant> {
    registries
        .resource_id_by_holder_id("minecraft:frog_variant", registry_id)
        .and_then(FrogModelVariant::for_resource_id)
}

pub(super) fn pig_model_variant(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> PigModelVariant {
    let Some(accessors) =
        vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(entity_type_id)
    else {
        return PigModelVariant::Temperate;
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == accessors.variant_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::PigVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registries) = registries {
                pig_variant_from_registry_id(registries, id).unwrap_or_else(|| {
                    pig_default_model_variant(accessors.variant_default_resource_id)
                })
            } else {
                PigModelVariant::for_vanilla_registry_id(id).unwrap_or_else(|| {
                    pig_default_model_variant(accessors.variant_default_resource_id)
                })
            }
        })
        .unwrap_or_else(|| pig_default_model_variant(accessors.variant_default_resource_id))
}

fn pig_default_model_variant(default_resource_id: &str) -> PigModelVariant {
    PigModelVariant::for_resource_id(default_resource_id)
        .expect("generated vanilla pig default variant must map to a model")
}

pub(super) fn pig_variant_from_registry_id(
    registries: &dyn EntityVariantRegistryLookup,
    registry_id: i32,
) -> Option<PigModelVariant> {
    registries
        .resource_id_by_holder_id("minecraft:pig_variant", registry_id)
        .and_then(PigModelVariant::for_resource_id)
}

pub(super) fn cow_model_variant(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> CowModelVariant {
    let Some(accessors) =
        vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(entity_type_id)
    else {
        return CowModelVariant::Temperate;
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == accessors.variant_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CowVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registries) = registries {
                cow_variant_from_registry_id(registries, id).unwrap_or_else(|| {
                    cow_default_model_variant(accessors.variant_default_resource_id)
                })
            } else {
                CowModelVariant::for_vanilla_registry_id(id).unwrap_or_else(|| {
                    cow_default_model_variant(accessors.variant_default_resource_id)
                })
            }
        })
        .unwrap_or_else(|| cow_default_model_variant(accessors.variant_default_resource_id))
}

fn cow_default_model_variant(default_resource_id: &str) -> CowModelVariant {
    CowModelVariant::for_resource_id(default_resource_id)
        .expect("generated vanilla cow default variant must map to a model")
}

pub(super) fn cow_variant_from_registry_id(
    registries: &dyn EntityVariantRegistryLookup,
    registry_id: i32,
) -> Option<CowModelVariant> {
    registries
        .resource_id_by_holder_id("minecraft:cow_variant", registry_id)
        .and_then(CowModelVariant::for_resource_id)
}

pub(super) fn chested_horse_has_chest(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(entity_type_id)
        .is_some_and(|(data_id, default)| entity_data_bool(values, data_id, default))
}

pub(super) fn zombie_baby(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_zombie_accessors_for_entity_type(entity_type_id)
        .map(|accessors| entity_data_bool(values, accessors.baby_data_id, accessors.baby_default))
        .unwrap_or_default()
}

pub(super) fn piglin_baby(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    bbb_world::vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(values, data_id, default))
        .unwrap_or_default()
}

pub(super) fn bogged_sheared(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(values, data_id, default))
        .unwrap_or_default()
}
