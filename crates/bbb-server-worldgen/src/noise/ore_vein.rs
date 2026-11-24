//! Overworld ore-vein material rule (`OreVeinifier.java:23-60`).

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

use super::density::{NoiseDensitySource, Router, VeinRouter};
use crate::random::PositionalRandom;

pub struct VeinType {
    ore: i32,
    raw_ore: i32,
    filler: i32,
    min_y: i32,
    max_y: i32,
}

pub struct OreVein {
    random: PositionalRandom,
    copper: VeinType,
    iron: VeinType,
}

impl OreVein {
    pub fn new(source: &NoiseDensitySource) -> Option<Self> {
        source.ore_veins_enabled().then(|| Self {
            random: PositionalRandom::new(source.world_seed(), "minecraft:ore"),
            copper: VeinType {
                ore: state("minecraft:copper_ore"),
                raw_ore: state("minecraft:raw_copper_block"),
                filler: state("minecraft:granite"),
                min_y: 0,
                max_y: 50,
            },
            iron: VeinType {
                ore: state("minecraft:deepslate_iron_ore"),
                raw_ore: state("minecraft:raw_iron_block"),
                filler: state("minecraft:tuff"),
                min_y: -60,
                max_y: -8,
            },
        })
    }

    pub fn compute(&self, source: &NoiseDensitySource, x: i32, y: i32, z: i32) -> Option<i32> {
        self.compute_with(&mut &*source, x, y, z)
    }

    pub fn compute_with(
        &self,
        router: &mut impl VeinRouter,
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<i32> {
        let toggle = router.vein(Router::VeinToggle, x, y, z);
        let vein = if toggle > 0.0 {
            &self.copper
        } else {
            &self.iron
        };
        let distance_from_top = vein.max_y - y;
        let distance_from_bottom = y - vein.min_y;
        if distance_from_bottom < 0 || distance_from_top < 0 {
            return None;
        }
        let distance_from_edge = distance_from_top.min(distance_from_bottom);
        let edge_roundoff = clamped_map(
            f64::from(distance_from_edge),
            0.0,
            20.0,
            f64::from(-0.2_f32),
            0.0,
        );
        let veininess = toggle.abs();
        if veininess + edge_roundoff < f64::from(0.4_f32) {
            return None;
        }
        let mut random = self.random.at(x, y, z);
        if random.next_float() > 0.7_f32 || router.vein(Router::VeinRidged, x, y, z) >= 0.0 {
            return None;
        }
        let richness = clamped_map(
            veininess,
            f64::from(0.4_f32),
            f64::from(0.6_f32),
            f64::from(0.1_f32),
            f64::from(0.3_f32),
        );
        if f64::from(random.next_float()) < richness
            && router.vein(Router::VeinGap, x, y, z) > f64::from(-0.3_f32)
        {
            if random.next_float() < 0.02_f32 {
                Some(vein.raw_ore)
            } else {
                Some(vein.ore)
            }
        } else {
            Some(vein.filler)
        }
    }
}

pub fn state(resource_id: &str) -> i32 {
    vanilla_block_type_for_resource_id(resource_id)
        .and_then(|block| vanilla_block_default_state_id(block.id))
        .unwrap_or_else(|| panic!("Vanilla 26.1 has no default state for {resource_id}"))
}

pub fn clamped_map(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64 {
    let factor = ((value - from_min) / (from_max - from_min)).clamp(0.0, 1.0);
    to_min + factor * (to_max - to_min)
}
