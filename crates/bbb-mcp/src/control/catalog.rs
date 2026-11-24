//! Client MCP tool and resource catalog shared by `bbb-client` and remote
//! gateways (`bbb-web-host`) without renderer or world crate dependencies.

use serde_json::{json, Map, Value};

use crate::host::{ResourceDescriptor, ToolDescriptor};

/// Maximum UTF-16 code units accepted by `input.text` (`InputControlRequest::MAX_TEXT_UTF16_UNITS`).
pub const MAX_INPUT_TEXT_UTF16_UNITS: usize = 4_096;
/// Maximum ticks accepted by timed `hold_ticks` on `input.key` / `input.mouse_button`.
pub const MAX_HOLD_TICKS: u32 = 100;
/// Maximum ticks accepted by `sim.wait_ticks` (stays well within the 10s MCP request timeout).
pub const MAX_WAIT_TICKS: u32 = 100;
/// Maximum positions accepted in one `world.probe_blocks` call.
pub const MAX_WORLD_PROBE_BLOCKS: usize = 256;
/// Maximum columns accepted in one `world.probe_surface` rectangle.
pub const MAX_WORLD_SURFACE_COLUMNS: usize = 256;
/// Maximum entities returned by one `world.nearby_entities` call.
pub const MAX_NEARBY_ENTITIES: usize = 256;

/// Semantic key names accepted by `input.key` (`InputKeyControl::NAMES`).
pub const INPUT_KEY_NAMES: &[&str] = &[
    "forward",
    "backward",
    "left",
    "right",
    "jump",
    "sneak",
    "sprint",
    "hotbar1",
    "hotbar2",
    "hotbar3",
    "hotbar4",
    "hotbar5",
    "hotbar6",
    "hotbar7",
    "hotbar8",
    "hotbar9",
    "inventory",
    "chat",
    "command",
    "enter",
    "escape",
    "up",
    "down",
    "page_up",
    "page_down",
    "backspace",
    "f1",
    "f3",
    "f4",
    "f5",
    "f6",
    "f9",
    "tab",
    "drop",
    "swap_offhand",
    "advancements",
    "social",
];

/// One client control method projected as an MCP tool (and optional resource).
pub struct ClientTool {
    pub name: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub read_only: bool,
    /// Whether applying this tool can end something the player has: the
    /// process, a container, an item. A confirmation hint, never authority.
    pub destructive: bool,
    /// The URI this readout is also served under, for readouts that take no argument.
    pub resource_uri: Option<&'static str>,
    pub schema: fn() -> Value,
}

impl ClientTool {
    pub fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: self.name,
            title: self.title,
            summary: self.summary,
            input_schema: (self.schema)(),
            read_only: self.read_only,
            destructive: self.destructive,
            meta: Some(json!({ "bbb/controlMethod": self.name })),
        }
    }

    pub fn resource(&self) -> Option<ResourceDescriptor> {
        self.resource_uri.map(|uri| ResourceDescriptor {
            uri,
            name: self.name,
            title: self.title,
            summary: self.summary,
        })
    }
}

pub fn catalog() -> &'static [ClientTool] {
    TOOLS
}

pub fn tool_by_name(name: &str) -> Option<&'static ClientTool> {
    TOOLS.iter().find(|tool| tool.name == name)
}

pub fn resources() -> Vec<ResourceDescriptor> {
    TOOLS.iter().filter_map(ClientTool::resource).collect()
}

pub fn tool_by_resource_uri(uri: &str) -> Option<&'static ClientTool> {
    TOOLS.iter().find(|tool| tool.resource_uri == Some(uri))
}

macro_rules! readout {
    ($name:literal, $uri:literal, $title:literal, $summary:literal) => {
        ClientTool {
            name: $name,
            title: $title,
            summary: $summary,
            read_only: true,
            destructive: false,
            resource_uri: Some($uri),
            schema: no_arguments,
        }
    };
}

macro_rules! write {
    ($name:literal, $title:literal, $summary:literal, $schema:expr) => {
        ClientTool {
            name: $name,
            title: $title,
            summary: $summary,
            read_only: false,
            destructive: false,
            resource_uri: None,
            schema: $schema,
        }
    };
}

static TOOLS: &[ClientTool] = &[
    ClientTool {
        name: "app.quit",
        title: "Quit",
        summary: "Asks the client to shut down.",
        read_only: false,
        destructive: true,
        resource_uri: None,
        schema: no_arguments,
    },
    write!(
        "renderer.screenshot",
        "Screenshot",
        "Captures the next presented frame, saves it and returns the completed image.",
        || object(&[("path", non_empty_string(), true)])
    ),
    write!(
        "renderer.recording_start",
        "Start video recording",
        "Records rendered frames to H.264 MP4 on Windows. Video only; defaults to 30 fps and 8 Mbps.",
        || object(&[
            ("path", non_empty_string(), true),
            ("fps", json!({"type":"integer", "minimum":1, "maximum":120}), false),
            ("bitrate", json!({"type":"integer", "minimum":1, "maximum":4_294_967_295_u64}), false),
        ])
    ),
    write!(
        "renderer.recording_stop",
        "Stop video recording",
        "Drains accepted video frames and finalizes the MP4 before returning the saved recording status.",
        no_arguments
    ),
    readout!(
        "renderer.recording_status",
        "bbb://client/renderer/recording_status",
        "Video recording status",
        "Current recording, accepted/dropped frames and the saved path or encoder error."
    ),
    write!(
        "net.accept_code_of_conduct",
        "Accept code of conduct",
        "Accepts the server's code of conduct, optionally remembering the choice.",
        || object(&[("remember", boolean(), false)])
    ),
    write!(
        "net.decline_code_of_conduct",
        "Decline code of conduct",
        "Declines the server's code of conduct.",
        no_arguments
    ),
    write!(
        "net.clear_code_of_conduct_acceptance",
        "Clear code of conduct acceptance",
        "Forgets a remembered code of conduct acceptance.",
        no_arguments
    ),
    write!(
        "net.send_chat_message",
        "Send chat message",
        "Sends one chat message.",
        || object(&[("message", non_empty_string(), true)])
    ),
    write!(
        "net.send_chat_command",
        "Send chat command",
        "Sends one command without its leading slash.",
        || object(&[("command", non_empty_string(), true)])
    ),
    write!(
        "net.request_command_suggestions",
        "Request command suggestions",
        "Asks the server to complete a partial command.",
        || object(&[
            ("id", integer(), true),
            ("command", non_empty_string(), true)
        ])
    ),
    write!(
        "net.query_block_entity_tag",
        "Query block entity tag",
        "Asks the server for a block entity's NBT.",
        || object(&[
            ("transaction_id", integer(), true),
            ("x", integer(), true),
            ("y", integer(), true),
            ("z", integer(), true),
        ])
    ),
    write!(
        "net.query_entity_tag",
        "Query entity tag",
        "Asks the server for an entity's NBT.",
        || object(&[
            ("transaction_id", integer(), true),
            ("entity_id", integer(), true)
        ])
    ),
    write!(
        "net.spectate_entity",
        "Spectate entity",
        "Spectates an entity by id.",
        || object(&[("entity_id", integer(), true)])
    ),
    write!(
        "net.teleport_to_entity",
        "Teleport to entity",
        "Teleports the spectating player to an entity by uuid.",
        || object(&[("uuid", string_with("format", json!("uuid")), true)])
    ),
    write!(
        "net.change_difficulty",
        "Change difficulty",
        "Asks the server to change the difficulty.",
        || object(&[(
            "difficulty",
            enumeration(&["peaceful", "easy", "normal", "hard"]),
            true,
        )])
    ),
    write!(
        "net.change_game_mode",
        "Change game mode",
        "Asks the server to change the player's game mode.",
        || object(&[(
            "game_mode",
            enumeration(&["survival", "creative", "adventure", "spectator"]),
            true,
        )])
    ),
    write!(
        "net.lock_difficulty",
        "Lock difficulty",
        "Locks or unlocks the difficulty.",
        || object(&[("locked", boolean(), true)])
    ),
    write!(
        "net.set_held_slot",
        "Set held slot",
        "Selects a hotbar slot.",
        || object(&[("slot", integer_range(0, 8), true)])
    ),
    write!(
        "net.set_flying",
        "Set flying",
        "Starts or stops flying.",
        || object(&[("flying", boolean(), true)])
    ),
    write!(
        "input.focus",
        "Focus",
        "Gives or takes the window's input focus; set capture_cursor=true with focused=true to enter first-person gameplay control.",
        || object(&[
            ("focused", boolean(), true),
            ("capture_cursor", boolean(), false),
        ])
    ),
    write!(
        "input.key",
        "Key",
        "Presses or releases one semantic key. When pressed=true, optional hold_ticks automatically releases the key after that many client ticks (in real-time sessions the call waits until auto-release completes; in --lockstep sessions it returns after pressing).",
        || object(&[
            ("key", enumeration(INPUT_KEY_NAMES), true),
            ("pressed", boolean(), true),
            ("hold_ticks", integer_range(1, i64::from(MAX_HOLD_TICKS)), false),
        ])
    ),
    write!(
        "input.text",
        "Text",
        "Types text into whatever owns the keyboard.",
        || object(&[(
            "text",
            {
                let mut schema = string_with("maxLength", json!(MAX_INPUT_TEXT_UTF16_UNITS));
                schema["description"] = json!(format!(
                    "At most {} UTF-16 code units; supplementary characters count as two units.",
                    MAX_INPUT_TEXT_UTF16_UNITS
                ));
                schema
            },
            true,
        )])
    ),
    write!(
        "input.mouse_button",
        "Mouse button",
        "Presses or releases one mouse button. When pressed=true, optional hold_ticks automatically releases the button after that many client ticks (in real-time sessions the call waits until auto-release completes; in --lockstep sessions it returns after pressing).",
        || object(&[
            ("button", enumeration(&["left", "right", "middle"]), true),
            ("pressed", boolean(), true),
            ("hold_ticks", integer_range(1, i64::from(MAX_HOLD_TICKS)), false),
        ])
    ),
    write!(
        "input.mouse_tap",
        "Mouse tap",
        "Presses and releases one mouse button.",
        || object(&[("button", enumeration(&["left", "right", "middle"]), true)])
    ),
    write!(
        "input.cursor",
        "Cursor move",
        "Moves the cursor to window pixel coordinates.",
        || object(&[("x", number(), true), ("y", number(), true)])
    ),
    write!(
        "input.click",
        "Click",
        "Moves the cursor to window pixel coordinates and clicks one mouse button there.",
        || object(&[
            ("x", number(), true),
            ("y", number(), true),
            ("button", enumeration(&["left", "right", "middle"]), true)
        ])
    ),
    write!(
        "input.scroll",
        "Mouse wheel scroll",
        "Scrolls the mouse wheel by line deltas (dy > 0 scrolls up, dy < 0 scrolls down).",
        || object(&[("dy", number(), true), ("dx", number(), false)])
    ),
    write!(
        "input.look_delta",
        "Look delta",
        "Moves the look direction by a mouse delta.",
        || object(&[("dx", number(), true), ("dy", number(), true)])
    ),
    write!(
        "input.look",
        "Look angles",
        "Sets the local player's absolute yaw (y_rot) and pitch (x_rot in -90..=90) while gameplay cursor capture is active.",
        || object(&[
            ("yaw", number(), true),
            ("pitch", json!({ "type": "number", "minimum": -90, "maximum": 90 }), true),
        ])
    ),
    write!(
        "input.look_at",
        "Look at target",
        "Aims the local player's eyes at world coordinates (x, y, z) or a loaded entity_id (anchor: center, eyes, or feet) while gameplay cursor capture is active.",
        || {
            let mut schema = object(&[
                ("x", number(), false),
                ("y", number(), false),
                ("z", number(), false),
                ("entity_id", integer(), false),
                ("anchor", enumeration(&["center", "eyes", "feet"]), false),
            ]);
            schema["oneOf"] = json!([
                {
                    "required": ["x", "y", "z"],
                    "not": { "required": ["entity_id"] }
                },
                {
                    "required": ["entity_id"],
                    "not": {
                        "anyOf": [
                            { "required": ["x"] },
                            { "required": ["y"] },
                            { "required": ["z"] }
                        ]
                    }
                }
            ]);
            schema
        }
    ),
    write!(
        "net.perform_respawn",
        "Respawn",
        "Respawns after death.",
        no_arguments
    ),
    write!(
        "net.request_stats",
        "Request stats",
        "Asks the server for the player's statistics.",
        no_arguments
    ),
    write!(
        "net.request_game_rule_values",
        "Request game rule values",
        "Asks the server for the game rule values.",
        no_arguments
    ),
    write!(
        "net.place_recipe",
        "Place recipe",
        "Places a recipe from the recipe book into the open crafting container.",
        || object(&[
            ("container_id", integer(), true),
            ("recipe_index", integer(), true),
            ("use_max_items", boolean(), true),
        ])
    ),
    write!(
        "net.change_recipe_book_settings",
        "Change recipe book settings",
        "Opens, closes or filters one recipe book.",
        || object(&[
            (
                "book_type",
                enumeration(&["crafting", "furnace", "blast_furnace", "smoker"]),
                true,
            ),
            ("open", boolean(), true),
            ("filtering", boolean(), true),
        ])
    ),
    write!(
        "net.mark_recipe_seen",
        "Mark recipe seen",
        "Marks a recipe as seen.",
        || object(&[("recipe_index", integer(), true)])
    ),
    write!(
        "net.rename_item",
        "Rename item",
        "Renames the item in the open anvil.",
        || object(&[("name", string_with("maxLength", json!(32767)), true)])
    ),
    write!(
        "net.edit_book",
        "Edit book",
        "Writes a book's pages, and signs it when a title is given.",
        || object(&[
            ("slot", integer(), true),
            (
                "pages",
                array_of(string_with("maxLength", json!(1024)), Some(100)),
                true,
            ),
            (
                "title",
                nullable(string_with("maxLength", json!(32))),
                false
            ),
        ])
    ),
    write!(
        "net.update_sign",
        "Update sign",
        "Writes the four lines of one sign face.",
        || object(&[
            ("x", integer(), true),
            ("y", integer(), true),
            ("z", integer(), true),
            ("is_front_text", boolean(), true),
            (
                "lines",
                fixed_array_of(string_with("maxLength", json!(384)), 4),
                true
            ),
        ])
    ),
    write!(
        "net.open_advancements_tab",
        "Open advancements tab",
        "Opens one advancements tab by resource location.",
        || object(&[(
            "tab",
            string_with("pattern", json!("^([a-z0-9_.-]+:)?[a-z0-9_./-]+$")),
            true
        )])
    ),
    write!(
        "net.close_advancements_screen",
        "Close advancements screen",
        "Closes the advancements screen.",
        no_arguments
    ),
    write!(
        "net.select_trade",
        "Select trade",
        "Selects a merchant trade by index.",
        || object(&[("item", integer_min(0), true)])
    ),
    write!(
        "net.set_beacon",
        "Set beacon",
        "Sets the open beacon's effects by mob effect id.",
        || object(&[
            ("primary_effect", nullable(integer()), false),
            ("secondary_effect", nullable(integer()), false),
        ])
    ),
    write!(
        "net.set_creative_mode_slot",
        "Set creative mode slot",
        "Puts an item stack into an inventory slot in creative mode.",
        || object(&[
            ("slot_num", integer(), true),
            ("item", creative_item_stack(), false),
        ])
    ),
    write!(
        "net.select_bundle_item",
        "Select bundle item",
        "Selects an item inside a bundle, or -1 to select none.",
        || object(&[
            ("slot_id", integer(), true),
            ("selected_item_index", integer_min(-1), true),
        ])
    ),
    write!(
        "net.container_button_click",
        "Container button click",
        "Clicks a button of the open container.",
        || object(&[
            ("container_id", integer(), true),
            ("button_id", integer(), true)
        ])
    ),
    write!(
        "net.container_click",
        "Container click",
        "Sends one raw container click with its predicted slot changes.",
        || object(&[
            ("container_id", integer(), true),
            ("state_id", integer(), true),
            ("slot_num", integer(), true),
            ("button_num", integer(), true),
            ("input", container_input(), true),
            (
                "changed_slots",
                array_of(
                    object(&[("slot", integer(), true), ("stack", hashed_stack(), true)]),
                    None,
                ),
                false,
            ),
            ("carried_item", hashed_stack(), false),
        ])
    ),
    write!(
        "net.container_click_slot",
        "Container click slot",
        "Clicks a slot of the open container or opened player inventory screen (press input.key('inventory') first when no container is open), letting the client predict the change. button_num: 0=left-click, 1=right-click (split/place-one), 0..8=hotbar slot for swap. Player inventory_menu slots: 0=craft result, 1..4=2x2 grid, 5..8=armor, 9..35=main, 36..44=hotbar, 45=offhand.",
        || object(&[
            ("slot_num", integer(), true),
            ("button_num", integer(), true),
            ("input", container_input(), true),
        ])
    ),
    ClientTool {
        name: "net.container_close",
        title: "Container close",
        summary: "Closes the open container.",
        read_only: false,
        destructive: true,
        resource_uri: None,
        schema: || object(&[("container_id", integer(), true)]),
    },
    write!(
        "net.container_slot_state_changed",
        "Container slot state changed",
        "Toggles a crafter slot.",
        || object(&[
            ("slot_id", integer(), true),
            ("container_id", integer(), true),
            ("new_state", boolean(), true),
        ])
    ),
    readout!(
        "sim.lockstep_status",
        "bbb://client/sim/lockstep_status",
        "Lockstep status",
        "Replay barrier counters of a --lockstep client: frames read and written, quiescence."
    ),
    write!(
        "sim.step",
        "Lockstep step",
        "Advances a --lockstep client's clock by whole ticks.",
        || object(&[
            ("ticks", integer_min(1), false),
            ("tick_nanos", integer_min(1), false)
        ])
    ),
    write!(
        "sim.wait_ticks",
        "Wait client ticks",
        "Waits for the given number of real-time client ticks (1..=100, 20 ticks/sec) before returning a fresh generation.",
        || object(&[("ticks", integer_range(1, i64::from(MAX_WAIT_TICKS)), true)])
    ),
    readout!(
        "app.status",
        "bbb://client/app/status",
        "App status",
        "The application, network, audio, renderer and world counters at once."
    ),
    readout!(
        "net.counters",
        "bbb://client/net/counters",
        "Network counters",
        "The connection state and packet counters."
    ),
    readout!(
        "audio.counters",
        "bbb://client/audio/counters",
        "Audio counters",
        "The audio runtime counters."
    ),
    readout!(
        "renderer.counters",
        "bbb://client/renderer/counters",
        "Renderer counters",
        "The renderer counters."
    ),
    readout!(
        "world.counters",
        "bbb://client/world/counters",
        "World counters",
        "The client world's counters."
    ),
    readout!(
        "world.client_local_player",
        "bbb://client/world/client_local_player",
        "Local player",
        "The local player's state."
    ),
    ClientTool {
        name: "world.probe_block",
        title: "Probe block",
        summary: "The block at one position, or null when it is not loaded.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || {
            object(&[
                ("x", integer(), true),
                ("y", integer(), true),
                ("z", integer(), true),
            ])
        },
    },
    ClientTool {
        name: "world.probe_blocks",
        title: "Probe blocks",
        summary: "Reads up to 256 client-loaded block states in request order. Set skip_air=true to omit loaded air blocks for compact results.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || {
            let mut positions = array_of(
                object(&[
                    ("x", integer(), true),
                    ("y", integer(), true),
                    ("z", integer(), true),
                ]),
                Some(MAX_WORLD_PROBE_BLOCKS),
            );
            positions["minItems"] = json!(1);
            object(&[("positions", positions, true), ("skip_air", boolean(), false)])
        },
    },
    ClientTool {
        name: "world.probe_surface",
        title: "Probe surface",
        summary: "Highest non-air block (including fluids) for an inclusive rectangle of up to 256 columns.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || {
            object(&[
                ("min_x", integer(), true),
                ("min_z", integer(), true),
                ("max_x", integer(), true),
                ("max_z", integer(), true),
                ("max_y", integer(), false),
            ])
        },
    },
    readout!(
        "world.crosshair_target",
        "bbb://client/world/crosshair_target",
        "Crosshair target",
        "The block (with face and adjacent placement coordinate) or entity currently under the gameplay crosshair."
    ),
    readout!(
        "world.apply_diagnostics",
        "bbb://client/world/apply_diagnostics",
        "Apply diagnostics",
        "How the world applied what the server sent."
    ),
    readout!(
        "world.registries",
        "bbb://client/world/registries",
        "Registries",
        "The registries the server synchronized."
    ),
    readout!(
        "world.level_state",
        "bbb://client/world/level_state",
        "Level state",
        "The dimension, level info and gameplay state."
    ),
    readout!(
        "world.client_advancements",
        "bbb://client/world/client_advancements",
        "Advancements",
        "The client's advancements."
    ),
    readout!(
        "world.client_audio",
        "bbb://client/world/client_audio",
        "Client audio",
        "The sounds the world asked to play."
    ),
    readout!(
        "world.client_chat",
        "bbb://client/world/client_chat",
        "Chat",
        "The chat state."
    ),
    readout!(
        "world.client_combat",
        "bbb://client/world/client_combat",
        "Combat",
        "The combat state."
    ),
    readout!(
        "world.client_cooldowns",
        "bbb://client/world/client_cooldowns",
        "Cooldowns",
        "The item cooldowns."
    ),
    readout!(
        "world.client_effects",
        "bbb://client/world/client_effects",
        "Effects",
        "Recent client explosion and level particle effects (use world.probe_entity_status for mob effects)."
    ),
    readout!(
        "world.client_command_suggestions",
        "bbb://client/world/client_command_suggestions",
        "Command suggestions",
        "The last command suggestions received."
    ),
    readout!(
        "world.client_features",
        "bbb://client/world/client_features",
        "Features",
        "The enabled feature flags."
    ),
    readout!(
        "world.client_known_packs",
        "bbb://client/world/client_known_packs",
        "Known packs",
        "The known data packs."
    ),
    readout!(
        "world.client_debug_query",
        "bbb://client/world/client_debug_query",
        "Debug query",
        "The last debug query answers."
    ),
    readout!(
        "world.client_debug_game",
        "bbb://client/world/client_debug_game",
        "Debug game",
        "The game debug state."
    ),
    readout!(
        "world.client_hud",
        "bbb://client/world/client_hud",
        "HUD",
        "The HUD state."
    ),
    readout!(
        "world.client_inventory",
        "bbb://client/world/client_inventory",
        "Inventory",
        "The player's inventory and the open container."
    ),
    readout!(
        "world.client_player_info",
        "bbb://client/world/client_player_info",
        "Player info",
        "The player list."
    ),
    readout!(
        "world.client_recipe_book",
        "bbb://client/world/client_recipe_book",
        "Recipe book",
        "The recipe book state."
    ),
    readout!(
        "world.client_recipes",
        "bbb://client/world/client_recipes",
        "Recipes",
        "The recipes the server synchronized."
    ),
    readout!(
        "world.client_scoreboard",
        "bbb://client/world/client_scoreboard",
        "Scoreboard",
        "The scoreboard state."
    ),
    readout!(
        "world.client_stats",
        "bbb://client/world/client_stats",
        "Stats",
        "The player's statistics."
    ),
    readout!(
        "world.client_waypoints",
        "bbb://client/world/client_waypoints",
        "Waypoints",
        "The tracked waypoints."
    ),
    readout!(
        "world.client_ui",
        "bbb://client/world/client_ui",
        "UI",
        "The open screens and dialogs."
    ),
    readout!(
        "world.client_maps",
        "bbb://client/world/client_maps",
        "Maps",
        "The map items."
    ),
    readout!(
        "world.last_map_color_patch",
        "bbb://client/world/last_map_color_patch",
        "Last map color patch",
        "The last map color patch received."
    ),
    readout!(
        "world.command_tree",
        "bbb://client/world/command_tree",
        "Command tree",
        "The command tree the server sent."
    ),
    readout!(
        "world.last_block_changed_ack",
        "bbb://client/world/last_block_changed_ack",
        "Last block changed ack",
        "The last block change acknowledgement."
    ),
    readout!(
        "world.client_block_events",
        "bbb://client/world/client_block_events",
        "Block events",
        "Block destructions, block events and level events."
    ),
    readout!(
        "world.world_border",
        "bbb://client/world/world_border",
        "World border",
        "The world border."
    ),
    readout!(
        "world.level_clock",
        "bbb://client/world/level_clock",
        "Level clock",
        "The world time, weather and ticking state."
    ),
    readout!(
        "world.chunk_view",
        "bbb://client/world/chunk_view",
        "Chunk view",
        "The chunk cache center and radius."
    ),
    readout!(
        "world.server_presentation",
        "bbb://client/world/server_presentation",
        "Server presentation",
        "Titles, tab list and other server presentation."
    ),
    ClientTool {
        name: "world.probe_chunk",
        title: "Probe chunk",
        summary: "A summary of one chunk, or null when it is not loaded.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || object(&[("x", integer(), true), ("z", integer(), true)]),
    },
    ClientTool {
        name: "world.probe_entity",
        title: "Probe entity",
        summary: "One entity's state by id, or null.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || object(&[("id", integer(), true)]),
    },
    ClientTool {
        name: "world.probe_entity_status",
        title: "Probe entity status",
        summary: "One entity's status by id, or null.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || object(&[("id", integer(), true)]),
    },
    ClientTool {
        name: "world.probe_entity_transform",
        title: "Probe entity transform",
        summary: "One entity's transform by id, or null.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || object(&[("id", integer(), true)]),
    },
    readout!(
        "world.entity_transforms",
        "bbb://client/world/entity_transforms",
        "Entity transforms",
        "Every entity's transform."
    ),
    readout!(
        "world.last_projectile_power",
        "bbb://client/world/last_projectile_power",
        "Last projectile power",
        "The last projectile power update."
    ),
    ClientTool {
        name: "world.entity_pick_targets",
        title: "Entity pick targets",
        summary: "Every pickable entity's bounding box at a partial tick.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || object(&[("partial_tick", number(), false)]),
    },
    ClientTool {
        name: "world.nearby_entities",
        title: "Nearby entities",
        summary: "Entities near the local player sorted by distance, including entity type names and dropped item stacks.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || {
            object(&[
                ("max_distance", json!({ "type": "number", "minimum": 0 }), false),
                ("limit", integer_range(1, MAX_NEARBY_ENTITIES as i64), false),
                ("include_local_player", boolean(), false),
            ])
        },
    },
    ClientTool {
        name: "world.terrain_chunk_summary",
        title: "Terrain chunk summary",
        summary: "A terrain summary of one chunk, or null when it is not loaded.",
        read_only: true,
        destructive: false,
        resource_uri: None,
        schema: || object(&[("x", integer(), true), ("z", integer(), true)]),
    },
];

fn no_arguments() -> Value {
    object(&[])
}

fn object(properties: &[(&str, Value, bool)]) -> Value {
    let mut schema = Map::new();
    let mut required = Vec::new();
    for (name, property, is_required) in properties {
        schema.insert((*name).to_string(), property.clone());
        if *is_required {
            required.push(Value::from(*name));
        }
    }
    let mut value = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": Value::Object(schema),
    });
    if !required.is_empty() {
        value["required"] = Value::Array(required);
    }
    value
}

fn integer() -> Value {
    json!({ "type": "integer" })
}

fn integer_min(minimum: i64) -> Value {
    json!({ "type": "integer", "minimum": minimum })
}

fn integer_range(minimum: i64, maximum: i64) -> Value {
    json!({ "type": "integer", "minimum": minimum, "maximum": maximum })
}

fn number() -> Value {
    json!({ "type": "number" })
}

fn boolean() -> Value {
    json!({ "type": "boolean" })
}

fn string() -> Value {
    json!({ "type": "string" })
}

fn non_empty_string() -> Value {
    string_with("minLength", json!(1))
}

fn string_with(keyword: &str, constraint: Value) -> Value {
    let mut schema = string();
    schema[keyword] = constraint;
    schema
}

fn enumeration(values: &[&str]) -> Value {
    json!({ "type": "string", "enum": values })
}

fn nullable(schema: Value) -> Value {
    json!({ "anyOf": [schema, { "type": "null" }] })
}

fn array_of(items: Value, max_items: Option<usize>) -> Value {
    let mut schema = json!({ "type": "array", "items": items });
    if let Some(max_items) = max_items {
        schema["maxItems"] = json!(max_items);
    }
    schema
}

fn fixed_array_of(items: Value, count: usize) -> Value {
    json!({ "type": "array", "items": items, "minItems": count, "maxItems": count })
}

fn container_input() -> Value {
    enumeration(&[
        "pickup",
        "quick_move",
        "swap",
        "clone",
        "throw",
        "quick_craft",
        "pickup_all",
    ])
}

fn creative_item_stack() -> Value {
    json!({
        "oneOf": [
            object(&[("kind", json!({ "const": "empty" }), true)]),
            object(&[
                ("kind", json!({ "const": "item" }), true),
                ("item_id", integer_min(0), true),
                ("count", integer_min(1), true),
            ]),
        ],
    })
}

fn hashed_stack() -> Value {
    json!({
        "oneOf": [
            object(&[("kind", json!({ "const": "empty" }), true)]),
            object(&[
                ("kind", json!({ "const": "item" }), true),
                ("item_id", integer(), true),
                ("count", integer(), true),
                (
                    "components",
                    json!({
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "added_components": {
                                "type": "object",
                                "propertyNames": { "pattern": "^-?[0-9]+$" },
                                "additionalProperties": integer(),
                            },
                            "removed_components": array_of(integer(), None),
                        },
                    }),
                    false,
                ),
            ]),
        ],
    })
}
