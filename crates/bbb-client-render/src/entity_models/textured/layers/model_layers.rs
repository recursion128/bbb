use super::*;

pub(in crate::entity_models) fn bed_model_layer(part: BedModelPart) -> ModelLayer {
    match part {
        BedModelPart::Head => ModelLayer::BED_HEAD,
        BedModelPart::Foot => ModelLayer::BED_FOOT,
    }
}

pub(super) const DECORATED_POT_BASE_PARTS: &[&str] = &["neck", "top", "bottom"];

pub(super) const DECORATED_POT_BACK_PART: &[&str] = &["back"];

pub(super) const DECORATED_POT_LEFT_PART: &[&str] = &["left"];

pub(super) const DECORATED_POT_RIGHT_PART: &[&str] = &["right"];

pub(super) const DECORATED_POT_FRONT_PART: &[&str] = &["front"];

/// Vanilla `WardenRenderer`'s pulsating-spots `alphaFunction`: `max(0, cos(ageInTicks · 0.045 +
/// phase) · 0.25)`. The two layers use phase `0` and `π` so they fade in and out in opposition.
pub(in crate::entity_models) fn warden_pulsating_spots_alpha(age_in_ticks: f32, phase: f32) -> f32 {
    ((age_in_ticks * 0.045 + phase).cos() * 0.25).max(0.0)
}

// The warden emissive overlays are each baked by `retainExactParts` over `WardenModel.createBodyLayer`,
// so each draws only its named parts (a retained ancestor short-circuits its descendants — vanilla
// `clearRecursively`). `createBioluminescentLayer` keeps the head, arms, and legs; `createPulsatingSpotsLayer`
// adds the body (whose retention drops its head/arm children, leaving body + legs); `createTendrilsLayer`
// keeps only the two head tendrils; `createHeartLayer` keeps only the body.
pub(super) const WARDEN_BIOLUMINESCENT_PARTS: &[&str] =
    &["head", "left_arm", "right_arm", "left_leg", "right_leg"];

pub(super) const WARDEN_PULSATING_SPOTS_PARTS: &[&str] = &[
    "body",
    "head",
    "left_arm",
    "right_arm",
    "left_leg",
    "right_leg",
];

pub(super) const WARDEN_TENDRILS_PARTS: &[&str] = &["left_tendril", "right_tendril"];

pub(super) const WARDEN_HEART_PARTS: &[&str] = &["body"];

pub(in crate::entity_models) fn villager_type_hat_visible(
    data: VillagerModelData,
    zombie: bool,
) -> bool {
    let type_hat = if zombie {
        VillagerModelHat::None
    } else {
        villager_type_hat(data.villager_type)
    };
    let profession_hat = villager_profession_hat(data.profession);
    profession_hat == VillagerModelHat::None
        || profession_hat == VillagerModelHat::Partial && type_hat != VillagerModelHat::Full
}

pub(in crate::entity_models) fn player_extra_ears_layer_pass_with_texture(
    texture: EntityModelTextureRef,
) -> EntityModelLayerPass {
    EntityModelLayerPass {
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::ZeroWhiteOverlay,
            EntityModelLayerRenderType::EntitySolid,
            texture,
        )
    }
}

pub(in crate::entity_models) fn player_cape_layer_pass() -> EntityModelLayerPass {
    EntityModelLayerPass {
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::NoOverlay,
            EntityModelLayerRenderType::EntitySolid,
            PLAYER_PROFILE_CAPE_TEXTURE_REF,
        )
    }
}

pub(in crate::entity_models) fn player_parrot_on_shoulder_layer_pass(
    variant: ParrotModelVariant,
) -> EntityModelLayerPass {
    EntityModelLayerPass {
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::NoOverlay,
            EntityModelLayerRenderType::EntityCutout,
            parrot_texture_ref(variant),
        )
    }
}

pub(in crate::entity_models) fn player_spin_attack_effect_layer_pass() -> EntityModelLayerPass {
    EntityModelLayerPass {
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::NoOverlay,
            EntityModelLayerRenderType::EntityCutout,
            TRIDENT_RIPTIDE_TEXTURE_REF,
        )
    }
}

pub(in crate::entity_models) fn custom_head_skull_layer_pass(
    skull: EntityCustomHeadSkull,
    texture: EntityModelTextureRef,
) -> EntityModelLayerPass {
    let render_type = match skull {
        EntityCustomHeadSkull::Player(EntityPlayerSkin::ProfiledDefault(_))
        | EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(_)) => {
            EntityModelLayerRenderType::EntityTranslucent
        }
        _ => EntityModelLayerRenderType::EntityCutoutZOffset,
    };

    EntityModelLayerPass::new(EntityModelLayerKind::NoOverlay, render_type, texture)
}

pub(in crate::entity_models) fn equipment_layer_pass(
    kind: EntityModelLayerKind,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    tint: [f32; 4],
    order: i32,
) -> EntityModelLayerPass {
    EntityModelLayerPass {
        tint,
        order,
        is_base: false,
        ..EntityModelLayerPass::new(kind, render_type, texture)
    }
}

pub(in crate::entity_models) fn boat_model_layer(
    family: BoatModelFamily,
    chest: bool,
) -> ModelLayer {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => ModelLayer::BOAT_ACACIA,
        (BoatModelFamily::Acacia, true) => ModelLayer::CHEST_BOAT_ACACIA,
        (BoatModelFamily::Bamboo, false) => ModelLayer::BOAT_BAMBOO,
        (BoatModelFamily::Bamboo, true) => ModelLayer::CHEST_BOAT_BAMBOO,
        (BoatModelFamily::Birch, false) => ModelLayer::BOAT_BIRCH,
        (BoatModelFamily::Birch, true) => ModelLayer::CHEST_BOAT_BIRCH,
        (BoatModelFamily::Cherry, false) => ModelLayer::BOAT_CHERRY,
        (BoatModelFamily::Cherry, true) => ModelLayer::CHEST_BOAT_CHERRY,
        (BoatModelFamily::DarkOak, false) => ModelLayer::BOAT_DARK_OAK,
        (BoatModelFamily::DarkOak, true) => ModelLayer::CHEST_BOAT_DARK_OAK,
        (BoatModelFamily::Jungle, false) => ModelLayer::BOAT_JUNGLE,
        (BoatModelFamily::Jungle, true) => ModelLayer::CHEST_BOAT_JUNGLE,
        (BoatModelFamily::Mangrove, false) => ModelLayer::BOAT_MANGROVE,
        (BoatModelFamily::Mangrove, true) => ModelLayer::CHEST_BOAT_MANGROVE,
        (BoatModelFamily::Oak, false) => ModelLayer::BOAT_OAK,
        (BoatModelFamily::Oak, true) => ModelLayer::CHEST_BOAT_OAK,
        (BoatModelFamily::PaleOak, false) => ModelLayer::BOAT_PALE_OAK,
        (BoatModelFamily::PaleOak, true) => ModelLayer::CHEST_BOAT_PALE_OAK,
        (BoatModelFamily::Spruce, false) => ModelLayer::BOAT_SPRUCE,
        (BoatModelFamily::Spruce, true) => ModelLayer::CHEST_BOAT_SPRUCE,
    }
}

pub(in crate::entity_models) fn chicken_model_layer(
    variant: ChickenModelVariant,
    baby: bool,
) -> ModelLayer {
    match (variant.builtin_model_type(), baby) {
        (_, true) => ModelLayer::CHICKEN_BABY,
        (ChickenVariantModelType::Cold, false) => ModelLayer::COLD_CHICKEN,
        (ChickenVariantModelType::Normal, false) => ModelLayer::CHICKEN,
    }
}

pub(in crate::entity_models) fn pig_model_layer(
    variant: PigModelVariant,
    baby: bool,
) -> ModelLayer {
    match (variant.builtin_model_type(), baby) {
        (_, true) => ModelLayer::PIG_BABY,
        (PigVariantModelType::Cold, false) => ModelLayer::COLD_PIG,
        (PigVariantModelType::Normal, false) => ModelLayer::PIG,
    }
}

pub(in crate::entity_models) fn cow_model_layer(
    variant: CowModelVariant,
    baby: bool,
) -> ModelLayer {
    match (variant.builtin_model_type(), baby) {
        (CowVariantModelType::Normal, false) => ModelLayer::COW,
        (CowVariantModelType::Normal, true) => ModelLayer::COW_BABY,
        (CowVariantModelType::Warm, false) => ModelLayer::WARM_COW,
        (CowVariantModelType::Warm, true) => ModelLayer::WARM_COW_BABY,
        (CowVariantModelType::Cold, false) => ModelLayer::COLD_COW,
        (CowVariantModelType::Cold, true) => ModelLayer::COLD_COW_BABY,
    }
}

pub(in crate::entity_models) fn panda_model_layer(baby: bool) -> ModelLayer {
    if baby {
        ModelLayer::PANDA_BABY
    } else {
        ModelLayer::PANDA
    }
}

pub(in crate::entity_models) fn fox_model_layer(baby: bool) -> ModelLayer {
    if baby {
        ModelLayer::FOX_BABY
    } else {
        ModelLayer::FOX
    }
}

pub(in crate::entity_models) fn rabbit_model_layer(baby: bool) -> ModelLayer {
    if baby {
        ModelLayer::RABBIT_BABY
    } else {
        ModelLayer::RABBIT
    }
}

pub(in crate::entity_models) fn feline_model_layer(cat: bool, baby: bool) -> ModelLayer {
    match (cat, baby) {
        (true, false) => ModelLayer::CAT,
        (true, true) => ModelLayer::CAT_BABY,
        (false, false) => ModelLayer::OCELOT,
        (false, true) => ModelLayer::OCELOT_BABY,
    }
}

pub(in crate::entity_models) fn feline_collar_model_layer(baby: bool) -> ModelLayer {
    if baby {
        ModelLayer::CAT_BABY_COLLAR
    } else {
        ModelLayer::CAT_COLLAR
    }
}

pub(in crate::entity_models) fn mooshroom_model_layer(baby: bool) -> ModelLayer {
    if baby {
        ModelLayer::MOOSHROOM_BABY
    } else {
        ModelLayer::MOOSHROOM
    }
}

pub(in crate::entity_models) fn horse_model_layer(baby: bool) -> ModelLayer {
    if baby {
        ModelLayer::HORSE_BABY
    } else {
        ModelLayer::HORSE
    }
}

pub(in crate::entity_models) fn donkey_model_layer(
    family: DonkeyModelFamily,
    baby: bool,
) -> ModelLayer {
    match (family, baby) {
        (DonkeyModelFamily::Donkey, false) => ModelLayer::DONKEY,
        (DonkeyModelFamily::Donkey, true) => ModelLayer::DONKEY_BABY,
        (DonkeyModelFamily::Mule, false) => ModelLayer::MULE,
        (DonkeyModelFamily::Mule, true) => ModelLayer::MULE_BABY,
    }
}

pub(super) fn donkey_texture_ref(family: DonkeyModelFamily, baby: bool) -> EntityModelTextureRef {
    match (family, baby) {
        (DonkeyModelFamily::Donkey, false) => DONKEY_TEXTURE_REF,
        (DonkeyModelFamily::Donkey, true) => DONKEY_BABY_TEXTURE_REF,
        (DonkeyModelFamily::Mule, false) => MULE_TEXTURE_REF,
        (DonkeyModelFamily::Mule, true) => MULE_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn undead_horse_model_layer(
    family: UndeadHorseModelFamily,
    baby: bool,
) -> ModelLayer {
    match (family, baby) {
        (UndeadHorseModelFamily::Skeleton, false) => ModelLayer::SKELETON_HORSE,
        (UndeadHorseModelFamily::Skeleton, true) => ModelLayer::SKELETON_HORSE_BABY,
        (UndeadHorseModelFamily::Zombie, false) => ModelLayer::ZOMBIE_HORSE,
        (UndeadHorseModelFamily::Zombie, true) => ModelLayer::ZOMBIE_HORSE_BABY,
    }
}

pub(super) fn undead_horse_texture_ref(
    family: UndeadHorseModelFamily,
    baby: bool,
) -> EntityModelTextureRef {
    match (family, baby) {
        (UndeadHorseModelFamily::Skeleton, false) => SKELETON_HORSE_TEXTURE_REF,
        (UndeadHorseModelFamily::Skeleton, true) => SKELETON_HORSE_BABY_TEXTURE_REF,
        (UndeadHorseModelFamily::Zombie, false) => ZOMBIE_HORSE_TEXTURE_REF,
        (UndeadHorseModelFamily::Zombie, true) => ZOMBIE_HORSE_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn goat_model_layer(baby: bool) -> ModelLayer {
    if baby {
        ModelLayer::GOAT_BABY
    } else {
        ModelLayer::GOAT
    }
}

pub(super) fn goat_texture_ref(baby: bool) -> EntityModelTextureRef {
    if baby {
        GOAT_BABY_TEXTURE_REF
    } else {
        GOAT_TEXTURE_REF
    }
}

pub(in crate::entity_models) fn skeleton_model_layer(
    family: Option<SkeletonModelFamily>,
) -> ModelLayer {
    match family {
        None => ModelLayer::SKELETON,
        Some(SkeletonModelFamily::Stray) => ModelLayer::STRAY,
        Some(SkeletonModelFamily::Parched) => ModelLayer::PARCHED,
        Some(SkeletonModelFamily::WitherSkeleton) => ModelLayer::WITHER_SKELETON,
        Some(SkeletonModelFamily::Bogged { .. }) => ModelLayer::BOGGED,
    }
}

pub(super) fn skeleton_texture_ref(family: Option<SkeletonModelFamily>) -> EntityModelTextureRef {
    match family {
        None => SKELETON_TEXTURE_REF,
        Some(SkeletonModelFamily::Stray) => STRAY_TEXTURE_REF,
        Some(SkeletonModelFamily::Parched) => PARCHED_TEXTURE_REF,
        Some(SkeletonModelFamily::WitherSkeleton) => WITHER_SKELETON_TEXTURE_REF,
        Some(SkeletonModelFamily::Bogged { .. }) => BOGGED_TEXTURE_REF,
    }
}

pub(super) fn skeleton_clothing_layer_pass(
    family: Option<SkeletonModelFamily>,
) -> Option<EntityModelTextureRef> {
    match family {
        Some(SkeletonModelFamily::Stray) => Some(STRAY_OVERLAY_TEXTURE_REF),
        Some(SkeletonModelFamily::Bogged { .. }) => Some(BOGGED_OVERLAY_TEXTURE_REF),
        None | Some(SkeletonModelFamily::Parched) | Some(SkeletonModelFamily::WitherSkeleton) => {
            None
        }
    }
}
