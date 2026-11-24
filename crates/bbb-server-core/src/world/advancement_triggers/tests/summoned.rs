use super::*;

#[test]
fn summoned_entity_checks_entity_and_player_predicates_and_rewards_once() {
    let advancement = |entity: Value, player: Value, reward| {
        json!({"criteria":{"done":{"trigger":"minecraft:summoned_entity",
            "conditions":{"entity":entity,"player":player}}},
            "rewards":{"experience":reward}})
    };
    let sheep = json!({"type":"minecraft:sheep", "team":"flock", "nbt":"{Color:14b}",
        "components":{"minecraft:sheep/color":"red"}});
    let player = json!({"type":"minecraft:player","flags":{"is_on_ground":true}});
    let mut world = world(&[
        (
            "test:any",
            json!({"criteria":{"done":{"trigger":"minecraft:summoned_entity"}},"rewards":{"experience":1}}),
        ),
        ("test:object", advancement(sheep.clone(), player.clone(), 2)),
        (
            "test:list",
            advancement(
                json!([this_entity_properties(sheep)]),
                json!([this_entity_properties(player)]),
                3,
            ),
        ),
        (
            "test:zombie",
            advancement(json!({"type":"minecraft:zombie"}), json!({}), 7),
        ),
    ]);
    let (id, uuid) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    command(
        &mut world,
        &format!("data merge entity {uuid} {{Color:14b}}"),
    );
    command(&mut world, "team add flock");
    command(&mut world, &format!("team join flock {uuid}"));
    let dimension = world.levels[CONNECTION].dimension;
    world.levels[CONNECTION].set_on_ground(false);
    world.trigger_summoned_entity(CONNECTION, dimension, id, &mut Vec::new());
    assert_eq!(xp(&world), 1);
    world.levels[CONNECTION].set_on_ground(true);
    world.trigger_summoned_entity(CONNECTION, dimension, id, &mut Vec::new());
    assert!(world.player_advancement_is_done(CONNECTION, "test:object"));
    assert!(world.player_advancement_is_done(CONNECTION, "test:list"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:zombie"));
    assert_eq!(xp(&world), 6);
    world.trigger_summoned_entity(CONNECTION, dimension, id, &mut Vec::new());
    assert_eq!(xp(&world), 6);
}
