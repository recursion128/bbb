//! What `CommandDispatcher.getCompletionSuggestions` answers for this tree.
//!
//! Three vanilla details carry the whole behaviour. `findSuggestionContext`
//! (CommandContextBuilder.java:127-159) picks the node whose *children* are
//! offered: input the parse could not consume offers the last node's children,
//! and input it consumed exactly offers the children of the node before the one
//! the cursor sits in — which is why a complete command suggests its siblings
//! rather than its own continuations. Each of those children then filters the
//! candidates itself, in `listSuggestions`, and the filters are not all the
//! same rule: a literal compares with `String.startsWith`
//! (LiteralCommandNode.java:67-69) while everything that goes out through
//! `SharedSuggestionProvider.suggest` compares with `matchesSubStr`
//! ([`matches_sub_str`]) instead. Finally `SuggestionsBuilder.suggest`
//! (SuggestionsBuilder.java:54-61) drops a candidate equal to what is left to
//! read, so text the client already typed in full is never echoed back.
//!
//! Permission is where completion and parsing part company. The parse is the
//! one `handleCustomCommandSuggestions` made
//! (ServerGamePacketListenerImpl.java:596), so `canUse` decides which nodes it
//! could walk — but the suggestion loop itself
//! (com/mojang/brigadier/CommandDispatcher.java:299-311) iterates
//! `parent.getChildren()` of the server's own dispatcher without asking
//! `canUse` again, so vanilla answers with commands the source could not run.
//! Only what this server never registered is missing, which is every command
//! outside [`super::catalog::IMPLEMENTED_COMMANDS`]. Permission does reach one
//! candidate list: the six selector forms, which `EntitySelectorParser` offers
//! only to a source holding `COMMANDS_ENTITY_SELECTORS`.

use bbb_protocol::block_types::VANILLA_BLOCK_TYPES;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_PREDICATE_TYPES, VANILLA_DATA_COMPONENT_TYPES,
};
use bbb_protocol::entity_anchors::VANILLA_ENTITY_ANCHORS;
use bbb_protocol::game_types::VANILLA_GAME_TYPES;
use bbb_protocol::item_types::VANILLA_ITEM_TYPES;
use bbb_protocol::packets::{
    CommandArgumentTemplate, CommandSuggestion, VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID, VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID, VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID, VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID, VANILLA_COMMAND_TREE_ROOT,
};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPES;
use bbb_protocol::sound_event_types::VANILLA_SOUND_EVENT_RESOURCE_IDS;
use bbb_protocol::{
    scan_command_resource_location, scan_command_whitespace, CommandItemComponentPatchScanner,
    CommandItemComponentSuggestions, CommandItemPredicateSuggestions,
};

use bbb_server_datapack::damage::DAMAGE_TYPE_REGISTRY;

use super::parse::{
    block_argument_suggestions, parse, parses_as_block_pos, parses_as_column_pos, parses_as_vec2,
    parses_as_vec3, ArgumentValue, BlockArgumentCandidates, ParseResult, SelectorKind,
};
use super::properties::argument_template;
use super::run::{
    escape_if_required, score_holder_names_for_suggest, select_damage_entity_states,
    source_entity_state,
};
use super::types::CommandDamageEntityRef;
use super::{
    catalog, component, translatable, CommandCompletions, CommandOrigin, CommandPermission,
    CommandSource,
};
use crate::world::ServerWorld;

mod selector;

/// The suggestions the command-suggestion handler answers with.
pub(super) fn completions(
    command: &str,
    source: &CommandSource,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> CommandCompletions {
    let permission = source.permission;
    // `handleCustomCommandSuggestions` (ServerGamePacketListenerImpl.java:591-596)
    // skips one leading slash and parses the rest of that same string, so every
    // offset the answer carries is an index into what the client sent, slash
    // included.
    let offset = usize::from(command.starts_with('/'));
    let input = &command[offset..];
    let (parsed, cursor) = bbb_protocol::JavaString::with_utf16(input, |units| {
        (parse(units, permission, world), units.len())
    });

    let (parent, start) = match parsed.nodes.last() {
        Some(last) if last.end < cursor => (last.index, last.end + 1),
        Some(_) => {
            let mut previous = VANILLA_COMMAND_TREE_ROOT;
            let mut start = 0;
            for node in &parsed.nodes {
                if node.start <= cursor && cursor <= node.end {
                    start = node.start;
                    break;
                }
                previous = node.index;
            }
            (previous, start)
        }
        None => (VANILLA_COMMAND_TREE_ROOT, 0),
    };
    // A redirect offers the children of the node it points at, because that is
    // the node the parse would have continued from.
    let parent = catalog::redirect_target(parent).unwrap_or(parent);

    let start = bbb_protocol::command_utf16_byte_cursor(input, start.min(cursor));
    let remaining = &command[offset + start..];
    let remaining_lower = remaining.to_lowercase();
    let mut candidates: Vec<Candidate> = Vec::new();
    for child in catalog::implemented_children(parent) {
        if catalog::is_literal(child) {
            // `LiteralCommandNode.listSuggestions`
            // (LiteralCommandNode.java:67-69) offers the literal only when it
            // starts with what is left to read, both sides in lower case.
            let name = catalog::node(child).name;
            if name.to_lowercase().starts_with(&remaining_lower) {
                candidates.push(Candidate::at(offset + start, plain(name)));
            }
            continue;
        }
        candidates.extend(argument_suggestions(
            child,
            offset + start,
            remaining,
            &parsed,
            source,
            world,
            at,
        ));
    }
    merged(command, candidates)
}

/// `Suggestions.merge` (Suggestions.java:62-78) over what the children
/// answered, which for more than one of them is `create` (`:80-103`).
fn merged(command: &str, mut candidates: Vec<Candidate>) -> CommandCompletions {
    // `SuggestionsBuilder.suggest` (SuggestionsBuilder.java:54-61 for the
    // untooltipped form, :63-70 for the tooltipped one) drops a candidate equal
    // to the remaining text whichever list it came from, and does so after the
    // candidate's own filter has already accepted it — against the remainder of
    // the builder that candidate came off, which a re-based stage takes from
    // its own start. The integer form (`:72-75`) has no such test.
    candidates.retain(|candidate| {
        candidate.integer.is_some() || candidate.suggestion.text != command[candidate.start..]
    });
    let Some(range_start) = candidates.iter().map(|candidate| candidate.start).min() else {
        // `Suggestions.EMPTY` is a range at zero with no entries
        // (Suggestions.java:13), which is what `merge`/`create` (:62-103) fall
        // back to whenever nothing matched — so an empty answer carries no
        // offset at all.
        return CommandCompletions::default();
    };
    // `Suggestions.create` (:80-103) takes the least start and the greatest end
    // over the candidates and rewrites each one into that range with
    // `Suggestion.expand` (Suggestion.java:84-100), which prepends the input
    // between the two starts. Every candidate here ends at the cursor, so only
    // the prefix half of `expand` ever applies.
    for candidate in &mut candidates {
        if candidate.start > range_start {
            let mut units: Vec<u16> = command[range_start..candidate.start]
                .encode_utf16()
                .collect();
            units.extend_from_slice(candidate.suggestion.text.units());
            candidate.suggestion.text = units.into();
            // `expand` answers a plain `Suggestion` once it rewrites the text
            // (Suggestion.java:94-99), so a widened integer candidate loses the
            // numeric order it was sorted by.
            candidate.integer = None;
        }
    }
    // `create` collects into a set (:94-98) and then sorts it with
    // `Suggestion.compareToIgnoreCase` (Suggestion.java:80-82), which an
    // `IntegerSuggestion` answers by value (IntegerSuggestion.java:60-63). The
    // set keys on the tooltip as well as the text (`Suggestion.equals`,
    // :53-64), so two candidates sharing a text but not a tooltip both survive
    // — which is why this dedups on the whole suggestion rather than on
    // [`CommandSuggestion::text`].
    candidates.sort_by_key(|left| left.sort_key());
    let mut suggestions: Vec<CommandSuggestion> = candidates
        .into_iter()
        .map(|candidate| candidate.suggestion)
        .collect();
    suggestions.dedup();
    CommandCompletions {
        start: command[..range_start].encode_utf16().count() as i32,
        length: command[range_start..].encode_utf16().count() as i32,
        suggestions,
    }
}

/// One candidate with the byte offset of its argument's current suggestion stage.
struct Candidate {
    start: usize,
    integer: Option<i32>,
    suggestion: CommandSuggestion,
}

impl Candidate {
    fn at(start: usize, suggestion: CommandSuggestion) -> Self {
        Self {
            start,
            integer: None,
            suggestion,
        }
    }

    /// `Suggestion.compareToIgnoreCase` (Suggestion.java:80-82) as a key.
    ///
    /// An `IntegerSuggestion` compares by value against another one and by text
    /// against anything else (IntegerSuggestion.java:60-63); the only list that
    /// mixes the two is a tag's property values beside the `,` and `]` that
    /// close it, where `,` sorts under every digit and `]` over every digit, so
    /// a fixed-width value reproduces both orders. `IntegerProperty` is never
    /// negative (IntegerProperty.java:15-17).
    fn sort_key(&self) -> bbb_protocol::JavaString {
        match self.integer {
            Some(value) => format!("{value:010}").into(),
            None => self.suggestion.text.to_lowercase_root(),
        }
    }
}

/// `SuggestionsBuilder.suggest(String)` (SuggestionsBuilder.java:54-61): a
/// candidate with no tooltip, which is every one this dispatcher offers except
/// the selector forms.
fn plain(text: impl Into<bbb_protocol::JavaString>) -> CommandSuggestion {
    CommandSuggestion {
        text: text.into(),
        tooltip: None,
        tooltip_styled: None,
        tooltip_component: None,
    }
}

/// `SharedSuggestionProvider.matchesSubStr`
/// (SharedSuggestionProvider.java:271-283).
///
/// The pattern has to be a prefix of the input, or of what follows one of the
/// input's splitters, or of what follows a later one. `MATCH_SPLITTER` (:31) is
/// `CharMatcher.anyOf("._/")`, and `_` being in that set is what separates this
/// rule from a plain `starts_with`: `StringUtil.isValidPlayerName`
/// (StringUtil.java:66-68) rejects only characters at or below space and at or
/// above `\u{7f}`, so an underscore is legal in a player name and `smi` reaches
/// `Bob_Smith`.
///
/// Both sides arrive already lowercased, the way every
/// `SharedSuggestionProvider.suggest` overload lowercases them before calling
/// this (:226-236 for the `Iterable` one).
fn matches_sub_str(pattern: &str, input: &str) -> bool {
    let mut index = 0;
    while !input[index..].starts_with(pattern) {
        // `MATCH_SPLITTER.indexIn(input, index)` is the first splitter at or
        // after `index`; a `-1` from it ends the loop with a refusal (:276-278).
        let Some(splitter) = input[index..].find(['.', '_', '/']) else {
            return false;
        };
        index += splitter + 1;
    }
    true
}

/// `ArgumentType.listSuggestions` for the parsers this server publishes, with
/// the builder start each answer carries.
fn argument_suggestions(
    child: u16,
    start: usize,
    remaining: &str,
    parsed: &ParseResult,
    source: &CommandSource,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Vec<Candidate> {
    // `ArgumentCommandNode.listSuggestions` (ArgumentCommandNode.java:70-72)
    // dispatches to the node's own `customSuggestions` before asking the
    // argument parser for defaults.
    if let Some(suggestions) =
        custom_suggestions_override(child, remaining, parsed, source, world, at)
    {
        return suggestions
            .into_iter()
            .map(|suggestion| Candidate::at(start, suggestion))
            .collect();
    }
    match catalog::node(child).parser_type_id() {
        // `BlockStateArgument.listSuggestions` (BlockStateArgument.java:40-42)
        // and `BlockPredicateArgument.listSuggestions`
        // (BlockPredicateArgument.java:59-61) both hand the work to
        // `BlockStateParser.fillSuggestions` (BlockStateParser.java:128-141),
        // which answers from wherever its own parse stopped.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID) => {
            block_argument_candidates(start, remaining, world, false)
        }
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID) => {
            block_argument_candidates(start, remaining, world, true)
        }
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID) => {
            item_argument_candidates(start, remaining, world)
        }
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID) => {
            item_predicate_candidates(start, remaining, world)
        }
        Some(
            VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID
            | VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID
            | VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID,
        ) if source.permission >= CommandPermission::GameMaster => {
            selector::option_candidates(start, remaining, world)
                .unwrap_or_else(|| start_candidates(child, start, remaining, source, world))
        }
        _ => start_candidates(child, start, remaining, source, world),
    }
}

fn start_candidates(
    child: u16,
    start: usize,
    remaining: &str,
    source: &CommandSource,
    world: &ServerWorld,
) -> Vec<Candidate> {
    argument_start_suggestions(child, remaining, source, world)
        .into_iter()
        .map(|suggestion| Candidate::at(start, suggestion))
        .collect()
}

/// The argument parsers whose candidates all start where the argument does.
fn argument_start_suggestions(
    child: u16,
    remaining: &str,
    source: &CommandSource,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    let permission = source.permission;
    let node = catalog::node(child);
    match node.parser_type_id() {
        Some(
            VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID
            | VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID
            | VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID,
        ) => names_or_selector_forms(remaining, permission, world),
        // `ObjectiveArgument.listSuggestions` (ObjectiveArgument.java:57-64).
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID) => objective_suggestions(remaining, world),
        // `BoolArgumentType.listSuggestions` (BoolArgumentType.java:31-41)
        // compares with `String.startsWith` against the lowercased remainder
        // rather than going through `SharedSuggestionProvider`, so no splitter
        // rule applies to it.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID) => {
            let remaining = remaining.to_lowercase();
            ["true", "false"]
                .into_iter()
                .filter(|value| value.starts_with(&remaining))
                .map(plain)
                .collect()
        }
        // `GameModeArgument.listSuggestions` (GameModeArgument.java:38-42)
        // hands `GameType.values()` by name to
        // `SharedSuggestionProvider.suggest` (:40), so these are filtered by
        // `matchesSubStr` — which no game type's name can tell apart from a
        // prefix match, none of them carrying a splitter.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID) => {
            let remaining = remaining.to_lowercase();
            VANILLA_GAME_TYPES
                .iter()
                .filter(|entry| matches_sub_str(&remaining, &entry.serialized_name.to_lowercase()))
                .map(|entry| plain(entry.serialized_name))
                .collect()
        }
        // `ResourceArgument.listSuggestions` (ResourceArgument.java:122-123)
        // asks for `ElementSuggestionType.ELEMENTS`, which offers the registry's
        // element ids and no tag ids (`suggestRegistryElements`,
        // SharedSuggestionProvider.java:63-73).
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID) => {
            registry_suggestions(node, remaining, world)
        }
        // `ResourceKeyArgument.listSuggestions` (`ResourceKeyArgument.java:121-123`)
        // asks for `ElementSuggestionType.ELEMENTS` over the named registry.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID) => {
            resource_key_suggestions(node, remaining, world)
        }
        // `ResourceOrTagArgument.listSuggestions`
        // (ResourceOrTagArgument.java:93-95) asks for
        // `ElementSuggestionType.ALL` (`SharedSuggestionProvider.java:63-73`):
        // tags with `#`, then elements. This projection lists registry elements;
        // tags are a per-id lookup, so only elements are offered.
        Some(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID
            | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
        ) => resource_or_tag_suggestions(node, remaining, world),
        // `Vec3Argument.listSuggestions` (Vec3Argument.java:51-65).
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID) => {
            coordinate_suggestions(remaining, parses_as_vec3)
        }
        // `BlockPosArgument.listSuggestions` (BlockPosArgument.java:66-80) is
        // the same helper reached through `getRelevantCoordinates` instead of
        // `getAbsoluteCoordinates`, with the whole-number reader as the
        // validator -- so `~0.5` survives under a `<location>` and is dropped
        // under a `<pos>`.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID) => {
            coordinate_suggestions(remaining, parses_as_block_pos)
        }
        // `ColumnPosArgument.listSuggestions` (ColumnPosArgument.java:52-66)
        // reaches `suggest2DCoordinates` (SharedSuggestionProvider.java:196-224)
        // instead, which offers two candidates rather than three and nothing at
        // all once two fields are typed.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID) => {
            two_axis_coordinate_suggestions(remaining, parses_as_column_pos)
        }
        // `Vec2Argument.listSuggestions` (Vec2Argument.java:61-75) is the same
        // `suggest2DCoordinates` helper (SharedSuggestionProvider.java:196-224)
        // with `Vec2Argument.parse` as the validator.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID) => {
            two_axis_coordinate_suggestions(remaining, parses_as_vec2)
        }
        // `SlotArgument.listSuggestions` (`SlotArgument.java:48-50`) is
        // `SharedSuggestionProvider.suggest` over `singleSlotNames`.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID) => slot_suggestions(
            remaining,
            bbb_server_types::slot_ranges::single_slot_names()
                .iter()
                .map(String::as_str),
        ),
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID) => {
            slot_suggestions(remaining, bbb_server_types::slot_ranges::all_slot_names())
        }
        // `ParticleArgument.listSuggestions` (`ParticleArgument.java:85-87`)
        // is `suggestResource` over particle type ids. `{...}` stays unread.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID) => particle_id_suggestions(remaining),
        // `ResourceOrIdArgument.listSuggestions` (`ResourceOrIdArgument.java:135-137`)
        // is `SharedSuggestionProvider.listSuggestions` over registry elements.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID) => dialog_id_suggestions(remaining, world),
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID) => {
            loot_registry_suggestions("minecraft:loot_table", remaining, world)
        }
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID) => {
            loot_registry_suggestions("minecraft:item_modifier", remaining, world)
        }
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID) => {
            loot_registry_suggestions("minecraft:predicate", remaining, world)
        }
        // `EntityAnchorArgument.listSuggestions`
        // (EntityAnchorArgument.java:51-53) hands `Anchor.BY_NAME.keySet()` to
        // `SharedSuggestionProvider.suggest(Iterable, builder)`, so the two
        // names are filtered by `matchesSubStr` and then sorted by
        // `Suggestions.create` -- which is why the map's iteration order never
        // reaches the wire and the derived catalog's order can stand in for it.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID) => {
            let remaining = remaining.to_lowercase();
            VANILLA_ENTITY_ANCHORS
                .iter()
                .filter(|entry| matches_sub_str(&remaining, entry.serialized_name))
                .map(|entry| plain(entry.serialized_name))
                .collect()
        }
        // `DimensionArgument.listSuggestions` (DimensionArgument.java:36-40)
        // hands `SharedSuggestionProvider.levels()` by identifier to
        // `suggestResource`.
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID) => {
            let contents = remaining.to_lowercase();
            world
                .command_level_keys()
                .map(|key| key.resource_id())
                .filter(|id| resource_matches(&contents, id))
                .map(plain)
                .collect()
        }
        Some(bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID)
        | Some(bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID) => {
            // StringRepresentableArgument.java:40-47: enum names use SharedSuggestionProvider.
            let values = if node.parser_type_id()
                == Some(bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID)
            {
                bbb_protocol::packets::VANILLA_TEMPLATE_ROTATIONS
            } else {
                bbb_protocol::packets::VANILLA_TEMPLATE_MIRRORS
            };
            let contents = remaining.to_lowercase();
            values
                .iter()
                .filter(|value| matches_sub_str(&contents, value))
                .map(|value| plain(*value))
                .collect()
        }
        // Everything else — messages, words, integers, and `minecraft:rotation`
        // (RotationArgument.java:13-45) — declares no default suggestions.
        _ => Vec::new(),
    }
}

/// `SharedSuggestionProvider.suggestResource`
/// (SharedSuggestionProvider.java:133-137) over the registry the argument names.
///
/// The registries a `minecraft:resource` node this server publishes resolves
/// against are `minecraft:damage_type` (`DamageCommand.java:38`),
/// `minecraft:attribute` (`AttributeCommand.java:49`),
/// `minecraft:enchantment` (`EnchantCommand.java:46`) and
/// `minecraft:worldgen/biome` (`FillBiomeCommand.java:50`).
fn registry_suggestions(
    node: &'static bbb_protocol::packets::VanillaCommandNode,
    remaining: &str,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    let CommandArgumentTemplate::Registry { registry } = argument_template(node) else {
        unreachable!("the resource parser carries a registry template");
    };
    let contents = remaining.to_lowercase();
    if registry == DAMAGE_TYPE_REGISTRY {
        return world
            .damage_types()
            .iter()
            .filter(|entry| resource_matches(&contents, &entry.resource_id))
            .map(|entry| plain(&entry.resource_id))
            .collect();
    }
    if registry == catalog::ATTRIBUTE_REGISTRY {
        return bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES
            .iter()
            .filter(|entry| resource_matches(&contents, entry.resource_id))
            .map(|entry| plain(entry.resource_id))
            .collect();
    }
    if registry == catalog::ENCHANTMENT_REGISTRY {
        return world
            .enchantments()
            .iter()
            .filter(|entry| resource_matches(&contents, &entry.resource_id))
            .map(|entry| plain(&entry.resource_id))
            .collect();
    }
    if registry == catalog::MOB_EFFECT_REGISTRY {
        return bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_TYPES
            .iter()
            .filter(|entry| resource_matches(&contents, entry.resource_id))
            .map(|entry| plain(entry.resource_id))
            .collect();
    }
    // `TimeCommand` (TimeCommand.java:56).
    if registry == "minecraft:worldgen/biome" || registry == catalog::WORLD_CLOCK_REGISTRY {
        return world
            .registry_ids(&registry)
            .iter()
            .filter(|id| resource_matches(&contents, id))
            .map(plain)
            .collect();
    }
    Vec::new()
}

fn resource_key_suggestions(
    node: &'static bbb_protocol::packets::VanillaCommandNode,
    remaining: &str,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    let CommandArgumentTemplate::Registry { registry } = argument_template(node) else {
        unreachable!("the resource-key parser carries a registry template");
    };
    if registry == catalog::ADVANCEMENT_REGISTRY {
        let contents = remaining.to_lowercase();
        let tree = world.advancement_tree();
        return (0..tree.len())
            .map(|index| tree.node(index).id())
            .filter(|id| resource_matches(&contents, id))
            .map(plain)
            .collect();
    }
    if registry == catalog::RECIPE_REGISTRY {
        let contents = remaining.to_lowercase();
        return world
            .recipe_ids()
            .iter()
            .filter(|id| resource_matches(&contents, id))
            .map(plain)
            .collect();
    }
    let ids = match registry.as_str() {
        catalog::CONFIGURED_FEATURE_REGISTRY => {
            world.startup_registry_ids("minecraft:worldgen/configured_feature")
        }
        "minecraft:worldgen/structure" | "minecraft:worldgen/template_pool" => {
            world.startup_registry_ids(&registry)
        }
        _ => return Vec::new(),
    };
    let contents = remaining.to_lowercase();
    ids.iter()
        .filter(|id| resource_matches(&contents, id))
        .map(plain)
        .collect()
}

/// `SharedSuggestionProvider.suggest` over `criteria().keySet()`
/// (`AdvancementCommands.java:54-57`, `:145-148`).
fn advancement_criterion_suggestions(
    remaining: &str,
    parsed: &ParseResult,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    let Some(ArgumentValue::Resource(id)) = parsed.argument("advancement") else {
        return Vec::new();
    };
    let tree = world.advancement_tree();
    let Some(index) = tree.index_of(id) else {
        return Vec::new();
    };
    let remaining: Vec<u16> = remaining.to_lowercase().encode_utf16().collect();
    tree.node(index)
        .criteria()
        .iter()
        .map(|(name, _)| name)
        .filter(|name| matches_java_sub_str(&remaining, &name.to_lowercase_root()))
        .map(plain)
        .collect()
}

/// The clock the two `/time` suggestion hooks run against
/// (`TimeCommand.java:75`, `:104`), which is the `of <clock>` argument or
/// `getDefaultClock` (`:207-210`). That getter throws for a dimension type
/// declaring no clock, and `getCompletionSuggestions` answers a throwing hook
/// with nothing (`com/mojang/brigadier/CommandDispatcher.java:305-308`).
fn suggestion_clock(
    parsed: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Option<String> {
    match parsed.argument("clock") {
        Some(ArgumentValue::Resource(id)) => Some(id.clone()),
        _ => world.command_dimension_type(&at.level).1.default_clock,
    }
}

/// `TimeCommand.suggestTimeMarkers` (`TimeCommand.java:114-118`).
fn time_marker_suggestions(
    remaining: &str,
    parsed: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Vec<CommandSuggestion> {
    let Some(clock) = suggestion_clock(parsed, world, at) else {
        return Vec::new();
    };
    let contents = remaining.to_lowercase();
    // `ServerClockManager.commandTimeMarkersForClock` (`ServerClockManager.java:143-145`).
    world
        .clock_catalog()
        .clock(&clock)
        .map(|entry| entry.time_markers.as_slice())
        .unwrap_or_default()
        .iter()
        .filter(|marker| marker.show_in_commands)
        .map(|marker| marker.resource_id.as_str())
        .filter(|id| resource_matches(&contents, id))
        .map(plain)
        .collect()
}

/// `TimeCommand.suggestTimelines` (`TimeCommand.java:120-129`).
fn timeline_suggestions(
    remaining: &str,
    parsed: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Vec<CommandSuggestion> {
    let Some(clock) = suggestion_clock(parsed, world, at) else {
        return Vec::new();
    };
    let contents = remaining.to_lowercase();
    world
        .clock_catalog()
        .timelines()
        .iter()
        .filter(|timeline| timeline.clock == clock)
        .map(|timeline| timeline.resource_id.as_str())
        .filter(|id| resource_matches(&contents, id))
        .map(plain)
        .collect()
}

fn resource_or_tag_suggestions(
    node: &'static bbb_protocol::packets::VanillaCommandNode,
    remaining: &str,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    let CommandArgumentTemplate::Registry { registry } = argument_template(node) else {
        unreachable!("the resource-or-tag parser carries a registry template");
    };
    let contents = remaining.to_lowercase();
    // `suggestRegistryElements` (SharedSuggestionProvider.java:63-73): the
    // tags under `#`, then the elements.
    let mut suggestions: Vec<CommandSuggestion> = world
        .biome_tags()
        .into_iter()
        .flat_map(|tags| tags.tag_ids(&registry))
        .filter(|tag| tag_matches(&contents, tag))
        .map(|tag| plain(format!("#{tag}")))
        .collect();
    let owned;
    let ids: Vec<&str> = match registry.as_str() {
        "minecraft:worldgen/biome" => {
            owned = world.registry_ids("minecraft:worldgen/biome");
            owned.iter().map(String::as_str).collect()
        }
        "minecraft:worldgen/structure" => {
            owned = world.startup_registry_ids("minecraft:worldgen/structure");
            owned.iter().map(String::as_str).collect()
        }
        "minecraft:point_of_interest_type" => bbb_facts::poi_types::VANILLA_POI_TYPES
            .iter()
            .map(|entry| entry.resource_id)
            .collect(),
        _ => Vec::new(),
    };
    suggestions.extend(
        ids.into_iter()
            .filter(|id| resource_matches(&contents, id))
            .map(plain),
    );
    suggestions
}

/// `SharedSuggestionProvider.filterResources` (SharedSuggestionProvider.java:93-107):
/// a remainder carrying a colon is matched against the whole id, and one
/// without it against either half on its own, which is what lets `gen` reach
/// `minecraft:generic` while the namespace is still unwritten.
fn resource_matches(contents: &str, id: &str) -> bool {
    if contents.contains(':') {
        return matches_sub_str(contents, id);
    }
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    matches_sub_str(contents, namespace) || matches_sub_str(contents, path)
}

/// The prefixed `suggestResource` overload
/// (SharedSuggestionProvider.java:123-127) every tag list is offered under,
/// which goes through `filterResources`'s prefix form (`:109-121`) and so is a
/// different rule from the plain one: an empty remainder offers every tag
/// unfiltered (`:112-113`), and a non-empty one is dropped outright unless it
/// shares a common prefix with `#` (`:115-119`) — which for a one-character
/// prefix means starting with it. What survives is matched with the `#` already
/// stripped off.
fn tag_matches(contents: &str, id: &str) -> bool {
    match contents.strip_prefix('#') {
        Some(stripped) => resource_matches(stripped, id),
        None => contents.is_empty(),
    }
}

/// `FunctionCommand.SUGGEST_FUNCTION` (FunctionCommand.java:52-56): the tag
/// names under a `#`, then the function names, both onto the same builder — so
/// the answer is the two lists together rather than whichever one matched.
fn function_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let contents = remaining.to_lowercase();
    let mut suggestions = Vec::new();
    let functions = world.command_functions();
    for tag in functions.tag_names() {
        if tag_matches(&contents, tag) {
            suggestions.push(plain(format!("#{tag}")));
        }
    }
    for id in functions.ids() {
        if resource_matches(&contents, id) {
            suggestions.push(plain(id));
        }
    }
    suggestions
}

/// The `suggests` `/tag <targets> remove <name>` carries
/// (TagCommand.java:39): the union of the tags every target holds, offered
/// through `SharedSuggestionProvider.suggest(Iterable, builder)` and so
/// filtered by [`matches_sub_str`] over the lowercased remainder.
///
/// `getTags` resolves `EntityArgument.getEntities(c, "targets")`
/// (TagCommand.java:48-56), which throws for an empty result
/// (EntityArgument.java:63-64) -- and `getCompletionSuggestions` swallows that
/// exception into `Suggestions.empty()`
/// (com/mojang/brigadier/CommandDispatcher.java:305-308), so a `<targets>` that
/// matches nobody offers nothing rather than offering every tag.
fn matches_java_sub_str(remaining: &[u16], value: &bbb_protocol::JavaString) -> bool {
    let mut units = value.units();
    while !units.starts_with(remaining) {
        let Some(splitter) = units.iter().position(|unit| matches!(*unit, 46 | 95 | 47)) else {
            return false;
        };
        units = &units[splitter + 1..];
    }
    true
}

fn tag_suggestions(
    remaining: &str,
    parsed: &ParseResult,
    source: &CommandSource,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Vec<CommandSuggestion> {
    let Some(ArgumentValue::Players(selector)) = parsed.argument("targets") else {
        return Vec::new();
    };
    let targets = select_damage_entity_states(selector, source, at, &world.entity_reads())
        .unwrap_or_default();
    if targets.is_empty() {
        return Vec::new();
    }
    let remaining: Vec<u16> = remaining.to_lowercase().encode_utf16().collect();
    let mut offered: Vec<&bbb_protocol::JavaString> = Vec::new();
    for target in targets {
        for tag in target.entity_tags() {
            if !offered.contains(&tag) {
                offered.push(tag);
            }
        }
    }
    offered
        .into_iter()
        .filter(|tag| matches_java_sub_str(&remaining, &tag.to_lowercase_root()))
        .map(plain)
        .collect()
}

/// `ItemParser` retains the suggestion stage reached by its parse
/// (`ItemParser.java:124-191`, `:219-266`).
fn item_argument_candidates(start: usize, remaining: &str, world: &ServerWorld) -> Vec<Candidate> {
    let item = scan_command_resource_location(remaining, 0)
        .ok()
        .filter(|(_, id)| {
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(id).is_some()
        });
    let Some((end, _)) = item else {
        let contents = remaining.to_lowercase();
        return VANILLA_ITEM_TYPES
            .iter()
            .filter(|entry| resource_matches(&contents, entry.resource_id))
            .map(|entry| Candidate::at(start, plain(entry.resource_id)))
            .collect();
    };
    if end == remaining.len() {
        return vec![Candidate::at(start + end, plain("["))];
    }
    if remaining.as_bytes().get(end) != Some(&b'[') {
        return Vec::new();
    }
    let resolver = world.item_stack_resolver();
    let context = super::item_modifier::persistent_decode_context(&resolver);
    let mut scanner = CommandItemComponentPatchScanner::new(remaining, end);
    while let Ok(Some(entry)) = scanner.next_component() {
        if let Some(value) = &entry.value {
            if bbb_registry_codec::validate_persistent_component_nbt(
                entry.component.type_id,
                entry.component.resource_id,
                value,
                context,
            )
            .is_err()
            {
                return Vec::new();
            }
        }
    }
    let Some((cursor, stage)) = scanner.suggestions() else {
        return Vec::new();
    };
    let contents = &remaining[cursor..];
    let candidates = match stage {
        CommandItemComponentSuggestions::Components { removed } => {
            let lower = contents.to_lowercase();
            let mut candidates: Vec<_> = VANILLA_DATA_COMPONENT_TYPES
                .iter()
                .filter(|entry| entry.persistent && resource_matches(&lower, entry.resource_id))
                .map(|entry| {
                    plain(format!(
                        "{}{}",
                        entry.resource_id,
                        if removed { "" } else { "=" }
                    ))
                })
                .collect();
            if !removed {
                candidates.push(plain("!"));
            }
            candidates
        }
        CommandItemComponentSuggestions::Assignment if contents.is_empty() => vec![plain("=")],
        CommandItemComponentSuggestions::Separator if contents.is_empty() => {
            vec![plain(","), plain("]")]
        }
        _ => Vec::new(),
    };
    candidates
        .into_iter()
        .map(|suggestion| Candidate::at(start + cursor, suggestion))
        .collect()
}

fn item_predicate_candidates(start: usize, remaining: &str, world: &ServerWorld) -> Vec<Candidate> {
    let units: Vec<_> = remaining.encode_utf16().collect();
    let resolver = world.item_stack_resolver();
    let head = bbb_protocol::scan_command_item_predicate_head_utf16(&units, 0)
        .ok()
        .filter(|head| super::item_predicate::resolve_head(head, &resolver).is_ok());
    let Some(head) = head else {
        let cursor = scan_command_whitespace(remaining, 0);
        let contents = &remaining[cursor..];
        let (cursor, contents) = if contents.starts_with('#') {
            let cursor = scan_command_whitespace(remaining, cursor + 1);
            (cursor, &remaining[cursor..])
        } else {
            let lower = contents.to_lowercase();
            let mut candidates: Vec<_> = VANILLA_ITEM_TYPES
                .iter()
                .filter(|entry| resource_matches(&lower, entry.resource_id))
                .map(|entry| Candidate::at(start + cursor, plain(entry.resource_id)))
                .collect();
            for literal in ["#", "*"] {
                if literal.starts_with(&lower) {
                    candidates.push(Candidate::at(start + cursor, plain(literal)));
                }
            }
            return candidates;
        };
        let lower = contents.to_lowercase();
        return world
            .registry_tag_ids("minecraft:item")
            .filter(|id| resource_matches(&lower, id))
            .map(|id| Candidate::at(start + cursor, plain(id)))
            .collect();
    };
    let opening = bbb_protocol::command_utf16_byte_cursor(
        remaining,
        bbb_protocol::scan_command_whitespace_utf16(&units, head.end()),
    );
    if opening == remaining.len() {
        return vec![Candidate::at(start + opening, plain("["))];
    }
    if remaining.as_bytes().get(opening) != Some(&b'[') {
        return Vec::new();
    }
    let mut scanner = bbb_protocol::CommandItemPredicateTestScannerUtf16::new(&units, head.end());
    while let Ok(Some(term)) = scanner.next_term() {
        if super::item_predicate::compile_test(term.test, &resolver).is_err() {
            return Vec::new();
        }
    }
    let Some((cursor, stage)) = scanner.suggestions() else {
        return Vec::new();
    };
    let cursor = bbb_protocol::command_utf16_byte_cursor(remaining, cursor);
    let contents = remaining[cursor..].to_lowercase();
    let mut suggestions = Vec::new();
    match stage {
        CommandItemPredicateSuggestions::Names { negated, closing } => {
            suggestions.extend(
                VANILLA_DATA_COMPONENT_TYPES
                    .iter()
                    .filter(|entry| entry.persistent)
                    .map(|entry| entry.resource_id)
                    .chain(
                        VANILLA_DATA_COMPONENT_PREDICATE_TYPES
                            .iter()
                            .map(|entry| entry.resource_id),
                    )
                    .chain(["minecraft:count"])
                    .filter(|id| resource_matches(&contents, id))
                    .map(plain),
            );
            for literal in [(!negated).then_some("!"), closing.then_some("]")]
                .into_iter()
                .flatten()
            {
                if literal.starts_with(&contents) {
                    suggestions.push(plain(literal));
                }
            }
        }
        CommandItemPredicateSuggestions::Terminals {
            assignment,
            subpredicate,
            separator,
        } => {
            for literal in [
                assignment.then_some("="),
                subpredicate.then_some("~"),
                separator.then_some(","),
                separator.then_some("|"),
                separator.then_some("]"),
            ]
            .into_iter()
            .flatten()
            {
                if literal.starts_with(&contents) {
                    suggestions.push(plain(literal));
                }
            }
        }
    }
    suggestions
        .into_iter()
        .map(|suggestion| Candidate::at(start + cursor, suggestion))
        .collect()
}

fn slot_suggestions<'a>(
    remaining: &str,
    names: impl Iterator<Item = &'a str>,
) -> Vec<CommandSuggestion> {
    let lower = remaining.to_lowercase();
    names
        .filter(|name| matches_sub_str(&lower, &name.to_lowercase()))
        .map(plain)
        .collect()
}

fn particle_id_suggestions(remaining: &str) -> Vec<CommandSuggestion> {
    if remaining.contains('{') {
        return Vec::new();
    }
    let contents = remaining.to_lowercase();
    VANILLA_PARTICLE_TYPES
        .iter()
        .filter(|entry| resource_matches(&contents, entry.resource_id))
        .map(|entry| plain(entry.resource_id))
        .collect()
}

/// `BlockStateParser.fillSuggestions` (BlockStateParser.java:128-141) as this
/// dispatcher's candidates: the stage that parse stopped in, re-based onto its
/// own cursor (`:140`).
///
/// The id lists are spelled here rather than in the parser because they are
/// registry lists like every other one this file offers: `suggestItem`
/// (`:344-346`) and `suggestTag` (`:340-342`) both go out through
/// `SharedSuggestionProvider.suggestResource`
/// (SharedSuggestionProvider.java:133-137, `:123-127` for the prefixed form),
/// so they are filtered by [`resource_matches`] and [`tag_matches`].
fn block_argument_candidates(
    start: usize,
    remaining: &str,
    world: &ServerWorld,
    for_testing: bool,
) -> Vec<Candidate> {
    let tags = world.tags();
    let answer = block_argument_suggestions(
        remaining,
        for_testing.then_some(tags as &dyn bbb_server_types::block_state_parse::BlockTagLookup),
    );
    let start = start + answer.start;
    let stage_remaining = remaining[answer.start..].to_lowercase();
    let texts = match answer.candidates {
        BlockArgumentCandidates::BlockIds => matching_block_ids(&stage_remaining),
        BlockArgumentCandidates::Tags => matching_block_tags(tags, &stage_remaining),
        // `suggestBlockIdOrTag` (`:348-352`) puts both lists on one builder.
        BlockArgumentCandidates::BlockIdsOrTags => {
            let mut both = matching_block_tags(tags, &stage_remaining);
            both.extend(matching_block_ids(&stage_remaining));
            both
        }
        BlockArgumentCandidates::Literals(literals) => {
            return literals
                .into_iter()
                .map(|literal| Candidate {
                    start,
                    integer: literal.integer,
                    suggestion: plain(&literal.text),
                })
                .collect()
        }
    };
    texts
        .into_iter()
        .map(|suggestion| Candidate::at(start, suggestion))
        .collect()
}

fn matching_block_ids(contents: &str) -> Vec<CommandSuggestion> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| resource_matches(contents, entry.resource_id))
        .map(|entry| plain(entry.resource_id))
        .collect()
}

/// `BlockStateParser.suggestTag` (BlockStateParser.java:340-342): every block
/// tag under its `#`, which `filterResources`'s prefixed form matches with the
/// `#` stripped off (SharedSuggestionProvider.java:109-121).
///
/// The ids are `HolderLookup.listTagIds` (HolderLookup.java:25-27) of the loaded
/// block registry, the same tags the predicate's own reader resolves against.
fn matching_block_tags(
    tags: &crate::registry_tags::RegistryTagSnapshot,
    contents: &str,
) -> Vec<CommandSuggestion> {
    let mut ids: Vec<&str> = tags
        .tag_ids("minecraft:block")
        .filter(|tag| tag_matches(contents, tag))
        .collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|tag| plain(format!("#{tag}")))
        .collect()
}

/// `SharedSuggestionProvider.suggestCoordinates`
/// (SharedSuggestionProvider.java:157-193) over the one coordinate set the
/// coordinate arguments offer.
///
/// A remainder starting with `^` offers `TextCoordinates.DEFAULT_LOCAL`
/// (`:301`) and anything else offers what `getAbsoluteCoordinates`
/// (`minecraft:vec3`) or `getRelevantCoordinates` (`minecraft:block_pos`)
/// answers -- and both are the interface default `DEFAULT_GLOBAL` (`:49-55`,
/// `:302`), because `CommandSourceStack` overrides neither coordinate hook.
/// The two arguments part company only in `validator`, the
/// `Commands.createValidator` predicate each hands the helper.
fn coordinate_suggestions(remaining: &str, validator: fn(&str) -> bool) -> Vec<CommandSuggestion> {
    let axis = if remaining.starts_with('^') { "^" } else { "~" };
    let mut candidates = Vec::new();
    if remaining.is_empty() {
        // `:164-172`.
        let full = format!("{axis} {axis} {axis}");
        if validator(&full) {
            candidates.push(axis.to_string());
            candidates.push(format!("{axis} {axis}"));
            candidates.push(full);
        }
    } else {
        // `String.split(" ")` drops the trailing empty fields, so a remainder
        // ending in a separator has as many fields as the text before it.
        let mut fields = remaining.split(' ').collect::<Vec<_>>();
        while fields.last() == Some(&"") {
            fields.pop();
        }
        match fields.as_slice() {
            // `:175-182`.
            [x] => {
                let full = format!("{x} {axis} {axis}");
                if validator(&full) {
                    candidates.push(format!("{x} {axis}"));
                    candidates.push(full);
                }
            }
            // `:183-190`.
            [x, y] => {
                let full = format!("{x} {y} {axis}");
                if validator(&full) {
                    candidates.push(full);
                }
            }
            _ => {}
        }
    }
    // `suggest(Iterable, builder)` (SharedSuggestionProvider.java:226-236)
    // filters the collected list like every other candidate list.
    let lower = remaining.to_lowercase();
    candidates
        .into_iter()
        .filter(|candidate| matches_sub_str(&lower, &candidate.to_lowercase()))
        .map(|candidate| plain(&candidate))
        .collect()
}

/// `SharedSuggestionProvider.suggest2DCoordinates`
/// (SharedSuggestionProvider.java:196-224) under
/// `Commands.createValidator(ColumnPosArgument::parse)`.
///
/// The `^` half of `ColumnPosArgument.listSuggestions` (`:59-60`) is dead: it
/// offers `TextCoordinates.DEFAULT_LOCAL`, and the validator it is handed is
/// the parse that refuses `^`, so a remainder starting there is offered
/// nothing.
fn two_axis_coordinate_suggestions(
    remaining: &str,
    parses: fn(&str) -> bool,
) -> Vec<CommandSuggestion> {
    let axis = if remaining.starts_with('^') { "^" } else { "~" };
    let mut candidates = Vec::new();
    if remaining.is_empty() {
        // SharedSuggestionProvider.java:201-208.
        let full = format!("{axis} {axis}");
        if parses(&full) {
            candidates.push(axis.to_string());
            candidates.push(full);
        }
    } else {
        // `String.split(" ")` drops the trailing empty fields, so a remainder
        // ending in a separator has as many fields as the text before it.
        let mut fields = remaining.split(' ').collect::<Vec<_>>();
        while fields.last() == Some(&"") {
            fields.pop();
        }
        // `:211-217`: only a single field is answered; a second one leaves the
        // list empty.
        if let [x] = fields.as_slice() {
            let full = format!("{x} {axis}");
            if parses(&full) {
                candidates.push(full);
            }
        }
    }
    let lower = remaining.to_lowercase();
    candidates
        .into_iter()
        .filter(|candidate| matches_sub_str(&lower, &candidate.to_lowercase()))
        .map(|candidate| plain(&candidate))
        .collect()
}

fn loot_registry_suggestions(
    registry: &str,
    remaining: &str,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    let contents = remaining.to_lowercase();
    world
        .loot_elements(registry)
        .iter()
        .filter(|element| resource_matches(&contents, &element.id))
        .map(|element| plain(&element.id))
        .collect()
}

/// `ObjectiveArgument.listSuggestions` (ObjectiveArgument.java:57-64).
fn objective_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let remaining: Vec<u16> = remaining.to_lowercase().encode_utf16().collect();
    world
        .scoreboard()
        .objectives()
        .iter()
        .filter(|objective| matches_java_sub_str(&remaining, &objective.name.to_lowercase_root()))
        .map(|objective| plain(&objective.name))
        .collect()
}

fn str_name_suggestions<'a>(
    remaining: &str,
    names: impl Iterator<Item = &'a str>,
) -> Vec<CommandSuggestion> {
    let pattern = remaining.to_lowercase();
    names
        .filter(|name| matches_sub_str(&pattern, &name.to_lowercase()))
        .map(plain)
        .collect()
}

fn java_name_suggestions<'a>(
    remaining: &str,
    names: impl Iterator<Item = &'a bbb_protocol::JavaString>,
) -> Vec<CommandSuggestion> {
    let remaining: Vec<u16> = remaining.to_lowercase().encode_utf16().collect();
    names
        .filter(|name| matches_java_sub_str(&remaining, &name.to_lowercase_root()))
        .map(plain)
        .collect()
}

/// `TriggerCommand.suggestObjectives` (TriggerCommand.java:70-87).
fn trigger_objective_suggestions(
    remaining: &str,
    source: &CommandSource,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    let reads = world.entity_reads();
    let Some(entity) = source_entity_state(source, &reads) else {
        return Vec::new();
    };
    let remaining: Vec<u16> = remaining.to_lowercase().encode_utf16().collect();
    let board = world.scoreboard();
    entity.with_scoreboard_name(|owner| {
        board
            .objectives_for_criteria("trigger")
            .filter(|objective| {
                board
                    .score(owner, &objective.name)
                    .is_some_and(|score| !score.locked)
                    && matches_java_sub_str(&remaining, &objective.name.to_lowercase_root())
            })
            .map(|objective| plain(&objective.name))
            .collect()
    })
}

/// `ScoreboardCommand.suggestTriggers` (ScoreboardCommand.java:276, `:399-424`).
fn enable_trigger_suggestions(
    remaining: &str,
    parsed: &ParseResult,
    source: &CommandSource,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Vec<CommandSuggestion> {
    let Some(ArgumentValue::ScoreHolders(pattern)) = parsed.argument("targets") else {
        return Vec::new();
    };
    let Some(targets) = score_holder_names_for_suggest(pattern, source, world, at) else {
        return Vec::new();
    };
    let remaining: Vec<u16> = remaining.to_lowercase().encode_utf16().collect();
    let board = world.scoreboard();
    board
        .objectives_for_criteria("trigger")
        .filter(|objective| {
            targets.iter().any(|target| {
                board
                    .score(target, &objective.name)
                    .is_none_or(|score| score.locked)
            }) && matches_java_sub_str(&remaining, &objective.name.to_lowercase_root())
        })
        .map(|objective| plain(&objective.name))
        .collect()
}

/// `AttributeCommand.getAttributeModifiers` (AttributeCommand.java:167-171,
/// `:188-195`, `:289-292`).
fn attribute_modifier_suggestions(
    remaining: &str,
    parsed: &ParseResult,
    source: &CommandSource,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Vec<CommandSuggestion> {
    let Some(ArgumentValue::Players(selector)) = parsed.argument("target") else {
        return Vec::new();
    };
    let Some(ArgumentValue::Resource(attribute_id)) = parsed.argument("attribute") else {
        return Vec::new();
    };
    let Some(entry) = bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES
        .iter()
        .find(|entry| entry.resource_id == attribute_id)
    else {
        return Vec::new();
    };
    let reads = world.entity_reads();
    let Ok(targets) = select_damage_entity_states(selector, source, at, &reads) else {
        return Vec::new();
    };
    if targets.len() != 1 {
        return Vec::new();
    }
    let map = match targets[0] {
        CommandDamageEntityRef::Player(player) => player.attributes,
        CommandDamageEntityRef::Stored(stored) => {
            let Some(living) = stored.entity.living() else {
                return Vec::new();
            };
            living.attributes()
        }
    };
    let Some(instance) = map.instance(entry.id) else {
        return Vec::new();
    };
    let contents = remaining.to_lowercase();
    instance
        .modifiers()
        .iter()
        .map(|modifier| modifier.id.as_str())
        .filter(|id| resource_matches(&contents, id))
        .map(plain)
        .collect()
}

fn dialog_id_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let contents = remaining.to_lowercase();
    world
        .registry_ids("minecraft:dialog")
        .iter()
        .filter(|id| resource_matches(&contents, id))
        .map(plain)
        .collect()
}

fn schedule_id_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let remaining: Vec<u16> = remaining.to_lowercase().encode_utf16().collect();
    let mut ids = world.scheduled_event_ids();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .filter(|id| matches_java_sub_str(&remaining, &id.to_lowercase_root()))
        .map(plain)
        .collect()
}

/// `StopwatchCommand.SUGGEST_STOPWATCHES` (StopwatchCommand.java:25-27):
/// `SharedSuggestionProvider.suggestResource` over current ids.
fn stopwatch_id_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let contents = remaining.to_lowercase();
    world
        .stopwatch_ids()
        .filter(|id| resource_matches(&contents, id))
        .map(plain)
        .collect()
}

/// `RandomCommand.suggestRandomSequence` (RandomCommand.java:127-131):
/// `SharedSuggestionProvider.suggest` over each live key's `toString()`, not
/// `suggestResource`, so a remainder is matched against the whole id.
fn random_sequence_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let remaining = remaining.to_lowercase();
    world
        .random_sequence_ids()
        .iter()
        .filter(|key| matches_sub_str(&remaining, &key.to_lowercase()))
        .map(plain)
        .collect()
}

fn boss_bar_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let remaining = remaining.to_lowercase();
    world
        .boss_bar_ids()
        .filter(|id| resource_matches(&remaining, &id.to_lowercase()))
        .map(plain)
        .collect()
}

fn command_storage_suggestions(remaining: &str, world: &ServerWorld) -> Vec<CommandSuggestion> {
    let remaining = remaining.to_lowercase();
    world
        .storage_keys()
        .iter()
        .filter(|id| resource_matches(&remaining, &id.to_lowercase()))
        .map(plain)
        .collect()
}

/// Custom `.suggests(..)` overrides dispatched ahead of parser defaults
/// (`ArgumentCommandNode.java:70-72`). Returns `None` when the node relies on
/// its argument parser's built-in `listSuggestions`.
fn custom_suggestions_override(
    child: u16,
    remaining: &str,
    parsed: &ParseResult,
    source: &CommandSource,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Option<Vec<CommandSuggestion>> {
    let path = catalog::canonical_path(child);
    let node = catalog::node(child);
    // `OpCommand.register` (OpCommand.java:24-31).
    if path.as_slice() == ["op", "targets"] {
        let lists = world.admin_lists();
        return Some(str_name_suggestions(
            remaining,
            world
                .command_players()
                .filter(|player| !lists.operators.contains_key(&player.id))
                .map(|player| player.name),
        ));
    }
    // `DeOpCommands.register` (DeOpCommands.java:24).
    if path.as_slice() == ["deop", "targets"] {
        return Some(java_name_suggestions(
            remaining,
            world
                .admin_lists()
                .operators
                .values()
                .map(|entry| &entry.user.name),
        ));
    }
    // `WhitelistCommand.register` (WhitelistCommand.java:43-55).
    if path.as_slice() == ["whitelist", "add", "targets"] {
        let lists = world.admin_lists();
        return Some(str_name_suggestions(
            remaining,
            world
                .command_players()
                .filter(|player| !lists.allowlist.contains_key(&player.id))
                .map(|player| player.name),
        ));
    }
    // `WhitelistCommand.register` (WhitelistCommand.java:63).
    if path.as_slice() == ["whitelist", "remove", "targets"] {
        return Some(java_name_suggestions(
            remaining,
            world
                .admin_lists()
                .allowlist
                .values()
                .map(|user| &user.name),
        ));
    }
    // `PardonCommand.register` (PardonCommand.java:24).
    if path.as_slice() == ["pardon", "targets"] {
        return Some(java_name_suggestions(
            remaining,
            world
                .admin_lists()
                .player_bans
                .values()
                .map(|entry| &entry.user.name),
        ));
    }
    // `PardonIpCommand.register` (PardonIpCommand.java:24).
    if path.as_slice() == ["pardon-ip", "target"] {
        return Some(str_name_suggestions(
            remaining,
            world.admin_lists().ip_bans.keys().map(String::as_str),
        ));
    }
    if path.as_slice() == ["trigger", "objective"] {
        return Some(trigger_objective_suggestions(remaining, source, world));
    }
    if path.as_slice() == ["scoreboard", "players", "enable", "targets", "objective"] {
        return Some(enable_trigger_suggestions(
            remaining, parsed, source, world, at,
        ));
    }
    if matches!(
        path.as_slice(),
        [
            "attribute",
            "target",
            "attribute",
            "modifier",
            "remove",
            "id"
        ] | [
            "attribute",
            "target",
            "attribute",
            "modifier",
            "value",
            "get",
            "id"
        ]
    ) {
        return Some(attribute_modifier_suggestions(
            remaining, parsed, source, world, at,
        ));
    }
    // PlaceCommand.java:57-60: the template manager supplies resource suggestions.
    if path.as_slice() == ["place", "template", "template"] {
        let contents = remaining.to_lowercase();
        return Some(
            world
                .template_ids()
                .iter()
                .filter(|id| resource_matches(&contents, id))
                .map(plain)
                .collect(),
        );
    }
    if matches!(
        path.as_slice(),
        ["bossbar", "remove" | "set" | "get", "id"]
            | ["execute", "store", "result" | "success", "bossbar", "id"]
    ) && node.suggestions == Some("minecraft:ask_server")
    {
        return Some(boss_bar_suggestions(remaining, world));
    }
    // `SuggestionProviders.AVAILABLE_SOUNDS` (SuggestionProviders.java:20-22):
    // `suggestResource` over `getAvailableSounds`, which is every key of
    // `BuiltInRegistries.SOUND_EVENT` (CommandSourceStack.java:517-519).
    if node.suggestions == Some("minecraft:summonable_entities") {
        let contents = remaining.to_lowercase();
        return Some(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPES
                .iter()
                .filter(|entry| entry.can_summon && resource_matches(&contents, entry.resource_id))
                .map(|entry| plain(entry.resource_id))
                .collect(),
        );
    }
    if node.suggestions == Some("minecraft:available_sounds") {
        let contents = remaining.to_lowercase();
        return Some(
            VANILLA_SOUND_EVENT_RESOURCE_IDS
                .iter()
                .filter(|id| resource_matches(&contents, id))
                .map(|id| plain(*id))
                .collect(),
        );
    }
    if node.suggestions == Some("minecraft:ask_server")
        && node.parser == Some("minecraft:resource_location")
        && matches!(path.first(), Some(&"data" | &"execute" | &"function"))
        && path.windows(2).any(|pair| pair[0] == "storage")
    {
        return Some(command_storage_suggestions(remaining, world));
    }
    // `DebugCommand.java:64` shares `FunctionCommand.SUGGEST_FUNCTION`.
    if matches!(
        path.as_slice(),
        ["function", "name"]
            | ["execute", "if" | "unless", "function", "name"]
            | ["schedule", "function", "function"]
            | ["debug", "function", "name"]
    ) {
        return Some(function_suggestions(remaining, world));
    }
    if path.as_slice() == ["schedule", "clear", "function"] {
        return Some(schedule_id_suggestions(remaining, world));
    }
    if matches!(
        path.as_slice(),
        [
            "advancement",
            "grant" | "revoke",
            "targets",
            "only",
            "advancement",
            "criterion"
        ]
    ) {
        return Some(advancement_criterion_suggestions(remaining, parsed, world));
    }
    if path.as_slice() == ["tag", "targets", "remove", "name"] {
        return Some(tag_suggestions(remaining, parsed, source, world, at));
    }
    if matches!(
        path.as_slice(),
        ["time", "set", "timemarker"] | ["time", "of", "clock", "set", "timemarker"]
    ) {
        return Some(time_marker_suggestions(remaining, parsed, world, at));
    }
    if matches!(
        path.as_slice(),
        ["time", "query", "timeline"] | ["time", "of", "clock", "query", "timeline"]
    ) {
        return Some(timeline_suggestions(remaining, parsed, world, at));
    }
    // `DataPackCommand.SELECTED_PACKS` (DataPackCommand.java:79-81) is what
    // `disable` and the `after` / `before` anchors offer; `UNSELECTED_PACKS`
    // (`:82-95`) is what `enable` offers, minus the packs whose requested
    // features the world does not have.
    if matches!(
        path.as_slice(),
        ["datapack", "disable", "name"]
            | ["datapack", "enable", "name", "after" | "before", "existing"]
    ) {
        let packs = world.command_datapacks();
        return Some(pack_suggestions(
            remaining,
            packs.selected.iter().map(|pack| pack.id.as_str()),
        ));
    }
    if path.as_slice() == ["datapack", "enable", "name"] {
        let packs = world.command_datapacks();
        return Some(pack_suggestions(
            remaining,
            packs
                .available
                .iter()
                .filter(|pack| packs.features_available(pack) && !packs.is_selected(&pack.id))
                .map(|pack| pack.id.as_str()),
        ));
    }
    if matches!(
        path.as_slice(),
        ["random", "value", "range", "sequence"]
            | ["random", "roll", "range", "sequence"]
            | ["random", "reset", "sequence"]
    ) {
        return Some(random_sequence_suggestions(remaining, world));
    }
    if matches!(
        path.as_slice(),
        ["stopwatch", "query" | "restart" | "remove", "id"]
            | ["execute", "if" | "unless", "stopwatch", "id"]
    ) {
        return Some(stopwatch_id_suggestions(remaining, world));
    }
    let candidates: &[&str] = match path.as_slice() {
        // `DEFAULT_TICKRATE` is `String.valueOf(20)` (TickCommand.java:18),
        // offered at `:28-29`.
        ["tick", "rate", "rate"] => &["20"],
        // TickCommand.java:38-39.
        ["tick", "step", "time"] => &["1t", "1s"],
        // TickCommand.java:48-49.
        ["tick", "sprint", "time"] => &["60s", "1d", "3d"],
        _ => return None,
    };
    let remaining = remaining.to_lowercase();
    Some(
        candidates
            .iter()
            .filter(|candidate| matches_sub_str(&remaining, &candidate.to_lowercase()))
            .map(|candidate| plain(*candidate))
            .collect(),
    )
}

/// The two `/datapack` pack lists, which go out escaped
/// (`StringArgumentType::escapeIfRequired`) and are then filtered like every
/// other `SharedSuggestionProvider.suggest(Stream, builder)`
/// (SharedSuggestionProvider.java:238-242).
fn pack_suggestions<'a>(
    remaining: &str,
    ids: impl Iterator<Item = &'a str>,
) -> Vec<CommandSuggestion> {
    let remaining = remaining.to_lowercase();
    ids.map(escape_if_required)
        .filter(|id| matches_sub_str(&remaining, &id.to_lowercase()))
        .map(plain)
        .collect()
}

/// What `EntityArgument.listSuggestions` (EntityArgument.java:127-146) and
/// `GameProfileArgument.listSuggestions` (GameProfileArgument.java:71-86)
/// answer.
///
/// Both build an `EntitySelectorParser` over what is left to read, swallow
/// whatever it throws, and then call `fillSuggestions`
/// (EntitySelectorParser.java:541-543). The answer is therefore whichever hook
/// the parse left behind in `this.suggestions`, not what the parse returned.
/// `parse` (:457-473) installs `suggestNameOrSelector` (:484-491) before it
/// reads anything, and what the input does next decides which hook is left
/// standing:
///
/// * nothing typed at all leaves that first hook in place, which offers the
///   online names and then the six forms;
/// * a name moves `parseNameOrUUID` (:262-264) onto `suggestName` (:493-497),
///   which offers the names and no selector form;
/// * an `@` from a source without `COMMANDS_ENTITY_SELECTORS` throws at
///   :461-463 before any of that, so the first hook is still the one that runs
///   — but with `allowSelectors` false it adds no selector form either, which
///   leaves the names alone again;
/// * an `@` from a source that has it reaches `parseSelector` (:195-197),
///   which installs `suggestSelector` (:499-504) — the six forms, and not the
///   names — and keeps it until a type letter is read.
///
/// Once the letter is read, [`selector::option_candidates`] answers instead.
fn names_or_selector_forms(
    remaining: &str,
    permission: CommandPermission,
    world: &ServerWorld,
) -> Vec<CommandSuggestion> {
    // `EntitySelectorParser.allowSelectors`
    // (EntitySelectorParser.java:113-115) over `LevelBasedPermissionSet`
    // (LevelBasedPermissionSet.java:14-20), which grants
    // `COMMANDS_ENTITY_SELECTORS` from `GAMEMASTERS` up (:18).
    let allow_selectors = permission >= CommandPermission::GameMaster;
    if let Some(selector) = remaining.strip_prefix('@') {
        if allow_selectors {
            return match selector.chars().next() {
                // `parseSelector` refuses a missing type letter (:198-200) and
                // an unknown one (:213-215) with `suggestSelector` still in
                // place, so both answer with the six forms.
                Some(letter) if SelectorKind::from_letter(letter).is_some() => Vec::new(),
                _ => selector_forms(),
            };
        }
    }
    let pattern = remaining.to_lowercase();
    // `SharedSuggestionProvider.suggest` (:226-236) over
    // `getOnlinePlayerNames`, which both argument types pass in as the `names`
    // consumer (EntityArgument.java:139-141, GameProfileArgument.java:82). The
    // entity argument concatenates `getSelectedEntities` onto that list when it
    // is not players-only, and a server source contributes nothing there: only
    // the client overrides the empty default (SharedSuggestionProvider.java:39-41).
    let mut suggestions = world
        .command_players()
        .filter(|player| matches_sub_str(&pattern, &player.name.to_lowercase()))
        .map(|player| plain(player.name))
        .collect::<Vec<_>>();
    if remaining.is_empty() && allow_selectors {
        // `suggestNameOrSelector` (:484-491) adds the forms after the names,
        // onto the same builder and therefore under the same range.
        suggestions.extend(selector_forms());
    }
    suggestions
}

/// `EntitySelectorParser.fillSelectorSuggestions`
/// (EntitySelectorParser.java:475-482): six candidates, each with the tooltip
/// its own line names.
///
/// None of them is filtered against what is left to read — `builder.suggest`
/// is called unconditionally, so the only one that can still fall away is the
/// one equal to the remainder, which the caller drops for every candidate
/// alike.
fn selector_forms() -> Vec<CommandSuggestion> {
    SelectorKind::FORMS
        .into_iter()
        .map(|(letter, _, tooltip)| {
            let tooltip = component(translatable(tooltip, Vec::new()));
            CommandSuggestion {
                text: format!("@{letter}").into(),
                // The three tooltip fields are one Component and its two
                // projections, taken from the component itself rather than
                // spelled a second time here, so the suggestion that goes out
                // is the one a client decoding it reads back.
                tooltip: Some(tooltip.text().to_string()),
                tooltip_styled: Some(tooltip.styled().to_vec()),
                tooltip_component: Some(tooltip.component().clone()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Suggestions.create` (Suggestions.java:80-103) over candidates that do
    /// not share a start: the answer carries the least one, and
    /// `Suggestion.expand` (Suggestion.java:84-100) rewrites every later
    /// candidate into that range by prepending the input between the two
    /// starts.
    ///
    /// No node of the published tree puts a block argument beside a sibling, so
    /// the block stages reach [`completions`] as the only child's answer and it
    /// is this merge, not that path, that widens a range.
    #[test]
    fn a_merged_range_rewrites_the_candidates_that_start_later() {
        let command = "setblock 0 0 0 chest";
        let answer = merged(
            command,
            vec![
                Candidate::at(15, plain("minecraft:chest")),
                Candidate {
                    start: 20,
                    integer: None,
                    suggestion: plain("["),
                },
                Candidate {
                    start: 20,
                    integer: Some(3),
                    suggestion: plain("3"),
                },
            ],
        );
        assert_eq!((answer.start, answer.length), (15, 5));
        assert_eq!(
            answer
                .suggestions
                .iter()
                .map(|suggestion| &suggestion.text)
                .collect::<Vec<_>>(),
            ["chest3", "chest[", "minecraft:chest"]
        );
    }
}
