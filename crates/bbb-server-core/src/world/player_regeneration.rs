use bbb_protocol::difficulties::Difficulty;

use super::ServerWorld;

impl ServerWorld {
    // ServerPlayer.java:738-755; Player.aiStep (:447) precedes FoodData.tick.
    pub(super) fn tick_player_regeneration(&mut self) {
        if self.admin.settings().difficulty != Difficulty::Peaceful
            || !self.natural_health_regeneration()
        {
            return;
        }
        for player in self.levels.players_mut() {
            if player.avatar_removed || !player.body_tick_enabled {
                continue;
            }
            if player.tick_count % 20 == 0 {
                let max_health = player.max_health();
                if player.damage.health < max_health {
                    player.damage.heal(1.0, max_health);
                }
                let saturation = player.food.saturation_level();
                if saturation < 20.0 {
                    player.food.set_saturation(saturation + 1.0);
                }
            }
            if player.tick_count % 10 == 0 && player.food.needs_food() {
                player.food.set_food_level(player.food.food_level() + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests;
