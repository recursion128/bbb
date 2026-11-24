//! Fresh-player spawn selection through the join path.

use bbb_protocol::game_rules::{vanilla_game_rule, GameRuleValue};

use super::*;

const RESPAWN_RADIUS_RULE: &str = "minecraft:respawn_radius";

/// A restored flat world with the overworld border size and respawn radius the
/// spawn search is to be run against.
fn spawn_search_world(border_size: Option<f64>, respawn_radius: Option<i32>) -> ServerWorld {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    if let Some(size) = border_size {
        candidate.world_borders.insert(
            level_key(Java26BuiltinDimension::Overworld),
            bbb_server_persistence::Java26WorldBorderSettings {
                size,
                ..bbb_server_persistence::Java26WorldBorderSettings::default()
            },
        );
    }
    if let Some(radius) = respawn_radius {
        candidate.game_rules.insert(
            RESPAWN_RADIUS_RULE.to_string(),
            GameRuleValue::Integer(radius),
        );
    }
    ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap()
}

/// `GameRules.RESPAWN_RADIUS`'s own default, which the forced-radius case
/// below has to overrule.
fn default_respawn_radius() -> i32 {
    match vanilla_game_rule(RESPAWN_RADIUS_RULE)
        .expect("the generated catalog holds the respawn radius rule")
        .default_value
    {
        GameRuleValue::Integer(radius) => radius,
        other => panic!("the respawn radius rule is an integer, not {other:?}"),
    }
}

/// The flat fixture's floor is the only solid stack in the world, so every
/// column answers the block above it.
fn flat_floor_top() -> f64 {
    f64::from(FlatFixture::new().unwrap().floor_y() + 1)
}

fn fresh_join_position(world: &mut ServerWorld, connection_id: u64) -> Vec3d {
    let (sender, receiver) = mailbox();
    join_position(&sender, &receiver, world, connection_id)
}

/// [`fresh_join_position`] over a mailbox the caller keeps, so one world can
/// take more than one join without restarting the command sequence.
fn join_position(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) -> Vec3d {
    enqueue(
        sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(connection_id),
            profile: profile("spawn", u128::from(connection_id)),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    snapshot_player(&joined, connection_id).position
}

#[test]
fn restored_spawn_radius_is_clamped_to_its_levels_world_border() {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    candidate.world_borders.insert(
        level_key(Java26BuiltinDimension::Overworld),
        bbb_server_persistence::Java26WorldBorderSettings {
            size: 4.0,
            ..bbb_server_persistence::Java26WorldBorderSettings::default()
        },
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();

    assert_eq!(
        fresh_join_position(&mut world, 1),
        Vec3d {
            x: -1.5,
            y: crate::world::SPAWN_Y,
            z: -1.5,
        }
    );
}

/// The border force is a second `if`, not an `else` on the clamp
/// (`PlayerSpawnFinder.java:54-60`): within one block of the border the radius
/// is 1 whatever the clamp and the game rule left behind.
#[test]
fn a_border_one_block_away_forces_radius_one_over_the_clamp_and_the_rule() {
    let default_radius = default_respawn_radius();
    assert!(
        default_radius > 1,
        "the force is only observable above a radius of 1"
    );
    // Radius 0 searches the suggested column alone; radius 1 opens the walk at
    // the corner one block north-west of it.
    let suggestion = Vec3d {
        x: crate::world::SPAWN_X,
        y: flat_floor_top(),
        z: crate::world::SPAWN_Z,
    };
    let forced = Vec3d {
        x: suggestion.x - 1.0,
        z: suggestion.z - 1.0,
        ..suggestion
    };

    assert_eq!(
        fresh_join_position(&mut spawn_search_world(None, Some(0)), 5),
        suggestion
    );
    // A border of size 2 puts its edge exactly one block from the suggestion.
    assert_eq!(
        fresh_join_position(&mut spawn_search_world(Some(2.0), Some(0)), 6),
        forced
    );
    assert_eq!(
        fresh_join_position(&mut spawn_search_world(Some(0.0), Some(0)), 7),
        forced
    );
    // The clamp fires first here and would leave 0 if the force were an else.
    assert_eq!(
        fresh_join_position(&mut spawn_search_world(Some(0.0), None), 8),
        forced
    );
}

/// The flat generator searches like any other: the answer comes from its own
/// column heights (`PlayerSpawnFinder.java:148-175`), not from the suggestion.
#[test]
fn the_flat_generator_search_answers_from_its_own_column_heights() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (has_skylight, has_ceiling, _) = world.level_spawn_facts(Java26BuiltinDimension::Overworld);
    assert!(has_skylight && !has_ceiling);
    world.set_test_default_spawn(
        Java26BuiltinDimension::Overworld,
        Vec3d {
            x: crate::world::SPAWN_X,
            y: 0.0,
            z: crate::world::SPAWN_Z,
        },
    );

    let radius = f64::from(default_respawn_radius());
    assert_eq!(
        fresh_join_position(&mut world, 9),
        Vec3d {
            x: crate::world::SPAWN_X - radius,
            y: flat_floor_top(),
            z: crate::world::SPAWN_Z - radius,
        }
    );
}

/// `PlayerSpawnFinder.fixupSpawnHeight` (`PlayerSpawnFinder.java:89-103`) walks
/// up out of whatever collides and then back down onto the floor.
#[test]
fn the_direct_height_fixup_rises_out_of_a_solid_stack_and_off_liquid() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Adventure,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let floor = flat_floor_top();
    let buried = Vec3d {
        x: crate::world::SPAWN_X,
        y: 0.0,
        z: crate::world::SPAWN_Z,
    };
    world.set_test_default_spawn(Java26BuiltinDimension::Overworld, buried);
    assert_eq!(
        join_position(&sender, &receiver, &mut world, 10),
        Vec3d { y: floor, ..buried }
    );

    world.set_test_default_spawn(
        Java26BuiltinDimension::Overworld,
        Vec3d {
            y: floor + 100.0,
            ..buried
        },
    );
    assert_eq!(
        join_position(&sender, &receiver, &mut world, 11),
        Vec3d { y: floor, ..buried }
    );

    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".to_string(), "0".to_string())]),
    )
    .expect("built-in Java 26.1 water source state");
    for offset in 0..2 {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos {
                x: buried.x.floor() as i32,
                y: floor as i32 + offset,
                z: buried.z.floor() as i32,
            },
            water,
            SET_BLOCK_UPDATE_ALL,
        ));
    }
    world.set_test_default_spawn(
        Java26BuiltinDimension::Overworld,
        Vec3d { y: floor, ..buried },
    );
    assert_eq!(
        join_position(&sender, &receiver, &mut world, 12),
        Vec3d {
            y: floor + 2.0,
            ..buried
        }
    );

    // The upward walk stops at `getMaxY`, so a stack that reaches the ceiling
    // answers the last block inside the world rather than one above it.
    let stone = block_state_id(FlatFixture::new().unwrap().floor_block_name());
    let column = |y| BlockPos {
        x: buried.x.floor() as i32,
        y,
        z: buried.z.floor() as i32,
    };
    // `Level.setBlock` refuses a position outside build height
    // (`Level.java:217-220`), which is what names the ceiling here.
    let ceiling = (0..=1024)
        .rev()
        .find(|&y| {
            world.set_block(
                Java26BuiltinDimension::Overworld,
                column(y),
                stone,
                SET_BLOCK_UPDATE_ALL,
            )
        })
        .expect("the served overworld has a ceiling");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        column(ceiling - 1),
        stone,
        SET_BLOCK_UPDATE_ALL,
    ));
    world.set_test_default_spawn(
        Java26BuiltinDimension::Overworld,
        Vec3d {
            y: f64::from(ceiling - 1),
            ..buried
        },
    );
    assert_eq!(
        join_position(&sender, &receiver, &mut world, 13),
        Vec3d {
            y: f64::from(ceiling),
            ..buried
        }
    );
}

#[test]
fn adventure_and_no_skylight_use_direct_height_fixup() {
    let mut adventure = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Adventure,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    assert_eq!(
        fresh_join_position(&mut adventure, 2),
        Vec3d {
            x: crate::world::SPAWN_X,
            y: crate::world::SPAWN_Y,
            z: crate::world::SPAWN_Z,
        }
    );

    let mut nether = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let suggestion = Vec3d {
        x: 8.5,
        y: crate::world::SPAWN_Y,
        z: -4.5,
    };
    nether.set_test_default_spawn(Java26BuiltinDimension::Nether, suggestion);
    assert!(!nether.level_spawn_facts(Java26BuiltinDimension::Nether).0);
    assert_eq!(fresh_join_position(&mut nether, 3), suggestion);
}

fn replace_overworld_dimension_type(candidate: &mut Java26WorldCandidate, element_id: &str) {
    let document = &mut candidate.world_gen_settings.document;
    let data = document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "data")
        .unwrap();
    let NbtSummaryValue::Compound(data) = &mut data.value else {
        panic!("world-gen data is a compound")
    };
    let dimensions = data
        .iter_mut()
        .find(|entry| entry.name == "dimensions")
        .unwrap();
    let NbtSummaryValue::Compound(dimensions) = &mut dimensions.value else {
        panic!("dimensions is a compound")
    };
    let overworld = dimensions
        .iter_mut()
        .find(|entry| entry.name == "minecraft:overworld")
        .unwrap();
    let NbtSummaryValue::Compound(overworld) = &mut overworld.value else {
        panic!("overworld stem is a compound")
    };
    overworld
        .iter_mut()
        .find(|entry| entry.name == "type")
        .unwrap()
        .value = NbtSummaryValue::string(element_id);
    candidate.world_gen_settings =
        bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            document.clone(),
            0,
        )
        .unwrap();
}

#[test]
fn restored_ceiling_flat_uses_the_bound_generators_spawn_height() {
    let mut candidate = restore_candidate_without_tickets(40, None, Vec::new());
    replace_overworld_dimension_type(&mut candidate, "minecraft:overworld_caves");
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let facts = world.level_spawn_facts(Java26BuiltinDimension::Overworld);
    assert_eq!(facts, (true, true, Some(crate::world::SPAWN_Y as i32)));

    assert_eq!(
        fresh_join_position(&mut world, 4),
        Vec3d {
            x: -9.5,
            y: crate::world::SPAWN_Y,
            z: -9.5,
        }
    );
}

#[test]
fn spawn_levels_freeze_moving_world_borders_at_the_save_tick() {
    let mut candidate = restore_candidate_without_tickets(40, None, Vec::new());
    candidate.world_borders.insert(
        level_key(Java26BuiltinDimension::Overworld),
        bbb_server_persistence::Java26WorldBorderSettings {
            size: 100.0,
            lerp_time: 20,
            lerp_target: 200.0,
            ..bbb_server_persistence::Java26WorldBorderSettings::default()
        },
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    world.tick(Vec::new());
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    let border = request.world_borders[&level_key(Java26BuiltinDimension::Overworld)];
    assert_eq!(border.size, 105.0);
    assert_eq!(border.lerp_time, 19);
    assert_eq!(border.lerp_target, 200.0);
}

#[test]
fn spawn_restore_staticizes_an_equal_world_border_lerp() {
    let mut candidate = restore_candidate_without_tickets(40, None, Vec::new());
    candidate.world_borders.insert(
        level_key(Java26BuiltinDimension::Overworld),
        bbb_server_persistence::Java26WorldBorderSettings {
            size: 100.0,
            lerp_time: 20,
            lerp_target: 100.0,
            ..bbb_server_persistence::Java26WorldBorderSettings::default()
        },
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    let border = request.world_borders[&level_key(Java26BuiltinDimension::Overworld)];
    assert_eq!(border.size, 100.0);
    assert_eq!(border.lerp_time, 0);
    assert_eq!(border.lerp_target, 100.0);
}

#[test]
fn spawn_save_only_projects_declared_and_served_dimension_borders() {
    let mut candidate = restore_candidate_without_tickets(40, None, Vec::new());
    candidate
        .world_gen_settings
        .dimensions
        .remove(&level_key(Java26BuiltinDimension::End));
    candidate
        .world_gen_settings
        .dimensions
        .get_mut(&level_key(Java26BuiltinDimension::Nether))
        .unwrap()
        .generator = "minecraft:noise".to_string();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(world
        .unserved_levels()
        .contains(&("minecraft:the_nether", "minecraft:noise")));

    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.world_borders.keys().cloned().collect::<Vec<_>>(),
        vec![level_key(Java26BuiltinDimension::Overworld)]
    );
}
