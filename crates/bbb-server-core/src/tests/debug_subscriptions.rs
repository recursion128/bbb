use std::sync::Arc;

use bbb_protocol::debug_subscription_types::{
    VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID, VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID, VANILLA_DEBUG_SUBSCRIPTION_POIS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
};
use bbb_protocol::packets::{
    DebugBlockValue, DebugChunkValue, DebugEntityValue, DebugSubscriptionValue,
};

use super::*;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn debug_values(tick: &TickResult) -> Vec<(ConnectionId, Arc<DebugEntityValue>)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DebugEntityValue(update),
            } => Some((*connection_id, Arc::clone(update))),
            _ => None,
        })
        .collect()
}

fn spawn_mob(world: &mut ServerWorld, entity_type_id: i32, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            DIMENSION,
            entity_type_id,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(entity_type_id).unwrap(),
            ),
        )
        .0
}

fn spawn_idle_mob(world: &mut ServerWorld, entity_type_id: i32, position: Vec3d) -> i32 {
    let mut data = crate::entity::GeneralLivingData::new(entity_type_id).unwrap();
    data.set_no_ai(true);
    world
        .spawn_stored_entity_for_test(
            DIMENSION,
            entity_type_id,
            position,
            crate::entity::EntityData::GeneralLiving(data),
        )
        .0
}

fn request(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    subscription_ids: Vec<i32>,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::DebugSubscriptionRequest { subscription_ids },
    )
}

#[test]
fn operator_goal_publication_is_delayed_deduplicated_stable_and_exact() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let pig = spawn_idle_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        },
    );

    let non_operator = request(
        &sender,
        &receiver,
        &mut world,
        2,
        vec![VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID],
    );
    assert!(debug_values(&non_operator).is_empty(), "{non_operator:?}");
    assert!(non_operator.events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(1)),
        Some(ClientboundPlayPacket::AddEntity(add)) if add.id == pig
    )));

    let requested = request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![
            VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID,
            VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID,
        ],
    );
    assert!(debug_values(&requested).is_empty(), "{requested:?}");

    let initial = world.tick(receiver.drain());
    let values = debug_values(&initial);
    assert_eq!(values.len(), 1, "{values:?}");
    assert_eq!(values[0].0, ConnectionId(1));
    assert_eq!(values[0].1.entity_id, pig);
    assert_eq!(
        values[0].1.update.subscription_id,
        VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID
    );
    assert!(matches!(
        values[0].1.update.value,
        Some(DebugSubscriptionValue::Goals(ref goals))
            if goals.goals.iter().any(|goal| goal.name == "RandomStrollGoal")
                && goals.goals.iter().all(|goal| !goal.is_running)
    ));

    let stable = world.tick(receiver.drain());
    assert!(debug_values(&stable).is_empty(), "{stable:?}");
    let cancelled = request(&sender, &receiver, &mut world, 1, Vec::new());
    assert!(debug_values(&cancelled).is_empty(), "{cancelled:?}");
    let asleep = world.tick(receiver.drain());
    assert!(debug_values(&asleep).is_empty(), "{asleep:?}");
}

#[test]
fn a_new_subscriber_gets_the_cached_value_only_after_tracking() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1), ("bob", 2)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let pig = spawn_idle_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        },
    );
    request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID],
    );
    let initial = world.tick(receiver.drain());
    assert_eq!(debug_values(&initial).len(), 1, "{initial:?}");

    let subscribed = request(
        &sender,
        &receiver,
        &mut world,
        2,
        vec![VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID],
    );
    let values = debug_values(&subscribed);
    assert_eq!(values.len(), 1, "{values:?}");
    assert_eq!(values[0].0, ConnectionId(2));
    assert_eq!(values[0].1.entity_id, pig);
}

#[test]
fn an_untracked_mob_is_not_cached_and_first_tracking_sends_only_current_path() {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.game_type = GameType::Survival;
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let witch = spawn_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 3.5,
        },
    );
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID],
    );
    let awakened = world.tick(receiver.drain());
    assert!(debug_values(&awakened).is_empty(), "{awakened:?}");
    assert!(world.move_witch_ground_to_for_test(
        DIMENSION,
        witch,
        BlockPos { x: 6, y: 64, z: 3 },
        0,
        0.0,
    ));
    let cached = world.tick(receiver.drain());
    assert_eq!(debug_values(&cached).len(), 1, "{cached:?}");
    assert!(world.move_witch_ground_to_for_test(
        DIMENSION,
        witch,
        BlockPos { x: 9, y: 64, z: 3 },
        0,
        0.0,
    ));
    world.move_stored_entity_for_test(
        DIMENSION,
        witch,
        Vec3d {
            x: 1000.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let untracked = world.tick(receiver.drain());
    assert!(debug_values(&untracked).is_empty(), "{untracked:?}");
    assert!(world
        .player_tracking_viewers_for_test(DIMENSION, witch)
        .is_empty());
    world.move_stored_entity_for_test(
        DIMENSION,
        witch,
        Vec3d {
            x: 5.5,
            y: 64.0,
            z: 0.5,
        },
    );
    assert!(world
        .player_tracking_viewers_for_test(DIMENSION, witch)
        .is_empty());
    assert!(world.move_witch_ground_to_for_test(
        DIMENSION,
        witch,
        BlockPos { x: 12, y: 64, z: 0 },
        0,
        0.0,
    ));

    let tracked = world.tick(receiver.drain());
    let add_index = tracked
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::AddEntity(add),
                } if add.id == witch
            )
        })
        .expect("new tracking sends AddEntity");
    let debug_index = tracked
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::DebugEntityValue(update),
                } if update.entity_id == witch
            )
        })
        .expect("new tracking sends the current debug value");
    assert!(add_index < debug_index, "{tracked:?}");
    let values = debug_values(&tracked);
    assert_eq!(values.len(), 1, "no stale path replay: {values:?}");
    assert!(matches!(
        values[0].1.update.value,
        Some(DebugSubscriptionValue::Path(_))
    ));
}

#[test]
fn path_capture_starts_after_level_wake_and_disappearance_sends_typed_clear() {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.game_type = GameType::Survival;
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let witch = spawn_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 3.5,
        },
    );
    world.tick(receiver.drain());
    world.tick(receiver.drain());

    let cold = request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID],
    );
    assert!(debug_values(&cold).is_empty(), "{cold:?}");
    assert!(world.move_witch_ground_to_for_test(
        DIMENSION,
        witch,
        BlockPos { x: 5, y: 64, z: 3 },
        0,
        0.0,
    ));
    let wake = world.tick(receiver.drain());
    assert!(debug_values(&wake).is_empty(), "{wake:?}");

    assert!(world.move_witch_ground_to_for_test(
        DIMENSION,
        witch,
        BlockPos { x: 8, y: 64, z: 3 },
        0,
        1.0,
    ));
    let published = world.tick(receiver.drain());
    let values = debug_values(&published);
    assert!(
        values.iter().any(|(_, value)| matches!(
            value.update.value,
            Some(DebugSubscriptionValue::Path(_)) if value.entity_id == witch
        )),
        "{values:?}"
    );

    let clear =
        world.publish_missing_path_for_test(world.require_level_id(&level_key(DIMENSION)), witch);
    assert!(
        clear.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::DebugEntityValue(value),
            } if value.entity_id == witch
                && value.update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID
                && value.update.value.is_none()
        )),
        "{clear:?}"
    );
    let still_missing =
        world.publish_missing_path_for_test(world.require_level_id(&level_key(DIMENSION)), witch);
    assert!(still_missing.is_empty(), "{still_missing:?}");
}

fn block_values(tick: &TickResult) -> Vec<(ConnectionId, Arc<DebugBlockValue>)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DebugBlockValue(update),
            } => Some((*connection_id, Arc::clone(update))),
            _ => None,
        })
        .collect()
}

fn chunk_values(tick: &TickResult) -> Vec<(ConnectionId, Arc<DebugChunkValue>)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DebugChunkValue(update),
            } => Some((*connection_id, Arc::clone(update))),
            _ => None,
        })
        .collect()
}

#[test]
fn operator_raid_publication_answers_every_tracked_column_once() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let non_operator = request(
        &sender,
        &receiver,
        &mut world,
        2,
        vec![VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID],
    );
    assert!(chunk_values(&non_operator).is_empty(), "{non_operator:?}");

    request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID],
    );
    let initial = world.tick(receiver.drain());
    let values = chunk_values(&initial);
    assert!(!values.is_empty(), "{initial:?}");
    assert!(values.iter().all(|(connection_id, value)| {
        *connection_id == ConnectionId(1)
            && value.update.subscription_id == VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID
            && matches!(
                value.update.value,
                Some(DebugSubscriptionValue::Raids(ref centers)) if centers.is_empty()
            )
    }));
    let keys = values
        .iter()
        .map(|(_, value)| (value.pos.x, value.pos.z))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(keys.len(), values.len(), "{values:?}");
    assert!(keys.contains(&(0, 0)), "{keys:?}");

    let stable = world.tick(receiver.drain());
    assert!(chunk_values(&stable).is_empty(), "{stable:?}");

    let cancelled = request(&sender, &receiver, &mut world, 1, Vec::new());
    assert!(chunk_values(&cancelled).is_empty(), "{cancelled:?}");
    let asleep = world.tick(receiver.drain());
    assert!(chunk_values(&asleep).is_empty(), "{asleep:?}");
}

#[test]
fn operator_poi_publication_names_a_record_then_clears_the_position_it_leaves() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let pos = BlockPos { x: 5, y: 65, z: 5 };
    assert!(world.set_block(
        DIMENSION,
        BlockPos { y: 64, ..pos },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL
    ));
    let bell =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:bell").unwrap();
    let bell = bbb_protocol::block_state_types::vanilla_block_default_state_id(bell.id).unwrap();
    assert!(world.set_block(DIMENSION, pos, bell, 0));

    request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![VANILLA_DEBUG_SUBSCRIPTION_POIS_ID],
    );
    let initial = world.tick(receiver.drain());
    let values = block_values(&initial);
    assert_eq!(values.len(), 1, "{initial:?}");
    assert_eq!(values[0].0, ConnectionId(1));
    assert_eq!(values[0].1.pos, pos);
    assert_eq!(
        values[0].1.update.subscription_id,
        VANILLA_DEBUG_SUBSCRIPTION_POIS_ID
    );
    // `PoiTypes.MEETING` is holder 14 and carries 32 tickets (PoiTypes.java:138).
    assert!(matches!(
        values[0].1.update.value,
        Some(DebugSubscriptionValue::Poi(ref poi))
            if poi.pos == pos && poi.poi_type_holder_id == 14 && poi.free_ticket_count == 32
    ));

    let stable = world.tick(receiver.drain());
    assert!(block_values(&stable).is_empty(), "{stable:?}");

    assert!(world.acquire_poi_ticket(&level_key(DIMENSION), [pos.x, pos.y, pos.z]));
    let taken = world.tick(receiver.drain());
    let values = block_values(&taken);
    assert_eq!(values.len(), 1, "{taken:?}");
    assert!(matches!(
        values[0].1.update.value,
        Some(DebugSubscriptionValue::Poi(ref poi)) if poi.free_ticket_count == 31
    ));

    assert!(world.set_block(
        DIMENSION,
        pos,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL
    ));
    let removed = world.tick(receiver.drain());
    let values = block_values(&removed);
    assert_eq!(values.len(), 1, "{removed:?}");
    assert_eq!(values[0].1.pos, pos);
    assert!(values[0].1.update.value.is_none(), "{values:?}");

    let quiet = world.tick(receiver.drain());
    assert!(block_values(&quiet).is_empty(), "{quiet:?}");
}

fn brain_dumps(tick: &TickResult) -> Vec<(i32, bbb_protocol::packets::DebugBrainDump)> {
    debug_values(tick)
        .into_iter()
        .filter_map(|(_, value)| match &value.update.value {
            Some(DebugSubscriptionValue::Brain(brain)) => Some((value.entity_id, brain.clone())),
            _ => None,
        })
        .collect()
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation_id: u64,
    command: &str,
) -> TickResult {
    enqueue(
        sender,
        WorldCommand::Admin {
            envelope: admin_envelope(
                operation_id,
                &format!("{operation_id}:{command}"),
                AdminScope::Command,
            ),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

/// A villager staged the way `mob_brain_gate` stages one: no AI, a job site in
/// its brain and villager data beside it.
fn staged_villager_record() -> Vec<NbtSummaryEntry> {
    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }
    fn global_pos(pos: [i32; 3]) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
            entry("pos", NbtSummaryValue::IntArray(pos.to_vec())),
        ])
    }
    fn memory(module: &str, pos: [i32; 3]) -> NbtSummaryEntry {
        entry(
            module,
            NbtSummaryValue::Compound(vec![entry("value", global_pos(pos))]),
        )
    }
    vec![
        entry("NoAI", NbtSummaryValue::Byte(1)),
        entry("Age", NbtSummaryValue::Int(0)),
        entry(
            "Brain",
            NbtSummaryValue::Compound(vec![entry(
                "memories",
                NbtSummaryValue::Compound(vec![memory("minecraft:job_site", [5, 64, 5])]),
            )]),
        ),
        entry(
            "VillagerData",
            NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:plains")),
                entry("profession", NbtSummaryValue::string("minecraft:farmer")),
                entry("level", NbtSummaryValue::Int(1)),
            ]),
        ),
        entry("Xp", NbtSummaryValue::Int(5)),
        entry(
            "Inventory",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::string("minecraft:bread")),
                entry("count", NbtSummaryValue::Int(3)),
            ])]),
        ),
        entry(
            "Gossips",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("Target", NbtSummaryValue::IntArray(vec![0, 0, 0, 9])),
                entry("Type", NbtSummaryValue::string("minor_positive")),
                entry("Value", NbtSummaryValue::Int(12)),
            ])]),
        ),
    ]
}

/// `Mob.registerDebugValues` (Mob.java:1439-1441) publishes a dump for every
/// mob whose brain is not brain-dead, and `DebugBrainDump.takeBrainDump`
/// (DebugBrainDump.java:93-128) fills it out of what the mob holds.
#[test]
fn a_staged_villager_and_frog_publish_the_brain_the_save_handed_them() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    command(&sender, &receiver, &mut world, 1, "time set noon");
    let villager_id = spawn_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 2.5,
        },
    );
    world.load_stored_entity_record_for_test(DIMENSION, villager_id, &staged_villager_record());
    let frog_id = spawn_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FROG_ID,
        Vec3d {
            x: -0.5,
            y: 64.0,
            z: -3.5,
        },
    );
    let requested = request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID],
    );
    assert!(brain_dumps(&requested).is_empty(), "{requested:?}");

    let published = world.tick(receiver.drain());
    let dumps = brain_dumps(&published)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(dumps.len(), 2, "{dumps:?}");

    let villager = &dumps[&villager_id];
    assert_eq!(villager.profession, "minecraft:farmer");
    assert_eq!(villager.xp, 5);
    assert_eq!(villager.inventory, "[3 minecraft:bread]");
    assert_eq!(
        villager.gossips,
        [format!("{}: MINOR_POSITIVE: 12", uuid::Uuid::from_u128(9))]
    );
    assert_eq!(villager.max_health, 20.0);
    assert!((0.0..=villager.max_health).contains(&villager.health));
    assert!(!villager.wants_golem);
    assert_eq!(villager.anger_level, -1);
    // `time set noon` puts the schedule in its work keyframe
    // (`timeline/villager_schedule.json`), which the job site allows.
    assert_eq!(villager.activities, ["core", "work"]);
    assert!(villager.behaviors.is_empty());
    assert_eq!(villager.memories.len(), 30);
    assert!(villager
        .memories
        .iter()
        .any(|line| line.starts_with("job_site:") && !line.ends_with(": -")));
    assert_eq!(
        villager.pois,
        [bbb_protocol::packets::BlockPos { x: 5, y: 64, z: 5 }]
    );
    assert!(villager.potential_pois.is_empty());

    let frog = &dumps[&frog_id];
    assert_eq!(frog.profession, "");
    assert_eq!(frog.xp, 0);
    assert_eq!(frog.inventory, "");
    assert_eq!(frog.max_health, 10.0);
}

/// `EuclideanGameEventListenerRegistry.sendDebugInfo`
/// (`EuclideanGameEventListenerRegistry.java:45-59`): a subscribed operator
/// tracking the listener's column receives the listener's radius as a block
/// debug value, including when the listener already existed before the
/// subscription.
#[test]
fn operator_game_event_listener_publication_replays_registered_listeners() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let listener = BlockPos { x: 2, y: 64, z: 2 };
    let catalyst_state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_SCULK_CATALYST_ID,
    )
    .expect("the catalyst has a default state");
    world.add_block_entity_for_test(DIMENSION, listener, catalyst_state);

    request(
        &sender,
        &receiver,
        &mut world,
        1,
        vec![VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID],
    );
    let tick = world.tick(receiver.drain());
    let values = block_values(&tick);
    assert_eq!(
        values
            .iter()
            .filter(|(connection_id, update)| {
                *connection_id == ConnectionId(1)
                    && update.pos == listener
                    && update.update.subscription_id
                        == VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID
            })
            .map(|(_, update)| update.update.value.clone())
            .collect::<Vec<_>>(),
        vec![Some(DebugSubscriptionValue::GameEventListener(
            bbb_protocol::packets::DebugGameEventListenerInfo { listener_radius: 8 }
        ))]
    );

    let stable = world.tick(receiver.drain());
    assert!(
        block_values(&stable)
            .iter()
            .all(|(_, update)| update.update.subscription_id
                != VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID),
        "the listener value is not replayed while it stays unchanged"
    );
}

/// `RemoteSampleLogger.useSample` (`RemoteSampleLogger.java:21-25`): the tick
/// time sample reaches the operators subscribed to it and no one else.
#[test]
fn the_tick_time_sample_reaches_subscribed_operators() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let tick_time =
        bbb_protocol::debug_subscription_types::VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID;
    request(&sender, &receiver, &mut world, 1, vec![tick_time]);
    request(&sender, &receiver, &mut world, 2, vec![tick_time]);

    world.log_tick_time_sample([50, 4, 0, 46]);
    let tick = world.tick(receiver.drain());

    let samples: Vec<_> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DebugSample(sample),
            } => Some((*connection_id, sample.sample.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(samples, [(ConnectionId(1), vec![50, 4, 0, 46])]);
}
