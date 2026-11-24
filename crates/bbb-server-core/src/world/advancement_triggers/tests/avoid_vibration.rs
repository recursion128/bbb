use super::*;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::PlayerInput;

#[test]
fn sneaking_past_a_warden_or_sensor_awards_avoid_vibration_once() {
    for listener in ["warden", "sculk_sensor"] {
        let mut world = world(&[
            (
                "bbb:avoided",
                json!({
                    "criteria":{"done":{"trigger":"minecraft:avoid_vibration","conditions":{"player":{"type":"minecraft:player"}}}},
                    "rewards":{"experience":7}
                }),
            ),
            (
                "bbb:wrong_player",
                json!({
                    "criteria":{"done":{"trigger":"minecraft:avoid_vibration","conditions":{"player":{"type":"minecraft:cow"}}}},
                    "rewards":{"experience":100}
                }),
            ),
        ]);
        player_loaded(&mut world);
        command(&mut world, "gamemode survival");
        if listener == "warden" {
            command(&mut world, "summon minecraft:warden ~2 ~ ~ {NoAI:1b}");
        } else {
            let sensor = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_SCULK_SENSOR_ID,
            )
            .unwrap();
            beside_player(&mut world, sensor);
        }
        world.tick(Vec::new());
        let player = world.levels[CONNECTION].entity_id();
        let position = world.levels[CONNECTION].position();
        let pos = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32,
            z: position.z.floor() as i32,
        };
        world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
            world.last_command_sequence + 1,
            crate::mailbox::WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id: CONNECTION,
                packet: PlayServerbound::PlayerInput(PlayerInput {
                    shift: true,
                    ..Default::default()
                }),
            },
        )]);
        for _ in 0..2 {
            world.record_game_event_for_test(
                Java26BuiltinDimension::Overworld,
                VanillaGameEvent::STEP,
                pos,
                Some(player),
                None,
            );
            world.tick(Vec::new());
            assert_eq!(
                xp(&world),
                7,
                "{listener}: player predicate and one-time reward"
            );
        }
    }
}
