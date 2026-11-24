//! World -> renderer per-frame projections.
//!
//! Pure functions that read `WorldStore` (plus camera/effect context) and
//! produce renderer-facing state: lightmap and fog environments, clear/sky
//! colors, cloud frames, weather and lightning render state. No side
//! effects; everything GPU-facing stays in bbb-client-render.

use bbb_client_clock::Instant;
use std::{borrow::Cow, sync::Arc};

mod environment;
mod screens;
pub(crate) use environment::*;
pub(crate) use screens::*;

use bbb_client_render::{
    BlockDestroyOverlay, BlockSelectionRenderState, DebugFilledBox, DebugFilledRect, DebugLine,
    DebugTextBillboard, DisplayTextSurface, EntityFlameSurface, EntityModelInstance,
    EntityNameTagSurface, EntityShadowSurface, FirstPersonMapBackgroundSurface,
    FirstPersonMapBackgroundTexture, FirstPersonPlayerArm, HudActionBarText, HudAirSupply,
    HudBlockItemModel, HudBookScreen, HudBossBar, HudBossBarColor, HudBossBarOverlay,
    HudChatContext, HudChatInteraction, HudChatMessage, HudChatRestrictionsScreen,
    HudCommandBlockEditorScreen, HudCommandSuggestions, HudComponentTextRun,
    HudCrosshairAttackIndicator, HudDeathLinkConfirmationScreen, HudDeathMainScreen,
    HudDeathQuitConfirmationScreen, HudDeathScreen, HudDebugOptionsScreen, HudDebugOverlay,
    HudDisconnectedScreen, HudFoodEffect, HudGameEventScreen, HudGameRulesScreen,
    HudHotbarAttackIndicator, HudHotbarMode, HudInventoryScreen, HudJigsawBlockEditorScreen,
    HudMapPreviewSurface, HudMobEffect, HudNowPlayingToast, HudOffhandSlot, HudPauseScreen,
    HudPlayerHealth, HudPlayerList, HudPlayerListEntry, HudPlayerListScore, HudPortalOverlay,
    HudScoreboardSidebar, HudScoreboardSidebarEntry, HudSignEditorScreen, HudSleepOverlay,
    HudSocialInteractionsFocus, HudSocialInteractionsLinkConfirmationScreen,
    HudSocialInteractionsMainScreen, HudSocialInteractionsPage, HudSocialInteractionsPlayer,
    HudSocialInteractionsPlayerAction, HudSocialInteractionsScreen, HudSpectatorGui,
    HudSpectatorGuiSlot, HudSpectatorIcon, HudSpyglassScope, HudStatsScreen,
    HudStructureBlockEditorScreen, HudStyledTextRun, HudTestBlockEditorScreen,
    HudTestInstanceBlockEditorScreen, HudTextStyle, HudTitleText, HudToast, HudVehicleHealth,
    HudVignette, ItemEntityBillboard, ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture,
    ItemFrameMapSurface, ItemFrameMapTextSurface, ItemFrameMapTexture, ItemModelResidencyFrame,
    Renderer, RendererCameraFrame, SelectionOutline, SignTextOutlineContext, SignTextSurface,
    WorldBorderRenderState,
};

use crate::component_text::{
    localized_component_runs, visit_localized_template, LocalizedTemplatePart,
};

use super::*;

/// Projects the world's action-bar overlay state into the renderer's frame
/// vocabulary, mirroring `Gui.extractOverlayMessage`'s inputs:
/// present only while `overlayMessageTime > 0`. The world keeps the exact
/// Component and native resolves its localized runs against the selected
/// resource generation. The fade alpha itself is resolved by the renderer from
/// `remaining_ticks` and `partial_tick`.
pub(super) fn hud_action_bar_runs_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<Arc<[HudComponentTextRun]>> {
    let action_bar = world.action_bar()?;
    if action_bar.display_ticks <= 0 {
        return None;
    }
    Some(hud_component_text_runs(&action_bar.content, item_runtime).into())
}

#[cfg(test)]
pub(crate) fn hud_action_bar_text_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
) -> Option<HudActionBarText> {
    let action_bar = world.action_bar()?;
    if action_bar.display_ticks <= 0 {
        return None;
    }
    Some(HudActionBarText {
        runs: hud_component_text_runs(&action_bar.content, item_runtime),
        remaining_ticks: action_bar.display_ticks,
        partial_tick,
        animate_color: action_bar.animate_color,
    })
}

/// Projects Vanilla's integer `Gui.extractSleepOverlay` alpha from the
/// canonical local `Player.sleepCounter`. The GUI does not interpolate this
/// value with the frame partial tick.
pub(crate) fn hud_sleep_overlay_from_world(world: &WorldStore) -> Option<HudSleepOverlay> {
    hud_sleep_overlay_from_counter(world.local_player_sleep_counter())
}

pub(crate) fn hud_sleep_overlay_from_counter(counter: i32) -> Option<HudSleepOverlay> {
    if counter <= 0 {
        return None;
    }
    let sleep_timer = counter as f32;
    let mut amount = sleep_timer / 100.0;
    if amount > 1.0 {
        amount = 1.0 - (sleep_timer - 100.0) / 10.0;
    }
    let alpha = (220.0 * amount) as u8;
    (alpha > 0).then_some(HudSleepOverlay { alpha })
}

pub(super) type HudTitleTextRuns = (Arc<[HudComponentTextRun]>, Arc<[HudComponentTextRun]>);

pub(super) fn hud_title_runs_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudTitleTextRuns> {
    let title = world.title();
    let title_text = title.title.as_ref()?;
    if title.title_time <= 0 {
        return None;
    }
    let title_runs = hud_component_text_runs(title_text, item_runtime).into();
    let subtitle_runs = title
        .subtitle
        .as_ref()
        .map(|subtitle| hud_component_text_runs(subtitle, item_runtime).into())
        .unwrap_or_else(|| Arc::from([]));
    Some((title_runs, subtitle_runs))
}

/// Projects the world's title/subtitle overlay state, mirroring
/// `Gui.extractTitle`'s inputs: present only while a title is
/// set and `titleTime > 0`. The subtitle rides along only while a title is
/// active (vanilla draws it inside the title branch); fade windows pass
/// through so the renderer computes the vanilla alpha ramp per frame.
#[cfg(test)]
pub(crate) fn hud_title_text_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
) -> Option<HudTitleText> {
    let title = world.title();
    let title_text = title.title.as_ref()?;
    if title.title_time <= 0 {
        return None;
    }
    Some(HudTitleText {
        title_runs: hud_component_text_runs(title_text, item_runtime),
        subtitle_runs: title
            .subtitle
            .as_ref()
            .map(|subtitle| hud_component_text_runs(subtitle, item_runtime))
            .unwrap_or_default(),
        remaining_ticks: title.title_time,
        fade_in: title.fade_in,
        stay: title.stay,
        fade_out: title.fade_out,
        partial_tick,
    })
}

fn hud_component_text_runs(
    component: &bbb_protocol::RichTextComponent,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudComponentTextRun> {
    // `Font.width(FormattedText)` and `graphics.text(font, Component, …)` both
    // read the visual order, so the width the draw centers on and the glyphs it
    // draws come from this one resolved list.
    crate::component_text::legacy_resolved_runs(&localized_component_runs(
        component,
        &bbb_protocol::ComponentStyle::default(),
        item_runtime,
    ))
    .into_iter()
    .map(|run| HudComponentTextRun {
        visual: hud_chat_run(&run.text, &run.style),
        font: run.style.font,
    })
    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudChatEntryContextKey {
    level_generation: u64,
    display_refresh_revision: u64,
    resource_generation: u64,
    resources_loaded: bool,
}

#[derive(Debug, Clone)]
struct CachedChatMessageEntry {
    projected_runs: Box<[ProjectedChatRun]>,
    tag: Option<HudChatTag>,
    cached_message: Option<HudChatMessage>,
}

#[derive(Debug, Default)]
pub(super) struct HudChatMessageEntryCache {
    context_key: Option<HudChatEntryContextKey>,
    entries: bbb_client_world::FxHashMap<u64, CachedChatMessageEntry>,
    #[cfg(test)]
    pub(super) formatted_messages: u64,
}

#[cfg(test)]
pub(crate) fn hud_chat_messages_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudChatMessage> {
    world
        .client_chat()
        .messages
        .iter()
        .map(|message| hud_chat_message(world, message, item_runtime))
        .collect()
}

pub(super) fn hud_chat_messages_from_world_cached(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cache: &mut HudChatMessageEntryCache,
) -> Vec<HudChatMessage> {
    let context_key = HudChatEntryContextKey {
        level_generation: world.client_level_generation(),
        display_refresh_revision: world.client_chat().display_refresh_revision,
        resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
        resources_loaded: item_runtime.is_some(),
    };
    if cache.context_key != Some(context_key) {
        cache.context_key = Some(context_key);
        cache.entries.clear();
    }
    let messages = &world.client_chat().messages;
    if messages.is_empty() {
        cache.entries.clear();
        return Vec::new();
    }
    let mut out = Vec::with_capacity(messages.len());
    for message in messages {
        if let Some(entry) = cache.entries.get(&message.display_sequence) {
            if let Some(cached) = &entry.cached_message {
                out.push(cached.clone());
            } else {
                out.push(hud_chat_message_from_projected_runs(
                    world,
                    item_runtime,
                    message.display_sequence,
                    message.added_tick,
                    &entry.projected_runs,
                    entry.tag.clone(),
                ));
            }
            continue;
        }
        #[cfg(test)]
        {
            cache.formatted_messages = cache.formatted_messages.saturating_add(1);
        }
        let projected_runs: Box<[ProjectedChatRun]> =
            hud_chat_message_runs(message, item_runtime).into_boxed_slice();
        let has_show_item_hover = projected_runs.iter().any(|run| {
            matches!(
                run.hover_event,
                Some(bbb_protocol::ComponentHoverEvent::ShowItem { .. })
            )
        });
        let tag = message
            .tag
            .as_ref()
            .map(|tag| hud_chat_tag(tag, item_runtime));
        let built = hud_chat_message_from_projected_runs(
            world,
            item_runtime,
            message.display_sequence,
            message.added_tick,
            &projected_runs,
            tag.clone(),
        );
        let cached_message = (!has_show_item_hover).then(|| built.clone());
        cache.entries.insert(
            message.display_sequence,
            CachedChatMessageEntry {
                projected_runs,
                tag,
                cached_message,
            },
        );
        out.push(built);
    }
    if cache.entries.len() > messages.len() {
        let oldest_sequence = messages[0].display_sequence;
        cache
            .entries
            .retain(|&sequence, _| sequence >= oldest_sequence);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectedChatRun {
    visual: HudComponentTextRun,
    click_event: Option<bbb_protocol::ComponentClickEvent>,
    hover_event: Option<bbb_protocol::ComponentHoverEvent>,
    insertion: Option<bbb_protocol::JavaString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HudChatComponentInteraction {
    pub(crate) click_event: Option<bbb_protocol::ComponentClickEvent>,
    pub(crate) insertion: Option<bbb_protocol::JavaString>,
}

#[cfg(test)]
fn hud_chat_message(
    world: &WorldStore,
    message: &bbb_client_world::ChatMessageState,
    item_runtime: Option<&NativeItemRuntime>,
) -> HudChatMessage {
    let projected = hud_chat_message_runs(message, item_runtime);
    let tag = message
        .tag
        .as_ref()
        .map(|tag| hud_chat_tag(tag, item_runtime));
    hud_chat_message_from_projected_runs(
        world,
        item_runtime,
        message.display_sequence,
        message.added_tick,
        &projected,
        tag,
    )
}

fn hud_chat_message_from_projected_runs(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    display_sequence: u64,
    added_tick: u64,
    projected: &[ProjectedChatRun],
    tag: Option<HudChatTag>,
) -> HudChatMessage {
    let mut char_offset = 0usize;
    let mut interactions = Vec::new();
    let mut runs = Vec::with_capacity(projected.len());
    for (target_index, run) in projected.iter().enumerate() {
        let start_char = char_offset;
        char_offset = char_offset.saturating_add(run.visual.visual.text.chars().count());
        if run.click_event.is_some() || run.hover_event.is_some() || run.insertion.is_some() {
            interactions.push(HudChatInteraction {
                start_char,
                end_char: char_offset,
                target_index,
                clickable: run.click_event.is_some(),
                insertion: run.insertion.is_some(),
                tooltip: super::component_hover::hud_component_hover_tooltip(
                    world,
                    item_runtime,
                    run.hover_event.as_ref(),
                ),
            });
        }
        runs.push(run.visual.clone());
    }
    HudChatMessage {
        display_sequence,
        added_tick,
        runs,
        interactions,
        tag,
    }
}

fn hud_chat_message_runs(
    message: &bbb_client_world::ChatMessageState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<ProjectedChatRun> {
    if let Some(decoration) = &message.decoration {
        let template = localized_raw_chat_text(item_runtime, &decoration.translation_key);
        let parent = &decoration.style;
        let arguments = decoration
            .arguments
            .iter()
            .map(|argument| projected_chat_argument(argument, parent, item_runtime))
            .collect::<Vec<_>>();
        return format_localized_projected_chat_runs(&template, parent, &arguments);
    }

    if let Some(component) = message.content_component.clone().and_then(|component| {
        bbb_protocol::RichTextComponent::try_from_component_nbt(component).ok()
    }) {
        return localized_component_runs(
            &component,
            &bbb_protocol::ComponentStyle::default(),
            item_runtime,
        )
        .into_iter()
        .map(|run| projected_chat_run(&run.text, &run.style))
        .collect();
    }

    if let Some(key) = message.content_translation_key.as_deref() {
        let text = localized_chat_text(
            item_runtime,
            key,
            chat_translation_fallback(key).unwrap_or(key),
        );
        let style = message
            .styled_content
            .first()
            .map(|run| run.style.clone())
            .unwrap_or_default();
        return vec![projected_chat_run(text, &style)];
    }
    if message.styled_content.is_empty() {
        vec![ProjectedChatRun {
            visual: HudComponentTextRun::plain(message.content.clone()),
            click_event: None,
            hover_event: None,
            insertion: None,
        }]
    } else {
        message
            .styled_content
            .iter()
            .map(|run| projected_chat_run(&run.text, &run.style))
            .collect()
    }
}

fn projected_chat_argument(
    argument: &bbb_client_world::ChatDecorationArgumentState,
    parent: &bbb_protocol::ComponentStyle,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<ProjectedChatRun> {
    if let Some(component) = argument
        .component
        .clone()
        .and_then(|value| bbb_protocol::RichTextComponent::try_from_component_nbt(value).ok())
    {
        return localized_component_runs(&component, parent, item_runtime)
            .into_iter()
            .map(|run| projected_chat_run(&run.text, &run.style))
            .collect();
    }
    if argument.styled.is_empty() {
        vec![projected_chat_run(&argument.summary, parent)]
    } else {
        argument
            .styled
            .iter()
            .map(|run| projected_chat_run(&run.text, &run.style.apply_to(parent)))
            .collect()
    }
}

fn projected_chat_run(text: &str, style: &bbb_protocol::ComponentStyle) -> ProjectedChatRun {
    let mut visual = HudComponentTextRun {
        visual: hud_chat_run(text, style),
        font: style.font.clone(),
    };
    visual
        .visual
        .text
        .retain(|ch| ch == '\n' || !ch.is_control());
    ProjectedChatRun {
        visual,
        click_event: style.click_event.clone(),
        hover_event: style.hover_event.clone(),
        insertion: style.insertion.clone(),
    }
}

pub(crate) fn hud_chat_component_interaction(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    display_sequence: u64,
    target_index: usize,
) -> Option<HudChatComponentInteraction> {
    let message = world
        .client_chat()
        .messages
        .iter()
        .find(|message| message.display_sequence == display_sequence)?;
    let run = hud_chat_message_runs(message, item_runtime)
        .into_iter()
        .nth(target_index)?;
    Some(HudChatComponentInteraction {
        click_event: run.click_event,
        insertion: run.insertion,
    })
}

pub(crate) fn chat_narration_text(
    narration: &bbb_client_world::ChatNarrationRequest,
    item_runtime: Option<&NativeItemRuntime>,
) -> String {
    if let Some(component) = narration.content_component.clone().and_then(|component| {
        bbb_protocol::RichTextComponent::try_from_component_nbt(component).ok()
    }) {
        return localized_component_runs(
            &component,
            &bbb_protocol::ComponentStyle::default(),
            item_runtime,
        )
        .into_iter()
        .map(|run| run.text)
        .collect();
    }
    if let Some(key) = narration.translation_key.as_ref() {
        let template = localized_raw_chat_text(item_runtime, key);
        let arguments = narration
            .arguments
            .iter()
            .map(|argument| {
                projected_chat_argument(
                    argument,
                    &bbb_protocol::ComponentStyle::default(),
                    item_runtime,
                )
            })
            .collect::<Vec<_>>();
        return format_localized_projected_chat_runs(
            &template,
            &bbb_protocol::ComponentStyle::default(),
            &arguments,
        )
        .into_iter()
        .map(|run| run.visual.visual.text)
        .collect();
    }
    if narration.styled_content.is_empty() {
        narration.content.clone()
    } else {
        narration
            .styled_content
            .iter()
            .map(|run| run.text.as_str())
            .collect()
    }
}

const MAX_PROJECTED_CHAT_RUNS: usize = 4_096;
const MAX_PROJECTED_CHAT_UTF8_BYTES: usize = 256 * 1024;

fn format_localized_projected_chat_runs(
    template: &str,
    style: &bbb_protocol::ComponentStyle,
    arguments: &[Vec<ProjectedChatRun>],
) -> Vec<ProjectedChatRun> {
    let mut runs = Vec::new();
    let mut remaining_bytes = MAX_PROJECTED_CHAT_UTF8_BYTES;
    visit_localized_template(template, arguments.len(), |part| {
        if runs.len() >= MAX_PROJECTED_CHAT_RUNS || remaining_bytes == 0 {
            return;
        }
        match part {
            LocalizedTemplatePart::Literal(text) => {
                push_projected_chat_literal_bounded(&mut runs, &mut remaining_bytes, text, style);
            }
            LocalizedTemplatePart::Argument { index, raw } => {
                if let Some(argument) = arguments.get(index) {
                    for run in argument {
                        if runs.len() >= MAX_PROJECTED_CHAT_RUNS || remaining_bytes == 0 {
                            break;
                        }
                        let len = run.visual.visual.text.len();
                        if len <= remaining_bytes {
                            remaining_bytes -= len;
                            runs.push(run.clone());
                        } else {
                            let prefix =
                                truncate_utf8_prefix(&run.visual.visual.text, remaining_bytes);
                            if !prefix.is_empty() {
                                let mut truncated = run.clone();
                                truncated.visual.visual.text = prefix.to_string();
                                runs.push(truncated);
                            }
                            remaining_bytes = 0;
                            break;
                        }
                    }
                } else {
                    push_projected_chat_literal_bounded(
                        &mut runs,
                        &mut remaining_bytes,
                        raw,
                        style,
                    );
                }
            }
        }
    });
    if runs.is_empty() {
        let prefix = truncate_utf8_prefix(template, MAX_PROJECTED_CHAT_UTF8_BYTES);
        vec![projected_chat_run(prefix, style)]
    } else {
        runs
    }
}

fn push_projected_chat_literal_bounded(
    runs: &mut Vec<ProjectedChatRun>,
    remaining_bytes: &mut usize,
    text: &str,
    style: &bbb_protocol::ComponentStyle,
) {
    if text.is_empty() || runs.len() >= MAX_PROJECTED_CHAT_RUNS || *remaining_bytes == 0 {
        return;
    }
    if text.len() <= *remaining_bytes {
        *remaining_bytes -= text.len();
        runs.push(projected_chat_run(text, style));
    } else {
        let prefix = truncate_utf8_prefix(text, *remaining_bytes);
        if !prefix.is_empty() {
            runs.push(projected_chat_run(prefix, style));
        }
        *remaining_bytes = 0;
    }
}

fn truncate_utf8_prefix(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn hud_chat_run(text: &str, style: &bbb_protocol::ComponentStyle) -> HudStyledTextRun {
    HudStyledTextRun {
        shadow_color: style.shadow_color,
        text: text.to_string(),
        style: HudTextStyle {
            bold: style.bold == Some(true),
            italic: style.italic == Some(true),
            underlined: style.underlined == Some(true),
            strikethrough: style.strikethrough == Some(true),
            obfuscated: style.obfuscated == Some(true),
        },
        color: style.color,
    }
}

fn hud_chat_tag(
    tag: &bbb_client_world::ChatMessageTagState,
    item_runtime: Option<&NativeItemRuntime>,
) -> HudChatTag {
    use bbb_client_world::ChatMessageTagState;
    let (kind, indicator_color) = match tag {
        ChatMessageTagState::System => (HudChatTagKind::System, 13_684_944),
        ChatMessageTagState::SystemSinglePlayer => (HudChatTagKind::SystemSinglePlayer, 13_684_944),
        ChatMessageTagState::NotSecure => (HudChatTagKind::NotSecure, 13_684_944),
        ChatMessageTagState::Modified { .. } => (HudChatTagKind::Modified, 6_316_128),
        ChatMessageTagState::Error => (HudChatTagKind::Error, 16_733_525),
    };
    let (translation_key, fallback) = match tag {
        ChatMessageTagState::System => ("chat.tag.system", "Server message. Cannot be reported."),
        ChatMessageTagState::SystemSinglePlayer => {
            ("chat.tag.system_single_player", "Server message.")
        }
        ChatMessageTagState::NotSecure => (
            "chat.tag.not_secure",
            "Unverified message. Cannot be reported.",
        ),
        ChatMessageTagState::Modified { .. } => (
            "chat.tag.modified",
            "Message modified by the server. Original:",
        ),
        ChatMessageTagState::Error => ("chat.tag.error", "Server sent invalid message."),
    };
    let mut tooltip_runs = vec![HudStyledTextRun::plain(localized_chat_text(
        item_runtime,
        translation_key,
        fallback,
    ))];
    if let ChatMessageTagState::Modified { original_content } = tag {
        tooltip_runs.push(HudStyledTextRun {
            shadow_color: None,
            text: format!("\n{original_content}"),
            style: HudTextStyle::default(),
            color: Some(0xAA_AA_AA),
        });
    }
    HudChatTag {
        kind,
        indicator_color,
        tooltip_runs,
    }
}

fn localized_raw_chat_text<'a>(
    item_runtime: Option<&'a NativeItemRuntime>,
    key: &bbb_protocol::JavaString,
) -> Cow<'a, str> {
    if let Some(runtime) = item_runtime {
        if let Some(template) = key
            .with_utf8(|key| runtime.localized_text_if_present(key))
            .flatten()
        {
            return Cow::Borrowed(template);
        }
        if let Some(template) = runtime.localized_text_raw_if_present(key) {
            return Cow::Owned(template.to_display_string());
        }
    }
    key.with_utf8(chat_translation_fallback)
        .flatten()
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(key.to_display_string()))
}

fn localized_chat_text<'a>(
    item_runtime: Option<&'a NativeItemRuntime>,
    key: &'a str,
    fallback: &'a str,
) -> &'a str {
    item_runtime
        .and_then(|runtime| runtime.localized_text_if_present(key))
        .unwrap_or(fallback)
}

fn chat_translation_fallback(key: &str) -> Option<&'static str> {
    match key {
        "chat.type.text" => Some("<%s> %s"),
        "chat.type.announcement" => Some("[%s] %s"),
        "chat.type.emote" => Some("* %s %s"),
        "chat.deleted_marker" => Some("This chat message has been deleted by the server."),
        "chat.queue" => Some("%s messages waiting"),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn format_localized_chat_runs(
    template: &str,
    style: &bbb_protocol::ComponentStyle,
    arguments: &[Vec<HudStyledTextRun>],
) -> Vec<HudStyledTextRun> {
    let mut runs = Vec::new();
    visit_localized_template(template, arguments.len(), |part| match part {
        LocalizedTemplatePart::Literal(text) => push_hud_chat_literal(&mut runs, text, style),
        LocalizedTemplatePart::Argument { index, raw } => {
            if let Some(argument) = arguments.get(index) {
                runs.extend(argument.iter().cloned());
            } else {
                push_hud_chat_literal(&mut runs, raw, style);
            }
        }
    });
    if runs.is_empty() {
        vec![hud_chat_run(template, style)]
    } else {
        runs
    }
}

#[cfg(test)]
fn push_hud_chat_literal(
    runs: &mut Vec<HudStyledTextRun>,
    text: &str,
    style: &bbb_protocol::ComponentStyle,
) {
    if text.is_empty() {
        return;
    }
    let run = hud_chat_run(text, style);
    if let Some(previous) = runs.last_mut().filter(|previous| {
        previous.style == run.style
            && previous.color == run.color
            && previous.shadow_color == run.shadow_color
    }) {
        previous.text.push_str(text);
    } else {
        runs.push(run);
    }
}

/// Projects the world's tracked boss bars into the renderer's ordered frame
/// list, mirroring `BossHealthOverlay.extractRenderState`'s walk over its
/// event map. The world retains the
/// `LinkedHashMap` packet-add order, including in-place replacement and
/// remove/re-add behavior. Names retain their trusted inherited styled runs;
/// the `HudRenderRuntime` caller replaces the latest target progress with
/// `LerpingBossEvent`'s 100ms wall-clock presentation value. Bars whose stored
/// color/overlay names fall outside the vanilla `BossEvent` getName vocabularies
/// (impossible via the protocol path, reachable only through deserialized
/// state) are dropped. The darken-screen / world-fog flags stay behind the
/// world's `boss_overlay_should_*` queries; they are sky/fog effects, not
/// bar draws.
#[cfg(test)]
pub(crate) fn hud_boss_bars_from_world(world: &WorldStore) -> Vec<HudBossBar> {
    world
        .boss_bars()
        .iter()
        .filter_map(|bar| hud_boss_bar_from_world_state(bar, None, bar.progress))
        .collect()
}

pub(crate) fn hud_boss_bar_from_world_state(
    bar: &bbb_client_world::BossBarState,
    item_runtime: Option<&NativeItemRuntime>,
    progress: f32,
) -> Option<HudBossBar> {
    Some(HudBossBar {
        // `BossHealthOverlay` measures the name with `font.width(Component)`
        // and draws it as a Component, so both
        // the centred x and the glyphs read this resolved list.
        name_runs: hud_component_text_runs(&bar.name, item_runtime)
            .into_iter()
            .map(|run| run.visual)
            .collect(),
        progress,
        color: HudBossBarColor::from_name(&bar.color)?,
        overlay: HudBossBarOverlay::from_name(&bar.overlay)?,
    })
}

pub(crate) fn hud_player_list_from_world(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    tick_count: u64,
    show_heads: bool,
) -> Option<HudPlayerList> {
    if !hud_player_list_is_visible(input) {
        return None;
    }
    let entries = hud_player_list_entries_from_world(world, item_runtime, show_heads);
    let header = world
        .tab_list()
        .header
        .as_ref()
        .map(|component| hud_tab_list_component_runs(component, item_runtime));
    let footer = world
        .tab_list()
        .footer
        .as_ref()
        .map(|component| hud_tab_list_component_runs(component, item_runtime));
    Some(HudPlayerList {
        header,
        footer,
        entries,
        show_heads,
        tick_count,
    })
}

pub(crate) fn hud_player_list_is_visible(input: &ClientInputState) -> bool {
    !input.hide_gui() && input.player_list_key_down() && !input.blocks_world_input()
}

pub(crate) fn hud_player_list_narration_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> String {
    let players = hud_player_list_entries_from_world(world, item_runtime, false)
        .into_iter()
        .map(|entry| {
            entry
                .name_runs
                .into_iter()
                .map(|run| run.text)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(", ");
    localized_gui_text_with_args(
        item_runtime,
        "multiplayer.player.list.narration",
        "Online players: %s",
        &[&players],
    )
}

fn hud_player_list_entries_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    resolve_skins: bool,
) -> Vec<HudPlayerListEntry> {
    let scoreboard = world.scoreboard();
    let mut entries: Vec<_> = world
        .listed_players()
        .iter()
        .filter_map(|id| world.player_info_entry(*id))
        .map(|entry| {
            let team_name = scoreboard
                .team_for_scoreboard_name(&entry.profile.name)
                .map(|team| team.name.as_str())
                .unwrap_or("");
            let lowercase_name = entry.profile.name.to_lowercase();
            (entry, team_name, lowercase_name)
        })
        .collect();
    entries.sort_by(
        |(left, left_team, left_lower), (right, right_team, right_lower)| {
            right
                .list_order
                .cmp(&left.list_order)
                .then_with(|| left.is_spectator().cmp(&right.is_spectator()))
                .then_with(|| left_team.cmp(right_team))
                .then_with(|| left_lower.cmp(right_lower))
        },
    );
    entries
        .into_iter()
        .take(80)
        .map(|(entry, _, _)| {
            let mut name_runs = entry.display_name.as_ref().map_or_else(
                || {
                    hud_scoreboard_text_runs(
                        &scoreboard.team_formatted_literal_name(&entry.profile.name),
                        item_runtime,
                    )
                },
                |display_name| hud_tab_list_component_runs(display_name, item_runtime),
            );
            if entry.is_spectator() {
                for run in &mut name_runs {
                    run.style.italic = true;
                }
            }
            let score =
                scoreboard
                    .player_list_score(&entry.profile.name)
                    .map(|score| match score {
                        bbb_client_world::ScoreboardPlayerListScoreState::Integer(text) => {
                            HudPlayerListScore::Integer(hud_scoreboard_text_runs(
                                &text,
                                item_runtime,
                            ))
                        }
                        bbb_client_world::ScoreboardPlayerListScoreState::Hearts(value) => {
                            let hearts = bbb_protocol::java_float_to_string(value as f32 / 2.0_f32);
                            HudPlayerListScore::Hearts {
                                value,
                                hp_text: localized_gui_text_with_args(
                                    item_runtime,
                                    "multiplayer.player.list.hp",
                                    "%shp",
                                    &[&hearts],
                                ),
                                compact_text: hearts,
                            }
                        }
                    });
            HudPlayerListEntry {
                profile_id: entry.profile.uuid.as_u128(),
                name_runs,
                skin: if resolve_skins {
                    spectator_player_skin(world, &entry.profile, item_runtime)
                } else {
                    bbb_client_render::EntityPlayerSkin::ProfiledDefault(
                        bbb_client_render::item_model::default_player_skin_for_profile_id(
                            entry.profile.uuid.as_u128(),
                        ),
                    )
                },
                show_hat: entry.show_hat,
                upside_down: world.has_loaded_player_entity(entry.profile.uuid)
                    && matches!(entry.profile.name.as_str(), "Dinnerbone" | "Grumm"),
                latency: entry.latency,
                spectator: entry.is_spectator(),
                score,
            }
        })
        .collect()
}

fn hud_tab_list_component_runs(
    component: &bbb_protocol::RichTextComponent,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    hud_component_text_runs(component, item_runtime)
        .into_iter()
        .map(|run| run.visual)
        .collect()
}

pub(crate) fn hud_scoreboard_sidebar_from_world(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudScoreboardSidebar> {
    if input.hide_gui() {
        return None;
    }
    let sidebar = world.scoreboard_sidebar()?;
    Some(HudScoreboardSidebar {
        title_runs: hud_scoreboard_text_runs(&sidebar.title, item_runtime),
        entries: sidebar
            .entries
            .iter()
            .map(|entry| HudScoreboardSidebarEntry {
                name_runs: hud_scoreboard_text_runs(&entry.name, item_runtime),
                score_runs: hud_scoreboard_text_runs(&entry.score, item_runtime),
            })
            .collect(),
    })
}

pub(crate) fn hud_hotbar_mode_from_world(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    now: Instant,
) -> HudHotbarMode {
    if input.hide_gui() {
        return HudHotbarMode::Hidden;
    }
    if !world.local_player_is_spectator() {
        // Vanilla `Gui.extractItemHotbar` draws nothing when
        // `getCameraPlayer()` is null, which for a live session means the
        // camera was retargeted at a non-player entity.
        let camera_is_non_player = world
            .camera_entity_id()
            .is_some_and(|entity_id| world.hud_camera_player_id() != Some(entity_id));
        return if camera_is_non_player {
            HudHotbarMode::Hidden
        } else {
            HudHotbarMode::Ordinary
        };
    }
    let Some(page) = input.spectator_menu_page_at(now) else {
        return HudHotbarMode::Spectator(None);
    };
    let prompt_runs = match page.prompt {
        crate::input::spectator_menu::SpectatorMenuPrompt::Root => spectator_localized_runs(
            item_runtime,
            "spectatorMenu.root.prompt",
            "Press a key to select a command, and again to use it.",
        ),
        crate::input::spectator_menu::SpectatorMenuPrompt::Player => spectator_localized_runs(
            item_runtime,
            "spectatorMenu.teleport.prompt",
            "Select a player to teleport to",
        ),
        crate::input::spectator_menu::SpectatorMenuPrompt::Team => spectator_localized_runs(
            item_runtime,
            "spectatorMenu.team_teleport.prompt",
            "Select a team to teleport to",
        ),
    };
    HudHotbarMode::Spectator(Some(HudSpectatorGui {
        alpha: page.alpha,
        selected_slot: u8::try_from(page.selected_slot).ok(),
        prompt_runs,
        slots: page.slots.map(|item| {
            use crate::input::spectator_menu::SpectatorMenuItemView;
            let (icon, enabled, name_runs) = match item {
                SpectatorMenuItemView::Empty => return None,
                SpectatorMenuItemView::PlayersCategory { enabled } => (
                    HudSpectatorIcon::TeleportToPlayer,
                    enabled,
                    spectator_localized_runs(
                        item_runtime,
                        "spectatorMenu.teleport",
                        "Teleport to Player",
                    ),
                ),
                SpectatorMenuItemView::TeamsCategory { enabled } => (
                    HudSpectatorIcon::TeleportToTeam,
                    enabled,
                    spectator_localized_runs(
                        item_runtime,
                        "spectatorMenu.team_teleport",
                        "Teleport to Team Member",
                    ),
                ),
                SpectatorMenuItemView::Player(player) => (
                    HudSpectatorIcon::Player {
                        skin: spectator_player_skin(world, &player.profile, item_runtime),
                    },
                    item.enabled(),
                    vec![HudStyledTextRun::plain(&player.profile.name)],
                ),
                SpectatorMenuItemView::Team(team) => (
                    HudSpectatorIcon::Team {
                        skin: spectator_player_skin(world, &team.icon_player.profile, item_runtime),
                        color_rgb: team.color_rgb,
                    },
                    item.enabled(),
                    hud_scoreboard_text_runs(&team.display_name, item_runtime),
                ),
                SpectatorMenuItemView::PreviousPage => (
                    HudSpectatorIcon::PreviousPage,
                    item.enabled(),
                    spectator_localized_runs(
                        item_runtime,
                        "spectatorMenu.previous_page",
                        "Previous Page",
                    ),
                ),
                SpectatorMenuItemView::NextPage { .. } => (
                    HudSpectatorIcon::NextPage,
                    item.enabled(),
                    spectator_localized_runs(item_runtime, "spectatorMenu.next_page", "Next Page"),
                ),
                SpectatorMenuItemView::Close => (
                    HudSpectatorIcon::Close,
                    item.enabled(),
                    spectator_localized_runs(item_runtime, "spectatorMenu.close", "Close Menu"),
                ),
            };
            Some(HudSpectatorGuiSlot {
                icon,
                enabled,
                name_runs,
            })
        }),
    }))
}

fn spectator_localized_runs(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
) -> Vec<HudStyledTextRun> {
    vec![HudStyledTextRun::plain(localized_gui_text(
        item_runtime,
        translation_key,
        fallback,
    ))]
}

fn spectator_player_skin(
    world: &WorldStore,
    profile: &bbb_client_world::PlayerInfoProfileState,
    item_runtime: Option<&NativeItemRuntime>,
) -> bbb_client_render::EntityPlayerSkin {
    item_runtime.map_or_else(
        || {
            bbb_client_render::EntityPlayerSkin::ProfiledDefault(
                bbb_client_render::item_model::default_player_skin_for_profile_id(
                    profile.uuid.as_u128(),
                ),
            )
        },
        |item_runtime| {
            item_runtime.player_skin_for_player_info(
                profile,
                world.local_player_profile_id() == Some(profile.uuid),
                || {
                    bbb_client_net::verify_profile_texture_signature(
                        &profile.properties,
                        world.services_profile_property_keys(),
                    )
                },
            )
        },
    )
}

fn hud_scoreboard_text_runs(
    text: &bbb_client_world::ScoreboardTextState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    let source = text
        .segments
        .iter()
        .flat_map(|segment| match segment {
            bbb_client_world::ScoreboardTextSegmentState::Rich(component) => {
                localized_component_runs(component, &text.inherited_style, item_runtime)
            }
            bbb_client_world::ScoreboardTextSegmentState::Styled(runs) => runs
                .iter()
                .map(|run| bbb_protocol::StyledTextRun {
                    text: run.text.clone(),
                    style: run.style.apply_to(&text.inherited_style),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    // `Gui.displayScoreboardSidebar` and `PlayerTabOverlay` both measure with
    // `font.width(Component)` and draw with `graphics.text(font, Component, …)`
    // so the sidebar's
    // right-edge layout and the drawn glyphs read this one resolved list.
    crate::component_text::legacy_resolved_runs(&source)
        .iter()
        .map(|run| hud_chat_run(&run.text, &run.style))
        .collect()
}

/// One frame's worth of world->renderer state.
///
/// `pump_network_and_terrain` extracts each field at a vanilla-verified
/// client-tick sequence point, then commits the whole frame to the renderer in
/// one `apply_renderer_frame` call. Sky-flash-dependent environment fields read
/// after the `ClientLevel.tick`-equivalent `advance_sky_flash_time`.
pub(crate) enum RendererFrameUpdate<T> {
    Unchanged,
    Replace(T),
}

/// The projected chat entries; `refreshed` when the world re-displayed every
/// entry since the last projection.
pub(crate) struct HudChatMessagesFrame {
    pub(crate) messages: Vec<HudChatMessage>,
    pub(crate) refreshed: bool,
}

pub(crate) struct RendererInputLatencyFrame {
    pub(crate) sequence: u64,
    pub(crate) enqueued_at: Instant,
    pub(crate) simulation_applied_at: Instant,
    pub(crate) superseded_events: u64,
}

pub(crate) struct HudStaticUiFrame {
    pub(crate) hide_gui: bool,
    pub(crate) hud_sleep_overlay: Option<HudSleepOverlay>,
    pub(crate) hud_food: Option<i32>,
    pub(crate) hud_food_effect: HudFoodEffect,
    pub(crate) hud_mob_effects: Vec<HudMobEffect>,
    pub(crate) hud_inventory_shows_mob_effects: bool,
    pub(crate) hud_armor: Option<i32>,
    pub(crate) hud_air: Option<HudAirSupply>,
    pub(crate) hud_vehicle_health: Option<HudVehicleHealth>,
    pub(crate) hud_experience_level: Option<i32>,
    pub(crate) hud_selected_slot: u8,
    pub(crate) hud_camera_overlays: Vec<HudCameraOverlay>,
    pub(crate) hud_chat_context: HudChatContext,
}

pub(crate) struct HudHotbarItemsFrame {
    pub(crate) hud_hotbar_item_icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS],
    pub(crate) hud_offhand_slot: Option<HudOffhandSlot>,
}

#[derive(Default)]
pub(crate) struct HudScreensFrame {
    pub(crate) hud_inventory_screen: Option<HudInventoryScreen>,
    pub(crate) hud_inventory_bundle_tooltip: Option<bbb_client_render::HudBundleTooltip>,
    pub(crate) hud_book_screen: Option<HudBookScreen>,
    pub(crate) hud_screen_blur: bool,
    pub(crate) hud_screen_present: bool,
    pub(crate) hud_ingame_ui_present: bool,
    pub(crate) hud_sign_editor_screen: Option<HudSignEditorScreen>,
    pub(crate) hud_command_block_editor_screen: Option<HudCommandBlockEditorScreen>,
    pub(crate) hud_structure_block_editor_screen: Option<HudStructureBlockEditorScreen>,
    pub(crate) hud_jigsaw_block_editor_screen: Option<HudJigsawBlockEditorScreen>,
    pub(crate) hud_test_block_editor_screen: Option<HudTestBlockEditorScreen>,
    pub(crate) hud_test_instance_block_editor_screen: Option<HudTestInstanceBlockEditorScreen>,
    pub(crate) hud_pause_screen: Option<HudPauseScreen>,
    pub(crate) hud_game_rules_screen: Option<HudGameRulesScreen>,
    pub(crate) hud_disconnected_screen: Option<HudDisconnectedScreen>,
    pub(crate) hud_level_loading_screen: Option<HudLevelLoadingScreen>,
    pub(crate) hud_death_screen: Option<HudDeathScreen>,
    pub(crate) hud_command_confirmation: Option<bbb_client_render::HudCommandConfirmation>,
    pub(crate) hud_game_event_screen: Option<HudGameEventScreen>,
    pub(crate) hud_social_interactions_screen: Option<HudSocialInteractionsScreen>,
    pub(crate) hud_stats_screen: Option<HudStatsScreen>,
    pub(crate) hud_debug_options_screen: Option<HudDebugOptionsScreen>,
    pub(crate) hud_chat_restrictions_screen: Option<HudChatRestrictionsScreen>,
    pub(crate) hud_command_suggestions: Option<HudCommandSuggestions>,
}

pub(crate) struct RendererFrame<'a> {
    pub(crate) input_latency: Option<RendererInputLatencyFrame>,
    pub(crate) lightmap_environment: LightmapEnvironment,
    pub(crate) clear_color: ClearColor,
    pub(crate) fog_environment: FogEnvironment,
    pub(crate) sky_environment: SkyEnvironment,
    pub(crate) cloud_environment: CloudEnvironment,
    pub(crate) hud_static_ui: RendererFrameUpdate<HudStaticUiFrame>,
    pub(crate) hud_crosshair_visible: RendererFrameUpdate<bool>,
    pub(crate) hud_crosshair_attack_indicator: RendererFrameUpdate<HudCrosshairAttackIndicator>,
    pub(crate) hud_player_health: RendererFrameUpdate<Option<HudPlayerHealth>>,
    pub(crate) hud_vignette: RendererFrameUpdate<Option<HudVignette>>,
    /// Vanilla `ScreenEffectRenderer` overlays in draw order (view-blocking
    /// block, underwater, fire).
    pub(crate) screen_effect_quads: RendererFrameUpdate<Vec<bbb_client_render::ScreenEffectQuad>>,
    pub(crate) hud_spyglass_scope: RendererFrameUpdate<Option<HudSpyglassScope>>,
    pub(crate) hud_portal_overlay: RendererFrameUpdate<Option<HudPortalOverlay>>,
    pub(crate) hud_confusion_overlay: RendererFrameUpdate<Option<HudConfusionOverlay>>,
    pub(crate) hud_contextual_bar: RendererFrameUpdate<HudContextualBar>,
    pub(crate) hud_hotbar_mode: RendererFrameUpdate<HudHotbarMode>,
    pub(crate) hud_hotbar_items: RendererFrameUpdate<Box<HudHotbarItemsFrame>>,
    pub(crate) hud_hotbar_block_item_models: RendererFrameUpdate<Vec<Option<HudBlockItemModel>>>,
    pub(crate) hud_offhand_block_item_model: RendererFrameUpdate<Option<HudBlockItemModel>>,
    pub(crate) hud_hotbar_attack_indicator: RendererFrameUpdate<HudHotbarAttackIndicator>,
    pub(crate) hud_screens: RendererFrameUpdate<Box<HudScreensFrame>>,
    pub(crate) hud_toasts: RendererFrameUpdate<Vec<HudToast>>,
    pub(crate) hud_now_playing_toast: RendererFrameUpdate<Option<HudNowPlayingToast>>,
    pub(crate) hud_action_bar_text: RendererFrameUpdate<Option<HudActionBarText>>,
    pub(crate) hud_selected_item_name:
        RendererFrameUpdate<Option<bbb_client_render::HudSelectedItemName>>,
    pub(crate) hud_title_text: RendererFrameUpdate<Option<HudTitleText>>,
    pub(crate) hud_scoreboard_sidebar: RendererFrameUpdate<Option<HudScoreboardSidebar>>,
    pub(crate) hud_player_list: RendererFrameUpdate<Option<HudPlayerList>>,
    pub(crate) hud_player_list_tick_count: Option<u64>,
    pub(crate) hud_chat_messages: RendererFrameUpdate<HudChatMessagesFrame>,
    pub(crate) hud_debug_overlay: RendererFrameUpdate<Option<HudDebugOverlay>>,
    pub(crate) hud_boss_bars: RendererFrameUpdate<Vec<HudBossBar>>,
    pub(crate) item_entity_billboards: Vec<ItemEntityBillboard>,
    pub(crate) item_model_residency: ItemModelResidencyFrame,
    pub(crate) first_person_player_arms: Vec<FirstPersonPlayerArm>,
    pub(crate) first_person_map_background_textures: Vec<FirstPersonMapBackgroundTexture>,
    pub(crate) first_person_map_background_surfaces: Vec<FirstPersonMapBackgroundSurface>,
    pub(crate) first_person_map_textures: Vec<ItemFrameMapTexture>,
    pub(crate) first_person_map_surfaces: Vec<ItemFrameMapSurface>,
    pub(crate) first_person_map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub(crate) first_person_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub(crate) first_person_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    pub(crate) hud_map_preview_textures: Vec<ItemFrameMapTexture>,
    pub(crate) hud_map_preview_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapSurface>>,
    pub(crate) hud_map_preview_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub(crate) hud_map_preview_decoration_surfaces:
        Vec<HudMapPreviewSurface<ItemFrameMapDecorationSurface>>,
    pub(crate) hud_map_preview_text_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapTextSurface>>,
    pub(crate) item_frame_map_textures: Vec<ItemFrameMapTexture>,
    pub(crate) item_frame_map_surfaces: Vec<ItemFrameMapSurface>,
    pub(crate) item_frame_map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub(crate) item_frame_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub(crate) item_frame_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    pub(crate) sign_text_surfaces: RendererFrameUpdate<Vec<SignTextSurface>>,
    pub(crate) visible_sign_text_positions: Vec<[i32; 3]>,
    pub(crate) sign_text_outline_context: SignTextOutlineContext,
    pub(crate) display_text_surfaces: Vec<DisplayTextSurface>,
    pub(crate) entity_name_tag_surfaces: Vec<EntityNameTagSurface>,
    pub(crate) entity_flame_surfaces: Vec<EntityFlameSurface>,
    pub(crate) entity_shadow_surfaces: Vec<EntityShadowSurface>,
    pub(crate) entity_model_instances: Vec<EntityModelInstance>,
    pub(crate) terrain_sections_layout_revision: u64,
    pub(crate) terrain_sections_revision: u64,
    pub(crate) terrain_sections: &'a [bbb_client_render::TerrainSectionRenderState],
    pub(crate) terrain_fading_sections: &'a [(bbb_client_render::TerrainSectionMeshOwner, f32)],
    pub(crate) camera_pose: Option<CameraPose>,
    pub(crate) camera_hurt_effect: Option<bbb_client_render::CameraHurtEffect>,
    pub(crate) camera_view_bob_effect: Option<bbb_client_render::CameraViewBobEffect>,
    pub(crate) camera_spinning_effect: Option<bbb_client_render::CameraSpinningEffect>,
    pub(crate) camera_fov_degrees: f32,
    pub(crate) camera_hud_fov_degrees: f32,
    pub(crate) current_post_effect_id: Option<&'a str>,
    pub(crate) camera_post_effect_active: bool,
    pub(crate) shader_game_time_ticks: f64,
    pub(crate) cloud_frame: CloudFrame,
    pub(crate) weather_render_state: WeatherRenderState,
    pub(crate) world_border_render_state: WorldBorderRenderState,
    pub(crate) block_selection: Option<BlockSelectionRenderState>,
    pub(crate) chunk_border_outline: Option<SelectionOutline>,
    pub(crate) entity_scene_outline: Option<SelectionOutline>,
    pub(crate) debug_filled_boxes: Vec<DebugFilledBox>,
    pub(crate) debug_filled_rects: Vec<DebugFilledRect>,
    pub(crate) debug_text_billboards: Vec<DebugTextBillboard>,
    pub(crate) debug_lines: Vec<DebugLine>,
    pub(crate) game_test_highlight_boxes: RendererFrameUpdate<Vec<DebugFilledBox>>,
    pub(crate) game_test_highlight_text_billboards: RendererFrameUpdate<Vec<DebugTextBillboard>>,
    pub(crate) block_destroy_overlays: RendererFrameUpdate<Vec<BlockDestroyOverlay>>,
}

/// Render-thread-only observations for heap-backed non-entity extraction.
///
/// These values are copied into the next immutable backend-telemetry
/// generation. The scratch itself never crosses the render/simulation
/// boundary and therefore needs neither locking nor atomics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NonEntityRenderExtractionObservation {
    pub(crate) hud_nanos: u64,
    pub(crate) hud_peak_nanos: u64,
    pub(crate) item_nanos: u64,
    pub(crate) item_peak_nanos: u64,
    pub(crate) block_entity_nanos: u64,
    pub(crate) block_entity_peak_nanos: u64,
    pub(crate) world_debug_nanos: u64,
    pub(crate) world_debug_peak_nanos: u64,
    pub(crate) apply_nanos: u64,
    pub(crate) apply_peak_nanos: u64,
    pub(crate) capacity_growths: u64,
    pub(crate) cache_hits: u64,
    pub(crate) cache_misses: u64,
    pub(crate) rebuilt_owners: u64,
}

#[derive(Debug, Default)]
pub(crate) struct NonEntityRenderExtractionScratch {
    observation: NonEntityRenderExtractionObservation,
    christmas_chest_textures: crate::chest_scene::ChristmasChestTextures,
    stable_world: Option<u64>,
    crosshair: Option<(u64, Option<CameraPose>, crate::crosshair::CrosshairPick)>,
    stable_block_entities: Option<CachedStableBlockEntityProjection>,
    dynamic_block_entity_presence: Option<(
        bbb_client_world::StableBlockEntityRenderRevision,
        Option<ChunkPos>,
        Arc<bbb_client_world::BlockEntityRenderPresence>,
    )>,
    game_test_highlight_key: Option<(u64, u64)>,
    debug_buffers: DebugRenderExtractionBuffers,
    item_registry_keys: CachedItemRegistryKeys,
    map_textures: crate::map_textures::MapTextureExtractionCache,
    map_texture_level_generation: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StableBlockEntityCacheKey {
    pub(crate) revision: bbb_client_world::StableBlockEntityRenderRevision,
    pub(crate) text_filtering_enabled: bool,
}

#[derive(Debug)]
struct CachedStableBlockEntityProjection {
    key: StableBlockEntityCacheKey,
    sign_instances: Vec<EntityModelInstance>,
    bed_instances: Vec<EntityModelInstance>,
    lectern_instances: Vec<EntityModelInstance>,
}

pub(crate) struct StableBlockEntityFrame<'a> {
    pub(crate) sign_instances: &'a [EntityModelInstance],
    pub(crate) bed_instances: &'a [EntityModelInstance],
    pub(crate) lectern_instances: &'a [EntityModelInstance],
    pub(crate) sign_text_surfaces: RendererFrameUpdate<Vec<SignTextSurface>>,
}

#[derive(Debug, Default)]
pub(crate) struct DebugRenderExtractionBuffers {
    pub(crate) filled_boxes: Vec<DebugFilledBox>,
    pub(crate) filled_rects: Vec<DebugFilledRect>,
    pub(crate) text_billboards: Vec<DebugTextBillboard>,
    pub(crate) lines: Vec<DebugLine>,
    pub(crate) initial_capacities: [usize; 4],
}

impl DebugRenderExtractionBuffers {
    fn record_initial_capacities(&mut self) {
        self.initial_capacities = [
            self.filled_boxes.capacity(),
            self.filled_rects.capacity(),
            self.text_billboards.capacity(),
            self.lines.capacity(),
        ];
    }

    fn capacity_growths(&self) -> u64 {
        [
            self.filled_boxes.capacity(),
            self.filled_rects.capacity(),
            self.text_billboards.capacity(),
            self.lines.capacity(),
        ]
        .into_iter()
        .zip(self.initial_capacities)
        .filter(|(current, initial)| current > initial)
        .count() as u64
    }
}

#[derive(Debug, Default)]
struct CachedItemRegistryKeys {
    revision: Option<bbb_client_world::RegistryAccessRevision>,
    keys: ItemRegistryKeySet,
}

impl NonEntityRenderExtractionScratch {
    pub(crate) fn crosshair_pick(
        &mut self,
        world: &WorldStore,
        world_revision: u64,
        camera_entity_pose: Option<CameraPose>,
    ) -> crate::crosshair::CrosshairPick {
        if let Some((revision, pose, pick)) = self.crosshair {
            if revision == world_revision && pose == camera_entity_pose {
                return pick;
            }
        }
        let pick = crosshair_pick_from_camera(world, camera_entity_pose);
        self.crosshair = Some((world_revision, camera_entity_pose, pick));
        pick
    }

    pub(crate) fn observation(&self) -> NonEntityRenderExtractionObservation {
        self.observation
    }

    pub(crate) fn christmas_chest_textures(&self) -> crate::chest_scene::ChristmasChestTextures {
        self.christmas_chest_textures
    }

    /// Returns whether the world revision was already applied to the
    /// renderer.
    pub(crate) fn begin_stable_world_frame(&mut self, world_revision: u64) -> bool {
        let unchanged = self.stable_world == Some(world_revision);
        // Eye overlays sample the player at simulation time. Rotation and F5 arrive
        // through input commands that invalidate the same revision.
        if unchanged {
            self.observation.cache_hits = self.observation.cache_hits.saturating_add(4);
        } else {
            self.stable_world = Some(world_revision);
            self.observation.cache_misses = self.observation.cache_misses.saturating_add(4);
            self.observation.rebuilt_owners = self.observation.rebuilt_owners.saturating_add(4);
        }
        unchanged
    }

    pub(crate) fn invalidate_resources(&mut self) {
        self.stable_world = None;
        self.crosshair = None;
        self.stable_block_entities = None;
        self.dynamic_block_entity_presence = None;
        self.map_textures.clear();
    }

    pub(crate) fn dynamic_block_entity_presence(
        &mut self,
        world: &WorldStore,
        revision: bbb_client_world::StableBlockEntityRenderRevision,
        camera: Option<CameraPose>,
    ) -> Arc<bbb_client_world::BlockEntityRenderPresence> {
        let camera_column = camera.and_then(|pose| {
            let y = (pose.position[1] + pose.eye_height).floor() as i32;
            world.dimension().contains_y(y).then_some(ChunkPos {
                x: (pose.position[0].floor() as i32).div_euclid(16),
                z: (pose.position[2].floor() as i32).div_euclid(16),
            })
        });
        if let Some((cached_revision, cached_column, presence)) =
            &self.dynamic_block_entity_presence
        {
            if *cached_revision == revision && *cached_column == camera_column {
                self.observation.cache_hits = self.observation.cache_hits.saturating_add(1);
                return Arc::clone(presence);
            }
        }
        let columns = camera_column.map(|center| {
            let radius = (crate::block_entity_visibility::DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE / 16.0)
                .ceil() as i32;
            (
                ChunkPos {
                    x: center.x - radius,
                    z: center.z - radius,
                },
                ChunkPos {
                    x: center.x + radius,
                    z: center.z + radius,
                },
            )
        });
        let presence = Arc::new(world.block_entity_render_presence(columns));
        self.dynamic_block_entity_presence = Some((revision, camera_column, Arc::clone(&presence)));
        self.observation.cache_misses = self.observation.cache_misses.saturating_add(1);
        self.observation.rebuilt_owners = self.observation.rebuilt_owners.saturating_add(1);
        presence
    }

    pub(crate) fn stable_block_entity_frame(
        &mut self,
        key: StableBlockEntityCacheKey,
        build: impl FnOnce() -> (
            Vec<EntityModelInstance>,
            Vec<SignTextSurface>,
            Vec<EntityModelInstance>,
            Vec<EntityModelInstance>,
        ),
    ) -> StableBlockEntityFrame<'_> {
        let sign_text_surfaces = if self
            .stable_block_entities
            .as_ref()
            .is_some_and(|cached| cached.key == key)
        {
            self.observation.cache_hits = self.observation.cache_hits.saturating_add(4);
            RendererFrameUpdate::Unchanged
        } else {
            let (sign_instances, sign_text_surfaces, bed_instances, lectern_instances) = build();
            self.stable_block_entities = Some(CachedStableBlockEntityProjection {
                key,
                sign_instances,
                bed_instances,
                lectern_instances,
            });
            self.observation.cache_misses = self.observation.cache_misses.saturating_add(4);
            self.observation.rebuilt_owners = self.observation.rebuilt_owners.saturating_add(4);
            RendererFrameUpdate::Replace(sign_text_surfaces)
        };
        let cached = self
            .stable_block_entities
            .as_ref()
            .expect("stable block-entity projection initialized");
        StableBlockEntityFrame {
            sign_instances: &cached.sign_instances,
            bed_instances: &cached.bed_instances,
            lectern_instances: &cached.lectern_instances,
            sign_text_surfaces,
        }
    }

    pub(crate) fn game_test_highlight_update(
        &mut self,
        level_generation: u64,
        revision: u64,
        build: impl FnOnce() -> crate::game_test_highlights::GameTestHighlightGizmos,
    ) -> (
        RendererFrameUpdate<Vec<DebugFilledBox>>,
        RendererFrameUpdate<Vec<DebugTextBillboard>>,
    ) {
        let key = (level_generation, revision);
        if self.game_test_highlight_key == Some(key) {
            self.observation.cache_hits = self.observation.cache_hits.saturating_add(2);
            return (
                RendererFrameUpdate::Unchanged,
                RendererFrameUpdate::Unchanged,
            );
        }

        let gizmos = build();
        self.game_test_highlight_key = Some(key);
        self.observation.cache_misses = self.observation.cache_misses.saturating_add(2);
        self.observation.rebuilt_owners = self.observation.rebuilt_owners.saturating_add(2);
        (
            RendererFrameUpdate::Replace(gizmos.filled_boxes),
            RendererFrameUpdate::Replace(gizmos.text_billboards),
        )
    }

    pub(crate) fn begin_debug_frame(&mut self) -> DebugRenderExtractionBuffers {
        let mut buffers = std::mem::take(&mut self.debug_buffers);
        buffers.record_initial_capacities();
        buffers
    }

    pub(crate) fn record_debug_capacity_growths(&mut self, buffers: &DebugRenderExtractionBuffers) {
        self.observation.capacity_growths = self
            .observation
            .capacity_growths
            .saturating_add(buffers.capacity_growths());
    }

    pub(crate) fn recycle_debug_buffers(&mut self, buffers: DebugRenderExtractionBuffers) {
        debug_assert!(buffers.filled_boxes.is_empty());
        debug_assert!(buffers.filled_rects.is_empty());
        debug_assert!(buffers.text_billboards.is_empty());
        debug_assert!(buffers.lines.is_empty());
        self.debug_buffers = buffers;
    }

    /// Invalidates render-thread map pixels when the canonical level owner
    /// changes. Map ids are scoped to that owner and must not retain decoded
    /// pixels or GPU slots across disconnect, respawn, or dimension reset.
    pub(crate) fn synchronize_map_texture_level(&mut self, level_generation: u64) -> bool {
        let changed = self
            .map_texture_level_generation
            .replace(level_generation)
            .is_some_and(|previous| previous != level_generation);
        if changed {
            self.map_textures.clear();
        }
        changed
    }

    pub(crate) fn begin_map_texture_frame(
        &mut self,
    ) -> &mut crate::map_textures::MapTextureExtractionCache {
        self.map_textures.begin_frame();
        &mut self.map_textures
    }

    pub(crate) fn item_registry_keys(&mut self, world: &WorldStore) -> ItemRegistryKeySet {
        let revision = world.registry_access_revision();
        if self.item_registry_keys.revision == Some(revision) {
            self.observation.cache_hits = self.observation.cache_hits.saturating_add(4);
            return self.item_registry_keys.keys.clone();
        }

        self.item_registry_keys = CachedItemRegistryKeys {
            revision: Some(revision),
            keys: ItemRegistryKeySet::from_world(world),
        };
        self.observation.cache_misses = self.observation.cache_misses.saturating_add(4);
        self.item_registry_keys.keys.clone()
    }

    pub(crate) fn record_frame(
        &mut self,
        hud: Duration,
        item: Duration,
        block_entity: Duration,
        world_debug: Duration,
        apply: Duration,
    ) {
        let hud_nanos = duration_nanos(hud);
        let item_nanos = duration_nanos(item);
        let block_entity_nanos = duration_nanos(block_entity);
        let world_debug_nanos = duration_nanos(world_debug);
        let apply_nanos = duration_nanos(apply);
        self.observation.hud_nanos = hud_nanos;
        self.observation.hud_peak_nanos = self.observation.hud_peak_nanos.max(hud_nanos);
        self.observation.item_nanos = item_nanos;
        self.observation.item_peak_nanos = self.observation.item_peak_nanos.max(item_nanos);
        self.observation.block_entity_nanos = block_entity_nanos;
        self.observation.block_entity_peak_nanos = self
            .observation
            .block_entity_peak_nanos
            .max(block_entity_nanos);
        self.observation.world_debug_nanos = world_debug_nanos;
        self.observation.world_debug_peak_nanos = self
            .observation
            .world_debug_peak_nanos
            .max(world_debug_nanos);
        self.observation.apply_nanos = apply_nanos;
        self.observation.apply_peak_nanos = self.observation.apply_peak_nanos.max(apply_nanos);
    }
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

/// Commits one extracted frame to the renderer in a single call.
pub(crate) fn apply_renderer_frame(
    renderer: &mut Renderer,
    frame: RendererFrame<'_>,
    entity_render_extraction: &mut crate::entity_scene::EntityRenderExtractionScratch,
) -> DebugRenderExtractionBuffers {
    renderer.set_clear_color(frame.clear_color);
    let deferred_chat_context = if let RendererFrameUpdate::Replace(static_ui) = frame.hud_static_ui
    {
        renderer.set_hide_gui(static_ui.hide_gui);
        renderer.set_hud_sleep_overlay(static_ui.hud_sleep_overlay);
        renderer.set_hud_food(static_ui.hud_food);
        renderer.set_hud_food_effect(static_ui.hud_food_effect);
        renderer.set_hud_mob_effects(
            static_ui.hud_mob_effects,
            static_ui.hud_inventory_shows_mob_effects,
        );
        renderer.set_hud_armor(static_ui.hud_armor);
        renderer.set_hud_air(static_ui.hud_air);
        renderer.set_hud_vehicle_health(static_ui.hud_vehicle_health);
        renderer.set_hud_experience_level(static_ui.hud_experience_level);
        renderer.set_hud_selected_slot(static_ui.hud_selected_slot);
        renderer.set_hud_camera_overlays(static_ui.hud_camera_overlays);
        Some(static_ui.hud_chat_context)
    } else {
        None
    };
    if let RendererFrameUpdate::Replace(visible) = frame.hud_crosshair_visible {
        renderer.set_hud_crosshair_visible(visible);
    }
    if let RendererFrameUpdate::Replace(indicator) = frame.hud_crosshair_attack_indicator {
        renderer.set_hud_crosshair_attack_indicator(indicator);
    }
    if let RendererFrameUpdate::Replace(health) = frame.hud_player_health {
        renderer.set_hud_player_health(health);
    }
    if let RendererFrameUpdate::Replace(vignette) = frame.hud_vignette {
        renderer.set_hud_vignette(vignette);
    }
    if let RendererFrameUpdate::Replace(quads) = frame.screen_effect_quads {
        renderer.set_screen_effect_quads(quads);
    }
    if let RendererFrameUpdate::Replace(scope) = frame.hud_spyglass_scope {
        renderer.set_hud_spyglass_scope(scope);
    }
    if let RendererFrameUpdate::Replace(portal) = frame.hud_portal_overlay {
        renderer.set_hud_portal_overlay(portal);
    }
    if let RendererFrameUpdate::Replace(confusion) = frame.hud_confusion_overlay {
        renderer.set_hud_confusion_overlay(confusion);
    }
    if let RendererFrameUpdate::Replace(bar) = frame.hud_contextual_bar {
        renderer.set_hud_contextual_bar(bar);
    }
    if let RendererFrameUpdate::Replace(mode) = frame.hud_hotbar_mode {
        renderer.set_hud_hotbar_mode(mode);
    }
    if let RendererFrameUpdate::Replace(items) = frame.hud_hotbar_items {
        let items = *items;
        renderer.set_hud_hotbar_item_icons(items.hud_hotbar_item_icons);
        renderer.set_hud_offhand_slot(items.hud_offhand_slot);
    }
    if let RendererFrameUpdate::Replace(model) = frame.hud_offhand_block_item_model {
        renderer.set_hud_offhand_block_item_model(model);
    }
    if let RendererFrameUpdate::Replace(indicator) = frame.hud_hotbar_attack_indicator {
        renderer.set_hud_hotbar_attack_indicator(indicator);
    }
    if let RendererFrameUpdate::Replace(models) = frame.hud_hotbar_block_item_models {
        renderer.set_hud_hotbar_block_item_models(models);
    }
    let deferred_screens_tail = if let RendererFrameUpdate::Replace(screens) = frame.hud_screens {
        let screens = *screens;
        renderer.set_hud_inventory_screen_with_bundle_tooltip(
            screens.hud_inventory_screen,
            screens.hud_inventory_bundle_tooltip,
        );
        renderer.set_hud_book_screen(screens.hud_book_screen);
        renderer.set_hud_screen_blur(screens.hud_screen_blur);
        renderer.set_hud_screen_presence(screens.hud_screen_present, screens.hud_ingame_ui_present);
        renderer.set_hud_sign_editor_screen(screens.hud_sign_editor_screen);
        renderer.set_hud_command_block_editor_screen(screens.hud_command_block_editor_screen);
        renderer.set_hud_structure_block_editor_screen(screens.hud_structure_block_editor_screen);
        renderer.set_hud_jigsaw_block_editor_screen(screens.hud_jigsaw_block_editor_screen);
        renderer.set_hud_test_block_editor_screen(screens.hud_test_block_editor_screen);
        renderer.set_hud_test_instance_block_editor_screen(
            screens.hud_test_instance_block_editor_screen,
        );
        renderer.set_hud_pause_screen(screens.hud_pause_screen);
        renderer.set_hud_game_rules_screen(screens.hud_game_rules_screen);
        renderer.set_hud_disconnected_screen(screens.hud_disconnected_screen);
        renderer.set_hud_level_loading_screen(screens.hud_level_loading_screen);
        renderer.set_hud_death_screen(screens.hud_death_screen);
        renderer.set_hud_command_confirmation(screens.hud_command_confirmation);
        renderer.set_hud_game_event_screen(screens.hud_game_event_screen);
        renderer.set_hud_social_interactions_screen(screens.hud_social_interactions_screen);
        renderer.set_hud_stats_screen(screens.hud_stats_screen);
        renderer.set_hud_debug_options_screen(screens.hud_debug_options_screen);
        Some((
            screens.hud_chat_restrictions_screen,
            screens.hud_command_suggestions,
        ))
    } else {
        None
    };
    if let RendererFrameUpdate::Replace(toasts) = frame.hud_toasts {
        renderer.set_hud_toasts(toasts);
    }
    if let RendererFrameUpdate::Replace(toast) = frame.hud_now_playing_toast {
        renderer.set_hud_now_playing_toast(toast);
    }
    if let RendererFrameUpdate::Replace(action_bar) = frame.hud_action_bar_text {
        renderer.set_hud_action_bar_text(action_bar);
    }
    if let RendererFrameUpdate::Replace(selected_name) = frame.hud_selected_item_name {
        renderer.set_hud_selected_item_name(selected_name);
    }
    if let RendererFrameUpdate::Replace(title) = frame.hud_title_text {
        renderer.set_hud_title_text(title);
    }
    if let RendererFrameUpdate::Replace(sidebar) = frame.hud_scoreboard_sidebar {
        renderer.set_hud_scoreboard_sidebar(sidebar);
    }
    if let RendererFrameUpdate::Replace(chat) = frame.hud_chat_messages {
        if chat.refreshed {
            renderer.refresh_hud_chat_messages(chat.messages);
        } else {
            renderer.set_hud_chat_messages(chat.messages);
        }
    }
    if let Some(chat_context) = deferred_chat_context {
        renderer.set_hud_chat_context(chat_context);
    }
    if let Some((restrictions, suggestions)) = deferred_screens_tail {
        renderer.set_hud_chat_restrictions_screen(restrictions);
        renderer.set_hud_command_suggestions(suggestions);
    }
    match frame.hud_player_list {
        RendererFrameUpdate::Unchanged => {
            if let Some(tick_count) = frame.hud_player_list_tick_count {
                renderer.set_hud_player_list_tick_count(tick_count);
            }
        }
        RendererFrameUpdate::Replace(player_list) => {
            renderer.set_hud_player_list(player_list);
        }
    }
    if let RendererFrameUpdate::Replace(overlay) = frame.hud_debug_overlay {
        renderer.set_hud_debug_overlay(overlay);
    }
    if let RendererFrameUpdate::Replace(bars) = frame.hud_boss_bars {
        renderer.set_hud_boss_bars(bars);
    }
    renderer.set_item_entity_billboards(frame.item_entity_billboards);
    renderer
        .set_item_model_residency_frame(frame.item_model_residency)
        .expect("resident item-model frame must reference available exact owners");
    renderer.set_first_person_player_arms(frame.first_person_player_arms);
    renderer.set_first_person_map_background_surfaces(
        frame.first_person_map_background_textures,
        frame.first_person_map_background_surfaces,
    );
    renderer.set_map_surfaces_with_gui(
        frame.item_frame_map_textures,
        frame.first_person_map_textures,
        frame.hud_map_preview_textures,
        frame.item_frame_map_surfaces,
        frame.first_person_map_surfaces,
        frame.hud_map_preview_surfaces,
    );
    renderer.set_map_decoration_surfaces_with_gui(
        frame.item_frame_map_decoration_textures,
        frame.first_person_map_decoration_textures,
        frame.hud_map_preview_decoration_textures,
        frame.item_frame_map_decoration_surfaces,
        frame.first_person_map_decoration_surfaces,
        frame.hud_map_preview_decoration_surfaces,
    );
    renderer.set_map_text_surfaces_with_gui(
        frame.item_frame_map_text_surfaces,
        frame.first_person_map_text_surfaces,
        frame.hud_map_preview_text_surfaces,
    );
    if let RendererFrameUpdate::Replace(surfaces) = frame.sign_text_surfaces {
        renderer.set_sign_text_surfaces(surfaces);
    }
    renderer.set_visible_sign_text_positions(frame.visible_sign_text_positions);
    renderer.set_sign_text_outline_context(frame.sign_text_outline_context);
    renderer.set_display_text_surfaces(frame.display_text_surfaces);
    renderer.set_entity_name_tag_surfaces(frame.entity_name_tag_surfaces);
    renderer.set_entity_flame_surfaces(frame.entity_flame_surfaces);
    renderer.set_entity_shadow_surfaces(frame.entity_shadow_surfaces);
    renderer.set_terrain_section_states_with_fades(
        frame.terrain_sections_layout_revision,
        frame.terrain_sections_revision,
        frame.terrain_sections,
        frame.terrain_fading_sections,
    );
    let camera = RendererCameraFrame {
        lightmap_environment: frame.lightmap_environment,
        fog_environment: frame.fog_environment,
        sky_environment: frame.sky_environment,
        pose: frame.camera_pose,
        hurt_effect: frame.camera_hurt_effect,
        view_bob_effect: frame.camera_view_bob_effect,
        spinning_effect: frame.camera_spinning_effect,
        fov_degrees: frame.camera_fov_degrees,
        hud_fov_degrees: frame.camera_hud_fov_degrees,
        shader_game_time_ticks: frame.shader_game_time_ticks,
    };
    let recycled_instances = renderer.replace_scene_frame_with_clouds(
        camera,
        frame.entity_model_instances,
        frame.cloud_environment,
        frame.cloud_frame,
    );
    entity_render_extraction.recycle_instances(recycled_instances);
    renderer.set_camera_post_effect(
        frame.current_post_effect_id,
        frame.camera_post_effect_active,
    );
    renderer.set_weather_render_state(frame.weather_render_state);
    renderer.set_world_border_render_state(frame.world_border_render_state);
    renderer.set_block_selection(frame.block_selection);
    renderer.set_chunk_border_outline(frame.chunk_border_outline);
    renderer.set_entity_scene_outline(frame.entity_scene_outline);
    let filled_boxes = renderer.replace_debug_filled_boxes(frame.debug_filled_boxes);
    let filled_rects = renderer.replace_debug_filled_rects(frame.debug_filled_rects);
    let text_billboards = renderer.replace_debug_text_billboards(frame.debug_text_billboards);
    let lines = renderer.replace_debug_lines(frame.debug_lines);
    match (
        frame.game_test_highlight_boxes,
        frame.game_test_highlight_text_billboards,
    ) {
        (RendererFrameUpdate::Unchanged, RendererFrameUpdate::Unchanged) => {}
        (RendererFrameUpdate::Replace(boxes), RendererFrameUpdate::Replace(text_billboards)) => {
            renderer.set_game_test_highlights(boxes, text_billboards)
        }
        _ => unreachable!("GameTest boxes and text must replace atomically"),
    }
    if let RendererFrameUpdate::Replace(overlays) = frame.block_destroy_overlays {
        renderer.set_block_destroy_overlays(overlays);
    }
    if let Some(input_latency) = frame.input_latency {
        renderer.set_input_latency_stamp(bbb_client_render::RendererInputLatencyStamp {
            sequence: input_latency.sequence,
            enqueued_at: input_latency.enqueued_at,
            simulation_applied_at: input_latency.simulation_applied_at,
            renderer_extracted_at: bbb_client_clock::now(),
            superseded_events: input_latency.superseded_events,
        });
    }
    DebugRenderExtractionBuffers {
        filled_boxes,
        filled_rects,
        text_billboards,
        lines,
        initial_capacities: [0; 4],
    }
}

#[cfg(test)]
mod source_part_tests {
    use super::*;

    #[test]
    fn chat_projection_keeps_interaction_after_u16_ordinal_boundary() {
        let mut world = WorldStore::new();
        world.apply_system_chat(bbb_protocol::packets::SystemChat {
            content: Box::new(bbb_protocol::RichTextComponent::plain("seed")),
            overlay: false,
        });
        let mut message = world.client_chat().messages[0].clone();
        message.content_component = None;
        message.styled_content = vec![
            bbb_protocol::StyledTextRun {
                text: "a".into(),
                style: bbb_protocol::ComponentStyle::default(),
            };
            65537
        ];
        message.styled_content[65536].style.insertion = Some("last".into());
        let projected = hud_chat_message(&world, &message, None);
        assert_eq!(projected.interactions.len(), 1);
        assert_eq!(projected.interactions[0].target_index, 65536);
        assert_eq!(projected.interactions[0].start_char, 65536);
        assert_eq!(projected.interactions[0].end_char, 65537);
        assert_eq!(
            hud_chat_message_runs(&message, None)[65536]
                .insertion
                .as_ref(),
            Some(&"last".into())
        );
    }

    #[test]
    fn chat_source_parts_keep_insertion_and_filter_each_template_literal() {
        let raw = bbb_protocol::JavaString::from(vec![0xd800, 66, 0xdc00]);
        let style = bbb_protocol::ComponentStyle {
            insertion: Some(raw.clone()),
            ..Default::default()
        };
        let runs = format_localized_projected_chat_runs(
            "%s§\t%s",
            &bbb_protocol::ComponentStyle::default(),
            &[
                vec![projected_chat_run("A", &style)],
                vec![projected_chat_run("aX", &style)],
            ],
        );
        assert_eq!(
            runs.iter()
                .map(|r| r.visual.visual.text.as_str())
                .collect::<Vec<_>>(),
            ["A", "§", "aX"]
        );
        assert_eq!(runs[0].insertion.as_ref(), Some(&raw));
        assert_eq!(runs[1].insertion, None);
        assert_eq!(runs[2].insertion.as_ref(), Some(&raw));
    }

    #[test]
    fn chat_decoration_argument_prefers_component_parts_and_nested_translation() {
        let argument = bbb_client_world::ChatDecorationArgumentState {
            parameter: bbb_client_world::ChatTypeDecorationParameter::Content,
            summary: "stale".into(),
            styled: Vec::new(),
            component: Some(
                bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
                    "translate": "missing.parts", "fallback": "%s!", "with": [
                        {"text": "§", "extra": ["aX"], "insertion": "source"}
                    ]
                }))
                .unwrap(),
            ),
        };
        let parent = bbb_protocol::ComponentStyle {
            font: Some("test:font".into()),
            ..Default::default()
        };
        let runs = projected_chat_argument(&argument, &parent, None);
        assert_eq!(
            runs.iter()
                .map(|r| r.visual.visual.text.as_str())
                .collect::<Vec<_>>(),
            ["§", "aX", "!"]
        );
        assert!(runs
            .iter()
            .all(|r| r.visual.font.as_deref() == Some("test:font")));
        assert_eq!(runs[1].insertion.as_ref(), Some(&"source".into()));
        assert_eq!(runs[2].insertion, None);
    }

    #[test]
    fn chat_entry_cache_reuses_projected_runs_across_deliveries_and_invalidates_on_refresh() {
        let mut world = WorldStore::new();
        let mut cache = HudChatMessageEntryCache::default();

        world.apply_system_chat(bbb_protocol::packets::SystemChat {
            content: Box::new(bbb_protocol::RichTextComponent::plain("first")),
            overlay: false,
        });
        let first_cached = hud_chat_messages_from_world_cached(&world, None, &mut cache);
        assert_eq!(first_cached, hud_chat_messages_from_world(&world, None));
        assert_eq!(cache.formatted_messages, 1);

        world.apply_system_chat(bbb_protocol::packets::SystemChat {
            content: Box::new(bbb_protocol::RichTextComponent::plain("second")),
            overlay: false,
        });
        let second_cached = hud_chat_messages_from_world_cached(&world, None, &mut cache);
        assert_eq!(second_cached, hud_chat_messages_from_world(&world, None));
        assert_eq!(cache.formatted_messages, 2);

        let repeated = hud_chat_messages_from_world_cached(&world, None, &mut cache);
        assert_eq!(repeated, second_cached);
        assert_eq!(cache.formatted_messages, 2);
    }

    #[test]
    fn hud_render_runtime_gates_unchanged_static_ui_and_screen_frames() {
        let mut hud_rt = super::super::hud_presentations::HudRenderRuntime::default();
        let static_key = super::super::hud_presentations::HudStaticUiCacheKey {
            step_generation: 1,
            world_revision: 10,
            client_tick_index: 5,
            level_generation: 1,
            chat_delivery_revision: 0,
            chat_display_refresh_revision: 0,
            resource_generation: 0,
            resources_loaded: false,
            gui_surface_size: winit::dpi::PhysicalSize::new(1280, 720),
            hud_cursor_position: None,
        };
        let mut static_calls = 0usize;
        let build_static = |_: &mut super::super::hud_presentations::HudRenderRuntime| {
            static_calls += 1;
            HudStaticUiFrame {
                hide_gui: false,
                hud_sleep_overlay: None,
                hud_food: Some(20),
                hud_food_effect: HudFoodEffect {
                    saturation_empty: false,
                    hunger_effect: false,
                    tick_count: 5,
                },
                hud_mob_effects: Vec::new(),
                hud_inventory_shows_mob_effects: false,
                hud_armor: Some(0),
                hud_air: None,
                hud_vehicle_health: None,
                hud_experience_level: None,
                hud_selected_slot: 0,
                hud_camera_overlays: Vec::new(),
                hud_chat_context: HudChatContext::default(),
            }
        };
        assert!(matches!(
            hud_rt.static_ui_update(static_key, build_static),
            RendererFrameUpdate::Replace(_)
        ));
        assert_eq!(static_calls, 1);
        assert!(matches!(
            hud_rt.static_ui_update(static_key, |_| unreachable!("cached static UI")),
            RendererFrameUpdate::Unchanged
        ));

        let mut changed_key = static_key;
        changed_key.step_generation = 2;
        assert!(matches!(
            hud_rt.static_ui_update(changed_key, |_| HudStaticUiFrame {
                hide_gui: true,
                hud_sleep_overlay: None,
                hud_food: None,
                hud_food_effect: HudFoodEffect {
                    saturation_empty: false,
                    hunger_effect: false,
                    tick_count: 5,
                },
                hud_mob_effects: Vec::new(),
                hud_inventory_shows_mob_effects: false,
                hud_armor: None,
                hud_air: None,
                hud_vehicle_health: None,
                hud_experience_level: None,
                hud_selected_slot: 0,
                hud_camera_overlays: Vec::new(),
                hud_chat_context: HudChatContext::default(),
            }),
            RendererFrameUpdate::Replace(_)
        ));
    }
}
