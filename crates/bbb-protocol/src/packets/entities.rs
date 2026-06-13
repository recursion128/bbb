use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
};

use super::{
    chunks::decode_block_pos,
    decode_vec3d,
    inventory::{self, ItemStackSummary},
    read_resource_key,
    world_effects::{decode_particle_payload, ParticlePayload},
    BlockPos, Vec3d,
};

const MAX_ENTITY_ATTRIBUTES: usize = 1024;
const MAX_ENTITY_ID_LIST: usize = 8192;
const MAX_EQUIPMENT_SLOTS: usize = 8;
const MAX_ATTRIBUTE_MODIFIERS: usize = 1024;
const MAX_ENTITY_DATA_PARTICLES: usize = 65_536;
// Vanilla uses ByteBufCodecs.list() without an explicit wire cap. Keep a repo-side
// allocation guard so malformed packets cannot request unbounded step storage.
const MAX_MINECART_LERP_STEPS: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddEntity {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub x_rot: f32,
    pub y_rot: f32,
    pub y_head_rot: f32,
    pub data: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityAnimation {
    pub id: i32,
    pub action: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityEvent {
    pub entity_id: i32,
    pub event_id: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HurtAnimation {
    pub id: i32,
    pub yaw: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPositionSync {
    pub id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityMove {
    pub id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub y_rot: Option<f32>,
    pub x_rot: Option<f32>,
    pub on_ground: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoveVehicle {
    pub position: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveMinecartAlongTrack {
    pub entity_id: i32,
    pub lerp_steps: Vec<MinecartStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MinecartStep {
    pub position: Vec3d,
    pub movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveEntities {
    pub entity_ids: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeItemEntity {
    pub item_id: i32,
    pub player_id: i32,
    pub amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RotateHead {
    pub id: i32,
    pub y_head_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetEntityMotion {
    pub id: i32,
    pub delta_movement: Vec3d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetEntityLink {
    pub source_id: i32,
    pub dest_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetEquipment {
    pub entity_id: i32,
    pub slots: Vec<EquipmentSlotUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPassengers {
    pub vehicle_id: i32,
    pub passenger_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentSlotUpdate {
    pub slot: EquipmentSlot,
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateAttributes {
    pub entity_id: i32,
    pub attributes: Vec<AttributeSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeSnapshot {
    pub attribute_id: i32,
    pub base: f64,
    pub modifiers: Vec<AttributeModifier>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeModifier {
    pub id: String,
    pub amount: f64,
    pub operation_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Feet,
    Legs,
    Chest,
    Head,
    Body,
    Saddle,
}

impl EquipmentSlot {
    pub fn ordinal(self) -> u8 {
        match self {
            Self::MainHand => 0,
            Self::OffHand => 1,
            Self::Feet => 2,
            Self::Legs => 3,
            Self::Chest => 4,
            Self::Head => 5,
            Self::Body => 6,
            Self::Saddle => 7,
        }
    }

    fn from_ordinal(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::MainHand,
            1 => Self::OffHand,
            2 => Self::Feet,
            3 => Self::Legs,
            4 => Self::Chest,
            5 => Self::Head,
            6 => Self::Body,
            7 => Self::Saddle,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid equipment slot {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetEntityData {
    pub id: i32,
    pub values: Vec<EntityDataValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDataValue {
    pub data_id: u8,
    pub serializer_id: i32,
    pub value: EntityDataValueKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityDataValueKind {
    Byte(i8),
    Int(i32),
    Long(i64),
    Float(f32),
    String(String),
    Component(String),
    OptionalComponent(Option<String>),
    ItemStack(ItemStackSummary),
    Boolean(bool),
    Rotations {
        x: f32,
        y: f32,
        z: f32,
    },
    BlockPos(BlockPos),
    OptionalBlockPos(Option<BlockPos>),
    Direction(i32),
    OptionalLivingEntityReference(Option<Uuid>),
    BlockState(i32),
    OptionalBlockState(Option<i32>),
    Particle(ParticlePayload),
    Particles(Vec<ParticlePayload>),
    RegistryId {
        serializer: EntityDataRegistryHolder,
        id: i32,
    },
    EnumId {
        serializer: EntityDataEnumSerializer,
        id: i32,
    },
    VillagerData {
        villager_type: i32,
        profession: i32,
        level: i32,
    },
    OptionalUnsignedInt(Option<i32>),
    Pose(i32),
    OptionalGlobalPos(Option<GlobalPosData>),
    Vector3f {
        x: f32,
        y: f32,
        z: f32,
    },
    Quaternionf {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    },
    HumanoidArm(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityDataRegistryHolder {
    CatVariant,
    CatSoundVariant,
    CowVariant,
    CowSoundVariant,
    WolfVariant,
    WolfSoundVariant,
    FrogVariant,
    PigVariant,
    PigSoundVariant,
    ChickenVariant,
    ChickenSoundVariant,
    ZombieNautilusVariant,
    PaintingVariant,
}

impl EntityDataRegistryHolder {
    fn from_serializer_id(serializer_id: i32) -> Option<Self> {
        Some(match serializer_id {
            21 => Self::CatVariant,
            22 => Self::CatSoundVariant,
            23 => Self::CowVariant,
            24 => Self::CowSoundVariant,
            25 => Self::WolfVariant,
            26 => Self::WolfSoundVariant,
            27 => Self::FrogVariant,
            28 => Self::PigVariant,
            29 => Self::PigSoundVariant,
            30 => Self::ChickenVariant,
            31 => Self::ChickenSoundVariant,
            32 => Self::ZombieNautilusVariant,
            34 => Self::PaintingVariant,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityDataEnumSerializer {
    SnifferState,
    ArmadilloState,
    CopperGolemState,
    WeatheringCopperState,
}

impl EntityDataEnumSerializer {
    fn from_serializer_id(serializer_id: i32) -> Option<Self> {
        Some(match serializer_id {
            35 => Self::SnifferState,
            36 => Self::ArmadilloState,
            37 => Self::CopperGolemState,
            38 => Self::WeatheringCopperState,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalPosData {
    pub dimension: String,
    pub pos: BlockPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TeleportEntity {
    pub id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub relatives_mask: i32,
    pub on_ground: bool,
}

pub(super) fn decode_add_entity(decoder: &mut Decoder<'_>) -> Result<AddEntity> {
    Ok(AddEntity {
        id: decoder.read_var_i32()?,
        uuid: decoder.read_uuid()?,
        entity_type_id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_lp_vec3d(decoder)?,
        x_rot: unpack_degrees(decoder.read_i8()?),
        y_rot: unpack_degrees(decoder.read_i8()?),
        y_head_rot: unpack_degrees(decoder.read_i8()?),
        data: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_entity_animation(decoder: &mut Decoder<'_>) -> Result<EntityAnimation> {
    Ok(EntityAnimation {
        id: decoder.read_var_i32()?,
        action: decoder.read_u8()?,
    })
}

pub(super) fn decode_entity_event(decoder: &mut Decoder<'_>) -> Result<EntityEvent> {
    Ok(EntityEvent {
        entity_id: decoder.read_i32()?,
        event_id: decoder.read_i8()?,
    })
}

pub(super) fn decode_hurt_animation(decoder: &mut Decoder<'_>) -> Result<HurtAnimation> {
    Ok(HurtAnimation {
        id: decoder.read_var_i32()?,
        yaw: decoder.read_f32()?,
    })
}

pub(super) fn decode_entity_position_sync(decoder: &mut Decoder<'_>) -> Result<EntityPositionSync> {
    Ok(EntityPositionSync {
        id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
        on_ground: decoder.read_bool()?,
    })
}

pub(super) fn decode_move_entity(
    decoder: &mut Decoder<'_>,
    has_position: bool,
    has_rotation: bool,
) -> Result<EntityMove> {
    let id = decoder.read_var_i32()?;
    let (delta_x, delta_y, delta_z) = if has_position {
        (
            decoder.read_i16()?,
            decoder.read_i16()?,
            decoder.read_i16()?,
        )
    } else {
        (0, 0, 0)
    };
    let (y_rot, x_rot) = if has_rotation {
        (
            Some(unpack_degrees(decoder.read_i8()?)),
            Some(unpack_degrees(decoder.read_i8()?)),
        )
    } else {
        (None, None)
    };

    Ok(EntityMove {
        id,
        delta_x,
        delta_y,
        delta_z,
        y_rot,
        x_rot,
        on_ground: decoder.read_bool()?,
    })
}

pub(super) fn decode_move_vehicle(decoder: &mut Decoder<'_>) -> Result<MoveVehicle> {
    Ok(MoveVehicle {
        position: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
    })
}

pub(super) fn decode_move_minecart_along_track(
    decoder: &mut Decoder<'_>,
) -> Result<MoveMinecartAlongTrack> {
    let entity_id = decoder.read_var_i32()?;
    let step_count = decoder.read_len()?;
    if step_count > MAX_MINECART_LERP_STEPS {
        return Err(ProtocolError::PacketTooLarge(
            step_count,
            MAX_MINECART_LERP_STEPS,
        ));
    }

    let mut lerp_steps = Vec::with_capacity(step_count);
    for _ in 0..step_count {
        lerp_steps.push(MinecartStep {
            position: decode_vec3d(decoder)?,
            movement: decode_vec3d(decoder)?,
            y_rot: unpack_degrees(decoder.read_i8()?),
            x_rot: unpack_degrees(decoder.read_i8()?),
            weight: decoder.read_f32()?,
        });
    }

    Ok(MoveMinecartAlongTrack {
        entity_id,
        lerp_steps,
    })
}

pub(super) fn decode_remove_entities(decoder: &mut Decoder<'_>) -> Result<RemoveEntities> {
    let count = decoder.read_len()?;
    if count > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(count, MAX_ENTITY_ID_LIST));
    }
    let mut entity_ids = Vec::with_capacity(count);
    for _ in 0..count {
        entity_ids.push(decoder.read_var_i32()?);
    }
    Ok(RemoveEntities { entity_ids })
}

pub(super) fn decode_take_item_entity(decoder: &mut Decoder<'_>) -> Result<TakeItemEntity> {
    Ok(TakeItemEntity {
        item_id: decoder.read_var_i32()?,
        player_id: decoder.read_var_i32()?,
        amount: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_rotate_head(decoder: &mut Decoder<'_>) -> Result<RotateHead> {
    Ok(RotateHead {
        id: decoder.read_var_i32()?,
        y_head_rot: unpack_degrees(decoder.read_i8()?),
    })
}

pub(super) fn decode_set_entity_motion(decoder: &mut Decoder<'_>) -> Result<SetEntityMotion> {
    Ok(SetEntityMotion {
        id: decoder.read_var_i32()?,
        delta_movement: decode_lp_vec3d(decoder)?,
    })
}

pub(super) fn decode_set_entity_link(decoder: &mut Decoder<'_>) -> Result<SetEntityLink> {
    Ok(SetEntityLink {
        source_id: decoder.read_i32()?,
        dest_id: decoder.read_i32()?,
    })
}

pub(super) fn decode_set_passengers(decoder: &mut Decoder<'_>) -> Result<SetPassengers> {
    let vehicle_id = decoder.read_var_i32()?;
    let count = decoder.read_len()?;
    if count > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(count, MAX_ENTITY_ID_LIST));
    }
    let mut passenger_ids = Vec::with_capacity(count);
    for _ in 0..count {
        passenger_ids.push(decoder.read_var_i32()?);
    }
    Ok(SetPassengers {
        vehicle_id,
        passenger_ids,
    })
}

pub(super) fn decode_teleport_entity(decoder: &mut Decoder<'_>) -> Result<TeleportEntity> {
    Ok(TeleportEntity {
        id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
        relatives_mask: decoder.read_i32()?,
        on_ground: decoder.read_bool()?,
    })
}

pub(super) fn decode_set_equipment(decoder: &mut Decoder<'_>) -> Result<SetEquipment> {
    let entity_id = decoder.read_var_i32()?;
    let mut slots = Vec::new();
    loop {
        if slots.len() >= MAX_EQUIPMENT_SLOTS {
            return Err(ProtocolError::PacketTooLarge(
                slots.len() + 1,
                MAX_EQUIPMENT_SLOTS,
            ));
        }

        let raw_slot = decoder.read_u8()?;
        let should_continue = raw_slot & 0x80 != 0;
        let slot = EquipmentSlot::from_ordinal(raw_slot & 0x7f)?;
        let item = inventory::decode_item_stack_summary(decoder)?;
        slots.push(EquipmentSlotUpdate { slot, item });

        if !should_continue {
            break;
        }
    }
    Ok(SetEquipment { entity_id, slots })
}

pub(super) fn decode_set_entity_data(decoder: &mut Decoder<'_>) -> Result<SetEntityData> {
    let id = decoder.read_var_i32()?;
    let mut values = Vec::new();
    loop {
        let data_id = decoder.read_u8()?;
        if data_id == 0xff {
            break;
        }
        let serializer_id = decoder.read_var_i32()?;
        values.push(EntityDataValue {
            data_id,
            serializer_id,
            value: decode_entity_data_value(decoder, serializer_id)?,
        });
    }
    Ok(SetEntityData { id, values })
}

fn decode_entity_data_value(
    decoder: &mut Decoder<'_>,
    serializer_id: i32,
) -> Result<EntityDataValueKind> {
    Ok(match serializer_id {
        0 => EntityDataValueKind::Byte(decoder.read_i8()?),
        1 => EntityDataValueKind::Int(decoder.read_var_i32()?),
        2 => EntityDataValueKind::Long(decoder.read_var_i64()?),
        3 => EntityDataValueKind::Float(decoder.read_f32()?),
        4 => EntityDataValueKind::String(decoder.read_string(32767)?),
        5 => EntityDataValueKind::Component(decode_component_summary_from_decoder(decoder)?),
        6 => EntityDataValueKind::OptionalComponent(if decoder.read_bool()? {
            Some(decode_component_summary_from_decoder(decoder)?)
        } else {
            None
        }),
        7 => EntityDataValueKind::ItemStack(inventory::decode_item_stack_summary(decoder)?),
        8 => EntityDataValueKind::Boolean(decoder.read_bool()?),
        9 => EntityDataValueKind::Rotations {
            x: decoder.read_f32()?,
            y: decoder.read_f32()?,
            z: decoder.read_f32()?,
        },
        10 => EntityDataValueKind::BlockPos(decode_block_pos(decoder.read_i64()?)),
        11 => EntityDataValueKind::OptionalBlockPos(if decoder.read_bool()? {
            Some(decode_block_pos(decoder.read_i64()?))
        } else {
            None
        }),
        12 => EntityDataValueKind::Direction(decoder.read_var_i32()?),
        13 => EntityDataValueKind::OptionalLivingEntityReference(if decoder.read_bool()? {
            Some(decoder.read_uuid()?)
        } else {
            None
        }),
        14 => EntityDataValueKind::BlockState(decoder.read_var_i32()?),
        15 => {
            let id = decoder.read_var_i32()?;
            EntityDataValueKind::OptionalBlockState((id != 0).then_some(id))
        }
        16 => EntityDataValueKind::Particle(decode_particle_payload(decoder)?),
        17 => {
            let count = decoder.read_len()?;
            if count > MAX_ENTITY_DATA_PARTICLES {
                return Err(ProtocolError::PacketTooLarge(
                    count,
                    MAX_ENTITY_DATA_PARTICLES,
                ));
            }
            let mut particles = Vec::with_capacity(count);
            for _ in 0..count {
                particles.push(decode_particle_payload(decoder)?);
            }
            EntityDataValueKind::Particles(particles)
        }
        18 => EntityDataValueKind::VillagerData {
            villager_type: decoder.read_var_i32()?,
            profession: decoder.read_var_i32()?,
            level: decoder.read_var_i32()?,
        },
        19 => {
            let value = decoder.read_var_i32()?;
            EntityDataValueKind::OptionalUnsignedInt((value != 0).then_some(value - 1))
        }
        20 => EntityDataValueKind::Pose(decoder.read_var_i32()?),
        33 => EntityDataValueKind::OptionalGlobalPos(if decoder.read_bool()? {
            Some(GlobalPosData {
                dimension: read_resource_key(decoder)?,
                pos: decode_block_pos(decoder.read_i64()?),
            })
        } else {
            None
        }),
        39 => EntityDataValueKind::Vector3f {
            x: decoder.read_f32()?,
            y: decoder.read_f32()?,
            z: decoder.read_f32()?,
        },
        40 => EntityDataValueKind::Quaternionf {
            x: decoder.read_f32()?,
            y: decoder.read_f32()?,
            z: decoder.read_f32()?,
            w: decoder.read_f32()?,
        },
        42 => EntityDataValueKind::HumanoidArm(decoder.read_var_i32()?),
        other if let Some(serializer) = EntityDataRegistryHolder::from_serializer_id(other) => {
            EntityDataValueKind::RegistryId {
                serializer,
                id: decoder.read_var_i32()?,
            }
        }
        other if let Some(serializer) = EntityDataEnumSerializer::from_serializer_id(other) => {
            EntityDataValueKind::EnumId {
                serializer,
                id: decoder.read_var_i32()?,
            }
        }
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "unsupported entity data serializer {other}"
            )))
        }
    })
}

pub(super) fn decode_update_attributes(decoder: &mut Decoder<'_>) -> Result<UpdateAttributes> {
    let entity_id = decoder.read_var_i32()?;
    let attribute_count = decoder.read_len()?;
    if attribute_count > MAX_ENTITY_ATTRIBUTES {
        return Err(ProtocolError::InvalidData(format!(
            "attribute list has {attribute_count} entries, max is {MAX_ENTITY_ATTRIBUTES}"
        )));
    }
    let mut attributes = Vec::with_capacity(attribute_count);
    for _ in 0..attribute_count {
        let attribute_id = decoder.read_var_i32()?;
        let base = decoder.read_f64()?;
        let modifier_count = decoder.read_len()?;
        if modifier_count > MAX_ATTRIBUTE_MODIFIERS {
            return Err(ProtocolError::InvalidData(format!(
                "attribute modifier list has {modifier_count} entries, max is {MAX_ATTRIBUTE_MODIFIERS}"
            )));
        }
        let mut modifiers = Vec::with_capacity(modifier_count);
        for _ in 0..modifier_count {
            modifiers.push(AttributeModifier {
                id: read_resource_key(decoder)?,
                amount: decoder.read_f64()?,
                operation_id: decoder.read_var_i32()?,
            });
        }
        attributes.push(AttributeSnapshot {
            attribute_id,
            base,
            modifiers,
        });
    }
    Ok(UpdateAttributes {
        entity_id,
        attributes,
    })
}

fn decode_lp_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    let lowest = decoder.read_u8()?;
    if lowest == 0 {
        return Ok(Vec3d::default());
    }
    let middle = decoder.read_u8()? as u64;
    let highest = u32::from_be_bytes(
        decoder
            .read_exact(4, "lp vec3 highest")?
            .try_into()
            .expect("fixed length"),
    ) as u64;
    let buffer = (highest << 16) | (middle << 8) | u64::from(lowest);
    let mut scale = u64::from(lowest & 0x03);
    if lowest & 0x04 != 0 {
        scale |= u64::from(decoder.read_var_i32()? as u32) << 2;
    }
    let scale = scale as f64;
    Ok(Vec3d {
        x: unpack_lp_vec_component(buffer >> 3) * scale,
        y: unpack_lp_vec_component(buffer >> 18) * scale,
        z: unpack_lp_vec_component(buffer >> 33) * scale,
    })
}

fn unpack_lp_vec_component(value: u64) -> f64 {
    ((value & 0x7fff).min(32766) as f64) * 2.0 / 32766.0 - 1.0
}

fn unpack_degrees(value: i8) -> f32 {
    f32::from(value) * 360.0 / 256.0
}

#[cfg(test)]
mod tests;
