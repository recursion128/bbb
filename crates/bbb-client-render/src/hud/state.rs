use super::*;

#[inline]
fn set_hud_field<T: PartialEq>(slot: &mut T, next: T, generation: &mut u64) {
    if *slot != next {
        *slot = next;
        *generation = generation.wrapping_add(1);
    }
}

impl Renderer {
    #[inline]
    pub(crate) fn mark_hud_dirty(&mut self) {
        self.hud_generation = self.hud_generation.wrapping_add(1);
    }

    pub fn set_hud_screen_presence(&mut self, screen_present: bool, ingame_ui_present: bool) {
        if self.hud_screen_present != screen_present
            || self.hud_ingame_ui_present != ingame_ui_present
        {
            self.hud_screen_present = screen_present;
            self.hud_ingame_ui_present = ingame_ui_present;
            self.mark_hud_dirty();
        }
    }

    /// Updates the selected language's paragraph fallback after a font/resource
    /// generation is installed. Logical input measurements remain unchanged;
    /// completed display lines and their cached interaction geometry are rebuilt.
    pub fn set_default_text_right_to_left(&mut self, default_right_to_left: bool) {
        if self.pack.hud_font_glyphs.default_right_to_left() == default_right_to_left {
            return;
        }
        self.pack
            .hud_font_glyphs
            .set_default_right_to_left(default_right_to_left);
        self.pack
            .hud_font_book
            .set_default_right_to_left(default_right_to_left);
        self.rebuild_chat_lines();
        self.mark_hud_dirty();
    }

    pub fn default_text_right_to_left(&self) -> bool {
        self.pack.hud_font_glyphs.default_right_to_left()
    }

    pub fn set_hud_locator_styles(&mut self, styles: BTreeMap<String, HudLocatorStyle>) {
        set_hud_field(
            &mut self.pack.hud_locator_styles,
            styles,
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_code_of_conduct_screen(&mut self, screen: Option<HudCodeOfConductScreen>) {
        set_hud_field(
            &mut self.hud_code_of_conduct_screen,
            screen.and_then(code_of_conduct::sanitize_hud_code_of_conduct_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_sound_subtitles(&mut self, subtitles: HudSoundSubtitles) {
        set_hud_field(
            &mut self.hud_sound_subtitles,
            sound_subtitles::sanitize_hud_sound_subtitles(subtitles),
            &mut self.hud_generation,
        );
    }

    /// Projects `Gui.extractPlayerHealth`'s inputs (health, max health,
    /// absorption, base heart type, hardcore, regen, tick). `collect_hud_draws`
    /// derives the container/absorption rows, `numHealthRows`, the regen wave,
    /// and the low-health shake. Non-finite health/max/absorption (malformed
    /// projection) clear the row.
    pub fn set_hud_player_health(&mut self, health: Option<HudPlayerHealth>) {
        set_hud_field(
            &mut self.hud_player_health,
            health
                .filter(|health| {
                    health.health.is_finite()
                        && health.max_health.is_finite()
                        && health.absorption.is_finite()
                })
                .map(super::layout::clamp_hud_player_health),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_sleep_overlay(&mut self, overlay: Option<HudSleepOverlay>) {
        set_hud_field(
            &mut self.hud_sleep_overlay,
            sanitize_hud_sleep_overlay(overlay),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_spyglass_scope(&mut self, scope: Option<HudSpyglassScope>) {
        set_hud_field(
            &mut self.hud_spyglass_scope,
            scope.filter(|scope| scope.scale.is_finite() && scope.scale >= 0.0),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_crosshair_attack_indicator(&mut self, indicator: HudCrosshairAttackIndicator) {
        let next = match indicator {
            HudCrosshairAttackIndicator::Progress { strength_scale }
                if strength_scale.is_finite() =>
            {
                HudCrosshairAttackIndicator::Progress {
                    strength_scale: strength_scale.clamp(0.0, 1.0),
                }
            }
            HudCrosshairAttackIndicator::Progress { .. } => HudCrosshairAttackIndicator::None,
            indicator => indicator,
        };
        set_hud_field(
            &mut self.hud_crosshair_attack_indicator,
            next,
            &mut self.hud_generation,
        );
    }

    /// Applies Vanilla `Gui.extractCrosshair`'s camera/game-mode/interactive
    /// target gate without discarding the uploaded sprite owner.
    pub fn set_hud_crosshair_visible(&mut self, visible: bool) {
        set_hud_field(
            &mut self.hud_crosshair_visible,
            visible,
            &mut self.hud_generation,
        );
    }

    /// Applies Vanilla's transient F1 `Options.hideGui` presentation policy.
    ///
    /// The renderer retains every latest HUD, screen, toast, and first-person
    /// owner while hidden so toggling the GUI back on restores unchanged state
    /// immediately.
    pub fn set_hide_gui(&mut self, hide_gui: bool) {
        set_hud_field(&mut self.hide_gui, hide_gui, &mut self.hud_generation);
    }

    pub fn set_hud_food(&mut self, food: Option<i32>) {
        set_hud_field(&mut self.hud_food, food, &mut self.hud_generation);
    }

    /// Projects `Gui.extractArmor`'s `player.getArmorValue()`; the
    /// draw is gated on `armor > 0` in `collect_hud_draws`, matching vanilla's
    /// visibility test.
    pub fn set_hud_armor(&mut self, armor: Option<i32>) {
        set_hud_field(&mut self.hud_armor, armor, &mut self.hud_generation);
    }

    /// Projects `Gui.extractAirBubbles`'s inputs (air supply, max, eye-in-water,
    /// tick); the visibility gate (`isUnderWater || air < max`)
    /// is applied in `collect_hud_draws`. A non-positive max (malformed
    /// projection) clears the row rather than dividing by zero.
    pub fn set_hud_air(&mut self, air: Option<HudAirSupply>) {
        set_hud_field(
            &mut self.hud_air,
            air.filter(|air| air.max_air > 0),
            &mut self.hud_generation,
        );
    }

    /// Projects the local player's living vehicle health pair
    /// (`Gui.extractVehicleHealth` inputs); `collect_hud_draws` derives the
    /// heart count via `hud_vehicle_max_hearts` and suppresses the food row
    /// while it is non-zero. Non-finite values (malformed
    /// projection) clear the bar.
    pub fn set_hud_vehicle_health(&mut self, vehicle: Option<HudVehicleHealth>) {
        set_hud_field(
            &mut self.hud_vehicle_health,
            vehicle.filter(|vehicle| vehicle.health.is_finite() && vehicle.max_health.is_finite()),
            &mut self.hud_generation,
        );
    }

    /// Installs the one bar selected by native `Gui.nextContextualInfoState`
    /// projection. Experience-level text remains an independent field.
    pub fn set_hud_contextual_bar(&mut self, contextual_bar: HudContextualBar) {
        set_hud_field(
            &mut self.hud_contextual_bar,
            sanitize_hud_contextual_bar(contextual_bar),
            &mut self.hud_generation,
        );
    }

    /// Projects this frame's food-bar effect state (starvation-shake gate and
    /// hunger potion sprite swap); the food level is set by `set_hud_food`.
    pub fn set_hud_food_effect(&mut self, effect: HudFoodEffect) {
        set_hud_field(&mut self.hud_food_effect, effect, &mut self.hud_generation);
    }

    pub fn set_hud_mob_effects(
        &mut self,
        mut effects: Vec<HudMobEffect>,
        inventory_shows_effects: bool,
    ) {
        retain_map_in_place_bounded(&mut effects, 40, |effect| {
            Some(sanitize_hud_mob_effect(effect))
        });
        if self.hud_mob_effects != effects
            || self.hud_inventory_shows_mob_effects != inventory_shows_effects
        {
            self.hud_mob_effects = effects;
            self.hud_inventory_shows_mob_effects = inventory_shows_effects;
            self.mark_hud_dirty();
        }
    }

    /// Projects the experience level to render as centered text above the bar.
    /// Vanilla only draws it when `experienceLevel > 0`
    /// so non-positive levels clear the text.
    pub fn set_hud_experience_level(&mut self, level: Option<i32>) {
        set_hud_field(
            &mut self.hud_experience_level,
            hud_experience_level_projection(level),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_hotbar_mode(&mut self, mode: HudHotbarMode) {
        set_hud_field(
            &mut self.hud_hotbar_mode,
            spectator::sanitize_hud_hotbar_mode(mode),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_selected_slot(&mut self, slot: u8) {
        set_hud_field(
            &mut self.hud_selected_slot,
            slot.min((HUD_HOTBAR_SLOTS - 1) as u8),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_hotbar_item_uvs(&mut self, uvs: [Option<HudUvRect>; HUD_HOTBAR_SLOTS]) {
        set_hud_field(
            &mut self.hud_hotbar_item_icons,
            uvs.map(|uv| uv.and_then(sanitize_hud_hotbar_item_uv)),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_hotbar_item_icons(&mut self, icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS]) {
        set_hud_field(
            &mut self.hud_hotbar_item_icons,
            icons.map(|icon| icon.and_then(sanitize_hud_item_icon)),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_offhand_slot(&mut self, slot: Option<HudOffhandSlot>) {
        let next = slot.and_then(|slot| {
            sanitize_hud_item_icon(slot.icon).map(|icon| HudOffhandSlot {
                on_left: slot.on_left,
                icon,
            })
        });
        set_hud_field(&mut self.hud_offhand_slot, next, &mut self.hud_generation);
    }

    pub fn set_hud_offhand_block_item_model(&mut self, model: Option<HudBlockItemModel>) {
        let was_some = self.hud_offhand_block_item_model.is_some();
        let is_some = model.is_some();
        self.hud_offhand_block_item_model = model;
        if was_some != is_some {
            self.mark_hud_dirty();
        }
    }

    pub fn set_hud_hotbar_attack_indicator(&mut self, indicator: HudHotbarAttackIndicator) {
        let next = match indicator {
            HudHotbarAttackIndicator::Progress {
                strength_scale,
                main_arm_left,
            } if strength_scale.is_finite() => HudHotbarAttackIndicator::Progress {
                strength_scale: strength_scale.clamp(0.0, 1.0),
                main_arm_left,
            },
            HudHotbarAttackIndicator::Progress { .. } => HudHotbarAttackIndicator::None,
            HudHotbarAttackIndicator::None => HudHotbarAttackIndicator::None,
        };
        set_hud_field(
            &mut self.hud_hotbar_attack_indicator,
            next,
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_inventory_screen(&mut self, screen: Option<HudInventoryScreen>) {
        if screen.is_none() {
            set_hud_field(
                &mut self.hud_inventory_bundle_tooltip,
                None,
                &mut self.hud_generation,
            );
        }
        set_hud_field(
            &mut self.hud_inventory_screen,
            screen.map(sanitize_hud_inventory_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_inventory_bundle_tooltip(&mut self, bundle: Option<HudBundleTooltip>) {
        set_hud_field(
            &mut self.hud_inventory_bundle_tooltip,
            bundle.map(sanitize_hud_bundle_tooltip),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_inventory_screen_with_bundle_tooltip(
        &mut self,
        screen: Option<HudInventoryScreen>,
        bundle: Option<HudBundleTooltip>,
    ) {
        let sanitized_screen = screen.map(sanitize_hud_inventory_screen);
        let sanitized_bundle = sanitized_screen
            .as_ref()
            .and_then(|screen| screen.tooltip.as_ref())
            .and(bundle)
            .map(sanitize_hud_bundle_tooltip);
        set_hud_field(
            &mut self.hud_inventory_screen,
            sanitized_screen,
            &mut self.hud_generation,
        );
        set_hud_field(
            &mut self.hud_inventory_bundle_tooltip,
            sanitized_bundle,
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_sign_editor_screen(&mut self, screen: Option<HudSignEditorScreen>) {
        set_hud_field(
            &mut self.hud_sign_editor_screen,
            screen.and_then(sanitize_hud_sign_editor_screen),
            &mut self.hud_generation,
        );
    }

    /// The sign editor's native IME caret in logical GUI coordinates. The
    /// caller converts back to physical window pixels at the platform
    /// boundary (`Window.guiScaledWidth` ratio); `surface_size` here is the
    /// renderer's `gui_surface_size()`.
    pub fn hud_sign_editor_ime_cursor_area(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
        let projection = hud_sign_editor_preedit_projection(
            self.hud_sign_editor_screen.as_ref()?,
            surface_size,
            &self.pack.hud_font_glyphs,
        )?;
        Some((projection.ime_position, projection.ime_size))
    }

    pub fn set_hud_command_block_editor_screen(
        &mut self,
        screen: Option<HudCommandBlockEditorScreen>,
    ) {
        set_hud_field(
            &mut self.hud_command_block_editor_screen,
            screen.and_then(sanitize_hud_command_block_editor_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_structure_block_editor_screen(
        &mut self,
        screen: Option<HudStructureBlockEditorScreen>,
    ) {
        set_hud_field(
            &mut self.hud_structure_block_editor_screen,
            screen.and_then(sanitize_hud_structure_block_editor_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_jigsaw_block_editor_screen(
        &mut self,
        screen: Option<HudJigsawBlockEditorScreen>,
    ) {
        set_hud_field(
            &mut self.hud_jigsaw_block_editor_screen,
            screen.and_then(sanitize_hud_jigsaw_block_editor_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_test_block_editor_screen(&mut self, screen: Option<HudTestBlockEditorScreen>) {
        set_hud_field(
            &mut self.hud_test_block_editor_screen,
            screen.and_then(sanitize_hud_test_block_editor_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_test_instance_block_editor_screen(
        &mut self,
        screen: Option<HudTestInstanceBlockEditorScreen>,
    ) {
        set_hud_field(
            &mut self.hud_test_instance_block_editor_screen,
            screen.and_then(sanitize_hud_test_instance_block_editor_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_pause_screen(&mut self, screen: Option<HudPauseScreen>) {
        set_hud_field(
            &mut self.hud_pause_screen,
            screen.and_then(sanitize_hud_pause_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_stats_screen(&mut self, screen: Option<HudStatsScreen>) {
        set_hud_field(
            &mut self.hud_stats_screen,
            screen.and_then(sanitize_hud_stats_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_game_rules_screen(&mut self, screen: Option<HudGameRulesScreen>) {
        set_hud_field(
            &mut self.hud_game_rules_screen,
            screen.and_then(game_rules::sanitize_hud_game_rules_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_command_confirmation(&mut self, screen: Option<HudCommandConfirmation>) {
        set_hud_field(
            &mut self.hud_command_confirmation,
            screen,
            &mut self.hud_generation,
        );
    }

    pub fn refresh_hud_command_confirmation_hover(
        &self,
        screen: &mut HudCommandConfirmation,
        size: PhysicalSize<u32>,
        position: Option<(f64, f64)>,
    ) {
        command_confirmation::refresh_hover(screen, &self.pack.hud_font_book, size, position);
    }

    pub fn hud_command_confirmation_hit_target_at(
        &self,
        position: (f64, f64),
    ) -> Option<HudCommandConfirmationHitTarget> {
        command_confirmation::hit_target_at(
            self.hud_command_confirmation.as_ref()?,
            &self.pack.hud_font_book,
            self.gui_surface_size(),
            position,
        )
    }

    pub fn set_hud_death_screen(&mut self, screen: Option<HudDeathScreen>) {
        set_hud_field(
            &mut self.hud_death_screen,
            screen.and_then(death_screen::sanitize_hud_death_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_disconnected_screen(&mut self, screen: Option<HudDisconnectedScreen>) {
        set_hud_field(
            &mut self.hud_disconnected_screen,
            screen.and_then(disconnected_screen::sanitize_hud_disconnected_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_level_loading_screen(&mut self, screen: Option<HudLevelLoadingScreen>) {
        set_hud_field(
            &mut self.hud_level_loading_screen,
            screen.and_then(level_loading_screen::sanitize_hud_level_loading_screen),
            &mut self.hud_generation,
        );
    }

    pub fn hud_disconnected_screen_hit_target_at(
        &self,
        position: (f64, f64),
    ) -> Option<HudDisconnectedScreenHitTarget> {
        disconnected_screen::hud_disconnected_screen_hit_target_at(
            self.hud_disconnected_screen.as_ref()?,
            &self.pack.hud_font_book,
            self.gui_surface_size(),
            position,
        )
    }

    pub fn hud_death_screen_hit_target_at(
        &self,
        position: (f64, f64),
    ) -> Option<HudDeathScreenHitTarget> {
        death_screen::hud_death_screen_hit_target_at(
            self.hud_death_screen.as_ref()?,
            &self.pack.hud_font_book,
            self.gui_surface_size(),
            position,
        )
    }

    pub fn set_hud_game_event_screen(&mut self, screen: Option<HudGameEventScreen>) {
        set_hud_field(
            &mut self.hud_game_event_screen,
            screen.and_then(game_event::sanitize_hud_game_event_screen),
            &mut self.hud_generation,
        );
    }

    pub fn hud_game_event_hit_target_at(
        &self,
        position: (f64, f64),
    ) -> Option<HudGameEventHitTarget> {
        game_event::hud_game_event_hit_target_at(
            self.hud_game_event_screen.as_ref()?,
            &self.pack.hud_font_glyphs,
            self.gui_surface_size(),
            position,
        )
    }

    pub fn set_hud_book_screen(&mut self, screen: Option<HudBookScreen>) {
        set_hud_field(
            &mut self.hud_book_screen,
            screen.and_then(book::sanitize_hud_book_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_toasts(&mut self, mut toasts: Vec<HudToast>) {
        let mut occupied = [false; HUD_TOAST_SLOTS];
        retain_map_in_place_bounded(&mut toasts, HUD_TOAST_SLOTS, |toast| {
            sanitize_hud_toast(toast, &mut occupied)
        });
        set_hud_field(&mut self.hud_toasts, toasts, &mut self.hud_generation);
    }

    pub fn set_hud_now_playing_toast(&mut self, toast: Option<HudNowPlayingToast>) {
        set_hud_field(
            &mut self.hud_now_playing_toast,
            toast.and_then(sanitize_hud_now_playing_toast),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_system_toast(&mut self, toast: Option<HudSystemToast>) {
        self.set_hud_system_toasts(
            toast
                .map(|toast| HudSystemToastInstance { toast, slot: 0 })
                .into_iter()
                .collect(),
        );
    }

    pub fn set_hud_system_toasts(&mut self, toasts: Vec<HudSystemToastInstance>) {
        let mut combined = self.hud_toasts.clone();
        combined.retain(|toast| !matches!(toast, HudToast::System(_)));
        combined.extend(toasts.into_iter().map(HudToast::System));
        self.set_hud_toasts(combined);
    }

    pub fn set_hud_advancement_toasts(&mut self, toasts: Vec<HudAdvancementToast>) {
        let mut combined = self.hud_toasts.clone();
        combined.retain(|toast| !matches!(toast, HudToast::Advancement(_)));
        combined.extend(toasts.into_iter().map(HudToast::Advancement));
        self.set_hud_toasts(combined);
    }

    pub fn set_hud_recipe_toasts(&mut self, toasts: Vec<HudRecipeToast>) {
        let mut combined = self.hud_toasts.clone();
        combined.retain(|toast| !matches!(toast, HudToast::Recipe(_)));
        combined.extend(toasts.into_iter().map(HudToast::Recipe));
        self.set_hud_toasts(combined);
    }

    pub fn set_hud_tutorial_toasts(&mut self, toasts: Vec<HudTutorialToast>) {
        let mut combined = self.hud_toasts.clone();
        combined.retain(|toast| !matches!(toast, HudToast::Tutorial(_)));
        combined.extend(toasts.into_iter().map(HudToast::Tutorial));
        self.set_hud_toasts(combined);
    }

    pub fn set_hud_debug_options_screen(&mut self, screen: Option<HudDebugOptionsScreen>) {
        set_hud_field(
            &mut self.hud_debug_options_screen,
            screen.and_then(sanitize_hud_debug_options_screen),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_action_bar_text(&mut self, action_bar: Option<HudActionBarText>) {
        set_hud_field(
            &mut self.hud_action_bar_text,
            action_bar.filter(|state| state.partial_tick.is_finite()),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_selected_item_name(&mut self, name: Option<HudSelectedItemName>) {
        set_hud_field(
            &mut self.hud_selected_item_name,
            name.filter(|state| state.highlight_ticks > 0 && !state.runs.is_empty()),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_title_text(&mut self, title: Option<HudTitleText>) {
        set_hud_field(
            &mut self.hud_title_text,
            title.filter(|state| state.partial_tick.is_finite()),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_scoreboard_sidebar(&mut self, sidebar: Option<HudScoreboardSidebar>) {
        set_hud_field(
            &mut self.hud_scoreboard_sidebar,
            sidebar.map(scoreboard::sanitize_hud_scoreboard_sidebar),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_player_list(&mut self, player_list: Option<HudPlayerList>) {
        let next = player_list.map(player_list::sanitize_hud_player_list);
        if self.hud_player_list != next {
            self.hud_player_list = next;
            self.mark_hud_dirty();
        }
        self.hud_player_list_runtime
            .synchronize(self.hud_player_list.as_ref());
    }

    /// Advances vanilla's GUI-tick heart presentation without replacing the
    /// stable player-list entries projected by native.
    pub fn set_hud_player_list_tick_count(&mut self, tick_count: u64) {
        let Some(player_list) = self.hud_player_list.as_mut() else {
            return;
        };
        if player_list.tick_count == tick_count {
            return;
        }
        player_list.tick_count = tick_count;
        self.hud_player_list_runtime
            .synchronize(self.hud_player_list.as_ref());
        self.mark_hud_dirty();
    }

    /// Replaces the stable chat source entries and rebuilds the bounded
    /// renderer-owned wrapped-line cache once. Ordinary tick/fade frames use
    /// [`set_hud_chat_context`](Self::set_hud_chat_context) and never clone or
    /// remeasure the rich message payload.
    pub fn set_hud_chat_messages(&mut self, messages: Vec<HudChatMessage>) {
        self.replace_hud_chat_messages(messages, false);
    }

    /// Replaces the chat source entries after the world re-displayed every one
    /// of them, as `ChatComponent.refreshTrimmedMessages` does.
    pub fn refresh_hud_chat_messages(&mut self, messages: Vec<HudChatMessage>) {
        self.replace_hud_chat_messages(messages, true);
    }

    fn replace_hud_chat_messages(&mut self, mut messages: Vec<HudChatMessage>, refreshed: bool) {
        let skip = messages.len().saturating_sub(HUD_CHAT_MAX_MESSAGES);
        if skip > 0 {
            messages.drain(..skip);
        }
        retain_map_in_place(&mut messages, |message| {
            Some(sanitize_hud_chat_message(message))
        });
        let previous = std::mem::replace(&mut self.hud_chat_messages, messages);
        let previous_lines = self.hud_chat_lines.len();
        let message_lines = self.rebuild_chat_lines();
        if let Some(appended) = HudChatAppendedLines::replaced(
            &previous,
            previous_lines,
            &self.hud_chat_messages,
            &message_lines,
            refreshed,
        ) {
            self.hud_chat_appended_lines = Some(match self.hud_chat_appended_lines.take() {
                Some(pending) => pending.then(appended),
                None => appended,
            });
        }
    }

    /// The trimmed lines the source replacements since the last call appended,
    /// for the simulation-owned scroll position.
    pub fn take_hud_chat_appended_lines(&mut self) -> Option<HudChatAppendedLines> {
        self.hud_chat_appended_lines.take()
    }

    /// Startup chat page options; a changed wrap width rewraps the cached lines
    /// as `rescaleChat` does.
    pub fn set_hud_chat_options(&mut self, options: HudChatOptions) {
        let options = options.sanitized();
        if self.hud_chat_options == options {
            return;
        }
        let rewrap = options.wrap_width() != self.hud_chat_options.wrap_width();
        self.hud_chat_options = options;
        if rewrap {
            self.rebuild_chat_lines();
        } else {
            self.mark_hud_dirty();
        }
    }

    pub(super) fn hud_chat_geometry(&self) -> HudChatGeometry {
        HudChatGeometry::new(self.hud_chat_options, self.gui_surface_size())
    }

    /// The focused page `ChatComponent.scrollChat` clamps against for the
    /// current wrapped lines.
    pub fn hud_chat_scroll_page(&self) -> HudChatScrollPage {
        self.hud_chat_geometry()
            .scroll_page(self.hud_chat_lines.len())
    }

    pub fn set_hud_chat_context(&mut self, mut context: HudChatContext) {
        context.scroll.position = context.scroll.position.min(self.hud_chat_lines.len());
        context.queued_messages = context.queued_messages.min(1_024);
        context.queue_text = context
            .queue_text
            .map(|text| sanitize_hud_text_preserving_empty(text, 256))
            .filter(|text| !text.is_empty());
        context.queue_tooltip_text = context
            .queue_tooltip_text
            .map(|text| sanitize_hud_text_preserving_empty(text, 256))
            .filter(|text| !text.is_empty());
        context.restricted_text = context
            .restricted_text
            .map(|text| sanitize_hud_text_preserving_empty(text, 256))
            .filter(|text| !text.is_empty());
        set_hud_field(
            &mut self.hud_chat_context,
            context,
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_chat_restrictions_screen(&mut self, screen: Option<HudChatRestrictionsScreen>) {
        set_hud_field(
            &mut self.hud_chat_restrictions_screen,
            screen.and_then(sanitize_hud_chat_restrictions_screen),
            &mut self.hud_generation,
        );
    }

    pub(super) fn rebuild_chat_lines(&mut self) -> Vec<usize> {
        let (lines, message_lines) = hud_wrap_chat_message_lines(
            &self.hud_chat_messages,
            &self.pack.hud_font_book,
            self.hud_chat_options.wrap_width(),
        );
        self.hud_chat_lines = lines;
        self.mark_hud_dirty();
        message_lines
    }

    /// Resolves a focused-chat pointer against the exact cached line wrapping
    /// used for drawing. The returned revision token is intentionally narrow:
    /// simulation must still revalidate the target against canonical chat
    /// state before executing a component action.
    pub fn hud_chat_hit_target_at(&self, position: (f64, f64)) -> Option<HudChatHitTarget> {
        if let Some(screen) = self.hud_chat_restrictions_screen.as_ref() {
            if hud_chat_restrictions_done_contains(self.gui_surface_size(), position) {
                return Some(HudChatHitTarget {
                    level_generation: self.hud_chat_context.level_generation,
                    delivery_revision: self.hud_chat_context.delivery_revision,
                    chat_screen_revision: screen.generation,
                    resource_generation: self.hud_chat_context.resource_generation,
                    resources_loaded: self.hud_chat_context.resources_loaded,
                    kind: HudChatHitTargetKind::RestrictionsDone,
                });
            }
            return None;
        }
        hud_chat_hit_target_at(
            &self.hud_chat_lines,
            &self.hud_chat_context,
            &self.pack.hud_font_book,
            self.hud_chat_geometry(),
            position,
        )
    }

    pub fn hud_book_hit_target_at(&self, position: (f64, f64)) -> Option<HudBookHitTarget> {
        book::hud_book_hit_target_at(
            self.hud_book_screen.as_ref()?,
            &self.pack.hud_font_book,
            self.gui_surface_size(),
            position,
        )
    }

    pub fn hud_chat_ime_cursor_area(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
        let suggestions = self.hud_command_suggestions.as_ref()?;
        let glyphs = match suggestions.layout {
            HudCommandSuggestionsLayout::Chat => self.pack.hud_font_book.non_fishy_glyphs(None),
            HudCommandSuggestionsLayout::CommandBlock => &self.pack.hud_font_glyphs,
        };
        let projection = chat_input::preedit_projection(suggestions, surface_size, glyphs)?;
        Some((projection.ime_position, projection.ime_size))
    }

    /// Logical GUI IME cursor/preedit rectangle for whichever active HUD screen
    /// currently owns text input.
    pub fn hud_ime_cursor_area(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
        if let Some(area) = self.hud_sign_editor_ime_cursor_area(surface_size) {
            return Some(area);
        }
        if let Some(suggestions) = self.hud_command_suggestions.as_ref() {
            let glyphs = match suggestions.layout {
                HudCommandSuggestionsLayout::Chat => self.pack.hud_font_book.non_fishy_glyphs(None),
                HudCommandSuggestionsLayout::CommandBlock => &self.pack.hud_font_glyphs,
            };
            if let Some(area) = chat_input::ime_cursor_area(suggestions, surface_size, glyphs) {
                return Some(area);
            }
        }
        ime_preedit::active_screen_ime_cursor_area(self, surface_size)
    }

    pub fn set_hud_command_suggestions(&mut self, suggestions: Option<HudCommandSuggestions>) {
        set_hud_field(
            &mut self.hud_command_suggestions,
            suggestions.map(sanitize_hud_command_suggestions),
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_debug_overlay(&mut self, overlay: Option<HudDebugOverlay>) {
        set_hud_field(
            &mut self.hud_debug_overlay,
            overlay.and_then(sanitize_hud_debug_overlay),
            &mut self.hud_generation,
        );
    }

    /// Replaces this frame's boss bars (the world's projection of vanilla
    /// `BossHealthOverlay.events`), sanitizing each bar's progress.
    pub fn set_hud_boss_bars(&mut self, mut bars: Vec<HudBossBar>) {
        retain_map_in_place(&mut bars, |bar| Some(sanitize_hud_boss_bar(bar)));
        set_hud_field(&mut self.hud_boss_bars, bars, &mut self.hud_generation);
    }

    fn gui_sprite_concat<'a>(
        &'a self,
        prefix: &str,
        name: &str,
        suffix: &str,
    ) -> Option<&'a HudSpriteGpu> {
        let total = prefix.len() + name.len() + suffix.len();
        let mut buf = [0u8; 96];
        if total <= buf.len() {
            buf[..prefix.len()].copy_from_slice(prefix.as_bytes());
            buf[prefix.len()..prefix.len() + name.len()].copy_from_slice(name.as_bytes());
            buf[prefix.len() + name.len()..total].copy_from_slice(suffix.as_bytes());
            let key = std::str::from_utf8(&buf[..total]).ok()?;
            self.gui_sprite(key)
        } else {
            self.gui_sprite(&format!("{prefix}{name}{suffix}"))
        }
    }

    /// Resolves one food-icon sprite, honoring the Hunger potion swap: under the
    /// effect it prefers the uploaded `food_*_hunger` variant, falling back to
    /// the base sprite when the hunger variant is not loaded (vanilla
    /// `Gui.extractFood` selects the sprite id, Gui).
    pub(super) fn hud_food_variant_sprite(
        &self,
        which: HudFoodSprite,
        hunger: bool,
    ) -> Option<&HudSpriteGpu> {
        let (hunger_sprite, base_sprite) = match which {
            HudFoodSprite::Empty => (
                self.gui_sprite("hud/food_empty_hunger"),
                self.gui_sprite("hud/food_empty"),
            ),
            HudFoodSprite::Half => (
                self.gui_sprite("hud/food_half_hunger"),
                self.gui_sprite("hud/food_half"),
            ),
            HudFoodSprite::Full => (
                self.gui_sprite("hud/food_full_hunger"),
                self.gui_sprite("hud/food_full"),
            ),
        };
        hud_food_sprite_variant(hunger, hunger_sprite, base_sprite)
    }

    /// One player-heart sprite by `(kind, hardcore, half, blinking)` (vanilla
    /// `HeartType.getSprite`). `Container`'s half normalizes to full.
    pub(super) fn hud_heart_sprite(
        &self,
        kind: HudHeartKind,
        hardcore: bool,
        half: bool,
        blinking: bool,
    ) -> Option<&HudSpriteGpu> {
        let half = half && !matches!(kind, HudHeartKind::Container);
        self.gui_sprite_concat("hud/heart/", kind.sprite_name(hardcore, half, blinking), "")
    }

    /// The uploaded 182x5 sheet backing one bar layer (vanilla
    /// `BAR_{BACKGROUND,PROGRESS}_SPRITES` / `OVERLAY_*_SPRITES` lookups,
    pub(super) fn hud_boss_bar_sheet_sprite(
        &self,
        sheet: HudBossBarSheet,
    ) -> Option<&HudSpriteGpu> {
        match sheet {
            HudBossBarSheet::ColorBackground(color) => {
                self.gui_sprite_concat("boss_bar/", color.name(), "_background")
            }
            HudBossBarSheet::ColorProgress(color) => {
                self.gui_sprite_concat("boss_bar/", color.name(), "_progress")
            }
            HudBossBarSheet::NotchedBackground(overlay) => overlay
                .notched_index()
                .and_then(|_| self.gui_sprite_concat("boss_bar/", overlay.name(), "_background")),
            HudBossBarSheet::NotchedProgress(overlay) => overlay
                .notched_index()
                .and_then(|_| self.gui_sprite_concat("boss_bar/", overlay.name(), "_progress")),
        }
    }

    pub fn clear_hud_inventory_screen(&mut self) {
        set_hud_field(
            &mut self.hud_inventory_screen,
            None,
            &mut self.hud_generation,
        );
        set_hud_field(
            &mut self.hud_inventory_bundle_tooltip,
            None,
            &mut self.hud_generation,
        );
    }

    pub fn clear_hud_sign_editor_screen(&mut self) {
        set_hud_field(
            &mut self.hud_sign_editor_screen,
            None,
            &mut self.hud_generation,
        );
    }

    pub fn clear_hud_pause_screen(&mut self) {
        set_hud_field(&mut self.hud_pause_screen, None, &mut self.hud_generation);
    }

    pub fn clear_hud_stats_screen(&mut self) {
        set_hud_field(&mut self.hud_stats_screen, None, &mut self.hud_generation);
    }

    pub fn clear_hud_debug_options_screen(&mut self) {
        set_hud_field(
            &mut self.hud_debug_options_screen,
            None,
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_confusion_overlay(&mut self, overlay: Option<HudConfusionOverlay>) {
        let next = overlay
            .filter(|overlay| overlay.strength.is_finite() && overlay.strength > 0.0)
            .map(|overlay| HudConfusionOverlay {
                strength: overlay.strength.min(1.0),
            });
        set_hud_field(
            &mut self.hud_confusion_overlay,
            next,
            &mut self.hud_generation,
        );
    }

    pub fn set_hud_portal_overlay(&mut self, overlay: Option<HudPortalOverlay>) {
        let next = overlay.filter(|overlay| {
            overlay.intensity.is_finite()
                && overlay.intensity > 0.0
                && sanitize_hud_uv_rect(overlay.sprite_uv).is_some()
        });
        set_hud_field(&mut self.hud_portal_overlay, next, &mut self.hud_generation);
    }

    /// Replaces this frame's ordered `Gui.extractTextureOverlay` calls. An
    /// overlay whose texture is not resident is dropped rather than drawn with
    /// a stand-in.
    pub fn set_hud_camera_overlays(&mut self, mut overlays: Vec<HudCameraOverlay>) {
        overlays.retain_mut(|overlay| {
            if overlay.alpha.is_finite() && overlay.alpha > 0.0 {
                overlay.alpha = overlay.alpha.min(1.0);
                true
            } else {
                false
            }
        });
        set_hud_field(
            &mut self.hud_camera_overlays,
            overlays,
            &mut self.hud_generation,
        );
    }
}
