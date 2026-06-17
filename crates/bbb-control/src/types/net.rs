use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetCounters {
    pub connected: bool,
    pub state: Option<String>,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    #[serde(default)]
    pub unsupported_packets: usize,
    #[serde(default)]
    pub last_unsupported_packet_state: Option<String>,
    #[serde(default)]
    pub last_unsupported_packet_id: Option<i32>,
    #[serde(default)]
    pub last_unsupported_packet_len: Option<usize>,
    pub held_slot_commands_queued: usize,
    pub player_action_commands_queued: usize,
    pub player_command_commands_queued: usize,
    #[serde(default)]
    pub player_abilities_commands_queued: usize,
    pub player_input_commands_queued: usize,
    #[serde(default)]
    pub chat_command_commands_queued: usize,
    #[serde(default)]
    pub change_difficulty_commands_queued: usize,
    #[serde(default)]
    pub lock_difficulty_commands_queued: usize,
    #[serde(default)]
    pub attack_entity_commands_queued: usize,
    #[serde(default)]
    pub interact_entity_commands_queued: usize,
    pub swing_commands_queued: usize,
    pub use_item_on_commands_queued: usize,
    pub use_item_commands_queued: usize,
    pub pick_item_from_block_commands_queued: usize,
    #[serde(default)]
    pub pick_item_from_entity_commands_queued: usize,
    #[serde(default)]
    pub place_recipe_commands_queued: usize,
    #[serde(default)]
    pub recipe_book_change_settings_commands_queued: usize,
    #[serde(default)]
    pub recipe_book_seen_recipe_commands_queued: usize,
    #[serde(default)]
    pub rename_item_commands_queued: usize,
    #[serde(default)]
    pub edit_book_commands_queued: usize,
    #[serde(default)]
    pub advancements_seen_commands_queued: usize,
    #[serde(default)]
    pub select_trade_commands_queued: usize,
    #[serde(default)]
    pub set_beacon_commands_queued: usize,
    #[serde(default)]
    pub sign_update_commands_queued: usize,
    #[serde(default)]
    pub spectate_entity_commands_queued: usize,
    #[serde(default)]
    pub teleport_to_entity_commands_queued: usize,
    #[serde(default)]
    pub select_bundle_item_commands_queued: usize,
    pub command_suggestion_commands_queued: usize,
    #[serde(default)]
    pub block_entity_tag_query_commands_queued: usize,
    #[serde(default)]
    pub entity_tag_query_commands_queued: usize,
    #[serde(default)]
    pub container_close_commands_queued: usize,
    #[serde(default)]
    pub container_button_click_commands_queued: usize,
    #[serde(default)]
    pub container_click_commands_queued: usize,
    #[serde(default)]
    pub container_slot_state_changed_commands_queued: usize,
    pub player_move_commands_queued: usize,
    pub move_vehicle_commands_queued: usize,
    pub last_error: Option<String>,
}
