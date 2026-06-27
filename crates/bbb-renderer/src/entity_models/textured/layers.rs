#[cfg(test)]
use super::super::catalog::player_texture_ref;
use super::super::{
    catalog::{
        boat_texture_ref, camel_texture_ref, chicken_texture_ref, cow_texture_ref,
        llama_texture_ref, mooshroom_texture_ref, pig_texture_ref, sheep_wool_render_color,
        wolf_texture_ref, ArrowModelTexture, AxolotlModelVariant, BoatModelFamily,
        CamelModelFamily, CatModelVariant, ChickenModelVariant, CopperGolemWeathering,
        CowModelVariant, EntityDyeColor, EntityModelTextureRef, FoxModelVariant, FrogModelVariant,
        HoglinModelFamily, IllagerModelFamily, IronGolemCrackiness, LlamaVariant, MooshroomVariant,
        PandaModelVariant, ParrotModelVariant, PigModelVariant, PiglinModelFamily,
        PlayerModelPartVisibility, RabbitModelVariant, SalmonModelSize, SheepWoolColor,
        SkeletonModelFamily, TropicalFishModelShape, TropicalFishPattern, WolfModelVariant,
    },
    model_layers::*,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerKind {
    ArmadilloBase,
    ArmorStandBase,
    ArrowBase,
    AxolotlBase,
    BoatBase,
    BreezeEyes,
    CamelBase,
    ChickenBase,
    SalmonBase,
    TropicalFishBase,
    TropicalFishPattern,
    TridentBase,
    CowBase,
    CreeperBase,
    CreakingBase,
    CreakingEyes,
    EnderDragonEyes,
    EndermanBase,
    EndermanEyes,
    EvokerFangsBase,
    FelineBase,
    FoxBase,
    LeashKnotBase,
    CopperGolemBase,
    CopperGolemEyes,
    FrogBase,
    GoatBase,
    HoglinBase,
    LlamaSpitBase,
    LlamaBase,
    MooshroomBase,
    PandaBase,
    IronGolemBase,
    IronGolemCrackiness,
    PigBase,
    PlayerBase,
    SheepBase,
    SheepWool,
    SheepWoolUndercoat,
    SkeletonBase,
    SkeletonClothing,
    SlimeBase,
    SlimeOuter,
    SnifferBase,
    MagmaCubeBase,
    GhastBase,
    HappyGhastBase,
    MinecartBase,
    ParrotBase,
    ZombieBase,
    HuskBase,
    DrownedBase,
    DrownedOuter,
    ZombieVillagerBase,
    PiglinBase,
    RabbitBase,
    IllagerBase,
    BlazeBase,
    EndermiteBase,
    SilverfishBase,
    PhantomBase,
    PhantomEyes,
    PolarBearBase,
    RavagerBase,
    SnowGolemBase,
    SpiderBase,
    SpiderEyes,
    TadpoleBase,
    VillagerBase,
    WanderingTraderBase,
    WitchBase,
    WitherSkullBase,
    WolfBase,
    WolfCollar,
    FelineCollar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerRenderType {
    /// Vanilla `RenderTypes.entitySolid(texture)`.
    EntitySolid,
    /// Vanilla `RenderTypes.armorCutoutNoCull(texture)` used by equipment layers.
    ArmorCutoutNoCull,
    /// Vanilla `RenderTypes.armorTranslucent(texture)` used by translucent armor damage overlays.
    ArmorTranslucent,
    /// Vanilla `RenderTypes.entityCutout(texture)`; this is also the default
    /// `EntityModel` render type when the model does not override it.
    EntityCutout,
    /// Vanilla `RenderTypes.entityCutoutCull(texture)`.
    EntityCutoutCull,
    /// Vanilla `RenderTypes.entityCutoutZOffset(texture)`.
    EntityCutoutZOffset,
    /// Vanilla `RenderTypes.entityTranslucent(texture)`.
    EntityTranslucent,
    /// Vanilla `RenderTypes.entityTranslucentCullItemTarget(texture)`, used by
    /// `LivingEntityRenderer.getRenderType` for invisible entities still visible to this client.
    EntityTranslucentCullItemTarget,
    /// Vanilla `RenderTypes.entityGlint()` item foil overlay.
    EntityGlint,
    /// Vanilla eyes/emissive render type (`EyesLayer` / translucent emissive overlays).
    Eyes,
    /// Vanilla `RenderTypes.breezeWind(texture, u, v)`.
    BreezeWind,
    /// Vanilla `RenderTypes.energySwirl(texture, u, v)`.
    EnergySwirl,
    /// Vanilla `RenderTypes.endCrystalBeam(texture)`.
    EndCrystalBeam,
    /// Vanilla `RenderTypes.waterMask()`, used by the boat water patch.
    WaterMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerRenderBucket {
    Cutout,
    Translucent,
    Eyes,
    Scroll,
    AdditiveScroll,
    DepthOnly,
    GlintOnly,
}

impl EntityModelLayerRenderType {
    #[cfg(test)]
    pub(in crate::entity_models) const ALL: [Self; 14] = [
        Self::EntitySolid,
        Self::ArmorCutoutNoCull,
        Self::ArmorTranslucent,
        Self::EntityCutout,
        Self::EntityCutoutCull,
        Self::EntityCutoutZOffset,
        Self::EntityTranslucent,
        Self::EntityTranslucentCullItemTarget,
        Self::EntityGlint,
        Self::Eyes,
        Self::BreezeWind,
        Self::EnergySwirl,
        Self::EndCrystalBeam,
        Self::WaterMask,
    ];

    pub(in crate::entity_models) const fn mesh_bucket(self) -> EntityModelLayerRenderBucket {
        match self {
            Self::EntitySolid
            | Self::ArmorCutoutNoCull
            | Self::EntityCutout
            | Self::EntityCutoutCull
            | Self::EntityCutoutZOffset => EntityModelLayerRenderBucket::Cutout,
            Self::ArmorTranslucent
            | Self::EntityTranslucent
            | Self::EntityTranslucentCullItemTarget => EntityModelLayerRenderBucket::Translucent,
            Self::EntityGlint => EntityModelLayerRenderBucket::GlintOnly,
            Self::Eyes => EntityModelLayerRenderBucket::Eyes,
            Self::BreezeWind | Self::EndCrystalBeam => EntityModelLayerRenderBucket::Scroll,
            Self::EnergySwirl => EntityModelLayerRenderBucket::AdditiveScroll,
            Self::WaterMask => EntityModelLayerRenderBucket::DepthOnly,
        }
    }

    #[cfg(test)]
    pub(in crate::entity_models) const fn vanilla_name(self) -> &'static str {
        match self {
            Self::EntitySolid => "entitySolid",
            Self::ArmorCutoutNoCull => "armorCutoutNoCull",
            Self::ArmorTranslucent => "armorTranslucent",
            Self::EntityCutout => "entityCutout",
            Self::EntityCutoutCull => "entityCutoutCull",
            Self::EntityCutoutZOffset => "entityCutoutZOffset",
            Self::EntityTranslucent => "entityTranslucent",
            Self::EntityTranslucentCullItemTarget => "entityTranslucentCullItemTarget",
            Self::EntityGlint => "entityGlint",
            Self::Eyes => "eyes",
            Self::BreezeWind => "breezeWind",
            Self::EnergySwirl => "energySwirl",
            Self::EndCrystalBeam => "end_crystal_beam",
            Self::WaterMask => "waterMask",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerVisibility {
    All,
    PlayerParts(PlayerModelPartVisibility),
    /// Render only the cubes of the named parts, mirroring vanilla `PartDefinition.retainExactParts`
    /// (see [`super::super::model::ModelPart::render_textured_retained`]). Used by the per-layer
    /// emissive overlays that vanilla bakes as part subsets of one body mesh (e.g. the warden).
    RetainedParts(&'static [&'static str]),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity_models) struct EntityModelLayerPass {
    pub(in crate::entity_models) kind: EntityModelLayerKind,
    pub(in crate::entity_models) render_type: EntityModelLayerRenderType,
    pub(in crate::entity_models) model_layer: &'static str,
    pub(in crate::entity_models) texture: EntityModelTextureRef,
    pub(in crate::entity_models) visibility: EntityModelLayerVisibility,
    pub(in crate::entity_models) tint: [f32; 4],
    pub(in crate::entity_models) order: i32,
    pub(in crate::entity_models) submit_sequence: u32,
}

impl EntityModelLayerPass {
    /// A single render pass carrying only the fields the renderer consumes (render type, texture,
    /// tint). The routing-only fields (kind/model_layer/visibility/order/submit_sequence)
    /// get placeholder defaults; they are never read for a single-tree uniform entity. Used by the
    /// shared dispatch for entities whose textured render is one plain pass.
    pub(in crate::entity_models) fn base(
        render_type: EntityModelLayerRenderType,
        texture: EntityModelTextureRef,
        tint: [f32; 4],
    ) -> Self {
        Self {
            // `kind` selects which model tree a multi-tree emit walks; a single-tree uniform entity
            // never branches on it, so this is a neutral "base body" placeholder. `PlayerBase` is the
            // canonical base-body layer kind reused here.
            kind: EntityModelLayerKind::PlayerBase,
            render_type,
            // `model_layer` is the vestigial vanilla layer key; the renderer drives geometry off the
            // unified tree and never reads it, so the empty string is a safe placeholder.
            model_layer: "",
            texture,
            visibility: EntityModelLayerVisibility::All,
            tint,
            order: 0,
            submit_sequence: 0,
        }
    }

    /// Like [`EntityModelLayerPass::base`] but renders only the cubes of the named parts (vanilla
    /// `retainExactParts`), for an emissive overlay baked as a part subset of one body mesh. The
    /// routing-only fields keep their `base` placeholders; only `visibility` carries the subset.
    pub(in crate::entity_models) fn retained(
        render_type: EntityModelLayerRenderType,
        texture: EntityModelTextureRef,
        tint: [f32; 4],
        parts: &'static [&'static str],
    ) -> Self {
        Self {
            visibility: EntityModelLayerVisibility::RetainedParts(parts),
            ..Self::base(render_type, texture, tint)
        }
    }

    pub(in crate::entity_models) fn with_order(mut self, order: i32, submit_sequence: u32) -> Self {
        self.order = order;
        self.submit_sequence = submit_sequence;
        self
    }

    pub(in crate::entity_models) fn with_kind(mut self, kind: EntityModelLayerKind) -> Self {
        self.kind = kind;
        self
    }
}

pub(in crate::entity_models) fn boat_textured_layer_passes(
    family: BoatModelFamily,
    chest: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::BoatBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: boat_model_layer(family, chest),
        texture: boat_texture_ref(family, chest),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn chicken_textured_layer_passes(
    variant: ChickenModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::ChickenBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: chicken_model_layer(variant, baby),
        texture: chicken_texture_ref(variant, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn pig_textured_layer_passes(
    variant: PigModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PigBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: pig_model_layer(variant, baby),
        texture: pig_texture_ref(variant, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn cow_textured_layer_passes(
    variant: CowModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::CowBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: cow_model_layer(variant, baby),
        texture: cow_texture_ref(variant, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn salmon_textured_layer_passes(
    size: SalmonModelSize,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::SalmonBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: salmon_model_layer(size),
        texture: SALMON_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn tropical_fish_textured_layer_passes(
    shape: TropicalFishModelShape,
    base_color: EntityDyeColor,
    pattern: TropicalFishPattern,
    pattern_color: EntityDyeColor,
) -> Vec<EntityModelLayerPass> {
    let texture = match shape {
        TropicalFishModelShape::Small => TROPICAL_FISH_SMALL_TEXTURE_REF,
        TropicalFishModelShape::Large => TROPICAL_FISH_LARGE_TEXTURE_REF,
    };
    // Vanilla `getModelTint` tints the grayscale base body by the base color's texture diffuse
    // color; the `TropicalFishPatternLayer` then overlays the selected pattern texture (the body
    // mesh inflated by `FISH_PATTERN_DEFORMATION`) tinted by the pattern color.
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::TropicalFishBase,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: tropical_fish_model_layer(shape),
            texture,
            visibility: EntityModelLayerVisibility::All,
            tint: base_color.texture_diffuse_color(),
            order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::TropicalFishPattern,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: tropical_fish_pattern_model_layer(shape),
            texture: tropical_fish_pattern_texture_ref(pattern),
            visibility: EntityModelLayerVisibility::All,
            tint: pattern_color.texture_diffuse_color(),
            order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(in crate::entity_models) fn camel_textured_layer_passes(
    family: CamelModelFamily,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::CamelBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: camel_model_layer(family, baby),
        texture: camel_texture_ref(family, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn llama_textured_layer_passes(
    variant: LlamaVariant,
    baby: bool,
    // The chest no longer changes the layer pass (geometry comes from the unified model tree, whose
    // chest visibility rides the `has_chest` tree choice in `LlamaModel::new`); kept for API symmetry.
    _has_chest: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::LlamaBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: llama_model_layer(baby),
        texture: llama_texture_ref(variant, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn creeper_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::CreeperBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_CREEPER,
        texture: CREEPER_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn spider_textured_layer_passes(
    cave: bool,
) -> Vec<EntityModelLayerPass> {
    let model_layer = if cave {
        MODEL_LAYER_CAVE_SPIDER
    } else {
        MODEL_LAYER_SPIDER
    };
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SpiderBase,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer,
            texture: if cave {
                CAVE_SPIDER_TEXTURE_REF
            } else {
                SPIDER_TEXTURE_REF
            },
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SpiderEyes,
            render_type: EntityModelLayerRenderType::Eyes,
            model_layer,
            texture: SPIDER_EYES_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(in crate::entity_models) fn enderman_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::EndermanBase,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: MODEL_LAYER_ENDERMAN,
            texture: ENDERMAN_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::EndermanEyes,
            render_type: EntityModelLayerRenderType::Eyes,
            model_layer: MODEL_LAYER_ENDERMAN,
            texture: ENDERMAN_EYES_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(in crate::entity_models) fn copper_golem_textured_layer_passes(
    weathering: CopperGolemWeathering,
) -> Vec<EntityModelLayerPass> {
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::CopperGolemBase,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: MODEL_LAYER_COPPER_GOLEM,
            texture: copper_golem_texture_ref(weathering),
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::CopperGolemEyes,
            render_type: EntityModelLayerRenderType::Eyes,
            model_layer: MODEL_LAYER_COPPER_GOLEM,
            texture: copper_golem_eyes_texture_ref(weathering),
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(in crate::entity_models) fn iron_golem_textured_layer_passes(
    crackiness: IronGolemCrackiness,
) -> Vec<EntityModelLayerPass> {
    let mut passes = vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::IronGolemBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_IRON_GOLEM,
        texture: IRON_GOLEM_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }];
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
            kind: EntityModelLayerKind::IronGolemCrackiness,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: MODEL_LAYER_IRON_GOLEM,
            texture,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
        });
    }
    passes
}

pub(in crate::entity_models) fn snow_golem_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::SnowGolemBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_SNOW_GOLEM,
        texture: SNOW_GOLEM_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn witch_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::WitchBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_WITCH,
        texture: WITCH_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn slime_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SlimeBase,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: MODEL_LAYER_SLIME,
            texture: SLIME_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SlimeOuter,
            render_type: EntityModelLayerRenderType::EntityTranslucent,
            model_layer: MODEL_LAYER_SLIME_OUTER,
            texture: SLIME_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(in crate::entity_models) fn magma_cube_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::MagmaCubeBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_MAGMA_CUBE,
        texture: MAGMA_CUBE_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn ghast_textured_layer_passes(
    charging: bool,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `GhastRenderer.getTextureLocation`: `isCharging` swaps to the open-mouth shooting face.
    let texture = if charging {
        GHAST_SHOOTING_TEXTURE_REF
    } else {
        GHAST_TEXTURE_REF
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::GhastBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_GHAST,
        texture,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn happy_ghast_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::HappyGhastBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_HAPPY_GHAST,
        texture: HAPPY_GHAST_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn minecart_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::MinecartBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_MINECART,
        texture: MINECART_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn zombie_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // The unified `ZombieModel` tree drives the geometry, so the layer-pass parts are vestigial (`&[]`).
    let (model_layer, texture) = if baby {
        (MODEL_LAYER_ZOMBIE_BABY, ZOMBIE_BABY_TEXTURE_REF)
    } else {
        (MODEL_LAYER_ZOMBIE, ZOMBIE_TEXTURE_REF)
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::ZombieBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer,
        texture,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn husk_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `HuskRenderer extends ZombieRenderer`: it reuses `ZombieModel`/`BabyZombieModel`
    // geometry (`ModelLayers.HUSK` is `humanoidBodyLayer.apply(huskScale)` and `HUSK_BABY` is the
    // shared `babyZombieLayer`), so the husk reuses the unified `ZombieVariantModel` tree; only the
    // texture (`husk.png`/`husk_baby.png`) and the adult's 1.0625 mesh scale differ. The layer-pass
    // geometry is vestigial (`&[]`).
    let (model_layer, texture) = if baby {
        (MODEL_LAYER_HUSK_BABY, HUSK_BABY_TEXTURE_REF)
    } else {
        (MODEL_LAYER_HUSK, HUSK_TEXTURE_REF)
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::HuskBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer,
        texture,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn drowned_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `DrownedModel.createBodyLayer extends ZombieModel`; the non-swimming drowned reuses the
    // unified `ZombieVariantModel` (plain-zombie) tree for the base body. The base body and the outer
    // layer both come from unified model trees, so the layer-pass parts are vestigial (`&[]`).
    let (model_layer, texture) = if baby {
        (MODEL_LAYER_DROWNED_BABY, DROWNED_BABY_TEXTURE_REF)
    } else {
        (MODEL_LAYER_DROWNED, DROWNED_TEXTURE_REF)
    };
    // Vanilla `DrownedOuterLayer`: an always-on white cutout copy of the inflated drowned shell. The
    // adult re-renders `DrownedModel.createBodyLayer(0.25)` over `drowned_outer_layer.png`; the baby
    // re-renders the distinct `BabyDrownedModel.createBodyLayer(0.25)` (baby-zombie inflated mesh) over
    // `drowned_outer_layer_baby.png`.
    let (outer_model_layer, outer_texture) = if baby {
        (
            MODEL_LAYER_DROWNED_BABY_OUTER_LAYER,
            DROWNED_OUTER_LAYER_BABY_TEXTURE_REF,
        )
    } else {
        (
            MODEL_LAYER_DROWNED_OUTER_LAYER,
            DROWNED_OUTER_LAYER_TEXTURE_REF,
        )
    };
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::DrownedBase,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer,
            texture,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::DrownedOuter,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: outer_model_layer,
            texture: outer_texture,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
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
    let (model_layer, texture) = if baby {
        (
            MODEL_LAYER_ZOMBIE_VILLAGER_BABY,
            ZOMBIE_VILLAGER_BABY_TEXTURE_REF,
        )
    } else {
        (MODEL_LAYER_ZOMBIE_VILLAGER, ZOMBIE_VILLAGER_TEXTURE_REF)
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::ZombieVillagerBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer,
        texture,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn piglin_textured_layer_passes(
    family: PiglinModelFamily,
    baby_layout: bool,
) -> Vec<EntityModelLayerPass> {
    // All piglin families share `AbstractPiglinModel` geometry (`AdultZombifiedPiglinModel` /
    // `BabyZombifiedPiglinModel` forward to the piglin layers, and the brute reuses the adult
    // layer); only the model layer key and texture differ. `baby_layout` selects the baby parts
    // (the brute is never baby) in the unified `PiglinModel` tree the emitter walks; the held-vs-swung
    // arms are decided by the emitter, so the layer-pass geometry is vestigial (`&[]`).
    let (model_layer, texture) = match (family, baby_layout) {
        (PiglinModelFamily::Piglin, false) => (MODEL_LAYER_PIGLIN, PIGLIN_TEXTURE_REF),
        (PiglinModelFamily::Piglin, true) => (MODEL_LAYER_PIGLIN_BABY, PIGLIN_BABY_TEXTURE_REF),
        (PiglinModelFamily::PiglinBrute, _) => (MODEL_LAYER_PIGLIN_BRUTE, PIGLIN_BRUTE_TEXTURE_REF),
        (PiglinModelFamily::ZombifiedPiglin, false) => {
            (MODEL_LAYER_ZOMBIFIED_PIGLIN, ZOMBIFIED_PIGLIN_TEXTURE_REF)
        }
        (PiglinModelFamily::ZombifiedPiglin, true) => (
            MODEL_LAYER_ZOMBIFIED_PIGLIN_BABY,
            ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
        ),
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PiglinBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer,
        texture,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn illager_textured_layer_passes(
    family: IllagerModelFamily,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `IllagerModel.createBodyLayer` is shared across all four illagers; only the model
    // layer key and texture differ, plus per-renderer visibility: the pillager shows its separate
    // (swinging) arms, the evoker/vindicator show the static folded `arms` part, and the
    // illusioner additionally re-enables the head hat (`this.model.getHat().visible = true`).
    let (model_layer, texture) = match family {
        IllagerModelFamily::Evoker => (MODEL_LAYER_EVOKER, EVOKER_TEXTURE_REF),
        IllagerModelFamily::Vindicator => (MODEL_LAYER_VINDICATOR, VINDICATOR_TEXTURE_REF),
        IllagerModelFamily::Illusioner => (MODEL_LAYER_ILLUSIONER, ILLUSIONER_TEXTURE_REF),
        IllagerModelFamily::Pillager => (MODEL_LAYER_PILLAGER, PILLAGER_TEXTURE_REF),
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::IllagerBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer,
        texture,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn blaze_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::BlazeBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_BLAZE,
        texture: BLAZE_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn endermite_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::EndermiteBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_ENDERMITE,
        texture: ENDERMITE_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn silverfish_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::SilverfishBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_SILVERFISH,
        texture: SILVERFISH_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn leash_knot_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        LEASH_KNOT_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::LeashKnotBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn trident_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        TRIDENT_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::TridentBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn evoker_fangs_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        EVOKER_FANGS_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::EvokerFangsBase)]
}

pub(in crate::entity_models) fn tadpole_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        TADPOLE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::TadpoleBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn creaking_textured_layer_passes(
    eyes_glowing: bool,
) -> Vec<EntityModelLayerPass> {
    let mut passes = vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        CREAKING_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::CreakingBase)
    .with_order(0, 0)];
    // Vanilla `CreakingRenderer`'s `LivingEntityEmissiveLayer`: an active creaking re-renders the whole
    // model with the emissive `creaking_eyes.png` in the eyes render type (alpha `1.0` when glowing).
    if eyes_glowing {
        passes.push(
            EntityModelLayerPass::base(
                EntityModelLayerRenderType::Eyes,
                CREAKING_EYES_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
            )
            .with_kind(EntityModelLayerKind::CreakingEyes)
            .with_order(1, 1),
        );
    }
    passes
}

pub(in crate::entity_models) fn sniffer_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        SNIFFER_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::SnifferBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn parrot_textured_layer_passes(
    variant: ParrotModelVariant,
) -> Vec<EntityModelLayerPass> {
    // The five parrot colours share one model and differ only by texture
    // (`ParrotRenderer.getVariantTexture`).
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        parrot_texture_ref(variant),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::ParrotBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn shulker_textured_layer_passes(
    color: Option<EntityDyeColor>,
) -> Vec<EntityModelLayerPass> {
    // `ShulkerRenderer.getTextureLocation`: the default `shulker.png` when uncolored, else the dyed
    // `shulker_<color>.png`.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutoutZOffset,
        shulker_texture_ref(color),
        [1.0, 1.0, 1.0, 1.0],
    )]
}

pub(in crate::entity_models) fn ender_dragon_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![
        EntityModelLayerPass::base(
            EntityModelLayerRenderType::EntityCutout,
            ENDER_DRAGON_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
        )
        .with_order(0, 0),
        // Vanilla `EnderDragonRenderer` always re-submits the whole model with the emissive
        // `dragon_eyes.png` in the eyes render type before the optional healing beam custom geometry.
        EntityModelLayerPass::base(
            EntityModelLayerRenderType::Eyes,
            ENDER_DRAGON_EYES_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
        )
        .with_kind(EntityModelLayerKind::EnderDragonEyes)
        .with_order(0, 1),
    ]
}

pub(in crate::entity_models) fn nautilus_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            NAUTILUS_BABY_TEXTURE_REF
        } else {
            NAUTILUS_TEXTURE_REF
        },
        [1.0, 1.0, 1.0, 1.0],
    )]
}

pub(in crate::entity_models) fn zombie_nautilus_textured_layer_passes(
    coral: bool,
) -> Vec<EntityModelLayerPass> {
    // Vanilla `ZombieNautilusRenderer`: the `NORMAL` variant textures the shared adult `NautilusModel`
    // body with `zombie_nautilus.png`; the `WARM` variant textures the `ZombieNautilusCoralModel` with
    // `zombie_nautilus_coral.png`.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        if coral {
            ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF
        } else {
            ZOMBIE_NAUTILUS_TEXTURE_REF
        },
        [1.0, 1.0, 1.0, 1.0],
    )]
}

pub(in crate::entity_models) fn panda_textured_layer_passes(
    variant: PandaModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // The seven genes share one `PandaModel` / `BabyPandaModel` and differ only by texture × age.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        panda_texture_ref(variant, baby),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::PandaBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn axolotl_textured_layer_passes(
    variant: AxolotlModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // `AxolotlRenderer.getTextureLocation` picks the colour × age cell from `TEXTURE_BY_TYPE`.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        axolotl_texture_ref(variant, baby),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::AxolotlBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn fox_textured_layer_passes(
    variant: FoxModelVariant,
    baby: bool,
    sleeping: bool,
) -> Vec<EntityModelLayerPass> {
    // `FoxRenderer.getTextureLocation` picks the {red, snow} × {adult, baby} × {idle, sleeping} cell.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        fox_texture_ref(variant, baby, sleeping),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::FoxBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn rabbit_textured_layer_passes(
    variant: RabbitModelVariant,
    baby: bool,
    toast: bool,
) -> Vec<EntityModelLayerPass> {
    // `RabbitRenderer.getTextureLocation` picks the colour × age cell, overridden by `toast`/
    // `toast_baby` for the `Toast` named rabbit.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        rabbit_texture_ref(variant, baby, toast),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::RabbitBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn feline_textured_layer_passes(
    cat: bool,
    baby: bool,
    cat_variant: CatModelVariant,
    collar: Option<EntityDyeColor>,
) -> Vec<EntityModelLayerPass> {
    // The cat and ocelot share `AbstractFelineModel`, so the base pass differs only in which image it
    // binds: the per-breed `CatVariant` texture for cats, the `ocelot` texture otherwise.
    let mut passes = vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        feline_texture_ref(cat, baby, cat_variant),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::FelineBase)
    .with_order(0, 0)];
    // Vanilla `CatCollarLayer`: a tame cat re-renders the mesh with `cat_collar.png` tinted by the
    // dye's diffuse color (`collar` is already gated on cat && tame; the ocelot never carries one).
    if let Some(collar) = collar {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::FelineCollar,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: "",
            texture: feline_collar_texture_ref(baby),
            visibility: EntityModelLayerVisibility::All,
            tint: collar.texture_diffuse_color(),
            order: 1,
            submit_sequence: 1,
        });
    }
    passes
}

pub(in crate::entity_models) fn mooshroom_textured_layer_passes(
    baby: bool,
    variant: MooshroomVariant,
) -> Vec<EntityModelLayerPass> {
    // The mooshroom reuses the cow model tree (geometry drives off it), so this binds only the
    // mooshroom recolor (the red/brown variant face) over the shared cow UVs.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        mooshroom_texture_ref(baby, variant),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::MooshroomBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn arrow_textured_layer_passes(
    texture: ArrowModelTexture,
) -> Vec<EntityModelLayerPass> {
    // One model shared by the normal / tipped / spectral arrow; only the bound image differs.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutoutCull,
        arrow_texture_ref(texture),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::ArrowBase)]
}

pub(in crate::entity_models) fn llama_spit_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        LLAMA_SPIT_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::LlamaSpitBase)]
}

#[cfg(test)]
pub(in crate::entity_models) fn shulker_bullet_textured_layer_passes() -> Vec<EntityModelLayerPass>
{
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        SHULKER_BULLET_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
    )]
}

pub(in crate::entity_models) fn wither_skull_textured_layer_passes(
    dangerous: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityTranslucent,
        wither_skull_texture_ref(dangerous),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::WitherSkullBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn wither_textured_layer_passes(
    invulnerable_ticks: f32,
) -> Vec<EntityModelLayerPass> {
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
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        texture,
        [1.0, 1.0, 1.0, 1.0],
    )]
}

pub(in crate::entity_models) fn guardian_textured_layer_passes(
    elder: bool,
) -> Vec<EntityModelLayerPass> {
    // The guardian and elder guardian share one mesh, differing only by texture; the attack beam stays
    // deferred.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        if elder {
            GUARDIAN_ELDER_TEXTURE_REF
        } else {
            GUARDIAN_TEXTURE_REF
        },
        [1.0, 1.0, 1.0, 1.0],
    )]
}

pub(in crate::entity_models) fn armadillo_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    // The adult and baby armadillo share the UV layout; the baby binds its own texture.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            ARMADILLO_BABY_TEXTURE_REF
        } else {
            ARMADILLO_TEXTURE_REF
        },
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::ArmadilloBase)
    .with_order(0, 0)]
}

pub(in crate::entity_models) fn frog_textured_layer_passes(
    variant: FrogModelVariant,
) -> Vec<EntityModelLayerPass> {
    // The frog binds its temperature-variant base texture; all three share one `FrogModel` geometry.
    vec![EntityModelLayerPass::base(
        EntityModelLayerRenderType::EntityCutout,
        frog_texture_ref(variant),
        [1.0, 1.0, 1.0, 1.0],
    )
    .with_kind(EntityModelLayerKind::FrogBase)
    .with_order(0, 0)]
}

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
const WARDEN_BIOLUMINESCENT_PARTS: &[&str] =
    &["head", "left_arm", "right_arm", "left_leg", "right_leg"];
const WARDEN_PULSATING_SPOTS_PARTS: &[&str] = &[
    "body",
    "head",
    "left_arm",
    "right_arm",
    "left_leg",
    "right_leg",
];
const WARDEN_TENDRILS_PARTS: &[&str] = &["left_tendril", "right_tendril"];
const WARDEN_HEART_PARTS: &[&str] = &["body"];

pub(in crate::entity_models) fn warden_textured_layer_passes(
    age_in_ticks: f32,
    tendril_animation: f32,
    heart_animation: f32,
) -> Vec<EntityModelLayerPass> {
    // The warden's base body, then the five `WardenEmissiveLayer`s — each an eyes-render-type pass (the
    // pipeline being emissive + alpha-blended, matching vanilla `entityTranslucentEmissive`) over its
    // own part subset: the always-on bioluminescent overlay (alpha 1.0, head/arms/legs); the two
    // pulsating-spots overlays (body/legs, each fading on its own `cos(ageInTicks · 0.045)` pulse); the
    // tendril overlay, which reuses the base `warden.png` over the two tendril planes at the lerped
    // `tendrilAnimation` alpha; and the heart overlay (body only) at the lerped `heartAnimation` alpha.
    vec![
        EntityModelLayerPass::base(
            EntityModelLayerRenderType::EntityCutout,
            WARDEN_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
        ),
        EntityModelLayerPass::retained(
            EntityModelLayerRenderType::Eyes,
            WARDEN_BIOLUMINESCENT_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            WARDEN_BIOLUMINESCENT_PARTS,
        )
        .with_order(1, 1),
        EntityModelLayerPass::retained(
            EntityModelLayerRenderType::Eyes,
            WARDEN_PULSATING_SPOTS_1_TEXTURE_REF,
            [
                1.0,
                1.0,
                1.0,
                warden_pulsating_spots_alpha(age_in_ticks, 0.0),
            ],
            WARDEN_PULSATING_SPOTS_PARTS,
        )
        .with_order(1, 2),
        EntityModelLayerPass::retained(
            EntityModelLayerRenderType::Eyes,
            WARDEN_PULSATING_SPOTS_2_TEXTURE_REF,
            [
                1.0,
                1.0,
                1.0,
                warden_pulsating_spots_alpha(age_in_ticks, std::f32::consts::PI),
            ],
            WARDEN_PULSATING_SPOTS_PARTS,
        )
        .with_order(1, 3),
        EntityModelLayerPass::retained(
            EntityModelLayerRenderType::Eyes,
            WARDEN_TEXTURE_REF,
            [1.0, 1.0, 1.0, tendril_animation],
            WARDEN_TENDRILS_PARTS,
        )
        .with_order(1, 4),
        EntityModelLayerPass::retained(
            EntityModelLayerRenderType::Eyes,
            WARDEN_HEART_TEXTURE_REF,
            [1.0, 1.0, 1.0, heart_animation],
            WARDEN_HEART_PARTS,
        )
        .with_order(1, 5),
    ]
}

pub(in crate::entity_models) fn phantom_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::PhantomBase,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: MODEL_LAYER_PHANTOM,
            texture: PHANTOM_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 0,
            submit_sequence: 0,
        },
        // Vanilla `PhantomEyesLayer` (an `EyesLayer`): the whole model is re-rendered with
        // the emissive `phantom_eyes.png` in the eyes render type.
        EntityModelLayerPass {
            kind: EntityModelLayerKind::PhantomEyes,
            render_type: EntityModelLayerRenderType::Eyes,
            model_layer: MODEL_LAYER_PHANTOM,
            texture: PHANTOM_EYES_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(in crate::entity_models) fn polar_bear_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PolarBearBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: if baby {
            MODEL_LAYER_POLAR_BEAR_BABY
        } else {
            MODEL_LAYER_POLAR_BEAR
        },
        texture: if baby {
            POLAR_BEAR_BABY_TEXTURE_REF
        } else {
            POLAR_BEAR_TEXTURE_REF
        },
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn ravager_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::RavagerBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_RAVAGER,
        texture: RAVAGER_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn villager_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::VillagerBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: if baby {
            MODEL_LAYER_VILLAGER_BABY
        } else {
            MODEL_LAYER_VILLAGER
        },
        texture: if baby {
            VILLAGER_BABY_TEXTURE_REF
        } else {
            VILLAGER_TEXTURE_REF
        },
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn wandering_trader_textured_layer_passes() -> Vec<EntityModelLayerPass>
{
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::WanderingTraderBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: MODEL_LAYER_WANDERING_TRADER,
        texture: WANDERING_TRADER_TEXTURE_REF,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn hoglin_textured_layer_passes(
    family: HoglinModelFamily,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    let (model_layer, texture) = match (family, baby) {
        (HoglinModelFamily::Hoglin, false) => (MODEL_LAYER_HOGLIN, HOGLIN_TEXTURE_REF),
        (HoglinModelFamily::Hoglin, true) => (MODEL_LAYER_HOGLIN_BABY, HOGLIN_BABY_TEXTURE_REF),
        (HoglinModelFamily::Zoglin, false) => (MODEL_LAYER_ZOGLIN, ZOGLIN_TEXTURE_REF),
        (HoglinModelFamily::Zoglin, true) => (MODEL_LAYER_ZOGLIN_BABY, ZOGLIN_BABY_TEXTURE_REF),
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::HoglinBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer,
        texture,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

#[cfg(test)]
pub(in crate::entity_models) fn player_textured_layer_passes(
    slim: bool,
    parts: PlayerModelPartVisibility,
) -> Vec<EntityModelLayerPass> {
    player_textured_layer_passes_with_texture(slim, parts, player_texture_ref(slim))
}

pub(in crate::entity_models) fn player_textured_layer_passes_with_texture(
    slim: bool,
    parts: PlayerModelPartVisibility,
    texture: EntityModelTextureRef,
) -> Vec<EntityModelLayerPass> {
    // The unified `PlayerModel` tree drives the geometry (its overlay children are toggled by
    // `apply_part_visibility`), so the layer-pass parts are vestigial (`&[]`).
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PlayerBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: player_model_layer(slim),
        texture,
        visibility: EntityModelLayerVisibility::PlayerParts(parts),
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
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
    passes.push(EntityModelLayerPass {
        kind: EntityModelLayerKind::SheepBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: if baby {
            MODEL_LAYER_SHEEP_BABY
        } else {
            MODEL_LAYER_SHEEP
        },
        texture: if baby {
            SHEEP_BABY_TEXTURE_REF
        } else {
            SHEEP_TEXTURE_REF
        },
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    });
    if !baby && (jeb || wool_color != SheepWoolColor::White) {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWoolUndercoat,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: MODEL_LAYER_SHEEP_WOOL_UNDERCOAT,
            texture: SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
            visibility: EntityModelLayerVisibility::All,
            tint: wool_tint,
            order: 1,
            submit_sequence: 1,
        });
    }
    if !sheared {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWool,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer: if baby {
                MODEL_LAYER_SHEEP_BABY_WOOL
            } else {
                MODEL_LAYER_SHEEP_WOOL
            },
            texture: if baby {
                SHEEP_WOOL_BABY_TEXTURE_REF
            } else {
                SHEEP_WOOL_TEXTURE_REF
            },
            visibility: EntityModelLayerVisibility::All,
            tint: wool_tint,
            order: if baby { 1 } else { 0 },
            submit_sequence: 2,
        });
    }
    passes.sort_by_key(|pass| (pass.order, pass.submit_sequence));
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
    let model_layer = if baby {
        MODEL_LAYER_WOLF_BABY
    } else {
        MODEL_LAYER_WOLF
    };
    let mut passes = Vec::with_capacity(2);
    passes.push(EntityModelLayerPass {
        kind: EntityModelLayerKind::WolfBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer,
        texture: wolf_texture_ref(baby, tame, angry, variant),
        visibility: EntityModelLayerVisibility::All,
        tint: [wet_shade, wet_shade, wet_shade, 1.0],
        order: 0,
        submit_sequence: 0,
    });
    if let Some(collar_color) = tame.then_some(collar_color).flatten() {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::WolfCollar,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer,
            texture: if baby {
                WOLF_BABY_COLLAR_TEXTURE_REF
            } else {
                WOLF_COLLAR_TEXTURE_REF
            },
            visibility: EntityModelLayerVisibility::All,
            tint: collar_color.texture_diffuse_color(),
            order: 1,
            submit_sequence: 1,
        });
    }
    passes
}

pub(in crate::entity_models) fn goat_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::GoatBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: goat_model_layer(baby),
        texture: goat_texture_ref(baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn skeleton_textured_layer_passes(
    family: Option<SkeletonModelFamily>,
) -> Vec<EntityModelLayerPass> {
    // Both the base body and the clothing overlay come from unified model trees (the base
    // `SkeletonModel` and the textured-only `SkeletonClothingModel`), so the layer-pass parts are
    // vestigial (`&[]`).
    let mut passes = vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::SkeletonBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: skeleton_model_layer(family),
        texture: skeleton_texture_ref(family),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }];
    if let Some((model_layer, texture)) = skeleton_clothing_layer_pass(family) {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SkeletonClothing,
            render_type: EntityModelLayerRenderType::EntityCutout,
            model_layer,
            texture,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            order: 1,
            submit_sequence: 1,
        });
    }
    passes
}

fn boat_model_layer(family: BoatModelFamily, chest: bool) -> &'static str {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => MODEL_LAYER_ACACIA_BOAT,
        (BoatModelFamily::Acacia, true) => MODEL_LAYER_ACACIA_CHEST_BOAT,
        (BoatModelFamily::Bamboo, false) => MODEL_LAYER_BAMBOO_RAFT,
        (BoatModelFamily::Bamboo, true) => MODEL_LAYER_BAMBOO_CHEST_RAFT,
        (BoatModelFamily::Birch, false) => MODEL_LAYER_BIRCH_BOAT,
        (BoatModelFamily::Birch, true) => MODEL_LAYER_BIRCH_CHEST_BOAT,
        (BoatModelFamily::Cherry, false) => MODEL_LAYER_CHERRY_BOAT,
        (BoatModelFamily::Cherry, true) => MODEL_LAYER_CHERRY_CHEST_BOAT,
        (BoatModelFamily::DarkOak, false) => MODEL_LAYER_DARK_OAK_BOAT,
        (BoatModelFamily::DarkOak, true) => MODEL_LAYER_DARK_OAK_CHEST_BOAT,
        (BoatModelFamily::Jungle, false) => MODEL_LAYER_JUNGLE_BOAT,
        (BoatModelFamily::Jungle, true) => MODEL_LAYER_JUNGLE_CHEST_BOAT,
        (BoatModelFamily::Mangrove, false) => MODEL_LAYER_MANGROVE_BOAT,
        (BoatModelFamily::Mangrove, true) => MODEL_LAYER_MANGROVE_CHEST_BOAT,
        (BoatModelFamily::Oak, false) => MODEL_LAYER_OAK_BOAT,
        (BoatModelFamily::Oak, true) => MODEL_LAYER_OAK_CHEST_BOAT,
        (BoatModelFamily::PaleOak, false) => MODEL_LAYER_PALE_OAK_BOAT,
        (BoatModelFamily::PaleOak, true) => MODEL_LAYER_PALE_OAK_CHEST_BOAT,
        (BoatModelFamily::Spruce, false) => MODEL_LAYER_SPRUCE_BOAT,
        (BoatModelFamily::Spruce, true) => MODEL_LAYER_SPRUCE_CHEST_BOAT,
    }
}

fn player_model_layer(slim: bool) -> &'static str {
    if slim {
        MODEL_LAYER_PLAYER_SLIM
    } else {
        MODEL_LAYER_PLAYER
    }
}

fn chicken_model_layer(variant: ChickenModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (_, true) => MODEL_LAYER_CHICKEN_BABY,
        (ChickenModelVariant::Cold, false) => MODEL_LAYER_COLD_CHICKEN,
        (_, false) => MODEL_LAYER_CHICKEN,
    }
}

fn pig_model_layer(variant: PigModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (_, true) => MODEL_LAYER_PIG_BABY,
        (PigModelVariant::Cold, false) => MODEL_LAYER_COLD_PIG,
        (_, false) => MODEL_LAYER_PIG,
    }
}

fn cow_model_layer(variant: CowModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (CowModelVariant::Temperate, false) => MODEL_LAYER_COW,
        (CowModelVariant::Temperate, true) => MODEL_LAYER_COW_BABY,
        (CowModelVariant::Warm, false) => MODEL_LAYER_WARM_COW,
        (CowModelVariant::Warm, true) => MODEL_LAYER_WARM_COW_BABY,
        (CowModelVariant::Cold, false) => MODEL_LAYER_COLD_COW,
        (CowModelVariant::Cold, true) => MODEL_LAYER_COLD_COW_BABY,
    }
}

fn camel_model_layer(family: CamelModelFamily, baby: bool) -> &'static str {
    // The camel husk reuses the adult camel mesh, so only a real baby camel uses the
    // baby body layer.
    if family == CamelModelFamily::Camel && baby {
        MODEL_LAYER_CAMEL_BABY
    } else {
        MODEL_LAYER_CAMEL
    }
}

fn llama_model_layer(baby: bool) -> &'static str {
    if baby {
        MODEL_LAYER_LLAMA_BABY
    } else {
        MODEL_LAYER_LLAMA
    }
}

fn goat_model_layer(baby: bool) -> &'static str {
    if baby {
        MODEL_LAYER_GOAT_BABY
    } else {
        MODEL_LAYER_GOAT
    }
}

fn goat_texture_ref(baby: bool) -> EntityModelTextureRef {
    if baby {
        GOAT_BABY_TEXTURE_REF
    } else {
        GOAT_TEXTURE_REF
    }
}

fn skeleton_model_layer(family: Option<SkeletonModelFamily>) -> &'static str {
    match family {
        None => MODEL_LAYER_SKELETON,
        Some(SkeletonModelFamily::Stray) => MODEL_LAYER_STRAY,
        Some(SkeletonModelFamily::Parched) => MODEL_LAYER_PARCHED,
        Some(SkeletonModelFamily::WitherSkeleton) => MODEL_LAYER_WITHER_SKELETON,
        Some(SkeletonModelFamily::Bogged { .. }) => MODEL_LAYER_BOGGED,
    }
}

fn skeleton_texture_ref(family: Option<SkeletonModelFamily>) -> EntityModelTextureRef {
    match family {
        None => SKELETON_TEXTURE_REF,
        Some(SkeletonModelFamily::Stray) => STRAY_TEXTURE_REF,
        Some(SkeletonModelFamily::Parched) => PARCHED_TEXTURE_REF,
        Some(SkeletonModelFamily::WitherSkeleton) => WITHER_SKELETON_TEXTURE_REF,
        Some(SkeletonModelFamily::Bogged { .. }) => BOGGED_TEXTURE_REF,
    }
}

fn skeleton_clothing_layer_pass(
    family: Option<SkeletonModelFamily>,
) -> Option<(&'static str, EntityModelTextureRef)> {
    match family {
        Some(SkeletonModelFamily::Stray) => {
            Some((MODEL_LAYER_STRAY_OUTER_LAYER, STRAY_OVERLAY_TEXTURE_REF))
        }
        Some(SkeletonModelFamily::Bogged { .. }) => {
            Some((MODEL_LAYER_BOGGED_OUTER_LAYER, BOGGED_OVERLAY_TEXTURE_REF))
        }
        None | Some(SkeletonModelFamily::Parched) | Some(SkeletonModelFamily::WitherSkeleton) => {
            None
        }
    }
}
