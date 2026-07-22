use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{block_types::vanilla_block_type_for_resource_id, normalize_resource_location};

use super::push_registry_candidates;
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn push_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    include_tags: bool,
    access: FrozenRegistryAccess<'_>,
) {
    let (head, properties) = remaining
        .split_once('[')
        .map_or((remaining, None), |(head, properties)| {
            (head, Some(properties))
        });
    if head.contains('{') {
        return;
    }
    let tagged = head.starts_with('#');
    if tagged && !include_tags {
        return;
    }
    let raw_id = head.strip_prefix('#').unwrap_or(head);
    let prefix = if tagged { "#" } else { "" };
    if properties.is_none() {
        push_registry_candidates(output, remaining, "minecraft:block", include_tags, access);
    }
    let Ok(resource_id) = normalize_resource_location(raw_id.to_string()) else {
        return;
    };
    let membership = if tagged {
        access.tag_membership("minecraft:block", &resource_id)
    } else {
        access.resource_membership("minecraft:block", &resource_id)
    };
    if membership != FrozenRegistryMembership::Present {
        return;
    }
    let property_values = block_property_values(access, tagged, &resource_id);
    let has_block_entity = block_has_block_entity(access, tagged, &resource_id);
    let Some(properties) = properties else {
        if !property_values.is_empty() {
            output.entry(format!("{prefix}{raw_id}[")).or_default();
        }
        if has_block_entity {
            output.entry(format!("{prefix}{raw_id}{{")).or_default();
        }
        return;
    };
    let base = format!("{prefix}{raw_id}[");
    let Some((body, suffix)) = properties.split_once(']') else {
        push_property_candidates(output, &base, properties, &property_values);
        return;
    };
    if suffix.is_empty() && has_block_entity {
        output.entry(format!("{base}{body}]{{")).or_default();
    }
}

fn push_property_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    base: &str,
    body: &str,
    property_values: &BTreeMap<&str, BTreeSet<&str>>,
) {
    let (completed, current) = body.rsplit_once(',').map_or(("", body), |parts| parts);
    let mut visited = completed
        .split(',')
        .filter_map(|property| property.trim().split_once('=').map(|(name, _)| name.trim()))
        .collect::<BTreeSet<_>>();
    let current = current.trim_start();
    let current_prefix = if completed.is_empty() {
        base.to_string()
    } else {
        format!("{base}{completed},")
    };
    let Some((name, value)) = current.split_once('=') else {
        if current.is_empty() {
            output.entry(format!("{current_prefix}]")).or_default();
        }
        for property in property_values.keys() {
            if !visited.contains(property) && property.starts_with(current) {
                output
                    .entry(format!("{current_prefix}{property}="))
                    .or_default();
            }
        }
        return;
    };
    let name = name.trim();
    let value = value.trim_start();
    let Some(values) = property_values.get(name) else {
        return;
    };
    for candidate in values {
        if candidate.starts_with(value) {
            output
                .entry(format!("{current_prefix}{name}={candidate}"))
                .or_default();
        }
    }
    if values.contains(value) {
        visited.insert(name);
        let complete = format!("{current_prefix}{name}={value}");
        output.entry(format!("{complete}]")).or_default();
        if property_values
            .keys()
            .any(|property| !visited.contains(property))
        {
            output.entry(format!("{complete},")).or_default();
        }
    }
}

fn block_property_values<'a>(
    access: FrozenRegistryAccess<'a>,
    tagged: bool,
    resource_id: &str,
) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
    if !tagged {
        return access.block_property_values(resource_id);
    }
    let mut result = BTreeMap::<&str, BTreeSet<&str>>::new();
    for block in access
        .resource_ids_for_tag("minecraft:block", resource_id)
        .unwrap_or_default()
    {
        for (property, values) in access.block_property_values(block) {
            result.entry(property).or_default().extend(values);
        }
    }
    result
}

fn block_has_block_entity(
    access: FrozenRegistryAccess<'_>,
    tagged: bool,
    resource_id: &str,
) -> bool {
    let resources = if tagged {
        access
            .resource_ids_for_tag("minecraft:block", resource_id)
            .unwrap_or_default()
    } else {
        vec![resource_id]
    };
    resources.into_iter().any(|block| {
        vanilla_block_type_for_resource_id(block)
            .is_some_and(|entry| entry.block_entity_type_id.is_some())
    })
}
