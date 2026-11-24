use super::*;

#[test]
fn a_dispenser_brushes_adult_armadillos_and_keeps_its_brush_on_failure() {
    for age in [Some(0), Some(-100), None] {
        let mut world = NativeWorld::ticking(DROPPER);
        world.set(FLOOR, stone());
        world.set(UNDER_FRONT, stone());
        if let Some(age) = age {
            let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID;
            let (id, _) = world.world.spawn_stored_entity_for_test(
                bbb_server_types::save::Java26BuiltinDimension::Overworld,
                kind,
                bbb_protocol::packets::Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 1.5,
                },
                crate::entity::EntityData::GeneralLiving(
                    crate::entity::GeneralLivingData::new(kind).unwrap(),
                ),
            );
            let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
                .world
                .stored_entity_command_data_for_test(
                    bbb_server_types::save::Java26BuiltinDimension::Overworld,
                    id,
                )
                .unwrap()
            else {
                unreachable!()
            };
            for (name, value) in [
                ("Age", bbb_protocol::NbtSummaryValue::Int(age)),
                ("NoAI", bbb_protocol::NbtSummaryValue::Byte(1)),
            ] {
                entries.retain(|entry| entry.name != name);
                entries.push(bbb_protocol::NbtSummaryEntry {
                    name: name.into(),
                    value,
                });
            }
            world.world.load_stored_entity_record_for_test(
                bbb_server_types::save::Java26BuiltinDimension::Overworld,
                id,
                &entries,
            );
        }
        world.set(DROPPER, dispenser("south", false));
        let mut slots = empty_nine();
        slots[0] = stack("minecraft:brush", 1);
        set_items(&mut world, DROPPER, slots);
        let redstone = super::super::default_state("minecraft:redstone_block");
        world.set(SIDE, redstone);
        world.neighbor_changed(DROPPER, block_type_of(redstone).unwrap());
        for _ in 0..5 {
            world.run_tick();
        }
        let brush = held(&mut world);
        assert_eq!(brush.item_id, stack("minecraft:brush", 1).item_id);
        assert_eq!(brush.count, 1);
        assert_eq!(brush.component_patch.damage, (age == Some(0)).then_some(16));
        let drops: Vec<_> = world
            .entities()
            .into_iter()
            .filter_map(|entity| {
                let crate::entity::EntityData::Item(data) = entity.data() else {
                    return None;
                };
                Some(data.item().summary().clone())
            })
            .collect();
        assert_eq!(
            drops,
            if age == Some(0) {
                vec![stack("minecraft:armadillo_scute", 1)]
            } else {
                vec![]
            }
        );
    }
}
