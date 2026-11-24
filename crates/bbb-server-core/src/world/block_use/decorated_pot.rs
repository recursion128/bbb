//! `DecoratedPotBlock` (`DecoratedPotBlock.java`): the single item a click
//! inserts, and the wobble either answer plays.

use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::{
    BlockPos, InteractionHand, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::BlockUseContext;
use crate::block_behavior::decorated_pot as decorated_pot_block;
use crate::block_behavior::BlockUseResult;
use crate::block_entity::{decorated_pot, BlockEntity};
use crate::block_events::BlockEventData;
use crate::connection::ConnectionId;
use crate::level_event::VanillaGameEvent;
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{ServerWorld, WorldEvent};

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_decorated_pot_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_decorated_pot_item_on(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.update.hand,
        events,
    )
}

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_decorated_pot_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_decorated_pot_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}

impl ServerWorld {
    /// `DecoratedPotBlock.useItemOn` (`DecoratedPotBlock.java:96-139`).
    pub(super) fn use_decorated_pot_item_on(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let dimension = dimension.into();
        if self.insert_decorated_pot_item(connection_id, dimension, pos, hand, events) {
            BlockUseResult::Consumed
        } else {
            BlockUseResult::TryWithEmptyHand
        }
    }

    /// `DecoratedPotBlock.useWithoutItem` (`DecoratedPotBlock.java:142-153`).
    pub(super) fn use_decorated_pot_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::decorated_pot)
            .is_none()
        {
            return false;
        }
        self.wobble_decorated_pot(dimension, pos, decorated_pot::WobbleStyle::Negative);
        self.play_pot_sound(
            dimension,
            pos,
            decorated_pot_block::INSERT_FAIL_SOUND,
            1.0,
            events,
        );
        // `DecoratedPotBlock.java:148`.
        self.decorated_pot_player_change(connection_id, dimension, pos);
        true
    }

    pub(super) fn insert_decorated_pot_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::decorated_pot)
            .is_none()
        {
            return false;
        }
        // DecoratedPotBlockEntity.java:133-136: getTheItem unpacks before insertion admission.
        if let Err(error) =
            self.with_level(dimension, |level| level.unpack_container_loot(pos, None))
        {
            tracing::warn!("decorated pot loot unpack: {error}");
            return false;
        }
        let resolver = self.item_stack_resolver();
        let insertion = (|| {
            let player = self
                .levels
                .player(connection_id)
                .expect("connection presence was checked");
            let held = runtime_item_in_hand(player, hand);
            let Some(pot) = self
                .level_by_id(&dimension)
                .chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::decorated_pot)
            else {
                return Ok(None);
            };
            if !pot.can_insert_one(&resolver, held)? {
                return Ok(None);
            }
            let item_id = held.item_id();
            let one = held.with_count(1);
            let max = if pot.item().is_empty() {
                one.max_stack_size()?
            } else {
                pot.item().max_stack_size()?
            };
            let pitch = 0.7 + 0.5 * (pot.item().count() + 1) as f32 / max as f32;
            Ok::<_, bbb_server_types::ServerWorldError>(Some((one, item_id, pitch)))
        })();
        let (one, item_id, pitch) = match insertion {
            Ok(Some(insertion)) => insertion,
            Ok(None) => return false,
            Err(error) => {
                tracing::warn!("decorated pot insertion: {error}");
                return false;
            }
        };
        self.wobble_decorated_pot(dimension, pos, decorated_pot::WobbleStyle::Positive);
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if let Some(item_id) = item_id {
            player.award_stat_once(
                &mut self.scoreboard,
                bbb_server_types::stats::StatKey::new(
                    bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                    item_id,
                ),
            );
        }
        player
            .inventory
            .consume_in_hand(hand, 1, player.abilities.instabuild);
        let Some(pot) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::decorated_pot_mut)
        else {
            return false;
        };
        match pot.try_insert_one(&resolver, &one) {
            Ok(true) => {}
            Ok(false) => return false,
            Err(error) => {
                tracing::warn!("decorated pot insertion: {error}");
                return false;
            }
        }
        self.play_pot_sound(
            dimension,
            pos,
            decorated_pot_block::INSERT_SOUND,
            pitch,
            events,
        );
        self.level_mut_by_id(&dimension).entities.send_particles(
            bbb_protocol::packets::LevelParticles {
                override_limiter: false,
                always_show: false,
                position: Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y) + 1.2,
                    z: f64::from(pos.z) + 0.5,
                },
                offset: Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max_speed: 0.0,
                count: 7,
                particle: bbb_protocol::packets::ParticlePayload {
                    particle_type_id:
                        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DUST_PLUME_ID,
                    raw_options: Vec::new(),
                },
            },
        );
        self.with_level(dimension, |level| {
            if let Some(state) = level
                .chunks
                .block_entities()
                .get(pos)
                .map(BlockEntity::block_state)
            {
                level.block_entity_set_changed(pos, state);
            }
        });
        // `DecoratedPotBlock.java:131`.
        self.decorated_pot_player_change(connection_id, dimension, pos);
        true
    }

    fn decorated_pot_player_change(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) {
        let dimension = dimension.into();
        let source_entity_id = self
            .levels
            .player(connection_id)
            .map(|player| player.entity_id());
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(VanillaGameEvent::BLOCK_CHANGE, pos, source_entity_id, None);
    }

    pub(super) fn wobble_decorated_pot(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        style: decorated_pot::WobbleStyle,
    ) {
        let dimension = dimension.into();
        let Some(block_state_id) = self.block_state(dimension, pos) else {
            return;
        };
        let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) else {
            return;
        };
        self.level_mut_by_id(&dimension)
            .block_events
            .block_event(BlockEventData::new(
                pos,
                block_type_id,
                decorated_pot::EVENT_POT_WOBBLES,
                style as i32,
            ));
    }

    pub(super) fn play_pot_sound(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        sound_resource_id: &str,
        pitch: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(registry_id) = vanilla_sound_event_registry_id(sound_resource_id) else {
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
            volume: 1.0,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
    }
}
