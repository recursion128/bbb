use bbb_protocol::{
    chat_formattings::VANILLA_CHAT_FORMATTINGS,
    entity_anchors::VANILLA_ENTITY_ANCHORS,
    game_types::vanilla_game_type_for_serialized_name,
    heightmap_types::{HeightmapUsage, VANILLA_HEIGHTMAP_TYPES},
    normalize_resource_location,
    packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID, VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
    },
    scoreboard_display_slots::VANILLA_SCOREBOARD_DISPLAY_SLOTS,
};

use super::{
    number_lexeme, parse_unquoted_string, syntax_error, CommandSyntaxErrorKind,
    CommandSyntaxErrorState, ParsedValue,
};

pub(super) fn parse(
    type_id: i32,
    command: &str,
    start: usize,
    template: &CommandArgumentTemplate,
) -> Option<Result<ParsedValue, CommandSyntaxErrorState>> {
    let result = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID => parse_color(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID => parse_hex_color(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID | VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID => {
            Ok(validated(parse_unquoted_string(command, start)))
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID => parse_operation(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID => parse_angle(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID => parse_scoreboard_slot(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID => parse_swizzle(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID => parse_identifier(command, start, false),
        VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID => parse_identifier(command, start, true),
        bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID => {
            parse_entity_anchor(command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID => parse_int_range(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID => parse_float_range(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID => parse_game_mode(command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID => parse_time(command, start, template),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID => parse_heightmap(command, start),
        _ => return None,
    };
    Some(result)
}

fn parse_color(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = parse_unquoted_string(command, start);
    let cleaned: String = command[start..end]
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    let valid = VANILLA_CHAT_FORMATTINGS.iter().any(|entry| {
        !entry.is_format
            && entry
                .serialized_name
                .chars()
                .filter(|character| character.is_ascii_alphabetic())
                .eq(cleaned.chars())
    });
    valid.then(|| validated(end)).ok_or_else(|| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            end,
            format!("invalid color {}", &command[start..end]),
        )
    })
}

fn parse_hex_color(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = parse_unquoted_string(command, start);
    let raw = &command[start..end];
    (matches!(raw.len(), 3 | 6) && raw.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| validated(end))
        .ok_or_else(|| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                end,
                format!("invalid hex color {raw}"),
            )
        })
}

fn parse_operation(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = token_end(command, start);
    matches!(
        &command[start..end],
        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "<" | ">" | "><"
    )
    .then(|| validated(end))
    .ok_or_else(|| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            "invalid scoreboard operation",
        )
    })
}

fn parse_angle(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let mut cursor = start;
    if command.as_bytes().get(cursor) == Some(&b'~') {
        cursor += 1;
        if command
            .as_bytes()
            .get(cursor)
            .is_none_or(|byte| *byte == b' ')
        {
            return Ok(validated(cursor));
        }
    }
    let (end, raw) = number_lexeme(command, cursor)?;
    let value = raw.parse::<f32>().map_err(|_| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidNumber,
            cursor,
            format!("invalid angle {raw}"),
        )
    })?;
    value.is_finite().then(|| validated(end)).ok_or_else(|| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            end,
            "angle must be finite",
        )
    })
}

fn parse_scoreboard_slot(
    command: &str,
    start: usize,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = parse_unquoted_string(command, start);
    let raw = &command[start..end];
    VANILLA_SCOREBOARD_DISPLAY_SLOTS
        .iter()
        .any(|entry| entry.serialized_name == raw)
        .then(|| validated(end))
        .ok_or_else(|| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                end,
                format!("invalid scoreboard display slot {raw}"),
            )
        })
}

fn parse_swizzle(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = token_end(command, start);
    let mut axes = 0_u8;
    for (offset, byte) in command.as_bytes()[start..end].iter().copied().enumerate() {
        let bit = match byte {
            b'x' => 1,
            b'y' => 2,
            b'z' => 4,
            _ => {
                return Err(syntax_error(
                    CommandSyntaxErrorKind::InvalidArgument,
                    start + offset + 1,
                    "invalid swizzle axis",
                ))
            }
        };
        if axes & bit != 0 {
            return Err(syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                start + offset + 1,
                "duplicate swizzle axis",
            ));
        }
        axes |= bit;
    }
    Ok(validated(end))
}

fn parse_identifier(
    command: &str,
    start: usize,
    allow_tag: bool,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let mut identifier_start = start;
    if allow_tag && command.as_bytes().get(identifier_start) == Some(&b'#') {
        identifier_start += 1;
    }
    let mut end = identifier_start;
    while command.as_bytes().get(end).is_some_and(|byte| {
        byte.is_ascii_digit()
            || byte.is_ascii_lowercase()
            || matches!(byte, b'_' | b':' | b'/' | b'.' | b'-')
    }) {
        end += 1;
    }
    normalize_resource_location(command[identifier_start..end].to_string())
        .map(|_| validated(end))
        .map_err(|_| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                start,
                "invalid resource location",
            )
        })
}

fn parse_entity_anchor(
    command: &str,
    start: usize,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = parse_unquoted_string(command, start);
    let raw = &command[start..end];
    VANILLA_ENTITY_ANCHORS
        .iter()
        .any(|entry| entry.serialized_name == raw)
        .then(|| validated(end))
        .ok_or_else(|| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                start,
                format!("invalid entity anchor {raw}"),
            )
        })
}

fn parse_int_range(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (end, min, max) = parse_range(command, start, |raw| raw.parse::<i32>())?;
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        return Err(syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            "range minimum exceeds maximum",
        ));
    }
    Ok(validated(end))
}

fn parse_float_range(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (end, min, max) = parse_range(command, start, |raw| raw.parse::<f64>())?;
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        return Err(syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            "range minimum exceeds maximum",
        ));
    }
    Ok(validated(end))
}

fn parse_range<T: Copy, E>(
    command: &str,
    start: usize,
    parse_number: impl Fn(&str) -> Result<T, E>,
) -> Result<(usize, Option<T>, Option<T>), CommandSyntaxErrorState> {
    if start == command.len() {
        return Err(syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            "empty range",
        ));
    }
    let (mut cursor, min) = range_number(command, start, &parse_number)?;
    let max = if command.get(cursor..cursor.saturating_add(2)) == Some("..") {
        cursor += 2;
        let (end, max) = range_number(command, cursor, &parse_number)?;
        cursor = end;
        max
    } else {
        return min
            .map(|value| (cursor, Some(value), Some(value)))
            .ok_or_else(|| {
                syntax_error(
                    CommandSyntaxErrorKind::InvalidArgument,
                    start,
                    "empty range",
                )
            });
    };
    if min.is_none() && max.is_none() {
        return Err(syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            "empty range",
        ));
    }
    Ok((cursor, min, max))
}

fn range_number<T, E>(
    command: &str,
    start: usize,
    parse_number: &impl Fn(&str) -> Result<T, E>,
) -> Result<(usize, Option<T>), CommandSyntaxErrorState> {
    let mut end = start;
    while let Some(byte) = command.as_bytes().get(end).copied() {
        let range_separator = byte == b'.' && command.as_bytes().get(end + 1) == Some(&b'.');
        if range_separator || !(byte.is_ascii_digit() || matches!(byte, b'-' | b'.')) {
            break;
        }
        end += 1;
    }
    if end == start {
        return Ok((end, None));
    }
    let raw = &command[start..end];
    parse_number(raw)
        .map(|value| (end, Some(value)))
        .map_err(|_| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidNumber,
                start,
                format!("invalid range number {raw}"),
            )
        })
}

fn parse_game_mode(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = parse_unquoted_string(command, start);
    let raw = &command[start..end];
    vanilla_game_type_for_serialized_name(raw)
        .map(|_| validated(end))
        .ok_or_else(|| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                end,
                format!("invalid game mode {raw}"),
            )
        })
}

fn parse_time(
    command: &str,
    start: usize,
    template: &CommandArgumentTemplate,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let CommandArgumentTemplate::Time { minimum } = template else {
        return Err(syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            "time argument has the wrong template",
        ));
    };
    let (number_end, raw) = number_lexeme(command, start)?;
    let value = raw.parse::<f32>().map_err(|_| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidNumber,
            start,
            format!("invalid time {raw}"),
        )
    })?;
    let end = parse_unquoted_string(command, number_end);
    let factor = match &command[number_end..end] {
        "" | "t" => 1.0,
        "s" => 20.0,
        "d" => 24_000.0,
        _ => {
            return Err(syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                end,
                "invalid time unit",
            ))
        }
    };
    let ticks = java_round_to_i32(value * factor);
    (ticks >= *minimum).then(|| validated(end)).ok_or_else(|| {
        syntax_error(
            CommandSyntaxErrorKind::NumberTooLow,
            end,
            format!("time {ticks} is below minimum {minimum}"),
        )
    })
}

fn parse_heightmap(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = parse_unquoted_string(command, start);
    let raw = &command[start..end];
    VANILLA_HEIGHTMAP_TYPES
        .iter()
        .filter(|entry| entry.usage != HeightmapUsage::Worldgen)
        .any(|entry| entry.serialization_key.to_ascii_lowercase() == raw)
        .then(|| validated(end))
        .ok_or_else(|| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                end,
                format!("invalid heightmap type {raw}"),
            )
        })
}

fn java_round_to_i32(value: f32) -> i32 {
    if value.is_nan() {
        0
    } else if value >= i32::MAX as f32 {
        i32::MAX
    } else if value <= i32::MIN as f32 {
        i32::MIN
    } else {
        (value + 0.5).floor() as i32
    }
}

fn token_end(command: &str, start: usize) -> usize {
    command[start..]
        .find(' ')
        .map_or(command.len(), |offset| start + offset)
}

fn validated(end: usize) -> ParsedValue {
    ParsedValue {
        end,
        validated: true,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID, VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
    };

    use super::*;

    #[test]
    fn validates_catalog_backed_names_without_parallel_membership_tables() {
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID, "dark-blue");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID, "bold");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, "F0a");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, "FFFF");
        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
            "sidebar.team.dark_aqua",
        );
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
            "sidebar.team.orange",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, "spectator");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, "builder");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, "ocean_floor");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, "ocean_floor_wg");
    }

    #[test]
    fn validates_identifiers_functions_operations_angles_and_swizzles() {
        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
            "example:path/value",
        );
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
            "Example:path",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID, "#example:tick");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID, "><");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID, "==");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID, "~-5.5");
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID,
            "~9999999999999999999999999999999999999999",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID, "zyx");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID, "xx");
    }

    #[test]
    fn range_and_time_grammar_matches_vanilla_boundaries() {
        for range in ["0..5", "0", "-5", "-100..", "..100"] {
            assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID, range);
        }
        for range in ["0..5.2", "-5.4", "0...2"] {
            assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID, range);
        }
        for range in ["..", "5..0", "1...2", "one"] {
            assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID, range);
        }

        assert_time_valid("1.5s", 0);
        assert_time_valid("20", 20);
        assert_time_invalid("19t", 20);
        assert_time_invalid("2m", 0);
    }

    fn assert_valid(type_id: i32, input: &str) {
        let result = parse(type_id, input, 0, &CommandArgumentTemplate::Unit)
            .expect("owned context-free parser")
            .unwrap_or_else(|error| panic!("{input}: {error:?}"));
        assert_eq!(result.end, input.len());
        assert!(result.validated);
    }

    fn assert_invalid(type_id: i32, input: &str) {
        let result = parse(type_id, input, 0, &CommandArgumentTemplate::Unit)
            .expect("owned context-free parser");
        assert!(
            result.is_err() || result.is_ok_and(|value| value.end != input.len()),
            "{input}"
        );
    }

    fn assert_time_valid(input: &str, minimum: i32) {
        let result = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
            input,
            0,
            &CommandArgumentTemplate::Time { minimum },
        )
        .expect("time parser")
        .unwrap();
        assert_eq!(result.end, input.len());
    }

    fn assert_time_invalid(input: &str, minimum: i32) {
        assert!(parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
            input,
            0,
            &CommandArgumentTemplate::Time { minimum },
        )
        .expect("time parser")
        .is_err());
    }
}
