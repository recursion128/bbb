use super::*;

pub(in crate::entity_models) fn boat_textured_layer_passes(
    family: BoatModelFamily,
    chest: bool,
) -> Vec<EntityModelLayerPass> {
    let texture = boat_texture_ref(family, chest);
    let mut passes = vec![EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )];
    if family != BoatModelFamily::Bamboo {
        passes.push(EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::WaterMask,
                texture,
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn conduit_textured_layer_passes(
    part: ConduitModelPart,
) -> [EntityModelLayerPass; 1] {
    let (render_type, texture, is_base) = match part {
        ConduitModelPart::Shell => (
            EntityModelLayerRenderType::EntitySolid,
            CONDUIT_BASE_TEXTURE_REF,
            true,
        ),
        ConduitModelPart::Cage => (
            EntityModelLayerRenderType::EntityCutout,
            CONDUIT_CAGE_TEXTURE_REF,
            true,
        ),
        ConduitModelPart::OuterWind { phase } => (
            EntityModelLayerRenderType::EntityCutout,
            if phase == 1 {
                CONDUIT_WIND_VERTICAL_TEXTURE_REF
            } else {
                CONDUIT_WIND_TEXTURE_REF
            },
            false,
        ),
        ConduitModelPart::InnerWind { vertical } => (
            EntityModelLayerRenderType::EntityCutout,
            if vertical {
                CONDUIT_WIND_VERTICAL_TEXTURE_REF
            } else {
                CONDUIT_WIND_TEXTURE_REF
            },
            false,
        ),
        ConduitModelPart::Eye { open } => (
            EntityModelLayerRenderType::EntityCutout,
            if open {
                CONDUIT_OPEN_EYE_TEXTURE_REF
            } else {
                CONDUIT_CLOSED_EYE_TEXTURE_REF
            },
            false,
        ),
    };
    [EntityModelLayerPass {
        is_base,
        ..EntityModelLayerPass::new(EntityModelLayerKind::InheritOverlay, render_type, texture)
    }]
}

pub(in crate::entity_models) fn breeze_textured_layer_passes() -> [EntityModelLayerPass; 3] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityTranslucent,
            BREEZE_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::BreezeWind,
                BREEZE_WIND_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            visible_when_invisible: true,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                BREEZE_EYES_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn chicken_textured_layer_passes(
    variant: ChickenModelVariant,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        chicken_texture_ref(variant, baby),
    )]
}

pub(in crate::entity_models) fn pig_textured_layer_passes(
    variant: PigModelVariant,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        pig_texture_ref(variant, baby),
    )]
}

pub(in crate::entity_models) fn cow_textured_layer_passes(
    variant: CowModelVariant,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        cow_texture_ref(variant, baby),
    )]
}

pub(in crate::entity_models) fn salmon_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        SALMON_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn tropical_fish_textured_layer_passes(
    shape: TropicalFishModelShape,
    base_color: EntityDyeColor,
    pattern: TropicalFishPattern,
    pattern_color: EntityDyeColor,
) -> [EntityModelLayerPass; 2] {
    let texture = match shape {
        TropicalFishModelShape::Small => TROPICAL_FISH_SMALL_TEXTURE_REF,
        TropicalFishModelShape::Large => TROPICAL_FISH_LARGE_TEXTURE_REF,
    };
    // Vanilla `getModelTint` tints the grayscale base body by the base color's texture diffuse
    // color; the `TropicalFishPatternLayer` then overlays the selected pattern texture (the body
    // mesh inflated by `FISH_PATTERN_DEFORMATION`) tinted by the pattern color.
    [
        EntityModelLayerPass {
            tint: base_color.texture_diffuse_color(),
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntityCutout,
                texture,
            )
        },
        EntityModelLayerPass {
            tint: pattern_color.texture_diffuse_color(),
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityCutout,
                tropical_fish_pattern_texture_ref(pattern),
            )
        },
    ]
}

pub(in crate::entity_models) fn squid_textured_layer_passes(
    glow: bool,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        squid_texture_ref(glow, baby),
    )]
}

pub(in crate::entity_models) fn camel_textured_layer_passes(
    family: CamelModelFamily,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        camel_texture_ref(family, baby),
    )]
}

pub(in crate::entity_models) fn llama_textured_layer_passes(
    variant: LlamaVariant,
    baby: bool,
    // Chest geometry is driven by `LlamaModel::new`; kept for API symmetry.
    _has_chest: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        llama_texture_ref(variant, baby),
    )]
}

pub(in crate::entity_models) fn horse_textured_layer_passes(
    variant: HorseColorVariant,
    baby: bool,
    markings: HorseMarkings,
) -> Vec<EntityModelLayerPass> {
    let mut passes = vec![EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        horse_coat_texture_ref(variant, baby),
    )];
    if let Some(texture) = horse_markings_texture_ref(markings, baby) {
        passes.push(EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityTranslucent,
                texture,
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn donkey_textured_layer_passes(
    family: DonkeyModelFamily,
    baby: bool,
    // Chest visibility changes the `DonkeyModel` tree, not vanilla's texture submit metadata.
    _has_chest: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        donkey_texture_ref(family, baby),
    )]
}

pub(in crate::entity_models) fn undead_horse_textured_layer_passes(
    family: UndeadHorseModelFamily,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        undead_horse_texture_ref(family, baby),
    )]
}

pub(in crate::entity_models) fn creeper_textured_layer_passes() -> [EntityModelLayerPass; 2] {
    let grey = 128.0 / 255.0;
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            CREEPER_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            tint: [grey, grey, grey, 1.0],
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::EnergySwirl,
                CREEPER_ARMOR_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn spider_textured_layer_passes(
    cave: bool,
) -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            if cave {
                CAVE_SPIDER_TEXTURE_REF
            } else {
                SPIDER_TEXTURE_REF
            },
        ),
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            visible_when_invisible: true,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::Eyes,
                SPIDER_EYES_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn enderman_textured_layer_passes() -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            ENDERMAN_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            visible_when_invisible: true,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::Eyes,
                ENDERMAN_EYES_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn end_crystal_textured_layer_passes() -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::NoOverlay,
            EntityModelLayerRenderType::EntityCutout,
            END_CRYSTAL_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EndCrystalBeam,
                // The healing beam is custom geometry from `submitCrystalBeams`, not a baked ModelLayer.
                END_CRYSTAL_BEAM_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn end_gateway_beam_layer_passes() -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::BeaconBeamOpaque,
                END_GATEWAY_BEAM_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::BeaconBeamTranslucent,
                END_GATEWAY_BEAM_TEXTURE_REF,
            )
        },
    ]
}

/// Vanilla `BeaconRenderer.BEAM_LOCATION` through the same
/// `RenderTypes.beaconBeam(location, translucent)` pair the end gateway uses.
pub(in crate::entity_models) fn beacon_beam_layer_passes() -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::BeaconBeamOpaque,
                BEACON_BEAM_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::BeaconBeamTranslucent,
                BEACON_BEAM_TEXTURE_REF,
            )
        },
    ]
}

/// `CopperGolemStatueBlockRenderer.submit`: one `RenderTypes.entityCutout`
/// model pass at the block's weathering texture. There is no emissive eyes
/// layer — that belongs to the living golem's `LivingEntityEmissiveLayer`.
pub(in crate::entity_models) fn copper_golem_statue_block_layer_passes(
    weathering: CopperGolemWeathering,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        copper_golem_texture_ref(weathering),
    )]
}

pub(in crate::entity_models) fn copper_golem_textured_layer_passes(
    weathering: CopperGolemWeathering,
) -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            copper_golem_texture_ref(weathering),
        ),
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::Eyes,
                copper_golem_eyes_texture_ref(weathering),
            )
        },
    ]
}

pub(in crate::entity_models) fn iron_golem_textured_layer_passes(
    crackiness: IronGolemCrackiness,
) -> Vec<EntityModelLayerPass> {
    let mut passes = vec![EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        IRON_GOLEM_TEXTURE_REF,
    )];
    // Vanilla `IronGolemCrackinessLayer`: when cracked, re-render the same mesh with the matching
    // crack texture in a white Cutout overlay (`renderColoredCutoutModel(..., -1, ...)`).
    let crack_texture = match crackiness {
        IronGolemCrackiness::None => None,
        IronGolemCrackiness::Low => Some(IRON_GOLEM_CRACKINESS_LOW_TEXTURE_REF),
        IronGolemCrackiness::Medium => Some(IRON_GOLEM_CRACKINESS_MEDIUM_TEXTURE_REF),
        IronGolemCrackiness::High => Some(IRON_GOLEM_CRACKINESS_HIGH_TEXTURE_REF),
    };
    if let Some(texture) = crack_texture {
        passes.push(EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityCutout,
                texture,
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn snow_golem_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        SNOW_GOLEM_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn witch_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        WITCH_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn slime_textured_layer_passes() -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            SLIME_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::SlimeOuter,
                EntityModelLayerRenderType::EntityTranslucent,
                SLIME_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn magma_cube_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        MAGMA_CUBE_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn ghast_textured_layer_passes(
    charging: bool,
) -> [EntityModelLayerPass; 1] {
    // Vanilla `GhastRenderer.getTextureLocation`: `isCharging` swaps to the open-mouth shooting face.
    let texture = if charging {
        GHAST_SHOOTING_TEXTURE_REF
    } else {
        GHAST_TEXTURE_REF
    };
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )]
}

pub(in crate::entity_models) fn happy_ghast_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            HAPPY_GHAST_BABY_TEXTURE_REF
        } else {
            HAPPY_GHAST_TEXTURE_REF
        },
    )]
}

pub(in crate::entity_models) fn minecart_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::EntityCutout,
        MINECART_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn zombie_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    // The unified `ZombieModel` tree drives the geometry, so the layer-pass parts are vestigial (`&[]`).
    let texture = if baby {
        ZOMBIE_BABY_TEXTURE_REF
    } else {
        ZOMBIE_TEXTURE_REF
    };
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )]
}

pub(in crate::entity_models) fn husk_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `HuskRenderer extends ZombieRenderer`: it reuses `ZombieModel`/`BabyZombieModel`
    // geometry (`ModelLayers.HUSK` is `humanoidBodyLayer.apply(huskScale)` and `HUSK_BABY` is the
    // shared `babyZombieLayer`), so the husk reuses the unified `ZombieVariantModel` tree; only the
    // texture (`husk.png`/`husk_baby.png`) and the adult's 1.0625 mesh scale differ. The layer-pass
    // geometry is vestigial (`&[]`).
    let texture = if baby {
        HUSK_BABY_TEXTURE_REF
    } else {
        HUSK_TEXTURE_REF
    };
    vec![EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )]
}

pub(in crate::entity_models) fn drowned_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `DrownedModel.createBodyLayer extends ZombieModel`; the non-swimming drowned reuses the
    // unified `ZombieVariantModel` (plain-zombie) tree for the base body. The base body and the outer
    // layer both come from unified model trees, so the layer-pass parts are vestigial (`&[]`).
    let texture = if baby {
        DROWNED_BABY_TEXTURE_REF
    } else {
        DROWNED_TEXTURE_REF
    };
    // Vanilla `DrownedOuterLayer`: an always-on white cutout copy of the inflated drowned shell. The
    // adult re-renders `DrownedModel.createBodyLayer(0.25)` over `drowned_outer_layer.png`; the baby
    // re-renders the distinct `BabyDrownedModel.createBodyLayer(0.25)` (baby-zombie inflated mesh) over
    // `drowned_outer_layer_baby.png`.
    let outer_texture = if baby {
        DROWNED_OUTER_LAYER_BABY_TEXTURE_REF
    } else {
        DROWNED_OUTER_LAYER_TEXTURE_REF
    };
    vec![
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            texture,
        ),
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityCutout,
                outer_texture,
            )
        },
    ]
}

pub(in crate::entity_models) fn zombie_villager_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `ZombieVillagerModel.createBodyLayer` / `BabyZombieVillagerModel.createBodyLayer`
    // (the hatted base layer; the no-hat model selection and the profession/type/level overlays
    // stay deferred). The unified `ZombieVariantModel` tree drives the geometry, so the layer-pass
    // parts are vestigial (`&[]`).
    let texture = if baby {
        ZOMBIE_VILLAGER_BABY_TEXTURE_REF
    } else {
        ZOMBIE_VILLAGER_TEXTURE_REF
    };
    vec![EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )]
}

pub(in crate::entity_models) fn zombie_villager_data_textured_layer_passes(
    baby: bool,
    data: VillagerModelData,
) -> Vec<EntityModelLayerPass> {
    let mut passes = vec![EntityModelLayerPass {
        order: 1,
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::ZeroWhiteOverlay,
            EntityModelLayerRenderType::EntityCutout,
            zombie_villager_type_texture_ref(data.villager_type, baby),
        )
    }];
    if baby {
        return passes;
    }
    let Some(profession_texture) = zombie_villager_profession_texture_ref(data.profession) else {
        return passes;
    };
    passes.push(EntityModelLayerPass {
        order: 2,
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::ZeroWhiteOverlay,
            EntityModelLayerRenderType::EntityCutout,
            profession_texture,
        )
    });
    if villager_profession_renders_level_badge(data.profession) {
        passes.push(EntityModelLayerPass {
            order: 3,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityCutout,
                zombie_villager_level_texture_ref(data.level),
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn piglin_textured_layer_passes(
    family: PiglinModelFamily,
    baby_layout: bool,
) -> [EntityModelLayerPass; 1] {
    // All piglin families share `AbstractPiglinModel` geometry (`AdultZombifiedPiglinModel` /
    // `BabyZombifiedPiglinModel` forward to the piglin layers, and the brute reuses the adult
    // layer); only the model layer key and texture differ. `baby_layout` selects the baby parts
    // (the brute is never baby) in the unified `PiglinModel` tree the emitter walks; the held-vs-swung
    // arms are decided by the emitter, so the layer-pass geometry is vestigial (`&[]`).
    let texture = match (family, baby_layout) {
        (PiglinModelFamily::Piglin, false) => PIGLIN_TEXTURE_REF,
        (PiglinModelFamily::Piglin, true) => PIGLIN_BABY_TEXTURE_REF,
        (PiglinModelFamily::PiglinBrute, _) => PIGLIN_BRUTE_TEXTURE_REF,
        (PiglinModelFamily::ZombifiedPiglin, false) => ZOMBIFIED_PIGLIN_TEXTURE_REF,
        (PiglinModelFamily::ZombifiedPiglin, true) => ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
    };
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )]
}

pub(in crate::entity_models) fn illager_textured_layer_passes(
    family: IllagerModelFamily,
) -> [EntityModelLayerPass; 1] {
    // Vanilla `IllagerModel.createBodyLayer` is shared across all four illagers; only the model
    // layer key and texture differ, plus per-renderer visibility: the pillager shows its separate
    // (swinging) arms, the evoker/vindicator show the static folded `arms` part, and the
    // illusioner additionally re-enables the head hat (`this.model.getHat().visible = true`).
    let texture = match family {
        IllagerModelFamily::Evoker => EVOKER_TEXTURE_REF,
        IllagerModelFamily::Vindicator => VINDICATOR_TEXTURE_REF,
        IllagerModelFamily::Illusioner => ILLUSIONER_TEXTURE_REF,
        IllagerModelFamily::Pillager => PILLAGER_TEXTURE_REF,
    };
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )]
}

pub(in crate::entity_models) fn blaze_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        BLAZE_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn wind_charge_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::BreezeWind,
        WIND_CHARGE_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn endermite_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        ENDERMITE_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn silverfish_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        SILVERFISH_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn leash_knot_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::EntityCutout,
        LEASH_KNOT_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn trident_textured_layer_passes() -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::NoOverlay,
            EntityModelLayerRenderType::EntityCutout,
            TRIDENT_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::EntityGlint,
                ENCHANTED_GLINT_ITEM_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn evoker_fangs_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::EntityCutout,
        EVOKER_FANGS_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn tadpole_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        TADPOLE_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn creaking_textured_layer_passes(
    eyes_glowing: bool,
) -> Vec<EntityModelLayerPass> {
    const CREAKING_EYES_PARTS: &[&str] = &["head"];

    let mut passes = vec![EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        CREAKING_TEXTURE_REF,
    )];
    // Vanilla `CreakingRenderer`'s `LivingEntityEmissiveLayer`: an active creaking re-renders the
    // `createEyesLayer` head subset with `creaking_eyes.png` in the eyes render type (alpha `1.0`
    // when glowing).
    if eyes_glowing {
        passes.push(EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(CREAKING_EYES_PARTS),
            order: 1,
            is_base: false,
            visible_when_invisible: true,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::Eyes,
                CREAKING_EYES_TEXTURE_REF,
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn sniffer_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            SNIFFLET_TEXTURE_REF
        } else {
            SNIFFER_TEXTURE_REF
        },
    )]
}

pub(in crate::entity_models) fn parrot_textured_layer_passes(
    variant: ParrotModelVariant,
) -> [EntityModelLayerPass; 1] {
    // The five parrot colours share one model and differ only by texture
    // (`ParrotRenderer.getVariantTexture`).
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        parrot_texture_ref(variant),
    )]
}

pub(in crate::entity_models) fn shulker_textured_layer_passes(
    color: Option<EntityDyeColor>,
) -> [EntityModelLayerPass; 1] {
    // `ShulkerRenderer.getTextureLocation`: the default `shulker.png` when uncolored, else the dyed
    // `shulker_<color>.png`.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutoutZOffset,
        shulker_texture_ref(color),
    )]
}

pub(in crate::entity_models) fn ender_dragon_textured_layer_passes(
    death_time: f32,
) -> [EntityModelLayerPass; 3] {
    let dying_alpha = (1.0 - death_time / 200.0).clamp(0.0, 1.0);
    let dying = death_time > 0.0;
    [
        EntityModelLayerPass {
            tint: [1.0, 1.0, 1.0, dying_alpha],
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                if dying {
                    EntityModelLayerRenderType::EntityCutoutDissolve
                } else {
                    EntityModelLayerRenderType::EntityCutout
                },
                ENDER_DRAGON_TEXTURE_REF,
            )
        },
        // Vanilla `EnderDragonRenderer` always re-submits the whole model with the emissive
        // `dragon_eyes.png` in the eyes render type before the optional healing beam custom geometry.
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::Eyes,
                ENDER_DRAGON_EYES_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EndCrystalBeam,
                // The healing beam is custom geometry from `submitCrystalBeams`, not a baked ModelLayer.
                END_CRYSTAL_BEAM_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn nautilus_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            NAUTILUS_BABY_TEXTURE_REF
        } else {
            NAUTILUS_TEXTURE_REF
        },
    )]
}

pub(in crate::entity_models) fn zombie_nautilus_textured_layer_passes(
    variant: ZombieNautilusModelVariant,
) -> [EntityModelLayerPass; 1] {
    // Vanilla `ZombieNautilusRenderer`: the `NORMAL` variant textures the shared adult `NautilusModel`
    // body with `zombie_nautilus.png`; the `WARM` variant textures the `ZombieNautilusCoralModel` with
    // `zombie_nautilus_coral.png`.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        variant.builtin_texture_ref(),
    )]
}

pub(in crate::entity_models) fn panda_textured_layer_passes(
    variant: PandaModelVariant,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    // The seven genes share one `PandaModel` / `BabyPandaModel` and differ only by texture × age.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        panda_texture_ref(variant, baby),
    )]
}

pub(in crate::entity_models) fn axolotl_textured_layer_passes(
    variant: AxolotlModelVariant,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    // `AxolotlRenderer.getTextureLocation` picks the colour × age cell from `TEXTURE_BY_TYPE`.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        axolotl_texture_ref(variant, baby),
    )]
}

pub(in crate::entity_models) fn fox_textured_layer_passes(
    variant: FoxModelVariant,
    baby: bool,
    sleeping: bool,
) -> [EntityModelLayerPass; 1] {
    // `FoxRenderer.getTextureLocation` picks the {red, snow} × {adult, baby} × {idle, sleeping} cell.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        fox_texture_ref(variant, baby, sleeping),
    )]
}

pub(in crate::entity_models) fn rabbit_textured_layer_passes(
    variant: RabbitModelVariant,
    baby: bool,
    toast: bool,
) -> [EntityModelLayerPass; 1] {
    // `RabbitRenderer.getTextureLocation` picks the colour × age cell, overridden by `toast`/
    // `toast_baby` for the `Toast` named rabbit.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        rabbit_texture_ref(variant, baby, toast),
    )]
}

pub(in crate::entity_models) fn feline_textured_layer_passes(
    cat: bool,
    baby: bool,
    cat_variant: CatModelVariant,
    collar: Option<EntityDyeColor>,
) -> Vec<EntityModelLayerPass> {
    // The cat and ocelot share `AbstractFelineModel`, so the base pass differs only in which image it
    // binds: the per-breed `CatVariant` texture for cats, the `ocelot` texture otherwise.
    let mut passes = vec![EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        feline_texture_ref(cat, baby, cat_variant),
    )];
    // Vanilla `CatCollarLayer`: a tame cat re-renders the mesh with `cat_collar.png` tinted by the
    // dye's diffuse color (`collar` is already gated on cat && tame; the ocelot never carries one).
    if let Some(collar) = collar {
        passes.push(EntityModelLayerPass {
            tint: collar.texture_diffuse_color(),
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityCutout,
                feline_collar_texture_ref(baby),
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn mooshroom_textured_layer_passes(
    baby: bool,
    variant: MooshroomVariant,
) -> [EntityModelLayerPass; 1] {
    // The mooshroom reuses the cow model tree (geometry drives off it), so this binds only the
    // mooshroom recolor (the red/brown variant face) over the shared cow UVs.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        mooshroom_texture_ref(baby, variant),
    )]
}

/// Vanilla `ChestRenderer.submit`: one `submitModel` per chest block entity with the
/// `Sheets.chooseSprite(material, type)` sprite on the chest sheet and the model's
/// `RenderTypes::entityCutoutCull` render type (`ChestModel`'s constructor).
pub(in crate::entity_models) fn chest_textured_layer_passes(
    texture: ChestModelTexture,
    half: ChestModelHalf,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutoutCull,
        chest_texture_ref(texture, half),
    )]
}

/// Vanilla `AbstractSignRenderer.submitSign`: one `submitModel` per sign
/// block entity with the wood's sign-sheet sprite and the model's
/// `RenderTypes::entityCutout` render type (`StandingSignRenderer.createSignModel`
/// / `HangingSignRenderer.createSignModel` both build
/// `Model.Simple(_, RenderTypes::entityCutout)`).
pub(in crate::entity_models) fn sign_textured_layer_passes(
    wood: SignModelWood,
    attachment: SignModelAttachment,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        sign_texture_ref(wood, attachment),
    )]
}

/// Vanilla `BedRenderer.submitPiece`: one `submitModel` per bed block entity with the
/// `Sheets.getBedSprite(color)` sprite on the bed sheet and the piece model's
/// `RenderTypes::entitySolid` render type (`BedRenderer`'s `Model.Simple(bakeLayer(BED_HEAD /
/// BED_FOOT), RenderTypes::entitySolid)`).
pub(in crate::entity_models) fn bed_textured_layer_passes(
    color: EntityDyeColor,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntitySolid,
        bed_texture_ref(color),
    )]
}

/// Vanilla `BellRenderer.submit`: one `submitModel` per bell block entity with the
/// `bell/bell_body` block-entity sprite and the model's `RenderTypes::entitySolid` render type
/// (`BellModel`'s constructor `super(root, RenderTypes::entitySolid)`).
pub(in crate::entity_models) fn bell_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntitySolid,
        BELL_BODY_TEXTURE_REF,
    )]
}

/// Vanilla `EnchantTableRenderer.submit` / `LecternRenderer.submit`: one `submitModel` per book
/// with the `enchantment/enchanting_table_book` sprite and the `BookModel`'s
/// `RenderTypes::entitySolid` render type (`BookModel`'s constructor). The
/// enchanting-table and lectern books share the model, sprite, and render type; only the
/// render-state animation and the root transform differ.
pub(in crate::entity_models) fn book_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntitySolid,
        BOOK_TEXTURE_REF,
    )]
}

/// Vanilla `ShulkerBoxRenderer.submit`: one `submitModel` per shulker box block entity with the
/// `Sheets.getShulkerBoxSprite(color)` sprite (the undyed box binding
/// `DEFAULT_SHULKER_TEXTURE_LOCATION` — the same `entity/shulker/*` sheet family the mob uses)
/// and the model's `RenderTypes::entityCutout` render type
/// (`ShulkerBoxModel`'s constructor).
pub(in crate::entity_models) fn shulker_box_textured_layer_passes(
    color: Option<EntityDyeColor>,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        shulker_texture_ref(color),
    )]
}

/// Vanilla `DecoratedPotRenderer.submit`: five
/// `submitModelPart` calls, all on `RenderTypes::entitySolid` — `neck`/`top`/`bottom` with the
/// `Sheets.DECORATED_POT_BASE` sprite, then each side with its own
/// `getSideSprite(decorations.<side>())` (the sherd's pattern sheet, or the plain
/// `DECORATED_POT_SIDE` for an empty/unknown face). bbb keeps one pot model per instance, so the
/// per-part texture split rides retained-parts passes over the same posed tree.
pub(in crate::entity_models) fn decorated_pot_textured_layer_passes(
    back: Option<DecoratedPotPattern>,
    left: Option<DecoratedPotPattern>,
    right: Option<DecoratedPotPattern>,
    front: Option<DecoratedPotPattern>,
) -> [EntityModelLayerPass; 5] {
    let base = EntityModelLayerPass {
        visibility: EntityModelLayerVisibility::RetainedParts(DECORATED_POT_BASE_PARTS),
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntitySolid,
            DECORATED_POT_BASE_TEXTURE_REF,
        )
    };
    let [back, left, right, front] = [
        (back, DECORATED_POT_BACK_PART),
        (left, DECORATED_POT_LEFT_PART),
        (right, DECORATED_POT_RIGHT_PART),
        (front, DECORATED_POT_FRONT_PART),
    ]
    .map(|(pattern, retained)| EntityModelLayerPass {
        visibility: EntityModelLayerVisibility::RetainedParts(retained),
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntitySolid,
            decorated_pot_side_texture_ref(pattern),
        )
    });
    [base, back, left, right, front]
}

/// Vanilla `BannerRenderer.submitBanner`: the `BannerModel`
/// frame and the `BannerFlagModel` flag submit untinted `entitySolid` over `Sheets.BANNER_BASE`,
/// then `submitPatterns` re-submits the flag geometry per layer through
/// `sprite.renderType(RenderTypes::bannerPattern)` — `Sheets.BANNER_PATTERN_BASE` tinted by the
/// base color first, then each `BannerPatternLayers.Layer`'s sheet tinted by
/// `layer.color().getTextureDiffuseColor()`, clamped at `MAX_PATTERNS = 16`. bbb keeps one
/// banner model per instance, so the frame/flag/pattern split rides retained-parts passes over
/// the same posed tree; the pattern passes retain the dedicated `bannerPattern`
/// render identity while sharing the compatible translucent vertex bucket.
pub(in crate::entity_models) fn banner_textured_layer_passes(
    wall: bool,
    flag_only: bool,
    base_color: EntityDyeColor,
    layers: &[Option<BannerPatternLayer>; 16],
) -> Vec<EntityModelLayerPass> {
    let frame_parts = if wall {
        WALL_BANNER_FRAME_PARTS
    } else {
        STANDING_BANNER_FRAME_PARTS
    };
    let mut passes = Vec::with_capacity(3 + layers.iter().flatten().count());
    if !flag_only {
        passes.push(EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(frame_parts),
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntitySolid,
                BANNER_BASE_TEXTURE_REF,
            )
        });
    }
    passes.extend([
        EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(BANNER_FLAG_PARTS),
            is_base: flag_only,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntitySolid,
                BANNER_BASE_TEXTURE_REF,
            )
        },
        // `submitPatterns`' first layer: the full-flag `base` mask tinted by the banner's base
        // color.
        EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(BANNER_FLAG_PARTS),
            tint: base_color.texture_diffuse_color(),
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::BannerPattern,
                BANNER_PATTERN_BASE_TEXTURE_REF,
            )
        },
    ]);
    for layer in layers.iter().flatten() {
        passes.push(EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(BANNER_FLAG_PARTS),
            tint: layer.color.texture_diffuse_color(),
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::BannerPattern,
                match layer.texture {
                    BannerPatternTexture::Builtin(pattern) => banner_pattern_texture_ref(pattern),
                    // The runtime atlas supplies the real sprite. Keeping the base-mask ref here
                    // provides a valid static-atlas fallback if the dynamic handle disappears.
                    BannerPatternTexture::Runtime(_) => BANNER_PATTERN_BASE_TEXTURE_REF,
                },
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn arrow_textured_layer_passes(
    texture: ArrowModelTexture,
) -> [EntityModelLayerPass; 1] {
    // One model shared by the normal / tipped / spectral arrow; only the bound image differs.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::EntityCutoutCull,
        arrow_texture_ref(texture),
    )]
}

pub(in crate::entity_models) fn llama_spit_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::EntityCutout,
        LLAMA_SPIT_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn shulker_bullet_textured_layer_passes() -> [EntityModelLayerPass; 2]
{
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::NoOverlay,
            EntityModelLayerRenderType::EntityCutout,
            SHULKER_BULLET_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            tint: [1.0, 1.0, 1.0, 38.0 / 255.0],
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::EntityTranslucent,
                SHULKER_BULLET_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn wither_skull_textured_layer_passes(
    dangerous: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::EntityTranslucent,
        wither_skull_texture_ref(dangerous),
    )]
}

pub(in crate::entity_models) fn wither_textured_layer_passes(
    invulnerable_ticks: f32,
) -> [EntityModelLayerPass; 2] {
    // Vanilla `WitherBossRenderer.getTextureLocation`: `i = floor(invulnerableTicks)`; the
    // `wither_invulnerable.png` armor texture shows while `i > 0 && (i > 80 || i / 5 % 2 != 1)` — so a
    // freshly-summoned wither is solid invulnerable above 80 ticks, then flickers every 5 ticks back
    // to `wither.png` as it nears spawn. The wither boss and the wither skull otherwise share
    // `wither.png`.
    let i = invulnerable_ticks.floor() as i32;
    let texture = if i > 0 && (i > 80 || i / 5 % 2 != 1) {
        WITHER_INVULNERABLE_TEXTURE_REF
    } else {
        WITHER_TEXTURE_REF
    };
    let grey = 128.0 / 255.0;
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            texture,
        ),
        EntityModelLayerPass {
            tint: [grey, grey, grey, 1.0],
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::EnergySwirl,
                WITHER_ARMOR_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn guardian_textured_layer_passes(
    elder: bool,
) -> [EntityModelLayerPass; 2] {
    // The guardian and elder guardian share one body mesh; the elder uses a distinct vanilla
    // ModelLayer and root-scale. `GuardianRenderer.submit` then appends custom beam geometry with
    // `RenderTypes.entityCutout(guardian_beam.png)` on the same collector.
    let texture = if elder {
        GUARDIAN_ELDER_TEXTURE_REF
    } else {
        GUARDIAN_TEXTURE_REF
    };
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            texture,
        ),
        EntityModelLayerPass {
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntityCutout,
                // The beam is custom geometry, not a baked ModelLayer.
                GUARDIAN_BEAM_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn armadillo_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    // The adult and baby armadillo share the UV layout; the baby binds its own texture.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            ARMADILLO_BABY_TEXTURE_REF
        } else {
            ARMADILLO_TEXTURE_REF
        },
    )]
}

pub(in crate::entity_models) fn frog_textured_layer_passes(
    variant: FrogModelVariant,
) -> [EntityModelLayerPass; 1] {
    // The frog binds its temperature-variant base texture; all three share one `FrogModel` geometry.
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        frog_texture_ref(variant),
    )]
}

pub(in crate::entity_models) fn warden_textured_layer_passes(
    age_in_ticks: f32,
    tendril_animation: f32,
    heart_animation: f32,
) -> [EntityModelLayerPass; 6] {
    // The warden's base body, then the five `WardenEmissiveLayer`s — each an
    // entityTranslucentEmissive pass over its own part subset: the always-on bioluminescent overlay
    // (alpha 1.0, head/arms/legs); the two pulsating-spots overlays (body/legs, each fading on its
    // own `cos(ageInTicks · 0.045)` pulse); the tendril overlay, which reuses the base `warden.png`
    // over the two tendril planes at the lerped `tendrilAnimation` alpha; and the heart overlay
    // (body only) at the lerped `heartAnimation` alpha.
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            WARDEN_TEXTURE_REF,
        ),
        EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(WARDEN_BIOLUMINESCENT_PARTS),
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                WARDEN_BIOLUMINESCENT_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(WARDEN_PULSATING_SPOTS_PARTS),
            tint: [
                1.0,
                1.0,
                1.0,
                warden_pulsating_spots_alpha(age_in_ticks, 0.0),
            ],
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                WARDEN_PULSATING_SPOTS_1_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(WARDEN_PULSATING_SPOTS_PARTS),
            tint: [
                1.0,
                1.0,
                1.0,
                warden_pulsating_spots_alpha(age_in_ticks, std::f32::consts::PI),
            ],
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                WARDEN_PULSATING_SPOTS_2_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(WARDEN_TENDRILS_PARTS),
            tint: [1.0, 1.0, 1.0, tendril_animation],
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                WARDEN_TEXTURE_REF,
            )
        },
        EntityModelLayerPass {
            visibility: EntityModelLayerVisibility::RetainedParts(WARDEN_HEART_PARTS),
            tint: [1.0, 1.0, 1.0, heart_animation],
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                WARDEN_HEART_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn phantom_textured_layer_passes() -> [EntityModelLayerPass; 2] {
    [
        EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            PHANTOM_TEXTURE_REF,
        ),
        // Vanilla `PhantomEyesLayer` (an `EyesLayer`): the whole model is re-rendered with
        // the emissive `phantom_eyes.png` in the eyes render type.
        EntityModelLayerPass {
            order: 1,
            is_base: false,
            visible_when_invisible: true,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::Eyes,
                PHANTOM_EYES_TEXTURE_REF,
            )
        },
    ]
}

pub(in crate::entity_models) fn polar_bear_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            POLAR_BEAR_BABY_TEXTURE_REF
        } else {
            POLAR_BEAR_TEXTURE_REF
        },
    )]
}

pub(in crate::entity_models) fn ravager_textured_layer_passes() -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        RAVAGER_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn villager_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            VILLAGER_BABY_TEXTURE_REF
        } else {
            VILLAGER_TEXTURE_REF
        },
    )]
}

pub(in crate::entity_models) fn villager_data_textured_layer_passes(
    baby: bool,
    data: VillagerModelData,
) -> Vec<EntityModelLayerPass> {
    let mut passes = vec![EntityModelLayerPass {
        order: 1,
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::ZeroWhiteOverlay,
            EntityModelLayerRenderType::EntityCutout,
            villager_type_texture_ref(data.villager_type, baby),
        )
    }];
    if baby {
        return passes;
    }
    let Some(profession_texture) = villager_profession_texture_ref(data.profession) else {
        return passes;
    };
    passes.push(EntityModelLayerPass {
        order: 2,
        is_base: false,
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::ZeroWhiteOverlay,
            EntityModelLayerRenderType::EntityCutout,
            profession_texture,
        )
    });
    if villager_profession_renders_level_badge(data.profession) {
        passes.push(EntityModelLayerPass {
            order: 3,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityCutout,
                villager_level_texture_ref(data.level),
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn wandering_trader_textured_layer_passes() -> [EntityModelLayerPass; 1]
{
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        WANDERING_TRADER_TEXTURE_REF,
    )]
}

pub(in crate::entity_models) fn hoglin_textured_layer_passes(
    family: HoglinModelFamily,
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    let texture = match (family, baby) {
        (HoglinModelFamily::Hoglin, false) => HOGLIN_TEXTURE_REF,
        (HoglinModelFamily::Hoglin, true) => HOGLIN_BABY_TEXTURE_REF,
        (HoglinModelFamily::Zoglin, false) => ZOGLIN_TEXTURE_REF,
        (HoglinModelFamily::Zoglin, true) => ZOGLIN_BABY_TEXTURE_REF,
    };
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        texture,
    )]
}

#[cfg(test)]
pub(in crate::entity_models) fn player_textured_layer_passes(
    slim: bool,
    parts: PlayerModelPartVisibility,
) -> [EntityModelLayerPass; 1] {
    player_textured_layer_passes_with_texture(parts, player_texture_ref(slim))
}

pub(in crate::entity_models) fn sheep_textured_layer_passes(
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    jeb: bool,
    age_ticks: f32,
) -> Vec<EntityModelLayerPass> {
    let wool_tint = sheep_wool_render_color(wool_color, jeb, age_ticks);
    let mut passes = Vec::with_capacity(3);
    passes.push(EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            SHEEP_BABY_TEXTURE_REF
        } else {
            SHEEP_TEXTURE_REF
        },
    ));
    if !sheared {
        passes.push(EntityModelLayerPass {
            tint: wool_tint,
            order: if baby { 1 } else { 0 },
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::SheepWool,
                EntityModelLayerRenderType::EntityCutout,
                if baby {
                    SHEEP_WOOL_BABY_TEXTURE_REF
                } else {
                    SHEEP_WOOL_TEXTURE_REF
                },
            )
        });
    }
    if !baby && (jeb || wool_color != SheepWoolColor::White) {
        passes.push(EntityModelLayerPass {
            tint: wool_tint,
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::ZeroWhiteOverlay,
                EntityModelLayerRenderType::EntityCutout,
                SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn wolf_textured_layer_passes(
    baby: bool,
    tame: bool,
    angry: bool,
    collar_color: Option<EntityDyeColor>,
    variant: WolfModelVariant,
    wet_shade: f32,
) -> Vec<EntityModelLayerPass> {
    let mut passes = Vec::with_capacity(2);
    passes.push(EntityModelLayerPass {
        tint: [wet_shade, wet_shade, wet_shade, 1.0],
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            wolf_texture_ref(baby, tame, angry, variant),
        )
    });
    if let Some(collar_color) = tame.then_some(collar_color).flatten() {
        passes.push(EntityModelLayerPass {
            tint: collar_color.texture_diffuse_color(),
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::NoOverlay,
                EntityModelLayerRenderType::EntityCutout,
                if baby {
                    WOLF_BABY_COLLAR_TEXTURE_REF
                } else {
                    WOLF_COLLAR_TEXTURE_REF
                },
            )
        });
    }
    passes
}

pub(in crate::entity_models) fn goat_textured_layer_passes(
    baby: bool,
) -> [EntityModelLayerPass; 1] {
    [EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        goat_texture_ref(baby),
    )]
}

pub(in crate::entity_models) fn skeleton_textured_layer_passes(
    family: Option<SkeletonModelFamily>,
) -> Vec<EntityModelLayerPass> {
    // Both the base body and the clothing overlay come from unified model trees (the base
    // `SkeletonModel` and the textured-only `SkeletonClothingModel`), so the layer-pass parts are
    // vestigial (`&[]`).
    let mut passes = vec![EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        skeleton_texture_ref(family),
    )];
    if let Some(texture) = skeleton_clothing_layer_pass(family) {
        passes.push(EntityModelLayerPass {
            order: 1,
            is_base: false,
            ..EntityModelLayerPass::new(
                EntityModelLayerKind::SkeletonClothing,
                EntityModelLayerRenderType::EntityCutout,
                texture,
            )
        });
    }
    passes
}
