use std::{net::SocketAddr, path::PathBuf, time::Duration};

use bbb_protocol::{codec::name_uuid_from_bytes, json_value_to_nbt_summary, RichTextComponent};
use bbb_protocol::{
    difficulties::{vanilla_difficulty_for_serialized_name, Difficulty},
    game_types::{vanilla_game_type_for_serialized_name, GameType},
};
use bbb_server_core::admin::{
    clamp_broadcast_range, permission_level_by_id, AdminScope, PersistedSettings, DEFAULT_MOTD,
    MAX_DISTANCE_CHUNKS, MIN_DISTANCE_CHUNKS,
};
use bbb_server_core::commands::CommandPermission;
use bbb_server_net::{ConnectionLimits, ServerResourcePack};
use bbb_server_persistence::Java26WorldPreset;
use clap::Parser;

/// Command-line-only startup configuration for the headless dedicated server.
#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(name = "bbb-server", version, about)]
pub struct ServerOptions {
    /// Minecraft listener address. Use port 0 only for tests or supervision.
    #[arg(long, default_value = "127.0.0.1:25565")]
    pub bind: SocketAddr,
    /// Server-list description advertised by the status protocol (default
    /// `A BBB Server`). `serversettings/motd/set` saves its change to
    /// `server-settings.json` in `--admin-dir`; at startup an explicit flag
    /// wins over that file, and the file wins over the default. Every option
    /// below that says "saved" resolves the same way.
    #[arg(long)]
    pub motd: Option<String>,
    /// `max-players` (`DedicatedServerProperties.java:104`, default `20`).
    /// Saved by `serversettings/max_players/set`.
    #[arg(long, value_parser = clap::value_parser!(i32).range(1..))]
    pub max_players: Option<i32>,
    /// World difficulty, forced at startup like `server.properties`
    /// `difficulty` (`DedicatedServer.forceDifficulty`, default `easy`).
    /// Saved by `serversettings/difficulty/set`, not by `/difficulty`.
    #[arg(long, value_parser = parse_difficulty)]
    pub difficulty: Option<Difficulty>,
    /// World game type, forced at startup like `server.properties` `gamemode`
    /// (`DedicatedServer.java:237`, default `survival`). Saved by
    /// `serversettings/game_mode/set`, not by `/defaultgamemode`.
    #[arg(long, value_parser = parse_game_type)]
    pub gamemode: Option<GameType>,
    /// `server.properties` `force-gamemode`
    /// (`DedicatedServerProperties.java:63`, default `false`), which is what
    /// `DedicatedServer.forceGameMode` reads (`DedicatedServer.java:796-797`).
    ///
    /// Set, it makes [`ServerOptions::gamemode`] outrank a returning player's
    /// saved `playerGameType` instead of only supplying the default for a
    /// profile the save has never seen: `getForcedGameType`
    /// (`DedicatedServer.java:792-794`) answers the level's game type rather
    /// than null, and that is the first branch of
    /// `ServerPlayer.calculateGameModeForNewPlayer`
    /// (`ServerPlayer.java:2029-2036`).
    #[arg(long)]
    pub force_gamemode: bool,
    /// Spawn protection radius; nonpositive values disable protection
    /// (DedicatedServerProperties.java:89, default `16`). Saved by
    /// `serversettings/spawn_protection_radius/set`.
    #[arg(long, allow_hyphen_values = true)]
    pub spawn_protection: Option<i32>,
    /// `view-distance` in chunks (`DedicatedServerProperties.java:102`,
    /// default `10`; `ChunkMap.java:810` clamps to 2..=32). Saved by
    /// `serversettings/view_distance/set`.
    #[arg(long, value_parser = parse_distance)]
    pub view_distance: Option<i32>,
    /// `simulation-distance` in chunks (`DedicatedServerProperties.java:103`,
    /// default `10`). Saved by `serversettings/simulation_distance/set`.
    #[arg(long, value_parser = parse_distance)]
    pub simulation_distance: Option<i32>,
    /// `hide-online-players` (`DedicatedServerProperties.java:113`, default
    /// `false`): the status answer carries no player sample. Saved by
    /// `serversettings/hide_online_players/set`.
    #[arg(long, action = clap::ArgAction::Set)]
    pub hide_online_players: Option<bool>,
    /// `enable-status` (`DedicatedServerProperties.java:112`, default `true`):
    /// off, a server-list probe gets no answer. Saved by
    /// `serversettings/status_replies/set`.
    #[arg(long, action = clap::ArgAction::Set)]
    pub enable_status: Option<bool>,
    /// Accept incoming transfers on this server (default false).
    #[arg(long, action = clap::ArgAction::Set)]
    pub accepts_transfers: Option<bool>,
    /// `op-permission-level` (`DedicatedServerProperties.java:90-92`, default
    /// `4`): the level an operator is granted when none is named. Saved by
    /// `serversettings/operator_user_permission_level/set`.
    #[arg(long, value_parser = clap::value_parser!(i64).range(0..=4))]
    pub op_permission_level: Option<i64>,
    /// `function-permission-level` (`DedicatedServerProperties.java:93-98`,
    /// default `2`): the permissions every `.mcfunction` line is parsed with.
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(i64).range(0..=4))]
    pub function_permission_level: i64,
    /// Maximum simultaneous TCP connection tasks, including status probes.
    #[arg(long, default_value_t = 128, value_parser = parse_nonzero_usize)]
    pub max_connections: usize,
    /// Maximum simultaneous TCP connections accepted from a single remote IP.
    #[arg(long, default_value_t = 16, value_parser = parse_nonzero_usize)]
    pub max_connections_per_ip: usize,
    /// Vanilla compression threshold in bytes; -1 disables compression.
    #[arg(long, default_value_t = 256, value_parser = clap::value_parser!(i32).range(-1..))]
    pub compression_threshold: i32,
    /// Maximum queued network-to-tick events per connection.
    #[arg(long, default_value_t = 64, value_parser = parse_nonzero_usize)]
    pub connection_event_entries: usize,
    /// Maximum retained network-to-tick bytes per connection.
    #[arg(long, default_value_t = 2 * 1024 * 1024, value_parser = parse_nonzero_usize)]
    pub connection_event_bytes: usize,
    /// Maximum queued tick-to-network commands per connection.
    #[arg(long, default_value_t = 64, value_parser = parse_nonzero_usize)]
    pub connection_command_entries: usize,
    /// Maximum retained tick-to-network bytes per connection.
    #[arg(long, default_value_t = 16 * 1024 * 1024, value_parser = parse_nonzero_usize)]
    pub connection_command_bytes: usize,
    /// Seconds allowed for connection tasks to stop after SIGINT/SIGTERM.
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..))]
    pub shutdown_timeout_seconds: u64,
    /// Official Java 26.1 save directory loaded at startup and flushed on
    /// shutdown; absent keeps the ephemeral flat-fixture world without
    /// persistence.
    #[arg(long)]
    pub world: Option<PathBuf>,
    /// `level-seed` of a save created in an empty `--world`: empty picks a
    /// random seed, a number is used as is, other text is hashed.
    #[arg(long, default_value = "", allow_hyphen_values = true)]
    pub level_seed: String,
    /// `level-type` of a save created in an empty `--world`: `normal`, `flat`,
    /// `large_biomes` or `amplified`, optionally `minecraft:`-prefixed.
    #[arg(long, default_value = "normal", value_parser = parse_level_type)]
    pub level_type: Java26WorldPreset,
    /// `generate-structures` of a save created in an empty `--world`.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub generate_structures: bool,
    /// `hardcore` of a save created in an empty `--world`
    /// (`DedicatedServerProperties.java:87`, default `false`); an existing save
    /// keeps its own.
    #[arg(long)]
    pub hardcore: bool,
    /// `allow-flight` (`DedicatedServerProperties.java:59`, default `false`):
    /// true, a player or vehicle hanging in the air is never kicked for flying
    /// (`ServerGamePacketListenerImpl.java:507`, `:1151`). Saved by
    /// `serversettings/allow_flight/set`.
    #[arg(long, action = clap::ArgAction::Set)]
    pub allow_flight: Option<bool>,
    /// `pause-when-empty-seconds` (`DedicatedServerProperties.java:126`,
    /// default `60`): after this long with nobody online the server saves
    /// once and stops simulating until a player joins, while connections,
    /// status and management keep being served (`MinecraftServer.java:954-972`).
    /// Zero or less never pauses. Saved by
    /// `serversettings/pause_when_empty_seconds/set`.
    #[arg(long, allow_hyphen_values = true)]
    pub pause_when_empty_seconds: Option<i32>,
    /// `entity-broadcast-range-percentage` (`DedicatedServerProperties.java:114-116`,
    /// default `100`, clamped to 10..=1000): scales every entity tracking
    /// range (`DedicatedServer.java:764-766`). Saved by
    /// `serversettings/entity_broadcast_range/set`.
    #[arg(long, allow_hyphen_values = true, value_parser = parse_broadcast_range)]
    pub entity_broadcast_range_percentage: Option<i32>,
    /// `bug-report-link` (`DedicatedServerProperties.java:62`): an `http` or
    /// `https` link sent as the server's bug report link
    /// (`DedicatedServer.java:859-877`).
    #[arg(long, value_parser = parse_untrusted_link)]
    pub bug_report_link: Option<String>,
    /// Directory of `<language>.txt` code of conduct texts every joining
    /// player has to accept, chosen by the client's language with `en_us` as
    /// the fallback. Vanilla's `enable-code-of-conduct` reads `codeofconduct/`
    /// (`DedicatedServer.java:126-167`); absent, no code of conduct is shown.
    #[arg(long)]
    pub code_of_conduct: Option<PathBuf>,
    /// `resource-pack` (`DedicatedServerProperties.java:143`): the URL of the
    /// server resource pack pushed during configuration; absent sends none.
    #[arg(long)]
    pub resource_pack: Option<String>,
    /// `resource-pack-sha1`: 40 hex digits, or empty for none.
    #[arg(long, default_value = "", value_parser = parse_sha1)]
    pub resource_pack_sha1: String,
    /// `resource-pack-id`; absent derives it from the URL like vanilla
    /// (`UUID.nameUUIDFromBytes`, `DedicatedServerProperties.java:212-214`).
    #[arg(long)]
    pub resource_pack_id: Option<uuid::Uuid>,
    /// `require-resource-pack`: a player who declines the pack is disconnected.
    #[arg(long)]
    pub require_resource_pack: bool,
    /// `resource-pack-prompt`: a JSON text component shown with the pack.
    #[arg(long)]
    pub resource_pack_prompt: Option<String>,
    /// Vanilla `--safeMode` equivalent (`Main.java:76`): load the level with
    /// the vanilla datapack only, ignoring the saved DataPacks selection.
    #[arg(long)]
    pub safe_mode: bool,
    /// Directory holding the four stored user lists (`whitelist.json`,
    /// `ops.json`, `banned-players.json`, `banned-ips.json`) and
    /// `server-settings.json`. Absent keeps them for the lifetime of the
    /// process only.
    #[arg(long)]
    pub admin_dir: Option<PathBuf>,
    /// `server.properties` `white-list` (`DedicatedServerProperties.java:123`,
    /// default `false`). `/whitelist on|off` and the management API save their
    /// change to `server-settings.json` in `--admin-dir`; at startup an explicit
    /// flag wins over that file, and the file wins over the default.
    #[arg(long, action = clap::ArgAction::Set)]
    pub white_list: Option<bool>,
    /// `server.properties` `enforce-whitelist` (`DedicatedServerProperties.java:64`,
    /// default `false`): enabling the allowlist, or removing an entry from it,
    /// kicks players it no longer lists. Saved and resolved like `--white-list`.
    #[arg(long, action = clap::ArgAction::Set)]
    pub enforce_whitelist: Option<bool>,
    /// `server.properties` `player-idle-timeout` in minutes
    /// (`DedicatedServerProperties.java:121`, default `0` = never).
    /// `/setidletimeout` saves its change; resolved like `--white-list`.
    #[arg(long, value_parser = clap::value_parser!(i32).range(0..))]
    pub player_idle_timeout: Option<i32>,
    /// Do not serve the MCP management endpoint at all.
    #[arg(long)]
    pub no_mcp: bool,
    /// MCP management endpoint address. A non-loopback address is refused
    /// unless TLS and the OAuth 2.1 options below are all supplied.
    #[arg(long, default_value = "127.0.0.1:25585")]
    pub mcp_bind: SocketAddr,
    /// HTTP path the MCP endpoint is served at.
    #[arg(long, default_value = "/mcp")]
    pub mcp_path: String,
    /// Scope a credential-less loopback caller is granted: `none`, `read`,
    /// `manage` or `command`.
    #[arg(long, default_value = "manage", value_parser = parse_loopback_grant)]
    pub mcp_loopback_scope: LoopbackGrant,
    /// File of bearer grants, one `principal:scope:secret` per line. Secrets
    /// arrive by file because argv is readable by every process on the host.
    #[arg(long)]
    pub mcp_token_file: Option<PathBuf>,
    /// Browser origin the endpoint accepts. Repeatable; empty accepts none.
    #[arg(long)]
    pub mcp_allowed_origin: Vec<String>,
    /// PEM certificate chain the management endpoint terminates TLS with.
    #[arg(long)]
    pub mcp_tls_cert: Option<PathBuf>,
    /// PEM private key for `--mcp-tls-cert`.
    #[arg(long)]
    pub mcp_tls_key: Option<PathBuf>,
    /// Canonical OAuth 2.1 resource identifier for this endpoint, which an
    /// access token has to name in `aud`.
    #[arg(long)]
    pub mcp_oauth_resource: Option<String>,
    /// OAuth 2.1 issuer whose access tokens are accepted. Repeatable, paired by
    /// position with `--mcp-oauth-jwks`.
    #[arg(long)]
    pub mcp_oauth_issuer: Vec<String>,
    /// JWKS document for the issuer at the same position.
    #[arg(long)]
    pub mcp_oauth_jwks: Vec<PathBuf>,
    /// Seconds a management caller waits for the tick before its answer moves
    /// to the operation record.
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..))]
    pub mcp_request_timeout_seconds: u64,
    /// Built-in gameplay mods to enable for this session (e.g. `demo_boss`).
    #[arg(long = "mod")]
    pub mods: Vec<String>,
    /// Data-driven mod pack manifests or directories to register for this session.
    #[arg(long, visible_alias = "mod-pack")]
    pub mod_packs: Vec<PathBuf>,
}

impl ServerOptions {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.max_players.is_some_and(|players| players < 1) {
            return Err("max players must be at least 1".to_string());
        }
        for (name, value) in [
            ("max connections", self.max_connections),
            ("connection event entries", self.connection_event_entries),
            ("connection event bytes", self.connection_event_bytes),
            (
                "connection command entries",
                self.connection_command_entries,
            ),
            ("connection command bytes", self.connection_command_bytes),
        ] {
            if value == 0 {
                return Err(format!("{name} must be non-zero"));
            }
        }
        if self.compression_threshold < -1 {
            return Err("compression threshold must be -1 or non-negative".to_string());
        }
        if self.shutdown_timeout_seconds == 0 {
            return Err("shutdown timeout must be non-zero".to_string());
        }
        if self.mcp_tls_cert.is_some() != self.mcp_tls_key.is_some() {
            return Err("TLS needs both a certificate chain and a private key".to_string());
        }
        if self.mcp_oauth_issuer.len() != self.mcp_oauth_jwks.len() {
            return Err("every OAuth issuer needs one JWKS document".to_string());
        }
        if self.mcp_oauth_resource.is_some() != !self.mcp_oauth_issuer.is_empty() {
            return Err(
                "an OAuth protected resource needs both a resource identifier and an issuer"
                    .to_string(),
            );
        }
        if !self.no_mcp
            && !self.mcp_bind.ip().is_loopback()
            && !self.mcp_remote_authority_declared()
        {
            return Err(format!(
                "{} is not a loopback address; a remote management endpoint needs \
                 --mcp-tls-cert, --mcp-tls-key and the --mcp-oauth-* options, or an \
                 authenticated gateway in front of a loopback bind",
                self.mcp_bind
            ));
        }
        if self.mcp_path.is_empty() || !self.mcp_path.starts_with('/') {
            return Err("the MCP path must start with /".to_string());
        }
        self.server_resource_pack()?;
        Ok(())
    }

    /// `DedicatedServerProperties.getServerPackInfo` (`:179-225`), refusing
    /// what vanilla only warns about.
    pub(crate) fn server_resource_pack(&self) -> Result<Option<ServerResourcePack>, String> {
        let Some(url) = &self.resource_pack else {
            return Ok(None);
        };
        let prompt = self
            .resource_pack_prompt
            .as_deref()
            .map(|prompt| {
                let json = serde_json::from_str::<serde_json::Value>(prompt)
                    .map_err(|error| format!("resource pack prompt is not JSON: {error}"))?;
                json_value_to_nbt_summary(&json)
                    .map_err(|error| error.to_string())
                    .and_then(RichTextComponent::try_from_component_nbt)
                    .map_err(|error| format!("resource pack prompt is not a component: {error}"))
            })
            .transpose()?;
        Ok(Some(ServerResourcePack {
            id: self
                .resource_pack_id
                .unwrap_or_else(|| name_uuid_from_bytes(url.as_bytes())),
            url: url.clone(),
            hash: self.resource_pack_sha1.clone(),
            required: self.require_resource_pack,
            prompt,
        }))
    }

    /// Whether both halves of remote protection were named on the command line.
    /// The materials themselves are still read and checked at startup.
    pub(crate) fn mcp_remote_authority_declared(&self) -> bool {
        self.mcp_tls_cert.is_some()
            && self.mcp_tls_key.is_some()
            && self.mcp_oauth_resource.is_some()
            && !self.mcp_oauth_issuer.is_empty()
    }

    /// Fills every saved option the command line left unset from
    /// `server-settings.json`.
    pub(crate) fn apply_persisted(&mut self, saved: &PersistedSettings) {
        fn fill<T: Clone>(slot: &mut Option<T>, saved: &Option<T>) {
            if slot.is_none() {
                slot.clone_from(saved);
            }
        }
        fill(&mut self.white_list, &saved.white_list);
        fill(&mut self.enforce_whitelist, &saved.enforce_whitelist);
        fill(&mut self.player_idle_timeout, &saved.player_idle_timeout);
        fill(&mut self.motd, &saved.motd);
        fill(&mut self.max_players, &saved.max_players);
        fill(&mut self.difficulty, &saved.difficulty);
        fill(&mut self.gamemode, &saved.gamemode);
        fill(&mut self.spawn_protection, &saved.spawn_protection);
        fill(&mut self.view_distance, &saved.view_distance);
        fill(&mut self.simulation_distance, &saved.simulation_distance);
        fill(&mut self.hide_online_players, &saved.hide_online_players);
        fill(&mut self.enable_status, &saved.enable_status);
        fill(&mut self.accepts_transfers, &saved.accepts_transfers);
        fill(&mut self.allow_flight, &saved.allow_flight);
        fill(
            &mut self.pause_when_empty_seconds,
            &saved.pause_when_empty_seconds,
        );
        fill(
            &mut self.entity_broadcast_range_percentage,
            &saved.entity_broadcast_range_percentage,
        );
        fill(
            &mut self.op_permission_level,
            &saved.op_permission_level.map(|level| i64::from(level.id())),
        );
    }

    pub(crate) fn motd(&self) -> &str {
        self.motd.as_deref().unwrap_or(DEFAULT_MOTD)
    }

    pub(crate) fn max_players(&self) -> i32 {
        self.max_players.unwrap_or(20)
    }

    pub(crate) fn difficulty(&self) -> Difficulty {
        self.difficulty.unwrap_or(Difficulty::Easy)
    }

    pub(crate) fn gamemode(&self) -> GameType {
        self.gamemode.unwrap_or(GameType::Survival)
    }

    pub(crate) fn spawn_protection(&self) -> i32 {
        self.spawn_protection.unwrap_or(16)
    }

    pub(crate) fn view_distance(&self) -> i32 {
        self.view_distance.unwrap_or(DEFAULT_DISTANCE_CHUNKS)
    }

    pub(crate) fn simulation_distance(&self) -> i32 {
        self.simulation_distance.unwrap_or(DEFAULT_DISTANCE_CHUNKS)
    }

    pub(crate) fn op_permission_level(&self) -> CommandPermission {
        self.op_permission_level
            .and_then(permission_level_by_id)
            .unwrap_or(CommandPermission::Owner)
    }

    pub(crate) fn function_permission_level(&self) -> CommandPermission {
        permission_level_by_id(self.function_permission_level)
            .unwrap_or(CommandPermission::GameMaster)
    }

    pub(crate) fn mcp_request_timeout(&self) -> Duration {
        Duration::from_secs(self.mcp_request_timeout_seconds)
    }

    pub(crate) fn connection_limits(&self) -> ConnectionLimits {
        ConnectionLimits {
            event_entries: self.connection_event_entries,
            event_bytes: self.connection_event_bytes,
            command_entries: self.connection_command_entries,
            command_bytes: self.connection_command_bytes,
        }
    }

    pub(crate) fn compression_threshold(&self) -> Option<i32> {
        (self.compression_threshold >= 0).then_some(self.compression_threshold)
    }

    pub(crate) fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_seconds)
    }

    /// `WorldOptions.parseSeed` (`WorldOptions.java:74-85`); `None` asks for a
    /// random seed.
    pub(crate) fn parsed_level_seed(&self) -> Option<i64> {
        let seed = self.level_seed.trim_matches(|c: char| c <= ' ');
        if seed.is_empty() {
            return None;
        }
        Some(seed.parse::<i64>().unwrap_or_else(|_| {
            // `String.hashCode` over UTF-16 code units.
            i64::from(seed.encode_utf16().fold(0_i32, |hash, unit| {
                hash.wrapping_mul(31).wrapping_add(i32::from(unit))
            }))
        }))
    }
}

/// `DedicatedServerProperties.java:102-103`.
const DEFAULT_DISTANCE_CHUNKS: i32 = 10;

/// Refuses what `ChunkMap.setServerViewDistance` would clamp
/// (`ChunkMap.java:810`), so the effective distance is the one given.
fn parse_broadcast_range(value: &str) -> Result<i32, String> {
    value
        .parse::<i32>()
        .map(clamp_broadcast_range)
        .map_err(|error| format!("invalid percentage: {error}"))
}

fn parse_distance(value: &str) -> Result<i32, String> {
    let distance = value
        .parse::<i32>()
        .map_err(|error| format!("invalid distance: {error}"))?;
    if !(MIN_DISTANCE_CHUNKS..=MAX_DISTANCE_CHUNKS).contains(&distance) {
        return Err(format!(
            "distance must be in {MIN_DISTANCE_CHUNKS}..={MAX_DISTANCE_CHUNKS} chunks"
        ));
    }
    Ok(distance)
}

/// Unlike `DedicatedServerProperties.java:260-267`, an unknown preset is
/// refused rather than replaced by `normal`.
fn parse_level_type(value: &str) -> Result<Java26WorldPreset, String> {
    Java26WorldPreset::by_name(value).ok_or_else(|| {
        format!("unknown level type {value:?}; expected normal, flat, large_biomes or amplified")
    })
}

/// `server.properties` accepts the serialized names
/// (`DedicatedServerProperties.java:65-66` uses `Difficulty::byName`); the
/// numeric form vanilla also accepts has no place on a command line.
fn parse_difficulty(value: &str) -> Result<Difficulty, String> {
    vanilla_difficulty_for_serialized_name(value)
        .map(|entry| entry.difficulty)
        .ok_or_else(|| format!("unknown difficulty {value:?}"))
}

/// `GameType.byName` over the serialized names, refusing what it does not know.
///
/// Vanilla's property parser is `dispatchNumberOrString(GameType::byId,
/// GameType::byName)` (`DedicatedServerProperties.java:68-70`), whose string arm
/// answers `GameType.SURVIVAL` for a name it does not know
/// (`GameType.byName(name)`, `GameType.java:98-100`). A command line is not a
/// properties file that has to keep booting, so an unknown name is refused here
/// the same way `--difficulty` refuses one, and for the same reason: a typo that
/// silently changed the world is worse than a startup that says so.
fn parse_game_type(value: &str) -> Result<GameType, String> {
    vanilla_game_type_for_serialized_name(value)
        .map(|entry| entry.game_type)
        .ok_or_else(|| format!("unknown game mode {value:?}"))
}

/// What a credential-less loopback caller is granted.
///
/// `none` is a real answer: an operator who wants every caller to present a
/// credential, even locally, needs a way to say so, and an absent option would
/// mean the default instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackGrant {
    None,
    Scope(AdminScope),
}

impl LoopbackGrant {
    pub(crate) fn scope(self) -> Option<AdminScope> {
        match self {
            Self::None => None,
            Self::Scope(scope) => Some(scope),
        }
    }
}

fn parse_loopback_grant(value: &str) -> Result<LoopbackGrant, String> {
    if value == "none" {
        return Ok(LoopbackGrant::None);
    }
    AdminScope::by_name(value)
        .map(LoopbackGrant::Scope)
        .ok_or_else(|| format!("unknown scope {value:?}; expected none, read, manage or command"))
}

/// `Util.parseAndValidateUntrustedUri` (`Util.java:515-527`).
fn parse_untrusted_link(value: &str) -> Result<String, String> {
    match value.split_once(':') {
        Some((scheme, rest))
            if !rest.is_empty()
                && matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") =>
        {
            Ok(value.to_string())
        }
        _ => Err(format!("{value:?} is not an http or https link")),
    }
}

/// `DedicatedServerProperties.SHA1` (`:51`).
fn parse_sha1(value: &str) -> Result<String, String> {
    if value.is_empty() || (value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        Ok(value.to_string())
    } else {
        Err("a resource pack sha1 is 40 hex digits".to_string())
    }
}

fn parse_nonzero_usize(value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|error| format!("invalid positive integer: {error}"))?;
    if value == 0 {
        return Err("value must be non-zero".to_string());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_option_defaults_to_ephemeral_fixture() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(options.world, None);
        assert!(!options.safe_mode);
    }

    #[test]
    fn difficulty_defaults_to_the_vanilla_property_default_and_accepts_every_name() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(options.difficulty(), Difficulty::Easy);
        for (name, expected) in [
            ("peaceful", Difficulty::Peaceful),
            ("easy", Difficulty::Easy),
            ("normal", Difficulty::Normal),
            ("hard", Difficulty::Hard),
        ] {
            let options =
                ServerOptions::try_parse_from(["bbb-server", "--difficulty", name]).unwrap();
            assert_eq!(options.difficulty(), expected);
        }
        assert!(ServerOptions::try_parse_from(["bbb-server", "--difficulty", "1"]).is_err());
        assert!(ServerOptions::try_parse_from(["bbb-server", "--difficulty", "PEACEFUL"]).is_err());
    }

    #[test]
    fn game_mode_defaults_to_the_vanilla_property_default_and_accepts_every_name() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(options.gamemode(), GameType::Survival);
        for entry in bbb_protocol::game_types::VANILLA_GAME_TYPES {
            let options =
                ServerOptions::try_parse_from(["bbb-server", "--gamemode", entry.serialized_name])
                    .unwrap();
            assert_eq!(options.gamemode(), entry.game_type);
        }
        assert!(ServerOptions::try_parse_from(["bbb-server", "--gamemode", "1"]).is_err());
        assert!(ServerOptions::try_parse_from(["bbb-server", "--gamemode", "CREATIVE"]).is_err());
    }

    #[test]
    fn force_game_mode_defaults_to_the_vanilla_property_default_and_parses() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert!(
            !options.force_gamemode,
            "DedicatedServerProperties.java:63 declares force-gamemode false"
        );
        let forced = ServerOptions::try_parse_from(["bbb-server", "--force-gamemode"]).unwrap();
        assert!(forced.force_gamemode);
    }

    #[test]
    fn function_permission_level_defaults_to_gamemaster_and_parses() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(
            options.function_permission_level(),
            CommandPermission::GameMaster
        );
        let options =
            ServerOptions::try_parse_from(["bbb-server", "--function-permission-level", "4"])
                .unwrap();
        assert_eq!(
            options.function_permission_level(),
            CommandPermission::Owner
        );
        assert!(
            ServerOptions::try_parse_from(["bbb-server", "--function-permission-level", "5"])
                .is_err()
        );
    }

    #[test]
    fn allowlist_and_idle_flags_are_absent_unless_given() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(
            (
                options.white_list,
                options.enforce_whitelist,
                options.player_idle_timeout
            ),
            (None, None, None)
        );
        let options = ServerOptions::try_parse_from([
            "bbb-server",
            "--white-list",
            "true",
            "--enforce-whitelist",
            "false",
            "--player-idle-timeout",
            "5",
        ])
        .unwrap();
        assert_eq!(
            (
                options.white_list,
                options.enforce_whitelist,
                options.player_idle_timeout
            ),
            (Some(true), Some(false), Some(5))
        );
        assert!(
            ServerOptions::try_parse_from(["bbb-server", "--player-idle-timeout", "-1"]).is_err()
        );
    }

    #[test]
    fn saved_settings_fill_only_what_the_command_line_left_unset() {
        let mut options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(
            (options.view_distance(), options.simulation_distance()),
            (10, 10)
        );
        assert_eq!(options.op_permission_level(), CommandPermission::Owner);
        let mut options_with_flags =
            ServerOptions::try_parse_from(["bbb-server", "--view-distance", "6", "--motd", "flag"])
                .unwrap();
        let saved = PersistedSettings {
            motd: Some("saved".to_string()),
            view_distance: Some(12),
            simulation_distance: Some(4),
            max_players: Some(3),
            op_permission_level: Some(CommandPermission::GameMaster),
            ..PersistedSettings::default()
        };
        options.apply_persisted(&saved);
        options_with_flags.apply_persisted(&saved);
        assert_eq!(
            (
                options.motd(),
                options.view_distance(),
                options.simulation_distance(),
                options.max_players(),
                options.op_permission_level()
            ),
            ("saved", 12, 4, 3, CommandPermission::GameMaster)
        );
        assert_eq!(
            (
                options_with_flags.motd(),
                options_with_flags.view_distance()
            ),
            ("flag", 6)
        );
        for distance in ["1", "33"] {
            assert!(
                ServerOptions::try_parse_from(["bbb-server", "--view-distance", distance]).is_err()
            );
        }
    }

    #[test]
    fn broadcast_range_flag_is_clamped_and_overrides_the_saved_value() {
        let mut options = ServerOptions::try_parse_from([
            "bbb-server",
            "--entity-broadcast-range-percentage",
            "5000",
        ])
        .unwrap();
        options.apply_persisted(&PersistedSettings {
            entity_broadcast_range_percentage: Some(50),
            ..PersistedSettings::default()
        });
        assert_eq!(options.entity_broadcast_range_percentage, Some(1000));
    }

    #[test]
    fn safe_mode_flag_parses() {
        let options = ServerOptions::try_parse_from(["bbb-server", "--safe-mode"]).unwrap();
        assert!(options.safe_mode);
    }

    #[test]
    fn spawn_protection_defaults_and_accepts_configured_or_disabled_radius() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(options.spawn_protection(), 16);
        for radius in ["24", "0", "-1"] {
            let options =
                ServerOptions::try_parse_from(["bbb-server", "--spawn-protection", radius])
                    .unwrap();
            options.validate().unwrap();
            assert_eq!(options.spawn_protection(), radius.parse::<i32>().unwrap());
        }
    }

    #[test]
    fn world_option_accepts_save_directory() {
        let options =
            ServerOptions::try_parse_from(["bbb-server", "--world", "saves/world"]).unwrap();
        assert_eq!(options.world, Some(PathBuf::from("saves/world")));
    }

    #[test]
    fn new_world_options_default_to_a_random_normal_world_with_structures() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(options.parsed_level_seed(), None);
        assert_eq!(options.level_type, Java26WorldPreset::Normal);
        assert!(options.generate_structures);

        let options = ServerOptions::try_parse_from([
            "bbb-server",
            "--level-seed",
            "-42",
            "--level-type",
            "minecraft:large_biomes",
            "--generate-structures",
            "false",
        ])
        .unwrap();
        assert_eq!(options.parsed_level_seed(), Some(-42));
        assert_eq!(options.level_type, Java26WorldPreset::LargeBiomes);
        assert!(!options.generate_structures);
        for (name, preset) in [
            ("flat", Java26WorldPreset::Flat),
            ("amplified", Java26WorldPreset::Amplified),
            ("NORMAL", Java26WorldPreset::Normal),
        ] {
            let options =
                ServerOptions::try_parse_from(["bbb-server", "--level-type", name]).unwrap();
            assert_eq!(options.level_type, preset);
        }
        assert!(ServerOptions::try_parse_from(["bbb-server", "--level-type", "moon"]).is_err());
    }

    #[test]
    fn a_text_level_seed_hashes_like_java_strings() {
        let options =
            ServerOptions::try_parse_from(["bbb-server", "--level-seed", " bbb "]).unwrap();
        // "bbb".hashCode() == 98 * 31^2 + 98 * 31 + 98
        assert_eq!(options.parsed_level_seed(), Some(97_314));
        let options = ServerOptions::try_parse_from(["bbb-server", "--level-seed", "  "]).unwrap();
        assert_eq!(options.parsed_level_seed(), None);
    }

    #[test]
    fn world_option_requires_a_value() {
        assert!(ServerOptions::try_parse_from(["bbb-server", "--world"]).is_err());
    }

    #[test]
    fn the_management_endpoint_defaults_to_loopback_and_a_manage_grant() {
        let options = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert!(!options.no_mcp);
        assert_eq!(options.mcp_bind.to_string(), "127.0.0.1:25585");
        assert_eq!(options.mcp_path, "/mcp");
        assert_eq!(
            options.mcp_loopback_scope,
            LoopbackGrant::Scope(AdminScope::Manage)
        );
        assert_eq!(options.admin_dir, None);
        assert_eq!(options.validate(), Ok(()));
    }

    #[test]
    fn the_loopback_grant_can_be_narrowed_or_switched_off() {
        for (value, expected) in [
            ("none", LoopbackGrant::None),
            ("read", LoopbackGrant::Scope(AdminScope::Read)),
            ("command", LoopbackGrant::Scope(AdminScope::Command)),
        ] {
            let options =
                ServerOptions::try_parse_from(["bbb-server", "--mcp-loopback-scope", value])
                    .unwrap();
            assert_eq!(options.mcp_loopback_scope, expected);
        }
        assert!(
            ServerOptions::try_parse_from(["bbb-server", "--mcp-loopback-scope", "owner"]).is_err()
        );
    }

    #[test]
    fn a_remote_management_bind_is_refused_until_it_is_protected() {
        let remote =
            ServerOptions::try_parse_from(["bbb-server", "--mcp-bind", "0.0.0.0:25585"]).unwrap();
        assert!(remote.validate().is_err());

        // Switching the endpoint off is also a way to bind nothing remotely.
        let disabled = ServerOptions::try_parse_from([
            "bbb-server",
            "--mcp-bind",
            "0.0.0.0:25585",
            "--no-mcp",
        ])
        .unwrap();
        assert_eq!(disabled.validate(), Ok(()));

        let protected = ServerOptions::try_parse_from([
            "bbb-server",
            "--mcp-bind",
            "0.0.0.0:25585",
            "--mcp-tls-cert",
            "tls/chain.pem",
            "--mcp-tls-key",
            "tls/key.pem",
            "--mcp-oauth-resource",
            "https://server.example/mcp",
            "--mcp-oauth-issuer",
            "https://login.example",
            "--mcp-oauth-jwks",
            "tls/jwks.json",
        ])
        .unwrap();
        assert_eq!(protected.validate(), Ok(()));
    }

    #[test]
    fn half_declared_protection_is_refused_rather_than_half_applied() {
        let cases: [&[&str]; 3] = [
            &["--mcp-tls-cert", "tls/chain.pem"],
            &["--mcp-oauth-resource", "https://server.example/mcp"],
            &["--mcp-oauth-issuer", "https://login.example"],
        ];
        for case in cases {
            let mut arguments = vec!["bbb-server"];
            arguments.extend_from_slice(case);
            let options = ServerOptions::try_parse_from(arguments).unwrap();
            assert!(options.validate().is_err(), "{case:?}");
        }
    }

    #[test]
    fn max_connections_per_ip_defaults_to_sixteen_and_requires_nonzero() {
        let default_opts = ServerOptions::try_parse_from(["bbb-server"]).unwrap();
        assert_eq!(default_opts.max_connections_per_ip, 16);

        let raised =
            ServerOptions::try_parse_from(["bbb-server", "--max-connections-per-ip", "32"])
                .unwrap();
        assert_eq!(raised.max_connections_per_ip, 32);

        assert!(
            ServerOptions::try_parse_from(["bbb-server", "--max-connections-per-ip", "0"]).is_err()
        );
    }
}
