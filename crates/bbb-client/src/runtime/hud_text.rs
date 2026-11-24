use super::*;

pub(super) fn localized_gui_text(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
) -> String {
    item_runtime
        .and_then(|runtime| runtime.localized_text_if_present(translation_key))
        .unwrap_or(fallback)
        .to_string()
}

pub(super) fn hud_mob_effects_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudMobEffect> {
    let Some(local_player_id) = world.local_player_id() else {
        return Vec::new();
    };
    world
        .entity_effects(local_player_id)
        .unwrap_or_default()
        .into_values()
        .filter_map(|effect| {
            let descriptor = vanilla_mob_effect_type(effect.effect_id)?;
            let resource_path = descriptor
                .resource_id
                .strip_prefix("minecraft:")
                .unwrap_or(descriptor.resource_id);
            let name_key = format!("effect.minecraft.{resource_path}");
            let fallback_name = humanize_resource_path(resource_path);
            let mut display_name =
                localized_gui_text(item_runtime, &name_key, &fallback_name).to_string();
            if (1..=9).contains(&effect.amplifier) {
                let level = effect.amplifier + 1;
                let level_key = format!("enchantment.level.{level}");
                let fallback_level = mob_effect_level_fallback(level);
                let localized_level = localized_gui_text(item_runtime, &level_key, fallback_level);
                display_name.push(' ');
                display_name.push_str(&localized_level);
            }
            Some(HudMobEffect {
                effect_id: effect.effect_id,
                beneficial: descriptor.category == MobEffectCategory::Beneficial,
                color: descriptor.color,
                amplifier: effect.amplifier,
                duration_ticks: effect.duration_ticks,
                ambient: effect.ambient,
                show_icon: effect.show_icon,
                display_name,
                duration_text: hud_mob_effect_duration_text(
                    item_runtime,
                    effect.duration_ticks,
                    world.ticking().tick_rate,
                ),
            })
        })
        .collect()
}

pub(super) fn humanize_resource_path(resource_path: &str) -> String {
    resource_path
        .split(['_', '/'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| {
                    first
                        .to_uppercase()
                        .chain(characters.flat_map(char::to_lowercase))
                        .collect::<String>()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn mob_effect_level_fallback(level: i32) -> &'static str {
    match level {
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        7 => "VII",
        8 => "VIII",
        9 => "IX",
        10 => "X",
        _ => "",
    }
}

pub(super) fn hud_mob_effect_duration_text(
    item_runtime: Option<&NativeItemRuntime>,
    duration_ticks: i32,
    tick_rate: f32,
) -> String {
    if duration_ticks == -1 {
        return localized_gui_text(item_runtime, "effect.duration.infinite", "∞").to_string();
    }
    let tick_rate = if tick_rate.is_finite() && tick_rate > 0.0 {
        tick_rate
    } else {
        20.0
    };
    let total_seconds = ((duration_ticks.max(0) as f32) / tick_rate).floor() as u64;
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = total_seconds / 3_600;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub(super) fn hud_styled_runs_from_rich_text(
    component: &RichTextComponent,
) -> Vec<HudStyledTextRun> {
    hud_styled_runs_from_component_runs(component.styled())
}

pub(super) fn hud_styled_runs_from_component_runs(runs: &[StyledTextRun]) -> Vec<HudStyledTextRun> {
    // Every caller draws a Component — the container title,
    // the advancement toast title and the
    // test-instance status lines — so the codes resolve here with the part
    // style as the `§r` reset, exactly like `SubStringSource.create`.
    crate::component_text::legacy_resolved_runs(runs)
        .iter()
        .filter(|run| !run.text.is_empty())
        .map(|run| HudStyledTextRun {
            shadow_color: run.style.shadow_color,
            text: run.text.clone(),
            style: HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
        })
        .collect()
}

pub(super) fn server_pack_bytes_human_readable(bytes: u64) -> String {
    if bytes < 1_024 {
        return format!("{bytes} B");
    }
    let exponent = ((bytes as f64).ln() / 1_024_f64.ln()).floor() as usize;
    let prefixes = ['K', 'M', 'G', 'T', 'P', 'E'];
    let bounded_exponent = exponent.clamp(1, prefixes.len());
    format!(
        "{:.1} {}B",
        bytes as f64 / 1_024_f64.powi(bounded_exponent as i32),
        prefixes[bounded_exponent - 1]
    )
}

#[cfg(test)]
pub(super) fn hud_system_toasts(
    state: &mut HudRenderRuntime,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    now: Instant,
) -> (
    Vec<bbb_client_render::HudSystemToastInstance>,
    Option<LocalSoundEventState>,
) {
    let periodic_notification = state.periodic_notifications.update(now).cloned();
    let sounds = state.client_toasts.update_with_options(
        world,
        item_runtime,
        &bbb_client_render::HudFontGlyphMap::default(),
        periodic_notification,
        1.0,
        now,
    );
    (
        state.client_toasts.project_systems(item_runtime),
        sounds.into_iter().find(|sound| {
            matches!(
                sound.sound.location.as_deref(),
                Some("minecraft:ui.toast.in" | "minecraft:ui.toast.out")
            )
        }),
    )
}

#[cfg(test)]
pub(super) fn hud_system_toast(
    state: &mut HudRenderRuntime,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    now: Instant,
) -> (
    Option<bbb_client_render::HudSystemToast>,
    Option<LocalSoundEventState>,
) {
    let (toasts, sound) = hud_system_toasts(state, world, item_runtime, now);
    (toasts.into_iter().next().map(|toast| toast.toast), sound)
}
