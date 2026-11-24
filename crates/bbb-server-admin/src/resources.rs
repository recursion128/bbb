//! The MCP resource surface: subscribable administration answers, addressable by URI.
//!
//! Vanilla owner: `JsonRpcNotificationService`, which pushes player and server
//! lifecycle notifications to a connected management client. MCP has no
//! equivalent free-form notification; a client subscribes to a resource and the
//! server tells it when that resource changed. So the vanilla notification set
//! becomes a change classification over these URIs, and the client re-reads what
//! it cares about.
//!
//! Every resource is a projection of a `Read`-scope request. Reading one goes
//! through the same mailbox and the same tick as the equivalent tool call, so a
//! resource can never observe a half-ticked world.

use bbb_mcp::ResourceDescriptor;
use bbb_server_core::admin::AdminRequest;

pub struct AdminResource {
    pub uri: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    /// The request this resource is read with.
    pub request: fn() -> AdminRequest,
}

impl AdminResource {
    /// The `resources/list` entry, as the shared transport lists it.
    pub fn descriptor_entry(&self) -> ResourceDescriptor {
        ResourceDescriptor {
            uri: self.uri,
            name: self.name,
            title: self.title,
            summary: self.summary,
        }
    }

    /// The `resources/list` entry.
    pub fn descriptor(&self) -> serde_json::Value {
        self.descriptor_entry().to_value()
    }
}

pub fn resources() -> &'static [AdminResource] {
    RESOURCES
}

pub fn resource_by_uri(uri: &str) -> Option<&'static AdminResource> {
    RESOURCES.iter().find(|resource| resource.uri == uri)
}

static RESOURCES: &[AdminResource] = &[
    AdminResource {
        uri: "bbb://server/status",
        name: "server-status",
        title: "Server status",
        summary:
            "Whether the server is accepting players, who is on it, and which version it speaks.",
        request: || AdminRequest::Status,
    },
    AdminResource {
        uri: "bbb://server/players",
        name: "players",
        title: "Online players",
        summary: "Every player currently connected.",
        request: || AdminRequest::Players,
    },
    AdminResource {
        uri: "bbb://server/allowlist",
        name: "allowlist",
        title: "Allowlist",
        summary: "The players the allowlist admits.",
        request: || AdminRequest::Allowlist,
    },
    AdminResource {
        uri: "bbb://server/bans",
        name: "bans",
        title: "Player bans",
        summary: "The banned players.",
        request: || AdminRequest::Bans,
    },
    AdminResource {
        uri: "bbb://server/ip_bans",
        name: "ip-bans",
        title: "IP bans",
        summary: "The banned addresses.",
        request: || AdminRequest::IpBans,
    },
    AdminResource {
        uri: "bbb://server/operators",
        name: "operators",
        title: "Operators",
        summary: "The operators and their permission levels.",
        request: || AdminRequest::Operators,
    },
    AdminResource {
        uri: "bbb://server/settings",
        name: "server-settings",
        title: "Server settings",
        summary: "Every server setting this server owns.",
        request: || AdminRequest::Settings,
    },
    AdminResource {
        uri: "bbb://server/gamerules",
        name: "gamerules",
        title: "Game rules",
        summary: "Every enabled game rule and its value.",
        request: || AdminRequest::GameRules,
    },
    AdminResource {
        uri: "bbb://server/diagnostics",
        name: "diagnostics",
        title: "Runtime diagnostics",
        summary: "Bounded runtime counters.",
        request: || AdminRequest::Diagnostics,
    },
];

/// Which resources an applied change may have altered.
///
/// Derived from the action name rather than from a second hand-kept table: the
/// action is already the tool name, and its prefix is the surface it writes.
/// `command.execute` is deliberately treated as touching everything, because a
/// command can reach anything the typed surface can.
pub fn resources_changed_by(action: &str) -> Vec<&'static str> {
    let group = action.split('.').next().unwrap_or_default();
    let mut changed = match group {
        "allowlist" => vec!["bbb://server/allowlist"],
        "bans" => vec!["bbb://server/bans"],
        "ip_bans" => vec!["bbb://server/ip_bans"],
        "operators" => vec!["bbb://server/operators"],
        "serversettings" => vec!["bbb://server/settings"],
        "gamerules" => vec!["bbb://server/gamerules"],
        "players" => vec!["bbb://server/players", "bbb://server/status"],
        "server" => Vec::new(),
        "command" => RESOURCES
            .iter()
            .map(|resource| resource.uri)
            .filter(|uri| *uri != "bbb://server/diagnostics")
            .collect(),
        _ => Vec::new(),
    };
    // Every applied mutation advances the counters the diagnostics readout
    // reports, so it is the one resource nothing leaves alone.
    changed.push("bbb://server/diagnostics");
    changed
}

/// What a player joining or leaving changed. The world publishes this without
/// an administration request behind it, so it is named rather than derived.
pub fn resources_changed_by_player_list() -> [&'static str; 3] {
    [
        "bbb://server/players",
        "bbb://server/status",
        "bbb://server/diagnostics",
    ]
}
