use bbb_protocol::packets::{
    CommandArgumentParser as ProtocolCommandArgumentParser, CommandNode as ProtocolCommandNode,
    CommandNodeType as ProtocolCommandNodeType, Commands as ProtocolCommands,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

mod dispatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnattendedCommandCheck {
    NoIssues,
    ParseErrors,
    SignatureRequired,
    PermissionsRequired,
}

pub use dispatcher::{
    matches_suggestion_sub_str, CommandCompletionContext, CommandCompletionResult,
    CommandCompletionSuggestion, CommandParseResult, CommandSyntaxErrorKind,
    CommandSyntaxErrorState, ParsedCommandArgument, RawCommandParseResult,
    RawCommandSyntaxErrorState, RawParsedCommandArgument,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandCompletionRevision {
    sources: u64,
    registry: crate::RegistryAccessRevision,
    client_level_generation: u64,
    vanilla_feature_enabled: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct CommandCompletionTarget {
    looked_at_block: Option<(crate::BlockPos, [f64; 3])>,
    targeted_entity_uuid: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandTreeState {
    pub root_index: i32,
    pub nodes: Vec<CommandNodeState>,
    #[serde(skip)]
    completion_target: CommandCompletionTarget,
}

impl PartialEq for CommandTreeState {
    fn eq(&self, other: &Self) -> bool {
        self.root_index == other.root_index && self.nodes == other.nodes
    }
}

impl Eq for CommandTreeState {}

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

pub use bbb_protocol::packets::SignableCommandArgument;

impl CommandTreeState {
    pub fn command_requires_signed_arguments(&self, command: &str) -> bool {
        !self.parse_command(command).signable_arguments.is_empty()
    }

    pub fn signable_arguments(&self, command: &str) -> Vec<SignableCommandArgument> {
        self.parse_command(command).signable_arguments
    }

    pub fn command_requires_signed_arguments_utf16(&self, command: &[u16]) -> bool {
        !self
            .parse_command_utf16(command)
            .signable_arguments
            .is_empty()
    }

    pub fn signable_arguments_utf16(&self, command: &[u16]) -> Vec<SignableCommandArgument> {
        self.parse_command_utf16(command).signable_arguments
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
            completion_target: CommandCompletionTarget::default(),
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
        if *self.commands != commands {
            let previous_target = self.commands.completion_target;
            let mut updated = commands;
            updated.completion_target = previous_target;
            self.commands = updated.into();
            self.advance_command_completion_sources_revision();
        }
        self.update_command_tree_counters();
        true
    }

    pub fn command_completion_revision(&self) -> CommandCompletionRevision {
        CommandCompletionRevision {
            sources: self.command_completion_sources_revision,
            registry: self.registry_access_revision(),
            client_level_generation: self.client_level_generation(),
            vanilla_feature_enabled: self.is_feature_enabled("minecraft:vanilla"),
        }
    }

    pub(crate) fn advance_command_completion_sources_revision(&mut self) {
        self.command_completion_sources_revision =
            self.command_completion_sources_revision.saturating_add(1);
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

    /// The signed/unsigned decision and the signed values, read from the raw
    /// buffer the editor holds: a `String` projection would lose an isolated
    /// surrogate before the `SignedArgument` value was built.
    pub fn command_requires_signed_arguments_utf16(&self, command: &[u16]) -> bool {
        self.commands
            .command_requires_signed_arguments_utf16(command)
    }

    pub fn signable_command_arguments_utf16(
        &self,
        command: &[u16],
    ) -> Vec<SignableCommandArgument> {
        self.commands.signable_arguments_utf16(command)
    }

    pub fn parse_command(&self, command: &str) -> CommandParseResult {
        self.commands
            .parse_command_with_registry(command, Some(self.frozen_registry_access()))
    }

    pub fn parse_command_utf16(&self, command: &[u16]) -> RawCommandParseResult {
        self.commands
            .parse_raw_command_with_registry(command, Some(self.frozen_registry_access()))
    }

    pub fn check_unattended_command(&self, command: &str) -> UnattendedCommandCheck {
        self.commands
            .check_unattended_command(command, Some(self.frozen_registry_access()))
    }

    pub fn check_unattended_command_utf16(&self, command: &[u16]) -> UnattendedCommandCheck {
        self.commands
            .check_unattended_command_utf16(command, Some(self.frozen_registry_access()))
    }

    /// Records the current crosshair hit target for Vanilla
    /// `ClientSuggestionProvider` coordinate and entity UUID tab completion
    /// (`ClientSuggestionProvider.java:70-74, 125-144`).
    pub fn set_command_completion_target(
        &mut self,
        looked_at_block: Option<(crate::BlockPos, [f64; 3])>,
        targeted_entity_uuid: Option<uuid::Uuid>,
    ) {
        let target = CommandCompletionTarget {
            looked_at_block,
            targeted_entity_uuid,
        };
        if self.commands.completion_target != target {
            self.commands.completion_target = target;
            self.advance_command_completion_sources_revision();
        }
    }

    /// Completion positions are UTF-16 units of `command`, matching
    /// `SuggestionContext.startPos` and the edit buffer's own indexing.
    pub fn complete_command(&self, command: &str) -> CommandCompletionResult {
        self.complete_command_utf16(&command.encode_utf16().collect::<Vec<_>>())
    }

    pub fn complete_command_utf16(&self, command: &[u16]) -> CommandCompletionResult {
        let registry_access = self.frozen_registry_access();
        let target = self.commands.completion_target;
        self.commands.complete_command_with_sources(
            command,
            dispatcher::CommandCompletionSources {
                registry_access,
                online_player_names: self
                    .player_info
                    .entries
                    .values()
                    .map(|entry| entry.profile.name.as_str())
                    .collect(),
                team_names: self.scoreboard.teams.keys().map(String::as_str).collect(),
                available_levels: self.available_levels.iter().map(String::as_str).collect(),
                available_sounds: self
                    .available_command_sounds
                    .iter()
                    .map(String::as_str)
                    .collect(),
                // Every 26.1 entity type uses EntityType.Builder's default
                // FeatureFlags.VANILLA_SET; the source contains no per-type
                // requiredFeatures override.
                vanilla_feature_enabled: self.is_feature_enabled("minecraft:vanilla"),
                looked_at_block: target.looked_at_block,
                targeted_entity_uuid: target.targeted_entity_uuid,
            },
        )
    }

    pub fn set_available_command_sounds(&mut self, mut sounds: Vec<String>) {
        sounds.sort();
        sounds.dedup();
        if self.available_command_sounds.as_slice() != sounds.as_slice() {
            self.available_command_sounds = sounds.into();
            self.advance_command_completion_sources_revision();
        }
    }

    pub(crate) fn reset_command_tree_for_new_listener(&mut self) {
        let commands = CommandTreeState::default();
        if *self.commands != commands
            || self.commands.completion_target != CommandCompletionTarget::default()
        {
            self.commands = commands.into();
            self.advance_command_completion_sources_revision();
        }
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
        TagNetworkPayload, UpdateTags, VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID, VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID,
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
    fn completion_positions_are_units_of_the_raw_command() {
        let mut store = WorldStore::new();
        assert!(store.apply_commands(nested_message_command_tree()));

        // The message token starts at unit 20 after a selector holding an
        // emoji; the same prefix is 22 bytes, and `SuggestionContext.startPos`
        // counts characters -- as does the edit buffer the client splices into.
        let prefix = "/tell @e[name=\"😀\"] y";
        let raw: Vec<u16> = prefix.encode_utf16().collect();
        assert_eq!(raw.len(), 21);
        assert_eq!(prefix.len(), 23);
        let parsed = store.parse_command_utf16(&raw);
        assert_eq!(parsed.arguments.len(), 2);
        assert_eq!(parsed.arguments[1].start, 20);
        let completion = store.complete_command_utf16(&raw);
        assert_eq!((completion.start, completion.length), (20, 1));

        // An unfinished literal is replaced from the unit where it starts.
        let raw: Vec<u16> = "/tel".encode_utf16().collect();
        let completion = store.complete_command_utf16(&raw);
        assert_eq!((completion.start, completion.length), (1, 3));
        assert_eq!(completion.suggestions[0].text, "tell");

        // The `&str` entry answers with the same unit positions.
        let byte_entry = store.complete_command("/tel");
        assert_eq!(
            (byte_entry.start, byte_entry.length),
            (completion.start, completion.length)
        );
    }

    #[test]
    fn literal_completion_lowercases_the_whole_buffer_before_the_tail() {
        // `SuggestionsBuilder` cuts `SuggestionContext.startPos` out of the
        // lowercased buffer, so a unit the lowercase mapping lengthens before
        // that position moves the tail it hands to the literal.
        let mut store = WorldStore::new();
        assert!(store.apply_commands(nested_literal_command_tree(&["\u{130}", "ok"])));
        let expanding: Vec<u16> = "/\u{130} o".encode_utf16().collect();
        assert!(store
            .complete_command_utf16(&expanding)
            .suggestions
            .is_empty());

        // The same buffer without a lengthening mapping completes the child.
        let mut store = WorldStore::new();
        assert!(store.apply_commands(nested_literal_command_tree(&["a", "ok"])));
        let ascii: Vec<u16> = "/a o".encode_utf16().collect();
        let completion = store.complete_command_utf16(&ascii);
        assert_eq!((completion.start, completion.length), (3, 1));
        assert_eq!(completion.suggestions[0].text, "ok");
    }

    #[test]
    fn resource_provider_completion_matches_sound_ids_by_utf16_units() {
        // `SuggestionProviders.AVAILABLE_SOUNDS` (SuggestionProviders.java:20-22)
        // reaches `SharedSuggestionProvider.suggestResource` over the same
        // lowered tail; the expectations are the pinned jar's answers for these
        // buffers.
        let mut store = WorldStore::new();
        store.set_available_command_sounds(vec![
            "example:first".to_string(),
            "example:second".to_string(),
        ]);
        let mut sounds = command_tree("playsound", "brigadier:string");
        sounds.nodes[2].suggestions = Some("minecraft:available_sounds".to_string());
        assert!(store.apply_commands(sounds));

        // ASCII control: both ids answer, in unit order.
        let completion = store.complete_command_utf16(&units_of("/playsound ex"));
        assert_eq!((completion.start, completion.length), (11, 2));
        assert_eq!(
            suggestion_units(&completion),
            vec![units_of("example:first"), units_of("example:second")]
        );

        // A replaced unit matches none of them, and neither does the unpaired
        // surrogate the projection used to turn into one.
        assert!(store
            .complete_command_utf16(&command_units("/playsound e", &[0xfffd]))
            .suggestions
            .is_empty());
        assert!(store
            .complete_command_utf16(&command_units("/playsound e", &[0xd83d]))
            .suggestions
            .is_empty());
        assert!(store
            .complete_command("/playsound example:first")
            .suggestions
            .is_empty());
        assert_eq!(
            suggestion_units(&store.complete_command("/playsound EXAMPLE:FIRST")),
            vec![units_of("example:first")]
        );
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
                value: "  hello  world  ".into(),
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
                value: "hello world".into(),
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
                value: "redirected message".into(),
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

    #[test]
    fn local_completion_uses_literal_parser_and_registry_sources() {
        let mut store = WorldStore::new();
        let mut tree = command_tree("choose", "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
            name: "brigadier:bool".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(tree));

        let literal = store.complete_command("/cho");
        assert_eq!((literal.start, literal.length), (1, 3));
        assert_eq!(
            literal
                .suggestions
                .iter()
                .map(|suggestion| suggestion.text.units().to_vec())
                .collect::<Vec<_>>(),
            vec![units_of("choose")]
        );
        assert!(!literal.request_server);

        let boolean = store.complete_command("/choose f");
        assert_eq!((boolean.start, boolean.length), (8, 1));
        assert_eq!(boolean.suggestions[0].text, "false");
        assert!(!boolean.request_server);

        store.record_registry_entries(
            "example:things",
            0,
            vec![
                RegistryPacketEntry::stub("example:first"),
                RegistryPacketEntry::stub("example:second"),
            ],
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
        let mut tree = registry_command_tree(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
            "minecraft:resource_or_tag",
            "example:things",
        );
        tree.nodes[2].suggestions = None;
        assert!(store.apply_commands(tree));
        let registry = store.complete_command("/choose fir");
        assert_eq!(registry.suggestions[0].text, "example:first");
        assert!(!registry.request_server);
        assert_eq!(
            store.complete_command("/choose #").suggestions[0].text,
            "#example:selected"
        );
    }

    #[test]
    fn registered_completion_providers_are_local_and_unknown_providers_ask_server() {
        let mut store = WorldStore::new();
        store.set_available_command_sounds(vec![
            "example:wind".to_string(),
            "minecraft:block.note_block.harp".to_string(),
        ]);
        let mut sounds = command_tree("playsound", "brigadier:string");
        sounds.nodes[2].suggestions = Some("minecraft:available_sounds".to_string());
        assert!(store.apply_commands(sounds));
        let sound = store.complete_command("/playsound wind");
        assert_eq!(sound.suggestions[0].text, "example:wind");
        assert!(!sound.request_server);

        let mut summon = command_tree("summon", "brigadier:string");
        summon.nodes[2].suggestions = Some("minecraft:summonable_entities".to_string());
        assert!(store.apply_commands(summon));
        let summonable = store.complete_command("/summon player");
        assert!(summonable.suggestions.is_empty());
        assert!(!summonable.request_server);
        let lightning_completion = store.complete_command("/summon lightning");
        let lightning = lightning_completion
            .suggestions
            .iter()
            .find(|suggestion| suggestion.text == "minecraft:lightning_bolt")
            .expect("lightning bolt is summonable");
        assert_eq!(
            lightning.tooltip.as_deref(),
            Some("minecraft:lightning_bolt")
        );
        assert_eq!(
            lightning.tooltip_translation_key.as_deref(),
            Some("entity.minecraft.lightning_bolt")
        );
        assert!(store
            .complete_command("/summon minecraft:lightning_bolt")
            .suggestions
            .is_empty());
        store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec!["minecraft:trade_rebalance".to_string()],
        });
        assert!(store
            .complete_command("/summon lightning")
            .suggestions
            .is_empty());

        let mut unknown = command_tree("custom", "brigadier:string");
        unknown.nodes[2].suggestions = Some("example:remote".to_string());
        assert!(store.apply_commands(unknown));
        assert!(store.complete_command("/custom value").request_server);
    }

    #[test]
    fn client_server_fallback_and_item_component_completion_match_parser_owners() {
        let mut store = WorldStore::new();
        let mut objective = command_tree("score", "brigadier:string");
        objective.nodes[2].suggestions = None;
        objective.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID,
            name: "minecraft:objective".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(objective));
        assert!(store.complete_command("/score kills").request_server);

        let mut item = command_tree("give", "brigadier:string");
        item.nodes[2].suggestions = None;
        item.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
            name: "minecraft:item_stack".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(item));
        let item = store.complete_command("/give minecraft:stone");
        assert!(item
            .suggestions
            .iter()
            .any(|suggestion| suggestion.text == "minecraft:stone["));
        assert!(!item.request_server);
    }

    #[test]
    fn selector_completion_tracks_nested_option_applicability_and_value_providers() {
        let mut store = WorldStore::new();
        let mut tree = command_tree("target", "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
            name: "minecraft:entity".to_string(),
            properties: vec![0],
        });
        assert!(store.apply_commands(tree));

        assert_completion_contains(&store, "/target @e[so", "@e[sort=");
        assert_completion_contains(&store, "/target @e[sort=n", "@e[sort=nearest");
        assert_completion_contains(&store, "/target @e[sort=nearest", "@e[sort=nearest]");
        assert_completion_contains(&store, "/target @e[type=!mine", "@e[type=!minecraft:zombie");
        let repeated = store.complete_command("/target @e[type=!minecraft:zombie,type=");
        assert!(repeated
            .suggestions
            .iter()
            .all(|suggestion| text_contains(suggestion, "type=!")));
        let current = store.complete_command("/target @s[");
        assert!(current.suggestions.iter().all(
            |suggestion| !text_is(suggestion, "@s[limit=") && !text_is(suggestion, "@s[sort=")
        ));
        let players = store.complete_command("/target @p[");
        assert!(players
            .suggestions
            .iter()
            .all(|suggestion| suggestion.text != "@p[type="));
    }

    #[test]
    fn block_completion_uses_exact_state_and_active_tag_property_values() {
        let mut store = WorldStore::new();
        let mut tree = command_tree("block", "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
            name: "minecraft:block_state".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(tree));

        assert_completion_contains(&store, "/block oak_log[", "oak_log[axis=");
        assert_completion_contains(&store, "/block oak_log[axis=", "oak_log[axis=x");
        assert_completion_contains(&store, "/block oak_log[axis=x", "oak_log[axis=x]");
        assert_completion_contains(&store, "/block chest", "chest{");

        let oak_log =
            bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:oak_log")
                .unwrap()
                .id;
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![oak_log],
                }],
            }],
        });
        let mut tree = command_tree("match", "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
            name: "minecraft:block_predicate".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(tree));
        assert_completion_contains(&store, "/match #logs[", "#logs[axis=");
        assert_completion_contains(&store, "/match #logs[axis=", "#logs[axis=z");
    }

    #[test]
    fn item_completion_follows_component_assignment_removal_and_delimiter_states() {
        let mut store = WorldStore::new();
        let mut tree = command_tree("give", "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
            name: "minecraft:item_stack".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(tree));

        assert_completion_contains(
            &store,
            "/give stone[max_stack",
            "stone[minecraft:max_stack_size=",
        );
        assert_completion_contains(&store, "/give stone[!custom_name", "stone[!custom_name]");
        let value_start = store.complete_command("/give stone[max_stack_size=");
        assert!(value_start.suggestions.is_empty());
        assert_completion_contains(
            &store,
            "/give stone[max_stack_size=16",
            "stone[max_stack_size=16]",
        );
        assert_completion_contains(
            &store,
            "/give stone[max_stack_size=16,cu",
            "stone[max_stack_size=16,minecraft:custom_data=",
        );

        let mut predicate = command_tree("clear", "brigadier:string");
        predicate.nodes[2].suggestions = None;
        predicate.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
            name: "minecraft:item_predicate".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(predicate));
        assert_completion_contains(&store, "/clear *[max_stack", "*[minecraft:max_stack_size");
        assert_completion_contains(&store, "/clear *[max_stack_size", "*[max_stack_size=");
        assert_completion_contains(&store, "/clear *[count~{min:1}", "*[count~{min:1}]");
        assert_completion_contains(
            &store,
            "/clear *[count~{min:1}|da",
            "*[count~{min:1}|minecraft:damage",
        );
    }

    #[test]
    fn block_state_completion_replaces_a_value_the_definition_rejects() {
        // `BlockStateParser.setValue` (:490-499) rewinds the reader to the value
        // start when the definition rejects the value, so the live suggestion is
        // `addSuggestions` (:259-269) over every value of the property and its
        // range covers the rest of the buffer: `chest[facing=u` answers with the
        // four horizontal facings and `oak_log[axis=q` with `x`, `y` and `z`. A
        // value the definition accepts leaves `suggestNextPropertyOrEnd`
        // (:247-257) after the value instead, where the buffer its candidate
        // would repeat is dropped. Every expectation below is the pinned
        // client-26.1 jar's `fillSuggestions` answer for that buffer.
        let store = completion_store(
            "block",
            VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
            Vec::new(),
        );
        assert_completion_texts(
            &store,
            "/block chest[facing=u",
            &[
                "chest[facing=east",
                "chest[facing=north",
                "chest[facing=south",
                "chest[facing=west",
            ],
        );
        assert_completion_texts(
            &store,
            "/block oak_log[axis=q",
            &["oak_log[axis=x", "oak_log[axis=y", "oak_log[axis=z"],
        );
        assert_completion_texts(
            &store,
            "/block chest[waterlogged=tru",
            &["chest[waterlogged=false", "chest[waterlogged=true"],
        );
        // ASCII control: a value the property itself lists is not offered back.
        assert_completion_texts(&store, "/block oak_log[axis=x", &["oak_log[axis=x]"]);

        // ASCII control: the same buffer with a plain value is answered the same
        // way, so nothing the scan repeated -- units or not -- reaches a
        // candidate text; the surrogate input below answers that same list.
        let facings = [
            "chest[facing=east",
            "chest[facing=north",
            "chest[facing=south",
            "chest[facing=west",
        ];
        assert_completion_texts(&store, "/block chest[facing=\"x\",", &facings);
        let mut tail = command_units("chest[facing=\"", &[0xd800]);
        tail.extend(units_of("\","));
        let completion = store.complete_command_utf16(&command_units("/block ", &tail));
        assert_eq!(
            completion
                .suggestions
                .iter()
                .map(|suggestion| suggestion.text.clone())
                .collect::<Vec<_>>(),
            facings,
            "{completion:?}"
        );
    }

    #[test]
    fn block_state_completion_stops_at_a_value_the_definition_rejects() {
        // `readProperties` (:408-411) reads the value before it validates it, so
        // a value `setValue` then rejects leaves the reader at the value start:
        // the property-name layer after a `,` is never reached and the rejected
        // text is part of what the candidate replaces.
        let store = completion_store(
            "block",
            VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
            Vec::new(),
        );
        assert_completion_texts(
            &store,
            "/block chest[facing=up,",
            &[
                "chest[facing=east",
                "chest[facing=north",
                "chest[facing=south",
                "chest[facing=west",
            ],
        );
        assert_completion_texts(
            &store,
            "/block chest[waterlogged=maybe,",
            &["chest[waterlogged=false", "chest[waterlogged=true"],
        );
        assert_completion_texts(
            &store,
            "/block chest[waterlogged=maybe]{",
            &["chest[waterlogged=false", "chest[waterlogged=true"],
        );

        // A value the property lists is consumed, and the name state a `,`
        // moves to (:422) offers names only -- no `]`.
        assert_completion_texts(
            &store,
            "/block chest[waterlogged=true",
            &["chest[waterlogged=true,", "chest[waterlogged=true]"],
        );
        assert_completion_texts(
            &store,
            "/block chest[waterlogged=true,",
            &[
                "chest[waterlogged=true,facing=",
                "chest[waterlogged=true,type=",
            ],
        );
        assert_completion_texts(&store, "/block oak_log[axis=x,", &[]);
    }

    #[test]
    fn block_state_completion_offers_ids_only_while_the_tail_has_no_id() {
        // `parse` (:158-159) replaces the id list with
        // `suggestOpenPropertiesOrNbt` (:326-338) as soon as `readBlock` resolves
        // the whole tail; only the two error paths rewind to the id start and
        // leave `suggestItem` (:344-346) live. The pinned jar answers
        // `/block chest` with `chest[` and `chest{` alone -- not the eleven
        // registry ids that spell `chest` -- and an id the definition gives
        // neither a property nor a block entity (:327-334) with nothing.
        let store = completion_store(
            "block",
            VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
            Vec::new(),
        );
        assert_completion_texts(&store, "/block chest", &["chest[", "chest{"]);
        assert_completion_texts(
            &store,
            "/block minecraft:chest",
            &["minecraft:chest[", "minecraft:chest{"],
        );
        assert_completion_texts(&store, "/block oak_log", &["oak_log["]);
        assert_completion_texts(&store, "/block stone", &[]);

        // ASCII control: a tail whose own id does not resolve still reaches the
        // list, and one no registry id spells answers nothing.
        assert_completion_texts(
            &store,
            "/block oak_lo",
            &[
                "minecraft:dark_oak_log",
                "minecraft:oak_log",
                "minecraft:pale_oak_log",
                "minecraft:stripped_dark_oak_log",
                "minecraft:stripped_oak_log",
                "minecraft:stripped_pale_oak_log",
            ],
        );
        assert_completion_texts(&store, "/block chestt", &[]);
    }

    #[test]
    fn block_state_completion_accepts_every_spelling_of_a_listed_integer_value() {
        // `setValue` (:490-499) asks the definition's own `Property.getValue`,
        // and `IntegerProperty.getValue` (`IntegerProperty.java:52-59`) parses
        // the value as an `int` inside the range the definition lists, so
        // `stage=00`, `+0` and `-0` are the value `stage=0` the buffer never
        // spells. An accepted value leaves `suggestNextPropertyOrEnd`
        // (:247-257), and the reader carries on with the next property.
        let store = completion_store(
            "block",
            VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
            Vec::new(),
        );
        for (typed, answered) in [
            ("/block oak_sapling[stage=00", "oak_sapling[stage=00]"),
            ("/block oak_sapling[stage=+0", "oak_sapling[stage=+0]"),
            ("/block oak_sapling[stage=-0", "oak_sapling[stage=-0]"),
            (
                "/block oak_sapling[stage=00000000000000001",
                "oak_sapling[stage=00000000000000001]",
            ),
            (
                "/block oak_sapling[stage=\"00\"",
                "oak_sapling[stage=\"00\"]",
            ),
            ("/block wheat[age=00", "wheat[age=00]"),
            ("/block lava[level=00", "lava[level=00]"),
            // `Integer.parseInt` reads `Character.digit`, so a quoted
            // Arabic-Indic or fullwidth digit is the value it spells; the range
            // the definition lists still decides.
            (
                "/block oak_sapling[stage=\"\u{0660}\"",
                "oak_sapling[stage=\"\u{0660}\"]",
            ),
            (
                "/block oak_sapling[stage=\"\u{0661}\"",
                "oak_sapling[stage=\"\u{0661}\"]",
            ),
            (
                "/block oak_sapling[stage=\"\u{ff10}\"",
                "oak_sapling[stage=\"\u{ff10}\"]",
            ),
            ("/block wheat[age=\"\u{0660}\"", "wheat[age=\"\u{0660}\"]"),
            ("/block lava[level=\"\u{ff11}\"", "lava[level=\"\u{ff11}\"]"),
        ] {
            assert_completion_texts(&store, typed, &[answered]);
        }
        assert_completion_texts(
            &store,
            "/block cocoa[age=00",
            &["cocoa[age=00,", "cocoa[age=00]"],
        );
        assert_completion_texts(
            &store,
            "/block cocoa[age=00,facing=",
            &[
                "cocoa[age=00,facing=east",
                "cocoa[age=00,facing=north",
                "cocoa[age=00,facing=south",
                "cocoa[age=00,facing=west",
            ],
        );

        // ASCII control: a spelling the definition does not list and cannot
        // parse as an integer is still rejected, as is an integer outside the
        // range the definition lists.
        assert_completion_texts(
            &store,
            "/block chest[waterlogged=TRUE",
            &["chest[waterlogged=false", "chest[waterlogged=true"],
        );
        assert_completion_texts(
            &store,
            "/block oak_sapling[stage=0",
            &["oak_sapling[stage=0]"],
        );
        for rejected in [
            "/block oak_sapling[stage=7",
            "/block oak_sapling[stage=007",
            "/block oak_sapling[stage=2147483648",
            "/block oak_sapling[stage=0x0",
            "/block oak_sapling[stage=0_0",
            "/block oak_sapling[stage=+",
            "/block oak_sapling[stage=-",
        ] {
            assert_completion_texts(
                &store,
                rejected,
                &["oak_sapling[stage=0", "oak_sapling[stage=1"],
            );
        }
    }

    #[test]
    fn block_predicate_completion_offers_ids_only_while_the_tag_has_no_id() {
        // `readTag` (:365-378) resolves the whole tail before `parse` moves to
        // `suggestOpenVaguePropertiesOrNbt` (:300-324), so a tag that resolves
        // answers only the property delimiter; an unfinished tag still reaches
        // `suggestTag` (:340-342) with the registry's own ids.
        let mut store = WorldStore::new();
        let oak_log =
            bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:oak_log")
                .unwrap()
                .id;
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![oak_log],
                }],
            }],
        });
        let mut tree = command_tree("match", "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
            name: "minecraft:block_predicate".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(tree));

        assert_completion_texts(&store, "/match #minecraft:logs", &["#minecraft:logs["]);
        assert_completion_texts(&store, "/match #minecraft:log", &["#minecraft:logs"]);
        assert_completion_texts(&store, "/match #minecraft:logs_", &[]);
    }

    fn store_with_tags(store: &mut WorldStore, registry: &str, tags: &[&str]) {
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: registry.to_string(),
                tags: tags
                    .iter()
                    .map(|tag| TagNetworkPayload {
                        tag: (*tag).to_string(),
                        entries: Vec::new(),
                    })
                    .collect(),
            }],
        });
    }

    /// The four `minecraft:entity_type` tags the pinned client-26.1 jar binds at
    /// bootstrap, so the selector counts below are the jar's own.
    const JAR_ENTITY_TYPE_TAGS: [&str; 4] = [
        "minecraft:can_equip_harness",
        "minecraft:can_equip_saddle",
        "minecraft:can_wear_horse_armor",
        "minecraft:can_wear_nautilus_armor",
    ];

    /// `SharedSuggestionProvider.filterResources` (SharedSuggestionProvider.java:109-121)
    /// never asks the tail to carry the whole prefix, so the `type` option's
    /// four `suggestResource` calls (`EntitySelectorOptions.java:265-274`) all
    /// answer an empty tail, and `!` reaches the `!#` table as well as the `!`
    /// one.
    #[test]
    fn selector_type_candidates_reach_the_tag_table_through_the_common_prefix() {
        let mut store =
            completion_store("target", VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, vec![0]);
        store_with_tags(&mut store, "minecraft:entity_type", &JAR_ENTITY_TYPE_TAGS);
        for (input, count, present) in [
            ("@e[type=", 322, "@e[type=!#minecraft:can_equip_saddle"),
            ("@e[type=!", 161, "@e[type=!#minecraft:can_equip_saddle"),
            (
                "@e[type=!minecraft:zombie,type=",
                161,
                "@e[type=!minecraft:zombie,type=!#minecraft:can_equip_saddle",
            ),
        ] {
            let texts = completion_texts(&store, &format!("/target {input}"));
            assert_eq!(texts.len(), count, "{input}");
            assert!(
                texts.contains(&bbb_protocol::JavaString::from(present)),
                "{input} missing {present}"
            );
        }
        // The uninverted tables are gone once `type=!` limited the option
        // inversely, and `#` alone shares nothing with `!` or `!#`.
        assert_completion_texts(&store, "/target @e[type=!minecraft:zombie,type=#", &[]);
    }

    /// The same rule at the `#` call site: `suggestRegistryElements`
    /// (SharedSuggestionProvider.java:63-73) passes the tag table through
    /// `suggestResource(.., "#")` and the element table through the overload
    /// with no prefix.
    #[test]
    fn block_predicate_tag_candidates_follow_the_same_prefix_filter() {
        let tags = [
            "minecraft:leaves",
            "minecraft:mineable/axe",
            "minecraft:mineable/hoe",
            "minecraft:wool",
        ];
        let mut store = completion_store(
            "match",
            VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
            Vec::new(),
        );
        store_with_tags(&mut store, "minecraft:block", &tags);
        assert_completion_texts(
            &store,
            "/match #",
            &[
                "#minecraft:leaves",
                "#minecraft:mineable/axe",
                "#minecraft:mineable/hoe",
                "#minecraft:wool",
            ],
        );
        assert_completion_texts(
            &store,
            "/match #mineable",
            &["#minecraft:mineable/axe", "#minecraft:mineable/hoe"],
        );
        assert_completion_texts(&store, "/match ##", &[]);
        // A tail with a namespace reaches the tag table only; no block id
        // starts with `#`.
        assert_completion_texts(&store, "/match #minecraft:w", &["#minecraft:wool"]);
    }

    /// The 15 `minecraft:block` tags the pinned client-26.1 jar binds at
    /// bootstrap, so the `/match ` count below is the jar's own.
    const JAR_BLOCK_TAGS: [&str; 15] = [
        "minecraft:incorrect_for_copper_tool",
        "minecraft:incorrect_for_diamond_tool",
        "minecraft:incorrect_for_gold_tool",
        "minecraft:incorrect_for_iron_tool",
        "minecraft:incorrect_for_netherite_tool",
        "minecraft:incorrect_for_stone_tool",
        "minecraft:incorrect_for_wooden_tool",
        "minecraft:leaves",
        "minecraft:mineable/axe",
        "minecraft:mineable/hoe",
        "minecraft:mineable/pickaxe",
        "minecraft:mineable/shovel",
        "minecraft:sword_efficient",
        "minecraft:sword_instantly_mines",
        "minecraft:wool",
    ];

    fn add_online_players(store: &mut WorldStore, names: &[&str]) {
        for (index, name) in names.iter().enumerate() {
            let uuid = uuid::Uuid::from_u128(index as u128 + 1);
            store.player_info.entries.insert(
                uuid,
                crate::PlayerInfoEntryState {
                    profile: crate::PlayerInfoProfileState {
                        uuid,
                        name: (*name).to_string(),
                        properties: Vec::new(),
                    },
                    listed: true,
                    latency: 0,
                    game_mode: "survival".to_string(),
                    display_name: None,
                    show_hat: false,
                    list_order: 0,
                    chat_session_present: false,
                    chat_session: None,
                },
            );
        }
    }

    /// `CommandContextBuilder.findSuggestionContext` (brigadier 1.3.10,
    /// `CommandContextBuilder.java:129-137`) leaves the parsed node behind once
    /// the cursor is past its range: the parent becomes that node and the start
    /// its end plus one, so a trailing separator asks the node's own children
    /// with an empty tail. The three counts are the pinned client-26.1 jar's
    /// answers for the same buffers.
    #[test]
    fn a_trailing_separator_reaches_the_next_argument_table() {
        let store = completion_store(
            "block",
            VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
            Vec::new(),
        );
        let completion = store.complete_command("/block ");
        assert_eq!((completion.start, completion.length), (7, 0));
        assert_eq!(completion.suggestions.len(), 1168);

        let mut store = completion_store(
            "match",
            VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
            Vec::new(),
        );
        store_with_tags(&mut store, "minecraft:block", &JAR_BLOCK_TAGS);
        let completion = store.complete_command("/match ");
        assert_eq!((completion.start, completion.length), (7, 0));
        assert_eq!(completion.suggestions.len(), 1183);

        let mut store =
            completion_store("target", VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, vec![0]);
        add_online_players(&mut store, &["Alex", "Steve"]);
        assert_completion_texts(
            &store,
            "/target ",
            &["@a", "@e", "@n", "@p", "@r", "@s", "Alex", "Steve"],
        );
    }

    /// The same entry decides the start for every parent kind, so a literal, an
    /// argument and a redirect all advance past their own separator. The
    /// expectations are the pinned brigadier 1.3.10 `findSuggestionContext`
    /// answers for the same tree.
    #[test]
    fn the_completion_context_advances_for_every_parent_kind() {
        let mut store = WorldStore::new();
        let mut tree = redirected_message_command_tree();
        // The deepest node answers from the team table, so the candidates name
        // which node the context chose as the parent.
        tree.nodes[3].suggestions = None;
        tree.nodes[3].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID,
            name: "minecraft:team".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(tree));
        for name in ["alpha", "beta"] {
            store.scoreboard.teams.insert(
                name.to_string(),
                crate::ScoreboardTeam {
                    name: name.to_string(),
                    parameters: None,
                    players: Default::default(),
                },
            );
        }
        add_online_players(&mut store, &["Alex"]);

        // No separator yet: the cursor is still inside the literal's range, so
        // the parent stays the root and the start is where the literal began.
        let completion = store.complete_command("/tel");
        assert_eq!((completion.start, completion.length), (1, 3));
        assert_eq!(completion_texts(&store, "/tel"), ["tell"]);

        // Literal parent.
        let completion = store.complete_command("/tell ");
        assert_eq!((completion.start, completion.length), (6, 0));
        assert_completion_texts(
            &store,
            "/tell ",
            &["@a", "@e", "@n", "@p", "@r", "@s", "Alex"],
        );
        let completion = store.complete_command("/tell @e");
        assert_eq!((completion.start, completion.length), (6, 2));
        assert_completion_texts(&store, "/tell @e", &["@e["]);

        // Argument parent: the separator after `@e` moves the start past it.
        let completion = store.complete_command("/tell @e ");
        assert_eq!((completion.start, completion.length), (9, 0));
        assert_completion_texts(&store, "/tell @e ", &["alpha", "beta"]);
        let completion = store.complete_command("/tell @e al");
        assert_eq!((completion.start, completion.length), (9, 2));
        assert_completion_texts(&store, "/tell @e al", &["alpha"]);

        // Redirect parent: the separator after `w` reaches the redirect
        // target's children, and the argument below it advances the same way.
        let completion = store.complete_command("/w ");
        assert_eq!((completion.start, completion.length), (3, 0));
        assert_completion_texts(&store, "/w ", &["@a", "@e", "@n", "@p", "@r", "@s", "Alex"]);
        let completion = store.complete_command("/w @e ");
        assert_eq!((completion.start, completion.length), (6, 0));
        assert_completion_texts(&store, "/w @e ", &["alpha", "beta"]);
        let completion = store.complete_command("/w @e be");
        assert_eq!((completion.start, completion.length), (6, 2));
        assert_completion_texts(&store, "/w @e be", &["beta"]);

        // A childless literal answers nothing, but the start still advances.
        let completion = store.complete_command("/time ");
        assert_eq!((completion.start, completion.length), (6, 0));
        assert!(completion.suggestions.is_empty());
    }

    /// What the pinned client-26.1 jar's `EntitySelectorParser.fillSuggestions`
    /// (`EntitySelectorParser.java:541-543`) applied for one buffer. `Options`
    /// names the keys the option-name layer left out; every other key is
    /// answered as the buffer plus that key and `=`.
    enum JarSuggestions {
        Empty,
        Texts(&'static [&'static str]),
        Options(&'static [&'static str]),
    }

    /// Every option value syntax replayed against the jar: the four readers
    /// (`readString`, `readInt`, `readDouble`, `readUnquotedString`), the ten
    /// `MinMaxBounds` bounds, the `TagParser` compound, the `scores`,
    /// `advancements` and `predicate` structures and the by-name and registry
    /// lookups. A value its own modifier rejects stops the parse on that
    /// modifier's provider (`EntitySelectorOptions.java:107-117`, `:165-175`,
    /// `:176-192`), so the option-name layer is never reached and the
    /// availability the segment would have set is never applied.
    #[test]
    fn selector_option_value_replay_matches_the_pinned_jar() {
        use JarSuggestions::{Empty, Options, Texts};
        let store = completion_store("target", VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, vec![0]);
        for (input, expected) in [
            ("@e[sort=zzz,", Empty),
            ("@e[limit=0,", Empty),
            ("@e[gamemode=zzz,", Empty),
            ("@e[type=minecraft:zzz,", Empty),
            ("@e[distance=zzz,", Empty),
            ("@e[", Options(&[])),
            ("@e[sort", Texts(&["@e[sort="])),
            ("@e[ sort", Texts(&["@e[ sort="])),
            ("@e[zzz", Empty),
            ("@e[zzz=", Empty),
            ("@e[,", Empty),
            ("@e[\"sort\"=nearest,", Options(&["sort"])),
            ("@e[SORT=nearest,", Empty),
            ("@e[limit=1,limit", Options(&["limit"])),
            (
                "@e[sort=nearest",
                Texts(&["@e[sort=nearest,", "@e[sort=nearest]"]),
            ),
            ("@e[sort=nearest,", Options(&["sort"])),
            ("@e[sort=nearest]", Empty),
            ("@e[sort=nearest,]", Empty),
            (
                "@e[sort=nearest x",
                Texts(&["@e[sort=nearest ,", "@e[sort=nearest ]"]),
            ),
            ("@e[sort=nearest ,", Options(&["sort"])),
            ("@e[ sort = nearest ,", Options(&["sort"])),
            ("@e[sort=nearest,,", Empty),
            ("@e[name=,", Options(&["name"])),
            ("@e[name=\"a,", Empty),
            ("@e[name=!zzz,", Options(&[])),
            ("@e[name=!zzz,name=", Empty),
            ("@e[name=zzz,name=", Empty),
            ("@e[name=\"]\",", Options(&["name"])),
            ("@e[name=\"a,b\",", Options(&["name"])),
            ("@e[name='a',", Options(&["name"])),
            ("@e[distance=1,", Options(&["distance"])),
            ("@e[distance=-1,", Empty),
            ("@e[distance=5..1,", Empty),
            ("@e[distance=..,", Empty),
            ("@e[distance=-0,", Options(&["distance"])),
            ("@e[distance=..5,", Options(&["distance"])),
            ("@e[level=1,", Options(&["level"])),
            ("@e[level=-1,", Empty),
            ("@e[level=5..1,", Empty),
            ("@e[level=1.5,", Empty),
            ("@e[level=..,", Empty),
            ("@e[level=00,", Options(&["level"])),
            ("@e[x=1,", Options(&["x"])),
            ("@e[x=zzz,", Empty),
            ("@e[x=1.5,", Options(&["x"])),
            ("@e[dx=,", Empty),
            ("@e[x=-,", Empty),
            ("@e[x=.,", Empty),
            ("@e[dz=zzz,", Empty),
            ("@e[x_rotation=5..1,", Options(&["x_rotation"])),
            ("@e[x_rotation=zzz,", Empty),
            ("@e[y_rotation=5..1,", Options(&["y_rotation"])),
            ("@e[y_rotation=,", Empty),
            ("@e[x_rotation=1..,", Options(&["x_rotation"])),
            (
                "@e[x_rotation=1..2..3,",
                Texts(&["@e[x_rotation=1..2,", "@e[x_rotation=1..2]"]),
            ),
            ("@e[limit=1,", Options(&["limit"])),
            ("@e[limit=-1,", Empty),
            ("@e[limit=1.5,", Empty),
            ("@e[limit=zzz,", Empty),
            ("@e[limit=2147483648,", Empty),
            ("@e[limit=01,", Options(&["limit"])),
            (
                "@e[sort=",
                Texts(&[
                    "@e[sort=arbitrary",
                    "@e[sort=furthest",
                    "@e[sort=nearest",
                    "@e[sort=random",
                ]),
            ),
            ("@e[sort=n", Texts(&["@e[sort=nearest"])),
            ("@e[sort=nea,", Empty),
            (
                "@e[gamemode=",
                Texts(&[
                    "@e[gamemode=!adventure",
                    "@e[gamemode=!creative",
                    "@e[gamemode=!spectator",
                    "@e[gamemode=!survival",
                    "@e[gamemode=adventure",
                    "@e[gamemode=creative",
                    "@e[gamemode=spectator",
                    "@e[gamemode=survival",
                ]),
            ),
            (
                "@e[gamemode=!s",
                Texts(&["@e[gamemode=!spectator", "@e[gamemode=!survival"]),
            ),
            ("@e[gamemode=survival,", Options(&["gamemode"])),
            ("@e[gamemode=!survival,", Options(&[])),
            (
                "@e[gamemode=!survival,gamemode=",
                Texts(&[
                    "@e[gamemode=!survival,gamemode=!adventure",
                    "@e[gamemode=!survival,gamemode=!creative",
                    "@e[gamemode=!survival,gamemode=!spectator",
                    "@e[gamemode=!survival,gamemode=!survival",
                ]),
            ),
            ("@e[gamemode=!survival,gamemode=survival,", Empty),
            ("@e[team=zzz,", Options(&["team"])),
            ("@e[team=,", Options(&["team"])),
            (
                "@e[team=!zzz,team=",
                Texts(&["@e[team=!zzz,team=,", "@e[team=!zzz,team=]"]),
            ),
            ("@e[type=minecraft:zombie,", Options(&["type"])),
            ("@e[type=zombie,", Options(&["type"])),
            ("@e[type=#minecraft:zzz,", Options(&[])),
            ("@e[type=!minecraft:zzz,", Empty),
            ("@e[type=!zzz,", Empty),
            ("@e[type=#,", Options(&[])),
            ("@e[type=,", Empty),
            ("@e[type=!#,", Options(&[])),
            ("@e[type=minecraft:player,", Options(&["type"])),
            (
                "@e[type=!minecraft:zombie,type=!zom",
                Texts(&[
                    "@e[type=!minecraft:zombie,type=!minecraft:zombie",
                    "@e[type=!minecraft:zombie,type=!minecraft:zombie_horse",
                    "@e[type=!minecraft:zombie,type=!minecraft:zombie_nautilus",
                    "@e[type=!minecraft:zombie,type=!minecraft:zombie_villager",
                    "@e[type=!minecraft:zombie,type=!minecraft:zombified_piglin",
                ]),
            ),
            ("@e[type=!minecraft:zombie,type=m", Empty),
            ("@e[tag=zzz,", Options(&[])),
            ("@e[tag=,", Options(&[])),
            ("@e[tag=!,", Options(&[])),
            ("@e[nbt={},", Options(&[])),
            ("@e[nbt={a:1},", Options(&[])),
            ("@e[nbt=zzz,", Empty),
            ("@e[nbt={,", Empty),
            ("@e[nbt=[],", Empty),
            ("@e[nbt=!{},", Options(&[])),
            ("@e[nbt={a:\"}\"},", Options(&[])),
            ("@e[nbt={a:{b:1}},", Options(&[])),
            ("@e[scores={},", Options(&["scores"])),
            ("@e[scores={a=1},", Options(&["scores"])),
            ("@e[scores={a=zzz},", Empty),
            ("@e[scores=zzz,", Empty),
            ("@e[scores={a=1", Empty),
            ("@e[scores={ a = 1 },", Options(&["scores"])),
            ("@e[scores={a=1,b=2},", Options(&["scores"])),
            ("@e[scores={a=1,},", Options(&["scores"])),
            ("@e[scores={a=1..2},", Options(&["scores"])),
            ("@e[advancements={},", Options(&["advancements"])),
            (
                "@e[advancements={minecraft:story/root=true},",
                Options(&["advancements"]),
            ),
            (
                "@e[advancements={minecraft:story/root={a=true}},",
                Options(&["advancements"]),
            ),
            ("@e[advancements={zzz},", Empty),
            ("@e[advancements={minecraft:story/root=zzz},", Empty),
            ("@e[advancements={ },", Options(&["advancements"])),
            (
                "@e[advancements={minecraft:a={b=true,c=false}},",
                Options(&["advancements"]),
            ),
            (
                "@e[advancements={minecraft:a=true,},",
                Options(&["advancements"]),
            ),
            ("@e[predicate=minecraft:zzz,", Options(&[])),
            ("@e[predicate=zzz,", Options(&[])),
            ("@e[predicate=,", Options(&[])),
            (
                "@e[predicate=#zzz,",
                Texts(&["@e[predicate=,", "@e[predicate=]"]),
            ),
            ("@e[predicate=!minecraft:zzz,", Options(&[])),
            ("@s[sort=nearest,", Empty),
            ("@a[type=minecraft:zombie,", Empty),
            ("@s[limit=", Empty),
            ("@a[sort=nearest,", Options(&["sort", "type"])),
            ("@p[type=", Empty),
            ("@e[tag=a,tag=b,", Options(&[])),
            ("@e[x=1,y=2,z=3,", Options(&["x", "y", "z"])),
            ("@e[sort=zzz,limit=1,tag=", Empty),
            ("@e[distance=1,distance=", Empty),
            ("@e[level=0,level=", Empty),
            (
                "@e[sort=arbitrary,limit=1,tag=x,nbt={},predicate=a,",
                Options(&["limit", "sort"]),
            ),
            ("@e[name=a,tag=b,team=c,", Options(&["name", "team"])),
            ("@e[nbt={},nbt={},", Options(&[])),
            (
                "@e[predicate=minecraft:a,predicate=minecraft:b,",
                Options(&[]),
            ),
            ("@e[tag=,tag=,", Options(&[])),
            ("@e[y_rotation=0,y_rotation=", Empty),
            ("@e[scores={a=1},scores=", Empty),
            (
                "@e[advancements={minecraft:story/root=true},advancements=",
                Empty,
            ),
            (
                "@e[distance=..5,x_rotation=-90..90,",
                Options(&["distance", "x_rotation"]),
            ),
            ("@e[name=!a,name=!b,", Options(&[])),
            ("@e[team=!a,team=b,", Options(&["team"])),
            (
                "@e[type=#minecraft:zzz,type=minecraft:zombie,",
                Options(&["type"]),
            ),
            (
                "@e[gamemode=survival,type=zom",
                Texts(&[
                    "@e[gamemode=survival,type=minecraft:zombie",
                    "@e[gamemode=survival,type=minecraft:zombie_horse",
                    "@e[gamemode=survival,type=minecraft:zombie_nautilus",
                    "@e[gamemode=survival,type=minecraft:zombie_villager",
                    "@e[gamemode=survival,type=minecraft:zombified_piglin",
                ]),
            ),
            (
                "@e[nbt={a:1},sort=nea",
                Texts(&["@e[nbt={a:1},sort=nearest"]),
            ),
            (
                "@e[scores={a=1},gamemode=!s",
                Texts(&[
                    "@e[scores={a=1},gamemode=!spectator",
                    "@e[scores={a=1},gamemode=!survival",
                ]),
            ),
            (
                "@e[advancements={minecraft:story/root=true},gamemode=!",
                Texts(&[
                    "@e[advancements={minecraft:story/root=true},gamemode=!adventure",
                    "@e[advancements={minecraft:story/root=true},gamemode=!creative",
                    "@e[advancements={minecraft:story/root=true},gamemode=!spectator",
                    "@e[advancements={minecraft:story/root=true},gamemode=!survival",
                ]),
            ),
            (
                "@e[level=0..5,limit=3,sort=",
                Texts(&[
                    "@e[level=0..5,limit=3,sort=arbitrary",
                    "@e[level=0..5,limit=3,sort=furthest",
                    "@e[level=0..5,limit=3,sort=nearest",
                    "@e[level=0..5,limit=3,sort=random",
                ]),
            ),
            ("@e[nbt={},]", Empty),
            ("@e[x=1]", Empty),
            ("@e[type=minecraft:zombie]", Empty),
            ("@e[\"sort", Options(&[])),
            ("@e[\"sort\"", Empty),
            ("@e[name=\"a", Empty),
            ("@e[name=\"\\q\",", Empty),
            ("@e[nbt={a:1", Empty),
            ("@e[x_rotation=-0,", Options(&["x_rotation"])),
            ("@e[distance=0..,", Options(&["distance"])),
            ("@e[level=..0,", Options(&["level"])),
            ("@e[limit=+1,", Empty),
            ("@e[x=+1,", Empty),
            ("@e[sort=Nearest", Texts(&["@e[sort=nearest"])),
            ("@e[GAMEMODE=", Empty),
            ("@e[scores={\"a\"=1},", Empty),
            ("@e[advancements={minecraft:a={\"b\"=true}},", Empty),
            (
                "@e[type=!minecraft:zombie,tag=a,type=!zom",
                Texts(&[
                    "@e[type=!minecraft:zombie,tag=a,type=!minecraft:zombie",
                    "@e[type=!minecraft:zombie,tag=a,type=!minecraft:zombie_horse",
                    "@e[type=!minecraft:zombie,tag=a,type=!minecraft:zombie_nautilus",
                    "@e[type=!minecraft:zombie,tag=a,type=!minecraft:zombie_villager",
                    "@e[type=!minecraft:zombie,tag=a,type=!minecraft:zombified_piglin",
                ]),
            ),
        ] {
            let mut expected = match expected {
                Empty => Vec::new(),
                Texts(texts) => texts
                    .iter()
                    .copied()
                    .map(bbb_protocol::JavaString::from)
                    .collect(),
                Options(excluded) => bbb_protocol::VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS
                    .iter()
                    .filter(|option| !excluded.contains(&option.name))
                    .map(|option| {
                        bbb_protocol::JavaString::from(format!("{input}{}=", option.name))
                    })
                    .collect(),
            };
            expected.sort();
            assert_eq!(
                completion_texts(&store, &format!("/target {input}")),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn selector_completion_lowercases_option_names_on_both_sides() {
        // `EntitySelectorOptions.suggestNames` (:501-509) compares the option's
        // lowercased name with the lowercased remainder while the option lookup
        // itself stays case-sensitive (:487-499), and the live function at the
        // key position is `suggestOptionsKey` (:286, :517-520): names only.
        let store = completion_store("target", VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, vec![0]);
        assert_completion_texts(&store, "/target @e[SORT", &["@e[sort="]);
        assert_completion_texts(&store, "/target @e[Sort", &["@e[sort="]);
        assert_completion_texts(&store, "/target @e[so", &["@e[sort="]);
        assert_completion_texts(&store, "/target @e[SORT=", &[]);
        let texts = completion_texts(&store, "/target @e[");
        assert!(texts.iter().any(|text| text == "@e[sort="), "{texts:?}");
        assert!(!texts.iter().any(|text| text == "@e[]"), "{texts:?}");
    }

    #[test]
    fn selector_completion_ends_a_complete_option_value_with_its_delimiters() {
        // A value the option accepts leaves `parseOptions` (:303-305) on
        // `suggestOptionsNextOrClose` (:522-526), so the value provider is only
        // live while the value still fails, and `SuggestionsBuilder.suggest`
        // drops the candidate that repeats the remainder verbatim.
        let store = completion_store("target", VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, vec![0]);
        assert_completion_texts(
            &store,
            "/target @e[sort=nearest",
            &["@e[sort=nearest,", "@e[sort=nearest]"],
        );
        assert_completion_texts(
            &store,
            "/target @e[type=minecraft:zombie",
            &["@e[type=minecraft:zombie,", "@e[type=minecraft:zombie]"],
        );
        assert_completion_texts(
            &store,
            "/target @e[gamemode=!creative",
            &["@e[gamemode=!creative,", "@e[gamemode=!creative]"],
        );
        // ASCII control: an unfinished value still reaches the provider.
        assert_completion_texts(&store, "/target @e[sort=n", &["@e[sort=nearest"]);
    }

    #[test]
    fn block_predicate_completion_follows_the_vague_value_reader() {
        // `readVagueProperties` (:433-484) never validates a value, so
        // `suggestVaguePropertyValue` (:271-298) answers with every value of the
        // typed key, `,` while the tag still holds an unread property and `]`
        // always; the value the buffer already spells is dropped.
        let mut store = WorldStore::new();
        let oak_log =
            bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:oak_log")
                .unwrap()
                .id;
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![oak_log],
                }],
            }],
        });
        let mut tree = command_tree("match", "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
            name: "minecraft:block_predicate".to_string(),
            properties: Vec::new(),
        });
        assert!(store.apply_commands(tree));

        assert_completion_texts(
            &store,
            "/match #logs[axis=",
            &[
                "#logs[axis=]",
                "#logs[axis=x",
                "#logs[axis=y",
                "#logs[axis=z",
            ],
        );
        assert_completion_texts(
            &store,
            "/match #logs[axis=x",
            &["#logs[axis=]", "#logs[axis=y", "#logs[axis=z"],
        );
        assert_completion_texts(
            &store,
            "/match #logs[axis=q",
            &[
                "#logs[axis=]",
                "#logs[axis=x",
                "#logs[axis=y",
                "#logs[axis=z",
            ],
        );
        // A `,` moves to the name state (:471) with `axis` read, so nothing is
        // left to suggest for this one-block tag.
        assert_completion_texts(&store, "/match #logs[axis=x,", &[]);
        // A key no block of the tag defines has no values, and what remains of
        // the buffer is replaced: `,` because `axis` is still unread, and `]`.
        assert_completion_texts(
            &store,
            "/match #logs[\"x\"=x",
            &["#logs[\"x\"=,", "#logs[\"x\"=]"],
        );
    }

    /// A store whose tree holds one argument of `type_id`, which is what the
    /// completion tests above build by hand.
    fn completion_store(literal: &str, type_id: i32, properties: Vec<u8>) -> WorldStore {
        let mut store = WorldStore::new();
        let mut tree = command_tree(literal, "brigadier:string");
        tree.nodes[2].suggestions = None;
        tree.nodes[2].parser = Some(CommandArgumentParser {
            type_id,
            name: "argument".to_string(),
            properties,
        });
        assert!(store.apply_commands(tree));
        store
    }

    fn completion_texts(store: &WorldStore, input: &str) -> Vec<bbb_protocol::JavaString> {
        store
            .complete_command(input)
            .suggestions
            .into_iter()
            .map(|suggestion| suggestion.text)
            .collect()
    }

    fn assert_completion_texts(store: &WorldStore, input: &str, expected: &[&str]) {
        let completion = store.complete_command(input);
        let texts = completion
            .suggestions
            .iter()
            .map(|suggestion| suggestion.text.clone())
            .collect::<Vec<_>>();
        assert_eq!(texts, expected, "{input}");
        assert!(!completion.request_server, "{input}");
    }

    fn units_of(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    /// A command buffer whose tail holds units a `&str` cannot spell, such as an
    /// unpaired surrogate.
    fn command_units(prefix: &str, tail: &[u16]) -> Vec<u16> {
        let mut units = units_of(prefix);
        units.extend_from_slice(tail);
        units
    }

    fn suggestion_units(completion: &CommandCompletionResult) -> Vec<Vec<u16>> {
        completion
            .suggestions
            .iter()
            .map(|suggestion| suggestion.text.units().to_vec())
            .collect()
    }

    /// Candidate text carries the buffer's own units, so an ASCII expectation
    /// compares through them rather than through a UTF-8 projection.
    fn text_contains(suggestion: &CommandCompletionSuggestion, needle: &str) -> bool {
        let needle = units_of(needle);
        suggestion
            .text
            .units()
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn text_is(suggestion: &CommandCompletionSuggestion, expected: &str) -> bool {
        suggestion.text.units() == units_of(expected)
    }

    fn assert_completion_contains(store: &WorldStore, input: &str, expected: &str) {
        let completion = store.complete_command(input);
        assert!(
            completion
                .suggestions
                .iter()
                .any(|suggestion| suggestion.text == expected),
            "missing {expected:?} for {input:?}: {completion:?}"
        );
        assert!(!completion.request_server);
    }

    #[test]
    fn resource_sound_completion_view_is_publication_shared_and_generation_isolated() {
        let mut store = WorldStore::new();
        store.set_available_command_sounds(vec!["example:first".to_string()]);
        let generation_a = store.publication_snapshot();
        store.set_available_command_sounds(vec!["example:second".to_string()]);

        assert_eq!(
            generation_a.available_command_sounds.as_slice(),
            ["example:first"]
        );
        assert_eq!(
            store.available_command_sounds.as_slice(),
            ["example:second"]
        );
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

    fn literal_command_tree(literals: &[&str]) -> Commands {
        let mut nodes = vec![CommandNode {
            node_type: CommandNodeType::Root,
            flags: 0,
            children: (1..=literals.len() as i32).collect(),
            redirect: None,
            name: None,
            parser: None,
            suggestions: None,
            executable: false,
            restricted: false,
        }];
        nodes.extend(literals.iter().map(|literal| CommandNode {
            node_type: CommandNodeType::Literal,
            flags: 1,
            children: Vec::new(),
            redirect: None,
            name: Some(literal.to_string()),
            parser: None,
            suggestions: None,
            executable: true,
            restricted: false,
        }));
        Commands {
            root_index: 0,
            nodes,
        }
    }

    fn nested_literal_command_tree(literals: &[&str]) -> Commands {
        let mut tree = literal_command_tree(literals);
        tree.nodes[0].children = vec![1];
        for index in 1..literals.len() {
            tree.nodes[index].children = vec![(index + 1) as i32];
        }
        tree
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

    #[test]
    fn command_completion_suggests_looked_at_block_coords_and_targeted_entity_uuid() {
        let mut store = WorldStore::new();
        let targeted_uuid = uuid::Uuid::from_u128(0x11223344_5566_4777_8888_99aabbccddee);
        let tree = Commands {
            root_index: 0,
            nodes: vec![
                CommandNode {
                    node_type: CommandNodeType::Root,
                    flags: 0,
                    children: vec![1, 3, 5, 7, 9, 11],
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
                    name: Some("setblock".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("pos".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
                        name: "minecraft:block_pos".to_string(),
                        properties: Vec::new(),
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![4],
                    redirect: None,
                    name: Some("tp".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("loc".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID,
                        name: "minecraft:vec3".to_string(),
                        properties: Vec::new(),
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![6],
                    redirect: None,
                    name: Some("forceload".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("col".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID,
                        name: "minecraft:column_pos".to_string(),
                        properties: Vec::new(),
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![8],
                    redirect: None,
                    name: Some("spread".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("center".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID,
                        name: "minecraft:vec2".to_string(),
                        properties: Vec::new(),
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![10],
                    redirect: None,
                    name: Some("kill".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("target".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
                        name: "minecraft:entity".to_string(),
                        properties: vec![0],
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![12],
                    redirect: None,
                    name: Some("kick".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("player".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
                        name: "minecraft:entity".to_string(),
                        // FLAG_PLAYERS_ONLY (0x02)
                        properties: vec![2],
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
            ],
        };
        assert!(store.apply_commands(tree));

        store.set_command_completion_target(
            Some((
                crate::BlockPos {
                    x: 10,
                    y: 64,
                    z: -3,
                },
                [10.25, 64.5, -2.75],
            )),
            None,
        );
        let texts = |store: &WorldStore, cmd: &str| {
            store
                .complete_command(cmd)
                .suggestions
                .into_iter()
                .map(|s| s.text)
                .collect::<Vec<_>>()
        };
        assert_eq!(texts(&store, "/setblock "), vec!["10", "10 64", "10 64 -3"]);
        assert_eq!(texts(&store, "/setblock 10 "), vec!["10 64", "10 64 -3"]);
        assert_eq!(texts(&store, "/setblock ^"), vec!["^ ^", "^ ^ ^"]);
        assert_eq!(
            texts(&store, "/tp "),
            vec!["10.25", "10.25 64.50", "10.25 64.50 -2.75"]
        );
        assert_eq!(texts(&store, "/forceload "), vec!["10", "10 -3"]);
        assert_eq!(texts(&store, "/spread "), vec!["10.25", "10.25 -2.75"]);

        store.set_command_completion_target(None, Some(targeted_uuid));
        let targeted_uuid_text = targeted_uuid.to_string();
        let kill = texts(&store, "/kill ");
        assert!(kill.iter().any(|text| *text == targeted_uuid_text.as_str()));
        let kick = texts(&store, "/kick ");
        assert!(!kick.iter().any(|text| *text == targeted_uuid_text.as_str()));
    }
}
