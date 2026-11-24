use crate::block_behavior::brushable::{schedule_self, with_dusted};
use crate::block_entity::brushable::{BrushStroke, BrushableBlockEntity};
use crate::block_entity::BlockEntity;
use crate::connection::ConnectionId;
use crate::entity::block_drops::{loot_tool_for_runtime_stack, RecipeDropFacts};
use crate::entity::projectile_hit::aabb_clip;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::PendingLevelEvent;
use crate::loot_runtime::{
    loot_table_by_id, loot_table_drops, param_sets, LootEntities, LootEntity, LootOrigin,
    LootParams, LootStack, LootWeather,
};
use crate::random::GameRandom;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use crate::world::LevelId;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID, VANILLA_ATTRIBUTE_LUCK_ID,
};
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_GRAVEL_ID, VANILLA_BLOCK_SAND_ID, VANILLA_BLOCK_SUSPICIOUS_GRAVEL_ID,
    VANILLA_BLOCK_SUSPICIOUS_SAND_ID,
};
use bbb_protocol::entity_types::{vanilla_entity_type_dimensions, VANILLA_ENTITY_TYPE_ITEM_ID};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{BlockPos, Direction, InteractionHand, ItemStackSummary, Vec3d};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::directions::relative;
use bbb_server_types::geometry::{Aabb, Vec3Ops};
use bbb_server_types::interaction::player_pose_bounding_box;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::super::advancement_triggers::namespaced_loot_table;
use super::super::pov_clip::{clip_player, view_vector, ClipFluid, ClipShape};
use super::super::simulation_level::mob_death_loot::LevelLootEnvironment;
use super::super::{ServerWorld, WorldEvent};
use super::{use_item_class, UseItemClass, BRUSH_USE_DURATION};
use bbb_server_types::geometry::clip::ClipHit;

/// `BrushItem.ANIMATION_DURATION` (`BrushItem.java:30`).
const ANIMATION_DURATION: i32 = 10;
/// `LevelEvent.PARTICLES_AND_SOUND_BRUSH_BLOCK_COMPLETE` (`LevelEvent.java:73`).
const LEVEL_EVENT_BRUSH_BLOCK_COMPLETE: i32 = 3008;
/// `SoundEvents.BRUSH_GENERIC` (`SoundEvents.java:236`).
const BRUSH_GENERIC_SOUND: &str = "minecraft:item.brush.brushing.generic";

impl ServerWorld {
    pub(in crate::world) fn use_brush_on(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        stack: &ItemStackSummary,
    ) -> bool {
        if use_item_class(stack) != UseItemClass::Brush {
            return false;
        }
        let Some(player) = self.levels.player(connection_id) else {
            return false;
        };
        match player.cold.cooldowns.is_on_cooldown(stack) {
            Ok(true) => return false,
            Ok(false) => {}
            Err(error) => {
                tracing::warn!("brush cooldown: {error}");
                return false;
            }
        }
        if self.brush_hit(connection_id).is_some() {
            self.start_using_item(connection_id, hand);
        }
        if let Some(item_id) = stack.item_id {
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
        true
    }

    pub(super) fn brush_on_use_tick(
        &mut self,
        connection_id: ConnectionId,
        held: &bbb_server_types::item_stack::RuntimeItemStack,
        remaining: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let hit = if remaining >= 0 {
            self.brush_hit(connection_id)
        } else {
            None
        };
        let dimension = self.player_level_id(connection_id);
        let Some(hit) = hit else {
            self.release_using_item(connection_id, events);
            return;
        };
        let time_elapsed = BRUSH_USE_DURATION - remaining + 1;
        if time_elapsed % ANIMATION_DURATION != 5 {
            return;
        }
        let pos = hit.pos;
        let state = self.block_state_or_air(dimension, pos);
        let sound = brushable_block(state).map_or(BRUSH_GENERIC_SOUND, |block| block.brush_sound);
        self.play_block_sound_as(
            dimension,
            pos,
            sound,
            (1.0, 1.0),
            Some(connection_id),
            events,
        );
        let brushable = self
            .get_block_entity(dimension, pos)
            .and_then(BlockEntity::brushable)
            .is_some();
        if brushable && self.brush_block(connection_id, dimension, pos, hit.direction, held, events)
        {
            // The brush ticking here is the stack in the used hand.
            let hand = self.levels[connection_id].using_item.used_item_hand();
            self.hurt_and_break_in_hand(connection_id, hand, 1, events);
        }
    }

    fn brush_hit(&self, connection_id: ConnectionId) -> Option<ClipHit> {
        let player = self.levels.player(connection_id)?;
        let from = player.eye_position().ok()?;
        let range = player
            .attributes
            .value(VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID)?;
        let delta = view_vector(player.rotation().x_rot, player.rotation().y_rot).scale(range);
        let level = self.level_by_id(&player.dimension);
        let hit = clip_player(
            &level.chunks,
            player,
            from,
            from.plus(delta),
            ClipFluid::None,
            ClipShape::Collider,
        )?;
        let area = player_pose_bounding_box(player.position(), player.pose_id, player.pose_scale())
            .ok()?
            .expand_towards(delta)
            .inflate(1.0, 1.0, 1.0);
        let in_front = |bounds: Aabb| {
            bounds.intersects(area) && aabb_clip(bounds, from, hit.location).is_some()
        };
        let entity_in_front = level
            .entities
            .ids_in_box_sections(area)
            .into_iter()
            .filter_map(|id| level.entities.get(id))
            .any(|entity| entity.is_pickable() && in_front(entity.bounding_box()));
        let player_in_front = self.levels.player_entries().any(|(other_id, other)| {
            *other_id != connection_id
                && other.dimension == player.dimension
                && !other.avatar_removed
                && other.game_type != GameType::Spectator
                && player_pose_bounding_box(other.position(), other.pose_id, other.pose_scale())
                    .is_ok_and(&in_front)
        });
        (!entity_in_front && !player_in_front).then_some(hit)
    }

    fn brush_block(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        pos: BlockPos,
        direction: Direction,
        brush: &bbb_server_types::item_stack::RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let game_time = self.game_time();
        let proceeds = self
            .brushable_mut(dimension, pos)
            .is_some_and(|brushable| brushable.begin_brush(game_time, direction));
        if !proceeds || !self.unpack_brushable_loot(connection_id, dimension, pos, brush, events) {
            return false;
        }
        let Some(stroke) = self
            .brushable_mut(dimension, pos)
            .map(BrushableBlockEntity::stroke)
        else {
            return false;
        };
        match stroke {
            BrushStroke::Completed => match self.brushing_completed(dimension, pos, events) {
                Ok(()) => true,
                Err(error) => {
                    tracing::warn!("brushed item completion: {error}");
                    false
                }
            },
            BrushStroke::Dusted(dusted) => {
                let state = self.block_state_or_air(dimension, pos);
                self.with_level(dimension, |level| {
                    schedule_self(
                        &mut crate::block_behavior::shape_context::BlockShapeCtx::Live(level),
                        pos,
                        state,
                    )
                });
                if let Some(state) = dusted.and_then(|dusted| with_dusted(state, dusted)) {
                    self.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL);
                }
                false
            }
        }
    }

    ///
    /// A draw this server cannot finish is reported and the stroke dropped
    /// rather than counted, so a completed block never loses its loot.
    fn unpack_brushable_loot(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        pos: BlockPos,
        brush: &bbb_server_types::item_stack::RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some((table_id, seed)) = self
            .get_block_entity(dimension, pos)
            .and_then(BlockEntity::brushable)
            .and_then(|brushable| match brushable.loot_table()? {
                NbtSummaryValue::String(id) => Some((
                    id.with_utf8(namespaced_loot_table)?,
                    brushable.loot_table_seed(),
                )),
                _ => None,
            })
        else {
            return true;
        };
        self.trigger_generate_loot(connection_id, &table_id, events);
        let current_brush = brush.clone();
        let drawn = match self.archaeology_loot(
            connection_id,
            dimension,
            pos,
            &current_brush,
            &table_id,
            seed,
        ) {
            Ok(drawn) => drawn,
            Err(detail) => {
                tracing::warn!("loot table {table_id}: {detail}");
                return false;
            }
        };
        // Extra stacks are logged and dropped (`:103-110`).
        let item = drawn
            .into_iter()
            .next()
            .map(RuntimeItemStack::from_summary)
            .unwrap_or_default();
        if let Some(brushable) = self.brushable_mut(dimension, pos) {
            brushable.set_item(item);
            self.level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
        }
        true
    }

    fn archaeology_loot(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        pos: BlockPos,
        brush: &bbb_server_types::item_stack::RuntimeItemStack,
        table_id: &str,
        seed: i64,
    ) -> Result<Vec<LootStack>, String> {
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .ok_or_else(|| "no datapack generation".to_owned())?;
        // `ReloadableServerRegistries.getLootTable` answers the empty table for
        // an unknown key.
        if loot_table_by_id(&startup.loot, table_id).is_none() {
            return Ok(Vec::new());
        }
        let resolver = self.item_stack_resolver();
        let snapshot = self.entity_nbt_read_snapshot();
        let view = self.loot_view_for_player(self.levels.player_at(connection_id));
        let environment_weather = crate::world::environment_attributes::ScalarWeather::from_level(
            &self.level_by_id(&dimension).weather,
        );
        let sky_darkening = self.level_by_id(&dimension).sky_darkening.map(i32::from);
        let tags = &self.cold.loot_item_tags;
        let player = self.levels.player_at(connection_id);
        let luck = player
            .attributes
            .value(VANILLA_ATTRIBUTE_LUCK_ID)
            .unwrap_or(0.0) as f32;
        let tool = loot_tool_for_runtime_stack(brush, &self.datapack_tables.enchantment_registry);
        let key = dimension;
        let level_names = std::sync::Arc::clone(&self.levels.names);
        let environment_attributes = self.cold.dimension_sky_lights.loot_environment_context(
            self.cold
                .level_dimension_types
                .get(&key)
                .map(String::as_str),
            &self.registries,
            &self.world_clocks,
            environment_weather,
        );
        let mut entities = Vec::with_capacity(self.levels.len());
        let mut brushing_level = None;
        for (level_key, level) in self.levels.iter_mut() {
            entities.push((level_key, &level.entities));
            if level_key == key {
                brushing_level = Some((
                    level.dimension.as_str(),
                    &mut level.chunks,
                    &mut level.random,
                    LootWeather {
                        raining: level.weather.is_raining(),
                        thundering: level.weather.is_thundering(),
                    },
                ));
            }
        }
        let (dimension_key, chunks, world_random, weather) =
            brushing_level.ok_or_else(|| "the brushing session's level is absent".to_owned())?;
        let chunks = std::cell::RefCell::new(chunks);
        let maps = std::cell::RefCell::new(&mut self.maps);
        let bodies = entities
            .iter()
            .find(|(dimension, _)| *dimension == key)
            .map(|(_, store)| crate::entity::store::EntityBodyReadView::new(store, None, None));
        let sources = super::super::loot_context::PlayerLootNbtSources {
            snapshot: Some(&snapshot),
            resolver: Some(&resolver),
            dimension: Some(&key),
            bodies,
            other_levels: crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(
                &entities,
            ),
        };
        let player_slots = |uuid, slot| sources.slot(uuid, slot);
        let player_nbt = |uuid| sources.read(uuid);
        let scoreboard = crate::world::loot_context::entity_scoreboard::LootEntityScoreboard {
            scoreboard: self.scoreboard.board(),
            names: &level_names,
            dimension: key,
            bodies,
            other_levels: crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(
                &entities,
            ),
        };
        let environment = LevelLootEnvironment::new(
            RecipeDropFacts {
                recipes: &startup.recipes,
                recipe_index: &startup.recipe_index,
                item_tags: tags,
            },
            LootChunks::Chunks(&chunks),
            Some(&startup),
            Some(&self.datapack_tables.enchantment_registry),
            Some(&resolver),
        )
        .with_position_queries(&self.registries, self.cold.seed, sky_darkening)
        .with_environment_attributes(environment_attributes)
        .with_clocks(&self.world_clocks)
        .with_maps(
            &maps,
            self.cold.seed,
            dimension_key,
            self.cold.generate_structures,
        )
        .with_nbt_sources(snapshot.clone(), &self.cold.command_storage)
        .with_player_nbt(&player_nbt)
        .with_player_slots(&player_slots)
        .with_entity_scoreboard(scoreboard)
        .with_advancement_tree(&self.cold.advancement_tree);

        let centre = centre_of(pos);
        let params = LootParams {
            dimension: Some(dimension_key),
            origin: Some(LootOrigin {
                x: centre.x,
                y: centre.y,
                z: centre.z,
            }),
            entities: LootEntities {
                this_entity: LootEntity::Facts(&view),
                ..Default::default()
            },
            tool: Some(&tool),
            luck,
            weather: Some(weather),
            level: Some(&|pos| {
                LoadedBlocks {
                    chunks: &chunks.borrow(),
                }
                .loaded_block_state(pos)
            }),
            block_entity_data: Some(&|pos| {
                chunks
                    .borrow()
                    .block_entity_loot_data(pos, Some(&resolver))
                    .map_err(|error| {
                        crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                    })
            }),
            item_environment: Some(&environment),
            ..LootParams::new(param_sets::ARCHAEOLOGY)
        };
        let drawn = if seed != 0 {
            loot_table_drops(
                &startup.loot,
                table_id,
                &params,
                &GameRandom::with_seed(seed),
            )
        } else {
            loot_table_drops(&startup.loot, table_id, &params, world_random)
        };
        drawn.map_err(|error| error.to_string())
    }

    fn brushing_completed(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<()> {
        let Some((item, hit_direction)) = self
            .get_block_entity(dimension, pos)
            .and_then(BlockEntity::brushable)
            .map(|brushable| (brushable.item().clone(), brushable.hit_direction()))
        else {
            return Ok(());
        };
        if !item.is_empty() {
            self.drop_brushed_item(dimension, pos, item, hit_direction.unwrap_or(Direction::Up))?;
        }
        let state = self.block_state_or_air(dimension, pos);
        let event = PendingLevelEvent {
            event_type: LEVEL_EVENT_BRUSH_BLOCK_COMPLETE,
            pos,
            data: state,
        };
        self.broadcast_level_event_by_key(&dimension, None, event, events);
        let turns_into = brushable_block(state)
            .and_then(|block| vanilla_block_default_state_id(block.turns_into))
            .unwrap_or_else(|| self.level_by_id(&dimension).chunks.air_state_id());
        self.set_block(dimension, pos, turns_into, SET_BLOCK_UPDATE_ALL);
        Ok(())
    }

    fn drop_brushed_item(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        mut item: RuntimeItemStack,
        direction: Direction,
    ) -> bbb_server_types::Result<()> {
        let [width, height] = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_ITEM_ID).ok_or(
            bbb_server_types::ServerWorldError::MissingCatalogFact("item dimensions"),
        )?;
        let size = f64::from(width);
        let centre_range = 1.0 - size;
        let half_size = size / 2.0;
        let drop_pos = relative(pos, direction);
        let position = Vec3d {
            x: f64::from(drop_pos.x) + 0.5 * centre_range + half_size,
            y: f64::from(drop_pos.y) + 0.5 + f64::from(height / 2.0),
            z: f64::from(drop_pos.z) + 0.5 * centre_range + half_size,
        };
        let count = self.random.next_int(21) + 10;
        let split = item.split(count);
        if let Some(brushable) = self.brushable_mut(dimension, pos) {
            brushable.set_item(item);
        }
        let entities = &mut self.level_mut_by_id(&dimension).entities;
        if let Some(id) = crate::entity::spawn_item_entity(entities, position, split) {
            if let Some(entity) = entities.get_mut(id) {
                entity.set_delta_movement(Vec3d::default());
            }
        } else {
            tracing::warn!("brushed item: the entity store refused the drawn item UUID");
        }
        if let Some(brushable) = self.brushable_mut(dimension, pos) {
            brushable.take_item();
        }
        Ok(())
    }

    fn brushable_mut(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
    ) -> Option<&mut BrushableBlockEntity> {
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::brushable_mut)
    }
}

/// `Vec3.atCenterOf(pos)`.
fn centre_of(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

struct BrushableBlock {
    turns_into: i32,
    brush_sound: &'static str,
}

fn brushable_block(block_state: i32) -> Option<BrushableBlock> {
    match vanilla_block_state_block_type_id(block_state)? {
        VANILLA_BLOCK_SUSPICIOUS_SAND_ID => Some(BrushableBlock {
            turns_into: VANILLA_BLOCK_SAND_ID,
            brush_sound: "minecraft:item.brush.brushing.sand",
        }),
        VANILLA_BLOCK_SUSPICIOUS_GRAVEL_ID => Some(BrushableBlock {
            turns_into: VANILLA_BLOCK_GRAVEL_ID,
            brush_sound: "minecraft:item.brush.brushing.gravel",
        }),
        _ => None,
    }
}
