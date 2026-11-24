use bbb_protocol::{
    packets::{
        data_components::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH, DataComponentPatchSummary,
        ItemStackSummary,
    },
    ComponentHoverEvent, NbtSummaryValue, RichTextComponent, StyledTextRun,
};

pub(super) fn upgrade(stack: &mut ItemStackSummary) -> Result<(), String> {
    if !stack.component_patch.is_allocated() {
        return Ok(());
    }
    let mut pending: Vec<(&mut DataComponentPatchSummary, usize)> =
        vec![(&mut stack.component_patch, 0)];
    while let Some((patch, item_depth)) = pending.pop() {
        if item_depth > MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
            return Err("legacy hotbar item exceeds component nesting limit".into());
        }
        upgrade_patch(patch)?;
        let nested = patch
            .use_remainder
            .iter_mut()
            .map(|item| item.as_mut())
            .chain(patch.charged_projectiles_items.iter_mut())
            .chain(patch.bundle_contents_items.iter_mut())
            .chain(patch.container.slots.iter_mut().map(|slot| &mut slot.item));
        for item in nested {
            if item_depth + 1 > MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
                return Err("legacy hotbar item exceeds component nesting limit".into());
            }
            if item.component_patch.is_allocated() {
                pending.push((&mut item.component_patch, item_depth + 1));
            }
        }
        for predicate in [
            &mut patch.can_place_on_persistent,
            &mut patch.can_break_persistent,
        ]
        .into_iter()
        .flatten()
        .flat_map(|value| &mut value.predicates)
        {
            if let Some(exact) = &mut predicate.exact_components {
                // Template depth restarts inside exact components; the canonical
                // encoder separately validates adventure-predicate depth.
                pending.push((exact.as_mut(), 0));
            }
        }
    }
    Ok(())
}

fn upgrade_patch(patch: &mut DataComponentPatchSummary) -> Result<(), String> {
    upgrade_optional(
        &mut patch.custom_name_styled,
        patch.custom_name_nbt.as_ref(),
    )?;
    upgrade_optional(&mut patch.item_name_styled, patch.item_name_nbt.as_ref())?;
    for (runs, raw) in patch.lore_styled.iter_mut().zip(&patch.lore_nbt) {
        upgrade_runs(runs, raw)?;
    }
    for attribute in &mut patch.attribute_modifiers {
        upgrade_optional(
            &mut attribute.display_text_styled,
            attribute.display_text_nbt.as_ref(),
        )?;
    }
    for material in [
        &mut patch.armor_trim_material_direct,
        &mut patch.provides_trim_material_direct,
    ]
    .into_iter()
    .flatten()
    {
        upgrade_boxed(
            &mut material.description_styled,
            material.description_nbt.as_deref(),
        )?;
    }
    if let Some(pattern) = &mut patch.armor_trim_pattern_direct {
        upgrade_boxed(
            &mut pattern.description_styled,
            pattern.description_nbt.as_deref(),
        )?;
    }
    if let Some(instrument) = &mut patch.instrument.direct {
        upgrade_boxed(
            &mut instrument.description_styled,
            instrument.description_nbt.as_deref(),
        )?;
    }
    if let Some(song) = &mut patch.jukebox_direct_song {
        upgrade_boxed(
            &mut song.description_styled,
            song.description_nbt.as_deref(),
        )?;
    }
    if let Some(painting) = &mut patch.painting_variant_direct {
        upgrade_optional(&mut painting.title_styled, painting.title_nbt.as_deref())?;
        upgrade_optional(&mut painting.author_styled, painting.author_nbt.as_deref())?;
    }
    Ok(())
}

fn upgrade_optional(
    runs: &mut Option<Vec<StyledTextRun>>,
    raw: Option<&NbtSummaryValue>,
) -> Result<(), String> {
    if let (Some(runs), Some(raw)) = (runs, raw) {
        upgrade_runs(runs, raw)?;
    }
    Ok(())
}

fn upgrade_boxed(
    runs: &mut Option<Box<[StyledTextRun]>>,
    raw: Option<&NbtSummaryValue>,
) -> Result<(), String> {
    if let (Some(runs), Some(raw)) = (runs, raw) {
        if let Some(upgraded) = upgraded_runs(runs, raw)? {
            *runs = upgraded.into_boxed_slice();
        }
    }
    Ok(())
}

fn upgrade_runs(runs: &mut Vec<StyledTextRun>, raw: &NbtSummaryValue) -> Result<(), String> {
    if let Some(upgraded) = upgraded_runs(runs, raw)? {
        *runs = upgraded;
    }
    Ok(())
}

fn upgraded_runs(
    runs: &[StyledTextRun],
    raw: &NbtSummaryValue,
) -> Result<Option<Vec<StyledTextRun>>, String> {
    let canonical = RichTextComponent::try_from_component_nbt(raw.clone())?;
    if runs == canonical.styled() {
        return Ok(None);
    }
    // Only the exact pre-shadow projection can cross this import boundary.
    if runs != without_shadow(canonical.styled().to_vec()) {
        return Err("legacy hotbar text projection does not match component NBT".into());
    }
    Ok(Some(canonical.styled().to_vec()))
}

fn without_shadow(runs: Vec<StyledTextRun>) -> Vec<StyledTextRun> {
    let mut merged: Vec<StyledTextRun> = Vec::new();
    // Input comes from the depth-validated canonical Component decoder.
    for mut run in runs {
        run.style.shadow_color = None;
        match &mut run.style.hover_event {
            Some(ComponentHoverEvent::ShowText { value, .. }) => {
                *value = without_shadow(std::mem::take(value));
            }
            Some(ComponentHoverEvent::ShowEntity {
                name: Some(name), ..
            }) => {
                *name = without_shadow(std::mem::take(name));
            }
            _ => {}
        }
        if run.text.is_empty() {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if last.style == run.style {
                last.text.push_str(&run.text);
                continue;
            }
        }
        merged.push(run);
    }
    merged
}

#[cfg(test)]
mod tests;
