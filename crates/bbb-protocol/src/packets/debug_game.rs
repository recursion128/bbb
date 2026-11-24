use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::decode_rich_text_component_from_decoder_with_registry,
    content_registry::ContentRegistry,
    debug_subscription_types::{
        vanilla_debug_game_event_type_by_id, vanilla_debug_poi_type_by_id,
        vanilla_debug_subscription_type_by_id, VANILLA_DEBUG_SUBSCRIPTION_BEES_ID,
        VANILLA_DEBUG_SUBSCRIPTION_BEE_HIVES_ID, VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_BREEZES_ID,
        VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID,
        VANILLA_DEBUG_SUBSCRIPTION_ENTITY_BLOCK_INTERSECTIONS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID, VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID, VANILLA_DEBUG_SUBSCRIPTION_POIS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_REDSTONE_WIRE_ORIENTATIONS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_STRUCTURES_ID, VANILLA_DEBUG_SUBSCRIPTION_VILLAGE_SECTIONS_ID,
    },
    ids,
};
pub use bbb_facts::remote_debug_sample_types::RemoteDebugSampleType;

use super::{chunks, read_resource_location, BlockPos, ChunkPos, Vec3d};

const MAX_DEBUG_PAYLOAD: usize = 2 * 1024 * 1024;
const MAX_DEBUG_COLLECTION_VALUES: usize = 65_536;
const MAX_DEBUG_STRING_CHARS: usize = 32_767;
const MAX_DEBUG_SAMPLE_LONGS: usize = 1_000_000;
const MAX_GAME_RULE_VALUES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugBlockValue {
    pub pos: BlockPos,
    pub update: DebugSubscriptionUpdate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugChunkValue {
    pub pos: ChunkPos,
    pub update: DebugSubscriptionUpdate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugEntityValue {
    pub entity_id: i32,
    pub update: DebugSubscriptionUpdate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugEvent {
    pub event: DebugSubscriptionEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugSubscriptionUpdate {
    pub subscription_id: i32,
    pub value: Option<DebugSubscriptionValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugSubscriptionEvent {
    pub subscription_id: i32,
    pub value: DebugSubscriptionValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebugSubscriptionValue {
    Bee(DebugBeeInfo),
    Brain(DebugBrainDump),
    Breeze(DebugBreezeInfo),
    Goals(DebugGoalInfo),
    Path(DebugPathInfo),
    EntityBlockIntersection(DebugEntityBlockIntersection),
    Hive(DebugHiveInfo),
    Poi(DebugPoiInfo),
    RedstoneOrientation(DebugRedstoneOrientation),
    VillageSection,
    Raids(Vec<BlockPos>),
    Structures(Vec<DebugStructureInfo>),
    GameEventListener(DebugGameEventListenerInfo),
    NeighborUpdate(BlockPos),
    GameEvent(DebugGameEventInfo),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBeeInfo {
    pub hive_pos: Option<BlockPos>,
    pub flower_pos: Option<BlockPos>,
    pub travel_ticks: i32,
    pub blacklisted_hives: Vec<BlockPos>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugBrainDump {
    pub name: String,
    pub profession: String,
    pub xp: i32,
    pub health: f32,
    pub max_health: f32,
    pub inventory: String,
    pub wants_golem: bool,
    pub anger_level: i32,
    pub activities: Vec<String>,
    pub behaviors: Vec<String>,
    pub memories: Vec<String>,
    pub gossips: Vec<String>,
    pub pois: Vec<BlockPos>,
    pub potential_pois: Vec<BlockPos>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBreezeInfo {
    pub attack_target: Option<i32>,
    pub jump_target: Option<BlockPos>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugGoalInfo {
    pub goals: Vec<DebugGoal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugGoal {
    pub priority: i32,
    pub is_running: bool,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugPathInfo {
    pub path: DebugPath,
    pub max_node_distance: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugPath {
    pub reached: bool,
    pub next_node_index: i32,
    pub target: BlockPos,
    pub nodes: Vec<DebugPathNode>,
    pub target_nodes: Vec<DebugPathNode>,
    pub open_set: Vec<DebugPathNode>,
    pub closed_set: Vec<DebugPathNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DebugPathNode {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub walked_distance: f32,
    pub cost_malus: f32,
    pub closed: bool,
    pub path_type: DebugPathType,
    pub f: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum DebugPathType {
    Blocked,
    Open,
    Walkable,
    WalkableDoor,
    Trapdoor,
    PowderSnow,
    OnTopOfPowderSnow,
    Fence,
    Lava,
    Water,
    WaterBorder,
    Rail,
    UnpassableRail,
    FireInNeighbor,
    Fire,
    DamagingInNeighbor,
    Damaging,
    DoorOpen,
    DoorWoodClosed,
    DoorIronClosed,
    Breach,
    Leaves,
    StickyHoney,
    Cocoa,
    DamageCautious,
    OnTopOfTrapdoor,
    BigMobsCloseToDanger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugEntityBlockIntersection {
    InBlock,
    InFluid,
    InAir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugHiveInfo {
    pub block_registry_id: i32,
    pub occupant_count: i32,
    pub honey_level: i32,
    pub sedated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugPoiInfo {
    pub pos: BlockPos,
    pub poi_type_holder_id: i32,
    pub free_ticket_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugRedstoneOrientation {
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStructureInfo {
    pub bounding_box: DebugBoundingBox,
    pub pieces: Vec<DebugStructurePiece>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStructurePiece {
    pub bounding_box: DebugBoundingBox,
    pub is_start: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBoundingBox {
    pub min: BlockPos,
    pub max: BlockPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugGameEventListenerInfo {
    pub listener_radius: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DebugGameEventInfo {
    pub game_event_holder_id: i32,
    pub pos: Vec3d,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSample {
    pub sample: Vec<i64>,
    pub sample_type: RemoteDebugSampleType,
}

pub fn remote_debug_sample_type_name(sample_type: RemoteDebugSampleType) -> &'static str {
    match sample_type {
        RemoteDebugSampleType::TickTime => "tick_time",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValues {
    pub values: Vec<GameRuleValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValue {
    pub rule: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTestHighlightPos {
    pub absolute_pos: BlockPos,
    pub relative_pos: BlockPos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockStatus {
    pub status: crate::RichTextComponent,
    pub size: Option<Vec3i>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vec3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// `ClientboundDebugBlockValuePacket.STREAM_CODEC` (`ClientboundDebugBlockValuePacket.java:11-17`).
pub fn encode_play_clientbound_debug_block_value(
    update: &DebugBlockValue,
) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(update.pos));
    encode_debug_subscription_update(&mut out, &update.update)?;
    finish_debug_packet(ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE, out)
}

/// `ClientboundDebugChunkValuePacket.STREAM_CODEC` (`ClientboundDebugChunkValuePacket.java:11-17`).
pub fn encode_play_clientbound_debug_chunk_value(
    update: &DebugChunkValue,
) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_chunk_pos(update.pos));
    encode_debug_subscription_update(&mut out, &update.update)?;
    finish_debug_packet(ids::play::CLIENTBOUND_DEBUG_CHUNK_VALUE, out)
}

pub fn encode_play_clientbound_debug_entity_value(
    update: &DebugEntityValue,
) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    out.write_var_i32(update.entity_id);
    encode_debug_subscription_update(&mut out, &update.update)?;
    finish_debug_packet(ids::play::CLIENTBOUND_DEBUG_ENTITY_VALUE, out)
}

/// `DebugSubscription.Update.STREAM_CODEC` (`DebugSubscription.java:63-69`).
fn encode_debug_subscription_update(
    out: &mut Encoder,
    update: &DebugSubscriptionUpdate,
) -> Result<()> {
    validate_value_subscription_id(update.subscription_id)?;
    out.write_var_i32(update.subscription_id);
    out.write_bool(update.value.is_some());
    match &update.value {
        None => {}
        Some(DebugSubscriptionValue::Goals(goals))
            if update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID =>
        {
            encode_debug_goal_info(out, goals)?;
        }
        Some(DebugSubscriptionValue::Path(path))
            if update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID =>
        {
            encode_debug_path_info(out, path)?;
        }
        Some(DebugSubscriptionValue::Brain(dump))
            if update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID =>
        {
            encode_debug_brain_dump(out, dump)?;
        }
        Some(DebugSubscriptionValue::Poi(poi))
            if update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_POIS_ID =>
        {
            // `DebugPoiInfo.STREAM_CODEC` (`DebugPoiInfo.java:13-21`).
            out.write_i64(chunks::encode_block_pos(poi.pos));
            out.write_var_i32(poi.poi_type_holder_id);
            out.write_var_i32(poi.free_ticket_count);
        }
        Some(DebugSubscriptionValue::Raids(centers))
            if update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID =>
        {
            encode_debug_list(out, centers, |out, center| {
                out.write_i64(chunks::encode_block_pos(*center));
                Ok(())
            })?;
        }
        Some(DebugSubscriptionValue::GameEventListener(listener))
            if update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID =>
        {
            // `DebugGameEventListenerInfo.STREAM_CODEC` (`DebugGameEventListenerInfo.java:7-10`).
            out.write_var_i32(listener.listener_radius);
        }
        Some(_) => {
            return Err(ProtocolError::InvalidData(format!(
                "debug subscription id {} does not match a supported debug value",
                update.subscription_id,
            )))
        }
    }
    Ok(())
}

fn finish_debug_packet(packet_id: i32, out: Encoder) -> Result<(i32, Vec<u8>)> {
    let payload = out.into_inner();
    if payload.len() > MAX_DEBUG_PAYLOAD {
        return Err(ProtocolError::PacketTooLarge(
            payload.len(),
            MAX_DEBUG_PAYLOAD,
        ));
    }
    Ok((packet_id, payload))
}

/// `DebugBrainDump.write` (`DebugBrainDump.java:76-91`).
fn encode_debug_brain_dump(out: &mut Encoder, dump: &DebugBrainDump) -> Result<()> {
    encode_debug_string(out, &dump.name)?;
    encode_debug_string(out, &dump.profession)?;
    out.write_i32(dump.xp);
    out.write_f32(dump.health);
    out.write_f32(dump.max_health);
    encode_debug_string(out, &dump.inventory)?;
    out.write_bool(dump.wants_golem);
    out.write_i32(dump.anger_level);
    for strings in [
        &dump.activities,
        &dump.behaviors,
        &dump.memories,
        &dump.gossips,
    ] {
        encode_debug_list(out, strings, |out, value| encode_debug_string(out, value))?;
    }
    for positions in [&dump.pois, &dump.potential_pois] {
        encode_debug_list(out, positions, |out, pos| {
            out.write_i64(chunks::encode_block_pos(*pos));
            Ok(())
        })?;
    }
    Ok(())
}

/// The bound the matching read carries (`decode_string_list`), so a dump that
/// cannot be read back is refused where it is written.
fn encode_debug_string(out: &mut Encoder, value: &str) -> Result<()> {
    let chars = value.chars().count();
    if chars > MAX_DEBUG_STRING_CHARS {
        return Err(ProtocolError::StringTooLong {
            actual: chars,
            max: MAX_DEBUG_STRING_CHARS,
        });
    }
    out.write_string(value);
    Ok(())
}

fn encode_debug_goal_info(out: &mut Encoder, info: &DebugGoalInfo) -> Result<()> {
    encode_debug_list(out, &info.goals, |out, goal| {
        let name_len = goal.name.encode_utf16().count();
        if name_len > 255 {
            return Err(ProtocolError::StringTooLong {
                actual: name_len,
                max: 255,
            });
        }
        out.write_var_i32(goal.priority);
        out.write_bool(goal.is_running);
        out.write_string(&goal.name);
        Ok(())
    })
}

fn encode_debug_path_info(out: &mut Encoder, info: &DebugPathInfo) -> Result<()> {
    if info.path.target_nodes.is_empty() {
        return Err(ProtocolError::InvalidData(
            "debug path requires at least one target node".to_string(),
        ));
    }

    out.write_bool(info.path.reached);
    out.write_i32(info.path.next_node_index);
    out.write_i64(chunks::encode_block_pos(info.path.target));
    encode_path_node_list(out, &info.path.nodes)?;
    encode_path_node_list(out, &info.path.target_nodes)?;
    encode_path_node_list(out, &info.path.open_set)?;
    encode_path_node_list(out, &info.path.closed_set)?;
    out.write_f32(info.max_node_distance);
    Ok(())
}

fn encode_path_node_list(out: &mut Encoder, nodes: &[DebugPathNode]) -> Result<()> {
    encode_debug_list(out, nodes, |out, node| {
        out.write_i32(node.x);
        out.write_i32(node.y);
        out.write_i32(node.z);
        out.write_f32(node.walked_distance);
        out.write_f32(node.cost_malus);
        out.write_bool(node.closed);
        out.write_var_i32(node.path_type.ordinal());
        out.write_f32(node.f);
        Ok(())
    })
}

fn encode_debug_list<T>(
    out: &mut Encoder,
    values: &[T],
    mut encode: impl FnMut(&mut Encoder, &T) -> Result<()>,
) -> Result<()> {
    if values.len() > MAX_DEBUG_COLLECTION_VALUES {
        return Err(ProtocolError::PacketTooLarge(
            values.len(),
            MAX_DEBUG_COLLECTION_VALUES,
        ));
    }
    out.write_var_i32(values.len() as i32);
    for value in values {
        encode(out, value)?;
    }
    Ok(())
}

/// `ClientboundDebugEventPacket.STREAM_CODEC` (`ClientboundDebugEventPacket.java:9-15`):
/// the subscription id and its packed event value. The two event-shaped
/// channels this server publishes are the game-event and neighbor-update rows.
pub fn encode_play_clientbound_debug_event(event: &DebugEvent) -> Result<(i32, Vec<u8>)> {
    validate_value_subscription_id(event.event.subscription_id)?;

    let mut out = Encoder::new();
    out.write_var_i32(event.event.subscription_id);
    match &event.event.value {
        DebugSubscriptionValue::GameEvent(info)
            if event.event.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID =>
        {
            // `DebugGameEventInfo.STREAM_CODEC` (`DebugGameEventInfo.java:9-12`):
            // `holderRegistry(GAME_EVENT)` writes the bare holder id, then
            // `Vec3.STREAM_CODEC` writes three doubles.
            out.write_var_i32(info.game_event_holder_id);
            out.write_f64(info.pos.x);
            out.write_f64(info.pos.y);
            out.write_f64(info.pos.z);
        }
        DebugSubscriptionValue::NeighborUpdate(pos)
            if event.event.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID =>
        {
            // `BlockPos.STREAM_CODEC` (`BlockPos.java:111-116`).
            out.write_i64(chunks::encode_block_pos(*pos));
        }
        _ => {
            return Err(ProtocolError::InvalidData(format!(
                "debug subscription id {} does not match a supported event value",
                event.event.subscription_id
            )))
        }
    }

    finish_debug_packet(ids::play::CLIENTBOUND_DEBUG_EVENT, out)
}

pub(crate) fn decode_debug_block_value(decoder: &mut Decoder<'_>) -> Result<DebugBlockValue> {
    require_debug_payload_bound(decoder)?;
    let pos = chunks::decode_block_pos(decoder.read_i64()?);
    let update = decode_debug_subscription_update(decoder)?;
    require_debug_packet_end(decoder, "debug block value")?;
    Ok(DebugBlockValue { pos, update })
}

pub(crate) fn decode_debug_chunk_value(decoder: &mut Decoder<'_>) -> Result<DebugChunkValue> {
    require_debug_payload_bound(decoder)?;
    let pos = chunks::decode_chunk_pos(decoder.read_i64()?);
    let update = decode_debug_subscription_update(decoder)?;
    require_debug_packet_end(decoder, "debug chunk value")?;
    Ok(DebugChunkValue { pos, update })
}

pub(crate) fn decode_debug_entity_value(decoder: &mut Decoder<'_>) -> Result<DebugEntityValue> {
    require_debug_payload_bound(decoder)?;
    let entity_id = decoder.read_var_i32()?;
    let update = decode_debug_subscription_update(decoder)?;
    require_debug_packet_end(decoder, "debug entity value")?;
    Ok(DebugEntityValue { entity_id, update })
}

pub(crate) fn decode_debug_event(decoder: &mut Decoder<'_>) -> Result<DebugEvent> {
    require_debug_payload_bound(decoder)?;
    let subscription_id = decode_value_subscription_id(decoder)?;
    let value = decode_debug_subscription_value(decoder, subscription_id)?;
    require_debug_packet_end(decoder, "debug event")?;
    Ok(DebugEvent {
        event: DebugSubscriptionEvent {
            subscription_id,
            value,
        },
    })
}

fn decode_debug_subscription_update(decoder: &mut Decoder<'_>) -> Result<DebugSubscriptionUpdate> {
    let subscription_id = decode_value_subscription_id(decoder)?;
    let value = if decoder.read_bool()? {
        Some(decode_debug_subscription_value(decoder, subscription_id)?)
    } else {
        None
    };
    Ok(DebugSubscriptionUpdate {
        subscription_id,
        value,
    })
}

fn decode_value_subscription_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    validate_value_subscription_id(id)?;
    Ok(id)
}

fn validate_value_subscription_id(id: i32) -> Result<()> {
    let Some(entry) = vanilla_debug_subscription_type_by_id(id) else {
        return Err(ProtocolError::InvalidData(format!(
            "invalid debug subscription id {id}"
        )));
    };
    if id == VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID {
        return Err(ProtocolError::InvalidData(format!(
            "debug subscription {} has no value codec",
            entry.resource_id
        )));
    }
    Ok(())
}

fn decode_debug_subscription_value(
    decoder: &mut Decoder<'_>,
    subscription_id: i32,
) -> Result<DebugSubscriptionValue> {
    Ok(match subscription_id {
        VANILLA_DEBUG_SUBSCRIPTION_BEES_ID => {
            DebugSubscriptionValue::Bee(decode_debug_bee_info(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID => {
            DebugSubscriptionValue::Brain(decode_debug_brain_dump(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_BREEZES_ID => {
            DebugSubscriptionValue::Breeze(decode_debug_breeze_info(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID => {
            DebugSubscriptionValue::Goals(decode_debug_goal_info(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID => {
            DebugSubscriptionValue::Path(decode_debug_path_info(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_ENTITY_BLOCK_INTERSECTIONS_ID => {
            DebugSubscriptionValue::EntityBlockIntersection(decode_debug_entity_block_intersection(
                decoder,
            )?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_BEE_HIVES_ID => {
            DebugSubscriptionValue::Hive(decode_debug_hive_info(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_POIS_ID => {
            DebugSubscriptionValue::Poi(decode_debug_poi_info(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_REDSTONE_WIRE_ORIENTATIONS_ID => {
            DebugSubscriptionValue::RedstoneOrientation(decode_debug_orientation(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_VILLAGE_SECTIONS_ID => DebugSubscriptionValue::VillageSection,
        VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID => {
            DebugSubscriptionValue::Raids(decode_block_pos_list(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_STRUCTURES_ID => {
            DebugSubscriptionValue::Structures(decode_list(decoder, decode_debug_structure_info)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID => {
            DebugSubscriptionValue::GameEventListener(DebugGameEventListenerInfo {
                listener_radius: decoder.read_var_i32()?,
            })
        }
        VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID => {
            DebugSubscriptionValue::NeighborUpdate(decode_block_pos(decoder)?)
        }
        VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID => {
            let game_event_holder_id = decode_debug_game_event_holder_id(decoder)?;
            DebugSubscriptionValue::GameEvent(DebugGameEventInfo {
                game_event_holder_id,
                pos: Vec3d {
                    x: decoder.read_f64()?,
                    y: decoder.read_f64()?,
                    z: decoder.read_f64()?,
                },
            })
        }
        _ => {
            return Err(ProtocolError::InvalidData(format!(
                "debug subscription id {subscription_id} has no value decoder"
            )))
        }
    })
}

fn decode_debug_bee_info(decoder: &mut Decoder<'_>) -> Result<DebugBeeInfo> {
    Ok(DebugBeeInfo {
        hive_pos: decode_optional_block_pos(decoder)?,
        flower_pos: decode_optional_block_pos(decoder)?,
        travel_ticks: decoder.read_var_i32()?,
        blacklisted_hives: decode_block_pos_list(decoder)?,
    })
}

fn decode_debug_brain_dump(decoder: &mut Decoder<'_>) -> Result<DebugBrainDump> {
    Ok(DebugBrainDump {
        name: decoder.read_string(32767)?,
        profession: decoder.read_string(32767)?,
        xp: decoder.read_i32()?,
        health: decoder.read_f32()?,
        max_health: decoder.read_f32()?,
        inventory: decoder.read_string(32767)?,
        wants_golem: decoder.read_bool()?,
        anger_level: decoder.read_i32()?,
        activities: decode_string_list(decoder)?,
        behaviors: decode_string_list(decoder)?,
        memories: decode_string_list(decoder)?,
        gossips: decode_string_list(decoder)?,
        pois: decode_block_pos_list(decoder)?,
        potential_pois: decode_block_pos_list(decoder)?,
    })
}

fn decode_debug_breeze_info(decoder: &mut Decoder<'_>) -> Result<DebugBreezeInfo> {
    Ok(DebugBreezeInfo {
        attack_target: decode_optional_var_i32(decoder)?,
        jump_target: decode_optional_block_pos(decoder)?,
    })
}

fn decode_debug_goal_info(decoder: &mut Decoder<'_>) -> Result<DebugGoalInfo> {
    Ok(DebugGoalInfo {
        goals: decode_list(decoder, |decoder| {
            Ok(DebugGoal {
                priority: decoder.read_var_i32()?,
                is_running: decoder.read_bool()?,
                name: decoder.read_string(255)?,
            })
        })?,
    })
}

fn decode_debug_path_info(decoder: &mut Decoder<'_>) -> Result<DebugPathInfo> {
    let reached = decoder.read_bool()?;
    let next_node_index = decoder.read_i32()?;
    let target = decode_block_pos(decoder)?;
    let nodes = decode_path_node_list(decoder)?;
    let target_nodes = decode_path_node_list(decoder)?;
    let open_set = decode_path_node_list(decoder)?;
    let closed_set = decode_path_node_list(decoder)?;
    Ok(DebugPathInfo {
        path: DebugPath {
            reached,
            next_node_index,
            target,
            nodes,
            target_nodes,
            open_set,
            closed_set,
        },
        max_node_distance: decoder.read_f32()?,
    })
}

fn decode_path_node_list(decoder: &mut Decoder<'_>) -> Result<Vec<DebugPathNode>> {
    decode_list(decoder, decode_path_node)
}

fn decode_path_node(decoder: &mut Decoder<'_>) -> Result<DebugPathNode> {
    let x = decoder.read_i32()?;
    let y = decoder.read_i32()?;
    let z = decoder.read_i32()?;
    let walked_distance = decoder.read_f32()?;
    let cost_malus = decoder.read_f32()?;
    let closed = decoder.read_bool()?;
    let ordinal = decoder.read_var_i32()?;
    let Some(path_type) = DebugPathType::from_ordinal(ordinal) else {
        return Err(ProtocolError::InvalidData(format!(
            "invalid debug path type ordinal {ordinal}"
        )));
    };
    Ok(DebugPathNode {
        x,
        y,
        z,
        walked_distance,
        cost_malus,
        closed,
        path_type,
        f: decoder.read_f32()?,
    })
}

impl DebugPathType {
    const fn ordinal(self) -> i32 {
        self as i32
    }

    fn from_ordinal(ordinal: i32) -> Option<Self> {
        Some(match ordinal {
            0 => Self::Blocked,
            1 => Self::Open,
            2 => Self::Walkable,
            3 => Self::WalkableDoor,
            4 => Self::Trapdoor,
            5 => Self::PowderSnow,
            6 => Self::OnTopOfPowderSnow,
            7 => Self::Fence,
            8 => Self::Lava,
            9 => Self::Water,
            10 => Self::WaterBorder,
            11 => Self::Rail,
            12 => Self::UnpassableRail,
            13 => Self::FireInNeighbor,
            14 => Self::Fire,
            15 => Self::DamagingInNeighbor,
            16 => Self::Damaging,
            17 => Self::DoorOpen,
            18 => Self::DoorWoodClosed,
            19 => Self::DoorIronClosed,
            20 => Self::Breach,
            21 => Self::Leaves,
            22 => Self::StickyHoney,
            23 => Self::Cocoa,
            24 => Self::DamageCautious,
            25 => Self::OnTopOfTrapdoor,
            26 => Self::BigMobsCloseToDanger,
            _ => return None,
        })
    }
}

fn decode_debug_entity_block_intersection(
    decoder: &mut Decoder<'_>,
) -> Result<DebugEntityBlockIntersection> {
    Ok(match decoder.read_var_i32()? {
        1 => DebugEntityBlockIntersection::InFluid,
        2 => DebugEntityBlockIntersection::InAir,
        _ => DebugEntityBlockIntersection::InBlock,
    })
}

fn decode_debug_hive_info(decoder: &mut Decoder<'_>) -> Result<DebugHiveInfo> {
    Ok(DebugHiveInfo {
        block_registry_id: decode_non_negative_registry_id(decoder, "block")?,
        occupant_count: decoder.read_var_i32()?,
        honey_level: decoder.read_var_i32()?,
        sedated: decoder.read_bool()?,
    })
}

fn decode_debug_poi_info(decoder: &mut Decoder<'_>) -> Result<DebugPoiInfo> {
    Ok(DebugPoiInfo {
        pos: decode_block_pos(decoder)?,
        poi_type_holder_id: decode_debug_poi_holder_id(decoder)?,
        free_ticket_count: decoder.read_var_i32()?,
    })
}

fn decode_debug_orientation(decoder: &mut Decoder<'_>) -> Result<DebugRedstoneOrientation> {
    let index = decoder.read_var_i32()?;
    if !(0..48).contains(&index) {
        return Err(ProtocolError::InvalidData(format!(
            "invalid redstone orientation index {index}"
        )));
    }
    Ok(DebugRedstoneOrientation { index })
}

fn decode_debug_structure_info(decoder: &mut Decoder<'_>) -> Result<DebugStructureInfo> {
    Ok(DebugStructureInfo {
        bounding_box: decode_debug_bounding_box(decoder)?,
        pieces: decode_list(decoder, |decoder| {
            Ok(DebugStructurePiece {
                bounding_box: decode_debug_bounding_box(decoder)?,
                is_start: decoder.read_bool()?,
            })
        })?,
    })
}

fn decode_debug_bounding_box(decoder: &mut Decoder<'_>) -> Result<DebugBoundingBox> {
    let first = decode_block_pos(decoder)?;
    let second = decode_block_pos(decoder)?;
    Ok(DebugBoundingBox {
        min: BlockPos {
            x: first.x.min(second.x),
            y: first.y.min(second.y),
            z: first.z.min(second.z),
        },
        max: BlockPos {
            x: first.x.max(second.x),
            y: first.y.max(second.y),
            z: first.z.max(second.z),
        },
    })
}

fn decode_optional_var_i32(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    decoder
        .read_bool()?
        .then(|| decoder.read_var_i32())
        .transpose()
}

fn decode_optional_block_pos(decoder: &mut Decoder<'_>) -> Result<Option<BlockPos>> {
    decoder
        .read_bool()?
        .then(|| decode_block_pos(decoder))
        .transpose()
}

fn decode_block_pos(decoder: &mut Decoder<'_>) -> Result<BlockPos> {
    Ok(chunks::decode_block_pos(decoder.read_i64()?))
}

fn decode_block_pos_list(decoder: &mut Decoder<'_>) -> Result<Vec<BlockPos>> {
    decode_list(decoder, decode_block_pos)
}

fn decode_string_list(decoder: &mut Decoder<'_>) -> Result<Vec<String>> {
    decode_list(decoder, |decoder| decoder.read_string(32767))
}

fn decode_list<T>(
    decoder: &mut Decoder<'_>,
    mut decode: impl FnMut(&mut Decoder<'_>) -> Result<T>,
) -> Result<Vec<T>> {
    let len = decoder.read_len()?;
    if len > MAX_DEBUG_COLLECTION_VALUES {
        return Err(ProtocolError::PacketTooLarge(
            len,
            MAX_DEBUG_COLLECTION_VALUES,
        ));
    }
    let mut values = Vec::with_capacity(decoder.bounded_capacity(len, 1));
    for _ in 0..len {
        values.push(decode(decoder)?);
    }
    if decoder.position() > MAX_DEBUG_PAYLOAD {
        return Err(ProtocolError::PacketTooLarge(
            decoder.position(),
            MAX_DEBUG_PAYLOAD,
        ));
    }
    Ok(values)
}

fn decode_non_negative_registry_id(decoder: &mut Decoder<'_>, what: &str) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "negative {what} registry id {id}"
        )));
    }
    Ok(id)
}

fn decode_debug_game_event_holder_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decode_non_negative_registry_id(decoder, "game event")?;
    if vanilla_debug_game_event_type_by_id(id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "unknown game event registry id {id}"
        )));
    }
    Ok(id)
}

fn decode_debug_poi_holder_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decode_non_negative_registry_id(decoder, "point of interest type")?;
    if vanilla_debug_poi_type_by_id(id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "unknown point of interest type registry id {id}"
        )));
    }
    Ok(id)
}

fn require_debug_payload_bound(decoder: &Decoder<'_>) -> Result<()> {
    if decoder.remaining_len() > MAX_DEBUG_PAYLOAD {
        return Err(ProtocolError::PacketTooLarge(
            decoder.remaining_len(),
            MAX_DEBUG_PAYLOAD,
        ));
    }
    Ok(())
}

fn require_debug_packet_end(decoder: &Decoder<'_>, what: &str) -> Result<()> {
    if decoder.position() > MAX_DEBUG_PAYLOAD {
        return Err(ProtocolError::PacketTooLarge(
            decoder.position(),
            MAX_DEBUG_PAYLOAD,
        ));
    }
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {what} packet"
        )));
    }
    Ok(())
}

/// `ClientboundDebugSamplePacket.write`: a long array then the type ordinal.
pub fn encode_play_clientbound_debug_sample(sample: &DebugSample) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(sample.sample.len() as i32);
    for value in &sample.sample {
        out.write_i64(*value);
    }
    out.write_var_i32(sample.sample_type.ordinal());
    (ids::play::CLIENTBOUND_DEBUG_SAMPLE, out.into_inner())
}

pub(crate) fn decode_debug_sample(decoder: &mut Decoder<'_>) -> Result<DebugSample> {
    let count = decoder.read_len()?;
    if count > MAX_DEBUG_SAMPLE_LONGS {
        return Err(ProtocolError::PacketTooLarge(count, MAX_DEBUG_SAMPLE_LONGS));
    }

    let mut sample = Vec::with_capacity(decoder.bounded_capacity(count, 8));
    for _ in 0..count {
        sample.push(decoder.read_i64()?);
    }

    let ordinal = decoder.read_var_i32()?;
    let sample_type = RemoteDebugSampleType::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!(
            "invalid remote debug sample type ordinal {ordinal}"
        ))
    })?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after debug sample packet".to_string(),
        ));
    }
    Ok(DebugSample {
        sample,
        sample_type,
    })
}

pub(crate) fn decode_game_rule_values(decoder: &mut Decoder<'_>) -> Result<GameRuleValues> {
    let count = decoder.read_len()?;
    if count > MAX_GAME_RULE_VALUES {
        return Err(ProtocolError::PacketTooLarge(count, MAX_GAME_RULE_VALUES));
    }

    let mut values = Vec::with_capacity(decoder.bounded_capacity(count, 3));
    for _ in 0..count {
        values.push(GameRuleValue {
            rule: read_resource_location(decoder)?,
            value: decoder.read_string(32767)?,
        });
    }

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after game rule values packet".to_string(),
        ));
    }
    Ok(GameRuleValues { values })
}

pub fn encode_play_clientbound_game_test_highlight_pos(
    update: GameTestHighlightPos,
) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(update.absolute_pos));
    out.write_i64(chunks::encode_block_pos(update.relative_pos));
    (
        ids::play::CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS,
        out.into_inner(),
    )
}

pub fn encode_play_clientbound_test_instance_block_status(
    update: &TestInstanceBlockStatus,
) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    out.write_bytes(&crate::nbt_summary::encode_nbt_summary_bounded(
        Some(update.status.component()),
        crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize,
        crate::nbt_summary::MAX_NBT_DEPTH,
    )?);
    out.write_bool(update.size.is_some());
    if let Some(size) = update.size {
        out.write_var_i32(size.x);
        out.write_var_i32(size.y);
        out.write_var_i32(size.z);
    }
    Ok((
        ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS,
        out.into_inner(),
    ))
}

pub(crate) fn decode_game_test_highlight_pos(
    decoder: &mut Decoder<'_>,
) -> Result<GameTestHighlightPos> {
    let packet = GameTestHighlightPos {
        absolute_pos: chunks::decode_block_pos(decoder.read_i64()?),
        relative_pos: chunks::decode_block_pos(decoder.read_i64()?),
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after game test highlight pos packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_test_instance_block_status_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<TestInstanceBlockStatus> {
    let status = decode_rich_text_component_from_decoder_with_registry(decoder, registry)?;
    let size = if decoder.read_bool()? {
        Some(decode_vec3i(decoder)?)
    } else {
        None
    };

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after test instance block status packet".to_string(),
        ));
    }

    Ok(TestInstanceBlockStatus { status, size })
}

fn decode_vec3i(decoder: &mut Decoder<'_>) -> Result<Vec3i> {
    Ok(Vec3i {
        x: decoder.read_var_i32()?,
        y: decoder.read_var_i32()?,
        z: decoder.read_var_i32()?,
    })
}

#[cfg(test)]
mod tests;
