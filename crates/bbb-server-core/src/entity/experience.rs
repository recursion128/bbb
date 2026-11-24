//! The `LivingEntity` experience facts a death listener reads:
//! `shouldDropExperience` (`LivingEntity.java:557-559`) and
//! `getExperienceReward` (`:581-583`) over `Mob.getBaseExperienceReward`
//! (`Mob.java:301-318`).

use crate::random::GameRandom;
use bbb_protocol::entity_types::{vanilla_entity_type, VanillaEntityMobCategory};
use bbb_protocol::equipment_slots::{EquipmentSlotType, VANILLA_EQUIPMENT_SLOTS};

use crate::entity::{Entity, EntityData, GeneralLivingData};
use crate::item_durability::{EnchantmentValueEffects, ValueContext};
use crate::loot_runtime::LootEntity;

/// `Chicken.getBaseExperienceReward` (`Chicken.java:194-196`).
const CHICKEN_JOCKEY_XP_REWARD: i32 = 10;
/// `Monster.xpReward = 5` (`Monster.java:34`), the value every monster class
/// inherits unless its own constructor sets another.
const MONSTER_XP_REWARD: i32 = 5;
/// `Zombie.getBaseExperienceReward` (`Zombie.java:178-184`): `(int)(xpReward *
/// 2.5)` for a baby.
const BABY_ZOMBIE_XP_REWARD: i32 = (MONSTER_XP_REWARD as f32 * 2.5) as i32;

/// `LivingEntity.shouldDropExperience()` (`LivingEntity.java:557-559`):
/// `!isBaby()`, under the three 26.1 overrides.
///
/// `Monster.shouldDropExperience` (`Monster.java:126-128`) and
/// `Hoglin.shouldDropExperience` (`Hoglin.java:229-231`, which `Zoglin`
/// inherits) answer true whatever the age is, so a baby monster and a baby
/// hoglin still pay; `Tadpole.shouldDropExperience` (`Tadpole.java:253-255`)
/// answers false, and a tadpole's `isBaby` is false too.
///
/// The monster category is how the generated catalog carries "extends
/// `Monster`". Every monster-category class outside that hierarchy is either a
/// type whose `isBaby` is always false (`Slime`, `MagmaCube`, `Ghast`,
/// `Phantom`, `Shulker`, `EnderDragon`, `CamelHusk.isBaby` `:76-78`,
/// `ZombieNautilus.isBaby` `:183-185`), one that overrides the answer to true
/// (`Hoglin`, `Zoglin`), or `ZombieHorse` -- an `AgeableMob` whose babies take
/// the base rule, and the one name this needs.
#[must_use]
pub fn should_drop_experience(entity: &Entity) -> bool {
    should_drop_experience_with_registry(
        entity,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

#[must_use]
pub fn should_drop_experience_with_registry(
    entity: &Entity,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        // `LivingEntity.isBaby` answers false (`:528-530`), and an armor stand
        // is the other living variant this crate files.
        return entity.living().is_some();
    };
    if data.entity_resource_id() == "minecraft:tadpole" {
        return false;
    }
    if !data.is_baby() {
        return true;
    }
    // A baby: only a class that overrides the base rule still pays.
    let resource_id = data.entity_resource_id();
    if matches!(resource_id, "minecraft:hoglin" | "minecraft:zoglin") {
        return true;
    }
    let mob_category = vanilla_entity_type(entity.entity_type_id())
        .map(|facts| facts.mob_category)
        .or_else(|| {
            content_registry
                .entity_by_id(entity.entity_type_id())
                .map(|spec| spec.category)
        });
    resource_id != "minecraft:zombie_horse"
        && mob_category == Some(VanillaEntityMobCategory::Monster)
}

/// `LivingEntity.getBaseExperienceReward`; the killer's term is
/// [`process_mob_experience`] (`LivingEntity.java:581-583`).
#[must_use]
pub fn experience_reward(entity: &Entity, random: &GameRandom) -> i32 {
    experience_reward_with_registry(
        entity,
        random,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

#[must_use]
pub fn experience_reward_with_registry(
    entity: &Entity,
    random: &GameRandom,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> i32 {
    let EntityData::GeneralLiving(data) = entity.data() else {
        // `LivingEntity.getBaseExperienceReward` (`:585-587`).
        return 0;
    };
    base_experience_reward(entity, data, random, content_registry)
}

/// `EnchantmentHelper.processMobExperience` (`EnchantmentHelper.java:108-118`):
/// every `mob_experience` effect a living `killer` wears in a matching slot,
/// tested in the killed entity's context (`Enchantment.java:238-242`).
#[must_use]
pub(crate) fn process_mob_experience(
    table: &[Option<EnchantmentValueEffects>],
    registry: &[String],
    killer: LootEntity<'_>,
    killed: ValueContext<'_>,
    amount: i32,
    random: &GameRandom,
    value_level: &crate::world::ValueEffectLevel<'_>,
) -> i32 {
    let mut value = amount as f32;
    for entry in VANILLA_EQUIPMENT_SLOTS {
        let Some(tool) = killer.equipment_item(entry.slot) else {
            continue;
        };
        for (id, level) in &tool.enchantments {
            let Some(effects) = registry
                .iter()
                .position(|candidate| candidate == id)
                .and_then(|index| table.get(index)?.as_ref())
                .filter(|effects| effects.matching_slot(entry.slot))
            else {
                continue;
            };
            match effects.apply(*level, killed, value, random, value_level) {
                Ok(next) => value = next,
                Err(detail) => tracing::debug!("mob_experience: {detail}"),
            }
        }
    }
    value as i32
}

/// `Mob.getBaseExperienceReward` (`Mob.java:301-318`) under the type overrides
/// this crate carries.
fn base_experience_reward(
    entity: &Entity,
    data: &GeneralLivingData,
    random: &GameRandom,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> i32 {
    let resource_id = data.entity_resource_id();
    // `Hoglin.getBaseExperienceReward` (`Hoglin.java:234-236`) answers its own
    // `xpReward`, which `ageBoundaryReached` (`:162-170`) moves between 3 and 5.
    if let Some(reward) = data.spawn_experience_reward() {
        return reward;
    }
    // `Piglin.getBaseExperienceReward` (`Piglin.java:278-280`) skips the
    // equipment draws.
    if resource_id == "minecraft:piglin" {
        return MONSTER_XP_REWARD;
    }
    // `Animal.getBaseExperienceReward` (`Animal.java:132-134`), which
    // `WaterAnimal` (`WaterAnimal.java:35-37`) and `AgeableWaterCreature`
    // (`AgeableWaterCreature.java:33-35`) repeat: a draw, whatever `xpReward`
    // says.
    if data.animal_retained().is_some()
        || data.ageable_water_retained().is_some()
        || crate::entity::general_living::is_water_animal_entity_type(entity.entity_type_id())
    {
        // `Chicken.getBaseExperienceReward` (`Chicken.java:194-196`).
        return if data.chicken_jockey() == Some(true) {
            CHICKEN_JOCKEY_XP_REWARD
        } else {
            1 + random.next_int(3)
        };
    }
    let mut reward = xp_reward(entity, data, resource_id, content_registry);
    if reward <= 0 {
        return reward;
    }
    // `Mob.getBaseExperienceReward` (`Mob.java:301-318`): equipment that can
    // carry experience adds a draw each, unless its drop chance exceeds one.
    let Some(mob) = data.mob.as_ref() else {
        return reward;
    };
    for entry in VANILLA_EQUIPMENT_SLOTS {
        // `EquipmentSlot.canIncreaseExperience` (`EquipmentSlot.java:82-84`).
        if entry.slot_type == EquipmentSlotType::Saddle {
            continue;
        }
        if entity
            .living()
            .and_then(|living| living.item_by_slot(entry.slot))
            .is_none()
        {
            continue;
        }
        if mob.drop_chance(entry.slot) <= 1.0 {
            reward += 1 + random.next_int(3);
        }
    }
    reward
}

/// `Mob.xpReward` (`Mob.java:123`) as the constructor of each type this crate
/// carries sets it.
fn xp_reward(
    entity: &Entity,
    data: &GeneralLivingData,
    resource_id: &str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> i32 {
    match resource_id {
        // `Blaze.java:40`, `Breeze.java:46`, `Evoker.java:44`,
        // `Guardian.java:63` (`ElderGuardian` inherits it).
        "minecraft:blaze"
        | "minecraft:breeze"
        | "minecraft:evoker"
        | "minecraft:elder_guardian"
        | "minecraft:guardian" => 10,
        // `Endermite.java:36`, `Vex.java:58`.
        "minecraft:endermite" | "minecraft:vex" => 3,
        // `Ghast.java:51`, `Illusioner.java:41`, `Phantom.java:54`,
        // `Piglin.java:71`, `Shulker.java:90`, `Warden.java:99`,
        // `Zoglin.java:75`.
        "minecraft:ghast"
        | "minecraft:illusioner"
        | "minecraft:phantom"
        | "minecraft:piglin"
        | "minecraft:shulker"
        | "minecraft:warden"
        | "minecraft:zoglin" => MONSTER_XP_REWARD,
        // `PiglinBrute.java:52`, `Ravager.java:67`.
        "minecraft:piglin_brute" | "minecraft:ravager" => 20,
        // `WitherBoss.java:75`.
        "minecraft:wither" => 50,
        // `Slime.setSize` (`Slime.java:104`) writes the size; `MagmaCube`
        // inherits the body.
        "minecraft:slime" | "minecraft:magma_cube" => {
            data.slime_retained().map_or(0, |slime| slime.size())
        }
        // `Zombie.getBaseExperienceReward` (`Zombie.java:178-184`), inherited
        // by the four zombie variants.
        "minecraft:drowned"
        | "minecraft:husk"
        | "minecraft:zombie"
        | "minecraft:zombie_villager"
        | "minecraft:zombified_piglin" => {
            if data.is_baby() {
                BABY_ZOMBIE_XP_REWARD
            } else {
                MONSTER_XP_REWARD
            }
        }
        // `Creaking.java:92` sets zero; `EnderDragon` extends `Mob` and
        // awards its orbs in its own `die` (`EnderDragon.java:520-543`).
        "minecraft:creaking" | "minecraft:ender_dragon" => 0,
        _ => {
            // Every other monster class inherits `Monster`'s five; custom mod
            // entities carry their registered `xp_reward`.
            if let Some(facts) = vanilla_entity_type(entity.entity_type_id()) {
                if facts.mob_category == VanillaEntityMobCategory::Monster {
                    MONSTER_XP_REWARD
                } else {
                    0
                }
            } else {
                content_registry
                    .entity_by_id(entity.entity_type_id())
                    .map_or(0, |spec| spec.xp_reward)
            }
        }
    }
}
