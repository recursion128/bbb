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
    Slime,
    Shulker,
    Zombie,
    AgeableMob,
    AbstractHorse,
    TamableAnimal,
    ArmorStand,
    AbstractArrow,
    Arrow,
    ThrownTrident,
    VehicleEntity,
    AbstractBoat,
    AbstractMinecart,
    HangingEntity,
    ItemFrame,
    Avatar,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VanillaEntityMetadataDefault {
    Byte(i8),
    Int(i32),
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
pub struct VanillaEntityTypeMetadataCatalogEntry {
    pub entity_type_id: i32,
    pub constant_name: &'static str,
    pub resource_id: &'static str,
    pub runtime_class: Option<&'static str>,
    pub schema_class: Option<&'static str>,
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
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPES;

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
            VanillaEntityMetadataFamily::Slime,
            VanillaEntityMetadataFamily::Shulker,
            VanillaEntityMetadataFamily::Zombie,
            VanillaEntityMetadataFamily::AgeableMob,
            VanillaEntityMetadataFamily::AbstractHorse,
            VanillaEntityMetadataFamily::TamableAnimal,
            VanillaEntityMetadataFamily::ArmorStand,
            VanillaEntityMetadataFamily::AbstractArrow,
            VanillaEntityMetadataFamily::Arrow,
            VanillaEntityMetadataFamily::ThrownTrident,
            VanillaEntityMetadataFamily::VehicleEntity,
            VanillaEntityMetadataFamily::AbstractBoat,
            VanillaEntityMetadataFamily::AbstractMinecart,
            VanillaEntityMetadataFamily::HangingEntity,
            VanillaEntityMetadataFamily::ItemFrame,
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
