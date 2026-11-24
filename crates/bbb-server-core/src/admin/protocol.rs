//! The typed administration request and answer surface.
//!
//! Vanilla owners: the `net.minecraft.server.jsonrpc.methods` services and the
//! DTOs they exchange. The transport differs — bbb speaks MCP where vanilla
//! speaks JSON-RPC over a WebSocket — so this module owns the *semantics* those
//! services define and nothing about their framing.
//!
//! Every request carries an [`AdminEnvelope`]: the authenticated principal, the
//! granted scope, the caller's request id, the server-assigned operation id, and
//! the tick by which the request must have been applied. Vanilla has no such
//! envelope because its management server is a trusted local socket; a bbb
//! server can be reached by an agent, so authority and deadline travel with the
//! request into the tick that applies it.

use bbb_protocol::{component_literal_nbt, component_translatable_nbt, ComponentStyle};
use bbb_protocol::{game_rules::GameRuleValue, packets::BlockPos, RichTextComponent};
use uuid::Uuid;

use crate::commands::CommandPermission;

use super::identity::{NameAndId, PlayerRef};
use super::settings::AdminSetting;
use super::stored_user_list::{BanRecord, MAX_ADMIN_TEXT_CHARS};

/// The bound on how many entries one management request may carry. Vanilla's
/// list methods are unbounded; a bounded mailbox needs a bounded request.
pub const MAX_REQUEST_ENTRIES: usize = 256;
pub const MAX_WORLD_QUERY_CELLS: usize = 256;
/// The command text budget of a single administration request.
pub const MAX_ADMIN_COMMAND_CHARS: usize = 256;
/// `Message.translatableParams` bound. Vanilla has none.
pub const MAX_MESSAGE_PARAMS: usize = 16;

/// What a principal is allowed to ask for.
///
/// Vanilla authenticates the management socket and then grants everything
/// (`AuthenticationHandler`); bbb splits authority so an agent can be given
/// read access without the ability to mutate, and so the command dispatcher
/// stays behind an explicitly separate grant. The ladder is ordered: a
/// `Command` grant satisfies a `Manage` requirement, and `Manage` satisfies
/// `Read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdminScope {
    /// Readouts only: status, players, the four lists, settings, game rules
    /// and diagnostics.
    Read,
    /// Every mutation the typed surface owns.
    Manage,
    /// `command.execute`, which delegates the sole command dispatcher and can
    /// therefore reach anything a command can reach.
    Command,
}

impl AdminScope {
    pub fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Manage => "manage",
            Self::Command => "command",
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        Some(match name {
            "read" => Self::Read,
            "manage" => Self::Manage,
            "command" => Self::Command,
            _ => return None,
        })
    }
}

/// The authority, identity and deadline one administration request travels
/// with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminEnvelope {
    /// Server-assigned, strictly increasing; the answer is routed by it.
    pub operation_id: u64,
    /// The caller's own id. Two requests from one principal that share it are
    /// the same operation: the second is answered from the first's record
    /// rather than applied again.
    pub request_id: String,
    /// The authenticated principal, as the transport resolved it.
    pub principal: String,
    /// The scope the transport granted this principal.
    pub scope: AdminScope,
    /// The last tick at which this request may still be applied. The world owns
    /// no wall clock, so the transport converts its own deadline into a tick.
    pub deadline_tick: u64,
    /// The wall-clock instant the request was accepted, in epoch milliseconds.
    /// Ban creation and expiry are the only state that needs it, and it arrives
    /// with the request for the same reason `save_request` takes `last_played`.
    pub now: i64,
}

/// A `players/kick` entry (`PlayerService.KickDto`, PlayerService.java:45-53).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KickRequest {
    pub player: PlayerRef,
    pub message: Option<AdminMessage>,
}

/// A `bans/add` or `bans/set` entry (`BanlistService.UserBanDto`,
/// BanlistService.java:113-135).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBanRequest {
    pub player: PlayerRef,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub expires: Option<i64>,
}

/// An `ip_bans/add` entry (`IpBanlistService.IncomingIpBanDto`,
/// IpBanlistService.java:90-111): either a literal address or a player whose
/// current address is banned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingIpBanRequest {
    pub player: Option<PlayerRef>,
    pub ip: Option<String>,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub expires: Option<i64>,
}

/// An `ip_bans/set` entry (`IpBanlistService.IpBanDto`,
/// IpBanlistService.java:133-147): the address is required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpBanRequest {
    pub ip: String,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub expires: Option<i64>,
}

/// An `operators/add` or `operators/set` entry (`OperatorService.OperatorDto`,
/// OperatorService.java:90-107). Both optional fields fall back to the values
/// `PlayerList.op` (PlayerList.java:537-543) falls back to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorRequest {
    pub player: PlayerRef,
    pub permission_level: Option<CommandPermission>,
    pub bypasses_player_limit: Option<bool>,
}

/// `Message` (Message.java:9-31).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdminMessage {
    pub literal: Option<String>,
    pub translatable: Option<String>,
    pub translatable_params: Option<Vec<String>>,
}

impl AdminMessage {
    pub fn literal(text: impl Into<String>) -> Self {
        Self {
            literal: Some(text.into()),
            ..Self::default()
        }
    }

    /// `Message.asComponent` (Message.java:19-30): the translatable form wins,
    /// and a message with neither field resolves to nothing at all.
    pub fn as_component(&self) -> Option<RichTextComponent> {
        if let Some(key) = &self.translatable {
            let params = self
                .translatable_params
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(component_literal_nbt)
                .collect();
            let component = component_translatable_nbt(key, params, &ComponentStyle::default())
                .expect("a translatable over literal arguments is representable");
            return RichTextComponent::try_from_component_nbt(component).ok();
        }
        self.literal
            .as_ref()
            .map(|text| RichTextComponent::plain(text.clone()))
    }

    fn validate(&self) -> Result<(), AdminError> {
        for text in [self.literal.as_ref(), self.translatable.as_ref()]
            .into_iter()
            .flatten()
        {
            bounded_text(text)?;
        }
        if let Some(params) = &self.translatable_params {
            if params.len() > MAX_MESSAGE_PARAMS {
                return Err(AdminError::invalid_params(format!(
                    "a message carries at most {MAX_MESSAGE_PARAMS} translatable parameters"
                )));
            }
            for param in params {
                bounded_text(param)?;
            }
        }
        Ok(())
    }
}

/// A `server/system_message` payload (`ServerStateService.SystemMessage`,
/// ServerStateService.java:75-84).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemMessageRequest {
    pub message: AdminMessage,
    pub overlay: bool,
    pub receiving_players: Option<Vec<PlayerRef>>,
}

/// One administration request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminRequest {
    /// `server/status` (ServerStateService.java:14-18).
    Status,
    /// `players` (PlayerService.java:17-19).
    Players,
    /// `allowlist` (AllowlistService.java:16-18).
    Allowlist,
    /// `bans` (BanlistService.java:27-34).
    Bans,
    /// `ip_bans` (IpBanlistService.java:26-28).
    IpBans,
    /// `operators` (OperatorService.java:20-22).
    Operators,
    /// The settings readout over the keys this server owns.
    Settings,
    /// `gamerules` (GameRulesService.java:17-21).
    GameRules,
    /// The bounded runtime diagnostics readout. Vanilla has no equivalent
    /// method; `MinecraftServer` exposes the same counters to its own console.
    Diagnostics,
    WorldInfo,
    WorldBlocks {
        dimension: String,
        positions: Vec<BlockPos>,
    },
    WorldSurface {
        dimension: String,
        min_x: i32,
        min_z: i32,
        max_x: i32,
        max_z: i32,
    },
    /// `server/save` (ServerStateService.java:20-22).
    Save {
        flush: bool,
    },
    /// `server/stop` (ServerStateService.java:24-27).
    Stop,
    /// `server/system_message` (ServerStateService.java:29-61).
    SystemMessage(SystemMessageRequest),
    /// `players/kick` (PlayerService.java:21-33).
    KickPlayers(Vec<KickRequest>),
    /// `allowlist/set` (AllowlistService.java:50-62).
    AllowlistSet(Vec<PlayerRef>),
    /// `allowlist/add` (AllowlistService.java:20-29).
    AllowlistAdd(Vec<PlayerRef>),
    /// `allowlist/remove` (AllowlistService.java:37-48).
    AllowlistRemove(Vec<PlayerRef>),
    /// `allowlist/clear` (AllowlistService.java:32-34).
    AllowlistClear,
    /// `bans/set` (BanlistService.java:75-97).
    BansSet(Vec<UserBanRequest>),
    /// `bans/add` (BanlistService.java:37-54).
    BansAdd(Vec<UserBanRequest>),
    /// `bans/remove` (BanlistService.java:61-72).
    BansRemove(Vec<PlayerRef>),
    /// `bans/clear` (BanlistService.java:56-59).
    BansClear,
    /// `ip_bans/set` (IpBanlistService.java:71-87).
    IpBansSet(Vec<IpBanRequest>),
    /// `ip_bans/add` (IpBanlistService.java:30-37).
    IpBansAdd(Vec<IncomingIpBanRequest>),
    /// `ip_bans/remove` (IpBanlistService.java:66-68).
    IpBansRemove(Vec<String>),
    /// `ip_bans/clear` (IpBanlistService.java:61-63).
    IpBansClear,
    /// `operators/set` (OperatorService.java:61-85).
    OperatorsSet(Vec<OperatorRequest>),
    /// `operators/add` (OperatorService.java:41-58).
    OperatorsAdd(Vec<OperatorRequest>),
    /// `operators/remove` (OperatorService.java:29-38).
    OperatorsRemove(Vec<PlayerRef>),
    /// `operators/clear` (OperatorService.java:24-27).
    OperatorsClear,
    /// `serversettings/<key>/set` for the keys this server owns.
    SetSetting(AdminSetting),
    /// `gamerules/update` (GameRulesService.java:32-36).
    UpdateGameRule {
        key: String,
        value: GameRuleValue,
    },
    /// The sole command dispatcher, reached with the caller's own permission
    /// level. Vanilla's management API has no command method; this is the bbb
    /// surface the ledger declares as explicitly highest-privilege.
    ExecuteCommand {
        command: String,
    },
}

impl AdminRequest {
    /// The scope a principal must hold to run this request.
    pub fn required_scope(&self) -> AdminScope {
        match self {
            Self::Status
            | Self::Players
            | Self::Allowlist
            | Self::Bans
            | Self::IpBans
            | Self::Operators
            | Self::Settings
            | Self::GameRules
            | Self::Diagnostics
            | Self::WorldInfo
            | Self::WorldBlocks { .. }
            | Self::WorldSurface { .. } => AdminScope::Read,
            Self::ExecuteCommand { .. } => AdminScope::Command,
            _ => AdminScope::Manage,
        }
    }

    /// Whether applying this request can change canonical state. Read requests
    /// are answered without an idempotency record because replaying them is
    /// free and always current.
    pub fn is_mutation(&self) -> bool {
        self.required_scope() > AdminScope::Read
    }

    /// The stable audit action name.
    pub fn action(&self) -> &'static str {
        match self {
            Self::Status => "server.status",
            Self::Players => "players",
            Self::Allowlist => "allowlist",
            Self::Bans => "bans",
            Self::IpBans => "ip_bans",
            Self::Operators => "operators",
            Self::Settings => "serversettings",
            Self::GameRules => "gamerules",
            Self::Diagnostics => "server.diagnostics",
            Self::WorldInfo => "world.info",
            Self::WorldBlocks { .. } => "world.blocks",
            Self::WorldSurface { .. } => "world.surface",
            Self::Save { .. } => "server.save",
            Self::Stop => "server.stop",
            Self::SystemMessage(_) => "server.system_message",
            Self::KickPlayers(_) => "players.kick",
            Self::AllowlistSet(_) => "allowlist.set",
            Self::AllowlistAdd(_) => "allowlist.add",
            Self::AllowlistRemove(_) => "allowlist.remove",
            Self::AllowlistClear => "allowlist.clear",
            Self::BansSet(_) => "bans.set",
            Self::BansAdd(_) => "bans.add",
            Self::BansRemove(_) => "bans.remove",
            Self::BansClear => "bans.clear",
            Self::IpBansSet(_) => "ip_bans.set",
            Self::IpBansAdd(_) => "ip_bans.add",
            Self::IpBansRemove(_) => "ip_bans.remove",
            Self::IpBansClear => "ip_bans.clear",
            Self::OperatorsSet(_) => "operators.set",
            Self::OperatorsAdd(_) => "operators.add",
            Self::OperatorsRemove(_) => "operators.remove",
            Self::OperatorsClear => "operators.clear",
            Self::SetSetting(_) => "serversettings.set",
            Self::UpdateGameRule { .. } => "gamerules.update",
            Self::ExecuteCommand { .. } => "command.execute",
        }
    }

    /// What this request retains on the heap, for the mailbox's admin-class
    /// byte quota. Vanilla needs no such accounting because its management
    /// server hands the service a live object graph rather than queueing it.
    pub fn retained_bytes(&self) -> usize {
        fn refs(players: &[PlayerRef]) -> usize {
            players
                .iter()
                .map(|player| {
                    player
                        .name
                        .as_ref()
                        .map_or(0, |name| name.capacity() * std::mem::size_of::<u16>())
                })
                .sum::<usize>()
                + std::mem::size_of_val(players)
        }
        fn text(value: &Option<String>) -> usize {
            value.as_ref().map_or(0, String::capacity)
        }
        fn message(value: &AdminMessage) -> usize {
            text(&value.literal)
                + text(&value.translatable)
                + value.translatable_params.as_ref().map_or(0, |params| {
                    params.capacity() * std::mem::size_of::<String>()
                        + params.iter().map(String::capacity).sum::<usize>()
                })
        }
        match self {
            Self::KickPlayers(entries) => {
                entries.capacity() * std::mem::size_of::<KickRequest>()
                    + entries
                        .iter()
                        .map(|entry| {
                            refs(std::slice::from_ref(&entry.player))
                                + entry.message.as_ref().map_or(0, message)
                        })
                        .sum::<usize>()
            }
            Self::AllowlistSet(players)
            | Self::AllowlistAdd(players)
            | Self::AllowlistRemove(players)
            | Self::BansRemove(players)
            | Self::OperatorsRemove(players) => refs(players),
            Self::BansSet(entries) | Self::BansAdd(entries) => {
                entries.capacity() * std::mem::size_of::<UserBanRequest>()
                    + entries
                        .iter()
                        .map(|entry| {
                            refs(std::slice::from_ref(&entry.player))
                                + text(&entry.reason)
                                + text(&entry.source)
                        })
                        .sum::<usize>()
            }
            Self::IpBansSet(entries) => {
                entries.capacity() * std::mem::size_of::<IpBanRequest>()
                    + entries
                        .iter()
                        .map(|entry| {
                            entry.ip.capacity() + text(&entry.reason) + text(&entry.source)
                        })
                        .sum::<usize>()
            }
            Self::IpBansAdd(entries) => {
                entries.capacity() * std::mem::size_of::<IncomingIpBanRequest>()
                    + entries
                        .iter()
                        .map(|entry| {
                            entry
                                .player
                                .as_ref()
                                .map_or(0, |player| refs(std::slice::from_ref(player)))
                                + text(&entry.ip)
                                + text(&entry.reason)
                                + text(&entry.source)
                        })
                        .sum::<usize>()
            }
            Self::IpBansRemove(addresses) => {
                addresses.capacity() * std::mem::size_of::<String>()
                    + addresses.iter().map(String::capacity).sum::<usize>()
            }
            Self::OperatorsSet(entries) | Self::OperatorsAdd(entries) => {
                entries.capacity() * std::mem::size_of::<OperatorRequest>()
                    + entries
                        .iter()
                        .map(|entry| refs(std::slice::from_ref(&entry.player)))
                        .sum::<usize>()
            }
            Self::SystemMessage(request) => {
                message(&request.message) + request.receiving_players.as_deref().map_or(0, refs)
            }
            Self::SetSetting(AdminSetting::Motd(motd)) => motd.capacity(),
            Self::UpdateGameRule { key, .. } => key.capacity(),
            Self::ExecuteCommand { command } => command.capacity(),
            Self::WorldBlocks {
                dimension,
                positions,
            } => dimension.capacity() + positions.capacity() * std::mem::size_of::<BlockPos>(),
            Self::WorldSurface { dimension, .. } => dimension.capacity(),
            _ => 0,
        }
    }

    /// Rejects an over-large or malformed request before it can reach state.
    pub fn validate(&self) -> Result<(), AdminError> {
        match self {
            Self::KickPlayers(entries) => {
                bounded_len(entries.len())?;
                for entry in entries {
                    if let Some(message) = &entry.message {
                        message.validate()?;
                    }
                }
            }
            Self::AllowlistSet(entries)
            | Self::AllowlistAdd(entries)
            | Self::AllowlistRemove(entries)
            | Self::BansRemove(entries)
            | Self::OperatorsRemove(entries) => bounded_len(entries.len())?,
            Self::BansSet(entries) | Self::BansAdd(entries) => {
                bounded_len(entries.len())?;
                for entry in entries {
                    bounded_option(entry.reason.as_ref())?;
                    bounded_option(entry.source.as_ref())?;
                }
            }
            Self::IpBansSet(entries) => {
                bounded_len(entries.len())?;
                for entry in entries {
                    bounded_option(entry.reason.as_ref())?;
                    bounded_option(entry.source.as_ref())?;
                }
            }
            Self::IpBansAdd(entries) => {
                bounded_len(entries.len())?;
                for entry in entries {
                    bounded_option(entry.reason.as_ref())?;
                    bounded_option(entry.source.as_ref())?;
                }
            }
            Self::IpBansRemove(addresses) => {
                bounded_len(addresses.len())?;
                for address in addresses {
                    bounded_text(address)?;
                }
            }
            Self::OperatorsSet(entries) | Self::OperatorsAdd(entries) => {
                bounded_len(entries.len())?;
            }
            Self::SystemMessage(request) => {
                request.message.validate()?;
                if let Some(players) = &request.receiving_players {
                    bounded_len(players.len())?;
                }
            }
            Self::WorldBlocks {
                dimension,
                positions,
            } => {
                bounded_text(dimension)?;
                if !(1..=MAX_WORLD_QUERY_CELLS).contains(&positions.len()) {
                    return Err(AdminError::invalid_params(format!(
                        "world.blocks needs 1 to {MAX_WORLD_QUERY_CELLS} positions"
                    )));
                }
            }
            Self::WorldSurface {
                dimension,
                min_x,
                min_z,
                max_x,
                max_z,
            } => {
                bounded_text(dimension)?;
                let width = i64::from(*max_x) - i64::from(*min_x) + 1;
                let depth = i64::from(*max_z) - i64::from(*min_z) + 1;
                let limit = MAX_WORLD_QUERY_CELLS as i64;
                if width <= 0 || depth <= 0 {
                    return Err(AdminError::invalid_params(
                        "world.surface bounds must be ordered",
                    ));
                }
                if width > limit || depth > limit || width * depth > limit {
                    return Err(AdminError::invalid_params(format!(
                        "world.surface can read at most {MAX_WORLD_QUERY_CELLS} columns"
                    )));
                }
            }
            Self::ExecuteCommand { command }
                if command.chars().count() > MAX_ADMIN_COMMAND_CHARS =>
            {
                return Err(AdminError::invalid_params(format!(
                    "a command is at most {} characters",
                    MAX_ADMIN_COMMAND_CHARS
                )));
            }
            _ => {}
        }
        Ok(())
    }
}

fn bounded_len(len: usize) -> Result<(), AdminError> {
    if len > MAX_REQUEST_ENTRIES {
        return Err(AdminError::invalid_params(format!(
            "a request carries at most {MAX_REQUEST_ENTRIES} entries"
        )));
    }
    Ok(())
}

fn bounded_option(text: Option<&String>) -> Result<(), AdminError> {
    match text {
        Some(text) => bounded_text(text),
        None => Ok(()),
    }
}

fn bounded_text(text: &str) -> Result<(), AdminError> {
    if text.chars().count() > MAX_ADMIN_TEXT_CHARS {
        return Err(AdminError::invalid_params(format!(
            "a management text field is at most {MAX_ADMIN_TEXT_CHARS} characters"
        )));
    }
    Ok(())
}

/// `ServerStateService.ServerState` (ServerStateService.java:64-73).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerStateReport {
    pub started: bool,
    pub players: Vec<PlayerRef>,
    pub version_name: &'static str,
    pub version_protocol: i32,
}

/// `BanlistService.UserBanDto` as an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBanReport {
    pub player: PlayerRef,
    pub ban: BanRecord,
}

/// `IpBanlistService.IpBanDto` as an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpBanReport {
    pub ip: String,
    pub ban: BanRecord,
}

/// `OperatorService.OperatorDto` as an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorReport {
    pub player: PlayerRef,
    pub permission_level: CommandPermission,
    pub bypasses_player_limit: bool,
}

/// `GameRulesService.GameRuleUpdate` as an answer, in its typed form
/// (GameRulesService.java:38-44).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRuleReport {
    pub key: &'static str,
    pub value: GameRuleValue,
}

/// The bounded runtime readout. Every field is a counter the tick already owns,
/// so answering costs no traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticsReport {
    pub tick: u64,
    pub online_players: usize,
    pub max_players: i32,
    pub loaded_chunks: usize,
    pub changed_blocks: usize,
    pub allowlist_entries: usize,
    pub operator_entries: usize,
    pub player_ban_entries: usize,
    pub ip_ban_entries: usize,
    pub processed_admin_operations: u64,
    pub stopping: bool,
}

/// What `command.execute` answers with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReport {
    /// The vanilla command return value, absent when the command failed.
    pub result: Option<i32>,
    /// `sendSuccess`/`sendFailure` messages addressed to the caller.
    pub messages: Vec<RichTextComponent>,
    /// The message the command broadcast to every player, if any.
    pub broadcast: Option<RichTextComponent>,
}

/// One administration answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldInfoReport {
    pub dimensions: Vec<WorldDimensionReport>,
    pub spawn: WorldSpawnReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDimensionReport {
    pub id: String,
    pub min_y: i32,
    pub max_y: i32,
    pub loaded_chunks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSpawnReport {
    pub dimension: String,
    pub position: BlockPos,
}

pub type WorldBlockState = bbb_protocol::block_state_types::VanillaBlockStateIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBlocksReport {
    pub dimension: String,
    pub blocks: Vec<WorldBlockReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBlockReport {
    pub position: BlockPos,
    pub state: Option<WorldBlockState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSurfaceReport {
    pub dimension: String,
    pub columns: Vec<WorldSurfaceColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSurfaceColumn {
    pub x: i32,
    pub z: i32,
    pub loaded: bool,
    pub top_y: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminResult {
    Status(ServerStateReport),
    Players(Vec<PlayerRef>),
    Allowlist(Vec<PlayerRef>),
    Bans(Vec<UserBanReport>),
    IpBans(Vec<IpBanReport>),
    Operators(Vec<OperatorReport>),
    Settings(Vec<AdminSetting>),
    Setting(AdminSetting),
    GameRules(Vec<GameRuleReport>),
    GameRule(GameRuleReport),
    Diagnostics(DiagnosticsReport),
    WorldInfo(WorldInfoReport),
    WorldBlocks(WorldBlocksReport),
    WorldSurface(WorldSurfaceReport),
    Kicked(Vec<PlayerRef>),
    Saving(bool),
    Stopping(bool),
    Sent(bool),
    Command(CommandReport),
}

/// Why an administration request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminError {
    /// The granted scope does not satisfy the request.
    ScopeDenied {
        required: AdminScope,
        granted: AdminScope,
    },
    /// The request sat in the mailbox past the deadline the caller set.
    DeadlineExceeded { deadline_tick: u64, tick: u64 },
    /// The request is malformed, over-bounded, or names something this server
    /// does not own.
    InvalidParams(String),
}

impl AdminError {
    pub fn invalid_params(detail: impl Into<String>) -> Self {
        Self::InvalidParams(detail.into())
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::ScopeDenied { .. } => "scope_denied",
            Self::DeadlineExceeded { .. } => "deadline_exceeded",
            Self::InvalidParams(_) => "invalid_params",
        }
    }
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScopeDenied { required, granted } => write!(
                formatter,
                "scope {} does not satisfy {}",
                granted.name(),
                required.name()
            ),
            Self::DeadlineExceeded {
                deadline_tick,
                tick,
            } => write!(
                formatter,
                "deadline tick {deadline_tick} passed before tick {tick}"
            ),
            Self::InvalidParams(detail) => formatter.write_str(detail),
        }
    }
}

/// One applied request: the answer and the audit record in one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminOutcome {
    pub operation_id: u64,
    pub request_id: String,
    pub principal: String,
    pub scope: AdminScope,
    pub action: &'static str,
    /// The tick the request was applied on.
    pub tick: u64,
    /// True when the answer came from the idempotency record rather than from
    /// a second application.
    pub replayed: bool,
    pub result: Result<AdminResult, AdminError>,
}

/// The identity a stored user list entry was created for, together with the
/// address the ip-ban surface may need. Produced by the world, consumed by the
/// admin state, so identity resolution never reaches back into the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlinePlayer {
    pub user: NameAndId,
    pub address: Option<String>,
}

impl OnlinePlayer {
    pub fn id(&self) -> Uuid {
        self.user.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scope_ladder_is_ordered() {
        assert!(AdminScope::Command > AdminScope::Manage);
        assert!(AdminScope::Manage > AdminScope::Read);
        for scope in [AdminScope::Read, AdminScope::Manage, AdminScope::Command] {
            assert_eq!(AdminScope::by_name(scope.name()), Some(scope));
        }
        assert_eq!(AdminScope::by_name("owner"), None);
    }

    #[test]
    fn read_requests_need_no_mutation_authority() {
        assert_eq!(AdminRequest::Status.required_scope(), AdminScope::Read);
        assert!(!AdminRequest::Status.is_mutation());
        assert_eq!(
            AdminRequest::AllowlistClear.required_scope(),
            AdminScope::Manage
        );
        assert!(AdminRequest::AllowlistClear.is_mutation());
        assert_eq!(
            AdminRequest::ExecuteCommand {
                command: "list".to_string()
            }
            .required_scope(),
            AdminScope::Command
        );
    }

    #[test]
    fn over_bounded_requests_are_refused() {
        let players = vec![PlayerRef::by_name("a"); MAX_REQUEST_ENTRIES + 1];
        assert!(AdminRequest::AllowlistAdd(players).validate().is_err());
        assert!(AdminRequest::ExecuteCommand {
            command: "a".repeat(MAX_ADMIN_COMMAND_CHARS + 1),
        }
        .validate()
        .is_err());
        assert!(AdminRequest::SystemMessage(SystemMessageRequest {
            message: AdminMessage::literal("m".repeat(MAX_ADMIN_TEXT_CHARS + 1)),
            overlay: false,
            receiving_players: None,
        })
        .validate()
        .is_err());
    }

    #[test]
    fn a_translatable_message_wins_over_a_literal() {
        let message = AdminMessage {
            literal: Some("ignored".to_string()),
            translatable: Some("multiplayer.disconnect.kicked".to_string()),
            translatable_params: None,
        };
        let component = message.as_component().expect("component");
        assert!(format!("{:?}", component.component()).contains("multiplayer.disconnect.kicked"));
        assert_eq!(
            AdminMessage::literal("plain")
                .as_component()
                .map(|component| component.text().to_string()),
            Some("plain".to_string())
        );
        assert_eq!(AdminMessage::default().as_component(), None);
    }
}
