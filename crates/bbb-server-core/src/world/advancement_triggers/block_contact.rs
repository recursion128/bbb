use crate::connection::ConnectionId;
use crate::loot_runtime::{matches_conditions, matches_state_properties};
use bbb_protocol::block_state_types::{vanilla_block_state_identity, VanillaBlockStateIdentity};
use bbb_server_datapack::advancement::block_contact::BlockContactCondition;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::{LootCondition, StatePropertiesPredicate};

use super::construct_beacon::player_conditions_from_canonical as player_conditions;
use super::{ServerWorld, WorldEvent};

pub(super) const SLIDE_DOWN_BLOCK: &str = "minecraft:slide_down_block";

pub(super) const ENTER_BLOCK: &str = "minecraft:enter_block";

pub(super) struct ContactCondition {
    block: Option<String>,
    state: Option<StatePropertiesPredicate>,
    player: Vec<LootCondition>,
}

impl ContactCondition {
    pub(super) fn compile(
        condition: &BlockContactCondition,
        trigger: &str,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            block: condition.block.clone(),
            state: condition.state.clone(),
            player: player_conditions(condition.player.as_ref(), trigger, lookup)?,
        })
    }

    // EnterBlockTrigger.java:53-54; SlideDownBlockTrigger.java:52-53.
    fn matches_block(&self, state: &VanillaBlockStateIdentity) -> bool {
        if self
            .block
            .as_ref()
            .is_some_and(|block| block != &state.name)
        {
            return false;
        }
        match &self.state {
            None => true,
            Some(predicate) => matches_state_properties(predicate, state) == Ok(true),
        }
    }
}

impl ServerWorld {
    pub(in crate::world) fn trigger_slide_down_block(
        &mut self,
        id: ConnectionId,
        state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_block_contact(id, SLIDE_DOWN_BLOCK, state_id, events);
    }

    pub(in crate::world) fn trigger_enter_block(
        &mut self,
        id: ConnectionId,
        state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_block_contact(id, ENTER_BLOCK, state_id, events);
    }

    // SimpleCriterionTrigger.java:43-70; EntityPredicate.java:182-188.
    fn trigger_block_contact(
        &mut self,
        id: ConnectionId,
        trigger: &str,
        state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(id) else {
            return;
        };
        let Some(conditions) = self.cold.advancement_triggers.block_contact.get(trigger) else {
            return;
        };
        let state = if let Some(identity) = vanilla_block_state_identity(state_id) {
            std::borrow::Cow::Borrowed(identity)
        } else if let Some((custom, offset)) = self
            .plugins()
            .content_registry()
            .block_by_state_id(state_id)
        {
            let mut properties = std::collections::BTreeMap::new();
            if custom.state_count > 1 || offset > 0 {
                properties.insert(
                    bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY.to_owned(),
                    offset.to_string(),
                );
            }
            std::borrow::Cow::Owned(bbb_protocol::block_state_types::VanillaBlockStateIdentity {
                id: state_id,
                name: custom.resource_id().to_owned(),
                properties,
            })
        } else {
            return;
        };
        let mut matching = Vec::new();
        let mut pending_player_check = Vec::new();
        for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
            let Some(condition) = conditions
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            else {
                continue;
            };
            if !condition.matches_block(&state) {
                continue;
            }
            if condition.player.is_empty() {
                matching.push((index, criterion.clone()));
            } else {
                pending_player_check.push((index, criterion.clone()));
            }
        }
        if !pending_player_check.is_empty() {
            let Some(startup) = self.cold.config.datapacks.as_ref() else {
                return;
            };
            let Some(checked) = self.with_advancement_player_params(player, |level, params| {
                let mut checked = Vec::new();
                for (index, criterion) in pending_player_check {
                    let Some(condition) = conditions
                        .get(&index)
                        .and_then(|criteria| criteria.get(&criterion))
                    else {
                        continue;
                    };
                    if matches_conditions(
                        &startup.loot,
                        trigger,
                        &condition.player,
                        &params,
                        &level.random,
                    ) == Ok(true)
                    {
                        checked.push((index, criterion));
                    }
                }
                checked
            }) else {
                return;
            };
            matching.extend(checked);
        }
        if !matching.is_empty() {
            self.apply_advancement_matches(id, matching, events);
        }
    }
}

#[cfg(test)]
mod tests;
