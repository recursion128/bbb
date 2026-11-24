use super::{ServerWorld, WorldContainer, WorldContainerAccess};
use crate::entity::EntityData;
use crate::world::LevelId;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::Result;

impl ServerWorld {
    pub(in crate::world) fn minecart_hopper_suck(&mut self, dimension: &LevelId, entity_id: i32) {
        let Some(entity) = self.level_by_id(dimension).entities.get(entity_id) else {
            return;
        };
        if !entity.is_alive()
            || !matches!(entity.data(), EntityData::HopperMinecart(data) if data.is_enabled())
        {
            return;
        }
        let position = entity.position();
        let bounds = entity.bounding_box().inflate(0.25, 0.0, 0.25);
        let source_center = Vec3d {
            y: position.y + 1.5,
            ..position
        };
        let pos = BlockPos {
            x: source_center.x.floor() as i32,
            y: source_center.y.floor() as i32,
            z: source_center.z.floor() as i32,
        };
        let mut access = WorldContainerAccess {
            world: self,
            dimension: *dimension,
            pos,
            facing: Direction::Down,
        };
        if let Err(error) = access.suck_minecart(entity_id, position, source_center, bounds) {
            access
                .world
                .with_level(dimension, |level| level.record_item_failure(pos, error));
        }
    }
}

impl WorldContainerAccess<'_> {
    fn suck_minecart(
        &mut self,
        id: i32,
        position: Vec3d,
        source_center: Vec3d,
        bounds: Aabb,
    ) -> Result<()> {
        let into = WorldContainer::Entity(id);
        if let Some(from) = self.container_at_center(self.pos, source_center) {
            if self.pull_from(into, from)? {
                return Ok(());
            }
        } else {
            let area = crate::block_entity::hopper::SUCK_AABB.moved(Vec3d {
                x: position.x - 0.5,
                y: position.y,
                z: position.z - 0.5,
            });
            if self.take_items_in(into, area)? {
                return Ok(());
            }
        }
        // A source container suppresses the upper scan, but not the cart's own pickup box.
        self.take_items_in(into, bounds)?;
        Ok(())
    }

    fn take_items_in(&mut self, into: WorldContainer, bounds: Aabb) -> Result<bool> {
        let ids = self
            .world
            .level_by_id(&self.dimension)
            .entities
            .ids_in_box_sections(bounds);
        for id in ids {
            if self
                .world
                .level_by_id(&self.dimension)
                .entities
                .get(id)
                .is_some_and(|entity| entity.bounding_box().intersects(bounds))
                && self.add_item_entity(into, id)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests;
