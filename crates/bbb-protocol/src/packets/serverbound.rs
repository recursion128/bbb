use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::client_command_actions::ClientCommandAction;
pub use crate::container_inputs::ContainerInput;
use crate::debug_subscription_types::vanilla_debug_subscription_type_by_id;
pub use crate::interaction_hands::InteractionHand;
pub use crate::player_action_kinds::PlayerActionKind;
pub use crate::player_command_actions::PlayerCommandAction;
use crate::{
    codec::{Encoder, ProtocolError, Result},
    game_rules::{vanilla_game_rule, VANILLA_GAME_RULES},
    ids,
    item_types::{VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE, VANILLA_ITEM_AIR_ID},
};

use super::client_features::{RecipeBookType, RecipeDisplayId};
use super::client_state::Difficulty;
use super::inventory::{DataComponentPatchSummary, ItemStackSummary};
use super::player_info::{GameType, PlayerInfoChatSession};
use super::{
    chunks,
    client_common::{self, CustomClickAction},
    connection, data_components, normalize_resource_location, BlockPos, MessageSignature, Vec3d,
};

const PLAYER_INPUT_FORWARD: u8 = 1;
const PLAYER_INPUT_BACKWARD: u8 = 2;
const PLAYER_INPUT_LEFT: u8 = 4;
const PLAYER_INPUT_RIGHT: u8 = 8;
const PLAYER_INPUT_JUMP: u8 = 16;
const PLAYER_INPUT_SHIFT: u8 = 32;
const PLAYER_INPUT_SPRINT: u8 = 64;
const LAST_SEEN_MESSAGES_BITSET_BITS: u32 = 20;
const LAST_SEEN_MESSAGES_BITSET_BYTES: usize = 3;
const EMPTY_LAST_SEEN_MESSAGES_CHECKSUM: u8 = 1;
const MESSAGE_SIGNATURE_BYTES: usize = 256;
const MAX_ARGUMENT_SIGNATURE_COUNT: usize = 8;
const MAX_ARGUMENT_SIGNATURE_NAME_CHARS: usize = 16;
const MAX_CHAT_MESSAGE_CHARS: usize = 256;
const MAX_CONTAINER_CLICK_CHANGED_SLOTS: usize = 128;
const MAX_HASHED_COMPONENT_PATCH_ENTRIES: usize = 256;
const MAX_GAME_RULE_VALUE_UTF16_UNITS: usize = 32767;
const MAX_PROFILE_PUBLIC_KEY_BYTES: usize = 512;
const MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES: usize = 4096;
const MAX_SERVERBOUND_CUSTOM_PAYLOAD: usize = 32767;
const MAX_CUSTOM_PAYLOAD_STRING_UTF16_UNITS: usize = 32767;
const MAX_EDIT_BOOK_PAGES: usize = 100;
const MAX_EDIT_BOOK_PAGE_UTF16_UNITS: usize = 1024;
const MAX_EDIT_BOOK_TITLE_UTF16_UNITS: usize = 32;
const LP_VEC3_ABS_MAX_VALUE: f64 = 1.7179869183E10;
const LP_VEC3_ABS_MIN_VALUE: f64 = 3.051944088384301E-5;
const LP_VEC3_MAX_QUANTIZED_VALUE: f64 = 32766.0;

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
pub struct ChatCommand {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCommandSigned {
    pub command: String,
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
pub struct ChatMessage {
    pub message: String,
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
pub struct EditBook {
    pub slot: i32,
    pub pages: Vec<String>,
    pub title: Option<String>,
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

pub fn encode_play_move_vehicle(
    x: f64,
    y: f64,
    z: f64,
    y_rot: f32,
    x_rot: f32,
    on_ground: bool,
) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f64(x);
    out.write_f64(y);
    out.write_f64(z);
    out.write_f32(y_rot);
    out.write_f32(x_rot);
    out.write_bool(on_ground);
    (ids::play::SERVERBOUND_MOVE_VEHICLE, out.into_inner())
}

pub fn encode_play_paddle_boat(packet: PaddleBoat) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_bool(packet.left);
    out.write_bool(packet.right);
    (ids::play::SERVERBOUND_PADDLE_BOAT, out.into_inner())
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

pub fn encode_play_recipe_book_seen_recipe(command: RecipeBookSeenRecipeCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.recipe.index);
    (
        ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE,
        out.into_inner(),
    )
}

pub fn encode_play_edit_book(packet: &EditBook) -> Result<(i32, Vec<u8>)> {
    if packet.pages.len() > MAX_EDIT_BOOK_PAGES {
        return Err(ProtocolError::InvalidData(format!(
            "edit book page count {} exceeds vanilla maximum {MAX_EDIT_BOOK_PAGES}",
            packet.pages.len()
        )));
    }
    for (index, page) in packet.pages.iter().enumerate() {
        if page.encode_utf16().count() > MAX_EDIT_BOOK_PAGE_UTF16_UNITS {
            return Err(ProtocolError::InvalidData(format!(
                "edit book page {index} exceeds vanilla maximum of {MAX_EDIT_BOOK_PAGE_UTF16_UNITS} UTF-16 code units"
            )));
        }
    }
    if packet
        .title
        .as_ref()
        .is_some_and(|title| title.encode_utf16().count() > MAX_EDIT_BOOK_TITLE_UTF16_UNITS)
    {
        return Err(ProtocolError::InvalidData(format!(
            "edit book title exceeds vanilla maximum of {MAX_EDIT_BOOK_TITLE_UTF16_UNITS} UTF-16 code units"
        )));
    }

    let mut out = Encoder::new();
    out.write_var_i32(packet.slot);
    out.write_var_i32(packet.pages.len() as i32);
    for page in &packet.pages {
        out.write_string(page);
    }
    match &packet.title {
        Some(title) => {
            out.write_bool(true);
            out.write_string(title);
        }
        None => out.write_bool(false),
    }
    Ok((ids::play::SERVERBOUND_EDIT_BOOK, out.into_inner()))
}

pub fn encode_play_rename_item(packet: &RenameItem) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_string(&packet.name);
    (ids::play::SERVERBOUND_RENAME_ITEM, out.into_inner())
}

pub fn encode_play_seen_advancements(packet: &SeenAdvancements) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.action_id());
    if let SeenAdvancements::OpenedTab { tab } = packet {
        out.write_string(tab);
    }
    (ids::play::SERVERBOUND_SEEN_ADVANCEMENTS, out.into_inner())
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

pub fn encode_play_set_beacon(packet: SetBeacon) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    encode_optional_mob_effect(&mut out, packet.primary_effect)?;
    encode_optional_mob_effect(&mut out, packet.secondary_effect)?;
    Ok((ids::play::SERVERBOUND_SET_BEACON, out.into_inner()))
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

pub fn encode_play_chat_command(packet: &ChatCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_string(&packet.command);
    (ids::play::SERVERBOUND_CHAT_COMMAND, out.into_inner())
}

pub fn encode_play_chat_command_signed(packet: &ChatCommandSigned) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_string(&packet.command);
    out.write_i64(packet.timestamp_millis);
    out.write_i64(packet.salt);
    encode_argument_signatures(&mut out, &packet.argument_signatures);
    encode_last_seen_messages_update(&mut out, packet.last_seen_messages);
    (ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED, out.into_inner())
}

pub fn encode_play_chat_message(packet: &ChatMessage) -> (i32, Vec<u8>) {
    assert!(
        packet.message.chars().count() <= MAX_CHAT_MESSAGE_CHARS,
        "chat message exceeds vanilla maximum of 256 characters"
    );

    let mut out = Encoder::new();
    out.write_string(&packet.message);
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
            entry.name.chars().count() <= MAX_ARGUMENT_SIGNATURE_NAME_CHARS,
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
    encode_lp_vec3(&mut out, packet.location);
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
        validate_hashed_stack(stack)?;
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

fn validate_hashed_stack(stack: &HashedStack) -> Result<()> {
    let HashedStack::Item(stack) = stack else {
        return Ok(());
    };
    super::inventory::validate_item_holder_id(stack.item_id)?;
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

fn encode_item_stack_summary(out: &mut Encoder, item: &ItemStackSummary) -> Result<()> {
    let Some(item_id) = item.item_id else {
        out.write_var_i32(0);
        return Ok(());
    };
    if item.count <= 0 {
        out.write_var_i32(0);
        return Ok(());
    }
    super::inventory::validate_item_holder_id(item_id)?;
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
    validate_creative_item_component_ranges(&item.component_patch)?;

    out.write_var_i32(item.count);
    out.write_var_i32(item_id);
    data_components::encode_delimited_data_component_patch_summary(out, &item.component_patch)?;
    Ok(())
}

fn validate_creative_item_component_ranges(patch: &DataComponentPatchSummary) -> Result<()> {
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
    if patch
        .fireworks_flight_duration
        .is_some_and(|value| !(0..=u8::MAX.into()).contains(&value))
    {
        return Err(ProtocolError::InvalidData(
            "creative mode fireworks flight_duration must be in 0..=255".to_string(),
        ));
    }
    if patch.potion_duration_scale_bits.is_some_and(|bits| {
        let value = f32::from_bits(bits);
        !value.is_finite() || value.total_cmp(&0.0).is_lt()
    }) {
        return Err(ProtocolError::InvalidData(
            "creative mode potion_duration_scale must be in 0.0..=Float.MAX_VALUE".to_string(),
        ));
    }
    Ok(())
}

fn encode_lp_vec3(out: &mut Encoder, value: Vec3d) {
    let x = sanitize_lp_vec3_value(value.x);
    let y = sanitize_lp_vec3_value(value.y);
    let z = sanitize_lp_vec3_value(value.z);
    let chessboard_length = x.abs().max(y.abs()).max(z.abs());
    if chessboard_length < LP_VEC3_ABS_MIN_VALUE {
        out.write_u8(0);
        return;
    }

    let scale = chessboard_length.ceil() as u64;
    let is_partial = (scale & 3) != scale;
    let markers = if is_partial { (scale & 3) | 4 } else { scale };
    let buffer = markers
        | (pack_lp_vec3_component(x / scale as f64) << 3)
        | (pack_lp_vec3_component(y / scale as f64) << 18)
        | (pack_lp_vec3_component(z / scale as f64) << 33);

    out.write_u8(buffer as u8);
    out.write_u8((buffer >> 8) as u8);
    out.write_i32((buffer >> 16) as i32);
    if is_partial {
        out.write_var_i32((scale >> 2) as i32);
    }
}

fn sanitize_lp_vec3_value(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-LP_VEC3_ABS_MAX_VALUE, LP_VEC3_ABS_MAX_VALUE)
    }
}

fn pack_lp_vec3_component(value: f64) -> u64 {
    ((value * 0.5 + 0.5) * LP_VEC3_MAX_QUANTIZED_VALUE).round() as u64
}

pub fn encode_play_set_carried_item(slot: i16) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i16(slot);
    (ids::play::SERVERBOUND_SET_CARRIED_ITEM, out.into_inner())
}

pub fn encode_play_set_creative_mode_slot(packet: &SetCreativeModeSlot) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    out.write_i16(packet.slot_num);
    encode_item_stack_summary(&mut out, &packet.item)?;
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
