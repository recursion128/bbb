//! `/bossbar` against the server boss event manager.

use bbb_facts::boss_bar_styles::{BossBarColor, BossBarOverlay};
use bbb_protocol::{NbtSummaryValue, RichTextComponent};

use super::*;

fn boss_world(ids: &[&str], players: &[NameAndId]) -> CommandTestWorld {
    world(players).with_bossbars(ids)
}

fn bar(world: &CommandTestWorld, id: &str) -> Option<CommandBossBarState> {
    world
        .world()
        .boss_bars()
        .into_iter()
        .find(|event| event.id == id)
}

fn boss_name(id: &str, name: &str, color: &str) -> NbtSummaryValue {
    compound(vec![
        ("translate", text("chat.square_brackets")),
        ("with", NbtSummaryValue::List(vec![text(name)])),
        ("color", text(color)),
        (
            "hover_event",
            compound(vec![("action", text("show_text")), ("value", text(id))]),
        ),
        ("insertion", text(id)),
    ])
}

#[test]
fn add_uses_the_default_namespace_and_custom_event_defaults() {
    let mut fresh = CommandTestWorld::default();
    let run = ran("bossbar add launch \"Launch\"", &mut fresh);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.bossbar.create.success"]);
    assert_eq!(
        arguments(&run, 0),
        [boss_name("minecraft:launch", "Launch", "white")]
    );
    assert!(run.feedback[0].inform_admins);
    let created = bar(&fresh, "minecraft:launch").expect("created");
    assert_eq!(created.name, RichTextComponent::plain("Launch"));
    assert_eq!(
        (
            created.value,
            created.max,
            created.color,
            created.overlay,
            created.visible
        ),
        (0, 100, BossBarColor::White, BossBarOverlay::Progress, true)
    );

    let events = ["minecraft:launch"];
    let duplicate = ran("bossbar add launch \"Again\"", boss_world(&events, &[]));
    assert_eq!(keys(&duplicate), ["commands.bossbar.create.failed"]);
    assert_eq!(arguments(&duplicate, 0), [text("minecraft:launch")]);
}

#[test]
fn list_and_remove_report_the_post_operation_counts() {
    let events = ["minecraft:first", "example:second"];
    let listed = ran("bossbar list", boss_world(&events, &[]));
    assert_eq!(listed.result, Some(2));
    assert_eq!(keys(&listed), ["commands.bossbar.list.bars.some"]);
    let args = arguments(&listed, 0);
    assert_eq!(args[0], NbtSummaryValue::Int(2));
    let NbtSummaryValue::Compound(root) = &args[1] else {
        panic!("formatted bossbar list");
    };
    let NbtSummaryValue::List(items) = &root
        .iter()
        .find(|entry| entry.name == "extra")
        .unwrap()
        .value
    else {
        panic!("bossbar items");
    };
    assert_eq!(items.len(), 3);
    for id in events {
        assert!(items.contains(&boss_name(id, "Old name", "white")));
    }
    assert!(!listed.feedback[0].inform_admins);

    let mut removing = boss_world(&events, &[]);
    let removed = ran("bossbar remove first", &mut removing);
    assert_eq!(removed.result, Some(1));
    assert_eq!(keys(&removed), ["commands.bossbar.remove.success"]);
    assert_eq!(bar(&removing, "minecraft:first"), None);
    assert!(removed.feedback[0].inform_admins);

    let empty = ran("bossbar list", CommandTestWorld::default());
    assert_eq!(empty.result, Some(0));
    assert_eq!(keys(&empty), ["commands.bossbar.list.bars.none"]);
    assert!(!empty.feedback[0].inform_admins);
}

#[test]
fn scalar_sets_emit_exact_mutations_results_and_feedback() {
    let events = ["minecraft:test"];
    type BossBarCase<'a> = (
        &'a str,
        fn(&CommandBossBarState) -> bool,
        i32,
        &'a str,
        Vec<NbtSummaryValue>,
    );
    let cases: [BossBarCase<'_>; 4] = [
        (
            "bossbar set test name \"New name\"",
            |bar| bar.name == RichTextComponent::plain("New name"),
            0,
            "commands.bossbar.set.name.success",
            vec![boss_name("minecraft:test", "New name", "white")],
        ),
        (
            "bossbar set test value 25",
            |bar| bar.value == 25,
            25,
            "commands.bossbar.set.value.success",
            vec![
                boss_name("minecraft:test", "Old name", "white"),
                NbtSummaryValue::Int(25),
            ],
        ),
        (
            "bossbar set test max 250",
            |bar| bar.max == 250,
            250,
            "commands.bossbar.set.max.success",
            vec![
                boss_name("minecraft:test", "Old name", "white"),
                NbtSummaryValue::Int(250),
            ],
        ),
        (
            "bossbar set test visible false",
            |bar| !bar.visible,
            0,
            "commands.bossbar.set.visible.success.hidden",
            vec![boss_name("minecraft:test", "Old name", "white")],
        ),
    ];
    for (command, changed, result, key, expected_arguments) in cases {
        let mut fixture = boss_world(&events, &[]);
        let run = ran(command, &mut fixture);
        assert_eq!(run.result, Some(result), "{command}");
        assert!(
            changed(&bar(&fixture, "minecraft:test").unwrap()),
            "{command}"
        );
        assert_eq!(keys(&run), [key], "{command}");
        assert_eq!(arguments(&run, 0), expected_arguments, "{command}");
        assert!(run.feedback[0].inform_admins, "{command}");
    }

    for (command, key) in [
        (
            "bossbar set test name \"Old name\"",
            "commands.bossbar.set.name.unchanged",
        ),
        (
            "bossbar set test color white",
            "commands.bossbar.set.color.unchanged",
        ),
        (
            "bossbar set test style progress",
            "commands.bossbar.set.style.unchanged",
        ),
        (
            "bossbar set test value 10",
            "commands.bossbar.set.value.unchanged",
        ),
        (
            "bossbar set test max 100",
            "commands.bossbar.set.max.unchanged",
        ),
        (
            "bossbar set test visible true",
            "commands.bossbar.set.visibility.unchanged.visible",
        ),
    ] {
        let run = ran(command, boss_world(&events, &[]));
        assert_eq!(run.result, None, "{command}");
        assert_eq!(keys(&run), [key], "{command}");
        assert!(arguments(&run, 0).is_empty(), "{command}");
    }
}

#[test]
fn every_color_uses_custom_boss_event_display_formatting() {
    for (literal, color, formatting) in [
        ("pink", BossBarColor::Pink, "red"),
        ("blue", BossBarColor::Blue, "blue"),
        ("red", BossBarColor::Red, "dark_red"),
        ("green", BossBarColor::Green, "green"),
        ("yellow", BossBarColor::Yellow, "yellow"),
        ("purple", BossBarColor::Purple, "dark_blue"),
        ("white", BossBarColor::White, "white"),
    ] {
        let mut fixture = boss_world(&["example:test"], &[]);
        if color == BossBarColor::White {
            fixture.setup("bossbar set example:test color pink");
        }
        let run = ran(
            &format!("bossbar set example:test color {literal}"),
            &mut fixture,
        );
        assert_eq!(bar(&fixture, "example:test").unwrap().color, color);
        assert_eq!(
            arguments(&run, 0),
            [boss_name("example:test", "Old name", formatting)]
        );
    }
}

#[test]
fn every_style_literal_maps_to_its_wire_overlay() {
    for (literal, overlay) in [
        ("notched_6", BossBarOverlay::Notched6),
        ("notched_10", BossBarOverlay::Notched10),
        ("notched_12", BossBarOverlay::Notched12),
        ("notched_20", BossBarOverlay::Notched20),
    ] {
        let events = ["minecraft:test"];
        let mut fixture = boss_world(&events, &[]);
        let run = ran(&format!("bossbar set test style {literal}"), &mut fixture);
        assert_eq!(run.result, Some(0));
        assert_eq!(keys(&run), ["commands.bossbar.set.style.success"]);
        assert_eq!(
            arguments(&run, 0),
            [boss_name("minecraft:test", "Old name", "white")]
        );
        assert_eq!(bar(&fixture, "minecraft:test").unwrap().overlay, overlay);
    }
}

#[test]
fn players_compare_persistent_membership_but_get_reads_online_members() {
    let players = [user("alice", 1), user("bob", 2)];
    let events = ["minecraft:test"];
    let make_world = || {
        let mut fixture = boss_world(&events, &players);
        fixture.setup("bossbar set test players alice");
        fixture
    };

    let unchanged = ran("bossbar set test players alice", make_world());
    assert_eq!(unchanged.result, None);
    assert_eq!(keys(&unchanged), ["commands.bossbar.set.players.unchanged"]);

    let mut replacing = make_world();
    let replaced = ran("bossbar set test players @a", &mut replacing);
    assert_eq!(replaced.result, Some(2));
    assert_eq!(
        keys(&replaced),
        ["commands.bossbar.set.players.success.some"]
    );
    assert_eq!(
        bar(&replacing, "minecraft:test").unwrap().players,
        [Uuid::from_u128(1), Uuid::from_u128(2)]
    );
    assert!(replaced.feedback[0].inform_admins);
    assert_eq!(
        arguments(&replaced, 0),
        [
            boss_name("minecraft:test", "Old name", "white"),
            NbtSummaryValue::Int(2),
            format_list(vec![display_name("alice", 1), display_name("bob", 2)]),
        ]
    );

    let queried = ran("bossbar get test players", make_world());
    assert_eq!(queried.result, Some(1));
    assert_eq!(keys(&queried), ["commands.bossbar.get.players.some"]);
    assert_eq!(
        arguments(&queried, 0),
        [
            boss_name("minecraft:test", "Old name", "white"),
            NbtSummaryValue::Int(1),
            display_name("alice", 1),
        ]
    );

    let mut clearing = make_world();
    let cleared = ran("bossbar set test players", &mut clearing);
    assert_eq!(cleared.result, Some(0));
    assert_eq!(
        keys(&cleared),
        ["commands.bossbar.set.players.success.none"]
    );
    assert_eq!(
        arguments(&cleared, 0),
        [boss_name("minecraft:test", "Old name", "white")]
    );
    assert!(bar(&clearing, "minecraft:test").unwrap().players.is_empty());
}

#[test]
fn get_returns_the_stored_values_and_visibility() {
    let events = ["minecraft:test"];
    for (command, result, key, expected_arguments) in [
        (
            "bossbar get test value",
            23,
            "commands.bossbar.get.value",
            vec![
                boss_name("minecraft:test", "Old name", "white"),
                NbtSummaryValue::Int(23),
            ],
        ),
        (
            "bossbar get test max",
            240,
            "commands.bossbar.get.max",
            vec![
                boss_name("minecraft:test", "Old name", "white"),
                NbtSummaryValue::Int(240),
            ],
        ),
        (
            "bossbar get test visible",
            0,
            "commands.bossbar.get.visible.hidden",
            vec![boss_name("minecraft:test", "Old name", "white")],
        ),
        (
            "bossbar get test players",
            0,
            "commands.bossbar.get.players.none",
            vec![boss_name("minecraft:test", "Old name", "white")],
        ),
    ] {
        let mut fixture = boss_world(&events, &[]);
        fixture.setup("bossbar set test value 23");
        fixture.setup("bossbar set test max 240");
        fixture.setup("bossbar set test visible false");
        let run = ran(command, &mut fixture);
        assert_eq!(run.result, Some(result), "{command}");
        assert_eq!(keys(&run), [key], "{command}");
        assert_eq!(arguments(&run, 0), expected_arguments, "{command}");
        assert!(run.feedback[0].inform_admins, "{command}");
    }
}

#[test]
fn unknown_events_fail_before_any_mutation() {
    for command in [
        "bossbar remove absent",
        "bossbar set absent value 1",
        "bossbar get absent max",
    ] {
        let run = ran(command, CommandTestWorld::default());
        assert_eq!(run.result, None, "{command}");
        assert_eq!(keys(&run), ["commands.bossbar.unknown"], "{command}");
        assert_eq!(arguments(&run, 0), [text("minecraft:absent")], "{command}");
    }
}

#[test]
fn bossbar_is_gamemaster_only_and_publishes_the_full_generated_subtree() {
    let refusal = refused("bossbar list", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
    assert_eq!(
        ran("bossbar list", CommandTestWorld::default()).result,
        Some(0)
    );

    let root = vanilla_command_lookup(&["bossbar"]).expect("the generated root contains bossbar");
    assert_eq!(
        catalog::effective_permission(root),
        CommandPermission::GameMaster
    );
    let mut pending = vec![root];
    while let Some(parent) = pending.pop() {
        let generated = catalog::node(parent)
            .children()
            .map(|child| child as u16)
            .collect::<Vec<_>>();
        let published = catalog::implemented_children(parent).collect::<Vec<_>>();
        assert_eq!(
            published,
            generated,
            "cut below {:?}",
            catalog::canonical_path(parent)
        );
        pending.extend(generated);
    }
}

#[test]
fn bossbar_ids_are_suggested_only_where_vanilla_asks_the_server() {
    let events = ["minecraft:first", "example:second"];
    let world = boss_world(&events, &[]);
    assert_eq!(
        texts(&completions_for(
            "bossbar remove ",
            CommandPermission::Owner,
            &world
        )),
        ["example:second", "minecraft:first"]
    );
    assert_eq!(
        texts(&completions_for(
            "bossbar get fir",
            CommandPermission::Owner,
            &world
        )),
        ["minecraft:first"]
    );
    assert_eq!(
        texts(&completions_for(
            "bossbar set example:s",
            CommandPermission::Owner,
            &world,
        )),
        ["example:second"]
    );
    assert!(texts(&completions_for(
        "bossbar get minecraft:first",
        CommandPermission::Owner,
        &world,
    ))
    .is_empty());
    assert!(texts(&completions_for(
        "bossbar add ",
        CommandPermission::Owner,
        &world,
    ))
    .is_empty());

    // `wrapStores` attaches the same `SUGGEST_BOSS_BAR`
    // (`ExecuteCommand.java:300`, `BossBarCommands.java:61-63`).
    assert_eq!(
        texts(&completions_for(
            "execute store result bossbar exam",
            CommandPermission::Owner,
            &world,
        )),
        ["example:second"]
    );
    assert_eq!(
        texts(&completions_for(
            "execute store success bossbar ",
            CommandPermission::Owner,
            &world,
        )),
        ["example:second", "minecraft:first"]
    );

    for path in [
        &["bossbar", "remove", "id"][..],
        &["bossbar", "set", "id"][..],
        &["bossbar", "get", "id"][..],
        &["execute", "store", "result", "bossbar", "id"][..],
        &["execute", "store", "success", "bossbar", "id"][..],
    ] {
        let id = vanilla_command_lookup(path)
            .expect("the generated bossbar subtree contains the id node");
        assert_eq!(catalog::node(id).suggestions, Some("minecraft:ask_server"));
    }
}
