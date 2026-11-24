use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_GRAVITY_ID;

use super::*;

impl ServerWorld {
    /// The ground answer of the server-side `LivingEntity.travel` every
    /// `ServerPlayer.doTick` runs (`LivingEntity.java:3061-3065`): gravity
    /// moves the body and `Entity.move` rewrites `verticalCollisionBelow`,
    /// `onGround` and the supporting block (`Entity.java:757-761`), then
    /// `tickPlayer` snaps the position back
    /// (`ServerGamePacketListenerImpl.java:314-315`). The client owns the
    /// position, so only those answers are kept until the next accepted move
    /// writes its own.
    pub(in crate::world) fn tick_player_ground(&mut self) {
        let order: Vec<ConnectionId> = self
            .levels
            .player_entries()
            .filter(|(_, player)| player.body_tick_enabled && !player.avatar_removed)
            .map(|(&connection_id, _)| connection_id)
            .collect();
        for connection_id in order {
            let player = &self.levels[connection_id];
            let passenger = self
                .level_by_id(&player.dimension)
                .entities
                .external_player_vehicle(player.entity_id())
                .is_some();
            // Player.java:233-235.
            if player.game_type == GameType::Spectator || passenger {
                self.levels[connection_id].set_on_ground(false);
                continue;
            }
            // A new body's first travel moves by the delta it was made with,
            // and a zero one leaves the ground alone (`Entity.java:757`).
            if player.tick_count == 1 && player.delta_movement().y == 0.0 {
                continue;
            }
            let gravity = player
                .attributes
                .value(VANILLA_ATTRIBUTE_GRAVITY_ID)
                .unwrap_or(0.08);
            // LivingEntity.java:2431-2442: the standing body's settled delta. A
            // flying body only scales its last one down (Player.java:1394-1397),
            // which stays below zero, so it asks the same question.
            let delta = Vec3d {
                x: 0.0,
                y: -gravity * 0.98,
                z: 0.0,
            };
            let target = player.position().plus(delta);
            // An unloaded column answers `None` and keeps the last answer.
            let Some(probe) = self.player_move_physics_probe_for_mover(
                connection_id,
                target,
                delta,
                player.on_ground(),
                crate::entity::MoverType::Self_,
            ) else {
                continue;
            };
            let on_ground = probe.next_vertical_collision_below;
            let Some(support) = self.player_support_for_movement(
                connection_id,
                player.position(),
                probe.solved_delta,
                on_ground,
            ) else {
                continue;
            };
            let player = &mut self.levels[connection_id];
            player.movement.vertical_collision_below = on_ground;
            player.set_on_ground(on_ground);
            (
                player.block_contact.supporting,
                player.block_contact.on_ground_no_blocks,
            ) = support;
        }
    }
}
