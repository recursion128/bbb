//! Fox mouth stack preservation through real stored-entity deaths.

use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;

#[test]
fn a_fox_death_preserves_its_whole_mouth_stack_without_ordinary_loot() {
    for held in [true, false] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config());
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        stand_at(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 8.5,
                ..MOB_POSITION
            },
        );
        for command in ["gamerule spawn_mobs false", "gamerule mob_drops false"] {
            let operation = world.publication().tick;
            let tick = console_command(&sender, &receiver, &mut world, operation, command);
            let outcome = admin_outcome(&tick);
            assert!(!outcome.replayed, "{command}");
            let Ok(AdminResult::Command(report)) = &outcome.result else {
                panic!("{command}: {outcome:?}");
            };
            assert!(report.result.is_some(), "{command}: {report:?}");
        }
        let fox = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_FOX_ID);
        let record = if held {
            r#"{NoAI:1b,Age:-24000,equipment:{mainhand:{id:"minecraft:diamond",count:7,components:{"minecraft:custom_data":{kept:17}}}},drop_chances:{mainhand:0f}}"#
        } else {
            "{NoAI:1b,Age:-24000,drop_chances:{mainhand:0f}}"
        };
        let NbtSummaryValue::Compound(record) = bbb_protocol::parse_snbt_compound(record).unwrap()
        else {
            panic!("the fox record is a compound");
        };
        load_record(&mut world, fox, record);
        let mouth = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, fox)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::MainHand)
            .cloned();
        if held {
            let stack = mouth.as_ref().expect("the saved mouth stack loads");
            assert_eq!(stack.summary().count, 7);
            assert_eq!(
                crate::tests::saved_item_component(&world, stack, "minecraft:custom_data"),
                Some(NbtSummaryValue::Compound(vec![nbt_entry(
                    "kept",
                    NbtSummaryValue::Int(17)
                )]))
            );
        } else {
            assert!(mouth.is_none());
        }

        kill_stored(&mut world, fox);

        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, fox)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::MainHand)
            .is_none());
        let dropped = items(&world);
        assert_eq!(dropped.len(), usize::from(held), "{dropped:?}");
        if let Some(drop) = dropped.first() {
            assert_eq!(drop.1, MOB_POSITION);
            assert_eq!(drop.4, 10);
            assert_eq!(
                world.stored_item_stack_for_test(Java26BuiltinDimension::Overworld, drop.0),
                mouth
            );
        }
        for _ in 0..30 {
            world.tick(receiver.drain());
        }
        let settled = items(&world);
        assert_eq!(settled.len(), dropped.len(), "{settled:?}");
        if let Some(drop) = settled.first() {
            assert_eq!(drop.0, dropped[0].0);
            assert_eq!(
                world.stored_item_stack_for_test(Java26BuiltinDimension::Overworld, drop.0),
                mouth
            );
        }
    }
}
