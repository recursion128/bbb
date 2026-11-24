//! Command-built doors and ordinary raid members running real world ticks.

use super::scene::{command, pre_wave, tick, DIMENSION};
use super::*;
use crate::entity::raider::RaiderRaid;
use crate::entity::{Entity, EntityData};
use bbb_protocol::block_state_types::{
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_property,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VINDICATOR_ID;

fn current_body(world: &ServerWorld, id: i32, uuid: Uuid, raid: i32) -> &Entity {
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    assert_eq!(body.uuid(), uuid);
    assert!(body.is_alive_with_health());
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    assert!(!data.is_no_ai());
    assert_eq!(
        data.raider_retained().unwrap().raid(),
        RaiderRaid::Current(raid)
    );
    assert!(raid_manager(world).row(raid).unwrap().active);
    body
}

#[test]
fn actual_raid_vindicators_open_and_break_wooden_doors_to_reach_a_player() {
    let (mut world, sender, receiver, raid_id, _) = pre_wave();
    let center = raid_manager(&world).row(raid_id).unwrap().center;
    let air =
        vanilla_block_state_id_by_name_and_properties("minecraft:air", &BTreeMap::new()).unwrap();
    for (stage, difficulty, griefing) in [(0, "normal", false), (1, "hard", true)] {
        world.tick_rate_mut().set_frozen(true);
        let door = BlockPos {
            x: center.x + 7,
            y: center.y,
            z: center.z + 8 + stage * 8,
        };
        let upper = BlockPos {
            y: door.y + 1,
            ..door
        };
        for text in [
            format!("difficulty {difficulty}"),
            format!("gamerule mob_griefing {griefing}"),
            "gamemode creative hero".into(),
            format!(
                "fill {} {} {} {} {} {} minecraft:stone",
                door.x - 5,
                door.y - 1,
                door.z - 1,
                door.x + 5,
                door.y + 2,
                door.z + 1,
            ),
            format!(
                "fill {} {} {} {} {} {} minecraft:air",
                door.x - 4,
                door.y,
                door.z,
                door.x + 4,
                door.y + 1,
                door.z,
            ),
            format!(
                "fill {} {} {} {} {} {} minecraft:stone",
                door.x, door.y, door.z, upper.x, upper.y, upper.z
            ),
            format!(
                "tp hero {} {} {}",
                f64::from(door.x) + 3.5,
                door.y,
                f64::from(door.z) + 0.5
            ),
        ] {
            command(&sender, &receiver, &mut world, &text);
        }
        if stage == 0 {
            command(
                &sender,
                &receiver,
                &mut world,
                "effect give hero minecraft:resistance infinite 4 true",
            );
        }
        let ids = world.stored_entity_ids_for_test(DIMENSION);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                "summon minecraft:vindicator {} {} {} {{CanJoinRaid:1b}}",
                f64::from(door.x) - 0.5,
                door.y,
                f64::from(door.z) + 0.5,
            ),
        );
        let body = world
            .stored_entity_ids_for_test(DIMENSION)
            .into_iter()
            .filter(|id| !ids.contains(id))
            .filter_map(|id| world.stored_entity_for_test(DIMENSION, id))
            .find(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_VINDICATOR_ID)
            .expect("the console summons an ordinary Vindicator");
        let (id, uuid) = (body.id(), body.uuid());
        world.tick_rate_mut().set_frozen(false);
        let mut joined = false;
        for _ in 0..60 {
            tick(&sender, &receiver, &mut world);
            let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
            let EntityData::GeneralLiving(data) = body.data() else {
                unreachable!()
            };
            if data.raider_retained().unwrap().raid() == RaiderRaid::Current(raid_id) {
                joined = true;
                break;
            }
        }
        assert!(joined, "the real body spatially joins the active raid");
        world.tick_rate_mut().set_frozen(true);
        for (pos, half) in [(door, "lower"), (upper, "upper")] {
            command(&sender, &receiver, &mut world, &format!(
                "setblock {} {} {} minecraft:oak_door[facing=east,half={half},hinge=left,open=false,powered=false] strict",
                pos.x, pos.y, pos.z,
            ));
        }
        command(&sender, &receiver, &mut world, "gamemode survival hero");
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:player_attack by hero"),
        );
        let health = current_body(&world, id, uuid, raid_id)
            .living()
            .unwrap()
            .health();
        let hero_health = world.player_health_for_test(HERO);
        world.tick_rate_mut().set_frozen(false);
        let (mut opened, mut cracking, mut crossed, mut broken) = (false, false, false, false);
        let mut breaking = false;
        let mut hero_right = true;
        let mut closures = 0;
        for _ in 0..2400 {
            let output = tick(&sender, &receiver, &mut world);
            let body = current_body(&world, id, uuid, raid_id);
            assert_eq!(body.living().unwrap().health(), health);
            let body_x = body.position().x;
            for event in &output.events {
                if let WorldEvent::Send {
                    connection_id: HERO,
                    packet: ClientboundPlayPacket::BlockDestruction(packet),
                } = event
                {
                    if packet.id == id && (packet.pos == door || packet.pos == upper) {
                        breaking = packet.progress <= 9;
                        cracking |= breaking;
                    }
                }
            }
            let lower_state = world.block_state(DIMENSION, door).unwrap();
            let upper_state = world.block_state(DIMENSION, upper).unwrap();
            let is_open = vanilla_block_state_property(lower_state, "open") == Some("true")
                && vanilla_block_state_property(upper_state, "open") == Some("true");
            opened |= is_open;
            broken |= lower_state == air && upper_state == air;
            let on_hero_side = if hero_right {
                body_x > f64::from(door.x) + 1.0
            } else {
                body_x < f64::from(door.x) - 1.0
            };
            crossed |= (!griefing || broken) && on_hero_side;
            if (!griefing && opened && crossed) || (griefing && cracking && broken && crossed) {
                break;
            }
            if griefing && on_hero_side && !breaking && !broken {
                // The player changes sides after an opening so the next approach meets the door again.
                world.tick_rate_mut().set_frozen(true);
                hero_right = !hero_right;
                command(
                    &sender,
                    &receiver,
                    &mut world,
                    &format!(
                        "tp hero {} {} {}",
                        f64::from(door.x) + if hero_right { 3.5 } else { -3.5 },
                        door.y,
                        f64::from(door.z) + 0.5,
                    ),
                );
                if is_open {
                    for (pos, half) in [(door, "lower"), (upper, "upper")] {
                        command(&sender, &receiver, &mut world, &format!(
                            "setblock {} {} {} minecraft:oak_door[facing=east,half={half},hinge=left,open=false,powered=false] strict",
                            pos.x, pos.y, pos.z,
                        ));
                    }
                    closures += 1;
                }
                world.tick_rate_mut().set_frozen(false);
            }
        }
        if griefing {
            assert!(cracking && broken && crossed,
                "Hard raid member cracks and removes both halves before crossing: cracking={cracking}, broken={broken}, crossed={crossed}, opened={opened}, closures={closures}, body={:?}, lower={:?}, upper={:?}",
                current_body(&world, id, uuid, raid_id).position(),
                world.block_state(DIMENSION, door), world.block_state(DIMENSION, upper));
            assert_eq!(world.block_state(DIMENSION, door), Some(air));
            assert_eq!(world.block_state(DIMENSION, upper), Some(air));
        } else {
            assert!(
                opened && crossed,
                "Normal raid member opens the door and walks through it"
            );
            assert!(
                !cracking && !broken,
                "mob_griefing=false preserves the wooden door"
            );
        }
        assert_eq!(world.player_health_for_test(HERO), hero_health);
        world.tick_rate_mut().set_frozen(true);
        command(&sender, &receiver, &mut world, &format!("kill {uuid}"));
    }
}
