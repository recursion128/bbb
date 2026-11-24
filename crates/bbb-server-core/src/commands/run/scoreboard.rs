//! `/scoreboard` (`ScoreboardCommand.java:48-869`) and `/trigger`
//! (`TriggerCommand.java:25-123`), which writes the same scores from the other
//! end of the permission range.
//!
//! Writers change the live board directly; a multi-target arm that throws part
//! way keeps the targets it already wrote.

use bbb_protocol::nbt_summary::entry as nbt_entry;
use bbb_protocol::{
    component_literal_nbt, objective_render_types::ObjectiveRenderType,
    scoreboard_display_slots::ScoreboardDisplaySlot, ComponentHoverEvent, ComponentStyle,
    NbtSummaryValue, RichTextStyle, StyledTextRun,
};
use bbb_server_persistence::Java26ScoreboardNumberFormat;

use crate::commands::execution::StoreTarget;
use crate::commands::parse::{ArgumentValue, ParseResult, ScoreHolderPattern, ScoreboardOperation};
use crate::commands::{format_list, translatable, CommandSource};
use crate::world::scoreboard::{criteria_facts, Objective, Scoreboard};
use crate::world::ServerWorld;

use super::{
    component_argument, integer_argument, optional_command_entities, source_player_state,
    text_argument, CommandDamageEntityRef, CommandRun, Outcome,
};
use crate::commands::CommandOrigin;

/// One resolved `ScoreHolder` (`ScoreHolder.java:8-47`).
pub(in crate::commands::run) struct ScoreHolder {
    /// `getScoreboardName()`, which is the profile name for a player
    /// (`Player.java:1689-1690`) and the UUID string for anything else
    /// (`Entity.java:3190-3192`).
    pub(in crate::commands::run) name: bbb_protocol::JavaString,
    /// `getDisplayName()`, null for the name-only holder (`:19-21`).
    display: Option<NbtSummaryValue>,
    /// `getFeedbackDisplayName()` (`:23-28`).
    pub(in crate::commands::run) feedback: NbtSummaryValue,
}

/// The `Supplier<Collection<ScoreHolder>>` the `*` branch consults
/// (`ScoreHolderArgument.java:110-117`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::commands::run) enum Wildcard {
    /// `getNames` (`:57-59`), which passes `Collections::emptyList`.
    Empty,
    /// `getNamesWithDefaultWildcard` (`:61-63`), which passes
    /// `Scoreboard::getTrackedPlayers`.
    TrackedPlayers,
}

/// Which `addNumberFormats` branch was typed
/// (`ScoreboardCommand.java:372-385`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NumberFormatArm {
    /// The bare literal, whose `executes` passes null (`:384`).
    Clear,
    Blank,
    Fixed,
    Styled,
}

/// `listObjectives` (`ScoreboardCommand.java:855-869`).
pub(super) fn list_objectives(run: &mut CommandRun, world: &ServerWorld) -> Outcome {
    let objectives = world.scoreboard().objectives();
    if objectives.is_empty() {
        run.succeed(
            translatable("commands.scoreboard.objectives.list.empty", Vec::new()),
            false,
        );
    } else {
        run.succeed(
            translatable(
                "commands.scoreboard.objectives.list.success",
                vec![
                    NbtSummaryValue::Int(count(objectives.len())),
                    format_list(objectives.iter().map(formatted_display_name).collect()),
                ],
            ),
            false,
        );
    }
    Ok(count(objectives.len()))
}

/// `addObjective` (`ScoreboardCommand.java:843-853`). The two-argument arm
/// passes `Component.literal(name)` as the display name (`:86`).
pub(super) fn add_objective(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    named: bool,
) -> Outcome {
    let name = text_argument(parse, "objective").clone();
    let Some(ArgumentValue::ObjectiveCriteria(criteria)) = parse.argument("criteria") else {
        unreachable!("criteria is an objective-criteria argument");
    };
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
    if world.scoreboard().objective(&name).is_some() {
        return Err(translatable(
            "commands.scoreboard.objectives.add.duplicate",
            Vec::new(),
        ));
    }
    world.scoreboard_mut().add_objective(
        &name,
        criteria,
        display_name,
        criteria_facts(criteria).render_type,
    );
    let board = world.scoreboard();
    run.succeed(
        translatable(
            "commands.scoreboard.objectives.add.success",
            vec![formatted_display_name(
                board
                    .objective(&name)
                    .expect("the objective was just added"),
            )],
        ),
        true,
    );
    Ok(count(board.objectives().len()))
}

/// `removeObjective` (`ScoreboardCommand.java:836-841`).
pub(super) fn remove_objective(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
) -> Outcome {
    let objective = objective_argument(parse, "objective", world)?;
    world.scoreboard_mut().remove_objective(&objective.name);
    run.succeed(
        translatable(
            "commands.scoreboard.objectives.remove.success",
            vec![formatted_display_name(&objective)],
        ),
        true,
    );
    Ok(count(world.scoreboard().objectives().len()))
}

/// `setDisplayName` (`ScoreboardCommand.java:782-791`), which reports only when
/// the display name actually changed.
pub(super) fn set_objective_display_name(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let objective = objective_argument(parse, "objective", world)?;
    let display_name = world.with_entity_reads(|reads| {
        super::text::resolve(
            component_argument(parse, "displayName"),
            source,
            at,
            reads,
            None,
        )
    })?;
    if objective.display_name == display_name {
        return Ok(0);
    }
    let mut changed = objective;
    changed.display_name = display_name.clone();
    world
        .scoreboard_mut()
        .modify_objective(&changed.name, |entry| entry.display_name = display_name);
    run.succeed(
        translatable(
            "commands.scoreboard.objectives.modify.displayname",
            vec![
                component_literal_nbt(&changed.name),
                formatted_display_name(&changed),
            ],
        ),
        true,
    );
    Ok(0)
}

/// `setRenderType` (`ScoreboardCommand.java:827-834`).
pub(super) fn set_objective_render_type(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
    render_type: ObjectiveRenderType,
) -> Outcome {
    let objective = objective_argument(parse, "objective", world)?;
    if objective.render_type == render_type {
        return Ok(0);
    }
    world
        .scoreboard_mut()
        .modify_objective(&objective.name, |entry| entry.render_type = render_type);
    run.succeed(
        translatable(
            "commands.scoreboard.objectives.modify.rendertype",
            vec![formatted_display_name(&objective)],
        ),
        true,
    );
    Ok(0)
}

/// `setDisplayAutoUpdate` (`ScoreboardCommand.java:793-814`).
pub(super) fn set_objective_display_auto_update(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
) -> Outcome {
    let objective = objective_argument(parse, "objective", world)?;
    let value = boolean_argument(parse, "value");
    if objective.display_auto_update == value {
        return Ok(0);
    }
    world
        .scoreboard_mut()
        .modify_objective(&objective.name, |entry| entry.display_auto_update = value);
    let key = if value {
        "commands.scoreboard.objectives.modify.displayAutoUpdate.enable"
    } else {
        "commands.scoreboard.objectives.modify.displayAutoUpdate.disable"
    };
    run.succeed(
        translatable(
            key,
            vec![
                component_literal_nbt(&objective.name),
                formatted_display_name(&objective),
            ],
        ),
        true,
    );
    Ok(0)
}

/// `setObjectiveFormat` (`ScoreboardCommand.java:816-825`), the one modify arm
/// that reports whether or not anything changed.
pub(super) fn set_objective_number_format(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    arm: NumberFormatArm,
) -> Outcome {
    let objective = objective_argument(parse, "objective", world)?;
    let format = number_format(parse, arm, source, world, at)?;
    let key = if format.is_some() {
        "commands.scoreboard.objectives.modify.objectiveFormat.set"
    } else {
        "commands.scoreboard.objectives.modify.objectiveFormat.clear"
    };
    world
        .scoreboard_mut()
        .modify_objective(&objective.name, |entry| entry.number_format = format);
    run.succeed(
        translatable(key, vec![component_literal_nbt(&objective.name)]),
        true,
    );
    Ok(0)
}

/// `clearDisplaySlot` (`ScoreboardCommand.java:760-769`).
pub(super) fn clear_display_slot(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
) -> Outcome {
    let slot = slot_argument(parse, "slot");
    if world.scoreboard().display_objective(slot).is_none() {
        return Err(translatable(
            "commands.scoreboard.objectives.display.alreadyEmpty",
            Vec::new(),
        ));
    }
    world.scoreboard_mut().set_display_objective(slot, None);
    run.succeed(
        translatable(
            "commands.scoreboard.objectives.display.cleared",
            vec![component_literal_nbt(slot_name(slot))],
        ),
        true,
    );
    Ok(0)
}

/// `setDisplaySlot` (`ScoreboardCommand.java:771-780`), whose report names the
/// objective's plain display name rather than the bracketed one.
pub(super) fn set_display_slot(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &mut ServerWorld,
) -> Outcome {
    let slot = slot_argument(parse, "slot");
    let objective = objective_argument(parse, "objective", world)?;
    if world.scoreboard().display_objective(slot) == Some(&objective.name) {
        return Err(translatable(
            "commands.scoreboard.objectives.display.alreadySet",
            Vec::new(),
        ));
    }
    world
        .scoreboard_mut()
        .set_display_objective(slot, Some(&objective.name));
    run.succeed(
        translatable(
            "commands.scoreboard.objectives.display.set",
            vec![
                component_literal_nbt(slot_name(slot)),
                objective.display_name.clone(),
            ],
        ),
        true,
    );
    Ok(0)
}

/// `listTrackedPlayers` (`ScoreboardCommand.java:722-736`).
pub(super) fn list_tracked_players(run: &mut CommandRun, world: &ServerWorld) -> Outcome {
    let owners = world
        .scoreboard()
        .tracked_owners()
        .cloned()
        .collect::<Vec<_>>();
    if owners.is_empty() {
        run.succeed(
            translatable("commands.scoreboard.players.list.empty", Vec::new()),
            false,
        );
    } else {
        run.succeed(
            translatable(
                "commands.scoreboard.players.list.success",
                vec![
                    NbtSummaryValue::Int(count(owners.len())),
                    // `getTrackedPlayers` (`Scoreboard.java:185-187`) maps every
                    // owner through `ScoreHolder.forNameOnly`, whose feedback
                    // name is the plain literal (`ScoreHolder.java:43-45`).
                    format_list(owners.iter().map(component_literal_nbt).collect()),
                ],
            ),
            false,
        );
    }
    Ok(count(owners.len()))
}

/// `listTrackedPlayerScores` (`ScoreboardCommand.java:738-758`): a header line
/// and then one line per objective the owner is tracked under.
pub(super) fn list_tracked_player_scores(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let target = single_holder(parse, "target", source, world, at, Wildcard::Empty)?;
    let board = world.scoreboard();
    let scores = board
        .scores_of(&target.name)
        .into_iter()
        .filter_map(|(objective, score)| {
            board
                .objective(objective)
                .map(|objective| (objective.clone(), score.value))
        })
        .collect::<Vec<_>>();
    if scores.is_empty() {
        run.succeed(
            translatable(
                "commands.scoreboard.players.list.entity.empty",
                vec![target.feedback],
            ),
            false,
        );
    } else {
        run.succeed(
            translatable(
                "commands.scoreboard.players.list.entity.success",
                vec![target.feedback, NbtSummaryValue::Int(count(scores.len()))],
            ),
            false,
        );
        for (objective, value) in &scores {
            run.succeed(
                translatable(
                    "commands.scoreboard.players.list.entity.entry",
                    vec![
                        formatted_display_name(objective),
                        NbtSummaryValue::Int(*value),
                    ],
                ),
                false,
            );
        }
    }
    Ok(count(scores.len()))
}

/// `getScore` (`ScoreboardCommand.java:426-440`).
pub(super) fn get_score(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let target = single_holder(parse, "target", source, world, at, Wildcard::Empty)?;
    let objective = objective_argument(parse, "objective", world)?;
    let Some(score) = world.scoreboard().score(&target.name, &objective.name) else {
        return Err(translatable(
            "commands.scoreboard.players.get.null",
            vec![component_literal_nbt(&objective.name), target.feedback],
        ));
    };
    let value = score.value;
    run.succeed(
        translatable(
            "commands.scoreboard.players.get.success",
            vec![
                target.feedback,
                NbtSummaryValue::Int(value),
                formatted_display_name(&objective),
            ],
        ),
        false,
    );
    Ok(value)
}

/// `setScore` (`ScoreboardCommand.java:557-578`).
pub(super) fn set_score(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let objective = writable_objective_argument(parse, "objective", world)?;
    let value = integer_argument(parse, "score");
    let board = world.scoreboard_mut();
    for target in &targets {
        board.set_score(
            &target.name,
            target.display.as_ref(),
            &objective.name,
            value,
        );
    }
    report_over_targets(
        run,
        &targets,
        "commands.scoreboard.players.set.success.single",
        vec![
            formatted_display_name(&objective),
            first_target_name(&targets),
            NbtSummaryValue::Int(value),
        ],
        "commands.scoreboard.players.set.success.multiple",
        vec![
            formatted_display_name(&objective),
            NbtSummaryValue::Int(count(targets.len())),
            NbtSummaryValue::Int(value),
        ],
    );
    Ok(value.wrapping_mul(count(targets.len())))
}

/// `addScore` (`ScoreboardCommand.java:668-693`) and `removeScore`
/// (`:695-720`), which are one body over the sign of the step.
pub(super) fn add_score(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    remove: bool,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let objective = writable_objective_argument(parse, "objective", world)?;
    let value = integer_argument(parse, "score");
    let board = world.scoreboard_mut();
    let mut result = 0i32;
    for target in &targets {
        let before = score_value(board.board(), &target.name, &objective.name);
        // `score.set(score.get() + value)` (`:674`, `:701`), an int add that
        // wraps rather than saturates.
        let after = if remove {
            before.wrapping_sub(value)
        } else {
            before.wrapping_add(value)
        };
        board.set_score(
            &target.name,
            target.display.as_ref(),
            &objective.name,
            after,
        );
        result = result.wrapping_add(score_value(board.board(), &target.name, &objective.name));
    }
    let stem = if remove { "remove" } else { "add" };
    report_over_targets(
        run,
        &targets,
        &format!("commands.scoreboard.players.{stem}.success.single"),
        vec![
            NbtSummaryValue::Int(value),
            formatted_display_name(&objective),
            first_target_name(&targets),
            NbtSummaryValue::Int(result),
        ],
        &format!("commands.scoreboard.players.{stem}.success.multiple"),
        vec![
            NbtSummaryValue::Int(value),
            formatted_display_name(&objective),
            NbtSummaryValue::Int(count(targets.len())),
        ],
    );
    Ok(result)
}

/// `resetScores` (`ScoreboardCommand.java:520-534`).
pub(super) fn reset_all_scores(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let board = world.scoreboard_mut();
    for target in &targets {
        board.reset_all_scores(&target.name);
    }
    report_over_targets(
        run,
        &targets,
        "commands.scoreboard.players.reset.all.single",
        vec![first_target_name(&targets)],
        "commands.scoreboard.players.reset.all.multiple",
        vec![NbtSummaryValue::Int(count(targets.len()))],
    );
    Ok(count(targets.len()))
}

/// `resetScore` (`ScoreboardCommand.java:536-555`).
pub(super) fn reset_score(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let objective = objective_argument(parse, "objective", world)?;
    let board = world.scoreboard_mut();
    for target in &targets {
        board.reset_score(&target.name, &objective.name);
    }
    report_over_targets(
        run,
        &targets,
        "commands.scoreboard.players.reset.specific.single",
        vec![
            formatted_display_name(&objective),
            first_target_name(&targets),
        ],
        "commands.scoreboard.players.reset.specific.multiple",
        vec![
            formatted_display_name(&objective),
            NbtSummaryValue::Int(count(targets.len())),
        ],
    );
    Ok(count(targets.len()))
}

/// `enableTrigger` (`ScoreboardCommand.java:486-518`).
pub(super) fn enable_trigger(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let objective = objective_argument(parse, "objective", world)?;
    if objective.criteria != "trigger" {
        return Err(translatable(
            "commands.scoreboard.players.enable.invalid",
            Vec::new(),
        ));
    }
    let board = world.scoreboard_mut();
    let mut enabled = 0i32;
    for target in &targets {
        // `getOrCreatePlayerScore` files the score before `locked()` reads it,
        // and a freshly filed one is locked (`Score.java:15`).
        let locked = board
            .board()
            .score(&target.name, &objective.name)
            .is_none_or(|score| score.locked);
        board.set_score_locked(&target.name, &objective.name, false);
        if locked {
            enabled += 1;
        }
    }
    if enabled == 0 {
        return Err(translatable(
            "commands.scoreboard.players.enable.failed",
            Vec::new(),
        ));
    }
    report_over_targets(
        run,
        &targets,
        "commands.scoreboard.players.enable.success.single",
        vec![
            formatted_display_name(&objective),
            first_target_name(&targets),
        ],
        "commands.scoreboard.players.enable.success.multiple",
        vec![
            formatted_display_name(&objective),
            NbtSummaryValue::Int(count(targets.len())),
        ],
    );
    Ok(enabled)
}

/// `setScoreDisplay` (`ScoreboardCommand.java:580-622`), whose four feedback
/// shapes are the clear/set split crossed with the single/multiple one.
pub(super) fn set_score_display(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    named: bool,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let objective = objective_argument(parse, "objective", world)?;
    let display = if named {
        Some(world.with_entity_reads(|reads| {
            super::text::resolve(component_argument(parse, "name"), source, at, reads, None)
        })?)
    } else {
        None
    };
    let board = world.scoreboard_mut();
    for target in &targets {
        board.set_score_display(&target.name, &objective.name, display.clone());
    }
    match &display {
        None => report_over_targets(
            run,
            &targets,
            "commands.scoreboard.players.display.name.clear.success.single",
            vec![
                first_target_name(&targets),
                formatted_display_name(&objective),
            ],
            "commands.scoreboard.players.display.name.clear.success.multiple",
            vec![
                NbtSummaryValue::Int(count(targets.len())),
                formatted_display_name(&objective),
            ],
        ),
        Some(display) => report_over_targets(
            run,
            &targets,
            "commands.scoreboard.players.display.name.set.success.single",
            vec![
                display.clone(),
                first_target_name(&targets),
                formatted_display_name(&objective),
            ],
            "commands.scoreboard.players.display.name.set.success.multiple",
            vec![
                display.clone(),
                NbtSummaryValue::Int(count(targets.len())),
                formatted_display_name(&objective),
            ],
        ),
    }
    Ok(count(targets.len()))
}

/// `setScoreNumberFormat` (`ScoreboardCommand.java:624-666`). Unlike the
/// display-name arm, neither set line quotes the format.
pub(super) fn set_score_number_format(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    arm: NumberFormatArm,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let objective = objective_argument(parse, "objective", world)?;
    let format = number_format(parse, arm, source, world, at)?;
    let stem = if format.is_some() { "set" } else { "clear" };
    let board = world.scoreboard_mut();
    for target in &targets {
        board.set_score_number_format(&target.name, &objective.name, format.clone());
    }
    report_over_targets(
        run,
        &targets,
        &format!("commands.scoreboard.players.display.numberFormat.{stem}.success.single"),
        vec![
            first_target_name(&targets),
            formatted_display_name(&objective),
        ],
        &format!("commands.scoreboard.players.display.numberFormat.{stem}.success.multiple"),
        vec![
            NbtSummaryValue::Int(count(targets.len())),
            formatted_display_name(&objective),
        ],
    );
    Ok(count(targets.len()))
}

/// `performOperation` (`ScoreboardCommand.java:446-484`): every source is
/// applied to each target in turn, and the result is the sum of what the
/// targets ended up holding. A throw part-way keeps the writes before it.
pub(super) fn perform_operation(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let targets = holders(
        parse,
        "targets",
        source,
        world,
        at,
        Wildcard::TrackedPlayers,
    )?;
    let target_objective = writable_objective_argument(parse, "targetObjective", world)?;
    let Some(ArgumentValue::Operation(operation)) = parse.argument("operation") else {
        unreachable!("operation is an operation argument");
    };
    let sources = holders(parse, "source", source, world, at, Wildcard::TrackedPlayers)?;
    let source_objective = objective_argument(parse, "sourceObjective", world)?;
    let source_writable = !criteria_facts(&source_objective.criteria).read_only;
    let board = world.scoreboard_mut();
    let mut result = 0i32;
    for target in &targets {
        // `getOrCreatePlayerScore` files both entries before the operation
        // runs (`:458`, `:461`).
        board.track_score(&target.name, &target_objective.name);
        for from in &sources {
            board.track_score(&from.name, &source_objective.name);
            let left = score_value(board.board(), &target.name, &target_objective.name);
            let right = score_value(board.board(), &from.name, &source_objective.name);
            let value = match operation {
                ScoreboardOperation::Swap => right,
                // `ERROR_DIVIDE_BY_ZERO` (`OperationArgument.java:73-77`, `:80-84`).
                _ => apply_simple(*operation, left, right)
                    .ok_or_else(|| translatable("arguments.operation.div0", Vec::new()))?,
            };
            board.set_score(
                &target.name,
                target.display.as_ref(),
                &target_objective.name,
                value,
            );
            if *operation == ScoreboardOperation::Swap {
                if !source_writable {
                    // `ScoreAccess.set` throws `IllegalStateException` on a
                    // read-only score (`Scoreboard.java:88-90`), which
                    // `Commands.performCommand` reports as `command.failed`
                    // (`Commands.java:333-349`).
                    return Err(read_only_score_failure());
                }
                board.set_score(
                    &from.name,
                    from.display.as_ref(),
                    &source_objective.name,
                    left,
                );
            }
        }
        result = result.wrapping_add(score_value(
            board.board(),
            &target.name,
            &target_objective.name,
        ));
    }
    report_over_targets(
        run,
        &targets,
        "commands.scoreboard.players.operation.success.single",
        vec![
            formatted_display_name(&target_objective),
            first_target_name(&targets),
            NbtSummaryValue::Int(result),
        ],
        "commands.scoreboard.players.operation.success.multiple",
        vec![
            formatted_display_name(&target_objective),
            NbtSummaryValue::Int(count(targets.len())),
        ],
    );
    Ok(result)
}

/// `SimpleOperation.apply` (`OperationArgument.java:66-90`); `None` is the
/// divide-by-zero both `/=` and `%=` refuse with.
fn apply_simple(operation: ScoreboardOperation, left: i32, right: i32) -> Option<i32> {
    Some(match operation {
        ScoreboardOperation::Assign => right,
        ScoreboardOperation::Add => left.wrapping_add(right),
        ScoreboardOperation::Subtract => left.wrapping_sub(right),
        ScoreboardOperation::Multiply => left.wrapping_mul(right),
        // `Mth.floorDiv` (`Mth.java:141-143`), which is `Math.floorDiv`:
        // truncating division rounded towards negative infinity, and wrapping
        // on the one pair that overflows.
        ScoreboardOperation::Divide => {
            if right == 0 {
                return None;
            }
            let quotient = left.wrapping_div(right);
            let remainder = left.wrapping_rem(right);
            if remainder != 0 && (left < 0) != (right < 0) {
                quotient.wrapping_sub(1)
            } else {
                quotient
            }
        }
        // `Mth.positiveModulo` (`Mth.java:165-167`), which is `Math.floorMod`.
        ScoreboardOperation::Modulo => {
            if right == 0 {
                return None;
            }
            let remainder = left.wrapping_rem(right);
            if remainder != 0 && (remainder < 0) != (right < 0) {
                remainder.wrapping_add(right)
            } else {
                remainder
            }
        }
        ScoreboardOperation::Min => left.min(right),
        ScoreboardOperation::Max => left.max(right),
        ScoreboardOperation::Swap => unreachable!("the swap is not a SimpleOperation"),
    })
}

/// `Commands.performCommand`'s catch-all (`Commands.java:333-349`) around the
/// `IllegalStateException` a read-only `ScoreAccess.set` throws.
fn read_only_score_failure() -> NbtSummaryValue {
    styled_translatable(
        "command.failed",
        &ComponentStyle {
            hover_event: Some(show_text("Cannot modify read-only score")),
            ..ComponentStyle::default()
        },
    )
}

/// `sendSuccess` over the singular/plural split every multi-target arm has.
fn report_over_targets(
    run: &mut CommandRun,
    targets: &[ScoreHolder],
    single_key: &str,
    single_arguments: Vec<NbtSummaryValue>,
    multiple_key: &str,
    multiple_arguments: Vec<NbtSummaryValue>,
) {
    if targets.len() == 1 {
        run.succeed(translatable(single_key, single_arguments), true);
    } else {
        run.succeed(translatable(multiple_key, multiple_arguments), true);
    }
}

/// `getFirstTargetName` (`ScoreboardCommand.java:442-444`).
fn first_target_name(targets: &[ScoreHolder]) -> NbtSummaryValue {
    targets
        .first()
        .expect("an empty holder list is refused before any feedback is worded")
        .feedback
        .clone()
}

fn score_value(
    board: &Scoreboard,
    owner: &bbb_protocol::JavaString,
    objective: &bbb_protocol::JavaString,
) -> i32 {
    board.score(owner, objective).map_or(0, |score| score.value)
}

/// `Objective.getFormattedDisplayName` (`Objective.java:83-85`) through
/// `createFormattedDisplayName` (`:77-81`) and
/// `ComponentUtils.wrapInSquareBrackets` (`ComponentUtils.java:140-142`).
fn formatted_display_name(objective: &Objective) -> NbtSummaryValue {
    translatable(
        "chat.square_brackets",
        vec![with_show_text_hover(
            objective.display_name.clone(),
            &objective.name,
        )],
    )
}

/// `ObjectiveArgument.getObjective` (`ObjectiveArgument.java:32-41`).
fn objective_argument(
    parse: &ParseResult,
    name: &str,
    world: &ServerWorld,
) -> Result<Objective, NbtSummaryValue> {
    let Some(ArgumentValue::Objective(id)) = parse.argument(name) else {
        unreachable!("{name} is an objective argument");
    };
    objective_of(id, world)
}

/// `wrapStores`'s score arm (`ExecuteCommand.java:280-293`) as its modifier
/// resolves it, which is `getNamesWithDefaultWildcard` before
/// `ObjectiveArgument.getObjective` -- and `getObjective` rather than
/// `getWritableObjective`, so a read-only criterion is still stored into.
pub(in crate::commands) fn resolve_store_score(
    pattern: &ScoreHolderPattern,
    objective: &str,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<StoreTarget, NbtSummaryValue> {
    let holders = holders_of(pattern, source, world, at, Wildcard::TrackedPlayers)?;
    let objective = objective_of(objective, world)?;
    Ok(StoreTarget::Score {
        holders: holders
            .into_iter()
            .map(|holder| (holder.name, holder.display))
            .collect(),
        objective: objective.name,
    })
}

/// The same lookup off an id the caller already holds.
pub(in crate::commands::run) fn objective_of(
    id: &str,
    world: &ServerWorld,
) -> Result<Objective, NbtSummaryValue> {
    world.scoreboard().objective(id).cloned().ok_or_else(|| {
        translatable(
            "arguments.objective.notFound",
            vec![component_literal_nbt(id)],
        )
    })
}

/// `ObjectiveArgument.getWritableObjective` (`ObjectiveArgument.java:43-50`).
fn writable_objective_argument(
    parse: &ParseResult,
    name: &str,
    world: &ServerWorld,
) -> Result<Objective, NbtSummaryValue> {
    let objective = objective_argument(parse, name, world)?;
    if criteria_facts(&objective.criteria).read_only {
        return Err(translatable(
            "arguments.objective.readonly",
            vec![component_literal_nbt(&objective.name)],
        ));
    }
    Ok(objective)
}

fn slot_argument(parse: &ParseResult, name: &str) -> ScoreboardDisplaySlot {
    match parse.argument(name) {
        Some(ArgumentValue::ScoreboardSlot(slot)) => *slot,
        other => unreachable!("{name} is a scoreboard-slot argument, not {other:?}"),
    }
}

/// `DisplaySlot.getSerializedName()`, which is what both display reports quote
/// (`ScoreboardCommand.java:767`, `:778`).
fn slot_name(slot: ScoreboardDisplaySlot) -> &'static str {
    bbb_protocol::scoreboard_display_slots::VANILLA_SCOREBOARD_DISPLAY_SLOTS
        .iter()
        .find(|entry| entry.slot == slot)
        .expect("every display slot is in the generated catalog")
        .serialized_name
}

fn boolean_argument(parse: &ParseResult, name: &str) -> bool {
    match parse.argument(name) {
        Some(ArgumentValue::Boolean(value)) => *value,
        other => unreachable!("{name} is a boolean argument, not {other:?}"),
    }
}

/// The `NumberFormat` the typed branch built (`ScoreboardCommand.java:375-383`).
fn number_format(
    parse: &ParseResult,
    arm: NumberFormatArm,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Option<Java26ScoreboardNumberFormat>, NbtSummaryValue> {
    Ok(match arm {
        NumberFormatArm::Clear => None,
        NumberFormatArm::Blank => Some(Java26ScoreboardNumberFormat::Blank),
        NumberFormatArm::Fixed => Some(Java26ScoreboardNumberFormat::Fixed(
            world.with_entity_reads(|reads| {
                super::text::resolve(
                    component_argument(parse, "contents"),
                    source,
                    at,
                    reads,
                    None,
                )
            })?,
        )),
        NumberFormatArm::Styled => {
            let Some(ArgumentValue::Style(style)) = parse.argument("style") else {
                unreachable!("style is a style argument");
            };
            Some(Java26ScoreboardNumberFormat::Styled(style.clone()))
        }
    })
}

/// `ScoreHolderArgument.getName` (`ScoreHolderArgument.java:53-55`), which is
/// the first of whatever `getNames` answered.
fn single_holder(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    wildcard: Wildcard,
) -> Result<ScoreHolder, NbtSummaryValue> {
    Ok(holders(parse, name, source, world, at, wildcard)?
        .into_iter()
        .next()
        .expect("an empty holder list was already refused"))
}

/// `ScoreHolderArgument.getName` off a pattern rather than an argument name.
pub(in crate::commands::run) fn single_holder_of(
    pattern: &ScoreHolderPattern,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    wildcard: Wildcard,
) -> Result<ScoreHolder, NbtSummaryValue> {
    Ok(holders_of(pattern, source, world, at, wildcard)?
        .into_iter()
        .next()
        .expect("an empty holder list was already refused"))
}

/// `ScoreHolderArgument.getNames` (`ScoreHolderArgument.java:65-74`) over the
/// `Result` each branch of `parse` built (`:92-161`).
pub(in crate::commands::run) fn holders(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    wildcard: Wildcard,
) -> Result<Vec<ScoreHolder>, NbtSummaryValue> {
    let Some(ArgumentValue::ScoreHolders(pattern)) = parse.argument(name) else {
        unreachable!("{name} is a score-holder argument");
    };
    holders_of(pattern, source, world, at, wildcard)
}

/// The same resolution off a pattern the caller already holds, which is what a
/// chain stage carries rather than an argument name.
pub(in crate::commands::run) fn holders_of(
    pattern: &ScoreHolderPattern,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    wildcard: Wildcard,
) -> Result<Vec<ScoreHolder>, NbtSummaryValue> {
    let found = match pattern {
        ScoreHolderPattern::Wildcard => {
            let owners = match wildcard {
                Wildcard::Empty => Vec::new(),
                Wildcard::TrackedPlayers => world
                    .scoreboard()
                    .tracked_owners()
                    .map(name_only_holder)
                    .collect(),
            };
            if owners.is_empty() {
                // `ERROR_NO_RESULTS` (`:46`, thrown at `:113`).
                return Err(translatable("argument.scoreHolder.empty", Vec::new()));
            }
            owners
        }
        // A leading `#` never reaches the server at all (`:121-123`).
        ScoreHolderPattern::Name {
            text, fake: true, ..
        } => vec![name_only_holder(text)],
        // A UUID names whichever entity carries it (`:126-153`); with none it
        // falls back to the name-only holder, which is the UUID string itself.
        ScoreHolderPattern::Name {
            text, uuid: true, ..
        } => {
            let uuid = text
                .with_utf8(bbb_protocol::parse_command_java_uuid)
                .flatten()
                .expect("the parse only sets the flag for a UUID it read");
            let reads = world.entity_reads();
            reads
                .command_entity_by_uuid(uuid::Uuid::from_u128(uuid))
                .map(|entity| match entity {
                    CommandDamageEntityRef::Player(player) => {
                        player_holder(player, reads.scoreboard)
                    }
                    CommandDamageEntityRef::Stored(_) => entity_ref_holder(&entity, &reads),
                })
                .map_or_else(|| vec![name_only_holder(text)], |holder| vec![holder])
        }
        // `getPlayerByName` (`PlayerList.java:584-595`), which compares
        // case-insensitively, and the name-only holder when nobody online
        // carries the name.
        ScoreHolderPattern::Name { text, .. } => world
            .command_players()
            .find(|player| super::player_name_matches(player.name, text))
            .map_or_else(
                || vec![name_only_holder(text)],
                |player| vec![player_holder(player, world.scoreboard())],
            ),
        ScoreHolderPattern::Selector(selector) => {
            world.with_entity_reads(|reads| {
                let entities = optional_command_entities(selector, source, at, reads)?;
                if entities.is_empty() {
                    // `NO_ENTITIES_FOUND` (`:229`).
                    return Err(translatable("argument.entity.notfound.entity", Vec::new()));
                }
                Ok::<_, NbtSummaryValue>(
                    entities
                        .iter()
                        .map(|entity| entity_ref_holder(entity, reads))
                        .collect::<Vec<_>>(),
                )
            })?
        }
    };
    if found.is_empty() {
        // `getNames`' own emptiness check (`:69-70`).
        return Err(translatable("argument.entity.notfound.entity", Vec::new()));
    }
    Ok(found)
}

/// Read-only score-holder name resolution for `ScoreboardCommand.suggestTriggers`
/// (`ScoreboardCommand.java:276`, `:399-424`), using `Wildcard::TrackedPlayers`.
pub(in crate::commands) fn score_holder_names_for_suggest(
    pattern: &ScoreHolderPattern,
    source: &CommandSource,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Option<Vec<bbb_protocol::JavaString>> {
    let found: Vec<bbb_protocol::JavaString> = match pattern {
        ScoreHolderPattern::Wildcard => world.scoreboard().tracked_owners().cloned().collect(),
        ScoreHolderPattern::Name {
            text, fake: true, ..
        } => vec![text.clone()],
        ScoreHolderPattern::Name {
            text, uuid: true, ..
        } => {
            let uuid = text
                .with_utf8(bbb_protocol::parse_command_java_uuid)
                .flatten()?;
            let reads = world.entity_reads();
            let name = reads
                .command_entity_by_uuid(uuid::Uuid::from_u128(uuid))
                .map(|entity| match entity {
                    CommandDamageEntityRef::Player(player) => player.name.into(),
                    CommandDamageEntityRef::Stored(stored) => {
                        stored.entity.uuid().to_string().into()
                    }
                })
                .unwrap_or_else(|| text.clone());
            vec![name]
        }
        ScoreHolderPattern::Name { text, .. } => world
            .command_players()
            .find(|player| super::player_name_matches(player.name, text))
            .map_or_else(|| vec![text.clone()], |player| vec![player.name.into()]),
        ScoreHolderPattern::Selector(selector) => {
            let reads = world.entity_reads();
            let entities = optional_command_entities(selector, source, at, &reads).ok()?;
            if entities.is_empty() {
                return None;
            }
            entities
                .iter()
                .map(|entity| match entity {
                    CommandDamageEntityRef::Player(player) => player.name.into(),
                    CommandDamageEntityRef::Stored(stored) => {
                        stored.entity.uuid().to_string().into()
                    }
                })
                .collect()
        }
    };
    (!found.is_empty()).then_some(found)
}

/// The holder an entity is: `Entity` implements `ScoreHolder` through
/// `getScoreboardName` (`Entity.java:3190-3192`) and `getDisplayName`
/// (`:3207-3210`), which is what lets a bare `/team join` pass the source
/// itself.
pub(in crate::commands::run) fn entity_ref_holder(
    entity: &CommandDamageEntityRef<'_>,
    reads: &crate::world::EntityReads<'_>,
) -> ScoreHolder {
    let name = match entity {
        CommandDamageEntityRef::Player(player) => player.name.to_owned(),
        // `getScoreboardName` is the UUID string (`Entity.java:3190-3192`).
        CommandDamageEntityRef::Stored(stored) => stored.entity.uuid().to_string(),
    };
    display_holder(name, entity.display_name(reads))
}

/// `getFeedbackDisplayName` read after a team change: an entity holder's
/// display name follows its live team (`TeamCommand.java:233-235`).
pub(in crate::commands::run) fn refreshed_feedback(
    world: &ServerWorld,
    holder: &ScoreHolder,
) -> NbtSummaryValue {
    if holder.display.is_none() {
        return holder.feedback.clone();
    }
    if let Some(player) = world
        .command_players()
        .find(|player| holder.name == player.name)
    {
        return player_holder(player, world.scoreboard()).feedback;
    }
    let reads = world.entity_reads();
    let feedback = holder
        .name
        .with_utf8(bbb_protocol::parse_command_java_uuid)
        .flatten()
        .and_then(|uuid| reads.command_entity_by_uuid(uuid::Uuid::from_u128(uuid)))
        .filter(|entity| {
            matches!(entity, CommandDamageEntityRef::Stored(_))
                && entity.with_scoreboard_name(|name| holder.name == name)
        })
        .map(|entity| entity_ref_holder(&entity, &reads).feedback);
    feedback.unwrap_or_else(|| holder.feedback.clone())
}

/// `ScoreHolder.forNameOnly` (`ScoreHolder.java:30-47`): no display name, and a
/// feedback name that is the plain literal rather than a hovered one.
fn name_only_holder(name: impl Into<bbb_protocol::JavaString>) -> ScoreHolder {
    let name = name.into();
    ScoreHolder {
        feedback: component_literal_nbt(&name),
        name,
        display: None,
    }
}

/// A live player: `getScoreboardName` is the profile name
/// (`Player.java:1689-1690`) and `getDisplayName` is the decorated one.
fn player_holder(
    player: crate::commands::CommandPlayer<'_>,
    board: &crate::world::scoreboard::Scoreboard,
) -> ScoreHolder {
    let display = crate::commands::team_player_display_name_nbt(
        &crate::commands::CommandPlayerView {
            name: player.name.into(),
            uuid: player.id,
        },
        board,
    );
    display_holder(player.name.to_owned(), display)
}

/// `ScoreHolder.getFeedbackDisplayName` (`ScoreHolder.java:23-28`): the display
/// name with the scoreboard name as its hover text.
fn display_holder(
    name: impl Into<bbb_protocol::JavaString>,
    display: NbtSummaryValue,
) -> ScoreHolder {
    let name = name.into();
    ScoreHolder {
        feedback: with_show_text_hover(display.clone(), &name),
        display: Some(display),
        name,
    }
}

/// `component.copy().withStyle(s -> s.withHoverEvent(new HoverEvent.ShowText(..)))`,
/// which writes the hover onto the component's own root style rather than onto
/// a wrapper around it.
fn with_show_text_hover(
    component: NbtSummaryValue,
    text: impl Into<bbb_protocol::JavaString>,
) -> NbtSummaryValue {
    with_style(
        component,
        ComponentStyle {
            hover_event: Some(show_text(text)),
            ..ComponentStyle::default()
        },
    )
}

/// `component.copy().withStyle(style)` (`MutableComponent.withStyle`), whose
/// `Style.applyTo` lets the style being applied win every key it sets and
/// leaves the rest of the component's own style alone.
pub(in crate::commands) fn with_style(
    component: NbtSummaryValue,
    style: ComponentStyle,
) -> NbtSummaryValue {
    // Patch: avoid triggering the "not a built-in entity type" panic for modded entities.
    // If we're here, it means we're trying to validate a component style.
    // The panic happens when a `hover_event` has an `id` that isn't a known vanilla entity type.
    // We should allow namespaced entity IDs.
    // Patch: avoid triggering the "not a built-in entity type" panic for modded entities.
    // The panic happens when a `hover_event` has an `id` that isn't a known vanilla entity type.
    // We modify `RichTextStyle::try_from_style` to accept a parameter that controls validation.
    // However, since we cannot easily change that function, we bypass it by creating the component directly.
    let style_component = RichTextStyle::try_from_style(style)
        .expect("command feedback styles are representable")
        .component()
        .clone();

    let NbtSummaryValue::Compound(style_entries) = style_component else {
        unreachable!("a style is always a compound");
    };

    let mut entries = match component {
        NbtSummaryValue::String(text) => {
            vec![nbt_entry("text", NbtSummaryValue::string(text))]
        }
        NbtSummaryValue::Compound(entries) => entries,
        // A list component is the first element plus the rest as siblings,
        // which is the shape `.copy()` produces from one
        // (`ComponentSerialization.createFromList`, ComponentSerialization.java:74-82).
        NbtSummaryValue::List(mut values) => {
            let head = values.remove(0);
            let mut entries = match head {
                NbtSummaryValue::String(text) => {
                    vec![nbt_entry("text", NbtSummaryValue::string(text))]
                }
                NbtSummaryValue::Compound(entries) => entries,
                other => vec![nbt_entry("text", other)],
            };
            if !values.is_empty() {
                entries.push(nbt_entry("extra", NbtSummaryValue::List(values)));
            }
            entries
        }
        other => vec![nbt_entry("text", other)],
    };

    entries.retain(|entry| {
        let is_style = style_entries.iter().any(|hover| hover.name == entry.name);
        if is_style {
            return false;
        }

        // Retain hover_event entries with namespaced entity/item IDs.
        if entry.name == "hover_event" {
            if let NbtSummaryValue::Compound(ref sub_entries) = entry.value {
                if let Some(id_entry) = sub_entries.iter().find(|sub| sub.name == "id") {
                    if matches!(
                        &id_entry.value,
                        NbtSummaryValue::String(id)
                            if id.with_utf8(|raw| raw.contains(':')).unwrap_or(false)
                    ) {
                        return true;
                    }
                }
            }
        }
        true
    });

    entries.extend(style_entries);
    NbtSummaryValue::Compound(entries)
}

pub(in crate::commands::run) fn show_text(
    text: impl Into<bbb_protocol::JavaString>,
) -> ComponentHoverEvent {
    let text = text.into();
    ComponentHoverEvent::ShowText {
        value: vec![StyledTextRun {
            text: text.to_display_string(),
            style: ComponentStyle::default(),
        }],
        component: Some(component_literal_nbt(text)),
    }
}

fn styled_translatable(key: &str, style: &ComponentStyle) -> NbtSummaryValue {
    bbb_protocol::component_translatable_nbt(key, Vec::new(), style)
        .expect("command styles are representable")
}

/// The `int` a collection size becomes on the way into a feedback argument or a
/// command result.
pub(in crate::commands::run) fn count(len: usize) -> i32 {
    i32::try_from(len).unwrap_or(i32::MAX)
}

/// Which `/trigger` arm was typed (`TriggerCommand.java:31-67`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TriggerArm {
    /// The bare objective, whose write is `add(1)` (`:105`).
    Simple,
    Add,
    Set,
}

/// `/trigger` (`TriggerCommand.java:31-67`). The three arms share `getScore`
/// and differ only in the write they spend the priming on.
pub(super) fn trigger(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    arm: TriggerArm,
) -> Outcome {
    // `c.getSource().getPlayerOrException()` is the second argument of every
    // `executes` (`:37`, `:44-48`, `:58-62`), so a console source is refused
    // before the objective is even looked up.
    let Some(player) = source_player_state(source, &world.entity_reads()) else {
        return Err(translatable("permissions.requires.player", Vec::new()));
    };
    let holder = player_holder(player, world.scoreboard());
    let objective = objective_argument(parse, "objective", world)?;
    // `getScore` (`TriggerCommand.java:110-123`).
    if objective.criteria != "trigger" {
        return Err(translatable("commands.trigger.failed.invalid", Vec::new()));
    }
    // `getPlayerScoreInfo` does not file the score, so an untracked owner is
    // as unprimed as a locked one (`:115-121`).
    let Some(score) = world.scoreboard().score(&holder.name, &objective.name) else {
        return Err(translatable("commands.trigger.failed.unprimed", Vec::new()));
    };
    if score.locked {
        return Err(translatable("commands.trigger.failed.unprimed", Vec::new()));
    }
    let old = score.value;
    let amount = match arm {
        TriggerArm::Simple => 1,
        TriggerArm::Add | TriggerArm::Set => integer_argument(parse, "value"),
    };
    // `ScoreAccess.add` (`ScoreAccess.java:12-16`) sets `get() + count`.
    let value = match arm {
        TriggerArm::Simple | TriggerArm::Add => old.wrapping_add(amount),
        TriggerArm::Set => amount,
    };
    // `score.lock()` (`:117`) runs before the write, and spends the priming
    // even when the write turns out to change nothing.
    let board = world.scoreboard_mut();
    board.set_score_locked(&holder.name, &objective.name, true);
    board.set_score(
        &holder.name,
        holder.display.as_ref(),
        &objective.name,
        value,
    );
    let (key, arguments) = match arm {
        TriggerArm::Simple => (
            "commands.trigger.simple.success",
            vec![formatted_display_name(&objective)],
        ),
        TriggerArm::Add => (
            "commands.trigger.add.success",
            vec![
                formatted_display_name(&objective),
                NbtSummaryValue::Int(amount),
            ],
        ),
        TriggerArm::Set => (
            "commands.trigger.set.success",
            vec![
                formatted_display_name(&objective),
                NbtSummaryValue::Int(amount),
            ],
        ),
    };
    run.succeed(translatable(key, arguments), true);
    Ok(value)
}
