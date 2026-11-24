use crate::entity_models::model_layer::ModelLayer;

#[cfg(test)]
use super::super::catalog::player_texture_ref;
use super::super::temperature_animal_variants::{
    ChickenVariantModelType, CowVariantModelType, PigVariantModelType,
};
use super::super::{
    banner_pattern_texture_ref,
    catalog::{
        boat_texture_ref, camel_texture_ref, chicken_texture_ref, cow_texture_ref,
        horse_coat_texture_ref, horse_markings_texture_ref, llama_texture_ref,
        mooshroom_texture_ref, pig_texture_ref, sheep_wool_render_color, squid_texture_ref,
        villager_level_texture_ref, villager_profession_hat,
        villager_profession_renders_level_badge, villager_profession_texture_ref,
        villager_type_hat, villager_type_texture_ref, wolf_texture_ref,
        zombie_villager_level_texture_ref, zombie_villager_profession_texture_ref,
        zombie_villager_type_texture_ref, ArrowModelTexture, AxolotlModelVariant,
        BannerPatternLayer, BannerPatternTexture, BedModelPart, BoatModelFamily, CamelModelFamily,
        ChestModelHalf, ChestModelTexture, ConduitModelPart, CopperGolemWeathering,
        DonkeyModelFamily, EntityCustomHeadSkull, EntityDyeColor, EntityModelTextureRef,
        EntityPlayerSkin, FoxModelVariant, HoglinModelFamily, HorseColorVariant, HorseMarkings,
        IllagerModelFamily, IronGolemCrackiness, LlamaVariant, MooshroomVariant, PandaModelVariant,
        ParrotModelVariant, PiglinModelFamily, PlayerModelPartVisibility, RabbitModelVariant,
        SheepWoolColor, SignModelAttachment, SignModelWood, SkeletonModelFamily,
        TropicalFishModelShape, TropicalFishPattern, UndeadHorseModelFamily, VillagerModelData,
        VillagerModelHat,
    },
    model_layers::*,
    CatModelVariant, ChickenModelVariant, CowModelVariant, DecoratedPotPattern, FrogModelVariant,
    PigModelVariant, WolfModelVariant, ZombieNautilusModelVariant,
};

mod model_layers;
mod passes;
pub(in crate::entity_models) use model_layers::*;
pub(in crate::entity_models) use passes::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerKind {
    InheritOverlay,
    NoOverlay,
    ZeroWhiteOverlay,
    ArmorStandBase,
    SheepWool,
    SlimeOuter,
    SkeletonClothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityModelLayerRenderType {
    /// Vanilla `RenderTypes.entitySolid(texture)`.
    EntitySolid,
    /// Vanilla `RenderTypes.entitySolidZOffsetForward(texture)`.
    EntitySolidZOffsetForward,
    /// Vanilla `RenderTypes.armorCutoutNoCull(texture)` used by equipment layers.
    ArmorCutoutNoCull,
    /// Vanilla armor-trim sheet through `RenderTypes.armorCutoutNoCull`.
    ArmorTrimCutoutNoCull,
    /// Vanilla `RenderTypes.createArmorDecalCutoutNoCull` for decal trim patterns.
    ArmorTrimDecalCutoutNoCull,
    /// Vanilla `RenderTypes.armorTranslucent(texture)` used by translucent armor damage overlays.
    ArmorTranslucent,
    /// Vanilla `RenderTypes.entityCutout(texture)`; this is also the default
    /// `EntityModel` render type when the model does not override it.
    EntityCutout,
    /// Vanilla `RenderTypes.entityCutoutDissolve(texture, maskTexture)`.
    EntityCutoutDissolve,
    /// Vanilla `RenderTypes.entityCutoutCull(texture)`.
    EntityCutoutCull,
    /// Vanilla `RenderTypes.entityCutoutZOffset(texture)`.
    EntityCutoutZOffset,
    /// Vanilla `RenderTypes.entityTranslucent(texture)`.
    EntityTranslucent,
    /// Vanilla `RenderTypes.bannerPattern(texture)`.
    BannerPattern,
    /// Vanilla `RenderTypes.entityTranslucentEmissive(texture, affectsOutline)`.
    EntityTranslucentEmissive,
    /// Vanilla `RenderTypes.entityTranslucentCullItemTarget(texture)`, used by
    /// `LivingEntityRenderer.getRenderType` for invisible entities still visible to this client.
    EntityTranslucentCullItemTarget,
    /// Vanilla `RenderTypes.outline(texture)`, used by `LivingEntityRenderer.getRenderType` when an
    /// invisible entity still appears glowing.
    Outline,
    /// Vanilla `RenderTypes.entityGlint()` item foil overlay.
    EntityGlint,
    /// Vanilla `RenderTypes.armorEntityGlint()` armor foil overlay.
    ArmorEntityGlint,
    /// Vanilla eyes/emissive render type (`EyesLayer` / translucent emissive overlays).
    Eyes,
    /// Vanilla `RenderTypes.breezeWind(texture, u, v)`.
    BreezeWind,
    /// Vanilla `RenderTypes.energySwirl(texture, u, v)`.
    EnergySwirl,
    /// Vanilla `RenderTypes.endCrystalBeam(texture)`.
    EndCrystalBeam,
    /// Vanilla `RenderTypes.endPortal()`.
    EndPortal,
    /// Vanilla `RenderTypes.endGateway()`.
    EndGateway,
    /// Vanilla opaque `RenderTypes.beaconBeam(texture, false)` core used by
    /// `TheEndGatewayRenderer`.
    BeaconBeamOpaque,
    /// Vanilla translucent `RenderTypes.beaconBeam(texture, true)` glow used by
    /// `TheEndGatewayRenderer`.
    BeaconBeamTranslucent,
    /// Vanilla `RenderTypes.dragonRays()`.
    DragonRays,
    /// Vanilla `RenderTypes.dragonRaysDepth()`.
    DragonRaysDepth,
    /// Vanilla `RenderTypes.leash()` (`RenderPipelines.LEASH`): untextured opaque
    /// position-color-lightmap geometry, used by the leash rope strips.
    Leash,
    /// Vanilla `RenderTypes.waterMask()`, used by the boat water patch.
    WaterMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerRenderBucket {
    /// Vanilla `ENTITY_SOLID`: cull-on entity shading without alpha cutout.
    Solid,
    Cutout,
    /// Vanilla `RenderPipelines.ENTITY_SOLID_Z_OFFSET_FORWARD`: the culling, opaque entity-solid
    /// state with `VIEW_OFFSET_Z_LAYERING_FORWARD` projection. It cannot share the ordinary solid
    /// bucket because the GPU path needs the forward z-offset matrix.
    SolidZOffsetForward,
    /// Vanilla `RenderPipelines.ENTITY_CUTOUT_DISSOLVE` (`RenderTypes.entityCutoutDissolve`): opaque
    /// cutout state plus a per-fragment DISSOLVE mask sample (the dying ender dragon). Kept out of the
    /// plain [`Self::Cutout`] bucket so the GPU path can own the dedicated dissolve pipeline and its
    /// second `mask_uv` vertex set.
    Dissolve,
    CutoutZOffset,
    Translucent,
    TranslucentEmissive,
    ItemEntityTranslucent,
    Eyes,
    /// Vanilla `BREEZE_WIND`.
    Scroll,
    /// Vanilla `ENERGY_SWIRL`.
    AdditiveScroll,
    EndCrystalBeam,
    BeaconBeamOpaque,
    BeaconBeamTranslucent,
    PositionColor,
    Portal,
    OutlineOnly,
    DepthOnly,
    GlintOnly,
}

impl EntityModelLayerRenderType {
    #[cfg(test)]
    pub(in crate::entity_models) const ALL: [Self; 29] = [
        Self::EntitySolid,
        Self::EntitySolidZOffsetForward,
        Self::ArmorCutoutNoCull,
        Self::ArmorTrimCutoutNoCull,
        Self::ArmorTrimDecalCutoutNoCull,
        Self::ArmorTranslucent,
        Self::EntityCutout,
        Self::EntityCutoutDissolve,
        Self::EntityCutoutCull,
        Self::EntityCutoutZOffset,
        Self::EntityTranslucent,
        Self::BannerPattern,
        Self::EntityTranslucentEmissive,
        Self::EntityTranslucentCullItemTarget,
        Self::Outline,
        Self::EntityGlint,
        Self::ArmorEntityGlint,
        Self::Eyes,
        Self::BreezeWind,
        Self::EnergySwirl,
        Self::EndCrystalBeam,
        Self::EndPortal,
        Self::EndGateway,
        Self::BeaconBeamOpaque,
        Self::BeaconBeamTranslucent,
        Self::DragonRays,
        Self::DragonRaysDepth,
        Self::Leash,
        Self::WaterMask,
    ];

    pub(in crate::entity_models) const fn mesh_bucket(self) -> EntityModelLayerRenderBucket {
        match self {
            Self::EntitySolid => EntityModelLayerRenderBucket::Solid,
            Self::ArmorCutoutNoCull
            | Self::ArmorTrimCutoutNoCull
            | Self::ArmorTrimDecalCutoutNoCull
            | Self::EntityCutout
            | Self::EntityCutoutCull => EntityModelLayerRenderBucket::Cutout,
            Self::EntitySolidZOffsetForward => EntityModelLayerRenderBucket::SolidZOffsetForward,
            Self::EntityCutoutDissolve => EntityModelLayerRenderBucket::Dissolve,
            Self::EntityCutoutZOffset => EntityModelLayerRenderBucket::CutoutZOffset,
            Self::ArmorTranslucent | Self::EntityTranslucent | Self::BannerPattern => {
                EntityModelLayerRenderBucket::Translucent
            }
            Self::EntityTranslucentEmissive => EntityModelLayerRenderBucket::TranslucentEmissive,
            Self::EntityTranslucentCullItemTarget => {
                EntityModelLayerRenderBucket::ItemEntityTranslucent
            }
            Self::Outline => EntityModelLayerRenderBucket::OutlineOnly,
            Self::EntityGlint | Self::ArmorEntityGlint => EntityModelLayerRenderBucket::GlintOnly,
            Self::Eyes => EntityModelLayerRenderBucket::Eyes,
            Self::BreezeWind => EntityModelLayerRenderBucket::Scroll,
            Self::EnergySwirl => EntityModelLayerRenderBucket::AdditiveScroll,
            Self::EndCrystalBeam => EntityModelLayerRenderBucket::EndCrystalBeam,
            Self::BeaconBeamOpaque => EntityModelLayerRenderBucket::BeaconBeamOpaque,
            Self::BeaconBeamTranslucent => EntityModelLayerRenderBucket::BeaconBeamTranslucent,
            Self::EndPortal | Self::EndGateway => EntityModelLayerRenderBucket::Portal,
            Self::DragonRays | Self::Leash => EntityModelLayerRenderBucket::PositionColor,
            Self::WaterMask => EntityModelLayerRenderBucket::DepthOnly,
            Self::DragonRaysDepth => EntityModelLayerRenderBucket::DepthOnly,
        }
    }

    pub(in crate::entity_models) const fn affects_outline(self) -> bool {
        matches!(
            self,
            Self::EntitySolid
                | Self::EntitySolidZOffsetForward
                | Self::ArmorCutoutNoCull
                | Self::ArmorTrimCutoutNoCull
                | Self::ArmorTrimDecalCutoutNoCull
                | Self::ArmorTranslucent
                | Self::EntityCutout
                | Self::EntityCutoutDissolve
                | Self::EntityCutoutCull
                | Self::EntityCutoutZOffset
                | Self::EntityTranslucent
                | Self::EntityTranslucentCullItemTarget
        )
    }

    /// Vanilla `RenderSetup.affectsCrumbling` for the represented render type.
    pub(in crate::entity_models) const fn affects_crumbling(self) -> bool {
        matches!(
            self,
            Self::EntitySolid
                | Self::EntitySolidZOffsetForward
                | Self::ArmorCutoutNoCull
                | Self::ArmorTrimCutoutNoCull
                | Self::ArmorTrimDecalCutoutNoCull
                | Self::ArmorTranslucent
                | Self::EntityCutout
                | Self::EntityCutoutDissolve
                | Self::EntityCutoutCull
                | Self::EntityCutoutZOffset
                | Self::EntityTranslucent
                | Self::EntityTranslucentEmissive
                | Self::EntityTranslucentCullItemTarget
        )
    }

    pub(in crate::entity_models) const fn has_blending(self) -> bool {
        matches!(
            self,
            Self::ArmorTranslucent
                | Self::EntityTranslucent
                | Self::BannerPattern
                | Self::EntityTranslucentEmissive
                | Self::EntityTranslucentCullItemTarget
                | Self::EntityGlint
                | Self::ArmorEntityGlint
                | Self::Eyes
                | Self::BreezeWind
                | Self::EnergySwirl
                | Self::BeaconBeamTranslucent
                | Self::DragonRays
        )
    }

    pub(in crate::entity_models) const fn surface_cull(self) -> bool {
        matches!(
            self,
            Self::EntitySolid
                | Self::EntitySolidZOffsetForward
                | Self::EntityCutoutCull
                | Self::BannerPattern
                | Self::EntityTranslucentCullItemTarget
        )
    }

    pub(in crate::entity_models) const fn outline_cull(self) -> bool {
        self.surface_cull()
    }

    #[cfg(test)]
    pub(in crate::entity_models) const fn vanilla_name(self) -> &'static str {
        match self {
            Self::EntitySolid => "entitySolid",
            Self::EntitySolidZOffsetForward => "entitySolidZOffsetForward",
            Self::ArmorCutoutNoCull => "armorCutoutNoCull",
            Self::ArmorTrimCutoutNoCull => "armorTrimsSheet",
            Self::ArmorTrimDecalCutoutNoCull => "armorTrimsDecalSheet",
            Self::ArmorTranslucent => "armorTranslucent",
            Self::EntityCutout => "entityCutout",
            Self::EntityCutoutDissolve => "entityCutoutDissolve",
            Self::EntityCutoutCull => "entityCutoutCull",
            Self::EntityCutoutZOffset => "entityCutoutZOffset",
            Self::EntityTranslucent => "entityTranslucent",
            Self::BannerPattern => "bannerPattern",
            Self::EntityTranslucentEmissive => "entityTranslucentEmissive",
            Self::EntityTranslucentCullItemTarget => "entityTranslucentCullItemTarget",
            Self::Outline => "outline",
            Self::EntityGlint => "entityGlint",
            Self::ArmorEntityGlint => "armorEntityGlint",
            Self::Eyes => "eyes",
            Self::BreezeWind => "breezeWind",
            Self::EnergySwirl => "energySwirl",
            Self::EndCrystalBeam => "end_crystal_beam",
            Self::EndPortal => "end_portal",
            Self::EndGateway => "end_gateway",
            Self::BeaconBeamOpaque | Self::BeaconBeamTranslucent => "beacon_beam",
            Self::DragonRays => "dragonRays",
            Self::DragonRaysDepth => "dragonRaysDepth",
            Self::Leash => "leash",
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
    pub(in crate::entity_models) texture: EntityModelTextureRef,
    pub(in crate::entity_models) visibility: EntityModelLayerVisibility,
    pub(in crate::entity_models) tint: [f32; 4],
    pub(in crate::entity_models) order: i32,
    pub(in crate::entity_models) is_base: bool,
    /// True for Vanilla layers that render even when `LivingEntityRenderState.isInvisible` is set
    /// (`EyesLayer`, `BreezeEyesLayer`, `LivingEntityEmissiveLayer(..., alwaysVisible = true)`).
    pub(in crate::entity_models) visible_when_invisible: bool,
}

impl EntityModelLayerPass {
    pub(in crate::entity_models) const fn new(
        kind: EntityModelLayerKind,
        render_type: EntityModelLayerRenderType,
        texture: EntityModelTextureRef,
    ) -> Self {
        Self {
            kind,
            render_type,
            texture,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0; 4],
            order: 0,
            is_base: true,
            visible_when_invisible: false,
        }
    }
}

pub(in crate::entity_models) fn player_textured_layer_passes_with_texture(
    parts: PlayerModelPartVisibility,
    texture: EntityModelTextureRef,
) -> [EntityModelLayerPass; 1] {
    // The unified `PlayerModel` tree drives the geometry (its overlay children are toggled by
    // `apply_part_visibility`), so the layer-pass parts are vestigial (`&[]`).
    [EntityModelLayerPass {
        visibility: EntityModelLayerVisibility::PlayerParts(parts),
        ..EntityModelLayerPass::new(
            EntityModelLayerKind::InheritOverlay,
            EntityModelLayerRenderType::EntityCutout,
            texture,
        )
    }]
}
