use super::*;

pub(super) fn sanitize_hud_mob_effect(mut effect: HudMobEffect) -> HudMobEffect {
    effect.display_name = sanitize_hud_text_preserving_empty(effect.display_name, 256);
    effect.duration_text = sanitize_hud_text_preserving_empty(effect.duration_text, 64);
    effect
}

pub(super) fn sanitize_hud_sleep_overlay(
    overlay: Option<HudSleepOverlay>,
) -> Option<HudSleepOverlay> {
    overlay.filter(|overlay| overlay.alpha > 0)
}

pub(super) fn sanitize_hud_contextual_bar(contextual_bar: HudContextualBar) -> HudContextualBar {
    match contextual_bar {
        HudContextualBar::Empty => HudContextualBar::Empty,
        HudContextualBar::Experience { progress } if progress.is_finite() => {
            HudContextualBar::Experience {
                progress: progress.clamp(0.0, 1.0),
            }
        }
        HudContextualBar::Experience { .. } => HudContextualBar::Empty,
        HudContextualBar::Jump(jump) if jump.progress.is_finite() => {
            HudContextualBar::Jump(HudJumpBar {
                progress: jump.progress.clamp(0.0, 1.0),
                cooldown: jump.cooldown,
            })
        }
        HudContextualBar::Jump(_) => HudContextualBar::Empty,
        HudContextualBar::Locator(mut locator) => {
            locator.markers.retain(|marker| {
                marker.angle_degrees.is_finite()
                    && marker.angle_degrees > -60.0
                    && marker.angle_degrees <= 60.0
                    && !marker.distance.is_nan()
                    && !marker.distance.is_sign_negative()
            });
            for marker in &mut locator.markers {
                marker.color_argb |= 0xFF00_0000;
            }
            HudContextualBar::Locator(locator)
        }
    }
}

pub(super) fn sanitize_hud_chat_restrictions_screen(
    mut screen: HudChatRestrictionsScreen,
) -> Option<HudChatRestrictionsScreen> {
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 256);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 128);
    let reason_text = sanitize_hud_text_preserving_empty(screen.reason.text, 1_024);
    if reason_text.is_empty() {
        return None;
    }
    screen.reason.text = reason_text;
    screen.permissions.truncate(4);
    screen.permissions.retain_mut(|line| {
        line.text = sanitize_hud_text_preserving_empty(std::mem::take(&mut line.text), 512);
        !line.text.is_empty()
    });
    (screen.permissions.len() == 4 && !screen.title.is_empty() && !screen.done.is_empty())
        .then_some(screen)
}

/// Clamps a projected bar's progress into `0.0..=1.0` (non-finite fills
/// nothing): vanilla trusts the `ClientboundBossEventPacket` float verbatim,
/// but an out-of-range fill would sample past the 182px sheet.
pub(super) fn sanitize_hud_boss_bar(bar: HudBossBar) -> HudBossBar {
    HudBossBar {
        progress: if bar.progress.is_finite() {
            bar.progress.clamp(0.0, 1.0)
        } else {
            0.0
        },
        ..bar
    }
}

pub(super) fn sanitize_hud_uv_rect(rect: HudUvRect) -> Option<HudUvRect> {
    let components = [rect.min[0], rect.min[1], rect.max[0], rect.max[1]];
    if !components.iter().all(|component| component.is_finite()) {
        return None;
    }

    let min_x = rect.min[0].clamp(0.0, 1.0);
    let min_y = rect.min[1].clamp(0.0, 1.0);
    let max_x = rect.max[0].clamp(0.0, 1.0);
    let max_y = rect.max[1].clamp(0.0, 1.0);
    Some(HudUvRect {
        min: [min_x.min(max_x), min_y.min(max_y)],
        max: [min_x.max(max_x), min_y.max(max_y)],
    })
}

pub(super) fn sanitize_hud_hotbar_item_uv(uv: HudUvRect) -> Option<HudItemIcon> {
    sanitize_hud_item_icon(HudItemIcon::single(uv))
}

pub(super) fn sanitize_hud_inventory_screen(screen: HudInventoryScreen) -> HudInventoryScreen {
    let mut background_layers = screen.background_layers;
    retain_map_in_place(
        &mut background_layers,
        sanitize_hud_inventory_background_layer,
    );
    let background_layer_count = background_layers.len();

    let mut map_previews = screen.map_previews;
    map_previews.retain(|preview| {
        preview.map_id >= 0
            && preview.scale.is_finite()
            && preview.scale > 0.0
            && preview.scale <= 4.0
            && preview.after_background_layer < background_layer_count
    });
    map_previews.truncate(MAX_HUD_INVENTORY_MAP_PREVIEWS);

    let mut banner_pattern_previews = screen.banner_pattern_previews;
    banner_pattern_previews.retain_mut(|preview| {
        if preview.width == 0
            || preview.height == 0
            || preview.width > 64
            || preview.height > 64
            || preview.after_background_layer >= background_layer_count
        {
            return false;
        }
        let Some(uv) = sanitize_hud_uv_rect(preview.uv) else {
            return false;
        };
        preview.uv = uv;
        true
    });
    banner_pattern_previews.truncate(MAX_HUD_INVENTORY_BANNER_PATTERN_PREVIEWS);

    let mut foreground_layers = screen.foreground_layers;
    retain_map_in_place(
        &mut foreground_layers,
        sanitize_hud_inventory_background_layer,
    );

    let mut fill_layers = screen.fill_layers;
    retain_map_in_place(&mut fill_layers, sanitize_hud_inventory_fill_layer);

    let mut slots = screen.slots;
    for slot in &mut slots {
        *slot = sanitize_hud_inventory_slot(std::mem::replace(
            slot,
            HudInventorySlot {
                slot_id: 0,
                x: 0,
                y: 0,
                icon: None,
                block_model: None,
            },
        ));
    }

    let mut floating_items = screen.floating_items;
    retain_map_in_place(&mut floating_items, sanitize_hud_inventory_item);

    let mut foreground_items = screen.foreground_items;
    retain_map_in_place(&mut foreground_items, sanitize_hud_inventory_item);

    let mut ghost_items = screen.ghost_items;
    retain_map_in_place(&mut ghost_items, sanitize_hud_inventory_ghost_item);

    let mut entity_previews = screen.entity_previews;
    retain_map_in_place(&mut entity_previews, |preview| {
        sanitize_hud_entity_preview(preview).filter(|preview| {
            preview
                .after_background_layer
                .is_none_or(|layer| layer < background_layer_count)
        })
    });

    let mut text_labels = screen.text_labels;
    retain_map_in_place(&mut text_labels, sanitize_hud_inventory_text_label);

    HudInventoryScreen {
        width: screen.width.clamp(1, 512),
        height: screen.height.clamp(1, 512),
        background_layers,
        foreground_layers,
        fill_layers,
        slots,
        floating_items,
        foreground_items,
        ghost_items,
        entity_previews,
        map_previews,
        banner_pattern_previews,
        text_labels,
        scroll_area: screen
            .scroll_area
            .and_then(sanitize_hud_inventory_scroll_area),
        preedit_overlay: screen
            .preedit_overlay
            .and_then(sanitize_hud_inventory_preedit_overlay),
        hovered_slot_id: screen.hovered_slot_id,
        tooltip: screen.tooltip.and_then(sanitize_hud_inventory_tooltip),
    }
}

pub(super) fn sanitize_hud_inventory_preedit_overlay(
    mut overlay: HudInventoryPreeditOverlay,
) -> Option<HudInventoryPreeditOverlay> {
    if overlay.background_width == 0 || overlay.background_height == 0 {
        return None;
    }
    let (text, text_len) = sanitize_hud_chars_with_count(overlay.text, 256, false);
    if text.is_empty() {
        return None;
    }
    overlay.text = text;
    overlay.focused_start = overlay.focused_start.min(text_len);
    overlay.focused_end = overlay.focused_end.clamp(overlay.focused_start, text_len);
    overlay.caret = overlay.caret.map(|caret| caret.min(text_len));
    overlay.background_width = overlay.background_width.min(2048);
    overlay.background_height = overlay.background_height.min(512);
    Some(overlay)
}

fn sanitize_hud_inventory_scroll_area(
    area: HudInventoryScrollArea,
) -> Option<HudInventoryScrollArea> {
    let scissor = sanitize_hud_inventory_item_scissor(area.scissor)?;
    let mut background_layers = area.background_layers;
    retain_map_in_place(
        &mut background_layers,
        sanitize_hud_inventory_background_layer,
    );
    let mut fill_layers = area.fill_layers;
    retain_map_in_place(&mut fill_layers, sanitize_hud_inventory_fill_layer);
    let mut text_labels = area.text_labels;
    retain_map_in_place(&mut text_labels, sanitize_hud_inventory_text_label);
    let mut text_areas = area.text_areas;
    retain_map_in_place(&mut text_areas, sanitize_hud_inventory_text_area);
    Some(HudInventoryScrollArea {
        scissor,
        background_layers,
        fill_layers,
        text_labels,
        text_areas,
        scrollbar: area.scrollbar.and_then(sanitize_hud_inventory_scrollbar),
    })
}

pub(super) fn sanitize_hud_inventory_text_area(
    mut area: HudInventoryTextArea,
) -> Option<HudInventoryTextArea> {
    if area.width == 0
        || area.height == 0
        || !area
            .text_tint
            .iter()
            .chain(area.cursor_tint.iter())
            .chain(area.counter_tint.iter())
            .all(|component| component.is_finite())
    {
        return None;
    }
    area.width = area.width.min(1024);
    area.height = area.height.min(512);
    let units = area.text.encode_utf16().count();
    area.cursor = area.cursor.min(units);
    area.selection = area.selection.min(units);
    area.scroll_amount = area.scroll_amount.max(0);
    area.text_tint = area.text_tint.map(|component| component.clamp(0.0, 1.0));
    area.cursor_tint = area.cursor_tint.map(|component| component.clamp(0.0, 1.0));
    area.counter_tint = area.counter_tint.map(|component| component.clamp(0.0, 1.0));
    Some(area)
}

fn sanitize_hud_inventory_scrollbar(
    scrollbar: HudInventoryScrollbar,
) -> Option<HudInventoryScrollbar> {
    if scrollbar.width == 0 || scrollbar.height == 0 || scrollbar.thumb_height == 0 {
        return None;
    }
    let width = scrollbar.width.min(512);
    let height = scrollbar.height.min(512);
    let thumb_height = scrollbar.thumb_height.min(height);
    let thumb_y = scrollbar.thumb_y.clamp(
        scrollbar.y,
        scrollbar
            .y
            .saturating_add(i32::try_from(height.saturating_sub(thumb_height)).unwrap_or(i32::MAX)),
    );
    Some(HudInventoryScrollbar {
        width,
        height,
        thumb_y,
        thumb_height,
        ..scrollbar
    })
}

pub(super) fn sanitize_hud_sign_editor_screen(
    mut screen: HudSignEditorScreen,
) -> Option<HudSignEditorScreen> {
    if !screen
        .text_tint
        .iter()
        .all(|component| component.is_finite())
    {
        return None;
    }
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 256);
    if screen.title.is_empty() {
        return None;
    }
    screen.lines = screen
        .lines
        .map(|line| sanitize_hud_text_preserving_empty(line, usize::MAX));
    screen.line = screen.line.min(screen.lines.len().saturating_sub(1));
    let line_len = screen.lines[screen.line].chars().count();
    screen.cursor = screen.cursor.min(line_len);
    screen.selection = screen.selection.min(line_len);
    screen.text_tint = screen.text_tint.map(|component| component.clamp(0.0, 1.0));
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 256);
    if screen.done.is_empty() {
        return None;
    }
    screen.preedit = screen.preedit.and_then(sanitize_hud_ime_preedit);
    screen.sign_preview = match screen.kind {
        HudSignEditorKind::Standing { .. } => {
            screen.sign_preview.and_then(sanitize_hud_entity_preview)
        }
        HudSignEditorKind::Hanging { .. } => None,
    };
    Some(screen)
}

pub(super) fn sanitize_hud_ime_preedit(mut preedit: HudImePreedit) -> Option<HudImePreedit> {
    let (text, text_len) = sanitize_hud_chars_with_count(preedit.text, 256, false);
    if text.is_empty() {
        return None;
    }
    preedit.text = text;
    preedit.focused_start = preedit.focused_start.min(text_len);
    preedit.focused_end = preedit.focused_end.clamp(preedit.focused_start, text_len);
    preedit.caret = preedit.caret.map(|caret| caret.min(text_len));
    Some(preedit)
}

pub(super) fn sanitize_hud_command_block_editor_screen(
    mut screen: HudCommandBlockEditorScreen,
) -> Option<HudCommandBlockEditorScreen> {
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 256);
    screen.command_label = sanitize_hud_text_preserving_empty(screen.command_label, 256);
    screen.previous_output_label =
        sanitize_hud_text_preserving_empty(screen.previous_output_label, 256);
    screen.previous_output =
        sanitize_hud_text_preserving_empty_utf16(screen.previous_output, 32_500);
    screen.mode = sanitize_hud_text_preserving_empty(screen.mode, 256);
    screen.conditional = sanitize_hud_text_preserving_empty(screen.conditional, 256);
    screen.automatic = sanitize_hud_text_preserving_empty(screen.automatic, 256);
    screen.track_output = sanitize_hud_text_preserving_empty(screen.track_output, 16);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 256);
    screen.cancel = sanitize_hud_text_preserving_empty(screen.cancel, 256);
    (!screen.title.is_empty()
        && !screen.command_label.is_empty()
        && !screen.done.is_empty()
        && !screen.cancel.is_empty())
    .then_some(screen)
}

pub(super) fn sanitize_hud_structure_block_editor_screen(
    mut screen: HudStructureBlockEditorScreen,
) -> Option<HudStructureBlockEditorScreen> {
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 256);
    screen.name_label = sanitize_hud_text_preserving_empty(screen.name_label, 256);
    screen.position_label = sanitize_hud_text_preserving_empty(screen.position_label, 256);
    screen.size_label = sanitize_hud_text_preserving_empty(screen.size_label, 256);
    screen.integrity_label = sanitize_hud_text_preserving_empty(screen.integrity_label, 256);
    screen.data_label = sanitize_hud_text_preserving_empty(screen.data_label, 256);
    screen.include_entities_label =
        sanitize_hud_text_preserving_empty(screen.include_entities_label, 256);
    screen.strict_label = sanitize_hud_text_preserving_empty(screen.strict_label, 256);
    screen.detect_label = sanitize_hud_text_preserving_empty(screen.detect_label, 256);
    screen.option_label = sanitize_hud_text_preserving_empty(screen.option_label, 256);
    screen.mode_description = sanitize_hud_text_preserving_empty(screen.mode_description, 256);
    let mut field_index = 0usize;
    screen.fields = screen.fields.map(|field| {
        let max = match field_index {
            0 | 9 => 128,
            8 => 31,
            _ => 15,
        };
        field_index += 1;
        sanitize_hud_text_field(field, max)
    });
    screen.mode_value = sanitize_hud_text_preserving_empty(screen.mode_value, 64);
    screen.mirror_value = sanitize_hud_text_preserving_empty(screen.mirror_value, 64);
    screen.rotation = sanitize_hud_text_preserving_empty(screen.rotation, 16);
    screen.include_entities = sanitize_hud_text_preserving_empty(screen.include_entities, 16);
    screen.strict = sanitize_hud_text_preserving_empty(screen.strict, 16);
    screen.option = sanitize_hud_text_preserving_empty(screen.option, 16);
    screen.action = sanitize_hud_text_preserving_empty(screen.action, 256);
    screen.detect = sanitize_hud_text_preserving_empty(screen.detect, 256);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 256);
    screen.cancel = sanitize_hud_text_preserving_empty(screen.cancel, 256);
    (!screen.title.is_empty()
        && !screen.mode_value.is_empty()
        && !screen.done.is_empty()
        && !screen.cancel.is_empty())
    .then_some(screen)
}

pub(super) fn sanitize_hud_jigsaw_block_editor_screen(
    mut screen: HudJigsawBlockEditorScreen,
) -> Option<HudJigsawBlockEditorScreen> {
    screen.labels = screen
        .labels
        .map(|label| sanitize_hud_text_preserving_empty(label, 256));
    screen.joint_label = sanitize_hud_text_preserving_empty(screen.joint_label, 256);
    let mut field_index = 0usize;
    screen.fields = screen.fields.map(|field| {
        let max = match field_index {
            0..=2 => 128,
            3 => 256,
            _ => 3,
        };
        field_index += 1;
        sanitize_hud_text_field(field, max)
    });
    screen.joint = sanitize_hud_text_preserving_empty(screen.joint, 64);
    screen.levels = sanitize_hud_text_preserving_empty(screen.levels, 64);
    let slider_value = f64::from_bits(screen.slider_value_bits);
    screen.slider_value_bits = if slider_value.is_finite() {
        slider_value.clamp(0.0, 1.0).to_bits()
    } else {
        0.0_f64.to_bits()
    };
    screen.keep_jigsaws = sanitize_hud_text_preserving_empty(screen.keep_jigsaws, 128);
    screen.generate = sanitize_hud_text_preserving_empty(screen.generate, 256);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 256);
    screen.cancel = sanitize_hud_text_preserving_empty(screen.cancel, 256);
    screen.tooltip = screen.tooltip.and_then(|mut tooltip| {
        tooltip.text = sanitize_hud_chars_with_count(tooltip.text, 4096, true).0;
        (!tooltip.text.is_empty()).then_some(tooltip)
    });
    (screen.labels.iter().all(|label| !label.is_empty())
        && !screen.levels.is_empty()
        && !screen.generate.is_empty()
        && !screen.done.is_empty()
        && !screen.cancel.is_empty())
    .then_some(screen)
}

pub(super) fn sanitize_hud_test_block_editor_screen(
    mut screen: HudTestBlockEditorScreen,
) -> Option<HudTestBlockEditorScreen> {
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 256);
    screen.message_label = sanitize_hud_text_preserving_empty(screen.message_label, 256);
    screen.message = sanitize_hud_text_field(screen.message, 128);
    screen.mode = sanitize_hud_text_preserving_empty(screen.mode, 64);
    screen.mode_info = sanitize_hud_text_preserving_empty(screen.mode_info, 256);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 256);
    screen.cancel = sanitize_hud_text_preserving_empty(screen.cancel, 256);
    (!screen.title.is_empty()
        && !screen.message_label.is_empty()
        && !screen.mode.is_empty()
        && !screen.mode_info.is_empty()
        && !screen.done.is_empty()
        && !screen.cancel.is_empty())
    .then_some(screen)
}

pub(super) fn sanitize_hud_test_instance_block_editor_screen(
    mut screen: HudTestInstanceBlockEditorScreen,
) -> Option<HudTestInstanceBlockEditorScreen> {
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 256);
    screen.id_label = sanitize_hud_text_preserving_empty(screen.id_label, 256);
    screen.size_label = sanitize_hud_text_preserving_empty(screen.size_label, 256);
    screen.rotation_label = sanitize_hud_text_preserving_empty(screen.rotation_label, 256);
    screen.entities_label = sanitize_hud_text_preserving_empty(screen.entities_label, 256);
    screen.id = sanitize_hud_text_field(screen.id, 128);
    screen.size = screen.size.map(|field| sanitize_hud_text_field(field, 15));
    let mut info_budget = 4096usize;
    screen.info_runs.truncate(256);
    screen.info_runs.retain_mut(|run| {
        if info_budget == 0 {
            return false;
        }
        let (text, count) =
            sanitize_hud_chars_with_count(std::mem::take(&mut run.text), info_budget, true);
        info_budget = info_budget.saturating_sub(count);
        if text.is_empty() {
            return false;
        }
        run.text = text;
        run.color = run.color.map(|color| color & 0xFF_FF_FF);
        true
    });
    screen.rotation = sanitize_hud_text_preserving_empty(screen.rotation, 64);
    screen.include_entities = sanitize_hud_text_preserving_empty(screen.include_entities, 64);
    screen.reset = sanitize_hud_text_preserving_empty(screen.reset, 256);
    screen.save = sanitize_hud_text_preserving_empty(screen.save, 256);
    screen.run = sanitize_hud_text_preserving_empty(screen.run, 256);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 256);
    screen.cancel = sanitize_hud_text_preserving_empty(screen.cancel, 256);
    (!screen.title.is_empty()
        && !screen.id_label.is_empty()
        && !screen.size_label.is_empty()
        && !screen.done.is_empty()
        && !screen.cancel.is_empty())
    .then_some(screen)
}

pub(super) fn sanitize_hud_text_field(mut field: HudTextField, max_length: usize) -> HudTextField {
    field.max_length = field.max_length.min(max_length);
    let (text, units) = sanitize_hud_utf16_with_units(field.text, field.max_length, false);
    field.text = text;
    field.cursor = field.cursor.min(units);
    field.selection = field.selection.min(units);
    field.scroll_to = field.scroll_to.min(units);
    field.preedit = field.preedit.and_then(sanitize_hud_ime_preedit);
    field
}

pub(super) fn sanitize_hud_inventory_background_layer(
    mut layer: HudInventoryBackgroundLayer,
) -> Option<HudInventoryBackgroundLayer> {
    if let HudInventoryBackgroundTexture::FadedSprite { id, alpha_bits } = layer.texture {
        let alpha = f32::from_bits(alpha_bits);
        if !alpha.is_finite() {
            return None;
        }
        layer.texture = HudInventoryBackgroundTexture::FadedSprite {
            id,
            alpha_bits: alpha.clamp(0.0, 1.0).to_bits(),
        };
    }
    let uv = match layer.texture {
        HudInventoryBackgroundTexture::Texture("textures/gui/inworld_menu_background.png") => {
            sanitize_repeating_hud_uv_rect(layer.uv)?
        }
        _ => sanitize_hud_uv_rect(layer.uv)?,
    };
    (layer.width > 0 && layer.height > 0).then_some(HudInventoryBackgroundLayer { uv, ..layer })
}

fn sanitize_repeating_hud_uv_rect(rect: HudUvRect) -> Option<HudUvRect> {
    let components = [rect.min[0], rect.min[1], rect.max[0], rect.max[1]];
    if !components.iter().all(|component| component.is_finite()) {
        return None;
    }
    Some(HudUvRect {
        min: [rect.min[0].min(rect.max[0]), rect.min[1].min(rect.max[1])],
        max: [rect.min[0].max(rect.max[0]), rect.min[1].max(rect.max[1])],
    })
}

fn sanitize_hud_inventory_fill_layer(
    layer: HudInventoryFillLayer,
) -> Option<HudInventoryFillLayer> {
    if layer.width == 0
        || layer.height == 0
        || !layer.tint.iter().all(|component| component.is_finite())
    {
        return None;
    }
    Some(HudInventoryFillLayer {
        tint: layer.tint.map(|component| component.clamp(0.0, 1.0)),
        ..layer
    })
}

pub(super) fn sanitize_hud_inventory_slot(slot: HudInventorySlot) -> HudInventorySlot {
    HudInventorySlot {
        slot_id: slot.slot_id,
        x: slot.x,
        y: slot.y,
        icon: slot.icon.and_then(sanitize_hud_item_icon),
        block_model: slot.block_model.filter(hud_block_item_model_is_renderable),
    }
}

pub(super) fn sanitize_hud_inventory_item(item: HudInventoryItem) -> Option<HudInventoryItem> {
    if !item.scale.is_finite()
        || item.scale <= 0.0
        || !item.scale_y.is_finite()
        || item.scale_y <= 0.0
    {
        return None;
    }
    let scale = item.scale.clamp(0.0625, 16.0);
    let scale_y = item.scale_y.clamp(0.0625, 16.0);
    let scissor = item.scissor.and_then(sanitize_hud_inventory_item_scissor);
    Some(HudInventoryItem {
        x: item.x,
        y: item.y,
        scale,
        scale_y,
        icon: sanitize_hud_item_icon(item.icon)?,
        scissor,
        draw_decorations: item.draw_decorations,
        block_model: item.block_model.filter(hud_block_item_model_is_renderable),
    })
}

fn sanitize_hud_inventory_item_scissor(
    scissor: HudInventoryItemScissor,
) -> Option<HudInventoryItemScissor> {
    (scissor.width > 0 && scissor.height > 0).then_some(HudInventoryItemScissor {
        width: scissor.width.min(512),
        height: scissor.height.min(512),
        ..scissor
    })
}

fn sanitize_hud_inventory_ghost_item(item: HudInventoryGhostItem) -> Option<HudInventoryGhostItem> {
    Some(HudInventoryGhostItem {
        x: item.x,
        y: item.y,
        icon: sanitize_hud_item_icon(item.icon)?,
        draw_decorations: item.draw_decorations,
    })
}

pub(super) fn sanitize_hud_entity_preview(
    mut preview: HudEntityPreview,
) -> Option<HudEntityPreview> {
    let is_flat_gui_preview = is_flat_gui_preview(&preview);
    if (!is_flat_gui_preview && preview.lighting != GuiItemLightingEntry::EntityInUi)
        || !preview.depth_isolated
    {
        return None;
    }
    preview.rect = sanitize_hud_entity_preview_rect(preview.rect)?;
    preview.scissor = match preview.scissor {
        Some(scissor) => Some(sanitize_hud_entity_preview_rect(scissor)?),
        None => None,
    };
    preview.visible_bounds()?;
    if !preview.scale.is_finite() || preview.scale <= 0.0 {
        return None;
    }
    if !preview
        .translation
        .iter()
        .all(|component| component.is_finite())
        || !preview
            .rotation
            .iter()
            .all(|component| component.is_finite())
        || !preview
            .override_camera_rotation
            .map(|rotation| rotation.iter().all(|component| component.is_finite()))
            .unwrap_or(true)
    {
        return None;
    }

    preview.entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    preview.entity.render_state.outline_color = 0;
    preview.entity.render_state.appears_glowing = false;
    for layer in &mut preview.item_layers {
        if layer.item_id < 0 || layer.count <= 0 {
            return None;
        }
        layer.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
        layer.overlay = ITEM_MODEL_NO_OVERLAY;
    }
    Some(preview)
}

pub(super) fn sanitize_hud_entity_preview_rect(
    rect: HudEntityPreviewRect,
) -> Option<HudEntityPreviewRect> {
    // Preview bounds live inside the (512-clamped) inventory screen; the clamp also bounds the
    // per-preview PIP color/depth texture allocation.
    (rect.width > 0 && rect.height > 0).then_some(HudEntityPreviewRect {
        width: rect.width.min(512),
        height: rect.height.min(512),
        ..rect
    })
}

pub(super) fn sanitize_hud_inventory_text_label(
    label: HudInventoryTextLabel,
) -> Option<HudInventoryTextLabel> {
    if !label.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    let x = label.x;
    let y = label.y;
    let width = label.width;
    let tint = label.tint;
    let background = label
        .background
        .and_then(sanitize_hud_inventory_text_background);
    let utf16_widget = label.input.is_some();
    let component_runs = if utf16_widget {
        Vec::new()
    } else {
        sanitize_hud_component_runs(label.component_runs, "", &mut 256, false)
    };
    let canonical_text = if component_runs.is_empty() {
        label.text
    } else {
        component_runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect()
    };
    let text = if utf16_widget {
        canonical_text
    } else {
        sanitize_hud_text_line(canonical_text)?
    };
    let input = label
        .input
        .and_then(|input| sanitize_hud_inventory_text_input(input, &text));
    if utf16_widget && input.is_none() {
        return None;
    }
    let scrolling = if input.is_none() {
        label
            .scrolling
            .and_then(sanitize_hud_inventory_scrolling_text)
    } else {
        None
    };
    let font = label
        .font
        .map(|font| sanitize_hud_text_preserving_empty(font, 256))
        .filter(|font| !font.is_empty());
    if text.is_empty() && input.is_none() {
        return None;
    }
    let runs = if utf16_widget {
        let mut runs = label.runs;
        for run in &mut runs {
            run.color = run.color.map(|color| color & 0xFF_FF_FF);
        }
        runs
    } else {
        sanitize_hud_styled_runs(label.runs, &text)
    };
    let width = if !component_runs.is_empty() && input.is_none() {
        width.max(1)
    } else {
        width
    };
    (width > 0).then_some(HudInventoryTextLabel {
        component_runs,
        x,
        y,
        width: width.min(512),
        text,
        tint: tint.map(|component| component.clamp(0.0, 1.0)),
        background,
        input,
        scrolling,
        shadow: label.shadow,
        font,
        runs,
    })
}

fn sanitize_hud_inventory_scrolling_text(
    scrolling: HudInventoryScrollingText,
) -> Option<HudInventoryScrollingText> {
    (scrolling.width > 0 && scrolling.height > 0).then_some(HudInventoryScrollingText {
        width: scrolling.width.min(1024),
        height: scrolling.height.min(512),
        ..scrolling
    })
}

fn sanitize_hud_inventory_text_input(
    input: HudInventoryTextInputDecoration,
    text: &str,
) -> Option<HudInventoryTextInputDecoration> {
    if !input
        .cursor_tint
        .iter()
        .all(|component| component.is_finite())
    {
        return None;
    }
    let units = text.encode_utf16().count();
    Some(HudInventoryTextInputDecoration {
        cursor: input.cursor.min(units),
        selection: input.selection.min(units),
        scroll_to: input.scroll_to.min(units),
        max_length: input.max_length,
        cursor_visible: input.cursor_visible,
        cursor_tint: input.cursor_tint.map(|component| component.clamp(0.0, 1.0)),
    })
}

fn sanitize_hud_inventory_text_background(
    background: HudInventoryTextBackground,
) -> Option<HudInventoryTextBackground> {
    if !background
        .tint
        .iter()
        .all(|component| component.is_finite())
    {
        return None;
    }
    (background.width > 0 && background.height > 0).then_some(HudInventoryTextBackground {
        width: background.width.min(512),
        height: background.height.min(512),
        tint: background.tint.map(|component| component.clamp(0.0, 1.0)),
        ..background
    })
}

fn sanitize_hud_inventory_tooltip(tooltip: HudInventoryTooltip) -> Option<HudInventoryTooltip> {
    let mut lines = tooltip.lines;
    retain_map_in_place(&mut lines, sanitize_hud_inventory_tooltip_line);
    lines.truncate(16);
    lines
        .iter()
        .any(|line| !line.text.is_empty())
        .then_some(HudInventoryTooltip { lines, ..tooltip })
}

pub(super) fn sanitize_hud_bundle_tooltip(bundle: HudBundleTooltip) -> HudBundleTooltip {
    let mut items = bundle
        .items
        .into_iter()
        .take(12)
        .filter_map(sanitize_hud_item_icon)
        .collect::<Vec<_>>();
    let max_shown = if bundle.hidden_item_count > 0 { 11 } else { 12 };
    items.truncate(max_shown);
    let selected_index = bundle.selected_index.filter(|&index| index < items.len());
    let selected_item_name = selected_index
        .and(bundle.selected_item_name)
        .and_then(sanitize_hud_inventory_tooltip_line)
        .filter(|line| !line.text.is_empty());
    let selected_item_tooltip_style = selected_item_name
        .as_ref()
        .and(bundle.selected_item_tooltip_style)
        .map(|style| sanitize_hud_text_preserving_empty(style, 256))
        .filter(|style| !style.is_empty());
    let occupancy = if bundle.occupancy.is_finite() {
        bundle.occupancy.max(0.0)
    } else {
        0.0
    };
    let empty_description = sanitize_hud_text_preserving_empty(bundle.empty_description, 256);
    let progress_label = bundle
        .progress_label
        .map(|label| sanitize_hud_text_preserving_empty(label, 64))
        .filter(|label| !label.is_empty());
    HudBundleTooltip {
        items,
        hidden_item_count: bundle.hidden_item_count,
        selected_index,
        selected_item_name,
        selected_item_tooltip_style,
        occupancy,
        empty_description,
        progress_label,
    }
}

pub(super) fn sanitize_hud_inventory_tooltip_line(
    line: HudInventoryTooltipLine,
) -> Option<HudInventoryTooltipLine> {
    if !line.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    // Empty Components are meaningful vertical separators in vanilla item and
    // enchanting tooltips, so retain them while still stripping controls.
    let text = sanitize_hud_text_preserving_empty(line.text, 256);
    let mut budget = 256usize;
    let runs = sanitize_hud_component_runs(line.runs, &text, &mut budget, false);
    Some(HudInventoryTooltipLine {
        text,
        tint: line.tint.map(|component| component.clamp(0.0, 1.0)),
        runs,
    })
}

pub(super) fn sanitize_hud_debug_overlay(overlay: HudDebugOverlay) -> Option<HudDebugOverlay> {
    let left_lines = sanitize_hud_debug_overlay_lines(overlay.left_lines);
    let right_lines = sanitize_hud_debug_overlay_lines(overlay.right_lines);
    let debug_crosshair = overlay
        .debug_crosshair
        .and_then(sanitize_hud_debug_crosshair);
    let game_mode_switcher = overlay
        .game_mode_switcher
        .and_then(sanitize_hud_debug_game_mode_switcher);
    let profiler_chart = overlay
        .profiler_chart
        .map(sanitize_hud_debug_profiler_chart);
    let fps_chart = overlay.fps_chart.map(sanitize_hud_debug_fps_chart);
    let tps_chart = overlay.tps_chart.map(sanitize_hud_debug_tps_chart);
    let network_charts = overlay
        .network_charts
        .map(sanitize_hud_debug_network_charts);
    (!left_lines.is_empty()
        || !right_lines.is_empty()
        || debug_crosshair.is_some()
        || game_mode_switcher.is_some()
        || profiler_chart.is_some()
        || fps_chart.is_some()
        || tps_chart.is_some()
        || network_charts.is_some()
        || overlay.show_lightmap_preview)
        .then_some(HudDebugOverlay {
            left_lines,
            right_lines,
            debug_crosshair,
            game_mode_switcher,
            profiler_chart,
            fps_chart,
            tps_chart,
            network_charts,
            show_lightmap_preview: overlay.show_lightmap_preview,
        })
}

fn sanitize_hud_debug_game_mode_switcher(
    switcher: HudDebugGameModeSwitcher,
) -> Option<HudDebugGameModeSwitcher> {
    let title = sanitize_hud_text_line(switcher.title)?;
    let help_runs = sanitize_hud_styled_runs(switcher.help_runs, "");
    let mut slots = switcher.slots;
    retain_map_in_place(&mut slots, sanitize_hud_debug_game_mode_switcher_slot);
    slots.truncate(4);
    (switcher.background_width > 0
        && switcher.background_height > 0
        && slots.len() == 4
        && help_runs.iter().any(|run| !run.text.is_empty()))
    .then_some(HudDebugGameModeSwitcher {
        title,
        help_runs,
        slots,
        ..switcher
    })
}

fn sanitize_pause_screen_tooltip(tooltip: HudPauseScreenTooltip) -> Option<HudPauseScreenTooltip> {
    let text = sanitize_hud_multiline_text(tooltip.text, 512);
    (!text.is_empty()).then_some(HudPauseScreenTooltip { text, ..tooltip })
}

fn sanitize_hud_label_or_default(text: String, fallback: &str) -> String {
    sanitize_hud_text_line(text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn sanitize_pause_screen_labels(labels: HudPauseScreenLabels) -> HudPauseScreenLabels {
    let defaults = HudPauseScreenLabels::default();
    HudPauseScreenLabels {
        return_to_game: sanitize_hud_label_or_default(
            labels.return_to_game,
            &defaults.return_to_game,
        ),
        advancements: sanitize_hud_label_or_default(labels.advancements, &defaults.advancements),
        stats: sanitize_hud_label_or_default(labels.stats, &defaults.stats),
        send_feedback: sanitize_hud_label_or_default(labels.send_feedback, &defaults.send_feedback),
        feedback: sanitize_hud_label_or_default(labels.feedback, &defaults.feedback),
        report_bugs: sanitize_hud_label_or_default(labels.report_bugs, &defaults.report_bugs),
        back: sanitize_hud_label_or_default(labels.back, &defaults.back),
        options: sanitize_hud_label_or_default(labels.options, &defaults.options),
        disconnect: sanitize_hud_label_or_default(labels.disconnect, &defaults.disconnect),
    }
}

pub(super) fn sanitize_hud_pause_screen(screen: HudPauseScreen) -> Option<HudPauseScreen> {
    let title = sanitize_hud_text_line(screen.title)?;
    let labels = sanitize_pause_screen_labels(screen.labels);
    let custom_dialog_label = screen
        .custom_dialog_label
        .map(|label| sanitize_hud_text_preserving_empty(label, 256));
    let custom_dialog_tooltip = screen
        .custom_dialog_tooltip
        .and_then(sanitize_pause_screen_tooltip);
    let options_tooltip = screen
        .options_tooltip
        .and_then(sanitize_pause_screen_tooltip);
    (!title.is_empty()).then_some(HudPauseScreen {
        title,
        labels,
        show_pause_menu: screen.show_pause_menu,
        feedback_subscreen: screen.feedback_subscreen,
        return_to_game_hovered: screen.return_to_game_hovered,
        advancements_hovered: screen.advancements_hovered,
        stats_hovered: screen.stats_hovered,
        send_feedback_hovered: screen.send_feedback_hovered,
        report_bugs_hovered: screen.report_bugs_hovered,
        report_bugs_enabled: screen.report_bugs_enabled,
        custom_dialog_label,
        custom_dialog_hovered: screen.custom_dialog_hovered,
        custom_dialog_tooltip,
        feedback_back_hovered: screen.feedback_back_hovered,
        options_hovered: screen.options_hovered,
        options_enabled: screen.options_enabled,
        options_tooltip,
        disconnect_hovered: screen.disconnect_hovered,
        disconnect_enabled: screen.disconnect_enabled,
    })
}

pub(super) fn sanitize_hud_stats_screen(screen: HudStatsScreen) -> Option<HudStatsScreen> {
    let title = sanitize_hud_text_line(screen.title)?;
    let loading_text = sanitize_hud_text_line(screen.loading_text)?;
    let general_tab = sanitize_hud_text_line(screen.general_tab)?;
    let items_tab = sanitize_hud_text_line(screen.items_tab)?;
    let mobs_tab = sanitize_hud_text_line(screen.mobs_tab)?;
    let item_column_labels = screen
        .item_column_labels
        .into_iter()
        .map(sanitize_hud_text_line)
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()?;
    let item_no_value = sanitize_hud_text_line(screen.item_no_value)?;
    let done = sanitize_hud_label_or_default(screen.done, "Done");
    let tooltip = screen.tooltip.and_then(sanitize_hud_debug_options_tooltip);
    let visible_rows = screen.visible_rows.min(64);
    let max_scroll_amount = screen.max_scroll_amount.clamp(0, 1_000_000);
    let scroll_amount = screen.scroll_amount.clamp(0, max_scroll_amount);
    let scroller_height = if max_scroll_amount > 0 {
        screen.scroller_height.clamp(1, 4096)
    } else {
        0
    };
    let mut rows = screen.rows;
    retain_map_in_place_bounded(&mut rows, visible_rows, |row| {
        Some(HudStatsRow {
            label: sanitize_hud_text_line(row.label)?,
            value: sanitize_hud_text_line(row.value)?,
            secondary_value: row.secondary_value.and_then(sanitize_hud_text_line),
        })
    });
    let mut item_rows = screen.item_rows;
    retain_map_in_place_bounded(&mut item_rows, visible_rows, |row| {
        Some(HudStatsItemRow {
            item_id: row.item_id,
            resource_id: sanitize_hud_text_line(row.resource_id)?,
            label: sanitize_hud_text_line(row.label)?,
            values: row
                .values
                .map(|value| value.and_then(sanitize_hud_text_line)),
            icon: row.icon.and_then(sanitize_hud_item_icon),
            block_model: row.block_model.filter(hud_block_item_model_is_renderable),
        })
    });
    (!title.is_empty() && !loading_text.is_empty()).then_some(HudStatsScreen {
        title,
        loading_text,
        is_loading: screen.is_loading,
        general_tab,
        items_tab,
        mobs_tab,
        active_tab: screen.active_tab.min(2),
        items_tab_enabled: screen.items_tab_enabled,
        mobs_tab_enabled: screen.mobs_tab_enabled,
        rows,
        item_rows,
        item_column_labels,
        item_no_value,
        item_sort_column: screen.item_sort_column.filter(|column| *column < 6),
        item_sort_order: screen.item_sort_order.clamp(-1, 1),
        item_hovered_column: screen.item_hovered_column.filter(|column| *column < 6),
        item_focused_column: screen.item_focused_column.filter(|column| *column < 6),
        scroll_row: screen.scroll_row.min(screen.total_rows),
        scroll_amount,
        max_scroll_amount,
        scroller_height,
        total_rows: screen.total_rows,
        visible_rows,
        row_height: screen.row_height.clamp(9, 64),
        tooltip,
        done,
        done_hovered: screen.done_hovered,
        done_focused: screen.done_focused,
    })
}

pub(super) fn sanitize_hud_system_toast(toast: HudSystemToast) -> Option<HudSystemToast> {
    if !toast.visible_portion.is_finite() {
        return None;
    }
    let title = sanitize_hud_text_line(toast.title)?;
    let mut message_lines = toast.message_lines;
    message_lines.truncate(256);
    retain_map_in_place(&mut message_lines, sanitize_hud_text_line);
    (!title.is_empty()).then_some(HudSystemToast {
        title,
        message_lines,
        width: toast.width.clamp(1, 4_096),
        visible_portion: toast.visible_portion.clamp(0.0, 1.0),
    })
}

pub(super) fn sanitize_hud_system_toast_instance(
    instance: HudSystemToastInstance,
) -> Option<HudSystemToastInstance> {
    let occupied = instance
        .toast
        .message_lines
        .len()
        .max(1)
        .saturating_mul(12)
        .saturating_add(20)
        .div_ceil(32);
    if usize::from(instance.slot).saturating_add(occupied) > HUD_TOAST_SLOTS {
        return None;
    }
    Some(HudSystemToastInstance {
        toast: sanitize_hud_system_toast(instance.toast)?,
        slot: instance.slot,
    })
}

pub(super) fn sanitize_hud_toast(
    toast: HudToast,
    occupied: &mut [bool; HUD_TOAST_SLOTS],
) -> Option<HudToast> {
    let toast = match toast {
        HudToast::System(toast) => HudToast::System(sanitize_hud_system_toast_instance(toast)?),
        HudToast::Advancement(toast) => {
            HudToast::Advancement(sanitize_hud_advancement_toast(toast)?)
        }
        HudToast::Recipe(toast) => HudToast::Recipe(sanitize_hud_recipe_toast(toast)?),
        HudToast::Tutorial(toast) => HudToast::Tutorial(sanitize_hud_tutorial_toast(toast)?),
    };
    let (slot, count) = match &toast {
        HudToast::System(toast) => (
            usize::from(toast.slot),
            toast
                .toast
                .message_lines
                .len()
                .max(1)
                .saturating_mul(12)
                .saturating_add(20)
                .div_ceil(32),
        ),
        HudToast::Advancement(toast) => (usize::from(toast.slot), 1),
        HudToast::Recipe(toast) => (usize::from(toast.slot), 1),
        HudToast::Tutorial(toast) => (usize::from(toast.slot), toast.lines.occupied_slots()),
    };
    let end = slot.checked_add(count)?;
    if end > HUD_TOAST_SLOTS || occupied[slot..end].iter().any(|value| *value) {
        return None;
    }
    occupied[slot..end].fill(true);
    Some(toast)
}

pub(super) fn sanitize_hud_now_playing_toast(
    toast: HudNowPlayingToast,
) -> Option<HudNowPlayingToast> {
    if !toast.visible_portion.is_finite() {
        return None;
    }
    let text = sanitize_hud_text_line(toast.text)?;
    (!text.is_empty()).then_some(HudNowPlayingToast {
        text,
        visible_portion: toast.visible_portion.clamp(0.0, 1.0),
        note_color_argb: toast.note_color_argb,
        client_tick: toast.client_tick,
        placement: toast.placement,
    })
}

pub(super) fn sanitize_hud_advancement_toast(
    toast: HudAdvancementToast,
) -> Option<HudAdvancementToast> {
    if !toast.visible_portion.is_finite() || usize::from(toast.slot) >= HUD_TOAST_SLOTS {
        return None;
    }
    let frame_label = sanitize_hud_text_line(toast.frame_label)?;
    let title_runs = sanitize_hud_multiline_styled_runs(toast.title_runs, "");
    let icon = sanitize_hud_item_icon(toast.icon)?;
    (!frame_label.is_empty()).then_some(HudAdvancementToast {
        frame_label,
        frame_label_color: toast.frame_label_color & 0xFF_FF_FF,
        title_runs,
        icon,
        block_model: toast.block_model,
        slot: toast.slot,
        visible_portion: toast.visible_portion.clamp(0.0, 1.0),
        fully_visible_ms: toast.fully_visible_ms,
    })
}

pub(super) fn sanitize_hud_recipe_toast(toast: HudRecipeToast) -> Option<HudRecipeToast> {
    if !toast.visible_portion.is_finite() || usize::from(toast.slot) >= HUD_TOAST_SLOTS {
        return None;
    }
    let title = sanitize_hud_text_line(toast.title)?;
    let description = sanitize_hud_text_line(toast.description)?;
    Some(HudRecipeToast {
        title,
        description,
        category_icon: toast.category_icon.and_then(sanitize_hud_item_icon),
        category_block_model: toast.category_block_model,
        unlocked_icon: toast.unlocked_icon.and_then(sanitize_hud_item_icon),
        unlocked_block_model: toast.unlocked_block_model,
        slot: toast.slot,
        visible_portion: toast.visible_portion.clamp(0.0, 1.0),
    })
}

pub(super) fn sanitize_hud_tutorial_toast(toast: HudTutorialToast) -> Option<HudTutorialToast> {
    if !toast.visible_portion.is_finite()
        || toast.progress.is_some_and(|progress| !progress.is_finite())
        || usize::from(toast.slot) >= HUD_TOAST_SLOTS
    {
        return None;
    }
    if usize::from(toast.slot).saturating_add(toast.lines.occupied_slots()) > HUD_TOAST_SLOTS {
        return None;
    }
    Some(HudTutorialToast {
        visible_portion: toast.visible_portion.clamp(0.0, 1.0),
        progress: toast.progress.map(|progress| progress.clamp(0.0, 1.0)),
        ..toast
    })
}

pub(super) fn sanitize_hud_tutorial_toast_lines(
    mut lines: Vec<Vec<HudStyledTextRun>>,
) -> Vec<Vec<HudStyledTextRun>> {
    lines.truncate(64);
    for runs in &mut lines {
        *runs = sanitize_hud_styled_runs(std::mem::take(runs), "");
    }
    lines
}

pub(super) fn sanitize_hud_debug_options_screen(
    screen: HudDebugOptionsScreen,
) -> Option<HudDebugOptionsScreen> {
    let title = sanitize_hud_text_line(screen.title)?;
    let warning = sanitize_hud_text_line(screen.warning)?;
    let search_hint = sanitize_hud_text_line(screen.search_hint)?;
    let never_label = sanitize_hud_text_line(screen.never_label)?;
    let overlay_label = sanitize_hud_text_line(screen.overlay_label)?;
    let always_label = sanitize_hud_text_line(screen.always_label)?;
    let default_profile_label = sanitize_hud_text_line(screen.default_profile_label)?;
    let performance_profile_label = sanitize_hud_text_line(screen.performance_profile_label)?;
    let done_label = sanitize_hud_text_line(screen.done_label)?;
    let search = sanitize_hud_text_field(screen.search, 32);
    let visible_rows = screen.visible_rows.min(64);
    let mut rows = screen.rows;
    retain_map_in_place_bounded(&mut rows, visible_rows, sanitize_hud_debug_options_row);
    let tooltip = screen.tooltip.and_then(sanitize_hud_debug_options_tooltip);
    (!title.is_empty()).then_some(HudDebugOptionsScreen {
        title,
        warning,
        search,
        search_hint,
        never_label,
        overlay_label,
        always_label,
        rows,
        tooltip,
        scroll_row: screen.scroll_row.min(screen.total_rows),
        total_rows: screen.total_rows.min(256),
        visible_rows,
        default_profile_active: screen.default_profile_active,
        default_profile_hovered: screen.default_profile_hovered,
        default_profile_focused: screen.default_profile_focused,
        default_profile_label,
        performance_profile_active: screen.performance_profile_active,
        performance_profile_hovered: screen.performance_profile_hovered,
        performance_profile_focused: screen.performance_profile_focused,
        performance_profile_label,
        done_hovered: screen.done_hovered,
        done_focused: screen.done_focused,
        done_label,
    })
}

fn sanitize_hud_debug_options_row(row: HudDebugOptionsRow) -> Option<HudDebugOptionsRow> {
    match row {
        HudDebugOptionsRow::Category { label } => {
            let label = sanitize_hud_text_line(label)?;
            Some(HudDebugOptionsRow::Category { label })
        }
        HudDebugOptionsRow::Entry {
            path,
            status,
            hovered_status,
            focused_status,
            allowed,
        } => {
            let path = sanitize_hud_text_line(path)?;
            Some(HudDebugOptionsRow::Entry {
                path,
                status,
                hovered_status,
                focused_status,
                allowed,
            })
        }
    }
}

pub(super) fn sanitize_hud_debug_options_tooltip(
    tooltip: HudDebugOptionsTooltip,
) -> Option<HudDebugOptionsTooltip> {
    let text = sanitize_hud_text_line(tooltip.text)?;
    (!text.is_empty()).then_some(HudDebugOptionsTooltip {
        text,
        x: tooltip.x,
        y: tooltip.y,
    })
}

fn sanitize_hud_debug_game_mode_switcher_slot(
    slot: HudDebugGameModeSwitcherSlot,
) -> Option<HudDebugGameModeSwitcherSlot> {
    (slot.width > 0 && slot.height > 0).then_some(HudDebugGameModeSwitcherSlot {
        mode: slot.mode,
        x: slot.x,
        y: slot.y,
        width: slot.width,
        height: slot.height,
        selected: slot.selected,
        icon: slot.icon.and_then(sanitize_hud_item_icon),
        block_model: slot.block_model.filter(hud_block_item_model_is_renderable),
    })
}

fn sanitize_hud_debug_crosshair(crosshair: HudDebugCrosshair) -> Option<HudDebugCrosshair> {
    (crosshair.x_rot_degrees.is_finite() && crosshair.y_rot_degrees.is_finite()).then_some(
        HudDebugCrosshair {
            gui_scale: crosshair.gui_scale.max(1),
            ..crosshair
        },
    )
}

fn sanitize_hud_debug_profiler_chart(mut chart: HudDebugProfilerChart) -> HudDebugProfilerChart {
    chart.current_node_name = sanitize_hud_text_preserving_empty(chart.current_node_name, 128);
    if !chart.current_global_percentage.is_finite() {
        chart.current_global_percentage = 0.0;
    }
    chart.current_global_percentage = chart.current_global_percentage.clamp(0.0, 100.0);
    retain_map_in_place(&mut chart.slices, sanitize_hud_debug_profiler_slice);
    chart.slices.truncate(HUD_DEBUG_PROFILER_SLICE_CAPACITY);
    chart
}

fn sanitize_hud_debug_profiler_slice(
    mut slice: HudDebugProfilerSlice,
) -> Option<HudDebugProfilerSlice> {
    if !slice.percentage.is_finite() || !slice.global_percentage.is_finite() {
        return None;
    }
    slice.name = sanitize_hud_text_preserving_empty(slice.name, 128);
    slice.percentage = slice.percentage.clamp(0.0, 100.0);
    slice.global_percentage = slice.global_percentage.clamp(0.0, 100.0);
    Some(slice)
}

fn sanitize_hud_debug_fps_chart(mut chart: HudDebugFrameTimeChart) -> HudDebugFrameTimeChart {
    if chart.frame_time_nanos.len() > HUD_DEBUG_CHART_SAMPLE_CAPACITY {
        let keep_from = chart.frame_time_nanos.len() - HUD_DEBUG_CHART_SAMPLE_CAPACITY;
        chart.frame_time_nanos.drain(..keep_from);
    }
    if !matches!(chart.configured_framerate_limit, Some(1..=250)) {
        chart.configured_framerate_limit = None;
    }
    chart
}

fn sanitize_hud_debug_tps_chart(mut chart: HudDebugTpsChart) -> HudDebugTpsChart {
    if chart.samples.len() > HUD_DEBUG_CHART_SAMPLE_CAPACITY {
        let keep_from = chart.samples.len() - HUD_DEBUG_CHART_SAMPLE_CAPACITY;
        chart.samples.drain(..keep_from);
    }
    if !chart.milliseconds_per_tick.is_finite() || chart.milliseconds_per_tick <= 0.0 {
        chart.milliseconds_per_tick = 50.0;
    }
    chart
}

fn sanitize_hud_debug_network_charts(mut charts: HudDebugNetworkCharts) -> HudDebugNetworkCharts {
    if charts.ping_millis.len() > HUD_DEBUG_CHART_SAMPLE_CAPACITY {
        let keep_from = charts.ping_millis.len() - HUD_DEBUG_CHART_SAMPLE_CAPACITY;
        charts.ping_millis.drain(..keep_from);
    }
    if charts.bandwidth_bytes_per_tick.len() > HUD_DEBUG_CHART_SAMPLE_CAPACITY {
        let keep_from = charts.bandwidth_bytes_per_tick.len() - HUD_DEBUG_CHART_SAMPLE_CAPACITY;
        charts.bandwidth_bytes_per_tick.drain(..keep_from);
    }
    charts
}

fn sanitize_hud_debug_overlay_lines(mut lines: Vec<String>) -> Vec<String> {
    lines.truncate(64);
    for line in &mut lines {
        *line = sanitize_hud_text_preserving_empty(std::mem::take(line), 256);
    }
    lines
}

pub(super) fn sanitize_hud_text_line(line: String) -> Option<String> {
    let line = sanitize_hud_text_preserving_empty(line, 256);
    (!line.is_empty()).then_some(line)
}

pub(super) fn sanitize_hud_text_preserving_empty(line: String, limit: usize) -> String {
    sanitize_hud_chars_with_count(line, limit, false).0
}

pub(super) fn sanitize_hud_chars_with_count(
    mut line: String,
    limit: usize,
    allow_newline: bool,
) -> (String, usize) {
    if limit == 0 {
        line.clear();
        return (line, 0);
    }
    let mut count = 0usize;
    let mut truncate_at = line.len();
    let mut dirty = false;
    for (idx, ch) in line.char_indices() {
        if count == limit {
            truncate_at = idx;
            break;
        }
        if (allow_newline && ch == '\n') || !ch.is_control() {
            count += 1;
        } else {
            dirty = true;
            break;
        }
    }
    if !dirty {
        line.truncate(truncate_at);
        return (line, count);
    }
    let mut sanitized = String::with_capacity(line.len().min(limit.saturating_mul(4)));
    count = 0;
    for ch in line
        .chars()
        .filter(|ch| (allow_newline && *ch == '\n') || !ch.is_control())
        .take(limit)
    {
        sanitized.push(ch);
        count += 1;
    }
    (sanitized, count)
}

fn sanitize_hud_utf16_with_units(
    mut line: String,
    limit: usize,
    allow_newline: bool,
) -> (String, usize) {
    if limit == 0 {
        line.clear();
        return (line, 0);
    }
    let mut used = 0usize;
    let mut truncate_at = line.len();
    let mut dirty = false;
    for (idx, ch) in line.char_indices() {
        if (allow_newline && ch == '\n') || !ch.is_control() {
            let len = ch.len_utf16();
            if used.saturating_add(len) > limit {
                truncate_at = idx;
                break;
            }
            used += len;
        } else {
            dirty = true;
            break;
        }
    }
    if !dirty {
        line.truncate(truncate_at);
        return (line, used);
    }
    let mut sanitized = String::with_capacity(line.len());
    used = 0;
    for ch in line
        .chars()
        .filter(|ch| (allow_newline && *ch == '\n') || !ch.is_control())
    {
        let len = ch.len_utf16();
        if used.saturating_add(len) > limit {
            break;
        }
        sanitized.push(ch);
        used += len;
    }
    (sanitized, used)
}

fn sanitize_hud_text_preserving_empty_utf16(line: String, limit: usize) -> String {
    sanitize_hud_utf16_with_units(line, limit, false).0
}

fn sanitize_hud_multiline_text(line: String, limit: usize) -> String {
    sanitize_hud_utf16_with_units(line, limit, true).0
}

/// Keeps the parse ranges inside the value they describe and in the
/// non-decreasing order `formatText` reads them in; anything else would only
/// colour units twice.
fn sanitize_hud_command_input_syntax(
    mut syntax: HudCommandInputSyntax,
    len: usize,
) -> HudCommandInputSyntax {
    let mut previous_end = 0usize;
    syntax.arguments.retain_mut(|argument| {
        let end = argument.end.min(len);
        if argument.start >= previous_end && end > argument.start {
            argument.end = end;
            previous_end = end;
            true
        } else {
            false
        }
    });
    syntax.unread = syntax.unread.and_then(|unread| {
        let end = unread.end.min(len);
        (unread.start >= previous_end && end > unread.start).then_some(HudCommandInputRange {
            start: unread.start,
            end,
        })
    });
    syntax
}

fn utf16_equals_str(utf16: &[u16], text: &str) -> bool {
    text.encode_utf16().eq(utf16.iter().copied())
}

pub(super) fn sanitize_hud_command_suggestions(
    mut state: HudCommandSuggestions,
) -> HudCommandSuggestions {
    state.preedit = state.preedit.and_then(sanitize_hud_ime_preedit);
    let (max_length, max_entries, max_list_height) = match state.layout {
        HudCommandSuggestionsLayout::Chat => (256, 10, 120),
        HudCommandSuggestionsLayout::CommandBlock => (32_500, 7, 84),
    };
    if state.layout == HudCommandSuggestionsLayout::CommandBlock {
        state.input_view = None;
        state.draft = false;
        state.syntax = None;
    }
    if let Some(view) = &mut state.input_view {
        view.value.truncate(256);
        view.cursor = view.cursor.min(view.value.len());
        view.selection = view.selection.min(view.value.len());
        view.display_start = view.display_start.min(view.value.len());
        if !utf16_equals_str(&view.value, &state.input_text) {
            state.input_text = String::from_utf16_lossy(&view.value);
        }
        state.cursor = view.cursor;
        state.selection = view.selection;
        let len = view.value.len();
        state.syntax = state
            .syntax
            .take()
            .map(|syntax| sanitize_hud_command_input_syntax(syntax, len));
    } else {
        // Without the raw value the ranges describe nothing.
        state.syntax = None;
        state.input_text = sanitize_hud_text_preserving_empty_utf16(state.input_text, max_length);
        let text_len = state.input_text.chars().count();
        state.cursor = state.cursor.min(text_len);
        state.selection = state.selection.min(text_len);
    }
    state.suffix = state
        .suffix
        .map(|suffix| sanitize_hud_text_preserving_empty_utf16(suffix, max_length))
        .filter(|suffix| !suffix.is_empty());
    state.list_width = state.list_width.min(4096);
    state.list_height = state.list_height.min(max_list_height);
    state.entries.truncate(max_entries);
    state.entries.retain_mut(|entry| {
        // No fallback run: an entry that sanitizes away must drop, not
        // come back as an empty row (`sanitize_hud_text_line`'s rule).
        let mut runs = sanitize_hud_styled_runs(std::mem::take(&mut entry.runs), "");
        runs.retain(|run| !run.text.is_empty());
        let keep = !runs.is_empty();
        entry.runs = runs;
        keep
    });
    state.tooltip = state.tooltip.and_then(|tooltip| {
        let mut runs = sanitize_hud_styled_runs(tooltip.runs, "");
        runs.retain(|run| !run.text.is_empty());
        (!runs.is_empty()).then_some(HudCommandSuggestionTooltip {
            runs,
            x: tooltip.x,
            y: tooltip.y,
        })
    });
    state
}

pub(super) fn sanitize_hud_chat_message(mut message: HudChatMessage) -> HudChatMessage {
    let mut message_budget = 16_384;
    message.runs = sanitize_hud_component_runs(message.runs, "", &mut message_budget, true);
    let char_count = message
        .runs
        .iter()
        .map(|run| run.visual.text.chars().count())
        .sum();
    message.interactions.retain_mut(|interaction| {
        interaction.start_char = interaction.start_char.min(char_count);
        interaction.end_char = interaction.end_char.min(char_count);
        interaction.tooltip = interaction
            .tooltip
            .take()
            .and_then(sanitize_hud_rich_tooltip);
        interaction.start_char < interaction.end_char
    });
    if let Some(tag) = message.tag.as_mut() {
        tag.tooltip_runs = sanitize_hud_chat_runs(std::mem::take(&mut tag.tooltip_runs), "");
    }
    message
}

pub(super) fn sanitize_hud_rich_tooltip(mut tooltip: HudRichTooltip) -> Option<HudRichTooltip> {
    tooltip.style = tooltip
        .style
        .map(|style| sanitize_hud_text_preserving_empty(style, 256))
        .filter(|style| !style.is_empty());
    let mut budget = 8_192usize;
    tooltip.lines = sanitize_hud_rich_tooltip_lines(tooltip.lines, &mut budget);
    tooltip.advanced_lines = tooltip
        .advanced_lines
        .map(|lines| sanitize_hud_rich_tooltip_lines(lines, &mut budget));
    let has_lines = !tooltip.lines.is_empty()
        || tooltip
            .advanced_lines
            .as_ref()
            .is_some_and(|lines| !lines.is_empty());
    has_lines.then_some(tooltip)
}

fn sanitize_hud_rich_tooltip_lines(
    mut lines: Vec<HudRichTooltipLine>,
    budget: &mut usize,
) -> Vec<HudRichTooltipLine> {
    lines.truncate(64);
    for line in &mut lines {
        line.tint = if line.tint.iter().all(|channel| channel.is_finite()) {
            line.tint.map(|channel| channel.clamp(0.0, 1.0))
        } else {
            HUD_TINT_WHITE
        };
        line.runs = sanitize_hud_component_runs(std::mem::take(&mut line.runs), "", budget, true);
    }
    lines
}

pub(super) fn sanitize_hud_component_runs(
    mut runs: Vec<HudComponentTextRun>,
    fallback_text: &str,
    budget: &mut usize,
    allow_newline: bool,
) -> Vec<HudComponentTextRun> {
    runs.retain_mut(|run| {
        if *budget == 0 {
            return false;
        }
        let (text, used) = sanitize_hud_utf16_with_units(
            std::mem::take(&mut run.visual.text),
            *budget,
            allow_newline,
        );
        *budget -= used;
        if text.is_empty() {
            return false;
        }
        run.visual.text = text;
        run.visual.color = run.visual.color.map(|color| color & 0xFF_FF_FF);
        run.font = run
            .font
            .take()
            .map(|font| sanitize_hud_text_preserving_empty(font, 256))
            .filter(|font| !font.is_empty());
        true
    });
    if runs.is_empty() && !fallback_text.is_empty() && *budget > 0 {
        let (fallback, used) =
            sanitize_hud_utf16_with_units(fallback_text.to_string(), *budget, true);
        *budget = (*budget).saturating_sub(used);
        runs.push(HudComponentTextRun::plain(fallback));
    }
    runs
}

fn sanitize_hud_chat_runs(
    mut runs: Vec<HudStyledTextRun>,
    fallback_text: &str,
) -> Vec<HudStyledTextRun> {
    let mut budget = 16_384usize;
    runs.retain_mut(|run| {
        if budget == 0 {
            return false;
        }
        let (text, used) =
            sanitize_hud_utf16_with_units(std::mem::take(&mut run.text), budget, true);
        budget -= used;
        if text.is_empty() {
            return false;
        }
        run.text = text;
        run.color = run.color.map(|color| color & 0xFF_FF_FF);
        true
    });
    if runs.is_empty() && !fallback_text.is_empty() {
        runs.push(HudStyledTextRun::plain(fallback_text));
    }
    runs
}

/// Sanitizes a line's styled runs under the same rules as
/// [`sanitize_hud_text_line`] (control characters stripped, 256-char budget
/// across the line) and clamps run colours to `0xRRGGBB`. Empty run lists —
/// plain producers — synthesize a single default-style run from the already
/// sanitized `fallback_text`, so the draw loops always consume runs.
pub(super) fn sanitize_hud_styled_runs(
    mut runs: Vec<HudStyledTextRun>,
    fallback_text: &str,
) -> Vec<HudStyledTextRun> {
    let mut budget = 256usize;
    runs.retain_mut(|run| {
        if budget == 0 {
            return false;
        }
        let (text, count) =
            sanitize_hud_chars_with_count(std::mem::take(&mut run.text), budget, false);
        if text.is_empty() {
            return false;
        }
        budget -= count;
        run.text = text;
        run.color = run.color.map(|color| color & 0xFF_FF_FF);
        true
    });
    if runs.is_empty() {
        runs.push(HudStyledTextRun::plain(fallback_text));
    }
    runs
}

fn sanitize_hud_multiline_styled_runs(
    mut runs: Vec<HudStyledTextRun>,
    fallback_text: &str,
) -> Vec<HudStyledTextRun> {
    let mut budget = 256usize;
    runs.retain_mut(|run| {
        if budget == 0 {
            return false;
        }
        let (text, count) =
            sanitize_hud_chars_with_count(std::mem::take(&mut run.text), budget, true);
        if text.is_empty() {
            return false;
        }
        budget -= count;
        run.text = text;
        run.color = run.color.map(|color| color & 0xFF_FF_FF);
        true
    });
    if runs.is_empty() {
        runs.push(HudStyledTextRun::plain(fallback_text));
    }
    runs
}

pub(super) fn sanitize_hud_item_icon(icon: HudItemIcon) -> Option<HudItemIcon> {
    if icon.lighting != GuiItemLightingEntry::ItemsFlat {
        return None;
    }
    let mut layers = icon.layers;
    retain_map_in_place(&mut layers, sanitize_hud_icon_layer);
    (!layers.is_empty()).then_some(HudItemIcon {
        pop: if icon.pop.is_finite() {
            icon.pop.clamp(0.0, VANILLA_ITEM_POP_TIME_TICKS)
        } else {
            0.0
        },
        lighting: GuiItemLightingEntry::ItemsFlat,
        layers,
        foil: icon.foil,
        count_label: icon.count_label.and_then(sanitize_hud_item_count_label),
        durability_bar: icon
            .durability_bar
            .and_then(sanitize_hud_item_durability_bar),
        cooldown_progress: sanitize_hud_item_cooldown_progress(icon.cooldown_progress),
    })
}

fn sanitize_hud_icon_layer(layer: HudIconLayer) -> Option<HudIconLayer> {
    if !layer.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    Some(HudIconLayer {
        uv: sanitize_hud_uv_rect(layer.uv)?,
        tint: layer.tint.map(|component| component.clamp(0.0, 1.0)),
    })
}

fn sanitize_hud_item_count_label(label: HudItemCountLabel) -> Option<HudItemCountLabel> {
    let text = label.text;
    (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
        .then_some(HudItemCountLabel { text })
}

fn sanitize_hud_item_durability_bar(bar: HudItemDurabilityBar) -> Option<HudItemDurabilityBar> {
    if !bar.color.iter().all(|component| component.is_finite()) {
        return None;
    }
    Some(HudItemDurabilityBar {
        width: bar.width.min(HUD_ITEM_BAR_BACKGROUND_WIDTH),
        color: bar.color.map(|component| component.clamp(0.0, 1.0)),
    })
}

fn sanitize_hud_item_cooldown_progress(progress: Option<f32>) -> Option<f32> {
    let progress = progress?;
    progress.is_finite().then_some(progress.clamp(0.0, 1.0))
}

pub(super) fn retain_map_in_place<T>(items: &mut Vec<T>, f: impl FnMut(T) -> Option<T>) {
    retain_map_in_place_bounded(items, usize::MAX, f);
}

pub(super) fn retain_map_in_place_bounded<T>(
    items: &mut Vec<T>,
    max_keep: usize,
    mut f: impl FnMut(T) -> Option<T>,
) {
    struct Guard<'a, T> {
        items: &'a mut Vec<T>,
        ptr: *mut T,
        read: usize,
        write: usize,
        len: usize,
    }
    impl<T> Drop for Guard<'_, T> {
        fn drop(&mut self) {
            unsafe {
                for idx in self.read..self.len {
                    self.ptr.add(idx).drop_in_place();
                }
                self.items.set_len(self.write);
            }
        }
    }

    let len = items.len();
    let ptr = items.as_mut_ptr();
    unsafe {
        items.set_len(0);
    }
    let mut guard = Guard {
        items,
        ptr,
        read: 0,
        write: 0,
        len,
    };
    while guard.read < guard.len && guard.write < max_keep {
        let item = unsafe { guard.ptr.add(guard.read).read() };
        guard.read += 1;
        if let Some(mapped) = f(item) {
            unsafe {
                guard.ptr.add(guard.write).write(mapped);
            }
            guard.write += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_hud_strings_and_runs_sanitize_in_place_without_reallocating() {
        let text = String::from("Speed II (0:45)");
        let text_ptr = text.as_ptr();
        let sanitized = sanitize_hud_text_preserving_empty(text, 256);
        assert_eq!(sanitized.as_ptr(), text_ptr);
        assert_eq!(sanitized, "Speed II (0:45)");

        let overlong = String::from("abcdef");
        let overlong_ptr = overlong.as_ptr();
        let truncated = sanitize_hud_text_preserving_empty(overlong, 4);
        assert_eq!(truncated.as_ptr(), overlong_ptr);
        assert_eq!(truncated, "abcd");

        let dirty = String::from("ab\0cd\nef");
        assert_eq!(
            sanitize_hud_text_preserving_empty(dirty.clone(), 256),
            "abcdef"
        );
        assert_eq!(sanitize_hud_multiline_text(dirty, 256), "abcd\nef");

        let run_text = String::from("Diamond Sword");
        let run_text_ptr = run_text.as_ptr();
        let mut runs = Vec::with_capacity(4);
        runs.push(HudStyledTextRun {
            shadow_color: None,
            text: run_text,
            style: HudTextStyle::default(),
            color: Some(0xFF_55_FF_55),
        });
        let runs_ptr = runs.as_ptr();
        let sanitized_runs = sanitize_hud_styled_runs(runs, "fallback");
        assert_eq!(sanitized_runs.as_ptr(), runs_ptr);
        assert_eq!(sanitized_runs[0].text.as_ptr(), run_text_ptr);
        assert_eq!(sanitized_runs[0].color, Some(0x55_FF_55));
    }
}
