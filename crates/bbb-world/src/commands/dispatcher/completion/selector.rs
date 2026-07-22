use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{
    parse_command_entity_selector_prefix, CommandEntitySelectorOptionKind,
    VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};

use super::{push_resource_candidates, push_string_candidates, CommandCompletionSources};

#[derive(Default)]
struct SelectorCompletionState<'a> {
    selector: char,
    seen_once: BTreeSet<&'a str>,
    name_equals: bool,
    name_not_equals: bool,
    limited: bool,
    sorted: bool,
    gamemode_equals: bool,
    gamemode_not_equals: bool,
    team_equals: bool,
    team_not_equals: bool,
    type_limited: bool,
    type_inverse: bool,
}

pub(super) fn push_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    _players_only: bool,
    sources: &CommandCompletionSources<'_>,
) {
    let Some(selector) = remaining.strip_prefix('@') else {
        push_string_candidates(
            output,
            remaining,
            sources.online_player_names.iter().copied(),
        );
        return;
    };
    let Some(selector_type) = selector.chars().next() else {
        push_string_candidates(output, remaining, ["@p", "@a", "@r", "@s", "@e", "@n"]);
        return;
    };
    if !matches!(selector_type, 'p' | 'a' | 'r' | 's' | 'e' | 'n') {
        push_string_candidates(output, remaining, ["@p", "@a", "@r", "@s", "@e", "@n"]);
        return;
    }
    let selector_len = '@'.len_utf8() + selector_type.len_utf8();
    if remaining.len() == selector_len {
        output.entry(format!("{remaining}[")).or_default();
        return;
    }
    if remaining.as_bytes().get(selector_len) != Some(&b'[') {
        return;
    }
    let body = &remaining[selector_len + 1..];
    if top_level_close(body).is_some() {
        return;
    }
    let segments = top_level_segments(body);
    let Some((current_start, current)) = segments.last().copied() else {
        return;
    };
    let mut state = SelectorCompletionState {
        selector: selector_type,
        type_limited: matches!(selector_type, 'p' | 'a' | 'r'),
        ..SelectorCompletionState::default()
    };
    for (_, segment) in &segments[..segments.len().saturating_sub(1)] {
        state.apply_completed(segment.trim());
    }
    let prefix = &body[..current_start];
    let current_trimmed = current.trim_start();
    let leading = &current[..current.len() - current_trimmed.len()];
    let Some((raw_key, raw_value)) = current_trimmed.split_once('=') else {
        push_option_names(
            output,
            remaining,
            selector_len,
            prefix,
            leading,
            current_trimmed,
            &state,
        );
        return;
    };
    let key = raw_key.trim();
    if !state.option_applicable(key) {
        return;
    }
    let value = raw_value.trim_start();
    let value_leading = &raw_value[..raw_value.len() - value.len()];
    let option_prefix = format!(
        "{}[{}{}{}={value_leading}",
        &remaining[..selector_len],
        prefix,
        leading,
        raw_key
    );
    match key {
        "sort" => push_string_candidates_with_prefix(
            output,
            &option_prefix,
            value,
            ["nearest", "furthest", "random", "arbitrary"],
        ),
        "gamemode" => push_gamemode_candidates(output, &option_prefix, value, &state),
        "type" => push_entity_type_candidates(output, &option_prefix, value, &state, sources),
        _ => {}
    }
    if selector_value_is_complete(remaining) {
        output.entry(format!("{remaining},")).or_default();
        output.entry(format!("{remaining}]")).or_default();
    }
}

impl SelectorCompletionState<'_> {
    fn apply_completed(&mut self, segment: &str) {
        let Some((key, value)) = segment.split_once('=') else {
            return;
        };
        let key = key.trim();
        let inverted = value.trim_start().starts_with('!');
        let Some(option) = VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS
            .iter()
            .find(|option| option.name == key)
        else {
            return;
        };
        match option.kind {
            CommandEntitySelectorOptionKind::Name => {
                if inverted {
                    self.name_not_equals = true;
                } else {
                    self.name_equals = true;
                }
            }
            CommandEntitySelectorOptionKind::Once => {
                self.seen_once.insert(option.name);
            }
            CommandEntitySelectorOptionKind::Limit => self.limited = true,
            CommandEntitySelectorOptionKind::Sort => self.sorted = true,
            CommandEntitySelectorOptionKind::Gamemode => {
                if inverted {
                    self.gamemode_not_equals = true;
                } else {
                    self.gamemode_equals = true;
                }
            }
            CommandEntitySelectorOptionKind::Team => {
                if inverted {
                    self.team_not_equals = true;
                } else {
                    self.team_equals = true;
                }
            }
            CommandEntitySelectorOptionKind::Type => {
                if inverted {
                    self.type_inverse = true;
                } else {
                    self.type_limited = true;
                }
            }
            CommandEntitySelectorOptionKind::Repeatable => {}
        }
    }

    fn option_applicable(&self, name: &str) -> bool {
        let Some(option) = VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS
            .iter()
            .find(|option| option.name == name)
        else {
            return false;
        };
        match option.kind {
            CommandEntitySelectorOptionKind::Name => !self.name_equals,
            CommandEntitySelectorOptionKind::Once => !self.seen_once.contains(option.name),
            CommandEntitySelectorOptionKind::Limit => self.selector != 's' && !self.limited,
            CommandEntitySelectorOptionKind::Sort => self.selector != 's' && !self.sorted,
            CommandEntitySelectorOptionKind::Gamemode => !self.gamemode_equals,
            CommandEntitySelectorOptionKind::Team => !self.team_equals,
            CommandEntitySelectorOptionKind::Type => !self.type_limited,
            CommandEntitySelectorOptionKind::Repeatable => true,
        }
    }
}

fn push_option_names(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    selector_len: usize,
    prefix: &str,
    leading: &str,
    current: &str,
    state: &SelectorCompletionState<'_>,
) {
    let base = format!("{}[{prefix}{leading}", &remaining[..selector_len]);
    if current.is_empty() {
        output.entry(format!("{base}]")).or_default();
    }
    for option in VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS {
        if state.option_applicable(option.name) && option.name.starts_with(current) {
            output.entry(format!("{base}{}=", option.name)).or_default();
        }
    }
}

fn push_gamemode_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    prefix: &str,
    remaining: &str,
    state: &SelectorCompletionState<'_>,
) {
    let modes = ["survival", "creative", "adventure", "spectator"];
    if remaining.starts_with('!') {
        push_string_candidates_with_prefix(
            output,
            prefix,
            remaining,
            modes.map(|mode| format!("!{mode}")),
        );
    } else if !state.gamemode_not_equals {
        push_string_candidates_with_prefix(output, prefix, remaining, modes);
        if remaining.is_empty() {
            push_string_candidates_with_prefix(
                output,
                prefix,
                remaining,
                modes.map(|mode| format!("!{mode}")),
            );
        }
    }
}

fn push_entity_type_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    prefix: &str,
    remaining: &str,
    state: &SelectorCompletionState<'_>,
    sources: &CommandCompletionSources<'_>,
) {
    let access = sources.registry_access;
    let Some(entity_types) = access.resource_ids_in_holder_order("minecraft:entity_type") else {
        return;
    };
    let tags = access.tag_ids("minecraft:entity_type").unwrap_or_default();
    let mut nested = BTreeMap::new();
    push_resource_candidates(&mut nested, remaining, entity_types.iter().copied(), "!");
    push_resource_candidates(&mut nested, remaining, tags.iter().copied(), "!#");
    if !state.type_inverse {
        push_resource_candidates(&mut nested, remaining, entity_types.iter().copied(), "");
        push_resource_candidates(&mut nested, remaining, tags.iter().copied(), "#");
    }
    for candidate in nested.into_keys() {
        output.entry(format!("{prefix}{candidate}")).or_default();
    }
}

fn push_string_candidates_with_prefix(
    output: &mut BTreeMap<String, Option<String>>,
    prefix: &str,
    remaining: &str,
    candidates: impl IntoIterator<Item = impl AsRef<str>>,
) {
    let mut nested = BTreeMap::new();
    push_string_candidates(&mut nested, remaining, candidates);
    for candidate in nested.into_keys() {
        output.entry(format!("{prefix}{candidate}")).or_default();
    }
}

fn selector_value_is_complete(remaining: &str) -> bool {
    let closed = format!("{remaining}]");
    parse_command_entity_selector_prefix(&closed)
        .is_some_and(|parsed| parsed.consumed == closed.len())
}

fn top_level_close(value: &str) -> Option<usize> {
    scan_top_level(value).find_map(|(index, byte)| (byte == b']').then_some(index))
}

fn top_level_segments(value: &str) -> Vec<(usize, &str)> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, byte) in scan_top_level(value) {
        if byte == b',' {
            result.push((start, &value[start..index]));
            start = index + 1;
        }
    }
    result.push((start, &value[start..]));
    result
}

fn scan_top_level(value: &str) -> impl Iterator<Item = (usize, u8)> + '_ {
    let mut quote = None;
    let mut escaped = false;
    let mut braces = 0usize;
    let mut brackets = 0usize;
    value.bytes().enumerate().filter(move |(_, byte)| {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == active_quote {
                quote = None;
            }
            return false;
        }
        match *byte {
            b'\'' | b'"' => quote = Some(*byte),
            b'{' => braces += 1,
            b'}' => braces = braces.saturating_sub(1),
            b'[' => brackets += 1,
            b']' if brackets > 0 => brackets -= 1,
            b',' | b']' if braces == 0 && brackets == 0 => return true,
            _ => {}
        }
        false
    })
}
