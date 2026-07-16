use std::collections::HashMap;

use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;
use bbb_item_model::{default_player_skin_for_profile_id, NativeItemRuntime};
use bbb_protocol::entity_types::*;
use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    },
    item_types,
    packets::{
        decode_profile_textures_from_properties, EntityDataEnumSerializer,
        EntityDataRegistryHolder, EntityDataValueKind, EquipmentSlot, GameProfilePropertySummary,
        ItemStackSummary, PlayerSkinPatchSummary, ResolvableProfileKindSummary,
        ResolvableProfileSummary, SwingAnimationTypeSummary,
    },
};
use bbb_renderer::{
    ArmorStandModelPose, ArrowModelTexture, AxolotlModelVariant, BoatModelFamily, CamelModelFamily,
    CatModelVariant, ChickenModelVariant, CopperGolemWeathering, CowModelVariant,
    DonkeyModelFamily, EndCrystalBeamRenderState, EnderDragonBeamRenderState, EntityArmorMaterial,
    EntityAttachmentFace, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDyeColor,
    EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind, EntityDynamicTexture,
    EntityEquipmentLayerTexture, EntityModelInstance, EntityModelKind, EntityPlayerSkin,
    FoxModelVariant, FrogModelVariant, GuardianBeamRenderState, HoglinModelFamily,
    HorseColorVariant, HorseMarkings, IllagerModelFamily, IronGolemCrackiness, LlamaModelFamily,
    LlamaVariant, MooshroomVariant, PandaModelVariant, ParrotModelVariant, PigModelVariant,
    PiglinModelFamily, PlayerModelPartVisibility, RabbitModelVariant, SalmonModelSize,
    SelectionBox, SelectionColoredBox, SelectionLine, SelectionOutline, SelectionPoint,
    SelectionTextLabel, SheepHeadEatPose, SheepWoolColor, SkeletonModelFamily, SleepingPose,
    SpearKineticWeapon, TropicalFishModelShape, TropicalFishPattern, UndeadHorseModelFamily,
    VillagerModelData, VillagerModelProfession, VillagerModelType, WolfArmorCrackiness,
    WolfModelVariant, ZombieNautilusModelVariant, ZombieVariantModelFamily,
    ENTITY_DEFAULT_OUTLINE_COLOR,
};
#[cfg(test)]
use bbb_renderer::{EntityDynamicPlayerSkinStatus, EntityPlayerSkinModel};
#[cfg(test)]
use bbb_world::vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_armadillo_state_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_armor_stand_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_arrow_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type;
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
use bbb_world::WolfVariantAssetPresentation;
#[cfg(test)]
use bbb_world::VANILLA_ARMADILLO_STATE_DATA_ID as ARMADILLO_STATE_DATA_ID;
use bbb_world::VANILLA_ARMADILLO_STATE_SCARED_ID as ARMADILLO_STATE_SCARED_ID;
#[cfg(test)]
use bbb_world::VANILLA_CAT_VARIANT_DATA_ID as CAT_VARIANT_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_ENTITY_CUSTOM_NAME_DATA_ID as ENTITY_CUSTOM_NAME_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_ENTITY_SHARED_FLAGS_DATA_ID as ENTITY_SHARED_FLAGS_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_ENTITY_SHARED_FLAG_GLOWING as ENTITY_SHARED_FLAG_GLOWING;
#[cfg(test)]
use bbb_world::VANILLA_ENTITY_SHARED_FLAG_INVISIBLE as ENTITY_SHARED_FLAG_INVISIBLE;
#[cfg(test)]
use bbb_world::VANILLA_ENTITY_SHARED_FLAG_ON_FIRE as ENTITY_SHARED_FLAG_ON_FIRE;
#[cfg(test)]
use bbb_world::VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID as GLOW_SQUID_DARK_TICKS_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID as HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID as LIVING_ENTITY_HEALTH_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID as SKELETON_STRAY_CONVERSION_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_STRIDER_SUFFOCATING_DATA_ID as STRIDER_SUFFOCATING_DATA_ID;
use bbb_world::VANILLA_VEX_FLAG_CHARGING as VEX_FLAG_IS_CHARGING;
#[cfg(test)]
use bbb_world::VANILLA_VILLAGER_DATA_DATA_ID as VILLAGER_DATA_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID as WITHER_SKULL_DANGEROUS_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_WOLF_VARIANT_DATA_ID as WOLF_VARIANT_DATA_ID;
use bbb_world::{
    vanilla_entity_metadata_creeper_accessors_for_entity_type,
    vanilla_entity_type_illager_arm_pose_uses_celebrating, vanilla_entity_type_is_living,
    ArmorMaterialKind as WorldArmorMaterialKind, CatVariantPresentation,
    EndCrystalBeamSource as WorldEndCrystalBeamSource,
    EnderDragonBeamSource as WorldEnderDragonBeamSource,
    EntityAttachmentFace as WorldEntityAttachmentFace, EntityModelSourceState,
    EntityPickTargetState, FarmAnimalVariantModel, FarmAnimalVariantPresentation,
    FrogVariantPresentation, GuardianBeamSource as WorldGuardianBeamSource,
    LlamaBodyDecorColor as WorldLlamaBodyDecorColor, VanillaArmorStandMetadataAccessors,
    WolfArmorCrackiness as WorldWolfArmorCrackiness, WolfVariantPresentation, WorldStore,
    ZombieNautilusVariantModelType, ZombieNautilusVariantPresentation,
    VANILLA_AGEABLE_MOB_BABY_DATA_ID as AGEABLE_MOB_BABY_DATA_ID,
    VANILLA_AGEABLE_MOB_DEFAULT_BABY as AGEABLE_MOB_DEFAULT_BABY,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER as ARMOR_STAND_CLIENT_FLAG_MARKER,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE as ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS as ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL as ARMOR_STAND_CLIENT_FLAG_SMALL,
    VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION as AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION,
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID as AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
};
use bbb_world::{CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY};
#[cfg(test)]
use bbb_world::{
    VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID as ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID,
    VANILLA_CHICKEN_VARIANT_DATA_ID as CHICKEN_VARIANT_DATA_ID,
    VANILLA_COW_VARIANT_DATA_ID as COW_VARIANT_DATA_ID,
    VANILLA_HORSE_VARIANT_DATA_ID as HORSE_VARIANT_DATA_ID,
    VANILLA_LLAMA_VARIANT_DATA_ID as LLAMA_VARIANT_DATA_ID,
    VANILLA_PIG_VARIANT_DATA_ID as PIG_VARIANT_DATA_ID,
};
#[cfg(test)]
use bbb_world::{
    VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID as COPPER_GOLEM_STATE_GETTING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID as COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_IDLE_ID as COPPER_GOLEM_STATE_IDLE_ID,
};
#[cfg(test)]
pub(crate) use bbb_world::{
    VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID as END_CRYSTAL_BEAM_TARGET_DATA_ID,
    VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID as END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
};
#[cfg(test)]
use bbb_world::{
    VANILLA_GOAT_LEFT_HORN_DATA_ID as GOAT_LEFT_HORN_DATA_ID,
    VANILLA_GOAT_RIGHT_HORN_DATA_ID as GOAT_RIGHT_HORN_DATA_ID,
};
use bbb_world::{
    VANILLA_HORSE_COLOR_MASK as HORSE_COLOR_MASK,
    VANILLA_HORSE_MARKINGS_MASK as HORSE_MARKINGS_MASK,
    VANILLA_HORSE_MARKINGS_SHIFT as HORSE_MARKINGS_SHIFT,
};
use bbb_world::{
    VANILLA_SHEEP_FLAG_SHEARED as SHEEP_WOOL_SHEARED_FLAG,
    VANILLA_SHEEP_WOOL_COLOR_MASK as SHEEP_WOOL_COLOR_MASK,
};
use bbb_world::{
    VANILLA_TROPICAL_FISH_BASE_COLOR_MASK as TROPICAL_FISH_BASE_COLOR_MASK,
    VANILLA_TROPICAL_FISH_BASE_COLOR_SHIFT as TROPICAL_FISH_BASE_COLOR_SHIFT,
    VANILLA_TROPICAL_FISH_PATTERN_COLOR_MASK as TROPICAL_FISH_PATTERN_COLOR_MASK,
    VANILLA_TROPICAL_FISH_PATTERN_COLOR_SHIFT as TROPICAL_FISH_PATTERN_COLOR_SHIFT,
    VANILLA_TROPICAL_FISH_PATTERN_MASK as TROPICAL_FISH_PATTERN_MASK,
};
#[cfg(test)]
use bbb_world::{
    VANILLA_TURTLE_HAS_EGG_DATA_ID as TURTLE_HAS_EGG_DATA_ID,
    VANILLA_TURTLE_LAYING_EGG_DATA_ID as TURTLE_LAYING_EGG_DATA_ID,
};
trait EntityVariantRegistryLookup {
    fn resource_id_by_holder_id(&self, registry: &str, holder_id: i32) -> Option<&str>;

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
}

impl EntityVariantRegistryLookup for WorldStore {
    fn resource_id_by_holder_id(&self, registry: &str, holder_id: i32) -> Option<&str> {
        self.registry_resource_id_by_holder_id(registry, holder_id)
    }
}

struct EntityVariantRenderContext<'a> {
    world: &'a WorldStore,
    textures: &'a NativeEntityVariantTextureRuntime,
}

impl EntityVariantRegistryLookup for EntityVariantRenderContext<'_> {
    fn resource_id_by_holder_id(&self, registry: &str, holder_id: i32) -> Option<&str> {
        self.world
            .registry_resource_id_by_holder_id(registry, holder_id)
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
// Vanilla Shulker.DATA_COLOR_ID (18, BYTE): after Mob.DATA_MOB_FLAGS_ID (15) and the two `Shulker`
// accessors DATA_ATTACH_FACE_ID (16) / DATA_PEEK_ID (17). `getColor()` returns the dye for 0..=15
// and `null` (the default, byte 16) otherwise.
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

fn entity_model_instances_from_world_with_registry_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Vec<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    world
        .entity_model_sources_at_partial_tick(entity_partial_tick)
        .into_iter()
        .filter(|source| {
            local_player_id != Some(source.entity_id) && camera_entity_id != Some(source.entity_id)
        })
        .filter_map(|source| {
            entity_model_instance(
                source,
                world,
                item_runtime,
                game_time,
                entity_partial_tick,
                variant_registries,
            )
        })
        .collect()
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
mod variants;

pub(crate) use entity_data::armor_material;
use entity_data::*;
use hands::*;
use instance::*;
use kind::*;
use variants::*;

pub(crate) fn default_spear_kinetic_weapon_for_resource_id(
    item_runtime: &NativeItemRuntime,
    resource_id: &str,
) -> Option<SpearKineticWeapon> {
    hands::spear_kinetic_weapon_for_resource_id(item_runtime, resource_id)
}

#[cfg(test)]
mod tests;
