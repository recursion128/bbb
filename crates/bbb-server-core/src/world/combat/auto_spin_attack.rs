//! A riptide trident's spin: `LivingEntity.aiStep`'s `autoSpinAttackTicks`
//! countdown (`LivingEntity.java:3092-3095`) over `checkAutoSpinAttack`
//! (`:3182-3203`), which on a session body attacks the first living entity
//! it touches through `Player.doAutoAttackOnTouch` (`Player.java:1178-1180`).

use bbb_protocol::game_types::GameType;
use bbb_server_types::geometry::Vec3Ops;

use super::player_melee::MeleeTarget;
use crate::connection::ConnectionId;
use crate::entity::EntityRegistryTagView;
use crate::world::{ServerWorld, WorldEvent};

impl ServerWorld {
    pub(in crate::world) fn tick_player_auto_spin_attacks(&mut self, events: &mut Vec<WorldEvent>) {
        let spinning: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                !player.avatar_removed
                    && player.body_tick_enabled
                    && player.using_item.auto_spin_attack.is_some()
            })
            .map(|(connection_id, _)| *connection_id)
            .collect();
        for connection_id in spinning {
            let player = &mut self.levels[connection_id];
            let Some((ticks, _, _)) = player.using_item.auto_spin_attack.as_mut() else {
                continue;
            };
            if *ticks > 0 {
                *ticks -= 1;
                self.check_auto_spin_attack(connection_id, events);
            }
            let player = &mut self.levels[connection_id];
            if player
                .using_item
                .auto_spin_attack
                .as_ref()
                .is_none_or(|(ticks, _, _)| *ticks <= 0)
            {
                player.using_item.end_auto_spin_attack();
            }
        }
    }

    /// `LivingEntity.checkAutoSpinAttack` (`LivingEntity.java:3182-3196`) over
    /// `old.minmax(current)`. A session body travels on its client, so the
    /// old box is the current one moved back by the last reported move.
    fn check_auto_spin_attack(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = &self.levels[connection_id];
        let Ok(bounds) = player.bounding_box() else {
            return;
        };
        let bounds = bounds.minmax(bounds.moved(player.known_movement.scale(-1.0)));
        let dimension = player.dimension;
        let horizontal_collision = player.body().horizontal_collision();
        let level = self.level_by_id(&dimension);
        let stored = level
            .entities
            .ids_in_box_sections(bounds)
            .into_iter()
            .filter_map(|id| level.entities.get(id))
            .filter(|entity| entity.bounding_box().intersects(bounds))
            .map(|entity| entity.living().map(|_| MeleeTarget::Stored(entity.id())));
        let sessions = self
            .levels
            .player_entries()
            .filter(|(&id, other)| {
                id != connection_id
                    && other.dimension == dimension
                    && !other.avatar_removed
                    && other.game_type != GameType::Spectator
                    && other
                        .bounding_box()
                        .is_ok_and(|other| other.intersects(bounds))
            })
            .map(|(&id, _)| Some(MeleeTarget::Session(id)));
        let mut touched = stored.chain(sessions).peekable();
        let any = touched.peek().is_some();
        let target = touched.flatten().next();
        let Some(target) = target else {
            if !any && horizontal_collision {
                if let Some((ticks, _, _)) = self.levels[connection_id]
                    .using_item
                    .auto_spin_attack
                    .as_mut()
                {
                    *ticks = 0;
                }
            }
            return;
        };
        let registry_tags = level
            .chunks
            .registry_tags_shared()
            .map(EntityRegistryTagView::Frozen)
            .unwrap_or(EntityRegistryTagView::Builtin26_1);
        let _ = self.player_melee_attack(connection_id, target, &registry_tags, events);
        let player = &mut self.levels[connection_id];
        if let Some((ticks, _, _)) = player.using_item.auto_spin_attack.as_mut() {
            *ticks = 0;
        }
        let delta = player.delta_movement().scale(-0.2);
        player.set_delta_movement(delta);
    }
}
