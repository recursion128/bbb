use super::*;

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_CREAKING_HEART_ID;
use bbb_protocol::entity_synched_data::VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use bbb_protocol::packets::EntityDataValueKind;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const HEART: BlockPos = BlockPos { x: 6, y: 300, z: 6 };

fn heart_state(natural: bool) -> i32 {
    let state = vanilla_block_default_state_id(VANILLA_BLOCK_CREAKING_HEART_ID)
        .expect("heart default state");
    [
        ("axis", "y"),
        ("creaking_heart_state", "dormant"),
        ("natural", if natural { "true" } else { "false" }),
    ]
    .into_iter()
    .fold(state, |state, (name, value)| {
        with_property(state, name, value).expect("heart property")
    })
}

fn heart_world(
    game_type: GameType,
    natural: bool,
    has_block_entity: bool,
    protector: Option<Uuid>,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    Option<i32>,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type,
        datapacks: datapack_config().datapacks,
        ..ServerWorldConfig::default()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "observer", 2);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 5.5,
            y: 300.0,
            z: 6.5,
        },
    );
    world.set_player_position_for_test(
        ConnectionId(2),
        Vec3d {
            x: 20.5,
            y: 300.0,
            z: 6.5,
        },
    );
    world.tick(receiver.drain());
    world.set_game_time_for_test(13_000);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13_000),
        &mut Vec::new(),
    );
    let floor = vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID)
        .expect("stone");
    for x in 3..=15 {
        for z in 3..=9 {
            assert!(world.set_block(
                DIM,
                BlockPos {
                    x,
                    y: HEART.y - 1,
                    z
                },
                floor,
                SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    let log = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:pale_oak_log",
        &[("axis".to_owned(), "y".to_owned())].into_iter().collect(),
    )
    .expect("upright pale oak log");
    for y in [HEART.y - 1, HEART.y + 1] {
        assert!(world.set_block(DIM, BlockPos { y, ..HEART }, log, SET_BLOCK_UPDATE_ALL));
    }
    assert!(world.set_block(DIM, HEART, heart_state(natural), SET_BLOCK_UPDATE_ALL));
    if !has_block_entity {
        world.remove_block_entity_for_test(DIM, HEART);
    }
    let protector_id = protector.map(|uuid| {
        world
            .level_block_entity_for_test(DIM, HEART)
            .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
            .expect("heart block entity")
            .set_protector_for_test(uuid);
        let id = world
            .restore_stored_entity_for_test(
                DIM,
                uuid,
                VANILLA_ENTITY_TYPE_CREAKING_ID,
                Vec3d {
                    x: 10.5,
                    y: 300.0,
                    z: 6.5,
                },
                crate::entity::EntityData::GeneralLiving(
                    crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID)
                        .expect("creaking data"),
                ),
            )
            .expect("protector");
        let entity = world
            .stored_entity_mut_for_test(DIM, id)
            .expect("protector");
        assert!(entity.set_creaking_transient(HEART));
        id
    });
    (sender, receiver, world, protector_id)
}

fn action(kind: PlayerActionKind, sequence: i32) -> PlayServerbound {
    PlayServerbound::PlayerAction(PlayerAction {
        action: kind,
        pos: HEART,
        direction: Direction::Up,
        sequence,
    })
}

fn mine_via_delayed_action(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> Vec<WorldEvent> {
    let mut events = play(
        sender,
        receiver,
        world,
        1,
        action(PlayerActionKind::StartDestroyBlock, 1),
    )
    .events;
    events.extend(
        play(
            sender,
            receiver,
            world,
            1,
            action(PlayerActionKind::StopDestroyBlock, 2),
        )
        .events,
    );
    for _ in 0..500 {
        if world.block_state(DIM, HEART) == Some(world.fixture().air_state_id()) {
            return events;
        }
        events.extend(world.tick(receiver.drain()).events);
    }
    panic!("delayed session break did not finish");
}

fn total_experience(world: &ServerWorld) -> i32 {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| match world.stored_entity_for_test(DIM, id)?.data() {
            crate::entity::EntityData::ExperienceOrb(orb) => Some(orb.value() * orb.count()),
            _ => None,
        })
        .sum()
}

#[test]
fn delayed_survival_break_uses_player_death_before_base_event_and_tears_down_on_tick_46() {
    let (sender, receiver, mut world, protector_id) = heart_world(
        GameType::Survival,
        true,
        true,
        Some(Uuid::from_u128(0x8724)),
    );
    let protector_id = protector_id.expect("protector");
    let mut events = mine_via_delayed_action(&sender, &receiver, &mut world);
    let protector = world.stored_entity_for_test(DIM, protector_id).unwrap_or_else(|| {
        panic!(
            "dead protector missing; death packet = {}, block destroy event = {}",
            events.iter().any(|event| matches!(event.sent_packet(), Some(ClientboundPlayPacket::EntityEvent(update)) if update.entity_id == protector_id && update.event_id == crate::entity::living::ENTITY_EVENT_DEATH)),
            world.game_events(DIM).0.iter().any(|event| event.game_event == VanillaGameEvent::BLOCK_DESTROY && event.pos == HEART),
        )
    });
    assert!(protector.creaking_is_tearing_down());
    assert_eq!(protector.living().expect("living").health(), 0.0);
    assert_eq!(
        protector.living().expect("living").last_hurt_by_player,
        Some(Uuid::from_u128(1))
    );
    assert_eq!(protector.living().expect("living").last_damage_type, None);
    assert!((20..=24).contains(&total_experience(&world)));
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:mob_kills")
        ),
        1
    );
    for _ in 0..20 {
        events.extend(world.tick(receiver.drain()).events);
    }
    assert!(events.iter().any(|event| matches!(event.sent_packet(), Some(ClientboundPlayPacket::SetEntityData(update)) if update.id == protector_id && update.values.iter().any(|value| value.data_id == VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID && value.value == EntityDataValueKind::Boolean(true)))));
    assert!(world.stored_entity_for_test(DIM, protector_id).is_some());
    for _ in 20..45 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world
            .stored_entity_for_test(DIM, protector_id)
            .and_then(|entity| entity.living())
            .map(|living| living.death_time),
        Some(45)
    );
    for observer in [ConnectionId(1), ConnectionId(2)] {
        assert!(world.player_health_for_test(observer) > 0.0);
    }
    let last = world.tick(receiver.drain());
    assert!(world.stored_entity_for_test(DIM, protector_id).is_none());
    for observer in [ConnectionId(1), ConnectionId(2)] {
        let bursts: Vec<_> = last
            .events
            .iter()
            .filter_map(|event| match event.packet_for(observer) {
                Some(ClientboundPlayPacket::LevelParticles(particles)) => Some(particles.count),
                _ => None,
            })
            .collect();
        assert_eq!(bursts, [100, 10]);
    }
}

#[test]
fn heart_player_will_destroy_xp_uses_natural_and_session_mode() {
    let (natural_sender, natural_receiver, mut natural, _) =
        heart_world(GameType::Survival, true, true, None);
    let (unnatural_sender, unnatural_receiver, mut unnatural, _) =
        heart_world(GameType::Survival, false, true, None);
    let seed = 0x7326;
    natural.random.reseed(seed);
    unnatural.random.reseed(seed);
    mine_via_delayed_action(&natural_sender, &natural_receiver, &mut natural);
    mine_via_delayed_action(&unnatural_sender, &unnatural_receiver, &mut unnatural);
    assert!((20..=24).contains(&total_experience(&natural)));
    assert_eq!(total_experience(&unnatural), 0);

    let (sender, receiver, mut creative, protector_id) = heart_world(
        GameType::Creative,
        true,
        true,
        Some(Uuid::from_u128(0x8725)),
    );
    let result = play(
        &sender,
        &receiver,
        &mut creative,
        1,
        action(PlayerActionKind::StartDestroyBlock, 1),
    );
    assert_eq!(
        creative.block_state(DIM, HEART),
        Some(creative.fixture().air_state_id())
    );
    assert!(creative
        .stored_entity_for_test(DIM, protector_id.expect("protector"))
        .is_some_and(|entity| entity.creaking_is_tearing_down()));
    assert_eq!(total_experience(&creative), 0);
    assert!(result.events.iter().any(|event| matches!(event.sent_packet(), Some(ClientboundPlayPacket::EntityEvent(update)) if update.event_id == crate::entity::living::ENTITY_EVENT_DEATH)));

    let (sender, receiver, mut spectator, protector_id) = heart_world(
        GameType::Spectator,
        true,
        true,
        Some(Uuid::from_u128(0x8726)),
    );
    play(
        &sender,
        &receiver,
        &mut spectator,
        1,
        action(PlayerActionKind::StartDestroyBlock, 1),
    );
    assert_eq!(spectator.block_state(DIM, HEART), Some(heart_state(true)));
    assert!(spectator
        .stored_entity_for_test(DIM, protector_id.expect("protector"))
        .is_some_and(|entity| !entity.creaking_is_tearing_down()));
    assert_eq!(total_experience(&spectator), 0);
}

#[test]
fn heart_player_will_destroy_requires_block_entity_but_not_live_protector_for_xp() {
    let (sender, receiver, mut no_be, _) = heart_world(GameType::Survival, true, false, None);
    mine_via_delayed_action(&sender, &receiver, &mut no_be);
    assert_eq!(total_experience(&no_be), 0);

    let (sender, receiver, mut no_live, _) = heart_world(GameType::Survival, true, true, None);
    no_live
        .level_block_entity_for_test(DIM, HEART)
        .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
        .expect("heart")
        .set_protector_for_test(Uuid::from_u128(0x9999));
    mine_via_delayed_action(&sender, &receiver, &mut no_live);
    assert!((20..=24).contains(&total_experience(&no_live)));
}
