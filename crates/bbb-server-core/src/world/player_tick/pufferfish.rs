use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::fish::pufferfish::poison;
use crate::level_event::PendingHurtEntity;
use crate::world::combat::EnchantmentEffectEntity;
use crate::world::{send, LevelId, ServerWorld, WorldEvent};
use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID;
use bbb_protocol::packets::GameEvent;

impl ServerWorld {
    // Pufferfish.java:148-157.
    pub(super) fn touch_stored_pufferfish(
        &mut self,
        dimension: &LevelId,
        connection: ConnectionId,
        source_entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let body = self.level_by_id(dimension).entities.get(source_entity_id)?;
        let puff_state = crate::entity::fish::pufferfish::puff_state(body);
        if body.removal_reason().is_some() || puff_state <= 0 {
            return Some(());
        }
        let silent = body.is_silent();
        let source = self.pending_hurt_source_entity(
            dimension,
            PendingHurtEntity::Stored {
                entity_id: source_entity_id,
            },
        );
        if self.hurt_connected_player_checked(
            connection,
            "minecraft:mob_attack",
            (1 + puff_state) as f32,
            source.clone(),
            source,
            None,
            events,
        )? {
            self.add_player_mob_effect_from(
                connection,
                poison(puff_state),
                Some((
                    *dimension,
                    EnchantmentEffectEntity::Stored(source_entity_id),
                )),
                events,
            );
            if !silent {
                events.push(send(
                    connection,
                    ClientboundPlayPacket::GameEvent(GameEvent {
                        event_id: VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID,
                        param: 0.0,
                    }),
                ));
            }
        }
        Some(())
    }
}
