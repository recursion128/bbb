//! The grammar of `minecraft:item_stack` and `minecraft:item_predicate`.
//!
//! Both arguments carry a bracketed list whose entries a caller has to check
//! one at a time -- vanilla validates each component value the moment it has
//! read it, so a list holding both a bad value and a later syntax fault reports
//! the value. A scan that read the whole list first would report the fault
//! instead, so the lists are handed out through cursor-driven scanners: the
//! caller pulls one entry, resolves it, and only then pulls the next.
//!
//! Resolving a component *name* stays here, because the `DataComponentType`
//! registry is built in -- it is a generated catalog in this crate, not
//! anything a datapack can move -- and because the name is what decides the
//! shape of the rest of the entry.

use std::{borrow::Cow, collections::BTreeSet};

use super::{
    scan_command_resource_location_utf16, scan_command_whitespace_utf16, strings,
    CommandSyntaxError, CommandSyntaxErrorKind, CommandSyntaxErrorUtf16,
};
use crate::{
    data_component_types::{
        vanilla_data_component_type_for_resource_id, DataComponentTypeCatalogEntry,
        VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
    },
    NbtSummaryValue, SnbtParseFailure,
};

/// The component type a component name stands for, or `None` when the name is
/// not a component that survives being written to disk -- the only ones an
/// item argument may spell.
pub fn command_persistent_data_component(
    resource_id: &str,
) -> Option<&'static DataComponentTypeCatalogEntry> {
    vanilla_data_component_type_for_resource_id(resource_id).filter(|entry| entry.persistent)
}

/// What an item predicate's `name=value` test may name: `Some(Some(entry))` for
/// a component, `Some(None)` for `minecraft:count` (which is a stack field
/// rather than a component), `None` for anything else.
pub fn command_item_predicate_component(
    resource_id: &str,
) -> Option<Option<&'static DataComponentTypeCatalogEntry>> {
    if resource_id == "minecraft:count" {
        Some(None)
    } else {
        command_persistent_data_component(resource_id).map(Some)
    }
}

/// Whether a name may appear on the left of an item predicate's `name~value`
/// sub-predicate test.
pub fn command_known_item_predicate(resource_id: &str) -> bool {
    resource_id == "minecraft:count"
        || VANILLA_DATA_COMPONENT_PREDICATE_TYPES
            .iter()
            .any(|entry| entry.resource_id == resource_id)
        || vanilla_data_component_type_for_resource_id(resource_id).is_some()
}

/// One entry of an item stack's `[...]` component patch.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedCommandItemComponent {
    /// Component-name offset, past any `!`.
    pub start: usize,
    /// Whether the entry was written as `!name`, which removes the component
    /// rather than assigning it and so carries no value.
    pub removed: bool,
    pub component: &'static DataComponentTypeCatalogEntry,
    pub value: Option<NbtSummaryValue>,
    /// Assigned-value offset; absent for removals.
    pub value_start: Option<usize>,
}

impl ScannedCommandItemComponent {
    fn map_offsets(self, to_byte: &dyn Fn(usize) -> usize) -> Self {
        Self {
            start: to_byte(self.start),
            removed: self.removed,
            component: self.component,
            value: self.value,
            value_start: self.value_start.map(to_byte),
        }
    }
}

pub type ScannedCommandItemComponentUtf16 = ScannedCommandItemComponent;

/// Candidates at the cursor where an item component patch stopped scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandItemComponentSuggestions {
    Components { removed: bool },
    Assignment,
    Separator,
}

pub struct CommandItemComponentPatchScannerUtf16<'a> {
    command: Cow<'a, [u16]>,
    cursor: usize,
    visited: BTreeSet<i32>,
    finished: bool,
    pending_separator: bool,
    snbt_failure: Option<SnbtParseFailure>,
    suggestions: Option<(usize, CommandItemComponentSuggestions)>,
}

impl<'a> CommandItemComponentPatchScannerUtf16<'a> {
    /// `start` is the UTF-16 position of the opening `[`.
    pub fn new(command: &'a [u16], start: usize) -> Self {
        Self::from_units(Cow::Borrowed(command), start)
    }

    fn from_units(command: Cow<'a, [u16]>, start: usize) -> Self {
        let cursor = start + 1;
        Self {
            command,
            cursor,
            visited: BTreeSet::new(),
            finished: false,
            pending_separator: false,
            snbt_failure: None,
            suggestions: Some((
                cursor,
                CommandItemComponentSuggestions::Components { removed: false },
            )),
        }
    }

    /// UTF-16 position after `]`, valid once the scan has finished.
    pub fn end(&self) -> usize {
        self.cursor
    }

    /// `readComponent` (ItemParser.java:209-217) lets `parseAsArgument` throw
    /// through, so the refusal a value's grammar raised is the one the argument
    /// reports; only the codec result is rewrapped.
    pub fn take_snbt_failure(&mut self) -> Option<SnbtParseFailure> {
        self.snbt_failure.take()
    }

    pub fn next_component(
        &mut self,
    ) -> Result<Option<ScannedCommandItemComponentUtf16>, CommandSyntaxErrorUtf16> {
        if self.finished {
            return Ok(None);
        }
        if self.pending_separator {
            self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
            match self.command.get(self.cursor) {
                Some(44) => {
                    self.cursor += 1;
                    self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
                    self.suggestions = Some((
                        self.cursor,
                        CommandItemComponentSuggestions::Components { removed: false },
                    ));
                }
                Some(93) => {}
                _ => {
                    return Err(invalid_item_component_utf16(
                        self.cursor,
                        "expected item component separator or end",
                    ))
                }
            }
            self.pending_separator = false;
        }
        if self.command.get(self.cursor) == Some(&93) {
            self.cursor += 1;
            self.finished = true;
            self.suggestions = None;
            return Ok(None);
        }
        self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
        self.suggestions = Some((
            self.cursor,
            CommandItemComponentSuggestions::Components { removed: false },
        ));
        if self.cursor >= self.command.len() {
            return Err(invalid_item_component_utf16(
                self.cursor,
                "unclosed item component patch",
            ));
        }
        let removed = self.command.get(self.cursor) == Some(&33);
        if removed {
            self.cursor += 1;
            self.suggestions = Some((
                self.cursor,
                CommandItemComponentSuggestions::Components { removed: true },
            ));
        }
        let start = self.cursor;
        let (end, resource_id) = scan_command_resource_location_utf16(&self.command, self.cursor)?;
        self.cursor = end;
        let component = command_persistent_data_component(&resource_id).ok_or_else(|| {
            invalid_item_component_utf16(start, format!("unknown item component {resource_id}"))
        })?;
        if !self.visited.insert(component.type_id) {
            return Err(invalid_item_component_utf16(
                start,
                format!("duplicate item component {resource_id}"),
            ));
        }
        self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
        self.suggestions = if removed {
            None
        } else {
            Some((self.cursor, CommandItemComponentSuggestions::Assignment))
        };
        let mut value = None;
        let mut value_start = None;
        if !removed {
            if self.command.get(self.cursor) != Some(&61) {
                return Err(invalid_item_component_utf16(
                    self.cursor,
                    "expected item component assignment",
                ));
            }
            self.cursor += 1;
            self.suggestions = None;
            self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
            value_start = Some(self.cursor);
            let (consumed, parsed) = crate::parse_snbt_prefix_utf16(&self.command[self.cursor..])
                .map_err(|mut failure| {
                failure.cursor += self.cursor;
                let cursor = failure.cursor;
                self.snbt_failure = Some(failure);
                invalid_item_component_utf16(cursor, "invalid item component SNBT")
            })?;
            self.cursor += consumed;
            value = Some(parsed);
        }
        self.pending_separator = true;
        self.suggestions = Some((
            scan_command_whitespace_utf16(&self.command, self.cursor),
            CommandItemComponentSuggestions::Separator,
        ));
        Ok(Some(ScannedCommandItemComponentUtf16 {
            start,
            removed,
            component,
            value,
            value_start,
        }))
    }
}

pub struct CommandItemComponentPatchScanner<'a> {
    command: &'a str,
    raw: CommandItemComponentPatchScannerUtf16<'a>,
}

impl<'a> CommandItemComponentPatchScanner<'a> {
    pub fn new(command: &'a str, start: usize) -> Self {
        let units = command.encode_utf16().collect::<Vec<_>>();
        let start = command[..start].encode_utf16().count();
        Self {
            command,
            raw: CommandItemComponentPatchScannerUtf16::from_units(Cow::Owned(units), start),
        }
    }

    pub fn end(&self) -> usize {
        strings::byte_cursor(self.command, self.raw.end())
    }

    /// Byte offset and candidates retained by the scan, including a failed scan.
    pub fn suggestions(&self) -> Option<(usize, CommandItemComponentSuggestions)> {
        self.raw
            .suggestions
            .map(|(cursor, stage)| (strings::byte_cursor(self.command, cursor), stage))
    }

    /// [`CommandItemComponentPatchScannerUtf16::take_snbt_failure`] with the
    /// cursor in this buffer's bytes.
    pub fn take_snbt_failure(&mut self) -> Option<SnbtParseFailure> {
        self.raw.take_snbt_failure().map(|mut failure| {
            failure.cursor = strings::byte_cursor(self.command, failure.cursor);
            failure
        })
    }

    pub fn next_component(
        &mut self,
    ) -> Result<Option<ScannedCommandItemComponent>, CommandSyntaxError> {
        let to_byte = |cursor: usize| strings::byte_cursor(self.command, cursor);
        self.raw
            .next_component()
            .map(|entry| entry.map(|entry| entry.map_offsets(&to_byte)))
            .map_err(|error| strings::byte_error(self.command, 0, error))
    }
}

fn invalid_item_component_utf16(
    cursor: usize,
    message: impl Into<crate::JavaString>,
) -> CommandSyntaxErrorUtf16 {
    CommandSyntaxErrorUtf16 {
        kind: CommandSyntaxErrorKind::InvalidArgument,
        cursor,
        message: message.into(),
    }
}

/// What an item predicate matches items by, before any `[...]` tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScannedCommandItemPredicateHead {
    /// `*`: every item.
    Any { end: usize },
    /// `#tag`.
    Tag {
        start: usize,
        resource_id: String,
        end: usize,
    },
    /// A single item name.
    Item {
        start: usize,
        resource_id: String,
        end: usize,
    },
}

impl ScannedCommandItemPredicateHead {
    pub fn end(&self) -> usize {
        match self {
            Self::Any { end } | Self::Tag { end, .. } | Self::Item { end, .. } => *end,
        }
    }

    fn map_offsets(self, to_byte: &dyn Fn(usize) -> usize) -> Self {
        match self {
            Self::Any { end } => Self::Any { end: to_byte(end) },
            Self::Tag {
                start,
                resource_id,
                end,
            } => Self::Tag {
                start: to_byte(start),
                resource_id,
                end: to_byte(end),
            },
            Self::Item {
                start,
                resource_id,
                end,
            } => Self::Item {
                start: to_byte(start),
                resource_id,
                end: to_byte(end),
            },
        }
    }
}

pub type ScannedCommandItemPredicateHeadUtf16 = ScannedCommandItemPredicateHead;

/// Scans what an item predicate opens with.
pub fn scan_command_item_predicate_head_utf16(
    command: &[u16],
    start: usize,
) -> Result<ScannedCommandItemPredicateHeadUtf16, CommandSyntaxErrorUtf16> {
    let start = scan_command_whitespace_utf16(command, start);
    match command.get(start) {
        Some(42) => Ok(ScannedCommandItemPredicateHead::Any { end: start + 1 }),
        Some(35) => {
            let (end, resource_id) = predicate_identifier_utf16(command, start + 1)?;
            Ok(ScannedCommandItemPredicateHead::Tag {
                start,
                resource_id,
                end,
            })
        }
        _ => {
            let (end, resource_id) = predicate_identifier_utf16(command, start)?;
            Ok(ScannedCommandItemPredicateHead::Item {
                start,
                resource_id,
                end,
            })
        }
    }
}

pub fn scan_command_item_predicate_head(
    command: &str,
    start: usize,
) -> Result<ScannedCommandItemPredicateHead, CommandSyntaxError> {
    strings::scan_from_start_via_utf16(
        command,
        start,
        scan_command_item_predicate_head_utf16,
        ScannedCommandItemPredicateHead::map_offsets,
    )
}

fn predicate_identifier_utf16(
    command: &[u16],
    start: usize,
) -> Result<(usize, String), CommandSyntaxErrorUtf16> {
    let start = scan_command_whitespace_utf16(command, start);
    let (end, id) = scan_command_resource_location_utf16(command, start)?;
    if end == start {
        return Err(invalid_item_component_utf16(
            start,
            "expected item predicate identifier",
        ));
    }
    Ok((end, id))
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScannedCommandItemPredicateTest {
    /// `name=value`: the component (or, when `component` is `None`, the
    /// `minecraft:count` stack field) must equal the value.
    Exact {
        start: usize,
        resource_id: String,
        component: Option<&'static DataComponentTypeCatalogEntry>,
        value: NbtSummaryValue,
        value_end: usize,
    },
    /// `name~value`: the value is a sub-predicate rather than a component.
    Subpredicate {
        start: usize,
        resource_id: String,
        value: NbtSummaryValue,
        value_end: usize,
    },
    /// A bare `name`: the component only has to be present.
    Presence { start: usize, resource_id: String },
}

impl ScannedCommandItemPredicateTest {
    fn map_offsets(self, to_byte: &dyn Fn(usize) -> usize) -> Self {
        match self {
            Self::Exact {
                start,
                resource_id,
                component,
                value,
                value_end,
            } => Self::Exact {
                start: to_byte(start),
                resource_id,
                component,
                value,
                value_end: to_byte(value_end),
            },
            Self::Subpredicate {
                start,
                resource_id,
                value,
                value_end,
            } => Self::Subpredicate {
                start: to_byte(start),
                resource_id,
                value,
                value_end: to_byte(value_end),
            },
            Self::Presence { start, resource_id } => Self::Presence {
                start: to_byte(start),
                resource_id,
            },
        }
    }
}

pub type ScannedCommandItemPredicateTestUtf16 = ScannedCommandItemPredicateTest;

/// One `term` (`ComponentPredicateParser.java:86-90`): the test, whether `!`
/// negates it, and whether `|` joined it to the previous term instead of `,`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedCommandItemPredicateTerm {
    pub negated: bool,
    pub alternative: bool,
    pub test: ScannedCommandItemPredicateTest,
}

impl ScannedCommandItemPredicateTerm {
    fn map_offsets(self, to_byte: &dyn Fn(usize) -> usize) -> Self {
        Self {
            negated: self.negated,
            alternative: self.alternative,
            test: self.test.map_offsets(to_byte),
        }
    }
}

pub type ScannedCommandItemPredicateTermUtf16 = ScannedCommandItemPredicateTerm;

/// Candidates at the furthest position reached by an item predicate scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandItemPredicateSuggestions {
    Names {
        negated: bool,
        closing: bool,
    },
    Terminals {
        assignment: bool,
        subpredicate: bool,
        separator: bool,
    },
}

pub struct CommandItemPredicateTestScannerUtf16<'a> {
    command: Cow<'a, [u16]>,
    cursor: usize,
    finished: bool,
    pending_separator: bool,
    /// `test`'s first alternative stores `StringReaderTerms.character('=')`
    /// before the bare-`componentType` alternative succeeds (`:94-97`), so a
    /// bare name whose next unit is refused names that terminal, not a
    /// separator.
    after_presence: bool,
    suggestions: Option<(usize, CommandItemPredicateSuggestions)>,
    snbt_failure: Option<SnbtParseFailure>,
}

impl<'a> CommandItemPredicateTestScannerUtf16<'a> {
    /// `start` is the UTF-16 position after the head, before optional whitespace and `[`.
    pub fn new(command: &'a [u16], start: usize) -> Self {
        Self::from_units(Cow::Borrowed(command), start)
    }

    fn from_units(command: Cow<'a, [u16]>, start: usize) -> Self {
        let opening = scan_command_whitespace_utf16(&command, start);
        if command.get(opening) != Some(&91) {
            return Self::stopped(command, start);
        }
        let cursor = scan_command_whitespace_utf16(&command, opening + 1);
        if command.get(cursor) == Some(&93) {
            return Self::stopped(command, cursor + 1);
        }
        Self {
            command,
            cursor,
            finished: false,
            pending_separator: false,
            after_presence: false,
            suggestions: Some((
                cursor,
                CommandItemPredicateSuggestions::Names {
                    negated: false,
                    closing: true,
                },
            )),
            snbt_failure: None,
        }
    }

    fn stopped(command: Cow<'a, [u16]>, cursor: usize) -> Self {
        Self {
            command,
            cursor,
            finished: true,
            pending_separator: false,
            after_presence: false,
            suggestions: None,
            snbt_failure: None,
        }
    }

    /// The cursor past the closing `]`, valid once the scan has finished.
    pub fn end(&self) -> usize {
        self.cursor
    }

    pub fn suggestions(&self) -> Option<(usize, CommandItemPredicateSuggestions)> {
        self.suggestions
    }

    /// [`CommandItemComponentPatchScannerUtf16::take_snbt_failure`] for the
    /// `tag` rule (`ComponentPredicateParser.java:120`), which stores the SNBT
    /// grammar's own refusal (`TagParseRule.java:22-27`).
    pub fn take_snbt_failure(&mut self) -> Option<SnbtParseFailure> {
        self.snbt_failure.take()
    }

    pub fn next_term(
        &mut self,
    ) -> Result<Option<ScannedCommandItemPredicateTermUtf16>, CommandSyntaxErrorUtf16> {
        if self.finished {
            return Ok(None);
        }
        let mut alternative = false;
        if self.pending_separator {
            self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
            match self.command.get(self.cursor) {
                Some(124) => {
                    alternative = true;
                    self.cursor += 1;
                }
                Some(44) => self.cursor += 1,
                Some(93) => {
                    self.cursor += 1;
                    self.finished = true;
                    self.suggestions = None;
                    return Ok(None);
                }
                _ => {
                    return Err(invalid_item_component_utf16(
                        self.cursor,
                        if self.after_presence {
                            "expected item component assignment"
                        } else {
                            "expected item predicate separator or end"
                        },
                    ))
                }
            }
            self.pending_separator = false;
            self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
            self.suggestions = Some((
                self.cursor,
                CommandItemPredicateSuggestions::Names {
                    negated: false,
                    closing: false,
                },
            ));
        }
        self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
        let negated = self.command.get(self.cursor) == Some(&33);
        if negated {
            self.cursor += 1;
        }
        self.cursor = scan_command_whitespace_utf16(&self.command, self.cursor);
        if negated {
            self.suggestions = Some((
                self.cursor,
                CommandItemPredicateSuggestions::Names {
                    negated: true,
                    closing: false,
                },
            ));
        }
        let start = self.cursor;
        let (end, resource_id) = predicate_identifier_utf16(&self.command, self.cursor)?;
        self.cursor = scan_command_whitespace_utf16(&self.command, end);
        let component = command_item_predicate_component(&resource_id);
        let subpredicate = command_known_item_predicate(&resource_id);
        if component.is_some() || subpredicate {
            self.suggestions = Some((
                self.cursor,
                CommandItemPredicateSuggestions::Terminals {
                    assignment: component.is_some(),
                    subpredicate,
                    separator: component.is_some(),
                },
            ));
        }
        let mut presence = false;
        let test = match self.command.get(self.cursor) {
            Some(61) => {
                let component = command_item_predicate_component(&resource_id)
                    .ok_or_else(|| self.name_not_a_component(end, &resource_id))?;
                self.cursor += 1;
                let value = self.read_value("invalid item component predicate SNBT")?;
                ScannedCommandItemPredicateTestUtf16::Exact {
                    start,
                    resource_id,
                    component,
                    value,
                    value_end: self.cursor,
                }
            }
            Some(126) => {
                if !command_known_item_predicate(&resource_id) {
                    // `predicateType` stores its own refusal at the same mark as
                    // `componentType`, which `test` already tried
                    // (`ComponentPredicateParser.java:94-97`), so the component
                    // lookup is the entry `LongestOnly` keeps.
                    return Err(invalid_item_component_utf16(
                        end,
                        format!("unknown item component {resource_id}"),
                    ));
                }
                self.cursor += 1;
                let value = self.read_value("invalid item predicate SNBT")?;
                ScannedCommandItemPredicateTestUtf16::Subpredicate {
                    start,
                    resource_id,
                    value,
                    value_end: self.cursor,
                }
            }
            _ => {
                if command_item_predicate_component(&resource_id).is_none() {
                    return Err(self.name_not_a_component(end, &resource_id));
                }
                presence = true;
                ScannedCommandItemPredicateTestUtf16::Presence { start, resource_id }
            }
        };
        self.after_presence = presence;
        self.pending_separator = true;
        if !presence {
            self.suggestions = Some((
                scan_command_whitespace_utf16(&self.command, self.cursor),
                CommandItemPredicateSuggestions::Terminals {
                    assignment: false,
                    subpredicate: false,
                    separator: true,
                },
            ));
        }
        Ok(Some(ScannedCommandItemPredicateTermUtf16 {
            negated,
            alternative,
            test,
        }))
    }

    /// A name the component registry does not hold stops at the component
    /// lookup, which names the identifier's own end; a name only the
    /// component-existence predicate accepts
    /// (`ItemPredicateArgument.java:173-178`) reaches the `~` terminal of the
    /// second alternative, which marks past whitespace like every terminal
    /// (`StringReaderTerms.java:56-57`).
    fn name_not_a_component(&self, end: usize, resource_id: &str) -> CommandSyntaxErrorUtf16 {
        if command_known_item_predicate(resource_id) {
            invalid_item_component_utf16(self.cursor, "expected item sub-predicate assignment")
        } else {
            invalid_item_component_utf16(end, format!("unknown item component {resource_id}"))
        }
    }

    fn read_value(&mut self, message: &str) -> Result<NbtSummaryValue, CommandSyntaxErrorUtf16> {
        self.suggestions = None;
        let (consumed, value) = crate::parse_snbt_prefix_utf16(&self.command[self.cursor..])
            .map_err(|mut failure| {
                failure.cursor += self.cursor;
                let cursor = failure.cursor;
                self.snbt_failure = Some(failure);
                invalid_item_component_utf16(cursor, message)
            })?;
        self.cursor += consumed;
        Ok(value)
    }
}

pub struct CommandItemPredicateTestScanner<'a> {
    command: &'a str,
    raw: CommandItemPredicateTestScannerUtf16<'a>,
}

impl<'a> CommandItemPredicateTestScanner<'a> {
    pub fn new(command: &'a str, start: usize) -> Self {
        let units = command.encode_utf16().collect::<Vec<_>>();
        let start = command[..start].encode_utf16().count();
        Self {
            command,
            raw: CommandItemPredicateTestScannerUtf16::from_units(Cow::Owned(units), start),
        }
    }

    pub fn end(&self) -> usize {
        strings::byte_cursor(self.command, self.raw.end())
    }

    /// Byte offset and candidates retained by the scan, including a failed scan.
    pub fn suggestions(&self) -> Option<(usize, CommandItemPredicateSuggestions)> {
        self.raw
            .suggestions
            .map(|(cursor, candidates)| (strings::byte_cursor(self.command, cursor), candidates))
    }

    /// [`CommandItemPredicateTestScannerUtf16::take_snbt_failure`] with the
    /// cursor in this buffer's bytes.
    pub fn take_snbt_failure(&mut self) -> Option<SnbtParseFailure> {
        self.raw.take_snbt_failure().map(|mut failure| {
            failure.cursor = strings::byte_cursor(self.command, failure.cursor);
            failure
        })
    }

    pub fn next_term(
        &mut self,
    ) -> Result<Option<ScannedCommandItemPredicateTerm>, CommandSyntaxError> {
        let to_byte = |cursor: usize| strings::byte_cursor(self.command, cursor);
        self.raw
            .next_term()
            .map(|term| term.map(|term| term.map_offsets(&to_byte)))
            .map_err(|error| strings::byte_error(self.command, 0, error))
    }
}

#[cfg(test)]
#[test]
fn raw_item_component_scanner_preserves_deferred_separator_and_legacy_byte_positions() {
    let input: Vec<u16> = "[custom_data='"
        .encode_utf16()
        .chain([0xd800])
        .chain("' !damage]".encode_utf16())
        .collect();
    let mut raw = CommandItemComponentPatchScannerUtf16::new(&input, 0);
    let value = raw.next_component().unwrap().unwrap();
    assert_eq!(
        value.value,
        Some(NbtSummaryValue::string_from_utf16(vec![0xd800]))
    );
    let error = raw.next_component().unwrap_err();
    assert_eq!(error.cursor, "[custom_data='".len() + 3);
    let input = "😀[custom_data={value:'😀'}]";
    let mut legacy = CommandItemComponentPatchScanner::new(input, "😀".len());
    let component = legacy.next_component().unwrap().unwrap();
    assert_eq!(component.start, "😀[".len());
    assert_eq!(component.value_start, Some("😀[custom_data=".len()));
    assert!(legacy.next_component().unwrap().is_none());
    assert_eq!(legacy.end(), input.len());
}

#[cfg(test)]
#[test]
fn raw_item_predicate_lookup_error_uses_identifier_end_before_whitespace() {
    for suffix in [" ]", " = 1]", " ~ {}]"] {
        let input: Vec<u16> = [0xd800]
            .into_iter()
            .chain("[missing".encode_utf16())
            .chain(suffix.encode_utf16())
            .collect();
        let mut scanner = CommandItemPredicateTestScannerUtf16::new(&input, 1);
        let error = scanner.next_term().unwrap_err();
        assert_eq!(error.cursor, 9);
    }
}
