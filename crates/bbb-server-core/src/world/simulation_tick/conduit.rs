use super::*;
use crate::block_entity::conduit::{ConduitBlockRead, ConduitSound};
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::entity::projectile_action::mob_effect::PendingMobEffectAction;
use crate::entity::FluidTag;
use crate::world::LevelId;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_DARK_PRISMARINE_ID, VANILLA_BLOCK_PRISMARINE_BRICKS_ID,
    VANILLA_BLOCK_PRISMARINE_ID, VANILLA_BLOCK_SEA_LANTERN_ID,
};
use bbb_server_types::geometry::Aabb;
use uuid::Uuid;

#[derive(Clone)]
enum ConduitTarget {
    Stored(LevelId, i32),
    Player(ConnectionId),
}

impl ServerWorld {
    pub(super) fn tick_conduit(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let game_time = self.game_time();
        if game_time % 40 != 0 {
            let random = self.random.clone();
            let Some(sounds) = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::conduit_mut)
                .map(|conduit| conduit.tick_non_refresh(game_time, |bound| random.next_int(bound)))
            else {
                return;
            };
            for sound in sounds.into_iter().flatten() {
                self.play_conduit_sound(
                    dimension,
                    block_center(pos),
                    match sound {
                        ConduitSound::Ambient => "minecraft:block.conduit.ambient",
                        ConduitSound::AmbientShort => "minecraft:block.conduit.ambient.short",
                    },
                    events,
                );
            }
            return;
        }
        let Some(mut entity) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .take(pos)
        else {
            return;
        };
        let original = entity.clone();
        let mut failure = None;
        let refresh =
            entity
                .conduit_mut()
                .expect("conduit dispatch")
                .tick_shape(pos, game_time, |check| {
                    match self.conduit_block_read(dimension, check) {
                        Ok(read) => read,
                        Err(error) => {
                            failure.get_or_insert(error);
                            ConduitBlockRead {
                                water: false,
                                valid_frame: false,
                            }
                        }
                    }
                });
        if let Some(error) = failure {
            self.put_back_block_entity(dimension, original);
            tracing::warn!("conduit shape: {error}");
            return;
        }
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        if let Some(active) = refresh.active_change {
            self.play_conduit_sound(
                dimension,
                block_center(pos),
                if active {
                    "minecraft:block.conduit.activate"
                } else {
                    "minecraft:block.conduit.deactivate"
                },
                events,
            );
        }
        let mut target_changed = false;
        if refresh.refreshed && refresh.active {
            self.apply_conduit_effects(dimension, pos, refresh.effect_size, events);
            let previous = entity.conduit().unwrap().destroy_target_uuid();
            let next = self.update_conduit_target(dimension, pos, previous, refresh.hunting);
            if let Some(target) = next.and_then(|uuid| self.resolve_conduit_target(dimension, uuid))
            {
                if let Some((position, _)) = self.conduit_target_position(&target) {
                    self.play_conduit_sound(
                        dimension,
                        position,
                        "minecraft:block.conduit.attack.target",
                        events,
                    );
                    self.hurt_conduit_target(dimension, target, events);
                }
            }
            target_changed = entity.conduit_mut().unwrap().set_destroy_target(next);
        }
        self.put_back_block_entity(dimension, entity);
        if target_changed {
            if let Some(state) = self.block_state_by_key(dimension, pos) {
                self.with_level(dimension, |level| {
                    level.send_block_updated(pos, state, state)
                });
            }
        }
        let random = self.random.clone();
        let Some(sounds) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::conduit_mut)
            .map(|conduit| conduit.ambient_tick(game_time, |bound| random.next_int(bound)))
        else {
            return;
        };
        for sound in sounds {
            self.play_conduit_sound(
                dimension,
                block_center(pos),
                match sound {
                    ConduitSound::Ambient => "minecraft:block.conduit.ambient",
                    ConduitSound::AmbientShort => "minecraft:block.conduit.ambient.short",
                },
                events,
            );
        }
    }

    fn hurt_conduit_target(
        &mut self,
        caller: &LevelId,
        target: ConduitTarget,
        events: &mut Vec<WorldEvent>,
    ) {
        match target {
            ConduitTarget::Player(id) => {
                let dimension = self.levels[id].dimension;
                self.hurt_connected_player_from_level(
                    id,
                    crate::world::combat::LevelPlayerDamage {
                        damage_type: "minecraft:magic",
                        amount: 4.0,
                        causing: None,
                        direct: None,
                        source_position: None,
                    },
                    Some(caller),
                    events,
                );
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
                if caller != &dimension {
                    self.apply_level_entity_lifecycle_by_key(caller, events);
                }
            }
            ConduitTarget::Stored(dimension, id) => {
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
                let entities = &self.level_by_id(&dimension).entities;
                let lifecycle = entities.lifecycle_checkpoint();
                let tags = self
                    .level_by_id(&dimension)
                    .chunks
                    .registry_tags_shared()
                    .map_or(
                        crate::entity::EntityRegistryTagView::Builtin26_1,
                        crate::entity::EntityRegistryTagView::Frozen,
                    );
                self.hurt_stored_entity_from_level(
                    dimension,
                    id,
                    "minecraft:magic",
                    4.0,
                    None,
                    None,
                    None,
                    tags,
                    false,
                    Some(caller),
                    events,
                );
                if caller != &dimension {
                    let broadcasts = self
                        .level_mut_by_id(&dimension)
                        .entities
                        .take_damage_broadcasts_since(lifecycle);
                    for (id, packet) in broadcasts {
                        crate::world::player_packets::broadcast_to(
                            self.level_by_id(caller).entity_tracker.viewers_of(id),
                            ClientboundPlayPacket::DamageEvent(packet),
                            events,
                        );
                    }
                }
                // The silverfish land in the conduit's level.
                let infested = self
                    .level_mut_by_id(&dimension)
                    .entities
                    .drain_commands_where(|command| {
                        matches!(
                            command,
                            crate::entity::EntityCommand::MobEffect(
                                PendingMobEffectAction::Infested { .. }
                            )
                        )
                    });
                for command in infested {
                    if let crate::entity::EntityCommand::MobEffect(
                        PendingMobEffectAction::Infested {
                            position,
                            velocities,
                        },
                    ) = command
                    {
                        super::super::combat::spawn_infested_children(
                            self.level_mut_by_id(caller),
                            position,
                            velocities,
                        );
                    }
                }
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
                if caller != &dimension {
                    self.apply_level_entity_lifecycle_by_key(caller, events);
                }
            }
        }
    }

    fn conduit_block_read(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> Result<ConduitBlockRead, String> {
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
        let state = super::super::stored_block_state(chunks, pos);
        let block_id = bbb_server_types::block_queries::block_type_of_with_registry(
            state,
            chunks.content_registry(),
        )
        .ok_or("conduit block identity unavailable")?;
        let fluid = bbb_server_types::fluid::block_state_fluid_state(state);
        let water = self
            .level_by_id(dimension)
            .chunks
            .registry_tags()
            .is_some_and(|tags| {
                tags.contains(
                    "minecraft:fluid",
                    "minecraft:water",
                    fluid.identity().resource_id(),
                )
            });
        Ok(ConduitBlockRead {
            water,
            valid_frame: matches!(
                block_id,
                VANILLA_BLOCK_PRISMARINE_ID
                    | VANILLA_BLOCK_PRISMARINE_BRICKS_ID
                    | VANILLA_BLOCK_SEA_LANTERN_ID
                    | VANILLA_BLOCK_DARK_PRISMARINE_ID
            ),
        })
    }

    fn play_conduit_sound(
        &mut self,
        dimension: &LevelId,
        position: Vec3d,
        resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let packet = SoundEvent {
            sound: SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(resource_id)
                    .expect("built-in conduit sound"),
            },
            source: SoundSource::Blocks,
            position,
            volume: 1.0,
            pitch: 1.0,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(dimension, &packet, None, events);
    }

    fn apply_conduit_effects(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        size: usize,
        events: &mut Vec<WorldEvent>,
    ) {
        let range = (size / 7 * 16) as f64;
        let level = self.level_by_id(dimension);
        let bounds = Aabb::of_block(pos)
            .inflate(range, range, range)
            .expand_towards(Vec3d {
                x: 0.0,
                y: level.chunks.section_count() as f64 * 16.0,
                z: 0.0,
            });
        let players: Vec<_> = self
            .levels
            .player_entries()
            .filter_map(|(&id, player)| {
                if &player.dimension != dimension
                    || player.avatar_removed
                    || player.game_type == GameType::Spectator
                {
                    return None;
                }
                let body = bbb_server_types::interaction::player_pose_bounding_box(
                    player.position(),
                    player.pose_id,
                    player.pose_scale(),
                )
                .ok()?;
                let at = block_position(player.position());
                let rain = level.weather.is_raining_at(&level.chunks, at)
                    || level.weather.is_raining_at(
                        &level.chunks,
                        BlockPos {
                            y: body.max_y.floor() as i32,
                            ..at
                        },
                    );
                (body.intersects(bounds)
                    && closer_than(pos, at, range)
                    && (player.fluid_interaction.is_in_fluid(FluidTag::Water) || rain))
                    .then_some(id)
            })
            .collect();
        for id in players {
            let effect = ActiveMobEffect::with_visibility(
                "minecraft:conduit_power",
                260,
                0,
                true,
                true,
                true,
            )
            .unwrap();
            self.add_player_mob_effect(id, effect, events);
        }
    }

    fn resolve_conduit_target(&self, holder: &LevelId, uuid: Uuid) -> Option<ConduitTarget> {
        if let Some((dimension, id)) = self.entity_reference(holder, uuid) {
            let entity = self.level_by_id(&dimension).entities.get(id)?;
            if entity.removal_reason().is_none() && entity.living().is_some() {
                return Some(ConduitTarget::Stored(dimension, id));
            }
        }
        let id = self.player_reference(uuid)?;
        (!self.levels[id].avatar_removed).then_some(ConduitTarget::Player(id))
    }

    fn conduit_target_position(&self, target: &ConduitTarget) -> Option<(Vec3d, bool)> {
        match target {
            ConduitTarget::Stored(dimension, id) => {
                let entity = self.level_by_id(dimension).entities.get(*id)?;
                Some((
                    entity.position(),
                    entity.removal_reason().is_none() && entity.living()?.health() > 0.0,
                ))
            }
            ConduitTarget::Player(id) => {
                let player = self.levels.player(*id)?;
                Some((
                    player.position(),
                    !player.avatar_removed && player.damage.health > 0.0,
                ))
            }
        }
    }

    // ConduitBlockEntity.java:199-225: an invalid existing reference clears for this refresh.
    fn update_conduit_target(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        target: Option<Uuid>,
        hunting: bool,
    ) -> Option<Uuid> {
        if !hunting {
            return None;
        }
        if let Some(uuid) = target {
            return self
                .resolve_conduit_target(dimension, uuid)
                .and_then(|target| self.conduit_target_position(&target))
                .filter(|(position, alive)| {
                    *alive && closer_than(pos, block_position(*position), 8.0)
                })
                .map(|_| uuid);
        }
        let bounds = Aabb::of_block(pos).inflate(8.0, 8.0, 8.0);
        let level = self.level_by_id(dimension);
        let candidates: Vec<_> = level
            .entities
            .ids_in_box_sections(bounds)
            .into_iter()
            .filter_map(|id| {
                let entity = level.entities.get(id)?;
                if entity.removal_reason().is_some()
                    || entity.living().is_none()
                    || !entity.bounding_box().intersects(bounds)
                    || !crate::entity::ai::entity_type_is_enemy(entity.entity_type_id())
                {
                    return None;
                }
                let feet = entity.block_position();
                let rain = level.weather.is_raining_at(&level.chunks, feet)
                    || level.weather.is_raining_at(
                        &level.chunks,
                        BlockPos {
                            y: entity.bounding_box().max_y.floor() as i32,
                            ..feet
                        },
                    );
                (entity.is_in_water() || rain).then_some(entity.uuid())
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let index = self
            .level_mut_by_id(dimension)
            .random
            .next_int(candidates.len() as i32) as usize;
        Some(candidates[index])
    }
}

fn block_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: pos.x as f64 + 0.5,
        y: pos.y as f64 + 0.5,
        z: pos.z as f64 + 0.5,
    }
}
fn block_position(pos: Vec3d) -> BlockPos {
    BlockPos {
        x: pos.x.floor() as i32,
        y: pos.y.floor() as i32,
        z: pos.z.floor() as i32,
    }
}
fn closer_than(a: BlockPos, b: BlockPos, range: f64) -> bool {
    let dx = a.x as f64 - b.x as f64;
    let dy = a.y as f64 - b.y as f64;
    let dz = a.z as f64 - b.z as f64;
    dx * dx + dy * dy + dz * dz < range * range
}
