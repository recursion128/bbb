//! Spider locomotion on live ticks and the climbing flag sent to a session.

use super::melee::{fixture, health, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_synched_data::{
    VANILLA_SPIDER_FLAGS_DATA_ID, VANILLA_SPIDER_FLAG_CLIMBING,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, VANILLA_ENTITY_TYPE_SPIDER_ID,
};
use bbb_protocol::packets::EntityDataValueKind;
use bbb_server_types::geometry::Vec3Ops;

const SPIDERS: [i32; 2] = [
    VANILLA_ENTITY_TYPE_SPIDER_ID,
    VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
];

fn climbing_update(tick: &TickResult, spider: i32) -> Option<bool> {
    tick.events.iter().find_map(|event| {
        let WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::SetEntityData(update),
        } = event
        else {
            return None;
        };
        if update.id != spider {
            return None;
        }
        update.values.iter().find_map(|row| match row.value {
            EntityDataValueKind::Byte(flags) if row.data_id == VANILLA_SPIDER_FLAGS_DATA_ID => {
                Some(flags & VANILLA_SPIDER_FLAG_CLIMBING != 0)
            }
            _ => None,
        })
    })
}

#[test]
fn spiders_climb_a_wall_to_bite_a_player_and_publish_leaving_the_wall() {
    for entity_type in SPIDERS {
        let (_sender, receiver, mut world) = fixture();
        let stone = block_state_id("minecraft:stone");
        for x in 0..=15 {
            for y in 65..=69 {
                for z in 8..=12 {
                    assert!(world.set_block(
                        DIMENSION,
                        BlockPos { x, y, z },
                        stone,
                        SET_BLOCK_UPDATE_ALL,
                    ));
                }
            }
        }
        let player = Vec3d {
            y: 70.0,
            z: 9.5,
            ..PLAYER_POSITION
        };
        world.set_player_position_for_test(ALICE, player);
        let (spider, _) = spawn(
            &mut world,
            entity_type,
            Vec3d {
                z: 4.5,
                ..PLAYER_POSITION
            },
        );
        // A real player blow supplies HurtByTarget even when the wall hides Alice.
        world.tick(receiver.drain());
        let player_id = world.player_entity_id_for_test(ALICE);
        let outcome = world
            .hurt_stored_entity_by_command_for_test(
                DIMENSION,
                spider,
                "minecraft:generic",
                1.0,
                Some(crate::commands::CommandDamageSourceEntity::Player {
                    user: crate::admin::NameAndId {
                        id: Uuid::from_u128(1),
                        name: "alice".into(),
                    }
                    .into(),
                    entity_id: player_id,
                    position: player,
                    instabuild: false,
                    may_build: true,
                    spectator: false,
                    display_name: NbtSummaryValue::string("alice"),
                }),
            )
            .unwrap();
        assert!(outcome.applied);
        let mut climbed = false;
        let mut left_wall = false;
        let mut max_y = PLAYER_POSITION.y;
        for _ in 0..500 {
            let tick = world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIMENSION, spider).unwrap();
            if let Some(flag) = climbing_update(&tick, spider) {
                left_wall |= climbed && !flag && body.position().y > 69.0;
                climbed |= flag;
            }
            max_y = max_y.max(body.position().y);
            if health(&world) < 20.0 && left_wall {
                break;
            }
        }
        let position = world
            .stored_entity_for_test(DIMENSION, spider)
            .unwrap()
            .position();
        assert!(
            max_y > 69.0,
            "{entity_type}: climbed above a jump: {max_y}, {position:?}"
        );
        assert!(
            health(&world) < 20.0,
            "{entity_type}: reached Alice and bit her: {position:?}"
        );
        assert!(
            climbed && left_wall,
            "{entity_type}: climbing metadata starts and clears"
        );
    }
}

#[test]
fn spiders_without_a_target_stroll_on_production_ticks() {
    for entity_type in SPIDERS {
        let (sender, receiver, mut world) = fixture();
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            12,
            "gamemode creative alice",
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
        let start = Vec3d {
            z: 4.5,
            ..PLAYER_POSITION
        };
        let (spider, _) = spawn(&mut world, entity_type, start);
        let mut moved = false;
        for _ in 0..600 {
            world.tick(receiver.drain());
            let position = world
                .stored_entity_for_test(DIMENSION, spider)
                .unwrap()
                .position();
            if position.distance_to_sqr(start) > 4.0 {
                moved = true;
                break;
            }
        }
        assert!(moved, "{entity_type}: an untargeted spider strolls");
        assert_eq!(target_of(&world, spider), None);
    }
}
