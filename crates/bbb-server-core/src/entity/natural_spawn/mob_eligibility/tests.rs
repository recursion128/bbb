use crate::entity::natural_spawn_facts_26_1::{
    vanilla_natural_spawn_type_facts_26_1, VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1,
};

use super::*;

#[derive(Clone, Copy)]
struct RulesProbe {
    pathfinding_cost_from_light_levels: f32,
    at: i32,
    below: i32,
    below_is_sand: bool,
    air_at_and_below: [bool; 3],
    turtle_going_home: bool,
    strider_is_in_lava: bool,
    hoglin_near_repellent: bool,
}

fn state(block_type_id: i32) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type_id).unwrap()
}

impl Default for RulesProbe {
    fn default() -> Self {
        Self {
            pathfinding_cost_from_light_levels: 0.0,
            at: state(block_types::VANILLA_BLOCK_BEDROCK_ID),
            below: state(block_types::VANILLA_BLOCK_BEDROCK_ID),
            below_is_sand: false,
            air_at_and_below: [false; 3],
            turtle_going_home: false,
            strider_is_in_lava: false,
            hoglin_near_repellent: false,
        }
    }
}

impl RulesProbe {
    fn runtime(
        self,
    ) -> NaturalSpawnRulesProbe<impl Fn() -> f32, impl Fn(usize) -> i32, impl Fn(i32) -> bool> {
        let blocks = [self.at, self.below];
        NaturalSpawnRulesProbe {
            pathfinding_cost_from_light_levels: move || self.pathfinding_cost_from_light_levels,
            block_at: move |depth| blocks[depth],
            below_is_sand: move |_| self.below_is_sand,
            air_at_and_below: self.air_at_and_below,
            turtle_going_home: self.turtle_going_home,
            strider_is_in_lava: self.strider_is_in_lava,
            hoglin_near_repellent: self.hoglin_near_repellent,
        }
    }
}

fn spawn_rules(id: i32, probe: RulesProbe) -> Result<bool, NaturalSpawnEligibilityError> {
    check_natural_spawn_rules(id, probe.runtime())
}

fn walk_value(id: i32, probe: RulesProbe) -> Result<f32, NaturalSpawnEligibilityError> {
    mob_walk_target_value(id, probe.runtime())
}

fn id(resource_id: &str) -> i32 {
    vanilla_natural_spawn_type_facts_26_1(resource_id)
        .unwrap()
        .entity_type_id
}

#[test]
fn every_generated_mob_has_all_four_runtime_decisions() {
    for facts in VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1
        .iter()
        .filter(|facts| facts.is_mob)
    {
        spawn_rules(facts.entity_type_id, RulesProbe::default())
            .unwrap_or_else(|error| panic!("{} spawn rules: {error:?}", facts.resource_id));
        assert_eq!(
            walk_value(facts.entity_type_id, RulesProbe::default()).is_ok(),
            facts.walk_target_value_owner.is_some(),
            "{} walk query support",
            facts.resource_id
        );
        check_natural_spawn_obstruction(
            facts.entity_type_id,
            NaturalSpawnObstructionProbe::default(),
        )
        .unwrap_or_else(|error| panic!("{} obstruction: {error:?}", facts.resource_id));
        natural_spawn_cluster_size(facts.entity_type_id)
            .unwrap_or_else(|error| panic!("{} cluster size: {error:?}", facts.resource_id));
        natural_spawn_group_size_reached(
            facts.entity_type_id,
            (facts.resource_id == "minecraft:tropical_fish").then_some(true),
        )
        .unwrap_or_else(|error| panic!("{} group size: {error:?}", facts.resource_id));
    }
}

#[test]
fn pathfinder_spawn_rules_dispatch_the_delegated_walk_target_owner() {
    let negative_cost = RulesProbe {
        pathfinding_cost_from_light_levels: -0.5,
        ..Default::default()
    };
    assert!(!spawn_rules(id("minecraft:cow"), negative_cost).unwrap());
    assert!(spawn_rules(
        id("minecraft:cow"),
        RulesProbe {
            below: state(block_types::VANILLA_BLOCK_GRASS_BLOCK_ID),
            ..negative_cost
        }
    )
    .unwrap());

    let positive_cost = RulesProbe {
        pathfinding_cost_from_light_levels: 0.5,
        ..Default::default()
    };
    assert!(!spawn_rules(id("minecraft:zombie"), positive_cost).unwrap());
    assert!(spawn_rules(id("minecraft:zombie"), negative_cost).unwrap());
    assert!(spawn_rules(id("minecraft:giant"), positive_cost).unwrap());
    assert!(!spawn_rules(id("minecraft:giant"), negative_cost).unwrap());
}

#[test]
fn stateful_and_medium_specific_walk_targets_keep_their_exact_branches() {
    let happy_ghast = id("minecraft:happy_ghast");
    for (air_at_and_below, expected) in [
        ([false, true, false], 0.0),
        ([true, true, true], 5.0),
        ([true, true, false], 10.0),
    ] {
        assert_eq!(
            walk_value(
                happy_ghast,
                RulesProbe {
                    air_at_and_below,
                    ..Default::default()
                }
            )
            .unwrap(),
            expected
        );
    }
    let guardian = id("minecraft:guardian");
    assert!(!spawn_rules(
        guardian,
        RulesProbe {
            pathfinding_cost_from_light_levels: 11.0,
            ..Default::default()
        }
    )
    .unwrap());
    assert!(spawn_rules(
        guardian,
        RulesProbe {
            pathfinding_cost_from_light_levels: -9.5,
            at: state(block_types::VANILLA_BLOCK_WATER_ID),
            ..Default::default()
        }
    )
    .unwrap());
    assert!(!spawn_rules(
        guardian,
        RulesProbe {
            pathfinding_cost_from_light_levels: -10.5,
            at: state(block_types::VANILLA_BLOCK_WATER_ID),
            ..Default::default()
        }
    )
    .unwrap());

    let turtle = id("minecraft:turtle");
    assert!(spawn_rules(
        turtle,
        RulesProbe {
            at: state(block_types::VANILLA_BLOCK_WATER_ID),
            pathfinding_cost_from_light_levels: -1.0,
            ..Default::default()
        }
    )
    .unwrap());
    assert!(!spawn_rules(
        turtle,
        RulesProbe {
            turtle_going_home: true,
            at: state(block_types::VANILLA_BLOCK_WATER_ID),
            pathfinding_cost_from_light_levels: -1.0,
            ..Default::default()
        }
    )
    .unwrap());
    assert!(spawn_rules(
        turtle,
        RulesProbe {
            turtle_going_home: true,
            below_is_sand: true,
            pathfinding_cost_from_light_levels: -1.0,
            ..Default::default()
        }
    )
    .unwrap());

    let strider = id("minecraft:strider");
    assert!(spawn_rules(
        strider,
        RulesProbe {
            at: state(block_types::VANILLA_BLOCK_LAVA_ID),
            strider_is_in_lava: true,
            ..Default::default()
        }
    )
    .unwrap());
    assert!(!spawn_rules(
        strider,
        RulesProbe {
            strider_is_in_lava: true,
            ..Default::default()
        }
    )
    .unwrap());
    assert!(!spawn_rules(
        id("minecraft:hoglin"),
        RulesProbe {
            hoglin_near_repellent: true,
            ..Default::default()
        }
    )
    .unwrap());
}

#[test]
fn silverfish_host_and_mushroom_mycelium_preempt_negative_fallback_costs() {
    let negative = RulesProbe {
        pathfinding_cost_from_light_levels: -1.0,
        ..Default::default()
    };
    assert!(spawn_rules(id("minecraft:silverfish"), negative).unwrap());
    assert!(spawn_rules(
        id("minecraft:mooshroom"),
        RulesProbe {
            below: state(block_types::VANILLA_BLOCK_MYCELIUM_ID),
            ..negative
        }
    )
    .unwrap());
    assert!(!spawn_rules(id("minecraft:mooshroom"), negative).unwrap());
    assert!(spawn_rules(
        id("minecraft:silverfish"),
        RulesProbe {
            pathfinding_cost_from_light_levels: 1.0,
            below: state(block_types::VANILLA_BLOCK_STONE_ID),
            ..Default::default()
        }
    )
    .unwrap());
}

#[test]
fn obstruction_owners_preserve_liquid_floor_and_extra_collision_differences() {
    let liquid = NaturalSpawnObstructionProbe {
        contains_any_liquid: true,
        ..Default::default()
    };
    assert!(!check_natural_spawn_obstruction(id("minecraft:bat"), liquid).unwrap());
    assert!(check_natural_spawn_obstruction(id("minecraft:cod"), liquid).unwrap());
    assert!(!check_natural_spawn_obstruction(id("minecraft:zombified_piglin"), liquid).unwrap());
    assert!(check_natural_spawn_obstruction(
        id("minecraft:ravager"),
        NaturalSpawnObstructionProbe {
            is_unobstructed: false,
            ..Default::default()
        }
    )
    .unwrap());

    let ocelot = id("minecraft:ocelot");
    assert!(!check_natural_spawn_obstruction(
        ocelot,
        NaturalSpawnObstructionProbe {
            position: BlockPos { x: 0, y: 62, z: 0 },
            ..Default::default()
        }
    )
    .unwrap());
    assert!(!check_natural_spawn_obstruction(
        ocelot,
        NaturalSpawnObstructionProbe {
            below_is_grass_or_leaves: false,
            ..Default::default()
        }
    )
    .unwrap());

    let golem = id("minecraft:iron_golem");
    assert!(!check_natural_spawn_obstruction(
        golem,
        NaturalSpawnObstructionProbe {
            above_two_is_valid_empty_spawn_block: false,
            ..Default::default()
        }
    )
    .unwrap());
    assert!(!check_natural_spawn_obstruction(
        id("minecraft:warden"),
        NaturalSpawnObstructionProbe {
            default_dimensions_no_collision: false,
            ..Default::default()
        }
    )
    .unwrap());
}

#[test]
fn cluster_and_group_limits_cover_every_override() {
    assert_eq!(natural_spawn_cluster_size(id("minecraft:pig")).unwrap(), 4);
    assert_eq!(
        natural_spawn_cluster_size(id("minecraft:horse")).unwrap(),
        6
    );
    assert_eq!(natural_spawn_cluster_size(id("minecraft:cod")).unwrap(), 8);
    assert_eq!(
        natural_spawn_cluster_size(id("minecraft:salmon")).unwrap(),
        5
    );
    assert_eq!(natural_spawn_cluster_size(id("minecraft:wolf")).unwrap(), 8);
    assert_eq!(
        natural_spawn_cluster_size(id("minecraft:ghast")).unwrap(),
        1
    );
    assert_eq!(
        natural_spawn_cluster_size(id("minecraft:pillager")).unwrap(),
        1
    );
    assert!(!natural_spawn_group_size_reached(id("minecraft:pig"), None).unwrap());
    assert!(!natural_spawn_group_size_reached(id("minecraft:tropical_fish"), Some(true)).unwrap());
    assert!(natural_spawn_group_size_reached(id("minecraft:tropical_fish"), Some(false)).unwrap());
}
