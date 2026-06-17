use bbb_protocol::packets::{
    self, AttackEntity, ChatCommand, CommandSuggestionRequest, ContainerButtonClick,
    ContainerClick, ContainerCloseRequest, ContainerSlotStateChanged, InteractEntity,
    InteractionHand, PaddleBoat, PickItemFromBlock, PickItemFromEntity, PlaceRecipeCommand,
    PlayerAbilitiesCommand, PlayerAction, PlayerCommand, PlayerInput, PlayerPositionState,
    RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, SelectBundleItem,
    SelectTradeCommand, SignUpdate, UseItem, UseItemOn, Vec3d,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerMoveCommand {
    pub state: PlayerPositionState,
    pub on_ground: bool,
    pub horizontal_collision: bool,
    #[serde(default)]
    pub force_position: bool,
}

impl PlayerMoveCommand {
    pub(crate) fn encode_packet_from(self, previous: PlayerPositionState) -> (i32, Vec<u8>) {
        let position_changed = self.force_position || self.state.position != previous.position;
        let rotation_changed =
            self.state.y_rot != previous.y_rot || self.state.x_rot != previous.x_rot;

        match (position_changed, rotation_changed) {
            (true, true) => packets::encode_play_move_player_pos_rot(
                self.state.position.x,
                self.state.position.y,
                self.state.position.z,
                self.state.y_rot,
                self.state.x_rot,
                self.on_ground,
                self.horizontal_collision,
            ),
            (true, false) => packets::encode_play_move_player_pos(
                self.state.position.x,
                self.state.position.y,
                self.state.position.z,
                self.on_ground,
                self.horizontal_collision,
            ),
            (false, true) => packets::encode_play_move_player_rot(
                self.state.y_rot,
                self.state.x_rot,
                self.on_ground,
                self.horizontal_collision,
            ),
            (false, false) => packets::encode_play_move_player_status_only(
                self.on_ground,
                self.horizontal_collision,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleMoveCommand {
    pub position: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}

impl VehicleMoveCommand {
    pub(crate) fn encode_packet(self) -> (i32, Vec<u8>) {
        packets::encode_play_move_vehicle(
            self.position.x,
            self.position.y,
            self.position.z,
            self.y_rot,
            self.x_rot,
            self.on_ground,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetCommand {
    MovePlayer(PlayerMoveCommand),
    MoveVehicle(VehicleMoveCommand),
    PlayerAction(PlayerAction),
    PlayerCommand(PlayerCommand),
    PlayerAbilities(PlayerAbilitiesCommand),
    PlayerInput(PlayerInput),
    ChatCommand(ChatCommand),
    AttackEntity(AttackEntity),
    InteractEntity(InteractEntity),
    SetHeldSlot(u8),
    Swing(InteractionHand),
    UseItemOn(UseItemOn),
    UseItem(UseItem),
    PickItemFromBlock(PickItemFromBlock),
    PickItemFromEntity(PickItemFromEntity),
    PaddleBoat(PaddleBoat),
    PingRequest(i64),
    PlaceRecipe(PlaceRecipeCommand),
    RecipeBookChangeSettings(RecipeBookChangeSettingsCommand),
    RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand),
    SelectTrade(SelectTradeCommand),
    SignUpdate(SignUpdate),
    SelectBundleItem(SelectBundleItem),
    ContainerButtonClick(ContainerButtonClick),
    ContainerClick(ContainerClick),
    ContainerClose(ContainerCloseRequest),
    ContainerSlotStateChanged(ContainerSlotStateChanged),
    CommandSuggestionRequest(CommandSuggestionRequest),
    AcceptCodeOfConduct,
    Disconnect,
}
