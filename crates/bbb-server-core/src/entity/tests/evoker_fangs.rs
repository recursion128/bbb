use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::WorldEvent;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::PlayerAbilities;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID;
use bbb_protocol::packets::Vec3d;

use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::{spawn_evoker_fangs, tick_entities, EntityData, EvokerFangsData};
use crate::level_event::VanillaGameEvent;

fn fang_at(store: &mut EntityStore, warmup: i32, owner: Option<uuid::Uuid>) -> i32 {
    let id = spawn_evoker_fangs(
        store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        0.0,
        warmup,
        owner,
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    id
}

fn fang_data(store: &EntityStore, id: i32) -> EvokerFangsData {
    match store.get(id).expect("entity is live").data() {
        EntityData::EvokerFangs(data) => data.clone(),
        other => panic!("expected evoker fangs, found {other:?}"),
    }
}

#[test]
fn a_spawned_fang_carries_its_warmup_life_and_derived_yaw() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = fang_at(&mut store, 5, None);
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID);
    let mut spun = EntityStore::new(2, EntityIdCounter::new());
    let spun_id = spawn_evoker_fangs(
        &mut spun,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        std::f32::consts::FRAC_PI_2,
        0,
        None,
    )
    .expect("a fresh UUID");
    assert!((spun.get(spun_id).unwrap().y_rot() - 90.0).abs() < 1.0e-3);
    let data = fang_data(&store, id);
    assert_eq!(data.warmup_delay_ticks(), 5);
    assert_eq!(data.life_ticks(), 22);
    assert!(!data.sent_spike_event());
}

#[test]
fn a_fang_hurts_a_player_and_sends_the_attack_event() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "victim", 1);
    native.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    native.world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let before = native.world.player_health_for_test(ConnectionId(1));
    let fang = native
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            EntityData::EvokerFangs(EvokerFangsData::new(0, None)),
        )
        .0;
    let mut events = Vec::new();
    for _ in 0..8 {
        events.extend(native.world.tick(receiver.drain()).events);
    }
    assert_eq!(
        native.world.player_health_for_test(ConnectionId(1)),
        before - 6.0
    );
    let magic = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:magic")
        .unwrap()
        .registry_id;
    assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(hurt), .. } if hurt.source_type_id == magic && hurt.source_cause_id == -1 && hurt.source_direct_id == -1)));
    assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(update), .. } if update.entity_id == fang && update.event_id == 4)));
}

#[test]
fn a_fang_with_a_living_owner_uses_indirect_magic_and_never_hits_the_owner() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    for (id, name) in [(1, "owner"), (2, "victim")] {
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut native.world,
            id,
            name,
            u128::from(id),
        );
        native.world.set_player_position_for_test(
            ConnectionId(id),
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        );
        native.world.set_player_mode_and_abilities_for_test(
            ConnectionId(id),
            GameType::Survival,
            PlayerAbilities::for_game_type(GameType::Survival),
        );
    }
    let owner_before = native.world.player_health_for_test(ConnectionId(1));
    let victim_before = native.world.player_health_for_test(ConnectionId(2));
    let owner_id = native.world.player_entity_id_for_test(ConnectionId(1));
    let fang = native
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            EntityData::EvokerFangs(EvokerFangsData::new(0, Some(Uuid::from_u128(1)))),
        )
        .0;
    let mut events = Vec::new();
    for _ in 0..8 {
        events.extend(native.world.tick(receiver.drain()).events);
    }
    assert_eq!(
        native.world.player_health_for_test(ConnectionId(1)),
        owner_before
    );
    assert_eq!(
        native.world.player_health_for_test(ConnectionId(2)),
        victim_before - 6.0
    );
    let magic = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:indirect_magic")
        .unwrap()
        .registry_id;
    assert!(events.iter().any(|event| matches!(event,WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(hurt), .. } if hurt.source_type_id==magic && hurt.source_cause_id==owner_id && hurt.source_direct_id==fang)));
}

#[test]
fn a_fang_refuses_to_damage_an_armor_stand_and_raises_no_game_event() {
    let mut store = EntityStore::new(6, EntityIdCounter::new());
    let stand = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    fang_at(&mut store, 0, None);
    store.drain_lifecycle_events();
    let health_before = store.get(stand).unwrap().living().unwrap().health();
    let mut level = NativeLevel::new();
    for _ in 0..8 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(stand).is_some());
    assert_eq!(
        store.get(stand).unwrap().living().unwrap().health(),
        health_before
    );
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .all(|event| event.recorded.game_event != VanillaGameEvent::ENTITY_DAMAGE));
}

#[test]
fn fang_with_mob_owner_respects_mob_griefing_before_armor_stand_damage_tags() {
    for mob_griefing in [false, true] {
        let mut store = EntityStore::new(0xF4A7, EntityIdCounter::new());
        let owner = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 8.0,
                    y: 64.0,
                    z: 8.0,
                },
            )
            .unwrap();
        let owner_uuid = store.get(owner).unwrap().uuid();
        let stand = crate::entity::spawn_armor_stand(
            &mut store,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
        let fang = fang_at(&mut store, -7, Some(owner_uuid));
        store.drain_lifecycle_events();
        let before = store.get(stand).unwrap().living().unwrap().health();
        let mut level = NativeLevel::new();
        let (sender, receiver) = crate::tests::mailbox();
        sender
            .try_send(crate::world::WorldCommand::Admin {
                envelope: crate::admin::AdminEnvelope {
                    operation_id: 1,
                    request_id: "rule".into(),
                    principal: "fixture".into(),
                    scope: crate::admin::AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 0,
                },
                request: crate::admin::AdminRequest::ExecuteCommand {
                    command: format!("gamerule mob_griefing {mob_griefing}"),
                },
            })
            .unwrap();
        level.world.tick(receiver.drain());
        level.install_tags(registry_tag_snapshot(&[(
            "minecraft:damage_type",
            "minecraft:can_break_armor_stand",
            &["minecraft:indirect_magic"],
        )]));

        level.with_level(|level| crate::entity::tick_one_entity(&mut store, level, fang));

        if mob_griefing {
            assert!(
                store.get(stand).is_none(),
                "the same actual source breaks the tagged stand when mob griefing permits it"
            );
        } else {
            assert_eq!(store.get(stand).unwrap().living().unwrap().health(), before);
            assert!(level
                .world
                .positioned_game_events_for_test(DIMENSION)
                .iter()
                .all(|event| event.recorded.game_event != VanillaGameEvent::ENTITY_DAMAGE));
        }
    }
}

#[test]
fn a_fang_discards_itself_after_its_life_runs_out() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = fang_at(&mut store, 0, None);
    let mut level = NativeLevel::new();
    for _ in 0..23 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        if store.get(id).is_none() {
            break;
        }
    }
    assert!(store.get(id).is_none(), "the fang removed itself");
}

#[test]
fn a_fang_round_trips_warmup_and_owner_through_the_save_form() {
    let owner = uuid::Uuid::from_u128(0xf00d);
    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let id = fang_at(&mut store, 9, Some(owner));
    let before = fang_data(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:evoker_fangs"
    );

    let mut reloaded = EntityStore::new(9, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = fang_data(&reloaded, ids[0]);
    assert_eq!(after.warmup_delay_ticks(), before.warmup_delay_ticks());
    assert_eq!(after.owner(), Some(owner));
    assert_eq!(after.life_ticks(), 22);
    assert!(!after.sent_spike_event());
}

#[test]
fn a_fang_without_an_owner_omits_the_owner_key() {
    let mut store = EntityStore::new(10, EntityIdCounter::new());
    fang_at(&mut store, 3, None);
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    let record = document.entities().next().unwrap();
    assert!(record.raw().iter().all(|entry| entry.name != "Owner"));
    let mut reloaded = EntityStore::new(11, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    assert_eq!(fang_data(&reloaded, ids[0]).owner(), None);
}
