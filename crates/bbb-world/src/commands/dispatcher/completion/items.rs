use std::collections::BTreeMap;

use bbb_protocol::{
    data_component_types::{
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
    },
    normalize_resource_location, parse_command_snbt_value_prefix,
};

use super::{push_registry_candidates, push_resource_candidates};
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn push_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    access: FrozenRegistryAccess<'_>,
) {
    let Some((item, components)) = remaining.split_once('[') else {
        push_registry_candidates(output, remaining, "minecraft:item", false, access);
        let Ok(resource_id) = normalize_resource_location(remaining.to_string()) else {
            return;
        };
        if access.resource_membership("minecraft:item", &resource_id)
            == FrozenRegistryMembership::Present
        {
            output.entry(format!("{remaining}[")).or_default();
        }
        return;
    };
    let Ok(item_id) = normalize_resource_location(item.to_string()) else {
        return;
    };
    if access.resource_membership("minecraft:item", &item_id) != FrozenRegistryMembership::Present {
        return;
    }
    push_component_patch_candidates(output, item, components, access);
}

pub(super) fn push_predicate_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    access: FrozenRegistryAccess<'_>,
) {
    let Some((item, conditions)) = remaining.split_once('[') else {
        if "*".starts_with(remaining) {
            output.entry("*".to_string()).or_default();
        }
        push_registry_candidates(output, remaining, "minecraft:item", true, access);
        if item_predicate_root_is_exact(remaining, access) {
            output.entry(format!("{remaining}[")).or_default();
        }
        return;
    };
    if !item_predicate_root_is_exact(item, access) || top_level_close(conditions).is_some() {
        return;
    }
    let (current_start, current) = last_condition_start(conditions);
    let prefix = &conditions[..current_start];
    let current = current.trim_start();
    let negated = current.starts_with('!');
    let term = current.strip_prefix('!').unwrap_or(current);
    let operator = term
        .char_indices()
        .find(|(_, character)| matches!(character, '=' | '~'));
    let base = format!("{item}[{prefix}{}", if negated { "!" } else { "" });
    let Some((operator_offset, operator)) = operator else {
        push_predicate_type_candidates(output, &base, term, access);
        let Ok(resource_id) = normalize_resource_location(term.to_string()) else {
            return;
        };
        let component = predicate_component(&resource_id);
        let predicate = predicate_type(&resource_id);
        if component {
            output.entry(format!("{base}{term}=")).or_default();
            push_predicate_delimiters(output, &format!("{base}{term}"));
        }
        if predicate {
            output.entry(format!("{base}{term}~")).or_default();
        }
        return;
    };
    let resource = &term[..operator_offset];
    let Ok(resource_id) = normalize_resource_location(resource.to_string()) else {
        return;
    };
    if (operator == '=' && !predicate_component(&resource_id))
        || (operator == '~' && !predicate_type(&resource_id))
    {
        return;
    }
    let value = &term[operator_offset + operator.len_utf8()..];
    let Some((consumed, _)) = parse_command_snbt_value_prefix(value) else {
        return;
    };
    if consumed == value.len() {
        push_predicate_delimiters(output, remaining);
    }
}

fn push_component_patch_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    item: &str,
    components: &str,
    access: FrozenRegistryAccess<'_>,
) {
    let mut cursor = 0;
    loop {
        skip_whitespace(components, &mut cursor);
        if cursor >= components.len() {
            push_component_ids(output, item, components, cursor, false, access);
            output
                .entry(format!("{item}[{}!", &components[..cursor]))
                .or_default();
            return;
        }
        if components.as_bytes().get(cursor) == Some(&b']') {
            return;
        }
        let entry_start = cursor;
        let removed = components.as_bytes().get(cursor) == Some(&b'!');
        if removed {
            cursor += 1;
        }
        let component_start = cursor;
        while components
            .as_bytes()
            .get(cursor)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b',' | b']'))
        {
            cursor += 1;
        }
        let raw_component = &components[component_start..cursor];
        let component = normalize_resource_location(raw_component.to_string())
            .ok()
            .filter(|resource_id| persistent_component(resource_id));
        if component.is_none() {
            push_component_ids(output, item, components, component_start, removed, access);
            return;
        }
        skip_whitespace(components, &mut cursor);
        if removed {
            if !push_next_or_end(output, item, components, cursor) {
                return;
            }
        } else {
            match components.as_bytes().get(cursor) {
                None => {
                    output
                        .entry(format!("{item}[{}=", &components[..cursor]))
                        .or_default();
                    return;
                }
                Some(b'=') => cursor += 1,
                _ => return,
            }
            skip_whitespace(components, &mut cursor);
            let Some((consumed, _)) = parse_command_snbt_value_prefix(&components[cursor..]) else {
                return;
            };
            cursor += consumed;
            skip_whitespace(components, &mut cursor);
            if !push_next_or_end(output, item, components, cursor) {
                return;
            }
        }
        match components.as_bytes().get(cursor) {
            Some(b',') => {
                cursor += 1;
            }
            Some(b']') => return,
            None => return,
            _ => return,
        }
        if cursor <= entry_start {
            return;
        }
    }
}

fn push_next_or_end(
    output: &mut BTreeMap<String, Option<String>>,
    item: &str,
    components: &str,
    cursor: usize,
) -> bool {
    if cursor != components.len() {
        return true;
    }
    let complete = format!("{item}[{}", &components[..cursor]);
    output.entry(format!("{complete},")).or_default();
    output.entry(format!("{complete}]")).or_default();
    false
}

fn push_component_ids(
    output: &mut BTreeMap<String, Option<String>>,
    item: &str,
    components: &str,
    component_start: usize,
    removed: bool,
    access: FrozenRegistryAccess<'_>,
) {
    let Some(component_ids) = access.resource_ids_in_holder_order("minecraft:data_component_type")
    else {
        return;
    };
    let component_ids = component_ids
        .into_iter()
        .filter(|resource_id| persistent_component(resource_id));
    let partial = &components[component_start..];
    let mut nested = BTreeMap::new();
    push_resource_candidates(&mut nested, partial, component_ids, "");
    let prefix = &components[..component_start];
    for component in nested.into_keys() {
        let assignment = if removed { "" } else { "=" };
        output
            .entry(format!("{item}[{prefix}{component}{assignment}"))
            .or_default();
    }
}

fn persistent_component(resource_id: &str) -> bool {
    vanilla_data_component_type_for_resource_id(resource_id).is_some_and(|entry| entry.persistent)
}

fn item_predicate_root_is_exact(remaining: &str, access: FrozenRegistryAccess<'_>) -> bool {
    if remaining == "*" {
        return true;
    }
    let (tagged, raw) = remaining
        .strip_prefix('#')
        .map_or((false, remaining), |raw| (true, raw));
    let Ok(resource_id) = normalize_resource_location(raw.to_string()) else {
        return false;
    };
    if tagged {
        access.tag_membership("minecraft:item", &resource_id) == FrozenRegistryMembership::Present
    } else {
        access.resource_membership("minecraft:item", &resource_id)
            == FrozenRegistryMembership::Present
    }
}

fn push_predicate_type_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    prefix: &str,
    remaining: &str,
    access: FrozenRegistryAccess<'_>,
) {
    let mut candidates = access
        .resource_ids_in_holder_order("minecraft:data_component_type")
        .unwrap_or_default()
        .into_iter()
        .filter(|resource_id| persistent_component(resource_id))
        .collect::<Vec<_>>();
    candidates.push("minecraft:count");
    candidates.extend(
        VANILLA_DATA_COMPONENT_PREDICATE_TYPES
            .iter()
            .map(|entry| entry.resource_id),
    );
    candidates.sort_unstable();
    candidates.dedup();
    let mut nested = BTreeMap::new();
    push_resource_candidates(&mut nested, remaining, candidates, "");
    for candidate in nested.into_keys() {
        output.entry(format!("{prefix}{candidate}")).or_default();
    }
}

fn predicate_component(resource_id: &str) -> bool {
    resource_id == "minecraft:count" || persistent_component(resource_id)
}

fn predicate_type(resource_id: &str) -> bool {
    resource_id == "minecraft:count"
        || VANILLA_DATA_COMPONENT_PREDICATE_TYPES
            .iter()
            .any(|entry| entry.resource_id == resource_id)
}

fn push_predicate_delimiters(output: &mut BTreeMap<String, Option<String>>, complete: &str) {
    output.entry(format!("{complete},")).or_default();
    output.entry(format!("{complete}|")).or_default();
    output.entry(format!("{complete}]")).or_default();
}

fn top_level_close(value: &str) -> Option<usize> {
    scan_top_level(value).find_map(|(index, byte)| (byte == b']').then_some(index))
}

fn last_condition_start(value: &str) -> (usize, &str) {
    let start = scan_top_level(value)
        .filter_map(|(index, byte)| matches!(byte, b',' | b'|').then_some(index + 1))
        .last()
        .unwrap_or(0);
    (start, &value[start..])
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
            b',' | b'|' | b']' if braces == 0 && brackets == 0 => return true,
            _ => {}
        }
        false
    })
}

fn skip_whitespace(value: &str, cursor: &mut usize) {
    while value
        .as_bytes()
        .get(*cursor)
        .is_some_and(u8::is_ascii_whitespace)
    {
        *cursor += 1;
    }
}
