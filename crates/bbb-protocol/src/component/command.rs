use crate::nbt_summary::{validate_snbt_compound_prefix, validate_snbt_prefix};

mod entity_selector;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandEntitySelectorPrefix {
    pub consumed: usize,
    pub multiple: bool,
    pub includes_entities: bool,
    pub self_selector: bool,
}

pub fn parse_command_entity_selector_prefix(value: &str) -> Option<CommandEntitySelectorPrefix> {
    let parsed = entity_selector::parse_entity_selector_prefix(value)?;
    Some(CommandEntitySelectorPrefix {
        consumed: parsed.consumed,
        multiple: parsed.multiple,
        includes_entities: parsed.includes_entities,
        self_selector: parsed.self_selector,
    })
}

pub(super) fn is_valid_entity_selector(value: &str) -> bool {
    parse_command_entity_selector_prefix(value).is_some_and(|parsed| parsed.consumed == value.len())
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
    let bytes = value.as_bytes();
    let mut cursor = 0;
    let axes = match kind {
        CommandCoordinatesKind::BlockPosition | CommandCoordinatesKind::Position3 => 3,
        CommandCoordinatesKind::ColumnPosition
        | CommandCoordinatesKind::Position2
        | CommandCoordinatesKind::Rotation => 2,
    };
    let local = bytes.first() == Some(&b'^');
    if local
        && !matches!(
            kind,
            CommandCoordinatesKind::BlockPosition | CommandCoordinatesKind::Position3
        )
    {
        return None;
    }
    let integer = matches!(
        kind,
        CommandCoordinatesKind::BlockPosition | CommandCoordinatesKind::ColumnPosition
    );

    for axis in 0..axes {
        let valid = if local {
            parse_local_coordinate(bytes, &mut cursor)
        } else {
            parse_world_coordinate(bytes, &mut cursor, integer)
        };
        if !valid {
            return None;
        }
        if axis + 1 < axes {
            if bytes.get(cursor) != Some(&b' ') {
                return None;
            }
            cursor += 1;
        }
    }
    Some(cursor)
}

pub(super) fn is_valid_block_position(value: &str) -> bool {
    parse_command_coordinates_prefix(value, CommandCoordinatesKind::BlockPosition)
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

pub fn parse_command_nbt_path_prefix(value: &str) -> Option<usize> {
    NbtPathParser {
        input: value,
        cursor: 0,
    }
    .parse()
}

pub fn parse_command_uuid_prefix(value: &str) -> Option<usize> {
    let consumed = value
        .bytes()
        .take_while(|byte| byte.is_ascii_hexdigit() || *byte == b'-')
        .count();
    (consumed != 0 && entity_selector::is_java_uuid_string(&value[..consumed])).then_some(consumed)
}

fn parse_world_coordinate(bytes: &[u8], cursor: &mut usize, integer: bool) -> bool {
    if *cursor >= bytes.len() || bytes[*cursor] == b'^' {
        return false;
    }
    let relative = bytes[*cursor] == b'~';
    if relative {
        *cursor += 1;
    }
    if *cursor >= bytes.len() || bytes[*cursor] == b' ' {
        // WorldCoordinate.parseInt treats an omitted value as zero, including
        // an absolute axis positioned directly at a separator.
        return true;
    }
    let number = read_brigadier_number(bytes, cursor);
    if relative || !integer {
        is_brigadier_double(number)
    } else {
        number.parse::<i32>().is_ok()
    }
}

fn parse_local_coordinate(bytes: &[u8], cursor: &mut usize) -> bool {
    if bytes.get(*cursor) != Some(&b'^') {
        return false;
    }
    *cursor += 1;
    if *cursor >= bytes.len() || bytes[*cursor] == b' ' {
        return true;
    }
    is_brigadier_double(read_brigadier_number(bytes, cursor))
}

fn read_brigadier_number<'a>(bytes: &'a [u8], cursor: &mut usize) -> &'a str {
    let start = *cursor;
    while bytes
        .get(*cursor)
        .is_some_and(|byte| byte.is_ascii_digit() || matches!(*byte, b'.' | b'-'))
    {
        *cursor += 1;
    }
    std::str::from_utf8(&bytes[start..*cursor]).expect("Brigadier number characters are ASCII")
}

fn is_brigadier_double(number: &str) -> bool {
    let unsigned = number.strip_prefix('-').unwrap_or(number);
    let mut halves = unsigned.split('.');
    let whole = halves.next().expect("split always yields one part");
    let fraction = halves.next();
    if halves.next().is_some() {
        return false;
    }
    let fraction = fraction.unwrap_or("");
    (!whole.is_empty() || !fraction.is_empty())
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn is_valid_nbt_path(value: &str) -> bool {
    parse_command_nbt_path_prefix(value) == Some(value.len())
}

struct NbtPathParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl NbtPathParser<'_> {
    fn parse(mut self) -> Option<usize> {
        let mut first_node = true;
        while self.peek().is_some_and(|next| next != ' ') {
            if !self.parse_node(first_node) {
                return None;
            }
            first_node = false;
            if let Some(next) = self.peek() {
                if next != ' ' && next != '[' && next != '{' && !self.consume('.') {
                    return None;
                }
            }
        }
        Some(self.cursor)
    }

    fn parse_node(&mut self, first_node: bool) -> bool {
        match self.peek() {
            Some('\'' | '"') => {
                if !self.read_quoted_name() {
                    return false;
                }
                self.parse_object_pattern()
            }
            Some('[') => self.parse_element_node(),
            Some('{') if first_node => self.parse_compound_pattern(),
            Some('{') | None => false,
            Some(_) => {
                if !self.read_unquoted_name() {
                    return false;
                }
                self.parse_object_pattern()
            }
        }
    }

    fn parse_object_pattern(&mut self) -> bool {
        self.peek() != Some('{') || self.parse_compound_pattern()
    }

    fn parse_element_node(&mut self) -> bool {
        self.consume('[');
        match self.peek() {
            Some('{') => self.parse_compound_pattern() && self.consume(']'),
            Some(']') => self.consume(']'),
            Some(_) => self.read_brigadier_int() && self.consume(']'),
            None => false,
        }
    }

    fn parse_compound_pattern(&mut self) -> bool {
        let Ok(consumed) = validate_snbt_compound_prefix(&self.input[self.cursor..]) else {
            return false;
        };
        self.cursor += consumed;
        true
    }

    fn read_quoted_name(&mut self) -> bool {
        let Some(quote @ ('\'' | '"')) = self.bump() else {
            return false;
        };
        let mut empty = true;
        loop {
            match self.bump() {
                Some(next) if next == quote => return !empty,
                Some('\\') => match self.bump() {
                    Some(escaped) if escaped == quote || escaped == '\\' => empty = false,
                    _ => return false,
                },
                Some(_) => empty = false,
                None => return false,
            }
        }
    }

    fn read_unquoted_name(&mut self) -> bool {
        let start = self.cursor;
        while self.peek().is_some_and(is_unquoted_nbt_path_name) {
            self.bump();
        }
        self.cursor != start
    }

    fn read_brigadier_int(&mut self) -> bool {
        let bytes = self.input.as_bytes();
        let number = read_brigadier_number(bytes, &mut self.cursor);
        !number.is_empty() && number.parse::<i32>().is_ok()
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.cursor += next.len_utf8();
        Some(next)
    }
}

fn is_unquoted_nbt_path_name(next: char) -> bool {
    !matches!(next, ' ' | '"' | '\'' | '[' | ']' | '.' | '{' | '}')
}

#[cfg(test)]
mod tests {
    use super::{
        is_valid_nbt_path, parse_command_coordinates_prefix, parse_command_nbt_path_prefix,
        parse_command_snbt_prefix, parse_command_uuid_prefix, CommandCoordinatesKind,
        CommandSnbtKind,
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
