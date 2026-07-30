use super::catalog::EntityDynamicPlayerTextureAtlasEntry;
use super::colored::{
    boat_model_root_transform, creeper_model_root_transform, drowned_model_root_transform,
    ender_dragon_model_root_transform, happy_ghast_model_root_transform,
    trident_model_root_transform, villager_adult_model_root_transform, wither_model_root_transform,
    LegacyRandom, GIANT_SCALE, HORSE_SCALE,
};
use super::dispatch::{
    dispatch_invisible_living_ungated_layers, dispatch_uniform_entity_model,
    dispatch_vanilla_entity_layers, TexturedSink,
};
use super::equipment::{
    equipment_layer_tint, EntityEquipmentLayerType, ResolvedEntityEquipmentAssetCatalog,
    ResolvedEntityEquipmentAssetLayer, ResolvedEntityEquipmentAssetLayerTexture,
};
use super::held_item::{custom_head_skull_transform, custom_head_skull_transform_with_root};
use super::model::{EntityModel, ModelPart};
#[cfg(test)]
use super::model_layers::PLAYER_WIDE_STEVE_TEXTURE_REF;
use super::{
    catalog::{
        ArrowModelTexture, CamelModelFamily, DonkeyModelFamily, EndGatewayBeamRenderState,
        EndPortalModelFace, EndPortalModelKind, EntityArmorMaterial, EntityArmorTrim,
        EntityCustomHeadSkull, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinAtlasEntry,
        EntityDynamicPlayerSkinAtlasLayout, EntityDynamicPlayerSkinStatus,
        EntityDynamicPlayerTexture, EntityDynamicPlayerTextureAtlasLayout, EntityDynamicTexture,
        EntityEquipmentLayerTexture, EntityModelKind, EntityModelOwnedTextureAtlasEntry,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureRef,
        EntityModelUvRect, EntityPlayerSkin, FirstPersonPlayerArm, HorseColorVariant,
        HorseMarkings, LeashRopeRenderState, LlamaModelFamily, ParrotModelVariant,
        PiglinModelFamily, PlayerModelPartVisibility, SkeletonModelFamily, UndeadHorseModelFamily,
        VillagerModelData, ZombieVariantModelFamily, PLAYER_MODEL_PARTS_ALL_VISIBLE,
    },
    entity_model_root_transform,
    geometry::{
        append_dissolve_textured_mesh, append_scrolled_textured_mesh, argb_to_tint,
        emit_textured_model_cube, emit_textured_model_parts, fill_entity_textured_light,
        fill_entity_textured_overlay, part_pose_transform, EntityModelDissolveMesh,
        EntityModelMesh, EntityModelPortalMesh, EntityModelPortalVertex, EntityModelScrollMesh,
        EntityModelScrollVertex, EntityModelTexturedMesh, EntityModelTexturedVertex,
        EntityModelVertex, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
        ENTITY_VERTEX_FULL_BRIGHT_LIGHT, ENTITY_VERTEX_NO_OVERLAY,
    },
    instances::{BlockEntityCrumbling, EntityModelInstance},
    mesh_transformer_scaled_model_root_transform,
    model_layers::{
        armor_layer_tint, armor_slot_texture_for_layer, default_player_skin_texture_ref,
        end_crystal_bob_y, end_crystal_get_y, end_crystal_glass_quaternions, equine_body_pose,
        equine_head_pose, equine_leg_pose, equine_tail_pose, happy_ghast_harness_texture_ref,
        head_look_at_rest, horse_body_armor_texture_layers, limb_swing_at_rest,
        llama_body_decor_texture_ref, nautilus_body_armor_texture_ref,
        wolf_armor_crackiness_texture_ref, wolf_body_armor_texture_layers, ArmorStandModel,
        ArrowModel, BeeStingerModel, BoatWaterPatchModel, BreezeWindModel, CamelModel,
        CreeperModel, CustomHeadDragonSkullModel, CustomHeadPiglinSkullModel, CustomHeadSkullModel,
        ElytraModel, EquineAnimationPose, GuardianModel, HappyGhastHarnessModel, HappyGhastModel,
        HumanoidArmorModelLayerSet, HumanoidArmorSlot, HumanoidBabyArmorKind, LlamaModel,
        NautilusModel, ParrotModel, PigModel, PiglinModel, PlayerEarsModel, PlayerModel,
        SkeletonClothingModel, SkeletonModel, SpinAttackEffectModel, StriderModel, TridentModel,
        VillagerModel, WitherModel, WolfModel, ZombieModel, ZombieVariantModel,
        ADULT_DONKEY_PARTS_TEXTURED, ADULT_DONKEY_PARTS_WITH_CHEST_TEXTURED,
        ADULT_DONKEY_SADDLE_PARTS_TEXTURED, ADULT_DONKEY_SADDLE_RIDDEN_PARTS_TEXTURED,
        ADULT_HORSE_ARMOR_PARTS_TEXTURED, ADULT_HORSE_PARTS_TEXTURED,
        ADULT_HORSE_SADDLE_PARTS_TEXTURED, ADULT_HORSE_SADDLE_RIDDEN_PARTS_TEXTURED,
        ARROW_TEXTURE_REF, BABY_DONKEY_PARTS_TEXTURED, BABY_HORSE_PARTS_TEXTURED,
        BEE_STINGER_TEXTURE_REF, CAMEL_HUSK_SADDLE_TEXTURE_REF, CAMEL_SADDLE_TEXTURE_REF,
        CREEPER_TEXTURE_REF, DONKEY_SADDLE_TEXTURE_REF, DRAGON_FIREBALL_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF, ENDER_DRAGON_EXPLODING_TEXTURE_REF,
        ENDER_DRAGON_TEXTURE_REF, END_CRYSTAL_TEXTURED_PARTS, END_PORTAL_TEXTURE_REF,
        END_SKY_TEXTURE_REF, EQUINE_BABY_DONKEY_LEG_STAND_CONFIG, EQUINE_STANDARD_LEG_STAND_CONFIG,
        EXPERIENCE_ORB_TEXTURE_REF, FISHING_HOOK_TEXTURE_REF, HAPPY_GHAST_ROPES_TEXTURE_REF,
        HORSE_SADDLE_TEXTURE_REF, HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND,
        HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND_SMALL, HUMANOID_ARMOR_MODEL_LAYERS_BOGGED,
        HUMANOID_ARMOR_MODEL_LAYERS_DROWNED, HUMANOID_ARMOR_MODEL_LAYERS_DROWNED_BABY,
        HUMANOID_ARMOR_MODEL_LAYERS_GIANT, HUMANOID_ARMOR_MODEL_LAYERS_HUSK,
        HUMANOID_ARMOR_MODEL_LAYERS_HUSK_BABY, HUMANOID_ARMOR_MODEL_LAYERS_PARCHED,
        HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN, HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BABY,
        HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BRUTE, HUMANOID_ARMOR_MODEL_LAYERS_PLAYER,
        HUMANOID_ARMOR_MODEL_LAYERS_PLAYER_SLIM, HUMANOID_ARMOR_MODEL_LAYERS_SKELETON,
        HUMANOID_ARMOR_MODEL_LAYERS_STRAY, HUMANOID_ARMOR_MODEL_LAYERS_WITHER_SKELETON,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE, HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_BABY,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER_BABY,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN_BABY, LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        LLAMA_BODY_TRADER_TEXTURE_REF, MODEL_LAYER_CAMEL_HUSK_SADDLE, MODEL_LAYER_CAMEL_SADDLE,
        MODEL_LAYER_DONKEY_SADDLE, MODEL_LAYER_ELYTRA, MODEL_LAYER_ELYTRA_BABY,
        MODEL_LAYER_HAPPY_GHAST_BABY_HARNESS, MODEL_LAYER_HAPPY_GHAST_BABY_ROPES,
        MODEL_LAYER_HAPPY_GHAST_HARNESS, MODEL_LAYER_HAPPY_GHAST_ROPES, MODEL_LAYER_HORSE_ARMOR,
        MODEL_LAYER_HORSE_SADDLE, MODEL_LAYER_LLAMA_BABY_DECOR, MODEL_LAYER_LLAMA_DECOR,
        MODEL_LAYER_MULE_SADDLE, MODEL_LAYER_NAUTILUS_ARMOR, MODEL_LAYER_NAUTILUS_SADDLE,
        MODEL_LAYER_PIG_SADDLE, MODEL_LAYER_SKELETON_HORSE_SADDLE, MODEL_LAYER_STRIDER_SADDLE,
        MODEL_LAYER_UNDEAD_HORSE_ARMOR, MODEL_LAYER_WOLF_ARMOR, MODEL_LAYER_ZOMBIE_HORSE_SADDLE,
        MULE_SADDLE_TEXTURE_REF, NAUTILUS_SADDLE_TEXTURE_REF, PIGLIN_OUTER_ARMOR_DEFORMATION,
        PIGLIN_TEXTURE_REF, PIG_SADDLE_TEXTURE_REF, PLAYER_PROFILE_CAPE_TEXTURE_REF,
        PLAYER_PROFILE_ELYTRA_TEXTURE_REF, SKELETON_HORSE_SADDLE_TEXTURE_REF, SKELETON_TEXTURE_REF,
        STANDARD_OUTER_ARMOR_DEFORMATION, STRIDER_SADDLE_TEXTURE_REF, WITHER_SKELETON_TEXTURE_REF,
        ZOMBIE_HORSE_SADDLE_TEXTURE_REF, ZOMBIE_TEXTURE_REF,
    },
    player_model_root_transform, wither_skeleton_model_root_transform, HUSK_SCALE,
};
use crate::{
    block_destroy::{BlockDestroyOverlayMesh, BlockDestroyVertex},
    particles::{camera_billboard_orientation, ParticleItemPickupProjectileKind},
    sheeted_decal::sheeted_decal_uv,
};
use glam::{Mat4, Quat, Vec3};
use std::cmp::Ordering;

const PLAYER_CAPE_CUBE: TexturedModelCubeDesc = TexturedModelCubeDesc {
    min: [-5.0, 0.0, -1.0],
    size: [10.0, 16.0, 1.0],
    uv_size: [10.0, 16.0, 1.0],
    tex: [0.0, 0.0],
    mirror: false,
};
const PLAYER_CUSTOM_HEAD_LAYER_SUBMIT_SEQUENCE: u32 = 4;
const PLAYER_WINGS_LAYER_SUBMIT_SEQUENCE: u32 = 5;
const PLAYER_ARROW_LAYER_SUBMIT_SEQUENCE: u32 = 1;
const PLAYER_STINGER_LAYER_SUBMIT_SEQUENCE: u32 = 9;
const MAX_PLAYER_STUCK_ITEMS_PER_KIND: usize = 64;
const ARMOR_STAND_WINGS_LAYER_SUBMIT_SEQUENCE: u32 = 1;
const ARMOR_STAND_CUSTOM_HEAD_LAYER_SUBMIT_SEQUENCE: u32 = 2;
const NON_PLAYER_CUSTOM_HEAD_LAYER_SUBMIT_SEQUENCE: u32 = 1;
const NON_PLAYER_WINGS_LAYER_SUBMIT_SEQUENCE: u32 = 2;
const SKELETON_CLOTHING_LAYER_SUBMIT_SEQUENCE: u32 = 5;
const DRAGON_RAYS_RANDOM_SEED: i64 = 432;
const DRAGON_RAYS_RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const DRAGON_RAYS_RANDOM_INCREMENT: u64 = 11;
const DRAGON_RAYS_RANDOM_MASK: u64 = (1_u64 << 48) - 1;
const DRAGON_RAYS_HALF_SQRT_3: f32 = 0.866_025_4;
const END_PORTAL_BOTTOM: f32 = 0.375;
const END_PORTAL_HEIGHT: f32 = 0.375;

mod layers;
#[cfg(test)]
pub(super) use layers::player_textured_layer_passes;
pub(super) use layers::shulker_bullet_textured_layer_passes;
#[cfg(test)]
pub(super) use layers::warden_pulsating_spots_alpha;
pub(crate) use layers::EntityModelLayerRenderType;
pub(super) use layers::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    banner_textured_layer_passes, bed_textured_layer_passes, bell_textured_layer_passes,
    blaze_textured_layer_passes, boat_textured_layer_passes, book_textured_layer_passes,
    breeze_textured_layer_passes, camel_textured_layer_passes, chest_textured_layer_passes,
    chicken_textured_layer_passes, conduit_textured_layer_passes,
    copper_golem_textured_layer_passes, cow_textured_layer_passes, creaking_textured_layer_passes,
    creeper_textured_layer_passes, custom_head_skull_layer_pass,
    decorated_pot_textured_layer_passes, donkey_textured_layer_passes,
    drowned_textured_layer_passes, end_crystal_textured_layer_passes,
    end_gateway_beam_layer_passes, ender_dragon_textured_layer_passes,
    enderman_textured_layer_passes, endermite_textured_layer_passes, equipment_layer_pass,
    evoker_fangs_textured_layer_passes, feline_textured_layer_passes, fox_textured_layer_passes,
    frog_textured_layer_passes, ghast_textured_layer_passes, goat_textured_layer_passes,
    guardian_textured_layer_passes, happy_ghast_textured_layer_passes,
    hoglin_textured_layer_passes, horse_textured_layer_passes, humanoid_armor_layer_pass,
    husk_textured_layer_passes, illager_textured_layer_passes, iron_golem_textured_layer_passes,
    leash_knot_textured_layer_passes, llama_spit_textured_layer_passes,
    llama_textured_layer_passes, magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    mooshroom_textured_layer_passes, nautilus_textured_layer_passes, panda_textured_layer_passes,
    parrot_textured_layer_passes, phantom_textured_layer_passes, pig_textured_layer_passes,
    piglin_textured_layer_passes, player_cape_layer_pass,
    player_extra_ears_layer_pass_with_texture, player_parrot_on_shoulder_layer_pass,
    player_spin_attack_effect_layer_pass, player_textured_layer_passes_with_texture,
    polar_bear_textured_layer_passes, rabbit_textured_layer_passes, ravager_textured_layer_passes,
    salmon_textured_layer_passes, sheep_textured_layer_passes, shulker_box_textured_layer_passes,
    shulker_textured_layer_passes, sign_textured_layer_passes, silverfish_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, sniffer_textured_layer_passes,
    snow_golem_textured_layer_passes, spider_textured_layer_passes, squid_textured_layer_passes,
    tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, undead_horse_textured_layer_passes,
    villager_data_textured_layer_passes, villager_textured_layer_passes, villager_type_hat_visible,
    wandering_trader_textured_layer_passes, warden_textured_layer_passes,
    wind_charge_textured_layer_passes, wings_layer_pass, witch_textured_layer_passes,
    wither_skull_textured_layer_passes, wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_nautilus_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_data_textured_layer_passes, zombie_villager_textured_layer_passes,
    EntityModelLayerKind, EntityModelLayerPass, EntityModelLayerRenderBucket,
    EntityModelLayerVisibility,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityModelRenderSubmission {
    pub(super) render_type: EntityModelLayerRenderType,
    pub(super) texture: EntityModelTextureRef,
    /// Index into the selected shared entity atlas's owned-texture table.
    /// `None` means `texture` is a static catalog reference.
    pub(super) owned_texture: Option<u32>,
    pub(super) dissolve_texture: Option<EntityModelTextureRef>,
    pub(super) dynamic_player_skin: Option<EntityDynamicPlayerSkin>,
    pub(super) dynamic_player_texture: Option<EntityDynamicPlayerTexture>,
    pub(super) dynamic_texture: Option<EntityDynamicTexture>,
    pub(super) tint: [f32; 4],
    pub(super) transform: Mat4,
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) outline_color: u32,
    pub(super) order: i32,
    pub(super) submit_sequence: u32,
    pub(super) affects_crumbling: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityModelCustomGeometrySubmission {
    pub(super) render_type: EntityModelLayerRenderType,
    pub(super) transform: Mat4,
    pub(super) order: i32,
    pub(super) submit_sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityModelPortalSubmission {
    pub(super) render_type: EntityModelLayerRenderType,
    pub(super) sky_texture: EntityModelTextureRef,
    pub(super) portal_texture: EntityModelTextureRef,
    pub(super) tint: [f32; 4],
    pub(super) transform: Mat4,
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) outline_color: u32,
    pub(super) order: i32,
    pub(super) submit_sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ElderGuardianParticleRenderInstance {
    pub(crate) transform: Mat4,
    pub(crate) tint: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExperienceOrbPickupParticleRenderInstance {
    pub(crate) transform: Mat4,
    pub(crate) icon: i32,
    pub(crate) age_ticks: f32,
    pub(crate) light: [f32; 2],
}

/// One item-pickup particle whose carried model is an arrow / trident, posed at
/// the world-space interpolated pickup position (vanilla
/// `ItemPickupParticleGroup.State.submit` renders the extracted projectile
/// render state through the entity render dispatcher).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProjectilePickupParticleRenderInstance {
    pub(crate) transform: Mat4,
    pub(crate) kind: ParticleItemPickupProjectileKind,
    pub(crate) light: [f32; 2],
}

#[derive(Clone, Copy)]
struct EntityModelSubmitSortKey {
    order: i32,
    distance_sq: f32,
    insertion_index: usize,
}

struct PendingSortedTexturedUpload {
    target: PendingSortedTexturedTarget,
    sort_key: EntityModelSubmitSortKey,
    mesh: EntityModelTexturedMesh,
}

#[derive(Clone, Copy)]
enum PendingSortedTexturedTarget {
    Static(EntityModelLayerRenderType),
    DynamicPlayerSkin(EntityModelLayerRenderType),
    DynamicPlayerTexture(EntityModelLayerRenderType),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EntityModelTexturedDrawAtlas {
    Static,
    DynamicPlayerSkin,
    DynamicPlayerTexture,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityModelTexturedDrawRange {
    pub(crate) atlas: EntityModelTexturedDrawAtlas,
    pub(crate) render_type: EntityModelLayerRenderType,
    pub(crate) surface_cull: bool,
    pub(crate) order: i32,
    pub(crate) distance_sq: f32,
    pub(crate) insertion_index: usize,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityModelScrollDrawRange {
    pub(crate) render_type: EntityModelLayerRenderType,
    pub(crate) order: i32,
    pub(crate) distance_sq: f32,
    pub(crate) insertion_index: usize,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EntityModelTranslucentDrawRange {
    Textured(EntityModelTexturedDrawRange),
    Scroll(EntityModelScrollDrawRange),
    AdditiveScroll(EntityModelScrollDrawRange),
    BeaconBeamTranslucent(EntityModelScrollDrawRange),
    PositionColor(EntityModelPositionColorDrawRange),
    Portal(EntityModelPortalDrawRange),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityModelPositionColorDrawRange {
    pub(crate) render_type: EntityModelLayerRenderType,
    pub(crate) order: i32,
    pub(crate) distance_sq: f32,
    pub(crate) insertion_index: usize,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityModelPortalDrawRange {
    pub(crate) render_type: EntityModelLayerRenderType,
    pub(crate) order: i32,
    pub(crate) distance_sq: f32,
    pub(crate) insertion_index: usize,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

struct PendingSortedScrollUpload {
    render_type: EntityModelLayerRenderType,
    owner: EntityModelScrollOwner,
    sort_key: EntityModelSubmitSortKey,
    mesh: EntityModelScrollMesh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityModelScrollOwner {
    BreezeWind,
    EnergySwirl,
    GuardianBeam,
    EndCrystalBeam,
    EndGatewayBeamOpaque,
    EndGatewayBeamTranslucent,
}

impl EntityModelScrollOwner {
    const fn is_opaque_beam(self) -> bool {
        matches!(
            self,
            Self::GuardianBeam | Self::EndCrystalBeam | Self::EndGatewayBeamOpaque
        )
    }
}

pub(super) struct EntityModelTexturedMeshes {
    /// Posed model geometry duplicated through Vanilla's
    /// `SheetedDecalTextureGenerator` contract for block-entity break progress.
    pub(super) crumbling: BlockDestroyOverlayMesh,
    /// Vanilla `RenderTypes.entitySolid(texture)` geometry. Unlike cutout-cull,
    /// this cull-on state preserves sampled texels below alpha `0.1`.
    pub(super) solid: EntityModelTexturedMesh,
    pub(super) cutout: EntityModelTexturedMesh,
    pub(super) cutout_cull: EntityModelTexturedMesh,
    /// Vanilla `RenderTypes.entitySolidZOffsetForward(texture)` geometry. This owns a dedicated
    /// bucket because it uses the forward view-offset projection instead of the ordinary entity
    /// camera matrix.
    pub(super) solid_z_offset_forward: EntityModelTexturedMesh,
    /// Vanilla `RenderTypes.entityCutoutDissolve(dragon.png, dragon_exploding.png)` geometry (the
    /// dying ender dragon body). Its base UVs match the plain cutout bucket, but each vertex also
    /// carries a `mask_uv` set so the GPU dissolve pipeline can sample `dragon_exploding.png` for the
    /// per-fragment erosion; kept out of the plain cutout bucket so it can own that pipeline.
    pub(super) dissolve: EntityModelDissolveMesh,
    /// Vanilla `RenderTypes.entityCutoutZOffset(texture)` geometry. Its shader state matches
    /// `entityCutout`, but vanilla also applies `VIEW_OFFSET_Z_LAYERING`; keep it out of the plain
    /// cutout bucket so the GPU path can own the z-offset pipeline separately.
    pub(super) cutout_z_offset: EntityModelTexturedMesh,
    /// Vanilla `armorCutoutNoCull` geometry keeps the entity lightmap and per-face lighting, but
    /// uses the armor pipeline's no-overlay shader state.
    pub(super) armor_cutout: EntityModelTexturedMesh,
    /// Armor-trim geometry is separated from ordinary equipment cutout so it can draw after the
    /// armor foil pass while still sampling the shared runtime entity texture atlas.
    pub(super) armor_trim: EntityModelTexturedMesh,
    pub(super) armor_trim_decal: EntityModelTexturedMesh,
    pub(super) translucent: EntityModelTexturedMesh,
    /// Vanilla `armorTranslucent` geometry uses the dedicated armor translucent pipeline instead
    /// of the generic `entityTranslucent` shader.
    pub(super) armor_translucent: EntityModelTexturedMesh,
    pub(super) translucent_emissive: EntityModelTexturedMesh,
    pub(super) item_entity_translucent: EntityModelTexturedMesh,
    pub(super) item_entity_translucent_cull: EntityModelTexturedMesh,
    pub(super) eyes: EntityModelTexturedMesh,
    /// CPU-retained geometry for vanilla `RenderTypes.outline(...)` submissions. Submission metadata
    /// preserves the original model tint, while the folded vertices carry `outlineColor` like
    /// vanilla's `OutlineBufferSource.EntityOutlineGenerator`. This bucket is for vanilla
    /// `OUTLINE_NO_CULL`.
    pub(super) outline: EntityModelTexturedMesh,
    /// CPU-retained outline copies derived from source render types whose vanilla pipeline has
    /// culling enabled. Vanilla `RenderType.outline()` forwards `state.pipeline.isCull()`.
    pub(super) outline_cull: EntityModelTexturedMesh,
    /// Ready remote player skins are rendered through a dedicated atlas, preserving their vanilla
    /// cutout/translucent render type while swapping only the texture source.
    pub(super) dynamic_player_skin_solid: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_cutout: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_cutout_cull: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_solid_z_offset_forward: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_cutout_z_offset: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_translucent: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_item_entity_translucent: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_item_entity_translucent_cull: EntityModelTexturedMesh,
    /// Ready remote non-skin player profile textures, such as capes and elytra, use a separate
    /// variable-size atlas while preserving the vanilla render type.
    pub(super) dynamic_player_texture_solid: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_cutout: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_cutout_cull: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_solid_z_offset_forward: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_cutout_z_offset: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_armor_cutout: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_armor_trim: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_armor_trim_decal: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_translucent: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_item_entity_translucent: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_item_entity_translucent_cull: EntityModelTexturedMesh,
    /// Translucent scrolling overlay (vanilla `breezeWind` — the wind charge).
    pub(super) scroll: EntityModelScrollMesh,
    /// Additive scrolling overlay (vanilla `energySwirl` — the charged-creeper / wither glow).
    pub(super) scroll_additive: EntityModelScrollMesh,
    /// Vanilla `GuardianRenderer` custom beam geometry through exact `entityCutout` state.
    pub(super) guardian_beam: EntityModelScrollMesh,
    /// Vanilla opaque `endCrystalBeam` custom geometry.
    pub(super) end_crystal_beam: EntityModelScrollMesh,
    /// Vanilla opaque `beaconBeam` core used by the End Gateway.
    pub(super) end_gateway_beam_opaque: EntityModelScrollMesh,
    /// Vanilla translucent `beaconBeam` glow used by the End Gateway.
    pub(super) end_gateway_beam_translucent: EntityModelScrollMesh,
    /// Vanilla `entityGlint` folded through the glint texture transform.
    pub(super) entity_glint: EntityModelScrollMesh,
    /// Vanilla `armorEntityGlint` folded through the armor glint texture transform.
    pub(super) armor_entity_glint: EntityModelScrollMesh,
    /// Vanilla `RenderTypes.waterMask()` depth-only boat patch geometry.
    pub(super) water_mask: EntityModelMesh,
    /// Vanilla `RenderTypes.dragonRays()` position-colour death rays. The geometry is no-texture
    /// custom geometry from `EnderDragonRenderer.submitRays`.
    pub(super) dragon_rays: EntityModelMesh,
    /// Vanilla `RenderTypes.dragonRaysDepth()` replays the same ray shape into a depth-only pipeline.
    pub(super) dragon_rays_depth: EntityModelMesh,
    /// Vanilla `RenderTypes.leash()` rope strips. Untextured opaque position-color-lightmap custom
    /// geometry from `LeashFeatureRenderer.renderLeash`, one two-strip rope per leash state.
    pub(super) leash: EntityModelMesh,
    /// Vanilla `RenderTypes.endPortal()` block-entity cube faces. The GPU shader owns the 15-layer
    /// portal sampler formula; the mesh only carries world positions and atlas sub-rect metadata.
    pub(super) end_portal: EntityModelPortalMesh,
    /// Vanilla `RenderTypes.endGateway()` block-entity cube faces. The GPU shader owns the 16-layer
    /// gateway sampler formula; the gateway beam is emitted separately through the scroll bucket.
    pub(super) end_gateway: EntityModelPortalMesh,
    /// Vanilla-shaped submit metadata for textured entity models. The current backend still folds
    /// compatible submits into shared meshes, but this preserves render type, order, tint, texture,
    /// transform, light, and overlay so the folded GPU buckets can be audited against explicit
    /// submissions.
    pub(super) submissions: Vec<EntityModelRenderSubmission>,
    /// Vanilla-shaped submit metadata for no-texture custom geometry such as
    /// `EnderDragonRenderer.submitRays`.
    pub(super) custom_submissions: Vec<EntityModelCustomGeometrySubmission>,
    /// Vanilla-shaped submit metadata for `endPortal` / `endGateway`. These pipelines bind
    /// `end_sky.png` and `end_portal.png` together, so they cannot use either the single-texture
    /// entity submission or no-texture custom-geometry metadata.
    pub(super) portal_submissions: Vec<EntityModelPortalSubmission>,
    /// Opaque custom beam ranges preserve submit identity while sharing one GPU buffer per exact
    /// render state. They draw in the main solid feature phase, never the translucent plan.
    pub(super) opaque_beam_draws: Vec<EntityModelScrollDrawRange>,
    pub(super) sorted_main_translucent_draws: Vec<EntityModelTranslucentDrawRange>,
    pub(super) sorted_translucent_draws: Vec<EntityModelTexturedDrawRange>,
    pub(super) sorted_item_entity_draws: Vec<EntityModelTexturedDrawRange>,
    pending_sorted_textured_uploads: Vec<PendingSortedTexturedUpload>,
    pending_sorted_scroll_uploads: Vec<PendingSortedScrollUpload>,
    sort_camera_position: Option<[f32; 3]>,
    /// The camera pose at mesh-build time, consumed by camera-facing billboard renderers
    /// (`ExperienceOrbRenderer`-style `mulPose(camera.orientation)` quads). The GPU path rebuilds
    /// these meshes on every camera-pose change, so the baked orientation stays current.
    camera_pose: Option<crate::CameraPose>,
    current_submission_light: [f32; 2],
    current_submission_overlay: [f32; 2],
    current_submission_outline_color: u32,
    current_block_entity_crumbling: Option<BlockEntityCrumbling>,
    current_force_transparent: bool,
    current_outline_only: bool,
    next_submission_index: usize,
}

impl EntityModelTexturedMeshes {
    pub(in crate::entity_models) fn new(sort_camera_position: Option<[f32; 3]>) -> Self {
        Self {
            crumbling: BlockDestroyOverlayMesh::new(),
            solid: EntityModelTexturedMesh::new(),
            cutout: EntityModelTexturedMesh::new(),
            cutout_cull: EntityModelTexturedMesh::new(),
            solid_z_offset_forward: EntityModelTexturedMesh::new(),
            dissolve: EntityModelDissolveMesh::new(),
            cutout_z_offset: EntityModelTexturedMesh::new(),
            armor_cutout: EntityModelTexturedMesh::new(),
            armor_trim: EntityModelTexturedMesh::new(),
            armor_trim_decal: EntityModelTexturedMesh::new(),
            translucent: EntityModelTexturedMesh::new(),
            armor_translucent: EntityModelTexturedMesh::new(),
            translucent_emissive: EntityModelTexturedMesh::new(),
            item_entity_translucent: EntityModelTexturedMesh::new(),
            item_entity_translucent_cull: EntityModelTexturedMesh::new(),
            eyes: EntityModelTexturedMesh::new(),
            outline: EntityModelTexturedMesh::new(),
            outline_cull: EntityModelTexturedMesh::new(),
            dynamic_player_skin_solid: EntityModelTexturedMesh::new(),
            dynamic_player_skin_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_skin_cutout_cull: EntityModelTexturedMesh::new(),
            dynamic_player_skin_solid_z_offset_forward: EntityModelTexturedMesh::new(),
            dynamic_player_skin_cutout_z_offset: EntityModelTexturedMesh::new(),
            dynamic_player_skin_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_skin_item_entity_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_skin_item_entity_translucent_cull: EntityModelTexturedMesh::new(),
            dynamic_player_texture_solid: EntityModelTexturedMesh::new(),
            dynamic_player_texture_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_texture_cutout_cull: EntityModelTexturedMesh::new(),
            dynamic_player_texture_solid_z_offset_forward: EntityModelTexturedMesh::new(),
            dynamic_player_texture_cutout_z_offset: EntityModelTexturedMesh::new(),
            dynamic_player_texture_armor_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_texture_armor_trim: EntityModelTexturedMesh::new(),
            dynamic_player_texture_armor_trim_decal: EntityModelTexturedMesh::new(),
            dynamic_player_texture_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_texture_item_entity_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_texture_item_entity_translucent_cull: EntityModelTexturedMesh::new(),
            scroll: EntityModelScrollMesh::new(),
            scroll_additive: EntityModelScrollMesh::new(),
            guardian_beam: EntityModelScrollMesh::new(),
            end_crystal_beam: EntityModelScrollMesh::new(),
            end_gateway_beam_opaque: EntityModelScrollMesh::new(),
            end_gateway_beam_translucent: EntityModelScrollMesh::new(),
            entity_glint: EntityModelScrollMesh::new(),
            armor_entity_glint: EntityModelScrollMesh::new(),
            water_mask: EntityModelMesh::new(),
            dragon_rays: EntityModelMesh::new(),
            dragon_rays_depth: EntityModelMesh::new(),
            leash: EntityModelMesh::new(),
            end_portal: EntityModelPortalMesh::new(),
            end_gateway: EntityModelPortalMesh::new(),
            submissions: Vec::new(),
            custom_submissions: Vec::new(),
            portal_submissions: Vec::new(),
            opaque_beam_draws: Vec::new(),
            sorted_main_translucent_draws: Vec::new(),
            sorted_translucent_draws: Vec::new(),
            sorted_item_entity_draws: Vec::new(),
            pending_sorted_textured_uploads: Vec::new(),
            pending_sorted_scroll_uploads: Vec::new(),
            sort_camera_position,
            camera_pose: None,
            current_submission_light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            current_submission_overlay: ENTITY_VERTEX_NO_OVERLAY,
            current_submission_outline_color: 0,
            current_block_entity_crumbling: None,
            current_force_transparent: false,
            current_outline_only: false,
            next_submission_index: 0,
        }
    }

    fn mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type {
            EntityModelLayerRenderType::EntityCutoutZOffset => return &mut self.cutout_z_offset,
            EntityModelLayerRenderType::ArmorCutoutNoCull => return &mut self.armor_cutout,
            EntityModelLayerRenderType::ArmorTrimCutoutNoCull => return &mut self.armor_trim,
            EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull => {
                return &mut self.armor_trim_decal;
            }
            EntityModelLayerRenderType::ArmorTranslucent => return &mut self.armor_translucent,
            _ => {}
        }
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Solid => &mut self.solid,
            EntityModelLayerRenderBucket::Cutout => {
                if render_type.surface_cull() {
                    &mut self.cutout_cull
                } else {
                    &mut self.cutout
                }
            }
            EntityModelLayerRenderBucket::SolidZOffsetForward => &mut self.solid_z_offset_forward,
            EntityModelLayerRenderBucket::CutoutZOffset => &mut self.cutout_z_offset,
            EntityModelLayerRenderBucket::Translucent => &mut self.translucent,
            EntityModelLayerRenderBucket::TranslucentEmissive => &mut self.translucent_emissive,
            EntityModelLayerRenderBucket::ItemEntityTranslucent => {
                if render_type.surface_cull() {
                    &mut self.item_entity_translucent_cull
                } else {
                    &mut self.item_entity_translucent
                }
            }
            EntityModelLayerRenderBucket::Eyes => &mut self.eyes,
            EntityModelLayerRenderBucket::OutlineOnly => &mut self.outline,
            EntityModelLayerRenderBucket::Dissolve
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll
            | EntityModelLayerRenderBucket::EndCrystalBeam
            | EntityModelLayerRenderBucket::BeaconBeamOpaque
            | EntityModelLayerRenderBucket::BeaconBeamTranslucent
            | EntityModelLayerRenderBucket::PositionColor
            | EntityModelLayerRenderBucket::Portal => {
                panic!("scroll/dissolve render types are not emitted into textured mesh buckets")
            }
            EntityModelLayerRenderBucket::DepthOnly | EntityModelLayerRenderBucket::GlintOnly => {
                panic!("non-color render types are not emitted into textured mesh buckets")
            }
        }
    }

    fn dynamic_player_skin_mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Solid => &mut self.dynamic_player_skin_solid,
            EntityModelLayerRenderBucket::Cutout => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_skin_cutout_cull
                } else {
                    &mut self.dynamic_player_skin_cutout
                }
            }
            EntityModelLayerRenderBucket::CutoutZOffset => {
                &mut self.dynamic_player_skin_cutout_z_offset
            }
            EntityModelLayerRenderBucket::SolidZOffsetForward => {
                &mut self.dynamic_player_skin_solid_z_offset_forward
            }
            EntityModelLayerRenderBucket::Translucent => &mut self.dynamic_player_skin_translucent,
            EntityModelLayerRenderBucket::ItemEntityTranslucent => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_skin_item_entity_translucent_cull
                } else {
                    &mut self.dynamic_player_skin_item_entity_translucent
                }
            }
            EntityModelLayerRenderBucket::Eyes
            | EntityModelLayerRenderBucket::TranslucentEmissive
            | EntityModelLayerRenderBucket::Dissolve
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll
            | EntityModelLayerRenderBucket::EndCrystalBeam
            | EntityModelLayerRenderBucket::BeaconBeamOpaque
            | EntityModelLayerRenderBucket::BeaconBeamTranslucent
            | EntityModelLayerRenderBucket::PositionColor
            | EntityModelLayerRenderBucket::Portal
            | EntityModelLayerRenderBucket::OutlineOnly
            | EntityModelLayerRenderBucket::DepthOnly
            | EntityModelLayerRenderBucket::GlintOnly => {
                panic!("unsupported dynamic player skin render type")
            }
        }
    }

    fn dynamic_player_texture_mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        if render_type == EntityModelLayerRenderType::ArmorCutoutNoCull {
            return &mut self.dynamic_player_texture_armor_cutout;
        }
        if render_type == EntityModelLayerRenderType::ArmorTrimCutoutNoCull {
            return &mut self.dynamic_player_texture_armor_trim;
        }
        if render_type == EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull {
            return &mut self.dynamic_player_texture_armor_trim_decal;
        }
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Solid => &mut self.dynamic_player_texture_solid,
            EntityModelLayerRenderBucket::Cutout => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_texture_cutout_cull
                } else {
                    &mut self.dynamic_player_texture_cutout
                }
            }
            EntityModelLayerRenderBucket::CutoutZOffset => {
                &mut self.dynamic_player_texture_cutout_z_offset
            }
            EntityModelLayerRenderBucket::SolidZOffsetForward => {
                &mut self.dynamic_player_texture_solid_z_offset_forward
            }
            EntityModelLayerRenderBucket::Translucent => {
                &mut self.dynamic_player_texture_translucent
            }
            EntityModelLayerRenderBucket::ItemEntityTranslucent => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_texture_item_entity_translucent_cull
                } else {
                    &mut self.dynamic_player_texture_item_entity_translucent
                }
            }
            EntityModelLayerRenderBucket::Eyes
            | EntityModelLayerRenderBucket::TranslucentEmissive
            | EntityModelLayerRenderBucket::Dissolve
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll
            | EntityModelLayerRenderBucket::EndCrystalBeam
            | EntityModelLayerRenderBucket::BeaconBeamOpaque
            | EntityModelLayerRenderBucket::BeaconBeamTranslucent
            | EntityModelLayerRenderBucket::PositionColor
            | EntityModelLayerRenderBucket::Portal
            | EntityModelLayerRenderBucket::OutlineOnly
            | EntityModelLayerRenderBucket::DepthOnly
            | EntityModelLayerRenderBucket::GlintOnly => {
                panic!("unsupported dynamic player texture render type")
            }
        }
    }

    fn set_current_submission_state(&mut self, instance: EntityModelInstance) {
        self.current_submission_light = instance.render_state.shader_light();
        self.current_submission_overlay = instance.render_state.overlay_coords();
        self.current_submission_outline_color = instance.render_state.outline_color;
        self.current_block_entity_crumbling = instance.render_state.block_entity_crumbling;
        let illusioner_body_visible = instance.illusioner_body_visible_when_invisible();
        self.current_force_transparent = instance.render_state.invisible
            && !instance.render_state.invisible_to_player
            && !illusioner_body_visible;
        self.current_outline_only = instance.render_state.invisible
            && instance.render_state.invisible_to_player
            && instance.render_state.appears_glowing
            && !illusioner_body_visible;
    }

    pub(in crate::entity_models) fn current_invisible_base_only(&self) -> bool {
        self.current_force_transparent || self.current_outline_only
    }

    fn record_submission(
        &mut self,
        submit: EntityModelSubmissionEmit,
    ) -> EntityModelRenderSubmission {
        self.record_submission_with_index(submit).0
    }

    fn record_submission_with_index(
        &mut self,
        submit: EntityModelSubmissionEmit,
    ) -> (EntityModelRenderSubmission, usize) {
        let mut submission = EntityModelRenderSubmission::from(submit);
        submission.light = submit.light.unwrap_or(self.current_submission_light);
        submission.overlay = submit.overlay.unwrap_or(self.current_submission_overlay);
        submission.outline_color = submit
            .outline_color
            .unwrap_or(self.current_submission_outline_color);
        let insertion_index = self.next_submission_index;
        self.next_submission_index += 1;
        self.submissions.push(submission);
        (submission, insertion_index)
    }

    fn record_custom_submission_with_index(
        &mut self,
        submission: EntityModelCustomGeometrySubmission,
    ) -> (EntityModelCustomGeometrySubmission, usize) {
        let insertion_index = self.next_submission_index;
        self.next_submission_index += 1;
        self.custom_submissions.push(submission);
        (submission, insertion_index)
    }

    fn record_portal_submission_with_index(
        &mut self,
        render_type: EntityModelLayerRenderType,
        transform: Mat4,
    ) -> (EntityModelPortalSubmission, usize) {
        let submission = EntityModelPortalSubmission {
            render_type,
            sky_texture: END_SKY_TEXTURE_REF,
            portal_texture: END_PORTAL_TEXTURE_REF,
            tint: [1.0; 4],
            transform,
            light: self.current_submission_light,
            overlay: self.current_submission_overlay,
            outline_color: self.current_submission_outline_color,
            order: 0,
            submit_sequence: 0,
        };
        let insertion_index = self.next_submission_index;
        self.next_submission_index += 1;
        self.portal_submissions.push(submission);
        (submission, insertion_index)
    }

    fn sorted_upload_key(
        &self,
        submission: EntityModelRenderSubmission,
        insertion_index: usize,
    ) -> Option<EntityModelSubmitSortKey> {
        if !submission.render_type.has_blending() {
            return None;
        }
        let camera_position = self.sort_camera_position?;
        let position = submission.transform.transform_point3(Vec3::ZERO);
        let camera = Vec3::from_array(camera_position);
        let distance_sq = (position - camera).length_squared();
        Some(EntityModelSubmitSortKey {
            order: submission.order,
            distance_sq: if distance_sq.is_finite() {
                distance_sq
            } else {
                0.0
            },
            insertion_index,
        })
    }

    fn custom_geometry_draw_key(
        &self,
        submission: EntityModelCustomGeometrySubmission,
        insertion_index: usize,
    ) -> Option<EntityModelSubmitSortKey> {
        let camera_position = self.sort_camera_position?;
        let position = submission.transform.transform_point3(Vec3::ZERO);
        let camera = Vec3::from_array(camera_position);
        let distance_sq = (position - camera).length_squared();
        Some(EntityModelSubmitSortKey {
            order: submission.order,
            distance_sq: if distance_sq.is_finite() {
                distance_sq
            } else {
                0.0
            },
            insertion_index,
        })
    }

    fn portal_draw_key(
        &self,
        submission: EntityModelPortalSubmission,
        insertion_index: usize,
    ) -> Option<EntityModelSubmitSortKey> {
        let camera_position = self.sort_camera_position?;
        let position = submission.transform.transform_point3(Vec3::ZERO);
        let camera = Vec3::from_array(camera_position);
        let distance_sq = (position - camera).length_squared();
        Some(EntityModelSubmitSortKey {
            order: submission.order,
            distance_sq: if distance_sq.is_finite() {
                distance_sq
            } else {
                0.0
            },
            insertion_index,
        })
    }

    fn push_position_color_draw_range(
        &mut self,
        render_type: EntityModelLayerRenderType,
        sort_key: EntityModelSubmitSortKey,
        index_start: u32,
        index_count: u32,
    ) {
        if index_count == 0 {
            return;
        }
        self.sorted_main_translucent_draws
            .push(EntityModelTranslucentDrawRange::PositionColor(
                EntityModelPositionColorDrawRange {
                    render_type,
                    order: sort_key.order,
                    distance_sq: sort_key.distance_sq,
                    insertion_index: sort_key.insertion_index,
                    index_start,
                    index_count,
                },
            ));
    }

    fn push_portal_draw_range(
        &mut self,
        render_type: EntityModelLayerRenderType,
        sort_key: EntityModelSubmitSortKey,
        index_start: u32,
        index_count: u32,
    ) {
        if index_count == 0 {
            return;
        }
        self.sorted_main_translucent_draws
            .push(EntityModelTranslucentDrawRange::Portal(
                EntityModelPortalDrawRange {
                    render_type,
                    order: sort_key.order,
                    distance_sq: sort_key.distance_sq,
                    insertion_index: sort_key.insertion_index,
                    index_start,
                    index_count,
                },
            ));
    }

    fn push_scroll_draw_range(
        &mut self,
        render_type: EntityModelLayerRenderType,
        owner: EntityModelScrollOwner,
        sort_key: EntityModelSubmitSortKey,
        index_start: u32,
        index_count: u32,
    ) {
        if index_count == 0 {
            return;
        }
        let draw = EntityModelScrollDrawRange {
            render_type,
            order: sort_key.order,
            distance_sq: sort_key.distance_sq,
            insertion_index: sort_key.insertion_index,
            index_start,
            index_count,
        };
        match owner {
            EntityModelScrollOwner::BreezeWind => self
                .sorted_main_translucent_draws
                .push(EntityModelTranslucentDrawRange::Scroll(draw)),
            EntityModelScrollOwner::EnergySwirl => self
                .sorted_main_translucent_draws
                .push(EntityModelTranslucentDrawRange::AdditiveScroll(draw)),
            EntityModelScrollOwner::EndGatewayBeamTranslucent => self
                .sorted_main_translucent_draws
                .push(EntityModelTranslucentDrawRange::BeaconBeamTranslucent(draw)),
            EntityModelScrollOwner::GuardianBeam
            | EntityModelScrollOwner::EndCrystalBeam
            | EntityModelScrollOwner::EndGatewayBeamOpaque => {
                unreachable!("opaque scroll owners never enter the translucent sort plan")
            }
        }
    }

    fn flush_sorted_uploads(&mut self) {
        self.pending_sorted_textured_uploads
            .sort_by(|left, right| compare_submit_sort_key(left.sort_key, right.sort_key));
        for upload in std::mem::take(&mut self.pending_sorted_textured_uploads) {
            let render_type = match upload.target {
                PendingSortedTexturedTarget::Static(render_type)
                | PendingSortedTexturedTarget::DynamicPlayerSkin(render_type)
                | PendingSortedTexturedTarget::DynamicPlayerTexture(render_type) => render_type,
            };
            let bucket = render_type.mesh_bucket();
            let surface_cull = render_type.surface_cull();
            let index_count = u32::try_from(upload.mesh.indices.len())
                .expect("sorted textured index count fits in u32");
            let (atlas, index_start) = match upload.target {
                PendingSortedTexturedTarget::Static(render_type) => (
                    EntityModelTexturedDrawAtlas::Static,
                    append_textured_mesh(self.mesh_mut(render_type), upload.mesh),
                ),
                PendingSortedTexturedTarget::DynamicPlayerSkin(render_type) => (
                    EntityModelTexturedDrawAtlas::DynamicPlayerSkin,
                    append_textured_mesh(
                        self.dynamic_player_skin_mesh_mut(render_type),
                        upload.mesh,
                    ),
                ),
                PendingSortedTexturedTarget::DynamicPlayerTexture(render_type) => (
                    EntityModelTexturedDrawAtlas::DynamicPlayerTexture,
                    append_textured_mesh(
                        self.dynamic_player_texture_mesh_mut(render_type),
                        upload.mesh,
                    ),
                ),
            };
            let draw = EntityModelTexturedDrawRange {
                atlas,
                render_type,
                surface_cull,
                order: upload.sort_key.order,
                distance_sq: upload.sort_key.distance_sq,
                insertion_index: upload.sort_key.insertion_index,
                index_start,
                index_count,
            };
            match bucket {
                EntityModelLayerRenderBucket::Translucent
                | EntityModelLayerRenderBucket::TranslucentEmissive
                    if index_count > 0 =>
                {
                    self.sorted_translucent_draws.push(draw);
                    self.sorted_main_translucent_draws
                        .push(EntityModelTranslucentDrawRange::Textured(draw));
                }
                EntityModelLayerRenderBucket::Eyes if index_count > 0 => {
                    self.sorted_main_translucent_draws
                        .push(EntityModelTranslucentDrawRange::Textured(draw));
                }
                EntityModelLayerRenderBucket::ItemEntityTranslucent if index_count > 0 => {
                    self.sorted_item_entity_draws.push(draw);
                }
                _ => {}
            }
        }

        self.pending_sorted_scroll_uploads
            .sort_by(|left, right| compare_submit_sort_key(left.sort_key, right.sort_key));
        for upload in std::mem::take(&mut self.pending_sorted_scroll_uploads) {
            let index_count = u32::try_from(upload.mesh.indices.len())
                .expect("sorted scroll index count fits in u32");
            let index_start =
                append_scroll_mesh(scroll_owner_mesh_mut(self, upload.owner), upload.mesh);
            self.push_scroll_draw_range(
                upload.render_type,
                upload.owner,
                upload.sort_key,
                index_start,
                index_count,
            );
        }
        self.opaque_beam_draws.sort_by(|left, right| {
            compare_submit_sort_key(
                EntityModelSubmitSortKey {
                    order: left.order,
                    distance_sq: left.distance_sq,
                    insertion_index: left.insertion_index,
                },
                EntityModelSubmitSortKey {
                    order: right.order,
                    distance_sq: right.distance_sq,
                    insertion_index: right.insertion_index,
                },
            )
        });
        self.sorted_main_translucent_draws.sort_by(|left, right| {
            compare_submit_sort_key(
                translucent_draw_sort_key(*left),
                translucent_draw_sort_key(*right),
            )
        });
    }
}

fn translucent_draw_sort_key(draw: EntityModelTranslucentDrawRange) -> EntityModelSubmitSortKey {
    match draw {
        EntityModelTranslucentDrawRange::Textured(draw) => EntityModelSubmitSortKey {
            order: draw.order,
            distance_sq: draw.distance_sq,
            insertion_index: draw.insertion_index,
        },
        EntityModelTranslucentDrawRange::Scroll(draw)
        | EntityModelTranslucentDrawRange::AdditiveScroll(draw)
        | EntityModelTranslucentDrawRange::BeaconBeamTranslucent(draw) => {
            EntityModelSubmitSortKey {
                order: draw.order,
                distance_sq: draw.distance_sq,
                insertion_index: draw.insertion_index,
            }
        }
        EntityModelTranslucentDrawRange::PositionColor(draw) => EntityModelSubmitSortKey {
            order: draw.order,
            distance_sq: draw.distance_sq,
            insertion_index: draw.insertion_index,
        },
        EntityModelTranslucentDrawRange::Portal(draw) => EntityModelSubmitSortKey {
            order: draw.order,
            distance_sq: draw.distance_sq,
            insertion_index: draw.insertion_index,
        },
    }
}

fn compare_submit_sort_key(
    left: EntityModelSubmitSortKey,
    right: EntityModelSubmitSortKey,
) -> Ordering {
    left.order
        .cmp(&right.order)
        .then_with(|| {
            right
                .distance_sq
                .partial_cmp(&left.distance_sq)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.insertion_index.cmp(&right.insertion_index))
}

#[derive(Clone, Copy)]
struct EntityModelSubmissionEmit {
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    owned_texture: Option<u32>,
    dissolve_texture: Option<EntityModelTextureRef>,
    dynamic_player_skin: Option<EntityDynamicPlayerSkin>,
    dynamic_player_texture: Option<EntityDynamicPlayerTexture>,
    dynamic_texture: Option<EntityDynamicTexture>,
    tint: [f32; 4],
    transform: Mat4,
    light: Option<[f32; 2]>,
    overlay: Option<[f32; 2]>,
    outline_color: Option<u32>,
    order: i32,
    submit_sequence: u32,
    affects_crumbling: bool,
}

impl EntityModelSubmissionEmit {
    fn new(
        render_type: EntityModelLayerRenderType,
        texture: EntityModelTextureRef,
        tint: [f32; 4],
        transform: Mat4,
        order: i32,
        submit_sequence: u32,
    ) -> Self {
        Self {
            render_type,
            texture,
            owned_texture: None,
            dissolve_texture: None,
            dynamic_player_skin: None,
            dynamic_player_texture: None,
            dynamic_texture: None,
            tint,
            transform,
            light: None,
            overlay: None,
            outline_color: None,
            order,
            submit_sequence,
            affects_crumbling: render_type.affects_crumbling(),
        }
    }

    fn with_dynamic_player_skin(mut self, skin: EntityDynamicPlayerSkin) -> Self {
        self.dynamic_player_skin = Some(skin);
        self
    }

    fn with_owned_texture(mut self, index: u32) -> Self {
        self.owned_texture = Some(index);
        self
    }

    fn with_dynamic_player_texture(mut self, texture: EntityDynamicPlayerTexture) -> Self {
        self.dynamic_player_texture = Some(texture);
        self.dynamic_texture = Some(texture.dynamic_texture());
        self
    }

    fn with_dynamic_texture(mut self, texture: EntityDynamicTexture) -> Self {
        self.dynamic_texture = Some(texture);
        self
    }

    fn with_dissolve_texture(mut self, texture: EntityModelTextureRef) -> Self {
        self.dissolve_texture = Some(texture);
        self
    }

    fn with_overlay(mut self, overlay: [f32; 2]) -> Self {
        self.overlay = Some(overlay);
        self
    }

    fn with_light(mut self, light: [f32; 2]) -> Self {
        self.light = Some(light);
        self
    }

    fn with_affects_crumbling(mut self, affects_crumbling: bool) -> Self {
        self.affects_crumbling = affects_crumbling;
        self
    }
}

impl From<EntityModelSubmissionEmit> for EntityModelRenderSubmission {
    fn from(submit: EntityModelSubmissionEmit) -> Self {
        Self {
            render_type: submit.render_type,
            texture: submit.texture,
            owned_texture: submit.owned_texture,
            dissolve_texture: submit.dissolve_texture,
            dynamic_player_skin: submit.dynamic_player_skin,
            dynamic_player_texture: submit.dynamic_player_texture,
            dynamic_texture: submit.dynamic_texture,
            tint: submit.tint,
            transform: submit.transform,
            light: submit.light.unwrap_or(ENTITY_VERTEX_FULL_BRIGHT_LIGHT),
            overlay: submit.overlay.unwrap_or(ENTITY_VERTEX_NO_OVERLAY),
            outline_color: submit.outline_color.unwrap_or(0),
            order: submit.order,
            submit_sequence: submit.submit_sequence,
            affects_crumbling: submit.affects_crumbling,
        }
    }
}

#[cfg(test)]
pub(super) fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures(instances, atlas, None, None)
}

#[cfg(test)]
pub(super) fn entity_model_textured_meshes_with_dynamic_skins(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures(
        instances,
        atlas,
        dynamic_player_skin_atlas,
        None,
    )
}

#[cfg(test)]
pub(super) fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures_for_camera(
        instances,
        atlas,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        None,
        None,
    )
}

#[cfg(test)]
pub(super) fn entity_model_textured_meshes_with_dynamic_textures_for_camera(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    sort_camera_position: Option<[f32; 3]>,
    camera_pose: Option<crate::CameraPose>,
) -> EntityModelTexturedMeshes {
    let equipment_assets = ResolvedEntityEquipmentAssetCatalog::legacy_static_fallback();
    entity_model_textured_meshes_with_equipment_for_camera(
        instances,
        atlas,
        &[],
        &equipment_assets,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        sort_camera_position,
        camera_pose,
    )
}

pub(super) fn entity_model_textured_meshes_with_equipment_for_camera(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    sort_camera_position: Option<[f32; 3]>,
    camera_pose: Option<crate::CameraPose>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(sort_camera_position);
    meshes.camera_pose = camera_pose;
    for instance in instances {
        // Vanilla `EntityRenderer.submit` submits leash ropes before any model or layer work and
        // without the invisibility gates below, so the rope emits ahead of both dispatch branches.
        render_leash_ropes(&mut meshes, *instance);
        if instance.render_state.invisible
            && instance.render_state.invisible_to_player
            && !instance.render_state.appears_glowing
            && !instance.illusioner_body_visible_when_invisible()
        {
            meshes.set_current_submission_state(*instance);
            // Vanilla `LivingEntityRenderer` still runs layers when the base body has no render
            // type. Keep only the layers whose own submit path has no `state.isInvisible` gate.
            let mut sink = TexturedSink {
                meshes: &mut meshes,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_player_skin_atlas,
                dynamic_player_texture_atlas,
            };
            dispatch_invisible_living_ungated_layers(instance, &mut sink, 0);
            continue;
        }
        meshes.set_current_submission_state(*instance);
        {
            let mut sink = TexturedSink {
                meshes: &mut meshes,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_player_skin_atlas,
                dynamic_player_texture_atlas,
            };
            dispatch_uniform_entity_model(instance, &mut sink);
        }
        if meshes.current_force_transparent || meshes.current_outline_only {
            // Keep vanilla layer exceptions for invisible bodies while preserving the existing gate
            // for invisible-gated helpers such as player capes and ears.
            let mut sink = TexturedSink {
                meshes: &mut meshes,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_player_skin_atlas,
                dynamic_player_texture_atlas,
            };
            dispatch_invisible_living_ungated_layers(instance, &mut sink, 1);
            continue;
        }
        // HumanoidArmorLayer, CustomHeadLayer skulls, WingsLayer, player-only late layers,
        // SkeletonClothingLayer, and villager profession overlays are dispatch-owned here in the
        // same registration order as their vanilla renderer constructors.
        {
            let mut sink = TexturedSink {
                meshes: &mut meshes,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_player_skin_atlas,
                dynamic_player_texture_atlas,
            };
            dispatch_vanilla_entity_layers(instance, &mut sink);
        }
    }
    meshes.flush_sorted_uploads();
    meshes
}

pub(super) fn elder_guardian_particle_textured_meshes(
    instances: &[ElderGuardianParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    let model = GuardianModel::new();
    let base_pass = guardian_textured_layer_passes(true)[0];
    for (index, instance) in instances.iter().copied().enumerate() {
        let submit_sequence =
            u32::try_from(index).expect("elder guardian particle submit count fits in u32");
        let pass = EntityModelLayerPass {
            render_type: EntityModelLayerRenderType::EntityTranslucent,
            tint: instance.tint,
            submit_sequence,
            ..base_pass
        };
        let submit = no_overlay_layer_submission_with_tint(pass, instance.tint, instance.transform)
            .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);
        render_textured_submission(&mut meshes, submit, atlas, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
    }
    meshes.flush_sorted_uploads();
    meshes
}

/// Bakes the item-pickup carried arrow / trident models. Vanilla
/// `ItemPickupParticleGroup.State.submit` runs the picked-up projectile's
/// extracted render state through `EntityRenderDispatcher.submit`
/// (`ArrowRenderer` / `ThrownTridentRenderer`); bbb draws the pickup group into
/// the blended particles target, so the passes are forced to
/// `EntityTranslucent` (alpha stays 1.0) like the elder guardian particle
/// model. The trident foil keeps its vanilla second submit
/// (`entityGlint` over the same model), carried here as a statically-sampled
/// glint pass for the 3-tick flash.
pub(super) fn projectile_pickup_particle_textured_meshes(
    instances: &[ProjectilePickupParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    let mut submit_sequence = 0_u32;
    for instance in instances.iter().copied() {
        match instance.kind {
            ParticleItemPickupProjectileKind::Arrow
            | ParticleItemPickupProjectileKind::TippedArrow
            | ParticleItemPickupProjectileKind::SpectralArrow => {
                let texture = match instance.kind {
                    ParticleItemPickupProjectileKind::TippedArrow => ArrowModelTexture::Tipped,
                    ParticleItemPickupProjectileKind::SpectralArrow => ArrowModelTexture::Spectral,
                    _ => ArrowModelTexture::Normal,
                };
                render_projectile_pickup_particle_pass(
                    &mut meshes,
                    &ArrowModel::new(),
                    arrow_textured_layer_passes(texture)[0],
                    instance,
                    &mut submit_sequence,
                    atlas,
                );
            }
            ParticleItemPickupProjectileKind::Trident { foil } => {
                let model = TridentModel::new();
                let passes = trident_textured_layer_passes();
                render_projectile_pickup_particle_pass(
                    &mut meshes,
                    &model,
                    passes[0],
                    instance,
                    &mut submit_sequence,
                    atlas,
                );
                if foil {
                    render_projectile_pickup_particle_pass(
                        &mut meshes,
                        &model,
                        passes[1],
                        instance,
                        &mut submit_sequence,
                        atlas,
                    );
                }
            }
        }
    }
    meshes.flush_sorted_uploads();
    meshes
}

fn render_projectile_pickup_particle_pass<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    base_pass: EntityModelLayerPass,
    instance: ProjectilePickupParticleRenderInstance,
    submit_sequence: &mut u32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let pass = EntityModelLayerPass {
        render_type: EntityModelLayerRenderType::EntityTranslucent,
        submit_sequence: *submit_sequence,
        ..base_pass
    };
    *submit_sequence += 1;
    let submit = no_overlay_layer_submission(pass, instance.transform).with_light(instance.light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

pub(super) fn experience_orb_pickup_particle_textured_mesh(
    instances: &[ExperienceOrbPickupParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMesh {
    let mut mesh = EntityModelTexturedMesh::new();
    let Some(entry) = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == EXPERIENCE_ORB_TEXTURE_REF)
    else {
        return mesh;
    };

    for instance in instances {
        append_experience_orb_pickup_particle_quad(&mut mesh, *instance, entry.uv);
    }
    mesh
}

fn append_experience_orb_pickup_particle_quad(
    mesh: &mut EntityModelTexturedMesh,
    instance: ExperienceOrbPickupParticleRenderInstance,
    texture_uv: EntityModelUvRect,
) {
    let tint = experience_orb_pickup_particle_tint(instance.age_ticks);
    append_experience_orb_quad(
        mesh,
        instance.transform,
        instance.icon,
        tint,
        instance.light,
        texture_uv,
    );
}

/// Vanilla `ExperienceOrbRenderer.submit` quad: the shared billboard corners on the 16px icon
/// cell `icon = ExperienceOrb.getIcon()` of the 64×64 `entity/experience/experience_orb.png`
/// sheet.
fn append_experience_orb_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    icon: i32,
    tint: [f32; 4],
    light: [f32; 2],
    texture_uv: EntityModelUvRect,
) {
    let icon = icon.clamp(0, 10);
    let icon_u0 = (icon % 4 * 16) as f32 / 64.0;
    let icon_u1 = (icon % 4 * 16 + 16) as f32 / 64.0;
    let icon_v0 = (icon / 4 * 16) as f32 / 64.0;
    let icon_v1 = (icon / 4 * 16 + 16) as f32 / 64.0;
    append_camera_billboard_quad(
        mesh,
        transform,
        [
            [icon_u0, icon_v1],
            [icon_u1, icon_v1],
            [icon_u1, icon_v0],
            [icon_u0, icon_v0],
        ],
        tint,
        light,
        texture_uv,
    );
}

/// The camera-billboard quad shared by vanilla `ExperienceOrbRenderer` and
/// `DragonFireballRenderer` `vertex` helpers: corners `(x - 0.5, y - 0.25, 0)` for x/y in
/// {0, 1} with a +Y normal, UVs supplied per renderer.
fn append_camera_billboard_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    uvs: [[f32; 2]; 4],
    tint: [f32; 4],
    light: [f32; 2],
    texture_uv: EntityModelUvRect,
) {
    let normal = transform
        .transform_vector3(Vec3::Y)
        .normalize_or_zero()
        .to_array();
    let corners = [
        Vec3::new(-0.5, -0.25, 0.0),
        Vec3::new(0.5, -0.25, 0.0),
        Vec3::new(0.5, 0.75, 0.0),
        Vec3::new(-0.5, 0.75, 0.0),
    ];
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend(
        corners
            .iter()
            .copied()
            .zip(uvs.iter().copied())
            .map(|(corner, uv)| EntityModelTexturedVertex {
                position: (transform * corner.extend(1.0)).truncate().to_array(),
                uv: normalized_entity_texture_uv(uv, texture_uv),
                tint,
                light,
                overlay: ENTITY_VERTEX_NO_OVERLAY,
                normal,
            }),
    );
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.cutout_faces += 1;
}

fn normalized_entity_texture_uv(uv: [f32; 2], rect: EntityModelUvRect) -> [f32; 2] {
    [
        rect.min[0] + uv[0] * (rect.max[0] - rect.min[0]),
        rect.min[1] + uv[1] * (rect.max[1] - rect.min[1]),
    ]
}

/// Vanilla 26.1 `ExperienceOrbRenderer.submit`: translate up 0.1, face the camera
/// (`mulPose(camera.orientation)`), scale 0.3, and emit one icon quad as
/// `RenderTypes.entityTranslucentCullItemTarget` with the age-driven red/blue sine tint at
/// alpha 128. Without a camera pose (no frame rendered yet) the billboard has no orientation and
/// is skipped; the mesh rebuild on the first camera update restores it.
pub(in crate::entity_models) fn render_experience_orb(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let EntityModelKind::ExperienceOrb { icon } = instance.kind else {
        return;
    };
    let Some(pose) = meshes.camera_pose else {
        return;
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position) + Vec3::Y * 0.1)
        * camera_billboard_orientation(pose)
        * Mat4::from_scale(Vec3::splat(0.3));
    let tint = experience_orb_pickup_particle_tint(instance.render_state.age_in_ticks);
    let light = instance.render_state.shader_light();
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EntityTranslucentCullItemTarget,
        EXPERIENCE_ORB_TEXTURE_REF,
        tint,
        transform,
        0,
        0,
    )
    .with_light(light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        append_experience_orb_quad(mesh, transform, icon, tint, light, entry.uv);
    });
}

/// Vanilla 26.1 `DragonFireballRenderer.submit`: scale 2, face the camera
/// (`mulPose(camera.orientation)`), and emit one full-sheet white quad as
/// `RenderTypes.entityCutout`. The constant `getBlockLightLevel` = 15 arrives through the
/// instance light coords (generated static-full-bright renderer catalog). Without a camera pose
/// (no frame rendered yet) the billboard has no orientation and is skipped; the mesh rebuild on
/// the first camera update restores it.
pub(in crate::entity_models) fn render_dragon_fireball(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::DragonFireball) {
        return;
    }
    let Some(pose) = meshes.camera_pose else {
        return;
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_scale(Vec3::splat(2.0))
        * camera_billboard_orientation(pose);
    let light = instance.render_state.shader_light();
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EntityCutout,
        DRAGON_FIREBALL_TEXTURE_REF,
        [1.0; 4],
        transform,
        0,
        0,
    )
    .with_light(light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        append_camera_billboard_quad(
            mesh,
            transform,
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            submit.tint,
            light,
            entry.uv,
        );
    });
}

/// Vanilla 26.1 `FishingHookRenderer.submit`: scale by one half, face the camera,
/// submit the full fishing-hook sheet through `entityCutoutCull`, then submit the
/// owner-dependent parabola through `RenderTypes.lines()`.
pub(in crate::entity_models) fn render_fishing_hook(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::FishingHook)
        || instance.render_state.fishing_line_origin_offset.is_none()
    {
        return;
    }
    let Some(pose) = meshes.camera_pose else {
        return;
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_scale(Vec3::splat(0.5))
        * camera_billboard_orientation(pose);
    let light = instance.render_state.shader_light();
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EntityCutoutCull,
        FISHING_HOOK_TEXTURE_REF,
        [1.0; 4],
        transform,
        0,
        0,
    )
    .with_light(light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        append_fishing_hook_quad(mesh, transform, light, entry.uv);
    });
    meshes.record_custom_submission_with_index(EntityModelCustomGeometrySubmission {
        render_type: EntityModelLayerRenderType::Lines,
        transform: Mat4::from_translation(Vec3::from_array(instance.position)),
        order: 0,
        submit_sequence: 1,
    });
}

fn append_fishing_hook_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    light: [f32; 2],
    texture_uv: EntityModelUvRect,
) {
    let normal = transform
        .transform_vector3(Vec3::Y)
        .normalize_or_zero()
        .to_array();
    let corners = [
        (Vec3::new(-0.5, -0.5, 0.0), [0.0, 1.0]),
        (Vec3::new(0.5, -0.5, 0.0), [1.0, 1.0]),
        (Vec3::new(0.5, 0.5, 0.0), [1.0, 0.0]),
        (Vec3::new(-0.5, 0.5, 0.0), [0.0, 0.0]),
    ];
    let base = mesh.vertices.len() as u32;
    mesh.vertices
        .extend(corners.map(|(corner, uv)| EntityModelTexturedVertex {
            position: (transform * corner.extend(1.0)).truncate().to_array(),
            uv: normalized_entity_texture_uv(uv, texture_uv),
            tint: [1.0; 4],
            light,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal,
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.cutout_faces += 1;
}

/// Vanilla `FishingHookRenderer.stringVertex` sampled over its fixed 16-step
/// parabola. The state offset ends at the rendered hand because its Y component
/// is measured from `(hookPosition + (0, 0.25, 0))`.
pub(crate) fn fishing_hook_line_segments(
    instance: EntityModelInstance,
    line_width: f32,
) -> Option<[crate::SelectionLine; 16]> {
    if !matches!(instance.kind, EntityModelKind::FishingHook) {
        return None;
    }
    let [x, y, z] = instance.render_state.fishing_line_origin_offset?;
    let point = |fraction: f32| {
        [
            instance.position[0] + x * fraction,
            instance.position[1] + y * (fraction * fraction + fraction) * 0.5 + 0.25,
            instance.position[2] + z * fraction,
        ]
    };
    Some(std::array::from_fn(|step| {
        let from = step as f32 / 16.0;
        let to = (step + 1) as f32 / 16.0;
        crate::SelectionLine {
            from: point(from),
            to: point(to),
            color: [0.0, 0.0, 0.0, 1.0],
            width: line_width,
        }
    }))
}

fn experience_orb_pickup_particle_tint(age_ticks: f32) -> [f32; 4] {
    let rr = age_ticks / 2.0;
    let red = (((rr).sin() + 1.0) * 0.5 * 255.0) as i32;
    let blue = (((rr + std::f32::consts::PI * 4.0 / 3.0).sin() + 1.0) * 0.1 * 255.0) as i32;
    [
        red.clamp(0, 255) as f32 / 255.0,
        1.0,
        blue.clamp(0, 255) as f32 / 255.0,
        128.0 / 255.0,
    ]
}

pub(super) fn first_person_player_arm_textured_meshes(
    arms: &[FirstPersonPlayerArm],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    for (index, arm) in arms.iter().copied().enumerate() {
        render_first_person_player_arm(
            &mut meshes,
            arm,
            index as u32,
            atlas,
            dynamic_player_skin_atlas,
        );
    }
    meshes.flush_sorted_uploads();
    meshes
}

fn render_first_person_player_arm(
    meshes: &mut EntityModelTexturedMeshes,
    arm: FirstPersonPlayerArm,
    submit_sequence: u32,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let arm_name = if arm.left { "left_arm" } else { "right_arm" };
    let mut model = PlayerModel::new(arm.skin.is_slim());
    let root = model.root_mut();
    let selected = root.child_mut(arm_name);
    selected.reset_pose();
    selected.visible = true;
    selected.pose.rotation[2] = if arm.left { -0.1 } else { 0.1 };
    selected.child_mut("sleeve").visible = arm.sleeve_visible;

    let texture = default_player_skin_texture_ref(arm.skin.fallback());
    let mut pass = player_textured_layer_passes_with_texture(
        arm.skin.is_slim(),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
        texture,
    )[0];
    pass.render_type = EntityModelLayerRenderType::EntityTranslucent;
    pass.submit_sequence = submit_sequence;
    let submit = textured_layer_submission(meshes, pass, arm.transform)
        .with_light(arm.light)
        .with_overlay(ENTITY_VERTEX_NO_OVERLAY);
    let Some(arm_part) = model.root().try_child(arm_name) else {
        return;
    };

    if let EntityPlayerSkin::Dynamic(dynamic_player_skin) = arm.skin {
        let submit = submit.with_dynamic_player_skin(dynamic_player_skin);
        if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
            if let Some(entry) = dynamic_player_skin_atlas_entry(
                dynamic_player_skin_atlas,
                dynamic_player_skin.handle,
            ) {
                render_textured_dynamic_player_skin_submission(
                    meshes,
                    submit,
                    entry,
                    |mesh, entry| {
                        arm_part.render_textured(
                            mesh,
                            submit.transform,
                            submit.texture,
                            entry.uv,
                            submit.tint,
                        );
                    },
                );
                return;
            }
        }

        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            arm_part.render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        return;
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        arm_part.render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

pub(super) fn entity_model_water_mask_mesh(instances: &[EntityModelInstance]) -> EntityModelMesh {
    let mut mesh = EntityModelMesh::new();
    for instance in instances {
        if instance.render_state.invisible {
            continue;
        }
        if let Some(submit) = boat_water_mask_submission(*instance) {
            append_boat_water_mask_mesh(&mut mesh, submit);
        }
    }
    mesh
}

#[cfg(test)]
pub(super) fn dynamic_player_texture_test_meshes(
    render_type: EntityModelLayerRenderType,
    dynamic_player_texture: EntityDynamicPlayerTexture,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    let model = PlayerModel::new(false);
    render_textured_pass_with_dynamic_player_texture(
        &mut meshes,
        &model,
        Mat4::IDENTITY,
        render_type,
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        dynamic_player_texture,
        [0.25, 0.5, 0.75, 1.0],
        atlas,
        dynamic_player_texture_atlas,
    );
    meshes
}

/// Vanilla `EndCrystalRenderer.submit`: render `EndCrystalModel` with `end_crystal.png` after the
/// renderer root transform (`scale(2)` + `translate(0,-0.5,0)`). The optional `DATA_BEAM_TARGET`
/// custom geometry is submitted separately by [`render_end_crystal_beam`].
pub(in crate::entity_models) fn render_end_crystal_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    transform: Mat4,
    instance: &EntityModelInstance,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    for pass in passes {
        let submit = textured_layer_submission(meshes, pass, transform);
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            if instance.render_state.end_crystal_shows_bottom {
                emit_textured_model_parts(
                    mesh,
                    &END_CRYSTAL_TEXTURED_PARTS[..1],
                    submit.transform,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            }

            let age = instance.render_state.age_in_ticks;
            let bob = end_crystal_bob_y(age);
            let (q_outer, q_inner) = end_crystal_glass_quaternions(age);
            let centre = submit.transform
                * part_pose_transform(PartPose {
                    offset: [0.0, 24.0 + bob, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                });
            let outer_t = centre * Mat4::from_quat(q_outer);
            let inner_t = outer_t * Mat4::from_quat(q_inner);
            let core_t = inner_t * Mat4::from_quat(q_inner);
            for cube in END_CRYSTAL_TEXTURED_PARTS[1].cubes {
                emit_textured_model_cube(
                    mesh,
                    outer_t,
                    *cube,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            }
            for cube in END_CRYSTAL_TEXTURED_PARTS[2].cubes {
                emit_textured_model_cube(
                    mesh,
                    inner_t,
                    *cube,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            }
            for cube in END_CRYSTAL_TEXTURED_PARTS[3].cubes {
                emit_textured_model_cube(
                    mesh,
                    core_t,
                    *cube,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            }
        });
    }
}

fn render_textured_pass_with_dynamic_player_skin<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    pass: EntityModelLayerPass,
    dynamic_player_skin: EntityDynamicPlayerSkin,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let submit = textured_layer_submission(meshes, pass, transform)
        .with_dynamic_player_skin(dynamic_player_skin);
    if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
        if let Some(entry) =
            dynamic_player_skin_atlas_entry(dynamic_player_skin_atlas, dynamic_player_skin.handle)
        {
            render_textured_dynamic_player_skin_submission(meshes, submit, entry, |mesh, entry| {
                model.root().render_textured(
                    mesh,
                    submit.transform,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            });
            return;
        }
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn render_textured_no_overlay_layer_pass<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    pass: EntityModelLayerPass,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let submit = no_overlay_layer_submission(pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn render_textured_no_overlay_layer_pass_with_dynamic_player_skin<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    pass: EntityModelLayerPass,
    dynamic_player_skin: EntityDynamicPlayerSkin,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let submit =
        no_overlay_layer_submission(pass, transform).with_dynamic_player_skin(dynamic_player_skin);
    if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
        if let Some(entry) =
            dynamic_player_skin_atlas_entry(dynamic_player_skin_atlas, dynamic_player_skin.handle)
        {
            render_textured_dynamic_player_skin_submission(meshes, submit, entry, |mesh, entry| {
                model.root().render_textured(
                    mesh,
                    submit.transform,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            });
            return;
        }
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

#[cfg(test)]
fn render_textured_pass_with_dynamic_player_texture<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    dynamic_player_texture: EntityDynamicPlayerTexture,
    tint: [f32; 4],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let submit = EntityModelSubmissionEmit::new(render_type, texture, tint, transform, 0, 0)
        .with_dynamic_player_texture(dynamic_player_texture);
    if let Some(entry) =
        dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, dynamic_player_texture)
    {
        render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        return;
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn render_textured_dynamic_player_skin_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    entry: EntityDynamicPlayerSkinAtlasEntry,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityDynamicPlayerSkinAtlasEntry),
) {
    let (submission, insertion_index) = meshes.record_submission_with_index(submit);
    let current_crumbling = meshes.current_block_entity_crumbling;
    if let Some(sort_key) = meshes.sorted_upload_key(submission, insertion_index) {
        let mut mesh = EntityModelTexturedMesh::new();
        emit(&mut mesh, entry);
        fill_textured_submission_vertices(&mut mesh, 0, submission);
        if let Some(copy) = submission_crumbling_copy(&mesh, 0, 0, submission, current_crumbling) {
            append_crumbling_copy(&mut meshes.crumbling, copy);
        }
        meshes
            .pending_sorted_textured_uploads
            .push(PendingSortedTexturedUpload {
                target: PendingSortedTexturedTarget::DynamicPlayerSkin(submit.render_type),
                sort_key,
                mesh,
            });
        return;
    }

    let crumbling_copy = {
        let mesh = meshes.dynamic_player_skin_mesh_mut(submit.render_type);
        let vertex_start = mesh.vertices.len();
        let index_start = mesh.indices.len();
        emit(mesh, entry);
        fill_textured_submission_vertices(mesh, vertex_start, submission);
        submission_crumbling_copy(
            mesh,
            vertex_start,
            index_start,
            submission,
            current_crumbling,
        )
    };
    if let Some(copy) = crumbling_copy {
        append_crumbling_copy(&mut meshes.crumbling, copy);
    }
}

fn render_textured_dynamic_player_texture_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    entry: EntityDynamicPlayerTextureAtlasEntry,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityDynamicPlayerTextureAtlasEntry),
) {
    let (submission, insertion_index) = meshes.record_submission_with_index(submit);
    let current_crumbling = meshes.current_block_entity_crumbling;
    if let Some(sort_key) = meshes.sorted_upload_key(submission, insertion_index) {
        let mut mesh = EntityModelTexturedMesh::new();
        emit(&mut mesh, entry);
        fill_textured_submission_vertices(&mut mesh, 0, submission);
        if let Some(copy) = submission_crumbling_copy(&mesh, 0, 0, submission, current_crumbling) {
            append_crumbling_copy(&mut meshes.crumbling, copy);
        }
        meshes
            .pending_sorted_textured_uploads
            .push(PendingSortedTexturedUpload {
                target: PendingSortedTexturedTarget::DynamicPlayerTexture(submit.render_type),
                sort_key,
                mesh,
            });
        return;
    }

    let crumbling_copy = {
        let mesh = meshes.dynamic_player_texture_mesh_mut(submit.render_type);
        let vertex_start = mesh.vertices.len();
        let index_start = mesh.indices.len();
        emit(mesh, entry);
        fill_textured_submission_vertices(mesh, vertex_start, submission);
        submission_crumbling_copy(
            mesh,
            vertex_start,
            index_start,
            submission,
            current_crumbling,
        )
    };
    if let Some(copy) = crumbling_copy {
        append_crumbling_copy(&mut meshes.crumbling, copy);
    }
}

const PAINTING_DYNAMIC_TEXTURE_FALLBACK_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/painting/back.png",
    size: [16, 16],
};

/// Emits one tile of vanilla `PaintingRenderer.renderPainting` custom geometry. The scene expands
/// the complete painting into independently lit tile instances; this helper retains the exact
/// front partitioning, full back sprite, and boundary-only thin edges on the `paintings` atlas.
pub(in crate::entity_models) fn render_painting_tile(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::PaintingTile {
        width,
        height,
        segment_x,
        segment_y,
        facing,
        front,
        back,
    } = instance.kind
    else {
        return;
    };
    if width <= 0
        || height <= 0
        || !(0..width).contains(&segment_x)
        || !(0..height).contains(&segment_y)
        || matches!(
            facing,
            super::catalog::EntityAttachmentFace::Down | super::catalog::EntityAttachmentFace::Up
        )
        || meshes.current_outline_only
    {
        return;
    }
    let Some(front_entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, front) else {
        return;
    };
    let Some(back_entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, back) else {
        return;
    };
    let yaw: f32 = match facing {
        super::catalog::EntityAttachmentFace::North => 0.0,
        super::catalog::EntityAttachmentFace::South => 180.0,
        super::catalog::EntityAttachmentFace::West => 90.0,
        super::catalog::EntityAttachmentFace::East => -90.0,
        super::catalog::EntityAttachmentFace::Down | super::catalog::EntityAttachmentFace::Up => {
            unreachable!("vertical painting facings were rejected")
        }
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y(yaw.to_radians());
    let (render_type, tint) = if meshes.current_force_transparent {
        (
            EntityModelLayerRenderType::EntityTranslucentCullItemTarget,
            [1.0, 1.0, 1.0, 38.0 / 255.0],
        )
    } else {
        (
            EntityModelLayerRenderType::EntitySolidZOffsetForward,
            [1.0; 4],
        )
    };
    let submit = EntityModelSubmissionEmit::new(
        render_type,
        PAINTING_DYNAMIC_TEXTURE_FALLBACK_REF,
        tint,
        transform,
        0,
        0,
    )
    .with_dynamic_texture(front);
    render_textured_dynamic_player_texture_submission(
        meshes,
        submit,
        front_entry,
        |mesh, front_entry| {
            append_painting_tile_geometry(
                mesh,
                transform,
                width,
                height,
                segment_x,
                segment_y,
                front_entry.uv,
                back_entry.uv,
            );
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn append_painting_tile_geometry(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    width: i32,
    height: i32,
    segment_x: i32,
    segment_y: i32,
    front: EntityModelUvRect,
    back: EntityModelUvRect,
) {
    let offset_x = -(width as f32) / 2.0;
    let offset_y = -(height as f32) / 2.0;
    let x0 = offset_x + (segment_x + 1) as f32;
    let x1 = offset_x + segment_x as f32;
    let y0 = offset_y + (segment_y + 1) as f32;
    let y1 = offset_y + segment_y as f32;
    let edge = 0.03125;
    let front_u0 = (width - segment_x) as f32 / width as f32;
    let front_u1 = (width - (segment_x + 1)) as f32 / width as f32;
    let front_v0 = (height - segment_y) as f32 / height as f32;
    let front_v1 = (height - (segment_y + 1)) as f32 / height as f32;
    append_painting_quad(
        mesh,
        transform,
        [
            ([x0, y1, -edge], painting_uv(front, front_u1, front_v0)),
            ([x1, y1, -edge], painting_uv(front, front_u0, front_v0)),
            ([x1, y0, -edge], painting_uv(front, front_u0, front_v1)),
            ([x0, y0, -edge], painting_uv(front, front_u1, front_v1)),
        ],
        [0.0, 0.0, -1.0],
    );
    append_painting_quad(
        mesh,
        transform,
        [
            ([x0, y0, edge], painting_uv(back, 1.0, 0.0)),
            ([x1, y0, edge], painting_uv(back, 0.0, 0.0)),
            ([x1, y1, edge], painting_uv(back, 0.0, 1.0)),
            ([x0, y1, edge], painting_uv(back, 1.0, 1.0)),
        ],
        [0.0, 0.0, 1.0],
    );
    if segment_y == height - 1 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x0, y0, -edge], painting_uv(back, 0.0, 0.0)),
                ([x1, y0, -edge], painting_uv(back, 1.0, 0.0)),
                ([x1, y0, edge], painting_uv(back, 1.0, 1.0 / 16.0)),
                ([x0, y0, edge], painting_uv(back, 0.0, 1.0 / 16.0)),
            ],
            [0.0, 1.0, 0.0],
        );
    }
    if segment_y == 0 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x0, y1, edge], painting_uv(back, 0.0, 0.0)),
                ([x1, y1, edge], painting_uv(back, 1.0, 0.0)),
                ([x1, y1, -edge], painting_uv(back, 1.0, 1.0 / 16.0)),
                ([x0, y1, -edge], painting_uv(back, 0.0, 1.0 / 16.0)),
            ],
            [0.0, -1.0, 0.0],
        );
    }
    if segment_x == width - 1 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x0, y0, edge], painting_uv(back, 1.0 / 16.0, 0.0)),
                ([x0, y1, edge], painting_uv(back, 1.0 / 16.0, 1.0)),
                ([x0, y1, -edge], painting_uv(back, 0.0, 1.0)),
                ([x0, y0, -edge], painting_uv(back, 0.0, 0.0)),
            ],
            [-1.0, 0.0, 0.0],
        );
    }
    if segment_x == 0 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x1, y0, -edge], painting_uv(back, 1.0 / 16.0, 0.0)),
                ([x1, y1, -edge], painting_uv(back, 1.0 / 16.0, 1.0)),
                ([x1, y1, edge], painting_uv(back, 0.0, 1.0)),
                ([x1, y0, edge], painting_uv(back, 0.0, 0.0)),
            ],
            [1.0, 0.0, 0.0],
        );
    }
}

fn painting_uv(rect: EntityModelUvRect, u: f32, v: f32) -> [f32; 2] {
    [
        rect.min[0] + u * (rect.max[0] - rect.min[0]),
        rect.min[1] + v * (rect.max[1] - rect.min[1]),
    ]
}

fn append_painting_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    vertices: [([f32; 3], [f32; 2]); 4],
    normal: [f32; 3],
) {
    let base = u32::try_from(mesh.vertices.len()).expect("painting vertex count fits in u32");
    let normal = transform
        .inverse()
        .transpose()
        .transform_vector3(Vec3::from_array(normal))
        .normalize_or_zero()
        .to_array();
    mesh.vertices.extend(vertices.map(|(position, uv)| {
        EntityModelTexturedVertex {
            position: transform
                .transform_point3(Vec3::from_array(position))
                .to_array(),
            uv,
            tint: [1.0; 4],
            light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal,
        }
    }));
    let a = Vec3::from_array(mesh.vertices[base as usize].position);
    let b = Vec3::from_array(mesh.vertices[(base + 1) as usize].position);
    let c = Vec3::from_array(mesh.vertices[(base + 2) as usize].position);
    if (b - a).cross(c - a).dot(Vec3::from_array(normal)) >= 0.0 {
        mesh.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    } else {
        // Vanilla emits quads and lets the backend expand them. Normalize the triangle-list
        // expansion against the submitted normal so the dedicated entity-solid cull state keeps
        // every front/back/edge face visible on wgpu.
        mesh.indices
            .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    mesh.cutout_faces += 1;
}

fn render_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    render_textured_submission_with_owned(meshes, submit, atlas, &[], emit);
}

fn render_textured_submission_with_owned(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    let (submission, insertion_index) = meshes.record_submission_with_index(submit);
    let current_crumbling = meshes.current_block_entity_crumbling;
    if submit
        .dissolve_texture
        .is_some_and(|texture| entity_model_texture_atlas_entry(atlas, texture).is_none())
    {
        return;
    }
    let entry = if let Some(index) = submit.owned_texture {
        owned_entries
            .get(index as usize)
            .map(|entry| EntityModelTextureAtlasEntry {
                texture: submit.texture,
                uv: entry.uv,
            })
    } else {
        entity_model_texture_atlas_entry(atlas, submit.texture)
    };
    if let Some(entry) = entry {
        if submission.render_type.mesh_bucket() == EntityModelLayerRenderBucket::GlintOnly {
            let mut scratch = EntityModelTexturedMesh::new();
            emit(&mut scratch, entry);
            fill_textured_submission_vertices(&mut scratch, 0, submission);
            append_scrolled_textured_mesh(
                glint_mesh_mut(meshes, submission.render_type),
                &scratch,
                entry.uv,
                [0.0, 0.0],
            );
            return;
        }
        if submission.render_type.mesh_bucket() == EntityModelLayerRenderBucket::Dissolve {
            // The mask atlas entry is guaranteed present here: a dissolve submission whose
            // `dissolve_texture` is missing from the atlas is suppressed by the early return above.
            let mask_uv_rect = submit
                .dissolve_texture
                .and_then(|texture| entity_model_texture_atlas_entry(atlas, texture))
                .map(|mask_entry| mask_entry.uv)
                .expect("dissolve submission without a mask atlas entry is suppressed earlier");
            let mut scratch = EntityModelTexturedMesh::new();
            emit(&mut scratch, entry);
            fill_textured_submission_vertices(&mut scratch, 0, submission);
            if let Some(copy) =
                submission_crumbling_copy(&scratch, 0, 0, submission, current_crumbling)
            {
                append_crumbling_copy(&mut meshes.crumbling, copy);
            }
            append_dissolve_textured_mesh(&mut meshes.dissolve, &scratch, entry.uv, mask_uv_rect);
            return;
        }
        if let Some(sort_key) = meshes.sorted_upload_key(submission, insertion_index) {
            let mut mesh = EntityModelTexturedMesh::new();
            emit(&mut mesh, entry);
            fill_textured_submission_vertices(&mut mesh, 0, submission);
            if let Some(copy) =
                submission_crumbling_copy(&mesh, 0, 0, submission, current_crumbling)
            {
                append_crumbling_copy(&mut meshes.crumbling, copy);
            }
            if submission_writes_outline_buffer(submission) {
                let (vertices, indices, cutout_faces) =
                    outline_copy_from_submission(&mesh, 0, 0, 0, submission.outline_color);
                let outline = if submission.render_type.outline_cull() {
                    &mut meshes.outline_cull
                } else {
                    &mut meshes.outline
                };
                append_textured_outline_copy(outline, vertices, indices, cutout_faces);
            }
            meshes
                .pending_sorted_textured_uploads
                .push(PendingSortedTexturedUpload {
                    target: PendingSortedTexturedTarget::Static(submit.render_type),
                    sort_key,
                    mesh,
                });
            return;
        }

        let (outline_copy, crumbling_copy) = {
            let mesh = meshes.mesh_mut(submit.render_type);
            let vertex_start = mesh.vertices.len();
            let index_start = mesh.indices.len();
            let face_start = mesh.cutout_faces;
            emit(mesh, entry);
            fill_textured_submission_vertices(mesh, vertex_start, submission);
            let outline_copy = if submission_writes_outline_buffer(submission) {
                Some(outline_copy_from_submission(
                    mesh,
                    vertex_start,
                    index_start,
                    face_start,
                    submission.outline_color,
                ))
            } else {
                None
            };
            let crumbling_copy = submission_crumbling_copy(
                mesh,
                vertex_start,
                index_start,
                submission,
                current_crumbling,
            );
            (outline_copy, crumbling_copy)
        };
        if let Some((vertices, indices, cutout_faces)) = outline_copy {
            let outline = if submission.render_type.outline_cull() {
                &mut meshes.outline_cull
            } else {
                &mut meshes.outline
            };
            append_textured_outline_copy(outline, vertices, indices, cutout_faces);
        }
        if let Some(copy) = crumbling_copy {
            append_crumbling_copy(&mut meshes.crumbling, copy);
        }
    }
}

fn append_textured_mesh(
    dest: &mut EntityModelTexturedMesh,
    source: EntityModelTexturedMesh,
) -> u32 {
    let index_start = u32::try_from(dest.indices.len()).expect("textured index count fits in u32");
    let base = u32::try_from(dest.vertices.len()).expect("textured vertex count fits in u32");
    dest.vertices.extend(source.vertices);
    dest.indices
        .extend(source.indices.into_iter().map(|index| index + base));
    dest.cutout_faces += source.cutout_faces;
    index_start
}

fn append_scroll_mesh(dest: &mut EntityModelScrollMesh, source: EntityModelScrollMesh) -> u32 {
    let index_start = u32::try_from(dest.indices.len()).expect("scroll index count fits in u32");
    let base = u32::try_from(dest.vertices.len()).expect("scroll vertex count fits in u32");
    dest.vertices.extend(source.vertices);
    dest.indices
        .extend(source.indices.into_iter().map(|index| index + base));
    index_start
}

fn append_portal_mesh(dest: &mut EntityModelPortalMesh, source: EntityModelPortalMesh) -> u32 {
    let index_start = u32::try_from(dest.indices.len()).expect("portal index count fits in u32");
    let base = u32::try_from(dest.vertices.len()).expect("portal vertex count fits in u32");
    dest.vertices.extend(source.vertices);
    dest.indices
        .extend(source.indices.into_iter().map(|index| index + base));
    index_start
}

fn append_entity_model_mesh(dest: &mut EntityModelMesh, source: EntityModelMesh) -> u32 {
    let index_start = u32::try_from(dest.indices.len()).expect("entity index count fits in u32");
    let base = u32::try_from(dest.vertices.len()).expect("entity vertex count fits in u32");
    dest.vertices.extend(source.vertices);
    dest.indices
        .extend(source.indices.into_iter().map(|index| index + base));
    dest.opaque_faces += source.opaque_faces;
    index_start
}

fn submission_writes_outline_buffer(submission: EntityModelRenderSubmission) -> bool {
    submission.outline_color != 0 && submission.render_type.affects_outline()
}

fn outline_copy_from_submission(
    mesh: &EntityModelTexturedMesh,
    vertex_start: usize,
    index_start: usize,
    face_start: usize,
    outline_color: u32,
) -> (Vec<EntityModelTexturedVertex>, Vec<u32>, usize) {
    let outline_tint = argb_to_tint(outline_color);
    let vertices = mesh.vertices[vertex_start..]
        .iter()
        .map(|vertex| {
            let mut vertex = *vertex;
            vertex.tint = outline_tint;
            vertex
        })
        .collect();
    let base = u32::try_from(vertex_start).expect("textured vertex start fits in u32");
    let indices = mesh.indices[index_start..]
        .iter()
        .map(|index| {
            debug_assert!(*index >= base);
            index - base
        })
        .collect();
    (vertices, indices, mesh.cutout_faces - face_start)
}

fn append_textured_outline_copy(
    outline: &mut EntityModelTexturedMesh,
    vertices: Vec<EntityModelTexturedVertex>,
    indices: Vec<u32>,
    cutout_faces: usize,
) {
    let base = u32::try_from(outline.vertices.len()).expect("outline vertex count fits in u32");
    outline.vertices.extend(vertices);
    outline
        .indices
        .extend(indices.into_iter().map(|index| index + base));
    outline.cutout_faces += cutout_faces;
}

fn crumbling_copy_from_submission(
    mesh: &EntityModelTexturedMesh,
    vertex_start: usize,
    index_start: usize,
    crumbling: BlockEntityCrumbling,
) -> BlockDestroyOverlayMesh {
    let uv_rect_size = [
        crumbling.uv.max[0] - crumbling.uv.min[0],
        crumbling.uv.max[1] - crumbling.uv.min[1],
    ];
    let vertices = mesh.vertices[vertex_start..]
        .iter()
        .map(|vertex| {
            let local_position = [
                vertex.position[0] - crumbling.origin[0],
                vertex.position[1] - crumbling.origin[1],
                vertex.position[2] - crumbling.origin[2],
            ];
            BlockDestroyVertex {
                position: vertex.position,
                normal: vertex.normal,
                decal_uv: sheeted_decal_uv(local_position, vertex.normal, 1.0),
                uv_rect_min: crumbling.uv.min,
                uv_rect_size,
            }
        })
        .collect();
    let source_base = u32::try_from(vertex_start).expect("textured vertex start fits in u32");
    let indices = mesh.indices[index_start..]
        .iter()
        .map(|index| {
            debug_assert!(*index >= source_base);
            index - source_base
        })
        .collect();
    BlockDestroyOverlayMesh { vertices, indices }
}

fn append_crumbling_copy(
    destination: &mut BlockDestroyOverlayMesh,
    source: BlockDestroyOverlayMesh,
) {
    let base = u32::try_from(destination.vertices.len())
        .expect("block-entity crumbling vertex count fits in u32");
    destination.vertices.extend(source.vertices);
    destination
        .indices
        .extend(source.indices.into_iter().map(|index| index + base));
}

fn submission_crumbling_copy(
    mesh: &EntityModelTexturedMesh,
    vertex_start: usize,
    index_start: usize,
    submission: EntityModelRenderSubmission,
    current: Option<BlockEntityCrumbling>,
) -> Option<BlockDestroyOverlayMesh> {
    submission
        .affects_crumbling
        .then_some(current)
        .flatten()
        .map(|crumbling| crumbling_copy_from_submission(mesh, vertex_start, index_start, crumbling))
}

fn fill_textured_submission_vertices(
    mesh: &mut EntityModelTexturedMesh,
    start: usize,
    submission: EntityModelRenderSubmission,
) {
    fill_entity_textured_light(mesh, start, submission.light);
    fill_entity_textured_overlay(mesh, start, submission.overlay);
    if submission.render_type == EntityModelLayerRenderType::Outline {
        let outline_tint = argb_to_tint(submission.outline_color);
        for vertex in &mut mesh.vertices[start..] {
            vertex.tint = outline_tint;
        }
    }
}

fn scroll_owner_for_render_type(render_type: EntityModelLayerRenderType) -> EntityModelScrollOwner {
    match render_type {
        EntityModelLayerRenderType::BreezeWind => EntityModelScrollOwner::BreezeWind,
        EntityModelLayerRenderType::EnergySwirl => EntityModelScrollOwner::EnergySwirl,
        EntityModelLayerRenderType::EndCrystalBeam => EntityModelScrollOwner::EndCrystalBeam,
        EntityModelLayerRenderType::EndGatewayBeamOpaque => {
            EntityModelScrollOwner::EndGatewayBeamOpaque
        }
        EntityModelLayerRenderType::EndGatewayBeamTranslucent => {
            EntityModelScrollOwner::EndGatewayBeamTranslucent
        }
        _ => panic!("only dedicated scroll render types are emitted through the scroll mesh"),
    }
}

fn scroll_owner_mesh_mut(
    meshes: &mut EntityModelTexturedMeshes,
    owner: EntityModelScrollOwner,
) -> &mut EntityModelScrollMesh {
    match owner {
        EntityModelScrollOwner::BreezeWind => &mut meshes.scroll,
        EntityModelScrollOwner::EnergySwirl => &mut meshes.scroll_additive,
        EntityModelScrollOwner::GuardianBeam => &mut meshes.guardian_beam,
        EntityModelScrollOwner::EndCrystalBeam => &mut meshes.end_crystal_beam,
        EntityModelScrollOwner::EndGatewayBeamOpaque => &mut meshes.end_gateway_beam_opaque,
        EntityModelScrollOwner::EndGatewayBeamTranslucent => {
            &mut meshes.end_gateway_beam_translucent
        }
    }
}

fn glint_mesh_mut(
    meshes: &mut EntityModelTexturedMeshes,
    render_type: EntityModelLayerRenderType,
) -> &mut EntityModelScrollMesh {
    match render_type {
        EntityModelLayerRenderType::EntityGlint => &mut meshes.entity_glint,
        EntityModelLayerRenderType::ArmorEntityGlint => &mut meshes.armor_entity_glint,
        _ => panic!("only glint render types are emitted through glint meshes"),
    }
}

pub(in crate::entity_models) fn render_boat_water_mask_submission(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let Some(submit) = boat_water_mask_submission(instance) else {
        return;
    };
    meshes.record_submission(submit);
    append_boat_water_mask_mesh(&mut meshes.water_mask, submit);
}

fn boat_water_mask_submission(instance: EntityModelInstance) -> Option<EntityModelSubmissionEmit> {
    let EntityModelKind::Boat { family, chest } = instance.kind else {
        return None;
    };
    // Vanilla `BoatRenderer.submitTypeAdditions`: wooden boats and chest boats submit the
    // `ModelLayers.BOAT_WATER_PATCH` depth-only `RenderTypes.waterMask()` model when not underwater.
    // Bamboo rafts use `RaftRenderer`, which does not override `submitTypeAdditions`.
    if instance.render_state.boat_underwater {
        return None;
    }
    let passes = boat_textured_layer_passes(family, chest);
    let pass = passes.get(1).copied()?;
    Some(no_overlay_layer_submission(
        pass,
        boat_model_root_transform(instance),
    ))
}

fn append_boat_water_mask_mesh(mesh: &mut EntityModelMesh, submit: EntityModelSubmissionEmit) {
    let model = BoatWaterPatchModel::new();
    model.root().render_colored(mesh, submit.transform);
}

pub(in crate::entity_models) fn render_trident_foil_submission(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Trident) || !instance.render_state.trident_foil {
        return;
    }
    let passes = trident_textured_layer_passes();
    let pass = passes[1];
    let mut model = TridentModel::new();
    model.prepare(&instance);
    render_textured_no_overlay_layer_pass(
        meshes,
        &model,
        trident_model_root_transform(instance),
        pass,
        atlas,
    );
}

fn render_scrolled_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    uv_offset: [f32; 2],
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    let (submission, insertion_index) = meshes.record_submission_with_index(submit);
    let Some(entry) = entity_model_texture_atlas_entry(atlas, submit.texture) else {
        return;
    };
    let mut scratch = EntityModelTexturedMesh::new();
    emit(&mut scratch, entry);
    fill_textured_submission_vertices(&mut scratch, 0, submission);
    let mut scroll = EntityModelScrollMesh::new();
    append_scrolled_textured_mesh(&mut scroll, &scratch, entry.uv, uv_offset);
    let owner = scroll_owner_for_render_type(submit.render_type);
    if let Some(sort_key) = meshes.sorted_upload_key(submission, insertion_index) {
        meshes
            .pending_sorted_scroll_uploads
            .push(PendingSortedScrollUpload {
                render_type: submit.render_type,
                owner,
                sort_key,
                mesh: scroll,
            });
        return;
    }
    append_scroll_mesh(scroll_owner_mesh_mut(meshes, owner), scroll);
}

pub(in crate::entity_models) fn render_no_overlay_scrolled_textured_layers<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    atlas: &EntityModelTextureAtlasLayout,
    uv_offset: [f32; 2],
) {
    for pass in passes {
        let submit = no_overlay_layer_submission(pass, transform);
        render_scrolled_textured_submission(meshes, submit, atlas, uv_offset, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
    }
}

fn render_scroll_geometry_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    owner: EntityModelScrollOwner,
    atlas: &EntityModelTextureAtlasLayout,
    emit: impl FnOnce(
        &mut EntityModelScrollMesh,
        EntityModelTextureAtlasEntry,
        EntityModelRenderSubmission,
    ),
) {
    let (submission, insertion_index) = meshes.record_submission_with_index(submit);
    let Some(entry) = entity_model_texture_atlas_entry(atlas, submit.texture) else {
        return;
    };
    if let Some(sort_key) = meshes.sorted_upload_key(submission, insertion_index) {
        let mut mesh = EntityModelScrollMesh::new();
        emit(&mut mesh, entry, submission);
        meshes
            .pending_sorted_scroll_uploads
            .push(PendingSortedScrollUpload {
                render_type: submit.render_type,
                owner,
                sort_key,
                mesh,
            });
        return;
    }
    let target = scroll_owner_mesh_mut(meshes, owner);
    let index_start =
        u32::try_from(target.indices.len()).expect("opaque beam index start fits in u32");
    emit(target, entry, submission);
    if owner.is_opaque_beam() {
        let index_end =
            u32::try_from(target.indices.len()).expect("opaque beam index end fits in u32");
        let index_count = index_end.saturating_sub(index_start);
        if index_count > 0 {
            meshes.opaque_beam_draws.push(EntityModelScrollDrawRange {
                render_type: submit.render_type,
                order: submission.order,
                distance_sq: 0.0,
                insertion_index,
                index_start,
                index_count,
            });
        }
    }
}

fn render_textured_root_pass(
    meshes: &mut EntityModelTexturedMeshes,
    root: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if layer_pass_hidden_by_invisible(meshes, pass) {
        return;
    }
    let submit = textured_layer_submission(meshes, pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        root.render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn no_overlay_submission(
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    tint: [f32; 4],
    transform: Mat4,
    order: i32,
    submit_sequence: u32,
) -> EntityModelSubmissionEmit {
    EntityModelSubmissionEmit::new(
        render_type,
        texture,
        tint,
        transform,
        order,
        submit_sequence,
    )
    .with_overlay(ENTITY_VERTEX_NO_OVERLAY)
}

fn no_overlay_layer_submission(
    pass: EntityModelLayerPass,
    transform: Mat4,
) -> EntityModelSubmissionEmit {
    no_overlay_layer_submission_with_tint(pass, pass.tint, transform)
}

fn no_overlay_layer_submission_with_tint(
    pass: EntityModelLayerPass,
    tint: [f32; 4],
    transform: Mat4,
) -> EntityModelSubmissionEmit {
    no_overlay_submission(
        pass.render_type,
        pass.texture,
        tint,
        transform,
        pass.order,
        pass.submit_sequence,
    )
}

fn textured_layer_submission(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
    transform: Mat4,
) -> EntityModelSubmissionEmit {
    let (render_type, tint) = if layer_pass_uses_outline_render_type(meshes, pass) {
        (
            EntityModelLayerRenderType::Outline,
            outline_layer_tint(pass),
        )
    } else if meshes.current_force_transparent && layer_pass_is_base(pass) {
        let mut tint = pass.tint;
        tint[3] *= 38.0 / 255.0;
        let render_type = if pass.kind == EntityModelLayerKind::ArmorStandBase
            && pass.render_type == EntityModelLayerRenderType::EntityTranslucent
        {
            // Vanilla `ArmorStandRenderer.getRenderType`: marker armor stands use
            // `entityTranslucent(texture, false)` for the force-transparent branch,
            // instead of the generic living `entityTranslucentCullItemTarget`.
            EntityModelLayerRenderType::EntityTranslucent
        } else {
            EntityModelLayerRenderType::EntityTranslucentCullItemTarget
        };
        (render_type, tint)
    } else {
        (pass.render_type, pass.tint)
    };
    let submit = EntityModelSubmissionEmit::new(
        render_type,
        pass.texture,
        tint,
        transform,
        pass.order,
        pass.submit_sequence,
    )
    .with_affects_crumbling(render_type.affects_crumbling());
    if pass.render_type == EntityModelLayerRenderType::EntityCutoutDissolve {
        submit
            .with_overlay(ENTITY_VERTEX_NO_OVERLAY)
            .with_dissolve_texture(ENDER_DRAGON_EXPLODING_TEXTURE_REF)
    } else if layer_pass_uses_no_overlay(pass) {
        submit.with_overlay(ENTITY_VERTEX_NO_OVERLAY)
    } else if layer_pass_uses_zero_white_overlay(pass) {
        submit.with_overlay([0.0, meshes.current_submission_overlay[1]])
    } else {
        submit
    }
}

fn layer_pass_is_base(pass: EntityModelLayerPass) -> bool {
    pass.order == 0 && pass.submit_sequence == 0
}

fn layer_pass_hidden_by_invisible(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
) -> bool {
    if meshes.current_force_transparent {
        return !layer_pass_is_base(pass);
    }
    if meshes.current_outline_only {
        return !layer_pass_is_base(pass) && !layer_pass_uses_invisible_outline(pass);
    }
    false
}

fn layer_pass_uses_outline_render_type(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
) -> bool {
    meshes.current_outline_only
        && (layer_pass_is_base(pass) || layer_pass_uses_invisible_outline(pass))
}

fn layer_pass_uses_invisible_outline(pass: EntityModelLayerPass) -> bool {
    matches!(
        pass.kind,
        EntityModelLayerKind::SheepWool | EntityModelLayerKind::SlimeOuter
    )
}

fn outline_layer_tint(pass: EntityModelLayerPass) -> [f32; 4] {
    match pass.kind {
        // Vanilla `SheepWoolLayer` passes ARGB `0xFF000000` for the invisible-glowing outline model.
        EntityModelLayerKind::SheepWool => [0.0, 0.0, 0.0, 1.0],
        _ => pass.tint,
    }
}

fn layer_pass_uses_no_overlay(pass: EntityModelLayerPass) -> bool {
    matches!(
        pass.kind,
        EntityModelLayerKind::ArrowBase
            | EntityModelLayerKind::BannerPattern
            | EntityModelLayerKind::BoatBase
            | EntityModelLayerKind::BoatWaterMask
            | EntityModelLayerKind::BreezeEyes
            | EntityModelLayerKind::BreezeWind
            | EntityModelLayerKind::CamelSaddle
            | EntityModelLayerKind::CreeperArmor
            | EntityModelLayerKind::CustomHeadSkull
            | EntityModelLayerKind::EndCrystalBase
            | EntityModelLayerKind::EnderDragonEyes
            | EntityModelLayerKind::EndermanEyes
            | EntityModelLayerKind::EquineBodyArmor
            | EntityModelLayerKind::EquineSaddle
            | EntityModelLayerKind::HappyGhastHarness
            | EntityModelLayerKind::EvokerFangsBase
            | EntityModelLayerKind::HumanoidArmor
            | EntityModelLayerKind::LeashKnotBase
            | EntityModelLayerKind::LlamaDecor
            | EntityModelLayerKind::LlamaSpitBase
            | EntityModelLayerKind::MinecartBase
            | EntityModelLayerKind::NautilusBodyArmor
            | EntityModelLayerKind::NautilusSaddle
            | EntityModelLayerKind::PhantomEyes
            | EntityModelLayerKind::PigSaddle
            | EntityModelLayerKind::PlayerCape
            | EntityModelLayerKind::PlayerLeftShoulderParrot
            | EntityModelLayerKind::PlayerRightShoulderParrot
            | EntityModelLayerKind::PlayerSpinAttackEffect
            | EntityModelLayerKind::ShulkerBulletBase
            | EntityModelLayerKind::ShulkerBulletShell
            | EntityModelLayerKind::SpiderEyes
            | EntityModelLayerKind::StriderSaddle
            | EntityModelLayerKind::TridentBase
            | EntityModelLayerKind::TridentFoil
            | EntityModelLayerKind::WindChargeBase
            | EntityModelLayerKind::WitherArmor
            | EntityModelLayerKind::WitherSkullBase
            | EntityModelLayerKind::Wings
            | EntityModelLayerKind::WolfBodyArmor
            | EntityModelLayerKind::WolfBodyArmorCrack
            | EntityModelLayerKind::WolfCollar
    )
}

fn layer_pass_uses_zero_white_overlay(pass: EntityModelLayerPass) -> bool {
    matches!(
        pass.kind,
        EntityModelLayerKind::CopperGolemEyes
            | EntityModelLayerKind::CreakingEyes
            | EntityModelLayerKind::FelineCollar
            | EntityModelLayerKind::HorseMarkings
            | EntityModelLayerKind::IronGolemCrackiness
            | EntityModelLayerKind::DrownedOuter
            | EntityModelLayerKind::PlayerExtraEars
            | EntityModelLayerKind::SkeletonClothing
            | EntityModelLayerKind::SheepWool
            | EntityModelLayerKind::SheepWoolUndercoat
            | EntityModelLayerKind::SlimeOuter
            | EntityModelLayerKind::TropicalFishPattern
            | EntityModelLayerKind::VillagerType
            | EntityModelLayerKind::VillagerProfession
            | EntityModelLayerKind::VillagerLevel
            | EntityModelLayerKind::WardenBioluminescent
            | EntityModelLayerKind::WardenHeart
            | EntityModelLayerKind::WardenPulsatingSpots1
            | EntityModelLayerKind::WardenPulsatingSpots2
            | EntityModelLayerKind::WardenTendrils
            | EntityModelLayerKind::ZombieVillagerType
            | EntityModelLayerKind::ZombieVillagerProfession
            | EntityModelLayerKind::ZombieVillagerLevel
    )
}

/// Render a model's full textured layer-pass list (already prepared) into `meshes`.
pub(in crate::entity_models) fn render_textured_layers<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    for pass in passes {
        // Vanilla `LivingEntityEmissiveLayer` skips layers whose computed alpha is effectively zero.
        if pass.tint[3] <= 1.0e-5
            && pass.render_type != EntityModelLayerRenderType::EntityCutoutDissolve
        {
            continue;
        }
        match pass.visibility {
            // A part-subset emissive overlay (vanilla `retainExactParts`): render only its named parts.
            layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                if layer_pass_hidden_by_invisible(meshes, pass) {
                    continue;
                }
                let submit = textured_layer_submission(meshes, pass, transform);
                render_textured_submission(meshes, submit, atlas, |mesh, entry| {
                    model.root().render_textured_retained(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                        "",
                        parts,
                    );
                });
            }
            // `All` (and the player-parts case, whose subset is pre-applied to the tree) render whole.
            _ => render_textured_root_pass(meshes, model.root(), transform, pass, atlas),
        }
    }
}

/// Renders a prepared model through the shared variable-size entity-texture atlas. The static pass
/// texture remains the explicit fallback when the handle is not present in the current atlas.
pub(in crate::entity_models) fn render_textured_layers_with_dynamic_texture<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    dynamic_texture: EntityDynamicTexture,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    for pass in passes {
        if pass.tint[3] <= 1.0e-5
            && pass.render_type != EntityModelLayerRenderType::EntityCutoutDissolve
        {
            continue;
        }
        if layer_pass_hidden_by_invisible(meshes, pass) {
            continue;
        }
        let submit = textured_layer_submission(meshes, pass, transform)
            .with_dynamic_texture(dynamic_texture);
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, dynamic_texture) {
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| match pass.visibility {
                    layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                        model.root().render_textured_retained(
                            mesh,
                            submit.transform,
                            submit.texture,
                            entry.uv,
                            submit.tint,
                            "",
                            parts,
                        );
                    }
                    _ => model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    ),
                },
            );
            continue;
        }
        render_textured_submission(meshes, submit, atlas, |mesh, entry| match pass.visibility {
            layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                model.root().render_textured_retained(
                    mesh,
                    submit.transform,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                    "",
                    parts,
                );
            }
            _ => model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            ),
        });
    }
}

/// The breeze's swirling wind body (vanilla `BreezeWindLayer`): the SEPARATE [`BreezeWindModel`] (the
/// `wind_body` shell chain on the 128×128 `breeze_wind.png`) rendered with the `breezeWind` render
/// type, whose texture matrix scrolls the U coordinate by `xOffset(ageInTicks) % 1 = (ageInTicks ·
/// 0.02) % 1` (V fixed at `0`). Like the wind charge, we render the wind model once with the normal
/// atlas UVs into a scratch mesh — its `setup_anim` applies the same idle sway + action swirls/pulses
/// as the base body so the two layers move together — then fold it into the translucent scrolling
/// overlay mesh, baking the per-instance U offset and carrying the atlas sub-rect for the shader wrap.
pub(in crate::entity_models) fn render_breeze_wind_scroll_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Breeze) {
        return;
    }
    let transform = entity_model_root_transform(instance);
    let passes = breeze_textured_layer_passes();
    let pass = passes[1];
    let submit = no_overlay_layer_submission(pass, transform);
    let mut model = BreezeWindModel::new();
    model.prepare(&instance);
    // Vanilla `BreezeWindLayer.xOffset(t) = t · 0.02`, taken `% 1.0`; `ageInTicks ≥ 0` so the Java
    // float modulo is `rem_euclid`. V does not scroll.
    let u_offset = (instance.render_state.age_in_ticks * 0.02).rem_euclid(1.0);
    render_scrolled_textured_submission(meshes, submit, atlas, [u_offset, 0.0], |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

/// The charged creeper's `CreeperPowerLayer` energy swirl (vanilla `EnergySwirlLayer`): when the
/// synced `isPowered` is set, the inflated `CREEPER_ARMOR` model (`CubeDeformation 2.0`, driven by the
/// same `setup_anim` so it tracks the body pose) is drawn with the additive, emissive `energySwirl`
/// render type — `creeper_armor.png` scrolling on both axes by `xOffset(ageInTicks) % 1 =
/// (ageInTicks · 0.01) % 1`, tinted by the vanilla `0xFF808080` half-grey. Folded into the additive
/// scroll mesh the same way the wind charge folds into the translucent one.
pub(in crate::entity_models) fn render_charged_creeper_energy_swirl(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if !instance.render_state.creeper_powered || !matches!(instance.kind, EntityModelKind::Creeper)
    {
        return;
    }
    let transform = creeper_model_root_transform(instance);
    let passes = creeper_textured_layer_passes();
    let pass = passes[1];
    let submit = no_overlay_layer_submission(pass, transform);
    let mut model = CreeperModel::new_armor();
    model.prepare(&instance);
    // Vanilla creeper `xOffset(t) = t · 0.01`, taken `% 1.0` on both U and V.
    let offset = (instance.render_state.age_in_ticks * 0.01).rem_euclid(1.0);
    // Vanilla `EnergySwirlLayer` tints by `0xFF808080` (half grey) under additive blend.
    render_scrolled_textured_submission(meshes, submit, atlas, [offset, offset], |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

/// The wither boss's `WitherArmorLayer` energy swirl (vanilla `EnergySwirlLayer`, the same family as
/// the charged creeper): when `isPowered` (the wither sits at or below half health), the inflated
/// `WITHER_ARMOR` model (`INNER_ARMOR_DEFORMATION` = `CubeDeformation 0.5`, driven by the same
/// `setup_anim` so it breathes with the body) is drawn with the additive, emissive `energySwirl`
/// render type — `wither_armor.png` tinted by the vanilla `0xFF808080` half-grey. Unlike the creeper's
/// linear scroll, the wither's `xOffset(t) = cos(t · 0.02) · 3` oscillates the U coordinate while V
/// scrolls linearly at `t · 0.01`; both are taken `% 1.0`. Folded into the same additive scroll mesh.
pub(in crate::entity_models) fn render_wither_energy_swirl(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if !instance.render_state.wither_powered || !matches!(instance.kind, EntityModelKind::Wither) {
        return;
    }
    let transform = wither_model_root_transform(instance);
    let passes = wither_textured_layer_passes(instance.render_state.wither_invulnerable_ticks);
    let pass = passes[1];
    let submit = no_overlay_layer_submission(pass, transform);
    let mut model = WitherModel::new_armor();
    model.prepare(&instance);
    // Vanilla `WitherArmorLayer.xOffset(t) = cos(t · 0.02) · 3` on U (oscillating, not linear like the
    // creeper), `t · 0.01` on V, each taken `% 1.0`. Java float modulo of a possibly-negative U keeps
    // the sign, then the shader's `fract` re-wraps it into `[0, 1)`, so plain `% 1.0` (`Rust` `rem`,
    // not `rem_euclid`) reproduces the vanilla offset exactly.
    let age = instance.render_state.age_in_ticks;
    let u_offset = ((age * 0.02).cos() * 3.0) % 1.0;
    let v_offset = (age * 0.01).rem_euclid(1.0);
    // Vanilla `EnergySwirlLayer` tints by `0xFF808080` (half grey) under additive blend.
    render_scrolled_textured_submission(
        meshes,
        submit,
        atlas,
        [u_offset, v_offset],
        |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        },
    );
}

/// The guardian attack beam (vanilla `GuardianRenderer.renderBeam`). When the guardian has an active
/// attack target, a world-space twisted prism is drawn from the guardian eye toward the target along
/// the world `beamVector` (`eye_to_target`): two crossed longitudinal strips (the inner `0.2`-radius
/// rays) plus a twisting `0.282`-radius top cap, the whole thing spun by `rot = attackTime · 0.05 ·
/// -1.5` and tinted by the attack-scale color ramp (`colorScale = scale²`). The `guardian_beam.png`
/// texture tiles vertically (V spans `length · 2.5` units, scrolled by `texVOff`) via the scroll
/// (fract-wrap) pass. Built in a world-aligned frame (`translate(pos) · translate(0, eyeHeight, 0) ·
/// rotY(yRot) · rotX(xRot)`, no body yaw / model flip), mirroring vanilla where the beam draws after
/// `super.submit` has popped the model's `setupRotations` back to the entity-origin frame.
pub(in crate::entity_models) fn render_guardian_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Guardian { .. }) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let Some(beam) = instance.render_state.guardian_beam else {
        return;
    };

    // Orient local +Y onto the world beam direction, then lift the origin from the entity feet to the
    // eye. Vanilla: `xRot = acos(dir.y)`, `yRot = π/2 − atan2(dir.z, dir.x)`.
    let beam_vector = Vec3::from_array(beam.eye_to_target);
    let length = beam_vector.length() + 1.0;
    let dir = beam_vector.normalize_or_zero();
    let x_rot = dir.y.clamp(-1.0, 1.0).acos();
    let y_rot = std::f32::consts::FRAC_PI_2 - dir.z.atan2(dir.x);
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::new(0.0, beam.eye_height, 0.0))
        * Mat4::from_rotation_y(y_rot)
        * Mat4::from_rotation_x(x_rot);

    // The prism cross-section: four inner rays at radius 0.2 and four outer cap rays at 0.282, each
    // offset around the beam axis by a fixed angle plus the time spin `rot`.
    use std::f32::consts::PI;
    let rot = beam.attack_time * 0.05 * -1.5;
    let ring = |angle: f32, radius: f32| {
        let a = rot + angle;
        (a.cos() * radius, a.sin() * radius)
    };
    let (wnx, wnz) = ring(PI * 3.0 / 4.0, 0.282);
    let (enx, enz) = ring(PI / 4.0, 0.282);
    let (wsx, wsz) = ring(PI * 5.0 / 4.0, 0.282);
    let (esx, esz) = ring(PI * 7.0 / 4.0, 0.282);
    let (wx, wz) = ring(PI, 0.2);
    let (ex, ez) = ring(0.0, 0.2);
    let (nx, nz) = ring(PI / 2.0, 0.2);
    let (sx, sz) = ring(PI * 3.0 / 2.0, 0.2);

    // Vanilla color ramp from the attack scale, truncated to ints exactly as the `(int)` casts do.
    let color_scale = beam.attack_scale * beam.attack_scale;
    let tint = [
        (64 + (color_scale * 191.0) as i32) as f32 / 255.0,
        (32 + (color_scale * 191.0) as i32) as f32 / 255.0,
        (128 - (color_scale * 64.0) as i32) as f32 / 255.0,
        1.0,
    ];
    // `GuardianRenderer.submit` calls `super.submit` first, then submits custom geometry with
    // `BEAM_RENDER_TYPE = RenderTypes.entityCutout(guardian_beam.png)` on the same collector. Consume
    // the explicit beam pass metadata before folding the tiled V coordinates into the scroll mesh.
    let elder = matches!(instance.kind, EntityModelKind::Guardian { elder: true });
    let passes = guardian_textured_layer_passes(elder);
    let beam_pass = passes[1];
    let submit = no_overlay_layer_submission_with_tint(beam_pass, tint, transform)
        .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);
    render_scroll_geometry_submission(
        meshes,
        submit,
        EntityModelScrollOwner::GuardianBeam,
        atlas,
        |mesh, entry, submit| {
            let top = length;
            let tex_v_off = (beam.attack_time * 0.5).rem_euclid(1.0);
            let min_v = -1.0 + tex_v_off;
            let max_v = min_v + length * 2.5;
            let v_base = if (beam.attack_time.floor() as i32).rem_euclid(2) == 0 {
                0.5
            } else {
                0.0
            };

            // 12 vertices in three quads (W↔E strip, N↔S strip, twisting top cap), local UVs in
            // `0..1` for U and tiling for V — matching `GuardianRenderer.vertex` exactly.
            let vertices: [(f32, f32, f32, f32, f32); 12] = [
                (wx, top, wz, 0.4999, max_v),
                (wx, 0.0, wz, 0.4999, min_v),
                (ex, 0.0, ez, 0.0, min_v),
                (ex, top, ez, 0.0, max_v),
                (nx, top, nz, 0.4999, max_v),
                (nx, 0.0, nz, 0.4999, min_v),
                (sx, 0.0, sz, 0.0, min_v),
                (sx, top, sz, 0.0, max_v),
                (wnx, top, wnz, 0.5, v_base + 0.5),
                (enx, top, enz, 1.0, v_base + 0.5),
                (esx, top, esz, 1.0, v_base),
                (wsx, top, wsz, 0.5, v_base),
            ];
            let rect = entry.uv;
            let size = [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
            let base = u32::try_from(mesh.vertices.len()).expect("scroll vertex count fits in u32");
            let normal = submit
                .transform
                .transform_vector3(Vec3::Y)
                .normalize_or_zero()
                .to_array();
            for (x, y, z, u, v) in vertices {
                let world = submit.transform.transform_point3(Vec3::new(x, y, z));
                mesh.vertices.push(EntityModelScrollVertex {
                    position: world.to_array(),
                    local_uv: [u, v],
                    uv_rect_min: rect.min,
                    uv_rect_size: size,
                    tint: submit.tint,
                    light: submit.light,
                    overlay: submit.overlay,
                    normal,
                });
            }
            // Each quad → two triangles (the scroll pipeline renders cull-off, so winding is
            // immaterial).
            for quad in 0..3u32 {
                let o = base + quad * 4;
                mesh.indices
                    .extend_from_slice(&[o, o + 1, o + 2, o, o + 2, o + 3]);
            }
        },
    );
}

/// Vanilla `EnderDragonRenderer.submitCrystalBeams`, as called by `EndCrystalRenderer.submit` when
/// `EndCrystalRenderState.beamOffset` is present. The crystal renderer first translates by the beam
/// target offset, then the shared helper translates up by two units, rotates local +Z toward the
/// crystal→beam-target delta (including the crystal bob), and submits eight prism quads with black
/// inner vertices, white outer vertices, and a vertically tiled `end_crystal_beam.png` texture.
pub(in crate::entity_models) fn render_end_crystal_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::EndCrystal) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let Some(beam) = instance.render_state.end_crystal_beam else {
        return;
    };

    let beam_offset = Vec3::from_array(beam.beam_offset);
    let age = instance.render_state.age_in_ticks;
    let delta = Vec3::new(
        -beam_offset.x,
        -beam_offset.y + end_crystal_get_y(age),
        -beam_offset.z,
    );
    let horizontal_length = (delta.x * delta.x + delta.z * delta.z).sqrt();
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(beam_offset)
        * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        * Mat4::from_rotation_y(-delta.z.atan2(delta.x) - std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(-horizontal_length.atan2(delta.y) - std::f32::consts::FRAC_PI_2);
    let passes = end_crystal_textured_layer_passes();
    let beam_pass = passes[1];
    let submit = no_overlay_layer_submission(beam_pass, transform);
    emit_crystal_beam_submission(meshes, submit, atlas, delta.length(), age);
}

/// Vanilla `EnderDragonRenderer.submit`: after body and eyes submits, a dragon with
/// `EnderDragonRenderState.beamOffset` calls the same `submitCrystalBeams` helper from the dragon's
/// entity-origin pose. Unlike an end crystal, the dragon does not pre-translate by the offset and does
/// not invert the delta; its `beamOffset` already points from the dragon to the bobbed crystal.
pub(in crate::entity_models) fn render_ender_dragon_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::EnderDragon) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let Some(beam) = instance.render_state.ender_dragon_beam else {
        return;
    };

    let delta = Vec3::from_array(beam.beam_offset);
    let horizontal_length = (delta.x * delta.x + delta.z * delta.z).sqrt();
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        * Mat4::from_rotation_y(-delta.z.atan2(delta.x) - std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(-horizontal_length.atan2(delta.y) - std::f32::consts::FRAC_PI_2);
    let passes = ender_dragon_textured_layer_passes(instance.render_state.ender_dragon_death_time);
    let beam_pass = passes[2];
    let submit = no_overlay_layer_submission(beam_pass, transform);
    emit_crystal_beam_submission(
        meshes,
        submit,
        atlas,
        delta.length(),
        instance.render_state.age_in_ticks,
    );
}

/// Vanilla `EnderDragonRenderer.submit`: immediately after body and eyes, a dying dragon pushes the
/// current dragon pose by `(0, -1, -2)` and submits the same seeded custom ray geometry twice:
/// `RenderTypes.dragonRays()` for additive colour and `RenderTypes.dragonRaysDepth()` for the
/// depth-only replay. The geometry is no-texture position-colour custom geometry, so it records a
/// separate custom submission instead of pretending to be texture-backed.
pub(in crate::entity_models) fn render_ender_dragon_death_rays(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    if !matches!(instance.kind, EntityModelKind::EnderDragon) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let death_time = instance.render_state.ender_dragon_death_time;
    if death_time <= 0.0 {
        return;
    }

    let transform = ender_dragon_model_root_transform(instance)
        * Mat4::from_translation(Vec3::new(0.0, -1.0, -2.0));
    let normalized_death_time = death_time / 200.0;
    emit_dragon_rays_custom_submission(
        meshes,
        EntityModelLayerRenderType::DragonRays,
        transform,
        normalized_death_time,
        2,
    );
    emit_dragon_rays_custom_submission(
        meshes,
        EntityModelLayerRenderType::DragonRaysDepth,
        transform,
        normalized_death_time,
        3,
    );
}

/// Vanilla `LeashFeatureRenderer.LEASH_RENDER_STEPS`.
const LEASH_RENDER_STEPS: u32 = 24;
/// Vanilla `LeashFeatureRenderer.LEASH_WIDTH`.
const LEASH_WIDTH: f32 = 0.05;
/// Vanilla `addVertexPair`'s constant rope colour before the alternating 0.7 modifier.
const LEASH_COLOR: [f32; 3] = [0.5, 0.4, 0.3];

/// Vanilla `EntityRenderer.submit` submits every extracted `LeashState` through
/// `SubmitNodeCollector.submitLeash` before any model or layer work, with no invisibility or glow
/// gate, so the rope emits for every instance that carries leash states.
pub(in crate::entity_models) fn render_leash_ropes(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    let Some(leash_ropes) = instance.render_state.leash_ropes else {
        return;
    };
    for rope in leash_ropes.ropes() {
        emit_leash_rope_custom_submission(meshes, instance.position, *rope);
    }
}

fn emit_leash_rope_custom_submission(
    meshes: &mut EntityModelTexturedMeshes,
    entity_position: [f32; 3],
    rope: LeashRopeRenderState,
) {
    let render_type = EntityModelLayerRenderType::Leash;
    // Vanilla translates the entity render pose by `leashState.offset`, so the rope's local origin
    // is the world-space rope start.
    let transform =
        Mat4::from_translation(Vec3::from_array(entity_position) + Vec3::from_array(rope.offset));
    let (submission, insertion_index) =
        meshes.record_custom_submission_with_index(EntityModelCustomGeometrySubmission {
            render_type,
            transform,
            order: 0,
            submit_sequence: 0,
        });
    let mut mesh = EntityModelMesh::new();
    append_leash_rope_mesh(&mut mesh, submission.transform, rope);
    let index_count = u32::try_from(mesh.indices.len()).expect("leash index count fits in u32");
    let index_start = append_entity_model_mesh(&mut meshes.leash, mesh);
    if let Some(sort_key) = meshes.custom_geometry_draw_key(submission, insertion_index) {
        meshes.push_position_color_draw_range(render_type, sort_key, index_start, index_count);
    }
}

/// Vanilla `LeashFeatureRenderer.renderLeash`: two 25-pair passes over the same vertex stream (the
/// forward pass fudges the first vertex of each pair up by the rope width, the backward pass the
/// second), submitted as one `TRIANGLE_STRIP`. The shared mesh is an indexed triangle list, so strip
/// triangle `i` unrolls to vertices `(i, i+1, i+2)`; the LEASH pipeline never culls, which makes the
/// strip's alternating winding irrelevant.
fn append_leash_rope_mesh(mesh: &mut EntityModelMesh, transform: Mat4, rope: LeashRopeRenderState) {
    let [dx, _, dz] = rope.delta;
    let horizontal_sq = dx * dx + dz * dz;
    // Vanilla `Mth.invSqrt(0)` would poison a perfectly vertical rope with NaN cross-section
    // offsets; collapse the cross-section instead so the rope degenerates to a line.
    let offset_factor = if horizontal_sq > 0.0 {
        horizontal_sq.sqrt().recip() * LEASH_WIDTH / 2.0
    } else {
        0.0
    };
    let dx_off = dz * offset_factor;
    let dz_off = dx * offset_factor;
    let base_vertex = u32::try_from(mesh.vertices.len()).expect("leash vertex count fits in u32");
    for step in 0..=LEASH_RENDER_STEPS {
        append_leash_vertex_pair(
            mesh,
            transform,
            rope,
            LEASH_WIDTH,
            dx_off,
            dz_off,
            step,
            false,
        );
    }
    for step in (0..=LEASH_RENDER_STEPS).rev() {
        append_leash_vertex_pair(mesh, transform, rope, 0.0, dx_off, dz_off, step, true);
    }
    let vertex_count =
        u32::try_from(mesh.vertices.len()).expect("leash vertex count fits in u32") - base_vertex;
    for index in 0..vertex_count.saturating_sub(2) {
        mesh.indices.extend_from_slice(&[
            base_vertex + index,
            base_vertex + index + 1,
            base_vertex + index + 2,
        ]);
    }
}

/// Vanilla `LeashFeatureRenderer.addVertexPair` for one strip step.
#[allow(clippy::too_many_arguments)]
fn append_leash_vertex_pair(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    rope: LeashRopeRenderState,
    fudge: f32,
    dx_off: f32,
    dz_off: f32,
    step: u32,
    backwards: bool,
) {
    let [dx, dy, dz] = rope.delta;
    let progress = step as f32 / LEASH_RENDER_STEPS as f32;
    let block = leash_lerped_light(progress, rope.start_block_light, rope.end_block_light);
    let sky = leash_lerped_light(progress, rope.start_sky_light, rope.end_sky_light);
    let light = [block / 15.0, sky / 15.0];
    let color_modifier = if step % 2 == u32::from(backwards) {
        0.7
    } else {
        1.0
    };
    let color = [
        LEASH_COLOR[0] * color_modifier,
        LEASH_COLOR[1] * color_modifier,
        LEASH_COLOR[2] * color_modifier,
        1.0,
    ];
    let x = dx * progress;
    let y = if rope.slack {
        // The sag curve leans toward the lower endpoint regardless of which side is higher.
        if dy > 0.0 {
            dy * progress * progress
        } else {
            dy - dy * (1.0 - progress) * (1.0 - progress)
        }
    } else {
        dy * progress
    };
    let z = dz * progress;
    for position in [
        [x - dx_off, y + fudge, z + dz_off],
        [x + dx_off, y + LEASH_WIDTH - fudge, z - dz_off],
    ] {
        let world = transform.transform_point3(Vec3::from_array(position));
        mesh.vertices.push(EntityModelVertex {
            position: world.to_array(),
            color,
            light,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal: [0.0, 1.0, 0.0],
        });
    }
}

/// Vanilla `(int)Mth.lerp(progress, start, end)`: the lerped light truncates to an integer level
/// before packing into the light coords.
fn leash_lerped_light(progress: f32, start: u8, end: u8) -> f32 {
    (f32::from(start) + progress * (f32::from(end) - f32::from(start))).trunc()
}

pub(in crate::entity_models) fn render_end_portal_block_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let EntityModelKind::EndPortalBlock { kind, faces } = instance.kind else {
        return;
    };
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let render_type = match kind {
        EndPortalModelKind::EndPortal => EntityModelLayerRenderType::EndPortal,
        EndPortalModelKind::EndGateway => EntityModelLayerRenderType::EndGateway,
    };
    let cube_transform = end_portal_block_root_transform(instance, kind);
    let (submission, insertion_index) =
        meshes.record_portal_submission_with_index(render_type, cube_transform);
    if let (Some(sky), Some(portal)) = (
        entity_model_texture_atlas_entry(atlas, submission.sky_texture),
        entity_model_texture_atlas_entry(atlas, submission.portal_texture),
    ) {
        emit_end_portal_submission(
            meshes,
            submission,
            insertion_index,
            faces,
            sky.uv,
            portal.uv,
        );
    }
    if kind == EndPortalModelKind::EndGateway {
        render_end_gateway_beam(meshes, instance, atlas);
    }
}

fn end_portal_block_root_transform(
    instance: EntityModelInstance,
    kind: EndPortalModelKind,
) -> Mat4 {
    let transform = Mat4::from_translation(Vec3::from_array(instance.position));
    match kind {
        EndPortalModelKind::EndPortal => {
            transform
                * Mat4::from_translation(Vec3::new(0.0, END_PORTAL_BOTTOM, 0.0))
                * Mat4::from_scale(Vec3::new(1.0, END_PORTAL_HEIGHT, 1.0))
        }
        EndPortalModelKind::EndGateway => transform,
    }
}

fn emit_end_portal_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submission: EntityModelPortalSubmission,
    insertion_index: usize,
    faces: [EndPortalModelFace; 2],
    sky_uv: EntityModelUvRect,
    portal_uv: EntityModelUvRect,
) {
    let mut mesh = EntityModelPortalMesh::new();
    append_end_portal_cube_mesh(&mut mesh, submission.transform, faces, sky_uv, portal_uv);
    let index_count =
        u32::try_from(mesh.indices.len()).expect("end portal cube index count fits in u32");
    let index_start = match submission.render_type {
        EntityModelLayerRenderType::EndPortal => append_portal_mesh(&mut meshes.end_portal, mesh),
        EntityModelLayerRenderType::EndGateway => append_portal_mesh(&mut meshes.end_gateway, mesh),
        _ => unreachable!("end portal submission render type"),
    };
    if let Some(sort_key) = meshes.portal_draw_key(submission, insertion_index) {
        meshes.push_portal_draw_range(submission.render_type, sort_key, index_start, index_count);
    }
}

fn append_end_portal_cube_mesh(
    mesh: &mut EntityModelPortalMesh,
    transform: Mat4,
    faces: [EndPortalModelFace; 2],
    sky_uv: EntityModelUvRect,
    portal_uv: EntityModelUvRect,
) {
    for face in faces {
        match face {
            EndPortalModelFace::Down => append_end_portal_face(
                mesh,
                transform,
                [
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                ],
                sky_uv,
                portal_uv,
            ),
            EndPortalModelFace::Up => append_end_portal_face(
                mesh,
                transform,
                [
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 0.0),
                ],
                sky_uv,
                portal_uv,
            ),
        }
    }
}

fn append_end_portal_face(
    mesh: &mut EntityModelPortalMesh,
    transform: Mat4,
    corners: [Vec3; 4],
    sky_uv: EntityModelUvRect,
    portal_uv: EntityModelUvRect,
) {
    let base = u32::try_from(mesh.vertices.len()).expect("end portal vertex count fits in u32");
    let sky_rect_size = [sky_uv.max[0] - sky_uv.min[0], sky_uv.max[1] - sky_uv.min[1]];
    let portal_rect_size = [
        portal_uv.max[0] - portal_uv.min[0],
        portal_uv.max[1] - portal_uv.min[1],
    ];
    for corner in corners {
        mesh.vertices.push(EntityModelPortalVertex {
            position: transform.transform_point3(corner).to_array(),
            sky_rect_min: sky_uv.min,
            sky_rect_size,
            portal_rect_min: portal_uv.min,
            portal_rect_size,
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn render_end_gateway_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(beam) = instance.render_state.end_gateway_beam else {
        return;
    };
    if beam.height <= 0 {
        return;
    }
    let tint = argb_to_tint(beam.color_argb);
    let transform = Mat4::from_translation(Vec3::from_array(instance.position));
    let [opaque_pass, translucent_pass] = end_gateway_beam_layer_passes();
    let opaque_submit = no_overlay_layer_submission_with_tint(opaque_pass, tint, transform)
        .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);
    let translucent_submit =
        no_overlay_layer_submission_with_tint(translucent_pass, tint, transform)
            .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);
    emit_end_gateway_beam_submission(meshes, opaque_submit, translucent_submit, atlas, beam);
}

fn emit_end_gateway_beam_submission(
    meshes: &mut EntityModelTexturedMeshes,
    opaque_submit: EntityModelSubmissionEmit,
    translucent_submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    beam: EndGatewayBeamRenderState,
) {
    let beam_start = -beam.height;
    let height = beam.height * 2;
    let beam_end = beam_start + height;
    let transform = opaque_submit.transform * Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5));
    let scroll = if height < 0 {
        beam.animation_time
    } else {
        -beam.animation_time
    };
    let tex_v_off = frac(scroll * 0.2 - (scroll * 0.1).floor());
    let vv2 = -1.0 + tex_v_off;

    let solid_radius = 0.15;
    let solid_vv1 = height as f32 * beam.scale * (0.5 / solid_radius) + vv2;
    let inner_transform =
        transform * Mat4::from_rotation_y((beam.animation_time * 2.25 - 45.0).to_radians());
    render_scroll_geometry_submission(
        meshes,
        opaque_submit,
        EntityModelScrollOwner::EndGatewayBeamOpaque,
        atlas,
        |mesh, entry, submit| {
            append_beacon_beam_part(
                mesh,
                entry,
                inner_transform,
                beam_start,
                beam_end,
                [
                    (0.0, solid_radius),
                    (solid_radius, 0.0),
                    (-solid_radius, 0.0),
                    (0.0, -solid_radius),
                ],
                submit.tint,
                solid_vv1,
                vv2,
            );
        },
    );

    let glow_radius = 0.175;
    let glow_vv1 = height as f32 * beam.scale + vv2;
    render_scroll_geometry_submission(
        meshes,
        translucent_submit,
        EntityModelScrollOwner::EndGatewayBeamTranslucent,
        atlas,
        |mesh, entry, submit| {
            let mut glow_tint = submit.tint;
            glow_tint[3] = 32.0 / 255.0;
            append_beacon_beam_part(
                mesh,
                entry,
                transform,
                beam_start,
                beam_end,
                [
                    (-glow_radius, -glow_radius),
                    (glow_radius, -glow_radius),
                    (-glow_radius, glow_radius),
                    (glow_radius, glow_radius),
                ],
                glow_tint,
                glow_vv1,
                vv2,
            );
        },
    );
}

fn append_beacon_beam_part(
    mesh: &mut EntityModelScrollMesh,
    entry: EntityModelTextureAtlasEntry,
    transform: Mat4,
    beam_start: i32,
    beam_end: i32,
    corners: [(f32, f32); 4],
    tint: [f32; 4],
    vv1: f32,
    vv2: f32,
) {
    let [(wnx, wnz), (enx, enz), (wsx, wsz), (esx, esz)] = corners;
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (wnx, wnz),
        (enx, enz),
        tint,
        vv1,
        vv2,
    );
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (esx, esz),
        (wsx, wsz),
        tint,
        vv1,
        vv2,
    );
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (enx, enz),
        (esx, esz),
        tint,
        vv1,
        vv2,
    );
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (wsx, wsz),
        (wnx, wnz),
        tint,
        vv1,
        vv2,
    );
}

fn append_beacon_beam_quad(
    mesh: &mut EntityModelScrollMesh,
    entry: EntityModelTextureAtlasEntry,
    transform: Mat4,
    beam_start: i32,
    beam_end: i32,
    first: (f32, f32),
    second: (f32, f32),
    tint: [f32; 4],
    vv1: f32,
    vv2: f32,
) {
    let base =
        u32::try_from(mesh.vertices.len()).expect("end gateway beam vertex count fits in u32");
    for (position, local_uv) in [
        (Vec3::new(first.0, beam_end as f32, first.1), [1.0, vv1]),
        (Vec3::new(first.0, beam_start as f32, first.1), [1.0, vv2]),
        (Vec3::new(second.0, beam_start as f32, second.1), [0.0, vv2]),
        (Vec3::new(second.0, beam_end as f32, second.1), [0.0, vv1]),
    ] {
        append_beacon_beam_vertex(mesh, entry, transform, position, local_uv, tint);
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn append_beacon_beam_vertex(
    mesh: &mut EntityModelScrollMesh,
    entry: EntityModelTextureAtlasEntry,
    transform: Mat4,
    position: Vec3,
    local_uv: [f32; 2],
    tint: [f32; 4],
) {
    let rect = entry.uv;
    let size = [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
    mesh.vertices.push(EntityModelScrollVertex {
        position: transform.transform_point3(position).to_array(),
        local_uv,
        uv_rect_min: rect.min,
        uv_rect_size: size,
        tint,
        light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
        overlay: ENTITY_VERTEX_NO_OVERLAY,
        normal: transform
            .transform_vector3(Vec3::Y)
            .normalize_or_zero()
            .to_array(),
    });
}

fn frac(value: f32) -> f32 {
    value - value.floor()
}

fn emit_dragon_rays_custom_submission(
    meshes: &mut EntityModelTexturedMeshes,
    render_type: EntityModelLayerRenderType,
    transform: Mat4,
    death_time: f32,
    submit_sequence: u32,
) {
    let (submission, insertion_index) =
        meshes.record_custom_submission_with_index(EntityModelCustomGeometrySubmission {
            render_type,
            transform,
            order: 0,
            submit_sequence,
        });
    let mut mesh = EntityModelMesh::new();
    append_dragon_rays_mesh(&mut mesh, submission.transform, death_time);
    let index_count =
        u32::try_from(mesh.indices.len()).expect("dragon rays index count fits in u32");
    let index_start = match render_type {
        EntityModelLayerRenderType::DragonRays => {
            append_entity_model_mesh(&mut meshes.dragon_rays, mesh)
        }
        EntityModelLayerRenderType::DragonRaysDepth => {
            append_entity_model_mesh(&mut meshes.dragon_rays_depth, mesh)
        }
        _ => unreachable!("dragon rays custom submission render type"),
    };
    if let Some(sort_key) = meshes.custom_geometry_draw_key(submission, insertion_index) {
        meshes.push_position_color_draw_range(render_type, sort_key, index_start, index_count);
    }
}

fn append_dragon_rays_mesh(mesh: &mut EntityModelMesh, transform: Mat4, death_time: f32) {
    let over_drive = if death_time > 0.8 {
        ((death_time - 0.8) / 0.2).min(1.0)
    } else {
        0.0
    };
    let inner_alpha = 1.0 - over_drive;
    let inner_color = [1.0, 1.0, 1.0, inner_alpha];
    let outer_color = [1.0, 0.0, 1.0, 1.0];
    let ray_count = ((death_time + death_time * death_time) / 2.0 * 60.0).floor() as usize;
    let mut random = DragonRaysRandom::new(DRAGON_RAYS_RANDOM_SEED);
    let mut pose = transform;

    for _ in 0..ray_count {
        let first = Quat::from_euler(
            glam::EulerRot::XYZ,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU,
        );
        let second = Quat::from_euler(
            glam::EulerRot::XYZ,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU + death_time * std::f32::consts::FRAC_PI_2,
        );
        pose = pose * Mat4::from_quat(first * second);
        let length = random.next_float() * 20.0 + 5.0 + over_drive * 10.0;
        let width = random.next_float() * 2.0 + 1.0 + over_drive * 2.0;
        let origin = Vec3::ZERO;
        let outer_left = Vec3::new(-DRAGON_RAYS_HALF_SQRT_3 * width, length, -0.5 * width);
        let outer_right = Vec3::new(DRAGON_RAYS_HALF_SQRT_3 * width, length, -0.5 * width);
        let outer_bottom = Vec3::new(0.0, length, width);
        append_dragon_ray_triangle(
            mesh,
            pose,
            origin,
            outer_left,
            outer_right,
            inner_color,
            outer_color,
        );
        append_dragon_ray_triangle(
            mesh,
            pose,
            origin,
            outer_right,
            outer_bottom,
            inner_color,
            outer_color,
        );
        append_dragon_ray_triangle(
            mesh,
            pose,
            origin,
            outer_bottom,
            outer_left,
            inner_color,
            outer_color,
        );
    }
}

fn append_dragon_ray_triangle(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    origin: Vec3,
    left: Vec3,
    right: Vec3,
    inner_color: [f32; 4],
    outer_color: [f32; 4],
) {
    let base = u32::try_from(mesh.vertices.len()).expect("dragon rays vertex count fits in u32");
    for (position, color) in [
        (origin, inner_color),
        (left, outer_color),
        (right, outer_color),
    ] {
        mesh.vertices.push(EntityModelVertex {
            position: transform.transform_point3(position).to_array(),
            color,
            light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal: [0.0, 1.0, 0.0],
        });
    }
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
}

#[derive(Clone, Copy)]
struct DragonRaysRandom {
    seed: u64,
}

impl DragonRaysRandom {
    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ DRAGON_RAYS_RANDOM_MULTIPLIER) & DRAGON_RAYS_RANDOM_MASK,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(DRAGON_RAYS_RANDOM_MULTIPLIER)
            .wrapping_add(DRAGON_RAYS_RANDOM_INCREMENT)
            & DRAGON_RAYS_RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.960_464_5e-8
    }
}

fn emit_crystal_beam_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    length: f32,
    age: f32,
) {
    render_scroll_geometry_submission(
        meshes,
        submit,
        EntityModelScrollOwner::EndCrystalBeam,
        atlas,
        |mesh, entry, submit| {
            let rect = entry.uv;
            let size = [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
            let v0 = -age * 0.01;
            let v1 = length / 32.0 - age * 0.01;
            let mut last_sin = 0.0;
            let mut last_cos = 0.75;
            let mut last_u = 0.0;

            for i in 1..=8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let sin = angle.sin() * 0.75;
                let cos = angle.cos() * 0.75;
                let u = i as f32 / 8.0;
                let base =
                    u32::try_from(mesh.vertices.len()).expect("scroll vertex count fits in u32");
                for (position, local_uv, tint) in [
                    (
                        Vec3::new(last_sin * 0.2, last_cos * 0.2, 0.0),
                        [last_u, v0],
                        [0.0, 0.0, 0.0, 1.0],
                    ),
                    (
                        Vec3::new(last_sin, last_cos, length),
                        [last_u, v1],
                        [1.0, 1.0, 1.0, 1.0],
                    ),
                    (Vec3::new(sin, cos, length), [u, v1], [1.0, 1.0, 1.0, 1.0]),
                    (
                        Vec3::new(sin * 0.2, cos * 0.2, 0.0),
                        [u, v0],
                        [0.0, 0.0, 0.0, 1.0],
                    ),
                ] {
                    let normal = submit
                        .transform
                        .transform_vector3(Vec3::NEG_Y)
                        .normalize_or_zero()
                        .to_array();
                    mesh.vertices.push(EntityModelScrollVertex {
                        position: submit.transform.transform_point3(position).to_array(),
                        local_uv,
                        uv_rect_min: rect.min,
                        uv_rect_size: size,
                        tint,
                        light: submit.light,
                        overlay: submit.overlay,
                        normal,
                    });
                }
                mesh.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base,
                    base + 2,
                    base + 3,
                ]);
                last_sin = sin;
                last_cos = cos;
                last_u = u;
            }
        },
    );
}

const OWNED_EQUIPMENT_TEXTURE_PROXY_PATH: &str = "bbb:owned_equipment_texture";

fn resolved_equipment_layer_texture(
    layer: ResolvedEntityEquipmentAssetLayer,
) -> (EntityModelTextureRef, Option<u32>) {
    match layer.texture {
        ResolvedEntityEquipmentAssetLayerTexture::Static(texture) => (texture, None),
        ResolvedEntityEquipmentAssetLayerTexture::Owned {
            index,
            logical_size,
        } => (
            EntityModelTextureRef {
                path: OWNED_EQUIPMENT_TEXTURE_PROXY_PATH,
                size: logical_size,
            },
            Some(index),
        ),
    }
}

fn resolved_equipment_asset_layers(
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    asset_id: Option<bbb_render_types::EntityEquipmentAssetId>,
    layer_type: EntityEquipmentLayerType,
) -> Option<&[ResolvedEntityEquipmentAssetLayer]> {
    asset_id
        .and_then(|asset_id| equipment_assets.asset(asset_id))
        .map(|asset| asset.layers(layer_type))
}

#[derive(Clone, Copy)]
struct EquipmentLayerSubmitSequence {
    next: u32,
    increment: bool,
}

impl EquipmentLayerSubmitSequence {
    const fn fixed(sequence: u32) -> Self {
        Self {
            next: sequence,
            increment: false,
        }
    }

    const fn incrementing(first: u32) -> Self {
        Self {
            next: first,
            increment: true,
        }
    }

    fn current(self) -> u32 {
        self.next
    }

    fn advance(&mut self) {
        self.next += u32::from(self.increment);
    }

    fn next(self) -> u32 {
        self.next
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_equipment_asset_layers(
    meshes: &mut EntityModelTexturedMeshes,
    layers: &[ResolvedEntityEquipmentAssetLayer],
    kind: EntityModelLayerKind,
    model_layer: &'static str,
    transform: Mat4,
    dye: Option<u32>,
    foil: bool,
    trim: Option<EntityArmorTrim>,
    trim_visible: bool,
    starting_order: i32,
    mut submit_sequence: EquipmentLayerSubmitSequence,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    player_texture_override: Option<EntityDynamicPlayerTexture>,
    mut emit: impl FnMut(
        &mut EntityModelTexturedMesh,
        EntityModelTextureRef,
        EntityModelUvRect,
        [f32; 4],
    ),
) -> u32 {
    if layers.is_empty() {
        return submit_sequence.next();
    }
    let mut next_order = starting_order;
    let mut foil_pending = foil;
    let mut trim_base_texture = resolved_equipment_layer_texture(layers[0]).0;
    for layer in layers.iter().copied() {
        let Some(tint) = equipment_layer_tint(layer.dyeable, dye) else {
            continue;
        };
        let (equipment_texture, owned_texture) = resolved_equipment_layer_texture(layer);
        let profile_texture = layer
            .use_player_texture
            .then_some(player_texture_override)
            .flatten();
        let pass_texture = profile_texture
            .map(player_profile_wings_texture_ref)
            .unwrap_or(equipment_texture);
        trim_base_texture = pass_texture;
        let pass = equipment_layer_pass(
            kind,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            model_layer,
            pass_texture,
            tint,
            next_order,
            submit_sequence.current(),
        );
        let mut submit = textured_layer_submission(meshes, pass, transform);
        if let Some(profile_texture) = profile_texture {
            submit = submit.with_dynamic_player_texture(profile_texture);
            if let Some(entry) =
                dynamic_player_texture_atlas_entry(dynamic_texture_atlas, profile_texture)
            {
                render_textured_dynamic_player_texture_submission(
                    meshes,
                    submit,
                    entry,
                    |mesh, entry| emit(mesh, submit.texture, entry.uv, submit.tint),
                );
            } else {
                // Preserve vanilla-shaped submission metadata even while the
                // selected dynamic player texture is pending atlas residency.
                meshes.record_submission(submit);
            }
        } else {
            if let Some(owned_texture) = owned_texture {
                submit = submit.with_owned_texture(owned_texture);
            }
            render_textured_submission_with_owned(
                meshes,
                submit,
                atlas,
                owned_entries,
                |mesh, entry| emit(mesh, submit.texture, entry.uv, submit.tint),
            );
        }
        next_order += 1;
        submit_sequence.advance();

        if foil_pending {
            let pass = equipment_layer_pass(
                kind,
                EntityModelLayerRenderType::ArmorEntityGlint,
                model_layer,
                ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
                tint,
                next_order,
                submit_sequence.current(),
            );
            let submit = no_overlay_layer_submission(pass, transform);
            render_textured_submission(meshes, submit, atlas, |mesh, entry| {
                emit(mesh, submit.texture, entry.uv, submit.tint);
            });
            next_order += 1;
            submit_sequence.advance();
            foil_pending = false;
        }
    }

    let Some(trim) = trim.filter(|_| trim_visible) else {
        return submit_sequence.next();
    };
    let render_type = if trim.decal {
        EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
    } else {
        EntityModelLayerRenderType::ArmorTrimCutoutNoCull
    };
    let pass = equipment_layer_pass(
        kind,
        render_type,
        model_layer,
        trim_base_texture,
        [1.0; 4],
        next_order,
        submit_sequence.current(),
    );
    let submit = no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
    if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
        render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
            emit(mesh, submit.texture, entry.uv, submit.tint);
        });
    } else {
        meshes.record_submission(submit);
    }
    submit_sequence.advance();
    submit_sequence.next()
}

/// The `HumanoidArmorLayer` worn-armor overlay (vanilla `HumanoidArmorLayer.submit`): for each filled
/// equipment slot the inflated `HumanoidArmorModel` piece (helmet / chestplate / leggings / boots) is
/// draped on the host humanoid's posed limbs ([`ModelPart::copy_child_poses_from`] = vanilla
/// `copyPropertiesTo`) and drawn into the cutout pass with the material's equipment-asset texture. The
/// pieces render in the vanilla order (chest, legs, feet, head). `transform` is the host entity's root
/// transform so the armor sits exactly on the body. Enchanted armor records the vanilla
/// `armorEntityGlint` submission at `order(2)` with the same slot-order `submit_sequence` as that
/// slot's `order(1)` armor layer; adult humanoid trims follow at the next order and baby humanoid
/// layer types suppress them.
fn emit_humanoid_armor(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    host_root: &ModelPart,
    transform: Mat4,
    outer: f32,
    baby_kind: Option<HumanoidBabyArmorKind>,
    armor_model_layers: HumanoidArmorModelLayerSet,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let render_state = &instance.render_state;
    let trim_visible = match instance.kind {
        EntityModelKind::Humanoid { baby, .. }
        | EntityModelKind::Zombie { baby }
        | EntityModelKind::ZombieVariant { baby, .. }
        | EntityModelKind::Piglin { baby, .. } => !baby,
        _ => true,
    };
    for (slot, asset_id, material, dye, foil, trim) in [
        (
            HumanoidArmorSlot::Chest,
            render_state.chest_equipment_asset,
            render_state.chest_armor,
            render_state.chest_armor_dye,
            render_state.chest_armor_foil,
            render_state.chest_armor_trim,
        ),
        (
            HumanoidArmorSlot::Legs,
            render_state.legs_equipment_asset,
            render_state.legs_armor,
            render_state.legs_armor_dye,
            render_state.legs_armor_foil,
            render_state.legs_armor_trim,
        ),
        (
            HumanoidArmorSlot::Feet,
            render_state.feet_equipment_asset,
            render_state.feet_armor,
            render_state.feet_armor_dye,
            render_state.feet_armor_foil,
            render_state.feet_armor_trim,
        ),
        (
            HumanoidArmorSlot::Head,
            render_state.head_equipment_asset,
            render_state.head_armor,
            render_state.head_armor_dye,
            render_state.head_armor_foil,
            render_state.head_armor_trim,
        ),
    ] {
        let submit_sequence = match slot {
            HumanoidArmorSlot::Chest => 1,
            HumanoidArmorSlot::Legs => 2,
            HumanoidArmorSlot::Feet => 3,
            HumanoidArmorSlot::Head => 4,
        };
        let mut tree = if let Some(kind) = baby_kind {
            slot.build_baby_tree(kind)
        } else {
            slot.build_tree(outer)
        };
        if baby_kind.is_some() {
            tree.copy_child_animation_from(host_root, slot.baby_pose_part_names());
        } else {
            tree.copy_child_poses_from(host_root, slot.part_names());
        }
        if !equipment_assets.uses_legacy_static_fallback() {
            let layer_type = if baby_kind.is_some() {
                EntityEquipmentLayerType::HumanoidBaby
            } else if slot == HumanoidArmorSlot::Legs {
                EntityEquipmentLayerType::HumanoidLeggings
            } else {
                EntityEquipmentLayerType::Humanoid
            };
            let Some(layers) = asset_id
                .and_then(|asset_id| equipment_assets.asset(asset_id))
                .map(|asset| asset.layers(layer_type))
            else {
                continue;
            };
            emit_equipment_asset_layers(
                meshes,
                layers,
                EntityModelLayerKind::HumanoidArmor,
                armor_model_layers.model_layer(slot),
                transform,
                dye,
                foil,
                trim,
                trim_visible,
                1,
                EquipmentLayerSubmitSequence::fixed(submit_sequence),
                atlas,
                owned_entries,
                dynamic_texture_atlas,
                None,
                |mesh, texture, uv, tint| {
                    tree.render_textured(mesh, transform, texture, uv, tint);
                },
            );
            continue;
        }
        let Some(material) = material else {
            continue;
        };
        let Some(texture) = armor_slot_texture_for_layer(material, slot, baby_kind.is_some())
        else {
            continue;
        };
        let pass = humanoid_armor_layer_pass(
            armor_model_layers,
            slot,
            material,
            texture,
            dye,
            submit_sequence,
        );
        let submit = textured_layer_submission(meshes, pass, transform);
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            tree.render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        if foil {
            let pass = equipment_layer_pass(
                EntityModelLayerKind::HumanoidArmor,
                EntityModelLayerRenderType::ArmorEntityGlint,
                armor_model_layers.model_layer(slot),
                ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
                pass.tint,
                2,
                submit_sequence,
            );
            let submit = no_overlay_layer_submission(pass, transform);
            render_textured_submission(meshes, submit, atlas, |mesh, entry| {
                tree.render_textured(
                    mesh,
                    submit.transform,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            });
        }
        if trim_visible {
            if let Some(trim) = trim {
                emit_humanoid_armor_trim(
                    meshes,
                    &tree,
                    transform,
                    armor_model_layers,
                    slot,
                    texture,
                    trim,
                    foil,
                    submit_sequence,
                    dynamic_texture_atlas,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_humanoid_armor_trim(
    meshes: &mut EntityModelTexturedMeshes,
    tree: &ModelPart,
    transform: Mat4,
    armor_model_layers: HumanoidArmorModelLayerSet,
    slot: HumanoidArmorSlot,
    base_texture: EntityModelTextureRef,
    trim: EntityArmorTrim,
    foil: bool,
    submit_sequence: u32,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) else {
        return;
    };
    let render_type = if trim.decal {
        EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
    } else {
        EntityModelLayerRenderType::ArmorTrimCutoutNoCull
    };
    let pass = equipment_layer_pass(
        EntityModelLayerKind::HumanoidArmor,
        render_type,
        armor_model_layers.model_layer(slot),
        base_texture,
        [1.0; 4],
        if foil { 3 } else { 2 },
        submit_sequence,
    );
    let submit = no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
    render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
        tree.render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn emit_worn_armor_stand_armor(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::ArmorStand {
        small,
        show_arms,
        show_base_plate,
        pose,
        ..
    } = instance.kind
    else {
        return;
    };
    let render_state = &instance.render_state;
    let no_asset = render_state.head_equipment_asset.is_none()
        && render_state.chest_equipment_asset.is_none()
        && render_state.legs_equipment_asset.is_none()
        && render_state.feet_equipment_asset.is_none();
    let no_legacy_armor = render_state.head_armor.is_none()
        && render_state.chest_armor.is_none()
        && render_state.legs_armor.is_none()
        && render_state.feet_armor.is_none();
    if (!equipment_assets.uses_legacy_static_fallback() && no_asset)
        || (equipment_assets.uses_legacy_static_fallback() && no_legacy_armor)
    {
        return;
    }
    let mut host = ArmorStandModel::new(small, show_arms, show_base_plate, pose);
    host.prepare(&instance);
    let transform = entity_model_root_transform(instance);
    let armor_model_layers = if small {
        HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND_SMALL
    } else {
        HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND
    };
    for (slot, asset_id, material, dye, foil, trim) in [
        (
            HumanoidArmorSlot::Chest,
            render_state.chest_equipment_asset,
            render_state.chest_armor,
            render_state.chest_armor_dye,
            render_state.chest_armor_foil,
            render_state.chest_armor_trim,
        ),
        (
            HumanoidArmorSlot::Legs,
            render_state.legs_equipment_asset,
            render_state.legs_armor,
            render_state.legs_armor_dye,
            render_state.legs_armor_foil,
            render_state.legs_armor_trim,
        ),
        (
            HumanoidArmorSlot::Feet,
            render_state.feet_equipment_asset,
            render_state.feet_armor,
            render_state.feet_armor_dye,
            render_state.feet_armor_foil,
            render_state.feet_armor_trim,
        ),
        (
            HumanoidArmorSlot::Head,
            render_state.head_equipment_asset,
            render_state.head_armor,
            render_state.head_armor_dye,
            render_state.head_armor_foil,
            render_state.head_armor_trim,
        ),
    ] {
        let submit_sequence = match slot {
            HumanoidArmorSlot::Chest => 1,
            HumanoidArmorSlot::Legs => 2,
            HumanoidArmorSlot::Feet => 3,
            HumanoidArmorSlot::Head => 4,
        };
        let mut tree = slot.build_armor_stand_tree(small, STANDARD_OUTER_ARMOR_DEFORMATION);
        tree.copy_child_poses_from(host.root(), slot.part_names());
        if !equipment_assets.uses_legacy_static_fallback() {
            let layer_type = if slot == HumanoidArmorSlot::Legs {
                EntityEquipmentLayerType::HumanoidLeggings
            } else {
                EntityEquipmentLayerType::Humanoid
            };
            let Some(layers) = asset_id
                .and_then(|asset_id| equipment_assets.asset(asset_id))
                .map(|asset| asset.layers(layer_type))
            else {
                continue;
            };
            emit_equipment_asset_layers(
                meshes,
                layers,
                EntityModelLayerKind::HumanoidArmor,
                armor_model_layers.model_layer(slot),
                transform,
                dye,
                foil,
                trim,
                true,
                1,
                EquipmentLayerSubmitSequence::fixed(submit_sequence),
                atlas,
                owned_entries,
                dynamic_texture_atlas,
                None,
                |mesh, texture, uv, tint| {
                    tree.render_textured(mesh, transform, texture, uv, tint);
                },
            );
            continue;
        }
        let Some(material) = material else {
            continue;
        };
        let Some(texture) = armor_slot_texture_for_layer(material, slot, false) else {
            continue;
        };
        let pass = humanoid_armor_layer_pass(
            armor_model_layers,
            slot,
            material,
            texture,
            dye,
            submit_sequence,
        );
        let submit = textured_layer_submission(meshes, pass, transform);
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            tree.render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        if foil {
            let pass = equipment_layer_pass(
                EntityModelLayerKind::HumanoidArmor,
                EntityModelLayerRenderType::ArmorEntityGlint,
                armor_model_layers.model_layer(slot),
                ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
                pass.tint,
                2,
                submit_sequence,
            );
            let submit = no_overlay_layer_submission(pass, transform);
            render_textured_submission(meshes, submit, atlas, |mesh, entry| {
                tree.render_textured(
                    mesh,
                    submit.transform,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            });
        }
        if let Some(trim) = trim {
            emit_humanoid_armor_trim(
                meshes,
                &tree,
                transform,
                armor_model_layers,
                slot,
                texture,
                trim,
                foil,
                submit_sequence,
                dynamic_texture_atlas,
            );
        }
    }
}

/// Worn armor for the humanoid armor wearers (vanilla `HumanoidModel.createArmorMeshSet`, `INNER 0.5`
/// / `OUTER 1.0`, the standard baby zombie/husk/drowned `createBabyArmorMeshSet`, or the piglin
/// family's `OUTER 1.02`). The base body is emitted by the shared dispatch / bespoke emits; here we
/// rebuild and pose an identical host humanoid model purely to read its limb poses, then drape the
/// armor pieces on it ([`emit_humanoid_armor`]). Covered: the adult zombie family (zombie, husk,
/// drowned, zombie villager), standard baby zombie/husk/drowned/zombie-villager armor, the skeleton
/// family (skeleton, stray, wither/normal/bogged), the player, the adult piglin family (piglin,
/// piglin brute, zombified piglin), and baby piglin / zombified-piglin armor models. Adult
/// humanoid and armor-stand armor trims share these posed meshes; remaining mob-specific armor
/// models are deferred.
pub(in crate::entity_models) fn render_worn_humanoid_armor(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let render_state = &instance.render_state;
    let no_asset = render_state.head_equipment_asset.is_none()
        && render_state.chest_equipment_asset.is_none()
        && render_state.legs_equipment_asset.is_none()
        && render_state.feet_equipment_asset.is_none();
    let no_legacy_armor = render_state.head_armor.is_none()
        && render_state.chest_armor.is_none()
        && render_state.legs_armor.is_none()
        && render_state.feet_armor.is_none();
    if (!equipment_assets.uses_legacy_static_fallback() && no_asset)
        || (equipment_assets.uses_legacy_static_fallback() && no_legacy_armor)
    {
        return;
    }
    match instance.kind {
        EntityModelKind::ArmorStand { .. } => {
            emit_worn_armor_stand_armor(
                meshes,
                instance,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Zombie { baby: false } => {
            let mut host = ZombieModel::new(false);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Zombie { baby: true } => {
            let mut host = ZombieModel::new(true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Standard),
                HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_BABY,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Giant => {
            let mut host = ZombieModel::new(false);
            host.prepare(&instance);
            let transform = mesh_transformer_scaled_model_root_transform(instance, GIANT_SCALE);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                HUMANOID_ARMOR_MODEL_LAYERS_GIANT,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family,
            baby: false,
        } => {
            // The husk wears the `HUSK_SCALE` mesh-transformer scale; the other variants render at 1.0×.
            let transform = if matches!(family, ZombieVariantModelFamily::Husk) {
                mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
            } else if matches!(family, ZombieVariantModelFamily::Drowned) {
                drowned_model_root_transform(instance)
            } else {
                entity_model_root_transform(instance)
            };
            let armor_model_layers = match family {
                ZombieVariantModelFamily::Husk => HUMANOID_ARMOR_MODEL_LAYERS_HUSK,
                ZombieVariantModelFamily::Drowned => HUMANOID_ARMOR_MODEL_LAYERS_DROWNED,
                ZombieVariantModelFamily::ZombieVillager => {
                    HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER
                }
            };
            let mut host = ZombieVariantModel::new(family, false);
            host.prepare(&instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                armor_model_layers,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family: family @ (ZombieVariantModelFamily::Husk | ZombieVariantModelFamily::Drowned),
            baby: true,
        } => {
            let transform = if matches!(family, ZombieVariantModelFamily::Husk) {
                mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
            } else {
                drowned_model_root_transform(instance)
            };
            let armor_model_layers = if matches!(family, ZombieVariantModelFamily::Husk) {
                HUMANOID_ARMOR_MODEL_LAYERS_HUSK_BABY
            } else {
                HUMANOID_ARMOR_MODEL_LAYERS_DROWNED_BABY
            };
            let mut host = ZombieVariantModel::new(family, true);
            host.prepare(&instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Standard),
                armor_model_layers,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true,
        } => {
            let mut host = ZombieVariantModel::new(ZombieVariantModelFamily::ZombieVillager, true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Standard),
                HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER_BABY,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Skeleton => {
            let mut host = SkeletonModel::new(None);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                HUMANOID_ARMOR_MODEL_LAYERS_SKELETON,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::SkeletonVariant { family } => {
            let transform = if matches!(family, SkeletonModelFamily::WitherSkeleton) {
                wither_skeleton_model_root_transform(instance)
            } else {
                entity_model_root_transform(instance)
            };
            let armor_model_layers = match family {
                SkeletonModelFamily::Stray => HUMANOID_ARMOR_MODEL_LAYERS_STRAY,
                SkeletonModelFamily::Parched => HUMANOID_ARMOR_MODEL_LAYERS_PARCHED,
                SkeletonModelFamily::WitherSkeleton => HUMANOID_ARMOR_MODEL_LAYERS_WITHER_SKELETON,
                SkeletonModelFamily::Bogged { .. } => HUMANOID_ARMOR_MODEL_LAYERS_BOGGED,
            };
            let mut host = SkeletonModel::new(Some(family));
            host.prepare(&instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                armor_model_layers,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Player { skin, .. } => {
            let slim = skin.is_slim();
            let mut host = PlayerModel::new(slim);
            host.prepare(&instance);
            let transform = player_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                if slim {
                    HUMANOID_ARMOR_MODEL_LAYERS_PLAYER_SLIM
                } else {
                    HUMANOID_ARMOR_MODEL_LAYERS_PLAYER
                },
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Piglin {
            family,
            baby: false,
        } => {
            // The piglin family (piglin, piglin brute, zombified piglin) wears the same base armor mesh
            // grown by the piglin `1.02` outer deformation (vanilla `AbstractPiglinModel.createArmorMeshSet`
            // = `PlayerModel.createArmorMeshSet(..).map(removeEars)`; the removed ears and the player's
            // empty sleeve/pants parts carry no geometry, so it is the standard mesh).
            let mut host = PiglinModel::new(family, false);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            let armor_model_layers = match family {
                PiglinModelFamily::Piglin => HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN,
                PiglinModelFamily::PiglinBrute => HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BRUTE,
                PiglinModelFamily::ZombifiedPiglin => HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN,
            };
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                PIGLIN_OUTER_ARMOR_DEFORMATION,
                None,
                armor_model_layers,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Piglin { family, baby: true }
            if family != PiglinModelFamily::PiglinBrute =>
        {
            let mut host = PiglinModel::new(family, true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            let armor_model_layers = match family {
                PiglinModelFamily::Piglin => HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BABY,
                PiglinModelFamily::ZombifiedPiglin => {
                    HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN_BABY
                }
                PiglinModelFamily::PiglinBrute => unreachable!("baby brute is handled separately"),
            };
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                PIGLIN_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Piglin),
                armor_model_layers,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: true,
        } => {
            let mut host = PiglinModel::new(PiglinModelFamily::PiglinBrute, true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                PIGLIN_OUTER_ARMOR_DEFORMATION,
                None,
                HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BRUTE,
                atlas,
                owned_entries,
                equipment_assets,
                dynamic_texture_atlas,
            );
        }
        _ => {}
    }
}

pub(in crate::entity_models) fn render_skeleton_clothing_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let EntityModelKind::SkeletonVariant { family } = instance.kind else {
        return;
    };
    let Some(pass) = skeleton_textured_layer_passes(Some(family))
        .into_iter()
        .find(|pass| matches!(pass.kind, EntityModelLayerKind::SkeletonClothing))
    else {
        return;
    };
    let transform = if matches!(family, SkeletonModelFamily::WitherSkeleton) {
        wither_skeleton_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    let mut model = SkeletonClothingModel::new(Some(family));
    model.prepare(&instance);
    render_textured_layers(meshes, &model, transform, [pass], atlas);
}

pub(in crate::entity_models) fn render_custom_head_skull_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    if instance.render_state.custom_head_skull.is_none() {
        return;
    }
    let Some(transform) = custom_head_skull_transform(&instance) else {
        return;
    };
    render_custom_head_skull_layer_at_transform(
        meshes,
        instance,
        transform,
        atlas,
        dynamic_player_skin_atlas,
    );
}

pub(in crate::entity_models) fn render_custom_head_skull_layer_with_root_transform(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    root_transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    if instance.render_state.custom_head_skull.is_none() {
        return;
    }
    let Some(transform) = custom_head_skull_transform_with_root(&instance, root_transform) else {
        return;
    };
    render_custom_head_skull_layer_at_transform(
        meshes,
        instance,
        transform,
        atlas,
        dynamic_player_skin_atlas,
    );
}

pub(in crate::entity_models) fn render_skull_block_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    skull: EntityCustomHeadSkull,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    match skull {
        EntityCustomHeadSkull::Dragon => {
            let mut model = CustomHeadDragonSkullModel::new();
            model.prepare(&instance);
            let pass = custom_head_skull_layer_pass(skull, custom_head_skull_texture_ref(skull));
            render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
            return;
        }
        EntityCustomHeadSkull::Piglin => {
            let mut model = CustomHeadPiglinSkullModel::new();
            model.prepare(&instance);
            let pass = custom_head_skull_layer_pass(skull, custom_head_skull_texture_ref(skull));
            render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
            return;
        }
        _ => {}
    }

    let mut model = CustomHeadSkullModel::new(matches!(skull, EntityCustomHeadSkull::Player(_)));
    model.prepare(&instance);
    let texture = custom_head_skull_texture_ref(skull);
    let pass = custom_head_skull_layer_pass(skull, texture);
    if let Some(dynamic_player_skin) = custom_head_dynamic_player_skin(skull) {
        render_textured_no_overlay_layer_pass_with_dynamic_player_skin(
            meshes,
            &model,
            transform,
            pass,
            dynamic_player_skin,
            atlas,
            dynamic_player_skin_atlas,
        );
        return;
    }
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
}

fn render_custom_head_skull_layer_at_transform(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let Some(skull) = instance.render_state.custom_head_skull else {
        return;
    };
    match skull {
        EntityCustomHeadSkull::Dragon => {
            let mut model = CustomHeadDragonSkullModel::new();
            model.prepare(&instance);
            let pass = custom_head_skull_layer_pass(skull, custom_head_skull_texture_ref(skull))
                .with_submit_sequence(custom_head_skull_submit_sequence(instance));
            render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
            return;
        }
        EntityCustomHeadSkull::Piglin => {
            let mut model = CustomHeadPiglinSkullModel::new();
            model.prepare(&instance);
            let pass = custom_head_skull_layer_pass(skull, custom_head_skull_texture_ref(skull))
                .with_submit_sequence(custom_head_skull_submit_sequence(instance));
            render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
            return;
        }
        _ => {}
    }

    let mut model = CustomHeadSkullModel::new(matches!(skull, EntityCustomHeadSkull::Player(_)));
    model.prepare(&instance);
    let texture = custom_head_skull_texture_ref(skull);
    let pass = custom_head_skull_layer_pass(skull, texture)
        .with_submit_sequence(custom_head_skull_submit_sequence(instance));
    if let Some(dynamic_player_skin) = custom_head_dynamic_player_skin(skull) {
        render_textured_no_overlay_layer_pass_with_dynamic_player_skin(
            meshes,
            &model,
            transform,
            pass,
            dynamic_player_skin,
            atlas,
            dynamic_player_skin_atlas,
        );
        return;
    }
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
}

fn custom_head_skull_submit_sequence(instance: EntityModelInstance) -> u32 {
    match instance.kind {
        EntityModelKind::Player { .. } => PLAYER_CUSTOM_HEAD_LAYER_SUBMIT_SEQUENCE,
        EntityModelKind::ArmorStand { .. } => ARMOR_STAND_CUSTOM_HEAD_LAYER_SUBMIT_SEQUENCE,
        _ => NON_PLAYER_CUSTOM_HEAD_LAYER_SUBMIT_SEQUENCE,
    }
}

fn custom_head_dynamic_player_skin(
    skull: EntityCustomHeadSkull,
) -> Option<EntityDynamicPlayerSkin> {
    match skull {
        EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(skin)) => Some(skin),
        _ => None,
    }
}

fn dynamic_player_skin_atlas_entry(
    atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    handle: u64,
) -> Option<EntityDynamicPlayerSkinAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == handle)
}

fn dynamic_player_texture_atlas_entry(
    atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    texture: EntityDynamicPlayerTexture,
) -> Option<EntityDynamicPlayerTextureAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == texture.handle)
}

fn dynamic_texture_atlas_entry(
    atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    texture: EntityDynamicTexture,
) -> Option<EntityDynamicPlayerTextureAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == texture.handle)
}

fn custom_head_skull_texture_ref(skull: EntityCustomHeadSkull) -> EntityModelTextureRef {
    match skull {
        EntityCustomHeadSkull::Skeleton => SKELETON_TEXTURE_REF,
        EntityCustomHeadSkull::WitherSkeleton => WITHER_SKELETON_TEXTURE_REF,
        EntityCustomHeadSkull::Player(skin) => default_player_skin_texture_ref(skin.fallback()),
        EntityCustomHeadSkull::Zombie => ZOMBIE_TEXTURE_REF,
        EntityCustomHeadSkull::Creeper => CREEPER_TEXTURE_REF,
        EntityCustomHeadSkull::Dragon => ENDER_DRAGON_TEXTURE_REF,
        EntityCustomHeadSkull::Piglin => PIGLIN_TEXTURE_REF,
    }
}

/// Vanilla `HappyGhastRenderer` `SimpleEquipmentLayer(HAPPY_GHAST_BODY)`. Both ages have a harness
/// model; the base texture is followed by optional armor glint and trim submissions.
pub(in crate::entity_models) fn render_happy_ghast_harness_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    first_submit_sequence: u32,
) -> u32 {
    let EntityModelKind::HappyGhast { baby } = instance.kind else {
        return first_submit_sequence;
    };

    let transform = happy_ghast_model_root_transform(instance, baby);
    let model_layer = if baby {
        MODEL_LAYER_HAPPY_GHAST_BABY_HARNESS
    } else {
        MODEL_LAYER_HAPPY_GHAST_HARNESS
    };
    let mut model = HappyGhastHarnessModel::new();
    model.prepare(&instance);
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.happy_ghast_body_equipment_asset,
            EntityEquipmentLayerType::HappyGhastBody,
        ) else {
            return first_submit_sequence;
        };
        return emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::HappyGhastHarness,
            model_layer,
            transform,
            instance.render_state.happy_ghast_harness_dye,
            instance.render_state.happy_ghast_harness_foil,
            instance.render_state.happy_ghast_harness_trim,
            true,
            0,
            EquipmentLayerSubmitSequence::incrementing(first_submit_sequence),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
    }

    let Some(color) = instance.render_state.happy_ghast_harness else {
        return first_submit_sequence;
    };
    let texture = happy_ghast_harness_texture_ref(color);
    let pass = equipment_layer_pass(
        EntityModelLayerKind::HappyGhastHarness,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        model_layer,
        texture,
        [1.0; 4],
        0,
        first_submit_sequence,
    );
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);

    let mut next_order = 1;
    let mut next_submit_sequence = first_submit_sequence + 1;
    if instance.render_state.happy_ghast_harness_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::HappyGhastHarness,
            EntityModelLayerRenderType::ArmorEntityGlint,
            model_layer,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        next_order += 1;
        next_submit_sequence += 1;
    }
    if let Some(trim) = instance.render_state.happy_ghast_harness_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::HappyGhastHarness,
                render_type,
                model_layer,
                texture,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
            next_submit_sequence += 1;
        }
    }
    next_submit_sequence
}

/// Vanilla `HappyGhastRenderer` `RopesLayer`: after the harness equipment layer, render a second
/// happy-ghast model inflated by `CubeDeformation(0.2F)`. The synchronized leash-holder bit and the
/// harness item tag are independent gates in vanilla; bbb projects the latter from the synchronized
/// `minecraft:item` registry tags rather than inferring it from the equipment asset.
pub(in crate::entity_models) fn render_happy_ghast_ropes_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    submit_sequence: u32,
) {
    if !instance.render_state.happy_ghast_is_leash_holder
        || !instance.render_state.happy_ghast_body_item_is_harness
    {
        return;
    }
    let EntityModelKind::HappyGhast { baby } = instance.kind else {
        return;
    };

    let mut model = HappyGhastModel::new_ropes(baby);
    model.prepare(&instance);
    let pass = equipment_layer_pass(
        EntityModelLayerKind::HappyGhastRopes,
        EntityModelLayerRenderType::EntityCutout,
        if baby {
            MODEL_LAYER_HAPPY_GHAST_BABY_ROPES
        } else {
            MODEL_LAYER_HAPPY_GHAST_ROPES
        },
        HAPPY_GHAST_ROPES_TEXTURE_REF,
        [1.0; 4],
        0,
        submit_sequence,
    );
    render_textured_no_overlay_layer_pass(
        meshes,
        &model,
        happy_ghast_model_root_transform(instance, baby),
        pass,
        atlas,
    );
}

/// Vanilla `PigRenderer` `SimpleEquipmentLayer(PIG_SADDLE)`: when the saddle slot contains the
/// saddle item, render an adult `PigModel.createBodyLayer(CubeDeformation(0.5F))` over the base pig
/// with the `pig_saddle/saddle.png` equipment texture. The vanilla layer has no baby model, so baby
/// pigs skip it even if the slot is filled.
pub(in crate::entity_models) fn render_pig_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if (equipment_assets.uses_legacy_static_fallback() && !instance.render_state.pig_saddle)
        || (!equipment_assets.uses_legacy_static_fallback()
            && instance.render_state.pig_saddle_equipment_asset.is_none())
    {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Pig { baby: false, .. }) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let mut model = PigModel::new_saddle();
    model.prepare(&instance);
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.pig_saddle_equipment_asset,
            EntityEquipmentLayerType::PigSaddle,
        ) else {
            return;
        };
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::PigSaddle,
            MODEL_LAYER_PIG_SADDLE,
            transform,
            instance.render_state.pig_saddle_dye,
            instance.render_state.pig_saddle_foil,
            instance.render_state.pig_saddle_trim,
            true,
            0,
            EquipmentLayerSubmitSequence::incrementing(1),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
        return;
    }

    let pass = equipment_layer_pass(
        EntityModelLayerKind::PigSaddle,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        MODEL_LAYER_PIG_SADDLE,
        PIG_SADDLE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
    );
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);

    let mut next_order = 1;
    let mut next_submit_sequence = 2;
    if instance.render_state.pig_saddle_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::PigSaddle,
            EntityModelLayerRenderType::ArmorEntityGlint,
            MODEL_LAYER_PIG_SADDLE,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        next_order += 1;
        next_submit_sequence += 1;
    }
    if let Some(trim) = instance.render_state.pig_saddle_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::PigSaddle,
                render_type,
                MODEL_LAYER_PIG_SADDLE,
                PIG_SADDLE_TEXTURE_REF,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
        }
    }
}

/// Vanilla `WolfArmorLayer`: adult wolves with a body armor item render the `WOLF_BODY` equipment
/// asset over `AdultWolfModel(ModelLayers.WOLF_ARMOR)`, baked with `CubeDeformation(0.2)`. The
/// armadillo-scute asset has a white base layer plus a dye-only overlay; damaged armor then adds an
/// `armorTranslucent` crack texture. `EquipmentLayerRenderer` inserts an optional runtime-atlas
/// trim after all base/foil layers and before those cracks.
pub(in crate::entity_models) fn render_wolf_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    first_submit_sequence: u32,
) -> u32 {
    if !matches!(instance.kind, EntityModelKind::Wolf { baby: false, .. }) {
        return first_submit_sequence;
    }
    let transform = entity_model_root_transform(instance);
    let mut model = WolfModel::armor(wolf_is_angry(instance.kind));
    model.prepare(&instance);
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.wolf_body_equipment_asset,
            EntityEquipmentLayerType::WolfBody,
        ) else {
            return first_submit_sequence;
        };
        let next_submit_sequence = emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::WolfBodyArmor,
            MODEL_LAYER_WOLF_ARMOR,
            transform,
            instance.render_state.wolf_body_armor_dye,
            instance.render_state.wolf_body_armor_foil,
            instance.render_state.wolf_body_armor_trim,
            true,
            1,
            EquipmentLayerSubmitSequence::incrementing(first_submit_sequence),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
        return emit_wolf_body_armor_crack(
            meshes,
            instance,
            &model,
            transform,
            atlas,
            next_submit_sequence,
        );
    }

    let Some(material) = instance.render_state.wolf_body_armor else {
        return first_submit_sequence;
    };
    let Some(layers) = wolf_body_armor_texture_layers(material) else {
        return first_submit_sequence;
    };
    let mut submit_sequence = first_submit_sequence;
    let mut next_order = 1;
    let mut foil_pending = instance.render_state.wolf_body_armor_foil;
    for layer in layers.iter() {
        let Some(tint) =
            wolf_body_armor_layer_tint(layer.dyeable, instance.render_state.wolf_body_armor_dye)
        else {
            continue;
        };
        let order = next_order;
        next_order += 1;
        let pass = equipment_layer_pass(
            EntityModelLayerKind::WolfBodyArmor,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_WOLF_ARMOR,
            layer.texture,
            tint,
            order,
            submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        submit_sequence += 1;

        if foil_pending {
            let pass = equipment_layer_pass(
                EntityModelLayerKind::WolfBodyArmor,
                EntityModelLayerRenderType::ArmorEntityGlint,
                MODEL_LAYER_WOLF_ARMOR,
                ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
                tint,
                next_order,
                submit_sequence,
            );
            render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
            next_order += 1;
            submit_sequence += 1;
            foil_pending = false;
        }
    }

    if let Some(trim) = instance.render_state.wolf_body_armor_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::WolfBodyArmor,
                render_type,
                MODEL_LAYER_WOLF_ARMOR,
                layers[0].texture,
                [1.0; 4],
                next_order,
                submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
            submit_sequence += 1;
        }
    }

    emit_wolf_body_armor_crack(meshes, instance, &model, transform, atlas, submit_sequence)
}

fn emit_wolf_body_armor_crack(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    model: &WolfModel,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
    submit_sequence: u32,
) -> u32 {
    let Some(crackiness) = instance.render_state.wolf_body_armor_crackiness else {
        return submit_sequence;
    };
    let pass = equipment_layer_pass(
        EntityModelLayerKind::WolfBodyArmorCrack,
        EntityModelLayerRenderType::ArmorTranslucent,
        MODEL_LAYER_WOLF_ARMOR,
        wolf_armor_crackiness_texture_ref(crackiness),
        [1.0, 1.0, 1.0, 1.0],
        0,
        submit_sequence,
    );
    render_textured_no_overlay_layer_pass(meshes, model, transform, pass, atlas);
    submit_sequence + 1
}

fn wolf_body_armor_layer_tint(dyeable: bool, dye: Option<u32>) -> Option<[f32; 4]> {
    if dyeable {
        dye.map(opaque_wolf_armor_rgb_to_tint)
    } else {
        Some([1.0, 1.0, 1.0, 1.0])
    }
}

fn opaque_wolf_armor_rgb_to_tint(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
}

fn wolf_is_angry(kind: EntityModelKind) -> bool {
    matches!(kind, EntityModelKind::Wolf { angry: true, .. })
}

/// Vanilla `SimpleEquipmentLayer` over `EquineSaddleModel` for horse, donkey, mule, skeleton-horse,
/// and zombie-horse saddles. The layer has no baby model, so baby equines skip it. The two bridle line
/// parts are visible only while `EquineRenderState.isRidden` is true.
pub(in crate::entity_models) fn render_equine_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    submit_sequence: u32,
) {
    if (equipment_assets.uses_legacy_static_fallback() && !instance.render_state.equine_saddle)
        || (!equipment_assets.uses_legacy_static_fallback()
            && instance
                .render_state
                .equine_saddle_equipment_asset
                .is_none())
    {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let ridden = instance.render_state.equine_saddle_ridden;
    let (parts, transform, model_layer, layer_type, texture, order, submit_sequence): (
        &[TexturedModelPartDesc],
        Mat4,
        &'static str,
        EntityEquipmentLayerType,
        EntityModelTextureRef,
        i32,
        u32,
    ) = match instance.kind {
        EntityModelKind::Horse { baby: false, .. } => (
            if ridden {
                &ADULT_HORSE_SADDLE_RIDDEN_PARTS_TEXTURED
            } else {
                &ADULT_HORSE_SADDLE_PARTS_TEXTURED
            },
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE),
            MODEL_LAYER_HORSE_SADDLE,
            EntityEquipmentLayerType::HorseSaddle,
            HORSE_SADDLE_TEXTURE_REF,
            2,
            submit_sequence,
        ),
        EntityModelKind::Donkey {
            family,
            baby: false,
            ..
        } => {
            let scale = match family {
                DonkeyModelFamily::Donkey => 0.87,
                DonkeyModelFamily::Mule => 0.92,
            };
            let texture = match family {
                DonkeyModelFamily::Donkey => DONKEY_SADDLE_TEXTURE_REF,
                DonkeyModelFamily::Mule => MULE_SADDLE_TEXTURE_REF,
            };
            let model_layer = match family {
                DonkeyModelFamily::Donkey => MODEL_LAYER_DONKEY_SADDLE,
                DonkeyModelFamily::Mule => MODEL_LAYER_MULE_SADDLE,
            };
            let layer_type = match family {
                DonkeyModelFamily::Donkey => EntityEquipmentLayerType::DonkeySaddle,
                DonkeyModelFamily::Mule => EntityEquipmentLayerType::MuleSaddle,
            };
            (
                if ridden {
                    &ADULT_DONKEY_SADDLE_RIDDEN_PARTS_TEXTURED
                } else {
                    &ADULT_DONKEY_SADDLE_PARTS_TEXTURED
                },
                mesh_transformer_scaled_model_root_transform(instance, scale),
                model_layer,
                layer_type,
                texture,
                0,
                1,
            )
        }
        EntityModelKind::UndeadHorse {
            family,
            baby: false,
        } => {
            let texture = match family {
                UndeadHorseModelFamily::Skeleton => SKELETON_HORSE_SADDLE_TEXTURE_REF,
                UndeadHorseModelFamily::Zombie => ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
            };
            let model_layer = match family {
                UndeadHorseModelFamily::Skeleton => MODEL_LAYER_SKELETON_HORSE_SADDLE,
                UndeadHorseModelFamily::Zombie => MODEL_LAYER_ZOMBIE_HORSE_SADDLE,
            };
            let layer_type = match family {
                UndeadHorseModelFamily::Skeleton => EntityEquipmentLayerType::SkeletonHorseSaddle,
                UndeadHorseModelFamily::Zombie => EntityEquipmentLayerType::ZombieHorseSaddle,
            };
            (
                if ridden {
                    &ADULT_HORSE_SADDLE_RIDDEN_PARTS_TEXTURED
                } else {
                    &ADULT_HORSE_SADDLE_PARTS_TEXTURED
                },
                entity_model_root_transform(instance),
                model_layer,
                layer_type,
                texture,
                0,
                submit_sequence,
            )
        }
        _ => return,
    };

    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.equine_saddle_equipment_asset,
            layer_type,
        ) else {
            return;
        };
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::EquineSaddle,
            model_layer,
            transform,
            instance.render_state.equine_saddle_dye,
            instance.render_state.equine_saddle_foil,
            instance.render_state.equine_saddle_trim,
            true,
            order,
            EquipmentLayerSubmitSequence::incrementing(submit_sequence),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                emit_equine_textured_posed(
                    mesh,
                    parts,
                    [2, 3, 4, 5],
                    1,
                    0.0,
                    1.0,
                    transform,
                    texture,
                    uv,
                    tint,
                    instance,
                );
            },
        );
        return;
    }

    let pass = equipment_layer_pass(
        EntityModelLayerKind::EquineSaddle,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        model_layer,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        order,
        submit_sequence,
    );
    emit_equine_textured_submission(
        meshes,
        parts,
        [2, 3, 4, 5],
        1,
        0.0,
        1.0,
        no_overlay_layer_submission(pass, transform),
        instance,
        atlas,
    );

    let mut next_order = order + 1;
    let mut next_submit_sequence = submit_sequence + 1;
    if instance.render_state.equine_saddle_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::EquineSaddle,
            EntityModelLayerRenderType::ArmorEntityGlint,
            model_layer,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        emit_equine_textured_submission(
            meshes,
            parts,
            [2, 3, 4, 5],
            1,
            0.0,
            1.0,
            no_overlay_layer_submission(pass, transform),
            instance,
            atlas,
        );
        next_order += 1;
        next_submit_sequence += 1;
    }

    if let Some(trim) = instance.render_state.equine_saddle_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::EquineSaddle,
                render_type,
                model_layer,
                texture,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    emit_equine_textured_posed(
                        mesh,
                        parts,
                        [2, 3, 4, 5],
                        1,
                        0.0,
                        1.0,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                        instance,
                    );
                },
            );
        }
    }
}

/// Vanilla `HorseRenderer` / `UndeadHorseRenderer` `SimpleEquipmentLayer(HORSE_BODY)`: an adult horse
/// or zombie horse with a body armor item renders `HorseModel(ModelLayers.*_HORSE_ARMOR)`. The living
/// horse armor model inherits the 1.1 `livingHorseScale`; the zombie horse armor model is unscaled.
/// Vanilla supplies no baby model. Skeleton horses use the same renderer class but the vanilla
/// `CAN_WEAR_HORSE_ARMOR` tag excludes them, so the world projection never sets this layer for them.
/// The shared equipment renderer inserts foil after the first base layer and a runtime-atlas trim
/// after all base layers, before a later saddle equipment layer.
pub(in crate::entity_models) fn render_equine_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> u32 {
    let (transform, model_layer, order, first_submit_sequence) = match instance.kind {
        EntityModelKind::Horse { baby: false, .. } => (
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE),
            MODEL_LAYER_HORSE_ARMOR,
            2,
            2,
        ),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false,
        } => (
            entity_model_root_transform(instance),
            MODEL_LAYER_UNDEAD_HORSE_ARMOR,
            0,
            1,
        ),
        EntityModelKind::Horse { .. } => return 2,
        EntityModelKind::UndeadHorse { .. } => return 1,
        _ => return 0,
    };
    if meshes.current_force_transparent || meshes.current_outline_only {
        return first_submit_sequence;
    }
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.equine_body_equipment_asset,
            EntityEquipmentLayerType::HorseBody,
        ) else {
            return first_submit_sequence;
        };
        return emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::EquineBodyArmor,
            model_layer,
            transform,
            instance.render_state.equine_body_armor_dye,
            instance.render_state.equine_body_armor_foil,
            instance.render_state.equine_body_armor_trim,
            true,
            order,
            EquipmentLayerSubmitSequence::incrementing(first_submit_sequence),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                emit_equine_textured_posed(
                    mesh,
                    &ADULT_HORSE_ARMOR_PARTS_TEXTURED,
                    [2, 3, 4, 5],
                    1,
                    0.0,
                    1.0,
                    transform,
                    texture,
                    uv,
                    tint,
                    instance,
                );
            },
        );
    }
    let Some(material) = instance.render_state.equine_body_armor else {
        return first_submit_sequence;
    };
    let Some(layers) = horse_body_armor_texture_layers(material) else {
        return first_submit_sequence;
    };

    let mut next_order = order;
    let mut next_submit_sequence = first_submit_sequence;
    let mut foil_pending = instance.render_state.equine_body_armor_foil;
    for layer in layers.iter() {
        let tint = if layer.dyeable {
            armor_layer_tint(
                EntityArmorMaterial::Leather,
                instance.render_state.equine_body_armor_dye,
            )
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        let pass = equipment_layer_pass(
            EntityModelLayerKind::EquineBodyArmor,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            model_layer,
            layer.texture,
            tint,
            next_order,
            next_submit_sequence,
        );
        emit_equine_textured_submission(
            meshes,
            &ADULT_HORSE_ARMOR_PARTS_TEXTURED,
            [2, 3, 4, 5],
            1,
            0.0,
            1.0,
            no_overlay_layer_submission(pass, transform),
            instance,
            atlas,
        );
        next_order += 1;
        next_submit_sequence += 1;

        if foil_pending {
            let pass = equipment_layer_pass(
                EntityModelLayerKind::EquineBodyArmor,
                EntityModelLayerRenderType::ArmorEntityGlint,
                model_layer,
                ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
                tint,
                next_order,
                next_submit_sequence,
            );
            emit_equine_textured_submission(
                meshes,
                &ADULT_HORSE_ARMOR_PARTS_TEXTURED,
                [2, 3, 4, 5],
                1,
                0.0,
                1.0,
                no_overlay_layer_submission(pass, transform),
                instance,
                atlas,
            );
            next_order += 1;
            next_submit_sequence += 1;
            foil_pending = false;
        }
    }

    if let Some(trim) = instance.render_state.equine_body_armor_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::EquineBodyArmor,
                render_type,
                model_layer,
                layers[0].texture,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    emit_equine_textured_posed(
                        mesh,
                        &ADULT_HORSE_ARMOR_PARTS_TEXTURED,
                        [2, 3, 4, 5],
                        1,
                        0.0,
                        1.0,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                        instance,
                    );
                },
            );
            next_submit_sequence += 1;
        }
    }

    next_submit_sequence
}

/// Vanilla `StriderRenderer` `SimpleEquipmentLayer(STRIDER_SADDLE)`: a non-empty saddle item renders
/// `AdultStriderModel(ModelLayers.STRIDER_SADDLE)` with `strider_saddle/saddle.png`. The layer has no
/// baby model, so baby striders skip it.
pub(in crate::entity_models) fn render_strider_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if (equipment_assets.uses_legacy_static_fallback() && !instance.render_state.strider_saddle)
        || (!equipment_assets.uses_legacy_static_fallback()
            && instance
                .render_state
                .strider_saddle_equipment_asset
                .is_none())
    {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Strider { baby: false, .. }) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let mut model = StriderModel::new(false);
    model.prepare(&instance);
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.strider_saddle_equipment_asset,
            EntityEquipmentLayerType::StriderSaddle,
        ) else {
            return;
        };
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::StriderSaddle,
            MODEL_LAYER_STRIDER_SADDLE,
            transform,
            instance.render_state.strider_saddle_dye,
            instance.render_state.strider_saddle_foil,
            instance.render_state.strider_saddle_trim,
            true,
            0,
            EquipmentLayerSubmitSequence::incrementing(1),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
        return;
    }
    let pass = equipment_layer_pass(
        EntityModelLayerKind::StriderSaddle,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        MODEL_LAYER_STRIDER_SADDLE,
        STRIDER_SADDLE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
    );
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);

    let mut next_order = 1;
    let mut next_submit_sequence = 2;
    if instance.render_state.strider_saddle_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::StriderSaddle,
            EntityModelLayerRenderType::ArmorEntityGlint,
            MODEL_LAYER_STRIDER_SADDLE,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        next_order += 1;
        next_submit_sequence += 1;
    }
    if let Some(trim) = instance.render_state.strider_saddle_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::StriderSaddle,
                render_type,
                MODEL_LAYER_STRIDER_SADDLE,
                STRIDER_SADDLE_TEXTURE_REF,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
        }
    }
}

/// Vanilla `CamelRenderer` / `CamelHuskRenderer` `SimpleEquipmentLayer`: a non-empty saddle item
/// renders `CamelSaddleModel(ModelLayers.CAMEL*_SADDLE)` with the family-specific equipment texture.
/// The layer has no baby model, so baby camels skip it; camel husks are adult-only and always use the
/// adult saddle model.
pub(in crate::entity_models) fn render_camel_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if (equipment_assets.uses_legacy_static_fallback() && !instance.render_state.camel_saddle)
        || (!equipment_assets.uses_legacy_static_fallback()
            && instance.render_state.camel_saddle_equipment_asset.is_none())
    {
        return;
    }

    let (model_layer, layer_type, texture) = match instance.kind {
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false,
        } => (
            MODEL_LAYER_CAMEL_SADDLE,
            EntityEquipmentLayerType::CamelSaddle,
            CAMEL_SADDLE_TEXTURE_REF,
        ),
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            ..
        } => (
            MODEL_LAYER_CAMEL_HUSK_SADDLE,
            EntityEquipmentLayerType::CamelHuskSaddle,
            CAMEL_HUSK_SADDLE_TEXTURE_REF,
        ),
        _ => return,
    };
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let mut model = CamelModel::new_saddle();
    model.prepare(&instance);
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.camel_saddle_equipment_asset,
            layer_type,
        ) else {
            return;
        };
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::CamelSaddle,
            model_layer,
            transform,
            instance.render_state.camel_saddle_dye,
            instance.render_state.camel_saddle_foil,
            instance.render_state.camel_saddle_trim,
            true,
            0,
            EquipmentLayerSubmitSequence::incrementing(1),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
        return;
    }
    let pass = equipment_layer_pass(
        EntityModelLayerKind::CamelSaddle,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        model_layer,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
    );
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);

    let mut next_order = 1;
    let mut next_submit_sequence = 2;
    if instance.render_state.camel_saddle_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::CamelSaddle,
            EntityModelLayerRenderType::ArmorEntityGlint,
            model_layer,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        next_order += 1;
        next_submit_sequence += 1;
    }
    if let Some(trim) = instance.render_state.camel_saddle_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::CamelSaddle,
                render_type,
                model_layer,
                texture,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
        }
    }
}

/// Vanilla `NautilusRenderer` / `ZombieNautilusRenderer` `SimpleEquipmentLayer(NAUTILUS_SADDLE)`:
/// a non-empty saddle item renders `NautilusSaddleModel(ModelLayers.NAUTILUS_SADDLE)` over adult
/// living nautilus and zombie nautilus. The layer has no baby model, so baby living nautilus skip it.
pub(in crate::entity_models) fn render_nautilus_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    submit_sequence: u32,
) {
    if (equipment_assets.uses_legacy_static_fallback() && !instance.render_state.nautilus_saddle)
        || (!equipment_assets.uses_legacy_static_fallback()
            && instance
                .render_state
                .nautilus_saddle_equipment_asset
                .is_none())
    {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if !matches!(
        instance.kind,
        EntityModelKind::Nautilus { baby: false } | EntityModelKind::ZombieNautilus { .. }
    ) {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let mut model = NautilusModel::new_saddle();
    model.prepare(&instance);
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.nautilus_saddle_equipment_asset,
            EntityEquipmentLayerType::NautilusSaddle,
        ) else {
            return;
        };
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::NautilusSaddle,
            MODEL_LAYER_NAUTILUS_SADDLE,
            transform,
            instance.render_state.nautilus_saddle_dye,
            instance.render_state.nautilus_saddle_foil,
            instance.render_state.nautilus_saddle_trim,
            true,
            0,
            EquipmentLayerSubmitSequence::incrementing(submit_sequence),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
        return;
    }
    let pass = equipment_layer_pass(
        EntityModelLayerKind::NautilusSaddle,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        MODEL_LAYER_NAUTILUS_SADDLE,
        NAUTILUS_SADDLE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        0,
        submit_sequence,
    );
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);

    let mut next_order = 1;
    let mut next_submit_sequence = submit_sequence + 1;
    if instance.render_state.nautilus_saddle_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::NautilusSaddle,
            EntityModelLayerRenderType::ArmorEntityGlint,
            MODEL_LAYER_NAUTILUS_SADDLE,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        next_order += 1;
        next_submit_sequence += 1;
    }

    if let Some(trim) = instance.render_state.nautilus_saddle_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::NautilusSaddle,
                render_type,
                MODEL_LAYER_NAUTILUS_SADDLE,
                NAUTILUS_SADDLE_TEXTURE_REF,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
        }
    }
}

/// Vanilla `NautilusRenderer` / `ZombieNautilusRenderer` `SimpleEquipmentLayer(NAUTILUS_BODY)`:
/// a non-empty nautilus body armor item renders `NautilusArmorModel(ModelLayers.NAUTILUS_ARMOR)` over
/// adult living nautilus and zombie nautilus. The layer has no baby model, so baby living nautilus
/// skip it. The shared equipment renderer inserts foil after the base armor and a runtime-atlas
/// trim after the foil, before the later saddle equipment layer.
pub(in crate::entity_models) fn render_nautilus_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> u32 {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return 1;
    }
    if !matches!(
        instance.kind,
        EntityModelKind::Nautilus { baby: false } | EntityModelKind::ZombieNautilus { .. }
    ) {
        return 1;
    }
    let transform = entity_model_root_transform(instance);
    let mut model = NautilusModel::new_armor();
    model.prepare(&instance);
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = resolved_equipment_asset_layers(
            equipment_assets,
            instance.render_state.nautilus_body_equipment_asset,
            EntityEquipmentLayerType::NautilusBody,
        ) else {
            return 1;
        };
        return emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::NautilusBodyArmor,
            MODEL_LAYER_NAUTILUS_ARMOR,
            transform,
            instance.render_state.nautilus_body_armor_dye,
            instance.render_state.nautilus_body_armor_foil,
            instance.render_state.nautilus_body_armor_trim,
            true,
            0,
            EquipmentLayerSubmitSequence::incrementing(1),
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
    }

    let Some(material) = instance.render_state.nautilus_body_armor else {
        return 1;
    };
    let Some(texture) = nautilus_body_armor_texture_ref(material) else {
        return 1;
    };
    let pass = equipment_layer_pass(
        EntityModelLayerKind::NautilusBodyArmor,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        MODEL_LAYER_NAUTILUS_ARMOR,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
    );
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);

    let mut next_order = 1;
    let mut next_submit_sequence = 2;
    if instance.render_state.nautilus_body_armor_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::NautilusBodyArmor,
            EntityModelLayerRenderType::ArmorEntityGlint,
            MODEL_LAYER_NAUTILUS_ARMOR,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        next_order += 1;
        next_submit_sequence += 1;
    }

    if let Some(trim) = instance.render_state.nautilus_body_armor_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::NautilusBodyArmor,
                render_type,
                MODEL_LAYER_NAUTILUS_ARMOR,
                texture,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
            next_submit_sequence += 1;
        }
    }

    next_submit_sequence
}

pub(in crate::entity_models) fn render_llama_decor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let EntityModelKind::Llama {
        family,
        baby,
        has_chest,
        ..
    } = instance.kind
    else {
        return;
    };
    let mut model = LlamaModel::new_decor(baby, has_chest);
    model.prepare(&instance);
    let transform = entity_model_root_transform(instance);
    let model_layer = if baby {
        MODEL_LAYER_LLAMA_BABY_DECOR
    } else {
        MODEL_LAYER_LLAMA_DECOR
    };
    if !equipment_assets.uses_legacy_static_fallback() && !baby {
        if let Some(asset_id) = instance.render_state.llama_body_equipment_asset {
            let Some(layers) = resolved_equipment_asset_layers(
                equipment_assets,
                Some(asset_id),
                EntityEquipmentLayerType::LlamaBody,
            ) else {
                return;
            };
            emit_equipment_asset_layers(
                meshes,
                layers,
                EntityModelLayerKind::LlamaDecor,
                model_layer,
                transform,
                instance.render_state.llama_body_decor_dye,
                instance.render_state.llama_body_decor_foil,
                instance.render_state.llama_body_decor_trim,
                true,
                1,
                EquipmentLayerSubmitSequence::incrementing(1),
                atlas,
                owned_entries,
                dynamic_texture_atlas,
                None,
                |mesh, texture, uv, tint| {
                    model
                        .root()
                        .render_textured(mesh, transform, texture, uv, tint);
                },
            );
            return;
        }
    }
    let texture = match (baby, instance.render_state.llama_body_decor, family) {
        (false, Some(color), _) if equipment_assets.uses_legacy_static_fallback() => {
            llama_body_decor_texture_ref(color)
        }
        (_, _, LlamaModelFamily::TraderLlama) if baby => LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        (_, _, LlamaModelFamily::TraderLlama) => LLAMA_BODY_TRADER_TEXTURE_REF,
        _ => return,
    };

    // Vanilla `EquipmentLayerRenderer.renderLayers(..., order = 1)` renders LLAMA_BODY with
    // `RenderTypes.armorCutoutNoCull`, even though the current backend folds it into the cutout mesh.
    let pass = equipment_layer_pass(
        EntityModelLayerKind::LlamaDecor,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        model_layer,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        1,
        1,
    );
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);

    // The trader fallback above supplies `ItemStack.EMPTY`; only an adult carpet item can carry
    // foil or trim components through `EquipmentLayerRenderer`.
    if baby || instance.render_state.llama_body_decor.is_none() {
        return;
    }
    let mut next_order = 2;
    let mut next_submit_sequence = 2;
    if instance.render_state.llama_body_decor_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::LlamaDecor,
            EntityModelLayerRenderType::ArmorEntityGlint,
            MODEL_LAYER_LLAMA_DECOR,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            next_submit_sequence,
        );
        render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
        next_order += 1;
        next_submit_sequence += 1;
    }
    if let Some(trim) = instance.render_state.llama_body_decor_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::LlamaDecor,
                render_type,
                MODEL_LAYER_LLAMA_DECOR,
                texture,
                [1.0; 4],
                next_order,
                next_submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
        }
    }
}

const VILLAGER_NO_HAT_EXCLUDED_PARTS: [&str; 2] = ["hat", "hat_rim"];

pub(in crate::entity_models) fn render_villager_profession_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    match instance.kind {
        EntityModelKind::Villager { baby } => {
            let transform = if baby {
                entity_model_root_transform(instance)
            } else {
                villager_adult_model_root_transform(instance)
            };
            let mut model = VillagerModel::new(baby);
            model.prepare(&instance);
            emit_villager_data_layers(
                meshes,
                &model,
                transform,
                baby,
                false,
                instance.render_state.villager_model_data,
                atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby,
        } => {
            let transform = entity_model_root_transform(instance);
            let mut model = ZombieVariantModel::new(ZombieVariantModelFamily::ZombieVillager, baby);
            model.prepare(&instance);
            emit_villager_data_layers(
                meshes,
                &model,
                transform,
                baby,
                true,
                instance.render_state.villager_model_data,
                atlas,
            );
        }
        _ => {}
    }
}

fn emit_villager_data_layers<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    baby: bool,
    zombie: bool,
    data: VillagerModelData,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let passes = if zombie {
        zombie_villager_data_textured_layer_passes(baby, data)
    } else {
        villager_data_textured_layer_passes(baby, data)
    };
    let Some(type_pass) = passes.first().copied() else {
        return;
    };
    emit_villager_profession_layer(
        meshes,
        model.root(),
        transform,
        type_pass,
        !villager_type_hat_visible(data, zombie),
        atlas,
    );

    for pass in passes.iter().copied().skip(1) {
        emit_villager_profession_layer(meshes, model.root(), transform, pass, false, atlas);
    }
}

fn emit_villager_profession_layer(
    meshes: &mut EntityModelTexturedMeshes,
    root: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    no_hat: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let submit = textured_layer_submission(meshes, pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        if no_hat {
            root.render_textured_excluding(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
                "",
                &VILLAGER_NO_HAT_EXCLUDED_PARTS,
            );
        } else {
            root.render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StuckBodyPlacement {
    InCube,
    OnSurface,
}

pub(in crate::entity_models) fn render_player_stuck_arrow_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let model = ArrowModel::new();
    render_player_stuck_body_layer(
        meshes,
        instance,
        atlas,
        &model,
        ARROW_TEXTURE_REF,
        instance.render_state.player_arrow_count,
        StuckBodyPlacement::InCube,
        PLAYER_ARROW_LAYER_SUBMIT_SEQUENCE,
        0.9,
    );
}

pub(in crate::entity_models) fn render_player_stinger_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let model = BeeStingerModel::new();
    render_player_stuck_body_layer(
        meshes,
        instance,
        atlas,
        &model,
        BEE_STINGER_TEXTURE_REF,
        instance.render_state.player_stinger_count,
        StuckBodyPlacement::OnSurface,
        PLAYER_STINGER_LAYER_SUBMIT_SEQUENCE,
        1.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_player_stuck_body_layer<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    stuck_model: &M,
    texture: EntityModelTextureRef,
    synchronized_count: i32,
    placement: StuckBodyPlacement,
    submit_sequence: u32,
    model_scale: f32,
) {
    let EntityModelKind::Player { skin, .. } = instance.kind else {
        return;
    };
    let count = usize::try_from(synchronized_count)
        .unwrap_or(0)
        .min(MAX_PLAYER_STUCK_ITEMS_PER_KIND);
    if count == 0 {
        return;
    }

    // Vanilla's layer reads the already-posed parent model. Rebuild through the same PlayerModel
    // constructor/setup path as the base submission so attachment transforms cannot drift into a
    // second pose implementation.
    let mut host = PlayerModel::new(skin.is_slim());
    host.prepare(&instance);
    let mut random = LegacyRandom::new(i64::from(instance.entity_id));
    let root = player_model_root_transform(instance);
    for _ in 0..count {
        let part_index = random.next_int_bound(host.stuck_body_part_count() as i32) as usize;
        let Some(cube_count) = host.stuck_body_part_cube_count(part_index) else {
            continue;
        };
        let Ok(cube_bound) = i32::try_from(cube_count) else {
            continue;
        };
        if cube_bound == 0 {
            continue;
        }
        // Even every vanilla player base part's one-cube list consumes nextInt(1).
        let cube_index = random.next_int_bound(cube_bound) as usize;
        let Some(attachment) = host.stuck_body_attachment(part_index, cube_index) else {
            continue;
        };
        let mut mid = [
            random.next_float(),
            random.next_float(),
            random.next_float(),
        ];
        if placement == StuckBodyPlacement::OnSurface {
            let plane = random.next_int_bound(3) as usize;
            mid[plane] = if mid[plane] > 0.5 { 1.0 } else { 0.5 };
        }
        let point = Vec3::from_array(std::array::from_fn(|axis| {
            (attachment.min[axis] + mid[axis] * (attachment.max[axis] - attachment.min[axis]))
                / 16.0
        }));
        let direction = Vec3::new(
            -(mid[0] * 2.0 - 1.0),
            -(mid[1] * 2.0 - 1.0),
            -(mid[2] * 2.0 - 1.0),
        );
        let direction_xz = (direction.x * direction.x + direction.z * direction.z).sqrt();
        let y_rot = direction.x.atan2(direction.z) - std::f32::consts::FRAC_PI_2;
        let z_rot = direction.y.atan2(direction_xz);
        let transform = root
            * attachment.transform
            * Mat4::from_translation(point)
            * Mat4::from_rotation_y(y_rot)
            * Mat4::from_rotation_z(z_rot)
            * Mat4::from_scale(Vec3::splat(model_scale));
        let submit = no_overlay_submission(
            EntityModelLayerRenderType::EntityCutoutCull,
            texture,
            [1.0; 4],
            transform,
            0,
            submit_sequence,
        );
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            stuck_model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
    }
}

pub(in crate::entity_models) fn render_player_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    skin: EntityPlayerSkin,
    parts: PlayerModelPartVisibility,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    // The unified `PlayerModel` tree drives both render paths; `setup_anim` looks the head, runs the
    // inherited `HumanoidModel` walk swing + idle arm bob, and applies the crouch sneaking pose. The
    // six skin overlay parts (hat/jacket/sleeves/pants) are toggled by the player's part visibility
    // after `prepare` (the colored fallback shows every overlay). Held-item/attack/swim arm poses still
    // defer; the profile cape and WingsLayer emit as separate layers after the base body.
    let transform = player_model_root_transform(*instance);
    let slim = skin.is_slim();
    let mut model = PlayerModel::new(slim);
    model.prepare(instance);
    model.apply_part_visibility(parts);
    let texture = default_player_skin_texture_ref(skin.fallback());
    if let EntityPlayerSkin::Dynamic(dynamic_player_skin) = skin {
        let passes = player_textured_layer_passes_with_texture(slim, parts, texture);
        render_textured_pass_with_dynamic_player_skin(
            meshes,
            &model,
            transform,
            passes[0],
            dynamic_player_skin,
            atlas,
            dynamic_player_skin_atlas,
        );
        return;
    }
    render_textured_layers(
        meshes,
        &model,
        transform,
        player_textured_layer_passes_with_texture(slim, parts, texture),
        atlas,
    );
}

pub(in crate::entity_models) fn render_player_extra_ears_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let EntityModelKind::Player { skin, .. } = instance.kind else {
        return;
    };
    if !instance.render_state.show_extra_ears || instance.render_state.invisible {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let transform = player_model_root_transform(instance);
    let mut model = PlayerEarsModel::new();
    model.prepare(&instance);
    let texture = default_player_skin_texture_ref(skin.fallback());
    let pass = player_extra_ears_layer_pass_with_texture(texture);
    let submit = textured_layer_submission(meshes, pass, transform);

    let submit = if let EntityPlayerSkin::Dynamic(dynamic_player_skin) = skin {
        let submit = submit.with_dynamic_player_skin(dynamic_player_skin);
        if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
            if let Some(entry) = dynamic_player_skin_atlas_entry(
                dynamic_player_skin_atlas,
                dynamic_player_skin.handle,
            ) {
                render_textured_dynamic_player_skin_submission(
                    meshes,
                    submit,
                    entry,
                    |mesh, entry| {
                        model.root().render_textured(
                            mesh,
                            submit.transform,
                            submit.texture,
                            entry.uv,
                            submit.tint,
                        );
                    },
                );
                return;
            }
        }
        submit
    } else {
        submit
    };

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

pub(in crate::entity_models) fn render_player_cape_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::Player { skin, parts } = instance.kind else {
        return;
    };
    if !parts.cape {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if instance.render_state.chest_equipment_has_wings {
        return;
    }
    let Some(cape_texture) = instance.render_state.player_cape_texture else {
        return;
    };
    let Some(entry) =
        dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, cape_texture)
    else {
        return;
    };

    let root = player_model_root_transform(instance);
    let layer_transform = root * player_cape_chest_equipment_transform(&instance);
    let mut model = PlayerModel::new(skin.is_slim());
    model.prepare(&instance);
    let Some(body_transform) = model.root().try_descendant_attach_transform(&["body"]) else {
        return;
    };
    let cape_transform = layer_transform
        * body_transform
        * part_pose_transform(PartPose {
            offset: [0.0, 0.0, 2.0],
            rotation: [0.0, std::f32::consts::PI, 0.0],
        })
        * player_cape_animation_transform(&instance);
    let pass = player_cape_layer_pass();
    let submit = textured_layer_submission(meshes, pass, layer_transform)
        .with_dynamic_player_texture(cape_texture);
    render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
        emit_textured_model_cube(
            mesh,
            cape_transform,
            PLAYER_CAPE_CUBE,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn player_cape_chest_equipment_transform(instance: &EntityModelInstance) -> Mat4 {
    if instance.render_state.chest_equipment_has_humanoid {
        Mat4::from_translation(Vec3::new(0.0, -0.053125, 0.06875))
    } else {
        Mat4::IDENTITY
    }
}

fn player_cape_animation_transform(instance: &EntityModelInstance) -> Mat4 {
    let state = &instance.render_state;
    let rotation = Quat::from_rotation_y(-std::f32::consts::PI)
        * Quat::from_rotation_x(
            (6.0 + state.player_cape_lean / 2.0 + state.player_cape_flap).to_radians(),
        )
        * Quat::from_rotation_z((state.player_cape_lean2 / 2.0).to_radians())
        * Quat::from_rotation_y((180.0 - state.player_cape_lean2 / 2.0).to_radians());
    Mat4::from_quat(rotation)
}

pub(in crate::entity_models) fn render_wings_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if !equipment_assets.uses_legacy_static_fallback() {
        let Some(layers) = instance
            .render_state
            .chest_wings_equipment_asset
            .and_then(|asset_id| equipment_assets.asset(asset_id))
            .map(|asset| asset.layers(EntityEquipmentLayerType::Wings))
        else {
            return;
        };
        let Some((transform, baby)) = wings_layer_transform_and_baby(instance) else {
            return;
        };
        let submit_sequence = wings_layer_submit_sequence(instance);
        let model_layer = if baby {
            MODEL_LAYER_ELYTRA_BABY
        } else {
            MODEL_LAYER_ELYTRA
        };
        let mut model = ElytraModel::new(baby);
        model.prepare(&instance);
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::Wings,
            model_layer,
            transform,
            instance.render_state.chest_armor_dye,
            instance.render_state.chest_wings_foil,
            instance.render_state.chest_wings_trim,
            true,
            0,
            EquipmentLayerSubmitSequence::fixed(submit_sequence),
            atlas,
            owned_entries,
            dynamic_player_texture_atlas,
            player_wings_profile_texture_override(&instance),
            |mesh, texture, uv, tint| {
                model
                    .root()
                    .render_textured(mesh, transform, texture, uv, tint);
            },
        );
        return;
    }

    let Some(layer) = instance.render_state.chest_wings_layer else {
        return;
    };
    let Some((transform, baby)) = wings_layer_transform_and_baby(instance) else {
        return;
    };
    let submit_sequence = wings_layer_submit_sequence(instance);

    let mut model = ElytraModel::new(baby);
    model.prepare(&instance);

    let base_texture = if let Some(profile_texture) = player_wings_profile_texture(&instance, layer)
    {
        let Some(entry) =
            dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, profile_texture)
        else {
            return;
        };
        let pass = wings_layer_pass(
            player_profile_wings_texture_ref(profile_texture),
            baby,
            submit_sequence,
        );
        let submit = textured_layer_submission(meshes, pass, transform)
            .with_dynamic_player_texture(profile_texture);
        render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        pass.texture
    } else {
        let pass = wings_layer_pass(layer.texture, baby, submit_sequence);
        let submit = textured_layer_submission(meshes, pass, transform);
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        pass.texture
    };

    let model_layer = if baby {
        MODEL_LAYER_ELYTRA_BABY
    } else {
        MODEL_LAYER_ELYTRA
    };
    let mut next_order = 1;
    if instance.render_state.chest_wings_foil {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::Wings,
            EntityModelLayerRenderType::ArmorEntityGlint,
            model_layer,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0; 4],
            next_order,
            submit_sequence,
        );
        let submit = no_overlay_layer_submission(pass, transform);
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        next_order += 1;
    }
    if let Some(trim) = instance.render_state.chest_wings_trim {
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_player_texture_atlas, trim.texture)
        {
            let render_type = if trim.decal {
                EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
            } else {
                EntityModelLayerRenderType::ArmorTrimCutoutNoCull
            };
            let pass = equipment_layer_pass(
                EntityModelLayerKind::Wings,
                render_type,
                model_layer,
                base_texture,
                [1.0; 4],
                next_order,
                submit_sequence,
            );
            let submit =
                no_overlay_layer_submission(pass, transform).with_dynamic_texture(trim.texture);
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| {
                    model.root().render_textured(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                    );
                },
            );
        }
    }
}

pub(in crate::entity_models) fn render_player_spin_attack_effect_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Player { .. })
        || instance.render_state.auto_spin_age_ticks.is_none()
    {
        return;
    }

    let mut model = SpinAttackEffectModel::new();
    model.prepare(&instance);
    let pass = player_spin_attack_effect_layer_pass();
    let submit = textured_layer_submission(meshes, pass, player_model_root_transform(instance));
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

pub(in crate::entity_models) fn render_player_parrot_on_shoulder_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Player { .. }) {
        return;
    }
    if let Some(variant) = instance.render_state.player_left_shoulder_parrot {
        emit_player_parrot_on_shoulder_submission(meshes, instance, atlas, variant, true);
    }
    if let Some(variant) = instance.render_state.player_right_shoulder_parrot {
        emit_player_parrot_on_shoulder_submission(meshes, instance, atlas, variant, false);
    }
}

fn emit_player_parrot_on_shoulder_submission(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    variant: ParrotModelVariant,
    is_left: bool,
) {
    let pass = player_parrot_on_shoulder_layer_pass(variant, is_left);
    if layer_pass_hidden_by_invisible(meshes, pass) {
        return;
    }

    let parrot_instance =
        EntityModelInstance::parrot(instance.entity_id, instance.position, 0.0, variant)
            .with_parrot_on_shoulder(true)
            .with_head_look(
                instance.render_state.head_yaw,
                instance.render_state.head_pitch,
            )
            .with_walk_animation(
                instance.render_state.walk_animation_pos,
                instance.render_state.walk_animation_speed,
            )
            .with_age_in_ticks(instance.render_state.age_in_ticks);
    let mut model = ParrotModel::new();
    model.prepare(&parrot_instance);
    let transform = player_shoulder_parrot_transform(instance, is_left);
    let submit = textured_layer_submission(meshes, pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn player_shoulder_parrot_transform(instance: EntityModelInstance, is_left: bool) -> Mat4 {
    player_model_root_transform(instance)
        * Mat4::from_translation(Vec3::new(
            if is_left { 0.4 } else { -0.4 },
            if instance.render_state.is_crouching {
                -1.3
            } else {
                -1.5
            },
            0.0,
        ))
}

fn wings_layer_transform_and_baby(instance: EntityModelInstance) -> Option<(Mat4, bool)> {
    let (root, baby) = match instance.kind {
        EntityModelKind::Player { .. } => (player_model_root_transform(instance), false),
        EntityModelKind::Zombie { baby } => (entity_model_root_transform(instance), baby),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby,
        } => {
            let transform = if baby {
                entity_model_root_transform(instance)
            } else {
                mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
            };
            (transform, baby)
        }
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby,
        } => (drowned_model_root_transform(instance), baby),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby,
        } => (entity_model_root_transform(instance), baby),
        EntityModelKind::Skeleton => (entity_model_root_transform(instance), false),
        EntityModelKind::SkeletonVariant { family } => {
            let transform = if matches!(family, SkeletonModelFamily::WitherSkeleton) {
                wither_skeleton_model_root_transform(instance)
            } else {
                entity_model_root_transform(instance)
            };
            (transform, false)
        }
        EntityModelKind::Piglin { baby, .. } => (entity_model_root_transform(instance), baby),
        EntityModelKind::ArmorStand { small, .. } => (entity_model_root_transform(instance), small),
        _ => return None,
    };
    Some((root * Mat4::from_translation(Vec3::Z * 0.125), baby))
}

fn wings_layer_submit_sequence(instance: EntityModelInstance) -> u32 {
    match instance.kind {
        EntityModelKind::Player { .. } => PLAYER_WINGS_LAYER_SUBMIT_SEQUENCE,
        EntityModelKind::ArmorStand { .. } => ARMOR_STAND_WINGS_LAYER_SUBMIT_SEQUENCE,
        _ => NON_PLAYER_WINGS_LAYER_SUBMIT_SEQUENCE,
    }
}

fn player_wings_profile_texture(
    instance: &EntityModelInstance,
    layer: EntityEquipmentLayerTexture,
) -> Option<EntityDynamicPlayerTexture> {
    if !layer.use_player_texture {
        return None;
    }
    player_wings_profile_texture_override(instance)
}

fn player_wings_profile_texture_override(
    instance: &EntityModelInstance,
) -> Option<EntityDynamicPlayerTexture> {
    let EntityModelKind::Player { parts, .. } = instance.kind else {
        return None;
    };
    instance.render_state.player_elytra_texture.or_else(|| {
        parts
            .cape
            .then_some(instance.render_state.player_cape_texture)
            .flatten()
    })
}

fn player_profile_wings_texture_ref(texture: EntityDynamicPlayerTexture) -> EntityModelTextureRef {
    match texture.kind {
        super::catalog::EntityDynamicPlayerTextureKind::Cape => PLAYER_PROFILE_CAPE_TEXTURE_REF,
        super::catalog::EntityDynamicPlayerTextureKind::Elytra => PLAYER_PROFILE_ELYTRA_TEXTURE_REF,
    }
}

/// The body part index in every equine layer, and the tail's child index under the body. The body is
/// always first; the tail is its first child. (Single source of truth lives in `colored::mounts`; these
/// mirror it for the textured path and are pinned identical by the textured-vs-colored rest test.)
const EQUINE_BODY_PART_INDEX: usize = 0;
const EQUINE_TAIL_CHILD_INDEX: usize = 0;
const BABY_DONKEY_BODY_CHILD_LEG_INDICES: [usize; 4] = [1, 2, 3, 4];
const BABY_DONKEY_BODY_CHILD_HEAD_PARTS_INDEX: usize = 5;
const BABY_DONKEY_TAIL_X_ROT_OFFSET: f32 = -std::f32::consts::FRAC_PI_4;

/// Textured counterpart of `colored::mounts::emit_equine_posed`: applies the vanilla
/// `AbstractEquineModel.setupAnim` default-branch poses — the walking leg swing on the four parts at
/// `leg_indices`, the head look/bob on the `head_parts` (neck) at `head_parts_index`, and the tail walk
/// lift/wag (`tail_x_rot_offset` = `getTailXRotOffset()`, `age_scale` = `getAgeScale()`) on the body's tail
/// child — to a [`TexturedModelPartDesc`] tree, emitting into `mesh` against one `texture`/`uv_rect`/
/// `tint`. The static tree is walked unchanged only when the gait, head look, and tail are all at rest;
/// otherwise the body subtree is hand-emitted so the `&'static` tail child can take the swung pose. The
/// pose math is shared with the colored path (the `equine_*_pose` helpers are geometry-agnostic), so the
/// two paths stay in lockstep.
#[allow(clippy::too_many_arguments)]
fn emit_equine_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    parts: &[TexturedModelPartDesc],
    leg_indices: [usize; 4],
    head_parts_index: usize,
    tail_x_rot_offset: f32,
    age_scale: f32,
    submit: EntityModelSubmissionEmit,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        emit_equine_textured_posed(
            mesh,
            parts,
            leg_indices,
            head_parts_index,
            tail_x_rot_offset,
            age_scale,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
            instance,
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn emit_equine_textured_layer_pass(
    meshes: &mut EntityModelTexturedMeshes,
    parts: &[TexturedModelPartDesc],
    leg_indices: [usize; 4],
    head_parts_index: usize,
    tail_x_rot_offset: f32,
    age_scale: f32,
    pass: EntityModelLayerPass,
    transform: Mat4,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if pass.tint[3] <= 1.0e-5 {
        return;
    }
    if layer_pass_hidden_by_invisible(meshes, pass) {
        return;
    }
    let submit = textured_layer_submission(meshes, pass, transform);
    emit_equine_textured_submission(
        meshes,
        parts,
        leg_indices,
        head_parts_index,
        tail_x_rot_offset,
        age_scale,
        submit,
        instance,
        atlas,
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_equine_textured_posed(
    mesh: &mut EntityModelTexturedMesh,
    parts: &[TexturedModelPartDesc],
    leg_indices: [usize; 4],
    head_parts_index: usize,
    tail_x_rot_offset: f32,
    age_scale: f32,
    transform: Mat4,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
    instance: EntityModelInstance,
) {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    let animation = EquineAnimationPose {
        head_yaw_deg: head_yaw,
        head_pitch_deg: head_pitch,
        walk_animation_pos: limb_swing,
        walk_animation_speed: limb_swing_amount,
        in_water: instance.render_state.in_water,
        age_in_ticks: instance.render_state.age_in_ticks,
        eat_animation: instance.render_state.equine_eat_animation,
        stand_animation: instance.render_state.equine_stand_animation,
        feeding_animation: instance.render_state.equine_feeding_animation,
    };

    let tail_rest = parts[EQUINE_BODY_PART_INDEX].children[EQUINE_TAIL_CHILD_INDEX].pose;
    let posed_tail = equine_tail_pose(
        tail_rest,
        tail_x_rot_offset,
        limb_swing_amount,
        age_scale,
        instance.render_state.equine_animate_tail,
        instance.render_state.age_in_ticks,
    );
    let tail_resting = posed_tail == tail_rest;

    if legs_resting
        && head_look_at_rest(head_yaw, head_pitch)
        && tail_resting
        && animation.event_pose_at_rest()
    {
        emit_textured_model_parts(mesh, parts, transform, texture, uv_rect, tint);
        return;
    }

    let mut posed = parts.to_vec();
    posed[EQUINE_BODY_PART_INDEX].pose =
        equine_body_pose(posed[EQUINE_BODY_PART_INDEX].pose, animation);
    if !legs_resting || animation.stand_animation != 0.0 {
        let left_front_offset = posed[leg_indices[2]].pose.offset;
        let left_hind_y = posed[leg_indices[0]].pose.offset[1];
        for index in leg_indices {
            posed[index].pose = equine_leg_pose(
                posed[index].pose,
                animation,
                EQUINE_STANDARD_LEG_STAND_CONFIG,
                left_front_offset,
                left_hind_y,
            );
        }
    }
    posed[head_parts_index].pose = equine_head_pose(posed[head_parts_index].pose, animation, false);

    // Hand-emit the body subtree so the tail (a `&'static` child) can take the swung pose, then the
    // remaining parts (neck + legs) in depth-first order via the `[1..]` slice.
    let body = &posed[EQUINE_BODY_PART_INDEX];
    let body_transform = transform * part_pose_transform(body.pose);
    let mut body_children = body.children.to_vec();
    body_children[EQUINE_TAIL_CHILD_INDEX].pose = posed_tail;
    for &cube in body.cubes {
        emit_textured_model_cube(mesh, body_transform, cube, texture, uv_rect, tint);
    }
    emit_textured_model_parts(mesh, &body_children, body_transform, texture, uv_rect, tint);
    emit_textured_model_parts(
        mesh,
        &posed[EQUINE_BODY_PART_INDEX + 1..],
        transform,
        texture,
        uv_rect,
        tint,
    );
}

fn emit_baby_donkey_textured_posed(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
    instance: EntityModelInstance,
) {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_yaw = instance.render_state.head_yaw;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    let animation = EquineAnimationPose {
        head_yaw_deg: head_yaw,
        head_pitch_deg: 0.0,
        walk_animation_pos: limb_swing,
        walk_animation_speed: limb_swing_amount,
        in_water: instance.render_state.in_water,
        age_in_ticks: instance.render_state.age_in_ticks,
        eat_animation: instance.render_state.equine_eat_animation,
        stand_animation: instance.render_state.equine_stand_animation,
        feeding_animation: instance.render_state.equine_feeding_animation,
    };

    let body = &BABY_DONKEY_PARTS_TEXTURED[EQUINE_BODY_PART_INDEX];
    let body_pose = equine_body_pose(body.pose, animation);
    let mut body_children = body.children.to_vec();
    if !legs_resting || animation.stand_animation != 0.0 {
        let left_front_offset = body_children[BABY_DONKEY_BODY_CHILD_LEG_INDICES[2]]
            .pose
            .offset;
        let left_hind_y = body_children[BABY_DONKEY_BODY_CHILD_LEG_INDICES[0]]
            .pose
            .offset[1];
        for index in BABY_DONKEY_BODY_CHILD_LEG_INDICES {
            body_children[index].pose = equine_leg_pose(
                body_children[index].pose,
                animation,
                EQUINE_BABY_DONKEY_LEG_STAND_CONFIG,
                left_front_offset,
                left_hind_y,
            );
        }
    }
    body_children[BABY_DONKEY_BODY_CHILD_HEAD_PARTS_INDEX].pose = equine_head_pose(
        body_children[BABY_DONKEY_BODY_CHILD_HEAD_PARTS_INDEX].pose,
        animation,
        true,
    );
    body_children[EQUINE_TAIL_CHILD_INDEX].pose = equine_tail_pose(
        body_children[EQUINE_TAIL_CHILD_INDEX].pose,
        BABY_DONKEY_TAIL_X_ROT_OFFSET,
        limb_swing_amount,
        instance.render_state.age_scale,
        instance.render_state.equine_animate_tail,
        instance.render_state.age_in_ticks,
    );

    let body_transform = transform * part_pose_transform(body_pose);
    for &cube in body.cubes {
        emit_textured_model_cube(mesh, body_transform, cube, texture, uv_rect, tint);
    }
    emit_textured_model_parts(mesh, &body_children, body_transform, texture, uv_rect, tint);
}

/// The textured donkey / mule base layer. Vanilla `DonkeyModel` is the shared
/// `AbstractEquineModel.createBodyMesh` with `modifyMesh` (bigger ears replacing the horse ears, plus the
/// two side chest boxes shown when `hasChest`), on the 64×64 `donkey.png` / `mule.png` at the
/// `DonkeyModel.DONKEY_SCALE` 0.87 / `MULE_SCALE` 0.92 mesh-transformer scale. The ADULT takes the same
/// equine leg swing / head look/bob / tail walk lift as the horse (`AbstractEquineModel.setupAnim`), so
/// it rides `emit_equine_textured_posed`. The BABY is the distinct re-parented `BabyDonkeyModel` mesh
/// (legs/head/tail nested under the body) whose `setupAnim` forces `xRot = -30°` and uses
/// `getTailXRotOffset = -π/4`, so it rides a baby-specific nested-body pose path (unscaled, matching the
/// colored baby path); its empty chest children make `hasChest` immaterial.
pub(in crate::entity_models) fn render_donkey_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    family: DonkeyModelFamily,
    baby: bool,
    has_chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let passes = donkey_textured_layer_passes(family, baby, has_chest);
    let pass = passes[0];
    if baby {
        let transform = entity_model_root_transform(*instance);
        if layer_pass_hidden_by_invisible(meshes, pass) {
            return;
        }
        let submit = textured_layer_submission(meshes, pass, transform);
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            emit_baby_donkey_textured_posed(
                mesh,
                transform,
                submit.texture,
                entry.uv,
                submit.tint,
                *instance,
            );
        });
        return;
    }
    let parts: &[TexturedModelPartDesc] = if has_chest {
        &ADULT_DONKEY_PARTS_WITH_CHEST_TEXTURED
    } else {
        &ADULT_DONKEY_PARTS_TEXTURED
    };
    // `DonkeyModel.DONKEY_SCALE` / `MULE_SCALE` mesh-transformer scaling (mirrors the colored
    // `donkey_model_scale`).
    let scale = match family {
        DonkeyModelFamily::Donkey => 0.87,
        DonkeyModelFamily::Mule => 0.92,
    };
    let transform = mesh_transformer_scaled_model_root_transform(*instance, scale);
    emit_equine_textured_layer_pass(
        meshes,
        parts,
        [2, 3, 4, 5],
        1,
        0.0,
        instance.render_state.age_scale,
        pass,
        transform,
        *instance,
        atlas,
    );
}

/// The textured living horse base layer plus the `HorseMarkingLayer` overlay. Vanilla `HorseRenderer`
/// renders `HorseModel` with a per-coat `horse_<color>(_baby).png` base texture, then layers the white
/// markings (`horse_markings_*(_baby).png`, `entityTranslucent`, `order(1)`) on top when the coat has
/// markings. The adult body carries the `livingHorseScale` 1.1 mesh-transformer scale (`emit_horse_model`'s
/// transform); the baby uses the unscaled re-parented layer. The leg swing / head look/bob / tail walk
/// lift are the shared `AbstractEquineModel.setupAnim` default-branch poses (the same as the undead
/// horse), driven on the textured path here. The variant chooses the base coat, the markings the overlay;
/// both ride the same `HorseModel` pose, so the overlay tracks the body for free.
pub(in crate::entity_models) fn render_horse_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    variant: HorseColorVariant,
    baby: bool,
    markings: HorseMarkings,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let (parts, leg_indices, head_parts_index, tail_x_rot_offset, age_scale, transform): (
        &[TexturedModelPartDesc],
        [usize; 4],
        usize,
        f32,
        f32,
        Mat4,
    ) = if baby {
        (
            &BABY_HORSE_PARTS_TEXTURED,
            [1, 2, 3, 4],
            5,
            -std::f32::consts::FRAC_PI_2,
            instance.render_state.age_scale,
            entity_model_root_transform(*instance),
        )
    } else {
        (
            &ADULT_HORSE_PARTS_TEXTURED,
            [2, 3, 4, 5],
            1,
            0.0,
            instance.render_state.age_scale,
            mesh_transformer_scaled_model_root_transform(*instance, HORSE_SCALE),
        )
    };
    let passes = horse_textured_layer_passes(variant, baby, markings);
    emit_equine_textured_layer_pass(
        meshes,
        parts,
        leg_indices,
        head_parts_index,
        tail_x_rot_offset,
        age_scale,
        passes[0],
        transform,
        *instance,
        atlas,
    );
    // `HorseMarkingLayer`: a translucent white overlay of the SAME posed model, drawn after the base
    // when the coat carries markings (`Markings.NONE` → `INVISIBLE_TEXTURE`, skipped). It rides the
    // identical pose, so re-emitting the same tree into the translucent mesh tracks the body.
    for pass in passes.iter().skip(1).copied() {
        emit_equine_textured_layer_pass(
            meshes,
            parts,
            leg_indices,
            head_parts_index,
            tail_x_rot_offset,
            age_scale,
            pass,
            transform,
            *instance,
            atlas,
        );
    }
}

/// The textured skeleton / zombie horse base layer. Vanilla `UndeadHorseRenderer extends
/// HorseRenderer`, so the undead horses reuse `HorseModel`; the textured body takes the same equine leg
/// swing, head look/bob, and tail walk lift as the colored fallback ([`emit_undead_horse_model`]). Only
/// the texture differs — the tint is white (the `horse_skeleton` / `horse_zombie` texture, not a per-cube
/// color, carries the look). The adult layer uses `HorseModel.createBodyLayer` (legs `[2, 3, 4, 5]`,
/// neck `1`, `getTailXRotOffset = 0`, `ageScale = 1`); the baby uses `BabyHorseModel.createBabyLayer`,
/// which re-parents the parts (legs `[1, 2, 3, 4]`, neck `5`) and overrides `getTailXRotOffset = −π/2`,
/// `ageScale = 0.5`, and the projected `animateTail` yRot wag. The shared posed submission also folds
/// in the projected ridden, eating, feeding, and standing-up poses.
pub(in crate::entity_models) fn render_undead_horse_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    family: UndeadHorseModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let (parts, leg_indices, head_parts_index, tail_x_rot_offset, age_scale): (
        &[TexturedModelPartDesc],
        [usize; 4],
        usize,
        f32,
        f32,
    ) = if baby {
        (
            &BABY_HORSE_PARTS_TEXTURED,
            [1, 2, 3, 4],
            5,
            -std::f32::consts::FRAC_PI_2,
            instance.render_state.age_scale,
        )
    } else {
        (
            &ADULT_HORSE_PARTS_TEXTURED,
            [2, 3, 4, 5],
            1,
            0.0,
            instance.render_state.age_scale,
        )
    };
    let passes = undead_horse_textured_layer_passes(family, baby);
    emit_equine_textured_layer_pass(
        meshes,
        parts,
        leg_indices,
        head_parts_index,
        tail_x_rot_offset,
        age_scale,
        passes[0],
        entity_model_root_transform(*instance),
        *instance,
        atlas,
    );
}

fn entity_model_texture_atlas_entry(
    atlas: &EntityModelTextureAtlasLayout,
    texture: EntityModelTextureRef,
) -> Option<EntityModelTextureAtlasEntry> {
    atlas
        .entries
        .iter()
        .copied()
        .find(|entry| entry.texture == texture)
}
