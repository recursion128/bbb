//! Native ownership for Vanilla's shared bounded toast queue and wall-clock animation.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use bbb_item_model::{ItemModelKeybindContext, NativeItemRuntime};
use bbb_protocol::packets::{AdvancementFrameType, ItemStackSummary};
use bbb_renderer::{HudAdvancementToast, HudRecipeToast, HUD_TOAST_SLOTS};
use bbb_world::{ClientToastEvent, LocalSoundEventState, SoundHolderState, WorldStore};

use crate::terrain_runtime::TerrainTextureState;

use super::{
    block_item_3d_model, hud_item_icon_for_stack, hud_styled_runs_from_rich_text,
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
            Self::Recipe(_) => None,
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
}

impl ClientToastRuntime {
    pub(super) fn update(&mut self, world: &WorldStore, now: Instant) -> Vec<LocalSoundEventState> {
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
            self.ingest_event(event.clone());
        }

        let mut sounds = Vec::new();
        let mut visibility_sound_played = false;
        self.visible.retain_mut(|toast| {
            let transition = toast.update(now);
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
        while let Some(slot) = (0..HUD_TOAST_SLOTS)
            .map(|slot| u8::try_from(slot).expect("toast slot count fits u8"))
            .find(|slot| self.visible.iter().all(|toast| toast.slot != *slot))
        {
            let Some(toast) = self.queued.pop_front() else {
                break;
            };
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

    fn ingest_event(&mut self, event: ClientToastEvent) {
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
        } else {
            return;
        };

        if self.queued.len() == MAX_NATIVE_TOASTS {
            self.queued.pop_front();
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
        self.queued.push_back(toast);
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
                let icon = hud_item_icon_for_stack(
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
                    block_model: block_item_3d_model(
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
                let category_icon = hud_item_icon_for_stack(
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
                let unlocked_icon = hud_item_icon_for_stack(
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
                    category_block_model: block_item_3d_model(
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
                    unlocked_block_model: block_item_3d_model(
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
}

impl ClientToastInstance {
    fn update(&mut self, now: Instant) -> Option<bool> {
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
