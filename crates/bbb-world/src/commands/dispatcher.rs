use std::collections::HashSet;

use bbb_protocol::packets::{
    command_argument_template, CommandArgumentTemplate, CommandStringMode,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
};
use serde::{Deserialize, Serialize};

use super::{
    CommandArgumentParserState, CommandNodeKindState, CommandTreeState, SignableCommandArgument,
};

mod context_free;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandParseResult {
    /// Byte cursor into the original command text, including a normalized
    /// leading slash/whitespace prefix when one was supplied.
    pub cursor: usize,
    pub fully_consumed: bool,
    pub executable: bool,
    pub restricted: bool,
    /// False while the selected route contains a parser family whose owning
    /// implementation phase has not landed yet. Such a route may still expose
    /// signed spans, but must not be certified as locally valid.
    pub validation_complete: bool,
    pub arguments: Vec<ParsedCommandArgument>,
    pub signable_arguments: Vec<SignableCommandArgument>,
    pub error: Option<CommandSyntaxErrorState>,
    pub completion: CommandCompletionContext,
}

impl CommandParseResult {
    pub fn is_valid_command(&self) -> bool {
        self.fully_consumed && self.executable && self.validation_complete && self.error.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCommandArgument {
    pub name: String,
    pub parser: String,
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub validated: bool,
    pub signable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandCompletionContext {
    pub node_index: i32,
    pub start: usize,
    pub suggestions: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandSyntaxErrorKind {
    UnknownCommand,
    UnknownArgument,
    IncorrectLiteral,
    ExpectedSeparator,
    ExpectedBoolean,
    InvalidBoolean,
    ExpectedNumber,
    InvalidNumber,
    NumberTooLow,
    NumberTooHigh,
    ExpectedQuoteEnd,
    InvalidEscape,
    InvalidArgument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSyntaxErrorState {
    pub kind: CommandSyntaxErrorKind,
    pub cursor: usize,
    pub message: String,
}

#[derive(Clone)]
struct ParseCandidate {
    cursor: usize,
    current_node: i32,
    executable: bool,
    restricted: bool,
    validation_complete: bool,
    arguments: Vec<ParsedCommandArgument>,
    error: Option<CommandSyntaxErrorState>,
    suggestions: Option<String>,
    parsed_any_node: bool,
}

impl ParseCandidate {
    fn root(root_index: i32) -> Self {
        Self {
            cursor: 0,
            current_node: root_index,
            executable: false,
            restricted: false,
            validation_complete: true,
            arguments: Vec::new(),
            error: None,
            suggestions: None,
            parsed_any_node: false,
        }
    }
}

pub(super) struct ParsedValue {
    pub(super) end: usize,
    pub(super) validated: bool,
}

impl CommandTreeState {
    pub fn parse_command(&self, source: &str) -> CommandParseResult {
        let (command, source_offset) = normalized_command(source);
        let mut active = HashSet::new();
        let candidate = self.parse_nodes(
            self.root_index,
            command,
            ParseCandidate::root(self.root_index),
            &mut active,
        );

        let fully_consumed = candidate.cursor == command.len();
        let mut error = candidate.error;
        if error.is_none() && !fully_consumed {
            error = Some(syntax_error(
                if candidate.parsed_any_node {
                    CommandSyntaxErrorKind::UnknownArgument
                } else {
                    CommandSyntaxErrorKind::UnknownCommand
                },
                candidate.cursor,
                if candidate.parsed_any_node {
                    "unknown command argument"
                } else {
                    "unknown command"
                },
            ));
        }

        let mut arguments = candidate.arguments;
        let signable_arguments = arguments
            .iter()
            .filter(|argument| argument.signable)
            .map(|argument| SignableCommandArgument {
                name: argument.name.clone(),
                value: argument.value.clone(),
            })
            .collect();
        for argument in &mut arguments {
            argument.start += source_offset;
            argument.end += source_offset;
        }
        if let Some(error) = &mut error {
            error.cursor += source_offset;
        }

        CommandParseResult {
            cursor: candidate.cursor + source_offset,
            fully_consumed,
            executable: candidate.executable,
            restricted: candidate.restricted,
            validation_complete: candidate.validation_complete,
            arguments,
            signable_arguments,
            error,
            completion: CommandCompletionContext {
                node_index: candidate.current_node,
                start: candidate.cursor + source_offset,
                suggestions: candidate.suggestions,
            },
        }
    }

    fn parse_nodes(
        &self,
        node_index: i32,
        command: &str,
        context: ParseCandidate,
        active: &mut HashSet<(i32, usize)>,
    ) -> ParseCandidate {
        if !active.insert((node_index, context.cursor)) {
            return ParseCandidate {
                error: Some(syntax_error(
                    CommandSyntaxErrorKind::InvalidArgument,
                    context.cursor,
                    "command redirect did not advance the cursor",
                )),
                ..context
            };
        }

        let relevant = self.relevant_children(node_index, command, context.cursor);
        let mut errors = Vec::new();
        let mut potentials = Vec::new();

        for child_index in relevant {
            let Some(child) = self.node(child_index) else {
                continue;
            };
            let mut candidate = context.clone();
            candidate.error = None;

            match self.parse_child(child_index, command, &mut candidate) {
                Ok(()) => {}
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            }

            if command
                .as_bytes()
                .get(candidate.cursor)
                .is_some_and(|byte| *byte != b' ')
            {
                errors.push(syntax_error(
                    CommandSyntaxErrorKind::ExpectedSeparator,
                    candidate.cursor,
                    "expected a space between command arguments",
                ));
                continue;
            }

            candidate.executable = child.executable;
            candidate.restricted |= child.restricted;
            candidate.current_node = child_index;
            candidate.suggestions = child.suggestions.clone();
            candidate.parsed_any_node = true;

            let redirect_width = usize::from(child.redirect.is_none()) + 1;
            if command.len().saturating_sub(candidate.cursor) >= redirect_width {
                candidate.cursor += 1;
                if let Some(redirect) = child.redirect {
                    candidate.current_node = redirect;
                    let result = self.parse_nodes(redirect, command, candidate, active);
                    active.remove(&(node_index, context.cursor));
                    return result;
                }
                let next = self.parse_nodes(child_index, command, candidate, active);
                potentials.push(next);
            } else {
                potentials.push(candidate);
            }
        }

        active.remove(&(node_index, context.cursor));

        if !potentials.is_empty() {
            potentials.sort_by_key(|candidate| {
                (candidate.cursor != command.len(), candidate.error.is_some())
            });
            return potentials.remove(0);
        }

        let error = if errors.len() == 1 {
            errors.pop()
        } else {
            Some(syntax_error(
                if context.parsed_any_node {
                    CommandSyntaxErrorKind::UnknownArgument
                } else {
                    CommandSyntaxErrorKind::UnknownCommand
                },
                context.cursor,
                if context.parsed_any_node {
                    "unknown command argument"
                } else {
                    "unknown command"
                },
            ))
        };
        ParseCandidate { error, ..context }
    }

    fn relevant_children(&self, node_index: i32, command: &str, cursor: usize) -> Vec<i32> {
        let Some(node) = self.node(node_index) else {
            return Vec::new();
        };
        // CommandNode.addChild keeps the first child for a name in one
        // LinkedHashMap, then indexes that same owner by its original node
        // kind. Preserve both first-owner and insertion-order semantics.
        let mut child_names = HashSet::new();
        let mut arguments = Vec::new();
        let mut literal_match = None;
        let token_end = command[cursor..]
            .find(' ')
            .map_or(command.len(), |offset| cursor + offset);
        let token = &command[cursor..token_end];

        for child_index in &node.children {
            let Some(child) = self.node(*child_index) else {
                continue;
            };
            let name = child.name.as_deref().unwrap_or_default();
            if !child_names.insert(name) {
                continue;
            }
            match child.node_type {
                CommandNodeKindState::Literal => {
                    if name == token && literal_match.is_none() {
                        literal_match = Some(*child_index);
                    }
                }
                CommandNodeKindState::Argument => arguments.push(*child_index),
                _ => {}
            }
        }

        if let Some(literal_match) = literal_match {
            return vec![literal_match];
        }

        arguments
    }

    fn parse_child(
        &self,
        child_index: i32,
        command: &str,
        candidate: &mut ParseCandidate,
    ) -> Result<(), CommandSyntaxErrorState> {
        let child = self.node(child_index).ok_or_else(|| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                candidate.cursor,
                "command child is missing",
            )
        })?;
        match child.node_type {
            CommandNodeKindState::Literal => {
                let literal = child.name.as_deref().unwrap_or_default();
                let end = candidate.cursor.saturating_add(literal.len());
                if command.get(candidate.cursor..end) == Some(literal)
                    && command.as_bytes().get(end).is_none_or(|byte| *byte == b' ')
                {
                    candidate.cursor = end;
                    Ok(())
                } else {
                    Err(syntax_error(
                        CommandSyntaxErrorKind::IncorrectLiteral,
                        candidate.cursor,
                        format!("expected literal {literal}"),
                    ))
                }
            }
            CommandNodeKindState::Argument => {
                let parser = child.parser.as_ref().ok_or_else(|| {
                    syntax_error(
                        CommandSyntaxErrorKind::InvalidArgument,
                        candidate.cursor,
                        "argument node has no parser",
                    )
                })?;
                let start = candidate.cursor;
                let parsed = parse_argument(command, start, parser)?;
                let signable = parser.type_id == VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID;
                candidate.arguments.push(ParsedCommandArgument {
                    name: child.name.clone().unwrap_or_default(),
                    parser: parser.name.clone(),
                    start,
                    end: parsed.end,
                    value: command[start..parsed.end].to_string(),
                    validated: parsed.validated,
                    signable,
                });
                candidate.cursor = parsed.end;
                candidate.validation_complete &= parsed.validated;
                Ok(())
            }
            CommandNodeKindState::Root => Err(syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                candidate.cursor,
                "root node cannot be parsed as a child",
            )),
        }
    }
}

fn normalized_command(source: &str) -> (&str, usize) {
    let trimmed = source.trim_start();
    let mut offset = source.len() - trimmed.len();
    let command = if let Some(command) = trimmed.strip_prefix('/') {
        offset += 1;
        let command = command.trim_start();
        offset += trimmed.len() - 1 - command.len();
        command
    } else {
        trimmed
    };
    (command, offset)
}

fn parse_argument(
    command: &str,
    start: usize,
    parser: &CommandArgumentParserState,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let template =
        command_argument_template(parser.type_id, &parser.properties).map_err(|error| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                start,
                format!("invalid {} template: {error}", parser.name),
            )
        })?;

    match (parser.type_id, template) {
        (VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID, CommandArgumentTemplate::Unit) => {
            parse_boolean(command, start)
        }
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
            CommandArgumentTemplate::Float { min, max },
        ) => parse_float(command, start, min, max),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
            CommandArgumentTemplate::Double { min, max },
        ) => parse_double(command, start, min, max),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
            CommandArgumentTemplate::Integer { min, max },
        ) => parse_integer(command, start, min, max),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID,
            CommandArgumentTemplate::Long { min, max },
        ) => parse_long(command, start, min, max),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
            CommandArgumentTemplate::String { mode },
        ) => parse_string(command, start, mode),
        (VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID, CommandArgumentTemplate::Unit) => {
            parse_message(command, start)
        }
        (type_id, template) => context_free::parse(type_id, command, start, &template)
            .unwrap_or_else(|| parse_unimplemented_family(command, start, &parser.name)),
    }
}

fn parse_boolean(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = parse_read_string(command, start)?;
    let raw = &command[start..end];
    let value = unquote_read_string(raw).unwrap_or(raw);
    match value {
        "true" | "false" => Ok(ParsedValue {
            end,
            validated: true,
        }),
        "" => Err(syntax_error(
            CommandSyntaxErrorKind::ExpectedBoolean,
            start,
            "expected boolean",
        )),
        _ => Err(syntax_error(
            CommandSyntaxErrorKind::InvalidBoolean,
            start,
            format!("invalid boolean {value}"),
        )),
    }
}

fn parse_integer(
    command: &str,
    start: usize,
    min: Option<i32>,
    max: Option<i32>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let value = number.parse::<i32>().map_err(|_| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidNumber,
            start,
            format!("invalid integer {number}"),
        )
    })?;
    check_bounds(value, min, max, start)?;
    Ok(ParsedValue {
        end,
        validated: true,
    })
}

fn parse_long(
    command: &str,
    start: usize,
    min: Option<i64>,
    max: Option<i64>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let value = number.parse::<i64>().map_err(|_| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidNumber,
            start,
            format!("invalid long {number}"),
        )
    })?;
    check_bounds(value, min, max, start)?;
    Ok(ParsedValue {
        end,
        validated: true,
    })
}

fn parse_float(
    command: &str,
    start: usize,
    min: Option<f32>,
    max: Option<f32>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let value = number.parse::<f32>().map_err(|_| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidNumber,
            start,
            format!("invalid float {number}"),
        )
    })?;
    check_bounds(
        value,
        Some(min.unwrap_or(-f32::MAX)),
        Some(max.unwrap_or(f32::MAX)),
        start,
    )?;
    Ok(ParsedValue {
        end,
        validated: true,
    })
}

fn parse_double(
    command: &str,
    start: usize,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let value = number.parse::<f64>().map_err(|_| {
        syntax_error(
            CommandSyntaxErrorKind::InvalidNumber,
            start,
            format!("invalid double {number}"),
        )
    })?;
    check_bounds(
        value,
        Some(min.unwrap_or(-f64::MAX)),
        Some(max.unwrap_or(f64::MAX)),
        start,
    )?;
    Ok(ParsedValue {
        end,
        validated: true,
    })
}

pub(super) fn number_lexeme(
    command: &str,
    start: usize,
) -> Result<(usize, &str), CommandSyntaxErrorState> {
    let mut end = start;
    while command
        .as_bytes()
        .get(end)
        .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'-'))
    {
        end += 1;
    }
    if end == start {
        return Err(syntax_error(
            CommandSyntaxErrorKind::ExpectedNumber,
            start,
            "expected number",
        ));
    }
    Ok((end, &command[start..end]))
}

fn check_bounds<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: Option<T>,
    max: Option<T>,
    cursor: usize,
) -> Result<(), CommandSyntaxErrorState> {
    if min.as_ref().is_some_and(|min| value < *min) {
        return Err(syntax_error(
            CommandSyntaxErrorKind::NumberTooLow,
            cursor,
            format!("number {value} is below the minimum"),
        ));
    }
    if max.as_ref().is_some_and(|max| value > *max) {
        return Err(syntax_error(
            CommandSyntaxErrorKind::NumberTooHigh,
            cursor,
            format!("number {value} is above the maximum"),
        ));
    }
    Ok(())
}

fn parse_string(
    command: &str,
    start: usize,
    mode: CommandStringMode,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let end = match mode {
        CommandStringMode::SingleWord => parse_unquoted_string(command, start),
        CommandStringMode::QuotablePhrase => parse_read_string(command, start)?,
        CommandStringMode::GreedyPhrase => command.len(),
    };
    Ok(ParsedValue {
        end,
        validated: true,
    })
}

fn parse_read_string(command: &str, start: usize) -> Result<usize, CommandSyntaxErrorState> {
    match command.as_bytes().get(start).copied() {
        Some(quote @ (b'\'' | b'\"')) => parse_quoted_string(command, start, quote),
        _ => Ok(parse_unquoted_string(command, start)),
    }
}

pub(super) fn parse_unquoted_string(command: &str, start: usize) -> usize {
    let mut end = start;
    while command.as_bytes().get(end).is_some_and(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'+')
    }) {
        end += 1;
    }
    end
}

fn parse_quoted_string(
    command: &str,
    start: usize,
    terminator: u8,
) -> Result<usize, CommandSyntaxErrorState> {
    let bytes = command.as_bytes();
    let mut cursor = start + 1;
    let mut escaped = false;
    while let Some(byte) = bytes.get(cursor).copied() {
        cursor += 1;
        if escaped {
            if byte != terminator && byte != b'\\' {
                return Err(syntax_error(
                    CommandSyntaxErrorKind::InvalidEscape,
                    cursor - 1,
                    format!("invalid escape {}", char::from(byte)),
                ));
            }
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == terminator {
            return Ok(cursor);
        }
    }
    Err(syntax_error(
        CommandSyntaxErrorKind::ExpectedQuoteEnd,
        command.len(),
        "expected quote terminator",
    ))
}

fn unquote_read_string(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    (bytes.len() >= 2
        && matches!(bytes.first(), Some(b'\'' | b'\"'))
        && bytes.first() == bytes.last())
    .then(|| &raw[1..raw.len() - 1])
}

fn parse_message(command: &str, start: usize) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let utf16_length = command[start..].encode_utf16().count();
    if utf16_length > 256 {
        return Err(syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            format!("message length {utf16_length} exceeds 256"),
        ));
    }
    Ok(ParsedValue {
        end: command.len(),
        // Entity-selector validation belongs to the context-free family phase.
        validated: !command[start..].contains('@'),
    })
}

fn parse_unimplemented_family(
    command: &str,
    start: usize,
    parser_name: &str,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let part_count = match parser_name {
        "minecraft:vec3" | "minecraft:block_pos" => 3,
        "minecraft:vec2" | "minecraft:rotation" | "minecraft:column_pos" => 2,
        _ => 1,
    };
    let mut end = start;
    for part in 0..part_count {
        if part != 0 {
            if command.as_bytes().get(end) != Some(&b' ') {
                return Err(syntax_error(
                    CommandSyntaxErrorKind::ExpectedSeparator,
                    end,
                    "expected a space between argument parts",
                ));
            }
            end += 1;
        }
        end = parse_balanced_token(command, end).ok_or_else(|| {
            syntax_error(
                CommandSyntaxErrorKind::InvalidArgument,
                end,
                format!("invalid {parser_name} argument"),
            )
        })?;
    }
    Ok(ParsedValue {
        end,
        validated: false,
    })
}

fn parse_balanced_token(command: &str, start: usize) -> Option<usize> {
    let bytes = command.as_bytes();
    let mut cursor = start;
    let mut escaped = false;
    let mut quote = None;
    let mut delimiters = Vec::new();
    while let Some(byte) = bytes.get(cursor).copied() {
        if escaped {
            escaped = false;
            cursor += 1;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            cursor += 1;
            continue;
        }
        if let Some(active_quote) = quote {
            if byte == active_quote {
                quote = None;
            }
            cursor += 1;
            continue;
        }
        match byte {
            b'\"' | b'\'' => quote = Some(byte),
            b'[' => delimiters.push(b']'),
            b'{' => delimiters.push(b'}'),
            b'(' => delimiters.push(b')'),
            b']' | b'}' | b')' if delimiters.last() == Some(&byte) => {
                delimiters.pop();
            }
            b' ' if delimiters.is_empty() => break,
            _ => {}
        }
        cursor += 1;
    }
    (cursor > start && quote.is_none() && delimiters.is_empty()).then_some(cursor)
}

pub(super) fn syntax_error(
    kind: CommandSyntaxErrorKind,
    cursor: usize,
    message: impl Into<String>,
) -> CommandSyntaxErrorState {
    CommandSyntaxErrorState {
        kind,
        cursor,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        CommandArgumentParser, VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
    };

    use super::*;
    use crate::commands::CommandNodeState;

    #[test]
    fn parses_primitives_with_brigadier_spans_and_bounds() {
        let tree = primitive_tree();

        let parsed = tree.parse_command("/set true 3 'hello world'");
        assert!(parsed.is_valid_command());
        assert!(parsed.restricted);
        assert_eq!(parsed.arguments.len(), 3);
        assert_eq!((parsed.arguments[0].start, parsed.arguments[0].end), (5, 9));
        assert_eq!(parsed.arguments[2].value, "'hello world'");

        let quoted_bool = boolean_tree().parse_command("flag \"true\"");
        assert!(quoted_bool.is_valid_command());

        let too_high = integer_tree().parse_command("number 11");
        assert_eq!(
            too_high.error.as_ref().map(|error| error.kind),
            Some(CommandSyntaxErrorKind::NumberTooHigh)
        );
        assert_eq!(too_high.error.unwrap().cursor, 7);

        let exponent = integer_tree().parse_command("number 1e2");
        assert_eq!(
            exponent.error.as_ref().map(|error| error.kind),
            Some(CommandSyntaxErrorKind::ExpectedSeparator)
        );
        assert_eq!(exponent.cursor, 7);
    }

    #[test]
    fn parses_all_numeric_primitive_families_and_exact_separator() {
        let float = numeric_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
            "brigadier:float",
            bounded_f32_properties(-1.0, 1.0),
        );
        assert!(float.parse_command("number .5").is_valid_command());
        assert_eq!(
            float
                .parse_command("number 2")
                .error
                .map(|error| error.kind),
            Some(CommandSyntaxErrorKind::NumberTooHigh)
        );

        let double = numeric_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
            "brigadier:double",
            vec![0],
        );
        assert!(double.parse_command("number -1.25").is_valid_command());
        let overflow = double.parse_command(&format!("number {}", "9".repeat(400)));
        assert_eq!(
            overflow.error.map(|error| error.kind),
            Some(CommandSyntaxErrorKind::NumberTooHigh)
        );

        let long = numeric_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID,
            "brigadier:long",
            vec![0],
        );
        assert!(long
            .parse_command("number 9223372036854775807")
            .is_valid_command());
        assert_eq!(
            long.parse_command("number --1")
                .error
                .map(|error| error.kind),
            Some(CommandSyntaxErrorKind::InvalidNumber)
        );

        let boolean = boolean_tree().parse_command("flag true\tignored");
        // A failed child resets the parse reader to the node-entry cursor;
        // the retained syntax exception still points at the bad separator.
        assert_eq!(boolean.cursor, 5);
        assert_eq!(boolean.error.as_ref().map(|error| error.cursor), Some(9));
        assert_eq!(
            boolean.error.map(|error| error.kind),
            Some(CommandSyntaxErrorKind::ExpectedSeparator)
        );
    }

    #[test]
    fn string_reader_rejects_invalid_escapes_and_reports_utf8_byte_spans() {
        let invalid_escape = quotable_tree().parse_command("name \"bad\\q\"");
        assert_eq!(
            invalid_escape.error.as_ref().map(|error| error.kind),
            Some(CommandSyntaxErrorKind::InvalidEscape)
        );
        assert_eq!(invalid_escape.error.unwrap().cursor, 10);

        let unterminated = quotable_tree().parse_command("name 'oops");
        assert_eq!(
            unterminated.error.as_ref().map(|error| error.kind),
            Some(CommandSyntaxErrorKind::ExpectedQuoteEnd)
        );

        let greedy = greedy_tree().parse_command("say 你好 world");
        assert!(greedy.is_valid_command());
        assert_eq!(greedy.arguments[0].start, 4);
        assert_eq!(greedy.arguments[0].end, "say 你好 world".len());
    }

    #[test]
    fn exact_literal_precedence_does_not_fall_back_to_argument() {
        let tree = literal_precedence_tree();
        let exact_dead_end = tree.parse_command("time extra");
        assert!(!exact_dead_end.is_valid_command());
        // Brigadier consumes the one argument separator before recursing.
        assert_eq!(exact_dead_end.cursor, 5);

        let argument = tree.parse_command("other");
        assert!(argument.is_valid_command());
        assert_eq!(argument.arguments[0].value, "other");
    }

    #[test]
    fn redirect_keeps_prior_arguments_and_signable_span() {
        let tree = redirect_tree();
        let parsed = tree.parse_command("relay Alex through hello world");

        assert!(!parsed.validation_complete);
        assert!(parsed.executable);
        assert_eq!(parsed.arguments.len(), 2);
        assert_eq!(
            parsed.signable_arguments,
            vec![SignableCommandArgument {
                name: "message".to_string(),
                value: "hello world".to_string(),
            }]
        );
    }

    #[test]
    fn branch_selection_prefers_full_consumption_then_no_error_stably() {
        let tree = branching_tree();
        let parsed = tree.parse_command("choose 4 tail");
        assert!(parsed.is_valid_command());
        assert_eq!(parsed.arguments[0].name, "head");
        assert_eq!(parsed.arguments[1].name, "tail");
    }

    #[test]
    fn context_free_family_marks_the_selected_dispatch_route_validated() {
        let tree = numeric_tree(
            bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
            "minecraft:gamemode",
            vec![],
        );

        let valid = tree.parse_command("number survival");
        assert!(valid.is_valid_command());
        assert!(valid.arguments[0].validated);

        let invalid = tree.parse_command("number builder");
        assert!(!invalid.is_valid_command());
        assert_eq!(
            invalid.error.map(|error| error.kind),
            Some(CommandSyntaxErrorKind::InvalidArgument)
        );
    }

    fn primitive_tree() -> CommandTreeState {
        tree(vec![
            literal("set", vec![2], false, false, None),
            argument(
                "enabled",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
                    "brigadier:bool",
                    vec![],
                ),
                vec![3],
                false,
                false,
                None,
            ),
            argument(
                "count",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
                    "brigadier:integer",
                    vec![3, 0, 0, 0, 1, 0, 0, 0, 10],
                ),
                vec![4],
                false,
                false,
                None,
            ),
            argument(
                "name",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    "brigadier:string",
                    vec![1],
                ),
                vec![],
                true,
                true,
                None,
            ),
        ])
    }

    fn boolean_tree() -> CommandTreeState {
        tree(vec![
            literal("flag", vec![2], false, false, None),
            argument(
                "value",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
                    "brigadier:bool",
                    vec![],
                ),
                vec![],
                true,
                false,
                None,
            ),
        ])
    }

    fn integer_tree() -> CommandTreeState {
        numeric_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
            "brigadier:integer",
            vec![3, 0, 0, 0, 0, 0, 0, 0, 10],
        )
    }

    fn numeric_tree(type_id: i32, name: &str, properties: Vec<u8>) -> CommandTreeState {
        tree(vec![
            literal("number", vec![2], false, false, None),
            argument(
                "value",
                parser(type_id, name, properties),
                vec![],
                true,
                false,
                None,
            ),
        ])
    }

    fn bounded_f32_properties(min: f32, max: f32) -> Vec<u8> {
        let mut properties = vec![3];
        properties.extend(min.to_be_bytes());
        properties.extend(max.to_be_bytes());
        properties
    }

    fn quotable_tree() -> CommandTreeState {
        string_tree("name", 1)
    }

    fn greedy_tree() -> CommandTreeState {
        string_tree("say", 2)
    }

    fn string_tree(literal_name: &str, mode: u8) -> CommandTreeState {
        tree(vec![
            literal(literal_name, vec![2], false, false, None),
            argument(
                "value",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    "brigadier:string",
                    vec![mode],
                ),
                vec![],
                true,
                false,
                None,
            ),
        ])
    }

    fn literal_precedence_tree() -> CommandTreeState {
        let mut tree = tree(vec![
            literal("time", vec![], false, false, None),
            argument(
                "fallback",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    "brigadier:string",
                    vec![0],
                ),
                vec![],
                true,
                false,
                None,
            ),
        ]);
        tree.nodes[0].children.push(2);
        tree
    }

    fn redirect_tree() -> CommandTreeState {
        tree(vec![
            literal("relay", vec![2], false, false, None),
            argument(
                "target",
                parser(
                    bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
                    "minecraft:entity",
                    vec![1],
                ),
                vec![3],
                false,
                false,
                None,
            ),
            literal("through", vec![], false, false, Some(4)),
            literal("redirect_target", vec![5], false, false, None),
            argument(
                "message",
                parser(
                    bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                    "minecraft:message",
                    vec![],
                ),
                vec![],
                true,
                false,
                None,
            ),
        ])
    }

    fn branching_tree() -> CommandTreeState {
        tree(vec![
            literal("choose", vec![2, 3], false, false, None),
            argument(
                "short",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
                    "brigadier:integer",
                    vec![0],
                ),
                vec![],
                true,
                false,
                None,
            ),
            argument(
                "head",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
                    "brigadier:integer",
                    vec![0],
                ),
                vec![4],
                false,
                false,
                None,
            ),
            argument(
                "tail",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    "brigadier:string",
                    vec![0],
                ),
                vec![],
                true,
                false,
                None,
            ),
        ])
    }

    fn tree(mut nodes: Vec<CommandNodeState>) -> CommandTreeState {
        nodes.insert(
            0,
            CommandNodeState {
                node_type: CommandNodeKindState::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
        );
        CommandTreeState {
            root_index: 0,
            nodes,
        }
    }

    fn literal(
        name: &str,
        children: Vec<i32>,
        executable: bool,
        restricted: bool,
        redirect: Option<i32>,
    ) -> CommandNodeState {
        CommandNodeState {
            node_type: CommandNodeKindState::Literal,
            flags: 1,
            children,
            redirect,
            name: Some(name.to_string()),
            parser: None,
            suggestions: None,
            executable,
            restricted,
        }
    }

    fn argument(
        name: &str,
        parser: CommandArgumentParser,
        children: Vec<i32>,
        executable: bool,
        restricted: bool,
        suggestions: Option<&str>,
    ) -> CommandNodeState {
        CommandNodeState {
            node_type: CommandNodeKindState::Argument,
            flags: 2,
            children,
            redirect: None,
            name: Some(name.to_string()),
            parser: Some(CommandArgumentParserState {
                type_id: parser.type_id,
                name: parser.name,
                properties: parser.properties,
            }),
            suggestions: suggestions.map(str::to_string),
            executable,
            restricted,
        }
    }

    fn parser(type_id: i32, name: &str, properties: Vec<u8>) -> CommandArgumentParser {
        CommandArgumentParser {
            type_id,
            name: name.to_string(),
            properties,
        }
    }
}
