use crate::{
    nbt_summary::{
        parse_snbt_prefix, parse_snbt_prefix_utf16, validate_snbt_compound_prefix,
        validate_snbt_prefix,
    },
    JavaString, NbtSummaryValue,
};

mod entity_selector;
mod nbt_path;
mod syntax;

pub use entity_selector::{
    CommandEntitySelectorOption, CommandEntitySelectorOptionKind,
    VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};
pub use nbt_path::{
    command_nbt_is_too_deep, merge_command_nbt_compound, parse_command_nbt_path_value_prefix,
    parse_command_nbt_path_value_prefix_utf16, CommandNbtPath, CommandNbtPathErrorKind,
    CommandNbtPathNode, CommandNbtPathNodeKind, CommandNbtPathParseError,
};
pub use syntax::*;

pub fn command_utf16_byte_cursor(input: &str, units: usize) -> usize {
    syntax::byte_cursor(input, units)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandEntitySelectorPrefix {
    pub consumed: usize,
    pub multiple: bool,
    pub includes_entities: bool,
    pub self_selector: bool,
    /// Range of the `advancements={...}` payload inside `value`, braces
    /// included, for a consumer that needs the ids this validator discards. It
    /// is measured in whatever the entry point measured `consumed` in.
    pub advancements_span: Option<(usize, usize)>,
}

pub fn parse_command_entity_selector_prefix(value: &str) -> Option<CommandEntitySelectorPrefix> {
    published(entity_selector::parse_entity_selector_prefix(value)?)
}

pub fn parse_command_entity_selector_prefix_with_registry(
    value: &str,
    registry: &crate::content_registry::ContentRegistry,
) -> Option<CommandEntitySelectorPrefix> {
    published(entity_selector::parse_entity_selector_prefix_with_registry(
        value, registry,
    )?)
}

/// [`parse_command_entity_selector_prefix`] over a buffer's own units, with
/// `consumed` and `advancements_span` in units.
pub fn parse_command_entity_selector_prefix_utf16(
    value: &[u16],
) -> Option<CommandEntitySelectorPrefix> {
    published(entity_selector::parse_entity_selector_prefix_utf16(value)?)
}

pub fn parse_command_entity_selector_prefix_utf16_with_registry(
    value: &[u16],
    registry: &crate::content_registry::ContentRegistry,
) -> Option<CommandEntitySelectorPrefix> {
    published(entity_selector::parse_entity_selector_prefix_utf16_with_registry(value, registry)?)
}

fn published(parsed: entity_selector::EntitySelectorPrefix) -> Option<CommandEntitySelectorPrefix> {
    Some(CommandEntitySelectorPrefix {
        consumed: parsed.consumed,
        multiple: parsed.multiple,
        includes_entities: parsed.includes_entities,
        self_selector: parsed.self_selector,
        advancements_span: parsed.advancements_span,
    })
}

/// The identity a bare `<targets>` names when it is a UUID rather than a player
/// name (`EntitySelectorParser.parseNameOrUUID`, EntitySelectorParser.java:270),
/// packed the way `java.util.UUID` packs its two longs.
///
/// `None` is the `IllegalArgumentException` arm (`:272`), which is where the
/// parser falls back to reading the word as a player name.
pub fn parse_command_java_uuid(value: &str) -> Option<u128> {
    entity_selector::java_uuid_from_string(value)
}

pub fn is_valid_entity_selector(value: &str) -> bool {
    parse_command_entity_selector_prefix(value).is_some_and(|parsed| parsed.consumed == value.len())
}

pub fn is_valid_entity_selector_with_registry(
    value: &str,
    registry: &crate::content_registry::ContentRegistry,
) -> bool {
    parse_command_entity_selector_prefix_with_registry(value, registry)
        .is_some_and(|parsed| parsed.consumed == value.len())
}

pub fn is_valid_entity_selector_utf16(value: &[u16]) -> bool {
    parse_command_entity_selector_prefix_utf16(value)
        .is_some_and(|parsed| parsed.consumed == value.len())
}

pub fn is_valid_entity_selector_utf16_with_registry(
    value: &[u16],
    registry: &crate::content_registry::ContentRegistry,
) -> bool {
    parse_command_entity_selector_prefix_utf16_with_registry(value, registry)
        .is_some_and(|parsed| parsed.consumed == value.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCoordinatesKind {
    BlockPosition,
    ColumnPosition,
    Position2,
    Position3,
    Rotation,
}

/// Parses the coordinate prefix consumed by the corresponding vanilla 26.1
/// command argument. A following space belongs to the command dispatcher, not
/// to this result; spaces between axes are consumed by the coordinate parser.
pub fn parse_command_coordinates_prefix(
    value: &str,
    kind: CommandCoordinatesKind,
) -> Option<usize> {
    // Every accepted coordinate unit is ASCII, so its byte and UTF-16 lengths agree.
    parse_command_coordinates_prefix_utf16(&value.encode_utf16().collect::<Vec<_>>(), kind)
}

pub fn parse_command_coordinates_prefix_utf16(
    value: &[u16],
    kind: CommandCoordinatesKind,
) -> Option<usize> {
    scan_command_coordinates_argument_utf16(value, 0, kind).ok()
}

pub fn scan_command_coordinates_argument_utf16(
    units: &[u16],
    start: usize,
    kind: CommandCoordinatesKind,
) -> Result<usize, CommandSyntaxErrorUtf16> {
    let axes = match kind {
        CommandCoordinatesKind::BlockPosition | CommandCoordinatesKind::Position3 => 3,
        CommandCoordinatesKind::ColumnPosition
        | CommandCoordinatesKind::Position2
        | CommandCoordinatesKind::Rotation => 2,
    };
    let incomplete = || {
        coordinate_error(
            start,
            CommandSyntaxErrorKind::InvalidArgument,
            "incomplete coordinates",
        )
    };
    if axes == 2 && start == units.len() {
        return Err(incomplete());
    }
    let local = axes == 3 && units.get(start) == Some(&94);
    let integer = matches!(
        kind,
        CommandCoordinatesKind::BlockPosition | CommandCoordinatesKind::ColumnPosition
    );
    let mut cursor = start;
    for axis in 0..axes {
        if local {
            parse_local_coordinate(units, &mut cursor, start)?;
        } else {
            parse_world_coordinate(units, &mut cursor, integer)?;
        }
        if axis + 1 < axes {
            if units.get(cursor) != Some(&32) {
                return Err(incomplete());
            }
            cursor += 1;
        }
    }
    Ok(cursor)
}

fn coordinate_error(
    cursor: usize,
    kind: CommandSyntaxErrorKind,
    message: &str,
) -> CommandSyntaxErrorUtf16 {
    CommandSyntaxErrorUtf16 {
        kind,
        cursor,
        message: message.into(),
    }
}

pub fn is_valid_block_position(value: &str) -> bool {
    parse_command_coordinates_prefix(value, CommandCoordinatesKind::BlockPosition)
        == Some(value.len())
}

pub fn is_valid_block_position_utf16(value: &[u16]) -> bool {
    parse_command_coordinates_prefix_utf16(value, CommandCoordinatesKind::BlockPosition)
        == Some(value.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSnbtKind {
    Compound,
    Tag,
}

pub fn parse_command_snbt_prefix(value: &str, kind: CommandSnbtKind) -> Option<usize> {
    match kind {
        CommandSnbtKind::Compound => validate_snbt_compound_prefix(value).ok(),
        CommandSnbtKind::Tag => validate_snbt_prefix(value).ok(),
    }
}

pub fn parse_command_snbt_value_prefix(
    value: &str,
) -> std::result::Result<(usize, NbtSummaryValue), crate::nbt_summary::SnbtParseFailure> {
    parse_snbt_prefix(value)
}

/// [`parse_command_snbt_value_prefix`] over a buffer's own units, with the
/// consumed count in units.
pub fn parse_command_snbt_value_prefix_utf16(
    value: &[u16],
) -> std::result::Result<(usize, NbtSummaryValue), crate::nbt_summary::SnbtParseFailure> {
    parse_snbt_prefix_utf16(value)
}

pub fn parse_command_nbt_path_prefix(value: &str) -> Option<usize> {
    parse_command_nbt_path_value_prefix(value)
        .ok()
        .map(|(consumed, _)| consumed)
}

pub fn parse_command_uuid_prefix(value: &str) -> Option<usize> {
    parse_command_uuid_prefix_utf16(&value.encode_utf16().collect::<Vec<_>>())
}

pub fn parse_command_uuid_prefix_utf16(value: &[u16]) -> Option<usize> {
    let consumed = value
        .iter()
        .take_while(|unit| matches!(**unit, 48..=57 | 65..=70 | 97..=102 | 45))
        .count();
    JavaString::with_utf8_units(&value[..consumed], |raw| {
        (consumed != 0 && entity_selector::is_java_uuid_string(raw)).then_some(consumed)
    })
    .expect("UUID lexeme is ASCII")
}

fn parse_world_coordinate(
    units: &[u16],
    cursor: &mut usize,
    integer: bool,
) -> Result<(), CommandSyntaxErrorUtf16> {
    if units.get(*cursor) == Some(&94) {
        return Err(coordinate_error(
            *cursor,
            CommandSyntaxErrorKind::InvalidArgument,
            "mixed coordinate types",
        ));
    }
    if *cursor == units.len() {
        return Err(coordinate_error(
            *cursor,
            CommandSyntaxErrorKind::ExpectedNumber,
            "expected coordinate",
        ));
    }
    let relative = units[*cursor] == 126;
    if relative {
        *cursor += 1;
    }
    if *cursor >= units.len() || units[*cursor] == 32 {
        return Ok(());
    }
    read_coordinate_number(units, cursor, integer && !relative)
}

fn parse_local_coordinate(
    units: &[u16],
    cursor: &mut usize,
    start: usize,
) -> Result<(), CommandSyntaxErrorUtf16> {
    if *cursor == units.len() {
        return Err(coordinate_error(
            *cursor,
            CommandSyntaxErrorKind::ExpectedNumber,
            "expected coordinate",
        ));
    }
    if units.get(*cursor) != Some(&94) {
        return Err(coordinate_error(
            start,
            CommandSyntaxErrorKind::InvalidArgument,
            "mixed coordinate types",
        ));
    }
    *cursor += 1;
    if *cursor >= units.len() || units[*cursor] == 32 {
        return Ok(());
    }
    read_coordinate_number(units, cursor, false)
}

fn read_coordinate_number(
    units: &[u16],
    cursor: &mut usize,
    integer: bool,
) -> Result<(), CommandSyntaxErrorUtf16> {
    let (end, raw) = scan_command_number_lexeme_utf16(units, *cursor)?;
    JavaString::with_utf8_units(raw, |number| {
        let valid = if integer {
            number.parse::<i32>().is_ok()
        } else {
            number.parse::<f64>().is_ok()
        };
        if valid {
            Ok(())
        } else {
            Err(coordinate_error(
                *cursor,
                CommandSyntaxErrorKind::InvalidNumber,
                &format!("invalid coordinate {number}"),
            ))
        }
    })
    .expect("coordinate number lexeme is ASCII")?;
    *cursor = end;
    Ok(())
}

pub fn is_valid_nbt_path(value: &str) -> bool {
    parse_command_nbt_path_prefix(value) == Some(value.len())
}

pub fn is_valid_nbt_path_utf16(value: &[u16]) -> bool {
    parse_command_nbt_path_value_prefix_utf16(value)
        .is_ok_and(|(consumed, _)| consumed == value.len())
}

#[cfg(test)]
mod tests {
    use super::{
        is_valid_nbt_path, parse_command_coordinates_prefix, parse_command_nbt_path_prefix,
        parse_command_snbt_prefix, parse_command_snbt_value_prefix, parse_command_uuid_prefix,
        CommandCoordinatesKind, CommandSnbtKind, NbtSummaryValue,
    };

    #[test]
    fn parses_vanilla_command_coordinate_families_and_prefixes() {
        for (kind, value) in [
            (CommandCoordinatesKind::Position3, "0.5 ~ ^"),
            (CommandCoordinatesKind::Position3, "^1 ^ ^-5"),
            (CommandCoordinatesKind::BlockPosition, "1 ~ ~.5"),
            (CommandCoordinatesKind::ColumnPosition, "1 ~-2"),
            (CommandCoordinatesKind::Position2, "0.1 -0.5"),
            (CommandCoordinatesKind::Rotation, "~-5 ~5"),
        ] {
            let expected = if value == "0.5 ~ ^" {
                None
            } else {
                Some(value.len())
            };
            assert_eq!(
                parse_command_coordinates_prefix(value, kind),
                expected,
                "coordinate {value:?}"
            );
        }

        assert_eq!(
            parse_command_coordinates_prefix(
                "~1 ~2 ~3 trailing",
                CommandCoordinatesKind::Position3
            ),
            Some("~1 ~2 ~3".len())
        );
        assert_eq!(
            parse_command_coordinates_prefix("1  2", CommandCoordinatesKind::Position3),
            Some("1  2".len())
        );
        assert_eq!(
            parse_command_coordinates_prefix("^ ^", CommandCoordinatesKind::Position2),
            None
        );
        assert_eq!(
            parse_command_coordinates_prefix("1.5 2", CommandCoordinatesKind::ColumnPosition),
            None
        );
    }

    #[test]
    fn exposes_exact_snbt_nbt_path_and_uuid_prefixes() {
        assert_eq!(
            parse_command_snbt_prefix("{foo:[1,2]} trailing", CommandSnbtKind::Compound),
            Some("{foo:[1,2]}".len())
        );
        assert_eq!(
            parse_command_snbt_prefix("[I;1,2] trailing", CommandSnbtKind::Tag),
            Some("[I;1,2]".len())
        );
        assert_eq!(
            parse_command_snbt_prefix("[1]", CommandSnbtKind::Compound),
            None
        );
        assert_eq!(
            parse_command_nbt_path_prefix("foo[{id:'pig'}].bar trailing"),
            Some("foo[{id:'pig'}].bar".len())
        );
        assert_eq!(
            parse_command_uuid_prefix("dd12be42-52a9-4a91-a8a1-11c01849e498 trailing"),
            Some(36)
        );
        assert_eq!(parse_command_uuid_prefix("1-2-3-4-5 trailing"), Some(9));
        assert_eq!(parse_command_uuid_prefix("not-a-uuid"), None);

        let (consumed, value) =
            parse_command_snbt_value_prefix("{foo:[1,2]} trailing").expect("parsed SNBT value");
        assert_eq!(consumed, "{foo:[1,2]}".len());
        assert!(matches!(value, NbtSummaryValue::Compound(_)));
    }

    #[test]
    fn accepts_vanilla_nbt_path_nodes_and_argument_compounds() {
        for path in [
            "",
            "foo",
            "foo.bar",
            "foo.",
            "foo[0]",
            "foo[-1]",
            "[0]",
            "[]",
            "{foo:bar}",
            "foo{kind:'test'}",
            "foo[{id:'minecraft:pig'}].bar",
            "\"quoted.key\"[0]",
            "'single\"quote'",
            "\"escaped\\\\path\"",
            "\"name with spaces\"",
            "foo.[0]",
            "{nested:{values:[1,2,],flag:true,},}",
        ] {
            assert!(is_valid_nbt_path(path), "rejected {path:?}");
        }
    }

    #[test]
    fn rejects_incomplete_mixed_or_trailing_nbt_path_data() {
        for path in [
            " ",
            "foo bar",
            ".foo",
            "foo..bar",
            "foo[",
            "foo[0",
            "foo[0.0]",
            "foo[2147483648]",
            "foo[{}",
            "foo[{a:1]",
            "foo{a:1",
            "foo{a=1}",
            "foo{a:1}bar",
            "foo{a:1}{b:2}",
            "foo[\"index\"]",
            "\"\"",
            "\"bad\\q\"",
            "foo.{a:1}",
        ] {
            assert!(!is_valid_nbt_path(path), "accepted {path:?}");
        }
    }
}
