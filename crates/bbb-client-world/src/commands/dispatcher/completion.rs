use rustc_hash::FxHashSet;
use std::collections::BTreeMap;

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
    JavaString,
};

use super::{
    CommandCompletionResult, CommandCompletionSuggestion, CommandNodeKindState, CommandTreeState,
};
use crate::FrozenRegistryAccess;

mod blocks;
mod items;
mod selector;

#[derive(Default)]
struct CompletionMetadata {
    tooltip: Option<String>,
    tooltip_translation_key: Option<String>,
}

pub(in crate::commands) struct CommandCompletionSources<'a> {
    pub(in crate::commands) registry_access: FrozenRegistryAccess<'a>,
    pub(in crate::commands) online_player_names: Vec<&'a str>,
    pub(in crate::commands) team_names: Vec<&'a str>,
    pub(in crate::commands) available_levels: Vec<&'a str>,
    pub(in crate::commands) available_sounds: Vec<&'a str>,
    pub(in crate::commands) vanilla_feature_enabled: bool,
    pub(in crate::commands) looked_at_block: Option<(crate::BlockPos, [f64; 3])>,
    pub(in crate::commands) targeted_entity_uuid: Option<uuid::Uuid>,
}

impl CommandTreeState {
    pub(in crate::commands) fn complete_command_with_sources(
        &self,
        source: &[u16],
        sources: CommandCompletionSources<'_>,
    ) -> CommandCompletionResult {
        let parse = self.parse_raw_command_with_registry(source, Some(sources.registry_access));
        // `CommandSuggestions.java:225-244` keeps the whole command in the
        // reader and passes an edit-box cursor, which Brigadier turns into
        // `SuggestionsBuilder.remainingLowerCase`: the input is truncated at the
        // cursor, lowercased whole, and only then cut at
        // `SuggestionContext.startPos` -- a Java `char` index. Both sides of
        // `LiteralCommandNode.listSuggestions` stay UTF-16 units, so a typed
        // high surrogate is a prefix of the pair it begins.
        let start = parse.completion.start.min(source.len());
        let input_lower = JavaString::from(source.to_vec()).to_lowercase_root();
        let remaining_lower = input_lower.units().get(start..).unwrap_or_default();
        // The provider and registry branches read `builder.getRemaining()`, which
        // is the tail itself rather than the tail of the lowered buffer, so they
        // match the raw units of the same slice. `SuggestionsBuilder.suggest`
        // drops a candidate equal to that entire raw tail.
        let remaining = source.get(start..).unwrap_or_default();
        let mut suggestions = BTreeMap::<JavaString, CompletionMetadata>::new();
        let mut request_server = false;
        let Some(parent) = self.node(parse.completion.node_index) else {
            return CommandCompletionResult {
                start,
                length: source.len().saturating_sub(start),
                suggestions: Vec::new(),
                request_server: false,
            };
        };

        let mut child_names = FxHashSet::default();
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
                    if literal_matches(name, remaining_lower)
                        && !name.encode_utf16().eq(remaining.iter().copied())
                    {
                        suggestions.entry(JavaString::from(name)).or_default();
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
                            "minecraft:summonable_entities" => push_summonable_candidates(
                                &mut suggestions,
                                remaining,
                                sources.vanilla_feature_enabled,
                                sources.registry_access.content_registry(),
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
                .map(|(text, metadata)| CommandCompletionSuggestion {
                    text,
                    tooltip: metadata.tooltip,
                    tooltip_translation_key: metadata.tooltip_translation_key,
                })
                .collect(),
            request_server,
        }
    }
}

/// `LiteralCommandNode.listSuggestions`: the node's own lowercased name against
/// the lowercased tail of the buffer, both compared as UTF-16 units.
fn literal_matches(name: &str, remaining_lower: &[u16]) -> bool {
    JavaString::from(name)
        .to_lowercase_root()
        .units()
        .starts_with(remaining_lower)
}

fn push_parser_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    type_id: i32,
    template: &CommandArgumentTemplate,
    remaining: &[u16],
    sources: &CommandCompletionSources<'_>,
) -> bool {
    match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID => {
            push_string_candidates(output, remaining, ["true", "false"]);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID | VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID => {
            let players_only = type_id == VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID
                || matches!(
                    template,
                    CommandArgumentTemplate::Entity {
                        players_only: true,
                        ..
                    }
                );
            selector::push_candidates(output, remaining, players_only, sources);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID => {
            push_coordinate_candidates(
                output,
                remaining,
                CoordinateSuggestionKind::Relevant3D,
                sources.looked_at_block,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID => {
            push_coordinate_candidates(
                output,
                remaining,
                CoordinateSuggestionKind::Absolute3D,
                sources.looked_at_block,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID => {
            push_coordinate_candidates(
                output,
                remaining,
                CoordinateSuggestionKind::Relevant2D,
                sources.looked_at_block,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID => {
            push_coordinate_candidates(
                output,
                remaining,
                CoordinateSuggestionKind::Absolute2D,
                sources.looked_at_block,
            );
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID => {
            blocks::push_candidates(output, remaining, false, sources.registry_access);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID => {
            blocks::push_candidates(output, remaining, true, sources.registry_access);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID => {
            items::push_candidates(output, remaining, sources.registry_access);
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID => {
            items::push_predicate_candidates(output, remaining, sources.registry_access);
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

/// `SharedSuggestionProvider.suggest` (SharedSuggestionProvider.java:226-236):
/// the lowered tail is the pattern, so every candidate the lowered name matches
/// is suggested by its own spelling.
fn push_string_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    remaining: &[u16],
    candidates: impl IntoIterator<Item = impl AsRef<str>>,
) {
    let lower_prefix = lower_units(remaining);
    for candidate in candidates {
        let candidate = candidate.as_ref();
        if matches_sub_str(lower_prefix.units(), lower_str(candidate).units())
            && !candidate.encode_utf16().eq(remaining.iter().copied())
        {
            output.entry(JavaString::from(candidate)).or_default();
        }
    }
}

/// `SharedSuggestionProvider.suggestResource` (SharedSuggestionProvider.java:123-137)
/// through `filterResources` (:93-121), which leaves the id's own spelling alone
/// and reads the namespace and the path separately when the tail has no colon.
///
/// The `prefix` overload (:109-121) does not ask the tail to carry the prefix:
/// an empty tail passes the whole table, and any other tail is cut at
/// `Strings.commonPrefix` alone, so `@e[type=!` still reaches the `!#` tag
/// table. A tail sharing nothing with the prefix reaches no table at all.
fn push_resource_candidates<'a>(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    remaining: &[u16],
    candidates: impl IntoIterator<Item = &'a str>,
    prefix: &str,
) {
    let lowered = lower_units(remaining);
    let tail = lowered.units();
    let contents = if prefix.is_empty() {
        Some(tail)
    } else if tail.is_empty() {
        None
    } else {
        match common_prefix_len(tail, prefix) {
            0 => return,
            common => Some(&tail[common..]),
        }
    };
    for candidate in candidates {
        if contents.is_none_or(|contents| resource_candidate_matches(contents, candidate))
            && !prefix
                .encode_utf16()
                .chain(candidate.encode_utf16())
                .eq(remaining.iter().copied())
        {
            output
                .entry(JavaString::from(format!("{prefix}{candidate}")))
                .or_default();
        }
    }
}

/// `Strings.commonPrefix`, whose surrogate-pair backoff cannot fire here: every
/// prefix the completion path passes is ASCII.
fn common_prefix_len(tail: &[u16], prefix: &str) -> usize {
    tail.iter()
        .zip(prefix.encode_utf16())
        .take_while(|(unit, expected)| **unit == *expected)
        .count()
}

/// One table through every prefix the completion path passes, pinned on the
/// pinned client-26.1 jar's `SharedSuggestionProvider.suggestResource`: the
/// tail is cut only at `Strings.commonPrefix`, so `!` reaches the `!#` table
/// and `!#` accepts a tail that only spells `!`, while a tail sharing nothing
/// with the prefix reaches no table at all.
#[cfg(test)]
#[test]
fn resource_candidates_cut_the_tail_at_the_common_prefix() {
    const TABLE: [&str; 6] = [
        "minecraft:stone",
        "minecraft:oak_log",
        "minecraft:logs",
        "example:stone",
        "minecraft:deep/slate",
        "minecraft:a_b.c",
    ];
    for (prefix, tail, expected) in [
        (
            "",
            "",
            &[
                "example:stone",
                "minecraft:a_b.c",
                "minecraft:deep/slate",
                "minecraft:logs",
                "minecraft:oak_log",
                "minecraft:stone",
            ][..],
        ),
        ("", "stone", &["example:stone", "minecraft:stone"]),
        (
            "",
            "min",
            &[
                "minecraft:a_b.c",
                "minecraft:deep/slate",
                "minecraft:logs",
                "minecraft:oak_log",
                "minecraft:stone",
            ],
        ),
        ("", "#", &[]),
        ("", "!", &[]),
        ("", "minecraft:sto", &["minecraft:stone"]),
        ("", "slate", &["minecraft:deep/slate"]),
        ("", "c", &["minecraft:a_b.c"]),
        (
            "#",
            "",
            &[
                "#example:stone",
                "#minecraft:a_b.c",
                "#minecraft:deep/slate",
                "#minecraft:logs",
                "#minecraft:oak_log",
                "#minecraft:stone",
            ],
        ),
        (
            "#",
            "#",
            &[
                "#example:stone",
                "#minecraft:a_b.c",
                "#minecraft:deep/slate",
                "#minecraft:logs",
                "#minecraft:oak_log",
                "#minecraft:stone",
            ],
        ),
        (
            "#",
            "#min",
            &[
                "#minecraft:a_b.c",
                "#minecraft:deep/slate",
                "#minecraft:logs",
                "#minecraft:oak_log",
                "#minecraft:stone",
            ],
        ),
        // `SuggestionsBuilder.suggest` drops a candidate equal to the tail.
        ("#", "#minecraft:logs", &[]),
        ("#", "stone", &[]),
        ("#", "!", &[]),
        ("#", "##", &[]),
        ("#", "#example:s", &["#example:stone"]),
        (
            "!",
            "",
            &[
                "!example:stone",
                "!minecraft:a_b.c",
                "!minecraft:deep/slate",
                "!minecraft:logs",
                "!minecraft:oak_log",
                "!minecraft:stone",
            ],
        ),
        (
            "!",
            "!",
            &[
                "!example:stone",
                "!minecraft:a_b.c",
                "!minecraft:deep/slate",
                "!minecraft:logs",
                "!minecraft:oak_log",
                "!minecraft:stone",
            ],
        ),
        (
            "!",
            "!min",
            &[
                "!minecraft:a_b.c",
                "!minecraft:deep/slate",
                "!minecraft:logs",
                "!minecraft:oak_log",
                "!minecraft:stone",
            ],
        ),
        ("!", "!#", &[]),
        ("!", "!#min", &[]),
        ("!", "#", &[]),
        ("!", "stone", &[]),
        ("!", "!!", &[]),
        (
            "!#",
            "",
            &[
                "!#example:stone",
                "!#minecraft:a_b.c",
                "!#minecraft:deep/slate",
                "!#minecraft:logs",
                "!#minecraft:oak_log",
                "!#minecraft:stone",
            ],
        ),
        (
            "!#",
            "!",
            &[
                "!#example:stone",
                "!#minecraft:a_b.c",
                "!#minecraft:deep/slate",
                "!#minecraft:logs",
                "!#minecraft:oak_log",
                "!#minecraft:stone",
            ],
        ),
        (
            "!#",
            "!#",
            &[
                "!#example:stone",
                "!#minecraft:a_b.c",
                "!#minecraft:deep/slate",
                "!#minecraft:logs",
                "!#minecraft:oak_log",
                "!#minecraft:stone",
            ],
        ),
        (
            "!#",
            "!#min",
            &[
                "!#minecraft:a_b.c",
                "!#minecraft:deep/slate",
                "!#minecraft:logs",
                "!#minecraft:oak_log",
                "!#minecraft:stone",
            ],
        ),
        (
            "!#",
            "!min",
            &[
                "!#minecraft:a_b.c",
                "!#minecraft:deep/slate",
                "!#minecraft:logs",
                "!#minecraft:oak_log",
                "!#minecraft:stone",
            ],
        ),
        ("!#", "#", &[]),
        ("!#", "!#minecraft:logs", &[]),
        ("!#", "x", &[]),
    ] {
        let mut output = BTreeMap::new();
        let tail_units: Vec<u16> = tail.encode_utf16().collect();
        push_resource_candidates(&mut output, &tail_units, TABLE, prefix);
        let texts: Vec<String> = output
            .into_keys()
            .map(|text| String::from_utf16(text.units()).expect("ASCII candidate"))
            .collect();
        assert_eq!(texts, expected, "{prefix:?} {tail:?}");
    }
}

fn resource_candidate_matches(contents: &[u16], candidate: &str) -> bool {
    let (namespace, path) = candidate
        .split_once(':')
        .unwrap_or(("minecraft", candidate));
    if contents.contains(&u16::from(b':')) {
        matches_sub_str(contents, lower_str(candidate).units())
    } else {
        matches_sub_str(contents, lower_str(namespace).units())
            || matches_sub_str(contents, lower_str(path).units())
    }
}

fn push_registry_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    remaining: &[u16],
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

fn push_objective_criteria_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    remaining: &[u16],
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

#[derive(Clone, Copy)]
enum CoordinateSuggestionKind {
    Relevant3D,
    Absolute3D,
    Relevant2D,
    Absolute2D,
}

fn is_valid_coordinate_token(token: &str, local: bool, integer_only: bool) -> bool {
    if token.is_empty() {
        return false;
    }
    if local {
        let Some(rest) = token.strip_prefix('^') else {
            return false;
        };
        rest.is_empty() || rest.parse::<f64>().is_ok_and(f64::is_finite)
    } else if let Some(rest) = token.strip_prefix('~') {
        rest.is_empty() || rest.parse::<f64>().is_ok_and(f64::is_finite)
    } else if integer_only {
        token.parse::<i32>().is_ok()
    } else {
        token.parse::<f64>().is_ok_and(f64::is_finite)
    }
}

/// Vanilla `BlockPosArgument` / `Vec3Argument` / `ColumnPosArgument` /
/// `Vec2Argument` coordinate completion (`ClientSuggestionProvider.java:125-144`,
/// `SharedSuggestionProvider.java:157-224`).
fn push_coordinate_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    remaining: &[u16],
    kind: CoordinateSuggestionKind,
    looked_at_block: Option<(crate::BlockPos, [f64; 3])>,
) {
    let is_local = remaining.first() == Some(&u16::from(b'^'));
    let (width, integer_only, absolute) = match kind {
        CoordinateSuggestionKind::Relevant3D => (3, true, false),
        CoordinateSuggestionKind::Absolute3D => (3, false, true),
        CoordinateSuggestionKind::Relevant2D => (2, true, false),
        CoordinateSuggestionKind::Absolute2D => (2, false, true),
    };
    let (coord_x, coord_y, coord_z) = if is_local {
        ("^".to_string(), "^".to_string(), "^".to_string())
    } else if let Some((block_pos, location)) = looked_at_block {
        if absolute {
            (
                format!("{:.2}", location[0]),
                format!("{:.2}", location[1]),
                format!("{:.2}", location[2]),
            )
        } else {
            (
                block_pos.x.to_string(),
                block_pos.y.to_string(),
                block_pos.z.to_string(),
            )
        }
    } else {
        ("~".to_string(), "~".to_string(), "~".to_string())
    };
    let axis_values: [&str; 3] = if width == 2 {
        [coord_x.as_str(), coord_z.as_str(), ""]
    } else {
        [coord_x.as_str(), coord_y.as_str(), coord_z.as_str()]
    };

    if remaining.is_empty() {
        let mut built = String::new();
        for axis in axis_values.iter().take(width) {
            if !built.is_empty() {
                built.push(' ');
            }
            built.push_str(axis);
            output.entry(JavaString::from(built.as_str())).or_default();
        }
        return;
    }

    let Ok(input_str) = String::from_utf16(remaining) else {
        return;
    };
    // Mirror Java `String.split(" ")` (drops trailing empty tokens).
    let mut fields: Vec<&str> = input_str.split(' ').collect();
    while fields.last() == Some(&"") {
        fields.pop();
    }
    if fields.is_empty() || fields.len() >= width {
        return;
    }
    if !fields
        .iter()
        .all(|token| is_valid_coordinate_token(token, is_local, integer_only))
    {
        return;
    }

    let mut candidates = Vec::with_capacity(width - fields.len());
    let mut built = fields.join(" ");
    for axis in axis_values.iter().take(width).skip(fields.len()) {
        built.push(' ');
        built.push_str(axis);
        candidates.push(built.clone());
    }
    push_string_candidates(output, remaining, candidates);
}

fn push_time_candidates(output: &mut BTreeMap<JavaString, CompletionMetadata>, remaining: &[u16]) {
    let number_len = remaining
        .iter()
        .take_while(|unit| matches!(**unit, 0x30..=0x39 | 0x2b | 0x2d | 0x2e))
        .count();
    if number_len == 0 {
        return;
    }
    let (number, unit) = remaining.split_at(number_len);
    for suffix in ["d", "s", "t"] {
        let suffix_units = suffix.encode_utf16().collect::<Vec<_>>();
        if suffix_units.starts_with(unit) {
            let mut candidate = number.to_vec();
            candidate.extend_from_slice(&suffix_units);
            output.entry(JavaString::from(candidate)).or_default();
        }
    }
}

fn push_summonable_candidates(
    output: &mut BTreeMap<JavaString, CompletionMetadata>,
    remaining: &[u16],
    vanilla_feature_enabled: bool,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) {
    let contents = lower_units(remaining);
    let mod_summonables = (!content_registry.is_vanilla_only()).then(|| {
        content_registry
            .entities()
            .iter()
            .filter(|spec| spec.can_summon)
            .map(|spec| spec.resource_id.as_str())
    });
    for resource_id in VANILLA_ENTITY_TYPES
        .iter()
        .filter(|entry| vanilla_feature_enabled && entry.can_summon)
        .map(|entry| entry.resource_id)
        .chain(mod_summonables.into_iter().flatten())
    {
        if !resource_candidate_matches(contents.units(), resource_id) {
            continue;
        }
        if resource_id.encode_utf16().eq(remaining.iter().copied()) {
            continue;
        }
        let (namespace, path) = resource_id
            .split_once(':')
            .unwrap_or(("minecraft", resource_id));
        output.insert(
            JavaString::from(resource_id),
            CompletionMetadata {
                tooltip: Some(resource_id.to_string()),
                tooltip_translation_key: Some(format!(
                    "entity.{namespace}.{}",
                    path.replace('/', ".")
                )),
            },
        );
    }
}

fn lower_str(text: &str) -> JavaString {
    JavaString::from(text).to_lowercase_root()
}

fn lower_units(units: &[u16]) -> JavaString {
    JavaString::from(units.to_vec()).to_lowercase_root()
}

/// Candidate text for a prefixed or suffixed provider, assembled from the
/// buffer's own units: the literal pieces are ASCII and the middle keeps
/// whatever the scan read.
#[derive(Clone)]
pub(super) struct CandidateText(Vec<u16>);

impl CandidateText {
    pub(super) fn ascii(text: &str) -> Self {
        Self(text.encode_utf16().collect())
    }

    pub(super) fn units(units: &[u16]) -> Self {
        Self(units.to_vec())
    }

    pub(super) fn then_ascii(mut self, text: &str) -> Self {
        self.0.extend(text.encode_utf16());
        self
    }

    pub(super) fn then_units(mut self, units: &[u16]) -> Self {
        self.0.extend_from_slice(units);
        self
    }

    pub(super) fn then_unit(mut self, byte: u8) -> Self {
        self.0.push(u16::from(byte));
        self
    }

    pub(super) fn finish(self) -> JavaString {
        JavaString::from(self.0)
    }
}

/// The block, item and selector scans read the buffer's own units. The
/// `String::from_utf16_lossy` projection they used to split kept one character
/// per unit, so an ASCII separator sat at the same unit offset either way, but
/// only the units keep an unpaired surrogate instead of U+FFFD -- in the
/// comparisons and in the candidate text spliced back out of the buffer alike.
fn split_once_unit(units: &[u16], byte: u8) -> Option<(&[u16], &[u16])> {
    let separator = units.iter().position(|unit| *unit == u16::from(byte))?;
    Some((&units[..separator], &units[separator + 1..]))
}

fn find_any_unit(units: &[u16], bytes: &[u8]) -> Option<usize> {
    units
        .iter()
        .position(|unit| bytes.iter().any(|byte| u16::from(*byte) == *unit))
}

fn contains_unit(units: &[u16], byte: u8) -> bool {
    units.contains(&u16::from(byte))
}

fn starts_with_unit(units: &[u16], byte: u8) -> bool {
    units.first() == Some(&u16::from(byte))
}

fn unit_at(units: &[u16], index: usize, byte: u8) -> bool {
    units.get(index) == Some(&u16::from(byte))
}

fn trim_start_units(units: &[u16]) -> &[u16] {
    let start = units
        .iter()
        .position(|unit| !is_whitespace_unit(*unit))
        .unwrap_or(units.len());
    &units[start..]
}

/// `text.equals(units)` for one ASCII text of the grammar.
fn units_eq_ascii(units: &[u16], text: &str) -> bool {
    text.encode_utf16().eq(units.iter().copied())
}

/// The units as text, or `None` when they leave ASCII. Every consumer of this
/// -- the identifier, property and component lookups -- accepts ASCII only, so
/// the projection turned such a unit into U+FFFD just to reject it one step
/// later.
fn ascii_text(units: &[u16]) -> Option<String> {
    units
        .iter()
        .all(|unit| *unit < 0x80)
        .then(|| String::from_utf16(units).expect("ASCII units"))
}

/// Rust's `char::is_whitespace` set, which is entirely inside the BMP.
fn is_whitespace_unit(unit: u16) -> bool {
    matches!(unit, 0x0009..=0x000d | 0x0020 | 0x0085 | 0x00a0 | 0x1680
        | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000)
}

/// `SharedSuggestionProvider.matchesSubStr` (SharedSuggestionProvider.java:271-284).
///
/// `MATCH_SPLITTER` (:31) is `CharMatcher.anyOf("._/")` and both `indexIn` and
/// `String.startsWith` count Java `char` units, so the restart positions are
/// unit positions and an unpaired surrogate is only ever equal to itself.
///
/// The chat runtime's custom tab completion runs the same `suggest` overload, so
/// this is the one implementation both consumers call.
pub fn matches_sub_str(pattern: &[u16], input: &[u16]) -> bool {
    let mut index = 0;
    while !input
        .get(index..)
        .is_some_and(|rest| rest.starts_with(pattern))
    {
        let Some(offset) = input[index..]
            .iter()
            .position(|unit| matches!(unit, 0x2e | 0x5f | 0x2f))
        else {
            return false;
        };
        index += offset + 1;
    }
    true
}
