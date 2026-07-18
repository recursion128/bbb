use std::collections::HashSet;

use bbb_protocol::packets::{
    CommandArgumentParser as ProtocolCommandArgumentParser, CommandNode as ProtocolCommandNode,
    CommandNodeType as ProtocolCommandNodeType, Commands as ProtocolCommands,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const SIGNABLE_COMMAND_ARGUMENT_PARSER: &str = "minecraft:message";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandTreeState {
    pub root_index: i32,
    pub nodes: Vec<CommandNodeState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandNodeState {
    pub node_type: CommandNodeKindState,
    pub flags: u8,
    pub children: Vec<i32>,
    pub redirect: Option<i32>,
    pub name: Option<String>,
    pub parser: Option<CommandArgumentParserState>,
    pub suggestions: Option<String>,
    pub executable: bool,
    pub restricted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandNodeKindState {
    Root,
    Literal,
    Argument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandArgumentParserState {
    pub type_id: i32,
    pub name: String,
    pub properties: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignableCommandArgument {
    pub name: String,
    pub value: String,
}

impl CommandTreeState {
    pub fn command_requires_signed_arguments(&self, command: &str) -> bool {
        !self.signable_arguments(command).is_empty()
    }

    pub fn signable_arguments(&self, command: &str) -> Vec<SignableCommandArgument> {
        let command = command.trim_start().strip_prefix('/').unwrap_or(command);
        let command = command.trim_start();
        if command.is_empty() {
            return Vec::new();
        }

        let mut visited = HashSet::new();
        self.node_signable_arguments(self.root_index, command, 0, &mut visited)
            .unwrap_or_default()
    }

    fn node_signable_arguments(
        &self,
        node_index: i32,
        command: &str,
        cursor: usize,
        visited: &mut HashSet<(i32, usize)>,
    ) -> Option<Vec<SignableCommandArgument>> {
        if !visited.insert((node_index, cursor)) {
            return None;
        }

        let Some(node) = self.node(node_index) else {
            return None;
        };

        if let Some(redirect) = node.redirect {
            if let Some(arguments) =
                self.node_signable_arguments(redirect, command, cursor, visited)
            {
                return Some(arguments);
            }
        }

        let argument_start = skip_ascii_whitespace(command, cursor);
        if argument_start == command.len() {
            return None;
        }

        let mut matched_literal = false;
        for child_index in &node.children {
            let Some(child) = self.node(*child_index) else {
                continue;
            };
            if child.node_type != CommandNodeKindState::Literal {
                continue;
            }
            let Some(literal_end) = child
                .name
                .as_deref()
                .and_then(|literal| match_literal(command, argument_start, literal))
            else {
                continue;
            };
            matched_literal = true;
            if let Some(arguments) =
                self.node_signable_arguments(*child_index, command, literal_end, visited)
            {
                return Some(arguments);
            }
        }

        if matched_literal {
            return None;
        }

        for child_index in &node.children {
            let Some(child) = self.node(*child_index) else {
                continue;
            };
            if child.node_type != CommandNodeKindState::Argument {
                continue;
            }
            let Some(parser) = child.parser.as_ref() else {
                continue;
            };
            if parser.name == SIGNABLE_COMMAND_ARGUMENT_PARSER {
                return Some(vec![SignableCommandArgument {
                    name: child.name.clone().unwrap_or_default(),
                    value: command[argument_start..].to_string(),
                }]);
            }
            let Some(argument_end) = parse_argument_end(command, argument_start, parser) else {
                continue;
            };
            if let Some(arguments) =
                self.node_signable_arguments(*child_index, command, argument_end, visited)
            {
                return Some(arguments);
            }
        }

        None
    }

    fn node(&self, index: i32) -> Option<&CommandNodeState> {
        let index = usize::try_from(index).ok()?;
        self.nodes.get(index)
    }
}

impl From<ProtocolCommands> for CommandTreeState {
    fn from(packet: ProtocolCommands) -> Self {
        Self {
            root_index: packet.root_index,
            nodes: packet
                .nodes
                .into_iter()
                .map(CommandNodeState::from)
                .collect(),
        }
    }
}

impl From<ProtocolCommandNode> for CommandNodeState {
    fn from(node: ProtocolCommandNode) -> Self {
        Self {
            node_type: CommandNodeKindState::from(node.node_type),
            flags: node.flags,
            children: node.children,
            redirect: node.redirect,
            name: node.name,
            parser: node.parser.map(CommandArgumentParserState::from),
            suggestions: node.suggestions,
            executable: node.executable,
            restricted: node.restricted,
        }
    }
}

impl From<ProtocolCommandNodeType> for CommandNodeKindState {
    fn from(node_type: ProtocolCommandNodeType) -> Self {
        match node_type {
            ProtocolCommandNodeType::Root => Self::Root,
            ProtocolCommandNodeType::Literal => Self::Literal,
            ProtocolCommandNodeType::Argument => Self::Argument,
        }
    }
}

impl From<ProtocolCommandArgumentParser> for CommandArgumentParserState {
    fn from(parser: ProtocolCommandArgumentParser) -> Self {
        Self {
            type_id: parser.type_id,
            name: parser.name,
            properties: parser.properties,
        }
    }
}

impl WorldStore {
    pub fn apply_commands(&mut self, packet: ProtocolCommands) -> &CommandTreeState {
        self.counters.command_tree_packets += 1;
        self.commands = CommandTreeState::from(packet);
        self.update_command_tree_counters();
        &self.commands
    }

    pub fn commands(&self) -> &CommandTreeState {
        &self.commands
    }

    pub fn command_requires_signed_arguments(&self, command: &str) -> bool {
        self.commands.command_requires_signed_arguments(command)
    }

    pub fn signable_command_arguments(&self, command: &str) -> Vec<SignableCommandArgument> {
        self.commands.signable_arguments(command)
    }

    pub(crate) fn reset_command_tree_for_new_listener(&mut self) {
        self.commands = CommandTreeState::default();
        self.update_command_tree_counters();
    }

    fn update_command_tree_counters(&mut self) {
        self.counters.command_nodes_tracked = self.commands.nodes.len();
        self.counters.command_literal_nodes_tracked = self
            .commands
            .nodes
            .iter()
            .filter(|node| node.node_type == CommandNodeKindState::Literal)
            .count();
        self.counters.command_argument_nodes_tracked = self
            .commands
            .nodes
            .iter()
            .filter(|node| node.node_type == CommandNodeKindState::Argument)
            .count();
        self.counters.command_redirect_nodes_tracked = self
            .commands
            .nodes
            .iter()
            .filter(|node| node.redirect.is_some())
            .count();
        self.counters.command_executable_nodes_tracked = self
            .commands
            .nodes
            .iter()
            .filter(|node| node.executable)
            .count();
        self.counters.command_restricted_nodes_tracked = self
            .commands
            .nodes
            .iter()
            .filter(|node| node.restricted)
            .count();
        self.counters.last_command_root_index =
            (!self.commands.nodes.is_empty()).then_some(self.commands.root_index);
    }
}

fn skip_ascii_whitespace(command: &str, mut cursor: usize) -> usize {
    while command
        .as_bytes()
        .get(cursor)
        .is_some_and(u8::is_ascii_whitespace)
    {
        cursor += 1;
    }
    cursor
}

fn match_literal(command: &str, start: usize, literal: &str) -> Option<usize> {
    let end = start.checked_add(literal.len())?;
    (command.get(start..end) == Some(literal)
        && command
            .as_bytes()
            .get(end)
            .is_none_or(u8::is_ascii_whitespace))
    .then_some(end)
}

fn parse_argument_end(
    command: &str,
    start: usize,
    parser: &CommandArgumentParserState,
) -> Option<usize> {
    match parser.name.as_str() {
        "brigadier:string" => match parser.properties.first().copied().unwrap_or_default() {
            2 => Some(command.len()),
            1 => parse_quotable_argument_end(command, start),
            _ => parse_lexical_argument_end(command, start),
        },
        "minecraft:vec3" | "minecraft:block_pos" => parse_argument_parts_end(command, start, 3),
        "minecraft:vec2" | "minecraft:rotation" | "minecraft:column_pos" => {
            parse_argument_parts_end(command, start, 2)
        }
        _ => parse_lexical_argument_end(command, start),
    }
}

fn parse_argument_parts_end(command: &str, mut cursor: usize, count: usize) -> Option<usize> {
    for _ in 0..count {
        cursor = skip_ascii_whitespace(command, cursor);
        cursor = parse_lexical_argument_end(command, cursor)?;
    }
    Some(cursor)
}

fn parse_quotable_argument_end(command: &str, start: usize) -> Option<usize> {
    if command.as_bytes().get(start) == Some(&b'\"') {
        return parse_quoted_end(command, start);
    }
    parse_lexical_argument_end(command, start)
}

fn parse_quoted_end(command: &str, start: usize) -> Option<usize> {
    let bytes = command.as_bytes();
    let quote = *bytes.get(start)?;
    let mut cursor = start + 1;
    let mut escaped = false;
    while let Some(byte) = bytes.get(cursor).copied() {
        cursor += 1;
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == quote {
            return Some(cursor);
        }
    }
    None
}

fn parse_lexical_argument_end(command: &str, start: usize) -> Option<usize> {
    let bytes = command.as_bytes();
    if matches!(bytes.get(start), Some(b'\"' | b'\'')) {
        return parse_quoted_end(command, start);
    }

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
            byte if byte.is_ascii_whitespace() && delimiters.is_empty() => break,
            _ => {}
        }
        cursor += 1;
    }
    (cursor > start && quote.is_none() && delimiters.is_empty()).then_some(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        CommandArgumentParser, CommandNode, CommandNodeType, Commands,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
    };

    #[test]
    fn commands_replace_current_command_tree_and_counters() {
        let mut store = WorldStore::new();

        store.apply_commands(command_tree("say", "brigadier:string"));
        store.apply_commands(command_tree("tell", "brigadier:string"));

        let commands = store.commands();
        assert_eq!(commands.root_index, 0);
        assert_eq!(commands.nodes.len(), 3);
        assert_eq!(commands.nodes[1].name.as_deref(), Some("tell"));
        assert_eq!(commands.nodes[2].name.as_deref(), Some("message"));
        assert_eq!(
            commands.nodes[2].parser.as_ref().unwrap().name,
            "brigadier:string"
        );
        assert_eq!(
            commands.nodes[2].suggestions.as_deref(),
            Some("minecraft:ask_server")
        );

        let counters = store.counters();
        assert_eq!(counters.command_tree_packets, 2);
        assert_eq!(counters.command_nodes_tracked, 3);
        assert_eq!(counters.command_literal_nodes_tracked, 1);
        assert_eq!(counters.command_argument_nodes_tracked, 1);
        assert_eq!(counters.command_executable_nodes_tracked, 1);
        assert_eq!(counters.command_restricted_nodes_tracked, 1);
        assert_eq!(counters.command_redirect_nodes_tracked, 0);
        assert_eq!(counters.last_command_root_index, Some(0));
    }

    #[test]
    fn configuration_reentry_rebuilds_command_tree_and_preserves_packet_counters() {
        let mut store = WorldStore::new();
        store.apply_commands(command_tree("say", "minecraft:message"));

        store.clear_client_level();

        assert_eq!(store.commands(), &CommandTreeState::default());
        assert_eq!(store.counters().command_tree_packets, 1);
        assert_eq!(store.counters().command_nodes_tracked, 0);
        assert_eq!(store.counters().command_literal_nodes_tracked, 0);
        assert_eq!(store.counters().command_argument_nodes_tracked, 0);
        assert_eq!(store.counters().command_redirect_nodes_tracked, 0);
        assert_eq!(store.counters().command_executable_nodes_tracked, 0);
        assert_eq!(store.counters().command_restricted_nodes_tracked, 0);
        assert_eq!(store.counters().last_command_root_index, None);
    }

    #[test]
    fn detects_signable_message_argument_commands() {
        let mut store = WorldStore::new();
        store.apply_commands(command_tree("say", "minecraft:message"));

        assert!(store.command_requires_signed_arguments("say hello world"));
        assert!(store.command_requires_signed_arguments("/say hello world"));
        assert_eq!(
            store.signable_command_arguments("say   hello  world  "),
            vec![SignableCommandArgument {
                name: "message".to_string(),
                value: "hello  world  ".to_string(),
            }]
        );
        assert!(!store.command_requires_signed_arguments("time set day"));
        assert!(!store.command_requires_signed_arguments(""));
        assert!(store.signable_command_arguments("say").is_empty());
    }

    #[test]
    fn does_not_treat_plain_string_arguments_as_signable() {
        let mut store = WorldStore::new();
        store.apply_commands(command_tree("say", "brigadier:string"));

        assert!(!store.command_requires_signed_arguments("say hello"));
    }

    #[test]
    fn detects_nested_signable_message_argument_commands() {
        let mut store = WorldStore::new();
        store.apply_commands(nested_message_command_tree());

        assert!(store.command_requires_signed_arguments("tell Steve hello world"));
        assert_eq!(
            store.signable_command_arguments("tell @e[name=\"A B\"] hello world"),
            vec![SignableCommandArgument {
                name: "message".to_string(),
                value: "hello world".to_string(),
            }]
        );
        assert!(!store.command_requires_signed_arguments("tell"));
    }

    #[test]
    fn follows_redirects_and_keeps_literal_precedence() {
        let mut store = WorldStore::new();
        store.apply_commands(redirected_message_command_tree());

        assert_eq!(
            store.signable_command_arguments("w Steve redirected message"),
            vec![SignableCommandArgument {
                name: "message".to_string(),
                value: "redirected message".to_string(),
            }]
        );
        assert!(store.signable_command_arguments("time set day").is_empty());
    }

    fn command_tree(literal: &str, parser_name: &str) -> Commands {
        Commands {
            root_index: 0,
            nodes: vec![
                CommandNode {
                    node_type: CommandNodeType::Root,
                    flags: 0,
                    children: vec![1],
                    redirect: None,
                    name: None,
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![2],
                    redirect: None,
                    name: Some(literal.to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 54,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("message".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: if parser_name == "minecraft:message" {
                            VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID
                        } else {
                            VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID
                        },
                        name: parser_name.to_string(),
                        properties: vec![2],
                    }),
                    suggestions: Some("minecraft:ask_server".to_string()),
                    executable: true,
                    restricted: true,
                },
            ],
        }
    }

    fn nested_message_command_tree() -> Commands {
        Commands {
            root_index: 0,
            nodes: vec![
                CommandNode {
                    node_type: CommandNodeType::Root,
                    flags: 0,
                    children: vec![1],
                    redirect: None,
                    name: None,
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![2],
                    redirect: None,
                    name: Some("tell".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 2,
                    children: vec![3],
                    redirect: None,
                    name: Some("target".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
                        name: "minecraft:entity".to_string(),
                        properties: Vec::new(),
                    }),
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("message".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                        name: "minecraft:message".to_string(),
                        properties: Vec::new(),
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
            ],
        }
    }

    fn redirected_message_command_tree() -> Commands {
        let mut tree = nested_message_command_tree();
        tree.nodes[0].children.extend([4, 5]);
        tree.nodes.push(CommandNode {
            node_type: CommandNodeType::Literal,
            flags: 1,
            children: Vec::new(),
            redirect: Some(1),
            name: Some("w".to_string()),
            parser: None,
            suggestions: None,
            executable: false,
            restricted: false,
        });
        tree.nodes.push(CommandNode {
            node_type: CommandNodeType::Literal,
            flags: 1,
            children: Vec::new(),
            redirect: None,
            name: Some("time".to_string()),
            parser: None,
            suggestions: None,
            executable: true,
            restricted: false,
        });
        tree
    }
}
