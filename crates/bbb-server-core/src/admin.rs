//! The authoritative server administration domain.
//!
//! Vanilla owner: `net.minecraft.server.jsonrpc`. `MinecraftApi` hands its
//! services a `DedicatedServer` and runs every one of them on the server thread
//! through `MinecraftExecutorServiceImpl.submit`; the services themselves read
//! and write `PlayerList`'s four stored user lists, the level's `GameRules` and
//! `DedicatedServerProperties`.
//!
//! bbb keeps the semantics and replaces the transport: management requests
//! arrive over MCP, enter the same bounded world mailbox the network producer
//! uses, and apply inside `ServerWorld::tick`. That is the same barrier
//! vanilla's executor reaches, so no management operation can observe or mutate
//! a half-ticked world.
//!
//! Layout: [`identity`] owns player references, [`stored_user_list`] owns the
//! four documents, [`settings`] owns the mutable server settings, [`protocol`]
//! owns the request/answer surface, and [`state`] is the single place a request
//! applies.

pub mod identity;
pub mod protocol;
pub mod settings;
pub mod state;
pub mod stored_user_list;

pub use identity::{IdentityDirectory, NameAndId, PlayerRef};
pub use protocol::{
    AdminEnvelope, AdminError, AdminMessage, AdminOutcome, AdminRequest, AdminResult, AdminScope,
    CommandReport, DiagnosticsReport, GameRuleReport, IncomingIpBanRequest, IpBanReport,
    IpBanRequest, KickRequest, OnlinePlayer, OperatorReport, OperatorRequest, ServerStateReport,
    SystemMessageRequest, UserBanReport, UserBanRequest, MAX_MESSAGE_PARAMS, MAX_REQUEST_ENTRIES,
};
pub use settings::{
    clamp_broadcast_range, difficulty_by_name, game_type_by_name, permission_level_by_id,
    AdminSetting, AdminSettings, PersistedSettings, SettingRejected, ADMIN_SETTING_KEYS,
    DEFAULT_ENTITY_BROADCAST_RANGE_PERCENTAGE, DEFAULT_MOTD, MAX_DISTANCE_CHUNKS,
    MIN_DISTANCE_CHUNKS, PERSISTED_SETTINGS_FILE,
};
pub use state::{AdminEffect, AdminState, AdminWorldView, AdmissionSnapshot};
pub use stored_user_list::{
    format_ban_date, format_ban_expiry_date, format_instant_iso8601, parse_ban_date,
    parse_instant_iso8601, valid_ip_literal, BanRecord, OperatorEntry, PlayerBanEntry,
    StoredUserListDocuments, StoredUserListError, StoredUserLists, ALLOWLIST_FILE, IP_BANS_FILE,
    MAX_ADMIN_TEXT_CHARS, MAX_LIST_ENTRIES, OPERATORS_FILE, PLAYER_BANS_FILE,
};
