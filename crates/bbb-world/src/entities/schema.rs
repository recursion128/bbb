//! Vanilla 26.1 entity metadata semantic facade.
//!
//! Mechanical class/accessor/default/flag/type facts live in the checked-in
//! generated child module. This facade owns the stable Rust value types,
//! compatibility queries, and virtual-default resolution used by world/native
//! consumers. Ordinary builds never read the external vanilla source tree.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityMetadataFamily {
    Entity,
    LivingEntity,
    Mob,
    Allay,
    Creeper,
    Warden,
    WitherBoss,
    EnderMan,
    Slime,
    Shulker,
    Zombie,
    AgeableMob,
    Armadillo,
    Axolotl,
    Bee,
    Dolphin,
    GlowSquid,
    HappyGhast,
    Goat,
    Fox,
    CopperGolem,
    Ocelot,
    Panda,
    PolarBear,
    Sheep,
    Sniffer,
    Turtle,
    AbstractHorse,
    Camel,
    Horse,
    AbstractChestedHorse,
    Llama,
    TamableAnimal,
    Cat,
    Wolf,
    Chicken,
    Cow,
    MushroomCow,
    Pig,
    Frog,
    AbstractNautilus,
    ZombieNautilus,
    Parrot,
    Rabbit,
    AbstractVillager,
    ArmorStand,
    AbstractArrow,
    Arrow,
    ThrownTrident,
    WitherSkull,
    EndCrystal,
    EnderDragon,
    Interaction,
    ExperienceOrb,
    ItemEntity,
    PrimedTnt,
    FireworkRocket,
    FishingHook,
    OminousItemSpawner,
    Guardian,
    SnowGolem,
    IronGolem,
    Bat,
    Phantom,
    Bogged,
    Skeleton,
    Ghast,
    Vex,
    Creaking,
    Strider,
    Zoglin,
    Hoglin,
    AbstractPiglin,
    Piglin,
    Raider,
    Pillager,
    SpellcasterIllager,
    AbstractFish,
    Salmon,
    TropicalFish,
    Pufferfish,
    Tadpole,
    VehicleEntity,
    AbstractBoat,
    AbstractMinecart,
    MinecartFurnace,
    HangingEntity,
    ItemFrame,
    Painting,
    Avatar,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VanillaEntityMetadataDefault {
    Byte(i8),
    Int(i32),
    Long(i64),
    Float(f32),
    Boolean(bool),
    OptionalComponentNone,
    ParticlesEmpty,
    Pose(i32),
    OptionalBlockPosNone,
    OptionalUnsignedIntNone,
    HumanoidArm(i32),
    Rotations([f32; 3]),
    Direction(i32),
    ItemStackEmpty,
    ItemStackDefaultItem(&'static str),
    RegistryHolderResource(&'static str),
    /// Virtual default: vanilla selects an arbitrary existing holder from this registry.
    /// This does not identify a concrete resource id and must not be used as one.
    RegistryAny(&'static str),
    BlockStateDefault {
        resource_id: &'static str,
        properties: &'static [(&'static str, &'static str)],
    },
    OptionalBlockStateNone,
    OptionalLivingEntityReferenceNone,
    MinecartDefaultDisplayOffset,
    MaxAirSupply,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaEntityMetadataFieldSchema {
    pub name: &'static str,
    pub data_id: u8,
    pub serializer_id: i32,
    pub default: VanillaEntityMetadataDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityMetadataFlagSchema {
    pub name: &'static str,
    pub data_id: u8,
    pub mask: i8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaEntityMetadataFamilySchema {
    pub family: VanillaEntityMetadataFamily,
    pub parent: Option<VanillaEntityMetadataFamily>,
    pub inherited_field_count: u8,
    pub own_fields: &'static [VanillaEntityMetadataFieldSchema],
    pub own_flags: &'static [VanillaEntityMetadataFlagSchema],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityMetadataClassCatalogEntry {
    pub java_class: &'static str,
    pub superclass: Option<&'static str>,
    pub interfaces: &'static [&'static str],
    pub source_path: &'static str,
    pub kind: &'static str,
    pub inherited_accessor_count: u16,
    pub own_accessor_count: u16,
}

/// One vanilla class declaration of `handleEntityEvent(byte)` and the fixed event ids it owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityEventHandlerCatalogEntry {
    pub owner: &'static str,
    pub super_handler_owner: Option<&'static str>,
    pub source_path: &'static str,
    pub direct_event_ids: &'static [u8],
    pub delegated_event_ids: &'static [u8],
    pub delegates_to: Option<&'static str>,
    pub forwards_unhandled_to_super: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VanillaEnderDragonPhaseCatalogEntry {
    pub phase_id: i32,
    pub java_field_name: &'static str,
    pub implementation_class: &'static str,
    pub is_sitting: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaEntityMetadataAccessorCatalogEntry {
    pub owner: &'static str,
    pub java_name: &'static str,
    pub serializer_name: &'static str,
    pub serializer_id: i32,
    pub declaration_order: u16,
    pub data_id: u8,
    pub source_path: &'static str,
    pub default_expression: &'static str,
    pub typed_default: Option<VanillaEntityMetadataDefault>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityMetadataFlagStatus {
    Resolved,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityMetadataFlagCatalogEntry {
    pub owner: &'static str,
    pub java_name: &'static str,
    pub source_expression: &'static str,
    pub value: Option<i32>,
    pub accessor_name: Option<&'static str>,
    pub status: VanillaEntityMetadataFlagStatus,
    pub diagnostic: Option<&'static str>,
    pub source_path: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityMetadataResolverEvidence {
    pub java_class: &'static str,
    pub accessor_name: &'static str,
    pub evidence_member: &'static str,
    pub source_expression: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntitySoundSourceKind {
    Neutral,
    Hostile,
    Ambient,
    Weather,
    Players,
    Dynamic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntitySoundVariantFamily {
    Cat,
    Chicken,
    Cow,
    Pig,
    Wolf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntitySoundCondition {
    ArmadilloScared,
    InWater,
    UnderWater,
    Baby,
    GoatScreaming,
    Tiny,
    ShulkerClosed,
    CreakingHeartBound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityHurtSoundPolicy {
    NonLiving,
    Fixed(&'static str),
    Conditional {
        condition: VanillaEntitySoundCondition,
        when_true: &'static str,
        when_false: &'static str,
    },
    NautilusAgeWater {
        baby_underwater: &'static str,
        baby_land: &'static str,
        adult_underwater: &'static str,
        adult_land: &'static str,
    },
    CopperGolemOxidation {
        unaffected: &'static str,
        exposed: &'static str,
        weathered: &'static str,
        oxidized: &'static str,
    },
    SoundVariant(VanillaEntitySoundVariantFamily),
    WolfVariant {
        armor_damage: &'static str,
    },
    PlayerDamageType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityDeathSoundPolicy {
    NonLiving,
    Fixed(&'static str),
    Conditional {
        condition: VanillaEntitySoundCondition,
        when_true: &'static str,
        when_false: &'static str,
    },
    NautilusAgeWater {
        baby_underwater: &'static str,
        baby_land: &'static str,
        adult_underwater: &'static str,
        adult_land: &'static str,
    },
    CopperGolemOxidation {
        unaffected: &'static str,
        exposed: &'static str,
        weathered: &'static str,
        oxidized: &'static str,
    },
    SoundVariant(VanillaEntitySoundVariantFamily),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntitySoundVolumePolicy {
    NonLiving,
    Fixed(u32),
    SlimeSize,
    HappyGhastAge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityPushabilityKind {
    Never,
    Always,
    Living,
    NotVehicle,
    LivingCanMove,
    LivingNotDiggingOrEmerging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityCollidabilityKind {
    Never,
    Always,
    Alive,
    HappyGhast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VanillaEntityScaleSanitizer {
    ClampMax(f32),
    Fixed(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEndermanRenderMetadataAccessors {
    pub carried_block_state_data_id: u8,
    pub carried_block_state_default: Option<i32>,
    pub creepy_data_id: u8,
    pub creepy_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEndCrystalMetadataAccessors {
    pub beam_target_data_id: u8,
    pub show_bottom_data_id: u8,
    pub show_bottom_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaGoatMetadataAccessors {
    pub screaming_data_id: u8,
    pub screaming_default: bool,
    pub left_horn_data_id: u8,
    pub left_horn_default: bool,
    pub right_horn_data_id: u8,
    pub right_horn_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaPandaMetadataAccessors {
    pub unhappy_counter_data_id: u8,
    pub unhappy_counter_default: i32,
    pub sneeze_counter_data_id: u8,
    pub sneeze_counter_default: i32,
    pub eat_counter_data_id: u8,
    pub eat_counter_default: i32,
    pub main_gene_data_id: u8,
    pub main_gene_default: i8,
    pub hidden_gene_data_id: u8,
    pub hidden_gene_default: i8,
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub sneezing_mask: i8,
    pub rolling_mask: i8,
    pub sitting_mask: i8,
    pub on_back_mask: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractHorseMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub tame_mask: i8,
    pub bred_mask: i8,
    pub eating_mask: i8,
    pub standing_mask: i8,
    pub open_mouth_mask: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaTamableAnimalMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub sitting_mask: i8,
    pub tame_mask: i8,
    pub owner_data_id: u8,
    pub owner_default: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaLlamaMetadataAccessors {
    pub strength_data_id: u8,
    pub strength_default: i32,
    pub variant_data_id: u8,
    pub variant_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaFarmAnimalVariantMetadataAccessors {
    pub variant_data_id: u8,
    pub variant_default_resource_id: &'static str,
    pub sound_variant_data_id: u8,
    pub sound_variant_default_resource_id: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCompanionVariantMetadataAccessors {
    pub variant_data_id: u8,
    pub variant_default_resource_id: &'static str,
    pub sound_variant_data_id: u8,
    pub sound_variant_default_resource_id: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCatMetadataAccessors {
    pub lying_data_id: u8,
    pub lying_default: bool,
    pub relax_state_one_data_id: u8,
    pub relax_state_one_default: bool,
    pub collar_color_data_id: u8,
    pub collar_color_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaBeeMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub roll_mask: i8,
    pub has_stung_mask: i8,
    pub has_nectar_mask: i8,
    pub anger_end_time_data_id: u8,
    pub anger_end_time_default: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaDolphinMetadataAccessors {
    pub got_fish_data_id: u8,
    pub got_fish_default: bool,
    pub moistness_data_id: u8,
    pub moistness_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaFoxMetadataAccessors {
    pub type_data_id: u8,
    pub type_default: i32,
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub sitting_mask: i8,
    pub crouching_mask: i8,
    pub interested_mask: i8,
    pub pouncing_mask: i8,
    pub sleeping_mask: i8,
    pub faceplanted_mask: i8,
    pub defending_mask: i8,
    pub trusted_data_ids: [u8; 2],
    pub trusted_default: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaWolfMetadataAccessors {
    pub interested_data_id: u8,
    pub interested_default: bool,
    pub collar_color_data_id: u8,
    pub collar_color_default: i32,
    pub anger_end_time_data_id: u8,
    pub anger_end_time_default: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaTropicalFishVariantAccessors {
    pub variant_data_id: u8,
    pub variant_default: i32,
    pub pattern_mask: i32,
    pub base_color_mask: i32,
    pub base_color_shift: i32,
    pub pattern_color_mask: i32,
    pub pattern_color_shift: i32,
}

impl VanillaTropicalFishVariantAccessors {
    pub fn pattern_id(self, packed: i32) -> i32 {
        packed & self.pattern_mask
    }

    pub fn base_color_id(self, packed: i32) -> i32 {
        (packed >> self.base_color_shift) & self.base_color_mask
    }

    pub fn pattern_color_id(self, packed: i32) -> i32 {
        (packed >> self.pattern_color_shift) & self.pattern_color_mask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaMobMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub no_ai_mask: i8,
    pub left_handed_mask: i8,
    pub aggressive_mask: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityMetadataAccessors {
    pub shared_flags_data_id: u8,
    pub shared_flags_default: i8,
    pub on_fire_mask: i8,
    pub shift_key_down_mask: i8,
    pub sprinting_mask: i8,
    pub swimming_mask: i8,
    pub invisible_mask: i8,
    pub glowing_mask: i8,
    pub fall_flying_mask: i8,
    pub air_supply_data_id: u8,
    pub custom_name_data_id: u8,
    pub custom_name_visible_data_id: u8,
    pub custom_name_visible_default: bool,
    pub silent_data_id: u8,
    pub silent_default: bool,
    pub no_gravity_data_id: u8,
    pub no_gravity_default: bool,
    pub pose_data_id: u8,
    pub pose_default: i32,
    pub ticks_frozen_data_id: u8,
    pub ticks_frozen_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaLivingEntityMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub is_using_mask: i8,
    pub off_hand_mask: i8,
    pub spin_attack_mask: i8,
    pub health_data_id: u8,
    pub health_default: f32,
    pub arrow_count_data_id: u8,
    pub arrow_count_default: i32,
    pub stinger_count_data_id: u8,
    pub stinger_count_default: i32,
    pub sleeping_pos_data_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaTurtleMetadataAccessors {
    pub has_egg_data_id: u8,
    pub has_egg_default: bool,
    pub laying_egg_data_id: u8,
    pub laying_egg_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaHangingEntityMetadataAccessor {
    pub direction_data_id: u8,
    pub direction_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaItemFrameMetadataAccessors {
    pub direction_data_id: u8,
    pub direction_default: i32,
    pub item_data_id: u8,
    pub rotation_data_id: u8,
    pub rotation_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractBoatMetadataAccessors {
    pub paddle_left_data_id: u8,
    pub paddle_left_default: bool,
    pub paddle_right_data_id: u8,
    pub paddle_right_default: bool,
    pub bubble_time_data_id: u8,
    pub bubble_time_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaVehicleMetadataAccessors {
    pub hurt_time_data_id: u8,
    pub hurt_time_default: i32,
    pub hurt_dir_data_id: u8,
    pub hurt_dir_default: i32,
    pub damage_data_id: u8,
    pub damage_default: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaPrimedTntMetadataAccessors {
    pub fuse_data_id: u8,
    pub fuse_default: i32,
    pub block_state_data_id: u8,
    pub block_state_default: VanillaEntityMetadataDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaFireworkRocketMetadataAccessors {
    pub item_data_id: u8,
    pub item_default_resource_id: &'static str,
    pub attached_to_target_data_id: u8,
    pub attached_to_target_default: Option<i32>,
    pub shot_at_angle_data_id: u8,
    pub shot_at_angle_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VanillaGuardianMetadataAccessors {
    pub(crate) moving_data_id: u8,
    pub(crate) moving_default: bool,
    pub(crate) attack_target_data_id: u8,
    pub(crate) attack_target_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VanillaWitherSideHeadTargetMetadataAccessors {
    pub(crate) target_b_data_id: u8,
    pub(crate) target_b_default: i32,
    pub(crate) target_c_data_id: u8,
    pub(crate) target_c_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaShulkerMetadataAccessors {
    pub attach_face_data_id: u8,
    pub attach_face_default: i32,
    pub peek_data_id: u8,
    pub peek_default: i8,
    pub color_data_id: u8,
    pub color_default: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractArrowMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub crit_mask: i8,
    pub no_physics_mask: i8,
    pub pierce_level_data_id: u8,
    pub pierce_level_default: i8,
    pub in_ground_data_id: u8,
    pub in_ground_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaArrowMetadataAccessors {
    pub effect_color_data_id: u8,
    pub effect_color_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaThrownTridentMetadataAccessors {
    pub loyalty_data_id: u8,
    pub loyalty_default: i8,
    pub foil_data_id: u8,
    pub foil_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaZombieMetadataAccessors {
    pub baby_data_id: u8,
    pub baby_default: bool,
    pub special_type_data_id: u8,
    pub special_type_default: i32,
    pub drowned_conversion_data_id: u8,
    pub drowned_conversion_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractMinecartMetadataAccessors {
    pub custom_display_block_data_id: u8,
    pub display_offset_data_id: u8,
    pub display_offset_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaArmorStandMetadataAccessors {
    pub client_flags_data_id: u8,
    pub client_flags_default: i8,
    pub small_mask: i8,
    pub show_arms_mask: i8,
    pub no_baseplate_mask: i8,
    pub marker_mask: i8,
    pub head_pose_data_id: u8,
    pub head_pose_default: [f32; 3],
    pub body_pose_data_id: u8,
    pub body_pose_default: [f32; 3],
    pub left_arm_pose_data_id: u8,
    pub left_arm_pose_default: [f32; 3],
    pub right_arm_pose_data_id: u8,
    pub right_arm_pose_default: [f32; 3],
    pub left_leg_pose_data_id: u8,
    pub left_leg_pose_default: [f32; 3],
    pub right_leg_pose_data_id: u8,
    pub right_leg_pose_default: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaInteractionMetadataAccessors {
    pub width_data_id: u8,
    pub width_default: f32,
    pub height_data_id: u8,
    pub height_default: f32,
    pub response_data_id: u8,
    pub response_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCreeperMetadataAccessors {
    pub swell_dir_data_id: u8,
    pub swell_dir_default: i32,
    pub powered_data_id: u8,
    pub powered_default: bool,
    pub ignited_data_id: u8,
    pub ignited_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaHappyGhastMetadataAccessors {
    pub is_leash_holder_data_id: u8,
    pub is_leash_holder_default: bool,
    pub stays_still_data_id: u8,
    pub stays_still_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCopperGolemMetadataAccessors {
    pub weather_state_data_id: u8,
    pub weather_state_default: i32,
    pub state_data_id: u8,
    pub state_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityBabyBehavior {
    Never,
    Metadata,
    ArmorStandSmall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityDefaultAttribute {
    pub attribute_id: i32,
    /// Exact Java `double` bits passed to `AttributeSupplier.Builder.add`.
    /// `None` means the one-argument overload retained the Attribute registry
    /// default rather than supplying an entity-family override.
    pub base_value_override_bits: Option<u64>,
}

impl VanillaEntityDefaultAttribute {
    pub const fn registry_default(attribute_id: i32) -> Self {
        Self {
            attribute_id,
            base_value_override_bits: None,
        }
    }

    pub const fn overridden(attribute_id: i32, base_value_override_bits: u64) -> Self {
        Self {
            attribute_id,
            base_value_override_bits: Some(base_value_override_bits),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityTypeMetadataCatalogEntry {
    pub entity_type_id: i32,
    pub constant_name: &'static str,
    pub resource_id: &'static str,
    pub runtime_class: Option<&'static str>,
    pub schema_class: Option<&'static str>,
    /// Exact membership of the Vanilla `DefaultAttributes` supplier for this
    /// entity type, expressed in the canonical built-in attribute id space.
    /// Non-living entity types have an empty slice.
    pub default_attributes: &'static [VanillaEntityDefaultAttribute],
    /// Nearest vanilla class that declares `handleEntityEvent(byte)` for this runtime type.
    pub handle_entity_event_owner: Option<&'static str>,
    /// Nearest runtime-class declarations for the two attachment hooks and
    /// the controller decision. These close the fixed-type reverse inventory
    /// without duplicating family membership in handwritten code.
    pub passenger_attachment_owner: Option<&'static str>,
    pub vehicle_attachment_owner: Option<&'static str>,
    pub controlling_passenger_owner: Option<&'static str>,
    /// Effective vanilla declaration owners for leash attachment and rope-holder
    /// positioning. `leash_offset_owner` is absent for non-`Leashable` types;
    /// its generated value follows the interface default's zero-argument
    /// virtual dispatch when no class overrides the partial-tick overload.
    pub leash_offset_owner: Option<&'static str>,
    pub rope_hold_position_owner: Option<&'static str>,
    /// Nearest runtime-class declarations for the methods driven by
    /// `ClientLevel.tickNonPassenger` / `tickPassenger`. These are generated
    /// audit facts, not a claim that each whole method body executes client side.
    pub tick_owner: Option<&'static str>,
    pub base_tick_owner: Option<&'static str>,
    pub ai_step_owner: Option<&'static str>,
    pub ride_tick_owner: Option<&'static str>,
    /// Source-derived client transform/timer owner for the runtime class.
    /// This is a semantic dispatch policy, not permission to run server-side
    /// collision consequences, damage, AI, spawning, or removal locally.
    pub client_movement_policy: VanillaEntityClientMovementPolicy,
    /// Source-verified `applyEffectsFromBlocks` call contract for one vanilla
    /// client entity tick. This is kept on the generated runtime-class row so
    /// the canonical tick owner never grows a parallel entity-type table.
    pub block_effect_tick_policy: VanillaEntityBlockEffectTickPolicy,
    /// Generated runtime-class policy for client movement interpolation.
    /// Keeping this fact on the fixed EntityType row avoids walking the Java
    /// class graph for every entity on every client tick.
    pub interpolation_policy: VanillaEntityInterpolationPolicy,
    /// Source-derived nearest `LivingEntity.isBaby()` behavior for this runtime class.
    pub baby_behavior: VanillaEntityBabyBehavior,
    /// Baby branch of the nearest `LivingEntity.getAgeScale()` implementation, as exact f32 bits.
    pub baby_age_scale_bits: u32,
    pub is_pickable_by_class: bool,
    pub sound_source_kind: VanillaEntitySoundSourceKind,
    /// Nearest Vanilla declarations and their source-interpreted
    /// `LivingEntity.handleEntityEvent(3)` / `handleDamageEvent` audio
    /// policies. Non-living rows have no owners and carry explicit `NonLiving`
    /// policies.
    pub death_sound_owner: Option<&'static str>,
    pub death_sound_policy: VanillaEntityDeathSoundPolicy,
    pub hurt_sound_owner: Option<&'static str>,
    pub hurt_sound_policy: VanillaEntityHurtSoundPolicy,
    pub sound_volume_owner: Option<&'static str>,
    pub sound_volume_policy: VanillaEntitySoundVolumePolicy,
    pub pushability_kind: VanillaEntityPushabilityKind,
    pub collidability_kind: VanillaEntityCollidabilityKind,
    pub is_living: bool,
    pub is_animal: bool,
    pub is_tamable_animal: bool,
    pub is_projectile: bool,
    pub is_hurting_projectile: bool,
    pub is_wind_charge: bool,
    pub is_vehicle: bool,
    pub is_boat: bool,
    pub boat_ride_height_multiplier_bits: Option<u32>,
    pub can_use_every_equipment_slot: bool,
    pub is_abstract_chest_boat: bool,
    pub is_minecart: bool,
    pub is_abstract_horse: bool,
    pub is_abstract_chested_horse: bool,
    pub is_abstract_nautilus: bool,
    pub is_llama: bool,
    pub is_item_frame: bool,
    pub is_guardian: bool,
    pub is_flying_animal: bool,
    pub is_camel: bool,
    pub is_squid: bool,
    pub is_slime: bool,
    pub is_abstract_piglin: bool,
    pub is_spider: bool,
    pub illager_arm_pose_uses_celebrating: bool,
    pub is_player_rideable_jumping: bool,
    pub has_custom_inventory_screen: bool,
    pub diagnostic: Option<&'static str>,
}

mod generated_26_1;

pub use generated_26_1::*;

pub const fn vanilla_entity_metadata_schema(
    family: VanillaEntityMetadataFamily,
) -> &'static VanillaEntityMetadataFamilySchema {
    generated_26_1::generated_entity_metadata_schema(family)
}

pub fn vanilla_entity_type_has_attribute(entity_type_id: i32, attribute_id: i32) -> bool {
    vanilla_entity_type_default_attribute(entity_type_id, attribute_id).is_some()
}

pub fn vanilla_entity_type_default_attribute(
    entity_type_id: i32,
    attribute_id: i32,
) -> Option<&'static VanillaEntityDefaultAttribute> {
    let Ok(index) = usize::try_from(entity_type_id) else {
        return None;
    };
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(index)
        .and_then(|entry| {
            entry
                .default_attributes
                .iter()
                .find(|entry| entry.attribute_id == attribute_id)
        })
}

pub fn vanilla_entity_metadata_field(
    mut family: VanillaEntityMetadataFamily,
    data_id: u8,
) -> Option<&'static VanillaEntityMetadataFieldSchema> {
    loop {
        let schema = vanilla_entity_metadata_schema(family);
        if let Some(field) = schema
            .own_fields
            .iter()
            .find(|field| field.data_id == data_id)
        {
            return Some(field);
        }
        family = schema.parent?;
    }
}

/// Resolves an accessor declared by an entity type's generated schema class or one of its
/// superclasses. This exposes extracted wire metadata without requiring a typed-default family
/// facade for accessors whose vanilla default remains runtime-dependent.
pub fn vanilla_entity_metadata_accessor_for_entity_type(
    entity_type_id: i32,
    java_name: &str,
) -> Option<&'static VanillaEntityMetadataAccessorCatalogEntry> {
    vanilla_entity_metadata_accessor_index()
        .get(usize::try_from(entity_type_id).ok()?)?
        .iter()
        .copied()
        .find(|entry| entry.java_name == java_name)
}

/// Resolves the accessor occupying `data_id` along an entity type's generated
/// metadata inheritance chain.
///
/// Vanilla indexes `SynchedEntityData.itemsById` by this value before comparing
/// the accessor's serializer in `assignValue`. Keeping the same lookup on the
/// generated schema prevents a packet from attaching a valid wire value to an
/// accessor owned by another entity family.
pub fn vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
    entity_type_id: i32,
    data_id: u8,
) -> Option<&'static VanillaEntityMetadataAccessorCatalogEntry> {
    vanilla_entity_metadata_accessor_index()
        .get(usize::try_from(entity_type_id).ok()?)?
        .iter()
        .copied()
        .find(|entry| entry.data_id == data_id)
}

fn vanilla_entity_type_metadata_entry(
    entity_type_id: i32,
) -> Option<&'static VanillaEntityTypeMetadataCatalogEntry> {
    let entry = VANILLA_ENTITY_TYPE_METADATA_26_1.get(usize::try_from(entity_type_id).ok()?)?;
    (entry.entity_type_id == entity_type_id).then_some(entry)
}

fn vanilla_entity_metadata_class(
    java_class: &str,
) -> Option<&'static VanillaEntityMetadataClassCatalogEntry> {
    VANILLA_ENTITY_METADATA_CLASSES_26_1
        .binary_search_by(|entry| entry.java_class.cmp(java_class))
        .ok()
        .map(|index| &VANILLA_ENTITY_METADATA_CLASSES_26_1[index])
}

fn vanilla_entity_metadata_accessors_on_class(
    java_class: &str,
) -> &'static [VanillaEntityMetadataAccessorCatalogEntry] {
    let first =
        VANILLA_ENTITY_METADATA_ACCESSORS_26_1.partition_point(|entry| entry.owner < java_class);
    let count = VANILLA_ENTITY_METADATA_ACCESSORS_26_1[first..]
        .partition_point(|entry| entry.owner == java_class);
    &VANILLA_ENTITY_METADATA_ACCESSORS_26_1[first..first + count]
}

fn vanilla_entity_metadata_accessor_index(
) -> &'static [Box<[&'static VanillaEntityMetadataAccessorCatalogEntry]>] {
    static INDEX: OnceLock<Vec<Box<[&'static VanillaEntityMetadataAccessorCatalogEntry]>>> =
        OnceLock::new();
    INDEX.get_or_init(|| {
        VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .map(|entity_type| {
                let mut resolved = Vec::new();
                let mut java_class = entity_type.schema_class;
                while let Some(class_name) = java_class {
                    for accessor in vanilla_entity_metadata_accessors_on_class(class_name) {
                        if resolved.iter().all(
                            |resolved: &&VanillaEntityMetadataAccessorCatalogEntry| {
                                resolved.java_name != accessor.java_name
                            },
                        ) {
                            resolved.push(accessor);
                        }
                    }
                    java_class = vanilla_entity_metadata_class(class_name)
                        .and_then(|class| class.superclass);
                }
                resolved.into_boxed_slice()
            })
            .collect()
    })
}

/// Resolves the generated Player absorption accessor and its typed default.
pub fn vanilla_entity_metadata_player_absorption_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, f32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_PLAYER_ABSORPTION_ID",
    )?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.player.Player",
            "FLOAT",
            Some(VanillaEntityMetadataDefault::Float(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the nearest generated `DATA_VILLAGER_DATA` accessor for an entity type.
///
/// Villager and ZombieVillager declare the same semantic field at different wire indices. The
/// generated schema hierarchy owns both membership and index selection, so render consumers do
/// not need a parallel entity-type switch.
pub fn vanilla_entity_metadata_villager_data_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<u8> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VILLAGER_DATA")?;
    (accessor.serializer_name == "VILLAGER_DATA").then_some(accessor.data_id)
}

/// Resolves the generated Skeleton freeze-conversion accessor and its typed default.
///
/// Only Skeleton declares `DATA_STRAY_CONVERSION_ID`; other renderers using
/// `AbstractSkeletonRenderer` inherit the base fully-frozen shake but not this metadata field.
pub fn vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_STRAY_CONVERSION_ID",
    )?;
    match (accessor.serializer_name, accessor.typed_default) {
        ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(default))) => {
            Some((accessor.data_id, default))
        }
        _ => None,
    }
}

/// Resolves the generated Strider boost-time accessor and its typed default.
pub fn vanilla_entity_metadata_strider_boost_time_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_BOOST_TIME")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.Strider",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated `ItemBasedSteering` boost-time accessor for the two
/// Vanilla owners that compose it. The generated nearest-field lookup owns
/// membership and wire index; this helper only gives the shared client tick a
/// typed semantic surface.
pub(crate) fn vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_BOOST_TIME")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.pig.Pig"
            | "net.minecraft.world.entity.monster.Strider",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Strider suffocating accessor and its typed default.
pub fn vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SUFFOCATING")?;
    match (accessor.serializer_name, accessor.typed_default) {
        ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(default))) => {
            Some((accessor.data_id, default))
        }
        _ => None,
    }
}

/// Resolves the generated Witch using-item accessor and its typed default.
pub fn vanilla_entity_metadata_witch_using_item_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_USING_ITEM")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.Witch",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated ZombieVillager curing-conversion accessor and its typed default.
pub fn vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_CONVERTING_ID")?;
    match (accessor.serializer_name, accessor.typed_default) {
        ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(default))) => {
            Some((accessor.data_id, default))
        }
        _ => None,
    }
}

/// Resolves the nearest generated zombification-immunity accessor and its typed default.
///
/// AbstractPiglin owns the field inherited by Piglin and PiglinBrute, while Hoglin declares an
/// independent accessor at a different wire index. Zoglin and ZombifiedPiglin own neither.
pub fn vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_IMMUNE_TO_ZOMBIFICATION",
    )?;
    match (accessor.serializer_name, accessor.typed_default) {
        ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(default))) => {
            Some((accessor.data_id, default))
        }
        _ => None,
    }
}

/// Resolves the generated EnderMan render metadata accessors and their typed defaults.
///
/// `EndermanRenderer.extractRenderState` reads both carried-block and creepy state, so this
/// descriptor keeps their membership, wire indices, serializers, and defaults on one schema-owned
/// path instead of a parallel entity-type predicate and fixed aliases.
pub fn vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaEndermanRenderMetadataAccessors> {
    let carried =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_CARRY_STATE")?;
    let creepy = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_CREEPY")?;
    match (
        (
            carried.owner,
            carried.serializer_name,
            carried.typed_default,
        ),
        (creepy.owner, creepy.serializer_name, creepy.typed_default),
    ) {
        (
            (
                "net.minecraft.world.entity.monster.EnderMan",
                "OPTIONAL_BLOCK_STATE",
                Some(VanillaEntityMetadataDefault::OptionalBlockStateNone),
            ),
            (
                "net.minecraft.world.entity.monster.EnderMan",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(creepy_default)),
            ),
        ) => Some(VanillaEndermanRenderMetadataAccessors {
            carried_block_state_data_id: carried.data_id,
            carried_block_state_default: None,
            creepy_data_id: creepy.data_id,
            creepy_default,
        }),
        _ => None,
    }
}

/// Resolves the generated EndCrystal beam-target and show-bottom accessors with typed defaults.
pub fn vanilla_entity_metadata_end_crystal_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaEndCrystalMetadataAccessors> {
    let beam_target =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_BEAM_TARGET")?;
    let show_bottom =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SHOW_BOTTOM")?;
    match (
        (
            beam_target.owner,
            beam_target.serializer_name,
            beam_target.typed_default,
        ),
        (
            show_bottom.owner,
            show_bottom.serializer_name,
            show_bottom.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.boss.enderdragon.EndCrystal",
                "OPTIONAL_BLOCK_POS",
                Some(VanillaEntityMetadataDefault::OptionalBlockPosNone),
            ),
            (
                "net.minecraft.world.entity.boss.enderdragon.EndCrystal",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(show_bottom_default)),
            ),
        ) => Some(VanillaEndCrystalMetadataAccessors {
            beam_target_data_id: beam_target.data_id,
            show_bottom_data_id: show_bottom.data_id,
            show_bottom_default,
        }),
        _ => None,
    }
}

/// Resolves the generated Ghast charging accessor and its typed default.
pub fn vanilla_entity_metadata_ghast_charging_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_IS_CHARGING")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.Ghast",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated HappyGhast collision-state accessors and typed defaults.
pub fn vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaHappyGhastMetadataAccessors> {
    let leash_holder =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "IS_LEASH_HOLDER")?;
    let stays_still =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "STAYS_STILL")?;
    match (
        (
            leash_holder.owner,
            leash_holder.serializer_name,
            leash_holder.typed_default,
        ),
        (
            stays_still.owner,
            stays_still.serializer_name,
            stays_still.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.happyghast.HappyGhast",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(is_leash_holder_default)),
            ),
            (
                "net.minecraft.world.entity.animal.happyghast.HappyGhast",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(stays_still_default)),
            ),
        ) => Some(VanillaHappyGhastMetadataAccessors {
            is_leash_holder_data_id: leash_holder.data_id,
            is_leash_holder_default,
            stays_still_data_id: stays_still.data_id,
            stays_still_default,
        }),
        _ => None,
    }
}

/// Resolves the generated GlowSquid dark-ticks accessor and its typed default.
pub fn vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_DARK_TICKS_REMAINING",
    )?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.squid.GlowSquid",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves Goat's generated screaming and horn accessors with typed defaults.
pub fn vanilla_entity_metadata_goat_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaGoatMetadataAccessors> {
    let screaming =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_IS_SCREAMING_GOAT")?;
    let left_horn =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_HAS_LEFT_HORN")?;
    let right_horn =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_HAS_RIGHT_HORN")?;
    match (
        (
            screaming.owner,
            screaming.serializer_name,
            screaming.typed_default,
        ),
        (
            left_horn.owner,
            left_horn.serializer_name,
            left_horn.typed_default,
        ),
        (
            right_horn.owner,
            right_horn.serializer_name,
            right_horn.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.goat.Goat",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(screaming_default)),
            ),
            (
                "net.minecraft.world.entity.animal.goat.Goat",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(left_horn_default)),
            ),
            (
                "net.minecraft.world.entity.animal.goat.Goat",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(right_horn_default)),
            ),
        ) => Some(VanillaGoatMetadataAccessors {
            screaming_data_id: screaming.data_id,
            screaming_default,
            left_horn_data_id: left_horn.data_id,
            left_horn_default,
            right_horn_data_id: right_horn.data_id,
            right_horn_default,
        }),
        _ => None,
    }
}

/// Resolves Turtle's generated egg-state accessors with typed defaults.
pub fn vanilla_entity_metadata_turtle_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaTurtleMetadataAccessors> {
    let has_egg = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "HAS_EGG")?;
    let laying_egg =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "LAYING_EGG")?;
    match (
        (
            has_egg.owner,
            has_egg.serializer_name,
            has_egg.typed_default,
        ),
        (
            laying_egg.owner,
            laying_egg.serializer_name,
            laying_egg.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.turtle.Turtle",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(has_egg_default)),
            ),
            (
                "net.minecraft.world.entity.animal.turtle.Turtle",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(laying_egg_default)),
            ),
        ) => Some(VanillaTurtleMetadataAccessors {
            has_egg_data_id: has_egg.data_id,
            has_egg_default,
            laying_egg_data_id: laying_egg.data_id,
            laying_egg_default,
        }),
        _ => None,
    }
}

/// Resolves the generated WitherSkull dangerous accessor and its typed default.
pub fn vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_DANGEROUS")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.projectile.hurtingprojectile.WitherSkull",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the inherited hanging-entity direction accessor and its typed default.
pub fn vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaHangingEntityMetadataAccessor> {
    let direction =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_DIRECTION")?;
    match (
        direction.owner,
        direction.serializer_name,
        direction.typed_default,
    ) {
        (
            "net.minecraft.world.entity.decoration.HangingEntity",
            "DIRECTION",
            Some(VanillaEntityMetadataDefault::Direction(direction_default)),
        ) => Some(VanillaHangingEntityMetadataAccessor {
            direction_data_id: direction.data_id,
            direction_default,
        }),
        _ => None,
    }
}

/// Resolves Painting's synchronized variant accessor.
///
/// The generated typed default is a virtual "any registry holder" marker, not a stable concrete
/// holder id, so consumers only receive the validated wire index and continue resolving holder
/// values through the active session registry.
pub fn vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<u8> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_PAINTING_VARIANT_ID",
    )?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.decoration.painting.Painting",
            "PAINTING_VARIANT",
            Some(VanillaEntityMetadataDefault::RegistryAny("minecraft:painting_variant")),
        ) => Some(accessor.data_id),
        _ => None,
    }
}

/// Resolves the complete ItemFrame metadata layout, including its inherited direction accessor.
pub fn vanilla_entity_metadata_item_frame_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaItemFrameMetadataAccessors> {
    let direction =
        vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(entity_type_id)?;
    let item = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ITEM")?;
    let rotation =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ROTATION")?;
    match (
        (item.owner, item.serializer_name, item.typed_default),
        (
            rotation.owner,
            rotation.serializer_name,
            rotation.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.decoration.ItemFrame",
                "ITEM_STACK",
                Some(VanillaEntityMetadataDefault::ItemStackEmpty),
            ),
            (
                "net.minecraft.world.entity.decoration.ItemFrame",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(rotation_default)),
            ),
        ) => Some(VanillaItemFrameMetadataAccessors {
            direction_data_id: direction.direction_data_id,
            direction_default: direction.direction_default,
            item_data_id: item.data_id,
            rotation_data_id: rotation.data_id,
            rotation_default,
        }),
        _ => None,
    }
}

/// Resolves the generated Bat flags accessor, typed default, and named resting mask.
pub fn vanilla_entity_metadata_bat_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_FLAGS")?;
    let resting_mask = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Bat)
        .own_flags
        .iter()
        .find(|flag| flag.name == "FLAG_RESTING" && flag.data_id == accessor.data_id)?
        .mask;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.ambient.Bat",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => Some((accessor.data_id, default, resting_mask)),
        _ => None,
    }
}

/// Resolves Bee's generated flags, named masks, and anger-end-time accessor.
pub fn vanilla_entity_metadata_bee_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaBeeMetadataAccessors> {
    let flags = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_FLAGS_ID")?;
    let anger_end_time =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ANGER_END_TIME")?;
    match (
        (flags.owner, flags.serializer_name, flags.typed_default),
        (
            anger_end_time.owner,
            anger_end_time.serializer_name,
            anger_end_time.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.bee.Bee",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(flags_default)),
            ),
            (
                "net.minecraft.world.entity.animal.bee.Bee",
                "LONG",
                Some(VanillaEntityMetadataDefault::Long(anger_end_time_default)),
            ),
        ) => {
            let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Bee);
            let mask = |name| {
                schema
                    .own_flags
                    .iter()
                    .find(|flag| flag.name == name && flag.data_id == flags.data_id)
                    .map(|flag| flag.mask)
            };
            Some(VanillaBeeMetadataAccessors {
                flags_data_id: flags.data_id,
                flags_default,
                roll_mask: mask("FLAG_ROLL")?,
                has_stung_mask: mask("FLAG_HAS_STUNG")?,
                has_nectar_mask: mask("FLAG_HAS_NECTAR")?,
                anger_end_time_data_id: anger_end_time.data_id,
                anger_end_time_default,
            })
        }
        _ => None,
    }
}

/// Resolves the generated Bee flags accessor and its typed default.
pub fn vanilla_entity_metadata_bee_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8)> {
    vanilla_entity_metadata_bee_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.flags_data_id, accessors.flags_default))
}

/// Resolves Dolphin's generated got-fish and moistness accessors.
pub fn vanilla_entity_metadata_dolphin_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaDolphinMetadataAccessors> {
    let got_fish = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "GOT_FISH")?;
    let moistness =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "MOISTNESS_LEVEL")?;
    match (
        (
            got_fish.owner,
            got_fish.serializer_name,
            got_fish.typed_default,
        ),
        (
            moistness.owner,
            moistness.serializer_name,
            moistness.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.dolphin.Dolphin",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(got_fish_default)),
            ),
            (
                "net.minecraft.world.entity.animal.dolphin.Dolphin",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(moistness_default)),
            ),
        ) => Some(VanillaDolphinMetadataAccessors {
            got_fish_data_id: got_fish.data_id,
            got_fish_default,
            moistness_data_id: moistness.data_id,
            moistness_default,
        }),
        _ => None,
    }
}

/// Resolves Fox's generated variant, flags, named masks, and trusted-entity accessors.
pub fn vanilla_entity_metadata_fox_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaFoxMetadataAccessors> {
    let variant = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TYPE_ID")?;
    let flags = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_FLAGS_ID")?;
    let trusted_0 =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TRUSTED_ID_0")?;
    let trusted_1 =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TRUSTED_ID_1")?;
    match (
        (
            variant.owner,
            variant.serializer_name,
            variant.typed_default,
        ),
        (flags.owner, flags.serializer_name, flags.typed_default),
        (
            trusted_0.owner,
            trusted_0.serializer_name,
            trusted_0.typed_default,
        ),
        (
            trusted_1.owner,
            trusted_1.serializer_name,
            trusted_1.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.fox.Fox",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(type_default)),
            ),
            (
                "net.minecraft.world.entity.animal.fox.Fox",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(flags_default)),
            ),
            (
                "net.minecraft.world.entity.animal.fox.Fox",
                "OPTIONAL_LIVING_ENTITY_REFERENCE",
                Some(VanillaEntityMetadataDefault::OptionalLivingEntityReferenceNone),
            ),
            (
                "net.minecraft.world.entity.animal.fox.Fox",
                "OPTIONAL_LIVING_ENTITY_REFERENCE",
                Some(VanillaEntityMetadataDefault::OptionalLivingEntityReferenceNone),
            ),
        ) => {
            let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Fox);
            let mask = |name| {
                schema
                    .own_flags
                    .iter()
                    .find(|flag| flag.name == name && flag.data_id == flags.data_id)
                    .map(|flag| flag.mask)
            };
            Some(VanillaFoxMetadataAccessors {
                type_data_id: variant.data_id,
                type_default,
                flags_data_id: flags.data_id,
                flags_default,
                sitting_mask: mask("FLAG_SITTING")?,
                crouching_mask: mask("FLAG_CROUCHING")?,
                interested_mask: mask("FLAG_INTERESTED")?,
                pouncing_mask: mask("FLAG_POUNCING")?,
                sleeping_mask: mask("FLAG_SLEEPING")?,
                faceplanted_mask: mask("FLAG_FACEPLANTED")?,
                defending_mask: mask("FLAG_DEFENDING")?,
                trusted_data_ids: [trusted_0.data_id, trusted_1.data_id],
                trusted_default: None,
            })
        }
        _ => None,
    }
}

/// Resolves the generated Fox flags accessor and its typed default.
pub fn vanilla_entity_metadata_fox_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8)> {
    vanilla_entity_metadata_fox_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.flags_data_id, accessors.flags_default))
}

/// Resolves the generated IronGolem flags accessor, typed default, and player-created mask.
pub fn vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_FLAGS_ID")?;
    let player_created_mask =
        vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::IronGolem)
            .own_flags
            .iter()
            .find(|flag| flag.name == "isPlayerCreated" && flag.data_id == accessor.data_id)?
            .mask;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.golem.IronGolem",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => Some((accessor.data_id, default, player_created_mask)),
        _ => None,
    }
}

/// Resolves the generated SnowGolem pumpkin accessor, typed default, and named mask.
pub fn vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_PUMPKIN_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.golem.SnowGolem",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => Some((accessor.data_id, default, VANILLA_SNOW_GOLEM_PUMPKIN_FLAG)),
        _ => None,
    }
}

/// Resolves the generated Sheep wool accessor, typed default, and named masks.
pub fn vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8, u8)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_WOOL_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.sheep.Sheep",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => Some((
            accessor.data_id,
            default,
            VANILLA_SHEEP_FLAG_SHEARED,
            VANILLA_SHEEP_WOOL_COLOR_MASK,
        )),
        _ => None,
    }
}

/// Resolves the generated Rabbit variant accessor and its typed default.
pub fn vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TYPE_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.rabbit.Rabbit",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Pufferfish puff-state accessor and its typed default.
pub fn vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "PUFF_STATE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.fish.Pufferfish",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the inherited Slime size accessor and its typed default.
pub fn vanilla_entity_metadata_slime_size_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "ID_SIZE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.Slime",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Phantom size accessor and its typed default.
pub fn vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "ID_SIZE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.Phantom",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Salmon variant accessor and its typed default.
pub fn vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TYPE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.fish.Salmon",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated TropicalFish packed-variant accessor and bit layout.
pub fn vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaTropicalFishVariantAccessors> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_TYPE_VARIANT")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.fish.TropicalFish",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some(VanillaTropicalFishVariantAccessors {
            variant_data_id: accessor.data_id,
            variant_default: default,
            pattern_mask: VANILLA_TROPICAL_FISH_PATTERN_MASK,
            base_color_mask: VANILLA_TROPICAL_FISH_BASE_COLOR_MASK,
            base_color_shift: VANILLA_TROPICAL_FISH_BASE_COLOR_SHIFT,
            pattern_color_mask: VANILLA_TROPICAL_FISH_PATTERN_COLOR_MASK,
            pattern_color_shift: VANILLA_TROPICAL_FISH_PATTERN_COLOR_SHIFT,
        }),
        _ => None,
    }
}

/// Resolves the generated Parrot variant accessor and its typed default.
pub fn vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VARIANT_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.parrot.Parrot",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Axolotl variant accessor and its typed default.
pub fn vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VARIANT")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.axolotl.Axolotl",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Axolotl from-bucket accessor and its typed default.
pub fn vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "FROM_BUCKET")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.axolotl.Axolotl",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated inherited AbstractFish from-bucket accessor and its typed default.
pub fn vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "FROM_BUCKET")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.fish.AbstractFish",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Ocelot trusting accessor and its typed default.
pub fn vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TRUSTING")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.feline.Ocelot",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Fox variant accessor and its typed default.
pub fn vanilla_entity_metadata_fox_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    vanilla_entity_metadata_fox_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.type_data_id, accessors.type_default))
}

/// Resolves the generated MushroomCow variant accessor and its typed default.
pub fn vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TYPE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.cow.MushroomCow",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Bogged sheared accessor and its typed default.
pub fn vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SHEARED")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.skeleton.Bogged",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Allay dancing accessor and its typed default.
pub fn vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_DANCING")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.allay.Allay",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Allay can-duplicate accessor and its typed default.
pub fn vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_CAN_DUPLICATE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.allay.Allay",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Axolotl playing-dead accessor and its typed default.
pub fn vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_PLAYING_DEAD")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.axolotl.Axolotl",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Camel dash accessor and its typed default.
pub fn vanilla_entity_metadata_camel_dash_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DASH")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.camel.Camel",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Camel last-pose-change accessor and its typed default.
pub fn vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i64)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "LAST_POSE_CHANGE_TICK")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.camel.Camel",
            "LONG",
            Some(VanillaEntityMetadataDefault::Long(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves Cat's generated lying, relax-state-one, and collar-color accessors.
pub fn vanilla_entity_metadata_cat_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCatMetadataAccessors> {
    let lying = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "IS_LYING")?;
    let relax_state_one =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "RELAX_STATE_ONE")?;
    let collar_color =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_COLLAR_COLOR")?;
    match (
        (lying.owner, lying.serializer_name, lying.typed_default),
        (
            relax_state_one.owner,
            relax_state_one.serializer_name,
            relax_state_one.typed_default,
        ),
        (
            collar_color.owner,
            collar_color.serializer_name,
            collar_color.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.feline.Cat",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(lying_default)),
            ),
            (
                "net.minecraft.world.entity.animal.feline.Cat",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(relax_state_one_default)),
            ),
            (
                "net.minecraft.world.entity.animal.feline.Cat",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(collar_color_default)),
            ),
        ) => Some(VanillaCatMetadataAccessors {
            lying_data_id: lying.data_id,
            lying_default,
            relax_state_one_data_id: relax_state_one.data_id,
            relax_state_one_default,
            collar_color_data_id: collar_color.data_id,
            collar_color_default,
        }),
        _ => None,
    }
}

/// Resolves the generated Cat lying accessor and its typed default.
pub fn vanilla_entity_metadata_cat_lying_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    vanilla_entity_metadata_cat_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.lying_data_id, accessors.lying_default))
}

/// Resolves the generated Cat relax-state-one accessor and its typed default.
pub fn vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    vanilla_entity_metadata_cat_accessors_for_entity_type(entity_type_id).map(|accessors| {
        (
            accessors.relax_state_one_data_id,
            accessors.relax_state_one_default,
        )
    })
}

/// Resolves the generated Creaking can-move accessor and its typed default.
pub fn vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "CAN_MOVE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.creaking.Creaking",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Creaking active accessor and its typed default.
pub fn vanilla_entity_metadata_creaking_active_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "IS_ACTIVE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.creaking.Creaking",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Creaking tearing-down accessor and its typed default.
pub fn vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "IS_TEARING_DOWN")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.creaking.Creaking",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Creaking home-position accessor with its empty optional default.
pub fn vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<u8> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "HOME_POS")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.creaking.Creaking",
            "OPTIONAL_BLOCK_POS",
            Some(VanillaEntityMetadataDefault::OptionalBlockPosNone),
        ) => Some(accessor.data_id),
        _ => None,
    }
}

/// Resolves the generated CopperGolem weathering and animation-state accessors and typed defaults.
pub fn vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCopperGolemMetadataAccessors> {
    let weather_state =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_WEATHER_STATE")?;
    let state =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "COPPER_GOLEM_STATE")?;
    match (
        (
            weather_state.owner,
            weather_state.serializer_name,
            weather_state.typed_default,
        ),
        (state.owner, state.serializer_name, state.typed_default),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.golem.CopperGolem",
                "WEATHERING_COPPER_STATE",
                Some(VanillaEntityMetadataDefault::Int(weather_state_default)),
            ),
            (
                "net.minecraft.world.entity.animal.golem.CopperGolem",
                "COPPER_GOLEM_STATE",
                Some(VanillaEntityMetadataDefault::Int(state_default)),
            ),
        ) => Some(VanillaCopperGolemMetadataAccessors {
            weather_state_data_id: weather_state.data_id,
            weather_state_default,
            state_data_id: state.data_id,
            state_default,
        }),
        _ => None,
    }
}

/// Compatibility projection for consumers that need only CopperGolem's animation-state accessor.
pub fn vanilla_entity_metadata_copper_golem_state_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.state_data_id, accessors.state_default))
}

/// Resolves the generated Sniffer animation-state accessor and its typed default.
pub fn vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_STATE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.sniffer.Sniffer",
            "SNIFFER_STATE",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves Sniffer's generated seed-drop tick accessor and typed default.
pub fn vanilla_entity_metadata_sniffer_drop_seed_at_tick_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_DROP_SEED_AT_TICK")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.sniffer.Sniffer",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated villager-data-finalized boolean accessor and default.
///
/// Villager and ZombieVillager each define their own `DATA_VILLAGER_DATA_FINALIZED`
/// slot, so the class-chain lookup lands on the owner's id for either type.
pub fn vanilla_entity_metadata_villager_data_finalized_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_VILLAGER_DATA_FINALIZED",
    )?;
    match (accessor.serializer_name, accessor.typed_default) {
        ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(default))) => {
            Some((accessor.data_id, default))
        }
        _ => None,
    }
}

/// Resolves AreaEffectCloud's generated synced radius accessor and default.
pub fn vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, f32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_RADIUS")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.AreaEffectCloud",
            "FLOAT",
            Some(VanillaEntityMetadataDefault::Float(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves EyeOfEnder's generated synced item-stack accessor.
pub fn vanilla_entity_metadata_eye_of_ender_item_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<u8> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ITEM_STACK")?;
    match (accessor.owner, accessor.serializer_name) {
        ("net.minecraft.world.entity.projectile.EyeOfEnder", "ITEM_STACK") => {
            Some(accessor.data_id)
        }
        _ => None,
    }
}

/// Resolves OminousItemSpawner's generated item-stack accessor and empty default.
pub fn vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<u8> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ITEM")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.OminousItemSpawner",
            "ITEM_STACK",
            Some(VanillaEntityMetadataDefault::ItemStackEmpty),
        ) => Some(accessor.data_id),
        _ => None,
    }
}

/// Resolves ItemEntity's generated item-stack accessor and empty default.
pub fn vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<u8> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ITEM")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.item.ItemEntity",
            "ITEM_STACK",
            Some(VanillaEntityMetadataDefault::ItemStackEmpty),
        ) => Some(accessor.data_id),
        _ => None,
    }
}

/// Resolves the generated Armadillo animation-state accessor and its typed default.
pub fn vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "ARMADILLO_STATE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.armadillo.Armadillo",
            "ARMADILLO_STATE",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Warden client-anger accessor and its typed default.
pub fn vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "CLIENT_ANGER_LEVEL")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.warden.Warden",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated EnderDragon phase accessor and its typed default.
pub fn vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_PHASE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.boss.enderdragon.EnderDragon",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Pillager charging-crossbow accessor and its typed default.
pub fn vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "IS_CHARGING_CROSSBOW")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.illager.Pillager",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated Piglin charging-crossbow accessor and its typed default.
pub fn vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_IS_CHARGING_CROSSBOW",
    )?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.piglin.Piglin",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated PolarBear standing accessor and its typed default.
pub fn vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_STANDING_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.polarbear.PolarBear",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves Wolf's generated interested, collar-color, and anger-end-time accessors.
pub fn vanilla_entity_metadata_wolf_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaWolfMetadataAccessors> {
    let interested =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_INTERESTED_ID")?;
    let collar_color =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_COLLAR_COLOR")?;
    let anger_end_time =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ANGER_END_TIME")?;
    match (
        (
            interested.owner,
            interested.serializer_name,
            interested.typed_default,
        ),
        (
            collar_color.owner,
            collar_color.serializer_name,
            collar_color.typed_default,
        ),
        (
            anger_end_time.owner,
            anger_end_time.serializer_name,
            anger_end_time.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.wolf.Wolf",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(interested_default)),
            ),
            (
                "net.minecraft.world.entity.animal.wolf.Wolf",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(collar_color_default)),
            ),
            (
                "net.minecraft.world.entity.animal.wolf.Wolf",
                "LONG",
                Some(VanillaEntityMetadataDefault::Long(anger_end_time_default)),
            ),
        ) => Some(VanillaWolfMetadataAccessors {
            interested_data_id: interested.data_id,
            interested_default,
            collar_color_data_id: collar_color.data_id,
            collar_color_default,
            anger_end_time_data_id: anger_end_time.data_id,
            anger_end_time_default,
        }),
        _ => None,
    }
}

/// Resolves the generated Wolf interested accessor and its typed default.
pub fn vanilla_entity_metadata_wolf_interested_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    vanilla_entity_metadata_wolf_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.interested_data_id, accessors.interested_default))
}

/// Resolves the inherited TamableAnimal accessors, typed defaults, and named masks.
pub fn vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaTamableAnimalMetadataAccessors> {
    let flags = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_FLAGS_ID")?;
    let owner =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_OWNERUUID_ID")?;
    match (
        (flags.owner, flags.serializer_name, flags.typed_default),
        (owner.owner, owner.serializer_name, owner.typed_default),
    ) {
        (
            (
                "net.minecraft.world.entity.TamableAnimal",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(flags_default)),
            ),
            (
                "net.minecraft.world.entity.TamableAnimal",
                "OPTIONAL_LIVING_ENTITY_REFERENCE",
                Some(VanillaEntityMetadataDefault::OptionalLivingEntityReferenceNone),
            ),
        ) => {
            let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::TamableAnimal);
            let mask = |name| {
                schema
                    .own_flags
                    .iter()
                    .find(|flag| flag.name == name && flag.data_id == flags.data_id)
                    .map(|flag| flag.mask)
            };
            Some(VanillaTamableAnimalMetadataAccessors {
                flags_data_id: flags.data_id,
                flags_default,
                sitting_mask: mask("FLAG_SITTING")?,
                tame_mask: mask("FLAG_TAME")?,
                owner_data_id: owner.data_id,
                owner_default: None,
            })
        }
        _ => None,
    }
}

/// Resolves the inherited TamableAnimal flags accessor and its typed default.
pub fn vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8)> {
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.flags_data_id, accessors.flags_default))
}

/// Resolves the inherited AbstractBoat paddle and bubble-time accessors with typed defaults.
pub fn vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractBoatMetadataAccessors> {
    let paddle_left =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_PADDLE_LEFT")?;
    let paddle_right =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_PADDLE_RIGHT")?;
    let bubble_time =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_BUBBLE_TIME")?;
    match (
        (
            paddle_left.owner,
            paddle_left.serializer_name,
            paddle_left.typed_default,
        ),
        (
            paddle_right.owner,
            paddle_right.serializer_name,
            paddle_right.typed_default,
        ),
        (
            bubble_time.owner,
            bubble_time.serializer_name,
            bubble_time.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.vehicle.boat.AbstractBoat",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(paddle_left_default)),
            ),
            (
                "net.minecraft.world.entity.vehicle.boat.AbstractBoat",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(paddle_right_default)),
            ),
            (
                "net.minecraft.world.entity.vehicle.boat.AbstractBoat",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(bubble_time_default)),
            ),
        ) => Some(VanillaAbstractBoatMetadataAccessors {
            paddle_left_data_id: paddle_left.data_id,
            paddle_left_default,
            paddle_right_data_id: paddle_right.data_id,
            paddle_right_default,
            bubble_time_data_id: bubble_time.data_id,
            bubble_time_default,
        }),
        _ => None,
    }
}

/// Resolves the inherited VehicleEntity hurt-roll accessors with typed defaults.
pub fn vanilla_entity_metadata_vehicle_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaVehicleMetadataAccessors> {
    let hurt_time =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_HURT")?;
    let hurt_dir =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_HURTDIR")?;
    let damage =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_DAMAGE")?;
    match (
        (
            hurt_time.owner,
            hurt_time.serializer_name,
            hurt_time.typed_default,
        ),
        (
            hurt_dir.owner,
            hurt_dir.serializer_name,
            hurt_dir.typed_default,
        ),
        (damage.owner, damage.serializer_name, damage.typed_default),
    ) {
        (
            (
                "net.minecraft.world.entity.vehicle.VehicleEntity",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(hurt_time_default)),
            ),
            (
                "net.minecraft.world.entity.vehicle.VehicleEntity",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(hurt_dir_default)),
            ),
            (
                "net.minecraft.world.entity.vehicle.VehicleEntity",
                "FLOAT",
                Some(VanillaEntityMetadataDefault::Float(damage_default)),
            ),
        ) => Some(VanillaVehicleMetadataAccessors {
            hurt_time_data_id: hurt_time.data_id,
            hurt_time_default,
            hurt_dir_data_id: hurt_dir.data_id,
            hurt_dir_default,
            damage_data_id: damage.data_id,
            damage_default,
        }),
        _ => None,
    }
}

/// Resolves the generated MinecartFurnace fuel accessor and its typed default.
///
/// The owner also supplies the furnace-minecart membership, so display projection does not need a
/// parallel type predicate or fixed data-id import.
pub fn vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_FUEL")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.vehicle.minecart.MinecartFurnace",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated PrimedTnt fuse and block-state accessors with typed defaults.
///
/// Requiring both owner fields makes their schema membership the single type gate shared by
/// model, fuse interpolation, and client smoke projection.
pub fn vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaPrimedTntMetadataAccessors> {
    let fuse = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_FUSE_ID")?;
    let block_state =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_BLOCK_STATE_ID")?;
    match (
        (fuse.owner, fuse.serializer_name, fuse.typed_default),
        (
            block_state.owner,
            block_state.serializer_name,
            block_state.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.item.PrimedTnt",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(fuse_default)),
            ),
            (
                "net.minecraft.world.entity.item.PrimedTnt",
                "BLOCK_STATE",
                Some(block_state_default @ VanillaEntityMetadataDefault::BlockStateDefault { .. }),
            ),
        ) => Some(VanillaPrimedTntMetadataAccessors {
            fuse_data_id: fuse.data_id,
            fuse_default,
            block_state_data_id: block_state.data_id,
            block_state_default,
        }),
        _ => None,
    }
}

/// Resolves FireworkRocket's generated item, attachment, and shot-angle accessors.
pub fn vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaFireworkRocketMetadataAccessors> {
    let item =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_FIREWORKS_ITEM")?;
    let attached_to_target = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_ATTACHED_TO_TARGET",
    )?;
    let shot_at_angle =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SHOT_AT_ANGLE")?;
    match (
        (item.owner, item.serializer_name, item.typed_default),
        (
            attached_to_target.owner,
            attached_to_target.serializer_name,
            attached_to_target.typed_default,
        ),
        (
            shot_at_angle.owner,
            shot_at_angle.serializer_name,
            shot_at_angle.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.projectile.FireworkRocketEntity",
                "ITEM_STACK",
                Some(VanillaEntityMetadataDefault::ItemStackDefaultItem(item_default_resource_id)),
            ),
            (
                "net.minecraft.world.entity.projectile.FireworkRocketEntity",
                "OPTIONAL_UNSIGNED_INT",
                Some(VanillaEntityMetadataDefault::OptionalUnsignedIntNone),
            ),
            (
                "net.minecraft.world.entity.projectile.FireworkRocketEntity",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(shot_at_angle_default)),
            ),
        ) => Some(VanillaFireworkRocketMetadataAccessors {
            item_data_id: item.data_id,
            item_default_resource_id,
            attached_to_target_data_id: attached_to_target.data_id,
            attached_to_target_default: None,
            shot_at_angle_data_id: shot_at_angle.data_id,
            shot_at_angle_default,
        }),
        _ => None,
    }
}

/// Resolves ExperienceOrb's generated value accessor and typed default.
pub fn vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VALUE")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.ExperienceOrb",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the inherited Guardian moving and attack-target accessors with typed defaults.
pub(crate) fn vanilla_entity_metadata_guardian_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaGuardianMetadataAccessors> {
    let moving =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_MOVING")?;
    let attack_target =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_ATTACK_TARGET")?;
    match (
        (moving.owner, moving.serializer_name, moving.typed_default),
        (
            attack_target.owner,
            attack_target.serializer_name,
            attack_target.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.monster.Guardian",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(moving_default)),
            ),
            (
                "net.minecraft.world.entity.monster.Guardian",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(attack_target_default)),
            ),
        ) => Some(VanillaGuardianMetadataAccessors {
            moving_data_id: moving.data_id,
            moving_default,
            attack_target_data_id: attack_target.data_id,
            attack_target_default,
        }),
        _ => None,
    }
}

/// Resolves the generated Shulker attachment, peek, and color accessors with typed defaults.
pub fn vanilla_entity_metadata_shulker_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaShulkerMetadataAccessors> {
    let attach_face =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ATTACH_FACE_ID")?;
    let peek = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_PEEK_ID")?;
    let color = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_COLOR_ID")?;
    match (
        (
            attach_face.owner,
            attach_face.serializer_name,
            attach_face.typed_default,
        ),
        (peek.owner, peek.serializer_name, peek.typed_default),
        (color.owner, color.serializer_name, color.typed_default),
    ) {
        (
            (
                "net.minecraft.world.entity.monster.Shulker",
                "DIRECTION",
                Some(VanillaEntityMetadataDefault::Direction(attach_face_default)),
            ),
            (
                "net.minecraft.world.entity.monster.Shulker",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(peek_default)),
            ),
            (
                "net.minecraft.world.entity.monster.Shulker",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(color_default)),
            ),
        ) => Some(VanillaShulkerMetadataAccessors {
            attach_face_data_id: attach_face.data_id,
            attach_face_default,
            peek_data_id: peek.data_id,
            peek_default,
            color_data_id: color.data_id,
            color_default,
        }),
        _ => None,
    }
}

/// Resolves the inherited AbstractArrow accessors, typed defaults, and named flag masks.
pub fn vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractArrowMetadataAccessors> {
    let flags = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "ID_FLAGS")?;
    let pierce_level =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "PIERCE_LEVEL")?;
    let in_ground = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "IN_GROUND")?;
    let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractArrow);
    let mask = |name| {
        schema
            .own_flags
            .iter()
            .find(|flag| flag.name == name && flag.data_id == flags.data_id)
            .map(|flag| flag.mask)
    };
    match (
        (flags.owner, flags.serializer_name, flags.typed_default),
        (
            pierce_level.owner,
            pierce_level.serializer_name,
            pierce_level.typed_default,
        ),
        (
            in_ground.owner,
            in_ground.serializer_name,
            in_ground.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.projectile.arrow.AbstractArrow",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(flags_default)),
            ),
            (
                "net.minecraft.world.entity.projectile.arrow.AbstractArrow",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(pierce_level_default)),
            ),
            (
                "net.minecraft.world.entity.projectile.arrow.AbstractArrow",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(in_ground_default)),
            ),
        ) => Some(VanillaAbstractArrowMetadataAccessors {
            flags_data_id: flags.data_id,
            flags_default,
            crit_mask: mask("FLAG_CRIT")?,
            no_physics_mask: mask("FLAG_NOPHYSICS")?,
            pierce_level_data_id: pierce_level.data_id,
            pierce_level_default,
            in_ground_data_id: in_ground.data_id,
            in_ground_default,
        }),
        _ => None,
    }
}

/// Resolves AbstractMinecart's display metadata and the generated per-type virtual offset default.
pub fn vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractMinecartMetadataAccessors> {
    let custom_display_block = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_ID_CUSTOM_DISPLAY_BLOCK",
    )?;
    let display_offset =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_DISPLAY_OFFSET")?;
    match (
        (
            custom_display_block.owner,
            custom_display_block.serializer_name,
            custom_display_block.typed_default,
        ),
        (
            display_offset.owner,
            display_offset.serializer_name,
            display_offset.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart",
                "OPTIONAL_BLOCK_STATE",
                Some(VanillaEntityMetadataDefault::OptionalBlockStateNone),
            ),
            (
                "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart",
                "INT",
                Some(VanillaEntityMetadataDefault::MinecartDefaultDisplayOffset),
            ),
        ) => Some(VanillaAbstractMinecartMetadataAccessors {
            custom_display_block_data_id: custom_display_block.data_id,
            display_offset_data_id: display_offset.data_id,
            display_offset_default: vanilla_minecart_default_display_offset(entity_type_id)?,
        }),
        _ => None,
    }
}

/// Resolves the generated Arrow effect-color accessor and its typed default.
pub fn vanilla_entity_metadata_arrow_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaArrowMetadataAccessors> {
    let effect_color =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "ID_EFFECT_COLOR")?;
    match (
        effect_color.owner,
        effect_color.serializer_name,
        effect_color.typed_default,
    ) {
        (
            "net.minecraft.world.entity.projectile.arrow.Arrow",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(effect_color_default)),
        ) => Some(VanillaArrowMetadataAccessors {
            effect_color_data_id: effect_color.data_id,
            effect_color_default,
        }),
        _ => None,
    }
}

/// Resolves the generated ThrownTrident loyalty/foil accessors and typed defaults.
pub fn vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaThrownTridentMetadataAccessors> {
    let loyalty = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "ID_LOYALTY")?;
    let foil = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "ID_FOIL")?;
    match (
        (
            loyalty.owner,
            loyalty.serializer_name,
            loyalty.typed_default,
        ),
        (foil.owner, foil.serializer_name, foil.typed_default),
    ) {
        (
            (
                "net.minecraft.world.entity.projectile.arrow.ThrownTrident",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(loyalty_default)),
            ),
            (
                "net.minecraft.world.entity.projectile.arrow.ThrownTrident",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(foil_default)),
            ),
        ) => Some(VanillaThrownTridentMetadataAccessors {
            loyalty_data_id: loyalty.data_id,
            loyalty_default,
            foil_data_id: foil.data_id,
            foil_default,
        }),
        _ => None,
    }
}

/// Resolves the generated ArmorStand flags and pose accessors with typed defaults.
pub fn vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaArmorStandMetadataAccessors> {
    let client_flags =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_CLIENT_FLAGS")?;
    let client_flags_default = match (
        client_flags.owner,
        client_flags.serializer_name,
        client_flags.typed_default,
    ) {
        (
            "net.minecraft.world.entity.decoration.ArmorStand",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => default,
        _ => return None,
    };
    let rotations = |java_name| -> Option<(u8, [f32; 3])> {
        let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, java_name)?;
        match (
            accessor.owner,
            accessor.serializer_name,
            accessor.typed_default,
        ) {
            (
                "net.minecraft.world.entity.decoration.ArmorStand",
                "ROTATIONS",
                Some(VanillaEntityMetadataDefault::Rotations(default)),
            ) => Some((accessor.data_id, default)),
            _ => None,
        }
    };
    let (head_pose_data_id, head_pose_default) = rotations("DATA_HEAD_POSE")?;
    let (body_pose_data_id, body_pose_default) = rotations("DATA_BODY_POSE")?;
    let (left_arm_pose_data_id, left_arm_pose_default) = rotations("DATA_LEFT_ARM_POSE")?;
    let (right_arm_pose_data_id, right_arm_pose_default) = rotations("DATA_RIGHT_ARM_POSE")?;
    let (left_leg_pose_data_id, left_leg_pose_default) = rotations("DATA_LEFT_LEG_POSE")?;
    let (right_leg_pose_data_id, right_leg_pose_default) = rotations("DATA_RIGHT_LEG_POSE")?;
    let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ArmorStand);
    let mask = |name| {
        schema
            .own_flags
            .iter()
            .find(|flag| flag.name == name && flag.data_id == client_flags.data_id)
            .map(|flag| flag.mask)
    };
    Some(VanillaArmorStandMetadataAccessors {
        client_flags_data_id: client_flags.data_id,
        client_flags_default,
        small_mask: mask("CLIENT_FLAG_SMALL")?,
        show_arms_mask: mask("CLIENT_FLAG_SHOW_ARMS")?,
        no_baseplate_mask: mask("CLIENT_FLAG_NO_BASEPLATE")?,
        marker_mask: mask("CLIENT_FLAG_MARKER")?,
        head_pose_data_id,
        head_pose_default,
        body_pose_data_id,
        body_pose_default,
        left_arm_pose_data_id,
        left_arm_pose_default,
        right_arm_pose_data_id,
        right_arm_pose_default,
        left_leg_pose_data_id,
        left_leg_pose_default,
        right_leg_pose_data_id,
        right_leg_pose_default,
    })
}

/// Resolves the generated Interaction dimensions and response accessors with typed defaults.
pub fn vanilla_entity_metadata_interaction_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaInteractionMetadataAccessors> {
    let width = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_WIDTH_ID")?;
    let height =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_HEIGHT_ID")?;
    let response =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_RESPONSE_ID")?;
    match (
        (width.owner, width.serializer_name, width.typed_default),
        (height.owner, height.serializer_name, height.typed_default),
        (
            response.owner,
            response.serializer_name,
            response.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.Interaction",
                "FLOAT",
                Some(VanillaEntityMetadataDefault::Float(width_default)),
            ),
            (
                "net.minecraft.world.entity.Interaction",
                "FLOAT",
                Some(VanillaEntityMetadataDefault::Float(height_default)),
            ),
            (
                "net.minecraft.world.entity.Interaction",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(response_default)),
            ),
        ) => Some(VanillaInteractionMetadataAccessors {
            width_data_id: width.data_id,
            width_default,
            height_data_id: height.data_id,
            height_default,
            response_data_id: response.data_id,
            response_default,
        }),
        _ => None,
    }
}

/// Resolves the generated Creeper fuse and powered-state accessors with typed defaults.
pub fn vanilla_entity_metadata_creeper_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCreeperMetadataAccessors> {
    let swell_dir =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SWELL_DIR")?;
    let powered =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_IS_POWERED")?;
    let ignited =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_IS_IGNITED")?;
    match (
        (
            swell_dir.owner,
            swell_dir.serializer_name,
            swell_dir.typed_default,
        ),
        (
            powered.owner,
            powered.serializer_name,
            powered.typed_default,
        ),
        (
            ignited.owner,
            ignited.serializer_name,
            ignited.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.monster.Creeper",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(swell_dir_default)),
            ),
            (
                "net.minecraft.world.entity.monster.Creeper",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(powered_default)),
            ),
            (
                "net.minecraft.world.entity.monster.Creeper",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(ignited_default)),
            ),
        ) => Some(VanillaCreeperMetadataAccessors {
            swell_dir_data_id: swell_dir.data_id,
            swell_dir_default,
            powered_data_id: powered.data_id,
            powered_default,
            ignited_data_id: ignited.data_id,
            ignited_default,
        }),
        _ => None,
    }
}

/// Resolves Panda's generated counters, genes, and flags with typed defaults.
pub fn vanilla_entity_metadata_panda_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaPandaMetadataAccessors> {
    let unhappy_counter =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "UNHAPPY_COUNTER")?;
    let sneeze_counter =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "SNEEZE_COUNTER")?;
    let eat_counter =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "EAT_COUNTER")?;
    let main_gene =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "MAIN_GENE_ID")?;
    let hidden_gene =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "HIDDEN_GENE_ID")?;
    let flags = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_FLAGS")?;
    match (
        (
            unhappy_counter.owner,
            unhappy_counter.serializer_name,
            unhappy_counter.typed_default,
        ),
        (
            sneeze_counter.owner,
            sneeze_counter.serializer_name,
            sneeze_counter.typed_default,
        ),
        (
            eat_counter.owner,
            eat_counter.serializer_name,
            eat_counter.typed_default,
        ),
        (
            main_gene.owner,
            main_gene.serializer_name,
            main_gene.typed_default,
        ),
        (
            hidden_gene.owner,
            hidden_gene.serializer_name,
            hidden_gene.typed_default,
        ),
        (flags.owner, flags.serializer_name, flags.typed_default),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.panda.Panda",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(unhappy_counter_default)),
            ),
            (
                "net.minecraft.world.entity.animal.panda.Panda",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(sneeze_counter_default)),
            ),
            (
                "net.minecraft.world.entity.animal.panda.Panda",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(eat_counter_default)),
            ),
            (
                "net.minecraft.world.entity.animal.panda.Panda",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(main_gene_default)),
            ),
            (
                "net.minecraft.world.entity.animal.panda.Panda",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(hidden_gene_default)),
            ),
            (
                "net.minecraft.world.entity.animal.panda.Panda",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(flags_default)),
            ),
        ) => {
            let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Panda);
            let mask = |name| {
                schema
                    .own_flags
                    .iter()
                    .find(|flag| flag.name == name && flag.data_id == flags.data_id)
                    .map(|flag| flag.mask)
            };
            Some(VanillaPandaMetadataAccessors {
                unhappy_counter_data_id: unhappy_counter.data_id,
                unhappy_counter_default,
                sneeze_counter_data_id: sneeze_counter.data_id,
                sneeze_counter_default,
                eat_counter_data_id: eat_counter.data_id,
                eat_counter_default,
                main_gene_data_id: main_gene.data_id,
                main_gene_default,
                hidden_gene_data_id: hidden_gene.data_id,
                hidden_gene_default,
                flags_data_id: flags.data_id,
                flags_default,
                sneezing_mask: mask("FLAG_SNEEZE")?,
                rolling_mask: mask("FLAG_ROLL")?,
                sitting_mask: mask("FLAG_SIT")?,
                on_back_mask: mask("FLAG_ON_BACK")?,
            })
        }
        _ => None,
    }
}

/// Resolves the generated Panda flags accessor and its typed default.
pub fn vanilla_entity_metadata_panda_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8)> {
    vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.flags_data_id, accessors.flags_default))
}

/// Resolves AbstractHorse's generated flags accessor, typed default, and named masks.
pub fn vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractHorseMetadataAccessors> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_FLAGS")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.equine.AbstractHorse",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(flags_default)),
        ) => {
            let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractHorse);
            let mask = |name| {
                schema
                    .own_flags
                    .iter()
                    .find(|flag| flag.name == name && flag.data_id == accessor.data_id)
                    .map(|flag| flag.mask)
            };
            Some(VanillaAbstractHorseMetadataAccessors {
                flags_data_id: accessor.data_id,
                flags_default,
                tame_mask: mask("FLAG_TAME")?,
                bred_mask: mask("FLAG_BRED")?,
                eating_mask: mask("FLAG_EATING")?,
                standing_mask: mask("FLAG_STANDING")?,
                open_mouth_mask: mask("FLAG_OPEN_MOUTH")?,
            })
        }
        _ => None,
    }
}

/// Resolves the generated AbstractHorse flags accessor and its typed default.
pub fn vanilla_entity_metadata_abstract_horse_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8)> {
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.flags_data_id, accessors.flags_default))
}

/// Resolves Horse's generated packed color/markings accessor and its typed default.
pub fn vanilla_entity_metadata_horse_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_TYPE_VARIANT")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.equine.Horse",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves AbstractChestedHorse's generated chest accessor and its typed default.
pub fn vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_CHEST")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.equine.AbstractChestedHorse",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves Llama's generated strength and variant accessors with typed defaults.
pub fn vanilla_entity_metadata_llama_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaLlamaMetadataAccessors> {
    let strength =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_STRENGTH_ID")?;
    let variant =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VARIANT_ID")?;
    match (
        (
            strength.owner,
            strength.serializer_name,
            strength.typed_default,
        ),
        (
            variant.owner,
            variant.serializer_name,
            variant.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.equine.Llama",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(strength_default)),
            ),
            (
                "net.minecraft.world.entity.animal.equine.Llama",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(variant_default)),
            ),
        ) => Some(VanillaLlamaMetadataAccessors {
            strength_data_id: strength.data_id,
            strength_default,
            variant_data_id: variant.data_id,
            variant_default,
        }),
        _ => None,
    }
}

/// Resolves Chicken/Cow/Pig synchronized presentation and sound holders with virtual defaults.
pub fn vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaFarmAnimalVariantMetadataAccessors> {
    let variant =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VARIANT_ID")?;
    let sound_variant =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SOUND_VARIANT_ID")?;
    match (
        (
            variant.owner,
            variant.serializer_name,
            variant.typed_default,
        ),
        (
            sound_variant.owner,
            sound_variant.serializer_name,
            sound_variant.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.chicken.Chicken",
                "CHICKEN_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    variant_default_resource_id,
                )),
            ),
            (
                "net.minecraft.world.entity.animal.chicken.Chicken",
                "CHICKEN_SOUND_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    sound_variant_default_resource_id,
                )),
            ),
        )
        | (
            (
                "net.minecraft.world.entity.animal.cow.Cow",
                "COW_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    variant_default_resource_id,
                )),
            ),
            (
                "net.minecraft.world.entity.animal.cow.Cow",
                "COW_SOUND_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    sound_variant_default_resource_id,
                )),
            ),
        )
        | (
            (
                "net.minecraft.world.entity.animal.pig.Pig",
                "PIG_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    variant_default_resource_id,
                )),
            ),
            (
                "net.minecraft.world.entity.animal.pig.Pig",
                "PIG_SOUND_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    sound_variant_default_resource_id,
                )),
            ),
        ) => Some(VanillaFarmAnimalVariantMetadataAccessors {
            variant_data_id: variant.data_id,
            variant_default_resource_id,
            sound_variant_data_id: sound_variant.data_id,
            sound_variant_default_resource_id,
        }),
        _ => None,
    }
}

/// Resolves Cat/Wolf synchronized presentation and sound holders with virtual defaults.
pub fn vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCompanionVariantMetadataAccessors> {
    let variant =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VARIANT_ID")?;
    let sound_variant =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SOUND_VARIANT_ID")?;
    match (
        (
            variant.owner,
            variant.serializer_name,
            variant.typed_default,
        ),
        (
            sound_variant.owner,
            sound_variant.serializer_name,
            sound_variant.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.animal.feline.Cat",
                "CAT_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    variant_default_resource_id,
                )),
            ),
            (
                "net.minecraft.world.entity.animal.feline.Cat",
                "CAT_SOUND_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    sound_variant_default_resource_id,
                )),
            ),
        )
        | (
            (
                "net.minecraft.world.entity.animal.wolf.Wolf",
                "WOLF_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    variant_default_resource_id,
                )),
            ),
            (
                "net.minecraft.world.entity.animal.wolf.Wolf",
                "WOLF_SOUND_VARIANT",
                Some(VanillaEntityMetadataDefault::RegistryHolderResource(
                    sound_variant_default_resource_id,
                )),
            ),
        ) => Some(VanillaCompanionVariantMetadataAccessors {
            variant_data_id: variant.data_id,
            variant_default_resource_id,
            sound_variant_data_id: sound_variant.data_id,
            sound_variant_default_resource_id,
        }),
        _ => None,
    }
}

/// Resolves Frog's synchronized presentation holder and virtual resource default.
pub fn vanilla_entity_metadata_frog_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, &'static str)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VARIANT_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.frog.Frog",
            "FROG_VARIANT",
            Some(VanillaEntityMetadataDefault::RegistryHolderResource(default_resource_id)),
        ) => Some((accessor.data_id, default_resource_id)),
        _ => None,
    }
}

/// Resolves ZombieNautilus's synchronized presentation holder and virtual resource default.
pub fn vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, &'static str)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_VARIANT_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.animal.nautilus.ZombieNautilus",
            "ZOMBIE_NAUTILUS_VARIANT",
            Some(VanillaEntityMetadataDefault::RegistryHolderResource(default_resource_id)),
        ) => Some((accessor.data_id, default_resource_id)),
        _ => None,
    }
}

/// Resolves the generated Mob flags accessor, typed default, and named bit masks.
pub fn vanilla_entity_metadata_mob_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaMobMetadataAccessors> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_MOB_FLAGS_ID")?;
    let flags_default = match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.Mob",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => default,
        _ => return None,
    };
    let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Mob);
    let mask = |name| {
        schema
            .own_flags
            .iter()
            .find(|flag| flag.name == name && flag.data_id == accessor.data_id)
            .map(|flag| flag.mask)
    };
    Some(VanillaMobMetadataAccessors {
        flags_data_id: accessor.data_id,
        flags_default,
        no_ai_mask: mask("MOB_FLAG_NO_AI")?,
        left_handed_mask: mask("MOB_FLAG_LEFTHANDED")?,
        aggressive_mask: mask("MOB_FLAG_AGGRESSIVE")?,
    })
}

/// Resolves the generated base Entity fields, typed defaults, and shared flag masks.
pub fn vanilla_entity_metadata_entity_accessors() -> Option<VanillaEntityMetadataAccessors> {
    let accessor = |java_name| {
        VANILLA_ENTITY_METADATA_ACCESSORS_26_1.iter().find(|entry| {
            entry.owner == "net.minecraft.world.entity.Entity" && entry.java_name == java_name
        })
    };
    let shared_flags = accessor("DATA_SHARED_FLAGS_ID")?;
    let air_supply = accessor("DATA_AIR_SUPPLY_ID")?;
    let custom_name = accessor("DATA_CUSTOM_NAME")?;
    let custom_name_visible = accessor("DATA_CUSTOM_NAME_VISIBLE")?;
    let silent = accessor("DATA_SILENT")?;
    let no_gravity = accessor("DATA_NO_GRAVITY")?;
    let pose = accessor("DATA_POSE")?;
    let ticks_frozen = accessor("DATA_TICKS_FROZEN")?;
    let (
        shared_flags_default,
        custom_name_visible_default,
        silent_default,
        no_gravity_default,
        pose_default,
        ticks_frozen_default,
    ) = match (
        (shared_flags.serializer_name, shared_flags.typed_default),
        (air_supply.serializer_name, air_supply.typed_default),
        (custom_name.serializer_name, custom_name.typed_default),
        (
            custom_name_visible.serializer_name,
            custom_name_visible.typed_default,
        ),
        (silent.serializer_name, silent.typed_default),
        (no_gravity.serializer_name, no_gravity.typed_default),
        (pose.serializer_name, pose.typed_default),
        (ticks_frozen.serializer_name, ticks_frozen.typed_default),
    ) {
        (
            ("BYTE", Some(VanillaEntityMetadataDefault::Byte(shared_flags_default))),
            ("INT", Some(VanillaEntityMetadataDefault::MaxAirSupply)),
            ("OPTIONAL_COMPONENT", Some(VanillaEntityMetadataDefault::OptionalComponentNone)),
            ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(custom_name_visible_default))),
            ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(silent_default))),
            ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(no_gravity_default))),
            ("POSE", Some(VanillaEntityMetadataDefault::Pose(pose_default))),
            ("INT", Some(VanillaEntityMetadataDefault::Int(ticks_frozen_default))),
        ) => (
            shared_flags_default,
            custom_name_visible_default,
            silent_default,
            no_gravity_default,
            pose_default,
            ticks_frozen_default,
        ),
        _ => return None,
    };
    let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Entity);
    let mask = |name| {
        schema
            .own_flags
            .iter()
            .find(|flag| flag.name == name && flag.data_id == shared_flags.data_id)
            .map(|flag| flag.mask)
    };
    Some(VanillaEntityMetadataAccessors {
        shared_flags_data_id: shared_flags.data_id,
        shared_flags_default,
        on_fire_mask: mask("FLAG_ONFIRE")?,
        shift_key_down_mask: mask("FLAG_SHIFT_KEY_DOWN")?,
        sprinting_mask: mask("FLAG_SPRINTING")?,
        swimming_mask: mask("FLAG_SWIMMING")?,
        invisible_mask: mask("FLAG_INVISIBLE")?,
        glowing_mask: mask("FLAG_GLOWING")?,
        fall_flying_mask: mask("FLAG_FALL_FLYING")?,
        air_supply_data_id: air_supply.data_id,
        custom_name_data_id: custom_name.data_id,
        custom_name_visible_data_id: custom_name_visible.data_id,
        custom_name_visible_default,
        silent_data_id: silent.data_id,
        silent_default,
        no_gravity_data_id: no_gravity.data_id,
        no_gravity_default,
        pose_data_id: pose.data_id,
        pose_default,
        ticks_frozen_data_id: ticks_frozen.data_id,
        ticks_frozen_default,
    })
}

/// Resolves generated LivingEntity flags/health/sleep fields and named bit masks.
pub fn vanilla_entity_metadata_living_entity_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaLivingEntityMetadataAccessors> {
    let flags = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_LIVING_ENTITY_FLAGS",
    )?;
    let health =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_HEALTH_ID")?;
    let arrow_count =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ARROW_COUNT_ID")?;
    let stinger_count =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_STINGER_COUNT_ID")?;
    let sleeping_pos =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "SLEEPING_POS_ID")?;
    let (flags_default, health_default, arrow_count_default, stinger_count_default) = match (
        (flags.owner, flags.serializer_name, flags.typed_default),
        (health.owner, health.serializer_name, health.typed_default),
        (
            arrow_count.owner,
            arrow_count.serializer_name,
            arrow_count.typed_default,
        ),
        (
            stinger_count.owner,
            stinger_count.serializer_name,
            stinger_count.typed_default,
        ),
        (
            sleeping_pos.owner,
            sleeping_pos.serializer_name,
            sleeping_pos.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.LivingEntity",
                "BYTE",
                Some(VanillaEntityMetadataDefault::Byte(flags_default)),
            ),
            (
                "net.minecraft.world.entity.LivingEntity",
                "FLOAT",
                Some(VanillaEntityMetadataDefault::Float(health_default)),
            ),
            (
                "net.minecraft.world.entity.LivingEntity",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(arrow_count_default)),
            ),
            (
                "net.minecraft.world.entity.LivingEntity",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(stinger_count_default)),
            ),
            (
                "net.minecraft.world.entity.LivingEntity",
                "OPTIONAL_BLOCK_POS",
                Some(VanillaEntityMetadataDefault::OptionalBlockPosNone),
            ),
        ) => (
            flags_default,
            health_default,
            arrow_count_default,
            stinger_count_default,
        ),
        _ => return None,
    };
    let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::LivingEntity);
    let mask = |name| {
        schema
            .own_flags
            .iter()
            .find(|flag| flag.name == name && flag.data_id == flags.data_id)
            .map(|flag| flag.mask)
    };
    Some(VanillaLivingEntityMetadataAccessors {
        flags_data_id: flags.data_id,
        flags_default,
        is_using_mask: mask("LIVING_ENTITY_FLAG_IS_USING")?,
        off_hand_mask: mask("LIVING_ENTITY_FLAG_OFF_HAND")?,
        spin_attack_mask: mask("LIVING_ENTITY_FLAG_SPIN_ATTACK")?,
        health_data_id: health.data_id,
        health_default,
        arrow_count_data_id: arrow_count.data_id,
        arrow_count_default,
        stinger_count_data_id: stinger_count.data_id,
        stinger_count_default,
        sleeping_pos_data_id: sleeping_pos.data_id,
    })
}

/// Resolves the generated Vex flags accessor, typed default, and named charging mask.
pub fn vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_FLAGS_ID")?;
    let charging_mask = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Vex)
        .own_flags
        .iter()
        .find(|flag| flag.name == "FLAG_IS_CHARGING" && flag.data_id == accessor.data_id)?
        .mask;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.Vex",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => Some((accessor.data_id, default, charging_mask)),
        _ => None,
    }
}

/// Resolves the inherited Raider celebrating accessor and its typed default.
pub fn vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "IS_CELEBRATING")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.raid.Raider",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the inherited SpellcasterIllager casting-spell accessor and its typed default.
pub fn vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SPELL_CASTING_ID")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.illager.SpellcasterIllager",
            "BYTE",
            Some(VanillaEntityMetadataDefault::Byte(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the inherited AbstractVillager unhappy-counter accessor and its typed default.
pub fn vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_UNHAPPY_COUNTER")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.npc.villager.AbstractVillager",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated WitherBoss invulnerability-ticks accessor and its typed default.
///
/// The accessor owner supplies both the Wither presentation membership and its wire index, so
/// world consumers do not need a parallel single-type predicate or fixed data-id import.
pub fn vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i32)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_ID_INV")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.boss.wither.WitherBoss",
            "INT",
            Some(VanillaEntityMetadataDefault::Int(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the generated WitherBoss side-head target accessors and their typed defaults.
pub(crate) fn vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaWitherSideHeadTargetMetadataAccessors> {
    let target_b =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TARGET_B")?;
    let target_c =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_TARGET_C")?;
    match (
        (
            target_b.owner,
            target_b.serializer_name,
            target_b.typed_default,
        ),
        (
            target_c.owner,
            target_c.serializer_name,
            target_c.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.boss.wither.WitherBoss",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(target_b_default)),
            ),
            (
                "net.minecraft.world.entity.boss.wither.WitherBoss",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(target_c_default)),
            ),
        ) => Some(VanillaWitherSideHeadTargetMetadataAccessors {
            target_b_data_id: target_b.data_id,
            target_b_default,
            target_c_data_id: target_c.data_id,
            target_c_default,
        }),
        _ => None,
    }
}

/// Resolves the nearest generated boolean `DATA_BABY_ID` accessor and its typed default.
///
/// Vanilla declares this accessor independently on `AgeableMob`, `Zombie`, `Piglin`, and
/// `Zoglin`; walking the generated schema hierarchy preserves their distinct wire indices without
/// maintaining a parallel entity-type membership table.
pub fn vanilla_entity_metadata_baby_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_BABY_ID")?;
    match (accessor.serializer_name, accessor.typed_default) {
        ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(default))) => {
            Some((accessor.data_id, default))
        }
        _ => None,
    }
}

/// Returns source-derived `isBaby` dispatch and the exact baby `getAgeScale` value for a living
/// runtime type. Non-living and unknown ids have no living age-scale state.
pub(crate) fn vanilla_entity_metadata_baby_behavior_and_age_scale(
    entity_type_id: i32,
) -> Option<(VanillaEntityBabyBehavior, f32)> {
    let entry = VANILLA_ENTITY_TYPE_METADATA_26_1.get(usize::try_from(entity_type_id).ok()?)?;
    entry.is_living.then_some((
        entry.baby_behavior,
        f32::from_bits(entry.baby_age_scale_bits),
    ))
}

/// Resolves the nearest generated boolean `AGE_LOCKED` accessor and its typed default.
///
/// Vanilla declares this field on `AgeableMob` and independently on `Tadpole`; walking the
/// generated schema hierarchy preserves both owners without a parallel entity-type table.
pub fn vanilla_entity_metadata_age_locked_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "AGE_LOCKED")?;
    match (accessor.serializer_name, accessor.typed_default) {
        ("BOOLEAN", Some(VanillaEntityMetadataDefault::Boolean(default))) => {
            Some((accessor.data_id, default))
        }
        _ => None,
    }
}

/// Resolves the generated Zombie baby, special-type and drowned-conversion accessors and defaults.
pub fn vanilla_entity_metadata_zombie_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaZombieMetadataAccessors> {
    let baby = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_BABY_ID")?;
    let special_type =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_SPECIAL_TYPE_ID")?;
    let drowned_conversion = vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        "DATA_DROWNED_CONVERSION_ID",
    )?;
    match (
        (baby.owner, baby.serializer_name, baby.typed_default),
        (
            special_type.owner,
            special_type.serializer_name,
            special_type.typed_default,
        ),
        (
            drowned_conversion.owner,
            drowned_conversion.serializer_name,
            drowned_conversion.typed_default,
        ),
    ) {
        (
            (
                "net.minecraft.world.entity.monster.zombie.Zombie",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(baby_default)),
            ),
            (
                "net.minecraft.world.entity.monster.zombie.Zombie",
                "INT",
                Some(VanillaEntityMetadataDefault::Int(special_type_default)),
            ),
            (
                "net.minecraft.world.entity.monster.zombie.Zombie",
                "BOOLEAN",
                Some(VanillaEntityMetadataDefault::Boolean(drowned_conversion_default)),
            ),
        ) => Some(VanillaZombieMetadataAccessors {
            baby_data_id: baby.data_id,
            baby_default,
            special_type_data_id: special_type.data_id,
            special_type_default,
            drowned_conversion_data_id: drowned_conversion.data_id,
            drowned_conversion_default,
        }),
        _ => None,
    }
}

/// Resolves the generated Piglin dancing accessor and its typed default.
pub fn vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    let accessor =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_IS_DANCING")?;
    match (
        accessor.owner,
        accessor.serializer_name,
        accessor.typed_default,
    ) {
        (
            "net.minecraft.world.entity.monster.piglin.Piglin",
            "BOOLEAN",
            Some(VanillaEntityMetadataDefault::Boolean(default)),
        ) => Some((accessor.data_id, default)),
        _ => None,
    }
}

/// Resolves the single accessor with `serializer_name` along an entity type's generated schema
/// inheritance chain. Missing and ambiguous matches both return `None`, so consumers cannot silently
/// choose one of several same-serializer fields.
pub fn vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
    entity_type_id: i32,
    serializer_name: &str,
) -> Option<&'static VanillaEntityMetadataAccessorCatalogEntry> {
    let mut java_class = vanilla_entity_type_metadata_entry(entity_type_id)?.schema_class?;
    let mut resolved = None;
    loop {
        for accessor in vanilla_entity_metadata_accessors_on_class(java_class)
            .iter()
            .filter(|entry| entry.serializer_name == serializer_name)
        {
            if resolved.replace(accessor).is_some() {
                return None;
            }
        }
        let Some(superclass) =
            vanilla_entity_metadata_class(java_class).and_then(|entry| entry.superclass)
        else {
            return resolved;
        };
        java_class = superclass;
    }
}

/// Returns whether the generated vanilla runtime class inherits `LivingEntity`.
///
/// This is class/inheritance catalog data only; living-entity behavior remains owned by the
/// world client simulation and presentation modules.
pub fn vanilla_entity_metadata_type_is_living(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_living)
        .unwrap_or(false)
}

/// Returns whether the generated vanilla runtime class inherits `Mob`.
pub fn vanilla_entity_metadata_type_is_mob(entity_type_id: i32) -> bool {
    const MOB: &str = "net.minecraft.world.entity.Mob";
    let Some(runtime_class) = VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .and_then(|entry| entry.runtime_class)
    else {
        return false;
    };
    vanilla_class_is_or_implements(runtime_class, MOB, 0)
}

/// Returns the nearest generated vanilla `handleEntityEvent(byte)` declaration owner.
///
/// Event ids and their client-side effects remain behavior owned by the world/runtime paths;
/// this query only exposes source-derived subclass dispatch membership.
pub fn vanilla_entity_metadata_handle_entity_event_owner(
    entity_type_id: i32,
) -> Option<&'static str> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .and_then(|entry| entry.handle_entity_event_owner)
}

pub(crate) fn vanilla_entity_metadata_passenger_attachment_owner(
    entity_type_id: i32,
) -> Option<&'static str> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .and_then(|entry| entry.passenger_attachment_owner)
}

pub(crate) fn vanilla_entity_metadata_vehicle_attachment_owner(
    entity_type_id: i32,
) -> Option<&'static str> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .and_then(|entry| entry.vehicle_attachment_owner)
}

pub(crate) fn vanilla_entity_metadata_controlling_passenger_owner(
    entity_type_id: i32,
) -> Option<&'static str> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .and_then(|entry| entry.controlling_passenger_owner)
}

pub(crate) fn vanilla_entity_metadata_leash_offset_owner(
    entity_type_id: i32,
) -> Option<&'static str> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .and_then(|entry| entry.leash_offset_owner)
}

pub(crate) fn vanilla_entity_metadata_rope_hold_position_owner(
    entity_type_id: i32,
) -> Option<&'static str> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .and_then(|entry| entry.rope_hold_position_owner)
}

/// Resolves the vanilla declaration that consumes one entity event after following the
/// source-generated `super.handleEntityEvent(id)` chain. Listener-special ids (`21`, `35`, and
/// `63`) are intercepted by `ClientPacketListener` before this class dispatch and therefore do
/// not resolve here.
pub(crate) fn vanilla_entity_metadata_handle_entity_event_owner_for_event(
    entity_type_id: i32,
    event_id: u8,
) -> Option<&'static str> {
    let mut owner = vanilla_entity_metadata_handle_entity_event_owner(entity_type_id)?;
    loop {
        let handler = VANILLA_ENTITY_EVENT_HANDLERS_26_1
            .iter()
            .find(|handler| handler.owner == owner)?;
        if handler.direct_event_ids.contains(&event_id)
            || handler.delegated_event_ids.contains(&event_id)
        {
            return Some(owner);
        }
        if !handler.forwards_unhandled_to_super {
            return None;
        }
        owner = handler.super_handler_owner?;
    }
}

/// Returns whether entity event dispatch reaches `Ocelot.handleEntityEvent(byte)` first.
pub(crate) fn vanilla_entity_metadata_type_uses_ocelot_entity_event_handler(
    entity_type_id: i32,
) -> bool {
    vanilla_entity_metadata_handle_entity_event_owner(entity_type_id)
        == Some("net.minecraft.world.entity.animal.feline.Ocelot")
}

/// Returns whether entity event dispatch reaches `FishingHook.handleEntityEvent(byte)` first.
pub(crate) fn vanilla_entity_metadata_type_uses_fishing_hook_entity_event_handler(
    entity_type_id: i32,
) -> bool {
    vanilla_entity_metadata_handle_entity_event_owner(entity_type_id)
        == Some("net.minecraft.world.entity.projectile.FishingHook")
}

/// Returns whether entity event dispatch reaches `Player.handleEntityEvent(byte)` first.
pub(crate) fn vanilla_entity_metadata_type_uses_player_entity_event_handler(
    entity_type_id: i32,
) -> bool {
    vanilla_entity_metadata_handle_entity_event_owner(entity_type_id)
        == Some("net.minecraft.world.entity.player.Player")
}

/// Returns whether vanilla's generated class hierarchy and entity-type tags make this type
/// eligible for `Entity.isPickable` before per-instance state gates.
///
/// The generated class flag interprets the nearest `isPickable` override. Redirectable
/// projectiles are composed from the generated registry tag catalog; the world query separately
/// handles removed entities, spectator players, and marker armor stands.
pub fn vanilla_entity_metadata_type_is_pickable(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_pickable_by_class)
        .unwrap_or(false)
        || bbb_protocol::entity_types::vanilla_entity_type_is_redirectable_projectile(
            entity_type_id,
        )
}

/// Returns the nearest generated `Entity.getSoundSource()` class behavior.
///
/// `Dynamic` currently identifies the evil-rabbit variant override; callers without entity
/// metadata must preserve their explicit fallback rather than treating it as a fixed category.
pub fn vanilla_entity_metadata_type_sound_source_kind(
    entity_type_id: i32,
) -> Option<VanillaEntitySoundSourceKind> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .map(|entry| entry.sound_source_kind)
}

/// Returns the generated nearest-`getDeathSound` policy for one fixed EntityType.
pub fn vanilla_entity_metadata_type_death_sound_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityDeathSoundPolicy> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .map(|entry| entry.death_sound_policy)
}

/// Returns the generated nearest-`getHurtSound` policy for one fixed EntityType.
pub fn vanilla_entity_metadata_type_hurt_sound_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityHurtSoundPolicy> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .map(|entry| entry.hurt_sound_policy)
}

/// Returns the generated nearest-`getSoundVolume` policy for one fixed EntityType.
pub fn vanilla_entity_metadata_type_sound_volume_policy(
    entity_type_id: i32,
) -> Option<VanillaEntitySoundVolumePolicy> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .map(|entry| entry.sound_volume_policy)
}

/// Returns the nearest generated `Entity.isPushable()` class behavior.
///
/// Dynamic variants retain only the class-level predicate shape here. Health, spectator,
/// climbable, passenger, pose, and synchronized-metadata state remain caller-owned.
pub fn vanilla_entity_metadata_type_pushability_kind(
    entity_type_id: i32,
) -> Option<VanillaEntityPushabilityKind> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .map(|entry| entry.pushability_kind)
}

/// Returns the nearest generated `Entity.canBeCollidedWith(other)` class behavior.
///
/// `Alive` and `HappyGhast` retain their instance predicates for the world collision owner.
pub fn vanilla_entity_metadata_type_collidability_kind(
    entity_type_id: i32,
) -> Option<VanillaEntityCollidabilityKind> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .map(|entry| entry.collidability_kind)
}

/// Returns the post-chain resource selected by vanilla `GameRenderer` for a camera entity type.
///
/// The checked-in table is extracted from `GameRenderer.checkEntityPostEffect` and expanded over
/// the generated entity class hierarchy, so subclasses such as CaveSpider inherit Spider's chain.
pub fn vanilla_entity_camera_post_effect_id(entity_type_id: i32) -> Option<&'static str> {
    VANILLA_ENTITY_CAMERA_POST_EFFECTS_26_1
        .binary_search_by_key(&entity_type_id, |(candidate, _)| *candidate)
        .ok()
        .map(|index| VANILLA_ENTITY_CAMERA_POST_EFFECTS_26_1[index].1)
}

/// Returns the nearest non-identity vanilla `LivingEntity.sanitizeScale` override.
pub fn vanilla_entity_scale_sanitizer(entity_type_id: i32) -> Option<VanillaEntityScaleSanitizer> {
    VANILLA_ENTITY_SCALE_SANITIZERS_26_1
        .binary_search_by_key(&entity_type_id, |(candidate, _)| *candidate)
        .ok()
        .map(|index| VANILLA_ENTITY_SCALE_SANITIZERS_26_1[index].1)
}

/// Returns whether the generated vanilla runtime class implements `Leashable`,
/// directly or through its superclass/interface graph.
///
/// This is extracted Java type-relationship data only. Client leash packet
/// application remains owned by the entity update path.
pub fn vanilla_entity_metadata_type_is_leashable(entity_type_id: i32) -> bool {
    const LEASHABLE: &str = "net.minecraft.world.entity.Leashable";
    let Some(runtime_class) = VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .and_then(|entry| entry.runtime_class)
    else {
        return false;
    };
    vanilla_class_is_or_implements(runtime_class, LEASHABLE, 0)
}

/// Returns whether the generated vanilla runtime class implements `HoglinBase`.
///
/// Vanilla uses this interface for the shared Hoglin/Zoglin attack-animation timer exposed to
/// their renderer. Entity-event application and client-tick advancement remain owned by the world
/// animation state.
pub(crate) fn vanilla_entity_metadata_type_is_hoglin_base(entity_type_id: i32) -> bool {
    const HOGLIN_BASE: &str = "net.minecraft.world.entity.monster.hoglin.HoglinBase";
    let Some(runtime_class) = VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .and_then(|entry| entry.runtime_class)
    else {
        return false;
    };
    vanilla_class_is_or_implements(runtime_class, HOGLIN_BASE, 0)
}

fn vanilla_class_is_or_implements(java_class: &str, target: &str, depth: usize) -> bool {
    if java_class == target {
        return true;
    }
    if depth >= VANILLA_ENTITY_METADATA_CLASSES_26_1.len() {
        return false;
    }
    let Some(class) = vanilla_entity_metadata_class(java_class) else {
        return false;
    };
    class
        .interfaces
        .iter()
        .any(|interface| vanilla_class_is_or_implements(interface, target, depth + 1))
        || class
            .superclass
            .is_some_and(|superclass| vanilla_class_is_or_implements(superclass, target, depth + 1))
}

/// Returns whether the generated vanilla runtime class inherits `Animal`.
pub fn vanilla_entity_metadata_type_is_animal(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_animal)
        .unwrap_or(false)
}

/// Returns whether the generated vanilla runtime class inherits `TamableAnimal`.
pub fn vanilla_entity_metadata_type_is_tamable_animal(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_tamable_animal)
        .unwrap_or(false)
}

/// Returns whether the generated vanilla runtime class inherits `Projectile`.
pub fn vanilla_entity_metadata_type_is_projectile(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_projectile)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_hurting_projectile(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_hurting_projectile)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_wind_charge(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_wind_charge)
        .unwrap_or(false)
}

pub(crate) fn vanilla_entity_metadata_type_is_vehicle(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_vehicle)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_boat(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_boat)
        .unwrap_or(false)
}

/// Returns the generated concrete boat `rideHeight(EntityDimensions)` multiplier.
pub fn vanilla_entity_metadata_type_boat_ride_height_multiplier(
    entity_type_id: i32,
) -> Option<f32> {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).ok()?)
        .and_then(|entry| entry.boat_ride_height_multiplier_bits)
        .map(f32::from_bits)
}

/// Returns whether the nearest generated `LivingEntity.canUseSlot` implementation is constant true.
pub fn vanilla_entity_metadata_type_can_use_every_equipment_slot(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.can_use_every_equipment_slot)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_abstract_chest_boat(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_abstract_chest_boat)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_minecart(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_minecart)
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityInterpolationPolicy {
    DirectSetPos,
    GenericThreeStep,
    DisplayImmediate,
    MinecartFeatureGated,
    ShulkerAttachPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityClientMovementPolicy {
    ServerSynchronized,
    Vehicle,
    Item,
    ExperienceOrb,
    FallingBlock,
    PrimedTnt,
    ThrowableProjectile,
    ThrownExperienceBottle,
    ThrownPotion,
    AbstractArrow,
    HurtingProjectile,
    WindCharge,
    DragonFireball,
    WitherSkull,
    LlamaSpit,
    EyeOfEnder,
    FireworkRocket,
    FishingHook,
    ShulkerBullet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityBlockEffectTickPolicy {
    /// No client tick callback. Server movement/interpolation remains authoritative.
    None,
    /// `LivingEntity.aiStep`: only the local authoritative instance executes it.
    LocalAuthoritativeLiving,
    /// One ordinary callback after this entity's client-owned movement.
    OnceAfterMovement,
    /// `ItemEntity` may reuse the last applied movement list while resting.
    ItemWithLastMovements,
    /// `AbstractArrow` can apply once at rest or once per explicit hit segment.
    AbstractArrowSegments,
    /// `FireworkRocketEntity` applies only while it is not attached to an entity.
    DetachedFirework,
    /// `AbstractBoat.tick` deliberately invokes the callback twice.
    BoatTwice,
}

pub(crate) fn vanilla_entity_metadata_type_client_movement_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityClientMovementPolicy> {
    let entry = VANILLA_ENTITY_TYPE_METADATA_26_1.get(usize::try_from(entity_type_id).ok()?)?;
    Some(entry.client_movement_policy)
}

pub fn vanilla_entity_metadata_type_block_effect_tick_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityBlockEffectTickPolicy> {
    let entry = VANILLA_ENTITY_TYPE_METADATA_26_1.get(usize::try_from(entity_type_id).ok()?)?;
    Some(entry.block_effect_tick_policy)
}

pub(crate) fn vanilla_entity_metadata_type_interpolation_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityInterpolationPolicy> {
    let entry = VANILLA_ENTITY_TYPE_METADATA_26_1.get(usize::try_from(entity_type_id).ok()?)?;
    Some(entry.interpolation_policy)
}

/// Returns the vanilla client interpolation-handler length for an entity type.
///
/// The membership follows the generated 26.1 runtime-class graph rather than a
/// hand-maintained registry-id list. `Shulker` overrides the inherited
/// `LivingEntity` handler with `null`; displays keep their explicit zero-length
/// handler; and minecarts expose the old-behavior handler only while the
/// `minecart_improvements` feature is disabled.
pub(crate) fn vanilla_entity_metadata_type_interpolation_steps(
    entity_type_id: i32,
    minecart_improvements_enabled: bool,
) -> Option<u8> {
    match vanilla_entity_metadata_type_interpolation_policy(entity_type_id)? {
        VanillaEntityInterpolationPolicy::GenericThreeStep => Some(3),
        VanillaEntityInterpolationPolicy::DisplayImmediate => Some(0),
        VanillaEntityInterpolationPolicy::MinecartFeatureGated
            if !minecart_improvements_enabled =>
        {
            Some(3)
        }
        VanillaEntityInterpolationPolicy::DirectSetPos
        | VanillaEntityInterpolationPolicy::MinecartFeatureGated
        | VanillaEntityInterpolationPolicy::ShulkerAttachPosition => None,
    }
}

pub fn vanilla_entity_metadata_type_is_abstract_horse(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_abstract_horse)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_abstract_chested_horse(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_abstract_chested_horse)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_abstract_nautilus(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_abstract_nautilus)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_llama(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_llama)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_item_frame(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_item_frame)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_guardian(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_guardian)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_flying_animal(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_flying_animal)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_camel(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_camel)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_squid(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_squid)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_slime(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_slime)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_abstract_piglin(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_abstract_piglin)
        .unwrap_or(false)
}

/// Returns whether the generated metadata accessor hierarchy inherits
/// `SpellcasterIllager` and therefore owns `DATA_SPELL_CASTING_ID`.
pub fn vanilla_entity_metadata_type_is_spellcaster_illager(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(entity_type_id).is_some()
}

pub fn vanilla_entity_metadata_type_is_spider(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_spider)
        .unwrap_or(false)
}

/// Returns whether the nearest generated vanilla illager `getArmPose` implementation can return
/// `IllagerArmPose.CELEBRATING`.
pub fn vanilla_entity_metadata_type_illager_arm_pose_uses_celebrating(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.illager_arm_pose_uses_celebrating)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_is_player_rideable_jumping(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_player_rideable_jumping)
        .unwrap_or(false)
}

pub fn vanilla_entity_metadata_type_has_custom_inventory_screen(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.has_custom_inventory_screen)
        .unwrap_or(false)
}

pub fn vanilla_minecart_default_display_offset(entity_type_id: i32) -> Option<i32> {
    VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1
        .binary_search_by_key(&entity_type_id, |(candidate, _)| *candidate)
        .ok()
        .map(|index| VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1[index].1)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
        VANILLA_ATTRIBUTE_MAX_HEALTH_ID, VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID,
    };
    use bbb_protocol::entity_data_serializers::{
        vanilla_entity_data_serializer_for_constant_name, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    };
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ALLAY_ID,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_ARROW_ID,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID, VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_BEE_ID,
        VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_BOGGED_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID,
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREAKING_ID,
        VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_DROWNED_ID,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_FROG_ID,
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_GHAST_ID,
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_GOAT_ID,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_HUSK_ID,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID, VANILLA_ENTITY_TYPE_INTERACTION_ID,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID, VANILLA_ENTITY_TYPE_MULE_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
        VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARCHED_ID,
        VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
        VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_SALMON_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_SKELETON_ID,
        VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
        VANILLA_ENTITY_TYPE_SPIDER_ID, VANILLA_ENTITY_TYPE_STRAY_ID,
        VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_TNT_ID,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        VANILLA_ENTITY_TYPE_TRIDENT_ID, VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        VANILLA_ENTITY_TYPE_WARDEN_ID, VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
        VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
    };

    use super::*;

    #[test]
    fn generated_default_attribute_membership_is_entity_specific() {
        assert!(vanilla_entity_type_has_attribute(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
        ));
        assert!(!vanilla_entity_type_has_attribute(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
        ));
        assert!(vanilla_entity_type_has_attribute(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID,
        ));
        assert!(vanilla_entity_type_has_attribute(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        ));
        assert!(!vanilla_entity_type_has_attribute(
            VANILLA_ENTITY_TYPE_ITEM_ID,
            VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        ));
        assert!(!vanilla_entity_type_has_attribute(
            -1,
            VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        ));
        assert_eq!(
            vanilla_entity_type_default_attribute(
                VANILLA_ENTITY_TYPE_TURTLE_ID,
                VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            )
            .and_then(|entry| entry.base_value_override_bits)
            .map(f64::from_bits),
            Some(30.0),
        );
        assert_eq!(
            vanilla_entity_type_default_attribute(
                VANILLA_ENTITY_TYPE_TURTLE_ID,
                VANILLA_ATTRIBUTE_ARMOR_ID,
            )
            .map(|entry| entry.base_value_override_bits),
            Some(None),
        );
        assert!(vanilla_entity_type_default_attribute(
            VANILLA_ENTITY_TYPE_ITEM_ID,
            VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        )
        .is_none());
    }

    #[test]
    fn entity_store_default_attribute_query_uses_only_generated_catalogs() {
        let store_source = include_str!("store.rs");
        let function = store_source
            .split("fn vanilla_default_attribute_base")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("default attribute query source");

        assert!(function.contains("vanilla_entity_type_default_attribute"));
        assert!(function.contains("vanilla_attribute_type"));
        assert!(!function.contains("match attribute_id"));
        assert!(!function.contains("VANILLA_ENTITY_TYPE_TURTLE_ID"));
    }

    #[test]
    fn generated_accessor_catalog_is_complete_unique_and_default_closed() {
        assert_eq!(VANILLA_ENTITY_METADATA_ACCESSORS_26_1.len(), 227);
        let mut keys = HashSet::new();
        for entry in VANILLA_ENTITY_METADATA_ACCESSORS_26_1 {
            assert!(keys.insert((entry.owner, entry.java_name)));
            assert!(!entry.default_expression.is_empty());
            assert_eq!(
                vanilla_entity_data_serializer_for_constant_name(entry.serializer_name)
                    .map(|serializer| serializer.id),
                Some(entry.serializer_id)
            );
        }
    }

    #[test]
    fn generated_accessor_data_id_lookup_follows_entity_inheritance() {
        let base = vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity accessors must resolve");
        for entity_type in VANILLA_ENTITY_TYPE_METADATA_26_1 {
            let inherited =
                &vanilla_entity_metadata_accessor_index()[entity_type.entity_type_id as usize];
            let mut data_ids = HashSet::new();
            for accessor in inherited {
                assert!(
                    data_ids.insert(accessor.data_id),
                    "{} inherits duplicate metadata data id {}",
                    entity_type.resource_id,
                    accessor.data_id
                );
            }
            assert_eq!(
                vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
                    entity_type.entity_type_id,
                    base.shared_flags_data_id,
                )
                .map(|accessor| accessor.java_name),
                Some("DATA_SHARED_FLAGS_ID"),
                "{} must inherit the base Entity schema",
                entity_type.resource_id
            );
        }
        let player = vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            base.shared_flags_data_id,
        )
        .expect("Player must inherit the base shared-flags accessor");
        assert_eq!(player.owner, "net.minecraft.world.entity.Entity");
        assert_eq!(player.serializer_id, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID);

        let player_absorption = vanilla_entity_metadata_player_absorption_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        )
        .expect("generated Player absorption accessor must resolve");
        assert_eq!(
            vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                player_absorption.0,
            )
            .map(|accessor| accessor.serializer_id),
            Some(VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID)
        );
        assert!(
            vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
                VANILLA_ENTITY_TYPE_ITEM_ID,
                player_absorption.0,
            )
            .is_none()
        );
        assert!(
            vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                u8::MAX - 1,
            )
            .is_none()
        );
    }

    #[test]
    fn generated_catalog_order_supports_narrow_runtime_lookup() {
        assert!(VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .enumerate()
            .all(|(index, entry)| entry.entity_type_id == index as i32));
        assert!(VANILLA_ENTITY_METADATA_CLASSES_26_1
            .windows(2)
            .all(|entries| entries[0].java_class < entries[1].java_class));
        assert!(VANILLA_ENTITY_METADATA_ACCESSORS_26_1
            .windows(2)
            .all(|entries| entries[0].owner <= entries[1].owner));
    }

    #[test]
    fn derived_accessor_index_matches_generated_inheritance_walk() {
        fn direct_lookup(
            entity_type_id: i32,
            java_name: &str,
        ) -> Option<&'static VanillaEntityMetadataAccessorCatalogEntry> {
            let mut java_class = vanilla_entity_type_metadata_entry(entity_type_id)?.schema_class?;
            loop {
                if let Some(accessor) = vanilla_entity_metadata_accessors_on_class(java_class)
                    .iter()
                    .find(|entry| entry.java_name == java_name)
                {
                    return Some(accessor);
                }
                java_class = vanilla_entity_metadata_class(java_class)?.superclass?;
            }
        }

        for entity_type in VANILLA_ENTITY_TYPE_METADATA_26_1 {
            for java_name in VANILLA_ENTITY_METADATA_ACCESSORS_26_1
                .iter()
                .map(|entry| entry.java_name)
                .chain(["__missing_generated_accessor__"])
            {
                let direct = direct_lookup(entity_type.entity_type_id, java_name)
                    .map(|entry| (entry.owner, entry.java_name, entry.data_id));
                let indexed = vanilla_entity_metadata_accessor_for_entity_type(
                    entity_type.entity_type_id,
                    java_name,
                )
                .map(|entry| (entry.owner, entry.java_name, entry.data_id));
                assert_eq!(indexed, direct, "{} {java_name}", entity_type.resource_id);
            }
        }
        assert!(vanilla_entity_metadata_accessor_for_entity_type(-1, "DATA_POSE").is_none());
        assert!(vanilla_entity_metadata_accessor_for_entity_type(i32::MAX, "DATA_POSE").is_none());
    }

    #[test]
    fn generated_class_inheritance_computes_contiguous_indices_and_sibling_reuse() {
        let classes = VANILLA_ENTITY_METADATA_CLASSES_26_1
            .iter()
            .map(|entry| (entry.java_class, entry))
            .collect::<HashMap<_, _>>();
        assert_eq!(classes.len(), VANILLA_ENTITY_METADATA_CLASSES_26_1.len());
        for class in VANILLA_ENTITY_METADATA_CLASSES_26_1 {
            for interface in class.interfaces {
                assert!(classes.contains_key(interface));
            }
        }

        let mut by_owner: HashMap<&str, Vec<&VanillaEntityMetadataAccessorCatalogEntry>> =
            HashMap::new();
        for entry in VANILLA_ENTITY_METADATA_ACCESSORS_26_1 {
            by_owner.entry(entry.owner).or_default().push(entry);
        }
        for (owner, fields) in &mut by_owner {
            fields.sort_by_key(|field| field.declaration_order);
            let class = classes[owner];
            assert_eq!(class.own_accessor_count as usize, fields.len());
            for (order, field) in fields.iter().enumerate() {
                assert_eq!(field.declaration_order as usize, order);
                assert_eq!(
                    field.data_id as u16,
                    class.inherited_accessor_count + order as u16
                );
            }
        }
    }

    #[test]
    fn generated_entity_event_handler_catalog_closes_every_vanilla_declaration() {
        const PLAYER: &str = "net.minecraft.world.entity.player.Player";
        const LIVING_ENTITY: &str = "net.minecraft.world.entity.LivingEntity";
        const MINECART_SPAWNER: &str =
            "net.minecraft.world.entity.vehicle.minecart.MinecartSpawner";
        let classes = VANILLA_ENTITY_METADATA_CLASSES_26_1
            .iter()
            .map(|entry| entry.java_class)
            .collect::<HashSet<_>>();
        let mut owners = HashSet::new();
        for handler in VANILLA_ENTITY_EVENT_HANDLERS_26_1 {
            assert!(owners.insert(handler.owner));
            assert!(classes.contains(handler.owner));
            assert!(handler
                .super_handler_owner
                .is_none_or(|owner| owners.contains(owner) || classes.contains(owner)));
            assert!(!handler.source_path.is_empty());
            assert!(
                !handler.direct_event_ids.is_empty() || !handler.delegated_event_ids.is_empty()
            );
            assert_eq!(
                handler.delegates_to.is_some(),
                !handler.delegated_event_ids.is_empty()
            );
            let mut event_ids = HashSet::new();
            assert!(handler
                .direct_event_ids
                .iter()
                .chain(handler.delegated_event_ids)
                .all(|event_id| event_ids.insert(*event_id)));
        }
        assert_eq!(owners.len(), 35);
        for handler in VANILLA_ENTITY_EVENT_HANDLERS_26_1 {
            assert!(handler
                .super_handler_owner
                .is_none_or(|owner| owners.contains(owner)));
        }
        for owner in VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .filter_map(|entry| entry.handle_entity_event_owner)
        {
            assert!(owners.contains(owner));
        }

        assert_eq!(
            VANILLA_ENTITY_EVENT_HANDLERS_26_1
                .iter()
                .find(|handler| handler.owner == PLAYER)
                .unwrap()
                .direct_event_ids,
            &[9, 23, 22]
        );
        assert_eq!(
            VANILLA_ENTITY_EVENT_HANDLERS_26_1
                .iter()
                .find(|handler| handler.owner == LIVING_ENTITY)
                .unwrap()
                .direct_event_ids,
            &[2, 3, 46, 47, 48, 49, 50, 51, 52, 54, 55, 60, 65, 67, 68]
        );
        let minecart_spawner = VANILLA_ENTITY_EVENT_HANDLERS_26_1
            .iter()
            .find(|handler| handler.owner == MINECART_SPAWNER)
            .unwrap();
        assert!(minecart_spawner.direct_event_ids.is_empty());
        assert_eq!(minecart_spawner.delegated_event_ids, &[1]);
        assert_eq!(
            minecart_spawner.delegates_to,
            Some("net.minecraft.world.level.BaseSpawner.onEventTriggered")
        );

        let handlers_without_super_fallback = VANILLA_ENTITY_EVENT_HANDLERS_26_1
            .iter()
            .filter(|handler| !handler.forwards_unhandled_to_super)
            .map(|handler| handler.owner)
            .collect::<HashSet<_>>();
        assert_eq!(
            handlers_without_super_fallback,
            HashSet::from([
                "net.minecraft.world.entity.Entity",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.Snowball",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrownEgg",
                MINECART_SPAWNER,
            ])
        );
    }

    #[test]
    fn generated_entity_event_dispatch_follows_super_and_non_forwarding_handlers() {
        const ENTITY: &str = "net.minecraft.world.entity.Entity";
        const LIVING_ENTITY: &str = "net.minecraft.world.entity.LivingEntity";
        const ANIMAL: &str = "net.minecraft.world.entity.animal.Animal";
        const TAMABLE_ANIMAL: &str = "net.minecraft.world.entity.TamableAnimal";
        const ARROW: &str = "net.minecraft.world.entity.projectile.arrow.Arrow";
        const SNOWBALL: &str =
            "net.minecraft.world.entity.projectile.throwableitemprojectile.Snowball";
        const MINECART_SPAWNER: &str =
            "net.minecraft.world.entity.vehicle.minecart.MinecartSpawner";

        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_CAT_ID,
                7,
            ),
            Some(TAMABLE_ANIMAL)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_CAT_ID,
                18,
            ),
            Some(ANIMAL)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                3,
            ),
            Some(LIVING_ENTITY)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_ARROW_ID,
                0,
            ),
            Some(ARROW)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_ARROW_ID,
                53,
            ),
            Some(ENTITY)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_SNOWBALL_ID,
                3,
            ),
            Some(SNOWBALL)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_SNOWBALL_ID,
                53,
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
                1,
            ),
            Some(MINECART_SPAWNER)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner_for_event(
                VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
                53,
            ),
            None
        );
    }

    #[test]
    fn generated_class_graph_resolves_leashable_capability() {
        assert!(vanilla_entity_metadata_type_is_leashable(
            VANILLA_ENTITY_TYPE_CHICKEN_ID
        ));
        assert!(vanilla_entity_metadata_type_is_leashable(
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_leashable(
            VANILLA_ENTITY_TYPE_MINECART_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_leashable(
            VANILLA_ENTITY_TYPE_PLAYER_ID
        ));
        assert!(vanilla_entity_metadata_type_is_leashable(
            VANILLA_ENTITY_TYPE_TURTLE_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_leashable(-1));
        assert!(!vanilla_entity_metadata_type_is_leashable(157));
    }

    #[test]
    fn generated_class_graph_resolves_mob_event_membership() {
        assert!(vanilla_entity_metadata_type_is_mob(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID
        ));
        assert!(vanilla_entity_metadata_type_is_mob(
            VANILLA_ENTITY_TYPE_OCELOT_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_mob(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_mob(
            VANILLA_ENTITY_TYPE_PLAYER_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_mob(-1));
        assert!(!vanilla_entity_metadata_type_is_mob(
            VANILLA_ENTITY_TYPE_METADATA_26_1.len() as i32
        ));
    }

    #[test]
    fn generated_class_graph_resolves_interpolation_handler_membership() {
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_type_interpolation_steps(entity_type_id, false),
                Some(3)
            );
        }
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_steps(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
                false
            ),
            Some(0)
        );
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_steps(VANILLA_ENTITY_TYPE_SHULKER_ID, false),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_steps(VANILLA_ENTITY_TYPE_ITEM_ID, false),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_steps(
                VANILLA_ENTITY_TYPE_MINECART_ID,
                false
            ),
            Some(3)
        );
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_steps(VANILLA_ENTITY_TYPE_MINECART_ID, true),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_steps(-1, false),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_steps(
                VANILLA_ENTITY_TYPE_METADATA_26_1.len() as i32,
                false
            ),
            None
        );
    }

    #[test]
    fn generated_transform_policy_partitions_all_fixed_entity_types_once() {
        let mut ids = HashSet::new();
        let mut counts = [0usize; 5];
        for entry in VANILLA_ENTITY_TYPES {
            assert!(ids.insert(entry.id), "duplicate EntityType id {}", entry.id);
            let policy = vanilla_entity_metadata_type_interpolation_policy(entry.id)
                .unwrap_or_else(|| panic!("missing transform policy for {}", entry.resource_id));
            let (index, legacy_steps, improved_steps) = match policy {
                VanillaEntityInterpolationPolicy::DirectSetPos => (0, None, None),
                VanillaEntityInterpolationPolicy::GenericThreeStep => (1, Some(3), Some(3)),
                VanillaEntityInterpolationPolicy::DisplayImmediate => (2, Some(0), Some(0)),
                VanillaEntityInterpolationPolicy::MinecartFeatureGated => {
                    assert!(vanilla_entity_metadata_type_is_minecart(entry.id));
                    (3, Some(3), None)
                }
                VanillaEntityInterpolationPolicy::ShulkerAttachPosition => {
                    assert_eq!(entry.id, VANILLA_ENTITY_TYPE_SHULKER_ID);
                    (4, None, None)
                }
            };
            counts[index] += 1;
            assert_eq!(
                vanilla_entity_metadata_type_interpolation_steps(entry.id, false),
                legacy_steps,
                "legacy interpolation policy drifted for {}",
                entry.resource_id
            );
            assert_eq!(
                vanilla_entity_metadata_type_interpolation_steps(entry.id, true),
                improved_steps,
                "improved interpolation policy drifted for {}",
                entry.resource_id
            );
        }

        assert_eq!(ids.len(), 157);
        assert_eq!(ids.len(), VANILLA_ENTITY_TYPE_METADATA_26_1.len());
        assert_eq!(counts.iter().sum::<usize>(), ids.len());
        assert_eq!(
            counts,
            [33, 113, 3, 7, 1],
            "direct/generic/display/minecart/Shulker partition drifted"
        );
        assert_eq!(vanilla_entity_metadata_type_interpolation_policy(-1), None);
        assert_eq!(
            vanilla_entity_metadata_type_interpolation_policy(
                VANILLA_ENTITY_TYPE_METADATA_26_1.len() as i32
            ),
            None
        );
    }

    #[test]
    fn generated_client_movement_policy_partitions_all_fixed_entity_types_once() {
        let mut counts = [0usize; 19];
        for entry in VANILLA_ENTITY_TYPES {
            let policy = vanilla_entity_metadata_type_client_movement_policy(entry.id)
                .unwrap_or_else(|| {
                    panic!("missing client movement policy for {}", entry.resource_id)
                });
            let index = match policy {
                VanillaEntityClientMovementPolicy::ServerSynchronized => 0,
                VanillaEntityClientMovementPolicy::Vehicle => 1,
                VanillaEntityClientMovementPolicy::Item => 2,
                VanillaEntityClientMovementPolicy::ExperienceOrb => 3,
                VanillaEntityClientMovementPolicy::FallingBlock => 4,
                VanillaEntityClientMovementPolicy::PrimedTnt => 5,
                VanillaEntityClientMovementPolicy::ThrowableProjectile => 6,
                VanillaEntityClientMovementPolicy::ThrownExperienceBottle => 7,
                VanillaEntityClientMovementPolicy::ThrownPotion => 8,
                VanillaEntityClientMovementPolicy::AbstractArrow => 9,
                VanillaEntityClientMovementPolicy::HurtingProjectile => 10,
                VanillaEntityClientMovementPolicy::WindCharge => 11,
                VanillaEntityClientMovementPolicy::DragonFireball => 12,
                VanillaEntityClientMovementPolicy::WitherSkull => 13,
                VanillaEntityClientMovementPolicy::LlamaSpit => 14,
                VanillaEntityClientMovementPolicy::EyeOfEnder => 15,
                VanillaEntityClientMovementPolicy::FireworkRocket => 16,
                VanillaEntityClientMovementPolicy::FishingHook => 17,
                VanillaEntityClientMovementPolicy::ShulkerBullet => 18,
            };
            counts[index] += 1;
        }

        assert_eq!(
            counts,
            [106, 27, 1, 1, 1, 1, 3, 1, 2, 3, 2, 2, 1, 1, 1, 1, 1, 1, 1],
            "the source-derived 26.1 movement-owner partition drifted"
        );
        assert_eq!(counts.iter().sum::<usize>(), VANILLA_ENTITY_TYPES.len());
        assert_eq!(
            vanilla_entity_metadata_type_client_movement_policy(-1),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_type_client_movement_policy(VANILLA_ENTITY_TYPES.len() as i32),
            None
        );
    }

    #[test]
    fn generated_runtime_tick_owners_remain_in_the_closed_entity_client_tick_inventory() {
        assert_eq!(VANILLA_ENTITY_TYPE_METADATA_26_1.len(), 157);
        for entry in VANILLA_ENTITY_TYPE_METADATA_26_1 {
            assert!(
                entry.runtime_class.is_some()
                    && entry.tick_owner.is_some()
                    && entry.base_tick_owner.is_some()
                    && entry.ride_tick_owner.is_some(),
                "generated runtime tick ownership is incomplete for {}",
                entry.resource_id
            );
            assert_eq!(
                vanilla_entity_metadata_type_client_movement_policy(entry.entity_type_id),
                Some(entry.client_movement_policy),
                "client movement disposition drifted for {}",
                entry.resource_id
            );
            assert_eq!(
                vanilla_entity_metadata_type_block_effect_tick_policy(entry.entity_type_id),
                Some(entry.block_effect_tick_policy),
                "block-effect tick disposition drifted for {}",
                entry.resource_id
            );
        }
    }

    #[test]
    fn generated_class_graph_exposes_hoglin_base_and_rejects_unknown_types() {
        assert!(VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .any(|entry| vanilla_entity_metadata_type_is_hoglin_base(entry.entity_type_id)));
        assert!(!vanilla_entity_metadata_type_is_hoglin_base(-1));
        assert!(!vanilla_entity_metadata_type_is_hoglin_base(
            VANILLA_ENTITY_TYPE_METADATA_26_1.len() as i32
        ));
    }

    #[test]
    fn generated_handle_entity_event_owners_follow_nearest_class_declaration() {
        const SQUID_OWNER: &str = "net.minecraft.world.entity.animal.squid.Squid";
        const OCELOT_OWNER: &str = "net.minecraft.world.entity.animal.feline.Ocelot";
        const WOLF_OWNER: &str = "net.minecraft.world.entity.animal.wolf.Wolf";
        const FISHING_HOOK_OWNER: &str = "net.minecraft.world.entity.projectile.FishingHook";
        const PLAYER_OWNER: &str = "net.minecraft.world.entity.player.Player";
        let squid_dispatch_types = VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .filter(|entry| entry.handle_entity_event_owner == Some(SQUID_OWNER))
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            squid_dispatch_types,
            vec![
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SQUID_ID,
            ]
        );
        for entity_type_id in squid_dispatch_types {
            assert_eq!(
                vanilla_entity_metadata_handle_entity_event_owner(entity_type_id),
                Some(SQUID_OWNER)
            );
        }
        assert_ne!(
            vanilla_entity_metadata_handle_entity_event_owner(VANILLA_ENTITY_TYPE_GUARDIAN_ID),
            Some(SQUID_OWNER)
        );
        assert_eq!(vanilla_entity_metadata_handle_entity_event_owner(-1), None);
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner(
                VANILLA_ENTITY_TYPE_METADATA_26_1.len() as i32
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner(VANILLA_ENTITY_TYPE_OCELOT_ID),
            Some(OCELOT_OWNER)
        );
        assert!(
            vanilla_entity_metadata_type_uses_ocelot_entity_event_handler(
                VANILLA_ENTITY_TYPE_OCELOT_ID
            )
        );
        assert!(
            !vanilla_entity_metadata_type_uses_ocelot_entity_event_handler(
                VANILLA_ENTITY_TYPE_CAT_ID
            )
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner(VANILLA_ENTITY_TYPE_WOLF_ID),
            Some(WOLF_OWNER)
        );
        assert_ne!(
            vanilla_entity_metadata_handle_entity_event_owner(VANILLA_ENTITY_TYPE_CAT_ID),
            Some(WOLF_OWNER)
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner(
                VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID
            ),
            Some(FISHING_HOOK_OWNER)
        );
        assert!(
            vanilla_entity_metadata_type_uses_fishing_hook_entity_event_handler(
                VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID
            )
        );
        assert!(
            !vanilla_entity_metadata_type_uses_fishing_hook_entity_event_handler(
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        );
        assert_eq!(
            vanilla_entity_metadata_handle_entity_event_owner(VANILLA_ENTITY_TYPE_PLAYER_ID),
            Some(PLAYER_OWNER)
        );
        assert!(
            vanilla_entity_metadata_type_uses_player_entity_event_handler(
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        );
        assert!(
            !vanilla_entity_metadata_type_uses_player_entity_event_handler(
                VANILLA_ENTITY_TYPE_COW_ID
            )
        );
    }

    #[test]
    fn generated_long_defaults_are_mechanically_typed() {
        for (owner, accessor, expected) in [
            (
                "net.minecraft.world.entity.animal.bee.Bee",
                "DATA_ANGER_END_TIME",
                VanillaEntityMetadataDefault::Long(-1),
            ),
            (
                "net.minecraft.world.entity.animal.camel.Camel",
                "LAST_POSE_CHANGE_TICK",
                VanillaEntityMetadataDefault::Long(0),
            ),
            (
                "net.minecraft.world.entity.animal.wolf.Wolf",
                "DATA_ANGER_END_TIME",
                VanillaEntityMetadataDefault::Long(-1),
            ),
        ] {
            let entry = VANILLA_ENTITY_METADATA_ACCESSORS_26_1
                .iter()
                .find(|entry| entry.owner == owner && entry.java_name == accessor)
                .expect("generated long accessor");
            assert_eq!(entry.typed_default, Some(expected));
        }
    }

    #[test]
    fn generated_entity_type_catalog_resolves_all_fixed_types() {
        assert_eq!(VANILLA_ENTITY_TYPE_METADATA_26_1.len(), 157);
        assert_eq!(VANILLA_ENTITY_TYPES.len(), 157);
        let mut resources = HashSet::new();
        let mut constants = HashSet::new();
        for (entry, protocol) in VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .zip(VANILLA_ENTITY_TYPES)
        {
            assert_eq!(entry.entity_type_id, protocol.id);
            assert_eq!(entry.resource_id, protocol.resource_id);
            assert!(resources.insert(entry.resource_id));
            assert!(constants.insert(entry.constant_name));
            assert!(entry.runtime_class.is_some());
            assert!(entry.schema_class.is_some());
            assert_eq!(entry.diagnostic, None);
        }
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_pickable_by_class)
                .count(),
            127
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| vanilla_entity_metadata_type_is_pickable(entry.entity_type_id))
                .count(),
            130
        );
        for resource_id in [
            "minecraft:allay",
            "minecraft:end_crystal",
            "minecraft:falling_block",
            "minecraft:item_frame",
            "minecraft:leash_knot",
            "minecraft:painting",
            "minecraft:shulker_bullet",
            "minecraft:tnt",
        ] {
            let entry = VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .unwrap();
            assert!(entry.is_pickable_by_class, "{resource_id}");
            assert!(vanilla_entity_metadata_type_is_pickable(
                entry.entity_type_id
            ));
        }
        for resource_id in [
            "minecraft:breeze_wind_charge",
            "minecraft:fireball",
            "minecraft:wind_charge",
        ] {
            let entry = VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .unwrap();
            assert!(!entry.is_pickable_by_class, "{resource_id}");
            assert!(vanilla_entity_metadata_type_is_pickable(
                entry.entity_type_id
            ));
        }
        for resource_id in [
            "minecraft:arrow",
            "minecraft:dragon_fireball",
            "minecraft:ender_dragon",
            "minecraft:evoker_fangs",
            "minecraft:experience_orb",
            "minecraft:llama_spit",
            "minecraft:spectral_arrow",
            "minecraft:trident",
            "minecraft:wither_skull",
        ] {
            let entry = VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .unwrap();
            assert!(!entry.is_pickable_by_class, "{resource_id}");
            assert!(!vanilla_entity_metadata_type_is_pickable(
                entry.entity_type_id
            ));
        }
        assert!(!vanilla_entity_metadata_type_is_pickable(-1));
        assert!(!vanilla_entity_metadata_type_is_pickable(157));
        for (kind, expected_count) in [
            (VanillaEntitySoundSourceKind::Neutral, 110),
            (VanillaEntitySoundSourceKind::Hostile, 42),
            (VanillaEntitySoundSourceKind::Ambient, 2),
            (VanillaEntitySoundSourceKind::Weather, 1),
            (VanillaEntitySoundSourceKind::Players, 1),
            (VanillaEntitySoundSourceKind::Dynamic, 1),
        ] {
            assert_eq!(
                VANILLA_ENTITY_TYPE_METADATA_26_1
                    .iter()
                    .filter(|entry| entry.sound_source_kind == kind)
                    .count(),
                expected_count,
                "{kind:?}"
            );
        }
        for (entity_type_id, expected) in [
            (
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                VanillaEntitySoundSourceKind::Players,
            ),
            (
                VANILLA_ENTITY_TYPE_ITEM_ID,
                VanillaEntitySoundSourceKind::Ambient,
            ),
            (
                VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
                VanillaEntitySoundSourceKind::Ambient,
            ),
            (
                VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
                VanillaEntitySoundSourceKind::Weather,
            ),
            (
                VANILLA_ENTITY_TYPE_BLAZE_ID,
                VanillaEntitySoundSourceKind::Hostile,
            ),
            (
                VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
                VanillaEntitySoundSourceKind::Hostile,
            ),
            (
                VANILLA_ENTITY_TYPE_HOGLIN_ID,
                VanillaEntitySoundSourceKind::Hostile,
            ),
            (
                VANILLA_ENTITY_TYPE_PARCHED_ID,
                VanillaEntitySoundSourceKind::Hostile,
            ),
            (
                VANILLA_ENTITY_TYPE_RABBIT_ID,
                VanillaEntitySoundSourceKind::Dynamic,
            ),
            (
                VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
                VanillaEntitySoundSourceKind::Neutral,
            ),
        ] {
            assert_eq!(
                vanilla_entity_metadata_type_sound_source_kind(entity_type_id),
                Some(expected)
            );
        }
        assert_eq!(vanilla_entity_metadata_type_sound_source_kind(-1), None);
        assert_eq!(vanilla_entity_metadata_type_sound_source_kind(157), None);
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_living)
                .count(),
            92
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_animal)
                .count(),
            34
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_tamable_animal)
                .count(),
            5
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_projectile)
                .count(),
            19
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_hurting_projectile)
                .count(),
            6
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_wind_charge)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_boat)
                .count(),
            20
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.boat_ride_height_multiplier_bits.is_some())
                .count(),
            20
        );
        for entry in VANILLA_ENTITY_TYPE_METADATA_26_1 {
            assert_eq!(
                entry.is_boat,
                entry.boat_ride_height_multiplier_bits.is_some(),
                "{}",
                entry.resource_id
            );
        }
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.can_use_every_equipment_slot)
                .count(),
            82
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_abstract_chest_boat)
                .count(),
            10
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_minecart)
                .count(),
            7
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_abstract_horse)
                .count(),
            9
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_abstract_chested_horse)
                .map(|entry| entry.entity_type_id)
                .collect::<Vec<_>>(),
            vec![
                VANILLA_ENTITY_TYPE_DONKEY_ID,
                VANILLA_ENTITY_TYPE_LLAMA_ID,
                VANILLA_ENTITY_TYPE_MULE_ID,
                VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            ]
        );
        assert!(!vanilla_entity_metadata_type_is_abstract_chested_horse(-1));
        assert!(!vanilla_entity_metadata_type_is_abstract_chested_horse(157));
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_abstract_nautilus)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_llama)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_item_frame)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_guardian)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_flying_animal)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_camel)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_squid)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_slime)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_abstract_piglin)
                .count(),
            2
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_spider)
                .count(),
            2
        );
        assert!(VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .any(|entry| entry.illager_arm_pose_uses_celebrating));
        assert!(!vanilla_entity_metadata_type_illager_arm_pose_uses_celebrating(-1));
        assert!(
            !vanilla_entity_metadata_type_illager_arm_pose_uses_celebrating(
                VANILLA_ENTITY_TYPE_METADATA_26_1.len() as i32
            )
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.is_player_rideable_jumping)
                .count(),
            11
        );
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| entry.has_custom_inventory_screen)
                .count(),
            21
        );
        assert!(vanilla_entity_metadata_type_is_living(
            VANILLA_ENTITY_TYPE_PLAYER_ID
        ));
        assert!(vanilla_entity_metadata_type_is_living(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        ));
        assert!(vanilla_entity_metadata_type_is_living(
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_living(
            VANILLA_ENTITY_TYPE_MINECART_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_living(-1));
        assert!(!vanilla_entity_metadata_type_is_living(157));
        assert!(vanilla_entity_metadata_type_is_animal(
            VANILLA_ENTITY_TYPE_AXOLOTL_ID
        ));
        assert!(vanilla_entity_metadata_type_is_animal(
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
        ));
        assert!(vanilla_entity_metadata_type_is_animal(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_animal(
            VANILLA_ENTITY_TYPE_DOLPHIN_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_animal(-1));
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_CAT_ID,
            VANILLA_ENTITY_TYPE_PARROT_ID,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        ] {
            assert!(vanilla_entity_metadata_type_is_tamable_animal(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_metadata_type_is_tamable_animal(
            VANILLA_ENTITY_TYPE_HORSE_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_tamable_animal(-1));
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .filter(|entry| {
                    vanilla_entity_metadata_baby_accessor_for_entity_type(entry.entity_type_id)
                        .is_some()
                })
                .count(),
            46
        );
        for (entity_type_id, expected_data_id) in [
            (VANILLA_ENTITY_TYPE_CAMEL_ID, 16),
            (VANILLA_ENTITY_TYPE_ZOMBIE_ID, 16),
            (VANILLA_ENTITY_TYPE_PIGLIN_ID, 17),
            (VANILLA_ENTITY_TYPE_ZOGLIN_ID, 16),
        ] {
            assert_eq!(
                vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id),
                Some((expected_data_id, false))
            );
        }
        assert_eq!(
            vanilla_entity_metadata_baby_accessor_for_entity_type(VANILLA_ENTITY_TYPE_PILLAGER_ID),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_baby_accessor_for_entity_type(-1),
            None
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID] {
            assert_eq!(
                vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id),
                Some((17, false))
            );
        }
        for entity_type_id in [VANILLA_ENTITY_TYPE_ZOMBIE_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id),
                None
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_EVOKER_ID,
            VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
        ] {
            assert!(vanilla_entity_metadata_type_is_spellcaster_illager(
                entity_type_id
            ));
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            -1,
        ] {
            assert!(!vanilla_entity_metadata_type_is_spellcaster_illager(
                entity_type_id
            ));
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_type_boat_ride_height_multiplier(entity_type_id),
                Some(1.0_f32 / 3.0)
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
            VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_type_boat_ride_height_multiplier(entity_type_id),
                Some(0.888_888_9)
            );
        }
        assert_eq!(
            vanilla_entity_metadata_type_boat_ride_height_multiplier(VANILLA_ENTITY_TYPE_HORSE_ID),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_type_boat_ride_height_multiplier(-1),
            None
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_HORSE_ID,
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
            VANILLA_ENTITY_TYPE_LLAMA_ID,
        ] {
            assert!(vanilla_entity_metadata_type_can_use_every_equipment_slot(
                entity_type_id
            ));
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_DONKEY_ID,
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            VANILLA_ENTITY_TYPE_PIG_ID,
            VANILLA_ENTITY_TYPE_STRIDER_ID,
        ] {
            assert!(!vanilla_entity_metadata_type_can_use_every_equipment_slot(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_metadata_type_can_use_every_equipment_slot(
            -1
        ));
        assert!(vanilla_entity_metadata_type_is_projectile(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID
        ));
        assert!(!vanilla_entity_metadata_type_is_projectile(
            VANILLA_ENTITY_TYPE_MINECART_ID
        ));

        let old_table = ["VANILLA_LIVING_", "ENTITY_TYPE_IDS"].concat();
        let dimensions = include_str!("dimensions.rs");
        assert!(!dimensions.contains(&old_table));
        let old_pickable_table = ["VANILLA_ENTITY_PICKABLE_", "TYPE_IDS"].concat();
        assert!(!dimensions.contains(&old_pickable_table));
        let animal_query = dimensions
            .split("pub(crate) fn vanilla_animal_entity_type")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("animal query source");
        assert!(!animal_query.contains("matches!"));
        let projectile_source = include_str!("projectiles.rs");
        let saved_query = projectile_source
            .split("pub(crate) fn is_vanilla_saved_projectile_type")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("saved projectile query source");
        assert!(!saved_query.contains("matches!"));
        let passenger_source = include_str!("passengers.rs");
        for function in [
            "is_vanilla_player_rideable_jumping_type",
            "is_vanilla_custom_inventory_screen_type",
        ] {
            let body = passenger_source
                .split(&format!("fn {function}"))
                .nth(1)
                .and_then(|source| source.split_once("\n}"))
                .map(|(body, _)| body)
                .expect("passenger interface query source");
            assert!(!body.contains("matches!"));
        }
        let entity_source = include_str!("../entities.rs");
        for function in [
            "is_vanilla_boat_type",
            "is_vanilla_minecart_type",
            "is_vanilla_abstract_horse_type",
            "is_vanilla_abstract_nautilus_type",
            "is_vanilla_llama_type",
            "vanilla_entity_type_is_squid",
            "is_vanilla_can_equip_saddle_type",
            "is_vanilla_can_wear_horse_armor_type",
        ] {
            let body = entity_source
                .split(&format!("fn {function}"))
                .nth(1)
                .and_then(|source| source.split_once("\n}"))
                .map(|(body, _)| body)
                .expect("vehicle class query source");
            assert!(!body.contains("matches!"));
        }
        let dimensions_source = include_str!("dimensions.rs");
        assert!(!dimensions_source.contains("ITEM_FRAME_ENTITY_TYPE_IDS"));
        let chest_boat_query = dimensions_source
            .split("fn is_vanilla_chest_boat_or_raft_type")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("chest boat class query source");
        assert!(!chest_boat_query.contains("matches!"));
        let animations_source = include_str!("animations.rs");
        let flying_animal_query = animations_source
            .split("fn is_flying_animal")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("flying-animal membership query source");
        assert!(!flying_animal_query.contains("matches!"));
        assert!(!animations_source.contains("fn is_vanilla_arrow_type"));
    }

    #[test]
    fn generated_living_event_audio_policies_cover_exact_class_inventory() {
        let mut death_owners = HashSet::new();
        let mut hurt_owners = HashSet::new();
        let mut volume_owners = HashSet::new();
        let mut fixed_resources = Vec::new();
        for entry in VANILLA_ENTITY_TYPE_METADATA_26_1 {
            assert_eq!(
                vanilla_entity_metadata_type_death_sound_policy(entry.entity_type_id),
                Some(entry.death_sound_policy)
            );
            assert_eq!(
                vanilla_entity_metadata_type_hurt_sound_policy(entry.entity_type_id),
                Some(entry.hurt_sound_policy)
            );
            assert_eq!(
                vanilla_entity_metadata_type_sound_volume_policy(entry.entity_type_id),
                Some(entry.sound_volume_policy)
            );
            if entry.is_living {
                death_owners.insert(entry.death_sound_owner.expect("living death-sound owner"));
                hurt_owners.insert(entry.hurt_sound_owner.expect("living hurt-sound owner"));
                volume_owners.insert(entry.sound_volume_owner.expect("living sound-volume owner"));
                assert_ne!(
                    entry.death_sound_policy,
                    VanillaEntityDeathSoundPolicy::NonLiving,
                    "{}",
                    entry.resource_id
                );
                assert_ne!(
                    entry.hurt_sound_policy,
                    VanillaEntityHurtSoundPolicy::NonLiving,
                    "{}",
                    entry.resource_id
                );
                assert_ne!(
                    entry.sound_volume_policy,
                    VanillaEntitySoundVolumePolicy::NonLiving,
                    "{}",
                    entry.resource_id
                );
            } else {
                assert_eq!(entry.death_sound_owner, None, "{}", entry.resource_id);
                assert_eq!(entry.hurt_sound_owner, None, "{}", entry.resource_id);
                assert_eq!(entry.sound_volume_owner, None, "{}", entry.resource_id);
                assert_eq!(
                    entry.death_sound_policy,
                    VanillaEntityDeathSoundPolicy::NonLiving,
                    "{}",
                    entry.resource_id
                );
                assert_eq!(
                    entry.hurt_sound_policy,
                    VanillaEntityHurtSoundPolicy::NonLiving,
                    "{}",
                    entry.resource_id
                );
                assert_eq!(
                    entry.sound_volume_policy,
                    VanillaEntitySoundVolumePolicy::NonLiving,
                    "{}",
                    entry.resource_id
                );
            }
            match entry.death_sound_policy {
                VanillaEntityDeathSoundPolicy::Fixed(resource_id) => {
                    fixed_resources.push(resource_id);
                }
                VanillaEntityDeathSoundPolicy::Conditional {
                    when_true,
                    when_false,
                    ..
                } => fixed_resources.extend([when_true, when_false]),
                VanillaEntityDeathSoundPolicy::NautilusAgeWater {
                    baby_underwater,
                    baby_land,
                    adult_underwater,
                    adult_land,
                } => fixed_resources.extend([
                    baby_underwater,
                    baby_land,
                    adult_underwater,
                    adult_land,
                ]),
                VanillaEntityDeathSoundPolicy::CopperGolemOxidation {
                    unaffected,
                    exposed,
                    weathered,
                    oxidized,
                } => fixed_resources.extend([unaffected, exposed, weathered, oxidized]),
                VanillaEntityDeathSoundPolicy::NonLiving
                | VanillaEntityDeathSoundPolicy::SoundVariant(_) => {}
            }
            match entry.hurt_sound_policy {
                VanillaEntityHurtSoundPolicy::Fixed(resource_id) => {
                    fixed_resources.push(resource_id);
                }
                VanillaEntityHurtSoundPolicy::Conditional {
                    when_true,
                    when_false,
                    ..
                } => fixed_resources.extend([when_true, when_false]),
                VanillaEntityHurtSoundPolicy::NautilusAgeWater {
                    baby_underwater,
                    baby_land,
                    adult_underwater,
                    adult_land,
                } => fixed_resources.extend([
                    baby_underwater,
                    baby_land,
                    adult_underwater,
                    adult_land,
                ]),
                VanillaEntityHurtSoundPolicy::CopperGolemOxidation {
                    unaffected,
                    exposed,
                    weathered,
                    oxidized,
                } => fixed_resources.extend([unaffected, exposed, weathered, oxidized]),
                VanillaEntityHurtSoundPolicy::WolfVariant { armor_damage } => {
                    fixed_resources.push(armor_damage);
                }
                VanillaEntityHurtSoundPolicy::NonLiving
                | VanillaEntityHurtSoundPolicy::SoundVariant(_)
                | VanillaEntityHurtSoundPolicy::PlayerDamageType => {}
            }
        }
        assert_eq!(death_owners.len(), 88);
        assert_eq!(hurt_owners.len(), 89);
        assert_eq!(volume_owners.len(), 13);
        assert!(fixed_resources.iter().all(|resource_id| {
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id).is_some()
        }));
        assert_eq!(
            vanilla_entity_metadata_type_hurt_sound_policy(VANILLA_ENTITY_TYPE_PLAYER_ID),
            Some(VanillaEntityHurtSoundPolicy::PlayerDamageType)
        );
        assert_eq!(
            vanilla_entity_metadata_type_death_sound_policy(VANILLA_ENTITY_TYPE_PLAYER_ID),
            Some(VanillaEntityDeathSoundPolicy::Fixed(
                "minecraft:entity.player.death"
            ))
        );
        assert_eq!(
            vanilla_entity_metadata_type_hurt_sound_policy(VANILLA_ENTITY_TYPE_CAT_ID),
            Some(VanillaEntityHurtSoundPolicy::SoundVariant(
                VanillaEntitySoundVariantFamily::Cat
            ))
        );
        assert_eq!(
            vanilla_entity_metadata_type_death_sound_policy(VANILLA_ENTITY_TYPE_WOLF_ID),
            Some(VanillaEntityDeathSoundPolicy::SoundVariant(
                VanillaEntitySoundVariantFamily::Wolf
            ))
        );
        assert_eq!(
            vanilla_entity_metadata_type_sound_volume_policy(VANILLA_ENTITY_TYPE_SLIME_ID),
            Some(VanillaEntitySoundVolumePolicy::SlimeSize)
        );
        assert_eq!(vanilla_entity_metadata_type_death_sound_policy(-1), None);
        assert_eq!(vanilla_entity_metadata_type_hurt_sound_policy(-1), None);
        assert_eq!(vanilla_entity_metadata_type_sound_volume_policy(157), None);
    }

    #[test]
    fn generated_camera_post_effects_follow_game_renderer_class_patterns() {
        assert_eq!(VANILLA_ENTITY_CAMERA_POST_EFFECTS_26_1.len(), 4);
        assert!(VANILLA_ENTITY_CAMERA_POST_EFFECTS_26_1
            .windows(2)
            .all(|entries| entries[0].0 < entries[1].0));
        assert_eq!(
            vanilla_entity_camera_post_effect_id(VANILLA_ENTITY_TYPE_CREEPER_ID),
            Some("minecraft:creeper")
        );
        assert_eq!(
            vanilla_entity_camera_post_effect_id(VANILLA_ENTITY_TYPE_SPIDER_ID),
            Some("minecraft:spider")
        );
        assert_eq!(
            vanilla_entity_camera_post_effect_id(VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID),
            Some("minecraft:spider")
        );
        assert_eq!(
            vanilla_entity_camera_post_effect_id(VANILLA_ENTITY_TYPE_ENDERMAN_ID),
            Some("minecraft:invert")
        );
        assert_eq!(
            vanilla_entity_camera_post_effect_id(VANILLA_ENTITY_TYPE_PLAYER_ID),
            None
        );
        assert_eq!(vanilla_entity_camera_post_effect_id(-1), None);
    }

    #[test]
    fn generated_scale_sanitizers_follow_nearest_override() {
        assert_eq!(VANILLA_ENTITY_SCALE_SANITIZERS_26_1.len(), 3);
        assert_eq!(
            vanilla_entity_scale_sanitizer(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID),
            Some(VanillaEntityScaleSanitizer::ClampMax(1.0))
        );
        assert_eq!(
            vanilla_entity_scale_sanitizer(VANILLA_ENTITY_TYPE_SHULKER_ID),
            Some(VanillaEntityScaleSanitizer::ClampMax(3.0))
        );
        assert_eq!(
            vanilla_entity_scale_sanitizer(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID),
            Some(VanillaEntityScaleSanitizer::Fixed(1.0))
        );
        assert_eq!(
            vanilla_entity_scale_sanitizer(VANILLA_ENTITY_TYPE_PLAYER_ID),
            None
        );
        assert_eq!(vanilla_entity_scale_sanitizer(-1), None);
    }

    #[test]
    fn generated_pushability_kinds_follow_nearest_class_override() {
        for (kind, expected_count) in [
            (VanillaEntityPushabilityKind::Never, 40),
            (VanillaEntityPushabilityKind::Always, 28),
            (VanillaEntityPushabilityKind::Living, 78),
            (VanillaEntityPushabilityKind::NotVehicle, 9),
            (VanillaEntityPushabilityKind::LivingCanMove, 1),
            (VanillaEntityPushabilityKind::LivingNotDiggingOrEmerging, 1),
        ] {
            assert_eq!(
                VANILLA_ENTITY_TYPE_METADATA_26_1
                    .iter()
                    .filter(|entry| entry.pushability_kind == kind)
                    .count(),
                expected_count,
                "{kind:?}"
            );
        }
        for (entity_type_id, expected) in [
            (
                VANILLA_ENTITY_TYPE_ARROW_ID,
                VanillaEntityPushabilityKind::Never,
            ),
            (
                VANILLA_ENTITY_TYPE_BAT_ID,
                VanillaEntityPushabilityKind::Never,
            ),
            (
                VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                VanillaEntityPushabilityKind::Never,
            ),
            (
                VANILLA_ENTITY_TYPE_PARROT_ID,
                VanillaEntityPushabilityKind::Always,
            ),
            (
                VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                VanillaEntityPushabilityKind::Always,
            ),
            (
                VANILLA_ENTITY_TYPE_MINECART_ID,
                VanillaEntityPushabilityKind::Always,
            ),
            (
                VANILLA_ENTITY_TYPE_COW_ID,
                VanillaEntityPushabilityKind::Living,
            ),
            (
                VANILLA_ENTITY_TYPE_HORSE_ID,
                VanillaEntityPushabilityKind::NotVehicle,
            ),
            (
                VANILLA_ENTITY_TYPE_CREAKING_ID,
                VanillaEntityPushabilityKind::LivingCanMove,
            ),
            (
                VANILLA_ENTITY_TYPE_WARDEN_ID,
                VanillaEntityPushabilityKind::LivingNotDiggingOrEmerging,
            ),
        ] {
            assert_eq!(
                vanilla_entity_metadata_type_pushability_kind(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(vanilla_entity_metadata_type_pushability_kind(-1), None);
        assert_eq!(vanilla_entity_metadata_type_pushability_kind(157), None);
    }

    #[test]
    fn generated_baby_behavior_and_age_scale_follow_nearest_vanilla_overrides() {
        for (entity_type_id, expected_behavior, expected_scale) in [
            (
                VANILLA_ENTITY_TYPE_ARMADILLO_ID,
                VanillaEntityBabyBehavior::Metadata,
                0.6_f32,
            ),
            (
                VANILLA_ENTITY_TYPE_CAMEL_ID,
                VanillaEntityBabyBehavior::Metadata,
                0.6,
            ),
            (
                VANILLA_ENTITY_TYPE_DOLPHIN_ID,
                VanillaEntityBabyBehavior::Metadata,
                0.65,
            ),
            (
                VANILLA_ENTITY_TYPE_GOAT_ID,
                VanillaEntityBabyBehavior::Metadata,
                0.55,
            ),
            (
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
                VanillaEntityBabyBehavior::Metadata,
                0.2375,
            ),
            (
                VANILLA_ENTITY_TYPE_TURTLE_ID,
                VanillaEntityBabyBehavior::Metadata,
                0.3,
            ),
            (
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                VanillaEntityBabyBehavior::Metadata,
                0.5,
            ),
            (
                VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                VanillaEntityBabyBehavior::ArmorStandSmall,
                0.5,
            ),
            (
                VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
                VanillaEntityBabyBehavior::Never,
                0.6,
            ),
            (
                VANILLA_ENTITY_TYPE_FROG_ID,
                VanillaEntityBabyBehavior::Never,
                0.5,
            ),
            (
                VANILLA_ENTITY_TYPE_PARROT_ID,
                VanillaEntityBabyBehavior::Never,
                0.5,
            ),
            (
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                VanillaEntityBabyBehavior::Never,
                0.5,
            ),
        ] {
            let (behavior, scale) =
                vanilla_entity_metadata_baby_behavior_and_age_scale(entity_type_id)
                    .expect("living entity age-scale facts");
            assert_eq!(behavior, expected_behavior, "entity type {entity_type_id}");
            assert_eq!(
                scale.to_bits(),
                expected_scale.to_bits(),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_baby_behavior_and_age_scale(VANILLA_ENTITY_TYPE_ITEM_ID),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_baby_behavior_and_age_scale(-1),
            None
        );
    }

    #[test]
    fn generated_collidability_kinds_follow_nearest_class_override() {
        for (kind, expected_count) in [
            (VanillaEntityCollidabilityKind::Never, 135),
            (VanillaEntityCollidabilityKind::Always, 20),
            (VanillaEntityCollidabilityKind::Alive, 1),
            (VanillaEntityCollidabilityKind::HappyGhast, 1),
        ] {
            assert_eq!(
                VANILLA_ENTITY_TYPE_METADATA_26_1
                    .iter()
                    .filter(|entry| entry.collidability_kind == kind)
                    .count(),
                expected_count,
                "{kind:?}"
            );
        }
        for (entity_type_id, expected) in [
            (
                VANILLA_ENTITY_TYPE_COW_ID,
                VanillaEntityCollidabilityKind::Never,
            ),
            (
                VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                VanillaEntityCollidabilityKind::Always,
            ),
            (
                VANILLA_ENTITY_TYPE_SHULKER_ID,
                VanillaEntityCollidabilityKind::Alive,
            ),
            (
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
                VanillaEntityCollidabilityKind::HappyGhast,
            ),
        ] {
            assert_eq!(
                vanilla_entity_metadata_type_collidability_kind(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(vanilla_entity_metadata_type_collidability_kind(-1), None);
        assert_eq!(vanilla_entity_metadata_type_collidability_kind(157), None);
    }

    #[test]
    fn generated_entity_type_accessor_query_follows_schema_inheritance() {
        for (resource_id, expected_owner) in [
            (
                "minecraft:egg",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrowableItemProjectile",
            ),
            (
                "minecraft:ender_pearl",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrowableItemProjectile",
            ),
            (
                "minecraft:experience_bottle",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrowableItemProjectile",
            ),
            (
                "minecraft:eye_of_ender",
                "net.minecraft.world.entity.projectile.EyeOfEnder",
            ),
            (
                "minecraft:fireball",
                "net.minecraft.world.entity.projectile.hurtingprojectile.Fireball",
            ),
            (
                "minecraft:lingering_potion",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrowableItemProjectile",
            ),
            (
                "minecraft:small_fireball",
                "net.minecraft.world.entity.projectile.hurtingprojectile.Fireball",
            ),
            (
                "minecraft:snowball",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrowableItemProjectile",
            ),
            (
                "minecraft:splash_potion",
                "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrowableItemProjectile",
            ),
        ] {
            let entity_type_id = VANILLA_ENTITY_TYPE_METADATA_26_1
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .unwrap()
                .entity_type_id;
            let accessor = vanilla_entity_metadata_accessor_for_entity_type(
                entity_type_id,
                "DATA_ITEM_STACK",
            )
            .unwrap();
            assert_eq!(accessor.owner, expected_owner);
            assert_eq!(accessor.data_id, 8);
            assert_eq!(accessor.serializer_name, "ITEM_STACK");
            assert_eq!(accessor.typed_default, None);
        }

        let item_type_id = VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .find(|entry| entry.resource_id == "minecraft:item")
            .unwrap()
            .entity_type_id;
        assert!(
            vanilla_entity_metadata_accessor_for_entity_type(item_type_id, "DATA_ITEM_STACK")
                .is_none()
        );
        let item_accessor = vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
            item_type_id,
            "ITEM_STACK",
        )
        .unwrap();
        assert_eq!(
            item_accessor.owner,
            "net.minecraft.world.entity.item.ItemEntity"
        );
        assert_eq!(item_accessor.java_name, "DATA_ITEM");
        assert_eq!(item_accessor.data_id, 8);

        let player_type_id = VANILLA_ENTITY_TYPE_METADATA_26_1
            .iter()
            .find(|entry| entry.resource_id == "minecraft:player")
            .unwrap()
            .entity_type_id;
        assert!(
            vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
                player_type_id,
                "INT"
            )
            .is_none()
        );
    }

    #[test]
    fn generated_wither_invulnerability_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WITHER_ID
            ),
            Some((19, 0))
        );
        assert_eq!(
            vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_wither_side_head_accessors_own_membership_indices_and_defaults() {
        let wither = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::WitherBoss);
        let target_b = wither
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_TARGET_B")
            .expect("generated WitherBoss target-B field");
        let target_c = wither
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_TARGET_C")
            .expect("generated WitherBoss target-C field");
        let VanillaEntityMetadataDefault::Int(target_b_default) = target_b.default else {
            panic!("generated WitherBoss target-B default must be INT");
        };
        let VanillaEntityMetadataDefault::Int(target_c_default) = target_c.default else {
            panic!("generated WitherBoss target-C default must be INT");
        };
        let expected = VanillaWitherSideHeadTargetMetadataAccessors {
            target_b_data_id: target_b.data_id,
            target_b_default,
            target_c_data_id: target_c.data_id,
            target_c_default,
        };
        assert_eq!(
            vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_WITHER_ID
            ),
            Some(expected)
        );
        assert_eq!(
            vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_enderman_render_accessors_own_membership_indices_and_defaults() {
        let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::EnderMan);
        let carried = schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_CARRY_STATE")
            .expect("generated EnderMan carried-block field");
        let creepy = schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_CREEPY")
            .expect("generated EnderMan creepy field");
        assert_eq!(
            carried.default,
            VanillaEntityMetadataDefault::OptionalBlockStateNone
        );
        let VanillaEntityMetadataDefault::Boolean(creepy_default) = creepy.default else {
            panic!("generated EnderMan creepy default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_ENDERMAN_ID
            ),
            Some(VanillaEndermanRenderMetadataAccessors {
                carried_block_state_data_id: carried.data_id,
                carried_block_state_default: None,
                creepy_data_id: creepy.data_id,
                creepy_default,
            })
        );
        assert_eq!(
            vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_enderman_render_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_end_crystal_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_end_crystal_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
            ),
            Some(VanillaEndCrystalMetadataAccessors {
                beam_target_data_id: 8,
                show_bottom_data_id: 9,
                show_bottom_default: true,
            })
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_ghast_charging_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_ghast_charging_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_GHAST_ID
            ),
            Some((16, false))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_ghast_charging_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_happy_ghast_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            ),
            Some(VanillaHappyGhastMetadataAccessors {
                is_leash_holder_data_id: 18,
                is_leash_holder_default: false,
                stays_still_data_id: 19,
                stays_still_default: false,
            })
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_GHAST_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_glow_squid_dark_ticks_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
            ),
            Some((18, 0))
        );
        for entity_type_id in [bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SQUID_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_goat_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_goat_accessors_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID
            ),
            Some(VanillaGoatMetadataAccessors {
                screaming_data_id: 18,
                screaming_default: false,
                left_horn_data_id: 19,
                left_horn_default: true,
                right_horn_data_id: 20,
                right_horn_default: true,
            })
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_COW_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_goat_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_turtle_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_turtle_accessors_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID
            ),
            Some(VanillaTurtleMetadataAccessors {
                has_egg_data_id: VANILLA_TURTLE_HAS_EGG_DATA_ID,
                has_egg_default: VANILLA_TURTLE_DEFAULT_HAS_EGG,
                laying_egg_data_id: VANILLA_TURTLE_LAYING_EGG_DATA_ID,
                laying_egg_default: VANILLA_TURTLE_DEFAULT_LAYING_EGG,
            })
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_BAT_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_turtle_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_wither_skull_dangerous_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_SKULL_ID
            ),
            Some((8, false))
        );
        for entity_type_id in [
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_hanging_direction_accessor_owns_membership_index_and_default() {
        let expected = Some(VanillaHangingEntityMetadataAccessor {
            direction_data_id: 8,
            direction_default: 3,
        });
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            VANILLA_ENTITY_TYPE_PAINTING_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(entity_type_id),
                expected
            );
        }
        for entity_type_id in [VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_painting_variant_accessor_owns_membership_index_and_virtual_default() {
        assert_eq!(
            vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PAINTING_ID
            ),
            Some(9)
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_painting_variant_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_item_frame_accessors_own_membership_indices_and_defaults() {
        let expected = Some(VanillaItemFrameMetadataAccessors {
            direction_data_id: 8,
            direction_default: 3,
            item_data_id: 9,
            rotation_data_id: 10,
            rotation_default: 0,
        });
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity_type_id),
                expected
            );
        }
        for entity_type_id in [VANILLA_ENTITY_TYPE_PAINTING_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_bat_flags_accessor_owns_membership_index_default_and_mask() {
        assert_eq!(
            vanilla_entity_metadata_bat_flags_accessor_for_entity_type(VANILLA_ENTITY_TYPE_BAT_ID),
            Some((16, 0, 1))
        );
        assert_eq!(
            vanilla_entity_metadata_bat_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_bat_flags_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_bee_accessors_own_membership_indices_defaults_and_masks() {
        let expected = VanillaBeeMetadataAccessors {
            flags_data_id: VANILLA_BEE_FLAGS_DATA_ID,
            flags_default: VANILLA_BEE_DEFAULT_FLAGS,
            roll_mask: VANILLA_BEE_FLAG_ROLL,
            has_stung_mask: VANILLA_BEE_FLAG_HAS_STUNG,
            has_nectar_mask: VANILLA_BEE_FLAG_HAS_NECTAR,
            anger_end_time_data_id: VANILLA_BEE_ANGER_END_TIME_DATA_ID,
            anger_end_time_default: VANILLA_BEE_DEFAULT_ANGER_END_TIME,
        };
        assert_eq!(
            vanilla_entity_metadata_bee_accessors_for_entity_type(VANILLA_ENTITY_TYPE_BEE_ID),
            Some(expected)
        );
        assert_eq!(
            vanilla_entity_metadata_bee_flags_accessor_for_entity_type(VANILLA_ENTITY_TYPE_BEE_ID),
            Some((expected.flags_data_id, expected.flags_default))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_CHICKEN_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_bee_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_dolphin_accessors_own_membership_indices_and_defaults() {
        let expected = VanillaDolphinMetadataAccessors {
            got_fish_data_id: VANILLA_DOLPHIN_GOT_FISH_DATA_ID,
            got_fish_default: VANILLA_DOLPHIN_DEFAULT_GOT_FISH,
            moistness_data_id: VANILLA_DOLPHIN_MOISTNESS_DATA_ID,
            moistness_default: VANILLA_DOLPHIN_DEFAULT_MOISTNESS,
        };
        assert_eq!(
            vanilla_entity_metadata_dolphin_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_DOLPHIN_ID
            ),
            Some(expected)
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_CHICKEN_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_dolphin_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_fox_accessors_own_membership_indices_defaults_and_masks() {
        let expected = VanillaFoxMetadataAccessors {
            type_data_id: VANILLA_FOX_TYPE_DATA_ID,
            type_default: VANILLA_FOX_DEFAULT_TYPE,
            flags_data_id: VANILLA_FOX_FLAGS_DATA_ID,
            flags_default: VANILLA_FOX_DEFAULT_FLAGS,
            sitting_mask: VANILLA_FOX_FLAG_SITTING,
            crouching_mask: VANILLA_FOX_FLAG_CROUCHING,
            interested_mask: VANILLA_FOX_FLAG_INTERESTED,
            pouncing_mask: VANILLA_FOX_FLAG_POUNCING,
            sleeping_mask: VANILLA_FOX_FLAG_SLEEPING,
            faceplanted_mask: VANILLA_FOX_FLAG_FACEPLANTED,
            defending_mask: VANILLA_FOX_FLAG_DEFENDING,
            trusted_data_ids: [VANILLA_FOX_TRUSTED_DATA_ID_0, VANILLA_FOX_TRUSTED_DATA_ID_1],
            trusted_default: None,
        };
        assert_eq!(
            vanilla_entity_metadata_fox_accessors_for_entity_type(VANILLA_ENTITY_TYPE_FOX_ID),
            Some(expected)
        );
        assert_eq!(
            vanilla_entity_metadata_fox_flags_accessor_for_entity_type(VANILLA_ENTITY_TYPE_FOX_ID),
            Some((expected.flags_data_id, expected.flags_default))
        );
        assert_eq!(
            vanilla_entity_metadata_fox_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_FOX_ID
            ),
            Some((expected.type_data_id, expected.type_default))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_CHICKEN_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_fox_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_iron_golem_flags_accessor_owns_membership_index_default_and_named_mask() {
        let iron_golem = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::IronGolem);
        let flags = iron_golem
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_FLAGS_ID")
            .expect("generated IronGolem flags field");
        let VanillaEntityMetadataDefault::Byte(flags_default) = flags.default else {
            panic!("generated IronGolem flags default must be BYTE");
        };
        let player_created = iron_golem
            .own_flags
            .iter()
            .find(|flag| flag.name == "isPlayerCreated")
            .expect("generated IronGolem player-created flag");
        assert_eq!(
            vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
            ),
            Some((flags.data_id, flags_default, player_created.mask))
        );
        assert_eq!(
            vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_snow_golem_pumpkin_accessor_owns_membership_index_and_default() {
        let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::SnowGolem);
        let field = schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_PUMPKIN_ID")
            .expect("generated SnowGolem pumpkin field");
        let flag = schema
            .own_flags
            .iter()
            .find(|flag| flag.name == "PUMPKIN_FLAG")
            .expect("generated SnowGolem pumpkin flag");
        let VanillaEntityMetadataDefault::Byte(default) = field.default else {
            panic!("generated SnowGolem pumpkin default must be byte")
        };
        assert_eq!(
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID
            ),
            Some((field.data_id, default, flag.mask))
        );
        assert_eq!(
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_sheep_wool_accessor_owns_membership_index_default_and_masks() {
        let sheep = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Sheep);
        let wool = sheep
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_WOOL_ID")
            .expect("generated Sheep wool field");
        let sheared = sheep
            .own_flags
            .iter()
            .find(|flag| flag.name == "isSheared")
            .expect("generated Sheep sheared flag");
        let VanillaEntityMetadataDefault::Byte(default) = wool.default else {
            panic!("generated Sheep wool default must be BYTE");
        };
        assert_eq!(
            vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SHEEP_ID
            ),
            Some((
                wool.data_id,
                default,
                sheared.mask,
                VANILLA_SHEEP_WOOL_COLOR_MASK,
            ))
        );
        assert_eq!(
            vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_rabbit_type_accessor_owns_membership_index_and_default() {
        let rabbit = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Rabbit);
        let variant = rabbit
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_TYPE_ID")
            .expect("generated Rabbit variant field");
        let VanillaEntityMetadataDefault::Int(default) = variant.default else {
            panic!("generated Rabbit variant default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_RABBIT_ID
            ),
            Some((variant.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_pufferfish_puff_state_accessor_owns_membership_index_and_default() {
        let pufferfish = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Pufferfish);
        let puff_state = pufferfish
            .own_fields
            .iter()
            .find(|field| field.name == "PUFF_STATE")
            .expect("generated Pufferfish puff-state field");
        let VanillaEntityMetadataDefault::Int(default) = puff_state.default else {
            panic!("generated Pufferfish puff-state default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PUFFERFISH_ID
            ),
            Some((puff_state.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_slime_size_accessor_owns_inherited_membership_index_and_default() {
        let slime = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Slime);
        let size = slime
            .own_fields
            .iter()
            .find(|field| field.name == "ID_SIZE")
            .expect("generated Slime size field");
        let VanillaEntityMetadataDefault::Int(default) = size.default else {
            panic!("generated Slime size default must be INT");
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_SLIME_ID,
            VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id),
                Some((size.data_id, default)),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_slime_size_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_slime_size_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_phantom_size_accessor_owns_membership_index_and_default() {
        let phantom = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Phantom);
        let size = phantom
            .own_fields
            .iter()
            .find(|field| field.name == "ID_SIZE")
            .expect("generated Phantom size field");
        let VanillaEntityMetadataDefault::Int(default) = size.default else {
            panic!("generated Phantom size default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PHANTOM_ID
            ),
            Some((size.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SLIME_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_phantom_size_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_salmon_variant_accessor_owns_membership_index_and_default() {
        let salmon = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Salmon);
        let variant = salmon
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_TYPE")
            .expect("generated Salmon variant field");
        let VanillaEntityMetadataDefault::Int(default) = variant.default else {
            panic!("generated Salmon variant default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SALMON_ID
            ),
            Some((variant.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_tropical_fish_variant_accessor_owns_membership_index_default_and_layout() {
        let tropical_fish =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::TropicalFish);
        let variant = tropical_fish
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_ID_TYPE_VARIANT")
            .expect("generated TropicalFish packed-variant field");
        let VanillaEntityMetadataDefault::Int(default) = variant.default else {
            panic!("generated TropicalFish packed-variant default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID
            ),
            Some(VanillaTropicalFishVariantAccessors {
                variant_data_id: variant.data_id,
                variant_default: default,
                pattern_mask: VANILLA_TROPICAL_FISH_PATTERN_MASK,
                base_color_mask: VANILLA_TROPICAL_FISH_BASE_COLOR_MASK,
                base_color_shift: VANILLA_TROPICAL_FISH_BASE_COLOR_SHIFT,
                pattern_color_mask: VANILLA_TROPICAL_FISH_PATTERN_COLOR_MASK,
                pattern_color_shift: VANILLA_TROPICAL_FISH_PATTERN_COLOR_SHIFT,
            })
        );
        assert_eq!(
            vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_parrot_variant_accessor_owns_membership_index_and_default() {
        let parrot = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Parrot);
        let variant = parrot
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_VARIANT_ID")
            .expect("generated Parrot variant field");
        let VanillaEntityMetadataDefault::Int(default) = variant.default else {
            panic!("generated Parrot variant default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PARROT_ID
            ),
            Some((variant.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_axolotl_variant_accessor_owns_membership_index_and_default() {
        let axolotl = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Axolotl);
        let variant = axolotl
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_VARIANT")
            .expect("generated Axolotl variant field");
        let VanillaEntityMetadataDefault::Int(default) = variant.default else {
            panic!("generated Axolotl variant default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID
            ),
            Some((variant.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PARROT_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_axolotl_from_bucket_accessor_owns_membership_index_and_default() {
        let axolotl = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Axolotl);
        let from_bucket = axolotl
            .own_fields
            .iter()
            .find(|field| field.name == "FROM_BUCKET")
            .expect("generated Axolotl from-bucket field");
        let VanillaEntityMetadataDefault::Boolean(default) = from_bucket.default else {
            panic!("generated Axolotl from-bucket default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID
            ),
            Some((from_bucket.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PARROT_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_abstract_fish_from_bucket_accessor_owns_membership_index_and_default() {
        let fish = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractFish);
        let from_bucket = fish
            .own_fields
            .iter()
            .find(|field| field.name == "FROM_BUCKET")
            .expect("generated AbstractFish from-bucket field");
        let VanillaEntityMetadataDefault::Boolean(default) = from_bucket.default else {
            panic!("generated AbstractFish from-bucket default must be BOOLEAN");
        };
        for entity_type_id in [
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COD_ID,
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
            VANILLA_ENTITY_TYPE_SALMON_ID,
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
            VANILLA_ENTITY_TYPE_TADPOLE_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type(
                    entity_type_id
                ),
                Some((from_bucket.data_id, default))
            );
        }
        assert_eq!(
            vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_ocelot_trusting_accessor_owns_membership_index_and_default() {
        let ocelot = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Ocelot);
        let trusting = ocelot
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_TRUSTING")
            .expect("generated Ocelot trusting field");
        let VanillaEntityMetadataDefault::Boolean(default) = trusting.default else {
            panic!("generated Ocelot trusting default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_OCELOT_ID
            ),
            Some((trusting.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CAT_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_mushroom_cow_variant_accessor_owns_membership_index_and_default() {
        let mushroom_cow = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::MushroomCow);
        let variant = mushroom_cow
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_TYPE")
            .expect("generated MushroomCow variant field");
        let VanillaEntityMetadataDefault::Int(default) = variant.default else {
            panic!("generated MushroomCow variant default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID
            ),
            Some((variant.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_COW_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_bogged_sheared_accessor_owns_membership_index_and_default() {
        let bogged = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Bogged);
        let sheared = bogged
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_SHEARED")
            .expect("generated Bogged sheared field");
        let VanillaEntityMetadataDefault::Boolean(default) = sheared.default else {
            panic!("generated Bogged sheared default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_BOGGED_ID
            ),
            Some((sheared.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SKELETON_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_allay_dancing_accessor_owns_membership_index_and_default() {
        let allay = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Allay);
        let dancing = allay
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_DANCING")
            .expect("generated Allay dancing field");
        let VanillaEntityMetadataDefault::Boolean(default) = dancing.default else {
            panic!("generated Allay dancing default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ALLAY_ID
            ),
            Some((dancing.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_BOGGED_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_allay_can_duplicate_accessor_owns_membership_index_and_default() {
        let allay = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Allay);
        let can_duplicate = allay
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_CAN_DUPLICATE")
            .expect("generated Allay can-duplicate field");
        let VanillaEntityMetadataDefault::Boolean(default) = can_duplicate.default else {
            panic!("generated Allay can-duplicate default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ALLAY_ID
            ),
            Some((can_duplicate.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_BOGGED_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_strider_boost_time_accessor_owns_membership_index_and_default() {
        let strider = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Strider);
        let boost_time = strider
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_BOOST_TIME")
            .expect("generated Strider boost-time field");
        let VanillaEntityMetadataDefault::Int(default) = boost_time.default else {
            panic!("generated Strider boost-time default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_strider_boost_time_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_STRIDER_ID
            ),
            Some((boost_time.data_id, default))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_COW_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_strider_boost_time_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_item_based_steering_accessor_composes_only_pig_and_strider() {
        let pig = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Pig);
        let pig_boost = pig
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_BOOST_TIME")
            .expect("generated Pig boost-time field");
        let VanillaEntityMetadataDefault::Int(pig_default) = pig_boost.default else {
            panic!("generated Pig boost-time default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIG_ID
            ),
            Some((pig_boost.data_id, pig_default))
        );
        let strider = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Strider);
        let strider_boost = strider
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_BOOST_TIME")
            .expect("generated Strider boost-time field");
        let VanillaEntityMetadataDefault::Int(strider_default) = strider_boost.default else {
            panic!("generated Strider boost-time default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_STRIDER_ID
            ),
            Some((strider_boost.data_id, strider_default))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_COW_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_witch_using_item_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_witch_using_item_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WITCH_ID
            ),
            Some((17, false))
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            VANILLA_ENTITY_TYPE_COW_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_witch_using_item_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_player_absorption_accessor_owns_membership_index_and_default() {
        let player = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Player);
        let absorption = player
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_PLAYER_ABSORPTION_ID")
            .expect("generated Player absorption field");
        let VanillaEntityMetadataDefault::Float(default) = absorption.default else {
            panic!("generated Player absorption default must be FLOAT");
        };
        assert_eq!(
            vanilla_entity_metadata_player_absorption_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PLAYER_ID
            ),
            Some((absorption.data_id, default))
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            VANILLA_ENTITY_TYPE_COW_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_player_absorption_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_axolotl_playing_dead_accessor_owns_membership_index_and_default() {
        let axolotl = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Axolotl);
        let playing_dead = axolotl
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_PLAYING_DEAD")
            .expect("generated Axolotl playing-dead field");
        let VanillaEntityMetadataDefault::Boolean(default) = playing_dead.default else {
            panic!("generated Axolotl playing-dead default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID
            ),
            Some((playing_dead.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ALLAY_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_camel_dash_accessor_owns_inherited_membership_index_and_default() {
        let camel = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Camel);
        let dash = camel
            .own_fields
            .iter()
            .find(|field| field.name == "DASH")
            .expect("generated Camel dash field");
        let VanillaEntityMetadataDefault::Boolean(default) = dash.default else {
            panic!("generated Camel dash default must be BOOLEAN");
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_camel_dash_accessor_for_entity_type(entity_type_id),
                Some((dash.data_id, default)),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_camel_dash_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_NAUTILUS_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_camel_dash_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_camel_pose_tick_accessor_owns_inherited_membership_index_and_default() {
        let camel = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Camel);
        let pose_tick = camel
            .own_fields
            .iter()
            .find(|field| field.name == "LAST_POSE_CHANGE_TICK")
            .expect("generated Camel last-pose-change field");
        let VanillaEntityMetadataDefault::Long(default) = pose_tick.default else {
            panic!("generated Camel last-pose-change default must be LONG");
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                    entity_type_id
                ),
                Some((pose_tick.data_id, default)),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_NAUTILUS_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_cat_accessors_own_membership_indices_and_defaults() {
        let cat = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Cat);
        let field = |name| {
            cat.own_fields
                .iter()
                .find(|field| field.name == name)
                .unwrap_or_else(|| panic!("generated Cat {name} field"))
        };
        let lying = field("IS_LYING");
        let relax_state_one = field("RELAX_STATE_ONE");
        let collar_color = field("DATA_COLLAR_COLOR");
        let VanillaEntityMetadataDefault::Boolean(lying_default) = lying.default else {
            panic!("generated Cat lying default must be BOOLEAN");
        };
        let VanillaEntityMetadataDefault::Boolean(relax_state_one_default) =
            relax_state_one.default
        else {
            panic!("generated Cat relax-state-one default must be BOOLEAN");
        };
        let VanillaEntityMetadataDefault::Int(collar_color_default) = collar_color.default else {
            panic!("generated Cat collar-color default must be INT");
        };
        let expected = VanillaCatMetadataAccessors {
            lying_data_id: lying.data_id,
            lying_default,
            relax_state_one_data_id: relax_state_one.data_id,
            relax_state_one_default,
            collar_color_data_id: collar_color.data_id,
            collar_color_default,
        };
        assert_eq!(
            vanilla_entity_metadata_cat_accessors_for_entity_type(VANILLA_ENTITY_TYPE_CAT_ID),
            Some(expected)
        );
        assert_eq!(
            vanilla_entity_metadata_cat_lying_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAT_ID),
            Some((expected.lying_data_id, expected.lying_default))
        );
        assert_eq!(
            vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CAT_ID
            ),
            Some((
                expected.relax_state_one_data_id,
                expected.relax_state_one_default,
            ))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_WOLF_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_cat_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_creaking_accessors_own_membership_indices_and_defaults() {
        let creaking = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Creaking);
        let field = |name| {
            creaking
                .own_fields
                .iter()
                .find(|field| field.name == name)
                .unwrap_or_else(|| panic!("generated Creaking {name} field"))
        };
        let can_move = field("CAN_MOVE");
        let active = field("IS_ACTIVE");
        let tearing_down = field("IS_TEARING_DOWN");
        let home_pos = field("HOME_POS");
        let VanillaEntityMetadataDefault::Boolean(can_move_default) = can_move.default else {
            panic!("generated Creaking can-move default must be BOOLEAN");
        };
        let VanillaEntityMetadataDefault::Boolean(active_default) = active.default else {
            panic!("generated Creaking active default must be BOOLEAN");
        };
        let VanillaEntityMetadataDefault::Boolean(tearing_down_default) = tearing_down.default
        else {
            panic!("generated Creaking tearing-down default must be BOOLEAN");
        };
        assert_eq!(
            home_pos.default,
            VanillaEntityMetadataDefault::OptionalBlockPosNone
        );
        assert_eq!(
            vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID
            ),
            Some((can_move.data_id, can_move_default))
        );
        assert_eq!(
            vanilla_entity_metadata_creaking_active_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID
            ),
            Some((active.data_id, active_default))
        );
        assert_eq!(
            vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID
            ),
            Some((tearing_down.data_id, tearing_down_default))
        );
        assert_eq!(
            vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID
            ),
            Some(home_pos.data_id)
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_WARDEN_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(entity_type_id),
                None
            );
            assert_eq!(
                vanilla_entity_metadata_creaking_active_accessor_for_entity_type(entity_type_id),
                None
            );
            assert_eq!(
                vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
            assert_eq!(
                vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_copper_golem_accessors_own_membership_indices_and_defaults() {
        let copper_golem = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::CopperGolem);
        let weather_state = copper_golem
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_WEATHER_STATE")
            .expect("generated CopperGolem weather-state field");
        let state = copper_golem
            .own_fields
            .iter()
            .find(|field| field.name == "COPPER_GOLEM_STATE")
            .expect("generated CopperGolem animation-state field");
        let VanillaEntityMetadataDefault::Int(weather_state_default) = weather_state.default else {
            panic!("generated CopperGolem weather-state default must be enum INT");
        };
        let VanillaEntityMetadataDefault::Int(state_default) = state.default else {
            panic!("generated CopperGolem animation-state default must be enum INT");
        };
        let expected = VanillaCopperGolemMetadataAccessors {
            weather_state_data_id: weather_state.data_id,
            weather_state_default,
            state_data_id: state.data_id,
            state_default,
        };
        assert_eq!(
            vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
            ),
            Some(expected)
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_copper_golem_accessors_for_entity_type(entity_type_id),
                None
            );
        }
        assert_eq!(
            vanilla_entity_metadata_copper_golem_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
            ),
            Some((expected.state_data_id, expected.state_default))
        );
    }

    #[test]
    fn generated_sniffer_state_accessor_owns_membership_index_and_default() {
        let sniffer = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Sniffer);
        let state = sniffer
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_STATE")
            .expect("generated Sniffer state field");
        let VanillaEntityMetadataDefault::Int(state_default) = state.default else {
            panic!("generated Sniffer state default must be enum INT");
        };
        assert_eq!(
            vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SNIFFER_ID
            ),
            Some((state.data_id, state_default))
        );
        assert_eq!(
            vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ARMADILLO_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_sniffer_drop_seed_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_sniffer_drop_seed_at_tick_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SNIFFER_ID
            ),
            Some((19, 0))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_ARMADILLO_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_sniffer_drop_seed_at_tick_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_ominous_item_spawner_item_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID
            ),
            Some(8)
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_COW_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_item_entity_stack_accessor_owns_membership_index_and_empty_default() {
        let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ItemEntity);
        let item = schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_ITEM")
            .expect("generated ItemEntity item field");
        assert_eq!(item.default, VanillaEntityMetadataDefault::ItemStackEmpty);
        assert_eq!(
            vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ITEM_ID
            ),
            Some(item.data_id)
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
            VANILLA_ENTITY_TYPE_COW_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_armadillo_state_accessor_owns_membership_index_and_default() {
        let armadillo = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Armadillo);
        let state = armadillo
            .own_fields
            .iter()
            .find(|field| field.name == "ARMADILLO_STATE")
            .expect("generated Armadillo state field");
        let VanillaEntityMetadataDefault::Int(state_default) = state.default else {
            panic!("generated Armadillo state default must be enum INT");
        };
        assert_eq!(
            vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ARMADILLO_ID
            ),
            Some((state.data_id, state_default))
        );
        assert_eq!(
            vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SNIFFER_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_warden_anger_accessor_owns_membership_index_and_default() {
        let warden = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Warden);
        let anger = warden
            .own_fields
            .iter()
            .find(|field| field.name == "CLIENT_ANGER_LEVEL")
            .expect("generated Warden client-anger field");
        let VanillaEntityMetadataDefault::Int(anger_default) = anger.default else {
            panic!("generated Warden client-anger default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WARDEN_ID
            ),
            Some((anger.data_id, anger_default))
        );
        assert_eq!(
            vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_local_save_projection_accessors_own_membership_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_villager_data_finalized_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_VILLAGER_ID
            ),
            Some((VANILLA_VILLAGER_DATA_FINALIZED_DATA_ID, false))
        );
        assert_eq!(
            vanilla_entity_metadata_villager_data_finalized_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            ),
            Some((VANILLA_ZOMBIE_VILLAGER_DATA_FINALIZED_DATA_ID, false))
        );
        assert_eq!(
            vanilla_entity_metadata_villager_data_finalized_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WITHER_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID
            ),
            Some((8, 3.0))
        );
        assert_eq!(
            vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(-1),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_eye_of_ender_item_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID
            ),
            Some(8)
        );
        assert_eq!(
            vanilla_entity_metadata_eye_of_ender_item_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ITEM_ID
            ),
            None
        );
    }

    #[test]
    fn generated_ender_dragon_phase_accessor_owns_membership_index_and_default() {
        let dragon = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::EnderDragon);
        let phase = dragon
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_PHASE")
            .expect("generated EnderDragon phase field");
        let VanillaEntityMetadataDefault::Int(phase_default) = phase.default else {
            panic!("generated EnderDragon phase default must be INT");
        };
        assert_eq!(
            vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
            ),
            Some((phase.data_id, phase_default))
        );
        assert_eq!(
            vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WITHER_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_crossbow_and_dancing_accessors_separate_pillager_and_piglin_membership() {
        let piglin = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Piglin);
        let pillager = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Pillager);
        let pillager_charging = pillager
            .own_fields
            .iter()
            .find(|field| field.name == "IS_CHARGING_CROSSBOW")
            .expect("generated Pillager charging-crossbow field");
        let piglin_charging = piglin
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_IS_CHARGING_CROSSBOW")
            .expect("generated Piglin charging-crossbow field");
        let VanillaEntityMetadataDefault::Boolean(pillager_charging_default) =
            pillager_charging.default
        else {
            panic!("generated Pillager charging-crossbow default must be BOOLEAN");
        };
        let VanillaEntityMetadataDefault::Boolean(piglin_charging_default) =
            piglin_charging.default
        else {
            panic!("generated Piglin charging-crossbow default must be BOOLEAN");
        };
        let dancing = piglin
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_IS_DANCING")
            .expect("generated Piglin dancing field");
        let VanillaEntityMetadataDefault::Boolean(dancing_default) = dancing.default else {
            panic!("generated Piglin dancing default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PILLAGER_ID
            ),
            Some((pillager_charging.data_id, pillager_charging_default))
        );
        assert_eq!(
            vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_ID
            ),
            Some((piglin_charging.data_id, piglin_charging_default))
        );
        assert_eq!(
            vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_ID
            ),
            Some((dancing.data_id, dancing_default))
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
            assert_eq!(
                vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
            assert_eq!(
                vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_polar_bear_standing_accessor_owns_membership_index_and_default() {
        let polar_bear = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::PolarBear);
        let standing = polar_bear
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_STANDING_ID")
            .expect("generated PolarBear standing field");
        let VanillaEntityMetadataDefault::Boolean(default) = standing.default else {
            panic!("generated PolarBear standing default must be BOOLEAN");
        };
        assert_eq!(
            vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
            ),
            Some((standing.data_id, default))
        );
        assert_eq!(
            vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PANDA_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_wolf_accessors_own_membership_indices_and_defaults() {
        let wolf = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Wolf);
        let field = |name| {
            wolf.own_fields
                .iter()
                .find(|field| field.name == name)
                .unwrap_or_else(|| panic!("generated Wolf {name} field"))
        };
        let interested = field("DATA_INTERESTED_ID");
        let collar_color = field("DATA_COLLAR_COLOR");
        let anger_end_time = field("DATA_ANGER_END_TIME");
        let VanillaEntityMetadataDefault::Boolean(interested_default) = interested.default else {
            panic!("generated Wolf interested default must be BOOLEAN");
        };
        let VanillaEntityMetadataDefault::Int(collar_color_default) = collar_color.default else {
            panic!("generated Wolf collar-color default must be INT");
        };
        let VanillaEntityMetadataDefault::Long(anger_end_time_default) = anger_end_time.default
        else {
            panic!("generated Wolf anger-end-time default must be LONG");
        };
        let expected = VanillaWolfMetadataAccessors {
            interested_data_id: interested.data_id,
            interested_default,
            collar_color_data_id: collar_color.data_id,
            collar_color_default,
            anger_end_time_data_id: anger_end_time.data_id,
            anger_end_time_default,
        };
        assert_eq!(
            vanilla_entity_metadata_wolf_accessors_for_entity_type(VANILLA_ENTITY_TYPE_WOLF_ID),
            Some(expected)
        );
        assert_eq!(
            vanilla_entity_metadata_wolf_interested_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WOLF_ID
            ),
            Some((expected.interested_data_id, expected.interested_default))
        );
        assert_eq!(
            vanilla_entity_metadata_wolf_accessors_for_entity_type(VANILLA_ENTITY_TYPE_CAT_ID),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_wolf_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_tamable_animal_accessors_own_inherited_membership_indices_defaults_and_masks() {
        let expected = VanillaTamableAnimalMetadataAccessors {
            flags_data_id: VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
            flags_default: VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS,
            sitting_mask: VANILLA_TAMABLE_ANIMAL_FLAG_SITTING,
            tame_mask: VANILLA_TAMABLE_ANIMAL_FLAG_TAME,
            owner_data_id: VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
            owner_default: None,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_CAT_ID,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            VANILLA_ENTITY_TYPE_PARROT_ID,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
            assert_eq!(
                vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type(
                    entity_type_id
                ),
                Some((expected.flags_data_id, expected.flags_default)),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_abstract_arrow_accessors_own_inherited_membership_indices_defaults_and_masks() {
        let schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractArrow);
        let field = |name| {
            schema
                .own_fields
                .iter()
                .find(|field| field.name == name)
                .expect("generated AbstractArrow field")
        };
        let flags = field("ID_FLAGS");
        let pierce_level = field("PIERCE_LEVEL");
        let in_ground = field("IN_GROUND");
        let flag = |name| {
            schema
                .own_flags
                .iter()
                .find(|flag| flag.name == name)
                .expect("generated AbstractArrow flag")
        };
        let VanillaEntityMetadataDefault::Byte(flags_default) = flags.default else {
            panic!("generated AbstractArrow flags default must be BYTE");
        };
        let VanillaEntityMetadataDefault::Byte(pierce_level_default) = pierce_level.default else {
            panic!("generated AbstractArrow pierce-level default must be BYTE");
        };
        let VanillaEntityMetadataDefault::Boolean(in_ground_default) = in_ground.default else {
            panic!("generated AbstractArrow in-ground default must be BOOLEAN");
        };
        let expected = VanillaAbstractArrowMetadataAccessors {
            flags_data_id: flags.data_id,
            flags_default,
            crit_mask: flag("FLAG_CRIT").mask,
            no_physics_mask: flag("FLAG_NOPHYSICS").mask,
            pierce_level_data_id: pierce_level.data_id,
            pierce_level_default,
            in_ground_data_id: in_ground.data_id,
            in_ground_default,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ARROW_ID,
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_abstract_minecart_accessors_own_membership_indices_and_virtual_default() {
        for (entity_type_id, display_offset_default) in [
            (VANILLA_ENTITY_TYPE_MINECART_ID, 6),
            (VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, 8),
            (VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, 1),
            (VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, 6),
        ] {
            assert_eq!(
                vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(entity_type_id),
                Some(VanillaAbstractMinecartMetadataAccessors {
                    custom_display_block_data_id: 11,
                    display_offset_data_id: 12,
                    display_offset_default,
                }),
                "entity type {entity_type_id}"
            );
        }
        for entity_type_id in [VANILLA_ENTITY_TYPE_COW_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_zombie_accessors_own_inherited_membership_indices_and_defaults() {
        let expected = VanillaZombieMetadataAccessors {
            baby_data_id: 16,
            baby_default: false,
            special_type_data_id: 17,
            special_type_default: 0,
            drowned_conversion_data_id: 18,
            drowned_conversion_default: false,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ENTITY_TYPE_HUSK_ID,
            VANILLA_ENTITY_TYPE_DROWNED_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_zombie_accessors_for_entity_type(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            VANILLA_ENTITY_TYPE_ZOGLIN_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_zombie_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_arrow_accessors_own_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_arrow_accessors_for_entity_type(VANILLA_ENTITY_TYPE_ARROW_ID),
            Some(VanillaArrowMetadataAccessors {
                effect_color_data_id: 11,
                effect_color_default: -1,
            })
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_arrow_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_thrown_trident_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_TRIDENT_ID
            ),
            Some(VanillaThrownTridentMetadataAccessors {
                loyalty_data_id: 11,
                loyalty_default: 0,
                foil_data_id: 12,
                foil_default: false,
            })
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ARROW_ID,
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_abstract_boat_accessors_own_inherited_membership_indices_and_defaults() {
        let expected = VanillaAbstractBoatMetadataAccessors {
            paddle_left_data_id: VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID,
            paddle_left_default: VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT,
            paddle_right_data_id: VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID,
            paddle_right_default: VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT,
            bubble_time_data_id: VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID,
            bubble_time_default: VANILLA_ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_MINECART_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_vehicle_accessors_own_inherited_membership_indices_and_defaults() {
        let expected = VanillaVehicleMetadataAccessors {
            hurt_time_data_id: VANILLA_VEHICLE_HURT_TIME_DATA_ID,
            hurt_time_default: VANILLA_VEHICLE_DEFAULT_HURT_TIME,
            hurt_dir_data_id: VANILLA_VEHICLE_HURT_DIR_DATA_ID,
            hurt_dir_default: VANILLA_VEHICLE_DEFAULT_HURT_DIR,
            damage_data_id: VANILLA_VEHICLE_DAMAGE_DATA_ID,
            damage_default: VANILLA_VEHICLE_DEFAULT_DAMAGE,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            VANILLA_ENTITY_TYPE_MINECART_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_vehicle_accessors_for_entity_type(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_vehicle_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_vehicle_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_furnace_minecart_fuel_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
            ),
            Some((13, false))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_MINECART_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_primed_tnt_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_TNT_ID
            ),
            Some(VanillaPrimedTntMetadataAccessors {
                fuse_data_id: 8,
                fuse_default: 80,
                block_state_data_id: 9,
                block_state_default: VanillaEntityMetadataDefault::BlockStateDefault {
                    resource_id: "minecraft:tnt",
                    properties: &[("unstable", "false")],
                },
            })
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_TNT_MINECART_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_firework_rocket_accessors_own_membership_indices_and_defaults() {
        let expected = VanillaFireworkRocketMetadataAccessors {
            item_data_id: VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID,
            item_default_resource_id: VANILLA_FIREWORK_ROCKET_DEFAULT_ITEM_RESOURCE_ID,
            attached_to_target_data_id: VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID,
            attached_to_target_default: None,
            shot_at_angle_data_id: VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID,
            shot_at_angle_default: VANILLA_FIREWORK_ROCKET_DEFAULT_SHOT_AT_ANGLE,
        };
        assert_eq!(
            vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID
            ),
            Some(expected)
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_TNT_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_experience_orb_value_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID
            ),
            Some((
                VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID,
                VANILLA_EXPERIENCE_ORB_DEFAULT_VALUE,
            ))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_ITEM_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_guardian_accessors_own_inherited_membership_indices_and_defaults() {
        let guardian_schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Guardian);
        let moving = guardian_schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_ID_MOVING")
            .expect("generated Guardian moving field");
        let attack_target = guardian_schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_ID_ATTACK_TARGET")
            .expect("generated Guardian attack-target field");
        let VanillaEntityMetadataDefault::Boolean(moving_default) = moving.default else {
            panic!("generated Guardian moving default must be boolean")
        };
        let VanillaEntityMetadataDefault::Int(attack_target_default) = attack_target.default else {
            panic!("generated Guardian attack-target default must be int")
        };
        let expected = VanillaGuardianMetadataAccessors {
            moving_data_id: moving.data_id,
            moving_default,
            attack_target_data_id: attack_target.data_id,
            attack_target_default,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_GUARDIAN_ID,
            VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_guardian_accessors_for_entity_type(entity_type_id),
                Some(expected),
                "entity type {entity_type_id}"
            );
        }
        assert_eq!(
            vanilla_entity_metadata_guardian_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_guardian_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_shulker_accessors_own_membership_indices_and_defaults() {
        let shulker_schema = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Shulker);
        let attach_face = shulker_schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_ATTACH_FACE_ID")
            .expect("generated Shulker attach-face field");
        let peek = shulker_schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_PEEK_ID")
            .expect("generated Shulker peek field");
        let color = shulker_schema
            .own_fields
            .iter()
            .find(|field| field.name == "DATA_COLOR_ID")
            .expect("generated Shulker color field");
        let VanillaEntityMetadataDefault::Direction(attach_face_default) = attach_face.default
        else {
            panic!("generated Shulker attach-face default must be direction")
        };
        let VanillaEntityMetadataDefault::Byte(peek_default) = peek.default else {
            panic!("generated Shulker peek default must be byte")
        };
        let VanillaEntityMetadataDefault::Byte(color_default) = color.default else {
            panic!("generated Shulker color default must be byte")
        };
        assert_eq!(
            vanilla_entity_metadata_shulker_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_SHULKER_ID
            ),
            Some(VanillaShulkerMetadataAccessors {
                attach_face_data_id: attach_face.data_id,
                attach_face_default,
                peek_data_id: peek.data_id,
                peek_default,
                color_data_id: color.data_id,
                color_default,
            })
        );
        assert_eq!(
            vanilla_entity_metadata_shulker_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_shulker_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_armor_stand_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
            ),
            Some(VanillaArmorStandMetadataAccessors {
                client_flags_data_id: VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
                client_flags_default: VANILLA_ARMOR_STAND_DEFAULT_CLIENT_FLAGS,
                small_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL,
                show_arms_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS,
                no_baseplate_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
                marker_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER,
                head_pose_data_id: VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID,
                head_pose_default: VANILLA_ARMOR_STAND_DEFAULT_HEAD_POSE,
                body_pose_data_id: VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID,
                body_pose_default: VANILLA_ARMOR_STAND_DEFAULT_BODY_POSE,
                left_arm_pose_data_id: VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
                left_arm_pose_default: VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE,
                right_arm_pose_data_id: VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
                right_arm_pose_default: VANILLA_ARMOR_STAND_DEFAULT_RIGHT_ARM_POSE,
                left_leg_pose_data_id: VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID,
                left_leg_pose_default: VANILLA_ARMOR_STAND_DEFAULT_LEFT_LEG_POSE,
                right_leg_pose_data_id: VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
                right_leg_pose_default: VANILLA_ARMOR_STAND_DEFAULT_RIGHT_LEG_POSE,
            })
        );
        assert_eq!(
            vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_armor_stand_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_interaction_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_interaction_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_INTERACTION_ID
            ),
            Some(VanillaInteractionMetadataAccessors {
                width_data_id: VANILLA_INTERACTION_WIDTH_DATA_ID,
                width_default: VANILLA_INTERACTION_DEFAULT_WIDTH,
                height_data_id: VANILLA_INTERACTION_HEIGHT_DATA_ID,
                height_default: VANILLA_INTERACTION_DEFAULT_HEIGHT,
                response_data_id: VANILLA_INTERACTION_RESPONSE_DATA_ID,
                response_default: VANILLA_INTERACTION_DEFAULT_RESPONSE,
            })
        );
        assert_eq!(
            vanilla_entity_metadata_interaction_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_interaction_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_creeper_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_creeper_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CREEPER_ID
            ),
            Some(VanillaCreeperMetadataAccessors {
                swell_dir_data_id: VANILLA_CREEPER_SWELL_DIR_DATA_ID,
                swell_dir_default: VANILLA_CREEPER_DEFAULT_SWELL_DIR,
                powered_data_id: VANILLA_CREEPER_POWERED_DATA_ID,
                powered_default: VANILLA_CREEPER_DEFAULT_POWERED,
                ignited_data_id: VANILLA_CREEPER_IGNITED_DATA_ID,
                ignited_default: VANILLA_CREEPER_DEFAULT_IGNITED,
            })
        );
        assert_eq!(
            vanilla_entity_metadata_creeper_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_creeper_accessors_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_panda_accessors_own_membership_indices_defaults_and_masks() {
        assert_eq!(
            vanilla_entity_metadata_panda_accessors_for_entity_type(VANILLA_ENTITY_TYPE_PANDA_ID),
            Some(VanillaPandaMetadataAccessors {
                unhappy_counter_data_id: VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID,
                unhappy_counter_default: VANILLA_PANDA_DEFAULT_UNHAPPY_COUNTER,
                sneeze_counter_data_id: VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID,
                sneeze_counter_default: VANILLA_PANDA_DEFAULT_SNEEZE_COUNTER,
                eat_counter_data_id: VANILLA_PANDA_EAT_COUNTER_DATA_ID,
                eat_counter_default: VANILLA_PANDA_DEFAULT_EAT_COUNTER,
                main_gene_data_id: VANILLA_PANDA_MAIN_GENE_DATA_ID,
                main_gene_default: VANILLA_PANDA_DEFAULT_MAIN_GENE,
                hidden_gene_data_id: VANILLA_PANDA_HIDDEN_GENE_DATA_ID,
                hidden_gene_default: VANILLA_PANDA_DEFAULT_HIDDEN_GENE,
                flags_data_id: VANILLA_PANDA_FLAGS_DATA_ID,
                flags_default: VANILLA_PANDA_DEFAULT_FLAGS,
                sneezing_mask: VANILLA_PANDA_FLAG_SNEEZING,
                rolling_mask: VANILLA_PANDA_FLAG_ROLLING,
                sitting_mask: VANILLA_PANDA_FLAG_SITTING,
                on_back_mask: VANILLA_PANDA_FLAG_ON_BACK,
            })
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_CHICKEN_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_panda_flags_accessor_delegates_to_bundle() {
        let accessors =
            vanilla_entity_metadata_panda_accessors_for_entity_type(VANILLA_ENTITY_TYPE_PANDA_ID)
                .expect("generated Panda metadata accessors must resolve");
        assert_eq!(
            vanilla_entity_metadata_panda_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PANDA_ID
            ),
            Some((accessors.flags_data_id, accessors.flags_default))
        );
        assert_eq!(
            vanilla_entity_metadata_panda_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_panda_flags_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_abstract_horse_accessors_own_membership_index_default_and_masks() {
        let expected = VanillaAbstractHorseMetadataAccessors {
            flags_data_id: VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
            flags_default: VANILLA_ABSTRACT_HORSE_DEFAULT_FLAGS,
            tame_mask: VANILLA_ABSTRACT_HORSE_FLAG_TAME,
            bred_mask: VANILLA_ABSTRACT_HORSE_FLAG_BRED,
            eating_mask: VANILLA_ABSTRACT_HORSE_FLAG_EATING,
            standing_mask: VANILLA_ABSTRACT_HORSE_FLAG_STANDING,
            open_mouth_mask: VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH,
        };
        for entity_type_id in [VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_CAMEL_ID] {
            assert_eq!(
                vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id),
                Some(expected)
            );
        }
        assert_eq!(
            vanilla_entity_metadata_abstract_horse_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_HORSE_ID
            ),
            Some((expected.flags_data_id, expected.flags_default))
        );
        for entity_type_id in [VANILLA_ENTITY_TYPE_CHICKEN_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_equine_accessors_own_membership_indices_and_defaults() {
        assert_eq!(
            vanilla_entity_metadata_horse_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_HORSE_ID
            ),
            Some((VANILLA_HORSE_VARIANT_DATA_ID, VANILLA_HORSE_DEFAULT_VARIANT))
        );
        assert_eq!(
            vanilla_entity_metadata_horse_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_DONKEY_ID
            ),
            None
        );

        for entity_type_id in [
            VANILLA_ENTITY_TYPE_DONKEY_ID,
            VANILLA_ENTITY_TYPE_MULE_ID,
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(
                    entity_type_id
                ),
                Some((
                    VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID,
                    VANILLA_ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST,
                ))
            );
        }
        assert_eq!(
            vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_HORSE_ID
            ),
            None
        );

        for entity_type_id in [
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_llama_accessors_for_entity_type(entity_type_id),
                Some(VanillaLlamaMetadataAccessors {
                    strength_data_id: VANILLA_LLAMA_STRENGTH_DATA_ID,
                    strength_default: VANILLA_LLAMA_DEFAULT_STRENGTH,
                    variant_data_id: VANILLA_LLAMA_VARIANT_DATA_ID,
                    variant_default: VANILLA_LLAMA_DEFAULT_VARIANT,
                })
            );
        }
        for entity_type_id in [VANILLA_ENTITY_TYPE_HORSE_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_llama_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_farm_animal_variant_accessors_own_membership_indices_and_defaults() {
        for (entity_type_id, expected) in [
            (
                VANILLA_ENTITY_TYPE_CHICKEN_ID,
                VanillaFarmAnimalVariantMetadataAccessors {
                    variant_data_id: VANILLA_CHICKEN_VARIANT_DATA_ID,
                    variant_default_resource_id: VANILLA_CHICKEN_DEFAULT_VARIANT_RESOURCE_ID,
                    sound_variant_data_id: VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID,
                    sound_variant_default_resource_id:
                        VANILLA_CHICKEN_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
                },
            ),
            (
                VANILLA_ENTITY_TYPE_COW_ID,
                VanillaFarmAnimalVariantMetadataAccessors {
                    variant_data_id: VANILLA_COW_VARIANT_DATA_ID,
                    variant_default_resource_id: VANILLA_COW_DEFAULT_VARIANT_RESOURCE_ID,
                    sound_variant_data_id: VANILLA_COW_SOUND_VARIANT_DATA_ID,
                    sound_variant_default_resource_id:
                        VANILLA_COW_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
                },
            ),
            (
                VANILLA_ENTITY_TYPE_PIG_ID,
                VanillaFarmAnimalVariantMetadataAccessors {
                    variant_data_id: VANILLA_PIG_VARIANT_DATA_ID,
                    variant_default_resource_id: VANILLA_PIG_DEFAULT_VARIANT_RESOURCE_ID,
                    sound_variant_data_id: VANILLA_PIG_SOUND_VARIANT_DATA_ID,
                    sound_variant_default_resource_id:
                        VANILLA_PIG_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
                },
            ),
        ] {
            assert_eq!(
                vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
                    entity_type_id
                ),
                Some(expected)
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_companion_variant_accessors_own_membership_indices_and_defaults() {
        for (entity_type_id, expected) in [
            (
                VANILLA_ENTITY_TYPE_CAT_ID,
                VanillaCompanionVariantMetadataAccessors {
                    variant_data_id: VANILLA_CAT_VARIANT_DATA_ID,
                    variant_default_resource_id: VANILLA_CAT_DEFAULT_VARIANT_RESOURCE_ID,
                    sound_variant_data_id: VANILLA_CAT_SOUND_VARIANT_DATA_ID,
                    sound_variant_default_resource_id:
                        VANILLA_CAT_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
                },
            ),
            (
                VANILLA_ENTITY_TYPE_WOLF_ID,
                VanillaCompanionVariantMetadataAccessors {
                    variant_data_id: VANILLA_WOLF_VARIANT_DATA_ID,
                    variant_default_resource_id: VANILLA_WOLF_DEFAULT_VARIANT_RESOURCE_ID,
                    sound_variant_data_id: VANILLA_WOLF_SOUND_VARIANT_DATA_ID,
                    sound_variant_default_resource_id:
                        VANILLA_WOLF_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
                },
            ),
        ] {
            assert_eq!(
                vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity_type_id),
                Some(expected)
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_zombie_nautilus_variant_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
            ),
            Some((
                VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
                VANILLA_ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID,
            ))
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            VANILLA_ENTITY_TYPE_DROWNED_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
                    entity_type_id
                ),
                None
            );
        }
    }

    #[test]
    fn generated_frog_variant_accessor_owns_membership_index_and_default() {
        assert_eq!(
            vanilla_entity_metadata_frog_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_FROG_ID
            ),
            Some((
                VANILLA_FROG_VARIANT_DATA_ID,
                VANILLA_FROG_DEFAULT_VARIANT_RESOURCE_ID,
            ))
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_TADPOLE_ID,
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_frog_variant_accessor_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_mob_accessors_own_membership_index_default_and_masks() {
        let expected = VanillaMobMetadataAccessors {
            flags_data_id: VANILLA_MOB_FLAGS_DATA_ID,
            flags_default: 0,
            no_ai_mask: VANILLA_MOB_FLAG_NO_AI,
            left_handed_mask: VANILLA_MOB_FLAG_LEFT_HANDED,
            aggressive_mask: VANILLA_MOB_FLAG_AGGRESSIVE,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id),
                Some(expected)
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_living_entity_accessors_own_membership_indices_defaults_and_masks() {
        let expected = VanillaLivingEntityMetadataAccessors {
            flags_data_id: VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
            flags_default: 0,
            is_using_mask: VANILLA_LIVING_ENTITY_FLAG_IS_USING,
            off_hand_mask: VANILLA_LIVING_ENTITY_FLAG_OFF_HAND,
            spin_attack_mask: VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK,
            health_data_id: VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
            health_default: 1.0,
            arrow_count_data_id: VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID,
            arrow_count_default: 0,
            stinger_count_data_id: VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID,
            stinger_count_default: 0,
            sleeping_pos_data_id: VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
        };
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id),
                Some(expected)
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ARROW_ID,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id),
                None
            );
        }
    }

    #[test]
    fn generated_entity_accessors_own_indices_defaults_and_masks() {
        assert_eq!(
            vanilla_entity_metadata_entity_accessors(),
            Some(VanillaEntityMetadataAccessors {
                shared_flags_data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
                shared_flags_default: 0,
                on_fire_mask: VANILLA_ENTITY_SHARED_FLAG_ON_FIRE,
                shift_key_down_mask: VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
                sprinting_mask: VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
                swimming_mask: VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
                invisible_mask: VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
                glowing_mask: VANILLA_ENTITY_SHARED_FLAG_GLOWING,
                fall_flying_mask: VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
                air_supply_data_id: VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
                custom_name_data_id: VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
                custom_name_visible_data_id: VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID,
                custom_name_visible_default: false,
                silent_data_id: VANILLA_ENTITY_SILENT_DATA_ID,
                silent_default: false,
                no_gravity_data_id: VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
                no_gravity_default: false,
                pose_data_id: VANILLA_ENTITY_POSE_DATA_ID,
                pose_default: 0,
                ticks_frozen_data_id: VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
                ticks_frozen_default: 0,
            })
        );
    }

    #[test]
    fn generated_vex_flags_accessor_owns_membership_index_default_and_mask() {
        assert_eq!(
            vanilla_entity_metadata_vex_flags_accessor_for_entity_type(VANILLA_ENTITY_TYPE_VEX_ID),
            Some((16, 0, 1))
        );
        assert_eq!(
            vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_vex_flags_accessor_for_entity_type(-1),
            None
        );
    }

    #[test]
    fn generated_spellcaster_casting_accessor_owns_membership_index_and_default() {
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_EVOKER_ID,
            VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
                    entity_type_id
                ),
                Some((17, 0)),
                "entity type {entity_type_id}"
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
                    entity_type_id
                ),
                None,
                "entity type {entity_type_id}"
            );
        }
    }

    #[test]
    fn generated_raider_celebrating_accessor_owns_membership_index_and_default() {
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_EVOKER_ID,
            VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type(entity_type_id),
                Some((16, false)),
                "entity type {entity_type_id}"
            );
        }
        for entity_type_id in [VANILLA_ENTITY_TYPE_CHICKEN_ID, -1] {
            assert_eq!(
                vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type(entity_type_id),
                None,
                "entity type {entity_type_id}"
            );
        }
    }

    #[test]
    fn generated_abstract_villager_unhappy_accessor_owns_membership_index_and_default() {
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        ] {
            assert_eq!(
                vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
                    entity_type_id
                ),
                Some((18, 0)),
                "entity type {entity_type_id}"
            );
        }
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            VANILLA_ENTITY_TYPE_PANDA_ID,
            -1,
        ] {
            assert_eq!(
                vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
                    entity_type_id
                ),
                None,
                "entity type {entity_type_id}"
            );
        }
    }

    #[test]
    fn generated_resolved_flags_are_nonzero_unique_and_byte_bound() {
        let accessors = VANILLA_ENTITY_METADATA_ACCESSORS_26_1
            .iter()
            .map(|entry| ((entry.owner, entry.java_name), entry))
            .collect::<HashMap<_, _>>();
        let mut raw_masks = HashSet::new();
        for flag in VANILLA_ENTITY_METADATA_FLAGS_26_1 {
            match flag.status {
                VanillaEntityMetadataFlagStatus::Resolved => {
                    let value = flag.value.expect("resolved flag value");
                    let accessor_name = flag.accessor_name.expect("resolved flag accessor");
                    assert_ne!(value, 0);
                    assert!(raw_masks.insert((flag.owner, accessor_name, value)));
                    assert_eq!(
                        accessors[&(flag.owner, accessor_name)].serializer_id,
                        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
                    );
                    assert_eq!(flag.diagnostic, None);
                }
                VanillaEntityMetadataFlagStatus::Ambiguous => {
                    assert!(flag.diagnostic.is_some());
                }
            }
        }

        for family in [
            VanillaEntityMetadataFamily::Entity,
            VanillaEntityMetadataFamily::LivingEntity,
            VanillaEntityMetadataFamily::Mob,
            VanillaEntityMetadataFamily::Allay,
            VanillaEntityMetadataFamily::Creeper,
            VanillaEntityMetadataFamily::Warden,
            VanillaEntityMetadataFamily::WitherBoss,
            VanillaEntityMetadataFamily::EnderMan,
            VanillaEntityMetadataFamily::Slime,
            VanillaEntityMetadataFamily::Shulker,
            VanillaEntityMetadataFamily::Zombie,
            VanillaEntityMetadataFamily::AgeableMob,
            VanillaEntityMetadataFamily::Armadillo,
            VanillaEntityMetadataFamily::Axolotl,
            VanillaEntityMetadataFamily::Bee,
            VanillaEntityMetadataFamily::Dolphin,
            VanillaEntityMetadataFamily::GlowSquid,
            VanillaEntityMetadataFamily::HappyGhast,
            VanillaEntityMetadataFamily::Goat,
            VanillaEntityMetadataFamily::Fox,
            VanillaEntityMetadataFamily::CopperGolem,
            VanillaEntityMetadataFamily::Ocelot,
            VanillaEntityMetadataFamily::Panda,
            VanillaEntityMetadataFamily::PolarBear,
            VanillaEntityMetadataFamily::Sheep,
            VanillaEntityMetadataFamily::Sniffer,
            VanillaEntityMetadataFamily::Turtle,
            VanillaEntityMetadataFamily::AbstractHorse,
            VanillaEntityMetadataFamily::Camel,
            VanillaEntityMetadataFamily::Horse,
            VanillaEntityMetadataFamily::AbstractChestedHorse,
            VanillaEntityMetadataFamily::Llama,
            VanillaEntityMetadataFamily::TamableAnimal,
            VanillaEntityMetadataFamily::Cat,
            VanillaEntityMetadataFamily::Wolf,
            VanillaEntityMetadataFamily::Chicken,
            VanillaEntityMetadataFamily::Cow,
            VanillaEntityMetadataFamily::MushroomCow,
            VanillaEntityMetadataFamily::Pig,
            VanillaEntityMetadataFamily::Frog,
            VanillaEntityMetadataFamily::AbstractNautilus,
            VanillaEntityMetadataFamily::ZombieNautilus,
            VanillaEntityMetadataFamily::Parrot,
            VanillaEntityMetadataFamily::Rabbit,
            VanillaEntityMetadataFamily::AbstractVillager,
            VanillaEntityMetadataFamily::ArmorStand,
            VanillaEntityMetadataFamily::AbstractArrow,
            VanillaEntityMetadataFamily::Arrow,
            VanillaEntityMetadataFamily::ThrownTrident,
            VanillaEntityMetadataFamily::WitherSkull,
            VanillaEntityMetadataFamily::EndCrystal,
            VanillaEntityMetadataFamily::EnderDragon,
            VanillaEntityMetadataFamily::Interaction,
            VanillaEntityMetadataFamily::ExperienceOrb,
            VanillaEntityMetadataFamily::ItemEntity,
            VanillaEntityMetadataFamily::PrimedTnt,
            VanillaEntityMetadataFamily::FireworkRocket,
            VanillaEntityMetadataFamily::FishingHook,
            VanillaEntityMetadataFamily::OminousItemSpawner,
            VanillaEntityMetadataFamily::Guardian,
            VanillaEntityMetadataFamily::SnowGolem,
            VanillaEntityMetadataFamily::IronGolem,
            VanillaEntityMetadataFamily::Bat,
            VanillaEntityMetadataFamily::Phantom,
            VanillaEntityMetadataFamily::Bogged,
            VanillaEntityMetadataFamily::Skeleton,
            VanillaEntityMetadataFamily::Ghast,
            VanillaEntityMetadataFamily::Vex,
            VanillaEntityMetadataFamily::Creaking,
            VanillaEntityMetadataFamily::Strider,
            VanillaEntityMetadataFamily::Zoglin,
            VanillaEntityMetadataFamily::Hoglin,
            VanillaEntityMetadataFamily::AbstractPiglin,
            VanillaEntityMetadataFamily::Piglin,
            VanillaEntityMetadataFamily::Raider,
            VanillaEntityMetadataFamily::Pillager,
            VanillaEntityMetadataFamily::SpellcasterIllager,
            VanillaEntityMetadataFamily::AbstractFish,
            VanillaEntityMetadataFamily::Salmon,
            VanillaEntityMetadataFamily::TropicalFish,
            VanillaEntityMetadataFamily::Pufferfish,
            VanillaEntityMetadataFamily::Tadpole,
            VanillaEntityMetadataFamily::VehicleEntity,
            VanillaEntityMetadataFamily::AbstractBoat,
            VanillaEntityMetadataFamily::AbstractMinecart,
            VanillaEntityMetadataFamily::MinecartFurnace,
            VanillaEntityMetadataFamily::HangingEntity,
            VanillaEntityMetadataFamily::ItemFrame,
            VanillaEntityMetadataFamily::Painting,
            VanillaEntityMetadataFamily::Avatar,
            VanillaEntityMetadataFamily::Player,
        ] {
            let flags = vanilla_entity_metadata_schema(family).own_flags;
            for (index, flag) in flags.iter().enumerate() {
                assert_ne!(flag.mask, 0);
                assert!(flags[..index]
                    .iter()
                    .all(|previous| previous.mask != flag.mask));
                assert_eq!(
                    vanilla_entity_metadata_field(family, flag.data_id)
                        .map(|field| field.serializer_id),
                    Some(VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID)
                );
            }
        }
        assert!(VANILLA_ENTITY_METADATA_FLAG_RESOLVER_EVIDENCE_26_1
            .iter()
            .all(|entry| !entry.java_class.is_empty()
                && !entry.accessor_name.is_empty()
                && !entry.evidence_member.is_empty()
                && !entry.source_expression.is_empty()
                && !entry.reason.is_empty()));
    }

    #[test]
    fn compatibility_queries_and_virtual_defaults_preserve_existing_semantics() {
        let arrow = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Arrow);
        let allay = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Allay);
        assert_eq!(allay.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(allay.inherited_field_count, 16);
        assert_eq!(allay.own_fields.len(), 2);
        let (dancing_data_id, dancing_default) =
            vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ALLAY_ID,
            )
            .expect("generated Allay dancing accessor");
        assert_eq!(dancing_data_id, allay.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(dancing_default),
            allay.own_fields[0].default
        );
        assert_eq!(VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID, 17);
        assert!(VANILLA_ALLAY_DEFAULT_CAN_DUPLICATE);
        let creeper = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Creeper);
        assert_eq!(creeper.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(creeper.inherited_field_count, 16);
        assert_eq!(creeper.own_fields.len(), 3);
        assert_eq!(VANILLA_CREEPER_SWELL_DIR_DATA_ID, 16);
        assert_eq!(VANILLA_CREEPER_POWERED_DATA_ID, 17);
        assert_eq!(VANILLA_CREEPER_IGNITED_DATA_ID, 18);
        assert_eq!(VANILLA_CREEPER_DEFAULT_SWELL_DIR, -1);
        assert!(!VANILLA_CREEPER_DEFAULT_POWERED);
        assert!(!VANILLA_CREEPER_DEFAULT_IGNITED);
        let warden = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Warden);
        assert_eq!(warden.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(warden.inherited_field_count, 16);
        assert_eq!(warden.own_fields.len(), 1);
        let (anger_data_id, anger_default) =
            vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WARDEN_ID,
            )
            .expect("generated Warden client-anger accessor");
        assert_eq!(anger_data_id, warden.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(anger_default),
            warden.own_fields[0].default
        );
        let wither = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::WitherBoss);
        assert_eq!(wither.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(wither.inherited_field_count, 16);
        assert_eq!(wither.own_fields.len(), 4);
        assert_eq!(VANILLA_WITHER_TARGET_A_DATA_ID, 16);
        assert_eq!(VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID, 19);
        assert_eq!(VANILLA_WITHER_DEFAULT_TARGET_A, 0);
        assert_eq!(VANILLA_WITHER_DEFAULT_INVULNERABLE_TICKS, 0);
        let side_heads = vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_WITHER_ID,
        )
        .expect("generated WitherBoss side-head target accessors");
        assert_eq!(side_heads.target_b_data_id, wither.own_fields[1].data_id);
        assert_eq!(side_heads.target_c_data_id, wither.own_fields[2].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(side_heads.target_b_default),
            wither.own_fields[1].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Int(side_heads.target_c_default),
            wither.own_fields[2].default
        );
        let enderman = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::EnderMan);
        assert_eq!(enderman.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(enderman.inherited_field_count, 16);
        assert_eq!(enderman.own_fields.len(), 3);
        assert_eq!(VANILLA_ENDERMAN_CARRY_STATE_DATA_ID, 16);
        assert_eq!(VANILLA_ENDERMAN_CREEPY_DATA_ID, 17);
        assert_eq!(VANILLA_ENDERMAN_STARED_AT_DATA_ID, 18);
        assert_eq!(VANILLA_ENDERMAN_DEFAULT_CARRY_STATE, None);
        assert!(!VANILLA_ENDERMAN_DEFAULT_CREEPY);
        assert!(!VANILLA_ENDERMAN_DEFAULT_STARED_AT);
        let armadillo = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Armadillo);
        assert_eq!(
            armadillo.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(armadillo.inherited_field_count, 18);
        assert_eq!(armadillo.own_fields.len(), 1);
        let (state_data_id, state_default) =
            vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            )
            .expect("generated Armadillo state accessor");
        assert_eq!(state_data_id, armadillo.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(state_default),
            armadillo.own_fields[0].default
        );
        assert_eq!(VANILLA_ARMADILLO_STATE_IDLE_ID, 0);
        assert_eq!(VANILLA_ARMADILLO_STATE_ROLLING_ID, 1);
        assert_eq!(VANILLA_ARMADILLO_STATE_SCARED_ID, 2);
        assert_eq!(VANILLA_ARMADILLO_STATE_UNROLLING_ID, 3);
        let axolotl = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Axolotl);
        assert_eq!(
            axolotl.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(axolotl.inherited_field_count, 18);
        assert_eq!(axolotl.own_fields.len(), 3);
        let (variant_data_id, variant_default) =
            vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            )
            .expect("generated Axolotl variant accessor");
        assert_eq!(variant_data_id, axolotl.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(variant_default),
            axolotl.own_fields[0].default
        );
        let (playing_dead_data_id, playing_dead_default) =
            vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            )
            .expect("generated Axolotl playing-dead accessor");
        assert_eq!(playing_dead_data_id, axolotl.own_fields[1].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(playing_dead_default),
            axolotl.own_fields[1].default
        );
        assert_eq!(VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID, 20);
        assert!(!VANILLA_AXOLOTL_DEFAULT_FROM_BUCKET);
        let bee = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Bee);
        assert_eq!(bee.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(bee.inherited_field_count, 18);
        assert_eq!(bee.own_fields.len(), 2);
        assert_eq!(VANILLA_BEE_FLAGS_DATA_ID, 18);
        assert_eq!(VANILLA_BEE_ANGER_END_TIME_DATA_ID, 19);
        assert_eq!(VANILLA_BEE_DEFAULT_FLAGS, 0);
        assert_eq!(VANILLA_BEE_DEFAULT_ANGER_END_TIME, -1);
        assert_eq!(VANILLA_BEE_FLAG_ROLL, 2);
        assert_eq!(VANILLA_BEE_FLAG_HAS_STUNG, 4);
        assert_eq!(VANILLA_BEE_FLAG_HAS_NECTAR, 8);
        let dolphin = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Dolphin);
        assert_eq!(
            dolphin.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(dolphin.inherited_field_count, 18);
        assert_eq!(dolphin.own_fields.len(), 2);
        assert_eq!(VANILLA_DOLPHIN_GOT_FISH_DATA_ID, 18);
        assert_eq!(VANILLA_DOLPHIN_MOISTNESS_DATA_ID, 19);
        assert!(!VANILLA_DOLPHIN_DEFAULT_GOT_FISH);
        assert_eq!(VANILLA_DOLPHIN_DEFAULT_MOISTNESS, 2400);
        let glow_squid = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::GlowSquid);
        assert_eq!(
            glow_squid.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(glow_squid.inherited_field_count, 18);
        assert_eq!(glow_squid.own_fields.len(), 1);
        assert_eq!(VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID, 18);
        assert_eq!(VANILLA_GLOW_SQUID_DEFAULT_DARK_TICKS, 0);
        let happy_ghast = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::HappyGhast);
        assert_eq!(
            happy_ghast.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(happy_ghast.inherited_field_count, 18);
        assert_eq!(happy_ghast.own_fields.len(), 2);
        assert_eq!(VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID, 18);
        assert_eq!(VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID, 19);
        assert!(!VANILLA_HAPPY_GHAST_DEFAULT_IS_LEASH_HOLDER);
        assert!(!VANILLA_HAPPY_GHAST_DEFAULT_STAYS_STILL);
        let goat = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Goat);
        assert_eq!(goat.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(goat.inherited_field_count, 18);
        assert_eq!(goat.own_fields.len(), 3);
        assert_eq!(VANILLA_GOAT_SCREAMING_DATA_ID, 18);
        assert_eq!(VANILLA_GOAT_LEFT_HORN_DATA_ID, 19);
        assert_eq!(VANILLA_GOAT_RIGHT_HORN_DATA_ID, 20);
        assert!(!VANILLA_GOAT_DEFAULT_SCREAMING);
        assert!(VANILLA_GOAT_DEFAULT_HAS_LEFT_HORN);
        assert!(VANILLA_GOAT_DEFAULT_HAS_RIGHT_HORN);
        let fox = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Fox);
        assert_eq!(fox.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(fox.inherited_field_count, 18);
        assert_eq!(fox.own_fields.len(), 4);
        assert_eq!(fox.own_flags.len(), 7);
        assert_eq!(VANILLA_FOX_TYPE_DATA_ID, 18);
        assert_eq!(VANILLA_FOX_FLAGS_DATA_ID, 19);
        assert_eq!(VANILLA_FOX_TRUSTED_DATA_ID_0, 20);
        assert_eq!(VANILLA_FOX_TRUSTED_DATA_ID_1, 21);
        assert_eq!(VANILLA_FOX_DEFAULT_TYPE, 0);
        assert_eq!(VANILLA_FOX_DEFAULT_FLAGS, 0);
        assert_eq!(VANILLA_FOX_FLAG_SITTING, 1);
        assert_eq!(VANILLA_FOX_FLAG_CROUCHING, 4);
        assert_eq!(VANILLA_FOX_FLAG_INTERESTED, 8);
        assert_eq!(VANILLA_FOX_FLAG_POUNCING, 16);
        assert_eq!(VANILLA_FOX_FLAG_SLEEPING, 32);
        assert_eq!(VANILLA_FOX_FLAG_FACEPLANTED, 64);
        assert_eq!(VANILLA_FOX_FLAG_DEFENDING, -128);
        let copper_golem = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::CopperGolem);
        assert_eq!(copper_golem.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(copper_golem.inherited_field_count, 16);
        assert_eq!(copper_golem.own_fields.len(), 2);
        let accessors = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        )
        .expect("generated CopperGolem metadata accessors");
        assert_eq!(
            accessors.weather_state_data_id,
            copper_golem.own_fields[0].data_id
        );
        assert_eq!(accessors.state_data_id, copper_golem.own_fields[1].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(accessors.weather_state_default),
            copper_golem.own_fields[0].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Int(accessors.state_default),
            copper_golem.own_fields[1].default
        );
        assert_eq!(VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID, 0);
        assert_eq!(VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID, 1);
        assert_eq!(VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID, 2);
        assert_eq!(VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID, 3);
        assert_eq!(VANILLA_COPPER_GOLEM_STATE_IDLE_ID, 0);
        assert_eq!(VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID, 1);
        assert_eq!(VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID, 2);
        assert_eq!(VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID, 3);
        assert_eq!(VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID, 4);
        let ocelot = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Ocelot);
        assert_eq!(ocelot.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(ocelot.inherited_field_count, 18);
        assert_eq!(ocelot.own_fields.len(), 1);
        assert_eq!(VANILLA_OCELOT_TRUSTING_DATA_ID, 18);
        assert!(!VANILLA_OCELOT_DEFAULT_TRUSTING);
        let panda = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Panda);
        assert_eq!(panda.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(panda.inherited_field_count, 18);
        assert_eq!(panda.own_fields.len(), 6);
        assert_eq!(VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID, 18);
        assert_eq!(VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID, 19);
        assert_eq!(VANILLA_PANDA_EAT_COUNTER_DATA_ID, 20);
        assert_eq!(VANILLA_PANDA_MAIN_GENE_DATA_ID, 21);
        assert_eq!(VANILLA_PANDA_HIDDEN_GENE_DATA_ID, 22);
        assert_eq!(VANILLA_PANDA_FLAGS_DATA_ID, 23);
        assert_eq!(VANILLA_PANDA_DEFAULT_UNHAPPY_COUNTER, 0);
        assert_eq!(VANILLA_PANDA_DEFAULT_SNEEZE_COUNTER, 0);
        assert_eq!(VANILLA_PANDA_DEFAULT_EAT_COUNTER, 0);
        assert_eq!(VANILLA_PANDA_DEFAULT_MAIN_GENE, 0);
        assert_eq!(VANILLA_PANDA_DEFAULT_HIDDEN_GENE, 0);
        assert_eq!(VANILLA_PANDA_DEFAULT_FLAGS, 0);
        assert_eq!(VANILLA_PANDA_FLAG_SNEEZING, 2);
        assert_eq!(VANILLA_PANDA_FLAG_ROLLING, 4);
        assert_eq!(VANILLA_PANDA_FLAG_SITTING, 8);
        assert_eq!(VANILLA_PANDA_FLAG_ON_BACK, 16);
        let sheep = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Sheep);
        assert_eq!(sheep.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(sheep.inherited_field_count, 18);
        assert_eq!(sheep.own_fields.len(), 1);
        assert_eq!(sheep.own_flags.len(), 1);
        let (wool_data_id, wool_default, sheared_mask, _) =
            vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
            )
            .expect("generated Sheep wool accessor");
        assert_eq!(wool_data_id, sheep.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Byte(wool_default),
            sheep.own_fields[0].default
        );
        assert_eq!(sheared_mask, sheep.own_flags[0].mask);
        assert_eq!(VANILLA_HORSE_COLOR_MASK, 0xFF);
        assert_eq!(VANILLA_HORSE_MARKINGS_MASK, 0xFF00);
        assert_eq!(VANILLA_HORSE_MARKINGS_SHIFT, 8);
        let sniffer = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Sniffer);
        assert_eq!(
            sniffer.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(sniffer.inherited_field_count, 18);
        assert_eq!(sniffer.own_fields.len(), 2);
        assert_eq!(VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID, 19);
        assert_eq!(VANILLA_SNIFFER_DEFAULT_DROP_SEED_AT_TICK, 0);
        let (state_data_id, state_default) =
            vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SNIFFER_ID,
            )
            .expect("generated Sniffer state accessor");
        assert_eq!(state_data_id, sniffer.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(state_default),
            sniffer.own_fields[0].default
        );
        assert_eq!(VANILLA_SNIFFER_STATE_IDLING_ID, 0);
        assert_eq!(VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID, 1);
        assert_eq!(VANILLA_SNIFFER_STATE_SCENTING_ID, 2);
        assert_eq!(VANILLA_SNIFFER_STATE_SNIFFING_ID, 3);
        assert_eq!(VANILLA_SNIFFER_STATE_SEARCHING_ID, 4);
        assert_eq!(VANILLA_SNIFFER_STATE_DIGGING_ID, 5);
        assert_eq!(VANILLA_SNIFFER_STATE_RISING_ID, 6);
        let turtle = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Turtle);
        assert_eq!(turtle.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(turtle.inherited_field_count, 18);
        assert_eq!(turtle.own_fields.len(), 2);
        assert_eq!(VANILLA_TURTLE_HAS_EGG_DATA_ID, 18);
        assert_eq!(VANILLA_TURTLE_LAYING_EGG_DATA_ID, 19);
        assert!(!VANILLA_TURTLE_DEFAULT_HAS_EGG);
        assert!(!VANILLA_TURTLE_DEFAULT_LAYING_EGG);
        let camel = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Camel);
        assert_eq!(
            camel.parent,
            Some(VanillaEntityMetadataFamily::AbstractHorse)
        );
        assert_eq!(camel.inherited_field_count, 19);
        assert_eq!(camel.own_fields.len(), 2);
        let (dash_data_id, dash_default) =
            vanilla_entity_metadata_camel_dash_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CAMEL_ID,
            )
            .expect("generated Camel dash accessor");
        assert_eq!(dash_data_id, camel.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(dash_default),
            camel.own_fields[0].default
        );
        let (pose_tick_data_id, pose_tick_default) =
            vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CAMEL_ID,
            )
            .expect("generated Camel last-pose-change accessor");
        assert_eq!(pose_tick_data_id, camel.own_fields[1].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Long(pose_tick_default),
            camel.own_fields[1].default
        );
        let horse = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Horse);
        assert_eq!(
            horse.parent,
            Some(VanillaEntityMetadataFamily::AbstractHorse)
        );
        assert_eq!(horse.inherited_field_count, 19);
        assert_eq!(horse.own_fields.len(), 1);
        assert_eq!(VANILLA_HORSE_VARIANT_DATA_ID, 19);
        assert_eq!(VANILLA_HORSE_DEFAULT_VARIANT, 0);
        let chested =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractChestedHorse);
        assert_eq!(
            chested.parent,
            Some(VanillaEntityMetadataFamily::AbstractHorse)
        );
        assert_eq!(chested.inherited_field_count, 19);
        assert_eq!(chested.own_fields.len(), 1);
        assert_eq!(VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, 19);
        assert!(!VANILLA_ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST);
        let llama = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Llama);
        assert_eq!(
            llama.parent,
            Some(VanillaEntityMetadataFamily::AbstractChestedHorse)
        );
        assert_eq!(llama.inherited_field_count, 20);
        assert_eq!(llama.own_fields.len(), 2);
        assert_eq!(VANILLA_LLAMA_STRENGTH_DATA_ID, 20);
        assert_eq!(VANILLA_LLAMA_VARIANT_DATA_ID, 21);
        assert_eq!(VANILLA_LLAMA_DEFAULT_STRENGTH, 0);
        assert_eq!(VANILLA_LLAMA_DEFAULT_VARIANT, 0);
        let cat = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Cat);
        assert_eq!(cat.parent, Some(VanillaEntityMetadataFamily::TamableAnimal));
        assert_eq!(cat.inherited_field_count, 20);
        assert_eq!(cat.own_fields.len(), 5);
        assert_eq!(VANILLA_CAT_VARIANT_DATA_ID, 20);
        let accessors =
            vanilla_entity_metadata_cat_accessors_for_entity_type(VANILLA_ENTITY_TYPE_CAT_ID)
                .expect("generated Cat metadata accessors");
        assert_eq!(accessors.lying_data_id, cat.own_fields[1].data_id);
        assert_eq!(accessors.relax_state_one_data_id, cat.own_fields[2].data_id);
        assert_eq!(accessors.collar_color_data_id, cat.own_fields[3].data_id);
        assert_eq!(VANILLA_CAT_SOUND_VARIANT_DATA_ID, 24);
        assert_eq!(VANILLA_CAT_DEFAULT_VARIANT_RESOURCE_ID, "minecraft:black");
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(accessors.lying_default),
            cat.own_fields[1].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(accessors.relax_state_one_default),
            cat.own_fields[2].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Int(accessors.collar_color_default),
            cat.own_fields[3].default
        );
        assert_eq!(
            VANILLA_CAT_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
            "minecraft:classic"
        );
        assert_eq!(
            cat.own_fields
                .iter()
                .map(|field| field.default)
                .collect::<Vec<_>>(),
            vec![
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:black"),
                VanillaEntityMetadataDefault::Boolean(false),
                VanillaEntityMetadataDefault::Boolean(false),
                VanillaEntityMetadataDefault::Int(14),
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:classic"),
            ]
        );
        let wolf = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Wolf);
        assert_eq!(
            wolf.parent,
            Some(VanillaEntityMetadataFamily::TamableAnimal)
        );
        assert_eq!(wolf.inherited_field_count, 20);
        assert_eq!(wolf.own_fields.len(), 5);
        let accessors =
            vanilla_entity_metadata_wolf_accessors_for_entity_type(VANILLA_ENTITY_TYPE_WOLF_ID)
                .expect("generated Wolf metadata accessors");
        assert_eq!(accessors.interested_data_id, wolf.own_fields[0].data_id);
        assert_eq!(accessors.collar_color_data_id, wolf.own_fields[1].data_id);
        assert_eq!(accessors.anger_end_time_data_id, wolf.own_fields[2].data_id);
        assert_eq!(VANILLA_WOLF_VARIANT_DATA_ID, 23);
        assert_eq!(VANILLA_WOLF_SOUND_VARIANT_DATA_ID, 24);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(accessors.interested_default),
            wolf.own_fields[0].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Int(accessors.collar_color_default),
            wolf.own_fields[1].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Long(accessors.anger_end_time_default),
            wolf.own_fields[2].default
        );
        assert_eq!(VANILLA_WOLF_DEFAULT_VARIANT_RESOURCE_ID, "minecraft:pale");
        assert_eq!(
            VANILLA_WOLF_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
            "minecraft:classic"
        );
        assert_eq!(
            wolf.own_fields
                .iter()
                .map(|field| field.default)
                .collect::<Vec<_>>(),
            vec![
                VanillaEntityMetadataDefault::Boolean(false),
                VanillaEntityMetadataDefault::Int(14),
                VanillaEntityMetadataDefault::Long(-1),
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:pale"),
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:classic"),
            ]
        );
        let chicken = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Chicken);
        assert_eq!(
            chicken.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(chicken.inherited_field_count, 18);
        assert_eq!(chicken.own_fields.len(), 2);
        assert_eq!(VANILLA_CHICKEN_VARIANT_DATA_ID, 18);
        assert_eq!(VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID, 19);
        assert_eq!(
            VANILLA_CHICKEN_DEFAULT_VARIANT_RESOURCE_ID,
            "minecraft:temperate"
        );
        assert_eq!(
            VANILLA_CHICKEN_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
            "minecraft:classic"
        );
        assert_eq!(
            chicken
                .own_fields
                .iter()
                .map(|field| field.default)
                .collect::<Vec<_>>(),
            vec![
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:temperate"),
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:classic"),
            ]
        );
        let cow = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Cow);
        assert_eq!(cow.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(cow.inherited_field_count, 18);
        assert_eq!(cow.own_fields.len(), 2);
        assert_eq!(VANILLA_COW_VARIANT_DATA_ID, 18);
        assert_eq!(VANILLA_COW_SOUND_VARIANT_DATA_ID, 19);
        assert_eq!(
            VANILLA_COW_DEFAULT_VARIANT_RESOURCE_ID,
            "minecraft:temperate"
        );
        assert_eq!(
            VANILLA_COW_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
            "minecraft:classic"
        );
        assert_eq!(
            cow.own_fields
                .iter()
                .map(|field| field.default)
                .collect::<Vec<_>>(),
            vec![
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:temperate"),
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:classic"),
            ]
        );
        let mushroom_cow = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::MushroomCow);
        assert_eq!(
            mushroom_cow.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(mushroom_cow.inherited_field_count, 18);
        assert_eq!(mushroom_cow.own_fields.len(), 1);
        let (variant_data_id, variant_default) =
            vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            )
            .expect("generated MushroomCow variant accessor");
        assert_eq!(variant_data_id, mushroom_cow.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(variant_default),
            mushroom_cow.own_fields[0].default
        );
        let pig = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Pig);
        assert_eq!(pig.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(pig.inherited_field_count, 18);
        assert_eq!(pig.own_fields.len(), 3);
        assert_eq!(VANILLA_PIG_BOOST_TIME_DATA_ID, 18);
        assert_eq!(VANILLA_PIG_VARIANT_DATA_ID, 19);
        assert_eq!(VANILLA_PIG_SOUND_VARIANT_DATA_ID, 20);
        assert_eq!(VANILLA_PIG_DEFAULT_BOOST_TIME, 0);
        assert_eq!(
            VANILLA_PIG_DEFAULT_VARIANT_RESOURCE_ID,
            "minecraft:temperate"
        );
        assert_eq!(
            VANILLA_PIG_DEFAULT_SOUND_VARIANT_RESOURCE_ID,
            "minecraft:classic"
        );
        assert_eq!(
            pig.own_fields
                .iter()
                .map(|field| field.default)
                .collect::<Vec<_>>(),
            vec![
                VanillaEntityMetadataDefault::Int(0),
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:temperate"),
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:classic"),
            ]
        );
        let frog = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Frog);
        assert_eq!(frog.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(frog.inherited_field_count, 18);
        assert_eq!(frog.own_fields.len(), 2);
        assert_eq!(VANILLA_FROG_VARIANT_DATA_ID, 18);
        assert_eq!(VANILLA_FROG_TONGUE_TARGET_DATA_ID, 19);
        assert_eq!(
            VANILLA_FROG_DEFAULT_VARIANT_RESOURCE_ID,
            "minecraft:temperate"
        );
        assert_eq!(
            frog.own_fields
                .iter()
                .map(|field| field.default)
                .collect::<Vec<_>>(),
            vec![
                VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:temperate"),
                VanillaEntityMetadataDefault::OptionalUnsignedIntNone,
            ]
        );
        let abstract_nautilus =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractNautilus);
        assert_eq!(
            abstract_nautilus.parent,
            Some(VanillaEntityMetadataFamily::TamableAnimal)
        );
        assert_eq!(abstract_nautilus.inherited_field_count, 20);
        assert_eq!(abstract_nautilus.own_fields.len(), 1);
        assert_eq!(VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID, 20);
        assert!(!VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH);
        assert_eq!(
            abstract_nautilus.own_fields[0].default,
            VanillaEntityMetadataDefault::Boolean(false)
        );
        let zombie_nautilus =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ZombieNautilus);
        assert_eq!(
            zombie_nautilus.parent,
            Some(VanillaEntityMetadataFamily::AbstractNautilus)
        );
        assert_eq!(zombie_nautilus.inherited_field_count, 21);
        assert_eq!(zombie_nautilus.own_fields.len(), 1);
        assert_eq!(VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID, 21);
        assert_eq!(
            VANILLA_ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID,
            "minecraft:temperate"
        );
        assert_eq!(
            zombie_nautilus.own_fields[0].default,
            VanillaEntityMetadataDefault::RegistryHolderResource("minecraft:temperate")
        );
        let parrot = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Parrot);
        assert_eq!(
            parrot.parent,
            Some(VanillaEntityMetadataFamily::TamableAnimal)
        );
        assert_eq!(parrot.inherited_field_count, 20);
        assert_eq!(parrot.own_fields.len(), 1);
        let (variant_data_id, variant_default) =
            vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PARROT_ID,
            )
            .expect("generated Parrot variant accessor");
        assert_eq!(variant_data_id, parrot.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(variant_default),
            parrot.own_fields[0].default
        );
        let rabbit = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Rabbit);
        assert_eq!(rabbit.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(rabbit.inherited_field_count, 18);
        assert_eq!(rabbit.own_fields.len(), 1);
        let (variant_data_id, variant_default) =
            vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_RABBIT_ID,
            )
            .expect("generated Rabbit variant accessor");
        assert_eq!(variant_data_id, rabbit.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(variant_default),
            rabbit.own_fields[0].default
        );
        assert_eq!(VANILLA_RABBIT_VARIANT_EVIL_ID, 99);
        let abstract_villager =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractVillager);
        assert_eq!(
            abstract_villager.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(abstract_villager.inherited_field_count, 18);
        assert_eq!(abstract_villager.own_fields.len(), 1);
        assert_eq!(VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID, 18);
        assert_eq!(VANILLA_ABSTRACT_VILLAGER_DEFAULT_UNHAPPY_COUNTER, 0);
        assert_eq!(VANILLA_VILLAGER_DATA_DATA_ID, 19);
        assert_eq!(VANILLA_VILLAGER_DATA_FINALIZED_DATA_ID, 20);
        assert!(!VANILLA_VILLAGER_DEFAULT_DATA_FINALIZED);
        assert_eq!(VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID, 19);
        assert_eq!(VANILLA_ZOMBIE_VILLAGER_DATA_DATA_ID, 20);
        assert_eq!(VANILLA_ZOMBIE_VILLAGER_DATA_FINALIZED_DATA_ID, 21);
        assert!(!VANILLA_ZOMBIE_VILLAGER_DEFAULT_CONVERTING);
        assert!(!VANILLA_ZOMBIE_VILLAGER_DEFAULT_DATA_FINALIZED);
        assert_eq!(
            vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            ),
            Some((19, false))
        );
        assert_eq!(
            vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(-1),
            None
        );
        let zombie_villager_data = VANILLA_ENTITY_METADATA_ACCESSORS_26_1
            .iter()
            .find(|entry| {
                entry.owner == "net.minecraft.world.entity.monster.zombie.ZombieVillager"
                    && entry.java_name == "DATA_VILLAGER_DATA"
            })
            .expect("generated zombie villager data accessor");
        assert_eq!(zombie_villager_data.data_id, 20);
        assert_eq!(zombie_villager_data.typed_default, None);
        assert_eq!(
            zombie_villager_data.default_expression,
            "this.initializeVillagerData ()"
        );
        assert_eq!(
            vanilla_entity_metadata_villager_data_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_VILLAGER_ID
            ),
            Some(19)
        );
        assert_eq!(
            vanilla_entity_metadata_villager_data_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            ),
            Some(20)
        );
        assert_eq!(
            vanilla_entity_metadata_villager_data_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_villager_data_accessor_for_entity_type(-1),
            None
        );
        let furnace_minecart =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::MinecartFurnace);
        assert_eq!(
            furnace_minecart.parent,
            Some(VanillaEntityMetadataFamily::AbstractMinecart)
        );
        assert_eq!(furnace_minecart.inherited_field_count, 13);
        assert_eq!(furnace_minecart.own_fields.len(), 1);
        assert_eq!(VANILLA_FURNACE_MINECART_FUEL_DATA_ID, 13);
        assert!(!VANILLA_FURNACE_MINECART_DEFAULT_FUEL);
        assert_eq!(VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID, 13);
        assert_eq!(VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID, 14);
        let command_block_minecart_defaults = VANILLA_ENTITY_METADATA_ACCESSORS_26_1
            .iter()
            .filter(|entry| {
                entry.owner == "net.minecraft.world.entity.vehicle.minecart.MinecartCommandBlock"
            })
            .map(|entry| {
                (
                    entry.java_name,
                    entry.default_expression,
                    entry.typed_default,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            command_block_minecart_defaults,
            vec![
                ("DATA_ID_COMMAND_NAME", "\"\"", None),
                ("DATA_ID_LAST_OUTPUT", "CommonComponents.EMPTY", None),
            ]
        );
        let salmon = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Salmon);
        assert_eq!(
            salmon.parent,
            Some(VanillaEntityMetadataFamily::AbstractFish)
        );
        assert_eq!(salmon.inherited_field_count, 17);
        assert_eq!(salmon.own_fields.len(), 1);
        let (variant_data_id, variant_default) =
            vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SALMON_ID,
            )
            .expect("generated Salmon variant accessor");
        assert_eq!(variant_data_id, salmon.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(variant_default),
            salmon.own_fields[0].default
        );
        let tropical_fish =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::TropicalFish);
        assert_eq!(
            tropical_fish.parent,
            Some(VanillaEntityMetadataFamily::AbstractFish)
        );
        assert_eq!(tropical_fish.inherited_field_count, 17);
        assert_eq!(tropical_fish.own_fields.len(), 1);
        let accessors = vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        )
        .expect("generated TropicalFish packed-variant accessor");
        assert_eq!(
            accessors.variant_data_id,
            tropical_fish.own_fields[0].data_id
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Int(accessors.variant_default),
            tropical_fish.own_fields[0].default
        );
        assert_eq!(
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1.len(),
            28
        );
        let default_evidence = |java_class| {
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1
                .iter()
                .find(|item| item.java_class == java_class)
                .copied()
                .expect("generated enum default resolver evidence")
        };
        let parrot_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.parrot.Parrot");
        assert_eq!(
            parrot_default_evidence.java_class,
            "net.minecraft.world.entity.animal.parrot.Parrot"
        );
        assert_eq!(parrot_default_evidence.accessor_name, "DATA_VARIANT_ID");
        assert_eq!(
            parrot_default_evidence.source_expression,
            "Parrot.Variant.DEFAULT.id"
        );
        assert!(!parrot_default_evidence.evidence_member.is_empty());
        assert!(!parrot_default_evidence.reason.is_empty());
        let rabbit_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.rabbit.Rabbit");
        assert_eq!(rabbit_default_evidence.accessor_name, "DATA_TYPE_ID");
        assert_eq!(
            rabbit_default_evidence.source_expression,
            "Rabbit.Variant.DEFAULT.id"
        );
        let salmon_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.fish.Salmon");
        assert_eq!(salmon_default_evidence.accessor_name, "DATA_TYPE");
        assert_eq!(
            salmon_default_evidence.source_expression,
            "Salmon.Variant.DEFAULT.id ()"
        );
        let tropical_fish_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.fish.TropicalFish");
        assert_eq!(
            tropical_fish_default_evidence.accessor_name,
            "DATA_ID_TYPE_VARIANT"
        );
        assert_eq!(
            tropical_fish_default_evidence.evidence_member,
            "net.minecraft.world.entity.animal.fish.TropicalFish.DEFAULT_VARIANT"
        );
        assert_eq!(
            tropical_fish_default_evidence.source_expression,
            "DEFAULT_VARIANT.getPackedId () -> new TropicalFish.Variant (TropicalFish.Pattern.KOB, DyeColor.WHITE, DyeColor.WHITE); this.packedId = base.id | index << 8; pattern.getPackedId () & 65535 | (baseColor.getId () & 0xFF) << 16 | (patternColor.getId () & 0xFF) << 24"
        );
        let cat_default_evidence = |accessor_name| {
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1
                .iter()
                .find(|item| {
                    item.java_class == "net.minecraft.world.entity.animal.feline.Cat"
                        && item.accessor_name == accessor_name
                })
                .copied()
                .expect("generated cat default resolver evidence")
        };
        assert_eq!(
            cat_default_evidence("DATA_VARIANT_ID").evidence_member,
            "net.minecraft.world.entity.animal.feline.Cat.DEFAULT_VARIANT -> CatVariants.BLACK -> CatVariants.createKey"
        );
        assert_eq!(
            cat_default_evidence("DATA_SOUND_VARIANT_ID").source_expression,
            "catSoundVariants.get (CatSoundVariants.CLASSIC).or (catSoundVariants::getAny).orElseThrow () -> createKey (CatSoundVariants.SoundSet.CLASSIC) -> ResourceKey.create (Registries.CAT_SOUND_VARIANT, Identifier.withDefaultNamespace (catSoundVariant.getIdentifier ())) -> this.identifier -> \"classic\""
        );
        assert_eq!(
            cat_default_evidence("DATA_COLLAR_COLOR").source_expression,
            "DEFAULT_COLLAR_COLOR.getId () -> DyeColor.RED"
        );
        let wolf_default_evidence = |accessor_name| {
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1
                .iter()
                .find(|item| {
                    item.java_class == "net.minecraft.world.entity.animal.wolf.Wolf"
                        && item.accessor_name == accessor_name
                })
                .copied()
                .expect("generated wolf default resolver evidence")
        };
        assert_eq!(
            wolf_default_evidence("DATA_VARIANT_ID").evidence_member,
            "net.minecraft.world.entity.animal.wolf.WolfVariants.DEFAULT -> WolfVariants.PALE -> WolfVariants.createKey"
        );
        assert_eq!(
            wolf_default_evidence("DATA_SOUND_VARIANT_ID").source_expression,
            "wolfSoundVariants.get (WolfSoundVariants.CLASSIC).or (wolfSoundVariants::getAny).orElseThrow () -> createKey (WolfSoundVariants.SoundSet.CLASSIC) -> ResourceKey.create (Registries.WOLF_SOUND_VARIANT, Identifier.withDefaultNamespace (wolfSoundVariant.getIdentifier ())) -> this.identifier -> \"classic\""
        );
        assert_eq!(
            wolf_default_evidence("DATA_COLLAR_COLOR").source_expression,
            "DEFAULT_COLLAR_COLOR.getId () -> DyeColor.RED"
        );
        let chicken_default_evidence = |accessor_name| {
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1
                .iter()
                .find(|item| {
                    item.java_class == "net.minecraft.world.entity.animal.chicken.Chicken"
                        && item.accessor_name == accessor_name
                })
                .copied()
                .expect("generated chicken default resolver evidence")
        };
        assert_eq!(
            chicken_default_evidence("DATA_VARIANT_ID").source_expression,
            "VariantUtils.getDefaultOrAny (this.registryAccess (), ChickenVariants.TEMPERATE) -> createKey (TemperatureVariants.TEMPERATE) -> Identifier.withDefaultNamespace (\"temperate\") -> ResourceKey.create (Registries.CHICKEN_VARIANT, id)"
        );
        assert_eq!(
            chicken_default_evidence("DATA_SOUND_VARIANT_ID").evidence_member,
            "net.minecraft.world.entity.animal.chicken.ChickenSoundVariants.CLASSIC -> ChickenSoundVariants.createKey -> SoundSet.getIdentifier -> SoundSet.CLASSIC"
        );
        let cow_default_evidence = |accessor_name| {
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1
                .iter()
                .find(|item| {
                    item.java_class == "net.minecraft.world.entity.animal.cow.Cow"
                        && item.accessor_name == accessor_name
                })
                .copied()
                .expect("generated cow default resolver evidence")
        };
        assert_eq!(
            cow_default_evidence("DATA_VARIANT_ID").source_expression,
            "VariantUtils.getDefaultOrAny (this.registryAccess (), CowVariants.TEMPERATE) -> createKey (TemperatureVariants.TEMPERATE) -> Identifier.withDefaultNamespace (\"temperate\") -> ResourceKey.create (Registries.COW_VARIANT, id)"
        );
        assert_eq!(
            cow_default_evidence("DATA_SOUND_VARIANT_ID").evidence_member,
            "net.minecraft.world.entity.animal.cow.CowSoundVariants.CLASSIC -> CowSoundVariants.createKey -> SoundSet.getIdentifier -> SoundSet.CLASSIC"
        );
        let mushroom_cow_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.cow.MushroomCow");
        assert_eq!(mushroom_cow_default_evidence.accessor_name, "DATA_TYPE");
        assert_eq!(
            mushroom_cow_default_evidence.evidence_member,
            "net.minecraft.world.entity.animal.cow.MushroomCow.Variant.DEFAULT -> RED"
        );
        let pig_default_evidence = |accessor_name| {
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1
                .iter()
                .find(|item| {
                    item.java_class == "net.minecraft.world.entity.animal.pig.Pig"
                        && item.accessor_name == accessor_name
                })
                .copied()
                .expect("generated pig default resolver evidence")
        };
        assert_eq!(
            pig_default_evidence("DATA_VARIANT_ID").source_expression,
            "VariantUtils.getDefaultOrAny (this.registryAccess (), PigVariants.DEFAULT) -> TEMPERATE -> createKey (TemperatureVariants.TEMPERATE) -> Identifier.withDefaultNamespace (\"temperate\") -> ResourceKey.create (Registries.PIG_VARIANT, id)"
        );
        assert_eq!(
            pig_default_evidence("DATA_SOUND_VARIANT_ID").evidence_member,
            "net.minecraft.world.entity.animal.pig.PigSoundVariants.CLASSIC -> PigSoundVariants.createKey -> SoundSet.getIdentifier -> SoundSet.CLASSIC"
        );
        let frog_default_evidence = default_evidence("net.minecraft.world.entity.animal.frog.Frog");
        assert_eq!(frog_default_evidence.accessor_name, "DATA_VARIANT_ID");
        assert_eq!(
            frog_default_evidence.source_expression,
            "VariantUtils.getDefaultOrAny (this.registryAccess (), DEFAULT_VARIANT) -> FrogVariants.TEMPERATE -> createKey (TemperatureVariants.TEMPERATE) -> Identifier.withDefaultNamespace (\"temperate\") -> ResourceKey.create (Registries.FROG_VARIANT, id)"
        );
        let zombie_nautilus_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.nautilus.ZombieNautilus");
        assert_eq!(
            zombie_nautilus_default_evidence.accessor_name,
            "DATA_VARIANT_ID"
        );
        assert_eq!(
            zombie_nautilus_default_evidence.source_expression,
            "VariantUtils.getDefaultOrAny (this.registryAccess (), ZombieNautilusVariants.TEMPERATE) -> createKey (TemperatureVariants.TEMPERATE) -> Identifier.withDefaultNamespace (\"temperate\") -> ResourceKey.create (Registries.ZOMBIE_NAUTILUS_VARIANT, id)"
        );
        let ender_dragon_default_evidence =
            default_evidence("net.minecraft.world.entity.boss.enderdragon.EnderDragon");
        assert_eq!(ender_dragon_default_evidence.accessor_name, "DATA_PHASE");
        assert_eq!(
            ender_dragon_default_evidence.evidence_member,
            "net.minecraft.world.entity.boss.enderdragon.phases.EnderDragonPhase.HOVERING -> create registration 10"
        );
        assert_eq!(
            ender_dragon_default_evidence.source_expression,
            "EnderDragonPhase.HOVERING.getId () -> new EnderDragonPhase <> (phases.length, instanceClass, name)"
        );
        let painting_default_evidence =
            default_evidence("net.minecraft.world.entity.decoration.painting.Painting");
        assert_eq!(
            painting_default_evidence.accessor_name,
            "DATA_PAINTING_VARIANT_ID"
        );
        assert_eq!(
            painting_default_evidence.evidence_member,
            "net.minecraft.core.registries.Registries.PAINTING_VARIANT -> createRegistryKey"
        );
        assert_eq!(
            painting_default_evidence.source_expression,
            "VariantUtils.getAny (this.registryAccess (), Registries.PAINTING_VARIANT) -> createRegistryKey (\"painting_variant\") -> ResourceKey.createRegistryKey (Identifier.withDefaultNamespace (name))"
        );
        let fox_default_evidence = default_evidence("net.minecraft.world.entity.animal.fox.Fox");
        assert_eq!(fox_default_evidence.accessor_name, "DATA_TYPE_ID");
        assert_eq!(
            fox_default_evidence.source_expression,
            "Fox.Variant.DEFAULT.getId ()"
        );
        let sniffer_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.sniffer.Sniffer");
        assert_eq!(sniffer_default_evidence.accessor_name, "DATA_STATE");
        assert_eq!(
            sniffer_default_evidence.source_expression,
            "Sniffer.State.IDLING"
        );
        let armadillo_default_evidence =
            default_evidence("net.minecraft.world.entity.animal.armadillo.Armadillo");
        assert_eq!(armadillo_default_evidence.accessor_name, "ARMADILLO_STATE");
        assert_eq!(
            armadillo_default_evidence.source_expression,
            "Armadillo.ArmadilloState.IDLE"
        );
        let copper_default_evidence = |accessor_name| {
            VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1
                .iter()
                .find(|item| {
                    item.java_class == "net.minecraft.world.entity.animal.golem.CopperGolem"
                        && item.accessor_name == accessor_name
                })
                .copied()
                .expect("generated copper golem default resolver evidence")
        };
        assert_eq!(
            copper_default_evidence("DATA_WEATHER_STATE").source_expression,
            "WeatheringCopper.WeatherState.UNAFFECTED"
        );
        assert_eq!(
            copper_default_evidence("COPPER_GOLEM_STATE").source_expression,
            "CopperGolemState.IDLE"
        );
        let firework_default_evidence =
            default_evidence("net.minecraft.world.entity.projectile.FireworkRocketEntity");
        assert_eq!(
            firework_default_evidence.accessor_name,
            "DATA_ID_FIREWORKS_ITEM"
        );
        assert_eq!(
            firework_default_evidence.evidence_member,
            "net.minecraft.world.entity.projectile.FireworkRocketEntity.getDefaultItem"
        );
        assert_eq!(
            firework_default_evidence.source_expression,
            "getDefaultItem () -> new ItemStack (Items.FIREWORK_ROCKET)"
        );
        let primed_tnt_default_evidence =
            default_evidence("net.minecraft.world.entity.item.PrimedTnt");
        assert_eq!(
            primed_tnt_default_evidence.accessor_name,
            "DATA_BLOCK_STATE_ID"
        );
        assert_eq!(
            primed_tnt_default_evidence.evidence_member,
            "net.minecraft.world.level.block.TntBlock.registerDefaultState"
        );
        assert_eq!(
            primed_tnt_default_evidence.source_expression,
            "DEFAULT_BLOCK_STATE -> Blocks.TNT.defaultBlockState (); this.registerDefaultState (this.defaultBlockState ().setValue (UNSTABLE, false))"
        );
        for evidence in VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1 {
            assert!(!evidence.evidence_member.is_empty());
            assert!(!evidence.reason.is_empty());
        }
        let polar_bear = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::PolarBear);
        assert_eq!(
            polar_bear.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(polar_bear.inherited_field_count, 18);
        assert_eq!(polar_bear.own_fields.len(), 1);
        let (standing_data_id, standing_default) =
            vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
            )
            .expect("generated PolarBear standing accessor");
        assert_eq!(standing_data_id, polar_bear.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(standing_default),
            polar_bear.own_fields[0].default
        );
        assert_eq!(
            arrow.parent,
            Some(VanillaEntityMetadataFamily::AbstractArrow)
        );
        assert_eq!(arrow.inherited_field_count, 11);
        assert_eq!(
            arrow.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "ID_EFFECT_COLOR",
                data_id: 11,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                default: VanillaEntityMetadataDefault::Int(-1),
            }]
        );
        let trident = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ThrownTrident);
        assert_eq!(
            trident.parent,
            Some(VanillaEntityMetadataFamily::AbstractArrow)
        );
        assert_eq!(trident.inherited_field_count, 11);
        assert_eq!(trident.own_fields.len(), 2);
        assert_eq!(VANILLA_TRIDENT_LOYALTY_DATA_ID, 11);
        assert_eq!(VANILLA_TRIDENT_FOIL_DATA_ID, 12);
        assert_eq!(VANILLA_TRIDENT_DEFAULT_LOYALTY, 0);
        assert!(!VANILLA_TRIDENT_DEFAULT_FOIL);
        let wither_skull = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::WitherSkull);
        assert_eq!(
            wither_skull.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(wither_skull.inherited_field_count, 8);
        assert_eq!(
            wither_skull.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_DANGEROUS",
                data_id: 8,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(false),
            }]
        );
        assert_eq!(VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID, 8);
        assert!(!VANILLA_WITHER_SKULL_DEFAULT_DANGEROUS);
        let end_crystal = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::EndCrystal);
        assert_eq!(
            end_crystal.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(end_crystal.inherited_field_count, 8);
        assert_eq!(end_crystal.own_fields.len(), 2);
        assert_eq!(
            end_crystal.own_fields[0],
            VanillaEntityMetadataFieldSchema {
                name: "DATA_BEAM_TARGET",
                data_id: 8,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                default: VanillaEntityMetadataDefault::OptionalBlockPosNone,
            }
        );
        assert_eq!(
            end_crystal.own_fields[1],
            VanillaEntityMetadataFieldSchema {
                name: "DATA_SHOW_BOTTOM",
                data_id: 9,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(true),
            }
        );
        assert_eq!(VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID, 8);
        assert_eq!(VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID, 9);
        assert!(VANILLA_END_CRYSTAL_DEFAULT_SHOW_BOTTOM);
        let ender_dragon = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::EnderDragon);
        assert_eq!(ender_dragon.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(ender_dragon.inherited_field_count, 16);
        assert_eq!(ender_dragon.own_fields.len(), 1);
        let (phase_data_id, phase_default) =
            vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            )
            .expect("generated EnderDragon phase accessor");
        assert_eq!(phase_data_id, ender_dragon.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(phase_default),
            ender_dragon.own_fields[0].default
        );
        assert_eq!(VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID, 0);
        let painting = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Painting);
        assert_eq!(
            painting.parent,
            Some(VanillaEntityMetadataFamily::HangingEntity)
        );
        assert_eq!(painting.inherited_field_count, 9);
        assert_eq!(painting.own_fields.len(), 1);
        assert_eq!(VANILLA_PAINTING_VARIANT_DATA_ID, 9);
        assert_eq!(
            painting.own_fields[0].default,
            VanillaEntityMetadataDefault::RegistryAny("minecraft:painting_variant")
        );
        let interaction = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Interaction);
        assert_eq!(
            interaction.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(interaction.inherited_field_count, 8);
        assert_eq!(interaction.own_fields.len(), 3);
        assert_eq!(VANILLA_INTERACTION_WIDTH_DATA_ID, 8);
        assert_eq!(VANILLA_INTERACTION_HEIGHT_DATA_ID, 9);
        assert_eq!(VANILLA_INTERACTION_RESPONSE_DATA_ID, 10);
        assert_eq!(VANILLA_INTERACTION_DEFAULT_WIDTH, 1.0);
        assert_eq!(VANILLA_INTERACTION_DEFAULT_HEIGHT, 1.0);
        assert!(!VANILLA_INTERACTION_DEFAULT_RESPONSE);
        assert_eq!(
            interaction
                .own_fields
                .iter()
                .map(|field| (field.name, field.data_id, field.serializer_id, field.default))
                .collect::<Vec<_>>(),
            vec![
                (
                    "DATA_WIDTH_ID",
                    8,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                    VanillaEntityMetadataDefault::Float(1.0),
                ),
                (
                    "DATA_HEIGHT_ID",
                    9,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                    VanillaEntityMetadataDefault::Float(1.0),
                ),
                (
                    "DATA_RESPONSE_ID",
                    10,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    VanillaEntityMetadataDefault::Boolean(false),
                ),
            ]
        );
        let experience_orb =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ExperienceOrb);
        assert_eq!(
            experience_orb.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(experience_orb.inherited_field_count, 8);
        assert_eq!(
            experience_orb.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_VALUE",
                data_id: 8,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                default: VanillaEntityMetadataDefault::Int(0),
            }]
        );
        assert_eq!(VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID, 8);
        assert_eq!(VANILLA_EXPERIENCE_ORB_DEFAULT_VALUE, 0);
        let item_entity = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ItemEntity);
        assert_eq!(
            item_entity.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(item_entity.inherited_field_count, 8);
        assert_eq!(
            item_entity.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_ITEM",
                data_id: 8,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                default: VanillaEntityMetadataDefault::ItemStackEmpty,
            }]
        );
        assert_eq!(
            vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ITEM_ID
            ),
            Some(item_entity.own_fields[0].data_id)
        );
        let primed_tnt = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::PrimedTnt);
        assert_eq!(primed_tnt.parent, Some(VanillaEntityMetadataFamily::Entity));
        assert_eq!(primed_tnt.inherited_field_count, 8);
        assert_eq!(primed_tnt.own_fields.len(), 2);
        assert_eq!(VANILLA_PRIMED_TNT_FUSE_DATA_ID, 8);
        assert_eq!(VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID, 9);
        assert_eq!(VANILLA_PRIMED_TNT_DEFAULT_FUSE, 80);
        assert_eq!(
            VANILLA_PRIMED_TNT_DEFAULT_BLOCK_STATE,
            VanillaEntityMetadataDefault::BlockStateDefault {
                resource_id: "minecraft:tnt",
                properties: &[("unstable", "false")],
            }
        );
        let firework = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::FireworkRocket);
        assert_eq!(firework.parent, Some(VanillaEntityMetadataFamily::Entity));
        assert_eq!(firework.inherited_field_count, 8);
        assert_eq!(firework.own_fields.len(), 3);
        assert_eq!(VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID, 8);
        assert_eq!(VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID, 9);
        assert_eq!(VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID, 10);
        assert_eq!(
            firework.own_fields[0].default,
            VanillaEntityMetadataDefault::ItemStackDefaultItem("minecraft:firework_rocket")
        );
        assert_eq!(
            firework.own_fields[1].default,
            VanillaEntityMetadataDefault::OptionalUnsignedIntNone
        );
        assert_eq!(
            VANILLA_FIREWORK_ROCKET_DEFAULT_ITEM_RESOURCE_ID,
            "minecraft:firework_rocket"
        );
        assert!(!VANILLA_FIREWORK_ROCKET_DEFAULT_SHOT_AT_ANGLE);
        let fishing_hook = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::FishingHook);
        assert_eq!(
            fishing_hook.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(fishing_hook.inherited_field_count, 8);
        assert_eq!(VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID, 8);
        assert_eq!(VANILLA_FISHING_HOOK_BITING_DATA_ID, 9);
        assert_eq!(VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY, 0);
        assert!(!VANILLA_FISHING_HOOK_DEFAULT_BITING);
        assert_eq!(
            fishing_hook.own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_HOOKED_ENTITY",
                    data_id: 8,
                    serializer_id:
                        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_BITING",
                    data_id: 9,
                    serializer_id:
                        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
            ]
        );
        let ominous_item_spawner =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::OminousItemSpawner);
        assert_eq!(
            ominous_item_spawner.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(ominous_item_spawner.inherited_field_count, 8);
        assert_eq!(VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID, 8);
        assert_eq!(
            ominous_item_spawner.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_ITEM",
                data_id: 8,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                default: VanillaEntityMetadataDefault::ItemStackEmpty,
            }]
        );
        let guardian = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Guardian);
        assert_eq!(guardian.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(guardian.inherited_field_count, 16);
        assert_eq!(guardian.own_fields.len(), 2);
        assert_eq!(
            guardian
                .own_fields
                .iter()
                .map(|field| (field.name, field.serializer_id, field.default))
                .collect::<Vec<_>>(),
            vec![
                (
                    "DATA_ID_MOVING",
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    VanillaEntityMetadataDefault::Boolean(false),
                ),
                (
                    "DATA_ID_ATTACK_TARGET",
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    VanillaEntityMetadataDefault::Int(0),
                ),
            ]
        );
        let snow_golem = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::SnowGolem);
        assert_eq!(snow_golem.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(snow_golem.inherited_field_count, 16);
        let (pumpkin_data_id, pumpkin_default, pumpkin_mask) =
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
            )
            .expect("generated SnowGolem pumpkin accessor");
        assert_eq!(
            snow_golem.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_PUMPKIN_ID",
                data_id: pumpkin_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                default: VanillaEntityMetadataDefault::Byte(pumpkin_default),
            }]
        );
        let iron_golem = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::IronGolem);
        assert_eq!(iron_golem.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(iron_golem.inherited_field_count, 16);
        assert_eq!(iron_golem.own_fields.len(), 1);
        let (flags_data_id, flags_default, player_created_mask) =
            vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            )
            .expect("generated IronGolem flags accessor");
        assert_eq!(flags_data_id, iron_golem.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Byte(flags_default),
            iron_golem.own_fields[0].default
        );
        assert_eq!(player_created_mask, VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED);
        assert_eq!(
            snow_golem.own_flags,
            &[VanillaEntityMetadataFlagSchema {
                name: "PUMPKIN_FLAG",
                data_id: pumpkin_data_id,
                mask: pumpkin_mask,
            }]
        );
        let bat = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Bat);
        assert_eq!(bat.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(bat.inherited_field_count, 16);
        assert_eq!(
            bat.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_ID_FLAGS",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                default: VanillaEntityMetadataDefault::Byte(0),
            }]
        );
        assert_eq!(VANILLA_BAT_FLAGS_DATA_ID, 16);
        assert_eq!(VANILLA_BAT_DEFAULT_FLAGS, 0);
        assert_eq!(VANILLA_BAT_FLAG_RESTING, 1);
        assert_eq!(
            bat.own_flags,
            &[VanillaEntityMetadataFlagSchema {
                name: "FLAG_RESTING",
                data_id: 16,
                mask: 1,
            }]
        );
        let phantom = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Phantom);
        assert_eq!(phantom.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(phantom.inherited_field_count, 16);
        assert_eq!(phantom.own_fields.len(), 1);
        let (size_data_id, size_default) =
            vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PHANTOM_ID,
            )
            .expect("generated Phantom size accessor");
        assert_eq!(size_data_id, phantom.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(size_default),
            phantom.own_fields[0].default
        );
        let bogged = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Bogged);
        assert_eq!(bogged.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(bogged.inherited_field_count, 16);
        assert_eq!(bogged.own_fields.len(), 1);
        let (sheared_data_id, sheared_default) =
            vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_BOGGED_ID,
            )
            .expect("generated Bogged sheared accessor");
        assert_eq!(sheared_data_id, bogged.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(sheared_default),
            bogged.own_fields[0].default
        );
        let skeleton = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Skeleton);
        assert_eq!(skeleton.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(skeleton.inherited_field_count, 16);
        assert_eq!(VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID, 16);
        assert!(!VANILLA_SKELETON_DEFAULT_STRAY_CONVERSION);
        assert_eq!(
            vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SKELETON_ID
            ),
            Some((16, false))
        );
        assert_eq!(
            vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_STRAY_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type(-1),
            None
        );
        assert_eq!(
            skeleton.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_STRAY_CONVERSION_ID",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(false),
            }]
        );
        let ghast = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Ghast);
        assert_eq!(ghast.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(ghast.inherited_field_count, 16);
        assert_eq!(
            ghast.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_IS_CHARGING",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(false),
            }]
        );
        assert_eq!(VANILLA_GHAST_CHARGING_DATA_ID, 16);
        assert!(!VANILLA_GHAST_DEFAULT_CHARGING);
        let vex = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Vex);
        assert_eq!(vex.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(vex.inherited_field_count, 16);
        assert_eq!(vex.own_fields.len(), 1);
        assert_eq!(vex.own_fields[0].name, "DATA_FLAGS_ID");
        assert_eq!(
            vex.own_fields[0].default,
            VanillaEntityMetadataDefault::Byte(0)
        );
        assert_eq!(VANILLA_VEX_FLAGS_DATA_ID, 16);
        assert_eq!(VANILLA_VEX_DEFAULT_FLAGS, 0);
        assert_eq!(VANILLA_VEX_FLAG_CHARGING, 1);
        let creaking = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Creaking);
        assert_eq!(creaking.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(creaking.inherited_field_count, 16);
        assert_eq!(
            creaking
                .own_fields
                .iter()
                .map(|field| (field.name, field.serializer_id, field.default))
                .collect::<Vec<_>>(),
            vec![
                (
                    "CAN_MOVE",
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    VanillaEntityMetadataDefault::Boolean(true),
                ),
                (
                    "IS_ACTIVE",
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    VanillaEntityMetadataDefault::Boolean(false),
                ),
                (
                    "IS_TEARING_DOWN",
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    VanillaEntityMetadataDefault::Boolean(false),
                ),
                (
                    "HOME_POS",
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                    VanillaEntityMetadataDefault::OptionalBlockPosNone,
                ),
            ]
        );
        let (can_move_data_id, can_move_default) =
            vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID,
            )
            .expect("generated Creaking can-move accessor");
        let (active_data_id, active_default) =
            vanilla_entity_metadata_creaking_active_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID,
            )
            .expect("generated Creaking active accessor");
        let (tearing_down_data_id, tearing_down_default) =
            vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID,
            )
            .expect("generated Creaking tearing-down accessor");
        let home_pos_data_id = vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking home-position accessor");
        for (data_id, default, field) in [
            (can_move_data_id, can_move_default, &creaking.own_fields[0]),
            (active_data_id, active_default, &creaking.own_fields[1]),
            (
                tearing_down_data_id,
                tearing_down_default,
                &creaking.own_fields[2],
            ),
        ] {
            assert_eq!(data_id, field.data_id);
            assert_eq!(
                VanillaEntityMetadataDefault::Boolean(default),
                field.default
            );
        }
        assert_eq!(home_pos_data_id, creaking.own_fields[3].data_id);
        assert_eq!(
            creaking.own_fields[3].default,
            VanillaEntityMetadataDefault::OptionalBlockPosNone
        );
        let strider = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Strider);
        assert_eq!(
            strider.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(strider.inherited_field_count, 18);
        assert_eq!(strider.own_fields.len(), 2);
        assert_eq!(strider.own_fields[0].name, "DATA_BOOST_TIME");
        assert_eq!(
            strider.own_fields[0].default,
            VanillaEntityMetadataDefault::Int(0)
        );
        assert_eq!(strider.own_fields[1].name, "DATA_SUFFOCATING");
        assert_eq!(
            strider.own_fields[1].default,
            VanillaEntityMetadataDefault::Boolean(false)
        );
        assert_eq!(VANILLA_STRIDER_BOOST_TIME_DATA_ID, 18);
        assert_eq!(VANILLA_STRIDER_SUFFOCATING_DATA_ID, 19);
        assert_eq!(VANILLA_STRIDER_DEFAULT_BOOST_TIME, 0);
        assert!(!VANILLA_STRIDER_DEFAULT_SUFFOCATING);
        assert_eq!(
            vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_STRIDER_ID
            ),
            Some((19, false))
        );
        assert_eq!(
            vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIG_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(-1),
            None
        );
        let zoglin = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Zoglin);
        assert_eq!(zoglin.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(zoglin.inherited_field_count, 16);
        assert_eq!(
            zoglin.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_BABY_ID",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(false),
            }]
        );
        assert_eq!(VANILLA_ZOGLIN_BABY_DATA_ID, 16);
        assert!(!VANILLA_ZOGLIN_DEFAULT_BABY);
        let hoglin = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Hoglin);
        assert_eq!(hoglin.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(hoglin.inherited_field_count, 18);
        assert_eq!(
            hoglin.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_IMMUNE_TO_ZOMBIFICATION",
                data_id: 18,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(false),
            }]
        );
        assert_eq!(VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID, 18);
        assert!(!VANILLA_HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION);
        let abstract_piglin =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractPiglin);
        assert_eq!(
            abstract_piglin.parent,
            Some(VanillaEntityMetadataFamily::Mob)
        );
        assert_eq!(abstract_piglin.inherited_field_count, 16);
        assert_eq!(abstract_piglin.own_fields.len(), 1);
        let piglin = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Piglin);
        assert_eq!(
            piglin.parent,
            Some(VanillaEntityMetadataFamily::AbstractPiglin)
        );
        assert_eq!(piglin.inherited_field_count, 17);
        assert_eq!(piglin.own_fields.len(), 3);
        let (immunity_data_id, immunity_default) =
            vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_ID,
            )
            .expect("generated AbstractPiglin immunity accessor");
        let (baby_data_id, baby_default) =
            vanilla_entity_metadata_baby_accessor_for_entity_type(VANILLA_ENTITY_TYPE_PIGLIN_ID)
                .expect("generated Piglin baby accessor");
        let (dancing_data_id, dancing_default) =
            vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_ID,
            )
            .expect("generated Piglin dancing accessor");
        assert_eq!(immunity_data_id, abstract_piglin.own_fields[0].data_id);
        assert_eq!(baby_data_id, piglin.own_fields[0].data_id);
        let (charging_data_id, charging_default) =
            vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_ID,
            )
            .expect("generated Piglin charging-crossbow accessor");
        assert_eq!(charging_data_id, piglin.own_fields[1].data_id);
        assert_eq!(dancing_data_id, piglin.own_fields[2].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(immunity_default),
            abstract_piglin.own_fields[0].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(baby_default),
            piglin.own_fields[0].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(charging_default),
            piglin.own_fields[1].default
        );
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(dancing_default),
            piglin.own_fields[2].default
        );
        assert_eq!(
            vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_ID
            ),
            Some((immunity_data_id, immunity_default))
        );
        assert_eq!(
            vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            ),
            Some((immunity_data_id, immunity_default))
        );
        assert_eq!(
            vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_HOGLIN_ID
            ),
            Some((18, false))
        );
        assert_eq!(
            vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOGLIN_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(-1),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_baby_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            ),
            None
        );
        assert_eq!(
            vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            ),
            None
        );
        let raider = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Raider);
        assert_eq!(raider.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(raider.inherited_field_count, 16);
        assert_eq!(raider.own_fields.len(), 1);
        let pillager = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Pillager);
        assert_eq!(pillager.parent, Some(VanillaEntityMetadataFamily::Raider));
        assert_eq!(pillager.inherited_field_count, 17);
        assert_eq!(pillager.own_fields.len(), 1);
        let (charging_data_id, charging_default) =
            vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PILLAGER_ID,
            )
            .expect("generated Pillager charging-crossbow accessor");
        assert_eq!(charging_data_id, pillager.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Boolean(charging_default),
            pillager.own_fields[0].default
        );
        let spellcaster =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::SpellcasterIllager);
        assert_eq!(
            spellcaster.parent,
            Some(VanillaEntityMetadataFamily::Raider)
        );
        assert_eq!(spellcaster.inherited_field_count, 17);
        assert_eq!(spellcaster.own_fields.len(), 1);
        let abstract_fish =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractFish);
        assert_eq!(abstract_fish.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(abstract_fish.inherited_field_count, 16);
        assert_eq!(
            abstract_fish.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "FROM_BUCKET",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(false),
            }]
        );
        let pufferfish = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Pufferfish);
        assert_eq!(
            pufferfish.parent,
            Some(VanillaEntityMetadataFamily::AbstractFish)
        );
        assert_eq!(pufferfish.inherited_field_count, 17);
        assert_eq!(pufferfish.own_fields.len(), 1);
        let (puff_state_data_id, puff_state_default) =
            vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
            )
            .expect("generated Pufferfish puff-state accessor");
        assert_eq!(puff_state_data_id, pufferfish.own_fields[0].data_id);
        assert_eq!(
            VanillaEntityMetadataDefault::Int(puff_state_default),
            pufferfish.own_fields[0].default
        );
        assert_eq!(VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID, 16);
        assert!(!VANILLA_ABSTRACT_FISH_DEFAULT_FROM_BUCKET);
        let tadpole = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Tadpole);
        assert_eq!(
            tadpole.parent,
            Some(VanillaEntityMetadataFamily::AbstractFish)
        );
        assert_eq!(tadpole.inherited_field_count, 17);
        assert_eq!(tadpole.own_fields.len(), 1);
        assert_eq!(VANILLA_TADPOLE_AGE_LOCKED_DATA_ID, 17);
        assert!(!VANILLA_TADPOLE_DEFAULT_AGE_LOCKED);
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Entity)
                .own_flags
                .iter()
                .map(|flag| flag.name)
                .collect::<Vec<_>>(),
            vec![
                "FLAG_ONFIRE",
                "FLAG_SHIFT_KEY_DOWN",
                "FLAG_SPRINTING",
                "FLAG_SWIMMING",
                "FLAG_INVISIBLE",
                "FLAG_GLOWING",
                "FLAG_FALL_FLYING",
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Avatar)
                .own_flags
                .iter()
                .map(|flag| flag.name)
                .collect::<Vec<_>>(),
            vec![
                "MODEL_PART_CAPE",
                "MODEL_PART_JACKET",
                "MODEL_PART_LEFT_SLEEVE",
                "MODEL_PART_RIGHT_SLEEVE",
                "MODEL_PART_LEFT_PANTS_LEG",
                "MODEL_PART_RIGHT_PANTS_LEG",
                "MODEL_PART_HAT",
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_field(
                VanillaEntityMetadataFamily::Player,
                VANILLA_ENTITY_POSE_DATA_ID,
            )
            .map(|field| field.name),
            Some("DATA_POSE")
        );
        assert_eq!(VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS, 300);
        assert_eq!(
            VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1.len(),
            7
        );
        assert!(VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1
            .windows(2)
            .all(|entries| entries[0].0 < entries[1].0));
        for (entity_type_id, expected) in [
            (VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, 8),
            (VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, 6),
            (VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, 6),
            (VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, 1),
            (VANILLA_ENTITY_TYPE_MINECART_ID, 6),
            (VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID, 6),
            (VANILLA_ENTITY_TYPE_TNT_MINECART_ID, 6),
        ] {
            assert_eq!(
                vanilla_minecart_default_display_offset(entity_type_id),
                Some(expected)
            );
        }
        assert_eq!(
            vanilla_minecart_default_display_offset(VANILLA_ENTITY_TYPE_PLAYER_ID),
            None
        );
        assert_eq!(vanilla_minecart_default_display_offset(-1), None);
        assert_eq!(
            VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE,
            [-10.0, 0.0, -10.0]
        );
        assert_eq!(VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID, 3);
    }
}
