//! Actual fox foraging through the normal world AI path.

use super::*;
use crate::entity::EntityData;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::sound_event_types::vanilla_sound_event_resource_id;

use super::helpers::console_command_ok as command;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const CROP: BlockPos = BlockPos { x: 10, y: 65, z: 8 };

fn ground_count(world: &ServerWorld, item: i32) -> i32 {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(
            |id| match world.stored_entity_for_test(DIM, id).unwrap().data() {
                EntityData::Item(ground) if ground.stack().summary().item_id == Some(item) => {
                    Some(ground.stack().count())
                }
                _ => None,
            },
        )
        .sum()
}

#[test]
fn fox_berries_normal_ai_approaches_and_harvests_sweet_and_glow_crops() {
    for glow in [false, true] {
        let (sender, receiver, mut world) = helpers::frozen_observer_world(&[
            "gamerule spawn_mobs false",
            "gamerule advance_time false",
            "gamerule random_tick_speed 0",
            "time set midnight",
            "weather clear",
            "tp observer 13.5 68 8.5",
            "forceload add 0 0 15 15",
            "fill 1 64 6 14 68 10 minecraft:stone",
            "fill 2 65 7 13 67 9 minecraft:air",
            "summon minecraft:fox 5.5 65 8.5 {NoAI:1b,CanPickUpLoot:1b,PersistenceRequired:1b}",
        ]);
        if glow {
            command(
                &sender,
                &receiver,
                &mut world,
                "setblock 10 66 8 minecraft:stone",
            );
            command(
                &sender,
                &receiver,
                &mut world,
                "setblock 10 65 8 minecraft:cave_vines[berries=true]",
            );
        } else {
            command(
                &sender,
                &receiver,
                &mut world,
                "setblock 10 64 8 minecraft:dirt",
            );
            command(
                &sender,
                &receiver,
                &mut world,
                "setblock 10 65 8 minecraft:sweet_berry_bush[age=3]",
            );
        }
        let initial_crop = world.block_state(DIM, CROP).unwrap();
        let id = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(DIM, *id)
                    .unwrap()
                    .entity_type_id()
                    == VANILLA_ENTITY_TYPE_FOX_ID
            })
            .unwrap();
        let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        for line in [
            format!("attribute {uuid} minecraft:movement_speed base set 0"),
            format!("data merge entity {uuid} {{NoAI:0b}}"),
            format!("item replace entity {uuid} weapon.mainhand with minecraft:air"),
        ] {
            command(&sender, &receiver, &mut world, &line);
        }
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..20 {
            world.tick(receiver.drain());
            if world.stored_entity_for_test(DIM, id).unwrap().on_ground() {
                break;
            }
        }
        assert!(world.stored_entity_for_test(DIM, id).unwrap().on_ground());
        world.tick_rate_mut().set_frozen(true);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{NoAI:1b}}"),
        );
        let start = world.stored_entity_for_test(DIM, id).unwrap().position();
        world.tick_rate_mut().set_frozen(false);
        let item = item_id(if glow {
            "minecraft:glow_berries"
        } else {
            "minecraft:sweet_berries"
        });
        for _ in 0..100 {
            world.tick(receiver.drain());
            assert_eq!(world.block_state(DIM, CROP), Some(initial_crop));
            assert_eq!(
                world.stored_entity_for_test(DIM, id).unwrap().position(),
                start
            );
            assert_eq!(ground_count(&world, item), 0);
        }
        world.tick_rate_mut().set_frozen(true);
        for line in [
            format!("attribute {uuid} minecraft:movement_speed base set 0.2"),
            format!("data merge entity {uuid} {{NoAI:0b}}"),
        ] {
            command(&sender, &receiver, &mut world, &line);
        }
        world.tick_rate_mut().set_frozen(false);
        let expected_sound = if glow {
            "minecraft:block.cave_vines.pick_berries"
        } else {
            "minecraft:block.sweet_berry_bush.pick_berries"
        };
        let (mut moved, mut harvested, mut heard) = (false, false, false);
        for _ in 0..1600 {
            let tick = world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            moved |= (body.position().x - start.x).hypot(body.position().z - start.z) > 0.5;
            heard |= tick.events.iter().any(|event| {
                let sound = match event {
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::Sound(sound),
                    } => &sound.sound,
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::SoundEntity(sound),
                    } => &sound.sound,
                    _ => return false,
                };
                match sound {
                    SoundEventHolder::Reference { registry_id } => {
                        vanilla_sound_event_resource_id(*registry_id) == Some(expected_sound)
                    }
                    SoundEventHolder::Direct { location, .. } => location == expected_sound,
                }
            });
            if world.block_state(DIM, CROP) != Some(initial_crop) {
                harvested = true;
                break;
            }
        }
        assert!(
            moved && harvested && heard,
            "glow={glow}: moved={moved}, harvested={harvested}, heard={heard}"
        );
        let state = world.block_state(DIM, CROP).unwrap();
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        assert_eq!(body.uuid(), uuid);
        assert!(body.is_alive_with_health());
        let mouth = body.living().unwrap().item_by_slot(EquipmentSlot::MainHand);
        let held = mouth
            .filter(|stack| stack.summary().item_id == Some(item))
            .map_or(0, |stack| stack.count());
        let ground = ground_count(&world, item);
        if glow {
            assert!(!crate::block_behavior::growing_plant::has_berries(state));
            assert_eq!(held + ground, 1);
        } else {
            assert_eq!(crate::block_behavior::sweet_berry_bush::age(state), Some(1));
            assert_eq!(held, 1);
            assert!((1..=2).contains(&ground));
            assert!(mouth.unwrap().summary().component_patch.is_empty());
        }
    }
}
