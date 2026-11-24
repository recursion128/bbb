use crate::commands::types::{CommandDamageEntityRef, CommandStoredEntity};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{EntityData, EntityStore};
use crate::loot_runtime::{LootEntity, LootRuntimeReason, StoredLootEntity};
use crate::world::level_keys::{LevelArena, LevelNames, OtherLevels};
use crate::world::scoreboard::Scoreboard;
use crate::world::LevelId;

pub(in crate::world) struct LootEntityScoreboard<'a> {
    pub scoreboard: &'a Scoreboard,
    pub names: &'a LevelNames,
    pub dimension: LevelId,
    pub bodies: Option<EntityBodyReadView<'a>>,
    pub other_levels: LootEntityLevels<'a>,
}

#[derive(Clone, Copy)]
pub(in crate::world) enum LootEntityLevels<'a> {
    Stores(&'a [(LevelId, &'a EntityStore)]),
    World(&'a LevelArena),
    Other(OtherLevels<'a>),
    Reads(&'a [crate::world::command_reads::LevelRead<'a>]),
}

impl<'a> LootEntityLevels<'a> {
    pub(in crate::world) fn stores(
        self,
        names: &'a LevelNames,
    ) -> impl Iterator<Item = (LevelId, &'a EntityStore)> + 'a {
        let (stores, world, other, reads) = match self {
            Self::Stores(v) => (Some(v), None, None, None),
            Self::World(v) => (None, Some(v), None, None),
            Self::Other(v) => (None, None, Some(v), None),
            Self::Reads(v) => (None, None, None, Some(v)),
        };
        stores
            .into_iter()
            .flatten()
            .copied()
            .chain(
                world
                    .into_iter()
                    .flat_map(|levels| levels.iter().map(|(id, level)| (id, &level.entities))),
            )
            .chain(
                other
                    .into_iter()
                    .flat_map(|levels| levels.iter().map(|(id, level)| (id, &level.entities))),
            )
            .chain(
                reads
                    .into_iter()
                    .flatten()
                    .filter_map(move |level| names.id(level.key).map(|id| (id, level.entities))),
            )
    }
}

impl<'a> LootEntityScoreboard<'a> {
    /// The online player `uuid` and the store it stands in; true when that is
    /// this level's.
    pub(in crate::world) fn online_player(
        &self,
        uuid: uuid::Uuid,
    ) -> Option<(&'a crate::world::PlayerState, &'a EntityStore, bool)> {
        if let Some(store) = self.bodies.map(EntityBodyReadView::store) {
            if let Some(player) = store.player_by_uuid(uuid) {
                return Some((player, store, true));
            }
        }
        self.other_levels
            .stores(self.names)
            .find_map(|(_, store)| Some((store.player_by_uuid(uuid)?, store, false)))
    }
}

impl LootEntityScoreboard<'_> {
    pub(in crate::world) fn matches(
        &self,
        entity: LootEntity<'_>,
        expected: &bbb_protocol::JavaString,
        bodies: Option<EntityBodyReadView<'_>>,
    ) -> Result<bool, LootRuntimeReason> {
        let sources = self.with_bodies(entity, bodies);
        let resolve_living = |uuid| {
            sources
                .entity(uuid, None)
                .map(|(entity, _)| entity)
                .filter(|entity| match entity {
                    CommandDamageEntityRef::Player(_) => true,
                    CommandDamageEntityRef::Stored(stored) => stored.entity.living().is_some(),
                })
        };
        let team = if let Some((uuid, tame_owner)) = entity.stored_scoreboard_identity() {
            self.scoreboard
                .stored_team_name(uuid, tame_owner, resolve_living)
        } else {
            self.scoreboard
                .entity_team_name(sources.resolve_player(entity)?, resolve_living)
        };
        Ok(team == Some(expected))
    }

    pub(in crate::world) fn score(
        &self,
        name: &(impl crate::world::scoreboard::ScoreboardName + ?Sized),
        objective: &bbb_protocol::JavaString,
    ) -> Option<i32> {
        self.scoreboard.objective(objective)?;
        self.scoreboard
            .score(name, objective)
            .map(|score| score.value)
    }

    pub(in crate::world) fn entity_score(
        &self,
        entity: LootEntity<'_>,
        objective: &bbb_protocol::JavaString,
        bodies: Option<EntityBodyReadView<'_>>,
    ) -> Result<Option<i32>, LootRuntimeReason> {
        if let Some((uuid, _)) = entity.stored_scoreboard_identity() {
            let mut buffer = uuid::Uuid::encode_buffer();
            return Ok(self.score(uuid.hyphenated().encode_lower(&mut buffer), objective));
        }
        let sources = self.with_bodies(entity, bodies);
        let entity = sources.resolve_player(entity)?;
        Ok(entity.with_scoreboard_name(|name| sources.score(name, objective)))
    }

    pub(in crate::world) fn entity_for_relations<'s>(
        &'s self,
        entity: LootEntity<'_>,
        bodies: Option<EntityBodyReadView<'s>>,
    ) -> Option<StoredLootEntity<'s>> {
        let LootEntity::Facts(facts) = entity else {
            return None;
        };
        let uuid = facts.player_uuid.or(facts.entity_uuid)?;
        let (entity, bodies) = self.entity(uuid, bodies)?;
        let entity = match entity {
            CommandDamageEntityRef::Player(_) => bodies.store().player_by_uuid(uuid)?.body(),
            CommandDamageEntityRef::Stored(stored) => stored.entity,
        };
        if entity.uuid() != uuid
            || entity.data().entity_resource_id() != facts.entity_type?
            || facts.player_uuid.is_some() != matches!(entity.data(), EntityData::Player)
        {
            return None;
        }
        Some(StoredLootEntity {
            entity,
            bodies: Some(bodies),
            raid_at_position: None,
        })
    }

    fn with_bodies<'s>(
        &'s self,
        entity: LootEntity<'s>,
        bodies: Option<EntityBodyReadView<'s>>,
    ) -> LootEntityScoreboard<'s> {
        LootEntityScoreboard {
            bodies: match entity {
                LootEntity::Stored(stored) => stored.bodies.or(bodies).or(self.bodies),
                _ => bodies.or(self.bodies),
            },
            scoreboard: self.scoreboard,
            names: self.names,
            dimension: self.dimension,
            other_levels: self.other_levels,
        }
    }

    fn resolve_player<'s>(
        &'s self,
        entity: LootEntity<'s>,
    ) -> Result<CommandDamageEntityRef<'s>, LootRuntimeReason> {
        let missing =
            || LootRuntimeReason::Catalog("entity requires an actual scoreboard identity".into());
        self.entity(entity.player_uuid().ok_or_else(missing)?, None)
            .map(|(entity, _)| entity)
            .ok_or_else(missing)
    }

    fn entity<'s>(
        &'s self,
        uuid: uuid::Uuid,
        bodies: Option<EntityBodyReadView<'s>>,
    ) -> Option<(CommandDamageEntityRef<'s>, EntityBodyReadView<'s>)> {
        for bodies in bodies.into_iter().chain(self.bodies) {
            if let Some(entity) = crate::world::command_reads::command_entity_for_uuid_in_store(
                bodies.store(),
                uuid,
                self.names,
                self.names.key(self.dimension),
                None,
            ) {
                return Some((entity, bodies));
            }
            if let Some(entity) = bodies
                .get_by_uuid(uuid)
                .filter(|entity| !matches!(entity.data(), EntityData::Player))
            {
                return Some((
                    CommandDamageEntityRef::Stored(CommandStoredEntity {
                        entity,
                        dimension: self.names.key(self.dimension),
                        registry_tags: None,
                    }),
                    bodies,
                ));
            }
        }
        let in_store = |dimension, store| {
            let entity = crate::world::command_reads::command_entity_for_uuid_in_store(
                store,
                uuid,
                self.names,
                self.names.key(dimension),
                None,
            )?;
            Some((entity, EntityBodyReadView::new(store, None, None)))
        };
        match self.other_levels {
            LootEntityLevels::Stores(levels) => levels
                .iter()
                .find_map(|(dimension, store)| in_store(*dimension, *store)),
            LootEntityLevels::World(levels) => levels
                .iter()
                .find_map(|(dimension, level)| in_store(dimension, &level.entities)),
            LootEntityLevels::Other(levels) => levels
                .iter()
                .find_map(|(dimension, level)| in_store(dimension, &level.entities)),
            LootEntityLevels::Reads(levels) => levels.iter().find_map(|level| {
                let entity = crate::world::command_reads::command_entity_for_uuid_in_store(
                    level.entities,
                    uuid,
                    self.names,
                    level.key,
                    level.tags,
                )?;
                Some((entity, EntityBodyReadView::new(level.entities, None, None)))
            }),
        }
    }
}
