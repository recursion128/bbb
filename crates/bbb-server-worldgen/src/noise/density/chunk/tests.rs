use crate::noise::beardifier::JigsawBeardifier;
use crate::noise::density::{ClimateSampler, NoiseDensitySource, Router, VeinRouter};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

use super::NoiseChunkSampler;

fn startup() -> DatapackStartup {
    DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap()
}

/// Drives `doFill` over one chunk and hands every block position to `visit`
/// with the sampler positioned on it.
fn drive_fill(
    sampler: &mut NoiseChunkSampler<'_>,
    chunk_start_x: i32,
    chunk_start_z: i32,
    noise_min_y: i32,
    noise_height: i32,
    beardifier: &JigsawBeardifier,
    mut visit: impl FnMut(&mut NoiseChunkSampler<'_>, i32, i32, i32),
) {
    let cell_width = sampler.cell_width;
    let cell_height = sampler.cell_height;
    let cell_count_xz = 16 / cell_width;
    let cell_min_y = noise_min_y.div_euclid(cell_height);
    let cell_count_y = noise_height / cell_height;
    sampler.initialize_for_first_cell_x();
    for cell_x in 0..cell_count_xz {
        sampler.advance_cell_x(cell_x);
        for cell_z in 0..cell_count_xz {
            for cell_y in (0..cell_count_y).rev() {
                sampler.select_cell_yz(cell_y, cell_z, beardifier);
                for in_y in (0..cell_height).rev() {
                    let y = (cell_min_y + cell_y) * cell_height + in_y;
                    sampler.update_for_y(y, f64::from(in_y) / f64::from(cell_height));
                    for in_x in 0..cell_width {
                        let x = chunk_start_x + cell_x * cell_width + in_x;
                        sampler.update_for_x(x, f64::from(in_x) / f64::from(cell_width));
                        for in_z in 0..cell_width {
                            let z = chunk_start_z + cell_z * cell_width + in_z;
                            sampler.update_for_z(z, f64::from(in_z) / f64::from(cell_width));
                            visit(sampler, x, y, z);
                        }
                    }
                }
            }
        }
        sampler.swap_slices();
    }
    sampler.stop_interpolation();
}

fn lerp(factor: f64, start: f64, end: f64) -> f64 {
    start + factor * (end - start)
}

#[test]
fn cell_fill_matches_the_uncached_interpolated_walk_bit_for_bit() {
    let startup = startup();
    for (settings, seed, chunk, min_y, height) in [
        ("minecraft:overworld", 0, (0, 0), -64, 384),
        ("minecraft:overworld", 42, (-3, 7), -64, 384),
        ("minecraft:overworld", 1_234_567, (11, -20), -64, 384),
        ("minecraft:nether", 42, (2, -1), 0, 128),
        ("minecraft:end", 7, (-40, 5), 0, 128),
    ] {
        let source =
            NoiseDensitySource::from_registry(&startup.registries, settings, seed).unwrap();
        let chunk_start_x = chunk.0 * 16;
        let chunk_start_z = chunk.1 * 16;
        // The uncached walk is the slow side; spread its columns over threads.
        let expected: Vec<Vec<u64>> = std::thread::scope(|scope| {
            (0..16)
                .map(|local_x| {
                    let source = &source;
                    scope.spawn(move || {
                        (0..16)
                            .flat_map(|local_z| {
                                (min_y..min_y + height).map(move |y| {
                                    source
                                        .sample_interpolated(
                                            chunk_start_x + local_x,
                                            y,
                                            chunk_start_z + local_z,
                                        )
                                        .to_bits()
                                })
                            })
                            .collect()
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect()
        });
        let beardifier = JigsawBeardifier::default();
        let mut sampler =
            NoiseChunkSampler::for_chunk(&source, chunk_start_x, chunk_start_z, min_y, height);
        let mut visited = 0;
        drive_fill(
            &mut sampler,
            chunk_start_x,
            chunk_start_z,
            min_y,
            height,
            &beardifier,
            |sampler, x, y, z| {
                visited += 1;
                let index = ((z - chunk_start_z) * height + (y - min_y)) as usize;
                assert_eq!(
                    sampler.interpolated_density().to_bits(),
                    expected[(x - chunk_start_x) as usize][index],
                    "{settings} seed {seed} at ({x}, {y}, {z})"
                );
            },
        );
        assert_eq!(visited, 16 * 16 * height);
    }
}

#[test]
fn vein_samples_lerp_the_uncached_corners_in_y_x_z_order() {
    let startup = startup();
    let source =
        NoiseDensitySource::from_registry(&startup.registries, "minecraft:overworld", 42).unwrap();
    let (cell_width, cell_height) = (source.cell_width(), source.cell_height());
    let corners = |x: i32, y: i32, z: i32, sample: &dyn Fn(i32, i32, i32) -> f64| {
        let x0 = x - x.rem_euclid(cell_width);
        let y0 = y - y.rem_euclid(cell_height);
        let z0 = z - z.rem_euclid(cell_width);
        let (x1, y1, z1) = (x0 + cell_width, y0 + cell_height, z0 + cell_width);
        let fx = f64::from(x - x0) / f64::from(cell_width);
        let fy = f64::from(y - y0) / f64::from(cell_height);
        let fz = f64::from(z - z0) / f64::from(cell_width);
        let xz00 = lerp(fy, sample(x0, y0, z0), sample(x0, y1, z0));
        let xz10 = lerp(fy, sample(x1, y0, z0), sample(x1, y1, z0));
        let xz01 = lerp(fy, sample(x0, y0, z1), sample(x0, y1, z1));
        let xz11 = lerp(fy, sample(x1, y0, z1), sample(x1, y1, z1));
        lerp(fz, lerp(fx, xz00, xz10), lerp(fx, xz01, xz11))
    };
    let mut sampler = NoiseChunkSampler::for_chunk(&source, -16, 32, -64, 384);
    let beardifier = JigsawBeardifier::default();
    let mut checked = 0;
    drive_fill(
        &mut sampler,
        -16,
        32,
        -64,
        384,
        &beardifier,
        |sampler, x, y, z| {
            if !(-60..=50).contains(&y) || (x + z) % 5 != 0 {
                return;
            }
            checked += 1;
            // Corner samples are cell aligned, where the uncached walk returns
            // the wrapped function itself.
            let toggle = corners(x, y, z, &|x, y, z| {
                source.sample_router(Router::VeinToggle, x, y, z)
            });
            let sampled = sampler.vein(Router::VeinToggle, x, y, z);
            assert_eq!(sampled.to_bits(), toggle.to_bits());
            let gap = source.sample_router(Router::VeinGap, x, y, z);
            assert_eq!(
                sampler.vein(Router::VeinGap, x, y, z).to_bits(),
                gap.to_bits()
            );
        },
    );
    assert!(checked > 1_000, "{checked}");
}

#[test]
fn chunk_climate_matches_the_uncached_climate_bit_for_bit() {
    let startup = startup();
    for (seed, chunk) in [(0, (0, 0)), (42, (-3, 7)), (99, (1_000, -1_000))] {
        let source = NoiseDensitySource::from_registry_with_climate(
            &startup.registries,
            "minecraft:overworld",
            seed,
        )
        .unwrap();
        let mut sampler =
            NoiseChunkSampler::for_chunk(&source, chunk.0 * 16, chunk.1 * 16, -64, 384);
        for quart_y in -16..80 {
            for local_x in 0..4 {
                for local_z in 0..4 {
                    let quart_x = chunk.0 * 4 + local_x;
                    let quart_z = chunk.1 * 4 + local_z;
                    assert_eq!(
                        sampler.climate_target(quart_x, quart_y, quart_z),
                        source.climate_target(quart_x, quart_y, quart_z),
                        "seed {seed} quart ({quart_x}, {quart_y}, {quart_z})"
                    );
                }
            }
        }
        let mut uncached = &source;
        assert_eq!(
            ClimateSampler::climate_target(&mut sampler, 3, 4, 5),
            ClimateSampler::climate_target(&mut uncached, 3, 4, 5)
        );
    }
}

#[test]
fn chunk_erosion_populates_cache_2d_and_matches_uncached_end_islands() {
    let startup = startup();
    let end =
        NoiseDensitySource::from_registry_with_climate(&startup.registries, "minecraft:end", 42)
            .unwrap();
    let mut sampler = NoiseChunkSampler::for_chunk(&end, 1600, -1600, 0, 128);
    let expected = end.sample_erosion(1608, 0, -1592).unwrap();
    for y in [0, 32, 64, 124] {
        assert_eq!(
            ClimateSampler::sample_erosion(&mut sampler, 1608, y, -1592),
            Some(expected)
        );
    }
    assert!(
        sampler.scratch.slots.iter().any(|slot| matches!(
            slot,
            super::Slot::Cache2d {
                last: Some(((1608, -1592), value))
            } if value.to_bits() == expected.to_bits()
        )),
        "End erosion sampling should hit NoiseChunk Cache2d"
    );
}
