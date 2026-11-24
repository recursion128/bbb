//! `/waypoint` (`WaypointCommand.java:33-158`).

use bbb_protocol::packets::{vanilla_command_lookup, WaypointIcon};

use super::*;

fn icon(style: &str, color_rgb: Option<u32>) -> WaypointIcon {
    WaypointIcon {
        style: style.to_owned(),
        color_rgb,
    }
}

#[test]
fn every_modify_shape_sets_the_complete_raw_icon_reports_and_returns_zero() {
    let players = [user("alice", 1), user("bob", 2)];
    let cases = [
        (
            "waypoint modify bob color red",
            icon("minecraft:pin", Some(0xff_55_55)),
            "commands.waypoint.modify.color",
            Some("red"),
        ),
        (
            "waypoint modify bob color hex 1aF",
            icon("minecraft:pin", Some(0x11_aa_ff)),
            "commands.waypoint.modify.color",
            Some("11AAFF"),
        ),
        (
            "waypoint modify bob color hex 123aBc",
            icon("minecraft:pin", Some(0x12_3a_bc)),
            "commands.waypoint.modify.color",
            Some("123ABC"),
        ),
        (
            "waypoint modify bob color reset",
            icon("minecraft:pin", None),
            "commands.waypoint.modify.color.reset",
            None,
        ),
        (
            "waypoint modify bob style set pins/green",
            icon("minecraft:pins/green", Some(0x01_02_03)),
            "commands.waypoint.modify.style",
            None,
        ),
        (
            "waypoint modify bob style reset",
            icon("minecraft:default", Some(0x01_02_03)),
            "commands.waypoint.modify.style",
            None,
        ),
    ];
    for (command, expected, key, text_arg) in cases {
        let mut fixture =
            world(&players).with_player_waypoint("bob", icon("minecraft:pin", Some(0x01_02_03)));
        let run = ran(command, &mut fixture);
        assert_eq!(fixture.player_waypoint_icon("bob"), &expected, "{command}");
        assert_eq!(run.result, Some(0), "{command}: {run:?}");
        assert_eq!(keys(&run), [key], "{command}");
        assert!(!run.feedback[0].inform_admins);
        match text_arg {
            Some(expected) => assert_eq!(component_text(&arguments(&run, 0)[0]), expected),
            None => assert!(arguments(&run, 0).is_empty()),
        }
    }
}

#[test]
fn named_reset_alias_reports_failure_but_exact_reset_resets() {
    let players = [user("alice", 1)];
    for command in [
        "waypoint modify alice color RESET",
        "waypoint modify alice color r_e_s_e_t",
    ] {
        let invalid = ran(
            command,
            world(&players).with_player_waypoint("alice", icon("minecraft:default", Some(7))),
        );
        assert_eq!(invalid.result, None, "{command}");
        assert_eq!(keys(&invalid), ["command.failed"], "{command}");
    }

    let mut fixture =
        world(&players).with_player_waypoint("alice", icon("minecraft:default", Some(7)));
    let reset = ran("waypoint modify alice color reset", &mut fixture);
    assert_eq!(keys(&reset), ["commands.waypoint.modify.color.reset"]);
    assert_eq!(
        fixture.player_waypoint_icon("alice"),
        &icon("minecraft:default", None)
    );
}

#[test]
fn stored_living_is_valid_and_non_living_is_waypoint_invalid() {
    let zombie = stored_entity(
        9,
        99,
        [2.0, 70.0, 3.0],
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            )
            .unwrap(),
        ),
    );
    let run = ran(
        "waypoint modify 0-0-0-0-9 style set custom",
        world(&[]).with_entities_in(level_key(Java26BuiltinDimension::Overworld), &[zombie]),
    );
    assert_eq!(run.result, Some(0));
    assert_eq!(keys(&run), ["commands.waypoint.modify.style"]);

    let non_living = [stored_entity(
        10,
        100,
        [0.0, 0.0, 0.0],
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(1)),
    )];
    let invalid = ran(
        "waypoint modify 0-0-0-0-a style reset",
        world(&[]).with_entities_in(level_key(Java26BuiltinDimension::Overworld), &non_living),
    );
    assert_eq!(keys(&invalid), ["argument.waypoint.invalid"]);
}

#[test]
fn list_reads_the_source_level_manager_and_decorates_raw_icons() {
    let empty = ran(
        "waypoint list",
        world(&[]).with_level(level_key(Java26BuiltinDimension::Nether)),
    );
    assert_eq!(empty.result, Some(0));
    assert_eq!(keys(&empty), ["commands.waypoint.list.empty"]);
    assert_eq!(arguments(&empty, 0), [text("minecraft:the_nether")]);

    let mut fixture = world(&[user("alice", 1), user("bob", 2)])
        .with_player_position("alice", [1.0, 64.0, -3.0])
        .with_player_position("bob", [-2.0, 5.0, 8.0])
        .with_player_waypoint("alice", icon("minecraft:default", None))
        .with_player_waypoint("bob", icon("minecraft:pin", Some(0x12_34_56)));
    let run = ran("waypoint list", &mut fixture);
    assert_eq!(run.result, Some(2));
    assert_eq!(keys(&run), ["commands.waypoint.list.success"]);
    let args = arguments(&run, 0);
    assert_eq!(args[0], NbtSummaryValue::Int(2));
    assert_eq!(args[1], text("minecraft:overworld"));
    let entries = list_components(&args[2]);
    assert_eq!(style_color(entries[0]), Some(0x00ff_ffff));
    assert_eq!(style_color(entries[1]), Some(0x12_34_56));
    assert_eq!(
        click_command(entries[0]).as_deref(),
        Some("/execute in minecraft:overworld run tp @s 1 64 -3")
    );
    assert_eq!(
        click_command(entries[1]).as_deref(),
        Some("/execute in minecraft:overworld run tp @s -2 5 8")
    );
    assert_eq!(
        hover_key(entries[0]).as_deref(),
        Some("chat.coordinates.tooltip")
    );
}

#[test]
fn root_permission_order_and_parser_shapes_match_the_catalog() {
    let denied = refused("waypoint list", CommandPermission::Moderator);
    assert_eq!(keys(&denied), ["command.unknown.command", CURSOR_CONTEXT]);
    assert_eq!(
        ran("waypoint list", CommandTestWorld::default()).result,
        Some(0)
    );
    assert_eq!(catalog::IMPLEMENTED_COMMANDS.len(), 87);
    let waypoint = vanilla_command_lookup(&["waypoint"]).unwrap();
    let weather = vanilla_command_lookup(&["weather"]).unwrap();
    assert!(waypoint < weather);
    for path in [
        &["waypoint", "modify", "waypoint", "color", "color"][..],
        &["waypoint", "modify", "waypoint", "color", "hex", "color"],
        &["waypoint", "modify", "waypoint", "color", "reset"],
        &["waypoint", "modify", "waypoint", "style", "reset"],
        &["waypoint", "modify", "waypoint", "style", "set", "style"],
    ] {
        assert!(vanilla_command_lookup(path).is_some(), "{path:?}");
    }
}

fn component_text(component: &NbtSummaryValue) -> &bbb_protocol::JavaString {
    match component {
        NbtSummaryValue::String(text) => text,
        NbtSummaryValue::Compound(fields) => fields
            .iter()
            .find(|field| field.name == "text")
            .and_then(|field| match &field.value {
                NbtSummaryValue::String(text) => Some(text),
                _ => None,
            })
            .unwrap(),
        other => panic!("component has no text: {other:?}"),
    }
}

fn list_components(component: &NbtSummaryValue) -> Vec<&NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = component else {
        return vec![component];
    };
    let Some(NbtSummaryValue::List(children)) = fields
        .iter()
        .find(|field| field.name == "extra")
        .map(|field| &field.value)
    else {
        return vec![component];
    };
    children.iter().step_by(2).collect()
}

fn field<'a>(component: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = component else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

fn style_color(component: &NbtSummaryValue) -> Option<u32> {
    let NbtSummaryValue::String(color) = field(component, "color")? else {
        return None;
    };
    color
        .with_utf8(|color| u32::from_str_radix(color.strip_prefix('#')?, 16).ok())
        .flatten()
}

fn click_command(component: &NbtSummaryValue) -> Option<String> {
    let event = field(component, "click_event")?;
    let NbtSummaryValue::String(command) = field(event, "command")? else {
        return None;
    };
    Some(command.try_to_string().unwrap())
}

fn hover_key(component: &NbtSummaryValue) -> Option<String> {
    let event = field(component, "hover_event")?;
    let value = field(event, "value")?;
    let NbtSummaryValue::String(key) = field(value, "translate")? else {
        return None;
    };
    Some(key.try_to_string().unwrap())
}
