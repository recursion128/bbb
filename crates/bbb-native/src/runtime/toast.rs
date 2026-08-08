//! Native ownership for Vanilla's shared bounded toast queue and wall-clock animation.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bbb_item_model::{ItemModelKeybindContext, NativeItemRuntime};
use bbb_protocol::packets::{AdvancementFrameType, ItemStackSummary};
use bbb_renderer::{
    hud_wrap_styled_text, HudAdvancementToast, HudFontGlyphMap, HudNowPlayingToast,
    HudNowPlayingToastPlacement, HudPauseScreen, HudRecipeToast, HudStyledTextRun, HudSystemToast,
    HudSystemToastInstance, HudTextStyle, HudToast, HudTutorialToast, HudTutorialToastIcon,
    HUD_TOAST_SLOTS,
};
use bbb_world::{
    ClientSystemToastEvent, ClientToastEvent, ClientTutorialToastEvent, ClientTutorialToastState,
    ClientTutorialToastToken, LocalSoundEventState, ResourcePackDownloadPhase,
    ResourcePackDownloadState, ResourcePackDownloadToastAction, SoundHolderState,
    TutorialToastKind, WorldStore,
};

use crate::terrain_runtime::TerrainTextureState;

use super::periodic_notifications::PeriodicNotificationToast;
use super::{
    fake_block_item_3d_model, hud_fake_item_icon_for_stack, hud_styled_runs_from_rich_text,
    localized_gui_text, localized_gui_text_with_args, server_pack_bytes_human_readable,
};

const MAX_NATIVE_TOASTS: usize = bbb_world::MAX_CLIENT_TOAST_EVENTS;
const TOAST_SLIDE_DURATION: Duration = Duration::from_millis(600);
const TOAST_VISIBLE_DURATION: Duration = Duration::from_millis(5_000);
const MUSIC_NOTE_COLOR_CHANGE_FREQUENCY: Duration = Duration::from_millis(25);
const MUSIC_NOTE_COLOR_DURATION_TICKS: i32 = 30;

#[derive(Debug)]
pub(super) struct NowPlayingToastRuntime {
    last_mutation_sequence: u64,
    animation_started_at: Option<Instant>,
    became_fully_visible_at: Option<Instant>,
    showing: bool,
    wanted_showing: bool,
    visible_portion: f32,
    fully_visible_ms: u64,
    finished: bool,
    update_toast: bool,
    notification_display_time: f32,
    last_note_color_change: Option<Instant>,
    note_color_tick: i32,
    note_color_argb: u32,
}

impl Default for NowPlayingToastRuntime {
    fn default() -> Self {
        Self {
            last_mutation_sequence: 0,
            animation_started_at: None,
            became_fully_visible_at: None,
            showing: false,
            wanted_showing: false,
            visible_portion: 0.0,
            fully_visible_ms: 0,
            finished: false,
            update_toast: false,
            notification_display_time: 1.0,
            last_note_color_change: None,
            note_color_tick: 0,
            note_color_argb: u32::MAX,
        }
    }
}

impl NowPlayingToastRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_and_project(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        display_state: crate::startup::MusicToastDisplayStateArg,
        final_music_volume: f32,
        notification_display_time: f32,
        pause_screen: Option<&HudPauseScreen>,
        client_tick: u64,
        now: Instant,
    ) -> Option<HudNowPlayingToast> {
        if !display_state.renders_pause() {
            return None;
        }

        let canonical = world.now_playing_toast();
        if canonical.mutation_sequence != self.last_mutation_sequence {
            self.last_mutation_sequence = canonical.mutation_sequence;
            if canonical.wanted_visible {
                self.reset_for_show(notification_display_time);
            } else {
                self.wanted_showing = false;
            }
        }
        let renders_in_pause =
            pause_screen.is_some_and(|screen| screen.show_pause_menu) && final_music_volume > 0.0;
        if canonical.sound_name.is_some() && (self.update_toast || renders_in_pause) {
            self.tick_music_note_color(now);
        }
        if self.animation_started_at.is_some() || canonical.mutation_sequence > 0 {
            self.update_animation(now);
        }

        let sound_name = canonical.sound_name.as_deref()?;
        let text = localized_gui_text(
            item_runtime,
            &now_playing_translation_key(sound_name),
            &now_playing_translation_key(sound_name),
        );
        let placement = if renders_in_pause {
            HudNowPlayingToastPlacement::PauseScreen
        } else if display_state.renders_toast() && pause_screen.is_none() && !self.finished {
            HudNowPlayingToastPlacement::SlidingToast
        } else {
            return None;
        };
        Some(HudNowPlayingToast {
            text,
            visible_portion: if placement == HudNowPlayingToastPlacement::PauseScreen {
                1.0
            } else {
                self.visible_portion
            },
            note_color_argb: self.note_color_argb,
            client_tick,
            placement,
        })
    }

    fn reset_for_show(&mut self, notification_display_time: f32) {
        self.animation_started_at = None;
        self.became_fully_visible_at = None;
        self.showing = false;
        self.wanted_showing = true;
        self.visible_portion = 0.0;
        self.fully_visible_ms = 0;
        self.finished = false;
        self.update_toast = true;
        self.notification_display_time = notification_display_time;
    }

    fn update_animation(&mut self, now: Instant) {
        let animation_started_at = *self.animation_started_at.get_or_insert_with(|| {
            self.showing = true;
            now
        });
        if self.showing
            && now.saturating_duration_since(animation_started_at) <= TOAST_SLIDE_DURATION
        {
            self.became_fully_visible_at = Some(now);
        }
        let became_fully_visible_at = self.became_fully_visible_at.unwrap_or(now);
        self.fully_visible_ms = now
            .saturating_duration_since(became_fully_visible_at)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let progress = (now
            .saturating_duration_since(animation_started_at)
            .as_secs_f32()
            / TOAST_SLIDE_DURATION.as_secs_f32())
        .clamp(0.0, 1.0)
        .powi(2);
        self.visible_portion = if self.showing {
            progress
        } else {
            1.0 - progress
        };
        if self.update_toast {
            self.wanted_showing = display_time_remaining(
                self.fully_visible_ms,
                TOAST_VISIBLE_DURATION.as_millis() as u64,
                self.notification_display_time,
            );
        }
        if self.wanted_showing != self.showing {
            let reverse_ms =
                ((1.0 - self.visible_portion) * TOAST_SLIDE_DURATION.as_millis() as f32) as u64;
            self.animation_started_at = Some(now - Duration::from_millis(reverse_ms));
            self.showing = self.wanted_showing;
        }
        let was_finished = self.finished;
        self.finished = !self.showing
            && now.saturating_duration_since(
                self.animation_started_at
                    .expect("now-playing animation start is initialized"),
            ) > TOAST_SLIDE_DURATION;
        if self.finished && !was_finished {
            self.update_toast = false;
        }
    }

    fn tick_music_note_color(&mut self, now: Instant) {
        if self.last_note_color_change.is_none_or(|previous| {
            now.saturating_duration_since(previous) > MUSIC_NOTE_COLOR_CHANGE_FREQUENCY
        }) {
            self.note_color_tick = self.note_color_tick.saturating_add(1);
            self.last_note_color_change = Some(now);
            self.note_color_argb = music_note_color(self.note_color_tick);
        }
    }
}

fn now_playing_translation_key(sound_name: &str) -> String {
    let (namespace, path) = sound_name
        .split_once(':')
        .unwrap_or(("minecraft", sound_name));
    let short = if namespace == "minecraft" {
        path.to_string()
    } else {
        format!("{namespace}.{path}")
    };
    short.replace('/', ".")
}

fn music_note_color(tick: i32) -> u32 {
    const COLORS: [bbb_render_types::VanillaDyeColor; 12] = [
        bbb_render_types::VanillaDyeColor::White,
        bbb_render_types::VanillaDyeColor::LightGray,
        bbb_render_types::VanillaDyeColor::LightBlue,
        bbb_render_types::VanillaDyeColor::Blue,
        bbb_render_types::VanillaDyeColor::Cyan,
        bbb_render_types::VanillaDyeColor::Green,
        bbb_render_types::VanillaDyeColor::Lime,
        bbb_render_types::VanillaDyeColor::Yellow,
        bbb_render_types::VanillaDyeColor::Orange,
        bbb_render_types::VanillaDyeColor::Pink,
        bbb_render_types::VanillaDyeColor::Red,
        bbb_render_types::VanillaDyeColor::Magenta,
    ];
    let value = tick.div_euclid(MUSIC_NOTE_COLOR_DURATION_TICKS);
    let color_count = i32::try_from(COLORS.len()).expect("music note color count fits i32");
    let first = COLORS[value.rem_euclid(color_count) as usize];
    let second = COLORS[(value + 1).rem_euclid(color_count) as usize];
    let sub_step = tick.rem_euclid(MUSIC_NOTE_COLOR_DURATION_TICKS) as f32
        / MUSIC_NOTE_COLOR_DURATION_TICKS as f32;
    super::argb_srgb_lerp(
        sub_step,
        modified_music_note_color(first) as i32,
        modified_music_note_color(second) as i32,
    ) as u32
}

fn modified_music_note_color(color: bbb_render_types::VanillaDyeColor) -> u32 {
    if color == bbb_render_types::VanillaDyeColor::White {
        return 0xFFE6_E6E6;
    }
    let diffuse = color.texture_diffuse_color();
    let brighten = |channel: u32| ((channel as f32 * 1.25).floor() as u32) & 0xff;
    0xFF00_0000
        | (brighten((diffuse >> 16) & 0xff) << 16)
        | (brighten((diffuse >> 8) & 0xff) << 8)
        | brighten(diffuse & 0xff)
}

#[derive(Debug, Default)]
pub(super) struct ClientToastRuntime {
    manager_generation: Option<u64>,
    pub(super) next_sequence: u64,
    next_periodic_sequence: usize,
    pub(super) queued: VecDeque<ClientToast>,
    pub(super) visible: Vec<ClientToastInstance>,
    pub(super) dropped_events: u64,
}

#[derive(Debug)]
pub(super) enum ClientToast {
    Advancement(ClientToastEvent),
    Recipe(RecipeToastState),
    Tutorial(TutorialToastState),
    System(SystemToastState),
}

#[derive(Debug)]
pub(super) struct RecipeToastState {
    pub(super) items: Vec<RecipeToastItem>,
    changed: bool,
    pub(super) last_changed_ms: u64,
    displayed_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SystemToastToken {
    NarratorToggle,
    ResourcePackLoadFailure,
    LowDiskSpace,
    ResourcePackDownload {
        session_generation: u64,
        batch_sequence: u64,
    },
}

#[derive(Debug)]
pub(super) struct SystemToastState {
    token: Option<SystemToastToken>,
    /// Vanilla retains the title `Component` and resolves it during render
    /// extraction. Current external-client owners use translatable titles, so
    /// keep that structured source separate from the construction/reset-time
    /// message projection and final width.
    title: SystemToastTitle,
    layout: SystemToastLayout,
    display_time_ms: u64,
    changed: bool,
    pub(super) last_changed_ms: u64,
    force_hide: bool,
}

#[derive(Debug)]
struct SystemToastLayout {
    message_lines: Vec<String>,
    width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemToastTitle {
    translation_key: String,
    fallback: String,
    args: Vec<String>,
}

impl SystemToastTitle {
    fn translated(
        translation_key: impl Into<String>,
        fallback: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            translation_key: translation_key.into(),
            fallback: fallback.into(),
            args,
        }
    }

    fn resolve(&self, item_runtime: Option<&NativeItemRuntime>) -> String {
        match self.args.as_slice() {
            [] => localized_gui_text(item_runtime, &self.translation_key, &self.fallback),
            [first] => localized_gui_text_with_args(
                item_runtime,
                &self.translation_key,
                &self.fallback,
                &[first],
            ),
            [first, second] => localized_gui_text_with_args(
                item_runtime,
                &self.translation_key,
                &self.fallback,
                &[first, second],
            ),
            args => {
                let args = args.iter().map(String::as_str).collect::<Vec<_>>();
                localized_gui_text_with_args(
                    item_runtime,
                    &self.translation_key,
                    &self.fallback,
                    &args,
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct RecipeToastItem {
    category_item: ItemStackSummary,
    pub(super) unlocked_item: ItemStackSummary,
}

#[derive(Debug)]
pub(super) struct TutorialToastState {
    pub(super) token: ClientTutorialToastToken,
    icon: HudTutorialToastIcon,
    pub(super) lines: Vec<Vec<HudStyledTextRun>>,
    pub(super) height: u32,
    target_progress: Option<f32>,
    smoothed_progress: Option<f32>,
    last_smoothing_ms: u64,
    pub(super) time_to_display_ms: u64,
    pub(super) force_hide: bool,
}

impl RecipeToastState {
    pub(super) fn displayed_item(&self) -> Option<&RecipeToastItem> {
        self.items.get(self.displayed_index)
    }
}

#[derive(Debug)]
pub(super) struct ClientToastInstance {
    pub(super) toast: ClientToast,
    pub(super) slot: u8,
    animation_started_at: Option<Instant>,
    became_fully_visible_at: Option<Instant>,
    pub(super) showing: bool,
    pub(super) visible_portion: f32,
    pub(super) fully_visible_ms: u64,
    finished: bool,
}

impl ClientToast {
    pub(super) fn advancement_event(&self) -> Option<&ClientToastEvent> {
        match self {
            Self::Advancement(event) => Some(event),
            Self::Recipe(_) | Self::Tutorial(_) | Self::System(_) => None,
        }
    }

    fn append_recipe(&mut self, item: RecipeToastItem) -> bool {
        let Self::Recipe(recipe) = self else {
            return false;
        };
        recipe.items.push(item);
        recipe.changed = true;
        true
    }

    pub(super) fn occupied_slot_count(&self) -> usize {
        match self {
            Self::Advancement(_) | Self::Recipe(_) => 1,
            Self::Tutorial(tutorial) => {
                usize::try_from(tutorial.height.div_ceil(32)).unwrap_or(usize::MAX)
            }
            Self::System(system) => system
                .layout
                .message_lines
                .len()
                .max(1)
                .saturating_mul(12)
                .saturating_add(20)
                .div_ceil(32),
        }
    }

    fn system_token(&self) -> Option<&SystemToastToken> {
        let Self::System(system) = self else {
            return None;
        };
        system.token.as_ref()
    }
}

impl ClientToastRuntime {
    #[cfg(test)]
    pub(super) fn update_with_resources(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        glyphs: &HudFontGlyphMap,
        now: Instant,
    ) -> Vec<LocalSoundEventState> {
        self.update_with_options(world, item_runtime, glyphs, None, 1.0, now)
    }

    pub(super) fn update_with_options(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        glyphs: &HudFontGlyphMap,
        periodic_notification: Option<PeriodicNotificationToast>,
        notification_display_time: f32,
        now: Instant,
    ) -> Vec<LocalSoundEventState> {
        let manager_generation = world.client_toasts().generation;
        let canonical_next_sequence = world.client_toasts().next_sequence;
        if self.manager_generation != Some(manager_generation)
            || canonical_next_sequence < self.next_sequence
        {
            let dropped_events = self.dropped_events;
            let next_periodic_sequence = self.next_periodic_sequence;
            *self = Self {
                manager_generation: Some(manager_generation),
                next_periodic_sequence,
                dropped_events,
                ..Self::default()
            };
        }

        for event in world.client_toast_events() {
            if event.sequence < self.next_sequence {
                continue;
            }
            if event.sequence > self.next_sequence {
                self.dropped_events = self
                    .dropped_events
                    .saturating_add(event.sequence - self.next_sequence);
            }
            self.next_sequence = event.sequence.saturating_add(1);
            self.ingest_event(event.clone(), item_runtime, glyphs);
        }
        if let Some(notification) = periodic_notification
            .filter(|notification| notification.sequence > self.next_periodic_sequence)
        {
            self.next_periodic_sequence = notification.sequence;
            self.ingest_periodic_notification(notification, item_runtime, glyphs);
        }

        let active_tutorial_toast = world.active_tutorial_toast();
        self.reconcile_tutorial_progress(active_tutorial_toast);
        let mut sounds = Vec::new();
        let mut visibility_sound_played = false;
        self.visible.retain_mut(|toast| {
            let transition = toast.update(now, active_tutorial_toast, notification_display_time);
            if !visibility_sound_played {
                if let Some(showing) = transition {
                    visibility_sound_played = true;
                    sounds.push(ui_sound(if showing {
                        "minecraft:ui.toast.in"
                    } else {
                        "minecraft:ui.toast.out"
                    }));
                }
            }
            !toast.finished
        });

        let mut challenge_sound_played = false;
        let mut queue_index = 0;
        while queue_index < self.queued.len() {
            let required = self.queued[queue_index].occupied_slot_count();
            let Some(slot) = self.find_free_slots(required) else {
                queue_index += 1;
                continue;
            };
            let toast = self
                .queued
                .remove(queue_index)
                .expect("selected queued toast exists");
            if toast
                .advancement_event()
                .and_then(ClientToastEvent::advancement)
                .is_some_and(|(_, display)| display.frame_type == AdvancementFrameType::Challenge)
                && !challenge_sound_played
            {
                sounds.push(ui_sound("minecraft:ui.toast.challenge_complete"));
                challenge_sound_played = true;
            }
            self.visible.push(ClientToastInstance {
                toast,
                slot,
                animation_started_at: None,
                became_fully_visible_at: None,
                showing: false,
                visible_portion: 0.0,
                fully_visible_ms: 0,
                finished: false,
            });
        }
        sounds
    }

    #[cfg(test)]
    pub(super) fn update(&mut self, world: &WorldStore, now: Instant) -> Vec<LocalSoundEventState> {
        self.update_with_resources(world, None, &HudFontGlyphMap::default(), now)
    }

    fn ingest_event(
        &mut self,
        event: ClientToastEvent,
        item_runtime: Option<&NativeItemRuntime>,
        glyphs: &HudFontGlyphMap,
    ) {
        let toast = if event.advancement().is_some() {
            ClientToast::Advancement(event)
        } else if let Some((_, category_item, unlocked_item)) = event.recipe() {
            let item = RecipeToastItem {
                category_item: category_item.clone(),
                unlocked_item: unlocked_item.clone(),
            };
            if self
                .visible
                .iter_mut()
                .any(|toast| toast.toast.append_recipe(item.clone()))
                || self
                    .queued
                    .iter_mut()
                    .any(|toast| toast.append_recipe(item.clone()))
            {
                return;
            }
            ClientToast::Recipe(RecipeToastState {
                items: vec![item],
                changed: true,
                last_changed_ms: 0,
                displayed_index: 0,
            })
        } else if let Some(tutorial) = event.tutorial() {
            match tutorial {
                ClientTutorialToastEvent::Show {
                    token,
                    kind,
                    time_to_display_ms,
                } => ClientToast::Tutorial(tutorial_toast_state(
                    token,
                    kind,
                    u64::from(time_to_display_ms),
                    item_runtime,
                    glyphs,
                )),
                ClientTutorialToastEvent::Hide { token } => {
                    if let Some(tutorial) = self.find_tutorial_mut(token) {
                        tutorial.force_hide = true;
                    }
                    return;
                }
            }
        } else if let Some(system) = event.system() {
            self.ingest_system_event(system.clone(), item_runtime, glyphs);
            return;
        } else {
            return;
        };

        self.push_toast(toast);
    }

    fn push_toast(&mut self, toast: ClientToast) {
        if self.queued.len() == MAX_NATIVE_TOASTS {
            self.queued.pop_front();
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
        self.queued.push_back(toast);
    }

    fn ingest_system_event(
        &mut self,
        event: ClientSystemToastEvent,
        item_runtime: Option<&NativeItemRuntime>,
        glyphs: &HudFontGlyphMap,
    ) {
        if let ClientSystemToastEvent::ResourcePackDownload {
            session_generation,
            state,
        } = &event
        {
            let token = SystemToastToken::ResourcePackDownload {
                session_generation: *session_generation,
                batch_sequence: state.batch_sequence,
            };
            if state.toast_action == ResourcePackDownloadToastAction::ForceHide {
                if let Some(toast) = self.find_system_mut(&token) {
                    toast.force_hide = true;
                }
                return;
            }
        }
        let preserve_message = matches!(
            &event,
            ClientSystemToastEvent::ResourcePackDownload { state, .. }
                if state.toast_action
                    == ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage
        );
        let (toast, update_existing) = system_toast_state(event, item_runtime, glyphs);
        if update_existing {
            if let Some(token) = toast.token.as_ref() {
                if let Some(existing) = self.find_system_mut(token) {
                    // `SystemToast.reset` updates content and lifetime, but its
                    // construction-time width remains final.
                    existing.title = toast.title;
                    if !preserve_message {
                        existing.layout.message_lines = toast.layout.message_lines;
                    }
                    existing.changed = true;
                    return;
                }
            }
        }
        self.push_toast(ClientToast::System(toast));
    }

    fn ingest_periodic_notification(
        &mut self,
        notification: PeriodicNotificationToast,
        item_runtime: Option<&NativeItemRuntime>,
        glyphs: &HudFontGlyphMap,
    ) {
        let elapsed = notification.elapsed_periods.to_string();
        let title = SystemToastTitle::translated(
            notification.title_key.clone(),
            notification.title_key,
            vec![elapsed.clone()],
        );
        let message = localized_gui_text_with_args(
            item_runtime,
            &notification.message_key,
            &notification.message_key,
            &[&elapsed],
        );
        self.push_toast(ClientToast::System(SystemToastState {
            token: None,
            layout: standard_system_toast_layout(
                title.resolve(item_runtime),
                Some(message),
                glyphs,
            ),
            title,
            display_time_ms: 5_000,
            changed: false,
            last_changed_ms: 0,
            force_hide: false,
        }));
    }

    fn find_system_mut(&mut self, token: &SystemToastToken) -> Option<&mut SystemToastState> {
        self.visible
            .iter_mut()
            .map(|instance| &mut instance.toast)
            .chain(self.queued.iter_mut())
            .find(|toast| toast.system_token() == Some(token))
            .and_then(|toast| match toast {
                ClientToast::System(system) => Some(system),
                _ => None,
            })
    }

    fn find_tutorial_mut(
        &mut self,
        token: ClientTutorialToastToken,
    ) -> Option<&mut TutorialToastState> {
        self.visible
            .iter_mut()
            .map(|instance| &mut instance.toast)
            .chain(self.queued.iter_mut())
            .find_map(|toast| match toast {
                ClientToast::Tutorial(tutorial) if tutorial.token == token => Some(tutorial),
                _ => None,
            })
    }

    fn reconcile_tutorial_progress(&mut self, active: Option<ClientTutorialToastState>) {
        for toast in self
            .visible
            .iter_mut()
            .map(|instance| &mut instance.toast)
            .chain(self.queued.iter_mut())
        {
            let ClientToast::Tutorial(tutorial) = toast else {
                continue;
            };
            if let Some(active) =
                active.filter(|active| tutorial.token == ClientTutorialToastToken::Step(active.id))
            {
                tutorial.target_progress = active
                    .progress_millionths
                    .map(|progress| progress as f32 / 1_000_000.0);
            }
        }
    }

    fn find_free_slots(&self, required: usize) -> Option<u8> {
        if required == 0 || required > HUD_TOAST_SLOTS {
            return None;
        }
        (0..=HUD_TOAST_SLOTS - required)
            .find(|first| {
                let end = first.saturating_add(required);
                self.visible.iter().all(|toast| {
                    let occupied_first = usize::from(toast.slot);
                    let occupied_end =
                        occupied_first.saturating_add(toast.toast.occupied_slot_count());
                    end <= occupied_first || *first >= occupied_end
                })
            })
            .and_then(|slot| u8::try_from(slot).ok())
    }

    #[cfg(test)]
    pub(super) fn project_advancements(
        &self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        terrain_textures: &TerrainTextureState,
        keybind_context: ItemModelKeybindContext,
        partial_tick: f32,
    ) -> Vec<HudAdvancementToast> {
        let Some(item_runtime) = item_runtime else {
            return Vec::new();
        };
        self.visible
            .iter()
            .filter_map(|instance| {
                project_advancement(
                    instance,
                    world,
                    item_runtime,
                    terrain_textures,
                    keybind_context,
                    partial_tick,
                )
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn project_recipes(
        &self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        terrain_textures: &TerrainTextureState,
        keybind_context: ItemModelKeybindContext,
        partial_tick: f32,
    ) -> Vec<HudRecipeToast> {
        self.visible
            .iter()
            .filter_map(|instance| {
                project_recipe(
                    instance,
                    world,
                    item_runtime,
                    terrain_textures,
                    keybind_context,
                    partial_tick,
                )
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn project_tutorials(&self) -> Vec<HudTutorialToast> {
        self.visible.iter().filter_map(project_tutorial).collect()
    }

    #[cfg(test)]
    pub(super) fn project_systems(
        &self,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> Vec<HudSystemToastInstance> {
        self.visible
            .iter()
            .filter_map(|instance| project_system(instance, item_runtime))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn project_toasts(
        &self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        terrain_textures: &TerrainTextureState,
        keybind_context: ItemModelKeybindContext,
        partial_tick: f32,
    ) -> Vec<HudToast> {
        self.visible
            .iter()
            .filter_map(|instance| match &instance.toast {
                ClientToast::System(_) => {
                    project_system(instance, item_runtime).map(HudToast::System)
                }
                ClientToast::Advancement(_) => item_runtime.and_then(|item_runtime| {
                    project_advancement(
                        instance,
                        world,
                        item_runtime,
                        terrain_textures,
                        keybind_context,
                        partial_tick,
                    )
                    .map(HudToast::Advancement)
                }),
                ClientToast::Recipe(_) => project_recipe(
                    instance,
                    world,
                    item_runtime,
                    terrain_textures,
                    keybind_context,
                    partial_tick,
                )
                .map(HudToast::Recipe),
                ClientToast::Tutorial(_) => project_tutorial(instance).map(HudToast::Tutorial),
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
fn project_advancement(
    instance: &ClientToastInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Option<HudAdvancementToast> {
    let (_, display) = instance
        .toast
        .advancement_event()?
        .advancement()
        .expect("advancement toast wraps an advancement event");
    let stack = ItemStackSummary {
        item_id: Some(display.icon.item_id),
        count: display.icon.count,
        component_patch: display.icon.component_patch.clone(),
    };
    let icon = hud_fake_item_icon_for_stack(
        world,
        Some(item_runtime),
        &stack,
        None,
        false,
        false,
        false,
        false,
        false,
        keybind_context,
        0,
        partial_tick,
    )?;
    let (translation_key, fallback, frame_label_color) = match display.frame_type {
        AdvancementFrameType::Task => ("advancements.toast.task", "Advancement Made!", 0xFF_FF_00),
        AdvancementFrameType::Challenge => (
            "advancements.toast.challenge",
            "Challenge Complete!",
            0xFF_88_FF,
        ),
        AdvancementFrameType::Goal => ("advancements.toast.goal", "Goal Reached!", 0xFF_FF_00),
    };
    Some(HudAdvancementToast {
        frame_label: localized_gui_text(Some(item_runtime), translation_key, fallback),
        frame_label_color,
        title_runs: hud_styled_runs_from_rich_text(&display.title),
        icon,
        block_model: fake_block_item_3d_model(
            world,
            &stack,
            Some(item_runtime),
            terrain_textures,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
        ),
        slot: instance.slot,
        visible_portion: instance.visible_portion,
        fully_visible_ms: instance.fully_visible_ms,
    })
}

#[allow(clippy::too_many_arguments)]
fn project_recipe(
    instance: &ClientToastInstance,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Option<HudRecipeToast> {
    let ClientToast::Recipe(recipe) = &instance.toast else {
        return None;
    };
    let item = recipe.displayed_item()?;
    let category_icon = hud_fake_item_icon_for_stack(
        world,
        item_runtime,
        &item.category_item,
        None,
        false,
        false,
        false,
        false,
        false,
        keybind_context,
        0,
        partial_tick,
    );
    let unlocked_icon = hud_fake_item_icon_for_stack(
        world,
        item_runtime,
        &item.unlocked_item,
        None,
        false,
        false,
        false,
        false,
        false,
        keybind_context,
        0,
        partial_tick,
    );
    Some(HudRecipeToast {
        title: localized_gui_text(
            item_runtime,
            "recipe.toast.title",
            "New Recipe(s) Unlocked!",
        ),
        description: localized_gui_text(
            item_runtime,
            "recipe.toast.description",
            "Check your recipe book",
        ),
        category_icon,
        category_block_model: fake_block_item_3d_model(
            world,
            &item.category_item,
            item_runtime,
            terrain_textures,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
        ),
        unlocked_icon,
        unlocked_block_model: fake_block_item_3d_model(
            world,
            &item.unlocked_item,
            item_runtime,
            terrain_textures,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
        ),
        slot: instance.slot,
        visible_portion: instance.visible_portion,
    })
}

fn project_tutorial(instance: &ClientToastInstance) -> Option<HudTutorialToast> {
    let ClientToast::Tutorial(tutorial) = &instance.toast else {
        return None;
    };
    Some(HudTutorialToast {
        icon: tutorial.icon,
        lines: tutorial.lines.clone(),
        slot: instance.slot,
        visible_portion: instance.visible_portion,
        progress: tutorial.smoothed_progress,
        progress_increasing: tutorial
            .target_progress
            .zip(tutorial.smoothed_progress)
            .is_none_or(|(target, smoothed)| target >= smoothed),
    })
}

fn project_system(
    instance: &ClientToastInstance,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudSystemToastInstance> {
    let ClientToast::System(system) = &instance.toast else {
        return None;
    };
    Some(HudSystemToastInstance {
        toast: HudSystemToast {
            title: system.title.resolve(item_runtime),
            message_lines: system.layout.message_lines.clone(),
            width: system.layout.width,
            visible_portion: instance.visible_portion,
        },
        slot: instance.slot,
    })
}

impl ClientToastInstance {
    fn update(
        &mut self,
        now: Instant,
        active_tutorial_toast: Option<ClientTutorialToastState>,
        notification_display_time: f32,
    ) -> Option<bool> {
        let showing_before_update = self.showing;
        let animation_started_at = *self.animation_started_at.get_or_insert_with(|| {
            self.showing = true;
            now
        });
        if self.showing
            && now.saturating_duration_since(animation_started_at) <= TOAST_SLIDE_DURATION
        {
            self.became_fully_visible_at = Some(now);
        }
        let became_fully_visible_at = self.became_fully_visible_at.unwrap_or(now);
        self.fully_visible_ms = now
            .saturating_duration_since(became_fully_visible_at)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let progress = (now
            .saturating_duration_since(animation_started_at)
            .as_secs_f32()
            / TOAST_SLIDE_DURATION.as_secs_f32())
        .clamp(0.0, 1.0)
        .powi(2);
        self.visible_portion = if self.showing {
            progress
        } else {
            1.0 - progress
        };
        let wanted_showing = match &mut self.toast {
            ClientToast::Advancement(_) => display_time_remaining(
                self.fully_visible_ms,
                TOAST_VISIBLE_DURATION.as_millis() as u64,
                notification_display_time,
            ),
            ClientToast::Recipe(recipe) => {
                if recipe.changed {
                    recipe.last_changed_ms = self.fully_visible_ms;
                    recipe.changed = false;
                }
                let elapsed = self.fully_visible_ms.saturating_sub(recipe.last_changed_ms);
                let item_count = recipe.items.len();
                if item_count > 0 {
                    let cycle_ms = (TOAST_VISIBLE_DURATION.as_millis() as f64
                        * f64::from(notification_display_time)
                        / item_count as f64)
                        .max(1.0);
                    recipe.displayed_index =
                        ((self.fully_visible_ms as f64 / cycle_ms) as usize) % item_count;
                }
                display_time_remaining(
                    elapsed,
                    TOAST_VISIBLE_DURATION.as_millis() as u64,
                    notification_display_time,
                )
            }
            ClientToast::Tutorial(tutorial) => {
                if tutorial.time_to_display_ms > 0 {
                    let progress = (self.fully_visible_ms as f32
                        / tutorial.time_to_display_ms as f32)
                        .min(1.0);
                    tutorial.target_progress = Some(progress);
                    tutorial.smoothed_progress = Some(progress);
                    tutorial.last_smoothing_ms = self.fully_visible_ms;
                } else if let (Some(smoothed), Some(target)) = (
                    tutorial.smoothed_progress.as_mut(),
                    tutorial.target_progress,
                ) {
                    let smoothing = self
                        .fully_visible_ms
                        .saturating_sub(tutorial.last_smoothing_ms)
                        as f32
                        / 100.0;
                    *smoothed += (target - *smoothed) * smoothing.clamp(0.0, 1.0);
                    tutorial.last_smoothing_ms = self.fully_visible_ms;
                }
                !tutorial.force_hide
                    && if tutorial.time_to_display_ms > 0 {
                        // The duration-bound constructor hides strictly after
                        // the requested duration, not at its final sample.
                        self.fully_visible_ms <= tutorial.time_to_display_ms
                    } else {
                        active_tutorial_toast.is_some_and(|active| {
                            tutorial.token == ClientTutorialToastToken::Step(active.id)
                        })
                    }
            }
            ClientToast::System(system) => {
                if system.changed {
                    system.last_changed_ms = self.fully_visible_ms;
                    system.changed = false;
                }
                !system.force_hide
                    && display_time_remaining(
                        self.fully_visible_ms.saturating_sub(system.last_changed_ms),
                        system.display_time_ms,
                        notification_display_time,
                    )
            }
        };
        if wanted_showing != self.showing {
            let reverse_ms =
                ((1.0 - self.visible_portion) * TOAST_SLIDE_DURATION.as_millis() as f32) as u64;
            self.animation_started_at = Some(now - Duration::from_millis(reverse_ms));
            self.showing = wanted_showing;
        }
        self.finished = !self.showing
            && now.saturating_duration_since(
                self.animation_started_at
                    .expect("advancement toast animation start is initialized"),
            ) > TOAST_SLIDE_DURATION;
        (showing_before_update != self.showing).then_some(self.showing)
    }
}

fn display_time_remaining(elapsed_ms: u64, duration_ms: u64, multiplier: f32) -> bool {
    (elapsed_ms as f64) < duration_ms as f64 * f64::from(multiplier.max(0.0))
}

fn system_toast_state(
    event: ClientSystemToastEvent,
    item_runtime: Option<&NativeItemRuntime>,
    glyphs: &HudFontGlyphMap,
) -> (SystemToastState, bool) {
    let (token, title, message, multiline, display_time_ms, update_existing) = match event {
        ClientSystemToastEvent::NarratorToggle {
            enabled,
            status_name,
        } => {
            let (title, message) = if enabled {
                (
                    SystemToastTitle::translated(
                        "narrator.toast.enabled",
                        "Narrator Enabled",
                        Vec::new(),
                    ),
                    Some(status_name),
                )
            } else {
                (
                    SystemToastTitle::translated(
                        "narrator.toast.disabled",
                        "Narrator Disabled",
                        Vec::new(),
                    ),
                    None,
                )
            };
            (
                Some(SystemToastToken::NarratorToggle),
                title,
                message,
                false,
                5_000,
                true,
            )
        }
        ClientSystemToastEvent::InsecureServerWarning => {
            let title = SystemToastTitle::translated(
                "multiplayer.unsecureserver.toast.title",
                "Chat messages can't be verified",
                Vec::new(),
            );
            let message = localized_gui_text(
                item_runtime,
                "multiplayer.unsecureserver.toast",
                "Messages sent on this server may be modified and might not reflect the original message",
            );
            (None, title, Some(message), true, 10_000, false)
        }
        ClientSystemToastEvent::ResourcePackLoadFailure => (
            Some(SystemToastToken::ResourcePackLoadFailure),
            SystemToastTitle::translated(
                "resourcePack.load_fail",
                "Resource reload failed",
                Vec::new(),
            ),
            None,
            false,
            5_000,
            true,
        ),
        ClientSystemToastEvent::LowDiskSpace => (
            Some(SystemToastToken::LowDiskSpace),
            SystemToastTitle::translated("chunk.toast.lowDiskSpace", "Low disk space!", Vec::new()),
            Some(localized_gui_text(
                item_runtime,
                "chunk.toast.lowDiskSpace.description",
                "Might not be able to save the world.",
            )),
            false,
            10_000,
            true,
        ),
        ClientSystemToastEvent::ResourcePackDownload {
            session_generation,
            state,
        } => {
            let (title, message) = download_system_toast_content(&state, item_runtime);
            (
                Some(SystemToastToken::ResourcePackDownload {
                    session_generation,
                    batch_sequence: state.batch_sequence,
                }),
                title,
                message,
                false,
                5_000,
                true,
            )
        }
    };
    let layout = if multiline {
        multiline_system_toast_layout(
            message.expect("multiline system toast has a message"),
            glyphs,
        )
    } else {
        standard_system_toast_layout(title.resolve(item_runtime), message, glyphs)
    };
    (
        SystemToastState {
            token,
            title,
            layout,
            display_time_ms,
            changed: false,
            last_changed_ms: 0,
            force_hide: false,
        },
        update_existing,
    )
}

fn standard_system_toast_layout(
    title: String,
    message: Option<String>,
    glyphs: &HudFontGlyphMap,
) -> SystemToastLayout {
    let width = glyphs
        .plain_text_width(&title)
        .max(
            message
                .as_deref()
                .map_or(0, |message| glyphs.plain_text_width(message)),
        )
        .saturating_add(30)
        .max(160);
    SystemToastLayout {
        message_lines: message.into_iter().collect(),
        width,
    }
}

fn multiline_system_toast_layout(message: String, glyphs: &HudFontGlyphMap) -> SystemToastLayout {
    let message_lines = hud_wrap_styled_text(&[HudStyledTextRun::plain(message)], 200, glyphs)
        .into_iter()
        .map(|line| line.into_iter().map(|run| run.text).collect::<String>())
        .collect::<Vec<_>>();
    let width = message_lines
        .iter()
        .map(|line| glyphs.plain_text_width(line))
        .max()
        .unwrap_or(200)
        .max(200)
        .saturating_add(30);
    SystemToastLayout {
        message_lines,
        width,
    }
}

fn download_system_toast_content(
    download: &ResourcePackDownloadState,
    item_runtime: Option<&NativeItemRuntime>,
) -> (SystemToastTitle, Option<String>) {
    let total_packs = download.total_packs.to_string();
    if download.phase == ResourcePackDownloadPhase::Failed {
        let failed_packs = download.failed_packs.to_string();
        return (
            SystemToastTitle::translated(
                "download.pack.failed",
                "%s out of %s pack(s) failed to download",
                vec![failed_packs, total_packs],
            ),
            None,
        );
    }
    let current_pack = download.current_pack.to_string();
    let progress = match download.total_bytes.filter(|total| *total > 0) {
        Some(total_bytes) => {
            let percent = download.downloaded_bytes.saturating_mul(100) / total_bytes;
            localized_gui_text_with_args(
                item_runtime,
                "download.pack.progress.percent",
                "Progress: %s%%",
                &[&percent.to_string()],
            )
        }
        None => localized_gui_text_with_args(
            item_runtime,
            "download.pack.progress.bytes",
            "Progress: %s (total size unknown)",
            &[&server_pack_bytes_human_readable(download.downloaded_bytes)],
        ),
    };
    (
        SystemToastTitle::translated(
            "download.pack.title",
            "Downloading resource pack %s/%s",
            vec![current_pack, total_packs],
        ),
        (download.toast_action == ResourcePackDownloadToastAction::AddOrUpdateProgress)
            .then_some(progress),
    )
}

fn tutorial_toast_state(
    token: ClientTutorialToastToken,
    kind: TutorialToastKind,
    time_to_display_ms: u64,
    item_runtime: Option<&NativeItemRuntime>,
    glyphs: &HudFontGlyphMap,
) -> TutorialToastState {
    let key = |translation_key, default_text| HudStyledTextRun {
        text: localized_gui_text(item_runtime, translation_key, default_text),
        style: HudTextStyle {
            bold: true,
            ..HudTextStyle::default()
        },
        color: None,
    };
    let plain = |translation_key, default_text| {
        vec![HudStyledTextRun::plain(localized_gui_text(
            item_runtime,
            translation_key,
            default_text,
        ))]
    };
    let with_keys = |translation_key, default_text, keys: &[HudStyledTextRun]| {
        localized_gui_styled_text_with_args(item_runtime, translation_key, default_text, keys)
    };
    let (icon, mut title, description) = match kind {
        TutorialToastKind::Movement => {
            let forward = key("key.keyboard.w", "W");
            let left = key("key.keyboard.a", "A");
            let back = key("key.keyboard.s", "S");
            let right = key("key.keyboard.d", "D");
            let jump = key("key.keyboard.space", "Space");
            (
                HudTutorialToastIcon::MovementKeys,
                with_keys(
                    "tutorial.move.title",
                    "Move with %s, %s, %s and %s",
                    &[forward, left, back, right],
                ),
                with_keys("tutorial.move.description", "Jump with %s", &[jump]),
            )
        }
        TutorialToastKind::Look => (
            HudTutorialToastIcon::Mouse,
            plain("tutorial.look.title", "Look around"),
            plain("tutorial.look.description", "Use your mouse to turn"),
        ),
        TutorialToastKind::FindTree => (
            HudTutorialToastIcon::Tree,
            plain("tutorial.find_tree.title", "Find a tree"),
            plain("tutorial.find_tree.description", "Punch it to collect wood"),
        ),
        TutorialToastKind::PunchTree => {
            let attack = key("key.mouse.left", "Left Button");
            (
                HudTutorialToastIcon::Tree,
                plain("tutorial.punch_tree.title", "Destroy the tree"),
                with_keys("tutorial.punch_tree.description", "Hold down %s", &[attack]),
            )
        }
        TutorialToastKind::OpenInventory => {
            let inventory = key("key.keyboard.e", "E");
            (
                HudTutorialToastIcon::RecipeBook,
                plain("tutorial.open_inventory.title", "Open your inventory"),
                with_keys(
                    "tutorial.open_inventory.description",
                    "Press %s",
                    &[inventory],
                ),
            )
        }
        TutorialToastKind::CraftPlanks => (
            HudTutorialToastIcon::WoodenPlanks,
            plain("tutorial.craft_planks.title", "Craft wooden planks"),
            plain(
                "tutorial.craft_planks.description",
                "The recipe book can help",
            ),
        ),
        TutorialToastKind::SocialInteractions => {
            let bound_key = crate::client_options::key_bindings::bound_key_display_key(
                "key.socialInteractions",
            )
            .unwrap_or("key.keyboard.p");
            let social_interactions = key(bound_key, "P");
            (
                HudTutorialToastIcon::SocialInteractions,
                plain("tutorial.socialInteractions.title", "Social Interactions"),
                with_keys(
                    "tutorial.socialInteractions.description",
                    "Press %s to open",
                    &[social_interactions],
                ),
            )
        }
    };
    for run in &mut title {
        run.color = Some(0x50_0050);
    }
    let mut lines = hud_wrap_styled_text(&title, 126, glyphs);
    lines.extend(hud_wrap_styled_text(&description, 126, glyphs));
    let height = 7u32
        .saturating_add(
            u32::try_from(lines.len().max(2))
                .unwrap_or(u32::MAX)
                .saturating_mul(11),
        )
        .saturating_add(3);
    let progress = kind.progressable().then_some(0.0);
    TutorialToastState {
        token,
        icon,
        lines,
        height,
        target_progress: progress,
        smoothed_progress: progress,
        last_smoothing_ms: 0,
        time_to_display_ms,
        force_hide: false,
    }
}

fn localized_gui_styled_text_with_args(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    default_text: &str,
    args: &[HudStyledTextRun],
) -> Vec<HudStyledTextRun> {
    let template = localized_gui_text(item_runtime, translation_key, default_text);
    format_localized_styled_arguments(&template, args)
}

fn format_localized_styled_arguments(
    template: &str,
    args: &[HudStyledTextRun],
) -> Vec<HudStyledTextRun> {
    let bytes = template.as_bytes();
    let mut result = Vec::new();
    let mut offset = 0;
    let mut sequential = 0usize;
    while offset < bytes.len() {
        let Some(relative_percent) = bytes[offset..].iter().position(|byte| *byte == b'%') else {
            push_tutorial_text_run(
                &mut result,
                &template[offset..],
                HudTextStyle::default(),
                None,
            );
            break;
        };
        let percent = offset + relative_percent;
        push_tutorial_text_run(
            &mut result,
            &template[offset..percent],
            HudTextStyle::default(),
            None,
        );
        if bytes.get(percent + 1) == Some(&b'%') {
            push_tutorial_text_run(&mut result, "%", HudTextStyle::default(), None);
            offset = percent + 2;
            continue;
        }
        if bytes.get(percent + 1) == Some(&b's') {
            if let Some(argument) = args.get(sequential) {
                push_tutorial_text_run(&mut result, &argument.text, argument.style, argument.color);
            } else {
                push_tutorial_text_run(&mut result, "%s", HudTextStyle::default(), None);
            }
            sequential += 1;
            offset = percent + 2;
            continue;
        }

        let mut cursor = percent + 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        let positional = (cursor > percent + 1
            && bytes.get(cursor) == Some(&b'$')
            && bytes.get(cursor + 1) == Some(&b's'))
        .then(|| {
            template[percent + 1..cursor]
                .parse::<usize>()
                .ok()
                .and_then(|index| index.checked_sub(1))
        })
        .flatten();
        if let Some(index) = positional {
            if let Some(argument) = args.get(index) {
                push_tutorial_text_run(&mut result, &argument.text, argument.style, argument.color);
            } else {
                push_tutorial_text_run(
                    &mut result,
                    &template[percent..cursor + 2],
                    HudTextStyle::default(),
                    None,
                );
            }
            offset = cursor + 2;
        } else {
            push_tutorial_text_run(&mut result, "%", HudTextStyle::default(), None);
            offset = percent + 1;
        }
    }
    result
}

fn push_tutorial_text_run(
    runs: &mut Vec<HudStyledTextRun>,
    text: &str,
    style: HudTextStyle,
    color: Option<u32>,
) {
    if text.is_empty() {
        return;
    }
    if let Some(previous) = runs
        .last_mut()
        .filter(|previous| previous.style == style && previous.color == color)
    {
        previous.text.push_str(text);
    } else {
        runs.push(HudStyledTextRun {
            text: text.to_string(),
            style,
            color,
        });
    }
}

fn ui_sound(location: &str) -> LocalSoundEventState {
    LocalSoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(location.to_string()),
            fixed_range: None,
        },
        source: "master".to_string(),
        volume: 1.0,
        pitch: 1.0,
        seed: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pause_screen(show_pause_menu: bool) -> HudPauseScreen {
        HudPauseScreen {
            title: "Game Menu".to_string(),
            show_pause_menu,
            feedback_subscreen: false,
            return_to_game_hovered: false,
            advancements_hovered: false,
            stats_hovered: false,
            send_feedback_hovered: false,
            report_bugs_hovered: false,
            report_bugs_enabled: true,
            custom_dialog_label: None,
            custom_dialog_hovered: false,
            custom_dialog_tooltip: None,
            feedback_back_hovered: false,
            options_hovered: false,
            options_enabled: true,
            player_reporting_hovered: false,
            player_reporting_enabled: true,
            disconnect_hovered: false,
            disconnect_enabled: true,
            has_draft_report: false,
            draft_confirmation: None,
        }
    }

    #[test]
    fn tutorial_localized_arguments_preserve_bold_positional_runs_and_escaped_percent() {
        let bold = HudTextStyle {
            bold: true,
            ..HudTextStyle::default()
        };
        let runs = format_localized_styled_arguments(
            "%2$s %% %1$s",
            &[
                HudStyledTextRun {
                    text: "A".to_string(),
                    style: bold,
                    color: None,
                },
                HudStyledTextRun {
                    text: "B".to_string(),
                    style: bold,
                    color: None,
                },
            ],
        );

        assert_eq!(
            runs,
            vec![
                HudStyledTextRun {
                    text: "B".to_string(),
                    style: bold,
                    color: None,
                },
                HudStyledTextRun::plain(" % "),
                HudStyledTextRun {
                    text: "A".to_string(),
                    style: bold,
                    color: None,
                },
            ]
        );
    }

    #[test]
    fn now_playing_uses_actual_sound_key_independent_timing_and_pause_projection() {
        let mut world = WorldStore::new();
        world.show_now_playing_music("minecraft:music/game/creative/creative1".to_string());
        let start = Instant::now();
        let mut runtime = NowPlayingToastRuntime::default();

        let first = runtime
            .update_and_project(
                &world,
                None,
                crate::startup::MusicToastDisplayStateArg::PauseAndToast,
                1.0,
                1.0,
                None,
                10,
                start,
            )
            .expect("sliding now-playing toast");
        assert_eq!(first.text, "music.game.creative.creative1");
        assert_eq!(first.placement, HudNowPlayingToastPlacement::SlidingToast);
        assert_eq!(first.visible_portion, 0.0);

        let fully_visible = runtime
            .update_and_project(
                &world,
                None,
                crate::startup::MusicToastDisplayStateArg::PauseAndToast,
                1.0,
                9.0,
                None,
                22,
                start + Duration::from_millis(600),
            )
            .expect("fully visible toast");
        assert_eq!(fully_visible.visible_portion, 1.0);

        assert!(runtime
            .update_and_project(
                &world,
                None,
                crate::startup::MusicToastDisplayStateArg::PauseAndToast,
                1.0,
                9.0,
                None,
                123,
                start + Duration::from_millis(5_601),
            )
            .is_some());
        assert!(runtime
            .update_and_project(
                &world,
                None,
                crate::startup::MusicToastDisplayStateArg::PauseAndToast,
                1.0,
                9.0,
                None,
                124,
                start + Duration::from_millis(6_202),
            )
            .is_none());

        let color_after_toast = runtime.note_color_argb;
        assert!(runtime
            .update_and_project(
                &world,
                None,
                crate::startup::MusicToastDisplayStateArg::PauseAndToast,
                1.0,
                9.0,
                None,
                125,
                start + Duration::from_millis(7_000),
            )
            .is_none());
        assert_eq!(runtime.note_color_argb, color_after_toast);

        let pause = pause_screen(true);
        let paused = runtime
            .update_and_project(
                &world,
                None,
                crate::startup::MusicToastDisplayStateArg::Pause,
                1.0,
                1.0,
                Some(&pause),
                126,
                start + Duration::from_millis(7_001),
            )
            .expect("static pause-screen now-playing surface");
        assert_eq!(paused.placement, HudNowPlayingToastPlacement::PauseScreen);
        assert_eq!(paused.visible_portion, 1.0);
        assert_ne!(runtime.note_color_argb, color_after_toast);
        assert!(runtime
            .update_and_project(
                &world,
                None,
                crate::startup::MusicToastDisplayStateArg::Pause,
                0.0,
                1.0,
                Some(&pause),
                127,
                start + Duration::from_millis(7_002),
            )
            .is_none());
    }

    #[test]
    fn now_playing_translation_and_music_note_palette_match_vanilla() {
        assert_eq!(
            now_playing_translation_key("minecraft:music/game/end/boss1"),
            "music.game.end.boss1"
        );
        assert_eq!(
            now_playing_translation_key("example:music/custom/song"),
            "example.music.custom.song"
        );
        assert_eq!(music_note_color(0), 0xFFE6_E6E6);
        assert_eq!(music_note_color(30), 0xFFC4_C4BC);
        assert_eq!(
            modified_music_note_color(bbb_render_types::VanillaDyeColor::Yellow),
            0xFF3D_0E4C
        );
    }
}
