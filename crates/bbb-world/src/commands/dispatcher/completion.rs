use std::collections::{BTreeMap, HashSet};

use bbb_protocol::{
    chat_formattings::VANILLA_CHAT_FORMATTINGS,
    entity_anchors::VANILLA_ENTITY_ANCHORS,
    entity_types::VANILLA_ENTITY_TYPES,
    game_types::VANILLA_GAME_TYPES,
    heightmap_types::{HeightmapUsage, VANILLA_HEIGHTMAP_TYPES},
    packets::{
        command_argument_template, CommandArgumentTemplate,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID, VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID, VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID,
        VANILLA_COMMAND_SLOTS, VANILLA_TEMPLATE_MIRRORS, VANILLA_TEMPLATE_ROTATIONS,
    },
    scoreboard_display_slots::VANILLA_SCOREBOARD_DISPLAY_SLOTS,
    stat_types::{VANILLA_OBJECTIVE_CRITERIA_NAMES, VANILLA_STAT_TYPES},
};

use super::{
    CommandCompletionResult, CommandCompletionSuggestion, CommandNodeKindState, CommandTreeState,
};
use crate::FrozenRegistryAccess;

pub(in crate::commands) struct CommandCompletionSources<'a> {
    pub(in crate::commands) registry_access: FrozenRegistryAccess<'a>,
    pub(in crate::commands) online_player_names: Vec<&'a str>,
    pub(in crate::commands) team_names: Vec<&'a str>,
    pub(in crate::commands) available_levels: Vec<&'a str>,
    pub(in crate::commands) available_sounds: Vec<&'a str>,
    pub(in crate::commands) vanilla_feature_enabled: bool,
}

impl CommandTreeState {
    pub(in crate::commands) fn complete_command_with_sources(
        &self,
        source: &str,
        sources: CommandCompletionSources<'_>,
    ) -> CommandCompletionResult {
        let parse = self.parse_command_with_registry(source, Some(sources.registry_access));
        let start = parse.completion.start.min(source.len());
        let remaining = source.get(start..).unwrap_or_default();
        let mut suggestions = BTreeMap::<String, Option<String>>::new();
        let mut request_server = false;
        let Some(parent) = self.node(parse.completion.node_index) else {
            return CommandCompletionResult {
                start,
                length: source.len().saturating_sub(start),
                suggestions: Vec::new(),
                request_server: false,
            };
        };

        let mut child_names = HashSet::new();
        for child_index in &parent.children {
            let Some(child) = self.node(*child_index) else {
                continue;
            };
            let name = child.name.as_deref().unwrap_or_default();
            if !child_names.insert(name) {
                continue;
            }
            match child.node_type {
                CommandNodeKindState::Literal => {
                    if name.to_lowercase().starts_with(&remaining.to_lowercase()) {
                        suggestions.entry(name.to_string()).or_default();
                    }
                }
                CommandNodeKindState::Argument => {
                    if let Some(provider) = child.suggestions.as_deref() {
                        match provider {
                            "minecraft:available_sounds" => push_resource_candidates(
                                &mut suggestions,
                                remaining,
                                sources.available_sounds.iter().copied(),
                                "",
                            ),
                            "minecraft:summonable_entities" => push_resource_candidates(
                                &mut suggestions,
                                remaining,
                                VANILLA_ENTITY_TYPES
                                    .iter()
                                    .filter(|entry| {
                                        sources.vanilla_feature_enabled && entry.can_summon
                                    })
                                    .map(|entry| entry.resource_id),
                                "",
                            ),
                            "minecraft:ask_server" => request_server = true,
                            _ => request_server = true,
                        }
                        continue;
                    }
                    let Some(parser) = child.parser.as_ref() else {
                        continue;
                    };
                    let template =
                        match command_argument_template(parser.type_id, &parser.properties) {
                            Ok(template) => template,
                            Err(_) => {
                                request_server = true;
                                continue;
                            }
                        };
                    request_server |= push_parser_candidates(
                        &mut suggestions,
                        parser.type_id,
                        &template,
                        remaining,
                        &sources,
                    );
                }
                CommandNodeKindState::Root => {}
            }
        }

        CommandCompletionResult {
            start,
            length: source.len().saturating_sub(start),
            suggestions: suggestions
                .into_iter()
                .map(|(text, tooltip)| CommandCompletionSuggestion { text, tooltip })
                .collect(),
            request_server,
        }
    }
}

fn push_parser_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    type_id: i32,
    template: &CommandArgumentTemplate,
    remaining: &str,
    sources: &CommandCompletionSources<'_>,
) -> bool {
    match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID => {
            push_string_candidates(output, remaining, ["true", "false"]);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID | VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID => {
            push_string_candidates(
                output,
                remaining,
                sources.online_player_names.iter().copied(),
            );
            push_string_candidates(output, remaining, ["@p", "@a", "@r", "@s", "@e", "@n"]);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID | VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID => {
            push_coordinate_candidates(output, remaining, 3);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID | VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID => {
            push_coordinate_candidates(output, remaining, 2);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID => {
            push_block_candidates(output, remaining, false, sources.registry_access);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID => {
            push_block_candidates(output, remaining, true, sources.registry_access);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID => {
            push_item_candidates(output, remaining, sources.registry_access);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID => {
            push_registry_candidates(
                output,
                remaining,
                "minecraft:item",
                true,
                sources.registry_access,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_CHAT_FORMATTINGS
                .iter()
                .filter(|entry| !entry.is_format)
                .map(|entry| entry.serialized_name),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID => {
            push_string_candidates(output, remaining, ["F00", "FF0000"]);
        }
        // Vanilla's client source delegates ObjectiveArgument to
        // customSuggestion rather than reading its client scoreboard.
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID => return true,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID => {
            push_objective_criteria_candidates(output, remaining, sources.registry_access);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID => push_string_candidates(
            output,
            remaining,
            ["=", "+=", "-=", "*=", "/=", "%=", "<", ">", "><"],
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID => {
            push_registry_candidates(
                output,
                remaining,
                "minecraft:particle_type",
                false,
                sources.registry_access,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_SCOREBOARD_DISPLAY_SLOTS
                .iter()
                .map(|entry| entry.serialized_name),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID => {
            push_string_candidates(output, remaining, sources.team_names.iter().copied());
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_COMMAND_SLOTS
                .iter()
                .filter(|(_, single)| *single)
                .map(|(name, _)| *name),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_COMMAND_SLOTS.iter().map(|(name, _)| *name),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_ENTITY_ANCHORS
                .iter()
                .map(|entry| entry.serialized_name),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID => push_resource_candidates(
            output,
            remaining,
            sources.available_levels.iter().copied(),
            "",
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_GAME_TYPES.iter().map(|entry| entry.serialized_name),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID => push_time_candidates(output, remaining),
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID => {
            let CommandArgumentTemplate::Registry { registry } = template else {
                return true;
            };
            if !push_registry_candidates(output, remaining, registry, true, sources.registry_access)
            {
                return true;
            }
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID => {
            let CommandArgumentTemplate::Registry { registry } = template else {
                return true;
            };
            if !push_registry_candidates(
                output,
                remaining,
                registry,
                false,
                sources.registry_access,
            ) {
                return true;
            }
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID => {
            push_string_candidates(output, remaining, VANILLA_TEMPLATE_MIRRORS.iter().copied());
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_TEMPLATE_ROTATIONS.iter().copied(),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID => push_string_candidates(
            output,
            remaining,
            VANILLA_HEIGHTMAP_TYPES
                .iter()
                .filter(|entry| entry.usage != HeightmapUsage::Worldgen)
                .map(|entry| entry.serialization_key.to_ascii_lowercase()),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID => {
            return !push_registry_candidates(
                output,
                remaining,
                "minecraft:loot_table",
                false,
                sources.registry_access,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID => {
            return !push_registry_candidates(
                output,
                remaining,
                "minecraft:predicate",
                false,
                sources.registry_access,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID => {
            return !push_registry_candidates(
                output,
                remaining,
                "minecraft:item_modifier",
                false,
                sources.registry_access,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID => {
            return !push_registry_candidates(
                output,
                remaining,
                "minecraft:dialog",
                false,
                sources.registry_access,
            );
        }
        _ => {}
    }
    false
}

fn push_string_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    candidates: impl IntoIterator<Item = impl AsRef<str>>,
) {
    let lower_remaining = remaining.to_lowercase();
    for candidate in candidates {
        let candidate = candidate.as_ref();
        if matches_suggestion_substring(&lower_remaining, &candidate.to_lowercase()) {
            output.entry(candidate.to_string()).or_default();
        }
    }
}

fn push_resource_candidates<'a>(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    candidates: impl IntoIterator<Item = &'a str>,
    prefix: &str,
) {
    let lower_remaining = remaining.to_lowercase();
    let Some(stripped) = lower_remaining.strip_prefix(prefix) else {
        return;
    };
    for candidate in candidates {
        let (namespace, path) = candidate
            .split_once(':')
            .unwrap_or(("minecraft", candidate));
        let matched = if stripped.contains(':') {
            matches_suggestion_substring(stripped, &candidate.to_lowercase())
        } else {
            matches_suggestion_substring(stripped, &namespace.to_lowercase())
                || matches_suggestion_substring(stripped, &path.to_lowercase())
        };
        if matched {
            output.entry(format!("{prefix}{candidate}")).or_default();
        }
    }
}

fn push_registry_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    registry: &str,
    include_tags: bool,
    access: FrozenRegistryAccess<'_>,
) -> bool {
    let Some(resources) = access.resource_ids_in_holder_order(registry) else {
        return false;
    };
    push_resource_candidates(output, remaining, resources, "");
    if include_tags {
        if let Some(tags) = access.tag_ids(registry) {
            push_resource_candidates(output, remaining, tags, "#");
        }
    }
    true
}

fn push_block_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    include_tags: bool,
    access: FrozenRegistryAccess<'_>,
) {
    push_registry_candidates(output, remaining, "minecraft:block", include_tags, access);
}

fn push_item_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    access: FrozenRegistryAccess<'_>,
) {
    let Some((item, components)) = remaining.split_once('[') else {
        push_registry_candidates(output, remaining, "minecraft:item", false, access);
        if access
            .resource_ids_in_holder_order("minecraft:item")
            .is_some_and(|resources| {
                resources
                    .into_iter()
                    .any(|candidate| candidate == remaining)
            })
        {
            output.entry(format!("{remaining}[")).or_default();
        }
        return;
    };
    if components.contains([',', '=', ']']) {
        return;
    }
    let Some(component_ids) = access.resource_ids_in_holder_order("minecraft:data_component_type")
    else {
        return;
    };
    if components.is_empty() {
        output.entry(format!("{item}[!")).or_default();
    }
    let prefix = components.strip_prefix('!').map_or("", |_| "!");
    let component_remaining = components.strip_prefix('!').unwrap_or(components);
    let mut nested = BTreeMap::new();
    push_resource_candidates(&mut nested, component_remaining, component_ids, "");
    for component in nested.into_keys() {
        let suffix = if prefix.is_empty() { "=" } else { "" };
        output
            .entry(format!("{item}[{prefix}{component}{suffix}"))
            .or_default();
    }
}

fn push_objective_criteria_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    access: FrozenRegistryAccess<'_>,
) {
    push_string_candidates(
        output,
        remaining,
        VANILLA_OBJECTIVE_CRITERIA_NAMES.iter().copied(),
    );
    let mut criteria = Vec::new();
    for stat_type in VANILLA_STAT_TYPES {
        let Some(values) = access.resource_ids_in_holder_order(stat_type.value_registry) else {
            continue;
        };
        let stat = stat_type.resource_id.replace(':', ".");
        criteria.extend(
            values
                .into_iter()
                .map(|value| format!("{stat}:{}", value.replace(':', "."))),
        );
    }
    push_string_candidates(output, remaining, criteria);
}

fn push_coordinate_candidates(
    output: &mut BTreeMap<String, Option<String>>,
    remaining: &str,
    width: usize,
) {
    let marker = if remaining.starts_with('^') { "^" } else { "~" };
    let parts = remaining.split_whitespace().count();
    if remaining.is_empty() {
        for count in 1..=width {
            output
                .entry(
                    std::iter::repeat_n(marker, count)
                        .collect::<Vec<_>>()
                        .join(" "),
                )
                .or_default();
        }
    } else if parts < width {
        let mut candidate = remaining.trim_end().to_string();
        for _ in parts..width {
            candidate.push(' ');
            candidate.push_str(marker);
            output.entry(candidate.clone()).or_default();
        }
    }
}

fn push_time_candidates(output: &mut BTreeMap<String, Option<String>>, remaining: &str) {
    let number_len = remaining
        .bytes()
        .take_while(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.'))
        .count();
    if number_len == 0 {
        return;
    }
    let number = &remaining[..number_len];
    let unit = &remaining[number_len..];
    for suffix in ["d", "s", "t"] {
        if suffix.starts_with(unit) {
            output.entry(format!("{number}{suffix}")).or_default();
        }
    }
}

fn matches_suggestion_substring(pattern: &str, input: &str) -> bool {
    input.starts_with(pattern)
        || input.char_indices().any(|(index, character)| {
            matches!(character, '.' | '_' | '/')
                && input[index + character.len_utf8()..].starts_with(pattern)
        })
}
