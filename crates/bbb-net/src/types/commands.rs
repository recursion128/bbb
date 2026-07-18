use bbb_protocol::packets::{
    self, AttackEntity, BlockEntityTagQuery, ChangeDifficultyCommand, ChangeGameModeCommand,
    ChatAcknowledgement, ChatCommand, ChatCommandSigned, ChatMessage, ChatSessionUpdate,
    ClientCommandAction, CommandSuggestionRequest, ContainerButtonClick, ContainerClick,
    ContainerCloseRequest, ContainerSlotStateChanged, CustomClickAction, EditBook, EntityTagQuery,
    InteractEntity, InteractionHand, LockDifficultyCommand, PaddleBoat, PickItemFromBlock,
    PickItemFromEntity, PlaceRecipeCommand, PlayerAbilitiesCommand, PlayerAction, PlayerCommand,
    PlayerInput, PlayerPositionState, RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand,
    RenameItem, ResourcePackResponseAction, SeenAdvancements, SelectBundleItem, SelectTradeCommand,
    ServerboundCustomPayload, SetBeacon, SetCreativeModeSlot, SignUpdate, SpectateEntity,
    TeleportToEntity, UseItem, UseItemOn, Vec3d,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AcceptTeleportCommand {
    pub teleport_id: i32,
    pub state: PlayerPositionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerMoveCommand {
    pub state: PlayerPositionState,
    pub on_ground: bool,
    pub horizontal_collision: bool,
    #[serde(default)]
    pub force_position: bool,
    #[serde(default)]
    pub force_pos_rot: bool,
    #[serde(default)]
    pub force_rotation_only: bool,
}

impl PlayerMoveCommand {
    pub(crate) fn encode_packet_from(self, previous: PlayerPositionState) -> (i32, Vec<u8>) {
        if self.force_pos_rot {
            return packets::encode_play_move_player_pos_rot(
                self.state.position.x,
                self.state.position.y,
                self.state.position.z,
                self.state.y_rot,
                self.state.x_rot,
                self.on_ground,
                self.horizontal_collision,
            );
        }
        if self.force_rotation_only {
            return packets::encode_play_move_player_rot(
                self.state.y_rot,
                self.state.x_rot,
                self.on_ground,
                self.horizontal_collision,
            );
        }
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

    pub(crate) fn apply_to_position_state(
        self,
        previous: PlayerPositionState,
    ) -> PlayerPositionState {
        let mut next = previous;
        next.delta_movement = self.state.delta_movement;
        if self.force_pos_rot {
            next.position = self.state.position;
            next.y_rot = self.state.y_rot;
            next.x_rot = self.state.x_rot;
            return next;
        }
        if self.force_rotation_only {
            next.y_rot = self.state.y_rot;
            next.x_rot = self.state.x_rot;
            return next;
        }

        let position_changed = self.force_position || self.state.position != previous.position;
        let rotation_changed =
            self.state.y_rot != previous.y_rot || self.state.x_rot != previous.x_rot;
        if position_changed {
            next.position = self.state.position;
        }
        if rotation_changed {
            next.y_rot = self.state.y_rot;
            next.x_rot = self.state.x_rot;
        }
        next
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
    ClientTickEnd,
    PlayerLoaded,
    /// The canonical world applied a server position correction; the
    /// connection answers with accept-teleportation plus a move echo of the
    /// post-apply transform and lifts the pending-teleport move gate.
    AcceptTeleport(AcceptTeleportCommand),
    /// The canonical world flushed its state for play-to-configuration
    /// re-entry; the connection sends any pending chat acknowledgement and
    /// acknowledges the configuration switch.
    AcknowledgeConfiguration {
        pending_chat_acknowledgement: Option<ChatAcknowledgement>,
    },
    MovePlayer(PlayerMoveCommand),
    MoveVehicle(VehicleMoveCommand),
    PlayerAction(PlayerAction),
    PlayerCommand(PlayerCommand),
    PlayerAbilities(PlayerAbilitiesCommand),
    PlayerInput(PlayerInput),
    ChatAcknowledgement(ChatAcknowledgement),
    ChatCommand(ChatCommand),
    ChatCommandSigned(ChatCommandSigned),
    ChatMessage(ChatMessage),
    ChatSessionUpdate(ChatSessionUpdate),
    CustomPayload(ServerboundCustomPayload),
    CustomClickAction(CustomClickAction),
    DebugSubscriptionRequest {
        tick_time: bool,
    },
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
    BlockEntityTagQuery(BlockEntityTagQuery),
    EntityTagQuery(EntityTagQuery),
    ChangeDifficulty(ChangeDifficultyCommand),
    ChangeGameMode(ChangeGameModeCommand),
    LockDifficulty(LockDifficultyCommand),
    PlaceRecipe(PlaceRecipeCommand),
    RecipeBookChangeSettings(RecipeBookChangeSettingsCommand),
    RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand),
    EditBook(EditBook),
    RenameItem(RenameItem),
    SeenAdvancements(SeenAdvancements),
    SelectTrade(SelectTradeCommand),
    SetBeacon(SetBeacon),
    SetCreativeModeSlot(SetCreativeModeSlot),
    SignUpdate(SignUpdate),
    SpectateEntity(SpectateEntity),
    TeleportToEntity(TeleportToEntity),
    SelectBundleItem(SelectBundleItem),
    PerformRespawn,
    RequestStats,
    RequestGameRuleValues,
    ContainerButtonClick(ContainerButtonClick),
    ContainerClick(ContainerClick),
    ContainerClose(ContainerCloseRequest),
    ContainerSlotStateChanged(ContainerSlotStateChanged),
    CommandSuggestionRequest(CommandSuggestionRequest),
    ResourcePackResponse {
        id: Uuid,
        action: ResourcePackResponseAction,
    },
    AcceptCodeOfConduct,
    Disconnect,
}

impl NetCommand {
    pub(crate) fn client_command_action(&self) -> Option<ClientCommandAction> {
        match self {
            Self::PerformRespawn => Some(ClientCommandAction::PerformRespawn),
            Self::RequestStats => Some(ClientCommandAction::RequestStats),
            Self::RequestGameRuleValues => Some(ClientCommandAction::RequestGameRuleValues),
            _ => None,
        }
    }
}
