use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity_data_serializers::*;
pub use crate::equipment_slots::EquipmentSlot;
use crate::{
    attribute_modifier_types::normalize_attribute_modifier_operation_id,
    attribute_types::vanilla_attribute_type,
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_rich_text_component_from_decoder,
        decode_rich_text_component_from_decoder_with_registry,
    },
    content_registry::ContentRegistry,
    ids,
};

pub use super::GlobalPos as GlobalPosData;
use super::{
    chunks::decode_block_pos,
    decode_optional_rich_component_with_registry, decode_vec3d,
    inventory::{self, ItemStackSummary},
    normalize_resource_location, read_resource_location,
    world_effects::{decode_particle_payload_with_registry, ParticlePayload},
    BlockPos, GameProfileProperty, Vec3d,
};

const MAX_ENTITY_ATTRIBUTES: usize = 1024;
const MAX_ENTITY_ID_LIST: usize = 8192;
const MAX_EQUIPMENT_SLOTS: usize = 8;
// ClientboundUpdateAttributesPacket.java:61-80 has no fixed modifier cap; raise
// to match frame budget while decode bounds pre-allocation by remaining bytes.
const MAX_ATTRIBUTE_MODIFIERS: usize = 65_536;
const MAX_ENTITY_DATA_VALUES: usize = 255;
const MAX_ENTITY_DATA_PARTICLES: usize = 65_536;
const MAX_GAME_PROFILE_PROPERTIES: usize = 16;
const MAX_GAME_PROFILE_PROPERTY_NAME_CHARS: usize = 64;
const MAX_GAME_PROFILE_PROPERTY_SIGNATURE_CHARS: usize = 1024;
const MAX_PLAYER_NAME_CHARS: usize = 16;
const MAX_RESOURCE_LOCATION_CHARS: usize = 32767;
const MAX_MOD_CUSTOM_DATA_BYTES: usize = 65_536;
/// Mod-defined byte-blob serializer slot, placed immediately after Vanilla 26.1
/// serializers `0..=42` (`EntityDataSerializers.java`).
pub const MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID: i32 = 43;
// Vanilla uses ByteBufCodecs.list() without an explicit wire cap. Keep a repo-side
// allocation guard so malformed packets cannot request unbounded step storage.
const MAX_MINECART_LERP_STEPS: usize = 65_536;
// `LpVec3.ABS_MAX_VALUE` / `ABS_MIN_VALUE` (`LpVec3.java:17-18`).
const LP_VEC_ABS_MAX_VALUE: f64 = 1.7179869183E10;
const LP_VEC_ABS_MIN_VALUE: f64 = 3.051944088384301E-5;

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
    /// Whether the concrete `ClientboundMoveEntityPacket` variant carries the
    /// three relative position deltas. A position-bearing packet may encode
    /// three zero deltas, so this fact cannot be reconstructed from the values.
    #[serde(default)]
    pub has_position: bool,
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
    String(crate::JavaString),
    Component(crate::RichTextComponent),
    OptionalComponent(Option<crate::RichTextComponent>),
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
        value: EntityDataRegistryReference,
    },
    PaintingVariant(PaintingVariantData),
    EnumId {
        serializer: EntityDataEnumSerializer,
        id: i32,
    },
    VillagerData {
        villager_type: EntityDataRegistryReference,
        profession: EntityDataRegistryReference,
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
    ResolvableProfile(ResolvableProfileData),
    HumanoidArm(i32),
    ModCustom(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintingVariantData {
    pub registry: Option<EntityDataRegistryReference>,
    pub direct: Option<DirectPaintingVariantData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityDataRegistrySource {
    Server,
    VanillaFallback,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDataRegistryRevision {
    pub session: u64,
    pub contents: u64,
    pub tags: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDataRegistryIdentity {
    registry: String,
    resource_id: String,
    source: EntityDataRegistrySource,
    revision: EntityDataRegistryRevision,
}

impl EntityDataRegistryIdentity {
    pub fn new(
        registry: impl Into<String>,
        resource_id: impl Into<String>,
        source: EntityDataRegistrySource,
        revision: EntityDataRegistryRevision,
    ) -> Self {
        Self {
            registry: registry.into(),
            resource_id: resource_id.into(),
            source,
            revision,
        }
    }

    pub fn registry(&self) -> &str {
        &self.registry
    }

    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn source(&self) -> EntityDataRegistrySource {
        self.source
    }

    pub fn revision(&self) -> EntityDataRegistryRevision {
        self.revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDataRegistryReference {
    holder_id: i32,
    identity: Option<EntityDataRegistryIdentity>,
}

impl EntityDataRegistryReference {
    pub const fn unresolved(holder_id: i32) -> Self {
        Self {
            holder_id,
            identity: None,
        }
    }

    pub fn bind(&mut self, identity: EntityDataRegistryIdentity) {
        self.identity = Some(identity);
    }

    pub fn holder_id(&self) -> i32 {
        self.holder_id
    }

    pub fn identity(&self) -> Option<&EntityDataRegistryIdentity> {
        self.identity.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectPaintingVariantData {
    pub width: i32,
    pub height: i32,
    pub asset_id: String,
    pub title: Option<crate::RichTextComponent>,
    pub author: Option<crate::RichTextComponent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvableProfileData {
    pub profile: ResolvableProfileKind,
    pub skin_patch: PlayerSkinPatchData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolvableProfileKind {
    GameProfile {
        id: Uuid,
        name: String,
        properties: Vec<GameProfileProperty>,
    },
    Partial {
        name: Option<String>,
        id: Option<Uuid>,
        properties: Vec<GameProfileProperty>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSkinPatchData {
    pub body: Option<String>,
    pub cape: Option<String>,
    pub elytra: Option<String>,
    pub slim_model: Option<bool>,
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
}

impl EntityDataRegistryHolder {
    fn from_serializer_id(serializer_id: i32) -> Option<Self> {
        Some(match serializer_id {
            VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID => Self::CatVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID => Self::CatSoundVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID => Self::CowVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID => Self::CowSoundVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_WOLF_VARIANT_ID => Self::WolfVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_WOLF_SOUND_VARIANT_ID => Self::WolfSoundVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_FROG_VARIANT_ID => Self::FrogVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_PIG_VARIANT_ID => Self::PigVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_PIG_SOUND_VARIANT_ID => Self::PigSoundVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_VARIANT_ID => Self::ChickenVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_SOUND_VARIANT_ID => Self::ChickenSoundVariant,
            VANILLA_ENTITY_DATA_SERIALIZER_ZOMBIE_NAUTILUS_VARIANT_ID => {
                Self::ZombieNautilusVariant
            }
            _ => return None,
        })
    }

    pub const fn registry(self) -> &'static str {
        match self {
            Self::CatVariant => "minecraft:cat_variant",
            Self::CatSoundVariant => "minecraft:cat_sound_variant",
            Self::CowVariant => "minecraft:cow_variant",
            Self::CowSoundVariant => "minecraft:cow_sound_variant",
            Self::WolfVariant => "minecraft:wolf_variant",
            Self::WolfSoundVariant => "minecraft:wolf_sound_variant",
            Self::FrogVariant => "minecraft:frog_variant",
            Self::PigVariant => "minecraft:pig_variant",
            Self::PigSoundVariant => "minecraft:pig_sound_variant",
            Self::ChickenVariant => "minecraft:chicken_variant",
            Self::ChickenSoundVariant => "minecraft:chicken_sound_variant",
            Self::ZombieNautilusVariant => "minecraft:zombie_nautilus_variant",
        }
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
            VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID => Self::SnifferState,
            VANILLA_ENTITY_DATA_SERIALIZER_ARMADILLO_STATE_ID => Self::ArmadilloState,
            VANILLA_ENTITY_DATA_SERIALIZER_COPPER_GOLEM_STATE_ID => Self::CopperGolemState,
            VANILLA_ENTITY_DATA_SERIALIZER_WEATHERING_COPPER_STATE_ID => {
                Self::WeatheringCopperState
            }
            _ => return None,
        })
    }
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
    let packet = AddEntity {
        id: decoder.read_var_i32()?,
        uuid: decoder.read_uuid()?,
        entity_type_id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_lp_vec3d(decoder)?,
        x_rot: unpack_degrees(decoder.read_i8()?),
        y_rot: unpack_degrees(decoder.read_i8()?),
        y_head_rot: unpack_degrees(decoder.read_i8()?),
        data: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after add entity packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_entity_animation(decoder: &mut Decoder<'_>) -> Result<EntityAnimation> {
    let update = EntityAnimation {
        id: decoder.read_var_i32()?,
        action: decoder.read_u8()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after entity animation packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_entity_event(decoder: &mut Decoder<'_>) -> Result<EntityEvent> {
    let update = EntityEvent {
        entity_id: decoder.read_i32()?,
        event_id: decoder.read_i8()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after entity event packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_hurt_animation(decoder: &mut Decoder<'_>) -> Result<HurtAnimation> {
    let update = HurtAnimation {
        id: decoder.read_var_i32()?,
        yaw: decoder.read_f32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after hurt animation packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_entity_position_sync(decoder: &mut Decoder<'_>) -> Result<EntityPositionSync> {
    let update = EntityPositionSync {
        id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
        on_ground: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after entity position sync packet".to_string(),
        ));
    }
    Ok(update)
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

    let update = EntityMove {
        id,
        has_position,
        delta_x,
        delta_y,
        delta_z,
        y_rot,
        x_rot,
        on_ground: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after move entity packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_move_vehicle(decoder: &mut Decoder<'_>) -> Result<MoveVehicle> {
    let update = MoveVehicle {
        position: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after move vehicle packet".to_string(),
        ));
    }
    Ok(update)
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

    let mut lerp_steps = Vec::with_capacity(decoder.bounded_capacity(step_count, 54));
    for _ in 0..step_count {
        lerp_steps.push(MinecartStep {
            position: decode_vec3d(decoder)?,
            movement: decode_vec3d(decoder)?,
            y_rot: unpack_degrees(decoder.read_i8()?),
            x_rot: unpack_degrees(decoder.read_i8()?),
            weight: decoder.read_f32()?,
        });
    }

    let update = MoveMinecartAlongTrack {
        entity_id,
        lerp_steps,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after move minecart along track packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_remove_entities(decoder: &mut Decoder<'_>) -> Result<RemoveEntities> {
    let count = decoder.read_len()?;
    if count > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(count, MAX_ENTITY_ID_LIST));
    }
    let mut entity_ids = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        entity_ids.push(decoder.read_var_i32()?);
    }
    let update = RemoveEntities { entity_ids };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after remove entities packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_take_item_entity(decoder: &mut Decoder<'_>) -> Result<TakeItemEntity> {
    let update = TakeItemEntity {
        item_id: decoder.read_var_i32()?,
        player_id: decoder.read_var_i32()?,
        amount: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after take item entity packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_rotate_head(decoder: &mut Decoder<'_>) -> Result<RotateHead> {
    let update = RotateHead {
        id: decoder.read_var_i32()?,
        y_head_rot: unpack_degrees(decoder.read_i8()?),
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after rotate head packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_set_entity_motion(decoder: &mut Decoder<'_>) -> Result<SetEntityMotion> {
    let update = SetEntityMotion {
        id: decoder.read_var_i32()?,
        delta_movement: decode_lp_vec3d(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set entity motion packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_set_entity_link(decoder: &mut Decoder<'_>) -> Result<SetEntityLink> {
    let update = SetEntityLink {
        source_id: decoder.read_i32()?,
        dest_id: decoder.read_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set entity link packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_set_passengers(decoder: &mut Decoder<'_>) -> Result<SetPassengers> {
    let vehicle_id = decoder.read_var_i32()?;
    let count = decoder.read_len()?;
    if count > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(count, MAX_ENTITY_ID_LIST));
    }
    let mut passenger_ids = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        passenger_ids.push(decoder.read_var_i32()?);
    }
    let update = SetPassengers {
        vehicle_id,
        passenger_ids,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set passengers packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_teleport_entity(decoder: &mut Decoder<'_>) -> Result<TeleportEntity> {
    let update = TeleportEntity {
        id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
        relatives_mask: decoder.read_i32()?,
        on_ground: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after teleport entity packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_set_equipment_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SetEquipment> {
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
        let slot = EquipmentSlot::from_ordinal(raw_slot & 0x7f).ok_or_else(|| {
            ProtocolError::InvalidData(format!("invalid equipment slot {}", raw_slot & 0x7f))
        })?;
        let item = inventory::decode_item_stack_summary_with_registry(decoder, registry)?;
        slots.push(EquipmentSlotUpdate { slot, item });

        if !should_continue {
            break;
        }
    }
    let update = SetEquipment { entity_id, slots };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set equipment packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_set_entity_data_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SetEntityData> {
    let id = decoder.read_var_i32()?;
    let mut values = Vec::new();
    loop {
        let data_id = decoder.read_u8()?;
        if data_id == 0xff {
            break;
        }
        if values.len() >= MAX_ENTITY_DATA_VALUES {
            return Err(ProtocolError::PacketTooLarge(
                values.len() + 1,
                MAX_ENTITY_DATA_VALUES,
            ));
        }
        let serializer_id = decoder.read_var_i32()?;
        values.push(EntityDataValue {
            data_id,
            serializer_id,
            value: decode_entity_data_value(decoder, serializer_id, registry)?,
        });
    }
    let packet = SetEntityData { id, values };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set entity data packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_entity_data_value(
    decoder: &mut Decoder<'_>,
    serializer_id: i32,
    registry: &ContentRegistry,
) -> Result<EntityDataValueKind> {
    Ok(match serializer_id {
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID => EntityDataValueKind::Byte(decoder.read_i8()?),
        VANILLA_ENTITY_DATA_SERIALIZER_INT_ID => EntityDataValueKind::Int(decoder.read_var_i32()?),
        VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID => {
            EntityDataValueKind::Long(decoder.read_var_i64()?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID => EntityDataValueKind::Float(decoder.read_f32()?),
        VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID => {
            EntityDataValueKind::String(decoder.read_java_string(32767)?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID => {
            EntityDataValueKind::Component(if registry.is_vanilla_only() {
                decode_rich_text_component_from_decoder(decoder)?
            } else {
                decode_rich_text_component_from_decoder_with_registry(decoder, registry)?
            })
        }
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID => {
            EntityDataValueKind::OptionalComponent(if decoder.read_bool()? {
                Some(if registry.is_vanilla_only() {
                    decode_rich_text_component_from_decoder(decoder)?
                } else {
                    decode_rich_text_component_from_decoder_with_registry(decoder, registry)?
                })
            } else {
                None
            })
        }
        VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID => {
            // Vanilla's EntityDataSerializers.ITEM_STACK is exactly
            // ItemStack.OPTIONAL_STREAM_CODEC. Keep metadata on the same bounded
            // decoder as inventory and equipment so nested component patches do
            // not acquire a second, lossy wire path.
            if registry.is_vanilla_only() {
                EntityDataValueKind::ItemStack(inventory::decode_item_stack_summary(decoder)?)
            } else {
                EntityDataValueKind::ItemStack(inventory::decode_item_stack_summary_with_registry(
                    decoder, registry,
                )?)
            }
        }
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID => {
            EntityDataValueKind::Boolean(decoder.read_bool()?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID => EntityDataValueKind::Rotations {
            x: decoder.read_f32()?,
            y: decoder.read_f32()?,
            z: decoder.read_f32()?,
        },
        VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_POS_ID => {
            EntityDataValueKind::BlockPos(decode_block_pos(decoder.read_i64()?))
        }
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID => {
            EntityDataValueKind::OptionalBlockPos(if decoder.read_bool()? {
                Some(decode_block_pos(decoder.read_i64()?))
            } else {
                None
            })
        }
        VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID => {
            EntityDataValueKind::Direction(decoder.read_var_i32()?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID => {
            EntityDataValueKind::OptionalLivingEntityReference(if decoder.read_bool()? {
                Some(decoder.read_uuid()?)
            } else {
                None
            })
        }
        VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID => {
            EntityDataValueKind::BlockState(decoder.read_var_i32()?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID => {
            let id = decoder.read_var_i32()?;
            EntityDataValueKind::OptionalBlockState((id != 0).then_some(id))
        }
        VANILLA_ENTITY_DATA_SERIALIZER_PARTICLE_ID => {
            EntityDataValueKind::Particle(decode_particle_payload_with_registry(decoder, registry)?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID => {
            let count = decoder.read_len()?;
            if count > MAX_ENTITY_DATA_PARTICLES {
                return Err(ProtocolError::PacketTooLarge(
                    count,
                    MAX_ENTITY_DATA_PARTICLES,
                ));
            }
            let mut particles = Vec::with_capacity(decoder.bounded_capacity(count, 1));
            for _ in 0..count {
                particles.push(decode_particle_payload_with_registry(decoder, registry)?);
            }
            EntityDataValueKind::Particles(particles)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID => EntityDataValueKind::VillagerData {
            villager_type: EntityDataRegistryReference::unresolved(decoder.read_var_i32()?),
            profession: EntityDataRegistryReference::unresolved(decoder.read_var_i32()?),
            level: decoder.read_var_i32()?,
        },
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID => {
            let value = decoder.read_var_i32()?;
            if value < 0 {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid optional unsigned int value {value}"
                )));
            }
            EntityDataValueKind::OptionalUnsignedInt((value != 0).then_some(value - 1))
        }
        VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID => {
            EntityDataValueKind::Pose(decoder.read_var_i32()?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_GLOBAL_POS_ID => {
            EntityDataValueKind::OptionalGlobalPos(if decoder.read_bool()? {
                Some(GlobalPosData {
                    dimension: read_resource_location(decoder)?,
                    pos: decode_block_pos(decoder.read_i64()?),
                })
            } else {
                None
            })
        }
        VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID => {
            EntityDataValueKind::PaintingVariant(decode_painting_variant_data(decoder, registry)?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID => EntityDataValueKind::Vector3f {
            x: decoder.read_f32()?,
            y: decoder.read_f32()?,
            z: decoder.read_f32()?,
        },
        VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID => EntityDataValueKind::Quaternionf {
            x: decoder.read_f32()?,
            y: decoder.read_f32()?,
            z: decoder.read_f32()?,
            w: decoder.read_f32()?,
        },
        VANILLA_ENTITY_DATA_SERIALIZER_RESOLVABLE_PROFILE_ID => {
            EntityDataValueKind::ResolvableProfile(decode_resolvable_profile_data(decoder)?)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID => {
            EntityDataValueKind::HumanoidArm(decoder.read_var_i32()?)
        }
        other if let Some(serializer) = EntityDataRegistryHolder::from_serializer_id(other) => {
            EntityDataValueKind::RegistryId {
                serializer,
                value: EntityDataRegistryReference::unresolved(decoder.read_var_i32()?),
            }
        }
        other if let Some(serializer) = EntityDataEnumSerializer::from_serializer_id(other) => {
            EntityDataValueKind::EnumId {
                serializer,
                id: decoder.read_var_i32()?,
            }
        }
        MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID => {
            let len = decoder.read_len()?;
            if len > MAX_MOD_CUSTOM_DATA_BYTES {
                return Err(ProtocolError::PacketTooLarge(
                    len,
                    MAX_MOD_CUSTOM_DATA_BYTES,
                ));
            }
            EntityDataValueKind::ModCustom(
                decoder.read_exact(len, "mod custom entity data")?.to_vec(),
            )
        }
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "unsupported entity data serializer {other}"
            )))
        }
    })
}

fn decode_painting_variant_data(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<PaintingVariantData> {
    let encoded_id = decoder.read_var_i32()?;
    if encoded_id < 0 {
        return Err(ProtocolError::NegativeLength(encoded_id));
    }
    if encoded_id == 0 {
        return Ok(PaintingVariantData {
            registry: None,
            direct: Some(decode_direct_painting_variant_data(decoder, registry)?),
        });
    }

    Ok(PaintingVariantData {
        registry: Some(EntityDataRegistryReference::unresolved(encoded_id - 1)),
        direct: None,
    })
}

/// `PaintingVariant.DIRECT_CODEC` (`PaintingVariant.java:20-21`) bounds painting
/// dimensions to `1..=16`.
const MAX_PAINTING_VARIANT_DIMENSION: i32 = 16;

fn decode_direct_painting_variant_data(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<DirectPaintingVariantData> {
    let width = decoder.read_var_i32()?;
    let height = decoder.read_var_i32()?;
    if !(1..=MAX_PAINTING_VARIANT_DIMENSION).contains(&width)
        || !(1..=MAX_PAINTING_VARIANT_DIMENSION).contains(&height)
    {
        return Err(ProtocolError::InvalidData(format!(
            "direct painting variant dimensions {width}x{height} must be in 1..={MAX_PAINTING_VARIANT_DIMENSION}"
        )));
    }
    Ok(DirectPaintingVariantData {
        width,
        height,
        asset_id: read_resource_location(decoder)?,
        title: decode_optional_rich_component_with_registry(decoder, registry)?,
        author: decode_optional_rich_component_with_registry(decoder, registry)?,
    })
}

fn decode_resolvable_profile_data(decoder: &mut Decoder<'_>) -> Result<ResolvableProfileData> {
    let profile = if decoder.read_bool()? {
        let id = decoder.read_uuid()?;
        let name = decoder.read_string(MAX_PLAYER_NAME_CHARS)?;
        let properties = decode_game_profile_properties(decoder)?;
        ResolvableProfileKind::GameProfile {
            id,
            name,
            properties,
        }
    } else {
        let name = decode_optional_string(decoder, MAX_PLAYER_NAME_CHARS)?;
        let id = if decoder.read_bool()? {
            Some(decoder.read_uuid()?)
        } else {
            None
        };
        let properties = decode_game_profile_properties(decoder)?;
        ResolvableProfileKind::Partial {
            name,
            id,
            properties,
        }
    };
    Ok(ResolvableProfileData {
        profile,
        skin_patch: decode_player_skin_patch(decoder)?,
    })
}

fn decode_game_profile_properties(decoder: &mut Decoder<'_>) -> Result<Vec<GameProfileProperty>> {
    let count = decoder.read_len()?;
    if count > MAX_GAME_PROFILE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_GAME_PROFILE_PROPERTIES,
        ));
    }
    let mut properties = Vec::with_capacity(count);
    for _ in 0..count {
        properties.push(GameProfileProperty {
            name: decoder.read_string(MAX_GAME_PROFILE_PROPERTY_NAME_CHARS)?,
            value: decoder.read_string(MAX_RESOURCE_LOCATION_CHARS)?,
            signature: decode_optional_string(decoder, MAX_GAME_PROFILE_PROPERTY_SIGNATURE_CHARS)?,
        });
    }
    Ok(properties)
}

fn decode_player_skin_patch(decoder: &mut Decoder<'_>) -> Result<PlayerSkinPatchData> {
    Ok(PlayerSkinPatchData {
        body: decode_optional_string(decoder, MAX_RESOURCE_LOCATION_CHARS)?,
        cape: decode_optional_string(decoder, MAX_RESOURCE_LOCATION_CHARS)?,
        elytra: decode_optional_string(decoder, MAX_RESOURCE_LOCATION_CHARS)?,
        slim_model: if decoder.read_bool()? {
            Some(decoder.read_bool()?)
        } else {
            None
        },
    })
}

fn decode_optional_string(decoder: &mut Decoder<'_>, max_chars: usize) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_string(max_chars)?))
    } else {
        Ok(None)
    }
}

pub(super) fn decode_update_attributes(decoder: &mut Decoder<'_>) -> Result<UpdateAttributes> {
    let entity_id = decoder.read_var_i32()?;
    let attribute_count = decoder.read_len()?;
    if attribute_count > MAX_ENTITY_ATTRIBUTES {
        return Err(ProtocolError::InvalidData(format!(
            "attribute list has {attribute_count} entries, max is {MAX_ENTITY_ATTRIBUTES}"
        )));
    }
    let mut attributes = Vec::with_capacity(decoder.bounded_capacity(attribute_count, 10));
    for _ in 0..attribute_count {
        // RegistryFriendlyByteBuf resolves this holder through the connection's
        // RegistryAccess in Vanilla. The protocol crate deliberately has no
        // session owner, so retain the wire id for the world boundary instead
        // of interpreting it as a built-in ordinal here.
        let attribute_id = decoder.read_var_i32()?;
        let base = decoder.read_f64()?;
        let modifier_count = decoder.read_len()?;
        if modifier_count > MAX_ATTRIBUTE_MODIFIERS {
            return Err(ProtocolError::InvalidData(format!(
                "attribute modifier list has {modifier_count} entries, max is {MAX_ATTRIBUTE_MODIFIERS}"
            )));
        }
        let mut modifiers = Vec::with_capacity(decoder.bounded_capacity(modifier_count, 11));
        for _ in 0..modifier_count {
            modifiers.push(AttributeModifier {
                id: read_resource_location(decoder)?,
                amount: decoder.read_f64()?,
                operation_id: normalize_attribute_modifier_operation_id(decoder.read_var_i32()?),
            });
        }
        attributes.push(AttributeSnapshot {
            attribute_id,
            base,
            modifiers,
        });
    }
    let update = UpdateAttributes {
        entity_id,
        attributes,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after update attributes packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_attribute_holder_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let attribute_id = decoder.read_var_i32()?;
    if vanilla_attribute_type(attribute_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "invalid vanilla attribute holder id {attribute_id}"
        )));
    }
    Ok(attribute_id)
}

/// `ClientboundAddEntityPacket.write`
/// (`ClientboundAddEntityPacket.java:109-121`): id, uuid, entity type, the
/// absolute position, the `LpVec3` movement, then `xRot` before `yRot`,
/// `yHeadRot` and the type-specific data int.
pub fn encode_play_clientbound_add_entity(update: &AddEntity) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encoder.write_uuid(update.uuid);
    encoder.write_var_i32(update.entity_type_id);
    encoder.write_f64(update.position.x);
    encoder.write_f64(update.position.y);
    encoder.write_f64(update.position.z);
    encode_lp_vec3d(&mut encoder, update.delta_movement);
    encoder.write_i8(pack_degrees(update.x_rot));
    encoder.write_i8(pack_degrees(update.y_rot));
    encoder.write_i8(pack_degrees(update.y_head_rot));
    encoder.write_var_i32(update.data);
    (ids::play::CLIENTBOUND_ADD_ENTITY, encoder.into_inner())
}

// ClientboundTeleportEntityPacket.java:14-24; PositionMoveRotation.java:12-21.
pub fn encode_play_clientbound_teleport_entity(update: &TeleportEntity) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encoder.write_f64(update.position.x);
    encoder.write_f64(update.position.y);
    encoder.write_f64(update.position.z);
    encoder.write_f64(update.delta_movement.x);
    encoder.write_f64(update.delta_movement.y);
    encoder.write_f64(update.delta_movement.z);
    encoder.write_f32(update.y_rot);
    encoder.write_f32(update.x_rot);
    encoder.write_i32(update.relatives_mask);
    encoder.write_bool(update.on_ground);
    (ids::play::CLIENTBOUND_TELEPORT_ENTITY, encoder.into_inner())
}

/// `ClientboundEntityPositionSyncPacket.STREAM_CODEC`
/// (`ClientboundEntityPositionSyncPacket.java:12-20`): varint id, then the
/// composed `PositionMoveRotation` and the on-ground flag. This is the absolute
/// resync the 26.1 tracker sends instead of a teleport, and
/// `PositionMoveRotation.STREAM_CODEC` keeps the rotations as floats rather
/// than the byte-packed form the relative move packets use.
pub fn encode_play_clientbound_entity_position_sync(update: &EntityPositionSync) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encoder.write_f64(update.position.x);
    encoder.write_f64(update.position.y);
    encoder.write_f64(update.position.z);
    encoder.write_f64(update.delta_movement.x);
    encoder.write_f64(update.delta_movement.y);
    encoder.write_f64(update.delta_movement.z);
    encoder.write_f32(update.y_rot);
    encoder.write_f32(update.x_rot);
    encoder.write_bool(update.on_ground);
    (
        ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC,
        encoder.into_inner(),
    )
}

/// `ClientboundMoveEntityPacket`: one of three concrete packet ids, each with
/// its own writer — `Pos` (`ClientboundMoveEntityPacket.java:111-117`),
/// `PosRot` (`:145-153`) and `Rot` (`:178-183`). Every variant writes `yRot`
/// before `xRot` and omits the fields it does not carry, so the id and the
/// payload must agree; a packet with neither position nor rotation has no
/// Vanilla encoding.
pub fn encode_play_clientbound_move_entity(update: &EntityMove) -> Result<(i32, Vec<u8>)> {
    if update.y_rot.is_some() != update.x_rot.is_some() {
        return Err(ProtocolError::InvalidData(
            "move entity packet needs both rotation axes or neither".to_string(),
        ));
    }
    let has_rotation = update.y_rot.is_some();
    let packet_id = match (update.has_position, has_rotation) {
        (true, true) => ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT,
        (true, false) => ids::play::CLIENTBOUND_MOVE_ENTITY_POS,
        (false, true) => ids::play::CLIENTBOUND_MOVE_ENTITY_ROT,
        (false, false) => {
            return Err(ProtocolError::InvalidData(
                "move entity packet carries neither position nor rotation".to_string(),
            ))
        }
    };
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    if update.has_position {
        encoder.write_i16(update.delta_x);
        encoder.write_i16(update.delta_y);
        encoder.write_i16(update.delta_z);
    }
    if let (Some(y_rot), Some(x_rot)) = (update.y_rot, update.x_rot) {
        encoder.write_i8(pack_degrees(y_rot));
        encoder.write_i8(pack_degrees(x_rot));
    }
    encoder.write_bool(update.on_ground);
    Ok((packet_id, encoder.into_inner()))
}

/// `ClientboundMoveVehiclePacket.STREAM_CODEC`
/// (`ClientboundMoveVehiclePacket.java:11-29`) composes `Vec3.STREAM_CODEC`
/// (`Vec3.java:25-34`) with `yRot` then `xRot`, both as floats.
pub fn encode_play_clientbound_move_vehicle(update: MoveVehicle) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_f64(update.position.x);
    encoder.write_f64(update.position.y);
    encoder.write_f64(update.position.z);
    encoder.write_f32(update.y_rot);
    encoder.write_f32(update.x_rot);
    (ids::play::CLIENTBOUND_MOVE_VEHICLE, encoder.into_inner())
}

/// `ClientboundSetEntityMotionPacket.STREAM_CODEC`
/// (`ClientboundSetEntityMotionPacket.java:12-18`): the varint entity id
/// followed by the same low-precision vector `ClientboundAddEntityPacket`
/// carries, so both writers share [`encode_lp_vec3d`].
pub fn encode_play_clientbound_set_entity_motion(update: &SetEntityMotion) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encode_lp_vec3d(&mut encoder, update.delta_movement);
    (
        ids::play::CLIENTBOUND_SET_ENTITY_MOTION,
        encoder.into_inner(),
    )
}

/// `ClientboundRemoveEntitiesPacket.write`
/// (`ClientboundRemoveEntitiesPacket.java:28-30`): `writeIntIdList`, a varint
/// count followed by the tracked entity ids the receiving client must drop.
pub fn encode_play_clientbound_remove_entities(update: &RemoveEntities) -> Result<(i32, Vec<u8>)> {
    if update.entity_ids.len() > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(
            update.entity_ids.len(),
            MAX_ENTITY_ID_LIST,
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.entity_ids.len() as i32);
    for &entity_id in &update.entity_ids {
        encoder.write_var_i32(entity_id);
    }
    Ok((ids::play::CLIENTBOUND_REMOVE_ENTITIES, encoder.into_inner()))
}

/// `ClientboundTakeItemEntityPacket.write`
/// (`ClientboundTakeItemEntityPacket.java:28-32`): three varints, the picked
/// entity, the one that picked it up, and how many items left the stack.
pub fn encode_play_clientbound_take_item_entity(update: &TakeItemEntity) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.item_id);
    encoder.write_var_i32(update.player_id);
    encoder.write_var_i32(update.amount);
    (
        ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY,
        encoder.into_inner(),
    )
}

pub fn encode_play_clientbound_move_minecart_along_track(
    update: &MoveMinecartAlongTrack,
) -> Result<(i32, Vec<u8>)> {
    if update.lerp_steps.len() > MAX_MINECART_LERP_STEPS {
        return Err(ProtocolError::PacketTooLarge(
            update.lerp_steps.len(),
            MAX_MINECART_LERP_STEPS,
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.entity_id);
    encoder.write_var_i32(update.lerp_steps.len() as i32);
    for step in &update.lerp_steps {
        encoder.write_f64(step.position.x);
        encoder.write_f64(step.position.y);
        encoder.write_f64(step.position.z);
        encoder.write_f64(step.movement.x);
        encoder.write_f64(step.movement.y);
        encoder.write_f64(step.movement.z);
        encoder.write_i8(pack_degrees(step.y_rot));
        encoder.write_i8(pack_degrees(step.x_rot));
        encoder.write_f32(step.weight);
    }
    Ok((
        ids::play::CLIENTBOUND_MOVE_MINECART_ALONG_TRACK,
        encoder.into_inner(),
    ))
}

/// `ClientboundRotateHeadPacket.write` (`ClientboundRotateHeadPacket.java:29-32`):
/// the entity id, then the head yaw as a packed byte.
pub fn encode_play_clientbound_rotate_head(update: &RotateHead) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encoder.write_i8(pack_degrees(update.y_head_rot));
    (ids::play::CLIENTBOUND_ROTATE_HEAD, encoder.into_inner())
}

/// `ClientboundUpdateAttributesPacket.STREAM_CODEC`
/// (`ClientboundUpdateAttributesPacket.java:20-26`): the entity id, then
/// each `AttributeSnapshot` (`:71-79`) as the attribute's registry id, its
/// base and its modifiers (`:62-70`: id, amount, operation id).
pub fn encode_play_clientbound_update_attributes(
    update: &UpdateAttributes,
) -> Result<(i32, Vec<u8>)> {
    if update.attributes.len() > MAX_ENTITY_ATTRIBUTES {
        return Err(ProtocolError::InvalidData(format!(
            "attribute list has {} entries, max is {MAX_ENTITY_ATTRIBUTES}",
            update.attributes.len()
        )));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.entity_id);
    encoder.write_var_i32(update.attributes.len() as i32);
    for attribute in &update.attributes {
        if attribute.modifiers.len() > MAX_ATTRIBUTE_MODIFIERS {
            return Err(ProtocolError::InvalidData(format!(
                "attribute modifier list has {} entries, max is {MAX_ATTRIBUTE_MODIFIERS}",
                attribute.modifiers.len()
            )));
        }
        encoder.write_var_i32(attribute.attribute_id);
        encoder.write_f64(attribute.base);
        encoder.write_var_i32(attribute.modifiers.len() as i32);
        for modifier in &attribute.modifiers {
            encoder.write_string(&normalize_resource_location(modifier.id.clone())?);
            encoder.write_f64(modifier.amount);
            encoder.write_var_i32(modifier.operation_id);
        }
    }
    Ok((
        ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES,
        encoder.into_inner(),
    ))
}

/// `ClientboundSetEquipmentPacket.write` (`ClientboundSetEquipmentPacket.java:39-51`):
/// the entity id, then each slot's ordinal with the high bit set while more
/// follow, and its stack (`ItemStack.OPTIONAL_STREAM_CODEC`).
///
/// An empty slot list is refused: the reader's `do`/`while` (`:27-37`) needs
/// at least one slot and no vanilla caller sends none.
pub fn encode_play_clientbound_set_equipment(update: &SetEquipment) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_set_equipment_with_content_registry(update, ContentRegistry::vanilla())
}

pub fn encode_play_clientbound_set_equipment_with_content_registry(
    update: &SetEquipment,
    registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    if update.slots.is_empty() {
        return Err(ProtocolError::InvalidData(
            "set equipment packet carries no slots".to_string(),
        ));
    }
    if update.slots.len() > MAX_EQUIPMENT_SLOTS {
        return Err(ProtocolError::PacketTooLarge(
            update.slots.len(),
            MAX_EQUIPMENT_SLOTS,
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.entity_id);
    for (index, slot) in update.slots.iter().enumerate() {
        let more = index + 1 < update.slots.len();
        encoder.write_u8(slot.slot.ordinal() | if more { 0x80 } else { 0 });
        super::serverbound::encode_item_stack_summary_with_content_registry(
            &mut encoder,
            &slot.item,
            super::data_components::ComponentValueFraming::Direct,
            registry,
        )?;
    }
    Ok((ids::play::CLIENTBOUND_SET_EQUIPMENT, encoder.into_inner()))
}

/// `ClientboundSetEntityLinkPacket.write`
/// (`ClientboundSetEntityLinkPacket.java:27-30`): two big-endian ints.
pub fn encode_play_clientbound_set_entity_link(update: &SetEntityLink) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i32(update.source_id);
    encoder.write_i32(update.dest_id);
    (ids::play::CLIENTBOUND_SET_ENTITY_LINK, encoder.into_inner())
}

pub fn encode_play_clientbound_set_passengers(update: &SetPassengers) -> Result<(i32, Vec<u8>)> {
    if update.passenger_ids.len() > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(
            update.passenger_ids.len(),
            MAX_ENTITY_ID_LIST,
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.vehicle_id);
    encoder.write_var_i32(update.passenger_ids.len() as i32);
    for &passenger_id in &update.passenger_ids {
        encoder.write_var_i32(passenger_id);
    }
    Ok((ids::play::CLIENTBOUND_SET_PASSENGERS, encoder.into_inner()))
}

/// `Vec3.LP_STREAM_CODEC`: the same low-precision vector the serverbound
/// interact packet aims with, so both readers share this one decoder.
pub(super) fn decode_lp_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
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

/// `LpVec3.write` (`LpVec3.java:41-63`): quantize each axis against the
/// chessboard length so a whole `Vec3` fits in six bytes, with a varint
/// continuation carrying the high scale bits when the scale needs more than
/// two. Movements below `ABS_MIN_VALUE` collapse to the single zero byte.
pub(super) fn encode_lp_vec3d(encoder: &mut Encoder, value: Vec3d) {
    let x = sanitize_lp_vec_component(value.x);
    let y = sanitize_lp_vec_component(value.y);
    let z = sanitize_lp_vec_component(value.z);
    let chessboard_length = x.abs().max(y.abs()).max(z.abs());
    if chessboard_length < LP_VEC_ABS_MIN_VALUE {
        encoder.write_u8(0);
        return;
    }
    let scale = chessboard_length.ceil() as i64;
    let is_partial = (scale & 3) != scale;
    let markers = if is_partial { (scale & 3) | 4 } else { scale } as u64;
    let buffer = markers
        | (pack_lp_vec_component(x / scale as f64) << 3)
        | (pack_lp_vec_component(y / scale as f64) << 18)
        | (pack_lp_vec_component(z / scale as f64) << 33);
    encoder.write_u8(buffer as u8);
    encoder.write_u8((buffer >> 8) as u8);
    encoder.write_i32((buffer >> 16) as u32 as i32);
    if is_partial {
        encoder.write_var_i32((scale >> 2) as i32);
    }
}

fn sanitize_lp_vec_component(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-LP_VEC_ABS_MAX_VALUE, LP_VEC_ABS_MAX_VALUE)
    }
}

fn pack_lp_vec_component(value: f64) -> u64 {
    // `Math.round` is half-up toward positive infinity, which `f64::round`
    // only matches for the non-negative products this quantization produces.
    ((value * 0.5 + 0.5) * 32766.0).round() as i64 as u64
}

/// `Mth.unpackDegrees` (`Mth.java:185-187`): `rot * 360 / 256.0F`.
///
/// Vanilla's numerator is the `int` product `rot * 360`, whose whole range
/// (`-46080..=45720`) is exact in `f32`, so the float multiply here produces
/// the same numerator and the single division is the only rounding step. The
/// ratio is not folded into one constant: `[`pack_degrees`]`'s inverse order
/// matters and this is the value it round-trips against.
pub fn unpack_degrees(value: i8) -> f32 {
    f32::from(value) * 360.0 / 256.0
}

/// `Mth.packDegrees` (`Mth.java:181-183`): `(byte)floor(angle * 256.0F /
/// 360.0F)`. The multiply precedes the divide in Vanilla, so keep that order
/// rather than folding the ratio into one constant — the two round
/// differently. This is the only packing implementation; server world code
/// projects rotations through it rather than repeating the constant.
pub fn pack_degrees(degrees: f32) -> i8 {
    (degrees * 256.0 / 360.0).floor() as i32 as i8
}

#[cfg(test)]
mod tests;
