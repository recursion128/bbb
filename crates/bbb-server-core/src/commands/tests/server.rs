//! `/datapack list`, `/debug start`/`stop`, `/reload`, `/seed`,
//! `/defaultgamemode`, `/difficulty`, `/gamerule`, `/save-*`,
//! `/setidletimeout`, `/setworldspawn`, `/stop`, `/version` and `/transfer`.

use bbb_protocol::packets::{SetDefaultSpawnPosition, Transfer};
use bbb_protocol::{component_literal_nbt, NbtSummaryValue};
use bbb_server_datapack::pack::ServerPackSource;

use crate::connection::ClientboundPlayPacket;

use super::*;
use crate::world::WorldEvent;

#[test]
fn datapack_list_reports_vanilla_enabled_and_no_available() {
    let run = ran("datapack list", CommandTestWorld::default());
    assert_eq!(
        keys(&run),
        [
            "commands.datapack.list.enabled.success",
            "commands.datapack.list.available.none",
        ]
    );
    assert_eq!(run.result, Some(1));
    assert!(!run.feedback[0].inform_admins);
    assert!(!run.feedback[1].inform_admins);
}

#[test]
fn datapack_list_enabled_returns_the_selected_count() {
    let run = ran("datapack list enabled", CommandTestWorld::default());
    assert_eq!(keys(&run), ["commands.datapack.list.enabled.success"]);
    assert_eq!(run.result, Some(1));
}

#[test]
fn datapack_list_available_reports_none_when_the_catalog_is_empty() {
    let run = ran("datapack list available", CommandTestWorld::default());
    assert_eq!(keys(&run), ["commands.datapack.list.available.none"]);
    assert_eq!(run.result, Some(0));
}

#[test]
fn datapack_list_available_names_an_unselected_pack() {
    let packs = [vanilla_pack(), world_pack("file/extra", &[])];
    let run = ran("datapack list available", packs_world(&packs, &packs[..1]));
    assert_eq!(keys(&run), ["commands.datapack.list.available.success"]);
    assert_eq!(run.result, Some(1));
}

/// `listAvailablePacks` drops an unselected pack whose requested features the
/// world does not have (DataPackCommand.java:241-243).
#[test]
fn datapack_list_available_hides_a_pack_the_feature_set_does_not_admit() {
    let packs = [
        vanilla_pack(),
        world_pack("file/extra", &["minecraft:trade_rebalance"]),
    ];
    let run = ran("datapack list available", packs_world(&packs, &packs[..1]));
    assert_eq!(keys(&run), ["commands.datapack.list.available.none"]);
    assert_eq!(run.result, Some(0));
}

#[test]
fn datapack_enable_announces_then_reloads_the_new_selection() {
    let packs = [vanilla_pack(), world_pack("file/extra", &[])];
    let run = ran(
        "datapack enable \"file/extra\"",
        packs_world(&packs, &packs[..1]),
    );
    assert_eq!(keys(&run), ["commands.datapack.modify.enable"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(2));
    assert_eq!(reloads(&run), [reload_of(&["vanilla", "file/extra"])]);
}

/// The bare leaf inserts at `getDefaultPosition` (`:107`): a `WORLD` pack is
/// TOP, so it appends, and the built-in `vanilla` pack is BOTTOM.
#[test]
fn datapack_enable_bare_uses_the_packs_default_position() {
    let packs = [vanilla_pack(), world_pack("file/extra", &[])];
    let run = ran("datapack enable vanilla", packs_world(&packs, &packs[1..]));
    assert_eq!(reloads(&run), [reload_of(&["vanilla", "file/extra"])]);
}

#[test]
fn datapack_enable_first_last_before_and_after_place_the_pack() {
    let packs = [
        vanilla_pack(),
        world_pack("file/a", &[]),
        world_pack("file/b", &[]),
    ];
    let selected = [packs[1].clone(), packs[2].clone()];
    for (command, order) in [
        ("first", vec!["vanilla", "file/a", "file/b"]),
        ("last", vec!["file/a", "file/b", "vanilla"]),
        ("after \"file/a\"", vec!["file/a", "vanilla", "file/b"]),
        ("before \"file/b\"", vec!["file/a", "vanilla", "file/b"]),
    ] {
        let command = format!("datapack enable vanilla {command}");
        let run = ran(&command, packs_world(&packs, &selected));
        assert_eq!(run.result, Some(3), "{command}");
        assert_eq!(reloads(&run), [reload_of(&order)], "{command}");
    }
}

/// `getPack(c, "existing", false)` runs inside the inserter (`:116`, `:127`),
/// which is after `name` was accepted and before the success line goes out.
#[test]
fn datapack_enable_after_an_unenabled_pack_refuses_without_announcing() {
    let packs = [
        vanilla_pack(),
        world_pack("file/a", &[]),
        world_pack("file/b", &[]),
    ];
    let run = ran(
        "datapack enable vanilla after \"file/b\"",
        packs_world(&packs, &packs[1..2]),
    );
    assert_eq!(keys(&run), ["commands.datapack.disable.failed"]);
    assert_eq!(run.result, None);
}

/// `disablePack` (DataPackCommand.java:222-229) drops the pack and reloads
/// what is left, naming it with `getChatLink(true)` (`:226`).
#[test]
fn datapack_disable_drops_the_pack_and_reloads_the_rest() {
    let packs = [vanilla_pack(), world_pack("file/extra", &[])];
    let run = ran(
        "datapack disable \"file/extra\"",
        packs_world(&packs, &packs),
    );
    assert_eq!(keys(&run), ["commands.datapack.modify.disable"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(1));
    assert_eq!(reloads(&run), [reload_of(&["vanilla"])]);
}

/// The `getPack` refusal ladder in its order (DataPackCommand.java:276-300).
#[test]
fn datapack_enable_and_disable_walk_the_get_pack_refusal_ladder() {
    let packs = [
        vanilla_pack(),
        world_pack("file/extra", &[]),
        CommandDatapack {
            source: ServerPackSource::Feature,
            ..world_pack("file/feature", &["minecraft:vanilla"])
        },
        world_pack(
            "file/future",
            &[
                "minecraft:redstone_experiments",
                "minecraft:trade_rebalance",
            ],
        ),
    ];
    let selected = [packs[0].clone(), packs[2].clone()];
    for (command, key) in [
        ("datapack enable missing", "commands.datapack.unknown"),
        ("datapack disable missing", "commands.datapack.unknown"),
        ("datapack enable vanilla", "commands.datapack.enable.failed"),
        (
            "datapack disable \"file/extra\"",
            "commands.datapack.disable.failed",
        ),
        (
            "datapack disable \"file/feature\"",
            "commands.datapack.disable.failed.feature",
        ),
        (
            "datapack enable \"file/future\"",
            "commands.datapack.enable.failed.no_flags",
        ),
    ] {
        let run = ran(command, packs_world(&packs, &selected));
        assert_eq!(keys(&run), [key], "{command}");
        assert_eq!(run.result, None, "{command}");
    }
}

/// `FeatureFlags.printMissingFlags` (FeatureFlags.java:22-26) names only the
/// requested flags the world is missing.
#[test]
fn datapack_enable_names_the_missing_feature_flags() {
    let packs = [
        vanilla_pack(),
        world_pack(
            "file/future",
            &["minecraft:trade_rebalance", "minecraft:vanilla"],
        ),
    ];
    let run = ran(
        "datapack enable \"file/future\"",
        packs_world(&packs, &packs[..1]),
    );
    assert_eq!(
        arguments(&run, 0),
        [
            component_literal_nbt("file/future"),
            component_literal_nbt("minecraft:trade_rebalance"),
        ]
    );
}

/// `UNSELECTED_PACKS` (DataPackCommand.java:82-95) drops the selected packs
/// and the ones the feature set does not admit; `SELECTED_PACKS` (`:79-81`) is
/// the selection. Both escape the id on the way out.
#[test]
fn datapack_enable_and_disable_offer_the_two_pack_lists() {
    let packs = [
        vanilla_pack(),
        world_pack("file/extra", &[]),
        world_pack("file/future", &["minecraft:trade_rebalance"]),
    ];
    let selected = [vanilla_pack()];
    let ask = |command: &str| {
        texts(&completions_for(
            command,
            CommandPermission::Owner,
            packs_world(&packs, &selected),
        ))
    };
    assert_eq!(ask("datapack enable "), vec!["\"file/extra\"".to_string()]);
    assert_eq!(ask("datapack disable "), vec!["vanilla".to_string()]);
    for anchor in ["after", "before"] {
        assert_eq!(
            ask(&format!("datapack enable \"file/extra\" {anchor} ")),
            vec!["vanilla".to_string()],
            "{anchor}"
        );
    }
}

fn vanilla_pack() -> CommandDatapack {
    CommandDatapack {
        id: "vanilla".to_string(),
        source: ServerPackSource::BuiltIn,
        description: CommandPackDescription::Vanilla,
        requested_features: Vec::new(),
    }
}

fn world_pack(id: &str, requested_features: &[&str]) -> CommandDatapack {
    CommandDatapack {
        id: id.to_string(),
        source: ServerPackSource::World,
        description: CommandPackDescription::Literal(format!("{id} pack").into()),
        requested_features: requested_features
            .iter()
            .map(|feature| (*feature).to_string())
            .collect(),
    }
}

fn packs_world(available: &[CommandDatapack], selected: &[CommandDatapack]) -> CommandTestWorld {
    CommandTestWorld::default().with_packs(available, selected)
}

fn reload_of(ids: &[&str]) -> Option<Vec<String>> {
    Some(ids.iter().map(|id| (*id).to_string()).collect())
}

/// The datapack selections the command asked the runtime to reload.
fn reloads(run: &CommandRun) -> Vec<Option<Vec<String>>> {
    run.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::ReloadRequested { selected } => Some(selected.clone()),
            _ => None,
        })
        .collect()
}

fn create_world() -> (std::path::PathBuf, CommandTestWorld) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "bbb-native-pack-create-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).unwrap();
    let mut world = CommandTestWorld::default().with_datapack_directory(&path);
    world.setup(r#"data merge storage test:text {word:"hi"}"#);
    (path, world)
}

#[test]
fn datapack_create_writes_the_pack_with_its_resolved_description() {
    let (path, mut world) = create_world();
    let run = ran(
        r#"datapack create my_pack {"nbt":"word","storage":"test:text","plain":true}"#,
        &mut world,
    );
    assert_eq!(keys(&run), ["commands.datapack.create.success"]);
    assert_eq!(run.result, Some(1));
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("my_pack/pack.mcmeta")).unwrap()).unwrap();
    assert!(metadata["pack"]["description"].to_string().contains("hi"));
    assert!(path.join("my_pack/data").is_dir());
    let again = ran(r#"datapack create my_pack "hi""#, &mut world);
    assert_eq!(keys(&again), ["commands.datapack.create.already_exists"]);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn datapack_create_reports_real_path_and_io_failures() {
    for (command, key) in [
        (
            r#"datapack create ".." "hi""#,
            "commands.datapack.create.invalid_name",
        ),
        (
            r#"datapack create con "hi""#,
            "commands.datapack.create.invalid_full_name",
        ),
        (
            r#"datapack create pack "hi""#,
            "commands.datapack.create.io_failure",
        ),
    ] {
        let run = ran_default(command);
        assert_eq!(keys(&run), [key]);
        assert_eq!(run.result, None);
    }
}

/// `Commands.hasPermission(Commands.LEVEL_OWNERS)` on the `create` literal
/// (DataPackCommand.java:151) leaves the arm unparseable for the gamemaster the
/// root itself admits, while `list` stays reachable.
#[test]
fn datapack_create_is_hidden_from_a_gamemaster() {
    let refusal = refused(
        r#"datapack create pack "hi""#,
        CommandPermission::GameMaster,
    );
    assert_eq!(keys(&refusal), ["command.unknown.argument", CURSOR_CONTEXT]);
    let run = ran_as(
        "datapack list",
        &source("alice", CommandPermission::GameMaster),
        CommandTestWorld::default(),
    );
    assert_eq!(
        keys(&run),
        [
            "commands.datapack.list.enabled.success",
            "commands.datapack.list.available.none",
        ]
    );
}

#[test]
fn datapack_is_hidden_from_a_moderator() {
    let refusal = refused("datapack list", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

#[test]
fn debug_start_arms_the_time_profiler() {
    let mut world = CommandTestWorld::default();
    let run = ran("debug start", &mut world);
    assert_eq!(keys(&run), ["commands.debug.started"]);
    assert_eq!(run.result, Some(0));
    assert!(world.world().debug_profiler_view().running);
    assert!(run.feedback[0].inform_admins);
}

#[test]
fn debug_start_refuses_when_the_profiler_is_already_running() {
    let run = ran(
        "debug start",
        CommandTestWorld::default().with_debug_started(),
    );
    assert_eq!(keys(&run), ["commands.debug.alreadyRunning"]);
}

#[test]
fn debug_stop_refuses_when_the_profiler_is_not_running() {
    let run = ran("debug stop", CommandTestWorld::default());
    assert_eq!(keys(&run), ["commands.debug.notRunning"]);
}

#[test]
fn debug_stop_reports_the_native_sampled_tick_count() {
    let mut world = CommandTestWorld::default().with_debug_sample(20);
    let run = ran("debug stop", &mut world);
    assert_eq!(keys(&run), ["commands.debug.stopped"]);
    assert!(run.result.is_some());
    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Int(20));
    assert!(!world.world().debug_profiler_view().running);
}

#[test]
fn debug_is_hidden_from_a_gamemaster() {
    let refusal = refused("debug start", CommandPermission::GameMaster);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

#[test]
fn reload_answers_before_the_reload_runs_and_asks_the_server_to_run_it() {
    let run = ran("reload", world(&[]));
    assert_eq!(keys(&run), ["commands.reload.success"]);
    assert_eq!(run.result, Some(0));
    assert_eq!(reloads(&run), [None]);
}

#[test]
fn a_reload_that_could_not_be_built_reports_the_vanilla_failure_key() {
    assert_eq!(
        translate_key(reload_failure().component()).as_deref(),
        Some("commands.reload.failure")
    );
}

#[test]
fn seed_reports_the_level_seed_as_copyable_text() {
    let run = ran("seed", CommandTestWorld::default().with_seed(-1234));
    assert_eq!(run.result, Some(-1234));
    assert_eq!(
        run.feedback[0].message.component(),
        &compound(vec![
            ("translate", text("commands.seed.success")),
            (
                "with",
                NbtSummaryValue::List(vec![compound(vec![
                    ("translate", text("chat.square_brackets")),
                    (
                        "with",
                        NbtSummaryValue::List(vec![compound(vec![
                            ("text", text("-1234")),
                            ("color", text("green")),
                            (
                                "click_event",
                                compound(vec![
                                    ("action", text("copy_to_clipboard")),
                                    ("value", text("-1234")),
                                ])
                            ),
                            (
                                "hover_event",
                                compound(vec![
                                    ("action", text("show_text")),
                                    (
                                        "value",
                                        compound(vec![("translate", text("chat.copy.click"))])
                                    ),
                                ])
                            ),
                            ("insertion", text("-1234")),
                        ])])
                    ),
                ])])
            ),
        ])
    );
}

#[test]
fn difficulty_queries_the_level_and_returns_its_id() {
    let run = ran(
        "difficulty",
        CommandTestWorld::default().with_difficulty(Difficulty::Hard),
    );
    assert_eq!(keys(&run), ["commands.difficulty.query"]);
    assert_eq!(
        arguments(&run, 0)[0],
        compound(vec![("translate", text("options.difficulty.hard"))])
    );
    assert_eq!(run.result, Some(3));
}

#[test]
fn difficulty_sets_the_level_and_refuses_the_one_already_in_force() {
    let mut peaceful = world(&[]);
    let run = ran("difficulty peaceful", &mut peaceful);
    assert_eq!(keys(&run), ["commands.difficulty.success"]);
    assert_eq!(run.result, Some(0));
    assert_eq!(peaceful.world().difficulty(), Difficulty::Peaceful);

    // The projection's default is `Difficulty.EASY`, so setting easy changes
    // nothing and reports the serialized name.
    let refused = ran("difficulty easy", world(&[]));
    assert_eq!(keys(&refused), ["commands.difficulty.failure"]);
    assert_eq!(arguments(&refused, 0), vec![text("easy")]);
}

#[test]
fn gamerule_queries_and_sets_through_both_spellings_of_the_rule() {
    let run = ran("gamerule random_tick_speed", world(&[]));
    assert_eq!(keys(&run), ["commands.gamerule.query"]);
    assert_eq!(
        arguments(&run, 0),
        vec![text("random_tick_speed"), text("3")]
    );
    assert_eq!(run.result, Some(3));

    let mut set = world(&[]);
    let qualified = ran("gamerule minecraft:random_tick_speed 7", &mut set);
    assert_eq!(keys(&qualified), ["commands.gamerule.set"]);
    assert_eq!(
        arguments(&qualified, 0),
        vec![text("random_tick_speed"), text("7")]
    );
    assert_eq!(run.result, Some(3));
    assert_eq!(
        set.world().game_rule("minecraft:random_tick_speed"),
        Some(GameRuleValue::Integer(7))
    );
}

#[test]
fn gamerule_reads_a_boolean_rule_and_reports_its_command_result() {
    let mut set = world(&[]);
    let run = ran("gamerule keep_inventory true", &mut set);
    assert_eq!(
        arguments(&run, 0),
        vec![text("keep_inventory"), text("true")]
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(
        set.world().game_rule("minecraft:keep_inventory"),
        Some(GameRuleValue::Boolean(true))
    );
}

#[test]
fn a_game_rule_value_below_its_minimum_is_refused_by_the_argument() {
    // The `value` node of an integer rule carries `{"min":0}`, which is where
    // the bound comes from rather than from a table written here.
    let run = ran("gamerule random_tick_speed -1", world(&[]));
    assert_eq!(keys(&run), ["argument.integer.low", CURSOR_CONTEXT]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::Int(0), NbtSummaryValue::Int(-1)]
    );
}

#[test]
fn a_boolean_argument_reports_the_word_it_could_not_read() {
    let run = ran("gamerule keep_inventory yes", world(&[]));
    assert_eq!(keys(&run), ["parsing.bool.invalid", CURSOR_CONTEXT]);
    assert_eq!(arguments(&run, 0), vec![text("yes")]);
}

#[test]
fn the_save_commands_report_the_save_they_asked_for() {
    let saved = ran("save-all", world(&[]));
    assert_eq!(
        keys(&saved),
        ["commands.save.saving", "commands.save.success"]
    );
    assert!(saved
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::SaveRequested { flush: false, .. })));

    let flushed = ran("save-all flush", world(&[]));
    assert!(flushed
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::SaveRequested { flush: true, .. })));

    let already_on = ran("save-on", world(&[]));
    assert_eq!(keys(&already_on), ["commands.save.alreadyOn"]);

    let mut saving = world(&[]);
    let off = ran("save-off", &mut saving);
    assert_eq!(keys(&off), ["commands.save.disabled"]);
    assert!(!saving.auto_save());

    let on = ran("save-on", &mut saving);
    assert_eq!(keys(&on), ["commands.save.enabled"]);
    assert!(saving.auto_save());
}

#[test]
fn stop_answers_before_it_halts_the_server() {
    let run = ran("stop", world(&[]));
    assert_eq!(keys(&run), ["commands.stop.stopping"]);
    assert!(run
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::StopRequested)));
    assert_eq!(run.result, Some(1));
}

#[test]
fn defaultgamemode_sets_the_world_default_and_reports_the_long_display_name() {
    // `DefaultGameModeCommands.setMode` (DefaultGameModeCommands.java:20-26)
    // writes the type and then reports `type.getLongDisplayName()`, which the
    // `GameType` constructor built as `Component.translatable("gameMode." +
    // name)` (GameType.java:38). The count it returns is
    // `enforceGameTypeForPlayers(getForcedGameType())` (:23), which is zero
    // whatever is online while `force-gamemode` is clear: `getForcedGameType`
    // is then the null branch (DedicatedServer.java:792-794) and
    // `enforceGameTypeForPlayers` returns before it reaches a player
    // (MinecraftServer.java:1390-1392).
    let mut defaulted = world(&[user("bob", 2)]);
    let run = ran("defaultgamemode creative", &mut defaulted);
    assert_eq!(keys(&run), ["commands.defaultgamemode.success"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Success]);
    assert_eq!(
        arguments(&run, 0),
        [compound(vec![("translate", text("gameMode.creative"))])]
    );
    assert_eq!(defaulted.default_game_type(), GameType::Creative);
    assert_eq!(run.result, Some(0));
}

/// With `force-gamemode` set, `/defaultgamemode` returns the number of sessions
/// it moved (`MinecraftServer.java:1394-1402`), and the type they are moved onto
/// is the one this very command wrote (`DefaultGameModeCommands.java:22-23`).
#[test]
fn defaultgamemode_counts_actual_sessions_and_changes_their_modes() {
    let users = [user("bob", 2), user("carol", 3), user("dave", 4)];
    let forced = || {
        world(&users)
            .with_force_game_mode(true)
            .with_player_game_type("carol", GameType::Creative)
            .with_player_game_type("dave", GameType::Spectator)
    };
    assert_eq!(ran("defaultgamemode survival", forced()).result, Some(2));
    assert_eq!(ran("defaultgamemode adventure", forced()).result, Some(3));
    assert_eq!(
        ran(
            "defaultgamemode survival",
            world(&users[..1]).with_force_game_mode(true)
        )
        .result,
        Some(0)
    );
    assert_eq!(
        ran(
            "defaultgamemode survival",
            world(&[]).with_force_game_mode(true)
        )
        .result,
        Some(0)
    );
    assert_eq!(
        ran("defaultgamemode adventure", world(&users)).result,
        Some(0)
    );
}

#[test]
fn a_gamemode_word_that_names_no_game_type_is_refused_at_the_end_of_the_word() {
    // `GameModeArgument.parse` (GameModeArgument.java:27-35) reads the word
    // with `readUnquotedString` (:28) and throws (:31) without putting the
    // cursor back, and `createWithContext`
    // (DynamicCommandExceptionType.java:18-20) copies that advanced cursor onto
    // the exception as a plain int (CommandSyntaxException.java:22-28). The
    // word here ends the input, so the context quotes the ten characters before
    // the end and the marker follows with nothing underlined between them.
    let outcome = refused("defaultgamemode bogus", CommandPermission::Owner);
    let messages = failure_messages(&outcome);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        translate_key(messages[0]).as_deref(),
        Some("argument.gamemode.invalid")
    );
    assert_eq!(
        messages[1],
        &compound(vec![
            ("text", text("")),
            ("color", text("gray")),
            (
                "click_event",
                compound(vec![
                    ("action", text("suggest_command")),
                    ("command", text("/defaultgamemode bogus")),
                ])
            ),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    text("..."),
                    text("mode bogus"),
                    compound(vec![
                        ("translate", text("command.context.here")),
                        ("italic", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                ])
            ),
        ])
    );
}

#[test]
fn a_game_rule_behind_an_experiment_has_no_node_at_all() {
    // `max_minecart_speed` is registered with
    // `FeatureFlagSet.of(FeatureFlags.MINECART_IMPROVEMENTS)`
    // (GameRules.java:55-57), and `GameRuleCommand.register` visits
    // `new GameRules(context.enabledFeatures())` (GameRuleCommand.java:17), so
    // a server running the default feature set publishes neither spelling of
    // it. Both are in the derived table, which was probed from a tree built
    // with every feature on.
    let tree = usable_command_tree(CommandPermission::Owner);
    let published = tree
        .nodes
        .iter()
        .filter_map(|node| node.name.as_deref())
        .filter(|name| name.contains("max_minecart_speed"))
        .collect::<Vec<_>>();
    assert!(published.is_empty(), "{published:?}");
    // A rule the feature set keeps is still there, under both spellings.
    let kept = tree
        .nodes
        .iter()
        .filter_map(|node| node.name.as_deref())
        .filter(|name| name.ends_with("keep_inventory"))
        .collect::<Vec<_>>();
    assert_eq!(kept, ["keep_inventory", "minecraft:keep_inventory"]);
    // Absent from the tree means absent from parsing, so the command fails
    // where an unknown literal fails rather than where an unknown rule would.
    let run = refused("gamerule max_minecart_speed", CommandPermission::Owner);
    assert_eq!(
        failure_messages(&run)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["command.unknown.argument"]
    );
    // And from completion, which walks the dispatcher rather than the
    // published tree.
    let completions = completions_for(
        "gamerule max_min",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert!(texts(&completions).is_empty(), "{completions:?}");
}

/// `VersionCommand` (VersionCommand.java:19-24 over `dumpVersion`, `:28-39`):
/// ten lines, every one of them on the `sendSystemMessage` channel rather than
/// on `sendSuccess`, so `send_command_feedback` cannot silence them.
#[test]
fn version_reports_ten_ungated_system_lines() {
    let run = ran("version", CommandTestWorld::default());
    assert_eq!(run.result, Some(1));
    assert_eq!(
        keys(&run),
        [
            "commands.version.header",
            "commands.version.id",
            "commands.version.name",
            "commands.version.data",
            "commands.version.series",
            "commands.version.protocol",
            "commands.version.build_time",
            "commands.version.pack.resource",
            "commands.version.pack.data",
            "commands.version.stable.yes",
        ]
    );
    assert!(kinds(&run)
        .iter()
        .all(|kind| *kind == CommandFeedbackKind::System));
}

/// The nine values are `SharedConstants.getCurrentVersion()`'s, which on a
/// released build is `version.json` -- the same document `bbb_protocol`'s
/// version constants are checked against.
#[test]
fn the_version_lines_carry_the_numbers_version_json_declares() {
    let run = ran("version", CommandTestWorld::default());
    assert_eq!(arguments(&run, 1), vec![text(bbb_protocol::MC_VERSION)]);
    assert_eq!(arguments(&run, 2), vec![text(bbb_protocol::MC_VERSION)]);
    assert_eq!(
        arguments(&run, 3),
        vec![NbtSummaryValue::Int(bbb_protocol::MC_DATA_VERSION)]
    );
    assert_eq!(
        arguments(&run, 4),
        vec![text(bbb_protocol::MC_DATA_VERSION_SERIES)]
    );
    // `"0x" + Integer.toHexString(protocolVersion)` (`:34`).
    assert_eq!(
        arguments(&run, 5),
        vec![
            NbtSummaryValue::Int(bbb_protocol::PROTOCOL_VERSION),
            text("0x307"),
        ]
    );
    assert_eq!(
        arguments(&run, 6),
        vec![text(&bbb_facts::mc_build_time_java_date_string())]
    );
    // `PackFormat.toString()` (PackFormat.java:97-101).
    assert_eq!(arguments(&run, 7), vec![text("84.0")]);
    assert_eq!(arguments(&run, 8), vec![text("101.1")]);
}

/// `TransferCommand.transfer` (TransferCommand.java:55-71): the shortest node
/// fills the port in (`:28`), the hostname travels as a raw `String` argument
/// and the port as a raw `int`, both of which `TranslatableContents` writes as
/// primitives (TranslatableContents.java:31-32) rather than as components.
#[test]
fn transfer_defaults_the_port_and_carries_both_values_as_primitives() {
    let players = [user("alice", 1)];
    let alice = source("alice", CommandPermission::Owner);
    let run = ran_as("transfer example.invalid", &alice, world(&players));
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.transfer.success.single"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            display_name("alice", 1),
            NbtSummaryValue::string("example.invalid"),
            NbtSummaryValue::Int(25565),
        ]
    );
    assert_eq!(
        run.packets(),
        [&ClientboundPlayPacket::Transfer(Transfer {
            host: "example.invalid".into(),
            port: 25565,
        })]
    );
}

/// The `<hostname>` is `StringArgumentType.string()` (TransferCommand.java:27),
/// which is `StringReader.readString` (StringArgumentType.java:42) -- so a
/// quoted host carries its spaces and the quotes are not part of the value.
#[test]
fn transfer_reads_a_quoted_hostname_and_the_port_range_the_node_publishes() {
    let players = [user("alice", 1), user("bob", 2)];
    let alice = source("alice", CommandPermission::Owner);
    let run = ran_as("transfer \"my host\" 1 @a", &alice, world(&players));
    assert_eq!(run.result, Some(2));
    assert_eq!(keys(&run), ["commands.transfer.success.multiple"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(2),
            NbtSummaryValue::string("my host"),
            NbtSummaryValue::Int(1),
        ]
    );
    assert_eq!(run.packets().len(), 2);

    // `IntegerArgumentType.integer(1, 65535)` (`:30`) is what refuses a port
    // outside the band, through `argument.integer.big`.
    let run = ran_as("transfer host 65536", &alice, world(&players));
    assert_eq!(keys(&run), ["argument.integer.big", CURSOR_CONTEXT]);
}

#[test]
fn transfer_from_a_raw_storage_macro_keeps_feedback_units_and_netty_wire_bytes() {
    use bbb_protocol::codec::Decoder;
    use bbb_protocol::packets::encode_play_clientbound_transfer;
    use bbb_server_datapack::function::{parse_function_source, FunctionElement, FunctionLibrary};

    let players = [user("alice", 1)];
    let alice = source("alice", CommandPermission::Owner);
    let mut world = CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            function_permission_level: CommandPermission::Owner,
            ..Default::default()
        },
        &players,
    )
    .with_functions(&FunctionLibrary {
        functions: vec![FunctionElement {
            id: "test:transfer".to_owned(),
            source_pack: "test".to_owned(),
            source: parse_function_source(r#"$transfer "$(host)" 25565 @a"#).unwrap(),
        }],
        tags: None,
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    });
    let host = [0xd800_u16, 0xfffd];
    let setup = "data merge storage test:transfer {host:\""
        .encode_utf16()
        .chain(host)
        .chain("\"}".encode_utf16())
        .collect::<Vec<_>>();
    assert_eq!(world.run_units(&setup, &alice).result, Some(1));

    let run = ran_as(
        "function test:transfer with storage test:transfer",
        &alice,
        &mut world,
    );
    let transfers = run
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Transfer(packet),
            } => Some((connection_id.0, packet)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(transfers.len(), 1);
    let (connection_id, packet) = transfers[0];
    assert_eq!(connection_id, 1);
    let (_, encoded) = encode_play_clientbound_transfer(packet).unwrap();
    let mut decoder = Decoder::new(&encoded);
    let host_bytes = usize::try_from(decoder.read_var_i32().unwrap()).unwrap();
    assert_eq!(
        decoder.read_exact(host_bytes, "transfer host").unwrap(),
        &[0x3f, 0xfd],
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 25565);
    assert!(decoder.is_empty());

    // Function bodies suppress their feedback. Check the raw primitive through
    // the same world's direct command entry after the macro wire assertion.
    let feedback_command = "transfer \""
        .encode_utf16()
        .chain(host)
        .chain("\" 25565 @a".encode_utf16())
        .collect::<Vec<_>>();
    let feedback_run = world.run_units(&feedback_command, &alice);
    assert_eq!(feedback_run.result, Some(1));
    let feedback = feedback_run
        .feedback
        .iter()
        .position(|feedback| {
            translate_key(body(feedback)).as_deref() == Some("commands.transfer.success.single")
        })
        .expect("the direct command reports transfer success");
    assert_eq!(
        arguments(&feedback_run, feedback),
        vec![
            display_name("alice", 1),
            NbtSummaryValue::string_from_utf16_slice(&host),
            NbtSummaryValue::Int(25565),
        ],
    );
}

/// `SetWorldSpawnCommand.setSpawn` (SetWorldSpawnCommand.java:33-52): always
/// `sendSuccess` + return 1, even when the stored `RespawnData` already matches.
#[test]
fn setworldspawn_writes_the_source_level_and_containing_position() {
    let mut spawned = CommandTestWorld::default()
        .with_position([10.7, 70.2, -3.1])
        .with_level(level_key(Java26BuiltinDimension::Nether));
    let run = ran("setworldspawn", &mut spawned);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.setworldspawn.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(10),
            NbtSummaryValue::Int(70),
            NbtSummaryValue::Int(-4),
            NbtSummaryValue::Float(0.0f32.to_bits()),
            NbtSummaryValue::Float(0.0f32.to_bits()),
            NbtSummaryValue::string("minecraft:the_nether"),
        ]
    );
    assert_eq!(
        spawned.world_spawn(),
        SetDefaultSpawnPosition {
            dimension: "minecraft:the_nether".to_string(),
            pos: BlockPos {
                x: 10,
                y: 70,
                z: -4
            },
            yaw: 0.0,
            pitch: 0.0,
        }
    );
}

/// `pos` uses `getSpawnablePos`; `pos rotation` feeds `RespawnData.of`
/// (`LevelData.java:54-55`): wrap yaw, clamp pitch.
#[test]
fn setworldspawn_pos_rotation_wraps_yaw_and_clamps_pitch() {
    let mut spawned = CommandTestWorld::default();
    let run = ran("setworldspawn 1 2 3 370 100", &mut spawned);
    assert_eq!(run.result, Some(1));
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(1),
            NbtSummaryValue::Int(2),
            NbtSummaryValue::Int(3),
            NbtSummaryValue::Float(10.0f32.to_bits()),
            NbtSummaryValue::Float(90.0f32.to_bits()),
            NbtSummaryValue::string("minecraft:overworld"),
        ]
    );
    assert_eq!(
        spawned.world_spawn(),
        SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos { x: 1, y: 2, z: 3 },
            yaw: 10.0,
            pitch: 90.0,
        }
    );
}

/// `BlockPosArgument.getSpawnablePos` (BlockPosArgument.java:52-58) is the only
/// bounds gate; the bare form never consults it (`SetWorldSpawnCommand.java:21`).
#[test]
fn setworldspawn_pos_refuses_outside_spawnable_bounds_but_the_bare_form_does_not() {
    let mut out_world = CommandTestWorld::default();
    let before = out_world.world_spawn();
    let out = ran("setworldspawn 30000000 0 0", &mut out_world);
    assert_eq!(keys(&out), ["argument.pos.outofbounds"]);
    assert_eq!(out_world.world_spawn(), before);
    assert_eq!(out.result, None);

    let high = ran("setworldspawn 0 20000000 0", CommandTestWorld::default());
    assert_eq!(keys(&high), ["argument.pos.outofbounds"]);

    let mut bare_world =
        CommandTestWorld::default().with_position([30_000_000.0, 20_000_000.0, 0.0]);
    let bare = ran("setworldspawn", &mut bare_world);
    assert_eq!(bare.result, Some(1));
    assert_eq!(
        bare_world.world_spawn().pos,
        BlockPos {
            x: 30_000_000,
            y: 20_000_000,
            z: 0
        }
    );
}

/// `SetWorldSpawnCommand.register` (SetWorldSpawnCommand.java:19) is
/// `LEVEL_GAMEMASTERS`; below it the root is gone.
#[test]
fn setworldspawn_is_hidden_from_a_moderator() {
    let refusal = refused("setworldspawn", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

/// `setIdleTimeout` (SetPlayerIdleTimeoutCommand.java:21-30) writes the timeout
/// unconditionally and picks its wording from the value it wrote.
#[test]
fn setidletimeout_writes_the_minutes_and_reports_them() {
    let mut timed = CommandTestWorld::default();
    let run = ran("setidletimeout 5", &mut timed);
    assert_eq!(keys(&run), ["commands.setidletimeout.success"]);
    assert_eq!(arguments(&run, 0), [NbtSummaryValue::Int(5)]);
    assert_eq!(timed.idle_timeout(), 5);
    assert_eq!(run.result, Some(5));
    assert!(run.feedback[0].inform_admins);
}

/// Zero is the floor `IntegerArgumentType.integer(0)` (`:15`) leaves reachable,
/// and it is the only value the disabled wording is used for.
#[test]
fn setidletimeout_zero_words_itself_as_disabled_and_still_writes() {
    let mut timed = CommandTestWorld::default();
    ran("setidletimeout 5", &mut timed);
    let run = ran("setidletimeout 0", &mut timed);
    assert_eq!(keys(&run), ["commands.setidletimeout.success.disabled"]);
    assert_eq!(arguments(&run, 0), []);
    assert_eq!(timed.idle_timeout(), 0);
    assert_eq!(run.result, Some(0));
}

/// The floor is the argument's, not the body's: `-1` never reaches
/// `setIdleTimeout`, so nothing is written.
#[test]
fn setidletimeout_refuses_a_negative_minute_count_before_it_writes() {
    let run = ran("setidletimeout -1", CommandTestWorld::default());
    assert_eq!(
        keys(&run),
        ["argument.integer.low", CURSOR_CONTEXT],
        "`IntegerArgumentType.integer(0)` (SetPlayerIdleTimeoutCommand.java:15)"
    );
    assert_eq!(run.result, None);
}

/// `Commands.LEVEL_ADMINS` (`:13`); below it the root is gone.
#[test]
fn setidletimeout_is_hidden_from_a_gamemaster() {
    let refusal = refused("setidletimeout 5", CommandPermission::GameMaster);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

/// The literal on its own is not executable -- `executes` sits on the argument
/// (`:16-17`) -- and with nothing left to read that is the dispatcher's
/// unknown-command refusal rather than an unknown-argument one.
#[test]
fn setidletimeout_alone_is_not_executable() {
    let run = ran("setidletimeout", CommandTestWorld::default());
    assert_eq!(keys(&run), ["command.unknown.command", CURSOR_CONTEXT]);
}
