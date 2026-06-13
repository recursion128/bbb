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
    read_resource_key, BlockPos, Vec3d,
};

const MAX_ENTITY_ATTRIBUTES: usize = 1024;
const MAX_ENTITY_ID_LIST: usize = 8192;
const MAX_EQUIPMENT_SLOTS: usize = 8;
const MAX_ATTRIBUTE_MODIFIERS: usize = 1024;

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
    BlockState(i32),
    OptionalBlockState(Option<i32>),
    VillagerData {
        villager_type: i32,
        profession: i32,
        level: i32,
    },
    OptionalUnsignedInt(Option<i32>),
    Pose(i32),
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
        14 => EntityDataValueKind::BlockState(decoder.read_var_i32()?),
        15 => {
            let id = decoder.read_var_i32()?;
            EntityDataValueKind::OptionalBlockState((id != 0).then_some(id))
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
mod tests {
    use super::*;
    use crate::{
        codec::Encoder,
        ids,
        packets::{
            decode_play_clientbound, DataComponentPatchSummary, PlayClientbound,
            PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_X,
        },
    };

    #[test]
    fn decodes_entity_lifecycle_packets() {
        let uuid = Uuid::from_u128(0x12345678123456781234567812345678);
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_uuid(uuid);
        payload.write_var_i32(7);
        payload.write_f64(1.0);
        payload.write_f64(64.0);
        payload.write_f64(-2.0);
        payload.write_bytes(&lp_vec3_axis_x());
        payload.write_i8(-64);
        payload.write_i8(64);
        payload.write_i8(32);
        payload.write_var_i32(99);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ADD_ENTITY, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::AddEntity(AddEntity {
                id: 123,
                uuid,
                entity_type_id: 7,
                position: Vec3d {
                    x: 1.0,
                    y: 64.0,
                    z: -2.0,
                },
                delta_movement: Vec3d {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                x_rot: -90.0,
                y_rot: 90.0,
                y_head_rot: 45.0,
                data: 99,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_f64(2.0);
        payload.write_f64(65.0);
        payload.write_f64(-3.0);
        payload.write_f64(0.0);
        payload.write_f64(0.25);
        payload.write_f64(0.0);
        payload.write_f32(180.0);
        payload.write_f32(30.0);
        payload.write_bool(true);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::EntityPositionSync(EntityPositionSync {
                id: 123,
                position: Vec3d {
                    x: 2.0,
                    y: 65.0,
                    z: -3.0,
                },
                delta_movement: Vec3d {
                    x: 0.0,
                    y: 0.25,
                    z: 0.0,
                },
                y_rot: 180.0,
                x_rot: 30.0,
                on_ground: true,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_i16(4096);
        payload.write_i16(0);
        payload.write_i16(-2048);
        payload.write_i8(-64);
        payload.write_i8(64);
        payload.write_bool(false);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::MoveEntity(EntityMove {
                id: 123,
                delta_x: 4096,
                delta_y: 0,
                delta_z: -2048,
                y_rot: Some(-90.0),
                x_rot: Some(90.0),
                on_ground: false,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_bytes(&[0]);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ENTITY_MOTION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEntityMotion(SetEntityMotion {
                id: 123,
                delta_movement: Vec3d::default(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_f64(0.5);
        payload.write_f64(1.0);
        payload.write_f64(-0.5);
        payload.write_f64(0.0);
        payload.write_f64(0.1);
        payload.write_f64(0.0);
        payload.write_f32(15.0);
        payload.write_f32(-5.0);
        payload.write_i32(PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y);
        payload.write_bool(true);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_TELEPORT_ENTITY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TeleportEntity(TeleportEntity {
                id: 123,
                position: Vec3d {
                    x: 0.5,
                    y: 1.0,
                    z: -0.5,
                },
                delta_movement: Vec3d {
                    x: 0.0,
                    y: 0.1,
                    z: 0.0,
                },
                y_rot: 15.0,
                x_rot: -5.0,
                relatives_mask: PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y,
                on_ground: true,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_i8(64);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ROTATE_HEAD, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::RotateHead(RotateHead {
                id: 123,
                y_head_rot: 90.0,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(123);
        payload.write_var_i32(456);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_REMOVE_ENTITIES,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::RemoveEntities(RemoveEntities {
                entity_ids: vec![123, 456],
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(321);
        payload.write_var_i32(654);
        payload.write_var_i32(3);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: 321,
                player_id: 654,
                amount: 3,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(0);
        payload.write_var_i32(0);
        payload.write_i8(0x20);
        payload.write_u8(2);
        payload.write_var_i32(1);
        payload.write_var_i32(300);
        payload.write_u8(3);
        payload.write_var_i32(5);
        payload.write_bytes(&nbt_string_root("Name"));
        payload.write_u8(4);
        payload.write_var_i32(6);
        payload.write_bool(false);
        payload.write_u8(5);
        payload.write_var_i32(9);
        payload.write_f32(1.0);
        payload.write_f32(2.0);
        payload.write_f32(3.0);
        payload.write_u8(8);
        payload.write_var_i32(7);
        payload.write_var_i32(3);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_u8(0xff);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ENTITY_DATA,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEntityData(SetEntityData {
                id: 123,
                values: vec![
                    EntityDataValue {
                        data_id: 0,
                        serializer_id: 0,
                        value: EntityDataValueKind::Byte(0x20),
                    },
                    EntityDataValue {
                        data_id: 2,
                        serializer_id: 1,
                        value: EntityDataValueKind::Int(300),
                    },
                    EntityDataValue {
                        data_id: 3,
                        serializer_id: 5,
                        value: EntityDataValueKind::Component("Name".to_string()),
                    },
                    EntityDataValue {
                        data_id: 4,
                        serializer_id: 6,
                        value: EntityDataValueKind::OptionalComponent(None),
                    },
                    EntityDataValue {
                        data_id: 5,
                        serializer_id: 9,
                        value: EntityDataValueKind::Rotations {
                            x: 1.0,
                            y: 2.0,
                            z: 3.0,
                        },
                    },
                    EntityDataValue {
                        data_id: 8,
                        serializer_id: 7,
                        value: EntityDataValueKind::ItemStack(ItemStackSummary {
                            item_id: Some(42),
                            count: 3,
                            component_patch: Default::default(),
                        }),
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_entity_transient_event_packets() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(3);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ANIMATE, &payload.into_inner()).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::EntityAnimation(EntityAnimation { id: 123, action: 3 })
        );

        let mut payload = Encoder::new();
        payload.write_i32(123);
        payload.write_i8(35);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ENTITY_EVENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 123,
                event_id: 35,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_f32(45.5);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_HURT_ANIMATION, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::HurtAnimation(HurtAnimation { id: 123, yaw: 45.5 })
        );
    }

    #[test]
    fn decodes_set_equipment_slots_and_item_stacks() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(EquipmentSlot::MainHand.ordinal() | 0x80);
        payload.write_var_i32(0);
        payload.write_u8(EquipmentSlot::Head.ordinal());
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_EQUIPMENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEquipment(SetEquipment {
                entity_id: 123,
                slots: vec![
                    EquipmentSlotUpdate {
                        slot: EquipmentSlot::MainHand,
                        item: ItemStackSummary::empty(),
                    },
                    EquipmentSlotUpdate {
                        slot: EquipmentSlot::Head,
                        item: ItemStackSummary {
                            item_id: Some(42),
                            count: 1,
                            component_patch: DataComponentPatchSummary::default(),
                        },
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_update_attributes_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(2);
        payload.write_var_i32(21);
        payload.write_f64(20.0);
        payload.write_var_i32(1);
        payload.write_string("minecraft:health_bonus");
        payload.write_f64(4.0);
        payload.write_var_i32(0);
        payload.write_var_i32(26);
        payload.write_f64(0.7);
        payload.write_var_i32(0);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::UpdateAttributes(UpdateAttributes {
                entity_id: 123,
                attributes: vec![
                    AttributeSnapshot {
                        attribute_id: 21,
                        base: 20.0,
                        modifiers: vec![AttributeModifier {
                            id: "minecraft:health_bonus".to_string(),
                            amount: 4.0,
                            operation_id: 0,
                        }],
                    },
                    AttributeSnapshot {
                        attribute_id: 26,
                        base: 0.7,
                        modifiers: Vec::new(),
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_set_passengers_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(2);
        payload.write_var_i32(123);
        payload.write_var_i32(456);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PASSENGERS, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPassengers(SetPassengers {
                vehicle_id: 7,
                passenger_ids: vec![123, 456],
            })
        );
    }

    #[test]
    fn decodes_set_entity_link_packet() {
        let mut payload = Encoder::new();
        payload.write_i32(123);
        payload.write_i32(456);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ENTITY_LINK,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEntityLink(SetEntityLink {
                source_id: 123,
                dest_id: 456,
            })
        );
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }

    fn lp_vec3_axis_x() -> [u8; 6] {
        let buffer = 1u64 | (32766u64 << 3) | (16383u64 << 18) | (16383u64 << 33);
        [
            buffer as u8,
            (buffer >> 8) as u8,
            ((buffer >> 16) >> 24) as u8,
            ((buffer >> 16) >> 16) as u8,
            ((buffer >> 16) >> 8) as u8,
            (buffer >> 16) as u8,
        ]
    }
}
