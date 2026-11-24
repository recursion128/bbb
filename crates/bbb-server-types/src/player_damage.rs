//! Persisted `LivingEntity` player health, absorption, and hurt/death timers.

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
};

use crate::interaction::player_attribute_value;

/// The player's base `minecraft:max_health` from `DefaultAttributes` (`Player.createAttributes`).
pub fn player_max_health() -> f32 {
    player_attribute_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        .expect("the generated catalog carries the player's max health") as f32
}

/// The player's base `minecraft:max_absorption` (`0.0` before `MobEffects.ABSORPTION` modifiers).
pub fn player_max_absorption() -> f32 {
    player_attribute_value(VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID)
        .expect("the generated catalog carries the player's max absorption") as f32
}

/// `LivingEntity.hurtDuration` (`LivingEntity.java:1208`).
pub const HURT_DURATION: i32 = 10;

/// Full invulnerability window on hit (`LivingEntity.java:1206`).
pub const INVULNERABLE_TICKS: i32 = 20;

/// Half-window threshold (`LivingEntity.java:1196`).
pub const INVULNERABLE_TICKS_HALF: i32 = 10;

/// Player damage attribution window (`LivingEntity.java:1340`).
pub const LAST_HURT_BY_PLAYER_MEMORY_TICKS: i32 = 100;

/// `LivingEntity.tickDeath` removal tick (`LivingEntity.java:549-555`).
pub const DEATH_TICKS: i32 = 20;

/// The `LivingEntity` fields a hurt reads and writes, and `playerdata` persists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerDamageState {
    /// `DATA_HEALTH_ID` (`LivingEntity.getHealth`, `LivingEntity.java:1142-1144`).
    pub health: f32,
    /// `DATA_PLAYER_ABSORPTION_ID` (`Player.getAbsorptionAmount`, `Player.java:1699-1701`).
    pub absorption_amount: f32,
    /// `Entity.invulnerableTime` (`Entity.java:252`).
    pub invulnerable_time: i32,
    /// `LivingEntity.lastHurt` (`LivingEntity.java:232`, runtime-only).
    pub last_hurt: f32,
    /// `LivingEntity.hurtTime` (`LivingEntity.java:215`).
    pub hurt_time: i32,
    /// `LivingEntity.deathTime` (`LivingEntity.java:217`).
    pub death_time: i32,
}

impl PlayerDamageState {
    /// Freshly created `LivingEntity` damage state (`LivingEntity.java:275`).
    pub fn created() -> Self {
        Self {
            health: player_max_health(),
            absorption_amount: 0.0,
            invulnerable_time: 0,
            last_hurt: 0.0,
            hurt_time: 0,
            death_time: 0,
        }
    }

    /// `LivingEntity.isDeadOrDying` (`LivingEntity.java:1150-1152`).
    pub fn is_dead_or_dying(&self) -> bool {
        self.health <= 0.0
    }

    /// `LivingEntity.setHealth` (`LivingEntity.java:1146-1148`).
    pub fn set_health(&mut self, health: f32, max_health: f32) {
        self.health = health.clamp(0.0, max_health);
    }

    /// `LivingEntity.heal(float)` (`LivingEntity.java:1135-1140`).
    pub fn heal(&mut self, amount: f32, max_health: f32) {
        if self.health > 0.0 {
            self.set_health(self.health + amount, max_health);
        }
    }

    /// `LivingEntity.setAbsorptionAmount` (`LivingEntity.java:3333-3335`).
    pub fn set_absorption_amount(&mut self, absorption: f32, max_absorption: f32) {
        self.absorption_amount = absorption.clamp(0.0, max_absorption);
    }
}
