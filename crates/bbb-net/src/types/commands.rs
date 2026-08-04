use bbb_protocol::packets::{
    self, AttackEntity, BlockEntityTagQuery, ChangeDifficultyCommand, ChangeGameModeCommand,
    ChatAcknowledgement, ClientCommandAction, ClientInformation, CommandSuggestionRequest,
    ContainerButtonClick, ContainerClick, ContainerCloseRequest, ContainerSlotStateChanged,
    CustomClickAction, EditBook, EntityTagQuery, InteractEntity, InteractionHand, JigsawGenerate,
    LockDifficultyCommand, PaddleBoat, PickItemFromBlock, PickItemFromEntity, PlaceRecipeCommand,
    PlayerAbilitiesCommand, PlayerAction, PlayerCommand, PlayerInput, PlayerPositionState,
    RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, RenameItem,
    ResourcePackResponseAction, SeenAdvancements, SelectBundleItem, SelectTradeCommand,
    ServerboundCustomPayload, SetBeacon, SetCommandBlock, SetCommandMinecart, SetCreativeModeSlot,
    SetGameRule, SetJigsawBlock, SetStructureBlock, SetTestBlock, SignUpdate, SpectateEntity,
    TeleportToEntity, TestInstanceBlockAction, UseItem, UseItemOn, Vec3d,
};
use bbb_world::{OutboundLastSeenMessages, SignableCommandArgument};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundChatCommandSigning {
    pub timestamp_millis: i64,
    pub salt: i64,
    pub arguments: Vec<SignableCommandArgument>,
    pub last_seen: OutboundLastSeenMessages,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundChatCommand {
    pub command: String,
    pub signing: Option<OutboundChatCommandSigning>,
}

impl OutboundChatCommand {
    pub fn unsigned(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            signing: None,
        }
    }

    pub fn signed(
        command: impl Into<String>,
        timestamp_millis: i64,
        salt: i64,
        arguments: Vec<SignableCommandArgument>,
        last_seen: OutboundLastSeenMessages,
    ) -> Self {
        debug_assert!(!arguments.is_empty());
        Self {
            command: command.into(),
            signing: Some(OutboundChatCommandSigning {
                timestamp_millis,
                salt,
                arguments,
                last_seen,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundChatMessage {
    pub message: String,
    pub timestamp_millis: i64,
    pub salt: i64,
    pub last_seen: OutboundLastSeenMessages,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackEntityCommand {
    pub packet: AttackEntity,
    pub selected_hotbar_slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InteractEntityCommand {
    pub packet: InteractEntity,
    pub selected_hotbar_slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UseItemOnCommand {
    pub packet: UseItemOn,
    pub selected_hotbar_slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UseItemCommand {
    pub packet: UseItem,
    pub selected_hotbar_slot: u8,
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
    ChatCommand(OutboundChatCommand),
    ChatMessage(OutboundChatMessage),
    CustomPayload(ServerboundCustomPayload),
    CustomClickAction(CustomClickAction),
    DebugSubscriptionRequest {
        subscription_ids: Vec<i32>,
    },
    AttackEntity(AttackEntityCommand),
    InteractEntity(InteractEntityCommand),
    SetHeldSlot(u8),
    Swing(InteractionHand),
    UseItemOn(UseItemOnCommand),
    UseItem(UseItemCommand),
    PickItemFromBlock(PickItemFromBlock),
    PickItemFromEntity(PickItemFromEntity),
    PaddleBoat(PaddleBoat),
    PingRequest(i64),
    BlockEntityTagQuery(BlockEntityTagQuery),
    EntityTagQuery(EntityTagQuery),
    ChangeDifficulty(ChangeDifficultyCommand),
    ChangeGameMode(ChangeGameModeCommand),
    LockDifficulty(LockDifficultyCommand),
    SetGameRule(SetGameRule),
    PlaceRecipe(PlaceRecipeCommand),
    RecipeBookChangeSettings(RecipeBookChangeSettingsCommand),
    RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand),
    EditBook(EditBook),
    RenameItem(RenameItem),
    SeenAdvancements(SeenAdvancements),
    SelectTrade(SelectTradeCommand),
    SetBeacon(SetBeacon),
    SetCommandBlock(SetCommandBlock),
    SetCommandMinecart(SetCommandMinecart),
    SetJigsawBlock(SetJigsawBlock),
    JigsawGenerate(JigsawGenerate),
    SetStructureBlock(SetStructureBlock),
    SetTestBlock(SetTestBlock),
    TestInstanceBlockAction(TestInstanceBlockAction),
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
    /// `Options.broadcastOptions`, which `Options.save` runs unconditionally
    /// after writing the file: the current player information goes back out on
    /// whichever phase the connection is in.
    BroadcastClientInformation(ClientInformation),
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
