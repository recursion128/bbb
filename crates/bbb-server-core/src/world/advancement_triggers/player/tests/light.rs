use super::*;

#[test]
fn location_light_follows_daylight_and_rewards_each_match_once() {
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    startup.advancements = AdvancementSet {
        advancements: [
            ("test:daylight", json!({"min": 12})),
            ("test:night", json!({"max": 4})),
        ]
        .into_iter()
        .map(|(id, light)| {
            let location = json!({"light": {"light": light}});
            let player = if id == "test:night" {
                json!([{"condition": "minecraft:location_check", "predicate": location}])
            } else {
                json!({"location": location})
            };
            AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: parse_advancement(&json!({
                    "criteria": {"located": {
                        "trigger": "minecraft:location",
                        "conditions": {"player": player}
                    }},
                    "rewards": {"experience": 7}
                }))
                .unwrap(),
            }
        })
        .collect(),
        warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "explorer", 1);
    let player = ConnectionId(1);
    let mut position = world.levels[player].position();
    position.y = 100.0;
    world.levels[player].set_position(position);
    crate::tests::console_command(&sender, &receiver, &mut world, 2, "time set 6000");
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(world.player_advancement_is_done(player, "test:daylight"));
    assert!(!world.player_advancement_is_done(player, "test:night"));
    assert_eq!(world.levels[player].experience.total(), 7);
    crate::tests::console_command(&sender, &receiver, &mut world, 3, "time set 18000");
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(world.player_advancement_is_done(player, "test:night"));
    assert_eq!(world.levels[player].experience.total(), 14);
    crate::tests::console_command(&sender, &receiver, &mut world, 4, "time set 6000");
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert_eq!(world.levels[player].experience.total(), 14);
}
