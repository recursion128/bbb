//! `BellBlock` (`BellBlock.java`): the ring a proper hit starts.

use bbb_protocol::packets::{
    BlockPos, Direction, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::BlockUseContext;
use crate::block_behavior::bell as bell_block;
use crate::block_entity::{bell, BlockEntity};
use crate::world::{LevelId, ServerWorld, WorldEvent};

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_bell_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_bell_without_item(context, events)
}

impl ServerWorld {
    /// `BellBlock.useWithoutItem` (`BellBlock.java:91-95`).
    pub(super) fn use_bell_without_item(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let block_state_id = context.block_state_id;
        let clicked = context.update.hit.direction;
        let cursor_y = context.update.hit.cursor_y;
        if !bell_block::is_proper_hit(block_state_id, clicked, cursor_y) {
            return false;
        }
        let source_entity_id = self.levels[connection_id].entity_id();
        let did_ring = self.ring_bell(&dimension, pos, clicked, Some(source_entity_id), events);
        if did_ring {
            // `BellBlock.onHit` (`BellBlock.java:104-107`): the counter is on
            // the ring, not on the proper hit, and the `player != null` half of
            // the test is the redstone and projectile callers (`:77`, `:86-87`)
            // rather than this one.
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                bbb_server_types::stats::custom_stat("minecraft:bell_ring"),
            );
        }
        did_ring
    }

    /// `BellBlock.attemptToRing` (`BellBlock.java:139-153`).
    /// `BellBlockEntity.updateEntities` (`BellBlockEntity.java:103-118`):
    /// every living mob within 32 blocks of the bell hears it.
    pub(in crate::world) fn wake_bell_listeners(&mut self, dimension: &LevelId, pos: BlockPos) {
        let game_time = self.game_time();
        let center = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        };
        let entities = &mut self.level_mut_by_id(dimension).entities;
        let area = bbb_server_types::geometry::Aabb {
            min_x: center.x - 48.5,
            min_y: center.y - 48.5,
            min_z: center.z - 48.5,
            max_x: center.x + 48.5,
            max_y: center.y + 48.5,
            max_z: center.z + 48.5,
        };
        let listeners: Vec<i32> = entities
            .ids_in_box_sections(area)
            .into_iter()
            .filter(|id| {
                entities.get(*id).is_some_and(|entity| {
                    let position = entity.position();
                    let (dx, dy, dz) = (
                        position.x - center.x,
                        position.y - center.y,
                        position.z - center.z,
                    );
                    entity.living().is_some()
                        && entity.is_alive_with_health()
                        && dx * dx + dy * dy + dz * dz < 32.0 * 32.0
                })
            })
            .collect();
        for id in listeners {
            entities.set_mob_brain_memory(
                id,
                None,
                crate::entity::ai::MemoryModuleType::HeardBellTime,
                Some(crate::entity::ai::MemoryValue::Long(game_time)),
            );
        }
    }

    pub(in crate::world) fn tick_bell(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let action = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::bell_mut)
            .expect("ticking bell")
            .tick();
        match action {
            bell::BellTick::Search => {
                if self.bell_raiders(dimension, pos, 32.0).is_empty() {
                    return;
                }
                self.level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(BlockEntity::bell_mut)
                    .expect("ticking bell")
                    .start_resonating();
                self.play_bell_sound_named(
                    dimension,
                    pos,
                    "minecraft:block.bell.resonate",
                    1.0,
                    events,
                );
            }
            bell::BellTick::Glow => {
                // `BellBlockEntity.glow` (`BellBlockEntity.java:161-163`).
                for id in self.bell_raiders(dimension, pos, 48.0) {
                    let effect = crate::entity::living::mob_effects::ActiveMobEffect::new(
                        "minecraft:glowing",
                        60,
                        0,
                    )
                    .expect("vanilla effect");
                    let connection = self.level_by_id(dimension).entities.player_connection(id);
                    match connection {
                        Some(connection) => {
                            self.add_player_mob_effect(connection, effect, events);
                        }
                        None => {
                            self.add_stored_mob_effect(dimension, id, effect);
                        }
                    }
                }
            }
            bell::BellTick::None => {}
        }
    }

    /// `BellBlockEntity.areRaidersNearby` / `isRaiderWithinRange`
    /// (`BellBlockEntity.java:120-128`, `:157-159`).
    fn bell_raiders(&self, dimension: &LevelId, pos: BlockPos, radius: f64) -> Vec<i32> {
        let level = self.level_by_id(dimension);
        let Some(tags) = level.chunks.registry_tags() else {
            return Vec::new();
        };
        let center = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        };
        let area = bbb_server_types::geometry::Aabb {
            min_x: center.x - radius,
            min_y: center.y - radius,
            min_z: center.z - radius,
            max_x: center.x + radius,
            max_y: center.y + radius,
            max_z: center.z + radius,
        };
        crate::entity::store::EntityBodyReadView::new(&level.entities, None, None)
            .ids_in_box_sections_with_players(area)
            .into_iter()
            .filter(|id| {
                let player = level.entities.player(*id);
                player
                    .map(|player| player.body())
                    .or_else(|| level.entities.get(*id))
                    .is_some_and(|entity| {
                        let position = entity.position();
                        let (dx, dy, dz) = (
                            position.x - center.x,
                            position.y - center.y,
                            position.z - center.z,
                        );
                        let alive = player.map_or_else(
                            || entity.living().is_some() && entity.is_alive_with_health(),
                            |player| {
                                !player.avatar_removed
                                    && entity.is_alive()
                                    && !player.damage.is_dead_or_dying()
                            },
                        );
                        alive
                            && dx * dx + dy * dy + dz * dz < radius * radius
                            && tags.contains(
                                "minecraft:entity_type",
                                "minecraft:raiders",
                                entity.data().entity_resource_id(),
                            )
                    })
            })
            .collect()
    }

    pub(in crate::world) fn ring_bell(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        direction: Direction,
        source_entity_id: Option<i32>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(block_state_id) = self.block_state_by_key(dimension, pos) else {
            return false;
        };
        let rang = self.with_level(*dimension, |level| {
            bell_block::ring(level, pos, block_state_id, direction, source_entity_id)
        });
        if rang {
            self.flush_level_events(dimension, events);
            self.post_pending_game_events(dimension);
        }
        rang
    }

    fn play_bell_sound_named(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        name: &str,
        volume: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(registry_id) = vanilla_sound_event_registry_id(name) else {
            return;
        };
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume,
            pitch: 1.0,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(dimension, &sound, None, events);
    }
}
