use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{
    entity_types::vanilla_entity_type_for_resource_id, game_types::VANILLA_GAME_TYPES,
    read_command_string_value_utf16, scan_command_degrees_range_argument_utf16,
    scan_command_number_lexeme_utf16, scan_command_range_argument_utf16,
    scan_command_resource_location_utf16, scan_command_snbt_argument_utf16,
    scan_command_unquoted_string_utf16, scan_command_whitespace_utf16, CommandEntitySelectorOption,
    CommandEntitySelectorOptionKind, CommandSnbtKind, JavaString,
    VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};

use super::{
    ascii_text, lower_str, lower_units, push_resource_candidates, push_string_candidates, unit_at,
    units_eq_ascii, CandidateText, CommandCompletionSources, CompletionMetadata,
};

/// `@` plus one selector letter: the units every option candidate repeats.
const SELECTOR_LEN: usize = 2;

const SORT_ORDERS: [&str; 4] = ["nearest", "furthest", "random", "arbitrary"];

pub(super) fn push_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    units: &[u16],
    players_only: bool,
    sources: &CommandCompletionSources<'_>,
) {
    let Some(remaining) = units.strip_prefix(&[u16::from(b'@')]) else {
        push_string_candidates(output, units, sources.online_player_names.iter().copied());
        if !players_only {
            if let Some(uuid) = sources.targeted_entity_uuid {
                let uuid_text = uuid.to_string();
                push_string_candidates(output, units, [uuid_text.as_str()]);
            }
        }
        // `parseNameOrUUID` (EntitySelectorParser.java:261-263) replaces
        // `parse`'s own `suggestNameOrSelector` (:457-459) only while the
        // reader can read, so an empty tail keeps the selector letters
        // `fillSelectorSuggestions` (:475-481) adds beside the names.
        if units.is_empty() {
            push_string_candidates(output, units, ["@p", "@a", "@r", "@s", "@e", "@n"]);
        }
        return;
    };
    let Some(selector_type) = remaining
        .first()
        .copied()
        .filter(|unit| is_selector_type(*unit))
    else {
        push_string_candidates(output, units, ["@p", "@a", "@r", "@s", "@e", "@n"]);
        return;
    };
    // A bare `@x`: the option list has not been opened.
    if remaining.len() == SELECTOR_LEN - 1 {
        output
            .entry(CandidateText::units(units).then_unit(b'[').finish())
            .or_default();
        return;
    }
    if !unit_at(remaining, 1, b'[') {
        return;
    }
    let scan = scan_options(
        units,
        SELECTOR_LEN + 1,
        selector_type as u8,
        sources.registry_access.content_registry(),
    );
    let repeated = CandidateText::units(&units[..scan.cursor]);
    let remaining = &units[scan.cursor..];
    match scan.state {
        OptionState::Nothing => {}
        OptionState::Key => {
            // `EntitySelectorOptions.suggestNames` (:501-509) lowercases both the
            // typed key and the option's own name, so `@e[SORT` answers `sort=`.
            let typed = lower_units(remaining);
            for option in VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS {
                if scan.availability.option_applicable(option.name)
                    && lower_str(option.name).units().starts_with(typed.units())
                {
                    push_candidate(output, &repeated, remaining, &format!("{}=", option.name));
                }
            }
        }
        OptionState::NextOrClose => {
            push_candidate(output, &repeated, remaining, ",");
            push_candidate(output, &repeated, remaining, "]");
        }
        OptionState::Sort => {
            let mut nested = BTreeMap::new();
            push_string_candidates(&mut nested, remaining, SORT_ORDERS);
            push_prefixed_candidates(output, &repeated, remaining, nested);
        }
        OptionState::Gamemode => {
            push_gamemode_candidates(output, &repeated, remaining, &scan.availability);
        }
        OptionState::Type => {
            push_entity_type_candidates(output, &repeated, remaining, &scan.availability, sources);
        }
    }
}

/// The `@` is already stripped, so one selector letter is unit one of `units`.
fn is_selector_type(unit: u16) -> bool {
    unit < 0x80 && matches!(unit as u8, b'p' | b'a' | b'r' | b's' | b'e' | b'n')
}

/// The suggestion function `EntitySelectorParser.parseOptions` (:285-325) left
/// behind, whether it returned or a modifier threw.
enum OptionState {
    /// `suggestOptionsKey` (:286, :315, :517-520): the applicable option names.
    Key,
    /// `SUGGEST_NOTHING` (:71, :302, :321).
    Nothing,
    /// `suggestOptionsNextOrClose` (:305, :522-526): `,` and `]`.
    NextOrClose,
    /// `sort`'s own provider (`EntitySelectorOptions.java:179`).
    Sort,
    /// `gamemode`'s own provider (`EntitySelectorOptions.java:194-220`).
    Gamemode,
    /// `type`'s own provider (`EntitySelectorOptions.java:265-274`).
    Type,
}

struct OptionScan {
    /// Where the jar creates its offset: the reader's own cursor (:541-543).
    cursor: usize,
    state: OptionState,
    availability: SelectorCompletionState,
}

/// Where one modifier gave up, and the provider it had installed by then --
/// the `SUGGEST_NOTHING` the loop set before calling it
/// (`EntitySelectorParser.java:302`) unless the modifier replaced it.
struct ValueStop {
    cursor: usize,
    state: OptionState,
}

#[derive(Default)]
struct SelectorCompletionState {
    selector: u8,
    seen_once: BTreeSet<&'static str>,
    name_equals: bool,
    name_not_equals: bool,
    limited: bool,
    sorted: bool,
    gamemode_equals: bool,
    gamemode_not_equals: bool,
    team_equals: bool,
    type_limited: bool,
    type_inverse: bool,
}

/// `EntitySelectorParser.parseOptions` replayed over the buffer's own units.
/// Every option value is read by that option's own modifier, so a value one of
/// them rejects stops the replay on its provider -- `SUGGEST_NOTHING` for all
/// but three -- instead of reaching the next option name.
fn scan_options(
    units: &[u16],
    start: usize,
    selector: u8,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> OptionScan {
    let mut availability = SelectorCompletionState {
        selector,
        type_limited: matches!(selector, b'p' | b'a' | b'r'),
        ..SelectorCompletionState::default()
    };
    let mut state = OptionState::Key;
    let mut cursor = scan_command_whitespace_utf16(units, start);
    while cursor < units.len() && !unit_at(units, cursor, b']') {
        cursor = scan_command_whitespace_utf16(units, cursor);
        let key_start = cursor;
        let (key_end, key) = match read_command_string_value_utf16(units, cursor) {
            Ok(read) => read,
            Err(error) => return stopped(error.cursor.min(units.len()), state, availability),
        };
        let Some(option) = ascii_text(key.units()).and_then(|key| option_named(&key)) else {
            // `EntitySelectorOptions.get` rewinds to the key start (:496-497).
            return stopped(key_start, state, availability);
        };
        if !availability.option_applicable(option.name) {
            // `EntitySelectorOptions.get` reports at the cursor instead (:493).
            return stopped(key_end, state, availability);
        }
        cursor = scan_command_whitespace_utf16(units, key_end);
        if !unit_at(units, cursor, b'=') {
            // `ERROR_EXPECTED_OPTION_VALUE` (:295-298) rewinds to the key start.
            return stopped(key_start, state, availability);
        }
        cursor = scan_command_whitespace_utf16(units, cursor + 1);
        match read_option_value(units, cursor, option, &mut availability, content_registry) {
            Ok(end) => cursor = end,
            Err(value) => return stopped(value.cursor, value.state, availability),
        }
        cursor = scan_command_whitespace_utf16(units, cursor);
        state = OptionState::NextOrClose;
        if cursor < units.len() {
            if !unit_at(units, cursor, b',') {
                if !unit_at(units, cursor, b']') {
                    // `ERROR_EXPECTED_END_OF_OPTIONS` (:309) reports at the cursor.
                    return stopped(cursor, state, availability);
                }
                break;
            }
            cursor += 1;
            state = OptionState::Key;
        }
    }
    if cursor < units.len() {
        // The reader is on `]`; consuming it leaves no provider (:319-321).
        return stopped(cursor + 1, OptionState::Nothing, availability);
    }
    stopped(cursor, state, availability)
}

/// One modifier's value syntax (`EntitySelectorOptions.java:90-483`), which is
/// the only reader of that option's value. Only `sort`, `gamemode` and `type`
/// install a provider of their own; the rest fail into the `SUGGEST_NOTHING`
/// `parseOptions` set before calling them.
fn read_option_value(
    units: &[u16],
    start: usize,
    option: &'static CommandEntitySelectorOption,
    availability: &mut SelectorCompletionState,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<usize, ValueStop> {
    let nothing = |cursor: usize| ValueStop {
        cursor,
        state: OptionState::Nothing,
    };
    match option.name {
        "name" => {
            let (value_start, inverted) = read_inversion(units, start);
            let (end, _) = read_command_string_value_utf16(units, value_start)
                .map_err(|error| nothing(error.cursor.min(units.len())))?;
            if availability.name_not_equals && !inverted {
                return Err(nothing(start));
            }
            if inverted {
                availability.name_not_equals = true;
            } else {
                availability.name_equals = true;
            }
            Ok(end)
        }
        "distance" | "level" => {
            let integer = option.name == "level";
            let end = scan_command_range_argument_utf16(units, start, integer)
                .map_err(|_| nothing(start))?;
            // `:110-116` and `:121-127`: a negative bound rewinds to the value.
            if range_is_negative(&units[start..end], integer) {
                return Err(nothing(start));
            }
            availability.seen_once.insert(option.name);
            Ok(end)
        }
        "x" | "y" | "z" | "dx" | "dy" | "dz" => {
            let (end, raw) =
                scan_command_number_lexeme_utf16(units, start).map_err(|_| nothing(start))?;
            ascii_text(raw)
                .and_then(|raw| raw.parse::<f64>().ok())
                .ok_or_else(|| nothing(start))?;
            availability.seen_once.insert(option.name);
            Ok(end)
        }
        "x_rotation" | "y_rotation" => {
            let end = scan_command_degrees_range_argument_utf16(units, start)
                .map_err(|_| nothing(start))?;
            availability.seen_once.insert(option.name);
            Ok(end)
        }
        "limit" => {
            let (end, raw) =
                scan_command_number_lexeme_utf16(units, start).map_err(|_| nothing(start))?;
            let count = ascii_text(raw)
                .and_then(|raw| raw.parse::<i32>().ok())
                .ok_or_else(|| nothing(start))?;
            if count < 1 {
                return Err(nothing(start));
            }
            availability.limited = true;
            Ok(end)
        }
        "sort" => {
            // The provider is installed after the word is read and before the
            // switch rejects it (:178-190), so every rejection answers here.
            let end = scan_command_unquoted_string_utf16(units, start);
            let stop = ValueStop {
                cursor: start,
                state: OptionState::Sort,
            };
            if !SORT_ORDERS
                .iter()
                .any(|order| units_eq_ascii(&units[start..end], order))
            {
                return Err(stop);
            }
            availability.sorted = true;
            Ok(end)
        }
        "gamemode" => {
            // `setSuggestions` (:194) runs before anything is read.
            let stop = ValueStop {
                cursor: start,
                state: OptionState::Gamemode,
            };
            let (value_start, inverted) = read_inversion(units, start);
            if availability.gamemode_not_equals && !inverted {
                return Err(stop);
            }
            let end = scan_command_unquoted_string_utf16(units, value_start);
            if !VANILLA_GAME_TYPES
                .iter()
                .any(|entry| units_eq_ascii(&units[value_start..end], entry.serialized_name))
            {
                return Err(stop);
            }
            if inverted {
                availability.gamemode_not_equals = true;
            } else {
                availability.gamemode_equals = true;
            }
            Ok(end)
        }
        "team" => {
            let (value_start, inverted) = read_inversion(units, start);
            if !inverted {
                availability.team_equals = true;
            }
            Ok(scan_command_unquoted_string_utf16(units, value_start))
        }
        "tag" => {
            let (value_start, _) = read_inversion(units, start);
            Ok(scan_command_unquoted_string_utf16(units, value_start))
        }
        "type" => {
            let stop = ValueStop {
                cursor: start,
                state: OptionState::Type,
            };
            let (value_start, inverted) = read_inversion(units, start);
            if availability.type_inverse && !inverted {
                return Err(stop);
            }
            if inverted {
                // `setTypeLimitedInversely` (:282-284) runs before the id is
                // read, so a rejected value still narrows the provider.
                availability.type_inverse = true;
            }
            let (id_start, tagged) = read_tag_prefix(units, value_start);
            let (end, resource_id) = scan_command_resource_location_utf16(units, id_start)
                .map_err(|error| ValueStop {
                    cursor: error.cursor,
                    state: OptionState::Type,
                })?;
            if tagged {
                return Ok(end);
            }
            // `BuiltInRegistries.ENTITY_TYPE.getOptional` (:291-294), plus custom
            // session `ContentRegistry` entity types.
            if vanilla_entity_type_for_resource_id(&resource_id).is_none()
                && (content_registry.is_vanilla_only()
                    || content_registry
                        .entity_by_resource_id(&resource_id)
                        .is_none())
            {
                return Err(stop);
            }
            if !inverted {
                availability.type_limited = true;
            }
            Ok(end)
        }
        "nbt" => {
            let (value_start, _) = read_inversion(units, start);
            scan_command_snbt_argument_utf16(units, value_start, CommandSnbtKind::Compound)
                .map_err(|error| nothing(error.cursor))
        }
        "predicate" => {
            let (value_start, _) = read_inversion(units, start);
            scan_command_resource_location_utf16(units, value_start)
                .map(|(end, _)| end)
                .map_err(|error| nothing(error.cursor))
        }
        "scores" => {
            let mut cursor = expect_unit(units, start, b'{').ok_or_else(|| nothing(start))?;
            cursor = scan_command_whitespace_utf16(units, cursor);
            while cursor < units.len() && !unit_at(units, cursor, b'}') {
                cursor = scan_command_whitespace_utf16(units, cursor);
                cursor = scan_command_unquoted_string_utf16(units, cursor);
                cursor = scan_command_whitespace_utf16(units, cursor);
                cursor = expect_unit(units, cursor, b'=').ok_or_else(|| nothing(cursor))?;
                cursor = scan_command_whitespace_utf16(units, cursor);
                cursor = scan_command_range_argument_utf16(units, cursor, true)
                    .map_err(|error| nothing(error.cursor))?;
                cursor = scan_command_whitespace_utf16(units, cursor);
                if unit_at(units, cursor, b',') {
                    cursor += 1;
                }
            }
            let end = expect_unit(units, cursor, b'}').ok_or_else(|| nothing(cursor))?;
            availability.seen_once.insert(option.name);
            Ok(end)
        }
        "advancements" => {
            let mut cursor = expect_unit(units, start, b'{').ok_or_else(|| nothing(start))?;
            cursor = scan_command_whitespace_utf16(units, cursor);
            while cursor < units.len() && !unit_at(units, cursor, b'}') {
                cursor = scan_command_whitespace_utf16(units, cursor);
                cursor = scan_command_resource_location_utf16(units, cursor)
                    .map(|(end, _)| end)
                    .map_err(|error| nothing(error.cursor))?;
                cursor = scan_command_whitespace_utf16(units, cursor);
                cursor = expect_unit(units, cursor, b'=').ok_or_else(|| nothing(cursor))?;
                cursor = scan_command_whitespace_utf16(units, cursor);
                cursor = if unit_at(units, cursor, b'{') {
                    scan_command_whitespace_utf16(units, read_criteria_map(units, cursor)?)
                } else {
                    read_boolean_value(units, cursor)?
                };
                cursor = scan_command_whitespace_utf16(units, cursor);
                if unit_at(units, cursor, b',') {
                    cursor += 1;
                }
            }
            let end = expect_unit(units, cursor, b'}').ok_or_else(|| nothing(cursor))?;
            availability.seen_once.insert(option.name);
            Ok(end)
        }
        // The catalog's other spellings are all above; an added one fails
        // closed rather than skipping a value the parse would have read.
        _ => Err(nothing(start)),
    }
}

fn stopped(cursor: usize, state: OptionState, availability: SelectorCompletionState) -> OptionScan {
    OptionScan {
        cursor,
        state,
        availability,
    }
}

/// The criterion map `advancements` reads for one id (`:389-408`).
fn read_criteria_map(units: &[u16], start: usize) -> Result<usize, ValueStop> {
    let nothing = |cursor: usize| ValueStop {
        cursor,
        state: OptionState::Nothing,
    };
    let mut cursor = expect_unit(units, start, b'{').ok_or_else(|| nothing(start))?;
    cursor = scan_command_whitespace_utf16(units, cursor);
    while cursor < units.len() && !unit_at(units, cursor, b'}') {
        cursor = scan_command_whitespace_utf16(units, cursor);
        cursor = scan_command_unquoted_string_utf16(units, cursor);
        cursor = scan_command_whitespace_utf16(units, cursor);
        cursor = expect_unit(units, cursor, b'=').ok_or_else(|| nothing(cursor))?;
        cursor = scan_command_whitespace_utf16(units, cursor);
        cursor = read_boolean_value(units, cursor)?;
        cursor = scan_command_whitespace_utf16(units, cursor);
        if unit_at(units, cursor, b',') {
            cursor += 1;
        }
    }
    cursor = scan_command_whitespace_utf16(units, cursor);
    expect_unit(units, cursor, b'}').ok_or_else(|| nothing(cursor))
}

/// `StringReader.readBoolean`: a `readString` that has to spell one of the two.
fn read_boolean_value(units: &[u16], start: usize) -> Result<usize, ValueStop> {
    let nothing = |cursor: usize| ValueStop {
        cursor,
        state: OptionState::Nothing,
    };
    let (end, value) = read_command_string_value_utf16(units, start)
        .map_err(|error| nothing(error.cursor.min(units.len())))?;
    if ["true", "false"]
        .iter()
        .any(|spelling| units_eq_ascii(value.units(), spelling))
    {
        Ok(end)
    } else {
        Err(nothing(start))
    }
}

/// `EntitySelectorParser.shouldInvertValue` (:327-336).
fn read_inversion(units: &[u16], start: usize) -> (usize, bool) {
    read_marker(units, start, b'!')
}

/// `EntitySelectorParser.isTag` (:338-347).
fn read_tag_prefix(units: &[u16], start: usize) -> (usize, bool) {
    read_marker(units, start, b'#')
}

fn read_marker(units: &[u16], start: usize, byte: u8) -> (usize, bool) {
    let cursor = scan_command_whitespace_utf16(units, start);
    if unit_at(units, cursor, byte) {
        (scan_command_whitespace_utf16(units, cursor + 1), true)
    } else {
        (cursor, false)
    }
}

/// `StringReader.expect`.
fn expect_unit(units: &[u16], cursor: usize, byte: u8) -> Option<usize> {
    unit_at(units, cursor, byte).then_some(cursor + 1)
}

/// The two bounds `MinMaxBounds.Bounds.fromReader` (`MinMaxBounds.java:139-168`)
/// built, read back out of the span it accepted: one lexeme, or two around the
/// `..` that `isAllowedInputChar` (`:191-198`) refuses to consume.
fn range_bounds(span: &[u16]) -> (&[u16], &[u16]) {
    let dots = [u16::from(b'.'); 2];
    span.windows(2)
        .position(|window| window == dots)
        .map_or((span, span), |index| (&span[..index], &span[index + 2..]))
}

fn range_is_negative(span: &[u16], integer: bool) -> bool {
    let (min, max) = range_bounds(span);
    [min, max].into_iter().any(|lexeme| {
        ascii_text(lexeme).is_some_and(|raw| {
            if integer {
                raw.parse::<i32>().is_ok_and(|value| value < 0)
            } else {
                raw.parse::<f64>().is_ok_and(|value| value < 0.0)
            }
        })
    })
}

fn option_named(name: &str) -> Option<&'static CommandEntitySelectorOption> {
    VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS
        .iter()
        .find(|option| option.name == name)
}

impl SelectorCompletionState {
    /// The `canUse` predicate each option was registered with
    /// (`EntitySelectorOptions.java:90-483`), read by `get` (:487-499) and by
    /// `suggestNames` (:501-509).
    fn option_applicable(&self, name: &str) -> bool {
        let Some(option) = option_named(name) else {
            return false;
        };
        match option.kind {
            CommandEntitySelectorOptionKind::Name => !self.name_equals,
            CommandEntitySelectorOptionKind::Once => !self.seen_once.contains(option.name),
            CommandEntitySelectorOptionKind::Limit => self.selector != b's' && !self.limited,
            CommandEntitySelectorOptionKind::Sort => self.selector != b's' && !self.sorted,
            CommandEntitySelectorOptionKind::Gamemode => !self.gamemode_equals,
            CommandEntitySelectorOptionKind::Team => !self.team_equals,
            CommandEntitySelectorOptionKind::Type => !self.type_limited,
            CommandEntitySelectorOptionKind::Repeatable => true,
        }
    }
}

fn push_gamemode_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    repeated: &CandidateText,
    remaining: &[u16],
    availability: &SelectorCompletionState,
) {
    // `EntitySelectorOptions.java:194-220`: the lowered remainder is a prefix,
    // and its own leading `!` decides which of the two spellings is offered.
    let lowered = lower_units(remaining);
    let mut prefix = lowered.units();
    let mut normal = !availability.gamemode_not_equals;
    let mut inverted = true;
    if let Some((first, rest)) = prefix.split_first() {
        if *first == u16::from(b'!') {
            normal = false;
            prefix = rest;
        } else {
            inverted = false;
        }
    }
    for entry in VANILLA_GAME_TYPES {
        let name = entry.serialized_name;
        if !lower_str(name).units().starts_with(prefix) {
            continue;
        }
        if inverted {
            push_candidate(output, repeated, remaining, &format!("!{name}"));
        }
        if normal {
            push_candidate(output, repeated, remaining, name);
        }
    }
}

fn push_entity_type_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    repeated: &CandidateText,
    remaining: &[u16],
    availability: &SelectorCompletionState,
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
    if !availability.type_inverse {
        push_resource_candidates(&mut nested, remaining, entity_types.iter().copied(), "");
        push_resource_candidates(&mut nested, remaining, tags.iter().copied(), "#");
    }
    push_prefixed_candidates(output, repeated, remaining, nested);
}

/// `SuggestionsBuilder.suggest` drops the candidate whose text equals the
/// remainder the range covers, and every provider here runs through it.
fn push_candidate(
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

fn push_prefixed_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    repeated: &CandidateText,
    remaining: &[u16],
    nested: BTreeMap<JavaString, CompletionMetadata>,
) {
    for candidate in nested.into_keys() {
        if candidate.units() == remaining {
            continue;
        }
        output
            .entry(repeated.clone().then_units(candidate.units()).finish())
            .or_default();
    }
}
