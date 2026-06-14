use bbb_protocol::packets::{
    self, CommandSuggestionRequest, ContainerButtonClick, ContainerCloseRequest,
    ContainerSlotStateChanged, InteractionHand, PickItemFromBlock, PlayerAction, PlayerCommand,
    PlayerInput, PlayerPositionState, UseItem, UseItemOn, Vec3d,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerMoveCommand {
    pub state: PlayerPositionState,
    pub on_ground: bool,
    pub horizontal_collision: bool,
}

impl PlayerMoveCommand {
    pub(crate) fn encode_packet(self) -> (i32, Vec<u8>) {
        packets::encode_play_move_player_pos_rot(
            self.state.position.x,
            self.state.position.y,
            self.state.position.z,
            self.state.y_rot,
            self.state.x_rot,
            self.on_ground,
            self.horizontal_collision,
        )
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
    PlayerInput(PlayerInput),
    SetHeldSlot(u8),
    Swing(InteractionHand),
    UseItemOn(UseItemOn),
    UseItem(UseItem),
    PickItemFromBlock(PickItemFromBlock),
    ContainerButtonClick(ContainerButtonClick),
    ContainerClose(ContainerCloseRequest),
    ContainerSlotStateChanged(ContainerSlotStateChanged),
    CommandSuggestionRequest(CommandSuggestionRequest),
    Disconnect,
}
