use super::{CommandSyntaxError, CommandSyntaxErrorKind};
use crate::JavaString;

/// Cursor and message retain Java UTF-16 units.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommandSyntaxErrorUtf16 {
    pub kind: CommandSyntaxErrorKind,
    pub cursor: usize,
    pub message: JavaString,
}

pub fn scan_command_unquoted_string_utf16(command: &[u16], start: usize) -> usize {
    let mut end = start;
    while command
        .get(end)
        .is_some_and(|unit| matches!(*unit, 48..=57 | 65..=90 | 97..=122 | 95 | 45 | 46 | 43))
    {
        end += 1;
    }
    end
}

pub fn scan_command_quotable_string_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, CommandSyntaxErrorUtf16> {
    let Some(quote @ (39 | 34)) = command.get(start).copied() else {
        return Ok(scan_command_unquoted_string_utf16(command, start));
    };
    let mut cursor = start + 1;
    let mut escaped = false;
    while let Some(unit) = command.get(cursor).copied() {
        cursor += 1;
        if escaped {
            if unit != quote && unit != 92 {
                return Err(CommandSyntaxErrorUtf16 {
                    kind: CommandSyntaxErrorKind::InvalidEscape,
                    cursor: cursor - 1,
                    message: "invalid escape "
                        .encode_utf16()
                        .chain([unit])
                        .collect::<Vec<_>>()
                        .into(),
                });
            }
            escaped = false;
        } else if unit == 92 {
            escaped = true;
        } else if unit == quote {
            return Ok(cursor);
        }
    }
    Err(CommandSyntaxErrorUtf16 {
        kind: CommandSyntaxErrorKind::ExpectedQuoteEnd,
        cursor: command.len(),
        message: "expected quote terminator".into(),
    })
}

pub fn read_command_string_value_utf16(
    command: &[u16],
    start: usize,
) -> Result<(usize, JavaString), CommandSyntaxErrorUtf16> {
    let end = scan_command_quotable_string_utf16(command, start)?;
    let raw = &command[start..end];
    if !matches!(raw.first(), Some(39 | 34)) {
        return Ok((end, raw.to_vec().into()));
    }
    let mut value = Vec::with_capacity(raw.len().saturating_sub(2));
    let mut escaped = false;
    for unit in &raw[1..raw.len() - 1] {
        if !escaped && *unit == 92 {
            escaped = true;
        } else {
            value.push(*unit);
            escaped = false;
        }
    }
    Ok((end, value.into()))
}

pub fn scan_command_unquoted_string(command: &str, start: usize) -> usize {
    if !command.is_char_boundary(start) {
        return start;
    }
    let input: Vec<_> = command[start..].encode_utf16().collect();
    start
        + byte_cursor(
            &command[start..],
            scan_command_unquoted_string_utf16(&input, 0),
        )
}

pub fn scan_command_quotable_string(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    if !command.is_char_boundary(start) {
        return Ok(start);
    }
    scan_suffix_via_utf16(command, start, |input| {
        scan_command_quotable_string_utf16(input, 0)
    })
}

pub fn read_command_string_value(
    command: &str,
    start: usize,
) -> Result<(usize, String), CommandSyntaxError> {
    scan_suffix_value_via_utf16(
        command,
        start,
        |input| read_command_string_value_utf16(input, 0),
        |(end, value), to_byte| {
            (
                to_byte(end),
                String::from_utf16(value.units())
                    .expect("UTF-8 input and quote unescaping preserve scalar values"),
            )
        },
    )
}

pub(super) fn scan_suffix_value_via_utf16<T, U>(
    command: &str,
    start: usize,
    scan: impl FnOnce(&[u16]) -> Result<T, CommandSyntaxErrorUtf16>,
    map: impl FnOnce(T, &dyn Fn(usize) -> usize) -> U,
) -> Result<U, CommandSyntaxError> {
    let suffix = &command[start..];
    let input: Vec<_> = suffix.encode_utf16().collect();
    let to_byte = |cursor: usize| start + byte_cursor(suffix, cursor);
    scan(&input)
        .map(|value| map(value, &to_byte))
        .map_err(|error| byte_error(command, start, error))
}

pub(super) fn scan_suffix_via_utf16(
    command: &str,
    start: usize,
    scan: impl FnOnce(&[u16]) -> Result<usize, CommandSyntaxErrorUtf16>,
) -> Result<usize, CommandSyntaxError> {
    scan_suffix_value_via_utf16(command, start, scan, |end, to_byte| to_byte(end))
}

pub(super) fn scan_from_start_via_utf16<T, U>(
    command: &str,
    start: usize,
    scan: impl FnOnce(&[u16], usize) -> Result<T, CommandSyntaxErrorUtf16>,
    map: impl FnOnce(T, &dyn Fn(usize) -> usize) -> U,
) -> Result<U, CommandSyntaxError> {
    let units: Vec<_> = command.encode_utf16().collect();
    let unit_start = command[..start].encode_utf16().count();
    let to_byte = |cursor: usize| byte_cursor(command, cursor);
    scan(&units, unit_start)
        .map(|value| map(value, &to_byte))
        .map_err(|error| byte_error(command, 0, error))
}

pub(super) fn byte_error(
    command: &str,
    start: usize,
    error: CommandSyntaxErrorUtf16,
) -> CommandSyntaxError {
    CommandSyntaxError {
        kind: error.kind,
        cursor: Some(start + byte_cursor(&command[start..], error.cursor)),
        message: String::from_utf16_lossy(error.message.units()),
    }
}

pub(in crate::component::command) fn byte_cursor(input: &str, units: usize) -> usize {
    let mut cursor = 0;
    for (byte, ch) in input.char_indices() {
        if cursor >= units {
            return byte;
        }
        cursor += ch.len_utf16();
    }
    input.len()
}
