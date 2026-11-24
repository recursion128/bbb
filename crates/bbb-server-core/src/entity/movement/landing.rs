use crate::block_behavior::ground_contact::{self, FallOnBlock, GroundBody};
use crate::connection::ClientboundPlayPacket;
use crate::entity::{Entity, EntityStore, PendingEntitySound};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::EntityEvent;

pub(super) fn fall_on(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    distance: f64,
) -> Option<()> {
    let pos = entity.on_pos_legacy();
    let state = level.block_state(pos);
    let body = GroundBody {
        player: false,
        ..crate::entity::inside_blocks::ground_body(entity, level)
    };
    let landing = ground_contact::fall_on_with_registry(
        state,
        distance,
        body,
        level.level_random(),
        level.content_registry(),
    )?;
    match landing.block {
        FallOnBlock::None => {}
        // FarmlandBlock.java:109-119; the detached body is back in the store
        // when the queued lift runs.
        FallOnBlock::TrampleFarmland => {
            let dirt = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID,
            )?;
            store.queue_command(crate::entity::EntityCommand::PushEntitiesUp {
                old_state: state,
                new_state: dirt,
                pos,
            });
            crate::block_behavior::farmland::turn_to_dirt_with_source(
                level,
                pos,
                Some(entity.id()),
            );
        }
        FallOnBlock::DecreaseTurtleEggs => level.decrease_turtle_eggs(
            crate::entity::level::EntityBlockContext::new(Some(&mut *store), Some(entity)),
            state,
            pos,
        )?,
        FallOnBlock::FallSound { big } => {
            let sound = crate::entity::living::fall_sound_for(entity, big);
            play_sound(entity, store, sound, 1.0, 1.0);
        }
        FallOnBlock::HoneySlide => {
            play_sound(entity, store, "minecraft:block.honey_block.slide", 1.0, 1.0);
            let entity_id = entity.id();
            store.broadcast_to_tracking(
                entity_id,
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: 54,
                }),
            );
        }
    }
    if let Some(damage) = landing.damage {
        let hurt = crate::entity::living::cause_fall_damage(
            entity,
            store,
            level,
            damage.distance,
            damage.modifier,
            damage.damage_type,
        )?;
        if let Some((sound, volume, pitch)) = landing.damaged_sound.filter(|_| hurt) {
            play_sound(entity, store, sound, volume, pitch);
        }
    }
    let affected = entity
        .main_supporting_block_pos
        .map(|pos| level.block_state(pos))
        .unwrap_or(state);
    level.game_event_at_with_state(
        VanillaGameEvent::HIT_GROUND,
        entity.position(),
        Some(entity.id()),
        Some(affected),
    );
    Some(())
}

fn play_sound(
    entity: &Entity,
    store: &mut EntityStore,
    resource_id: &'static str,
    volume: f32,
    pitch: f32,
) {
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: resource_id.to_owned(),
            source: crate::entity::living::living_sound_source(entity),
            volume,
            pitch,
        });
    }
}
