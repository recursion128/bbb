use super::*;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    VANILLA_ENTITY_TYPE_ENDERMITE_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_GUARDIAN_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
    VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
    VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID,
    VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;

use crate::entity::experience::{experience_reward, should_drop_experience};
use crate::entity::Entity;

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

/// A detached mob of one type, which is the shape every reward read runs on.
fn mob(entity_type_id: i32) -> Box<Entity> {
    let mut store = EntityStore::new(0xCA7A, EntityIdCounter::new());
    let id = store
        .spawn_general_living(entity_type_id, position())
        .expect("the type is living");
    store.detach(id).expect("the id it just filed detaches")
}

fn reward(entity: &Entity) -> i32 {
    experience_reward(entity, &GameRandom::with_seed(0x5EED))
}

/// `Mob.xpReward` (`Mob.java:123`) per constructor: the monster default of five
/// (`Monster.java:34`) and every override this crate carries.
#[test]
fn the_experience_reward_is_the_types_constructor_xp_reward() {
    for (entity_type_id, expected) in [
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, 5),
        (VANILLA_ENTITY_TYPE_SKELETON_ID, 5),
        (VANILLA_ENTITY_TYPE_SHULKER_ID, 5),
        (VANILLA_ENTITY_TYPE_GHAST_ID, 5),
        (VANILLA_ENTITY_TYPE_BLAZE_ID, 10),
        (VANILLA_ENTITY_TYPE_GUARDIAN_ID, 10),
        (VANILLA_ENTITY_TYPE_ENDERMITE_ID, 3),
        (VANILLA_ENTITY_TYPE_RAVAGER_ID, 20),
        (VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, 20),
        (VANILLA_ENTITY_TYPE_WITHER_ID, 50),
        (VANILLA_ENTITY_TYPE_CREAKING_ID, 0),
        // `Pillager`, `Vindicator` and `Witch` inherit `Monster`'s five;
        // `EnderDragon` awards its own orbs in `die` (`EnderDragon.java:520`).
        (VANILLA_ENTITY_TYPE_PILLAGER_ID, 5),
        (VANILLA_ENTITY_TYPE_VINDICATOR_ID, 5),
        (VANILLA_ENTITY_TYPE_WITCH_ID, 5),
        (VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, 0),
    ] {
        assert_eq!(reward(&mob(entity_type_id)), expected, "{entity_type_id}");
    }
}

/// `Animal.getBaseExperienceReward` (`Animal.java:132-134`) and the water
/// classes that repeat it draw `1 + random.nextInt(3)`, whatever the type's
/// `xpReward` says; `Chicken` answers ten for a jockey (`Chicken.java:194-196`)
/// and `Hoglin` answers its own retained reward (`Hoglin.java:234-236`).
#[test]
fn animals_draw_and_the_overrides_answer_their_own_value() {
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_COD_ID,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    ] {
        let entity = mob(entity_type_id);
        for seed in 0..16 {
            let drawn = experience_reward(&entity, &GameRandom::with_seed(seed));
            assert!((1..=3).contains(&drawn), "{entity_type_id} drew {drawn}");
        }
    }

    let mut chicken = mob(VANILLA_ENTITY_TYPE_CHICKEN_ID);
    assert!((1..=3).contains(&reward(&chicken)));
    assert!(crate::entity::set_spawner_chicken_jockey(&mut chicken));
    assert_eq!(reward(&chicken), 10);

    let hoglin = mob(VANILLA_ENTITY_TYPE_HOGLIN_ID);
    assert_eq!(reward(&hoglin), 5, "the constructor's adult reward");
    let mut baby_hoglin = mob(VANILLA_ENTITY_TYPE_HOGLIN_ID);
    assert!(crate::entity::set_spawn_experience_reward(
        &mut baby_hoglin,
        3
    ));
    assert_eq!(reward(&baby_hoglin), 3, "ageBoundaryReached's baby reward");
}

/// `Zombie.getBaseExperienceReward` (`Zombie.java:178-184`): a baby doubles
/// `xpReward` before `Mob`'s own body runs, so a baby zombie answers
/// `(int)(5 * 2.5)`.
#[test]
fn a_baby_zombie_doubles_its_reward() {
    let mut zombie = mob(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(reward(&zombie), 5);
    assert!(crate::entity::zombie::set_baby(&mut zombie, true));
    assert_eq!(reward(&zombie), 12);
}

/// `Mob.getBaseExperienceReward` (`Mob.java:301-318`): equipment in a slot that
/// can carry experience adds a draw unless its drop chance exceeds one.
#[test]
fn equipment_adds_a_draw_unless_its_drop_chance_exceeds_one() {
    let mut zombie = mob(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    zombie
        .living_mut()
        .expect("a zombie is living")
        .set_item_slot(
            EquipmentSlot::MainHand,
            plain_stack("minecraft:stone_sword", 1),
        );
    for seed in 0..16 {
        let drawn = experience_reward(&zombie, &GameRandom::with_seed(seed));
        assert!((6..=8).contains(&drawn), "drew {drawn}");
    }

    let EntityData::GeneralLiving(data) = &mut zombie.data else {
        unreachable!("a zombie uses GeneralLiving")
    };
    assert!(data
        .mob
        .as_mut()
        .expect("a zombie is a Mob")
        .set_drop_chance(EquipmentSlot::MainHand, 2.0));
    assert_eq!(reward(&zombie), 5, "a drop chance above one drops the draw");
}

#[test]
fn an_armed_piglin_still_drops_a_flat_five() {
    let mut piglin = mob(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID);
    piglin
        .living_mut()
        .expect("a piglin is living")
        .set_item_slot(
            EquipmentSlot::MainHand,
            plain_stack("minecraft:golden_sword", 1),
        );
    for seed in 0..16 {
        assert_eq!(experience_reward(&piglin, &GameRandom::with_seed(seed)), 5);
    }
}

/// `LivingEntity.shouldDropExperience()` (`:557-559`) is `!isBaby()` under
/// three class overrides (`Monster.java:126-128`, `Hoglin.java:229-231`,
/// `Tadpole.java:253-255`), and the baby state this crate models comes from
/// three families.
#[test]
fn should_drop_experience_follows_the_base_rule_and_its_overrides() {
    let mut zombie = mob(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert!(should_drop_experience(&zombie));
    assert!(crate::entity::zombie::set_baby(&mut zombie, true));
    assert!(
        should_drop_experience(&zombie),
        "`Monster.shouldDropExperience` answers true for a baby monster"
    );

    let mut cow = mob(VANILLA_ENTITY_TYPE_COW_ID);
    assert!(should_drop_experience(&cow));
    assert!(crate::entity::set_spawner_age(&mut cow, -24_000));
    assert!(!should_drop_experience(&cow));

    let mut piglin = mob(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID);
    assert!(should_drop_experience(&piglin));
    assert!(crate::entity::set_piglin_baby(&mut piglin, true));
    assert!(should_drop_experience(&piglin), "a piglin is a monster too");

    // `Tadpole.shouldDropExperience` (`:253-255`) is the one override that
    // answers false.
    assert!(!should_drop_experience(&mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TADPOLE_ID
    )));

    // `Hoglin.shouldDropExperience` (`:229-231`) answers true for a baby too,
    // and `Zoglin` inherits it.
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    ] {
        let mut hoglin = mob(entity_type_id);
        assert!(crate::entity::set_spawner_age(&mut hoglin, -24_000));
        assert!(
            should_drop_experience(&hoglin),
            "{entity_type_id} overrides the base rule"
        );
    }

    // `ZombieHorse` is monster category without being a `Monster`, so its
    // `AgeableMob` baby takes the base rule.
    let mut zombie_horse = mob(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID);
    assert!(should_drop_experience(&zombie_horse));
    assert!(crate::entity::set_spawner_age(&mut zombie_horse, -24_000));
    assert!(!should_drop_experience(&zombie_horse));
}
