use super::{WorldContainer, WorldContainerAccess};
use crate::block_entity::hopper::{merge_item, HOPPER_CONTAINER_SIZE};
use bbb_protocol::packets::Direction;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

impl WorldContainerAccess<'_> {
    pub(super) fn transfer_items(
        &mut self,
        hopper: WorldContainer,
        item_id: Option<i32>,
    ) -> Result<bool> {
        let mut changed = false;
        if !self.is_empty(hopper)? {
            if let Some((into, direction)) = self.attached() {
                changed = self.eject(hopper, into, direction)?;
            }
        }
        let mut full = true;
        for slot in 0..HOPPER_CONTAINER_SIZE {
            let item = self.get_item(hopper, slot)?;
            if item.is_empty() || item.count() != item.max_stack_size()? {
                full = false;
                break;
            }
        }
        if !full {
            if let Some(item_id) = item_id {
                changed |= self.add_item_entity(hopper, item_id)?;
            } else if let Some(from) = self.source() {
                changed |= self.pull_from(hopper, from)?;
            } else if !self.items_blocked_above() {
                let area =
                    crate::block_entity::hopper::SUCK_AABB.moved(bbb_protocol::packets::Vec3d {
                        x: f64::from(self.pos.x),
                        y: f64::from(self.pos.y),
                        z: f64::from(self.pos.z),
                    });
                let ids = self
                    .world
                    .level_by_id(&self.dimension)
                    .entities
                    .ids_in_box_sections(area);
                for id in ids {
                    let inside = self
                        .world
                        .level_by_id(&self.dimension)
                        .entities
                        .get(id)
                        .is_some_and(|entity| {
                            entity.is_alive()
                                && entity.bounding_box().intersects(area)
                                && matches!(entity.data(), crate::entity::EntityData::Item(_))
                        });
                    if inside && self.add_item_entity(hopper, id)? {
                        changed = true;
                        break;
                    }
                }
            }
        }
        Ok(changed)
    }

    pub(super) fn pull_from(
        &mut self,
        hopper: WorldContainer,
        from: WorldContainer,
    ) -> Result<bool> {
        for slot in self.slots(from, Some(Direction::Down)) {
            let original = self.get_item(from, slot)?;
            if original.is_empty() || !self.can_take(from, slot, &original, hopper)? {
                continue;
            }
            let moving = if self.extracts_entire_slot(from, slot) {
                original
            } else {
                original.with_count(1)
            };
            let remaining = self.add_item(Some(from), hopper, moving, None)?;
            if remaining.is_empty() {
                self.remove_one(from, slot)?;
                self.set_changed(from);
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn items_blocked_above(&mut self) -> bool {
        let pos = crate::fluid::above(self.pos);
        let chunks = &self.world.level_by_id(&self.dimension).chunks;
        let state = crate::world::level_blocks::stored_block_state(chunks, pos);
        let in_tag = crate::block_getter::BlockTags::block_in_tag(
            chunks
                .registry_tags()
                .unwrap_or_else(|| crate::registry_tags::RegistryTagSnapshot::builtin()),
            state,
            "minecraft:does_not_block_hoppers",
        );
        if in_tag {
            return false;
        }
        if crate::block_behavior::collision_shape_full_block_at(state, pos) {
            return true;
        }
        crate::world::movement_collision::block_collision_shape_checked(chunks, pos, None)
            .and_then(|boxes| bbb_server_types::geometry::BoxShape::from_boxes(&boxes))
            .and_then(|shape| shape.optimized())
            .is_some_and(|shape| {
                shape.boxes().as_slice()
                    == [bbb_server_types::geometry::Aabb::of_block(
                        bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
                    )]
            })
    }

    pub(super) fn add_item_entity(&mut self, hopper: WorldContainer, id: i32) -> Result<bool> {
        let Some(entity) = self.world.level_by_id(&self.dimension).entities.get(id) else {
            return Ok(false);
        };
        let crate::entity::EntityData::Item(data) = entity.data() else {
            return Ok(false);
        };
        if !entity.is_alive() || data.item().is_empty() {
            return Ok(false);
        }
        let remaining = self.add_item(None, hopper, data.item().clone(), None)?;
        let consumed = remaining.is_empty();
        let store = &mut self.world.level_mut_by_id(&self.dimension).entities;
        if let Some(entity) = store.get_mut(id) {
            entity.set_dropped_item(remaining);
        }
        if consumed {
            store.remove(id, crate::entity::RemovalReason::Discarded);
        }
        Ok(consumed)
    }

    fn eject(
        &mut self,
        from: WorldContainer,
        into: WorldContainer,
        direction: Direction,
    ) -> Result<bool> {
        for slot in self.slots(into, Some(direction)) {
            let item = self.get_item(into, slot)?;
            if item.count() < item.max_stack_size()? {
                return self.eject_into_available(from, into, direction);
            }
        }
        Ok(false)
    }

    fn eject_into_available(
        &mut self,
        from: WorldContainer,
        into: WorldContainer,
        direction: Direction,
    ) -> Result<bool> {
        for slot in 0..self.size(from) {
            let original = self.get_item(from, slot)?;
            if original.is_empty() {
                continue;
            }
            let remaining =
                self.add_item(Some(from), into, original.with_count(1), Some(direction))?;
            if remaining.is_empty() {
                self.remove_one(from, slot)?;
                self.set_changed(into);
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(in crate::world) fn add_item(
        &mut self,
        from: Option<WorldContainer>,
        into: WorldContainer,
        mut item: RuntimeItemStack,
        direction: Option<Direction>,
    ) -> Result<RuntimeItemStack> {
        let resolver = self.world.item_stack_resolver();
        let mut was_empty = self.is_empty(into)?;
        for slot in self.slots(into, direction) {
            if item.is_empty() {
                return Ok(RuntimeItemStack::empty());
            }
            let mut current = self.get_item(into, slot)?;
            if !self.can_place(into, slot, &item, direction)? {
                continue;
            }
            let slot_empty = current.is_empty();
            let success = merge_item(&mut current, &mut item, &resolver)?;
            if success {
                if slot_empty {
                    self.set_item(into, slot, current)?;
                } else {
                    self.set_count(into, slot, current.count())?;
                }
                if was_empty {
                    self.receiving_hopper_cooldown(from, into);
                    was_empty = false;
                }
                self.set_changed(into);
            }
        }
        if item.is_empty() {
            Ok(RuntimeItemStack::empty())
        } else {
            Ok(item)
        }
    }
}
