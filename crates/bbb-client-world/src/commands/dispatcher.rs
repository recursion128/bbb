use rustc_hash::FxHashSet;

use bbb_protocol::packets::{
    command_argument_template, CommandArgumentTemplate, CommandStringMode,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
};
use bbb_protocol::{
    parse_command_entity_selector_prefix_utf16, scan_command_number_lexeme_utf16 as number_lexeme,
    scan_command_quotable_string_utf16 as parse_read_string,
    scan_command_unquoted_string_utf16 as parse_unquoted_string, CommandSyntaxErrorUtf16,
    JavaString,
};
use serde::{Deserialize, Serialize};

/// The dispatcher's name for one raised `CommandSyntaxException`. The grammar
/// scanners in `bbb-protocol` raise the same value, so a route that mixes
/// grammar faults with registry faults reports both through one type.
pub use bbb_protocol::CommandSyntaxError as CommandSyntaxErrorState;
pub use bbb_protocol::CommandSyntaxErrorKind;

use super::{
    CommandArgumentParserState, CommandNodeKindState, CommandTreeState, SignableCommandArgument,
};
use crate::FrozenRegistryAccess;

mod blocks;
mod completion;
mod context_free;
mod items;
mod particle;
mod registry_aware;
mod resource_or_id;
mod text;

pub use completion::matches_sub_str as matches_suggestion_sub_str;
pub(super) use completion::CommandCompletionSources;

/// One raised `CommandSyntaxException` in the raw terms a consumer of the edit
/// buffer needs: `cursor` is a UTF-16 unit index.
///
/// Vanilla raises some of these without parser context, so
/// `CommandSyntaxException.getContext()` is null and there is no cursor to
/// carry; `contextual` records that, while `cursor` still names the position the
/// raise came from. The raw consumer can therefore always map an error back
/// onto the buffer, and the byte-positioned projection below keeps Vanilla's
/// null-context shape for the callers that check it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCommandSyntaxErrorState {
    pub kind: CommandSyntaxErrorKind,
    pub cursor: usize,
    pub contextual: bool,
    pub message: JavaString,
}

impl RawCommandSyntaxErrorState {
    /// A raise that has no reader of its own -- a registry or codec rejection --
    /// positioned at the argument it came from.
    pub fn without_context(
        kind: CommandSyntaxErrorKind,
        cursor: usize,
        message: impl Into<JavaString>,
    ) -> Self {
        Self {
            kind,
            cursor,
            contextual: false,
            message: message.into(),
        }
    }
}

impl From<CommandSyntaxErrorUtf16> for RawCommandSyntaxErrorState {
    fn from(error: CommandSyntaxErrorUtf16) -> Self {
        Self {
            kind: error.kind,
            cursor: error.cursor,
            contextual: true,
            message: error.message,
        }
    }
}

/// The dispatcher's answer for raw UTF-16 input: every cursor is a unit index
/// into the buffer that was parsed, and every argument keeps its value units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCommandParseResult {
    /// Unit cursor into the original command, including a normalized leading
    /// slash/whitespace prefix when one was supplied.
    pub cursor: usize,
    pub fully_consumed: bool,
    pub executable: bool,
    pub restricted: bool,
    /// False while the selected route contains a parser family whose owning
    /// implementation phase has not landed yet. Such a route may still expose
    /// signed spans, but must not be certified as locally valid.
    pub validation_complete: bool,
    /// Index of the first entry in `arguments` the innermost redirect/fork
    /// context owns, per `CommandDispatcher.java:167-171`.
    pub innermost_context_argument_start: usize,
    pub arguments: Vec<RawParsedCommandArgument>,
    pub signable_arguments: Vec<SignableCommandArgument>,
    pub error: Option<RawCommandSyntaxErrorState>,
    pub completion: CommandCompletionContext,
}

impl RawCommandParseResult {
    pub fn is_valid_command(&self) -> bool {
        self.fully_consumed && self.executable && self.validation_complete && self.error.is_none()
    }
}

/// One parsed argument with the value units it was parsed from and the raw span
/// it occupies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawParsedCommandArgument {
    pub name: String,
    pub parser: String,
    /// First unit of the argument in the parsed buffer.
    pub start: usize,
    /// One past the last unit the argument consumed.
    pub end: usize,
    pub value: JavaString,
    pub validated: bool,
    pub signable: bool,
}

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
    /// Index of the first entry in `arguments` the innermost redirect/fork
    /// context owns. `formatText` colours only
    /// `currentParse.getContext().getLastChild().getArguments()`, and a
    /// redirect child starts a fresh `CommandContextBuilder`
    /// (CommandDispatcher.java:167-171), so everything parsed before the
    /// deepest redirect belongs to an outer context. Zero when the selected
    /// route never redirects, which makes the whole list one context.
    pub innermost_context_argument_start: usize,
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

/// Completion positions are UTF-16 units, which is what Brigadier's
/// `SuggestionContext.startPos` counts and what the edit buffer is indexed by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandCompletionContext {
    /// Node whose children own the suggestions at `start`.
    pub node_index: i32,
    pub start: usize,
    pub suggestions: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCompletionResult {
    pub start: usize,
    pub length: usize,
    pub suggestions: Vec<CommandCompletionSuggestion>,
    pub request_server: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCompletionSuggestion {
    /// The candidate the match produced, in the buffer's own units: it is
    /// spliced back into the edit box, so it may hold an unpaired surrogate.
    pub text: JavaString,
    /// Display-only text; `StringTooltip` never takes part in the match.
    pub tooltip: Option<String>,
    pub tooltip_translation_key: Option<String>,
}

/// One raised `CommandSyntaxErrorUtf16` positioned at the argument it came from.
fn syntax_error(
    kind: CommandSyntaxErrorKind,
    cursor: usize,
    message: impl Into<JavaString>,
) -> RawCommandSyntaxErrorState {
    RawCommandSyntaxErrorState {
        kind,
        cursor,
        contextual: true,
        message: message.into(),
    }
}

/// The `StringReader` position of `unit` inside `text`.
///
/// `text` is the projection of the buffer being parsed, whose unit count equals
/// the buffer's, so this is the byte offset the `&str`-shaped scanners need and
/// its inverse [`unit_of_byte`] maps their cursors back to raw units.
fn byte_of_unit(text: &str, unit: usize) -> usize {
    let mut cursor = 0;
    for (byte, ch) in text.char_indices() {
        if cursor >= unit {
            return byte;
        }
        cursor += ch.len_utf16();
    }
    text.len()
}

fn unit_of_byte(text: &str, byte: usize) -> usize {
    text.char_indices()
        .take_while(|(index, _)| *index < byte)
        .map(|(_, ch)| ch.len_utf16())
        .sum()
}

/// Text for the `&str`-shaped entries of the parse path and for its reports,
/// never for a position and never for a candidate text: the projection keeps
/// one unit per buffer unit, so [`unit_of_byte`] maps its byte offsets back
/// onto raw ones, while an isolated surrogate reaches those consumers as
/// U+FFFD.
fn text_of(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

fn eq_ascii(units: &[u16], expected: &str) -> bool {
    expected.encode_utf16().eq(units.iter().copied())
}

#[derive(Clone)]
struct ParseCandidate {
    cursor: usize,
    current_node: i32,
    executable: bool,
    restricted: bool,
    validation_complete: bool,
    innermost_context_argument_start: usize,
    arguments: Vec<RawParsedCommandArgument>,
    error: Option<RawCommandSyntaxErrorState>,
    suggestions: Option<String>,
    parsed_any_node: bool,
    completion_parent: i32,
    completion_start: usize,
}

impl ParseCandidate {
    fn root(root_index: i32) -> Self {
        Self {
            cursor: 0,
            current_node: root_index,
            executable: false,
            restricted: false,
            validation_complete: true,
            innermost_context_argument_start: 0,
            arguments: Vec::new(),
            error: None,
            suggestions: None,
            parsed_any_node: false,
            completion_parent: root_index,
            completion_start: 0,
        }
    }
}

#[derive(Debug)]
pub(super) struct ParsedValue {
    pub(super) end: usize,
    pub(super) validated: bool,
}

const MAX_COMMAND_PARSE_DEPTH: usize = 256;
const MAX_COMMAND_PARSE_STEPS: usize = 16_384;

impl CommandTreeState {
    pub(super) fn check_unattended_command(
        &self,
        source: &str,
        registry_access: Option<FrozenRegistryAccess<'_>>,
    ) -> super::UnattendedCommandCheck {
        self.check_unattended_command_utf16(
            &source.encode_utf16().collect::<Vec<_>>(),
            registry_access,
        )
    }

    pub(super) fn check_unattended_command_utf16(
        &self,
        source: &[u16],
        registry_access: Option<FrozenRegistryAccess<'_>>,
    ) -> super::UnattendedCommandCheck {
        use super::UnattendedCommandCheck;
        // Vanilla's unattended-command check parses the value exactly as typed:
        // the leading slash is already gone by the time the screen calls it.
        let current = self.parse_raw_command(source, registry_access, true, false);
        if !current.is_valid_command() {
            UnattendedCommandCheck::ParseErrors
        } else if !current.signable_arguments.is_empty() {
            UnattendedCommandCheck::SignatureRequired
        } else if !self
            .parse_raw_command(source, registry_access, false, false)
            .is_valid_command()
        {
            UnattendedCommandCheck::PermissionsRequired
        } else {
            UnattendedCommandCheck::NoIssues
        }
    }

    pub fn parse_command(&self, source: &str) -> CommandParseResult {
        self.parse_command_with_registry(source, None)
    }

    /// Parses raw UTF-16 units, which is the source of truth: Brigadier's
    /// `StringReader` counts Java `char`s and the chat edit box stores units, so
    /// an isolated surrogate stays distinguishable from U+FFFD throughout.
    pub fn parse_command_utf16(&self, source: &[u16]) -> RawCommandParseResult {
        self.parse_raw_command_with_registry(source, None)
    }

    pub fn parse_raw_command_with_registry(
        &self,
        source: &[u16],
        registry_access: Option<FrozenRegistryAccess<'_>>,
    ) -> RawCommandParseResult {
        self.parse_raw_command(source, registry_access, true, true)
    }

    pub(crate) fn parse_command_with_registry(
        &self,
        source: &str,
        registry_access: Option<FrozenRegistryAccess<'_>>,
    ) -> CommandParseResult {
        byte_parse_result(
            source,
            self.parse_raw_command_with_registry(
                &source.encode_utf16().collect::<Vec<_>>(),
                registry_access,
            ),
        )
    }

    fn parse_raw_command(
        &self,
        source: &[u16],
        registry_access: Option<FrozenRegistryAccess<'_>>,
        allow_restricted: bool,
        normalize_prefix: bool,
    ) -> RawCommandParseResult {
        let (command, source_offset) = if normalize_prefix {
            normalized_command(source)
        } else {
            (source, 0)
        };
        let mut active = FxHashSet::default();
        let mut failed_states = FxHashSet::default();
        let mut remaining_steps = MAX_COMMAND_PARSE_STEPS;
        let candidate = self.parse_nodes(
            self.root_index,
            command,
            ParseCandidate::root(self.root_index),
            &mut active,
            &mut failed_states,
            0,
            &mut remaining_steps,
            registry_access,
            allow_restricted,
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

        RawCommandParseResult {
            cursor: candidate.cursor + source_offset,
            fully_consumed,
            executable: candidate.executable,
            restricted: candidate.restricted,
            validation_complete: candidate.validation_complete,
            innermost_context_argument_start: candidate.innermost_context_argument_start,
            arguments,
            signable_arguments,
            error,
            completion: CommandCompletionContext {
                node_index: candidate.completion_parent,
                start: candidate.completion_start + source_offset,
                suggestions: candidate.suggestions,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_nodes(
        &self,
        node_index: i32,
        command: &[u16],
        context: ParseCandidate,
        active: &mut FxHashSet<(i32, usize)>,
        failed_states: &mut FxHashSet<(i32, usize)>,
        depth: usize,
        remaining_steps: &mut usize,
        registry_access: Option<FrozenRegistryAccess<'_>>,
        allow_restricted: bool,
    ) -> ParseCandidate {
        if depth >= MAX_COMMAND_PARSE_DEPTH || *remaining_steps == 0 {
            return ParseCandidate {
                error: Some(syntax_error(
                    CommandSyntaxErrorKind::InvalidArgument,
                    context.cursor,
                    "command parse limit exceeded",
                )),
                ..context
            };
        }
        *remaining_steps -= 1;

        let state_key = (node_index, context.cursor);
        if failed_states.contains(&state_key) {
            return ParseCandidate {
                error: Some(syntax_error(
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
                )),
                ..context
            };
        }
        if !active.insert(state_key) {
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
            if !allow_restricted && child.restricted {
                continue;
            }
            let mut candidate = context.clone();
            candidate.error = None;

            match self.parse_child(child_index, command, &mut candidate, registry_access) {
                Ok(()) => {}
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            }

            candidate.completion_parent = node_index;
            candidate.completion_start = context.cursor;

            if command
                .get(candidate.cursor)
                .is_some_and(|unit| *unit != 32)
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
                    candidate.completion_parent = redirect;
                    candidate.completion_start = candidate.cursor;
                    // The redirect target parses into a fresh context, so the
                    // arguments pushed from here on are the innermost ones. A
                    // deeper redirect overwrites this with a later boundary,
                    // which is what `getLastChild()` walks down to.
                    candidate.innermost_context_argument_start = candidate.arguments.len();
                    let result = self.parse_nodes(
                        redirect,
                        command,
                        candidate,
                        active,
                        failed_states,
                        depth + 1,
                        remaining_steps,
                        registry_access,
                        allow_restricted,
                    );
                    active.remove(&state_key);
                    if result.error.is_some() || result.cursor != command.len() {
                        failed_states.insert(state_key);
                    }
                    return result;
                }
                candidate.completion_parent = child_index;
                candidate.completion_start = candidate.cursor;
                let next = self.parse_nodes(
                    child_index,
                    command,
                    candidate,
                    active,
                    failed_states,
                    depth + 1,
                    remaining_steps,
                    registry_access,
                    allow_restricted,
                );
                potentials.push(next);
            } else {
                // Brigadier keeps this node's range out of the context only
                // while the cursor is inside it; past it,
                // `CommandContextBuilder.findSuggestionContext` (1.3.10) takes
                // the last parsed node as the parent and its end plus one as
                // the start, so a trailing separator reaches this node's own
                // children instead of re-offering the node itself.
                if candidate.cursor < command.len() {
                    candidate.completion_parent = child_index;
                    candidate.completion_start = candidate.cursor + 1;
                }
                potentials.push(candidate);
            }
        }

        active.remove(&state_key);

        if !potentials.is_empty() {
            potentials.sort_by_key(|candidate| {
                (candidate.cursor != command.len(), candidate.error.is_some())
            });
            let best = potentials.remove(0);
            if best.error.is_some() || best.cursor != command.len() {
                failed_states.insert(state_key);
            }
            return best;
        }

        failed_states.insert(state_key);
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

    fn relevant_children(&self, node_index: i32, command: &[u16], cursor: usize) -> Vec<i32> {
        let Some(node) = self.node(node_index) else {
            return Vec::new();
        };
        // CommandNode.addChild keeps the first child for a name in one
        // LinkedHashMap, then indexes that same owner by its original node
        // kind. Preserve both first-owner and insertion-order semantics.
        let mut child_names = FxHashSet::default();
        let mut arguments = Vec::new();
        let mut literal_match = None;
        let suffix = command.get(cursor..).unwrap_or_default();
        let token_end = suffix
            .iter()
            .position(|unit| *unit == 32)
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
                    if eq_ascii(token, name) && literal_match.is_none() {
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
        command: &[u16],
        candidate: &mut ParseCandidate,
        registry_access: Option<FrozenRegistryAccess<'_>>,
    ) -> Result<(), RawCommandSyntaxErrorState> {
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
                let units: Vec<u16> = literal.encode_utf16().collect();
                let end = candidate.cursor.saturating_add(units.len());
                let matches = command
                    .get(candidate.cursor..end)
                    .is_some_and(|token| token == units.as_slice())
                    && command.get(end).is_none_or(|unit| *unit == 32);
                if matches {
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
                let parsed = parse_argument(command, start, parser, registry_access)?;
                let signable = parser.type_id == VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID;
                candidate.arguments.push(RawParsedCommandArgument {
                    name: child.name.clone().unwrap_or_default(),
                    parser: parser.name.clone(),
                    start,
                    end: parsed.end,
                    value: JavaString::from(command[start..parsed.end].to_vec()),
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

/// The byte-positioned projection of a raw parse, for the `&str` entries.
fn byte_parse_result(source: &str, raw: RawCommandParseResult) -> CommandParseResult {
    let byte = |unit: usize| byte_of_unit(source, unit);
    CommandParseResult {
        cursor: byte(raw.cursor),
        fully_consumed: raw.fully_consumed,
        executable: raw.executable,
        restricted: raw.restricted,
        validation_complete: raw.validation_complete,
        innermost_context_argument_start: raw.innermost_context_argument_start,
        arguments: raw
            .arguments
            .into_iter()
            .map(|argument| ParsedCommandArgument {
                name: argument.name,
                parser: argument.parser,
                start: byte(argument.start),
                end: byte(argument.end),
                value: text_of(argument.value.units()),
                validated: argument.validated,
                signable: argument.signable,
            })
            .collect(),
        signable_arguments: raw.signable_arguments,
        error: raw.error.map(|error| CommandSyntaxErrorState {
            // A raise with no reader of its own keeps Vanilla's null context.
            cursor: error.contextual.then(|| byte(error.cursor)),
            kind: error.kind,
            message: text_of(error.message.units()),
        }),
        completion: CommandCompletionContext {
            node_index: raw.completion.node_index,
            start: byte(raw.completion.start),
            suggestions: raw.completion.suggestions,
        },
    }
}

fn normalized_command(source: &[u16]) -> (&[u16], usize) {
    let text = text_of(source);
    let trimmed = text.trim_start();
    let mut offset = unit_of_byte(&text, text.len() - trimmed.len());
    if let Some(command) = trimmed.strip_prefix('/') {
        offset += 1;
        offset += unit_of_byte(command, command.len() - command.trim_start().len());
    }
    (&source[offset..], offset)
}

fn parse_argument(
    command: &[u16],
    start: usize,
    parser: &CommandArgumentParserState,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
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
            .or_else(|| blocks::parse(type_id, command, start, registry_access))
            .or_else(|| items::parse(type_id, command, start, registry_access))
            .or_else(|| particle::parse(type_id, command, start, registry_access))
            .or_else(|| resource_or_id::parse(type_id, command, start, registry_access))
            .or_else(|| text::parse(type_id, command, start, registry_access))
            .or_else(|| registry_aware::parse(type_id, command, start, &template, registry_access))
            .unwrap_or_else(|| parse_unimplemented_family(command, start, &parser.name)),
    }
}

fn parse_boolean(command: &[u16], start: usize) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let end = parse_read_string(command, start)?;
    let raw = &command[start..end];
    let value = unquote_read_string(raw).unwrap_or(raw);
    if eq_ascii(value, "true") || eq_ascii(value, "false") {
        Ok(ParsedValue {
            end,
            validated: true,
        })
    } else if value.is_empty() {
        Err(syntax_error(
            CommandSyntaxErrorKind::ExpectedBoolean,
            start,
            "expected boolean",
        ))
    } else {
        Err(syntax_error(
            CommandSyntaxErrorKind::InvalidBoolean,
            start,
            format!("invalid boolean {}", text_of(value)),
        ))
    }
}

fn parse_integer(
    command: &[u16],
    start: usize,
    min: Option<i32>,
    max: Option<i32>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let number = text_of(number);
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
    command: &[u16],
    start: usize,
    min: Option<i64>,
    max: Option<i64>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let number = text_of(number);
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
    command: &[u16],
    start: usize,
    min: Option<f32>,
    max: Option<f32>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let number = text_of(number);
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
    command: &[u16],
    start: usize,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let (end, number) = number_lexeme(command, start)?;
    let number = text_of(number);
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

fn check_bounds<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: Option<T>,
    max: Option<T>,
    cursor: usize,
) -> Result<(), RawCommandSyntaxErrorState> {
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
    command: &[u16],
    start: usize,
    mode: CommandStringMode,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
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

fn unquote_read_string(raw: &[u16]) -> Option<&[u16]> {
    (raw.len() >= 2 && matches!(raw.first(), Some(39 | 34)) && raw.first() == raw.last())
        .then(|| &raw[1..raw.len() - 1])
}

/// `minecraft:message`: the reader's remaining length, in Java `char`s, against
/// Vanilla's 256 unit cap (`MessageArgument.java:121-124`). The raise carries no
/// reader of its own there, so the position recorded is the argument start.
fn parse_message(command: &[u16], start: usize) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let utf16_length = command.len().saturating_sub(start);
    if utf16_length > 256 {
        return Err(syntax_error(
            CommandSyntaxErrorKind::InvalidArgument,
            start,
            format!("message length {utf16_length} exceeds 256"),
        ));
    }
    validate_message_selectors(command, start)?;
    Ok(ParsedValue {
        end: command.len(),
        validated: true,
    })
}

fn validate_message_selectors(
    command: &[u16],
    start: usize,
) -> Result<(), RawCommandSyntaxErrorState> {
    let mut cursor = start;
    while let Some(offset) = command
        .get(cursor..)
        .unwrap_or_default()
        .iter()
        .position(|unit| *unit == 64)
    {
        cursor += offset;
        let selector_type = command.get(cursor + 1).copied();
        if !matches!(selector_type, Some(97 | 101 | 110 | 112 | 114 | 115)) {
            cursor += 1;
            continue;
        }
        let selector =
            parse_command_entity_selector_prefix_utf16(&command[cursor..]).ok_or_else(|| {
                syntax_error(
                    CommandSyntaxErrorKind::InvalidArgument,
                    cursor,
                    "invalid entity selector in message",
                )
            })?;
        cursor += selector.consumed;
    }
    Ok(())
}

fn parse_unimplemented_family(
    command: &[u16],
    start: usize,
    parser_name: &str,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let part_count = match parser_name {
        "minecraft:vec3" | "minecraft:block_pos" => 3,
        "minecraft:vec2" | "minecraft:rotation" | "minecraft:column_pos" => 2,
        _ => 1,
    };
    let mut end = start;
    for part in 0..part_count {
        if part != 0 {
            if command.get(end) != Some(&32) {
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

fn parse_balanced_token(command: &[u16], start: usize) -> Option<usize> {
    let mut cursor = start;
    let mut escaped = false;
    let mut quote = None;
    let mut delimiters = Vec::new();
    while let Some(unit) = command.get(cursor).copied() {
        if escaped {
            escaped = false;
            cursor += 1;
            continue;
        }
        if unit == 92 {
            escaped = true;
            cursor += 1;
            continue;
        }
        if let Some(active_quote) = quote {
            if unit == active_quote {
                quote = None;
            }
            cursor += 1;
            continue;
        }
        match unit {
            34 | 39 => quote = Some(unit),
            91 => delimiters.push(93),
            123 => delimiters.push(125),
            40 => delimiters.push(41),
            93 | 125 | 41 if delimiters.last() == Some(&unit) => {
                delimiters.pop();
            }
            32 if delimiters.is_empty() => break,
            _ => {}
        }
        cursor += 1;
    }
    (cursor > start && quote.is_none() && delimiters.is_empty()).then_some(cursor)
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

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn command_particle_codec_error_has_no_context_after_dispatch_and_serialization() {
        let tree = tree(vec![
            literal("emit", vec![2], false, false, None),
            argument(
                "value",
                parser(
                    bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID,
                    "minecraft:particle",
                    vec![],
                ),
                vec![],
                true,
                false,
                None,
            ),
        ]);
        let registries = crate::RegistrySet::vanilla_26_1();
        for input in [
            " /emit dust",
            " /emit dust{color:1,scale:8}",
            " /emit item",
            " /emit item{item:{id:'missing'}}",
        ] {
            let parsed = tree.parse_command_with_registry(input, Some(registries.frozen_access()));
            let error = parsed.error.unwrap();
            assert_eq!(error.cursor, None, "{input}");
            let encoded = serde_json::to_value(&error).unwrap();
            assert!(encoded["cursor"].is_null());
            assert_eq!(
                serde_json::from_value::<CommandSyntaxErrorState>(encoded).unwrap(),
                error
            );
        }
        let parsed =
            tree.parse_command_with_registry(" /emit dust{", Some(registries.frozen_access()));
        assert!(parsed.error.unwrap().cursor.is_some());
    }

    #[test]
    fn unattended_command_check_orders_parse_signature_and_permissions() {
        use crate::commands::UnattendedCommandCheck::*;
        let ordinary = boolean_tree();
        assert_eq!(
            ordinary.check_unattended_command("flag true", None),
            NoIssues
        );
        assert_eq!(ordinary.check_unattended_command("flag", None), ParseErrors);
        assert_eq!(
            ordinary.check_unattended_command(" flag true", None),
            ParseErrors
        );
        assert_eq!(
            ordinary.check_unattended_command("/flag true", None),
            ParseErrors
        );
        let restricted = primitive_tree();
        assert_eq!(
            restricted.check_unattended_command("set true 3 tail", None),
            PermissionsRequired
        );
        let mut signed = redirect_tree();
        signed.nodes[1].restricted = true;
        assert_eq!(
            signed.check_unattended_command("relay Alex through hello", None),
            SignatureRequired
        );
        assert_eq!(
            signed.check_unattended_command("relay Alex through @e[type=missing]", None),
            ParseErrors
        );
    }

    #[test]
    fn unattended_reparse_uses_an_unrestricted_alternative_argument() {
        let arg = |name, restricted| {
            argument(
                name,
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    "brigadier:string",
                    vec![0],
                ),
                vec![],
                true,
                restricted,
                None,
            )
        };
        let tree = tree(vec![
            literal("choose", vec![2, 3], false, false, None),
            arg("privileged", true),
            arg("ordinary", false),
        ]);
        assert!(tree.parse_command("choose value").restricted);
        let without = tree.parse_raw_command(&units("choose value"), None, false, false);
        assert!(without.is_valid_command());
        assert_eq!(without.arguments[0].name, "ordinary");
        assert_eq!(
            tree.check_unattended_command("choose value", None),
            crate::commands::UnattendedCommandCheck::NoIssues
        );
    }

    #[test]
    fn unattended_reparse_preserves_literal_priority_and_redirect_requirements() {
        let mut exact = literal_precedence_tree();
        exact.nodes[1].restricted = true;
        assert!(!exact
            .parse_raw_command(&units("time"), None, false, false)
            .is_valid_command());
        assert!(exact
            .parse_raw_command(&units("other"), None, false, false)
            .is_valid_command());
        let mut redirected = redirect_tree();
        redirected.nodes[4].restricted = true;
        // Brigadier redirects into the target's children without invoking its requirement.
        assert!(redirected
            .parse_raw_command(&units("relay Alex through hello"), None, false, false)
            .is_valid_command());
        redirected.nodes[5].restricted = true;
        assert!(!redirected
            .parse_raw_command(&units("relay Alex through hello"), None, false, false)
            .is_valid_command());
    }

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
        assert_eq!(too_high.error.unwrap().cursor, Some(7));

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
        assert_eq!(
            boolean.error.as_ref().map(|error| error.cursor),
            Some(Some(9))
        );
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
        assert_eq!(invalid_escape.error.unwrap().cursor, Some(10));

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

        assert!(parsed.is_valid_command());
        assert!(parsed.executable);
        assert_eq!(parsed.arguments.len(), 2);
        assert_eq!(
            parsed.signable_arguments,
            vec![SignableCommandArgument {
                name: "message".to_string(),
                value: "hello world".into(),
            }]
        );
    }

    #[test]
    fn parse_records_the_innermost_context_argument_boundary() {
        // No redirect: the flat list is one context, so `getLastChild()` is
        // the root context and owns every argument.
        let plain = primitive_tree().parse_command("set true 3 'hello world'");
        assert_eq!(plain.innermost_context_argument_start, 0);

        // `through` redirects after `target` was parsed, so the innermost
        // context owns only `message` (CommandDispatcher.java:167-171).
        let redirected = redirect_tree().parse_command("relay Alex through hello world");
        assert_eq!(redirected.innermost_context_argument_start, 1);
        assert_eq!(redirected.arguments[0].name, "target");
        assert_eq!(redirected.arguments[1].name, "message");
    }

    /// Root -> "say" -> one `minecraft:message` argument.
    fn message_tree() -> CommandTreeState {
        tree(vec![
            literal("say", vec![2], false, false, None),
            argument(
                "message",
                parser(
                    VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
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

    #[test]
    fn raw_arguments_span_the_redirect_context_in_units() {
        let tree = redirect_tree();
        let raw: Vec<u16> = "relay Alex through 😀x".encode_utf16().collect();
        let parsed = tree.parse_command_utf16(&raw);

        assert!(parsed.is_valid_command());
        // `through` redirects after `target` was parsed, so the innermost
        // context owns the message only (CommandDispatcher.java:167-171).
        assert_eq!(parsed.innermost_context_argument_start, 1);
        assert_eq!(parsed.arguments[0].name, "target");
        assert_eq!(parsed.arguments[0].start, units("relay ").len());
        assert_eq!(parsed.arguments[0].end, units("relay Alex").len());
        assert_eq!(parsed.arguments[1].name, "message");
        // The message starts after `through `, counted in Java `char`s: the
        // emoji before it is two units, not the four bytes a String cursor
        // would have reported.
        assert_eq!(
            parsed.arguments[1].start,
            units("relay Alex through ").len()
        );
        assert_eq!(parsed.arguments[1].end, raw.len());
        assert_eq!(parsed.arguments[1].value.units(), units("😀x").as_slice());
        assert_eq!(parsed.cursor, raw.len());
        assert_eq!(
            parsed.signable_arguments,
            vec![SignableCommandArgument {
                name: "message".to_string(),
                value: units("😀x").into(),
            }]
        );
    }

    #[test]
    fn raw_message_argument_caps_at_256_units_and_reports_the_unit_cursor() {
        let tree = message_tree();
        // 128 surrogate pairs are 256 Java chars but 512 UTF-8 bytes; Vanilla
        // measures `StringReader.getRemainingLength()` (MessageArgument.java:121-124).
        let mut raw: Vec<u16> = units("say ");
        for _ in 0..128 {
            raw.extend([0xd83d, 0xde00]);
        }
        let parsed = tree.parse_command_utf16(&raw);
        assert_eq!(parsed.arguments[0].value.units().len(), 256);
        assert_eq!(parsed.arguments[0].start, 4);
        assert!(
            parsed.is_valid_command(),
            "256 units is the last accepted message"
        );

        for unit in [0x61u16, 0xd800] {
            let mut over: Vec<u16> = units("say ");
            over.extend(std::iter::repeat_n(unit, 257));
            let parsed = tree.parse_command_utf16(&over);
            let error = parsed.error.expect("257 units is over the cap");
            assert_eq!(error.kind, CommandSyntaxErrorKind::InvalidArgument);
            // The argument start, in raw units, even though Vanilla raises this
            // one without a reader of its own.
            assert_eq!(error.cursor, 4);
            assert_eq!(
                error.message.units(),
                units("message length 257 exceeds 256").as_slice()
            );
        }
    }

    #[test]
    fn raw_errors_carry_the_unit_cursor_they_were_raised_at() {
        let tree = message_tree();
        // `say` is a literal, so `/sez 😀` fails at the literal's own position.
        let parsed = tree.parse_command_utf16(&units("/sez 😀"));
        let error = parsed.error.expect("unknown command");
        assert_eq!(error.kind, CommandSyntaxErrorKind::UnknownCommand);
        assert_eq!(error.cursor, 1);
        assert!(error.contextual);

        // A findable argument failure keeps the offending unit too.
        let tree = quotable_tree();
        let parsed = tree.parse_command_utf16(&units("/name 'oops"));
        let error = parsed.error.expect("unterminated quote");
        assert_eq!(error.kind, CommandSyntaxErrorKind::ExpectedQuoteEnd);
        assert_eq!(error.cursor, units("/name 'oops").len());
        assert!(error.contextual);
    }

    #[test]
    fn message_validates_selector_parts_but_ignores_unknown_at_sequences() {
        assert!(parse_message(&units("hello @e[type=minecraft:pig]"), 0).is_ok());
        assert!(parse_message(&units("email@example and @unknown"), 0).is_ok());
        let error = match parse_message(&units("broken @e[type=missing]"), 0) {
            Err(error) => error,
            Ok(_) => panic!("malformed known selector was accepted"),
        };
        assert_eq!(error.kind, CommandSyntaxErrorKind::InvalidArgument);
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
            completion_target: crate::commands::CommandCompletionTarget::default(),
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
