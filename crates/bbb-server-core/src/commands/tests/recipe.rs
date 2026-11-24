//! `/recipe` grammar and visible feedback against actual recipe books.
use super::*;
use crate::world::scoreboard::Scoreboard;
use bbb_protocol::chat_formattings::ChatFormatting;

fn recipe_ids() -> Vec<String> {
    vec!["minecraft:bread".into(), "minecraft:stone_pickaxe".into()]
}
fn recipe_world(players: &[NameAndId], recipes: &[String]) -> CommandTestWorld {
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    std::sync::Arc::make_mut(&mut startup.recipes)
        .recipes
        .retain(|recipe| recipes.contains(&recipe.id));
    CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(std::sync::Arc::new(startup)),
            ..Default::default()
        },
        players,
    )
}

#[test]
fn recipe_is_gamemaster_only_and_keeps_vanilla_root_order() {
    let players = [user("alice", 1)];
    let recipes = recipe_ids();
    let refusal = ran_as(
        "recipe give alice minecraft:bread",
        &source("alice", CommandPermission::Moderator),
        recipe_world(&players, &recipes),
    );
    assert_eq!(
        keys(&refusal),
        ["command.unknown.command", "(cursor context)"]
    );

    assert_eq!(catalog::IMPLEMENTED_COMMANDS.len(), 87);
    let root = vanilla_command_lookup(&["recipe"]).expect("generated recipe root");
    assert_eq!(
        catalog::effective_permission(root),
        CommandPermission::GameMaster
    );
    assert_eq!(
        catalog::implemented_children(root)
            .map(|child| catalog::node(child).name)
            .collect::<Vec<_>>(),
        ["give", "take"]
    );
    let at = catalog::IMPLEMENTED_COMMANDS
        .iter()
        .position(|name| *name == "recipe")
        .expect("recipe is implemented");
    assert_eq!(
        &catalog::IMPLEMENTED_COMMANDS[at - 1..=at + 1],
        &["reload", "recipe", "fetchprofile"]
    );
    assert_eq!(
        ran("help", recipe_world(&players, &recipes)).result,
        Some(87)
    );
}

#[test]
fn resource_parser_and_suggestions_use_the_live_recipe_catalog() {
    let players = [user("alice", 1)];
    let recipes = recipe_ids();

    let all = completions_for(
        "recipe give alice ",
        CommandPermission::GameMaster,
        recipe_world(&players, &recipes),
    );
    assert_eq!(
        texts(&all),
        ["*", "minecraft:bread", "minecraft:stone_pickaxe"]
    );
    assert_eq!(
        texts(&completions_for(
            "recipe give alice br",
            CommandPermission::GameMaster,
            recipe_world(&players, &recipes),
        )),
        ["minecraft:bread"]
    );

    let recipe_leaf = vanilla_command_lookup(&["recipe", "give", "targets", "recipe"])
        .expect("generated resource-key leaf");
    assert_eq!(
        argument_template(catalog::node(recipe_leaf)),
        CommandArgumentTemplate::Registry {
            registry: catalog::RECIPE_REGISTRY.to_string()
        }
    );
}

#[test]
fn unknown_recipe_is_checked_after_target_resolution() {
    let players = [user("alice", 1)];
    let recipes = recipe_ids();
    let missing = ran(
        "recipe give alice minecraft:missing",
        recipe_world(&players, &recipes),
    );
    assert_eq!(keys(&missing), ["recipe.notFound"]);
    assert_eq!(
        failure_messages(&missing)
            .first()
            .and_then(|message| translate_key(message))
            .as_deref(),
        Some("recipe.notFound")
    );

    let both_invalid = ran(
        "recipe give nobody minecraft:missing",
        recipe_world(&players, &recipes),
    );
    assert_eq!(keys(&both_invalid), ["argument.entity.notfound.player"]);
}

#[test]
fn single_feedback_uses_the_team_formatted_display_name() {
    let players = [user("alice", 1)];
    let recipes = recipe_ids();
    let mut board = Scoreboard::default();
    assert!(board.add_team("red"));
    {
        let team = board.team_mut("red").expect("red team");
        team.player_prefix = Some(text("[R] "));
        team.player_suffix = Some(text("!"));
        team.color = ChatFormatting::Red;
    }
    board
        .add_player_to_team("alice", "red")
        .expect("red team exists");
    let run = ran(
        "recipe give alice minecraft:bread",
        recipe_world(&players, &recipes).with_scoreboard(&board),
    );
    let NbtSummaryValue::Compound(decorated) = display_name("alice", 1) else {
        panic!("player display is a compound")
    };
    let mut expected = vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: text(""),
        },
        NbtSummaryEntry {
            name: "color".into(),
            value: text("red"),
        },
        NbtSummaryEntry {
            name: "extra".into(),
            value: NbtSummaryValue::List(vec![text("[R] "), text("alice"), text("!")]),
        },
    ];
    expected.extend(decorated.into_iter().filter(|field| field.name != "text"));
    assert_eq!(
        arguments(&run, 0),
        [NbtSummaryValue::Int(1), NbtSummaryValue::Compound(expected)]
    );
}

#[test]
fn recipe_feedback_counts_the_named_inputs_and_refuses_an_unchanged_book() {
    let players = [user("alice", 1), user("bob", 2)];
    let recipes = recipe_ids();
    let mut fixture = recipe_world(&players, &recipes);
    let single = ran("recipe give alice bread", &mut fixture);
    assert_eq!(single.result, Some(1));
    assert_eq!(keys(&single), ["commands.recipe.give.success.single"]);
    assert_eq!(
        arguments(&single, 0),
        [NbtSummaryValue::Int(1), display_name("alice", 1)]
    );
    let duplicate = ran("recipe give alice bread", &mut fixture);
    assert_eq!(duplicate.result, None);
    assert_eq!(keys(&duplicate), ["commands.recipe.give.failed"]);
    fixture.setup("recipe give @a *");
    let multiple = ran("recipe take @a *", &mut fixture);
    assert_eq!(multiple.result, Some(4));
    assert_eq!(keys(&multiple), ["commands.recipe.take.success.multiple"]);
    assert_eq!(
        arguments(&multiple, 0),
        [NbtSummaryValue::Int(2), NbtSummaryValue::Int(2)]
    );
    let unchanged = ran("recipe take @a *", &mut fixture);
    assert_eq!(unchanged.result, None);
    assert_eq!(keys(&unchanged), ["commands.recipe.take.failed"]);
}
