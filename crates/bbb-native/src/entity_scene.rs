use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;
use crate::terrain_runtime::TerrainUploadState;
use bbb_item_model::{default_player_skin_for_profile_id, NativeItemRuntime};
#[cfg(test)]
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID;
use bbb_protocol::entity_types::*;
#[cfg(test)]
use bbb_protocol::packets::{EntityDataRegistryIdentity, EntityDataRegistrySource};
use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    },
    item_types,
    packets::{
        decode_profile_textures_from_properties, EntityDataEnumSerializer,
        EntityDataRegistryHolder, EntityDataRegistryReference, EntityDataRegistryRevision,
        EntityDataValueKind, EquipmentSlot, GameProfilePropertySummary, ItemStackSummary,
        PlayerSkinPatchSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
    },
};
use bbb_renderer::{
    ArmorStandModelPose, ArrowModelTexture, AxolotlModelVariant, BoatModelFamily, CamelModelFamily,
    CameraFrustum, CameraPose, CatModelVariant, ChickenModelVariant, CopperGolemWeathering,
    CowModelVariant, DonkeyModelFamily, EndCrystalBeamRenderState, EnderDragonAnimationRenderState,
    EnderDragonBeamRenderState, EnderDragonFlightSample, EntityArmorTrim, EntityAttachmentFace,
    EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDyeColor, EntityDynamicPlayerTexture,
    EntityDynamicPlayerTextureKind, EntityDynamicTexture, EntityEquipmentAssetId,
    EntityLeashRopesRenderState, EntityModelInstance, EntityModelKind, EntityPlayerSkin,
    FoxModelVariant, FrogModelVariant, GuardianBeamRenderState, GuardianEyeTrackingRenderState,
    HoglinModelFamily, HorseColorVariant, HorseMarkings, IllagerModelFamily, IronGolemCrackiness,
    LeashRopeRenderState, LlamaModelFamily, LlamaVariant, MooshroomVariant, PandaModelVariant,
    ParrotModelVariant, PigModelVariant, PiglinModelFamily, PlayerModelPartVisibility,
    RabbitModelVariant, SalmonModelSize, SelectionBox, SelectionColoredBox, SelectionLine,
    SelectionOutline, SelectionPoint, SelectionTextLabel, SheepHeadEatPose, SheepWoolColor,
    SkeletonModelFamily, SleepingPose, SpearKineticWeapon, TropicalFishModelShape,
    TropicalFishPattern, UndeadHorseModelFamily, VillagerModelData, VillagerModelProfession,
    VillagerModelType, WolfArmorCrackiness, WolfModelVariant, ZombieNautilusModelVariant,
    ZombieVariantModelFamily, ENTITY_DEFAULT_OUTLINE_COLOR,
};
#[cfg(test)]
use bbb_renderer::{EntityDynamicPlayerSkinStatus, EntityPlayerSkinModel};
#[cfg(test)]
use bbb_world::vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_armadillo_state_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_armor_stand_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_arrow_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_baby_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_bee_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_cat_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_companion_variant_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_copper_golem_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_entity_accessors;
use bbb_world::vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_fox_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_frog_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_ghast_charging_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_goat_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_horse_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_living_entity_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_llama_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_panda_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_parrot_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_phantom_size_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_rabbit_type_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_salmon_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_sheep_wool_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_shulker_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_slime_size_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_tamable_animal_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_thrown_trident_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_turtle_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_vex_flags_accessor_for_entity_type;
#[cfg(test)]
use bbb_world::vanilla_entity_metadata_villager_data_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_wolf_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_zombie_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type;
#[cfg(test)]
use bbb_world::vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type;
#[cfg(test)]
use bbb_world::DisplayEntityCullingState;
#[cfg(test)]
use bbb_world::WolfVariantAssetPresentation;
use bbb_world::VANILLA_ARMADILLO_STATE_SCARED_ID as ARMADILLO_STATE_SCARED_ID;
use bbb_world::{
    vanilla_entity_metadata_creeper_accessors_for_entity_type,
    vanilla_entity_renderer_culling_kind, vanilla_entity_type_illager_arm_pose_uses_celebrating,
    vanilla_entity_type_is_living, ArmorTrimSprite as WorldArmorTrimSprite, CatVariantPresentation,
    DisplayEntityRenderSourceState, EndCrystalBeamSource as WorldEndCrystalBeamSource,
    EnderDragonAnimationSource as WorldEnderDragonAnimationSource,
    EnderDragonBeamSource as WorldEnderDragonBeamSource,
    EntityAttachmentFace as WorldEntityAttachmentFace, EntityModelSourceState,
    EntityNameTagRenderSourceState, EntityPickTargetState, EntityRenderCandidateState,
    EntityRendererSpecialCullingState, EntityShadowRenderSourceState,
    EquipmentAssetId as WorldEquipmentAssetId, FarmAnimalVariantModel,
    FarmAnimalVariantPresentation, FishingHookRenderSourceState, FrogVariantPresentation,
    GuardianBeamSource as WorldGuardianBeamSource,
    GuardianEyeTrackingSource as WorldGuardianEyeTrackingSource,
    ItemFrameFacing as WorldItemFrameFacing, LeashRopeSource as WorldLeashRopeSource,
    VanillaArmorStandMetadataAccessors, VanillaEntityRendererCullingKind,
    WolfArmorCrackiness as WorldWolfArmorCrackiness, WolfVariantPresentation, WorldStore,
    ZombieNautilusVariantModelType, ZombieNautilusVariantPresentation,
    VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION as AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION,
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID as AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
};
use bbb_world::{CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY};
#[cfg(test)]
use bbb_world::{
    VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID as COPPER_GOLEM_STATE_GETTING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID as COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_IDLE_ID as COPPER_GOLEM_STATE_IDLE_ID,
};
use bbb_world::{
    VANILLA_HORSE_COLOR_MASK as HORSE_COLOR_MASK,
    VANILLA_HORSE_MARKINGS_MASK as HORSE_MARKINGS_MASK,
    VANILLA_HORSE_MARKINGS_SHIFT as HORSE_MARKINGS_SHIFT,
};
trait EntityVariantRegistryLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision;

    fn farm_animal_variant_presentation(
        &self,
        _registry: &str,
        _holder_id: i32,
    ) -> Option<FarmAnimalVariantPresentation> {
        None
    }

    fn cat_variant_presentation(&self, _holder_id: i32) -> Option<CatVariantPresentation> {
        None
    }

    fn frog_variant_presentation(&self, _holder_id: i32) -> Option<FrogVariantPresentation> {
        None
    }

    fn wolf_variant_presentation(&self, _holder_id: i32) -> Option<WolfVariantPresentation> {
        None
    }

    fn zombie_nautilus_variant_presentation(
        &self,
        _holder_id: i32,
    ) -> Option<ZombieNautilusVariantPresentation> {
        None
    }

    fn ready_texture(&self, _asset_id: &str) -> Option<EntityDynamicTexture> {
        None
    }

    fn ready_painting_texture(&self, _asset_id: &str) -> Option<EntityDynamicTexture> {
        None
    }
}

impl EntityVariantRegistryLookup for WorldStore {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        let revision = self.registry_access_revision();
        EntityDataRegistryRevision {
            session: revision.session,
            contents: revision.contents,
            tags: revision.tags,
        }
    }
}

struct EntityVariantRenderContext<'a> {
    world: &'a WorldStore,
    textures: &'a NativeEntityVariantTextureRuntime,
}

impl EntityVariantRegistryLookup for EntityVariantRenderContext<'_> {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        let revision = self.world.registry_access_revision();
        EntityDataRegistryRevision {
            session: revision.session,
            contents: revision.contents,
            tags: revision.tags,
        }
    }

    fn farm_animal_variant_presentation(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Option<FarmAnimalVariantPresentation> {
        self.world
            .synchronized_farm_animal_variant_presentation(registry, holder_id)
            .ok()
            .flatten()
    }

    fn cat_variant_presentation(&self, holder_id: i32) -> Option<CatVariantPresentation> {
        self.world
            .synchronized_cat_variant_presentation(holder_id)
            .ok()
            .flatten()
    }

    fn frog_variant_presentation(&self, holder_id: i32) -> Option<FrogVariantPresentation> {
        self.world
            .synchronized_frog_variant_presentation(holder_id)
            .ok()
            .flatten()
    }

    fn wolf_variant_presentation(&self, holder_id: i32) -> Option<WolfVariantPresentation> {
        self.world
            .synchronized_wolf_variant_presentation(holder_id)
            .ok()
            .flatten()
    }

    fn zombie_nautilus_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Option<ZombieNautilusVariantPresentation> {
        self.world
            .synchronized_zombie_nautilus_variant_presentation(holder_id)
            .ok()
            .flatten()
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        self.textures.ready_texture(asset_id)
    }

    fn ready_painting_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        self.textures.ready_painting_texture(asset_id)
    }
}

const ENTITY_HITBOX_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const ENTITY_EYE_HEIGHT_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const ENTITY_DRAGON_PART_HITBOX_COLOR: [f32; 4] = [0.25, 1.0, 0.0, 1.0];
const ENTITY_PASSENGER_VEHICLE_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
const ENTITY_VIEW_VECTOR_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const ENTITY_MISSING_SERVER_LABEL_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const ENTITY_MISSING_SERVER_LABEL_TEXT: &str = "Missing Server Entity";
const ENTITY_EYE_HEIGHT_PADDING: f32 = 0.01;
const ENTITY_VIEW_VECTOR_LENGTH: f32 = 2.0;
const ENTITY_POSITION_POINT_SIZE: f32 = 2.0;
const MANNEQUIN_DEFAULT_MODEL_CUSTOMIZATION: i8 = PlayerModelPartVisibility::ALL_MASK as i8;
/// Vanilla `SpellcasterIllager.DATA_SPELL_CASTING_ID` data id (17): the byte holding the spell
/// id, the first `SpellcasterIllager` accessor after `Raider.IS_CELEBRATING` (16). The byte is
/// `> 0` while `isCastingSpell()`.
/// Vanilla `Pillager.IS_CHARGING_CROSSBOW` data id (17): the boolean set while the pillager draws its
/// crossbow, the first `Pillager` accessor after `Raider.IS_CELEBRATING` (16) — the same slot the
/// spellcaster illagers use for `DATA_SPELL_CASTING_ID`, since each illager subclass adds its own
/// accessor in that position. `getArmPose` returns `CROSSBOW_CHARGE` while true, else `CROSSBOW_HOLD`.
/// Vanilla `Raider.IS_CELEBRATING` data id (16): the boolean set while a raider celebrates a raid
/// victory, the first `Raider` accessor after `Mob.DATA_MOB_FLAGS_ID` (15). The evoker and vindicator
/// `getArmPose` return `CELEBRATING` while it is true (when not casting / not aggressive).
// Vanilla Shulker.DATA_COLOR_ID (BYTE): `getColor()` returns the dye for 0..=15
// and `null` for the generated default otherwise.
/// Vanilla `Camel.SITDOWN_DURATION_TICKS` (40 ticks = 2.0 s `CAMEL_SIT`): the sit-down
/// window, after which `CAMEL_SIT_POSE` takes over.
const CAMEL_SITDOWN_DURATION_TICKS: i64 = 40;
/// Vanilla `Camel.STANDUP_DURATION_TICKS` (52 ticks = 2.6 s `CAMEL_STANDUP`): the stand-up
/// transition window.
const CAMEL_STANDUP_DURATION_TICKS: i64 = 52;
pub(crate) fn entity_scene_outline_from_world_at_partial_tick(
    world: &WorldStore,
    entity_partial_tick: f32,
) -> Option<SelectionOutline> {
    entity_scene_outline_from_world_at_partial_tick_with_server_details(
        world,
        entity_partial_tick,
        false,
    )
}

pub(crate) fn entity_scene_outline_from_world_at_partial_tick_with_server_details(
    world: &WorldStore,
    entity_partial_tick: f32,
    show_local_server_entity_hit_boxes: bool,
) -> Option<SelectionOutline> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let sources = world.entity_model_sources_at_partial_tick(entity_partial_tick);
    let sources_by_id: HashMap<_, _> = sources
        .iter()
        .map(|source| (source.entity_id, source))
        .collect();
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let mut points = Vec::new();
    let mut text_labels = Vec::new();

    for target in world
        .entity_debug_hitbox_targets_at_partial_tick(entity_partial_tick)
        .into_iter()
    {
        if let Some(parent_id) = world.ender_dragon_part_parent_id(target.entity_id) {
            if local_player_id == Some(parent_id) || camera_entity_id == Some(parent_id) {
                continue;
            }
            boxes.push(entity_debug_hitbox_box_with_color(
                target,
                ENTITY_DRAGON_PART_HITBOX_COLOR,
            ));
            continue;
        };
        if local_player_id == Some(target.entity_id) || camera_entity_id == Some(target.entity_id) {
            continue;
        }
        push_entity_debug_gizmos(
            &mut boxes,
            &mut lines,
            &mut points,
            target,
            sources_by_id.get(&target.entity_id).copied(),
            world,
        );
        if show_local_server_entity_hit_boxes {
            text_labels.push(entity_debug_missing_server_label(target));
        }
    }

    (!boxes.is_empty() || !lines.is_empty() || !points.is_empty() || !text_labels.is_empty()).then(
        || {
            SelectionOutline::from_colored_boxes_lines_points_and_labels(
                boxes,
                lines,
                points,
                text_labels,
            )
        },
    )
}

#[cfg(test)]
pub(crate) fn entity_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
) -> Vec<EntityModelInstance> {
    entity_model_instances_from_world_with_registry_at_partial_tick(
        world,
        item_runtime,
        entity_partial_tick,
        Some(world),
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EntityRenderExtractionObservation {
    pub(crate) candidates: usize,
    pub(crate) survivors: usize,
    pub(crate) projected: usize,
    pub(crate) candidate_nanos: u64,
    pub(crate) cull_nanos: u64,
    pub(crate) projection_nanos: u64,
    pub(crate) model_build_nanos: u64,
    pub(crate) candidate_peak_nanos: u64,
    pub(crate) cull_peak_nanos: u64,
    pub(crate) projection_peak_nanos: u64,
    pub(crate) model_build_peak_nanos: u64,
    pub(crate) capacity_growths: u64,
}

#[derive(Debug, Default)]
pub(crate) struct EntityRenderExtractionScratch {
    candidates: Vec<EntityRenderCandidateState>,
    survivors: Vec<EntityRenderCandidateState>,
    candidate_index: HashMap<i32, usize>,
    sources: Vec<EntityModelSourceState>,
    display_sources: Vec<DisplayEntityRenderSourceState>,
    name_tag_sources: Vec<EntityNameTagRenderSourceState>,
    shadow_sources: Vec<EntityShadowRenderSourceState>,
    instances: Vec<EntityModelInstance>,
    observation: EntityRenderExtractionObservation,
}

impl EntityRenderExtractionScratch {
    pub(crate) fn observation(&self) -> EntityRenderExtractionObservation {
        self.observation
    }

    /// The number of `EntityRenderState` equivalents extracted for the current frame.
    ///
    /// Vanilla records this immediately after `LevelRenderer.extractVisibleEntities`; it is
    /// intentionally the survivor count rather than the number of model instances because one
    /// entity can emit multiple renderer-owned pieces.
    pub(crate) fn visible_entity_render_state_count(&self) -> usize {
        self.survivors.len()
    }

    pub(crate) fn recycle_instances(&mut self, mut instances: Vec<EntityModelInstance>) {
        instances.clear();
        self.instances = instances;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct EntityRenderVisibilityContext<'a> {
    pub(crate) camera_pose: Option<CameraPose>,
    pub(crate) camera_detached: bool,
    pub(crate) surface_size: [u32; 2],
    pub(crate) camera_fov_degrees: f32,
    pub(crate) base_camera_fov_degrees: f32,
    pub(crate) render_distance_chunks: u32,
    pub(crate) entity_distance_scaling: f64,
    pub(crate) terrain_upload: &'a TerrainUploadState,
    pub(crate) entity_shadows: bool,
    pub(crate) shadow_sky_darken: u8,
}

pub(crate) fn visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    textures: &NativeEntityVariantTextureRuntime,
    entity_partial_tick: f32,
    visibility: EntityRenderVisibilityContext<'_>,
    scratch: &mut EntityRenderExtractionScratch,
) -> Vec<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let previous_capacities = [
        scratch.candidates.capacity(),
        scratch.survivors.capacity(),
        scratch.candidate_index.capacity(),
        scratch.sources.capacity(),
        scratch.name_tag_sources.capacity(),
        scratch.shadow_sources.capacity(),
        scratch.instances.capacity(),
    ];

    let candidate_started_at = Instant::now();
    world.fill_entity_render_candidates_at_partial_tick(
        entity_partial_tick,
        &mut scratch.candidates,
    );
    let candidate_duration = candidate_started_at.elapsed();

    let cull_started_at = Instant::now();
    scratch.candidate_index.clear();
    scratch.candidate_index.reserve(scratch.candidates.len());
    scratch.candidate_index.extend(
        scratch
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| (candidate.entity_id, index)),
    );
    scratch.survivors.clear();
    scratch.survivors.reserve(scratch.candidates.len());
    if let Some(camera_pose) = visibility.camera_pose {
        let aspect = visibility.surface_size[0] as f32 / visibility.surface_size[1].max(1) as f32;
        let frustum = CameraFrustum::from_pose(camera_pose, aspect, visibility.camera_fov_degrees);
        let camera_entity_id = world.local_player().camera.entity_id;
        let camera_sleeping = camera_entity_id
            .and_then(|id| scratch.candidate_index.get(&id).copied())
            .is_some_and(|index| scratch.candidates[index].is_sleeping);
        scratch
            .survivors
            .extend(scratch.candidates.iter().copied().filter(|candidate| {
                entity_render_candidate_is_visible(
                    world,
                    *candidate,
                    &scratch.candidates,
                    &scratch.candidate_index,
                    camera_pose,
                    frustum,
                    camera_sleeping,
                    visibility,
                )
            }));
    } else {
        scratch.survivors.extend(
            scratch
                .candidates
                .iter()
                .copied()
                .filter(|candidate| !candidate.is_spectator),
        );
    }
    let cull_duration = cull_started_at.elapsed();

    let projection_started_at = Instant::now();
    world.fill_entity_model_sources_for_candidates_at_partial_tick(
        &scratch.survivors,
        &scratch.candidates,
        entity_partial_tick,
        &mut scratch.sources,
    );
    let projection_duration = projection_started_at.elapsed();
    let projected = scratch.sources.len();
    world.fill_entity_name_tag_render_sources_for_model_sources(
        &scratch.sources,
        &mut scratch.name_tag_sources,
    );
    if let Some(camera_pose) = visibility.camera_pose {
        world.fill_entity_shadow_render_sources_for_model_sources(
            &scratch.survivors,
            &scratch.sources,
            entity_partial_tick,
            bbb_world::EntityVec3 {
                x: f64::from(camera_pose.position[0]),
                y: f64::from(camera_pose.position[1] + camera_pose.eye_height),
                z: f64::from(camera_pose.position[2]),
            },
            visibility.shadow_sky_darken,
            visibility.entity_shadows,
            &mut scratch.shadow_sources,
        );
    } else {
        scratch.shadow_sources.clear();
    }

    let model_started_at = Instant::now();
    let context = EntityVariantRenderContext { world, textures };
    fill_entity_model_instances_from_sources(
        world,
        item_runtime,
        entity_partial_tick,
        Some(&context),
        scratch.sources.drain(..),
        false,
        &mut scratch.instances,
    );
    // Vanilla's local-owner near-plane origin applies only to an attached
    // first-person camera. Detached camera types use the ordinary owner-hand
    // origin already projected into the instance.
    if !visibility.camera_detached {
        if let Some(camera_pose) = visibility.camera_pose {
            for instance in &mut scratch.instances {
                if !matches!(instance.kind, EntityModelKind::FishingHook) {
                    continue;
                }
                let Some(source) = world.fishing_hook_render_source_at_partial_tick(
                    instance.entity_id,
                    entity_partial_tick,
                ) else {
                    continue;
                };
                if Some(source.owner_id) != world.local_player_id() {
                    continue;
                }
                instance.render_state.fishing_line_origin_offset =
                    Some(fishing_hook_first_person_line_origin_offset(
                        source,
                        camera_pose,
                        visibility.surface_size,
                        visibility.base_camera_fov_degrees,
                    ));
            }
        }
    }
    let model_duration = model_started_at.elapsed();
    let current_capacities = [
        scratch.candidates.capacity(),
        scratch.survivors.capacity(),
        scratch.candidate_index.capacity(),
        scratch.sources.capacity(),
        scratch.name_tag_sources.capacity(),
        scratch.shadow_sources.capacity(),
        scratch.instances.capacity(),
    ];
    let capacity_growths = previous_capacities
        .into_iter()
        .zip(current_capacities)
        .filter(|(before, after)| after > before)
        .count() as u64;
    scratch.observation = EntityRenderExtractionObservation {
        candidates: scratch.candidates.len(),
        survivors: scratch.survivors.len(),
        projected,
        candidate_nanos: duration_nanos(candidate_duration),
        cull_nanos: duration_nanos(cull_duration),
        projection_nanos: duration_nanos(projection_duration),
        model_build_nanos: duration_nanos(model_duration),
        candidate_peak_nanos: scratch
            .observation
            .candidate_peak_nanos
            .max(duration_nanos(candidate_duration)),
        cull_peak_nanos: scratch
            .observation
            .cull_peak_nanos
            .max(duration_nanos(cull_duration)),
        projection_peak_nanos: scratch
            .observation
            .projection_peak_nanos
            .max(duration_nanos(projection_duration)),
        model_build_peak_nanos: scratch
            .observation
            .model_build_peak_nanos
            .max(duration_nanos(model_duration)),
        capacity_growths: scratch
            .observation
            .capacity_growths
            .saturating_add(capacity_growths),
    };
    std::mem::take(&mut scratch.instances)
}

/// Returns the generic name-tag facts projected from the exact survivor/model
/// source generation consumed by the preceding entity extraction.
pub(crate) fn visible_entity_name_tag_render_sources(
    scratch: &EntityRenderExtractionScratch,
) -> &[EntityNameTagRenderSourceState] {
    &scratch.name_tag_sources
}

/// Returns dispatcher shadow pieces extracted from the same visible entity generation.
pub(crate) fn visible_entity_shadow_render_sources(
    scratch: &EntityRenderExtractionScratch,
) -> &[EntityShadowRenderSourceState] {
    &scratch.shadow_sources
}

/// Projects the dedicated Display render sources for the exact survivor set
/// produced by the immediately preceding visibility-first entity extraction.
pub(crate) fn visible_display_entity_render_sources_at_partial_tick<'a>(
    world: &WorldStore,
    entity_partial_tick: f32,
    scratch: &'a mut EntityRenderExtractionScratch,
) -> &'a [DisplayEntityRenderSourceState] {
    world.fill_display_entity_render_sources_for_candidates_at_partial_tick(
        &scratch.survivors,
        entity_partial_tick.clamp(0.0, 1.0),
        &mut scratch.display_sources,
    );
    &scratch.display_sources
}

#[cfg(test)]
pub(crate) fn entity_model_instances_from_world_with_variant_textures_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    textures: &NativeEntityVariantTextureRuntime,
    entity_partial_tick: f32,
) -> Vec<EntityModelInstance> {
    let context = EntityVariantRenderContext { world, textures };
    entity_model_instances_from_world_with_registry_at_partial_tick(
        world,
        item_runtime,
        entity_partial_tick,
        Some(&context),
    )
}

#[cfg(test)]
fn entity_model_instances_from_world_with_registry_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Vec<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let sources = world.entity_model_sources_at_partial_tick(entity_partial_tick);
    entity_model_instances_from_sources(
        world,
        item_runtime,
        entity_partial_tick,
        variant_registries,
        sources,
    )
}

#[cfg(test)]
fn entity_model_instances_from_sources(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
    sources: impl IntoIterator<Item = EntityModelSourceState>,
) -> Vec<EntityModelInstance> {
    let mut instances = Vec::new();
    fill_entity_model_instances_from_sources(
        world,
        item_runtime,
        entity_partial_tick,
        variant_registries,
        sources,
        true,
        &mut instances,
    );
    instances
}

fn fill_entity_model_instances_from_sources(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
    sources: impl IntoIterator<Item = EntityModelSourceState>,
    hide_local_and_camera_entities: bool,
    instances: &mut Vec<EntityModelInstance>,
) {
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    instances.clear();
    for source in sources {
        if hide_local_and_camera_entities
            && (local_player_id == Some(source.entity_id)
                || camera_entity_id == Some(source.entity_id))
        {
            continue;
        }
        if let Some(painting_instances) =
            painting_model_instances(&source, world, variant_registries)
        {
            instances.extend(painting_instances);
        } else if let Some(instance) = entity_model_instance(
            source,
            world,
            item_runtime,
            game_time,
            entity_partial_tick,
            variant_registries,
        ) {
            instances.push(instance);
        }
    }
}

fn entity_render_candidate_is_visible(
    world: &WorldStore,
    candidate: EntityRenderCandidateState,
    candidates: &[EntityRenderCandidateState],
    candidate_index: &HashMap<i32, usize>,
    camera_pose: CameraPose,
    frustum: CameraFrustum,
    camera_sleeping: bool,
    visibility: EntityRenderVisibilityContext<'_>,
) -> bool {
    if candidate.is_spectator {
        return false;
    }
    // Vanilla `LevelRenderer.extractVisibleEntities`: dispatcher visibility
    // or the local-player indirect-passenger exception comes first.
    if !candidate_has_local_player_indirect_passenger(
        candidate.entity_id,
        world.local_player_id(),
        candidates,
        candidate_index,
    ) && !entity_renderer_should_render_with_scaling(
        candidate,
        candidates,
        candidate_index,
        camera_pose,
        frustum,
        visibility.render_distance_chunks,
        visibility.entity_distance_scaling,
    ) {
        return false;
    }

    if !visibility
        .terrain_upload
        .is_entity_section_compiled_and_visible(world, candidate.current_position)
    {
        return false;
    }

    let camera_entity_id = world.local_player().camera.entity_id;
    // Vanilla hides an awake camera entity only while the camera is attached.
    // Sleeping living cameras remain visible even in first person.
    if attached_camera_hides_entity(
        camera_entity_id,
        candidate.entity_id,
        visibility.camera_detached,
        camera_sleeping,
    ) {
        return false;
    }
    if world.local_player_id() == Some(candidate.entity_id)
        && camera_entity_id != Some(candidate.entity_id)
    {
        return false;
    }
    true
}

fn attached_camera_hides_entity(
    camera_entity_id: Option<i32>,
    candidate_entity_id: i32,
    camera_detached: bool,
    camera_sleeping: bool,
) -> bool {
    camera_entity_id == Some(candidate_entity_id) && !camera_detached && !camera_sleeping
}

#[cfg(test)]
fn entity_renderer_should_render(
    candidate: EntityRenderCandidateState,
    candidates: &[EntityRenderCandidateState],
    candidate_index: &HashMap<i32, usize>,
    camera_pose: CameraPose,
    frustum: CameraFrustum,
    render_distance_chunks: u32,
) -> bool {
    entity_renderer_should_render_with_scaling(
        candidate,
        candidates,
        candidate_index,
        camera_pose,
        frustum,
        render_distance_chunks,
        1.0,
    )
}

fn entity_renderer_should_render_with_scaling(
    candidate: EntityRenderCandidateState,
    candidates: &[EntityRenderCandidateState],
    candidate_index: &HashMap<i32, usize>,
    camera_pose: CameraPose,
    frustum: CameraFrustum,
    render_distance_chunks: u32,
    entity_distance_scaling: f64,
) -> bool {
    let culling_kind = vanilla_entity_renderer_culling_kind(candidate.entity_type_id);
    if candidate.entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID
        && !candidate
            .fishing_hook_owner_id
            .and_then(|owner_id| candidate_index.get(&owner_id))
            .and_then(|index| candidates.get(*index))
            .is_some_and(|owner| owner.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID)
    {
        return false;
    }
    if entity_passes_vanilla_render_distance(
        candidate,
        camera_pose,
        render_distance_chunks,
        entity_distance_scaling,
    ) {
        if culling_kind == VanillaEntityRendererCullingKind::Display
            && candidate
                .display_culling
                .is_some_and(|display| !display.affected_by_culling())
        {
            return true;
        }
        if culling_kind == VanillaEntityRendererCullingKind::NoFrustum {
            return true;
        }

        let bounds = entity_renderer_culling_bounds(candidate, true);
        if frustum.intersects_aabb(bounds.0, bounds.1) {
            return true;
        }
        if let Some(holder) = candidate
            .leash_holder_id
            .and_then(|holder_id| candidate_index.get(&holder_id))
            .and_then(|index| candidates.get(*index))
            .copied()
        {
            let holder_bounds = entity_renderer_culling_bounds(holder, false);
            if frustum.intersects_aabb(holder_bounds.0, holder_bounds.1)
                || frustum.intersects_aabb(
                    [
                        bounds.0[0].min(holder_bounds.0[0]),
                        bounds.0[1].min(holder_bounds.0[1]),
                        bounds.0[2].min(holder_bounds.0[2]),
                    ],
                    [
                        bounds.1[0].max(holder_bounds.1[0]),
                        bounds.1[1].max(holder_bounds.1[1]),
                        bounds.1[2].max(holder_bounds.1[2]),
                    ],
                )
            {
                return true;
            }
        }
    }

    entity_renderer_special_culling_is_visible(candidate, candidates, candidate_index, frustum)
}

fn entity_renderer_special_culling_is_visible(
    candidate: EntityRenderCandidateState,
    candidates: &[EntityRenderCandidateState],
    candidate_index: &HashMap<i32, usize>,
    frustum: CameraFrustum,
) -> bool {
    match candidate.special_culling {
        EntityRendererSpecialCullingState::Guardian { .. } => {
            guardian_renderer_culling_bounds(candidate, candidates, candidate_index)
                .is_some_and(|(min, max)| frustum.intersects_aabb(min, max))
        }
        EntityRendererSpecialCullingState::EndCrystal { has_beam_target } => has_beam_target,
        EntityRendererSpecialCullingState::Shulker { .. } => {
            shulker_renderer_culling_bounds(candidate)
                .is_some_and(|(min, max)| frustum.intersects_aabb(min, max))
        }
        EntityRendererSpecialCullingState::None => false,
    }
}

fn guardian_renderer_culling_bounds(
    candidate: EntityRenderCandidateState,
    candidates: &[EntityRenderCandidateState],
    candidate_index: &HashMap<i32, usize>,
) -> Option<([f64; 3], [f64; 3])> {
    let EntityRendererSpecialCullingState::Guardian {
        attack_target_id,
        eye_height,
    } = candidate.special_culling
    else {
        return None;
    };
    if attack_target_id == 0 {
        return None;
    }
    let target = candidate_index
        .get(&attack_target_id)
        .and_then(|index| candidates.get(*index))
        .copied()
        .filter(|target| vanilla_entity_type_is_living(target.entity_type_id))?;
    let target_half_height = f64::from(target.bounds.max[1] - target.bounds.min[1]) * 0.5;
    let start = [
        candidate.current_position.x,
        candidate.current_position.y + f64::from(eye_height),
        candidate.current_position.z,
    ];
    let target = [
        target.current_position.x,
        target.current_position.y + target_half_height,
        target.current_position.z,
    ];
    Some(aabb_between(start, target))
}

fn shulker_renderer_culling_bounds(
    candidate: EntityRenderCandidateState,
) -> Option<([f64; 3], [f64; 3])> {
    let EntityRendererSpecialCullingState::Shulker {
        render_offset_at_zero: Some(start),
        half_width,
        half_height,
    } = candidate.special_culling
    else {
        return None;
    };
    let half_width = f64::from(half_width);
    let half_height = f64::from(half_height);
    let start = [start.x, start.y + half_height, start.z];
    let target = [
        candidate.current_position.x.floor() + 0.5,
        candidate.current_position.y.floor() + half_height,
        candidate.current_position.z.floor() + 0.5,
    ];
    let (mut min, mut max) = aabb_between(start, target);
    inflate_aabb(&mut min, &mut max, [half_width, half_height, half_width]);
    Some((min, max))
}

fn aabb_between(start: [f64; 3], end: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    (
        std::array::from_fn(|axis| start[axis].min(end[axis])),
        std::array::from_fn(|axis| start[axis].max(end[axis])),
    )
}

fn candidate_has_local_player_indirect_passenger(
    candidate_id: i32,
    local_player_id: Option<i32>,
    candidates: &[EntityRenderCandidateState],
    candidate_index: &HashMap<i32, usize>,
) -> bool {
    let mut current = local_player_id
        .and_then(|entity_id| candidate_index.get(&entity_id))
        .and_then(|index| candidates.get(*index))
        .and_then(|player| player.vehicle_id);
    for _ in 0..candidates.len() {
        let Some(vehicle_id) = current else {
            return false;
        };
        if vehicle_id == candidate_id {
            return true;
        }
        current = candidate_index
            .get(&vehicle_id)
            .and_then(|index| candidates.get(*index))
            .and_then(|vehicle| vehicle.vehicle_id);
    }
    false
}

fn entity_passes_vanilla_render_distance(
    candidate: EntityRenderCandidateState,
    camera_pose: CameraPose,
    render_distance_chunks: u32,
    entity_distance_scaling: f64,
) -> bool {
    // `LevelRenderer.setupRender` writes the shared `Entity.viewScale` from
    // effective render distance and `Options.entityDistanceScaling` before
    // `Entity.shouldRenderAtSqrDistance` consumes it.
    let view_scale = (f64::from(render_distance_chunks) / 8.0).clamp(1.0, 2.5)
        * entity_distance_scaling.clamp(0.5, 5.0);
    let camera = [
        f64::from(camera_pose.position[0]),
        f64::from(camera_pose.position[1] + camera_pose.eye_height),
        f64::from(camera_pose.position[2]),
    ];
    let dx = candidate.position.x - camera[0];
    let dy = candidate.position.y - camera[1];
    let dz = candidate.position.z - camera[2];
    let distance_sqr = dx * dx + dy * dy + dz * dz;
    if let Some(display) = candidate.display_culling {
        let max_distance = f64::from(display.view_range) * 64.0 * view_scale;
        return distance_sqr < max_distance * max_distance;
    }
    let x_size = f64::from(candidate.bounds.max[0] - candidate.bounds.min[0]);
    let y_size = f64::from(candidate.bounds.max[1] - candidate.bounds.min[1]);
    let z_size = f64::from(candidate.bounds.max[2] - candidate.bounds.min[2]);
    let mut size = (x_size + y_size + z_size) / 3.0;
    if size.is_nan() {
        size = 1.0;
    }
    if candidate.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
        // `RemotePlayer.shouldRenderAtSqrDistance` expands the average AABB
        // size before applying the shared static view scale.
        size *= 10.0;
    }
    let max_distance = size * 64.0 * view_scale;
    distance_sqr < max_distance * max_distance
}

fn entity_renderer_culling_bounds(
    candidate: EntityRenderCandidateState,
    inflate_for_dispatcher: bool,
) -> ([f64; 3], [f64; 3]) {
    let (mut min, mut max) = if let Some(display) = candidate.display_culling {
        let half_width = f64::from(display.width) / 2.0;
        (
            [-half_width, 0.0, -half_width],
            [half_width, f64::from(display.height), half_width],
        )
    } else {
        (
            candidate.bounds.min.map(f64::from),
            candidate.bounds.max.map(f64::from),
        )
    };
    if candidate.wears_dragon_head && vanilla_entity_type_is_living(candidate.entity_type_id) {
        inflate_aabb(&mut min, &mut max, [0.5; 3]);
    }
    match vanilla_entity_renderer_culling_kind(candidate.entity_type_id) {
        VanillaEntityRendererCullingKind::Inflate(inflation) => {
            inflate_aabb(&mut min, &mut max, inflation.map(f64::from));
        }
        VanillaEntityRendererCullingKind::ExtendMinYByHalfHeight => {
            min[1] -= (max[1] - min[1]) * 0.5;
        }
        VanillaEntityRendererCullingKind::Standard
        | VanillaEntityRendererCullingKind::NoFrustum
        | VanillaEntityRendererCullingKind::Display
        | VanillaEntityRendererCullingKind::ConservativeSpecial => {}
    }
    if inflate_for_dispatcher {
        inflate_aabb(&mut min, &mut max, [0.5; 3]);
    }
    let average_size = ((max[0] - min[0]) + (max[1] - min[1]) + (max[2] - min[2])) / 3.0;
    if min.into_iter().chain(max).any(|value| value.is_nan()) || average_size == 0.0 {
        min = [-2.0; 3];
        max = [2.0; 3];
    }
    (
        [
            candidate.position.x + min[0],
            candidate.position.y + min[1],
            candidate.position.z + min[2],
        ],
        [
            candidate.position.x + max[0],
            candidate.position.y + max[1],
            candidate.position.z + max[2],
        ],
    )
}

fn inflate_aabb(min: &mut [f64; 3], max: &mut [f64; 3], inflation: [f64; 3]) {
    for axis in 0..3 {
        min[axis] -= inflation[axis];
        max[axis] += inflation[axis];
    }
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn entity_model_instance_from_world_entity_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    entity_partial_tick: f32,
) -> Option<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let source = world.entity_model_source_at_partial_tick(entity_id, entity_partial_tick)?;
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    entity_model_instance(
        source,
        world,
        item_runtime,
        game_time,
        entity_partial_tick,
        Some(world),
    )
}

pub(crate) fn entity_model_instance_from_world_entity_with_variant_textures_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    textures: &NativeEntityVariantTextureRuntime,
    entity_id: i32,
    entity_partial_tick: f32,
) -> Option<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let source = world.entity_model_source_at_partial_tick(entity_id, entity_partial_tick)?;
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    let context = EntityVariantRenderContext { world, textures };
    entity_model_instance(
        source,
        world,
        item_runtime,
        game_time,
        entity_partial_tick,
        Some(&context),
    )
}

pub(crate) fn entity_model_kind_for_world_entity_type_at_partial_tick(
    world: &WorldStore,
    entity_type_id: i32,
    entity_age_ticks: f32,
) -> EntityModelKind {
    let data_values: &[bbb_protocol::packets::EntityDataValue] = &[];
    entity_model_kind_with_time_and_registries(
        entity_type_id,
        data_values,
        entity_age_ticks,
        world.world_time().map(|time| time.game_time).unwrap_or(0),
        Some(world),
    )
}

fn entity_pick_target_box(target: EntityPickTargetState) -> SelectionBox {
    SelectionBox {
        min: [
            (target.position.x + f64::from(target.bounds.min[0])) as f32,
            (target.position.y + f64::from(target.bounds.min[1])) as f32,
            (target.position.z + f64::from(target.bounds.min[2])) as f32,
        ],
        max: [
            (target.position.x + f64::from(target.bounds.max[0])) as f32,
            (target.position.y + f64::from(target.bounds.max[1])) as f32,
            (target.position.z + f64::from(target.bounds.max[2])) as f32,
        ],
    }
}

fn push_entity_debug_gizmos(
    boxes: &mut Vec<SelectionColoredBox>,
    lines: &mut Vec<SelectionLine>,
    points: &mut Vec<SelectionPoint>,
    target: EntityPickTargetState,
    source: Option<&EntityModelSourceState>,
    world: &WorldStore,
) {
    boxes.push(entity_debug_hitbox_box(target));
    points.push(entity_debug_position_point([
        target.position.x,
        target.position.y,
        target.position.z,
    ]));
    if let Some(vehicle_target) = world.entity_debug_passenger_vehicle_target(target.entity_id) {
        boxes.push(entity_debug_passenger_vehicle_box(vehicle_target));
    }
    let Some(pose) = world.probe_entity_camera_pose(target.entity_id) else {
        return;
    };
    let entity_type_id = source
        .map(|source| source.entity_type_id)
        .or_else(|| world.entity_type_id(target.entity_id));
    if entity_type_id.is_some_and(entity_debug_target_type_is_living) {
        boxes.push(entity_debug_eye_height_box(target, pose.eye_height));
    }
    let y_rot = source.map_or(pose.y_rot, |source| source.y_rot);
    let x_rot = source.map_or(pose.x_rot, |source| source.x_rot);
    lines.push(entity_debug_view_vector_line(
        [target.position.x, target.position.y, target.position.z],
        pose.eye_height,
        y_rot,
        x_rot,
    ));
}

fn entity_debug_target_type_is_living(entity_type_id: i32) -> bool {
    vanilla_entity_type_is_living(entity_type_id)
        || entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
}

fn entity_debug_hitbox_box(target: EntityPickTargetState) -> SelectionColoredBox {
    entity_debug_hitbox_box_with_color(target, ENTITY_HITBOX_COLOR)
}

fn entity_debug_passenger_vehicle_box(target: EntityPickTargetState) -> SelectionColoredBox {
    entity_debug_hitbox_box_with_color(target, ENTITY_PASSENGER_VEHICLE_COLOR)
}

fn entity_debug_missing_server_label(target: EntityPickTargetState) -> SelectionTextLabel {
    SelectionTextLabel {
        position: [
            target.position.x as f32,
            target.position.y as f32 + entity_debug_bounds_height(target) + 1.5,
            target.position.z as f32,
        ],
        text: ENTITY_MISSING_SERVER_LABEL_TEXT.to_string(),
        color: ENTITY_MISSING_SERVER_LABEL_COLOR,
        centered: true,
    }
}

fn entity_debug_bounds_height(target: EntityPickTargetState) -> f32 {
    (target.bounds.max[1] - target.bounds.min[1]).abs()
}

fn entity_debug_hitbox_box_with_color(
    target: EntityPickTargetState,
    color: [f32; 4],
) -> SelectionColoredBox {
    let selection_box = entity_pick_target_box(target);
    SelectionColoredBox {
        min: selection_box.min,
        max: selection_box.max,
        color,
        line_width: 1.0,
        always_on_top: false,
    }
}

fn entity_debug_eye_height_box(
    target: EntityPickTargetState,
    eye_height: f32,
) -> SelectionColoredBox {
    let selection_box = entity_pick_target_box(target);
    let eye_y = selection_box.min[1] + eye_height;
    SelectionColoredBox {
        min: [
            selection_box.min[0],
            eye_y - ENTITY_EYE_HEIGHT_PADDING,
            selection_box.min[2],
        ],
        max: [
            selection_box.max[0],
            eye_y + ENTITY_EYE_HEIGHT_PADDING,
            selection_box.max[2],
        ],
        color: ENTITY_EYE_HEIGHT_COLOR,
        line_width: 1.0,
        always_on_top: false,
    }
}

fn entity_debug_position_point(position: [f64; 3]) -> SelectionPoint {
    SelectionPoint {
        position: [position[0] as f32, position[1] as f32, position[2] as f32],
        color: ENTITY_HITBOX_COLOR,
        size: ENTITY_POSITION_POINT_SIZE,
    }
}

fn entity_debug_view_vector_line(
    position: [f64; 3],
    eye_height: f32,
    y_rot: f32,
    x_rot: f32,
) -> SelectionLine {
    let direction = entity_view_vector(y_rot, x_rot);
    let from = [
        position[0] as f32,
        position[1] as f32 + eye_height,
        position[2] as f32,
    ];
    let to = [
        from[0] + direction[0] * ENTITY_VIEW_VECTOR_LENGTH,
        from[1] + direction[1] * ENTITY_VIEW_VECTOR_LENGTH,
        from[2] + direction[2] * ENTITY_VIEW_VECTOR_LENGTH,
    ];
    SelectionLine {
        from,
        to,
        color: ENTITY_VIEW_VECTOR_COLOR,
        width: 1.0,
    }
}

fn entity_view_vector(y_rot: f32, x_rot: f32) -> [f32; 3] {
    let yaw = y_rot.to_radians();
    let pitch = x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let x = -yaw.sin() * cos_pitch;
    let y = -pitch.sin();
    let z = yaw.cos() * cos_pitch;
    let len = (x * x + y * y + z * z).sqrt();
    if len <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [x / len, y / len, z / len]
    }
}

#[allow(clippy::too_many_arguments)]
/// Appends the pure-passthrough `EntityModelSourceState` -> `EntityRenderState`
/// projections to an [`EntityModelInstance`] builder: each `with_field field`
/// entry expands to `.with_field(source.field)`. Only fields copied verbatim
/// (identical builder and source name, no computation) belong here; every derived
/// projection stays in the hand-written chain above the invocation. `macro_rules!`
/// cannot synthesize the `with_` builder name, so each entry spells it out,
/// mirroring the renderer-side `entity_render_state!` per-field list.
macro_rules! entity_render_state_passthrough {
    (
        $base:expr, $source:expr
        $(, $with:ident $field:ident )* $(,)?
    ) => {
        $base $( .$with($source.$field) )*
    };
}

mod entity_data;
mod hands;
mod instance;
mod kind;
mod painting;
mod variants;

use entity_data::*;
#[cfg(test)]
pub(crate) use hands::PIGLIN_LOVED_ITEM_TAG;
use hands::*;
use instance::*;
use kind::*;
use painting::*;
use variants::*;

pub(crate) fn default_spear_kinetic_weapon_for_resource_id(
    item_runtime: &NativeItemRuntime,
    resource_id: &str,
) -> Option<SpearKineticWeapon> {
    hands::spear_kinetic_weapon_for_resource_id(item_runtime, resource_id)
}

pub(crate) fn player_info_resolvable_profile(
    profile: &bbb_world::PlayerInfoProfileState,
) -> ResolvableProfileSummary {
    variants::player_info_profile_resolvable(profile)
}

#[cfg(test)]
mod tests;
