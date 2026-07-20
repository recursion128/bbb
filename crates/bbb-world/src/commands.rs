use bbb_protocol::packets::{
    CommandArgumentParser as ProtocolCommandArgumentParser, CommandNode as ProtocolCommandNode,
    CommandNodeType as ProtocolCommandNodeType, Commands as ProtocolCommands,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

mod dispatcher;

pub use dispatcher::{
    CommandCompletionContext, CommandParseResult, CommandSyntaxErrorKind, CommandSyntaxErrorState,
    ParsedCommandArgument,
};

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
        !self.parse_command(command).signable_arguments.is_empty()
    }

    pub fn signable_arguments(&self, command: &str) -> Vec<SignableCommandArgument> {
        self.parse_command(command).signable_arguments
    }

    fn node(&self, index: i32) -> Option<&CommandNodeState> {
        let index = usize::try_from(index).ok()?;
        self.nodes.get(index)
    }
}

impl TryFrom<ProtocolCommands> for CommandTreeState {
    type Error = String;

    fn try_from(packet: ProtocolCommands) -> Result<Self, Self::Error> {
        Ok(Self {
            root_index: packet.root_index,
            nodes: packet
                .nodes
                .into_iter()
                .enumerate()
                .map(|(index, node)| {
                    CommandNodeState::try_from(node)
                        .map_err(|error| format!("command node {index}: {error}"))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl TryFrom<ProtocolCommandNode> for CommandNodeState {
    type Error = String;

    fn try_from(node: ProtocolCommandNode) -> Result<Self, Self::Error> {
        let parser = node
            .parser
            .map(CommandArgumentParserState::try_from)
            .transpose()?;
        Ok(Self {
            node_type: CommandNodeKindState::from(node.node_type),
            flags: node.flags,
            children: node.children,
            redirect: node.redirect,
            name: node.name,
            parser,
            suggestions: node.suggestions,
            executable: node.executable,
            restricted: node.restricted,
        })
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

impl TryFrom<ProtocolCommandArgumentParser> for CommandArgumentParserState {
    type Error = String;

    fn try_from(parser: ProtocolCommandArgumentParser) -> Result<Self, Self::Error> {
        parser
            .template()
            .map_err(|error| format!("invalid {} template: {error}", parser.name))?;
        Ok(Self {
            type_id: parser.type_id,
            name: parser.name,
            properties: parser.properties,
        })
    }
}

impl WorldStore {
    pub fn apply_commands(&mut self, packet: ProtocolCommands) -> bool {
        self.counters.command_tree_packets += 1;
        let commands = match CommandTreeState::try_from(packet) {
            Ok(commands) => commands,
            Err(error) => {
                self.record_apply_error("command_tree", error);
                return false;
            }
        };
        self.commands = commands;
        self.update_command_tree_counters();
        true
    }

    pub fn commands(&self) -> &CommandTreeState {
        &self.commands
    }

    pub fn command_requires_signed_arguments(&self, command: &str) -> bool {
        !self.parse_command(command).signable_arguments.is_empty()
    }

    pub fn signable_command_arguments(&self, command: &str) -> Vec<SignableCommandArgument> {
        self.parse_command(command).signable_arguments
    }

    pub fn parse_command(&self, command: &str) -> CommandParseResult {
        self.commands
            .parse_command_with_registry(command, Some(self.frozen_registry_access()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::packets::{
        CommandArgumentParser, CommandNode, CommandNodeType, Commands, RegistryTags,
        TagNetworkPayload, UpdateTags, VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID,
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
    fn command_tree_replacement_rejects_invalid_typed_template_atomically() {
        let mut store = WorldStore::new();
        assert!(store.apply_commands(command_tree("say", "minecraft:message")));
        let retained = store.commands().clone();

        let mut invalid = command_tree("speed", "brigadier:string");
        invalid.nodes[2]
            .parser
            .as_mut()
            .expect("fixture argument parser")
            .properties
            .push(0);
        assert!(!store.apply_commands(invalid));

        assert_eq!(store.commands(), &retained);
        assert_eq!(store.counters().command_tree_packets, 2);
        assert_eq!(store.counters().command_nodes_tracked, 3);
        assert_eq!(store.counters().world_apply_errors, 1);
        assert!(store
            .apply_diagnostics()
            .last_apply_error
            .as_ref()
            .is_some_and(
                |error| error.source == "command_tree" && error.message.contains("trailing bytes")
            ));
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
                // Brigadier consumes exactly one argument separator. The
                // remaining spaces are part of the greedy signed message.
                value: "  hello  world  ".to_string(),
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

    #[test]
    fn registry_aware_dispatch_uses_exact_world_generation_membership() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "example:things",
            0,
            vec![
                RegistryPacketEntry::stub("example:first"),
                RegistryPacketEntry::stub("example:second"),
            ],
        );

        store.apply_commands(registry_command_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
            "minecraft:resource",
            "example:things",
        ));
        assert!(store
            .parse_command("choose example:first")
            .is_valid_command());
        assert!(!store
            .parse_command("choose example:missing")
            .is_valid_command());

        store.apply_commands(registry_command_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
            "minecraft:resource_key",
            "example:things",
        ));
        assert!(store
            .parse_command("choose example:future")
            .is_valid_command());

        store.apply_commands(registry_command_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID,
            "minecraft:resource_selector",
            "example:things",
        ));
        assert!(store.parse_command("choose example:f*").is_valid_command());
        assert!(!store.parse_command("choose other:*").is_valid_command());
    }

    #[test]
    fn registry_tag_replacement_and_retained_store_clone_do_not_cross_generations() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "example:things",
            0,
            vec![RegistryPacketEntry::stub("example:first")],
        );
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "example:things".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:selected".to_string(),
                    entries: vec![0],
                }],
            }],
        });
        store.apply_commands(registry_command_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
            "minecraft:resource_or_tag",
            "example:things",
        ));
        let generation_a = store.clone();
        assert!(generation_a
            .parse_command("choose #example:selected")
            .is_valid_command());

        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "example:things".to_string(),
                tags: Vec::new(),
            }],
        });
        assert!(!store
            .parse_command("choose #example:selected")
            .is_valid_command());
        assert!(generation_a
            .parse_command("choose #example:selected")
            .is_valid_command());
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
                        properties: (parser_name == "brigadier:string")
                            .then_some(vec![2])
                            .unwrap_or_default(),
                    }),
                    suggestions: Some("minecraft:ask_server".to_string()),
                    executable: true,
                    restricted: true,
                },
            ],
        }
    }

    fn registry_command_tree(type_id: i32, parser_name: &str, registry: &str) -> Commands {
        let mut properties = Encoder::new();
        properties.write_string(registry);
        let mut tree = command_tree("choose", "brigadier:string");
        let argument = tree.nodes[2]
            .parser
            .as_mut()
            .expect("fixture argument parser");
        argument.type_id = type_id;
        argument.name = parser_name.to_string();
        argument.properties = properties.into_inner();
        tree.nodes[2].name = Some("value".to_string());
        tree
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
                        // Vanilla EntityArgument.Info flags: multiple entities,
                        // not players-only.
                        properties: vec![0],
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
