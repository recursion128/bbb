use crate::noise::SimplexNoise;
use crate::random::WorldgenRandom;

/// `DensityFunctions.EndIslandDensityFunction` (`DensityFunctions.java:525-567`).
pub struct EndIslandDensity {
    island_noise: SimplexNoise,
}

impl EndIslandDensity {
    pub fn new(world_seed: i64) -> Self {
        let mut random = WorldgenRandom::named(world_seed, "minecraft:end_islands");
        Self {
            island_noise: SimplexNoise::new(&mut random),
        }
    }

    pub fn sample(&self, block_x: i32, block_z: i32) -> f64 {
        let section_x = block_x / 8;
        let section_z = block_z / 8;
        (f64::from(self.height_value(section_x, section_z)) - 8.0) / 128.0
    }

    fn height_value(&self, section_x: i32, section_z: i32) -> f32 {
        // Java integer division and remainder truncate toward zero at negative coordinates.
        let chunk_x = section_x / 2;
        let chunk_z = section_z / 2;
        let sub_section_x = section_x % 2;
        let sub_section_z = section_z % 2;
        let squared = section_x
            .wrapping_mul(section_x)
            .wrapping_add(section_z.wrapping_mul(section_z));
        let mut offset = clamp(100.0 - (squared as f32).sqrt() * 8.0, -100.0, 80.0);

        for x_offset in -12..=12 {
            for z_offset in -12..=12 {
                let total_chunk_x = i64::from(chunk_x + x_offset);
                let total_chunk_z = i64::from(chunk_z + z_offset);
                if total_chunk_x * total_chunk_x + total_chunk_z * total_chunk_z > 4096
                    && self
                        .island_noise
                        .get_value(total_chunk_x as f64, total_chunk_z as f64)
                        < f64::from(-0.9_f32)
                {
                    let island_size = ((total_chunk_x as f32).abs() * 3439.0
                        + (total_chunk_z as f32).abs() * 147.0)
                        % 13.0
                        + 9.0;
                    let x_distance = (sub_section_x - x_offset * 2) as f32;
                    let z_distance = (sub_section_z - z_offset * 2) as f32;
                    let candidate = clamp(
                        100.0
                            - (x_distance * x_distance + z_distance * z_distance).sqrt()
                                * island_size,
                        -100.0,
                        80.0,
                    );
                    offset = java_max(offset, candidate);
                }
            }
        }

        offset
    }
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value.is_nan() {
        value
    } else if value > max {
        max
    } else {
        value
    }
}

fn java_max(first: f32, second: f32) -> f32 {
    if first.is_nan() {
        first
    } else if second.is_nan() {
        second
    } else if first == 0.0 && second == 0.0 {
        if first.is_sign_positive() || second.is_sign_positive() {
            0.0
        } else {
            -0.0
        }
    } else if second > first {
        second
    } else {
        first
    }
}
