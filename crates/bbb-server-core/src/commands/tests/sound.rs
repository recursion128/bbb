//! `/playsound`.

use bbb_protocol::packets::{SoundEventHolder, StopSound};

use super::*;
use crate::connection::ClientboundPlayPacket;

/// The alice-at-the-origin, bob-twenty-blocks-east pair the `/playsound` range
/// tests stand on: `SoundEvent.getRange(1.0)` is sixteen blocks
/// (SoundEvent.java:40-42), so bob is out of range at the default volume.
fn sound_listeners() -> [NameAndId; 2] {
    [user("alice", 1), user("bob", 2)]
}
fn sound_world(players: &[NameAndId]) -> CommandTestWorld {
    world(players).with_player_position("bob", [20.0, 0.0, 0.0])
}

/// One listener's copy of a played sound: its fixture connection id, the
/// position it hears the sound at and the volume.
#[derive(Debug, PartialEq)]
struct Heard {
    connection: u64,
    position: [f64; 3],
    volume: f32,
}

/// The sound packet every listener got (sound, source, pitch) and who got it.
fn played(run: &CommandRun) -> (Option<(String, SoundSource, f32)>, Vec<Heard>) {
    let mut sound = None;
    let mut heard = Vec::new();
    for (connection, packet) in run.sends() {
        let ClientboundPlayPacket::Sound(packet) = packet else {
            panic!("sound packet, not {packet:?}");
        };
        let SoundEventHolder::Direct { location, .. } = &packet.sound else {
            panic!("direct sound holder, not {:?}", packet.sound);
        };
        sound = Some((location.clone(), packet.source, packet.pitch));
        heard.push(Heard {
            connection,
            position: [packet.position.x, packet.position.y, packet.position.z],
            volume: packet.volume,
        });
    }
    (sound, heard)
}

/// `PlaySoundCommand.register`'s bare `<sound>` executor
/// (PlaySoundCommand.java:36-47): `MASTER`, the calling player, the source
/// position and the three defaults.
#[test]
fn playsound_plays_to_the_calling_player_on_master_when_no_literal_is_typed() {
    let players = sound_listeners();
    let run = ran(
        "playsound minecraft:block.note_block.harp",
        sound_world(&players),
    );
    let (sound, heard) = played(&run);
    assert_eq!(
        sound,
        Some((
            "minecraft:block.note_block.harp".to_string(),
            SoundSource::Master,
            1.0
        ))
    );
    assert_eq!(
        heard,
        [Heard {
            connection: 1,
            position: [0.0, 0.0, 0.0],
            volume: 1.0,
        }]
    );
    // `:198-201`: the id as a literal (Component.java:216-218) and the one
    // listener's display name, with `true`.
    assert_eq!(keys(&run), ["commands.playsound.success.single"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            text("minecraft:block.note_block.harp"),
            display_name("alice", 1)
        ]
    );
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(1));

    // `Identifier.read` qualifies a bare path (Identifier.java:189-199) and
    // refuses a `..` namespace as `argument.id.invalid` (`:231-233`). An
    // uppercase letter is not a refusal of the argument: `readGreedy` stops in
    // front of it, an empty path is a valid identifier, and what is left is
    // the dispatcher's `command.expected.separator`.
    let qualified = ran("playsound block.note_block.harp", sound_world(&players));
    let (sound, _) = played(&qualified);
    assert_eq!(sound.unwrap().0, "minecraft:block.note_block.harp");
    let refused = ran("playsound ..:harp", sound_world(&players));
    assert_eq!(
        failure_messages(&refused)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.id.invalid"]
    );
    let uppercase = ran("playsound Harp", sound_world(&players));
    assert_eq!(
        failure_messages(&uppercase)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["command.expected.separator"]
    );
}

/// `PlaySoundCommand.playSound`'s range loop (PlaySoundCommand.java:167-191).
#[test]
fn playsound_drops_a_listener_past_the_range_unless_a_min_volume_was_asked_for() {
    let players = sound_listeners();
    // Bob is twenty blocks away and the range at volume one is sixteen
    // (`:162`, SoundEvent.java:40-42), so `@a` reaches alice alone.
    let near = ran("playsound foo master @a", sound_world(&players));
    let (_, heard) = played(&near);
    assert_eq!(heard.iter().map(|h| h.connection).collect::<Vec<_>>(), [1]);
    assert_eq!(keys(&near), ["commands.playsound.success.single"]);

    // A `minVolume` above zero keeps him (`:176-178`): he hears the sound two
    // blocks towards it (`:181-183`) at that volume (`:184`).
    let far = ran(
        "playsound foo master @a ~ ~ ~ 1 1 0.5",
        sound_world(&players),
    );
    assert_eq!(
        played(&far).1,
        [
            Heard {
                connection: 1,
                position: [0.0, 0.0, 0.0],
                volume: 1.0,
            },
            Heard {
                connection: 2,
                position: [18.0, 0.0, 0.0],
                volume: 0.5,
            },
        ]
    );
    // `:203`: the id and the count, with `true`.
    assert_eq!(keys(&far), ["commands.playsound.success.multiple"]);
    assert_eq!(
        arguments(&far, 0),
        vec![text("minecraft:foo"), NbtSummaryValue::Int(2)]
    );
    assert_eq!(far.result, Some(2));

    // A volume above one stretches the range to `16 * volume`
    // (SoundEvent.java:41), so at volume two bob is simply in range.
    let loud = ran("playsound foo master @a ~ ~ ~ 2", sound_world(&players));
    let (_, heard) = played(&loud);
    assert_eq!(heard.len(), 2);
    assert_eq!(heard[1].position, [0.0, 0.0, 0.0]);
    assert_eq!(heard[1].volume, 2.0);

    // `player.level() == level` (`:168`): a target in another level never
    // hears it, whatever the distance.
    let elsewhere = [user("alice", 1), user("bob", 2)];
    let elsewhere = ran(
        "playsound foo master @a",
        world(&elsewhere).with_player_level("bob", level_key(Java26BuiltinDimension::Nether)),
    );
    assert_eq!(played(&elsewhere).1.len(), 1);
}

/// `ERROR_TOO_FAR` (PlaySoundCommand.java:194-196): nobody in range is the
/// refusal.
#[test]
fn playsound_refuses_when_nobody_hears_it() {
    let players = sound_listeners();
    let refused = ran("playsound foo master bob", sound_world(&players));
    assert_eq!(
        failure_messages(&refused)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["commands.playsound.failed"]
    );
    assert!(refused.sends().is_empty());

    // `getCallingPlayerAsCollection(source.getPlayer())` (`:147-149`) is
    // empty for a source with no player, so the bare form refuses too.
    let console = ran_as(
        "playsound foo",
        &CommandSource::server("console", CommandPermission::Owner),
        sound_world(&players),
    );
    assert_eq!(
        failure_messages(&console)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["commands.playsound.failed"]
    );
}

/// Each source literal is one `SoundSource.values()` entry under
/// `source.getName()` (PlaySoundCommand.java:49-51, `:57`), and the
/// `<pitch>` and `<volume>` reach the packet as typed.
#[test]
fn playsound_literals_name_the_sound_source_and_the_floats_travel_as_typed() {
    let players = sound_listeners();
    let run = ran(
        "playsound foo record alice 1 2 3 0.25 1.75",
        sound_world(&players),
    );
    let (sound, heard) = played(&run);
    let (_, source, pitch) = sound.unwrap();
    assert_eq!(source, SoundSource::Records);
    assert_eq!(pitch, 1.75);
    // `Vec3Argument.vec3()` is the centre-correcting form
    // (Vec3Argument.java:30-32), so a whole `x` and `z` land on the block's
    // middle while `y` stays where it was typed.
    assert_eq!(
        heard,
        [Heard {
            connection: 1,
            position: [1.5, 2.0, 3.5],
            volume: 0.25,
        }]
    );
    // `FloatArgumentType.floatArg(0.0F, 2.0F)` (`:113`).
    let refused = ran(
        "playsound foo record alice 1 2 3 1 2.5",
        sound_world(&players),
    );
    assert_eq!(
        failure_messages(&refused)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.float.big"]
    );
}

/// `SuggestionProviders.AVAILABLE_SOUNDS` (SuggestionProviders.java:20-22)
/// over `BuiltInRegistries.SOUND_EVENT` (CommandSourceStack.java:517-519).
#[test]
fn the_sound_argument_offers_the_built_in_sound_events() {
    let completions = completions_for(
        "playsound minecraft:block.note_block.ha",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(
        texts(&completions),
        [
            "minecraft:block.note_block.harp",
            "minecraft:block.note_block.hat"
        ]
    );
    // `filterResources` (SharedSuggestionProvider.java:93-107) matches a
    // colon-less remainder against the path alone, and `matchesSubStr`
    // (`:271-281`) restarts at every `.`/`_`/`/` splitter of it, so the id is
    // reached from inside its own path too.
    let bare = completions_for(
        "playsound note_block.harp",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&bare), ["minecraft:block.note_block.harp"]);
    let by_path = completions_for(
        "playsound block.note_block.harp",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&by_path), ["minecraft:block.note_block.harp"]);
    // `PlaySoundCommand.register` gates the root on `LEVEL_GAMEMASTERS`
    // (`:53`).
    let outcome = refused("playsound foo", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&outcome)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

/// The one `StopSound` packet a run sent and the connections it reached.
fn stopped(run: &CommandRun) -> (&StopSound, Vec<u64>) {
    let sends = run.sends();
    let packets: Vec<_> = sends
        .iter()
        .map(|(_, packet)| match packet {
            ClientboundPlayPacket::StopSound(packet) => packet,
            other => panic!("stop sound packet, not {other:?}"),
        })
        .collect();
    assert!(packets.iter().all(|packet| packet == &packets[0]));
    (
        packets[0],
        sends.iter().map(|(connection, _)| *connection).collect(),
    )
}

/// The bare `<targets>` executor (StopSoundCommand.java:23): no source, no
/// sound, every target -- range and dimension unchecked (`:53-55`).
#[test]
fn stopsound_reaches_every_target_wherever_it_stands() {
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran(
        "stopsound @a",
        world(&players)
            .with_player_position("bob", [4000.0, 0.0, 0.0])
            .with_player_level("bob", level_key(Java26BuiltinDimension::Nether)),
    );
    let (packet, recipients) = stopped(&run);
    assert_eq!(
        packet,
        &StopSound {
            source: None,
            name: None
        }
    );
    assert_eq!(recipients, [1, 2]);
    // `:69`: the argument-less key, with `true`.
    assert_eq!(keys(&run), ["commands.stopsound.success.sourceless.any"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(2));
}

/// One report per shape (`:56-70`): the source literal fills the source half,
/// the `*` literal leaves it empty with the sound named, and both together
/// name the sound before the source.
#[test]
fn stopsound_names_what_it_stopped_the_way_its_owner_words_it() {
    let players = [user("alice", 1)];

    let source_only = ran("stopsound alice music", world(&players));
    assert_eq!(
        stopped(&source_only).0,
        &StopSound {
            source: Some(SoundSource::Music),
            name: None
        }
    );
    assert_eq!(
        keys(&source_only),
        ["commands.stopsound.success.source.any"]
    );
    assert_eq!(arguments(&source_only, 0), vec![text("music")]);

    let sound_only = ran("stopsound alice * block.note_block.harp", world(&players));
    assert_eq!(
        stopped(&sound_only).0,
        &StopSound {
            source: None,
            name: Some("minecraft:block.note_block.harp".to_string())
        }
    );
    assert_eq!(
        keys(&sound_only),
        ["commands.stopsound.success.sourceless.sound"]
    );
    assert_eq!(
        arguments(&sound_only, 0),
        vec![text("minecraft:block.note_block.harp")]
    );

    let both = ran(
        "stopsound alice weather minecraft:weather.rain",
        world(&players),
    );
    assert_eq!(
        stopped(&both).0,
        &StopSound {
            source: Some(SoundSource::Weather),
            name: Some("minecraft:weather.rain".to_string())
        }
    );
    assert_eq!(keys(&both), ["commands.stopsound.success.source.sound"]);
    assert_eq!(
        arguments(&both, 0),
        vec![text("minecraft:weather.rain"), text("weather")]
    );
    assert_eq!(both.result, Some(1));
}

/// `EntityArgument.getPlayers` refuses an empty resolution before the body
/// runs (EntityArgument.java:96-102), so nothing is sent and nothing reported.
#[test]
fn stopsound_refuses_when_no_target_resolves() {
    let run = ran("stopsound @a", world(&[]));
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
    assert!(run.sends().is_empty());
    assert_eq!(run.result, None);
}

/// `/stopsound` needs `LEVEL_GAMEMASTERS` (StopSoundCommand.java:45); below it
/// the root is unknown rather than refused.
#[test]
fn stopsound_needs_gamemaster() {
    let refusal = refused("stopsound @a", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}
