//! Keyboard bookkeeping and gameplay actions below native screen dispatch.

use super::death_screen::DeathScreenPlatformRequest;
use super::disconnected_screen::DisconnectedScreenPlatformRequest;
use super::game_event_screen::GameEventScreenPlatformRequest;
use super::social::SocialInteractionsPlatformRequest;
use super::*;
use crate::narrator_runtime::{ScreenNarrationInput, ScreenNarrationTarget};
use crate::runtime::handle_dialog_key_input_with_modifiers_for_surface_and_measurer;
use crate::simulation::effects::{PlatformEffect, PlatformEffects};

pub(crate) fn capture_cursor(captured: &mut bool, requested: bool, platform: &mut PlatformEffects) {
    if *captured != requested {
        *captured = requested;
        platform.submit(PlatformEffect::SetCursorCapture(requested));
    }
}

fn record_key_input(input: &mut ClientInputState, code: KeyCode, pressed: bool) {
    input.note_key_input_type(code);
    input.set_key_down(code, pressed);
    input.set_shift_key(code, pressed);
    input.set_control_key(code, pressed);
}

fn handle_player_status_key(
    input: &ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
) -> bool {
    if !pressed {
        return false;
    }
    if world.local_player_is_dead() {
        if matches!(code, KeyCode::Enter | KeyCode::Space) {
            queue_perform_respawn_command(counters, net_commands);
        }
        return true;
    }

    if world.local_player_is_sleeping() {
        if matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::Space) {
            queue_player_command_action(
                counters,
                world,
                net_commands,
                PlayerCommandAction::StopSleeping,
                0,
            );
            return true;
        }
        // Vanilla `InBedChatScreen` allows opening chat/command while sleeping.
        if input.binding_matches("key.chat", code) || input.binding_matches("key.command", code) {
            return false;
        }
        return true;
    }
    false
}

/// Vanilla `Minecraft.getQuickActionsDialog` (`Minecraft.java:2036-2045`):
/// resolves `#minecraft:quick_actions` from `minecraft:dialog`, opening the
/// sole entry directly or `minecraft:quick_actions` when multiple exist.
fn resolve_quick_actions_dialog(world: &WorldStore) -> Option<bbb_client_world::DialogState> {
    const DIALOG_REGISTRY: &str = "minecraft:dialog";
    const QUICK_ACTIONS_ID: &str = "minecraft:quick_actions";
    let holder_ids = world.registry_holder_ids_for_tag(DIALOG_REGISTRY, QUICK_ACTIONS_ID)?;
    if holder_ids.is_empty() {
        return None;
    }
    let (registry_id, resource_id) = if holder_ids.len() == 1 {
        let holder_id = holder_ids[0];
        let resource_id = world
            .registry_resource_id_by_holder_id(DIALOG_REGISTRY, holder_id)
            .map(str::to_string);
        (Some(holder_id), resource_id)
    } else {
        let holder_id =
            world.registry_holder_id_by_resource_id(DIALOG_REGISTRY, QUICK_ACTIONS_ID)?;
        (Some(holder_id), Some(QUICK_ACTIONS_ID.to_string()))
    };
    world.resolve_dialog_state(&bbb_client_world::DialogState {
        holder_kind: "reference".to_string(),
        registry_id,
        resource_id,
        dialog: None,
        resolution_error: None,
        data: None,
        semantic_error: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_inventory_screen_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    code: KeyCode,
    pressed: bool,
    just_pressed: bool,
    surface_size: PhysicalSize<u32>,
    mut clipboard: Option<&mut dyn DebugClipboard>,
) {
    if handle_player_status_key(input, counters, world, net_commands, code, pressed) {
        return;
    }

    if just_pressed {
        if matches!(code, KeyCode::Escape)
            && input.screen.container().is_some()
            && maybe_close_narrow_recipe_book(input, world, counters, net_commands, surface_size)
        {
            return;
        }
        if matches!(code, KeyCode::Escape)
            && queue_container_close_command(counters, world, net_commands)
        {
            input.apply_screen_requests(world, counters, net_commands);
            return;
        }
        if input.binding_matches("key.inventory", code)
            && !anvil_rename_entry_consumes_key(world, code)
            && !recipe_book_search_entry_consumes_key(input, world, code)
            && !creative_inventory_search_entry_consumes_key(input, world, code)
            && queue_container_close_command(counters, world, net_commands)
        {
            input.apply_screen_requests(world, counters, net_commands);
            return;
        }
    }
    // Vanilla routes every open-screen hotbar key through
    // AbstractContainerScreen.checkHotbarKeyPressed. In particular, the
    // creative C/X activators do not reach handleHotbarLoadOrSave while
    // CreativeModeInventoryScreen is open; the hovered-slot swap wins.
    if pressed {
        handle_inventory_key_input_with_clipboard(
            input,
            world,
            counters,
            net_commands,
            item_runtime,
            code,
            &mut clipboard,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_gameplay_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    code: KeyCode,
    pressed: bool,
    just_pressed: bool,
    text_measurer: &impl CommandSuggestionTextMeasurer,
    first_person_hand_presentation: &mut crate::item_models::FirstPersonHandPresentation,
    entity_partial_tick: f32,
    level_random: &mut bbb_client_world::LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn crate::audio_runtime::AudioEventSink>,
) {
    if handle_player_status_key(input, counters, world, net_commands, code, pressed) {
        return;
    }

    if pressed {
        if just_pressed && matches!(code, KeyCode::Escape) {
            input.set_key_down(code, false);
            input.open_debug_pause_screen_with_menu(world, counters, net_commands);
            return;
        }
        if just_pressed {
            input.record_debug_profiler_chart_navigation_key(code);
        }
        if just_pressed && input.binding_matches("key.toggleGui", code) {
            input.hide_gui = !input.hide_gui;
            return;
        }
        if just_pressed && input.binding_matches("key.toggleSpectatorShaderEffects", code) {
            input.toggle_camera_post_effect();
            return;
        }
        if just_pressed && input.binding_matches("key.togglePerspective", code) {
            input.cycle_camera_type(current_post_effect_id_from_world(world));
            return;
        }
        if let Some(slot) = hotbar_slot_for_key(input.key_bindings(), code) {
            if world.local_player_is_spectator() {
                input.handle_spectator_hotbar_selected(world, counters, net_commands, slot);
                return;
            }
            if handle_creative_hotbar_preset_key(input, world, counters, net_commands, slot) {
                return;
            }
            select_hotbar_slot(counters, world, net_commands, slot);
            return;
        }
        if just_pressed && input.binding_matches("key.drop", code) {
            if world.local_player_is_spectator() {
                return;
            }
            let drop_all = input.control_down();
            let action = if drop_all {
                PlayerActionKind::DropAllItems
            } else {
                PlayerActionKind::DropItem
            };
            let dropped_item = world.drop_local_selected_hotbar_item(drop_all);
            queue_zero_pos_player_action_command(counters, net_commands, action);
            if dropped_item {
                queue_swing_command(world, counters, net_commands, InteractionHand::MainHand);
            }
            return;
        }
        if just_pressed && input.binding_matches("key.swapOffhand", code) {
            if world.local_player_is_spectator() {
                return;
            }
            queue_zero_pos_player_action_command(
                counters,
                net_commands,
                PlayerActionKind::SwapItemWithOffhand,
            );
            return;
        }
        if just_pressed && input.binding_matches("key.inventory", code) {
            if world
                .local_player_server_controlled_inventory_vehicle_id()
                .is_some()
            {
                queue_player_command_action(
                    counters,
                    world,
                    net_commands,
                    PlayerCommandAction::OpenInventory,
                    0,
                );
            } else {
                world.tutorial_on_open_inventory();
                input.close_screens(counters, net_commands);
                world.open_local_inventory();
                input.apply_screen_requests(world, counters, net_commands);
                input.refresh_creative_visible_items(world, item_runtime);
            }
            return;
        }
        if just_pressed && input.binding_matches("key.advancements", code) {
            input.open_advancements_screen(counters, world, net_commands, false);
            return;
        }
        if just_pressed && input.binding_matches("key.quickActions", code) {
            if let Some(dialog) = resolve_quick_actions_dialog(world) {
                input.show_dialog(dialog);
            }
            return;
        }
        if just_pressed && input.binding_matches("key.socialInteractions", code) {
            input.open_social_interactions_screen(world, counters, net_commands);
            return;
        }
        if just_pressed && input.binding_matches("key.smoothCamera", code) {
            // Vanilla `Minecraft.handleKeybinds`: each `key.smoothCamera`
            // click flips the transient `Options.smoothCamera` toggle.
            input.smooth_camera = !input.smooth_camera;
            return;
        }
        if just_pressed && input.binding_matches("key.chat", code) {
            open_chat_entry_with_measurer(
                input,
                world,
                counters,
                net_commands,
                ChatMethod::Message,
                true,
                text_measurer,
            );
            return;
        }
        if just_pressed && input.binding_matches("key.command", code) {
            open_chat_entry_with_measurer(
                input,
                world,
                counters,
                net_commands,
                ChatMethod::Command,
                true,
                text_measurer,
            );
            return;
        }
    }

    if !pressed || just_pressed {
        let state = if pressed {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
        mouse::handle_gameplay_bound_input(
            input,
            first_person_hand_presentation,
            world,
            counters,
            net_commands,
            BoundInput::Key(code),
            state,
            entity_partial_tick,
            level_random,
            audio_events,
        );
    }

    let before = player_input_from_state(input);
    let matches_binding =
        |name: &str, alias: KeyCode| input.binding_matches(name, code) || code == alias;
    let handled = if matches_binding("key.forward", KeyCode::ArrowUp) {
        input.forward = pressed;
        if pressed && !before.forward {
            maybe_apply_forward_sprint_trigger(input, world);
        }
        true
    } else if matches_binding("key.back", KeyCode::ArrowDown) {
        input.backward = pressed;
        if pressed {
            input.clear_sprint_trigger();
        }
        true
    } else if matches_binding("key.left", KeyCode::ArrowLeft) {
        input.left = pressed;
        true
    } else if matches_binding("key.right", KeyCode::ArrowRight) {
        input.right = pressed;
        true
    } else if input.binding_matches("key.jump", code) {
        input.jump = pressed;
        true
    } else if matches_binding("key.sneak", KeyCode::ShiftRight) {
        let down = if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            input.shift_down()
        } else {
            pressed
        };
        input.sneak = input.toggled_down(input.toggle_crouch, input.sneak, just_pressed, down);
        if input.sneak {
            input.clear_sprint_trigger();
        }
        true
    } else if matches_binding("key.sprint", KeyCode::ControlRight) {
        input.sprint_key_down = input.toggled_down(
            input.toggle_sprint,
            input.sprint_key_down,
            just_pressed,
            pressed,
        );
        if input.sprint_key_down {
            input.clear_sprint_trigger();
        }
        true
    } else {
        false
    };
    if handled {
        let after = player_input_from_state(input);
        if after != before {
            queue_player_input_command(counters, net_commands, after);
            let just_toggled_creative_flight =
                maybe_toggle_creative_flight(input, counters, world, net_commands, before, after);
            if should_queue_start_fall_flying(world, before, after) && !just_toggled_creative_flight
            {
                queue_player_command_action(
                    counters,
                    world,
                    net_commands,
                    PlayerCommandAction::StartFallFlying,
                    0,
                );
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn handle_key_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    crate::input::key_events::dispatch_key_input(
        input,
        counters,
        world,
        net_commands,
        None,
        physical_key,
        state,
        PhysicalSize::new(1280, 720),
        &FallbackTextMeasurer,
        None,
        None,
        &mut true,
        &mut crate::simulation::effects::PlatformEffects::default(),
    );
}

fn handle_chat_entry_text(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
    text_measurer: &impl CommandSuggestionTextMeasurer,
) {
    let opened_from_command_text = input.screen.chat_entry().is_none() && text.starts_with('/');
    if input.screen.chat_entry().is_none() {
        if !text.starts_with('/') {
            return;
        }
        open_chat_entry_with_measurer(
            input,
            world,
            counters,
            net_commands,
            ChatMethod::Command,
            true,
            text_measurer,
        );
    }

    let text = if opened_from_command_text {
        text.strip_prefix('/').unwrap_or(text)
    } else {
        text
    };

    let chat_binding = input.key_bindings().input("key.chat");
    let command_binding = input.key_bindings().input("key.command");
    let suppress_opening_text = input.screen.chat_entry_mut().is_some_and(|entry| {
        if !entry.suppress_open_key_commit {
            return false;
        }
        entry.suppress_open_key_commit = false;
        match &entry.owner {
            CommandEntryKind::Chat(screen) => match screen.method {
                ChatMethod::Message => match chat_binding {
                    BoundInput::Key(code) => {
                        crate::input::key_code_matches_opening_text(code, text)
                    }
                    _ => matches!(text, "t" | "T"),
                },
                ChatMethod::Command => {
                    text.is_empty()
                        || text == "/"
                        || matches!(
                            command_binding,
                            BoundInput::Key(code)
                                if crate::input::key_code_matches_opening_text(code, text)
                        )
                }
            },
            CommandEntryKind::CommandBlock(_) | CommandEntryKind::MinecartCommandBlock(_) => false,
        }
    });
    if suppress_opening_text {
        maybe_queue_command_suggestion_request(input, counters, world, net_commands);
        return;
    }
    let layout = input.chat_input_layout(text_measurer);
    let Some(entry) = input.screen.chat_entry_mut() else {
        return;
    };
    let edited = insert_chat_entry_text(entry, text, &layout);
    update_chat_entry_after_text_change(input, counters, world, net_commands, edited);
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_key_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    physical_key: PhysicalKey,
    state: ElementState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &impl CommandSuggestionTextMeasurer,
    clipboard: Option<&mut dyn DebugClipboard>,
    terrain_reload: Option<&mut dyn TerrainReloadRequestSink>,
    cursor_captured: &mut bool,
    platform: &mut PlatformEffects,
) -> bool {
    let mut first_person_hand_presentation =
        crate::item_models::FirstPersonHandPresentation::default();
    let mut level_random = bbb_client_world::LevelEventSoundRandomState::default();
    let mut audio_events = None;
    dispatch_key_input_with_gameplay_effects(
        input,
        counters,
        world,
        net_commands,
        item_runtime,
        physical_key,
        state,
        surface_size,
        text_measurer,
        clipboard,
        terrain_reload,
        cursor_captured,
        platform,
        &mut first_person_hand_presentation,
        1.0,
        &mut level_random,
        &mut audio_events,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_key_input_with_gameplay_effects(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    physical_key: PhysicalKey,
    state: ElementState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &impl CommandSuggestionTextMeasurer,
    mut clipboard: Option<&mut dyn DebugClipboard>,
    terrain_reload: Option<&mut dyn TerrainReloadRequestSink>,
    cursor_captured: &mut bool,
    platform: &mut PlatformEffects,
    first_person_hand_presentation: &mut crate::item_models::FirstPersonHandPresentation,
    entity_partial_tick: f32,
    level_random: &mut bbb_client_world::LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn crate::audio_runtime::AudioEventSink>,
) -> bool {
    input.set_chat_input_surface_size(surface_size);
    let PhysicalKey::Code(code) = physical_key else {
        return false;
    };
    let pressed = state == ElementState::Pressed;
    let just_pressed = pressed && !input.physical_key_down(code);
    if input.handle_global_key_press(physical_key, state) {
        record_key_input(input, code, pressed);
        return true;
    }
    input.set_physical_key_down(code, pressed);
    let screen_was_open = input.current_screen().is_some();
    macro_rules! narrator_key {
        () => {
            if pressed && code == KeyCode::KeyB {
                input.handle_narrator_hotkey(world);
            }
        };
    }
    let mut terrain_reload = terrain_reload;
    macro_rules! debug_key {
        () => {
            if input.handle_debug_overlay_key_with_clipboard_and_net(
                physical_key,
                state,
                Some(world),
                terrain_reload.take(),
                clipboard
                    .as_mut()
                    .map(|v| &mut **v as &mut dyn DebugClipboard),
                counters,
                net_commands,
            ) {
                if screen_was_open && input.current_screen().is_none() {
                    capture_cursor(cursor_captured, true, platform);
                } else if !screen_was_open && input.current_screen().is_some() {
                    capture_cursor(cursor_captured, false, platform);
                }
                return true;
            }
        };
    }
    match input.current_screen() {
        Some(Screen::CommandConfirmation(_)) => {
            record_key_input(input, code, pressed);
            narrator_key!();
            input.handle_command_confirmation_key(
                world,
                counters,
                net_commands,
                code,
                pressed,
                text_measurer,
            );
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::CommandConfirmation,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
            capture_cursor(
                cursor_captured,
                !input.wants_cursor() && !input.command_entry_is_active(),
                platform,
            );
        }
        Some(Screen::Disconnected(_)) => {
            record_key_input(input, code, pressed);
            capture_cursor(cursor_captured, false, platform);
            input.handle_disconnected_screen_key(code, state);
            collect_disconnected_screen_platform_effects(input, platform);
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Disconnected,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
        Some(Screen::ChatRestrictions(_)) => {
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            if just_pressed
                && matches!(
                    code,
                    KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter
                )
            {
                input.close_chat_restrictions_screen();
            }
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Chat,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
        Some(Screen::Death(_)) => {
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_death_screen_key(counters, net_commands, code, just_pressed);
            collect_death_screen_platform_effects(input, platform);
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::Death,
                input: ScreenNarrationInput::KeyboardAction,
            });
            if input.death_screen().is_none() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::GameEvent(_)) => {
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_game_event_screen_key(counters, net_commands, code, pressed, just_pressed);
            collect_game_event_screen_platform_effects(input, platform);
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::GameEvent,
                input: ScreenNarrationInput::KeyboardAction,
            });
            if input.game_event_screen_runtime().is_none() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::Social(_)) => {
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.refresh_social_interactions_scroll_bounds(surface_size);
            input.handle_social_interactions_key_state(
                world,
                code,
                pressed,
                just_pressed,
                &mut clipboard,
            );
            collect_social_interactions_platform_effects(input, platform);
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::SocialInteractions,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
        Some(Screen::Pause(_)) => {
            if !pressed {
                record_key_input(input, code, false);
            }
            if input.handle_debug_pause_screen_key(world, physical_key, state) {
                if pressed {
                    platform.submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Pause,
                        input: ScreenNarrationInput::KeyboardAction,
                    });
                }
            } else {
                let handled = input.handle_debug_overlay_key_with_clipboard_and_net(
                    physical_key,
                    state,
                    Some(world),
                    terrain_reload.take(),
                    clipboard.take(),
                    counters,
                    net_commands,
                );
                if screen_was_open && input.current_screen().is_none() {
                    capture_cursor(cursor_captured, true, platform);
                }
                return handled;
            }
        }
        Some(Screen::DebugOptions(_)) => {
            if !pressed {
                record_key_input(input, code, false);
            }
            let search_before = input.debug_options_screen_search_text();
            if input.handle_debug_options_screen_key_with_clipboard_and_surface(
                physical_key,
                state,
                clipboard
                    .as_mut()
                    .map(|v| &mut **v as &mut dyn DebugClipboard),
                surface_size,
            ) {
                if pressed {
                    platform.submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::DebugOptions,
                        input: if input.debug_options_screen_search_text() != search_before {
                            ScreenNarrationInput::Immediate
                        } else {
                            ScreenNarrationInput::KeyboardAction
                        },
                    });
                }
                if input.wants_cursor() {
                    capture_cursor(cursor_captured, false, platform);
                }
            } else {
                let handled = input.handle_debug_overlay_key_with_clipboard_and_net(
                    physical_key,
                    state,
                    Some(world),
                    terrain_reload.take(),
                    clipboard.take(),
                    counters,
                    net_commands,
                );
                if screen_was_open && input.current_screen().is_none() {
                    capture_cursor(cursor_captured, true, platform);
                }
                return handled;
            }
        }
        Some(Screen::GameModeSwitcher(_)) => {
            if !pressed {
                record_key_input(input, code, false);
            }
            debug_key!();
        }
        Some(
            Screen::Dialog(_)
            | Screen::DialogLink(_)
            | Screen::DialogWarning(_)
            | Screen::DialogWaiting(_),
        ) => {
            debug_key!();
            if !pressed {
                record_key_input(input, code, false);
            } else {
                input.set_shift_key(code, true);
                input.set_control_key(code, true);
            }
            capture_cursor(cursor_captured, false, platform);
            let shift = input.shift_down();
            let control = if matches!(
                code,
                KeyCode::KeyA | KeyCode::KeyC | KeyCode::KeyV | KeyCode::KeyX
            ) {
                input.text_edit_shortcut_down()
            } else {
                input.text_field_modifiers().word && !input.alt_down()
            };
            handle_dialog_key_input_with_modifiers_for_surface_and_measurer(
                input,
                counters,
                world,
                net_commands,
                clipboard,
                physical_key,
                state,
                shift,
                control,
                surface_size,
                text_measurer.command_suggestion_glyphs(),
            );
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Dialog,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
            if !input.wants_cursor() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::GameRules(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_game_rules_screen_key(
                counters,
                net_commands,
                code,
                pressed,
                surface_size,
                &mut clipboard,
            );
        }
        Some(Screen::StructureBlock(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_structure_block_editor_key(
                counters,
                net_commands,
                code,
                pressed,
                &mut clipboard,
            );
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::StructureBlock,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
            if !input.wants_cursor() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::JigsawBlock(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_jigsaw_block_editor_key(
                counters,
                net_commands,
                code,
                pressed,
                &mut clipboard,
            );
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::JigsawBlock,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
            if !input.wants_cursor() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::TestBlock(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_test_block_editor_key(code, pressed, &mut clipboard);
            if !input.wants_cursor() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::TestInstanceBlock(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_test_instance_block_editor_key(
                counters,
                net_commands,
                code,
                pressed,
                &mut clipboard,
            );
            if !input.wants_cursor() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::Sign(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            handle_sign_editor_key(
                input,
                counters,
                net_commands,
                code,
                pressed,
                text_measurer,
                &mut clipboard,
            );
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Sign,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
        Some(Screen::CommandEntry(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            let is_command_block = input.command_block_editor_hud_state().is_some();
            handle_chat_entry_key(
                input,
                counters,
                world,
                net_commands,
                code,
                pressed,
                clipboard,
                text_measurer,
            );
            // Vanilla `InBedChatScreen.onClose` (`InBedChatScreen.java:25-32`):
            // pressing Escape to close bed chat also wakes the player up.
            if just_pressed
                && code == KeyCode::Escape
                && !is_command_block
                && world.local_player_is_sleeping()
            {
                queue_player_command_action(
                    counters,
                    world,
                    net_commands,
                    PlayerCommandAction::StopSleeping,
                    0,
                );
            }
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: if is_command_block {
                        ScreenNarrationTarget::CommandBlock
                    } else {
                        ScreenNarrationTarget::Chat
                    },
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
            if !input.command_entry_is_active() && !input.wants_cursor() {
                capture_cursor(cursor_captured, true, platform);
            }
        }
        Some(Screen::Book(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_book_screen_key(
                counters,
                world,
                net_commands,
                code,
                pressed,
                text_measurer,
                &mut clipboard,
            );
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Book,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
        Some(Screen::Advancements(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            input.handle_advancements_screen_key(counters, world, net_commands, code, pressed);
        }
        Some(Screen::Stats(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            capture_cursor(cursor_captured, false, platform);
            if pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Stats,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
            input.handle_stats_screen_key(counters, world, net_commands, code, pressed);
        }
        Some(Screen::Container(_) | Screen::Creative(_)) => {
            debug_key!();
            record_key_input(input, code, pressed);
            narrator_key!();
            let lectern_open =
                input.screen.container().is_some() && world.lectern_book_view().is_some();
            let recipe_book_open = recipe_book_screen_is_open(input, world);
            handle_inventory_screen_key(
                input,
                counters,
                world,
                net_commands,
                item_runtime,
                code,
                pressed,
                just_pressed,
                surface_size,
                clipboard,
            );
            if !input.wants_cursor() && !input.command_entry_is_active() {
                capture_cursor(cursor_captured, true, platform);
            }
            if pressed {
                let target = if lectern_open {
                    ScreenNarrationTarget::Book
                } else if recipe_book_open {
                    ScreenNarrationTarget::RecipeBook
                } else {
                    ScreenNarrationTarget::Inventory
                };
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
        None => {
            if world.level_load().screen_is_up() {
                if !pressed {
                    record_key_input(input, code, false);
                }
                capture_cursor(cursor_captured, false, platform);
                return false;
            }
            debug_key!();
            let pause_escape = just_pressed && code == KeyCode::Escape;
            if pause_escape {
                capture_cursor(cursor_captured, false, platform);
                release_active_input(input, world, counters, net_commands);
            }
            if !*cursor_captured && !pause_escape {
                if !pressed {
                    record_key_input(input, code, false);
                }
                return false;
            }
            record_key_input(input, code, pressed);
            narrator_key!();
            handle_gameplay_key(
                input,
                counters,
                world,
                net_commands,
                item_runtime,
                code,
                pressed,
                just_pressed,
                text_measurer,
                first_person_hand_presentation,
                entity_partial_tick,
                level_random,
                audio_events,
            );
            if input.chat_entry_is_active() && pressed {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Chat,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
            if input.social_interactions_runtime().is_some() && pressed {
                capture_cursor(cursor_captured, false, platform);
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::SocialInteractions,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
    }
    // Centralize cursor recapture/release on Screen transitions, matching
    // Vanilla `Minecraft.setScreen` (`Minecraft.java:1146-1158`).
    if screen_was_open && input.current_screen().is_none() && !world.level_load().screen_is_up() {
        capture_cursor(cursor_captured, true, platform);
    } else if !screen_was_open
        && (input.current_screen().is_some() || world.level_load().screen_is_up())
    {
        capture_cursor(cursor_captured, false, platform);
    }
    false
}

pub(crate) fn recipe_book_screen_is_open(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
) -> bool {
    input.screen.container().is_some()
        && inventory_screen_layout(world)
            .and_then(|layout| recipe_book_type_for_background(layout.background))
            .is_some_and(|book_type| recipe_book_type_settings(world, book_type).open)
}

pub(crate) fn collect_game_event_screen_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffects,
) {
    for request in input.take_game_event_screen_platform_requests() {
        platform.submit(match request {
            GameEventScreenPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url.into()),
            GameEventScreenPlatformRequest::ClipboardWrite(text) => {
                PlatformEffect::ClipboardWrite(text.into())
            }
        });
    }
}

pub(crate) fn collect_social_interactions_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffects,
) {
    for request in input.take_social_interactions_platform_requests() {
        platform.submit(match request {
            SocialInteractionsPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url.into()),
            SocialInteractionsPlatformRequest::ClipboardWrite(text) => {
                PlatformEffect::ClipboardWrite(text.into())
            }
        });
    }
}

pub(crate) fn collect_death_screen_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffects,
) {
    for request in input.take_death_screen_platform_requests() {
        platform.submit(match request {
            DeathScreenPlatformRequest::OpenUri(url) => PlatformEffect::OpenUri(url),
            DeathScreenPlatformRequest::ClipboardWrite(text) => {
                PlatformEffect::ClipboardWrite(text)
            }
        });
    }
}

pub(crate) fn collect_disconnected_screen_platform_effects(
    input: &mut crate::input::ClientInputState,
    platform: &mut PlatformEffects,
) {
    for request in input.take_disconnected_screen_platform_requests() {
        platform.submit(match request {
            DisconnectedScreenPlatformRequest::OpenPath(path) => PlatformEffect::OpenPath(path),
            #[cfg(not(target_arch = "wasm32"))]
            DisconnectedScreenPlatformRequest::Exit => PlatformEffect::Exit,
            DisconnectedScreenPlatformRequest::Reconnect => PlatformEffect::Reconnect,
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextInputSource {
    Ime,
    Control,
}

pub(crate) fn handle_text_input_with_item_runtime(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    text: &str,
    text_measurer: &impl CommandSuggestionTextMeasurer,
) {
    dispatch_text_input(
        input,
        counters,
        world,
        net_commands,
        item_runtime,
        text,
        text_measurer,
        &mut true,
        &mut PlatformEffects::default(),
        TextInputSource::Control,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_text_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    text: &str,
    text_measurer: &impl CommandSuggestionTextMeasurer,
    cursor_captured: &mut bool,
    platform: &mut PlatformEffects,
    source: TextInputSource,
) {
    if source == TextInputSource::Control && !input.focused {
        return;
    }
    match input.current_screen() {
        Some(Screen::CommandConfirmation(_)) => {}
        Some(Screen::DebugOptions(_)) => {
            capture_cursor(cursor_captured, false, platform);
            let before = input.debug_options_screen_search_text();
            input.handle_debug_options_screen_text_input(text);
            if input.debug_options_screen_search_text() != before {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::DebugOptions,
                    input: ScreenNarrationInput::Immediate,
                });
            }
        }
        Some(
            Screen::Dialog(_)
            | Screen::DialogLink(_)
            | Screen::DialogWarning(_)
            | Screen::DialogWaiting(_),
        ) => {
            capture_cursor(cursor_captured, false, platform);
            crate::runtime::handle_dialog_text_input_with_measurer(
                input,
                world,
                text,
                text_measurer.command_suggestion_glyphs(),
            );
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::Dialog,
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::Social(_)) => {
            capture_cursor(cursor_captured, false, platform);
            input.handle_social_interactions_text(world, text);
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::SocialInteractions,
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::GameRules(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if !input.focused {
                return;
            }
            input.handle_game_rules_screen_text(text);
        }
        Some(Screen::StructureBlock(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if !input.focused {
                return;
            }
            input.handle_structure_block_editor_text(text);
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::StructureBlock,
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::JigsawBlock(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if !input.focused {
                return;
            }
            input.handle_jigsaw_block_editor_text(text);
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::JigsawBlock,
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::TestBlock(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if !input.focused {
                return;
            }
            input.handle_test_block_editor_text(text);
        }
        Some(Screen::TestInstanceBlock(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if !input.focused {
                return;
            }
            input.handle_test_instance_block_editor_text(counters, net_commands, text);
        }
        Some(Screen::Sign(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if !input.focused {
                return;
            }
            handle_sign_editor_text(input, text, text_measurer);
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::Sign,
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::Book(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if !input.focused {
                return;
            }
            input.handle_book_screen_text(text, text_measurer);
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: ScreenNarrationTarget::Book,
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::Container(_) | Screen::Creative(_)) => {
            capture_cursor(cursor_captured, false, platform);
            if input.focused {
                let _ = handle_inventory_text_input(
                    input,
                    world,
                    counters,
                    net_commands,
                    item_runtime,
                    text,
                );
            }
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: if recipe_book_screen_is_open(input, world) {
                    ScreenNarrationTarget::RecipeBook
                } else {
                    ScreenNarrationTarget::Inventory
                },
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::CommandEntry(_)) => {
            capture_cursor(cursor_captured, false, platform);
            let is_command_block = input.command_block_editor_hud_state().is_some();
            if input.focused {
                handle_chat_entry_text(input, counters, world, net_commands, text, text_measurer);
            }
            platform.submit(PlatformEffect::ScreenNarrationInput {
                target: if is_command_block {
                    ScreenNarrationTarget::CommandBlock
                } else {
                    ScreenNarrationTarget::Chat
                },
                input: ScreenNarrationInput::KeyboardAction,
            });
        }
        Some(Screen::Advancements(_) | Screen::Stats(_)) => {
            capture_cursor(cursor_captured, false, platform);
        }
        Some(Screen::ChatRestrictions(_) | Screen::Death(_) | Screen::GameEvent(_)) => {}
        Some(Screen::Disconnected(_) | Screen::Pause(_) | Screen::GameModeSwitcher(_)) | None => {
            if !*cursor_captured || !input.focused {
                return;
            }
            handle_chat_entry_text(input, counters, world, net_commands, text, text_measurer);
            if input.chat_entry_is_active() {
                platform.submit(PlatformEffect::ScreenNarrationInput {
                    target: ScreenNarrationTarget::Chat,
                    input: ScreenNarrationInput::KeyboardAction,
                });
            }
        }
    }
}
