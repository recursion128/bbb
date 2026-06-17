use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{codec::Encoder, ids};

use super::client_features::{RecipeBookType, RecipeDisplayId};
use super::client_state::Difficulty;
use super::{chunks, connection, BlockPos, Vec3d};

const PLAYER_INPUT_FORWARD: u8 = 1;
const PLAYER_INPUT_BACKWARD: u8 = 2;
const PLAYER_INPUT_LEFT: u8 = 4;
const PLAYER_INPUT_RIGHT: u8 = 8;
const PLAYER_INPUT_JUMP: u8 = 16;
const PLAYER_INPUT_SHIFT: u8 = 32;
const PLAYER_INPUT_SPRINT: u8 = 64;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeDifficultyCommand {
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockDifficultyCommand {
    pub locked: bool,
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
pub enum ContainerInput {
    Pickup,
    QuickMove,
    Swap,
    Clone,
    Throw,
    QuickCraft,
    PickupAll,
}

impl ContainerInput {
    fn id(self) -> i32 {
        match self {
            Self::Pickup => 0,
            Self::QuickMove => 1,
            Self::Swap => 2,
            Self::Clone => 3,
            Self::Throw => 4,
            Self::QuickCraft => 5,
            Self::PickupAll => 6,
        }
    }
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
pub enum PlayerActionKind {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
    SwapItemWithOffhand,
    Stab,
}

impl PlayerActionKind {
    fn ordinal(self) -> i32 {
        match self {
            Self::StartDestroyBlock => 0,
            Self::AbortDestroyBlock => 1,
            Self::StopDestroyBlock => 2,
            Self::DropAllItems => 3,
            Self::DropItem => 4,
            Self::ReleaseUseItem => 5,
            Self::SwapItemWithOffhand => 6,
            Self::Stab => 7,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerCommandAction {
    StopSleeping,
    StartSprinting,
    StopSprinting,
    StartRidingJump,
    StopRidingJump,
    OpenInventory,
    StartFallFlying,
}

impl PlayerCommandAction {
    fn ordinal(self) -> i32 {
        match self {
            Self::StopSleeping => 0,
            Self::StartSprinting => 1,
            Self::StopSprinting => 2,
            Self::StartRidingJump => 3,
            Self::StopRidingJump => 4,
            Self::OpenInventory => 5,
            Self::StartFallFlying => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionHand {
    MainHand,
    OffHand,
}

impl InteractionHand {
    fn id(self) -> i32 {
        match self {
            Self::MainHand => 0,
            Self::OffHand => 1,
        }
    }
}

pub fn encode_play_keep_alive(id: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(id);
    (ids::play::SERVERBOUND_KEEP_ALIVE, out.into_inner())
}

pub fn encode_play_cookie_response(key: &str, payload: Option<&[u8]>) -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_COOKIE_RESPONSE,
        super::connection::encode_cookie_response_payload(key, payload),
    )
}

pub fn encode_play_pong(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i32(id);
    (ids::play::SERVERBOUND_PONG, out.into_inner())
}

pub fn encode_play_client_information_default() -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_CLIENT_INFORMATION,
        connection::encode_client_information_payload_default(),
    )
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

pub fn encode_play_lock_difficulty(command: LockDifficultyCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_bool(command.locked);
    (ids::play::SERVERBOUND_LOCK_DIFFICULTY, out.into_inner())
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

pub fn encode_play_edit_book(packet: &EditBook) -> (i32, Vec<u8>) {
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
    (ids::play::SERVERBOUND_EDIT_BOOK, out.into_inner())
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

pub fn encode_play_set_beacon(packet: SetBeacon) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    encode_optional_mob_effect(&mut out, packet.primary_effect);
    encode_optional_mob_effect(&mut out, packet.secondary_effect);
    (ids::play::SERVERBOUND_SET_BEACON, out.into_inner())
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

fn encode_optional_mob_effect(out: &mut Encoder, effect_id: Option<i32>) {
    match effect_id {
        Some(effect_id) => {
            out.write_bool(true);
            out.write_var_i32(effect_id);
        }
        None => out.write_bool(false),
    }
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

pub fn encode_play_container_button_click(packet: ContainerButtonClick) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.container_id);
    out.write_var_i32(packet.button_id);
    (
        ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK,
        out.into_inner(),
    )
}

pub fn encode_play_container_click(packet: ContainerClick) -> (i32, Vec<u8>) {
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
    (ids::play::SERVERBOUND_CONTAINER_CLICK, out.into_inner())
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

pub fn encode_play_chunk_batch_received(desired_chunks_per_tick: f32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f32(desired_chunks_per_tick);
    (
        ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED,
        out.into_inner(),
    )
}

pub fn encode_play_perform_respawn() -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(0);
    (ids::play::SERVERBOUND_CLIENT_COMMAND, out.into_inner())
}

pub fn encode_play_configuration_acknowledged() -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED,
        Vec::new(),
    )
}
