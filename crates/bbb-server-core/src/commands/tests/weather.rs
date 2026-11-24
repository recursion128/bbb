//! `/weather` (`WeatherCommand.java:15-42`).

use super::*;

use crate::commands::CommandWeatherKind;

/// `setClear` / `setRain` / `setThunder` (`WeatherCommand.java:48-63`): the
/// literal fixes the kind, the bare arm carries no duration, and the return
/// value is the argument rather than the duration that was used.
#[test]
fn each_literal_sets_its_own_weather_and_reports_its_own_key() {
    for (command, kind, key) in [
        (
            "weather clear",
            CommandWeatherKind::Clear,
            "commands.weather.set.clear",
        ),
        (
            "weather rain",
            CommandWeatherKind::Rain,
            "commands.weather.set.rain",
        ),
        (
            "weather thunder",
            CommandWeatherKind::Thunder,
            "commands.weather.set.thunder",
        ),
    ] {
        let mut world = CommandTestWorld::default();
        let run = ran(command, &mut world);
        assert_eq!(keys(&run), [key], "{command}");
        // `sendSuccess(.., true)` (`:50`, `:56`, `:62`).
        assert!(run.feedback[0].inform_admins, "{command}");
        // The three keys take no arguments.
        assert_eq!(arguments(&run, 0), Vec::new(), "{command}");
        let weather = world.weather_data();
        assert_eq!(
            (weather.raining, weather.thundering),
            match kind {
                CommandWeatherKind::Clear => (false, false),
                CommandWeatherKind::Rain => (true, false),
                CommandWeatherKind::Thunder => (true, true),
            }
        );
        assert!(if kind == CommandWeatherKind::Clear {
            weather.clear_weather_time > 0
        } else {
            weather.rain_time > 0
        });
        // `return duration` with the `-1` `register` passed (`:21`, `:51`).
        assert_eq!(run.result, Some(-1), "{command}");
    }
}

/// `TimeArgument.time(1)` (`WeatherCommand.java:23`): a float and one of the
/// four units (`TimeArgument.java:44-58`, `:78-83`), converted to ticks.
#[test]
fn the_duration_argument_is_a_time_in_ticks() {
    for (command, ticks) in [
        ("weather rain 1", 1),
        ("weather rain 1t", 1),
        ("weather rain 1s", 20),
        ("weather rain 1d", 24_000),
        ("weather clear 2.5s", 50),
    ] {
        let mut world = CommandTestWorld::default();
        let run = ran(command, &mut world);
        let weather = world.weather_data();
        if command.contains("clear") {
            assert_eq!(weather.clear_weather_time, ticks);
            assert!(!weather.raining);
        } else {
            assert_eq!((weather.rain_time, weather.thunder_time), (ticks, ticks));
            assert!(weather.raining);
        }
        assert_eq!(run.result, Some(ticks), "{command}");
    }
}

/// The minimum is one tick (`WeatherCommand.java:23`, `:30`, `:37`), which
/// `TimeArgument.parse` enforces (`TimeArgument.java:52-53`).
#[test]
fn a_duration_below_one_tick_is_refused() {
    for command in ["weather rain 0", "weather thunder 0t", "weather clear 0s"] {
        let refusal = ran(command, CommandTestWorld::default());
        assert_eq!(
            keys(&refusal),
            ["argument.time.tick_count_too_low", CURSOR_CONTEXT],
            "{command}"
        );
    }
    let mut world = CommandTestWorld::default();
    assert_eq!(ran("weather rain 1", &mut world).result, Some(1));
    assert_eq!(world.weather_data().rain_time, 1);
}

#[test]
fn an_unknown_time_unit_is_refused() {
    let refusal = ran("weather rain 1w", CommandTestWorld::default());
    assert_eq!(
        keys(&refusal),
        ["argument.time.invalid_unit", CURSOR_CONTEXT]
    );
}

/// `/execute in` retargets `source.getLevel()`, which is the level
/// `getDuration` samples off (`WeatherCommand.java:45`).
#[test]
fn execute_in_changes_the_source_for_weather_commands() {
    let mut world = CommandTestWorld::default();
    let run = ran("execute in the_nether run weather rain 10t", &mut world);
    assert_eq!(keys(&run), ["commands.weather.set.rain"]);
    let weather = world.weather_data();
    assert_eq!(weather.rain_time, 10);
    assert!(weather.raining);
}

/// `WeatherCommand.register` (`WeatherCommand.java:18`) gates the root on
/// `LEVEL_GAMEMASTERS`.
#[test]
fn weather_is_hidden_from_a_moderator() {
    let refusal = refused("weather clear", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
    // A game master sees it, so the refusal above is the permission and not a
    // missing registration.
    let allowed = ran_as(
        "weather clear",
        &source("alice", CommandPermission::GameMaster),
        CommandTestWorld::default(),
    );
    assert_eq!(keys(&allowed), ["commands.weather.set.clear"]);
}

/// `register` builds exactly three children and each takes exactly one
/// optional `<duration>` (`WeatherCommand.java:19-40`); nothing else parses.
#[test]
fn weather_has_no_fourth_literal_and_no_second_argument() {
    for command in ["weather", "weather snow", "weather rain 1t 2t"] {
        let refusal = ran(command, CommandTestWorld::default());
        assert!(
            matches!(
                keys(&refusal).first().map(String::as_str),
                Some("command.unknown.command" | "command.unknown.argument")
            ),
            "{command}: {:?}",
            keys(&refusal)
        );
    }
}
