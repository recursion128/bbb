//! `/weather` (`WeatherCommand.java:15-42`).

use super::super::types::CommandWeatherKind;
use super::{translatable, CommandCx, CommandRun, Outcome};

use super::super::parse::ParseResult;

/// `WeatherCommand.setClear` / `setRain` / `setThunder`
/// (`WeatherCommand.java:48-63`), which differ only in the parameters they hand
/// `MinecraftServer.setWeatherParameters` and the key they report.
///
/// `duration` is the `<duration>` argument or `None` for the bare literal, the
/// `-1` `register` passes (`:21`, `:28`, `:35`); `getDuration` (`:44-46`)
/// replaces that with a draw off the source level's random. The return value is the argument itself rather than the
/// duration that was used (`:51`, `:57`, `:63`), so a bare `/weather clear`
/// answers `-1`.
pub(super) fn set_weather(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    kind: CommandWeatherKind,
    with_duration: bool,
) -> Outcome {
    let duration = with_duration.then(|| super::time_argument(parse, "duration"));
    cx.world
        .set_weather_by_command(kind, duration, &cx.origin.level);
    run.succeed(translatable(kind.feedback_key(), Vec::new()), true);
    Ok(duration.unwrap_or(-1))
}
