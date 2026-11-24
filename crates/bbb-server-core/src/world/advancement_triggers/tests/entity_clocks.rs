use super::*;

#[test]
fn entity_context_clocks_on_a_real_kill_read_the_named_world_clock() {
    let time =
        |clock, value| json!({"condition":"minecraft:time_check", "clock":clock, "value":value});
    let periodic = |clock, value| json!({"condition":"minecraft:time_check", "clock":clock, "period":100, "value":value});
    let criterion = |player, entity, reward| {
        let mut definition = killed(
            "minecraft:player_killed_entity",
            json!({
                "player":[player],
                "entity":[this_entity_properties(json!({"type":"minecraft:player"})), entity]
            }),
        );
        definition["rewards"] = json!({"experience":reward});
        definition
    };
    let rejected = [
        "test:wrong_player_time",
        "test:wrong_period",
        "test:wrong_clock",
    ];
    let mut world = kill_world(&[
        (
            "test:clocked",
            criterion(
                time("minecraft:the_end", 107),
                periodic("minecraft:the_end", 7),
                3,
            ),
        ),
        (
            rejected[0],
            criterion(
                time("minecraft:the_end", 108),
                periodic("minecraft:the_end", 7),
                5,
            ),
        ),
        (
            rejected[1],
            criterion(
                time("minecraft:the_end", 107),
                periodic("minecraft:the_end", 8),
                7,
            ),
        ),
        (
            rejected[2],
            criterion(
                time("minecraft:the_end", 107),
                periodic("minecraft:overworld", 7),
                11,
            ),
        ),
    ]);
    for (input, feedback) in [
        ("time of minecraft:the_end pause", "commands.time.pause"),
        ("time of minecraft:overworld pause", "commands.time.pause"),
        (
            "time of minecraft:the_end set 107",
            "commands.time.set.absolute",
        ),
        (
            "time of minecraft:overworld set 6000",
            "commands.time.set.absolute",
        ),
    ] {
        let events = command(&mut world, input);
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::SystemChat(chat) }
                if chat.content.text().contains(feedback)
        )), "{input}: {events:?}");
    }
    assert_eq!(world.clock_total_ticks("minecraft:the_end"), Some(107));
    assert_eq!(world.clock_total_ticks("minecraft:overworld"), Some(6000));
    let events = command(
        &mut world,
        "damage victim_player 40 minecraft:generic by inventory_player",
    );
    assert_eq!(world.player_health_for_test(VICTIM), 0.0);
    assert!(world.player_advancement_is_done(CONNECTION, "test:clocked"));
    assert!(!world.player_advancement_is_done(VICTIM, "test:clocked"));
    for id in rejected {
        assert!(!world.player_advancement_is_done(CONNECTION, id));
    }
    assert_eq!(xp(&world), 3);
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
            if update.progress.iter().any(|progress| progress.id == "test:clocked"
                && progress.criteria.iter().any(|criterion|
                    criterion.name == "killed" && criterion.obtained_epoch_millis.is_some()))
    )));
    let saved = world.player_advancements_to_data(CONNECTION);
    assert!(saved.progress["test:clocked"]
        .criteria()
        .iter()
        .any(|(name, progress)| name == "killed" && progress.is_done()));
    for id in rejected {
        assert!(saved.progress.get(id).is_none_or(|progress| progress
            .criteria()
            .iter()
            .all(|(_, criterion)| !criterion.is_done())));
    }
}
