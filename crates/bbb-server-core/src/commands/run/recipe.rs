//! `/recipe`.

use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use crate::commands::parse::ParseResult;
use crate::commands::{team_player_display_name_nbt, translatable, CommandPlayerView};

use super::{
    player_states_argument, resource_argument, CommandCx, CommandRun, CommandSource, Outcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecipeCommandAction {
    Give,
    Take,
}

impl RecipeCommandAction {
    fn translation_stem(self) -> &'static str {
        match self {
            Self::Give => "commands.recipe.give",
            Self::Take => "commands.recipe.take",
        }
    }
}

/// `RecipeCommand.register` (`RecipeCommand.java:21-70`) resolves targets
/// before the recipe manager lookup in both resource-key executors;
/// `giveRecipes` and `takeRecipes` (`:72-113`) mutate every target before
/// testing the aggregate display count and wording one line.
pub(super) fn recipe(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    action: RecipeCommandAction,
    all: bool,
) -> Outcome {
    let available_recipes = cx.world.recipe_ids();
    let (targets, display_names, recipe_ids) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let targets = player_states_argument(parse, "targets", source, reads, at)?;
        let recipe_ids = if all {
            available_recipes
        } else {
            let id = resource_argument(parse, "recipe");
            if !available_recipes.iter().any(|candidate| candidate == id) {
                return Err(translatable(
                    "recipe.notFound",
                    vec![component_literal_nbt(id)],
                ));
            }
            vec![id.to_string()]
        };
        let display_names = targets
            .iter()
            .map(|target| recipe_player_display_name(*target, reads.scoreboard))
            .collect::<Vec<_>>();
        let targets = targets
            .iter()
            .map(|target| target.target())
            .collect::<Vec<_>>();
        Ok((targets, display_names, recipe_ids))
    })?;
    let failed = || translatable(format!("{}.failed", action.translation_stem()), Vec::new());
    let display_count = cx
        .world
        .recipe_command(action, &targets, &recipe_ids, cx.events)
        .filter(|count| *count > 0)
        .ok_or_else(failed)?;
    let input_count = i32::try_from(recipe_ids.len()).unwrap_or(i32::MAX);
    let (suffix, target_argument) = match display_names.as_slice() {
        [single] => ("single", single.clone()),
        _ => (
            "multiple",
            NbtSummaryValue::Int(i32::try_from(targets.len()).unwrap_or(i32::MAX)),
        ),
    };
    run.succeed(
        translatable(
            format!("{}.success.{suffix}", action.translation_stem()),
            vec![NbtSummaryValue::Int(input_count), target_argument],
        ),
        true,
    );
    Ok(display_count)
}

fn recipe_player_display_name(
    target: crate::commands::types::CommandPlayer<'_>,
    board: &crate::world::scoreboard::Scoreboard,
) -> NbtSummaryValue {
    team_player_display_name_nbt(
        &CommandPlayerView {
            name: target.name.into(),
            uuid: target.id,
        },
        board,
    )
}
