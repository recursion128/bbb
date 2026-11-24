//! The statistics half of the save: every counter this server can move has to
//! survive `PlayerList.save` into `players/stats/<uuid>.json` and read back.

use super::*;

/// A storage-backed flat world with one session joined, for the counters below:
/// every one of them has to survive `save_snapshot` into
/// `players/stats/<uuid>.json`.
fn stats_world(
    name: &str,
) -> (
    std::path::PathBuf,
    WorldSavePaths,
    Java26WorldStorage,
    Java26WorldCandidate,
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
) {
    stats_world_with(
        name,
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    )
}

/// [`stats_world`] over a chosen configuration and session list.
fn stats_world_with(
    name: &str,
    config: ServerWorldConfig,
    sessions: &[(u64, &str, u128)],
) -> (
    std::path::PathBuf,
    WorldSavePaths,
    Java26WorldStorage,
    Java26WorldCandidate,
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
) {
    let root = std::env::temp_dir().join(format!("bbb-client-world-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let fixture = FlatFixture::new().unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: format!("bbb-{name}"),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();
    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config, &candidate).unwrap();
    for &(connection_id, name, uuid) in sessions {
        join_loaded(&sender, &receiver, &mut world, connection_id, name, uuid);
    }
    (root, paths, storage, candidate, sender, receiver, world)
}

/// The counter as `players/stats/<uuid>.json` holds it after one save.
fn saved_stat_value(
    storage: &Java26WorldStorage,
    candidate: &Java26WorldCandidate,
    paths: &WorldSavePaths,
    world: &mut ServerWorld,
    uuid: uuid::Uuid,
    stat: bbb_server_types::stats::StatKey,
) -> i32 {
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(candidate, &request).unwrap();
    bbb_server_types::stats::ServerStatsCounter::parse_bytes(
        &std::fs::read(paths.player_stats_file(uuid)).expect("the save wrote the counter"),
    )
    .expect("the document this world writes is one it reads")
    .value(stat)
}

/// `ItemEntity.playerTouch` (`ItemEntity.java:339-353`) awards
/// `Stats.ITEM_PICKED_UP` for the item the stack held (`:342`) counted by the
/// stack's whole original size (`:343`, `:351`), not by the part the inventory
/// found room for.
#[test]
fn a_picked_up_item_counts_into_the_statistics_file() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world("picked-up");
    let uuid = profile("newcomer", 1).uuid;
    let picked_up = bbb_server_types::stats::StatKey::new(
        bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_PICKED_UP_ID,
        item_id("minecraft:dandelion"),
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), picked_up), 0);

    let standing = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, standing);
    world.spawn_ground_item_for_test(
        Java26BuiltinDimension::Overworld,
        standing,
        &ItemStackSummary {
            item_id: Some(item_id("minecraft:dandelion")),
            count: 7,
            component_patch: Default::default(),
        },
    );
    let mut ticks = 0;
    while world.player_stat_for_test(ConnectionId(1), picked_up) == 0 {
        world.tick(receiver.drain());
        ticks += 1;
        assert!(ticks < 40, "the drop was never picked up");
    }
    // `:343`, `:351`: the whole stack, not the one slot it landed in.
    assert_eq!(world.player_stat_for_test(ConnectionId(1), picked_up), 7);

    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, picked_up),
        7
    );
    let reloaded = storage.load_candidate().unwrap();
    let mut restored = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &reloaded,
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut restored, 2, "newcomer", 1);
    assert_eq!(restored.player_stat_for_test(ConnectionId(2), picked_up), 7);
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `ServerPlayer.checkMovementStatistics` (`ServerPlayer.java:1420-1468`) off the
/// delta `handleMovePlayer` hands it (`ServerGamePacketListenerImpl.java:1175`):
/// one metre of ground movement is a hundred centimetres on exactly one counter.
#[test]
fn walked_centimetres_count_into_the_statistics_file() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world("walked");
    let uuid = profile("newcomer", 1).uuid;
    let walk = bbb_server_types::stats::custom_stat("minecraft:walk_one_cm");
    let sprint = bbb_server_types::stats::custom_stat("minecraft:sprint_one_cm");

    let start = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, start);
    let walked_before = world.player_stat_for_test(ConnectionId(1), walk);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: start.x + 1.0,
            ..start
        },
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), walk) - walked_before,
        100
    );
    // The chain awards one counter per move (`:1447-1456`).
    assert_eq!(world.player_stat_for_test(ConnectionId(1), sprint), 0);

    // `:1421` through `didNotMove` (`:1492-1494`): a move of nothing returns
    // before the chain, so the counter it would otherwise have created at zero
    // -- `AVIATE_ONE_CM` is the one branch with no floor under it (`:1460`) --
    // is not in the document at all.
    world.set_player_fall_flying_for_test(ConnectionId(1), true);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x + 1.0,
                ..start
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );

    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, walk),
        walked_before + 100
    );
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(paths.player_stats_file(uuid)).unwrap()).unwrap();
    assert!(
        document["stats"]["minecraft:custom"]
            .get("minecraft:aviate_one_cm")
            .is_none(),
        "{document}"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `ResultSlot.checkTakeAchievements` (`ResultSlot.java:50-53`) hands the take's
/// `removeCount` to `ItemStack.onCraftedBy` (`ItemStack.java:723-726`), which
/// awards `Stats.ITEM_CRAFTED` for the result's own item. `PlayerList.save`
/// writes that counter to `players/stats/<uuid>.json` beside the advancements
/// (`PlayerList.java:286-297`), so a craft has to survive to the file.
#[test]
fn a_crafted_item_counts_into_the_statistics_file() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID,
    };
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-crafted-stat-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let fixture = FlatFixture::new().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: "bbb-crafted-stat".to_string(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();

    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &candidate,
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "newcomer", 1);
    let uuid = profile("newcomer", 1).uuid;
    let crafted = bbb_server_types::stats::StatKey::new(
        bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
        VANILLA_ITEM_FLINT_AND_STEEL_ID,
    );

    for (slot_num, item_id) in [(1, VANILLA_ITEM_IRON_INGOT_ID), (2, VANILLA_ITEM_FLINT_ID)] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 5,
                    component_patch: Default::default(),
                },
            }),
        );
    }
    assert_eq!(
        world
            .player_crafting_result_for_test(ConnectionId(1))
            .item_id,
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );

    // One take of the result slot is one craft (`ResultSlot.remove`
    // `ResultSlot.java:30-36`), and the quick-move that follows crafts the rest
    // of the grid in one click (`AbstractContainerMenu.java:423-427`), so the
    // award is the take's whole `removeCount` rather than one per click.
    let mut take = |input| {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClick(ContainerClick {
                container_id: 0,
                state_id: 1,
                slot_num: 0,
                button_num: 0,
                input,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        );
    };
    take(ContainerInput::Pickup);
    take(ContainerInput::QuickMove);

    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    // `PlayerList.saveAll` reaches the counter through the same `save` the
    // advancements go through (`PlayerList.java:286-297`).
    assert_eq!(request.player_patches[0].stats.value(crafted), 5);
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let written = bbb_server_types::stats::ServerStatsCounter::parse_bytes(
        &std::fs::read(paths.player_stats_file(uuid)).expect("the save wrote the counter"),
    )
    .expect("the document this world writes is one it reads");
    assert_eq!(written.value(crafted), 5);
    std::fs::remove_dir_all(&root).unwrap();
}

/// `ServerPlayer.die` (`ServerPlayer.java:869-929`) counts on both sides of a
/// kill: the victim takes `ENTITY_KILLED_BY` for the killer's own entity type
/// (`:913-914`) and `DEATHS` (`:920`), and the killer takes `PLAYER_KILLS` out
/// of `awardKillScore` (`:915` into `:947`). All three have to reach
/// `players/stats/<uuid>.json`.
///
/// `ENTITY_KILLED` is deliberately not among them. It is awarded by
/// `Player.killedEntity` (`Player.java:1486-1489`), which is reached only from
/// `LivingEntity.die` (`LivingEntity.java:1446`); `ServerPlayer.die` overrides
/// that method whole and calls no `super`, so no player kill has ever moved the
/// killer's `entity_killed`.
#[test]
fn a_player_kill_counts_on_both_sides_of_the_statistics_files() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "player-kill",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..operator_config(&[("alice", 1)])
        },
        &[(1, "alice", 1), (2, "bob", 2), (3, "carol", 3)],
    );
    world.tick(receiver.drain());
    let killed_by_player = bbb_server_types::stats::StatKey::new(
        bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    );
    let deaths = bbb_server_types::stats::custom_stat("minecraft:deaths");
    let player_kills = bbb_server_types::stats::custom_stat("minecraft:player_kills");

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:generic by alice",
    );
    assert!(
        killed.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::PlayerCombatKill(_)
            }
        )),
        "{killed:?}"
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(2), killed_by_player),
        1
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(2), deaths), 1);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), player_kills), 1);
    // `:915` is on the killer, not the victim, and `awardKillScore` gives the
    // victim nothing but the advancement trigger.
    assert_eq!(world.player_stat_for_test(ConnectionId(2), player_kills), 0);
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), killed_by_player),
        0
    );

    // `:912`: with no player remembered there is no kill credit, so the whole
    // `:913-916` arm is skipped and only the death is counted.
    chat_command(&sender, &receiver, &mut world, 1, "damage carol 40");
    assert_eq!(world.player_stat_for_test(ConnectionId(3), deaths), 1);
    assert_eq!(
        world.player_stat_for_test(ConnectionId(3), killed_by_player),
        0
    );

    // `awardKillScore` is entirely behind `victim != this` (`:942`), so a
    // session that killed itself counts the credit and the death but never
    // credits itself a kill.
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 40 minecraft:generic by alice",
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), killed_by_player),
        1
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), deaths), 1);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), player_kills), 1);

    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let saved = |uuid| {
        bbb_server_types::stats::ServerStatsCounter::parse_bytes(
            &std::fs::read(paths.player_stats_file(uuid)).expect("the save wrote the counter"),
        )
        .expect("the document this world writes is one it reads")
    };
    let victim = saved(profile("bob", 2).uuid);
    assert_eq!(victim.value(killed_by_player), 1);
    assert_eq!(victim.value(deaths), 1);
    let killer = saved(profile("alice", 1).uuid);
    assert_eq!(killer.value(player_kills), 1);
    assert_eq!(killer.value(killed_by_player), 1);
    assert_eq!(killer.value(deaths), 1);
    let unattributed: serde_json::Value = serde_json::from_slice(
        &std::fs::read(paths.player_stats_file(profile("carol", 3).uuid)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        unattributed["stats"]["minecraft:custom"]["minecraft:deaths"],
        1
    );
    assert!(
        unattributed["stats"].get("minecraft:killed_by").is_none(),
        "{unattributed}"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// A storage-backed flat world whose one session is riding a Boat, for the two
/// `checkRidingStatistics` counters below.
fn boat_world(
    name: &str,
) -> (
    std::path::PathBuf,
    WorldSavePaths,
    Java26WorldStorage,
    Java26WorldCandidate,
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
) {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world(name);
    let standing = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, standing);
    let boat = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            standing,
            crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
        )
        .0;
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    let mounted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("mounting teleports the rider onto the Boat");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    (
        root, paths, storage, candidate, sender, receiver, world, boat,
    )
}

/// `minecraft:boat_one_cm` as `players/stats/<uuid>.json` holds it after one
/// save, distinguishing a counter written at zero from one that is not there.
fn saved_boat_centimetres(
    storage: &Java26WorldStorage,
    candidate: &Java26WorldCandidate,
    paths: &WorldSavePaths,
    world: &mut ServerWorld,
    uuid: uuid::Uuid,
) -> Option<i64> {
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(candidate, &request).unwrap();
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(paths.player_stats_file(uuid)).unwrap()).unwrap();
    document["stats"]["minecraft:custom"]
        .get("minecraft:boat_one_cm")
        .map(|value| value.as_i64().expect("a counter is an integer"))
}

/// `ServerPlayer.checkRidingStatistics` (`ServerPlayer.java:1470-1490`) has one
/// guard and it is `didNotMove` (`:1471`, `:1492-1494`), not a positive
/// distance: a rider that moved by less than half a centimetre still awards,
/// and the counter is written at zero. `StatsCounter.increment` routes through
/// `setValue` (`StatsCounter.java:15-22`), which puts the key in the map, and
/// `toJson` writes the map entire (`ServerStatsCounter.java:115-119`), so the
/// zero reaches the file.
///
/// This is the tick half, `rideTick` (`:1411-1418`): the Boat is the one that
/// moved and the rider was carried.
#[test]
fn a_carried_rider_that_moves_too_little_to_round_up_still_writes_the_counter() {
    let (root, paths, storage, candidate, _sender, receiver, mut world, boat) =
        boat_world("boat-carried");
    let uuid = profile("newcomer", 1).uuid;

    // A Boat at rest on the ground carries the rider nowhere, which is the
    // `didNotMove` half of the same guard: no award at all, so no key.
    world.tick(receiver.drain());
    assert_eq!(
        saved_boat_centimetres(&storage, &candidate, &paths, &mut world, uuid),
        None
    );

    let resting = world
        .boat_control_state_for_test(Java26BuiltinDimension::Overworld, boat)
        .0;
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, boat)
        .expect("the Boat is live")
        // A millimetre is a tenth of a centimetre, which rounds to nothing.
        .set_pos(Vec3d {
            x: resting.x + 0.001,
            ..resting
        });
    world.tick(receiver.drain());
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:boat_one_cm")
        ),
        0
    );
    assert_eq!(
        saved_boat_centimetres(&storage, &candidate, &paths, &mut world, uuid),
        Some(0)
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// The same guard on the packet half: `handleMoveVehicle`
/// (`ServerGamePacketListenerImpl.java:439-515`) is where a driven Boat's own
/// displacement arrives, and vanilla accounts for it through the same
/// `checkRidingStatistics` (`ServerPlayer.java:1470-1490`) on the tick that
/// follows.
#[test]
fn a_driven_boat_move_too_small_to_round_up_still_writes_the_counter() {
    let (root, paths, storage, candidate, sender, receiver, mut world, _boat) =
        boat_world("boat-driven");
    let uuid = profile("newcomer", 1).uuid;

    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 0.001,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:boat_one_cm")
        ),
        0
    );
    assert_eq!(
        saved_boat_centimetres(&storage, &candidate, &paths, &mut world, uuid),
        Some(0)
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// One block state by name and properties, for the openers below.
fn opener_block_state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties,
    )
    .unwrap_or_else(|| panic!("{name} is a vanilla 26.1 block"))
}

/// The counters `players/stats/<uuid>.json` holds under `minecraft:custom`
/// after one save.
fn saved_custom_stats(
    storage: &Java26WorldStorage,
    candidate: &Java26WorldCandidate,
    paths: &WorldSavePaths,
    world: &mut ServerWorld,
    uuid: uuid::Uuid,
) -> serde_json::Value {
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(candidate, &request).unwrap();
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(paths.player_stats_file(uuid)).unwrap()).unwrap();
    document["stats"]["minecraft:custom"].clone()
}

/// Every `useWithoutItem` that opens a container counts one custom statistic
/// beside `openMenu`: `ChestBlock.java:262` through `getOpenChestStat`
/// (`:270-272`) and its trapped override (`TrappedChestBlock.java:37-39`),
/// `BarrelBlock.java:48`, `ShulkerBoxBlock.java:81`, and `DispenserBlock.java:80`,
/// which reads the dropper subclass off the block entity rather than taking a
/// second arm.
#[test]
fn container_openers_count_into_the_statistics_file() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world("openers");
    let uuid = profile("newcomer", 1).uuid;
    let standing = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, standing);

    let openers = [
        (
            BlockPos { x: 0, y: 64, z: 1 },
            opener_block_state(
                "minecraft:chest",
                &[
                    ("facing", "north"),
                    ("type", "single"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:open_chest",
        ),
        (
            BlockPos { x: 0, y: 64, z: 2 },
            opener_block_state(
                "minecraft:trapped_chest",
                &[
                    ("facing", "north"),
                    ("type", "single"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:trigger_trapped_chest",
        ),
        (
            BlockPos { x: 2, y: 64, z: 1 },
            opener_block_state(
                "minecraft:barrel",
                &[("facing", "north"), ("open", "false")],
            ),
            "minecraft:open_barrel",
        ),
        (
            BlockPos { x: 2, y: 64, z: 2 },
            opener_block_state("minecraft:shulker_box", &[("facing", "up")]),
            "minecraft:open_shulker_box",
        ),
        (
            BlockPos { x: 1, y: 64, z: 0 },
            opener_block_state(
                "minecraft:dispenser",
                &[("facing", "north"), ("triggered", "false")],
            ),
            "minecraft:inspect_dispenser",
        ),
        (
            BlockPos { x: 2, y: 64, z: 0 },
            opener_block_state(
                "minecraft:dropper",
                &[("facing", "north"), ("triggered", "false")],
            ),
            "minecraft:inspect_dropper",
        ),
    ];
    for (sequence, (at, block_state_id, stat)) in openers.iter().enumerate() {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            *at,
            *block_state_id,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
        let clicked = right_click(
            &sender,
            &receiver,
            &mut world,
            1,
            *at,
            i32::try_from(sequence).unwrap() + 1,
        );
        assert!(
            clicked.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::OpenScreen(_),
                    ..
                }
            )),
            "{stat} needs a menu to have opened: {clicked:?}"
        );
        assert_eq!(
            world.player_stat_for_test(ConnectionId(1), bbb_server_types::stats::custom_stat(stat)),
            1,
            "{stat}"
        );
    }

    let saved = saved_custom_stats(&storage, &candidate, &paths, &mut world, uuid);
    for (_, _, stat) in openers {
        assert_eq!(saved[stat], 1, "{stat}: {saved}");
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// A spectator reaches the same chest menu through `state.getMenuProvider` in
/// `ServerPlayerGameMode.useItemOn` (`ServerPlayerGameMode.java:358-365`),
/// which runs no `useWithoutItem` and so counts nothing. The counter therefore
/// belongs beside `openMenu` in the block, not inside the menu opener both
/// arms share.
#[test]
fn a_spectator_opens_the_chest_without_counting_it() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "spectator-opener",
        ServerWorldConfig {
            game_type: GameType::Spectator,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    let uuid = profile("newcomer", 1).uuid;
    let standing = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, standing);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        opener_block_state(
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let clicked = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(
        clicked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::OpenScreen(_),
                ..
            }
        )),
        "a spectator still reaches the menu provider: {clicked:?}"
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:open_chest")
        ),
        0
    );
    let saved = saved_custom_stats(&storage, &candidate, &paths, &mut world, uuid);
    assert!(saved.get("minecraft:open_chest").is_none(), "{saved}");
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// Three more counters that sit beside the action rather than beside the menu:
/// `CakeBlock.eat` counts before it feeds (`CakeBlock.java:99-100`),
/// `BellBlock.onHit` counts the ring and not the proper hit
/// (`BellBlock.java:104-107`), and `LecternBlock.openScreen` counts the screen
/// it opened (`LecternBlock.java:254-259`).
#[test]
fn the_cake_bell_and_lectern_counters_reach_the_statistics_file() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world("interacts");
    let uuid = profile("newcomer", 1).uuid;
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        },
    );

    let cake = BlockPos { x: 0, y: 64, z: 0 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        cake,
        opener_block_state("minecraft:cake", &[("bites", "0")]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    right_click(&sender, &receiver, &mut world, 1, cake, 1);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:eat_cake_slice")
        ),
        1
    );

    let bell = BlockPos { x: 2, y: 64, z: 3 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        bell,
        opener_block_state(
            "minecraft:bell",
            &[
                ("facing", "north"),
                ("attachment", "floor"),
                ("powered", "false"),
            ],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    // `BellBlock.isProperHit` (`BellBlock.java:115-129`): a floor bell answers
    // only to a horizontal face, so `right_click`'s `Direction::Up` rings
    // nothing and `onHit` never reaches its `awardStat`.
    right_click(&sender, &receiver, &mut world, 1, bell, 2);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:bell_ring")
        ),
        0
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: bell,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence: 3,
        }),
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:bell_ring")
        ),
        1
    );

    let lectern = BlockPos { x: 0, y: 64, z: 3 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lectern,
        opener_block_state(
            "minecraft:lectern",
            &[
                ("facing", "north"),
                ("has_book", "false"),
                ("powered", "false"),
            ],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                writable_book_pages: vec!["one".into()],
                ..Default::default()
            }
            .into(),
        },
    );
    // `LecternBlock.useItemOn` places the book; only the empty-hand click that
    // follows runs `openScreen`.
    right_click(&sender, &receiver, &mut world, 1, lectern, 4);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_lectern")
        ),
        0,
        "putting the book down opens no screen"
    );
    right_click(&sender, &receiver, &mut world, 1, lectern, 5);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_lectern")
        ),
        1
    );

    let saved = saved_custom_stats(&storage, &candidate, &paths, &mut world, uuid);
    for stat in [
        "minecraft:eat_cake_slice",
        "minecraft:bell_ring",
        "minecraft:interact_with_lectern",
    ] {
        assert_eq!(saved[stat], 1, "{stat}: {saved}");
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// Sends one `MovePlayer` at an absolute position and returns nothing but the
/// tick, for the movement counters below.
fn move_to(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    position: Vec3d,
    on_ground: bool,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(position),
            rotation: None,
            on_ground,
            horizontal_collision: false,
        }),
    );
}

/// `ServerPlayer.checkMovementStatistics`'s climbable arm (`:1440-1443`)
/// through `LivingEntity.onClimbable` (`LivingEntity.java:1688-1706`): it sits
/// above the on-ground arm, so a session on a ladder counts a rise as
/// `climb_one_cm` and counts nothing at all for the horizontal part or for
/// going down.
#[test]
fn climbing_a_ladder_counts_into_the_statistics_file() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world("climb");
    let uuid = profile("newcomer", 1).uuid;
    let climb = bbb_server_types::stats::custom_stat("minecraft:climb_one_cm");
    let walk = bbb_server_types::stats::custom_stat("minecraft:walk_one_cm");

    // `LadderBlock.canSurvive` (`LadderBlock.java:52-55`) needs the face behind
    // the ladder, which is the block opposite its facing.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 0, y: 64, z: 1 },
        opener_block_state("minecraft:stone", &[]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let ladder = BlockPos { x: 1, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        ladder,
        opener_block_state(
            "minecraft:ladder",
            &[("facing", "east"), ("waterlogged", "false")],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(crate::block_behavior::ladder::is_ladder(
        world
            .block_state(Java26BuiltinDimension::Overworld, ladder)
            .expect("the ladder column is loaded")
    ));

    let foot = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, foot);
    let walked_before = world.player_stat_for_test(ConnectionId(1), walk);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 64.6, ..foot },
        true,
    );
    // `:1442` rounds `dy * 100.0` as a double, and the arm outranks the
    // on-ground one the same tick would otherwise have taken.
    assert_eq!(world.player_stat_for_test(ConnectionId(1), climb), 60);
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), walk),
        walked_before
    );

    // `:1441`: only a rise counts, and the arm still swallows the tick.
    move_to(&sender, &receiver, &mut world, foot, true);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), climb), 60);
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), walk),
        walked_before
    );

    // One block east is air, so the same shape of move reaches the on-ground
    // arm instead.
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { x: 2.5, ..foot },
        true,
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), climb), 60);
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), walk) - walked_before,
        100
    );

    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, climb),
        60
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `LivingEntity.trapdoorUsableAsLadder` (`LivingEntity.java:1708-1715`): an
/// open trapdoor over a ladder facing the same way is climbable, and the same
/// trapdoor shut is not.
#[test]
fn an_open_trapdoor_over_a_ladder_counts_as_a_climb() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world("trapdoor");
    let uuid = profile("newcomer", 1).uuid;
    let climb = bbb_server_types::stats::custom_stat("minecraft:climb_one_cm");
    let trapdoor = BlockPos { x: 1, y: 65, z: 2 };
    let trapdoor_state = |open: &str, facing: &str| {
        opener_block_state(
            "minecraft:oak_trapdoor",
            &[
                ("facing", facing),
                ("half", "bottom"),
                ("open", open),
                ("powered", "false"),
                ("waterlogged", "false"),
            ],
        )
    };
    for (at, state) in [
        (
            BlockPos { x: 0, y: 64, z: 2 },
            opener_block_state("minecraft:stone", &[]),
        ),
        (
            BlockPos { x: 1, y: 64, z: 2 },
            opener_block_state(
                "minecraft:ladder",
                &[("facing", "east"), ("waterlogged", "false")],
            ),
        ),
        (trapdoor, trapdoor_state("true", "east")),
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            at,
            state,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }

    let foot = Vec3d {
        x: 1.5,
        y: 65.0,
        z: 2.5,
    };
    // Standing up onto the trapdoor is itself a rise inside it, so the arm is
    // measured from where that left the counter.
    stand_at(&sender, &receiver, &mut world, 1, foot);
    let climbed_before = world.player_stat_for_test(ConnectionId(1), climb);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 65.25, ..foot },
        true,
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), climb) - climbed_before,
        25
    );

    // `:1709-1711`: a shut trapdoor is no ladder, so the rise falls through to
    // the on-ground arm, which counts no vertical distance at all.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        trapdoor,
        trapdoor_state("false", "east"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    stand_at(&sender, &receiver, &mut world, 1, foot);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 65.25, ..foot },
        true,
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), climb) - climbed_before,
        25
    );

    // `:1714`: the facings have to agree, and the ladder below faces east.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        trapdoor,
        trapdoor_state("true", "west"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    stand_at(&sender, &receiver, &mut world, 1, foot);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 65.25, ..foot },
        true,
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), climb) - climbed_before,
        25
    );

    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, climb),
        climbed_before + 25
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `ServerGamePacketListenerImpl.handleMovePlayer` (`:1115-1118`) reads a jump
/// off the packet -- the session was on the ground, says it is not any more,
/// and rose -- and `ServerPlayer.jumpFromGround` (`ServerPlayer.java:1532-1540`)
/// counts it.
#[test]
fn a_jump_read_off_the_movement_packet_counts_into_the_statistics_file() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world("jump");
    let uuid = profile("newcomer", 1).uuid;
    let jump = bbb_server_types::stats::custom_stat("minecraft:jump");
    let ground = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, ground);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), jump), 0);

    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 64.42, ..ground },
        false,
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), jump), 1);

    // `:1116` reads the server's own `onGround`, which the packet above just
    // cleared, so the rest of the arc is not a second jump.
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 64.75, ..ground },
        false,
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), jump), 1);

    // `:1115`: leaving the ground without rising is not a jump either.
    stand_at(&sender, &receiver, &mut world, 1, ground);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { x: 2.5, ..ground },
        false,
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), jump), 1);

    let launch = Vec3d { x: 2.5, ..ground };
    stand_at(&sender, &receiver, &mut world, 1, launch);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 64.42, ..launch },
        false,
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), jump), 2);

    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, jump),
        2
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// The jump test (`:1112-1117`) reads the `onGround` the server-side
/// `travel` of every `ServerPlayer.doTick` leaves behind
/// (`LivingEntity.java:3061-3065`), not only the last packet's flag.
#[test]
fn a_jump_reads_the_ground_the_tick_found_under_the_session() {
    let (root, _paths, storage, _candidate, sender, receiver, mut world) =
        stats_world("ground-tick");
    let jump = bbb_server_types::stats::custom_stat("minecraft:jump");
    let ground = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, ground);
    // Standing on the ground while claiming to be airborne: the tick puts the
    // body back on the ground, so the rise that follows is a jump.
    move_to(&sender, &receiver, &mut world, ground, false);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 64.42, ..ground },
        false,
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), jump), 1);

    // Claiming the ground over a hole: the tick takes it away again.
    move_to(&sender, &receiver, &mut world, ground, true);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 1, y: 63, z: 1 },
        block_state_id("minecraft:air"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { y: 64.42, ..ground },
        false,
    );
    assert_eq!(world.player_stat_for_test(ConnectionId(1), jump), 1);
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `handleMovePlayer` runs `checkMovementStatistics` only on the arm that
/// accepted a move (`:1076`, `:1175`); the teleport arm above it snaps the
/// rotation and returns (`:1067-1068`). A packet that arrives mid-teleport must
/// therefore not count the delta the last accepted move left behind.
#[test]
fn a_move_refused_mid_teleport_counts_nothing_twice() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "refused-move",
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            ..operator_config(&[("newcomer", 1)])
        },
        &[(1, "newcomer", 1)],
    );
    let uuid = profile("newcomer", 1).uuid;
    let walk = bbb_server_types::stats::custom_stat("minecraft:walk_one_cm");
    let start = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    stand_at(&sender, &receiver, &mut world, 1, start);
    let walked_before = world.player_stat_for_test(ConnectionId(1), walk);
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { x: 2.5, ..start },
        true,
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), walk) - walked_before,
        100
    );

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "tp newcomer 1.5 64.0 1.5",
    );
    move_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d { x: 3.5, ..start },
        true,
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), walk) - walked_before,
        100,
        "the refused packet must not re-count the move before it"
    );

    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, walk),
        walked_before + 100
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `Player.actuallyHurt`'s two counters (`Player.java:756-758`, `:764-766`).
///
/// Both are tenths of a heart and both are guarded on the value alone, which is
/// what lets them be awarded from `HurtOutcome`'s two amounts rather than from
/// the health delta: `setHealth` clamps at zero (`LivingEntity.java:1146-1148`),
/// so a blow larger than
/// the health left counts what it dealt and not what it moved.
#[test]
fn the_two_damage_counters_reach_the_statistics_file() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "damage-counters",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    let _ = (&sender, &receiver);
    let uuid = profile("newcomer", 1).uuid;
    let taken = bbb_server_types::stats::custom_stat("minecraft:damage_taken");
    let absorbed = bbb_server_types::stats::custom_stat("minecraft:damage_absorbed");
    let mut events = Vec::new();
    // `LivingEntity.hurtServer` (`:1196-1203`) only lets a second hit through
    // on the part above the last one until the invulnerability window closes.
    let cool_down = |world: &mut ServerWorld| {
        for _ in 0..20 {
            world.tick(receiver.drain());
        }
    };

    world.hurt_connected_player_for_test(ConnectionId(1), 6.0, &mut events);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), taken), 60);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), absorbed), 0);

    // A zero request is refused before `actuallyHurt` runs at all
    // (`Player.java:708`), so neither counter moves; the `dmg != 0.0F` arm
    // itself (`:760`) is what the fully-absorbed case below pins.
    cool_down(&mut world);
    world.hurt_connected_player_for_test(ConnectionId(1), 0.0, &mut events);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), taken), 60);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), absorbed), 0);

    // `:753-758`: absorption takes the whole hit, so the health never moves and
    // only `DAMAGE_ABSORBED` counts.
    cool_down(&mut world);
    world.add_player_mob_effect(
        ConnectionId(1),
        crate::entity::living::mob_effects::ActiveMobEffect::new("minecraft:absorption", 400, 0)
            .expect("a vanilla effect id"),
        &mut events,
    );
    assert_eq!(world.player_absorption_for_test(ConnectionId(1)), 4.0);
    let health = world.player_health_for_test(ConnectionId(1));
    world.hurt_connected_player_for_test(ConnectionId(1), 3.0, &mut events);
    assert_eq!(world.player_health_for_test(ConnectionId(1)), health);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), absorbed), 30);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), taken), 60);

    // The remaining heart of absorption is spent first and the rest lands, so
    // one hit moves both counters.
    cool_down(&mut world);
    assert_eq!(world.player_absorption_for_test(ConnectionId(1)), 1.0);
    world.hurt_connected_player_for_test(ConnectionId(1), 6.0, &mut events);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), absorbed), 40);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), taken), 110);

    // `:765` is `Math.round(dmg * 10.0F)`, not a truncation: half a heart of
    // damage is five and a half tenths and counts six.
    cool_down(&mut world);
    world.hurt_connected_player_for_test(ConnectionId(1), 0.55, &mut events);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), taken), 116);

    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, taken),
        116
    );
    assert_eq!(
        saved_stat_value(&storage, &candidate, &paths, &mut world, uuid, absorbed),
        40
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// Both guards in `Player.actuallyHurt` (`Player.java:756`, `:760`, `:764`) are
/// exclusions rather than clamps, and `StatsCounter.increment` routes through
/// the `setValue` that puts the key (`StatsCounter.java:15-22`), so awarding a
/// refused hit as zero would be
/// visible in the file: neither key may appear at all.
#[test]
fn a_hit_outside_the_damage_guards_writes_neither_counter() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "damage-counter-guards",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    let _ = (&sender, &receiver);
    let uuid = profile("newcomer", 1).uuid;
    let mut events = Vec::new();

    // `Player.java:708` refuses this one before `actuallyHurt`.
    world.hurt_connected_player_for_test(ConnectionId(1), 0.0, &mut events);
    // `LivingEntity.java:1191-1193` clamps a NaN or infinite request to
    // `Float.MAX_VALUE`, which is what the upper guard is there to refuse: a
    // hit that large counts nothing rather than saturating the counter.
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    world.hurt_connected_player_for_test(ConnectionId(1), f32::MAX, &mut events);

    let stats = saved_custom_stats(&storage, &candidate, &paths, &mut world, uuid);
    assert_eq!(stats.get("minecraft:damage_taken"), None);
    assert_eq!(stats.get("minecraft:damage_absorbed"), None);
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// The `dmg != 0.0F` arm (`Player.java:760`) is what keeps `DAMAGE_TAKEN` away
/// from a hit absorption swallowed whole: the health never moves, so the file
/// must carry the absorbed tenths and no `damage_taken` key at all.
#[test]
fn a_fully_absorbed_hit_counts_only_the_absorbed_counter() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "damage-fully-absorbed",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    let _ = (&sender, &receiver);
    let uuid = profile("newcomer", 1).uuid;
    let mut events = Vec::new();

    world.add_player_mob_effect(
        ConnectionId(1),
        crate::entity::living::mob_effects::ActiveMobEffect::new("minecraft:absorption", 400, 0)
            .expect("a vanilla effect id"),
        &mut events,
    );
    let health = world.player_health_for_test(ConnectionId(1));
    world.hurt_connected_player_for_test(ConnectionId(1), 2.5, &mut events);
    assert_eq!(world.player_health_for_test(ConnectionId(1)), health);

    let stats = saved_custom_stats(&storage, &candidate, &paths, &mut world, uuid);
    assert_eq!(stats["minecraft:damage_absorbed"], serde_json::json!(25));
    assert_eq!(stats.get("minecraft:damage_taken"), None);
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// A stack of the item every campfire recipe in the vanilla snapshot cooks the
/// longest: `cooked_porkchop_from_campfire_cooking.json` asks for 600 ticks,
/// which is the value `placeFood` copies rather than the 100-tick default.
fn campfire_food(count: i32) -> bbb_protocol::packets::ItemStackSummary {
    bbb_protocol::packets::ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_PORKCHOP_ID),
        count,
        component_patch: Default::default(),
    }
}

fn campfire_state(lit: bool) -> i32 {
    opener_block_state(
        "minecraft:campfire",
        &[
            ("facing", "north"),
            ("lit", if lit { "true" } else { "false" }),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    )
}

fn lit_campfire() -> i32 {
    campfire_state(true)
}

fn stick(count: i32) -> bbb_protocol::packets::ItemStackSummary {
    bbb_protocol::packets::ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STICK_ID),
        count,
        component_patch: Default::default(),
    }
}

fn campfire_after_click(
    world: &mut ServerWorld,
    at: BlockPos,
) -> crate::block_entity::campfire::CampfireBlockEntity {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .expect("set_block created the campfire block entity")
        .campfire()
        .expect("the block entity is a campfire")
        .clone()
}

/// `CampfireBlock.useItemOn` (`CampfireBlock.java:99-104`) puts one item in the
/// first free slot with the recipe's own cooking time
/// (`CampfireBlockEntity.java:171-179`) and counts the interaction.
#[test]
fn a_campfire_takes_one_food_item_and_counts_the_interaction() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "campfire-food",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    let uuid = profile("newcomer", 1).uuid;
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lit_campfire(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, campfire_food(4));

    right_click(&sender, &receiver, &mut world, 1, at, 1);

    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_campfire")
        ),
        1
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
            && event.pos == at
            && event.source_entity_id == Some(entity_id_of(&world, 1))
    }));
    let campfire = campfire_after_click(&mut world, at);
    assert_eq!(campfire.items()[0].summary().clone(), campfire_food(1));
    assert_eq!(campfire.cooking_time()[0], 600);
    // `CampfireBlock.getTicker` (`CampfireBlock.java:308-321`) runs in the same
    // tick that placed the food, so the progress has already advanced once.
    assert_eq!(campfire.cooking_progress()[0], 1);
    assert!(&campfire.items()[1].is_empty());

    let saved = saved_custom_stats(&storage, &candidate, &paths, &mut world, uuid);
    assert_eq!(
        saved["minecraft:interact_with_campfire"],
        serde_json::json!(1)
    );
    // `:179`: the hand paid for the one item that went in.
    let request = world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.player_patches[0].created.inventory.runtime_slots()[0].count(),
        3
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `placeFood` offers the item to the first free slot only and answers false
/// with none left (`CampfireBlockEntity.java:168-170`, `:186`), which is
/// `CONSUME` without the counter (`CampfireBlock.java:102-107`). An item no
/// campfire recipe accepts never reaches either (`:101`).
#[test]
fn a_full_campfire_and_an_uncookable_item_both_count_nothing() {
    let (root, paths, storage, candidate, sender, receiver, mut world) = stats_world_with(
        "campfire-refusals",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    let uuid = profile("newcomer", 1).uuid;
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lit_campfire(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stick(1));
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    let empty = campfire_after_click(&mut world, at);
    assert!(empty
        .items()
        .iter()
        .all(bbb_server_types::item_stack::RuntimeItemStack::is_empty));
    let request = world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.player_patches[0].created.inventory.runtime_slots()[0].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_STICK_ID)
    );
    assert_eq!(
        request.player_patches[0].created.inventory.runtime_slots()[0].count(),
        1
    );

    world.set_inventory_slot_for_test(ConnectionId(1), 0, campfire_food(6));

    for sequence in 2..=6 {
        right_click(&sender, &receiver, &mut world, 1, at, sequence);
    }
    // Four slots took one item each; the fifth click had nowhere to put one.
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_campfire")
        ),
        4
    );
    let campfire = campfire_after_click(&mut world, at);
    for slot in 0..4 {
        assert_eq!(
            campfire.items()[slot].summary().clone(),
            campfire_food(1),
            "slot {slot}"
        );
    }
    // Two of the six items stayed in the hand: the fifth click paid nothing.
    let request = world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.player_patches[0].created.inventory.runtime_slots()[0].count(),
        2
    );

    world.set_inventory_slot_for_test(ConnectionId(1), 0, stick(1));
    right_click(&sender, &receiver, &mut world, 1, at, 7);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_campfire")
        ),
        4
    );
    let campfire = campfire_after_click(&mut world, at);
    for slot in 0..4 {
        assert_eq!(
            campfire.items()[slot].summary().clone(),
            campfire_food(1),
            "slot {slot}"
        );
    }

    let saved = saved_custom_stats(&storage, &candidate, &paths, &mut world, uuid);
    assert_eq!(
        saved["minecraft:interact_with_campfire"],
        serde_json::json!(4)
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `useItemOn` (`CampfireBlock.java:99-107`) never reads `LIT`; that property
/// only selects `cookTick` versus `cooldownTick` (`:310-317`).
#[test]
fn an_unlit_campfire_still_takes_food() {
    let (root, _paths, storage, _candidate, sender, receiver, mut world) = stats_world_with(
        "campfire-unlit",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        campfire_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, campfire_food(2));

    right_click(&sender, &receiver, &mut world, 1, at, 1);

    let campfire = campfire_after_click(&mut world, at);
    assert_eq!(campfire.items()[0].summary().clone(), campfire_food(1));
    assert_eq!(campfire.cooking_time()[0], 600);
    let request = world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.player_patches[0].created.inventory.runtime_slots()[0].count(),
        1
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `ItemStack.consume` (`ItemStack.java:1082-1085`) leaves the hand when the
/// source has infinite materials; the slot still receives the split stack
/// (`:1088-1091`).
#[test]
fn a_creative_session_fills_a_campfire_slot_without_taking_from_the_hand() {
    let (root, _paths, storage, _candidate, sender, receiver, mut world) = stats_world_with(
        "campfire-creative",
        ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &[(1, "newcomer", 1)],
    );
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lit_campfire(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, campfire_food(4));
    let mut abilities = world.player_abilities_for_test(ConnectionId(1));
    abilities.instabuild = true;
    world.set_player_mode_and_abilities_for_test(ConnectionId(1), GameType::Creative, abilities);

    right_click(&sender, &receiver, &mut world, 1, at, 1);

    let campfire = campfire_after_click(&mut world, at);
    assert_eq!(campfire.items()[0].summary().clone(), campfire_food(1));
    assert_eq!(campfire.cooking_time()[0], 600);
    let request = world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.player_patches[0].created.inventory.runtime_slots()[0].count(),
        4
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}
