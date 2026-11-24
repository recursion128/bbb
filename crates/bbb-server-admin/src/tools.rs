//! The MCP tool catalog: one tool per administration request.
//!
//! Vanilla owner: `IncomingRpcMethods`, which registers one JSON-RPC method per
//! service call with a declared parameter and response schema. Each tool below
//! projects one of those methods, and every argument name is the name vanilla's
//! own `param(...)` declaration uses, so a management client written against the
//! official server's schema needs no renaming to read this one.
//!
//! bbb also exposes its own administration tools:
//!
//! * `serversettings` and `serversettings.set` collapse vanilla's per-key method
//!   pairs into one readout and one keyed write. The key set is still vanilla's
//!   own, asserted by `scripts/tests/test_admin_settings_alignment.py`.
//! * `server.diagnostics` has no vanilla method; it reports counters the tick
//!   already owns.
//! * `world.*` reads world metadata and loaded terrain through the tick.
//! * `command.execute` has no vanilla method either. It is the ledger's
//!   explicitly highest-privilege surface and delegates the single command
//!   dispatcher rather than reimplementing anything it can reach.
//!
//! Authority never comes from this table. A tool's required scope is read from
//! the `AdminRequest` it decodes to, and the annotations are derived from the
//! same request, so a mislabelled annotation cannot widen what a principal may
//! do.

use bbb_mcp::ToolDescriptor;
use bbb_protocol::game_rules::{vanilla_game_rule, GameRuleValue, GameRuleValueKind};
use bbb_protocol::packets::BlockPos;
use bbb_server_core::admin::protocol::MAX_WORLD_QUERY_CELLS;
use bbb_server_core::admin::{
    difficulty_by_name, game_type_by_name, parse_instant_iso8601, permission_level_by_id,
    AdminError, AdminMessage, AdminRequest, AdminScope, AdminSetting, IncomingIpBanRequest,
    IpBanRequest, KickRequest, OperatorRequest, PlayerRef, SystemMessageRequest,
    ADMIN_SETTING_KEYS, MAX_ADMIN_TEXT_CHARS, MAX_MESSAGE_PARAMS, MAX_REQUEST_ENTRIES,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

/// One tool: what it is called, what it accepts, and what request it becomes.
pub struct AdminTool {
    /// The MCP tool name, which is also the request's audit action.
    pub name: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    /// The `IncomingRpcMethods` method this projects, when there is one.
    pub vanilla_method: Option<&'static str>,
    /// Whether applying this tool can remove or end something. A hint for a
    /// host's confirmation policy, never an authorization decision.
    pub destructive: bool,
    /// A canonical request of this tool's own kind. Scope and the read-only
    /// annotation are read from it rather than declared twice.
    pub example: fn() -> AdminRequest,
    pub schema: fn() -> Value,
    pub decode: fn(&Value) -> Result<AdminRequest, AdminError>,
}

impl AdminTool {
    pub fn scope(&self) -> AdminScope {
        (self.example)().required_scope()
    }

    pub fn read_only(&self) -> bool {
        !(self.example)().is_mutation()
    }

    /// The audit action of the request this tool decodes to, which the catalog
    /// test asserts equals [`Self::name`].
    pub fn action(&self) -> &'static str {
        (self.example)().action()
    }

    /// The `tools/list` entry, as the shared transport lists it.
    pub fn descriptor_entry(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: self.name,
            title: self.title,
            summary: self.summary,
            input_schema: (self.schema)(),
            read_only: self.read_only(),
            destructive: self.destructive,
            meta: Some(json!({
                "bbb/scope": self.scope().name(),
                "bbb/vanillaMethod": self.vanilla_method,
            })),
        }
    }

    /// The `tools/list` entry.
    pub fn descriptor(&self) -> Value {
        self.descriptor_entry().to_value()
    }
}

pub fn catalog() -> &'static [AdminTool] {
    TOOLS
}

pub fn tool_by_name(name: &str) -> Option<&'static AdminTool> {
    TOOLS.iter().find(|tool| tool.name == name)
}

static TOOLS: &[AdminTool] = &[
    AdminTool {
        name: "server.status",
        title: "Server status",
        summary: "Whether the server is accepting players, who is on it, and which version it speaks.",
        vanilla_method: Some("server/status"),
        destructive: false,
        example: || AdminRequest::Status,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Status)
        },
    },
    AdminTool {
        name: "players",
        title: "Online players",
        summary: "Every player currently connected, by id and name.",
        vanilla_method: Some("players"),
        destructive: false,
        example: || AdminRequest::Players,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Players)
        },
    },
    AdminTool {
        name: "allowlist",
        title: "Allowlist",
        summary: "The players the allowlist admits.",
        vanilla_method: Some("allowlist"),
        destructive: false,
        example: || AdminRequest::Allowlist,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Allowlist)
        },
    },
    AdminTool {
        name: "bans",
        title: "Player bans",
        summary: "The banned players, with reason, source and expiry.",
        vanilla_method: Some("bans"),
        destructive: false,
        example: || AdminRequest::Bans,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Bans)
        },
    },
    AdminTool {
        name: "ip_bans",
        title: "IP bans",
        summary: "The banned addresses, with reason, source and expiry.",
        vanilla_method: Some("ip_bans"),
        destructive: false,
        example: || AdminRequest::IpBans,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::IpBans)
        },
    },
    AdminTool {
        name: "operators",
        title: "Operators",
        summary: "The operators, with permission level and player-limit bypass.",
        vanilla_method: Some("operators"),
        destructive: false,
        example: || AdminRequest::Operators,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Operators)
        },
    },
    AdminTool {
        name: "serversettings",
        title: "Server settings",
        summary: "Every server setting this server owns, with its current value.",
        vanilla_method: None,
        destructive: false,
        example: || AdminRequest::Settings,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Settings)
        },
    },
    AdminTool {
        name: "gamerules",
        title: "Game rules",
        summary: "Every game rule enabled on this world, with its type and current value.",
        vanilla_method: Some("gamerules"),
        destructive: false,
        example: || AdminRequest::GameRules,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::GameRules)
        },
    },
    AdminTool {
        name: "server.diagnostics",
        title: "Runtime diagnostics",
        summary: "Bounded runtime counters: tick, players, loaded chunks, list sizes and applied operations.",
        vanilla_method: None,
        destructive: false,
        example: || AdminRequest::Diagnostics,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Diagnostics)
        },
    },
    AdminTool {
        name: "world.info",
        title: "World information",
        summary: "Lists dimension IDs, inclusive build heights, loaded chunk counts, and the shared spawn position.",
        vanilla_method: None,
        destructive: false,
        example: || AdminRequest::WorldInfo,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::WorldInfo)
        },
    },
    AdminTool {
        name: "world.blocks",
        title: "Inspect block states",
        summary: "Reads block states at 1 to 256 in-bounds positions in one dimension, in request order. Unloaded positions have a null state.",
        vanilla_method: None,
        destructive: false,
        example: || AdminRequest::WorldBlocks {
            dimension: "minecraft:overworld".to_owned(),
            positions: vec![BlockPos { x: 0, y: 64, z: 0 }],
        },
        schema: || {
            let coord = coordinate_schema();
            object_schema(
                json!({
                    "dimension": dimension_schema(),
                    "positions": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": MAX_WORLD_QUERY_CELLS,
                        "items": object_schema(json!({ "x": coord, "y": coord, "z": coord }), &["x", "y", "z"]),
                    },
                }),
                &["dimension", "positions"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["dimension", "positions"])?;
            let dimension = text(required(arguments, "dimension")?, "dimension")?;
            let raw_positions = required(arguments, "positions")?
                .as_array()
                .ok_or_else(|| AdminError::invalid_params("positions is an array"))?;
            if !(1..=MAX_WORLD_QUERY_CELLS).contains(&raw_positions.len()) {
                return Err(AdminError::invalid_params(format!(
                    "world.blocks needs 1 to {MAX_WORLD_QUERY_CELLS} positions"
                )));
            }
            let positions = raw_positions
                .iter()
                .map(|value| {
                    let position = object(value, "position")?;
                    reject_unknown(position, &["x", "y", "z"], "position")?;
                    Ok(BlockPos {
                        x: coordinate(position, "x")?,
                        y: coordinate(position, "y")?,
                        z: coordinate(position, "z")?,
                    })
                })
                .collect::<Result<Vec<_>, AdminError>>()?;
            Ok(AdminRequest::WorldBlocks { dimension, positions })
        },
    },
    AdminTool {
        name: "world.surface",
        title: "Inspect surface heights",
        summary: "Reports the Y of the highest non-air block, including liquids, for an ordered, inclusive rectangle of up to 256 columns. Unloaded columns report loaded=false and a null topY. Empty loaded columns also have a null topY.",
        vanilla_method: None,
        destructive: false,
        example: || AdminRequest::WorldSurface {
            dimension: "minecraft:overworld".to_owned(),
            min_x: 0,
            min_z: 0,
            max_x: 0,
            max_z: 0,
        },
        schema: || {
            let coord = coordinate_schema();
            object_schema(
                json!({
                    "dimension": dimension_schema(),
                    "minX": coord,
                    "minZ": coord,
                    "maxX": coord,
                    "maxZ": coord,
                }),
                &["dimension", "minX", "minZ", "maxX", "maxZ"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["dimension", "minX", "minZ", "maxX", "maxZ"])?;
            Ok(AdminRequest::WorldSurface {
                dimension: text(required(arguments, "dimension")?, "dimension")?,
                min_x: coordinate(arguments, "minX")?,
                min_z: coordinate(arguments, "minZ")?,
                max_x: coordinate(arguments, "maxX")?,
                max_z: coordinate(arguments, "maxZ")?,
            })
        },
    },
    AdminTool {
        name: "server.save",
        title: "Save the world",
        summary: "Saves player data and every loaded chunk. Set flush to wait for the write to reach disk.",
        vanilla_method: Some("server/save"),
        destructive: false,
        example: || AdminRequest::Save { flush: false },
        schema: || {
            object_schema(
                json!({ "flush": { "type": "boolean", "description": "Wait for the write to reach disk." } }),
                &[],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["flush"])?;
            let flush = match field(arguments, "flush") {
                Some(value) => boolean(value, "flush")?,
                None => false,
            };
            Ok(AdminRequest::Save { flush })
        },
    },
    AdminTool {
        name: "server.stop",
        title: "Stop the server",
        summary: "Saves and stops the server. Every player is disconnected.",
        vanilla_method: Some("server/stop"),
        destructive: true,
        example: || AdminRequest::Stop,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::Stop)
        },
    },
    AdminTool {
        name: "server.system_message",
        title: "Send a system message",
        summary: "Sends a system message to every player, or to the named players only.",
        vanilla_method: Some("server/system_message"),
        destructive: false,
        example: || {
            AdminRequest::SystemMessage(SystemMessageRequest {
                message: AdminMessage::literal("hello"),
                overlay: false,
                receiving_players: None,
            })
        },
        schema: || {
            object_schema(
                json!({
                    "message": {
                        "type": "object",
                        "description": "The message, its overlay flag and its optional recipients.",
                        "properties": {
                            "message": message_schema(),
                            "overlay": { "type": "boolean", "description": "Show above the hotbar instead of in chat." },
                            "receivingPlayers": players_schema("The players to send to; absent broadcasts."),
                        },
                        "required": ["message", "overlay"],
                        "additionalProperties": false,
                    },
                }),
                &["message"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["message"])?;
            let request = object(required(arguments, "message")?, "message")?;
            reject_unknown(request, &["message", "overlay", "receivingPlayers"], "message")?;
            let receiving_players = match field(request, "receivingPlayers") {
                Some(value) => Some(player_list(value, "receivingPlayers")?),
                None => None,
            };
            Ok(AdminRequest::SystemMessage(SystemMessageRequest {
                message: message(required(request, "message")?)?,
                overlay: boolean(required(request, "overlay")?, "overlay")?,
                receiving_players,
            }))
        },
    },
    AdminTool {
        name: "players.kick",
        title: "Kick players",
        summary: "Disconnects the named players, with an optional message each.",
        vanilla_method: Some("players/kick"),
        destructive: true,
        example: || AdminRequest::KickPlayers(Vec::new()),
        schema: || {
            object_schema(
                json!({
                    "kick": {
                        "type": "array",
                        "description": "The players to disconnect.",
                        "maxItems": MAX_REQUEST_ENTRIES,
                        "items": {
                            "type": "object",
                            "properties": { "player": player_schema(), "message": message_schema() },
                            "required": ["player"],
                            "additionalProperties": false,
                        },
                    },
                }),
                &["kick"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["kick"])?;
            let entries = entries(required(arguments, "kick")?, "kick", |entry| {
                let entry = object(entry, "kick")?;
                reject_unknown(entry, &["player", "message"], "kick")?;
                Ok(KickRequest {
                    player: player(required(entry, "player")?)?,
                    message: match field(entry, "message") {
                        Some(value) => Some(message(value)?),
                        None => None,
                    },
                })
            })?;
            Ok(AdminRequest::KickPlayers(entries))
        },
    },
    AdminTool {
        name: "allowlist.set",
        title: "Replace the allowlist",
        summary: "Replaces the allowlist with exactly these players.",
        vanilla_method: Some("allowlist/set"),
        destructive: true,
        example: || AdminRequest::AllowlistSet(Vec::new()),
        schema: || players_argument("players", "The players the allowlist will hold."),
        decode: |value| Ok(AdminRequest::AllowlistSet(players_argument_of(value, "players")?)),
    },
    AdminTool {
        name: "allowlist.add",
        title: "Add to the allowlist",
        summary: "Adds players to the allowlist.",
        vanilla_method: Some("allowlist/add"),
        destructive: false,
        example: || AdminRequest::AllowlistAdd(Vec::new()),
        schema: || players_argument("add", "The players to admit."),
        decode: |value| Ok(AdminRequest::AllowlistAdd(players_argument_of(value, "add")?)),
    },
    AdminTool {
        name: "allowlist.remove",
        title: "Remove from the allowlist",
        summary: "Removes players from the allowlist. With enforcement on, they are kicked.",
        vanilla_method: Some("allowlist/remove"),
        destructive: true,
        example: || AdminRequest::AllowlistRemove(Vec::new()),
        schema: || players_argument("remove", "The players to stop admitting."),
        decode: |value| {
            Ok(AdminRequest::AllowlistRemove(players_argument_of(
                value, "remove",
            )?))
        },
    },
    AdminTool {
        name: "allowlist.clear",
        title: "Clear the allowlist",
        summary: "Empties the allowlist.",
        vanilla_method: Some("allowlist/clear"),
        destructive: true,
        example: || AdminRequest::AllowlistClear,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::AllowlistClear)
        },
    },
    AdminTool {
        name: "bans.set",
        title: "Replace the ban list",
        summary: "Replaces the player ban list with exactly these bans.",
        vanilla_method: Some("bans/set"),
        destructive: true,
        example: || AdminRequest::BansSet(Vec::new()),
        schema: || user_bans_argument("bans", "The bans the list will hold."),
        decode: |value| Ok(AdminRequest::BansSet(user_bans_argument_of(value, "bans")?)),
    },
    AdminTool {
        name: "bans.add",
        title: "Ban players",
        summary: "Bans players and disconnects them if they are online.",
        vanilla_method: Some("bans/add"),
        destructive: true,
        example: || AdminRequest::BansAdd(Vec::new()),
        schema: || user_bans_argument("add", "The bans to add."),
        decode: |value| Ok(AdminRequest::BansAdd(user_bans_argument_of(value, "add")?)),
    },
    AdminTool {
        name: "bans.remove",
        title: "Unban players",
        summary: "Removes players from the ban list.",
        vanilla_method: Some("bans/remove"),
        destructive: false,
        example: || AdminRequest::BansRemove(Vec::new()),
        schema: || players_argument("remove", "The players to unban."),
        decode: |value| Ok(AdminRequest::BansRemove(players_argument_of(value, "remove")?)),
    },
    AdminTool {
        name: "bans.clear",
        title: "Clear the ban list",
        summary: "Empties the player ban list.",
        vanilla_method: Some("bans/clear"),
        destructive: false,
        example: || AdminRequest::BansClear,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::BansClear)
        },
    },
    AdminTool {
        name: "ip_bans.set",
        title: "Replace the IP ban list",
        summary: "Replaces the address ban list with exactly these bans.",
        vanilla_method: Some("ip_bans/set"),
        destructive: true,
        example: || AdminRequest::IpBansSet(Vec::new()),
        schema: || {
            object_schema(
                json!({
                    "banlist": {
                        "type": "array",
                        "description": "The address bans the list will hold.",
                        "maxItems": MAX_REQUEST_ENTRIES,
                        "items": {
                            "type": "object",
                            "properties": {
                                "ip": { "type": "string", "description": "An IP literal, never a host name." },
                                "reason": ban_text_schema("Why the address is banned."),
                                "source": ban_text_schema("Who banned it."),
                                "expires": instant_schema(),
                            },
                            "required": ["ip"],
                            "additionalProperties": false,
                        },
                    },
                }),
                &["banlist"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["banlist"])?;
            let entries = entries(required(arguments, "banlist")?, "banlist", |entry| {
                let entry = object(entry, "banlist")?;
                reject_unknown(entry, &["ip", "reason", "source", "expires"], "banlist")?;
                Ok(IpBanRequest {
                    ip: text(required(entry, "ip")?, "ip")?,
                    reason: optional_text(entry, "reason")?,
                    source: optional_text(entry, "source")?,
                    expires: optional_instant(entry, "expires")?,
                })
            })?;
            Ok(AdminRequest::IpBansSet(entries))
        },
    },
    AdminTool {
        name: "ip_bans.add",
        title: "Ban addresses",
        summary: "Bans an address literal, or the address a named player is connected from.",
        vanilla_method: Some("ip_bans/add"),
        destructive: true,
        example: || AdminRequest::IpBansAdd(Vec::new()),
        schema: || {
            object_schema(
                json!({
                    "add": {
                        "type": "array",
                        "description": "The address bans to add.",
                        "maxItems": MAX_REQUEST_ENTRIES,
                        "items": {
                            "type": "object",
                            "properties": {
                                "player": player_schema(),
                                "ip": { "type": "string", "description": "An IP literal, never a host name." },
                                "reason": ban_text_schema("Why the address is banned."),
                                "source": ban_text_schema("Who banned it."),
                                "expires": instant_schema(),
                            },
                            "anyOf": [{ "required": ["player"] }, { "required": ["ip"] }],
                            "additionalProperties": false,
                        },
                    },
                }),
                &["add"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["add"])?;
            let entries = entries(required(arguments, "add")?, "add", |entry| {
                let entry = object(entry, "add")?;
                reject_unknown(entry, &["player", "ip", "reason", "source", "expires"], "add")?;
                let player = match field(entry, "player") {
                    Some(value) => Some(player(value)?),
                    None => None,
                };
                let ip = optional_text(entry, "ip")?;
                if player.is_none() && ip.is_none() {
                    return Err(AdminError::invalid_params(
                        "an address ban names a player or an ip",
                    ));
                }
                Ok(IncomingIpBanRequest {
                    player,
                    ip,
                    reason: optional_text(entry, "reason")?,
                    source: optional_text(entry, "source")?,
                    expires: optional_instant(entry, "expires")?,
                })
            })?;
            Ok(AdminRequest::IpBansAdd(entries))
        },
    },
    AdminTool {
        name: "ip_bans.remove",
        title: "Unban addresses",
        summary: "Removes address literals from the ban list.",
        vanilla_method: Some("ip_bans/remove"),
        destructive: false,
        example: || AdminRequest::IpBansRemove(Vec::new()),
        schema: || {
            object_schema(
                json!({
                    "ip": {
                        "type": "array",
                        "description": "The address literals to unban.",
                        "maxItems": MAX_REQUEST_ENTRIES,
                        "items": { "type": "string" },
                    },
                }),
                &["ip"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["ip"])?;
            let entries = entries(required(arguments, "ip")?, "ip", |entry| text(entry, "ip"))?;
            Ok(AdminRequest::IpBansRemove(entries))
        },
    },
    AdminTool {
        name: "ip_bans.clear",
        title: "Clear the IP ban list",
        summary: "Empties the address ban list.",
        vanilla_method: Some("ip_bans/clear"),
        destructive: false,
        example: || AdminRequest::IpBansClear,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::IpBansClear)
        },
    },
    AdminTool {
        name: "operators.set",
        title: "Replace the operators",
        summary: "Replaces the operator list with exactly these operators.",
        vanilla_method: Some("operators/set"),
        destructive: true,
        example: || AdminRequest::OperatorsSet(Vec::new()),
        schema: || operators_argument("operators", "The operators the list will hold."),
        decode: |value| {
            Ok(AdminRequest::OperatorsSet(operators_argument_of(
                value,
                "operators",
            )?))
        },
    },
    AdminTool {
        name: "operators.add",
        title: "Op players",
        summary: "Grants operator status, at the default level unless one is named.",
        vanilla_method: Some("operators/add"),
        destructive: false,
        example: || AdminRequest::OperatorsAdd(Vec::new()),
        schema: || operators_argument("add", "The players to op."),
        decode: |value| Ok(AdminRequest::OperatorsAdd(operators_argument_of(value, "add")?)),
    },
    AdminTool {
        name: "operators.remove",
        title: "Deop players",
        summary: "Removes operator status.",
        vanilla_method: Some("operators/remove"),
        destructive: true,
        example: || AdminRequest::OperatorsRemove(Vec::new()),
        schema: || players_argument("remove", "The players to deop."),
        decode: |value| {
            Ok(AdminRequest::OperatorsRemove(players_argument_of(
                value, "remove",
            )?))
        },
    },
    AdminTool {
        name: "operators.clear",
        title: "Deop everyone",
        summary: "Empties the operator list.",
        vanilla_method: Some("operators/clear"),
        destructive: true,
        example: || AdminRequest::OperatorsClear,
        schema: no_arguments,
        decode: |value| {
            arguments(value, &[])?;
            Ok(AdminRequest::OperatorsClear)
        },
    },
    AdminTool {
        name: "serversettings.set",
        title: "Set a server setting",
        summary: "Writes one server setting and answers with the value that took effect.",
        vanilla_method: None,
        destructive: false,
        example: || AdminRequest::SetSetting(AdminSetting::StatusReplies(true)),
        schema: || {
            object_schema(
                json!({
                    "key": {
                        "type": "string",
                        "description": "Which setting to write.",
                        "enum": ADMIN_SETTING_KEYS,
                    },
                    "value": {
                        "type": ["boolean", "integer", "string"],
                        "description": "The new value, in the type that setting takes.",
                    },
                }),
                &["key", "value"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["key", "value"])?;
            let key = text(required(arguments, "key")?, "key")?;
            setting_from_json(&key, required(arguments, "value")?).map(AdminRequest::SetSetting)
        },
    },
    AdminTool {
        name: "gamerules.update",
        title: "Update a game rule",
        summary: "Writes one game rule and answers with its typed value.",
        vanilla_method: Some("gamerules/update"),
        destructive: false,
        example: || AdminRequest::UpdateGameRule {
            key: "minecraft:advance_time".to_string(),
            value: GameRuleValue::Boolean(true),
        },
        schema: || {
            object_schema(
                json!({
                    "gamerule": {
                        "type": "object",
                        "description": "The rule to write and its new value.",
                        "properties": {
                            "key": { "type": "string", "description": "The game rule id, such as minecraft:advance_time." },
                            "value": { "type": ["boolean", "integer"], "description": "The new value, in the rule's own type." },
                        },
                        "required": ["key", "value"],
                        "additionalProperties": false,
                    },
                }),
                &["gamerule"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["gamerule"])?;
            let update = object(required(arguments, "gamerule")?, "gamerule")?;
            reject_unknown(update, &["key", "value"], "gamerule")?;
            let key = text(required(update, "key")?, "key")?;
            let value = game_rule_value(&key, required(update, "value")?)?;
            Ok(AdminRequest::UpdateGameRule { key, value })
        },
    },
    AdminTool {
        name: "command.execute",
        title: "Run a server command",
        summary: "Runs one command through the server's single dispatcher at owner permission. This can reach anything a command can reach.",
        vanilla_method: None,
        destructive: true,
        example: || AdminRequest::ExecuteCommand {
            command: "list".to_string(),
        },
        schema: || {
            object_schema(
                json!({
                    "command": {
                        "type": "string",
                        "description": "The command, without a leading slash.",
                        "maxLength": bbb_server_core::admin::protocol::MAX_ADMIN_COMMAND_CHARS,
                    },
                }),
                &["command"],
            )
        },
        decode: |value| {
            let arguments = arguments(value, &["command"])?;
            Ok(AdminRequest::ExecuteCommand {
                command: text(required(arguments, "command")?, "command")?,
            })
        },
    },
];

fn no_arguments() -> Value {
    object_schema(json!({}), &[])
}

fn object_schema(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn dimension_schema() -> Value {
    json!({
        "type": "string",
        "maxLength": MAX_ADMIN_TEXT_CHARS,
        "description": "A namespaced dimension ID, such as minecraft:overworld. See world.info.",
    })
}

fn coordinate_schema() -> Value {
    json!({ "type": "integer", "minimum": i32::MIN, "maximum": i32::MAX })
}

/// `Schema.PLAYER_SCHEMA` (Schema.java:68): a player is named by id, by name,
/// or by both.
fn player_schema() -> Value {
    json!({
        "type": "object",
        "description": "A player, by id, by name, or by both.",
        "properties": {
            "id": { "type": "string", "format": "uuid" },
            "name": { "type": "string" },
        },
        "anyOf": [{ "required": ["id"] }, { "required": ["name"] }],
        "additionalProperties": false,
    })
}

fn players_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "maxItems": MAX_REQUEST_ENTRIES,
        "items": player_schema(),
    })
}

/// `Schema.MESSAGE_SCHEMA` (Schema.java:92).
fn message_schema() -> Value {
    json!({
        "type": "object",
        "description": "A message, as literal text or as a translation key with arguments.",
        "properties": {
            "literal": { "type": "string", "maxLength": MAX_ADMIN_TEXT_CHARS },
            "translatable": { "type": "string", "maxLength": MAX_ADMIN_TEXT_CHARS },
            "translatableParams": {
                "type": "array",
                "maxItems": MAX_MESSAGE_PARAMS,
                "items": { "type": "string", "maxLength": MAX_ADMIN_TEXT_CHARS },
            },
        },
        "additionalProperties": false,
    })
}

fn instant_schema() -> Value {
    json!({
        "type": "string",
        "format": "date-time",
        "description": "When the ban expires, as an ISO-8601 instant. Absent means never.",
    })
}

fn ban_text_schema(description: &str) -> Value {
    json!({ "type": "string", "description": description, "maxLength": MAX_ADMIN_TEXT_CHARS })
}

fn players_argument(field: &str, description: &str) -> Value {
    let mut properties = Map::new();
    properties.insert(field.to_string(), players_schema(description));
    object_schema(Value::Object(properties), &[field])
}

fn players_argument_of(value: &Value, name: &str) -> Result<Vec<PlayerRef>, AdminError> {
    let arguments = arguments(value, &[name])?;
    player_list(required(arguments, name)?, name)
}

fn user_bans_argument(field: &str, description: &str) -> Value {
    let mut properties = Map::new();
    properties.insert(
        field.to_string(),
        json!({
            "type": "array",
            "description": description,
            "maxItems": MAX_REQUEST_ENTRIES,
            "items": {
                "type": "object",
                "properties": {
                    "player": player_schema(),
                    "reason": ban_text_schema("Why the player is banned."),
                    "source": ban_text_schema("Who banned them."),
                    "expires": instant_schema(),
                },
                "required": ["player"],
                "additionalProperties": false,
            },
        }),
    );
    object_schema(Value::Object(properties), &[field])
}

fn user_bans_argument_of(
    value: &Value,
    name: &str,
) -> Result<Vec<bbb_server_core::admin::UserBanRequest>, AdminError> {
    let arguments = arguments(value, &[name])?;
    entries(required(arguments, name)?, name, |entry| {
        let entry = object(entry, name)?;
        reject_unknown(entry, &["player", "reason", "source", "expires"], name)?;
        Ok(bbb_server_core::admin::UserBanRequest {
            player: player(required(entry, "player")?)?,
            reason: optional_text(entry, "reason")?,
            source: optional_text(entry, "source")?,
            expires: optional_instant(entry, "expires")?,
        })
    })
}

fn operators_argument(field: &str, description: &str) -> Value {
    let mut properties = Map::new();
    properties.insert(
        field.to_string(),
        json!({
            "type": "array",
            "description": description,
            "maxItems": MAX_REQUEST_ENTRIES,
            "items": {
                "type": "object",
                "properties": {
                    "player": player_schema(),
                    "permissionLevel": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 4,
                        "description": "The command permission level; absent takes the server default.",
                    },
                    "bypassesPlayerLimit": {
                        "type": "boolean",
                        "description": "Whether this operator may join a full server.",
                    },
                },
                "required": ["player"],
                "additionalProperties": false,
            },
        }),
    );
    object_schema(Value::Object(properties), &[field])
}

fn operators_argument_of(value: &Value, name: &str) -> Result<Vec<OperatorRequest>, AdminError> {
    let arguments = arguments(value, &[name])?;
    entries(required(arguments, name)?, name, |entry| {
        let entry = object(entry, name)?;
        reject_unknown(
            entry,
            &["player", "permissionLevel", "bypassesPlayerLimit"],
            name,
        )?;
        let permission_level = match field(entry, "permissionLevel") {
            Some(value) => {
                let level = integer(value, "permissionLevel")?;
                Some(permission_level_by_id(level).ok_or_else(|| {
                    AdminError::invalid_params(format!("{level} is not a permission level"))
                })?)
            }
            None => None,
        };
        let bypasses_player_limit = match field(entry, "bypassesPlayerLimit") {
            Some(value) => Some(boolean(value, "bypassesPlayerLimit")?),
            None => None,
        };
        Ok(OperatorRequest {
            player: player(required(entry, "player")?)?,
            permission_level,
            bypasses_player_limit,
        })
    })
}

/// One setting from its key and its JSON value, in the type that key's vanilla
/// `serversettings/<key>/set` method takes.
pub fn setting_from_json(key: &str, value: &Value) -> Result<AdminSetting, AdminError> {
    let setting = match key {
        "difficulty" => {
            let name = text(value, "value")?;
            AdminSetting::Difficulty(difficulty_by_name(&name).ok_or_else(|| {
                AdminError::invalid_params(format!("{name:?} is not a difficulty"))
            })?)
        }
        "game_mode" => {
            let name = text(value, "value")?;
            AdminSetting::GameMode(game_type_by_name(&name).ok_or_else(|| {
                AdminError::invalid_params(format!("{name:?} is not a game mode"))
            })?)
        }
        "enforce_allowlist" => AdminSetting::EnforceAllowlist(boolean(value, "value")?),
        "use_allowlist" => AdminSetting::UseAllowlist(boolean(value, "value")?),
        "max_players" => AdminSetting::MaxPlayers(bounded_integer(value)?),
        "motd" => AdminSetting::Motd(text(value, "value")?),
        "spawn_protection_radius" => AdminSetting::SpawnProtectionRadius(bounded_integer(value)?),
        "operator_user_permission_level" => {
            let level = integer(value, "value")?;
            AdminSetting::OperatorUserPermissionLevel(permission_level_by_id(level).ok_or_else(
                || AdminError::invalid_params(format!("{level} is not a permission level")),
            )?)
        }
        "view_distance" => AdminSetting::ViewDistance(bounded_integer(value)?),
        "simulation_distance" => AdminSetting::SimulationDistance(bounded_integer(value)?),
        "hide_online_players" => AdminSetting::HideOnlinePlayers(boolean(value, "value")?),
        "status_replies" => AdminSetting::StatusReplies(boolean(value, "value")?),
        "accept_transfers" => AdminSetting::AcceptTransfers(boolean(value, "value")?),
        "allow_flight" => AdminSetting::AllowFlight(boolean(value, "value")?),
        "pause_when_empty_seconds" => AdminSetting::PauseWhenEmptySeconds(bounded_integer(value)?),
        "entity_broadcast_range" => AdminSetting::EntityBroadcastRange(bounded_integer(value)?),
        _ => {
            return Err(AdminError::invalid_params(format!(
                "{key:?} is not a setting this server owns"
            )))
        }
    };
    Ok(setting)
}

/// One game rule value, in the type the named rule declares. An unknown rule is
/// refused here rather than at the tick, so a typo never reaches the world.
fn game_rule_value(key: &str, value: &Value) -> Result<GameRuleValue, AdminError> {
    let entry = vanilla_game_rule(key)
        .ok_or_else(|| AdminError::invalid_params(format!("{key:?} is not a game rule")))?;
    match entry.value_kind {
        GameRuleValueKind::Boolean => Ok(GameRuleValue::Boolean(boolean(value, "value")?)),
        GameRuleValueKind::Integer { .. } => Ok(GameRuleValue::Integer(bounded_integer(value)?)),
    }
}

fn arguments<'a>(value: &'a Value, allowed: &[&str]) -> Result<&'a Map<String, Value>, AdminError> {
    let arguments = match value {
        Value::Null => return Ok(EMPTY_ARGUMENTS.get_or_init(Map::new)),
        other => object(other, "arguments")?,
    };
    reject_unknown(arguments, allowed, "arguments")?;
    Ok(arguments)
}

static EMPTY_ARGUMENTS: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();

/// An argument this tool does not declare is refused rather than ignored: a
/// silently dropped field is how a management client believes it wrote
/// something it did not.
fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    owner: &str,
) -> Result<(), AdminError> {
    for name in object.keys() {
        if !allowed.contains(&name.as_str()) {
            return Err(AdminError::invalid_params(format!(
                "{owner} has no {name:?} field"
            )));
        }
    }
    Ok(())
}

fn object<'a>(value: &'a Value, name: &str) -> Result<&'a Map<String, Value>, AdminError> {
    value
        .as_object()
        .ok_or_else(|| AdminError::invalid_params(format!("{name} is an object")))
}

fn field<'a>(arguments: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    arguments.get(name).filter(|value| !value.is_null())
}

fn required<'a>(arguments: &'a Map<String, Value>, name: &str) -> Result<&'a Value, AdminError> {
    field(arguments, name).ok_or_else(|| AdminError::invalid_params(format!("{name} is required")))
}

fn text(value: &Value, name: &str) -> Result<String, AdminError> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AdminError::invalid_params(format!("{name} is a string")))
}

fn optional_text(arguments: &Map<String, Value>, name: &str) -> Result<Option<String>, AdminError> {
    match field(arguments, name) {
        Some(value) => text(value, name).map(Some),
        None => Ok(None),
    }
}

fn boolean(value: &Value, name: &str) -> Result<bool, AdminError> {
    value
        .as_bool()
        .ok_or_else(|| AdminError::invalid_params(format!("{name} is a boolean")))
}

fn integer(value: &Value, name: &str) -> Result<i64, AdminError> {
    value
        .as_i64()
        .ok_or_else(|| AdminError::invalid_params(format!("{name} is an integer")))
}

fn bounded_integer(value: &Value) -> Result<i32, AdminError> {
    i32::try_from(integer(value, "value")?)
        .map_err(|_| AdminError::invalid_params("value is out of range for a 32-bit integer"))
}

fn coordinate(arguments: &Map<String, Value>, name: &str) -> Result<i32, AdminError> {
    i32::try_from(integer(required(arguments, name)?, name)?).map_err(|_| {
        AdminError::invalid_params(format!("{name} is out of range for a 32-bit integer"))
    })
}

fn optional_instant(arguments: &Map<String, Value>, name: &str) -> Result<Option<i64>, AdminError> {
    match field(arguments, name) {
        Some(value) => {
            let text = text(value, name)?;
            parse_instant_iso8601(&text)
                .map(Some)
                .ok_or_else(|| AdminError::invalid_params(format!("{name} is not an instant")))
        }
        None => Ok(None),
    }
}

/// `Message` (Message.java:9). Both fields are optional on the wire; the world
/// resolves which one wins.
fn message(value: &Value) -> Result<AdminMessage, AdminError> {
    let object = object(value, "message")?;
    reject_unknown(
        object,
        &["literal", "translatable", "translatableParams"],
        "message",
    )?;
    let translatable_params = match field(object, "translatableParams") {
        Some(value) => Some(entries(value, "translatableParams", |entry| {
            text(entry, "translatableParams")
        })?),
        None => None,
    };
    Ok(AdminMessage {
        literal: optional_text(object, "literal")?,
        translatable: optional_text(object, "translatable")?,
        translatable_params,
    })
}

fn player(value: &Value) -> Result<PlayerRef, AdminError> {
    let object = object(value, "player")?;
    reject_unknown(object, &["id", "name"], "player")?;
    let id = match field(object, "id") {
        Some(value) => {
            let text = text(value, "id")?;
            Some(
                Uuid::parse_str(&text)
                    .map_err(|_| AdminError::invalid_params(format!("{text:?} is not a UUID")))?,
            )
        }
        None => None,
    };
    let name = optional_text(object, "name")?.map(Into::into);
    if id.is_none() && name.is_none() {
        return Err(AdminError::invalid_params(
            "a player is named by id, by name, or by both",
        ));
    }
    Ok(PlayerRef { id, name })
}

fn player_list(value: &Value, name: &str) -> Result<Vec<PlayerRef>, AdminError> {
    entries(value, name, player)
}

fn entries<T>(
    value: &Value,
    name: &str,
    mut each: impl FnMut(&Value) -> Result<T, AdminError>,
) -> Result<Vec<T>, AdminError> {
    let array = value
        .as_array()
        .ok_or_else(|| AdminError::invalid_params(format!("{name} is an array")))?;
    if array.len() > MAX_REQUEST_ENTRIES {
        return Err(AdminError::invalid_params(format!(
            "{name} carries at most {MAX_REQUEST_ENTRIES} entries"
        )));
    }
    array.iter().map(&mut each).collect()
}
