//! `/advancement grant` and `/advancement revoke`.

use bbb_protocol::component_literal_nbt;
use bbb_protocol::NbtSummaryValue;

use crate::tests::parse_advancement;
use bbb_server_datapack::advancement::AdvancementElement;

use super::*;

const ROOT: &str = "minecraft:story/root";
const MINE: &str = "minecraft:story/mine_stone";
const TOOLS: &str = "minecraft:story/upgrade_tools";
const IRON: &str = "minecraft:story/smelt_iron";

pub(super) fn entry(id: &str, parent: Option<&str>, criteria: &[&str]) -> AdvancementElement {
    let criteria = criteria
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                serde_json::json!({"trigger":"minecraft:impossible"}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let mut json = serde_json::json!({"criteria":criteria});
    if let Some(parent) = parent {
        json["parent"] = serde_json::json!(parent);
    }
    AdvancementElement {
        id: id.to_string(),
        source_pack: "test".to_string(),
        advancement: parse_advancement(&json).expect("valid advancement"),
    }
}

fn tree() -> Vec<AdvancementElement> {
    vec![
        entry(ROOT, None, &["crafting_table"]),
        entry(MINE, Some(ROOT), &["get_stone"]),
        entry(TOOLS, Some(MINE), &["stone_pickaxe"]),
        entry(IRON, Some(MINE), &["iron"]),
    ]
}

pub(super) fn stage(
    entries: &[AdvancementElement],
    setup: &[String],
    online: &[NameAndId],
) -> CommandTestWorld {
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    startup.advancements.advancements = entries.to_vec();
    startup
        .advancements
        .advancements
        .sort_by(|a, b| a.id.cmp(&b.id));
    startup.advancements.warnings.clear();
    let mut world = CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            datapacks: Some(std::sync::Arc::new(startup)),
            game_type: GameType::Survival,
            ..Default::default()
        },
        online,
    );
    for command in setup {
        world.setup(command);
    }
    world
}
fn unstarted() -> Vec<String> {
    Vec::new()
}
fn finished(id: &str, criterion: &str) -> Vec<String> {
    vec![format!("advancement grant alice only {id} {criterion}")]
}
fn alice() -> [NameAndId; 1] {
    [user("alice", 1)]
}
fn grant_only_mine(setup: &[String]) -> CommandRun {
    ran(
        "advancement grant @s only minecraft:story/mine_stone",
        stage(&tree(), setup, &alice()),
    )
}

#[test]
fn grant_only_awards_the_named_advancement() {
    let run = grant_only_mine(&unstarted());
    assert_eq!(
        keys(&run),
        ["commands.advancement.grant.one.to.one.success"]
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(
        arguments(&run, 0),
        vec![component_literal_nbt(MINE), display_name("alice", 1)]
    );
}

#[test]
fn grant_only_refuses_an_already_done_advancement() {
    let run = grant_only_mine(&finished(MINE, "get_stone"));
    assert_eq!(
        keys(&run),
        ["commands.advancement.grant.one.to.one.failure"]
    );
    assert_eq!(run.result, None);
}

#[test]
fn grant_from_awards_the_target_and_descendants() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let mut fixture = stage(&entries, &progress, &online);
    let run = ran(
        "advancement grant @s from minecraft:story/mine_stone",
        &mut fixture,
    );
    assert_eq!(
        keys(&run),
        ["commands.advancement.grant.many.to.one.success"]
    );
    assert_eq!(run.result, Some(3));
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::Int(3), display_name("alice", 1)]
    );
    assert_eq!(
        completed(&fixture),
        [MINE, TOOLS, IRON]
            .into_iter()
            .map(str::to_string)
            .collect()
    );
}

#[test]
fn grant_until_awards_the_target_and_ancestors() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let mut fixture = stage(&entries, &progress, &online);
    let run = ran(
        "advancement grant @s until minecraft:story/mine_stone",
        &mut fixture,
    );
    assert_eq!(run.result, Some(2));

    assert_eq!(
        completed(&fixture),
        [ROOT, MINE].into_iter().map(str::to_string).collect()
    );
}

#[test]
fn grant_through_awards_ancestors_and_descendants() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let mut fixture = stage(&entries, &progress, &online);
    let run = ran(
        "advancement grant @s through minecraft:story/mine_stone",
        &mut fixture,
    );
    assert_eq!(run.result, Some(4));

    assert_eq!(
        completed(&fixture),
        [ROOT, MINE, TOOLS, IRON]
            .into_iter()
            .map(str::to_string)
            .collect()
    );
}

#[test]
fn grant_everything_suppresses_advancement_messages() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let run = ran(
        "advancement grant @s everything",
        stage(&entries, &progress, &online),
    );
    assert_eq!(run.result, Some(4));
}

#[test]
fn revoke_everything_keeps_advancement_messages() {
    let entries = tree();
    let online = alice();
    let progress = finished(MINE, "get_stone");
    let mut fixture = stage(&entries, &progress, &online);
    let run = ran("advancement revoke @s everything", &mut fixture);
    assert_eq!(run.result, Some(1));
    assert!(completed(&fixture).is_empty());
}

#[test]
fn grant_criterion_awards_one_name() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let run = ran(
        "advancement grant @s only minecraft:story/mine_stone get_stone",
        stage(&entries, &progress, &online),
    );
    assert_eq!(
        keys(&run),
        ["commands.advancement.grant.criterion.to.one.success"]
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(
        arguments(&run, 0),
        vec![
            component_literal_nbt("get_stone"),
            component_literal_nbt(MINE),
            display_name("alice", 1)
        ]
    );
}

#[test]
fn an_unknown_advancement_is_refused_at_execute() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let run = ran(
        "advancement grant @s only minecraft:story/missing",
        stage(&entries, &progress, &online),
    );
    assert_eq!(keys(&run), ["advancement.advancementNotFound"]);
    assert_eq!(
        arguments(&run, 0),
        vec![component_literal_nbt("minecraft:story/missing")]
    );
}

#[test]
fn an_unknown_criterion_is_refused_before_any_write() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let run = ran(
        "advancement grant @s only minecraft:story/mine_stone nosuch",
        stage(&entries, &progress, &online),
    );
    assert_eq!(keys(&run), ["commands.advancement.criterionNotFound"]);
    assert_eq!(
        arguments(&run, 0),
        vec![component_literal_nbt(MINE), component_literal_nbt("nosuch")]
    );
}

#[test]
fn a_missing_player_selection_uses_the_player_not_found_key() {
    let entries = tree();
    let progress = unstarted();
    let run = ran(
        "advancement grant @a only minecraft:story/mine_stone",
        stage(&entries, &progress, &[]),
    );
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);
}

#[test]
fn advancement_is_hidden_from_a_moderator_and_allowed_for_a_gamemaster() {
    let refusal = refused(
        "advancement grant @s only minecraft:story/mine_stone",
        CommandPermission::Moderator,
    );
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let allowed = ran_as(
        "advancement grant @s only minecraft:story/mine_stone",
        &source("alice", CommandPermission::GameMaster),
        stage(&entries, &progress, &online),
    );
    assert_eq!(
        keys(&allowed),
        ["commands.advancement.grant.one.to.one.success"]
    );
}

#[test]
fn grant_suggests_loaded_advancement_ids() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let completions = completions_for(
        "advancement grant @s only ",
        CommandPermission::GameMaster,
        stage(&entries, &progress, &online),
    );
    assert_eq!(texts(&completions), [MINE, ROOT, IRON, TOOLS]);
}

#[test]
fn only_suggests_the_named_advancements_criteria() {
    let entries = tree();
    let online = alice();
    let progress = unstarted();
    let completions = completions_for(
        "advancement grant @s only minecraft:story/mine_stone ",
        CommandPermission::GameMaster,
        stage(&entries, &progress, &online),
    );
    assert_eq!(texts(&completions), ["get_stone"]);
}

#[test]
fn display_advancement_uses_decorated_name_and_live_mutation_count() {
    let first = AdvancementElement {
        id: "test:first".to_string(),
        source_pack: "test".to_string(),
        advancement: parse_advancement(&serde_json::json!({
            "display": {
                "icon": {"id": "minecraft:stone"},
                "title": "First Step",
                "description": "Dig a block",
                "frame": "task"
            },
            "criteria": {"a": {"trigger": "minecraft:impossible"}},
            "rewards": {"recipes": ["minecraft:oak_planks"]}
        }))
        .expect("valid display advancement"),
    };
    let second = AdvancementElement {
        id: "test:second".to_string(),
        source_pack: "test".to_string(),
        advancement: parse_advancement(&serde_json::json!({
            "parent": "test:first",
            "criteria": {
                "unlocked": {
                    "trigger": "minecraft:recipe_unlocked",
                    "conditions": {"recipe": "minecraft:oak_planks"}
                }
            }
        }))
        .expect("valid recipe_unlocked advancement"),
    };
    let mut fixture = stage(&[first, second], &unstarted(), &alice());
    // Grant `from test:first`: `test:first`'s recipe reward synchronously
    // unlocks `test:second` via `recipe_unlocked` before the outer loop visits
    // `test:second`, so only 1 advancement was transitioned by the loop itself.
    let run = ran("advancement grant @s from test:first", &mut fixture);
    assert_eq!(run.result, Some(1), "{run:?}");

    // Single displayed advancement uses `chat.square_brackets` decorated name in
    // both `only` and `only ... <criterion>` messages (and unknown criterion).
    let missing_criterion = ran("advancement revoke @s only test:first nosuch", &mut fixture);
    assert_eq!(
        keys(&missing_criterion),
        ["commands.advancement.criterionNotFound"]
    );
    assert_eq!(
        translate_key(&arguments(&missing_criterion, 0)[0]).as_deref(),
        Some("chat.square_brackets")
    );

    let revoked = ran("advancement revoke @s only test:first a", &mut fixture);
    assert_eq!(
        keys(&revoked),
        ["commands.advancement.revoke.criterion.to.one.success"]
    );
    assert_eq!(
        translate_key(&arguments(&revoked, 0)[1]).as_deref(),
        Some("chat.square_brackets")
    );

    let granted = ran("advancement grant @s only test:first", &mut fixture);
    assert_eq!(
        keys(&granted),
        ["commands.advancement.grant.one.to.one.success"]
    );
    assert_eq!(
        translate_key(&arguments(&granted, 0)[0]).as_deref(),
        Some("chat.square_brackets")
    );
}

fn completed(fixture: &CommandTestWorld) -> std::collections::BTreeSet<String> {
    let world = fixture.world();
    let tree = world.advancement_tree();
    let player = world
        .command_player_by_id(uuid::Uuid::from_u128(1))
        .expect("the native player has progress");
    (0..tree.len())
        .filter(|&index| {
            player
                .advancements
                .progress(index)
                .is_some_and(|progress| progress.is_done())
        })
        .map(|index| tree.node(index).id().to_owned())
        .collect()
}
