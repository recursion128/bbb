//! Typed structure-start restore, BBB compact save, and reference assembly.

use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{normalize_resource_location, packets::BlockPos, NbtSummaryValue};

use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::structure_pool::{
    ProcessorListHolder, StructureLiquidSettings, StructurePoolElement, StructurePoolProjection,
};

use super::buried_treasure::{BuriedTreasureStructureStart, BURIED_TREASURE_ID};
use super::end_city::{EndCityStructureStart, END_CITY_TYPE_ID};
use super::igloo::{IglooStructureStart, IGLOO_TYPE_ID};
use super::mineshaft::MineshaftStructureStart;
use super::nether_fortress::{FortressStructureStart, FORTRESS_TYPE_ID};
use super::nether_fossil::{NetherFossilStructureStart, NETHER_FOSSIL_TYPE_ID};
use super::ocean_monument::{OceanMonumentStructureStart, OCEAN_MONUMENT_TYPE_ID};
use super::ocean_ruin::OceanRuinStructureStart;
use super::ruined_portal::{RuinedPortalStructureStart, RUINED_PORTAL_TYPE_ID};
use super::scattered::{ScatteredFeatureKind, ScatteredFeatureStructureStart};
use super::shipwreck::ShipwreckStructureStart;
use super::stronghold::{StrongholdStructureStart, STRONGHOLD_TYPE_ID};
use super::structure_nbt::{start_to_nbt, NbtFields, NbtRead};
use super::structure_pieces::{
    element_bounding_box, CachedStartBoundingBox, JigsawBoundingBox, JigsawGraphJunction,
    JigsawPiece, JigsawPieceGraph,
};
use super::structures::{NoiseStructurePlan, StructureTerrainAdjustment};
use super::woodland_mansion::{WoodlandMansionStructureStart, WOODLAND_MANSION_TYPE_ID};

const JIGSAW_PIECE_ID: &str = "minecraft:jigsaw";
const PLACED_FEATURE_REGISTRY: &str = "minecraft:worldgen/placed_feature";
const MAX_STRUCTURE_REFERENCE_DISTANCE: i64 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RestoredStructureStart {
    pub(crate) references: i32,
    pub(crate) terrain_adjustment: StructureTerrainAdjustment,
    pub(crate) pieces: Vec<JigsawBoundingBox>,
    pub(crate) cached_bounding_box: CachedStartBoundingBox,
}

impl RestoredStructureStart {
    pub(super) fn try_add_reference(&mut self) -> bool {
        if self.references >= 1 {
            return false;
        }
        self.references = self.references.wrapping_add(1);
        true
    }

    pub(super) fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.cached_bounding_box
            .get_or_init(|| {
                JigsawBoundingBox::adjusted_union(
                    self.pieces.iter().copied(),
                    self.terrain_adjustment,
                )
            })
            .is_some_and(|bounds| bounds.intersects_chunk(chunk_x, chunk_z))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JigsawStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    liquid_settings: StructureLiquidSettings,
    pieces: Vec<JigsawPiece>,
    junctions: Vec<JigsawGraphJunction>,
    cached_bounding_box: CachedStartBoundingBox,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoiseStructureChunkData {
    chunk: [i32; 2],
    starts: Vec<JigsawStructureStart>,
    buried_treasure_start: buried_treasure::BuriedTreasureStarts,
    mineshaft_starts: Vec<MineshaftStructureStart>,
    shipwreck_starts: Vec<ShipwreckStructureStart>,
    ocean_ruin_starts: Vec<OceanRuinStructureStart>,
    igloo_starts: Vec<IglooStructureStart>,
    scattered_starts: Vec<ScatteredFeatureStructureStart>,
    ruined_portal_starts: Vec<RuinedPortalStructureStart>,
    ocean_monument_starts: Vec<OceanMonumentStructureStart>,
    woodland_mansion_starts: Vec<WoodlandMansionStructureStart>,
    stronghold_starts: Vec<StrongholdStructureStart>,
    fortress_starts: Vec<FortressStructureStart>,
    nether_fossil_starts: Vec<NetherFossilStructureStart>,
    end_city_starts: Vec<EndCityStructureStart>,
    restored_bounds: BTreeMap<String, RestoredStructureStart>,
    explicit_starts: BTreeSet<String>,
    references: BTreeMap<String, Vec<i64>>,
}

pub type JigsawStructureChunkData = NoiseStructureChunkData;

impl JigsawStructureStart {
    /// `Structure.generate` materializes the ordered pieces into a start only
    /// after the selected structure produced a valid generation stub
    /// (`Structure.java:83-118`). Candidate selection and biome gating remain
    /// with the later STRUCTURE_STARTS task owner.
    pub fn from_graph(
        plan: &NoiseStructurePlan,
        graph: JigsawPieceGraph,
        references: i32,
    ) -> Option<Self> {
        if graph.pieces.is_empty() {
            return None;
        }
        // Junctions are owned and saved by their piece, so the runtime start
        // iterates pieces first while retaining insertion order within each
        // piece (`PoolElementStructurePiece.java:78-84`).
        let mut junctions = graph.junctions;
        junctions.sort_by_key(|junction| junction.piece_index);
        Some(Self {
            terrain_adjustment: plan
                .structures()
                .get(&graph.structure_id)?
                .terrain_adjustment,
            structure_id: graph.structure_id,
            start_chunk: graph.start_chunk,
            references,
            liquid_settings: graph.liquid_settings,
            pieces: graph.pieces,
            junctions,
            cached_bounding_box: CachedStartBoundingBox::new(),
        })
    }

    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub const fn start_chunk(&self) -> [i32; 2] {
        self.start_chunk
    }

    pub const fn references(&self) -> i32 {
        self.references
    }

    pub(super) fn try_add_reference(&mut self) -> bool {
        if self.references >= 1 {
            return false;
        }
        self.references = self.references.wrapping_add(1);
        true
    }

    pub const fn terrain_adjustment(&self) -> StructureTerrainAdjustment {
        self.terrain_adjustment
    }

    pub fn pieces(&self) -> &[JigsawPiece] {
        &self.pieces
    }

    pub fn junctions(&self) -> &[JigsawGraphJunction] {
        &self.junctions
    }

    /// `Structure.adjustBoundingBox` inflates every non-NONE terrain-adapting
    /// start by twelve blocks (`Structure.java:75-81`).
    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                JigsawBoundingBox::adjusted_union(
                    self.pieces.iter().map(|piece| piece.bounding_box),
                    self.terrain_adjustment,
                )
            })
            .expect("jigsaw start has at least one piece")
    }

    pub fn to_nbt(&self) -> NbtSummaryValue {
        let children = self
            .pieces
            .iter()
            .enumerate()
            .map(|(piece_index, piece)| {
                piece_to_nbt(
                    piece,
                    self.liquid_settings,
                    self.junctions
                        .iter()
                        .filter(|junction| junction.piece_index == piece_index),
                )
            })
            .collect();
        start_to_nbt(
            &self.structure_id,
            self.start_chunk,
            self.references,
            children,
        )
    }

    fn from_nbt(
        outer_id: &str,
        value: &NbtSummaryValue,
        plan: &NoiseStructurePlan,
        startup: &DatapackStartup,
    ) -> Option<Self> {
        let fields = value.compound()?;
        let mut pieces = Vec::new();
        let mut junctions = Vec::new();
        let mut liquid_settings = StructureLiquidSettings::ApplyWaterlogging;
        for (piece_index, child) in fields.field("Children")?.list()?.iter().enumerate() {
            let (mut piece, child_liquids, child_junctions) =
                piece_from_nbt(child, piece_index, startup)?;
            // The save carries the generation-time collision box, including
            // expansion-hack height. The loading constructor deliberately
            // replaces it with `element.getBoundingBox` after `super` decoded
            // the stored box (`PoolElementStructurePiece.java:54-66`).
            piece.bounding_box = element_bounding_box(
                &piece.element,
                &startup.structures,
                piece.position,
                piece.rotation,
            )?;
            if piece_index == 0 {
                liquid_settings = child_liquids;
            }
            pieces.push(piece);
            junctions.extend(child_junctions);
        }
        if pieces.is_empty() {
            return None;
        }
        Some(Self {
            structure_id: outer_id.to_string(),
            start_chunk: [
                fields.field("ChunkX")?.int()?,
                fields.field("ChunkZ")?.int()?,
            ],
            references: fields.field("references")?.int()?,
            terrain_adjustment: plan.structures().get(outer_id)?.terrain_adjustment,
            liquid_settings,
            pieces,
            junctions,
            cached_bounding_box: CachedStartBoundingBox::new(),
        })
    }
}

impl NoiseStructurePlan {
    /// Runs the random-spread half of `createStructures` for structure sets
    /// whose every member is owned by the jigsaw codec. Weighted retries reset
    /// generation to the large-feature seed, then biome-gate the completed
    /// generation stub (`ChunkGenerator.java:476-589`, `Structure.java:131-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_jigsaw_starts(
        &self,
        startup: &DatapackStartup,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        world_min_y: i32,
        world_height: i32,
        existing_starts: &[JigsawStructureStart],
        height_access: &dyn super::structure_pieces::JigsawHeightAccess,
        biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Vec<JigsawStructureStart> {
        self.generate_missing_jigsaw_starts_with(
            startup,
            level_seed,
            chunk_x,
            chunk_z,
            world_min_y,
            world_height,
            existing_starts,
            height_access,
            biome_at,
            |set| self.random_spread_attempt_order(&set.id, level_seed, chunk_x, chunk_z),
        )
    }

    /// Generates missing jigsaw starts across both `RandomSpread` and
    /// `ConcentricRings` structure sets (`ChunkGenerator.java:476-589`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_jigsaw_starts_with_rings(
        &self,
        startup: &DatapackStartup,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        rings: &BTreeMap<String, BTreeSet<[i32; 2]>>,
        world_min_y: i32,
        world_height: i32,
        existing_starts: &[JigsawStructureStart],
        height_access: &dyn super::structure_pieces::JigsawHeightAccess,
        biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Vec<JigsawStructureStart> {
        self.generate_missing_jigsaw_starts_with(
            startup,
            level_seed,
            chunk_x,
            chunk_z,
            world_min_y,
            world_height,
            existing_starts,
            height_access,
            biome_at,
            |set| {
                if self.is_structure_chunk(&set.id, level_seed, chunk_x, chunk_z, rings)
                    == Some(true)
                {
                    self.weighted_attempt_order(&set.id, level_seed, chunk_x, chunk_z)
                } else {
                    None
                }
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_missing_jigsaw_starts_with(
        &self,
        startup: &DatapackStartup,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        world_min_y: i32,
        world_height: i32,
        existing_starts: &[JigsawStructureStart],
        height_access: &dyn super::structure_pieces::JigsawHeightAccess,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut attempts_for_set: impl FnMut(&super::structures::NoiseStructureSet) -> Option<Vec<String>>,
    ) -> Vec<JigsawStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id.clone())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            let jigsaw_owned = !set.structures.is_empty()
                && set.structures.iter().all(|selection| {
                    self.structures()
                        .get(&selection.structure)
                        .is_some_and(|definition| definition.jigsaw.is_some())
                });
            if !jigsaw_owned
                || set
                    .structures
                    .iter()
                    .any(|selection| existing_ids.contains(&selection.structure))
            {
                continue;
            }
            let Some(attempts) = attempts_for_set(set) else {
                continue;
            };
            for structure_id in attempts {
                let Some(planned) = self.plan_jigsaw_anchor(
                    startup,
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    world_min_y,
                    world_height,
                    height_access,
                ) else {
                    continue;
                };
                let Some(biome) = biome_at(planned.generation_anchor())
                    .and_then(|biome| normalize_resource_location(biome).ok())
                else {
                    continue;
                };
                if !self
                    .structures()
                    .get(&structure_id)
                    .is_some_and(|definition| definition.biomes.contains(&biome))
                {
                    continue;
                }
                let Some(graph) = planned.expand(startup, height_access) else {
                    continue;
                };
                let Some(start) = JigsawStructureStart::from_graph(self, graph, 0) else {
                    continue;
                };
                existing_ids.insert(structure_id);
                generated.push(start);
                break;
            }
        }
        generated
    }
}

impl NoiseStructureChunkData {
    /// Applies the distance and adjusted-box half of `createReferences` to a
    /// caller-supplied set of nearby starts (`ChunkGenerator.java:598-627`).
    /// `from_complete_reference_window` owns the complete source scan.
    pub fn from_supplied_starts<'a>(
        chunk_x: i32,
        chunk_z: i32,
        owned_starts: impl IntoIterator<Item = JigsawStructureStart>,
        nearby_starts: impl IntoIterator<Item = &'a JigsawStructureStart>,
    ) -> Self {
        let mut starts = Vec::new();
        insert_starts(
            &mut starts,
            owned_starts,
            JigsawStructureStart::structure_id,
        );
        let target = chunk_block_bounds(chunk_x, chunk_z);
        let mut sources = nearby_starts.into_iter().collect::<Vec<_>>();
        sources.sort_by_key(|start| {
            (
                start.start_chunk[0],
                start.start_chunk[1],
                start.structure_id.clone(),
            )
        });
        let mut references = BTreeMap::<String, Vec<i64>>::new();
        sources.dedup_by(|left, right| {
            left.start_chunk == right.start_chunk && left.structure_id == right.structure_id
        });
        for start in sources {
            let distance = (i64::from(start.start_chunk[0]) - i64::from(chunk_x))
                .abs()
                .max((i64::from(start.start_chunk[1]) - i64::from(chunk_z)).abs());
            if distance > MAX_STRUCTURE_REFERENCE_DISTANCE
                || !start.bounding_box().intersects(target)
            {
                continue;
            }
            references
                .entry(start.structure_id.clone())
                .or_default()
                .push(pack_chunk(start.start_chunk[0], start.start_chunk[1]));
        }
        Self {
            chunk: [chunk_x, chunk_z],
            starts,
            references,
            ..Self::default()
        }
    }

    /// `ChunkSerializer.unpackStructureStart` and `unpackStructureReferences`:
    /// unknown or malformed starts and references are skipped
    /// (`StructureStart.java:39-67`).
    pub fn from_nbt(
        value: &NbtSummaryValue,
        chunk_x: i32,
        chunk_z: i32,
        plan: &NoiseStructurePlan,
        startup: &DatapackStartup,
    ) -> Self {
        let mut data = Self {
            chunk: [chunk_x, chunk_z],
            ..Self::default()
        };
        let fields = value.compound().unwrap_or_default();
        let template_size = |id: &str| {
            startup
                .structures
                .template(id)
                .map(|element| element.template.size)
        };
        for start in fields
            .field("starts")
            .and_then(NbtRead::compound)
            .unwrap_or_default()
        {
            let Some(id) = start
                .name
                .try_to_string()
                .ok()
                .and_then(|name| normalize_resource_location(name).ok())
            else {
                continue;
            };
            let Some(definition) = plan.structures().get(&id) else {
                continue;
            };
            let value = &start.value;
            let type_id = definition.type_id.as_str();
            let decoded = if type_id == BURIED_TREASURE_ID {
                BuriedTreasureStructureStart::from_nbt(&id, value)
                    .map(|decoded| {
                        data.buried_treasure_start
                            .push(decoded.with_terrain_adjustment(definition.terrain_adjustment))
                    })
                    .is_some()
            } else if definition.mineshaft.is_some() {
                MineshaftStructureStart::from_nbt(&id, value, plan)
                    .map(|decoded| data.mineshaft_starts.push(decoded))
                    .is_some()
            } else if definition.shipwreck.is_some() {
                let Some(decoded) = ShipwreckStructureStart::from_nbt(&id, value) else {
                    continue;
                };
                let mut decoded = decoded.with_terrain_adjustment(definition.terrain_adjustment);
                let Some(size) = template_size(decoded.piece().template()) else {
                    continue;
                };
                decoded.piece_mut().set_template_size(size);
                data.shipwreck_starts.push(decoded);
                true
            } else if definition.ocean_ruin.is_some() {
                let Some(decoded) = OceanRuinStructureStart::from_nbt(&id, value) else {
                    continue;
                };
                let mut decoded = decoded.with_terrain_adjustment(definition.terrain_adjustment);
                if decoded.pieces_mut().iter_mut().all(|piece| {
                    template_size(piece.template())
                        .map(|size| piece.set_template_size(size))
                        .is_some()
                }) {
                    data.ocean_ruin_starts.push(decoded);
                    true
                } else {
                    false
                }
            } else if type_id == IGLOO_TYPE_ID {
                let Some(decoded) = IglooStructureStart::from_nbt(&id, value) else {
                    continue;
                };
                let mut decoded = decoded.with_terrain_adjustment(definition.terrain_adjustment);
                if decoded.pieces_mut().iter_mut().all(|piece| {
                    template_size(piece.template())
                        .map(|size| piece.set_template_size(size))
                        .is_some()
                }) {
                    data.igloo_starts.push(decoded);
                    true
                } else {
                    false
                }
            } else if let Some(kind) = ScatteredFeatureKind::from_type_id(type_id) {
                ScatteredFeatureStructureStart::from_nbt(&id, value, kind)
                    .map(|decoded| {
                        data.scattered_starts
                            .push(decoded.with_terrain_adjustment(definition.terrain_adjustment))
                    })
                    .is_some()
            } else if type_id == RUINED_PORTAL_TYPE_ID {
                let Some(decoded) = RuinedPortalStructureStart::from_nbt(&id, value) else {
                    continue;
                };
                let mut decoded = decoded.with_terrain_adjustment(definition.terrain_adjustment);
                let Some(size) = template_size(decoded.piece().template()) else {
                    continue;
                };
                decoded.piece_mut().set_template_size(size);
                data.ruined_portal_starts.push(decoded);
                true
            } else if type_id == OCEAN_MONUMENT_TYPE_ID {
                OceanMonumentStructureStart::from_nbt(&id, value)
                    .map(|decoded| {
                        data.ocean_monument_starts
                            .push(decoded.with_terrain_adjustment(definition.terrain_adjustment))
                    })
                    .is_some()
            } else if type_id == WOODLAND_MANSION_TYPE_ID {
                let Some(decoded) = WoodlandMansionStructureStart::from_nbt(&id, value) else {
                    continue;
                };
                let mut decoded = decoded.with_terrain_adjustment(definition.terrain_adjustment);
                if decoded.pieces_mut().iter_mut().all(|piece| {
                    template_size(piece.template())
                        .map(|size| piece.set_template_size(size))
                        .is_some()
                }) {
                    data.woodland_mansion_starts.push(decoded);
                    true
                } else {
                    false
                }
            } else if type_id == STRONGHOLD_TYPE_ID {
                StrongholdStructureStart::from_nbt(&id, value)
                    .map(|decoded| {
                        data.stronghold_starts
                            .push(decoded.with_terrain_adjustment(definition.terrain_adjustment))
                    })
                    .is_some()
            } else if type_id == FORTRESS_TYPE_ID {
                FortressStructureStart::from_nbt(&id, value, plan)
                    .map(|decoded| data.fortress_starts.push(decoded))
                    .is_some()
            } else if type_id == NETHER_FOSSIL_TYPE_ID {
                let Some(mut decoded) = NetherFossilStructureStart::from_nbt(&id, value, plan)
                else {
                    continue;
                };
                let Some(size) = template_size(decoded.piece().template()) else {
                    continue;
                };
                decoded.piece_mut().set_template_size(size);
                data.nether_fossil_starts.push(decoded);
                true
            } else if type_id == END_CITY_TYPE_ID {
                Self::decode_end_city_start(&id, value, definition.terrain_adjustment, startup)
                    .map(|decoded| data.end_city_starts.push(decoded))
                    .is_some()
            } else if definition.jigsaw.is_some() {
                JigsawStructureStart::from_nbt(&id, value, plan, startup)
                    .map(|decoded| data.starts.push(decoded))
                    .is_some()
            } else {
                false
            };
            if decoded {
                data.explicit_starts.insert(id);
            }
        }
        for entry in fields
            .field("bbb_starts")
            .and_then(NbtRead::compound)
            .unwrap_or_default()
        {
            let Some(id) = entry
                .name
                .try_to_string()
                .ok()
                .and_then(|name| normalize_resource_location(name).ok())
            else {
                continue;
            };
            let Some(definition) = plan.structures().get(&id) else {
                continue;
            };
            if data.has_valid_start(&id) {
                continue;
            }
            let Some(start_fields) = entry.value.compound() else {
                continue;
            };
            let references = start_fields
                .field("references")
                .and_then(NbtRead::int)
                .unwrap_or(0);
            let coords = start_fields
                .field("bounds")
                .and_then(NbtRead::int_array)
                .unwrap_or_default();
            if coords.len() % 6 != 0 {
                continue;
            }
            let pieces = coords
                .as_chunks::<6>()
                .0
                .iter()
                .map(|b| JigsawBoundingBox {
                    min: BlockPos {
                        x: b[0],
                        y: b[1],
                        z: b[2],
                    },
                    max: BlockPos {
                        x: b[3],
                        y: b[4],
                        z: b[5],
                    },
                })
                .collect::<Vec<_>>();
            data.restored_bounds.insert(
                id,
                RestoredStructureStart {
                    references,
                    terrain_adjustment: definition.terrain_adjustment,
                    pieces,
                    cached_bounding_box: CachedStartBoundingBox::new(),
                },
            );
        }
        for reference in fields
            .field("References")
            .and_then(NbtRead::compound)
            .unwrap_or_default()
        {
            let Some(id) = reference
                .name
                .try_to_string()
                .ok()
                .and_then(|name| normalize_resource_location(name).ok())
            else {
                continue;
            };
            let NbtSummaryValue::LongArray(positions) = &reference.value else {
                continue;
            };
            if !plan.structures().contains_key(&id) {
                continue;
            }
            let positions = positions
                .iter()
                .copied()
                .filter(|&packed| {
                    let [source_x, source_z] = unpack_chunk(packed);
                    (i64::from(source_x) - i64::from(chunk_x))
                        .abs()
                        .max((i64::from(source_z) - i64::from(chunk_z)).abs())
                        <= MAX_STRUCTURE_REFERENCE_DISTANCE
                })
                .collect();
            data.references.insert(id, positions);
        }
        data.starts
            .sort_by(|left, right| left.structure_id.cmp(&right.structure_id));
        data.mineshaft_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.shipwreck_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.ocean_ruin_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.igloo_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.scattered_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.ruined_portal_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.ocean_monument_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.woodland_mansion_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.stronghold_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.fortress_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.nether_fossil_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.end_city_starts
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
        data.buried_treasure_start.sort();
        data
    }

    pub fn has_bbb_starts(&self) -> bool {
        !self.restored_bounds.is_empty()
    }

    pub fn is_explicit_start(&self, structure_id: &str) -> bool {
        self.explicit_starts.contains(structure_id)
    }

    pub fn mark_explicit_start(&mut self, structure_id: &str) -> bool {
        let has_typed =
            self.has_valid_start(structure_id) && !self.restored_bounds.contains_key(structure_id);
        if has_typed {
            self.explicit_starts.insert(structure_id.to_string());
        }
        has_typed
    }

    /// Marks every typed start currently stored on this chunk as explicit so
    /// `to_nbt` preserves its full Vanilla `starts` entry (pieces, templates,
    /// `junctions`, and `ground_level_delta`) rather than summarizing to
    /// `bbb_starts`.
    pub fn with_explicit_starts(mut self) -> Self {
        let typed_ids = self
            .owned_start_references()
            .into_keys()
            .filter(|id| !self.restored_bounds.contains_key(id))
            .collect::<Vec<_>>();
        self.explicit_starts.extend(typed_ids);
        self
    }

    /// Replaces compact BBB-saved start summaries with deterministically
    /// regenerated typed starts when available, keeping saved reference counts
    /// and leaving non-natural and explicit starts intact.
    pub fn hydrate_bbb_starts(&mut self, natural: Self) {
        if self.restored_bounds.is_empty() {
            return;
        }
        macro_rules! hydrate_vec {
            ($vec:ident, $src:expr) => {
                for mut start in $src {
                    if self.explicit_starts.contains(start.structure_id()) {
                        continue;
                    }
                    if let Some(saved) = self.restored_bounds.remove(start.structure_id()) {
                        if saved.references > 0 && start.references() == 0 {
                            let _ = start.try_add_reference();
                        }
                        self.$vec
                            .retain(|existing| existing.structure_id() != start.structure_id());
                        self.$vec.push(start);
                    }
                }
                self.$vec
                    .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
            };
        }
        hydrate_vec!(starts, natural.starts);
        for mut start in natural.buried_treasure_start.0 {
            if self.explicit_starts.contains(start.structure_id()) {
                continue;
            }
            if let Some(saved) = self.restored_bounds.remove(start.structure_id()) {
                if saved.references > 0 && start.references() == 0 {
                    let _ = start.try_add_reference();
                }
                self.buried_treasure_start
                    .retain(|existing| existing.structure_id() != start.structure_id());
                self.buried_treasure_start.push(start);
            }
        }
        self.buried_treasure_start.sort();
        hydrate_vec!(mineshaft_starts, natural.mineshaft_starts);
        hydrate_vec!(shipwreck_starts, natural.shipwreck_starts);
        hydrate_vec!(ocean_ruin_starts, natural.ocean_ruin_starts);
        hydrate_vec!(igloo_starts, natural.igloo_starts);
        hydrate_vec!(scattered_starts, natural.scattered_starts);
        hydrate_vec!(ruined_portal_starts, natural.ruined_portal_starts);
        hydrate_vec!(ocean_monument_starts, natural.ocean_monument_starts);
        hydrate_vec!(woodland_mansion_starts, natural.woodland_mansion_starts);
        hydrate_vec!(stronghold_starts, natural.stronghold_starts);
        hydrate_vec!(fortress_starts, natural.fortress_starts);
        hydrate_vec!(nether_fossil_starts, natural.nether_fossil_starts);
        hydrate_vec!(end_city_starts, natural.end_city_starts);
    }

    /// `ProtoChunk.setStartForStructure` (`ProtoChunk.java:203-213`): retains
    /// only starts whose piece Y-span lies within `[min_y, max_y_exclusive)`.
    pub fn retain_starts_within_y(&mut self, min_y: i32, max_y_exclusive: i32) {
        let drop_ids = self
            .owned_start_references()
            .into_keys()
            .filter(|id| {
                self.raw_start_geometry(id).is_some_and(|pieces| {
                    if pieces.is_empty() {
                        return false;
                    }
                    let start_min_y = pieces.iter().map(|b| b.min.y).min().unwrap();
                    let start_max_y = pieces.iter().map(|b| b.max.y).max().unwrap();
                    start_min_y < min_y || start_max_y >= max_y_exclusive
                })
            })
            .collect::<Vec<_>>();
        for id in drop_ids {
            self.remove_start(&id);
        }
    }

    pub(super) fn owned_start_references(&self) -> BTreeMap<String, i32> {
        let mut out = BTreeMap::new();
        for (id, restored) in &self.restored_bounds {
            out.insert(id.clone(), restored.references);
        }
        macro_rules! collect_refs {
            ($starts:expr) => {
                for start in $starts.iter() {
                    out.insert(start.structure_id().to_string(), start.references());
                }
            };
        }
        collect_refs!(self.starts);
        collect_refs!(self.buried_treasure_start);
        collect_refs!(self.mineshaft_starts);
        collect_refs!(self.shipwreck_starts);
        collect_refs!(self.ocean_ruin_starts);
        collect_refs!(self.igloo_starts);
        collect_refs!(self.scattered_starts);
        collect_refs!(self.ruined_portal_starts);
        collect_refs!(self.ocean_monument_starts);
        collect_refs!(self.woodland_mansion_starts);
        collect_refs!(self.stronghold_starts);
        collect_refs!(self.fortress_starts);
        collect_refs!(self.nether_fossil_starts);
        collect_refs!(self.end_city_starts);
        out
    }

    fn remove_start(&mut self, structure_id: &str) {
        self.restored_bounds.remove(structure_id);
        self.explicit_starts.remove(structure_id);
        self.starts.retain(|s| s.structure_id() != structure_id);
        self.buried_treasure_start
            .retain(|s| s.structure_id() != structure_id);
        self.mineshaft_starts
            .retain(|s| s.structure_id() != structure_id);
        self.shipwreck_starts
            .retain(|s| s.structure_id() != structure_id);
        self.ocean_ruin_starts
            .retain(|s| s.structure_id() != structure_id);
        self.igloo_starts
            .retain(|s| s.structure_id() != structure_id);
        self.scattered_starts
            .retain(|s| s.structure_id() != structure_id);
        self.ruined_portal_starts
            .retain(|s| s.structure_id() != structure_id);
        self.ocean_monument_starts
            .retain(|s| s.structure_id() != structure_id);
        self.woodland_mansion_starts
            .retain(|s| s.structure_id() != structure_id);
        self.stronghold_starts
            .retain(|s| s.structure_id() != structure_id);
        self.fortress_starts
            .retain(|s| s.structure_id() != structure_id);
        self.nether_fossil_starts
            .retain(|s| s.structure_id() != structure_id);
        self.end_city_starts
            .retain(|s| s.structure_id() != structure_id);
    }

    pub const fn chunk(&self) -> [i32; 2] {
        self.chunk
    }

    pub fn starts(&self) -> &[JigsawStructureStart] {
        &self.starts
    }

    pub fn mineshaft_starts(&self) -> &[MineshaftStructureStart] {
        &self.mineshaft_starts
    }

    pub fn shipwreck_starts(&self) -> &[ShipwreckStructureStart] {
        &self.shipwreck_starts
    }

    pub fn ocean_ruin_starts(&self) -> &[OceanRuinStructureStart] {
        &self.ocean_ruin_starts
    }

    pub fn igloo_starts(&self) -> &[IglooStructureStart] {
        &self.igloo_starts
    }

    pub fn scattered_starts(&self) -> &[ScatteredFeatureStructureStart] {
        &self.scattered_starts
    }

    pub fn ruined_portal_starts(&self) -> &[RuinedPortalStructureStart] {
        &self.ruined_portal_starts
    }

    pub fn ocean_monument_starts(&self) -> &[OceanMonumentStructureStart] {
        &self.ocean_monument_starts
    }

    pub fn woodland_mansion_starts(&self) -> &[WoodlandMansionStructureStart] {
        &self.woodland_mansion_starts
    }

    pub fn stronghold_starts(&self) -> &[StrongholdStructureStart] {
        &self.stronghold_starts
    }

    pub fn with_jigsaw_starts(
        mut self,
        starts: impl IntoIterator<Item = JigsawStructureStart>,
    ) -> Self {
        insert_starts(&mut self.starts, starts, JigsawStructureStart::structure_id);
        self
    }

    pub fn with_mineshaft_starts(
        mut self,
        starts: impl IntoIterator<Item = MineshaftStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.mineshaft_starts,
            starts,
            MineshaftStructureStart::structure_id,
        );
        self
    }

    pub fn with_shipwreck_starts(
        mut self,
        starts: impl IntoIterator<Item = ShipwreckStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.shipwreck_starts,
            starts,
            ShipwreckStructureStart::structure_id,
        );
        self
    }

    pub fn with_ocean_ruin_starts(
        mut self,
        starts: impl IntoIterator<Item = OceanRuinStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.ocean_ruin_starts,
            starts,
            OceanRuinStructureStart::structure_id,
        );
        self
    }

    pub fn with_igloo_starts(
        mut self,
        starts: impl IntoIterator<Item = IglooStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.igloo_starts,
            starts,
            IglooStructureStart::structure_id,
        );
        self
    }

    pub fn with_scattered_starts(
        mut self,
        starts: impl IntoIterator<Item = ScatteredFeatureStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.scattered_starts,
            starts,
            ScatteredFeatureStructureStart::structure_id,
        );
        self
    }

    pub fn with_ruined_portal_starts(
        mut self,
        starts: impl IntoIterator<Item = RuinedPortalStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.ruined_portal_starts,
            starts,
            RuinedPortalStructureStart::structure_id,
        );
        self
    }

    pub fn with_ocean_monument_starts(
        mut self,
        starts: impl IntoIterator<Item = OceanMonumentStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.ocean_monument_starts,
            starts,
            OceanMonumentStructureStart::structure_id,
        );
        self
    }

    pub fn with_woodland_mansion_starts(
        mut self,
        starts: impl IntoIterator<Item = WoodlandMansionStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.woodland_mansion_starts,
            starts,
            WoodlandMansionStructureStart::structure_id,
        );
        self
    }

    pub fn with_stronghold_starts(
        mut self,
        starts: impl IntoIterator<Item = StrongholdStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.stronghold_starts,
            starts,
            StrongholdStructureStart::structure_id,
        );
        self
    }
}

mod buried_treasure;
mod command_placement;
mod end_city;
pub mod jigsaw;
pub mod nbt;
mod nether;
mod persistence;
mod placement;
pub use placement::PlaceableStart;
mod references;
pub mod spawn_overrides;
mod terrain;
#[cfg(test)]
pub mod tests;

use nbt::*;
