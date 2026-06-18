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

impl CommandTreeState {
    pub fn command_requires_signed_arguments(&self, command: &str) -> bool {
        let command = command.trim().trim_start_matches('/').trim_start();
        if command.is_empty() {
            return false;
        }

        let tokens = command.split_whitespace().collect::<Vec<_>>();
        let mut visited = HashSet::new();
        self.node_requires_signed_arguments(self.root_index, &tokens, 0, &mut visited)
    }

    fn node_requires_signed_arguments(
        &self,
        node_index: i32,
        tokens: &[&str],
        position: usize,
        visited: &mut HashSet<(i32, usize)>,
    ) -> bool {
        if !visited.insert((node_index, position)) {
            return false;
        }

        let Some(node) = self.node(node_index) else {
            return false;
        };

        if let Some(redirect) = node.redirect {
            if self.node_requires_signed_arguments(redirect, tokens, position, visited) {
                return true;
            }
        }

        let Some(token) = tokens.get(position) else {
            return false;
        };

        let mut matched_literal = false;
        for child_index in &node.children {
            let Some(child) = self.node(*child_index) else {
                continue;
            };
            if child.node_type == CommandNodeKindState::Literal
                && child.name.as_deref() == Some(*token)
            {
                matched_literal = true;
                if self.node_requires_signed_arguments(*child_index, tokens, position + 1, visited)
                {
                    return true;
                }
            }
        }

        if matched_literal {
            return false;
        }

        for child_index in &node.children {
            let Some(child) = self.node(*child_index) else {
                continue;
            };
            if child.node_type != CommandNodeKindState::Argument {
                continue;
            }
            if child
                .parser
                .as_ref()
                .is_some_and(|parser| parser.name == SIGNABLE_COMMAND_ARGUMENT_PARSER)
            {
                return true;
            }
            if self.node_requires_signed_arguments(*child_index, tokens, position + 1, visited) {
                return true;
            }
        }

        false
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
        self.counters.last_command_root_index = Some(self.commands.root_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{CommandArgumentParser, CommandNode, CommandNodeType, Commands};

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
    fn detects_signable_message_argument_commands() {
        let mut store = WorldStore::new();
        store.apply_commands(command_tree("say", "minecraft:message"));

        assert!(store.command_requires_signed_arguments("say hello world"));
        assert!(store.command_requires_signed_arguments("/say hello world"));
        assert!(!store.command_requires_signed_arguments("time set day"));
        assert!(!store.command_requires_signed_arguments(""));
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
        assert!(!store.command_requires_signed_arguments("tell"));
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
                            20
                        } else {
                            5
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
                        type_id: 6,
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
                        type_id: 20,
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
}
