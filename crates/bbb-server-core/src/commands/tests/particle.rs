//! `/particle` (`ParticleCommand.java:24-167`).

use super::*;

use bbb_protocol::packets::{
    decode_play_clientbound, encode_play_clientbound_level_particles, LevelParticles,
    PlayClientbound,
};

use crate::connection::ClientboundPlayPacket;

/// The particle packet a run fanned out, with the connections it reached.
fn particles(run: &CommandRun) -> (Vec<u64>, &LevelParticles) {
    let sends = run.sends();
    let mut packets = sends.iter().map(|(_, packet)| match packet {
        ClientboundPlayPacket::LevelParticles(packet) => packet,
        other => panic!("particle packet, not {other:?}"),
    });
    let packet = packets.next().expect("one particle packet");
    assert!(packets.all(|other| other == packet));
    (
        sends.iter().map(|(connection, _)| *connection).collect(),
        packet,
    )
}

fn heart_id() -> i32 {
    bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_HEART_ID
}

/// `ParticleCommand.sendParticles` (`ParticleCommand.java:31-40`) on the bare
/// `<name>` node: source position, zero delta/speed/count, every player.
#[test]
fn particle_emits_one_effect_and_the_success_line() {
    let players = [user("alice", 1)];
    let run = ran("particle heart", world(&players));
    let (recipients, packet) = particles(&run);
    assert_eq!(recipients, [1]);
    assert_eq!(packet.particle.particle_type_id, heart_id());
    assert!(packet.particle.raw_options.is_empty());
    assert!(!packet.override_limiter);
    assert!(!packet.always_show);
    assert_eq!(packet.count, 0);
    assert_eq!(packet.max_speed, 0.0);
    assert_eq!(keys(&run), ["commands.particle.success"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(arguments(&run, 0), vec![text("minecraft:heart")]);
    assert_eq!(run.result, Some(1));
}

/// `ServerLevel.sendParticles` (`ServerLevel.java:1363`): 32 blocks unless
/// `force` raises the limit to 512.
#[test]
fn particle_drops_a_viewer_past_thirty_two_blocks_unless_forced() {
    let far = user("bob", 2);
    let players = [user("alice", 1), far];
    let near = ran(
        "particle heart",
        world(&players).with_player_position("bob", [100.0, 0.0, 0.0]),
    );
    assert_eq!(particles(&near).0, [1]);
    assert_eq!(near.result, Some(1));

    let forced = ran(
        "particle heart 0 0 0 0 0 0 0 0 force @a",
        world(&players).with_player_position("bob", [100.0, 0.0, 0.0]),
    );
    let (recipients, packet) = particles(&forced);
    assert_eq!(recipients, [1, 2]);
    assert!(packet.override_limiter);
    assert_eq!(forced.result, Some(2));
}

/// `delta` is `Vec3Argument.vec3(false)` (`ParticleCommand.java:57`): whole
/// numbers stay on the block corner while `pos` is centre-corrected.
#[test]
fn particle_delta_is_not_centre_corrected() {
    let players = [user("alice", 1)];
    let run = ran("particle heart 1 2 3 1 2 3 0 0", world(&players));
    let (_, packet) = particles(&run);
    assert_eq!(
        [packet.position.x, packet.position.y, packet.position.z],
        [1.5, 2.0, 3.5]
    );
    assert_eq!(
        [packet.offset.x, packet.offset.y, packet.offset.z],
        [1.0, 2.0, 3.0]
    );
}

/// `ERROR_FAILED` (`ParticleCommand.java:160-161`) when nobody is in range.
#[test]
fn particle_refuses_when_nobody_sees_it() {
    let far = user("bob", 2);
    let run = ran(
        "particle heart",
        world(&[far]).with_player_position("bob", [100.0, 0.0, 0.0]),
    );
    assert_eq!(keys(&run), ["commands.particle.failed"]);
    assert!(run.sends().is_empty());
}

/// `ParticleArgument.readParticleType` (`ParticleArgument.java:64-67`).
#[test]
fn particle_refuses_an_unknown_type() {
    let players = [user("alice", 1)];
    let run = ran("particle not_a_particle", world(&players));
    assert_eq!(keys(&run), ["particle.notFound", CURSOR_CONTEXT]);
    assert!(run.sends().is_empty());
}

/// RequiredMap types without `{` fail the empty-map codec
/// (`ParticleArgument.java:78-81`).
#[test]
fn particle_refuses_a_required_map_without_options() {
    let players = [user("alice", 1)];
    let run = ran("particle dust", world(&players));
    assert_eq!(keys(&run), ["particle.invalidOptions"]);
    assert!(run.sends().is_empty());
}

/// An Empty type decodes any compound to its one value, and the reader stops
/// at the compound's end (`ParticleArgument.java:75-81`).
#[test]
fn particle_reads_the_options_compound_an_empty_type_ignores() {
    let players = [user("alice", 1)];
    let run = ran("particle heart{color:1}", world(&players));
    let (_, packet) = particles(&run);
    assert!(packet.particle.raw_options.is_empty());
    assert_eq!(keys(&run), ["commands.particle.success"]);

    let trailing = ran("particle heart{}x", world(&players));
    assert_eq!(
        keys(&trailing),
        ["command.expected.separator", CURSOR_CONTEXT]
    );
    assert_eq!(
        underlined_remainder(body(&trailing.feedback[1])).as_deref(),
        Some("x")
    );
}

/// A codec refusal is `ERROR_INVALID_OPTIONS::create` (`ParticleArgument.java:81`),
/// which carries no input, so `finishParsing` sends the message alone
/// (`Commands.java:366-369`).
#[test]
fn particle_refuses_undecodable_options_on_one_line() {
    let players = [user("alice", 1)];
    let run = ran(r#"particle dust{color:"red",scale:1}"#, world(&players));
    assert_eq!(keys(&run), ["particle.invalidOptions"]);
    assert!(run.sends().is_empty());
    assert_eq!(run.result, None);
}

/// `particle.notFound` quotes the reader after the id, and an SNBT syntax error
/// keeps its own position (`ParticleArgument.java:65-67`, `:75-76`).
#[test]
fn particle_positions_type_and_snbt_refusals() {
    let players = [user("alice", 1)];
    let unknown = ran("particle nope 0 0 0", world(&players));
    assert_eq!(keys(&unknown), ["particle.notFound", CURSOR_CONTEXT]);
    assert_eq!(
        underlined_remainder(body(&unknown.feedback[1])).as_deref(),
        Some(" 0 0 0")
    );

    let unclosed = ran("particle flame{a:1", world(&players));
    assert_eq!(
        keys(&unclosed),
        ["argument.literal.incorrect", CURSOR_CONTEXT]
    );
    assert_eq!(underlined_remainder(body(&unclosed.feedback[1])), None);
}

/// The options the command decodes reach the recipients as bytes the client
/// codec reads back (`ParticleTypes.java:158-159`).
#[test]
fn particle_sends_options_the_client_decodes() {
    let players = [user("alice", 1)];
    for (command, options) in [
        ("particle dust{color:[1,0,0],scale:1}", "ffff00003f800000"),
        (
            r#"particle item{item:{id:"diamond_sword",components:{"!minecraft:damage":{},"!minecraft:max_damage":{},"minecraft:custom_name":"hi"}}}"#,
            "a9070101020608000268690302",
        ),
        (
            r#"particle vibration{destination:{type:"block",pos:[1,2,3]},arrival_in_ticks:20}"#,
            "00000000400000300214",
        ),
    ] {
        let run = ran(command, world(&players));
        let (_, packet) = particles(&run);
        assert_eq!(hex(&packet.particle.raw_options), options, "{command}");
        let (packet_id, payload) = encode_play_clientbound_level_particles(packet.clone());
        assert_eq!(
            decode_play_clientbound(packet_id, &payload).expect(command),
            PlayClientbound::LevelParticles(packet.clone()),
        );
    }
}

/// OptionalMap types accept the codec defaults with no compound
/// (`SpellParticleOption.java:20-21`).
#[test]
fn particle_effect_uses_the_spell_defaults() {
    let players = [user("alice", 1)];
    let run = ran("particle effect", world(&players));
    let (_, packet) = particles(&run);
    assert_eq!(
        packet.particle.particle_type_id,
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EFFECT_ID
    );
    assert_eq!(packet.particle.raw_options.len(), 8);
    assert_eq!(keys(&run), ["commands.particle.success"]);
}

/// `ParticleArgument.listSuggestions` (`ParticleArgument.java:85-87`).
#[test]
fn particle_suggests_type_ids_until_the_options_compound() {
    let players = [user("alice", 1)];
    let offered = completions_for("particle hea", CommandPermission::Owner, world(&players));
    assert!(
        texts(&offered).iter().any(|text| text == "minecraft:heart"),
        "{offered:?}"
    );
    let patched = completions_for("particle heart{", CommandPermission::Owner, world(&players));
    assert!(texts(&patched).is_empty(), "{patched:?}");
}

/// `ParticleCommand.register` (`ParticleCommand.java:27`) gates the root on
/// `LEVEL_GAMEMASTERS`.
#[test]
fn particle_is_hidden_from_a_moderator() {
    let refusal = refused("particle heart", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
