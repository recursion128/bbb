//! Frozen simulation state for control and MCP queries.

use std::sync::Arc;

use crate::control::{
    AudioCounters, ControlSnapshot, ControlWorldSnapshot, RendererCounters, SharedSnapshot,
};

use super::SimulationCore;

/// The event loop owns publication. An input-only step can reuse the frozen
/// world while projecting the current UI.
pub(crate) fn resolve_base_client_ui(
    world: &bbb_client_world::WorldStore,
    rebuild_world: bool,
    cached_base_client_ui: &mut Option<serde_json::Value>,
) -> serde_json::Value {
    if rebuild_world || cached_base_client_ui.is_none() {
        let value = bbb_protocol::to_display_json_value(world.client_ui()).unwrap();
        *cached_base_client_ui = Some(value.clone());
        value
    } else {
        cached_base_client_ui
            .as_ref()
            .expect("cached client UI checked")
            .clone()
    }
}

pub(crate) fn publish(
    snapshot: &SharedSnapshot,
    core: &SimulationCore,
    rebuild_world: bool,
    renderer: &RendererCounters,
    mut audio: AudioCounters,
    cached_base_client_ui: &mut Option<serde_json::Value>,
) -> u64 {
    let recipe_book_search = core.input.recipe_book_search_receipt(&core.world);
    let input_runtime = core.input_runtime.publication();
    core.environment_audio.project_diagnostics(&mut audio);
    let (generation, world, mut app) = {
        let current = snapshot.load();
        let world = if rebuild_world || current.generation == 0 {
            Arc::new(core.world.publication_snapshot())
        } else {
            Arc::clone(&current.world_view.world)
        };
        (
            current.generation.saturating_add(1),
            world,
            current.app.clone(),
        )
    };
    let creative_inventory = core.input.creative_inventory_context().map(|operator_tab| {
        let catalog_generation = core
            .world
            .creative_inventory_snapshot()
            .map_or(0, |snapshot| snapshot.generation);
        serde_json::json!({
            "client_level_generation": core.world.client_level_generation(),
            "catalog_generation": catalog_generation,
            "display_operator_tab": operator_tab,
        })
    });
    let client_ui = core.input.dialog_control_projection(resolve_base_client_ui(
        &core.world,
        rebuild_world,
        cached_base_client_ui,
    ));
    app.running = core.running();
    app.cursor_gui_position = input_runtime
        .cursor_position
        .map(|position| [position.x, position.y]);
    app.gui_surface_width = input_runtime.gui_scaled_width;
    app.gui_surface_height = input_runtime.gui_scaled_height;
    app.cursor_captured = input_runtime.cursor_captured;
    app.input_focused = core.input.is_focused();
    app.mouse_button_events = input_runtime.mouse_button_events;
    app.left_mouse_button_events = input_runtime.left_mouse_button_events;
    app.left_mouse_button_press_events = input_runtime.left_mouse_button_press_events;
    app.inventory_mouse_button_events = input_runtime.inventory_mouse_button_events;
    app.inventory_mouse_button_consumed_events =
        input_runtime.inventory_mouse_button_consumed_events;
    app.inventory_recipe_book_button_hit_events =
        input_runtime.inventory_recipe_book_button_hit_events;
    app.keyboard_events = input_runtime.keyboard_events;
    app.keyboard_press_events = input_runtime.keyboard_press_events;
    app.key_t_events = input_runtime.key_t_events;
    app.inventory_keyboard_events = input_runtime.inventory_keyboard_events;
    app.inventory_key_t_press_events = input_runtime.inventory_key_t_press_events;
    (
        app.recipe_book_search_focused,
        app.recipe_book_search_text_char_count,
    ) = recipe_book_search;
    app.recipe_book_language_requests = input_runtime.recipe_book_language_requests;
    app.recipe_book_language_target_available = input_runtime.recipe_book_language_target_available;
    app.recipe_book_language_transitions = input_runtime.recipe_book_language_transitions;
    app.disconnected_screen = core.input.disconnected_screen_status();
    app.chat_screen = core
        .input
        .chat_screen_status(core.world.chat_presentation_config().visibility);
    app.death_screen = core.input.death_screen_status();
    snapshot.store(Arc::new(ControlSnapshot {
        generation,
        app,
        net: core.net_counters.clone(),
        audio,
        renderer: renderer.clone(),
        world_view: ControlWorldSnapshot {
            world,
            client_ui,
            creative_inventory,
        },
    }));
    generation
}

#[cfg(test)]
mod tests {
    use crate::control::{ChatScreenControlKind, NetCounters};
    use crate::input::ClientInputState;
    use bbb_client_world::WorldStore;
    use winit::{
        event::ElementState,
        keyboard::{KeyCode, PhysicalKey},
    };

    #[test]
    fn disconnected_screen_status_is_bounded() {
        let mut input = ClientInputState::default();
        input.open_disconnected_screen(
            crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost,
            crate::input::disconnected_screen::DisconnectedScreenDetails {
                reason: bbb_client_net::DisconnectReason::from_plain("x".repeat(300)),
                report_directory: None,
                bug_report_link: None,
            },
        );

        let status = input.disconnected_screen_status().unwrap();
        assert_eq!(status.title_translation_key, "disconnect.lost");
        assert_eq!(status.reason_preview.len(), 256);
        assert!(status.reason_truncated);
        assert_eq!(status.reason_retained_bytes, 300);
    }

    #[test]
    fn chat_screen_status_carries_its_world_permissions() {
        let mut world = WorldStore::new();
        let mut chat = world.chat_presentation_config().clone();
        chat.visibility = bbb_client_world::ChatVisibilityState::System;
        world.set_chat_presentation_config(chat);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        crate::input::handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyT),
            ElementState::Pressed,
        );
        crate::input::handle_text_input(&mut input, &mut counters, &mut world, &None, "draft 中");
        assert!(input.open_chat_restrictions_screen());

        let status = input
            .chat_screen_status(world.chat_presentation_config().visibility)
            .expect("chat status");
        assert_eq!(status.screen, ChatScreenControlKind::Restrictions);
        assert_eq!(status.input_text, "draft 中");
        assert_eq!(status.input_utf16_units, 7);
        assert!(!status.can_send_messages);
        assert!(status.can_send_commands);
    }

    #[test]
    fn lazy_control_publication_skips_when_idle_and_rate_limits_active_polling() {
        let demand = crate::control::ControlPublicationDemand::default();
        let start = bbb_client_clock::Instant::now();

        assert!(!crate::session_shared::should_publish_control_snapshot(
            &demand, false, None, start,
        ));

        demand.record_demand();
        assert!(crate::session_shared::should_publish_control_snapshot(
            &demand, false, None, start,
        ));
        assert!(!crate::session_shared::should_publish_control_snapshot(
            &demand,
            false,
            Some(start),
            start + std::time::Duration::from_millis(10),
        ));
        assert!(crate::session_shared::should_publish_control_snapshot(
            &demand,
            false,
            Some(start),
            start + std::time::Duration::from_millis(50),
        ));

        let idle_demand = crate::control::ControlPublicationDemand::default();
        idle_demand.request_flush();
        assert!(crate::session_shared::should_publish_control_snapshot(
            &idle_demand,
            false,
            Some(start),
            start + std::time::Duration::from_millis(1),
        ));
        assert!(crate::session_shared::should_publish_control_snapshot(
            &idle_demand,
            true,
            Some(start),
            start + std::time::Duration::from_millis(1),
        ));
    }

    #[test]
    fn cached_base_client_ui_only_reserializes_on_world_rebuild() {
        let mut world = WorldStore::new();
        world.apply_low_disk_space_warning();
        let mut cached_ui = None;

        let first = super::resolve_base_client_ui(&world, false, &mut cached_ui);
        assert_eq!(first["low_disk_space_warning_count"], 1);

        world.apply_low_disk_space_warning();
        let reused = super::resolve_base_client_ui(&world, false, &mut cached_ui);
        assert_eq!(reused["low_disk_space_warning_count"], 1);

        let rebuilt = super::resolve_base_client_ui(&world, true, &mut cached_ui);
        assert_eq!(rebuilt["low_disk_space_warning_count"], 2);
    }

    #[test]
    fn publication_demand_tracks_attached_simulation_consumers_and_dirty_flush() {
        let demand = std::sync::Arc::new(crate::control::ControlPublicationDemand::default());
        let now = bbb_client_clock::Instant::now();
        assert!(!demand.needs_flush());

        demand.mark_dirty();
        assert!(!demand.needs_flush(), "unattached unit tests never flush");

        demand.attach_simulation();
        assert!(demand.needs_flush());

        let guard = demand.enter_consumer();
        assert!(demand.has_demand_at(now + std::time::Duration::from_secs(60)));
        drop(guard);
        assert!(!demand.has_demand_at(now + std::time::Duration::from_secs(60)));

        demand.clear_dirty();
        assert!(!demand.needs_flush());
        assert!(crate::control::method_reads_snapshot("world.client_ui"));
        assert!(crate::control::method_reads_snapshot("app.status"));
        assert!(!crate::control::method_reads_snapshot(
            "sim.lockstep_status"
        ));
        assert!(!crate::control::method_reads_snapshot("sim.step"));
    }
}
