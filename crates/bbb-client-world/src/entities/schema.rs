//! Vanilla 26.1 entity metadata facts.
//!
//! The generated child module holds one row per entity type and one field
//! table per metadata family (every Java class that declares synched data).
//! This facade owns the value types and the queries over those tables.

use bbb_protocol::entity_data_serializers::{
    vanilla_entity_data_serializer, vanilla_entity_data_serializer_for_constant_name,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
    VANILLA_ENTITY_TYPE_PAINTING_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
};

mod accessor_types;
mod accessors;
pub use accessor_types::*;
pub use accessors::*;

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

/// One synched-data accessor declared by a metadata family.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaEntityMetadataFieldSchema {
    pub family: VanillaEntityMetadataFamily,
    pub java_name: &'static str,
    pub data_id: u8,
    pub serializer_id: i32,
    /// `None` when Vanilla computes the default at runtime.
    pub default: Option<VanillaEntityMetadataDefault>,
}

// Typed defaults for the generated constants; mismatches fail const evaluation.
impl VanillaEntityMetadataFieldSchema {
    pub(crate) const fn typed_default(&self) -> VanillaEntityMetadataDefault {
        match self.default {
            Some(default) => default,
            None => panic!("field has a runtime default"),
        }
    }

    pub(crate) const fn bool_default(&self) -> bool {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Boolean(value) => value,
            _ => panic!("not a boolean field"),
        }
    }

    pub(crate) const fn byte_default(&self) -> i8 {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Byte(value) => value,
            _ => panic!("not a byte field"),
        }
    }

    pub(crate) const fn int_default(&self) -> i32 {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Int(value) => value,
            _ => panic!("not an int field"),
        }
    }

    pub(crate) const fn long_default(&self) -> i64 {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Long(value) => value,
            _ => panic!("not a long field"),
        }
    }

    pub(crate) const fn float_default(&self) -> f32 {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Float(value) => value,
            _ => panic!("not a float field"),
        }
    }

    pub(crate) const fn pose_default(&self) -> i32 {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Pose(value) => value,
            _ => panic!("not a pose field"),
        }
    }

    pub(crate) const fn direction_default(&self) -> i32 {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Direction(value) => value,
            _ => panic!("not a direction field"),
        }
    }

    pub(crate) const fn rotations_default(&self) -> [f32; 3] {
        match self.typed_default() {
            VanillaEntityMetadataDefault::Rotations(value) => value,
            _ => panic!("not a rotations field"),
        }
    }

    pub(crate) const fn resource_default(&self) -> &'static str {
        match self.typed_default() {
            VanillaEntityMetadataDefault::RegistryHolderResource(value)
            | VanillaEntityMetadataDefault::ItemStackDefaultItem(value) => value,
            _ => panic!("not a resource field"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaEntityMetadataFamilySchema {
    pub java_class: &'static str,
    pub parent: Option<VanillaEntityMetadataFamily>,
    /// Bit `family as u32` is set for this family and every ancestor.
    pub ancestors: u128,
    pub inherited_field_count: u8,
    pub own_fields: &'static [VanillaEntityMetadataFieldSchema],
}

/// A field described the way the Java source names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityMetadataAccessorCatalogEntry {
    pub owner: &'static str,
    pub java_name: &'static str,
    pub serializer_name: &'static str,
    pub serializer_id: i32,
    pub data_id: u8,
}

impl VanillaEntityMetadataAccessorCatalogEntry {
    fn of(field: &VanillaEntityMetadataFieldSchema) -> Self {
        Self {
            owner: vanilla_entity_metadata_schema(field.family).java_class,
            java_name: field.java_name,
            serializer_name: vanilla_entity_data_serializer(field.serializer_id)
                .map_or("", |serializer| serializer.constant_name),
            serializer_id: field.serializer_id,
            data_id: field.data_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityEventHandlerCatalogEntry {
    pub owner: VanillaEntityOwner,
    pub super_handler_owner: Option<VanillaEntityOwner>,
    pub direct_event_ids: &'static [u8],
    pub delegated_event_ids: &'static [u8],
    pub forwards_unhandled_to_super: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VanillaEnderDragonPhaseCatalogEntry {
    pub phase_id: i32,
    pub java_field_name: &'static str,
    pub implementation_class: &'static str,
    pub is_sitting: bool,
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
pub enum VanillaEntityBabyBehavior {
    Never,
    Metadata,
    ArmorStandSmall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityFoodPredicate {
    Never,
    ItemTag(&'static str),
    Nautilus {
        untamed_adult_tag: &'static str,
        default_tag: &'static str,
    },
}

pub use bbb_facts::default_attributes_26_1::{
    vanilla_entity_default_attributes_by_type_id_26_1, VanillaEntityDefaultAttribute,
};

/// One row per fixed entity type, indexed by registry id.
///
/// `*_owner` fields name the nearest Java class declaring that method for the
/// runtime class; `facts` is a set of `entity_type_facts` bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityTypeMetadataCatalogEntry {
    pub resource_id: &'static str,
    /// Nearest class of the runtime class that declares synched data.
    pub family: Option<VanillaEntityMetadataFamily>,
    pub facts: u64,
    pub handle_entity_event_owner: VanillaEntityOwner,
    pub interact_owner: VanillaEntityOwner,
    /// Absent for runtime classes that do not inherit `Mob`.
    pub mob_interact_owner: Option<VanillaEntityOwner>,
    pub passenger_attachment_owner: VanillaEntityOwner,
    pub vehicle_attachment_owner: VanillaEntityOwner,
    pub controlling_passenger_owner: VanillaEntityOwner,
    /// Absent for non-`Leashable` types; follows the interface default's
    /// zero-argument virtual dispatch when no class overrides the overload.
    pub leash_offset_owner: Option<VanillaEntityOwner>,
    pub rope_hold_position_owner: VanillaEntityOwner,
    pub food_predicate: Option<VanillaEntityFoodPredicate>,
    pub client_movement_policy: VanillaEntityClientMovementPolicy,
    pub block_effect_tick_policy: VanillaEntityBlockEffectTickPolicy,
    pub interpolation_policy: VanillaEntityInterpolationPolicy,
    pub baby_behavior: VanillaEntityBabyBehavior,
    /// Baby branch of the nearest `LivingEntity.getAgeScale()`, as f32 bits.
    pub baby_age_scale_bits: u32,
    pub death_sound_policy: VanillaEntityDeathSoundPolicy,
    pub hurt_sound_policy: VanillaEntityHurtSoundPolicy,
    pub sound_volume_policy: VanillaEntitySoundVolumePolicy,
    pub pushability_kind: VanillaEntityPushabilityKind,
    pub collidability_kind: VanillaEntityCollidabilityKind,
    pub boat_ride_height_multiplier_bits: Option<u32>,
}

mod generated_26_1;

use generated_26_1::entity_type_facts as fact;
pub use generated_26_1::*;

fn entry(entity_type_id: i32) -> Option<&'static VanillaEntityTypeMetadataCatalogEntry> {
    VANILLA_ENTITY_TYPE_METADATA_26_1.get(usize::try_from(entity_type_id).ok()?)
}

fn has_fact(entity_type_id: i32, fact: u64) -> bool {
    entry(entity_type_id).is_some_and(|entry| entry.facts & fact != 0)
}

/// Returns whether the entity type's metadata class is `family` or inherits it.
pub(crate) fn vanilla_entity_type_extends(
    entity_type_id: i32,
    family: VanillaEntityMetadataFamily,
) -> bool {
    entry(entity_type_id)
        .and_then(|entry| entry.family)
        .is_some_and(|own| vanilla_entity_metadata_schema(own).ancestors >> family as u32 & 1 != 0)
}

/// Walks the family chain from `family` up to `Entity`.
fn family_chain(
    family: Option<VanillaEntityMetadataFamily>,
) -> impl Iterator<Item = &'static VanillaEntityMetadataFamilySchema> {
    std::iter::successors(family.map(vanilla_entity_metadata_schema), |schema| {
        schema.parent.map(vanilla_entity_metadata_schema)
    })
}

pub fn vanilla_entity_type_has_attribute(entity_type_id: i32, attribute_id: i32) -> bool {
    vanilla_entity_type_default_attribute(entity_type_id, attribute_id).is_some()
}

pub fn vanilla_entity_type_default_attribute(
    entity_type_id: i32,
    attribute_id: i32,
) -> Option<VanillaEntityDefaultAttribute> {
    vanilla_entity_default_attributes_by_type_id_26_1(entity_type_id)?
        .iter()
        .copied()
        .find(|&(id, _)| id == attribute_id)
}

/// Resolves the field at `data_id` along `family`'s inheritance chain.
fn vanilla_entity_metadata_field(
    family: VanillaEntityMetadataFamily,
    data_id: u8,
) -> Option<&'static VanillaEntityMetadataFieldSchema> {
    let schema =
        family_chain(Some(family)).find(|schema| data_id >= schema.inherited_field_count)?;
    schema
        .own_fields
        .get(usize::from(data_id - schema.inherited_field_count))
}

/// Resolves the field an entity type publishes at `data_id`.
///
/// Vanilla indexes `SynchedEntityData.itemsById` by this value before comparing
/// the accessor's serializer in `assignValue`.
pub(crate) fn vanilla_entity_metadata_field_for_entity_type(
    entity_type_id: i32,
    data_id: u8,
) -> Option<&'static VanillaEntityMetadataFieldSchema> {
    vanilla_entity_metadata_field(entry(entity_type_id)?.family?, data_id)
}

/// Resolves the nearest field named `java_name` for an entity type.
pub fn vanilla_entity_metadata_accessor_for_entity_type(
    entity_type_id: i32,
    java_name: &str,
) -> Option<VanillaEntityMetadataAccessorCatalogEntry> {
    family_chain(entry(entity_type_id)?.family)
        .find_map(|schema| {
            schema
                .own_fields
                .iter()
                .find(|field| field.java_name == java_name)
        })
        .map(VanillaEntityMetadataAccessorCatalogEntry::of)
}

pub fn vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
    entity_type_id: i32,
    data_id: u8,
) -> Option<VanillaEntityMetadataAccessorCatalogEntry> {
    vanilla_entity_metadata_field_for_entity_type(entity_type_id, data_id)
        .map(VanillaEntityMetadataAccessorCatalogEntry::of)
}

/// Resolves the only field an entity type publishes with `serializer_name`.
pub fn vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
    entity_type_id: i32,
    serializer_name: &str,
) -> Option<VanillaEntityMetadataAccessorCatalogEntry> {
    let serializer_id = vanilla_entity_data_serializer_for_constant_name(serializer_name)?.id;
    let mut matches = family_chain(entry(entity_type_id)?.family)
        .flat_map(|schema| schema.own_fields)
        .filter(|field| field.serializer_id == serializer_id);
    let field = matches.next()?;
    matches
        .next()
        .is_none()
        .then(|| VanillaEntityMetadataAccessorCatalogEntry::of(field))
}

/// Returns `isBaby` dispatch and the exact baby `getAgeScale` value for a living type.
pub(crate) fn vanilla_entity_metadata_baby_behavior_and_age_scale(
    entity_type_id: i32,
) -> Option<(VanillaEntityBabyBehavior, f32)> {
    let entry = entry(entity_type_id)?;
    (entry.facts & fact::LIVING != 0).then_some((
        entry.baby_behavior,
        f32::from_bits(entry.baby_age_scale_bits),
    ))
}

macro_rules! type_facts {
    ($($vis:vis fn $name:ident => $fact:ident;)*) => {$(
        $vis fn $name(entity_type_id: i32) -> bool {
            has_fact(entity_type_id, fact::$fact)
        }
    )*};
}

type_facts! {
    pub fn vanilla_entity_metadata_type_is_living => LIVING;
    pub fn vanilla_entity_metadata_type_is_mob => MOB;
    pub fn vanilla_entity_metadata_type_is_animal => ANIMAL;
    pub fn vanilla_entity_metadata_type_is_tamable_animal => TAMABLE_ANIMAL;
    pub fn vanilla_entity_metadata_type_is_projectile => PROJECTILE;
    pub fn vanilla_entity_metadata_type_is_hurting_projectile => HURTING_PROJECTILE;
    pub fn vanilla_entity_metadata_type_is_wind_charge => WIND_CHARGE;
    pub(crate) fn vanilla_entity_metadata_type_is_vehicle => VEHICLE;
    pub fn vanilla_entity_metadata_type_is_boat => BOAT;
    pub fn vanilla_entity_metadata_type_is_abstract_chest_boat => ABSTRACT_CHEST_BOAT;
    pub fn vanilla_entity_metadata_type_is_minecart => MINECART;
    pub fn vanilla_entity_metadata_type_is_abstract_horse => ABSTRACT_HORSE;
    pub fn vanilla_entity_metadata_type_is_abstract_nautilus => ABSTRACT_NAUTILUS;
    pub fn vanilla_entity_metadata_type_is_llama => LLAMA;
    pub fn vanilla_entity_metadata_type_is_guardian => GUARDIAN;
    pub fn vanilla_entity_metadata_type_is_camel => CAMEL;
    pub fn vanilla_entity_metadata_type_is_squid => SQUID;
    pub fn vanilla_entity_metadata_type_is_slime => SLIME;
    pub fn vanilla_entity_metadata_type_is_abstract_piglin => ABSTRACT_PIGLIN;
    pub fn vanilla_entity_metadata_type_is_flying_animal => FLYING_ANIMAL;
    pub fn vanilla_entity_metadata_type_is_leashable => LEASHABLE;
    pub(crate) fn vanilla_entity_metadata_type_is_hoglin_base => HOGLIN_BASE;
    pub fn vanilla_entity_metadata_type_is_player_rideable_jumping => PLAYER_RIDEABLE_JUMPING;
    pub fn vanilla_entity_metadata_type_has_custom_inventory_screen => HAS_CUSTOM_INVENTORY_SCREEN;
    pub fn vanilla_entity_metadata_type_is_menu_provider => MENU_PROVIDER;
    pub(crate) fn vanilla_entity_metadata_type_pushes_local_player_on_client_tick
        => PUSHES_LOCAL_PLAYER_ON_CLIENT_TICK;
    pub fn vanilla_entity_metadata_type_can_use_every_equipment_slot => CAN_USE_EVERY_EQUIPMENT_SLOT;
    pub fn vanilla_entity_metadata_type_illager_arm_pose_uses_celebrating
        => ILLAGER_ARM_POSE_USES_CELEBRATING;
}

/// Returns whether vanilla's class hierarchy and entity-type tags make this type
/// eligible for `Entity.isPickable` before per-instance state gates.
pub fn vanilla_entity_metadata_type_is_pickable(entity_type_id: i32) -> bool {
    has_fact(entity_type_id, fact::PICKABLE_BY_CLASS)
        || bbb_protocol::entity_types::vanilla_entity_type_is_redirectable_projectile(
            entity_type_id,
        )
}

pub fn vanilla_entity_metadata_handle_entity_event_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id).map(|entry| entry.handle_entity_event_owner)
}

pub(crate) fn vanilla_entity_metadata_interact_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id).map(|entry| entry.interact_owner)
}

pub(crate) fn vanilla_entity_metadata_mob_interact_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id)?.mob_interact_owner
}

pub(crate) fn vanilla_entity_metadata_food_predicate(
    entity_type_id: i32,
) -> Option<VanillaEntityFoodPredicate> {
    entry(entity_type_id)?.food_predicate
}

pub(crate) fn vanilla_entity_metadata_passenger_attachment_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id).map(|entry| entry.passenger_attachment_owner)
}

pub(crate) fn vanilla_entity_metadata_vehicle_attachment_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id).map(|entry| entry.vehicle_attachment_owner)
}

pub(crate) fn vanilla_entity_metadata_controlling_passenger_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id).map(|entry| entry.controlling_passenger_owner)
}

pub(crate) fn vanilla_entity_metadata_leash_offset_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id)?.leash_offset_owner
}

pub(crate) fn vanilla_entity_metadata_rope_hold_position_owner(
    entity_type_id: i32,
) -> Option<VanillaEntityOwner> {
    entry(entity_type_id).map(|entry| entry.rope_hold_position_owner)
}

/// Resolves the vanilla declaration that consumes one entity event after following the
/// source-generated `super.handleEntityEvent(id)` chain. Listener-special ids (`21`, `35`, and
/// `63`) are intercepted by `ClientPacketListener` before this class dispatch and therefore do
/// not resolve here.
pub(crate) fn vanilla_entity_metadata_handle_entity_event_owner_for_event(
    entity_type_id: i32,
    event_id: u8,
) -> Option<VanillaEntityOwner> {
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
        == Some(VanillaEntityOwner::Ocelot)
}

/// Returns whether entity event dispatch reaches `FishingHook.handleEntityEvent(byte)` first.
pub(crate) fn vanilla_entity_metadata_type_uses_fishing_hook_entity_event_handler(
    entity_type_id: i32,
) -> bool {
    vanilla_entity_metadata_handle_entity_event_owner(entity_type_id)
        == Some(VanillaEntityOwner::FishingHook)
}

/// Returns whether entity event dispatch reaches `Player.handleEntityEvent(byte)` first.
pub(crate) fn vanilla_entity_metadata_type_uses_player_entity_event_handler(
    entity_type_id: i32,
) -> bool {
    vanilla_entity_metadata_handle_entity_event_owner(entity_type_id)
        == Some(VanillaEntityOwner::Player)
}

pub fn vanilla_entity_metadata_type_death_sound_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityDeathSoundPolicy> {
    entry(entity_type_id).map(|entry| entry.death_sound_policy)
}

pub fn vanilla_entity_metadata_type_hurt_sound_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityHurtSoundPolicy> {
    entry(entity_type_id).map(|entry| entry.hurt_sound_policy)
}

pub fn vanilla_entity_metadata_type_sound_volume_policy(
    entity_type_id: i32,
) -> Option<VanillaEntitySoundVolumePolicy> {
    entry(entity_type_id).map(|entry| entry.sound_volume_policy)
}

/// Returns the nearest `Entity.isPushable()` class behavior; instance state stays caller-owned.
pub fn vanilla_entity_metadata_type_pushability_kind(
    entity_type_id: i32,
) -> Option<VanillaEntityPushabilityKind> {
    entry(entity_type_id).map(|entry| entry.pushability_kind)
}

/// Returns the nearest `Entity.canBeCollidedWith(other)` class behavior.
pub fn vanilla_entity_metadata_type_collidability_kind(
    entity_type_id: i32,
) -> Option<VanillaEntityCollidabilityKind> {
    entry(entity_type_id).map(|entry| entry.collidability_kind)
}

/// Returns the post-chain resource selected by vanilla `GameRenderer` for a camera entity type.
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

/// Returns whether Vanilla's client-side `hurtClient` path accepts an ordinary
/// player attack and reaches `Player.causeExtraKnockback` for this non-living type.
///
/// The 26.1 accepting owners are `VehicleEntity` (all subclasses), `ShulkerBullet`
/// and `EndCrystal`. `BlockAttachedEntity` accepts client hurt too, but its
/// `skipAttackInteraction` hook consumes the attack first.
pub(crate) fn vanilla_entity_metadata_type_accepts_local_attack_hurt(entity_type_id: i32) -> bool {
    has_fact(entity_type_id, fact::VEHICLE)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID | VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
        )
}

/// Returns whether Vanilla's external client accepts an AbstractArrow damage
/// simulation for this runtime class.
///
/// The accepting 26.1 overrides are VehicleEntity, ExperienceOrb, ItemEntity,
/// ShulkerBullet, EndCrystal and the BlockAttachedEntity family (Painting,
/// LeashFenceKnotEntity and item frames; remote frames keep `fixed == false`).
/// ItemEntity's damage-resistant component and EndCrystal's EnderDragon-owner
/// exception are refined by WorldStore.
pub(crate) fn vanilla_entity_metadata_type_accepts_projectile_hurt_client(
    entity_type_id: i32,
) -> bool {
    has_fact(entity_type_id, fact::VEHICLE)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID
                | VANILLA_ENTITY_TYPE_ITEM_ID
                | VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID
                | VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
                | VANILLA_ENTITY_TYPE_PAINTING_ID
                | VANILLA_ENTITY_TYPE_LEASH_KNOT_ID
                | VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
                | VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
        )
}

/// Returns the concrete boat `rideHeight(EntityDimensions)` multiplier.
pub fn vanilla_entity_metadata_type_boat_ride_height_multiplier(
    entity_type_id: i32,
) -> Option<f32> {
    entry(entity_type_id)?
        .boat_ride_height_multiplier_bits
        .map(f32::from_bits)
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
    entry(entity_type_id).map(|entry| entry.client_movement_policy)
}

pub fn vanilla_entity_metadata_type_block_effect_tick_policy(
    entity_type_id: i32,
) -> Option<VanillaEntityBlockEffectTickPolicy> {
    entry(entity_type_id).map(|entry| entry.block_effect_tick_policy)
}

/// Returns the vanilla client interpolation-handler length for an entity type.
///
/// `Shulker` overrides the inherited `LivingEntity` handler with `null`; displays
/// keep an explicit zero-length handler; minecarts expose the old-behavior
/// handler only while `minecart_improvements` is disabled.
pub(crate) fn vanilla_entity_metadata_type_interpolation_steps(
    entity_type_id: i32,
    minecart_improvements_enabled: bool,
) -> Option<u8> {
    match entry(entity_type_id)?.interpolation_policy {
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

pub fn vanilla_minecart_default_display_offset(entity_type_id: i32) -> Option<i32> {
    VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1
        .binary_search_by_key(&entity_type_id, |(candidate, _)| *candidate)
        .ok()
        .map(|index| VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1[index].1)
}
