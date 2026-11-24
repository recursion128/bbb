//! `/team` (`TeamCommand.java:26-441`).
//!
//! Every arm words its feedback from the board *after* the write, which is not
//! a shortcut: `sendSuccess` takes a `Supplier<Component>` and calls it once
//! the setter has already run (`CommandSourceStack.java:464-477`), so
//! `/team modify <t> color red` reports a bracketed name that is already red
//! and `/team modify <t> displayName` reports the name it just set.

use bbb_facts::scoreboard_team_options::{
    TeamCollisionRule, TeamVisibility, VANILLA_TEAM_COLLISION_RULES, VANILLA_TEAM_VISIBILITIES,
};
use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::{component_literal_nbt, ComponentStyle, NbtSummaryValue};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{
    chat_formatting_style, format_and_sort_list, format_list, translatable, CommandSource,
};
use crate::world::scoreboard::PlayerTeam;
use crate::world::ServerWorld;

use super::scoreboard::{
    count, entity_ref_holder, holders, refreshed_feedback, show_text, with_style, ScoreHolder,
    Wildcard,
};
use super::{component_argument, text_argument, CommandRun, Outcome};
use crate::commands::CommandOrigin;

/// `listTeams` (`TeamCommand.java:414-428`), whose list is the unsorted
/// `formatList` over each team's formatted name.
pub(super) fn list_teams(run: &mut CommandRun, world: &ServerWorld) -> Outcome {
    let teams = world.scoreboard().teams();
    if teams.is_empty() {
        run.succeed(
            translatable("commands.team.list.teams.empty", Vec::new()),
            false,
        );
    } else {
        run.succeed(
            translatable(
                "commands.team.list.teams.success",
                vec![
                    NbtSummaryValue::Int(count(teams.len())),
                    format_list(teams.iter().map(formatted_team_name).collect()),
                ],
            ),
            false,
        );
    }
    Ok(count(teams.len()))
}

/// `listMembers` (`:398-412`). The member list goes through the
/// `Collection<String>` overload, which is the sorted green one.
pub(super) fn list_team_members(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let players: Vec<_> = world.scoreboard().team_members(&team.name).collect();
    let size = count(players.len());
    if players.is_empty() {
        run.succeed(
            translatable(
                "commands.team.list.members.empty",
                vec![formatted_team_name(&team)],
            ),
            false,
        );
    } else {
        run.succeed(
            translatable(
                "commands.team.list.members.success",
                vec![
                    formatted_team_name(&team),
                    NbtSummaryValue::Int(size),
                    format_and_sort_list(players),
                ],
            ),
            false,
        );
    }
    Ok(size)
}

/// `createTeam` (`:382-396`). Both arms run `setDisplayName` after
/// `addPlayerTeam`, so even the bare one writes the name a second time -- and
/// the second write is its own broadcast.
pub(super) fn add_team(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    named: bool,
) -> Outcome {
    let name = text_argument(parse, "team").to_owned();
    // The bare arm passes `Component.literal(name)` (`:382-384`).
    let display_name = if named {
        world.with_entity_reads(|reads| {
            super::text::resolve(
                component_argument(parse, "displayName"),
                source,
                at,
                reads,
                None,
            )
        })?
    } else {
        component_literal_nbt(&name)
    };
    if world.scoreboard().team(&name).is_some() {
        return Err(translatable("commands.team.add.duplicate", Vec::new()));
    }
    let board = world.scoreboard_mut();
    board.add_team(&name);
    board.change_team(&name, |team| team.display_name = display_name);
    report_team(
        run,
        world,
        &name,
        "commands.team.add.success",
        Vec::new(),
        true,
    );
    Ok(count(world.scoreboard().teams().len()))
}

/// `deleteTeam` (`:375-380`), which answers with what is left rather than with
/// what it removed.
pub(super) fn remove_team(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    world.scoreboard_mut().remove_team(&team.name);
    run.succeed(
        translatable(
            "commands.team.remove.success",
            vec![formatted_team_name(&team)],
        ),
        true,
    );
    Ok(count(world.scoreboard().teams().len()))
}

/// `emptyTeam` (`:360-373`), which walks a copy of the member list because the
/// removals mutate the set it came from.
pub(super) fn empty_team(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let players: Vec<_> = world
        .scoreboard()
        .team_members(&team.name)
        .cloned()
        .collect();
    if players.is_empty() {
        return Err(translatable("commands.team.empty.unchanged", Vec::new()));
    }
    let board = world.scoreboard_mut();
    for player in &players {
        board.remove_player_from_named_team(player, &team.name);
    }
    run.succeed(
        translatable(
            "commands.team.empty.success",
            vec![
                NbtSummaryValue::Int(count(players.len())),
                formatted_team_name(&team),
            ],
        ),
        true,
    );
    Ok(count(players.len()))
}

/// `joinTeam` (`:253-269`). The team is resolved before the members are, so an
/// unknown team is what a bare `/team join` from the console reports.
pub(super) fn join_team(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    targeted: bool,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let members = members(parse, source, world, at, targeted)?;
    let board = world.scoreboard_mut();
    for member in &members {
        board.add_player_to_team(&member.name, &team.name);
    }
    let formatted = formatted_team_name(&team);
    if members.len() == 1 {
        run.succeed(
            translatable(
                "commands.team.join.success.single",
                vec![refreshed_feedback(world, &members[0]), formatted],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.team.join.success.multiple",
                vec![NbtSummaryValue::Int(count(members.len())), formatted],
            ),
            true,
        );
    }
    Ok(count(members.len()))
}

/// `leaveTeam` (`:237-251`), whose feedback never names a team: a member who
/// was on none is still counted.
pub(super) fn leave_team(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let members = members(parse, source, world, at, true)?;
    let board = world.scoreboard_mut();
    for member in &members {
        board.remove_player_from_team(&member.name);
    }
    if members.len() == 1 {
        run.succeed(
            translatable(
                "commands.team.leave.success.single",
                vec![refreshed_feedback(world, &members[0])],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.team.leave.success.multiple",
                vec![NbtSummaryValue::Int(count(members.len()))],
            ),
            true,
        );
    }
    Ok(count(members.len()))
}

/// `setDisplayName` (`:340-348`), whose guard is component equality rather
/// than an identity check.
pub(super) fn set_team_display_name(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let display_name = world.with_entity_reads(|reads| {
        super::text::resolve(
            component_argument(parse, "displayName"),
            source,
            at,
            reads,
            None,
        )
    })?;
    if team.display_name == display_name {
        return Err(translatable(
            "commands.team.option.name.unchanged",
            Vec::new(),
        ));
    }
    modify(
        run,
        world,
        &team.name,
        |team| team.display_name = display_name,
        "commands.team.option.name.success",
        Vec::new(),
    )
}

/// `setColor` (`:350-358`). The second argument is `ChatFormatting.getName()`
/// (`ChatFormatting.java:97-99`), a bare string rather than a component.
pub(super) fn set_team_color(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let Some(ArgumentValue::Color(color)) = parse.argument("value") else {
        unreachable!("value is a colour argument");
    };
    if team.color == *color {
        return Err(translatable(
            "commands.team.option.color.unchanged",
            Vec::new(),
        ));
    }
    let color = *color;
    modify(
        run,
        world,
        &team.name,
        |team| team.color = color,
        "commands.team.option.color.success",
        vec![NbtSummaryValue::string(color.as_str())],
    )
}

/// `setFriendlyFire` (`:324-338`) and `setFriendlySight` (`:307-322`), which
/// are the same shape over two fields: an unchanged write is refused with the
/// key for the state it is already in, and the success key names the new one.
pub(super) fn set_team_flag(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
    sight: bool,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let allowed = boolean_argument(parse, "allowed");
    let option = if sight {
        "seeFriendlyInvisibles"
    } else {
        "friendlyfire"
    };
    let current = if sight {
        team.see_friendly_invisibles
    } else {
        team.allow_friendly_fire
    };
    let state = if allowed { "enabled" } else { "disabled" };
    if current == allowed {
        let already = if allowed {
            "alreadyEnabled"
        } else {
            "alreadyDisabled"
        };
        return Err(translatable(
            format!("commands.team.option.{option}.{already}"),
            Vec::new(),
        ));
    }
    modify(
        run,
        world,
        &team.name,
        |team| {
            if sight {
                team.see_friendly_invisibles = allowed;
            } else {
                team.allow_friendly_fire = allowed;
            }
        },
        &format!("commands.team.option.{option}.{state}"),
        Vec::new(),
    )
}

/// `setNametagVisibility` (`:271-281`) and `setDeathMessageVisibility`
/// (`:283-293`), whose second argument is `Visibility.getDisplayName()`
/// (`Team.java:83-85`).
pub(super) fn set_team_visibility(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
    visibility: TeamVisibility,
    death_message: bool,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let option = if death_message {
        "deathMessageVisibility"
    } else {
        "nametagVisibility"
    };
    let current = if death_message {
        team.death_message_visibility
    } else {
        team.name_tag_visibility
    };
    if current == visibility {
        return Err(translatable(
            format!("commands.team.option.{option}.unchanged"),
            Vec::new(),
        ));
    }
    modify(
        run,
        world,
        &team.name,
        |team| {
            if death_message {
                team.death_message_visibility = visibility;
            } else {
                team.name_tag_visibility = visibility;
            }
        },
        &format!("commands.team.option.{option}.success"),
        vec![translatable(
            format!("team.visibility.{}", visibility.as_str()),
            Vec::new(),
        )],
    )
}

/// `setCollision` (`:295-305`) over `CollisionRule.getDisplayName()`
/// (`Team.java:56-58`).
pub(super) fn set_team_collision(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
    rule: TeamCollisionRule,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    if team.collision_rule == rule {
        return Err(translatable(
            "commands.team.option.collisionRule.unchanged",
            Vec::new(),
        ));
    }
    modify(
        run,
        world,
        &team.name,
        |team| team.collision_rule = rule,
        "commands.team.option.collisionRule.success",
        vec![translatable(
            format!("team.collision.{}", rule.as_str()),
            Vec::new(),
        )],
    )
}

/// `setPrefix` (`:430-434`) and `setSuffix` (`:436-440`), the two arms with no
/// guard at all: they report the component that was typed rather than the
/// team, they report it quietly, and they answer with one.
pub(super) fn set_team_affix(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    suffix: bool,
) -> Outcome {
    let team = team_argument(parse, "team", world)?;
    let name = if suffix { "suffix" } else { "prefix" };
    let affix = world.with_entity_reads(|reads| {
        super::text::resolve(component_argument(parse, name), source, at, reads, None)
    })?;
    let written = Some(affix.clone());
    world.scoreboard_mut().change_team(&team.name, |team| {
        if suffix {
            team.player_suffix = written;
        } else {
            team.player_prefix = written;
        }
    });
    run.succeed(
        translatable(format!("commands.team.option.{name}.success"), vec![affix]),
        false,
    );
    Ok(1)
}

/// The four `nametagVisibility` and `deathMessageVisibility` literals
/// (`TeamCommand.java:154-168`) are spelled exactly as `Visibility`'s own
/// `name` field (`Team.java:67-70`), so the node name resolves the constant.
pub(super) fn team_visibility(name: &str) -> TeamVisibility {
    VANILLA_TEAM_VISIBILITIES
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.visibility)
        .expect("the published tree names only the four serialized visibilities")
}

/// The same for the four `collisionRule` literals (`:194-208`) against
/// `CollisionRule.name` (`Team.java:40-43`).
pub(super) fn team_collision_rule(name: &str) -> TeamCollisionRule {
    VANILLA_TEAM_COLLISION_RULES
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.collision_rule)
        .expect("the published tree names only the four serialized collision rules")
}

/// One guarded `/team modify` arm: write, then word the feedback off the team
/// as the write left it.
fn modify(
    run: &mut CommandRun,
    world: &mut ServerWorld,
    team: &(impl crate::world::scoreboard::ScoreboardName + ?Sized),
    write: impl FnOnce(&mut PlayerTeam),
    key: &str,
    trailing: Vec<NbtSummaryValue>,
) -> Outcome {
    world.scoreboard_mut().change_team(team, write);
    report_team(run, world, team, key, trailing, true);
    Ok(0)
}

fn report_team(
    run: &mut CommandRun,
    world: &ServerWorld,
    team: &(impl crate::world::scoreboard::ScoreboardName + ?Sized),
    key: &str,
    trailing: Vec<NbtSummaryValue>,
    inform_admins: bool,
) {
    let formatted = world
        .scoreboard()
        .team(team)
        .map(formatted_team_name)
        .expect("the team this run wrote is still on the board");
    let mut arguments = vec![formatted];
    arguments.extend(trailing);
    run.succeed(translatable(key, arguments), inform_admins);
}

/// `PlayerTeam.getFormattedDisplayName` (`PlayerTeam.java:73-81`): the display
/// name under the team's own insertion and hover (`:41`), wrapped in square
/// brackets, and then coloured -- the colour lands on the wrapper rather than
/// on the name inside it.
pub(super) fn formatted_team_name(team: &PlayerTeam) -> NbtSummaryValue {
    let inner = with_style(
        team.display_name.clone(),
        ComponentStyle {
            insertion: Some(team.name.clone()),
            hover_event: Some(show_text(&team.name)),
            ..ComponentStyle::default()
        },
    );
    let wrapped = translatable("chat.square_brackets", vec![inner]);
    if team.color == ChatFormatting::Reset {
        return wrapped;
    }
    with_style(wrapped, chat_formatting_style(team.color))
}

/// `TeamArgument.getTeam` (`TeamArgument.java:29-38`).
fn team_argument(
    parse: &ParseResult,
    name: &str,
    world: &ServerWorld,
) -> Result<PlayerTeam, NbtSummaryValue> {
    let Some(ArgumentValue::Team(id)) = parse.argument(name) else {
        unreachable!("{name} is a team argument");
    };
    world
        .scoreboard()
        .team(id)
        .cloned()
        .ok_or_else(|| translatable("team.notFound", vec![component_literal_nbt(id)]))
}

/// The members `/team join` and `/team leave` write: the score-holder argument
/// when one was typed, and otherwise `getEntityOrException` (`:97`).
fn members(
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    targeted: bool,
) -> Result<Vec<ScoreHolder>, NbtSummaryValue> {
    if targeted {
        return holders(
            parse,
            "members",
            source,
            world,
            at,
            Wildcard::TrackedPlayers,
        );
    }
    // `getEntityOrException` (`CommandSourceStack.java:393-398`) over
    // `ERROR_NOT_ENTITY` (`:53`).
    let reads = world.entity_reads();
    super::source_entity_state(source, &reads)
        .map(|entity| vec![entity_ref_holder(&entity, &reads)])
        .ok_or_else(|| translatable("permissions.requires.entity", Vec::new()))
}

fn boolean_argument(parse: &ParseResult, name: &str) -> bool {
    match parse.argument(name) {
        Some(ArgumentValue::Boolean(value)) => *value,
        other => unreachable!("{name} is a boolean argument, not {other:?}"),
    }
}
