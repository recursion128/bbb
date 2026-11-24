//! `PlayerPredicate` records and `looking_at` through real advancement triggers.

use super::*;
use crate::tests::using_item::{stack, Fixture, ALICE, HOTBAR_MENU_SLOT};
use serde_json::json;

#[test]
fn player_predicate_terms_and_the_spyglass_advancements() {
    let player = |terms: serde_json::Value| {
        json!([{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type_specific":
                terms.as_object().unwrap().clone().into_iter()
                    .chain([("type".to_string(), json!("minecraft:player"))])
                    .collect::<serde_json::Map<_, _>>()}}])
    };
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, terms) in [
        (
            "test:stats",
            json!({"stats":[{"type":"minecraft:custom","stat":"minecraft:jump","value":{"max":0}}]}),
        ),
        (
            "test:wrong_stats",
            json!({"stats":[{"type":"minecraft:custom","stat":"minecraft:jump","value":{"min":1}}]}),
        ),
        (
            "test:recipes",
            json!({"recipes":{"test:never_unlocked":false}}),
        ),
        (
            "test:wrong_recipes",
            json!({"recipes":{"test:never_unlocked":true}}),
        ),
        (
            "test:advancements",
            json!({"advancements":{"minecraft:story/mine_diamond":false}}),
        ),
        (
            "test:wrong_advancements",
            json!({"advancements":{"minecraft:story/mine_diamond":{"diamond":true}}}),
        ),
        ("test:input", json!({"input":{"sneak":false}})),
        ("test:wrong_input", json!({"input":{"sneak":true}})),
    ] {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&json!({
                    "criteria":{"spyglass":{"trigger":"minecraft:using_item",
                        "conditions":{"item":{"items":"minecraft:spyglass"},
                            "player":player(terms)}}}
                }))
                .unwrap(),
            },
        );
    }
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    let position = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    // Straight ahead of Alice's eye: yaw 0 looks along +z.
    fixture.command(&format!(
        "summon minecraft:parrot {} {} {} {{NoAI:1b}}",
        position.x,
        position.y + 1.62 - 0.45,
        position.z + 3.0
    ));
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:spyglass", 1));
    fixture.use_item(InteractionHand::MainHand);
    fixture.idle();
    let done = |id: &str| {
        fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), id)
    };
    for id in [
        "test:stats",
        "test:recipes",
        "test:advancements",
        "test:input",
        "minecraft:adventure/spyglass_at_parrot",
    ] {
        assert!(done(id), "{id}");
    }
    for id in [
        "test:wrong_stats",
        "test:wrong_recipes",
        "test:wrong_advancements",
        "test:wrong_input",
        "minecraft:adventure/spyglass_at_ghast",
        "minecraft:adventure/spyglass_at_dragon",
    ] {
        assert!(!done(id), "{id}");
    }
}

/// `PlayerPredicate.looking_at` (`PlayerPredicate.java:118-127`): another
/// player between the spyglass and the parrot is the nearest hit, so the
/// parrot advancement is not earned.
#[test]
fn another_player_blocks_the_spyglass_view() {
    let mut fixture = Fixture::with_config(GameType::Survival, datapack_config());
    let position = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    fixture.command(&format!(
        "summon minecraft:parrot {} {} {} {{NoAI:1b}}",
        position.x,
        position.y + 1.62 - 0.45,
        position.z + 3.0
    ));
    fixture.command(&format!(
        "tp bob {} {} {}",
        position.x,
        position.y,
        position.z + 1.5
    ));
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:spyglass", 1));
    fixture.use_item(InteractionHand::MainHand);
    fixture.idle();
    assert!(!fixture.world.player_advancement_is_done(
        ConnectionId(ALICE),
        "minecraft:adventure/spyglass_at_parrot"
    ));
}
