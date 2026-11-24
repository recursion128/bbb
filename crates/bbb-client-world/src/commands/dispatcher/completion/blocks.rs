use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{
    block_types::vanilla_block_type_for_resource_id, java_character_is_whitespace,
    normalize_resource_location, read_command_string_value_utf16, JavaString,
};

use super::{
    ascii_text, contains_unit, lower_str, lower_units, push_registry_candidates, split_once_unit,
    starts_with_unit, unit_at, units_eq_ascii, CandidateText, CompletionMetadata,
};
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn push_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    units: &[u16],
    include_tags: bool,
    access: FrozenRegistryAccess<'_>,
) {
    let (head, properties) = split_once_unit(units, b'[')
        .map_or((units, None), |(head, properties)| (head, Some(properties)));
    if contains_unit(head, b'{') {
        return;
    }
    let tagged = starts_with_unit(head, b'#');
    if tagged && !include_tags {
        return;
    }
    let raw_id = if tagged { &head[1..] } else { head };
    let prefix = if tagged { "#" } else { "" };
    // `parse` (:143-164) leaves the id list live only while the tail's own id
    // did not resolve: a successful `readBlock` or `readTag` replaces the
    // provider with `suggestOpenPropertiesOrNbt` (:326-338) or
    // `suggestOpenVaguePropertiesOrNbt` (:300-324), while `ERROR_UNKNOWN_BLOCK`
    // (:354-363), `ERROR_UNKNOWN_TAG` (:365-378) and an unreadable id all rewind
    // to the id start, where `suggestItem` (:344-346) or `suggestTag`
    // (:340-342) still answers the whole tail.
    let resolved = ascii_text(raw_id)
        .and_then(|raw_id| normalize_resource_location(raw_id).ok())
        .filter(|resource_id| {
            let membership = if tagged {
                access.tag_membership("minecraft:block", resource_id)
            } else {
                access.resource_membership("minecraft:block", resource_id)
            };
            membership == FrozenRegistryMembership::Present
        });
    let Some(resource_id) = resolved else {
        push_registry_candidates(output, units, "minecraft:block", include_tags, access);
        return;
    };
    let property_values = block_property_values(access, tagged, &resource_id);
    let has_block_entity = block_has_block_entity(access, tagged, &resource_id);
    let Some(properties) = properties else {
        if !property_values.is_empty() {
            output
                .entry(
                    CandidateText::ascii(prefix)
                        .then_units(raw_id)
                        .then_unit(b'[')
                        .finish(),
                )
                .or_default();
        }
        if has_block_entity {
            output
                .entry(
                    CandidateText::ascii(prefix)
                        .then_units(raw_id)
                        .then_unit(b'{')
                        .finish(),
                )
                .or_default();
        }
        return;
    };
    let base = CandidateText::ascii(prefix)
        .then_units(raw_id)
        .then_unit(b'[');
    push_property_candidates(
        output,
        &base,
        properties,
        &property_values,
        tagged,
        has_block_entity,
    );
}

/// The state `BlockStateParser.readProperties` (:380-431) or
/// `readVagueProperties` (:433-484) left its suggestion function in.
enum PropertyState {
    /// `suggestPropertyNameOrEnd` (:172-178, :382, :435): `]`, then the names.
    NameOrEnd,
    /// `suggestPropertyName` (:188-198, :422, :471): the names.
    Name,
    /// `suggestEquals` (:239-245, :401): `=`.
    Equals,
    /// `addSuggestions` (:259-269, :408): every value of the property.
    Values(JavaString),
    /// `suggestVaguePropertyValue` (:271-298, :456): the values of the typed
    /// key, then `,` and `]`.
    VagueValues(JavaString),
    /// `suggestNextPropertyOrEnd` (:247-257, :411): `]`, and `,` while the
    /// definition still has an unread property.
    NextOrEnd,
    /// `suggestOpenNbt` (:215-221): `{`.
    OpenNbt,
}

struct PropertyScan {
    /// Where the jar creates its offset: the reader's own cursor.
    cursor: usize,
    consumed: BTreeSet<JavaString>,
    state: PropertyState,
}

/// `BlockStateParser.readProperties` and `readVagueProperties` replayed over the
/// buffer's own units. Every suggestion is answered from the offset the parse
/// reached, so the cursor decides the repeated text and the value layer: a
/// value the property rejects rewinds to the value start (:490-499), which
/// replaces the rest of the buffer, and `addSuggestions` never reads the
/// remainder it is answering for.
fn scan_properties(
    body: &[u16],
    property_values: &BTreeMap<&str, BTreeSet<&str>>,
    tagged: bool,
) -> PropertyScan {
    let mut consumed = BTreeSet::new();
    let mut state = PropertyState::NameOrEnd;
    let mut cursor = 0;
    // `readVagueProperties` rewinds a value it left unfinished (:457, :478-480).
    let mut unfinished_value = None::<usize>;
    loop {
        if cursor >= body.len() || unit_at(body, cursor, b']') {
            break;
        }
        cursor = skip_whitespace_units(body, cursor);
        let key_start = cursor;
        let (key_end, key) = match read_command_string_value_utf16(body, cursor) {
            Ok(read) => read,
            Err(error) => {
                return PropertyScan {
                    cursor: error.cursor.min(body.len()),
                    consumed,
                    state,
                }
            }
        };
        cursor = key_end;
        if consumed.contains(&key) || (!tagged && !is_property_name(property_values, &key)) {
            // `ERROR_UNKNOWN_PROPERTY` (:388-393) and `ERROR_DUPLICATE_PROPERTY`
            // (:395-398, :443-446) both report at the key start.
            return PropertyScan {
                cursor: key_start,
                consumed,
                state,
            };
        }
        cursor = skip_whitespace_units(body, cursor);
        if !unit_at(body, cursor, b'=') {
            // `readProperties` reports at the cursor it reached (:402-404),
            // `readVagueProperties` rewinds to the key start (:449-452).
            return PropertyScan {
                cursor: if tagged { key_start } else { cursor },
                consumed,
                state: if tagged { state } else { PropertyState::Equals },
            };
        }
        cursor += 1;
        cursor = skip_whitespace_units(body, cursor);
        let value_start = cursor;
        let (value_end, value) = match read_command_string_value_utf16(body, cursor) {
            Ok(read) => read,
            Err(error) => {
                return PropertyScan {
                    cursor: error.cursor.min(body.len()),
                    consumed,
                    state: if tagged {
                        PropertyState::VagueValues(key)
                    } else {
                        PropertyState::Values(key)
                    },
                }
            }
        };
        cursor = value_end;
        if tagged {
            consumed.insert(key.clone());
            state = PropertyState::VagueValues(key);
            unfinished_value = Some(value_start);
        } else {
            if !property_accepts(property_values, &key, &value) {
                return PropertyScan {
                    cursor: value_start,
                    consumed,
                    state: PropertyState::Values(key),
                };
            }
            consumed.insert(key);
            state = PropertyState::NextOrEnd;
        }
        cursor = skip_whitespace_units(body, cursor);
        if cursor < body.len() {
            unfinished_value = None;
            if !unit_at(body, cursor, b',') {
                if !unit_at(body, cursor, b']') {
                    return PropertyScan {
                        cursor,
                        consumed,
                        state,
                    };
                }
                break;
            }
            cursor += 1;
            state = PropertyState::Name;
        }
    }
    if cursor < body.len() {
        // The reader is on `]`; both exits consume it, and `parse` (:154-163)
        // then suggests the NBT body.
        return PropertyScan {
            cursor: cursor + 1,
            consumed,
            state: PropertyState::OpenNbt,
        };
    }
    if let Some(value_start) = unfinished_value {
        cursor = value_start;
    }
    PropertyScan {
        cursor,
        consumed,
        state,
    }
}

fn push_property_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    base: &CandidateText,
    body: &[u16],
    property_values: &BTreeMap<&str, BTreeSet<&str>>,
    tagged: bool,
    has_block_entity: bool,
) {
    let scan = scan_properties(body, property_values, tagged);
    let repeated = base.clone().then_units(&body[..scan.cursor]);
    let remaining = &body[scan.cursor..];
    match scan.state {
        PropertyState::NameOrEnd | PropertyState::Name => {
            if matches!(scan.state, PropertyState::NameOrEnd) && remaining.is_empty() {
                push_suggestion(output, &repeated, remaining, "]");
            }
            let prefix = lower_units(remaining);
            for property in property_values.keys() {
                if !is_consumed(&scan.consumed, property)
                    && lower_str(property).units().starts_with(prefix.units())
                {
                    output
                        .entry(
                            repeated
                                .clone()
                                .then_ascii(property)
                                .then_unit(b'=')
                                .finish(),
                        )
                        .or_default();
                }
            }
        }
        PropertyState::Equals => {
            if remaining.is_empty() {
                push_suggestion(output, &repeated, remaining, "=");
            }
        }
        PropertyState::Values(key) => {
            for value in values_of(property_values, &key).into_iter().flatten() {
                push_suggestion(output, &repeated, remaining, value);
            }
        }
        PropertyState::VagueValues(key) => {
            for value in values_of(property_values, &key).into_iter().flatten() {
                push_suggestion(output, &repeated, remaining, value);
            }
            if property_values
                .keys()
                .any(|property| !is_consumed(&scan.consumed, property))
            {
                push_suggestion(output, &repeated, remaining, ",");
            }
            push_suggestion(output, &repeated, remaining, "]");
        }
        PropertyState::NextOrEnd => {
            if remaining.is_empty() {
                push_suggestion(output, &repeated, remaining, "]");
                if scan.consumed.len() < property_values.len() {
                    push_suggestion(output, &repeated, remaining, ",");
                }
            }
        }
        PropertyState::OpenNbt => {
            if remaining.is_empty() && has_block_entity {
                push_suggestion(output, &repeated, remaining, "{");
            }
        }
    }
}

/// `SuggestionsBuilder.suggest(String)` keeps a candidate only when its text
/// differs from the remainder the range covers, so the buffer the suggestion
/// repeats verbatim is never offered back.
fn push_suggestion(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    repeated: &CandidateText,
    remaining: &[u16],
    text: &str,
) {
    if units_eq_ascii(remaining, text) {
        return;
    }
    output
        .entry(repeated.clone().then_ascii(text).finish())
        .or_default();
}

fn values_of<'map, 'value>(
    property_values: &'map BTreeMap<&'value str, BTreeSet<&'value str>>,
    key: &JavaString,
) -> Option<&'map BTreeSet<&'value str>> {
    property_values
        .iter()
        .find_map(|(property, values)| units_eq_ascii(key.units(), property).then_some(values))
}

fn is_property_name(property_values: &BTreeMap<&str, BTreeSet<&str>>, key: &JavaString) -> bool {
    values_of(property_values, key).is_some()
}

fn is_consumed(consumed: &BTreeSet<JavaString>, property: &str) -> bool {
    consumed
        .iter()
        .any(|key| units_eq_ascii(key.units(), property))
}

/// `setValue` (:490-499) asks the definition's own `Property.getValue`, so the
/// acceptance test is the definition owner's.
fn property_accepts(
    property_values: &BTreeMap<&str, BTreeSet<&str>>,
    key: &JavaString,
    value: &JavaString,
) -> bool {
    values_of(property_values, key)
        .is_some_and(|values| super::super::blocks::property_accepts_value(values, value.units()))
}

fn skip_whitespace_units(units: &[u16], mut cursor: usize) -> usize {
    while units
        .get(cursor)
        .is_some_and(|unit| java_character_is_whitespace(*unit))
    {
        cursor += 1;
    }
    cursor
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
