//! `/advancement` (`AdvancementCommands.java:32-458`).

use bbb_protocol::{component_literal_nbt, JavaString, NbtSummaryValue};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::world::scoreboard::Scoreboard;
use bbb_server_datapack::player_advancements::AdvancementTree;

use crate::commands::types::CommandAdvancementAction;
use crate::commands::{
    team_player_display_name_nbt, translatable, CommandPlayerTarget, CommandPlayerView,
    CommandSource,
};

use super::{player_states_argument, resource_argument, view_of, CommandCx, CommandRun, Outcome};

#[derive(Clone, Copy)]
enum Mode {
    Only,
    Through,
    From,
    Until,
    Everything,
}

impl Mode {
    fn parents(self) -> bool {
        matches!(self, Self::Through | Self::Until)
    }

    fn children(self) -> bool {
        matches!(self, Self::Through | Self::From)
    }
}

/// `AdvancementCommands.perform` (`AdvancementCommands.java:220-287`).
fn perform(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    action: CommandAdvancementAction,
    mode: Mode,
) -> Outcome {
    let (targets, ids, single_name) = cx.world.with_entity_reads(|world| {
        let targets: Vec<(CommandPlayerTarget, CommandPlayerView)> =
            player_states_argument(parse, "targets", source, world, &cx.origin)?
                .into_iter()
                .map(|player| (player.target(), view_of(player)))
                .collect();
        let tree = world.advancements;
        let advancements = match mode {
            Mode::Everything => (0..tree.len()).collect(),
            _ => collect_advancements(tree, resource_argument(parse, "advancement"), mode)?,
        };
        let single_name =
            (advancements.len() == 1).then(|| advancement_name(tree, advancements[0]));
        let ids: Vec<String> = advancements
            .into_iter()
            .map(|index| tree.node(index).id().to_owned())
            .collect();
        Ok::<_, NbtSummaryValue>((targets, ids, single_name))
    })?;
    // Grant `everything` is the only arm that passes `showAdvancements=false`
    // (`:116-122`); revoke `everything` uses the default true (`:207-212`).
    let show_advancements =
        !(matches!(mode, Mode::Everything) && action == CommandAdvancementAction::Grant);
    let mut count = 0;
    for (target, _) in &targets {
        count += cx.world.apply_mutate_advancements_slice(
            target.clone(),
            action,
            &ids,
            None,
            show_advancements,
            cx.events,
        );
    }
    report_perform(
        run,
        action,
        &targets,
        single_name,
        ids.len(),
        count,
        cx.world.scoreboard(),
    )
}

/// `AdvancementCommands.performCriterion` (`AdvancementCommands.java:289-336`).
fn perform_criterion(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    action: CommandAdvancementAction,
) -> Outcome {
    let (targets, id, name, criterion) = cx.world.with_entity_reads(|world| {
        let targets: Vec<(CommandPlayerTarget, CommandPlayerView)> =
            player_states_argument(parse, "targets", source, world, &cx.origin)?
                .into_iter()
                .map(|player| (player.target(), view_of(player)))
                .collect();
        let tree = world.advancements;
        let id = resource_argument(parse, "advancement");
        let index = tree.index_of(id).ok_or_else(|| not_found(id))?;
        let criterion = match parse.argument("criterion") {
            Some(ArgumentValue::Text(criterion)) => criterion.clone(),
            other => unreachable!("criterion is a greedy string, not {other:?}"),
        };
        let name = advancement_name(tree, index);
        if !tree
            .node(index)
            .criteria()
            .iter()
            .any(|(existing, _)| existing == &criterion)
        {
            return Err(translatable(
                "commands.advancement.criterionNotFound",
                vec![name, component_literal_nbt(&criterion)],
            ));
        }
        Ok::<_, NbtSummaryValue>((targets, id.to_owned(), name, criterion))
    })?;
    let mut count = 0;
    let ids = [id];
    for (target, _) in &targets {
        count += cx.world.apply_mutate_advancements_slice(
            target.clone(),
            action,
            &ids,
            Some(&criterion),
            true,
            cx.events,
        );
    }
    report_criterion(
        run,
        action,
        &targets,
        name,
        &criterion,
        count,
        cx.world.scoreboard(),
    )
}

/// The twelve `/advancement` executors (`AdvancementCommands.java:41-212`).
pub(super) fn advancement(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    path: &[&str],
) -> Outcome {
    let action = match path[1] {
        "grant" => CommandAdvancementAction::Grant,
        _ => CommandAdvancementAction::Revoke,
    };
    let mode = match &path[3..] {
        ["only", "advancement", "criterion"] => {
            return perform_criterion(run, parse, source, cx, action);
        }
        ["only", "advancement"] => Mode::Only,
        ["from", "advancement"] => Mode::From,
        ["until", "advancement"] => Mode::Until,
        ["through", "advancement"] => Mode::Through,
        ["everything"] => Mode::Everything,
        _ => unreachable!("no advancement executor for {path:?}"),
    };
    perform(run, parse, source, cx, action, mode)
}

/// `ResourceKeyArgument.getAdvancement` (`ResourceKeyArgument.java:105-113`).
fn not_found(id: &str) -> NbtSummaryValue {
    translatable(
        "advancement.advancementNotFound",
        vec![component_literal_nbt(id)],
    )
}

/// `Advancement.name` (`Advancement.java:79-81`): decorated display name when
/// present, otherwise the id literal.
fn advancement_name(tree: &AdvancementTree, index: usize) -> NbtSummaryValue {
    tree.node(index)
        .display()
        .and_then(|display| crate::world::ServerWorld::decorate_advancement_name(display).ok())
        .unwrap_or_else(|| component_literal_nbt(tree.node(index).id()))
}

/// `AdvancementCommands.getAdvancements` (`AdvancementCommands.java:338-359`).
fn collect_advancements(
    tree: &AdvancementTree,
    id: &str,
    mode: Mode,
) -> Result<Vec<usize>, NbtSummaryValue> {
    let Some(target) = tree.index_of(id) else {
        return Err(not_found(id));
    };
    let mut advancements = Vec::new();
    if mode.parents() {
        let mut parent = tree.node(target).parent();
        while let Some(index) = parent {
            advancements.push(index);
            parent = tree.node(index).parent();
        }
    }
    advancements.push(target);
    if mode.children() {
        add_children(tree, target, &mut advancements);
    }
    Ok(advancements)
}

/// `AdvancementCommands.addChildren` (`AdvancementCommands.java:362-367`).
fn add_children(tree: &AdvancementTree, root: usize, output: &mut Vec<usize>) {
    let mut children = vec![Vec::new(); tree.len()];
    for index in 0..tree.len() {
        if let Some(parent) = tree.node(index).parent() {
            children[parent].push(index);
        }
    }
    fn visit(node: usize, children: &[Vec<usize>], output: &mut Vec<usize>) {
        for &child in &children[node] {
            output.push(child);
            visit(child, children, output);
        }
    }
    visit(root, &children, output);
}

fn report_perform(
    run: &mut CommandRun,
    action: CommandAdvancementAction,
    targets: &[(CommandPlayerTarget, CommandPlayerView)],
    single_name: Option<NbtSummaryValue>,
    advancement_count: usize,
    count: usize,
    board: &Scoreboard,
) -> Outcome {
    let key = action.key();
    if count == 0 {
        return Err(perform_message(
            key,
            "failure",
            targets,
            single_name,
            advancement_count,
            board,
        ));
    }
    run.succeed(
        perform_message(
            key,
            "success",
            targets,
            single_name,
            advancement_count,
            board,
        ),
        true,
    );
    Ok(i32::try_from(count).unwrap_or(i32::MAX))
}

fn report_criterion(
    run: &mut CommandRun,
    action: CommandAdvancementAction,
    targets: &[(CommandPlayerTarget, CommandPlayerView)],
    advancement_name: NbtSummaryValue,
    criterion: &JavaString,
    count: usize,
    board: &Scoreboard,
) -> Outcome {
    let key = action.key();
    let message = if targets.len() == 1 {
        translatable(
            format!(
                "{key}.criterion.to.one.{}",
                if count == 0 { "failure" } else { "success" }
            ),
            vec![
                component_literal_nbt(criterion),
                advancement_name,
                team_player_display_name_nbt(&targets[0].1, board),
            ],
        )
    } else {
        translatable(
            format!(
                "{key}.criterion.to.many.{}",
                if count == 0 { "failure" } else { "success" }
            ),
            vec![
                component_literal_nbt(criterion),
                advancement_name,
                NbtSummaryValue::Int(i32::try_from(targets.len()).unwrap_or(i32::MAX)),
            ],
        )
    };
    if count == 0 {
        return Err(message);
    }
    run.succeed(message, true);
    Ok(i32::try_from(count).unwrap_or(i32::MAX))
}

fn perform_message(
    key: &str,
    outcome: &str,
    targets: &[(CommandPlayerTarget, CommandPlayerView)],
    single_name: Option<NbtSummaryValue>,
    advancement_count: usize,
    board: &Scoreboard,
) -> NbtSummaryValue {
    match (advancement_count, targets.len()) {
        (1, 1) => translatable(
            format!("{key}.one.to.one.{outcome}"),
            vec![
                single_name.expect("single advancement carries a display name"),
                team_player_display_name_nbt(&targets[0].1, board),
            ],
        ),
        (1, _) => translatable(
            format!("{key}.one.to.many.{outcome}"),
            vec![
                single_name.expect("single advancement carries a display name"),
                NbtSummaryValue::Int(i32::try_from(targets.len()).unwrap_or(i32::MAX)),
            ],
        ),
        (_, 1) => translatable(
            format!("{key}.many.to.one.{outcome}"),
            vec![
                NbtSummaryValue::Int(i32::try_from(advancement_count).unwrap_or(i32::MAX)),
                team_player_display_name_nbt(&targets[0].1, board),
            ],
        ),
        _ => translatable(
            format!("{key}.many.to.many.{outcome}"),
            vec![
                NbtSummaryValue::Int(i32::try_from(advancement_count).unwrap_or(i32::MAX)),
                NbtSummaryValue::Int(i32::try_from(targets.len()).unwrap_or(i32::MAX)),
            ],
        ),
    }
}
