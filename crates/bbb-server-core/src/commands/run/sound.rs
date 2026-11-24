//! `/playsound` and `/stopsound`.

use bbb_facts::sound_sources::{vanilla_sound_source_for_serialized_name, SoundSource};
use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use bbb_protocol::packets::StopSound;

use crate::commands::parse::ParseResult;
use crate::commands::types::{CommandPlayer, CommandSoundRecipient};
use crate::commands::{
    team_player_display_name_nbt, translatable, CommandCx, CommandPlayerView, CommandSource,
};
use crate::connection::ClientboundPlayPacket;

use super::{
    coordinates_argument, float_argument, identifier_argument, player_states_argument,
    player_targets, source_player_state, CommandRun, Outcome,
};

/// `PlaySoundCommand.playSound` (PlaySoundCommand.java:151-207).
///
/// `sound_source` is the source literal the parse walked, or `None` on the
/// bare `<sound>` node, which plays on `SoundSource.MASTER` (`:41`). The
/// optional arguments are read by presence -- every executor under the literal
/// passes the same defaults for the ones it did not take (`1.0F`, `1.0F`,
/// `0.0F`, `:63-67` through `:121-123`).
pub(super) fn play_sound(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    sound_source: Option<&str>,
) -> Outcome {
    let (sound, sound_source, pitch, recipients) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let sound = identifier_argument(parse, "sound").to_string();
        let sound_source = sound_source.map_or(SoundSource::Master, sound_source_literal);
        // `EntityArgument.getPlayers` under the `<targets>` node (`:75`, `:89`,
        // ..), or `getCallingPlayerAsCollection(source.getPlayer())` (`:147-149`)
        // above it: the source's own player, or nobody when the source has none.
        let players: Vec<CommandPlayer<'_>> = if parse.argument("targets").is_some() {
            player_states_argument(parse, "targets", source, reads, at)?
        } else {
            source_player_state(source, reads).into_iter().collect()
        };
        // `Vec3Argument.getVec3` (`:92`) or `source.getPosition()` (`:42`, `:64`,
        // `:78`).
        let position = if parse.argument("pos").is_some() {
            coordinates_argument(parse, "pos").position(at)
        } else {
            at.position
        };
        let volume = if parse.argument("volume").is_some() {
            float_argument(parse, "volume")
        } else {
            1.0
        };
        let pitch = if parse.argument("pitch").is_some() {
            float_argument(parse, "pitch")
        } else {
            1.0
        };
        let min_volume = if parse.argument("minVolume").is_some() {
            float_argument(parse, "minVolume")
        } else {
            0.0
        };
        // `Holder.direct(SoundEvent.createVariableRangeEvent(sound))` (`:161`)
        // carries no fixed range, so `getRange(volume)` (SoundEvent.java:40-42) is
        // sixteen blocks, scaled up by a volume above one.
        let range = if volume > 1.0 { 16.0 * volume } else { 16.0 };
        let max_distance_squared = f64::from(range) * f64::from(range);
        let mut recipients = Vec::new();
        for player in players {
            // `player.level() == level` (`:168`).
            if player.dimension != &at.level {
                continue;
            }
            let delta = [
                position[0] - player.position[0],
                position[1] - player.position[1],
                position[2] - player.position[2],
            ];
            let distance_squared = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
            let (local_position, local_volume) = if distance_squared > max_distance_squared {
                // `:175-185`: out of range, which is nothing unless a `minVolume`
                // was asked for, and then two blocks from the listener towards
                // the sound at that volume.
                if min_volume <= 0.0 {
                    continue;
                }
                let distance = distance_squared.sqrt();
                (
                    [
                        player.position[0] + delta[0] / distance * 2.0,
                        player.position[1] + delta[1] / distance * 2.0,
                        player.position[2] + delta[2] / distance * 2.0,
                    ],
                    min_volume,
                )
            } else {
                (position, volume)
            };
            recipients.push(CommandSoundRecipient {
                player: player.target(),
                position: local_position,
                volume: local_volume,
            });
        }
        Ok::<_, NbtSummaryValue>((sound, sound_source, pitch, recipients))
    })?;
    let count = i32::try_from(recipients.len()).unwrap_or(i32::MAX);
    let heard_by = recipients.first().map(|recipient| recipient.player.clone());
    // The seed is drawn before anything is sent (`:164`), recipients or not,
    // which is why the sound goes out ahead of the refusal below.
    cx.world.play_command_sound(
        &cx.origin.level,
        &sound,
        sound_source,
        pitch,
        recipients,
        cx.events,
    );
    // `:194-196`.
    let Some(first) = heard_by else {
        return Err(translatable("commands.playsound.failed", Vec::new()));
    };
    // `:198-204`, both with `true`; `Component.translationArg(Identifier)` is
    // the id as a literal (Component.java:216-218).
    if count == 1 {
        run.succeed(
            translatable(
                "commands.playsound.success.single",
                vec![
                    component_literal_nbt(&sound),
                    team_player_display_name_nbt(
                        &CommandPlayerView {
                            name: first.name.clone(),
                            uuid: first.id,
                        },
                        cx.world.scoreboard(),
                    ),
                ],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.playsound.success.multiple",
                vec![component_literal_nbt(&sound), NbtSummaryValue::Int(count)],
            ),
            true,
        );
    }
    Ok(count)
}

/// `StopSoundCommand.stopSound` (StopSoundCommand.java:47-71).
///
/// `sound_source` is the source literal the parse walked, or `None` on the
/// bare `<targets>` node and under the `*` literal alike -- the `*` branch is
/// the explicit any-source form (`:26-31`). Unlike `/playsound` there is no
/// level or range check: every resolved target gets the packet (`:53-55`).
pub(super) fn stop_sound(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    sound_source: Option<&str>,
) -> Outcome {
    let (recipients, _) = player_targets(parse, Some("targets"), source, cx.world, &cx.origin)?;
    let sound = parse
        .argument("sound")
        .map(|_| identifier_argument(parse, "sound").to_string());
    let count = i32::try_from(recipients.len()).unwrap_or(i32::MAX);
    cx.world.send_to_command_players(
        &recipients,
        ClientboundPlayPacket::StopSound(StopSound {
            source: sound_source.map(sound_source_literal),
            name: sound.clone(),
        }),
        cx.events,
    );
    // The four reports (`:56-70`), all with `true`;
    // `Component.translationArg(Identifier)` is the id as a literal and
    // `soundSource.getName()` its serialized name, which is the literal the
    // parse walked.
    match (sound_source, sound) {
        (Some(source_name), Some(sound)) => run.succeed(
            translatable(
                "commands.stopsound.success.source.sound",
                vec![
                    component_literal_nbt(&sound),
                    component_literal_nbt(source_name),
                ],
            ),
            true,
        ),
        (Some(source_name), None) => run.succeed(
            translatable(
                "commands.stopsound.success.source.any",
                vec![component_literal_nbt(source_name)],
            ),
            true,
        ),
        (None, Some(sound)) => run.succeed(
            translatable(
                "commands.stopsound.success.sourceless.sound",
                vec![component_literal_nbt(&sound)],
            ),
            true,
        ),
        (None, None) => run.succeed(
            translatable("commands.stopsound.success.sourceless.any", Vec::new()),
            true,
        ),
    }
    Ok(count)
}

/// The `SoundSource` a source literal names: `/playsound` registers one
/// literal per `SoundSource.values()` under `source.getName()`
/// (PlaySoundCommand.java:49-51, `:57`), which is the serialized name the
/// catalog carries.
fn sound_source_literal(name: &str) -> SoundSource {
    vanilla_sound_source_for_serialized_name(name)
        .unwrap_or_else(|| unreachable!("{name} is a sound source literal of the derived tree"))
        .source
}
