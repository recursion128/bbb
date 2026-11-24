//! Stable dimension IDs and resource-name boundary lookup.

use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

use super::{ConnectionId, PlayerState, ServerLevel, ServerWorld};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct LevelId(u32);

impl From<Java26BuiltinDimension> for LevelId {
    fn from(dimension: Java26BuiltinDimension) -> Self {
        Self(match dimension {
            Java26BuiltinDimension::Overworld => 0,
            Java26BuiltinDimension::Nether => 1,
            Java26BuiltinDimension::End => 2,
        })
    }
}

impl From<&LevelId> for LevelId {
    fn from(dimension: &LevelId) -> Self {
        *dimension
    }
}

impl LevelId {
    pub(crate) fn builtin(self) -> Option<Java26BuiltinDimension> {
        match self.0 {
            0 => Some(Java26BuiltinDimension::Overworld),
            1 => Some(Java26BuiltinDimension::Nether),
            2 => Some(Java26BuiltinDimension::End),
            _ => None,
        }
    }
}

pub(super) struct LevelArena {
    entries: Vec<Option<ServerLevel>>,
    pub(super) names: std::sync::Arc<LevelNames>,
    /// Where each online player's slot is: its level and body entity id.
    players: rustc_hash::FxHashMap<ConnectionId, (LevelId, i32)>,
    player_list_revision: u64,
}

#[derive(Clone, Copy)]
pub(super) struct OtherLevels<'a> {
    before: &'a [Option<ServerLevel>],
    after: &'a [Option<ServerLevel>],
    names: &'a LevelNames,
}

impl<'a> OtherLevels<'a> {
    pub(super) fn names(self) -> &'a LevelNames {
        self.names
    }

    pub(super) fn iter(self) -> impl Iterator<Item = (LevelId, &'a ServerLevel)> {
        let after_start = self.before.len() + 1;
        self.before
            .iter()
            .enumerate()
            .chain(
                self.after
                    .iter()
                    .enumerate()
                    .map(move |(index, level)| (after_start + index, level)),
            )
            .filter_map(|(index, level)| level.as_ref().map(|level| (LevelId(index as u32), level)))
    }
}

pub(super) struct LevelNames {
    by_key: std::collections::BTreeMap<std::sync::Arc<Java26LevelKey>, LevelId>,
    keys: Vec<Option<std::sync::Arc<Java26LevelKey>>>,
}

impl LevelNames {
    pub(super) fn id(&self, key: &Java26LevelKey) -> Option<LevelId> {
        self.by_key.get(key).copied()
    }

    pub(super) fn key(&self, id: LevelId) -> &Java26LevelKey {
        self.keys[id.0 as usize]
            .as_ref()
            .expect("served level name")
    }
}

impl LevelArena {
    pub(super) fn new(
        levels: std::collections::BTreeMap<Java26LevelKey, ServerLevel>,
    ) -> bbb_server_types::Result<Self> {
        // Reserved built-in slots make their IDs independent of which stems are served.
        let mut entries = vec![None, None, None];
        let mut by_key = std::collections::BTreeMap::new();
        let mut keys = vec![None, None, None];
        for (key, mut level) in levels {
            let key = std::sync::Arc::new(key);
            let id = match key.builtin() {
                Some(builtin) => builtin.into(),
                None => LevelId(u32::try_from(entries.len()).map_err(|_| {
                    bbb_server_types::ServerWorldError::InvalidConfiguration(
                        "dimension count exceeds LevelId capacity".to_string(),
                    )
                })?),
            };
            level.dimension = std::sync::Arc::clone(&key);
            if id.0 as usize == entries.len() {
                entries.push(Some(level));
            } else {
                entries[id.0 as usize] = Some(level);
            }
            if id.0 as usize == keys.len() {
                keys.push(Some(key.clone()));
            } else {
                keys[id.0 as usize] = Some(key.clone());
            }
            by_key.insert(key, id);
        }
        Ok(Self {
            entries,
            names: std::sync::Arc::new(LevelNames { by_key, keys }),
            players: rustc_hash::FxHashMap::default(),
            player_list_revision: 0,
        })
    }

    pub(super) fn insert_player(&mut self, player: PlayerState) {
        let key = (player.dimension, player.entity_id());
        let (connection_id, uuid) = (player.connection_id, player.profile.uuid);
        self.get_mut(&key.0)
            .expect("a player joins a served level")
            .entities
            .insert_player(key.1, uuid, Box::new(player));
        self.players.insert(connection_id, key);
        self.player_list_revision = self.player_list_revision.wrapping_add(1);
    }

    pub(super) fn remove_player(&mut self, connection_id: ConnectionId) -> Option<PlayerState> {
        let (level, id) = self.players.remove(&connection_id)?;
        let player = self.get_mut(&level)?.entities.remove_player(id)?;
        self.player_list_revision = self.player_list_revision.wrapping_add(1);
        Some(*player)
    }

    pub(super) fn bump_player_list_revision(&mut self) {
        self.player_list_revision = self.player_list_revision.wrapping_add(1);
    }

    pub(super) fn player_list_revision(&self) -> u64 {
        self.player_list_revision
    }

    /// Refiles a player's slot into `level`'s store.
    pub(super) fn move_player(&mut self, connection_id: ConnectionId, level: LevelId) {
        let Some(&(from, id)) = self.players.get(&connection_id) else {
            return;
        };
        if from == level {
            return;
        }
        let mut player = self
            .get_mut(&from)
            .and_then(|from| from.entities.remove_player(id))
            .expect("an indexed player has a slot");
        player.dimension = level;
        let uuid = player.profile.uuid;
        self.get_mut(&level)
            .expect("a player moves to a served level")
            .entities
            .insert_player(id, uuid, player);
        self.players.insert(connection_id, (level, id));
    }

    pub(super) fn player(&self, connection_id: ConnectionId) -> Option<&PlayerState> {
        let (level, id) = self.players.get(&connection_id)?;
        self.get(level)?.entities.player(*id)
    }

    pub(super) fn player_mut(&mut self, connection_id: ConnectionId) -> Option<&mut PlayerState> {
        let (level, id) = *self.players.get(&connection_id)?;
        self.get_mut(&level)?.entities.player_mut(id)
    }

    /// The player behind `connection_id`, which the caller knows is online.
    #[track_caller]
    pub(super) fn player_at(&self, connection_id: ConnectionId) -> &PlayerState {
        self.player(connection_id).expect("the session is online")
    }

    pub(super) fn contains_player(&self, connection_id: ConnectionId) -> bool {
        self.players.contains_key(&connection_id)
    }

    pub(super) fn player_count(&self) -> usize {
        self.players.len()
    }

    pub(super) fn player_keys(&self) -> impl Iterator<Item = &ConnectionId> + Clone {
        self.players.keys()
    }

    pub(super) fn player_entries(
        &self,
    ) -> impl Iterator<Item = (&ConnectionId, &PlayerState)> + Clone {
        self.players.keys().map(|connection_id| {
            (
                connection_id,
                self.player(*connection_id)
                    .expect("an indexed player has a slot"),
            )
        })
    }

    pub(super) fn players(&self) -> impl Iterator<Item = &PlayerState> + Clone {
        self.player_entries().map(|(_, player)| player)
    }

    pub(super) fn players_mut(&mut self) -> impl Iterator<Item = &mut PlayerState> {
        self.entries
            .iter_mut()
            .filter_map(Option::as_mut)
            .flat_map(|level| level.entities.players_mut())
    }

    pub(super) fn get(&self, key: &LevelId) -> Option<&ServerLevel> {
        self.entries.get(key.0 as usize).and_then(Option::as_ref)
    }

    pub(super) fn get_mut(&mut self, key: &LevelId) -> Option<&mut ServerLevel> {
        self.entries
            .get_mut(key.0 as usize)
            .and_then(Option::as_mut)
    }

    pub(super) fn split_mut(
        &mut self,
        key: &LevelId,
    ) -> Option<(&mut ServerLevel, OtherLevels<'_>)> {
        let index = key.0 as usize;
        if index >= self.entries.len() {
            return None;
        }
        let (before, rest) = self.entries.split_at_mut(index);
        let (current, after) = rest.split_first_mut()?;
        Some((
            current.as_mut()?,
            OtherLevels {
                before,
                after,
                names: &self.names,
            },
        ))
    }

    pub(super) fn contains_key(&self, key: &LevelId) -> bool {
        self.get(key).is_some()
    }
    pub(super) fn len(&self) -> usize {
        self.names.by_key.len()
    }
    pub(super) fn values(&self) -> impl Iterator<Item = &ServerLevel> {
        self.entries.iter().filter_map(Option::as_ref)
    }
    pub(super) fn values_mut(&mut self) -> impl Iterator<Item = &mut ServerLevel> {
        self.entries.iter_mut().filter_map(Option::as_mut)
    }
    pub(super) fn keys(&self) -> impl Iterator<Item = LevelId> + '_ {
        self.iter().map(|(id, _)| id)
    }
    pub(super) fn next_key(&self, after: Option<LevelId>) -> Option<LevelId> {
        let start = after.map_or(0, |id| id.0 as usize + 1);
        self.entries[start..]
            .iter()
            .position(Option::is_some)
            .map(|offset| LevelId((start + offset) as u32))
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = (LevelId, &ServerLevel)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, level)| level.as_ref().map(|level| (LevelId(index as u32), level)))
    }
    pub(super) fn iter_mut_with_names(
        &mut self,
    ) -> (
        &LevelNames,
        impl Iterator<Item = (LevelId, &mut ServerLevel)>,
    ) {
        (
            &self.names,
            self.entries
                .iter_mut()
                .enumerate()
                .filter_map(|(index, level)| {
                    level.as_mut().map(|level| (LevelId(index as u32), level))
                }),
        )
    }

    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = (LevelId, &mut ServerLevel)> {
        self.entries
            .iter_mut()
            .enumerate()
            .filter_map(|(index, level)| level.as_mut().map(|level| (LevelId(index as u32), level)))
    }
    pub(super) fn pair_mut(
        &mut self,
        source: LevelId,
        target: LevelId,
    ) -> (&mut ServerLevel, &mut ServerLevel) {
        let source = source.0 as usize;
        let target = target.0 as usize;
        assert_ne!(source, target);
        if source < target {
            let (first, second) = self.entries.split_at_mut(target);
            (
                first[source].as_mut().expect("served source"),
                second[0].as_mut().expect("served target"),
            )
        } else {
            let (first, second) = self.entries.split_at_mut(source);
            (
                second[0].as_mut().expect("served source"),
                first[target].as_mut().expect("served target"),
            )
        }
    }
}

/// The player behind a connection the caller knows is online.
impl std::ops::Index<ConnectionId> for LevelArena {
    type Output = PlayerState;
    #[track_caller]
    fn index(&self, connection_id: ConnectionId) -> &Self::Output {
        self.player(connection_id).expect("the session is online")
    }
}

impl std::ops::IndexMut<ConnectionId> for LevelArena {
    #[track_caller]
    fn index_mut(&mut self, connection_id: ConnectionId) -> &mut Self::Output {
        self.player_mut(connection_id)
            .expect("the session is online")
    }
}

impl std::ops::Index<&LevelId> for LevelArena {
    type Output = ServerLevel;
    fn index(&self, key: &LevelId) -> &Self::Output {
        self.get(key).expect("served level")
    }
}

impl ServerWorld {
    pub(crate) fn require_level_id(&self, key: &Java26LevelKey) -> LevelId {
        self.level_id(key).expect("served level")
    }
    pub(crate) fn level_id(&self, key: &Java26LevelKey) -> Option<LevelId> {
        self.levels.names.id(key)
    }
    pub(crate) fn level_key(&self, id: LevelId) -> &Java26LevelKey {
        self.levels.names.key(id)
    }

    pub(crate) fn player_level_id(&self, connection_id: ConnectionId) -> LevelId {
        self.levels[connection_id].dimension
    }

    #[cfg(test)]
    pub(crate) fn player_dimension(&self, connection_id: ConnectionId) -> Java26BuiltinDimension {
        self.player_level_id(connection_id)
            .builtin()
            .expect("this caller requires a built-in dimension")
    }

    pub(super) fn level(&self, dimension: Java26BuiltinDimension) -> &ServerLevel {
        self.level_if_present(dimension)
            .expect("every reachable dimension is a level this world runs")
    }

    #[cfg(test)]
    pub(crate) fn level_entities(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> &crate::entity::EntityStore {
        &self.level(dimension).entities
    }

    pub(super) fn level_by_id(&self, dimension: &LevelId) -> &ServerLevel {
        self.levels
            .get(dimension)
            .expect("every reachable dimension is a level this world runs")
    }

    pub(super) fn level_if_present_by_id(&self, dimension: &LevelId) -> Option<&ServerLevel> {
        self.levels.get(dimension)
    }

    pub(super) fn has_level_key(&self, key: &Java26LevelKey) -> bool {
        self.level_id(key).is_some()
    }
    pub(super) fn has_level_id(&self, dimension: &LevelId) -> bool {
        self.level_if_present_by_id(dimension).is_some()
    }

    pub(super) fn level_if_present(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Option<&ServerLevel> {
        self.levels.get(&LevelId::from(dimension))
    }

    pub(super) fn has_level(&self, dimension: Java26BuiltinDimension) -> bool {
        self.level_if_present(dimension).is_some()
    }

    #[cfg(test)]
    pub(super) fn level_mut(&mut self, dimension: Java26BuiltinDimension) -> &mut ServerLevel {
        self.level_mut_if_present(dimension)
            .expect("every reachable dimension is a level this world runs")
    }

    pub(super) fn level_mut_by_id(&mut self, dimension: &LevelId) -> &mut ServerLevel {
        self.levels
            .get_mut(dimension)
            .expect("every reachable dimension is a level this world runs")
    }

    pub(super) fn level_mut_if_present_by_id(
        &mut self,
        dimension: &LevelId,
    ) -> Option<&mut ServerLevel> {
        self.levels.get_mut(dimension)
    }

    #[cfg(test)]
    pub(super) fn level_mut_if_present(
        &mut self,
        dimension: Java26BuiltinDimension,
    ) -> Option<&mut ServerLevel> {
        self.levels.get_mut(&LevelId::from(dimension))
    }

    /// The stems this server could not serve.
    #[must_use]
    pub fn unserved_levels(&self) -> Vec<(&str, &str)> {
        self.cold
            .unserved_levels
            .iter()
            .map(|(dimension, generator)| (dimension.resource_id(), generator.as_str()))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn served_builtin_dimensions(&self) -> Vec<Java26BuiltinDimension> {
        Java26BuiltinDimension::ALL
            .into_iter()
            .filter(|dimension| self.has_level(*dimension))
            .collect()
    }

    pub(crate) fn level_ids(&self) -> Vec<LevelId> {
        self.levels.keys().collect()
    }

    // Entity stores are borrowed by the frame; only player facts need a projection.
    pub(in crate::world) fn refresh_cross_level_entities_by_id(&mut self, dimension: &LevelId) {
        let mut projection =
            std::mem::take(&mut self.level_mut_by_id(dimension).cross_level_entities);
        projection.clear();
        for player in self.levels.players() {
            if player.dimension != *dimension && !player.avatar_removed {
                projection.insert_session_input(Self::nearby_player_facts(player));
            }
        }
        self.level_mut_by_id(dimension).cross_level_entities = projection;
    }

    // A move changes remote pose/facts for every level, not just source/target presence.
    pub(in crate::world) fn refresh_cross_level_entities_across(
        &mut self,
        source: &LevelId,
        target: &LevelId,
    ) {
        if !self.levels.contains_key(source) && !self.levels.contains_key(target) {
            return;
        }
        for key in self.level_ids() {
            self.refresh_cross_level_entities_by_id(&key);
        }
    }
}
