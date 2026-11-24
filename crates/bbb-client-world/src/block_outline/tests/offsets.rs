use crate::BlockPos;

use super::super::shapes::outline_shape_for_state;
use super::support::state_id;

fn boxes(name: &str, pos: Option<BlockPos>) -> Vec<([f64; 3], [f64; 3])> {
    outline_shape_for_state(state_id(name, &Default::default()).unwrap(), pos)
        .unwrap()
        .boxes_min_max()
}

#[test]
fn flower_outline_moves_by_the_vanilla_position_offset() {
    // FlowerBlock.SHAPE is Block.column(6, 0, 10) moved by getOffset(pos).
    let centered = (
        [5.0 / 16.0, 0.0, 5.0 / 16.0],
        [11.0 / 16.0, 10.0 / 16.0, 11.0 / 16.0],
    );
    let close = |a: [f64; 3], b: [f64; 3]| (0..3).all(|axis| (a[axis] - b[axis]).abs() < 1e-9);
    let [(min, max)] = boxes("minecraft:poppy", None)[..] else {
        panic!("one poppy box");
    };
    assert!(close(min, centered.0) && close(max, centered.1));

    // Mth.getSeed(0, 0, 0) is 0, so BlockPos.ZERO sits at the -0.25 clamp.
    let [(min, max)] = boxes("minecraft:poppy", Some(BlockPos { x: 0, y: 64, z: 0 }))[..] else {
        panic!("one poppy box");
    };
    assert!(close(
        min,
        [centered.0[0] - 0.25, 0.0, centered.0[2] - 0.25]
    ));
    assert!(close(
        max,
        [centered.1[0] - 0.25, centered.1[1], centered.1[2] - 0.25]
    ));

    let shifted = boxes("minecraft:poppy", Some(BlockPos { x: 7, y: 64, z: -3 }));
    assert_ne!(
        shifted,
        boxes("minecraft:poppy", Some(BlockPos { x: 0, y: 64, z: 0 }))
    );
    // Short grass has an offset function but a static shape.
    assert_eq!(
        boxes(
            "minecraft:short_grass",
            Some(BlockPos { x: 7, y: 64, z: -3 })
        ),
        boxes("minecraft:short_grass", None)
    );
}

#[test]
fn dynamic_outline_shapes_follow_vanilla_owners() {
    let width = |name: &str, properties: &[(&str, &str)]| {
        let properties = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let shape = outline_shape_for_state(state_id(name, &properties).unwrap(), None).unwrap();
        let [(min, max)] = shape.boxes_min_max()[..] else {
            panic!("one {name} box");
        };
        (max[0] - min[0]) * 16.0
    };
    assert_eq!(width("minecraft:bamboo", &[("leaves", "none")]), 6.0);
    assert_eq!(width("minecraft:bamboo", &[("leaves", "large")]), 10.0);
    assert_eq!(
        width("minecraft:pointed_dripstone", &[("thickness", "base")]),
        12.0
    );
    assert!(outline_shape_for_state(
        state_id("minecraft:moving_piston", &Default::default()).unwrap(),
        None
    )
    .is_none());
}
