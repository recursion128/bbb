use std::fmt;

use crate::nbt_summary::{
    parse_snbt_compound_prefix_utf16, NbtSummaryEntry, NbtSummaryValue, SnbtParseFailure,
};
use crate::JavaString;

const MAX_COMMAND_NBT_MUTATION_DEPTH: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandNbtPathErrorKind {
    InvalidNode,
    NothingFound,
    ExpectedList,
    InvalidIndex,
    TooDeep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandNbtPathParseError {
    pub kind: CommandNbtPathErrorKind,
    pub node_index: Option<usize>,
    pub offset: usize,
    pub prefix: String,
    pub context: String,
    /// The refusal the shared SNBT reader raised inside a `{...}` filter.
    /// `NbtPathArgument` (NbtPathArgument.java:93, :110, :121) calls
    /// `TagParser.parseCompoundAsArgument` without catching it, so it reaches
    /// the player instead of `arguments.nbtpath.node.invalid`.
    pub snbt: Option<SnbtParseFailure>,
}

impl fmt::Display for CommandNbtPathParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "command NBT path {:?} at byte {} after {:?}: {}",
            self.kind, self.offset, self.prefix, self.context
        )
    }
}

impl std::error::Error for CommandNbtPathParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandNbtPathNodeKind {
    CompoundChild(JavaString),
    AllElements,
    IndexedElement(i32),
    MatchElement(NbtSummaryValue),
    MatchObject {
        name: JavaString,
        pattern: NbtSummaryValue,
    },
    MatchRootObject(NbtSummaryValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandNbtPathNode {
    pub kind: CommandNbtPathNodeKind,
    /// Where this node ends, in the argument's own units.
    pub end_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandNbtPath {
    source: JavaString,
    nodes: Vec<CommandNbtPathNode>,
}

impl CommandNbtPath {
    pub fn source(&self) -> &JavaString {
        &self.source
    }

    /// The argument's own units, which a name may need when it holds one no
    /// `String` can.
    pub fn source_units(&self) -> &[u16] {
        self.source.units()
    }

    pub fn nodes(&self) -> &[CommandNbtPathNode] {
        &self.nodes
    }

    #[allow(
        clippy::result_large_err,
        reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
    )]
    pub fn get(
        &self,
        root: &NbtSummaryValue,
    ) -> Result<Vec<NbtSummaryValue>, CommandNbtPathParseError> {
        let addresses = self.get_addresses(root).map_err(|err| *err)?;
        Ok(addresses
            .iter()
            .filter_map(|address| value_at(root, address))
            .collect())
    }

    pub fn count_matching(&self, root: &NbtSummaryValue) -> usize {
        self.walk_addresses(root).last().map_or(1, Vec::len)
    }

    #[allow(
        clippy::result_large_err,
        reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
    )]
    pub fn get_or_create(
        &self,
        root: &mut NbtSummaryValue,
        new_value: NbtSummaryValue,
    ) -> Result<Vec<NbtSummaryValue>, CommandNbtPathParseError> {
        let addresses = self
            .get_or_create_addresses(root, || new_value.clone())
            .map_err(|err| *err)?;
        Ok(addresses
            .iter()
            .filter_map(|address| value_at(root, address))
            .collect())
    }

    #[allow(
        clippy::result_large_err,
        reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
    )]
    pub fn modify_or_create(
        &self,
        root: &mut NbtSummaryValue,
        new_value: NbtSummaryValue,
        mut update: impl FnMut(&mut NbtSummaryValue),
    ) -> Result<(), CommandNbtPathParseError> {
        let addresses = self
            .get_or_create_addresses(root, || new_value.clone())
            .map_err(|err| *err)?;
        for address in addresses {
            if let Some(value) = value_mut_at(root, &address) {
                update(value);
            }
        }
        Ok(())
    }

    #[allow(
        clippy::result_large_err,
        reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
    )]
    pub fn set(
        &self,
        root: &mut NbtSummaryValue,
        new_value: NbtSummaryValue,
    ) -> Result<usize, CommandNbtPathParseError> {
        self.ensure_mutation_depth(&new_value).map_err(|err| *err)?;
        let (last, parents) = self.get_or_create_parents(root).map_err(|err| *err)?;
        let mut changed = 0;
        for parent in parents {
            if let Some(parent) = value_mut_at(root, &parent) {
                changed += set_node(parent, &last.kind, &new_value);
            }
        }
        Ok(changed)
    }

    #[allow(
        clippy::result_large_err,
        reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
    )]
    pub fn insert(
        &self,
        root: &mut NbtSummaryValue,
        index: i32,
        values: &[NbtSummaryValue],
    ) -> Result<usize, CommandNbtPathParseError> {
        for value in values {
            self.ensure_mutation_depth(value).map_err(|err| *err)?;
        }
        let targets = self
            .get_or_create_addresses(root, || NbtSummaryValue::List(Vec::new()))
            .map_err(|err| *err)?;
        let mut changed = 0;
        for (target_number, address) in targets.into_iter().enumerate() {
            let Some(target) = value_mut_at(root, &address) else {
                continue;
            };
            if collection_len(target).is_none() {
                return Err(*self.execution_error(
                    CommandNbtPathErrorKind::ExpectedList,
                    self.nodes.len().checked_sub(1),
                    format!("target {target_number} is not a collection"),
                ));
            }
            let size = collection_len(target).expect("checked collection");
            let mut actual = if index < 0 {
                i64::try_from(size).unwrap_or(i64::MAX) + i64::from(index) + 1
            } else {
                i64::from(index)
            };
            let mut modified = false;
            for value in values {
                match insert_collection_value(target, actual, value.clone()) {
                    InsertResult::Inserted => {
                        actual += 1;
                        modified = true;
                    }
                    InsertResult::Rejected => {}
                    InsertResult::InvalidIndex => {
                        return Err(*self.execution_error(
                            CommandNbtPathErrorKind::InvalidIndex,
                            self.nodes.len().checked_sub(1),
                            format!(
                                "insertion index {actual} is outside collection length {}",
                                collection_len(target).expect("collection remains a collection")
                            ),
                        ));
                    }
                }
            }
            changed += usize::from(modified);
        }
        Ok(changed)
    }

    /// `DataCommands.decorateModification`'s `merge` manipulator
    /// (`DataCommands.java:120-141`) over every value selected by this path.
    #[allow(
        clippy::result_large_err,
        reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
    )]
    pub fn merge_compound(
        &self,
        root: &mut NbtSummaryValue,
        source: &[NbtSummaryEntry],
    ) -> Result<usize, CommandNbtPathParseError> {
        let targets = self
            .get_or_create_addresses(root, || NbtSummaryValue::Compound(Vec::new()))
            .map_err(|err| *err)?;
        let mut changed = 0;
        for (target_number, address) in targets.into_iter().enumerate() {
            let Some(NbtSummaryValue::Compound(target)) = value_mut_at(root, &address) else {
                return Err(*self.execution_error(
                    CommandNbtPathErrorKind::InvalidNode,
                    self.nodes.len().checked_sub(1),
                    format!("target {target_number} is not a compound"),
                ));
            };
            changed += usize::from(merge_command_nbt_compound(target, source));
        }
        Ok(changed)
    }

    pub fn remove(&self, root: &mut NbtSummaryValue) -> usize {
        let Some((last, parent_nodes)) = self.nodes.split_last() else {
            return 0;
        };
        let mut current = vec![Vec::new()];
        for node in parent_nodes {
            current = apply_get_node(root, current, &node.kind);
        }
        let mut removed = 0;
        for address in current {
            if let Some(parent) = value_mut_at(root, &address) {
                removed += remove_node(parent, &last.kind);
            }
        }
        removed
    }

    fn get_addresses(
        &self,
        root: &NbtSummaryValue,
    ) -> Result<Vec<Address>, Box<CommandNbtPathParseError>> {
        let mut current = vec![Vec::new()];
        for (index, node) in self.nodes.iter().enumerate() {
            current = apply_get_node(root, current, &node.kind);
            if current.is_empty() {
                return Err(self.execution_error(
                    CommandNbtPathErrorKind::NothingFound,
                    Some(index),
                    "node matched no values".to_string(),
                ));
            }
        }
        Ok(current)
    }

    fn walk_addresses(&self, root: &NbtSummaryValue) -> Vec<Vec<Address>> {
        let mut stages = Vec::with_capacity(self.nodes.len());
        let mut current = vec![Vec::new()];
        for node in &self.nodes {
            current = apply_get_node(root, current, &node.kind);
            stages.push(current.clone());
            if current.is_empty() {
                break;
            }
        }
        stages
    }

    fn get_or_create_parents(
        &self,
        root: &mut NbtSummaryValue,
    ) -> Result<(&CommandNbtPathNode, Vec<Address>), Box<CommandNbtPathParseError>> {
        let Some((last, parents)) = self.nodes.split_last() else {
            return Err(self.execution_error(
                CommandNbtPathErrorKind::InvalidNode,
                None,
                "an empty path has no mutation target".to_string(),
            ));
        };
        let mut current = vec![Vec::new()];
        for (index, node) in parents.iter().enumerate() {
            let preferred = preferred_parent(&self.nodes[index + 1].kind);
            current = apply_get_or_create_node(root, current, &node.kind, || preferred.clone());
            if current.is_empty() {
                return Err(self.execution_error(
                    CommandNbtPathErrorKind::NothingFound,
                    Some(index),
                    "node matched no values".to_string(),
                ));
            }
        }
        Ok((last, current))
    }

    fn get_or_create_addresses<F>(
        &self,
        root: &mut NbtSummaryValue,
        mut new_value: F,
    ) -> Result<Vec<Address>, Box<CommandNbtPathParseError>>
    where
        F: FnMut() -> NbtSummaryValue,
    {
        let (last, parents) = self.get_or_create_parents(root)?;
        Ok(apply_get_or_create_node(
            root,
            parents,
            &last.kind,
            &mut new_value,
        ))
    }

    fn ensure_mutation_depth(
        &self,
        value: &NbtSummaryValue,
    ) -> Result<(), Box<CommandNbtPathParseError>> {
        if is_too_deep(value, self.nodes.len()) {
            Err(self.execution_error(
                CommandNbtPathErrorKind::TooDeep,
                self.nodes.len().checked_sub(1),
                format!(
                    "value and path exceed the {MAX_COMMAND_NBT_MUTATION_DEPTH}-level mutation budget"
                ),
            ))
        } else {
            Ok(())
        }
    }

    fn execution_error(
        &self,
        kind: CommandNbtPathErrorKind,
        node_index: Option<usize>,
        context: String,
    ) -> Box<CommandNbtPathParseError> {
        let units = node_index
            .and_then(|index| self.nodes.get(index))
            .map_or(0, |node| node.end_offset);
        let prefix = String::from_utf16_lossy(&self.source.units()[..units]);
        Box::new(CommandNbtPathParseError {
            kind,
            node_index,
            // The text consumers spell positions in bytes, like the argument
            // reader that produced this path.
            offset: prefix.len(),
            prefix,
            context,
            snbt: None,
        })
    }
}

/// `CompoundTag.merge` (`CompoundTag.java:435-445`).
pub fn merge_command_nbt_compound(
    target: &mut Vec<NbtSummaryEntry>,
    source: &[NbtSummaryEntry],
) -> bool {
    let before = target.clone();
    for source_entry in source {
        let existing = target
            .iter_mut()
            .find(|target_entry| target_entry.name == source_entry.name);
        match (existing, &source_entry.value) {
            (
                Some(NbtSummaryEntry {
                    value: NbtSummaryValue::Compound(target_value),
                    ..
                }),
                NbtSummaryValue::Compound(source_value),
            ) => {
                merge_command_nbt_compound(target_value, source_value);
            }
            (Some(target_entry), source_value) => target_entry.value = source_value.clone(),
            (None, _) => target.push(source_entry.clone()),
        }
    }
    *target != before
}

/// `NbtPathArgument.NbtPath.isTooDeep` (`NbtPathArgument.java:605-624`).
pub fn command_nbt_is_too_deep(value: &NbtSummaryValue, depth: usize) -> bool {
    is_too_deep(value, depth)
}

// NbtPathArgument.java:62-152 parses each node and records its ending source offset.
/// Every position is a byte index into `value`.
#[allow(
    clippy::result_large_err,
    reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
)]
pub fn parse_command_nbt_path_value_prefix(
    value: &str,
) -> Result<(usize, CommandNbtPath), CommandNbtPathParseError> {
    let units: Vec<u16> = value.encode_utf16().collect();
    match parse_command_nbt_path_units(&units) {
        Ok((consumed, path)) => Ok((super::syntax::byte_cursor(value, consumed), path)),
        Err(mut error) => {
            error.offset = super::syntax::byte_cursor(value, error.offset);
            Err(*error)
        }
    }
}

/// [`parse_command_nbt_path_value_prefix`] over a buffer's own units, with every
/// position in units and every name carried as the units it was written with.
#[allow(
    clippy::result_large_err,
    reason = "CommandNbtPathParseError public struct is constructed directly in bbb-server-core"
)]
pub fn parse_command_nbt_path_value_prefix_utf16(
    value: &[u16],
) -> Result<(usize, CommandNbtPath), CommandNbtPathParseError> {
    parse_command_nbt_path_units(value).map_err(|err| *err)
}

fn parse_command_nbt_path_units(
    value: &[u16],
) -> Result<(usize, CommandNbtPath), Box<CommandNbtPathParseError>> {
    Parser {
        input: value,
        cursor: 0,
        nodes: Vec::new(),
    }
    .parse()
}

struct Parser<'a> {
    input: &'a [u16],
    cursor: usize,
    nodes: Vec<CommandNbtPathNode>,
}

impl Parser<'_> {
    fn parse(mut self) -> Result<(usize, CommandNbtPath), Box<CommandNbtPathParseError>> {
        let mut first = true;
        while self.peek().is_some_and(|next| next != SPACE) {
            let kind = self.parse_node(first)?;
            self.nodes.push(CommandNbtPathNode {
                kind,
                end_offset: self.cursor,
            });
            first = false;
            if let Some(next) = self.peek() {
                if next != SPACE && next != OPEN_BRACKET && next != OPEN_BRACE && !self.consume(DOT)
                {
                    return Err(self.error("expected '.', '[', '{', or argument end"));
                }
            }
        }
        let consumed = self.cursor;
        let source = JavaString::from(self.input[..consumed].to_vec());
        Ok((
            consumed,
            CommandNbtPath {
                source,
                nodes: self.nodes,
            },
        ))
    }

    fn parse_node(
        &mut self,
        first: bool,
    ) -> Result<CommandNbtPathNodeKind, Box<CommandNbtPathParseError>> {
        match self.peek() {
            Some(SINGLE_QUOTE | DOUBLE_QUOTE) => {
                let name = self.read_quoted_name()?;
                self.parse_object_node(name)
            }
            Some(OPEN_BRACKET) => self.parse_element_node(),
            Some(OPEN_BRACE) if first => self
                .parse_compound_pattern()
                .map(CommandNbtPathNodeKind::MatchRootObject),
            Some(OPEN_BRACE) | None => Err(self.error("invalid NBT path node")),
            Some(_) => {
                let name = self.read_unquoted_name()?;
                self.parse_object_node(name)
            }
        }
    }

    fn parse_object_node(
        &mut self,
        name: JavaString,
    ) -> Result<CommandNbtPathNodeKind, Box<CommandNbtPathParseError>> {
        if self.peek() == Some(OPEN_BRACE) {
            Ok(CommandNbtPathNodeKind::MatchObject {
                name,
                pattern: self.parse_compound_pattern()?,
            })
        } else {
            Ok(CommandNbtPathNodeKind::CompoundChild(name))
        }
    }

    fn parse_element_node(
        &mut self,
    ) -> Result<CommandNbtPathNodeKind, Box<CommandNbtPathParseError>> {
        self.consume(OPEN_BRACKET);
        match self.peek() {
            Some(OPEN_BRACE) => {
                let pattern = self.parse_compound_pattern()?;
                if !self.consume(CLOSE_BRACKET) {
                    return Err(self.error("expected ']' after list pattern"));
                }
                Ok(CommandNbtPathNodeKind::MatchElement(pattern))
            }
            Some(CLOSE_BRACKET) => {
                self.consume(CLOSE_BRACKET);
                Ok(CommandNbtPathNodeKind::AllElements)
            }
            Some(_) => {
                let start = self.cursor;
                let (end, token) =
                    super::syntax::scan_command_number_lexeme_utf16(self.input, start)
                        .map_err(|_| self.error("expected a signed 32-bit list index"))?;
                let index = JavaString::with_utf8_units(token, |raw| raw.parse::<i32>())
                    .expect("list index lexeme is ASCII")
                    .map_err(|_| self.error("expected a signed 32-bit list index"))?;
                self.cursor = end;
                if !self.consume(CLOSE_BRACKET) {
                    return Err(self.error("expected ']' after list index"));
                }
                Ok(CommandNbtPathNodeKind::IndexedElement(index))
            }
            None => Err(self.error("unterminated list node")),
        }
    }

    fn parse_compound_pattern(&mut self) -> Result<NbtSummaryValue, Box<CommandNbtPathParseError>> {
        match parse_snbt_compound_prefix_utf16(&self.input[self.cursor..]) {
            Ok((consumed, pattern)) => {
                self.cursor += consumed;
                Ok(pattern)
            }
            Err(mut failure) => {
                // The filter reports its own cursor; rebase it onto the path so
                // the node error quotes the same offset.
                failure.cursor += self.cursor;
                self.cursor = failure.cursor;
                let mut error = self.error(failure.to_string());
                error.snbt = Some(failure);
                Err(error)
            }
        }
    }

    fn read_quoted_name(&mut self) -> Result<JavaString, Box<CommandNbtPathParseError>> {
        let result = super::syntax::read_command_string_value_utf16(self.input, self.cursor);
        let (end, name) = result.map_err(|error| {
            self.cursor = error.cursor;
            self.error(String::from_utf16_lossy(error.message.units()))
        })?;
        self.cursor = end;
        if name.units().is_empty() {
            return Err(self.error("NBT path names cannot be empty"));
        }
        Ok(name)
    }

    fn read_unquoted_name(&mut self) -> Result<JavaString, Box<CommandNbtPathParseError>> {
        let start = self.cursor;
        while self.peek().is_some_and(is_unquoted_name_character) {
            self.bump();
        }
        if self.cursor == start {
            Err(self.error("NBT path names cannot be empty"))
        } else {
            Ok(JavaString::from(self.input[start..self.cursor].to_vec()))
        }
    }

    fn error(&self, context: impl Into<String>) -> Box<CommandNbtPathParseError> {
        let prefix_end = self.cursor.min(self.input.len());
        Box::new(CommandNbtPathParseError {
            kind: CommandNbtPathErrorKind::InvalidNode,
            node_index: Some(self.nodes.len()),
            offset: self.cursor,
            prefix: String::from_utf16_lossy(&self.input[..prefix_end]),
            context: context.into(),
            snbt: None,
        })
    }

    fn consume(&mut self, expected: u16) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u16> {
        self.input.get(self.cursor).copied()
    }

    fn bump(&mut self) -> Option<u16> {
        let next = self.peek()?;
        self.cursor += 1;
        Some(next)
    }
}

const SPACE: u16 = b' ' as u16;
const DOT: u16 = b'.' as u16;
const SINGLE_QUOTE: u16 = b'\'' as u16;
const DOUBLE_QUOTE: u16 = b'"' as u16;
const OPEN_BRACKET: u16 = b'[' as u16;
const CLOSE_BRACKET: u16 = b']' as u16;
const OPEN_BRACE: u16 = b'{' as u16;
const CLOSE_BRACE: u16 = b'}' as u16;

fn is_unquoted_name_character(next: u16) -> bool {
    !matches!(
        next,
        SPACE
            | DOUBLE_QUOTE
            | SINGLE_QUOTE
            | OPEN_BRACKET
            | CLOSE_BRACKET
            | DOT
            | OPEN_BRACE
            | CLOSE_BRACE
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AddressStep {
    Compound(usize),
    Element(usize),
}

type Address = Vec<AddressStep>;

fn apply_get_node(
    root: &NbtSummaryValue,
    parents: Vec<Address>,
    node: &CommandNbtPathNodeKind,
) -> Vec<Address> {
    let mut output = Vec::new();
    for parent in parents {
        let Some(value) = value_ref_at(root, &parent) else {
            continue;
        };
        append_node_matches(value, &parent, node, &mut output);
    }
    output
}

fn append_node_matches(
    parent: &NbtSummaryValue,
    address: &Address,
    node: &CommandNbtPathNodeKind,
    output: &mut Vec<Address>,
) {
    match node {
        CommandNbtPathNodeKind::CompoundChild(name) => {
            if let Some(index) = compound_index(parent, name) {
                output.push(with_step(address, AddressStep::Compound(index)));
            }
        }
        CommandNbtPathNodeKind::AllElements => {
            if let Some(size) = collection_len(parent) {
                output
                    .extend((0..size).map(|index| with_step(address, AddressStep::Element(index))));
            }
        }
        CommandNbtPathNodeKind::IndexedElement(index) => {
            if let Some(actual) = resolve_index(*index, collection_len(parent)) {
                output.push(with_step(address, AddressStep::Element(actual)));
            }
        }
        CommandNbtPathNodeKind::MatchElement(pattern) => {
            if let NbtSummaryValue::List(values) = parent {
                output.extend(
                    values
                        .iter()
                        .enumerate()
                        .filter(|(_, value)| compare_nbt(pattern, value))
                        .map(|(index, _)| with_step(address, AddressStep::Element(index))),
                );
            }
        }
        CommandNbtPathNodeKind::MatchObject { name, pattern } => {
            if let NbtSummaryValue::Compound(entries) = parent {
                if let Some((index, entry)) = entries
                    .iter()
                    .enumerate()
                    .find(|(_, entry)| name_matches(&entry.name, name))
                {
                    if compare_nbt(pattern, &entry.value) {
                        output.push(with_step(address, AddressStep::Compound(index)));
                    }
                }
            }
        }
        CommandNbtPathNodeKind::MatchRootObject(pattern) => {
            if matches!(parent, NbtSummaryValue::Compound(_)) && compare_nbt(pattern, parent) {
                output.push(address.clone());
            }
        }
    }
}

fn apply_get_or_create_node<F>(
    root: &mut NbtSummaryValue,
    parents: Vec<Address>,
    node: &CommandNbtPathNodeKind,
    mut child: F,
) -> Vec<Address>
where
    F: FnMut() -> NbtSummaryValue,
{
    let mut output = Vec::new();
    for address in parents {
        let Some(parent) = value_mut_at(root, &address) else {
            continue;
        };
        match node {
            CommandNbtPathNodeKind::CompoundChild(name) => {
                if let NbtSummaryValue::Compound(entries) = parent {
                    // `compound.put(this.name, child.get())`
                    // (`NbtPathArgument.java:246-256`) keys a `Map<String, Tag>`,
                    // so the node's own units become the created key.
                    let index = entries
                        .iter()
                        .position(|entry| name_matches(&entry.name, name))
                        .unwrap_or_else(|| {
                            entries.push(NbtSummaryEntry {
                                name: name.clone(),
                                value: child(),
                            });
                            entries.len() - 1
                        });
                    output.push(with_step(&address, AddressStep::Compound(index)));
                }
            }
            CommandNbtPathNodeKind::AllElements => {
                if collection_len(parent) == Some(0) {
                    let _ = insert_collection_value(parent, 0, child());
                }
                if let Some(size) = collection_len(parent) {
                    output.extend(
                        (0..size).map(|index| with_step(&address, AddressStep::Element(index))),
                    );
                }
            }
            CommandNbtPathNodeKind::IndexedElement(index) => {
                if let Some(actual) = resolve_index(*index, collection_len(parent)) {
                    output.push(with_step(&address, AddressStep::Element(actual)));
                }
            }
            CommandNbtPathNodeKind::MatchElement(pattern) => {
                if let NbtSummaryValue::List(values) = parent {
                    let matches = values
                        .iter()
                        .enumerate()
                        .filter_map(|(index, value)| compare_nbt(pattern, value).then_some(index))
                        .collect::<Vec<_>>();
                    if matches.is_empty() {
                        values.push(pattern.clone());
                        output.push(with_step(&address, AddressStep::Element(values.len() - 1)));
                    } else {
                        output.extend(
                            matches
                                .into_iter()
                                .map(|index| with_step(&address, AddressStep::Element(index))),
                        );
                    }
                }
            }
            CommandNbtPathNodeKind::MatchObject { name, pattern } => {
                if let NbtSummaryValue::Compound(entries) = parent {
                    match entries
                        .iter()
                        .position(|entry| name_matches(&entry.name, name))
                    {
                        Some(index) if compare_nbt(pattern, &entries[index].value) => {
                            output.push(with_step(&address, AddressStep::Compound(index)));
                        }
                        Some(_) => {}
                        None => {
                            entries.push(NbtSummaryEntry {
                                name: name.clone(),
                                value: pattern.clone(),
                            });
                            output.push(with_step(
                                &address,
                                AddressStep::Compound(entries.len() - 1),
                            ));
                        }
                    }
                }
            }
            CommandNbtPathNodeKind::MatchRootObject(pattern) => {
                if matches!(parent, NbtSummaryValue::Compound(_)) && compare_nbt(pattern, parent) {
                    output.push(address);
                }
            }
        }
    }
    output
}

fn set_node(
    parent: &mut NbtSummaryValue,
    node: &CommandNbtPathNodeKind,
    new_value: &NbtSummaryValue,
) -> usize {
    match node {
        CommandNbtPathNodeKind::CompoundChild(name) => {
            let NbtSummaryValue::Compound(entries) = parent else {
                return 0;
            };
            match entries
                .iter_mut()
                .find(|entry| name_matches(&entry.name, name))
            {
                Some(entry) if entry.value == *new_value => 0,
                Some(entry) => {
                    entry.value = new_value.clone();
                    1
                }
                None => {
                    entries.push(NbtSummaryEntry {
                        name: name.clone(),
                        value: new_value.clone(),
                    });
                    1
                }
            }
        }
        CommandNbtPathNodeKind::AllElements => set_all_elements(parent, new_value),
        CommandNbtPathNodeKind::IndexedElement(index) => {
            let Some(actual) = resolve_index(*index, collection_len(parent)) else {
                return 0;
            };
            replace_collection_value(parent, actual, new_value)
        }
        CommandNbtPathNodeKind::MatchElement(pattern) => {
            let NbtSummaryValue::List(values) = parent else {
                return 0;
            };
            if values.is_empty() {
                values.push(new_value.clone());
                return 1;
            }
            let mut changed = 0;
            for value in values {
                if compare_nbt(pattern, value) && *value != *new_value {
                    *value = new_value.clone();
                    changed += 1;
                }
            }
            changed
        }
        CommandNbtPathNodeKind::MatchObject { name, pattern } => {
            let NbtSummaryValue::Compound(entries) = parent else {
                return 0;
            };
            let Some(entry) = entries
                .iter_mut()
                .find(|entry| name_matches(&entry.name, name))
            else {
                return 0;
            };
            if compare_nbt(pattern, &entry.value) && entry.value != *new_value {
                entry.value = new_value.clone();
                1
            } else {
                0
            }
        }
        CommandNbtPathNodeKind::MatchRootObject(_) => 0,
    }
}

fn remove_node(parent: &mut NbtSummaryValue, node: &CommandNbtPathNodeKind) -> usize {
    match node {
        CommandNbtPathNodeKind::CompoundChild(name) => remove_compound_child(parent, name, None),
        CommandNbtPathNodeKind::AllElements => clear_collection(parent),
        CommandNbtPathNodeKind::IndexedElement(index) => {
            let Some(actual) = resolve_index(*index, collection_len(parent)) else {
                return 0;
            };
            usize::from(remove_collection_value(parent, actual))
        }
        CommandNbtPathNodeKind::MatchElement(pattern) => {
            let NbtSummaryValue::List(values) = parent else {
                return 0;
            };
            let before = values.len();
            values.retain(|value| !compare_nbt(pattern, value));
            before - values.len()
        }
        CommandNbtPathNodeKind::MatchObject { name, pattern } => {
            remove_compound_child(parent, name, Some(pattern))
        }
        CommandNbtPathNodeKind::MatchRootObject(_) => 0,
    }
}

fn remove_compound_child(
    parent: &mut NbtSummaryValue,
    name: &JavaString,
    pattern: Option<&NbtSummaryValue>,
) -> usize {
    let NbtSummaryValue::Compound(entries) = parent else {
        return 0;
    };
    let Some(index) = entries.iter().position(|entry| {
        name_matches(&entry.name, name)
            && pattern.is_none_or(|pattern| compare_nbt(pattern, &entry.value))
    }) else {
        return 0;
    };
    entries.remove(index);
    1
}

// NbtPathArgument.java:185-210 clears before a rejected replacement but keeps its computed count.
fn set_all_elements(parent: &mut NbtSummaryValue, new_value: &NbtSummaryValue) -> usize {
    match parent {
        NbtSummaryValue::List(values) => {
            if values.is_empty() {
                values.push(new_value.clone());
                return 1;
            }
            let changed = values.iter().filter(|value| **value != *new_value).count();
            if changed != 0 {
                values.fill(new_value.clone());
            }
            changed
        }
        NbtSummaryValue::ByteArray(values) => {
            if values.is_empty() {
                if let Some(value) = numeric_as_i8(new_value) {
                    values.push(value);
                }
                return 1;
            }
            let Some(value) = numeric_as_i8(new_value) else {
                values.clear();
                return 0;
            };
            let changed = if matches!(new_value, NbtSummaryValue::Byte(new) if *new == value) {
                values.iter().filter(|current| **current != value).count()
            } else {
                values.len()
            };
            if changed != 0 {
                values.fill(value);
            }
            changed
        }
        NbtSummaryValue::IntArray(values) => {
            if values.is_empty() {
                if let Some(value) = numeric_as_i32(new_value) {
                    values.push(value);
                }
                return 1;
            }
            let Some(value) = numeric_as_i32(new_value) else {
                values.clear();
                return 0;
            };
            let changed = if matches!(new_value, NbtSummaryValue::Int(new) if *new == value) {
                values.iter().filter(|current| **current != value).count()
            } else {
                values.len()
            };
            if changed != 0 {
                values.fill(value);
            }
            changed
        }
        NbtSummaryValue::LongArray(values) => {
            if values.is_empty() {
                if let Some(value) = numeric_as_i64(new_value) {
                    values.push(value);
                }
                return 1;
            }
            let Some(value) = numeric_as_i64(new_value) else {
                values.clear();
                return 0;
            };
            let changed = if matches!(new_value, NbtSummaryValue::Long(new) if *new == value) {
                values.iter().filter(|current| **current != value).count()
            } else {
                values.len()
            };
            if changed != 0 {
                values.fill(value);
            }
            changed
        }
        _ => 0,
    }
}

fn collection_len(value: &NbtSummaryValue) -> Option<usize> {
    match value {
        NbtSummaryValue::List(values) => Some(values.len()),
        NbtSummaryValue::ByteArray(values) => Some(values.len()),
        NbtSummaryValue::IntArray(values) => Some(values.len()),
        NbtSummaryValue::LongArray(values) => Some(values.len()),
        _ => None,
    }
}

fn clear_collection(value: &mut NbtSummaryValue) -> usize {
    match value {
        NbtSummaryValue::List(values) => take_and_clear(values),
        NbtSummaryValue::ByteArray(values) => take_and_clear(values),
        NbtSummaryValue::IntArray(values) => take_and_clear(values),
        NbtSummaryValue::LongArray(values) => take_and_clear(values),
        _ => 0,
    }
}

fn take_and_clear<T>(values: &mut Vec<T>) -> usize {
    let size = values.len();
    values.clear();
    size
}

enum InsertResult {
    Inserted,
    Rejected,
    InvalidIndex,
}

fn insert_collection_value(
    collection: &mut NbtSummaryValue,
    index: i64,
    value: NbtSummaryValue,
) -> InsertResult {
    match collection {
        NbtSummaryValue::List(values) => {
            let Ok(index) = usize::try_from(index) else {
                return InsertResult::InvalidIndex;
            };
            if index > values.len() {
                return InsertResult::InvalidIndex;
            }
            values.insert(index, value);
            InsertResult::Inserted
        }
        NbtSummaryValue::ByteArray(values) => {
            insert_primitive(values, index, numeric_as_i8(&value))
        }
        NbtSummaryValue::IntArray(values) => {
            insert_primitive(values, index, numeric_as_i32(&value))
        }
        NbtSummaryValue::LongArray(values) => {
            insert_primitive(values, index, numeric_as_i64(&value))
        }
        _ => InsertResult::Rejected,
    }
}

fn insert_primitive<T>(values: &mut Vec<T>, index: i64, value: Option<T>) -> InsertResult {
    let Some(value) = value else {
        return InsertResult::Rejected;
    };
    let Ok(index) = usize::try_from(index) else {
        return InsertResult::InvalidIndex;
    };
    if index > values.len() {
        return InsertResult::InvalidIndex;
    }
    values.insert(index, value);
    InsertResult::Inserted
}

fn replace_collection_value(
    collection: &mut NbtSummaryValue,
    index: usize,
    value: &NbtSummaryValue,
) -> usize {
    match collection {
        NbtSummaryValue::List(values) => {
            let Some(current) = values.get_mut(index) else {
                return 0;
            };
            if *current == *value {
                0
            } else {
                *current = value.clone();
                1
            }
        }
        NbtSummaryValue::ByteArray(values) => {
            let equal =
                matches!(value, NbtSummaryValue::Byte(new) if values.get(index) == Some(new));
            replace_primitive(values, index, numeric_as_i8(value), equal)
        }
        NbtSummaryValue::IntArray(values) => {
            let equal =
                matches!(value, NbtSummaryValue::Int(new) if values.get(index) == Some(new));
            replace_primitive(values, index, numeric_as_i32(value), equal)
        }
        NbtSummaryValue::LongArray(values) => {
            let equal =
                matches!(value, NbtSummaryValue::Long(new) if values.get(index) == Some(new));
            replace_primitive(values, index, numeric_as_i64(value), equal)
        }
        _ => 0,
    }
}

fn replace_primitive<T: Copy + PartialEq>(
    values: &mut [T],
    index: usize,
    replacement: Option<T>,
    tag_equal: bool,
) -> usize {
    let (Some(current), Some(replacement)) = (values.get_mut(index), replacement) else {
        return 0;
    };
    if tag_equal {
        0
    } else {
        *current = replacement;
        1
    }
}

fn remove_collection_value(collection: &mut NbtSummaryValue, index: usize) -> bool {
    match collection {
        NbtSummaryValue::List(values) if index < values.len() => {
            values.remove(index);
            true
        }
        NbtSummaryValue::ByteArray(values) if index < values.len() => {
            values.remove(index);
            true
        }
        NbtSummaryValue::IntArray(values) if index < values.len() => {
            values.remove(index);
            true
        }
        NbtSummaryValue::LongArray(values) if index < values.len() => {
            values.remove(index);
            true
        }
        _ => false,
    }
}

fn resolve_index(index: i32, size: Option<usize>) -> Option<usize> {
    let size = i64::try_from(size?).ok()?;
    let actual = if index < 0 {
        size + i64::from(index)
    } else {
        i64::from(index)
    };
    (0..size)
        .contains(&actual)
        .then(|| usize::try_from(actual).expect("nonnegative collection index"))
}

fn compound_index(value: &NbtSummaryValue, name: &JavaString) -> Option<usize> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .position(|entry| name_matches(&entry.name, name))
}

fn name_matches(key: &JavaString, name: &JavaString) -> bool {
    key == name
}

fn preferred_parent(node: &CommandNbtPathNodeKind) -> NbtSummaryValue {
    match node {
        CommandNbtPathNodeKind::CompoundChild(_)
        | CommandNbtPathNodeKind::MatchObject { .. }
        | CommandNbtPathNodeKind::MatchRootObject(_) => NbtSummaryValue::Compound(Vec::new()),
        CommandNbtPathNodeKind::AllElements
        | CommandNbtPathNodeKind::IndexedElement(_)
        | CommandNbtPathNodeKind::MatchElement(_) => NbtSummaryValue::List(Vec::new()),
    }
}

fn with_step(address: &Address, step: AddressStep) -> Address {
    let mut child = address.clone();
    child.push(step);
    child
}

fn value_ref_at<'a>(
    value: &'a NbtSummaryValue,
    address: &[AddressStep],
) -> Option<&'a NbtSummaryValue> {
    let Some((step, remaining)) = address.split_first() else {
        return Some(value);
    };
    match (value, step) {
        (NbtSummaryValue::Compound(entries), AddressStep::Compound(index)) => {
            value_ref_at(&entries.get(*index)?.value, remaining)
        }
        (NbtSummaryValue::List(values), AddressStep::Element(index)) => {
            value_ref_at(values.get(*index)?, remaining)
        }
        _ => None,
    }
}

fn value_at(value: &NbtSummaryValue, address: &[AddressStep]) -> Option<NbtSummaryValue> {
    let Some((step, remaining)) = address.split_first() else {
        return Some(value.clone());
    };
    match (value, step) {
        (NbtSummaryValue::Compound(entries), AddressStep::Compound(index)) => {
            value_at(&entries.get(*index)?.value, remaining)
        }
        (NbtSummaryValue::List(values), AddressStep::Element(index)) => {
            value_at(values.get(*index)?, remaining)
        }
        (NbtSummaryValue::ByteArray(values), AddressStep::Element(index))
            if remaining.is_empty() =>
        {
            values.get(*index).copied().map(NbtSummaryValue::Byte)
        }
        (NbtSummaryValue::IntArray(values), AddressStep::Element(index))
            if remaining.is_empty() =>
        {
            values.get(*index).copied().map(NbtSummaryValue::Int)
        }
        (NbtSummaryValue::LongArray(values), AddressStep::Element(index))
            if remaining.is_empty() =>
        {
            values.get(*index).copied().map(NbtSummaryValue::Long)
        }
        _ => None,
    }
}

fn value_mut_at<'a>(
    value: &'a mut NbtSummaryValue,
    address: &[AddressStep],
) -> Option<&'a mut NbtSummaryValue> {
    let Some((step, remaining)) = address.split_first() else {
        return Some(value);
    };
    match (value, step) {
        (NbtSummaryValue::Compound(entries), AddressStep::Compound(index)) => {
            value_mut_at(&mut entries.get_mut(*index)?.value, remaining)
        }
        (NbtSummaryValue::List(values), AddressStep::Element(index)) => {
            value_mut_at(values.get_mut(*index)?, remaining)
        }
        _ => None,
    }
}

// NbtUtils.java:65-121 uses recursive compound subset and list member matching.
fn compare_nbt(expected: &NbtSummaryValue, actual: &NbtSummaryValue) -> bool {
    match (expected, actual) {
        (NbtSummaryValue::Compound(expected), NbtSummaryValue::Compound(actual)) => {
            actual.len() >= expected.len()
                && expected.iter().all(|expected_entry| {
                    actual
                        .iter()
                        .find(|actual_entry| actual_entry.name == expected_entry.name)
                        .is_some_and(|actual_entry| {
                            compare_nbt(&expected_entry.value, &actual_entry.value)
                        })
                })
        }
        (NbtSummaryValue::List(expected), NbtSummaryValue::List(actual)) => {
            if expected.is_empty() {
                actual.is_empty()
            } else {
                actual.len() >= expected.len()
                    && expected.iter().all(|expected_value| {
                        actual
                            .iter()
                            .any(|actual_value| compare_nbt(expected_value, actual_value))
                    })
            }
        }
        _ => expected == actual,
    }
}

// NbtPathArgument.java:605-663 budgets set/insert payload depth from the path depth.
fn is_too_deep(value: &NbtSummaryValue, initial_depth: usize) -> bool {
    let mut pending = vec![(value, initial_depth)];
    while let Some((value, depth)) = pending.pop() {
        if depth >= MAX_COMMAND_NBT_MUTATION_DEPTH {
            return true;
        }
        match value {
            NbtSummaryValue::Compound(entries) => pending.extend(
                entries
                    .iter()
                    .map(|entry| (&entry.value, depth.saturating_add(1))),
            ),
            NbtSummaryValue::List(values) => {
                pending.extend(values.iter().map(|value| (value, depth.saturating_add(1))))
            }
            _ => {}
        }
    }
    false
}

// FloatTag.java:80-96 and DoubleTag.java:80-96 define their distinct floor and narrowing order.
fn numeric_as_i8(value: &NbtSummaryValue) -> Option<i8> {
    match value {
        NbtSummaryValue::Byte(value) => Some(*value),
        NbtSummaryValue::Short(value) => Some(*value as i8),
        NbtSummaryValue::Int(value) => Some(*value as i8),
        NbtSummaryValue::Long(value) => Some(*value as i8),
        NbtSummaryValue::Float(value) => Some(f32::from_bits(*value).floor() as i32 as i8),
        NbtSummaryValue::Double(value) => Some(f64::from_bits(*value).floor() as i32 as i8),
        _ => None,
    }
}

fn numeric_as_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Long(value) => Some(*value as i32),
        NbtSummaryValue::Float(value) => Some(f32::from_bits(*value).floor() as i32),
        NbtSummaryValue::Double(value) => Some(f64::from_bits(*value).floor() as i32),
        _ => None,
    }
}

fn numeric_as_i64(value: &NbtSummaryValue) -> Option<i64> {
    match value {
        NbtSummaryValue::Long(value) => Some(*value),
        NbtSummaryValue::Byte(value) => Some(i64::from(*value)),
        NbtSummaryValue::Short(value) => Some(i64::from(*value)),
        NbtSummaryValue::Int(value) => Some(i64::from(*value)),
        NbtSummaryValue::Float(value) => Some(f32::from_bits(*value) as i64),
        NbtSummaryValue::Double(value) => Some(f64::from_bits(*value).floor() as i64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn parse(input: &str) -> CommandNbtPath {
        let (consumed, path) = parse_command_nbt_path_value_prefix(input).expect("valid path");
        assert_eq!(path.source(), &input[..consumed]);
        path
    }

    #[test]
    fn nbt_path_parses_every_node_and_preserves_prefix_offsets() {
        let source = "root.\"quoted.key\"[].items[-2][{kind:'wanted'}].value tail";
        let (consumed, path) = parse_command_nbt_path_value_prefix(source).expect("valid path");
        assert_eq!(
            &source[..consumed],
            "root.\"quoted.key\"[].items[-2][{kind:'wanted'}].value"
        );
        assert_eq!(path.source(), &source[..consumed]);
        assert_eq!(path.nodes().len(), 7);
        assert!(path
            .nodes()
            .windows(2)
            .all(|nodes| nodes[0].end_offset < nodes[1].end_offset));
        assert!(matches!(
            &path.nodes()[1].kind,
            CommandNbtPathNodeKind::CompoundChild(name) if name == "quoted.key"
        ));
        assert!(matches!(
            path.nodes()[2].kind,
            CommandNbtPathNodeKind::AllElements
        ));
        assert!(matches!(
            path.nodes()[4].kind,
            CommandNbtPathNodeKind::IndexedElement(-2)
        ));
        assert!(matches!(
            path.nodes()[5].kind,
            CommandNbtPathNodeKind::MatchElement(_)
        ));

        let root = parse("{enabled:true}");
        assert!(matches!(
            root.nodes()[0].kind,
            CommandNbtPathNodeKind::MatchRootObject(_)
        ));
        let matched = parse("child{enabled:true}");
        assert!(matches!(
            matched.nodes()[0].kind,
            CommandNbtPathNodeKind::MatchObject { .. }
        ));
    }

    #[test]
    fn nbt_path_get_count_and_negative_index_match_values() {
        let root = compound(vec![entry(
            "values",
            NbtSummaryValue::List(vec![
                compound(vec![entry("id", NbtSummaryValue::String("first".into()))]),
                compound(vec![entry("id", NbtSummaryValue::String("last".into()))]),
            ]),
        )]);
        let path = parse("values[-1].id");
        assert_eq!(
            path.get(&root).unwrap(),
            vec![NbtSummaryValue::String("last".into())]
        );
        assert_eq!(path.count_matching(&root), 1);
        assert_eq!(parse("values[9]").count_matching(&root), 0);
        let error = parse("values[9]").get(&root).unwrap_err();
        assert_eq!(error.kind, CommandNbtPathErrorKind::NothingFound);
        assert_eq!(error.prefix, "values[9]");
    }

    #[test]
    fn nbt_path_patterns_recursively_use_partial_compounds_and_list_members() {
        let matched_value = compound(vec![
            entry(
                "nested",
                compound(vec![entry("flag", NbtSummaryValue::Byte(1))]),
            ),
            entry(
                "members",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(2), NbtSummaryValue::Int(1)]),
            ),
            entry("extra", NbtSummaryValue::String("kept".into())),
        ]);
        let root = compound(vec![
            entry("values", NbtSummaryValue::List(vec![matched_value.clone()])),
            entry("holder", matched_value),
        ]);
        let path = parse("values[{nested:{flag:1b},members:[1,2]}]");
        assert_eq!(path.count_matching(&root), 1);
        assert_eq!(path.get(&root).unwrap().len(), 1);
        assert_eq!(
            parse("values[{members:[]}]").count_matching(&root),
            0,
            "an empty pattern list only matches an empty candidate list"
        );

        let root_path = parse("{values:[{nested:{flag:1b}}]}");
        assert_eq!(root_path.count_matching(&root), 1);
        let object_path = parse("holder{nested:{flag:1b}}");
        assert_eq!(object_path.count_matching(&root), 1);
    }

    #[test]
    fn nbt_path_get_or_create_builds_preferred_parents_and_patterns() {
        let mut root = compound(Vec::new());
        let created = parse("outer.items[].name")
            .get_or_create(&mut root, NbtSummaryValue::String("new".into()))
            .unwrap();
        assert_eq!(created, vec![NbtSummaryValue::String("new".into())]);
        assert_eq!(
            parse("outer.items[].name").get(&root).unwrap(),
            vec![NbtSummaryValue::String("new".into())]
        );

        let mut pattern_root = compound(vec![entry("items", NbtSummaryValue::List(Vec::new()))]);
        let created = parse("items[{kind:'new'}]")
            .get_or_create(&mut pattern_root, compound(Vec::new()))
            .unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(
            parse("items[{kind:'new'}]").count_matching(&pattern_root),
            1
        );
    }

    #[test]
    fn nbt_path_set_creates_parents_and_reports_matched_mutations() {
        let mut root = compound(Vec::new());
        let path = parse("outer.value");
        assert_eq!(path.set(&mut root, NbtSummaryValue::Int(3)).unwrap(), 1);
        assert_eq!(path.set(&mut root, NbtSummaryValue::Int(3)).unwrap(), 0);

        let mut root = compound(vec![entry(
            "items",
            NbtSummaryValue::List(vec![
                compound(vec![entry("kind", NbtSummaryValue::String("x".into()))]),
                compound(vec![entry("kind", NbtSummaryValue::String("y".into()))]),
            ]),
        )]);
        let path = parse("items[{kind:'x'}]");
        assert_eq!(path.count_matching(&root), 1);
        assert_eq!(path.set(&mut root, compound(Vec::new())).unwrap(), 1);
        assert_eq!(path.count_matching(&root), 0);
    }

    #[test]
    fn nbt_path_insert_handles_negative_positions_and_errors() {
        let mut root = compound(vec![entry(
            "values",
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(1), NbtSummaryValue::Int(3)]),
        )]);
        let path = parse("values");
        assert_eq!(
            path.insert(&mut root, -1, &[NbtSummaryValue::Int(2)])
                .unwrap(),
            1
        );
        assert_eq!(
            path.get(&root).unwrap(),
            vec![NbtSummaryValue::List(vec![
                NbtSummaryValue::Int(1),
                NbtSummaryValue::Int(3),
                NbtSummaryValue::Int(2),
            ])]
        );
        let error = path
            .insert(&mut root, 9, &[NbtSummaryValue::Int(4)])
            .unwrap_err();
        assert_eq!(error.kind, CommandNbtPathErrorKind::InvalidIndex);

        let mut scalar = compound(vec![entry("value", NbtSummaryValue::Int(1))]);
        let error = parse("value")
            .insert(&mut scalar, 0, &[NbtSummaryValue::Int(2)])
            .unwrap_err();
        assert_eq!(error.kind, CommandNbtPathErrorKind::ExpectedList);
    }

    #[test]
    fn nbt_path_remove_counts_matches_before_mutating() {
        let mut root = compound(vec![entry(
            "items",
            NbtSummaryValue::List(vec![
                compound(vec![entry("remove", NbtSummaryValue::Byte(1))]),
                compound(vec![entry("keep", NbtSummaryValue::Byte(1))]),
                compound(vec![entry("remove", NbtSummaryValue::Byte(1))]),
            ]),
        )]);
        let path = parse("items[{remove:1b}]");
        let before = path.count_matching(&root);
        assert_eq!(before, 2);
        assert_eq!(path.remove(&mut root), before);
        assert_eq!(path.count_matching(&root), 0);
    }

    #[test]
    fn nbt_path_all_elements_and_arrays_follow_collection_mutations() {
        let mut root = compound(vec![
            entry(
                "values",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Int(1),
                    NbtSummaryValue::String("x".into()),
                ]),
            ),
            entry("array", NbtSummaryValue::IntArray(vec![1, 2, 3])),
        ]);
        assert_eq!(
            parse("values")
                .insert(&mut root, -1, &[NbtSummaryValue::Long(3)])
                .unwrap(),
            1
        );
        let all = parse("values[]");
        assert_eq!(all.count_matching(&root), 3);
        assert_eq!(all.set(&mut root, NbtSummaryValue::Byte(4)).unwrap(), 3);
        assert_eq!(all.remove(&mut root), 3);

        let indexed = parse("array[-1]");
        assert_eq!(indexed.get(&root).unwrap(), vec![NbtSummaryValue::Int(3)]);
        assert_eq!(indexed.set(&mut root, NbtSummaryValue::Long(7)).unwrap(), 1);
        assert_eq!(indexed.get(&root).unwrap(), vec![NbtSummaryValue::Int(7)]);

        assert_eq!(
            parse("array")
                .insert(&mut root, 1, &[NbtSummaryValue::Double(9.8_f64.to_bits())])
                .unwrap(),
            1
        );
        assert_eq!(
            parse("array").get(&root).unwrap(),
            vec![NbtSummaryValue::IntArray(vec![1, 9, 2, 7])]
        );
    }

    #[test]
    fn nbt_path_all_elements_clears_rejected_nonempty_primitive_arrays() {
        for mut array in [
            NbtSummaryValue::ByteArray(vec![1]),
            NbtSummaryValue::IntArray(vec![1]),
            NbtSummaryValue::LongArray(vec![1]),
        ] {
            let all = parse("[]");
            assert_eq!(all.count_matching(&array), 1);
            assert_eq!(
                all.set(&mut array, NbtSummaryValue::String("rejected".into()))
                    .unwrap(),
                0
            );
            assert_eq!(collection_len(&array), Some(0));
        }

        for mut array in [
            NbtSummaryValue::ByteArray(Vec::new()),
            NbtSummaryValue::IntArray(Vec::new()),
            NbtSummaryValue::LongArray(Vec::new()),
        ] {
            assert_eq!(
                parse("[]")
                    .set(&mut array, NbtSummaryValue::String("rejected".into()))
                    .unwrap(),
                1
            );
            assert_eq!(collection_len(&array), Some(0));
        }
    }

    #[test]
    fn nbt_path_numeric_array_conversions_match_java_numeric_tags() {
        for (value, byte, int, long) in [
            (NbtSummaryValue::Byte(-7), -7, -7, -7),
            (NbtSummaryValue::Short(0x123), 0x23, 0x123, 0x123),
            (
                NbtSummaryValue::Int(0x1234_5680),
                -128,
                0x1234_5680,
                0x1234_5680,
            ),
            (NbtSummaryValue::Long(0x1_0000_0001), 1, 1, 0x1_0000_0001),
            (NbtSummaryValue::Float((-1.2_f32).to_bits()), -2, -2, -1),
            (NbtSummaryValue::Double((-1.2_f64).to_bits()), -2, -2, -2),
        ] {
            assert_eq!(numeric_as_i8(&value), Some(byte));
            assert_eq!(numeric_as_i32(&value), Some(int));
            assert_eq!(numeric_as_i64(&value), Some(long));
        }

        for value in [
            NbtSummaryValue::Float(f32::NAN.to_bits()),
            NbtSummaryValue::Double(f64::NAN.to_bits()),
        ] {
            assert_eq!(numeric_as_i8(&value), Some(0));
            assert_eq!(numeric_as_i32(&value), Some(0));
            assert_eq!(numeric_as_i64(&value), Some(0));
        }
        for value in [
            NbtSummaryValue::Float(f32::INFINITY.to_bits()),
            NbtSummaryValue::Double(f64::INFINITY.to_bits()),
        ] {
            assert_eq!(numeric_as_i8(&value), Some(-1));
            assert_eq!(numeric_as_i32(&value), Some(i32::MAX));
            assert_eq!(numeric_as_i64(&value), Some(i64::MAX));
        }
        for value in [
            NbtSummaryValue::Float(f32::NEG_INFINITY.to_bits()),
            NbtSummaryValue::Double(f64::NEG_INFINITY.to_bits()),
        ] {
            assert_eq!(numeric_as_i8(&value), Some(0));
            assert_eq!(numeric_as_i32(&value), Some(i32::MIN));
            assert_eq!(numeric_as_i64(&value), Some(i64::MIN));
        }

        let above_int = NbtSummaryValue::Double(2_147_483_648.25_f64.to_bits());
        assert_eq!(numeric_as_i8(&above_int), Some(-1));
        assert_eq!(numeric_as_i32(&above_int), Some(i32::MAX));
        assert_eq!(numeric_as_i64(&above_int), Some(2_147_483_648));

        let all = parse("[]");
        let mut bytes = NbtSummaryValue::ByteArray(vec![0]);
        assert_eq!(
            all.set(&mut bytes, NbtSummaryValue::Float((-1.2_f32).to_bits()))
                .unwrap(),
            1
        );
        assert_eq!(bytes, NbtSummaryValue::ByteArray(vec![-2]));

        let mut ints = NbtSummaryValue::IntArray(vec![0]);
        assert_eq!(
            all.set(&mut ints, NbtSummaryValue::Float((-1.2_f32).to_bits()))
                .unwrap(),
            1
        );
        assert_eq!(ints, NbtSummaryValue::IntArray(vec![-2]));

        let mut longs = NbtSummaryValue::LongArray(vec![0]);
        assert_eq!(
            all.set(&mut longs, NbtSummaryValue::Double((-1.2_f64).to_bits()))
                .unwrap(),
            1
        );
        assert_eq!(longs, NbtSummaryValue::LongArray(vec![-2]));
    }

    #[test]
    fn nbt_path_merge_updates_each_selected_compound_independently() {
        let mut root = compound(vec![entry(
            "values",
            NbtSummaryValue::List(vec![
                compound(vec![
                    entry("kept", NbtSummaryValue::Int(1)),
                    entry(
                        "nested",
                        compound(vec![entry("left", NbtSummaryValue::Int(2))]),
                    ),
                ]),
                compound(vec![entry("kept", NbtSummaryValue::Int(3))]),
            ]),
        )]);
        let source = vec![
            entry(
                "nested",
                compound(vec![entry("right", NbtSummaryValue::Int(4))]),
            ),
            entry("added", NbtSummaryValue::Byte(1)),
        ];

        assert_eq!(parse("values[]").merge_compound(&mut root, &source), Ok(2));
        assert_eq!(
            parse("values[]").get(&root).unwrap(),
            vec![
                compound(vec![
                    entry("kept", NbtSummaryValue::Int(1)),
                    entry(
                        "nested",
                        compound(vec![
                            entry("left", NbtSummaryValue::Int(2)),
                            entry("right", NbtSummaryValue::Int(4)),
                        ]),
                    ),
                    entry("added", NbtSummaryValue::Byte(1)),
                ]),
                compound(vec![
                    entry("kept", NbtSummaryValue::Int(3)),
                    entry(
                        "nested",
                        compound(vec![entry("right", NbtSummaryValue::Int(4))]),
                    ),
                    entry("added", NbtSummaryValue::Byte(1)),
                ]),
            ]
        );
        assert_eq!(parse("values[]").merge_compound(&mut root, &source), Ok(0));
    }

    #[test]
    fn command_nbt_depth_and_root_compound_merge_are_shared_with_data_commands() {
        let mut target = vec![entry(
            "nested",
            compound(vec![entry("old", NbtSummaryValue::Int(1))]),
        )];
        assert!(merge_command_nbt_compound(
            &mut target,
            &[entry(
                "nested",
                compound(vec![entry("new", NbtSummaryValue::Int(2))]),
            )]
        ));
        assert!(!merge_command_nbt_compound(
            &mut target,
            &[entry(
                "nested",
                compound(vec![entry("new", NbtSummaryValue::Int(2))]),
            )]
        ));

        let mut boundary = NbtSummaryValue::Int(0);
        for _ in 0..511 {
            boundary = NbtSummaryValue::List(vec![boundary]);
        }
        assert!(!command_nbt_is_too_deep(&boundary, 0));
        assert!(command_nbt_is_too_deep(&boundary, 1));
    }

    #[test]
    fn nbt_path_mutation_depth_budget_has_a_512_boundary() {
        let mut root = compound(Vec::new());
        let path = parse("value");
        let mut allowed = NbtSummaryValue::Int(1);
        for _ in 0..510 {
            allowed = NbtSummaryValue::List(vec![allowed]);
        }
        assert_eq!(path.set(&mut root, allowed).unwrap(), 1);

        let mut rejected = NbtSummaryValue::Int(1);
        for _ in 0..511 {
            rejected = NbtSummaryValue::List(vec![rejected]);
        }
        let error = path.set(&mut root, rejected).unwrap_err();
        assert_eq!(error.kind, CommandNbtPathErrorKind::TooDeep);
    }

    #[test]
    fn nbt_path_parse_errors_expose_prefix_and_context() {
        for invalid in [".name", "items[", "items[2147483648]", "name{bad=1}"] {
            let error = parse_command_nbt_path_value_prefix(invalid).unwrap_err();
            assert_eq!(error.kind, CommandNbtPathErrorKind::InvalidNode);
            assert!(!error.context.is_empty());
            assert!(invalid.starts_with(&error.prefix));
        }
    }
}
