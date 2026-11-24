//! `EntitySelector.findEntities` and `findPlayers`
//! (commands/arguments/selector/EntitySelector.java:137-244) over the live
//! entities, with the predicates `EntitySelectorOptions.bootStrap` registers.
//!
//! A predicate that cannot be answered the way vanilla would refuses the
//! whole command with `command.failed` rather than narrowing the result set.

pub(super) mod loot;
mod names;
pub(crate) use names::block_description_id_opt;
pub(super) use names::{entity_name, item_name_from_nbt_with_registry};

use crate::entity::store::SectionKey;
use crate::entity::EntityRegistryTagView;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::block_state_parse::block_entity_data_matches;
use bbb_server_types::geometry::Aabb;
use bbb_server_types::interaction::player_pose_bounding_box;
use uuid::Uuid;

use super::*;
use crate::commands::selector_options::SelectorBounds;
use crate::commands::types::CommandStoredEntity;
use crate::world::EntityReads;
use bbb_server_types::save::Java26LevelKey;

type Refusal = NbtSummaryValue;

/// `EntitySelector.findEntities` (`:137-180`).
pub(in crate::commands) fn select_damage_entity_states<'a>(
    selector: &Selector,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'a>,
) -> Result<Vec<CommandDamageEntityRef<'a>>, Refusal> {
    // `:139-141`.
    if !selector.includes_entities {
        return Ok(select_player_states(selector, source, at, world)?
            .into_iter()
            .map(CommandDamageEntityRef::Player)
            .collect());
    }
    let kind = match &selector.form {
        SelectorForm::Named(name) => {
            let found = match name.with_utf8(parse_command_java_uuid).flatten() {
                // `:143-146`.
                None => player_by_name(world, name).map(CommandDamageEntityRef::Player),
                // `:148-159`: the level holding the UUID answers.
                Some(uuid) => world.command_entity_by_uuid(Uuid::from_u128(uuid)),
            };
            return Ok(found.into_iter().collect());
        }
        SelectorForm::Kind(kind) => *kind,
    };
    let query = SelectorQuery::new(selector, kind, at, world);
    // `:163-166`.
    if kind == SelectorKind::Source {
        return Ok(match source_entity_state(source, world) {
            Some(entity) if query.admits(entity)? => vec![entity],
            _ => Vec::new(),
        });
    }
    let limit = query.result_limit();
    let mut found = Vec::new();
    let options = &selector.options;
    let level_filter = options.world_limited.then_some(&at.level);
    let skip_players = query.type_limit.is_some_and(|id| id != "minecraft:player");
    if let Some(aabb) = query.aabb {
        let players = (!skip_players)
            .then(|| {
                world
                    .players()
                    .filter(|player| !options.world_limited || *player.dimension == at.level)
                    .map(CommandDamageEntityRef::Player)
            })
            .into_iter()
            .flatten();
        let stored = world
            .stored_entities_in_box(aabb, level_filter)
            .into_iter()
            .map(CommandDamageEntityRef::Stored);
        for entity in players.chain(stored) {
            if found.len() >= limit {
                break;
            }
            if query
                .type_limit
                .is_some_and(|id| entity.type_resource_id() != id)
            {
                continue;
            }
            if !in_walked_section(aabb, block_section(entity))
                || !aabb.intersects(bounding_box(entity))
            {
                continue;
            }
            if query.admits_inner(entity, false)? {
                found.push(entity);
            }
        }
    } else {
        let players = (!skip_players)
            .then(|| {
                world
                    .players()
                    .filter(|player| !options.world_limited || *player.dimension == at.level)
                    .map(CommandDamageEntityRef::Player)
            })
            .into_iter()
            .flatten();
        let stored = world
            .stored_entities()
            .filter(|stored| !options.world_limited || stored.dimension == &at.level)
            .map(CommandDamageEntityRef::Stored);
        for entity in players.chain(stored) {
            if found.len() >= limit {
                break;
            }
            if query
                .type_limit
                .is_some_and(|id| entity.type_resource_id() != id)
            {
                continue;
            }
            if query.admits_inner(entity, false)? {
                found.push(entity);
            }
        }
    }
    Ok(query.sort_and_limit(found, CommandDamageEntityRef::position))
}

/// `EntitySelector.findPlayers` (`:207-244`).
///
/// A `@e` or `@n` resolved here rather than through
/// [`select_damage_entity_states`] is answered with the players alone, which is
/// the half of `getEntities` a caller that can only act on players reaches.
pub(in crate::commands) fn select_player_states<'a>(
    selector: &Selector,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'a>,
) -> Result<Vec<CommandPlayer<'a>>, Refusal> {
    let kind = match &selector.form {
        // `parseNameOrUUID` (EntitySelectorParser.java:261-283) sets exactly one
        // of the two fields `EntitySelector.findPlayers` reads
        // (EntitySelector.java:209-217).
        SelectorForm::Named(name) => {
            let found = match name.with_utf8(parse_command_java_uuid).flatten() {
                Some(uuid) => world.players().find(|player| player.id.as_u128() == uuid),
                None => player_by_name(world, name),
            };
            return Ok(found.into_iter().collect());
        }
        SelectorForm::Kind(kind) => *kind,
    };
    let query = SelectorQuery::new(selector, kind, at, world);
    // EntitySelector.java:222-224.
    if kind == SelectorKind::Source {
        return Ok(match source_player_state(source, world) {
            Some(player) if query.admits(CommandDamageEntityRef::Player(player))? => vec![player],
            _ => Vec::new(),
        });
    }
    // EntitySelector.java:226-241: the source level's own players when the selector is world
    // limited, the player list otherwise, both cut at the result limit.
    let limit = query.result_limit();
    let mut found = Vec::new();
    for player in world
        .players()
        .filter(|player| !selector.options.world_limited || *player.dimension == at.level)
    {
        if found.len() >= limit {
            break;
        }
        if query.admits(CommandDamageEntityRef::Player(player))? {
            found.push(player);
        }
    }
    Ok(query.sort_and_limit(found, |player| player.position))
}

/// `PlayerList.getPlayerByName` (PlayerList.java:584-595).
fn player_by_name<'a>(
    world: &EntityReads<'a>,
    name: &bbb_protocol::JavaString,
) -> Option<CommandPlayer<'a>> {
    world
        .players()
        .find(|player| player_name_matches(player.name, name))
}

/// One `EntitySelector` as `getSelector` (EntitySelectorParser.java:122-157)
/// built it, bound to the source it is resolved from.
struct SelectorQuery<'s, 'a> {
    selector: &'s Selector,
    kind: SelectorKind,
    world: &'s EntityReads<'a>,
    level: &'s Java26LevelKey,
    /// `position.apply(sender.getPosition())` (EntitySelector.java:161).
    position: [f64; 3],
    /// `getAbsoluteAabb` (`:246-248`).
    aabb: Option<Aabb>,
    /// `EntitySelectorParser.limitToType`'s type (EntitySelectorOptions.java:300-302).
    type_limit: Option<&'s str>,
}

impl<'s, 'a> SelectorQuery<'s, 'a> {
    fn new(
        selector: &'s Selector,
        kind: SelectorKind,
        at: &'s CommandOrigin,
        world: &'s EntityReads<'a>,
    ) -> Self {
        let options = &selector.options;
        let origin = at.position;
        let position = [
            options.x.unwrap_or(origin[0]),
            options.y.unwrap_or(origin[1]),
            options.z.unwrap_or(origin[2]),
        ];
        let aabb = relative_aabb(&selector.options).map(|aabb| {
            aabb.moved(Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            })
        });
        let type_limit = options
            .predicates
            .iter()
            .find_map(|predicate| match predicate {
                SelectorPredicate::Type {
                    id,
                    inverted: false,
                } => Some(id.as_str()),
                _ => None,
            });
        Self {
            selector,
            kind,
            world,
            level: &at.level,
            position,
            aabb,
            type_limit,
        }
    }

    /// `getResultLimit` (EntitySelector.java:193-195).
    fn result_limit(&self) -> usize {
        if self.selector.order() == SelectorOrder::Arbitrary {
            usize::try_from(self.selector.max_results()).unwrap_or(usize::MAX)
        } else {
            usize::MAX
        }
    }

    /// `getPredicate` (EntitySelector.java:250-277): `Util.allOf` over the option predicates in
    /// the order they were added, the volume, then the distance range.
    fn admits(&self, entity: CommandDamageEntityRef<'a>) -> Result<bool, Refusal> {
        self.admits_inner(entity, true)
    }

    fn admits_inner(
        &self,
        entity: CommandDamageEntityRef<'a>,
        check_aabb: bool,
    ) -> Result<bool, Refusal> {
        let options = &self.selector.options;
        // `parseSelector` adds `Entity::isAlive` for `@e` and `@n` before any
        // option (EntitySelectorParser.java:249-251).
        if matches!(
            self.kind,
            SelectorKind::AllEntities | SelectorKind::NearestEntity
        ) && !is_alive(entity)
        {
            return Ok(false);
        }
        for predicate in &options.predicates {
            if !self.predicate_holds(predicate, entity)? {
                return Ok(false);
            }
        }
        // `finalizePredicates` (EntitySelectorParser.java:172-184).
        let [y_rot, x_rot] = entity.rotation();
        if options
            .rot_x
            .is_some_and(|range| !rotation_matches(range, x_rot))
            || options
                .rot_y
                .is_some_and(|range| !rotation_matches(range, y_rot))
        {
            return Ok(false);
        }
        if let Some(level) = options.level {
            let matched = match entity {
                CommandDamageEntityRef::Player(player) => level.matches(player.experience.level()),
                CommandDamageEntityRef::Stored(_) => false,
            };
            if !matched {
                return Ok(false);
            }
        }
        if check_aabb
            && self
                .aabb
                .is_some_and(|aabb| !aabb.intersects(bounding_box(entity)))
        {
            return Ok(false);
        }
        Ok(options.distance.is_none_or(|range| {
            range.matches_sqr(distance_squared(entity.position(), self.position))
        }))
    }

    fn predicate_holds(
        &self,
        predicate: &SelectorPredicate,
        entity: CommandDamageEntityRef<'a>,
    ) -> Result<bool, Refusal> {
        let world = self.world;
        Ok(match predicate {
            // `e.getPlainTextName().equals(name) != not` (EntitySelectorOptions.java:105).
            SelectorPredicate::Name { name, inverted } => {
                names::plain_text_name(entity, world)?
                    .into_iter()
                    .eq(name.units().iter().copied())
                    != *inverted
            }
            // `:236-243`.
            SelectorPredicate::GameMode {
                game_type,
                inverted,
            } => match entity {
                CommandDamageEntityRef::Player(player) => {
                    (player.game_type == *game_type) ^ inverted
                }
                CommandDamageEntityRef::Stored(_) => false,
            },
            // `:253-257`.
            SelectorPredicate::Team { name, inverted } => {
                (team_name(entity, world).map_or(name.is_empty(), |team| team == name)) != *inverted
            }
            // `:299`.
            SelectorPredicate::Type { id, inverted } => {
                (entity.type_resource_id() == id) != *inverted
            }
            // `e.is(id) != inverted` (`:288`).
            SelectorPredicate::TypeTag { tag, inverted } => {
                type_in_tag(entity, tag, world) != *inverted
            }
            // `:308`.
            SelectorPredicate::Tag { name, inverted } => {
                let tags = entity.entity_tags();
                let held = if name.is_empty() {
                    tags.is_empty()
                } else {
                    tags.iter().any(|entry| entry == name)
                };
                held != *inverted
            }
            // `:313-326`, whose compared tag is `/data get entity`'s own
            // (`NbtPredicate.getEntityTagToCompare`, NbtPredicate.java:42-54).
            SelectorPredicate::Nbt { tag, inverted } => {
                let Some(data) =
                    super::entity::command_entity_data(entity, world).map_err(unanswerable)?
                else {
                    return Err(unanswerable("No saved data is projected for this entity"));
                };
                block_entity_data_matches(Some(tag), Some(&data)) != *inverted
            }
            // EntitySelectorOptions.java:350-370.
            SelectorPredicate::Scores(expected) => entity.with_scoreboard_name(|holder| {
                let board = world.scoreboard;
                expected.iter().all(|(objective, bounds)| {
                    board.objective(objective).is_some()
                        && board
                            .score(holder, objective)
                            .is_some_and(|score| bounds.matches(score.value))
                })
            }),
            // EntitySelectorOptions.java:433-449.
            SelectorPredicate::Advancements(requirements) => match entity {
                CommandDamageEntityRef::Player(player) => {
                    crate::commands::selector_advancements::player_matches(
                        requirements,
                        world.advancements,
                        player.advancements,
                    )
                }
                CommandDamageEntityRef::Stored(_) => false,
            },
            SelectorPredicate::Predicate { id, inverted } => {
                loot::predicate_holds(id, *inverted, entity, world)?
            }
        })
    }

    /// `sortAndLimit` (EntitySelector.java:279-285) around the selector's own position.
    fn sort_and_limit<T: Copy>(
        &self,
        mut found: Vec<T>,
        position: impl Fn(T) -> [f64; 3],
    ) -> Vec<T> {
        let origin = self.position;
        let max_results = usize::try_from(self.selector.max_results()).unwrap_or(usize::MAX);
        if found.len() > 1 {
            match self.selector.order() {
                SelectorOrder::Arbitrary => {}
                SelectorOrder::Nearest if max_results == 1 => {
                    if let Some(best) = found.into_iter().min_by(|left, right| {
                        distance_squared(position(*left), origin)
                            .total_cmp(&distance_squared(position(*right), origin))
                    }) {
                        return vec![best];
                    }
                    return Vec::new();
                }
                SelectorOrder::Furthest if max_results == 1 => {
                    if let Some(best) = found.into_iter().min_by(|left, right| {
                        distance_squared(position(*right), origin)
                            .total_cmp(&distance_squared(position(*left), origin))
                    }) {
                        return vec![best];
                    }
                    return Vec::new();
                }
                SelectorOrder::Nearest => found.sort_by(|left, right| {
                    distance_squared(position(*left), origin)
                        .total_cmp(&distance_squared(position(*right), origin))
                }),
                SelectorOrder::Furthest => found.sort_by(|left, right| {
                    distance_squared(position(*right), origin)
                        .total_cmp(&distance_squared(position(*left), origin))
                }),
                SelectorOrder::Random => shuffle(&mut found, self.world.random(self.level)),
            }
        }
        found.truncate(max_results);
        found
    }
}

/// The volume `getSelector` builds before it is moved onto the position
/// (EntitySelectorParser.java:123-133, `createAabb` `:159-170`).
fn relative_aabb(options: &crate::commands::selector_options::SelectorOptions) -> Option<Aabb> {
    if options.delta_x.is_none() && options.delta_y.is_none() && options.delta_z.is_none() {
        let max = options.distance?.max?;
        return Some(Aabb {
            min_x: -max,
            min_y: -max,
            min_z: -max,
            max_x: max + 1.0,
            max_y: max + 1.0,
            max_z: max + 1.0,
        });
    }
    let side = |delta: Option<f64>| {
        let delta = delta.unwrap_or(0.0);
        if delta < 0.0 {
            (delta, 1.0)
        } else {
            (0.0, delta + 1.0)
        }
    };
    let (min_x, max_x) = side(options.delta_x);
    let (min_y, max_y) = side(options.delta_y);
    let (min_z, max_z) = side(options.delta_z);
    Some(Aabb {
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    })
}

/// `EntitySectionStorage.forEachAccessibleNonEmptySection`'s bounds
/// (EntitySectionStorage.java:37-43): two blocks outward horizontally, four
/// down and none up.
fn in_walked_section(aabb: Aabb, section_pos: SectionKey) -> bool {
    let section = |value: f64| (value.floor() as i32) >> 4;
    (section(aabb.min_x - 2.0)..=section(aabb.max_x + 2.0)).contains(&section_pos.x())
        && (section(aabb.min_y - 4.0)..=section(aabb.max_y)).contains(&section_pos.y())
        && (section(aabb.min_z - 2.0)..=section(aabb.max_z + 2.0)).contains(&section_pos.z())
}

/// `createRotationPredicate` (EntitySelectorParser.java:186-193).
fn rotation_matches(range: SelectorBounds<f32>, rotation: f32) -> bool {
    let min = wrap_degrees(range.min.unwrap_or(0.0));
    let max = wrap_degrees(range.max.unwrap_or(359.0));
    let rotation = wrap_degrees(rotation);
    if min > max {
        rotation >= min || rotation <= max
    } else {
        rotation >= min && rotation <= max
    }
}

/// `Mth.wrapDegrees(float)` (Mth.java:215-226).
fn wrap_degrees(angle: f32) -> f32 {
    let mut normalized = angle % 360.0;
    if normalized >= 180.0 {
        normalized -= 360.0;
    }
    if normalized < -180.0 {
        normalized += 360.0;
    }
    normalized
}

/// `Entity.isAlive` (Entity.java:2146-2148) and its `LivingEntity` override
/// (LivingEntity.java:1718-1720).
fn is_alive(entity: CommandDamageEntityRef<'_>) -> bool {
    match entity {
        CommandDamageEntityRef::Player(player) => player.damage.health > 0.0,
        CommandDamageEntityRef::Stored(stored) => {
            stored.entity.is_alive()
                && stored
                    .entity
                    .living()
                    .is_none_or(|living| living.health() > 0.0)
        }
    }
}

/// `Entity.getBoundingBox`: a player's is its pose's dimensions at its scale
/// (`LivingEntity.getDimensions`).
fn bounding_box(entity: CommandDamageEntityRef<'_>) -> Aabb {
    match entity {
        CommandDamageEntityRef::Player(player) => {
            let scale = player
                .attributes
                .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
                .unwrap_or(1.0) as f32;
            player_pose_bounding_box(
                Vec3d {
                    x: player.position[0],
                    y: player.position[1],
                    z: player.position[2],
                },
                player.pose_id,
                scale,
            )
            .expect("every player pose has catalogued dimensions")
        }
        CommandDamageEntityRef::Stored(stored) => stored.entity.bounding_box(),
    }
}

/// `SectionPos.asLong(entity.blockPosition())`, the section an entity is filed
/// under (PersistentEntitySectionManager.java:79).
fn block_section(entity: CommandDamageEntityRef<'_>) -> SectionKey {
    let [x, y, z] = entity.position().map(|value| (value.floor() as i32) >> 4);
    SectionKey::new(x, y, z)
}

pub(super) fn team_name<'a>(
    entity: CommandDamageEntityRef<'_>,
    world: &EntityReads<'a>,
) -> Option<&'a bbb_protocol::JavaString> {
    world
        .scoreboard
        .entity_team_name(entity, |uuid| world.living_command_entity_by_uuid(uuid))
}

/// `Entity.is(TagKey)` over the entity type's built-in holder.
fn type_in_tag(entity: CommandDamageEntityRef<'_>, tag: &str, world: &EntityReads<'_>) -> bool {
    const REGISTRY: &str = "minecraft:entity_type";
    match entity {
        CommandDamageEntityRef::Stored(stored) => {
            stored.tag_contains(REGISTRY, tag, entity.type_resource_id())
        }
        CommandDamageEntityRef::Player(_) => {
            let player =
                bbb_protocol::entity_types::vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID)
                    .expect("the player entity type is in the generated catalog")
                    .resource_id;
            match world.entity_tags {
                Some(tags) => tags.contains(REGISTRY, tag, player),
                None => EntityRegistryTagView::Builtin26_1.contains(REGISTRY, tag, player),
            }
        }
    }
}

fn distance_squared(left: [f64; 3], right: [f64; 3]) -> f64 {
    (0..3)
        .map(|axis| (left[axis] - right[axis]) * (left[axis] - right[axis]))
        .sum()
}

/// `EntitySelectorParser.ORDER_RANDOM` (EntitySelectorParser.java:70).
fn shuffle<T>(candidates: &mut [T], random: &GameRandom) {
    for index in (1..candidates.len()).rev() {
        let bound = i32::try_from(index + 1).unwrap_or(i32::MAX);
        let drawn = usize::try_from(random.next_int(bound)).unwrap_or(0);
        candidates.swap(index, drawn);
    }
}

/// `Commands.performCommand`'s catch-all (Commands.java:333-349), the refusal a
/// predicate throwing inside `findEntities` reaches the player as.
fn unanswerable(detail: impl AsRef<str>) -> Refusal {
    styled_translatable(
        "command.failed",
        Vec::new(),
        &ComponentStyle {
            hover_event: Some(scoreboard::show_text(detail.as_ref())),
            ..ComponentStyle::default()
        },
    )
}
