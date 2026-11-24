use super::*;

pub(super) fn push_ominous_bottle_tooltip_lines(
    language: &LanguageCatalog,
    amplifier: Option<i32>,
    tooltip_tick_rate: f32,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(amplifier) = amplifier else {
        return;
    };

    lines.push(NativeItemTooltipLine::plain(
        potion_effect_tooltip_text(
            language,
            "minecraft:bad_omen",
            amplifier,
            OMINOUS_BOTTLE_BAD_OMEN_DURATION_TICKS,
            1.0,
            tooltip_tick_rate,
        ),
        TOOLTIP_TEXT_BLUE,
    ));
}

pub(super) fn push_potion_contents_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    duration_scale: f32,
    tooltip_tick_rate: f32,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let mut has_effect_entries = false;
    let mut attribute_modifiers = Vec::new();
    if let Some(potion_id) = component_patch.potion_contents.potion_id {
        if let Some(effects) = vanilla_potion(potion_id).map(|potion| potion.effects) {
            has_effect_entries |= !effects.is_empty();
            for effect in effects {
                push_potion_effect_tooltip_line(
                    language,
                    effect.effect_id,
                    effect.amplifier,
                    effect.duration,
                    duration_scale,
                    tooltip_tick_rate,
                    lines,
                );
                collect_potion_effect_attribute_modifiers(
                    effect.effect_id,
                    effect.amplifier,
                    &mut attribute_modifiers,
                );
            }
        }
    }
    has_effect_entries |= !component_patch.potion_contents.custom_effects.is_empty();
    for effect in &component_patch.potion_contents.custom_effects {
        push_potion_effect_tooltip_line(
            language,
            effect.effect_id,
            effect.amplifier,
            effect.duration,
            duration_scale,
            tooltip_tick_rate,
            lines,
        );
        collect_potion_effect_attribute_modifiers(
            effect.effect_id,
            effect.amplifier,
            &mut attribute_modifiers,
        );
    }
    if !has_effect_entries && potion_contents_component_present(component_patch) {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key("effect.none").to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    push_potion_attribute_modifier_tooltip_lines(language, &attribute_modifiers, lines);
}

pub(super) fn potion_contents_component_present(
    component_patch: &DataComponentPatchSummary,
) -> bool {
    component_patch.potion_contents.potion_id.is_some()
        || component_patch.potion_contents.custom_color.is_some()
        || component_patch
            .added_type_ids
            .contains(&component_id::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
        || component_patch.potion_contents.custom_name.is_some()
        || !component_patch.potion_contents.custom_effects.is_empty()
}

pub(super) fn push_potion_effect_tooltip_line(
    language: &LanguageCatalog,
    effect_id: i32,
    amplifier: i32,
    duration: i32,
    duration_scale: f32,
    tooltip_tick_rate: f32,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(effect_key) = vanilla_mob_effect_type(effect_id).map(|effect| effect.resource_id)
    else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        potion_effect_tooltip_text(
            language,
            effect_key,
            amplifier,
            duration,
            duration_scale,
            tooltip_tick_rate,
        ),
        mob_effect_tooltip_tint_for_id(effect_id),
    ));
}

pub(super) fn collect_potion_effect_attribute_modifiers(
    effect_id: i32,
    amplifier: i32,
    lines: &mut Vec<PotionAttributeModifierLine>,
) {
    let amplifier_scale = f64::from(amplifier) + 1.0;
    lines.extend(
        vanilla_mob_effect_type(effect_id)
            .map(|effect| effect.attribute_modifiers)
            .unwrap_or(&[])
            .iter()
            .filter_map(|modifier| {
                Some(PotionAttributeModifierLine {
                    attribute_description_key: vanilla_attribute_type(modifier.attribute_id)?
                        .description_key,
                    amount: f64::from_bits(modifier.amount_bits) * amplifier_scale,
                    operation_id: modifier.operation_id,
                })
            }),
    );
}

pub(super) fn push_potion_attribute_modifier_tooltip_lines(
    language: &LanguageCatalog,
    modifiers: &[PotionAttributeModifierLine],
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if modifiers.is_empty() {
        return;
    }
    lines.push(NativeItemTooltipLine::plain(
        String::new(),
        TOOLTIP_TEXT_WHITE,
    ));
    lines.push(NativeItemTooltipLine::plain(
        language.get_or_key(POTION_WHEN_DRANK_KEY).to_string(),
        TOOLTIP_TEXT_DARK_PURPLE,
    ));

    for modifier in modifiers {
        push_potion_attribute_modifier_tooltip_line(language, modifier, lines);
    }
}

pub(super) fn push_potion_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    modifier: &PotionAttributeModifierLine,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if modifier.amount == 0.0 {
        return;
    }
    let mut display_amount = if modifier.operation_id
        == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
        || modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
    {
        modifier.amount * 100.0
    } else {
        modifier.amount
    };
    let (key_prefix, tint) = if modifier.amount > 0.0 {
        ("attribute.modifier.plus", TOOLTIP_TEXT_BLUE)
    } else {
        display_amount *= -1.0;
        ("attribute.modifier.take", TOOLTIP_TEXT_RED)
    };
    let amount = format_attribute_modifier_amount(display_amount);
    let attribute = language
        .get_or_key(modifier.attribute_description_key)
        .to_string();
    let text = translate_with_two_args(
        language,
        &format!("{key_prefix}.{}", modifier.operation_id),
        &amount,
        &attribute,
    )
    .replace("%%", "%");
    lines.push(NativeItemTooltipLine::plain(text, tint));
}

pub(super) fn format_attribute_modifier_amount(amount: f64) -> String {
    let mut text = format!("{amount:.2}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        "0".to_string()
    } else {
        text
    }
}

pub(super) fn push_stack_attribute_modifiers_tooltip_lines(
    language: &LanguageCatalog,
    modifiers: &[AttributeModifierSummary],
    enchantment_modifiers: &[EnchantmentAttributeModifier],
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for slot_group in VANILLA_EQUIPMENT_SLOT_GROUPS {
        let slot_id = slot_group.id;
        let slot_key = slot_group.serialized_name;
        let mut section_lines = Vec::new();
        for modifier in modifiers
            .iter()
            .filter(|modifier| modifier.slot_id == slot_id)
        {
            if modifier.display_id == VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID {
                continue;
            }
            let Some(line) = stack_attribute_modifier_tooltip_line(language, modifier, options)
            else {
                continue;
            };
            section_lines.push(line);
        }
        section_lines.extend(
            enchantment_modifiers
                .iter()
                .filter(|modifier| modifier.slot == slot_key)
                .filter_map(|modifier| {
                    enchantment_attribute_modifier_tooltip_line(language, modifier, options)
                }),
        );
        if !section_lines.is_empty() {
            lines.push(NativeItemTooltipLine::plain(
                String::new(),
                TOOLTIP_TEXT_WHITE,
            ));
            lines.push(NativeItemTooltipLine::plain(
                language
                    .get_or_key(&format!("item.modifiers.{slot_key}"))
                    .to_string(),
                TOOLTIP_TEXT_GRAY,
            ));
            lines.extend(section_lines);
        }
    }
}

pub(super) fn stack_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    modifier: &AttributeModifierSummary,
    options: NativeItemTooltipOptions<'_>,
) -> Option<NativeItemTooltipLine> {
    if modifier.display_id == VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID {
        return modifier
            .display_text
            .as_ref()
            .map(|text| NativeItemTooltipLine {
                text: text.clone(),
                tint: TOOLTIP_TEXT_WHITE,
                runs: hud_runs_from_component(
                    modifier.display_text_styled.as_deref().unwrap_or_default(),
                    text,
                    &ComponentStyle::default(),
                ),
            });
    }

    let attribute = tooltip_attribute(modifier.attribute_id, options.attribute_keys)?;
    numeric_attribute_modifier_tooltip_line(
        language,
        attribute,
        &modifier.modifier_id,
        f64::from_bits(modifier.amount_bits),
        modifier.operation_id,
        options,
    )
}

pub(super) fn enchantment_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    modifier: &EnchantmentAttributeModifier,
    options: NativeItemTooltipOptions<'_>,
) -> Option<NativeItemTooltipLine> {
    numeric_attribute_modifier_tooltip_line(
        language,
        vanilla_attribute_type_for_resource_id(&modifier.attribute)?,
        &modifier.modifier_id,
        modifier.amount,
        modifier.operation_id,
        options,
    )
}

pub(super) fn numeric_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    attribute: &'static AttributeTypeCatalogEntry,
    modifier_id: &str,
    raw_amount: f64,
    operation_id: i32,
    options: NativeItemTooltipOptions<'_>,
) -> Option<NativeItemTooltipLine> {
    let (amount, display_with_base) = match modifier_id {
        "minecraft:base_attack_damage" => options
            .player_base_attack_damage
            .map(|base| (raw_amount + base, true))
            .unwrap_or((raw_amount, false)),
        "minecraft:base_attack_speed" => options
            .player_base_attack_speed
            .map(|base| (raw_amount + base, true))
            .unwrap_or((raw_amount, false)),
        _ => (raw_amount, false),
    };
    if amount == 0.0 && !display_with_base {
        return None;
    }
    let mut display_amount = if operation_id
        == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
        || operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
    {
        amount * 100.0
    } else if attribute.resource_id == "minecraft:knockback_resistance" {
        amount * 10.0
    } else {
        amount
    };
    if display_with_base {
        let text = translate_with_two_args(
            language,
            &format!("attribute.modifier.equals.{operation_id}"),
            &format_attribute_modifier_amount(display_amount),
            language.get_or_key(attribute.description_key),
        )
        .replace("%%", "%");
        return Some(NativeItemTooltipLine::plain(
            format!(" {text}"),
            TOOLTIP_TEXT_DARK_GREEN,
        ));
    }

    let (key_prefix, tint) = if amount > 0.0 {
        (
            "attribute.modifier.plus",
            stack_attribute_modifier_tint(attribute.sentiment, true),
        )
    } else {
        display_amount *= -1.0;
        (
            "attribute.modifier.take",
            stack_attribute_modifier_tint(attribute.sentiment, false),
        )
    };
    let amount = format_attribute_modifier_amount(display_amount);
    let attribute_name = language.get_or_key(attribute.description_key).to_string();
    let text = translate_with_two_args(
        language,
        &format!("{key_prefix}.{operation_id}"),
        &amount,
        &attribute_name,
    )
    .replace("%%", "%");
    Some(NativeItemTooltipLine::plain(text, tint))
}

pub(super) fn tooltip_attribute(
    attribute_id: i32,
    attribute_keys: Option<&[String]>,
) -> Option<&'static AttributeTypeCatalogEntry> {
    match attribute_keys {
        Some(keys) => usize::try_from(attribute_id)
            .ok()
            .and_then(|index| keys.get(index))
            .and_then(|key| vanilla_attribute_type_for_resource_id(key)),
        None => vanilla_attribute_type(attribute_id),
    }
}

pub(super) fn stack_attribute_modifier_tint(
    sentiment: AttributeSentiment,
    value_increase: bool,
) -> [f32; 4] {
    match sentiment {
        AttributeSentiment::Positive => {
            if value_increase {
                TOOLTIP_TEXT_BLUE
            } else {
                TOOLTIP_TEXT_RED
            }
        }
        AttributeSentiment::Neutral => TOOLTIP_TEXT_GRAY,
        AttributeSentiment::Negative => {
            if value_increase {
                TOOLTIP_TEXT_RED
            } else {
                TOOLTIP_TEXT_BLUE
            }
        }
    }
}

pub(super) fn push_suspicious_stew_tooltip_lines(
    language: &LanguageCatalog,
    effects: &[SuspiciousStewEffectSummary],
    creative: bool,
    tooltip_tick_rate: f32,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !creative {
        return;
    }
    for effect in effects {
        let Some(effect_key) =
            vanilla_mob_effect_type(effect.effect_id).map(|effect| effect.resource_id)
        else {
            continue;
        };
        lines.push(NativeItemTooltipLine::plain(
            potion_effect_tooltip_text(
                language,
                effect_key,
                0,
                effect.duration,
                1.0,
                tooltip_tick_rate,
            ),
            mob_effect_tooltip_tint_for_id(effect.effect_id),
        ));
    }
}

pub(crate) fn potion_effect_tooltip_text(
    language: &LanguageCatalog,
    effect_key: &str,
    amplifier: i32,
    duration_ticks: i32,
    duration_scale: f32,
    tooltip_tick_rate: f32,
) -> String {
    let mut effect = language
        .get_or_key(&description_key("effect", effect_key))
        .to_string();
    if amplifier > 0 {
        let potency = language
            .get_or_key(&format!("potion.potency.{amplifier}"))
            .to_string();
        effect = translate_with_two_args(language, "potion.withAmplifier", &effect, &potency);
    }

    if duration_ticks == -1 || duration_ticks > 20 {
        let duration = if duration_ticks == -1 {
            language.get_or_key("effect.duration.infinite").to_string()
        } else {
            format_tick_duration(
                ((duration_ticks as f32) * duration_scale).floor() as i32,
                tooltip_tick_rate,
            )
        };
        effect = translate_with_two_args(language, "potion.withDuration", &effect, &duration);
    }

    effect
}

pub(super) fn mob_effect_tooltip_tint_for_id(effect_id: i32) -> [f32; 4] {
    match vanilla_mob_effect_type(effect_id).map(|effect| effect.category) {
        Some(MobEffectCategory::Harmful) => TOOLTIP_TEXT_RED,
        Some(MobEffectCategory::Beneficial | MobEffectCategory::Neutral) => TOOLTIP_TEXT_BLUE,
        None => TOOLTIP_TEXT_GRAY,
    }
}

pub(super) fn format_tick_duration(ticks: i32, tickrate: f32) -> String {
    let mut seconds = ((ticks as f32) / tickrate).floor() as i32;
    let mut minutes = seconds / 60;
    seconds %= 60;
    let hours = minutes / 60;
    minutes %= 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub(super) fn push_dyed_color_tooltip_lines(
    language: &LanguageCatalog,
    dyed_color: Option<i32>,
    advanced: bool,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(dyed_color) = dyed_color else {
        return;
    };
    if advanced {
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "item.color", &dyed_color_tooltip_hex(dyed_color)),
            TOOLTIP_TEXT_GRAY,
        ));
    } else {
        let text = language.get_or_key("item.dyed").to_string();
        lines.push(NativeItemTooltipLine {
            text: text.clone(),
            tint: TOOLTIP_TEXT_GRAY,
            runs: vec![HudComponentTextRun {
                visual: HudStyledTextRun {
                    shadow_color: None,
                    text,
                    style: HudTextStyle {
                        italic: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0xAA_AA_AA),
                },
                font: None,
            }],
        });
    }
}

pub(super) fn dyed_color_tooltip_hex(dyed_color: i32) -> String {
    format!("#{:06X}", dyed_color as u32)
}

pub(super) fn push_fireworks_tooltip_lines(
    language: &LanguageCatalog,
    flight_duration: Option<i32>,
    explosions: &[FireworkExplosionSummary],
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(flight_duration) = flight_duration.filter(|flight_duration| *flight_duration > 0) {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{} {}",
                language.get_or_key("item.minecraft.firework_rocket.flight"),
                flight_duration
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }

    let mut current = None;
    let mut count = 0;
    for explosion in explosions {
        match current {
            None => {
                current = Some(explosion);
                count = 1;
            }
            Some(previous) if previous == explosion => {
                count += 1;
            }
            Some(previous) => {
                push_fireworks_explosion_group_tooltip_lines(language, previous, count, lines);
                current = Some(explosion);
                count = 1;
            }
        }
    }
    if let Some(explosion) = current {
        push_fireworks_explosion_group_tooltip_lines(language, explosion, count, lines);
    }
}

pub(super) fn push_fireworks_explosion_group_tooltip_lines(
    language: &LanguageCatalog,
    explosion: &FireworkExplosionSummary,
    count: usize,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let shape_name = firework_explosion_shape_text(language, explosion.shape);
    let text = if count == 1 {
        translate_with_first_arg(
            language,
            "item.minecraft.firework_rocket.single_star",
            &shape_name,
        )
    } else {
        translate_with_two_args(
            language,
            "item.minecraft.firework_rocket.multiple_stars",
            &count.to_string(),
            &shape_name,
        )
    };
    lines.push(NativeItemTooltipLine::plain(text, TOOLTIP_TEXT_GRAY));
    push_firework_explosion_additional_tooltip_lines(
        language,
        &explosion.colors,
        &explosion.fade_colors,
        explosion.has_trail,
        explosion.has_twinkle,
        "  ",
        lines,
    );
}

pub(super) fn push_firework_explosion_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(explosion) = component_patch.firework_explosion.as_ref() else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        firework_explosion_shape_text(language, explosion.shape),
        TOOLTIP_TEXT_GRAY,
    ));
    push_firework_explosion_additional_tooltip_lines(
        language,
        &explosion.colors,
        &explosion.fade_colors,
        explosion.has_trail,
        explosion.has_twinkle,
        "",
        lines,
    );
}

pub(super) fn push_firework_explosion_additional_tooltip_lines(
    language: &LanguageCatalog,
    colors: &[i32],
    fade_colors: &[i32],
    has_trail: bool,
    has_twinkle: bool,
    prefix: &str,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !colors.is_empty() {
        lines.push(NativeItemTooltipLine::plain(
            format!("{prefix}{}", firework_color_names(language, colors)),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    if !fade_colors.is_empty() {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{prefix}{} {}",
                language.get_or_key("item.minecraft.firework_star.fade_to"),
                firework_color_names(language, fade_colors)
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    if has_trail {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{prefix}{}",
                language.get_or_key("item.minecraft.firework_star.trail")
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    if has_twinkle {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{prefix}{}",
                language.get_or_key("item.minecraft.firework_star.flicker")
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_jukebox_playable_tooltip_lines(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    songs: &JukeboxSongRegistry,
    song_id: Option<i32>,
    song: Option<&JukeboxSongSummary>,
    session_keys: Option<&[String]>,
    session_presentations: Option<&[Option<JukeboxSongPresentation>]>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(song) = song {
        push_jukebox_description(
            language,
            bound_key_lookup,
            &song.description,
            song.description_styled.as_deref().unwrap_or(&[]),
            song.description_nbt.as_deref(),
            lines,
        );
        return;
    }
    let Some(song_id) = song_id else {
        return;
    };
    let Some(index) = usize::try_from(song_id).ok() else {
        return;
    };
    if let Some(session_presentations) = session_presentations {
        if let Some(Some(song)) = session_presentations.get(index) {
            push_jukebox_description(
                language,
                bound_key_lookup,
                &song.description,
                &song.description_styled,
                Some(&song.description_nbt),
                lines,
            );
            return;
        }
    }
    let song_key = if session_presentations.is_some() {
        session_keys
            .and_then(|keys| keys.get(index))
            .filter(|key| !key.is_empty())
            .map(String::as_str)
    } else {
        songs.song_id(song_id)
    };
    let Some(song_key) = song_key else {
        return;
    };
    let description = language
        .get_or_key(&description_key("jukebox_song", song_key))
        .to_string();
    lines.push(NativeItemTooltipLine::plain(description, TOOLTIP_TEXT_GRAY));
}

pub(super) fn push_jukebox_description(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    description: &str,
    description_styled: &[StyledTextRun],
    description_nbt: Option<&NbtSummaryValue>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let (text, styled) = localized_component_description_with_binding(
        language,
        description,
        description_styled,
        description_nbt,
        bound_key_lookup,
    );
    let base = ComponentStyle {
        color: Some(TOOLTIP_GRAY_TEXT_COLOR),
        ..ComponentStyle::default()
    };
    lines.push(NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_GRAY,
        runs: hud_runs_from_component(&styled, &text, &base),
    });
}

pub(super) fn localized_component_description_with_binding(
    language: &LanguageCatalog,
    description: &str,
    description_styled: &[StyledTextRun],
    description_nbt: Option<&NbtSummaryValue>,
    bound_key_lookup: Option<BoundKeyLookupFn>,
) -> (String, Vec<StyledTextRun>) {
    let Some(component) = description_nbt else {
        return (description.to_string(), description_styled.to_vec());
    };
    let lookup = TooltipTextLookup {
        language,
        bound_key_lookup,
    };
    let styled =
        bbb_client_assets::localized_component_runs(component, &ComponentStyle::default(), &lookup);
    let text = styled.iter().map(|run| run.text.as_str()).collect();
    (text, styled)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_armor_trim_tooltip_lines(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    material_id: Option<i32>,
    material: Option<&TrimMaterialSummary>,
    pattern_id: Option<i32>,
    pattern: Option<&TrimPatternSummary>,
    material_keys: Option<&[String]>,
    material_presentations: Option<&[Option<TrimMaterialPresentation>]>,
    pattern_keys: Option<&[String]>,
    pattern_presentations: Option<&[Option<TrimPatternPresentation>]>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(material_tooltip) = armor_trim_material_tooltip(
        language,
        bound_key_lookup,
        material_id,
        material,
        material_keys,
        material_presentations,
    ) else {
        return;
    };
    let Some(pattern_description) = armor_trim_pattern_description(
        language,
        bound_key_lookup,
        pattern_id,
        pattern,
        pattern_keys,
        pattern_presentations,
    ) else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key("item.minecraft.smithing_template.upgrade")
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
    lines.push(armor_trim_description_line(
        &pattern_description,
        &material_tooltip.style,
    ));
    lines.push(armor_trim_description_line(
        &material_tooltip.description,
        &ComponentStyle::default(),
    ));
}

pub(super) struct ArmorTrimMaterialTooltip {
    pub(super) description: ArmorTrimDescription,
    pub(super) style: ComponentStyle,
}

pub(super) struct ArmorTrimDescription {
    pub(super) text: String,
    pub(super) runs: Vec<StyledTextRun>,
}

pub(super) fn armor_trim_material_tooltip(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    material_id: Option<i32>,
    material: Option<&TrimMaterialSummary>,
    material_keys: Option<&[String]>,
    material_presentations: Option<&[Option<TrimMaterialPresentation>]>,
) -> Option<ArmorTrimMaterialTooltip> {
    if let Some(material) = material {
        let material = direct_trim_material_presentation(material)?;
        let description = armor_trim_component_description(
            language,
            bound_key_lookup,
            &material.description,
            &material.description_styled,
            material.description_nbt.as_ref(),
        );
        let style = armor_trim_material_style(&description.runs);
        return Some(ArmorTrimMaterialTooltip { description, style });
    }
    let material_id = material_id?;
    let index = usize::try_from(material_id).ok()?;
    if let Some(Some(material)) =
        material_presentations.and_then(|presentations| presentations.get(index))
    {
        let description = armor_trim_component_description(
            language,
            bound_key_lookup,
            &material.description,
            &material.description_styled,
            material.description_nbt.as_ref(),
        );
        let style = armor_trim_material_style(&description.runs);
        return Some(ArmorTrimMaterialTooltip { description, style });
    }
    let key = match material_keys {
        Some(keys) => keys.get(index)?.as_str(),
        // `minecraft:trim_material` is loaded from data files
        // so the unsynchronized holder id is a
        // sorted-resource-id index.
        None => loader_ordered_resource_id(TRIM_MATERIAL_REGISTRY, material_id)?,
    };
    let material = VanillaTrimMaterialIdentity::for_resource_id(key)?;
    let text = language
        .get_or_key(&description_key("trim_material", key))
        .to_string();
    let style = ComponentStyle {
        color: Some(material.description_color()),
        ..ComponentStyle::default()
    };
    Some(ArmorTrimMaterialTooltip {
        description: ArmorTrimDescription {
            text: text.clone(),
            runs: vec![StyledTextRun {
                text,
                style: style.clone(),
            }],
        },
        style,
    })
}

pub(super) fn armor_trim_pattern_description(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    pattern_id: Option<i32>,
    pattern: Option<&TrimPatternSummary>,
    pattern_keys: Option<&[String]>,
    pattern_presentations: Option<&[Option<TrimPatternPresentation>]>,
) -> Option<ArmorTrimDescription> {
    if let Some(pattern) = pattern {
        let pattern = direct_trim_pattern_presentation(pattern)?;
        return Some(armor_trim_component_description(
            language,
            bound_key_lookup,
            &pattern.description,
            &pattern.description_styled,
            pattern.description_nbt.as_ref(),
        ));
    }
    let pattern_id = pattern_id?;
    let index = usize::try_from(pattern_id).ok()?;
    if let Some(Some(pattern)) =
        pattern_presentations.and_then(|presentations| presentations.get(index))
    {
        return Some(armor_trim_component_description(
            language,
            bound_key_lookup,
            &pattern.description,
            &pattern.description_styled,
            pattern.description_nbt.as_ref(),
        ));
    }
    let key = match pattern_keys {
        Some(keys) => keys.get(index)?.as_str(),
        // `minecraft:trim_pattern` is loaded from data files
        // so the unsynchronized holder id is a
        // sorted-resource-id index.
        None => loader_ordered_resource_id(TRIM_PATTERN_REGISTRY, pattern_id)?,
    };
    let text = language
        .get_or_key(&description_key("trim_pattern", key))
        .to_string();
    Some(ArmorTrimDescription {
        text: text.clone(),
        runs: vec![StyledTextRun {
            text,
            style: ComponentStyle::default(),
        }],
    })
}

pub(super) fn armor_trim_component_description(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    text: &str,
    styled: &[StyledTextRun],
    nbt: Option<&NbtSummaryValue>,
) -> ArmorTrimDescription {
    let styled = if styled.is_empty() {
        vec![StyledTextRun {
            text: text.to_string(),
            style: ComponentStyle::default(),
        }]
    } else {
        styled.to_vec()
    };
    let (text, runs) = localized_component_description_with_binding(
        language,
        text,
        &styled,
        nbt,
        bound_key_lookup,
    );
    ArmorTrimDescription { text, runs }
}

pub(super) fn armor_trim_material_style(runs: &[StyledTextRun]) -> ComponentStyle {
    runs.iter()
        .find(|run| !run.text.is_empty())
        .map(|run| run.style.clone())
        .unwrap_or_default()
}

pub(super) fn armor_trim_description_line(
    description: &ArmorTrimDescription,
    base: &ComponentStyle,
) -> NativeItemTooltipLine {
    let mut runs = Vec::with_capacity(description.runs.len() + 1);
    runs.push(HudComponentTextRun::plain(" "));
    runs.extend(hud_runs_from_component(
        &description.runs,
        &description.text,
        base,
    ));
    NativeItemTooltipLine {
        text: format!(" {}", description.text),
        tint: TOOLTIP_TEXT_WHITE,
        runs,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_enchantments_tooltip_lines(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    enchantments: &[ItemEnchantmentSummary],
    enchantment_keys: Option<&[String]>,
    enchantment_definitions: Option<&[Option<EnchantmentTooltipDefinition>]>,
    tooltip_order: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if enchantments.is_empty() {
        return;
    }

    let resolved = enchantments
        .iter()
        .enumerate()
        .filter_map(|(index, enchantment)| {
            enchantment_key_for_holder(enchantment.holder_id, enchantment_keys).map(|key| {
                (
                    index,
                    enchantment,
                    key,
                    enchantment_definition_for_holder(
                        enchantment.holder_id,
                        enchantment_definitions,
                    ),
                )
            })
        })
        .collect::<Vec<_>>();
    let mut emitted = vec![false; enchantments.len()];
    if let Some(tooltip_order) = tooltip_order {
        for order_key in tooltip_order {
            push_matching_ordered_enchantments(
                language,
                bound_key_lookup,
                &resolved,
                &mut emitted,
                order_key,
                enchantment_tags,
                lines,
            );
        }
    } else {
        for order_key in VANILLA_ENCHANTMENT_TOOLTIP_ORDER {
            push_matching_ordered_enchantments(
                language,
                bound_key_lookup,
                &resolved,
                &mut emitted,
                order_key,
                enchantment_tags,
                lines,
            );
        }
    }

    for (index, enchantment, key, definition) in resolved {
        if !emitted[index] {
            push_enchantment_tooltip_line(
                language,
                bound_key_lookup,
                key,
                enchantment.level,
                definition,
                enchantment_tags,
                lines,
            );
        }
    }
}

pub(super) fn push_matching_ordered_enchantments(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    resolved: &[(
        usize,
        &ItemEnchantmentSummary,
        &str,
        Option<&EnchantmentTooltipDefinition>,
    )],
    emitted: &mut [bool],
    order_key: &str,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for (index, enchantment, key, definition) in resolved {
        if !emitted[*index] && *key == order_key && enchantment.level > 0 {
            push_enchantment_tooltip_line(
                language,
                bound_key_lookup,
                key,
                enchantment.level,
                *definition,
                enchantment_tags,
                lines,
            );
            emitted[*index] = true;
        }
    }
}

pub(super) fn push_enchantment_tooltip_line(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    enchantment_key: &str,
    level: i32,
    definition: Option<&EnchantmentTooltipDefinition>,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let tint = if enchantment_is_curse(enchantment_key, enchantment_tags) {
        TOOLTIP_TEXT_RED
    } else {
        TOOLTIP_TEXT_GRAY
    };
    let base = ComponentStyle {
        color: tooltip_text_color_for_tint(tint),
        ..ComponentStyle::default()
    };
    let (mut text, mut component_runs) =
        enchantment_description(language, bound_key_lookup, enchantment_key, definition);
    let max_level = definition
        .map(|definition| definition.max_level)
        .or_else(|| {
            VanillaEnchantmentIdentity::for_resource_id(enchantment_key)
                .map(VanillaEnchantmentIdentity::max_level)
        })
        .unwrap_or(1);
    if level != 1 || max_level != 1 {
        let appended_style = component_runs
            .first()
            .map(|run| run.style.clone())
            .unwrap_or_default();
        let level_text = language
            .get_or_key(&format!("enchantment.level.{level}"))
            .to_string();
        text.push(' ');
        text.push_str(&level_text);
        component_runs.push(StyledTextRun {
            text: " ".to_string(),
            style: appended_style.clone(),
        });
        component_runs.push(StyledTextRun {
            text: level_text,
            style: appended_style,
        });
    }
    let runs = hud_runs_from_component(&component_runs, &text, &base);
    lines.push(NativeItemTooltipLine { text, tint, runs });
}

pub(super) fn enchantment_description(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    enchantment_key: &str,
    definition: Option<&EnchantmentTooltipDefinition>,
) -> (String, Vec<StyledTextRun>) {
    let Some(definition) = definition else {
        let text = language
            .get_or_key(&description_key("enchantment", enchantment_key))
            .to_string();
        return (
            text.clone(),
            vec![StyledTextRun {
                text,
                style: ComponentStyle::default(),
            }],
        );
    };
    let (text, mut runs) = localized_component_description_with_binding(
        language,
        &definition.description,
        &definition.description_styled,
        definition.description_nbt.as_ref(),
        bound_key_lookup,
    );
    if runs.is_empty() {
        runs.push(StyledTextRun {
            text: text.clone(),
            style: ComponentStyle::default(),
        });
    }
    (text, runs)
}

pub(super) fn enchantment_key_for_holder(
    holder_id: i32,
    enchantment_keys: Option<&[String]>,
) -> Option<&str> {
    if let Some(enchantment_keys) = enchantment_keys {
        let index = usize::try_from(holder_id).ok()?;
        return enchantment_keys.get(index).map(String::as_str);
    }
    // `minecraft:enchantment` is loaded from data files
    // so the unsynchronized holder id is a
    // sorted-resource-id index rather than an `Enchantments` bootstrap ordinal.
    loader_ordered_resource_id(ENCHANTMENT_REGISTRY, holder_id)
}

pub(super) fn enchantment_definition_for_holder(
    holder_id: i32,
    definitions: Option<&[Option<EnchantmentTooltipDefinition>]>,
) -> Option<&EnchantmentTooltipDefinition> {
    definitions?.get(usize::try_from(holder_id).ok()?)?.as_ref()
}

pub(super) fn enchantment_is_curse(
    enchantment_key: &str,
    enchantment_tags: Option<&TagCatalog>,
) -> bool {
    enchantment_tags.is_some_and(|tags| tags.contains("minecraft:curse", enchantment_key))
        || matches!(
            enchantment_key,
            "minecraft:binding_curse" | "minecraft:vanishing_curse"
        )
}

pub(super) fn push_profile_tooltip_lines(
    language: &LanguageCatalog,
    profile: Option<&ResolvableProfileSummary>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !profile.is_some_and(is_dynamic_profile) {
        return;
    }
    lines.push(NativeItemTooltipLine::plain(
        language.get_or_key("component.profile.dynamic").to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
}

pub(super) fn is_dynamic_profile(profile: &ResolvableProfileSummary) -> bool {
    profile.kind == ResolvableProfileKindSummary::Partial
        && profile.properties.is_empty()
        && (profile.name.is_some() != profile.uuid.is_some())
}

pub(super) fn firework_explosion_shape_text(
    language: &LanguageCatalog,
    shape: FireworkExplosionShapeSummary,
) -> String {
    language
        .get_or_key(&format!(
            "item.minecraft.firework_star.shape.{}",
            shape.as_str()
        ))
        .to_string()
}

pub(super) fn firework_color_names(language: &LanguageCatalog, colors: &[i32]) -> String {
    colors
        .iter()
        .map(|color| {
            language
                .get_or_key(firework_color_translation_key(*color))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn firework_color_translation_key(color: i32) -> &'static str {
    match color {
        15_790_320 => "item.minecraft.firework_star.white",
        15_435_844 => "item.minecraft.firework_star.orange",
        12_801_229 => "item.minecraft.firework_star.magenta",
        6_719_955 => "item.minecraft.firework_star.light_blue",
        14_602_026 => "item.minecraft.firework_star.yellow",
        4_312_372 => "item.minecraft.firework_star.lime",
        14_188_952 => "item.minecraft.firework_star.pink",
        4_408_131 => "item.minecraft.firework_star.gray",
        11_250_603 => "item.minecraft.firework_star.light_gray",
        2_651_799 => "item.minecraft.firework_star.cyan",
        8_073_150 => "item.minecraft.firework_star.purple",
        2_437_522 => "item.minecraft.firework_star.blue",
        5_320_730 => "item.minecraft.firework_star.brown",
        3_887_386 => "item.minecraft.firework_star.green",
        11_743_532 => "item.minecraft.firework_star.red",
        1_973_019 => "item.minecraft.firework_star.black",
        _ => "item.minecraft.firework_star.custom_color",
    }
}
