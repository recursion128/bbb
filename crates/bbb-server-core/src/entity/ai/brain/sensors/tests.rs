use super::*;
use crate::entity::ai::brain::tests::{entity_facts, test_world};
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{Brain, BrainDamageSource, BrainEntityFacts, DEFAULT_SCAN_RATE};

/// Runs the ticks that carry every registered sensor to its next scan:
/// `Sensor.randomlyDelayStart` drew a zero here, so the first tick scans, the
/// scan resets the counter to the rate, and the counter
/// comes due again one whole rate later.
fn tick_one_scan(
    brain: &mut Brain,
    context: &mut BrainTickContext,
    world: &mut crate::entity::ai::brain::tests::TestWorld,
) {
    for _ in 0..DEFAULT_SCAN_RATE {
        context.game_time += 1;
        brain.tick(context, world);
    }
}

fn damaged(damage_type: &str, source_entity: Option<i32>) -> BrainDamageSource {
    BrainDamageSource {
        damage_type: damage_type.to_string(),
        source_entity,
    }
}

fn brain_with_sensors() -> Brain {
    let mut brain = Brain::new();
    brain.register_sensor(SensorType::HurtBy);
    brain.register_sensor(SensorType::IsInWater);
    brain
}

#[test]
fn hurt_by_sensor_writes_the_last_damage_source_it_reads() {
    let mut brain = brain_with_sensors();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(damaged("minecraft:player_attack", Some(7)));
    world.entity_facts = Some(entity_facts(7, "minecraft:player"));

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::HurtBy),
        Some(&MemoryValue::DamageSource(damaged(
            "minecraft:player_attack",
            Some(7)
        )))
    );
    // `damageSource.getEntity() instanceof LivingEntity`
    assert_eq!(
        brain.memory(MemoryModuleType::HurtByEntity),
        Some(&MemoryValue::EntityRef(7))
    );
}

#[test]
fn hurt_by_sensor_refuses_an_entity_that_is_not_living() {
    let mut brain = brain_with_sensors();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(damaged("minecraft:explosion", Some(9)));
    world.entity_facts = Some(BrainEntityFacts {
        living: false,
        ..entity_facts(9, "minecraft:arrow")
    });

    brain.tick(&mut context, &mut world);
    assert!(brain.has_memory_value(MemoryModuleType::HurtBy));
    assert!(!brain.has_memory_value(MemoryModuleType::HurtByEntity));
}

#[test]
fn hurt_by_sensor_erases_the_entity_memory_of_a_target_that_is_gone() {
    let mut brain = brain_with_sensors();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(damaged("minecraft:mob_attack", Some(4)));
    world.entity_facts = Some(entity_facts(4, "minecraft:player"));
    brain.tick(&mut context, &mut world);
    assert!(brain.has_memory_value(MemoryModuleType::HurtByEntity));

    // `!hurtByEntity.isAlive() || hurtByEntity.level() != level`
    // with a source that still answers.
    world.entity_facts = None;
    tick_one_scan(&mut brain, &mut context, &mut world);
    assert!(brain.has_memory_value(MemoryModuleType::HurtBy));
    assert!(!brain.has_memory_value(MemoryModuleType::HurtByEntity));
}

#[test]
fn hurt_by_sensor_keeps_a_live_entity_through_a_null_source() {
    let mut brain = brain_with_sensors();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(damaged("minecraft:mob_attack", Some(4)));
    world.entity_facts = Some(entity_facts(4, "minecraft:player"));
    brain.tick(&mut context, &mut world);

    // The forty-tick window `getLastDamageSource` answers in
    // closes: only `HURT_BY` is erased
    world.body.last_damage = None;
    tick_one_scan(&mut brain, &mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::HurtBy));
    assert_eq!(
        brain.memory(MemoryModuleType::HurtByEntity),
        Some(&MemoryValue::EntityRef(4))
    );
}

#[test]
fn is_in_water_sensor_sets_and_erases_the_unit_memory() {
    let mut brain = brain_with_sensors();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.in_water = true;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::IsInWater),
        Some(&MemoryValue::Unit)
    );

    // `Sensor.tick` resets the counter to the rate after a scan
    // so the memory stays until the next scan, one
    // whole rate later.
    world.body.in_water = false;
    tick_one_scan(&mut brain, &mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::IsInWater));
}

#[test]
fn the_wired_sensors_require_vanillas_memories() {
    assert_eq!(
        SensorType::HurtBy.requires(),
        &[MemoryModuleType::HurtBy, MemoryModuleType::HurtByEntity]
    );
    assert_eq!(
        SensorType::IsInWater.requires(),
        &[MemoryModuleType::IsInWater]
    );
    assert_eq!(
        SensorType::NearestLivingEntities.requires(),
        &[
            MemoryModuleType::NearestLivingEntities,
            MemoryModuleType::NearestVisibleLivingEntities,
        ]
    );
    // `FrogAttackablesSensor.requires`.
    assert_eq!(
        SensorType::FrogAttackables.requires(),
        &[
            MemoryModuleType::NearestAttackable,
            MemoryModuleType::NearestVisibleLivingEntities,
            MemoryModuleType::UnreachableTongueTargets,
        ]
    );
    // `Sensor`: none of the five names a rate.
    for rate in [
        SensorType::HurtBy.scan_rate(),
        SensorType::IsInWater.scan_rate(),
        SensorType::NearestLivingEntities.scan_rate(),
        SensorType::FrogAttackables.scan_rate(),
        SensorType::FrogTemptations.scan_rate(),
    ] {
        assert_eq!(rate, DEFAULT_SCAN_RATE);
    }
    // `TemptingSensor.requires`.
    assert_eq!(
        SensorType::FrogTemptations.requires(),
        &[MemoryModuleType::TemptingPlayer]
    );
    // `SensorType`'s registry paths (`SensorType`).
    assert_eq!(
        [
            SensorType::NearestLivingEntities.resource_id(),
            SensorType::HurtBy.resource_id(),
            SensorType::FrogTemptations.resource_id(),
            SensorType::FrogAttackables.resource_id(),
            SensorType::IsInWater.resource_id(),
        ],
        [
            "minecraft:nearest_living_entities",
            "minecraft:hurt_by",
            "minecraft:frog_temptations",
            "minecraft:frog_attackables",
            "minecraft:is_in_water",
        ]
    );
}

/// A brain with the two sensors the creaking's provider registers, so the
/// player scan has the same neighbours it has in production.
fn brain_with_players() -> Brain {
    let mut brain = Brain::new();
    brain.register_sensor(SensorType::NearestLivingEntities);
    brain.register_sensor(SensorType::NearestPlayers);
    brain
}

/// `PlayerSensor.doTick`: the `NEAREST_PLAYERS`
/// list keeps the scan's order, the singular memories are its firsts, and the
/// attackable list is a filter of the visible one.
#[test]
fn the_player_sensor_writes_the_scan_and_its_two_filters() {
    let mut brain = brain_with_players();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.nearest_players = vec![9, 4];
    world.entity_facts = Some(entity_facts(9, "minecraft:player"));
    world.targeting = true;
    world.combat_targeting = false;

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::NearestPlayers),
        Some(&MemoryValue::EntityList(vec![9, 4]))
    );
    assert_eq!(
        brain.memory(MemoryModuleType::NearestVisiblePlayer),
        Some(&MemoryValue::EntityRef(9))
    );
    // `isEntityAttackable` answers for neither, and `setMemory` erases a slot
    // a list would empty.
    assert_eq!(
        brain.memory(MemoryModuleType::NearestVisibleAttackablePlayers),
        None
    );
    assert_eq!(
        brain.memory(MemoryModuleType::NearestVisibleAttackablePlayer),
        None
    );
}

/// `EntitySelector.NO_SPECTATORS` and the follow-range
/// scan the world answers: a spectator-only scan leaves every slot empty.
#[test]
fn the_player_sensor_refuses_a_spectator() {
    let mut brain = brain_with_players();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.nearest_players = vec![9];
    world.entity_facts = Some(BrainEntityFacts {
        spectator: true,
        ..entity_facts(9, "minecraft:player")
    });

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.memory(MemoryModuleType::NearestPlayers), None);
    assert_eq!(brain.memory(MemoryModuleType::NearestVisiblePlayer), None);
}

/// The attackable list is `visiblePlayers.stream().filter(isEntityAttackable)`
/// so an attackable player that is not targetable is
/// in neither list.
#[test]
fn the_player_sensor_keeps_the_attackable_list_inside_the_visible_one() {
    let mut brain = brain_with_players();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.nearest_players = vec![9];
    world.entity_facts = Some(entity_facts(9, "minecraft:player"));
    world.targeting = true;
    world.combat_targeting = true;

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::NearestVisiblePlayer),
        Some(&MemoryValue::EntityRef(9))
    );
    assert_eq!(
        brain.memory(MemoryModuleType::NearestVisibleAttackablePlayers),
        Some(&MemoryValue::EntityList(vec![9]))
    );
    assert_eq!(
        brain.memory(MemoryModuleType::NearestVisibleAttackablePlayer),
        Some(&MemoryValue::EntityRef(9))
    );
}

/// A brain with only `TemptingSensor`, which is the fifth sensor
/// `Frog.BRAIN_PROVIDER` names.
fn brain_with_temptations() -> Brain {
    let mut brain = Brain::new();
    brain.register_sensor(SensorType::FrogTemptations);
    brain
}

/// One session player of the temptation scan, holding `item` in its main
/// hand.
fn holding(entity_id: i32, item: i32) -> BrainPlayer {
    BrainPlayer {
        entity_id,
        main_hand_item_id: Some(item),
        off_hand_item_id: None,
    }
}

/// `minecraft:slime_ball`, the one member of `#minecraft:frog_food`
/// and `minecraft:stone`, which is not a
/// member.
fn item(resource_id: &str) -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in item"))
        .id
}

/// The world a tempting scan reads: `#minecraft:frog_food` names the slime
/// ball and nothing else.
fn tempting_world() -> crate::entity::ai::brain::tests::TestWorld {
    let mut world = test_world();
    world.tempting_item_id = Some(item("minecraft:slime_ball"));
    world.item_tags = true;
    world.entity_facts = Some(entity_facts(9, "minecraft:player"));
    world.distance_to_sqr = Some(4.0);
    // `Animal.createAnimalAttributes` gives every animal ten blocks
    world.body.tempt_range = 10.0;
    world
}

/// `TemptingSensor.doTick`: the scan is sorted
/// nearest first, the temptation filter runs per player, and the first
/// survivor is the one the memory names.
#[test]
fn the_tempting_sensor_takes_the_nearest_player_holding_frog_food() {
    let mut brain = brain_with_temptations();
    let mut context = BrainTickContext::default();
    let mut world = tempting_world();
    // The scan the world answers is already nearest first; the nearest player
    // holds something that is not `#minecraft:frog_food`.
    world.players_within = vec![
        holding(4, item("minecraft:stone")),
        holding(9, item("minecraft:slime_ball")),
        holding(7, item("minecraft:slime_ball")),
    ];

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::TemptingPlayer),
        Some(&MemoryValue::EntityRef(9)),
        "the first player whose hands hold #minecraft:frog_food"
    );
}

/// The off hand is tried after the main hand, and
/// an item the tag does not name is not a temptation.
#[test]
fn the_tempting_sensor_reads_both_hands_and_the_item_tag() {
    let mut brain = brain_with_temptations();
    let mut context = BrainTickContext::default();
    let mut world = tempting_world();
    world.players_within = vec![BrainPlayer {
        entity_id: 9,
        main_hand_item_id: Some(item("minecraft:stone")),
        off_hand_item_id: Some(item("minecraft:slime_ball")),
    }];

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::TemptingPlayer),
        Some(&MemoryValue::EntityRef(9))
    );

    // Neither hand holds the food, so nothing is tempting and the slot is
    // erased. The scan happens once a rate
    // so the second answer needs its own scan.
    world.players_within = vec![holding(9, item("minecraft:stone"))];
    tick_one_scan(&mut brain, &mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::TemptingPlayer));
}

/// The targeting condition `TemptingSensor` builds, which is the only
/// range test the scan makes, and `!body.hasPassenger(player)`.
#[test]
fn the_tempting_sensor_builds_its_own_condition_and_refuses_a_passenger() {
    let tempting = holding(9, item("minecraft:slime_ball"));

    // `TEMPT_TARGETING.copy().range(body.getAttributeValue(TEMPT_RANGE))`
    // (`:22`, `:39`): a non-combat condition that ignores line of sight and
    // tests invisibility, ranged by the body's own attribute rather than by
    // the follow range `Sensor.isEntityTargetable` uses.
    let mut brain = brain_with_temptations();
    let mut context = BrainTickContext::default();
    let mut world = tempting_world();
    world.players_within = vec![tempting];

    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.targeting_tests.borrow().as_slice(),
        &[EntityTargetingTest {
            entity_id: 9,
            combat: false,
            range: 10.0,
            ignore_line_of_sight: true,
            ignore_invisibility: false,
        }],
        "the body's TEMPT_RANGE reached the condition"
    );
    assert_eq!(
        brain.memory(MemoryModuleType::TemptingPlayer),
        Some(&MemoryValue::EntityRef(9))
    );

    // A player the body carries is not a temptation (`:45`).
    let mut brain = brain_with_temptations();
    let mut world = tempting_world();
    world.players_within = vec![tempting];
    world.passengers = vec![9];
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::TemptingPlayer));

    // A condition the world answers false for refuses the player before the
    // hands are read (`:43`).
    let mut brain = brain_with_temptations();
    let mut world = tempting_world();
    world.players_within = vec![tempting];
    world.targeting = false;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::TemptingPlayer));
}
