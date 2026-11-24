use super::*;
use crate::narrator_runtime::ScreenNarrationFrames;
use bbb_client_render::HudVignette;

pub(super) const HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY: usize = 240;
pub(super) const HUD_DEBUG_NETWORK_SAMPLE_CAPACITY: usize = 240;
pub(super) const HUD_DEBUG_TPS_SAMPLE_CAPACITY: usize = 240;
pub(super) const HUD_DEBUG_NETWORK_SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
const BOSS_BAR_PROGRESS_LERP_DURATION: Duration = Duration::from_millis(100);
const HUD_HEALTH_DISPLAY_HOLD_DURATION: Duration = Duration::from_millis(1_000);
pub(super) fn push_capped_hud_debug_sample<T>(samples: &mut Vec<T>, value: T, capacity: usize) {
    samples.push(value);
    if samples.len() > capacity {
        let overflow = samples.len() - capacity;
        samples.drain(0..overflow);
    }
}

type CachedTitleOverlay = (
    HudOverlayTextCacheKey,
    Arc<[HudComponentTextRun]>,
    Arc<[HudComponentTextRun]>,
);
type HotbarItemsModelsUpdate = (
    RendererFrameUpdate<Box<super::render_extract::HudHotbarItemsFrame>>,
    RendererFrameUpdate<Option<HudBlockItemModel>>,
    RendererFrameUpdate<Vec<Option<HudBlockItemModel>>>,
);

#[derive(Debug, Default)]
pub(crate) struct HudRenderRuntime {
    pub(super) periodic_notifications: PeriodicNotificationRuntime,
    pub(super) client_toasts: ClientToastRuntime,
    pub(super) now_playing_toast: NowPlayingToastRuntime,
    pub(super) screen_narrations: ScreenNarrationFrames,
    win_screen_resource_owner: Option<Arc<crate::win_screen_resources::WinScreenResources>>,
    pub(super) win_screen_lines: Arc<[HudWinScreenLine]>,
    last_ui_frame_at: Option<Instant>,
    pub(super) panorama_spin_degrees: f32,
    panorama_scroll_speed: PanoramaScrollSpeed,
    static_ui_cache_key: Option<HudStaticUiCacheKey>,
    screens_cache_key: Option<HudScreensCacheKey>,
    last_actual_pause_menu: Option<bool>,
    last_inventory_screen: Option<HudInventoryScreen>,
    chat_cache_key: Option<HudChatCacheKey>,
    chat_entry_cache: super::render_extract::HudChatMessageEntryCache,
    hotbar_items_cache: Option<HudHotbarItemsCacheState>,
    hotbar_block_item_models_cache_key: Option<HudHotbarBlockItemModelsCacheKey>,
    scoreboard_sidebar_cache_key: Option<HudScoreboardSidebarCacheKey>,
    player_list_cache_key: Option<HudPlayerListCacheKey>,
    last_player_list_tick_count: Option<u64>,
    boss_generations: Option<(u64, Option<u64>)>,
    boss_bar_revision: Option<u64>,
    boss_visible_cap: Option<Option<usize>>,
    boss_bars_has_pending_lerp: bool,
    boss_progress: BTreeMap<uuid::Uuid, BossBarProgressPresentation>,
    action_bar_cache: Option<(HudOverlayTextCacheKey, Arc<[HudComponentTextRun]>)>,
    action_bar_published_none: bool,
    title_cache: Option<CachedTitleOverlay>,
    title_published_none: bool,
    toasts_published_empty: bool,
    now_playing_published_none: bool,
    debug_overlay_empty_key: Option<(u64, u64)>,
    debug_overlay_published_none: bool,
    terrain_textures_epoch: u64,
    last_crosshair_visible: Option<bool>,
    last_crosshair_attack_indicator: Option<HudCrosshairAttackIndicator>,
    last_hotbar_attack_indicator: Option<HudHotbarAttackIndicator>,
    last_published_player_health: Option<Option<HudPlayerHealth>>,
    last_vignette: Option<Option<HudVignette>>,
    last_spyglass_scope: Option<Option<HudSpyglassScope>>,
    last_portal_overlay: Option<Option<HudPortalOverlay>>,
    last_confusion_overlay: Option<Option<HudConfusionOverlay>>,
    last_contextual_bar: Option<HudContextualBar>,
    last_hotbar_mode: Option<HudHotbarMode>,
    pub(super) contextual: waypoint_locator::HudContextualRuntime,
    pub(super) vignette: vignette::VignetteRuntime,
    scope_scale: f32,
    pub(super) player_health: HudPlayerHealthPresentation,
    selected_item_name: HudSelectedItemNamePresentation,
    last_bubble_pop_sound_played: i32,
    pub(super) smithing_screen: smithing_screen::SmithingScreenRuntime,
    pub(super) chat_scroll_page: Option<bbb_client_render::HudChatScrollPage>,
}

#[derive(Debug, Default)]
struct HudSelectedItemNamePresentation {
    highlight_ticks: i32,
    last_item_id: Option<i32>,
    last_hover_name: Option<String>,
    last_runs: Vec<bbb_client_render::HudComponentTextRun>,
    runs_revision: u64,
    last_published: Option<(bool, i32, bool, u64)>,
}

#[derive(Debug, Default)]
pub(super) struct HudPlayerHealthPresentation {
    last_health: i32,
    display_health: i32,
    last_health_at: Option<Instant>,
    pub(super) blink_until_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudPlayerHealthFrame {
    pub(super) display_health: i32,
    pub(super) blinking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudAirBubblePopSound {
    pub(super) volume: f32,
    pub(super) pitch: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct HudFrameDeltaTicks {
    pub(super) game_time: f32,
    pub(super) realtime: f32,
}

#[derive(Debug)]
struct BossBarProgressPresentation {
    instance_revision: u64,
    progress_revision: u64,
    start_progress: f32,
    target_progress: f32,
    started_at: Instant,
    published_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudStaticUiCacheKey {
    pub(super) step_generation: u64,
    pub(super) world_revision: u64,
    pub(super) client_tick_index: u64,
    pub(super) level_generation: u64,
    pub(super) chat_delivery_revision: u64,
    pub(super) chat_display_refresh_revision: u64,
    pub(super) resource_generation: u64,
    pub(super) resources_loaded: bool,
    pub(super) gui_surface_size: PhysicalSize<u32>,
    pub(super) hud_cursor_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudScreensCacheKey {
    pub(super) step_generation: u64,
    pub(super) world_revision: u64,
    pub(super) client_tick_index: u64,
    pub(super) level_generation: u64,
    pub(super) resource_generation: u64,
    pub(super) resources_loaded: bool,
    pub(super) terrain_textures_epoch: u64,
    pub(super) gui_surface_size: PhysicalSize<u32>,
    pub(super) gui_cursor_bits: Option<(u64, u64)>,
    pub(super) hud_cursor_position: Option<(i32, i32)>,
    pub(super) partial_tick_bits: Option<u32>,
    pub(super) panorama_rotation_bits: Option<u32>,
    pub(super) win_screen_resources_ptr: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudHotbarItemsCacheState {
    key: HudHotbarBlockItemModelsCacheKey,
    has_frame_varying_icon: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudOverlayTextCacheKey {
    step_generation: u64,
    world_revision: u64,
    level_generation: u64,
    resource_generation: u64,
    resources_loaded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudChatCacheKey {
    level_generation: u64,
    delivery_revision: u64,
    display_refresh_revision: u64,
    resource_generation: u64,
    resources_loaded: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct HudHotbarBlockItemModelsCacheKey {
    pub(super) ordinary_hotbar: bool,
    pub(super) resource_generation: u64,
    pub(super) resources_loaded: bool,
    pub(super) shift_down: bool,
    pub(super) keybind_context: ItemModelKeybindContext,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct HudScoreboardSidebarCacheKey {
    pub(super) level_generation: u64,
    pub(super) display_revision: u64,
    pub(super) resource_generation: u64,
    pub(super) resources_loaded: bool,
    pub(super) hidden: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct HudPlayerListCacheKey {
    pub(super) level_generation: u64,
    pub(super) display_revision: u64,
    pub(super) profile_presentation_revision: u64,
    pub(super) resource_generation: u64,
    pub(super) resources_loaded: bool,
    pub(super) visible: bool,
    pub(super) show_heads: bool,
}

impl HudRenderRuntime {
    pub(crate) fn prepare_periodic_notifications(
        catalog: &bbb_client_assets::PeriodicNotificationCatalog,
    ) -> anyhow::Result<Option<PeriodicNotificationSchedule>> {
        PeriodicNotificationSchedule::prepare(
            catalog,
            periodic_notifications::host_country_iso3().as_deref(),
        )
    }

    pub(crate) fn install_periodic_notifications(
        &mut self,
        generation: u64,
        schedule: Option<PeriodicNotificationSchedule>,
        now: Instant,
    ) {
        self.periodic_notifications
            .install(generation, schedule, now);
    }

    pub(crate) fn screen_narrations(&self) -> &ScreenNarrationFrames {
        &self.screen_narrations
    }

    pub(super) fn win_screen_lines(
        &mut self,
        resources: &Arc<crate::win_screen_resources::WinScreenResources>,
    ) -> Arc<[HudWinScreenLine]> {
        let unchanged = self
            .win_screen_resource_owner
            .as_ref()
            .is_some_and(|previous| Arc::ptr_eq(previous, resources));
        if !unchanged {
            self.win_screen_resource_owner = Some(Arc::clone(resources));
            self.win_screen_lines = resources
                .lines
                .iter()
                .map(|line| HudWinScreenLine {
                    runs: line.runs.clone(),
                    centered: line.centered,
                })
                .collect::<Vec<_>>()
                .into();
        }
        Arc::clone(&self.win_screen_lines)
    }

    pub(super) fn frame_delta_ticks(
        &mut self,
        now: Instant,
        tick_interval: Duration,
    ) -> HudFrameDeltaTicks {
        let Some(previous) = self.last_ui_frame_at.replace(now) else {
            return HudFrameDeltaTicks::default();
        };
        let elapsed_seconds = now.saturating_duration_since(previous).as_secs_f32();
        if !elapsed_seconds.is_finite() || elapsed_seconds <= 0.0 {
            return HudFrameDeltaTicks::default();
        }
        let tick_interval_seconds = tick_interval.as_secs_f32();
        let game_time = if tick_interval_seconds.is_finite() && tick_interval_seconds > 0.0 {
            let delta = elapsed_seconds / tick_interval_seconds;
            if delta.is_finite() && delta > 0.0 {
                delta
            } else {
                0.0
            }
        } else {
            0.0
        };
        let realtime_ticks = elapsed_seconds * 20.0;
        let realtime = if !realtime_ticks.is_finite() || realtime_ticks <= 0.0 {
            0.0
        } else if realtime_ticks > 7.0 {
            0.5
        } else {
            realtime_ticks
        };
        HudFrameDeltaTicks {
            game_time,
            realtime,
        }
    }

    /// Vanilla `Options.panoramaSpeed` (`panoramaScrollSpeed`); non-finite or
    /// negative startup values keep the vanilla default.
    pub(crate) fn set_panorama_scroll_speed(&mut self, speed: f64) {
        if speed.is_finite() && speed >= 0.0 {
            self.panorama_scroll_speed = PanoramaScrollSpeed(speed);
        }
    }

    pub(super) fn panorama_rotation_degrees(
        &mut self,
        visible: bool,
        frame_delta: HudFrameDeltaTicks,
    ) -> f32 {
        if visible {
            // Vanilla `Panorama.extractRenderState`: `spin = wrapDegrees(spin
            // + (float)(realtimeDelta * panoramaSpeed) * 0.1F)`.
            let delta =
                (f64::from(frame_delta.realtime.max(0.0)) * self.panorama_scroll_speed.0) as f32;
            self.panorama_spin_degrees =
                panorama_wrap_degrees(self.panorama_spin_degrees + delta * 0.1);
        }
        -self.panorama_spin_degrees
    }

    pub(super) fn selected_item_name_frame(
        &mut self,
        ticks: u32,
        raised: bool,
        selected: Option<NativeItemHoverName>,
        selected_item_id: Option<i32>,
        notification_display_time: f32,
    ) -> Option<bbb_client_render::HudSelectedItemName> {
        let state = &mut self.selected_item_name;
        for _ in 0..ticks {
            match &selected {
                None => state.highlight_ticks = 0,
                Some(hover) => {
                    if state.last_item_id != selected_item_id
                        || state.last_hover_name.as_deref() != Some(hover.text.as_str())
                    {
                        state.highlight_ticks =
                            (40.0 * f64::from(notification_display_time)) as i32;
                    } else if state.highlight_ticks > 0 {
                        state.highlight_ticks -= 1;
                    }
                }
            }
            let next_runs = selected
                .as_ref()
                .map(|hover| hover.runs.clone())
                .unwrap_or_default();
            if state.last_runs != next_runs {
                state.runs_revision = state.runs_revision.wrapping_add(1);
                state.last_runs = next_runs;
            }
            state.last_item_id = selected_item_id;
            state.last_hover_name = selected.as_ref().map(|hover| hover.text.clone());
        }
        (state.highlight_ticks > 0 && !state.last_runs.is_empty()).then(|| {
            bbb_client_render::HudSelectedItemName {
                runs: state.last_runs.clone(),
                highlight_ticks: state.highlight_ticks,
                raised,
            }
        })
    }

    pub(super) fn selected_item_name_update(
        &mut self,
        ticks: u32,
        local_player_present: bool,
        raised: bool,
        notification_display_time: f32,
        resolve_hover: impl FnOnce() -> (Option<NativeItemHoverName>, Option<i32>),
    ) -> RendererFrameUpdate<Option<bbb_client_render::HudSelectedItemName>> {
        if !local_player_present {
            let key = (false, 0, false, self.selected_item_name.runs_revision);
            if self.selected_item_name.last_published == Some(key) {
                return RendererFrameUpdate::Unchanged;
            }
            self.selected_item_name.last_published = Some(key);
            return RendererFrameUpdate::Replace(None);
        }
        if ticks > 0 {
            let (selected, selected_item_id) = resolve_hover();
            let _ = self.selected_item_name_frame(
                ticks,
                raised,
                selected,
                selected_item_id,
                notification_display_time,
            );
        }
        let state = &mut self.selected_item_name;
        let visible = state.highlight_ticks > 0 && !state.last_runs.is_empty();
        let key = (
            visible,
            if visible { state.highlight_ticks } else { 0 },
            raised,
            state.runs_revision,
        );
        if state.last_published == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        state.last_published = Some(key);
        RendererFrameUpdate::Replace(visible.then(|| bbb_client_render::HudSelectedItemName {
            runs: state.last_runs.clone(),
            highlight_ticks: state.highlight_ticks,
            raised,
        }))
    }

    pub(super) fn player_health_frame(
        &mut self,
        now: Instant,
        tick_count: u64,
        current_health: i32,
        invulnerable_time: i32,
    ) -> HudPlayerHealthFrame {
        // Vanilla computes this frame's phase before observing a health edge,
        // so a newly armed deadline cannot blink until a later extraction.
        let remaining_blink_ticks = self
            .player_health
            .blink_until_tick
            .saturating_sub(tick_count);
        let blinking = self.player_health.blink_until_tick > tick_count
            && (remaining_blink_ticks / 3) % 2 == 1;

        if invulnerable_time > 0 {
            let blink_duration = if current_health < self.player_health.last_health {
                Some(20)
            } else if current_health > self.player_health.last_health {
                Some(10)
            } else {
                None
            };
            if let Some(blink_duration) = blink_duration {
                self.player_health.last_health_at = Some(now);
                self.player_health.blink_until_tick = tick_count.saturating_add(blink_duration);
            }
        }

        let display_hold_expired = self.player_health.last_health_at.is_none_or(|last| {
            now.saturating_duration_since(last) > HUD_HEALTH_DISPLAY_HOLD_DURATION
        });
        if display_hold_expired {
            self.player_health.display_health = current_health;
            self.player_health.last_health_at = Some(now);
        }
        self.player_health.last_health = current_health;

        HudPlayerHealthFrame {
            display_health: self.player_health.display_health,
            blinking,
        }
    }

    pub(super) fn air_bubble_pop_sound(
        &mut self,
        state: HudAirSupply,
        extracted: bool,
    ) -> Option<HudAirBubblePopSound> {
        if !extracted || !hud_air_supply_visible(state) {
            return None;
        }
        if !state.eye_in_water {
            self.last_bubble_pop_sound_played = 0;
            return None;
        }
        let pop = hud_air_supply_pop(state)?;
        if self.last_bubble_pop_sound_played == pop.bubble {
            return None;
        }
        self.last_bubble_pop_sound_played = pop.bubble;
        Some(HudAirBubblePopSound {
            volume: 0.5 + 0.1 * (pop.empty_bubbles - 2).max(0) as f32,
            pitch: 1.0 + 0.1 * (pop.empty_bubbles - 4).max(0) as f32,
        })
    }

    pub(super) fn win_screen_realtime_delta_ticks(
        generation: Option<u64>,
        frame_delta: HudFrameDeltaTicks,
    ) -> Option<(u64, f32)> {
        let generation = generation?;
        (frame_delta.realtime > 0.0).then_some((generation, frame_delta.realtime))
    }

    pub(super) fn spyglass_scope(
        &mut self,
        game_time_delta_ticks: f32,
        hide_gui: bool,
        first_person: bool,
        scoping: bool,
    ) -> Option<HudSpyglassScope> {
        if hide_gui {
            return None;
        }
        let delta_ticks = if game_time_delta_ticks.is_finite() && game_time_delta_ticks > 0.0 {
            game_time_delta_ticks
        } else {
            0.0
        };
        let step = (0.5 * delta_ticks).min(1.0);
        self.scope_scale += step * (VANILLA_SPYGLASS_SCOPE_TARGET_SCALE - self.scope_scale);
        if !first_person {
            return None;
        }
        if scoping {
            Some(HudSpyglassScope {
                scale: self.scope_scale,
            })
        } else {
            self.scope_scale = VANILLA_SPYGLASS_SCOPE_RESET_SCALE;
            None
        }
    }

    pub(super) fn bump_terrain_textures_epoch(&mut self, changed: bool) -> u64 {
        if changed {
            self.terrain_textures_epoch = self.terrain_textures_epoch.wrapping_add(1);
        }
        self.terrain_textures_epoch
    }

    pub(super) fn static_ui_update(
        &mut self,
        key: HudStaticUiCacheKey,
        project: impl FnOnce(&mut Self) -> super::render_extract::HudStaticUiFrame,
    ) -> RendererFrameUpdate<super::render_extract::HudStaticUiFrame> {
        if self.static_ui_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        let built = project(self);
        self.static_ui_cache_key = Some(key);
        RendererFrameUpdate::Replace(built)
    }

    pub(super) fn screens_update(
        &mut self,
        key: HudScreensCacheKey,
        project: impl FnOnce(&mut Self) -> Box<super::render_extract::HudScreensFrame>,
    ) -> (
        RendererFrameUpdate<Box<super::render_extract::HudScreensFrame>>,
        Option<bool>,
    ) {
        if self.screens_cache_key == Some(key) {
            return (RendererFrameUpdate::Unchanged, self.last_actual_pause_menu);
        }
        let screens = project(self);
        let pause_menu = screens
            .hud_pause_screen
            .as_ref()
            .filter(|screen| !screen.feedback_subscreen)
            .map(|screen| screen.show_pause_menu);
        self.screens_cache_key = Some(key);
        self.last_actual_pause_menu = pause_menu;
        self.last_inventory_screen = screens.hud_inventory_screen.clone();
        (RendererFrameUpdate::Replace(screens), pause_menu)
    }

    pub(super) fn cached_inventory_screen(&self) -> Option<&HudInventoryScreen> {
        self.last_inventory_screen.as_ref()
    }

    pub(super) fn chat_messages_update(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> RendererFrameUpdate<HudChatMessagesFrame> {
        let key = HudChatCacheKey {
            level_generation: world.client_level_generation(),
            delivery_revision: world.client_chat().delivery_revision,
            display_refresh_revision: world.client_chat().display_refresh_revision,
            resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
            resources_loaded: item_runtime.is_some(),
        };
        let previous = self.chat_cache_key.replace(key);
        if previous == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        RendererFrameUpdate::Replace(HudChatMessagesFrame {
            messages: super::render_extract::hud_chat_messages_from_world_cached(
                world,
                item_runtime,
                &mut self.chat_entry_cache,
            ),
            refreshed: previous.is_some_and(|previous| {
                previous.display_refresh_revision != key.display_refresh_revision
            }),
        })
    }

    pub(super) fn hotbar_items_and_models_update<C>(
        &mut self,
        world: &WorldStore,
        stable_world_unchanged: bool,
        key: HudHotbarBlockItemModelsCacheKey,
        project_items: impl FnOnce() -> (super::render_extract::HudHotbarItemsFrame, C),
        project_models: impl FnOnce(
            C,
            bool,
        )
            -> (Option<HudBlockItemModel>, Vec<Option<HudBlockItemModel>>),
    ) -> HotbarItemsModelsUpdate {
        let models_cached =
            stable_world_unchanged && self.hotbar_block_item_models_cache_key == Some(key);
        let items_cached = stable_world_unchanged
            && self
                .hotbar_items_cache
                .is_some_and(|cached| cached.key == key && !cached.has_frame_varying_icon);
        if items_cached && models_cached {
            return (
                RendererFrameUpdate::Unchanged,
                RendererFrameUpdate::Unchanged,
                RendererFrameUpdate::Unchanged,
            );
        }
        let (items, ctx) = project_items();
        let has_pop = world.hotbar_pop_times().iter().any(|&ticks| ticks > 0);
        let has_cooldown = items
            .hud_hotbar_item_icons
            .iter()
            .flatten()
            .any(|icon| icon.cooldown_progress.is_some())
            || items
                .hud_offhand_slot
                .as_ref()
                .is_some_and(|slot| slot.icon.cooldown_progress.is_some());
        self.hotbar_items_cache = Some(HudHotbarItemsCacheState {
            key,
            has_frame_varying_icon: has_pop || has_cooldown,
        });
        let has_offhand = items.hud_offhand_slot.is_some();
        let items_update = RendererFrameUpdate::Replace(Box::new(items));
        if models_cached {
            return (
                items_update,
                RendererFrameUpdate::Unchanged,
                RendererFrameUpdate::Unchanged,
            );
        }
        self.hotbar_block_item_models_cache_key = Some(key);
        let (offhand_model, hotbar_models) = project_models(ctx, has_offhand);
        (
            items_update,
            RendererFrameUpdate::Replace(offhand_model),
            RendererFrameUpdate::Replace(hotbar_models),
        )
    }

    pub(super) fn action_bar_update(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        step_generation: u64,
        world_revision: u64,
        partial_tick: f32,
    ) -> RendererFrameUpdate<Option<bbb_client_render::HudActionBarText>> {
        let Some(action_bar) = world.action_bar().filter(|bar| bar.display_ticks > 0) else {
            self.action_bar_cache = None;
            if self.action_bar_published_none {
                return RendererFrameUpdate::Unchanged;
            }
            self.action_bar_published_none = true;
            return RendererFrameUpdate::Replace(None);
        };
        let key = HudOverlayTextCacheKey {
            step_generation,
            world_revision,
            level_generation: world.client_level_generation(),
            resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
            resources_loaded: item_runtime.is_some(),
        };
        let runs = match &self.action_bar_cache {
            Some((cached_key, cached_runs)) if *cached_key == key => Arc::clone(cached_runs),
            _ => {
                let Some(runs) =
                    super::render_extract::hud_action_bar_runs_from_world(world, item_runtime)
                else {
                    self.action_bar_cache = None;
                    if self.action_bar_published_none {
                        return RendererFrameUpdate::Unchanged;
                    }
                    self.action_bar_published_none = true;
                    return RendererFrameUpdate::Replace(None);
                };
                self.action_bar_cache = Some((key, Arc::clone(&runs)));
                runs
            }
        };
        self.action_bar_published_none = false;
        RendererFrameUpdate::Replace(Some(bbb_client_render::HudActionBarText {
            runs: runs.to_vec(),
            remaining_ticks: action_bar.display_ticks,
            partial_tick,
            animate_color: action_bar.animate_color,
        }))
    }

    pub(super) fn title_text_update(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        step_generation: u64,
        world_revision: u64,
        partial_tick: f32,
    ) -> RendererFrameUpdate<Option<bbb_client_render::HudTitleText>> {
        let title = world.title();
        if title.title.is_none() || title.title_time <= 0 {
            self.title_cache = None;
            if self.title_published_none {
                return RendererFrameUpdate::Unchanged;
            }
            self.title_published_none = true;
            return RendererFrameUpdate::Replace(None);
        }
        let key = HudOverlayTextCacheKey {
            step_generation,
            world_revision,
            level_generation: world.client_level_generation(),
            resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
            resources_loaded: item_runtime.is_some(),
        };
        let (title_runs, subtitle_runs) = match &self.title_cache {
            Some((cached_key, cached_title, cached_subtitle)) if *cached_key == key => {
                (Arc::clone(cached_title), Arc::clone(cached_subtitle))
            }
            _ => {
                let Some((title_runs, subtitle_runs)) =
                    super::render_extract::hud_title_runs_from_world(world, item_runtime)
                else {
                    self.title_cache = None;
                    if self.title_published_none {
                        return RendererFrameUpdate::Unchanged;
                    }
                    self.title_published_none = true;
                    return RendererFrameUpdate::Replace(None);
                };
                self.title_cache = Some((key, Arc::clone(&title_runs), Arc::clone(&subtitle_runs)));
                (title_runs, subtitle_runs)
            }
        };
        self.title_published_none = false;
        RendererFrameUpdate::Replace(Some(bbb_client_render::HudTitleText {
            title_runs: title_runs.to_vec(),
            subtitle_runs: subtitle_runs.to_vec(),
            remaining_ticks: title.title_time,
            fade_in: title.fade_in,
            stay: title.stay,
            fade_out: title.fade_out,
            partial_tick,
        }))
    }

    pub(super) fn toasts_update(
        &mut self,
        project: impl FnOnce(&ClientToastRuntime) -> Vec<bbb_client_render::HudToast>,
    ) -> RendererFrameUpdate<Vec<bbb_client_render::HudToast>> {
        if !self.client_toasts.has_visible_toasts() {
            if self.toasts_published_empty {
                return RendererFrameUpdate::Unchanged;
            }
            self.toasts_published_empty = true;
            return RendererFrameUpdate::Replace(Vec::new());
        }
        let toasts = project(&self.client_toasts);
        if toasts.is_empty() {
            if self.toasts_published_empty {
                return RendererFrameUpdate::Unchanged;
            }
            self.toasts_published_empty = true;
            return RendererFrameUpdate::Replace(Vec::new());
        }
        self.toasts_published_empty = false;
        RendererFrameUpdate::Replace(toasts)
    }

    pub(super) fn now_playing_toast_update(
        &mut self,
        projected: Option<bbb_client_render::HudNowPlayingToast>,
    ) -> RendererFrameUpdate<Option<bbb_client_render::HudNowPlayingToast>> {
        if projected.is_none() {
            if self.now_playing_published_none {
                return RendererFrameUpdate::Unchanged;
            }
            self.now_playing_published_none = true;
            return RendererFrameUpdate::Replace(None);
        }
        self.now_playing_published_none = false;
        RendererFrameUpdate::Replace(projected)
    }

    pub(super) fn debug_overlay_update(
        &mut self,
        can_be_cached_empty: bool,
        empty_key: (u64, u64),
        project: impl FnOnce() -> Option<HudDebugOverlay>,
    ) -> RendererFrameUpdate<Option<HudDebugOverlay>> {
        if can_be_cached_empty
            && self.debug_overlay_empty_key == Some(empty_key)
            && self.debug_overlay_published_none
        {
            return RendererFrameUpdate::Unchanged;
        }
        match project() {
            None => {
                if can_be_cached_empty {
                    self.debug_overlay_empty_key = Some(empty_key);
                } else {
                    self.debug_overlay_empty_key = None;
                }
                if self.debug_overlay_published_none {
                    RendererFrameUpdate::Unchanged
                } else {
                    self.debug_overlay_published_none = true;
                    RendererFrameUpdate::Replace(None)
                }
            }
            Some(overlay) => {
                self.debug_overlay_empty_key = None;
                self.debug_overlay_published_none = false;
                RendererFrameUpdate::Replace(Some(overlay))
            }
        }
    }

    pub(super) fn player_list_tick_count_update(&mut self, tick_count: u64) -> Option<u64> {
        (self.last_player_list_tick_count.replace(tick_count) != Some(tick_count))
            .then_some(tick_count)
    }

    pub(super) fn crosshair_visible_update(&mut self, visible: bool) -> RendererFrameUpdate<bool> {
        publish_if_changed(&mut self.last_crosshair_visible, visible)
    }

    pub(super) fn crosshair_attack_indicator_update(
        &mut self,
        indicator: HudCrosshairAttackIndicator,
    ) -> RendererFrameUpdate<HudCrosshairAttackIndicator> {
        publish_if_changed(&mut self.last_crosshair_attack_indicator, indicator)
    }

    pub(super) fn hotbar_attack_indicator_update(
        &mut self,
        indicator: HudHotbarAttackIndicator,
    ) -> RendererFrameUpdate<HudHotbarAttackIndicator> {
        publish_if_changed(&mut self.last_hotbar_attack_indicator, indicator)
    }

    pub(super) fn player_health_update(
        &mut self,
        health: Option<HudPlayerHealth>,
    ) -> RendererFrameUpdate<Option<HudPlayerHealth>> {
        publish_if_changed(&mut self.last_published_player_health, health)
    }

    pub(super) fn vignette_update(
        &mut self,
        vignette: Option<HudVignette>,
    ) -> RendererFrameUpdate<Option<HudVignette>> {
        publish_if_changed(&mut self.last_vignette, vignette)
    }

    pub(super) fn spyglass_scope_update(
        &mut self,
        scope: Option<HudSpyglassScope>,
    ) -> RendererFrameUpdate<Option<HudSpyglassScope>> {
        publish_if_changed(&mut self.last_spyglass_scope, scope)
    }

    pub(super) fn portal_overlay_update(
        &mut self,
        overlay: Option<HudPortalOverlay>,
    ) -> RendererFrameUpdate<Option<HudPortalOverlay>> {
        publish_if_changed(&mut self.last_portal_overlay, overlay)
    }

    pub(super) fn confusion_overlay_update(
        &mut self,
        overlay: Option<HudConfusionOverlay>,
    ) -> RendererFrameUpdate<Option<HudConfusionOverlay>> {
        publish_if_changed(&mut self.last_confusion_overlay, overlay)
    }

    pub(super) fn contextual_bar_update(
        &mut self,
        bar: HudContextualBar,
    ) -> RendererFrameUpdate<HudContextualBar> {
        publish_if_changed(&mut self.last_contextual_bar, bar)
    }

    pub(super) fn hotbar_mode_update(
        &mut self,
        mode: HudHotbarMode,
    ) -> RendererFrameUpdate<HudHotbarMode> {
        publish_if_changed(&mut self.last_hotbar_mode, mode)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(super) fn hotbar_block_item_models_update(
        &mut self,
        stable_world_unchanged: bool,
        key: HudHotbarBlockItemModelsCacheKey,
        project: impl FnOnce() -> Vec<Option<HudBlockItemModel>>,
    ) -> RendererFrameUpdate<Vec<Option<HudBlockItemModel>>> {
        if stable_world_unchanged && self.hotbar_block_item_models_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        self.hotbar_block_item_models_cache_key = Some(key);
        RendererFrameUpdate::Replace(project())
    }

    pub(super) fn scoreboard_sidebar_update(
        &mut self,
        key: HudScoreboardSidebarCacheKey,
        project: impl FnOnce() -> Option<HudScoreboardSidebar>,
    ) -> RendererFrameUpdate<Option<HudScoreboardSidebar>> {
        if self.scoreboard_sidebar_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        self.scoreboard_sidebar_cache_key = Some(key);
        RendererFrameUpdate::Replace(project())
    }

    pub(super) fn player_list_update(
        &mut self,
        key: HudPlayerListCacheKey,
        project: impl FnOnce() -> Option<HudPlayerList>,
    ) -> RendererFrameUpdate<Option<HudPlayerList>> {
        if self.player_list_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        self.player_list_cache_key = Some(key);
        RendererFrameUpdate::Replace(project())
    }

    #[cfg(test)]
    pub(super) fn boss_bars_update(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        connection_generation: u64,
        change_anchor: Instant,
        render_now: Instant,
        stable_world_unchanged: bool,
    ) -> RendererFrameUpdate<Vec<bbb_client_render::HudBossBar>> {
        self.boss_bars_update_for_height(
            world,
            item_runtime,
            connection_generation,
            change_anchor,
            render_now,
            stable_world_unchanged,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn boss_bars_update_for_height(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        connection_generation: u64,
        change_anchor: Instant,
        render_now: Instant,
        stable_world_unchanged: bool,
        gui_height: Option<u32>,
    ) -> RendererFrameUpdate<Vec<bbb_client_render::HudBossBar>> {
        let generations = (
            connection_generation,
            item_runtime.map(NativeItemRuntime::resource_generation),
        );
        let presentation_changed = self.boss_generations != Some(generations);
        let connection_changed =
            self.boss_generations.map(|key| key.0) != Some(connection_generation);
        let revision = world.boss_bar_revision();
        let revision_changed = self.boss_bar_revision != Some(revision);
        let visible_cap = gui_height.map(hud_boss_bar_max_visible);
        let visible_cap_changed = self.boss_visible_cap != Some(visible_cap);

        if !presentation_changed
            && !revision_changed
            && !visible_cap_changed
            && !self.boss_bars_has_pending_lerp
            && stable_world_unchanged
        {
            return RendererFrameUpdate::Unchanged;
        }

        self.boss_generations = Some(generations);
        self.boss_bar_revision = Some(revision);
        self.boss_visible_cap = Some(visible_cap);
        if connection_changed {
            self.boss_progress.clear();
        }
        if revision_changed || connection_changed {
            self.boss_progress
                .retain(|id, _| world.boss_bar(*id).is_some());
        }

        let mut must_replace = presentation_changed
            || revision_changed
            || visible_cap_changed
            || !stable_world_unchanged;
        let all_bars = world.boss_bars();
        let visible_count = visible_cap.map_or(all_bars.len(), |cap| all_bars.len().min(cap));
        let mut bars = Vec::with_capacity(visible_count);
        let mut any_pending_lerp = false;
        for (index, bar) in all_bars.iter().enumerate() {
            let presentation =
                self.boss_progress
                    .entry(bar.id)
                    .or_insert_with(|| BossBarProgressPresentation {
                        instance_revision: bar.instance_revision,
                        progress_revision: bar.progress_revision,
                        start_progress: bar.progress,
                        target_progress: bar.progress,
                        started_at: change_anchor,
                        published_complete: true,
                    });
            if presentation.instance_revision != bar.instance_revision {
                *presentation = BossBarProgressPresentation {
                    instance_revision: bar.instance_revision,
                    progress_revision: bar.progress_revision,
                    start_progress: bar.progress,
                    target_progress: bar.progress,
                    started_at: change_anchor,
                    published_complete: true,
                };
            } else if presentation.progress_revision != bar.progress_revision {
                let visible_progress = presentation.progress_at(change_anchor);
                presentation.progress_revision = bar.progress_revision;
                presentation.start_progress = visible_progress;
                presentation.target_progress = bar.progress;
                presentation.started_at = change_anchor;
                presentation.published_complete = false;
            }

            if index >= visible_count {
                continue;
            }
            let progress = presentation.progress_at(render_now);
            let active = presentation.is_active_at(render_now);
            if active || !presentation.published_complete {
                must_replace = true;
            }
            if active {
                any_pending_lerp = true;
            } else {
                presentation.published_complete = true;
            }
            if let Some(projected) = hud_boss_bar_from_world_state(bar, item_runtime, progress) {
                bars.push(projected);
            }
        }
        self.boss_bars_has_pending_lerp = any_pending_lerp;

        if must_replace {
            RendererFrameUpdate::Replace(bars)
        } else {
            RendererFrameUpdate::Unchanged
        }
    }
}

/// Vanilla `BossHealthOverlay.extractRenderState` (`BossHealthOverlay.java:65-78`):
/// draws from `y = 12` stepping `19` px per bar and stops once `y >= guiHeight / 3`.
fn hud_boss_bar_max_visible(gui_height: u32) -> usize {
    let cutoff = (gui_height.max(1) / 3) as i32;
    if cutoff <= 12 {
        1
    } else {
        ((cutoff - 13) / 19 + 1) as usize
    }
}

impl BossBarProgressPresentation {
    fn progress_at(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started_at);
        let fraction =
            (elapsed.as_secs_f32() / BOSS_BAR_PROGRESS_LERP_DURATION.as_secs_f32()).clamp(0.0, 1.0);
        self.start_progress + fraction * (self.target_progress - self.start_progress)
    }

    fn is_active_at(&self, now: Instant) -> bool {
        self.start_progress != self.target_progress
            && now.saturating_duration_since(self.started_at) < BOSS_BAR_PROGRESS_LERP_DURATION
    }
}

/// Vanilla `Options.panoramaSpeed` default (`OptionInstance.UnitDouble`, 1.0).
#[derive(Debug)]
struct PanoramaScrollSpeed(f64);

impl Default for PanoramaScrollSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

fn publish_if_changed<T: PartialEq + Clone>(
    slot: &mut Option<T>,
    next: T,
) -> RendererFrameUpdate<T> {
    if slot.as_ref() == Some(&next) {
        return RendererFrameUpdate::Unchanged;
    }
    *slot = Some(next.clone());
    RendererFrameUpdate::Replace(next)
}

fn panorama_wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}
