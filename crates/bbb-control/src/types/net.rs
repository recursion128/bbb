use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One producer partition in the native simulation command mailbox.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationCommandMailboxClassCounters {
    pub command_limit: usize,
    pub retained_byte_limit: u64,
    pub pending: usize,
    pub peak_pending: usize,
    pub retained_bytes: u64,
    pub peak_retained_bytes: u64,
    pub blocked_submissions: u64,
}

/// Bounded command-mailbox observation published with the simulation
/// generation that consumed it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationCommandMailboxCounters {
    pub command_limit: usize,
    pub retained_byte_limit: u64,
    pub pending: usize,
    pub peak_pending: usize,
    pub retained_bytes: u64,
    pub peak_retained_bytes: u64,
    pub blocked_submissions: u64,
    pub coalesced_submissions: u64,
    pub oversized_rejections: u64,
    pub closed_rejections: u64,
    pub platform_input: SimulationCommandMailboxClassCounters,
    pub control: SimulationCommandMailboxClassCounters,
    pub render_feedback: SimulationCommandMailboxClassCounters,
    pub async_completion: SimulationCommandMailboxClassCounters,
    pub lifecycle: SimulationCommandMailboxClassCounters,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientResourcePackDiagnostic {
    pub priority: usize,
    pub id: String,
    pub source: String,
    pub compatibility: String,
    pub required: bool,
    pub fixed_position: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientResourceDiagnostics {
    pub generation: u64,
    pub selected_packs: Vec<ClientResourcePackDiagnostic>,
    /// Mounted external Vanilla asset-index identity and lazy provider inventory.
    #[serde(default)]
    pub indexed_asset_index_id: Option<String>,
    #[serde(default)]
    pub indexed_asset_object_count: usize,
    #[serde(default)]
    pub indexed_asset_exposed_resource_count: usize,
    #[serde(default)]
    pub indexed_asset_root_resource_count: usize,
    #[serde(default)]
    pub indexed_asset_retained_logical_bytes: u64,
    /// Selected language and one consumer-resolved translation from this generation.
    #[serde(default)]
    pub selected_language: Option<String>,
    #[serde(default)]
    pub selected_language_probe_key: Option<String>,
    #[serde(default)]
    pub selected_language_probe_value: Option<String>,
    /// Direction selected from the committed client pack language metadata.
    /// This is the reload-generation value consumed by renderer text caches.
    #[serde(default)]
    pub language_bidirectional: bool,
    #[serde(default)]
    pub image_rgba_limit_bytes: u64,
    #[serde(default)]
    pub image_rgba_charged_bytes: u64,
    #[serde(default)]
    pub image_rgba_peak_charged_bytes: u64,
    #[serde(default)]
    pub data_logical_limit_bytes: u64,
    #[serde(default)]
    pub data_logical_charged_bytes: u64,
    #[serde(default)]
    pub data_logical_peak_charged_bytes: u64,
    #[serde(default)]
    pub data_json_source_bytes: u64,
    #[serde(default)]
    pub data_json_documents: u64,
    #[serde(default)]
    pub data_json_nodes: u64,
    #[serde(default)]
    pub data_json_string_bytes: u64,
    #[serde(default)]
    pub data_listing_entries: u64,
    #[serde(default)]
    pub data_derived_logical_bytes: u64,
    #[serde(default)]
    pub data_rejections: u64,
    pub last_reload_generation: Option<u64>,
    pub last_reload_failure_stage: Option<String>,
    pub last_reload_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetCounters {
    pub connected: bool,
    /// Monotonic identity of the current network connection. Disconnect keeps
    /// the last identity while `connected` distinguishes its terminal edge.
    #[serde(default)]
    pub connection_generation: u64,
    #[serde(default, with = "client_resource_diagnostics_arc")]
    pub client_resources: Arc<ClientResourceDiagnostics>,
    pub state: Option<String>,
    /// Common-listener remote disconnects that requested Vanilla's unfiltered
    /// SoundManager stop before the terminal resource-pack reload.
    #[serde(default)]
    pub simulation_remote_disconnect_audio_stop_requests: u64,
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
    /// `Options.broadcastOptions` resends after a `save`.
    #[serde(default)]
    pub broadcast_client_information_commands_queued: usize,
    #[serde(default)]
    pub request_game_rule_values_commands_queued: usize,
    #[serde(default)]
    pub set_game_rule_commands_queued: usize,
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
    pub set_command_block_commands_queued: usize,
    #[serde(default)]
    pub set_command_minecart_commands_queued: usize,
    #[serde(default)]
    pub set_structure_block_commands_queued: usize,
    #[serde(default)]
    pub set_jigsaw_block_commands_queued: usize,
    #[serde(default)]
    pub jigsaw_generate_commands_queued: usize,
    #[serde(default)]
    pub set_test_block_commands_queued: usize,
    #[serde(default)]
    pub test_instance_block_action_commands_queued: usize,
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
    /// Count/retained-byte budgets and pressure for commands entering the
    /// single simulation owner.
    #[serde(default)]
    pub simulation_command_mailbox: SimulationCommandMailboxCounters,
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
    /// Immutable world/ECS inputs prepared for the most recent entity tick.
    #[serde(default)]
    pub simulation_entity_client_tick_preparation_nanos: u64,
    #[serde(default)]
    pub simulation_entity_client_tick_preparation_peak_nanos: u64,
    /// Root/passenger schedule construction for the most recent entity tick.
    #[serde(default)]
    pub simulation_entity_client_tick_schedule_nanos: u64,
    #[serde(default)]
    pub simulation_entity_client_tick_schedule_peak_nanos: u64,
    /// Projectile parent-lifecycle work for the most recent entity tick.
    #[serde(default)]
    pub simulation_entity_client_tick_projectile_nanos: u64,
    #[serde(default)]
    pub simulation_entity_client_tick_projectile_peak_nanos: u64,
    /// Transform/interpolation work for the most recent entity tick.
    #[serde(default)]
    pub simulation_entity_client_tick_interpolation_nanos: u64,
    #[serde(default)]
    pub simulation_entity_client_tick_interpolation_peak_nanos: u64,
    /// Swept terrain intersection and ordered block callbacks for the most
    /// recent entity client-tick batch.
    #[serde(default)]
    pub simulation_entity_client_tick_block_effect_nanos: u64,
    #[serde(default)]
    pub simulation_entity_client_tick_block_effect_peak_nanos: u64,
    /// Entity callback invocations, distinct terrain cells, and callbacks
    /// applied by the most recent swept block-effect pass.
    #[serde(default)]
    pub simulation_entity_client_tick_block_effect_entities: usize,
    #[serde(default)]
    pub simulation_entity_client_tick_block_effect_blocks_visited: usize,
    #[serde(default)]
    pub simulation_entity_client_tick_block_effect_callbacks_applied: usize,
    /// Common and family animation work for the most recent entity tick.
    #[serde(default)]
    pub simulation_entity_client_tick_animation_nanos: u64,
    #[serde(default)]
    pub simulation_entity_client_tick_animation_peak_nanos: u64,
    /// Tracked entity rows considered for the most recent client-tick batch.
    #[serde(default)]
    pub simulation_entity_client_tick_scanned_entities: usize,
    /// Ordered root/passenger entities whose animation component changed in
    /// that batch.
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

mod client_resource_diagnostics_arc {
    use super::*;

    pub(super) fn serialize<S>(
        value: &Arc<ClientResourceDiagnostics>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_ref().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Arc<ClientResourceDiagnostics>, D::Error>
    where
        D: Deserializer<'de>,
    {
        ClientResourceDiagnostics::deserialize(deserializer).map(Arc::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_diagnostics_clone_shares_owner_and_preserves_json_shape() {
        let mut counters = NetCounters::default();
        counters.client_resources = Arc::new(ClientResourceDiagnostics {
            generation: 17,
            language_bidirectional: true,
            selected_packs: vec![ClientResourcePackDiagnostic {
                priority: 0,
                id: "vanilla".to_string(),
                source: "vanilla".to_string(),
                compatibility: "compatible".to_string(),
                required: true,
                fixed_position: false,
            }],
            ..ClientResourceDiagnostics::default()
        });

        let cloned = counters.clone();

        assert!(Arc::ptr_eq(
            &counters.client_resources,
            &cloned.client_resources
        ));
        let json = serde_json::to_value(cloned).unwrap();
        assert_eq!(json["client_resources"]["generation"], 17);
        assert_eq!(
            json["client_resources"]["selected_packs"][0]["id"],
            "vanilla"
        );
        assert_eq!(json["client_resources"]["language_bidirectional"], true);
    }

    #[test]
    fn net_counters_only_expose_connection_runtime_and_queue_projections() {
        let source = include_str!("net.rs");
        let fields = source
            .split_once("pub struct NetCounters {")
            .expect("NetCounters declaration is present")
            .1
            .split_once('}')
            .expect("NetCounters body is closed")
            .0
            .lines()
            .filter_map(|line| {
                let field = line.trim().strip_prefix("pub ").unwrap_or(line.trim());
                let (name, _) = field.split_once(':')?;
                name.chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric())
                    .then_some(name)
            })
            .collect::<std::collections::BTreeSet<_>>();
        let json = serde_json::to_value(NetCounters::default()).unwrap();
        let serialized_fields = json
            .as_object()
            .expect("NetCounters serializes as an object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(fields.len() > 80, "counter guard must not pass vacuously");
        assert_eq!(
            fields, serialized_fields,
            "every NetCounters field must remain an explicit control observation"
        );

        for field in fields {
            let connection_or_runtime = matches!(
                field,
                "connected"
                    | "connection_generation"
                    | "client_resources"
                    | "state"
                    | "compression_threshold"
                    | "packets_seen"
                    | "unsupported_packets"
                    | "last_unsupported_packet_state"
                    | "last_unsupported_packet_id"
                    | "last_unsupported_packet_len"
                    | "last_error"
            );
            assert!(
                connection_or_runtime
                    || field.ends_with("_commands_queued")
                    || field.starts_with("simulation_"),
                "NetCounters field {field:?} is outside connection/runtime status and command-queue projections"
            );
        }
    }

    #[test]
    fn resource_memory_diagnostics_default_for_older_payloads() {
        let diagnostics: ClientResourceDiagnostics = serde_json::from_value(serde_json::json!({
            "generation": 3,
            "selected_packs": [],
            "last_reload_generation": null,
            "last_reload_failure_stage": null,
            "last_reload_error": null
        }))
        .unwrap();

        assert_eq!(diagnostics.image_rgba_limit_bytes, 0);
        assert_eq!(diagnostics.image_rgba_charged_bytes, 0);
        assert_eq!(diagnostics.image_rgba_peak_charged_bytes, 0);
        assert_eq!(diagnostics.data_logical_limit_bytes, 0);
        assert_eq!(diagnostics.data_logical_charged_bytes, 0);
        assert_eq!(diagnostics.data_logical_peak_charged_bytes, 0);
        assert_eq!(diagnostics.data_json_source_bytes, 0);
        assert_eq!(diagnostics.data_json_documents, 0);
        assert_eq!(diagnostics.data_json_nodes, 0);
        assert_eq!(diagnostics.data_json_string_bytes, 0);
        assert_eq!(diagnostics.data_listing_entries, 0);
        assert_eq!(diagnostics.data_derived_logical_bytes, 0);
        assert_eq!(diagnostics.data_rejections, 0);
    }

    #[test]
    fn publication_build_observation_defaults_for_older_counter_payloads() {
        let mut value = serde_json::to_value(NetCounters::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("connection_generation");
        object.remove("client_resources");
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
        object.remove("simulation_command_mailbox");
        object.remove("simulation_publication_build_peak_nanos");
        object.remove("simulation_entity_client_tick_nanos");
        object.remove("simulation_entity_client_tick_peak_nanos");
        object.remove("simulation_entity_client_tick_preparation_nanos");
        object.remove("simulation_entity_client_tick_preparation_peak_nanos");
        object.remove("simulation_entity_client_tick_schedule_nanos");
        object.remove("simulation_entity_client_tick_schedule_peak_nanos");
        object.remove("simulation_entity_client_tick_projectile_nanos");
        object.remove("simulation_entity_client_tick_projectile_peak_nanos");
        object.remove("simulation_entity_client_tick_interpolation_nanos");
        object.remove("simulation_entity_client_tick_interpolation_peak_nanos");
        object.remove("simulation_entity_client_tick_block_effect_nanos");
        object.remove("simulation_entity_client_tick_block_effect_peak_nanos");
        object.remove("simulation_entity_client_tick_block_effect_entities");
        object.remove("simulation_entity_client_tick_block_effect_blocks_visited");
        object.remove("simulation_entity_client_tick_block_effect_callbacks_applied");
        object.remove("simulation_entity_client_tick_animation_nanos");
        object.remove("simulation_entity_client_tick_animation_peak_nanos");
        object.remove("simulation_entity_client_tick_scanned_entities");
        object.remove("simulation_entity_client_tick_mutated_entities");
        object.remove("simulation_entity_snapshot_build_nanos");
        object.remove("simulation_entity_snapshot_build_peak_nanos");
        object.remove("simulation_entity_snapshot_rebuilt_entities");

        let counters: NetCounters = serde_json::from_value(value).unwrap();
        assert_eq!(counters.connection_generation, 0);
        assert_eq!(
            counters.client_resources.as_ref(),
            &ClientResourceDiagnostics::default()
        );
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
        assert_eq!(
            counters.simulation_command_mailbox,
            SimulationCommandMailboxCounters::default()
        );
        assert_eq!(counters.simulation_publication_build_peak_nanos, 0);
        assert_eq!(counters.simulation_entity_client_tick_nanos, 0);
        assert_eq!(counters.simulation_entity_client_tick_peak_nanos, 0);
        assert_eq!(counters.simulation_entity_client_tick_preparation_nanos, 0);
        assert_eq!(
            counters.simulation_entity_client_tick_preparation_peak_nanos,
            0
        );
        assert_eq!(counters.simulation_entity_client_tick_schedule_nanos, 0);
        assert_eq!(
            counters.simulation_entity_client_tick_schedule_peak_nanos,
            0
        );
        assert_eq!(counters.simulation_entity_client_tick_projectile_nanos, 0);
        assert_eq!(
            counters.simulation_entity_client_tick_projectile_peak_nanos,
            0
        );
        assert_eq!(
            counters.simulation_entity_client_tick_interpolation_nanos,
            0
        );
        assert_eq!(
            counters.simulation_entity_client_tick_interpolation_peak_nanos,
            0
        );
        assert_eq!(counters.simulation_entity_client_tick_block_effect_nanos, 0);
        assert_eq!(
            counters.simulation_entity_client_tick_block_effect_peak_nanos,
            0
        );
        assert_eq!(
            counters.simulation_entity_client_tick_block_effect_entities,
            0
        );
        assert_eq!(
            counters.simulation_entity_client_tick_block_effect_blocks_visited,
            0
        );
        assert_eq!(
            counters.simulation_entity_client_tick_block_effect_callbacks_applied,
            0
        );
        assert_eq!(counters.simulation_entity_client_tick_animation_nanos, 0);
        assert_eq!(
            counters.simulation_entity_client_tick_animation_peak_nanos,
            0
        );
        assert_eq!(counters.simulation_entity_client_tick_scanned_entities, 0);
        assert_eq!(counters.simulation_entity_client_tick_mutated_entities, 0);
        assert_eq!(counters.simulation_entity_snapshot_build_nanos, 0);
        assert_eq!(counters.simulation_entity_snapshot_build_peak_nanos, 0);
        assert_eq!(counters.simulation_entity_snapshot_rebuilt_entities, 0);
    }

    #[test]
    fn editor_packet_counters_default_for_older_payloads() {
        let mut value = serde_json::to_value(NetCounters::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("set_command_block_commands_queued");
        object.remove("set_command_minecart_commands_queued");
        object.remove("set_structure_block_commands_queued");
        object.remove("set_jigsaw_block_commands_queued");
        object.remove("jigsaw_generate_commands_queued");
        object.remove("set_test_block_commands_queued");
        object.remove("test_instance_block_action_commands_queued");

        let counters: NetCounters = serde_json::from_value(value).unwrap();
        assert_eq!(counters.set_command_block_commands_queued, 0);
        assert_eq!(counters.set_command_minecart_commands_queued, 0);
        assert_eq!(counters.set_structure_block_commands_queued, 0);
        assert_eq!(counters.set_jigsaw_block_commands_queued, 0);
        assert_eq!(counters.jigsaw_generate_commands_queued, 0);
        assert_eq!(counters.set_test_block_commands_queued, 0);
        assert_eq!(counters.test_instance_block_action_commands_queued, 0);
    }
}
