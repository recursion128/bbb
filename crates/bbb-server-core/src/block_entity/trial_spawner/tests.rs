use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_TRIAL_SPAWNER_ID;
use bbb_protocol::packets::{BlockPos, ItemStackSummary};

use super::*;
use crate::block_entity::{new_block_entity, unique_field};

fn pos() -> BlockPos {
    BlockPos { x: 4, y: 5, z: 6 }
}

fn trial_state() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_TRIAL_SPAWNER_ID)
        .expect("minecraft:trial_spawner has a default state")
}

fn spawn_data() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![entry(
        "entity",
        NbtSummaryValue::Compound(vec![entry(
            "id",
            NbtSummaryValue::string("minecraft:breeze"),
        )]),
    )])
}

fn player_list() -> NbtSummaryValue {
    NbtSummaryValue::List(vec![NbtSummaryValue::IntArray(vec![1, 2, 3, 4])])
}

fn normal_config() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("spawn_range", NbtSummaryValue::Int(6)),
        entry(
            "spawn_potentials",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("data", spawn_data()),
                entry("weight", NbtSummaryValue::Int(2)),
            ])]),
        ),
    ])
}

#[test]
fn a_trial_spawner_round_trips_owned_map_codec_fields() {
    let state = trial_state();
    let mut record = new_block_entity(pos(), state)
        .expect("a trial spawner builds")
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("trial spawner save");
    record.push(entry(TAG_SPAWN_DATA, spawn_data()));
    record.push(entry(TAG_NEXT_MOB_SPAWNS_AT, NbtSummaryValue::Long(40)));
    record.push(entry(
        TAG_TARGET_COOLDOWN_LENGTH,
        NbtSummaryValue::Int(1200),
    ));
    record.push(entry(TAG_REGISTERED_PLAYERS, player_list()));
    record.push(entry(TAG_NORMAL_CONFIG, normal_config()));
    let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
        .expect("trial spawner load")
        .expect("a trial spawner loads");
    let saved = loaded
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("trial spawner save");
    let again = crate::block_entity::load_test_fixture(pos(), state, &saved)
        .expect("trial spawner reload")
        .expect("the round-trip still names a trial spawner")
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("trial spawner save");
    for name in [
        TAG_SPAWN_DATA,
        TAG_NEXT_MOB_SPAWNS_AT,
        TAG_TARGET_COOLDOWN_LENGTH,
        TAG_REGISTERED_PLAYERS,
        TAG_NORMAL_CONFIG,
    ] {
        assert_eq!(
            unique_field(&saved, name),
            unique_field(&again, name),
            "{name} must round-trip through the trial-spawner owner"
        );
        assert_eq!(
            unique_field(&saved, name),
            unique_field(&record, name),
            "{name} must survive load"
        );
    }
    assert_eq!(
        unique_field(
            &loaded
                .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                .expect("trial spawner update"),
            TAG_SPAWN_DATA
        ),
        Some(&spawn_data())
    );
    // `TrialSpawnerBlockEntity.getUpdateTag` (`:51-53`) reads the block
    // state, whose default is `inactive`.
    assert_eq!(
        unique_field(
            &loaded
                .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                .expect("trial spawner update"),
            TAG_NEXT_MOB_SPAWNS_AT
        ),
        None
    );
    assert_eq!(
        unique_field(
            &loaded
                .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                .expect("trial spawner update"),
            TAG_NORMAL_CONFIG
        ),
        None
    );
}

/// `TrialSpawnerConfig.DIRECT_CODEC` (`:20-53`): the scalars default, and a
/// config that carries only the weighted lists keeps them verbatim.
#[test]
fn the_config_scalars_default_and_validate() {
    let config = TrialSpawnerConfig::from_nbt(&normal_config()).expect("a valid config");
    assert_eq!(config.spawn_range(), 6);
    assert_eq!(config.ticks_between_spawn(), 40);
    assert_eq!(config.calculate_target_total_mobs(0), 6);
    assert_eq!(config.calculate_target_total_mobs(2), 10);
    assert_eq!(config.calculate_target_simultaneous_mobs(0), 2);
    assert_eq!(config.calculate_target_simultaneous_mobs(3), 5);
    // The weighted list rides through untouched.
    let written = config.to_nbt();
    let NbtSummaryValue::Compound(fields) = &written else {
        panic!("a config writes a compound");
    };
    assert_eq!(
        unique_field(fields, "spawn_potentials"),
        unique_field(
            match &normal_config() {
                NbtSummaryValue::Compound(fields) => fields.clone(),
                _ => unreachable!(),
            }
            .as_slice(),
            "spawn_potentials"
        )
    );
}

fn breeze_potential() -> NbtSummaryValue {
    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        entry("data", spawn_data()),
        entry("weight", NbtSummaryValue::Int(1)),
    ])])
}

fn config_with_potential() -> TrialSpawnerFullConfig {
    TrialSpawnerFullConfig::from_nbt(&[entry(
        TAG_NORMAL_CONFIG,
        NbtSummaryValue::Compound(vec![entry(TAG_SPAWN_POTENTIALS, breeze_potential())]),
    )])
    .expect("a config with one potential")
}

/// Phase 0 puts every detection on a multiple of 20.
fn facts(game_time: i64) -> TrialSpawnerTickFacts {
    TrialSpawnerTickFacts {
        game_time,
        detection_phase: 0,
        can_spawn_in_level: true,
        is_ominous: false,
        players_in_line_of_sight: Vec::new(),
        players_without_sight: Vec::new(),
    }
}

fn random() -> crate::random::GameRandom {
    crate::random::GameRandom::with_seed(7)
}

fn tick_with_world_loot(
    state: TrialSpawnerState,
    data: &mut TrialSpawnerStateData,
    full_config: &TrialSpawnerFullConfig,
    facts: &TrialSpawnerTickFacts,
    random: &GameRandom,
) -> TrialSpawnerTickResult {
    let world = crate::world::ServerWorld::new(crate::tests::datapack_config()).unwrap();
    let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld.into();
    let loot = TrialSpawnerDispensingRoll::new(&world, &dimension, BlockPos { x: 0, y: 64, z: 0 });
    tick_and_get_next(state, data, full_config, facts, random, &loot)
        .expect("the real loot owner accepts the step")
}

fn detected_player(uuid: Uuid) -> TrialSpawnerDetectedPlayer {
    TrialSpawnerDetectedPlayer {
        uuid,
        eye_position: Vec3d {
            x: 1.5,
            y: 2.0,
            z: 3.5,
        },
        has_trial_omen: false,
        has_bad_omen: false,
    }
}

/// `TrialSpawnerState.tickAndGetNext`'s INACTIVE and WAITING arms
/// (`TrialSpawnerState.java`).
#[test]
fn the_state_machine_waits_for_a_player_then_goes_active() {
    let config = config_with_potential();
    let mut data = TrialSpawnerStateData::default();
    let random = random();

    // No spawn data naming a type: INACTIVE stays put, and the empty
    // spawn data `getOrCreateNextSpawnData` caches keeps it there
    // (`TrialSpawnerStateData.java`).
    let mut empty = TrialSpawnerStateData::default();
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Inactive,
            &mut empty,
            &TrialSpawnerFullConfig::default(),
            &facts(0),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );

    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Inactive,
            &mut data,
            &config,
            &facts(0),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Transition(TrialSpawnerState::WaitingForPlayers)
    );

    // The detection scan only runs on the twentieth tick.
    let mut throttled = facts(1);
    throttled.players_in_line_of_sight = vec![detected_player(Uuid::from_u64_pair(1, 2))];
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::WaitingForPlayers,
            &mut data,
            &config,
            &throttled,
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );
    assert!(data.detected_players().is_empty());

    let mut detected = facts(20);
    detected.players_in_line_of_sight = vec![detected_player(Uuid::from_u64_pair(1, 2))];
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::WaitingForPlayers,
            &mut data,
            &config,
            &detected,
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Transition(TrialSpawnerState::Active)
    );
    // `TrialSpawnerStateData.java`: the first player delays the
    // next spawn by 40 ticks.
    assert_eq!(data.next_mob_spawns_at(), 60);

    // A level that cannot spawn resets the statistics and stays.
    let mut blocked = facts(40);
    blocked.can_spawn_in_level = false;
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Active,
            &mut data,
            &config,
            &blocked,
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Transition(TrialSpawnerState::WaitingForPlayers)
    );
    assert!(data.detected_players().is_empty());
    assert_eq!(data.total_mobs_spawned(), 0);
}

/// The ACTIVE arm's spawn cadence and its end
/// (`TrialSpawnerState.java`).
#[test]
fn the_active_arm_spawns_up_to_its_target_then_cools_down() {
    let config = config_with_potential();
    let mut data = TrialSpawnerStateData::default();
    let random = random();
    let mut now = 60;

    // Target total is `total_mobs` (6.0) with no additional players, and
    // at most `simultaneous_mobs` (2.0) may be alive at once
    // (`TrialSpawnerConfig.java`), so each spawned mob dies before
    // the next.
    for _ in 0..6 {
        assert_eq!(
            tick_with_world_loot(
                TrialSpawnerState::Active,
                &mut data,
                &config,
                &facts(now),
                &random,
            )
            .outcome,
            TrialSpawnerTickOutcome::Stay { spawn_mob: true }
        );
        data.note_mob_spawned(
            Uuid::from_u64_pair(u64::from(now as u32), 1),
            now,
            config.active(false),
            &random,
        );
        data.current_mobs_mut().clear();
        now += 40;
    }
    assert_eq!(data.total_mobs_spawned(), 6);
    assert!(data.current_mobs().is_empty());

    // The target is reached and no mob is alive yet tracked: the arm
    // still waits for the tracked set to empty (`:99-105`), which it
    // already is here.
    data.current_mobs_mut().insert(Uuid::from_u64_pair(9, 9));
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Active,
            &mut data,
            &config,
            &facts(now),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );

    data.current_mobs_mut().clear();
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Active,
            &mut data,
            &config,
            &facts(now),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Transition(TrialSpawnerState::WaitingForRewardEjection)
    );
    assert_eq!(
        data.cooldown_ends_at(),
        now + i64::from(TrialSpawnerFullConfig::DEFAULT_TARGET_COOLDOWN_LENGTH)
    );
    assert_eq!(data.total_mobs_spawned(), 0);
    assert_eq!(data.next_mob_spawns_at(), 0);
}

/// `TrialSpawnerState.java`: COOLDOWN re-arms on a player and
/// returns to WAITING_FOR_PLAYERS when its countdown runs out.
#[test]
fn the_cooldown_arm_rearms_or_ends() {
    let config = config_with_potential();
    let mut data = TrialSpawnerStateData::default();
    data.set_cooldown_ends_at(1000);
    let random = random();

    let mut seen = facts(20);
    seen.players_without_sight = vec![Uuid::from_u64_pair(3, 4)];
    // The COOLDOWN arm skips the scan unless the block became ominous.
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Cooldown,
            &mut data,
            &config,
            &seen,
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );

    data.registered_players_mut()
        .insert(Uuid::from_u64_pair(3, 4));
    data.set_total_mobs_spawned(2);
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Cooldown,
            &mut data,
            &config,
            &facts(40),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Transition(TrialSpawnerState::Active)
    );
    assert_eq!(data.total_mobs_spawned(), 0);

    data.registered_players_mut().clear();
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::Cooldown,
            &mut data,
            &config,
            &facts(1000),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::RemoveOminous {
            next: TrialSpawnerState::WaitingForPlayers
        }
    );
    assert_eq!(data.cooldown_ends_at(), 0);
    assert_eq!(data.total_mobs_spawned(), 0);
}

/// A config with one spawn potential, one weighted eject table and a
/// 100-tick cooldown, which is what makes the reward gates measurable.
fn reward_config() -> TrialSpawnerFullConfig {
    TrialSpawnerFullConfig::from_nbt(&[
        entry(
            TAG_NORMAL_CONFIG,
            NbtSummaryValue::Compound(vec![
                entry(TAG_SPAWN_POTENTIALS, breeze_potential()),
                entry(
                    TAG_LOOT_TABLES_TO_EJECT,
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry(
                            "data",
                            NbtSummaryValue::string("minecraft:spawners/trial_chamber/key"),
                        ),
                        entry("weight", NbtSummaryValue::Int(1)),
                    ])]),
                ),
            ]),
        ),
        entry(TAG_TARGET_COOLDOWN_LENGTH, NbtSummaryValue::Int(100)),
    ])
    .expect("a reward config")
}

/// `TrialSpawnerState.java`: the shutter opens 40 ticks after the
/// cooldown it is measured from started.
#[test]
fn the_waiting_for_reward_ejection_arm_waits_forty_ticks_then_opens_the_shutter() {
    let config = reward_config();
    let mut data = TrialSpawnerStateData::default();
    // `cooldownEndsAt - targetCooldownLength` is the instant the gate is
    // measured from, so the shutter is due at 101.
    data.set_cooldown_ends_at(161);
    let random = random();

    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::WaitingForRewardEjection,
            &mut data,
            &config,
            &facts(100),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::WaitingForRewardEjection,
            &mut data,
            &config,
            &facts(101),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::OpenShutter {
            next: TrialSpawnerState::EjectingReward
        }
    );
}

/// `TrialSpawnerState.java`: one ejection per 30 ticks, one
/// detected player per ejection, one table pick for the whole sequence,
/// and the close-shutter arm once the last player is consumed.
#[test]
fn the_ejecting_reward_arm_picks_once_consumes_one_player_per_step_and_closes() {
    let config = reward_config();
    let mut data = TrialSpawnerStateData::default();
    data.set_cooldown_ends_at(161);
    let first = Uuid::from_u64_pair(1, 1);
    let second = Uuid::from_u64_pair(2, 2);
    data.registered_players_mut().insert(first);
    data.registered_players_mut().insert(second);
    let random = random();

    // 102 is not a multiple of 30 from 61, so the modulo gate holds.
    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::EjectingReward,
            &mut data,
            &config,
            &facts(102),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );
    assert_eq!(data.detected_players().len(), 2);
    assert_eq!(data.ejecting_loot_table(), None);

    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::EjectingReward,
            &mut data,
            &config,
            &facts(121),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::EjectReward {
            loot_table: "minecraft:spawners/trial_chamber/key".into()
        }
    );
    data.remove_one_detected_player();
    assert_eq!(
        data.ejecting_loot_table(),
        Some("minecraft:spawners/trial_chamber/key")
    );
    assert_eq!(
        data.detected_players().iter().copied().collect::<Vec<_>>(),
        vec![second]
    );

    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::EjectingReward,
            &mut data,
            &config,
            &facts(151),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::EjectReward {
            loot_table: "minecraft:spawners/trial_chamber/key".into()
        }
    );
    data.remove_one_detected_player();
    assert!(data.detected_players().is_empty());

    assert_eq!(
        tick_with_world_loot(
            TrialSpawnerState::EjectingReward,
            &mut data,
            &config,
            &facts(181),
            &random,
        )
        .outcome,
        TrialSpawnerTickOutcome::CloseShutter {
            next: TrialSpawnerState::Cooldown
        }
    );
    assert_eq!(data.ejecting_loot_table(), None);
}

/// `TrialSpawner.overrideEntityToSpawn` (`:340-344`).
#[test]
fn a_spawn_egg_override_replaces_both_configs_and_clears_the_state_data() {
    let mut entity = TrialSpawnerBlockEntity::default();
    let mut data = TrialSpawnerStateData::default();
    data.registered_players_mut()
        .insert(Uuid::from_u64_pair(5, 6));
    data.set_total_mobs_spawned(3);
    entity.load_additional(&data.to_nbt());
    entity.override_entity_to_spawn("minecraft:breeze");

    assert!(entity.state_data().registered_players().is_empty());
    assert_eq!(entity.state_data().total_mobs_spawned(), 0);
    for config in [entity.config().active(false), entity.config().active(true)] {
        assert_eq!(config.spawn_potentials().len(), 1);
        assert_eq!(
            config.spawn_potentials()[0].data.entity_resource_id(),
            Some(&bbb_protocol::JavaString::from("minecraft:breeze"))
        );
        // The override keeps the other config fields.
        assert_eq!(config.spawn_range(), 4);
    }
}

/// `TrialSpawnerBlockEntity.getUpdateTag` (`:51-53`) over the ACTIVE
/// block state.
#[test]
fn an_active_block_state_carries_the_countdown() {
    let active = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:trial_spawner",
        &std::collections::BTreeMap::from([
            ("ominous".to_string(), "false".to_string()),
            ("trial_spawner_state".to_string(), "active".to_string()),
        ]),
    )
    .expect("minecraft:trial_spawner has that state");
    let mut record = new_block_entity(pos(), active)
        .expect("a trial spawner builds")
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("trial spawner save");
    record.push(entry(TAG_NEXT_MOB_SPAWNS_AT, NbtSummaryValue::Long(40)));
    record.push(entry(TAG_SPAWN_DATA, spawn_data()));
    let loaded = crate::block_entity::load_test_fixture(pos(), active, &record)
        .expect("trial spawner load")
        .expect("a trial spawner loads");
    assert_eq!(
        loaded
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("trial spawner update"),
        vec![
            entry(TAG_NEXT_MOB_SPAWNS_AT, NbtSummaryValue::Long(40)),
            entry(TAG_SPAWN_DATA, spawn_data()),
        ]
    );
}

/// `TrialSpawnerStateData.findPlayerWithOminousEffect` (`:159-177`): the
/// first trial omen wins, else the last bad omen, and the winner names the
/// effect `:130` compares against BAD_OMEN.
#[test]
fn the_activation_picks_the_first_trial_omen_else_the_last_bad_omen() {
    let mut first = detected_player(Uuid::from_u64_pair(1, 1));
    first.has_bad_omen = true;
    let mut second = detected_player(Uuid::from_u64_pair(2, 2));
    second.has_bad_omen = true;

    let mut third = detected_player(Uuid::from_u64_pair(3, 3));
    third.has_trial_omen = true;
    assert_eq!(
        find_player_with_ominous_effect(&[first, second, third]),
        Some((&third, false))
    );

    // A player holding both is the trial omen, so nothing is transformed.
    let mut both = detected_player(Uuid::from_u64_pair(4, 4));
    both.has_trial_omen = true;
    both.has_bad_omen = true;
    assert_eq!(
        find_player_with_ominous_effect(&[both]),
        Some((&both, false))
    );
    assert_eq!(find_player_with_ominous_effect(&[]), None);
}

/// `TrialSpawnerStateData.tryDetectPlayers`'s detection broadcast
/// (`:147-153`) over a real step.
#[test]
fn a_detection_publishes_3013_or_3019_or_nothing() {
    let config = config_with_potential();
    let mut data = TrialSpawnerStateData::default();
    let random = random();
    let alice = Uuid::from_u64_pair(1, 2);
    let mut sighted = facts(20);
    sighted.players_in_line_of_sight = vec![detected_player(alice)];

    let result = tick_with_world_loot(
        TrialSpawnerState::WaitingForPlayers,
        &mut data,
        &config,
        &sighted,
        &random,
    );
    assert_eq!(
        result.detection,
        Some(TrialSpawnerDetectionEffect::Players {
            event_id: 3013,
            detected: 1
        })
    );
    assert!(!result.is_ominous);
    assert_eq!(data.next_mob_spawns_at(), 60);

    // The same player again changes no set, so no event is raised.
    let result = tick_with_world_loot(
        TrialSpawnerState::Active,
        &mut data,
        &config,
        &sighted,
        &random,
    );
    assert_eq!(result.detection, None);

    // An already-ominous spawner raises 3019 instead (`:150`).
    let mut ominous = facts(40);
    ominous.is_ominous = true;
    ominous.players_without_sight = vec![Uuid::from_u64_pair(5, 6)];
    let result = tick_with_world_loot(
        TrialSpawnerState::Active,
        &mut data,
        &config,
        &ominous,
        &random,
    );
    assert_eq!(
        result.detection,
        Some(TrialSpawnerDetectionEffect::Players {
            event_id: 3019,
            detected: 2
        })
    );

    // Off the detection phase nothing is decided at all.
    let result = tick_with_world_loot(
        TrialSpawnerState::Active,
        &mut data,
        &config,
        &facts(41),
        &random,
    );
    assert_eq!(result.detection, None);
}

/// `TrialSpawnerStateData.java` with `:179-199`: the activation
/// picks the player, resets the ominous half of the state data off the
/// ominous config and hands the retired mobs back.
#[test]
fn an_ominous_activation_resets_the_state_data_with_the_ominous_config() {
    let config = ominous_config(7, true);
    let mut data = TrialSpawnerStateData::default();
    let alice = Uuid::from_u64_pair(1, 2);
    let mob = Uuid::from_u64_pair(7, 8);
    data.registered_players_mut().insert(alice);
    data.current_mobs_mut().insert(mob);
    data.set_total_mobs_spawned(3);
    let random = random();
    data.get_or_create_next_spawn_data(config.active(false), &random);
    assert!(data.next_spawn_data().is_some());

    let mut with_bad_omen = detected_player(alice);
    with_bad_omen.has_bad_omen = true;
    let mut facts = facts(20);
    facts.players_in_line_of_sight = vec![with_bad_omen];
    let result = tick_with_world_loot(
        TrialSpawnerState::Active,
        &mut data,
        &config,
        &facts,
        &random,
    );

    assert!(result.is_ominous);
    assert_eq!(
        result.outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );
    assert_eq!(
        result.detection,
        Some(TrialSpawnerDetectionEffect::Ominous(
            TrialSpawnerOminousActivation {
                player: alice,
                eye_position: with_bad_omen.eye_position,
                transform_bad_omen: true,
                removed_mobs: vec![mob],
            }
        ))
    );
    assert!(data.current_mobs().is_empty());
    assert_eq!(data.total_mobs_spawned(), 0);
    // The ominous config's own `ticks_between_spawn`, and the constant
    // `ticksBetweenItemSpawners` (`TrialSpawnerConfig.java`).
    assert_eq!(data.next_mob_spawns_at(), 27);
    assert_eq!(data.cooldown_ends_at(), 180);
    assert_eq!(data.next_spawn_data(), None);
}

/// The same record with an ominous half of `ticks` between spawns.
fn ominous_config(ticks: i32, potentials: bool) -> TrialSpawnerFullConfig {
    let mut ominous = vec![entry(TAG_TICKS_BETWEEN_SPAWN, NbtSummaryValue::Int(ticks))];
    if potentials {
        ominous.push(entry(TAG_SPAWN_POTENTIALS, breeze_potential()));
    }
    TrialSpawnerFullConfig::from_nbt(&[
        entry(
            TAG_NORMAL_CONFIG,
            NbtSummaryValue::Compound(vec![entry(TAG_SPAWN_POTENTIALS, breeze_potential())]),
        ),
        entry(TAG_OMINOUS_CONFIG, NbtSummaryValue::Compound(ominous)),
    ])
    .expect("a config with both halves")
}

/// `TrialSpawnerStateData.java`: an already-ominous spawner in COOLDOWN
/// never scans, while a normal one there can still be turned.
#[test]
fn a_cooldown_scan_is_skipped_once_the_spawner_is_ominous() {
    let config = ominous_config(7, true);
    let mut data = TrialSpawnerStateData::default();
    data.set_cooldown_ends_at(1000);
    let random = random();
    let mut sighted = facts(20);
    sighted.is_ominous = true;
    sighted.players_in_line_of_sight = vec![detected_player(Uuid::from_u64_pair(1, 2))];

    let result = tick_with_world_loot(
        TrialSpawnerState::Cooldown,
        &mut data,
        &config,
        &sighted,
        &random,
    );
    assert_eq!(result.detection, None);
    assert!(data.detected_players().is_empty());
    assert_eq!(
        result.outcome,
        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
    );

    // `:142`: a normal spawner's COOLDOWN scan can still activate it, and
    // `:149` still suppresses the event on that tick.
    let mut with_bad_omen = detected_player(Uuid::from_u64_pair(1, 2));
    with_bad_omen.has_bad_omen = true;
    let mut normal = facts(40);
    normal.players_in_line_of_sight = vec![with_bad_omen];
    let result = tick_with_world_loot(
        TrialSpawnerState::Cooldown,
        &mut data,
        &config,
        &normal,
        &random,
    );
    assert!(result.is_ominous);
    assert!(matches!(
        result.detection,
        Some(TrialSpawnerDetectionEffect::Ominous(_))
    ));
    assert_eq!(
        result.outcome,
        TrialSpawnerTickOutcome::Transition(TrialSpawnerState::Active)
    );
}

fn dispensing_summary(item_id: i32, count: i32) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id),
        count,
        ..ItemStackSummary::empty()
    })
}

/// A config whose ominous half names its own dispensing table.
fn config_with_ominous_dispensing() -> TrialSpawnerFullConfig {
    TrialSpawnerFullConfig::from_nbt(&[entry(
        TAG_OMINOUS_CONFIG,
        NbtSummaryValue::Compound(vec![entry(
            TAG_ITEMS_TO_DROP_WHEN_OMINOUS,
            NbtSummaryValue::string(DEFAULT_ITEMS_TO_DROP_WHEN_OMINOUS),
        )]),
    )])
    .expect("a config with an ominous dispensing table")
}

fn active_data_with_mob() -> TrialSpawnerStateData {
    let mut data = TrialSpawnerStateData::default();
    data.set_next_spawn_data(SpawnerSpawnData::from_nbt(&spawn_data()));
    data
}

/// `TrialSpawnerState.spawnOminousOminousItemSpawner` (`:158-174`): only
/// the ominous ACTIVE arm spawns an item spawner, gated by its cooldown.
#[test]
fn an_ominous_active_step_returns_an_item_and_obeys_the_cooldown() {
    let config = config_with_ominous_dispensing();
    let mut ominous = facts(20);
    ominous.is_ominous = true;
    let world = crate::world::ServerWorld::new(crate::tests::datapack_config_with_loot_table(
        DEFAULT_ITEMS_TO_DROP_WHEN_OMINOUS,
        &serde_json::json!({
            "type": "minecraft:chest",
            "pools": [{"rolls": 1, "entries": [{"type": "minecraft:item", "name": "minecraft:apple"}]}]
        }),
    )).unwrap();
    let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld.into();
    let loot = TrialSpawnerDispensingRoll::new(&world, &dimension, BlockPos { x: 0, y: 64, z: 0 });
    let random = random();
    let mut data = active_data_with_mob();

    let normal = tick_and_get_next(
        TrialSpawnerState::Active,
        &mut data,
        &config,
        &facts(20),
        &random,
        &loot,
    )
    .expect("dispensing tick");
    assert_eq!(normal.spawn_ominous_item, None);

    let allowed = tick_and_get_next(
        TrialSpawnerState::Active,
        &mut data,
        &config,
        &ominous,
        &random,
        &loot,
    )
    .expect("dispensing tick");
    assert_eq!(
        allowed.spawn_ominous_item,
        Some(dispensing_summary(
            bbb_protocol::item_types::VANILLA_ITEM_APPLE_ID,
            1
        ))
    );

    data.set_cooldown_ends_at(21);
    let refused = tick_and_get_next(
        TrialSpawnerState::Active,
        &mut data,
        &config,
        &ominous,
        &random,
        &loot,
    )
    .expect("dispensing tick");
    assert_eq!(refused.spawn_ominous_item, None);

    // `:170`: `ticksBetweenItemSpawners` is the constant 160.
    data.note_ominous_item_spawned(20);
    assert_eq!(data.cooldown_ends_at(), 180);
    assert!(data.is_time_to_spawn_item_spawner(180));
    assert!(!data.is_time_to_spawn_item_spawner(179));
}
