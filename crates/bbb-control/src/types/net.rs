use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetCounters {
    pub connected: bool,
    /// Monotonic identity of the current network connection. Disconnect keeps
    /// the last identity while `connected` distinguishes its terminal edge.
    #[serde(default)]
    pub connection_generation: u64,
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
    #[serde(default)]
    pub perform_respawn_commands_queued: usize,
    #[serde(default)]
    pub request_stats_commands_queued: usize,
    #[serde(default)]
    pub request_game_rule_values_commands_queued: usize,
    pub player_input_commands_queued: usize,
    #[serde(default)]
    pub chat_command_commands_queued: usize,
    #[serde(default)]
    pub chat_message_commands_queued: usize,
    #[serde(default)]
    pub chat_acknowledgement_commands_queued: usize,
    #[serde(default)]
    pub custom_click_action_commands_queued: usize,
    #[serde(default)]
    pub change_difficulty_commands_queued: usize,
    #[serde(default)]
    pub change_game_mode_commands_queued: usize,
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
    pub set_creative_mode_slot_commands_queued: usize,
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
    #[serde(default)]
    pub paddle_boat_commands_queued: usize,
    pub move_vehicle_commands_queued: usize,
    /// Wall-clock cost of building the most recently published immutable
    /// simulation generation, including its world snapshot, in nanoseconds.
    #[serde(default)]
    pub simulation_publication_build_nanos: u64,
    /// Scheduler invocations that applied tick, network, or command work.
    #[serde(default)]
    pub simulation_productive_steps: u64,
    /// Producer wakes that found no command, network event, or due tick and
    /// therefore emitted no publication/output.
    #[serde(default)]
    pub simulation_noop_steps: u64,
    /// Immutable simulation generations published after startup.
    #[serde(default)]
    pub simulation_publications: u64,
    /// Published generations that rebuilt the immutable world snapshot.
    #[serde(default)]
    pub simulation_world_snapshot_rebuilds: u64,
    /// Published generations that reused the previous exact world Arc.
    #[serde(default)]
    pub simulation_world_snapshot_reuses: u64,
    /// Non-empty effect batches inserted into the lossless render FIFO.
    #[serde(default)]
    pub simulation_effect_batches: u64,
    /// Productive steps whose empty effect collectors were suppressed.
    #[serde(default)]
    pub simulation_empty_effect_batches_suppressed: u64,
    /// Event-loop wakes emitted for simulation outputs as of the latest
    /// render-owned telemetry sample.
    #[serde(default)]
    pub simulation_render_wakes_sent: u64,
    /// Simulation output wakes coalesced behind an already-pending wake.
    #[serde(default)]
    pub simulation_render_wakes_coalesced: u64,
    /// Current queued simulation outputs at the latest render sample.
    #[serde(default)]
    pub simulation_output_backlog_current: u64,
    /// Peak queued simulation outputs observed by the render owner.
    #[serde(default)]
    pub simulation_output_backlog_peak: u64,
    /// Largest immutable simulation publication build observed in this
    /// process, in nanoseconds.
    #[serde(default)]
    pub simulation_publication_build_peak_nanos: u64,
    /// Wall-clock cost of the most recent non-empty entity client-tick batch,
    /// excluding immutable publication construction, in nanoseconds.
    #[serde(default)]
    pub simulation_entity_client_tick_nanos: u64,
    /// Largest non-empty entity client-tick batch observed in this process, in
    /// nanoseconds.
    #[serde(default)]
    pub simulation_entity_client_tick_peak_nanos: u64,
    /// Entity rows visited by the main animation query in the most recent
    /// non-empty client-tick batch.
    #[serde(default)]
    pub simulation_entity_client_tick_scanned_entities: usize,
    /// Entity rows whose animation component changed in that batch.
    #[serde(default)]
    pub simulation_entity_client_tick_mutated_entities: usize,
    /// Wall-clock cost of rebuilding the entity ECS shell for the most recent
    /// immutable simulation generation, in nanoseconds.
    #[serde(default)]
    pub simulation_entity_snapshot_build_nanos: u64,
    /// Largest entity ECS shell rebuild observed in this process, in
    /// nanoseconds.
    #[serde(default)]
    pub simulation_entity_snapshot_build_peak_nanos: u64,
    /// Entity shells rebuilt for the most recent immutable generation.
    #[serde(default)]
    pub simulation_entity_snapshot_rebuilt_entities: usize,
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_build_observation_defaults_for_older_counter_payloads() {
        let mut value = serde_json::to_value(NetCounters::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("connection_generation");
        object.remove("simulation_publication_build_nanos");
        object.remove("simulation_productive_steps");
        object.remove("simulation_noop_steps");
        object.remove("simulation_publications");
        object.remove("simulation_world_snapshot_rebuilds");
        object.remove("simulation_world_snapshot_reuses");
        object.remove("simulation_effect_batches");
        object.remove("simulation_empty_effect_batches_suppressed");
        object.remove("simulation_render_wakes_sent");
        object.remove("simulation_render_wakes_coalesced");
        object.remove("simulation_output_backlog_current");
        object.remove("simulation_output_backlog_peak");
        object.remove("simulation_publication_build_peak_nanos");
        object.remove("simulation_entity_client_tick_nanos");
        object.remove("simulation_entity_client_tick_peak_nanos");
        object.remove("simulation_entity_client_tick_scanned_entities");
        object.remove("simulation_entity_client_tick_mutated_entities");
        object.remove("simulation_entity_snapshot_build_nanos");
        object.remove("simulation_entity_snapshot_build_peak_nanos");
        object.remove("simulation_entity_snapshot_rebuilt_entities");

        let counters: NetCounters = serde_json::from_value(value).unwrap();
        assert_eq!(counters.connection_generation, 0);
        assert_eq!(counters.simulation_publication_build_nanos, 0);
        assert_eq!(counters.simulation_productive_steps, 0);
        assert_eq!(counters.simulation_noop_steps, 0);
        assert_eq!(counters.simulation_publications, 0);
        assert_eq!(counters.simulation_world_snapshot_rebuilds, 0);
        assert_eq!(counters.simulation_world_snapshot_reuses, 0);
        assert_eq!(counters.simulation_effect_batches, 0);
        assert_eq!(counters.simulation_empty_effect_batches_suppressed, 0);
        assert_eq!(counters.simulation_render_wakes_sent, 0);
        assert_eq!(counters.simulation_render_wakes_coalesced, 0);
        assert_eq!(counters.simulation_output_backlog_current, 0);
        assert_eq!(counters.simulation_output_backlog_peak, 0);
        assert_eq!(counters.simulation_publication_build_peak_nanos, 0);
        assert_eq!(counters.simulation_entity_client_tick_nanos, 0);
        assert_eq!(counters.simulation_entity_client_tick_peak_nanos, 0);
        assert_eq!(counters.simulation_entity_client_tick_scanned_entities, 0);
        assert_eq!(counters.simulation_entity_client_tick_mutated_entities, 0);
        assert_eq!(counters.simulation_entity_snapshot_build_nanos, 0);
        assert_eq!(counters.simulation_entity_snapshot_build_peak_nanos, 0);
        assert_eq!(counters.simulation_entity_snapshot_rebuilt_entities, 0);
    }
}
