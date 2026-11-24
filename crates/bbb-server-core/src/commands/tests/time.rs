use super::*;

const OVERWORLD: usize = 0;
const THE_END: usize = 1;

fn at(clock: usize, total_ticks: i64) -> [i64; 2] {
    let mut ticks = [0; 2];
    ticks[clock] = total_ticks;
    ticks
}

fn ran_at(command: &str, total_ticks: &[i64]) -> CommandRun {
    ran(
        command,
        CommandTestWorld::default().with_clock_ticks(total_ticks),
    )
}

fn nether_world() -> CommandTestWorld {
    CommandTestWorld::new(crate::world::ServerWorldConfig::default(), &[])
        .with_level(Java26BuiltinDimension::Nether.into())
}

fn ran_in_the_nether(command: &str) -> CommandRun {
    ran(command, nether_world())
}

#[test]
fn time_query_gametime_wraps_the_result_at_int_max() {
    let run = ran(
        "time query gametime",
        CommandTestWorld::default().with_game_time(2_147_483_647),
    );
    assert_eq!(run.result, Some(0));
    assert_eq!(arguments(&run, 0), [NbtSummaryValue::Long(2_147_483_647)]);
}

#[test]
fn time_query_time_reports_the_default_clock_total_ticks() {
    let run = ran_at("time query time", &at(OVERWORLD, 50_000));
    assert_eq!(keys(&run), ["commands.time.query.absolute"]);
    assert!(!run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(50_000));
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:overworld"),
            NbtSummaryValue::Long(50_000),
        ]
    );
}

#[test]
fn time_of_reads_the_clock_the_argument_names() {
    let run = ran_at("time of minecraft:the_end query time", &at(THE_END, 7));
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:the_end"),
            NbtSummaryValue::Long(7),
        ]
    );
    assert_eq!(run.result, Some(7));

    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let mut world = CommandTestWorld::default()
        .with_clock_ticks(&[50_000, 107])
        .with_game_time(7)
        .with_chest(pos);
    let source = CommandSource::server("console", CommandPermission::Owner);
    for (clock, period, value, expected) in [
        ("minecraft:the_end", "", 107, 7),
        ("minecraft:overworld", "", 107, 1),
        ("minecraft:the_end", ",period:100", 7, 7),
    ] {
        world.setup("item replace block 1 64 1 container.0 with diamond");
        let run = ran_as(
            &format!("item modify block 1 64 1 container.0 {{function:'minecraft:set_count',count:7,conditions:[{{condition:'minecraft:time_check',clock:'{clock}'{period},value:{value}}}]}}"),
            &source,
            &mut world,
        );
        assert_eq!(run.result, Some(1), "{clock}{period}: {run:?}");
        assert_eq!(world.container_item(pos, 0).unwrap().count(), expected);
    }
    world.setup("time of minecraft:the_end set 108");
    world.setup("item replace block 1 64 1 container.0 with diamond");
    let run = ran_as("item modify block 1 64 1 container.0 {function:'minecraft:set_count',count:7,conditions:[{condition:'minecraft:time_check',clock:'minecraft:the_end',period:100,value:7}]}", &source, &mut world);
    assert_eq!(run.result, Some(1), "{run:?}");
    assert_eq!(world.container_item(pos, 0).unwrap().count(), 1);
}

#[test]
fn time_of_refuses_a_clock_no_registry_holds() {
    let run = ran(
        "time of minecraft:nether query time",
        CommandTestWorld::default(),
    );
    assert_eq!(keys(&run), ["argument.resource.not_found", CURSOR_CONTEXT]);
}

#[test]
fn time_set_writes_the_total_ticks_and_answers_the_argument() {
    let run = ran_at("time set 1000", &at(OVERWORLD, 50_000));
    assert_eq!(keys(&run), ["commands.time.set.absolute"]);
    assert!(run.feedback[0].inform_admins);

    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:overworld"),
            NbtSummaryValue::Int(1_000),
        ]
    );
    assert_eq!(run.result, Some(1_000));
}

#[test]
fn time_set_reads_the_time_units() {
    assert_eq!(
        ran_at("time set 2d", &at(OVERWORLD, 0)).result,
        Some(48_000)
    );
    assert_eq!(ran_at("time set 3s", &at(OVERWORLD, 0)).result, Some(60));
    assert_eq!(ran_at("time set 7t", &at(OVERWORLD, 0)).result, Some(7));
}

#[test]
fn time_set_reads_a_count_below_the_bound_as_a_time_marker() {
    let refusal = ran_at("time set -1", &at(OVERWORLD, 0));
    assert_eq!(keys(&refusal), ["commands.time.no_time_marker_found"]);
    assert_eq!(
        arguments(&refusal, 0)[0],
        NbtSummaryValue::string("ResourceKey[minecraft:clock_time_marker / minecraft:-1]")
    );
    assert_eq!(
        keys(&ran_at("time set 1", &at(OVERWORLD, 0))),
        ["commands.time.set.absolute"]
    );
}

#[test]
fn time_add_reports_the_clock_reading_the_write_leaves() {
    let run = ran_at("time add 1000", &at(OVERWORLD, 50_000));
    assert_eq!(keys(&run), ["commands.time.set.absolute"]);

    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:overworld"),
            NbtSummaryValue::Long(51_000),
        ]
    );
    assert_eq!(run.result, Some(51_000));
}

#[test]
fn time_add_floors_the_sum_at_zero() {
    let run = ran_at("time add -500", &at(OVERWORLD, 100));

    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Long(0));
    assert_eq!(run.result, Some(0));
}

#[test]
fn time_set_time_marker_moves_to_the_next_occurrence() {
    let mut fixture = CommandTestWorld::default().with_clock_ticks(&at(OVERWORLD, 0));
    let run = ran("time set minecraft:noon", &mut fixture);
    assert_eq!(keys(&run), ["commands.time.set.time_marker"]);
    assert!(run.feedback[0].inform_admins);

    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:overworld"),
            NbtSummaryValue::string("minecraft:noon"),
        ]
    );
    assert_eq!(run.result, Some(6_000));
    assert_eq!(fixture.clock_state(OVERWORLD).total_ticks, 6_000);
    let run = ran("time set minecraft:noon", &mut fixture);
    assert_eq!(run.result, Some(30_000));
    assert_eq!(fixture.clock_state(OVERWORLD).total_ticks, 30_000);
}

#[test]
fn time_set_time_marker_accepts_a_marker_that_is_not_suggested() {
    let run = ran_at("time set minecraft:wake_up_from_sleep", &at(OVERWORLD, 100));
    assert_eq!(keys(&run), ["commands.time.set.time_marker"]);
}

#[test]
fn time_set_time_marker_refuses_a_marker_the_clock_does_not_carry() {
    let run = ran_at(
        "time of minecraft:the_end set minecraft:noon",
        &at(THE_END, 0),
    );
    assert_eq!(keys(&run), ["commands.time.no_time_marker_found"]);
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("ResourceKey[minecraft:clock_time_marker / minecraft:noon]"),
            NbtSummaryValue::string("minecraft:the_end"),
        ]
    );
}

#[test]
fn time_pause_and_resume_write_the_flag_and_answer_one() {
    let paused = ran_at("time pause", &at(OVERWORLD, 0));
    assert_eq!(keys(&paused), ["commands.time.pause"]);
    assert!(paused.feedback[0].inform_admins);

    assert_eq!(
        arguments(&paused, 0),
        [NbtSummaryValue::string("minecraft:overworld")]
    );
    assert_eq!(paused.result, Some(1));

    let resumed = ran_at("time of minecraft:the_end resume", &at(THE_END, 0));
    assert_eq!(keys(&resumed), ["commands.time.resume"]);

    assert_eq!(resumed.result, Some(1));
}

#[test]
fn time_rate_writes_the_rate_and_reports_it() {
    let run = ran_at("time rate 0.5", &at(OVERWORLD, 0));
    assert_eq!(keys(&run), ["commands.time.rate"]);
    assert!(run.feedback[0].inform_admins);

    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:overworld"),
            NbtSummaryValue::Float(0.5_f32.to_bits()),
        ]
    );
    assert_eq!(run.result, Some(1));

    let refusal = ran_at("time rate 1001", &at(OVERWORLD, 0));
    assert_eq!(keys(&refusal), ["argument.float.big", CURSOR_CONTEXT]);
}

#[test]
fn time_query_timeline_reports_the_period_position_and_count() {
    let ticks = at(OVERWORLD, 50_000);
    let run = ran_at("time query minecraft:day", &ticks);
    assert_eq!(keys(&run), ["commands.time.query.timeline"]);
    assert!(!run.feedback[0].inform_admins);
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:day"),
            NbtSummaryValue::Long(2_000),
        ]
    );
    assert_eq!(run.result, Some(2_000));

    let run = ran_at("time query minecraft:day repetition", &ticks);
    assert_eq!(keys(&run), ["commands.time.query.timeline.repetitions"]);
    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Long(2));
    assert_eq!(run.result, Some(2));
}

#[test]
fn time_query_a_timeline_without_a_period_counts_no_repetitions() {
    let ticks = at(OVERWORLD, 50_000);
    let run = ran_at("time query minecraft:early_game", &ticks);
    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Long(50_000));
    let run = ran_at("time query minecraft:early_game repetition", &ticks);
    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Long(0));
}

#[test]
fn time_query_refuses_a_timeline_of_another_clock() {
    let run = ran_at(
        "time of minecraft:the_end query minecraft:day",
        &at(THE_END, 0),
    );
    assert_eq!(keys(&run), ["commands.time.wrong_timeline_for_clock"]);
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::string("minecraft:day"),
            NbtSummaryValue::string("minecraft:the_end"),
        ]
    );
}

#[test]
fn time_refuses_every_bare_arm_in_a_dimension_with_no_default_clock() {
    for command in [
        "time set 0",
        "time set minecraft:noon",
        "time add 1",
        "time pause",
        "time resume",
        "time rate 1",
        "time query time",
        "time query minecraft:day",
        "time query minecraft:day repetition",
    ] {
        let run = ran_in_the_nether(command);
        assert_eq!(keys(&run), ["commands.time.no_default_clock"], "{command}");
        assert_eq!(
            arguments(&run, 0),
            [NbtSummaryValue::string("minecraft:the_nether")],
            "{command}"
        );
    }
    let run = ran_in_the_nether("time query gametime");
    assert_eq!(keys(&run), ["commands.time.query.gametime"]);
    let run = ran_in_the_nether("time of minecraft:overworld pause");
    assert_eq!(keys(&run), ["commands.time.pause"]);
}

#[test]
fn time_set_offers_only_the_markers_that_show_in_commands() {
    let completions = completions_for(
        "time set ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(
        texts(&completions),
        [
            "minecraft:day",
            "minecraft:midnight",
            "minecraft:night",
            "minecraft:noon",
        ]
    );
    let completions = completions_for(
        "time of minecraft:the_end set ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert!(texts(&completions).is_empty());
}

#[test]
fn time_query_offers_the_timelines_of_the_queried_clock() {
    let completions = completions_for(
        "time query ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(
        texts(&completions),
        [
            "gametime",
            "minecraft:day",
            "minecraft:early_game",
            "minecraft:moon",
            "minecraft:villager_schedule",
            "time",
        ]
    );
    let completions = completions_for(
        "time of minecraft:the_end query ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["time"]);
}

#[test]
fn time_offers_no_marker_or_timeline_without_a_default_clock() {
    let mut nether = nether_world();
    assert!(texts(&completions_for(
        "time set ",
        CommandPermission::Owner,
        &mut nether
    ))
    .is_empty());
    assert_eq!(
        texts(&completions_for(
            "time query ",
            CommandPermission::Owner,
            &mut nether
        )),
        ["gametime", "time"]
    );
}

#[test]
fn time_is_hidden_from_a_moderator() {
    let refusal = refused("time query gametime", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

#[test]
fn clock_commands_change_the_saved_clock_state() {
    let mut world = CommandTestWorld::default();
    ran("time set 6000", &mut world);
    assert_eq!(world.clock_state(OVERWORLD).total_ticks, 6000);
    ran("time set minecraft:noon", &mut world);
    assert_eq!(world.clock_state(OVERWORLD).total_ticks, 30000);
    ran("time pause", &mut world);
    ran("time rate 0.5", &mut world);
    assert!(world.clock_state(OVERWORLD).paused);
    assert_eq!(world.clock_state(OVERWORLD).rate, 0.5);
    ran("time resume", &mut world);
    assert!(!world.clock_state(OVERWORLD).paused);
}
