//! The Brigadier walk over the derived command tree.
//!
//! The walk preserves `CommandDispatcher.parseNodes`'s relevant-children rule
//! (an exact literal wins, otherwise only argument children are tried), the
//! one-separator skip, candidate ordering, and `Commands.getParseException`
//! (Commands.java:469-479) choosing
//! between a single argument error, the unknown-command error and the
//! unknown-argument error. The same walk exists client-side over a received
//! tree (`relevant_children` and `parse_nodes` in
//! `bbb_client_world::commands::dispatcher`); this one walks the derived catalog and
//! carries the argument values an executor needs, and the two cannot share code
//! because the client crate is not a dependency of the server.
//!
//! Every refusal is the message `BrigadierExceptions`
//! (BrigadierExceptions.java:10-80) or the argument type itself reports, so it
//! travels as a translation key the client resolves rather than as English.

mod coordinates;
mod execute;
mod message;
pub(super) mod particle;
mod resources;
mod scoreboard;
mod team;

pub(in crate::commands) use coordinates::*;
pub(in crate::commands) use execute::*;
use resources::{read_resource_or_tag, read_resource_or_tag_key};
pub(in crate::commands) use scoreboard::*;
pub(in crate::commands) use team::*;

use bbb_protocol::{
    read_command_string_value_utf16, scan_command_number_lexeme_utf16,
    scan_command_resource_location_utf16, scan_command_token_utf16,
    scan_command_unquoted_string_utf16, CommandSyntaxErrorKind, CommandSyntaxErrorUtf16,
    JavaString,
};

use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::entity_anchors::{EntityAnchor, VANILLA_ENTITY_ANCHORS};
use bbb_protocol::game_types::{vanilla_game_type_for_serialized_name, GameType};
use bbb_protocol::heightmap_types::{HeightmapUsage, VANILLA_HEIGHTMAP_TYPES};
use bbb_protocol::scoreboard_display_slots::ScoreboardDisplaySlot;
use bbb_protocol::{
    collapsed_component_nbt, component_literal_nbt,
    packets::{
        CommandArgumentTemplate, CommandStringMode, VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID, VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID, VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ROTATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID, VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID, VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID, VANILLA_COMMAND_TREE_ROOT,
    },
    parse_command_nbt_path_value_prefix_utf16, parse_command_snbt_value_prefix_utf16,
    CommandNbtPath, CommandNbtPathErrorKind, NbtSummaryValue, SnbtParseFailure,
};

use bbb_server_datapack::damage::DAMAGE_TYPE_REGISTRY;
use bbb_server_types::block_state_parse::{self, BlockStateParseError};

use super::dialog::read_dialog;
use super::item_modifier::{
    read_item_component_patch, read_loot_modifier, CommandComponentPatchEntry, CommandItemModifier,
};
use super::loot_table::read_loot_table;
use super::properties::argument_template;
use super::selector_options::{self, SelectorOptions, SelectorOrder};
use super::types::CommandBlockInput;
use super::{catalog, translatable, CommandPermission};
use crate::world::ServerWorld;

/// One node a parse read, with the input span it covers.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ParsedNode {
    pub index: u16,
    pub start: usize,
    pub end: usize,
    pub value: Option<ArgumentValue>,
}

/// The value an argument node read.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ArgumentValue {
    Boolean(bool),
    Integer(i32),
    /// `brigadier:string`, in either the word or the greedy mode.
    Text(JavaString),
    /// `minecraft:message`, which is also the only signable argument type
    /// (`SignableCommand.of`, SignableCommand.java:18-31).
    Message(message::CommandMessage),
    /// `minecraft:entity`, resolved against the player list when the command
    /// runs, the way `EntityArgument.getPlayers` resolves it.
    Players(Selector),
    /// `minecraft:game_profile`, resolved against the name cache instead.
    Profiles(Selector),
    /// `minecraft:time`, already converted to ticks.
    Time(i32),
    /// `brigadier:float`.
    Float(f32),
    /// `brigadier:double`.
    Double(f64),
    /// `minecraft:vec2` (`Vec2Argument.parse`, Vec2Argument.java:43-58).
    Vec2(Coordinates),
    /// `minecraft:gamemode` (`GameModeArgument`, GameModeArgument.java:27-35).
    GameMode(GameType),
    /// `minecraft:resource` (`ResourceArgument`, ResourceArgument.java:113-119),
    /// already resolved: the parse refuses an id the registry does not hold, so
    /// what survives is a registry id an executor can look up again.
    Resource(String),
    /// `minecraft:vec3` (`Vec3Argument.parse`, Vec3Argument.java:46-48). The
    /// argument is a `Coordinates`, not a position: `getVec3`
    /// (`:38-40`) applies the source only when the command runs.
    Coordinates(Coordinates),
    /// `minecraft:rotation` (`RotationArgument.parse`,
    /// RotationArgument.java:25-40), which is a `Coordinates` too -- the same
    /// `WorldCoordinates` record read back through `getRotation` rather than
    /// through `getPosition`. It is a separate variant because the two argument
    /// types are separate parsers with separate refusals, and because an
    /// executor asking for one must not be handed the other.
    Rotation(Coordinates),
    /// `minecraft:entity_anchor` (`EntityAnchorArgument.parse`,
    /// EntityAnchorArgument.java:38-48).
    EntityAnchor(EntityAnchor),
    /// `minecraft:swizzle` (`SwizzleArgument.parse`, SwizzleArgument.java:27-47).
    Swizzle {
        x: bool,
        y: bool,
        z: bool,
    },
    /// `minecraft:objective` (`ObjectiveArgument.parse`,
    /// ObjectiveArgument.java:52-54): the word, unresolved. `getObjective`
    /// (`:32-41`) is what refuses a name no scoreboard holds, and it runs when
    /// the command does.
    Objective(String),
    /// `minecraft:objective_criteria` (`ObjectiveCriteriaArgument.parse`,
    /// ObjectiveCriteriaArgument.java:40-52), already resolved: the parse
    /// refuses a name `ObjectiveCriteria.byName` does not answer.
    ObjectiveCriteria(String),
    /// `minecraft:scoreboard_slot` (`ScoreboardSlotArgument.parse`,
    /// ScoreboardSlotArgument.java:35-43).
    ScoreboardSlot(ScoreboardDisplaySlot),
    /// `minecraft:score_holder` (`ScoreHolderArgument.parse`,
    /// ScoreHolderArgument.java:92-161), which is a closure over a source and a
    /// wildcard supplier rather than a set of names.
    ScoreHolders(ScoreHolderPattern),
    /// `minecraft:operation` (`OperationArgument.parse`,
    /// OperationArgument.java:34-46).
    Operation(ScoreboardOperation),
    /// `minecraft:style` (`StyleArgument`, StyleArgument.java:27-29), kept as
    /// the validated NBT the styled number format stores.
    Style(NbtSummaryValue),
    /// `minecraft:team` (`TeamArgument.parse`, TeamArgument.java:40-42): the
    /// word, unresolved. `getTeam` (`:29-38`) is what refuses a name no
    /// scoreboard holds, and it runs when the command does.
    Team(String),
    /// `minecraft:color` (`ColorArgument.parse`, ColorArgument.java:35-43),
    /// already resolved to one of the sixteen colours or `RESET`.
    Color(ChatFormatting),
    /// `minecraft:hex_color` (`HexColorArgument.parse`,
    /// HexColorArgument.java:35-50), expanded to opaque ARGB.
    HexColor(i32),
    /// `minecraft:heightmap` (`HeightmapTypeArgument`, HeightmapTypeArgument.java:11-18):
    /// a live-world heightmap id. Worldgen-only types are not spellable.
    Heightmap(i32),
    /// `minecraft:resource_or_tag` (`ResourceOrTagArgument.parse`,
    /// ResourceOrTagArgument.java:66-89). `tag` is the `#` branch.
    ResourceOrTag {
        tag: bool,
        id: String,
    },
    /// `minecraft:function` (`FunctionArgument.parse`,
    /// FunctionArgument.java:31-74), which reads the name and nothing else: the
    /// library lookup is in `create`/`unwrapToCollection` (`:76-87`), which run
    /// when the command does.
    Function {
        /// A leading `#`, the whole difference between the two branches of
        /// `parse` (`:32`).
        tag: bool,
        /// What `Identifier.read` accepted, already qualified.
        id: String,
    },
    /// `minecraft:block_pos` (`BlockPosArgument.parse`,
    /// BlockPosArgument.java:61-63), which is a `Coordinates` like
    /// [`Self::Coordinates`] is: `getLoadedBlockPos` (`:37-46`) applies the
    /// source and checks the level only when the command runs.
    ///
    /// It is a variant of its own rather than a second `minecraft:vec3`
    /// because the two grammars differ. `WorldCoordinates.parseInt`
    /// (WorldCoordinates.java:39-57) reads whole numbers through
    /// `WorldCoordinate.parseInt` (WorldCoordinate.java:41-59), which never
    /// centre-corrects and refuses a missing axis with
    /// `argument.pos.missing.int` rather than `argument.pos.missing.double`.
    BlockPos(Coordinates),
    /// `minecraft:column_pos` (`ColumnPosArgument.parse`,
    /// ColumnPosArgument.java:34-49), the two-axis form of [`Self::BlockPos`].
    ///
    /// It is a `Coordinates` because vanilla's is: `parse` builds a
    /// `WorldCoordinates` whose vertical axis is a relative zero it never read
    /// (`:43`), and `getColumnPos` (`:29-32`) resolves that record as a block
    /// position and keeps only X and Z.
    ColumnPos(Coordinates),
    /// `minecraft:block_state` (`BlockStateArgument.parse`,
    /// BlockStateArgument.java:30-33), already resolved to the `BlockInput` the
    /// command places.
    BlockState(CommandBlockInput),
    /// `minecraft:block_predicate` (`BlockPredicateArgument.parse`,
    /// BlockPredicateArgument.java:46-52), whose two shapes are the two the
    /// `Either` there maps.
    BlockPredicate(CommandBlockPredicate),
    /// `minecraft:dimension` (`DimensionArgument.parse`,
    /// DimensionArgument.java:31-33): the identifier `Identifier.read` accepted,
    /// already qualified. Existence is not this parse's question —
    /// `DimensionArgument.getDimension` (`:51-59`) looks the id up when the
    /// `/execute in` modifier runs.
    Dimension(String),
    /// `minecraft:resource_location` (`IdentifierArgument.parse`,
    /// IdentifierArgument.java:23-25): `Identifier.read` and nothing else, so
    /// the id names no registry and is checked against none. `/playsound`
    /// wraps it in a direct `SoundEvent` holder (PlaySoundCommand.java:161)
    /// and `/stopsound` puts it on the wire as it is (StopSoundCommand.java:50).
    Identifier(String),
    /// `minecraft:nbt_compound_tag` (`CompoundTagArgument.parse`,
    /// CompoundTagArgument.java:26-28), which is
    /// `TagParser.parseCompoundAsArgument` (TagParser.java:83-86). Always a
    /// [`NbtSummaryValue::Compound`].
    CompoundTag(NbtSummaryValue),
    /// `minecraft:nbt_tag` (`NbtTagArgument`, NbtTagArgument.java:12-18).
    Tag(NbtSummaryValue),
    /// `minecraft:nbt_path` (`NbtPathArgument.parse`,
    /// NbtPathArgument.java:62-84).
    NbtPath(CommandNbtPath),
    /// `minecraft:component` (`ComponentArgument`, ComponentArgument.java:23-32),
    /// already validated by `ComponentSerialization.CODEC`.
    Component(NbtSummaryValue),
    /// `minecraft:int_range` (`RangeArgument.Ints.parse`,
    /// RangeArgument.java:45-47): `MinMaxBounds.Ints.fromReader`
    /// (MinMaxBounds.java:306-316), where a missing half means unbounded.
    IntRange {
        min: Option<i32>,
        max: Option<i32>,
    },
    /// `minecraft:float_range` (`RangeArgument.Floats.parse`,
    /// RangeArgument.java:28-30): `MinMaxBounds.Doubles.fromReader`
    /// (MinMaxBounds.java:239-250).
    FloatRange {
        min: Option<f64>,
        max: Option<f64>,
    },
    /// `minecraft:item_stack` (`ItemArgument.parse`, ItemArgument.java:26-28)
    /// through `ItemParser.readItem` (`ItemParser.java:134-141`) and the
    /// optional `[...]` component patch (`:128-131`, `:143-191`). Every patch
    /// entry is already decoded, so what survives is exactly the list
    /// `ItemInput.createItemStack` (`ItemInput.java:21-25`) applies.
    ItemStack {
        item_id: i32,
        resource_id: String,
        patch: Vec<CommandComponentPatchEntry>,
    },
    /// `minecraft:item_slot` (`SlotArgument.parse`, SlotArgument.java:35-45)
    /// through `SlotRanges.nameToIds` (`SlotRanges.java:84-86`). A range name
    /// (`container.*`) fails `slot.only_single_allowed`.
    ItemSlot(i32),
    ItemSlots(&'static [i32]),
    ItemPredicate(super::item_predicate::CommandItemPredicate),
    /// `minecraft:loot_modifier` (`ResourceOrIdArgument.lootModifier`,
    /// ResourceOrIdArgument.java:95-97), already resolved: a name is looked up
    /// in `Registries.ITEM_MODIFIER` and an inline value is decoded by
    /// `LootItemFunctions.ROOT_CODEC` while the argument parses, so what
    /// survives is a function `ItemCommands.applyModifier` can run.
    LootModifier(CommandItemModifier),
    /// `minecraft:loot_table` (`ResourceOrIdArgument.lootTable`,
    /// ResourceOrIdArgument.java:87-92), already resolved.
    LootTable(super::loot_table::CommandLootTable),
    LootPredicate(super::loot_predicate::CommandLootPredicate),
    /// `minecraft:dialog` (`ResourceOrIdArgument.dialog`,
    /// ResourceOrIdArgument.java:111-117), already resolved: a name is looked
    /// up in `Registries.DIALOG` and an inline value is decoded by
    /// `Dialog.DIRECT_CODEC` while the argument parses.
    Dialog(bbb_protocol::packets::ShowDialog),
    /// `minecraft:particle` (`ParticleArgument.parse`, ParticleArgument.java:50-52),
    /// with the options already encoded the way `ParticleTypes.STREAM_CODEC`
    /// writes them after the type id.
    Particle {
        type_id: i32,
        resource_id: String,
        raw_options: Vec<u8>,
    },
}

pub(super) use bbb_server_types::block_state_parse::{
    block_argument_suggestions, BlockArgumentCandidates, BlockPredicate as CommandBlockPredicate,
};

/// What an entity or game-profile argument read, before it is resolved.
///
/// `includes_entities` and `self_selector` are the flags
/// `EntitySelectorParser` sets while parsing, read back from the shared prefix
/// parser rather than recomputed: they are what the two argument types check
/// before they accept a selector at all. A game-profile name never ran that
/// parser (`GameProfileArgument.parse`, GameProfileArgument.java:55-67), so both
/// flags stay at the value a parser that never ran left them.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Selector {
    pub form: SelectorForm,
    pub includes_entities: bool,
    pub self_selector: bool,
    /// The `[..]` option list, as [`selector_options`] read it.
    pub options: SelectorOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SelectorForm {
    /// A name rather than a selector. `minecraft:entity` reads it with
    /// `parseNameOrUUID` (EntitySelectorParser.java:261-283), which also accepts
    /// a UUID; `minecraft:game_profile` reads it as a bare word instead
    /// (GameProfileArgument.java:56-62), which is why the two are separate
    /// readers over the same form.
    Named(JavaString),
    /// `parseSelector` (EntitySelectorParser.java:195-259).
    Kind(SelectorKind),
}

impl Selector {
    /// `EntitySelector.getMaxResults`
    /// (commands/arguments/selector/EntitySelector.java:99-101).
    ///
    /// `parseSelector` sets it per letter: `Integer.MAX_VALUE` for `@a`
    /// (EntitySelectorParser.java:207) and `@e` (:218), and `1` for `@n` (:224),
    /// `@p` (:230), `@r` (:237) and `@s` (:244). A bare name or UUID goes
    /// through `parseNameOrUUID`, which sets it to `1` (:282). `limit=` moves it
    /// afterwards (EntitySelectorOptions.java:173).
    pub(super) fn max_results(&self) -> i32 {
        if let Some(limit) = self.options.max_results {
            return limit;
        }
        match self.form {
            SelectorForm::Kind(SelectorKind::AllPlayers | SelectorKind::AllEntities) => i32::MAX,
            _ => 1,
        }
    }

    /// Whether `getMaxResults` answers more than one, which is what the
    /// single-entity arguments refuse on.
    pub(super) fn max_results_exceeds_one(&self) -> bool {
        self.max_results() > 1
    }

    /// `EntitySelector.order`, which `parseSelector` sets per letter --
    /// `ORDER_ARBITRARY` for `@a` (EntitySelectorParser.java:209) and `@e`
    /// (:220), `ORDER_NEAREST` for `@n` (:226) and `@p` (:232), `ORDER_RANDOM`
    /// for `@r` (:239) -- and `sort=` replaces
    /// (EntitySelectorOptions.java:181-190). `@s` and a bare name leave the
    /// field default (`:88`).
    pub(super) fn order(&self) -> SelectorOrder {
        if let Some(order) = self.options.order {
            return order;
        }
        match self.form {
            SelectorForm::Kind(SelectorKind::NearestEntity | SelectorKind::NearestPlayer) => {
                SelectorOrder::Nearest
            }
            SelectorForm::Kind(SelectorKind::RandomPlayer) => SelectorOrder::Random,
            _ => SelectorOrder::Arbitrary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectorKind {
    /// `@a`: every player, in the player list's own order
    /// (`EntitySelector.ORDER_ARBITRARY`).
    AllPlayers,
    /// `@p`: the player nearest the source position.
    NearestPlayer,
    /// `@r`: one player drawn at random.
    RandomPlayer,
    /// `@s`: the source itself.
    Source,
    /// `@e`: every entity.
    AllEntities,
    /// `@n`: the nearest entity.
    NearestEntity,
}

impl SelectorKind {
    /// The six selector forms, each with the letter `parseSelector`
    /// (EntitySelectorParser.java:203-248) reads it back from and the tooltip
    /// key `fillSelectorSuggestions` (:475-482) offers it under, in the order
    /// that method suggests them in.
    ///
    /// The parse and the completion answer read the same six rows: a letter
    /// this table does not carry is exactly the `default` arm (:213-215) that
    /// refuses the selector, and a letter it does carry is exactly a form
    /// [`super::suggest`] may offer.
    pub(super) const FORMS: [(char, Self, &'static str); 6] = [
        (
            'p',
            Self::NearestPlayer,
            "argument.entity.selector.nearestPlayer",
        ),
        ('a', Self::AllPlayers, "argument.entity.selector.allPlayers"),
        (
            'r',
            Self::RandomPlayer,
            "argument.entity.selector.randomPlayer",
        ),
        ('s', Self::Source, "argument.entity.selector.self"),
        (
            'e',
            Self::AllEntities,
            "argument.entity.selector.allEntities",
        ),
        (
            'n',
            Self::NearestEntity,
            "argument.entity.selector.nearestEntity",
        ),
    ];

    /// The kind `parseSelector`'s switch (EntitySelectorParser.java:205-248)
    /// reads `letter` as, or `None` when it reaches the `default` arm.
    pub(super) fn from_letter(letter: char) -> Option<Self> {
        Self::FORMS
            .iter()
            .find(|(candidate, ..)| *candidate == letter)
            .map(|(_, kind, _)| *kind)
    }
}

/// The failure `Commands.finishParsing` reports, and where the cursor sits.
/// `None` is an exception created without context, which gets no cursor
/// line (Commands.java:368).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ParseFailure {
    pub message: NbtSummaryValue,
    pub cursor: Option<usize>,
}

/// The outcome of one parse.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ParseResult {
    pub nodes: Vec<ParsedNode>,
    /// Where the walk stopped, which is the cursor a failure quotes.
    pub cursor: usize,
    /// The errors the deepest level collected, as `ParseResults.getExceptions`
    /// holds them: `getParseException` only surfaces one when it is the only
    /// one there.
    pub errors: Vec<ParseFailure>,
}

impl ParseResult {
    /// The node the parse ended on when that node carries a command, which is
    /// the condition `ContextChain.tryFlatten` needs to build anything.
    pub(super) fn executable_node(&self) -> Option<u16> {
        let last = self.nodes.last()?;
        catalog::node(last.index).executable.then_some(last.index)
    }

    /// The value the argument node named `name` read on the inner command.
    ///
    /// Only argument nodes are looked at: a literal may carry the same name as
    /// the argument below it — `/gamemode` names both — and only one of the two
    /// ever holds a value. `/execute … run` and `/return run` redirect at the
    /// dispatcher root, so the executable's `CommandContext` does not see the
    /// modifier arguments that share a name (`pos` on both `positioned` and
    /// `/setblock`).
    pub(super) fn argument(&self, name: &str) -> Option<&ArgumentValue> {
        self.executable_argument_nodes()
            .filter(|node| node.value.is_some())
            .find(|node| catalog::node(node.index).name == name)
            .and_then(|node| node.value.as_ref())
    }

    /// Nodes belonging to the command after the last `/execute run` or
    /// `/return run` redirect.
    fn executable_argument_nodes(&self) -> impl Iterator<Item = &ParsedNode> {
        let start = self
            .nodes
            .iter()
            .rposition(|node| {
                matches!(
                    catalog::canonical_path(node.index).as_slice(),
                    ["execute", "run"] | ["return", "run"]
                )
            })
            .map_or(0, |index| index + 1);
        self.nodes[start..].iter()
    }

    /// The names of the `minecraft:message` arguments the parse read, which is
    /// exactly what `SignableCommand.of` collects.
    pub(super) fn signable_arguments(&self) -> Vec<&'static str> {
        self.nodes
            .iter()
            .filter(|node| matches!(node.value, Some(ArgumentValue::Message(_))))
            .map(|node| catalog::node(node.index).name)
            .collect()
    }

    /// `Commands.getParseException` (Commands.java:469-479).
    pub(super) fn failure(&self, command: &[u16]) -> Option<ParseFailure> {
        if self.cursor >= command.len() {
            return None;
        }
        if self.errors.len() == 1 {
            return self.errors.first().cloned();
        }
        Some(ParseFailure {
            message: if self.nodes.is_empty() {
                translatable("command.unknown.command", Vec::new())
            } else {
                translatable("command.unknown.argument", Vec::new())
            },
            cursor: Some(self.cursor),
        })
    }
}

/// Parses `command` against the tree `permission` may use.
///
/// The world travels with the walk because one argument type resolves against
/// it: `ResourceArgument` (ResourceArgument.java:113-119) refuses an id its
/// registry does not hold, and the registry it was built from
/// (`CommandBuildContext`) is the datapack generation this world loaded.
pub(super) fn parse(
    command: &[u16],
    permission: CommandPermission,
    world: &ServerWorld,
) -> ParseResult {
    let mut pending = vec![(
        Some(VANILLA_COMMAND_TREE_ROOT),
        ParseResult {
            nodes: Vec::new(),
            cursor: 0,
            errors: Vec::new(),
        },
    )];
    let mut best = None;
    let priority = |candidate: &ParseResult| {
        (
            candidate.cursor != command.len(),
            !candidate.errors.is_empty(),
        )
    };

    // Redirects can repeat throughout a function line. Keep pending branches
    // off the call stack and move the prefix along unambiguous paths.
    while let Some((node, mut context)) = pending.pop() {
        if let Some(node) = node {
            let mut errors = Vec::new();
            let mut candidates = Vec::new();
            for child in relevant_children(node, command, context.cursor, permission) {
                let parsed = match parse_child(child, command, context.cursor, permission, world) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        errors.push(error);
                        continue;
                    }
                };
                if command.get(parsed.end).is_some_and(|byte| *byte != 32) {
                    errors.push(ParseFailure {
                        message: translatable("command.expected.separator", Vec::new()),
                        cursor: Some(parsed.end),
                    });
                    continue;
                }
                candidates.push(parsed);
            }
            if !candidates.is_empty() {
                // Reverse insertion keeps depth-first registration order, so
                // equal-priority results still choose the first child.
                for (index, parsed) in candidates.into_iter().enumerate().rev() {
                    let redirect = catalog::redirect_target(parsed.index);
                    let width = if redirect.is_some() { 1 } else { 2 };
                    let descend = command.len() - parsed.end >= width;
                    let next = descend.then_some(redirect.unwrap_or(parsed.index));
                    let cursor = parsed.end + usize::from(descend);
                    let mut nodes = if index == 0 {
                        std::mem::take(&mut context.nodes)
                    } else {
                        context.nodes.clone()
                    };
                    nodes.push(parsed);
                    pending.push((
                        next,
                        ParseResult {
                            nodes,
                            cursor,
                            errors: Vec::new(),
                        },
                    ));
                }
                continue;
            }
            context.errors = errors;
        }

        if priority(&context) == (false, false) {
            return context;
        }
        if best
            .as_ref()
            .is_none_or(|current| priority(&context) < priority(current))
        {
            best = Some(context);
        }
    }
    best.expect("the root produces at least one parse result")
}

/// `CommandNode.getRelevantNodes`: a literal child whose name is exactly the
/// next word is the only candidate, and otherwise only the argument children
/// are tried, so a misspelled literal never falls through to an argument.
///
/// The children are filtered by `canUse` first, which for this dispatcher also
/// drops the commands it does not implement: `fillUsableCommands`
/// (Commands.java:419-437) publishes exactly the tree a source may parse, so
/// parsing and publishing read one set.
fn relevant_children(
    node: u16,
    command: &[u16],
    cursor: usize,
    permission: CommandPermission,
) -> Vec<u16> {
    let cursor = cursor.min(command.len());
    let word_end = bbb_protocol::scan_command_token_utf16(command, cursor);
    let word = &command[cursor..word_end];

    let mut arguments = Vec::new();
    for child in catalog::node(node).children().map(|child| child as u16) {
        if catalog::is_literal(child) {
            let name = catalog::node(child).name;
            if name.len() == word.len()
                && name.bytes().map(u16::from).eq(word.iter().copied())
                && catalog::is_usable(child, permission)
            {
                return vec![child];
            }
        } else if catalog::is_usable(child, permission) {
            arguments.push(child);
        }
    }
    arguments
}

fn parse_child(
    child: u16,
    command: &[u16],
    cursor: usize,
    permission: CommandPermission,
    world: &ServerWorld,
) -> Result<ParsedNode, ParseFailure> {
    let node = catalog::node(child);
    if catalog::is_literal(child) {
        let end = cursor + node.name.len();
        let matches = command.len() >= end
            && node
                .name
                .bytes()
                .map(u16::from)
                .eq(command[cursor..end].iter().copied())
            && command.get(end).is_none_or(|byte| *byte == 32);
        if !matches {
            // `LiteralCommandNode.parse` reports the literal it wanted. The
            // relevant-children rule only hands a literal over on an exact word
            // match, so this is reachable only where a node carries two
            // children of the same name.
            return Err(ParseFailure {
                message: translatable(
                    "argument.literal.incorrect",
                    vec![component_literal_nbt(node.name)],
                ),
                cursor: Some(cursor),
            });
        }
        return Ok(ParsedNode {
            index: child,
            start: cursor,
            end,
            value: None,
        });
    }

    let (end, value) = parse_argument(child, command, cursor, permission, world)?;
    Ok(ParsedNode {
        index: child,
        start: cursor,
        end,
        value: Some(value),
    })
}

fn parse_argument(
    child: u16,
    command: &[u16],
    cursor: usize,
    permission: CommandPermission,
    world: &ServerWorld,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let node = catalog::node(child);
    let type_id = node
        .parser_type_id()
        .expect("an argument node names a registered parser");
    match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID => read_boolean(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID => {
            let CommandArgumentTemplate::Integer { min, max } = argument_template(node) else {
                unreachable!("the integer parser carries an integer template");
            };
            read_integer(command, cursor, min, max)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID => {
            let CommandArgumentTemplate::Float { min, max } = argument_template(node) else {
                unreachable!("the float parser carries a float template");
            };
            read_float(command, cursor, min, max)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID => {
            let CommandArgumentTemplate::Double { min, max } = argument_template(node) else {
                unreachable!("the double parser carries a double template");
            };
            read_brigadier_double(command, cursor, min, max)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID => {
            let CommandArgumentTemplate::String { mode } = argument_template(node) else {
                unreachable!("the string parser carries a string template");
            };
            read_string(command, cursor, mode)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID => message::read_message(
            command,
            cursor,
            permission,
            world.plugins().content_registry(),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID => {
            let CommandArgumentTemplate::Entity {
                single,
                players_only,
            } = argument_template(node)
            else {
                unreachable!("the entity parser carries an entity template");
            };
            let (end, selector) = read_selector_with_registry(
                command,
                cursor,
                permission,
                world.plugins().content_registry(),
            )?;
            // `EntityArgument.parse` (EntityArgument.java:106-123) checks the
            // two template flags in this order, and every refusal first does
            // `reader.setCursor(0)` so `createWithContext` copies the start of
            // the command rather than the position the parse stopped at.
            //
            // The count check comes first (:110-117): a selector whose
            // `getMaxResults` is above one cannot satisfy a `single` argument,
            // and the key it is refused with depends on the other flag --
            // `argument.player.toomany` (:31, thrown at :113) when the argument
            // is players-only, `argument.entity.toomany` (:30, thrown at :116)
            // when it is not.
            if selector.max_results_exceeds_one() && single {
                let key = if players_only {
                    "argument.player.toomany"
                } else {
                    "argument.entity.toomany"
                };
                return Err(ParseFailure {
                    message: translatable(key, Vec::new()),
                    cursor: Some(0),
                });
            }
            // Then the players-only check (:118-120), which refuses anything
            // that reaches entities unless it is the source itself.
            if selector.includes_entities && players_only && !selector.self_selector {
                return Err(ParseFailure {
                    message: translatable("argument.player.entities", Vec::new()),
                    cursor: Some(0),
                });
            }
            Ok((end, ArgumentValue::Players(selector)))
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID => read_game_profile(
            command,
            cursor,
            permission,
            world.plugins().content_registry(),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID => {
            let CommandArgumentTemplate::Time { minimum } = argument_template(node) else {
                unreachable!("the time parser carries a time template");
            };
            read_time(command, cursor, minimum)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID => read_game_mode(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID => {
            let CommandArgumentTemplate::Registry { registry } = argument_template(node) else {
                unreachable!("the resource parser carries a registry template");
            };
            read_resource(command, cursor, &registry, world)
        }
        bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID => {
            read_template_enum(
                command,
                cursor,
                bbb_protocol::packets::VANILLA_TEMPLATE_ROTATIONS,
            )
        }
        bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID => {
            read_template_enum(
                command,
                cursor,
                bbb_protocol::packets::VANILLA_TEMPLATE_MIRRORS,
            )
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID => {
            let CommandArgumentTemplate::Registry { registry } = argument_template(node) else {
                unreachable!("the resource-key parser carries a registry template");
            };
            read_resource_key(command, cursor, &registry)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID => {
            let CommandArgumentTemplate::Registry { registry } = argument_template(node) else {
                unreachable!("the resource-or-tag parser carries a registry template");
            };
            read_resource_or_tag(command, cursor, &registry, world)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID => {
            read_resource_or_tag_key(command, cursor)
        }
        // `/particle`'s `delta` is the only `Vec3Argument.vec3(false)` node
        // (ParticleCommand.java:57); the protocol carries no such flag.
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID => read_vec3(command, cursor, node.name != "delta"),
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID => read_vec2(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID => read_block_pos(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID => read_column_pos(command, cursor),
        // `BlockStateArgument` refuses a tag and `BlockPredicateArgument`
        // accepts one, which is the `forTesting` flag `parseForBlock`
        // (BlockStateParser.java:92-103) and `parseForTesting` (`:111-126`)
        // hand the parser and the whole difference between the two grammars.
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID => {
            read_block_state_with_registry(command, cursor, world.plugins().content_registry())
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID => read_block_predicate_with_registry(
            command,
            cursor,
            world.tags(),
            world.plugins().content_registry(),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_ROTATION_ID => read_rotation(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID => read_entity_anchor(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID => read_swizzle(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID => read_heightmap(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID => read_function(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID => read_compound_tag(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID => read_nbt_tag(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID => read_nbt_path(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID => {
            read_component(command, cursor, world.plugins().content_registry())
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID => read_dimension(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID => read_identifier(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID => read_int_range(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID => read_float_range(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID => read_item_stack(command, cursor, world),
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID => read_item_slot(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID => read_item_slots(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID => {
            super::item_predicate::read(command, cursor, world)
                .map(|(end, predicate)| (end, ArgumentValue::ItemPredicate(predicate)))
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID => {
            read_loot_modifier(command, cursor, world)
                .map(|(end, modifier)| (end, ArgumentValue::LootModifier(modifier)))
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID => {
            super::loot_predicate::read(command, cursor, world)
                .map(|(end, predicate)| (end, ArgumentValue::LootPredicate(predicate)))
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID => read_loot_table(command, cursor, world)
            .map(|(end, table)| (end, ArgumentValue::LootTable(table))),
        VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID => read_dialog(command, cursor, world)
            .map(|(end, dialog)| (end, ArgumentValue::Dialog(dialog))),
        VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID => {
            particle::read_particle(command, cursor, world)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID => read_objective(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID => {
            scoreboard::read_objective_criteria_with_registry(
                command,
                cursor,
                world.plugins().content_registry(),
            )
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID => read_scoreboard_slot(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID => {
            let CommandArgumentTemplate::ScoreHolder { multiple } = argument_template(node) else {
                unreachable!("the score-holder parser carries a score-holder template");
            };
            scoreboard::read_score_holder_with_registry(
                command,
                cursor,
                permission,
                multiple,
                world.plugins().content_registry(),
            )
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID => read_operation(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID => scoreboard::read_style_with_registry(
            command,
            cursor,
            world.plugins().content_registry(),
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID => read_team(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID => read_color(command, cursor),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID => read_hex_color(command, cursor),
        // Every argument node this server publishes names one of the parsers
        // above, which `every_argument_parser_this_server_publishes_has_a_reader`
        // asserts over the whole published tree. A parser that is not read is
        // therefore a command this dispatcher does not offer, and `parseNodes`
        // treats a child that refuses the input as a child that did not match:
        // the walk falls back to the unknown-argument error at the cursor rather
        // than taking the process down.
        _ => Err(ParseFailure {
            message: translatable("command.unknown.argument", Vec::new()),
            cursor: Some(cursor),
        }),
    }
}

/// `GameProfileArgument.parse` (GameProfileArgument.java:46-68).
///
/// Only a leading `@` reaches the selector parser (:47). Everything else is read
/// to the next space verbatim (:56-62) and handed to the name cache, which is a
/// different grammar from the one `EntitySelectorParser.parseNameOrUUID` reads:
/// there is no quoting, no sixteen-character bound and no character set that
/// ends the word early, so `/ban "Bob"` asks the cache for `"Bob"` — quotes
/// included — rather than for Bob.
fn read_game_profile(
    command: &[u16],
    cursor: usize,
    permission: CommandPermission,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    if !command[cursor..].starts_with(&[64]) {
        let end = bbb_protocol::scan_command_token_utf16(command, cursor);
        return Ok((
            end,
            ArgumentValue::Profiles(Selector {
                form: SelectorForm::Named(command[cursor..end].into()),
                // No `EntitySelectorParser` ran, so neither flag it would have
                // set is set: this branch builds its own `Result` (:63-66)
                // instead of a `SelectorResult`.
                includes_entities: false,
                self_selector: false,
                options: SelectorOptions::default(),
            }),
        ));
    }
    let (end, selector) =
        read_selector_with_registry(command, cursor, permission, content_registry)?;
    // `parse` (:50-51) asks only whether the selector reaches entities, so `@s`
    // is refused here where the entity argument accepts it. `parser.parse()`
    // (:49) has already consumed the whole selector and nothing puts the reader
    // back before `createWithContext` (:51) copies its position, so the refusal
    // quotes the end of the selector. That is the one place this differs from
    // `EntityArgument.parse`, which rewinds with an explicit
    // `reader.setCursor(0)` (EntityArgument.java:119) first.
    if selector.includes_entities {
        return Err(ParseFailure {
            message: translatable("argument.player.entities", Vec::new()),
            cursor: Some(end),
        });
    }
    Ok((end, ArgumentValue::Profiles(selector)))
}

/// `EntitySelectorParser.parse` (EntitySelectorParser.java:457-473).
///
/// The grammar itself is the shared prefix parser
/// (`bbb_protocol::parse_command_entity_selector_prefix_utf16`), which is the one
/// implementation of the selector syntax in this workspace; only the selector
/// letter and the refusals are read back here.
pub(in crate::commands) fn read_selector_with_registry(
    command: &[u16],
    cursor: usize,
    permission: CommandPermission,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, Selector), ParseFailure> {
    let rest = &command[cursor..];
    if !rest.starts_with(&[64]) {
        let Some(prefix) =
            bbb_protocol::component::parse_command_entity_selector_prefix_utf16_with_registry(
                rest,
                content_registry,
            )
        else {
            // `ERROR_INVALID_NAME_OR_UUID` (:273-276): an empty name, or one
            // longer than sixteen characters, that is not a UUID either.
            return Err(ParseFailure {
                message: translatable("argument.entity.invalid", Vec::new()),
                cursor: Some(cursor),
            });
        };
        return Ok((
            cursor + prefix.consumed,
            Selector {
                form: SelectorForm::Named(
                    read_command_string_value_utf16(rest, 0)
                        .expect("the selector prefix accepted a name")
                        .1,
                ),
                includes_entities: prefix.includes_entities,
                self_selector: prefix.self_selector,
                // A name carries no option list at all.
                options: SelectorOptions::default(),
            },
        ));
    }

    // `LevelBasedPermissionSet` (:18) grants `COMMANDS_ENTITY_SELECTORS` from
    // `PermissionLevel.GAMEMASTERS` up, and `parse` (:461-463) refuses the `@`
    // outright below it.
    if permission < CommandPermission::GameMaster {
        return Err(ParseFailure {
            message: translatable("argument.entity.selector.not_allowed", Vec::new()),
            cursor: Some(cursor),
        });
    }
    let Some(letter) = rest.get(1).copied() else {
        return Err(ParseFailure {
            message: translatable("argument.entity.selector.missing", Vec::new()),
            cursor: Some(cursor + 1),
        });
    };
    let Some(kind) = char::from_u32(u32::from(letter)).and_then(SelectorKind::from_letter) else {
        // `parseSelector` (:202-215) rewinds to the character it read the type
        // from, which is the one after the `@`, and reports from there.
        return Err(ParseFailure {
            message: translatable(
                "argument.entity.selector.unknown",
                vec![component_literal_nbt(vec![64, letter])],
            ),
            cursor: Some(cursor + 1),
        });
    };
    // `parseOptions` (:285-325) ahead of the shared parser, so a refusal is
    // worded and placed the way vanilla raises it.
    let options = selector_options::read_options_with_registry(rest, kind, content_registry)
        .map_err(|failure| ParseFailure {
            message: failure.message,
            cursor: failure.offset.map(|offset| cursor + offset),
        })?;
    let Some(prefix) =
        bbb_protocol::component::parse_command_entity_selector_prefix_utf16_with_registry(
            rest,
            content_registry,
        )
    else {
        // Only reachable where the two readers of the same grammar disagree;
        // the `[` is the one position both agree the list starts at.
        return Err(ParseFailure {
            message: translatable("argument.entity.options.unterminated", Vec::new()),
            cursor: Some(cursor + 2),
        });
    };
    Ok((
        cursor + prefix.consumed,
        Selector {
            form: SelectorForm::Kind(kind),
            // The shared parser applies every `setIncludesEntities(false)` the
            // options carry -- `type=player` (EntitySelectorOptions.java:296),
            // `gamemode` (`:235`), `level` (`:123`) and a non-empty
            // `advancements` map (`:450`) -- which is what
            // `EntityArgument.parse` (:118-120) checks before it accepts a `@e`
            // for a players-only argument.
            includes_entities: prefix.includes_entities,
            self_selector: prefix.self_selector,
            options,
        },
    ))
}

/// `StringReader.readBoolean` (StringReader.java:247-264): the word comes
/// from `readString` (:249), so a quoted `"true"` is accepted too.
///
/// Both refusals name the start of the word: the empty case never advanced the
/// cursor, and the invalid case is put back explicitly by `this.cursor = start`
/// (:262) before the throw. That rewind is what separates this reader from
/// `read_game_mode`, which has none.
fn read_boolean(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, ArgumentValue::Text(word)) = read_quotable_phrase(command, cursor)? else {
        unreachable!("readString answers text");
    };
    match word {
        word if word == "true" => Ok((end, ArgumentValue::Boolean(true))),
        word if word == "false" => Ok((end, ArgumentValue::Boolean(false))),
        word if word.is_empty() => Err(ParseFailure {
            message: translatable("parsing.bool.expected", Vec::new()),
            cursor: Some(cursor),
        }),
        other => Err(ParseFailure {
            message: translatable("parsing.bool.invalid", vec![component_literal_nbt(other)]),
            cursor: Some(cursor),
        }),
    }
}

/// `GameModeArgument.parse` (GameModeArgument.java:27-35): an unquoted word
/// read through `GameType.byName(name, null)` (GameType.java:103-106), which is
/// `CODEC.byName` — the enum's own serialized names and nothing else. A word
/// that names no game type is `argument.gamemode.invalid` carrying the word
/// (:23-25), reported at the *end* of the word: `readUnquotedString` (:28)
/// advanced the cursor past it and `parse` never puts the cursor back before
/// throwing (:31), so `createWithContext`
/// (DynamicCommandExceptionType.java:18-20) snapshots the end position onto the
/// exception. That snapshot is a plain `int` field
/// (CommandSyntaxException.java:22-28); the `reader.setCursor(cursor)` a caller
/// runs afterwards cannot reach it. `read_boolean` hands back the start of the
/// word instead, and the difference is not a modelling choice: `readBoolean`
/// has an explicit `this.cursor = start` before its throw
/// (StringReader.java:262) and this parser has no such line.
fn read_game_mode(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    let word = &command[cursor..end];
    match JavaString::with_utf8_units(word, vanilla_game_type_for_serialized_name).flatten() {
        Some(entry) => Ok((end, ArgumentValue::GameMode(entry.game_type))),
        None => Err(ParseFailure {
            message: translatable(
                "argument.gamemode.invalid",
                vec![component_literal_nbt(word)],
            ),
            cursor: Some(end),
        }),
    }
}

/// `FloatArgumentType.parse`
/// (com/mojang/brigadier/arguments/FloatArgumentType.java:43-55) over
/// `StringReader.readFloat` (com/mojang/brigadier/StringReader.java:157-175).
///
/// The two bound messages take the bound before the value that broke it, which
/// is the argument order `BrigadierExceptions` reverses them into
/// (BrigadierExceptions.java:16-20) rather than the order Brigadier's own
/// literal message uses.
fn read_float(
    command: &[u16],
    cursor: usize,
    min: Option<f32>,
    max: Option<f32>,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = number_end(command, cursor);
    let number = &command[cursor..end];
    if number.is_empty() {
        return Err(ParseFailure {
            message: translatable("parsing.float.expected", Vec::new()),
            cursor: Some(cursor),
        });
    }
    let Some(value) = JavaString::with_utf8_units(number, |number| number.parse::<f32>().ok())
        .flatten()
        .filter(|value| value.is_finite())
    else {
        return Err(ParseFailure {
            message: translatable("parsing.float.invalid", vec![component_literal_nbt(number)]),
            cursor: Some(cursor),
        });
    };
    let min = min.unwrap_or(-f32::MAX);
    let max = max.unwrap_or(f32::MAX);
    if value < min {
        return Err(ParseFailure {
            message: translatable(
                "argument.float.low",
                vec![
                    NbtSummaryValue::Float(min.to_bits()),
                    NbtSummaryValue::Float(value.to_bits()),
                ],
            ),
            cursor: Some(cursor),
        });
    }
    if value > max {
        return Err(ParseFailure {
            message: translatable(
                "argument.float.big",
                vec![
                    NbtSummaryValue::Float(max.to_bits()),
                    NbtSummaryValue::Float(value.to_bits()),
                ],
            ),
            cursor: Some(cursor),
        });
    }
    Ok((end, ArgumentValue::Float(value)))
}

/// `DoubleArgumentType.parse` over `StringReader.readDouble`, with the same
/// bound-then-value argument order `BrigadierExceptions` uses
/// (BrigadierExceptions.java:10-14).
fn read_brigadier_double(
    command: &[u16],
    cursor: usize,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, value) = read_double(command, cursor)?;
    let min = min.unwrap_or(-f64::MAX);
    let max = max.unwrap_or(f64::MAX);
    if value < min {
        return Err(ParseFailure {
            message: translatable(
                "argument.double.low",
                vec![
                    NbtSummaryValue::Double(min.to_bits()),
                    NbtSummaryValue::Double(value.to_bits()),
                ],
            ),
            cursor: Some(cursor),
        });
    }
    if value > max {
        return Err(ParseFailure {
            message: translatable(
                "argument.double.big",
                vec![
                    NbtSummaryValue::Double(max.to_bits()),
                    NbtSummaryValue::Double(value.to_bits()),
                ],
            ),
            cursor: Some(cursor),
        });
    }
    Ok((end, ArgumentValue::Double(value)))
}

/// `StringReader.readInt` followed by the bounds `IntegerArgumentType` holds,
/// which the node's own properties carry.
fn read_integer(
    command: &[u16],
    cursor: usize,
    min: Option<i32>,
    max: Option<i32>,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = number_end(command, cursor);
    let number = &command[cursor..end];
    if number.is_empty() {
        return Err(ParseFailure {
            message: translatable("parsing.int.expected", Vec::new()),
            cursor: Some(cursor),
        });
    }
    let Some(value) =
        JavaString::with_utf8_units(number, |number| number.parse::<i32>().ok()).flatten()
    else {
        return Err(ParseFailure {
            message: translatable("parsing.int.invalid", vec![component_literal_nbt(number)]),
            cursor: Some(cursor),
        });
    };
    if min.is_some_and(|min| value < min) {
        return Err(ParseFailure {
            message: translatable(
                "argument.integer.low",
                vec![
                    NbtSummaryValue::Int(min.expect("the bound was just read")),
                    NbtSummaryValue::Int(value),
                ],
            ),
            cursor: Some(cursor),
        });
    }
    if max.is_some_and(|max| value > max) {
        return Err(ParseFailure {
            message: translatable(
                "argument.integer.big",
                vec![
                    NbtSummaryValue::Int(max.expect("the bound was just read")),
                    NbtSummaryValue::Int(value),
                ],
            ),
            cursor: Some(cursor),
        });
    }
    Ok((end, ArgumentValue::Integer(value)))
}

/// `MinMaxBounds.Ints.fromReader` (MinMaxBounds.java:306-316) over
/// `MinMaxBounds.Bounds.fromReader` (`:139-168`): a number, an optional `..`
/// followed by another, either of which may be absent. A single number is both
/// bounds at once. Every failure reports the argument's start, because the
/// outer catch there rebuilds the exception at `start` (`:164-167`).
pub(super) fn read_int_range(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let range_empty = || ParseFailure {
        message: translatable("argument.range.empty", Vec::new()),
        cursor: Some(cursor),
    };
    if cursor >= command.len() {
        return Err(range_empty());
    }
    let (after_min, min) = read_range_number(command, cursor, cursor, "parsing.int.invalid")?;
    let mut end = after_min;
    let max = if command[end..].starts_with(&[46, 46]) {
        end += 2;
        let (after_max, max) = read_range_number(command, end, cursor, "parsing.int.invalid")?;
        end = after_max;
        max
    } else {
        min
    };
    if min.is_none() && max.is_none() {
        return Err(range_empty());
    }
    // `MinMaxBounds.Ints.fromReader`'s own swapped check (`:310-313`).
    if let (Some(min), Some(max)) = (min, max) {
        if min > max {
            return Err(ParseFailure {
                message: translatable("argument.range.swapped", Vec::new()),
                cursor: Some(cursor),
            });
        }
    }
    Ok((end, ArgumentValue::IntRange { min, max }))
}

/// `MinMaxBounds.Doubles.fromReader` (MinMaxBounds.java:239-250) over the same
/// `Bounds.fromReader` grammar (`:139-168`), with `Double::parseDouble` and
/// `parsing.double.invalid`.
pub(super) fn read_float_range(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let range_empty = || ParseFailure {
        message: translatable("argument.range.empty", Vec::new()),
        cursor: Some(cursor),
    };
    if cursor >= command.len() {
        return Err(range_empty());
    }
    let (after_min, min) = read_range_number(command, cursor, cursor, "parsing.double.invalid")?;
    let mut end = after_min;
    let max = if command[end..].starts_with(&[46, 46]) {
        end += 2;
        let (after_max, max) = read_range_number(command, end, cursor, "parsing.double.invalid")?;
        end = after_max;
        max
    } else {
        min
    };
    if min.is_none() && max.is_none() {
        return Err(range_empty());
    }
    if let (Some(min), Some(max)) = (min, max) {
        if min > max {
            return Err(ParseFailure {
                message: translatable("argument.range.swapped", Vec::new()),
                cursor: Some(cursor),
            });
        }
    }
    Ok((end, ArgumentValue::FloatRange { min, max }))
}

/// `MinMaxBounds.Bounds.readNumber` (MinMaxBounds.java:170-199).
pub(in crate::commands) fn read_range_number<T: std::str::FromStr>(
    command: &[u16],
    cursor: usize,
    start: usize,
    invalid_key: &'static str,
) -> Result<(usize, Option<T>), ParseFailure> {
    let mut end = cursor;
    while let Some(&unit) = command.get(end) {
        if !matches!(unit, 48..=57 | 45) && (unit != 46 || command.get(end + 1) == Some(&46)) {
            break;
        }
        end += 1;
    }
    let number = &command[cursor..end];
    if number.is_empty() {
        return Ok((cursor, None));
    }
    JavaString::with_utf8_units(number, |number| number.parse::<T>().ok())
        .flatten()
        .map(|value| (end, Some(value)))
        .ok_or_else(|| ParseFailure {
            message: translatable(invalid_key, vec![component_literal_nbt(number)]),
            cursor: Some(start),
        })
}

/// `StringArgumentType.parse` (StringArgumentType.java:36-44): `word` is
/// `readUnquotedString` (:42), `greedy` takes the rest of the input (:38-40)
/// and `phrase` is `readString`.
///
/// Only the `phrase` mode can fail, and only inside a quoted run:
/// `StringReader.readString` (StringReader.java:233-245) falls through to
/// `readUnquotedString` for anything that does not open with a quote, so the
/// two refusals `readStringUntil` (`:205-231`) raises -- an escape of a
/// character that is neither the terminator nor a backslash, and an input that
/// ends before the closing quote -- are reachable only after one was typed.
pub(super) fn read_string(
    command: &[u16],
    cursor: usize,
    mode: CommandStringMode,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = match mode {
        CommandStringMode::SingleWord => word_end(command, cursor),
        CommandStringMode::GreedyPhrase => command.len(),
        CommandStringMode::QuotablePhrase => return read_quotable_phrase(command, cursor),
    };
    Ok((end, ArgumentValue::Text(command[cursor..end].into())))
}

/// `StringReader.readString` (StringReader.java:233-245): a quote opens a
/// quoted run, anything else is an unquoted word, and the end of the input is
/// the empty string.
///
/// `isQuotedStringStart` accepts both the double and the single quote
/// (StringReader.java:6-8, `:87-89`), and the terminator is whichever one
/// opened the run (`:238-240`).
fn read_quotable_phrase(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    read_command_string_value_utf16(command, cursor)
        .map(|(end, text)| (end, ArgumentValue::Text(text)))
        .map_err(string_failure)
}

pub(super) fn string_failure(error: CommandSyntaxErrorUtf16) -> ParseFailure {
    let message = if error.kind == CommandSyntaxErrorKind::InvalidEscape {
        translatable(
            "parsing.quote.escape",
            vec![component_literal_nbt(
                &error.message.units()[error.message.units().len() - 1..],
            )],
        )
    } else {
        translatable("parsing.quote.expected.end", Vec::new())
    };
    ParseFailure {
        message,
        cursor: Some(error.cursor),
    }
}

/// `TimeArgument.parse` (TimeArgument.java:44-58): a float, then one of the
/// four units (:78-83), then the minimum the node's properties carry.
pub(super) fn read_time(
    command: &[u16],
    cursor: usize,
    minimum: i32,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let number_end = number_end(command, cursor);
    let number = &command[cursor..number_end];
    if number.is_empty() {
        return Err(ParseFailure {
            message: translatable("parsing.float.expected", Vec::new()),
            cursor: Some(cursor),
        });
    }
    let Some(value) =
        JavaString::with_utf8_units(number, |number| number.parse::<f32>().ok()).flatten()
    else {
        return Err(ParseFailure {
            message: translatable("parsing.float.invalid", vec![component_literal_nbt(number)]),
            cursor: Some(cursor),
        });
    };
    let end = word_end(command, number_end);
    let factor = match &command[number_end..end] {
        [100] => 24_000,
        [115] => 20,
        [116] | [] => 1,
        _ => {
            // `parse` (TimeArgument.java:48-49) throws where
            // `readUnquotedString` (:46) left the reader, which is the end of
            // the unit word.
            return Err(ParseFailure {
                message: translatable("argument.time.invalid_unit", Vec::new()),
                cursor: Some(end),
            });
        }
    };
    let ticks = java_round(value * factor as f32);
    if ticks < minimum {
        // Nothing between the unit and this refusal moves the reader
        // (TimeArgument.java:51-53), so it quotes the same position the
        // invalid-unit refusal above does.
        return Err(ParseFailure {
            message: translatable(
                "argument.time.tick_count_too_low",
                vec![NbtSummaryValue::Int(minimum), NbtSummaryValue::Int(ticks)],
            ),
            cursor: Some(end),
        });
    }
    Ok((end, ArgumentValue::Time(ticks)))
}

/// `ResourceArgument.parse` (ResourceArgument.java:113-119): a resource
/// location read greedily, then looked up in the registry the argument was
/// built for.
///
/// The registries this dispatcher publishes a `minecraft:resource` argument
/// against are `minecraft:damage_type` (`DamageCommand.java:38`),
/// `minecraft:attribute` (`AttributeCommand.java:49`),
/// `minecraft:enchantment` (`EnchantCommand.java:46`),
/// `minecraft:mob_effect` (`EffectCommands.java:44`, `:54`),
/// `minecraft:worldgen/biome` (`FillBiomeCommand.java:50`),
/// `minecraft:entity_type` (`SummonCommand.java:40`),
/// `minecraft:world_clock` (`TimeCommand.java:56`) and
/// `minecraft:timeline` (`TimeCommand.java:103`). A node naming any
/// other registry falls through to the unknown-argument refusal.
fn read_resource(
    command: &[u16],
    cursor: usize,
    registry: &str,
    world: &ServerWorld,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let known = registry == DAMAGE_TYPE_REGISTRY
        || registry == catalog::ATTRIBUTE_REGISTRY
        || registry == catalog::ENCHANTMENT_REGISTRY
        || registry == catalog::MOB_EFFECT_REGISTRY
        || registry == catalog::ENTITY_TYPE_REGISTRY
        || registry == catalog::WORLD_CLOCK_REGISTRY
        || registry == catalog::TIMELINE_REGISTRY
        || registry == BIOME_REGISTRY;
    if !known {
        return Err(ParseFailure {
            message: translatable("command.unknown.argument", Vec::new()),
            cursor: Some(cursor),
        });
    }
    // `Identifier.read` (Identifier.java:189-199) reads the greedy run and then
    // parses it; a run whose namespace or path holds a character the greedy set
    // admits but the identifier grammar does not -- `:` and `/` in a namespace,
    // a second `:` anywhere -- rewinds to the start and reports
    // `argument.id.invalid` (`:197`).
    let (end, resource_id) = read_identifier_value(command, cursor)?;
    // `orElseThrow` (`:116-118`) reports where the read stopped, because
    // `createWithContext` snapshots the reader as `Identifier.read` left it.
    let found = if registry == DAMAGE_TYPE_REGISTRY {
        world
            .damage_types()
            .iter()
            .any(|entry| entry.resource_id == resource_id)
    } else if registry == catalog::ENCHANTMENT_REGISTRY {
        world
            .enchantments()
            .iter()
            .any(|entry| entry.resource_id == resource_id)
    } else if registry == catalog::MOB_EFFECT_REGISTRY {
        bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(&resource_id)
            .is_some()
    } else if registry == BIOME_REGISTRY {
        world.registry_contains(BIOME_REGISTRY, &resource_id)
    } else if registry == catalog::ENTITY_TYPE_REGISTRY {
        bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(&resource_id).is_some()
            || world
                .plugins()
                .content_registry()
                .entity_by_resource_id(&resource_id)
                .is_some()
    } else if registry == catalog::WORLD_CLOCK_REGISTRY {
        world.clock_catalog().clock(&resource_id).is_some()
    } else if registry == catalog::TIMELINE_REGISTRY {
        world.clock_catalog().timeline(&resource_id).is_some()
    } else {
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES
            .iter()
            .any(|entry| entry.resource_id == resource_id)
    };
    if !found {
        return Err(ParseFailure {
            message: translatable(
                "argument.resource.not_found",
                vec![
                    component_literal_nbt(&resource_id),
                    component_literal_nbt(registry),
                ],
            ),
            cursor: Some(end),
        });
    }
    Ok((end, ArgumentValue::Resource(resource_id)))
}

/// `ResourceKeyArgument.parse` (`ResourceKeyArgument.java:115-118`): an
/// identifier only. `getAdvancement` (`:105-113`), `getConfiguredFeature`
/// (`:87-89`) and `getRecipe` (`:99-103`) refuse a name the registry does not
/// hold when the command runs.
fn read_resource_key(
    command: &[u16],
    cursor: usize,
    registry: &str,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    if registry != catalog::ADVANCEMENT_REGISTRY
        && registry != catalog::CONFIGURED_FEATURE_REGISTRY
        && registry != catalog::RECIPE_REGISTRY
        && registry != "minecraft:worldgen/template_pool"
        && registry != "minecraft:worldgen/structure"
    {
        return Err(ParseFailure {
            message: translatable("command.unknown.argument", Vec::new()),
            cursor: Some(cursor),
        });
    }
    let (end, resource_id) = read_identifier_value(command, cursor)?;
    Ok((end, ArgumentValue::Resource(resource_id)))
}

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

/// `FunctionArgument.parse` (FunctionArgument.java:31-74).
///
/// Both branches read one `Identifier` and stop; neither consults the library,
/// because `FunctionArgument.Result` defers that to the `create` and `unwrap`
/// bodies the command calls (`:37-51`, `:56-71`). So an unknown function is a
/// command that failed rather than a parse that failed, which is what puts
/// `arguments.function.unknown` on the executor's side of this module.
///
/// The rewind on a malformed id is `Identifier.read`'s own
/// (Identifier.java:189-199): `setCursor(start)` puts the cursor back where the
/// greedy read began, and for the `#` branch that is the character after the
/// `#`, because `reader.skip()` (`:33`) already consumed it.
fn read_function(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let tag = command[cursor..].starts_with(&[35]);
    let start = cursor + usize::from(tag);
    let (end, id) = read_identifier_value(command, start)?;
    Ok((end, ArgumentValue::Function { tag, id }))
}

/// `CompoundTagArgument.parse` (CompoundTagArgument.java:26-28):
/// `TagParser.parseCompoundAsArgument` (TagParser.java:83-86).
///
/// `parseAsArgument` (`:79-81`) consumes one SNBT value and leaves the
/// dispatcher-owned remainder unread. A well-formed non-compound then fails
/// `castToCompoundOrThrow` (`:52-57`) at the cursor the value left behind.
pub(super) fn read_compound_tag(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (consumed, value) = match parse_command_snbt_value_prefix_utf16(&command[cursor..]) {
        Ok(parsed) => parsed,
        Err(failure) => {
            return Err(
                snbt_failure(cursor, &failure).unwrap_or_else(|| ParseFailure {
                    message: translatable("argument.nbt.expected.compound", Vec::new()),
                    cursor: Some(cursor),
                }),
            )
        }
    };
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(ParseFailure {
            message: translatable("argument.nbt.expected.compound", Vec::new()),
            cursor: Some(cursor + consumed),
        });
    }
    Ok((cursor + consumed, ArgumentValue::CompoundTag(value)))
}

fn read_nbt_tag(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (consumed, value) = match parse_command_snbt_value_prefix_utf16(&command[cursor..]) {
        Ok(parsed) => parsed,
        Err(failure) => {
            return Err(
                snbt_failure(cursor, &failure).unwrap_or_else(|| ParseFailure {
                    message: translatable("argument.nbt.invalid", Vec::new()),
                    cursor: Some(cursor),
                }),
            )
        }
    };
    Ok((cursor + consumed, ArgumentValue::Tag(value)))
}

/// `Grammar.parseForCommands` (Grammar.java:29-56) throws the farthest failure
/// itself and `withCodec` (CommandArgumentParser.java:38-45) only wraps what the
/// codec rejects, so every SNBT argument reports that key at that cursor.
/// `None` is a bound this repository adds and Vanilla does not have: no Vanilla
/// key describes it, so the argument keeps its own refusal.
pub(in crate::commands) fn snbt_failure(
    start: usize,
    failure: &SnbtParseFailure,
) -> Option<ParseFailure> {
    Some(ParseFailure {
        message: translatable(
            failure.key?,
            failure
                .arguments
                .iter()
                .map(component_literal_nbt)
                .collect(),
        ),
        cursor: Some(start + failure.cursor),
    })
}

fn read_nbt_path(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    match parse_command_nbt_path_value_prefix_utf16(&command[cursor..]) {
        Ok((consumed, path)) => Ok((cursor + consumed, ArgumentValue::NbtPath(path))),
        // The compound filter's own refusal is the grammar's, not the path's
        // (`NbtPathArgument.java:93`, `:110`, `:121`).
        Err(error) => Err(error
            .snbt
            .as_ref()
            .and_then(|failure| snbt_failure(cursor, failure))
            .unwrap_or_else(|| ParseFailure {
                message: translatable(
                    match error.kind {
                        CommandNbtPathErrorKind::InvalidNode => "arguments.nbtpath.node.invalid",
                        CommandNbtPathErrorKind::TooDeep => "arguments.nbtpath.too_deep",
                        CommandNbtPathErrorKind::NothingFound => "arguments.nbtpath.nothing_found",
                        CommandNbtPathErrorKind::ExpectedList => {
                            "commands.data.modify.expected_list"
                        }
                        CommandNbtPathErrorKind::InvalidIndex => {
                            "commands.data.modify.invalid_index"
                        }
                    },
                    Vec::new(),
                ),
                cursor: Some(cursor + error.offset),
            })),
    }
}

/// `ComponentArgument` (ComponentArgument.java:23-32): one SNBT value, then
/// `ComponentSerialization.CODEC`. `withCodec` (CommandArgumentParser.java:38-45)
/// rewinds to the argument start and throws `argument.component.invalid`
/// wrapping the codec message.
fn read_component(
    command: &[u16],
    cursor: usize,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (consumed, value) = match parse_command_snbt_value_prefix_utf16(&command[cursor..]) {
        Ok(parsed) => parsed,
        Err(failure) => {
            return Err(snbt_failure(cursor, &failure)
                .unwrap_or_else(|| invalid_component(cursor, failure.to_string())))
        }
    };
    if let Err(error) =
        bbb_protocol::validate_component_nbt_with_registry(&value, "component", content_registry)
    {
        return Err(invalid_component(cursor, error));
    }
    // The codec collapses an unstyled literal on the way out
    // (ComponentSerialization.java:107-113, Component.java:56-58), so the
    // argument every consumer reads is already the shape a save writes.
    Ok((
        cursor + consumed,
        ArgumentValue::Component(collapsed_component_nbt(&value)),
    ))
}

fn invalid_component(cursor: usize, message: impl Into<String>) -> ParseFailure {
    ParseFailure {
        // `Component.translatableEscape` (Component.java:147-155) keeps a
        // String argument as a primitive `with` entry.
        message: translatable(
            "argument.component.invalid",
            vec![NbtSummaryValue::String(message.into().into())],
        ),
        cursor: Some(cursor),
    }
}

/// `DimensionArgument.parse` (DimensionArgument.java:31-33): `Identifier.read`
/// and nothing else. The level lookup is `getDimension` (`:51-59`), which the
/// `/execute in` modifier calls after the parse, so an unknown id is a command
/// that failed rather than a parse that failed.
fn read_dimension(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, id) = read_identifier_value(command, cursor)?;
    Ok((end, ArgumentValue::Dimension(id)))
}

/// `IdentifierArgument.parse` (IdentifierArgument.java:23-25), which is
/// `Identifier.read` (Identifier.java:189-199): the greedy identifier run,
/// refused as `argument.id.invalid` from where it started when `parse` rejects
/// it (`:196-197`).
fn read_identifier(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, id) = read_identifier_value(command, cursor)?;
    Ok((end, ArgumentValue::Identifier(id)))
}

/// One stage of the `ContextChain` a parse flattened to that carries a
/// `RedirectModifier` (`BuildContexts.java:58-87`), in chain order.
///
/// A redirect without a modifier is not a stage: `/execute run` and the four
/// command aliases are `redirect`/`forward` with none, so they cost nothing and
/// change nothing about the source.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ChainStage {
    /// `ExecuteCommand.java:255-260`, as `CommandSourceStack.withLevel`.
    ExecuteIn(String),
    /// `ExecuteCommand.java:178-181`, as `withPosition` then `withAnchor(FEET)`.
    ExecutePositioned(Coordinates),
    /// `ExecuteCommand.java:206-209`, as `withRotation`.
    ExecuteRotated(Coordinates),
    /// `ExecuteCommand.java:239`, as `CommandSourceStack.facing(Vec3)`.
    ExecuteFacing(Coordinates),
    /// `ExecuteCommand.java:154-162`, as `fork(execute, withEntity)`.
    ExecuteAs(Selector),
    /// Entity relation forks (`ExecuteCommand.java:994-1049`).
    ExecuteOn(super::types::ExecuteRelation),
    /// Spawn then bind an entity (`ExecuteCommand.java:1052-1055`).
    ExecuteSummon(String),
    /// `ExecuteCommand.java:163-171`, as `fork(execute, withLevel+pos+rot)`.
    ExecuteAt(Selector),
    /// `ExecuteCommand.java:183-191`, as `fork(execute, withPosition)`.
    ExecutePositionedAs(Selector),
    /// `ExecuteCommand.java:192-203`, as `hasChunk` then `withPosition` at
    /// `Level.getHeight`. The heightmap kind is the catalog id.
    ExecutePositionedOver(i32),
    /// `ExecuteCommand.java:211-219`, as `fork(execute, withRotation)`.
    ExecuteRotatedAs(Selector),
    /// `ExecuteCommand.java:224-236`, as `fork(execute, facing(entity, anchor))`.
    ExecuteFacingEntity {
        targets: Selector,
        anchor: EntityAnchor,
    },
    /// `ExecuteCommand.java:244-245`, as `withPosition(position.align(axes))`.
    ExecuteAlign { x: bool, y: bool, z: bool },
    /// `ExecuteCommand.java:251-252`, as `withAnchor(anchor)`.
    ExecuteAnchored(EntityAnchor),
    /// One `if`/`unless` arm used as a fork (`addConditional`'s
    /// `.fork(root, c -> expect(c, expected, predicate.test(c)))`,
    /// `ExecuteCommand.java:883`): the source survives when the condition
    /// answers what the literal asked for.
    ExecuteConditional {
        expected: bool,
        condition: Box<ExecuteCondition>,
    },
    /// `ReturnCommand.ReturnFromCommandCustomModifier` (ReturnCommand.java:28),
    /// which is a `CustomModifierExecutor` and so ends the prepare loop
    /// (`BuildContexts.java:59-62`) rather than transforming the source.
    ReturnRun,
    /// `ExecuteIfFunctionCustomModifier` (`ExecuteCommand.java:1126-1151`):
    /// another `CustomModifierExecutor`, so it also ends the prepare loop.
    ExecuteIfFunction {
        expected: bool,
        tag: bool,
        id: String,
    },
    /// One `/execute store` arm as `wrapStores` (`ExecuteCommand.java:274-302`)
    /// registers it: a plain `redirect` whose modifier hangs a callback off the
    /// source rather than changing where the source stands.
    ExecuteStore {
        /// `storeResult` (`:276`): the `result` literal rather than `success`.
        store_result: bool,
        target: Box<ExecuteStoreArm>,
    },
}

/// `BlockStateArgument.parse` (BlockStateArgument.java:30-33).
#[cfg(test)]
pub(super) fn read_block_state(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    read_block_state_with_registry(
        command,
        cursor,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(super) fn read_block_state_with_registry(
    command: &[u16],
    cursor: usize,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, block) =
        block_state_parse::read_block_state_utf16_with_registry(command, cursor, content_registry)?;
    Ok((end, ArgumentValue::BlockState(block)))
}

/// `BlockPredicateArgument.parse` (BlockPredicateArgument.java:46-52).
#[cfg(test)]
pub(super) fn read_block_predicate(
    command: &[u16],
    cursor: usize,
    tags: &crate::registry_tags::RegistryTagSnapshot,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    read_block_predicate_with_registry(command, cursor, tags, tags.content_registry())
}

pub(super) fn read_block_predicate_with_registry(
    command: &[u16],
    cursor: usize,
    tags: &crate::registry_tags::RegistryTagSnapshot,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, block) = block_state_parse::read_block_predicate_utf16_with_registry(
        command,
        cursor,
        tags,
        content_registry,
    )?;
    Ok((end, ArgumentValue::BlockPredicate(block)))
}

impl From<BlockStateParseError> for ParseFailure {
    fn from(error: BlockStateParseError) -> Self {
        ParseFailure {
            message: translatable(
                error.key,
                error
                    .arguments
                    .into_iter()
                    .map(component_literal_nbt)
                    .collect(),
            ),
            cursor: Some(error.cursor),
        }
    }
}

/// `SlotArgument.parse` (`SlotArgument.java:35-45`): `ParserUtils.readWhile`
/// (`:36`) takes every character that is not a space, then `nameToIds`.
fn read_item_slot(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let rest = &command[cursor..];
    let len = scan_command_token_utf16(rest, 0);
    let name = &rest[..len];
    let end = cursor + len;
    match JavaString::with_utf8_units(name, bbb_server_types::slot_ranges::parse_single_slot)
        .unwrap_or(Err(bbb_server_types::slot_ranges::SlotNameError::Unknown))
    {
        Ok(slot) => Ok((end, ArgumentValue::ItemSlot(slot))),
        Err(bbb_server_types::slot_ranges::SlotNameError::Unknown) => Err(ParseFailure {
            message: translatable("slot.unknown", vec![component_literal_nbt(name)]),
            cursor: Some(cursor),
        }),
        Err(bbb_server_types::slot_ranges::SlotNameError::OnlySingleAllowed) => Err(ParseFailure {
            message: translatable(
                "slot.only_single_allowed",
                vec![component_literal_nbt(name)],
            ),
            cursor: Some(cursor),
        }),
    }
}

fn read_item_slots(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let rest = &command[cursor..];
    let len = scan_command_token_utf16(rest, 0);
    let name = &rest[..len];
    JavaString::with_utf8_units(name, bbb_server_types::slot_ranges::slot_range)
        .flatten()
        .map(|slots| (cursor + len, ArgumentValue::ItemSlots(slots)))
        .ok_or_else(|| ParseFailure {
            message: translatable("slot.unknown", vec![component_literal_nbt(name)]),
            cursor: Some(cursor),
        })
}

/// `ItemArgument.parse` (`ItemArgument.java:26-28`) / `ItemParser.readItem`
/// (`ItemParser.java:134-141`) plus the optional `[...]` component patch
/// (`:128-131`, `:143-191`).
fn read_item_stack(
    command: &[u16],
    cursor: usize,
    world: &ServerWorld,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, resource_id) = read_identifier_value(command, cursor)?;
    let Some(item_id) = bbb_protocol::item_types::vanilla_item_type_for_resource_id(&resource_id)
        .map(|item| item.id)
        .or_else(|| {
            world
                .plugins()
                .content_registry()
                .item_by_resource_id(&resource_id)
                .map(|item| item.id)
        })
    else {
        return Err(ParseFailure {
            message: translatable(
                "argument.item.id.invalid",
                vec![component_literal_nbt(&resource_id)],
            ),
            cursor: Some(cursor),
        });
    };
    // `ItemParser.State.parse` reads the patch only when the next character is
    // `[` (`ItemParser.java:128-131`); an identifier alone is unchanged.
    let mut patch = Vec::new();
    let mut end = end;
    if command.get(end) == Some(&91) {
        let (patch_end, entries) = read_item_component_patch(command, end, world)?;
        end = patch_end;
        patch = entries;
    }
    Ok((
        end,
        ArgumentValue::ItemStack {
            item_id,
            resource_id,
            patch,
        },
    ))
}

/// `Identifier.isAllowedInIdentifier` (Identifier.java:216-218), the character
/// set `readGreedy` (`:179-187`) consumes.
pub(super) fn read_identifier_value(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, String), ParseFailure> {
    scan_command_resource_location_utf16(command, cursor).map_err(|_| ParseFailure {
        message: translatable("argument.id.invalid", Vec::new()),
        cursor: Some(cursor),
    })
}

/// `EntityAnchorArgument.parse` (EntityAnchorArgument.java:38-48): an unquoted
/// word looked up in `Anchor.BY_NAME` (`:64-68`), with the reader put back at
/// the start of the word before the refusal quotes it (`:43-44`).
fn read_entity_anchor(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    let name = &command[cursor..end];
    match VANILLA_ENTITY_ANCHORS.iter().find(|entry| {
        entry
            .serialized_name
            .encode_utf16()
            .eq(name.iter().copied())
    }) {
        Some(entry) => Ok((end, ArgumentValue::EntityAnchor(entry.anchor))),
        // `ERROR_INVALID` (`:26-28`), a `translatableEscape` whose one argument
        // is the word that named no anchor.
        None => Err(ParseFailure {
            message: translatable("argument.anchor.invalid", vec![component_literal_nbt(name)]),
            cursor: Some(cursor),
        }),
    }
}

/// `SwizzleArgument.parse` (SwizzleArgument.java:27-47): consume `x`/`y`/`z`
/// until a space, refuse a duplicate or any other character with
/// `arguments.swizzle.invalid`, and keep an empty set when the token is empty.
fn read_swizzle(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let bytes = command;
    let mut i = cursor;
    let mut x = false;
    let mut y = false;
    let mut z = false;
    while i < bytes.len() && bytes[i] != 32 {
        let axis = bytes[i];
        i += 1;
        let slot = match axis {
            120 => &mut x,
            121 => &mut y,
            122 => &mut z,
            _ => {
                return Err(ParseFailure {
                    message: translatable("arguments.swizzle.invalid", Vec::new()),
                    cursor: Some(i),
                });
            }
        };
        if *slot {
            return Err(ParseFailure {
                message: translatable("arguments.swizzle.invalid", Vec::new()),
                cursor: Some(i),
            });
        }
        *slot = true;
    }
    Ok((i, ArgumentValue::Swizzle { x, y, z }))
}

/// `HeightmapTypeArgument` (`HeightmapTypeArgument.java:11-18`) via
/// `StringRepresentableArgument.parse` (`StringRepresentableArgument.java:34-37`):
/// an unquoted word that names a `keepAfterWorldgen` type, else
/// `argument.enum.invalid`.
fn read_heightmap(command: &[u16], cursor: usize) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    let raw = &command[cursor..end];
    let kind = VANILLA_HEIGHTMAP_TYPES.iter().find(|entry| {
        entry.usage != HeightmapUsage::Worldgen
            && JavaString::with_utf8_units(raw, |raw| {
                entry.serialization_key.eq_ignore_ascii_case(raw)
            })
            .unwrap_or(false)
    });
    match kind {
        Some(entry) => Ok((end, ArgumentValue::Heightmap(entry.id))),
        None => Err(ParseFailure {
            message: translatable("argument.enum.invalid", vec![component_literal_nbt(raw)]),
            cursor: Some(end),
        }),
    }
}

/// `LocalCoordinates.readDouble` (LocalCoordinates.java:58-68): every axis must
/// carry its own `^`, and one that does not rewinds to the argument's start
/// before reporting the mixed-form refusal.
fn read_local_coordinate(
    command: &[u16],
    cursor: usize,
    start_of_argument: usize,
) -> Result<(usize, f64), ParseFailure> {
    if cursor >= command.len() {
        return Err(ParseFailure {
            message: translatable("argument.pos.missing.double", Vec::new()),
            cursor: Some(cursor),
        });
    }
    if !command[cursor..].starts_with(&[94]) {
        return Err(ParseFailure {
            message: translatable("argument.pos.mixed", Vec::new()),
            cursor: Some(start_of_argument),
        });
    }
    let start = cursor + 1;
    if command.get(start).is_some_and(|byte| *byte != 32) {
        read_double(command, start)
    } else {
        Ok((start, 0.0))
    }
}

/// `Math.round(float)`, which the JDK defines as `floor(value + 0.5)` computed
/// in exact arithmetic.
///
/// Rust's `f32::round` rounds half away from zero instead, so the two disagree
/// on every negative half: vanilla turns `-0.5` ticks into `0` and `f32::round`
/// turns it into `-1`.
///
/// Widening to `f64` before the add is what keeps the floor faithful. For a
/// value of magnitude at least `2^-30` the exact sum needs at most 53
/// significant bits and is therefore representable, and below that the sum lies
/// within `2^-30` of `0.5` and floors to zero however it was rounded. The `as`
/// cast then saturates the way Java's `(int)` narrowing conversion does, which
/// is the branch `Math.round` falls back to for a value it cannot shift.
fn java_round(value: f32) -> i32 {
    (f64::from(value) + 0.5).floor() as i32
}

// StringRepresentableArgument.java:34-36: refusal keeps the consumed cursor.
fn read_template_enum(
    command: &[u16],
    cursor: usize,
    values: &[&str],
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    let value = &command[cursor..end];
    if values
        .iter()
        .any(|candidate| candidate.encode_utf16().eq(value.iter().copied()))
    {
        Ok((end, ArgumentValue::Text(value.into())))
    } else {
        Err(ParseFailure {
            message: translatable(
                "argument.enum.invalid",
                vec![NbtSummaryValue::string(value)],
            ),
            cursor: Some(end),
        })
    }
}

/// `StringReader.readUnquotedString`'s character set, which is also what the
/// client-side dispatcher reads (`parse_unquoted_string`).
fn word_end(command: &[u16], cursor: usize) -> usize {
    scan_command_unquoted_string_utf16(command, cursor)
}

/// `StringReader.readInt` and `readFloat` accept the same characters and let
/// the conversion decide.
fn number_end(command: &[u16], cursor: usize) -> usize {
    scan_command_number_lexeme_utf16(command, cursor).map_or(cursor, |(end, _)| end)
}
