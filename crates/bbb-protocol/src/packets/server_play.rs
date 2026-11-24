use std::collections::BTreeMap;

use uuid::Uuid;

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::RichTextComponent,
    ids,
    nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH},
    resource_pack_response_actions::ResourcePackResponseAction,
};
use bbb_facts::player_action_kinds::VANILLA_PLAYER_ACTION_KINDS;

use super::play_clientbound::AwardStats;
use super::tags::encode_update_tags;
use super::world_effects;
use super::{
    chunks, command_blocks, jigsaw_blocks, normalize_resource_location, serverbound,
    structure_blocks, test_blocks, test_instance_blocks, ArgumentSignature, ArgumentSignatures,
    AttackEntity, BlockChangedAck, BlockEntityData, BlockEntityTagQuery, BlockHitResult,
    BlockUpdate, ChangeDifficulty, ChangeGameModeCommand, ChatCommandSigned, ChatMessage,
    ChatSessionUpdate, ClientCommandAction, ClientInformation, CommandSuggestionRequest,
    CommonPlayerSpawnInfo, ContainerButtonClick, ContainerClick, ContainerClose,
    ContainerCloseRequest, ContainerInput, ContainerSetContent, ContainerSetData, ContainerSetSlot,
    ContainerSlotStateChanged, Cooldown, CustomClickAction, CustomPayload, DamageEvent, Difficulty,
    Direction, EditBook, EntityAnimation, EntityDataValueKind, EntityEvent, EntityTagQuery,
    GameEvent, GameRuleValues, HashedComponentPatch, HashedItemStack, HashedStack, InteractEntity,
    InteractionHand, JigsawGenerate, LastSeenMessagesUpdate, MessageSignature, MountScreenOpen,
    MoveVehicleCommand, OpenBook, OpenScreen, OpenSignEditor, PaddleBoat, PaintingVariantData,
    PickItemFromBlock, PickItemFromEntity, PlaceRecipeCommand, PlayLogin, PlayTime,
    PlayerAbilities, PlayerAction, PlayerCombatKill, PlayerCommand, PlayerExperience, PlayerHealth,
    PlayerInput, PlayerLookAt, PlayerPositionUpdate, PlayerRotationUpdate, PongResponse,
    RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, RemoveMobEffect, RenameItem,
    Respawn, SeenAdvancements, SelectAdvancementsTab, SelectBundleItem, SelectTradeCommand,
    SetBeacon, SetCamera, SetCommandBlock, SetCommandMinecart, SetCreativeModeSlot, SetCursorItem,
    SetEntityData, SetGameRule, SetHeldSlot, SetJigsawBlock, SetPlayerInventory, SetStructureBlock,
    SetTestBlock, SignUpdate, SpectateEntity, SystemChat, TagQuery, TeleportToEntity,
    TestInstanceBlockAction, TickingState, TickingStep, Transfer, UpdateMobEffect, UpdateTags,
    UseItem, UseItemOn, Vec3d,
};

const COMPONENT_NBT_BYTES: usize = crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize;
/// `ByteBufCodecs.STRING_UTF8` (`ByteBufCodecs.java:168`).
const GAME_RULE_VALUES_STRING_MAX_CHARS: usize = 32767;
/// `ServerboundPlayerAbilitiesPacket.FLAG_FLYING`
/// (`ServerboundPlayerAbilitiesPacket.java:13`).
const PLAYER_ABILITIES_FLAG_FLYING: u8 = 2;
/// The `ClientboundPlayerAbilitiesPacket` bitfield
/// (`ClientboundPlayerAbilitiesPacket.java:13-16`).
const CLIENTBOUND_PLAYER_ABILITIES_FLAG_INVULNERABLE: u8 = 1;
const CLIENTBOUND_PLAYER_ABILITIES_FLAG_FLYING: u8 = 2;
const CLIENTBOUND_PLAYER_ABILITIES_FLAG_CAN_FLY: u8 = 4;
const CLIENTBOUND_PLAYER_ABILITIES_FLAG_INSTABUILD: u8 = 8;

/// Serverbound play packets defined by Vanilla 26.1 (`GameProtocols.PLAY`).
#[derive(Debug, Clone, PartialEq)]
pub enum PlayServerbound {
    AcceptTeleportation {
        id: i32,
    },
    /// `ServerGamePacketListenerImpl.handleAttack`
    /// (ServerGamePacketListenerImpl.java:1784-1810): one melee attack against
    /// the entity the client names, carrying nothing but its id.
    Attack(AttackEntity),
    /// `ServerGamePacketListenerImpl.handleChangeDifficulty`
    /// (ServerGamePacketListenerImpl.java:2063-2074): a gamemaster-gated
    /// request to change the world difficulty.
    ChangeDifficulty {
        difficulty: Difficulty,
    },
    Chat(ChatMessage),
    /// `ServerboundChatCommandSignedPacket` (:13-30): the same command string
    /// as the unsigned request, plus the signature material for the arguments
    /// a command declares signable and the acknowledgement window the
    /// signatures were made against.
    ChatCommandSigned(ChatCommandSigned),
    /// `ServerboundCommandSuggestionPacket` (:13-25): the partial command the
    /// client wants completed, and the transaction id the answer echoes.
    CommandSuggestion(CommandSuggestionRequest),
    ChatCommand {
        command: String,
    },
    /// `ServerGamePacketListenerImpl.handleChunkBatchReceived`
    /// (ServerGamePacketListenerImpl.java:2138-2141): the client's answer to a
    /// finished chunk batch, carrying how many chunks per tick it wants next.
    ChunkBatchReceived {
        desired_chunks_per_tick: f32,
    },
    ClientInformation(ClientInformation),
    ClientTickEnd,
    /// `ServerGamePacketListenerImpl.handleContainerButtonClick`
    /// (ServerGamePacketListenerImpl.java:1985-1997): a menu's own button,
    /// which only the menus that declare one accept.
    ContainerButtonClick(ContainerButtonClick),
    /// `ServerGamePacketListenerImpl.handleContainerClick`
    /// (ServerGamePacketListenerImpl.java:1915-1951): one click against the
    /// open menu, carrying the client's own prediction of what it produced.
    ContainerClick(ContainerClick),
    /// `ServerGamePacketListenerImpl.handleContainerClose`
    /// (ServerGamePacketListenerImpl.java:1909-1912). The handler ignores the
    /// container id the packet carries and closes whatever is open.
    ContainerClose(ContainerCloseRequest),
    /// `ServerGamePacketListenerImpl.handleContainerSlotStateChanged`
    /// (ServerGamePacketListenerImpl.java:1033-1040): the crafter's per-slot
    /// enable toggle.
    ContainerSlotStateChanged(ContainerSlotStateChanged),
    /// `ServerCommonPacketListenerImpl.handleCookieResponse`
    /// (ServerCommonPacketListenerImpl.java:114-116): a server that never
    /// requested a cookie treats the answer as an unexpected query.
    CookieResponse {
        key: String,
        payload: Option<Vec<u8>>,
    },
    ConfigurationAcknowledged,
    CustomClickAction(CustomClickAction),
    CustomPayload(CustomPayload),
    /// `ServerGamePacketListenerImpl.handleInteract`
    /// (ServerGamePacketListenerImpl.java:1813-1839): a use-on-entity, carrying
    /// the hand, the point on the hitbox the client aimed at and whether the
    /// player was holding the secondary-action key when it sent this.
    Interact(InteractEntity),
    KeepAlive {
        id: i64,
    },
    /// `ServerGamePacketListenerImpl.handleLockDifficulty`
    /// (ServerGamePacketListenerImpl.java:2091-2096): the gamemaster-gated
    /// difficulty lock toggle.
    LockDifficulty {
        locked: bool,
    },
    MovePlayer(MovePlayer),
    /// `ServerGamePacketListenerImpl.handleChangeGameMode`
    /// (ServerGamePacketListenerImpl.java:2077-2086): the game mode switcher,
    /// gated on the same permission the `/gamemode` command is.
    ChangeGameMode(ChangeGameModeCommand),
    /// `ServerGamePacketListenerImpl.handleSetGameRule`
    /// (ServerGamePacketListenerImpl.java:2103-2122): the world-options screen
    /// writing game rule values back.
    SetGameRule(SetGameRule),
    /// `ServerGamePacketListenerImpl.handleClientCommand`
    /// (ServerGamePacketListenerImpl.java:1859-1891): respawning, asking for
    /// statistics, and asking for the game rule values.
    ClientCommand(ClientCommandAction),
    /// `ServerGamePacketListenerImpl.handleChatAck`
    /// (ServerGamePacketListenerImpl.java:1670-1679): how many messages the
    /// client has seen since it last said so.
    ChatAck {
        offset: i32,
    },
    /// `ServerGamePacketListenerImpl.handleChatSessionUpdate`
    /// (`ServerGamePacketListenerImpl.java:2099-2113`): the client's profile key
    /// announcement, ignored when the server has no Mojang services key.
    ChatSessionUpdate(ChatSessionUpdate),
    /// `ServerGamePacketListenerImpl.handleBlockEntityTagQuery`
    /// (ServerGamePacketListenerImpl.java:449-456): the F3 block-entity NBT
    /// readout, gated on the gamemaster permission.
    BlockEntityTagQuery(BlockEntityTagQuery),
    /// `ServerGamePacketListenerImpl.handleEntityTagQuery`
    /// (ServerGamePacketListenerImpl.java:483-496): the same readout aimed at
    /// an entity.
    EntityTagQuery(EntityTagQuery),
    /// `ServerGamePacketListenerImpl.handleDebugSubscriptionRequest`
    /// (ServerGamePacketListenerImpl.java): the debug streams a client asks to
    /// receive. The request is always kept; what it is worth is decided when
    /// the subscriptions are read.
    DebugSubscriptionRequest {
        subscription_ids: Vec<i32>,
    },
    /// `ServerGamePacketListenerImpl.handleJigsawGenerate`: the jigsaw block
    /// editor's generate button.
    JigsawGenerate(JigsawGenerate),
    /// `ServerGamePacketListenerImpl.handleSetCommandBlock`: the command block
    /// editor's save button.
    SetCommandBlock(SetCommandBlock),
    /// `ServerGamePacketListenerImpl.handleSetCommandMinecart`: the same
    /// editor over a command-block minecart.
    SetCommandMinecart(SetCommandMinecart),
    /// `ServerGamePacketListenerImpl.handleSetJigsawBlock`: the jigsaw block
    /// editor's save button.
    SetJigsawBlock(SetJigsawBlock),
    /// `ServerGamePacketListenerImpl.handleSetStructureBlock`: the structure
    /// block editor, whose update kind decides whether it saves, loads or
    /// scans.
    SetStructureBlock(SetStructureBlock),
    /// `ServerGamePacketListenerImpl.handleSetTestBlock`: the test block
    /// editor's save button.
    SetTestBlock(SetTestBlock),
    /// `ServerGamePacketListenerImpl.handleTestInstanceBlockAction`: every
    /// button on the test instance block editor.
    TestInstanceBlockAction(TestInstanceBlockAction),
    /// `ServerGamePacketListenerImpl.handleSpectateEntity`
    /// (ServerGamePacketListenerImpl.java:1842-1856): the spectator camera
    /// switch.
    SpectateEntity(SpectateEntity),
    /// `ServerGamePacketListenerImpl.handleTeleportToEntityPacket`
    /// (ServerGamePacketListenerImpl.java:1422-1433): the spectator menu's
    /// teleport, which names its target by profile id.
    TeleportToEntity(TeleportToEntity),
    /// `ServerGamePacketListenerImpl.handlePickItemFromBlock`
    /// (ServerGamePacketListenerImpl.java:691-709): the creative pick, naming
    /// the block the client is looking at.
    PickItemFromBlock(PickItemFromBlock),
    /// `ServerGamePacketListenerImpl.handlePickItemFromEntity`
    /// (ServerGamePacketListenerImpl.java:725-739): the same pick aimed at an
    /// entity instead.
    PickItemFromEntity(PickItemFromEntity),
    PingRequest {
        time: i64,
    },
    /// `ServerGamePacketListenerImpl.handlePlayerAbilities`
    /// (ServerGamePacketListenerImpl.java:2047-2050). The wire form carries
    /// only the flying bit (`ServerboundPlayerAbilitiesPacket.java:13,25-31`);
    /// every other ability stays server-authoritative.
    PlayerAbilities {
        flying: bool,
    },
    PlayerAction(PlayerAction),
    /// `ServerGamePacketListenerImpl.handlePlayerCommand`
    /// (ServerGamePacketListenerImpl.java:1689-1733): the discrete player state
    /// changes a client asks for, sprinting among them.
    PlayerCommand(PlayerCommand),
    /// `ServerGamePacketListenerImpl.handlePlayerInput`
    /// (ServerGamePacketListenerImpl.java:417-424): the movement keys the
    /// client is holding, which the server keeps and projects the sneak bit
    /// from.
    PlayerInput(PlayerInput),
    PlayerLoaded,
    Pong {
        id: i32,
    },
    ResourcePack {
        id: Uuid,
        action: ResourcePackResponseAction,
    },
    SetCarriedItem {
        slot: i16,
    },
    SetCreativeModeSlot(SetCreativeModeSlot),
    Swing {
        hand: InteractionHand,
    },
    /// `ServerGamePacketListenerImpl.handleEditBook`
    /// (ServerGamePacketListenerImpl.java:977-989): the book-and-quill editor
    /// saving its pages or signing them into a written book, naming the
    /// inventory slot the book is in.
    EditBook(EditBook),
    /// `ServerGamePacketListenerImpl.handlePlaceRecipe`
    /// (ServerGamePacketListenerImpl.java:1954-1981): the recipe book laying a
    /// recipe out in the open crafting menu.
    PlaceRecipe(PlaceRecipeCommand),
    /// `ServerGamePacketListenerImpl.handleBundleItemSelectedPacket`
    /// (ServerGamePacketListenerImpl.java:565-568): which item inside a bundle
    /// the open menu has selected.
    SelectBundleItem(SelectBundleItem),
    /// `ServerGamePacketListenerImpl.handleSignUpdate`
    /// (ServerGamePacketListenerImpl.java:2028-2031): the sign editor's four
    /// lines, aimed at one side of one sign.
    SignUpdate(SignUpdate),
    /// `ServerGamePacketListenerImpl.handleMoveVehicle`
    /// (ServerGamePacketListenerImpl.java:439-516): where the client says the
    /// vehicle it is steering has moved to.
    MoveVehicle(MoveVehicleCommand),
    /// `ServerGamePacketListenerImpl.handlePaddleBoat`
    /// (ServerGamePacketListenerImpl.java:1436-1441): which oars the rower is
    /// pulling.
    PaddleBoat(PaddleBoat),
    /// `ServerGamePacketListenerImpl.handleRecipeBookChangeSettingsPacket`
    /// (ServerGamePacketListenerImpl.java:571-574): the recipe book's own
    /// open/filter toggles, per book type.
    RecipeBookChangeSettings(RecipeBookChangeSettingsCommand),
    /// `ServerGamePacketListenerImpl.handleRecipeBookSeenRecipePacket`
    /// (ServerGamePacketListenerImpl.java:556-562): the highlight the client
    /// says it has now seen, named by the display index the server published.
    RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand),
    /// `ServerGamePacketListenerImpl.handleRenameItem`
    /// (ServerGamePacketListenerImpl.java:761-771): the anvil text box, which
    /// only an open `AnvilMenu` reads.
    RenameItem(RenameItem),
    /// `ServerGamePacketListenerImpl.handleSeenAdvancements`
    /// (ServerGamePacketListenerImpl.java:577-586): which advancement tab the
    /// screen has open.
    SeenAdvancements(SeenAdvancements),
    /// `ServerGamePacketListenerImpl.handleSelectTrade`
    /// (ServerGamePacketListenerImpl.java:962-973): the merchant offer the
    /// trading screen highlighted, which only an open `MerchantMenu` reads.
    SelectTrade(SelectTradeCommand),
    /// `ServerGamePacketListenerImpl.handleSetBeaconPacket`
    /// (ServerGamePacketListenerImpl.java:774-784): the two effects the beacon
    /// screen chose, which only an open `BeaconMenu` reads.
    SetBeacon(SetBeacon),
    /// `ServerGamePacketListenerImpl.handleUseItem`
    /// (ServerGamePacketListenerImpl.java:1356-1377): the right-click that
    /// hits no block, which carries the rotation it was aimed with.
    UseItem(UseItem),
    UseItemOn(UseItemOn),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovePlayer {
    pub position: Option<Vec3d>,
    pub rotation: Option<PlayerRotation>,
    pub on_ground: bool,
    pub horizontal_collision: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerRotation {
    pub y_rot: f32,
    pub x_rot: f32,
}

mod clientbound_player;
mod clientbound_world;
mod decode;

pub use clientbound_player::*;
pub use clientbound_world::*;
pub use decode::{decode_play_serverbound, decode_play_serverbound_with_content_registry};

pub(super) fn encode_position(encoder: &mut Encoder, position: Vec3d) {
    encoder.write_f64(position.x);
    encoder.write_f64(position.y);
    encoder.write_f64(position.z);
}

pub(super) fn encode_common_spawn_info(
    encoder: &mut Encoder,
    spawn: &CommonPlayerSpawnInfo,
) -> Result<()> {
    encoder.write_var_i32(spawn.dimension_type_id);
    encoder.write_string(&normalize_resource_location(spawn.dimension.clone())?);
    encoder.write_i64(spawn.seed);
    encoder.write_i8(spawn.game_type);
    encoder.write_i8(spawn.previous_game_type);
    encoder.write_bool(spawn.is_debug);
    encoder.write_bool(spawn.is_flat);
    encoder.write_bool(spawn.last_death_location.is_some());
    if let Some(last_death) = &spawn.last_death_location {
        encoder.write_string(&normalize_resource_location(last_death.dimension.clone())?);
        encoder.write_i64(chunks::encode_block_pos(last_death.pos));
    }
    encoder.write_var_i32(spawn.portal_cooldown);
    encoder.write_var_i32(spawn.sea_level);
    Ok(())
}

pub(super) fn write_len(encoder: &mut Encoder, len: usize, what: &'static str) -> Result<()> {
    let len = i32::try_from(len)
        .map_err(|_| ProtocolError::InvalidData(format!("{what} length exceeds i32")))?;
    encoder.write_var_i32(len);
    Ok(())
}

pub(super) fn ensure_finished(decoder: &Decoder<'_>, packet: &'static str) -> Result<()> {
    if decoder.is_empty() {
        Ok(())
    } else {
        Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {packet} packet"
        )))
    }
}

#[cfg(test)]
mod tests;
