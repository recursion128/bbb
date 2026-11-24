//! `Player.drop` (`Player.java:582-583`): the item entity a player-initiated
//! drop becomes, and the statistics a stack thrown from the hand awards.

use crate::connection::ConnectionId;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_DROPPED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::{custom_stat, StatKey};

use super::{advancement_loot::reward_drop_velocity, ServerWorld};

/// `createItemStackToDrop`'s `randomly` arm (`LivingEntity.java:3384-3387`).
fn random_drop_velocity(random: &crate::random::GameRandom) -> Vec3d {
    let pow = random.next_float() * 0.5;
    let dir = random.next_float() * (std::f64::consts::PI * 2.0) as f32;
    Vec3d {
        x: f64::from(-bbb_server_types::geometry::mth_sin(f64::from(dir)) * pow),
        y: f64::from(0.2_f32),
        z: f64::from(bbb_server_types::geometry::mth_cos(f64::from(dir)) * pow),
    }
}

/// `TickThrottler` (`TickThrottler.java:5-24`) with the two constants
/// `ServerGamePacketListenerImpl` builds for creative drop spam (`:244`).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct DropSpamThrottler {
    count: i32,
}

impl DropSpamThrottler {
    /// `new TickThrottler(20, 1480)` (`ServerGamePacketListenerImpl.java:244`).
    pub(super) const INCREMENT_STEP: i32 = 20;
    pub(super) const THRESHOLD: i32 = 1480;

    /// `TickThrottler.isUnderThreshold` (`TickThrottler.java:18-20`).
    pub(super) const fn is_under_threshold(&self) -> bool {
        self.count < Self::THRESHOLD
    }

    /// `TickThrottler.increment` (`:10-12`).
    pub(super) fn increment(&mut self) {
        self.count += Self::INCREMENT_STEP;
    }

    /// `TickThrottler.tick` (`:14-18`), run once per connection tick
    /// (`ServerGamePacketListenerImpl.java:299`).
    pub(super) fn tick(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        }
    }
}

impl ServerWorld {
    /// `Player.drop(ItemStack, boolean)` (`Player.java:582-583`) through
    /// `LivingEntity.createItemStackToDrop` (`LivingEntity.java:3372-3400`),
    /// plus the two statistics `ServerPlayer.drop` (`ServerPlayer.java:2003-2013`)
    /// awards when the stack left the player's hand.
    ///
    /// A stack with no persistent projection is logged and not dropped rather
    /// than becoming a different stack on the ground.
    /// Pins the creative drop throttle at a count so a test can reach the
    /// refusal arm without spending 1480 connection ticks.
    #[cfg(test)]
    pub(crate) fn set_drop_spam_count_for_test(&mut self, connection_id: ConnectionId, count: i32) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        player.cold.drop_spam_throttler.count = count;
    }

    pub(super) fn drop_player_stack<S: Clone + Into<RuntimeItemStack>>(
        &mut self,
        connection_id: ConnectionId,
        stack: &S,
        thrown_from_hand: bool,
    ) {
        let stack: RuntimeItemStack = stack.clone().into();
        let _ = self.drop_player_stack_from(connection_id, stack, false, thrown_from_hand);
    }

    pub(super) fn drop_player_original_stack(
        &mut self,
        connection_id: ConnectionId,
        stack: RuntimeItemStack,
        thrown_from_hand: bool,
    ) -> Option<i32> {
        self.drop_player_stack_from(connection_id, stack, false, thrown_from_hand)
    }

    /// `drop(stack, true, false)`, the scatter `Inventory.dropAll` uses
    /// (`Inventory.java:448`).
    pub(super) fn drop_player_stack_randomly(
        &mut self,
        connection_id: ConnectionId,
        stack: &RuntimeItemStack,
    ) {
        let _ = self.drop_player_stack_from(connection_id, stack.clone(), true, false);
    }

    fn drop_player_stack_from(
        &mut self,
        connection_id: ConnectionId,
        original: RuntimeItemStack,
        randomly: bool,
        thrown_from_hand: bool,
    ) -> Option<i32> {
        let stack = original.clone();
        let item_id = stack.item_id().filter(|_| stack.count() > 0)?;
        let player = self.levels.player(connection_id)?;
        let dimension = player.dimension;
        let thrower = player.profile.uuid;
        let Ok(eye) = player.eye_position() else {
            return None;
        };
        // `double yHandPos = this.getEyeY() - 0.3F` (`:3377-3378`).
        let position = Vec3d {
            x: eye.x,
            y: eye.y - f64::from(0.3_f32),
            z: eye.z,
        };
        // The four draws come off the player's own `Entity.random`
        // (`LivingEntity.java:3389-3397`).
        let player = self.levels.player_mut(connection_id)?;
        let velocity = if randomly {
            random_drop_velocity(&self.random)
        } else {
            reward_drop_velocity(player.rotation(), &self.random)
        };
        let spawned = crate::entity::item::spawn_player_dropped_item_entity(
            &mut self.level_mut_by_id(&dimension).entities,
            position,
            original,
            velocity,
            thrown_from_hand,
            thrower,
        );
        let Some(spawned) = spawned else {
            tracing::warn!("the entity store refused the item entity's drawn UUID");
            return None;
        };
        if thrown_from_hand {
            crate::entity::item::set_thrower_reference(
                self.level_mut_by_id(&dimension).entities.get_mut(spawned)?,
                Some(crate::entity::EntityReference::Uuid(thrower)),
            )?;
        }
        if !thrown_from_hand {
            return Some(spawned);
        }
        let player = self.levels.player_mut(connection_id)?;
        player.award_stat(
            &mut self.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_DROPPED_ID, item_id),
            stack.count(),
        );
        player.award_stat_once(&mut self.scoreboard, custom_stat("minecraft:drop"));
        Some(spawned)
    }
    pub(super) fn drop_runtime_container_contents(
        &mut self,
        dimension: &crate::world::LevelId,
        position: Vec3d,
        slots: &[RuntimeItemStack],
    ) {
        let slots = super::simulation_level::pending_spawns::physical_container_drop_stacks(slots);
        if !crate::entity::block_drops::drop_container_contents(
            &mut self.level_mut_by_id(dimension).entities,
            position,
            &slots,
        ) {
            tracing::warn!("container item publication refused");
        }
    }

    pub(super) fn assemble_cooking_recipe(
        &self,
        recipe: &bbb_server_datapack::recipe::CookingRecipe,
    ) -> bbb_server_types::Result<RuntimeItemStack> {
        Ok(RuntimeItemStack::from_summary(recipe.assemble()))
    }
}

#[cfg(test)]
mod tests {
    use super::DropSpamThrottler;

    /// `TickThrottler` (`TickThrottler.java:5-24`) with the creative drop
    /// constants (`ServerGamePacketListenerImpl.java:244`).
    #[test]
    fn the_creative_drop_throttle_counts_up_by_twenty_and_down_by_one() {
        let mut throttler = DropSpamThrottler::default();
        assert!(throttler.is_under_threshold());
        for _ in 0..74 {
            throttler.increment();
        }
        assert_eq!(throttler.count, 1480);
        assert!(!throttler.is_under_threshold());
        throttler.tick();
        assert!(throttler.is_under_threshold());
        for _ in 0..1480 {
            throttler.tick();
        }
        assert_eq!(throttler.count, 0);
    }
}
