use crate::loot_runtime::{
    loot_table_by_id, loot_table_drops, param_sets, LootEntities, LootEntity, LootOrigin,
    LootParams,
};
use crate::random::GameRandom;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use bbb_protocol::packets::{PlayerRotation, SoundEvent, SoundEventHolder, SoundSource, Vec3d};
use bbb_server_types::geometry::{mth_cos, mth_sin};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::simulation_level::mob_death_loot::LevelLootEnvironment;
use super::ConnectionId;
use super::{ServerWorld, WorldEvent};

impl ServerWorld {
    pub(super) fn grant_advancement_loot(
        &mut self,
        connection_id: ConnectionId,
        loot_tables: &[String],
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        if loot_tables.is_empty() {
            return Ok(());
        }
        let mut changes = false;
        let result =
            self.grant_advancement_loot_inner(connection_id, loot_tables, events, &mut changes);
        if changes {
            self.broadcast_player_menu(
                connection_id,
                crate::inventory::MenuBroadcastMode::Changes,
                events,
            );
        }
        result
    }

    fn grant_advancement_loot_inner(
        &mut self,
        connection_id: ConnectionId,
        loot_tables: &[String],
        events: &mut Vec<WorldEvent>,
        changes: &mut bool,
    ) -> Result<(), String> {
        let resolver = self.item_stack_resolver();
        let startup = self.cold.config.datapacks.clone().ok_or_else(|| {
            format!(
                "loot table {} registry: no datapack generation",
                loot_tables[0]
            )
        })?;
        let player = self
            .levels
            .player(connection_id)
            .ok_or_else(|| format!("loot table {} player: connection is absent", loot_tables[0]))?;
        let dimension = player.dimension;
        let position = player.position();
        for table_id in loot_tables {
            if loot_table_by_id(&startup.loot, table_id).is_none() {
                continue;
            }
            let stacks = {
                let player_view = self.loot_view_for_player(
                    self.levels
                        .player(connection_id)
                        .expect("held advancement reward body"),
                );
                let reward_player = self
                    .levels
                    .player(connection_id)
                    .expect("held advancement reward body");
                let reward_uuid = reward_player.profile.uuid;
                let reward_nbt = self
                    .command_player_data(reward_player, &resolver)
                    .map_err(|error| error.to_string());
                let (level, tables, facts, _) = self.level_and_drop_tables(&dimension);
                let chunks = std::cell::RefCell::new(&mut level.chunks);

                let environment = LevelLootEnvironment::for_level(
                    tables.recipes,
                    LootChunks::Chunks(&chunks),
                    &facts,
                    Some(crate::entity::store::EntityBodyReadView::new(
                        &level.entities,
                        None,
                        None,
                    )),
                );
                let sources = crate::world::loot_context::PlayerLootNbtSources {
                    snapshot: facts.entity_nbt.as_ref(),
                    resolver: facts.item_stack_resolver.as_ref(),
                    dimension: facts.dimension.as_ref(),
                    bodies: Some(crate::entity::store::EntityBodyReadView::new(
                        &level.entities,
                        None,
                        None,
                    )),
                    other_levels:
                        facts.other_levels.map_or(
                            crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(
                                &[],
                            ),
                            crate::world::loot_context::entity_scoreboard::LootEntityLevels::Other,
                        ),
                };
                let player_slots = |uuid, slot| sources.slot(uuid, slot);
                let player_nbt = |uuid| {
                    if uuid == reward_uuid {
                        reward_nbt
                            .clone()
                            .map_err(crate::loot_runtime::LootRuntimeReason::Catalog)
                    } else {
                        sources.read(uuid)
                    }
                };
                let environment = environment.map(|environment| {
                    environment
                        .with_player_nbt(&player_nbt)
                        .with_player_slots(&player_slots)
                });
                let params = LootParams {
                    dimension: facts.dimension_key.as_deref().map(|key| key.as_str()),
                    origin: Some(LootOrigin {
                        x: position.x,
                        y: position.y,
                        z: position.z,
                    }),
                    entities: LootEntities {
                        this_entity: LootEntity::Facts(&player_view),
                        ..Default::default()
                    },
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
                    item_environment: environment.as_ref().map(|environment| {
                        environment as &dyn crate::loot_runtime::LootItemEnvironment
                    }),
                    weather: Some(crate::loot_runtime::LootWeather {
                        raining: level.weather.is_raining(),
                        thundering: level.weather.is_thundering(),
                    }),
                    ..LootParams::new(param_sets::ADVANCEMENT_REWARD)
                };
                loot_table_drops(&startup.loot, table_id, &params, &level.random)
                    .map_err(|error| error.to_string())?
            };
            for (index, stack) in stacks.into_iter().enumerate() {
                let path = format!("loot table {table_id} stack[{index}]");
                let mut stack = RuntimeItemStack::from_summary(stack);
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("grant retains its player");
                let creative = player.abilities.instabuild;
                let added = player
                    .inventory
                    .add_anywhere(&resolver, &mut stack, creative)
                    .map_err(|error| format!("{path} inventory: {error}"))?;
                if added {
                    *changes = true;
                    let pitch =
                        ((self.random.next_float() - self.random.next_float()) * 0.7 + 1.0) * 2.0;
                    let registry_id =
                        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                            "minecraft:entity.item.pickup",
                        )
                        .ok_or_else(|| format!("{path} sound: pickup registry entry is absent"))?;
                    let sound = SoundEvent {
                        sound: SoundEventHolder::Reference { registry_id },
                        source: SoundSource::Players,
                        position,
                        volume: 0.2,
                        pitch,
                        seed: self.random.next_long(),
                    };
                    self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
                } else if !stack.is_empty() {
                    self.drop_advancement_loot(connection_id, stack, &path)?;
                }
            }
        }
        Ok(())
    }

    fn drop_advancement_loot(
        &mut self,
        connection_id: ConnectionId,
        stack: RuntimeItemStack,
        path: &str,
    ) -> Result<(), String> {
        let item = stack;
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("grant retains its player");
        let eye = player
            .eye_position()
            .map_err(|error| format!("{path} drop position: {error}"))?;
        let position = Vec3d {
            x: eye.x,
            y: eye.y - f64::from(0.3_f32),
            z: eye.z,
        };
        let velocity = reward_drop_velocity(player.rotation(), &self.random);
        let target = player.profile.uuid;
        let dimension = player.dimension;
        crate::entity::item::spawn_advancement_reward_item(
            &mut self.level_mut_by_id(&dimension).entities,
            position,
            item,
            velocity,
            target,
        )
        .ok_or_else(|| format!("{path} entity spawn: item UUID was refused"))?;
        Ok(())
    }
}

pub(super) fn reward_drop_velocity(rotation: PlayerRotation, random: &GameRandom) -> Vec3d {
    let radians = (std::f64::consts::PI / 180.0) as f32;
    let sin_x = mth_sin(f64::from(rotation.x_rot * radians));
    let cos_x = mth_cos(f64::from(rotation.x_rot * radians));
    let sin_y = mth_sin(f64::from(rotation.y_rot * radians));
    let cos_y = mth_cos(f64::from(rotation.y_rot * radians));
    let dir = random.next_float() * (std::f64::consts::PI * 2.0) as f32;
    let power = 0.02_f32 * random.next_float();
    Vec3d {
        x: f64::from(-sin_y * cos_x * 0.3_f32) + f64::from(dir).cos() * f64::from(power),
        y: f64::from(
            -sin_x * 0.3_f32 + 0.1_f32 + (random.next_float() - random.next_float()) * 0.1_f32,
        ),
        z: f64::from(cos_y * cos_x * 0.3_f32) + f64::from(dir).sin() * f64::from(power),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reward_drop_uses_four_player_draws_and_player_look_direction() {
        let random = GameRandom::with_seed(123);
        let velocity = reward_drop_velocity(
            PlayerRotation {
                y_rot: 0.0,
                x_rot: 0.0,
            },
            &random,
        );
        assert!(velocity.x.abs() <= 0.02);
        assert!((0.0..=0.2).contains(&velocity.y));
        assert!((0.28..=0.32).contains(&velocity.z));

        let downward = reward_drop_velocity(
            PlayerRotation {
                y_rot: 0.0,
                x_rot: 90.0,
            },
            &GameRandom::with_seed(123),
        );
        assert!(downward.x.abs() <= 0.02);
        assert!(downward.z.abs() <= 0.02);
        assert!((-0.3..=-0.1).contains(&downward.y));
    }
}

#[cfg(test)]
#[path = "advancement_loot/tests.rs"]
mod integration_tests;
