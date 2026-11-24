//! The grammar shared by the arguments that name something a registry holds.
//!
//! None of these scanners open a registry: they read the name, the wildcard
//! pattern, the inline SNBT or the particle options list, and stop. Whether the
//! name is in the registry is a question only a caller holding one can answer.

use super::{
    invalid_argument, strings, CommandSyntaxError, CommandSyntaxErrorKind, CommandSyntaxErrorUtf16,
};
use crate::{packets::normalize_resource_location, JavaString, NbtSummaryEntry, NbtSummaryValue};

pub fn scan_command_resource_location(
    command: &str,
    start: usize,
) -> Result<(usize, String), CommandSyntaxError> {
    strings::scan_suffix_value_via_utf16(
        command,
        start,
        |input| scan_command_resource_location_utf16(input, 0),
        |(end, value), to_byte| (to_byte(end), value),
    )
}

pub fn scan_command_resource_location_utf16(
    command: &[u16],
    start: usize,
) -> Result<(usize, String), CommandSyntaxErrorUtf16> {
    let end = resource_end(command, start, false);
    JavaString::with_utf8_units(&command[start..end], |raw| normalize_resource_location(raw))
        .expect("resource lexeme is ASCII")
        .map(|value| (end, value))
        .map_err(|_| CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::InvalidArgument,
            cursor: start,
            message: "invalid resource location".into(),
        })
}

pub fn scan_command_resource_selector(
    command: &str,
    start: usize,
) -> Result<(usize, String), CommandSyntaxError> {
    if !command.is_char_boundary(start) {
        return Err(invalid_argument(
            start,
            "resource selector matched no resources",
        ));
    }
    strings::scan_suffix_value_via_utf16(
        command,
        start,
        |input| scan_command_resource_selector_utf16(input, 0),
        |(end, value), to_byte| (to_byte(end), value),
    )
}

pub fn scan_command_resource_selector_utf16(
    command: &[u16],
    start: usize,
) -> Result<(usize, String), CommandSyntaxErrorUtf16> {
    let end = resource_end(command, start, true);
    let pattern = JavaString::with_utf8_units(&command[start..end], |raw| {
        if raw.contains(':') {
            raw.to_owned()
        } else {
            format!("minecraft:{raw}")
        }
    })
    .expect("resource pattern is ASCII");
    Ok((end, pattern))
}

fn resource_end(command: &[u16], start: usize, wildcard: bool) -> usize {
    let mut end = start;
    while command.get(end).is_some_and(|unit| {
        matches!(*unit, 48..=57 | 97..=122 | 95 | 58 | 47 | 46 | 45)
            || (wildcard && matches!(*unit, 42 | 63))
    }) {
        end += 1;
    }
    end
}

#[cfg(test)]
#[test]
fn raw_command_resource_scanning_keeps_positions_and_registry_owned_empty_patterns() {
    let input: Vec<u16> = [0xd800]
        .into_iter()
        .chain(":stone".encode_utf16())
        .chain([0xdc00])
        .collect();
    assert_eq!(
        scan_command_resource_location_utf16(&input, 1).unwrap(),
        (7, "minecraft:stone".into())
    );
    let invalid: Vec<u16> = [0xd800].into_iter().chain("a:b:c".encode_utf16()).collect();
    let error = scan_command_resource_location_utf16(&invalid, 1).unwrap_err();
    assert_eq!(error.cursor, 1);
    assert_eq!(error.kind, CommandSyntaxErrorKind::InvalidArgument);
    assert_eq!(
        scan_command_resource_location("😀 :stone rest", 5).unwrap(),
        (11, "minecraft:stone".into())
    );
    let pattern: Vec<u16> = "st?*".encode_utf16().chain([0xd800]).collect();
    assert_eq!(
        scan_command_resource_selector_utf16(&pattern, 0).unwrap(),
        (4, "minecraft:st?*".into())
    );
    assert_eq!(
        scan_command_resource_location_utf16(&pattern, 0).unwrap(),
        (2, "minecraft:st".into())
    );
    assert_eq!(
        scan_command_resource_selector_utf16(&[0xd800], 0).unwrap(),
        (0, "minecraft:".into())
    );
    assert_eq!(
        scan_command_resource_selector("", 0).unwrap(),
        (0, "minecraft:".into())
    );
    assert_eq!(
        scan_command_resource_selector("😀", 1).unwrap_err().cursor,
        Some(1)
    );
}

/// Whether a resource-selector pattern matches one resource name: `*` spans any
/// run and `?` any single character, the shapes `FileNameUtils` glob matching
/// gives the argument.
pub fn command_resource_selector_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let mut pattern_index = 0;
    let mut value_index = 0;
    let mut star = None;
    let mut star_value_index = 0;

    while value_index < value.len() {
        if pattern
            .get(pattern_index)
            .is_some_and(|token| *token == b'?' || *token == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern.get(pattern_index) == Some(&b'*') {
            star = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }
    while pattern.get(pattern_index) == Some(&b'*') {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

/// The scoreboard-criteria spelling of a resource location: statistics name
/// their type and value with dots where a resource location has a colon.
pub fn command_stat_key_for_resource_location(resource_id: &str) -> String {
    resource_id.replace(':', ".")
}

/// `minecraft:component` and `minecraft:style`, whose whole grammar is one SNBT
/// value; what the value has to look like is a codec question.
pub fn scan_command_text_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<(usize, NbtSummaryValue), CommandSyntaxErrorUtf16> {
    crate::parse_snbt_prefix_utf16(&command[start..])
        .map(|(consumed, value)| (start + consumed, value))
        .map_err(|_| resource_syntax_error(start, "invalid text SNBT"))
}

pub fn scan_command_text_argument(
    command: &str,
    start: usize,
) -> Result<(usize, NbtSummaryValue), CommandSyntaxError> {
    strings::scan_from_start_via_utf16(
        command,
        start,
        scan_command_text_argument_utf16,
        |(end, value), to_byte| (to_byte(end), value),
    )
}

/// What a `ResourceOrIdArgument` (`loot_table`, `loot_predicate`,
/// `loot_modifier`, `dialog`) was spelled as.
#[derive(Debug, Clone, PartialEq)]
pub enum ScannedCommandResourceOrId {
    /// A name pointing at something the registry is expected to hold.
    Reference { end: usize, resource_id: String },
    /// An inline definition written as SNBT.
    Inline { end: usize, value: NbtSummaryValue },
}

impl ScannedCommandResourceOrId {
    pub fn end(&self) -> usize {
        match self {
            Self::Reference { end, .. } | Self::Inline { end, .. } => *end,
        }
    }

    fn map_offsets(self, to_byte: &dyn Fn(usize) -> usize) -> Self {
        match self {
            Self::Reference { end, resource_id } => Self::Reference {
                end: to_byte(end),
                resource_id,
            },
            Self::Inline { end, value } => Self::Inline {
                end: to_byte(end),
                value,
            },
        }
    }
}

pub type ScannedCommandResourceOrIdUtf16 = ScannedCommandResourceOrId;

pub fn scan_command_resource_or_id_utf16(
    command: &[u16],
    start: usize,
) -> Result<ScannedCommandResourceOrIdUtf16, CommandSyntaxErrorUtf16> {
    let start = super::scan_command_whitespace_utf16(command, start);
    if let Ok((end, resource_id)) = scan_command_resource_location_utf16(command, start) {
        if end != start {
            return Ok(ScannedCommandResourceOrId::Reference { end, resource_id });
        }
    }
    let (consumed, value) =
        crate::parse_snbt_prefix_utf16(&command[start..]).map_err(|_| CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::InvalidArgument,
            cursor: start,
            message: "invalid inline resource SNBT".into(),
        })?;
    Ok(ScannedCommandResourceOrId::Inline {
        end: start + consumed,
        value,
    })
}

pub fn scan_command_resource_or_id(
    command: &str,
    start: usize,
) -> Result<ScannedCommandResourceOrId, CommandSyntaxError> {
    strings::scan_from_start_via_utf16(
        command,
        start,
        scan_command_resource_or_id_utf16,
        ScannedCommandResourceOrId::map_offsets,
    )
}

#[cfg(test)]
#[test]
fn raw_command_resource_or_id_preserves_values_and_identifier_priority() {
    for token in ["true", "012", "example:notice"] {
        let input: Vec<_> = [0xd800, 0x2000]
            .into_iter()
            .chain(token.encode_utf16())
            .chain([0xdc00])
            .collect();
        let ScannedCommandResourceOrIdUtf16::Reference { end, .. } =
            scan_command_resource_or_id_utf16(&input, 1).unwrap()
        else {
            panic!("identifier must win")
        };
        assert_eq!(end, input.len() - 1);
    }
    let input: Vec<_> = [0xd800, 32]
        .into_iter()
        .chain("{title:'".encode_utf16())
        .chain([0xdc00])
        .chain("'} tail".encode_utf16())
        .collect();
    let ScannedCommandResourceOrIdUtf16::Inline {
        end,
        value: NbtSummaryValue::Compound(entries),
    } = scan_command_resource_or_id_utf16(&input, 1).unwrap()
    else {
        panic!("inline compound required")
    };
    assert_eq!(
        entries[0].value,
        NbtSummaryValue::string_from_utf16(vec![0xdc00])
    );
    assert_eq!(end, input.len() - 5);
    let input = "😀  example:notice tail";
    let ScannedCommandResourceOrId::Reference { end, .. } =
        scan_command_resource_or_id(input, "😀".len()).unwrap()
    else {
        panic!("reference required")
    };
    assert_eq!(end, "😀  example:notice".len());
}

/// The options compound of a `minecraft:particle` argument.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScannedCommandParticleOptions {
    pub end: usize,
    /// The compound's fields, or `None` when the argument carried no compound.
    pub fields: Option<Vec<NbtSummaryEntry>>,
}

impl ScannedCommandParticleOptions {
    fn map_offsets(self, to_byte: &dyn Fn(usize) -> usize) -> Self {
        Self {
            end: to_byte(self.end),
            fields: self.fields,
        }
    }
}

pub type ScannedCommandParticleOptionsUtf16 = ScannedCommandParticleOptions;

pub fn scan_command_particle_options_utf16(
    command: &[u16],
    start: usize,
) -> Result<ScannedCommandParticleOptionsUtf16, CommandSyntaxErrorUtf16> {
    let mut cursor = start;
    let has_map = command.get(cursor) == Some(&123);
    let mut fields = None;
    if has_map {
        let (consumed, value) = crate::parse_snbt_compound_prefix_utf16(&command[cursor..])
            .map_err(|_| resource_syntax_error(cursor, "invalid particle options"))?;
        cursor += consumed;
        let NbtSummaryValue::Compound(compound) = value else {
            unreachable!("compound parser")
        };
        fields = Some(compound);
    }
    Ok(ScannedCommandParticleOptions {
        end: cursor,
        fields,
    })
}

pub fn scan_command_particle_options(
    command: &str,
    start: usize,
) -> Result<ScannedCommandParticleOptions, CommandSyntaxError> {
    strings::scan_from_start_via_utf16(
        command,
        start,
        scan_command_particle_options_utf16,
        ScannedCommandParticleOptions::map_offsets,
    )
}

fn resource_syntax_error(cursor: usize, message: &str) -> CommandSyntaxErrorUtf16 {
    CommandSyntaxErrorUtf16 {
        kind: CommandSyntaxErrorKind::InvalidArgument,
        cursor,
        message: message.into(),
    }
}

#[cfg(test)]
#[test]
fn raw_command_particle_options_keep_compound_adjacency_and_positions() {
    let input: Vec<_> = [0xd800]
        .into_iter()
        .chain("{custom:'".encode_utf16())
        .chain([0xdc00])
        .chain("'} tail".encode_utf16())
        .collect();
    let parsed = scan_command_particle_options_utf16(&input, 1).unwrap();
    assert_eq!(parsed.end, input.len() - 5);
    assert_eq!(
        parsed.fields.unwrap()[0].value,
        NbtSummaryValue::string_from_utf16(vec![0xdc00])
    );
    let input: Vec<_> = " {}".encode_utf16().collect();
    let parsed = scan_command_particle_options_utf16(&input, 0).unwrap();
    assert_eq!(parsed.end, 0);
    assert!(parsed.fields.is_none());
    let input = "😀{custom:'😀'} tail";
    assert_eq!(
        scan_command_particle_options(input, "😀".len())
            .unwrap()
            .end,
        "😀{custom:'😀'}".len()
    );
}
