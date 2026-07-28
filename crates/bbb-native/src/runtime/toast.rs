//! Native ownership for Vanilla's shared bounded toast queue and wall-clock animation.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bbb_item_model::{ItemModelKeybindContext, NativeItemRuntime};
use bbb_protocol::packets::{AdvancementFrameType, ItemStackSummary};
use bbb_renderer::{
    hud_wrap_styled_text, HudAdvancementToast, HudFontGlyphMap, HudRecipeToast, HudStyledTextRun,
    HudTextStyle, HudTutorialToast, HudTutorialToastIcon, HUD_TOAST_SLOTS,
};
use bbb_world::{
    ClientToastEvent, ClientTutorialToastState, LocalSoundEventState, SoundHolderState,
    TutorialToastKind, WorldStore,
};

use crate::terrain_runtime::TerrainTextureState;

use super::{
    fake_block_item_3d_model, hud_fake_item_icon_for_stack, hud_styled_runs_from_rich_text,
    localized_gui_text,
};

const MAX_NATIVE_TOASTS: usize = bbb_world::MAX_CLIENT_TOAST_EVENTS;
const TOAST_SLIDE_DURATION: Duration = Duration::from_millis(600);
const TOAST_VISIBLE_DURATION: Duration = Duration::from_millis(5_000);

#[derive(Debug, Default)]
pub(super) struct ClientToastRuntime {
    pub(super) level_generation: Option<u64>,
    pub(super) next_sequence: u64,
    pub(super) queued: VecDeque<ClientToast>,
    pub(super) visible: Vec<ClientToastInstance>,
    pub(super) dropped_events: u64,
}

#[derive(Debug)]
pub(super) enum ClientToast {
    Advancement(ClientToastEvent),
    Recipe(RecipeToastState),
    Tutorial(TutorialToastState),
}

#[derive(Debug)]
pub(super) struct RecipeToastState {
    pub(super) items: Vec<RecipeToastItem>,
    changed: bool,
    pub(super) last_changed_ms: u64,
}

#[derive(Debug, Clone)]
pub(super) struct RecipeToastItem {
    category_item: ItemStackSummary,
    pub(super) unlocked_item: ItemStackSummary,
}

#[derive(Debug)]
pub(super) struct TutorialToastState {
    id: u64,
    icon: HudTutorialToastIcon,
    pub(super) lines: Vec<Vec<HudStyledTextRun>>,
    pub(super) height: u32,
    target_progress: Option<f32>,
    smoothed_progress: Option<f32>,
    last_smoothing_ms: u64,
}

impl RecipeToastState {
    pub(super) fn displayed_item(&self, fully_visible_ms: u64) -> Option<&RecipeToastItem> {
        let item_count = u64::try_from(self.items.len()).ok()?;
        let cycle_ms = (TOAST_VISIBLE_DURATION.as_millis() as u64 / item_count).max(1);
        let index = usize::try_from((fully_visible_ms / cycle_ms) % item_count).ok()?;
        self.items.get(index)
    }
}

#[derive(Debug)]
pub(super) struct ClientToastInstance {
    pub(super) toast: ClientToast,
    pub(super) slot: u8,
    animation_started_at: Option<Instant>,
    became_fully_visible_at: Option<Instant>,
    showing: bool,
    pub(super) visible_portion: f32,
    pub(super) fully_visible_ms: u64,
    finished: bool,
}

impl ClientToast {
    pub(super) fn advancement_event(&self) -> Option<&ClientToastEvent> {
        match self {
            Self::Advancement(event) => Some(event),
            Self::Recipe(_) | Self::Tutorial(_) => None,
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

    fn occupied_slot_count(&self) -> usize {
        match self {
            Self::Advancement(_) | Self::Recipe(_) => 1,
            Self::Tutorial(tutorial) => {
                usize::try_from(tutorial.height.div_ceil(32)).unwrap_or(usize::MAX)
            }
        }
    }
}

impl ClientToastRuntime {
    pub(super) fn update_with_resources(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        glyphs: &HudFontGlyphMap,
        now: Instant,
    ) -> Vec<LocalSoundEventState> {
        let level_generation = world.client_level_generation();
        let canonical_next_sequence = world.client_toasts().next_sequence;
        if self.level_generation != Some(level_generation)
            || canonical_next_sequence < self.next_sequence
        {
            let dropped_events = self.dropped_events;
            *self = Self {
                level_generation: Some(level_generation),
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

        let active_tutorial_toast = world.active_tutorial_toast();
        self.reconcile_tutorial_progress(active_tutorial_toast);
        let mut sounds = Vec::new();
        let mut visibility_sound_played = false;
        self.visible.retain_mut(|toast| {
            let transition = toast.update(now, active_tutorial_toast);
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
        loop {
            let Some((queue_index, slot)) =
                self.queued.iter().enumerate().find_map(|(index, toast)| {
                    self.find_free_slots(toast.occupied_slot_count())
                        .map(|slot| (index, slot))
                })
            else {
                break;
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
            })
        } else if let Some((id, kind)) = event.tutorial() {
            ClientToast::Tutorial(tutorial_toast_state(id, kind, item_runtime, glyphs))
        } else {
            return;
        };

        if self.queued.len() == MAX_NATIVE_TOASTS {
            self.queued.pop_front();
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
        self.queued.push_back(toast);
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
            if let Some(active) = active.filter(|active| active.id == tutorial.id) {
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
            .filter_map(|toast| {
                let (_, display) = toast
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
                    AdvancementFrameType::Task => {
                        ("advancements.toast.task", "Advancement Made!", 0xFF_FF_00)
                    }
                    AdvancementFrameType::Challenge => (
                        "advancements.toast.challenge",
                        "Challenge Complete!",
                        0xFF_88_FF,
                    ),
                    AdvancementFrameType::Goal => {
                        ("advancements.toast.goal", "Goal Reached!", 0xFF_FF_00)
                    }
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
                    slot: toast.slot,
                    visible_portion: toast.visible_portion,
                    fully_visible_ms: toast.fully_visible_ms,
                })
            })
            .collect()
    }

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
            .filter_map(|toast| {
                let ClientToast::Recipe(recipe) = &toast.toast else {
                    return None;
                };
                let item = recipe.displayed_item(toast.fully_visible_ms)?;
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
                    slot: toast.slot,
                    visible_portion: toast.visible_portion,
                })
            })
            .collect()
    }

    pub(super) fn project_tutorials(&self) -> Vec<HudTutorialToast> {
        self.visible
            .iter()
            .filter_map(|instance| {
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
            })
            .collect()
    }
}

impl ClientToastInstance {
    fn update(
        &mut self,
        now: Instant,
        active_tutorial_toast: Option<ClientTutorialToastState>,
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
            ClientToast::Advancement(_) => {
                Duration::from_millis(self.fully_visible_ms) < TOAST_VISIBLE_DURATION
            }
            ClientToast::Recipe(recipe) => {
                if recipe.changed {
                    recipe.last_changed_ms = self.fully_visible_ms;
                    recipe.changed = false;
                }
                self.fully_visible_ms.saturating_sub(recipe.last_changed_ms)
                    < TOAST_VISIBLE_DURATION.as_millis() as u64
            }
            ClientToast::Tutorial(tutorial) => {
                if let (Some(smoothed), Some(target)) = (
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
                active_tutorial_toast.is_some_and(|active| active.id == tutorial.id)
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

fn tutorial_toast_state(
    id: u64,
    kind: TutorialToastKind,
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
        id,
        icon,
        lines,
        height,
        target_progress: progress,
        smoothed_progress: progress,
        last_smoothing_ms: 0,
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
}
