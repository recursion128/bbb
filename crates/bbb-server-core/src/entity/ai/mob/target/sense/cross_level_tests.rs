use super::*;
use crate::connection::ConnectionId;
use crate::entity::ai::mob::target::conditions::is_allied_to;
use crate::entity::ai::mob::target::goals::{NearestAttackableTargetGoal, NearestVariant};
use crate::entity::ai::mob::target::owner::{OwnerMemory, OwnerTargetGoal};
use crate::entity::ai::mob::target::selector::TargetSelector;
use crate::entity::ai::mob::target::{TargetRef, TargetView};
use crate::entity::ai::{mob::MobAiContext, Goal};
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::EntityIdCounter;
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use crate::random::GameRandom;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;

const REMOTE: Java26BuiltinDimension = Java26BuiltinDimension::Nether;
fn at(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 0.5 }
}

fn command(
    native: &mut NativeLevel,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    text: &str,
) {
    sender
        .try_send(crate::world::WorldCommand::Admin {
            envelope: crate::admin::AdminEnvelope {
                operation_id: 1,
                request_id: text.into(),
                principal: "fixture".into(),
                scope: crate::admin::AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: crate::admin::AdminRequest::ExecuteCommand {
                command: text.into(),
            },
        })
        .unwrap();
    let result = native.world.tick(receiver.drain());
    assert!(result.events.iter().any(
        |event| matches!(event, crate::world::WorldEvent::Admin(outcome) if matches!(&outcome.result, Ok(crate::admin::AdminResult::Command(report)) if report.result.is_some()))
    ), "{text}: {:?}", result.events);
}

fn scene() -> (NativeLevel, WorldCommandSender, WorldCommandReceiver) {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    for x in -1..=5 {
        for z in -1..=1 {
            native.world.set_block(
                REMOTE,
                bbb_protocol::packets::BlockPos { x, y: 63, z },
                state("minecraft:stone"),
                2 | 16,
            );
            for y in 64..=67 {
                native.world.set_block(
                    REMOTE,
                    bbb_protocol::packets::BlockPos { x, y, z },
                    state("minecraft:air"),
                    2 | 16,
                );
            }
        }
    }
    command(
        &mut native,
        &sender,
        &receiver,
        "execute in minecraft:the_nether run forceload add 0 0",
    );
    (native, sender, receiver)
}

fn spawn(native: &mut NativeLevel, dimension: Java26BuiltinDimension, kind: i32, x: f64) -> i32 {
    native
        .world
        .spawn_stored_entity_for_test(
            dimension,
            kind,
            at(x),
            EntityData::GeneralLiving(crate::entity::GeneralLivingData::new(kind).unwrap()),
        )
        .0
}
fn reference(native: &NativeLevel, dimension: Java26BuiltinDimension, id: i32) -> TargetRef {
    TargetRef {
        entity_id: id,
        uuid: native
            .world
            .stored_entity_for_test(dimension, id)
            .unwrap()
            .uuid(),
    }
}
fn owner(native: &mut NativeLevel, dimension: Java26BuiltinDimension, id: i32, uuid: Uuid) {
    let EntityData::GeneralLiving(data) = &mut native
        .world
        .stored_entity_mut_for_test(dimension, id)
        .unwrap()
        .data
    else {
        panic!("living")
    };
    let tame = data.tamable_retained_mut().unwrap();
    tame.set_owner(Some(uuid));
    tame.set_tame(true);
}

fn local_store(native: &NativeLevel, ids: &[i32]) -> EntityStore {
    let counter = EntityIdCounter::new();
    let mut store = EntityStore::new(71, counter.clone());
    store.set_difficulty(Difficulty::Normal);
    let mut previous = 0;
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    for id in ids {
        for _ in previous + 1..id {
            counter.next_id();
        }
        let source = native.world.stored_entity_for_test(DIMENSION, id).unwrap();
        store
            .restore(
                source.uuid(),
                source.entity_type_id(),
                source.position(),
                source.data().clone(),
            )
            .unwrap();
        previous = id;
    }
    store
}
fn collect<'a, 'l, 'w>(
    store: &'a EntityStore,
    id: i32,
    level: &'a crate::world::simulation_level::LevelCtx<'l, 'w>,
) -> TargetView<'a, 'l, 'w> {
    store
        .mob_ai(id)
        .unwrap()
        .target_view(store.get(id).unwrap(), store, level)
}
fn ai_context(
    store: &EntityStore,
    id: i32,
    world: &TargetView<'_, '_, '_>,
    seed: i64,
) -> MobAiContext {
    let body = store.get(id).unwrap();
    MobAiContext::from_living_body(
        id,
        20,
        body.position(),
        body.bounding_box(),
        world.body().eye_y(),
        0.0,
        0.0,
        true,
        false,
        false,
        body.living().unwrap(),
        GameRandom::with_seed(seed),
    )
}

#[test]
fn remote_owner_root_team_hurt_and_held_target_use_the_actual_world() {
    let (mut native, sender, receiver) = scene();
    let body = spawn(&mut native, DIMENSION, VANILLA_ENTITY_TYPE_WOLF_ID, 0.5);
    let guardian = spawn(&mut native, REMOTE, VANILLA_ENTITY_TYPE_WOLF_ID, 1.5);
    let root = spawn(&mut native, REMOTE, VANILLA_ENTITY_TYPE_COW_ID, 2.5);
    let attacker = spawn(&mut native, REMOTE, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 3.5);
    let guardian_ref = reference(&native, REMOTE, guardian);
    let root_ref = reference(&native, REMOTE, root);
    let attacker_ref = reference(&native, REMOTE, attacker);
    owner(&mut native, DIMENSION, body, guardian_ref.uuid);
    owner(&mut native, REMOTE, guardian, root_ref.uuid);
    let body_ref = reference(&native, DIMENSION, body);
    let villager = spawn(&mut native, REMOTE, VANILLA_ENTITY_TYPE_VILLAGER_ID, 4.5);
    let EntityData::GeneralLiving(data) = &mut native
        .world
        .stored_entity_mut_for_test(REMOTE, villager)
        .unwrap()
        .data
    else {
        panic!("villager")
    };
    data.villager_gossips_mut()
        .unwrap()
        .push(crate::entity::general_living::VillagerGossip {
            target: body_ref.uuid,
            kind: crate::entity::general_living::GossipKind::MajorNegative,
            value: 4,
        });
    native.world.tick(receiver.drain());
    command(&mut native, &sender, &receiver, "team add family");
    for member in [body_ref.uuid, root_ref.uuid] {
        command(
            &mut native,
            &sender,
            &receiver,
            &format!("team join family {member}"),
        );
    }
    command(
        &mut native,
        &sender,
        &receiver,
        &format!(
            "execute in minecraft:the_nether run damage {} 1 minecraft:mob_attack by {}",
            guardian_ref.uuid, attacker_ref.uuid
        ),
    );
    for _ in 0..32 {
        native.world.tick(receiver.drain());
        if native.world.mob_target_for_test(REMOTE, guardian).is_some() {
            break;
        }
    }
    assert_eq!(
        native.world.mob_target_for_test(REMOTE, guardian),
        Some(attacker_ref.uuid)
    );
    native.world.tick(receiver.drain());
    let local = local_store(&native, &[body]);
    native.with_level(|level| {
        let mut world = collect(&local, body, level);
        assert_eq!(world.body().owner(), Some(guardian));
        assert_eq!(world.body().root_owner(), Some(root));
        assert_eq!(
            world.facts(villager).unwrap().reputation_of(body_ref.uuid),
            -20
        );
        let facts = world.facts(guardian).unwrap();
        assert!(!facts.in_body_level());
        assert_eq!(facts.owner(), Some(root));
        assert_eq!(facts.last_hurt_by_mob(), Some(attacker));
        assert_eq!(facts.valid_target(), Some(attacker_ref));
        assert_eq!(facts.team(), Some(0));
        assert_eq!(world.facts(root).unwrap().team(), Some(0));
        assert!(is_allied_to(
            &world,
            &world.body(),
            &world.facts(root).unwrap()
        ));
        assert_eq!(facts.rides_body(), Some(false));
        let mut context = ai_context(&local, body, &world, 3);
        let mut goal = OwnerTargetGoal::new(OwnerMemory::HurtBy);
        assert!(goal.can_use(&mut context, &mut world));
        goal.start(&mut context, &mut world);
        assert_eq!(context.target.target, Some(attacker_ref));
    });
}

#[test]
fn fox_first_resolved_remote_trusted_player_without_attacker_never_falls_through() {
    let (mut native, sender, receiver) = scene();
    for connection in [1, 2] {
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut native.world,
            connection,
            &format!("trusted{connection}"),
            9000 + u128::from(connection),
        );
        let id = ConnectionId(connection);
        native.world.set_player_mode_and_abilities_for_test(
            id,
            bbb_protocol::game_types::GameType::Survival,
            bbb_protocol::packets::PlayerAbilities::for_game_type(
                bbb_protocol::game_types::GameType::Survival,
            ),
        );
        let mut events = Vec::new();
        assert!(native.world.teleport_player(
            id,
            crate::world::TeleportTransition {
                new_level: REMOTE.into(),
                position: at(1.5),
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
            &mut events
        ));
        for event in events {
            if let crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::PlayerPosition(update),
                ..
            } = event
            {
                sender
                    .try_send(crate::world::WorldCommand::Play {
                        connection_id: id,
                        packet: bbb_protocol::packets::PlayServerbound::AcceptTeleportation {
                            id: update.id,
                        },
                    })
                    .unwrap();
            }
        }
        native.world.tick(receiver.drain());
    }
    native.world.set_player_shared_flag_for_test(
        ConnectionId(1),
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        true,
    );
    let inventory = native.world.player_inventory_for_test(ConnectionId(1));
    let head = inventory
        .equipment_slots()
        .into_iter()
        .find(|(slot, _)| *slot == bbb_protocol::equipment_slots::EquipmentSlot::Head)
        .unwrap()
        .1;
    *inventory.slot_mut(head).unwrap() =
        crate::tests::native_stack(bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_HELMET_ID),
            count: 1,
            component_patch: Default::default(),
        });
    native.world.tick(receiver.drain());
    let first = native
        .world
        .nearby_players_for_test(REMOTE)
        .into_iter()
        .find(|p| p.entity_id == native.world.player_entity_id_for_test(ConnectionId(1)))
        .unwrap();
    let second = native
        .world
        .nearby_players_for_test(REMOTE)
        .into_iter()
        .find(|p| p.entity_id == native.world.player_entity_id_for_test(ConnectionId(2)))
        .unwrap();
    let fox = spawn(&mut native, DIMENSION, VANILLA_ENTITY_TYPE_FOX_ID, 0.5);
    let attacker = spawn(&mut native, REMOTE, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 2.5);
    let EntityData::GeneralLiving(data) = &mut native
        .world
        .stored_entity_mut_for_test(REMOTE, attacker)
        .unwrap()
        .data
    else {
        panic!("living");
    };
    data.set_no_ai(true);
    let attacker_ref = reference(&native, REMOTE, attacker);
    let mut local = local_store(&native, &[fox]);
    let trusted = NbtSummaryEntry {
        name: "Trusted".into(),
        value: NbtSummaryValue::List(vec![
            crate::entity::save::uuid_int_array(first.uuid),
            crate::entity::save::uuid_int_array(second.uuid),
        ]),
    };
    let NbtSummaryValue::Compound(mut saved) = local.command_data(fox).unwrap() else {
        panic!("fox compound");
    };
    saved.retain(|field| field.name != "Trusted");
    saved.push(trusted);
    local.load_command_data(fox, &saved).unwrap();
    command(
        &mut native,
        &sender,
        &receiver,
        &format!(
            "execute in minecraft:the_nether run damage trusted2 1 minecraft:mob_attack by {}",
            attacker_ref.uuid
        ),
    );
    native.with_level(|level| {
        let mut world = collect(&local, fox, level);
        let facts = world.facts(first.entity_id).unwrap();
        assert!(!facts.in_body_level());
        let live = facts.player().unwrap();
        assert_eq!(facts.eye_y(), live.position.y + live.eye_height);
        assert_eq!(facts.armor_cover(), 0.25);
        assert_eq!(
            world.facts(second.entity_id).unwrap().last_hurt_by_mob(),
            Some(attacker)
        );
        for seed in 0..100 {
            let mut goal = NearestAttackableTargetGoal::new(
                NearestVariant::FoxDefendTrusted,
                "LivingEntity",
                10,
                false,
                TargetSelector::None,
            );
            let mut context = ai_context(&local, fox, &world, seed);
            assert!(!goal.can_use(&mut context, &mut world));
        }
    });
    command(
        &mut native,
        &sender,
        &receiver,
        &format!(
            "execute in minecraft:the_nether run damage trusted1 1 minecraft:mob_attack by {}",
            attacker_ref.uuid
        ),
    );
    native.world.tick(receiver.drain());
    let selected = native.with_level(|level| {
        let mut world = collect(&local, fox, level);
        (0..100).find_map(|seed| {
            let mut goal = NearestAttackableTargetGoal::new(
                NearestVariant::FoxDefendTrusted,
                "LivingEntity",
                10,
                false,
                TargetSelector::None,
            );
            let mut context = ai_context(&local, fox, &world, seed);
            if !goal.can_use(&mut context, &mut world) {
                return None;
            }
            goal.start(&mut context, &mut world);
            context.target.target
        })
    });
    assert_eq!(selected, Some(attacker_ref));
}

#[test]
fn named_remote_entities_at_same_coordinates_do_not_enter_real_nearest_search() {
    let (mut native, sender, receiver) = scene();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "remote", 9000);
    assert!(native.world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: REMOTE.into(),
            position: at(0.5),
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new()
    ));
    let session = native.world.nearby_players_for_test(REMOTE)[0];
    let body = spawn(&mut native, DIMENSION, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 0.5);
    let cow = spawn(&mut native, REMOTE, VANILLA_ENTITY_TYPE_COW_ID, 0.5);
    native.world.tick(receiver.drain());
    let local = local_store(&native, &[body]);
    native.with_level(|level| {
        let mut world = collect(&local, body, level);
        assert!(world.facts(cow).is_some());
        assert!(world.facts(session.entity_id).is_some());
        assert_eq!(world.local_living().count(), 0);
        for class in ["LivingEntity", "Player"] {
            let mut context = ai_context(&local, body, &world, 3);
            let mut goal = NearestAttackableTargetGoal::new(
                NearestVariant::Plain,
                class,
                0,
                false,
                TargetSelector::None,
            );
            assert!(!goal.can_use(&mut context, &mut world));
        }
    });
    let nearby = spawn(&mut native, DIMENSION, VANILLA_ENTITY_TYPE_COW_ID, 1.5);
    let local = local_store(&native, &[body, nearby]);
    let expected = reference(&native, DIMENSION, nearby);
    native.with_level(|level| {
        let mut world = collect(&local, body, level);
        let mut context = ai_context(&local, body, &world, 3);
        let mut goal = NearestAttackableTargetGoal::new(
            NearestVariant::Plain,
            "LivingEntity",
            0,
            false,
            TargetSelector::None,
        );
        assert!(goal.can_use(&mut context, &mut world));
        goal.start(&mut context, &mut world);
        assert_eq!(context.target.target, Some(expected));
    });
}

#[test]
fn hurt_alert_only_changes_local_kin_even_when_remote_kin_overlaps_body() {
    let (mut native, sender, receiver) = scene();
    let body = spawn(&mut native, DIMENSION, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 0.5);
    let ally = spawn(&mut native, DIMENSION, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 1.5);
    let attacker = spawn(&mut native, DIMENSION, VANILLA_ENTITY_TYPE_COW_ID, 2.5);
    let foreign = spawn(&mut native, REMOTE, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 0.5);
    let body_ref = reference(&native, DIMENSION, body);
    native.world.tick(receiver.drain());
    let attacker_ref = reference(&native, DIMENSION, attacker);
    command(
        &mut native,
        &sender,
        &receiver,
        &format!(
            "damage {} 1 minecraft:mob_attack by {}",
            body_ref.uuid, attacker_ref.uuid
        ),
    );
    for _ in 0..32 {
        native.world.tick(receiver.drain());
        if native.world.mob_target_for_test(DIMENSION, body).is_some()
            && native.world.mob_target_for_test(DIMENSION, ally).is_some()
        {
            break;
        }
    }
    assert_eq!(
        native.world.mob_target_for_test(DIMENSION, body),
        Some(attacker_ref.uuid)
    );
    assert_eq!(
        native.world.mob_target_for_test(DIMENSION, ally),
        Some(attacker_ref.uuid)
    );
    assert_eq!(native.world.mob_target_for_test(REMOTE, foreign), None);
}
