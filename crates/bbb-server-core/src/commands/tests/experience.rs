//! `/experience`.

use super::*;

/// What `/xp query <name> <kind>` answers on `world`.
fn queried(world: &mut CommandTestWorld, name: &str, kind: &str) -> Option<i32> {
    ran(&format!("xp query {name} {kind}"), world).result
}

/// `ExperienceCommand.queryExperience` (ExperienceCommand.java:125-129) over
/// the two `Type.query` readers (`:183`, `:187`).
#[test]
fn experience_query_reports_the_level_or_the_points_of_the_current_bar() {
    // Level 2 needs `7 + 2 * 2` points (Player.java:1553-1558), so a bar at
    // 0.5 holds `Mth.floor(0.5 * 11)`.
    let players = [user("alice", 1)];
    let points = ran(
        "experience query alice points",
        world(&players).with_player_experience("alice", PlayerExperience::from_saved(0.5, 2, 0)),
    );
    assert_eq!(keys(&points), ["commands.experience.query.points"]);
    assert_eq!(
        arguments(&points, 0),
        vec![display_name("alice", 1), NbtSummaryValue::Int(5)]
    );
    assert_eq!(points.result, Some(5));
    // `:127` reports with `false`.
    assert!(!points.feedback[0].inform_admins);

    // `/xp` is the redirect `register` ends with (`:122`).
    let levels = ran(
        "xp query alice levels",
        world(&players).with_player_experience("alice", PlayerExperience::from_saved(0.5, 2, 0)),
    );
    assert_eq!(keys(&levels), ["commands.experience.query.levels"]);
    assert_eq!(
        arguments(&levels, 0),
        vec![display_name("alice", 1), NbtSummaryValue::Int(2)]
    );
    assert_eq!(levels.result, Some(2));

    // `EntityArgument.user()` (`:110`): one player, so `@a` is refused at
    // the parse (EntityArgument.java:110-113).
    let many = ran(
        "experience query @a points",
        world(&players).with_player_experience("alice", PlayerExperience::from_saved(0.5, 2, 0)),
    );
    assert_eq!(
        failure_messages(&many)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.player.toomany"]
    );
}

/// `ExperienceCommand.addExperience` (ExperienceCommand.java:131-147): one
/// `Type.add` per target, one line after them.
#[test]
fn experience_add_gives_every_target_and_reports_one_line() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut added = world(&players);
    let levels = ran("xp add @a 5 levels", &mut added);
    assert_eq!(queried(&mut added, "alice", "levels"), Some(5));
    assert_eq!(queried(&mut added, "bob", "levels"), Some(5));
    // `:143`: the amount and the target count, with `true`.
    assert_eq!(
        keys(&levels),
        ["commands.experience.add.levels.success.multiple"]
    );
    assert_eq!(
        arguments(&levels, 0),
        vec![NbtSummaryValue::Int(5), NbtSummaryValue::Int(2)]
    );
    assert!(levels.feedback[0].inform_admins);
    assert_eq!(levels.result, Some(2));

    // The bare `<amount>` is `Type.POINTS` (`:35-42`), the same as the
    // `points` literal (`:45-52`), and `IntegerArgumentType.integer()` (`:34`)
    // admits a negative amount.
    // Taking 3 points off an empty level-2 bar drops alice to level 1.
    let mut lowered =
        world(&players).with_player_experience("alice", PlayerExperience::from_saved(0.0, 2, 0));
    let points = ran("experience add alice -3", &mut lowered);
    assert_eq!(queried(&mut lowered, "alice", "levels"), Some(1));
    // `:139-141`: the amount and the one target's display name.
    assert_eq!(
        keys(&points),
        ["commands.experience.add.points.success.single"]
    );
    assert_eq!(
        arguments(&points, 0),
        vec![NbtSummaryValue::Int(-3), display_name("alice", 1)]
    );
    assert_eq!(points.result, Some(1));
    let mut raised = world(&players);
    ran("experience add alice 3 points", &mut raised);
    assert_eq!(queried(&mut raised, "alice", "points"), Some(3));
}

/// `ExperienceCommand.setExperience` (ExperienceCommand.java:149-173) over the
/// two `Type.set` predicates (`:177-182`, `:184-186`).
#[test]
fn experience_set_points_refuses_a_count_the_current_level_cannot_hold() {
    // A fresh player needs 7 points for level one (Player.java:1558), so 7 is
    // `a >= getXpNeededForNextLevel()` (`:177`) and 6 is not.
    let players = [user("alice", 1)];
    let mut set_world = world(&players);
    let refused = ran("xp set alice 7 points", &mut set_world);
    assert_eq!(
        failure_messages(&refused)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["commands.experience.set.points.invalid"]
    );
    assert_eq!(queried(&mut set_world, "alice", "points"), Some(0));

    let set = ran("xp set alice 6", &mut set_world);
    assert_eq!(queried(&mut set_world, "alice", "points"), Some(6));
    // `:165-167`, with `true`.
    assert_eq!(
        keys(&set),
        ["commands.experience.set.points.success.single"]
    );
    assert_eq!(
        arguments(&set, 0),
        vec![NbtSummaryValue::Int(6), display_name("alice", 1)]
    );
    assert!(set.feedback[0].inform_admins);
    assert_eq!(set.result, Some(1));

    // `IntegerArgumentType.integer(0)` (`:73`): a negative count never reaches
    // the predicate.
    let negative = ran("xp set alice -1", world(&players));
    assert_eq!(
        failure_messages(&negative)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.integer.low"]
    );
}

/// `setExperience` counts the targets the predicate admitted (`:154-158`) but
/// reports over the whole list (`:164-170`): one refusal among two targets is
/// still the `.multiple` line over both.
#[test]
fn experience_set_reports_over_every_target_when_at_least_one_was_set() {
    // Level 20 needs `37 + 5 * 5` points (Player.java:1557), so 10 fits bob's
    // bar and not alice's.
    let players = [user("alice", 1), user("bob", 2)];
    let mut mixed =
        world(&players).with_player_experience("bob", PlayerExperience::from_saved(0.0, 20, 0));
    let set = ran("xp set @a 10 points", &mut mixed);
    assert_eq!(queried(&mut mixed, "alice", "points"), Some(0));
    assert_eq!(queried(&mut mixed, "bob", "points"), Some(10));
    assert_eq!(
        keys(&set),
        ["commands.experience.set.points.success.multiple"]
    );
    assert_eq!(
        arguments(&set, 0),
        vec![NbtSummaryValue::Int(10), NbtSummaryValue::Int(2)]
    );
    assert_eq!(set.result, Some(2));

    // `Type.LEVELS.set` always answers true (`:184-186`), even for the level
    // the player already has: whether anything moves is
    // `setExperienceLevels`'s own question (ServerPlayer.java:520).
    let mut reset =
        world(&players).with_player_experience("bob", PlayerExperience::from_saved(0.0, 20, 0));
    let levels = ran("xp set @a 0 levels", &mut reset);
    assert_eq!(queried(&mut reset, "bob", "levels"), Some(0));
    assert_eq!(
        keys(&levels),
        ["commands.experience.set.levels.success.multiple"]
    );
}

/// `ExperienceCommand.register` gates both literals on `LEVEL_GAMEMASTERS`
/// (ExperienceCommand.java:28, `:122`).
#[test]
fn experience_and_its_alias_are_hidden_below_gamemaster() {
    for command in ["experience query alice levels", "xp query alice levels"] {
        let outcome = refused(command, CommandPermission::Moderator);
        assert_eq!(
            translate_key(failure_messages(&outcome)[0]).as_deref(),
            Some("command.unknown.command"),
            "{command}"
        );
    }
    let names = usable_command_tree(CommandPermission::GameMaster)
        .nodes
        .iter()
        .filter_map(|node| node.name.clone())
        .collect::<Vec<_>>();
    assert!(names.contains(&"experience".to_string()));
    assert!(names.contains(&"xp".to_string()));
}
