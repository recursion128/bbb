//! Vanilla 26.1 entity metadata semantic facade.
//!
//! Mechanical class/accessor/default/flag/type facts live in the checked-in
//! generated child module. This facade owns the stable Rust value types,
//! compatibility queries, and virtual-default resolution used by world/native
//! consumers. Ordinary builds never read the external vanilla source tree.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
};

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
pub struct VanillaEntityTypeMetadataCatalogEntry {
    pub entity_type_id: i32,
    pub constant_name: &'static str,
    pub resource_id: &'static str,
    pub runtime_class: Option<&'static str>,
    pub schema_class: Option<&'static str>,
    pub is_pickable_by_class: bool,
    pub sound_source_kind: VanillaEntitySoundSourceKind,
    pub is_living: bool,
    pub is_animal: bool,
    pub is_projectile: bool,
    pub is_hurting_projectile: bool,
    pub is_wind_charge: bool,
    pub is_boat: bool,
    pub is_abstract_chest_boat: bool,
    pub is_minecart: bool,
    pub is_abstract_horse: bool,
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
    let mut java_class = VANILLA_ENTITY_TYPE_METADATA_26_1
        .iter()
        .find(|entry| entry.entity_type_id == entity_type_id)?
        .schema_class?;
    loop {
        if let Some(accessor) = VANILLA_ENTITY_METADATA_ACCESSORS_26_1
            .iter()
            .find(|entry| entry.owner == java_class && entry.java_name == java_name)
        {
            return Some(accessor);
        }
        java_class = VANILLA_ENTITY_METADATA_CLASSES_26_1
            .iter()
            .find(|entry| entry.java_class == java_class)?
            .superclass?;
    }
}

/// Resolves the single accessor with `serializer_name` along an entity type's generated schema
/// inheritance chain. Missing and ambiguous matches both return `None`, so consumers cannot silently
/// choose one of several same-serializer fields.
pub fn vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
    entity_type_id: i32,
    serializer_name: &str,
) -> Option<&'static VanillaEntityMetadataAccessorCatalogEntry> {
    let mut java_class = VANILLA_ENTITY_TYPE_METADATA_26_1
        .iter()
        .find(|entry| entry.entity_type_id == entity_type_id)?
        .schema_class?;
    let mut resolved = None;
    loop {
        for accessor in VANILLA_ENTITY_METADATA_ACCESSORS_26_1
            .iter()
            .filter(|entry| entry.owner == java_class && entry.serializer_name == serializer_name)
        {
            if resolved.replace(accessor).is_some() {
                return None;
            }
        }
        let Some(superclass) = VANILLA_ENTITY_METADATA_CLASSES_26_1
            .iter()
            .find(|entry| entry.java_class == java_class)
            .and_then(|entry| entry.superclass)
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

fn vanilla_class_is_or_implements(java_class: &str, target: &str, depth: usize) -> bool {
    if java_class == target {
        return true;
    }
    if depth >= VANILLA_ENTITY_METADATA_CLASSES_26_1.len() {
        return false;
    }
    let Some(class) = VANILLA_ENTITY_METADATA_CLASSES_26_1
        .iter()
        .find(|entry| entry.java_class == java_class)
    else {
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

pub fn vanilla_entity_metadata_type_is_boat(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_boat)
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

pub fn vanilla_entity_metadata_type_is_abstract_horse(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_abstract_horse)
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

pub fn vanilla_entity_metadata_type_is_spider(entity_type_id: i32) -> bool {
    VANILLA_ENTITY_TYPE_METADATA_26_1
        .get(usize::try_from(entity_type_id).unwrap_or(usize::MAX))
        .map(|entry| entry.is_spider)
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

pub const fn vanilla_minecart_default_display_offset(entity_type_id: i32) -> i32 {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => VANILLA_CHEST_MINECART_DEFAULT_DISPLAY_OFFSET,
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => VANILLA_HOPPER_MINECART_DEFAULT_DISPLAY_OFFSET,
        _ => VANILLA_ABSTRACT_MINECART_DEFAULT_DISPLAY_OFFSET,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use bbb_protocol::entity_data_serializers::{
        vanilla_entity_data_serializer_for_constant_name, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    };
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
        VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
        VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        VANILLA_ENTITY_TYPE_PARCHED_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
        VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    };

    use super::*;

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

        assert_eq!(VANILLA_SLIME_SIZE_DATA_ID, 16);
        assert_eq!(VANILLA_SHULKER_ATTACH_FACE_DATA_ID, 16);
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
        assert!(!vanilla_entity_metadata_type_is_leashable(-1));
        assert!(!vanilla_entity_metadata_type_is_leashable(157));
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
            "is_vanilla_camel_type",
            "vanilla_entity_type_is_squid",
            "is_vanilla_slime_type",
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
        let piglin_query = dimensions_source
            .split("fn vanilla_piglin_melee_attack_family")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("piglin class query source");
        assert!(!piglin_query.contains("matches!"));
        let chest_boat_query = dimensions_source
            .split("fn is_vanilla_chest_boat_or_raft_type")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("chest boat class query source");
        assert!(!chest_boat_query.contains("matches!"));
        let animations_source = include_str!("animations.rs");
        for function in ["is_guardian_entity_type", "is_flying_animal"] {
            let body = animations_source
                .split(&format!("fn {function}"))
                .nth(1)
                .and_then(|source| source.split_once("\n}"))
                .map(|(body, _)| body)
                .expect("animation membership query source");
            assert!(!body.contains("matches!"));
        }
        let arrow_query = animations_source
            .split("fn is_vanilla_arrow_type")
            .nth(1)
            .and_then(|source| source.split_once("\n}"))
            .map(|(body, _)| body)
            .expect("arrow tag query source");
        assert!(!arrow_query.contains("matches!"));
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
        assert_eq!(VANILLA_ALLAY_DANCING_DATA_ID, 16);
        assert_eq!(VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID, 17);
        assert!(!VANILLA_ALLAY_DEFAULT_DANCING);
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
        assert_eq!(VANILLA_WARDEN_ANGER_LEVEL_DATA_ID, 16);
        assert_eq!(VANILLA_WARDEN_DEFAULT_ANGER_LEVEL, 0);
        let wither = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::WitherBoss);
        assert_eq!(wither.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(wither.inherited_field_count, 16);
        assert_eq!(wither.own_fields.len(), 4);
        assert_eq!(VANILLA_WITHER_TARGET_A_DATA_ID, 16);
        assert_eq!(VANILLA_WITHER_TARGET_B_DATA_ID, 17);
        assert_eq!(VANILLA_WITHER_TARGET_C_DATA_ID, 18);
        assert_eq!(VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID, 19);
        assert_eq!(VANILLA_WITHER_DEFAULT_TARGET_A, 0);
        assert_eq!(VANILLA_WITHER_DEFAULT_TARGET_B, 0);
        assert_eq!(VANILLA_WITHER_DEFAULT_TARGET_C, 0);
        assert_eq!(VANILLA_WITHER_DEFAULT_INVULNERABLE_TICKS, 0);
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
        assert_eq!(VANILLA_ARMADILLO_STATE_DATA_ID, 18);
        assert_eq!(VANILLA_ARMADILLO_DEFAULT_STATE, 0);
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
        assert_eq!(VANILLA_AXOLOTL_VARIANT_DATA_ID, 18);
        assert_eq!(VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID, 19);
        assert_eq!(VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID, 20);
        assert_eq!(VANILLA_AXOLOTL_DEFAULT_VARIANT, 0);
        assert!(!VANILLA_AXOLOTL_DEFAULT_PLAYING_DEAD);
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
        assert_eq!(VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID, 16);
        assert_eq!(VANILLA_COPPER_GOLEM_STATE_DATA_ID, 17);
        assert_eq!(VANILLA_COPPER_GOLEM_DEFAULT_WEATHER_STATE, 0);
        assert_eq!(VANILLA_COPPER_GOLEM_DEFAULT_STATE, 0);
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
        assert_eq!(VANILLA_SHEEP_WOOL_DATA_ID, 18);
        assert_eq!(VANILLA_SHEEP_DEFAULT_WOOL, 0);
        assert_eq!(VANILLA_SHEEP_FLAG_SHEARED, 16);
        assert_eq!(VANILLA_SHEEP_WOOL_COLOR_MASK, 15);
        assert_eq!(VANILLA_HORSE_COLOR_MASK, 0xFF);
        assert_eq!(VANILLA_HORSE_MARKINGS_MASK, 0xFF00);
        assert_eq!(VANILLA_HORSE_MARKINGS_SHIFT, 8);
        assert_eq!(VANILLA_TROPICAL_FISH_BASE_COLOR_MASK, 0xFF);
        assert_eq!(VANILLA_TROPICAL_FISH_BASE_COLOR_SHIFT, 16);
        assert_eq!(VANILLA_TROPICAL_FISH_PATTERN_COLOR_MASK, 0xFF);
        assert_eq!(VANILLA_TROPICAL_FISH_PATTERN_COLOR_SHIFT, 24);
        assert_eq!(VANILLA_TROPICAL_FISH_PATTERN_MASK, 0xFFFF);
        let sniffer = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Sniffer);
        assert_eq!(
            sniffer.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(sniffer.inherited_field_count, 18);
        assert_eq!(sniffer.own_fields.len(), 2);
        assert_eq!(VANILLA_SNIFFER_STATE_DATA_ID, 18);
        assert_eq!(VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID, 19);
        assert_eq!(VANILLA_SNIFFER_DEFAULT_STATE, 0);
        assert_eq!(VANILLA_SNIFFER_DEFAULT_DROP_SEED_AT_TICK, 0);
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
        assert_eq!(VANILLA_CAMEL_DASH_DATA_ID, 19);
        assert_eq!(VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, 20);
        assert!(!VANILLA_CAMEL_DEFAULT_DASH);
        assert_eq!(VANILLA_CAMEL_DEFAULT_LAST_POSE_CHANGE_TICK, 0);
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
        assert_eq!(VANILLA_CAT_IS_LYING_DATA_ID, 21);
        assert_eq!(VANILLA_CAT_RELAX_STATE_ONE_DATA_ID, 22);
        assert_eq!(VANILLA_CAT_COLLAR_COLOR_DATA_ID, 23);
        assert_eq!(VANILLA_CAT_SOUND_VARIANT_DATA_ID, 24);
        assert_eq!(VANILLA_CAT_DEFAULT_VARIANT_RESOURCE_ID, "minecraft:black");
        assert!(!VANILLA_CAT_DEFAULT_IS_LYING);
        assert!(!VANILLA_CAT_DEFAULT_RELAX_STATE_ONE);
        assert_eq!(VANILLA_CAT_DEFAULT_COLLAR_COLOR, 14);
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
        assert_eq!(VANILLA_WOLF_INTERESTED_DATA_ID, 20);
        assert_eq!(VANILLA_WOLF_COLLAR_COLOR_DATA_ID, 21);
        assert_eq!(VANILLA_WOLF_ANGER_END_TIME_DATA_ID, 22);
        assert_eq!(VANILLA_WOLF_VARIANT_DATA_ID, 23);
        assert_eq!(VANILLA_WOLF_SOUND_VARIANT_DATA_ID, 24);
        assert!(!VANILLA_WOLF_DEFAULT_INTERESTED);
        assert_eq!(VANILLA_WOLF_DEFAULT_COLLAR_COLOR, 14);
        assert_eq!(VANILLA_WOLF_DEFAULT_ANGER_END_TIME, -1);
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
        assert_eq!(VANILLA_MUSHROOM_COW_TYPE_DATA_ID, 18);
        assert_eq!(VANILLA_MUSHROOM_COW_DEFAULT_TYPE, 0);
        assert_eq!(
            mushroom_cow.own_fields[0].default,
            VanillaEntityMetadataDefault::Int(0)
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
        assert_eq!(VANILLA_PARROT_VARIANT_DATA_ID, 20);
        assert_eq!(VANILLA_PARROT_DEFAULT_VARIANT, 0);
        let rabbit = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Rabbit);
        assert_eq!(rabbit.parent, Some(VanillaEntityMetadataFamily::AgeableMob));
        assert_eq!(rabbit.inherited_field_count, 18);
        assert_eq!(rabbit.own_fields.len(), 1);
        assert_eq!(VANILLA_RABBIT_TYPE_DATA_ID, 18);
        assert_eq!(VANILLA_RABBIT_DEFAULT_TYPE, 0);
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
        assert_eq!(VANILLA_SALMON_VARIANT_DATA_ID, 17);
        assert_eq!(VANILLA_SALMON_DEFAULT_VARIANT, 1);
        let tropical_fish =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::TropicalFish);
        assert_eq!(
            tropical_fish.parent,
            Some(VanillaEntityMetadataFamily::AbstractFish)
        );
        assert_eq!(tropical_fish.inherited_field_count, 17);
        assert_eq!(tropical_fish.own_fields.len(), 1);
        assert_eq!(VANILLA_TROPICAL_FISH_VARIANT_DATA_ID, 17);
        assert_eq!(VANILLA_TROPICAL_FISH_DEFAULT_VARIANT, 0);
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
        assert_eq!(VANILLA_POLAR_BEAR_STANDING_DATA_ID, 18);
        assert!(!VANILLA_POLAR_BEAR_DEFAULT_STANDING);
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
        assert_eq!(VANILLA_ENDER_DRAGON_PHASE_DATA_ID, 16);
        assert_eq!(VANILLA_ENDER_DRAGON_DEFAULT_PHASE, 10);
        assert_eq!(VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID, 0);
        assert_eq!(VANILLA_ENDER_DRAGON_PHASE_SITTING_FLAMING_ID, 5);
        assert_eq!(VANILLA_ENDER_DRAGON_PHASE_SITTING_SCANNING_ID, 6);
        assert_eq!(VANILLA_ENDER_DRAGON_PHASE_SITTING_ATTACKING_ID, 7);
        assert_eq!(VANILLA_ENDER_DRAGON_PHASE_HOVERING_ID, 10);
        assert_eq!(
            ender_dragon.own_fields[0].default,
            VanillaEntityMetadataDefault::Int(10)
        );
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
        assert_eq!(VANILLA_ITEM_ENTITY_STACK_DATA_ID, 8);
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
        assert_eq!(VANILLA_GUARDIAN_MOVING_DATA_ID, 16);
        assert_eq!(VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID, 17);
        assert!(!VANILLA_GUARDIAN_DEFAULT_MOVING);
        assert_eq!(VANILLA_GUARDIAN_DEFAULT_ATTACK_TARGET, 0);
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
        assert_eq!(
            snow_golem.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_PUMPKIN_ID",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                default: VanillaEntityMetadataDefault::Byte(16),
            }]
        );
        assert_eq!(VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID, 16);
        assert_eq!(VANILLA_SNOW_GOLEM_DEFAULT_PUMPKIN_FLAGS, 16);
        assert_eq!(VANILLA_SNOW_GOLEM_PUMPKIN_FLAG, 16);
        let iron_golem = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::IronGolem);
        assert_eq!(iron_golem.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(iron_golem.inherited_field_count, 16);
        assert_eq!(iron_golem.own_fields.len(), 1);
        assert_eq!(VANILLA_IRON_GOLEM_FLAGS_DATA_ID, 16);
        assert_eq!(VANILLA_IRON_GOLEM_DEFAULT_FLAGS, 0);
        assert_eq!(VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED, 1);
        assert_eq!(
            snow_golem.own_flags,
            &[VanillaEntityMetadataFlagSchema {
                name: "PUMPKIN_FLAG",
                data_id: 16,
                mask: 16,
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
        assert_eq!(
            phantom.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "ID_SIZE",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                default: VanillaEntityMetadataDefault::Int(0),
            }]
        );
        assert_eq!(VANILLA_PHANTOM_SIZE_DATA_ID, 16);
        assert_eq!(VANILLA_PHANTOM_DEFAULT_SIZE, 0);
        let bogged = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Bogged);
        assert_eq!(bogged.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(bogged.inherited_field_count, 16);
        assert_eq!(
            bogged.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_SHEARED",
                data_id: 16,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                default: VanillaEntityMetadataDefault::Boolean(false),
            }]
        );
        assert_eq!(VANILLA_BOGGED_SHEARED_DATA_ID, 16);
        assert!(!VANILLA_BOGGED_DEFAULT_SHEARED);
        let skeleton = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Skeleton);
        assert_eq!(skeleton.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(skeleton.inherited_field_count, 16);
        assert_eq!(VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID, 16);
        assert!(!VANILLA_SKELETON_DEFAULT_STRAY_CONVERSION);
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
        assert_eq!(VANILLA_CREAKING_CAN_MOVE_DATA_ID, 16);
        assert_eq!(VANILLA_CREAKING_ACTIVE_DATA_ID, 17);
        assert_eq!(VANILLA_CREAKING_TEARING_DOWN_DATA_ID, 18);
        assert_eq!(VANILLA_CREAKING_HOME_POS_DATA_ID, 19);
        assert!(VANILLA_CREAKING_DEFAULT_CAN_MOVE);
        assert!(!VANILLA_CREAKING_DEFAULT_ACTIVE);
        assert!(!VANILLA_CREAKING_DEFAULT_TEARING_DOWN);
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
        assert_eq!(VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID, 16);
        assert_eq!(VANILLA_PIGLIN_BABY_DATA_ID, 17);
        assert_eq!(VANILLA_PIGLIN_CHARGING_CROSSBOW_DATA_ID, 18);
        assert_eq!(VANILLA_PIGLIN_DANCING_DATA_ID, 19);
        assert!(!VANILLA_ABSTRACT_PIGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION);
        assert!(!VANILLA_PIGLIN_DEFAULT_BABY);
        assert!(!VANILLA_PIGLIN_DEFAULT_CHARGING_CROSSBOW);
        assert!(!VANILLA_PIGLIN_DEFAULT_DANCING);
        let raider = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Raider);
        assert_eq!(raider.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(raider.inherited_field_count, 16);
        assert_eq!(raider.own_fields.len(), 1);
        let pillager = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Pillager);
        assert_eq!(pillager.parent, Some(VanillaEntityMetadataFamily::Raider));
        assert_eq!(pillager.inherited_field_count, 17);
        assert_eq!(pillager.own_fields.len(), 1);
        assert_eq!(VANILLA_RAIDER_CELEBRATING_DATA_ID, 16);
        assert_eq!(VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID, 17);
        assert!(!VANILLA_RAIDER_DEFAULT_CELEBRATING);
        assert!(!VANILLA_PILLAGER_DEFAULT_CHARGING_CROSSBOW);
        let spellcaster =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::SpellcasterIllager);
        assert_eq!(
            spellcaster.parent,
            Some(VanillaEntityMetadataFamily::Raider)
        );
        assert_eq!(spellcaster.inherited_field_count, 17);
        assert_eq!(spellcaster.own_fields.len(), 1);
        assert_eq!(VANILLA_SPELLCASTER_ILLAGER_CASTING_DATA_ID, 17);
        assert_eq!(VANILLA_SPELLCASTER_ILLAGER_DEFAULT_CASTING, 0);
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
        assert_eq!(
            pufferfish.own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "PUFF_STATE",
                data_id: 17,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                default: VanillaEntityMetadataDefault::Int(0),
            }]
        );
        assert_eq!(VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID, 16);
        assert!(!VANILLA_ABSTRACT_FISH_DEFAULT_FROM_BUCKET);
        assert_eq!(VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID, 17);
        assert_eq!(VANILLA_PUFFERFISH_DEFAULT_PUFF_STATE, 0);
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
            vanilla_minecart_default_display_offset(VANILLA_ENTITY_TYPE_CHEST_MINECART_ID),
            8
        );
        assert_eq!(
            vanilla_minecart_default_display_offset(VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID),
            1
        );
        assert_eq!(vanilla_minecart_default_display_offset(-1), 6);
        assert_eq!(
            VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE,
            [-10.0, 0.0, -10.0]
        );
        assert_eq!(VANILLA_SHULKER_DEFAULT_ATTACH_FACE_ID, 0);
        assert_eq!(VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID, 3);
    }
}
