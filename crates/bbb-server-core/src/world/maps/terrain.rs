//! Held-map terrain sampling against published FULL columns.

use super::ServerMapStore;
use crate::block_behavior::default_block_state_id;
use crate::connection::ConnectionId;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{vanilla_block_state_map_color_id, VanillaBlockSupportType};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_STONE_ID,
    VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::fluid::block_state_fluid_state;

#[derive(Clone, Copy)]
pub(in crate::world) struct HeldMapTerrainTask {
    pub(in crate::world) connection_id: ConnectionId,
    pub(in crate::world) map_id: i32,
    pub(in crate::world) x: f64,
    pub(in crate::world) z: f64,
}

struct AreaSample {
    material: u8,
    height: f64,
    water_depth: i32,
}

impl ServerMapStore {
    /// `MapItem.update` (`MapItem.java:76-187`) for one holder: each tick
    /// redraws the reveal-radius columns with `x % 16 == step % 16`, and a
    /// column that changed also redraws the next one. Returns the chunks a
    /// sampled area still waits on.
    pub(in crate::world) fn scan_held_terrain(
        &mut self,
        level: &LevelCtx<'_, '_>,
        task: HeldMapTerrainTask,
        has_ceiling: bool,
    ) -> Vec<(i32, i32)> {
        let mut waiting = Vec::new();
        let Some(map) = self.ensure_loaded_mut(task.map_id) else {
            return waiting;
        };
        // MapItem.java:77-89 and MapItem.java:283-284: only an unlocked same-level hand map draws.
        if map.locked || map.dimension != level.dimension.as_str() {
            return waiting;
        }
        let scale = 1_i32 << map.scale;
        let player_x = (task.x - f64::from(map.center_x)).floor() as i32 / scale + 64;
        let player_y = (task.z - f64::from(map.center_z)).floor() as i32 / scale + 64;
        let radius = 128 / scale / if has_ceiling { 2 } else { 1 };
        let (center_x, center_z) = (map.center_x, map.center_z);
        let Some(holder) = map.holding_players.get_mut(&task.connection_id) else {
            return waiting;
        };
        holder.step = holder.step.wrapping_add(1);
        let step = holder.step;
        let mut ready: Option<((i32, i32), bool)> = None;
        let mut changed = false;
        for x in
            (player_x.saturating_sub(radius) + 1).max(0)..player_x.saturating_add(radius).min(128)
        {
            if (x & 15) != (step & 15) && !changed {
                continue;
            }
            changed = false;
            let mut previous_height = 0.0;
            for y in (player_y.saturating_sub(radius) - 1).max(-1)
                ..player_y.saturating_add(radius).min(128)
            {
                let world_x = (center_x / scale + x - 64).wrapping_mul(scale);
                let world_z = (center_z / scale + y - 64).wrapping_mul(scale);
                let chunk = (world_x >> 4, world_z >> 4);
                let is_ready = match ready {
                    Some((at, is_ready)) if at == chunk => is_ready,
                    _ => {
                        let is_ready = level
                            .chunks
                            .validate_served_full_column(chunk.0, chunk.1)
                            .is_ok()
                            && level
                                .chunks
                                .published_worldgen_entity_records(chunk.0, chunk.1)
                                .is_empty();
                        ready = Some((chunk, is_ready));
                        is_ready
                    }
                };
                if !is_ready {
                    if !waiting.contains(&chunk) {
                        waiting.push(chunk);
                    }
                    continue;
                }
                if !has_ceiling {
                    if let Some(map) = self.maps.get_mut(&task.map_id) {
                        map.check_banners_in_area(level, world_x, world_z, scale, scale);
                    }
                }
                let sample = sample_area(level, world_x, world_z, scale, has_ceiling);
                let parity = (x + y) & 1;
                // MapItem.java:159-175 and MapColor.java:107-111 define the material shading.
                let brightness = if sample.material == material(VANILLA_BLOCK_WATER_ID) {
                    let diff = f64::from(sample.water_depth) * 0.1 + f64::from(parity) * 0.2;
                    if diff < 0.5 {
                        2
                    } else if diff > 0.9 {
                        0
                    } else {
                        1
                    }
                } else {
                    let diff = (sample.height - previous_height) * 4.0 / f64::from(scale + 4)
                        + (f64::from(parity) - 0.5) * 0.4;
                    if diff > 0.6 {
                        2
                    } else if diff < -0.6 {
                        0
                    } else {
                        1
                    }
                };
                previous_height = sample.height;
                let distance = (x - player_x).pow(2) + (y - player_y).pow(2);
                // MapItem.java:101-104 and MapItem.java:179-181: circular reveal with a dithered edge.
                if y >= 0
                    && distance < radius * radius
                    && (distance <= (radius - 2) * (radius - 2) || parity != 0)
                {
                    changed |= self
                        .update_color(
                            task.map_id,
                            x as usize,
                            y as usize,
                            sample.material << 2 | brightness,
                        )
                        .expect("sampled map pixel is inside the map");
                }
            }
        }
        waiting
    }
}

fn material(block: i32) -> u8 {
    vanilla_block_state_map_color_id(default_block_state_id(block))
        .expect("builtin block has map material facts")
}

fn sample_area(
    level: &LevelCtx<'_, '_>,
    x: i32,
    z: i32,
    scale: i32,
    has_ceiling: bool,
) -> AreaSample {
    // MapItem.java:110-119: a dimension with a ceiling shows deterministic dirt/stone noise.
    if has_ceiling {
        let noise = x.wrapping_add(z.wrapping_mul(231_871));
        let noise = noise
            .wrapping_mul(noise)
            .wrapping_mul(31_287_121)
            .wrapping_add(noise.wrapping_mul(11));
        return AreaSample {
            material: material(if (noise >> 20) & 1 == 0 {
                VANILLA_BLOCK_DIRT_ID
            } else {
                VANILLA_BLOCK_STONE_ID
            }),
            height: 100.0,
            water_depth: 0,
        };
    }
    let mut counts = [0_u16; 64];
    let mut order = [0_u8; 64];
    let mut order_len = 0;
    let mut height = 0;
    let mut depth = 0;
    let min_y = level.chunks.min_y();
    let none = material(VANILLA_BLOCK_AIR_ID);
    // MapItem.java:121-157: scan past NONE, retain first material on equal area counts.
    for dx in 0..scale {
        for dz in 0..scale {
            let (sx, sz) = (x + dx, z + dz);
            let mut y = level
                .chunks
                .final_heightmap_first_available(
                    sx >> 4,
                    sz >> 4,
                    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                    sx & 15,
                    sz & 15,
                )
                .expect("published FULL has a world surface heightmap");
            let content_registry = level.chunks.content_registry();
            let mut state = default_block_state_id(VANILLA_BLOCK_BEDROCK_ID);
            if y > min_y {
                loop {
                    y -= 1;
                    state = level.block_state(BlockPos { x: sx, y, z: sz });
                    if block_map_color_id(state, content_registry, none) != none || y <= min_y {
                        break;
                    }
                }
                let fluid = block_state_fluid_state(state);
                if y > min_y && !fluid.is_empty() {
                    let mut below_y = y - 1;
                    loop {
                        let below = level.block_state(BlockPos {
                            x: sx,
                            y: below_y,
                            z: sz,
                        });
                        below_y -= 1;
                        depth += 1;
                        if below_y <= min_y || block_state_fluid_state(below).is_empty() {
                            break;
                        }
                    }
                    // MapItem.java:191-193: a non-supporting fluid surface uses its fluid's block.
                    if !bbb_server_types::block_queries::face_sturdy_at_with_registry(
                        state,
                        BlockPos { x: sx, y, z: sz },
                        Direction::Up,
                        VanillaBlockSupportType::Full,
                        content_registry,
                    ) {
                        state = fluid.create_legacy_block();
                    }
                }
            }
            height += y;
            let color = block_map_color_id(state, content_registry, none);
            if counts[color as usize] == 0 {
                order[order_len] = color;
                order_len += 1;
            }
            counts[color as usize] += 1;
        }
    }
    let mut dominant = none;
    let mut most = 0;
    for &color in &order[..order_len] {
        if counts[color as usize] > most {
            dominant = color;
            most = counts[color as usize];
        }
    }
    AreaSample {
        material: dominant,
        height: f64::from(height) / f64::from(scale * scale),
        water_depth: depth / (scale * scale),
    }
}

fn block_map_color_id(
    state: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    none: u8,
) -> u8 {
    if let Some(color) = vanilla_block_state_map_color_id(state) {
        return color;
    }
    if content_registry
        .block_by_state_id(state)
        .is_some_and(|(block, _)| block.full_cube_collision)
    {
        return vanilla_block_state_map_color_id(default_block_state_id(VANILLA_BLOCK_STONE_ID))
            .unwrap_or(none);
    }
    none
}
