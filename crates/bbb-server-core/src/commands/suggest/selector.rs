//! `EntitySelectorParser` suggestions once a selector's type letter is read
//! (EntitySelectorParser.java:253-325, :506-531).

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPES;
use bbb_protocol::game_types::VANILLA_GAME_TYPES;
use bbb_protocol::packets::CommandSuggestion;

use super::{matches_sub_str, plain, resource_matches, Candidate};
use crate::commands::{component, translatable};
use crate::world::ServerWorld;

/// `EntitySelectorOptions` registrations (EntitySelectorOptions.java:90-481).
const OPTIONS: [&str; 21] = [
    "name",
    "distance",
    "level",
    "x",
    "y",
    "z",
    "dx",
    "dy",
    "dz",
    "x_rotation",
    "y_rotation",
    "limit",
    "sort",
    "gamemode",
    "team",
    "type",
    "tag",
    "nbt",
    "scores",
    "advancements",
    "predicate",
];

const SORTS: [&str; 4] = ["nearest", "furthest", "random", "arbitrary"];

/// The candidates for `remaining` (the argument text from `start`) when it is
/// `@` and a type letter; `None` for anything else.
pub(super) fn option_candidates(
    start: usize,
    remaining: &str,
    world: &ServerWorld,
) -> Option<Vec<Candidate>> {
    let letter = remaining.strip_prefix('@')?.chars().next()?;
    if !matches!(letter, 'a' | 'e' | 'n' | 'p' | 'r' | 's') {
        return None;
    }
    let text = remaining.as_bytes();
    let mut pos = 2;
    // `suggestOpenOptions` (:506-509).
    if pos == text.len() {
        return Some(vec![Candidate::at(start + pos, plain("["))]);
    }
    if text[pos] != b'[' {
        return Some(Vec::new());
    }
    pos += 1;
    let mut used: Vec<(&str, bool)> = Vec::new();
    loop {
        pos = skip_whitespace(text, pos);
        let key_start = pos;
        if pos < text.len() && text[pos] == b']' {
            return Some(Vec::new());
        }
        while pos < text.len() && is_unquoted(text[pos]) {
            pos += 1;
        }
        let key = &remaining[key_start..pos];
        pos = skip_whitespace(text, pos);
        // `suggestOptionsKey` (:517-520) over the unknown, unfinished or
        // `=`-less key the parse rewound to (:291-298).
        if !OPTIONS.contains(&key) || pos == text.len() || text[pos] != b'=' {
            return Some(keys(
                start + key_start,
                &remaining[key_start..],
                letter,
                &used,
            ));
        }
        pos = skip_whitespace(text, pos + 1);
        let value_start = pos;
        let value_end = value_end(text, pos);
        let value = &remaining[value_start..value_end];
        if value_end == text.len() {
            return Some(value_candidates(
                key,
                value,
                start + value_start,
                start + value_end,
                &used,
                world,
            ));
        }
        used.push((key, value.starts_with('!')));
        if text[value_end] == b']' {
            return Some(Vec::new());
        }
        pos = value_end + 1;
    }
}

/// `EntitySelectorOptions.suggestNames` (EntitySelectorOptions.java:501-509)
/// with each option's `canUse`.
fn keys(start: usize, remaining: &str, letter: char, used: &[(&str, bool)]) -> Vec<Candidate> {
    let prefix = remaining.to_lowercase();
    let has = |name: &str| used.iter().any(|(key, _)| *key == name);
    let has_equals = |name: &str| used.iter().any(|(key, inverted)| *key == name && !inverted);
    OPTIONS
        .into_iter()
        .filter(|name| match *name {
            "name" | "gamemode" | "team" => !has_equals(name),
            "type" => !matches!(letter, 'a' | 'p' | 'r') && !has_equals(name),
            "limit" | "sort" => letter != 's' && !has(name),
            "tag" | "nbt" | "predicate" => true,
            other => !has(other),
        })
        .filter(|name| name.starts_with(&prefix))
        .map(|name| {
            let tooltip = component(translatable(
                format!("argument.entity.options.{name}.description"),
                Vec::new(),
            ));
            Candidate::at(
                start,
                CommandSuggestion {
                    text: format!("{name}=").into(),
                    tooltip: Some(tooltip.text().to_string()),
                    tooltip_styled: Some(tooltip.styled().to_vec()),
                    tooltip_component: Some(tooltip.component().clone()),
                },
            )
        })
        .collect()
}

/// The hook a value leaves in place: the option's own list while it does not
/// parse, `suggestOptionsNextOrClose` (:522-526) once it does (:302-305).
fn value_candidates(
    key: &str,
    value: &str,
    start: usize,
    end: usize,
    used: &[(&str, bool)],
    world: &ServerWorld,
) -> Vec<Candidate> {
    let lower = value.to_lowercase();
    let bare = value.strip_prefix('!').unwrap_or(value);
    let texts: Vec<String> = match key {
        // EntitySelectorOptions.java:176-192.
        "sort" if !SORTS.contains(&value) => SORTS
            .into_iter()
            .filter(|sort| matches_sub_str(&lower, sort))
            .map(str::to_owned)
            .collect(),
        // EntitySelectorOptions.java:193-249.
        "gamemode"
            if !VANILLA_GAME_TYPES
                .iter()
                .any(|mode| mode.serialized_name == bare) =>
        {
            let not_equals = used
                .iter()
                .any(|(key, inverted)| *key == "gamemode" && *inverted);
            let (prefix, normal, inverted) = match lower.strip_prefix('!') {
                Some(prefix) => (prefix, false, true),
                None if lower.is_empty() => (lower.as_str(), !not_equals, true),
                None => (lower.as_str(), !not_equals, false),
            };
            let mut texts = Vec::new();
            for mode in VANILLA_GAME_TYPES {
                if mode.serialized_name.starts_with(prefix) {
                    if inverted {
                        texts.push(format!("!{}", mode.serialized_name));
                    }
                    if normal {
                        texts.push(mode.serialized_name.to_owned());
                    }
                }
            }
            texts
        }
        // EntitySelectorOptions.java:264-304.
        "type" if !type_parses(bare) => {
            let inversely = used
                .iter()
                .any(|(key, inverted)| *key == "type" && *inverted);
            let tags: Vec<&str> = world
                .biome_tags()
                .map(|tags| tags.tag_ids("minecraft:entity_type").collect())
                .unwrap_or_default();
            let ids = VANILLA_ENTITY_TYPES.iter().map(|entry| entry.resource_id);
            let mut texts = Vec::new();
            texts.extend(
                ids.clone()
                    .filter(|id| prefixed_matches(&lower, "!", id))
                    .map(|id| format!("!{id}")),
            );
            texts.extend(
                tags.iter()
                    .filter(|id| prefixed_matches(&lower, "!#", id))
                    .map(|id| format!("!#{id}")),
            );
            if !inversely {
                texts.extend(
                    ids.filter(|id| resource_matches(&lower, id))
                        .map(str::to_owned),
                );
                texts.extend(
                    tags.iter()
                        .filter(|id| prefixed_matches(&lower, "#", id))
                        .map(|id| format!("#{id}")),
                );
            }
            texts
        }
        "sort" | "gamemode" | "type" => return next_or_close(end),
        _ if value.is_empty() => Vec::new(),
        _ => return next_or_close(end),
    };
    texts
        .iter()
        .map(|text| Candidate::at(start, plain(text)))
        .collect()
}

fn next_or_close(end: usize) -> Vec<Candidate> {
    [",", "]"]
        .into_iter()
        .map(|text| Candidate::at(end, plain(text)))
        .collect()
}

/// `SharedSuggestionProvider.filterResources`'s prefixed form
/// (SharedSuggestionProvider.java:109-121).
fn prefixed_matches(contents: &str, prefix: &str, id: &str) -> bool {
    if contents.is_empty() {
        return true;
    }
    let common = contents
        .bytes()
        .zip(prefix.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    common > 0 && resource_matches(&contents[common..], id)
}

/// Whether the `type` value reads: a tag is any identifier, a type must exist.
fn type_parses(value: &str) -> bool {
    match value.strip_prefix('#') {
        Some(tag) => !tag.is_empty(),
        None => {
            let id = if value.contains(':') {
                value.to_owned()
            } else {
                format!("minecraft:{value}")
            };
            VANILLA_ENTITY_TYPES
                .iter()
                .any(|entry| entry.resource_id == id)
        }
    }
}

fn skip_whitespace(text: &[u8], mut pos: usize) -> usize {
    while pos < text.len() && text[pos] == b' ' {
        pos += 1;
    }
    pos
}

/// `StringReader.isAllowedInUnquotedString`.
fn is_unquoted(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'+')
}

/// Where an option value ends: the first `,` or `]` outside quotes and
/// nested brackets or braces.
fn value_end(text: &[u8], mut pos: usize) -> usize {
    let mut depth = 0usize;
    let mut quote = None;
    while pos < text.len() {
        let byte = text[pos];
        match quote {
            Some(open) => {
                if byte == b'\\' {
                    pos += 1;
                } else if byte == open {
                    quote = None;
                }
            }
            None => match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'{' | b'[' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                b']' if depth > 0 => depth -= 1,
                b',' | b']' if depth == 0 => return pos,
                _ => {}
            },
        }
        pos += 1;
    }
    text.len()
}
