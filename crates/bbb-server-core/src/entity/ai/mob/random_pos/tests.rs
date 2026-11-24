use super::*;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::GroundPathType;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::block_state_types::{
    vanilla_block_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

const FLOOR: i32 = 64;
const QUARTER_TURN: f64 = std::f32::consts::FRAC_PI_2 as f64;

fn mob(entity_type_id: i32) -> RandomPosMob {
    RandomPosMob {
        position: Vec3d {
            x: 0.5,
            y: f64::from(FLOOR),
            z: 0.5,
        },
        entity_type_id,
        home: None,
        malus: PathfindingMalus::for_entity_type(entity_type_id),
        turtle_going_home: false,
        strider_is_in_lava: false,
        hoglin_repellent: None,
    }
}

fn state_with(name: &str, props: &[(&str, &str)]) -> i32 {
    let properties = props
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties).unwrap()
}

fn floor() -> NativeLevel {
    let mut native = NativeLevel::new();
    for x in [-8, 8] {
        for z in [-8, 8] {
            assert_eq!(
                native
                    .world
                    .block_state(DIMENSION, BlockPos { x, y: FLOOR - 1, z }),
                Some(state("minecraft:stone"))
            );
        }
    }
    native
}

fn fluid_tags(water: &[&str], lava: &[&str]) -> RegistryTagSnapshot {
    RegistryTagSnapshot::new([
        (
            "minecraft:fluid".into(),
            "minecraft:water".into(),
            water.iter().map(|id| (*id).into()).collect(),
        ),
        (
            "minecraft:fluid".into(),
            "minecraft:lava".into(),
            lava.iter().map(|id| (*id).into()).collect(),
        ),
    ])
}

#[test]
fn a_frog_accepts_real_water_below_a_destination_without_solid_render() {
    let mut native = NativeLevel::new();
    let target = BlockPos {
        x: 0,
        y: FLOOR,
        z: 0,
    };
    native.set(
        BlockPos {
            y: FLOOR - 1,
            ..target
        },
        state("minecraft:water"),
    );
    native.with_level(|level| {
        assert_eq!(
            random_pos_stable_destination(level, VANILLA_ENTITY_TYPE_FROG_ID, target),
            Some(true)
        );
        assert_eq!(
            random_pos_stable_destination(level, VANILLA_ENTITY_TYPE_COW_ID, target),
            Some(false)
        );
    });
}

#[test]
fn waterlogged_slab_uses_installed_water_tag_while_trapdoor_keeps_static_property() {
    let slab = state_with(
        "minecraft:oak_slab",
        &[("type", "top"), ("waterlogged", "true")],
    );
    let trapdoor = state_with(
        "minecraft:oak_trapdoor",
        &[
            ("facing", "north"),
            ("half", "top"),
            ("open", "true"),
            ("powered", "true"),
            ("waterlogged", "true"),
        ],
    );
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        assert_eq!(block_state_water_pathfindable(level, slab), Some(true));
        assert_eq!(block_state_water_pathfindable(level, trapdoor), Some(true));
    });
    native.install_tags(fluid_tags(
        &[],
        &["minecraft:lava", "minecraft:flowing_lava"],
    ));
    native.with_level(|level| {
        assert_eq!(block_state_water_pathfindable(level, slab), Some(false));
        assert_eq!(block_state_water_pathfindable(level, trapdoor), Some(true));
    });
}

#[test]
fn liquid_path_uses_installed_source_lava_membership_for_flowing_state() {
    let flowing = state_with("minecraft:water", &[("level", "1")]);
    let mut native = NativeLevel::new();
    native
        .with_level(|level| assert_eq!(block_state_water_pathfindable(level, flowing), Some(true)));
    native.install_tags(fluid_tags(
        &["minecraft:water", "minecraft:flowing_water"],
        &["minecraft:flowing_water"],
    ));
    native
        .with_level(|level| assert_eq!(block_state_water_pathfindable(level, flowing), Some(true)));
    native.install_tags(fluid_tags(
        &["minecraft:water", "minecraft:flowing_water"],
        &["minecraft:water"],
    ));
    native.with_level(|level| {
        assert_eq!(block_state_water_pathfindable(level, flowing), Some(false))
    });
    native.install_tags(fluid_tags(
        &["minecraft:water", "minecraft:flowing_water"],
        &[],
    ));
    native.with_level(|level| {
        assert_eq!(
            block_state_water_pathfindable(level, state_with("minecraft:lava", &[("level", "1")])),
            Some(true)
        )
    });
}

#[test]
fn a_swim_target_stays_in_real_water_inside_the_home_radius() {
    let mut native = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            for y in 64..=70 {
                native.set(BlockPos { x, y, z }, state("minecraft:water"));
            }
        }
    }
    let mut frog = mob(VANILLA_ENTITY_TYPE_FROG_ID);
    frog.position.y = 67.0;
    frog.home = Some((BlockPos { x: 0, y: 67, z: 0 }, 4));
    frog.malus.set(GroundPathType::Water, 0.0);
    let random = GameRandom::with_seed(11);
    let target = native
        .with_level(|level| swim_target_pos(level, &random, frog))
        .unwrap()
        .unwrap();
    native.with_level(|level| {
        assert_eq!(
            block_state_water_pathfindable(level, level.block_state(containing_block(target))),
            Some(true)
        );
    });
    let centre = Vec3d {
        x: 0.5,
        y: 67.5,
        z: 0.5,
    };
    assert!(
        (target.x - centre.x).powi(2)
            + (target.y - centre.y).powi(2)
            + (target.z - centre.z).powi(2)
            < 16.0
    );
}

#[test]
fn swim_home_restriction_uses_the_candidate_vector_and_the_home_center() {
    let mut frog = mob(VANILLA_ENTITY_TYPE_FROG_ID);
    frog.home = Some((BlockPos { x: 0, y: 0, z: 0 }, 2));
    let candidate = Vec3d {
        x: 2.4,
        y: 0.5,
        z: 0.5,
    };
    assert!(!is_restricted_vec3(true, frog, candidate));
    assert!(is_restricted(true, frog, containing_block(candidate)));
}

#[test]
fn a_distance_of_one_or_less_never_reaches_the_home_shift() {
    let mut mob = mob(VANILLA_ENTITY_TYPE_COW_ID);
    mob.home = Some((
        BlockPos {
            x: 0,
            y: FLOOR,
            z: 0,
        },
        64,
    ));
    let random = GameRandom::with_seed(3);
    let pos =
        generate_random_pos_toward_direction(&random, mob, 1.0, BlockPos { x: 4, y: 0, z: 4 });
    assert_eq!(
        pos,
        BlockPos {
            x: 4,
            y: FLOOR,
            z: 4
        }
    );
}

#[test]
fn the_real_grass_support_outweighs_a_stone_support_for_an_animal() {
    let mut native = NativeLevel::new();
    let grass = BlockPos {
        x: 0,
        y: FLOOR,
        z: 0,
    };
    let stone = BlockPos {
        x: 1,
        y: FLOOR,
        z: 0,
    };
    native.set(
        BlockPos {
            y: FLOOR - 1,
            ..grass
        },
        state("minecraft:grass_block"),
    );
    native.with_level(|level| {
        assert_eq!(
            walk_target_value(level, mob(VANILLA_ENTITY_TYPE_COW_ID), grass),
            Some(10.0)
        );
        assert!(
            walk_target_value(level, mob(VANILLA_ENTITY_TYPE_COW_ID), grass).unwrap()
                > walk_target_value(level, mob(VANILLA_ENTITY_TYPE_COW_ID), stone).unwrap()
        );
    });
}

#[test]
fn a_monster_prefers_the_real_darkness_an_animal_rejects() {
    let mut native = floor();
    let target = BlockPos {
        x: 0,
        y: FLOOR,
        z: 0,
    };
    let animal = mob(VANILLA_ENTITY_TYPE_COW_ID);
    let monster = mob(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(&sender, &receiver, &mut native.world, 1, "time set 6000");
    let bright = native.with_level(|level| {
        (
            walk_target_value(level, animal, target).unwrap(),
            walk_target_value(level, monster, target).unwrap(),
        )
    });
    crate::tests::console_command(&sender, &receiver, &mut native.world, 2, "time set 18000");
    let dark = native.with_level(|level| {
        (
            walk_target_value(level, animal, target).unwrap(),
            walk_target_value(level, monster, target).unwrap(),
        )
    });
    assert!(bright.0 > dark.0);
    assert!(dark.1 > bright.1);
}

#[test]
fn the_land_sampler_climbs_out_of_a_real_solid_column_the_default_one_rejects() {
    let mut native = floor();
    let mut sunk = mob(VANILLA_ENTITY_TYPE_COW_ID);
    sunk.position.y = f64::from(FLOOR - 1);
    native.with_level(|level| {
        let random = GameRandom::with_seed(11);
        assert_eq!(default_random_pos(level, &random, sunk, 0, 0), Some(None));
        let random = GameRandom::with_seed(11);
        assert_eq!(
            land_random_pos(level, &random, sunk, 0, 0)
                .unwrap()
                .unwrap()
                .y,
            f64::from(FLOOR)
        );
    });
}

#[test]
fn the_land_sampler_rejects_a_real_water_surface() {
    let mut native = NativeLevel::new();
    native.set(
        BlockPos {
            x: 0,
            y: FLOOR - 1,
            z: 0,
        },
        state("minecraft:water"),
    );
    let mut sunk = mob(VANILLA_ENTITY_TYPE_COW_ID);
    sunk.position.y = f64::from(FLOOR - 2);
    let random = GameRandom::with_seed(11);
    native.with_level(|level| assert_eq!(land_random_pos(level, &random, sunk, 0, 0), Some(None)));
}

#[test]
fn a_restricted_mob_keeps_every_sample_inside_its_home_radius() {
    let centre = BlockPos {
        x: 0,
        y: FLOOR,
        z: 0,
    };
    let mut restricted = mob(VANILLA_ENTITY_TYPE_COW_ID);
    restricted.home = Some((centre, 4));
    // `GoalUtils.mobRestricted`.
    assert!(mob_restricted(restricted, 10.0));
    // `GoalUtils.isRestricted` over `Mob.isWithinHome` (`GoalUtils`)
    assert!(!is_restricted(
        true,
        restricted,
        BlockPos {
            x: 3,
            y: FLOOR,
            z: 0
        }
    ));
    assert!(is_restricted(
        true,
        restricted,
        BlockPos {
            x: 4,
            y: FLOOR,
            z: 0
        }
    ));
    assert!(!is_restricted(
        false,
        restricted,
        BlockPos {
            x: 40,
            y: FLOOR,
            z: 0
        }
    ));

    let mut far = restricted;
    far.position.x = 1000.5;
    assert!(!mob_restricted(far, 10.0));
    assert!(!mob_restricted(mob(VANILLA_ENTITY_TYPE_COW_ID), 10.0));
}

#[test]
fn actual_build_height_and_stable_floor_reject_invalid_destinations() {
    let mut native = floor();
    native.with_level(|level| {
        assert!(is_outside_limits(
            level,
            BlockPos {
                x: 0,
                y: level.min_y() - 1,
                z: 0
            }
        ));
        assert!(is_outside_limits(
            level,
            BlockPos {
                x: 0,
                y: level.max_y() + 1,
                z: 0
            }
        ));
        let ground = BlockPos {
            x: 0,
            y: FLOOR,
            z: 0,
        };
        assert!(!is_outside_limits(level, ground));
        assert_eq!(
            is_not_stable(level, mob(VANILLA_ENTITY_TYPE_COW_ID), ground),
            Some(false)
        );
        assert_eq!(
            is_not_stable(
                level,
                mob(VANILLA_ENTITY_TYPE_COW_ID),
                BlockPos {
                    y: FLOOR + 1,
                    ..ground
                }
            ),
            Some(true)
        );
    });
}

#[test]
fn the_infested_host_list_includes_deepslate_and_the_six_stone_hosts() {
    use crate::block_behavior::infested::is_compatible_host;

    for resource_id in [
        "minecraft:stone",
        "minecraft:cobblestone",
        "minecraft:stone_bricks",
        "minecraft:mossy_stone_bricks",
        "minecraft:cracked_stone_bricks",
        "minecraft:chiseled_stone_bricks",
        "minecraft:deepslate",
    ] {
        assert!(is_compatible_host(state(resource_id)));
    }
    assert!(!is_compatible_host(state("minecraft:air")));
    assert!(!is_compatible_host(
        vanilla_block_state_id(-1).unwrap_or(-1)
    ));
}

#[test]
fn a_hover_sample_finishes_in_open_air_above_real_stable_ground() {
    let mut native = floor();
    let parrot = mob(VANILLA_ENTITY_TYPE_PARROT_ID);
    let random = GameRandom::with_seed(0);
    let mut found = 0;
    for _ in 0..64 {
        let target = native
            .with_level(|level| {
                hover_random_pos(level, &random, parrot, 8, 7, 0.0, 1.0, QUARTER_TURN, 3, 1)
            })
            .unwrap();
        // A sample can miss; the ones that land must be in hover range.
        let Some(target) = target else {
            continue;
        };
        found += 1;
        assert!((f64::from(FLOOR + 1)..=f64::from(FLOOR + 3)).contains(&target.y));
        native.with_level(|level| {
            assert_eq!(
                level.block_state(containing_block(target)),
                state("minecraft:air")
            )
        });
    }
    assert!(found > 0);
}

#[test]
fn an_air_and_water_sample_finishes_outside_real_solid_ground() {
    let mut native = floor();
    let parrot = mob(VANILLA_ENTITY_TYPE_PARROT_ID);
    let random = GameRandom::with_seed(5);
    let target = native
        .with_level(|level| {
            air_and_water_random_pos(level, &random, parrot, 8, 4, -2, 0.0, 1.0, QUARTER_TURN)
        })
        .unwrap()
        .unwrap();
    assert!(target.y >= f64::from(FLOOR));
    native.with_level(|level| {
        assert_eq!(
            level.block_state(containing_block(target)),
            state("minecraft:air")
        )
    });
}

#[test]
fn moving_above_real_solid_rises_from_the_first_open_block() {
    let mut native = floor();
    let ground = BlockPos {
        x: 0,
        y: FLOOR - 1,
        z: 0,
    };
    native.with_level(|level| {
        assert_eq!(
            move_up_to_above_solid(level, ground, 3),
            Some(BlockPos {
                y: FLOOR + 3,
                ..ground
            })
        );
        assert_eq!(
            move_up_to_above_solid(level, ground, 0),
            Some(BlockPos { y: FLOOR, ..ground })
        );
        let open = BlockPos {
            y: FLOOR + 5,
            ..ground
        };
        assert_eq!(move_up_to_above_solid(level, open, 3), Some(open));
    });
}
