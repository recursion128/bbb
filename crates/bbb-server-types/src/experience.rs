//! Experience values, block-break experience providers, orb splitting, and
//! player experience arithmetic.

use bbb_facts::block_types::vanilla_block_type;

/// `ExperienceOrb.LIFETIME` (`ExperienceOrb.java:32`).
pub const LIFETIME: i32 = 6000;

/// One block's `Block.tryDropExperience` `IntProvider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakExperience {
    /// `ConstantInt.of(value)`, whose `sample` draws nothing.
    Constant(i32),
    /// `UniformInt.of(min, max)`, whose `sample` is
    /// `Mth.randomBetweenInclusive` (`UniformInt.java:28-30`, `Mth.java:664-666`).
    Uniform(i32, i32),
    /// `SpawnerBlock.spawnAfterBreak` (`SpawnerBlock.java:40-47`):
    /// `15 + random.nextInt(15) + random.nextInt(15)`, handed straight to
    /// `popExperience` rather than through `tryDropExperience`, so no
    /// enchantment ever modifies it.
    SpawnerMagicCount,
}

impl BreakExperience {
    /// The provider's `sample(level.getRandom())`, drawing from `next_int`
    /// exactly as often as vanilla does.
    pub fn sample(self, mut next_int: impl FnMut(i32) -> i32) -> i32 {
        match self {
            // `ConstantInt.sample` (`ConstantInt.java:19-21`) answers without
            // touching the generator.
            Self::Constant(value) => value,
            Self::Uniform(min, max) => next_int(max - min + 1) + min,
            Self::SpawnerMagicCount => 15 + next_int(15) + next_int(15),
        }
    }

    /// Whether the amount goes through `EnchantmentHelper.processBlockExperience`
    /// (`Block.java:621`), which is what lets silk touch zero it.
    #[must_use]
    pub const fn is_enchantment_modified(self) -> bool {
        !matches!(self, Self::SpawnerMagicCount)
    }
}

/// Every block whose break drops experience, by registry id (`Blocks.java`,
/// `RedStoneOreBlock.java:94-99`, `SculkBlock.java:23-25`,
/// `SculkCatalystBlock.java:60-65`, `SculkSensorBlock.java:288-293`,
/// `SculkShriekerBlock.java:127-132`, `SpawnerBlock.java:40-47`).
pub const BREAK_EXPERIENCE: &[(&str, BreakExperience)] = &[
    ("minecraft:coal_ore", BreakExperience::Uniform(0, 2)),
    ("minecraft:copper_ore", BreakExperience::Constant(0)),
    (
        "minecraft:deepslate_coal_ore",
        BreakExperience::Uniform(0, 2),
    ),
    (
        "minecraft:deepslate_copper_ore",
        BreakExperience::Constant(0),
    ),
    (
        "minecraft:deepslate_diamond_ore",
        BreakExperience::Uniform(3, 7),
    ),
    (
        "minecraft:deepslate_emerald_ore",
        BreakExperience::Uniform(3, 7),
    ),
    ("minecraft:deepslate_gold_ore", BreakExperience::Constant(0)),
    ("minecraft:deepslate_iron_ore", BreakExperience::Constant(0)),
    (
        "minecraft:deepslate_lapis_ore",
        BreakExperience::Uniform(2, 5),
    ),
    ("minecraft:diamond_ore", BreakExperience::Uniform(3, 7)),
    ("minecraft:emerald_ore", BreakExperience::Uniform(3, 7)),
    ("minecraft:gold_ore", BreakExperience::Constant(0)),
    ("minecraft:iron_ore", BreakExperience::Constant(0)),
    ("minecraft:lapis_ore", BreakExperience::Uniform(2, 5)),
    ("minecraft:nether_gold_ore", BreakExperience::Uniform(0, 1)),
    (
        "minecraft:nether_quartz_ore",
        BreakExperience::Uniform(2, 5),
    ),
    (
        "minecraft:deepslate_redstone_ore",
        BreakExperience::Uniform(1, 5),
    ),
    ("minecraft:redstone_ore", BreakExperience::Uniform(1, 5)),
    ("minecraft:sculk", BreakExperience::Constant(1)),
    (
        "minecraft:calibrated_sculk_sensor",
        BreakExperience::Constant(5),
    ),
    ("minecraft:sculk_catalyst", BreakExperience::Constant(5)),
    ("minecraft:sculk_sensor", BreakExperience::Constant(5)),
    ("minecraft:sculk_shrieker", BreakExperience::Constant(5)),
    ("minecraft:spawner", BreakExperience::SpawnerMagicCount),
];

/// The `IntProvider` `Block.tryDropExperience` gets for one block registry id,
/// or `None` for a block whose break drops none (`CreakingHeartBlock` awards
/// experience from `playerWillDestroy`, not `spawnAfterBreak`).
#[must_use]
pub fn block_break_experience(block_type_id: i32) -> Option<BreakExperience> {
    let resource_id = vanilla_block_type(block_type_id)?.resource_id;
    BREAK_EXPERIENCE
        .iter()
        .find(|(id, _)| *id == resource_id)
        .map(|(_, experience)| *experience)
}

/// `ExperienceOrb.getExperienceValue(int)` (`ExperienceOrb.java:346-368`): the
/// largest orb `awardWithDirection` may cut off the remaining amount.
#[must_use]
pub const fn experience_value(max_value: i32) -> i32 {
    if max_value >= 2477 {
        2477
    } else if max_value >= 1237 {
        1237
    } else if max_value >= 617 {
        617
    } else if max_value >= 307 {
        307
    } else if max_value >= 149 {
        149
    } else if max_value >= 73 {
        73
    } else if max_value >= 37 {
        37
    } else if max_value >= 17 {
        17
    } else if max_value >= 7 {
        7
    } else if max_value >= 3 {
        3
    } else {
        1
    }
}

/// `Player.experienceLevel`, `experienceProgress`, `totalExperience`,
/// `enchantmentSeed` (`Player.java:160-163`) and the death-screen score
/// (`DATA_SCORE_ID`, `:135`), with the methods that move them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerExperience {
    level: i32,
    progress: f32,
    total: i32,
    enchantment_seed: i32,
    score: i32,
    /// The level a rise reached on a multiple of five, which the session
    /// owner turns into the level-up sound (`:1546-1550`).
    pending_level_up: Option<i32>,
}

impl Default for PlayerExperience {
    fn default() -> Self {
        Self::created()
    }
}

impl PlayerExperience {
    /// The field initialisers (`Player.java:160-162`).
    #[must_use]
    pub const fn created() -> Self {
        Self {
            level: 0,
            progress: 0.0,
            total: 0,
            enchantment_seed: 0,
            score: 0,
            pending_level_up: None,
        }
    }

    /// `Player.readAdditionalSaveData`'s three reads
    /// (`Player.java:629-631`), each with its own default.
    #[must_use]
    pub const fn from_saved(progress: f32, level: i32, total: i32) -> Self {
        Self {
            level,
            progress,
            total,
            enchantment_seed: 0,
            score: 0,
            pending_level_up: None,
        }
    }

    /// `XpSeed` (`Player.java:632-635`, `:655`).
    #[must_use]
    pub const fn with_enchantment_seed(mut self, seed: i32) -> Self {
        self.enchantment_seed = seed;
        self
    }

    /// `Score` (`Player.java:637`, `:656`).
    #[must_use]
    pub const fn with_score(mut self, score: i32) -> Self {
        self.score = score;
        self
    }

    #[must_use]
    pub const fn enchantment_seed(&self) -> i32 {
        self.enchantment_seed
    }

    /// `Player.onEnchantmentPerformed` (`Player.java:1527-1536`), with the
    /// next seed drawn by the caller.
    pub fn on_enchantment_performed(&mut self, cost: i32, next_seed: i32) {
        self.level -= cost;
        if self.level < 0 {
            self.level = 0;
            self.progress = 0.0;
            self.total = 0;
        }
        self.enchantment_seed = next_seed;
    }

    #[must_use]
    pub const fn level(&self) -> i32 {
        self.level
    }

    #[must_use]
    pub const fn progress(&self) -> f32 {
        self.progress
    }

    #[must_use]
    pub const fn total(&self) -> i32 {
        self.total
    }

    /// `Player.getXpNeededForNextLevel()` (`Player.java:1553-1559`), using
    /// saturating arithmetic so extreme saved/command levels never overflow.
    #[must_use]
    pub const fn xp_needed_for_next_level(&self) -> i32 {
        if self.level >= 30 {
            let delta = self.level.saturating_sub(30);
            delta.saturating_mul(9).saturating_add(112)
        } else if self.level >= 15 {
            37 + (self.level - 15) * 5
        } else if self.level <= 0 {
            7
        } else {
            7 + self.level * 2
        }
    }

    #[must_use]
    pub const fn score(&self) -> i32 {
        self.score
    }

    /// The level of the first rise onto a multiple of five since the last
    /// call; `lastLevelUpTime` (`Player.java:1546`) is the caller's to check.
    pub fn take_pending_level_up(&mut self) -> Option<i32> {
        self.pending_level_up.take()
    }

    /// `Player.giveExperienceLevels(int)` (`Player.java:1538-1551`).
    pub fn give_experience_levels(&mut self, amount: i32) {
        self.level = self.level.saturating_add(amount);
        if self.level < 0 {
            self.level = 0;
            self.progress = 0.0;
            self.total = 0;
        }
        if amount > 0 && self.level % 5 == 0 {
            self.pending_level_up.get_or_insert(self.level);
        }
    }

    /// `Player.giveExperiencePoints(int)` (`Player.java:1500-1521`).
    ///
    /// Both loops are transcribed as vanilla wrote them, float arithmetic
    /// included: `experienceProgress` is a `float`, and the divisions are the
    /// ones that decide when a level ticks over.
    pub fn give_experience_points(&mut self, amount: i32) {
        // `increaseScore` (`:1501`, `:505-508`).
        self.score = self.score.wrapping_add(amount);
        // `:1502-1503`.
        if !self.progress.is_finite() {
            self.progress = 0.0;
        }
        self.progress += amount as f32 / self.xp_needed_for_next_level() as f32;
        self.total = (self.total.saturating_add(amount)).clamp(0, i32::MAX);
        // `:1505-1514`.
        while self.progress < 0.0 {
            let remaining = self.progress * self.xp_needed_for_next_level() as f32;
            if self.level > 0 {
                self.give_experience_levels(-1);
                self.progress = 1.0 + remaining / self.xp_needed_for_next_level() as f32;
            } else {
                self.give_experience_levels(-1);
                self.progress = 0.0;
            }
        }
        // `:1516-1520`.
        while self.progress >= 1.0 {
            if self.level == i32::MAX {
                let limit = self.xp_needed_for_next_level() as f32;
                self.progress = ((limit - 1.0) / limit).clamp(0.0, 1.0 - f32::EPSILON);
                break;
            }
            self.progress = (self.progress - 1.0) * self.xp_needed_for_next_level() as f32;
            self.give_experience_levels(1);
            self.progress /= self.xp_needed_for_next_level() as f32;
        }
    }

    /// `ServerPlayer.setExperiencePoints(int)` (`ServerPlayer.java:509-517`):
    /// the progress bar is set to `amount` out of the current level's
    /// requirement, clamped below the next level, and the answer is whether it
    /// moved -- the condition `lastSentExp = -1` is written under (`:513-516`),
    /// which the session owner applies.
    ///
    /// The clamp is `Mth.clamp(amount / limit, 0, (limit - 1) / limit)` in
    /// single precision (`:510-512`), so a point count at or above the
    /// requirement lands one point short of it rather than rolling over --
    /// `/experience set .. points` refuses such a count before it gets here
    /// (ExperienceCommand.java:177-179), but the method itself does not.
    pub fn set_experience_points(&mut self, amount: i32) -> bool {
        let limit = self.xp_needed_for_next_level() as f32;
        let max = (limit - 1.0) / limit;
        let progress = (amount as f32 / limit).clamp(0.0, max);
        if progress == self.progress {
            return false;
        }
        self.progress = progress;
        true
    }

    /// `ServerPlayer.setExperienceLevels(int)` (`ServerPlayer.java:519-524`):
    /// the level alone moves, and the answer is whether it did (`:520`).
    ///
    /// Neither the progress nor the total is touched, which is why
    /// `/experience set .. levels` leaves the bar where it was: vanilla's own
    /// `totalExperience` is only ever a running sum (`Player.java:1503`).
    pub fn set_experience_levels(&mut self, amount: i32) -> bool {
        if amount == self.level {
            return false;
        }
        self.level = amount;
        true
    }
}

#[cfg(test)]
mod tests;
