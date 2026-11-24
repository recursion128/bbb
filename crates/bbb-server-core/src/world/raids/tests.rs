use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bbb_protocol::packets::{
    BlockPos, BossEventOperation, GameProfile, NbtSummaryEntry, NbtSummaryValue, Vec3d,
};
use bbb_server_persistence::world_storage::saved_data::Java26Raid;
use bbb_server_persistence::{
    Java26SaveRequest, Java26WorldCandidate, Java26WorldStorage, RegionCompression,
};
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

use crate::tests::{created_fixture_save, datapack_config, spawn_pinned_restore};
use crate::world::JoinContext;
use crate::world::ServerWorld;

use super::*;

mod villagers;

fn raid(status: Java26RaidStatus) -> Java26Raid {
    Java26Raid {
        started: true,
        active: status == Java26RaidStatus::Ongoing,
        ticks_active: 7,
        raid_omen_level: 1,
        groups_spawned: 1,
        cooldown_ticks: 0,
        post_raid_ticks: 0,
        total_health: 20.0,
        group_count: 5,
        status,
        center: RAID_CENTER,
        heroes_of_the_village: BTreeSet::new(),
    }
}

fn manager(rows: &[(i32, Java26RaidStatus)]) -> LevelRaids {
    let rows: Vec<(i32, Java26Raid)> = rows
        .iter()
        .map(|&(id, status)| (id, raid(status)))
        .collect();
    manager_rows(&rows)
}

fn manager_rows(rows: &[(i32, Java26Raid)]) -> LevelRaids {
    let mut saved = Java26Raids::default();
    for (id, row) in rows {
        saved.raids.insert(*id, row.clone());
    }
    saved.next_id = 9;
    saved.tick = 4;
    LevelRaids::from_saved(saved)
}

fn raid_at(status: Java26RaidStatus, center: BlockPos) -> Java26Raid {
    Java26Raid {
        center,
        // `Raid.stop` is the only writer that clears the flag, so a raid that
        // is already over still carries the one its ongoing arm last set.
        active: true,
        ..raid(status)
    }
}

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

const RAID_CENTER: BlockPos = BlockPos { x: 0, y: 64, z: 0 };

#[test]
fn only_stored_ids_resolve_and_joined_raiders_reach_their_wave_and_leader_maps() {
    let mut raids = manager(&[(3, Java26RaidStatus::Ongoing)]);
    assert!(raids.contains(3));
    assert!(!raids.contains(4));

    assert!(raids.add_wave_mob(3, 2, uuid(1)));
    assert!(raids.add_wave_mob(3, 2, uuid(2)));
    assert!(raids.set_leader(3, 2, uuid(1)));
    // `Raid.addWaveMob` keys its set by UUID, so the same raider joining twice
    // is still one member of the wave.
    assert!(raids.add_wave_mob(3, 2, uuid(1)));
    assert_eq!(raids.total_raiders_alive(3), 2);
    assert_eq!(raids.leader(3, 2), Some(uuid(1)));
    assert_eq!(raids.leader(3, 1), None);

    // An id `Raids.get` does not hold has no wave map to join.
    assert!(!raids.add_wave_mob(4, 1, uuid(3)));
    assert!(!raids.set_leader(4, 1, uuid(3)));
    assert_eq!(raids.total_raiders_alive(4), 0);
}

fn pillager_with_raid_id(
    world: &mut ServerWorld,
    x: f64,
    raid_id: Option<i32>,
    wave: i32,
    patrol_leader: bool,
) -> Uuid {
    let (id, uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID,
        Vec3d { x, y: 65.0, z: 0.5 },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID,
            )
            .unwrap(),
        ),
    );
    let entities = &mut world
        .level_mut_by_id(&crate::world::LevelId::from(
            Java26BuiltinDimension::Overworld,
        ))
        .entities;
    let NbtSummaryValue::Compound(mut data) = entities.command_data(id).unwrap() else {
        panic!("a pillager is a compound");
    };
    data.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "Wave" | "PatrolLeader" | "RaidId"
        )
    });
    data.push(NbtSummaryEntry {
        name: "Wave".into(),
        value: NbtSummaryValue::Int(wave),
    });
    data.push(NbtSummaryEntry {
        name: "PatrolLeader".into(),
        value: NbtSummaryValue::Byte(i8::from(patrol_leader)),
    });
    if let Some(raid_id) = raid_id {
        data.push(NbtSummaryEntry {
            name: "RaidId".into(),
            value: NbtSummaryValue::Int(raid_id),
        });
    }
    entities.load_command_data(id, &data).unwrap();
    uuid
}

/// The saved `RaidId` of every raider in one dimension's entity chunks, after
/// the world wrote them back out.
fn saved_raid_ids(request: &Java26SaveRequest) -> BTreeMap<Uuid, Option<i32>> {
    let mut ids = BTreeMap::new();
    for write in &request.entity_writes {
        let data = bbb_server_persistence::Java26EntityChunkData::parse(
            write.document.clone(),
            write.chunk_x,
            write.chunk_z,
        )
        .unwrap();
        for record in data.entities() {
            let raid_id = record.raw().iter().find_map(|entry| match &entry.value {
                NbtSummaryValue::Int(value) if entry.name == "RaidId" => Some(*value),
                _ => None,
            });
            ids.insert(record.uuid(), raid_id);
        }
    }
    ids
}

/// Writes `RaidId` into the saved raiders that carry each `Wave`, which the
/// world itself cannot do: an id it never resolved is one it never writes.
fn inject_raid_ids(request: &mut Java26SaveRequest, by_wave: &[(i32, i32)]) {
    let mut injected = 0;
    for write in &mut request.entity_writes {
        for field in write.document.entries_mut() {
            if field.name != "Entities" {
                continue;
            }
            let NbtSummaryValue::List(entities) = &mut field.value else {
                continue;
            };
            for entity in entities {
                let NbtSummaryValue::Compound(fields) = entity else {
                    continue;
                };
                let wave = fields.iter().find_map(|entry| match entry.value {
                    NbtSummaryValue::Int(value) if entry.name == "Wave" => Some(value),
                    _ => None,
                });
                let Some(&(_, raid_id)) = by_wave.iter().find(|(w, _)| Some(*w) == wave) else {
                    continue;
                };
                fields.push(NbtSummaryEntry {
                    name: "RaidId".into(),
                    value: NbtSummaryValue::Int(raid_id),
                });
                injected += 1;
            }
        }
    }
    assert_eq!(injected, by_wave.len(), "every seeded raider takes an id");
}

fn seeded_raid_candidate(
    storage: &Java26WorldStorage,
    request: &Java26SaveRequest,
    baseline: &Java26WorldCandidate,
    by_wave: &[(i32, i32)],
) -> Java26WorldCandidate {
    let mut request = request.clone();
    inject_raid_ids(&mut request, by_wave);
    request.raids.insert(
        Java26LevelKey::from(Java26BuiltinDimension::Overworld),
        Java26Raids {
            next_id: 4,
            tick: 12,
            raids: BTreeMap::from([(3, raid_row())]),
        },
    );
    storage.save_snapshot(baseline, &request).unwrap();
    storage.load_candidate().unwrap()
}

fn raid_row() -> Java26Raid {
    Java26Raid {
        started: true,
        active: true,
        ticks_active: 40,
        raid_omen_level: 1,
        groups_spawned: 1,
        cooldown_ticks: 0,
        post_raid_ticks: 0,
        total_health: 24.0,
        group_count: 5,
        status: Java26RaidStatus::Ongoing,
        center: BlockPos { x: 0, y: 65, z: 0 },
        heroes_of_the_village: Default::default(),
    }
}

#[test]
fn a_restored_raider_binds_to_its_dimension_raid_and_only_a_bound_one_writes_its_id_back() {
    let root = std::env::temp_dir().join(format!("bbb-raid-binding-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-raid-binding");
    let baseline = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &baseline);
    let bound = pillager_with_raid_id(&mut world, 0.5, Some(3), 2, true);
    let stale = pillager_with_raid_id(&mut world, 2.5, Some(99), 1, false);
    let unbound = pillager_with_raid_id(&mut world, 4.5, None, 0, false);
    let seeding = world
        .save_request(1_786_000_000_100, 10, RegionCompression::Zlib)
        .unwrap();
    let candidate = seeded_raid_candidate(&storage, &seeding, &baseline, &[(2, 3), (1, 99)]);
    drop(world);

    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    let request = world
        .save_request(1_786_000_000_101, 11, RegionCompression::Zlib)
        .unwrap();
    let ids = saved_raid_ids(&request);
    assert_eq!(ids[&bound], Some(3), "raid 3 is in this level's manager");
    assert_eq!(ids[&stale], None, "no raid 99 exists to bind to");
    assert_eq!(ids[&unbound], None);
    assert_eq!(
        request.raids[&Java26LevelKey::from(Java26BuiltinDimension::Overworld)]
            .raids
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![3],
        "binding raiders does not change the stored rows"
    );
    drop(world);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_raids_game_rule_drops_the_stored_raid_and_the_binding_the_raider_held() {
    let root = std::env::temp_dir().join(format!("bbb-raid-rule-off-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-raid-rule-off");
    let baseline = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &baseline);
    let bound = pillager_with_raid_id(&mut world, 0.5, Some(3), 2, true);
    let seeding = world
        .save_request(1_786_000_000_200, 20, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = seeded_raid_candidate(&storage, &seeding, &baseline, &[(2, 3)]);
    candidate.game_rules.insert(
        "minecraft:raids".to_string(),
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
    );
    drop(world);

    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    assert_eq!(
        saved_raid_ids(
            &world
                .save_request(1_786_000_000_201, 21, RegionCompression::Zlib)
                .unwrap()
        )[&bound],
        Some(3),
        "the binding survives restore until a tick runs"
    );
    world.tick(Vec::new());
    let request = world
        .save_request(1_786_000_000_202, 22, RegionCompression::Zlib)
        .unwrap();
    assert!(
        request.raids[&Java26LevelKey::from(Java26BuiltinDimension::Overworld)]
            .raids
            .is_empty()
    );
    assert_eq!(
        request.raids[&Java26LevelKey::from(Java26BuiltinDimension::Overworld)].tick,
        13
    );
    assert_eq!(saved_raid_ids(&request)[&bound], None);
    drop(world);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

const HERO: ConnectionId = ConnectionId(1);
const HERO_UUID: Uuid = Uuid::from_u128(1);

fn overworld() -> Java26LevelKey {
    Java26LevelKey::from(Java26BuiltinDimension::Overworld)
}

/// A world with one joined session, whose profile uuid a raid can record.
fn hero_world() -> ServerWorld {
    let mut world = ServerWorld::new(datapack_config()).expect("the fixture world");
    world.join(
        HERO,
        GameProfile {
            uuid: HERO_UUID,
            name: "hero".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    assert!(world.levels.contains_player(HERO));
    world
}

fn raid_win(world: &ServerWorld) -> i32 {
    world.levels[HERO]
        .cold
        .stats
        .value(custom_stat("minecraft:raid_win"))
}

fn hero(uuid: Uuid, raid_omen_level: i32) -> RaidHero {
    RaidHero {
        uuid,
        raid_omen_level,
    }
}

/// `Raid.java` takes the spectator arm of `isSpectator` before any of the
/// three awards.
#[test]
fn a_spectating_hero_takes_none_of_the_three_awards() {
    let mut world = hero_world();
    world.levels[HERO].game_type = GameType::Spectator;
    let mut events = Vec::new();
    world.award_raid_heroes(
        &world.require_level_id(&overworld()),
        vec![hero(HERO_UUID, 1)],
        &mut events,
    );

    assert!(world
        .player_active_effect_for_test(HERO, HERO_OF_THE_VILLAGE_EFFECT)
        .is_none());
    assert_eq!(raid_win(&world), 0);
    assert!(!world.player_advancement_is_done(HERO, "minecraft:adventure/hero_of_the_village"));
    assert!(events.is_empty());
}

/// `Raid.java`: the wave that just died opens the next cooldown, gives
/// the bar its plain name back and ends the pass there, before the twentieth
fn trigger(village: &[BlockPos]) -> RaidTrigger {
    RaidTrigger {
        spectator: false,
        raids_rule: true,
        can_start_raid: Some(true),
        village_poi: Some(village.to_vec()),
        difficulty: Difficulty::Hard,
        raid_omen_amplifier: Some(0),
    }
}

const TRIGGER_POS: BlockPos = BlockPos { x: 0, y: 64, z: 0 };

/// `Raids.java`: the average of the occupied village records is the
/// centre of the raid `getOrCreateRaid` builds at the level's difficulty, and
/// `getUniqueId` pre-increments so a fresh manager's first raid is id two.
#[test]
fn a_trigger_over_no_raid_creates_one_at_the_village_average_and_absorbs_the_omen() {
    let mut raids = LevelRaids::default();
    let village = [
        BlockPos { x: 10, y: 64, z: 0 },
        BlockPos { x: 13, y: 65, z: 3 },
    ];

    let outcome = raids.create_or_extend_raid(TRIGGER_POS, &trigger(&village));

    assert_eq!(outcome, RaidTriggerOutcome::Raid { id: 2, award: true });
    let row = raids.row(2).expect("the trigger created a raid");
    assert_eq!(row.center, BlockPos { x: 11, y: 64, z: 1 });
    assert_eq!(row.cooldown_ticks, DEFAULT_PRE_RAID_TICKS);
    assert_eq!(row.group_count, num_groups(Difficulty::Hard));
    assert_eq!(row.status, Java26RaidStatus::Ongoing);
    assert!(row.active && !row.started);
    assert_eq!(row.ticks_active, 0);
    assert_eq!(row.total_health, 0.0);
    // `Raid.java`, which the codec constructor does not run.
    assert_eq!(raids.runtime[&2].boss.progress(), 0.0);
    // `Raid.absorbRaidOmen` (`Raid.java`).
    assert_eq!(row.raid_omen_level, 1);
}

/// `Raids.java`: a trigger the POI query answers with nothing keeps
/// the position it was raised at as the centre.
#[test]
fn a_trigger_with_no_village_record_centres_the_raid_on_itself() {
    let mut raids = LevelRaids::default();

    let outcome = raids.create_or_extend_raid(TRIGGER_POS, &trigger(&[]));

    assert_eq!(outcome, RaidTriggerOutcome::Raid { id: 2, award: true });
    assert_eq!(raids.row(2).expect("raid row").center, TRIGGER_POS);
}

/// `Raids.java`: the three gates that answer `null`, and the two facts
/// this level may not be able to answer at all.
#[test]
fn each_gate_refuses_the_trigger_without_creating_a_raid() {
    let refusals = [
        (
            RaidTrigger {
                spectator: true,
                ..trigger(&[])
            },
            RaidTriggerOutcome::NoRaid,
        ),
        (
            RaidTrigger {
                raids_rule: false,
                ..trigger(&[])
            },
            RaidTriggerOutcome::NoRaid,
        ),
        (
            RaidTrigger {
                can_start_raid: Some(false),
                ..trigger(&[])
            },
            RaidTriggerOutcome::NoRaid,
        ),
        (
            RaidTrigger {
                can_start_raid: None,
                ..trigger(&[])
            },
            RaidTriggerOutcome::Unanswered("minecraft:gameplay/can_start_raid"),
        ),
        (
            RaidTrigger {
                village_poi: None,
                ..trigger(&[])
            },
            RaidTriggerOutcome::Unanswered("occupied minecraft:village points of interest"),
        ),
    ];

    for (trigger, expected) in refusals {
        let mut raids = LevelRaids::default();
        assert_eq!(raids.create_or_extend_raid(TRIGGER_POS, &trigger), expected);
        assert!(raids.to_saved().raids.is_empty());
        assert_eq!(raids.to_saved().next_id, 1);
    }
}

/// `Raids.java` over `ServerLevel.getRaidAt`: the raid already at the
/// centre is the one the trigger extends, and the map neither gains a row nor
/// spends an id on it.
#[test]
fn a_trigger_over_an_active_raid_extends_it_rather_than_creating_a_second() {
    let mut raids = manager_rows(&[(3, raid_at(Java26RaidStatus::Ongoing, TRIGGER_POS))]);

    let outcome = raids.create_or_extend_raid(TRIGGER_POS, &trigger(&[]));

    // `raid_at` starts a raid whose first wave has spawned, so no award.
    assert_eq!(
        outcome,
        RaidTriggerOutcome::Raid {
            id: 3,
            award: false
        }
    );
    let saved = raids.to_saved();
    assert_eq!(saved.raids.keys().copied().collect::<Vec<_>>(), vec![3]);
    assert_eq!(saved.next_id, 9);
    assert_eq!(saved.raids[&3].raid_omen_level, 2);
}

/// `Raid.absorbRaidOmen` (`Raid.java`): the amplifier and one raise the
/// level, `Mth.clamp` holds it at the maximum, and only a raid before its first
/// wave owes the stat and the criterion.
#[test]
fn absorbing_a_raid_omen_clamps_the_level_and_only_a_first_wave_raid_is_owed_the_awards() {
    let unstarted = Java26Raid {
        started: false,
        groups_spawned: 0,
        raid_omen_level: 4,
        ..raid_at(Java26RaidStatus::Ongoing, TRIGGER_POS)
    };
    let mut raids = manager_rows(&[(3, unstarted)]);

    let outcome = raids.create_or_extend_raid(
        TRIGGER_POS,
        &RaidTrigger {
            raid_omen_amplifier: Some(3),
            ..trigger(&[])
        },
    );

    assert_eq!(outcome, RaidTriggerOutcome::Raid { id: 3, award: true });
    assert_eq!(
        raids.to_saved().raids[&3].raid_omen_level,
        MAX_RAID_OMEN_LEVEL
    );
}

/// `Raids.java`: a started raid already at its maximum omen level
/// absorbs nothing, and a session carrying no raid omen absorbs nothing either
/// (`Raid.java`).
#[test]
fn a_started_raid_at_its_maximum_and_a_session_without_the_omen_both_absorb_nothing() {
    let maxed = Java26Raid {
        raid_omen_level: MAX_RAID_OMEN_LEVEL,
        ..raid_at(Java26RaidStatus::Ongoing, TRIGGER_POS)
    };
    let mut raids = manager_rows(&[(3, maxed)]);
    assert_eq!(
        raids.create_or_extend_raid(TRIGGER_POS, &trigger(&[])),
        RaidTriggerOutcome::Raid {
            id: 3,
            award: false
        }
    );
    assert_eq!(
        raids.to_saved().raids[&3].raid_omen_level,
        MAX_RAID_OMEN_LEVEL
    );

    let mut raids = manager_rows(&[(3, raid_at(Java26RaidStatus::Ongoing, TRIGGER_POS))]);
    assert_eq!(
        raids.create_or_extend_raid(
            TRIGGER_POS,
            &RaidTrigger {
                raid_omen_amplifier: None,
                ..trigger(&[])
            }
        ),
        RaidTriggerOutcome::Raid {
            id: 3,
            award: false
        }
    );
    assert_eq!(raids.to_saved().raids[&3].raid_omen_level, 1);
}

/// The block a session stands on in [`hero_world`], and the bell beside it
/// that makes the section a village.
const OMEN_POS: BlockPos = BlockPos { x: 0, y: 65, z: 0 };

fn level_raids(world: &ServerWorld, dimension: &Java26LevelKey) -> Java26Raids {
    world.levels[&world.require_level_id(dimension)]
        .raids
        .to_saved()
}

fn raid_trigger_stat(world: &ServerWorld) -> i32 {
    world.levels[HERO]
        .cold
        .stats
        .value(custom_stat("minecraft:raid_trigger"))
}

/// `RaidOmenMobEffect.applyEffectTick` (`RaidOmenMobEffect.java`): the
/// last tick of a raid omen over a recorded position creates the raid, clears
/// the position and takes the instance with it, and the raid it started before
/// its first wave owes the session `Stats.RAID_TRIGGER` (`Raid.java`).
#[test]
fn a_raid_omen_running_out_starts_the_raid_at_the_position_the_session_recorded() {
    let mut world = hero_world();
    world.levels[HERO].cold.raid_omen_position = Some(OMEN_POS);
    world.add_player_mob_effect(
        HERO,
        ActiveMobEffect::new(RAID_OMEN_EFFECT, 1, 1).expect("a vanilla effect id"),
        &mut Vec::new(),
    );

    world.tick(Vec::new());

    let saved = level_raids(&world, &overworld());
    assert_eq!(saved.raids.keys().copied().collect::<Vec<_>>(), vec![2]);
    let row = &saved.raids[&2];
    assert_eq!(row.center, OMEN_POS);
    // The amplifier and one (`Raid.java`).
    assert_eq!(row.raid_omen_level, 2);
    assert_eq!(
        world.levels[HERO].cold.raid_omen_position, None,
        "clears the position the raid was started at"
    );
    assert_eq!(
        world.player_active_effect_for_test(HERO, RAID_OMEN_EFFECT),
        None,
        "the raid omen expires"
    );
    assert_eq!(raid_trigger_stat(&world), 1);
}

/// `ServerPlayer.java:398,417`: the recorded position crosses a rejoin, so an
/// omen that runs out after it still starts the raid.
#[test]
fn a_raid_omen_running_out_after_a_rejoin_starts_the_raid_at_the_saved_position() {
    let mut world = hero_world();
    world.levels[HERO].cold.raid_omen_position = Some(OMEN_POS);
    world.add_player_mob_effect(
        HERO,
        ActiveMobEffect::new(RAID_OMEN_EFFECT, 1, 0).expect("a vanilla effect id"),
        &mut Vec::new(),
    );
    world.remove_player(HERO, &mut Vec::new());
    world.join(
        HERO,
        GameProfile {
            uuid: HERO_UUID,
            name: "hero".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    assert_eq!(world.levels[HERO].cold.raid_omen_position, Some(OMEN_POS));

    world.tick(Vec::new());

    let saved = level_raids(&world, &overworld());
    assert_eq!(saved.raids.len(), 1);
    assert_eq!(saved.raids.values().next().unwrap().center, OMEN_POS);
}

/// `RaidOmenMobEffect.java`: a session with no recorded position leaves
/// the instance to run out on its own, which starts nothing.
#[test]
fn a_raid_omen_without_a_recorded_position_runs_out_without_starting_a_raid() {
    let mut world = hero_world();
    world.add_player_mob_effect(
        HERO,
        ActiveMobEffect::new(RAID_OMEN_EFFECT, 1, 0).expect("a vanilla effect id"),
        &mut Vec::new(),
    );

    world.tick(Vec::new());

    assert!(level_raids(&world, &overworld()).raids.is_empty());
    assert_eq!(
        world.player_active_effect_for_test(HERO, RAID_OMEN_EFFECT),
        None
    );
    assert_eq!(raid_trigger_stat(&world), 0);
}

/// [`hero_world`] with an occupied bell in the session's own section, which is
/// what `ServerLevel.isVillage` answers on (`ServerLevel.java`).
fn hero_world_in_a_village() -> (ServerWorld, BlockPos) {
    let mut world = hero_world();
    let session = world.levels[HERO].position();
    let stood_on = BlockPos {
        x: session.x.floor() as i32,
        y: session.y.floor() as i32,
        z: session.z.floor() as i32,
    };
    let bell = BlockPos {
        x: stood_on.x + 1,
        ..stood_on
    };
    let bell_state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:bell")
            .expect("built-in Java 26.1 block")
            .id,
    )
    .expect("built-in Java 26.1 block state");
    assert!(world.set_block(Java26BuiltinDimension::Overworld, bell, bell_state, 0));
    assert!(world.acquire_poi_ticket(&overworld(), [bell.x, bell.y, bell.z]));
    assert_eq!(
        world.is_close_to_village(&overworld(), [stood_on.x, stood_on.y, stood_on.z], 1),
        Some(true)
    );
    (world, stood_on)
}

fn effect_duration(world: &ServerWorld, resource_id: &str) -> Option<i32> {
    world
        .player_active_effect_for_test(HERO, resource_id)
        .map(|effect| effect.duration())
}

/// `BadOmenMobEffect.applyEffectTick` (`BadOmenMobEffect.java`): a
/// `BadOmenMobEffect.java:20-34`: a non-spectator session carrying bad omen
/// into a village in non-peaceful difficulty converts it into a 600-tick raid
/// omen at the same amplification and records where it stood.
#[test]
fn bad_omen_carried_into_a_village_converts_to_raid_omen() {
    let (mut world, stood_on) = hero_world_in_a_village();
    for (resource_id, amplifier) in [("minecraft:bad_omen", 1), ("minecraft:speed", 0)] {
        world.add_player_mob_effect(
            HERO,
            ActiveMobEffect::new(resource_id, 40, amplifier).expect("a vanilla effect id"),
            &mut Vec::new(),
        );
    }

    world.tick(Vec::new());

    assert_eq!(effect_duration(&world, "minecraft:bad_omen"), None);
    assert_eq!(effect_duration(&world, RAID_OMEN_EFFECT), Some(599));
    assert_eq!(
        world
            .player_active_effect_for_test(HERO, RAID_OMEN_EFFECT)
            .map(|effect| effect.amplifier()),
        Some(1)
    );
    assert_eq!(world.levels[HERO].cold.raid_omen_position, Some(stood_on));
    assert_eq!(effect_duration(&world, "minecraft:speed"), Some(39));
    assert!(level_raids(&world, &overworld()).raids.is_empty());
}

/// `BadOmenMobEffect.java`: outside a village, and beside a raid already
/// at its maximum omen level, bad omen hands out nothing and keeps ticking.
#[test]
fn bad_omen_outside_a_village_or_over_a_maxed_raid_hands_out_no_raid_omen() {
    let mut world = hero_world();
    world.add_player_mob_effect(
        HERO,
        ActiveMobEffect::new("minecraft:bad_omen", 40, 0).expect("a vanilla effect id"),
        &mut Vec::new(),
    );
    world.tick(Vec::new());
    assert_eq!(effect_duration(&world, RAID_OMEN_EFFECT), None);
    assert_eq!(effect_duration(&world, "minecraft:bad_omen"), Some(39));

    let (mut world, stood_on) = hero_world_in_a_village();
    let maxed = Java26Raid {
        raid_omen_level: MAX_RAID_OMEN_LEVEL,
        ..raid_at(Java26RaidStatus::Ongoing, stood_on)
    };
    world
        .levels
        .get_mut(&world.require_level_id(&overworld()))
        .expect("the overworld")
        .raids = manager_rows(&[(3, maxed)]);
    world.add_player_mob_effect(
        HERO,
        ActiveMobEffect::new("minecraft:bad_omen", 40, 0).expect("a vanilla effect id"),
        &mut Vec::new(),
    );

    world.tick(Vec::new());

    assert_eq!(effect_duration(&world, RAID_OMEN_EFFECT), None);
    assert_eq!(effect_duration(&world, "minecraft:bad_omen"), Some(39));
    assert_eq!(world.levels[HERO].cold.raid_omen_position, None);
}

/// `Raids.java`: the raid a trigger starts is centred on the average
/// of the occupied village records the level answers with, not on the position
/// the omen was recorded at.
#[test]
fn the_raid_a_trigger_starts_is_centred_on_the_occupied_village_records() {
    let (mut world, stood_on) = hero_world_in_a_village();
    world.levels[HERO].cold.raid_omen_position = Some(stood_on);
    world.add_player_mob_effect(
        HERO,
        ActiveMobEffect::new(RAID_OMEN_EFFECT, 1, 0).expect("a vanilla effect id"),
        &mut Vec::new(),
    );

    world.tick(Vec::new());

    let saved = level_raids(&world, &overworld());
    assert_eq!(
        saved.raids[&2].center,
        BlockPos {
            x: stood_on.x + 1,
            ..stood_on
        },
        "the one occupied bell is the whole average"
    );
}

/// `ServerLevel.getRaidAt(pos) != null` (`ServerLevel.java`) over
/// `Raids.getNearbyRaid(pos, 9216)` (`Raids.java`), which is the second
/// disjunct of `Raider.hasRaid` (`Raider.java`).
///
/// The radius is exclusive on both counts: a raid whose `isActive()` is false
/// is not near anything, and a centre 96 blocks away — `9216` squared — is the
/// first distance the walk does not take.
#[test]
fn get_raid_at_answers_the_active_raids_that_cover_a_position() {
    let ongoing = manager(&[(1, Java26RaidStatus::Ongoing)]);
    assert!(ongoing.raid_at(RAID_CENTER));
    assert!(ongoing.raid_at(BlockPos {
        x: RAID_CENTER.x + 95,
        y: RAID_CENTER.y,
        z: RAID_CENTER.z,
    }));
    assert!(!ongoing.raid_at(BlockPos {
        x: RAID_CENTER.x + 96,
        y: RAID_CENTER.y,
        z: RAID_CENTER.z,
    }));
    assert!(!ongoing.raid_at(BlockPos {
        x: RAID_CENTER.x,
        y: RAID_CENTER.y,
        z: RAID_CENTER.z + 97,
    }));

    let stopped = manager(&[(1, Java26RaidStatus::Victory)]);
    assert!(!stopped.raid_at(RAID_CENTER));
}
fn raid_world(rows: &[(i32, Java26Raid)], village: bool, difficulty: Difficulty) -> ServerWorld {
    let mut world = if village {
        hero_world_in_a_village().0
    } else {
        hero_world()
    };
    world.admin.set_difficulty(difficulty);
    let chunks: Vec<_> = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| bbb_protocol::packets::ChunkPos { x, z }))
        .collect();
    world.set_chunks_forced(Java26BuiltinDimension::Overworld.into(), true, &chunks);
    world.tick(Vec::new());
    world.levels[HERO].set_position(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    world
        .level_mut_by_id(&Java26BuiltinDimension::Overworld.into())
        .raids = manager_rows(rows);
    world
}

fn raid_step(world: &mut ServerWorld) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    world.tick_level_raids(&Java26BuiltinDimension::Overworld.into(), &mut events);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorldEvent::ConsoleMessage(_))),
        "{events:?}"
    );
    events
}

fn raid_manager(world: &ServerWorld) -> &LevelRaids {
    &world
        .level_by_id(&Java26BuiltinDimension::Overworld.into())
        .raids
}

fn raid_manager_mut(world: &mut ServerWorld) -> &mut LevelRaids {
    &mut world
        .level_mut_by_id(&Java26BuiltinDimension::Overworld.into())
        .raids
}

fn boss_operations(events: &[WorldEvent]) -> Vec<&BossEventOperation> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::BossEvent(event),
            } if *connection_id == HERO => Some(&event.operation),
            _ => None,
        })
        .collect()
}

fn wave_world(
    groups_spawned: i32,
    group_count: i32,
    omen: i32,
    difficulty: Difficulty,
) -> ServerWorld {
    let mut world = raid_world(
        &[(
            3,
            Java26Raid {
                groups_spawned,
                group_count,
                raid_omen_level: omen,
                cooldown_ticks: 1,
                total_health: 0.0,
                ticks_active: 19,
                ..raid(Java26RaidStatus::Ongoing)
            },
        )],
        true,
        difficulty,
    );
    let chunks: Vec<_> = (-4..=4)
        .flat_map(|x| (-4..=4).map(move |z| bbb_protocol::packets::ChunkPos { x, z }))
        .collect();
    world.set_chunks_forced(Java26BuiltinDimension::Overworld.into(), true, &chunks);
    world
}

fn wave_entities(world: &ServerWorld, wave: i32) -> Vec<&crate::entity::Entity> {
    let level = world.level_by_id(&Java26BuiltinDimension::Overworld.into());
    raid_manager(world)
        .runtime
        .get(&3)
        .unwrap()
        .raiders
        .get(&wave)
        .unwrap()
        .keys()
        .map(|uuid| {
            let id = level.entities.entity_id_for_uuid(*uuid).unwrap();
            level.entities.get(id).unwrap()
        })
        .collect()
}

#[test]
fn a_world_raid_spawns_its_wave_counts_leader_and_total_health() {
    let mut world = wave_world(3, 7, 1, Difficulty::Hard);
    let events = raid_step(&mut world);
    assert!(!boss_operations(&events).is_empty());
    let entities = wave_entities(&world, 4);
    let count = |kind| {
        entities
            .iter()
            .filter(|entity| entity.entity_type_id() == kind)
            .count()
    };
    assert_eq!(count(VANILLA_ENTITY_TYPE_WITCH_ID), 3);
    assert_eq!(count(VANILLA_ENTITY_TYPE_EVOKER_ID), 0);
    assert_eq!(count(VANILLA_ENTITY_TYPE_RAVAGER_ID), 0);
    assert!((1..=3).contains(&count(VANILLA_ENTITY_TYPE_VINDICATOR_ID)));
    assert!((4..=6).contains(&count(VANILLA_ENTITY_TYPE_PILLAGER_ID)));
    let row = raid_manager(&world).row(3).unwrap();
    assert_eq!(row.groups_spawned, 4);
    assert_eq!(
        row.total_health,
        entities
            .iter()
            .map(|entity| entity.living().unwrap().health())
            .sum::<f32>()
    );
    let leaders: Vec<_> = entities
        .iter()
        .filter(|entity| crate::entity::patrol_spawn_state(entity).unwrap().1)
        .collect();
    assert_eq!(leaders.len(), 1);
    assert_eq!(raid_manager(&world).leader(3, 4), Some(leaders[0].uuid()));
}

#[test]
fn a_world_raid_bonus_wave_uses_its_group_count_and_is_the_last_wave() {
    let mut world = wave_world(3, 3, 2, Difficulty::Easy);
    raid_step(&mut world);
    let entities = wave_entities(&world, 4);
    let count = |kind| {
        entities
            .iter()
            .filter(|entity| entity.entity_type_id() == kind)
            .count()
    };
    assert_eq!(count(VANILLA_ENTITY_TYPE_EVOKER_ID), 0);
    assert_eq!(count(VANILLA_ENTITY_TYPE_WITCH_ID), 0);
    assert_eq!(count(VANILLA_ENTITY_TYPE_RAVAGER_ID), 1);
    let row = raid_manager(&world).row(3).unwrap();
    assert_eq!(row.groups_spawned, 4);
    assert!(!has_more_waves(row));
}

#[test]
fn world_raid_ravagers_carry_the_riders_their_wave_selects() {
    for (groups_spawned, expected) in [
        (4, vec![VANILLA_ENTITY_TYPE_PILLAGER_ID]),
        (
            6,
            vec![
                VANILLA_ENTITY_TYPE_EVOKER_ID,
                VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            ],
        ),
    ] {
        let mut world = wave_world(groups_spawned, 7, 1, Difficulty::Hard);
        raid_step(&mut world);
        let level = world.level_by_id(&Java26BuiltinDimension::Overworld.into());
        let mut riders: Vec<_> = wave_entities(&world, groups_spawned + 1)
            .iter()
            .filter_map(|entity| {
                let vehicle = level.entities.vehicle(entity.id())?;
                assert_eq!(
                    level.entities.get(vehicle).unwrap().entity_type_id(),
                    VANILLA_ENTITY_TYPE_RAVAGER_ID
                );
                Some(entity.entity_type_id())
            })
            .collect();
        let mut expected = expected;
        riders.sort_unstable();
        expected.sort_unstable();
        assert_eq!(riders, expected);
    }
}

#[test]
fn a_world_raid_updates_health_leadership_and_the_players_bar_when_a_raider_leaves() {
    let mut world = raid_world(
        &[(
            3,
            Java26Raid {
                ticks_active: 19,
                total_health: 40.0,
                ..raid(Java26RaidStatus::Ongoing)
            },
        )],
        true,
        Difficulty::Hard,
    );
    let far = pillager_with_raid_id(&mut world, 112.0, Some(3), 1, true);
    let near = pillager_with_raid_id(&mut world, 111.0, Some(3), 1, false);
    let dimension = Java26BuiltinDimension::Overworld.into();
    for (uuid, health) in [(far, 20.0), (near, 10.0)] {
        let level = world.level_mut_by_id(&dimension);
        let id = level.entities.entity_id_for_uuid(uuid).unwrap();
        level
            .entities
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_health(health);
        assert!(level.raids.add_wave_mob(3, 1, uuid));
    }
    assert!(raid_manager_mut(&mut world).set_leader(3, 1, far));
    let events = raid_step(&mut world);
    assert_eq!(raid_manager(&world).total_raiders_alive(3), 1);
    assert_eq!(raid_manager(&world).row(3).unwrap().total_health, 20.0);
    assert_eq!(raid_manager(&world).leader(3, 1), None);
    let progress =
        boss_operations(&events)
            .into_iter()
            .rev()
            .find_map(|operation| match operation {
                BossEventOperation::Add {
                    progress,
                    color,
                    overlay,
                    ..
                } => {
                    assert_eq!(*color, BossBarColor::Red);
                    assert_eq!(*overlay, BossBarOverlay::Notched10);
                    Some(*progress)
                }
                BossEventOperation::UpdateProgress { progress } => Some(*progress),
                _ => None,
            });
    assert_eq!(progress, Some(0.5));
    {
        let mut position = world.levels[HERO].position();
        position.x = 1000.0;
        world.levels[HERO].set_position(position);
    }
    raid_manager_mut(&mut world)
        .saved
        .raids
        .get_mut(&3)
        .unwrap()
        .ticks_active = 19;
    assert!(boss_operations(&raid_step(&mut world)).contains(&&BossEventOperation::Remove));
}

#[test]
fn hurting_a_raider_lowers_its_raids_bar() {
    let mut world = raid_world(
        &[(
            3,
            Java26Raid {
                ticks_active: 19,
                total_health: 24.0,
                ..raid(Java26RaidStatus::Ongoing)
            },
        )],
        true,
        Difficulty::Hard,
    );
    let raider = pillager_with_raid_id(&mut world, 111.0, Some(3), 1, false);
    assert!(raid_manager_mut(&mut world).add_wave_mob(3, 1, raider));
    raid_step(&mut world);
    assert_eq!(raid_manager(&world).runtime[&3].boss.progress(), 1.0);
    let id = world
        .level_by_id(&Java26BuiltinDimension::Overworld.into())
        .entities
        .entity_id_for_uuid(raider)
        .unwrap();
    assert!(crate::entity::raider::join_raid(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap(),
        3,
        1
    ));
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            id,
            "minecraft:generic",
            6.0,
            None,
        )
        .unwrap();
    assert!(outcome.succeeded);
    assert_eq!(raid_manager(&world).runtime[&3].boss.progress(), 0.75);
}

#[test]
fn a_world_raid_loses_without_a_village_and_peaceful_clears_it() {
    let mut started = raid_world(
        &[(3, raid(Java26RaidStatus::Ongoing))],
        false,
        Difficulty::Hard,
    );
    raid_step(&mut started);
    assert_eq!(
        raid_manager(&started).row(3).unwrap().status,
        Java26RaidStatus::Loss
    );
    let mut unstarted = raid_world(
        &[(
            3,
            Java26Raid {
                groups_spawned: 0,
                ..raid(Java26RaidStatus::Ongoing)
            },
        )],
        false,
        Difficulty::Hard,
    );
    raid_step(&mut unstarted);
    assert_eq!(
        raid_manager(&unstarted).row(3).unwrap().status,
        Java26RaidStatus::Stopped
    );
    let mut peaceful = raid_world(
        &[(3, raid(Java26RaidStatus::Ongoing))],
        true,
        Difficulty::Peaceful,
    );
    raid_step(&mut peaceful);
    raid_step(&mut peaceful);
    assert!(raid_manager(&peaceful).to_saved().raids.is_empty());
}

#[test]
fn a_world_raid_victory_awards_its_hero_and_publishes_the_result() {
    let mut world = raid_world(
        &[(
            3,
            Java26Raid {
                ticks_active: 19,
                raid_omen_level: 3,
                groups_spawned: 6,
                post_raid_ticks: 39,
                heroes_of_the_village: BTreeSet::from([HERO_UUID]),
                ..raid(Java26RaidStatus::Ongoing)
            },
        )],
        true,
        Difficulty::Hard,
    );
    raid_step(&mut world);
    raid_step(&mut world);
    assert_eq!(
        raid_manager(&world).row(3).unwrap().status,
        Java26RaidStatus::Victory
    );
    let effect = world
        .player_active_effect_for_test(HERO, HERO_OF_THE_VILLAGE_EFFECT)
        .unwrap();
    assert_eq!(effect.duration(), HERO_OF_THE_VILLAGE_DURATION);
    assert_eq!(effect.amplifier(), 2);
    assert!(!effect.ambient());
    assert!(!effect.show_particles());
    assert!(effect.show_icon());
    assert_eq!(raid_win(&world), 1);
    assert!(world.player_advancement_is_done(HERO, "minecraft:adventure/hero_of_the_village"));
    let mut result = Vec::new();
    for _ in 0..20 {
        result.extend(raid_step(&mut world));
    }
    assert!(boss_operations(&result).iter().any(|operation| matches!(operation, BossEventOperation::UpdateName { name } if name.component() == &crate::commands::translatable("event.minecraft.raid.victory.full", Vec::new()))));
}

#[test]
fn a_world_raid_victory_awards_hero_effect_to_a_non_player_living_entity() {
    let mut world = raid_world(&[], true, Difficulty::Hard);
    let golem_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID;
    let horse_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID;
    let (golem_id, golem_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        golem_type,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(golem_type).unwrap(),
        ),
    );
    let (horse_id, horse_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        horse_type,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(horse_type).unwrap(),
        ),
    );
    let hero_entity_id = world.player_entity_id_for_test(HERO);
    let dimension = world.require_level_id(&overworld());
    assert!(world
        .level_mut_by_id(&dimension)
        .entities
        .attach_test_player(
            hero_entity_id,
            horse_id,
            crate::entity::ExternalPlayerCanRide::Allowed,
        ));
    let mut initial_events = Vec::new();
    world.apply_level_entity_lifecycle_by_key(&dimension, &mut initial_events);
    world.level_mut_by_id(&dimension).raids = manager_rows(&[(
        3,
        Java26Raid {
            ticks_active: 19,
            raid_omen_level: 3,
            groups_spawned: 6,
            post_raid_ticks: 39,
            heroes_of_the_village: BTreeSet::from([golem_uuid, horse_uuid]),
            ..raid(Java26RaidStatus::Ongoing)
        },
    )]);

    raid_step(&mut world);
    let victory_events = raid_step(&mut world);

    assert_eq!(
        raid_manager(&world).row(3).unwrap().status,
        Java26RaidStatus::Victory
    );
    for entity_id in [golem_id, horse_id] {
        let effect = world
            .level_by_id(&dimension)
            .entities
            .get(entity_id)
            .unwrap()
            .living()
            .unwrap()
            .active_effect(HERO_OF_THE_VILLAGE_EFFECT)
            .expect("living raid hero receives Hero of the Village");
        assert_eq!(effect.duration(), HERO_OF_THE_VILLAGE_DURATION);
        assert_eq!(effect.amplifier(), 2);
        assert!(!effect.ambient());
        assert!(!effect.show_particles());
        assert!(effect.show_icon());
    }
    assert!(victory_events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::UpdateMobEffect(packet),
        } if *connection_id == HERO
            && packet.entity_id == horse_id
            && packet.duration_ticks == HERO_OF_THE_VILLAGE_DURATION
            && packet.amplifier == 2
    )));
    assert!(world
        .player_active_effect_for_test(HERO, HERO_OF_THE_VILLAGE_EFFECT)
        .is_none());
    assert_eq!(raid_win(&world), 0);
    assert!(!world.player_advancement_is_done(HERO, "minecraft:adventure/hero_of_the_village"));
}

#[test]
fn a_world_raid_expires_after_its_timeout_or_celebration() {
    let mut timed_out = raid_world(
        &[(
            3,
            Java26Raid {
                ticks_active: RAID_TIMEOUT_TICKS - 1,
                ..raid(Java26RaidStatus::Ongoing)
            },
        )],
        true,
        Difficulty::Hard,
    );
    raid_step(&mut timed_out);
    raid_step(&mut timed_out);
    assert!(raid_manager(&timed_out).to_saved().raids.is_empty());
    for status in [Java26RaidStatus::Victory, Java26RaidStatus::Loss] {
        let mut world = raid_world(&[(3, raid(status))], true, Difficulty::Hard);
        for _ in 0..=MAX_CELEBRATION_TICKS {
            raid_step(&mut world);
        }
        assert!(raid_manager(&world).to_saved().raids.is_empty());
    }
}

#[test]
fn an_unloaded_raid_center_hides_the_players_bar_and_reloading_it_restores_the_bar() {
    let mut world = raid_world(
        &[(
            3,
            Java26Raid {
                ticks_active: 19,
                ..raid(Java26RaidStatus::Ongoing)
            },
        )],
        true,
        Difficulty::Hard,
    );
    let raider = pillager_with_raid_id(&mut world, 0.5, Some(3), 1, false);
    raid_manager_mut(&mut world).add_wave_mob(3, 1, raider);
    assert!(boss_operations(&raid_step(&mut world))
        .iter()
        .any(|operation| matches!(operation, BossEventOperation::Add { .. })));
    raid_manager_mut(&mut world)
        .saved
        .raids
        .get_mut(&3)
        .unwrap()
        .center
        .x = 1_000_000;
    assert!(boss_operations(&raid_step(&mut world)).contains(&&BossEventOperation::Remove));
    let row = raid_manager(&world).row(3).unwrap();
    assert!(!row.active);
    assert_eq!(row.status, Java26RaidStatus::Ongoing);
    raid_manager_mut(&mut world)
        .saved
        .raids
        .get_mut(&3)
        .unwrap()
        .center = RAID_CENTER;
    assert!(boss_operations(&raid_step(&mut world))
        .iter()
        .any(|operation| matches!(operation, BossEventOperation::Add { .. })));
    assert!(raid_manager(&world).row(3).unwrap().active);
}

mod autojoin;
mod banner;
mod celebration;
mod doors;
mod scene;
mod travel;
mod village;

/// Raider.java:124-126: a raider a player kills during its raid makes that
/// player a hero, whom winning the raid will reward.
#[test]
fn a_player_killing_a_raider_becomes_a_hero_of_its_raid() {
    let mut world = wave_world(3, 7, 1, Difficulty::Hard);
    raid_step(&mut world);
    let raider = wave_entities(&world, 4)[0].id();
    let hero_entity = world.player_entity_id_for_test(HERO);
    let source = crate::commands::CommandDamageSourceEntity::Player {
        user: crate::admin::NameAndId {
            id: HERO_UUID,
            name: "hero".into(),
        }
        .into(),
        entity_id: hero_entity,
        position: Vec3d::default(),
        instabuild: false,
        may_build: true,
        spectator: false,
        display_name: NbtSummaryValue::string("hero"),
    };

    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            raider,
            "minecraft:generic",
            1000.0,
            Some(source),
        )
        .expect("the raider takes the damage");

    assert!(outcome.succeeded);
    assert_eq!(
        raid_manager(&world).row(3).unwrap().heroes_of_the_village,
        BTreeSet::from([HERO_UUID])
    );
}

/// `Raider.java:115-133`: a dying raider leaves its raid at once, between the
/// twenty-tick `updateRaiders` passes, and keeps the raid's total health.
#[test]
fn a_dying_raider_leaves_its_raid_at_once_and_keeps_the_total_health() {
    let mut world = wave_world(3, 7, 1, Difficulty::Hard);
    raid_step(&mut world);
    let alive = raid_manager(&world).total_raiders_alive(3);
    let total = raid_manager(&world).row(3).unwrap().total_health;
    let raider = wave_entities(&world, 4)[0].id();
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            raider,
            "minecraft:generic",
            1000.0,
            None,
        )
        .expect("the raider takes the damage");
    assert!(outcome.succeeded);
    assert_ne!(raid_manager(&world).row(3).unwrap().ticks_active % 20, 19);
    raid_step(&mut world);
    assert_eq!(raid_manager(&world).total_raiders_alive(3), alive - 1);
    assert_eq!(raid_manager(&world).row(3).unwrap().total_health, total);
}

/// `Raid.java:465-498` with `Raid.java:612-626`: a raider its chunk unloaded
/// takes its health off the raid's total at the next `updateRaiders` pass.
#[test]
fn a_raider_unloaded_with_its_chunk_takes_its_health_off_the_total() {
    let mut world = wave_world(3, 7, 1, Difficulty::Hard);
    raid_step(&mut world);
    let total = raid_manager(&world).row(3).unwrap().total_health;
    let raider = wave_entities(&world, 4)[0];
    let (id, health) = (raider.id(), raider.living().unwrap().health());
    world
        .level_mut_by_id(&Java26BuiltinDimension::Overworld.into())
        .entities
        .remove(id, crate::entity::RemovalReason::UnloadedToChunk)
        .expect("the stored raider");
    raid_manager_mut(&mut world)
        .saved
        .raids
        .get_mut(&3)
        .unwrap()
        .ticks_active = 39;
    raid_step(&mut world);
    assert_eq!(
        raid_manager(&world).row(3).unwrap().total_health,
        total - health
    );
}
