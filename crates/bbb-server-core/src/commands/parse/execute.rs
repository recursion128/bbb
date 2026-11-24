//! The `/execute` chain: the stages a parse walked and the source transforms
//! each one applies. Moved out of the parse walk unchanged.

use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::nbt_summary::{canonical_double_tag_bits, canonical_float_tag_bits};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{component_literal_nbt, CommandNbtPath, NbtSummaryValue};
use bbb_server_types::save::Java26LevelKey;

use super::super::loot_predicate::CommandLootPredicate;
use super::super::{catalog, translatable, CommandOrigin};
use super::{
    ArgumentValue, ChainStage, CommandBlockPredicate, Coordinates, ParseResult, ScoreHolderPattern,
    Selector,
};
use crate::world::ServerWorld;

/// The stages [`ChainStage`] names, in the order the parse walked them.
pub(in crate::commands) fn chain_stages(parse: &ParseResult) -> Vec<ChainStage> {
    let mut stages = Vec::new();
    for (at, node) in parse.nodes.iter().enumerate() {
        match catalog::canonical_path(node.index).as_slice() {
            ["execute", "in", "dimension"] => {
                let Some(ArgumentValue::Dimension(id)) = node.value.as_ref() else {
                    unreachable!("the dimension argument stores an identifier");
                };
                stages.push(ChainStage::ExecuteIn(id.clone()));
            }
            ["execute", "positioned", "pos"] => {
                let Some(ArgumentValue::Coordinates(coordinates)) = node.value.as_ref() else {
                    unreachable!("the positioned argument stores coordinates");
                };
                stages.push(ChainStage::ExecutePositioned(*coordinates));
            }
            ["execute", "rotated", "rot"] => {
                let Some(ArgumentValue::Rotation(coordinates)) = node.value.as_ref() else {
                    unreachable!("the rotated argument stores a rotation");
                };
                stages.push(ChainStage::ExecuteRotated(*coordinates));
            }
            ["execute", "facing", "pos"] => {
                let Some(ArgumentValue::Coordinates(coordinates)) = node.value.as_ref() else {
                    unreachable!("the facing argument stores coordinates");
                };
                stages.push(ChainStage::ExecuteFacing(*coordinates));
            }
            ["execute", "summon", "entity"] => {
                let Some(ArgumentValue::Resource(entity)) = node.value.as_ref() else {
                    unreachable!("the summon argument stores an entity type");
                };
                stages.push(ChainStage::ExecuteSummon(entity.clone()));
            }
            ["execute", "on", relation] => {
                use super::super::types::ExecuteRelation;
                let relation = match *relation {
                    "owner" => ExecuteRelation::Owner,
                    "leasher" => ExecuteRelation::Leasher,
                    "target" => ExecuteRelation::Target,
                    "attacker" => ExecuteRelation::Attacker,
                    "vehicle" => ExecuteRelation::Vehicle,
                    "controller" => ExecuteRelation::Controller,
                    "origin" => ExecuteRelation::Origin,
                    "passengers" => ExecuteRelation::Passengers,
                    _ => unreachable!("the catalog declares the entity relations"),
                };
                stages.push(ChainStage::ExecuteOn(relation));
            }
            ["execute", "as", "targets"] => {
                let Some(ArgumentValue::Players(selector)) = node.value.as_ref() else {
                    unreachable!("the as argument stores an entity selector");
                };
                stages.push(ChainStage::ExecuteAs(selector.clone()));
            }
            ["execute", "at", "targets"] => {
                let Some(ArgumentValue::Players(selector)) = node.value.as_ref() else {
                    unreachable!("the at argument stores an entity selector");
                };
                stages.push(ChainStage::ExecuteAt(selector.clone()));
            }
            ["execute", "positioned", "as", "targets"] => {
                let Some(ArgumentValue::Players(selector)) = node.value.as_ref() else {
                    unreachable!("the positioned as argument stores an entity selector");
                };
                stages.push(ChainStage::ExecutePositionedAs(selector.clone()));
            }
            ["execute", "positioned", "over", "heightmap"] => {
                let Some(ArgumentValue::Heightmap(kind)) = node.value.as_ref() else {
                    unreachable!("the positioned over argument stores a heightmap id");
                };
                stages.push(ChainStage::ExecutePositionedOver(*kind));
            }
            ["execute", "rotated", "as", "targets"] => {
                let Some(ArgumentValue::Players(selector)) = node.value.as_ref() else {
                    unreachable!("the rotated as argument stores an entity selector");
                };
                stages.push(ChainStage::ExecuteRotatedAs(selector.clone()));
            }
            ["execute", "facing", "entity", "targets", "anchor"] => {
                let Some(ArgumentValue::EntityAnchor(anchor)) = node.value.as_ref() else {
                    unreachable!("the facing entity anchor stores an entity anchor");
                };
                let Some(ArgumentValue::Players(targets)) =
                    prior_argument(parse, at, &["execute", "facing", "entity", "targets"])
                else {
                    unreachable!("facing entity parsed targets before the anchor");
                };
                stages.push(ChainStage::ExecuteFacingEntity {
                    targets: targets.clone(),
                    anchor: *anchor,
                });
            }
            ["execute", "align", "axes"] => {
                let Some(ArgumentValue::Swizzle { x, y, z }) = node.value.as_ref() else {
                    unreachable!("the align argument stores a swizzle");
                };
                stages.push(ChainStage::ExecuteAlign {
                    x: *x,
                    y: *y,
                    z: *z,
                });
            }
            ["execute", "anchored", "anchor"] => {
                let Some(ArgumentValue::EntityAnchor(anchor)) = node.value.as_ref() else {
                    unreachable!("the anchored argument stores an entity anchor");
                };
                stages.push(ChainStage::ExecuteAnchored(*anchor));
            }
            // `addConditional` gives the same node both a fork and an
            // `.executes` (`ExecuteCommand.java:883-890`); the executable one
            // is the leaf the run table answers, not a stage.
            ["execute", "if" | "unless", "block", "pos", "block"]
            | ["execute", "if" | "unless", "dimension", "dimension"]
            | ["execute", "if" | "unless", "entity", "entities"]
            | ["execute", "if" | "unless", "predicate", "predicate"]
            | ["execute", "if" | "unless", "items", "entity" | "block", _, "slots", "item_predicate"]
            | ["execute", "if" | "unless", "loaded", "pos"]
            | ["execute", "if" | "unless", "biome", "pos", "biome"]
            | ["execute", "if" | "unless", "stopwatch", "id", "range"]
            | ["execute", "if" | "unless", "score", "target", "targetObjective", "matches", "range"]
            | ["execute", "if" | "unless", "score", "target", "targetObjective", _, "source", "sourceObjective"]
            | ["execute", "if" | "unless", "data", "block", "sourcePos", "path"]
            | ["execute", "if" | "unless", "data", "entity", "source", "path"]
            | ["execute", "if" | "unless", "data", "storage", "source", "path"]
            | ["execute", "if" | "unless", "blocks", "start", "end", "destination", "all" | "masked"]
                if at + 1 < parse.nodes.len() || parse.executable_node().is_none() =>
            {
                stages.push(ChainStage::ExecuteConditional {
                    expected: catalog::canonical_path(node.index)[1] == "if",
                    condition: Box::new(execute_condition(parse, at)),
                });
            }
            // `wrapStores` (`ExecuteCommand.java:274-412`) puts the redirect on
            // the last node of each arm, so the arm is read off that node's
            // path and the arguments before it.
            ["execute", "store", kind, "score", "targets", "objective"] => {
                let Some(ArgumentValue::Objective(objective)) = node.value.as_ref() else {
                    unreachable!("the store objective argument stores an objective");
                };
                let Some(ArgumentValue::ScoreHolders(targets)) =
                    prior_argument(parse, at, &["execute", "store", kind, "score", "targets"])
                else {
                    unreachable!("the score arm parsed its holders before the objective");
                };
                stages.push(ChainStage::ExecuteStore {
                    store_result: *kind == "result",
                    target: Box::new(ExecuteStoreArm::Score {
                        targets: targets.clone(),
                        objective: objective.clone(),
                    }),
                });
            }
            ["execute", "store", kind, "bossbar", "id", into] => {
                let Some(ArgumentValue::Identifier(id)) =
                    prior_argument(parse, at, &["execute", "store", kind, "bossbar", "id"])
                else {
                    unreachable!("the bossbar arm parsed its id before the literal");
                };
                stages.push(ChainStage::ExecuteStore {
                    store_result: *kind == "result",
                    target: Box::new(ExecuteStoreArm::BossBar {
                        id: id.clone(),
                        into_value: *into == "value",
                    }),
                });
            }
            ["execute", "store", kind, name @ ("entity" | "block" | "storage"), argument, "path", tag, "scale"] =>
            {
                let Some(ArgumentValue::Double(scale)) = node.value.as_ref() else {
                    unreachable!("the store scale argument stores a double");
                };
                let Some(ArgumentValue::NbtPath(path)) = prior_argument(
                    parse,
                    at,
                    &["execute", "store", kind, name, argument, "path"],
                ) else {
                    unreachable!("a storeData arm parsed its path before the scale");
                };
                let accessor =
                    prior_argument(parse, at, &["execute", "store", kind, name, argument]);
                let provider = match accessor {
                    Some(ArgumentValue::BlockPos(pos)) => ExecuteStoreProvider::Block(*pos),
                    Some(ArgumentValue::Players(selector)) => {
                        ExecuteStoreProvider::Entity(selector.clone())
                    }
                    Some(ArgumentValue::Identifier(id)) => {
                        ExecuteStoreProvider::Storage(id.clone())
                    }
                    other => unreachable!("the store data accessor stores {other:?}"),
                };
                stages.push(ChainStage::ExecuteStore {
                    store_result: *kind == "result",
                    target: Box::new(ExecuteStoreArm::Data {
                        provider: Box::new(provider),
                        path: path.clone(),
                        tag: StoreDataTag::of_literal(tag),
                        scale: *scale,
                    }),
                });
            }
            ["return", "run"] => stages.push(ChainStage::ReturnRun),
            ["execute", "if" | "unless", "function", "name"] => {
                let Some(ArgumentValue::Function { tag, id }) = node.value.as_ref() else {
                    unreachable!("the conditional function argument stores a function");
                };
                stages.push(ChainStage::ExecuteIfFunction {
                    expected: catalog::canonical_path(node.index)[1] == "if",
                    tag: *tag,
                    id: id.clone(),
                });
            }
            _ => {}
        }
    }
    stages
}

/// One `/execute in <dimension>` stage as `CommandSourceStack.withLevel`.
///
/// `DimensionArgument.getDimension` (`DimensionArgument.java:51-59`) is the
/// lookup the modifier itself performs: a well-formed id the server does not
/// run is `argument.dimension.invalid`, not a parse refusal.
pub(in crate::commands) fn apply_execute_in(
    id: &str,
    world: &ServerWorld,
    at: CommandOrigin,
) -> Result<CommandOrigin, NbtSummaryValue> {
    let invalid = || {
        translatable(
            "argument.dimension.invalid",
            vec![component_literal_nbt(id)],
        )
    };
    let Ok(level) = Java26LevelKey::parse(id) else {
        return Err(invalid());
    };
    // `getDimension` (`DimensionArgument.java:54-56`) is `getLevel == null`,
    // not "the id is one of the three built-in names": a restored official
    // nether stem this server did not build a level for is invalid here.
    if world.level_id(&level).is_none() {
        return Err(invalid());
    }
    Ok(at.with_level(world, &level))
}

/// One `/execute positioned <pos>` stage as `withPosition` then `withAnchor(FEET)`
/// (`ExecuteCommand.java:181`).
pub(in crate::commands) fn apply_execute_positioned(
    coordinates: Coordinates,
    at: CommandOrigin,
) -> CommandOrigin {
    let position = coordinates.position(&at);
    at.with_position(position).with_anchor(EntityAnchor::Feet)
}

/// One `/execute positioned over <heightmap>` stage (`ExecuteCommand.java:192-203`).
///
/// `hasChunk` on the source xz throws `ERROR_NOT_LOADED` (`:197-199`); a
/// loaded column without a live heightmap fails the same way. The y is
/// `Level.getHeight` (`Level.java:340-352`); x, z and the source anchor stay
/// put.
pub(in crate::commands) fn apply_execute_positioned_over(
    heightmap_kind_id: i32,
    world: &ServerWorld,
    at: CommandOrigin,
) -> Result<CommandOrigin, NbtSummaryValue> {
    let [x, _, z] = at.position;
    let floor_x = x.floor() as i32;
    let floor_z = z.floor() as i32;
    let column = BlockPos {
        x: floor_x,
        y: 0,
        z: floor_z,
    };
    let not_loaded = || translatable("argument.pos.unloaded", Vec::new());
    let blocks = world.command_blocks(&at.level);
    if !blocks.has_chunk_at(column) {
        return Err(not_loaded());
    }
    let Some(height) = blocks.height_at(heightmap_kind_id, floor_x, floor_z) else {
        return Err(not_loaded());
    };
    Ok(at.with_position([x, f64::from(height), z]))
}

/// One `/execute rotated <rot>` stage as `withRotation` (`ExecuteCommand.java:209`).
pub(in crate::commands) fn apply_execute_rotated(
    coordinates: Coordinates,
    at: CommandOrigin,
) -> CommandOrigin {
    let rotation = coordinates.rotation(at.rotation);
    at.with_rotation(rotation)
}

/// One `/execute facing <pos>` stage as `facing(Vec3)` (`ExecuteCommand.java:239`).
pub(in crate::commands) fn apply_execute_facing(
    coordinates: Coordinates,
    at: CommandOrigin,
) -> CommandOrigin {
    let target = coordinates.position(&at);
    at.facing(target)
}

/// One `/execute align` stage as `withPosition(position.align(axes))`
/// (`ExecuteCommand.java:244-245`). `Vec3.align` (`Vec3.java:277-282`) floors
/// only the selected axes.
pub(in crate::commands) fn apply_execute_align(
    x: bool,
    y: bool,
    z: bool,
    at: CommandOrigin,
) -> CommandOrigin {
    let [px, py, pz] = at.position;
    at.with_position([
        if x { px.floor() } else { px },
        if y { py.floor() } else { py },
        if z { pz.floor() } else { pz },
    ])
}

/// Which `wrapStores` arm a `/execute store` stage names
/// (`ExecuteCommand.java:277-408`), carrying the arguments read before its
/// redirect; the world lookups each one owns run when the modifier does.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) enum ExecuteStoreArm {
    /// The `score` arm (`:277-294`): `getNamesWithDefaultWildcard` then
    /// `getObjective`, in that order.
    Score {
        targets: ScoreHolderPattern,
        objective: String,
    },
    /// The `bossbar` arm (`:296-303`), whose `value`/`max` literal is
    /// `storeIntoValue`.
    BossBar { id: String, into_value: bool },
    /// One `DataCommands.TARGET_PROVIDERS` arm (`:306-409`), whose tag literal
    /// and `scale` are the `IntFunction<Tag>` each redirect builds.
    Data {
        provider: Box<ExecuteStoreProvider>,
        path: CommandNbtPath,
        tag: StoreDataTag,
        scale: f64,
    },
}

/// Which `DataCommands.TARGET_PROVIDERS` accessor a `store` arm names
/// (DataCommands.java:58-60), carrying the argument that provider reads.
///
/// This is the write half of [`ExecuteDataProvider`]: `StorageDataAccessor.access`
/// only keeps the id (StorageDataAccessor.java:24-26), and the namespace is
/// pulled into the live map by `getData`, which `storeData` runs inside the
/// callback rather than while the modifier resolved the accessor.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) enum ExecuteStoreProvider {
    /// `BlockDataAccessor.PROVIDER` (BlockDataAccessor.java:30-40), whose
    /// argument is `getLoadedBlockPos` (`:33`).
    Block(Coordinates),
    /// `EntityDataAccessor.PROVIDER` (EntityDataAccessor.java:29-42), whose
    /// argument is the single-entity `EntityArgument` (`:32`).
    Entity(Selector),
    /// `StorageDataAccessor.PROVIDER` (StorageDataAccessor.java:22-37).
    Storage(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) enum ExecuteItemSource {
    Entity(Selector),
    Block(Coordinates),
}

/// The `IntFunction<Tag>` one `storeData` arm carries
/// (`ExecuteCommand.java:312-407`), which is the tag type the scaled value is
/// narrowed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoreDataTag {
    Int,
    Float,
    Short,
    Long,
    Double,
    Byte,
}

impl StoreDataTag {
    /// The literal each one is registered under (`ExecuteCommand.java:312`,
    /// `:328`, `:344`, `:360`, `:376`, `:392`).
    fn of_literal(literal: &str) -> Self {
        match literal {
            "int" => Self::Int,
            "float" => Self::Float,
            "short" => Self::Short,
            "long" => Self::Long,
            "double" => Self::Double,
            "byte" => Self::Byte,
            other => unreachable!("no store tag for the literal {other:?}"),
        }
    }

    /// `constructor.apply(value)` for `value * scale`, narrowed the way each
    /// `Tag` factory's cast does.
    ///
    /// Java narrows a `double` to `int` or `long` by saturating with NaN as
    /// zero, which is what Rust's `as` does; it narrows to `short` or `byte`
    /// in two steps (JLS 5.1.3), through that `int` and then by truncation, so
    /// those two cannot be written as a single `as`.
    pub(crate) fn tag(self, value: i32, scale: f64) -> NbtSummaryValue {
        let scaled = f64::from(value) * scale;
        match self {
            Self::Int => NbtSummaryValue::Int(scaled as i32),
            Self::Float => NbtSummaryValue::Float(canonical_float_tag_bits(scaled as f32)),
            Self::Short => NbtSummaryValue::Short(scaled as i32 as i16),
            Self::Long => NbtSummaryValue::Long(scaled as i64),
            Self::Double => NbtSummaryValue::Double(canonical_double_tag_bits(scaled)),
            Self::Byte => NbtSummaryValue::Byte(scaled as i32 as i8),
        }
    }
}

/// What one published `if`/`unless` arm asks the world.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) enum ExecuteCondition {
    /// `ExecuteCommand.java:471-483`:
    /// `BlockPredicateArgument.getBlockPredicate(..).test(new BlockInWorld(..))`
    /// over `BlockPosArgument.getLoadedBlockPos`.
    Block {
        pos: Coordinates,
        predicate: CommandBlockPredicate,
    },
    /// `ExecuteCommand.java:511-521`:
    /// `DimensionArgument.getDimension(..) == c.getSource().getLevel()`.
    Dimension(String),
    /// `ExecuteCommand.java:632-639`, the one published arm that counts rather
    /// than answers yes or no: the fork asks whether
    /// `getOptionalEntities` is empty (`:636`) and the trailing executor
    /// reports its `size()` (`:637`).
    Entity(Selector),
    Predicate(CommandLootPredicate),
    Items {
        source: ExecuteItemSource,
        slots: &'static [i32],
        predicate: super::super::item_predicate::CommandItemPredicate,
    },
    /// `ExecuteCommand.java:501-509`: `isChunkLoaded` over
    /// `BlockPosArgument.getBlockPos` (not `getLoadedBlockPos`).
    Loaded {
        pos: Coordinates,
    },
    /// `ExecuteCommand.java:486-497`: `getBiome` over `getLoadedBlockPos`
    /// tested against a `resource_or_tag` biome.
    Biome {
        pos: Coordinates,
        tag: bool,
        id: String,
    },
    /// `ExecuteCommand.checkScore` (ExecuteCommand.java:830-847). Both
    /// overloads read the target holder and its objective; what they compare
    /// the score against is [`ScoreTest`].
    Score {
        target: ScoreHolderPattern,
        objective: String,
        test: ScoreTest,
    },
    /// `ExecuteCommand.checkMatchingData` (ExecuteCommand.java:826-828):
    /// `countMatching` over the accessor the arm's provider builds.
    Data {
        provider: ExecuteDataProvider,
        path: CommandNbtPath,
    },
    /// `addIfBlocksConditional` (ExecuteCommand.java:893-897), the one arm
    /// whose answer is an `OptionalInt` rather than a count: `masked` is
    /// `checkRegions`'s `skipAir` (`:920-992`).
    Blocks {
        start: Coordinates,
        end: Coordinates,
        destination: Coordinates,
        skip_air: bool,
    },
    /// `ExecuteCommand.checkStopwatch` (ExecuteCommand.java:849-860).
    Stopwatch {
        id: String,
        min: Option<f64>,
        max: Option<f64>,
    },
}

/// Which `DataCommands.SOURCE_PROVIDERS` accessor an `if data` arm names
/// (DataCommands.java:61-63), carrying the argument that provider reads.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) enum ExecuteDataProvider {
    /// `BlockDataAccessor.PROVIDER` (BlockDataAccessor.java:43-48), whose
    /// argument is `getLoadedBlockPos` (`:33`).
    Block(Coordinates),
    /// `EntityDataAccessor.PROVIDER` (EntityDataAccessor.java:36-41), whose
    /// argument is the single-entity `EntityArgument` (`:40`).
    Entity(Selector),
    /// `StorageDataAccessor.PROVIDER` (StorageDataAccessor.java:22-37), whose
    /// argument is only an id: the namespace is pulled into the live map by
    /// `getData` (StorageDataAccessor.java:56-58).
    Storage(String),
}

/// What one `execute if score` arm compares the target's score against.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::commands) enum ScoreTest {
    /// The five comparison literals (`ExecuteCommand.java:530-603`), whose
    /// `IntBiPredicate` runs over a second holder's score (`:830-839`).
    Compare {
        operation: ScoreComparison,
        source: Box<ScoreHolderPattern>,
        objective: String,
    },
    /// `matches <range>` (`:605-614`) as `MinMaxBounds.Ints.matches` (`:846`).
    Matches { min: Option<i32>, max: Option<i32> },
}

/// The `IntBiPredicate` each comparison literal passes to `checkScore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::commands) enum ScoreComparison {
    Equal,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl ScoreComparison {
    /// The literal each one is registered under (`ExecuteCommand.java:530`,
    /// `:545`, `:560`, `:575`, `:590`).
    pub(in crate::commands) fn of_literal(literal: &str) -> Self {
        match literal {
            "=" => Self::Equal,
            "<" => Self::Less,
            "<=" => Self::LessOrEqual,
            ">" => Self::Greater,
            ">=" => Self::GreaterOrEqual,
            other => unreachable!("no score comparison for the literal {other:?}"),
        }
    }

    pub(in crate::commands) fn holds(self, target: i32, source: i32) -> bool {
        match self {
            Self::Equal => target == source,
            Self::Less => target < source,
            Self::LessOrEqual => target <= source,
            Self::Greater => target > source,
            Self::GreaterOrEqual => target >= source,
        }
    }
}

/// The nearest argument before `at` whose canonical path is `path`: a chain may
/// carry the same arm twice, and the one an arm reads is the one before it.
fn prior_argument<'a>(
    parse: &'a ParseResult,
    at: usize,
    path: &[&str],
) -> Option<&'a ArgumentValue> {
    parse.nodes[..at].iter().rev().find_map(|prior| {
        (catalog::canonical_path(prior.index).as_slice() == path)
            .then_some(prior.value.as_ref())
            .flatten()
    })
}

/// The `target`/`targetObjective` pair both `checkScore` overloads read first
/// (`ExecuteCommand.java:831-832`, `:842-843`).
fn score_target(parse: &ParseResult, at: usize, conditional: &str) -> (ScoreHolderPattern, String) {
    let Some(ArgumentValue::ScoreHolders(target)) =
        prior_argument(parse, at, &["execute", conditional, "score", "target"])
    else {
        unreachable!("the conditional target stores a score holder");
    };
    let Some(ArgumentValue::Objective(objective)) = prior_argument(
        parse,
        at,
        &["execute", conditional, "score", "target", "targetObjective"],
    ) else {
        unreachable!("the conditional target objective stores an objective");
    };
    (target.clone(), objective.clone())
}

/// The condition one published `if`/`unless` node carries, read off the parse
/// the way `addConditional`'s own lambda reads its arguments.
/// `at` is the node's place in the parse rather than its catalog index: a
/// chain may carry the same arm twice, and the position each one reads is the
/// nearest one before it.
pub(in crate::commands) fn execute_condition(parse: &ParseResult, at: usize) -> ExecuteCondition {
    let node = &parse.nodes[at];
    match catalog::canonical_path(node.index).as_slice() {
        ["execute", _, "predicate", "predicate"] => {
            let Some(ArgumentValue::LootPredicate(predicate)) = node.value.as_ref() else {
                unreachable!("the predicate argument stores a loot predicate");
            };
            ExecuteCondition::Predicate(predicate.clone())
        }
        ["execute", conditional, "items", kind, target, "slots", "item_predicate"] => {
            let Some(ArgumentValue::ItemPredicate(predicate)) = node.value.as_ref() else {
                unreachable!("items stores an item predicate");
            };
            let Some(ArgumentValue::ItemSlots(slots)) = prior_argument(
                parse,
                at,
                &["execute", conditional, "items", kind, target, "slots"],
            ) else {
                unreachable!("items stores a slot range");
            };
            let source =
                match prior_argument(parse, at, &["execute", conditional, "items", kind, target]) {
                    Some(ArgumentValue::Players(selector)) => {
                        ExecuteItemSource::Entity(selector.clone())
                    }
                    Some(ArgumentValue::BlockPos(pos)) => ExecuteItemSource::Block(*pos),
                    _ => unreachable!("items stores a source"),
                };
            ExecuteCondition::Items {
                source,
                slots,
                predicate: predicate.clone(),
            }
        }
        ["execute", _, "block", "pos", "block"] => {
            let Some(ArgumentValue::BlockPredicate(predicate)) = node.value.as_ref() else {
                unreachable!("the conditional block argument stores a block predicate");
            };
            let Some(ArgumentValue::BlockPos(pos)) =
                parse.nodes[..at].iter().rev().find_map(|prior| {
                    matches!(
                        catalog::canonical_path(prior.index).as_slice(),
                        ["execute", "if" | "unless", "block", "pos"]
                    )
                    .then(|| prior.value.as_ref())
                    .flatten()
                })
            else {
                unreachable!("the conditional block position stores coordinates");
            };
            ExecuteCondition::Block {
                pos: *pos,
                predicate: predicate.clone(),
            }
        }
        ["execute", _, "entity", "entities"] => {
            let Some(ArgumentValue::Players(selector)) = node.value.as_ref() else {
                unreachable!("the conditional entity argument stores an entity selector");
            };
            ExecuteCondition::Entity(selector.clone())
        }
        ["execute", _, "dimension", "dimension"] => {
            let Some(ArgumentValue::Dimension(id)) = node.value.as_ref() else {
                unreachable!("the conditional dimension argument stores an identifier");
            };
            ExecuteCondition::Dimension(id.clone())
        }
        ["execute", _, "loaded", "pos"] => {
            let Some(ArgumentValue::BlockPos(pos)) = node.value.as_ref() else {
                unreachable!("the conditional loaded argument stores coordinates");
            };
            ExecuteCondition::Loaded { pos: *pos }
        }
        ["execute", _, "biome", "pos", "biome"] => {
            let Some(ArgumentValue::ResourceOrTag { tag, id }) = node.value.as_ref() else {
                unreachable!("the conditional biome argument stores a resource or tag");
            };
            let Some(ArgumentValue::BlockPos(pos)) =
                parse.nodes[..at].iter().rev().find_map(|prior| {
                    matches!(
                        catalog::canonical_path(prior.index).as_slice(),
                        ["execute", "if" | "unless", "biome", "pos"]
                    )
                    .then(|| prior.value.as_ref())
                    .flatten()
                })
            else {
                unreachable!("the conditional biome position stores coordinates");
            };
            ExecuteCondition::Biome {
                pos: *pos,
                tag: *tag,
                id: id.clone(),
            }
        }
        ["execute", conditional, "score", "target", "targetObjective", operation, "source", "sourceObjective"] =>
        {
            let Some(ArgumentValue::Objective(source_objective)) = node.value.as_ref() else {
                unreachable!("the conditional source objective stores an objective");
            };
            let Some(ArgumentValue::ScoreHolders(source)) = prior_argument(
                parse,
                at,
                &[
                    "execute",
                    conditional,
                    "score",
                    "target",
                    "targetObjective",
                    operation,
                    "source",
                ],
            ) else {
                unreachable!("the conditional source stores a score holder");
            };
            let (target, objective) = score_target(parse, at, conditional);
            ExecuteCondition::Score {
                target,
                objective,
                test: ScoreTest::Compare {
                    operation: ScoreComparison::of_literal(operation),
                    source: Box::new(source.clone()),
                    objective: source_objective.clone(),
                },
            }
        }
        ["execute", conditional, "score", "target", "targetObjective", "matches", "range"] => {
            let Some(ArgumentValue::IntRange { min, max }) = node.value.as_ref() else {
                unreachable!("the conditional score range stores an int range");
            };
            let (target, objective) = score_target(parse, at, conditional);
            ExecuteCondition::Score {
                target,
                objective,
                test: ScoreTest::Matches {
                    min: *min,
                    max: *max,
                },
            }
        }
        ["execute", conditional, "data", provider @ ("block" | "entity" | "storage"), argument, "path"] =>
        {
            let Some(ArgumentValue::NbtPath(path)) = node.value.as_ref() else {
                unreachable!("the conditional data argument stores an NBT path");
            };
            let accessor = prior_argument(
                parse,
                at,
                &["execute", conditional, "data", provider, argument],
            );
            let provider = match accessor {
                Some(ArgumentValue::BlockPos(pos)) => ExecuteDataProvider::Block(*pos),
                Some(ArgumentValue::Players(selector)) => {
                    ExecuteDataProvider::Entity(selector.clone())
                }
                Some(ArgumentValue::Identifier(id)) => ExecuteDataProvider::Storage(id.clone()),
                other => unreachable!("the conditional data accessor stores {other:?}"),
            };
            ExecuteCondition::Data {
                provider,
                path: path.clone(),
            }
        }
        ["execute", conditional, "blocks", "start", "end", "destination", mode @ ("all" | "masked")] =>
        {
            let corner = |path: &[&str]| match prior_argument(parse, at, path) {
                Some(ArgumentValue::BlockPos(pos)) => *pos,
                other => unreachable!("the conditional region corner stores {other:?}"),
            };
            ExecuteCondition::Blocks {
                start: corner(&["execute", conditional, "blocks", "start"]),
                end: corner(&["execute", conditional, "blocks", "start", "end"]),
                destination: corner(&[
                    "execute",
                    conditional,
                    "blocks",
                    "start",
                    "end",
                    "destination",
                ]),
                skip_air: *mode == "masked",
            }
        }
        ["execute", _, "stopwatch", "id", "range"] => {
            let Some(ArgumentValue::FloatRange { min, max }) = node.value.as_ref() else {
                unreachable!("the conditional stopwatch range stores a float range");
            };
            let Some(ArgumentValue::Identifier(id)) =
                parse.nodes[..at].iter().rev().find_map(|prior| {
                    matches!(
                        catalog::canonical_path(prior.index).as_slice(),
                        ["execute", "if" | "unless", "stopwatch", "id"]
                    )
                    .then(|| prior.value.as_ref())
                    .flatten()
                })
            else {
                unreachable!("the conditional stopwatch id stores an identifier");
            };
            ExecuteCondition::Stopwatch {
                id: id.clone(),
                min: *min,
                max: *max,
            }
        }
        other => unreachable!("no conditional reader for {other:?}"),
    }
}

/// One `/execute anchored` stage as `withAnchor`
/// (`ExecuteCommand.java:251-252`).
pub(in crate::commands) fn apply_execute_anchored(
    anchor: EntityAnchor,
    at: CommandOrigin,
) -> CommandOrigin {
    at.with_anchor(anchor)
}
