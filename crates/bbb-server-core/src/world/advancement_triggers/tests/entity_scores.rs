use super::*;

#[test]
fn entity_context_scores_on_a_real_player_kill_use_profile_names_and_publish_progress() {
    let named_score = json!({
        "condition":"minecraft:value_check",
        "value":{"type":"minecraft:score",
            "target":{"type":"minecraft:fixed","name":"global"},"score":"points"},
        "range":9
    });
    let criterion = |scores: Value, reward| {
        let mut definition = killed(
            "minecraft:player_killed_entity",
            json!({
                "player":[
                    {"condition":"minecraft:entity_scores","entity":"this","scores":{"points":4}},
                    named_score.clone()
                ],
                "entity":[
                    this_entity_properties(json!({"type":"minecraft:player"})),
                    {"condition":"minecraft:entity_scores","entity":"this","scores":scores},
                    named_score.clone()
                ]
            }),
        );
        definition["rewards"] = json!({"experience":reward});
        definition
    };
    let rejected = [
        "test:wrong_score",
        "test:missing_objective",
        "test:missing_score",
    ];
    let mut world = kill_world(&[
        ("test:scored", criterion(json!({"points":17}), 3)),
        (rejected[0], criterion(json!({"points":18}), 5)),
        (rejected[1], criterion(json!({"missing":0}), 7)),
        (rejected[2], criterion(json!({"unassigned":0}), 11)),
    ]);
    for input in [
        "scoreboard objectives add points dummy",
        "scoreboard objectives add unassigned dummy",
        "scoreboard players set inventory_player points 4",
        "scoreboard players set victim_player points 17",
        "scoreboard players set global points 9",
    ] {
        command(&mut world, input);
    }
    let events = command(
        &mut world,
        "damage victim_player 40 minecraft:generic by inventory_player",
    );
    assert_eq!(world.player_health_for_test(VICTIM), 0.0);
    assert!(world.player_advancement_is_done(CONNECTION, "test:scored"));
    for id in rejected {
        assert!(!world.player_advancement_is_done(CONNECTION, id));
    }
    assert!(!world.player_advancement_is_done(VICTIM, "test:scored"));
    assert_eq!(xp(&world), 3);
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
            if *connection_id == CONNECTION && update.progress.iter().any(|progress|
                progress.id == "test:scored" && progress.criteria.iter().any(|criterion|
                    criterion.name == "killed" && criterion.obtained_epoch_millis.is_some()))
    )));
    assert!(!events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
            if *connection_id == CONNECTION && update.progress.iter().any(|progress|
                rejected.contains(&progress.id.as_str()) && progress.criteria.iter().any(|criterion|
                    criterion.obtained_epoch_millis.is_some()))
    )));
    let saved = world.player_advancements_to_data(CONNECTION);
    assert!(saved.progress["test:scored"]
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
