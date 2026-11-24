use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::debug_subscription_types::vanilla_debug_subscription_type_by_id;
pub use crate::player_command_actions::PlayerCommandAction;
use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    content_registry::ContentRegistry,
    game_rules::{vanilla_game_rule, VANILLA_GAME_RULES},
    ids,
    item_types::{VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE, VANILLA_ITEM_AIR_ID},
};
pub use bbb_facts::client_command_actions::ClientCommandAction;
pub use bbb_facts::client_command_actions::VANILLA_CLIENT_COMMAND_ACTIONS;
pub use bbb_facts::container_inputs::ContainerInput;
pub use bbb_facts::interaction_hands::InteractionHand;
pub use bbb_facts::player_action_kinds::PlayerActionKind;

use super::client_features::{RecipeBookType, RecipeDisplayId};
use super::client_state::Difficulty;
use super::inventory::{
    AdventureModePredicateDataSummary, DataComponentPatchSummary, ItemStackSummary,
};
use super::player_info::{GameType, PlayerInfoChatSession};
use super::{
    chunks,
    client_common::{self, CustomClickAction},
    connection, data_components, normalize_resource_location, read_resource_location, BlockPos,
    MessageSignature, Vec3d,
};

pub(crate) const PLAYER_INPUT_FORWARD: u8 = 1;
pub(crate) const PLAYER_INPUT_BACKWARD: u8 = 2;
pub(crate) const PLAYER_INPUT_LEFT: u8 = 4;
pub(crate) const PLAYER_INPUT_RIGHT: u8 = 8;
pub(crate) const PLAYER_INPUT_JUMP: u8 = 16;
pub(crate) const PLAYER_INPUT_SHIFT: u8 = 32;
pub(crate) const PLAYER_INPUT_SPRINT: u8 = 64;
const LAST_SEEN_MESSAGES_BITSET_BITS: u32 = 20;
pub(super) const LAST_SEEN_MESSAGES_BITSET_BYTES: usize = 3;
const EMPTY_LAST_SEEN_MESSAGES_CHECKSUM: u8 = 1;
/// `MessageSignature.BYTES` (MessageSignature.java:16).
pub const MESSAGE_SIGNATURE_BYTES: usize = 256;
pub(super) const MAX_ARGUMENT_SIGNATURE_COUNT: usize = 8;
pub(super) const MAX_ARGUMENT_SIGNATURE_NAME_CHARS: usize = 16;
pub(super) const MAX_CHAT_MESSAGE_CHARS: usize = 256;
pub(super) const MAX_CHAT_COMMAND_CHARS: usize = 32767;
pub(super) const MAX_CONTAINER_CLICK_CHANGED_SLOTS: usize = 128;
pub(super) const MAX_HASHED_COMPONENT_PATCH_ENTRIES: usize = 256;
const MAX_GAME_RULE_VALUE_UTF16_UNITS: usize = 32767;
/// `ServerboundRenameItemPacket(FriendlyByteBuf)` (:18-20) reads the name with
/// `readUtf()`, whose default bound is `FriendlyByteBuf.MAX_STRING_LENGTH`.
const MAX_RENAME_ITEM_UTF16_UNITS: usize = 32767;
const MAX_PROFILE_PUBLIC_KEY_BYTES: usize = 512;
const MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES: usize = 4096;
const MAX_SERVERBOUND_CUSTOM_PAYLOAD: usize = 32767;
const MAX_CUSTOM_PAYLOAD_STRING_UTF16_UNITS: usize = 32767;
const MAX_EDIT_BOOK_PAGES: usize = 100;
const MAX_EDIT_BOOK_PAGE_UTF16_UNITS: usize = 1024;
const MAX_EDIT_BOOK_TITLE_UTF16_UNITS: usize = 32;
/// `ServerboundSignUpdatePacket.MAX_STRING_LENGTH` (:13).
const MAX_SIGN_UPDATE_LINE_UTF16_UNITS: usize = 384;
/// `BundleContents.NO_SELECTED_ITEM_INDEX` (BundleContents.java:34).
pub const NO_SELECTED_BUNDLE_ITEM_INDEX: i32 = -1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub shift: bool,
    pub sprint: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCommand {
    pub entity_id: i32,
    pub action: PlayerCommandAction,
    pub data: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAction {
    pub action: PlayerActionKind,
    pub pos: BlockPos,
    pub direction: Direction,
    pub sequence: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCommand<S = String> {
    pub command: S,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCommandSigned<S = String> {
    pub command: S,
    pub timestamp_millis: i64,
    pub salt: i64,
    pub argument_signatures: ArgumentSignatures,
    pub last_seen_messages: LastSeenMessagesUpdate,
}

impl ChatCommandSigned {
    pub fn unsigned_arguments(
        command: impl Into<String>,
        timestamp_millis: i64,
        salt: i64,
    ) -> Self {
        Self {
            command: command.into(),
            timestamp_millis,
            salt,
            argument_signatures: ArgumentSignatures::default(),
            last_seen_messages: LastSeenMessagesUpdate::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentSignatures {
    pub entries: Vec<ArgumentSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentSignature {
    pub name: String,
    pub signature: MessageSignature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage<S = String> {
    pub message: S,
    pub timestamp_millis: i64,
    pub salt: i64,
    #[serde(default)]
    pub signature: Option<MessageSignature>,
    pub last_seen_messages: LastSeenMessagesUpdate,
}

pub type ChatSessionUpdate = PlayerInfoChatSession;

impl ChatMessage {
    pub fn unsigned(message: impl Into<String>, timestamp_millis: i64, salt: i64) -> Self {
        Self {
            message: message.into(),
            timestamp_millis,
            salt,
            signature: None,
            last_seen_messages: LastSeenMessagesUpdate::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerboundCustomPayload {
    Brand { brand: String },
    Unknown { id: String, raw_payload: Vec<u8> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastSeenMessagesUpdate {
    pub offset: i32,
    pub acknowledged: u32,
    pub checksum: u8,
}

impl Default for LastSeenMessagesUpdate {
    fn default() -> Self {
        Self {
            offset: 0,
            acknowledged: 0,
            checksum: EMPTY_LAST_SEEN_MESSAGES_CHECKSUM,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundLastSeenMessages {
    pub update: LastSeenMessagesUpdate,
    pub signatures: Vec<MessageSignature>,
}

/// One `SignedArgument` of a parsed command: the argument's own name and the
/// raw value units Vanilla would sign (`ClientPacketListener.java:2653-2666`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignableCommandArgument {
    pub name: String,
    pub value: crate::JavaString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeDifficultyCommand {
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeGameModeCommand {
    pub game_mode: GameType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatAcknowledgement {
    pub offset: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockDifficultyCommand {
    pub locked: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetGameRule {
    pub entries: Vec<SetGameRuleEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetGameRuleEntry {
    pub rule: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEntityTagQuery {
    pub transaction_id: i32,
    pub pos: BlockPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityTagQuery {
    pub transaction_id: i32,
    pub entity_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackEntity {
    pub entity_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InteractEntity {
    pub entity_id: i32,
    pub hand: InteractionHand,
    pub location: Vec3d,
    pub using_secondary_action: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlockHitResult {
    pub pos: BlockPos,
    pub direction: Direction,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside: bool,
    pub world_border_hit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UseItemOn {
    pub hand: InteractionHand,
    pub hit: BlockHitResult,
    pub sequence: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UseItem {
    pub hand: InteractionHand,
    pub sequence: i32,
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickItemFromBlock {
    pub pos: BlockPos,
    pub include_data: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickItemFromEntity {
    pub entity_id: i32,
    pub include_data: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaddleBoat {
    pub left: bool,
    pub right: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerButtonClick {
    pub container_id: i32,
    pub button_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerClick {
    pub container_id: i32,
    pub state_id: i32,
    pub slot_num: i16,
    pub button_num: i8,
    pub input: ContainerInput,
    pub changed_slots: BTreeMap<i16, HashedStack>,
    pub carried_item: HashedStack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerCloseRequest {
    pub container_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSlotStateChanged {
    pub slot_id: i32,
    pub container_id: i32,
    pub new_state: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectBundleItem {
    pub slot_id: i32,
    pub selected_item_index: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookChangeSettingsCommand {
    pub book_type: RecipeBookType,
    pub open: bool,
    pub filtering: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookSeenRecipeCommand {
    pub recipe: RecipeDisplayId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditBook<T = String> {
    pub slot: i32,
    pub pages: Vec<T>,
    pub title: Option<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameItem {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeenAdvancements {
    OpenedTab { tab: String },
    ClosedScreen,
}

impl SeenAdvancements {
    fn action_id(&self) -> i32 {
        match self {
            Self::OpenedTab { .. } => 0,
            Self::ClosedScreen => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignUpdate {
    pub pos: BlockPos,
    pub is_front_text: bool,
    pub lines: [String; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpectateEntity {
    pub entity_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeleportToEntity {
    pub uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCreativeModeSlot {
    pub slot_num: i16,
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashedStack {
    Empty,
    Item(HashedItemStack),
}

impl HashedStack {
    pub fn empty() -> Self {
        Self::Empty
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashedItemStack {
    pub item_id: i32,
    pub count: i32,
    pub components: HashedComponentPatch,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashedComponentPatch {
    pub added_components: BTreeMap<i32, i32>,
    pub removed_components: BTreeSet<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    pub fn id(self) -> u8 {
        match self {
            Self::Down => 0,
            Self::Up => 1,
            Self::North => 2,
            Self::South => 3,
            Self::West => 4,
            Self::East => 5,
        }
    }
}

pub fn encode_play_keep_alive(id: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(id);
    (ids::play::SERVERBOUND_KEEP_ALIVE, out.into_inner())
}

pub fn encode_play_cookie_response(key: &str, payload: Option<&[u8]>) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::play::SERVERBOUND_COOKIE_RESPONSE,
        super::connection::encode_cookie_response_payload(key, payload)?,
    ))
}

pub fn encode_play_pong(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i32(id);
    (ids::play::SERVERBOUND_PONG, out.into_inner())
}

pub fn encode_play_client_information_default() -> Result<(i32, Vec<u8>)> {
    encode_play_client_information(&connection::ClientInformation::default())
}

pub fn encode_play_client_information(
    information: &connection::ClientInformation,
) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::play::SERVERBOUND_CLIENT_INFORMATION,
        connection::encode_client_information_payload(information)?,
    ))
}

/// `ServerboundMoveVehiclePacket` (:11), which — unlike the clientbound form
/// (`ClientboundMoveVehiclePacket.java:11`) — also carries the vehicle's
/// on-ground flag.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoveVehicleCommand {
    pub position: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}

pub fn encode_play_move_vehicle(command: MoveVehicleCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f64(command.position.x);
    out.write_f64(command.position.y);
    out.write_f64(command.position.z);
    out.write_f32(command.y_rot);
    out.write_f32(command.x_rot);
    out.write_bool(command.on_ground);
    (ids::play::SERVERBOUND_MOVE_VEHICLE, out.into_inner())
}

/// `ServerboundMoveVehiclePacket.STREAM_CODEC` (:12-22).
pub(super) fn decode_move_vehicle_command(decoder: &mut Decoder<'_>) -> Result<MoveVehicleCommand> {
    Ok(MoveVehicleCommand {
        position: Vec3d {
            x: decoder.read_f64()?,
            y: decoder.read_f64()?,
            z: decoder.read_f64()?,
        },
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
        on_ground: decoder.read_bool()?,
    })
}

pub fn encode_play_paddle_boat(packet: PaddleBoat) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_bool(packet.left);
    out.write_bool(packet.right);
    (ids::play::SERVERBOUND_PADDLE_BOAT, out.into_inner())
}

/// `ServerboundPaddleBoatPacket.STREAM_CODEC` (:12-18).
pub(super) fn decode_paddle_boat(decoder: &mut Decoder<'_>) -> Result<PaddleBoat> {
    Ok(PaddleBoat {
        left: decoder.read_bool()?,
        right: decoder.read_bool()?,
    })
}

pub fn encode_play_ping_request(time: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(time);
    (ids::play::SERVERBOUND_PING_REQUEST, out.into_inner())
}

pub fn encode_play_player_loaded() -> (i32, Vec<u8>) {
    (ids::play::SERVERBOUND_PLAYER_LOADED, Vec::new())
}

pub fn encode_play_client_tick_end() -> (i32, Vec<u8>) {
    (ids::play::SERVERBOUND_CLIENT_TICK_END, Vec::new())
}

pub fn encode_play_change_difficulty(command: ChangeDifficultyCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.difficulty.id());
    (ids::play::SERVERBOUND_CHANGE_DIFFICULTY, out.into_inner())
}

pub fn encode_play_change_game_mode(command: ChangeGameModeCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.game_mode.id());
    (ids::play::SERVERBOUND_CHANGE_GAME_MODE, out.into_inner())
}

pub fn encode_play_chat_acknowledgement(command: ChatAcknowledgement) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.offset);
    (ids::play::SERVERBOUND_CHAT_ACK, out.into_inner())
}

pub fn encode_play_custom_payload(packet: &ServerboundCustomPayload) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    match packet {
        ServerboundCustomPayload::Brand { brand } => {
            if brand.encode_utf16().count() > MAX_CUSTOM_PAYLOAD_STRING_UTF16_UNITS {
                return Err(ProtocolError::InvalidData(format!(
                    "custom payload brand exceeds vanilla maximum of {MAX_CUSTOM_PAYLOAD_STRING_UTF16_UNITS} UTF-16 code units"
                )));
            }
            out.write_string("minecraft:brand");
            out.write_string(brand);
        }
        ServerboundCustomPayload::Unknown { id, raw_payload } => {
            let id = normalize_resource_location(id.clone())?;
            if id.encode_utf16().count() > MAX_CUSTOM_PAYLOAD_STRING_UTF16_UNITS {
                return Err(ProtocolError::InvalidData(format!(
                    "custom payload id exceeds vanilla maximum of {MAX_CUSTOM_PAYLOAD_STRING_UTF16_UNITS} UTF-16 code units"
                )));
            }
            if raw_payload.len() > MAX_SERVERBOUND_CUSTOM_PAYLOAD {
                return Err(ProtocolError::PacketTooLarge(
                    raw_payload.len(),
                    MAX_SERVERBOUND_CUSTOM_PAYLOAD,
                ));
            }
            out.write_string(&id);
            out.write_bytes(raw_payload);
        }
    }
    Ok((ids::play::SERVERBOUND_CUSTOM_PAYLOAD, out.into_inner()))
}

pub fn encode_play_custom_click_action(packet: &CustomClickAction) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::play::SERVERBOUND_CUSTOM_CLICK_ACTION,
        client_common::encode_custom_click_action_payload(packet)?,
    ))
}

pub fn encode_play_debug_subscription_request(subscription_ids: &[i32]) -> Result<(i32, Vec<u8>)> {
    let mut subscription_ids = subscription_ids.to_vec();
    subscription_ids.sort_unstable();
    subscription_ids.dedup();
    for id in &subscription_ids {
        if vanilla_debug_subscription_type_by_id(*id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid debug subscription id {id}"
            )));
        }
    }
    let mut out = Encoder::new();
    out.write_var_i32(i32::try_from(subscription_ids.len()).map_err(|_| {
        ProtocolError::InvalidData("too many debug subscriptions requested".to_string())
    })?);
    for id in subscription_ids {
        out.write_var_i32(id);
    }
    Ok((
        ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST,
        out.into_inner(),
    ))
}

pub fn encode_play_lock_difficulty(command: LockDifficultyCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_bool(command.locked);
    (ids::play::SERVERBOUND_LOCK_DIFFICULTY, out.into_inner())
}

pub fn encode_play_set_game_rule(packet: &SetGameRule) -> Result<(i32, Vec<u8>)> {
    if packet.entries.len() > VANILLA_GAME_RULES.len() {
        return Err(ProtocolError::InvalidData(format!(
            "set game rule entry count {} exceeds built-in game rule count {}",
            packet.entries.len(),
            VANILLA_GAME_RULES.len()
        )));
    }

    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(packet.entries.len());
    for entry in &packet.entries {
        let rule = normalize_resource_location(entry.rule.clone())?;
        let catalog = vanilla_game_rule(&rule).ok_or_else(|| {
            ProtocolError::InvalidData(format!("unknown built-in game rule {rule}"))
        })?;
        if !seen.insert(rule.clone()) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate set game rule entry {rule}"
            )));
        }
        if entry.value.encode_utf16().count() > MAX_GAME_RULE_VALUE_UTF16_UNITS {
            return Err(ProtocolError::InvalidData(format!(
                "set game rule value exceeds vanilla maximum of {MAX_GAME_RULE_VALUE_UTF16_UNITS} UTF-16 code units"
            )));
        }
        if catalog.parse_value(&entry.value).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid value `{}` for built-in game rule {rule}",
                entry.value
            )));
        }
        entries.push((rule, entry.value.as_str()));
    }

    let mut out = Encoder::new();
    out.write_var_i32(i32::try_from(entries.len()).expect("built-in game rule count fits i32"));
    for (rule, value) in entries {
        out.write_string(&rule);
        out.write_string(value);
    }
    Ok((ids::play::SERVERBOUND_SET_GAME_RULE, out.into_inner()))
}

/// `ServerboundClientCommandPacket(FriendlyByteBuf)` (:18-20).
pub(super) fn decode_client_command(decoder: &mut Decoder<'_>) -> Result<ClientCommandAction> {
    let ordinal = decoder.read_var_i32()?;
    VANILLA_CLIENT_COMMAND_ACTIONS
        .iter()
        .find(|entry| entry.ordinal == ordinal)
        .map(|entry| entry.action)
        .ok_or_else(|| {
            ProtocolError::InvalidData(format!("invalid client command ordinal {ordinal}"))
        })
}

/// `ServerboundChangeGameModePacket.STREAM_CODEC` (:10-12) over
/// `GameType.STREAM_CODEC` (GameType.java:24-25), whose `ByIdMap` answers
/// `SURVIVAL` for an id outside the four rather than failing.
pub(super) fn decode_change_game_mode(decoder: &mut Decoder<'_>) -> Result<ChangeGameModeCommand> {
    Ok(ChangeGameModeCommand {
        game_mode: GameType::from_id(decoder.read_var_i32()?),
    })
}

/// `ServerboundSetGameRulePacket.STREAM_CODEC` (:14-16) over its `Entry`
/// (:27-34).
///
/// The registry key is decoded as the resource location it is on the wire.
/// `handleSetGameRule` looks it up and skips an entry it cannot resolve
/// (:2116-2118), so an unknown rule is not a decode failure.
pub(super) fn decode_set_game_rule(decoder: &mut Decoder<'_>) -> Result<SetGameRule> {
    let count = decoder.read_len()?;
    if count > VANILLA_GAME_RULES.len() {
        return Err(ProtocolError::InvalidData(format!(
            "set game rule entry count {count} exceeds built-in game rule count {}",
            VANILLA_GAME_RULES.len()
        )));
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(SetGameRuleEntry {
            rule: decoder.read_string(MAX_GAME_RULE_VALUE_UTF16_UNITS)?,
            value: decoder.read_string(MAX_GAME_RULE_VALUE_UTF16_UNITS)?,
        });
    }
    Ok(SetGameRule { entries })
}

/// `ServerboundBlockEntityTagQueryPacket.STREAM_CODEC`
/// (ServerboundBlockEntityTagQueryPacket.java).
pub(super) fn decode_block_entity_tag_query(
    decoder: &mut Decoder<'_>,
) -> Result<BlockEntityTagQuery> {
    Ok(BlockEntityTagQuery {
        transaction_id: decoder.read_var_i32()?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
    })
}

/// `ServerboundEntityTagQueryPacket.STREAM_CODEC`
/// (ServerboundEntityTagQueryPacket.java).
pub(super) fn decode_entity_tag_query(decoder: &mut Decoder<'_>) -> Result<EntityTagQuery> {
    Ok(EntityTagQuery {
        transaction_id: decoder.read_var_i32()?,
        entity_id: decoder.read_var_i32()?,
    })
}

/// `ServerboundSpectateEntityPacket.STREAM_CODEC`.
pub(super) fn decode_spectate_entity(decoder: &mut Decoder<'_>) -> Result<SpectateEntity> {
    Ok(SpectateEntity {
        entity_id: decoder.read_var_i32()?,
    })
}

/// `ServerboundTeleportToEntityPacket(FriendlyByteBuf)` (:22-24): the target is
/// named by profile id rather than by entity id, because a spectator may ask
/// for a player in another dimension.
pub(super) fn decode_teleport_to_entity(decoder: &mut Decoder<'_>) -> Result<TeleportToEntity> {
    Ok(TeleportToEntity {
        uuid: decoder.read_uuid()?,
    })
}

/// `ServerboundDebugSubscriptionRequestPacket.STREAM_CODEC` (:14-18), a
/// registry-id set. An id outside the registry is a decode failure on both
/// sides: `ByteBufCodecs.registry` has no fallback.
pub(super) fn decode_debug_subscription_request(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
    const MAX_DEBUG_SUBSCRIPTION_REQUEST_COUNT: usize = 64;
    let count = decoder.read_len()?;
    if count > MAX_DEBUG_SUBSCRIPTION_REQUEST_COUNT {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_DEBUG_SUBSCRIPTION_REQUEST_COUNT,
        ));
    }
    let mut subscription_ids = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    let mut seen_mask = 0u64;
    for _ in 0..count {
        let id = decoder.read_var_i32()?;
        if vanilla_debug_subscription_type_by_id(id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid debug subscription id {id}"
            )));
        }
        let bit = 1u64 << (id as u32);
        if seen_mask & bit == 0 {
            seen_mask |= bit;
            subscription_ids.push(id);
        }
    }
    Ok(subscription_ids)
}

pub fn encode_play_block_entity_tag_query(packet: BlockEntityTagQuery) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.transaction_id);
    out.write_i64(chunks::encode_block_pos(packet.pos));
    (
        ids::play::SERVERBOUND_BLOCK_ENTITY_TAG_QUERY,
        out.into_inner(),
    )
}

pub fn encode_play_entity_tag_query(packet: EntityTagQuery) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.transaction_id);
    out.write_var_i32(packet.entity_id);
    (ids::play::SERVERBOUND_ENTITY_TAG_QUERY, out.into_inner())
}

pub fn encode_play_player_input(input: PlayerInput) -> (i32, Vec<u8>) {
    let mut flags = 0u8;
    if input.forward {
        flags |= PLAYER_INPUT_FORWARD;
    }
    if input.backward {
        flags |= PLAYER_INPUT_BACKWARD;
    }
    if input.left {
        flags |= PLAYER_INPUT_LEFT;
    }
    if input.right {
        flags |= PLAYER_INPUT_RIGHT;
    }
    if input.jump {
        flags |= PLAYER_INPUT_JUMP;
    }
    if input.shift {
        flags |= PLAYER_INPUT_SHIFT;
    }
    if input.sprint {
        flags |= PLAYER_INPUT_SPRINT;
    }

    let mut out = Encoder::new();
    out.write_u8(flags);
    (ids::play::SERVERBOUND_PLAYER_INPUT, out.into_inner())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceRecipeCommand {
    pub container_id: i32,
    pub recipe_index: i32,
    pub use_max_items: bool,
}

/// `ServerboundPlaceRecipePacket.STREAM_CODEC` (:11-18): the menu the request
/// is for, the display index, and whether the shift-click "as many as possible"
/// form was used.
pub(super) fn decode_place_recipe(decoder: &mut Decoder<'_>) -> Result<PlaceRecipeCommand> {
    Ok(PlaceRecipeCommand {
        container_id: decoder.read_var_i32()?,
        recipe_index: decoder.read_var_i32()?,
        use_max_items: decoder.read_bool()?,
    })
}

pub fn encode_play_place_recipe(command: PlaceRecipeCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.container_id);
    out.write_var_i32(command.recipe_index);
    out.write_bool(command.use_max_items);
    (ids::play::SERVERBOUND_PLACE_RECIPE, out.into_inner())
}

pub fn encode_play_recipe_book_change_settings(
    command: RecipeBookChangeSettingsCommand,
) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.book_type.id());
    out.write_bool(command.open);
    out.write_bool(command.filtering);
    (
        ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS,
        out.into_inner(),
    )
}

/// `ServerboundRecipeBookChangeSettingsPacket.STREAM_CODEC` (:13-21) over
/// `RecipeBookType.STREAM_CODEC`, an ordinal mapper with no fallback.
pub(super) fn decode_recipe_book_change_settings(
    decoder: &mut Decoder<'_>,
) -> Result<RecipeBookChangeSettingsCommand> {
    let ordinal = decoder.read_var_i32()?;
    let book_type = RecipeBookType::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid recipe book type ordinal {ordinal}"))
    })?;
    Ok(RecipeBookChangeSettingsCommand {
        book_type,
        open: decoder.read_bool()?,
        filtering: decoder.read_bool()?,
    })
}

pub fn encode_play_recipe_book_seen_recipe(command: RecipeBookSeenRecipeCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.recipe.index);
    (
        ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE,
        out.into_inner(),
    )
}

/// `ServerboundRecipeBookSeenRecipePacket.STREAM_CODEC` (:12-14) over
/// `RecipeDisplayId.STREAM_CODEC`, which is the raw index: the bounds are the
/// server's to check (`RecipeManager.getRecipeFromDisplay`, :179-182).
pub(super) fn decode_recipe_book_seen_recipe(
    decoder: &mut Decoder<'_>,
) -> Result<RecipeBookSeenRecipeCommand> {
    Ok(RecipeBookSeenRecipeCommand {
        recipe: RecipeDisplayId {
            index: decoder.read_var_i32()?,
        },
    })
}

pub fn encode_play_edit_book(packet: &EditBook) -> Result<(i32, Vec<u8>)> {
    encode_play_edit_book_utf16(&EditBook {
        slot: packet.slot,
        pages: packet
            .pages
            .iter()
            .map(|text| text.encode_utf16().collect())
            .collect(),
        title: packet
            .title
            .as_ref()
            .map(|text| text.encode_utf16().collect()),
    })
}

pub fn encode_play_edit_book_utf16(packet: &EditBook<Vec<u16>>) -> Result<(i32, Vec<u8>)> {
    if packet.pages.len() > MAX_EDIT_BOOK_PAGES {
        return Err(ProtocolError::InvalidData(format!(
            "edit book page count {} exceeds vanilla maximum {MAX_EDIT_BOOK_PAGES}",
            packet.pages.len()
        )));
    }
    for (index, page) in packet.pages.iter().enumerate() {
        if page.len() > MAX_EDIT_BOOK_PAGE_UTF16_UNITS {
            return Err(ProtocolError::InvalidData(format!(
                "edit book page {index} exceeds vanilla maximum of {MAX_EDIT_BOOK_PAGE_UTF16_UNITS} UTF-16 code units"
            )));
        }
    }
    if packet
        .title
        .as_ref()
        .is_some_and(|title| title.len() > MAX_EDIT_BOOK_TITLE_UTF16_UNITS)
    {
        return Err(ProtocolError::InvalidData(format!(
            "edit book title exceeds vanilla maximum of {MAX_EDIT_BOOK_TITLE_UTF16_UNITS} UTF-16 code units"
        )));
    }

    let mut out = Encoder::new();
    out.write_var_i32(packet.slot);
    out.write_var_i32(packet.pages.len() as i32);
    for page in &packet.pages {
        out.write_utf16_string(page);
    }
    match &packet.title {
        Some(title) => {
            out.write_bool(true);
            out.write_utf16_string(title);
        }
        None => out.write_bool(false),
    }
    Ok((ids::play::SERVERBOUND_EDIT_BOOK, out.into_inner()))
}

/// `ServerboundEditBookPacket.STREAM_CODEC` (:12-20), whose page list, page
/// length and title length are all bounded on the wire.
pub(super) fn decode_edit_book(decoder: &mut Decoder<'_>) -> Result<EditBook> {
    let slot = decoder.read_var_i32()?;
    let count = decoder.read_len()?;
    if count > MAX_EDIT_BOOK_PAGES {
        return Err(ProtocolError::PacketTooLarge(count, MAX_EDIT_BOOK_PAGES));
    }
    let mut pages = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        pages.push(decoder.read_string(MAX_EDIT_BOOK_PAGE_UTF16_UNITS)?);
    }
    let title = decoder
        .read_bool()?
        .then(|| decoder.read_string(MAX_EDIT_BOOK_TITLE_UTF16_UNITS))
        .transpose()?;
    Ok(EditBook { slot, pages, title })
}

pub fn encode_play_rename_item(packet: &RenameItem) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_string(&packet.name);
    (ids::play::SERVERBOUND_RENAME_ITEM, out.into_inner())
}

/// `ServerboundRenameItemPacket(FriendlyByteBuf)` (:18-20): the anvil text box
/// as the client last had it, with `readUtf`'s default bound.
pub(super) fn decode_rename_item(decoder: &mut Decoder<'_>) -> Result<RenameItem> {
    Ok(RenameItem {
        name: decoder.read_string(MAX_RENAME_ITEM_UTF16_UNITS)?,
    })
}

pub fn encode_play_seen_advancements(packet: &SeenAdvancements) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.action_id());
    if let SeenAdvancements::OpenedTab { tab } = packet {
        out.write_string(tab);
    }
    (ids::play::SERVERBOUND_SEEN_ADVANCEMENTS, out.into_inner())
}

/// `ServerboundSeenAdvancementsPacket(FriendlyByteBuf)` (:31-38): the tab id
/// is on the wire only behind `OPENED_TAB`, and `readEnum` rejects an ordinal
/// outside the two rather than falling back.
pub(super) fn decode_seen_advancements(decoder: &mut Decoder<'_>) -> Result<SeenAdvancements> {
    let action_id = decoder.read_var_i32()?;
    match action_id {
        0 => Ok(SeenAdvancements::OpenedTab {
            tab: read_resource_location(decoder)?,
        }),
        1 => Ok(SeenAdvancements::ClosedScreen),
        other => Err(ProtocolError::InvalidData(format!(
            "invalid seen advancements action ordinal {other}"
        ))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectTradeCommand {
    pub item: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetBeacon {
    pub primary_effect: Option<i32>,
    pub secondary_effect: Option<i32>,
}

pub fn encode_play_select_trade(command: SelectTradeCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.item);
    (ids::play::SERVERBOUND_SELECT_TRADE, out.into_inner())
}

/// `ServerboundSelectTradePacket(FriendlyByteBuf)` (:22-24): the offer index the
/// merchant screen highlighted, which the menu bounds-checks rather than the
/// reader.
pub(super) fn decode_select_trade(decoder: &mut Decoder<'_>) -> Result<SelectTradeCommand> {
    Ok(SelectTradeCommand {
        item: decoder.read_var_i32()?,
    })
}

pub fn encode_play_set_beacon(packet: SetBeacon) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    encode_optional_mob_effect(&mut out, packet.primary_effect)?;
    encode_optional_mob_effect(&mut out, packet.secondary_effect)?;
    Ok((ids::play::SERVERBOUND_SET_BEACON, out.into_inner()))
}

/// `ServerboundSetBeaconPacket.STREAM_CODEC` (:13-19): two optional mob effect
/// holders, each a registry id the reader validates.
pub(super) fn decode_set_beacon(decoder: &mut Decoder<'_>) -> Result<SetBeacon> {
    Ok(SetBeacon {
        primary_effect: decode_optional_mob_effect(decoder)?,
        secondary_effect: decode_optional_mob_effect(decoder)?,
    })
}

pub fn encode_play_select_bundle_item(packet: SelectBundleItem) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.slot_id);
    out.write_var_i32(packet.selected_item_index);
    (
        ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED,
        out.into_inner(),
    )
}

/// `ServerboundSelectBundleItemPacket(FriendlyByteBuf)` (:13-19): the reader
/// itself rejects a negative index other than the `NO_SELECTED_ITEM_INDEX`
/// sentinel (`BundleContents.NO_SELECTED_ITEM_INDEX`, :34).
pub(super) fn decode_select_bundle_item(decoder: &mut Decoder<'_>) -> Result<SelectBundleItem> {
    let slot_id = decoder.read_var_i32()?;
    let selected_item_index = decoder.read_var_i32()?;
    if selected_item_index < 0 && selected_item_index != NO_SELECTED_BUNDLE_ITEM_INDEX {
        return Err(ProtocolError::InvalidData(format!(
            "invalid selectedItemIndex: {selected_item_index}"
        )));
    }
    Ok(SelectBundleItem {
        slot_id,
        selected_item_index,
    })
}

fn decode_optional_mob_effect(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    let effect_id = decoder.read_var_i32()?;
    super::client_state::validate_mob_effect_holder_id(effect_id)?;
    Ok(Some(effect_id))
}

fn encode_optional_mob_effect(out: &mut Encoder, effect_id: Option<i32>) -> Result<()> {
    match effect_id {
        Some(effect_id) => {
            super::client_state::validate_mob_effect_holder_id(effect_id)?;
            out.write_bool(true);
            out.write_var_i32(effect_id);
        }
        None => out.write_bool(false),
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAbilitiesCommand {
    pub flying: bool,
}

pub fn encode_play_player_abilities(command: PlayerAbilitiesCommand) -> (i32, Vec<u8>) {
    let mut flags = 0u8;
    if command.flying {
        flags |= 0x02;
    }
    let mut out = Encoder::new();
    out.write_u8(flags);
    (ids::play::SERVERBOUND_PLAYER_ABILITIES, out.into_inner())
}

pub fn encode_play_player_command(command: PlayerCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.entity_id);
    out.write_var_i32(command.action.ordinal());
    out.write_var_i32(command.data);
    (ids::play::SERVERBOUND_PLAYER_COMMAND, out.into_inner())
}

pub fn encode_play_player_action(action: PlayerAction) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(action.action.ordinal());
    out.write_i64(chunks::encode_block_pos(action.pos));
    out.write_u8(action.direction.id());
    out.write_var_i32(action.sequence);
    (ids::play::SERVERBOUND_PLAYER_ACTION, out.into_inner())
}

pub fn encode_play_chat_command<S: Clone + Into<crate::JavaString>>(
    packet: &ChatCommand<S>,
) -> (i32, Vec<u8>) {
    let command: crate::JavaString = packet.command.clone().into();
    assert!(
        command.units().len() <= MAX_CHAT_COMMAND_CHARS,
        "chat command exceeds vanilla maximum of 32767 UTF-16 units"
    );
    let mut out = Encoder::new();
    out.write_utf16_string(command.units());
    (ids::play::SERVERBOUND_CHAT_COMMAND, out.into_inner())
}

pub fn encode_play_chat_command_signed<S: Clone + Into<crate::JavaString>>(
    packet: &ChatCommandSigned<S>,
) -> (i32, Vec<u8>) {
    let command: crate::JavaString = packet.command.clone().into();
    assert!(
        command.units().len() <= MAX_CHAT_COMMAND_CHARS,
        "chat command exceeds vanilla maximum of 32767 UTF-16 units"
    );
    let mut out = Encoder::new();
    out.write_utf16_string(command.units());
    out.write_i64(packet.timestamp_millis);
    out.write_i64(packet.salt);
    encode_argument_signatures(&mut out, &packet.argument_signatures);
    encode_last_seen_messages_update(&mut out, packet.last_seen_messages);
    (ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED, out.into_inner())
}

pub fn encode_play_chat_message<S: Clone + Into<crate::JavaString>>(
    packet: &ChatMessage<S>,
) -> (i32, Vec<u8>) {
    let message: crate::JavaString = packet.message.clone().into();
    assert!(
        message.units().len() <= MAX_CHAT_MESSAGE_CHARS,
        "chat message exceeds vanilla maximum of 256 characters"
    );

    let mut out = Encoder::new();
    out.write_utf16_string(message.units());
    out.write_i64(packet.timestamp_millis);
    out.write_i64(packet.salt);
    match packet.signature.as_ref() {
        Some(signature) => {
            assert!(
                signature.bytes.len() == MESSAGE_SIGNATURE_BYTES,
                "chat message signature must be exactly 256 bytes"
            );
            out.write_bool(true);
            out.write_bytes(&signature.bytes);
        }
        None => out.write_bool(false),
    }
    encode_last_seen_messages_update(&mut out, packet.last_seen_messages);
    (ids::play::SERVERBOUND_CHAT, out.into_inner())
}

pub fn encode_play_chat_session_update(packet: &ChatSessionUpdate) -> (i32, Vec<u8>) {
    assert!(
        packet.public_key.len() <= MAX_PROFILE_PUBLIC_KEY_BYTES,
        "profile public key exceeds vanilla maximum of 512 bytes"
    );
    assert!(
        packet.key_signature.len() <= MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES,
        "profile public key signature exceeds vanilla maximum of 4096 bytes"
    );

    let mut out = Encoder::new();
    out.write_uuid(packet.session_id);
    out.write_i64(packet.expires_at_epoch_millis);
    out.write_var_i32(packet.public_key.len() as i32);
    out.write_bytes(&packet.public_key);
    out.write_var_i32(packet.key_signature.len() as i32);
    out.write_bytes(&packet.key_signature);
    (ids::play::SERVERBOUND_CHAT_SESSION_UPDATE, out.into_inner())
}

fn encode_argument_signatures(out: &mut Encoder, signatures: &ArgumentSignatures) {
    assert!(
        signatures.entries.len() <= MAX_ARGUMENT_SIGNATURE_COUNT,
        "argument signatures exceed vanilla maximum of 8 entries"
    );

    out.write_var_i32(signatures.entries.len() as i32);
    for entry in &signatures.entries {
        assert!(
            entry.name.encode_utf16().count() <= MAX_ARGUMENT_SIGNATURE_NAME_CHARS,
            "argument signature name exceeds vanilla maximum of 16 characters"
        );
        assert!(
            entry.signature.bytes.len() == MESSAGE_SIGNATURE_BYTES,
            "argument signature must be exactly 256 bytes"
        );
        out.write_string(&entry.name);
        out.write_bytes(&entry.signature.bytes);
    }
}

fn encode_last_seen_messages_update(out: &mut Encoder, update: LastSeenMessagesUpdate) {
    assert!(
        update.acknowledged < (1 << LAST_SEEN_MESSAGES_BITSET_BITS),
        "last seen acknowledgement bitset exceeds 20 bits"
    );

    out.write_var_i32(update.offset);
    out.write_bytes(&update.acknowledged.to_le_bytes()[..LAST_SEEN_MESSAGES_BITSET_BYTES]);
    out.write_u8(update.checksum);
}

pub fn encode_play_attack_entity(packet: AttackEntity) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.entity_id);
    (ids::play::SERVERBOUND_ATTACK, out.into_inner())
}

pub fn encode_play_interact_entity(packet: InteractEntity) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.entity_id);
    out.write_var_i32(packet.hand.id());
    super::entities::encode_lp_vec3d(&mut out, packet.location);
    out.write_bool(packet.using_secondary_action);
    (ids::play::SERVERBOUND_INTERACT, out.into_inner())
}

pub fn encode_play_swing(hand: InteractionHand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(hand.id());
    (ids::play::SERVERBOUND_SWING, out.into_inner())
}

pub fn encode_play_use_item_on(packet: UseItemOn) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.hand.id());
    chunks::encode_block_hit_result(&mut out, packet.hit);
    out.write_var_i32(packet.sequence);
    (ids::play::SERVERBOUND_USE_ITEM_ON, out.into_inner())
}

pub fn encode_play_use_item(packet: UseItem) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.hand.id());
    out.write_var_i32(packet.sequence);
    out.write_f32(packet.y_rot);
    out.write_f32(packet.x_rot);
    (ids::play::SERVERBOUND_USE_ITEM, out.into_inner())
}

pub fn encode_play_pick_item_from_block(packet: PickItemFromBlock) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_bool(packet.include_data);
    (
        ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK,
        out.into_inner(),
    )
}

pub fn encode_play_pick_item_from_entity(packet: PickItemFromEntity) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.entity_id);
    out.write_bool(packet.include_data);
    (
        ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY,
        out.into_inner(),
    )
}

pub fn encode_play_sign_update(packet: &SignUpdate) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_bool(packet.is_front_text);
    for line in &packet.lines {
        out.write_string(line);
    }
    (ids::play::SERVERBOUND_SIGN_UPDATE, out.into_inner())
}

/// `ServerboundSignUpdatePacket(FriendlyByteBuf)` (:26-34): the position, the
/// side, then exactly four lines with the packet's own bound.
pub(super) fn decode_sign_update(decoder: &mut Decoder<'_>) -> Result<SignUpdate> {
    let pos = chunks::decode_block_pos(decoder.read_i64()?);
    let is_front_text = decoder.read_bool()?;
    let mut lines: [String; 4] = Default::default();
    for line in &mut lines {
        *line = decoder.read_string(MAX_SIGN_UPDATE_LINE_UTF16_UNITS)?;
    }
    Ok(SignUpdate {
        pos,
        is_front_text,
        lines,
    })
}

pub fn encode_play_spectate_entity(packet: SpectateEntity) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.entity_id);
    (ids::play::SERVERBOUND_SPECTATE_ENTITY, out.into_inner())
}

pub fn encode_play_teleport_to_entity(packet: TeleportToEntity) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_uuid(packet.uuid);
    (ids::play::SERVERBOUND_TELEPORT_TO_ENTITY, out.into_inner())
}

pub fn encode_play_container_button_click(packet: ContainerButtonClick) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.container_id);
    out.write_var_i32(packet.button_id);
    (
        ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK,
        out.into_inner(),
    )
}

pub fn encode_play_container_click(packet: ContainerClick) -> Result<(i32, Vec<u8>)> {
    encode_play_container_click_with_content_registry(packet, ContentRegistry::vanilla())
}

pub fn encode_play_container_click_with_content_registry(
    packet: ContainerClick,
    registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    if packet.changed_slots.len() > MAX_CONTAINER_CLICK_CHANGED_SLOTS {
        return Err(ProtocolError::InvalidData(format!(
            "container click changed slot count {} exceeds vanilla maximum {MAX_CONTAINER_CLICK_CHANGED_SLOTS}",
            packet.changed_slots.len()
        )));
    }
    for stack in packet
        .changed_slots
        .values()
        .chain(std::iter::once(&packet.carried_item))
    {
        validate_hashed_stack(stack, registry)?;
    }

    let mut out = Encoder::new();
    out.write_var_i32(packet.container_id);
    out.write_var_i32(packet.state_id);
    out.write_i16(packet.slot_num);
    out.write_i8(packet.button_num);
    out.write_var_i32(packet.input.id());
    out.write_var_i32(packet.changed_slots.len() as i32);
    for (slot, stack) in &packet.changed_slots {
        out.write_i16(*slot);
        encode_hashed_stack(&mut out, stack);
    }
    encode_hashed_stack(&mut out, &packet.carried_item);
    Ok((ids::play::SERVERBOUND_CONTAINER_CLICK, out.into_inner()))
}

pub fn encode_play_container_close(packet: ContainerCloseRequest) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.container_id);
    (ids::play::SERVERBOUND_CONTAINER_CLOSE, out.into_inner())
}

pub fn encode_play_container_slot_state_changed(
    packet: ContainerSlotStateChanged,
) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.slot_id);
    out.write_var_i32(packet.container_id);
    out.write_bool(packet.new_state);
    (
        ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED,
        out.into_inner(),
    )
}

fn encode_hashed_stack(out: &mut Encoder, stack: &HashedStack) {
    match stack {
        HashedStack::Empty => out.write_bool(false),
        HashedStack::Item(stack) => {
            out.write_bool(true);
            out.write_var_i32(stack.item_id);
            out.write_var_i32(stack.count);
            encode_hashed_component_patch(out, &stack.components);
        }
    }
}

fn validate_hashed_stack(stack: &HashedStack, registry: &ContentRegistry) -> Result<()> {
    let HashedStack::Item(stack) = stack else {
        return Ok(());
    };
    super::inventory::validate_item_holder_id_with_registry(stack.item_id, registry)?;
    if stack.components.added_components.len() > MAX_HASHED_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::InvalidData(format!(
            "hashed component added count {} exceeds vanilla maximum {MAX_HASHED_COMPONENT_PATCH_ENTRIES}",
            stack.components.added_components.len()
        )));
    }
    if stack.components.removed_components.len() > MAX_HASHED_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::InvalidData(format!(
            "hashed component removed count {} exceeds vanilla maximum {MAX_HASHED_COMPONENT_PATCH_ENTRIES}",
            stack.components.removed_components.len()
        )));
    }
    Ok(())
}

fn encode_hashed_component_patch(out: &mut Encoder, patch: &HashedComponentPatch) {
    out.write_var_i32(patch.added_components.len() as i32);
    for (component_type_id, hash) in &patch.added_components {
        out.write_var_i32(*component_type_id);
        out.write_i32(*hash);
    }

    out.write_var_i32(patch.removed_components.len() as i32);
    for component_type_id in &patch.removed_components {
        out.write_var_i32(*component_type_id);
    }
}

pub(super) fn encode_item_stack_summary_with_content_registry(
    out: &mut Encoder,
    item: &ItemStackSummary,
    framing: data_components::ComponentValueFraming,
    registry: &ContentRegistry,
) -> Result<()> {
    let Some(item_id) = item.item_id else {
        out.write_var_i32(0);
        return Ok(());
    };
    if item.count <= 0 {
        out.write_var_i32(0);
        return Ok(());
    }
    super::inventory::validate_item_holder_id_with_registry(item_id, registry)?;
    if item_id == VANILLA_ITEM_AIR_ID {
        out.write_var_i32(0);
        return Ok(());
    }
    if item.count > VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE {
        return Err(ProtocolError::InvalidData(
            format!(
                "creative mode item stack count {} exceeds Vanilla persistent maximum {VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE}",
                item.count
            ),
        ));
    }
    out.write_var_i32(item.count);
    out.write_var_i32(item_id);
    data_components::encode_data_component_patch_summary_with_framing_and_registry(
        out,
        &item.component_patch,
        framing,
        registry,
    )?;
    Ok(())
}

/// Creative packets validate persistent component ranges before entering live state
/// (`ItemStack.java:197-204`).
pub(super) fn validate_creative_item_component_ranges(
    patch: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    if patch.added_type_ids.is_empty() && patch.removed_type_ids.is_empty() {
        return Ok(());
    }

    let mut pending = vec![(patch, 0usize)];
    while let Some((patch, predicate_depth)) = pending.pop() {
        if patch
            .max_stack_size
            .is_some_and(|value| !(1..=VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE).contains(&value))
        {
            return Err(ProtocolError::InvalidData(
                "creative mode max_stack_size must be in 1..=99".to_string(),
            ));
        }
        if patch.max_damage.is_some_and(|value| value <= 0) {
            return Err(ProtocolError::InvalidData(
                "creative mode max_damage must be positive".to_string(),
            ));
        }
        if patch.damage.is_some_and(|value| value < 0) {
            return Err(ProtocolError::InvalidData(
                "creative mode damage must be non-negative".to_string(),
            ));
        }
        if patch.repair_cost.is_some_and(|value| value < 0) {
            return Err(ProtocolError::InvalidData(
                "creative mode repair_cost must be non-negative".to_string(),
            ));
        }
        if patch.use_effects.is_some_and(|effects| {
            effects.speed_multiplier.total_cmp(&0.0).is_lt()
                || effects.speed_multiplier.total_cmp(&1.0).is_gt()
        }) {
            return Err(ProtocolError::InvalidData(
                "creative mode use_effects speed_multiplier must be in 0.0..=1.0".to_string(),
            ));
        }
        if patch.minimum_attack_charge_bits.is_some_and(|bits| {
            let value = f32::from_bits(bits);
            value.total_cmp(&0.0).is_lt() || value.total_cmp(&1.0).is_gt()
        }) {
            return Err(ProtocolError::InvalidData(
                "creative mode minimum_attack_charge must be in 0.0..=1.0".to_string(),
            ));
        }
        for (what, enchantments) in [
            ("enchantments", patch.enchantments.as_slice()),
            ("stored_enchantments", patch.stored_enchantments.as_slice()),
        ] {
            if enchantments
                .iter()
                .any(|enchantment| !(1..=u8::MAX.into()).contains(&enchantment.level))
            {
                return Err(ProtocolError::InvalidData(format!(
                    "creative mode {what} levels must be in 1..=255"
                )));
            }
        }
        if patch
            .fireworks_flight_duration
            .is_some_and(|value| !(0..=u8::MAX.into()).contains(&value))
        {
            return Err(ProtocolError::InvalidData(
                "creative mode fireworks flight_duration must be in 0..=255".to_string(),
            ));
        }
        if patch
            .potion_duration_scale_bits
            .is_some_and(|bits| !is_non_negative_finite_f32(bits))
        {
            return Err(ProtocolError::InvalidData(
                "creative mode potion_duration_scale must be in 0.0..=Float.MAX_VALUE".to_string(),
            ));
        }
        if patch.food_nutrition.is_some_and(|value| value < 0) {
            return Err(ProtocolError::InvalidData(
                "creative mode food nutrition must be non-negative".to_string(),
            ));
        }
        if patch
            .food_saturation_bits
            .is_some_and(|bits| !f32::from_bits(bits).is_finite())
        {
            return Err(ProtocolError::InvalidData(
                "creative mode food saturation must be finite".to_string(),
            ));
        }
        if let Some(consumable) = patch.consumable {
            if !consumable.consume_seconds.is_finite()
                || consumable.consume_seconds.total_cmp(&0.0).is_lt()
            {
                return Err(ProtocolError::InvalidData(
                    "creative mode consumable consume_seconds must be non-negative and finite"
                        .to_string(),
                ));
            }
        }
        if let Some(sound) = patch.consumable_sound.as_ref() {
            validate_creative_sound_event(sound, registry)?;
        }
        for effect in patch
            .consumable_effects
            .iter()
            .chain(&patch.death_protection_effects)
        {
            validate_creative_consume_effect(effect, registry)?;
        }
        if let Some(bits) = patch.use_cooldown_seconds_bits {
            if !is_positive_finite_f32(bits) {
                return Err(ProtocolError::InvalidData(
                    "creative mode use_cooldown seconds must be positive and finite".to_string(),
                ));
            }
        }
        if let Some(tool) = patch.tool.as_ref() {
            if tool.damage_per_block < 0 {
                return Err(ProtocolError::InvalidData(
                    "creative mode tool damage_per_block must be non-negative".to_string(),
                ));
            }
            if tool.rules.iter().any(|rule| {
                rule.speed_bits
                    .is_some_and(|bits| !is_positive_finite_f32(bits))
            }) {
                return Err(ProtocolError::InvalidData(
                    "creative mode tool rule speed must be positive and finite".to_string(),
                ));
            }
        }
        if patch
            .weapon_item_damage_per_attack
            .is_some_and(|value| value < 0)
        {
            return Err(ProtocolError::InvalidData(
                "creative mode weapon item_damage_per_attack must be non-negative".to_string(),
            ));
        }
        if patch
            .weapon_disable_blocking_seconds_bits
            .is_some_and(|bits| !is_non_negative_finite_f32(bits))
        {
            return Err(ProtocolError::InvalidData(
                "creative mode weapon disable_blocking_for_seconds must be non-negative and finite"
                    .to_string(),
            ));
        }
        if patch.enchantable_value.is_some_and(|value| value <= 0) {
            return Err(ProtocolError::InvalidData(
                "creative mode enchantable value must be positive".to_string(),
            ));
        }
        if let Some(equippable) = patch.equippable.as_ref() {
            validate_creative_sound_event(&equippable.equip_sound, registry)?;
            validate_creative_sound_event(&equippable.shearing_sound, registry)?;
        }
        if let Some(blocks_attacks) = patch.blocks_attacks.as_ref() {
            if !is_non_negative_finite_f32(blocks_attacks.block_delay_seconds_bits)
                || !is_non_negative_finite_f32(blocks_attacks.disable_cooldown_scale_bits)
                || !is_non_negative_finite_f32(blocks_attacks.item_damage.threshold_bits)
                || blocks_attacks
                    .damage_reductions
                    .iter()
                    .any(|r| !is_positive_finite_f32(r.horizontal_blocking_angle_bits))
            {
                return Err(ProtocolError::InvalidData(
                    "creative mode blocks_attacks numeric field is out of persistent range"
                        .to_string(),
                ));
            }
            if let Some(sound) = blocks_attacks.block_sound.as_ref() {
                validate_creative_sound_event(sound, registry)?;
            }
            if let Some(sound) = blocks_attacks.disable_sound.as_ref() {
                validate_creative_sound_event(sound, registry)?;
            }
        }
        if let Some(piercing) = patch.piercing_weapon.as_ref() {
            if let Some(sound) = piercing.sound.as_ref() {
                validate_creative_sound_event(sound, registry)?;
            }
            if let Some(sound) = piercing.hit_sound.as_ref() {
                validate_creative_sound_event(sound, registry)?;
            }
        }
        if let Some(kinetic) = patch.kinetic_weapon.as_ref() {
            if kinetic.contact_cooldown_ticks < 0
                || kinetic.delay_ticks < 0
                || [
                    kinetic.dismount_conditions,
                    kinetic.knockback_conditions,
                    kinetic.damage_conditions,
                ]
                .into_iter()
                .flatten()
                .any(|cond| cond.max_duration_ticks < 0)
            {
                return Err(ProtocolError::InvalidData(
                    "creative mode kinetic_weapon ticks must be non-negative".to_string(),
                ));
            }
            if let Some(sound) = kinetic.sound.as_ref() {
                validate_creative_sound_event(sound, registry)?;
            }
            if let Some(sound) = kinetic.hit_sound.as_ref() {
                validate_creative_sound_event(sound, registry)?;
            }
        }
        if patch
            .swing_animation
            .is_some_and(|swing| swing.duration <= 0)
        {
            return Err(ProtocolError::InvalidData(
                "creative mode swing_animation duration must be positive".to_string(),
            ));
        }
        if let Some(sound) = patch.break_sound.as_ref() {
            validate_creative_sound_event(sound, registry)?;
        }
        if let Some(song) = patch.jukebox_direct_song.as_deref() {
            validate_creative_sound_event(&song.sound_event, registry)?;
        }
        if let Some(instrument) = patch.instrument.direct.as_deref() {
            validate_creative_sound_event(&instrument.sound_event, registry)?;
        }
        if let Some(painting) = patch.painting_variant_direct.as_ref() {
            if !(1..=16).contains(&painting.width) || !(1..=16).contains(&painting.height) {
                return Err(ProtocolError::InvalidData(
                    "creative mode painting variant dimensions must be in 1..=16".to_string(),
                ));
            }
        }
        if let Some(book) = patch.written_book.as_ref() {
            if !(0..=3).contains(&book.generation) {
                return Err(ProtocolError::InvalidData(
                    "creative mode written_book generation must be in 0..=3".to_string(),
                ));
            }
        }
        if let Some(profile) = patch.profile.as_ref() {
            if let Some(name) = profile.name.as_deref() {
                if !bbb_facts::player_names::valid_player_name(name) {
                    return Err(ProtocolError::InvalidData(
                        "creative mode profile player name contains disallowed characters"
                            .to_string(),
                    ));
                }
            }
        }

        for (what, predicate) in [
            ("can_place_on", patch.can_place_on_persistent.as_ref()),
            ("can_break", patch.can_break_persistent.as_ref()),
        ] {
            if let Some(predicate) = predicate {
                validate_creative_adventure_mode_predicate(
                    what,
                    predicate,
                    predicate_depth,
                    &mut pending,
                )?;
            }
        }

        if let Some(template) = patch.use_remainder.as_deref() {
            validate_creative_item_stack_template_count(template.count)?;
            pending.push((&template.component_patch, 0));
        }
        for template in &patch.charged_projectiles_items {
            validate_creative_item_stack_template_count(template.count)?;
            pending.push((&template.component_patch, 0));
        }
        for template in &patch.bundle_contents_items {
            validate_creative_item_stack_template_count(template.count)?;
            pending.push((&template.component_patch, 0));
        }
        for template in patch.container.items() {
            validate_creative_item_stack_template_count(template.count)?;
            pending.push((&template.component_patch, 0));
        }
    }
    data_components::validate_persistent_data_component_patch_summary_with_registry(
        patch, registry,
    )?;
    Ok(())
}

fn is_non_negative_finite_f32(bits: u32) -> bool {
    let value = f32::from_bits(bits);
    value.is_finite() && value.total_cmp(&0.0).is_ge()
}

fn is_positive_finite_f32(bits: u32) -> bool {
    let value = f32::from_bits(bits);
    value.is_finite() && value > 0.0
}

fn validate_creative_sound_event(
    sound: &data_components::SoundEventSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    if let Some(id) = sound.registry_id {
        if crate::sound_event_types::vanilla_sound_event_resource_id(id).is_none()
            && registry.sound_event_by_id(id).is_none()
        {
            return Err(ProtocolError::InvalidData(format!(
                "creative mode sound_event registry id {id} is unknown"
            )));
        }
    }
    Ok(())
}

fn validate_creative_consume_effect(
    effect: &data_components::ConsumeEffectSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    match effect {
        data_components::ConsumeEffectSummary::ApplyEffects {
            probability_bits, ..
        } => {
            let p = f32::from_bits(*probability_bits);
            if !p.is_finite() || p.total_cmp(&0.0).is_lt() || p.total_cmp(&1.0).is_gt() {
                return Err(ProtocolError::InvalidData(
                    "creative mode apply_effects probability must be in 0.0..=1.0".to_string(),
                ));
            }
        }
        data_components::ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
            if !is_positive_finite_f32(*diameter_bits) {
                return Err(ProtocolError::InvalidData(
                    "creative mode teleport_randomly diameter must be positive and finite"
                        .to_string(),
                ));
            }
        }
        data_components::ConsumeEffectSummary::PlaySound { sound } => {
            validate_creative_sound_event(sound, registry)?;
        }
        data_components::ConsumeEffectSummary::RemoveEffects { .. }
        | data_components::ConsumeEffectSummary::ClearAllEffects => {}
    }
    Ok(())
}

fn validate_creative_item_stack_template_count(count: i32) -> Result<()> {
    if (1..=VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE).contains(&count) {
        Ok(())
    } else {
        Err(ProtocolError::InvalidData(format!(
            "creative mode nested item stack count {count} must be in 1..=99"
        )))
    }
}

fn validate_creative_adventure_mode_predicate<'a>(
    what: &str,
    predicate: &'a AdventureModePredicateDataSummary,
    depth: usize,
    pending: &mut Vec<(&'a DataComponentPatchSummary, usize)>,
) -> Result<()> {
    if depth >= data_components::MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return Err(ProtocolError::InvalidData(format!(
            "creative mode adventure predicate nesting exceeds {}",
            data_components::MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH
        )));
    }
    if predicate.predicates.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "creative mode {what} predicate list must not be empty"
        )));
    }
    for block in &predicate.predicates {
        if let Some(properties) = block.state.as_ref() {
            let mut names = BTreeSet::new();
            for property in properties {
                if !names.insert(&property.name) {
                    return Err(ProtocolError::InvalidData(format!(
                        "creative mode {what} contains duplicate state property `{}`",
                        property.name.to_display_string()
                    )));
                }
            }
        }
        if let Some(exact) = block.exact_components.as_deref() {
            pending.push((exact, depth + 1));
        }
    }
    Ok(())
}

pub fn encode_play_set_carried_item(slot: i16) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i16(slot);
    (ids::play::SERVERBOUND_SET_CARRIED_ITEM, out.into_inner())
}

pub fn encode_play_set_creative_mode_slot(packet: &SetCreativeModeSlot) -> Result<(i32, Vec<u8>)> {
    encode_play_set_creative_mode_slot_with_content_registry(packet, ContentRegistry::vanilla())
}

pub fn encode_play_set_creative_mode_slot_with_content_registry(
    packet: &SetCreativeModeSlot,
    registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    if packet.item.count > 0
        && packet
            .item
            .item_id
            .is_some_and(|item_id| item_id != VANILLA_ITEM_AIR_ID)
    {
        validate_creative_item_component_ranges(&packet.item.component_patch, registry)?;
    }
    let mut out = Encoder::new();
    out.write_i16(packet.slot_num);
    encode_item_stack_summary_with_content_registry(
        &mut out,
        &packet.item,
        data_components::ComponentValueFraming::LengthPrefixed,
        registry,
    )?;
    Ok((
        ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT,
        out.into_inner(),
    ))
}

pub fn encode_play_chunk_batch_received(desired_chunks_per_tick: f32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f32(desired_chunks_per_tick);
    (
        ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED,
        out.into_inner(),
    )
}

pub fn encode_play_client_command(action: ClientCommandAction) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(action.ordinal());
    (ids::play::SERVERBOUND_CLIENT_COMMAND, out.into_inner())
}

pub fn encode_play_perform_respawn() -> (i32, Vec<u8>) {
    encode_play_client_command(ClientCommandAction::PerformRespawn)
}

pub fn encode_play_configuration_acknowledged() -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED,
        Vec::new(),
    )
}
