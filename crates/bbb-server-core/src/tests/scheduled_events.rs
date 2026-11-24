//! World apply and tick drain for `TimerQueue`.

use super::*;
use crate::commands::CommandScheduledCallback;

#[test]
fn replace_append_and_callback_order_survive_a_save_and_restart() {
    let root = std::env::temp_dir().join(format!("bbb-scheduled-events-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-scheduled-events");
    let paths = storage.paths().clone();
    let baseline = storage.load_candidate().unwrap();
    let config = operator_config(&[("op", 1)]);
    let mut world = ServerWorld::restore(config.clone(), &baseline).unwrap();
    world.schedule_event(
        "minecraft:foo".to_string(),
        4,
        CommandScheduledCallback::Function("minecraft:foo".to_string()),
        true,
    );
    world.schedule_event(
        "minecraft:foo".to_string(),
        8,
        CommandScheduledCallback::Function("minecraft:foo".to_string()),
        false,
    );
    assert_eq!(
        world.projected_scheduled_events_for_test().len(),
        2,
        "append keeps the earlier trigger"
    );

    world.schedule_event(
        "minecraft:foo".to_string(),
        12,
        CommandScheduledCallback::Function("minecraft:foo".to_string()),
        true,
    );
    let replaced = world.projected_scheduled_events_for_test();
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].trigger_time, 12);
    use crate::tests::function_tags::function_tag_entries;
    use bbb_server_datapack::function::{
        parse_function_source, FunctionElement, FunctionLibrary, FUNCTION_REGISTRY_ID,
    };
    let library = FunctionLibrary {
        functions: [
            ("test:b", "say MARK-B\nschedule clear test:skip"),
            ("test:a", "say MARK-A"),
            ("test:c", "say MARK-C"),
            ("test:skip", "say MARK-skip"),
        ]
        .into_iter()
        .map(|(id, source)| FunctionElement {
            id: id.into(),
            source_pack: "test".into(),
            source: parse_function_source(source).unwrap(),
        })
        .collect(),
        tags: Some(function_tag_entries(
            FUNCTION_REGISTRY_ID,
            &[("test:group", &["test:c"])],
        )),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    world.replace_function_library(&library);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let scheduled = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "schedule function test:b 30t",
    );
    assert!(system_chats(&scheduled)
        .iter()
        .any(|(_, text)| text.contains("commands.schedule.created.function")));
    let trigger = world
        .projected_scheduled_events_for_test()
        .iter()
        .find(|event| event.id == "test:b")
        .unwrap()
        .trigger_time;
    for (id, callback) in [
        (
            "test:a",
            CommandScheduledCallback::Function("test:a".into()),
        ),
        (
            "#test:group",
            CommandScheduledCallback::FunctionTag("test:group".into()),
        ),
        (
            "test:skip",
            CommandScheduledCallback::Function("test:skip".into()),
        ),
    ] {
        world.schedule_event(id.into(), trigger, callback, false);
    }
    let request = world
        .save_request(1_786_000_108_000, 205, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();
    assert!(paths
        .saved_data_dir()
        .join("minecraft")
        .join("scheduled_events.dat")
        .exists());
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert_eq!(candidate.scheduled_events, request.scheduled_events);
    let mut world = ServerWorld::restore(config, &candidate).unwrap();
    world.replace_function_library(&library);
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let mut markers = Vec::new();
    while world.game_time() <= trigger {
        let tick = world.tick(receiver.drain());
        for (_, text) in chat_lines(&world, &tick) {
            for marker in ["MARK-B", "MARK-A", "MARK-C", "MARK-skip"] {
                if text.contains(marker) {
                    assert_eq!(world.game_time(), trigger);
                    markers.push(marker);
                }
            }
        }
    }
    assert_eq!(markers, ["MARK-B", "MARK-A", "MARK-C"]);
    assert!(world.projected_scheduled_events_for_test().is_empty());
    let request = world
        .save_request(1_786_000_109_000, 206, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let paths = storage.paths().clone();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    assert!(storage
        .load_candidate()
        .unwrap()
        .scheduled_events
        .is_empty());
}

#[test]
fn duplicate_id_and_time_is_ignored() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    for _ in 0..2 {
        world.schedule_event(
            "minecraft:foo".to_string(),
            4,
            CommandScheduledCallback::Function("minecraft:foo".to_string()),
            false,
        );
    }
    assert_eq!(world.projected_scheduled_events_for_test().len(), 1);
}

#[test]
fn clear_removes_the_row() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.schedule_event(
        "minecraft:foo".to_string(),
        4,
        CommandScheduledCallback::Function("minecraft:foo".to_string()),
        true,
    );
    assert_eq!(
        world.clear_scheduled_events(&bbb_protocol::JavaString::from("minecraft:foo")),
        1
    );
    assert!(world.projected_scheduled_events_for_test().is_empty());
}
