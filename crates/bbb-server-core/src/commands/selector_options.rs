//! The `[..]` option list of a selector, read as values.
//!
//! `parseOptions` (EntitySelectorParser.java:285-325) walks the list and hands
//! each key to `EntitySelectorOptions.get` (EntitySelectorOptions.java:487-499);
//! every modifier `bootStrap` registers is read here with the refusal and cursor
//! it throws. The shared prefix parser in `bbb_protocol` still decides where the
//! selector ends and which flags `EntityArgument` checks.

use bbb_protocol::{
    component_literal_nbt,
    entity_types::vanilla_entity_type_for_resource_id,
    game_types::{vanilla_game_type_for_serialized_name, GameType},
    read_command_string_value_utf16, scan_command_resource_location_utf16,
    scan_command_unquoted_string_utf16, scan_command_whitespace_utf16, JavaString, NbtSummaryValue,
    VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};

use super::parse::{
    read_compound_tag, read_double, read_float_range, read_int, read_int_range, string_failure,
    ArgumentValue, ParseFailure, SelectorKind,
};
use super::selector_advancements::{self, AdvancementRequirement};
use super::translatable;

/// What an option list added to the selector.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct SelectorOptions {
    /// `EntitySelectorParser.predicates`, in the order the options added them.
    pub predicates: Vec<SelectorPredicate>,
    /// `setMaxResults` (EntitySelectorOptions.java:173).
    pub max_results: Option<i32>,
    /// `setOrder` (`:181-190`).
    pub order: Option<SelectorOrder>,
    /// `setWorldLimited` (EntitySelectorParser.java:357-359).
    pub world_limited: bool,
    pub distance: Option<SelectorBounds<f64>>,
    pub level: Option<SelectorBounds<i32>>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub delta_x: Option<f64>,
    pub delta_y: Option<f64>,
    pub delta_z: Option<f64>,
    pub rot_x: Option<SelectorBounds<f32>>,
    pub rot_y: Option<SelectorBounds<f32>>,
}

/// `MinMaxBounds.Bounds` (MinMaxBounds.java:49-52).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SelectorBounds<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl SelectorBounds<f64> {
    /// `MinMaxBounds.Doubles.matchesSqr` (MinMaxBounds.java:233-237) over the
    /// `boundsSqr` the constructor squares (`:209-211`).
    pub(super) fn matches_sqr(self, value_sqr: f64) -> bool {
        let min = self.min.map(|min| min * min);
        let max = self.max.map(|max| max * max);
        if min.is_some_and(|min| min > value_sqr) {
            return false;
        }
        max.is_none_or(|max| max.partial_cmp(&value_sqr) != Some(std::cmp::Ordering::Less))
    }
}

impl SelectorBounds<i32> {
    /// `MinMaxBounds.Ints.matches` (MinMaxBounds.java:296-298).
    pub(super) fn matches(self, value: i32) -> bool {
        if self.min.is_some_and(|min| min > value) {
            return false;
        }
        self.max.is_none_or(|max| max >= value)
    }
}

/// One `parser.addPredicate` an option registered.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum SelectorPredicate {
    /// `name=` (EntitySelectorOptions.java:105).
    Name { name: JavaString, inverted: bool },
    /// `gamemode=` (`:236-243`).
    GameMode { game_type: GameType, inverted: bool },
    /// `team=` (`:253-257`).
    Team { name: String, inverted: bool },
    /// `type=<id>` (`:299`).
    Type { id: String, inverted: bool },
    /// `type=#<tag>` (`:287-288`).
    TypeTag { tag: String, inverted: bool },
    /// `tag=` (`:308`).
    Tag { name: String, inverted: bool },
    /// `nbt=` (`:313-326`).
    Nbt {
        tag: NbtSummaryValue,
        inverted: bool,
    },
    /// `scores={..}` (`:350-370`), one bound per objective.
    Scores(Vec<(String, SelectorBounds<i32>)>),
    /// `advancements={..}` (`:433-449`).
    Advancements(Vec<AdvancementRequirement>),
    /// `predicate=` (`:460-479`).
    Predicate { id: String, inverted: bool },
}

/// The four orders `sort=` names (EntitySelectorOptions.java:181-190).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectorOrder {
    /// `EntitySelector.ORDER_ARBITRARY`
    /// (commands/arguments/selector/EntitySelector.java:32), which leaves the
    /// gathered list alone.
    Arbitrary,
    /// `EntitySelectorParser.ORDER_NEAREST` (EntitySelectorParser.java:64-66).
    Nearest,
    /// `ORDER_FURTHEST` (`:67-69`).
    Furthest,
    /// `ORDER_RANDOM` (`:70`).
    Random,
}

/// A refusal raised while reading the list, with its offset inside the selector.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SelectorOptionFailure {
    pub message: NbtSummaryValue,
    pub offset: Option<usize>,
}

impl From<ParseFailure> for SelectorOptionFailure {
    fn from(failure: ParseFailure) -> Self {
        Self {
            message: failure.message,
            offset: failure.cursor,
        }
    }
}

type Read<T> = Result<T, SelectorOptionFailure>;

/// Reads the option list of `selector`, which starts at its `@` and was read as
/// `kind`.
///
/// The answer covers only the list: a selector without one reads as no options
/// at all, which is `parseSelector`'s `peek() != '['` arm
/// (EntitySelectorParser.java:254).
#[cfg(test)]
pub(super) fn read_options(selector: &[u16], kind: SelectorKind) -> Read<SelectorOptions> {
    read_options_with_registry(
        selector,
        kind,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(super) fn read_options_with_registry(
    selector: &[u16],
    kind: SelectorKind,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Read<SelectorOptions> {
    if selector.get(2) != Some(&91) {
        return Ok(SelectorOptions::default());
    }
    OptionListReader {
        reader: OptionReader {
            input: selector,
            cursor: 3,
        },
        state: SelectorState::of(kind),
        options: SelectorOptions::default(),
        content_registry,
    }
    .parse_options()
}

/// The `EntitySelectorParser` flags the `canUse` predicates and the modifiers'
/// own inapplicability checks read.
struct SelectorState {
    /// `isCurrentEntity` (EntitySelectorParser.java:533-535), which only `@s`
    /// sets (`:246`).
    current_entity: bool,
    /// `isTypeLimited` (`:617-619`): `limitToType` ran, which `@a`, `@p` and
    /// `@r` do for the player type (`:210`, `:233`, `:240`).
    type_limited: bool,
    type_inverse: bool,
    limited: bool,
    sorted: bool,
    name_equals: bool,
    name_not_equals: bool,
    gamemode_equals: bool,
    gamemode_not_equals: bool,
    team_equals: bool,
    scores: bool,
    advancements: bool,
}

impl SelectorState {
    fn of(kind: SelectorKind) -> Self {
        Self {
            current_entity: matches!(kind, SelectorKind::Source),
            type_limited: matches!(
                kind,
                SelectorKind::AllPlayers | SelectorKind::NearestPlayer | SelectorKind::RandomPlayer
            ),
            type_inverse: false,
            limited: false,
            sorted: false,
            name_equals: false,
            name_not_equals: false,
            gamemode_equals: false,
            gamemode_not_equals: false,
            team_equals: false,
            scores: false,
            advancements: false,
        }
    }
}

struct OptionListReader<'a> {
    reader: OptionReader<'a>,
    state: SelectorState,
    options: SelectorOptions,
    content_registry: &'a bbb_protocol::content_registry::ContentRegistry,
}

impl OptionListReader<'_> {
    /// `EntitySelectorParser.parseOptions` (EntitySelectorParser.java:285-325).
    fn parse_options(mut self) -> Read<SelectorOptions> {
        self.reader.skip_whitespace();
        while self.reader.peek().is_some_and(|next| next != 93) {
            self.reader.skip_whitespace();
            let start = self.reader.cursor;
            let key = self.reader.read_string()?;
            // `EntitySelectorOptions.get` (EntitySelectorOptions.java:487-499):
            // an unknown key rewinds to where it began (`:496`) while an
            // inapplicable one reports from where the key ended.
            let Some(option) = VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS
                .iter()
                .find(|option| key == option.name)
            else {
                return Err(refusal(
                    translatable(
                        "argument.entity.options.unknown",
                        vec![component_literal_nbt(&key)],
                    ),
                    start,
                ));
            };
            let key = option.name;
            if !self.can_use(key) {
                return Err(inapplicable(key, self.reader.cursor));
            }
            self.reader.skip_whitespace();
            if !self.reader.consume('=') {
                // `:295-297`.
                return Err(refusal(
                    translatable(
                        "argument.entity.options.valueless",
                        vec![component_literal_nbt(key)],
                    ),
                    start,
                ));
            }
            self.reader.skip_whitespace();
            self.read_value(key)?;
            self.reader.skip_whitespace();
            match self.reader.peek() {
                Some(44) => {
                    self.reader.bump();
                }
                // EntitySelectorParser.java:307-311.
                Some(93) => break,
                Some(_) => {
                    return Err(refusal(
                        translatable("argument.entity.options.unterminated", Vec::new()),
                        self.reader.cursor,
                    ));
                }
                None => break,
            }
        }
        if !self.reader.consume(']') {
            // EntitySelectorParser.java:322-323.
            return Err(refusal(
                translatable("argument.entity.options.unterminated", Vec::new()),
                self.reader.cursor,
            ));
        }
        Ok(self.options)
    }

    /// The `canUse` predicate `bootStrap` registers beside each modifier
    /// (EntitySelectorOptions.java:88-484).
    fn can_use(&self, option: &str) -> bool {
        let state = &self.state;
        let options = &self.options;
        match option {
            "name" => !state.name_equals,
            "distance" => options.distance.is_none(),
            "level" => options.level.is_none(),
            "x" => options.x.is_none(),
            "y" => options.y.is_none(),
            "z" => options.z.is_none(),
            "dx" => options.delta_x.is_none(),
            "dy" => options.delta_y.is_none(),
            "dz" => options.delta_z.is_none(),
            "x_rotation" => options.rot_x.is_none(),
            "y_rotation" => options.rot_y.is_none(),
            "limit" => !state.current_entity && !state.limited,
            "sort" => !state.current_entity && !state.sorted,
            "gamemode" => !state.gamemode_equals,
            "team" => !state.team_equals,
            "type" => !state.type_limited,
            "scores" => !state.scores,
            "advancements" => !state.advancements,
            _ => true,
        }
    }

    fn read_value(&mut self, key: &str) -> Read<()> {
        match key {
            "name" => self.read_name_option(),
            "distance" => self.read_distance_option(),
            "level" => self.read_level_option(),
            "x" => {
                self.options.world_limited = true;
                self.options.x = Some(self.reader.read_double()?);
                Ok(())
            }
            "y" => {
                self.options.world_limited = true;
                self.options.y = Some(self.reader.read_double()?);
                Ok(())
            }
            "z" => {
                self.options.world_limited = true;
                self.options.z = Some(self.reader.read_double()?);
                Ok(())
            }
            "dx" => {
                self.options.world_limited = true;
                self.options.delta_x = Some(self.reader.read_double()?);
                Ok(())
            }
            "dy" => {
                self.options.world_limited = true;
                self.options.delta_y = Some(self.reader.read_double()?);
                Ok(())
            }
            "dz" => {
                self.options.world_limited = true;
                self.options.delta_z = Some(self.reader.read_double()?);
                Ok(())
            }
            "x_rotation" => {
                self.options.rot_x = Some(self.reader.read_float_degrees()?);
                Ok(())
            }
            "y_rotation" => {
                self.options.rot_y = Some(self.reader.read_float_degrees()?);
                Ok(())
            }
            "limit" => self.read_limit_option(),
            "sort" => self.read_sort_option(),
            "gamemode" => self.read_gamemode_option(),
            "team" => {
                self.read_team_option();
                Ok(())
            }
            "type" => self.read_type_option(),
            "tag" => {
                self.read_tag_option();
                Ok(())
            }
            "nbt" => self.read_nbt_option(),
            "scores" => self.read_scores_option(),
            "advancements" => self.read_advancements_option(),
            "predicate" => self.read_predicate_option(),
            other => unreachable!("{other} is not a registered selector option"),
        }
    }

    /// `register("name")` (EntitySelectorOptions.java:90-106).
    fn read_name_option(&mut self) -> Read<()> {
        let start = self.reader.cursor;
        let inverted = self.reader.read_inversion();
        let name = self.reader.read_string()?;
        if self.state.name_not_equals && !inverted {
            return Err(inapplicable("name", start));
        }
        if inverted {
            self.state.name_not_equals = true;
        } else {
            self.state.name_equals = true;
        }
        self.options
            .predicates
            .push(SelectorPredicate::Name { name, inverted });
        Ok(())
    }

    /// `register("distance")` (EntitySelectorOptions.java:107-117).
    fn read_distance_option(&mut self) -> Read<()> {
        let start = self.reader.cursor;
        let bounds = self.reader.read_double_bounds()?;
        if bounds.min.is_some_and(|min| min < 0.0) || bounds.max.is_some_and(|max| max < 0.0) {
            return Err(refusal(
                translatable("argument.entity.options.distance.negative", Vec::new()),
                start,
            ));
        }
        self.options.distance = Some(bounds);
        self.options.world_limited = true;
        Ok(())
    }

    /// `register("level")` (EntitySelectorOptions.java:118-128).
    fn read_level_option(&mut self) -> Read<()> {
        let start = self.reader.cursor;
        let bounds = self.reader.read_int_bounds()?;
        if bounds.min.is_some_and(|min| min < 0) || bounds.max.is_some_and(|max| max < 0) {
            return Err(refusal(
                translatable("argument.entity.options.level.negative", Vec::new()),
                start,
            ));
        }
        self.options.level = Some(bounds);
        Ok(())
    }

    /// `register("limit")` (EntitySelectorOptions.java:165-175).
    fn read_limit_option(&mut self) -> Read<()> {
        let start = self.reader.cursor;
        let count = self.reader.read_int()?;
        if count < 1 {
            return Err(refusal(
                translatable("argument.entity.options.limit.toosmall", Vec::new()),
                start,
            ));
        }
        self.options.max_results = Some(count);
        self.state.limited = true;
        Ok(())
    }

    /// `register("sort")` (EntitySelectorOptions.java:176-192).
    fn read_sort_option(&mut self) -> Read<()> {
        let start = self.reader.cursor;
        let name = self.reader.read_unquoted_string();
        let order = match name.as_str() {
            "nearest" => SelectorOrder::Nearest,
            "furthest" => SelectorOrder::Furthest,
            "random" => SelectorOrder::Random,
            "arbitrary" => SelectorOrder::Arbitrary,
            _ => {
                return Err(refusal(
                    translatable(
                        "argument.entity.options.sort.irreversible",
                        vec![component_literal_nbt(&name)],
                    ),
                    start,
                ));
            }
        };
        self.options.order = Some(order);
        self.state.sorted = true;
        Ok(())
    }

    /// `register("gamemode")` (EntitySelectorOptions.java:193-249).
    fn read_gamemode_option(&mut self) -> Read<()> {
        let start = self.reader.cursor;
        let inverted = self.reader.read_inversion();
        if self.state.gamemode_not_equals && !inverted {
            return Err(inapplicable("gamemode", start));
        }
        let name = self.reader.read_unquoted_string();
        let Some(entry) = vanilla_game_type_for_serialized_name(&name) else {
            return Err(refusal(
                translatable(
                    "argument.entity.options.mode.invalid",
                    vec![component_literal_nbt(&name)],
                ),
                start,
            ));
        };
        self.options.predicates.push(SelectorPredicate::GameMode {
            game_type: entry.game_type,
            inverted,
        });
        if inverted {
            self.state.gamemode_not_equals = true;
        } else {
            self.state.gamemode_equals = true;
        }
        Ok(())
    }

    /// `register("team")` (EntitySelectorOptions.java:250-263).
    fn read_team_option(&mut self) {
        let inverted = self.reader.read_inversion();
        let name = self.reader.read_unquoted_string();
        self.options
            .predicates
            .push(SelectorPredicate::Team { name, inverted });
        if !inverted {
            self.state.team_equals = true;
        }
    }

    /// `register("type")` (EntitySelectorOptions.java:264-304).
    fn read_type_option(&mut self) -> Read<()> {
        let start = self.reader.cursor;
        let inverted = self.reader.read_inversion();
        if self.state.type_inverse && !inverted {
            return Err(inapplicable("type", start));
        }
        if inverted {
            self.state.type_inverse = true;
        }
        if self.reader.read_tag_prefix() {
            let tag = self.reader.read_identifier()?;
            self.options
                .predicates
                .push(SelectorPredicate::TypeTag { tag, inverted });
            return Ok(());
        }
        let id = self.reader.read_identifier()?;
        if vanilla_entity_type_for_resource_id(&id).is_none()
            && self.content_registry.entity_by_resource_id(&id).is_none()
        {
            // `:291-294`, which words the identifier it read.
            return Err(refusal(
                translatable(
                    "argument.entity.options.type.invalid",
                    vec![component_literal_nbt(&id)],
                ),
                start,
            ));
        }
        if !inverted {
            self.state.type_limited = true;
        }
        self.options
            .predicates
            .push(SelectorPredicate::Type { id, inverted });
        Ok(())
    }

    /// `register("tag")` (EntitySelectorOptions.java:305-309).
    fn read_tag_option(&mut self) {
        let inverted = self.reader.read_inversion();
        let name = self.reader.read_unquoted_string();
        self.options
            .predicates
            .push(SelectorPredicate::Tag { name, inverted });
    }

    /// `register("nbt")` (EntitySelectorOptions.java:310-327).
    fn read_nbt_option(&mut self) -> Read<()> {
        let inverted = self.reader.read_inversion();
        let tag = self.reader.read_compound()?;
        self.options
            .predicates
            .push(SelectorPredicate::Nbt { tag, inverted });
        Ok(())
    }

    /// `register("scores")` (EntitySelectorOptions.java:328-374).
    fn read_scores_option(&mut self) -> Read<()> {
        let reader = &mut self.reader;
        reader.expect('{')?;
        reader.skip_whitespace();
        let mut expected: Vec<(String, SelectorBounds<i32>)> = Vec::new();
        while reader.peek().is_some_and(|next| next != 125) {
            reader.skip_whitespace();
            let name = reader.read_unquoted_string();
            reader.skip_whitespace();
            reader.expect('=')?;
            reader.skip_whitespace();
            let bounds = reader.read_int_bounds()?;
            // `expected.put` over a `Maps.newHashMap` (`:341`).
            expected.retain(|(held, _)| *held != name);
            expected.push((name, bounds));
            reader.skip_whitespace();
            if reader.peek() == Some(44) {
                reader.bump();
            }
        }
        reader.expect('}')?;
        if !expected.is_empty() {
            self.options
                .predicates
                .push(SelectorPredicate::Scores(expected));
        }
        self.state.scores = true;
        Ok(())
    }

    /// `register("advancements")` (EntitySelectorOptions.java:375-454), whose
    /// payload [`selector_advancements`] owns.
    fn read_advancements_option(&mut self) -> Read<()> {
        let requirements = selector_advancements::read_map(&mut self.reader)?;
        self.state.advancements = true;
        if !requirements.is_empty() {
            self.options
                .predicates
                .push(SelectorPredicate::Advancements(requirements));
        }
        Ok(())
    }

    /// `register("predicate")` (EntitySelectorOptions.java:455-483).
    fn read_predicate_option(&mut self) -> Read<()> {
        let inverted = self.reader.read_inversion();
        let id = self.reader.read_identifier()?;
        self.options
            .predicates
            .push(SelectorPredicate::Predicate { id, inverted });
        Ok(())
    }
}

fn refusal(message: NbtSummaryValue, offset: usize) -> SelectorOptionFailure {
    SelectorOptionFailure {
        message,
        offset: Some(offset),
    }
}

fn inapplicable(option: &str, offset: usize) -> SelectorOptionFailure {
    refusal(
        translatable(
            "argument.entity.options.inapplicable",
            vec![component_literal_nbt(option)],
        ),
        offset,
    )
}

/// A cursor over the option list, with the `StringReader` primitives the
/// modifiers call.
pub(super) struct OptionReader<'a> {
    input: &'a [u16],
    cursor: usize,
}

impl OptionReader<'_> {
    /// `EntitySelectorParser.shouldInvertValue`
    /// (EntitySelectorParser.java:327-336).
    fn read_inversion(&mut self) -> bool {
        self.skip_whitespace();
        if self.consume('!') {
            self.skip_whitespace();
            true
        } else {
            false
        }
    }

    /// `EntitySelectorParser.isTag` (`:338-347`).
    fn read_tag_prefix(&mut self) -> bool {
        self.skip_whitespace();
        if self.consume('#') {
            self.skip_whitespace();
            true
        } else {
            false
        }
    }

    fn read_int(&mut self) -> Read<i32> {
        let (end, value) = read_int(self.input, self.cursor)?;
        self.cursor = end;
        Ok(value)
    }

    fn read_double(&mut self) -> Read<f64> {
        let (end, value) = read_double(self.input, self.cursor)?;
        self.cursor = end;
        Ok(value)
    }

    /// `MinMaxBounds.Ints.fromReader` (MinMaxBounds.java:306-317).
    pub(super) fn read_int_bounds(&mut self) -> Read<SelectorBounds<i32>> {
        let (end, value) = read_int_range(self.input, self.cursor)?;
        let ArgumentValue::IntRange { min, max } = value else {
            unreachable!("the int range reader answers an int range");
        };
        self.cursor = end;
        Ok(SelectorBounds { min, max })
    }

    /// `MinMaxBounds.Doubles.fromReader` (MinMaxBounds.java:239-250).
    fn read_double_bounds(&mut self) -> Read<SelectorBounds<f64>> {
        let (end, value) = read_float_range(self.input, self.cursor)?;
        let ArgumentValue::FloatRange { min, max } = value else {
            unreachable!("the double range reader answers a double range");
        };
        self.cursor = end;
        Ok(SelectorBounds { min, max })
    }

    /// `MinMaxBounds.FloatDegrees.fromReader` (MinMaxBounds.java:260-265): the
    /// `Bounds.fromReader` grammar (`:139-168`) with `Float::parseFloat` and no
    /// swapped check.
    fn read_float_degrees(&mut self) -> Read<SelectorBounds<f32>> {
        let start = self.cursor;
        if start >= self.input.len() {
            return Err(refusal(
                translatable("argument.range.empty", Vec::new()),
                start,
            ));
        }
        let min = self.read_float_bound(start)?;
        let max = if self.input[self.cursor..].starts_with(&[46, 46]) {
            self.cursor += 2;
            self.read_float_bound(start)?
        } else {
            min
        };
        if min.is_none() && max.is_none() {
            self.cursor = start;
            return Err(refusal(
                translatable("argument.range.empty", Vec::new()),
                start,
            ));
        }
        Ok(SelectorBounds { min, max })
    }

    /// `MinMaxBounds.Bounds.readNumber` (MinMaxBounds.java:170-199), whose
    /// refusal the outer catch moves to the start of the range (`:164-167`).
    fn read_float_bound(&mut self, start: usize) -> Read<Option<f32>> {
        let (end, value) = super::parse::read_range_number(
            self.input,
            self.cursor,
            start,
            "parsing.float.invalid",
        )?;
        self.cursor = end;
        Ok(value)
    }

    /// `TagParser.parseCompoundAsArgument` (TagParser.java:83-86).
    fn read_compound(&mut self) -> Read<NbtSummaryValue> {
        let (end, value) = read_compound_tag(self.input, self.cursor)?;
        let ArgumentValue::CompoundTag(tag) = value else {
            unreachable!("the compound reader answers a compound");
        };
        self.cursor = end;
        Ok(tag)
    }

    /// `Identifier.read` (Identifier.java:189-199): the greedy run, rewound to
    /// its start when it does not parse.
    pub(super) fn read_identifier(&mut self) -> Read<String> {
        let start = self.cursor;
        match scan_command_resource_location_utf16(self.input, start) {
            Ok((end, id)) => {
                self.cursor = end;
                Ok(id)
            }
            Err(_) => Err(refusal(
                translatable("argument.id.invalid", Vec::new()),
                start,
            )),
        }
    }

    /// `StringReader.readString` (com/mojang/brigadier/StringReader.java:233-245).
    pub(super) fn read_string(&mut self) -> Read<JavaString> {
        let (end, value) =
            read_command_string_value_utf16(self.input, self.cursor).map_err(string_failure)?;
        self.cursor = end;
        Ok(value)
    }

    /// `StringReader.readBoolean` (com/mojang/brigadier/StringReader.java:247-264).
    pub(super) fn read_boolean(&mut self) -> Read<bool> {
        let start = self.cursor;
        let value = self.read_string()?;
        match value {
            value if value.is_empty() => Err(refusal(
                translatable("parsing.bool.expected", Vec::new()),
                self.cursor,
            )),
            value if value == "true" => Ok(true),
            value if value == "false" => Ok(false),
            _ => {
                self.cursor = start;
                Err(refusal(
                    translatable("parsing.bool.invalid", vec![component_literal_nbt(&value)]),
                    start,
                ))
            }
        }
    }

    /// `StringReader.expect` (com/mojang/brigadier/StringReader.java:266-272).
    pub(super) fn expect(&mut self, symbol: char) -> Read<()> {
        if self.consume(symbol) {
            return Ok(());
        }
        Err(refusal(
            translatable(
                "parsing.expected",
                vec![component_literal_nbt(symbol.to_string())],
            ),
            self.cursor,
        ))
    }

    pub(super) fn read_unquoted_string(&mut self) -> String {
        let start = self.cursor;
        self.cursor = scan_command_unquoted_string_utf16(self.input, start);
        JavaString::with_utf8_units(&self.input[start..self.cursor], str::to_owned)
            .expect("an unquoted word is ASCII")
    }

    pub(super) fn read_unquoted_java_string(&mut self) -> JavaString {
        let start = self.cursor;
        self.cursor = scan_command_unquoted_string_utf16(self.input, start);
        self.input[start..self.cursor].into()
    }

    pub(super) fn skip_whitespace(&mut self) {
        self.cursor = scan_command_whitespace_utf16(self.input, self.cursor);
    }

    pub(super) fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected as u16) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn peek(&self) -> Option<u16> {
        self.input.get(self.cursor).copied()
    }

    pub(super) fn bump(&mut self) -> Option<u16> {
        let next = self.peek()?;
        self.cursor += 1;
        Some(next)
    }
}

#[cfg(test)]
mod tests;
