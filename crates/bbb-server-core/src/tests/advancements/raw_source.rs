use super::*;
use bbb_protocol::JavaString;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

#[test]
fn raw_pack_criteria_reach_tick_predicates_commands_and_saved_progress() {
    let root = std::env::temp_dir().join(format!("bbb-raw-advancement-{}", Uuid::new_v4()));
    let pack = root.join("datapacks/raw");
    let definitions = pack.join("data/test/advancement");
    std::fs::create_dir_all(&definitions).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        br#"{"pack":{"description":"raw criteria","min_format":[101,1],"max_format":[101,1]}}"#,
    )
    .unwrap();
    let mut source = r#"{
        "display":{"icon":{"id":"minecraft:stone","components":{"minecraft:custom_data":{"\ud801":"\ud800"}}},
            "title":{"text":"\ud800"},"description":"raw criteria","announce_to_chat":false},
        "criteria":{"\ud800":{"trigger":"tick"},"\ud801":{"trigger":"impossible"},
            "\ufffd":{"trigger":"tick"},"\ud83d\udcda":{"trigger":"tick"}},
        "requirements":[["\ud800"],["\ud801"],["\ufffd"],["\ud83d\udcda"]],"ignored":[
    "#
    .to_string();
    source.push_str(&"null,".repeat(4097));
    source.push_str(&"[".repeat(80));
    source.push('0');
    source.push_str(&"]".repeat(80));
    source.push_str("]}");
    std::fs::write(definitions.join("raw.json"), source).unwrap();
    std::fs::write(
        definitions.join("observed.json"),
        br#"{"criteria":{"seen":{"trigger":"tick","conditions":{"player":{"type_specific":{
            "type":"minecraft:player","advancements":{"test:raw":{
                "\ud800":true,"\ud801":false,"\ufffd":true,"\ud83d\udcda":true}}}}}}}}"#,
    )
    .unwrap();
    let startup = Arc::new(
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: Some(root.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
        })
        .unwrap(),
    );
    assert!(
        startup.advancements.advancement("test:raw").is_some(),
        "{:?}",
        startup.advancements.warnings
    );
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(Arc::clone(&startup));
    let mut world = ServerWorld::new(config.clone()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    for _ in 0..2 {
        world.tick(vec![]);
    }
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:raw"));
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:observed"));
    let suggestions = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::CommandSuggestion(bbb_protocol::packets::CommandSuggestionRequest {
            id: 1,
            command: "advancement grant alice only test:raw ".into(),
        }),
    );
    let suggested = suggestions
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::CommandSuggestions(update),
                ..
            } => Some(update),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        suggested
            .suggestions
            .iter()
            .map(|suggestion| suggestion.text.units().to_vec())
            .collect::<std::collections::BTreeSet<_>>(),
        [
            vec![0xd800],
            vec![0xd801],
            vec![0xfffd],
            vec![0xd83d, 0xdcda]
        ]
        .into_iter()
        .collect()
    );
    let names = |data: &Java26PlayerAdvancements| {
        data.progress["test:raw"]
            .criteria()
            .iter()
            .filter(|(_, progress)| progress.is_done())
            .map(|(name, _)| name.units().to_vec())
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(
        names(&world.player_advancements_to_data(ConnectionId(1))),
        [
            [0xd800].to_vec(),
            [0xfffd].to_vec(),
            [0xd83d, 0xdcda].to_vec()
        ]
        .into_iter()
        .collect()
    );
    let granted = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "advancement grant alice only test:raw",
    );
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:raw"));
    let updates = update_advancements(&granted, 1);
    let update = updates
        .iter()
        .find(|update| update.added.iter().any(|added| added.id == "test:raw"))
        .expect("completing the root publishes its display and progress");
    let (id, payload) =
        bbb_protocol::packets::encode_play_clientbound_update_advancements(update).unwrap();
    let bbb_protocol::packets::PlayClientbound::UpdateAdvancements(decoded) =
        bbb_protocol::packets::decode_play_clientbound(id, &payload).unwrap()
    else {
        panic!("the world update must reach the advancement decoder");
    };
    let added = decoded
        .added
        .iter()
        .find(|added| added.id == "test:raw")
        .unwrap();
    assert_eq!(
        added.display,
        update
            .added
            .iter()
            .find(|added| added.id == "test:raw")
            .unwrap()
            .display
    );
    assert_eq!(
        bbb_server_datapack::language::component_plain_text(
            added.display.as_ref().unwrap().title.component()
        ),
        [0xd800]
    );
    assert_eq!(
        added.requirements,
        vec![
            vec![JavaString::from("?")],
            vec!["?".into()],
            vec!["�".into()],
            vec!["📚".into()]
        ]
    );
    let progress = decoded
        .progress
        .iter()
        .find(|progress| progress.id == "test:raw")
        .unwrap();
    let mut wire_names = progress
        .criteria
        .iter()
        .map(|criterion| criterion.name.units().to_vec())
        .collect::<Vec<_>>();
    wire_names.sort();
    assert_eq!(
        wire_names,
        vec![vec![63], vec![63], vec![0xd83d, 0xdcda], vec![0xfffd]]
    );
    let saved = world.player_advancements_to_data(ConnectionId(1));
    assert_eq!(names(&saved).len(), 4);
    let path = root.join("player.json");
    std::fs::write(&path, saved.to_file_bytes()).unwrap();
    let restored = Java26PlayerAdvancements::parse_bytes(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(names(&restored), names(&saved));
    let mut restarted = ServerWorld::new(config).unwrap();
    restarted.seed_restored_player_advancements(profile("alice", 1).uuid, restored);
    let (sender, receiver) = mailbox();
    join_loaded_ticks(&sender, &receiver, &mut restarted, 1, "alice", 1);
    assert!(restarted.player_advancement_is_done(ConnectionId(1), "test:raw"));
    assert_eq!(
        names(&restarted.player_advancements_to_data(ConnectionId(1))),
        names(&saved)
    );
    std::fs::remove_dir_all(root).unwrap();
}
