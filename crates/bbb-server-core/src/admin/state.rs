//! The authoritative administration state and the one place requests apply.
//!
//! Vanilla owners: `PlayerList` holds the four stored user lists and the
//! op/allowlist decisions (`PlayerList.java:334-362`, `:537-582`),
//! `MinecraftServer.kickUnlistedPlayers` (:1656-1667) enforces the allowlist,
//! and the `net.minecraft.server.jsonrpc.methods` services sequence the
//! multi-step list operations. Vanilla runs those services on the server thread
//! through `MinecraftExecutorServiceImpl.submit`; bbb reaches the same barrier
//! by carrying the request through the bounded world mailbox into `tick`.
//!
//! This module owns no world state and performs no I/O: it reads a projected
//! [`AdminWorldView`] and returns [`AdminEffect`]s for the world to run, so the
//! management semantics stay testable without a server.

use std::collections::{BTreeMap, VecDeque};

use bbb_protocol::{
    difficulties::Difficulty,
    game_rules::{vanilla_game_rule, GameRuleCatalogEntry, GameRuleValue, VANILLA_GAME_RULES},
    game_types::GameType,
    RichTextComponent, MC_VERSION, PROTOCOL_VERSION,
};
use uuid::Uuid;

use crate::commands::{self, BanKind, CommandPermission, CommandRun, CommandSource};

use super::identity::{IdentityDirectory, NameAndId, PlayerRef};
use super::protocol::{
    AdminEnvelope, AdminError, AdminMessage, AdminOutcome, AdminRequest, AdminResult,
    CommandReport, DiagnosticsReport, GameRuleReport, IncomingIpBanRequest, IpBanReport,
    IpBanRequest, KickRequest, OnlinePlayer, OperatorReport, OperatorRequest, ServerStateReport,
    SystemMessageRequest, UserBanReport, UserBanRequest,
};
use super::settings::{AdminSetting, AdminSettings};
use super::stored_user_list::{
    format_ban_expiry_date, valid_ip_literal, BanRecord, OperatorEntry, PlayerBanEntry,
    StoredUserListDocuments, StoredUserLists,
};

/// `BanlistService.BAN_SOURCE` / `IpBanlistService.BAN_SOURCE`
/// (BanlistService.java:25, IpBanlistService.java:24).
const MANAGEMENT_SOURCE: &str = "Management server";
/// `PlayerService.DEFAULT_KICK_MESSAGE` (PlayerService.java:15).
const DEFAULT_KICK_KEY: &str = "multiplayer.disconnect.kicked";
/// `MinecraftServer.kickUnlistedPlayers` (:1663) and `PlayerList.canPlayerLogin`
/// (:345).
const NOT_ALLOWLISTED_KEY: &str = "multiplayer.disconnect.not_whitelisted";
/// `BanlistService.add` (BanlistService.java:48).
const BANNED_KEY: &str = "multiplayer.disconnect.banned";
/// `IpBanlistService.add` (IpBanlistService.java:35).
const IP_BANNED_KEY: &str = "multiplayer.disconnect.ip_banned";
/// How many recent operations the idempotency record keeps per server. A
/// management client that retries an unanswered request finds its record here;
/// anything older has certainly been answered.
const IDEMPOTENCY_RECORDS: usize = 256;

/// The world facts one administration request may read.
#[derive(Debug, Clone, Copy)]
pub struct AdminWorldView<'a> {
    pub tick: u64,
    pub stopping: bool,
    pub online: &'a [OnlinePlayer],
    pub loaded_chunks: usize,
    pub changed_blocks: usize,
    /// What `command.execute` ran to. The world runs an admitted command
    /// against itself before the request applies.
    pub command: Option<&'a CommandRun>,
}

/// What the world must do after a request applies.
#[derive(Debug, Clone, PartialEq)]
pub enum AdminEffect {
    /// `ServerGamePacketListenerImpl.disconnect` with the given reason.
    Disconnect {
        player: Uuid,
        reason: RichTextComponent,
    },
    /// `ServerPlayer.sendSystemMessage(component, overlay)`.
    SystemMessage {
        player: Uuid,
        message: RichTextComponent,
        overlay: bool,
    },
    /// `MinecraftServerStateServiceImpl.broadcastSystemMessage`.
    BroadcastSystemMessage {
        message: RichTextComponent,
        overlay: bool,
    },
    /// `PlayerList.sendPlayerPermissionLevel` (PlayerList.java:559-572).
    PermissionLevelChanged {
        player: Uuid,
        level: CommandPermission,
    },
    /// `MinecraftServer.setDifficulty` (MinecraftServer.java:1288-1304).
    DifficultyChanged(Difficulty),
    /// `DedicatedServer.setGameMode` (`DedicatedServer.java:809-813`).
    GameModeChanged(GameType),
    /// `PlayerList.setViewDistance` (PlayerList.java:801-808), which
    /// `DedicatedServer.setViewDistance` (DedicatedServer.java:403-406) calls
    /// after it writes the property.
    ViewDistanceChanged(i32),
    /// `PlayerList.setSimulationDistance` (PlayerList.java:810-817), which
    /// `DedicatedServer.setSimulationDistance` (DedicatedServer.java:412-415)
    /// calls after it writes the property.
    SimulationDistanceChanged(i32),
    /// `MinecraftServer.saveEverything` (`server/save`).
    Save { flush: bool },
    /// `MinecraftServer.halt` (`server/stop`).
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdempotencyRecord {
    principal: String,
    request_id: String,
    outcome: AdminOutcome,
}

/// The administration state one server owns.
pub struct AdminState {
    lists: StoredUserLists,
    settings: AdminSettings,
    /// The `difficulty` property: only `serversettings/difficulty/set` writes
    /// it back (`DedicatedServer.setDifficulty`, `DedicatedServer.java:389-392`),
    /// while `/difficulty` changes the level alone.
    saved_difficulty: Difficulty,
    /// The `gamemode` property: only `serversettings/game_mode/set` writes it
    /// back (`DedicatedServer.setGameMode`, `DedicatedServer.java:809-813`),
    /// while `/defaultgamemode` changes the live default alone.
    saved_game_mode: GameType,
    game_rules: BTreeMap<&'static str, GameRuleValue>,
    /// Bumped whenever a stored user list changes, so the runtime knows when to
    /// rewrite the four documents.
    list_revision: u64,
    settings_revision: u64,
    processed_operations: u64,
    records: VecDeque<IdempotencyRecord>,
}

/// What `PlayerList.canPlayerLogin` (`PlayerList.java:334-362`) reads, copied
/// out of the world so the login phase can refuse before configuration
/// (`ServerLoginPacketListenerImpl.java:140-145`). The world's join repeats
/// the check, as `ServerConfigurationPacketListenerImpl.java:172` does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdmissionSnapshot {
    lists: StoredUserLists,
    use_allowlist: bool,
    max_players: usize,
    online_players: usize,
}

impl AdmissionSnapshot {
    pub fn refusal(
        &self,
        user: &NameAndId,
        address: Option<&str>,
        now: i64,
    ) -> Option<RichTextComponent> {
        login_refusal(&self.lists, self.use_allowlist, user, address, now).or_else(|| {
            (self.online_players >= self.max_players
                && !bypasses_player_limit(&self.lists, user.id))
            .then(|| commands::disconnect_reason("multiplayer.disconnect.server_full"))
        })
    }
}

fn login_refusal(
    lists: &StoredUserLists,
    use_allowlist: bool,
    user: &NameAndId,
    address: Option<&str>,
    now: i64,
) -> Option<RichTextComponent> {
    if let Some(entry) = lists.player_bans.get(&user.id) {
        if !entry.ban.has_expired(now) {
            return Some(ban_refusal(BanKind::Player, &entry.ban));
        }
    }
    if !is_allowlisted(lists, use_allowlist, user) {
        return Some(commands::disconnect_reason(NOT_ALLOWLISTED_KEY));
    }
    if let Some(address) = address {
        if let Some(record) = lists.ip_bans.get(address) {
            if !record.has_expired(now) {
                return Some(ban_refusal(BanKind::Ip, record));
            }
        }
    }
    None
}

fn is_allowlisted(lists: &StoredUserLists, use_allowlist: bool, user: &NameAndId) -> bool {
    !use_allowlist
        || lists.operators.contains_key(&user.id)
        || lists.allowlist.contains_key(&user.id)
}

fn bypasses_player_limit(lists: &StoredUserLists, player: Uuid) -> bool {
    lists
        .operators
        .get(&player)
        .is_some_and(|entry| entry.bypasses_player_limit)
}

impl AdminState {
    /// Builds the state from the documents the runtime loaded, the CLI-derived
    /// settings, and the feature flags the datapack surface enabled.
    ///
    /// `GameRules(FeatureFlagSet)` (GameRules.java:112-114) starts every enabled
    /// rule at its registered default and keeps no disabled rule at all.
    pub fn new(settings: AdminSettings, lists: StoredUserLists, enabled_features: &[&str]) -> Self {
        let game_rules = VANILLA_GAME_RULES
            .iter()
            .filter(|entry| entry.is_enabled(enabled_features))
            .map(|entry| (entry.resource_id, entry.default_value))
            .collect();
        Self {
            lists,
            saved_difficulty: settings.difficulty,
            saved_game_mode: settings.game_mode,
            settings,
            game_rules,
            list_revision: 0,
            settings_revision: 0,
            processed_operations: 0,
            records: VecDeque::new(),
        }
    }

    pub fn settings(&self) -> &AdminSettings {
        &self.settings
    }

    pub fn lists(&self) -> &StoredUserLists {
        &self.lists
    }

    pub fn saved_difficulty(&self) -> Difficulty {
        self.saved_difficulty
    }

    /// `MinecraftServer.setDifficulty` writing `ServerLevelData.setDifficulty`.
    ///
    /// The difficulty is one value with two writers — the gamemaster-gated
    /// packet handler and `serversettings/difficulty/set` — so the world routes
    /// its own writer through here rather than keeping a second field.
    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        if self.settings.difficulty != difficulty {
            self.settings.difficulty = difficulty;
            self.settings_revision += 1;
        }
    }

    pub fn saved_game_mode(&self) -> GameType {
        self.saved_game_mode
    }

    pub fn set_game_mode(&mut self, game_mode: GameType) {
        if self.settings.game_mode != game_mode {
            self.settings.game_mode = game_mode;
            self.settings_revision += 1;
        }
    }

    pub fn list_revision(&self) -> u64 {
        self.list_revision
    }

    pub fn settings_revision(&self) -> u64 {
        self.settings_revision
    }

    /// The four documents as the runtime should write them.
    pub fn documents(&self) -> StoredUserListDocuments {
        self.lists.documents()
    }

    /// `PlayerList.isOp` reduced to what this server models: the operator list
    /// alone. There is no singleplayer owner and no `allowCommandsForAllPlayers`
    /// here, so an absent entry is the baseline level.
    pub fn permission_level(&self, player: Uuid) -> CommandPermission {
        self.lists
            .operators
            .get(&player)
            .map_or(CommandPermission::All, |entry| entry.level)
    }

    pub fn is_operator(&self, player: Uuid) -> bool {
        self.lists.operators.contains_key(&player)
    }

    /// `MinecraftServer.setUsingWhitelist` (MinecraftServer.java:1791-1793):
    /// the bare field write `/whitelist on` and `/whitelist off` make. The kick
    /// that follows the enabling one is a separate call in vanilla too
    /// (WhitelistCommand.java:126), so it stays a separate effect here.
    pub fn set_using_whitelist(&mut self, enabled: bool) {
        if self.settings.use_allowlist != enabled {
            self.settings.use_allowlist = enabled;
            self.settings_revision += 1;
        }
    }

    /// `GameRules.set` (GameRuleCommand.java:38) for a rule the caller already
    /// resolved against the generated catalog. A rule this world's feature set
    /// does not enable is not registered at all (`GameRules(FeatureFlagSet)`),
    /// and writing one is refused rather than inventing an entry.
    pub fn set_game_rule(&mut self, resource_id: &'static str, value: GameRuleValue) -> bool {
        let Some(slot) = self.game_rules.get_mut(resource_id) else {
            return false;
        };
        *slot = value;
        true
    }

    /// `GameRules.get` (`gamerules/GameRules.java:124-131`) for one rule, as
    /// an option rather than the `IllegalArgumentException` vanilla throws for
    /// a rule the level's feature set left out.
    pub fn game_rule_value(&self, resource_id: &str) -> Option<GameRuleValue> {
        self.game_rules.get(resource_id).copied()
    }

    /// Every enabled rule and its current value, which is the projection the
    /// command dispatcher reads `/gamerule` against.
    pub fn game_rule_values(&self) -> Vec<(&'static str, GameRuleValue)> {
        self.game_rules
            .iter()
            .map(|(key, value)| (*key, *value))
            .collect()
    }

    /// `PlayerList.op` (PlayerList.java:537-548) as `OpCommand` calls it: the
    /// server's own default level, and whatever bypass flag the entry already
    /// carried.
    pub fn op_player(
        &mut self,
        user: NameAndId,
        view: AdminWorldView<'_>,
        effects: &mut Vec<AdminEffect>,
    ) {
        let request = OperatorRequest {
            player: PlayerRef::from(&user),
            permission_level: None,
            bypasses_player_limit: None,
        };
        self.op(user, &request, view, effects);
    }

    /// `PlayerList.deop` (PlayerList.java:550-557).
    pub fn deop_player(
        &mut self,
        player: &Uuid,
        view: AdminWorldView<'_>,
        effects: &mut Vec<AdminEffect>,
    ) {
        self.deop(player, view, effects);
    }

    /// `UserWhiteList.add` (WhitelistCommand.java:85).
    pub fn add_whitelisted(&mut self, user: NameAndId) {
        self.allowlist_add(user);
    }

    /// `UserWhiteList.remove` (WhitelistCommand.java:105).
    pub fn remove_whitelisted(&mut self, player: &Uuid) {
        self.allowlist_remove(player);
    }

    /// `UserBanList.add` (BanPlayerCommands.java:42-43): a command ban carries
    /// the source's text name and no expiry.
    pub fn add_player_ban(
        &mut self,
        user: NameAndId,
        source: bbb_protocol::JavaString,
        reason: Option<bbb_protocol::JavaString>,
        now: i64,
    ) {
        self.player_ban_add(
            user,
            BanRecord {
                created: now,
                source,
                expires: None,
                reason,
            },
        );
    }

    /// `UserBanList.remove` (PardonCommand.java:36).
    pub fn remove_player_ban(&mut self, player: &Uuid) {
        if self.lists.player_bans.remove(player).is_some() {
            self.list_revision += 1;
        }
    }

    /// `IpBanList.add` (BanIpCommands.java:58-59).
    pub fn add_ip_ban(
        &mut self,
        address: String,
        source: bbb_protocol::JavaString,
        reason: Option<bbb_protocol::JavaString>,
        now: i64,
    ) {
        self.ip_ban_add(
            address,
            BanRecord {
                created: now,
                source,
                expires: None,
                reason,
            },
        );
    }

    /// `IpBanList.remove` (PardonIpCommand.java:40).
    pub fn remove_ip_ban(&mut self, address: &str) {
        if self.lists.ip_bans.remove(address).is_some() {
            self.list_revision += 1;
        }
    }

    /// `MinecraftServer.kickUnlistedPlayers` (MinecraftServer.java:1656-1667),
    /// which the allowlist commands and `/deop` all end with.
    pub fn kick_unlisted(&self, view: AdminWorldView<'_>, effects: &mut Vec<AdminEffect>) {
        self.kick_unlisted_players(view, effects);
    }

    /// `PlayerList.canPlayerLogin` (PlayerList.java:334-362) in its declared
    /// order: player ban, then allowlist, then ip ban. The capacity check stays
    /// with the world, which owns the player map.
    pub fn login_refusal(
        &self,
        user: &NameAndId,
        address: Option<&str>,
        now: i64,
    ) -> Option<RichTextComponent> {
        login_refusal(&self.lists, self.settings.use_allowlist, user, address, now)
    }

    /// `PlayerList.isWhiteListed` (PlayerList.java:574-576).
    pub fn is_allowlisted(&self, user: &NameAndId) -> bool {
        is_allowlisted(&self.lists, self.settings.use_allowlist, user)
    }

    /// `PlayerList.canBypassPlayerLimit` (PlayerList.java:837-839) is always
    /// false in vanilla's dedicated server; the operator entry's own flag is
    /// what a management client sets, and this server honors it.
    pub fn bypasses_player_limit(&self, player: Uuid) -> bool {
        bypasses_player_limit(&self.lists, player)
    }

    /// The login check as of now, for the login phase off the world thread.
    pub fn admission_snapshot(&self, online_players: usize) -> AdmissionSnapshot {
        AdmissionSnapshot {
            lists: self.lists.clone(),
            use_allowlist: self.settings.use_allowlist,
            max_players: usize::try_from(self.settings.max_players).unwrap_or(usize::MAX),
            online_players,
        }
    }

    /// Applies one request at the tick barrier.
    pub fn apply(
        &mut self,
        envelope: &AdminEnvelope,
        request: &AdminRequest,
        view: AdminWorldView<'_>,
        effects: &mut Vec<AdminEffect>,
    ) -> AdminOutcome {
        match self.admit(envelope, request, view.tick) {
            Ok(()) => self.apply_admitted(envelope, request, view, None, effects),
            Err(outcome) => *outcome,
        }
    }

    /// The answers a request gets before anything runs: a mutation already
    /// recorded is answered from its record, and scope, deadline and validation
    /// refuse in that order.
    pub fn admit(
        &self,
        envelope: &AdminEnvelope,
        request: &AdminRequest,
        tick: u64,
    ) -> Result<(), Box<AdminOutcome>> {
        let action = request.action();
        let required = request.required_scope();
        if envelope.scope < required {
            return Err(Box::new(self.outcome(
                envelope,
                action,
                tick,
                Err(AdminError::ScopeDenied {
                    required,
                    granted: envelope.scope,
                }),
            )));
        }
        if request.is_mutation() {
            if let Some(recorded) = self.recorded(envelope) {
                if envelope.scope < recorded.scope {
                    return Err(Box::new(self.outcome(
                        envelope,
                        action,
                        tick,
                        Err(AdminError::ScopeDenied {
                            required: recorded.scope,
                            granted: envelope.scope,
                        }),
                    )));
                }
                return Err(Box::new(AdminOutcome {
                    operation_id: envelope.operation_id,
                    replayed: true,
                    ..recorded
                }));
            }
        }
        if tick > envelope.deadline_tick {
            return Err(Box::new(self.outcome(
                envelope,
                action,
                tick,
                Err(AdminError::DeadlineExceeded {
                    deadline_tick: envelope.deadline_tick,
                    tick,
                }),
            )));
        }
        if let Err(error) = request.validate() {
            return Err(Box::new(self.outcome(envelope, action, tick, Err(error))));
        }
        Ok(())
    }

    /// [`Self::apply`] for a request [`Self::admit`] already let through.
    pub fn apply_admitted(
        &mut self,
        envelope: &AdminEnvelope,
        request: &AdminRequest,
        view: AdminWorldView<'_>,
        world_query: Option<Result<AdminResult, AdminError>>,
        effects: &mut Vec<AdminEffect>,
    ) -> AdminOutcome {
        // `StoredUserList.get` prunes expired entries before every read
        // (StoredUserList.java:61-63), so the answer never names a lapsed ban.
        if self.lists.remove_expired(envelope.now) {
            self.list_revision += 1;
        }
        self.processed_operations += 1;
        let result = world_query.unwrap_or_else(|| self.dispatch(envelope, request, view, effects));
        let outcome = self.outcome(envelope, request.action(), view.tick, result);
        if request.is_mutation() {
            self.record(envelope, &outcome);
        }
        outcome
    }

    fn outcome(
        &self,
        envelope: &AdminEnvelope,
        action: &'static str,
        tick: u64,
        result: Result<AdminResult, AdminError>,
    ) -> AdminOutcome {
        AdminOutcome {
            operation_id: envelope.operation_id,
            request_id: envelope.request_id.clone(),
            principal: envelope.principal.clone(),
            scope: envelope.scope,
            action,
            tick,
            replayed: false,
            result,
        }
    }

    fn recorded(&self, envelope: &AdminEnvelope) -> Option<AdminOutcome> {
        self.records
            .iter()
            .find(|record| {
                record.principal == envelope.principal && record.request_id == envelope.request_id
            })
            .map(|record| record.outcome.clone())
    }

    fn record(&mut self, envelope: &AdminEnvelope, outcome: &AdminOutcome) {
        if self.records.len() == IDEMPOTENCY_RECORDS {
            self.records.pop_front();
        }
        self.records.push_back(IdempotencyRecord {
            principal: envelope.principal.clone(),
            request_id: envelope.request_id.clone(),
            outcome: outcome.clone(),
        });
    }

    fn dispatch(
        &mut self,
        envelope: &AdminEnvelope,
        request: &AdminRequest,
        view: AdminWorldView<'_>,
        effects: &mut Vec<AdminEffect>,
    ) -> Result<AdminResult, AdminError> {
        Ok(match request {
            AdminRequest::Status => AdminResult::Status(ServerStateReport {
                // `ServerStateService.status` (ServerStateService.java:14-18)
                // answers NOT_STARTED until the server is ready; a stopping
                // server is no longer serving either.
                started: !view.stopping,
                players: online_refs(view.online),
                version_name: MC_VERSION,
                version_protocol: PROTOCOL_VERSION,
            }),
            AdminRequest::Players => AdminResult::Players(online_refs(view.online)),
            AdminRequest::Allowlist => AdminResult::Allowlist(self.allowlist_refs()),
            AdminRequest::Bans => AdminResult::Bans(self.ban_reports()),
            AdminRequest::IpBans => AdminResult::IpBans(self.ip_ban_reports()),
            AdminRequest::Operators => AdminResult::Operators(self.operator_reports()),
            AdminRequest::Settings => AdminResult::Settings(self.settings.all()),
            AdminRequest::GameRules => AdminResult::GameRules(self.game_rule_reports()),
            AdminRequest::Diagnostics => AdminResult::Diagnostics(DiagnosticsReport {
                tick: view.tick,
                online_players: view.online.len(),
                max_players: self.settings.max_players,
                loaded_chunks: view.loaded_chunks,
                changed_blocks: view.changed_blocks,
                allowlist_entries: self.lists.allowlist.len(),
                operator_entries: self.lists.operators.len(),
                player_ban_entries: self.lists.player_bans.len(),
                ip_ban_entries: self.lists.ip_bans.len(),
                processed_admin_operations: self.processed_operations,
                stopping: view.stopping,
            }),
            AdminRequest::Save { flush } => {
                effects.push(AdminEffect::Save { flush: *flush });
                AdminResult::Saving(true)
            }
            AdminRequest::Stop => {
                effects.push(AdminEffect::Stop);
                AdminResult::Stopping(true)
            }
            AdminRequest::SystemMessage(request) => {
                AdminResult::Sent(system_message(request, view, effects))
            }
            AdminRequest::KickPlayers(entries) => {
                AdminResult::Kicked(kick_players(entries, view, effects))
            }
            AdminRequest::AllowlistAdd(players) => {
                // `AllowlistService.add` (AllowlistService.java:20-29) adds
                // without kicking; only remove and set enforce afterwards.
                let directory = self.directory_users(view);
                for user in resolve_all(&directory, players) {
                    self.allowlist_add(user);
                }
                AdminResult::Allowlist(self.allowlist_refs())
            }
            AdminRequest::AllowlistRemove(players) => {
                let directory = self.directory_users(view);
                for user in resolve_all(&directory, players) {
                    self.allowlist_remove(&user.id);
                }
                self.kick_unlisted_players(view, effects);
                AdminResult::Allowlist(self.allowlist_refs())
            }
            AdminRequest::AllowlistClear => {
                if !self.lists.allowlist.is_empty() {
                    self.lists.allowlist.clear();
                    self.list_revision += 1;
                }
                AdminResult::Allowlist(self.allowlist_refs())
            }
            AdminRequest::AllowlistSet(players) => {
                // `AllowlistService.set` (AllowlistService.java:50-62): remove
                // what is no longer listed, add what is newly listed, enforce.
                let directory = self.directory_users(view);
                let target = resolve_all(&directory, players);
                let removed = self
                    .lists
                    .allowlist
                    .keys()
                    .copied()
                    .filter(|id| !target.iter().any(|user| user.id == *id))
                    .collect::<Vec<_>>();
                for id in removed {
                    self.allowlist_remove(&id);
                }
                for user in target {
                    self.allowlist_add(user);
                }
                self.kick_unlisted_players(view, effects);
                AdminResult::Allowlist(self.allowlist_refs())
            }
            AdminRequest::BansAdd(bans) => {
                let directory = self.directory_users(view);
                for request in bans {
                    let Some(user) = resolve_one(&directory, &request.player) else {
                        continue;
                    };
                    self.ban_add(user.clone(), request, envelope.now);
                    if view.online.iter().any(|player| player.id() == user.id) {
                        effects.push(AdminEffect::Disconnect {
                            player: user.id,
                            reason: commands::disconnect_reason(BANNED_KEY),
                        });
                    }
                }
                AdminResult::Bans(self.ban_reports())
            }
            AdminRequest::BansRemove(players) => {
                let directory = self.directory_users(view);
                for user in resolve_all(&directory, players) {
                    if self.lists.player_bans.remove(&user.id).is_some() {
                        self.list_revision += 1;
                    }
                }
                AdminResult::Bans(self.ban_reports())
            }
            AdminRequest::BansClear => {
                if !self.lists.player_bans.is_empty() {
                    self.lists.player_bans.clear();
                    self.list_revision += 1;
                }
                AdminResult::Bans(self.ban_reports())
            }
            AdminRequest::BansSet(bans) => {
                let directory = self.directory_users(view);
                let mut target = Vec::new();
                for request in bans {
                    if let Some(user) = resolve_one(&directory, &request.player) {
                        target.push((user, request));
                    }
                }
                let removed = self
                    .lists
                    .player_bans
                    .iter()
                    .filter(|(id, entry)| {
                        !target.iter().any(|(user, request)| {
                            user.id == **id && ban_matches(&entry.ban, request)
                        })
                    })
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>();
                for id in removed {
                    self.lists.player_bans.remove(&id);
                    self.list_revision += 1;
                }
                for (user, request) in target {
                    let already = self
                        .lists
                        .player_bans
                        .get(&user.id)
                        .is_some_and(|entry| ban_matches(&entry.ban, request));
                    if already {
                        continue;
                    }
                    self.ban_add(user.clone(), request, envelope.now);
                    if view.online.iter().any(|player| player.id() == user.id) {
                        effects.push(AdminEffect::Disconnect {
                            player: user.id,
                            reason: commands::disconnect_reason(BANNED_KEY),
                        });
                    }
                }
                AdminResult::Bans(self.ban_reports())
            }
            AdminRequest::IpBansAdd(bans) => {
                for request in bans {
                    let Some((ip, record)) = self.incoming_ip_ban(request, view, envelope.now)
                    else {
                        continue;
                    };
                    self.ip_ban_add(ip.clone(), record);
                    disconnect_address(&ip, view, IP_BANNED_KEY, effects);
                }
                AdminResult::IpBans(self.ip_ban_reports())
            }
            AdminRequest::IpBansRemove(addresses) => {
                for address in addresses {
                    if self.lists.ip_bans.remove(address).is_some() {
                        self.list_revision += 1;
                    }
                }
                AdminResult::IpBans(self.ip_ban_reports())
            }
            AdminRequest::IpBansClear => {
                if !self.lists.ip_bans.is_empty() {
                    self.lists.ip_bans.clear();
                    self.list_revision += 1;
                }
                AdminResult::IpBans(self.ip_ban_reports())
            }
            AdminRequest::IpBansSet(bans) => {
                // `IpBanlistService.set` (IpBanlistService.java:71-87) drops
                // every entry whose address is not an IP literal before it
                // diffs.
                let target = bans
                    .iter()
                    .filter(|request| valid_ip_literal(&request.ip))
                    .map(|request| (request.ip.clone(), ip_ban_record(request, envelope.now)))
                    .collect::<Vec<_>>();
                let removed = self
                    .lists
                    .ip_bans
                    .iter()
                    .filter(|(ip, record)| {
                        !target.iter().any(|(target_ip, target)| {
                            *ip == target_ip && ip_ban_matches(record, target)
                        })
                    })
                    .map(|(ip, _)| ip.clone())
                    .collect::<Vec<_>>();
                for ip in removed {
                    self.lists.ip_bans.remove(&ip);
                    self.list_revision += 1;
                }
                for (ip, record) in target {
                    let already = self
                        .lists
                        .ip_bans
                        .get(&ip)
                        .is_some_and(|current| ip_ban_matches(current, &record));
                    if already {
                        continue;
                    }
                    self.ip_ban_add(ip.clone(), record);
                    disconnect_address(&ip, view, IP_BANNED_KEY, effects);
                }
                AdminResult::IpBans(self.ip_ban_reports())
            }
            AdminRequest::OperatorsAdd(operators) => {
                let directory = self.directory_users(view);
                for request in operators {
                    let Some(user) = resolve_one(&directory, &request.player) else {
                        continue;
                    };
                    self.op(user, request, view, effects);
                }
                AdminResult::Operators(self.operator_reports())
            }
            AdminRequest::OperatorsRemove(players) => {
                let directory = self.directory_users(view);
                for user in resolve_all(&directory, players) {
                    self.deop(&user.id, view, effects);
                }
                AdminResult::Operators(self.operator_reports())
            }
            AdminRequest::OperatorsClear => {
                let ids = self.lists.operators.keys().copied().collect::<Vec<_>>();
                for id in ids {
                    self.deop(&id, view, effects);
                }
                AdminResult::Operators(self.operator_reports())
            }
            AdminRequest::OperatorsSet(operators) => {
                // `OperatorService.set` (OperatorService.java:61-85) compares
                // the *optional* level and bypass fields, so a request that
                // omits them never matches an existing entry and the entry is
                // deopped and re-opped with the server defaults. That quirk is
                // vanilla's, and reproducing it keeps `set` idempotent in the
                // only form vanilla makes idempotent: the fully specified one.
                let directory = self.directory_users(view);
                let mut target = Vec::new();
                for request in operators {
                    if let Some(user) = resolve_one(&directory, &request.player) {
                        target.push((user, request));
                    }
                }
                let removed = self
                    .lists
                    .operators
                    .iter()
                    .filter(|(id, entry)| {
                        !target.iter().any(|(user, request)| {
                            user.id == **id && operator_matches(entry, request)
                        })
                    })
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>();
                for id in removed {
                    self.deop(&id, view, effects);
                }
                for (user, request) in target {
                    let already = self
                        .lists
                        .operators
                        .get(&user.id)
                        .is_some_and(|entry| operator_matches(entry, request));
                    if !already {
                        self.op(user, request, view, effects);
                    }
                }
                AdminResult::Operators(self.operator_reports())
            }
            AdminRequest::SetSetting(setting) => {
                let effective = self
                    .settings
                    .set(setting.clone())
                    .map_err(|rejected| AdminError::invalid_params(rejected.detail))?;
                self.settings_revision += 1;
                match &effective {
                    AdminSetting::Difficulty(difficulty) => {
                        self.saved_difficulty = *difficulty;
                        effects.push(AdminEffect::DifficultyChanged(*difficulty));
                    }
                    AdminSetting::GameMode(game_mode) => {
                        self.saved_game_mode = *game_mode;
                        effects.push(AdminEffect::GameModeChanged(*game_mode));
                    }
                    // `MinecraftServerSettingsServiceImpl` kicks unlisted
                    // players after either allowlist toggle (:52, :65).
                    AdminSetting::EnforceAllowlist(_) | AdminSetting::UseAllowlist(_) => {
                        self.kick_unlisted_players(view, effects);
                    }
                    // Both distance setters answer their getter and broadcast
                    // whatever was written, with no equality check anywhere on
                    // the path: `MinecraftServerSettingsServiceImpl`
                    // (`:171-175`, `:183-187`) calls the server setter, which
                    // writes the property and calls the `PlayerList` one, which
                    // broadcasts. So a write of the value the server already had
                    // still costs the packet.
                    AdminSetting::ViewDistance(distance) => {
                        effects.push(AdminEffect::ViewDistanceChanged(*distance));
                    }
                    AdminSetting::SimulationDistance(distance) => {
                        effects.push(AdminEffect::SimulationDistanceChanged(*distance));
                    }
                    _ => {}
                }
                AdminResult::Setting(effective)
            }
            AdminRequest::UpdateGameRule { key, value } => {
                let entry = self.game_rule(key)?;
                if !entry.accepts(*value) {
                    return Err(AdminError::invalid_params(format!(
                        "{} does not accept that value",
                        entry.resource_id
                    )));
                }
                self.set_game_rule(entry.resource_id, *value);
                AdminResult::GameRule(GameRuleReport {
                    key: entry.resource_id,
                    value: *value,
                })
            }
            AdminRequest::ExecuteCommand { .. } => {
                AdminResult::Command(Self::execute_command(view)?)
            }
            AdminRequest::WorldInfo
            | AdminRequest::WorldBlocks { .. }
            | AdminRequest::WorldSurface { .. } => {
                unreachable!("the world answers its admitted read request");
            }
        })
    }

    fn game_rule(&self, key: &str) -> Result<&'static GameRuleCatalogEntry, AdminError> {
        // `ResourceLocation.parse` defaults the namespace, which is how every
        // vanilla registry lookup accepts a bare id.
        let qualified = if key.contains(':') {
            key.to_string()
        } else {
            format!("minecraft:{key}")
        };
        let entry = vanilla_game_rule(&qualified)
            .filter(|entry| self.game_rules.contains_key(entry.resource_id))
            .ok_or_else(|| AdminError::invalid_params(format!("no game rule {key}")))?;
        Ok(entry)
    }

    /// `command.execute`: the one dispatcher, which the world already ran at
    /// owner level against itself once the request was admitted.
    fn execute_command(view: AdminWorldView<'_>) -> Result<CommandReport, AdminError> {
        let run = view
            .command
            .expect("the world runs a command.execute request before it applies");
        Ok(command_report(run.clone()))
    }

    /// The source `command.execute` runs as. The management principal is not a
    /// player and has no identity in the world; `Commands.LEVEL_OWNERS` is what
    /// the ledger declares this surface runs at.
    pub(crate) fn command_source(envelope: &AdminEnvelope) -> CommandSource {
        CommandSource::server(&envelope.principal, CommandPermission::Owner)
    }

    /// Every identity resolution may consult: the players on the server plus
    /// every name already recorded in a stored user list.
    fn directory_users(&self, view: AdminWorldView<'_>) -> Vec<NameAndId> {
        self.known_profiles(view.online)
    }

    /// `MinecraftServer.services().nameToIdCache()` as this server can answer
    /// it: every identity it has actually recorded.
    ///
    /// This is the same directory the management surface resolves a
    /// `PlayerDto` against, so a name means one identity whichever surface
    /// asked.
    pub fn known_profiles(&self, online: &[OnlinePlayer]) -> Vec<NameAndId> {
        let mut users = online
            .iter()
            .map(|player| player.user.clone())
            .collect::<Vec<_>>();
        users.extend(self.lists.allowlist.values().cloned());
        users.extend(
            self.lists
                .operators
                .values()
                .map(|entry| entry.user.clone()),
        );
        users.extend(
            self.lists
                .player_bans
                .values()
                .map(|entry| entry.user.clone()),
        );
        users
    }

    fn allowlist_add(&mut self, user: NameAndId) {
        if self.lists.allowlist.get(&user.id) == Some(&user) {
            return;
        }
        self.lists.allowlist.insert(user.id, user);
        self.list_revision += 1;
    }

    fn allowlist_remove(&mut self, id: &Uuid) {
        if self.lists.allowlist.remove(id).is_some() {
            self.list_revision += 1;
        }
    }

    fn ban_add(&mut self, user: NameAndId, request: &UserBanRequest, now: i64) {
        self.player_ban_add(
            user,
            BanRecord {
                created: now,
                source: request
                    .source
                    .as_deref()
                    .unwrap_or(MANAGEMENT_SOURCE)
                    .into(),
                expires: request.expires,
                reason: request.reason.as_ref().map(bbb_protocol::JavaString::from),
            },
        );
    }

    fn player_ban_add(&mut self, user: NameAndId, ban: BanRecord) {
        self.lists
            .player_bans
            .insert(user.id, PlayerBanEntry { user, ban });
        self.list_revision += 1;
    }

    fn ip_ban_add(&mut self, ip: String, record: BanRecord) {
        self.lists.ip_bans.insert(ip, record);
        self.list_revision += 1;
    }

    /// `IpBanlistService.banIp` (IpBanlistService.java:40-53): a literal
    /// address wins, and a player reference falls back to that player's current
    /// address only while they are online.
    fn incoming_ip_ban(
        &self,
        request: &IncomingIpBanRequest,
        view: AdminWorldView<'_>,
        now: i64,
    ) -> Option<(String, BanRecord)> {
        let record = |ip: String| {
            (
                ip,
                BanRecord {
                    created: now,
                    source: request
                        .source
                        .as_deref()
                        .unwrap_or(MANAGEMENT_SOURCE)
                        .into(),
                    expires: request.expires,
                    reason: request.reason.as_ref().map(bbb_protocol::JavaString::from),
                },
            )
        };
        if let Some(ip) = request.ip.as_ref().filter(|ip| valid_ip_literal(ip)) {
            return Some(record(ip.clone()));
        }
        let reference = request.player.as_ref()?;
        let player = view
            .online
            .iter()
            .find(|player| match_online(player, reference))?;
        player.address.clone().map(record)
    }

    /// `PlayerList.op` (PlayerList.java:537-548).
    fn op(
        &mut self,
        user: NameAndId,
        request: &OperatorRequest,
        view: AdminWorldView<'_>,
        effects: &mut Vec<AdminEffect>,
    ) {
        let existing = self.lists.operators.get(&user.id);
        let level = request
            .permission_level
            .unwrap_or(self.settings.operator_user_permission_level);
        let bypasses_player_limit = request
            .bypasses_player_limit
            .unwrap_or_else(|| existing.is_some_and(|entry| entry.bypasses_player_limit));
        let entry = OperatorEntry {
            user: user.clone(),
            level,
            bypasses_player_limit,
        };
        if existing == Some(&entry) {
            return;
        }
        self.lists.operators.insert(user.id, entry);
        self.list_revision += 1;
        if view.online.iter().any(|player| player.id() == user.id) {
            effects.push(AdminEffect::PermissionLevelChanged {
                player: user.id,
                level,
            });
        }
    }

    /// `PlayerList.deop` (PlayerList.java:550-557).
    fn deop(&mut self, id: &Uuid, view: AdminWorldView<'_>, effects: &mut Vec<AdminEffect>) {
        if self.lists.operators.remove(id).is_none() {
            return;
        }
        self.list_revision += 1;
        if view.online.iter().any(|player| player.id() == *id) {
            effects.push(AdminEffect::PermissionLevelChanged {
                player: *id,
                level: CommandPermission::All,
            });
        }
    }

    /// `MinecraftServer.kickUnlistedPlayers` (MinecraftServer.java:1656-1667).
    ///
    /// The check is the raw allowlist membership, not `PlayerList.isWhiteListed`
    /// — an operator who is not on the allowlist is kicked here even though the
    /// same operator would have been allowed to log in.
    fn kick_unlisted_players(&self, view: AdminWorldView<'_>, effects: &mut Vec<AdminEffect>) {
        if !(self.settings.enforce_allowlist && self.settings.use_allowlist) {
            return;
        }
        for player in view.online {
            if !self.lists.allowlist.contains_key(&player.id()) {
                effects.push(AdminEffect::Disconnect {
                    player: player.id(),
                    reason: commands::disconnect_reason(NOT_ALLOWLISTED_KEY),
                });
            }
        }
    }

    fn allowlist_refs(&self) -> Vec<PlayerRef> {
        self.lists.allowlist.values().map(PlayerRef::from).collect()
    }

    fn ban_reports(&self) -> Vec<UserBanReport> {
        self.lists
            .player_bans
            .values()
            .map(|entry| UserBanReport {
                player: PlayerRef::from(&entry.user),
                ban: entry.ban.clone(),
            })
            .collect()
    }

    fn ip_ban_reports(&self) -> Vec<IpBanReport> {
        self.lists
            .ip_bans
            .iter()
            .map(|(ip, ban)| IpBanReport {
                ip: ip.clone(),
                ban: ban.clone(),
            })
            .collect()
    }

    fn operator_reports(&self) -> Vec<OperatorReport> {
        self.lists
            .operators
            .values()
            .map(|entry| OperatorReport {
                player: PlayerRef::from(&entry.user),
                permission_level: entry.level,
                bypasses_player_limit: entry.bypasses_player_limit,
            })
            .collect()
    }

    fn game_rule_reports(&self) -> Vec<GameRuleReport> {
        self.game_rules
            .iter()
            .map(|(key, value)| GameRuleReport { key, value: *value })
            .collect()
    }
}

/// The answer one run reports: every message the source would have seen, in
/// the order `sendSuccess` and `sendFailure` produced them.
fn command_report(run: CommandRun) -> CommandReport {
    CommandReport {
        // `CommandRun::result` is absent when the command ended in a
        // `CommandSyntaxException` instead of returning, which is the same
        // condition that put a `sendFailure` message in the list, and for
        // `/function`, which returns nothing at all -- see the field's own
        // note.
        result: run.result,
        messages: run
            .feedback
            .into_iter()
            .map(|feedback| feedback.message)
            .collect(),
        // Every command this server publishes broadcasts at most one line —
        // `SayCommand` and `EmoteCommands` are the only two that broadcast at
        // all — so the report names it rather than the whole list.
        broadcast: run.broadcast.into_iter().find_map(|message| match message {
            crate::commands::CommandBroadcast::System(message) => Some(message),
            crate::commands::CommandBroadcast::Chat(message) => Some(message.decorated()),
            crate::commands::CommandBroadcast::Whisper { .. } => None,
        }),
    }
}

fn resolve_one(known: &[NameAndId], reference: &PlayerRef) -> Option<NameAndId> {
    IdentityDirectory { known }.resolve(reference)
}

fn resolve_all(known: &[NameAndId], players: &[PlayerRef]) -> Vec<NameAndId> {
    let directory = IdentityDirectory { known };
    players
        .iter()
        .filter_map(|reference| directory.resolve(reference))
        .collect()
}

/// `PlayerService.getServerPlayer` (PlayerService.java:36-41) and
/// `ServerStateService.systemMessage` (:43-53): the id wins, the name answers
/// only when there is no id, and a reference with neither names nobody.
fn match_online(player: &OnlinePlayer, reference: &PlayerRef) -> bool {
    match (reference.id, reference.name.as_ref()) {
        (Some(id), _) => player.user.id == id,
        // `PlayerList.getPlayerByName` (PlayerList.java:584-595) compares
        // case-insensitively.
        (None, Some(name)) => player
            .user
            .name
            .with_utf8(|player| {
                name.with_utf8(|name| player.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
            })
            .unwrap_or(false),
        (None, None) => false,
    }
}

fn online_refs(online: &[OnlinePlayer]) -> Vec<PlayerRef> {
    online
        .iter()
        .map(|player| PlayerRef::from(&player.user))
        .collect()
}

fn system_message(
    request: &SystemMessageRequest,
    view: AdminWorldView<'_>,
    effects: &mut Vec<AdminEffect>,
) -> bool {
    // `ServerStateService.systemMessage` (ServerStateService.java:29-61): a
    // message that resolves to no component, and an empty explicit receiver
    // list, both answer false without sending anything.
    let Some(component) = request.message.as_component() else {
        return false;
    };
    let Some(receivers) = &request.receiving_players else {
        effects.push(AdminEffect::BroadcastSystemMessage {
            message: component,
            overlay: request.overlay,
        });
        return true;
    };
    if receivers.is_empty() {
        return false;
    }
    for reference in receivers {
        if let Some(player) = view
            .online
            .iter()
            .find(|player| match_online(player, reference))
        {
            effects.push(AdminEffect::SystemMessage {
                player: player.id(),
                message: component.clone(),
                overlay: request.overlay,
            });
        }
    }
    true
}

fn kick_players(
    entries: &[KickRequest],
    view: AdminWorldView<'_>,
    effects: &mut Vec<AdminEffect>,
) -> Vec<PlayerRef> {
    let mut kicked = Vec::new();
    for entry in entries {
        let Some(player) = view
            .online
            .iter()
            .find(|player| match_online(player, &entry.player))
        else {
            continue;
        };
        let reason = entry
            .message
            .as_ref()
            .and_then(AdminMessage::as_component)
            .unwrap_or_else(|| commands::disconnect_reason(DEFAULT_KICK_KEY));
        effects.push(AdminEffect::Disconnect {
            player: player.id(),
            reason,
        });
        kicked.push(entry.player.clone());
    }
    kicked
}

fn disconnect_address(
    ip: &str,
    view: AdminWorldView<'_>,
    key: &str,
    effects: &mut Vec<AdminEffect>,
) {
    // `PlayerList.getPlayersWithAddress` (PlayerList.java) matches the plain
    // address string the connection reported.
    for player in view.online {
        if player.address.as_deref() == Some(ip) {
            effects.push(AdminEffect::Disconnect {
                player: player.id(),
                reason: commands::disconnect_reason(key),
            });
        }
    }
}

/// `BanlistService.UserBan` equality (BanlistService.java:100-108): the record
/// compares the user, reason, source and expiry, and never the creation
/// instant.
fn ban_matches(record: &BanRecord, request: &UserBanRequest) -> bool {
    record.reason == request.reason.as_ref().map(bbb_protocol::JavaString::from)
        && record.source
            == request
                .source
                .clone()
                .unwrap_or_else(|| MANAGEMENT_SOURCE.to_string())
        && record.expires == request.expires
}

fn ip_ban_record(request: &IpBanRequest, now: i64) -> BanRecord {
    BanRecord {
        created: now,
        source: request
            .source
            .as_deref()
            .unwrap_or(MANAGEMENT_SOURCE)
            .into(),
        expires: request.expires,
        reason: request.reason.as_ref().map(bbb_protocol::JavaString::from),
    }
}

fn ip_ban_matches(record: &BanRecord, target: &BanRecord) -> bool {
    record.reason == target.reason
        && record.source == target.source
        && record.expires == target.expires
}

/// `OperatorService.Op` equality (OperatorService.java:87-88): the optional
/// fields are compared as written, so an omitted level never equals a stored
/// one.
fn operator_matches(entry: &OperatorEntry, request: &OperatorRequest) -> bool {
    request.permission_level == Some(entry.level)
        && request.bypasses_player_limit == Some(entry.bypasses_player_limit)
}

/// `PlayerList.canPlayerLogin` (PlayerList.java:336-342, :349-355): the reason
/// component, with the expiry appended when the ban has one.
fn ban_refusal(kind: BanKind, record: &BanRecord) -> RichTextComponent {
    let expires = record.expires.map(format_ban_expiry_date);
    commands::ban_refusal(kind, record.reason.as_ref(), expires.as_deref())
}

impl std::fmt::Debug for AdminState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdminState")
            .field("lists", &self.lists)
            .field("settings", &self.settings)
            .field("game_rules", &self.game_rules.len())
            .field("list_revision", &self.list_revision)
            .field("processed_operations", &self.processed_operations)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CommandTestWorld, ServerWorldConfig};

    use crate::admin::settings::MIN_DISTANCE_CHUNKS;
    use crate::admin::AdminScope;

    const VANILLA_FEATURE: &str = "minecraft:vanilla";

    fn settings() -> AdminSettings {
        AdminSettings {
            difficulty: Difficulty::Easy,
            game_mode: GameType::Survival,
            enforce_allowlist: false,
            use_allowlist: false,
            max_players: 20,
            motd: "A BBB Server".to_string(),
            spawn_protection_radius: 16,
            operator_user_permission_level: CommandPermission::Owner,
            view_distance: MIN_DISTANCE_CHUNKS,
            simulation_distance: MIN_DISTANCE_CHUNKS,
            hide_online_players: false,
            status_replies: true,
            accepts_transfers: false,
            allow_flight: false,
            pause_when_empty_seconds: 60,
            entity_broadcast_range_percentage: 100,
        }
    }

    fn state() -> AdminState {
        AdminState::new(settings(), StoredUserLists::default(), &[VANILLA_FEATURE])
    }

    fn online(name: &str, address: &str) -> OnlinePlayer {
        OnlinePlayer {
            user: NameAndId::offline(name),
            address: Some(address.to_string()),
        }
    }

    fn view<'a>(online: &'a [OnlinePlayer], command: Option<&'a CommandRun>) -> AdminWorldView<'a> {
        AdminWorldView {
            tick: 100,
            stopping: false,
            online,
            loaded_chunks: 1,
            changed_blocks: 0,
            command,
        }
    }

    fn envelope(scope: AdminScope, request_id: &str) -> AdminEnvelope {
        AdminEnvelope {
            operation_id: 1,
            request_id: request_id.to_string(),
            principal: "agent".to_string(),
            scope,
            deadline_tick: 200,
            now: 1_755_000_000_000,
        }
    }

    fn apply(
        state: &mut AdminState,
        envelope: &AdminEnvelope,
        request: AdminRequest,
        online: &[OnlinePlayer],
    ) -> (AdminOutcome, Vec<AdminEffect>) {
        let mut effects = Vec::new();
        // The world runs an admitted `command.execute` against itself first.
        let command = match &request {
            AdminRequest::ExecuteCommand { command }
                if state.admit(envelope, &request, 100).is_ok() =>
            {
                let users = online
                    .iter()
                    .map(|player| player.user.clone())
                    .collect::<Vec<_>>();
                Some(
                    CommandTestWorld::new(ServerWorldConfig::default(), &users)
                        .run(command, &AdminState::command_source(envelope)),
                )
            }
            _ => None,
        };
        let outcome = state.apply(
            envelope,
            &request,
            view(online, command.as_ref()),
            &mut effects,
        );
        (outcome, effects)
    }

    #[test]
    fn a_request_beyond_the_granted_scope_is_refused_without_applying() {
        let mut state = state();
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Read, "r1"),
            AdminRequest::AllowlistClear,
            &[],
        );
        assert_eq!(
            outcome.result,
            Err(AdminError::ScopeDenied {
                required: AdminScope::Manage,
                granted: AdminScope::Read,
            })
        );
        assert!(effects.is_empty());
        assert_eq!(state.list_revision(), 0);

        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "r2"),
            AdminRequest::ExecuteCommand {
                command: "list".into(),
            },
            &[],
        );
        assert_eq!(
            outcome.result,
            Err(AdminError::ScopeDenied {
                required: AdminScope::Command,
                granted: AdminScope::Manage,
            })
        );
    }

    #[test]
    fn a_request_that_missed_its_deadline_is_refused() {
        let mut state = state();
        let mut expired = envelope(AdminScope::Manage, "r1");
        expired.deadline_tick = 99;
        let (outcome, effects) = apply(&mut state, &expired, AdminRequest::Stop, &[]);
        assert_eq!(
            outcome.result,
            Err(AdminError::DeadlineExceeded {
                deadline_tick: 99,
                tick: 100,
            })
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn a_repeated_request_id_is_answered_from_the_record_without_reapplying() {
        let mut state = state();
        let recorded = envelope(AdminScope::Manage, "add-once");
        let players = vec![PlayerRef::by_name("bbb-client")];
        let (first, _) = apply(
            &mut state,
            &recorded,
            AdminRequest::AllowlistAdd(players.clone()),
            &[],
        );
        assert!(!first.replayed);
        let revision = state.list_revision();

        // A different request under the same id answers the recorded outcome.
        let (second, effects) = apply(&mut state, &recorded, AdminRequest::AllowlistClear, &[]);
        assert!(second.replayed);
        assert_eq!(second.result, first.result);
        assert!(effects.is_empty());
        assert_eq!(state.list_revision(), revision);
        assert_eq!(state.lists().allowlist.len(), 1);

        // A lower-scope token sharing a principal cannot replay a higher-scope record.
        let (denied_same_action, _) = apply(
            &mut state,
            &envelope(AdminScope::Read, "add-once"),
            AdminRequest::AllowlistClear,
            &[],
        );
        assert_eq!(
            denied_same_action.result,
            Err(AdminError::ScopeDenied {
                required: AdminScope::Manage,
                granted: AdminScope::Read,
            })
        );
        assert!(!denied_same_action.replayed);

        let cmd_envelope = envelope(AdminScope::Command, "cmd-once");
        let (cmd_first, _) = apply(
            &mut state,
            &cmd_envelope,
            AdminRequest::ExecuteCommand {
                command: "list".into(),
            },
            &[],
        );
        assert!(!cmd_first.replayed);
        assert!(matches!(cmd_first.result, Ok(AdminResult::Command(_))));
        let (denied_cross_action, _) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "cmd-once"),
            AdminRequest::AllowlistClear,
            &[],
        );
        assert_eq!(
            denied_cross_action.result,
            Err(AdminError::ScopeDenied {
                required: AdminScope::Command,
                granted: AdminScope::Manage,
            })
        );
        assert!(!denied_cross_action.replayed);

        // Reads are never recorded, so they always answer current state.
        let (read, _) = apply(
            &mut state,
            &envelope(AdminScope::Read, "add-once"),
            AdminRequest::Allowlist,
            &[],
        );
        assert!(!read.replayed);
    }

    #[test]
    fn the_allowlist_kicks_unlisted_players_only_when_both_toggles_are_on() {
        let mut state = state();
        let players = [online("bbb-client", "203.0.113.7")];
        // Removing while the allowlist is off kicks nobody.
        let (_, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "r1"),
            AdminRequest::AllowlistRemove(vec![PlayerRef::by_name("bbb-client")]),
            &players,
        );
        assert!(effects.is_empty());

        for (index, setting) in [
            AdminSetting::UseAllowlist(true),
            AdminSetting::EnforceAllowlist(true),
        ]
        .into_iter()
        .enumerate()
        {
            let (outcome, effects) = apply(
                &mut state,
                &envelope(AdminScope::Manage, &format!("set{index}")),
                AdminRequest::SetSetting(setting.clone()),
                &players,
            );
            assert_eq!(outcome.result, Ok(AdminResult::Setting(setting)));
            if index == 1 {
                assert_eq!(
                    effects,
                    vec![AdminEffect::Disconnect {
                        player: NameAndId::offline("bbb-client").id,
                        reason: commands::disconnect_reason(NOT_ALLOWLISTED_KEY),
                    }]
                );
            } else {
                assert!(effects.is_empty(), "one toggle alone enforces nothing");
            }
        }

        // An operator who is not on the allowlist is still kicked, which is
        // what `MinecraftServer.kickUnlistedPlayers` checks.
        let (_, _) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "op"),
            AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::by_name("bbb-client"),
                permission_level: None,
                bypasses_player_limit: None,
            }]),
            &players,
        );
        let (_, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "clear"),
            AdminRequest::AllowlistClear,
            &players,
        );
        assert!(effects.is_empty(), "clear does not enforce on its own");
        let (_, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "set-empty"),
            AdminRequest::AllowlistSet(Vec::new()),
            &players,
        );
        assert_eq!(effects.len(), 1);
    }

    #[test]
    fn banning_an_online_player_disconnects_them_and_refuses_the_next_login() {
        let mut state = state();
        let players = [online("bbb-client", "203.0.113.7")];
        let user = NameAndId::offline("bbb-client");
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "ban"),
            AdminRequest::BansAdd(vec![UserBanRequest {
                player: PlayerRef::by_name("bbb-client"),
                reason: Some("griefing".to_string()),
                source: None,
                expires: None,
            }]),
            &players,
        );
        let Ok(AdminResult::Bans(bans)) = outcome.result else {
            panic!("expected a ban list answer");
        };
        assert_eq!(bans.len(), 1);
        assert_eq!(bans[0].ban.source, MANAGEMENT_SOURCE);
        assert_eq!(bans[0].ban.created, 1_755_000_000_000);
        assert_eq!(
            effects,
            vec![AdminEffect::Disconnect {
                player: user.id,
                reason: commands::disconnect_reason(BANNED_KEY),
            }]
        );
        assert!(state
            .login_refusal(&user, Some("203.0.113.7"), 1_755_000_000_000)
            .is_some());

        // An expired ban stops refusing and is pruned on the next request.
        let (_, _) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "ban-expiring"),
            AdminRequest::BansSet(vec![UserBanRequest {
                player: PlayerRef::by_name("bbb-client"),
                reason: None,
                source: None,
                expires: Some(1_755_000_001_000),
            }]),
            &[],
        );
        assert!(state
            .login_refusal(&user, None, 1_755_000_002_000)
            .is_none());
        let mut later = envelope(AdminScope::Read, "list-bans");
        later.now = 1_755_000_002_000;
        let (outcome, _) = apply(&mut state, &later, AdminRequest::Bans, &[]);
        assert_eq!(outcome.result, Ok(AdminResult::Bans(Vec::new())));
        assert!(state.lists().player_bans.is_empty());
    }

    #[test]
    fn an_ip_ban_may_name_a_player_and_disconnects_that_address() {
        let mut state = state();
        let players = [online("bbb-client", "203.0.113.7")];
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "ipban"),
            AdminRequest::IpBansAdd(vec![IncomingIpBanRequest {
                player: Some(PlayerRef::by_name("bbb-client")),
                ip: None,
                reason: None,
                source: None,
                expires: None,
            }]),
            &players,
        );
        let Ok(AdminResult::IpBans(bans)) = outcome.result else {
            panic!("expected an ip ban list answer");
        };
        assert_eq!(bans.len(), 1);
        assert_eq!(bans[0].ip, "203.0.113.7");
        assert_eq!(
            effects,
            vec![AdminEffect::Disconnect {
                player: NameAndId::offline("bbb-client").id,
                reason: commands::disconnect_reason(IP_BANNED_KEY),
            }]
        );

        // A host name is never stored as an address ban.
        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "hostname"),
            AdminRequest::IpBansSet(vec![IpBanRequest {
                ip: "example.invalid".to_string(),
                reason: None,
                source: None,
                expires: None,
            }]),
            &players,
        );
        assert_eq!(outcome.result, Ok(AdminResult::IpBans(Vec::new())));
    }

    #[test]
    fn opping_uses_the_configured_default_level_and_announces_it() {
        let mut state = state();
        let players = [online("bbb-client", "203.0.113.7")];
        let user = NameAndId::offline("bbb-client");
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "op"),
            AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::by_name("bbb-client"),
                permission_level: None,
                bypasses_player_limit: None,
            }]),
            &players,
        );
        assert_eq!(
            outcome.result,
            Ok(AdminResult::Operators(vec![OperatorReport {
                player: PlayerRef::from(&user),
                permission_level: CommandPermission::Owner,
                bypasses_player_limit: false,
            }]))
        );
        assert_eq!(
            effects,
            vec![AdminEffect::PermissionLevelChanged {
                player: user.id,
                level: CommandPermission::Owner,
            }]
        );
        assert_eq!(state.permission_level(user.id), CommandPermission::Owner);

        let (_, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "deop"),
            AdminRequest::OperatorsRemove(vec![PlayerRef::by_name("bbb-client")]),
            &players,
        );
        assert_eq!(
            effects,
            vec![AdminEffect::PermissionLevelChanged {
                player: user.id,
                level: CommandPermission::All,
            }]
        );
        assert_eq!(state.permission_level(user.id), CommandPermission::All);
    }

    #[test]
    fn operator_membership_is_independent_of_permission_level() {
        let mut state = state();
        let user = NameAndId::offline("bbb-client");
        let absent = NameAndId::offline("absent");
        let players = [];
        apply(
            &mut state,
            &envelope(AdminScope::Manage, "op-all"),
            AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::from(&user),
                permission_level: Some(CommandPermission::All),
                bypasses_player_limit: None,
            }]),
            &players,
        );

        assert!(state.is_operator(user.id));
        assert!(!state.is_operator(absent.id));
    }

    #[test]
    fn a_fully_specified_operator_set_is_idempotent() {
        let mut state = state();
        let request = AdminRequest::OperatorsSet(vec![OperatorRequest {
            player: PlayerRef::by_name("bbb-client"),
            permission_level: Some(CommandPermission::GameMaster),
            bypasses_player_limit: Some(true),
        }]);
        let players = [online("bbb-client", "203.0.113.7")];
        let (_, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "set1"),
            request.clone(),
            &players,
        );
        assert_eq!(effects.len(), 1);
        let revision = state.list_revision();
        let (_, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "set2"),
            request,
            &players,
        );
        assert!(effects.is_empty());
        assert_eq!(state.list_revision(), revision);
    }

    #[test]
    fn game_rules_start_at_their_vanilla_defaults_and_reject_the_wrong_domain() {
        let mut state = state();
        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Read, "rules"),
            AdminRequest::GameRules,
            &[],
        );
        let Ok(AdminResult::GameRules(rules)) = outcome.result else {
            panic!("expected a game rule readout");
        };
        assert!(rules
            .iter()
            .any(|rule| rule.key == "minecraft:keep_inventory"
                && rule.value == GameRuleValue::Boolean(false)));
        // `max_minecart_speed` needs a feature this server does not enable.
        assert!(!rules
            .iter()
            .any(|rule| rule.key == "minecraft:max_minecart_speed"));

        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "update"),
            AdminRequest::UpdateGameRule {
                key: "keep_inventory".to_string(),
                value: GameRuleValue::Boolean(true),
            },
            &[],
        );
        assert_eq!(
            outcome.result,
            Ok(AdminResult::GameRule(GameRuleReport {
                key: "minecraft:keep_inventory",
                value: GameRuleValue::Boolean(true),
            }))
        );

        for (key, value) in [
            ("keep_inventory", GameRuleValue::Integer(1)),
            ("max_snow_accumulation_height", GameRuleValue::Integer(9)),
            ("max_minecart_speed", GameRuleValue::Integer(8)),
            ("no_such_rule", GameRuleValue::Boolean(true)),
        ] {
            let (outcome, _) = apply(
                &mut state,
                &envelope(AdminScope::Manage, key),
                AdminRequest::UpdateGameRule {
                    key: key.to_string(),
                    value,
                },
                &[],
            );
            assert!(
                matches!(outcome.result, Err(AdminError::InvalidParams(_))),
                "{key} must be refused"
            );
        }
    }

    #[test]
    fn command_execute_delegates_the_one_dispatcher_at_owner_level() {
        let mut state = state();
        let players = [online("bbb-client", "203.0.113.7")];
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Command, "list"),
            AdminRequest::ExecuteCommand {
                command: "list".into(),
            },
            &players,
        );
        let Ok(AdminResult::Command(report)) = outcome.result else {
            panic!("expected a command answer");
        };
        assert_eq!(report.result, Some(1));
        assert_eq!(report.messages.len(), 1);
        assert!(report.broadcast.is_none());
        assert!(effects.is_empty());

        // `/say` is gamemaster-gated, so it proves the owner permission and
        // produces the broadcast the world has to deliver.
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Command, "say"),
            AdminRequest::ExecuteCommand {
                command: "say hello".into(),
            },
            &players,
        );
        let Ok(AdminResult::Command(report)) = outcome.result else {
            panic!("expected a command answer");
        };
        assert!(report.broadcast.is_some(), "say broadcasts");
        assert!(effects.is_empty());

        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Command, "unknown"),
            AdminRequest::ExecuteCommand {
                command: "nope".into(),
            },
            &players,
        );
        let Ok(AdminResult::Command(report)) = outcome.result else {
            panic!("expected a command answer");
        };
        assert_eq!(report.result, None);
        assert!(!report.messages.is_empty());
    }

    #[test]
    fn a_system_message_reaches_the_named_players_or_everyone() {
        let mut state = state();
        let players = [
            online("bbb-client", "203.0.113.7"),
            online("bbb-peer", "203.0.113.8"),
        ];
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "broadcast"),
            AdminRequest::SystemMessage(SystemMessageRequest {
                message: AdminMessage::literal("hello"),
                overlay: true,
                receiving_players: None,
            }),
            &players,
        );
        assert_eq!(outcome.result, Ok(AdminResult::Sent(true)));
        assert!(matches!(
            effects.as_slice(),
            [AdminEffect::BroadcastSystemMessage { overlay: true, .. }]
        ));

        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "named"),
            AdminRequest::SystemMessage(SystemMessageRequest {
                message: AdminMessage::literal("hello"),
                overlay: false,
                // A case-insensitive name and an unknown player.
                receiving_players: Some(vec![
                    PlayerRef::by_name("BBB-PEER"),
                    PlayerRef::by_name("absent"),
                ]),
            }),
            &players,
        );
        assert_eq!(outcome.result, Ok(AdminResult::Sent(true)));
        assert_eq!(effects.len(), 1);

        for (label, request) in [
            (
                "no component",
                SystemMessageRequest {
                    message: AdminMessage::default(),
                    overlay: false,
                    receiving_players: None,
                },
            ),
            (
                "empty receiver list",
                SystemMessageRequest {
                    message: AdminMessage::literal("hello"),
                    overlay: false,
                    receiving_players: Some(Vec::new()),
                },
            ),
        ] {
            let (outcome, effects) = apply(
                &mut state,
                &envelope(AdminScope::Manage, label),
                AdminRequest::SystemMessage(request),
                &players,
            );
            assert_eq!(outcome.result, Ok(AdminResult::Sent(false)), "{label}");
            assert!(effects.is_empty(), "{label}");
        }
    }

    #[test]
    fn kicking_answers_with_the_references_it_actually_kicked() {
        let mut state = state();
        let players = [online("bbb-client", "203.0.113.7")];
        let (outcome, effects) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "kick"),
            AdminRequest::KickPlayers(vec![
                KickRequest {
                    player: PlayerRef::by_name("bbb-client"),
                    message: Some(AdminMessage::literal("bye")),
                },
                KickRequest {
                    player: PlayerRef::by_name("absent"),
                    message: None,
                },
            ]),
            &players,
        );
        assert_eq!(
            outcome.result,
            Ok(AdminResult::Kicked(vec![PlayerRef::by_name("bbb-client")]))
        );
        assert_eq!(effects.len(), 1);
    }

    #[test]
    fn status_and_diagnostics_report_the_projected_world() {
        let mut state = state();
        let players = [online("bbb-client", "203.0.113.7")];
        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Read, "status"),
            AdminRequest::Status,
            &players,
        );
        assert_eq!(
            outcome.result,
            Ok(AdminResult::Status(ServerStateReport {
                started: true,
                players: vec![PlayerRef::from(&NameAndId::offline("bbb-client"))],
                version_name: MC_VERSION,
                version_protocol: PROTOCOL_VERSION,
            }))
        );

        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Read, "diagnostics"),
            AdminRequest::Diagnostics,
            &players,
        );
        let Ok(AdminResult::Diagnostics(report)) = outcome.result else {
            panic!("expected a diagnostics answer");
        };
        assert_eq!(report.tick, 100);
        assert_eq!(report.online_players, 1);
        assert_eq!(report.max_players, 20);
        assert_eq!(report.processed_admin_operations, 2);
        assert!(!report.stopping);
    }

    #[test]
    fn every_applied_request_carries_its_audit_identity() {
        let mut state = state();
        let (outcome, _) = apply(
            &mut state,
            &envelope(AdminScope::Manage, "audit"),
            AdminRequest::AllowlistClear,
            &[],
        );
        assert_eq!(outcome.operation_id, 1);
        assert_eq!(outcome.request_id, "audit");
        assert_eq!(outcome.principal, "agent");
        assert_eq!(outcome.scope, AdminScope::Manage);
        assert_eq!(outcome.action, "allowlist.clear");
        assert_eq!(outcome.tick, 100);
        assert!(!outcome.replayed);
    }
}
