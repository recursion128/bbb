//! Shipwreck `StructureStart` materialization and biome-gated selection.

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::{
    ShipwreckGenerationStub, ShipwreckPiece, ShipwreckStructureStart, INITIAL_TEMPLATE_Y,
    SHIPWRECK_BEACHED_TEMPLATES, SHIPWRECK_OCEAN_TEMPLATES, SHIPWRECK_PIVOT,
};
use crate::noise::structure_pieces::JigsawRotation;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};

const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

impl NoiseStructurePlan {
    /// Runs the shipwreck-owned random-spread half of `createStructures` and
    /// biome-gates the completed generation stub (`ChunkGenerator.java:476-589`,
    /// `Structure.java:131-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_shipwreck_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ShipwreckStructureStart],
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Vec<ShipwreckStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::Shipwreck
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
                let Some(stub) = self.shipwreck_generation_stub(
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
                let terrain_adjustment = self.structures().get(&structure_id).map_or(
                    crate::noise::structures::StructureTerrainAdjustment::None,
                    |def| def.terrain_adjustment,
                );
                existing_ids.insert(structure_id);
                generated.push(
                    ShipwreckStructureStart::from_generation_stub(stub, 0)
                        .with_terrain_adjustment(terrain_adjustment),
                );
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-296`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_shipwreck_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Option<ShipwreckStructureStart> {
        let terrain_adjustment = self.structures().get(structure_id).map_or(
            crate::noise::structures::StructureTerrainAdjustment::None,
            |def| def.terrain_adjustment,
        );
        self.shipwreck_generation_stub(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            &mut first_occupied_height,
            &mut biome_at,
            &mut template_size,
            false,
        )
        .map(|stub| {
            ShipwreckStructureStart::from_generation_stub(stub, 0)
                .with_terrain_adjustment(terrain_adjustment)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn shipwreck_generation_stub(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
        check_allowed_biome: bool,
    ) -> Option<ShipwreckGenerationStub> {
        let definition = self.structures().get(structure_id)?;
        let settings = definition.shipwreck?;
        let min_x = chunk_x * 16;
        let min_z = chunk_z * 16;
        // `ShipwreckStructure.findGenerationPoint` (`ShipwreckStructure.java:27-29`).
        let heightmap = if settings.is_beached {
            StructureHeightmap::WorldSurfaceWg
        } else {
            StructureHeightmap::OceanFloorWg
        };
        let generation_anchor = BlockPos {
            x: min_x + 8,
            y: first_occupied_height(min_x + 8, min_z + 8, heightmap)?,
            z: min_z + 8,
        };
        // Generation-stub biome validation samples its anchor (`Structure.java:131-140`).
        let biome = normalize_resource_location(biome_at(generation_anchor)?).ok()?;
        if check_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }

        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        let rotation = ROTATIONS[random.next_int(ROTATIONS.len() as i32) as usize];
        let templates = if settings.is_beached {
            SHIPWRECK_BEACHED_TEMPLATES
        } else {
            SHIPWRECK_OCEAN_TEMPLATES
        };
        let template = templates[random.next_int(templates.len() as i32) as usize];
        let size = template_size(template)?;
        let mut piece = ShipwreckPiece::new(
            template,
            BlockPos {
                x: min_x,
                y: INITIAL_TEMPLATE_Y,
                z: min_z,
            },
            rotation,
            settings.is_beached,
        )
        .with_template_size(size);
        if ShipwreckPiece::is_too_big_to_fit_in_world_gen_region(size) {
            let bounds = template_bounding_box(piece.template_position(), rotation, size);
            let height = if settings.is_beached {
                // Call site passes `(minX, xSpan, minZ, zSpan)` into
                // `getLowestY(minX, minZ, sizeX, sizeZ)` (`ShipwreckStructure.java:42`,
                // `Structure.java:178-180`).
                let min_y = get_lowest_y(
                    first_occupied_height,
                    bounds.min_x,
                    bounds.x_span(),
                    bounds.min_z,
                    bounds.z_span(),
                )?;
                let extra = random.next_int(3);
                ShipwreckPiece::calculate_beached_position(min_y, size[1], extra)
            } else {
                get_mean_first_occupied_height(
                    first_occupied_height,
                    bounds.min_x,
                    bounds.x_span(),
                    bounds.min_z,
                    bounds.z_span(),
                )?
            };
            piece.adjust_position_height(height);
        }

        Some(ShipwreckGenerationStub::new(
            [chunk_x, chunk_z],
            generation_anchor,
            structure_id,
            piece,
        ))
    }
}

struct TemplateBounds {
    min_x: i32,
    min_z: i32,
    max_x: i32,
    max_z: i32,
}

impl TemplateBounds {
    fn x_span(&self) -> i32 {
        self.max_x.wrapping_sub(self.min_x).wrapping_add(1)
    }

    fn z_span(&self) -> i32 {
        self.max_z.wrapping_sub(self.min_z).wrapping_add(1)
    }
}

/// `StructureTemplate.getBoundingBox` with `Mirror.NONE` and pivot `(4, 0, 15)`
/// (`StructureTemplate.java:615-629`, `ShipwreckPieces.java:32-33`).
fn template_bounding_box(
    position: BlockPos,
    rotation: JigsawRotation,
    size: [i32; 3],
) -> TemplateBounds {
    let corner1 = transform(BlockPos { x: 0, y: 0, z: 0 }, rotation);
    let corner2 = transform(
        BlockPos {
            x: size[0].wrapping_sub(1),
            y: size[1].wrapping_sub(1),
            z: size[2].wrapping_sub(1),
        },
        rotation,
    );
    let min_x = corner1.x.min(corner2.x).wrapping_add(position.x);
    let min_z = corner1.z.min(corner2.z).wrapping_add(position.z);
    let max_x = corner1.x.max(corner2.x).wrapping_add(position.x);
    let max_z = corner1.z.max(corner2.z).wrapping_add(position.z);
    TemplateBounds {
        min_x,
        min_z,
        max_x,
        max_z,
    }
}

/// `StructureTemplate.transform` with `Mirror.NONE` (`StructureTemplate.java:528-556`).
fn transform(pos: BlockPos, rotation: JigsawRotation) -> BlockPos {
    let BlockPos { x, y, z } = pos;
    let (pivot_x, pivot_z) = (SHIPWRECK_PIVOT.x, SHIPWRECK_PIVOT.z);
    match rotation {
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: pivot_x.wrapping_sub(pivot_z).wrapping_add(z),
            y,
            z: pivot_x.wrapping_add(pivot_z).wrapping_sub(x),
        },
        JigsawRotation::Clockwise90 => BlockPos {
            x: pivot_x.wrapping_add(pivot_z).wrapping_sub(z),
            y,
            z: pivot_z.wrapping_sub(pivot_x).wrapping_add(x),
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: pivot_x.wrapping_add(pivot_x).wrapping_sub(x),
            y,
            z: pivot_z.wrapping_add(pivot_z).wrapping_sub(z),
        },
        JigsawRotation::None => pos,
    }
}

/// Corner heights always use `WORLD_SURFACE_WG` (`Structure.java:154-164`).
fn corner_heights(
    first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
    min_x: i32,
    size_x: i32,
    min_z: i32,
    size_z: i32,
) -> Option<[i32; 4]> {
    let max_x = min_x.wrapping_add(size_x);
    let max_z = min_z.wrapping_add(size_z);
    let mut sample =
        |x: i32, z: i32| first_occupied_height(x, z, StructureHeightmap::WorldSurfaceWg);
    Some([
        sample(min_x, min_z)?,
        sample(min_x, max_z)?,
        sample(max_x, min_z)?,
        sample(max_x, max_z)?,
    ])
}

fn get_mean_first_occupied_height(
    first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
    min_x: i32,
    size_x: i32,
    min_z: i32,
    size_z: i32,
) -> Option<i32> {
    let corners = corner_heights(first_occupied_height, min_x, size_x, min_z, size_z)?;
    Some((corners[0] + corners[1] + corners[2] + corners[3]) / 4)
}

fn get_lowest_y(
    first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
    min_x: i32,
    min_z: i32,
    size_x: i32,
    size_z: i32,
) -> Option<i32> {
    let corners = corner_heights(first_occupied_height, min_x, size_x, min_z, size_z)?;
    Some(corners[0].min(corners[1]).min(corners[2]).min(corners[3]))
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
        let source = NoiseBiomeSource::from_registry(
            &startup.registries,
            "minecraft:overworld",
            "minecraft:overworld",
            0,
        )
        .unwrap();
        NoiseStructurePlan::from_startup(startup, source.overworld_biome_order().unwrap()).unwrap()
    }

    fn small_template_size(_: &str) -> Option<[i32; 3]> {
        Some([16, 10, 16])
    }

    /// A candidate chunk whose weighted order tries the ocean shipwreck first.
    fn candidate(plan: &NoiseStructurePlan) -> [i32; 2] {
        (-64..64)
            .flat_map(|x| (-64..64).map(move |z| [x, z]))
            .find(|&[x, z]| {
                plan.random_spread_attempt_order(crate::noise::shipwreck::SHIPWRECK_SET_ID, 0, x, z)
                    .is_some_and(|order| {
                        order.first().map(String::as_str) == Some("minecraft:shipwreck")
                    })
            })
            .expect("an ocean-first shipwreck candidate near the origin")
    }

    fn middle(chunk: [i32; 2]) -> (i32, i32) {
        (chunk[0] * 16 + 8, chunk[1] * 16 + 8)
    }

    #[test]
    fn ocean_candidate_generates_ocean_shipwreck_stub() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = candidate(&plan);
        let (mid_x, mid_z) = middle(chunk);
        let mut height_queries = Vec::new();
        let starts = plan.generate_missing_random_spread_shipwreck_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            |x, z, heightmap| {
                height_queries.push((x, z, heightmap));
                Some(63)
            },
            |_| Some("minecraft:ocean".to_string()),
            small_template_size,
        );

        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), "minecraft:shipwreck");
        assert_eq!(start.start_chunk(), chunk);
        assert_eq!(start.references(), 0);
        assert!(!start.piece().is_beached());
        assert!(!start.piece().height_adjusted());
        assert!(SHIPWRECK_OCEAN_TEMPLATES.contains(&start.piece().template()));
        assert_eq!(
            height_queries,
            [(mid_x, mid_z, StructureHeightmap::OceanFloorWg)]
        );
    }

    #[test]
    fn biome_miss_skips_attempt_and_tries_weighted_next() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = candidate(&plan);
        let starts = plan.generate_missing_random_spread_shipwreck_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            |_, _, _| Some(64),
            |_| Some("minecraft:beach".to_string()),
            small_template_size,
        );

        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), "minecraft:shipwreck_beached");
        assert!(start.piece().is_beached());
        assert!(SHIPWRECK_BEACHED_TEMPLATES.contains(&start.piece().template()));
    }

    #[test]
    fn too_big_ocean_piece_uses_world_surface_mean_height() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = candidate(&plan);
        let (mid_x, mid_z) = middle(chunk);
        let mut height_queries = Vec::new();
        let starts = plan.generate_missing_random_spread_shipwreck_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            |x, z, heightmap| {
                height_queries.push((x, z, heightmap));
                Some(40)
            },
            |_| Some("minecraft:ocean".to_string()),
            |_| Some([33, 10, 16]),
        );

        let start = &starts[0];
        assert!(start.piece().height_adjusted());
        assert_eq!(start.piece().template_position().y, 40);
        assert_eq!(
            height_queries[0],
            (mid_x, mid_z, StructureHeightmap::OceanFloorWg)
        );
        assert!(height_queries[1..]
            .iter()
            .all(|(_, _, heightmap)| *heightmap == StructureHeightmap::WorldSurfaceWg));
        assert_eq!(height_queries.len(), 5);
    }

    #[test]
    fn too_big_beached_piece_uses_lowest_y_then_beached_offset() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = candidate(&plan);
        let (mid_x, mid_z) = middle(chunk);
        let mut height_queries = Vec::new();
        let starts = plan.generate_missing_random_spread_shipwreck_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            |x, z, heightmap| {
                height_queries.push((x, z, heightmap));
                Some(50)
            },
            |_| Some("minecraft:beach".to_string()),
            |_| Some([33, 10, 16]),
        );

        let start = &starts[0];
        assert!(start.piece().is_beached());
        assert!(start.piece().height_adjusted());
        let extra = {
            let mut random = large_feature_random(0, chunk[0], chunk[1]);
            let _ = random.next_int(4);
            let _ = random.next_int(SHIPWRECK_BEACHED_TEMPLATES.len() as i32);
            random.next_int(3)
        };
        assert_eq!(
            start.piece().template_position().y,
            ShipwreckPiece::calculate_beached_position(50, 10, extra)
        );
        assert_eq!(
            height_queries[0],
            (mid_x, mid_z, StructureHeightmap::OceanFloorWg)
        );
        assert_eq!(
            height_queries[1],
            (mid_x, mid_z, StructureHeightmap::WorldSurfaceWg)
        );
        assert!(height_queries[2..]
            .iter()
            .all(|(_, _, heightmap)| *heightmap == StructureHeightmap::WorldSurfaceWg));
    }
}
