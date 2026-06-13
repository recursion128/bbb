use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

const MAX_COMMAND_NODES: usize = 65_536;
const MAX_COMMAND_CHILDREN: usize = 65_536;

const MASK_TYPE: u8 = 0x03;
const FLAG_EXECUTABLE: u8 = 0x04;
const FLAG_REDIRECT: u8 = 0x08;
const FLAG_CUSTOM_SUGGESTIONS: u8 = 0x10;
const FLAG_RESTRICTED: u8 = 0x20;
const KNOWN_FLAGS: u8 =
    MASK_TYPE | FLAG_EXECUTABLE | FLAG_REDIRECT | FLAG_CUSTOM_SUGGESTIONS | FLAG_RESTRICTED;

const COMMAND_ARGUMENT_PARSERS: [&str; 57] = [
    "brigadier:bool",
    "brigadier:float",
    "brigadier:double",
    "brigadier:integer",
    "brigadier:long",
    "brigadier:string",
    "minecraft:entity",
    "minecraft:game_profile",
    "minecraft:block_pos",
    "minecraft:column_pos",
    "minecraft:vec3",
    "minecraft:vec2",
    "minecraft:block_state",
    "minecraft:block_predicate",
    "minecraft:item_stack",
    "minecraft:item_predicate",
    "minecraft:color",
    "minecraft:hex_color",
    "minecraft:component",
    "minecraft:style",
    "minecraft:message",
    "minecraft:nbt_compound_tag",
    "minecraft:nbt_tag",
    "minecraft:nbt_path",
    "minecraft:objective",
    "minecraft:objective_criteria",
    "minecraft:operation",
    "minecraft:particle",
    "minecraft:angle",
    "minecraft:rotation",
    "minecraft:scoreboard_slot",
    "minecraft:score_holder",
    "minecraft:swizzle",
    "minecraft:team",
    "minecraft:item_slot",
    "minecraft:item_slots",
    "minecraft:resource_location",
    "minecraft:function",
    "minecraft:entity_anchor",
    "minecraft:int_range",
    "minecraft:float_range",
    "minecraft:dimension",
    "minecraft:gamemode",
    "minecraft:time",
    "minecraft:resource_or_tag",
    "minecraft:resource_or_tag_key",
    "minecraft:resource",
    "minecraft:resource_key",
    "minecraft:resource_selector",
    "minecraft:template_mirror",
    "minecraft:template_rotation",
    "minecraft:heightmap",
    "minecraft:loot_table",
    "minecraft:loot_predicate",
    "minecraft:loot_modifier",
    "minecraft:dialog",
    "minecraft:uuid",
];

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

pub(super) fn decode_commands(decoder: &mut Decoder<'_>) -> Result<Commands> {
    let count = decoder.read_len()?;
    if count > MAX_COMMAND_NODES {
        return Err(ProtocolError::PacketTooLarge(count, MAX_COMMAND_NODES));
    }

    let mut nodes = Vec::with_capacity(count);
    for _ in 0..count {
        nodes.push(decode_command_node(decoder)?);
    }

    let root_index = decoder.read_var_i32()?;
    validate_command_tree(&nodes, root_index)?;

    Ok(Commands { nodes, root_index })
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

    let mut children = Vec::with_capacity(child_count);
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
            let properties_before = decoder.remaining().to_vec();
            skip_command_argument_properties(decoder, type_id)?;
            let properties_len = properties_before
                .len()
                .checked_sub(decoder.remaining_len())
                .ok_or_else(|| {
                    ProtocolError::InvalidData(
                        "command argument property length underflow".to_string(),
                    )
                })?;
            let properties = properties_before[..properties_len].to_vec();
            let suggestions = if flags & FLAG_CUSTOM_SUGGESTIONS != 0 {
                Some(decoder.read_string(32767)?)
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
    COMMAND_ARGUMENT_PARSERS.get(index).copied()
}

fn skip_command_argument_properties(decoder: &mut Decoder<'_>, type_id: i32) -> Result<()> {
    match type_id {
        1 => skip_number_properties(decoder, NumberPropertyWidth::F32),
        2 => skip_number_properties(decoder, NumberPropertyWidth::F64),
        3 => skip_number_properties(decoder, NumberPropertyWidth::I32),
        4 => skip_number_properties(decoder, NumberPropertyWidth::I64),
        5 => {
            let string_type = decoder.read_var_i32()?;
            if !(0..=2).contains(&string_type) {
                return Err(ProtocolError::InvalidData(format!(
                    "unknown brigadier string argument type {string_type}"
                )));
            }
            Ok(())
        }
        6 | 31 => {
            decoder.read_u8()?;
            Ok(())
        }
        43 => {
            decoder.read_i32()?;
            Ok(())
        }
        44..=48 => {
            decoder.read_string(32767)?;
            Ok(())
        }
        id if command_argument_parser_name(id).is_some() => Ok(()),
        _ => Err(ProtocolError::InvalidData(format!(
            "unknown command argument parser id {type_id}"
        ))),
    }
}

#[derive(Debug, Clone, Copy)]
enum NumberPropertyWidth {
    F32,
    F64,
    I32,
    I64,
}

fn skip_number_properties(decoder: &mut Decoder<'_>, width: NumberPropertyWidth) -> Result<()> {
    let flags = decoder.read_u8()?;
    if flags & !0x03 != 0 {
        return Err(ProtocolError::InvalidData(format!(
            "unknown number argument flags {flags:#04x}"
        )));
    }

    if flags & 0x01 != 0 {
        skip_number_property(decoder, width)?;
    }
    if flags & 0x02 != 0 {
        skip_number_property(decoder, width)?;
    }
    Ok(())
}

fn skip_number_property(decoder: &mut Decoder<'_>, width: NumberPropertyWidth) -> Result<()> {
    match width {
        NumberPropertyWidth::F32 => {
            decoder.read_f32()?;
        }
        NumberPropertyWidth::F64 => {
            decoder.read_f64()?;
        }
        NumberPropertyWidth::I32 => {
            decoder.read_i32()?;
        }
        NumberPropertyWidth::I64 => {
            decoder.read_i64()?;
        }
    }
    Ok(())
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
        payload.write_string("minecraft:ask_server");

        payload.write_var_i32(0);
        let payload = payload.into_inner();

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
        assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:ask_server");
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());
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
            payload.write_string("minecraft:worldgen/biome");
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
            commands.nodes[2].parser.as_ref().unwrap().name,
            "minecraft:entity"
        );
        assert_eq!(
            commands.nodes[2].parser.as_ref().unwrap().properties,
            vec![3]
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
            commands.nodes[4].parser.as_ref().unwrap().name,
            "minecraft:resource"
        );
        assert!(!commands.nodes[4]
            .parser
            .as_ref()
            .unwrap()
            .properties
            .is_empty());
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
}
