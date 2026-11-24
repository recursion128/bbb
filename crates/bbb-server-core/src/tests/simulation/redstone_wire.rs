use super::*;

#[test]
fn a_live_wire_junction_powers_and_clears_after_its_source_is_removed() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let source = BlockPos { x: 1, y: 64, z: 8 };
    let wires: Vec<_> = (2..=13)
        .map(|x| BlockPos { x, y: 64, z: 8 })
        .chain((9..=13).map(|z| BlockPos { x: 8, y: 64, z }))
        .collect();
    for pos in &wires {
        let floor = BlockPos {
            y: pos.y - 1,
            ..*pos
        };
        world.set_block(
            Java26BuiltinDimension::Overworld,
            floor,
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL,
        );
        assert_eq!(
            live_block(&mut world, floor),
            Some(block_state_id("minecraft:stone"))
        );
    }
    for pos in &wires {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            *pos,
            helpers::default_block_state("minecraft:redstone_wire"),
            SET_BLOCK_UPDATE_ALL,
        ));
    }
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        source,
        block_state_id("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    ));
    for (pos, expected) in [
        (BlockPos { x: 2, y: 64, z: 8 }, 15),
        (BlockPos { x: 8, y: 64, z: 8 }, 9),
        (BlockPos { x: 13, y: 64, z: 8 }, 4),
        (BlockPos { x: 8, y: 64, z: 13 }, 4),
    ] {
        let state = live_block(&mut world, pos).unwrap();
        assert_eq!(
            bbb_server_types::block_queries::int_property(state, "power"),
            Some(expected)
        );
    }
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        source,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
    for pos in wires {
        let state = live_block(&mut world, pos).unwrap();
        assert_eq!(
            bbb_server_types::block_queries::int_property(state, "power"),
            Some(0)
        );
    }
}
