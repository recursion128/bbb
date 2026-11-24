//! Synchronous block ground-contact dispatch for server-owned bodies.

use crate::random::GameRandom;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride,
    VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID, VANILLA_BLOCK_DEEPSLATE_REDSTONE_ORE_ID,
    VANILLA_BLOCK_FARMLAND_ID, VANILLA_BLOCK_HAY_BLOCK_ID, VANILLA_BLOCK_HONEY_BLOCK_ID,
    VANILLA_BLOCK_MAGMA_BLOCK_ID, VANILLA_BLOCK_POINTED_DRIPSTONE_ID, VANILLA_BLOCK_POWDER_SNOW_ID,
    VANILLA_BLOCK_REDSTONE_ORE_ID, VANILLA_BLOCK_SCULK_SENSOR_ID, VANILLA_BLOCK_SCULK_SHRIEKER_ID,
    VANILLA_BLOCK_SLIME_BLOCK_ID, VANILLA_BLOCK_TURTLE_EGG_ID,
};
use bbb_protocol::packets::{Direction, Vec3d};

use bbb_protocol::block_sound_facts_26_1::vanilla_block_state_sound_type_26_1;

use super::{block_type_of, direction_property, string_property};

/// The body facts the step and landing rules read.
#[derive(Debug, Clone, Copy)]
pub struct GroundBody {
    pub living: bool,
    pub player: bool,
    pub zombie: bool,
    pub turtle_or_bat: bool,
    /// `isSteppingCarefully` and `isSuppressingBounce`, both the shift-key flag.
    pub careful: bool,
    /// `None` when the rule value is unavailable.
    pub mob_griefing: Option<bool>,
    pub width: f32,
    pub height: f32,
}

impl GroundBody {
    fn may_break_blocks(self) -> Option<bool> {
        Some(self.living && (self.player || self.mob_griefing?))
    }
}

/// What standing on a block asks of the body owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepOn {
    None,
    Hurt(&'static str, f32),
    Movement(Vec3d),
    DecreaseTurtleEggs,
    InteractRedstoneOre,
    SculkSensor,
    SculkShrieker,
}

// Block.java:463-464: the base callback is empty; unowned overrides remain unavailable.
pub fn step_on(state: i32, body: GroundBody, delta: Vec3d, random: &GameRandom) -> Option<StepOn> {
    step_on_with_registry(
        state,
        body,
        delta,
        random,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn step_on_with_registry(
    state: i32,
    body: GroundBody,
    delta: Vec3d,
    random: &GameRandom,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<StepOn> {
    let Some(block_id) = block_type_of(state) else {
        registry.block_by_state_id(state)?;
        return Some(StepOn::None);
    };
    Some(match block_id {
        // MagmaBlock.java:24-30.
        VANILLA_BLOCK_MAGMA_BLOCK_ID if !body.careful && body.living => {
            StepOn::Hurt("minecraft:hot_floor", 1.0)
        }
        // SlimeBlock.java:50-58.
        VANILLA_BLOCK_SLIME_BLOCK_ID if delta.y.abs() < 0.1 && !body.careful => {
            let scale = 0.4 + delta.y.abs() * 0.2;
            StepOn::Movement(Vec3d {
                x: delta.x * scale,
                y: delta.y,
                z: delta.z * scale,
            })
        }
        // TurtleEggBlock.java:56-62; TurtleEggBlock.java:73-79; TurtleEggBlock.java:177-183.
        VANILLA_BLOCK_TURTLE_EGG_ID
            if !body.careful
                && !body.turtle_or_bat
                && body.may_break_blocks()?
                && random.next_int(100) == 0 =>
        {
            StepOn::DecreaseTurtleEggs
        }
        // RedStoneOreBlock.java:45-51.
        VANILLA_BLOCK_REDSTONE_ORE_ID | VANILLA_BLOCK_DEEPSLATE_REDSTONE_ORE_ID
            if !body.careful =>
        {
            StepOn::InteractRedstoneOre
        }
        VANILLA_BLOCK_SCULK_SENSOR_ID | VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID => {
            StepOn::SculkSensor
        }
        VANILLA_BLOCK_SCULK_SHRIEKER_ID => StepOn::SculkShrieker,
        _ => StepOn::None,
    })
}

/// The block effect a landing applies before its fall damage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FallOnBlock {
    None,
    TrampleFarmland,
    DecreaseTurtleEggs,
    FallSound {
        big: bool,
    },
    /// The honey slide sound and entity event 54.
    HoneySlide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FallDamage {
    pub distance: f64,
    pub modifier: f32,
    pub damage_type: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FallOn {
    pub block: FallOnBlock,
    pub damage: Option<FallDamage>,
    /// Played only when the fall damage applied (`HoneyBlock.java:52-60`).
    pub damaged_sound: Option<(&'static str, f32, f32)>,
}

impl FallOn {
    fn damage(distance: f64, modifier: f32, damage_type: &'static str) -> Self {
        Self::after(FallOnBlock::None, distance, modifier, damage_type)
    }

    fn after(block: FallOnBlock, distance: f64, modifier: f32, damage_type: &'static str) -> Self {
        Self {
            block,
            damage: Some(FallDamage {
                distance,
                modifier,
                damage_type,
            }),
            damaged_sound: None,
        }
    }
}

/// `Block.fallOn` (`Block.java:494-496`) and its 26.1 overrides.
pub fn fall_on(state: i32, distance: f64, body: GroundBody, random: &GameRandom) -> Option<FallOn> {
    fall_on_with_registry(
        state,
        distance,
        body,
        random,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn fall_on_with_registry(
    state: i32,
    distance: f64,
    body: GroundBody,
    random: &GameRandom,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<FallOn> {
    let Some(block_id) = block_type_of(state) else {
        registry.block_by_state_id(state)?;
        return Some(FallOn::damage(distance, 1.0, "minecraft:fall"));
    };
    let block = vanilla_block_type(block_id)?;
    Some(match block_id {
        // `FarmlandBlock.java:109-119`: the level draw precedes every body gate.
        VANILLA_BLOCK_FARMLAND_ID => {
            let trample = f64::from(random.next_float()) < distance - 0.5
                && body.may_break_blocks()?
                && body.width * body.width * body.height > 0.512_f32;
            FallOn::after(
                if trample {
                    FallOnBlock::TrampleFarmland
                } else {
                    FallOnBlock::None
                },
                distance,
                1.0,
                "minecraft:fall",
            )
        }
        // `TurtleEggBlock.java:65-79`; `TurtleEggBlock.java:177-183`.
        VANILLA_BLOCK_TURTLE_EGG_ID => {
            let decrease = !body.zombie
                && !body.turtle_or_bat
                && body.may_break_blocks()?
                && random.next_int(3) == 0;
            FallOn::after(
                if decrease {
                    FallOnBlock::DecreaseTurtleEggs
                } else {
                    FallOnBlock::None
                },
                distance,
                1.0,
                "minecraft:fall",
            )
        }
        // `SlimeBlock.java:26-30`: suppression does not fall through to base.
        VANILLA_BLOCK_SLIME_BLOCK_ID => FallOn {
            block: FallOnBlock::None,
            damage: (!body.careful).then_some(FallDamage {
                distance,
                modifier: 0.0,
                damage_type: "minecraft:fall",
            }),
            damaged_sound: None,
        },
        // `PowderSnowBlock.java:101-107`.
        VANILLA_BLOCK_POWDER_SNOW_ID => FallOn {
            block: if distance.partial_cmp(&4.0) != Some(std::cmp::Ordering::Less) && body.living {
                FallOnBlock::FallSound {
                    big: distance.partial_cmp(&7.0) != Some(std::cmp::Ordering::Less),
                }
            } else {
                FallOnBlock::None
            },
            damage: None,
            damaged_sound: None,
        },
        // `HoneyBlock.java:52-60`.
        VANILLA_BLOCK_HONEY_BLOCK_ID => {
            let sound = vanilla_block_state_sound_type_26_1(state);
            FallOn {
                damaged_sound: Some((sound.fall_sound, sound.volume * 0.5, sound.pitch * 0.75)),
                ..FallOn::after(FallOnBlock::HoneySlide, distance, 0.2, "minecraft:fall")
            }
        }
        // `HayBlock.java:25-27`.
        VANILLA_BLOCK_HAY_BLOCK_ID => FallOn::damage(distance, 0.2, "minecraft:fall"),
        // `PointedDripstoneBlock.java:155-161`.
        VANILLA_BLOCK_POINTED_DRIPSTONE_ID => {
            let up = direction_property(state, "vertical_direction")? == Direction::Up;
            let tip = string_property(state, "thickness")? == "tip";
            if up && tip {
                FallOn::damage(distance + 2.5, 2.0, "minecraft:stalagmite")
            } else {
                FallOn::damage(distance, 1.0, "minecraft:fall")
            }
        }
        // `BedBlock.java:133-135`: scale distance before entity calculation.
        _ if block.use_without_item_override
            == Some(VanillaBlockUseWithoutItemOverride::BedBlock) =>
        {
            FallOn::damage(distance * 0.5, 1.0, "minecraft:fall")
        }
        _ => FallOn::damage(distance, 1.0, "minecraft:fall"),
    })
}

/// `Block.java:498-500`; `SlimeBlock.java:33-47`; `BedBlock.java:138-152`.
pub fn movement_after_fall_on(
    state: i32,
    is_living: bool,
    suppressing_bounce: bool,
    movement: Vec3d,
) -> Option<Vec3d> {
    movement_after_fall_on_with_registry(
        state,
        is_living,
        suppressing_bounce,
        movement,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn movement_after_fall_on_with_registry(
    state: i32,
    is_living: bool,
    suppressing_bounce: bool,
    movement: Vec3d,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<Vec3d> {
    let Some(block_id) = block_type_of(state) else {
        registry.block_by_state_id(state)?;
        let mut result = movement;
        result.y *= 0.0;
        return Some(result);
    };
    let block = vanilla_block_type(block_id)?;
    let bed = block.use_without_item_override == Some(VanillaBlockUseWithoutItemOverride::BedBlock);
    let mut result = movement;
    if suppressing_bounce || (block_id != VANILLA_BLOCK_SLIME_BLOCK_ID && !bed) {
        result.y *= 0.0;
    } else if movement.y < 0.0 {
        let living_factor = if is_living { 1.0 } else { 0.8 };
        result.y = if bed {
            -movement.y * f64::from(0.66_f32) * living_factor
        } else {
            -movement.y * living_factor
        };
    }
    Some(result)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod step_tests;

#[cfg(test)]
mod velocity_tests;
