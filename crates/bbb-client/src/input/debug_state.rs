use super::*;

impl ClientInputState {
    pub(crate) fn debug_overlay_visible(&self) -> bool {
        self.debug_entries.is_overlay_visible()
    }

    pub(crate) fn debug_profiler_chart_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_profiler_chart_visible
    }

    pub(crate) fn debug_profiling_recording(&self) -> bool {
        self.debug_profiling_recording
    }

    pub(crate) fn apply_debug_profiling_started(&mut self, world: &mut WorldStore) {
        self.debug_profiling_recording = true;
        push_debug_feedback_chat_message(
            Some(world),
            "Profiling started for 10 seconds. Use F3 + L to stop early",
        );
    }

    pub(crate) fn apply_debug_profiling_stopped(
        &mut self,
        world: &mut WorldStore,
        duration: Duration,
        ticks: usize,
        ticks_per_second: f64,
    ) {
        self.debug_profiling_recording = false;
        push_debug_feedback_chat_message(
            Some(world),
            &format!(
                "Stopped tick profiling after {:.2} seconds and {} ticks ({:.2} ticks per second)",
                duration.as_secs_f64(),
                ticks,
                ticks_per_second
            ),
        );
    }

    pub(crate) fn apply_debug_profiling_report_saved(
        &mut self,
        world: &mut WorldStore,
        path: &Path,
    ) {
        let path = path.display().to_string();
        push_debug_feedback_chat_runs(
            Some(world),
            vec![
                StyledTextRun {
                    text: "Profiling ended. Saved results to ".to_string(),
                    style: ComponentStyle::default(),
                },
                StyledTextRun {
                    text: path.clone(),
                    style: ComponentStyle {
                        underlined: Some(true),
                        click_event: Some(ComponentClickEvent::OpenFile { path: path.into() }),
                        ..ComponentStyle::default()
                    },
                },
            ],
        );
    }

    pub(crate) fn apply_debug_profiling_report_failed(
        &mut self,
        world: &mut WorldStore,
        error: &str,
    ) {
        push_debug_feedback_chat_message(Some(world), &format!("Profiling failed: {error}"));
    }

    pub(crate) fn debug_fps_charts_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_fps_charts_visible
    }

    pub(crate) fn debug_network_charts_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_network_charts_visible
    }

    pub(crate) fn debug_lightmap_texture_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_lightmap_texture_visible
    }

    #[cfg(test)]
    pub(crate) fn debug_entity_hitboxes_visible(&self) -> bool {
        self.debug_screen_entry_enabled(DebugScreenEntryId::EntityHitboxes, false)
    }

    #[cfg(test)]
    pub(crate) fn debug_chunk_borders_visible(&self) -> bool {
        self.debug_screen_entry_enabled(DebugScreenEntryId::ChunkBorders, false)
    }

    pub(crate) fn debug_entity_hitboxes_visible_for_world(&self, world: &WorldStore) -> bool {
        self.debug_screen_entry_enabled(
            DebugScreenEntryId::EntityHitboxes,
            self.show_only_reduced_info(world),
        )
    }

    pub(crate) fn debug_chunk_borders_visible_for_world(&self, world: &WorldStore) -> bool {
        self.debug_screen_entry_enabled(
            DebugScreenEntryId::ChunkBorders,
            self.show_only_reduced_info(world),
        )
    }

    pub(crate) fn debug_screen_entry_enabled(
        &self,
        entry: DebugScreenEntryId,
        reduced_debug_info: bool,
    ) -> bool {
        self.debug_entries
            .is_currently_enabled(entry, reduced_debug_info)
    }

    #[cfg(test)]
    pub(crate) fn set_debug_screen_entry_status(
        &mut self,
        entry: DebugScreenEntryId,
        status: DebugScreenEntryStatus,
    ) {
        self.set_debug_screen_entry_status_inner(entry, status);
    }

    pub(crate) fn debug_screen_entry_status(
        &self,
        entry: DebugScreenEntryId,
    ) -> DebugScreenEntryStatus {
        self.debug_entries.status(entry)
    }

    pub(crate) fn load_debug_screen_profile(&mut self, profile: DebugScreenProfile) {
        self.debug_entries.load_profile(profile);
        self.persist_debug_screen_entries();
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn set_debug_screen_entries(&mut self, entries: DebugScreenEntryList) {
        self.debug_entries = entries;
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn set_debug_profile_store_path(&mut self, path: PathBuf) {
        self.debug_profile_store_path = Some(path);
    }

    pub(crate) fn set_hotbar_preset_store_path(&mut self, path: PathBuf) {
        self.hotbar_preset_store = Some(HotbarPresetStore::new(path));
    }

    pub(super) fn toggle_debug_screen_entry_status(&mut self, entry: DebugScreenEntryId) -> bool {
        let enabled = self.debug_entries.toggle_status(entry);
        self.persist_debug_screen_entries();
        enabled
    }

    pub(super) fn set_debug_screen_entry_status_inner(
        &mut self,
        entry: DebugScreenEntryId,
        status: DebugScreenEntryStatus,
    ) {
        self.debug_entries.set_status(entry, status);
        self.persist_debug_screen_entries();
    }

    pub(super) fn persist_debug_screen_entries(&self) {
        let Some(path) = &self.debug_profile_store_path else {
            return;
        };
        if let Err(err) = self.debug_entries.save_to_debug_profile_file(path) {
            tracing::warn!(?err, path = %path.display(), "failed to save debug profile store");
        }
    }

    pub(crate) fn debug_advanced_item_tooltips(&self) -> bool {
        self.debug_advanced_item_tooltips
    }

    pub(crate) fn set_debug_advanced_item_tooltips(&mut self, enabled: bool) {
        self.debug_advanced_item_tooltips = enabled;
    }

    /// The `options.txt` lines a debug hotkey flipped since the last drain.
    /// `KeyboardHandler` calls `Options.save` inside each of those branches; the
    /// file owner lives on the render thread here, so the intent is queued the
    /// way every other debug-hotkey side effect is.
    pub(crate) fn take_debug_client_option_writes(&mut self) -> Vec<ClientOptionWrite> {
        std::mem::take(&mut self.debug_client_option_writes)
    }

    pub(crate) fn take_recipe_book_language_requests(&mut self) -> u32 {
        std::mem::take(&mut self.recipe_book_language_requests)
    }

    pub(crate) fn debug_show_local_server_entity_hit_boxes(&self) -> bool {
        self.debug_show_local_server_entity_hit_boxes
    }

    pub(crate) fn set_debug_show_local_server_entity_hit_boxes(&mut self, enabled: bool) {
        self.debug_show_local_server_entity_hit_boxes = enabled;
    }

    pub(crate) fn set_debug_hotkeys_enabled(&mut self, enabled: bool) {
        self.debug_hotkeys_enabled = enabled;
    }

    pub(crate) fn set_debug_feature_count_enabled(&mut self, enabled: bool) {
        self.debug_feature_count_enabled = enabled;
    }

    pub(crate) fn set_remote_debug_renderers(
        &mut self,
        renderers: crate::remote_debug::RemoteDebugRendererSelection,
    ) {
        self.remote_debug_renderers = renderers;
    }

    pub(crate) fn remote_debug_renderers(
        &self,
    ) -> crate::remote_debug::RemoteDebugRendererSelection {
        self.remote_debug_renderers
    }

    pub(crate) fn debug_fog_enabled(&self) -> bool {
        self.debug_fog_enabled
    }

    pub(crate) fn debug_smart_cull_enabled(&self) -> bool {
        self.debug_smart_cull_enabled
    }

    pub(crate) fn debug_pause_on_lost_focus(&self) -> bool {
        self.debug_pause_on_lost_focus
    }

    pub(crate) fn set_debug_pause_on_lost_focus(&mut self, enabled: bool) {
        self.debug_pause_on_lost_focus = enabled;
    }

    /// `Minecraft.pauseIfInactive`: while the window
    /// is unfocused with `pauseOnLostFocus`, opens the pause screen after
    /// 500 ms unless a screen is already open. `pauseGame` suppresses on any
    /// current screen, so the input-side all-screen
    /// predicate (including chat/command entry, debug
    /// screens, containers and the editor screens) gates it; any active frame
    /// refreshes the timestamp. The vanilla touchscreen branch is out of scope
    /// (this client has no touchscreen option consumer).
    pub(crate) fn pause_if_inactive(
        &mut self,
        now: Instant,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if !self.focused && self.debug_pause_on_lost_focus() {
            if now.saturating_duration_since(self.last_active_at.unwrap_or(now))
                > Duration::from_millis(500)
                && !self.screen.is_open()
            {
                self.open_debug_pause_screen_with_menu(world, counters, net_commands);
            }
        } else {
            self.last_active_at = Some(now);
        }
    }

    /// Test seam for `pauseIfInactive` ordering regressions: pins the
    /// last-active timestamp so the strict >500 ms delay is fully
    /// deterministic.
    #[cfg(test)]
    pub(crate) fn set_last_active_at_for_test(&mut self, at: Instant) {
        self.last_active_at = Some(at);
    }

    pub(crate) fn set_reduced_debug_info_option(&mut self, enabled: bool) {
        self.debug_reduced_debug_info_option = enabled;
    }

    /// `Minecraft.showOnlyReducedInfo`: the synced
    /// `LocalPlayer` flag or the `reducedDebugInfo` option, so the option alone
    /// still restricts the debug entries before a local player exists. The
    /// `KeyboardHandler` debug bindings deliberately read the player flag on
    /// its own instead — they are a different vanilla predicate.
    pub(crate) fn show_only_reduced_info(&self, world: &WorldStore) -> bool {
        world.local_player_has_reduced_debug_info() || self.debug_reduced_debug_info_option
    }

    pub(crate) fn take_debug_resource_pack_reload_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_resource_pack_reload_requests)
    }

    pub(crate) fn take_debug_dynamic_texture_dump_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_dynamic_texture_dump_requests)
    }

    pub(crate) fn take_debug_profiling_toggle_requests(
        &mut self,
    ) -> Vec<DebugProfilingToggleRequest> {
        std::mem::take(&mut self.debug_profiling_toggle_requests)
    }

    pub(crate) fn take_debug_profiler_chart_navigation_requests(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.debug_profiler_chart_navigation_requests)
    }

    pub(crate) fn take_debug_options_screen_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_options_screen_requests)
    }

    pub(crate) fn take_debug_pause_screen_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_pause_screen_requests)
    }

    pub(crate) fn take_debug_frustum_requests(&mut self) -> Vec<DebugFrustumRequest> {
        std::mem::take(&mut self.debug_frustum_requests)
    }

    pub(crate) fn take_debug_feature_count_requests(&mut self) -> Vec<DebugFeatureCountRequest> {
        std::mem::take(&mut self.debug_feature_count_requests)
    }
}
