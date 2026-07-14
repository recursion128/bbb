use crate::nbt_summary::validate_snbt_compound_prefix;

mod entity_selector;

pub(super) fn is_valid_entity_selector(value: &str) -> bool {
    entity_selector::is_valid_entity_selector(value)
}

pub(super) fn is_valid_block_position(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut cursor = 0;
    let local = bytes.first() == Some(&b'^');
    for axis in 0..3 {
        let valid = if local {
            parse_local_coordinate(bytes, &mut cursor)
        } else {
            parse_world_block_coordinate(bytes, &mut cursor)
        };
        if !valid {
            return false;
        }
        if axis < 2 {
            if bytes.get(cursor) != Some(&b' ') {
                return false;
            }
            cursor += 1;
        }
    }
    cursor == bytes.len()
}

fn parse_world_block_coordinate(bytes: &[u8], cursor: &mut usize) -> bool {
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
    if relative {
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
    NbtPathParser {
        input: value,
        cursor: 0,
    }
    .parse()
}

struct NbtPathParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl NbtPathParser<'_> {
    fn parse(mut self) -> bool {
        let mut first_node = true;
        while self.peek().is_some_and(|next| next != ' ') {
            if !self.parse_node(first_node) {
                return false;
            }
            first_node = false;
            if let Some(next) = self.peek() {
                if next != ' ' && next != '[' && next != '{' && !self.consume('.') {
                    return false;
                }
            }
        }
        self.cursor == self.input.len()
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
    use super::is_valid_nbt_path;

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
