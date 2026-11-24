use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_chat_component_interaction(
    interaction: crate::runtime::HudChatComponentInteraction,
    shift_down: bool,
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    net_counters: &mut crate::control::NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    platform: &mut PlatformEffects,
    measurer: &impl crate::input::CommandSuggestionTextMeasurer,
) -> bool {
    if shift_down {
        let Some(insertion) = interaction.insertion else {
            return false;
        };
        input.insert_chat_screen_text(&insertion, net_counters, world, net_commands, measurer);
        return false;
    }
    let Some(event) = interaction.click_event else {
        return false;
    };
    run_component_click_event(
        event,
        input,
        world,
        net_counters,
        net_commands,
        platform,
        measurer,
    )
}

pub(super) fn simulation_command_profile_kind(
    command: &SimulationCommand,
) -> SimulationCommandProfileKind {
    match command {
        SimulationCommand::Key { .. }
        | SimulationCommand::ImePreedit { .. }
        | SimulationCommand::ImeCommit(_)
        | SimulationCommand::ImeDisabled => SimulationCommandProfileKind::Keyboard,
        SimulationCommand::MouseButton { .. }
        | SimulationCommand::MouseMotion { .. }
        | SimulationCommand::MouseWheel { .. }
        | SimulationCommand::CursorMoved { .. }
        | SimulationCommand::CursorLeft => SimulationCommandProfileKind::Mouse,
        _ => SimulationCommandProfileKind::Other,
    }
}

pub(super) fn apply_hud_air_bubble_pop_sound(
    world: &mut WorldStore,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    level_generation: u64,
    position: bbb_protocol::packets::Vec3d,
    volume: f32,
    pitch: f32,
) {
    if world.client_level_generation() != level_generation {
        return;
    }
    let state = world.hud_air_bubble_pop_sound(position, volume, pitch);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_positioned_sound(&state);
    }
}

pub(super) fn debug_profiling_commands(
    requests: Vec<DebugProfilingToggleRequest>,
) -> Vec<DebugProfilingCommand> {
    requests
        .into_iter()
        .map(|request| match request {
            DebugProfilingToggleRequest::Start => DebugProfilingCommand::Start,
            DebugProfilingToggleRequest::Stop => DebugProfilingCommand::Stop,
        })
        .collect()
}

pub(super) const RECIPE_BOOK_LANGUAGE_TARGET: &str = "en_pt";

/// `RecipeBookComponent.checkSearchStringUpdate` updates `LanguageManager`
/// and `Options.languageCode`, starts `Minecraft.reloadResourcePacks`, then
/// calls `Options.save`; `save` writes the option and broadcasts the updated
/// `ClientInformation`. Ordinary platform effects remain ahead of the reload
/// barrier, so the render/resource owner observes the new locale first.
pub(super) fn apply_recipe_book_language_transition(
    requests: u32,
    target_available: bool,
    client_information: &mut ClientInformation,
    server_resource_packs: &mut ServerResourcePackRuntime,
    platform: &mut PlatformEffects,
) -> bool {
    if requests == 0
        || !target_available
        || client_information.language == RECIPE_BOOK_LANGUAGE_TARGET
    {
        return false;
    }

    client_information.language = RECIPE_BOOK_LANGUAGE_TARGET.to_string();
    platform.submit(PlatformEffect::SetClientLocale(
        RECIPE_BOOK_LANGUAGE_TARGET.to_string(),
    ));
    server_resource_packs.request_reload();
    platform.submit(PlatformEffect::SaveClientOptions {
        writes: vec![crate::options::ClientOptionWrite {
            key: "lang",
            value: crate::options::ClientOptionValue::Text(RECIPE_BOOK_LANGUAGE_TARGET),
        }],
        broadcast: client_information.clone(),
    });
    true
}

pub(super) fn apply_client_options_save_completed(
    information: ClientInformation,
    net_counters: &mut crate::control::NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    crate::input::queue_broadcast_client_information_command(
        net_counters,
        net_commands,
        &information,
    );
}

/// `Minecraft.resizeGui`: `Window.calculateScale`
/// plus `Window.setGuiScale`'s ceiling-division projection.
pub(super) fn project_gui_scale(
    surface_size: PhysicalSize<u32>,
    gui_scale_max: u32,
    enforce_unicode: bool,
) -> (u32, u32, u32) {
    let gui_scale = crate::window_surface::calculate_gui_scale(
        gui_scale_max,
        enforce_unicode,
        surface_size.width,
        surface_size.height,
    );
    let (gui_scaled_width, gui_scaled_height) =
        crate::window_surface::gui_scaled_size(surface_size.width, surface_size.height, gui_scale);
    (gui_scale, gui_scaled_width, gui_scaled_height)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ResourceReloadAckEffect {
    None,
    RebuildScreen,
    ShowLoadFailureToast,
    AbortRecovery { error: String },
}

pub(super) fn apply_resource_reload_ack(
    mut ack: crate::resource_reload::ResourceReloadAck,
    context: &mut SimulationInputContext<'_, '_>,
) -> ResourceReloadAckEffect {
    let Some((finish, result)) =
        context
            .server_resource_packs
            .finish_reload(ack.generation, || {
                let commit = ack.result?;
                commit.item_defaults.apply(context.world).map_err(|error| {
                    let error = error.to_string();
                    ack.diagnostics.last_reload_failure_stage = Some("item_defaults".to_string());
                    ack.diagnostics.last_reload_error = Some(error.clone());
                    error
                })?;
                Ok((commit.available_command_sounds, commit.win_screen_resources))
            })
    else {
        tracing::warn!(
            generation = ack.generation,
            "ignored stale server resource pack reload acknowledgement"
        );
        return ResourceReloadAckEffect::None;
    };
    let recovery_error = match (&finish, &result) {
        (ResourcePackReloadFinish::RecoveryFailed, Err(error)) => Some(error.clone()),
        _ => None,
    };
    context.net_counters.client_resources = std::sync::Arc::new(ack.diagnostics);
    match result {
        Ok((available_command_sounds, win_screen_resources)) => {
            context.input.creative_catalog_changed(context.world);
            context
                .input
                .refresh_creative_visible_items(context.world, context.item_runtime.as_ref());
            context
                .world
                .set_available_command_sounds(available_command_sounds);
            *context.win_screen_resources = std::sync::Arc::new(win_screen_resources);
            tracing::info!(
                generation = ack.generation,
                "applied server resource pack reload"
            );
        }
        Err(error) => tracing::warn!(
            generation = ack.generation,
            error = %error,
            "server resource pack reload failed"
        ),
    }
    context
        .server_resource_packs
        .publish_runtime_diagnostics(context.world);
    match finish {
        ResourcePackReloadFinish::Accepted => ResourceReloadAckEffect::RebuildScreen,
        ResourcePackReloadFinish::RecoverySucceeded => {
            ResourceReloadAckEffect::ShowLoadFailureToast
        }
        ResourcePackReloadFinish::RecoveryFailed => ResourceReloadAckEffect::AbortRecovery {
            error: recovery_error.expect("failed recovery acknowledgement carries an error"),
        },
    }
}

pub(super) fn chat_hit_target_is_current(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    target: HudChatHitTarget,
) -> bool {
    target.level_generation == world.client_level_generation()
        && target.delivery_revision == world.client_chat().delivery_revision
        && target.chat_screen_revision == input.chat_screen_revision()
        && target.resource_generation
            == item_runtime.map_or(0, NativeItemRuntime::resource_generation)
        && target.resources_loaded == item_runtime.is_some()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_book_component_click_event(
    mode: HudBookMode,
    event: ComponentClickEvent,
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    net_counters: &mut crate::control::NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    platform: &mut PlatformEffects,
    measurer: &impl crate::input::CommandSuggestionTextMeasurer,
) -> bool {
    match (mode, event) {
        (HudBookMode::View, ComponentClickEvent::ChangePage { page }) => {
            let page = usize::try_from(page.saturating_sub(1).max(0)).unwrap_or_default();
            if let Some(screen) = input.book_screen_mut() {
                screen.book.set_current_page(page);
            }
            true
        }
        (HudBookMode::Lectern, ComponentClickEvent::ChangePage { page }) => {
            let target_page = page.saturating_sub(1);
            let Some(lectern) = world.lectern_book_view() else {
                return false;
            };
            if target_page != i32::try_from(lectern.current_page).unwrap_or(i32::MAX) {
                queue_lectern_button_click(
                    world,
                    net_counters,
                    net_commands,
                    100_i32.saturating_add(target_page),
                );
            }
            true
        }
        (HudBookMode::View, ComponentClickEvent::RunCommand { command }) => {
            input.start_book_unattended_command(&command, world, net_counters, net_commands);
            true
        }
        (HudBookMode::Lectern, ComponentClickEvent::RunCommand { command }) => {
            crate::input::queue_container_close_command(net_counters, world, net_commands);
            input.apply_screen_requests(world, net_counters, net_commands);
            run_component_click_event(
                ComponentClickEvent::RunCommand { command },
                input,
                world,
                net_counters,
                net_commands,
                platform,
                measurer,
            )
        }
        (_, event) => run_component_click_event(
            event,
            input,
            world,
            net_counters,
            net_commands,
            platform,
            measurer,
        ),
    }
}

pub(super) fn run_component_click_event(
    event: ComponentClickEvent,
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    net_counters: &mut crate::control::NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    platform: &mut PlatformEffects,
    measurer: &impl crate::input::CommandSuggestionTextMeasurer,
) -> bool {
    match event {
        ComponentClickEvent::ChangePage { .. } => false,
        ComponentClickEvent::OpenFile { path } => {
            // `Util.getPlatform().openFile(openFile.file())`.
            // Only a client-made component carries this action: the serialized
            // codec refuses `open_file` from a server
            // (`ClickEvent.Action.OPEN_FILE` is `allowFromServer = false`).
            platform.submit(PlatformEffect::OpenPath(std::path::PathBuf::from(
                path.to_jdk_string(),
            )));
            true
        }
        ComponentClickEvent::OpenUrl { url } => {
            let handled = input.open_chat_url(url);
            let queued_effects = RefCell::new(Vec::new());
            collect_dialog_open_uri_effects(input, &queued_effects);
            flush_platform_effects(&queued_effects, platform);
            handled
        }
        ComponentClickEvent::RunCommand { command } => {
            input.start_unattended_command(&command, world, net_counters, net_commands);
            true
        }
        ComponentClickEvent::SuggestCommand { command } => {
            input.replace_chat_entry_java_text_with_measurer(command, measurer)
        }
        ComponentClickEvent::ShowDialog { dialog } => {
            input.show_dialog_from_component_click(world, &dialog)
        }
        ComponentClickEvent::CopyToClipboard { value } => {
            if value.units().is_empty() {
                return true;
            }
            platform.submit(PlatformEffect::ClipboardWrite(value));
            true
        }
        ComponentClickEvent::Custom { id, payload } => {
            if let Some(tx) = net_commands {
                if tx
                    .try_send(NetCommand::CustomClickAction(CustomClickAction {
                        id,
                        payload,
                    }))
                    .is_ok()
                {
                    net_counters.custom_click_action_commands_queued += 1;
                }
            }
            true
        }
    }
}
