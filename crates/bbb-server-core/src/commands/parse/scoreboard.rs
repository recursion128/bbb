//! The six argument types the scoreboard roots add to the walk.
//!
//! Each is its own vanilla `ArgumentType`, and each refuses with that type's
//! own key: nothing here reports a generic parse failure where vanilla names
//! the criterion, the slot or the operator that was not understood.
//!
//! Two of them resolve nothing at parse time. `minecraft:objective` reads a
//! bare word (`ObjectiveArgument.java:52-54`) and leaves the lookup to
//! `getObjective` (`:32-41`), which runs when the command does; a score holder
//! is a `Result` closure (`ScoreHolderArgument.java:213-216`) that only names
//! anyone once a source and a wildcard supplier are handed to it. Both are
//! carried here as what was typed rather than as what it means.

use super::super::{translatable, CommandPermission};
use super::{read_selector_with_registry, word_end, ArgumentValue, ParseFailure, Selector};
use bbb_protocol::scoreboard_display_slots::VANILLA_SCOREBOARD_DISPLAY_SLOTS;
use bbb_protocol::{
    component_literal_nbt, parse_command_snbt_value_prefix_utf16, JavaString, NbtSummaryValue,
};

/// `ScoreHolderArgument.Result` (`ScoreHolderArgument.java:213-216`) as the
/// four branches of `parse` (`:92-161`) build it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) enum ScoreHolderPattern {
    /// The `*` branch (`:109-118`), which answers whatever the wildcard
    /// supplier the reader passed in returns.
    Wildcard,
    /// A name that is not `*`. `fake` is the leading `#` (`:121-123`), which
    /// short-circuits to the name-only holder without consulting the server,
    /// and `uuid` is the branch `UUID.fromString` accepted (`:125-153`).
    Name {
        text: JavaString,
        fake: bool,
        uuid: bool,
    },
    /// The `@` branch (`:93-100`).
    Selector(Box<Selector>),
}

/// `OperationArgument.getOperation` (`OperationArgument.java:58-90`), which is
/// the swap plus the eight `SimpleOperation`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::commands) enum ScoreboardOperation {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Min,
    Max,
    Swap,
}

impl ScoreboardOperation {
    /// The nine operators, spelled as `listSuggestions` offers them (`:50`).
    const SPELLINGS: [(&'static str, Self); 9] = [
        ("=", Self::Assign),
        ("+=", Self::Add),
        ("-=", Self::Subtract),
        ("*=", Self::Multiply),
        ("/=", Self::Divide),
        ("%=", Self::Modulo),
        ("<", Self::Min),
        (">", Self::Max),
        ("><", Self::Swap),
    ];
}

/// `ObjectiveArgument.parse` (`ObjectiveArgument.java:52-54`):
/// `readUnquotedString` and nothing else. An empty word is accepted here and
/// refused by `getObjective` (`:32-41`) as an objective no scoreboard holds.
pub(in crate::commands) fn read_objective(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    Ok((
        end,
        ArgumentValue::Objective(
            JavaString::with_utf8_units(&command[cursor..end], str::to_owned)
                .expect("an unquoted word is ASCII"),
        ),
    ))
}

/// `ObjectiveCriteriaArgument.parse` (`ObjectiveCriteriaArgument.java:40-52`):
/// everything up to the next space, then `ObjectiveCriteria.byName` (`:99-111`),
/// with the cursor rewound to the start of the word before the refusal is
/// built (`:48-51`).
#[cfg(test)]
pub(in crate::commands) fn read_objective_criteria(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    read_objective_criteria_with_registry(
        command,
        cursor,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(in crate::commands) fn read_objective_criteria_with_registry(
    command: &[u16],
    cursor: usize,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = bbb_protocol::scan_command_token_utf16(command, cursor);
    let name = &command[cursor..end];
    let Some(criteria) = JavaString::with_utf8_units(name, |name| {
        bbb_server_persistence::canonical_objective_criteria_with_registry(name, content_registry)
    })
    .flatten() else {
        return Err(ParseFailure {
            message: translatable(
                "argument.criteria.invalid",
                vec![component_literal_nbt(name)],
            ),
            cursor: Some(cursor),
        });
    };
    Ok((end, ArgumentValue::ObjectiveCriteria(criteria)))
}

/// `ScoreboardSlotArgument.parse` (`ScoreboardSlotArgument.java:35-43`):
/// `readUnquotedString` through `DisplaySlot.CODEC.byName`, refused at the
/// cursor the word ended on rather than at the one it started from.
pub(in crate::commands) fn read_scoreboard_slot(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    let name = &command[cursor..end];
    match VANILLA_SCOREBOARD_DISPLAY_SLOTS.iter().find(|entry| {
        entry
            .serialized_name
            .encode_utf16()
            .eq(name.iter().copied())
    }) {
        Some(entry) => Ok((end, ArgumentValue::ScoreboardSlot(entry.slot))),
        None => Err(ParseFailure {
            message: translatable(
                "argument.scoreboardDisplaySlot.invalid",
                vec![component_literal_nbt(name)],
            ),
            cursor: Some(end),
        }),
    }
}

/// `ScoreHolderArgument.parse` (`ScoreHolderArgument.java:92-161`).
///
/// The `@` branch reuses the entity-selector reader, so a selector this server
/// does not model stays as fail-closed here as it is for `minecraft:entity`.
/// Everything else is read to the next space verbatim (`:102-108`), which is a
/// wider character set than `readUnquotedString`: `#fake` and a dotted UUID
/// both survive it.
pub(in crate::commands) fn read_score_holder_with_registry(
    command: &[u16],
    cursor: usize,
    permission: CommandPermission,
    multiple: bool,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    if command[cursor..].starts_with(&[64]) {
        let (end, selector) =
            read_selector_with_registry(command, cursor, permission, content_registry)?;
        // `EntityArgument.ERROR_NOT_SINGLE_ENTITY` (`:96-97`), thrown at the
        // cursor the selector ended on rather than at the command start.
        if !multiple && selector.max_results_exceeds_one() {
            return Err(ParseFailure {
                message: translatable("argument.entity.toomany", Vec::new()),
                cursor: Some(end),
            });
        }
        return Ok((
            end,
            ArgumentValue::ScoreHolders(ScoreHolderPattern::Selector(Box::new(selector))),
        ));
    }
    let end = bbb_protocol::scan_command_token_utf16(command, cursor);
    let text = &command[cursor..end];
    if text == [42] {
        return Ok((
            end,
            ArgumentValue::ScoreHolders(ScoreHolderPattern::Wildcard),
        ));
    }
    Ok((
        end,
        ArgumentValue::ScoreHolders(ScoreHolderPattern::Name {
            text: text.into(),
            fake: text.starts_with(&[35]),
            uuid: JavaString::with_utf8_units(text, bbb_protocol::parse_command_java_uuid)
                .flatten()
                .is_some(),
        }),
    ))
}

/// `OperationArgument.parse` (`OperationArgument.java:34-46`): the whole word
/// up to the next space, then the switch at `:58-90`. The empty-input refusal
/// (`:35-37`) and the unknown-operator one (`:88`) are the same key.
pub(in crate::commands) fn read_operation(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = bbb_protocol::scan_command_token_utf16(command, cursor);
    match ScoreboardOperation::SPELLINGS.iter().find(|(spelling, _)| {
        spelling
            .encode_utf16()
            .eq(command[cursor..end].iter().copied())
    }) {
        Some((_, operation)) => Ok((end, ArgumentValue::Operation(*operation))),
        None => Err(ParseFailure {
            message: translatable("arguments.operation.invalid", Vec::new()),
            cursor: Some(cursor),
        }),
    }
}

/// `StyleArgument` (`StyleArgument.java:27-29`): one SNBT value through
/// `Style.Serializer.CODEC`, with `withCodec`
/// (`CommandArgumentParser.java:38-45`) rewinding to the argument start and
/// reporting `argument.style.invalid` around the codec's own message.
pub(in crate::commands) fn read_style_with_registry(
    command: &[u16],
    cursor: usize,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (consumed, value) = match parse_command_snbt_value_prefix_utf16(&command[cursor..]) {
        Ok(parsed) => parsed,
        Err(failure) => {
            return Err(super::snbt_failure(cursor, &failure)
                .unwrap_or_else(|| invalid_style(cursor, failure.to_string())))
        }
    };
    if let Err(error) = bbb_protocol::RichTextStyle::try_from_style_nbt_with_registry(
        value.clone(),
        content_registry,
    ) {
        return Err(invalid_style(cursor, error));
    }
    Ok((cursor + consumed, ArgumentValue::Style(value)))
}

fn invalid_style(cursor: usize, message: impl Into<String>) -> ParseFailure {
    ParseFailure {
        message: translatable(
            "argument.style.invalid",
            vec![NbtSummaryValue::String(message.into().into())],
        ),
        cursor: Some(cursor),
    }
}
