use super::{ServerWorld, WorldEvent};
use crate::chunk::poi::PoiOccupancy;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::RichTextComponent;
use bbb_server_types::geometry::Aabb;
use bbb_server_types::save::Java26BuiltinDimension;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct CatSpawner {
    next_tick: i32,
}

impl ServerWorld {
    pub(super) fn tick_cat_spawner(&mut self, dimension: &LevelId, events: &mut Vec<WorldEvent>) {
        if dimension.builtin() != Some(Java26BuiltinDimension::Overworld)
            || !self.game_rules.spawn_mobs()
        {
            return;
        }
        if let Err(detail) = self.advance_cat_spawner(dimension) {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!("cat spawn: {detail}"),
            )));
        }
    }

    fn advance_cat_spawner(&mut self, dimension: &LevelId) -> Result<(), String> {
        // CatSpawner.java:23-35; ServerLevel.java:939-942.
        self.cold.cat_spawner.next_tick = self.cold.cat_spawner.next_tick.wrapping_sub(1);
        if self.cold.cat_spawner.next_tick > 0 {
            return Ok(());
        }
        self.cold.cat_spawner.next_tick = 1_200;
        let players: Vec<_> = self.levels[dimension]
            .player_order
            .iter()
            .filter_map(|id| self.levels.player(*id))
            .filter(|player| !player.avatar_removed && !player.damage.is_dead_or_dying())
            .map(|player| BlockPos {
                x: player.position().x.floor() as i32,
                y: player.position().y.floor() as i32,
                z: player.position().z.floor() as i32,
            })
            .collect();
        if players.is_empty() {
            return Ok(());
        }
        let random = &self.random;
        let index = random.next_int(
            i32::try_from(players.len()).map_err(|_| "player count exceeds Java list bounds")?,
        ) as usize;
        let center = players[index];
        let x = (8 + random.next_int(24)) * if random.next_boolean() { -1 } else { 1 };
        let z = (8 + random.next_int(24)) * if random.next_boolean() { -1 } else { 1 };
        let position = BlockPos {
            x: center.x.wrapping_add(x),
            y: center.y,
            z: center.z.wrapping_add(z),
        };
        if !self.custom_spawn_has_chunks_at(dimension, position)
            || !self.cat_spawn_position_ok(dimension, position)?
        {
            return Ok(());
        }
        self.try_spawn_local_cat(dimension, position)
    }

    fn try_spawn_local_cat(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
    ) -> Result<(), String> {
        // CatSpawner.java:36-63: a village candidate never falls back to the hut arm.
        let village = self
            .is_close_to_village_id(dimension, [position.x, position.y, position.z], 2)
            .ok_or("missing village POI context")?;
        if village {
            let homes = self
                .poi_in_range_id(
                    dimension,
                    [position.x, position.y, position.z],
                    48,
                    PoiOccupancy::IsOccupied,
                    |poi| poi.type_id == "minecraft:home",
                )
                .ok_or("missing occupied HOME context")?;
            if homes.len() > 4 && self.local_cat_count(dimension, position, 48.0) < 5 {
                self.spawn_custom_cat(dimension, position, false)?;
            }
        } else if self.cat_spawn_in_hut(dimension, position)
            && self.local_cat_count(dimension, position, 16.0) == 0
        {
            self.spawn_custom_cat(dimension, position, true)?;
        }
        Ok(())
    }

    fn local_cat_count(&self, dimension: &LevelId, position: BlockPos, radius: f64) -> usize {
        let bounds = Aabb::of_block(position).inflate(radius, 8.0, radius);
        let entities = &self.levels[dimension].entities;
        entities
            .ids_in_box_sections(bounds)
            .into_iter()
            .filter(|id| {
                entities.get(*id).is_some_and(|entity| {
                    entity.entity_type_id() == VANILLA_ENTITY_TYPE_CAT_ID
                        && entity.bounding_box().intersects(bounds)
                })
            })
            .count()
    }
}

#[cfg(test)]
mod tests;
