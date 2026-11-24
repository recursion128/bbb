use super::*;
use crate::tests::entities::{console_command, console_command_answer, restore_entity_writes};
use bbb_protocol::entity_synched_data::VANILLA_WOLF_ANGER_END_TIME_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID;

fn uuid_words(uuid: Uuid) -> Vec<i32> {
    let bits = uuid.as_u128();
    vec![
        (bits >> 96) as i32,
        (bits >> 64) as i32,
        (bits >> 32) as i32,
        bits as i32,
    ]
}

fn uuid_snbt(uuid: Uuid) -> String {
    let words = uuid_words(uuid);
    format!("[I;{},{},{},{}]", words[0], words[1], words[2], words[3])
}

fn assert_saved(world: &mut ServerWorld, ids: &[i32], expected: (i64, Option<Uuid>)) {
    for &id in ids {
        assert_eq!(saved_anger(world, id), (Some(expected.0), expected.1));
    }
    let request = world
        .save_request(1_786_000_005_000, 206, RegionCompression::Zlib)
        .unwrap();
    for record in saved_entities(&request) {
        assert_eq!(
            field(&record, "anger_end_time"),
            &NbtSummaryValue::Long(expected.0)
        );
        assert!(!record.iter().any(|entry| entry.name == "AngerTime"));
        let saved = record.iter().find(|entry| entry.name == "angry_at");
        assert_eq!(
            saved.map(|entry| &entry.value),
            expected
                .1
                .map(|uuid| NbtSummaryValue::IntArray(uuid_words(uuid)))
                .as_ref()
        );
    }
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(datapack_config(), &candidate);
    for &id in ids {
        let uuid = world.stored_entity_for_test(DIMENSION, id).unwrap().uuid();
        let restored_id = restored
            .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
            .unwrap();
        assert_eq!(
            saved_anger(&restored, restored_id),
            (Some(expected.0), expected.1)
        );
    }
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let result = console_command(sender, receiver, world, 1, text);
    assert_eq!(console_command_answer(&result).0, Some(1), "{text}");
    result
}

#[test]
fn frozen_wolf_command_publishes_the_same_long_deadline_as_its_typed_save() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    install_floor(&mut world);
    command(&sender, &receiver, &mut world, "tp alice 4.5 65 4.5");
    command(&sender, &receiver, &mut world, "tick freeze");
    let (wolf, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let paired = command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:true}}"),
    );
    assert!(paired.events.iter().any(|event| matches!(event,WorldEvent::Send{packet:ClientboundPlayPacket::AddEntity(add),..} if add.id == wolf)));
    let time = world.game_time();
    let ticks = world
        .stored_entity_for_test(DIMENSION, wolf)
        .unwrap()
        .tick_count();
    let changed = command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "data merge entity {uuid} {{anger_end_time:1234567890123L,angry_at:{}}}",
            uuid_snbt(Uuid::from_u128(1))
        ),
    );
    assert_eq!(target_of(&world, wolf), Some(Uuid::from_u128(1)));
    let next = world.tick(receiver.drain());
    let values: Vec<_> = changed
        .events
        .iter()
        .chain(&next.events)
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(update),
                ..
            } if update.id == wolf => Some(&update.values),
            _ => None,
        })
        .flatten()
        .filter(|row| row.data_id == VANILLA_WOLF_ANGER_END_TIME_DATA_ID)
        .map(|row| &row.value)
        .collect();
    assert!(values.contains(&&EntityDataValueKind::Long(1_234_567_890_123)));
    assert_eq!(
        saved_anger(&world, wolf),
        (Some(1_234_567_890_123), Some(Uuid::from_u128(1)))
    );
    assert_saved(
        &mut world,
        &[wolf],
        (1_234_567_890_123, Some(Uuid::from_u128(1))),
    );
    assert_eq!(world.game_time(), time);
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, wolf)
            .unwrap()
            .tick_count(),
        ticks
    );
}
