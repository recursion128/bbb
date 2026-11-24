#[derive(Clone, Debug)]
pub struct SmallIds<const N: usize = 32> {
    inline: [i32; N],
    /// Inline element count, or `usize::MAX` after spilling to `heap`.
    len: usize,
    heap: Vec<i32>,
}

impl<const N: usize> Default for SmallIds<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> SmallIds<N> {
    pub const fn new() -> Self {
        Self {
            inline: [0; N],
            len: 0,
            heap: Vec::new(),
        }
    }

    pub fn push(&mut self, value: i32) {
        if self.len < N {
            self.inline[self.len] = value;
            self.len += 1;
        } else if self.len == N {
            self.heap.reserve(N.saturating_mul(2).max(1));
            self.heap.extend_from_slice(&self.inline);
            self.heap.push(value);
            self.len = usize::MAX;
        } else {
            self.heap.push(value);
        }
    }

    pub fn extend<I: IntoIterator<Item = i32>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&i32) -> bool) {
        if self.len == usize::MAX {
            self.heap.retain(keep);
        } else {
            let mut write = 0;
            for read in 0..self.len {
                let value = self.inline[read];
                if keep(&value) {
                    self.inline[write] = value;
                    write += 1;
                }
            }
            self.len = write;
        }
    }

    pub fn as_slice(&self) -> &[i32] {
        if self.len == usize::MAX {
            self.heap.as_slice()
        } else {
            &self.inline[..self.len]
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [i32] {
        if self.len == usize::MAX {
            self.heap.as_mut_slice()
        } else {
            &mut self.inline[..self.len]
        }
    }

    pub fn into_vec(self) -> Vec<i32> {
        if self.len == usize::MAX {
            self.heap
        } else {
            self.inline[..self.len].to_vec()
        }
    }
}

impl<const N: usize> std::ops::Deref for SmallIds<N> {
    type Target = [i32];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const N: usize> std::ops::DerefMut for SmallIds<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<const N: usize, const M: usize> PartialEq<SmallIds<M>> for SmallIds<N> {
    fn eq(&self, other: &SmallIds<M>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> Eq for SmallIds<N> {}

impl<const N: usize> PartialEq<Vec<i32>> for SmallIds<N> {
    fn eq(&self, other: &Vec<i32>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> PartialEq<SmallIds<N>> for Vec<i32> {
    fn eq(&self, other: &SmallIds<N>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> PartialEq<[i32]> for SmallIds<N> {
    fn eq(&self, other: &[i32]) -> bool {
        self.as_slice() == other
    }
}

impl<const N: usize> PartialEq<&[i32]> for SmallIds<N> {
    fn eq(&self, other: &&[i32]) -> bool {
        self.as_slice() == *other
    }
}

impl<const N: usize, const M: usize> PartialEq<[i32; M]> for SmallIds<N> {
    fn eq(&self, other: &[i32; M]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> FromIterator<i32> for SmallIds<N> {
    fn from_iter<I: IntoIterator<Item = i32>>(iter: I) -> Self {
        let mut out = Self::new();
        out.extend(iter);
        out
    }
}

pub struct SmallIdsIntoIter<const N: usize = 32> {
    inline: [i32; N],
    index: usize,
    len: usize,
    heap: std::vec::IntoIter<i32>,
}

impl<const N: usize> Iterator for SmallIdsIntoIter<N> {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == usize::MAX {
            self.heap.next()
        } else if self.index < self.len {
            let value = self.inline[self.index];
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.len == usize::MAX {
            self.heap.size_hint()
        } else {
            let rem = self.len.saturating_sub(self.index);
            (rem, Some(rem))
        }
    }
}

impl<const N: usize> ExactSizeIterator for SmallIdsIntoIter<N> {}

impl<const N: usize> IntoIterator for SmallIds<N> {
    type Item = i32;
    type IntoIter = SmallIdsIntoIter<N>;

    fn into_iter(self) -> Self::IntoIter {
        SmallIdsIntoIter {
            inline: self.inline,
            index: 0,
            len: self.len,
            heap: self.heap.into_iter(),
        }
    }
}

impl<'a, const N: usize> IntoIterator for &'a SmallIds<N> {
    type Item = &'a i32;
    type IntoIter = std::slice::Iter<'a, i32>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a, const N: usize> IntoIterator for &'a mut SmallIds<N> {
    type Item = &'a mut i32;
    type IntoIter = std::slice::IterMut<'a, i32>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct SectionEntities(Vec<i32>);

impl SectionEntities {
    pub(super) fn insert(&mut self, id: i32) {
        if !self.0.contains(&id) {
            self.0.push(id);
        }
    }

    pub(super) fn remove(&mut self, id: &i32) {
        if let Some(index) = self.position(*id) {
            self.0.remove(index);
        }
    }

    pub(super) fn position(&self, id: i32) -> Option<usize> {
        self.0.iter().position(|candidate| *candidate == id)
    }

    pub(super) fn iter(&self) -> std::slice::Iter<'_, i32> {
        self.0.iter()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl super::EntityStore {
    /// Callers decide live body and player visibility for filed IDs.
    pub(crate) fn box_section_ids(
        &self,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> impl Iterator<Item = i32> + '_ {
        self.box_sections(bounds)
            .flat_map(|(_, ids)| ids.iter().copied())
    }

    /// Spatial section lookup across all filed sections overlapping `bounds`,
    /// without filtering by chunk accessibility.
    pub(crate) fn all_box_section_ids(
        &self,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> impl Iterator<Item = i32> + '_ {
        self.box_sections_matching(bounds, |_| true)
            .flat_map(|(_, ids)| ids.iter().copied())
    }

    pub(crate) fn for_each_box_section_id(
        &self,
        bounds: bbb_server_types::geometry::Aabb,
        visit: impl FnMut(i32),
    ) {
        self.box_section_ids(bounds).for_each(visit);
    }

    pub(crate) fn first_entity_in_box<'a>(
        &'a self,
        bounds: bbb_server_types::geometry::Aabb,
        mut predicate: impl FnMut(&'a crate::entity::Entity) -> bool,
    ) -> Option<&'a crate::entity::Entity> {
        self.find_map_box_sections(bounds, |_, ids| {
            ids.iter()
                .filter_map(|id| self.get(*id))
                .find(|entity| entity.bounding_box().intersects(bounds) && predicate(entity))
        })
    }

    pub(super) fn find_map_box_sections<'a, T>(
        &'a self,
        bounds: bbb_server_types::geometry::Aabb,
        mut visit: impl FnMut(super::SectionKey, &'a SectionEntities) -> Option<T>,
    ) -> Option<T> {
        self.box_sections(bounds)
            .find_map(|(section, ids)| visit(section, ids))
    }

    fn box_sections(
        &self,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> impl Iterator<Item = (super::SectionKey, &SectionEntities)> + '_ {
        self.box_sections_matching(bounds, |section| {
            self.section_visibility(section).is_accessible()
        })
    }

    fn box_sections_matching<'a>(
        &'a self,
        bounds: bbb_server_types::geometry::Aabb,
        include: impl Fn(super::SectionKey) -> bool + 'a,
    ) -> impl Iterator<Item = (super::SectionKey, &'a SectionEntities)> + 'a {
        let x_min = super::pos_to_section_coord(bounds.min_x - 2.0);
        let y_min = super::pos_to_section_coord(bounds.min_y - 4.0);
        let z_min = super::pos_to_section_coord(bounds.min_z - 2.0);
        let x_max = super::pos_to_section_coord(bounds.max_x + 2.0);
        let y_max = super::pos_to_section_coord(bounds.max_y);
        let z_max = super::pos_to_section_coord(bounds.max_z + 2.0);
        let span = |min: i32, max: i32| i64::from(max) - i64::from(min) + 1;
        let volume = span(x_min, x_max)
            .saturating_mul(span(y_min, y_max))
            .saturating_mul(span(z_min, z_max));
        let dense = volume <= self.by_section.len() as i64;
        let dense_sections = dense.then(|| {
            (x_min..=x_max).flat_map(move |x| {
                (y_min..=y_max).flat_map(move |y| {
                    (z_min..=z_max).map(move |z| super::SectionKey::new(x, y, z))
                })
            })
        });
        let sparse_sections = (!dense).then(|| {
            self.by_section.keys().copied().filter(move |section| {
                (x_min..=x_max).contains(&section.x())
                    && (y_min..=y_max).contains(&section.y())
                    && (z_min..=z_max).contains(&section.z())
            })
        });
        dense_sections
            .into_iter()
            .flatten()
            .chain(sparse_sections.into_iter().flatten())
            .filter(move |section| include(*section))
            .filter_map(move |section| self.by_section.get(&section).map(|ids| (section, ids)))
    }

    pub(crate) fn player_connection(&self, id: i32) -> Option<crate::connection::ConnectionId> {
        self.player(id)
            .map(crate::world::PlayerState::connection_id)
    }

    pub(crate) fn players_in_box_sections(
        &self,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> Vec<crate::connection::ConnectionId> {
        let mut players = Vec::new();
        self.for_each_box_section_id(bounds, |id| {
            if let Some(connection) = self.player_connection(id) {
                players.push(connection);
            }
        });
        players
    }

    pub fn refile_detached(&mut self, current: &mut crate::entity::Entity) {
        let id = current.id();
        let filed = current.filed_section();
        if !self.is_detached(id)
            || self.ids_by_uuid.get(&current.uuid()) != Some(&id)
            || self
                .by_section
                .get(&filed)
                .is_none_or(|ids| ids.position(id).is_none())
        {
            return;
        }
        let section = super::section_of_block(current.block_position());
        self.transfer_section(current, filed, section);
        current.set_filed_section(section);
    }

    pub(super) fn transfer_section(
        &mut self,
        entity: &crate::entity::Entity,
        filed: super::SectionKey,
        section: super::SectionKey,
    ) {
        if filed == section {
            return;
        }
        let id = entity.id();
        self.unfile(id, filed);
        self.file_in_section(id, section);
        self.mark_chunk_dirty(filed.column());
        if filed.column() != section.column() {
            self.mark_chunk_dirty(section.column());
        }
        self.mark_entity_chunk_dirty(id);
        self.transition_section_visibility(entity, filed, section);
    }

    pub(crate) fn iter_chunk_section_ids(
        &self,
        chunk: super::ChunkKey,
    ) -> impl Iterator<Item = i32> + '_ {
        self.sections_by_column
            .get(&chunk)
            .into_iter()
            .flat_map(|sections| {
                sections
                    .iter()
                    .filter_map(|section| self.by_section.get(section))
                    .flat_map(|ids| ids.iter().copied())
                    .filter(|id| self.get(*id).is_some())
            })
    }

    pub(crate) fn ids_in_chunk_section_order(&self, chunk: super::ChunkKey) -> SmallIds {
        self.iter_chunk_section_ids(chunk).collect()
    }
}

/// Actual bodies temporarily outside the store during nested entity callbacks.
pub struct DetachedEntityBodies<'a> {
    pub body: &'a crate::entity::Entity,
    pub parent: Option<&'a DetachedEntityBodies<'a>>,
}

#[derive(Clone, Copy)]
pub struct EntityBodyReadView<'a> {
    store: &'a super::EntityStore,
    current: Option<&'a crate::entity::Entity>,
    ancestors: Option<&'a DetachedEntityBodies<'a>>,
}

impl<'a> EntityBodyReadView<'a> {
    pub fn new(
        store: &'a super::EntityStore,
        current: Option<&'a crate::entity::Entity>,
        ancestors: Option<&'a DetachedEntityBodies<'a>>,
    ) -> Self {
        Self {
            store,
            current,
            ancestors,
        }
    }

    pub(crate) fn with_ancestors(self, ancestors: Option<&'a DetachedEntityBodies<'a>>) -> Self {
        Self {
            ancestors: ancestors.or(self.ancestors),
            ..self
        }
    }

    pub fn store(self) -> &'a super::EntityStore {
        self.store
    }

    pub fn get(self, id: i32) -> Option<&'a crate::entity::Entity> {
        self.store
            .get(id)
            .or_else(|| self.detached(|body| body.id() == id))
    }

    pub fn get_by_uuid(self, uuid: uuid::Uuid) -> Option<&'a crate::entity::Entity> {
        self.store
            .entity_id_for_uuid(uuid)
            .and_then(|id| self.get(id))
            .or_else(|| self.detached(|body| body.uuid() == uuid))
    }

    pub fn ids_in_box_sections(self, bounds: bbb_server_types::geometry::Aabb) -> SmallIds {
        self.store
            .ids_in_box_sections_where(bounds, |id| self.get(id).is_some())
    }

    pub fn ids_in_box_sections_with_players(
        self,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> SmallIds {
        self.store.ids_in_box_sections_where(bounds, |id| {
            self.get(id).is_some() || self.store.player(id).is_some()
        })
    }

    pub(crate) fn same_vehicle(self, first: i32, second: i32) -> Option<bool> {
        let root = |id| {
            if self.get(id).is_some() {
                self.root_vehicle_from(id)
            } else if self.store.player(id).is_some() {
                if self.store.vehicle(id).is_some() {
                    self.external_player_root_vehicle(id)
                } else {
                    Some(id)
                }
            } else {
                None
            }
        };
        Some(root(first)? == root(second)?)
    }

    pub(crate) fn external_player_root_vehicle(self, player_entity_id: i32) -> Option<i32> {
        let direct = self.external_player_vehicle(player_entity_id)?;
        self.root_vehicle_from(direct)
    }

    fn root_vehicle_from(self, mut root: i32) -> Option<i32> {
        let Some(first_parent) = self.valid_vehicle(root) else {
            return Some(root);
        };
        if first_parent == root {
            return None;
        }
        let Some(second_parent) = self.valid_vehicle(first_parent) else {
            return Some(first_parent);
        };
        if second_parent == root || second_parent == first_parent {
            return None;
        }
        let mut seen = std::collections::BTreeSet::from([root, first_parent, second_parent]);
        root = second_parent;
        while let Some(parent) = self.valid_vehicle(root) {
            if !seen.insert(parent) {
                return None;
            }
            root = parent;
        }
        Some(root)
    }

    pub(crate) fn external_player_vehicle(self, player_entity_id: i32) -> Option<i32> {
        if !self.store.is_external_player_passenger(player_entity_id)
            || self.get(player_entity_id).is_some()
        {
            return None;
        }
        let vehicle_id = self.store.vehicle(player_entity_id)?;
        self.get(vehicle_id)?;
        if !self
            .store
            .passenger_relation_matches(player_entity_id, Some(vehicle_id))
        {
            return None;
        }
        if vehicle_id == player_entity_id {
            return None;
        }
        if self.store.vehicle(vehicle_id).is_none() {
            return Some(vehicle_id);
        }
        let mut current = vehicle_id;
        let mut seen = std::collections::BTreeSet::from([player_entity_id]);
        loop {
            if !seen.insert(current) {
                return None;
            }
            let Some(parent) = self.store.vehicle(current) else {
                break;
            };
            if self.valid_vehicle(current) != Some(parent) {
                return None;
            }
            current = parent;
        }
        Some(vehicle_id)
    }

    pub(crate) fn external_player_count_in_tree(self, root_vehicle_id: i32) -> usize {
        if self.store.passengers(root_vehicle_id).is_empty() {
            return 0;
        }
        self.store
            .external_player_passenger_ids()
            .filter(|player_id| {
                self.external_player_root_vehicle(*player_id) == Some(root_vehicle_id)
            })
            .count()
    }

    pub(crate) fn valid_vehicle(self, passenger_id: i32) -> Option<i32> {
        let vehicle_id = self.store.vehicle(passenger_id)?;
        if passenger_id == vehicle_id
            || self.get(passenger_id).is_none()
            || self.get(vehicle_id).is_none()
            || !self
                .store
                .passenger_relation_matches(passenger_id, Some(vehicle_id))
        {
            return None;
        }
        if self.store.vehicle(vehicle_id).is_none() {
            return Some(vehicle_id);
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut current = vehicle_id;
        while seen.insert(current) {
            if current == passenger_id {
                return None;
            }
            let Some(parent) = self.store.vehicle(current) else {
                return Some(vehicle_id);
            };
            current = parent;
        }
        None
    }

    fn detached(
        self,
        matches: impl Fn(&crate::entity::Entity) -> bool,
    ) -> Option<&'a crate::entity::Entity> {
        if let Some(body) = self.current.filter(|body| matches(body)) {
            return Some(body);
        }
        let mut cursor = self.ancestors;
        while let Some(frame) = cursor {
            if matches(frame.body) {
                return Some(frame.body);
            }
            cursor = frame.parent;
        }
        None
    }
}
