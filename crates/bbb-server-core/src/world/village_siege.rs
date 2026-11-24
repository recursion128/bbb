use super::{ServerWorld, WorldEvent};
use crate::world::LevelId;
use bbb_protocol::game_types::GameType;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::RichTextComponent;
use bbb_server_types::geometry::{mth_cos, mth_sin};
use bbb_server_types::save::Java26BuiltinDimension;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct VillageSiege {
    active: bool,
    setup: bool,
    remaining: i32,
    next_spawn: i32,
    center: BlockPos,
}

impl Default for VillageSiege {
    fn default() -> Self {
        Self {
            active: false,
            setup: false,
            remaining: 0,
            next_spawn: 0,
            center: BlockPos { x: 0, y: 0, z: 0 },
        }
    }
}

impl ServerWorld {
    pub(super) fn tick_village_siege(&mut self, dimension: &LevelId, events: &mut Vec<WorldEvent>) {
        if dimension.builtin() != Some(Java26BuiltinDimension::Overworld)
            || !self.game_rules.spawn_mobs()
        {
            return;
        }
        if let Err(detail) = self.advance_village_siege(dimension, events) {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!("village siege: {detail}"),
            )));
        }
    }

    fn advance_village_siege(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        // VillageSiege.java:35-65: daylight resets setup, while a failed nightly roll does not.
        let bright = self
            .is_bright_outside(dimension)
            .ok_or("missing dimension brightness context")?;
        if bright
            || !self
                .game_rules
                .spawning_monsters(self.admin.settings().difficulty)
        {
            self.cold.village_siege.active = false;
            self.cold.village_siege.setup = false;
            return Ok(());
        }
        if self
            .is_at_dimension_time_marker(dimension, "minecraft:roll_village_siege")
            .ok_or("missing dimension clock context")?
        {
            self.cold.village_siege.active = self.random.next_int(10) == 0;
        }
        if !self.cold.village_siege.active {
            return Ok(());
        }
        if !self.cold.village_siege.setup {
            if !self.setup_village_siege(dimension, events)? {
                return Ok(());
            }
            self.cold.village_siege.setup = true;
        }
        if self.cold.village_siege.next_spawn > 0 {
            self.cold.village_siege.next_spawn -= 1;
            return Ok(());
        }
        self.cold.village_siege.next_spawn = 2;
        if self.cold.village_siege.remaining <= 0 {
            self.cold.village_siege.active = false;
            return Ok(());
        }
        let result = self.try_spawn_siege_zombie(dimension, events);
        self.cold.village_siege.remaining -= 1;
        result
    }

    fn setup_village_siege(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<bool, String> {
        // VillageSiege.java:69-93: only spectators are excluded, and the first eligible player wins.
        let centers: Vec<_> = self.levels[dimension]
            .player_order
            .iter()
            .filter_map(|id| self.levels.player(*id))
            .filter(|player| player.game_type != GameType::Spectator)
            .map(|player| BlockPos {
                x: player.position().x.floor() as i32,
                y: player.position().y.floor() as i32,
                z: player.position().z.floor() as i32,
            })
            .collect();
        for center in centers {
            if !self.siege_is_village(dimension, center)?
                || self.custom_spawn_biome_excluded(
                    dimension,
                    center,
                    "minecraft:without_zombie_sieges",
                )?
            {
                continue;
            }
            for _ in 0..10 {
                let angle = self.random.next_float() * std::f32::consts::TAU;
                self.cold.village_siege.center = BlockPos {
                    x: center
                        .x
                        .wrapping_add((mth_cos(f64::from(angle)) * 32.0).floor() as i32),
                    y: center.y,
                    z: center
                        .z
                        .wrapping_add((mth_sin(f64::from(angle)) * 32.0).floor() as i32),
                };
                if self.find_siege_spawn_position(dimension, events)?.is_some() {
                    self.cold.village_siege.next_spawn = 0;
                    self.cold.village_siege.remaining = 20;
                    break;
                }
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn siege_is_village(&self, dimension: &LevelId, pos: BlockPos) -> Result<bool, String> {
        self.is_close_to_village_id(dimension, [pos.x, pos.y, pos.z], 1)
            .ok_or_else(|| "missing village POI tag context".into())
    }

    fn try_spawn_siege_zombie(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        if let Some(pos) = self.find_siege_spawn_position(dimension, events)? {
            self.spawn_siege_zombie(dimension, pos)
                .ok_or("zombie EVENT finalization or publication failed")?;
        }
        Ok(())
    }

    fn find_siege_spawn_position(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<Option<BlockPos>, String> {
        // VillageSiege.java:113-126: WORLD_SURFACE and Monster rules, without ordinary placement/caps.
        for _ in 0..10 {
            let center = self.cold.village_siege.center;
            let level = self.level_mut_by_id(dimension);
            let x = center
                .x
                .wrapping_add(level.random.next_int(16))
                .wrapping_sub(8);
            let z = center
                .z
                .wrapping_add(level.random.next_int(16))
                .wrapping_sub(8);
            let y =
                self.custom_spawn_height(dimension, x, z, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID)?;
            let pos = BlockPos { x, y, z };
            if !self.siege_is_village(dimension, pos)? {
                continue;
            }
            if !self.flush_level_light(dimension, "village-siege", events) {
                return Err("candidate light could not be committed".into());
            }
            if self
                .siege_spawn_rules(dimension, pos)
                .ok_or("missing Monster EVENT environment")?
            {
                return Ok(Some(pos));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
