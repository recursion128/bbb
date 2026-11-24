//! `/list`, `/op`, `/deop`, `/kick`, `/ban`, `/ban-ip`, `/pardon`, `/pardon-ip`, `/banlist` and `/whitelist`.

use super::*;

#[test]
fn list_reports_the_online_players_through_the_vanilla_translation_key() {
    // `commands.list.players` carries the count, the capacity and the
    // formatted list; the official server never sends rendered English, so
    // neither may bbb.
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran_as(
        "list",
        &source("alice", CommandPermission::All),
        world(&players),
    );
    assert_eq!(run.result, Some(2));
    assert_eq!(
        body(&run.feedback[0]),
        &compound(vec![
            ("translate", text("commands.list.players")),
            (
                "with",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Int(2),
                    NbtSummaryValue::Int(20),
                    compound(vec![
                        ("text", text("")),
                        (
                            "extra",
                            NbtSummaryValue::List(vec![
                                display_name("alice", 1),
                                compound(vec![("text", text(", ")), ("color", text("gray"))]),
                                display_name("bob", 2),
                            ])
                        ),
                    ]),
                ])
            ),
        ])
    );
}

#[test]
fn a_single_player_list_is_not_wrapped_in_an_empty_root() {
    // `ComponentUtils.formatList` returns the one value itself rather than a
    // one-element list under an empty component.
    let players = [user("alice", 1)];
    let run = ran("list", world(&players));
    assert_eq!(arguments(&run, 0)[2], display_name("alice", 1));
}

#[test]
fn list_uuids_takes_the_longer_literal_path_and_spells_the_identity() {
    let players = [user("alice", 1)];
    let run = ran("list uuids", world(&players));
    assert_eq!(keys(&run), ["commands.list.players"]);
    assert_eq!(
        arguments(&run, 0)[2],
        compound(vec![
            ("translate", text("commands.list.nameAndId")),
            (
                "with",
                NbtSummaryValue::List(vec![text("alice"), text(&Uuid::from_u128(1).to_string()),])
            ),
        ])
    );
}

#[test]
fn op_and_deop_report_one_line_per_changed_player_and_refuse_a_no_op() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut granting = world(&players);
    let granted = ran("op bob", &mut granting);
    assert_eq!(keys(&granted), ["commands.op.success"]);
    assert_eq!(arguments(&granted, 0), vec![text("bob")]);
    assert_eq!(granted.result, Some(1));
    assert!(granting
        .world()
        .admin_lists()
        .operators
        .contains_key(&user("bob", 2).id));

    let operators = [user("bob", 2)];
    let already = ran("op bob", world(&players).with_operators(&operators));
    assert_eq!(keys(&already), ["commands.op.failed"]);
    assert_eq!(already.result, None);

    let mut removing = world(&players).with_operators(&operators);
    let removed = ran("deop bob", &mut removing);
    assert_eq!(keys(&removed), ["commands.deop.success"]);
    assert!(!removing
        .world()
        .admin_lists()
        .operators
        .contains_key(&user("bob", 2).id));
}

#[test]
fn a_game_profile_name_the_server_never_saw_resolves_to_its_offline_identity() {
    // On a server that does not authenticate, the name cache never misses:
    // `lookupGameProfile` falls through to `createUnknownProfile`
    // (CachedUserNameToIdResolver.java:63-74) and `resolveOfflineUsers` is
    // `!usesAuthentication()` (DedicatedServer.java:280), which is always true
    // here. `get` looks up the lower-cased string (:103) and hands that same
    // string to the lookup (:118), so the identity is the offline one for the
    // lower-cased name.
    let mut opped = world(&[]);
    let run = ran("op NoBody", &mut opped);
    assert_eq!(keys(&run), ["commands.op.success"]);
    assert_eq!(run.result, Some(1));
    assert!(opped
        .world()
        .admin_lists()
        .operators
        .contains_key(&NameAndId::offline("nobody").id));
    assert_eq!(arguments(&run, 0), vec![text("nobody")]);
    for units in [
        vec![0xd800],
        vec![0xd801],
        vec![0xfffd],
        vec![0xd83d, 0xde00],
    ] {
        let name = bbb_protocol::JavaString::from(units.clone());
        let mut fixture = world(&[]);
        let sender = CommandSource::server("console", CommandPermission::Owner);
        for verb in ["op ", "ban "] {
            let command: Vec<_> = verb.encode_utf16().chain(units.iter().copied()).collect();
            let run = fixture.run_units(&command, &sender);
            assert_eq!(run.result, Some(1), "{verb}: {run:?}");
            assert_eq!(arguments(&run, 0)[0], NbtSummaryValue::String(name.clone()));
        }
        let id = bbb_protocol::codec::offline_player_uuid_utf16(&units);
        let lists = fixture.world().admin_lists();
        assert_eq!(lists.operators[&id].user.name, name);
        assert_eq!(lists.player_bans[&id].user.name, name);
        let restored = crate::admin::StoredUserLists::parse(&lists.documents()).unwrap();
        assert_eq!(restored.operators[&id].user.name, name);
        assert_eq!(restored.player_bans[&id].user.name, name);
    }
}

#[test]
fn kick_disconnects_every_target_with_the_reason_it_was_given() {
    let players = [user("alice", 1), user("bob", 2)];
    let default_reason = ran("kick bob", world(&players));
    assert_eq!(keys(&default_reason), ["commands.kick.success"]);
    let kicked = component(
        component_translatable_nbt(
            "multiplayer.disconnect.kicked",
            Vec::new(),
            &ComponentStyle::default(),
        )
        .unwrap(),
    );
    assert_eq!(default_reason.closed(), [(2, &kicked)]);

    let with_reason = ran("kick bob go away", world(&players));
    assert_eq!(
        arguments(&with_reason, 0),
        vec![display_name("bob", 2), text("go away")]
    );
    assert_eq!(with_reason.result, Some(1));
}

#[test]
fn ban_records_the_ban_disconnects_the_player_and_refuses_a_second_one() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut banning = world(&players);
    let banned = ran("ban bob rude", &mut banning);
    assert_eq!(keys(&banned), ["commands.ban.success"]);
    assert_eq!(arguments(&banned, 0), vec![text("bob"), text("rude")]);
    assert_eq!(
        banning
            .world()
            .admin_lists()
            .player_bans
            .values()
            .map(|entry| (
                entry.user.clone(),
                entry.ban.source.clone(),
                entry.ban.reason.clone()
            ))
            .collect::<Vec<_>>(),
        [(user("bob", 2), "alice".into(), Some("rude".into()))]
    );
    let reason = component(
        component_translatable_nbt(
            "multiplayer.disconnect.banned",
            Vec::new(),
            &ComponentStyle::default(),
        )
        .unwrap(),
    );
    assert_eq!(banned.closed(), [(2, &reason)]);

    let bans = [(user("bob", 2), "alice", None)];
    let again = ran("ban bob", world(&players).with_player_bans(&bans));
    assert_eq!(keys(&again), ["commands.ban.failed"]);
}

#[test]
fn a_ban_without_a_reason_carries_the_default_reason_component() {
    let profiles = [user("bob", 2)];
    let run = ran(
        "ban bob",
        CommandTestWorld::default().with_whitelist(&profiles),
    );
    assert_eq!(
        arguments(&run, 0)[1],
        compound(vec![(
            "translate",
            text("multiplayer.disconnect.banned.reason.default")
        )])
    );
}

#[test]
fn ban_ip_takes_an_address_or_the_name_of_a_connected_player() {
    let players = [user("bob", 2)];
    let mut banning = world(&players).with_player_address("bob", "10.0.0.7");
    let literal = ran("ban-ip 10.0.0.7 spam", &mut banning);
    assert_eq!(
        keys(&literal),
        ["commands.banip.success", "commands.banip.info"]
    );
    assert_eq!(literal.result, Some(1));
    assert!(banning
        .world()
        .admin_lists()
        .ip_bans
        .contains_key("10.0.0.7"));
    let reason = component(
        component_translatable_nbt(
            "multiplayer.disconnect.ip_banned",
            Vec::new(),
            &ComponentStyle::default(),
        )
        .unwrap(),
    );
    assert_eq!(literal.closed(), [(1, &reason)]);

    let by_name = ran(
        "ban-ip bob",
        world(&players).with_player_address("bob", "10.0.0.7"),
    );
    assert_eq!(
        keys(&by_name),
        ["commands.banip.success", "commands.banip.info"]
    );

    let unknown = ran(
        "ban-ip nobody",
        world(&players).with_player_address("bob", "10.0.0.7"),
    );
    assert_eq!(keys(&unknown), ["commands.banip.invalid"]);
}

#[test]
fn pardon_and_pardon_ip_only_answer_for_an_entry_that_exists() {
    let profiles = [user("bob", 2)];
    let bans = [(user("bob", 2), "alice", None)];
    let ip_bans = [("10.0.0.7", "alice", None)];
    let mut pardoning = CommandTestWorld::default().with_player_bans(&bans);
    let pardoned = ran("pardon bob", &mut pardoning);
    assert_eq!(keys(&pardoned), ["commands.pardon.success"]);
    assert!(!pardoning
        .world()
        .admin_lists()
        .player_bans
        .contains_key(&user("bob", 2).id));

    let not_banned = ran(
        "pardon bob",
        CommandTestWorld::default().with_whitelist(&profiles),
    );
    assert_eq!(keys(&not_banned), ["commands.pardon.failed"]);

    let mut pardoning = CommandTestWorld::default().with_ip_bans(&ip_bans);
    let unbanned_ip = ran("pardon-ip 10.0.0.7", &mut pardoning);
    assert_eq!(keys(&unbanned_ip), ["commands.pardonip.success"]);
    assert!(!pardoning
        .world()
        .admin_lists()
        .ip_bans
        .contains_key("10.0.0.7"));

    let malformed = ran("pardon-ip nothing", world(&[]));
    assert_eq!(keys(&malformed), ["commands.pardonip.invalid"]);
}

#[test]
fn banlist_reports_a_header_and_one_message_per_entry() {
    let bans = [(user("bob", 2), "alice", Some("rude"))];
    let ip_bans = [("10.0.0.7", "alice", None)];
    let both = ran(
        "banlist",
        CommandTestWorld::default()
            .with_player_bans(&bans)
            .with_ip_bans(&ip_bans),
    );
    assert_eq!(
        keys(&both),
        [
            "commands.banlist.list",
            "commands.banlist.entry",
            "commands.banlist.entry"
        ]
    );
    assert_eq!(both.result, Some(2));
    assert_eq!(
        arguments(&both, 1),
        vec![text("bob"), text("alice"), text("rude")]
    );

    let players_only = ran(
        "banlist players",
        CommandTestWorld::default()
            .with_player_bans(&bans)
            .with_ip_bans(&ip_bans),
    );
    assert_eq!(players_only.result, Some(1));
    assert_eq!(
        arguments(&players_only, 1)[0],
        text("bob"),
        "the players list names no address"
    );

    let empty = ran("banlist ips", world(&[]));
    assert_eq!(keys(&empty), ["commands.banlist.none"]);
    assert_eq!(empty.result, Some(0));
}

#[test]
fn whitelist_switches_the_flag_and_refuses_the_state_it_is_already_in() {
    let mut switching = world(&[]);
    let enabled = ran("whitelist on", &mut switching);
    assert_eq!(keys(&enabled), ["commands.whitelist.enabled"]);
    assert!(switching.world().admin_settings().use_allowlist);

    let already_off = ran("whitelist off", world(&[]));
    assert_eq!(keys(&already_off), ["commands.whitelist.alreadyOff"]);

    let disabled = ran("whitelist off", &mut switching);
    assert_eq!(keys(&disabled), ["commands.whitelist.disabled"]);
    assert!(!switching.world().admin_settings().use_allowlist);
}

#[test]
fn whitelist_lists_the_names_joined_by_a_plain_separator() {
    let whitelist = [user("alice", 1), user("bob", 2)];
    let listed = ran(
        "whitelist list",
        CommandTestWorld::default().with_whitelist(&whitelist),
    );
    assert_eq!(keys(&listed), ["commands.whitelist.list"]);
    assert_eq!(
        arguments(&listed, 0),
        vec![NbtSummaryValue::Int(2), text("alice, bob")]
    );

    let empty = ran("whitelist list", world(&[]));
    assert_eq!(keys(&empty), ["commands.whitelist.none"]);
}

#[test]
fn whitelist_add_and_remove_change_one_entry_each() {
    let profiles = [user("bob", 2)];
    let mut adding = CommandTestWorld::default().with_operators(&profiles);
    let added = ran("whitelist add bob", &mut adding);
    assert_eq!(keys(&added), ["commands.whitelist.add.success"]);
    assert!(adding
        .world()
        .admin_lists()
        .allowlist
        .contains_key(&user("bob", 2).id));

    let whitelist = [user("bob", 2)];
    let already = ran(
        "whitelist add bob",
        CommandTestWorld::default().with_whitelist(&whitelist),
    );
    assert_eq!(keys(&already), ["commands.whitelist.add.failed"]);

    let mut removing = CommandTestWorld::default().with_whitelist(&whitelist);
    let removed = ran("whitelist remove bob", &mut removing);
    assert_eq!(keys(&removed), ["commands.whitelist.remove.success"]);
    assert!(!removing
        .world()
        .admin_lists()
        .allowlist
        .contains_key(&user("bob", 2).id));

    let reloaded = ran("whitelist reload", world(&[]));
    assert_eq!(keys(&reloaded), ["commands.whitelist.reloaded"]);
}

#[test]
fn a_game_profile_name_is_read_to_the_next_space_rather_than_by_the_selector_grammar() {
    // `GameProfileArgument.parse` (GameProfileArgument.java:46-68) hands the
    // input to `EntitySelectorParser` only when it starts with `@` (:47).
    // Everything else is `while (reader.canRead() && reader.peek() != ' ')
    // reader.skip()` (:58-60) and reaches the name cache verbatim (:62-66), so
    // none of what `parseNameOrUUID` applies is applied here: no quote
    // unwrapping, no sixteen-character bound and no character set that ends the
    // word early.
    let profiles = [user("Bob", 2), user("a-name-longer-than-sixteen", 3)];
    let mut known = CommandTestWorld::default().with_whitelist(&profiles);

    // The quotes are part of the name the cache is asked for, so `Bob` is not
    // what got banned: the quoted word misses the cache and resolves to its own
    // offline identity instead.
    let quoted = ran("ban \"Bob\"", &mut known);
    assert_eq!(keys(&quoted), ["commands.ban.success"]);
    assert_eq!(arguments(&quoted, 0)[0], text("\"bob\""));

    // The same word without them is the entry the cache holds, under the
    // casing it was stored with.
    let plain = ran("ban Bob", &mut known);
    assert_eq!(keys(&plain), ["commands.ban.success"]);
    assert_eq!(arguments(&plain, 0)[0], text("Bob"));

    // A name past `parseNameOrUUID`'s sixteen-character bound is still just a
    // word here, because that bound belongs to the grammar this branch skips.
    let long = ran("ban a-name-longer-than-sixteen", &mut known);
    assert_eq!(keys(&long), ["commands.ban.success"]);
    assert_eq!(arguments(&long, 0)[0], text("a-name-longer-than-sixteen"));
}

#[test]
fn an_online_player_name_matches_whatever_case_it_was_typed_in() {
    // `EntitySelector.findPlayers`
    // (commands/arguments/selector/EntitySelector.java:209-211) resolves a plain
    // name through `PlayerList.getPlayerByName`, which compares with
    // `equalsIgnoreCase` (PlayerList.java:589). `BanIpCommands.banIpOrName`
    // (BanIpCommands.java:42) reaches the same method.
    let players = [user("bob", 2)];
    let kicked = ran(
        "kick BOB",
        world(&players).with_player_address("bob", "10.0.0.7"),
    );
    assert_eq!(keys(&kicked), ["commands.kick.success"]);
    assert_eq!(kicked.result, Some(1));
    assert_eq!(kicked.closed().len(), 1);
    assert_eq!(kicked.closed()[0].0, 1);

    let banned = ran(
        "ban-ip BOB",
        world(&players).with_player_address("bob", "10.0.0.7"),
    );
    assert_eq!(
        keys(&banned),
        ["commands.banip.success", "commands.banip.info"]
    );
    assert_eq!(arguments(&banned, 0)[0], text("10.0.0.7"));
}

#[test]
fn the_name_cache_answers_a_lookup_whatever_case_it_was_typed_in() {
    // `CachedUserNameToIdResolver.get` (CachedUserNameToIdResolver.java:102-104)
    // looks the name up lower-cased and `safeAdd` (:56-61) stored it under the
    // same key, so the entry answers under any casing — and it answers with the
    // name it was stored with rather than the one that was typed.
    let profiles = [user("Bob", 2)];
    let mut known = CommandTestWorld::default().with_whitelist(&profiles);
    let run = ran("op BOB", &mut known);
    assert_eq!(keys(&run), ["commands.op.success"]);
    assert_eq!(arguments(&run, 0), vec![text("Bob")]);
    assert!(known
        .world()
        .admin_lists()
        .operators
        .contains_key(&user("Bob", 2).id));
}
