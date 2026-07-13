use std::collections::HashMap;

use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    },
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
    EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind, EntityEquipmentLayerTexture,
    EntityModelInstance, EntityModelKind, EntityPlayerSkin, FoxModelVariant, FrogModelVariant,
    GuardianBeamRenderState, HoglinModelFamily, HorseColorVariant, HorseMarkings,
    IllagerModelFamily, IronGolemCrackiness, LlamaModelFamily, LlamaVariant, MooshroomVariant,
    PandaModelVariant, ParrotModelVariant, PigModelVariant, PiglinModelFamily,
    PlayerModelPartVisibility, RabbitModelVariant, SalmonModelSize, SelectionBox,
    SelectionColoredBox, SelectionLine, SelectionOutline, SelectionPoint, SelectionTextLabel,
    SheepHeadEatPose, SheepWoolColor, SkeletonModelFamily, SleepingPose, SpearKineticWeapon,
    TropicalFishModelShape, TropicalFishPattern, UndeadHorseModelFamily, VillagerModelData,
    VillagerModelProfession, VillagerModelType, WolfArmorCrackiness, WolfModelVariant,
    ZombieVariantModelFamily, DEFAULT_ARMOR_STAND_MODEL_POSE, ENTITY_DEFAULT_OUTLINE_COLOR,
};
#[cfg(test)]
use bbb_renderer::{EntityDynamicPlayerSkinStatus, EntityPlayerSkinModel};
#[cfg(test)]
use bbb_world::VANILLA_COPPER_GOLEM_STATE_DATA_ID as COPPER_GOLEM_STATE_DATA_ID;
#[cfg(test)]
use bbb_world::VANILLA_ENTITY_SHARED_FLAG_GLOWING as ENTITY_SHARED_FLAG_GLOWING;
use bbb_world::{
    vanilla_entity_type_is_living, ArmorMaterialKind as WorldArmorMaterialKind,
    EndCrystalBeamSource as WorldEndCrystalBeamSource,
    EnderDragonBeamSource as WorldEnderDragonBeamSource,
    EntityAttachmentFace as WorldEntityAttachmentFace, EntityModelSourceState,
    EntityPickTargetState, GuardianBeamSource as WorldGuardianBeamSource,
    LlamaBodyDecorColor as WorldLlamaBodyDecorColor,
    WolfArmorCrackiness as WorldWolfArmorCrackiness, WorldStore,
    VANILLA_AGEABLE_MOB_BABY_DATA_ID as AGEABLE_MOB_BABY_DATA_ID,
    VANILLA_AGEABLE_MOB_DEFAULT_BABY as AGEABLE_MOB_DEFAULT_BABY,
    VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID as ARMOR_STAND_BODY_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID as ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER as ARMOR_STAND_CLIENT_FLAG_MARKER,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE as ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS as ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL as ARMOR_STAND_CLIENT_FLAG_SMALL,
    VANILLA_ARMOR_STAND_DEFAULT_CLIENT_FLAGS as ARMOR_STAND_DEFAULT_CLIENT_FLAGS,
    VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID as ARMOR_STAND_HEAD_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID as ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID as ARMOR_STAND_LEFT_LEG_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID as ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID as ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
    VANILLA_ARROW_DEFAULT_EFFECT_COLOR as ARROW_DEFAULT_EFFECT_COLOR,
    VANILLA_ARROW_EFFECT_COLOR_DATA_ID as ARROW_EFFECT_COLOR_DATA_ID,
    VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION as AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION,
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID as AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    VANILLA_CREEPER_DEFAULT_POWERED as CREEPER_DEFAULT_POWERED,
    VANILLA_CREEPER_POWERED_DATA_ID as CREEPER_IS_POWERED_DATA_ID,
    VANILLA_ENTITY_CUSTOM_NAME_DATA_ID as ENTITY_CUSTOM_NAME_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID as ENTITY_SHARED_FLAGS_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAG_INVISIBLE as ENTITY_SHARED_FLAG_INVISIBLE,
    VANILLA_ENTITY_SHARED_FLAG_ON_FIRE as ENTITY_SHARED_FLAG_ON_FIRE,
    VANILLA_LIVING_ENTITY_HEALTH_DATA_ID as LIVING_ENTITY_HEALTH_DATA_ID,
    VANILLA_SLIME_DEFAULT_SIZE as SLIME_DEFAULT_SIZE,
    VANILLA_SLIME_SIZE_DATA_ID as SLIME_SIZE_DATA_ID,
    VANILLA_TRIDENT_DEFAULT_FOIL as TRIDENT_DEFAULT_FOIL,
    VANILLA_TRIDENT_FOIL_DATA_ID as TRIDENT_FOIL_DATA_ID,
};
use bbb_world::{
    VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID as ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID,
    VANILLA_ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST as ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST,
    VANILLA_HORSE_COLOR_MASK as HORSE_COLOR_MASK,
    VANILLA_HORSE_DEFAULT_VARIANT as HORSE_DEFAULT_VARIANT,
    VANILLA_HORSE_MARKINGS_MASK as HORSE_MARKINGS_MASK,
    VANILLA_HORSE_MARKINGS_SHIFT as HORSE_MARKINGS_SHIFT,
    VANILLA_HORSE_VARIANT_DATA_ID as HORSE_VARIANT_DATA_ID,
    VANILLA_LLAMA_DEFAULT_VARIANT as LLAMA_DEFAULT_VARIANT,
    VANILLA_LLAMA_VARIANT_DATA_ID as LLAMA_VARIANT_DATA_ID,
};
pub(crate) use bbb_world::{
    VANILLA_ABSTRACT_PIGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION as PIGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION,
    VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID as PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
    VANILLA_PIGLIN_BABY_DATA_ID as PIGLIN_BABY_DATA_ID,
    VANILLA_PIGLIN_CHARGING_CROSSBOW_DATA_ID as PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID,
    VANILLA_PIGLIN_DANCING_DATA_ID as PIGLIN_IS_DANCING_DATA_ID,
    VANILLA_PIGLIN_DEFAULT_BABY as PIGLIN_DEFAULT_BABY,
    VANILLA_PIGLIN_DEFAULT_CHARGING_CROSSBOW as PIGLIN_DEFAULT_CHARGING_CROSSBOW,
    VANILLA_PIGLIN_DEFAULT_DANCING as PIGLIN_DEFAULT_DANCING,
};
use bbb_world::{
    VANILLA_ARMADILLO_STATE_DATA_ID as ARMADILLO_STATE_DATA_ID,
    VANILLA_ARMADILLO_STATE_SCARED_ID as ARMADILLO_STATE_SCARED_ID,
};
use bbb_world::{
    VANILLA_AXOLOTL_DEFAULT_VARIANT as AXOLOTL_DEFAULT_VARIANT,
    VANILLA_AXOLOTL_VARIANT_DATA_ID as AXOLOTL_VARIANT_DATA_ID,
};
use bbb_world::{
    VANILLA_BEE_ANGER_END_TIME_DATA_ID as BEE_ANGER_END_TIME_DATA_ID,
    VANILLA_BEE_DEFAULT_ANGER_END_TIME as BEE_DEFAULT_ANGER_END_TIME,
    VANILLA_BEE_DEFAULT_FLAGS as BEE_DEFAULT_FLAGS, VANILLA_BEE_FLAGS_DATA_ID as BEE_FLAGS_DATA_ID,
    VANILLA_BEE_FLAG_HAS_NECTAR as BEE_FLAG_HAS_NECTAR,
};
pub(crate) use bbb_world::{
    VANILLA_BOGGED_DEFAULT_SHEARED as BOGGED_DEFAULT_SHEARED,
    VANILLA_BOGGED_SHEARED_DATA_ID as BOGGED_SHEARED_DATA_ID,
};
use bbb_world::{
    VANILLA_CAMEL_DEFAULT_LAST_POSE_CHANGE_TICK as CAMEL_DEFAULT_LAST_POSE_CHANGE_TICK,
    VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID as CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
};
use bbb_world::{
    VANILLA_CAT_COLLAR_COLOR_DATA_ID as CAT_COLLAR_COLOR_DATA_ID,
    VANILLA_CAT_DEFAULT_COLLAR_COLOR as CAT_DEFAULT_COLLAR_COLOR_ID,
    VANILLA_CAT_VARIANT_DATA_ID as CAT_VARIANT_DATA_ID,
};
use bbb_world::{
    VANILLA_CHICKEN_DEFAULT_VARIANT_RESOURCE_ID as CHICKEN_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_CHICKEN_VARIANT_DATA_ID as CHICKEN_VARIANT_DATA_ID,
};
use bbb_world::{
    VANILLA_COPPER_GOLEM_DEFAULT_WEATHER_STATE as COPPER_GOLEM_DEFAULT_WEATHER_STATE,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID as COPPER_GOLEM_WEATHER_STATE_DATA_ID,
};
use bbb_world::{
    VANILLA_COW_DEFAULT_VARIANT_RESOURCE_ID as COW_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_COW_VARIANT_DATA_ID as COW_VARIANT_DATA_ID,
    VANILLA_MUSHROOM_COW_DEFAULT_TYPE as MUSHROOM_COW_DEFAULT_TYPE,
    VANILLA_MUSHROOM_COW_TYPE_DATA_ID as MUSHROOM_COW_TYPE_DATA_ID,
};
use bbb_world::{
    VANILLA_CREAKING_ACTIVE_DATA_ID as CREAKING_IS_ACTIVE_DATA_ID,
    VANILLA_CREAKING_DEFAULT_ACTIVE as CREAKING_DEFAULT_ACTIVE,
};
pub(crate) use bbb_world::{
    VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID as END_CRYSTAL_BEAM_TARGET_DATA_ID,
    VANILLA_END_CRYSTAL_DEFAULT_SHOW_BOTTOM as END_CRYSTAL_DEFAULT_SHOW_BOTTOM,
    VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID as END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
};
use bbb_world::{
    VANILLA_FOX_DEFAULT_TYPE as FOX_DEFAULT_VARIANT, VANILLA_FOX_TYPE_DATA_ID as FOX_TYPE_DATA_ID,
};
use bbb_world::{
    VANILLA_FROG_DEFAULT_VARIANT_RESOURCE_ID as FROG_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_FROG_VARIANT_DATA_ID as FROG_VARIANT_DATA_ID,
};
use bbb_world::{
    VANILLA_GHAST_CHARGING_DATA_ID as GHAST_IS_CHARGING_DATA_ID,
    VANILLA_GHAST_DEFAULT_CHARGING as GHAST_DEFAULT_CHARGING,
};
use bbb_world::{
    VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID as GLOW_SQUID_DARK_TICKS_DATA_ID,
    VANILLA_GLOW_SQUID_DEFAULT_DARK_TICKS as GLOW_SQUID_DEFAULT_DARK_TICKS,
};
use bbb_world::{
    VANILLA_GOAT_DEFAULT_HAS_LEFT_HORN as GOAT_DEFAULT_HAS_LEFT_HORN,
    VANILLA_GOAT_DEFAULT_HAS_RIGHT_HORN as GOAT_DEFAULT_HAS_RIGHT_HORN,
    VANILLA_GOAT_LEFT_HORN_DATA_ID as GOAT_LEFT_HORN_DATA_ID,
    VANILLA_GOAT_RIGHT_HORN_DATA_ID as GOAT_RIGHT_HORN_DATA_ID,
};
pub(crate) use bbb_world::{
    VANILLA_HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION as HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION,
    VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID as HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
};
use bbb_world::{
    VANILLA_PANDA_DEFAULT_EAT_COUNTER as PANDA_DEFAULT_EAT_COUNTER,
    VANILLA_PANDA_DEFAULT_FLAGS as PANDA_DEFAULT_FLAGS,
    VANILLA_PANDA_DEFAULT_HIDDEN_GENE as PANDA_DEFAULT_HIDDEN_GENE,
    VANILLA_PANDA_DEFAULT_MAIN_GENE as PANDA_DEFAULT_MAIN_GENE,
    VANILLA_PANDA_DEFAULT_SNEEZE_COUNTER as PANDA_DEFAULT_SNEEZE_COUNTER,
    VANILLA_PANDA_DEFAULT_UNHAPPY_COUNTER as PANDA_DEFAULT_UNHAPPY_COUNTER,
    VANILLA_PANDA_EAT_COUNTER_DATA_ID as PANDA_EAT_COUNTER_DATA_ID,
    VANILLA_PANDA_FLAGS_DATA_ID as PANDA_FLAGS_DATA_ID,
    VANILLA_PANDA_FLAG_SITTING as PANDA_SITTING_FLAG,
    VANILLA_PANDA_FLAG_SNEEZING as PANDA_SNEEZING_FLAG,
    VANILLA_PANDA_HIDDEN_GENE_DATA_ID as PANDA_HIDDEN_GENE_DATA_ID,
    VANILLA_PANDA_MAIN_GENE_DATA_ID as PANDA_MAIN_GENE_DATA_ID,
    VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID as PANDA_SNEEZE_COUNTER_DATA_ID,
    VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID as PANDA_UNHAPPY_COUNTER_DATA_ID,
};
#[cfg(test)]
use bbb_world::{
    VANILLA_PANDA_FLAG_ON_BACK as PANDA_ON_BACK_FLAG,
    VANILLA_PANDA_FLAG_ROLLING as PANDA_ROLLING_FLAG,
};
use bbb_world::{
    VANILLA_PARROT_DEFAULT_VARIANT as PARROT_DEFAULT_VARIANT,
    VANILLA_PARROT_VARIANT_DATA_ID as PARROT_VARIANT_DATA_ID,
};
pub(crate) use bbb_world::{
    VANILLA_PHANTOM_DEFAULT_SIZE as PHANTOM_DEFAULT_SIZE,
    VANILLA_PHANTOM_SIZE_DATA_ID as PHANTOM_SIZE_DATA_ID,
};
use bbb_world::{
    VANILLA_PIG_DEFAULT_VARIANT_RESOURCE_ID as PIG_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_PIG_VARIANT_DATA_ID as PIG_VARIANT_DATA_ID,
};
use bbb_world::{
    VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID as PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID,
    VANILLA_PILLAGER_DEFAULT_CHARGING_CROSSBOW as PILLAGER_DEFAULT_CHARGING_CROSSBOW,
    VANILLA_RAIDER_CELEBRATING_DATA_ID as RAIDER_IS_CELEBRATING_DATA_ID,
    VANILLA_RAIDER_DEFAULT_CELEBRATING as RAIDER_DEFAULT_CELEBRATING,
};
pub(crate) use bbb_world::{
    VANILLA_PUFFERFISH_DEFAULT_PUFF_STATE as PUFFERFISH_DEFAULT_PUFF_STATE,
    VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID as PUFFERFISH_PUFF_STATE_DATA_ID,
};
use bbb_world::{
    VANILLA_RABBIT_DEFAULT_TYPE as RABBIT_DEFAULT_TYPE,
    VANILLA_RABBIT_TYPE_DATA_ID as RABBIT_TYPE_DATA_ID,
    VANILLA_SALMON_DEFAULT_VARIANT as SALMON_DEFAULT_VARIANT,
    VANILLA_SALMON_VARIANT_DATA_ID as SALMON_VARIANT_DATA_ID,
    VANILLA_TROPICAL_FISH_BASE_COLOR_MASK as TROPICAL_FISH_BASE_COLOR_MASK,
    VANILLA_TROPICAL_FISH_BASE_COLOR_SHIFT as TROPICAL_FISH_BASE_COLOR_SHIFT,
    VANILLA_TROPICAL_FISH_DEFAULT_VARIANT as TROPICAL_FISH_DEFAULT_VARIANT,
    VANILLA_TROPICAL_FISH_PATTERN_COLOR_MASK as TROPICAL_FISH_PATTERN_COLOR_MASK,
    VANILLA_TROPICAL_FISH_PATTERN_COLOR_SHIFT as TROPICAL_FISH_PATTERN_COLOR_SHIFT,
    VANILLA_TROPICAL_FISH_PATTERN_MASK as TROPICAL_FISH_PATTERN_MASK,
    VANILLA_TROPICAL_FISH_VARIANT_DATA_ID as TROPICAL_FISH_VARIANT_DATA_ID,
};
use bbb_world::{
    VANILLA_SHEEP_DEFAULT_WOOL as SHEEP_DEFAULT_WOOL,
    VANILLA_SHEEP_FLAG_SHEARED as SHEEP_WOOL_SHEARED_FLAG,
    VANILLA_SHEEP_WOOL_COLOR_MASK as SHEEP_WOOL_COLOR_MASK,
    VANILLA_SHEEP_WOOL_DATA_ID as SHEEP_WOOL_DATA_ID,
};
use bbb_world::{
    VANILLA_SHULKER_COLOR_DATA_ID as SHULKER_COLOR_DATA_ID,
    VANILLA_SHULKER_DEFAULT_COLOR as SHULKER_DEFAULT_COLOR,
};
use bbb_world::{
    VANILLA_SKELETON_DEFAULT_STRAY_CONVERSION as SKELETON_DEFAULT_STRAY_CONVERSION,
    VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID as SKELETON_STRAY_CONVERSION_DATA_ID,
};
use bbb_world::{
    VANILLA_SPELLCASTER_ILLAGER_CASTING_DATA_ID as SPELLCASTER_ILLAGER_CASTING_DATA_ID,
    VANILLA_SPELLCASTER_ILLAGER_DEFAULT_CASTING as SPELLCASTER_ILLAGER_DEFAULT_CASTING,
};
use bbb_world::{
    VANILLA_STRIDER_DEFAULT_SUFFOCATING as STRIDER_DEFAULT_SUFFOCATING,
    VANILLA_STRIDER_SUFFOCATING_DATA_ID as STRIDER_SUFFOCATING_DATA_ID,
};
use bbb_world::{
    VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS as TAMABLE_ANIMAL_DEFAULT_FLAGS,
    VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID as TAMABLE_ANIMAL_FLAGS_DATA_ID,
    VANILLA_TAMABLE_ANIMAL_FLAG_SITTING as TAMABLE_ANIMAL_SITTING_FLAG,
    VANILLA_TAMABLE_ANIMAL_FLAG_TAME as TAMABLE_ANIMAL_TAME_FLAG,
};
use bbb_world::{
    VANILLA_TURTLE_DEFAULT_HAS_EGG as TURTLE_DEFAULT_HAS_EGG,
    VANILLA_TURTLE_DEFAULT_LAYING_EGG as TURTLE_DEFAULT_LAYING_EGG,
    VANILLA_TURTLE_HAS_EGG_DATA_ID as TURTLE_HAS_EGG_DATA_ID,
    VANILLA_TURTLE_LAYING_EGG_DATA_ID as TURTLE_LAYING_EGG_DATA_ID,
};
use bbb_world::{
    VANILLA_VEX_DEFAULT_FLAGS as VEX_DEFAULT_FLAGS, VANILLA_VEX_FLAGS_DATA_ID as VEX_FLAGS_DATA_ID,
    VANILLA_VEX_FLAG_CHARGING as VEX_FLAG_IS_CHARGING,
};
use bbb_world::{
    VANILLA_VILLAGER_DATA_DATA_ID as VILLAGER_DATA_DATA_ID,
    VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID as ZOMBIE_VILLAGER_CONVERTING_DATA_ID,
    VANILLA_ZOMBIE_VILLAGER_DATA_DATA_ID as ZOMBIE_VILLAGER_DATA_DATA_ID,
};
pub(crate) use bbb_world::{
    VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID as WITHER_SKULL_DANGEROUS_DATA_ID,
    VANILLA_WITHER_SKULL_DEFAULT_DANGEROUS as WITHER_SKULL_DEFAULT_DANGEROUS,
};
use bbb_world::{
    VANILLA_WOLF_ANGER_END_TIME_DATA_ID as WOLF_ANGER_END_TIME_DATA_ID,
    VANILLA_WOLF_COLLAR_COLOR_DATA_ID as WOLF_COLLAR_COLOR_DATA_ID,
    VANILLA_WOLF_DEFAULT_ANGER_END_TIME as WOLF_DEFAULT_ANGER_END_TIME,
    VANILLA_WOLF_DEFAULT_COLLAR_COLOR as WOLF_DEFAULT_COLLAR_COLOR_ID,
    VANILLA_WOLF_VARIANT_DATA_ID as WOLF_VARIANT_DATA_ID,
};
use bbb_world::{
    VANILLA_ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID as ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID as ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
};
// Vanilla `Zombie.DATA_DROWNED_CONVERSION_ID`: `Entity` 0..=7, `LivingEntity` 8..=14,
// `Mob` 15, then Zombie baby/special-type/drowned-conversion at 16..=18.
use bbb_world::{
    VANILLA_ZOMBIE_BABY_DATA_ID as ZOMBIE_BABY_DATA_ID,
    VANILLA_ZOMBIE_DEFAULT_BABY as ZOMBIE_DEFAULT_BABY,
    VANILLA_ZOMBIE_DEFAULT_DROWNED_CONVERSION as ZOMBIE_DEFAULT_DROWNED_CONVERSION,
    VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID as ZOMBIE_DROWNED_CONVERSION_DATA_ID,
};

use bbb_item_model::{
    default_player_skin_for_profile_id, vanilla_chicken_variant_resource_id,
    vanilla_cow_variant_resource_id, vanilla_frog_variant_resource_id,
    vanilla_pig_variant_resource_id, vanilla_zombie_nautilus_variant_resource_id,
    NativeItemRuntime,
};
use bbb_protocol::entity_types::*;

trait EntityVariantRegistryLookup {
    fn resource_id_by_holder_id(&self, registry: &str, holder_id: i32) -> Option<&str>;
}

impl EntityVariantRegistryLookup for WorldStore {
    fn resource_id_by_holder_id(&self, registry: &str, holder_id: i32) -> Option<&str> {
        self.registry_resource_id_by_holder_id(registry, holder_id)
    }
}

/// The thrown-item projectiles whose vanilla `ThrownItemRenderer` draws a camera-facing item sprite,
/// paired with that renderer's sprite scale (`poseStack.scale(scale)`). They render through the
/// item-entity billboard layer rather than the 3D model scene (see
/// [`crate::item_entities::item_entity_billboards_from_world`]). Most are unit scale; the two fireballs
/// are the only non-unit `ThrownItemRenderer` registrations (`fireball` ×3.0, `small_fireball` ×0.75).
pub(crate) const THROWN_ITEM_PROJECTILE_BILLBOARDS: &[(i32, f32)] = &[
    (VANILLA_ENTITY_TYPE_EGG_ID, 1.0),
    (VANILLA_ENTITY_TYPE_ENDER_PEARL_ID, 1.0),
    (VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, 1.0),
    (VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, 1.0),
    (VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, 1.0),
    (VANILLA_ENTITY_TYPE_LINGERING_POTION_ID, 1.0),
    (VANILLA_ENTITY_TYPE_SNOWBALL_ID, 1.0),
    (VANILLA_ENTITY_TYPE_FIREBALL_ID, 3.0),
    (VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, 0.75),
];
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
// Vanilla `AbstractArrow.IN_GROUND` (10, BOOLEAN): updating it to true after the first client tick
// starts the seven-tick `shakeTime` impact wobble that `ArrowRenderer.extractRenderState` projects.
#[cfg(test)]
const ARROW_IN_GROUND_DATA_ID: u8 = 10;
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

pub(crate) fn entity_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
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
                Some(world),
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
