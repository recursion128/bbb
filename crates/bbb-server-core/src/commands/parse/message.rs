//! Chat message text and its embedded selectors.

use super::{translatable, ArgumentValue, CommandPermission, ParseFailure, Selector};
use bbb_protocol::{JavaString, NbtSummaryValue};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) struct CommandMessage {
    pub text: JavaString,
    pub selectors: Vec<(usize, usize, Selector)>,
}

// MessageArgument.java:121-160.
pub(super) fn read_message(
    command: &[u16],
    cursor: usize,
    permission: CommandPermission,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let remaining = command.len() - cursor;
    if remaining > 256 {
        return Err(ParseFailure {
            message: translatable(
                "argument.message.too_long",
                vec![
                    NbtSummaryValue::Int(i32::try_from(remaining).unwrap_or(i32::MAX)),
                    NbtSummaryValue::Int(256),
                ],
            ),
            cursor: None,
        });
    }
    let mut selectors = Vec::new();
    let mut offset = cursor;
    // `parseText` (:127-130): without `COMMANDS_ENTITY_SELECTORS` the whole
    // remainder is plain text.
    if permission < CommandPermission::GameMaster {
        offset = command.len();
    }
    while let Some(at) = command[offset..].iter().position(|unit| *unit == 64) {
        let start = offset + at;
        let letter = command.get(start + 1).copied();
        if !letter.is_some_and(|letter| {
            char::from_u32(u32::from(letter))
                .and_then(super::SelectorKind::from_letter)
                .is_some()
        }) {
            offset = start + 1;
            continue;
        }
        let (end, selector) =
            super::read_selector_with_registry(command, start, permission, content_registry)?;
        selectors.push((start - cursor, end - cursor, selector));
        offset = end;
    }
    Ok((
        command.len(),
        ArgumentValue::Message(CommandMessage {
            text: command[cursor..].into(),
            selectors,
        }),
    ))
}
