use super::*;
use crate::block_entity::BlockEntity;
use crate::world::LevelId;
use bbb_protocol::block_types::{VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_END_STONE_ID};

impl ServerWorld {
    fn gateway_highest_section(
        &mut self,
        dimension: &LevelId,
        x: f64,
        z: f64,
    ) -> SaveResult<Option<usize>> {
        let chunk = ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32);
        self.publish_full_columns(dimension, &[chunk])?;
        let chunks = &self.level_by_id(dimension).chunks;
        let column = chunks
            .column(chunk.0, chunk.1)
            .ok_or_else(|| invalid_search("gateway FULL column missing"))?;
        Ok((0..column.section_count()).rev().find(|&index| {
            column
                .section(index)
                .count_cells(|state| !bbb_server_types::fluid::block_state_is_air(state))
                != 0
        }))
    }

    fn gateway_full_collision(&self, dimension: &LevelId, pos: BlockPos) -> SaveResult<bool> {
        let chunks = &self.level_by_id(dimension).chunks;
        let boxes =
            super::super::movement_collision::block_collision_shape_checked(chunks, pos, None)
                .ok_or_else(|| invalid_search("gateway collision unavailable"))?;
        let shape = bbb_server_types::geometry::BoxShape::from_boxes(&boxes)
            .and_then(|shape| shape.optimized())
            .ok_or_else(|| invalid_search("gateway collision cannot be resolved"))?;
        Ok(shape.boxes().as_slice() == [Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })])
    }

    // TheEndGatewayBlockEntity.java:202-221.
    fn gateway_tallest(
        &mut self,
        dimension: &LevelId,
        around: BlockPos,
        distance: i32,
        allow_bedrock: bool,
    ) -> SaveResult<BlockPos> {
        let mut columns = Vec::new();
        for x in (around.x - distance) >> 4..=(around.x + distance) >> 4 {
            for z in (around.z - distance) >> 4..=(around.z + distance) >> 4 {
                columns.push((x, z));
            }
        }
        self.publish_full_columns(dimension, &columns)?;
        let chunks = &self.level_by_id(dimension).chunks;
        let min_y = chunks.min_y();
        let max_y = min_y + chunks.section_count() as i32 * 16 - 1;
        let mut tallest: Option<BlockPos> = None;
        for dx in -distance..=distance {
            for dz in -distance..=distance {
                if dx == 0 && dz == 0 && !allow_bedrock {
                    continue;
                }
                let bottom = tallest.map_or(min_y, |pos| pos.y);
                for y in ((bottom + 1)..=max_y).rev() {
                    let pos = BlockPos {
                        x: around.x + dx,
                        y,
                        z: around.z + dz,
                    };
                    if self.gateway_full_collision(dimension, pos)?
                        && (allow_bedrock
                            || vanilla_block_state_block_type_id(super::super::stored_block_state(
                                chunks, pos,
                            )) != Some(VANILLA_BLOCK_BEDROCK_ID))
                    {
                        tallest = Some(pos);
                        break;
                    }
                }
            }
        }
        Ok(tallest.unwrap_or(around))
    }

    // TheEndGatewayBlockEntity.java:175-200,227-255; BlockPos.java:405-427.
    fn gateway_new_exit(&mut self, dimension: &LevelId, entry: BlockPos) -> SaveResult<BlockPos> {
        let length = (f64::from(entry.x).powi(2) + f64::from(entry.z).powi(2)).sqrt();
        let (dx, dz) = if length < 1.0e-5 {
            (0.0, 0.0)
        } else {
            (f64::from(entry.x) / length, f64::from(entry.z) / length)
        };
        let (mut x, mut z) = (dx * 1024.0, dz * 1024.0);
        for _ in 0..16 {
            if self.gateway_highest_section(dimension, x, z)?.is_none() {
                break;
            }
            x -= dx * 16.0;
            z -= dz * 16.0;
        }
        for _ in 0..16 {
            if self.gateway_highest_section(dimension, x, z)?.is_some() {
                break;
            }
            x += dx * 16.0;
            z += dz * 16.0;
        }
        let highest = self.gateway_highest_section(dimension, x, z)?;
        let chunks = &self.level_by_id(dimension).chunks;
        let max_y = chunks.min_y() + highest.map_or(0, |section| section as i32 * 16) + 15;
        let (min_x, min_z) = (
            ((x / 16.0).floor() as i32) << 4,
            ((z / 16.0).floor() as i32) << 4,
        );
        let mut closest: Option<(BlockPos, f64)> = None;
        for z in min_z..=min_z + 15 {
            for y in 30..=max_y {
                for x in min_x..=min_x + 15 {
                    let pos = BlockPos { x, y, z };
                    if vanilla_block_state_block_type_id(super::super::stored_block_state(
                        chunks, pos,
                    )) != Some(VANILLA_BLOCK_END_STONE_ID)
                    {
                        continue;
                    }
                    if self.gateway_full_collision(dimension, BlockPos { y: y + 1, ..pos })?
                        || self.gateway_full_collision(dimension, BlockPos { y: y + 2, ..pos })?
                    {
                        continue;
                    }
                    let distance = (f64::from(x) + 0.5).powi(2)
                        + (f64::from(y) + 0.5).powi(2)
                        + (f64::from(z) + 0.5).powi(2);
                    if closest.is_none_or(|(_, best)| distance < best) {
                        closest = Some((pos, distance));
                    }
                }
            }
        }
        let candidate = if let Some((pos, _)) = closest {
            pos
        } else {
            let pos = BlockPos {
                x: (x + 0.5).floor() as i32,
                y: 75,
                z: (z + 0.5).floor() as i32,
            };
            let mut island_columns = Vec::new();
            for cz in ((pos.z - 7) >> 4)..=((pos.z + 7) >> 4) {
                for cx in ((pos.x - 7) >> 4)..=((pos.x + 7) >> 4) {
                    island_columns.push((cx, cz));
                }
            }
            self.publish_full_columns(dimension, &island_columns)?;
            self.place_gateway_island(*dimension, pos);
            pos
        };
        self.gateway_tallest(dimension, candidate, 16, true)
    }

    // TheEndGatewayBlockEntity.java:130-150.
    pub(in crate::world) fn resolve_gateway_destination(
        &mut self,
        dimension: &LevelId,
        entry: BlockPos,
    ) -> SaveResult<Option<Vec3d>> {
        let Some((mut exit, exact)) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(entry)
            .and_then(BlockEntity::end_gateway)
            .map(|gateway| (gateway.exit_portal(), gateway.exact_teleport()))
        else {
            return Ok(None);
        };
        if exit.is_none() && dimension.builtin() == Some(Java26BuiltinDimension::End) {
            let ground = self.gateway_new_exit(dimension, entry)?;
            let portal = BlockPos {
                y: ground.y + 10,
                ..ground
            };
            let mut return_columns = Vec::new();
            for cz in ((portal.z - 1) >> 4)..=((portal.z + 1) >> 4) {
                for cx in ((portal.x - 1) >> 4)..=((portal.x + 1) >> 4) {
                    return_columns.push((cx, cz));
                }
            }
            self.publish_full_columns(dimension, &return_columns)?;
            self.create_gateway_return(*dimension, portal, entry);
            let level = self.level_mut_by_id(dimension);
            if let Some(gateway) = level
                .chunks
                .block_entities_mut()
                .get_mut(entry)
                .and_then(BlockEntity::end_gateway_mut)
            {
                gateway.set_exit_position(portal, exact);
                level.chunks.block_entities_mut().set_changed(entry);
            }
            exit = Some(portal);
        }
        let Some(exit) = exit else {
            return Ok(None);
        };
        let pos = if exact {
            exit
        } else {
            let around = BlockPos {
                y: exit.y + 2,
                ..exit
            };
            let ground = self.gateway_tallest(dimension, around, 5, false)?;
            BlockPos {
                y: ground.y + 1,
                ..ground
            }
        };
        Ok(Some(Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        }))
    }

    pub(super) fn transfer_stored_entity_through_gateway(
        &mut self,
        source: &LevelId,
        entity_id: i32,
        entry: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> SaveResult<bool> {
        let Some(position) = self.resolve_gateway_destination(source, entry)? else {
            return Ok(false);
        };
        if !self.portal_destination_allowed(source) {
            return Ok(false);
        }
        let pearl = self
            .level_by_id(source)
            .entities
            .get(entity_id)
            .is_some_and(|entity| {
                entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_PEARL_ID
            });
        let transition = StoredEntityTransition {
            position,
            delta_movement: Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: if pearl { 0 } else { PORTAL_RELATIVES_MASK },
        };
        let Some(tree) = super::super::entity_transfer::resolved_tree(
            &self.level_by_id(source).entities,
            entity_id,
            transition,
            true,
        )
        .map_err(|error| invalid_search(format!("gateway tree: {error}")))?
        else {
            return Ok(false);
        };
        let arrivals = tree
            .into_iter()
            .map(|(_, position, _)| block_position(position))
            .collect::<Vec<_>>();
        let ticket = self.prepare_portal_tickets_by_key(source, &arrivals, &[])?;
        let commit = self.publish_portal_destination(ticket);
        if self
            .teleport_entity_subtree(*source, *source, entity_id, transition, false, events)
            .map_err(|error| invalid_search(format!("gateway teleport: {error}")))?
            .is_none()
        {
            return Ok(false);
        }
        self.commit_portal_ticket_state(commit);
        Ok(true)
    }
}
