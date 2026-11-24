#[cfg(feature = "server-command-tree")]
use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::{normalize_resource_location, read_resource_location};
use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    ids,
};

#[cfg(feature = "server-command-tree")]
mod command_tree_26_1;
mod generated_26_1;

#[cfg(feature = "server-command-tree")]
pub use command_tree_26_1::VANILLA_COMMAND_TREE;
pub use generated_26_1::*;

const MAX_COMMAND_NODES: usize = 65_536;
const MAX_COMMAND_CHILDREN: usize = 65_536;

const MASK_TYPE: u8 = 0x03;
const FLAG_EXECUTABLE: u8 = 0x04;
const FLAG_REDIRECT: u8 = 0x08;
const FLAG_CUSTOM_SUGGESTIONS: u8 = 0x10;
const FLAG_RESTRICTED: u8 = 0x20;
const KNOWN_FLAGS: u8 =
    MASK_TYPE | FLAG_EXECUTABLE | FLAG_REDIRECT | FLAG_CUSTOM_SUGGESTIONS | FLAG_RESTRICTED;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commands {
    pub nodes: Vec<CommandNode>,
    pub root_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandNode {
    pub node_type: CommandNodeType,
    pub flags: u8,
    pub children: Vec<i32>,
    pub redirect: Option<i32>,
    pub name: Option<String>,
    pub parser: Option<CommandArgumentParser>,
    pub suggestions: Option<String>,
    pub executable: bool,
    pub restricted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandNodeType {
    Root,
    Literal,
    Argument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandArgumentParser {
    pub type_id: i32,
    pub name: String,
    pub properties: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandArgumentTemplate {
    Unit,
    Float { min: Option<f32>, max: Option<f32> },
    Double { min: Option<f64>, max: Option<f64> },
    Integer { min: Option<i32>, max: Option<i32> },
    Long { min: Option<i64>, max: Option<i64> },
    String { mode: CommandStringMode },
    Entity { single: bool, players_only: bool },
    ScoreHolder { multiple: bool },
    Time { minimum: i32 },
    Registry { registry: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStringMode {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

/// One node of the command tree a Vanilla 26.1 dedicated server registers.
///
/// The table itself is [`VANILLA_COMMAND_TREE`], generated from the runtime
/// probe; this type only names its columns. It is deliberately separate from
/// [`CommandNode`], which is the wire form: the wire form carries the flags and
/// the child indices a server chose to send one particular client, while this
/// one records what the official server built before any per-client filtering.
#[cfg(feature = "server-command-tree")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCommandNode {
    /// `CommandNode.getName()`; empty for the root.
    pub name: &'static str,
    /// The Brigadier node class, which is also the wire node type.
    pub kind: CommandNodeType,
    /// Index of the node this hangs off. `None` only for the root.
    pub parent: Option<u16>,
    /// First index of this node's contiguous run of children.
    pub first_child: u16,
    /// Length of that run; zero when the node is a leaf.
    pub child_count: u16,
    /// `CommandNode.getRedirect()` as an index into the same table.
    pub redirect: Option<u16>,
    /// `PermissionLevel.id()` of the lowest level that satisfies `requires`.
    pub permission: u8,
    /// `CommandNode.getCommand() != null`.
    pub executable: bool,
    /// `minecraft:command_argument_type` id of an argument node's parser.
    pub parser: Option<&'static str>,
    /// `SuggestionProviders.getName` of a custom suggestion provider.
    pub suggestions: Option<&'static str>,
    /// `ArgumentTypeInfo.serializeToJson` output, `None` when it is empty.
    pub properties: Option<&'static str>,
}

#[cfg(feature = "server-command-tree")]
impl VanillaCommandNode {
    /// The children of this node as indices into [`VANILLA_COMMAND_TREE`].
    pub fn children(&self) -> Range<usize> {
        let first = usize::from(self.first_child);
        first..first + usize::from(self.child_count)
    }

    /// The numeric parser id the wire form carries.
    ///
    /// The table stores the registry id as a string so that the numbering keeps
    /// a single owner: `ArgumentTypeInfos.bootstrap` registers the types in
    /// order, so a type's wire id is its position in
    /// [`VANILLA_COMMAND_ARGUMENT_TYPES`] and is resolved from there rather
    /// than duplicated here.
    pub fn parser_type_id(&self) -> Option<i32> {
        let parser = self.parser?;
        let index = VANILLA_COMMAND_ARGUMENT_TYPES
            .iter()
            .position(|name| *name == parser)?;
        i32::try_from(index).ok()
    }

    /// `NodeInspector.isRestricted` (Commands.java:182-185): a source holding
    /// no permission at all fails the requirement, which is exactly the case
    /// where some level above `PermissionLevel.ALL` is needed.
    pub fn restricted(&self) -> bool {
        self.permission > 0
    }
}

/// Index of the `RootCommandNode` in [`VANILLA_COMMAND_TREE`].
#[cfg(feature = "server-command-tree")]
pub const VANILLA_COMMAND_TREE_ROOT: u16 = 0;

/// The node at `index`, or `None` when the index is outside the table.
#[cfg(feature = "server-command-tree")]
pub fn vanilla_command_node(index: u16) -> Option<&'static VanillaCommandNode> {
    VANILLA_COMMAND_TREE.get(usize::from(index))
}

/// Every command name a dedicated server registers, in the table's order.
#[cfg(feature = "server-command-tree")]
pub fn vanilla_command_root_literals() -> impl Iterator<Item = &'static str> {
    VANILLA_COMMAND_TREE[VANILLA_COMMAND_TREE[usize::from(VANILLA_COMMAND_TREE_ROOT)].children()]
        .iter()
        .map(|node| node.name)
}

/// The child of `parent` named `name`.
///
/// Brigadier keys a node's children by name — `CommandNode.addChild` merges
/// into the existing child instead of adding a second one with the same name —
/// so at most one child can match and a scan over the parent's run is enough.
#[cfg(feature = "server-command-tree")]
pub fn vanilla_command_child(parent: u16, name: &str) -> Option<u16> {
    let node = vanilla_command_node(parent)?;
    node.children()
        .find(|index| VANILLA_COMMAND_TREE[*index].name == name)
        .and_then(|index| u16::try_from(index).ok())
}

/// Walk `path` down from the root, one literal or argument name per step.
#[cfg(feature = "server-command-tree")]
pub fn vanilla_command_lookup(path: &[&str]) -> Option<u16> {
    let mut index = VANILLA_COMMAND_TREE_ROOT;
    for name in path {
        index = vanilla_command_child(index, name)?;
    }
    Some(index)
}

/// The `PermissionLevel.id()` the node at `path` requires.
#[cfg(feature = "server-command-tree")]
pub fn vanilla_command_permission(path: &[&str]) -> Option<u8> {
    vanilla_command_node(vanilla_command_lookup(path)?).map(|node| node.permission)
}

impl CommandNode {
    /// `RootCommandNode`: `createEntry` (ClientboundCommandsPacket.java:159-161)
    /// gives it type 0, no stub, and no other flag.
    pub fn root() -> Self {
        Self::new(CommandNodeType::Root, None, None)
    }

    /// `LiteralCommandNode` (ClientboundCommandsPacket.java:169-172): type 1
    /// over the literal it matches.
    pub fn literal(name: impl Into<String>) -> Self {
        Self::new(CommandNodeType::Literal, Some(name.into()), None)
    }

    /// `ArgumentCommandNode` (ClientboundCommandsPacket.java:169-176): type 2
    /// over the argument name and its unpacked type template.
    pub fn argument(name: impl Into<String>, parser: CommandArgumentParser) -> Self {
        Self::new(CommandNodeType::Argument, Some(name.into()), Some(parser))
    }

    /// `NodeInspector.isExecutable` (Commands.java:177-179): the node carries a
    /// command, which `createEntry` (:150-152) reports as `FLAG_EXECUTABLE`.
    pub fn executes(mut self) -> Self {
        self.executable = true;
        self.rebuild_flags()
    }

    /// `NodeInspector.isRestricted` (Commands.java:182-185): the node's
    /// requirement rejects a source that holds no permission at all, which
    /// `createEntry` (:154-156) reports as `FLAG_RESTRICTED`.
    pub fn restrict(mut self) -> Self {
        self.restricted = true;
        self.rebuild_flags()
    }

    /// The child indices `createEntry` (:181) reads off the node, in the order
    /// `CommandNode.getChildren` hands them out — insertion order, because
    /// Brigadier keeps its children in a `LinkedHashMap` (CommandNode.java:23).
    pub fn with_children(mut self, children: Vec<i32>) -> Self {
        self.children = children;
        self
    }

    fn new(
        node_type: CommandNodeType,
        name: Option<String>,
        parser: Option<CommandArgumentParser>,
    ) -> Self {
        Self {
            node_type,
            flags: 0,
            children: Vec::new(),
            redirect: None,
            name,
            parser,
            suggestions: None,
            executable: false,
            restricted: false,
        }
        .rebuild_flags()
    }

    /// The flag byte is a projection of the fields around it rather than a
    /// field a caller sets, so every constructor recomputes it and the encoder
    /// refuses a node whose stored byte disagrees.
    fn rebuild_flags(mut self) -> Self {
        self.flags = command_node_flag_byte(&self);
        self
    }
}

impl CommandArgumentParser {
    /// Replays the packet framing decoder over the retained exact property
    /// bytes. This is the only semantic template decoder.
    pub fn template(&self) -> Result<CommandArgumentTemplate> {
        command_argument_template(self.type_id, &self.properties)
    }

    /// The inverse of [`Self::template`]: the property bytes
    /// `ArgumentTypeInfo.serializeToNetwork` writes for this template
    /// (`ArgumentNodeStub.serializeCap`, ClientboundCommandsPacket.java:206-213),
    /// with the parser name taken from the generated catalog rather than from
    /// the caller.
    pub fn from_template(type_id: i32, template: &CommandArgumentTemplate) -> Result<Self> {
        let name = command_argument_parser_name(type_id).ok_or_else(|| {
            ProtocolError::InvalidData(format!("unknown command argument parser id {type_id}"))
        })?;
        let mut properties = Encoder::new();
        encode_command_argument_template(&mut properties, type_id, template)?;
        Ok(Self {
            type_id,
            name: name.to_string(),
            properties: properties.into_inner(),
        })
    }
}

/// Replays the packet framing decoder over retained command-template bytes
/// without requiring callers that own an immutable projection to copy them.
pub fn command_argument_template(
    type_id: i32,
    properties: &[u8],
) -> Result<CommandArgumentTemplate> {
    let mut decoder = Decoder::new(properties);
    let template = decode_command_argument_template(&mut decoder, type_id)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing bytes after command argument template type {type_id}"
        )));
    }
    Ok(template)
}

/// `ClientboundCommandsPacket.write` (ClientboundCommandsPacket.java:60-63):
/// the node table, then the index of the root inside it.
///
/// Everything the decoder refuses is refused here too — the node and child
/// bounds, the flag byte, the tree topology, and the argument property bytes —
/// so the only packet this server can produce is one it could also read back.
pub fn encode_play_clientbound_commands(commands: &Commands) -> Result<(i32, Vec<u8>)> {
    if commands.nodes.len() > MAX_COMMAND_NODES {
        return Err(ProtocolError::PacketTooLarge(
            commands.nodes.len(),
            MAX_COMMAND_NODES,
        ));
    }
    validate_command_tree(&commands.nodes, commands.root_index)?;

    let mut out = Encoder::new();
    out.write_var_i32(commands.nodes.len() as i32);
    for node in &commands.nodes {
        encode_command_node(&mut out, node)?;
    }
    out.write_var_i32(commands.root_index);
    Ok((ids::play::CLIENTBOUND_COMMANDS, out.into_inner()))
}

/// `Entry.write` (ClientboundCommandsPacket.java:225-236): the flag byte, the
/// child index array, the redirect the flag announced, and the type's stub.
fn encode_command_node(out: &mut Encoder, node: &CommandNode) -> Result<()> {
    let flags = command_node_flags(node)?;
    if node.children.len() > MAX_COMMAND_CHILDREN {
        return Err(ProtocolError::PacketTooLarge(
            node.children.len(),
            MAX_COMMAND_CHILDREN,
        ));
    }

    out.write_u8(flags);
    out.write_var_i32(node.children.len() as i32);
    for child in &node.children {
        out.write_var_i32(*child);
    }
    if let Some(redirect) = node.redirect {
        out.write_var_i32(redirect);
    }

    match node.node_type {
        // `read` (:127-136) reads no stub for a root node.
        CommandNodeType::Root => {}
        CommandNodeType::Literal => {
            out.write_string(command_node_name(node)?);
        }
        CommandNodeType::Argument => {
            let parser = node.parser.as_ref().ok_or_else(|| {
                ProtocolError::InvalidData("argument command node carries no parser".to_string())
            })?;
            let expected_name = command_argument_parser_name(parser.type_id).ok_or_else(|| {
                ProtocolError::InvalidData(format!(
                    "unknown command argument parser id {}",
                    parser.type_id
                ))
            })?;
            if parser.name != expected_name {
                return Err(ProtocolError::InvalidData(format!(
                    "command argument parser id {} is `{expected_name}`, not `{}`",
                    parser.type_id, parser.name
                )));
            }
            // The retained bytes are replayed through the framing decoder
            // before they go on the wire: a property payload the decoder could
            // not read back — or one with bytes left over — is not a payload
            // `ArgumentTypeInfo.serializeToNetwork` could have written.
            command_argument_template(parser.type_id, &parser.properties)?;

            out.write_string(command_node_name(node)?);
            out.write_var_i32(parser.type_id);
            out.write_bytes(&parser.properties);
            if let Some(suggestions) = &node.suggestions {
                // `writeIdentifier` (:200-202) writes the namespaced form the
                // decoder normalizes an unqualified path into.
                out.write_string(&normalize_resource_location(suggestions.clone())?);
            }
        }
    }

    Ok(())
}

fn command_node_name(node: &CommandNode) -> Result<&str> {
    node.name
        .as_deref()
        .ok_or_else(|| ProtocolError::InvalidData("named command node carries no name".to_string()))
}

/// The flag byte `createEntry` (ClientboundCommandsPacket.java:139-180) builds,
/// checked against the one the node carries.
///
/// Vanilla derives every bit from the node it is looking at, so a node whose
/// stored byte says something its fields do not is a node this server never
/// built; refusing it here keeps `flags` a projection rather than a second
/// source of truth the wire could disagree with.
fn command_node_flags(node: &CommandNode) -> Result<u8> {
    match node.node_type {
        CommandNodeType::Root => {
            if node.name.is_some() || node.parser.is_some() || node.suggestions.is_some() {
                return Err(ProtocolError::InvalidData(
                    "root command node carries no name, parser, or suggestions".to_string(),
                ));
            }
            if node.executable || node.restricted || node.redirect.is_some() {
                return Err(ProtocolError::InvalidData(
                    "root command node must not carry executable, redirect, or restricted flags"
                        .to_string(),
                ));
            }
        }
        CommandNodeType::Literal => {
            if node.name.is_none() {
                return Err(ProtocolError::InvalidData(
                    "literal command node carries no name".to_string(),
                ));
            }
            if node.parser.is_some() {
                return Err(ProtocolError::InvalidData(
                    "literal command node carries no parser".to_string(),
                ));
            }
            if node.suggestions.is_some() {
                return Err(ProtocolError::InvalidData(
                    "custom command suggestions flag is only valid on argument nodes".to_string(),
                ));
            }
        }
        CommandNodeType::Argument => {
            if node.name.is_none() || node.parser.is_none() {
                return Err(ProtocolError::InvalidData(
                    "argument command node carries both a name and a parser".to_string(),
                ));
            }
        }
    }

    let flags = command_node_flag_byte(node);
    if flags != node.flags {
        return Err(ProtocolError::InvalidData(format!(
            "command node flags {:#04x} disagree with the node's own contents ({flags:#04x})",
            node.flags
        )));
    }
    Ok(flags)
}

fn command_node_flag_byte(node: &CommandNode) -> u8 {
    let mut flags = match node.node_type {
        CommandNodeType::Root => 0,
        CommandNodeType::Literal => 1,
        CommandNodeType::Argument => 2,
    };
    if node.executable {
        flags |= FLAG_EXECUTABLE;
    }
    if node.redirect.is_some() {
        flags |= FLAG_REDIRECT;
    }
    if node.suggestions.is_some() {
        flags |= FLAG_CUSTOM_SUGGESTIONS;
    }
    if node.restricted {
        flags |= FLAG_RESTRICTED;
    }
    flags
}

/// The inverse of [`decode_command_argument_template`], arm for arm: a template
/// only encodes under the parser id whose decoder produces it.
fn encode_command_argument_template(
    out: &mut Encoder,
    type_id: i32,
    template: &CommandArgumentTemplate,
) -> Result<()> {
    match (type_id, template) {
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
            CommandArgumentTemplate::Float { min, max },
        ) => encode_number_properties(out, *min, *max, Encoder::write_f32),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
            CommandArgumentTemplate::Double { min, max },
        ) => encode_number_properties(out, *min, *max, Encoder::write_f64),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
            CommandArgumentTemplate::Integer { min, max },
        ) => encode_number_properties(out, *min, *max, Encoder::write_i32),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID,
            CommandArgumentTemplate::Long { min, max },
        ) => encode_number_properties(out, *min, *max, Encoder::write_i64),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
            CommandArgumentTemplate::String { mode },
        ) => {
            out.write_var_i32(match mode {
                CommandStringMode::SingleWord => 0,
                CommandStringMode::QuotablePhrase => 1,
                CommandStringMode::GreedyPhrase => 2,
            });
        }
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
            CommandArgumentTemplate::Entity {
                single,
                players_only,
            },
        ) => out.write_u8(u8::from(*single) | u8::from(*players_only) << 1),
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID,
            CommandArgumentTemplate::ScoreHolder { multiple },
        ) => out.write_u8(u8::from(*multiple)),
        (VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID, CommandArgumentTemplate::Time { minimum }) => {
            out.write_i32(*minimum)
        }
        (
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID
                ..=VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID,
            CommandArgumentTemplate::Registry { registry },
        ) => out.write_string(&normalize_resource_location(registry.clone())?),
        // Every remaining catalog entry is a `SingletonArgumentInfo`, whose
        // template carries nothing — which is exactly what the decoder reports
        // when it reads a parser id out of an empty property payload.
        (_, CommandArgumentTemplate::Unit) if is_unit_command_argument_type(type_id) => {}
        _ => {
            return Err(ProtocolError::InvalidData(format!(
                "command argument parser id {type_id} does not carry this template"
            )))
        }
    }
    Ok(())
}

fn is_unit_command_argument_type(type_id: i32) -> bool {
    matches!(
        command_argument_template(type_id, &[]),
        Ok(CommandArgumentTemplate::Unit)
    )
}

fn encode_number_properties<T>(
    out: &mut Encoder,
    min: Option<T>,
    max: Option<T>,
    encode_value: impl Fn(&mut Encoder, T),
) {
    let mut flags = 0u8;
    if min.is_some() {
        flags |= 0x01;
    }
    if max.is_some() {
        flags |= 0x02;
    }
    out.write_u8(flags);
    if let Some(min) = min {
        encode_value(out, min);
    }
    if let Some(max) = max {
        encode_value(out, max);
    }
}

pub(super) fn decode_commands(decoder: &mut Decoder<'_>) -> Result<Commands> {
    let count = decoder.read_len()?;
    if count > MAX_COMMAND_NODES {
        return Err(ProtocolError::PacketTooLarge(count, MAX_COMMAND_NODES));
    }

    let mut nodes = Vec::with_capacity(decoder.bounded_capacity(count, 2));
    for _ in 0..count {
        nodes.push(decode_command_node(decoder)?);
    }

    let root_index = decoder.read_var_i32()?;
    validate_command_tree(&nodes, root_index)?;

    let packet = Commands { nodes, root_index };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after commands packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_command_node(decoder: &mut Decoder<'_>) -> Result<CommandNode> {
    let flags = decoder.read_u8()?;
    if flags & !KNOWN_FLAGS != 0 {
        return Err(ProtocolError::InvalidData(format!(
            "unknown command node flags {flags:#04x}"
        )));
    }

    let child_count = decoder.read_len()?;
    if child_count > MAX_COMMAND_CHILDREN {
        return Err(ProtocolError::PacketTooLarge(
            child_count,
            MAX_COMMAND_CHILDREN,
        ));
    }

    let mut children = Vec::with_capacity(decoder.bounded_capacity(child_count, 1));
    for _ in 0..child_count {
        children.push(decoder.read_var_i32()?);
    }

    let redirect = if flags & FLAG_REDIRECT != 0 {
        Some(decoder.read_var_i32()?)
    } else {
        None
    };

    let node_type = match flags & MASK_TYPE {
        0 => CommandNodeType::Root,
        1 => CommandNodeType::Literal,
        2 => CommandNodeType::Argument,
        _ => {
            return Err(ProtocolError::InvalidData(
                "invalid command node type".to_string(),
            ))
        }
    };

    if flags & FLAG_CUSTOM_SUGGESTIONS != 0 && node_type != CommandNodeType::Argument {
        return Err(ProtocolError::InvalidData(
            "custom command suggestions flag is only valid on argument nodes".to_string(),
        ));
    }
    if node_type == CommandNodeType::Root
        && flags & (FLAG_EXECUTABLE | FLAG_REDIRECT | FLAG_RESTRICTED) != 0
    {
        return Err(ProtocolError::InvalidData(
            "root command node must not carry executable, redirect, or restricted flags"
                .to_string(),
        ));
    }

    let (name, parser, suggestions) = match node_type {
        CommandNodeType::Root => (None, None, None),
        CommandNodeType::Literal => (Some(decoder.read_string(32767)?), None, None),
        CommandNodeType::Argument => {
            let name = decoder.read_string(32767)?;
            let type_id = decoder.read_var_i32()?;
            let parser_name = command_argument_parser_name(type_id).ok_or_else(|| {
                ProtocolError::InvalidData(format!("unknown command argument parser id {type_id}"))
            })?;
            let properties_start = decoder.position();
            decode_command_argument_template(decoder, type_id)?;
            let properties = decoder.bytes_from(properties_start).to_vec();
            let suggestions = if flags & FLAG_CUSTOM_SUGGESTIONS != 0 {
                Some(read_resource_location(decoder)?)
            } else {
                None
            };

            (
                Some(name),
                Some(CommandArgumentParser {
                    type_id,
                    name: parser_name.to_string(),
                    properties,
                }),
                suggestions,
            )
        }
    };

    Ok(CommandNode {
        node_type,
        flags,
        children,
        redirect,
        name,
        parser,
        suggestions,
        executable: flags & FLAG_EXECUTABLE != 0,
        restricted: flags & FLAG_RESTRICTED != 0,
    })
}

fn command_argument_parser_name(type_id: i32) -> Option<&'static str> {
    let index = usize::try_from(type_id).ok()?;
    VANILLA_COMMAND_ARGUMENT_TYPES.get(index).copied()
}

fn decode_command_argument_template(
    decoder: &mut Decoder<'_>,
    type_id: i32,
) -> Result<CommandArgumentTemplate> {
    match type_id {
        1 => {
            let (min, max) = decode_number_properties(decoder, |decoder| decoder.read_f32())?;
            Ok(CommandArgumentTemplate::Float { min, max })
        }
        2 => {
            let (min, max) = decode_number_properties(decoder, |decoder| decoder.read_f64())?;
            Ok(CommandArgumentTemplate::Double { min, max })
        }
        3 => {
            let (min, max) = decode_number_properties(decoder, |decoder| decoder.read_i32())?;
            Ok(CommandArgumentTemplate::Integer { min, max })
        }
        4 => {
            let (min, max) = decode_number_properties(decoder, |decoder| decoder.read_i64())?;
            Ok(CommandArgumentTemplate::Long { min, max })
        }
        5 => {
            let string_type = decoder.read_var_i32()?;
            let mode = match string_type {
                0 => CommandStringMode::SingleWord,
                1 => CommandStringMode::QuotablePhrase,
                2 => CommandStringMode::GreedyPhrase,
                _ => {
                    return Err(ProtocolError::InvalidData(format!(
                        "unknown brigadier string argument type {string_type}"
                    )))
                }
            };
            Ok(CommandArgumentTemplate::String { mode })
        }
        6 => {
            let flags = decoder.read_u8()?;
            Ok(CommandArgumentTemplate::Entity {
                single: flags & 0x01 != 0,
                players_only: flags & 0x02 != 0,
            })
        }
        31 => {
            let flags = decoder.read_u8()?;
            Ok(CommandArgumentTemplate::ScoreHolder {
                multiple: flags & 0x01 != 0,
            })
        }
        43 => Ok(CommandArgumentTemplate::Time {
            minimum: decoder.read_i32()?,
        }),
        44..=48 => Ok(CommandArgumentTemplate::Registry {
            registry: read_resource_location(decoder)?,
        }),
        id if command_argument_parser_name(id).is_some() => Ok(CommandArgumentTemplate::Unit),
        _ => Err(ProtocolError::InvalidData(format!(
            "unknown command argument parser id {type_id}"
        ))),
    }
}

fn decode_number_properties<T>(
    decoder: &mut Decoder<'_>,
    mut decode_value: impl FnMut(&mut Decoder<'_>) -> Result<T>,
) -> Result<(Option<T>, Option<T>)> {
    let flags = decoder.read_u8()?;
    if flags & !0x03 != 0 {
        return Err(ProtocolError::InvalidData(format!(
            "unknown number argument flags {flags:#04x}"
        )));
    }

    let min = (flags & 0x01 != 0)
        .then(|| decode_value(decoder))
        .transpose()?;
    let max = (flags & 0x02 != 0)
        .then(|| decode_value(decoder))
        .transpose()?;
    Ok((min, max))
}

fn validate_command_tree(nodes: &[CommandNode], root_index: i32) -> Result<()> {
    let root = validate_node_index(root_index, nodes.len(), "command root index")?;
    if nodes[root].node_type != CommandNodeType::Root {
        return Err(ProtocolError::InvalidData(
            "command root index must reference a root node".to_string(),
        ));
    }

    for (index, node) in nodes.iter().enumerate() {
        for child in &node.children {
            validate_node_index(*child, nodes.len(), "command child index").map_err(|err| {
                ProtocolError::InvalidData(format!("node {index} has invalid child: {err}"))
            })?;
        }
        if let Some(redirect) = node.redirect {
            validate_node_index(redirect, nodes.len(), "command redirect index").map_err(
                |err| {
                    ProtocolError::InvalidData(format!("node {index} has invalid redirect: {err}"))
                },
            )?;
        }
    }

    validate_redirect_acyclic(nodes)?;
    validate_children_acyclic(nodes)
}

fn validate_node_index(value: i32, len: usize, what: &'static str) -> Result<usize> {
    if value < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "{what} is negative: {value}"
        )));
    }
    let index = value as usize;
    if index >= len {
        return Err(ProtocolError::InvalidData(format!(
            "{what} {value} is outside node table of {len}"
        )));
    }
    Ok(index)
}

fn validate_redirect_acyclic(nodes: &[CommandNode]) -> Result<()> {
    // 0 = unvisited, 1 = visiting, 2 = resolved acyclic.
    let mut state = vec![0u8; nodes.len()];
    let mut path = Vec::new();
    for start in 0..nodes.len() {
        if state[start] != 0 {
            continue;
        }
        path.clear();
        let mut current = start;
        loop {
            match state[current] {
                0 => {
                    state[current] = 1;
                    path.push(current);
                    if let Some(redirect) = nodes[current].redirect {
                        current = redirect as usize;
                    } else {
                        break;
                    }
                }
                1 => {
                    return Err(ProtocolError::InvalidData(
                        "server sent an impossible command tree while resolving redirect"
                            .to_string(),
                    ));
                }
                _ => break,
            }
        }
        for visited in &path {
            state[*visited] = 2;
        }
    }
    Ok(())
}

fn validate_children_acyclic(nodes: &[CommandNode]) -> Result<()> {
    // Iterative 3-color DFS over child edges in O(V + E).
    let mut state = vec![0u8; nodes.len()];
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for start in 0..nodes.len() {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        stack.push((start, 0));
        while let Some((node_idx, child_pos)) = stack.last_mut() {
            let children = &nodes[*node_idx].children;
            if *child_pos >= children.len() {
                state[*node_idx] = 2;
                stack.pop();
                continue;
            }
            let next = children[*child_pos] as usize;
            *child_pos += 1;
            match state[next] {
                0 => {
                    state[next] = 1;
                    stack.push((next, 0));
                }
                1 => {
                    return Err(ProtocolError::InvalidData(
                        "server sent an impossible command tree while resolving children"
                            .to_string(),
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::{Decoder, Encoder},
        ids,
        packets::{decode_play_clientbound, PlayClientbound},
    };

    #[test]
    fn decodes_commands_packet_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(3);

        payload.write_u8(0);
        payload.write_var_i32(1);
        payload.write_var_i32(1);

        payload.write_u8(1);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_string("say");

        payload.write_u8(FLAG_EXECUTABLE | FLAG_CUSTOM_SUGGESTIONS | FLAG_RESTRICTED | 2);
        payload.write_var_i32(0);
        payload.write_string("message");
        payload.write_var_i32(5);
        payload.write_var_i32(2);
        payload.write_string("ask_server");

        payload.write_var_i32(0);
        let mut payload = payload.into_inner();

        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_COMMANDS, &payload).unwrap();
        let PlayClientbound::Commands(commands) = packet else {
            panic!("expected commands packet");
        };

        assert_eq!(commands.root_index, 0);
        assert_eq!(commands.nodes.len(), 3);
        assert_eq!(commands.nodes[0].node_type, CommandNodeType::Root);
        assert_eq!(commands.nodes[0].children, vec![1]);
        assert_eq!(commands.nodes[1].node_type, CommandNodeType::Literal);
        assert_eq!(commands.nodes[1].children, vec![2]);
        assert_eq!(commands.nodes[1].name.as_deref(), Some("say"));

        let argument = &commands.nodes[2];
        assert_eq!(argument.node_type, CommandNodeType::Argument);
        assert_eq!(argument.name.as_deref(), Some("message"));
        assert!(argument.executable);
        assert!(argument.restricted);
        assert_eq!(
            argument.suggestions.as_deref(),
            Some("minecraft:ask_server")
        );
        assert_eq!(
            argument.parser,
            Some(CommandArgumentParser {
                type_id: 5,
                name: "brigadier:string".to_string(),
                properties: vec![2],
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 3);
        assert_eq!(decoder.read_u8().unwrap(), 0);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_u8().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_string(32767).unwrap(), "say");
        assert_eq!(decoder.read_u8().unwrap(), 54);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(decoder.read_string(32767).unwrap(), "message");
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_string(32767).unwrap(), "ask_server");
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());

        payload.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_COMMANDS, &payload)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after commands packet")
        );
    }

    #[test]
    fn encodes_commands_packet_wire_order() {
        let commands = Commands {
            nodes: vec![
                CommandNode::root().with_children(vec![1]),
                CommandNode::literal("say")
                    .restrict()
                    .with_children(vec![2]),
                CommandNode::argument(
                    "message",
                    CommandArgumentParser::from_template(
                        VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                        &CommandArgumentTemplate::Unit,
                    )
                    .unwrap(),
                )
                .executes(),
            ],
            root_index: 0,
        };

        let (packet_id, payload) = encode_play_clientbound_commands(&commands).unwrap();
        assert_eq!(packet_id, ids::play::CLIENTBOUND_COMMANDS);

        let mut expected = Encoder::new();
        expected.write_var_i32(3);
        expected.write_u8(0);
        expected.write_var_i32(1);
        expected.write_var_i32(1);
        expected.write_u8(FLAG_RESTRICTED | 1);
        expected.write_var_i32(1);
        expected.write_var_i32(2);
        expected.write_string("say");
        expected.write_u8(FLAG_EXECUTABLE | 2);
        expected.write_var_i32(0);
        expected.write_string("message");
        expected.write_var_i32(VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID);
        expected.write_var_i32(0);
        assert_eq!(payload, expected.into_inner());

        assert_eq!(
            decode_play_clientbound(packet_id, &payload).unwrap(),
            PlayClientbound::Commands(commands)
        );
    }

    #[test]
    fn re_encodes_a_decoded_commands_packet_byte_for_byte() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);

        payload.write_u8(0);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_var_i32(3);

        payload.write_u8(FLAG_RESTRICTED | 1);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_string("say");

        payload.write_u8(FLAG_EXECUTABLE | FLAG_CUSTOM_SUGGESTIONS | 2);
        payload.write_var_i32(0);
        payload.write_string("message");
        payload.write_var_i32(VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID);
        payload.write_var_i32(2);
        payload.write_string("minecraft:ask_server");

        payload.write_u8(FLAG_EXECUTABLE | FLAG_REDIRECT | 1);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_string("execute");

        payload.write_var_i32(0);
        let payload = payload.into_inner();

        let PlayClientbound::Commands(commands) =
            decode_play_clientbound(ids::play::CLIENTBOUND_COMMANDS, &payload).unwrap()
        else {
            panic!("expected commands packet");
        };
        let (packet_id, re_encoded) = encode_play_clientbound_commands(&commands).unwrap();

        assert_eq!(packet_id, ids::play::CLIENTBOUND_COMMANDS);
        assert_eq!(re_encoded, payload);
    }

    #[test]
    fn every_generated_command_argument_template_round_trips_through_one_node() {
        for (type_id, name) in VANILLA_COMMAND_ARGUMENT_TYPES.iter().enumerate() {
            let type_id = type_id as i32;
            let template = match type_id {
                VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID => {
                    CommandArgumentTemplate::Float {
                        min: Some(0.25),
                        max: None,
                    }
                }
                VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID => {
                    CommandArgumentTemplate::Double {
                        min: None,
                        max: Some(5.0),
                    }
                }
                VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID => {
                    CommandArgumentTemplate::Integer {
                        min: Some(-1),
                        max: Some(7),
                    }
                }
                VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID => CommandArgumentTemplate::Long {
                    min: None,
                    max: None,
                },
                VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID => {
                    CommandArgumentTemplate::String {
                        mode: CommandStringMode::GreedyPhrase,
                    }
                }
                VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID => CommandArgumentTemplate::Entity {
                    single: true,
                    players_only: false,
                },
                VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID => {
                    CommandArgumentTemplate::ScoreHolder { multiple: true }
                }
                VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID => {
                    CommandArgumentTemplate::Time { minimum: 20 }
                }
                VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID
                    ..=VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID => {
                    CommandArgumentTemplate::Registry {
                        registry: "minecraft:worldgen/biome".to_string(),
                    }
                }
                _ => CommandArgumentTemplate::Unit,
            };

            let parser = CommandArgumentParser::from_template(type_id, &template).unwrap();
            assert_eq!(parser.name, *name);
            assert_eq!(parser.template().unwrap(), template);

            let commands = Commands {
                nodes: vec![
                    CommandNode::root().with_children(vec![1]),
                    CommandNode::argument("value", parser).executes(),
                ],
                root_index: 0,
            };
            let (packet_id, payload) = encode_play_clientbound_commands(&commands).unwrap();
            assert_eq!(
                decode_play_clientbound(packet_id, &payload).unwrap(),
                PlayClientbound::Commands(commands),
                "parser {type_id} {name} does not survive its own encoder"
            );
        }
    }

    #[test]
    fn rejects_encoding_a_template_its_parser_id_cannot_carry() {
        let error = CommandArgumentParser::from_template(
            VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
            &CommandArgumentTemplate::String {
                mode: CommandStringMode::GreedyPhrase,
            },
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("command argument parser id 20 does not carry this template"));

        // A typed family never encodes as the empty singleton payload either.
        let error = CommandArgumentParser::from_template(
            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
            &CommandArgumentTemplate::Unit,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("command argument parser id 5 does not carry this template"));
    }

    #[test]
    fn rejects_encoding_a_node_whose_flags_disagree_with_its_contents() {
        let mut node = CommandNode::literal("say").executes();
        node.flags = 1;
        let error = encode_play_clientbound_commands(&Commands {
            nodes: vec![CommandNode::root().with_children(vec![1]), node],
            root_index: 0,
        })
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("command node flags 0x01 disagree with the node's own contents (0x05)"));

        let mut root = CommandNode::root();
        root.executable = true;
        let error = encode_play_clientbound_commands(&Commands {
            nodes: vec![root],
            root_index: 0,
        })
        .unwrap_err();
        assert!(error.to_string().contains(
            "root command node must not carry executable, redirect, or restricted flags"
        ));
    }

    #[test]
    fn decodes_commands_argument_property_payloads() {
        let mut payload = Encoder::new();
        payload.write_var_i32(5);

        payload.write_u8(0);
        payload.write_var_i32(4);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(3);
        payload.write_var_i32(4);

        write_argument_node(&mut payload, "speed", 1, |payload| {
            payload.write_u8(3);
            payload.write_f32(0.25);
            payload.write_f32(5.0);
        });
        write_argument_node(&mut payload, "targets", 6, |payload| {
            payload.write_u8(3);
        });
        write_argument_node(&mut payload, "ticks", 43, |payload| {
            payload.write_i32(20);
        });
        write_argument_node(&mut payload, "biome", 46, |payload| {
            payload.write_string("worldgen/biome");
        });

        payload.write_var_i32(0);
        let payload = payload.into_inner();

        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_COMMANDS, &payload).unwrap();
        let PlayClientbound::Commands(commands) = packet else {
            panic!("expected commands packet");
        };

        assert_eq!(
            commands.nodes[1].parser.as_ref().unwrap().name,
            "brigadier:float"
        );
        assert_eq!(
            commands.nodes[1].parser.as_ref().unwrap().properties.len(),
            9
        );
        assert_eq!(
            commands.nodes[1]
                .parser
                .as_ref()
                .unwrap()
                .template()
                .unwrap(),
            CommandArgumentTemplate::Float {
                min: Some(0.25),
                max: Some(5.0),
            }
        );
        assert_eq!(
            commands.nodes[2].parser.as_ref().unwrap().name,
            "minecraft:entity"
        );
        assert_eq!(
            commands.nodes[2].parser.as_ref().unwrap().properties,
            vec![3]
        );
        assert_eq!(
            commands.nodes[2]
                .parser
                .as_ref()
                .unwrap()
                .template()
                .unwrap(),
            CommandArgumentTemplate::Entity {
                single: true,
                players_only: true,
            }
        );
        assert_eq!(
            commands.nodes[3].parser.as_ref().unwrap().name,
            "minecraft:time"
        );
        assert_eq!(
            commands.nodes[3].parser.as_ref().unwrap().properties,
            20i32.to_be_bytes()
        );
        assert_eq!(
            commands.nodes[3]
                .parser
                .as_ref()
                .unwrap()
                .template()
                .unwrap(),
            CommandArgumentTemplate::Time { minimum: 20 }
        );
        assert_eq!(
            commands.nodes[4].parser.as_ref().unwrap().name,
            "minecraft:resource"
        );

        let mut expected_resource_properties = Encoder::new();
        expected_resource_properties.write_string("worldgen/biome");
        assert_eq!(
            commands.nodes[4].parser.as_ref().unwrap().properties,
            expected_resource_properties.into_inner()
        );
        assert_eq!(
            commands.nodes[4]
                .parser
                .as_ref()
                .unwrap()
                .template()
                .unwrap(),
            CommandArgumentTemplate::Registry {
                registry: "minecraft:worldgen/biome".to_string(),
            }
        );
    }

    #[test]
    fn every_generated_command_argument_id_has_one_typed_template_family() {
        for (type_id, name) in VANILLA_COMMAND_ARGUMENT_TYPES.iter().enumerate() {
            let type_id = type_id as i32;
            let mut properties = Encoder::new();
            match type_id {
                1..=4 => properties.write_u8(0),
                5 => properties.write_var_i32(0),
                6 | 31 => properties.write_u8(0),
                43 => properties.write_i32(0),
                44..=48 => properties.write_string("minecraft:test"),
                _ => {}
            }
            let parser = CommandArgumentParser {
                type_id,
                name: (*name).to_string(),
                properties: properties.into_inner(),
            };
            assert!(
                parser.template().is_ok(),
                "generated parser {type_id} {name} has no typed template"
            );
        }

        let trailing = CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
            name: "brigadier:bool".to_string(),
            properties: vec![0],
        };
        assert!(trailing
            .template()
            .unwrap_err()
            .to_string()
            .contains("trailing bytes"));
    }

    #[test]
    fn rejects_invalid_command_custom_suggestion_id() {
        let payload = single_argument_command_payload(
            5,
            |payload| {
                payload.write_var_i32(0);
            },
            Some("minecraft:AskServer"),
        );

        let err = decode_play_clientbound(ids::play::CLIENTBOUND_COMMANDS, &payload).unwrap_err();
        assert!(err.to_string().contains("invalid resource location"));
    }

    #[test]
    fn rejects_invalid_command_resource_argument_registry_key() {
        for parser_id in 44..=48 {
            let payload = single_argument_command_payload(
                parser_id,
                |payload| {
                    payload.write_string("minecraft:Worldgen/Biome");
                },
                None,
            );

            let err =
                decode_play_clientbound(ids::play::CLIENTBOUND_COMMANDS, &payload).unwrap_err();
            assert!(
                err.to_string().contains("invalid resource location"),
                "parser {parser_id} produced unexpected error: {err}"
            );
        }
    }

    #[test]
    fn rejects_impossible_commands_tree() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);

        payload.write_u8(0);
        payload.write_var_i32(1);
        payload.write_var_i32(1);

        payload.write_u8(1);
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_string("cycle");

        payload.write_var_i32(0);

        let err = decode_play_clientbound(ids::play::CLIENTBOUND_COMMANDS, &payload.into_inner())
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("impossible command tree while resolving children"));
    }

    #[test]
    fn validates_deep_linear_command_chain_in_linear_time() {
        const CHAIN_LEN: usize = 8_192;
        let mut nodes = Vec::with_capacity(CHAIN_LEN);
        for index in 0..CHAIN_LEN {
            nodes.push(CommandNode {
                node_type: if index == 0 {
                    CommandNodeType::Root
                } else {
                    CommandNodeType::Literal
                },
                flags: if index == 0 { 0 } else { 1 },
                children: if index + 1 < CHAIN_LEN {
                    vec![(index + 1) as i32]
                } else {
                    Vec::new()
                },
                redirect: None,
                name: (index != 0).then(|| format!("n{index}")),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            });
        }
        validate_command_tree(&nodes, 0).expect("deep acyclic chain should validate");
    }

    fn write_argument_node(
        payload: &mut Encoder,
        name: &str,
        parser_id: i32,
        write_properties: impl FnOnce(&mut Encoder),
    ) {
        payload.write_u8(FLAG_EXECUTABLE | 2);
        payload.write_var_i32(0);
        payload.write_string(name);
        payload.write_var_i32(parser_id);
        write_properties(payload);
    }

    fn single_argument_command_payload(
        parser_id: i32,
        write_properties: impl FnOnce(&mut Encoder),
        suggestions: Option<&str>,
    ) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);

        payload.write_u8(0);
        payload.write_var_i32(1);
        payload.write_var_i32(1);

        let mut flags = FLAG_EXECUTABLE | 2;
        if suggestions.is_some() {
            flags |= FLAG_CUSTOM_SUGGESTIONS;
        }
        payload.write_u8(flags);
        payload.write_var_i32(0);
        payload.write_string("value");
        payload.write_var_i32(parser_id);
        write_properties(&mut payload);
        if let Some(suggestions) = suggestions {
            payload.write_string(suggestions);
        }

        payload.write_var_i32(0);
        payload.into_inner()
    }
}
