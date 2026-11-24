use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_SALMON_ID,
    VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use crate::entity::general_living::{is_abstract_horse_entity_type, is_water_animal_entity_type};
use crate::entity::natural_spawn_facts_26_1::{
    vanilla_natural_spawn_type_facts_by_id_26_1, VanillaNaturalSpawnTypeFacts26_1,
    VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1,
};

const MOB: &str = "net.minecraft.world.entity.Mob";
const PATHFINDER_MOB: &str = "net.minecraft.world.entity.PathfinderMob";
const ANIMAL: &str = "net.minecraft.world.entity.animal.Animal";
const AXOLOTL: &str = "net.minecraft.world.entity.animal.axolotl.Axolotl";
const BEE: &str = "net.minecraft.world.entity.animal.bee.Bee";
const MUSHROOM_COW: &str = "net.minecraft.world.entity.animal.cow.MushroomCow";
const HAPPY_GHAST: &str = "net.minecraft.world.entity.animal.happyghast.HappyGhast";
const NAUTILUS: &str = "net.minecraft.world.entity.animal.nautilus.AbstractNautilus";
const TURTLE: &str = "net.minecraft.world.entity.animal.turtle.Turtle";
const ENDERMAN: &str = "net.minecraft.world.entity.monster.EnderMan";
const GIANT: &str = "net.minecraft.world.entity.monster.Giant";
const GUARDIAN: &str = "net.minecraft.world.entity.monster.Guardian";
const MONSTER: &str = "net.minecraft.world.entity.monster.Monster";
const SILVERFISH: &str = "net.minecraft.world.entity.monster.Silverfish";
const STRIDER: &str = "net.minecraft.world.entity.monster.Strider";
const CREAKING: &str = "net.minecraft.world.entity.monster.creaking.Creaking";
const HOGLIN: &str = "net.minecraft.world.entity.monster.hoglin.Hoglin";
const PILLAGER: &str = "net.minecraft.world.entity.monster.illager.Pillager";
const WARDEN: &str = "net.minecraft.world.entity.monster.warden.Warden";

const AGEABLE_WATER_CREATURE: &str = "net.minecraft.world.entity.animal.AgeableWaterCreature";
const OCELOT: &str = "net.minecraft.world.entity.animal.feline.Ocelot";
const WATER_ANIMAL: &str = "net.minecraft.world.entity.animal.fish.WaterAnimal";
const IRON_GOLEM: &str = "net.minecraft.world.entity.animal.golem.IronGolem";
const RAVAGER: &str = "net.minecraft.world.entity.monster.Ravager";
const DROWNED: &str = "net.minecraft.world.entity.monster.zombie.Drowned";
const ZOMBIFIED_PIGLIN: &str = "net.minecraft.world.entity.monster.zombie.ZombifiedPiglin";

const ENTITY_TYPE_COUNT: usize = VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1.len();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum WalkTargetRule {
    None,
    Zero,
    Animal,
    MushroomCow,
    HappyGhast,
    Turtle,
    Giant,
    Guardian,
    Monster,
    Silverfish,
    Strider,
    Hoglin,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SpawnObstructionRule {
    Standard,
    WaterOrLava,
    Ocelot,
    IronGolem,
    Ravager,
    Warden,
}

const fn str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn classify_walk_target_rule(owner: Option<&str>) -> WalkTargetRule {
    let Some(owner) = owner else {
        return WalkTargetRule::None;
    };
    if str_eq(owner, PATHFINDER_MOB)
        || str_eq(owner, AXOLOTL)
        || str_eq(owner, BEE)
        || str_eq(owner, NAUTILUS)
        || str_eq(owner, ENDERMAN)
        || str_eq(owner, PILLAGER)
        || str_eq(owner, WARDEN)
        || str_eq(owner, CREAKING)
    {
        WalkTargetRule::Zero
    } else if str_eq(owner, ANIMAL) {
        WalkTargetRule::Animal
    } else if str_eq(owner, MUSHROOM_COW) {
        WalkTargetRule::MushroomCow
    } else if str_eq(owner, HAPPY_GHAST) {
        WalkTargetRule::HappyGhast
    } else if str_eq(owner, TURTLE) {
        WalkTargetRule::Turtle
    } else if str_eq(owner, GIANT) {
        WalkTargetRule::Giant
    } else if str_eq(owner, GUARDIAN) {
        WalkTargetRule::Guardian
    } else if str_eq(owner, MONSTER) {
        WalkTargetRule::Monster
    } else if str_eq(owner, SILVERFISH) {
        WalkTargetRule::Silverfish
    } else if str_eq(owner, STRIDER) {
        WalkTargetRule::Strider
    } else if str_eq(owner, HOGLIN) {
        WalkTargetRule::Hoglin
    } else {
        WalkTargetRule::Unsupported
    }
}

const fn classify_obstruction_rule(owner: Option<&str>) -> Option<SpawnObstructionRule> {
    let Some(owner) = owner else {
        return None;
    };
    if str_eq(owner, MOB) || str_eq(owner, ZOMBIFIED_PIGLIN) {
        Some(SpawnObstructionRule::Standard)
    } else if str_eq(owner, AGEABLE_WATER_CREATURE)
        || str_eq(owner, AXOLOTL)
        || str_eq(owner, WATER_ANIMAL)
        || str_eq(owner, NAUTILUS)
        || str_eq(owner, GUARDIAN)
        || str_eq(owner, STRIDER)
        || str_eq(owner, DROWNED)
    {
        Some(SpawnObstructionRule::WaterOrLava)
    } else if str_eq(owner, OCELOT) {
        Some(SpawnObstructionRule::Ocelot)
    } else if str_eq(owner, IRON_GOLEM) {
        Some(SpawnObstructionRule::IronGolem)
    } else if str_eq(owner, RAVAGER) {
        Some(SpawnObstructionRule::Ravager)
    } else if str_eq(owner, WARDEN) {
        Some(SpawnObstructionRule::Warden)
    } else {
        None
    }
}

const WALK_TARGET_RULES: [Option<WalkTargetRule>; ENTITY_TYPE_COUNT] = {
    let mut table = [None; ENTITY_TYPE_COUNT];
    let mut i = 0;
    while i < ENTITY_TYPE_COUNT {
        let facts = &VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1[i];
        if facts.is_mob {
            table[i] = Some(classify_walk_target_rule(facts.walk_target_value_owner));
        }
        i += 1;
    }
    table
};

const OBSTRUCTION_RULES: [Option<Option<SpawnObstructionRule>>; ENTITY_TYPE_COUNT] = {
    let mut table = [None; ENTITY_TYPE_COUNT];
    let mut i = 0;
    while i < ENTITY_TYPE_COUNT {
        let facts = &VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1[i];
        if facts.is_mob {
            table[i] = Some(classify_obstruction_rule(
                facts.check_spawn_obstruction_owner,
            ));
        }
        i += 1;
    }
    table
};

pub struct NaturalSpawnRulesProbe<L, B, S> {
    pub pathfinding_cost_from_light_levels: L,
    /// Block state at the candidate (0) or directly below it (1).
    pub block_at: B,
    pub below_is_sand: S,
    pub air_at_and_below: [bool; 3],
    pub turtle_going_home: bool,
    pub strider_is_in_lava: bool,
    pub hoglin_near_repellent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalSpawnObstructionProbe {
    pub position: BlockPos,
    pub sea_level: i32,
    pub is_unobstructed: bool,
    pub contains_any_liquid: bool,
    pub default_dimensions_no_collision: bool,
    pub below_is_grass_or_leaves: bool,
    pub below_entity_can_stand: bool,
    pub at_is_valid_empty_spawn_block: bool,
    pub above_one_is_valid_empty_spawn_block: bool,
    pub above_two_is_valid_empty_spawn_block: bool,
}

impl Default for NaturalSpawnObstructionProbe {
    fn default() -> Self {
        Self {
            position: BlockPos { x: 0, y: 64, z: 0 },
            sea_level: 63,
            is_unobstructed: true,
            contains_any_liquid: false,
            default_dimensions_no_collision: true,
            below_is_grass_or_leaves: true,
            below_entity_can_stand: true,
            at_is_valid_empty_spawn_block: true,
            above_one_is_valid_empty_spawn_block: true,
            above_two_is_valid_empty_spawn_block: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NaturalSpawnEligibilityError {
    UnknownEntityType(i32),
    NotMob(i32),
    UnsupportedOperation,
    MissingTropicalFishSchoolState,
}

// `Mob.java:783-788`; `PathfinderMob.java:20-31`.
pub fn check_natural_spawn_rules(
    entity_type_id: i32,
    probe: NaturalSpawnRulesProbe<impl Fn() -> f32, impl Fn(usize) -> i32, impl Fn(i32) -> bool>,
) -> Result<bool, NaturalSpawnEligibilityError> {
    let rule = walk_target_rule(entity_type_id)?;
    Ok(rule == WalkTargetRule::None || walk_target_value::<true>(rule, probe)? >= 0.0)
}

// `PathfinderMob.getWalkTargetValue(BlockPos, LevelReader)`
// (`PathfinderMob.java:24-26`) and every override of it.
pub fn mob_walk_target_value(
    entity_type_id: i32,
    probe: NaturalSpawnRulesProbe<impl Fn() -> f32, impl Fn(usize) -> i32, impl Fn(i32) -> bool>,
) -> Result<f32, NaturalSpawnEligibilityError> {
    walk_target_value::<false>(walk_target_rule(entity_type_id)?, probe)
}

fn walk_target_rule(entity_type_id: i32) -> Result<WalkTargetRule, NaturalSpawnEligibilityError> {
    let index = usize::try_from(entity_type_id)
        .ok()
        .filter(|&index| index < ENTITY_TYPE_COUNT)
        .ok_or(NaturalSpawnEligibilityError::UnknownEntityType(
            entity_type_id,
        ))?;
    WALK_TARGET_RULES[index].ok_or(NaturalSpawnEligibilityError::NotMob(entity_type_id))
}

// Eligibility skips world reads that only distinguish nonnegative values.
fn walk_target_value<const SIGN_ONLY: bool>(
    rule: WalkTargetRule,
    probe: NaturalSpawnRulesProbe<impl Fn() -> f32, impl Fn(usize) -> i32, impl Fn(i32) -> bool>,
) -> Result<f32, NaturalSpawnEligibilityError> {
    let cost = &probe.pathfinding_cost_from_light_levels;
    let block_at = &probe.block_at;
    let below_is = |id| vanilla_block_state_block_type_id(block_at(1)) == Some(id);
    let fluid_at = || block_state_fluid_state(block_at(0)).family();
    let value = match rule {
        // `PathfinderMob.java:20-26`; `Axolotl.java:121-123`; `Bee.java:171-173`;
        // `AbstractNautilus.java:130-132`; `EnderMan.java:109-111`;
        // `Pillager.java:147-149`; `Warden.java:155-157`; `Creaking.java:514-516`.
        WalkTargetRule::Zero => 0.0,
        // `Animal.java:92-94`.
        WalkTargetRule::Animal => {
            if below_is(block_types::VANILLA_BLOCK_GRASS_BLOCK_ID) {
                10.0
            } else {
                cost()
            }
        }
        // `MushroomCow.java:65-67`.
        WalkTargetRule::MushroomCow => {
            if below_is(block_types::VANILLA_BLOCK_MYCELIUM_ID) {
                10.0
            } else {
                cost()
            }
        }
        // HappyGhast.java:178-184.
        WalkTargetRule::HappyGhast => {
            if SIGN_ONLY {
                0.0
            } else {
                match probe.air_at_and_below {
                    [false, _, _] => 0.0,
                    [true, true, false] => 10.0,
                    _ => 5.0,
                }
            }
        }
        // `Turtle.java:234-240`.
        WalkTargetRule::Turtle => {
            if (!probe.turtle_going_home && fluid_at() == Some(FluidFamily::Water))
                || (probe.below_is_sand)(block_at(1))
            {
                10.0
            } else {
                cost()
            }
        }
        // `Giant.java:24-26`.
        WalkTargetRule::Giant => cost(),
        // `Guardian.java:178-180`; `Monster.java:82-84`.
        WalkTargetRule::Guardian => {
            if fluid_at() == Some(FluidFamily::Water) {
                10.0 + cost()
            } else {
                -cost()
            }
        }
        WalkTargetRule::Monster => -cost(),
        // `Silverfish.java:108-110`.
        WalkTargetRule::Silverfish => {
            if crate::block_behavior::infested::is_compatible_host(block_at(1)) {
                10.0
            } else {
                -cost()
            }
        }
        // `Strider.java:375-381`.
        WalkTargetRule::Strider => {
            if SIGN_ONLY && !probe.strider_is_in_lava {
                0.0
            } else if fluid_at() == Some(FluidFamily::Lava) {
                10.0
            } else if probe.strider_is_in_lava {
                f32::NEG_INFINITY
            } else {
                0.0
            }
        }
        // `Hoglin.java:195-201`.
        WalkTargetRule::Hoglin => {
            if probe.hoglin_near_repellent {
                -1.0
            } else if !SIGN_ONLY && below_is(block_types::VANILLA_BLOCK_CRIMSON_NYLIUM_ID) {
                10.0
            } else {
                0.0
            }
        }
        WalkTargetRule::None | WalkTargetRule::Unsupported => {
            return Err(NaturalSpawnEligibilityError::UnsupportedOperation)
        }
    };
    Ok(value)
}

pub fn natural_spawn_obstruction_rule(
    entity_type_id: i32,
) -> Result<SpawnObstructionRule, NaturalSpawnEligibilityError> {
    let index = usize::try_from(entity_type_id)
        .ok()
        .filter(|&index| index < ENTITY_TYPE_COUNT)
        .ok_or(NaturalSpawnEligibilityError::UnknownEntityType(
            entity_type_id,
        ))?;
    OBSTRUCTION_RULES[index]
        .ok_or(NaturalSpawnEligibilityError::NotMob(entity_type_id))?
        .ok_or(NaturalSpawnEligibilityError::UnsupportedOperation)
}

// `Mob.java:787-789`.
pub fn check_natural_spawn_obstruction(
    entity_type_id: i32,
    probe: NaturalSpawnObstructionProbe,
) -> Result<bool, NaturalSpawnEligibilityError> {
    let result = match natural_spawn_obstruction_rule(entity_type_id)? {
        SpawnObstructionRule::Standard => probe.is_unobstructed && !probe.contains_any_liquid,
        // `AgeableWaterCreature.java:23-25`; `Axolotl.java:309-311`;
        // `WaterAnimal.java:25-27`; `AbstractNautilus.java:150-152`;
        // `Guardian.java:300-302`; `Strider.java:191-193`;
        // `Drowned.java:218-220`.
        SpawnObstructionRule::WaterOrLava => probe.is_unobstructed,
        // `Ocelot.java:232-246`.
        SpawnObstructionRule::Ocelot => {
            probe.is_unobstructed
                && !probe.contains_any_liquid
                && probe.position.y >= probe.sea_level
                && probe.below_is_grass_or_leaves
        }
        // `IronGolem.java:305-323`.
        SpawnObstructionRule::IronGolem => {
            probe.below_entity_can_stand
                && probe.at_is_valid_empty_spawn_block
                && probe.above_one_is_valid_empty_spawn_block
                && probe.above_two_is_valid_empty_spawn_block
                && probe.is_unobstructed
        }
        // `Ravager.java:320-322`.
        SpawnObstructionRule::Ravager => !probe.contains_any_liquid,
        // `Warden.java:150-152`.
        SpawnObstructionRule::Warden => {
            probe.is_unobstructed
                && !probe.contains_any_liquid
                && probe.default_dimensions_no_collision
        }
    };
    Ok(result)
}

// `Mob.java:791-793`.
pub fn natural_spawn_cluster_size(
    entity_type_id: i32,
) -> Result<i32, NaturalSpawnEligibilityError> {
    mob_facts(entity_type_id)?;
    Ok(match entity_type_id {
        // `Salmon.java:45-47`.
        VANILLA_ENTITY_TYPE_SALMON_ID => 5,
        // `HappyGhast.java:237-239`; `Ghast.java:156-158`;
        // `Pillager.java:152-154`.
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        | VANILLA_ENTITY_TYPE_GHAST_ID
        | VANILLA_ENTITY_TYPE_PILLAGER_ID => 1,
        // `Wolf.java:543-545`.
        VANILLA_ENTITY_TYPE_WOLF_ID => 8,
        // `AbstractHorse.java:388-390`.
        id if is_abstract_horse_entity_type(id) => 6,
        // `AbstractFish.java:59-61`; `AbstractSchoolingFish.java:29-35`.
        id if is_water_animal_entity_type(id) => 8,
        _ => 4,
    })
}

// `Mob.java:795-797`; `TropicalFish.java:124-126`.
pub fn natural_spawn_group_size_reached(
    entity_type_id: i32,
    tropical_fish_is_school: Option<bool>,
) -> Result<bool, NaturalSpawnEligibilityError> {
    mob_facts(entity_type_id)?;
    match entity_type_id {
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => tropical_fish_is_school
            .map(|is_school| !is_school)
            .ok_or(NaturalSpawnEligibilityError::MissingTropicalFishSchoolState),
        _ => Ok(false),
    }
}

fn mob_facts(
    entity_type_id: i32,
) -> Result<&'static VanillaNaturalSpawnTypeFacts26_1, NaturalSpawnEligibilityError> {
    let facts = vanilla_natural_spawn_type_facts_by_id_26_1(entity_type_id).ok_or(
        NaturalSpawnEligibilityError::UnknownEntityType(entity_type_id),
    )?;
    if !facts.is_mob {
        return Err(NaturalSpawnEligibilityError::NotMob(entity_type_id));
    }
    Ok(facts)
}

#[cfg(test)]
mod tests;
