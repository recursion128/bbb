use super::*;

impl SimulationInputRuntime {
    pub(super) fn apply_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        clipboard_text: Option<String>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        self.keyboard_events = self.keyboard_events.saturating_add(1);
        if physical_key == PhysicalKey::Code(KeyCode::F9) {
            if state == ElementState::Pressed {
                context.platform.submit(PlatformEffect::ToggleRecording);
            }
            return;
        }
        if matches!(state, ElementState::Pressed) {
            self.keyboard_press_events = self.keyboard_press_events.saturating_add(1);
        }
        if matches!(physical_key, PhysicalKey::Code(KeyCode::KeyT)) {
            self.key_t_events = self.key_t_events.saturating_add(1);
        }
        // `KeyboardHandler.keyPress` answers the
        // fullscreen and screenshot keys before it hands the event to
        // `screen.keyPressed`, so every screen dispatch below runs after this
        // one rather than instead of it. Without this ordering a screen that
        // swallows unrecognised keys -- the pause screen does -- would make F11
        // and F2 dead for as long as it is open.
        if context.input.handle_global_key_press(physical_key, state) {
            self.apply_debug_requests(context);
            return;
        }

        if self.code_of_conduct_overlay.is_visible(context.world)
            && context.input.command_confirmation().is_none()
            && context.input.disconnected_screen_runtime().is_none()
            && !context.input.chat_restrictions_screen_is_open()
        {
            let overlay_input = self.code_of_conduct_overlay.handle_key_input(
                context.world,
                physical_key,
                state,
                self.gui_surface_size(),
                &self.text_measurements,
            );
            if let Some(request) = overlay_input.request {
                self.apply_code_of_conduct_control(request, context);
            }
            self.request_cursor_capture(false, context.platform);
            return;
        }
        let inventory_key_event = context.input.inventory_screen_is_open()
            && inventory_screen_layout(context.world).is_some();
        let queued_effects = RefCell::new(Vec::new());
        let mut clipboard = SimulationClipboard {
            read: clipboard_text,
            effects: &queued_effects,
        };
        let mut terrain_reload = SimulationTerrainReload {
            effects: &queued_effects,
        };
        let debug_key_handled = crate::input::key_events::dispatch_key_input_with_gameplay_effects(
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime.as_ref(),
            physical_key,
            state,
            self.gui_surface_size(),
            &self.text_measurements,
            Some(&mut clipboard),
            Some(&mut terrain_reload),
            &mut self.cursor_captured,
            context.platform,
            context.first_person_hand_presentation,
            context.entity_partial_tick,
            context.level_random,
            context.audio_events,
        );
        if inventory_key_event && !debug_key_handled {
            self.inventory_keyboard_events = self.inventory_keyboard_events.saturating_add(1);
            if matches!(
                (physical_key, state),
                (PhysicalKey::Code(KeyCode::KeyT), ElementState::Pressed)
            ) {
                self.inventory_key_t_press_events =
                    self.inventory_key_t_press_events.saturating_add(1);
            }
        }
        collect_dialog_open_uri_effects(context.input, &queued_effects);
        flush_platform_effects(&queued_effects, context.platform);
        if debug_key_handled {
            self.apply_debug_requests(context);
        }
        self.apply_recipe_book_language_requests(context);
    }

    pub(super) fn apply_debug_requests(&mut self, context: &mut SimulationInputContext<'_, '_>) {
        if context.input.wants_cursor() {
            self.request_cursor_capture(false, context.platform);
        }
        let reload_requests = context.input.take_debug_resource_pack_reload_requests();
        if reload_requests > 0 {
            context.server_resource_packs.request_reload();
            tracing::info!(
                reload_requests,
                "resource pack reload requested by debug hotkey"
            );
        }
        let dynamic_texture_dump_requests =
            context.input.take_debug_dynamic_texture_dump_requests();
        if dynamic_texture_dump_requests > 0 {
            context
                .platform
                .submit(PlatformEffect::DumpDynamicTextures {
                    requests: dynamic_texture_dump_requests,
                });
        }
        let client_option_writes = context.input.take_debug_client_option_writes();
        if !client_option_writes.is_empty() {
            context.platform.submit(PlatformEffect::SaveClientOptions {
                writes: client_option_writes,
                broadcast: self.client_information.clone(),
            });
        }
        for _ in 0..context.input.take_screenshot_requests() {
            context.platform.submit(PlatformEffect::GrabScreenshot);
        }
        for fullscreen in context.input.take_fullscreen_requests() {
            context
                .platform
                .submit(PlatformEffect::SetFullscreen(fullscreen));
        }
        self.flush_narrator_requests(context);
        let profiling_toggle_requests = context.input.take_debug_profiling_toggle_requests();
        if !profiling_toggle_requests.is_empty() {
            context.platform.submit(PlatformEffect::ProfilingToggled {
                commands: debug_profiling_commands(profiling_toggle_requests),
            });
        }
        let profiler_navigation = context
            .input
            .take_debug_profiler_chart_navigation_requests();
        if !profiler_navigation.is_empty() {
            context.platform.submit(PlatformEffect::NavigateProfiler {
                digits: profiler_navigation,
            });
        }
        let frustum_requests = context.input.take_debug_frustum_requests();
        if !frustum_requests.is_empty() {
            let captures = frustum_requests
                .iter()
                .filter(|request| matches!(request, DebugFrustumRequest::Capture))
                .count();
            let kills = frustum_requests
                .iter()
                .filter(|request| matches!(request, DebugFrustumRequest::Kill))
                .count();
            tracing::info!(
                requests = frustum_requests.len(),
                captures,
                kills,
                "debug frustum hotkey requested; native captured frustum is not implemented"
            );
        }
        let feature_count_requests = context.input.take_debug_feature_count_requests();
        if !feature_count_requests.is_empty() {
            let logs = feature_count_requests
                .iter()
                .filter(|request| matches!(request, DebugFeatureCountRequest::Log))
                .count();
            let clears = feature_count_requests
                .iter()
                .filter(|request| matches!(request, DebugFeatureCountRequest::Clear))
                .count();
            tracing::info!(
                requests = feature_count_requests.len(),
                logs,
                clears,
                "feature-count debug hotkey requested; native FeatureCountTracker is not implemented"
            );
        }
        let debug_options_requests = context.input.take_debug_options_screen_requests();
        for _ in 0..debug_options_requests {
            if context.input.debug_options_screen_is_open() {
                context.input.close_debug_options_screen();
            } else {
                release_active_input(
                    context.input,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
                context
                    .input
                    .open_debug_options_screen(context.net_counters, context.net_commands);
            }
        }
        if debug_options_requests > 0 {
            self.request_cursor_capture(false, context.platform);
            tracing::info!(
                debug_options_requests,
                open = context.input.debug_options_screen_is_open(),
                "debug options screen toggled by debug hotkey"
            );
        }
        let recreate_requests = context
            .input
            .take_debug_recreate_server_query_requests_for_level(
                context.world.client_level_generation(),
            );
        let recreate_request_count = recreate_requests.len();
        for request in recreate_requests {
            queue_debug_recreate_server_query_request(
                context.net_counters,
                context.net_commands,
                request,
            );
        }
        if recreate_request_count > 0 {
            tracing::info!(
                recreate_request_count,
                "debug recreate server query requested by F3+I"
            );
        }
        // Vanilla `pauseGame(debugModifierDown)` only suppresses the menu when
        // an unpublished singleplayer server can really pause; an external
        // multiplayer client always opens the full pause menu, so F3+Esc and
        // Esc share one with-menu path.
        let pause_screen_requests = context.input.take_debug_pause_screen_requests();
        if pause_screen_requests > 0 {
            release_active_input(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
            );
            context.input.open_debug_pause_screen_with_menu(
                context.world,
                context.net_counters,
                context.net_commands,
            );
            self.request_cursor_capture(false, context.platform);
            tracing::info!(
                pause_screen_requests,
                "opened pause screen requested by debug hotkey"
            );
        }
    }

    pub(super) fn apply_recipe_book_language_requests(
        &mut self,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        let requests = context.input.take_recipe_book_language_requests();
        let target_available = context
            .item_runtime
            .as_ref()
            .is_some_and(|runtime| runtime.has_available_language(RECIPE_BOOK_LANGUAGE_TARGET));
        self.recipe_book_language_requests = self
            .recipe_book_language_requests
            .saturating_add(u64::from(requests));
        self.recipe_book_language_target_available = target_available;
        if apply_recipe_book_language_transition(
            requests,
            target_available,
            &mut self.client_information,
            context.server_resource_packs,
            context.platform,
        ) {
            self.recipe_book_language_transitions =
                self.recipe_book_language_transitions.saturating_add(1);
        }
    }

    /// The element that currently owns pre-edit rendering, in the same
    /// priority order every pre-edit consumer uses.
    pub(super) fn ime_preedit_target(
        &self,
        context: &mut SimulationInputContext<'_, '_>,
    ) -> ImePreeditTarget {
        if !context.input.is_focused() || self.code_of_conduct_overlay.is_visible(context.world) {
            return ImePreeditTarget::None;
        }
        match context.input.current_screen() {
            Some(
                Screen::Dialog(_)
                | Screen::DialogWarning(_)
                | Screen::DialogLink(_)
                | Screen::DialogWaiting(_),
            ) => ImePreeditTarget::Dialog,
            Some(Screen::Sign(_)) => ImePreeditTarget::SignEditor,
            Some(Screen::CommandEntry(_)) if context.input.chat_entry_is_active() => {
                ImePreeditTarget::Chat
            }
            Some(Screen::CommandEntry(_)) if context.input.command_entry_is_active() => {
                ImePreeditTarget::CommandBlock
            }
            Some(Screen::Book(book)) => {
                book.input.as_ref().map_or(ImePreeditTarget::None, |input| {
                    ImePreeditTarget::BookEditor(input.mode)
                })
            }
            Some(Screen::Container(_)) => {
                if crate::input::recipe_book_search_is_active(context.input, context.world) {
                    ImePreeditTarget::RecipeBookSearch
                } else if crate::input::anvil_screen_is_open(context.world) {
                    ImePreeditTarget::AnvilRename
                } else {
                    ImePreeditTarget::None
                }
            }
            Some(Screen::Creative(state)) => {
                if context
                    .world
                    .creative_inventory_snapshot()
                    .is_some_and(|snapshot| state.search_is_active(snapshot))
                {
                    ImePreeditTarget::CreativeSearch
                } else {
                    ImePreeditTarget::None
                }
            }
            Some(Screen::StructureBlock(editor)) => {
                ImePreeditTarget::StructureBlock(editor.focus_index())
            }
            Some(Screen::JigsawBlock(editor)) => {
                ImePreeditTarget::JigsawBlock(editor.focus_index())
            }
            Some(Screen::TestBlock(editor)) if editor.message_is_editable() => {
                ImePreeditTarget::TestBlock
            }
            Some(Screen::TestInstanceBlock(editor)) => {
                ImePreeditTarget::TestInstanceBlock(editor.focus_index())
            }
            Some(Screen::Social(_)) if context.input.social_interactions_search_is_focused() => {
                ImePreeditTarget::SocialInteractions
            }
            Some(Screen::GameRules(rules)) => match rules.focused_text_is_search() {
                Some(true) => ImePreeditTarget::GameRulesSearch,
                Some(false) => ImePreeditTarget::GameRulesInteger,
                None => ImePreeditTarget::None,
            },
            Some(Screen::DebugOptions(_))
                if context.input.debug_options_screen_search_is_focused() =>
            {
                ImePreeditTarget::DebugOptions
            }
            _ => ImePreeditTarget::None,
        }
    }

    fn submit_ime_preedit_to_target(
        &mut self,
        target: ImePreeditTarget,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        context: &mut SimulationInputContext<'_, '_>,
        release_capture_on_screen: bool,
    ) {
        match target {
            ImePreeditTarget::Chat => context.input.handle_chat_ime_preedit(text, cursor_range),
            ImePreeditTarget::CommandBlock => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context.input.handle_chat_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::Dialog => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context.input.handle_dialog_ime_preedit(
                    text,
                    cursor_range,
                    context.input.dialog_screen_revision(),
                );
            }
            ImePreeditTarget::SignEditor => {
                context.input.sync_sign_editor(
                    context.world,
                    context.net_counters,
                    context.net_commands,
                );
                context
                    .input
                    .handle_sign_editor_ime_preedit(text, cursor_range);
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
            }
            ImePreeditTarget::BookEditor(_) => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context.input.handle_book_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::AnvilRename => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_anvil_rename_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::RecipeBookSearch => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_recipe_book_search_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::CreativeSearch => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_creative_search_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::StructureBlock(_) => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_structure_block_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::JigsawBlock(_) => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_jigsaw_block_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::TestBlock => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_test_block_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::TestInstanceBlock(_) => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_test_instance_block_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::SocialInteractions => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_social_interactions_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::GameRulesSearch | ImePreeditTarget::GameRulesInteger => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_game_rules_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::DebugOptions => {
                if release_capture_on_screen {
                    self.request_cursor_capture(false, context.platform);
                }
                context
                    .input
                    .handle_debug_options_ime_preedit(text, cursor_range);
            }
            ImePreeditTarget::None => {
                context.input.clear_all_ime_preedits();
                if release_capture_on_screen
                    && self.code_of_conduct_overlay.is_visible(context.world)
                {
                    self.request_cursor_capture(false, context.platform);
                }
            }
        }
    }

    fn clear_ime_preedit_target(
        target: ImePreeditTarget,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        match target {
            ImePreeditTarget::Chat | ImePreeditTarget::CommandBlock => {
                context.input.clear_chat_ime_preedit()
            }
            ImePreeditTarget::Dialog => context.input.clear_dialog_ime_preedit(),
            ImePreeditTarget::SignEditor => context.input.clear_sign_editor_ime_preedit(),
            ImePreeditTarget::BookEditor(_) => context.input.clear_book_ime_preedit(),
            ImePreeditTarget::AnvilRename => context.input.clear_anvil_rename_ime_preedit(),
            ImePreeditTarget::RecipeBookSearch => {
                context.input.clear_recipe_book_search_ime_preedit()
            }
            ImePreeditTarget::CreativeSearch => context.input.clear_creative_search_ime_preedit(),
            ImePreeditTarget::StructureBlock(_) => {
                context.input.clear_structure_block_ime_preedit()
            }
            ImePreeditTarget::JigsawBlock(_) => context.input.clear_jigsaw_block_ime_preedit(),
            ImePreeditTarget::TestBlock => context.input.clear_test_block_ime_preedit(),
            ImePreeditTarget::TestInstanceBlock(_) => {
                context.input.clear_test_instance_block_ime_preedit()
            }
            ImePreeditTarget::SocialInteractions => {
                context.input.clear_social_interactions_ime_preedit()
            }
            ImePreeditTarget::GameRulesSearch | ImePreeditTarget::GameRulesInteger => {
                context.input.clear_game_rules_ime_preedit()
            }
            ImePreeditTarget::DebugOptions => context.input.clear_debug_options_ime_preedit(),
            ImePreeditTarget::None => {}
        }
    }

    pub(super) fn apply_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        // Vanilla `KeyboardHandler.preeditCallback` stores the event before
        // routing, whether or not any element renders it right now.
        self.last_ime_preedit = Some((text.to_string(), cursor_range));
        let target = self.ime_preedit_target(context);
        if target != self.ime_preedit_target {
            Self::clear_ime_preedit_target(self.ime_preedit_target, context);
        }
        self.ime_preedit_target = target;
        self.submit_ime_preedit_to_target(target, text, cursor_range, context, true);
    }

    /// Vanilla `Minecraft.onTextInputFocusChange`: when the element owning
    /// pre-edit rendering changes, the losing one is cleared
    /// (`submitPreeditEvent(element, null)`) and the stored composition is
    /// resubmitted to the gaining one (`resubmitLastPreeditEvent`).
    /// `Minecraft.onTextInputFocusChange` plus `TextInputManager.tick`
    /// the window is told to accept composed
    /// text exactly while a text field has focus, and told to stop otherwise.
    pub(super) fn sync_text_input_enabled(&mut self, context: &mut SimulationInputContext<'_, '_>) {
        let enabled = context.input.text_input_is_active(context.world);
        if enabled == self.text_input_enabled {
            return;
        }
        self.text_input_enabled = enabled;
        context
            .platform
            .submit(PlatformEffect::SetTextInputEnabled(enabled));
    }

    pub(super) fn sync_ime_preedit_focus(&mut self, context: &mut SimulationInputContext<'_, '_>) {
        let target = self.ime_preedit_target(context);
        if target == self.ime_preedit_target {
            return;
        }
        Self::clear_ime_preedit_target(self.ime_preedit_target, context);
        if let Some((text, cursor_range)) = self.last_ime_preedit.clone() {
            if target != ImePreeditTarget::None {
                self.submit_ime_preedit_to_target(target, &text, cursor_range, context, false);
            }
        }
        self.ime_preedit_target = target;
    }

    pub(super) fn apply_ime_commit(
        &mut self,
        text: &str,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        // A commit ends the stored composition.
        self.last_ime_preedit = None;
        if context.input.command_confirmation().is_some() {
            return;
        }
        context.input.clear_all_ime_preedits();
        if self.code_of_conduct_overlay.is_visible(context.world) {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        crate::input::key_events::dispatch_text_input(
            context.input,
            context.net_counters,
            context.world,
            context.net_commands,
            context.item_runtime.as_ref(),
            text,
            &self.text_measurements,
            &mut self.cursor_captured,
            context.platform,
            crate::input::key_events::TextInputSource::Ime,
        );
        self.apply_recipe_book_language_requests(context);
    }
}
