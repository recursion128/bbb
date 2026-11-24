//! `/gamemode`.

use super::*;

fn game_type_of(world: &CommandTestWorld, name: &str) -> GameType {
    world
        .world()
        .command_players()
        .find(|player| player.name == name)
        .expect("fixture player")
        .game_type
}

/// `GameModeCommand.logGamemodeChange` (GameModeCommand.java:36-47) over the
/// source's own entity: `commands.gamemode.success.self` and nothing else.
#[test]
fn gamemode_over_the_source_reports_the_self_key_and_moves_one_session() {
    let alice = user("alice", 1);
    let mut moved = world(std::slice::from_ref(&alice));
    let run = ran("gamemode creative", &mut moved);
    assert_eq!(keys(&run), ["commands.gamemode.success.self"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Success]);
    assert_eq!(
        arguments(&run, 0),
        [compound(vec![("translate", text("gameMode.creative"))])]
    );
    assert_eq!(game_type_of(&moved, "alice"), GameType::Creative);
    assert_eq!(run.result, Some(1));
}

/// The other branch (`:41-45`): the target is told with `gameMode.changed`
/// before the source is answered with `commands.gamemode.success.other`, and
/// the notice is gated on `SEND_COMMAND_FEEDBACK` where the answer is not.
#[test]
fn gamemode_over_a_target_notifies_them_and_names_them_to_the_source() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let players = [alice, bob];
    let mut moved = world(&players);
    let run = ran("gamemode adventure bob", &mut moved);
    assert_eq!(keys(&run), ["commands.gamemode.success.other"]);
    assert_eq!(
        arguments(&run, 0),
        [
            display_name("bob", 2),
            compound(vec![("translate", text("gameMode.adventure"))]),
        ]
    );
    let notice = component(compound(vec![
        ("translate", text("gameMode.changed")),
        (
            "with",
            NbtSummaryValue::List(vec![compound(vec![(
                "translate",
                text("gameMode.adventure"),
            )])]),
        ),
    ]));
    assert_eq!(game_type_of(&moved, "bob"), GameType::Adventure);
    assert_eq!(run.messages_to(2), [&notice]);

    // `:41` reads the rule itself rather than going through `acceptsSuccess`,
    // so clearing it drops the notice and leaves the answer alone.
    let mut quiet = world(&players).with_rules(&[(
        "minecraft:send_command_feedback",
        GameRuleValue::Boolean(false),
    )]);
    let run = ran("gamemode adventure bob", &mut quiet);
    assert_eq!(keys(&run), ["commands.gamemode.success.other"]);
    assert_eq!(game_type_of(&quiet, "bob"), GameType::Adventure);
    assert!(run.messages_to(2).is_empty());
}

/// `GameModeCommand.setMode` (GameModeCommand.java:49-59) counts what
/// `setGameMode` answered, which is
/// `ServerPlayerGameMode.changeGameModeForPlayer`'s early return for a session
/// already in the mode (ServerPlayerGameMode.java:56-58) -- and a session that
/// did not move is not told about it either.
#[test]
fn gamemode_counts_and_reports_only_the_sessions_that_moved() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let players = [alice, bob];
    // Bob is already in creative, so `@a` moves alice alone -- and alice is the
    // source, which is what picks the `self` key over the `other` one
    // (GameModeCommand.java:38-39).
    let mut moved = world(&players).with_player_game_type("bob", GameType::Creative);
    let run = ran("gamemode creative @a", &mut moved);
    assert_eq!(keys(&run), ["commands.gamemode.success.self"]);
    assert_eq!(run.result, Some(1));
    assert_eq!(game_type_of(&moved, "alice"), GameType::Creative);
    // Everyone already in the mode moves nobody, and the run reports nothing
    // at all.
    let run = ran(
        "gamemode survival alice",
        world(&players).with_player_game_type("bob", GameType::Creative),
    );
    assert_eq!(keys(&run), Vec::<&str>::new());
    assert_eq!(run.result, Some(0));
}

/// `c.getSource().getPlayerOrException()` (GameModeCommand.java:27), whose
/// `ERROR_NOT_PLAYER` (CommandSourceStack.java:405) is
/// `permissions.requires.player`; the targeted overload instead reports
/// `EntityArgument.getPlayers`'s `NO_PLAYERS_FOUND` (EntityArgument.java:92).
#[test]
fn gamemode_refuses_a_source_with_no_player_and_a_selector_with_no_match() {
    let bob = user("bob", 2);
    let offstage = source("console", CommandPermission::Owner);
    let run = ran_as(
        "gamemode creative",
        &offstage,
        world(std::slice::from_ref(&bob)),
    );
    assert_eq!(keys(&run), ["permissions.requires.player"]);
    let run = ran_as(
        "gamemode creative nobody",
        &offstage,
        world(std::slice::from_ref(&bob)),
    );
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);
}
