//! Ocean-ruin `StructureStart` materialization and biome-gated selection.

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use crate::random::WorldgenRandom;
use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::{
    transform_about_origin, OceanRuinBiomeTemp, OceanRuinFloat, OceanRuinGenerationStub,
    OceanRuinPiece, OceanRuinStructureSettings, OceanRuinStructureStart, CLUSTER_CHILD_EXTENT,
    CLUSTER_PARENT_EXTENT, COLD_LARGE_BRICK_TEMPLATES, COLD_LARGE_CRACKED_TEMPLATES,
    COLD_LARGE_MOSSY_TEMPLATES, COLD_SMALL_BRICK_TEMPLATES, COLD_SMALL_CRACKED_TEMPLATES,
    COLD_SMALL_MOSSY_TEMPLATES, INITIAL_TEMPLATE_Y, WARM_LARGE_TEMPLATES, WARM_SMALL_TEMPLATES,
};
use crate::noise::structure_pieces::JigsawRotation;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};

const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

/// `OceanRuinPieces.java:161`.
const LARGE_INTEGRITY: f32 = 0.9;
/// `OceanRuinPieces.java:161,193`.
const SMALL_INTEGRITY: f32 = 0.8;
/// `OceanRuinPieces.java:239`.
const COLD_CRACKED_INTEGRITY: f32 = 0.7;
/// `OceanRuinPieces.java:242`.
const COLD_MOSSY_INTEGRITY: f32 = 0.5;

impl NoiseStructurePlan {
    /// Runs the ocean-ruin random-spread half of `createStructures` and
    /// biome-gates the completed generation stub (`ChunkGenerator.java:476-589`,
    /// `Structure.java:131-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_ocean_ruin_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanRuinStructureStart],
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Vec<OceanRuinStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::OceanRuin
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
                let Some(stub) = self.ocean_ruin_generation_stub(
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
                    OceanRuinStructureStart::from_generation_stub(stub, 0)
                        .with_terrain_adjustment(terrain_adjustment),
                );
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-296`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_ocean_ruin_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Option<OceanRuinStructureStart> {
        let terrain_adjustment = self.structures().get(structure_id).map_or(
            crate::noise::structures::StructureTerrainAdjustment::None,
            |def| def.terrain_adjustment,
        );
        self.ocean_ruin_generation_stub(
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
            OceanRuinStructureStart::from_generation_stub(stub, 0)
                .with_terrain_adjustment(terrain_adjustment)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn ocean_ruin_generation_stub(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
        check_allowed_biome: bool,
    ) -> Option<OceanRuinGenerationStub> {
        let definition = self.structures().get(structure_id)?;
        let settings = definition.ocean_ruin?;
        let min_x = chunk_x * 16;
        let min_z = chunk_z * 16;
        // `OceanRuinStructure.findGenerationPoint` (`OceanRuinStructure.java:40-42`).
        let generation_anchor = BlockPos {
            x: min_x + 8,
            y: first_occupied_height(min_x + 8, min_z + 8, StructureHeightmap::OceanFloorWg)?,
            z: min_z + 8,
        };
        let biome = normalize_resource_location(biome_at(generation_anchor)?).ok()?;
        if check_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }

        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        let rotation = ROTATIONS[random.next_int(ROTATIONS.len() as i32) as usize];
        let origin = BlockPos {
            x: min_x,
            y: INITIAL_TEMPLATE_Y,
            z: min_z,
        };
        let pieces = add_pieces(origin, rotation, &mut random, settings, template_size)?;
        Some(OceanRuinGenerationStub::new(
            [chunk_x, chunk_z],
            generation_anchor,
            structure_id,
            pieces,
        ))
    }
}

/// `OceanRuinPieces.addPieces` (`OceanRuinPieces.java:152-166`).
fn add_pieces(
    origin: BlockPos,
    rotation: JigsawRotation,
    random: &mut WorldgenRandom,
    settings: OceanRuinStructureSettings,
    template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
) -> Option<Vec<OceanRuinPiece>> {
    let is_large = random.next_float() <= settings.large_probability.get();
    let base_integrity = if is_large {
        LARGE_INTEGRITY
    } else {
        SMALL_INTEGRITY
    };
    let mut pieces = add_piece(
        origin,
        rotation,
        random,
        settings,
        is_large,
        base_integrity,
        template_size,
    )?;
    if is_large && random.next_float() <= settings.cluster_probability.get() {
        pieces.extend(add_cluster_ruins(
            origin,
            rotation,
            random,
            settings,
            template_size,
        )?);
    }
    Some(pieces)
}

/// `OceanRuinPieces.addClusterRuins` (`OceanRuinPieces.java:168-197`).
fn add_cluster_ruins(
    origin: BlockPos,
    rotation: JigsawRotation,
    random: &mut WorldgenRandom,
    settings: OceanRuinStructureSettings,
    template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
) -> Option<Vec<OceanRuinPiece>> {
    let parent_pos = BlockPos {
        x: origin.x,
        y: INITIAL_TEMPLATE_Y,
        z: origin.z,
    };
    let parent_corner = offset(
        parent_pos,
        transform_about_origin(CLUSTER_PARENT_EXTENT, rotation),
    );
    let parent_bb = bounds_from_corners(parent_pos, parent_corner);
    let parent_bottom_left = BlockPos {
        x: parent_pos.x.min(parent_corner.x),
        y: parent_pos.y,
        z: parent_pos.z.min(parent_corner.z),
    };
    let mut positions = all_positions(random, parent_bottom_left);
    let ruins = next_int_inclusive(random, 4, 8);
    let mut pieces = Vec::new();
    for _ in 0..ruins {
        if positions.is_empty() {
            break;
        }
        let idx = random.next_int(positions.len() as i32) as usize;
        let pos = positions.remove(idx);
        let next_rotation = ROTATIONS[random.next_int(ROTATIONS.len() as i32) as usize];
        let next_corner = offset(
            pos,
            transform_about_origin(CLUSTER_CHILD_EXTENT, next_rotation),
        );
        let next_bb = bounds_from_corners(pos, next_corner);
        if !next_bb.intersects(&parent_bb) {
            pieces.extend(add_piece(
                pos,
                next_rotation,
                random,
                settings,
                false,
                SMALL_INTEGRITY,
                template_size,
            )?);
        }
    }
    Some(pieces)
}

/// `OceanRuinPieces.addPiece` (`OceanRuinPieces.java:212-245`).
fn add_piece(
    position: BlockPos,
    rotation: JigsawRotation,
    random: &mut WorldgenRandom,
    settings: OceanRuinStructureSettings,
    is_large: bool,
    base_integrity: f32,
    template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
) -> Option<Vec<OceanRuinPiece>> {
    let mut pieces = match settings.biome_temp {
        OceanRuinBiomeTemp::Warm => {
            let templates = if is_large {
                WARM_LARGE_TEMPLATES
            } else {
                WARM_SMALL_TEMPLATES
            };
            let template = choose(templates, random);
            vec![OceanRuinPiece::new(
                template,
                position,
                rotation,
                OceanRuinBiomeTemp::Warm,
                OceanRuinFloat::from_f32(base_integrity),
                is_large,
            )]
        }
        OceanRuinBiomeTemp::Cold => {
            let (bricks, cracked, mossy) = if is_large {
                (
                    COLD_LARGE_BRICK_TEMPLATES,
                    COLD_LARGE_CRACKED_TEMPLATES,
                    COLD_LARGE_MOSSY_TEMPLATES,
                )
            } else {
                (
                    COLD_SMALL_BRICK_TEMPLATES,
                    COLD_SMALL_CRACKED_TEMPLATES,
                    COLD_SMALL_MOSSY_TEMPLATES,
                )
            };
            let idx = random.next_int(bricks.len() as i32) as usize;
            vec![
                OceanRuinPiece::new(
                    bricks[idx],
                    position,
                    rotation,
                    OceanRuinBiomeTemp::Cold,
                    OceanRuinFloat::from_f32(base_integrity),
                    is_large,
                ),
                OceanRuinPiece::new(
                    cracked[idx],
                    position,
                    rotation,
                    OceanRuinBiomeTemp::Cold,
                    OceanRuinFloat::from_f32(COLD_CRACKED_INTEGRITY),
                    is_large,
                ),
                OceanRuinPiece::new(
                    mossy[idx],
                    position,
                    rotation,
                    OceanRuinBiomeTemp::Cold,
                    OceanRuinFloat::from_f32(COLD_MOSSY_INTEGRITY),
                    is_large,
                ),
            ]
        }
    };
    for piece in &mut pieces {
        let size = template_size(piece.template())?;
        piece.set_template_size(size);
    }
    Some(pieces)
}

/// `OceanRuinPieces.allPositions` (`OceanRuinPieces.java:199-210`).
fn all_positions(random: &mut WorldgenRandom, origin: BlockPos) -> Vec<BlockPos> {
    vec![
        offset(
            origin,
            BlockPos {
                x: -16 + next_int_inclusive(random, 1, 8),
                y: 0,
                z: 16 + next_int_inclusive(random, 1, 7),
            },
        ),
        offset(
            origin,
            BlockPos {
                x: -16 + next_int_inclusive(random, 1, 8),
                y: 0,
                z: next_int_inclusive(random, 1, 7),
            },
        ),
        offset(
            origin,
            BlockPos {
                x: -16 + next_int_inclusive(random, 1, 8),
                y: 0,
                z: -16 + next_int_inclusive(random, 4, 8),
            },
        ),
        offset(
            origin,
            BlockPos {
                x: next_int_inclusive(random, 1, 7),
                y: 0,
                z: 16 + next_int_inclusive(random, 1, 7),
            },
        ),
        offset(
            origin,
            BlockPos {
                x: next_int_inclusive(random, 1, 7),
                y: 0,
                z: -16 + next_int_inclusive(random, 4, 6),
            },
        ),
        offset(
            origin,
            BlockPos {
                x: 16 + next_int_inclusive(random, 1, 7),
                y: 0,
                z: 16 + next_int_inclusive(random, 3, 8),
            },
        ),
        offset(
            origin,
            BlockPos {
                x: 16 + next_int_inclusive(random, 1, 7),
                y: 0,
                z: next_int_inclusive(random, 1, 7),
            },
        ),
        offset(
            origin,
            BlockPos {
                x: 16 + next_int_inclusive(random, 1, 7),
                y: 0,
                z: -16 + next_int_inclusive(random, 4, 8),
            },
        ),
    ]
}

fn choose<'a>(templates: &[&'a str], random: &mut WorldgenRandom) -> &'a str {
    templates[random.next_int(templates.len() as i32) as usize]
}

/// `Mth.nextInt` inclusive (`Mth.java:145-147`).
fn next_int_inclusive(random: &mut WorldgenRandom, min: i32, max: i32) -> i32 {
    if min >= max {
        min
    } else {
        random.next_int(max - min + 1) + min
    }
}

fn offset(origin: BlockPos, delta: BlockPos) -> BlockPos {
    BlockPos {
        x: origin.x + delta.x,
        y: origin.y + delta.y,
        z: origin.z + delta.z,
    }
}

fn bounds_from_corners(a: BlockPos, b: BlockPos) -> RuinBox {
    RuinBox {
        min_x: a.x.min(b.x),
        min_y: a.y.min(b.y),
        min_z: a.z.min(b.z),
        max_x: a.x.max(b.x),
        max_y: a.y.max(b.y),
        max_z: a.z.max(b.z),
    }
}

struct RuinBox {
    min_x: i32,
    min_y: i32,
    min_z: i32,
    max_x: i32,
    max_y: i32,
    max_z: i32,
}

impl RuinBox {
    fn intersects(&self, other: &Self) -> bool {
        self.max_x >= other.min_x
            && self.min_x <= other.max_x
            && self.max_z >= other.min_z
            && self.min_z <= other.max_z
            && self.max_y >= other.min_y
            && self.min_y <= other.max_y
    }
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

    fn template_size(startup: &DatapackStartup, template_id: &str) -> Option<[i32; 3]> {
        startup
            .structures
            .template(template_id)
            .map(|element| element.template.size)
    }

    #[test]
    fn candidate_chunk_generates_warm_ocean_ruin_stub() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(crate::noise::ocean_ruin::OCEAN_RUIN_SET_ID, 0);
        let mut height_queries = Vec::new();
        let starts = plan.generate_missing_random_spread_ocean_ruin_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            |x, z, heightmap| {
                height_queries.push((x, z, heightmap));
                Some(40)
            },
            |_| Some("minecraft:lukewarm_ocean".to_string()),
            |template_id| template_size(&startup, template_id),
        );

        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), "minecraft:ocean_ruin_warm");
        assert_eq!(start.start_chunk(), chunk);
        assert_eq!(start.references(), 0);
        assert!(!start.pieces().is_empty());
        assert!(start
            .pieces()
            .iter()
            .all(|piece| piece.biome_temp() == OceanRuinBiomeTemp::Warm));
        assert!(start
            .pieces()
            .iter()
            .all(|piece| piece.template_position().y == INITIAL_TEMPLATE_Y));
        assert_eq!(
            height_queries,
            [(
                chunk[0] * 16 + 8,
                chunk[1] * 16 + 8,
                StructureHeightmap::OceanFloorWg
            )]
        );
        let piece = &start.pieces()[0];
        assert!(piece
            .template()
            .starts_with("minecraft:underwater_ruin/warm_"));
        assert_ne!(piece.template_size(), [0, 0, 0]);
    }
}
