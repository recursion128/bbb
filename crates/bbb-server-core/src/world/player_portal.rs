use super::*;
use crate::entity::portal::{PortalKind, PortalProcessor};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{GameEvent, LevelEvent};
use bbb_server_types::interaction::player_pose_dimensions;

// TeleportTransition.java:51-55.
const LEVEL_EVENT_SOUND_PORTAL_TRAVEL: i32 = 1032;
const RULE_CREATIVE_DELAY: &str = "minecraft:players_nether_portal_creative_delay";
const RULE_DEFAULT_DELAY: &str = "minecraft:players_nether_portal_default_delay";

impl ServerWorld {
    // NetherPortalBlock.java:110-116, EndPortalBlock.java:74-111 and
    // EndGatewayBlock.java:89-98 for a session; Entity.java:2527-2538;
    // Player.java:392-394.
    pub(super) fn record_player_portal_contact(
        &mut self,
        id: ConnectionId,
        kind: PortalKind,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(id) else {
            return;
        };
        let dimension = player.dimension;
        // `LivingEntity.canUsePortal(false)` (LivingEntity.java:3686-3688).
        if player.avatar_removed
            || player.damage.is_dead_or_dying()
            || player.cold.sleeping_pos.is_some()
            || self
                .level_by_id(&dimension)
                .entities
                .external_player_vehicle(player.entity_id())
                .is_some()
        {
            return;
        }
        match kind {
            PortalKind::EndGateway => {
                let Some(gateway) = self
                    .level_mut_by_id(&dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(BlockEntity::end_gateway_mut)
                else {
                    return;
                };
                if gateway.is_cooling_down() {
                    return;
                }
                gateway.trigger_cooldown();
                let level = self.level_mut_by_id(&dimension);
                level.chunks.block_entities_mut().set_changed(pos);
                level
                    .block_events
                    .block_event(crate::block_events::BlockEventData::new(
                        pos,
                        bbb_protocol::block_types::VANILLA_BLOCK_END_GATEWAY_ID,
                        1,
                        0,
                    ));
            }
            PortalKind::End
                if dimension.builtin() == Some(Java26BuiltinDimension::End)
                    && !player.cold.seen_credits =>
            {
                self.show_end_credits(id, events);
                return;
            }
            PortalKind::End | PortalKind::Nether => {}
        }
        let player = self.levels.player_mut(id).unwrap();
        crate::entity::portal::record_portal_contact(
            &mut player.cold.portal_cooldown,
            &mut player.cold.portal_process,
            kind,
            pos,
            crate::entity::portal::PLAYER_PORTAL_COOLDOWN,
        );
    }

    // ServerPlayer.java:1069-1077.
    fn show_end_credits(&mut self, id: ConnectionId, events: &mut Vec<WorldEvent>) {
        // Player.java:1244-1250; AbstractContainerMenu.java:598-606: close open
        // containers and return cursor/crafting items before removing the avatar.
        if self.levels[id].inventory.container_id() != 0 {
            self.close_container(id, false, events);
        }
        self.close_container(id, false, events);
        self.remove_player_avatar(id, events);
        let player = &mut self.levels[id];
        if !player.cold.won_game {
            player.cold.won_game = true;
            player.cold.seen_credits = true;
            events.push(send(
                id,
                ClientboundPlayPacket::GameEvent(GameEvent {
                    event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
                    param: 0.0,
                }),
            ));
        }
    }

    // Entity.java:2540-2561; portal handling precedes the target fluid scan.
    pub(super) fn tick_player_portal(&mut self, id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let player = self.levels.player_at(id);
        let dimension = player.dimension;
        let allowed = !player.damage.is_dead_or_dying()
            && player.cold.sleeping_pos.is_none()
            && self
                .level_by_id(&dimension)
                .entities
                .external_player_vehicle(player.entity_id())
                .is_none();
        // NetherPortalBlock.java:119-127.
        let nether_delay = self
            .game_rule_int(if player.abilities.invulnerable {
                RULE_CREATIVE_DELAY
            } else {
                RULE_DEFAULT_DELAY
            })
            .max(0);
        let player = self.levels.player_mut(id).unwrap();
        if !player.is_changing_dimension && player.cold.portal_cooldown > 0 {
            player.cold.portal_cooldown -= 1;
        }
        let Some(kind) = player
            .cold
            .portal_process
            .as_ref()
            .map(PortalProcessor::kind)
        else {
            return;
        };
        let transition_time = if kind == PortalKind::Nether {
            nether_delay
        } else {
            0
        };
        let Some(transition) = crate::entity::portal::advance_portal_process(
            &mut player.cold.portal_cooldown,
            &mut player.cold.portal_process,
            transition_time,
            allowed,
            crate::entity::portal::PLAYER_PORTAL_COOLDOWN,
        ) else {
            return;
        };
        match transition.kind {
            PortalKind::EndGateway => {
                self.teleport_player_through_gateway(id, transition.entry, events)
            }
            PortalKind::Nether => {
                self.teleport_player_through_nether_portal(id, transition.entry, events)
            }
            PortalKind::End => self.teleport_player_through_end_portal(id, events),
        }
    }

    fn game_rule_int(&self, id: &str) -> i32 {
        match self.admin.game_rule_value(id) {
            Some(GameRuleValue::Integer(value)) => value,
            _ => 0,
        }
    }

    fn teleport_player_through_gateway(
        &mut self,
        id: ConnectionId,
        entry: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = self.levels[id].dimension;
        let Ok(Some(position)) = self.resolve_gateway_destination(&dimension, entry) else {
            return;
        };
        if !self.portal_destination_allowed(&dimension) {
            return;
        }
        let Ok(ticket) =
            self.prepare_portal_ticket_by_key(&dimension, block_position(position), &[])
        else {
            return;
        };
        let commit = self.publish_portal_destination(ticket);
        if self.teleport_player_id(
            id,
            TeleportTransition {
                new_level: dimension,
                position,
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8),
            },
            events,
        ) {
            self.commit_portal_ticket_state(commit);
        }
    }

    fn teleport_player_through_nether_portal(
        &mut self,
        id: ConnectionId,
        entry: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(id);
        let dimension = player.dimension;
        let Ok([width, height]) = player_pose_dimensions(player.pose_id, player.pose_scale())
        else {
            return;
        };
        let traveller = nether_portal::PortalTraveller {
            position: player.position(),
            width,
            height,
            reset_forward: true,
        };
        let destination = match self.resolve_nether_portal_destination(&dimension, traveller, entry)
        {
            Ok(Some(destination)) => destination,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!("player nether portal destination: {error}");
                return;
            }
        };
        let commit = self.publish_portal_destination(destination.ticket);
        let transition = destination.transition;
        if self.teleport_player_id(
            id,
            TeleportTransition {
                new_level: destination.target,
                position: transition.position,
                y_rot: transition.y_rot,
                x_rot: transition.x_rot,
                relatives_mask: transition.relatives_mask,
            },
            events,
        ) {
            self.send_portal_travel_sound(id, events);
            self.commit_portal_ticket_state(commit);
        }
    }

    fn teleport_player_through_end_portal(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(id);
        let dimension = player.dimension;
        let respawn = player.cold.respawn.clone();
        let destination = match self.player_end_portal_destination(&dimension, respawn.as_ref()) {
            Ok(Some(destination)) => destination,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!("player end portal destination: {error}");
                return;
            }
        };
        if !self.portal_destination_allowed(&destination.transition.new_level) {
            return;
        }
        // ServerPlayer.java:1084-1086.
        if destination.missing_respawn_block {
            events.push(send(
                id,
                ClientboundPlayPacket::GameEvent(GameEvent {
                    event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID,
                    param: 0.0,
                }),
            ));
        }
        if self.teleport_player_id(id, destination.transition, events) {
            if let Some(ticket) = destination.ticket {
                self.send_portal_travel_sound(id, events);
                self.commit_portal_ticket_state(ticket);
            }
        }
    }

    // TeleportTransition.java:51-55.
    pub(super) fn send_portal_travel_sound(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        events.push(send(
            id,
            ClientboundPlayPacket::LevelEvent(LevelEvent {
                event_type: LEVEL_EVENT_SOUND_PORTAL_TRAVEL,
                pos: BlockPos { x: 0, y: 0, z: 0 },
                data: 0,
                global: false,
            }),
        ));
    }
}

impl PlayerState {
    // Entity.java:580-581 and Player.java:392-394.
    pub(in crate::world) fn set_portal_cooldown(&mut self) {
        self.cold.portal_cooldown = crate::entity::portal::PLAYER_PORTAL_COOLDOWN;
    }
}
