use super::*;

const RECIPE: &str = "minecraft:flint_and_steel";

fn recipe_world(bob_knows_recipe: bool) -> ServerWorld {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(startup_generation());
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    if bob_knows_recipe {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("recipe give bob {RECIPE}"),
        );
    }
    assert!(!world.player_knows_recipe_for_test(ConnectionId(1), RECIPE));
    assert_eq!(
        world.player_knows_recipe_for_test(ConnectionId(2), RECIPE),
        bob_knows_recipe
    );
    world
}

fn run_recipe_minecart(world: &mut ServerWorld) -> i32 {
    let id = spawn_command_minecart(world);
    world.edit_command_minecart_for_test(
        Java26BuiltinDimension::Overworld,
        id,
        &format!("execute as @a run recipe give @s {RECIPE}"),
        true,
    );
    world.queue_command_minecart_execution_for_test(Java26BuiltinDimension::Overworld, id);
    world.run_pending_command_minecart_executions_for_test(
        Java26BuiltinDimension::Overworld,
        &mut Vec::new(),
    );
    id
}

#[test]
fn recipe_forks_accumulate_all_minecart_successes() {
    let mut world = recipe_world(false);
    let id = run_recipe_minecart(&mut world);

    assert!(world.player_knows_recipe_for_test(ConnectionId(1), RECIPE));
    assert!(world.player_knows_recipe_for_test(ConnectionId(2), RECIPE));
    let data = command_data(&world, id);
    assert_eq!(data.success_count(), 2);
    assert_eq!(data.last_execution(), world.game_time());
    assert!(data.last_output().is_some_and(|output| output
        .text()
        .contains("commands.recipe.give.success.single")));
    assert_eq!(data.synched_last_output(), data.last_output().unwrap());
}

/// `execute as` forks, so bob's `ERROR_GIVE_FAILED` is swallowed
/// (ContextChain.java:68-73) and alice's grant stays the last output.
#[test]
fn a_forked_recipe_failure_leaves_the_last_success_as_minecart_output() {
    let mut world = recipe_world(true);
    let id = run_recipe_minecart(&mut world);

    assert!(world.player_knows_recipe_for_test(ConnectionId(1), RECIPE));
    assert!(world.player_knows_recipe_for_test(ConnectionId(2), RECIPE));
    let data = command_data(&world, id);
    assert_eq!(data.success_count(), 1);
    assert!(data.last_output().is_some_and(|output| output
        .text()
        .contains("commands.recipe.give.success.single")));
    assert_eq!(data.synched_last_output(), data.last_output().unwrap());
}
