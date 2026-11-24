//! Vanilla `EndSpikeFeature` (`Feature.END_SPIKE`, `Feature.java:107`): the
//! ten obsidian pillars of the End, their cages, and the crystals on top.

use crate::random::WorldgenRandom;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::{EndSpike, EndSpikeConfig};

use super::{
    between_closed, block_set_bool, catalog_default_state, default_block_state, mth_floor_f64,
    offset_block_pos, util_shuffle, WorldGenLevel, FEATURE_SET_BLOCK,
};

/// `EndSpikeFeature.NUMBER_OF_SPIKES` / `SPIKE_DISTANCE` (`EndSpikeFeature.java:32-33`).
const NUMBER_OF_SPIKES: i32 = 10;
const SPIKE_DISTANCE: f64 = 42.0;

/// `EndSpike.isCenterWithinChunk`.
fn is_center_within_chunk(spike: &EndSpike, chunk_origin: BlockPos) -> bool {
    chunk_origin.x >> 4 == spike.center_x >> 4 && chunk_origin.z >> 4 == spike.center_z >> 4
}

/// Vanilla `EndSpikeFeature.getSpikesForLevel`: ten pillars on a circle,
/// their sizes shuffled per world.
pub fn spikes_for_seed(level_seed: i64) -> Vec<EndSpike> {
    let mut sizes: Vec<i32> = (0..NUMBER_OF_SPIKES).collect();
    util_shuffle(
        &mut sizes,
        &mut WorldgenRandom::named(level_seed, "minecraft:end_spikes"),
    );
    (0..NUMBER_OF_SPIKES)
        .map(|i| {
            let angle =
                2.0 * (-std::f64::consts::PI + (std::f64::consts::PI / 10.0) * f64::from(i));
            let x = mth_floor_f64(SPIKE_DISTANCE * angle.cos());
            let z = mth_floor_f64(SPIKE_DISTANCE * angle.sin());
            let size = sizes[i as usize];
            EndSpike {
                center_x: x,
                center_z: z,
                radius: 2 + size / 3,
                height: 76 + size * 3,
                guarded: size == 1 || size == 2,
            }
        })
        .collect()
}

/// Vanilla `EndSpikeFeature.place` (`EndSpikeFeature.java:49-66`).
pub fn place_end_spike<L: WorldGenLevel>(
    config: &EndSpikeConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let spikes = if config.spikes.is_empty() {
        let seed = level.get_seed();
        spikes_for_seed(seed)
    } else {
        config.spikes.clone()
    };
    for spike in spikes {
        if is_center_within_chunk(&spike, origin) {
            place_spike(level, random, config, spike)?;
        }
    }
    Some(true)
}

/// Vanilla `EndSpikeFeature.placeSpike` (`EndSpikeFeature.java:68-120`);
/// `Feature.setBlock` is flag 3 (`Feature.java:169-171`).
fn place_spike<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    config: &EndSpikeConfig,
    spike: EndSpike,
) -> Option<()> {
    let radius = spike.radius;
    let obsidian = default_block_state("minecraft:obsidian");
    let air = default_block_state("minecraft:air");
    for pos in between_closed(
        BlockPos {
            x: spike.center_x - radius,
            y: level.get_min_y(),
            z: spike.center_z - radius,
        },
        BlockPos {
            x: spike.center_x + radius,
            y: spike.height + 10,
            z: spike.center_z + radius,
        },
    ) {
        let dx = f64::from(pos.x) - f64::from(spike.center_x);
        let dz = f64::from(pos.z) - f64::from(spike.center_z);
        if dx * dx + dz * dz <= f64::from(radius * radius + 1) && pos.y < spike.height {
            let _ = level.set_block(pos, obsidian, FEATURE_SET_BLOCK);
        } else if pos.y > 65 {
            let _ = level.set_block(pos, air, FEATURE_SET_BLOCK);
        }
    }
    if spike.guarded {
        let iron_bars = catalog_default_state("minecraft:iron_bars");
        for dx in -2_i32..=2 {
            for dz in -2_i32..=2 {
                for dy in 0..=3 {
                    let is_x_side = dx.abs() == 2;
                    let is_z_side = dz.abs() == 2;
                    let top = dy == 3;
                    if is_x_side || is_z_side || top {
                        let x_edge = dx == -2 || dx == 2 || top;
                        let z_edge = dz == -2 || dz == 2 || top;
                        let mut state = iron_bars;
                        state = block_set_bool(state, "north", x_edge && dz != -2);
                        state = block_set_bool(state, "south", x_edge && dz != 2);
                        state = block_set_bool(state, "west", z_edge && dx != -2);
                        state = block_set_bool(state, "east", z_edge && dx != 2);
                        let _ = level.set_block(
                            BlockPos {
                                x: spike.center_x + dx,
                                y: spike.height + dy,
                                z: spike.center_z + dz,
                            },
                            state,
                            FEATURE_SET_BLOCK,
                        );
                    }
                }
            }
        }
    }
    // `EntityType.END_CRYSTAL.create` never fails in Vanilla, so the yaw draw
    // and the bedrock/fire writes always follow.
    let yaw = random.next_float() * 360.0;
    let crystal_pos = BlockPos {
        x: spike.center_x,
        y: spike.height + 1,
        z: spike.center_z,
    };
    level.add_end_crystal(
        [
            f64::from(spike.center_x) + 0.5,
            f64::from(spike.height + 1),
            f64::from(spike.center_z) + 0.5,
        ],
        yaw,
        config.crystal_beam_target,
        config.crystal_invulnerable,
    );
    let below = offset_block_pos(crystal_pos, Direction::Down);
    let _ = level.set_block(
        below,
        default_block_state("minecraft:bedrock"),
        FEATURE_SET_BLOCK,
    );
    let fire = fire_state(level, crystal_pos)?;
    let _ = level.set_block(crystal_pos, fire, FEATURE_SET_BLOCK);
    Some(())
}

/// `BaseFireBlock.getState` (`BaseFireBlock.java:45-49`): soul fire over a
/// `soul_fire_base_blocks` block (`SoulFireBlock.java:44-46`), otherwise
/// `FireBlock.getStateForPlacement` (`FireBlock.java:115-132`), whose
/// neighbour-flammability branch is unreachable over the bedrock just placed.
fn fire_state<L: WorldGenLevel>(level: &L, pos: BlockPos) -> Option<i32> {
    level.fire_state_for_placement(pos)
}

#[cfg(test)]
mod tests {
    use super::spikes_for_seed;

    #[test]
    fn spike_ring_matches_the_ten_vanilla_pillar_centres() {
        let spikes = spikes_for_seed(0);
        let centres: Vec<(i32, i32)> = spikes.iter().map(|s| (s.center_x, s.center_z)).collect();
        assert_eq!(
            centres,
            vec![
                (42, 0),
                (33, 24),
                (12, 39),
                (-13, 39),
                (-34, 24),
                // `Math.sin(-Math.PI)` is -1.2e-16, so the floor lands on -1.
                (-42, -1),
                (-34, -25),
                (-13, -40),
                (12, -40),
                (33, -25),
            ]
        );
        let mut sizes: Vec<i32> = spikes.iter().map(|s| (s.height - 76) / 3).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, (0..10).collect::<Vec<_>>());
        assert_eq!(spikes.iter().filter(|s| s.guarded).count(), 2);
    }
}
