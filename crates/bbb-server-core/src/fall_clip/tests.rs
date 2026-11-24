use super::*;
use crate::fluid::SharedFluidShapeCache;

fn point(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}
fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}
fn resetting_state(name: &str, properties: &[(&str, &str)]) -> i32 {
    if properties.is_empty() {
        let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
        return bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    }
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
    .unwrap()
}

fn resetting_membership(registry: &str, _: &str, resource: &str) -> bool {
    registry == "minecraft:fluid"
        && matches!(resource, "minecraft:water" | "minecraft:flowing_water")
}

#[test]
fn fall_resetting_clip_uses_full_tag_shape_instead_of_block_collision() {
    let from = point(0.5, 0.5, 0.5);
    let to = point(2.5, 0.5, 0.5);
    for (block, member, expected) in [
        ("minecraft:stone", false, false),
        ("minecraft:vine", true, true),
    ] {
        let state = resetting_state(block, &[]);
        assert_eq!(
            fall_resetting_clip_with(
                from,
                to,
                FallResettingContext::Stored,
                &SharedFluidShapeCache::default(),
                |_| Some(state),
                |registry, _, _| registry == "minecraft:block" && member
            ),
            Some(expected)
        );
    }
}

#[test]
fn fall_resetting_clip_player_portal_rules_never_apply_to_stored_bodies() {
    for name in [
        "minecraft:end_gateway",
        "minecraft:end_portal",
        "minecraft:nether_portal",
    ] {
        let state = resetting_state(name, &[]);
        for (context, expected) in [
            (FallResettingContext::Stored, false),
            (
                FallResettingContext::Player {
                    nether_portal_delay_zero: false,
                },
                name != "minecraft:nether_portal",
            ),
            (
                FallResettingContext::Player {
                    nether_portal_delay_zero: true,
                },
                true,
            ),
        ] {
            assert_eq!(
                fall_resetting_clip_with(
                    point(0.5, 0.5, 0.5),
                    point(2.5, 0.5, 0.5),
                    context,
                    &SharedFluidShapeCache::default(),
                    |_| Some(state),
                    resetting_membership
                ),
                Some(expected),
                "{name} {context:?}"
            );
        }
    }
}

#[test]
fn fall_resetting_clip_water_includes_flowing_and_waterlogged_shapes_and_live_above() {
    let air = resetting_state("minecraft:air", &[]);
    let water = resetting_state("minecraft:water", &[]);
    let flowing = resetting_state("minecraft:water", &[("level", "1")]);
    let slab = resetting_state(
        "minecraft:stone_slab",
        &[("type", "bottom"), ("waterlogged", "true")],
    );
    for (state, ray_y) in [(flowing, 0.8), (water, 0.9), (slab, 0.9)] {
        for (above, expected) in [(air, false), (water, true)] {
            assert_eq!(
                fall_resetting_clip_with(
                    point(0.5, ray_y, 0.5),
                    point(2.5, ray_y, 0.5),
                    FallResettingContext::Stored,
                    &SharedFluidShapeCache::default(),
                    |pos| Some(if pos.y == 0 { state } else { above }),
                    resetting_membership
                ),
                Some(expected)
            );
        }
    }
    assert_eq!(
        fall_resetting_clip_with(
            point(0.5, 0.7, 0.5),
            point(2.5, 0.7, 0.5),
            FallResettingContext::Stored,
            &SharedFluidShapeCache::default(),
            |pos| Some(if pos.y == 0 { flowing } else { air }),
            resetting_membership
        ),
        Some(true)
    );
}

#[test]
fn fall_resetting_clip_unknown_states_and_water_above_refuse() {
    let from = point(0.5, 0.5, 0.5);
    let to = point(2.5, 0.5, 0.5);
    let water = resetting_state("minecraft:water", &[]);
    assert_eq!(
        fall_resetting_clip_with(
            from,
            to,
            FallResettingContext::Stored,
            &SharedFluidShapeCache::default(),
            |_| None,
            resetting_membership
        ),
        None
    );
    assert_eq!(
        fall_resetting_clip_with(
            from,
            to,
            FallResettingContext::Stored,
            &SharedFluidShapeCache::default(),
            |_| Some(i32::MAX),
            resetting_membership
        ),
        None
    );
    assert_eq!(
        fall_resetting_clip_with(
            from,
            to,
            FallResettingContext::Stored,
            &SharedFluidShapeCache::default(),
            |pos| if pos.y == 0 { Some(water) } else { None },
            resetting_membership
        ),
        None
    );
}

#[test]
fn fall_resetting_clip_reads_both_shapes_but_stops_after_first_hit() {
    let air = resetting_state("minecraft:air", &[]);
    let stone = resetting_state("minecraft:stone", &[]);
    let from = point(0.5, 0.5, 0.5);
    let to = point(4.5, 0.5, 0.5);
    let state_at = |pos: BlockPos| match pos.x {
        0 => Some(air),
        1 => Some(stone),
        _ => panic!("cells beyond first hit must be lazy"),
    };
    assert_eq!(
        fall_resetting_clip_with(
            from,
            to,
            FallResettingContext::Stored,
            &SharedFluidShapeCache::default(),
            state_at,
            |registry, _, resource| registry == "minecraft:block" && resource == "minecraft:stone"
        ),
        Some(true)
    );
    assert_eq!(
        fall_resetting_clip_with(
            from,
            from,
            FallResettingContext::Stored,
            &SharedFluidShapeCache::default(),
            |_| panic!("zero segment has no state query"),
            |_, _, _| panic!("zero segment has no tag query")
        ),
        Some(false)
    );
}

#[test]
fn checked_fluid_clip_shape_empty_is_lazy_and_unknown_above_is_distinct() {
    use crate::fluid::checked_fluid_shape;
    use bbb_server_types::fluid::{FluidFamily, FluidState};
    assert_eq!(
        checked_fluid_shape(FluidState::Empty, pos(0, 0, 0), |_| panic!(
            "empty has no above read"
        )),
        Some(None)
    );
    let source = FluidState::source(FluidFamily::Water, false);
    assert_eq!(checked_fluid_shape(source, pos(0, 0, 0), |_| None), None);
    assert_eq!(
        checked_fluid_shape(source, pos(0, 0, 0), |_| Some(i32::MAX)),
        None
    );
    let air = resetting_state("minecraft:air", &[]);
    let shape = checked_fluid_shape(source, pos(0, 0, 0), |_| Some(air))
        .unwrap()
        .unwrap();
    assert_eq!(shape.max_y, f64::from(8.0_f32 / 9.0_f32));
}

#[test]
fn fall_resetting_clip_uses_live_fluid_tag_instead_of_builtin_water_identity() {
    let air = resetting_state("minecraft:air", &[]);
    for (name, member, expected) in [
        ("minecraft:water", false, false),
        ("minecraft:lava", true, true),
        ("minecraft:air", true, false),
    ] {
        let state = resetting_state(name, &[]);
        assert_eq!(
            fall_resetting_clip_with(
                point(0.5, 0.5, 0.5),
                point(2.5, 0.5, 0.5),
                FallResettingContext::Stored,
                &SharedFluidShapeCache::default(),
                |pos| {
                    if name == "minecraft:air" {
                        assert_eq!(pos.y, 0, "tagged empty fluid must not read above");
                    }
                    Some(if pos.y == 0 { state } else { air })
                },
                |registry, _, _| registry == "minecraft:fluid" && member
            ),
            Some(expected)
        );
    }
}
