//! Session owner of the synchronous standing-block callback.

use super::{ServerWorld, WorldEvent};
use crate::block_behavior::ground_contact::{GroundBody, StepOn};
use crate::connection::ConnectionId;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;

impl ServerWorld {
    pub(super) fn player_block_step_on(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let pos = self.player_legacy_support_pos(id)?;
        let body = self.player_ground_body(id)?;
        let player = self.levels.player(id).unwrap();
        let (dimension, delta) = (player.dimension, player.delta_movement());
        let state = self.level_by_id(&dimension).block_state(pos);
        let random = &self.random;
        match crate::block_behavior::ground_contact::step_on_with_registry(
            state,
            body,
            delta,
            random,
            self.registries.content_registry(),
        )? {
            StepOn::None => {}
            StepOn::Hurt(damage_type, amount) => {
                self.hurt_connected_player_checked(
                    id,
                    damage_type,
                    amount,
                    None,
                    None,
                    None,
                    events,
                )?;
            }
            StepOn::Movement(movement) => {
                self.levels[id].set_delta_movement(movement);
            }
            StepOn::DecreaseTurtleEggs => self.with_level(dimension, |level| {
                crate::block_behavior::turtle_egg::decrease_eggs(level, pos, state)
            })?,
            StepOn::InteractRedstoneOre => self.with_level(dimension, |level| {
                crate::block_behavior::redstone_ore::interact(level, pos, state)
            })?,
            StepOn::SculkSensor => self.step_on_sculk_sensor(&dimension, pos, state, id, events),
            StepOn::SculkShrieker => {
                let uuid = self.levels[id].profile.uuid;
                self.step_on_sculk_shrieker(&dimension, pos, uuid, events);
            }
        }
        Some(())
    }

    pub(super) fn player_ground_body(&self, id: ConnectionId) -> Option<GroundBody> {
        let player = self.levels.player(id)?;
        let [width, height] = bbb_server_types::interaction::player_pose_dimensions(
            player.pose_id,
            player.pose_scale(),
        )
        .ok()?;
        Some(GroundBody {
            living: true,
            player: true,
            zombie: false,
            turtle_or_bat: false,
            careful: player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
            mob_griefing: match self.admin.game_rule_value("minecraft:mob_griefing") {
                Some(bbb_protocol::game_rules::GameRuleValue::Boolean(value)) => Some(value),
                _ => None,
            },
            width,
            height,
        })
    }
}

#[cfg(test)]
mod tests;
