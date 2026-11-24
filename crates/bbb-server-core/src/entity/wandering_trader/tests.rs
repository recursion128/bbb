use super::*;
use crate::entity::general_living::{AbstractHorseRetainedState, AgeableMobRetainedState};
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::{Entity, MerchantOffers};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_PIG_ID as PIG, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID as LLAMA,
    VANILLA_ENTITY_TYPE_TURTLE_ID as TURTLE, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID as TRADER,
};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}
fn spawn(store: &mut EntityStore, kind: i32) -> i32 {
    let id = store.spawn_general_living(kind, position()).unwrap();
    if let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data {
        data.initialize_merchant_offers(MerchantOffers::new(Vec::new()));
    }
    id
}
fn store() -> EntityStore {
    EntityStore::new(197, EntityIdCounter::new())
}
fn living(entity: &mut Entity) -> &mut crate::entity::GeneralLivingData {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        panic!("living")
    };
    data
}
fn tick(store: &mut EntityStore) {
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(store, level, true))
    };
}

#[test]
fn wandering_trader_alive_tick_no_ai_and_trading_pause() {
    for no_ai in [false, true] {
        let mut native = NativeLevel::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
        native.world.set_player_position_for_test(
            crate::connection::ConnectionId(1),
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 2.5,
            },
        );
        let mut data = crate::entity::GeneralLivingData::new(TRADER).unwrap();
        data.set_no_ai(no_ai);
        let (trader, trader_uuid) = native.world.spawn_stored_entity_for_test(
            DIMENSION,
            TRADER,
            position(),
            crate::entity::EntityData::GeneralLiving(data),
        );
        native
            .world
            .stored_entity_mut_for_test(DIMENSION, trader)
            .unwrap()
            .set_trader_despawn_delay(200);
        sender
            .try_send(crate::world::WorldCommand::Play {
                connection_id: crate::connection::ConnectionId(1),
                packet: bbb_protocol::packets::PlayServerbound::Interact(
                    bbb_protocol::packets::InteractEntity {
                        entity_id: trader,
                        hand: bbb_protocol::packets::InteractionHand::MainHand,
                        location: Vec3d::default(),
                        using_secondary_action: false,
                    },
                ),
            })
            .unwrap();
        let opened = native.world.tick(receiver.drain());
        let screen = opened
            .events
            .iter()
            .find_map(|event| match event {
                crate::world::WorldEvent::Send {
                    packet: crate::connection::ClientboundPlayPacket::OpenScreen(screen),
                    ..
                } => Some(screen),
                _ => None,
            })
            .expect("actual interaction opens the trader menu");
        assert_eq!(
            native
                .world
                .stored_entity_for_test(DIMENSION, trader)
                .unwrap()
                .merchant_trading_player(),
            Some(
                native
                    .world
                    .player_entity_id_for_test(crate::connection::ConnectionId(1))
            )
        );

        let container_id = screen.container_id;
        native
            .world
            .stored_entity_mut_for_test(DIMENSION, trader)
            .unwrap()
            .set_trader_despawn_delay(2);
        native.world.tick(receiver.drain());
        assert_eq!(
            native
                .world
                .stored_entity_for_test(DIMENSION, trader)
                .unwrap()
                .trader_despawn_delay(),
            Some(2)
        );
        sender
            .try_send(crate::world::WorldCommand::Play {
                connection_id: crate::connection::ConnectionId(1),
                packet: bbb_protocol::packets::PlayServerbound::ContainerClose(
                    bbb_protocol::packets::ContainerCloseRequest { container_id },
                ),
            })
            .unwrap();
        native.world.tick(receiver.drain());
        assert_eq!(
            native
                .world
                .stored_entity_for_test(DIMENSION, trader)
                .unwrap()
                .merchant_trading_player(),
            None
        );
        assert_eq!(
            native
                .world
                .stored_entity_for_test(DIMENSION, trader)
                .unwrap()
                .trader_despawn_delay(),
            Some(1)
        );
        native.world.tick(receiver.drain());
        assert!(native
            .world
            .stored_entity_for_test(DIMENSION, trader)
            .is_none());
        let _ = trader_uuid;
    }
}

#[test]
fn wandering_trader_nonpositive_delay_does_not_overflow_or_discard() {
    for delay in [0, -1, i32::MIN] {
        let mut store = store();
        let id = spawn(&mut store, TRADER);
        store.get_mut(id).unwrap().set_trader_despawn_delay(delay);
        tick(&mut store);
        assert_eq!(store.get(id).unwrap().trader_despawn_delay(), Some(delay));
    }
}

#[test]
fn trader_llama_default_alive_tick_and_each_persistence_guard() {
    for guard in 0..4 {
        let mut store = store();
        let id = spawn(&mut store, LLAMA);
        assert_eq!(store.get(id).unwrap().trader_despawn_delay(), Some(47_999));
        store.get_mut(id).unwrap().set_trader_despawn_delay(2);
        match guard {
            0 => {
                *living(store.get_mut(id).unwrap())
                    .abstract_horse_retained_mut()
                    .unwrap() = AbstractHorseRetainedState::from_saved(false, false, 0, true, None)
            }
            1 => {
                *living(store.get_mut(id).unwrap())
                    .ageable_retained_mut()
                    .unwrap() = AgeableMobRetainedState::from_saved(0, 0, true)
            }
            2 => living(store.get_mut(id).unwrap())
                .mob
                .as_mut()
                .unwrap()
                .set_persistence_required(true),
            3 => {
                let holder = spawn(&mut store, PIG);
                assert!(store.leash_entity_to(id, holder, false));
            }
            _ => unreachable!(),
        }
        tick(&mut store);
        assert_eq!(
            store.get(id).unwrap().trader_despawn_delay(),
            Some(2),
            "guard {guard}"
        );
    }

    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    native.world.set_player_position_for_test(
        crate::connection::ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 2.5,
        },
    );
    native.world.stock_menu_slot_for_test(
        crate::connection::ConnectionId(1),
        36,
        bbb_protocol::packets::ItemStackSummary::empty(),
    );
    let mut data = crate::entity::GeneralLivingData::new(LLAMA).unwrap();
    data.set_no_ai(true);
    let (llama, _) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        LLAMA,
        position(),
        crate::entity::EntityData::GeneralLiving(data),
    );
    native
        .world
        .stored_entity_mut_for_test(DIMENSION, llama)
        .unwrap()
        .set_trader_despawn_delay(200);
    assert!(native
        .world
        .attach_external_player_for_test(crate::connection::ConnectionId(1), llama,));
    assert_eq!(
        native
            .world
            .external_player_vehicle_state_for_test(crate::connection::ConnectionId(1),)
            .2,
        Some(llama)
    );
    native
        .world
        .stored_entity_mut_for_test(DIMENSION, llama)
        .unwrap()
        .set_trader_despawn_delay(2);
    native.world.tick(receiver.drain());
    assert_eq!(
        native
            .world
            .stored_entity_for_test(DIMENSION, llama)
            .unwrap()
            .trader_despawn_delay(),
        Some(2)
    );
    let mut store = store();
    let id = spawn(&mut store, LLAMA);
    living(store.get_mut(id).unwrap())
        .mob
        .as_mut()
        .unwrap()
        .set_no_ai(true);
    store.get_mut(id).unwrap().set_trader_despawn_delay(2);
    tick(&mut store);
    assert_eq!(store.get(id).unwrap().trader_despawn_delay(), Some(1));
    tick(&mut store);
    assert!(store.get(id).is_none());
}

#[test]
fn trader_llama_holder_delay_is_live() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    native.world.set_player_position_for_test(
        crate::connection::ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 2.5,
        },
    );
    let mut data = crate::entity::GeneralLivingData::new(TRADER).unwrap();
    data.set_no_ai(true);
    let (trader, trader_uuid) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        TRADER,
        position(),
        crate::entity::EntityData::GeneralLiving(data),
    );
    native
        .world
        .stored_entity_mut_for_test(DIMENSION, trader)
        .unwrap()
        .set_trader_despawn_delay(200);
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: crate::connection::ConnectionId(1),
            packet: bbb_protocol::packets::PlayServerbound::Interact(
                bbb_protocol::packets::InteractEntity {
                    entity_id: trader,
                    hand: bbb_protocol::packets::InteractionHand::MainHand,
                    location: Vec3d::default(),
                    using_secondary_action: false,
                },
            ),
        })
        .unwrap();
    let opened = native.world.tick(receiver.drain());
    let _screen = opened
        .events
        .iter()
        .find_map(|event| match event {
            crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::OpenScreen(screen),
                ..
            } => Some(screen),
            _ => None,
        })
        .expect("actual interaction opens the trader menu");
    assert_eq!(
        native
            .world
            .stored_entity_for_test(DIMENSION, trader)
            .unwrap()
            .merchant_trading_player(),
        Some(
            native
                .world
                .player_entity_id_for_test(crate::connection::ConnectionId(1))
        )
    );

    native
        .world
        .stored_entity_mut_for_test(DIMENSION, trader)
        .unwrap()
        .set_trader_despawn_delay(8);
    let mut data = crate::entity::GeneralLivingData::new(LLAMA).unwrap();
    data.set_no_ai(true);
    let (llama, _) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        LLAMA,
        Vec3d {
            x: 1.5,
            ..position()
        },
        crate::entity::EntityData::GeneralLiving(data),
    );
    let NbtSummaryValue::Compound(mut record) = native
        .world
        .stored_entity_command_data_for_test(DIMENSION, llama)
        .unwrap()
    else {
        panic!("entity record")
    };
    record.retain(|entry| entry.name != "leash");
    record.push(bbb_protocol::NbtSummaryEntry {
        name: "leash".into(),
        value: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
            name: "UUID".into(),
            value: bbb_protocol::component_hover_uuid_nbt(trader_uuid),
        }]),
    });
    native
        .world
        .load_stored_entity_record_for_test(DIMENSION, llama, &record);
    native.world.tick(receiver.drain());
    let llama = native
        .world
        .stored_entity_for_test(DIMENSION, llama)
        .unwrap();
    assert_eq!(llama.trader_despawn_delay(), Some(7));
    assert_eq!(llama.mob_home(), Some((BlockPos { x: 0, y: 64, z: 0 }, 5)));
}

#[test]
fn trader_llama_unresolved_leash_does_not_pause_despawn() {
    let mut store = store();
    let id = spawn(&mut store, LLAMA);
    let entries = vec![bbb_protocol::NbtSummaryEntry {
        name: "leash".into(),
        value: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
            name: "UUID".into(),
            value: NbtSummaryValue::IntArray(vec![1, 2, 3, 4]),
        }]),
    }];
    living(store.get_mut(id).unwrap())
        .mob
        .as_mut()
        .unwrap()
        .leash = crate::entity::leash::read_leash_data(&entries);
    store.get_mut(id).unwrap().set_trader_despawn_delay(2);
    tick(&mut store);
    assert_eq!(store.get(id).unwrap().trader_despawn_delay(), Some(1));
    tick(&mut store);
    assert!(store.get(id).is_none());
}

#[test]
fn mob_home_and_trader_leash_roundtrip_resolves_uuid_after_load() {
    let mut original = store();
    let trader = spawn(&mut original, TRADER);
    let llama = spawn(&mut original, LLAMA);
    let target = BlockPos {
        x: -20,
        y: 72,
        z: 30,
    };
    original
        .get_mut(trader)
        .unwrap()
        .set_wandering_trader_target(Some(target));
    original
        .get_mut(trader)
        .unwrap()
        .set_trader_despawn_delay(300);
    original.get_mut(trader).unwrap().set_mob_home(target, 16);
    assert!(original.leash_entity_to(llama, trader, true));
    let trader_uuid = original.get(trader).unwrap().uuid();
    let llama_uuid = original.get(llama).unwrap().uuid();
    let saved = write_entity_chunk(&original, (0, 0)).unwrap();
    let mut restored = store();
    read_entity_chunk(&mut restored, &saved).unwrap();
    let trader = restored.entity_id_for_uuid(trader_uuid).unwrap();
    let llama = restored.entity_id_for_uuid(llama_uuid).unwrap();
    assert_eq!(
        restored.get(trader).unwrap().wandering_trader_target(),
        Some(target)
    );
    assert_eq!(restored.get(trader).unwrap().mob_home(), Some((target, 16)));
    assert_eq!(
        crate::entity::leash::leash_holder(restored.get(llama).unwrap()),
        None
    );
    tick(&mut restored);
    assert_eq!(
        crate::entity::leash::leash_holder(restored.get(llama).unwrap()),
        Some(trader)
    );
    assert_eq!(
        restored.get(trader).unwrap().trader_despawn_delay(),
        Some(299)
    );
    assert_eq!(
        restored.get(llama).unwrap().trader_despawn_delay(),
        Some(298)
    );
}

#[test]
fn mob_home_arbitrary_radius_and_turtle_leaf_override_have_one_tag() {
    for kind in [PIG, TURTLE] {
        let mut store = store();
        let id = spawn(&mut store, kind);
        let home = BlockPos { x: 8, y: 9, z: 10 };
        assert!(store.get_mut(id).unwrap().set_mob_home(home, -2));
        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        let fields = saved.entities().next().unwrap().raw();
        assert_eq!(
            fields
                .iter()
                .filter(|entry| entry.name == "home_pos")
                .count(),
            1
        );
        let mut restored = self::store();
        let ids = read_entity_chunk(&mut restored, &saved).unwrap();
        assert_eq!(
            restored.get(ids[0]).unwrap().mob_home(),
            Some((BlockPos { x: 0, y: 0, z: 0 }, -2))
        );
        store.get_mut(id).unwrap().set_mob_home(home, -1);
        assert_eq!(store.get(id).unwrap().mob_home(), None);
    }
}

#[test]
fn trader_llama_event_finalization_is_adult() {
    for kind in ["minecraft:trader_llama", "minecraft:wandering_trader"] {
        let mut level = crate::entity::general_living::TestLevel::new();
        level.store = store();
        let mut staged =
            crate::entity::create_trader_event_root(&mut level.store, kind, position()).unwrap();
        assert_eq!(staged.portal_cooldown(), 0);
        crate::entity::set_spawner_age(&mut staged, -24_000);
        staged.set_spawn_rotation(76.0);
        crate::entity::finalize_spawn(
            &mut level,
            &mut staged,
            crate::entity::SpawnReason::Event,
            None,
            &crate::random::GameRandom::with_seed(197),
        )
        .unwrap();
        if kind == "minecraft:trader_llama" {
            assert_eq!(living(&mut staged).ageable_retained().unwrap().age(), 0);
        }
        assert_eq!(staged.y_head_rot(), 76.0);
        assert_eq!(staged.living().unwrap().y_body_rot, 76.0);
        let id = level.store.add_fresh(staged);
        level.store.drain_lifecycle_events();
        assert!(level.store.play_trader_spawn_ambient(id));
        let (expected_sound, expected_volume, pitch_range) = if kind == "minecraft:trader_llama" {
            ("minecraft:entity.llama.ambient", 0.8, 0.8..=1.2)
        } else {
            ("minecraft:entity.wandering_trader.ambient", 1.0, 1.3..=1.7)
        };
        assert!(level.store.drain_lifecycle_events().iter().any(|event| matches!(event,
            EntityLifecycleEvent::Sound { sound }
                if matches!(sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                    if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
                        == Some(expected_sound))
                && sound.source == SoundSource::Neutral
                && sound.position == position()
                && sound.volume == expected_volume
                && pitch_range.contains(&sound.pitch)
        )));
        level.store.get_mut(id).unwrap().set_silent(true);
        assert!(level.store.play_trader_spawn_ambient(id));
        assert!(!level
            .store
            .drain_lifecycle_events()
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
        let creeper = level
            .store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID,
                position(),
            )
            .unwrap();
        assert!(level.store.play_spawn_ambient(creeper, None));
        assert!(!level
            .store
            .drain_lifecycle_events()
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    }
}

#[test]
fn wandering_trader_restore_clamps_negative_age_and_preserves_absent_target() {
    let mut original = store();
    spawn(&mut original, TRADER);
    let saved = write_entity_chunk(&original, (0, 0)).unwrap();
    let mut fields = saved.entities().next().unwrap().raw().to_vec();
    fields
        .iter_mut()
        .find(|entry| entry.name == "Age")
        .unwrap()
        .value = NbtSummaryValue::Int(-100);
    let entry = |name: &str, value| bbb_protocol::NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let saved = Java26EntityChunkData::parse(
        bbb_server_types::save::NamedCompound::new(
            "",
            vec![
                entry(
                    "DataVersion",
                    NbtSummaryValue::Int(bbb_server_types::save::JAVA_26_1_DATA_VERSION),
                ),
                entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                entry(
                    "Entities",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(fields)]),
                ),
            ],
        ),
        0,
        0,
    )
    .unwrap();
    let mut restored = store();
    let id = read_entity_chunk(&mut restored, &saved).unwrap()[0];
    assert_eq!(
        living(restored.get_mut(id).unwrap())
            .ageable_retained()
            .unwrap()
            .age(),
        0
    );
    assert_eq!(restored.get(id).unwrap().wandering_trader_target(), None);
}

#[test]
fn wandering_trader_and_trader_llama_dying_ai_step_still_counts_down() {
    for kind in [TRADER, LLAMA] {
        for delay in [1, 4] {
            let mut store = store();
            let id = spawn(&mut store, kind);
            let entity = store.get_mut(id).unwrap();
            entity.living_mut().unwrap().set_health(0.0);
            entity.set_trader_despawn_delay(delay);
            tick(&mut store);
            if delay == 1 {
                assert!(store.get(id).is_none());
            } else {
                let entity = store.get(id).unwrap();
                assert_eq!(entity.trader_despawn_delay(), Some(3));
                assert_eq!(entity.living().unwrap().death_time, 1);
            }
        }
    }
}

#[test]
fn trader_llama_breeding_offspring_retains_persistence() {
    let mut store = store();
    let parent = spawn(&mut store, LLAMA);
    let mut egg = ItemStackSummary {
        item_id: None,
        count: 1,
        component_patch: DataComponentPatchSummary {
            entity_data_entity_type_id: Some(LLAMA),
            ..Default::default()
        }
        .into(),
    };
    assert_eq!(
        crate::entity::spawn_egg_offspring::spawn_offspring_from_spawn_egg(
            &mut store,
            parent,
            crate::entity::spawn_egg_offspring::SpawnEggUser {
                hand_stack: &mut egg,
                infinite_materials: false,
                uuid: uuid::Uuid::nil(),
            },
            None,
            &crate::entity::level::EntityRegistryTagView::Builtin26_1,
            &crate::random::GameRandom::with_seed(0),
        ),
        crate::entity::spawn_egg_offspring::SpawnEggOffspring::Spawned
    );
    let baby = store.ids().into_iter().find(|id| *id != parent).unwrap();
    let data = living(store.get_mut(baby).unwrap());
    assert!(data.mob.as_ref().unwrap().persistence_required());
    assert!(data.ageable_retained().unwrap().is_baby());
    assert_eq!(
        store.get(baby).unwrap().trader_despawn_delay(),
        Some(47_999)
    );
}

#[test]
fn mob_home_command_data_negative_radius_keeps_previous_position() {
    let mut store = store();
    let id = spawn(&mut store, PIG);
    let home = BlockPos { x: 7, y: 80, z: -9 };
    store.get_mut(id).unwrap().set_mob_home(home, 16);
    let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
        panic!("compound");
    };
    fields
        .iter_mut()
        .find(|entry| entry.name == "home_radius")
        .unwrap()
        .value = NbtSummaryValue::Int(-2);
    store.load_command_data(id, &fields).unwrap();
    assert_eq!(store.get(id).unwrap().mob_home(), Some((home, -2)));
}

#[test]
fn mob_leash_command_data_removal_publishes_unlink_without_lead() {
    let mut store = store();
    let id = spawn(&mut store, PIG);
    let holder =
        crate::entity::hanging::spawn_leash_knot(&mut store, BlockPos { x: 0, y: 64, z: 0 })
            .unwrap();
    assert!(store.leash_entity_to(id, holder, true));
    let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
        panic!("compound");
    };
    fields.retain(|entry| entry.name != "leash");
    store.drain_lifecycle_events();
    store.load_command_data(id, &fields).unwrap();
    assert_eq!(
        crate::entity::leash::leash_holder(store.get(id).unwrap()),
        None
    );
    assert!(store.get(holder).is_none());
    assert!(store
        .ids()
        .iter()
        .all(|id| store.get(*id).unwrap().entity_type_id() != VANILLA_ENTITY_TYPE_ITEM_ID));
    let links: Vec<_> = store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Broadcast {
                packet: crate::connection::ClientboundPlayPacket::SetEntityLink(link),
                ..
            } => Some((link.source_id, link.dest_id)),
            _ => None,
        })
        .collect();
    assert_eq!(links, vec![(id, 0)]);
}

#[test]
fn trader_llama_dying_leash_holder_detaches_before_delay_countdown() {
    let mut store = store();
    let holder = spawn(&mut store, TRADER);
    let id = spawn(&mut store, LLAMA);
    store.get_mut(id).unwrap().set_trader_despawn_delay(10);
    assert!(store.leash_entity_to(id, holder, false));
    store
        .get_mut(holder)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    assert!(!store.leash_entity_to(id, holder, false));
    tick(&mut store);
    assert_eq!(
        crate::entity::leash::leash_holder(store.get(id).unwrap()),
        None
    );
    assert_eq!(store.get(id).unwrap().trader_despawn_delay(), Some(9));
}
