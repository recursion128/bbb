//! `/particle` (`ParticleCommand.java:24-167`).

use crate::level_event::particle_reaches;
use bbb_protocol::packets::{LevelParticles, ParticlePayload, Vec3d};
use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::types::CommandPlayer;
use crate::commands::{translatable, CommandCx, CommandSource};
use crate::connection::ClientboundPlayPacket;

use super::{
    coordinates_argument, float_argument, integer_argument, player_states_argument, CommandRun,
    Outcome,
};

/// `ParticleCommand.sendParticles` (`ParticleCommand.java:142-166`).
pub(super) fn send_particles(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    force: bool,
) -> Outcome {
    let (recipients, packet, resource_id) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let (type_id, resource_id, raw_options) = particle_argument(parse);
        let position = if parse.argument("pos").is_some() {
            coordinates_argument(parse, "pos").position(at)
        } else {
            at.position
        };
        let delta = if parse.argument("delta").is_some() {
            coordinates_argument(parse, "delta").position(at)
        } else {
            [0.0, 0.0, 0.0]
        };
        let speed = if parse.argument("speed").is_some() {
            float_argument(parse, "speed")
        } else {
            0.0
        };
        let count = if parse.argument("count").is_some() {
            integer_argument(parse, "count")
        } else {
            0
        };
        let players: Vec<CommandPlayer<'_>> = if parse.argument("viewers").is_some() {
            player_states_argument(parse, "viewers", source, reads, at)?
        } else {
            reads.players().collect()
        };
        let mut recipients = Vec::new();
        for player in players {
            // `ServerLevel.sendParticles` (`ServerLevel.java:1359-1363`): same
            // level, then `closerToCenterThan` 32 or 512 when `force`.
            if player.dimension != &at.level {
                continue;
            }
            if !particle_reaches(
                position[0],
                position[1],
                position[2],
                player.position[0],
                player.position[1],
                player.position[2],
                force,
            ) {
                continue;
            }
            recipients.push(player.target());
        }
        if recipients.is_empty() {
            return Err(translatable("commands.particle.failed", Vec::new()));
        }
        let packet = LevelParticles {
            override_limiter: force,
            always_show: false,
            position: Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            offset: Vec3d {
                x: delta[0],
                y: delta[1],
                z: delta[2],
            },
            max_speed: speed,
            count,
            particle: ParticlePayload {
                particle_type_id: type_id,
                raw_options,
            },
        };
        Ok::<_, NbtSummaryValue>((recipients, packet, resource_id))
    })?;
    let count_sent = i32::try_from(recipients.len()).unwrap_or(i32::MAX);
    cx.world.send_to_command_players(
        &recipients,
        ClientboundPlayPacket::LevelParticles(packet),
        cx.events,
    );
    run.succeed(
        translatable(
            "commands.particle.success",
            vec![component_literal_nbt(&resource_id)],
        ),
        true,
    );
    Ok(count_sent)
}

fn particle_argument(parse: &ParseResult) -> (i32, String, Vec<u8>) {
    match parse.argument("name") {
        Some(ArgumentValue::Particle {
            type_id,
            resource_id,
            raw_options,
        }) => (*type_id, resource_id.clone(), raw_options.clone()),
        other => unreachable!("name is a particle argument, not {other:?}"),
    }
}
