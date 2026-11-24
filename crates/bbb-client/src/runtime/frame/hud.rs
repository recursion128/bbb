use super::*;

pub(super) struct ExtractedHudAndScreens {
    pub(super) hud_player_health: RendererFrameUpdate<Option<HudPlayerHealth>>,
    pub(super) hud_jump_bar: Option<HudJumpBar>,
    pub(super) hud_experience_progress: Option<f32>,
    pub(super) hud_hotbar_mode: RendererFrameUpdate<HudHotbarMode>,
    pub(super) hud_selected_item_name:
        RendererFrameUpdate<Option<bbb_client_render::HudSelectedItemName>>,
    pub(super) hud_action_bar_text: RendererFrameUpdate<Option<HudActionBarText>>,
    pub(super) hud_title_text: RendererFrameUpdate<Option<HudTitleText>>,
    pub(super) hud_scoreboard_sidebar: RendererFrameUpdate<Option<HudScoreboardSidebar>>,
    pub(super) hud_player_list: RendererFrameUpdate<Option<HudPlayerList>>,
    pub(super) hud_player_list_tick_count: Option<u64>,
    pub(super) hud_chat_messages: RendererFrameUpdate<HudChatMessagesFrame>,
    pub(super) hud_static_ui: RendererFrameUpdate<HudStaticUiFrame>,
    pub(super) hud_boss_bars: RendererFrameUpdate<Vec<HudBossBar>>,
    pub(super) item_registry_keys: ItemRegistryKeySet,
    pub(super) hud_hotbar_items: RendererFrameUpdate<Box<HudHotbarItemsFrame>>,
    pub(super) hud_offhand_block_item_model: RendererFrameUpdate<Option<HudBlockItemModel>>,
    pub(super) hud_hotbar_block_item_models: RendererFrameUpdate<Vec<Option<HudBlockItemModel>>>,
    pub(super) hud_screens: RendererFrameUpdate<Box<HudScreensFrame>>,
    pub(super) hud_toasts: RendererFrameUpdate<Vec<HudToast>>,
    pub(super) hud_now_playing_toast: RendererFrameUpdate<Option<HudNowPlayingToast>>,
    pub(super) toasts_duration: Duration,
    pub(super) hud_duration: Duration,
}

pub(super) fn extract_hud_and_screens(
    view: &SimulationView<'_>,
    context: &mut SimulationRenderContext<'_>,
    basics: &FrameBasics,
) -> ExtractedHudAndScreens {
    let state = view.frame;
    let input = view.input;
    let world = view.world;
    let item_runtime = context.item_runtime;
    // Vanilla handles gameplay keybinds during `Minecraft.tick`, then `GameRenderer.extractGui`
    // calls `Gui.extractRenderState`; HUD values therefore read after input and use-item updates.
    let hud_started_at = Instant::now();
    let local_player = world.local_player();
    // Vanilla reaches `Gui.extractPlayerHealth` only while the GUI is visible
    // and `GameMode.canHurtPlayer()` is true (survival/adventure). The synced
    // health then pairs with the MAX_HEALTH attribute, absorption, hardcore,
    // `HeartType.forPlayer`, Regeneration, and the client tick.
    let game_mode_can_hurt_player = matches!(
        GameType::from_id(world.gameplay().game_type),
        GameType::Survival | GameType::Adventure
    );
    let camera_player = world.hud_camera_player_vitals();
    let health_player =
        (!input.hide_gui() && game_mode_can_hurt_player && !basics.level_loading_screen_is_up)
            .then_some(camera_player)
            .flatten();
    let raw_player_health = health_player
        .filter(|player| player.health.is_finite())
        .map(|player| {
            let presentation = context.hud_render_runtime.player_health_frame(
                basics.render_started_at,
                view.client_tick_index,
                player.health.ceil() as i32,
                player.invulnerable_time,
            );
            HudPlayerHealth {
                health: player.health,
                display_health: presentation.display_health,
                blinking: presentation.blinking,
                max_health: player.max_health,
                absorption: player.absorption,
                heart_type: camera_player_heart_type(player),
                hardcore: world.is_hardcore(),
                regen: player.regenerating,
                tick_count: view.client_tick_index,
            }
        });
    let hud_player_health = context
        .hud_render_runtime
        .player_health_update(raw_player_health);
    // Vanilla `Gui.nextContextualInfoState` selects the jumpable-vehicle bar
    // over the experience bar when the local controlled mount implements
    // `PlayerRideableJumping.canJump()`. The cooldown gate is the integer
    // `getJumpCooldown() > 0` (not partial-tick interpolated).
    let hud_jump_bar = world.local_player_rideable_jumping_vehicle_id().map(|_| {
        let cooldown = world
            .local_player_rideable_jumping_vehicle_cooldown(0.0)
            .is_some_and(|cooldown| cooldown > 0.0);
        HudJumpBar {
            progress: input.riding_jump_scale().unwrap_or(0.0),
            cooldown,
        }
    });
    let hud_experience_progress = local_player
        .experience
        .map(|experience| experience.progress);
    // Vanilla `Gui.extractHotbarAndDecorations` substitutes `SpectatorGui`
    // for the ordinary item hotbar and samples its wall-clock fade during
    // render extraction, after input publication.
    let raw_hotbar_mode =
        hud_hotbar_mode_from_world(input, world, item_runtime, basics.render_started_at);
    let ordinary_hotbar = matches!(&raw_hotbar_mode, HudHotbarMode::Ordinary);
    let hud_hotbar_mode = context
        .hud_render_runtime
        .hotbar_mode_update(raw_hotbar_mode);
    // Vanilla `Gui.extractItemHotbar` reads the camera player's
    // `Inventory.getSelectedSlot()`; a remote player's client-side inventory
    // keeps the construction-time slot 0.
    let hud_selected_slot = if world.hud_camera_player_is_local_player() {
        local_player.selected_hotbar_slot
    } else {
        0
    };
    let hud_selected_item_name = context.hud_render_runtime.selected_item_name_update(
        basics.advanced_ticks,
        world.local_player_id().is_some(),
        !game_mode_can_hurt_player,
        context.notification_display_time,
        || {
            let stack = world.inventory().hotbar_item_states()
                [usize::from(hud_selected_slot.min(8))]
            .map(|state| state.item);
            let hover = stack.and_then(|stack| {
                let effective = world.effective_item_stack(stack);
                item_runtime.and_then(|rt| styled_hover_name(rt, effective.as_ref()))
            });
            (hover, stack.and_then(|stack| stack.item_id))
        },
    );
    // Vanilla `Gui.extractOverlayMessage` / `extractTitle` read the post-tick
    // countdowns with the frame partial tick; the renderer resolves fade
    // alpha per frame from these projected timers.
    let hud_action_bar_text = context.hud_render_runtime.action_bar_update(
        world,
        item_runtime,
        state.step_generation,
        view.world_revision,
        basics.entity_partial_tick,
    );
    let hud_title_text = context.hud_render_runtime.title_text_update(
        world,
        item_runtime,
        state.step_generation,
        view.world_revision,
        basics.entity_partial_tick,
    );
    let hud_scoreboard_sidebar = context.hud_render_runtime.scoreboard_sidebar_update(
        HudScoreboardSidebarCacheKey {
            level_generation: world.client_level_generation(),
            display_revision: world.scoreboard_display_revision(),
            resource_generation: basics.resource_generation,
            resources_loaded: basics.resources_loaded,
            hidden: input.hide_gui(),
        },
        || hud_scoreboard_sidebar_from_world(input, world, item_runtime),
    );
    let hud_player_list_visible = hud_player_list_is_visible(input);
    let hud_player_list = context.hud_render_runtime.player_list_update(
        HudPlayerListCacheKey {
            level_generation: world.client_level_generation(),
            display_revision: world.player_list_display_revision(),
            profile_presentation_revision: item_runtime
                .map_or(0, NativeItemRuntime::profile_presentation_revision),
            resource_generation: basics.resource_generation,
            resources_loaded: basics.resources_loaded,
            visible: hud_player_list_visible,
            // Vanilla `PlayerTabOverlay` shows player heads only on a local
            // server or an encrypted connection; this offline-only client is
            // always plaintext, matching a vanilla client on an offline
            // server.
            show_heads: false,
        },
        || hud_player_list_from_world(input, world, item_runtime, view.client_tick_index, false),
    );
    let hud_player_list_tick_count = context
        .hud_render_runtime
        .player_list_tick_count_update(view.client_tick_index);
    let hud_chat_messages = context
        .hud_render_runtime
        .chat_messages_update(world, item_runtime);
    let hud_static_ui = extract_static_ui(
        view,
        context.hud_render_runtime,
        context.commands,
        item_runtime,
        basics,
        game_mode_can_hurt_player,
        camera_player,
        health_player,
        hud_selected_slot,
    );
    // `LerpingBossEvent` owns a 100ms wall-clock presentation curve outside
    // canonical packet state. Anchor newly published targets at the
    // simulation change time, then sample them at this render's wall clock.
    let hud_boss_bars = context.hud_render_runtime.boss_bars_update_for_height(
        world,
        item_runtime,
        view.net_counters.connection_generation,
        basics.now,
        basics.render_started_at,
        basics.stable_world_unchanged,
        Some(basics.gui_surface_size.height),
    );
    let item_registry_keys = context
        .non_entity_render_extraction
        .item_registry_keys(world);
    let hotbar_key = HudHotbarBlockItemModelsCacheKey {
        ordinary_hotbar,
        resource_generation: basics.resource_generation,
        resources_loaded: basics.resources_loaded,
        shift_down: input.shift_down(),
        keybind_context: basics.item_model_keybind_context,
    };
    // Vanilla `Gui.extractSlot` resolves the GUI item model after applying
    // the partial-tick pop pose. Keep that pose in the
    // frame-local icon projection while pop or cooldown animations are active;
    // retain both flat icons and 3D block-item models while stable.
    let (hud_hotbar_items, hud_offhand_block_item_model, hud_hotbar_block_item_models) = {
        let shift_down = input.shift_down();
        context.hud_render_runtime.hotbar_items_and_models_update(
            world,
            basics.stable_world_unchanged,
            hotbar_key,
            || {
                if !ordinary_hotbar {
                    return (
                        HudHotbarItemsFrame {
                            hud_hotbar_item_icons: std::array::from_fn(|_| None),
                            hud_offhand_slot: None,
                        },
                        None,
                    );
                }
                let models = HudItemModelContext::with_registry_keys(
                    world,
                    item_runtime,
                    item_registry_keys.clone(),
                );
                let items = HudHotbarItemsFrame {
                    hud_hotbar_item_icons: hotbar_item_icons_with_input_context(
                        &models,
                        basics.entity_partial_tick,
                        shift_down,
                        basics.item_model_keybind_context,
                    ),
                    hud_offhand_slot: hud_offhand_slot(
                        &models,
                        basics.entity_partial_tick,
                        shift_down,
                        basics.item_model_keybind_context,
                    ),
                };
                (items, Some(models))
            },
            |models, has_offhand| {
                let Some(models) = models else {
                    return (None, Vec::new());
                };
                let offhand_model = has_offhand
                    .then(|| {
                        hud_offhand_block_item_model(
                            &models,
                            context.terrain_textures,
                            shift_down,
                            basics.item_model_keybind_context,
                        )
                    })
                    .flatten();
                let hotbar_models = hotbar_block_item_models(
                    &models,
                    context.terrain_textures,
                    shift_down,
                    basics.item_model_keybind_context,
                );
                (offhand_model, hotbar_models)
            },
        )
    };
    let any_screen_open = input.current_screen().is_some();
    let screen_uses_partial_tick = match input.current_screen() {
        Some(Screen::Container(_)) => {
            world.local_inventory_is_open()
                || world
                    .inventory()
                    .open_container
                    .as_ref()
                    .is_some_and(|container| container.mount.is_some())
                || !world.cooldowns().is_empty()
        }
        Some(Screen::Creative(state)) => {
            world
                .creative_inventory_snapshot()
                .and_then(|snapshot| state.selected_tab(snapshot))
                .is_some_and(|tab| {
                    tab.tab_type == bbb_client_world::CreativeInventoryTabType::Inventory
                })
                || !world.cooldowns().is_empty()
        }
        Some(Screen::Stats(_) | Screen::Dialog(_) | Screen::Advancements(_)) => true,
        _ => false,
    };
    let screen_uses_panorama = matches!(input.current_screen(), Some(Screen::Disconnected(_)))
        || (basics.level_loading_screen_is_up
            && world.level_load().reason != WorldLevelLoadReason::EndPortal)
        || basics.idle_hint_screen_is_up;
    let screens_key = HudScreensCacheKey {
        step_generation: state.step_generation,
        world_revision: view.world_revision,
        client_tick_index: view.client_tick_index,
        level_generation: world.client_level_generation(),
        resource_generation: basics.resource_generation,
        resources_loaded: basics.resources_loaded,
        terrain_textures_epoch: basics.terrain_textures_epoch,
        gui_surface_size: basics.gui_surface_size,
        gui_cursor_bits: any_screen_open
            .then_some(context.gui_cursor_position)
            .flatten()
            .map(|pos| (pos.x.to_bits(), pos.y.to_bits())),
        hud_cursor_position: matches!(
            input.current_screen(),
            Some(Screen::ChatRestrictions(_) | Screen::Social(_))
        )
        .then_some(basics.hud_cursor_position)
        .flatten(),
        partial_tick_bits: screen_uses_partial_tick.then(|| basics.entity_partial_tick.to_bits()),
        panorama_rotation_bits: screen_uses_panorama
            .then(|| basics.panorama_rotation_degrees.to_bits()),
        win_screen_resources_ptr: matches!(input.current_screen(), Some(Screen::GameEvent(_)))
            .then(|| Arc::as_ptr(view.win_screen_resources) as usize),
    };
    let (hud_screens, actual_pause_menu) =
        context
            .hud_render_runtime
            .screens_update(screens_key, |hud_render_runtime| {
                extract_active_screens(
                    view,
                    hud_render_runtime,
                    context.renderer,
                    item_runtime,
                    context.terrain_textures,
                    context.entity_variant_textures,
                    context.gui_cursor_position,
                    context.extensions,
                    basics,
                )
            });
    let toasts_started_at = basics.profiling_enabled.then(Instant::now);
    let periodic_notification = context
        .hud_render_runtime
        .periodic_notifications
        .update(basics.now)
        .cloned();
    let toast_sounds = context
        .hud_render_runtime
        .client_toasts
        .update_with_options(
            world,
            item_runtime,
            context.renderer.hud_text_measurements_ref(),
            periodic_notification,
            context.notification_display_time,
            basics.now,
        );
    if let Some(audio_events) = context.audio_events.as_deref_mut() {
        for sound in &toast_sounds {
            audio_events.play_local_sound(sound);
        }
    }
    let hud_toasts = context.hud_render_runtime.toasts_update(|client_toasts| {
        client_toasts.project_toasts(
            world,
            item_runtime,
            context.terrain_textures,
            basics.item_model_keybind_context,
            basics.entity_partial_tick,
        )
    });
    let raw_now_playing_toast = context
        .hud_render_runtime
        .now_playing_toast
        .update_and_project_pause_menu(
            world,
            item_runtime,
            context.music_toast_display,
            context.final_music_volume,
            context.notification_display_time,
            actual_pause_menu,
            view.client_tick_index,
            basics.now,
        );
    let hud_now_playing_toast = context
        .hud_render_runtime
        .now_playing_toast_update(raw_now_playing_toast);
    let toasts_duration = optional_profile_elapsed(toasts_started_at);
    let hud_duration = hud_started_at.elapsed();
    ExtractedHudAndScreens {
        hud_player_health,
        hud_jump_bar,
        hud_experience_progress,
        hud_hotbar_mode,
        hud_selected_item_name,
        hud_action_bar_text,
        hud_title_text,
        hud_scoreboard_sidebar,
        hud_player_list,
        hud_player_list_tick_count,
        hud_chat_messages,
        hud_static_ui,
        hud_boss_bars,
        item_registry_keys,
        hud_hotbar_items,
        hud_offhand_block_item_model,
        hud_hotbar_block_item_models,
        hud_screens,
        hud_toasts,
        hud_now_playing_toast,
        toasts_duration,
        hud_duration,
    }
}

#[allow(clippy::too_many_arguments)]
fn extract_static_ui(
    view: &SimulationView<'_>,
    hud_render_runtime: &mut HudRenderRuntime,
    commands: &CommandQueue,
    item_runtime: Option<&NativeItemRuntime>,
    basics: &FrameBasics,
    game_mode_can_hurt_player: bool,
    camera_player: Option<HudCameraPlayerVitals>,
    health_player: Option<HudCameraPlayerVitals>,
    hud_selected_slot: u8,
) -> RendererFrameUpdate<HudStaticUiFrame> {
    let input = view.input;
    let world = view.world;
    let local_player = world.local_player();
    hud_render_runtime.static_ui_update(
        HudStaticUiCacheKey {
            step_generation: view.frame.step_generation,
            world_revision: view.world_revision,
            client_tick_index: view.client_tick_index,
            level_generation: world.client_level_generation(),
            chat_delivery_revision: world.client_chat().delivery_revision,
            chat_display_refresh_revision: world.client_chat().display_refresh_revision,
            resource_generation: basics.resource_generation,
            resources_loaded: basics.resources_loaded,
            gui_surface_size: basics.gui_surface_size,
            hud_cursor_position: basics.hud_cursor_position,
        },
        |hud_rt| {
            // Unlike ordinary HUD content, Vanilla extracts the sleep overlay outside
            // the `Options.hideGui` branch and does not partial-tick interpolate it.
            let hud_sleep_overlay = hud_sleep_overlay_from_world(world);
            // `Gui.extractHotbarAndDecorations` reaches `extractFood` only through
            // `GameMode.canHurtPlayer`; spectator/creative HUDs must not retain the
            // ordinary food row merely because synced health is still present.
            let hud_food = health_player.map(|player| player.food);
            // Vanilla `Gui.extractArmor` reads `player.getArmorValue()`,
            // the floor of the ARMOR attribute; the renderer gates the row on `armor > 0`.
            let hud_armor = health_player.map(|player| player.armor);
            // Vanilla `Gui.extractAirBubbles` inputs: the synced
            // `DATA_AIR_SUPPLY_ID` int, the fixed 300-tick `getMaxAirSupply()`, the
            // `isEyeInFluid(WATER)` gate, and the client tick for the all-empty
            // wobble; the renderer applies the `isUnderWater || air < max` visibility.
            let hud_air = health_player.map(|player| HudAirSupply {
                air: player.air,
                max_air: player.max_air,
                eye_in_water: player.eye_in_water,
                tick_count: view.client_tick_index,
            });
            // Vanilla `Gui.playAirBubblePoppedSound` plays on the camera player, so a
            // camera set to another player pops at that entity's position.
            if let Some((player, air)) = camera_player.zip(hud_air) {
                let position = if player.is_local_player {
                    world.local_player_pose().map(|pose| pose.position)
                } else {
                    player
                        .entity_id
                        .and_then(|entity_id| world.probe_entity_transform(entity_id))
                        .map(|transform| transform.position)
                };
                if let Some(position) = position {
                    if let Some(sound) = hud_rt.air_bubble_pop_sound(
                        air,
                        !input.hide_gui()
                            && game_mode_can_hurt_player
                            && !basics.level_loading_screen_is_up,
                    ) {
                        commands.push(
                            crate::simulation::commands::SimulationCommand::HudAirBubblePopSound {
                                level_generation: world.client_level_generation(),
                                position,
                                volume: sound.volume,
                                pitch: sound.pitch,
                            },
                        );
                    }
                }
            }
            // Vanilla `Gui.extractVehicleHealth` inputs: the camera
            // player's living vehicle health + MAX_HEALTH attribute; the renderer
            // derives the heart count (`getVehicleMaxHearts`) and replaces the food
            // row while it is non-zero.
            let hud_vehicle_health =
                camera_player
                    .and_then(|player| player.vehicle)
                    .map(|vehicle| HudVehicleHealth {
                        health: vehicle.health,
                        max_health: vehicle.max_health as f32,
                    });
            // Vanilla `Gui.extractFood` starvation-shake / hunger-swap inputs
            // (958): saturation-empty gate, the Hunger potion flag, and
            // the client `tickCount` (bbb's `LightmapTickState.client_tick_count`, the
            // same per-client-tick counter the lightmap uses).
            let hud_food_effect = HudFoodEffect {
                saturation_empty: camera_player.is_some_and(|player| player.saturation <= 0.0),
                hunger_effect: camera_player.is_some_and(|player| player.hungry),
                tick_count: view.client_tick_index,
            };
            let hud_mob_effects = hud_mob_effects_from_world(world, item_runtime);
            let hud_inventory_shows_mob_effects = world.local_inventory_is_open();
            // Gui.extractHotbarAndDecorations also requires GameMode.hasExperience (survival/adventure).
            let hud_experience_level = game_mode_can_hurt_player
                .then(|| local_player.experience.map(|experience| experience.level))
                .flatten();
            let first_person = input.camera_type().is_first_person();
            let scoping_visible =
                !input.hide_gui() && first_person && world.local_player_is_scoping();
            let hud_camera_overlays =
                hud_camera_overlays(world, input.hide_gui(), first_person, scoping_visible);
            let queued_chat_messages = world.client_chat().delayed_messages.len();
            let queued_chat_messages_text = (queued_chat_messages > 0).then(|| {
                localized_gui_text_with_args(
                    item_runtime,
                    "chat.queue",
                    "%s messages waiting",
                    &[&queued_chat_messages.to_string()],
                )
            });
            let hud_chat_context = HudChatContext {
                level_generation: world.client_level_generation(),
                delivery_revision: world.client_chat().delivery_revision,
                chat_screen_revision: input.chat_screen_revision(),
                resource_generation: basics.resource_generation,
                resources_loaded: basics.resources_loaded,
                gui_ticks: world.client_chat().gui_ticks,
                focused: input.chat_entry_is_active(),
                advanced_item_tooltips: input.debug_advanced_item_tooltips(),
                scroll: input.chat_scroll(),
                cursor_position: basics.hud_cursor_position,
                queued_messages: queued_chat_messages,
                queue_text: queued_chat_messages_text,
                queue_tooltip_text: (queued_chat_messages > 0).then(|| {
                    localized_gui_text(
                        item_runtime,
                        "chat.queue.tooltip",
                        "Click to display next message",
                    )
                }),
                restricted_text: (input.chat_entry_is_active()
                    && world.chat_presentation_config().visibility
                        != bbb_client_world::ChatVisibilityState::Full)
                    .then(|| {
                        localized_gui_text(
                            item_runtime,
                            "chat_screen.restricted",
                            "Chat is restricted. Click this message for details.",
                        )
                    }),
            };
            HudStaticUiFrame {
                hide_gui: input.hide_gui(),
                hud_sleep_overlay,
                hud_food,
                hud_food_effect,
                hud_mob_effects,
                hud_inventory_shows_mob_effects,
                hud_armor,
                hud_air,
                hud_vehicle_health,
                hud_experience_level,
                hud_selected_slot,
                hud_camera_overlays,
                hud_chat_context,
            }
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn extract_active_screens(
    view: &SimulationView<'_>,
    hud_render_runtime: &mut HudRenderRuntime,
    renderer: &Renderer,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    entity_variant_textures: &NativeEntityVariantTextureRuntime,
    gui_cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
    extensions: crate::qol_extensions::ClientExtensionSet,
    basics: &FrameBasics,
) -> Box<HudScreensFrame> {
    let input = view.input;
    let world = view.world;
    let mut screens = Box::<HudScreensFrame>::default();
    hud_render_runtime.screen_narrations = ScreenNarrationFrames::default();
    let (hud_screen_present, hud_ingame_ui_present, hud_screen_blur) = match input.current_screen()
    {
        Some(Screen::CommandConfirmation(_)) => {
            screens.hud_command_confirmation = super::hud_command_confirmation(input, item_runtime);
            if let Some(screen) = screens.hud_command_confirmation.as_mut() {
                renderer.refresh_hud_command_confirmation_hover(
                    screen,
                    basics.gui_surface_size,
                    gui_cursor_position.map(|position| (position.x, position.y)),
                );
            }
            hud_render_runtime.screen_narrations.confirmation = screens
                .hud_command_confirmation
                .as_ref()
                .map(|hud| command_confirmation_narration_frame(input, hud, item_runtime));
            (screens.hud_command_confirmation.is_some(), false, false)
        }
        Some(Screen::Disconnected(_)) => {
            screens.hud_disconnected_screen = super::hud_disconnected_screen(
                input,
                item_runtime,
                basics.panorama_rotation_degrees,
            );
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Disconnected,
                disconnected_screen_narration_frame(input, item_runtime),
            );
            let visible = screens.hud_disconnected_screen.is_some();
            (visible, false, visible)
        }
        Some(Screen::CommandEntry(_)) => {
            screens.hud_command_block_editor_screen = super::hud_command_block_editor_screen(
                input,
                item_runtime,
                basics.gui_surface_size,
            );
            if input.command_suggestions_active() {
                screens.hud_command_suggestions = super::hud_command_suggestions(
                    input,
                    world,
                    renderer,
                    item_runtime,
                    basics.gui_surface_size,
                );
            }
            if let Some(screen) = screens.hud_command_block_editor_screen.as_ref() {
                hud_render_runtime.screen_narrations.set(
                    ScreenNarrationTarget::CommandBlock,
                    Some(command_block_editor_narration_frame(
                        input,
                        screen,
                        item_runtime,
                    )),
                );
            } else {
                hud_render_runtime.screen_narrations.set(
                    ScreenNarrationTarget::Chat,
                    chat_screen_narration_frame(input, world, item_runtime),
                );
            }
            let visible = screens.hud_command_block_editor_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::ChatRestrictions(_)) => {
            screens.hud_chat_restrictions_screen = super::hud_chat_restrictions_screen(
                input,
                world,
                item_runtime,
                basics.hud_cursor_position,
            );
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Chat,
                chat_screen_narration_frame(input, world, item_runtime),
            );
            let visible = screens.hud_chat_restrictions_screen.is_some();
            (visible, false, visible)
        }
        Some(Screen::Sign(_)) => {
            screens.hud_sign_editor_screen =
                super::hud_sign_editor_screen(input, world, basics.gui_surface_size);
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Sign,
                screens
                    .hud_sign_editor_screen
                    .as_ref()
                    .map(|screen| sign_editor_narration_frame(screen, item_runtime)),
            );
            let visible = screens.hud_sign_editor_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::Death(_)) => {
            screens.hud_death_screen = super::hud_death_screen(input, item_runtime);
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Death,
                death_screen_narration_frame(input, item_runtime),
            );
            (
                screens.hud_death_screen.is_some(),
                false,
                matches!(
                    screens.hud_death_screen.as_ref(),
                    Some(bbb_client_render::HudDeathScreen::LinkConfirmation(_))
                ),
            )
        }
        Some(Screen::GameEvent(event)) => {
            screens.hud_game_event_screen = super::hud_game_event_screen(
                hud_render_runtime,
                input,
                view.win_screen_resources,
                item_runtime,
            );
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::GameEvent,
                screens.hud_game_event_screen.as_ref().map(|screen| {
                    game_event_screen_narration_frame(
                        screen,
                        view.win_screen_resources,
                        item_runtime,
                    )
                }),
            );
            (
                screens.hud_game_event_screen.is_some(),
                false,
                !matches!(event, GameEventScreenRuntime::Win(_)),
            )
        }
        Some(Screen::Social(_)) => {
            screens.hud_social_interactions_screen = super::hud_social_interactions_screen(
                input,
                world,
                item_runtime,
                basics.hud_cursor_position,
                Some(basics.now),
            );
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::SocialInteractions,
                screens
                    .hud_social_interactions_screen
                    .as_ref()
                    .and_then(|_| social_interactions_narration_frame(input, world, item_runtime)),
            );
            (
                screens.hud_social_interactions_screen.is_some(),
                false,
                false,
            )
        }
        Some(Screen::DebugOptions(_)) => {
            screens.hud_debug_options_screen = super::hud_debug_options_screen(
                input,
                world,
                item_runtime,
                basics.gui_surface_size,
            );
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::DebugOptions,
                screens.hud_debug_options_screen.as_ref().map(|screen| {
                    debug_options_screen_narration_frame(
                        input,
                        screen,
                        item_runtime,
                        basics.gui_surface_size,
                    )
                }),
            );
            (screens.hud_debug_options_screen.is_some(), false, false)
        }
        Some(Screen::Pause(_)) => {
            screens.hud_pause_screen =
                super::hud_pause_screen(input, world, item_runtime, basics.gui_surface_size);
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Pause,
                screens
                    .hud_pause_screen
                    .as_ref()
                    .map(|screen| pause_screen_narration_frame(screen, item_runtime)),
            );
            (screens.hud_pause_screen.is_some(), false, true)
        }
        Some(Screen::GameRules(screen)) => {
            screens.hud_game_rules_screen = Some(super::hud_game_rules_screen(
                screen,
                item_runtime,
                basics.gui_surface_size,
            ));
            (true, false, true)
        }
        Some(Screen::Stats(_)) => {
            if let Some(stats) = input.stats_screen() {
                let screen = super::hud_stats_screen(
                    stats,
                    world,
                    item_runtime,
                    terrain_textures,
                    basics.item_model_keybind_context,
                    basics.entity_partial_tick,
                    basics.gui_surface_size,
                );
                hud_render_runtime.screen_narrations.set(
                    ScreenNarrationTarget::Stats,
                    Some(stats_screen_narration_frame(
                        stats,
                        &screen,
                        item_runtime,
                        basics.gui_surface_size,
                    )),
                );
                screens.hud_stats_screen = Some(screen);
                (true, false, true)
            } else {
                (false, false, false)
            }
        }
        Some(Screen::StructureBlock(_)) => {
            screens.hud_structure_block_editor_screen = super::hud_structure_block_editor_screen(
                input,
                item_runtime,
                basics.gui_surface_size,
            );
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::StructureBlock,
                screens
                    .hud_structure_block_editor_screen
                    .as_ref()
                    .map(|screen| structure_block_editor_narration_frame(screen, item_runtime)),
            );
            let visible = screens.hud_structure_block_editor_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::JigsawBlock(_)) => {
            screens.hud_jigsaw_block_editor_screen =
                super::hud_jigsaw_block_editor_screen(input, item_runtime, basics.gui_surface_size);
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::JigsawBlock,
                screens
                    .hud_jigsaw_block_editor_screen
                    .as_ref()
                    .map(|screen| jigsaw_block_editor_narration_frame(screen, item_runtime)),
            );
            let visible = screens.hud_jigsaw_block_editor_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::TestBlock(_)) => {
            screens.hud_test_block_editor_screen =
                super::hud_test_block_editor_screen(input, item_runtime, basics.gui_surface_size);
            let visible = screens.hud_test_block_editor_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::TestInstanceBlock(_)) => {
            screens.hud_test_instance_block_editor_screen =
                super::hud_test_instance_block_editor_screen(
                    input,
                    item_runtime,
                    basics.gui_surface_size,
                );
            let visible = screens.hud_test_instance_block_editor_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::Book(_)) => {
            screens.hud_book_screen = super::hud_book_screen(input, world, item_runtime);
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Book,
                screens
                    .hud_book_screen
                    .as_ref()
                    .map(|screen| book_screen_narration_frame(screen, item_runtime)),
            );
            let visible = screens.hud_book_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::Container(_)) => {
            screens.hud_book_screen = super::hud_book_screen(input, world, item_runtime);
            if let Some(book) = screens.hud_book_screen.as_ref() {
                hud_render_runtime.screen_narrations.set(
                    ScreenNarrationTarget::Book,
                    Some(book_screen_narration_frame(book, item_runtime)),
                );
                (true, true, false)
            } else {
                screens.hud_inventory_screen = hud_inventory_screen_for_surface(
                    world,
                    item_runtime,
                    terrain_textures,
                    input.inventory_hovered_slot(),
                    InventoryHudLocalState {
                        stonecutter_recipe_scroll_row: Some(input.stonecutter_recipe_scroll_row()),
                        beacon_effect_selection: Some(input.beacon_effect_selection()),
                        loom_pattern_scroll_row: Some(input.loom_pattern_scroll_row()),
                        loom_selected_pattern_index: input.loom_selected_pattern_index(),
                        anvil_rename_text: Some(input.anvil_rename_text()),
                        anvil_rename_field: input.anvil_rename_field().cloned(),
                        recipe_book_search: input.recipe_book_search_hud_state(),
                        recipe_book_tabs: input.recipe_book_tab_selection_hud_state(),
                        recipe_book_pages: input.recipe_book_page_hud_state(),
                        recipe_book_presentation: input.recipe_book_presentation_hud_state(),
                        recipe_book_overlay: input.recipe_book_overlay_hud_state(),
                        cursor_position: input.inventory_cursor_position(),
                        merchant_focused_trade_row: input.merchant_focused_trade_row(),
                        quick_craft_button_num: input.inventory_quick_craft_button_num(),
                        quick_craft_slots: input.inventory_quick_craft_slots().to_vec(),
                        shift_down: input.shift_down(),
                        keybind_context: basics.item_model_keybind_context,
                        advanced_item_tooltips: input.debug_advanced_item_tooltips(),
                        item_inspector: extensions.item_inspector,
                        smithing: hud_render_runtime.smithing_screen.frame(
                            input.dialog_screen_revision(),
                            world,
                            item_runtime,
                            view.client_tick_index,
                            basics.entity_partial_tick,
                        ),
                    },
                    basics.gui_surface_size,
                    renderer.hud_font_book_ref(),
                    Some(entity_variant_textures),
                    input.enchantment_book_hud_state(basics.entity_partial_tick),
                    basics.entity_partial_tick,
                );
                if let Some(inventory) = screens.hud_inventory_screen.as_ref() {
                    screens.hud_inventory_bundle_tooltip = super::hud_inventory_bundle_tooltip(
                        world,
                        item_runtime,
                        input.inventory_hovered_slot(),
                        inventory.tooltip.as_ref(),
                        input.shift_down(),
                        basics.item_model_keybind_context,
                        basics.entity_partial_tick,
                    );
                    if let Some(recipe_book) = recipe_book_narration_frame(
                        input,
                        world,
                        item_runtime,
                        basics.gui_surface_size,
                        basics.entity_partial_tick,
                    ) {
                        hud_render_runtime
                            .screen_narrations
                            .set(ScreenNarrationTarget::RecipeBook, Some(recipe_book));
                    } else {
                        hud_render_runtime.screen_narrations.set(
                            ScreenNarrationTarget::Inventory,
                            Some(container_inventory_narration_frame(
                                input,
                                world,
                                inventory,
                                item_runtime,
                            )),
                        );
                    }
                }
                let visible = screens.hud_inventory_screen.is_some();
                (visible, visible, false)
            }
        }
        Some(Screen::Creative(_)) => {
            let creative_state = input.creative_inventory_hud_state(world);
            screens.hud_inventory_screen = creative_state.as_ref().and_then(|state| {
                hud_creative_inventory_screen(
                    world,
                    item_runtime,
                    terrain_textures,
                    state.clone(),
                    input.shift_down(),
                    basics.item_model_keybind_context,
                    input.debug_advanced_item_tooltips(),
                    extensions.item_inspector,
                    basics.gui_surface_size,
                    renderer.hud_font_book_ref().glyphs(None),
                    basics.entity_partial_tick,
                )
            });
            if let Some(inventory) = screens.hud_inventory_screen.as_ref() {
                screens.hud_inventory_bundle_tooltip = creative_state.as_ref().and_then(|state| {
                    super::hud_creative_inventory_bundle_tooltip(
                        world,
                        item_runtime,
                        state,
                        inventory.tooltip.as_ref(),
                        input.shift_down(),
                        basics.item_model_keybind_context,
                        basics.entity_partial_tick,
                    )
                });
                hud_render_runtime.screen_narrations.set(
                    ScreenNarrationTarget::Inventory,
                    creative_state.as_ref().and_then(|state| {
                        creative_inventory_narration_frame(world, item_runtime, state)
                    }),
                );
            }
            let visible = screens.hud_inventory_screen.is_some();
            (visible, visible, false)
        }
        Some(Screen::DialogLink(link)) => {
            screens.hud_inventory_screen = Some(hud_dialog_confirm_link_screen(
                link,
                link.focused_button,
                input.inventory_cursor_position(),
                basics.gui_surface_size,
            ));
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Dialog,
                dialog_screen_narration_frame(input, world, item_runtime),
            );
            (
                true,
                true,
                input.dialog_warning_is_open() || input.current_dialog().is_some(),
            )
        }
        Some(Screen::DialogWarning(focused)) => {
            screens.hud_inventory_screen = Some(hud_dialog_warning_screen(
                *focused,
                input.inventory_cursor_position(),
                basics.gui_surface_size,
            ));
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Dialog,
                dialog_screen_narration_frame(input, world, item_runtime),
            );
            (true, true, true)
        }
        Some(Screen::Dialog(_)) => {
            if let Some(dialog) = input.current_dialog() {
                screens.hud_inventory_screen = dialog_screen::hud_dialog_screen_with_measurer(
                    input,
                    world,
                    dialog,
                    item_runtime,
                    terrain_textures,
                    &dialog_screen::DialogHudLocalState {
                        dialog_ime_preedit: input.dialog_ime_preedit_hud_state(),
                        cursor_position: input.inventory_cursor_position(),
                        shift_down: input.shift_down(),
                        keybind_context: basics.item_model_keybind_context,
                        advanced_item_tooltips: input.debug_advanced_item_tooltips(),
                    },
                    basics.gui_surface_size,
                    renderer,
                    basics.entity_partial_tick,
                );
                let visible = screens.hud_inventory_screen.is_some();
                if visible {
                    hud_render_runtime.screen_narrations.set(
                        ScreenNarrationTarget::Dialog,
                        dialog_screen_narration_frame(input, world, item_runtime),
                    );
                }
                (visible, visible, visible)
            } else {
                (false, false, false)
            }
        }
        Some(Screen::DialogWaiting(ticks)) => {
            screens.hud_inventory_screen = Some(hud_waiting_response_screen(
                *ticks,
                input.inventory_cursor_position(),
                basics.gui_surface_size,
            ));
            hud_render_runtime.screen_narrations.set(
                ScreenNarrationTarget::Dialog,
                dialog_screen_narration_frame(input, world, item_runtime),
            );
            (true, true, true)
        }
        Some(Screen::Advancements(_)) => {
            if let Some(screen) = input.advancements_screen() {
                screens.hud_inventory_screen = Some(hud_advancements_screen(
                    world,
                    item_runtime,
                    terrain_textures,
                    screen.scroll_delta(world.selected_advancements_tab()),
                    screen.hover_fade(),
                    basics.gui_surface_size,
                    input.inventory_cursor_position(),
                    basics.item_model_keybind_context,
                    basics.entity_partial_tick,
                    renderer.hud_font_book_ref(),
                ));
                (true, false, true)
            } else {
                (false, false, false)
            }
        }
        Some(Screen::GameModeSwitcher(_)) | None => (false, false, false),
    };
    screens.hud_screen_present = hud_screen_present;
    screens.hud_ingame_ui_present = hud_ingame_ui_present;
    screens.hud_screen_blur = hud_screen_blur;
    screens.hud_level_loading_screen = screens
        .hud_disconnected_screen
        .is_none()
        .then(|| {
            super::hud_level_loading_screen(
                world,
                terrain_textures,
                item_runtime,
                basics.panorama_rotation_degrees,
            )
            .or_else(|| {
                basics
                    .idle_hint_screen_is_up
                    .then(|| bbb_client_render::HudLevelLoadingScreen {
                        background: bbb_client_render::HudLevelLoadingBackground::Panorama {
                            rotation_degrees: basics.panorama_rotation_degrees,
                        },
                        downloading_terrain: "Idle - launch with --server <host:port> to connect"
                            .to_string(),
                    })
            })
        })
        .flatten();
    screens
}
