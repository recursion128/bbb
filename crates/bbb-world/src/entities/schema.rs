use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID, VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
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

pub const VANILLA_ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
pub const VANILLA_ENTITY_AIR_SUPPLY_DATA_ID: u8 = 1;
pub const VANILLA_ENTITY_CUSTOM_NAME_DATA_ID: u8 = 2;
pub const VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID: u8 = 3;
pub const VANILLA_ENTITY_SILENT_DATA_ID: u8 = 4;
pub const VANILLA_ENTITY_NO_GRAVITY_DATA_ID: u8 = 5;
pub const VANILLA_ENTITY_POSE_DATA_ID: u8 = 6;
pub const VANILLA_ENTITY_TICKS_FROZEN_DATA_ID: u8 = 7;

pub const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
pub const VANILLA_LIVING_ENTITY_HEALTH_DATA_ID: u8 = 9;
pub const VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID: u8 = 10;
pub const VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID: u8 = 11;
pub const VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID: u8 = 12;
pub const VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID: u8 = 13;
pub const VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID: u8 = 14;
pub const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
pub const VANILLA_SLIME_SIZE_DATA_ID: u8 = 16;
pub const VANILLA_SHULKER_ATTACH_FACE_DATA_ID: u8 = 16;
pub const VANILLA_SHULKER_PEEK_DATA_ID: u8 = 17;
pub const VANILLA_SHULKER_COLOR_DATA_ID: u8 = 18;
pub const VANILLA_ZOMBIE_BABY_DATA_ID: u8 = 16;
pub const VANILLA_ZOMBIE_SPECIAL_TYPE_DATA_ID: u8 = 17;
pub const VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID: u8 = 18;
pub const VANILLA_AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
pub const VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID: u8 = 17;
pub const VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID: u8 = 18;
pub const VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID: u8 = 18;
pub const VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID: u8 = 19;
pub const VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID: u8 = 15;
pub const VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID: u8 = 16;
pub const VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID: u8 = 17;
pub const VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID: u8 = 18;
pub const VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID: u8 = 19;
pub const VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID: u8 = 20;
pub const VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID: u8 = 21;
pub const VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID: u8 = 8;
pub const VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID: u8 = 9;
pub const VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID: u8 = 10;
pub const VANILLA_VEHICLE_HURT_TIME_DATA_ID: u8 = 8;
pub const VANILLA_VEHICLE_HURT_DIR_DATA_ID: u8 = 9;
pub const VANILLA_VEHICLE_DAMAGE_DATA_ID: u8 = 10;
pub const VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID: u8 = 11;
pub const VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID: u8 = 12;
pub const VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID: u8 = 13;
pub const VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID: u8 = 11;
pub const VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID: u8 = 12;
pub const VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID: u8 = 8;
pub const VANILLA_ITEM_FRAME_ITEM_DATA_ID: u8 = 9;
pub const VANILLA_ITEM_FRAME_ROTATION_DATA_ID: u8 = 10;
pub const VANILLA_AVATAR_MAIN_HAND_DATA_ID: u8 = 15;
pub const VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID: u8 = 16;
pub const VANILLA_PLAYER_ABSORPTION_DATA_ID: u8 = 17;
pub const VANILLA_PLAYER_SCORE_DATA_ID: u8 = 18;
pub const VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID: u8 = 19;
pub const VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID: u8 = 20;

pub const VANILLA_ENTITY_SHARED_FLAG_ON_FIRE: i8 = 1 << 0;
pub const VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN: i8 = 1 << 1;
pub const VANILLA_ENTITY_SHARED_FLAG_SPRINTING: i8 = 1 << 3;
pub const VANILLA_ENTITY_SHARED_FLAG_SWIMMING: i8 = 1 << 4;
pub const VANILLA_ENTITY_SHARED_FLAG_INVISIBLE: i8 = 1 << 5;
pub const VANILLA_ENTITY_SHARED_FLAG_GLOWING: i8 = 1 << 6;
pub const VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING: i8 = 0x80_u8 as i8;

pub const VANILLA_LIVING_ENTITY_FLAG_IS_USING: i8 = 1 << 0;
pub const VANILLA_LIVING_ENTITY_FLAG_OFF_HAND: i8 = 1 << 1;
pub const VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK: i8 = 1 << 2;
pub const VANILLA_MOB_FLAG_NO_AI: i8 = 1 << 0;
pub const VANILLA_MOB_FLAG_LEFT_HANDED: i8 = 1 << 1;
pub const VANILLA_MOB_FLAG_AGGRESSIVE: i8 = 1 << 2;
pub const VANILLA_ABSTRACT_HORSE_FLAG_TAME: i8 = 2;
pub const VANILLA_ABSTRACT_HORSE_FLAG_BRED: i8 = 8;
pub const VANILLA_ABSTRACT_HORSE_FLAG_EATING: i8 = 16;
pub const VANILLA_ABSTRACT_HORSE_FLAG_STANDING: i8 = 32;
pub const VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH: i8 = 64;
pub const VANILLA_TAMABLE_ANIMAL_FLAG_SITTING: i8 = 1;
pub const VANILLA_TAMABLE_ANIMAL_FLAG_TAME: i8 = 4;
pub const VANILLA_ABSTRACT_ARROW_FLAG_CRIT: i8 = 1;
pub const VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS: i8 = 2;
pub const VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL: i8 = 1 << 0;
pub const VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS: i8 = 1 << 2;
pub const VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE: i8 = 1 << 3;
pub const VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER: i8 = 1 << 4;
pub const VANILLA_AVATAR_MODEL_PART_CAPE: i8 = 1 << 0;
pub const VANILLA_AVATAR_MODEL_PART_JACKET: i8 = 1 << 1;
pub const VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE: i8 = 1 << 2;
pub const VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE: i8 = 1 << 3;
pub const VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG: i8 = 1 << 4;
pub const VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG: i8 = 1 << 5;
pub const VANILLA_AVATAR_MODEL_PART_HAT: i8 = 1 << 6;

pub const VANILLA_HUMANOID_ARM_LEFT_ID: i32 = 0;
pub const VANILLA_HUMANOID_ARM_RIGHT_ID: i32 = 1;
pub const VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION: i8 = 0;
pub const VANILLA_SLIME_DEFAULT_SIZE: i32 = 1;
pub const VANILLA_SHULKER_DEFAULT_ATTACH_FACE_ID: i32 = 0;
pub const VANILLA_SHULKER_DEFAULT_PEEK: i8 = 0;
pub const VANILLA_SHULKER_DEFAULT_COLOR: i8 = 16;
pub const VANILLA_ZOMBIE_DEFAULT_BABY: bool = false;
pub const VANILLA_ZOMBIE_DEFAULT_SPECIAL_TYPE: i32 = 0;
pub const VANILLA_ZOMBIE_DEFAULT_DROWNED_CONVERSION: bool = false;
pub const VANILLA_AGEABLE_MOB_DEFAULT_BABY: bool = false;
pub const VANILLA_AGEABLE_MOB_DEFAULT_AGE_LOCKED: bool = false;
pub const VANILLA_ABSTRACT_HORSE_DEFAULT_FLAGS: i8 = 0;
pub const VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS: i8 = 0;
pub const VANILLA_ABSTRACT_ARROW_DEFAULT_FLAGS: i8 = 0;
pub const VANILLA_ABSTRACT_ARROW_DEFAULT_PIERCE_LEVEL: i8 = 0;
pub const VANILLA_ABSTRACT_ARROW_DEFAULT_IN_GROUND: bool = false;
pub const VANILLA_VEHICLE_DEFAULT_HURT_TIME: i32 = 0;
pub const VANILLA_VEHICLE_DEFAULT_HURT_DIR: i32 = 1;
pub const VANILLA_VEHICLE_DEFAULT_DAMAGE: f32 = 0.0;
pub const VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT: bool = false;
pub const VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT: bool = false;
pub const VANILLA_ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME: i32 = 0;
pub const VANILLA_ABSTRACT_MINECART_DEFAULT_DISPLAY_OFFSET: i32 = 6;
pub const VANILLA_CHEST_MINECART_DEFAULT_DISPLAY_OFFSET: i32 = 8;
pub const VANILLA_HOPPER_MINECART_DEFAULT_DISPLAY_OFFSET: i32 = 1;
pub const VANILLA_ARMOR_STAND_DEFAULT_CLIENT_FLAGS: i8 = 0;
pub const VANILLA_ARMOR_STAND_DEFAULT_HEAD_POSE: [f32; 3] = [0.0, 0.0, 0.0];
pub const VANILLA_ARMOR_STAND_DEFAULT_BODY_POSE: [f32; 3] = [0.0, 0.0, 0.0];
pub const VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE: [f32; 3] = [-10.0, 0.0, -10.0];
pub const VANILLA_ARMOR_STAND_DEFAULT_RIGHT_ARM_POSE: [f32; 3] = [-15.0, 0.0, 10.0];
pub const VANILLA_ARMOR_STAND_DEFAULT_LEFT_LEG_POSE: [f32; 3] = [-1.0, 0.0, -1.0];
pub const VANILLA_ARMOR_STAND_DEFAULT_RIGHT_LEG_POSE: [f32; 3] = [1.0, 0.0, 1.0];
pub const VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID: i32 = 3;
pub const VANILLA_ITEM_FRAME_DEFAULT_ROTATION: i32 = 0;
pub const VANILLA_PLAYER_DEFAULT_ABSORPTION: f32 = 0.0;
pub const VANILLA_PLAYER_DEFAULT_SCORE: i32 = 0;

/// Vanilla `Entity.getMaxAirSupply()` default. Individual entity classes can override this
/// virtual value, so the field schema records `MaxAirSupply` rather than fixing every entity to
/// this number.
pub const VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS: i32 = 300;

const ENTITY_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_SHARED_FLAGS_ID",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(0),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_AIR_SUPPLY_ID",
        data_id: VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::MaxAirSupply,
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_CUSTOM_NAME",
        data_id: VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
        default: VanillaEntityMetadataDefault::OptionalComponentNone,
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_CUSTOM_NAME_VISIBLE",
        data_id: VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(false),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_SILENT",
        data_id: VANILLA_ENTITY_SILENT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(false),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_NO_GRAVITY",
        data_id: VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(false),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_POSE",
        data_id: VANILLA_ENTITY_POSE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
        default: VanillaEntityMetadataDefault::Pose(VANILLA_ENTITY_POSE_STANDING_ID),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_TICKS_FROZEN",
        data_id: VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(0),
    },
];

const ENTITY_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_ONFIRE",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        mask: VANILLA_ENTITY_SHARED_FLAG_ON_FIRE,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_SHIFT_KEY_DOWN",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        mask: VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_SPRINTING",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        mask: VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_SWIMMING",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        mask: VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_INVISIBLE",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        mask: VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_GLOWING",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        mask: VANILLA_ENTITY_SHARED_FLAG_GLOWING,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_FALL_FLYING",
        data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        mask: VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
    },
];

const LIVING_ENTITY_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_LIVING_ENTITY_FLAGS",
        data_id: VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(0),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_HEALTH_ID",
        data_id: VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        default: VanillaEntityMetadataDefault::Float(1.0),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_EFFECT_PARTICLES",
        data_id: VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID,
        default: VanillaEntityMetadataDefault::ParticlesEmpty,
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_EFFECT_AMBIENCE_ID",
        data_id: VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(false),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ARROW_COUNT_ID",
        data_id: VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(0),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_STINGER_COUNT_ID",
        data_id: VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(0),
    },
    VanillaEntityMetadataFieldSchema {
        name: "SLEEPING_POS_ID",
        data_id: VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
        default: VanillaEntityMetadataDefault::OptionalBlockPosNone,
    },
];

const LIVING_ENTITY_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "LIVING_ENTITY_FLAG_IS_USING",
        data_id: VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
        mask: VANILLA_LIVING_ENTITY_FLAG_IS_USING,
    },
    VanillaEntityMetadataFlagSchema {
        name: "LIVING_ENTITY_FLAG_OFF_HAND",
        data_id: VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
        mask: VANILLA_LIVING_ENTITY_FLAG_OFF_HAND,
    },
    VanillaEntityMetadataFlagSchema {
        name: "LIVING_ENTITY_FLAG_SPIN_ATTACK",
        data_id: VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
        mask: VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK,
    },
];

const MOB_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[VanillaEntityMetadataFieldSchema {
    name: "DATA_MOB_FLAGS_ID",
    data_id: VANILLA_MOB_FLAGS_DATA_ID,
    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    default: VanillaEntityMetadataDefault::Byte(0),
}];

const MOB_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "MOB_FLAG_NO_AI",
        data_id: VANILLA_MOB_FLAGS_DATA_ID,
        mask: VANILLA_MOB_FLAG_NO_AI,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MOB_FLAG_LEFTHANDED",
        data_id: VANILLA_MOB_FLAGS_DATA_ID,
        mask: VANILLA_MOB_FLAG_LEFT_HANDED,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MOB_FLAG_AGGRESSIVE",
        data_id: VANILLA_MOB_FLAGS_DATA_ID,
        mask: VANILLA_MOB_FLAG_AGGRESSIVE,
    },
];

const SLIME_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[VanillaEntityMetadataFieldSchema {
    name: "ID_SIZE",
    data_id: VANILLA_SLIME_SIZE_DATA_ID,
    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    default: VanillaEntityMetadataDefault::Int(VANILLA_SLIME_DEFAULT_SIZE),
}];

const SHULKER_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ATTACH_FACE_ID",
        data_id: VANILLA_SHULKER_ATTACH_FACE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
        default: VanillaEntityMetadataDefault::Direction(VANILLA_SHULKER_DEFAULT_ATTACH_FACE_ID),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_PEEK_ID",
        data_id: VANILLA_SHULKER_PEEK_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_SHULKER_DEFAULT_PEEK),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_COLOR_ID",
        data_id: VANILLA_SHULKER_COLOR_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_SHULKER_DEFAULT_COLOR),
    },
];

const ZOMBIE_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_BABY_ID",
        data_id: VANILLA_ZOMBIE_BABY_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(VANILLA_ZOMBIE_DEFAULT_BABY),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_SPECIAL_TYPE_ID",
        data_id: VANILLA_ZOMBIE_SPECIAL_TYPE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(VANILLA_ZOMBIE_DEFAULT_SPECIAL_TYPE),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_DROWNED_CONVERSION_ID",
        data_id: VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(VANILLA_ZOMBIE_DEFAULT_DROWNED_CONVERSION),
    },
];

const AGEABLE_MOB_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_BABY_ID",
        data_id: VANILLA_AGEABLE_MOB_BABY_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(VANILLA_AGEABLE_MOB_DEFAULT_BABY),
    },
    VanillaEntityMetadataFieldSchema {
        name: "AGE_LOCKED",
        data_id: VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(VANILLA_AGEABLE_MOB_DEFAULT_AGE_LOCKED),
    },
];

const ABSTRACT_HORSE_FIELDS: &[VanillaEntityMetadataFieldSchema] =
    &[VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_FLAGS",
        data_id: VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_ABSTRACT_HORSE_DEFAULT_FLAGS),
    }];

const ABSTRACT_HORSE_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_TAME",
        data_id: VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
        mask: VANILLA_ABSTRACT_HORSE_FLAG_TAME,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_BRED",
        data_id: VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
        mask: VANILLA_ABSTRACT_HORSE_FLAG_BRED,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_EATING",
        data_id: VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
        mask: VANILLA_ABSTRACT_HORSE_FLAG_EATING,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_STANDING",
        data_id: VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
        mask: VANILLA_ABSTRACT_HORSE_FLAG_STANDING,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_OPEN_MOUTH",
        data_id: VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
        mask: VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH,
    },
];

const TAMABLE_ANIMAL_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_FLAGS_ID",
        data_id: VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_OWNERUUID_ID",
        data_id: VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
        default: VanillaEntityMetadataDefault::OptionalLivingEntityReferenceNone,
    },
];

const TAMABLE_ANIMAL_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_SITTING",
        data_id: VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
        mask: VANILLA_TAMABLE_ANIMAL_FLAG_SITTING,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_TAME",
        data_id: VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
        mask: VANILLA_TAMABLE_ANIMAL_FLAG_TAME,
    },
];

const ARMOR_STAND_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_CLIENT_FLAGS",
        data_id: VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_ARMOR_STAND_DEFAULT_CLIENT_FLAGS),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_HEAD_POSE",
        data_id: VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
        default: VanillaEntityMetadataDefault::Rotations(VANILLA_ARMOR_STAND_DEFAULT_HEAD_POSE),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_BODY_POSE",
        data_id: VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
        default: VanillaEntityMetadataDefault::Rotations(VANILLA_ARMOR_STAND_DEFAULT_BODY_POSE),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_LEFT_ARM_POSE",
        data_id: VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
        default: VanillaEntityMetadataDefault::Rotations(VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_RIGHT_ARM_POSE",
        data_id: VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
        default: VanillaEntityMetadataDefault::Rotations(
            VANILLA_ARMOR_STAND_DEFAULT_RIGHT_ARM_POSE,
        ),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_LEFT_LEG_POSE",
        data_id: VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
        default: VanillaEntityMetadataDefault::Rotations(VANILLA_ARMOR_STAND_DEFAULT_LEFT_LEG_POSE),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_RIGHT_LEG_POSE",
        data_id: VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
        default: VanillaEntityMetadataDefault::Rotations(
            VANILLA_ARMOR_STAND_DEFAULT_RIGHT_LEG_POSE,
        ),
    },
];

const ARMOR_STAND_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "CLIENT_FLAG_SMALL",
        data_id: VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
        mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL,
    },
    VanillaEntityMetadataFlagSchema {
        name: "CLIENT_FLAG_SHOW_ARMS",
        data_id: VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
        mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS,
    },
    VanillaEntityMetadataFlagSchema {
        name: "CLIENT_FLAG_NO_BASEPLATE",
        data_id: VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
        mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
    },
    VanillaEntityMetadataFlagSchema {
        name: "CLIENT_FLAG_MARKER",
        data_id: VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
        mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER,
    },
];

const ABSTRACT_ARROW_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "ID_FLAGS",
        data_id: VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_ABSTRACT_ARROW_DEFAULT_FLAGS),
    },
    VanillaEntityMetadataFieldSchema {
        name: "PIERCE_LEVEL",
        data_id: VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_ABSTRACT_ARROW_DEFAULT_PIERCE_LEVEL),
    },
    VanillaEntityMetadataFieldSchema {
        name: "IN_GROUND",
        data_id: VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(VANILLA_ABSTRACT_ARROW_DEFAULT_IN_GROUND),
    },
];

const ABSTRACT_ARROW_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_CRIT",
        data_id: VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID,
        mask: VANILLA_ABSTRACT_ARROW_FLAG_CRIT,
    },
    VanillaEntityMetadataFlagSchema {
        name: "FLAG_NOPHYSICS",
        data_id: VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID,
        mask: VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS,
    },
];

const VEHICLE_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_HURT",
        data_id: VANILLA_VEHICLE_HURT_TIME_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(VANILLA_VEHICLE_DEFAULT_HURT_TIME),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_HURTDIR",
        data_id: VANILLA_VEHICLE_HURT_DIR_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(VANILLA_VEHICLE_DEFAULT_HURT_DIR),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_DAMAGE",
        data_id: VANILLA_VEHICLE_DAMAGE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        default: VanillaEntityMetadataDefault::Float(VANILLA_VEHICLE_DEFAULT_DAMAGE),
    },
];

const ABSTRACT_BOAT_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_PADDLE_LEFT",
        data_id: VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_PADDLE_RIGHT",
        data_id: VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        default: VanillaEntityMetadataDefault::Boolean(VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_BUBBLE_TIME",
        data_id: VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(VANILLA_ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME),
    },
];

const ABSTRACT_MINECART_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_CUSTOM_DISPLAY_BLOCK",
        data_id: VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
        default: VanillaEntityMetadataDefault::OptionalBlockStateNone,
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ID_DISPLAY_OFFSET",
        data_id: VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::MinecartDefaultDisplayOffset,
    },
];

const HANGING_ENTITY_FIELDS: &[VanillaEntityMetadataFieldSchema] =
    &[VanillaEntityMetadataFieldSchema {
        name: "DATA_DIRECTION",
        data_id: VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
        default: VanillaEntityMetadataDefault::Direction(
            VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID,
        ),
    }];

const ITEM_FRAME_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ITEM",
        data_id: VANILLA_ITEM_FRAME_ITEM_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        default: VanillaEntityMetadataDefault::ItemStackEmpty,
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_ROTATION",
        data_id: VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(VANILLA_ITEM_FRAME_DEFAULT_ROTATION),
    },
];

const AVATAR_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_PLAYER_MAIN_HAND",
        data_id: VANILLA_AVATAR_MAIN_HAND_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
        default: VanillaEntityMetadataDefault::HumanoidArm(VANILLA_HUMANOID_ARM_RIGHT_ID),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_PLAYER_MODE_CUSTOMISATION",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        default: VanillaEntityMetadataDefault::Byte(VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION),
    },
];

const AVATAR_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[
    VanillaEntityMetadataFlagSchema {
        name: "MODEL_PART_CAPE",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        mask: VANILLA_AVATAR_MODEL_PART_CAPE,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MODEL_PART_JACKET",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        mask: VANILLA_AVATAR_MODEL_PART_JACKET,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MODEL_PART_LEFT_SLEEVE",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        mask: VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MODEL_PART_RIGHT_SLEEVE",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        mask: VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MODEL_PART_LEFT_PANTS_LEG",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        mask: VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MODEL_PART_RIGHT_PANTS_LEG",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        mask: VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG,
    },
    VanillaEntityMetadataFlagSchema {
        name: "MODEL_PART_HAT",
        data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        mask: VANILLA_AVATAR_MODEL_PART_HAT,
    },
];

const PLAYER_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[
    VanillaEntityMetadataFieldSchema {
        name: "DATA_PLAYER_ABSORPTION_ID",
        data_id: VANILLA_PLAYER_ABSORPTION_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        default: VanillaEntityMetadataDefault::Float(VANILLA_PLAYER_DEFAULT_ABSORPTION),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_SCORE_ID",
        data_id: VANILLA_PLAYER_SCORE_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        default: VanillaEntityMetadataDefault::Int(VANILLA_PLAYER_DEFAULT_SCORE),
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_SHOULDER_PARROT_LEFT",
        data_id: VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
        default: VanillaEntityMetadataDefault::OptionalUnsignedIntNone,
    },
    VanillaEntityMetadataFieldSchema {
        name: "DATA_SHOULDER_PARROT_RIGHT",
        data_id: VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
        default: VanillaEntityMetadataDefault::OptionalUnsignedIntNone,
    },
];

const ENTITY_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::Entity,
    parent: None,
    inherited_field_count: 0,
    own_fields: ENTITY_FIELDS,
    own_flags: ENTITY_FLAGS,
};

const LIVING_ENTITY_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::LivingEntity,
    parent: Some(VanillaEntityMetadataFamily::Entity),
    inherited_field_count: ENTITY_FIELDS.len() as u8,
    own_fields: LIVING_ENTITY_FIELDS,
    own_flags: LIVING_ENTITY_FLAGS,
};

const MOB_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::Mob,
    parent: Some(VanillaEntityMetadataFamily::LivingEntity),
    inherited_field_count: LIVING_ENTITY_SCHEMA.inherited_field_count
        + LIVING_ENTITY_FIELDS.len() as u8,
    own_fields: MOB_FIELDS,
    own_flags: MOB_FLAGS,
};

const SLIME_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::Slime,
    parent: Some(VanillaEntityMetadataFamily::Mob),
    inherited_field_count: MOB_SCHEMA.inherited_field_count + MOB_FIELDS.len() as u8,
    own_fields: SLIME_FIELDS,
    own_flags: &[],
};

const SHULKER_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::Shulker,
    parent: Some(VanillaEntityMetadataFamily::Mob),
    inherited_field_count: MOB_SCHEMA.inherited_field_count + MOB_FIELDS.len() as u8,
    own_fields: SHULKER_FIELDS,
    own_flags: &[],
};

const ZOMBIE_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::Zombie,
    parent: Some(VanillaEntityMetadataFamily::Mob),
    inherited_field_count: MOB_SCHEMA.inherited_field_count + MOB_FIELDS.len() as u8,
    own_fields: ZOMBIE_FIELDS,
    own_flags: &[],
};

const AGEABLE_MOB_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::AgeableMob,
    parent: Some(VanillaEntityMetadataFamily::Mob),
    inherited_field_count: MOB_SCHEMA.inherited_field_count + MOB_FIELDS.len() as u8,
    own_fields: AGEABLE_MOB_FIELDS,
    own_flags: &[],
};

const ABSTRACT_HORSE_SCHEMA: VanillaEntityMetadataFamilySchema =
    VanillaEntityMetadataFamilySchema {
        family: VanillaEntityMetadataFamily::AbstractHorse,
        parent: Some(VanillaEntityMetadataFamily::AgeableMob),
        inherited_field_count: AGEABLE_MOB_SCHEMA.inherited_field_count
            + AGEABLE_MOB_FIELDS.len() as u8,
        own_fields: ABSTRACT_HORSE_FIELDS,
        own_flags: ABSTRACT_HORSE_FLAGS,
    };

const TAMABLE_ANIMAL_SCHEMA: VanillaEntityMetadataFamilySchema =
    VanillaEntityMetadataFamilySchema {
        family: VanillaEntityMetadataFamily::TamableAnimal,
        parent: Some(VanillaEntityMetadataFamily::AgeableMob),
        inherited_field_count: AGEABLE_MOB_SCHEMA.inherited_field_count
            + AGEABLE_MOB_FIELDS.len() as u8,
        own_fields: TAMABLE_ANIMAL_FIELDS,
        own_flags: TAMABLE_ANIMAL_FLAGS,
    };

const ARMOR_STAND_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::ArmorStand,
    parent: Some(VanillaEntityMetadataFamily::LivingEntity),
    inherited_field_count: LIVING_ENTITY_SCHEMA.inherited_field_count
        + LIVING_ENTITY_FIELDS.len() as u8,
    own_fields: ARMOR_STAND_FIELDS,
    own_flags: ARMOR_STAND_FLAGS,
};

const ABSTRACT_ARROW_SCHEMA: VanillaEntityMetadataFamilySchema =
    VanillaEntityMetadataFamilySchema {
        family: VanillaEntityMetadataFamily::AbstractArrow,
        parent: Some(VanillaEntityMetadataFamily::Entity),
        inherited_field_count: ENTITY_FIELDS.len() as u8,
        own_fields: ABSTRACT_ARROW_FIELDS,
        own_flags: ABSTRACT_ARROW_FLAGS,
    };

const VEHICLE_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::VehicleEntity,
    parent: Some(VanillaEntityMetadataFamily::Entity),
    inherited_field_count: ENTITY_FIELDS.len() as u8,
    own_fields: VEHICLE_FIELDS,
    own_flags: &[],
};

const ABSTRACT_BOAT_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::AbstractBoat,
    parent: Some(VanillaEntityMetadataFamily::VehicleEntity),
    inherited_field_count: VEHICLE_SCHEMA.inherited_field_count + VEHICLE_FIELDS.len() as u8,
    own_fields: ABSTRACT_BOAT_FIELDS,
    own_flags: &[],
};

const ABSTRACT_MINECART_SCHEMA: VanillaEntityMetadataFamilySchema =
    VanillaEntityMetadataFamilySchema {
        family: VanillaEntityMetadataFamily::AbstractMinecart,
        parent: Some(VanillaEntityMetadataFamily::VehicleEntity),
        inherited_field_count: VEHICLE_SCHEMA.inherited_field_count + VEHICLE_FIELDS.len() as u8,
        own_fields: ABSTRACT_MINECART_FIELDS,
        own_flags: &[],
    };

const HANGING_ENTITY_SCHEMA: VanillaEntityMetadataFamilySchema =
    VanillaEntityMetadataFamilySchema {
        family: VanillaEntityMetadataFamily::HangingEntity,
        parent: Some(VanillaEntityMetadataFamily::Entity),
        inherited_field_count: ENTITY_FIELDS.len() as u8,
        own_fields: HANGING_ENTITY_FIELDS,
        own_flags: &[],
    };

const ITEM_FRAME_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::ItemFrame,
    parent: Some(VanillaEntityMetadataFamily::HangingEntity),
    inherited_field_count: HANGING_ENTITY_SCHEMA.inherited_field_count
        + HANGING_ENTITY_FIELDS.len() as u8,
    own_fields: ITEM_FRAME_FIELDS,
    own_flags: &[],
};

const AVATAR_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::Avatar,
    parent: Some(VanillaEntityMetadataFamily::LivingEntity),
    inherited_field_count: LIVING_ENTITY_SCHEMA.inherited_field_count
        + LIVING_ENTITY_FIELDS.len() as u8,
    own_fields: AVATAR_FIELDS,
    own_flags: AVATAR_FLAGS,
};

const PLAYER_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {
    family: VanillaEntityMetadataFamily::Player,
    parent: Some(VanillaEntityMetadataFamily::Avatar),
    inherited_field_count: AVATAR_SCHEMA.inherited_field_count + AVATAR_FIELDS.len() as u8,
    own_fields: PLAYER_FIELDS,
    own_flags: &[],
};

pub const fn vanilla_entity_metadata_schema(
    family: VanillaEntityMetadataFamily,
) -> &'static VanillaEntityMetadataFamilySchema {
    match family {
        VanillaEntityMetadataFamily::Entity => &ENTITY_SCHEMA,
        VanillaEntityMetadataFamily::LivingEntity => &LIVING_ENTITY_SCHEMA,
        VanillaEntityMetadataFamily::Mob => &MOB_SCHEMA,
        VanillaEntityMetadataFamily::Slime => &SLIME_SCHEMA,
        VanillaEntityMetadataFamily::Shulker => &SHULKER_SCHEMA,
        VanillaEntityMetadataFamily::Zombie => &ZOMBIE_SCHEMA,
        VanillaEntityMetadataFamily::AgeableMob => &AGEABLE_MOB_SCHEMA,
        VanillaEntityMetadataFamily::AbstractHorse => &ABSTRACT_HORSE_SCHEMA,
        VanillaEntityMetadataFamily::TamableAnimal => &TAMABLE_ANIMAL_SCHEMA,
        VanillaEntityMetadataFamily::ArmorStand => &ARMOR_STAND_SCHEMA,
        VanillaEntityMetadataFamily::AbstractArrow => &ABSTRACT_ARROW_SCHEMA,
        VanillaEntityMetadataFamily::VehicleEntity => &VEHICLE_SCHEMA,
        VanillaEntityMetadataFamily::AbstractBoat => &ABSTRACT_BOAT_SCHEMA,
        VanillaEntityMetadataFamily::AbstractMinecart => &ABSTRACT_MINECART_SCHEMA,
        VanillaEntityMetadataFamily::HangingEntity => &HANGING_ENTITY_SCHEMA,
        VanillaEntityMetadataFamily::ItemFrame => &ITEM_FRAME_SCHEMA,
        VanillaEntityMetadataFamily::Avatar => &AVATAR_SCHEMA,
        VanillaEntityMetadataFamily::Player => &PLAYER_SCHEMA,
    }
}

pub const fn vanilla_minecart_default_display_offset(entity_type_id: i32) -> i32 {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => VANILLA_CHEST_MINECART_DEFAULT_DISPLAY_OFFSET,
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => VANILLA_HOPPER_MINECART_DEFAULT_DISPLAY_OFFSET,
        _ => VANILLA_ABSTRACT_MINECART_DEFAULT_DISPLAY_OFFSET,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_family_fields_match_vanilla_define_id_order() {
        let entity = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Entity);
        assert_eq!(entity.inherited_field_count, 0);
        assert_eq!(
            entity
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (0..=7).collect::<Vec<_>>()
        );

        let living = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::LivingEntity);
        assert_eq!(living.parent, Some(VanillaEntityMetadataFamily::Entity));
        assert_eq!(living.inherited_field_count, 8);
        assert_eq!(
            living
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (8..=14).collect::<Vec<_>>()
        );

        let mob = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Mob);
        assert_eq!(mob.parent, Some(VanillaEntityMetadataFamily::LivingEntity));
        assert_eq!(mob.inherited_field_count, 15);
        assert_eq!(
            mob.own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            vec![15]
        );

        let slime = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Slime);
        assert_eq!(slime.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(slime.inherited_field_count, 16);
        assert_eq!(slime.own_fields[0].data_id, 16);

        let shulker = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Shulker);
        assert_eq!(shulker.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(shulker.inherited_field_count, 16);
        assert_eq!(
            shulker
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (16..=18).collect::<Vec<_>>()
        );

        let zombie = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Zombie);
        assert_eq!(zombie.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(zombie.inherited_field_count, 16);
        assert_eq!(
            zombie
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (16..=18).collect::<Vec<_>>()
        );

        let ageable = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AgeableMob);
        assert_eq!(ageable.parent, Some(VanillaEntityMetadataFamily::Mob));
        assert_eq!(ageable.inherited_field_count, 16);
        assert_eq!(
            ageable
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (16..=17).collect::<Vec<_>>()
        );

        let abstract_horse =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractHorse);
        assert_eq!(
            abstract_horse.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(abstract_horse.inherited_field_count, 18);
        assert_eq!(abstract_horse.own_fields[0].data_id, 18);

        let tamable = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::TamableAnimal);
        assert_eq!(
            tamable.parent,
            Some(VanillaEntityMetadataFamily::AgeableMob)
        );
        assert_eq!(tamable.inherited_field_count, 18);
        assert_eq!(
            tamable
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (18..=19).collect::<Vec<_>>()
        );

        let armor_stand = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ArmorStand);
        assert_eq!(
            armor_stand.parent,
            Some(VanillaEntityMetadataFamily::LivingEntity)
        );
        assert_eq!(armor_stand.inherited_field_count, 15);
        assert_eq!(
            armor_stand
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (15..=21).collect::<Vec<_>>()
        );

        let abstract_arrow =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractArrow);
        assert_eq!(
            abstract_arrow.parent,
            Some(VanillaEntityMetadataFamily::Entity)
        );
        assert_eq!(abstract_arrow.inherited_field_count, 8);
        assert_eq!(
            abstract_arrow
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (8..=10).collect::<Vec<_>>()
        );

        let vehicle = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::VehicleEntity);
        assert_eq!(vehicle.parent, Some(VanillaEntityMetadataFamily::Entity));
        assert_eq!(vehicle.inherited_field_count, 8);
        assert_eq!(
            vehicle
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (8..=10).collect::<Vec<_>>()
        );

        let abstract_boat =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractBoat);
        assert_eq!(
            abstract_boat.parent,
            Some(VanillaEntityMetadataFamily::VehicleEntity)
        );
        assert_eq!(abstract_boat.inherited_field_count, 11);
        assert_eq!(
            abstract_boat
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (11..=13).collect::<Vec<_>>()
        );

        let abstract_minecart =
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractMinecart);
        assert_eq!(
            abstract_minecart.parent,
            Some(VanillaEntityMetadataFamily::VehicleEntity)
        );
        assert_eq!(abstract_minecart.inherited_field_count, 11);
        assert_eq!(
            abstract_minecart
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (11..=12).collect::<Vec<_>>()
        );

        let hanging = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::HangingEntity);
        assert_eq!(hanging.parent, Some(VanillaEntityMetadataFamily::Entity));
        assert_eq!(hanging.inherited_field_count, 8);
        assert_eq!(hanging.own_fields[0].data_id, 8);

        let item_frame = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ItemFrame);
        assert_eq!(
            item_frame.parent,
            Some(VanillaEntityMetadataFamily::HangingEntity)
        );
        assert_eq!(item_frame.inherited_field_count, 9);
        assert_eq!(
            item_frame
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (9..=10).collect::<Vec<_>>()
        );

        let avatar = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Avatar);
        assert_eq!(
            avatar.parent,
            Some(VanillaEntityMetadataFamily::LivingEntity)
        );
        assert_eq!(avatar.inherited_field_count, 15);
        assert_eq!(
            avatar
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (15..=16).collect::<Vec<_>>()
        );

        let player = vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Player);
        assert_eq!(player.parent, Some(VanillaEntityMetadataFamily::Avatar));
        assert_eq!(player.inherited_field_count, 17);
        assert_eq!(
            player
                .own_fields
                .iter()
                .map(|field| field.data_id)
                .collect::<Vec<_>>(),
            (17..=20).collect::<Vec<_>>()
        );
    }

    #[test]
    fn schema_locks_serializer_and_default_semantics() {
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Entity).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_SHARED_FLAGS_ID",
                    data_id: 0,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_AIR_SUPPLY_ID",
                    data_id: 1,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::MaxAirSupply,
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_CUSTOM_NAME",
                    data_id: 2,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
                    default: VanillaEntityMetadataDefault::OptionalComponentNone,
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_CUSTOM_NAME_VISIBLE",
                    data_id: 3,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_SILENT",
                    data_id: 4,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_NO_GRAVITY",
                    data_id: 5,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_POSE",
                    data_id: 6,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                    default: VanillaEntityMetadataDefault::Pose(VANILLA_ENTITY_POSE_STANDING_ID,),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_TICKS_FROZEN",
                    data_id: 7,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::LivingEntity).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_LIVING_ENTITY_FLAGS",
                    data_id: 8,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_HEALTH_ID",
                    data_id: 9,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                    default: VanillaEntityMetadataDefault::Float(1.0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_EFFECT_PARTICLES",
                    data_id: 10,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID,
                    default: VanillaEntityMetadataDefault::ParticlesEmpty,
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_EFFECT_AMBIENCE_ID",
                    data_id: 11,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ARROW_COUNT_ID",
                    data_id: 12,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_STINGER_COUNT_ID",
                    data_id: 13,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "SLEEPING_POS_ID",
                    data_id: 14,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                    default: VanillaEntityMetadataDefault::OptionalBlockPosNone,
                },
            ]
        );
        assert_eq!(VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS, 300);
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Mob).own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_MOB_FLAGS_ID",
                data_id: 15,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                default: VanillaEntityMetadataDefault::Byte(0),
            }]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Slime).own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "ID_SIZE",
                data_id: 16,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                default: VanillaEntityMetadataDefault::Int(1),
            }]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Shulker).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ATTACH_FACE_ID",
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
                    default: VanillaEntityMetadataDefault::Direction(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_PEEK_ID",
                    data_id: 17,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_COLOR_ID",
                    data_id: 18,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(16),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Zombie).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_BABY_ID",
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_SPECIAL_TYPE_ID",
                    data_id: 17,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_DROWNED_CONVERSION_ID",
                    data_id: 18,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AgeableMob).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_BABY_ID",
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "AGE_LOCKED",
                    data_id: 17,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractHorse).own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_ID_FLAGS",
                data_id: 18,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                default: VanillaEntityMetadataDefault::Byte(0),
            }]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::TamableAnimal).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_FLAGS_ID",
                    data_id: 18,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_OWNERUUID_ID",
                    data_id: 19,
                    serializer_id:
                        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
                    default: VanillaEntityMetadataDefault::OptionalLivingEntityReferenceNone,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ArmorStand).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_CLIENT_FLAGS",
                    data_id: 15,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_HEAD_POSE",
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                    default: VanillaEntityMetadataDefault::Rotations([0.0, 0.0, 0.0]),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_BODY_POSE",
                    data_id: 17,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                    default: VanillaEntityMetadataDefault::Rotations([0.0, 0.0, 0.0]),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_LEFT_ARM_POSE",
                    data_id: 18,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                    default: VanillaEntityMetadataDefault::Rotations([-10.0, 0.0, -10.0]),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_RIGHT_ARM_POSE",
                    data_id: 19,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                    default: VanillaEntityMetadataDefault::Rotations([-15.0, 0.0, 10.0]),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_LEFT_LEG_POSE",
                    data_id: 20,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                    default: VanillaEntityMetadataDefault::Rotations([-1.0, 0.0, -1.0]),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_RIGHT_LEG_POSE",
                    data_id: 21,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                    default: VanillaEntityMetadataDefault::Rotations([1.0, 0.0, 1.0]),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractArrow).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "ID_FLAGS",
                    data_id: 8,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "PIERCE_LEVEL",
                    data_id: 9,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "IN_GROUND",
                    data_id: 10,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::VehicleEntity).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_HURT",
                    data_id: 8,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_HURTDIR",
                    data_id: 9,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(1),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_DAMAGE",
                    data_id: 10,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                    default: VanillaEntityMetadataDefault::Float(0.0),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractBoat).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_PADDLE_LEFT",
                    data_id: 11,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_PADDLE_RIGHT",
                    data_id: 12,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    default: VanillaEntityMetadataDefault::Boolean(false),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_BUBBLE_TIME",
                    data_id: 13,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractMinecart)
                .own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_CUSTOM_DISPLAY_BLOCK",
                    data_id: 11,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                    default: VanillaEntityMetadataDefault::OptionalBlockStateNone,
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ID_DISPLAY_OFFSET",
                    data_id: 12,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::MinecartDefaultDisplayOffset,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::HangingEntity).own_fields,
            &[VanillaEntityMetadataFieldSchema {
                name: "DATA_DIRECTION",
                data_id: 8,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
                default: VanillaEntityMetadataDefault::Direction(3),
            }]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ItemFrame).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ITEM",
                    data_id: 9,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    default: VanillaEntityMetadataDefault::ItemStackEmpty,
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_ROTATION",
                    data_id: 10,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Avatar).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_PLAYER_MAIN_HAND",
                    data_id: 15,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
                    default: VanillaEntityMetadataDefault::HumanoidArm(1),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_PLAYER_MODE_CUSTOMISATION",
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    default: VanillaEntityMetadataDefault::Byte(0),
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Player).own_fields,
            &[
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_PLAYER_ABSORPTION_ID",
                    data_id: 17,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                    default: VanillaEntityMetadataDefault::Float(0.0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_SCORE_ID",
                    data_id: 18,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    default: VanillaEntityMetadataDefault::Int(0),
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_SHOULDER_PARROT_LEFT",
                    data_id: 19,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
                    default: VanillaEntityMetadataDefault::OptionalUnsignedIntNone,
                },
                VanillaEntityMetadataFieldSchema {
                    name: "DATA_SHOULDER_PARROT_RIGHT",
                    data_id: 20,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
                    default: VanillaEntityMetadataDefault::OptionalUnsignedIntNone,
                },
            ]
        );
    }

    #[test]
    fn family_queries_include_parent_fields_and_flags_are_unique() {
        assert_eq!(
            vanilla_entity_metadata_field(
                VanillaEntityMetadataFamily::LivingEntity,
                VANILLA_ENTITY_POSE_DATA_ID
            )
            .map(|field| field.name),
            Some("DATA_POSE")
        );
        assert!(vanilla_entity_metadata_field(
            VanillaEntityMetadataFamily::Entity,
            VANILLA_LIVING_ENTITY_FLAGS_DATA_ID
        )
        .is_none());
        assert_eq!(
            vanilla_entity_metadata_field(
                VanillaEntityMetadataFamily::Mob,
                VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
            )
            .map(|field| field.name),
            Some("DATA_HEALTH_ID")
        );
        assert_eq!(
            vanilla_entity_metadata_field(
                VanillaEntityMetadataFamily::Player,
                VANILLA_AVATAR_MAIN_HAND_DATA_ID
            )
            .map(|field| field.name),
            Some("DATA_PLAYER_MAIN_HAND")
        );
        assert!(vanilla_entity_metadata_field(
            VanillaEntityMetadataFamily::Mob,
            VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID
        )
        .is_none());

        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Entity).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_ONFIRE",
                    data_id: 0,
                    mask: 1,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_SHIFT_KEY_DOWN",
                    data_id: 0,
                    mask: 2,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_SPRINTING",
                    data_id: 0,
                    mask: 8,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_SWIMMING",
                    data_id: 0,
                    mask: 16,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_INVISIBLE",
                    data_id: 0,
                    mask: 32,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_GLOWING",
                    data_id: 0,
                    mask: 64,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_FALL_FLYING",
                    data_id: 0,
                    mask: 0x80_u8 as i8,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::LivingEntity).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "LIVING_ENTITY_FLAG_IS_USING",
                    data_id: 8,
                    mask: 1,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "LIVING_ENTITY_FLAG_OFF_HAND",
                    data_id: 8,
                    mask: 2,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "LIVING_ENTITY_FLAG_SPIN_ATTACK",
                    data_id: 8,
                    mask: 4,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Mob).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "MOB_FLAG_NO_AI",
                    data_id: 15,
                    mask: 1,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MOB_FLAG_LEFTHANDED",
                    data_id: 15,
                    mask: 2,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MOB_FLAG_AGGRESSIVE",
                    data_id: 15,
                    mask: 4,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractHorse).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_TAME",
                    data_id: 18,
                    mask: 2,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_BRED",
                    data_id: 18,
                    mask: 8,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_EATING",
                    data_id: 18,
                    mask: 16,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_STANDING",
                    data_id: 18,
                    mask: 32,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_OPEN_MOUTH",
                    data_id: 18,
                    mask: 64,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::TamableAnimal).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_SITTING",
                    data_id: 18,
                    mask: 1,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_TAME",
                    data_id: 18,
                    mask: 4,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::AbstractArrow).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_CRIT",
                    data_id: 8,
                    mask: 1,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "FLAG_NOPHYSICS",
                    data_id: 8,
                    mask: 2,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::Avatar).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "MODEL_PART_CAPE",
                    data_id: 16,
                    mask: 1,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MODEL_PART_JACKET",
                    data_id: 16,
                    mask: 2,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MODEL_PART_LEFT_SLEEVE",
                    data_id: 16,
                    mask: 4,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MODEL_PART_RIGHT_SLEEVE",
                    data_id: 16,
                    mask: 8,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MODEL_PART_LEFT_PANTS_LEG",
                    data_id: 16,
                    mask: 16,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MODEL_PART_RIGHT_PANTS_LEG",
                    data_id: 16,
                    mask: 32,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "MODEL_PART_HAT",
                    data_id: 16,
                    mask: 64,
                },
            ]
        );
        assert_eq!(
            vanilla_entity_metadata_schema(VanillaEntityMetadataFamily::ArmorStand).own_flags,
            &[
                VanillaEntityMetadataFlagSchema {
                    name: "CLIENT_FLAG_SMALL",
                    data_id: 15,
                    mask: 1,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "CLIENT_FLAG_SHOW_ARMS",
                    data_id: 15,
                    mask: 4,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "CLIENT_FLAG_NO_BASEPLATE",
                    data_id: 15,
                    mask: 8,
                },
                VanillaEntityMetadataFlagSchema {
                    name: "CLIENT_FLAG_MARKER",
                    data_id: 15,
                    mask: 16,
                },
            ]
        );

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
    }

    #[test]
    fn minecart_display_offset_preserves_virtual_defaults() {
        assert_eq!(
            vanilla_minecart_default_display_offset(VANILLA_ENTITY_TYPE_CHEST_MINECART_ID),
            VANILLA_CHEST_MINECART_DEFAULT_DISPLAY_OFFSET
        );
        assert_eq!(
            vanilla_minecart_default_display_offset(VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID),
            VANILLA_HOPPER_MINECART_DEFAULT_DISPLAY_OFFSET
        );
        assert_eq!(
            vanilla_minecart_default_display_offset(-1),
            VANILLA_ABSTRACT_MINECART_DEFAULT_DISPLAY_OFFSET
        );
    }
}
