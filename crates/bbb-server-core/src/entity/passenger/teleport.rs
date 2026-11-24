use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::Vec3d;

use super::vehicle_geometry;
use crate::entity::{store::EntityStore, Entity};

#[derive(Debug, Clone)]
struct Identity {
    uuid: uuid::Uuid,
    entity_type: i32,
}

#[derive(Debug, Clone)]
pub struct TeleportTree {
    root: i32,
    entities: BTreeMap<i32, Entity>,
    original: BTreeMap<i32, Identity>,
    children: BTreeMap<i32, Vec<i32>>,
    vehicles: BTreeMap<i32, Option<i32>>,
    preorder: Vec<i32>,
    snapped: bool,
    rotated: BTreeSet<i32>,
    external: BTreeMap<i32, ExternalPlayerTeleportBody>,
    player_snaps: Vec<ExternalPlayerSnap>,
    apply_order: Vec<i32>,
    next_step: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum TeleportApplication {
    Native {
        entity_id: i32,
        waypoint_updated: bool,
    },
    Player(ExternalPlayerSnap),
}

#[derive(Debug, Clone, Copy)]
pub struct ExternalPlayerTeleportBody {
    pub uuid: uuid::Uuid,
    pub y_rot: f32,
    pub scale: f32,
    pub pose_id: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct ExternalPlayerSnap {
    pub entity_id: i32,
    pub uuid: uuid::Uuid,
    pub placement: vehicle_geometry::ExternalPlayerVehiclePlacement,
}

impl TeleportTree {
    pub(crate) fn capture(body: &Entity, store: &EntityStore) -> Option<Self> {
        Self::capture_with_players(body, store, |_| None)
    }

    pub fn capture_with_players(
        body: &Entity,
        store: &EntityStore,
        mut player: impl FnMut(i32) -> Option<ExternalPlayerTeleportBody>,
    ) -> Option<Self> {
        store.checked_same_vehicle(body, body.id())?;
        if store.entity_id_for_uuid(body.uuid()) != Some(body.id()) {
            return None;
        }
        if let Some(actual) = store.get(body.id()) {
            if actual.uuid() != body.uuid() || actual.entity_type_id() != body.entity_type_id() {
                return None;
            }
        }
        let mut tree = Self {
            root: body.id(),
            entities: BTreeMap::new(),
            original: BTreeMap::new(),
            children: BTreeMap::new(),
            vehicles: BTreeMap::new(),
            preorder: Vec::new(),
            snapped: false,
            rotated: BTreeSet::new(),
            external: BTreeMap::new(),
            player_snaps: Vec::new(),
            apply_order: Vec::new(),
            next_step: 0,
        };
        let mut pending = vec![body.id()];
        while let Some(id) = pending.pop() {
            let entity = if id == body.id() {
                body
            } else {
                store.get(id)?
            };
            if tree.entities.insert(id, entity.clone()).is_some() {
                return None;
            }
            let children = store.checked_teleport_passengers(body, id)?;
            for (index, child) in children.iter().enumerate() {
                if store.is_external_player_passenger(*child) {
                    let body = player(*child)?;
                    vehicle_geometry::external_player_placement(
                        entity,
                        index,
                        children.len(),
                        body.y_rot,
                        body.scale,
                        body.pose_id,
                        0,
                    )?;
                    tree.external.insert(*child, body);
                } else {
                    vehicle_geometry::placement(
                        entity,
                        store.get(*child)?,
                        index,
                        children.len(),
                        0,
                    )?;
                }
            }
            pending.extend(
                children
                    .iter()
                    .rev()
                    .copied()
                    .filter(|id| !tree.external.contains_key(id)),
            );
            tree.preorder.push(id);
            tree.vehicles.insert(id, store.vehicle(id));
            tree.children.insert(id, children);
        }
        tree.original = tree
            .entities
            .iter()
            .map(|(id, entity)| {
                (
                    *id,
                    Identity {
                        uuid: entity.uuid(),
                        entity_type: entity.entity_type_id(),
                    },
                )
            })
            .collect();
        Some(tree)
    }

    #[cfg(test)]
    pub(crate) fn body(&self) -> &Entity {
        &self.entities[&self.root]
    }

    #[cfg(test)]
    pub(crate) fn overlay(&self) -> &BTreeMap<i32, Entity> {
        &self.entities
    }

    pub fn snap_to(&mut self, destination: Vec3d, game_time: i64) -> Option<()> {
        self.snapped = false;
        self.rotated.clear();
        self.player_snaps.clear();
        self.apply_order.clear();
        self.next_step = 0;
        if !destination.x.is_finite() || !destination.y.is_finite() || !destination.z.is_finite() {
            return None;
        }
        // Entity.java:3264-3275: each parent snaps its children before recursing.
        let root = self.entities.get_mut(&self.root)?;
        root.set_pos(destination);
        root.set_old_pos_and_rot();
        for id in &self.preorder {
            let children = &self.children[id];
            for (index, child) in children.iter().enumerate() {
                if let Some(player) = self.external.get(child) {
                    self.player_snaps.push(ExternalPlayerSnap {
                        entity_id: *child,
                        uuid: player.uuid,
                        placement: vehicle_geometry::external_player_placement(
                            &self.entities[id],
                            index,
                            children.len(),
                            player.y_rot,
                            player.scale,
                            player.pose_id,
                            game_time,
                        )?,
                    });
                    continue;
                }
                let placement = vehicle_geometry::placement(
                    &self.entities[id],
                    &self.entities[child],
                    index,
                    children.len(),
                    game_time,
                )?;
                if placement.apply_snap(self.entities.get_mut(child)?) {
                    self.rotated.insert(*child);
                }
            }
        }
        self.apply_order.push(self.root);
        for parent in &self.preorder {
            self.apply_order
                .extend(self.children[parent].iter().copied());
        }
        self.snapped = true;
        Some(())
    }

    pub(crate) fn apply_to(mut self, body: &mut Entity, store: &mut EntityStore) -> Option<()> {
        if !self.external.is_empty() {
            return None;
        }
        while let Some(step) = self.apply_next(body, store)? {
            if let TeleportApplication::Native {
                entity_id,
                waypoint_updated: true,
            } = step
            {
                let actual = if entity_id == body.id() {
                    &mut *body
                } else {
                    store.get_mut(entity_id)?
                };
                actual.mark_waypoint_update_pending();
                if entity_id == body.id() {
                    store.refile_detached(body);
                } else {
                    store.refile(entity_id);
                }
            }
        }
        Some(())
    }

    // Entity.java:3264-3275,3740-3748: publish each live snap before the next child.
    pub fn apply_next(
        &mut self,
        body: &mut Entity,
        store: &mut EntityStore,
    ) -> Option<Option<TeleportApplication>> {
        if !self.snapped || store.get(self.root).is_some() {
            return None;
        }
        store.checked_same_vehicle(body, body.id())?;
        for id in &self.preorder {
            let actual = if *id == self.root {
                &*body
            } else {
                store.get(*id)?
            };
            let original = &self.original[id];
            if actual.id() != *id
                || actual.uuid() != original.uuid
                || actual.entity_type_id() != original.entity_type
                || store.vehicle(*id) != self.vehicles[id]
                || store.checked_teleport_passengers(body, *id)? != self.children[id]
            {
                return None;
            }
        }
        let Some(&id) = self.apply_order.get(self.next_step) else {
            return Some(None);
        };
        let step = if self.external.contains_key(&id) {
            TeleportApplication::Player(
                *self.player_snaps.iter().find(|snap| snap.entity_id == id)?,
            )
        } else {
            let actual = if id == self.root {
                &mut *body
            } else {
                store.get_mut(id)?
            };
            let staged = &self.entities[&id];
            actual.set_pos(staged.position());
            let waypoint_updated = actual.take_waypoint_update_pending();
            actual.set_old_pos_and_rot();
            if id != self.root && self.rotated.contains(&id) {
                actual.set_rot(staged.y_rot(), actual.x_rot());
                if let (Some(staged), Some(actual)) = (staged.living(), actual.living_mut()) {
                    actual.y_head_rot = staged.y_head_rot;
                    actual.y_body_rot = staged.y_body_rot;
                }
            }
            if id == self.root {
                store.refile_detached(body);
            } else {
                store.refile(id);
            }
            TeleportApplication::Native {
                entity_id: id,
                waypoint_updated,
            }
        };
        self.next_step += 1;
        Some(Some(step))
    }
}

#[cfg(test)]
mod tests;
