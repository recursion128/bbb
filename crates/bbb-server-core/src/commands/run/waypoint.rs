//! `/waypoint` list and raw icon mutation.

use bbb_protocol::nbt_summary::entry;
use bbb_protocol::{
    component_styled_literal_nbt, component_translatable_nbt, ComponentClickEvent,
    ComponentHoverEvent, ComponentStyle, NbtSummaryValue,
};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::types::{CommandAttributeTarget, CommandWaypointState};
use crate::commands::{
    chat_formatting_style, format_list, styled_literal, translatable, CommandOrigin, CommandSource,
};
use crate::world::ServerWorld;

use super::scoreboard::show_text;
use super::{
    identifier_argument, single_damage_entity_argument, CommandCx, CommandDamageEntityRef,
    CommandRun, Outcome,
};

const DEFAULT_STYLE: &str = "minecraft:default";

/// `WaypointCommand.listWaypoints` (`WaypointCommand.java:117-150`).
pub(super) fn list_waypoints(run: &mut CommandRun, world: &ServerWorld, at: &CommandOrigin) -> i32 {
    let dimension = at.level.resource_id();
    let waypoints = world.command_level_waypoints(&at.level);
    if waypoints.is_empty() {
        run.succeed(
            translatable(
                "commands.waypoint.list.empty",
                vec![NbtSummaryValue::string(dimension)],
            ),
            false,
        );
        return 0;
    }
    let names = waypoints
        .iter()
        .map(|waypoint| list_display_name(waypoint, dimension))
        .collect();
    let count = i32::try_from(waypoints.len()).unwrap_or(i32::MAX);
    run.succeed(
        translatable(
            "commands.waypoint.list.success",
            vec![
                NbtSummaryValue::Int(count),
                NbtSummaryValue::string(dimension),
                format_list(names),
            ],
        ),
        false,
    );
    count
}

/// `WaypointCommand.java:88-115`: all five mutation bodies return zero.
pub(super) fn modify_waypoint(
    run: &mut CommandRun,
    path: &[&str],
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let target = cx.world.with_entity_reads(|reads| {
        waypoint_target(single_damage_entity_argument(
            parse, "waypoint", source, &cx.origin, reads,
        )?)
    })?;
    let mut icon = cx
        .world
        .command_waypoint(&target)
        .ok_or_else(invalid_waypoint)?
        .icon;
    let manager_level = cx.world.require_level_id(&cx.origin.level);
    let feedback = match path {
        ["waypoint", "modify", "waypoint", "color", "color"] => {
            let Some(ArgumentValue::Color(color)) = parse.argument("color") else {
                unreachable!("color is a named color argument")
            };
            let Some(rgb) = color.color() else {
                cx.world
                    .apply_untrack_waypoint_by_key(&manager_level, target, cx.events);
                return Err(null_color_failure());
            };
            icon.color_rgb = Some(rgb);
            translatable(
                "commands.waypoint.modify.color",
                vec![styled_literal(
                    color.as_str(),
                    &chat_formatting_style(*color),
                )],
            )
        }
        ["waypoint", "modify", "waypoint", "color", "hex", "color"] => {
            let Some(ArgumentValue::HexColor(color)) = parse.argument("color") else {
                unreachable!("color is a hex color argument")
            };
            let rgb = (*color as u32) & 0x00ff_ffff;
            icon.color_rgb = Some(rgb);
            translatable(
                "commands.waypoint.modify.color",
                vec![styled_literal(
                    format!("{rgb:06X}"),
                    &ComponentStyle {
                        color: Some(rgb),
                        ..ComponentStyle::default()
                    },
                )],
            )
        }
        ["waypoint", "modify", "waypoint", "color", "reset"] => {
            icon.color_rgb = None;
            translatable("commands.waypoint.modify.color.reset", Vec::new())
        }
        ["waypoint", "modify", "waypoint", "style", "reset"] => {
            icon.style = DEFAULT_STYLE.to_owned();
            translatable("commands.waypoint.modify.style", Vec::new())
        }
        ["waypoint", "modify", "waypoint", "style", "set", "style"] => {
            icon.style = identifier_argument(parse, "style").to_owned();
            translatable("commands.waypoint.modify.style", Vec::new())
        }
        _ => unreachable!("not a waypoint mutation path: {path:?}"),
    };
    cx.world
        .apply_set_waypoint_icon_by_key(&manager_level, target, icon, cx.events);
    run.succeed(feedback, false);
    Ok(0)
}

fn waypoint_target(
    entity: CommandDamageEntityRef<'_>,
) -> Result<CommandAttributeTarget, NbtSummaryValue> {
    match entity {
        CommandDamageEntityRef::Player(player) => {
            Ok(CommandAttributeTarget::Player(player.target()))
        }
        CommandDamageEntityRef::Stored(stored) if stored.entity.living().is_some() => {
            Ok(CommandAttributeTarget::Stored {
                dimension: stored.dimension.clone(),
                entity_id: stored.entity.id(),
            })
        }
        CommandDamageEntityRef::Stored(_) => Err(invalid_waypoint()),
    }
}

fn invalid_waypoint() -> NbtSummaryValue {
    translatable("argument.waypoint.invalid", Vec::new())
}

/// `Commands.performCommand`'s catch-all (`Commands.java:333-349`) around the
/// null passed to `Optional.of` in `WaypointCommand.java:94-96`.
fn null_color_failure() -> NbtSummaryValue {
    component_translatable_nbt(
        "command.failed",
        Vec::new(),
        &ComponentStyle {
            hover_event: Some(show_text("java.lang.NullPointerException")),
            ..ComponentStyle::default()
        },
    )
    .expect("static command failure is a valid component")
}

fn list_display_name(waypoint: &CommandWaypointState, dimension: &str) -> NbtSummaryValue {
    let pos = waypoint.position;
    apply_root_style(
        waypoint.display_name.clone(),
        ComponentStyle {
            color: Some(waypoint.icon.color_rgb.unwrap_or(0x00ff_ffff) & 0x00ff_ffff),
            click_event: Some(ComponentClickEvent::SuggestCommand {
                command: format!(
                    "/execute in {dimension} run tp @s {} {} {}",
                    pos.x, pos.y, pos.z
                )
                .into(),
            }),
            hover_event: Some(ComponentHoverEvent::ShowText {
                value: Vec::new(),
                component: Some(translatable("chat.coordinates.tooltip", Vec::new())),
            }),
            ..ComponentStyle::default()
        },
    )
}

fn apply_root_style(component: NbtSummaryValue, style: ComponentStyle) -> NbtSummaryValue {
    let NbtSummaryValue::Compound(style_fields) =
        component_styled_literal_nbt("", &style).expect("waypoint list style is valid")
    else {
        unreachable!("a styled literal is a compound")
    };
    let style_fields = style_fields.into_iter().skip(1).collect::<Vec<_>>();
    let mut fields = match component {
        NbtSummaryValue::String(text) => vec![entry("text", NbtSummaryValue::string(text))],
        NbtSummaryValue::Compound(fields) => fields,
        NbtSummaryValue::List(children) => vec![
            entry("text", NbtSummaryValue::string(String::new())),
            entry("extra", NbtSummaryValue::List(children)),
        ],
        _ => unreachable!("waypoint names are validated components"),
    };
    fields.retain(|field| {
        !style_fields
            .iter()
            .any(|replacement| replacement.name == field.name)
    });
    fields.extend(style_fields);
    NbtSummaryValue::Compound(fields)
}
