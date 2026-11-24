//! `/fetchprofile` feedback reads actual player and mannequin profiles.
use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID;

fn player_world(players: &[NameAndId]) -> CommandTestWorld {
    world(players)
}
fn stored_world(entities: &[crate::entity::Entity]) -> CommandTestWorld {
    world(&[]).with_entities(entities)
}

fn signed_profile() -> NbtSummaryValue {
    compound(vec![
        ("id", NbtSummaryValue::IntArray(vec![0, 0, 0, 1])),
        ("name", text("alice")),
        (
            "properties",
            NbtSummaryValue::List(vec![compound(vec![
                ("name", text("textures")),
                ("value", text("signed-value")),
                ("signature", text("signed-signature")),
            ])]),
        ),
    ])
}

fn bracket(key: &str, click_action: &str, click_field: &str, click_value: &str) -> NbtSummaryValue {
    compound(vec![
        ("translate", text("chat.square_brackets")),
        (
            "with",
            NbtSummaryValue::List(vec![compound(vec![
                ("translate", text(key)),
                ("color", text("green")),
                (
                    "click_event",
                    compound(vec![
                        ("action", text(click_action)),
                        (click_field, text(click_value)),
                    ]),
                ),
            ])]),
        ),
    ])
}

fn strings(value: &NbtSummaryValue) -> Vec<String> {
    fn collect(value: &NbtSummaryValue, values: &mut Vec<String>) {
        match value {
            NbtSummaryValue::String(value) => values.push(
                value
                    .try_to_string()
                    .expect("fixture text is valid Unicode"),
            ),
            NbtSummaryValue::List(entries) => {
                for entry in entries {
                    collect(entry, values);
                }
            }
            NbtSummaryValue::Compound(entries) => {
                for entry in entries {
                    collect(&entry.value, values);
                }
            }
            _ => {}
        }
    }

    let mut values = Vec::new();
    collect(value, &mut values);
    values
}

#[test]
fn fetchprofile_is_gamemaster_only_and_publishes_only_entity() {
    let players = [user("alice", 1)];
    let refusal = ran_as(
        "fetchprofile entity alice",
        &source("alice", CommandPermission::Moderator),
        player_world(&players),
    );
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );

    assert_eq!(catalog::IMPLEMENTED_COMMANDS.len(), 87);
    let root = vanilla_command_lookup(&["fetchprofile"]).expect("generated fetchprofile root");
    assert_eq!(
        catalog::effective_permission(root),
        CommandPermission::GameMaster
    );
    assert_eq!(
        catalog::implemented_children(root)
            .map(|child| catalog::node(child).name)
            .collect::<Vec<_>>(),
        ["entity"]
    );
    for command in ["fetchprofile name alice", "fetchprofile id 0-0-0-0-0"] {
        let run = ran(command, player_world(&players));
        assert_eq!(keys(&run)[0], "command.unknown.argument", "{command}");
    }
    let at = catalog::IMPLEMENTED_COMMANDS
        .iter()
        .position(|name| *name == "fetchprofile")
        .expect("fetchprofile is implemented");
    assert_eq!(
        &catalog::IMPLEMENTED_COMMANDS[at - 1..=at + 1],
        &["recipe", "fetchprofile", "return"]
    );
    let help = ran("help", player_world(&players));
    assert_eq!(help.result, Some(87));
}

#[test]
fn avatar_success_encodes_all_four_clickable_entries_exactly() {
    let players = [user("alice", 1)];
    let run = ran(
        "fetchprofile entity alice",
        player_world(&players).with_profile_properties(
            "alice",
            &[bbb_protocol::packets::GameProfileProperty {
                name: "textures".into(),
                value: "signed-value".into(),
                signature: Some("signed-signature".into()),
            }],
        ),
    );
    let profile_snbt = "{id:[I;0,0,0,1],name:\"alice\",properties:[{name:\"textures\",signature:\"signed-signature\",value:\"signed-value\"}]}";
    assert_eq!(run.result, Some(1));
    assert_eq!(run.successful_contexts, 1);
    assert_eq!(keys(&run), ["commands.fetchprofile.entity.success"]);
    assert!(!run.feedback[0].inform_admins);

    let args = arguments(&run, 0);
    assert_eq!(args[0], display_name("alice", 1));
    let sprite_snbt = format!("{{player:{profile_snbt}}}");
    let copy_text = compound(vec![
        ("translate", text("chat.square_brackets")),
        (
            "with",
            NbtSummaryValue::List(vec![compound(vec![
                ("translate", text("commands.fetchprofile.copy_text")),
                (
                    "with",
                    NbtSummaryValue::List(vec![compound(vec![
                        ("player", signed_profile()),
                        ("color", text("white")),
                    ])]),
                ),
                ("color", text("green")),
                (
                    "click_event",
                    compound(vec![
                        ("action", text("copy_to_clipboard")),
                        ("value", text(&sprite_snbt)),
                    ]),
                ),
            ])]),
        ),
    ]);
    assert_eq!(
        args[1],
        compound(vec![
            ("text", text("")),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    bracket(
                        "commands.fetchprofile.copy_component",
                        "copy_to_clipboard",
                        "value",
                        profile_snbt,
                    ),
                    text(" "),
                    bracket(
                        "commands.fetchprofile.give_item",
                        "run_command",
                        "command",
                        &format!("give @s minecraft:player_head[profile={profile_snbt}]"),
                    ),
                    text(" "),
                    bracket(
                        "commands.fetchprofile.summon_mannequin",
                        "run_command",
                        "command",
                        &format!("summon minecraft:mannequin ~ ~ ~ {{profile:{profile_snbt}}}"),
                    ),
                    text(" "),
                    copy_text,
                ]),
            ),
        ])
    );
    assert!(!sprite_snbt.contains("hat"));
    assert!(!sprite_snbt.contains("color"));
}

#[test]
fn partial_and_full_mannequin_profiles_are_preserved_in_click_payloads() {
    let mannequin = stored_entity(
        2,
        17,
        [4.0, 70.0, 8.0],
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
                .expect("mannequin entity data"),
        ),
    );
    for (_profile, profile_snbt) in [
        (compound(vec![("name", text("Alex"))]), "{name:\"Alex\"}"),
        (
            signed_profile(),
            "{id:[I;0,0,0,1],name:\"alice\",properties:[{name:\"textures\",signature:\"signed-signature\",value:\"signed-value\"}]}",
        ),
    ] {
        let mut fixture = stored_world(std::slice::from_ref(&mannequin));
        fixture.setup(&format!("data merge entity 0-0-0-0-2 {{profile:{profile_snbt}}}"));
        let run = ran("fetchprofile entity 0-0-0-0-2", &mut fixture);
        let feedback_arguments = arguments(&run, 0);
        let values = strings(&feedback_arguments[1]);
        assert!(values.iter().any(|value| value == profile_snbt), "{values:?}");
        assert!(
            values.contains(&format!("{{player:{profile_snbt}}}")),
            "{values:?}"
        );
    }
}

#[test]
fn non_avatar_fails_without_success() {
    let entity = stored_entity(
        3,
        19,
        [0.0, 64.0, 0.0],
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(1)),
    );
    let not_avatar = ran(
        "fetchprofile entity 00000000-0000-0000-0000-000000000003",
        stored_world(std::slice::from_ref(&entity)),
    );
    let name = compound(vec![("translate", text("entity.minecraft.experience_orb"))]);
    let display = compound(vec![
        ("translate", text("entity.minecraft.experience_orb")),
        (
            "hover_event",
            compound(vec![
                ("action", text("show_entity")),
                ("id", text("minecraft:experience_orb")),
                ("uuid", NbtSummaryValue::IntArray(vec![0, 0, 0, 3])),
                ("name", name),
            ]),
        ),
        ("insertion", text(&Uuid::from_u128(3).to_string())),
    ]);
    assert_eq!(not_avatar.result, None);
    assert_eq!(keys(&not_avatar), ["commands.fetchprofile.no_profile"]);
    assert_eq!(arguments(&not_avatar, 0), [display]);
    assert_eq!(kinds(&not_avatar), [CommandFeedbackKind::Failure]);
}

#[test]
fn entity_argument_reports_not_found_and_too_many_before_deferring() {
    let empty = ran(
        "fetchprofile entity 00000000-0000-0000-0000-000000000099",
        CommandTestWorld::default(),
    );
    assert_eq!(keys(&empty), ["argument.entity.notfound.entity"]);

    let players = [user("alice", 1), user("bob", 2)];
    let many = ran("fetchprofile entity @e", player_world(&players));
    assert_eq!(keys(&many)[0], "argument.entity.toomany");
}

#[test]
fn execute_and_return_report_the_actual_profile_result() {
    for command in [
        "execute run fetchprofile entity alice",
        "return run fetchprofile entity alice",
    ] {
        let run = ran(command, player_world(&[user("alice", 1)]));
        assert_eq!(run.result, Some(1), "{command}");
        assert_eq!(keys(&run), ["commands.fetchprofile.entity.success"]);
    }
}
