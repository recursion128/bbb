use super::{ServerWorld, WorldEvent};
use crate::chunk::poi::PoiOccupancy;
use crate::entity::minecart_spawner::spawn_placement;
use crate::entity::{is_spawn_position_ok, SpawnHeightmap, SpawnPlacementProbe};
use crate::random::GameRandom;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::RichTextComponent;
use bbb_server_persistence::Java26WanderingTraderData;
use bbb_server_types::block_queries::collision_shape_empty_at;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};

const RULE: &str = "minecraft:spawn_wandering_traders";

pub(super) struct WanderingTraderSpawner {
    pub(super) data: Option<Java26WanderingTraderData>,
    tick_delay: i32,
}

impl Default for WanderingTraderSpawner {
    fn default() -> Self {
        // tickDelay is not saved.
        Self {
            data: None,
            tick_delay: 1_200,
        }
    }
}

impl WanderingTraderSpawner {
    fn poll(&mut self, random: &GameRandom) -> bool {
        // WanderingTraderSpawner.java:47-57. Roll against the old chance after storing the next.
        self.tick_delay -= 1;
        if self.tick_delay > 0 {
            return false;
        }
        self.tick_delay = 1_200;
        let data = self.data.get_or_insert_default();
        data.spawn_delay = data.spawn_delay.wrapping_sub(1_200);
        if data.spawn_delay > 0 {
            return false;
        }
        data.spawn_delay = 24_000;
        let chance = data.spawn_chance;
        data.spawn_chance = chance.wrapping_add(25).clamp(25, 75);
        random.next_int(100) <= chance
    }
}

impl ServerWorld {
    pub(super) fn tick_wandering_trader(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        if dimension.builtin() != Some(Java26BuiltinDimension::Overworld)
            || !self.game_rules.spawn_mobs()
            || self.admin.game_rule_value(RULE) != Some(GameRuleValue::Boolean(true))
            || !self.cold.wandering_trader.poll(&self.random)
        {
            return;
        }
        match self.try_spawn_wandering_trader(dimension, events) {
            Ok(true) => {
                self.cold
                    .wandering_trader
                    .data
                    .as_mut()
                    .unwrap()
                    .spawn_chance = 25
            }
            Ok(false) => {}
            Err(detail) => events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!("wandering trader spawn: {detail}"),
            ))),
        }
    }

    fn try_spawn_wandering_trader(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<bool, String> {
        // ServerLevel.java:939-942: alive spectators participate, in the level's player order.
        let positions: Vec<_> = self.levels[dimension]
            .player_order
            .iter()
            .filter_map(|id| self.levels.player(*id))
            .filter(|player| !player.avatar_removed && !player.damage.is_dead_or_dying())
            .map(|player| player.position())
            .collect();
        if positions.is_empty() {
            return Ok(true);
        }
        let index = self.random.next_int(positions.len() as i32);
        if self.random.next_int(10) != 0 {
            return Ok(false);
        }
        let player = positions[index as usize];
        let center = [
            player.x.floor() as i32,
            player.y.floor() as i32,
            player.z.floor() as i32,
        ];
        let reference = self
            .level_mut_by_id(dimension)
            .chunks
            .poi_mut()
            .find_type_in_range("minecraft:meeting", center, 48, PoiOccupancy::Any)
            .map_or(center, |record| record.pos);
        let reference = BlockPos {
            x: reference[0],
            y: reference[1],
            z: reference[2],
        };
        let Some(position) = self.find_trader_spawn_position(dimension, reference, 48)? else {
            return Ok(false);
        };
        if !self.trader_spawn_has_space(dimension, position)?
            || self.custom_spawn_biome_excluded(
                dimension,
                position,
                "minecraft:without_wandering_trader_spawns",
            )?
        {
            return Ok(false);
        }
        let Some(trader) =
            self.spawn_trader_event(dimension, "minecraft:wandering_trader", position)
        else {
            return Err("trader EVENT finalization or publication failed".into());
        };
        // WanderingTraderSpawner.java:96-105: each llama is optional; only then initialize the trader.
        let mut failure = None;
        for _ in 0..2 {
            match self.find_trader_spawn_position(dimension, position, 4) {
                Ok(Some(pos)) => {
                    match self.spawn_trader_event(dimension, "minecraft:trader_llama", pos) {
                        Some(llama) => {
                            if !self
                                .level_mut_by_id(dimension)
                                .entities
                                .leash_entity_to(llama, trader, true)
                            {
                                failure = Some("trader llama leash publication failed".to_owned());
                            }
                        }
                        None => {
                            failure = Some(
                                "trader llama EVENT finalization or publication failed".to_owned(),
                            )
                        }
                    }
                }
                Ok(None) => {}
                Err(detail) => failure = Some(detail),
            }
        }
        let entity = self
            .level_mut_by_id(dimension)
            .entities
            .get_mut(trader)
            .unwrap();
        entity.set_trader_despawn_delay(48_000);
        entity.set_wandering_trader_target(Some(reference));
        entity.set_mob_home(reference, 16);
        if let Some(detail) = failure {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!("wandering trader spawn: {detail}"),
            )));
        }
        Ok(true)
    }

    fn find_trader_spawn_position(
        &mut self,
        dimension: &LevelId,
        reference: BlockPos,
        radius: i32,
    ) -> Result<Option<BlockPos>, String> {
        let entry = spawn_placement("minecraft:wandering_trader");
        let heightmap = match entry.heightmap {
            SpawnHeightmap::MotionBlocking => VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            SpawnHeightmap::MotionBlockingNoLeaves => {
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID
            }
        };
        // WanderingTraderSpawner.java:122-137: llama candidates use the trader placement too.
        for _ in 0..10 {
            let x = reference
                .x
                .wrapping_add(self.random.next_int(radius * 2))
                .wrapping_sub(radius);
            let z = reference
                .z
                .wrapping_add(self.random.next_int(radius * 2))
                .wrapping_sub(radius);
            let y = self.custom_spawn_height(dimension, x, z, heightmap)?;
            let game_time = self.game_time;
            let level = self.level_mut_by_id(dimension);
            let position = BlockPos { x, y, z };
            let mut environment = super::spawner_minecarts::spawn_environment(
                &level.chunks,
                SpawnPlacementProbe {
                    entity_type_id: VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
                    position,
                    entry,
                },
            );
            environment.inside_world_border =
                level.world_border.contains_block_at(position, game_time);
            if is_spawn_position_ok(entry.placement, environment) {
                return Ok(Some(position));
            }
        }
        Ok(None)
    }

    fn trader_spawn_has_space(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> Result<bool, String> {
        // WanderingTraderSpawner.java:140-147: test collision shapes in the inclusive 2x3x2 box.
        for z in pos.z..=pos.z + 1 {
            for y in pos.y..=pos.y + 2 {
                for x in pos.x..=pos.x + 1 {
                    if !self.level_by_id(dimension).chunks.is_inside_build_height(y) {
                        continue;
                    }
                    self.install_full_chunks_by_key(dimension, &[(x >> 4, z >> 4)])
                        .map_err(|error| error.to_string())?;
                    let chunks = &self.level_by_id(dimension).chunks;
                    let state = chunks
                        .get_block_state(x, y, z)
                        .ok_or("trader space probe has no block state")?;
                    if !collision_shape_empty_at(state, BlockPos { x, y, z }) {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    pub(super) fn custom_spawn_has_chunks_at(
        &self,
        dimension: &LevelId,
        position: BlockPos,
    ) -> bool {
        let chunks = &self.levels[dimension].chunks;
        (position.x.wrapping_sub(10) >> 4..=position.x.wrapping_add(10) >> 4).all(|x| {
            (position.z.wrapping_sub(10) >> 4..=position.z.wrapping_add(10) >> 4)
                .all(|z| chunks.is_ticking_full(x, z))
        })
    }

    pub(super) fn custom_spawn_height(
        &self,
        dimension: &LevelId,
        x: i32,
        z: i32,
        heightmap: i32,
    ) -> Result<i32, String> {
        super::level_height(&self.levels[dimension].chunks, heightmap, x, z)
            .ok_or_else(|| "custom-spawn candidate has no FULL heightmap".into())
    }

    pub(super) fn custom_spawn_biome_excluded(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
        tag: &str,
    ) -> Result<bool, String> {
        let (x, y, z) = fuzzy_biome_quart_position(
            biome_zoom_seed(self.cold.seed),
            position.x,
            position.y,
            position.z,
        );
        self.install_full_chunks_by_key(dimension, &[(x >> 2, z >> 2)])
            .map_err(|error| error.to_string())?;
        let level = self.level_by_id(dimension);
        let biome = level
            .chunks
            .biome_id_at_quart(x, y, z)
            .ok_or("custom-spawn candidate has no biome")?;
        let id = self
            .registries
            .registry_element_id("minecraft:worldgen/biome", biome)
            .ok_or("custom-spawn biome holder is absent")?;
        let tags = self.levels[dimension]
            .chunks
            .registry_tags()
            .ok_or("custom-spawn biome tags are absent")?;
        Ok(tags.contains("minecraft:worldgen/biome", tag, id))
    }
}

#[cfg(test)]
mod tests;
