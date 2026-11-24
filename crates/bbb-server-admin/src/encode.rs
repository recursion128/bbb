//! Administration answers as MCP structured content.
//!
//! Vanilla owners: the `response(...)` declarations in `IncomingRpcMethods`
//! and the DTO records in `net.minecraft.server.jsonrpc.methods`. Tools that
//! project a vanilla method use its response field names.
//!
//! Instants use `ExtraCodecs.INSTANT_ISO8601`, the codec vanilla's own `expires`
//! fields are written with — not the `BanListEntry.DATE_FORMAT` the stored
//! documents on disk use.

use bbb_protocol::{game_rules::GameRuleValue, RichTextComponent};
use bbb_server_core::admin::{
    format_instant_iso8601, AdminResult, AdminSetting, BanRecord, DiagnosticsReport,
    GameRuleReport, IpBanReport, OperatorReport, PlayerRef, ServerStateReport, UserBanReport,
};
use serde_json::{json, Map, Value};

/// `PlayerDto` (PlayerDto.java:13).
pub fn player_value(player: &PlayerRef) -> Value {
    let mut object = Map::new();
    if let Some(id) = player.id {
        object.insert("id".to_string(), Value::from(id.to_string()));
    }
    if let Some(name) = &player.name {
        object.insert("name".to_string(), Value::from(name.to_display_string()));
    }
    Value::Object(object)
}

/// One `Component`, as both the flattened text a human reads and the canonical
/// value the client would resolve. Vanilla sends translation keys rather than
/// rendered English, so the canonical form travels with the summary instead of
/// being replaced by it.
pub fn component_value(component: &RichTextComponent) -> Value {
    json!({
        "text": component.text(),
        "component": bbb_protocol::to_display_json_value(component.component()).unwrap_or(Value::Null),
    })
}

pub fn game_rule_value(value: GameRuleValue) -> Value {
    match value {
        GameRuleValue::Boolean(value) => Value::from(value),
        GameRuleValue::Integer(value) => Value::from(value),
    }
}

/// `GameRuleType.getSerializedName` (GameRuleType.java:6-7).
pub fn game_rule_type(value: GameRuleValue) -> &'static str {
    match value {
        GameRuleValue::Boolean(_) => "boolean",
        GameRuleValue::Integer(_) => "integer",
    }
}

/// `Schema.TYPED_GAME_RULE_SCHEMA` (Schema.java:82-88).
pub fn typed_game_rule_value(report: &GameRuleReport) -> Value {
    json!({
        "key": report.key,
        "value": game_rule_value(report.value),
        "type": game_rule_type(report.value),
    })
}

/// The value half of one setting, in the JSON type its vanilla
/// `serversettings/<key>` method answers with.
pub fn setting_value(setting: &AdminSetting) -> Value {
    match setting {
        AdminSetting::Difficulty(value) => Value::from(value.serialized_name()),
        AdminSetting::GameMode(value) => Value::from(value.serialized_name()),
        AdminSetting::EnforceAllowlist(value)
        | AdminSetting::UseAllowlist(value)
        | AdminSetting::HideOnlinePlayers(value)
        | AdminSetting::StatusReplies(value)
        | AdminSetting::AcceptTransfers(value)
        | AdminSetting::AllowFlight(value) => Value::from(*value),
        AdminSetting::MaxPlayers(value)
        | AdminSetting::SpawnProtectionRadius(value)
        | AdminSetting::ViewDistance(value)
        | AdminSetting::SimulationDistance(value)
        | AdminSetting::PauseWhenEmptySeconds(value)
        | AdminSetting::EntityBroadcastRange(value) => Value::from(*value),
        AdminSetting::Motd(value) => Value::from(value.clone()),
        AdminSetting::OperatorUserPermissionLevel(value) => Value::from(value.id()),
    }
}

pub fn setting_entry(setting: &AdminSetting) -> Value {
    json!({ "key": setting.key(), "value": setting_value(setting) })
}

fn instant_value(epoch_millis: Option<i64>) -> Value {
    epoch_millis.map_or(Value::Null, |instant| {
        Value::from(format_instant_iso8601(instant))
    })
}

fn ban_fields(record: &BanRecord, object: &mut Map<String, Value>) {
    object.insert(
        "reason".to_string(),
        record.reason.as_ref().map_or(Value::Null, |reason| {
            Value::String(reason.to_display_string())
        }),
    );
    object.insert(
        "source".to_string(),
        Value::String(record.source.to_display_string()),
    );
    object.insert("expires".to_string(), instant_value(record.expires));
    object.insert("created".to_string(), instant_value(Some(record.created)));
}

/// `BanlistService.UserBanDto` (BanlistService.java:113), plus the `created`
/// instant the stored entry already carries.
pub fn user_ban_value(report: &UserBanReport) -> Value {
    let mut object = Map::new();
    object.insert("player".to_string(), player_value(&report.player));
    ban_fields(&report.ban, &mut object);
    Value::Object(object)
}

/// `IpBanlistService.IpBanDto` (IpBanlistService.java:126).
pub fn ip_ban_value(report: &IpBanReport) -> Value {
    let mut object = Map::new();
    object.insert("ip".to_string(), Value::from(report.ip.clone()));
    ban_fields(&report.ban, &mut object);
    Value::Object(object)
}

/// `OperatorService.OperatorDto` (OperatorService.java:90).
pub fn operator_value(report: &OperatorReport) -> Value {
    json!({
        "player": player_value(&report.player),
        "permissionLevel": report.permission_level.id(),
        "bypassesPlayerLimit": report.bypasses_player_limit,
    })
}

/// `ServerStateService.ServerState` (ServerStateService.java:64).
pub fn server_state_value(report: &ServerStateReport) -> Value {
    json!({
        "started": report.started,
        "players": report.players.iter().map(player_value).collect::<Vec<_>>(),
        "version": { "name": report.version_name, "protocol": report.version_protocol },
    })
}

/// The bounded runtime readout. Vanilla has no equivalent response; these are
/// the counters `MinecraftServer` already keeps for its own console.
pub fn diagnostics_value(report: &DiagnosticsReport) -> Value {
    json!({
        "tick": report.tick,
        "onlinePlayers": report.online_players,
        "maxPlayers": report.max_players,
        "loadedChunks": report.loaded_chunks,
        "changedBlocks": report.changed_blocks,
        "allowlistEntries": report.allowlist_entries,
        "operatorEntries": report.operator_entries,
        "playerBanEntries": report.player_ban_entries,
        "ipBanEntries": report.ip_ban_entries,
        "processedAdminOperations": report.processed_admin_operations,
        "stopping": report.stopping,
    })
}

/// One answer as the structured content of a tool result.
pub fn result_value(result: &AdminResult) -> Value {
    match result {
        AdminResult::Status(report) => json!({ "status": server_state_value(report) }),
        AdminResult::Players(players) => {
            json!({ "players": players.iter().map(player_value).collect::<Vec<_>>() })
        }
        AdminResult::Allowlist(players) => {
            json!({ "allowlist": players.iter().map(player_value).collect::<Vec<_>>() })
        }
        AdminResult::Bans(bans) => {
            json!({ "banlist": bans.iter().map(user_ban_value).collect::<Vec<_>>() })
        }
        AdminResult::IpBans(bans) => {
            json!({ "banlist": bans.iter().map(ip_ban_value).collect::<Vec<_>>() })
        }
        AdminResult::Operators(operators) => {
            json!({ "operators": operators.iter().map(operator_value).collect::<Vec<_>>() })
        }
        AdminResult::Settings(settings) => {
            json!({ "settings": settings.iter().map(setting_entry).collect::<Vec<_>>() })
        }
        AdminResult::Setting(setting) => json!({ "setting": setting_entry(setting) }),
        AdminResult::GameRules(rules) => {
            json!({ "gamerules": rules.iter().map(typed_game_rule_value).collect::<Vec<_>>() })
        }
        AdminResult::GameRule(rule) => json!({ "gamerule": typed_game_rule_value(rule) }),
        AdminResult::Diagnostics(report) => json!({ "diagnostics": diagnostics_value(report) }),
        AdminResult::WorldInfo(report) => {
            let dimensions = report
                .dimensions
                .iter()
                .map(|dimension| {
                    json!({
                        "id": dimension.id,
                        "minY": dimension.min_y,
                        "maxY": dimension.max_y,
                        "loadedChunks": dimension.loaded_chunks,
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "dimensions": dimensions,
                "spawn": {
                    "dimension": report.spawn.dimension,
                    "x": report.spawn.position.x,
                    "y": report.spawn.position.y,
                    "z": report.spawn.position.z,
                },
            })
        }
        AdminResult::WorldBlocks(report) => {
            let blocks = report
                .blocks
                .iter()
                .map(|block| {
                    json!({
                        "x": block.position.x,
                        "y": block.position.y,
                        "z": block.position.z,
                        "state": block.state,
                    })
                })
                .collect::<Vec<_>>();
            json!({ "dimension": report.dimension, "blocks": blocks })
        }
        AdminResult::WorldSurface(report) => {
            let columns = report
                .columns
                .iter()
                .map(|column| {
                    json!({
                        "x": column.x,
                        "z": column.z,
                        "loaded": column.loaded,
                        "topY": column.top_y,
                    })
                })
                .collect::<Vec<_>>();
            json!({ "dimension": report.dimension, "columns": columns })
        }
        AdminResult::Kicked(players) => {
            json!({ "kicked": players.iter().map(player_value).collect::<Vec<_>>() })
        }
        AdminResult::Saving(saving) => json!({ "saving": saving }),
        AdminResult::Stopping(stopping) => json!({ "stopping": stopping }),
        AdminResult::Sent(sent) => json!({ "sent": sent }),
        AdminResult::Command(report) => json!({
            "command": {
                "result": report.result.map_or(Value::Null, Value::from),
                "messages": report.messages.iter().map(component_value).collect::<Vec<_>>(),
                "broadcast": report.broadcast.as_ref().map_or(Value::Null, component_value),
            },
        }),
    }
}

/// The same answer as the text content an MCP client without structured-output
/// support still has to be able to read.
pub fn result_text(result: &AdminResult) -> String {
    serde_json::to_string_pretty(&result_value(result)).unwrap_or_else(|_| "{}".to_string())
}
