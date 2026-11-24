use super::*;
use crate::block_entity::beacon::{BeaconBlockRead, BeaconSound, BeaconTickAction};
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::world::LevelId;
use bbb_protocol::block_types::{
    vanilla_beacon_beam_dye_color_id_for_block_name, vanilla_block_type, VANILLA_BLOCK_BEDROCK_ID,
};
use bbb_protocol::builtin_enum_component_values::vanilla_dye_color_texture_diffuse_color;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
use bbb_server_types::geometry::Aabb;

impl ServerWorld {
    pub(super) fn tick_beacon(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let chunks = &self.level_by_id(dimension).chunks;
        let Some(surface) =
            super::super::level_height(chunks, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, pos.x, pos.z)
        else {
            return;
        };
        let min_y = chunks.min_y();
        let game_time = self.game_time();
        let actions = if game_time % 80 != 0 && chunks.is_ticking_full(pos.x >> 4, pos.z >> 4) {
            let Some(start_y) = chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::beacon)
                .map(|beacon| beacon.next_check_y(pos.y, min_y))
            else {
                return;
            };
            let mut pre_reads = [BeaconBlockRead {
                beam_color: None,
                light_dampening: 0,
                is_bedrock: false,
                is_base: false,
            }; 10];
            for (index, slot) in pre_reads.iter_mut().enumerate() {
                let y = start_y + index as i32;
                if y > surface {
                    break;
                }
                match beacon_block_read_loaded(chunks, BlockPos { y, ..pos }, false) {
                    Ok(read) => *slot = read,
                    Err(error) => {
                        tracing::warn!("beacon tick: {error}");
                        return;
                    }
                }
            }
            let Some(actions) = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::beacon_mut)
                .map(|beacon| {
                    beacon.tick(pos, min_y, surface, game_time, |check| {
                        pre_reads[(check.y - start_y) as usize]
                    })
                })
            else {
                return;
            };
            actions
        } else {
            let Some(mut entity) = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .take(pos)
            else {
                return;
            };
            let mut failure = None;
            let actions = entity.beacon_mut().expect("beacon dispatch").tick(
                pos,
                min_y,
                surface,
                game_time,
                |check| match self.beacon_block_read(dimension, check) {
                    Ok(read) => read,
                    Err(error) => {
                        failure.get_or_insert(error);
                        BeaconBlockRead {
                            beam_color: None,
                            light_dampening: 15,
                            is_bedrock: false,
                            is_base: false,
                        }
                    }
                },
            );
            self.put_back_block_entity(dimension, entity);
            if let Some(error) = failure {
                tracing::warn!("beacon tick: {error}");
                return;
            }
            actions
        };
        for action in actions {
            match action {
                BeaconTickAction::ApplyEffects {
                    levels,
                    primary,
                    secondary,
                } => self.apply_beacon_effects(
                    dimension,
                    pos,
                    levels,
                    primary.as_deref(),
                    secondary.as_deref(),
                    events,
                ),
                BeaconTickAction::Sound(sound) => {
                    let sound = match sound {
                        BeaconSound::Ambient => "minecraft:block.beacon.ambient",
                        BeaconSound::Activate => "minecraft:block.beacon.activate",
                        BeaconSound::Deactivate => "minecraft:block.beacon.deactivate",
                    };
                    self.play_queued_block_sound(
                        dimension,
                        PendingBlockSound {
                            pos,
                            resource_id: sound.into(),
                            volume: 1.0,
                            pitch: 1.0,
                            except_entity_id: None,
                        },
                        events,
                    );
                }
                BeaconTickAction::ConstructBeacon(levels) => {
                    // BeaconBlockEntity.java:185-187: the trigger uses a smaller, downward box.
                    let bounds = Aabb {
                        min_x: pos.x as f64,
                        max_x: pos.x as f64,
                        min_y: (pos.y - 4) as f64,
                        max_y: pos.y as f64,
                        min_z: pos.z as f64,
                        max_z: pos.z as f64,
                    }
                    .inflate(10.0, 5.0, 10.0);
                    for id in self.beacon_players(dimension, bounds) {
                        self.trigger_construct_beacon(id, levels, events);
                    }
                }
            }
        }
    }

    fn beacon_block_read(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> Result<BeaconBlockRead, String> {
        let column = (pos.x >> 4, pos.z >> 4);
        if !self
            .level_by_id(dimension)
            .chunks
            .is_ticking_full(column.0, column.1)
        {
            self.publish_full_columns(dimension, &[column])
                .map_err(|error| error.to_string())?;
        }
        let chunks = &self.level_by_id(dimension).chunks;
        beacon_block_read_loaded(chunks, pos, true)
    }

    fn beacon_players(&self, dimension: &LevelId, bounds: Aabb) -> Vec<ConnectionId> {
        self.levels
            .player_entries()
            .filter(|(_, player)| {
                &player.dimension == dimension
                    && !player.avatar_removed
                    && player.game_type != GameType::Spectator
                    && bbb_server_types::interaction::player_pose_bounding_box(
                        player.position(),
                        player.pose_id,
                        player.pose_scale(),
                    )
                    .is_ok_and(|body| body.intersects(bounds))
            })
            .map(|(&id, _)| id)
            .collect()
    }

    // BeaconBlockEntity.java:231-259: all primary effects precede all secondary effects.
    fn apply_beacon_effects(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        levels: i32,
        primary: Option<&str>,
        secondary: Option<&str>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(primary) = primary
            .and_then(bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id)
            .map(|effect| effect.resource_id)
        else {
            return;
        };
        let range = (levels * 10 + 10) as f64;
        let height = self.level_by_id(dimension).chunks.section_count() as f64 * 16.0;
        let bounds = Aabb::of_block(pos)
            .inflate(range, range, range)
            .expand_towards(Vec3d {
                x: 0.0,
                y: height,
                z: 0.0,
            });
        let players = self.beacon_players(dimension, bounds);
        let duration = (9 + levels * 2) * 20;
        let amplifier = i32::from(levels >= 4 && Some(primary) == secondary);
        let effect =
            ActiveMobEffect::with_visibility(primary, duration, amplifier, true, true, true)
                .expect("beacon selected valid effect");
        for &id in &players {
            self.add_player_mob_effect(id, effect.clone(), events);
        }
        if let Some(secondary) = secondary
            .and_then(bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id)
            .map(|effect| effect.resource_id)
            .filter(|secondary| levels >= 4 && *secondary != primary)
        {
            let effect = ActiveMobEffect::with_visibility(secondary, duration, 0, true, true, true)
                .expect("beacon selected valid secondary");
            for id in players {
                self.add_player_mob_effect(id, effect.clone(), events);
            }
        }
    }
}

fn beacon_block_read_loaded(
    chunks: &crate::chunk::ChunkStore,
    pos: BlockPos,
    check_base: bool,
) -> Result<BeaconBlockRead, String> {
    let state = super::super::stored_block_state(chunks, pos);
    let content_registry = chunks.content_registry();
    let (block_id, name) = if let Some(id) =
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
    {
        let name = vanilla_block_type(id)
            .ok_or("beacon block identity unavailable")?
            .resource_id;
        (id, name)
    } else if let Some((custom, _)) = content_registry.block_by_state_id(state) {
        (custom.block_id, custom.resource_id())
    } else {
        return Err("beacon block identity unavailable".to_string());
    };
    let is_base = check_base
        && chunks
            .registry_tags()
            .is_some_and(|tags| tags.contains_block("minecraft:beacon_base_blocks", block_id));
    Ok(BeaconBlockRead {
        beam_color: vanilla_beacon_beam_dye_color_id_for_block_name(name)
            .map(vanilla_dye_color_texture_diffuse_color),
        light_dampening: crate::block_behavior::support::light_dampening_with_registry(
            state,
            content_registry,
        ) as u8,
        is_bedrock: block_id == VANILLA_BLOCK_BEDROCK_ID,
        is_base,
    })
}
