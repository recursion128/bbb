//! `@e[advancements={..}]` (EntitySelectorOptions.java:375-454) end to end.

use super::*;
use bbb_server_datapack::advancement::AdvancementElement;

const ROOT: &str = "minecraft:story/root";
const CRITERION: &str = "crafting_table";

fn catalog() -> Vec<AdvancementElement> {
    vec![super::advancement::entry(ROOT, None, &[CRITERION])]
}

/// Alice has finished the advancement; bob has not started it.
fn progress() -> Vec<String> {
    vec![format!("advancement grant alice only {ROOT} {CRITERION}")]
}

fn advanced_world(
    players: &[NameAndId],
    entries: &[AdvancementElement],
    rows: &[String],
) -> CommandTestWorld {
    super::advancement::stage(entries, rows, players)
}

/// The tags `/tag ... add` pushed, which name exactly the players the selector
/// resolved to.
fn tagged(command: &str, mut world: CommandTestWorld) -> Vec<String> {
    ran(command, &mut world);
    world.tagged("marked")
}

#[test]
fn the_done_form_narrows_the_selection_to_the_players_that_match() {
    let players = [user("alice", 1), user("bob", 2)];
    let (entries, rows) = (catalog(), progress());

    assert_eq!(
        tagged(
            "tag @a[advancements={minecraft:story/root=true}] add marked",
            advanced_world(&players, &entries, &rows)
        ),
        ["alice"]
    );
    assert_eq!(
        tagged(
            "tag @a[advancements={minecraft:story/root=false}] add marked",
            advanced_world(&players, &entries, &rows)
        ),
        ["bob"]
    );
    // Mutation guard: without the option both players are named, so the two
    // answers above are the option's doing rather than the fixture's.
    assert_eq!(
        tagged(
            "tag @a add marked",
            advanced_world(&players, &entries, &rows)
        ),
        ["alice", "bob"]
    );
}

/// The criterion map (`:388-419`): a player with no progress at all answers
/// every declared criterion as not obtained, which is the progress
/// `getOrStartProgress` (`:442`) would have filed for them.
#[test]
fn the_criterion_form_narrows_on_one_criterion() {
    let players = [user("alice", 1), user("bob", 2)];
    let (entries, rows) = (catalog(), progress());

    assert_eq!(
        tagged(
            "tag @a[advancements={minecraft:story/root={crafting_table=true}}] add marked",
            advanced_world(&players, &entries, &rows)
        ),
        ["alice"]
    );
    assert_eq!(
        tagged(
            "tag @a[advancements={minecraft:story/root={crafting_table=false}}] add marked",
            advanced_world(&players, &entries, &rows)
        ),
        ["bob"]
    );
    // A criterion the advancement does not declare is refused for everyone
    // (`:412-413`), which `/tag` reports as a selection that found nobody.
    let run = ran(
        "tag @a[advancements={minecraft:story/root={nosuch=false}}] add marked",
        advanced_world(&players, &entries, &rows),
    );
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
}

/// `serverAdvancements.get(entry.getKey())` (`:441-442`).
#[test]
fn an_id_no_advancement_carries_selects_nobody() {
    let players = [user("alice", 1), user("bob", 2)];
    let (entries, rows) = (catalog(), progress());

    for expected in ["true", "false"] {
        let run = ran(
            &format!("tag @a[advancements={{minecraft:nosuch={expected}}}] add marked"),
            advanced_world(&players, &entries, &rows),
        );
        assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
    }
}

/// The predicate runs while the candidates are gathered, ahead of the order and
/// the result bound, so `@p` answers the nearest player that matches rather
/// than refusing when the nearest one does not.
#[test]
fn the_nearest_selector_answers_the_nearest_matching_player() {
    let near = user("bob", 2);
    let far = user("alice", 1);
    let players = [near, far];
    let (entries, rows) = (catalog(), progress());

    assert_eq!(
        tagged(
            "tag @p add marked",
            advanced_world(&players, &entries, &rows)
                .with_player_position("bob", [1.0, 0.0, 0.0])
                .with_player_position("alice", [10.0, 0.0, 0.0])
        ),
        ["bob"]
    );
    assert_eq!(
        tagged(
            "tag @p[advancements={minecraft:story/root=true}] add marked",
            advanced_world(&players, &entries, &rows)
                .with_player_position("bob", [1.0, 0.0, 0.0])
                .with_player_position("alice", [10.0, 0.0, 0.0])
        ),
        ["alice"]
    );
}

/// `:434-435` answers false for anything that is not a `ServerPlayer`, so a
/// non-empty map takes the stored entities out of `@e`.
#[test]
fn a_non_empty_map_takes_the_stored_entities_out_of_the_selection() {
    let zombie = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .expect("zombie is living");
    let entities = [stored_entity(
        9,
        99,
        [1.0, 64.0, 0.0],
        crate::entity::EntityData::GeneralLiving(zombie),
    )];
    let players = [user("alice", 1)];
    let (catalog, rows) = (catalog(), progress());

    // Without the option `/kill @e` reaches the zombie as well as alice.
    assert_eq!(
        ran(
            "kill @e",
            advanced_world(&players, &catalog, &rows).with_entities(&entities)
        )
        .result,
        Some(2)
    );
    // An empty map adds no predicate (`:432`), so it still does.
    assert_eq!(
        ran(
            "kill @e[advancements={}]",
            advanced_world(&players, &catalog, &rows).with_entities(&entities)
        )
        .result,
        Some(2)
    );
    // A map with an entry leaves only the matching player.
    assert_eq!(
        ran(
            "kill @e[advancements={minecraft:story/root=true}]",
            advanced_world(&players, &catalog, &rows).with_entities(&entities)
        )
        .result,
        Some(1)
    );
}

/// `setIncludesEntities(false)` (`:450`), which `EntityArgument.parse`
/// (EntityArgument.java:118-120) reads before it accepts a selector for a
/// players-only argument.
#[test]
fn a_non_empty_map_lets_an_at_e_reach_a_players_only_argument() {
    let players = [user("alice", 1), user("bob", 2)];
    let (entries, rows) = (catalog(), progress());

    // A parse refusal is the key plus the styled cursor line beside it.
    assert_eq!(
        keys(&ran(
            "gamemode creative @e",
            advanced_world(&players, &entries, &rows)
        )),
        ["argument.player.entities", CURSOR_CONTEXT]
    );
    // The same refusal stands for an empty map, which adds no predicate and so
    // never reaches `setIncludesEntities`.
    assert_eq!(
        keys(&ran(
            "gamemode creative @e[advancements={}]",
            advanced_world(&players, &entries, &rows)
        )),
        ["argument.player.entities", CURSOR_CONTEXT]
    );
    // Run as bob so the one match is somebody else, which is the branch that
    // proves the selector was accepted and resolved rather than folded into
    // the source.
    let bob = CommandSource::player("bob", Uuid::from_u128(2), CommandPermission::Owner);
    let run = ran_as(
        "gamemode creative @e[advancements={minecraft:story/root=true}]",
        &bob,
        advanced_world(&players, &entries, &rows),
    );
    assert_eq!(keys(&run), ["commands.gamemode.success.other"]);
}

/// An option beside `advancements` narrows the same set, in either order.
#[test]
fn an_option_beside_advancements_narrows_the_same_set() {
    let players = [user("alice", 1), user("bob", 2)];
    let (entries, rows) = (catalog(), progress());

    for command in [
        "tag @a[tag=boss,advancements={minecraft:story/root=true}] add marked",
        "tag @a[advancements={minecraft:story/root=true},tag=boss] add marked",
    ] {
        let run = ran(command, advanced_world(&players, &entries, &rows));
        assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
    }
    for command in [
        "tag @a[name=!bob,advancements={minecraft:story/root=true}] add marked",
        "tag @a[advancements={minecraft:story/root=true},distance=..1] add marked",
    ] {
        assert_eq!(
            tagged(command, advanced_world(&players, &entries, &rows)),
            ["alice"],
            "{command}"
        );
    }
}

/// `@s` runs the predicate over the source itself.
#[test]
fn the_self_selector_is_filtered_by_the_same_predicate() {
    let players = [user("alice", 1), user("bob", 2)];
    let (entries, rows) = (catalog(), progress());

    // `source` binds every name to uuid 1, so bob's source is built here.
    let alice = source("alice", CommandPermission::Owner);
    let bob = CommandSource::player("bob", Uuid::from_u128(2), CommandPermission::Owner);
    let mut world = advanced_world(&players, &entries, &rows);
    ran_as(
        "tag @s[advancements={minecraft:story/root=true}] add marked",
        &alice,
        &mut world,
    );
    assert_eq!(world.tagged("marked"), ["alice"]);
    let run = ran_as(
        "tag @s[advancements={minecraft:story/root=true}] add marked",
        &bob,
        advanced_world(&players, &entries, &rows),
    );
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
}
