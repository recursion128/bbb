//! `LivingEntity.checkTotemDeathProtection` (`LivingEntity.java:1361-1392`)
//! for a session.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::death_protection::{protecting_hand, ENTITY_EVENT_DEATH_PROTECTION};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::EntityEvent;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_datapack::damage::DamageTypeFacts;
use bbb_server_types::stats::StatKey;

use super::{components, runtime_item_in_hand};
use crate::world::advancement_triggers::USED_TOTEM;
use crate::world::{ServerWorld, WorldEvent};

impl ServerWorld {
    /// True when the death `source` dealt is cancelled. A held stack whose
    /// component cannot be read records a failure and lets the death through.
    pub(in crate::world) fn player_death_protection(
        &mut self,
        connection_id: ConnectionId,
        source: &DamageTypeFacts,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let player = self.levels.player_at(connection_id);
        let found = protecting_hand(source.bypasses_invulnerability, |hand| {
            let stack = runtime_item_in_hand(player, hand).summary();
            Ok::<_, String>(
                components::death_protection(stack)?.map(|effects| (stack.clone(), effects)),
            )
        });
        let (hand, (protection_item, effects)) = match found {
            Ok(Some(found)) => found,
            Ok(None) => return false,
            Err(error) => {
                tracing::warn!("death_protection: {error}");
                return false;
            }
        };

        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the protected session is connected");
        player.inventory.consume_in_hand(hand, 1, false);
        if let Some(item_id) = protection_item.item_id {
            player.award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
        self.trigger_item_criterion(connection_id, USED_TOTEM, &protection_item, events);
        self.cause_use_vibration(
            connection_id,
            &protection_item,
            VanillaGameEvent::ITEM_INTERACT_FINISH,
        );
        let Some(player) = self.levels.player_mut(connection_id) else {
            return true;
        };
        let max_health = player.max_health();
        player.damage.set_health(1.0, max_health);
        // `DeathProtection.applyEffects` (`DeathProtection.java:38-42`).
        for effect in &effects {
            if self.levels.player(connection_id).is_none() {
                return true;
            }
            self.apply_consume_effect(connection_id, effect, events);
        }
        let Some(entity_id) = self
            .levels
            .player(connection_id)
            .map(|player| player.entity_id())
        else {
            return true;
        };
        self.send_to_player_tracking_and_self(
            connection_id,
            entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: ENTITY_EVENT_DEATH_PROTECTION,
            }),
            events,
        );
        true
    }
}
