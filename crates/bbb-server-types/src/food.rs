//! FoodData hunger state, exhaustion cascade, natural regeneration and starvation.

use bbb_protocol::difficulties::Difficulty;

/// The food level a fresh `FoodData` holds (`FoodData.java:14`), which is also
/// the ceiling `add` clamps to (`:20`) and the gate the saturated regeneration
/// branch tests (`:45`).
pub const MAX_FOOD_LEVEL: i32 = 20;

/// `FoodData.saturationLevel`'s initializer (`FoodData.java:15`).
pub const DEFAULT_SATURATION_LEVEL: f32 = 5.0;

/// The ceiling `addExhaustion` clamps to (`FoodData.java:101`).
pub const MAX_EXHAUSTION_LEVEL: f32 = 40.0;

/// The exhaustion one drop of the cascade costs, and the level it needs before
/// it drops at all (`FoodData.java:35-36`).
///
/// The test is `>`, not `>=`, so a bar sitting on exactly this much is one
/// charge short of a drop rather than one drop behind.
pub const EXHAUSTION_DROP: f32 = 4.0;

/// How much saturation one drop of the cascade spends (`FoodData.java:38`).
const SATURATION_PER_DROP: f32 = 1.0;

/// The food level the fast regeneration branch needs (`FoodData.java:45`) and
/// the slow one needs (`:53`).
const SATURATED_REGEN_FOOD_LEVEL: i32 = MAX_FOOD_LEVEL;
const SLOW_REGEN_FOOD_LEVEL: i32 = 18;

/// The tick counts the three timed branches wait for (`FoodData.java:47`,
/// `:55`, `:62`).
const SATURATED_REGEN_TICKS: i32 = 10;
const SLOW_REGEN_TICKS: i32 = 80;
const STARVE_TICKS: i32 = 80;

/// The saturation the fast branch may spend in one heal (`FoodData.java:48`)
/// and the divisor that turns it into health (`:49`).
const SATURATED_REGEN_MAX_SPEND: f32 = 6.0;

/// The slow branch's fixed heal and its exhaustion cost (`FoodData.java:56-57`).
const SLOW_REGEN_HEAL: f32 = 1.0;
const SLOW_REGEN_EXHAUSTION: f32 = 6.0;

/// The damage one starvation tick deals (`FoodData.java:64`).
pub const STARVE_DAMAGE: f32 = 1.0;

/// The two health floors the starvation branch tests (`FoodData.java:63`).
const STARVE_FLOOR_EASY: f32 = 10.0;
const STARVE_FLOOR_NORMAL: f32 = 1.0;

/// `Block.playerDestroy`'s exhaustion (`Block.java:479`).
pub const EXHAUSTION_MINE: f32 = 0.005;

/// `Player.attack`'s exhaustion, which both of its two landing arms pay
/// (`Player.java:996`, `:1236`).
pub const EXHAUSTION_ATTACK: f32 = 0.1;

/// `ServerPlayer.jumpFromGround`'s two exhaustions
/// (`ServerPlayer.java:1532-1540`), which is also where `Stats.JUMP` is
/// awarded: sprinting costs the first and every other jump the second.
pub const EXHAUSTION_JUMP_SPRINTING: f32 = 0.2;
pub const EXHAUSTION_JUMP: f32 = 0.05;

/// What one [`FoodData::tick`] asks the caller to do to the player.
///
/// The three arms are exclusive because vanilla's `if`/`else if` chain
/// (`FoodData.java:44-71`) is: at most one of them runs in a tick, and the
/// fourth arm (`:69-71`) only resets the timer, which `tick` does itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FoodTickEffect {
    /// No branch reached its timer this tick.
    Nothing,
    /// `player.heal(amount)` (`FoodData.java:49`, `:56`).
    Heal(f32),
    /// `player.hurtServer(level, player.damageSources().starve(), 1.0F)`
    /// (`FoodData.java:64`).
    Starve,
}

/// The four fields `FoodData` declares (`FoodData.java:14-17`).
///
/// `exhaustion_level` and `tick_timer` have no vanilla getter -- 26.1 declares
/// neither `getExhaustionLevel` nor `getTickTimer` -- but both are written to
/// the playerdata document (`:83`, `:85`), so both are readable here. The
/// accessors exist for the save path and for tests, not because vanilla has
/// them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodData {
    food_level: i32,
    saturation_level: f32,
    exhaustion_level: f32,
    tick_timer: i32,
}

impl FoodData {
    /// The field initializers (`FoodData.java:14-17`): twenty food, five
    /// saturation, and both counters at zero.
    pub const fn created() -> Self {
        Self {
            food_level: MAX_FOOD_LEVEL,
            saturation_level: DEFAULT_SATURATION_LEVEL,
            exhaustion_level: 0.0,
            tick_timer: 0,
        }
    }

    /// `FoodData.readAdditionalSaveData` (`FoodData.java:74-79`).
    ///
    /// The four defaults are the reader's own (`:75-78`); a caller that has all
    /// four fields passes them and the defaults never apply. Nothing is clamped
    /// on the way in, exactly as vanilla clamps nothing there: the four writes
    /// are raw field assignments, so a document naming a food level of ninety
    /// restores ninety and stays there until `add` next clamps it.
    pub const fn from_saved(
        food_level: i32,
        tick_timer: i32,
        saturation_level: f32,
        exhaustion_level: f32,
    ) -> Self {
        Self {
            food_level,
            saturation_level,
            exhaustion_level,
            tick_timer,
        }
    }

    /// `FoodData.getFoodLevel()` (`FoodData.java:88-90`).
    pub const fn food_level(&self) -> i32 {
        self.food_level
    }

    /// `FoodData.getSaturationLevel()` (`FoodData.java:104-106`).
    pub const fn saturation_level(&self) -> f32 {
        self.saturation_level
    }

    /// `FoodData.exhaustionLevel` (`FoodData.java:16`), which
    /// `addAdditionalSaveData` writes as `foodExhaustionLevel` (`:85`).
    pub const fn exhaustion_level(&self) -> f32 {
        self.exhaustion_level
    }

    /// `FoodData.tickTimer` (`FoodData.java:17`), written as `foodTickTimer`
    /// (`:83`).
    pub const fn tick_timer(&self) -> i32 {
        self.tick_timer
    }

    /// `FoodData.needsFood()` (`FoodData.java:96-98`).
    pub const fn needs_food(&self) -> bool {
        self.food_level < MAX_FOOD_LEVEL
    }

    /// `FoodData.hasEnoughFood()` (`FoodData.java:92-94`).
    ///
    /// Vanilla compares the `int` against the `float` literal `6.0F`, so the
    /// comparison is `> 6` for every value an `int` can take; it is written as
    /// the integer test rather than reproducing the widening, which cannot
    /// change the answer.
    pub const fn has_enough_food(&self) -> bool {
        self.food_level > 6
    }

    /// `FoodData.setFoodLevel(int)` (`FoodData.java:108-110`): a raw write with
    /// no clamp, which is what the client's `SetHealth` handler and the
    /// PEACEFUL refill both use.
    pub const fn set_food_level(&mut self, food_level: i32) {
        self.food_level = food_level;
    }

    /// `FoodData.setSaturation(float)` (`FoodData.java:112-114`).
    pub const fn set_saturation(&mut self, saturation_level: f32) {
        self.saturation_level = saturation_level;
    }

    /// `FoodData.addExhaustion(float)` (`FoodData.java:100-102`).
    ///
    /// This is the raw accumulator: it has no ability gate of its own, which is
    /// why `FoodData.tick`'s own two calls (`:50`, `:57`) charge a creative
    /// player as well. The gate belongs to `Player.causeFoodExhaustion` --
    /// see [`cause_food_exhaustion`].
    pub fn add_exhaustion(&mut self, amount: f32) {
        if amount.is_nan() {
            return;
        }
        self.exhaustion_level = (self.exhaustion_level + amount).min(MAX_EXHAUSTION_LEVEL);
    }

    /// `FoodData.add(int, float)` (`FoodData.java:19-22`).
    ///
    /// The saturation clamp's upper bound is the food level the line above just
    /// wrote (`:21`), not the one the call started with.
    fn add(&mut self, food: i32, saturation: f32) {
        self.food_level = self
            .food_level
            .saturating_add(food)
            .clamp(0, MAX_FOOD_LEVEL);
        let current = if self.saturation_level.is_finite() {
            self.saturation_level
        } else {
            0.0
        };
        let delta = if saturation.is_nan() { 0.0 } else { saturation };
        let next = current + delta;
        self.saturation_level = if next.is_nan() {
            current.clamp(0.0, self.food_level as f32)
        } else {
            next.clamp(0.0, self.food_level as f32)
        };
    }

    /// `FoodData.eat(int, float)` (`FoodData.java:24-26`) over
    /// `FoodConstants.saturationByModifier` (`FoodConstants.java:30-32`).
    pub fn eat(&mut self, nutrition: i32, saturation_modifier: f32) {
        self.add(
            nutrition,
            saturation_by_modifier(nutrition, saturation_modifier),
        );
    }

    /// `FoodData.eat(FoodProperties)` (`FoodData.java:28-30`), whose second
    /// argument is the already-multiplied absolute saturation the component
    /// carries rather than a modifier.
    pub fn eat_food_properties(&mut self, nutrition: i32, saturation: f32) {
        self.add(nutrition, saturation);
    }

    /// `FoodData.tick(ServerPlayer)` (`FoodData.java:32-72`).
    ///
    /// `health` is `player.getHealth()`, read once at the top exactly as
    /// vanilla reads it inside the branch (`:63`): nothing between the two
    /// points can move it. `is_hurt` is `Player.isHurt()`
    /// (`Player.java:1585-1587`), which is `health > 0 && health < maxHealth`
    /// and therefore derivable from the same reading; it is computed here so a
    /// caller cannot pass a third opinion of the player's health.
    ///
    /// `natural_health_regeneration` is
    /// `GameRules.NATURAL_HEALTH_REGENERATION` (`GameRules.java:62`), renamed
    /// from the pre-26.1 `naturalRegeneration`.
    pub fn tick(
        &mut self,
        difficulty: Difficulty,
        natural_health_regeneration: bool,
        health: f32,
        max_health: f32,
    ) -> FoodTickEffect {
        // `:35-42`, the exhaustion cascade: four points of exhaustion buy one
        // point of saturation, or one point of food once the saturation is
        // gone. PEACEFUL stops at the saturation half.
        if self.exhaustion_level > EXHAUSTION_DROP {
            self.exhaustion_level -= EXHAUSTION_DROP;
            if self.saturation_level > 0.0 {
                self.saturation_level = (self.saturation_level - SATURATION_PER_DROP).max(0.0);
            } else if difficulty != Difficulty::Peaceful {
                self.food_level = (self.food_level - 1).max(0);
            }
        }

        let is_hurt = health > 0.0 && health < max_health;

        // `:45-52`.
        if natural_health_regeneration
            && self.saturation_level > 0.0
            && is_hurt
            && self.food_level >= SATURATED_REGEN_FOOD_LEVEL
        {
            self.tick_timer += 1;
            if self.tick_timer >= SATURATED_REGEN_TICKS {
                let saturation_spent = self.saturation_level.min(SATURATED_REGEN_MAX_SPEND);
                self.add_exhaustion(saturation_spent);
                self.tick_timer = 0;
                // `:49` heals before `:50` charges the exhaustion, and the two
                // do not read each other, so ordering the charge first here
                // changes nothing the caller can see.
                return FoodTickEffect::Heal(saturation_spent / SATURATED_REGEN_MAX_SPEND);
            }
            return FoodTickEffect::Nothing;
        }

        // `:53-59`.
        if natural_health_regeneration && self.food_level >= SLOW_REGEN_FOOD_LEVEL && is_hurt {
            self.tick_timer += 1;
            if self.tick_timer >= SLOW_REGEN_TICKS {
                self.add_exhaustion(SLOW_REGEN_EXHAUSTION);
                self.tick_timer = 0;
                return FoodTickEffect::Heal(SLOW_REGEN_HEAL);
            }
            return FoodTickEffect::Nothing;
        }

        // `:60-68`.
        if self.food_level <= 0 {
            self.tick_timer += 1;
            if self.tick_timer >= STARVE_TICKS {
                self.tick_timer = 0;
                // `:63`: the two lower difficulties floor the damage, HARD
                // does not, and PEACEFUL cannot reach this branch at all
                // because `:39` never lets the food level fall there.
                let starves = health > STARVE_FLOOR_EASY
                    || difficulty == Difficulty::Hard
                    || (health > STARVE_FLOOR_NORMAL && difficulty == Difficulty::Normal);
                if starves {
                    return FoodTickEffect::Starve;
                }
            }
            return FoodTickEffect::Nothing;
        }

        // `:69-71`.
        self.tick_timer = 0;
        FoodTickEffect::Nothing
    }
}

/// `FoodConstants.saturationByModifier(int, float)`
/// (`FoodConstants.java:30-32`).
pub fn saturation_by_modifier(nutrition: i32, modifier: f32) -> f32 {
    nutrition as f32 * modifier * 2.0
}

/// `Player.causeFoodExhaustion(float)` (`Player.java:1561-1567`).
///
/// The gate is `abilities.invulnerable`, not the game type and not
/// `instabuild`: creative and spectator both get it from
/// `GameType.updatePlayerAbilities` (`GameType.java:66`, `:70`), so both pay no
/// exhaustion, while a survival session with a saved `Invulnerable`-style
/// ability bit would pay none either. The `isClientSide` half (`:1563`) is
/// always false here.
pub fn cause_food_exhaustion(food: &mut FoodData, abilities_invulnerable: bool, amount: f32) {
    if !abilities_invulnerable {
        food.add_exhaustion(amount);
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;

    #[test]
    fn eat_food_properties_saturates_overflow_and_ignores_nan_saturation() {
        let mut food = FoodData::created();
        food.eat_food_properties(i32::MAX, f32::NAN);
        assert_eq!(food.food_level(), MAX_FOOD_LEVEL);
        assert!(food.saturation_level().is_finite());
        assert_eq!(food.saturation_level(), 5.0);

        food.eat_food_properties(0, f32::INFINITY);
        assert_eq!(food.saturation_level(), MAX_FOOD_LEVEL as f32);

        food.add_exhaustion(f32::NAN);
        assert!(food.exhaustion_level().is_finite());
    }
}

#[cfg(test)]
mod tests;
