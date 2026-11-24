//! Tab completion.

use super::*;

#[test]
fn completions_offer_the_children_of_the_node_the_cursor_sits_after() {
    let ask = |command: &str, permission| {
        let completions = completions_for(command, permission, CommandTestWorld::default());
        (completions.start, completions.length, texts(&completions))
    };

    let armor_slots = vec![
        "armor.body".to_string(),
        "armor.chest".to_string(),
        "armor.feet".to_string(),
        "armor.head".to_string(),
        "armor.legs".to_string(),
    ];
    assert_eq!(
        ask("item replace entity @s armor.", CommandPermission::Owner),
        (23, 6, armor_slots.clone())
    );
    let mut armor_ranges = vec!["armor.*".to_string()];
    armor_ranges.extend(armor_slots);
    assert_eq!(
        ask(
            "execute if items entity @s armor.",
            CommandPermission::Owner
        ),
        (27, 6, armor_ranges)
    );

    // A partial root literal completes to the commands that start with it.
    assert_eq!(
        ask("lis", CommandPermission::All),
        (0, 3, vec!["list".to_string()])
    );
    // The leading slash is skipped but still counted, so the range names an
    // offset into the string the client sent.
    assert_eq!(
        ask("/lis", CommandPermission::All),
        (1, 3, vec!["list".to_string()])
    );
    // A literal typed in full is dropped rather than echoed, and the cursor
    // sits in that node so its own children are not offered either.
    assert_eq!(ask("list", CommandPermission::All), (0, 0, Vec::new()));
    // Past the node, its children are what is offered.
    assert_eq!(
        ask("list u", CommandPermission::All),
        (5, 1, vec!["uuids".to_string()])
    );
    assert_eq!(
        ask("list uuids", CommandPermission::All),
        (0, 0, Vec::new())
    );
    // Nothing matches, and an empty answer carries no offset at all.
    assert_eq!(ask("zzz", CommandPermission::All), (0, 0, Vec::new()));
    assert_eq!(
        ask("sa", CommandPermission::Owner),
        (
            0,
            2,
            vec![
                "save-all".to_string(),
                "save-off".to_string(),
                "save-on".to_string(),
                "say".to_string(),
            ]
        )
    );
    // The permission filter that hides a command from parsing does not hide it
    // from completion: `getCompletionSuggestions`
    // (com/mojang/brigadier/CommandDispatcher.java:299-311) walks
    // `parent.getChildren()` of the server's own dispatcher and never asks
    // `canUse`, so the four `sa` commands are offered to a source that could
    // run none of them.
    assert_eq!(
        ask("sa", CommandPermission::All),
        ask("sa", CommandPermission::Owner)
    );
    // A trailing separator offers every child of the node before it.
    assert_eq!(
        ask("datapack ", CommandPermission::Owner),
        (
            9,
            0,
            vec![
                "create".to_string(),
                "disable".to_string(),
                "enable".to_string(),
                "list".to_string()
            ]
        )
    );
    assert_eq!(
        ask("datapack list ", CommandPermission::Owner),
        (14, 0, vec!["available".to_string(), "enabled".to_string()])
    );
    assert_eq!(
        ask("debug ", CommandPermission::Owner),
        (
            6,
            0,
            vec![
                "function".to_string(),
                "start".to_string(),
                "stop".to_string()
            ]
        )
    );
    assert_eq!(
        ask("time ", CommandPermission::Owner),
        (
            5,
            0,
            vec![
                "add".to_string(),
                "of".to_string(),
                "pause".to_string(),
                "query".to_string(),
                "rate".to_string(),
                "resume".to_string(),
                "set".to_string(),
            ]
        )
    );
    // The two literals under `query` are offered beside what
    // `suggestTimelines` (TimeCommand.java:120-129) answers for the source's
    // own clock.
    assert_eq!(
        ask("time query ", CommandPermission::Owner),
        (
            11,
            0,
            vec![
                "gametime".to_string(),
                "minecraft:day".to_string(),
                "minecraft:early_game".to_string(),
                "minecraft:moon".to_string(),
                "minecraft:villager_schedule".to_string(),
                "time".to_string(),
            ]
        )
    );
    assert_eq!(
        ask("clone ", CommandPermission::Owner),
        (
            6,
            0,
            vec![
                "from".to_string(),
                "~".to_string(),
                "~ ~".to_string(),
                "~ ~ ~".to_string()
            ]
        )
    );
    assert_eq!(
        ask("whitelist ", CommandPermission::Owner),
        (
            10,
            0,
            vec![
                "add".to_string(),
                "list".to_string(),
                "off".to_string(),
                "on".to_string(),
                "reload".to_string(),
                "remove".to_string(),
            ]
        )
    );
}

#[test]
fn completions_over_a_world_offer_the_online_player_names() {
    let players = [user("alice", 1), user("bob", 2)];
    let completions = completions_for("msg b", CommandPermission::All, world(&players));
    assert_eq!(completions.start, 4);
    assert_eq!(completions.length, 1);
    assert_eq!(texts(&completions), ["bob"]);

    // A game-profile argument reaches the same names through
    // `GameProfileArgument.listSuggestions`.
    let profiles = completions_for("ban a", CommandPermission::Owner, world(&players));
    assert_eq!(texts(&profiles), ["alice"]);
}

#[test]
fn a_player_name_completes_from_the_part_after_a_splitter() {
    // `SharedSuggestionProvider.suggest` (SharedSuggestionProvider.java:226-236)
    // filters with `matchesSubStr` (:271-283), which retries the prefix after
    // every character of `MATCH_SPLITTER` (:31) — `.`, `_` and `/`. An
    // underscore is a legal player-name character: `StringUtil.isValidPlayerName`
    // (StringUtil.java:66-68) rejects only names past sixteen characters and
    // characters at or below space or at or above `\u{7f}`, and `_` is neither.
    // So `smi` reaches `Bob_Smith`, which a plain prefix filter would drop.
    let players = [user("Bob_Smith", 1), user("Smithers", 2), user("Rob", 3)];
    let completions = completions_for("msg smi", CommandPermission::All, world(&players));
    assert_eq!(texts(&completions), ["Bob_Smith", "Smithers"]);

    // The retry only ever restarts just after a splitter, so a suffix that
    // follows none of the three still matches nothing: `ob` is inside `Rob` and
    // inside `Bob_Smith`, and neither is offered.
    assert_eq!(
        texts(&completions_for(
            "msg ob",
            CommandPermission::All,
            world(&players)
        )),
        Vec::<String>::new()
    );
}

#[test]
fn the_six_selector_forms_are_offered_to_a_source_that_may_use_them() {
    // With nothing typed, `parseNameOrUUID` leaves the hook `parse` installed
    // (EntitySelectorParser.java:459) alone, because the guard that would swap
    // it for `suggestName` needs input to read (:262-264). That hook is
    // `suggestNameOrSelector` (:484-491), which adds `fillSelectorSuggestions`
    // (:475-482) after the names. `Suggestions.create` then sorts with
    // `Suggestion.compareToIgnoreCase` (Suggestion.java:80-82), under which `@`
    // precedes every letter.
    let players = [user("alice", 1)];
    let offered = completions_for("msg ", CommandPermission::GameMaster, world(&players));
    assert_eq!(
        texts(&offered),
        ["@a", "@e", "@n", "@p", "@r", "@s", "alice"]
    );
    assert_eq!((offered.start, offered.length), (4, 0));

    // The same hook guards the forms on `allowSelectors` (:486), which
    // `LevelBasedPermissionSet.hasPermission` (LevelBasedPermissionSet.java:18)
    // grants only from `GAMEMASTERS` up, so a moderator gets the names alone.
    assert_eq!(
        texts(&completions_for(
            "msg ",
            CommandPermission::Moderator,
            world(&players)
        )),
        ["alice"]
    );

    // A name being typed moves the hook to `suggestName` (:493-497), which
    // never reaches the forms.
    assert_eq!(
        texts(&completions_for(
            "msg a",
            CommandPermission::GameMaster,
            world(&players)
        )),
        ["alice"]
    );

    // A `@` that names no type yet leaves `suggestSelector` (:499-504) in
    // place, both when the letter is missing (:198-200) and when it is one
    // `parseSelector`'s switch does not know (:213-215). That hook ignores the
    // names it is handed and offers the six forms alone.
    for command in ["msg @", "msg @z"] {
        assert_eq!(
            texts(&completions_for(
                command,
                CommandPermission::GameMaster,
                world(&players)
            )),
            ["@a", "@e", "@n", "@p", "@r", "@s"],
            "{command}"
        );
    }
}

#[test]
fn a_selector_form_carries_its_tooltip_onto_the_wire() {
    // Each `builder.suggest` in `fillSelectorSuggestions`
    // (EntitySelectorParser.java:475-482) passes a tooltip, and
    // `ClientboundCommandSuggestionsPacket` copies `Suggestion.getTooltip`
    // straight into the entry it writes (:31-45, :61-68). Round-tripping the
    // answer through this server's own codec is what pins that the tooltip
    // survives: the decoder rebuilds all three tooltip fields from the bytes
    // alone, so an answer that matches after the trip is one that put its whole
    // tooltip on the wire.
    let players = [user("alice", 1)];
    let offered = completions_for("msg ", CommandPermission::GameMaster, world(&players));
    assert_eq!(
        offered.suggestions[0].tooltip_component,
        Some(compound(vec![(
            "translate",
            text("argument.entity.selector.allPlayers")
        )])),
        "@a is offered under the tooltip its own line names"
    );
    assert_eq!(
        offered.suggestions[6].tooltip_component, None,
        "a player name reaches `suggest(String)` and carries no tooltip"
    );

    let (id, payload) = encode_play_clientbound_command_suggestions(&CommandSuggestions {
        id: 7,
        start: offered.start,
        length: offered.length,
        suggestions: offered.suggestions.clone(),
    })
    .expect("a tooltipped suggestion is representable on the wire");
    let PlayClientbound::CommandSuggestions(decoded) =
        decode_play_clientbound(id, &payload).expect("the answer decodes")
    else {
        panic!("the encoder named the command-suggestions packet");
    };
    assert_eq!(decoded.suggestions, offered.suggestions);
    assert_eq!(
        (decoded.start, decoded.length),
        (offered.start, offered.length)
    );
}

#[test]
fn completions_follow_a_redirect_to_the_command_it_points_at() {
    let players = [user("alice", 1)];
    let completions = completions_for("tell a", CommandPermission::All, world(&players));
    assert_eq!(texts(&completions), ["alice"]);
}

#[test]
fn completion_names_a_command_the_same_source_cannot_parse() {
    // `handleCustomCommandSuggestions` (ServerGamePacketListenerImpl.java:596-597)
    // asks the server's own dispatcher, and
    // `CommandDispatcher.getCompletionSuggestions`
    // (com/mojang/brigadier/CommandDispatcher.java:299-311) walks
    // `parent.getChildren()` with no `canUse` check, while `parseNodes` does
    // apply one. The two therefore disagree, and vanilla offers a command the
    // source could not run.
    assert_eq!(
        texts(&completions_for(
            "sto",
            CommandPermission::All,
            CommandTestWorld::default()
        )),
        ["stop", "stopsound", "stopwatch"]
    );
    let run = ran_as(
        "stop",
        &source("alice", CommandPermission::All),
        CommandTestWorld::default(),
    );
    assert_eq!(keys(&run), ["command.unknown.command", CURSOR_CONTEXT]);
}

#[test]
fn execute_if_biome_offers_registry_ids() {
    assert_eq!(
        texts(&completions_for(
            "execute if biome 0 64 0 pl",
            CommandPermission::Owner,
            CommandTestWorld::with_builtin_data(&[]),
        )),
        [
            "minecraft:plains",
            "minecraft:savanna_plateau",
            "minecraft:snowy_plains",
            "minecraft:sunflower_plains"
        ]
    );
}

#[test]
fn execute_if_function_offers_library_ids() {
    use bbb_server_datapack::function::{parse_function_source, FunctionElement, FunctionLibrary};
    let functions = FunctionLibrary {
        functions: ["test:alpha", "test:beta"]
            .into_iter()
            .map(|id| FunctionElement {
                id: id.into(),
                source_pack: "test".into(),
                source: parse_function_source("say test").unwrap(),
            })
            .collect(),
        tags: None,
        warnings: vec![],
        tag_warnings: vec![],
    };

    assert_eq!(
        texts(&completions_for(
            "execute if function be",
            CommandPermission::Owner,
            CommandTestWorld::default().with_functions(&functions),
        )),
        ["test:beta"]
    );
}

#[test]
fn item_predicate_completions_follow_names_terms_and_validated_values() {
    let prefix = "execute if items entity @s armor.* ";
    let fixture = CommandTestWorld::default();
    let ask = |predicate: &str| {
        completions_for(
            &format!("{prefix}{predicate}"),
            CommandPermission::Owner,
            &fixture,
        )
    };
    let initial = texts(&ask(""));
    for expected in ["#", "*", "minecraft:diamond"] {
        assert!(initial.iter().any(|text| text == expected), "{expected}");
    }
    assert_eq!(texts(&ask("minecraft:diamond")), ["["]);
    for (input, expected) in [
        ("* [ ", "minecraft:count"),
        ("*[", "minecraft:damage"),
        ("*[", "minecraft:enchantments"),
        ("*[", "!"),
        ("*[", "]"),
        ("*[!", "minecraft:count"),
        ("*[minecraft:dam", "minecraft:damage"),
        ("*[count=1,", "minecraft:damage"),
        ("*[count=1|", "!"),
        ("*[count=1, ! ", "minecraft:count"),
    ] {
        assert!(
            texts(&ask(input)).iter().any(|text| text == expected),
            "{input}: {expected}"
        );
    }
    assert!(!texts(&ask("*[!")).iter().any(|text| text == "!"));
    assert_eq!(texts(&ask("*[creative_slot_lock")), ["~"]);
    for input in ["*[count", "*[minecraft:damage", "*[damage "] {
        assert_eq!(texts(&ask(input)), [",", "=", "]", "|", "~"], "{input}");
    }
    for input in [
        "*[count=1",
        "*[damage=2 ",
        "*[count~{min:1}",
        "*[custom_data~{label:'😀'}",
    ] {
        assert_eq!(texts(&ask(input)), [",", "]", "|"], "{input}");
    }
    let predicate = "*[custom_data={label:'😀'}, minecraft:dam";
    let offered = ask(predicate);
    assert_eq!(
        texts(&offered),
        [
            "minecraft:damage",
            "minecraft:damage_resistant",
            "minecraft:damage_type"
        ]
    );
    assert_eq!(
        offered.start,
        (prefix.encode_utf16().count()
            + predicate[..predicate.rfind("minecraft:").unwrap()]
                .encode_utf16()
                .count()) as i32
    );
    assert_eq!(offered.length, "minecraft:dam".len() as i32);
    for input in [
        "*[]",
        "*[count=1]",
        "*[count=",
        "*[custom_data={",
        "*[count='bad',",
        "*[damage='bad'|",
        "*[count~{min:'bad'},",
    ] {
        assert!(texts(&ask(input)).is_empty(), "{input}");
    }
}

#[test]
fn item_predicate_completions_use_published_item_tags() {
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    startup
        .tags
        .registries
        .iter_mut()
        .find(|tags| tags.registry_id == "minecraft:item")
        .unwrap()
        .tags
        .insert("example:treasure".into(), vec!["minecraft:diamond".into()]);
    let fixture = || {
        CommandTestWorld::new(
            crate::world::ServerWorldConfig {
                datapacks: Some(std::sync::Arc::new(startup.clone())),
                ..Default::default()
            },
            &[],
        )
    };
    let offered = completions_for(
        "execute if items entity @s armor.* #example:tr",
        CommandPermission::Owner,
        fixture(),
    );
    assert_eq!(texts(&offered), ["example:treasure"]);
    assert_eq!((offered.start, offered.length), (36, 10));
    let offered = completions_for(
        "execute if items entity @s armor.* #example:treasure",
        CommandPermission::Owner,
        fixture(),
    );
    assert_eq!(texts(&offered), ["["]);
}

/// `EntitySelectorParser` past the type letter (EntitySelectorParser.java:253-325,
/// :506-531): `[`, option keys under their own range, each option's values, and
/// `,`/`]` once a value reads.
#[test]
fn selector_options_complete_after_the_type_letter() {
    let ask = |command: &str| {
        let completions = completions_for(
            command,
            CommandPermission::GameMaster,
            CommandTestWorld::default(),
        );
        (completions.start, texts(&completions))
    };
    assert_eq!(ask("kill @e"), (7, vec!["[".to_string()]));
    let (start, keys) = ask("kill @e[");
    assert_eq!(start, 8);
    assert!(keys.contains(&"type=".to_string()) && keys.contains(&"sort=".to_string()));
    let (_, player_keys) = ask("kill @a[");
    assert!(!player_keys.contains(&"type=".to_string()));
    assert_eq!(ask("kill @e[ty"), (8, vec!["type=".to_string()]));
    let (start, types) = ask("kill @e[type=zomb");
    assert_eq!(start, 13);
    assert!(types.contains(&"minecraft:zombie".to_string()), "{types:?}");
    assert_eq!(
        ask("kill @e[type=minecraft:zombie"),
        (29, vec![",".to_string(), "]".to_string()])
    );
    assert_eq!(ask("kill @e[limit=1,so").1, vec!["sort=".to_string()]);
    assert_eq!(
        ask("kill @e[sort=").1,
        ["arbitrary", "furthest", "nearest", "random"]
    );
}

/// `TimeCommand` takes a `minecraft:world_clock` resource (TimeCommand.java:56).
#[test]
fn time_of_suggests_the_world_clocks() {
    let offered = texts(&completions_for(
        "time of ",
        CommandPermission::GameMaster,
        CommandTestWorld::default(),
    ));
    assert!(
        offered.contains(&"minecraft:overworld".to_string()),
        "{offered:?}"
    );
}

/// `suggestRegistryElements` offers the tags under `#` before the elements
/// (SharedSuggestionProvider.java:63-73).
#[test]
fn resource_or_tag_suggests_tags() {
    let offered = texts(&completions_for(
        "locate biome #",
        CommandPermission::GameMaster,
        CommandTestWorld::default(),
    ));
    assert!(
        offered.iter().any(|text| text.starts_with('#')),
        "{offered:?}"
    );
}

#[test]
fn custom_server_suggestions_and_objective_scoreholder_completions() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let charlie = user("charlie", 3);
    let mut lists = crate::admin::StoredUserLists::default();
    lists.operators.insert(
        alice.id,
        crate::admin::OperatorEntry {
            user: alice.clone(),
            level: CommandPermission::Owner,
            bypasses_player_limit: false,
        },
    );
    lists.allowlist.insert(alice.id, alice.clone());
    lists.player_bans.insert(
        charlie.id,
        crate::admin::PlayerBanEntry {
            user: charlie.clone(),
            ban: crate::admin::BanRecord {
                created: 0,
                source: "Server".into(),
                expires: None,
                reason: None,
            },
        },
    );
    lists.ip_bans.insert(
        "127.0.0.1".to_string(),
        crate::admin::BanRecord {
            created: 0,
            source: "Server".into(),
            expires: None,
            reason: None,
        },
    );
    let mut fixture = CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            admin_lists: lists,
            ..Default::default()
        },
        &[alice.clone(), bob.clone()],
    );
    fixture.setup("scoreboard objectives add kills dummy");
    fixture.setup("scoreboard objectives add vote trigger");
    fixture.setup("scoreboard objectives add secret trigger");
    fixture.setup("scoreboard players enable alice vote");
    fixture.setup("attribute alice minecraft:max_health modifier add example:boost 2 add_value");

    // Admin list overrides filter out already-listed players and never echo selectors.
    assert_eq!(
        texts(&completions_for("op ", CommandPermission::Owner, &fixture)),
        ["bob"]
    );
    assert_eq!(
        texts(&completions_for(
            "deop ",
            CommandPermission::Owner,
            &fixture
        )),
        ["alice"]
    );
    assert_eq!(
        texts(&completions_for(
            "whitelist add ",
            CommandPermission::Owner,
            &fixture
        )),
        ["bob"]
    );
    assert_eq!(
        texts(&completions_for(
            "whitelist remove ",
            CommandPermission::Owner,
            &fixture
        )),
        ["alice"]
    );
    assert_eq!(
        texts(&completions_for(
            "pardon ",
            CommandPermission::Owner,
            &fixture
        )),
        ["charlie"]
    );
    assert_eq!(
        texts(&completions_for(
            "pardon-ip ",
            CommandPermission::Owner,
            &fixture
        )),
        ["127.0.0.1"]
    );

    // Objective & ScoreHolder default suggestions.
    assert_eq!(
        texts(&completions_for(
            "scoreboard objectives remove ",
            CommandPermission::GameMaster,
            &fixture
        )),
        ["kills", "secret", "vote"]
    );
    let holders = texts(&completions_for(
        "scoreboard players get ",
        CommandPermission::GameMaster,
        &fixture,
    ));
    assert!(
        holders.contains(&"alice".to_string()) && holders.contains(&"@a".to_string()),
        "{holders:?}"
    );

    // Trigger & scoreboard players enable custom suggestions: console has no
    // entity so `/trigger ` suggests nothing, while Alice sees unlocked `vote`.
    assert_eq!(
        texts(&completions_for(
            "trigger ",
            CommandPermission::All,
            &fixture
        )),
        Vec::<String>::new()
    );
    assert_eq!(
        texts(&completion_suggestions_in_world(
            "trigger ",
            &source("alice", CommandPermission::All),
            fixture.world(),
            fixture.origin(),
        )),
        ["vote"]
    );
    assert_eq!(
        texts(&completions_for(
            "scoreboard players enable alice ",
            CommandPermission::GameMaster,
            &fixture
        )),
        ["secret"]
    );

    // Attribute modifier id suggestions.
    assert_eq!(
        texts(&completions_for(
            "attribute alice minecraft:max_health modifier remove ",
            CommandPermission::GameMaster,
            &fixture
        )),
        ["example:boost"]
    );
    assert_eq!(
        texts(&completions_for(
            "attribute alice minecraft:max_health modifier value get ",
            CommandPermission::GameMaster,
            &fixture
        )),
        ["example:boost"]
    );
}
