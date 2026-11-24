//! The gamemaster requests a client sends outside the command line: its own
//! game mode, game rule writes, and the game rule readout.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use bbb_protocol::game_rules::{vanilla_game_rule, GameRuleValue};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    GameRuleValue as WireGameRuleValue, GameRuleValues, SetGameRule, SystemChat,
};
use bbb_protocol::{component_literal_nbt, normalize_resource_location, RichTextComponent};

use crate::commands::{translatable, CommandPermission};

use super::{send, ServerWorld, WorldCloseReason, WorldEvent};

impl ServerWorld {
    /// `ServerGamePacketListenerImpl.handleChangeGameMode` (:2077-2088).
    pub(super) fn apply_change_game_mode_request(
        &mut self,
        connection_id: ConnectionId,
        game_mode: GameType,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        if self.player_permission(connection_id) < CommandPermission::GameMaster {
            return None;
        }
        // `GameModeCommand.setGameMode(player, type)` (GameModeCommand.java:61-63)
        // runs over the player's own `createCommandSourceStack`, which is the
        // source and the call `/gamemode <mode>` typed by that player reaches
        // (`:27`, `:53`), so the request is that command.
        self.apply_chat_command(
            connection_id,
            &format!("gamemode {}", game_mode.serialized_name()),
            events,
        )
    }

    /// `ServerGamePacketListenerImpl.handleSetGameRule` (:787-816).
    pub(super) fn apply_set_game_rule_request(
        &mut self,
        connection_id: ConnectionId,
        packet: SetGameRule,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.player_permission(connection_id) < CommandPermission::GameMaster {
            return;
        }
        for entry in packet.entries {
            // `BuiltInRegistries.GAME_RULE` (:795) holds every rule, including
            // one the level's feature set leaves out; `GameRules.set` discards
            // that one and the broadcast still goes out (:807-808).
            let Some(rule) = normalize_resource_location(entry.rule)
                .ok()
                .and_then(|id| vanilla_game_rule(&id))
            else {
                continue;
            };
            // `GameRule.deserialize` (GameRule.java:71-79).
            let Some(value) = rule.parse_value(&entry.value) else {
                continue;
            };
            self.set_game_rule_value(rule.resource_id, value, events);
            self.broadcast_game_rule_change_to_operators(rule.resource_id, value, events);
        }
    }

    /// `broadcastGameRuleChangeToOperators` (ServerGamePacketListenerImpl.java:812-816):
    /// every online player on the operator list, whatever its level.
    fn broadcast_game_rule_change_to_operators(
        &self,
        resource_id: &str,
        value: GameRuleValue,
        events: &mut Vec<WorldEvent>,
    ) {
        // `GameRule.id` is `Identifier.toShortString` (GameRule.java:51-53).
        let id = resource_id
            .strip_prefix("minecraft:")
            .unwrap_or(resource_id);
        let message = RichTextComponent::try_from_component_nbt(translatable(
            "commands.gamerule.set",
            vec![
                component_literal_nbt(id),
                component_literal_nbt(value.serialize()),
            ],
        ))
        .expect("a translatable over two literals is a valid component");
        let operators = &self.admin.lists().operators;
        for (&connection_id, player) in self.levels.player_entries() {
            if operators.contains_key(&player.profile.uuid) {
                self.send_system_message(
                    connection_id,
                    SystemChat::from_component(message.clone(), false),
                    events,
                );
            }
        }
    }

    /// `ServerGamePacketListenerImpl.sendGameRuleValues` (:1893-1906): every
    /// rule the level's `GameRules` holds, serialized.
    pub(super) fn send_game_rule_values(
        &self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.player_permission(connection_id) < CommandPermission::GameMaster {
            return;
        }
        let values = self
            .admin
            .game_rule_values()
            .into_iter()
            .map(|(rule, value)| WireGameRuleValue {
                rule: rule.to_string(),
                value: value.serialize(),
            })
            .collect();
        events.push(send(
            connection_id,
            ClientboundPlayPacket::GameRuleValues(GameRuleValues { values }),
        ));
    }
}
