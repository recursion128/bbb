//! Fox mouth food through normal AI ticks and actual equipment publication.

use super::*;
use crate::tests::using_item::packets_to;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::sound_event_types::vanilla_sound_event_resource_id;

use super::helpers::console_command_ok as command;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

#[test]
fn fox_food_clears_poison_preserves_remaining_food_and_only_replaces_an_empty_mouth() {
    for (no_ai, count) in [(false, 1), (false, 2), (true, 1)] {
        let (sender, receiver, mut world) = helpers::frozen_observer_world(&[
            "gamerule spawn_mobs false",
            "gamerule advance_time false",
            "gamerule random_tick_speed 0",
            "time set midnight",
            "weather clear",
            "tp observer 12.5 65.0 8.5",
            "forceload add 0 0 15 15",
            "fill 0 64 0 15 68 15 minecraft:stone",
            "fill 0 65 0 15 68 15 minecraft:air",
            "summon minecraft:fox 4.5 65.0 8.5 {NoAI:1b,PersistenceRequired:1b}",
        ]);
        let id = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(DIM, *id)
                    .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_FOX_ID)
            })
            .unwrap();
        let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        for line in [
            format!("attribute {uuid} minecraft:movement_speed base set 0"),
            format!("data merge entity {uuid} {{NoAI:0b}}"),
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
        world.tick_rate_mut().set_frozen(true);
        assert!(world.stored_entity_for_test(DIM, id).unwrap().on_ground());
        if no_ai {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:1b}}"),
            );
        }
        for line in [
            format!("item replace entity {uuid} weapon.mainhand with minecraft:honey_bottle[minecraft:custom_data={{fox_food_marker:17}}] {count}"),
            format!("effect give {uuid} minecraft:poison 600"),
        ] {
            command(&sender, &receiver, &mut world, &line);
        }
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        assert!(
            matches!(body.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
        );
        let before = body
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand)
            .unwrap()
            .clone();
        assert_eq!(before.count(), count);
        assert!(body
            .living()
            .unwrap()
            .active_effect("minecraft:poison")
            .is_some());
        let expected = if count == 1 {
            ItemStackSummary {
                item_id: Some(item_id("minecraft:glass_bottle")),
                count: 1,
                component_patch: Default::default(),
            }
        } else {
            before.with_count(count - 1).summary().clone()
        };
        let (mut completed, mut published, mut heard, mut burped) = (false, false, false, false);
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..650 {
            let tick = world.tick(receiver.drain());
            for packet in packets_to(&tick, 1) {
                match packet {
                    ClientboundPlayPacket::SetEquipment(update) if update.entity_id == id => {
                        published |= update.slots.iter().any(|slot| {
                            slot.slot == EquipmentSlot::MainHand && slot.item == expected
                        });
                    }
                    ClientboundPlayPacket::Sound(sound) => {
                        let name = match &sound.sound {
                            SoundEventHolder::Reference { registry_id } => {
                                vanilla_sound_event_resource_id(*registry_id)
                            }
                            SoundEventHolder::Direct { location, .. } => Some(location.as_str()),
                        };
                        heard |= name == Some("minecraft:item.honey_bottle.drink");
                        burped |= name == Some("minecraft:entity.player.burp");
                    }
                    _ => {}
                }
            }
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.is_alive_with_health());
            assert!(
                matches!(body.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
            );
            let mouth = body
                .living()
                .unwrap()
                .item_by_slot(EquipmentSlot::MainHand)
                .unwrap();
            let poisoned = body
                .living()
                .unwrap()
                .active_effect("minecraft:poison")
                .is_some();
            if no_ai {
                assert_eq!(mouth, &before);
                assert!(poisoned);
            } else {
                completed |= mouth.summary() == &expected && !poisoned;
                if completed && published && heard {
                    break;
                }
            }
        }
        assert!(!burped);
        if no_ai {
            assert!(!completed && !published && !heard);
        } else {
            assert!(
                completed && published && heard,
                "completed={completed}, published={published}, heard={heard}"
            );
        }
        assert!(!world
            .stored_items_for_test(DIM)
            .iter()
            .any(|item| item.2 == "minecraft:glass_bottle"));
    }
}
