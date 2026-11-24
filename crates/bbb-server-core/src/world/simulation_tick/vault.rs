use super::*;
use crate::block_entity::vault::{VaultSound, VaultState};
use crate::entity::block_drops::{loot_tool_for_runtime_stack, RecipeDropFacts};
use crate::fluid::with_property;
use crate::loot_runtime::{
    loot_table_by_id, loot_table_drops, param_sets, LootEntities, LootEntity, LootOrigin,
    LootParams, LootWeather,
};
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use crate::world::LevelId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_LUCK_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::world::loot_context::PlayerLootNbtSources;
use crate::world::simulation_level::mob_death_loot::LevelLootEnvironment;

pub(crate) struct VaultLevel<'a> {
    pub world: &'a mut ServerWorld,
    pub dimension: &'a LevelId,
    pub pos: BlockPos,
    pub events: &'a mut Vec<WorldEvent>,
}

impl ServerWorld {
    pub(super) fn tick_vault(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(old_state) = self.block_state_by_key(dimension, pos) else {
            return;
        };
        let Some(state) = VaultState::from_block_state(old_state) else {
            return;
        };
        let time = self.game_time();
        let cycle_now = time % 20 == 0 && state == VaultState::Active;
        let Some((needs_tick, has_changes)) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::vault)
            .map(|vault| (vault.needs_tick(time), vault.has_pending_changes()))
        else {
            return;
        };
        if !cycle_now && !needs_tick && !has_changes {
            return;
        }
        let result = (|| -> bbb_server_types::Result<()> {
            if cycle_now {
                self.cycle_vault_display(dimension, pos)?;
            }
            if needs_tick {
                let Some(mut entity) = self
                    .level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .take(pos)
                else {
                    return Ok(());
                };
                let result = entity.vault_mut().expect("vault dispatch").tick(
                    state,
                    time,
                    &mut VaultLevel {
                        world: self,
                        dimension,
                        pos,
                        events,
                    },
                );
                self.put_back_block_entity(dimension, entity);
                result?;
                if state != VaultState::Active
                    && self
                        .block_state_by_key(dimension, pos)
                        .and_then(VaultState::from_block_state)
                        == Some(VaultState::Active)
                    && self
                        .level_by_id(dimension)
                        .chunks
                        .block_entities()
                        .get(pos)
                        .and_then(|entity| entity.vault())
                        .is_some_and(|vault| vault.shared_data().display_item().is_empty())
                {
                    self.cycle_vault_display(dimension, pos)?;
                }
            }
            Ok(())
        })();
        let changes = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(|entity| entity.vault_mut())
            .map(|vault| vault.take_changes());
        if let Some(changes) = changes {
            if changes.changed {
                self.level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .set_changed(pos);
            }
            if changes.shared_changed {
                let next_state = self.block_state_by_key(dimension, pos).unwrap_or(old_state);
                self.with_level(dimension, |level| {
                    level.send_block_updated(pos, old_state, next_state);
                });
            }
        }
        if let Err(error) = result {
            tracing::warn!("vault tick: {error}");
        }
    }

    fn cycle_vault_display(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> bbb_server_types::Result<()> {
        let Some(vault) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(|entity| entity.vault())
        else {
            return Ok(());
        };
        let config = vault.config();
        let table = (!config.key_item().is_empty()).then(|| {
            config
                .override_loot_table_to_display()
                .unwrap_or(config.loot_table())
                .to_owned()
        });
        let item = if let Some(table) = table {
            let mut items = self.draw_vault_items(dimension, pos, &table, None)?;
            if items.is_empty() {
                RuntimeItemStack::empty()
            } else {
                let index = self
                    .level_mut_by_id(dimension)
                    .random
                    .next_int(items.len() as i32) as usize;
                items.remove(index)
            }
        } else {
            RuntimeItemStack::empty()
        };
        if let Some(vault) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(|entity| entity.vault_mut())
        {
            vault.set_display(item);
        }
        Ok(())
    }

    pub(in crate::world) fn draw_vault_items(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        table_id: &str,
        player: Option<(ConnectionId, &RuntimeItemStack)>,
    ) -> bbb_server_types::Result<Vec<RuntimeItemStack>> {
        let fail =
            |detail: String| bbb_server_types::ServerWorldError::InvalidConfiguration(detail);
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .ok_or_else(|| fail("no datapack generation".into()))?;
        if loot_table_by_id(&startup.loot, table_id).is_none() {
            return Ok(Vec::new());
        }
        let view = player.and_then(|(id, _)| self.session_loot_view(id));
        let luck = player
            .and_then(|(id, _)| self.levels.player(id))
            .and_then(|player| player.attributes.value(VANILLA_ATTRIBUTE_LUCK_ID))
            .unwrap_or(0.0) as f32;
        let tool = player.map(|(_, item)| {
            loot_tool_for_runtime_stack(item, &self.datapack_tables.enchantment_registry)
        });
        let resolver = self.item_stack_resolver();
        let snapshot = self.entity_nbt_read_snapshot();
        let environment_weather = crate::world::environment_attributes::ScalarWeather::from_level(
            &self.level_by_id(dimension).weather,
        );
        let sky_darkening = self.level_by_id(dimension).sky_darkening.map(i32::from);
        let tags = &self.cold.loot_item_tags;
        let level_names = std::sync::Arc::clone(&self.levels.names);
        let environment_attributes = self.cold.dimension_sky_lights.loot_environment_context(
            self.cold
                .level_dimension_types
                .get(dimension)
                .map(String::as_str),
            &self.registries,
            &self.world_clocks,
            environment_weather,
        );
        let mut entities = Vec::with_capacity(self.levels.len());
        let mut caller = None;
        for (key, level) in self.levels.iter_mut() {
            entities.push((key, &level.entities));
            if key == *dimension {
                caller = Some((
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
        let (dimension_key, chunks, random, weather) =
            caller.ok_or_else(|| fail("vault level is absent".into()))?;
        let chunks = std::cell::RefCell::new(chunks);
        let maps = std::cell::RefCell::new(&mut self.maps);
        let bodies = entities
            .iter()
            .find(|(key, _)| *key == *dimension)
            .map(|(_, store)| crate::entity::store::EntityBodyReadView::new(store, None, None));
        let sources = PlayerLootNbtSources {
            snapshot: Some(&snapshot),
            resolver: Some(&resolver),
            dimension: Some(dimension),
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
            dimension: *dimension,
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

        let params = LootParams {
            dimension: Some(dimension_key),
            origin: Some(LootOrigin::at_center_of(pos)),
            entities: LootEntities {
                this_entity: view.as_ref().map_or(LootEntity::Absent, LootEntity::Facts),
                ..Default::default()
            },
            tool: tool.as_ref(),
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
            ..LootParams::new(param_sets::VAULT)
        };
        let drops = loot_table_drops(&startup.loot, table_id, &params, random)
            .map_err(|error| fail(error.to_string()))?;
        Ok(drops
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect())
    }

    fn play_vault_sound(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        sound: &str,
        pitch: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        self.flush_level_events(dimension, events);
        let packet = SoundEvent {
            sound: SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(sound).expect("built-in vault sound"),
            },
            source: SoundSource::Blocks,
            position: vault_center(pos),
            volume: 1.0,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(dimension, &packet, None, events);
    }
}

impl VaultLevel<'_> {
    pub(crate) fn connected_players(&mut self, range: f64) -> bbb_server_types::Result<Vec<Uuid>> {
        Ok(self
            .world
            .level_by_id(self.dimension)
            .player_order
            .iter()
            .filter_map(|id| self.world.levels.player(*id))
            .filter(|player| {
                &player.dimension == self.dimension
                    && !player.avatar_removed
                    && player.game_type != GameType::Spectator
                    && {
                        let at = BlockPos {
                            x: player.position().x.floor() as i32,
                            y: player.position().y.floor() as i32,
                            z: player.position().z.floor() as i32,
                        };
                        let distance = f64::from(at.x - self.pos.x).powi(2)
                            + f64::from(at.y - self.pos.y).powi(2)
                            + f64::from(at.z - self.pos.z).powi(2);
                        distance < range * range
                    }
            })
            .map(|player| player.profile.uuid)
            .collect())
    }

    pub(crate) fn set_state(
        &mut self,
        _from: VaultState,
        to: VaultState,
    ) -> bbb_server_types::Result<()> {
        let state = self
            .world
            .block_state_by_key(self.dimension, self.pos)
            .and_then(|state| with_property(state, "vault_state", to.serialized_name()))
            .ok_or_else(|| {
                bbb_server_types::ServerWorldError::InvalidConfiguration(
                    "vault state unavailable".into(),
                )
            })?;
        self.world
            .set_block(self.dimension, self.pos, state, SET_BLOCK_UPDATE_ALL);
        Ok(())
    }

    pub(crate) fn eject_item(
        &mut self,
        item: RuntimeItemStack,
        progress: f32,
    ) -> bbb_server_types::Result<()> {
        self.world.spawn_dispensed_stack(
            self.dimension,
            Vec3d {
                x: f64::from(self.pos.x) + 0.5,
                y: f64::from(self.pos.y) + 1.2,
                z: f64::from(self.pos.z) + 0.5,
            },
            Direction::Up,
            2,
            &item,
        )?;
        self.world
            .apply_level_entity_lifecycle_by_key(self.dimension, self.events);
        self.level_event(3017);
        self.world.play_vault_sound(
            self.dimension,
            self.pos,
            "minecraft:block.vault.eject_item",
            0.8 + 0.4 * progress,
            self.events,
        );
        Ok(())
    }

    pub(crate) fn sound(&mut self, sound: VaultSound) {
        let sound = match sound {
            VaultSound::Insert => "minecraft:block.vault.insert_item",
            VaultSound::InsertFail => "minecraft:block.vault.insert_item_fail",
            VaultSound::RejectRewardedPlayer => "minecraft:block.vault.reject_rewarded_player",
            VaultSound::OpenShutter => "minecraft:block.vault.open_shutter",
            VaultSound::CloseShutter => "minecraft:block.vault.close_shutter",
        };
        self.world
            .play_vault_sound(self.dimension, self.pos, sound, 1.0, self.events);
    }

    pub(crate) fn level_event(&mut self, id: i32) {
        let ominous = self
            .world
            .block_state_by_key(self.dimension, self.pos)
            .and_then(bbb_protocol::block_state_types::vanilla_block_state_identity)
            .is_some_and(|state| {
                state
                    .properties
                    .get("ominous")
                    .is_some_and(|value| value == "true")
            });
        self.world
            .level_mut_by_id(self.dimension)
            .event_sink
            .level_event(
                id,
                self.pos,
                if matches!(id, 3015 | 3016) {
                    i32::from(ominous)
                } else {
                    0
                },
            );
        self.world.flush_level_events(self.dimension, self.events);
    }
}

fn vault_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}
