use serde::{Deserialize, Serialize};

use super::read_resource_location;
use crate::codec::{Decoder, ProtocolError, Result};

mod generated_26_1;

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

impl CommandArgumentParser {
    /// Replays the packet framing decoder over the retained exact property
    /// bytes. This is the only semantic template decoder.
    pub fn template(&self) -> Result<CommandArgumentTemplate> {
        command_argument_template(self.type_id, &self.properties)
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

pub(super) fn decode_commands(decoder: &mut Decoder<'_>) -> Result<Commands> {
    let count = decoder.read_len()?;
    if count > MAX_COMMAND_NODES {
        return Err(ProtocolError::PacketTooLarge(count, MAX_COMMAND_NODES));
    }

    let mut nodes = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
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

    let mut children = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(
        child_count,
    ));
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

    validate_command_topology(nodes, "redirect", |node, pending| {
        node.redirect
            .map(|redirect| !pending[redirect as usize])
            .unwrap_or(true)
    })?;
    validate_command_topology(nodes, "children", |node, pending| {
        node.children.iter().all(|child| !pending[*child as usize])
    })
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

fn validate_command_topology<F>(
    nodes: &[CommandNode],
    phase: &'static str,
    mut can_remove: F,
) -> Result<()>
where
    F: FnMut(&CommandNode, &[bool]) -> bool,
{
    let mut pending = vec![true; nodes.len()];
    let mut remaining = nodes.len();

    while remaining > 0 {
        let mut progressed = false;
        for index in 0..nodes.len() {
            if pending[index] && can_remove(&nodes[index], &pending) {
                pending[index] = false;
                remaining -= 1;
                progressed = true;
            }
        }

        if !progressed {
            return Err(ProtocolError::InvalidData(format!(
                "server sent an impossible command tree while resolving {phase}"
            )));
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
