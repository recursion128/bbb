//! World configuration, inbound commands, outbound events, and tick results.

#[cfg(test)]
use std::sync::Arc;
use uuid::Uuid;

use super::LevelId;
use crate::admin::{AdminEnvelope, AdminOutcome, AdminRequest, StoredUserLists};
use crate::commands;
use crate::commands::CommandPermission;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::mailbox::WorldProducerClass;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::game_types::GameType;
#[cfg(test)]
use bbb_protocol::packets::{BlockPos, ItemStackSummary, PlayServerbound};
use bbb_protocol::packets::{ClientInformation, GameProfile, PlayerRotation, Vec3d};
use bbb_protocol::RichTextComponent;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::{Result, ServerWorldError};

#[derive(Debug, Clone)]
pub struct ServerWorldConfig {
    pub max_players: usize,
    pub chunk_radius: i32,
    pub simulation_distance: i32,
    pub keep_alive_interval_ticks: u64,
    pub difficulty: Difficulty,
    pub game_type: GameType,
    pub force_game_mode: bool,
    pub spawn_protection_radius: i32,
    /// The server-list description (`serversettings/motd`), which the
    /// management surface may change while the server runs.
    pub motd: String,
    /// `white-list`, `enforce-whitelist` and `player-idle-timeout` as startup
    /// resolved them.
    pub use_allowlist: bool,
    pub enforce_allowlist: bool,
    pub player_idle_timeout: i32,
    /// `hide-online-players`, `enable-status` and `op-permission-level`.
    pub hide_online_players: bool,
    pub status_replies: bool,
    pub accepts_transfers: bool,
    pub operator_user_permission_level: CommandPermission,
    /// `function-permission-level` (DedicatedServerProperties.java:93-98,
    /// DedicatedServer.java:601-603): every `.mcfunction` line is parsed with
    /// it (`Commands.createCompilationContext`, ServerFunctionLibrary.java:84)
    /// while the run belongs to the calling source.
    pub function_permission_level: CommandPermission,
    /// `allow-flight` (`DedicatedServerProperties.java:59`).
    pub allow_flight: bool,
    /// `pause-when-empty-seconds`; zero or less never pauses.
    pub pause_when_empty_seconds: i32,
    /// `entity-broadcast-range-percentage`, already clamped.
    pub entity_broadcast_range_percentage: i32,
    /// `MinecraftServer.isResourcePackRequired`: a server pack is configured
    /// with `require-resource-pack`.
    pub resource_pack_required: bool,
    /// The four stored user lists the runtime read at startup; empty in the
    /// ephemeral fixture mode that keeps no documents.
    pub admin_lists: StoredUserLists,
    /// The datapack surface loaded at startup (`WorldStem.load` equivalent);
    /// `None` keeps the bare flat-fixture defaults for tests.
    pub datapacks: Option<std::sync::Arc<bbb_server_datapack::startup::DatapackStartup>>,
    pub debug_trace_directory: std::path::PathBuf,
    pub generated_structure_directory: Option<std::path::PathBuf>,
    /// Ticket-driven generation commits once its parallel stages finish in a
    /// later tick instead of blocking the tick that filed it.
    pub background_generation: bool,
    pub plugins: std::sync::Arc<crate::plugin::ServerPluginRegistry>,
}

impl Default for ServerWorldConfig {
    fn default() -> Self {
        Self {
            max_players: 20,
            chunk_radius: 2,
            simulation_distance: 2,
            keep_alive_interval_ticks: 300,
            difficulty: Difficulty::Easy,
            game_type: GameType::Survival,
            force_game_mode: false,
            spawn_protection_radius: 16,
            motd: crate::admin::DEFAULT_MOTD.to_string(),
            use_allowlist: false,
            enforce_allowlist: false,
            player_idle_timeout: 0,
            hide_online_players: false,
            status_replies: true,
            accepts_transfers: false,
            operator_user_permission_level: CommandPermission::Owner,
            function_permission_level: CommandPermission::GameMaster,
            allow_flight: false,
            pause_when_empty_seconds: 0,
            entity_broadcast_range_percentage:
                crate::admin::DEFAULT_ENTITY_BROADCAST_RANGE_PERCENTAGE,
            resource_pack_required: false,
            admin_lists: StoredUserLists::default(),
            datapacks: None,
            debug_trace_directory: "debug".into(),
            generated_structure_directory: None,
            background_generation: false,
            plugins: std::sync::Arc::clone(crate::plugin::ServerPluginRegistry::vanilla()),
        }
    }
}

impl ServerWorldConfig {
    pub(in crate::world) fn validate(&self) -> Result<()> {
        if self.max_players == 0 || self.max_players > i32::MAX as usize {
            return Err(ServerWorldError::InvalidConfiguration(
                "max players must be in 1..=i32::MAX".to_string(),
            ));
        }
        if self.chunk_radius < 2 || self.simulation_distance < 2 {
            return Err(ServerWorldError::InvalidConfiguration(
                "chunk radius and simulation distance must be at least 2".to_string(),
            ));
        }
        if self.keep_alive_interval_ticks == 0 {
            return Err(ServerWorldError::InvalidConfiguration(
                "keep-alive interval must be non-zero".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_server_world_config_uses_survival_game_type() {
        assert_eq!(ServerWorldConfig::default().game_type, GameType::Survival);
    }
}

/// `TeleportTransition` (`net/minecraft/world/level/portal/TeleportTransition.java`)
/// as much of it as this server can build.
///
/// The record's four remaining fields are all absent for the same reason: no
/// producer here can set them. `deltaMovement` is `Vec3.ZERO` for every
/// transition `TeleportCommand` builds, `relatives` is empty for the same set,
/// `missingRespawnBlock` is only true on `TeleportTransition.missingRespawnBlock`,
/// `asPassenger` only on a passenger teleport, and `postTeleportTransition` is
/// the portal sound or the portal ticket. Adding one would be adding a caller
/// that could set it.
#[derive(Debug, Clone, PartialEq)]
pub struct TeleportTransition<L = Java26LevelKey> {
    /// `TeleportTransition.newLevel`.
    pub new_level: L,
    /// `TeleportTransition.position`.
    pub position: Vec3d,
    /// `TeleportTransition.yRot`.
    pub y_rot: f32,
    /// `TeleportTransition.xRot`.
    pub x_rot: f32,
    pub relatives_mask: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JoinContext {
    pub transferred: bool,
    pub remote_address: Option<String>,
    /// Epoch milliseconds.
    pub now: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldCommand {
    Join {
        connection_id: ConnectionId,
        profile: GameProfile,
        /// The options the client declared during configuration, which
        /// `PlayerList.placeNewPlayer` hands to the new `ServerPlayer`.
        client_information: ClientInformation,
        context: JoinContext,
    },
    PlayPacket {
        connection_id: ConnectionId,
        packet_id: i32,
        payload: Vec<u8>,
    },
    #[cfg(test)]
    Play {
        connection_id: ConnectionId,
        packet: PlayServerbound,
    },
    Disconnect {
        connection_id: ConnectionId,
    },
    /// One management request, carrying the authority and deadline the
    /// transport resolved. `MinecraftExecutorServiceImpl.submit` is the vanilla
    /// equivalent: the service body runs on the server thread, not on the
    /// connection that asked for it.
    Admin {
        envelope: AdminEnvelope,
        request: AdminRequest,
    },
    /// The generation a requested reload produced, or why it produced none.
    ///
    /// `MinecraftServer.reloadResources` installs the new resources on the
    /// server thread and, on failure, keeps the ones it had
    /// (`ReloadCommand.reloadPacks` reports `commands.reload.failure`). This
    /// command is that install: it arrives through the same mailbox as
    /// everything else, so a reload lands at a tick boundary rather than
    /// between two halves of one.
    Datapacks {
        startup: ReloadedDatapacks,
    },
    Shutdown,
}

impl WorldCommand {
    pub fn producer_class(&self) -> WorldProducerClass {
        match self {
            Self::Join { .. } | Self::PlayPacket { .. } | Self::Disconnect { .. } => {
                WorldProducerClass::Network
            }
            #[cfg(test)]
            Self::Play { .. } => WorldProducerClass::Network,
            Self::Admin { .. } => WorldProducerClass::Admin,
            Self::Datapacks { .. } | Self::Shutdown => WorldProducerClass::Lifecycle,
        }
    }
}

/// The outcome of one rebuild, as it travels back into the world.
///
/// Two rebuilds are the same install only when they are the same object: a
/// generation carries no identity of its own, and comparing two of them field
/// by field would say "equal" about two loads that ran at different times over
/// different files. Pointer identity is the honest comparison, and it is all
/// the mailbox's equality is ever used for.
#[derive(Debug, Clone)]
pub enum ReloadedDatapacks {
    Built(std::sync::Arc<bbb_server_datapack::startup::DatapackStartup>),
    Failed(String),
}

impl PartialEq for ReloadedDatapacks {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Built(left), Self::Built(right)) => std::sync::Arc::ptr_eq(left, right),
            (Self::Failed(left), Self::Failed(right)) => left == right,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldCloseReason {
    /// A refusal the management surface or a stored user list produced, which
    /// carries its own component: the kick message a management client chose,
    /// or the ban reason and expiry `PlayerList.canPlayerLogin` assembles.
    Administrative(RichTextComponent),
    DuplicateConnection,
    DuplicatePlayer,
    InvalidBlockInteraction,
    ChatValidationFailed,
    ChunkGenerationFailed {
        detail: String,
    },
    InvalidCommand,
    /// `multiplayer.disconnect.invalid_entity_attacked` (`handleAttack`,
    /// :1799): the client asked this server to attack something no client is
    /// allowed to attack.
    InvalidEntityAttacked,
    InvalidMovement,
    /// `multiplayer.disconnect.invalid_vehicle_movement` (`handleMoveVehicle`,
    /// :441-442): the client reported a vehicle position or rotation that is
    /// not a number.
    InvalidVehicleMovement,
    Floating,
    IdleTimeout,
    KeepAliveMismatch,
    KeepAliveTimeout,
    PacketOutsideActiveContract,
    PlayerCapacity,
    /// `multiplayer.requiredTexturePrompt.disconnect`
    /// (`ServerCommonPacketListenerImpl.java:107-110`).
    RequiredResourcePackDeclined,
    ServerShutdown,
    /// `ServerCommonPacketListenerImpl.DISCONNECT_UNEXPECTED_QUERY`: an answer
    /// to a query this server never asked.
    UnexpectedQuery,
    UnknownConnection,
}

impl WorldCloseReason {
    /// The component `ServerCommonPacketListenerImpl.disconnect` (:141-151)
    /// would carry, or `None` where vanilla ends the connection without one.
    ///
    /// Vanilla has two ways to end a session: `disconnect(Component)`, which
    /// tells the client why, and a handler that throws, which leaves the
    /// connection to die with whatever the exception path says. A reason that
    /// mirrors a throw -- or that this server refuses for a reason vanilla has
    /// no equivalent of -- carries no component rather than an invented one.
    pub fn reason_component(&self) -> Option<RichTextComponent> {
        let key = match self {
            // The management surface and the stored user lists build their own
            // component, so this reason carries it instead of a key.
            Self::Administrative(component) => return Some(component.clone()),
            Self::DuplicateConnection | Self::DuplicatePlayer => {
                "multiplayer.disconnect.duplicate_login"
            }
            Self::PlayerCapacity => "multiplayer.disconnect.server_full",
            Self::RequiredResourcePackDeclined => "multiplayer.requiredTexturePrompt.disconnect",
            Self::ServerShutdown => "multiplayer.disconnect.server_shutdown",
            // `ServerGamePacketListenerImpl.CHAT_VALIDATION_FAILED` (:236).
            Self::ChatValidationFailed => "multiplayer.disconnect.chat_validation_failed",
            // `handleAttack` (:1799).
            Self::InvalidEntityAttacked => "multiplayer.disconnect.invalid_entity_attacked",
            // `handleMovePlayer` (:529) and `handleMoveVehicle`'s player-side
            // twin (:1056).
            Self::InvalidMovement => "multiplayer.disconnect.invalid_player_movement",
            // `handleMoveVehicle` (:441-442).
            Self::InvalidVehicleMovement => "multiplayer.disconnect.invalid_vehicle_movement",
            // The vehicle arm of `tick` and `getMaximumFlyingTicks` (:337-363).
            Self::Floating => "multiplayer.disconnect.flying",
            // `ServerCommonPacketListenerImpl.TIMEOUT_DISCONNECTION_MESSAGE`
            // (:37), which both the unanswered challenge (:121-123) and the
            // wrong answer (:85-86) disconnect with.
            Self::KeepAliveMismatch | Self::KeepAliveTimeout => "disconnect.timeout",
            // `tick`'s idle arm (:300-305).
            Self::IdleTimeout => "multiplayer.disconnect.idling",
            // `ServerCommonPacketListenerImpl.DISCONNECT_UNEXPECTED_QUERY` (:38).
            Self::UnexpectedQuery => "multiplayer.disconnect.unexpected_query_response",
            // The refusals with no vanilla component behind them.
            //
            // `PacketOutsideActiveContract` mirrors a throw
            // (`handleConfigurationAcknowledged` :2126-2128) and the packet
            // kinds this server has no handler for at all;
            // `ChunkGenerationFailed` is this server's fail-closed answer to
            // an owned generator refusing before a packet exists;
            // `InvalidBlockInteraction` mirrors `ackBlockChangesUpTo`'s
            // `IllegalArgumentException` (:2270-2276); `InvalidCommand` is
            // this server's refusal for a command longer than the wire form
            // carries; `UnknownConnection` cannot happen to vanilla, whose
            // listener is the player's own.
            Self::ChunkGenerationFailed { .. }
            | Self::InvalidBlockInteraction
            | Self::InvalidCommand
            | Self::PacketOutsideActiveContract
            | Self::UnknownConnection => return None,
        };
        Some(commands::disconnect_reason(key))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldEvent {
    Broadcast {
        recipients: Vec<ConnectionId>,
        packet: ClientboundPlayPacket,
    },
    Send {
        connection_id: ConnectionId,
        packet: ClientboundPlayPacket,
    },
    Close {
        connection_id: ConnectionId,
        reason: WorldCloseReason,
    },
    /// One applied management request: the answer the transport routes back by
    /// operation id, and the audit record the runtime logs.
    Admin(AdminOutcome),
    SaveRequested {
        flush: bool,
        trigger: SaveTrigger,
    },
    /// `MinecraftServer.reloadResources`, which `ReloadCommand` hands the
    /// selected packs to: the new generation is built off the server thread and
    /// installed on it. The world owns no I/O and no threads, so it asks and
    /// the runtime builds; the answer comes back as `WorldCommand::Datapacks`.
    ///
    ReloadRequested {
        selected: Option<Vec<String>>,
    },
    StopRequested,
    ConsoleMessage(RichTextComponent),
}

#[cfg(test)]
impl WorldEvent {
    /// Returns the clientbound packet if this event delivers to `target` via
    /// either unicast `Send` or multi-recipient `Broadcast`.
    pub fn packet_for(&self, target: ConnectionId) -> Option<&ClientboundPlayPacket> {
        match self {
            Self::Send {
                connection_id,
                packet,
            } if *connection_id == target => Some(packet),
            Self::Broadcast { recipients, packet } if recipients.contains(&target) => Some(packet),
            _ => None,
        }
    }

    /// Returns the clientbound packet if this event is a `Send` or `Broadcast`,
    /// regardless of recipient.
    pub fn sent_packet(&self) -> Option<&ClientboundPlayPacket> {
        match self {
            Self::Send { packet, .. } | Self::Broadcast { packet, .. } => Some(packet),
            _ => None,
        }
    }

    /// Iterates `(recipient, packet)` deliveries for `Send` and `Broadcast`.
    pub fn deliveries(&self) -> impl Iterator<Item = (ConnectionId, &ClientboundPlayPacket)> {
        let (recipients, packet) = match self {
            Self::Send {
                connection_id,
                packet,
            } => (std::slice::from_ref(connection_id), Some(packet)),
            Self::Broadcast { recipients, packet } => (recipients.as_slice(), Some(packet)),
            _ => (&[][..], None),
        };
        recipients
            .iter()
            .filter_map(move |recipient| packet.map(|packet| (*recipient, packet)))
    }
}

/// Which of Vanilla's three `saveEverything` callers a save receipt belongs to.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveTrigger {
    /// An operator asked: `/save-all` from a session, or `server/save` from
    /// the management surface. Both reach `saveEverything(.., true)`, so
    /// neither can be told from the other inside the transaction.
    Management,
    /// The unattended interval elapsed: nobody asked.
    Autosave,
    /// The final flush after the last tick.
    Shutdown,
    ChunkUnload,
}

impl SaveTrigger {
    /// The receipt name, which is what the release gate matches an unattended
    /// save on.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Management => "management",
            Self::Autosave => "autosave",
            Self::Shutdown => "shutdown",
            Self::ChunkUnload => "chunk-unload",
        }
    }
}

/// A player's `Inventory` as the world publishes it: the storage slots
/// followed by the equipment slots, in the inventory's own numbering rather
/// than the menu's.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(test)]
pub struct InventorySnapshot {
    pub selected_slot: u8,
    pub slots: Vec<ItemStackSummary>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg(test)]
pub struct PlayerSnapshot {
    pub connection_id: ConnectionId,
    pub uuid: Uuid,
    pub name: String,
    pub entity_id: i32,
    pub position: Vec3d,
    pub rotation: PlayerRotation,
    pub on_ground: bool,
    pub client_loaded: bool,
    pub inventory: InventorySnapshot,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::world) struct WorldSpawn {
    pub(in crate::world) dimension: LevelId,
    pub(in crate::world) position: Vec3d,
    pub(in crate::world) rotation: PlayerRotation,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg(test)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub stopping: bool,
    pub changed_blocks: Vec<BlockSnapshot>,
    pub players: Vec<PlayerSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub struct BlockSnapshot {
    pub pos: BlockPos,
    pub block_state_id: i32,
}

#[derive(Debug, Clone)]
pub struct TickResult {
    pub tick: u64,
    pub processed_commands: usize,
    pub events: Vec<WorldEvent>,
    /// Published only when player membership or `allows_listing` state changed.
    pub online_players: Option<Vec<(Uuid, String)>>,
    pub online_player_count: usize,
    #[cfg(test)]
    pub snapshot: Arc<WorldSnapshot>,
}
