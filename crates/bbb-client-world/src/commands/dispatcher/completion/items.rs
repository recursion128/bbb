use std::collections::BTreeMap;

use bbb_protocol::{
    command_known_item_predicate,
    data_component_types::{
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
    },
    normalize_resource_location, parse_command_snbt_value_prefix_utf16, JavaString,
};

use super::{
    ascii_text, find_any_unit, push_registry_candidates, push_resource_candidates, split_once_unit,
    starts_with_unit, trim_start_units, unit_at, units_eq_ascii, CandidateText, CompletionMetadata,
};
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn push_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    units: &[u16],
    access: FrozenRegistryAccess<'_>,
) {
    let Some((item, components)) = split_once_unit(units, b'[') else {
        push_registry_candidates(output, units, "minecraft:item", false, access);
        let Some(resource_id) =
            ascii_text(units).and_then(|text| normalize_resource_location(text).ok())
        else {
            return;
        };
        if access.resource_membership("minecraft:item", &resource_id)
            == FrozenRegistryMembership::Present
        {
            output
                .entry(CandidateText::units(units).then_unit(b'[').finish())
                .or_default();
        }
        return;
    };
    let Some(item_id) = ascii_text(item).and_then(|text| normalize_resource_location(text).ok())
    else {
        return;
    };
    if access.resource_membership("minecraft:item", &item_id) != FrozenRegistryMembership::Present {
        return;
    }
    push_component_patch_candidates(output, item, components, access);
}

pub(super) fn push_predicate_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    units: &[u16],
    access: FrozenRegistryAccess<'_>,
) {
    let Some((item, conditions)) = split_once_unit(units, b'[') else {
        if units_eq_ascii(units, "*") {
            output.entry(JavaString::from("*")).or_default();
        }
        push_registry_candidates(output, units, "minecraft:item", true, access);
        if item_predicate_root_is_exact(units, access) {
            output
                .entry(CandidateText::units(units).then_unit(b'[').finish())
                .or_default();
        }
        return;
    };
    if !item_predicate_root_is_exact(item, access) || top_level_close(conditions).is_some() {
        return;
    }
    let (current_start, segment) = last_condition_start(conditions);
    let prefix = &conditions[..current_start];
    let current = trim_start_units(segment);
    let negated = starts_with_unit(current, b'!');
    let term = if negated { &current[1..] } else { current };
    let base = CandidateText::units(item)
        .then_unit(b'[')
        .then_units(prefix);
    let base = if negated { base.then_unit(b'!') } else { base };
    let Some(operator_offset) = find_any_unit(term, b"=~") else {
        push_predicate_type_candidates(output, &base, term, access);
        let Some(resource_id) =
            ascii_text(term).and_then(|term| normalize_resource_location(term).ok())
        else {
            return;
        };
        let component = predicate_component(&resource_id);
        let predicate = command_known_item_predicate(&resource_id);
        if component {
            output
                .entry(base.clone().then_units(term).then_unit(b'=').finish())
                .or_default();
            push_predicate_delimiters(output, &base.clone().then_units(term));
        }
        if predicate {
            output
                .entry(base.clone().then_units(term).then_unit(b'~').finish())
                .or_default();
        }
        return;
    };
    let resource = &term[..operator_offset];
    let Some(resource_id) =
        ascii_text(resource).and_then(|resource| normalize_resource_location(resource).ok())
    else {
        return;
    };
    let operator = term[operator_offset];
    if (operator == u16::from(b'=') && !predicate_component(&resource_id))
        || (operator == u16::from(b'~') && !command_known_item_predicate(&resource_id))
    {
        return;
    }
    let value = &term[operator_offset + 1..];
    let Ok((consumed, _)) = parse_command_snbt_value_prefix_utf16(value) else {
        return;
    };
    if consumed == value.len() {
        push_predicate_delimiters(output, &CandidateText::units(units));
    }
}

fn push_component_patch_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    item: &[u16],
    components: &[u16],
    access: FrozenRegistryAccess<'_>,
) {
    let base = CandidateText::units(item).then_unit(b'[');
    let mut cursor = 0;
    loop {
        skip_whitespace(components, &mut cursor);
        if cursor >= components.len() {
            push_component_ids(output, &base, components, cursor, false, access);
            output
                .entry(base.clone().then_units(components).then_unit(b'!').finish())
                .or_default();
            return;
        }
        if unit_at(components, cursor, b']') {
            return;
        }
        let entry_start = cursor;
        let removed = unit_at(components, cursor, b'!');
        if removed {
            cursor += 1;
        }
        let component_start = cursor;
        while components
            .get(cursor)
            .is_some_and(|unit| !is_ascii_whitespace_unit(*unit) && !is_component_terminator(*unit))
        {
            cursor += 1;
        }
        let component = ascii_text(&components[component_start..cursor])
            .and_then(|raw| normalize_resource_location(raw).ok())
            .filter(|resource_id| persistent_component(resource_id));
        if component.is_none() {
            push_component_ids(output, &base, components, component_start, removed, access);
            return;
        }
        skip_whitespace(components, &mut cursor);
        if removed {
            if !push_next_or_end(output, &base, components, cursor) {
                return;
            }
        } else {
            match components.get(cursor) {
                None => {
                    output
                        .entry(
                            base.clone()
                                .then_units(&components[..cursor])
                                .then_unit(b'=')
                                .finish(),
                        )
                        .or_default();
                    return;
                }
                Some(unit) if *unit == u16::from(b'=') => cursor += 1,
                _ => return,
            }
            skip_whitespace(components, &mut cursor);
            let Ok((consumed, _)) = parse_command_snbt_value_prefix_utf16(&components[cursor..])
            else {
                return;
            };
            cursor += consumed;
            skip_whitespace(components, &mut cursor);
            if !push_next_or_end(output, &base, components, cursor) {
                return;
            }
        }
        match components.get(cursor) {
            Some(unit) if *unit == u16::from(b',') => {
                cursor += 1;
            }
            Some(unit) if *unit == u16::from(b']') => return,
            None => return,
            _ => return,
        }
        if cursor <= entry_start {
            return;
        }
    }
}

fn push_next_or_end(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    base: &CandidateText,
    components: &[u16],
    cursor: usize,
) -> bool {
    if cursor != components.len() {
        return true;
    }
    let complete = base.clone().then_units(&components[..cursor]);
    output
        .entry(complete.clone().then_unit(b',').finish())
        .or_default();
    output.entry(complete.then_unit(b']').finish()).or_default();
    false
}

fn push_component_ids(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    base: &CandidateText,
    components: &[u16],
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
    let mut nested = BTreeMap::new();
    push_resource_candidates(
        &mut nested,
        &components[component_start..],
        component_ids,
        "",
    );
    let head = base.clone().then_units(&components[..component_start]);
    for component in nested.into_keys() {
        let assignment = if removed { "" } else { "=" };
        output
            .entry(
                head.clone()
                    .then_units(component.units())
                    .then_ascii(assignment)
                    .finish(),
            )
            .or_default();
    }
}

fn persistent_component(resource_id: &str) -> bool {
    vanilla_data_component_type_for_resource_id(resource_id).is_some_and(|entry| entry.persistent)
}

fn item_predicate_root_is_exact(remaining: &[u16], access: FrozenRegistryAccess<'_>) -> bool {
    if units_eq_ascii(remaining, "*") {
        return true;
    }
    let (tagged, raw) = if starts_with_unit(remaining, b'#') {
        (true, &remaining[1..])
    } else {
        (false, remaining)
    };
    let Some(resource_id) = ascii_text(raw).and_then(|raw| normalize_resource_location(raw).ok())
    else {
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
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    prefix: &CandidateText,
    remaining: &[u16],
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
        output
            .entry(prefix.clone().then_units(candidate.units()).finish())
            .or_default();
    }
}

fn predicate_component(resource_id: &str) -> bool {
    resource_id == "minecraft:count" || persistent_component(resource_id)
}

fn push_predicate_delimiters(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    complete: &CandidateText,
) {
    for suffix in [",", "|", "]"] {
        output
            .entry(complete.clone().then_ascii(suffix).finish())
            .or_default();
    }
}

fn top_level_close(value: &[u16]) -> Option<usize> {
    scan_top_level(value).find_map(|(index, unit)| (unit == u16::from(b']')).then_some(index))
}

fn last_condition_start(value: &[u16]) -> (usize, &[u16]) {
    let start = scan_top_level(value)
        .filter_map(|(index, unit)| {
            (unit == u16::from(b',') || unit == u16::from(b'|')).then_some(index + 1)
        })
        .last()
        .unwrap_or(0);
    (start, &value[start..])
}

fn scan_top_level(value: &[u16]) -> impl Iterator<Item = (usize, u16)> + '_ {
    let mut quote = None;
    let mut escaped = false;
    let mut braces = 0usize;
    let mut brackets = 0usize;
    value.iter().copied().enumerate().filter(move |(_, unit)| {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if *unit == u16::from(b'\\') {
                escaped = true;
            } else if *unit == active_quote {
                quote = None;
            }
            return false;
        }
        if *unit == u16::from(b'\'') || *unit == u16::from(b'"') {
            quote = Some(*unit);
        } else if *unit == u16::from(b'{') {
            braces += 1;
        } else if *unit == u16::from(b'}') {
            braces = braces.saturating_sub(1);
        } else if *unit == u16::from(b'[') {
            brackets += 1;
        } else if *unit == u16::from(b']') && brackets > 0 {
            brackets -= 1;
        } else if braces == 0
            && brackets == 0
            && (*unit == u16::from(b',') || *unit == u16::from(b'|') || *unit == u16::from(b']'))
        {
            return true;
        }
        false
    })
}

fn skip_whitespace(value: &[u16], cursor: &mut usize) {
    while value
        .get(*cursor)
        .is_some_and(|unit| is_ascii_whitespace_unit(*unit))
    {
        *cursor += 1;
    }
}

fn is_ascii_whitespace_unit(unit: u16) -> bool {
    unit < 0x80 && (unit as u8).is_ascii_whitespace()
}

/// `=`, `,` and `]` end a component name, as the whitespace above does.
fn is_component_terminator(unit: u16) -> bool {
    unit == u16::from(b'=') || unit == u16::from(b',') || unit == u16::from(b']')
}

#[cfg(test)]
#[test]
fn command_item_predicate_completion_accepts_component_existence_fallback() {
    let registries = crate::RegistrySet::vanilla_26_1();
    for name in [
        "custom_name",
        "creative_slot_lock",
        "additional_trade_cost",
        "map_post_processing",
    ] {
        let mut output = BTreeMap::new();
        let prefix = format!("*[{name}");
        let prefix_units: Vec<u16> = prefix.encode_utf16().collect();
        push_predicate_candidates(&mut output, &prefix_units, registries.frozen_access());
        assert!(
            output.contains_key(&JavaString::from(format!("{prefix}~"))),
            "{name}"
        );
        if name != "custom_name" {
            assert!(!output.contains_key(&JavaString::from(format!("{prefix}="))));
            assert!(!output.contains_key(&JavaString::from(format!("{prefix}]"))));
        }
        output.clear();
        let value_prefix = format!("{prefix}~{{}}");
        let value_units: Vec<u16> = value_prefix.encode_utf16().collect();
        push_predicate_candidates(&mut output, &value_units, registries.frozen_access());
        for delimiter in [",", "|", "]"] {
            assert!(
                output.contains_key(&JavaString::from(format!("{value_prefix}{delimiter}"))),
                "{name}"
            );
        }
    }
    let mut output = BTreeMap::new();
    let missing: Vec<u16> = "*[missing".encode_utf16().collect();
    push_predicate_candidates(&mut output, &missing, registries.frozen_access());
    assert!(!output.contains_key(&JavaString::from("*[missing~")));
}
