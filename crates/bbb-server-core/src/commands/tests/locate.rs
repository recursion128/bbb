//! `/locate` with live registry lookup, biome search and POI placement.

use super::*;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::BlockPos;

fn locate_world() -> CommandTestWorld {
    CommandTestWorld::with_builtin_data(&[]).with_position([12.9, 70.5, -3.1])
}

#[test]
fn locate_is_gamemaster_only_and_publishes_all_three_searches() {
    let mut world = locate_world();
    let refusal = ran_as(
        "locate biome minecraft:plains",
        &source("alice", CommandPermission::Moderator),
        &mut world,
    );
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
    let help = ran("help", &mut world);
    assert!(help
        .feedback
        .iter()
        .any(|feedback| body(feedback) == &text("/locate (structure|biome|poi)")));
    assert!(texts(&completions_for(
        "locate biome minecraft:pl",
        CommandPermission::GameMaster,
        &world
    ))
    .contains(&"minecraft:plains".to_string()));
}

#[test]
fn locate_rejects_unknown_elements_and_tags_during_parse() {
    let mut world = locate_world();
    for (command, key) in [
        (
            "locate biome minecraft:missing",
            "argument.resource.not_found",
        ),
        (
            "locate biome #minecraft:missing",
            "argument.resource_tag.not_found",
        ),
        (
            "locate poi minecraft:missing",
            "argument.resource.not_found",
        ),
        (
            "locate poi #minecraft:missing",
            "argument.resource_tag.not_found",
        ),
        (
            "locate structure minecraft:missing",
            "commands.locate.structure.invalid",
        ),
        (
            "locate structure #minecraft:missing",
            "commands.locate.structure.invalid",
        ),
    ] {
        assert_eq!(keys(&ran(command, &mut world))[0], key);
    }
}

#[test]
fn locate_success_reports_the_kind_distance_and_coordinates() {
    let bell = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:bell")
            .unwrap()
            .id,
    )
    .unwrap();
    let mut world = locate_world().with_block(BlockPos { x: 15, y: 74, z: 8 }, bell);
    for (kind, id, source_y, pos, distance, y) in [
        (
            "biome",
            "minecraft:plains",
            325.0,
            BlockPos {
                x: 12,
                y: 320,
                z: -4,
            },
            5,
            "320",
        ),
        (
            "poi",
            "minecraft:meeting",
            70.5,
            BlockPos { x: 15, y: 74, z: 8 },
            12,
            "~",
        ),
    ] {
        world = world.with_position([12.9, source_y, -3.1]);
        let run = ran(&format!("locate {kind} {id}"), &mut world);
        assert_eq!(run.result, Some(distance));
        assert_eq!(keys(&run), [format!("commands.locate.{kind}.success")]);
        assert!(!run.feedback[0].inform_admins);
        assert_eq!(
            arguments(&run, 0),
            vec![
                text(id),
                compound(vec![
                    ("translate", text("chat.square_brackets")),
                    (
                        "with",
                        NbtSummaryValue::List(vec![compound(vec![
                            ("translate", text("chat.coordinates")),
                            (
                                "with",
                                NbtSummaryValue::List(vec![
                                    NbtSummaryValue::Int(pos.x),
                                    text(y),
                                    NbtSummaryValue::Int(pos.z)
                                ])
                            ),
                        ])])
                    ),
                    ("color", text("green")),
                    (
                        "click_event",
                        compound(vec![
                            ("action", text("suggest_command")),
                            ("command", text(&format!("/tp @s {} {y} {}", pos.x, pos.z)))
                        ])
                    ),
                    (
                        "hover_event",
                        compound(vec![
                            ("action", text("show_text")),
                            (
                                "value",
                                compound(vec![("translate", text("chat.coordinates.tooltip"))])
                            )
                        ])
                    ),
                ]),
                NbtSummaryValue::Int(distance)
            ]
        );
    }
}

#[test]
fn locate_tag_success_names_the_actual_biome_in_the_plain_argument() {
    let run = ran("locate biome #minecraft:is_overworld", locate_world());
    assert_eq!(run.result, Some(0));
    assert_eq!(
        arguments(&run, 0)[0],
        text("#minecraft:is_overworld (minecraft:plains)")
    );
}

#[test]
fn locate_not_found_is_a_player_failure() {
    let mut world = locate_world();
    let missing = ran("locate biome minecraft:forest", &mut world);
    assert_eq!(missing.result, None);
    assert_eq!(keys(&missing), ["commands.locate.biome.not_found"]);
    assert_eq!(kinds(&missing), [CommandFeedbackKind::Failure]);
    assert_eq!(arguments(&missing, 0), [text("minecraft:forest")]);
}
