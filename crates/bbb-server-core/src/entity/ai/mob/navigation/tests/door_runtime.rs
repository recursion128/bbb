use super::*;
use crate::entity::tests::native_level::DIMENSION;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const DOOR_FRONT: Vec3d = Vec3d {
    x: 2.5,
    y: 64.0,
    z: 0.5,
};

fn position() -> Vec3d {
    Vec3d {
        x: 1.5,
        y: 64.0,
        z: 0.5,
    }
}

fn set_doors(store: &mut EntityStore, id: i32, value: bool) {
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        panic!("entity compound")
    };
    entries.retain(|e| e.name != "CanBreakDoors");
    entries.push(NbtSummaryEntry {
        name: "CanBreakDoors".into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    });
    store.load_command_data(id, &entries).unwrap();
}

fn can_open(store: &mut EntityStore, id: i32) -> bool {
    store
        .mob_ai_mut(id)
        .unwrap()
        .navigation
        .base()
        .can_open_doors
}

fn route_reaches_door_corridor(store: &mut EntityStore, id: i32) -> bool {
    let mut native = super::doors::door_corridor("minecraft:oak_door", false);
    native.with_level(|level| {
        let entity = store.get(id).unwrap();
        let malus = store.effective_pathfinding_malus(entity, level).unwrap();
        let mob = MobPathingSnapshot::from_entity(entity, false, false, None, malus).unwrap();
        let runtime = store.mob_ai_mut(id).unwrap();
        runtime.navigation.stop();
        runtime.navigate_to(level, mob, BlockPos { x: 5, y: 64, z: 0 }, 0, 0.7);
        runtime.ground_path().is_some_and(GroundPath::reached)
    })
}

#[test]
fn initialized_and_saved_zombie_navigation_uses_retained_door_permission() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store
        .restore_initialized(
            uuid::Uuid::from_u128(9),
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            position(),
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
            |entity| {
                entity.on_ground = true;
                assert!(crate::entity::zombie::apply_conversion_phenotype(
                    entity, false, true
                ));
            },
        )
        .unwrap();
    assert!(can_open(&mut store, id));
    assert!(route_reaches_door_corridor(&mut store, id));
    let saved = crate::entity::write_entity_chunk(&store, (0, 0)).unwrap();
    let mut restored = EntityStore::new(2, EntityIdCounter::new());
    let restored_id = crate::entity::read_entity_chunk(&mut restored, &saved).unwrap()[0];
    assert!(can_open(&mut restored, restored_id));
    assert!(route_reaches_door_corridor(&mut restored, restored_id));
    set_doors(&mut restored, restored_id, false);
    assert!(!can_open(&mut restored, restored_id));
    assert!(!route_reaches_door_corridor(&mut restored, restored_id));
}

fn breaking_zombie() -> (
    NativeLevel,
    i32,
    crate::mailbox::WorldCommandSender,
    crate::mailbox::WorldCommandReceiver,
) {
    let mut native = super::doors::door_corridor("minecraft:oak_door", false);
    for (y, half) in [(64, "lower"), (65, "upper")] {
        native.world.set_block(
            DIMENSION,
            BlockPos { x: 3, y, z: 0 },
            state_with(
                "minecraft:oak_door",
                &[
                    ("facing", "east"),
                    ("half", half),
                    ("hinge", "left"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            ),
            2 | 16,
        );
    }
    // A wall right behind the zombie keeps it inside `BreakDoorGoal`'s
    // two-block reach once it starts, whatever its stroll or target does.
    for y in [64, 65] {
        native.world.set_block(
            DIMENSION,
            BlockPos { x: 1, y, z: 0 },
            state("minecraft:stone"),
            2 | 16,
        );
    }
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "target", 999);
    let connection = crate::connection::ConnectionId(1);
    native.world.set_player_mode_and_abilities_for_test(
        connection,
        bbb_protocol::game_types::GameType::Survival,
        bbb_protocol::packets::PlayerAbilities::for_game_type(
            bbb_protocol::game_types::GameType::Survival,
        ),
    );
    native.world.set_player_position_for_test(
        connection,
        Vec3d {
            x: 5.5,
            y: 64.0,
            z: 0.5,
        },
    );
    command(&mut native, &sender, &receiver, "difficulty hard");
    let (id, uuid) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        DOOR_FRONT,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
        ),
    );
    let NbtSummaryValue::Compound(mut saved) = native
        .world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("entity compound");
    };
    {
        let entry = NbtSummaryEntry {
            name: "CanBreakDoors".into(),
            value: NbtSummaryValue::Byte(1),
        };
        saved.retain(|field| field.name != entry.name);
        saved.push(entry);
    }
    native
        .world
        .load_stored_entity_record_for_test(DIMENSION, id, &saved);
    command(
        &mut native,
        &sender,
        &receiver,
        &format!("damage {uuid} 1 minecraft:player_attack by target"),
    );
    (native, id, sender, receiver)
}

fn command(
    native: &mut NativeLevel,
    sender: &crate::mailbox::WorldCommandSender,
    receiver: &crate::mailbox::WorldCommandReceiver,
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
    ));
}

#[test]
fn zombie_real_travel_breaks_door_and_disk_restore_retains_permission() {
    let (mut native, id, _, receiver) = breaking_zombie();
    let mut broke = false;
    let mut swing = false;
    for _ in 0..2400 {
        let result = native.world.tick(receiver.drain());
        swing |= result.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::EntityAnimation(packet), .. }
            if packet.id == id));
        if native
            .world
            .block_state(DIMENSION, BlockPos { x: 3, y: 65, z: 0 })
            == Some(state("minecraft:air"))
        {
            broke = true;
            break;
        }
    }
    assert!(broke);
    assert!(swing);
    assert!(
        native
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .position()
            .x
            >= 2.0
    );
    for y in [64, 65] {
        assert_eq!(
            native
                .world
                .block_state(DIMENSION, BlockPos { x: 3, y, z: 0 }),
            Some(state("minecraft:air"))
        );
    }
    let NbtSummaryValue::Compound(saved) = native
        .world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("entity")
    };
    let mut restored = EntityStore::new(21, EntityIdCounter::new());
    let restored_id = restored
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    restored.load_command_data(restored_id, &saved).unwrap();
    assert!(can_open(&mut restored, restored_id));
}

#[test]
fn disabling_door_breaking_with_no_ai_preserves_the_door() {
    let (mut native, id, _, receiver) = breaking_zombie();
    let door = BlockPos { x: 3, y: 65, z: 0 };
    let mut cracking = false;
    for _ in 0..2400 {
        let tick = native.world.tick(receiver.drain());
        cracking |= tick.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::BlockDestruction(packet), .. }
            if packet.id == id && packet.pos == door && packet.progress <= 9));
        if cracking {
            break;
        }
    }
    assert!(cracking, "the real door-break overlay starts");
    let NbtSummaryValue::Compound(mut saved) = native
        .world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("entity compound");
    };
    for entry in [
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(1),
        },
        NbtSummaryEntry {
            name: "CanBreakDoors".into(),
            value: NbtSummaryValue::Byte(0),
        },
    ] {
        saved.retain(|field| field.name != entry.name);
        saved.push(entry);
    }
    native
        .world
        .load_stored_entity_record_for_test(DIMENSION, id, &saved);
    let mut cleared = false;
    for _ in 0..320 {
        let tick = native.world.tick(receiver.drain());
        cleared |= tick.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::BlockDestruction(packet), .. }
            if packet.id == id && packet.pos == door && packet.progress == u8::MAX));
    }
    assert!(cleared, "disabling the running goal clears its overlay");
    assert_eq!(
        native
            .world
            .block_state(DIMENSION, BlockPos { x: 3, y: 65, z: 0 }),
        Some(state_with(
            "minecraft:oak_door",
            &[
                ("facing", "east"),
                ("half", "upper"),
                ("hinge", "left"),
                ("open", "false"),
                ("powered", "false")
            ]
        ))
    );
    let NbtSummaryValue::Compound(saved) = native
        .world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("entity")
    };
    let mut restored = EntityStore::new(21, EntityIdCounter::new());
    let restored_id = restored
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    restored.load_command_data(restored_id, &saved).unwrap();
    assert!(!can_open(&mut restored, restored_id));
}
