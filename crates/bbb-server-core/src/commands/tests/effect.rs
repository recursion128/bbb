//! `/effect` (`EffectCommands.java:33-239`): the parse and the feedback; what
//! each target accepts is the live entity's answer.

use super::*;

use crate::entity::ActiveMobEffect;

fn give(
    resource_id: &'static str,
    duration: i32,
    amplifier: i32,
    particles: bool,
) -> ActiveMobEffect {
    ActiveMobEffect::give(resource_id, duration, amplifier, particles).expect("a built-in effect")
}

/// The effects `name` carries in `world`.
fn active(world: &CommandTestWorld, name: &str) -> Vec<ActiveMobEffect> {
    world
        .world()
        .command_players()
        .find(|player| player.name == name)
        .expect("fixture player")
        .effects
        .iter()
        .cloned()
        .collect()
}

/// The effects alice carries after `command` runs on `world`.
fn given(command: &str, mut world: CommandTestWorld) -> Vec<ActiveMobEffect> {
    ran(command, &mut world);
    active(&world, "alice")
}

/// `command` with every target accepting (`count`) or refusing (`0`) it.
fn settled(command: &str, mut fixture: CommandTestWorld, expected_changes: i32) -> CommandRun {
    if command.starts_with("effect clear") && expected_changes > 0 {
        fixture.setup("effect give alice speed");
    } else if command.starts_with("effect give") && expected_changes == 0 {
        fixture.setup(command);
    }
    ran(command, &mut fixture)
}

/// `EffectCommands.giveEffect` (`EffectCommands.java:55-58`, `:150-162`,
/// `:177-183`) with omitted seconds.
#[test]
fn effect_give_alice_speed_files_the_default_instance() {
    let players = [user("alice", 1)];
    assert_eq!(
        given("effect give alice speed", world(&players)),
        [give("minecraft:speed", 600, 0, true)]
    );
    let run = settled("effect give alice speed", world(&players), 1);
    assert_eq!(keys(&run), ["commands.effect.give.success.single"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(1));
    let args = arguments(&run, 0);
    assert_eq!(
        translate_key(&args[0]).as_deref(),
        Some("effect.minecraft.speed")
    );
    assert_eq!(args[1], display_name("alice", 1));
    assert_eq!(args[2], NbtSummaryValue::Int(30));
}

/// Instantaneous omitted seconds is duration 1 (`:158-159`); feedback is
/// `1 / 20` (`:180`).
#[test]
fn effect_give_omitted_seconds_on_instant_health_is_one_tick() {
    let players = [user("alice", 1)];
    assert_eq!(
        given("effect give alice instant_health", world(&players)),
        [give("minecraft:instant_health", 1, 0, true)]
    );
    let run = settled("effect give alice instant_health", world(&players), 1);
    assert_eq!(arguments(&run, 0)[2], NbtSummaryValue::Int(0));
}

/// Non-instantaneous seconds are multiplied by 20 (`:156`); instantaneous
/// seconds are taken verbatim (`:151-152`).
#[test]
fn effect_give_seconds_scale_only_for_lasting_effects() {
    let players = [user("alice", 1)];
    assert_eq!(
        given("effect give alice speed 5", world(&players)),
        [give("minecraft:speed", 100, 0, true)]
    );
    assert_eq!(
        given("effect give alice instant_health 5", world(&players)),
        [give("minecraft:instant_health", 5, 0, true)]
    );
}

/// The `infinite` literal passes -1 (`:103`); feedback is `-1 / 20` (`:180`).
#[test]
fn effect_give_alice_speed_infinite_is_minus_one() {
    let players = [user("alice", 1)];
    assert_eq!(
        given("effect give alice speed infinite", world(&players)),
        [give("minecraft:speed", -1, 0, true)]
    );
    let run = settled("effect give alice speed infinite", world(&players), 1);
    assert_eq!(arguments(&run, 0)[2], NbtSummaryValue::Int(0));
}

/// `hideParticles` inverts to `particles` (`:85-94`).
#[test]
fn effect_give_alice_speed_hides_particles_when_asked() {
    let players = [user("alice", 1)];
    assert_eq!(
        given("effect give alice speed 5 2 true", world(&players)),
        [give("minecraft:speed", 100, 2, false)]
    );
}

/// No target taking the effect is the generic failed key (`:185-187`).
#[test]
fn effect_give_nobody_accepting_fails() {
    let players = [user("alice", 1)];
    let run = settled("effect give alice speed", world(&players), 0);
    assert_eq!(keys(&run), ["commands.effect.give.failed"]);
    assert_eq!(run.result, None);
}

/// Bare `/effect clear` targets the source entity (`EffectCommands.java:39`).
#[test]
fn effect_clear_with_a_player_source_clears_self() {
    let mut fixture = world(&[user("alice", 1)]);
    fixture.setup("effect give alice speed");
    let run = ran_as(
        "effect clear",
        &source("alice", CommandPermission::Owner),
        &mut fixture,
    );
    assert_eq!(
        keys(&run),
        ["commands.effect.clear.everything.success.single"]
    );
    assert_eq!(run.result, Some(1));
}

/// Missing source entity is `permissions.requires.entity`
/// (`CommandSourceStack.java:393-399`, `:53`).
#[test]
fn effect_clear_from_the_console_requires_an_entity() {
    let run = ran_as(
        "effect clear",
        &CommandSource::server("console", CommandPermission::Owner),
        CommandTestWorld::default(),
    );
    assert_eq!(keys(&run), ["permissions.requires.entity"]);
}

/// `removeAllEffects` and `removeEffect` answer false when there is nothing to
/// remove (`:196-203`, `:215-226`).
#[test]
fn effect_clear_with_nothing_removed_fails() {
    let players = [user("alice", 1)];
    let run = settled("effect clear alice", world(&players), 0);
    assert_eq!(keys(&run), ["commands.effect.clear.everything.failed"]);
    let mut both = world(&players);
    both.setup("effect give alice speed");
    both.setup("effect give alice haste");
    ran("effect clear alice speed", &mut both);
    assert_eq!(
        active(&both, "alice"),
        [give("minecraft:haste", 600, 0, true)]
    );
    let run = settled("effect clear alice speed", world(&players), 1);
    assert_eq!(
        keys(&run),
        ["commands.effect.clear.specific.success.single"]
    );
    let run = settled("effect clear alice speed", world(&players), 0);
    assert_eq!(keys(&run), ["commands.effect.clear.specific.failed"]);
}

/// Two targets word `.multiple` over `entities.size()` (`:185-187`).
#[test]
fn effect_give_two_players_reports_multiple() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut fixture = world(&players);
    ran("effect give @a speed", &mut fixture);
    for name in ["alice", "bob"] {
        assert_eq!(
            active(&fixture, name),
            [give("minecraft:speed", 600, 0, true)],
            "{name}"
        );
    }
    let run = settled("effect give @a speed", world(&players), 2);
    assert_eq!(keys(&run), ["commands.effect.give.success.multiple"]);
    assert_eq!(run.result, Some(2));
    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Int(2));
    assert_eq!(arguments(&run, 0)[2], NbtSummaryValue::Int(30));
}

#[test]
fn effect_refuses_an_unknown_mob_effect() {
    let players = [user("alice", 1)];
    let run = ran("effect give alice not_an_effect", world(&players));
    assert_eq!(keys(&run), ["argument.resource.not_found", CURSOR_CONTEXT]);
}

#[test]
fn effect_give_seconds_zero_is_integer_too_low() {
    let players = [user("alice", 1)];
    let run = ran("effect give alice speed 0", world(&players));
    assert_eq!(keys(&run), ["argument.integer.low", CURSOR_CONTEXT]);
}
