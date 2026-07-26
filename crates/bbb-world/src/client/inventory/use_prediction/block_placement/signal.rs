use super::super::WorldStore;
use crate::BlockPos;

pub(super) fn placement_neighbor_signal_is_proven_absent(
    world: &WorldStore,
    positions: &[BlockPos],
) -> Option<()> {
    // Vanilla 26.1 `SignalGetter.getSignal` can also relay a direct source
    // through a conductor. A radius-two audit around every queried position
    // covers its six neighbours and every direct-source neighbour. Any source
    // owner found in the 26.1 getSignal/getDirectSignal override audit keeps
    // this branch server-authoritative until shared signal evaluation can
    // resolve its direction and strength.
    for center in positions {
        for x in center.x.saturating_sub(2)..=center.x.saturating_add(2) {
            for y in center.y.saturating_sub(2)..=center.y.saturating_add(2) {
                for z in center.z.saturating_sub(2)..=center.z.saturating_add(2) {
                    let distance =
                        x.abs_diff(center.x) + y.abs_diff(center.y) + z.abs_diff(center.z);
                    if distance > 2 {
                        continue;
                    }
                    let block = world.probe_block(BlockPos { x, y, z })?;
                    if block
                        .block_name
                        .as_deref()
                        .is_some_and(possible_redstone_signal_source)
                    {
                        return None;
                    }
                }
            }
        }
    }
    Some(())
}

fn possible_redstone_signal_source(name: &str) -> bool {
    name.ends_with("_button")
        || name.ends_with("_pressure_plate")
        || matches!(
            name,
            "minecraft:redstone_wire"
                | "minecraft:lightning_rod"
                | "minecraft:lever"
                | "minecraft:redstone_torch"
                | "minecraft:redstone_wall_torch"
                | "minecraft:repeater"
                | "minecraft:comparator"
                | "minecraft:redstone_block"
                | "minecraft:jukebox"
                | "minecraft:daylight_detector"
                | "minecraft:sculk_sensor"
                | "minecraft:calibrated_sculk_sensor"
                | "minecraft:detector_rail"
                | "minecraft:trapped_chest"
                | "minecraft:tripwire_hook"
                | "minecraft:target"
                | "minecraft:lectern"
                | "minecraft:observer"
                | "minecraft:test_block"
        )
}
