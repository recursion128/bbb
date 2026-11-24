use crate::random::large_feature_random;
use std::collections::BTreeSet;

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::pieces::start_house_tower;
use super::EndCityStructureStart;
use crate::noise::structure_pieces::JigsawRotation;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};
use crate::noise::woodland_mansion::start::lowest_y_in_5by5_box_offset_7_blocks;

const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

impl NoiseStructurePlan {
    /// `EndCityStructure.findGenerationPoint` (`EndCityStructure.java:22-35`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_end_city_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing: &[EndCityStructureStart],
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Vec<EndCityStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::EndCity
                || set
                    .structures
                    .iter()
                    .any(|selection| existing_ids.contains(&selection.structure))
            {
                continue;
            }
            let Some(attempts) =
                self.random_spread_attempt_order(&set.id, level_seed, chunk_x, chunk_z)
            else {
                continue;
            };
            for structure_id in attempts {
                let Some(start) = self.end_city_generation_start(
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    &mut first_occupied_height,
                    &mut biome_at,
                    &mut template_size,
                    true,
                ) else {
                    continue;
                };
                existing_ids.insert(structure_id);
                generated.push(start);
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-294`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_end_city_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Option<EndCityStructureStart> {
        self.end_city_generation_start(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            &mut first_occupied_height,
            &mut biome_at,
            &mut template_size,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn end_city_generation_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
        require_allowed_biome: bool,
    ) -> Option<EndCityStructureStart> {
        let definition = self.structures().get(structure_id)?;
        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        let rotation = ROTATIONS[random.next_int(4) as usize];
        let anchor = lowest_y_in_5by5_box_offset_7_blocks(
            chunk_x,
            chunk_z,
            rotation,
            first_occupied_height,
        )?;
        if anchor.y < 60 {
            return None;
        }
        let biome = normalize_resource_location(biome_at(anchor)?).ok()?;
        if require_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }
        let pieces = start_house_tower(anchor, rotation, &mut random, template_size)?;
        Some(
            EndCityStructureStart::from_parts(structure_id, [chunk_x, chunk_z], 0, pieces)
                .with_terrain_adjustment(definition.terrain_adjustment),
        )
    }
}
