//! Deterministic jigsaw piece-graph expansion before block placement.

use crate::fast_hash::FastHashMap;
use crate::random::large_feature_random;
use std::collections::{BTreeMap, VecDeque};
use std::sync::OnceLock;

use crate::random::WorldgenRandom;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use crate::height_provider::{sample_height_provider_with_bounds, HeightProvider, VerticalAnchor};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::structure_pool::{
    StructureLiquidSettings, StructurePoolDefinition, StructurePoolElement,
    StructurePoolProjection, StructurePoolSet,
};
use bbb_server_datapack::structure_template::{
    StructureTemplate, StructureTemplateDirection, StructureTemplateJigsaw,
    StructureTemplateJigsawJoint, StructureTemplateSet,
};

use super::position_seed;
use super::structures::{
    DimensionPadding, JigsawMaxDistance, JigsawStructureSettings, NoiseStructurePlan,
    PoolAliasBinding, StructureHeightmap, StructureTerrainAdjustment, WeightedAliasGroup,
    WeightedPool,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JigsawRotation {
    None,
    Clockwise90,
    Clockwise180,
    CounterClockwise90,
}

const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JigsawBoundingBox {
    pub min: BlockPos,
    pub max: BlockPos,
}

/// Lazily cached union + terrain-adjusted structure bounding box
/// (`StructureStart.java:69-77`, `Structure.java:79-81`).
#[derive(Debug, Default)]
pub(crate) struct CachedStartBoundingBox(OnceLock<Option<JigsawBoundingBox>>);

impl CachedStartBoundingBox {
    pub(crate) const fn new() -> Self {
        Self(OnceLock::new())
    }

    pub(crate) fn invalidate(&mut self) {
        self.0.take();
    }

    pub(crate) fn get_or_init(
        &self,
        init: impl FnOnce() -> Option<JigsawBoundingBox>,
    ) -> Option<JigsawBoundingBox> {
        *self.0.get_or_init(init)
    }
}

impl Clone for CachedStartBoundingBox {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq for CachedStartBoundingBox {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for CachedStartBoundingBox {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JigsawPiece {
    pub element: StructurePoolElement,
    pub position: BlockPos,
    pub ground_level_delta: i32,
    pub rotation: JigsawRotation,
    pub bounding_box: JigsawBoundingBox,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JigsawGraphJunction {
    pub piece_index: usize,
    pub source_x: i32,
    pub source_ground_y: i32,
    pub source_z: i32,
    pub delta_y: i32,
    pub destination_projection: StructurePoolProjection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JigsawPieceGraph {
    pub structure_id: String,
    pub start_chunk: [i32; 2],
    pub generation_anchor: BlockPos,
    pub center_bounding_box: JigsawBoundingBox,
    pub liquid_settings: StructureLiquidSettings,
    pub pieces: Vec<JigsawPiece>,
    pub junctions: Vec<JigsawGraphJunction>,
}

/// `ChunkGenerator.getFirstFreeHeight` for structure starts: jigsaw projection
/// and mesa mineshaft placement.
pub trait JigsawHeightAccess {
    fn first_free_height(&self, x: i32, z: i32, heightmap: StructureHeightmap) -> Option<i32>;

    fn generation_bounds(&self) -> Option<(i32, i32)> {
        None
    }
}

pub trait JigsawTemplateAccess {
    fn structure_template(&self, id: &str) -> Option<StructureTemplate>;
}

impl JigsawTemplateAccess for StructureTemplateSet {
    fn structure_template(&self, id: &str) -> Option<StructureTemplate> {
        self.template(id).map(|entry| entry.template.clone())
    }
}

/// `JigsawPlacement.generateJigsaw` (`JigsawPlacement.java:212-264`).
#[allow(clippy::too_many_arguments)]
pub fn jigsaw_pool_piece_graph(
    pools: &StructurePoolSet,
    templates: &dyn JigsawTemplateAccess,
    pool: &str,
    target: &str,
    max_depth: i32,
    position: BlockPos,
    seed: i64,
    world_min_y: i32,
    world_height: i32,
    height_access: &dyn JigsawHeightAccess,
) -> Option<JigsawPieceGraph> {
    let settings = JigsawStructureSettings {
        start_pool: pool.to_owned(),
        start_jigsaw_name: Some(target.to_owned()),
        size: max_depth,
        start_height: HeightProvider::Constant(VerticalAnchor::Absolute(position.y)),
        use_expansion_hack: false,
        project_start_to_heightmap: None,
        max_distance_from_center: JigsawMaxDistance {
            horizontal: 128,
            vertical: 128,
        },
        pool_aliases: Vec::new(),
        dimension_padding: DimensionPadding { bottom: 0, top: 0 },
        liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
    };
    let chunk = [position.x >> 4, position.z >> 4];
    let mut random = large_feature_random(seed, chunk[0], chunk[1]);
    build_graph(
        pool,
        chunk,
        position,
        &settings,
        pools,
        templates,
        world_min_y,
        world_min_y + world_height - 1,
        height_access,
        AliasLookup::default(),
        &mut random,
    )
}

/// Result of the center-piece phase of jigsaw placement, exposing
/// [`Self::generation_anchor`] so callers can gate on biome validity before
/// expanding child pieces (`JigsawPlacement.java:67-155`).
pub struct PlannedJigsawAnchor<'a> {
    structure_id: String,
    start_chunk: [i32; 2],
    generation_anchor: BlockPos,
    center_piece: JigsawPiece,
    settings: &'a JigsawStructureSettings,
    pools: &'a StructurePoolSet,
    world_min_y: i32,
    world_max_y: i32,
    aliases: AliasLookup,
    random: WorldgenRandom,
}

impl PlannedJigsawAnchor<'_> {
    pub const fn generation_anchor(&self) -> BlockPos {
        self.generation_anchor
    }

    pub fn expand(
        self,
        startup: &DatapackStartup,
        height_access: &dyn JigsawHeightAccess,
    ) -> Option<JigsawPieceGraph> {
        self.expand_with_templates(&startup.structures, height_access)
    }

    pub fn expand_with_templates(
        mut self,
        templates: &dyn JigsawTemplateAccess,
        height_access: &dyn JigsawHeightAccess,
    ) -> Option<JigsawPieceGraph> {
        expand_planned_anchor(
            self.structure_id,
            self.start_chunk,
            self.generation_anchor,
            self.center_piece,
            self.settings,
            self.pools,
            templates,
            self.world_min_y,
            self.world_max_y,
            height_access,
            self.aliases,
            &mut self.random,
        )
    }
}

impl NoiseStructurePlan {
    /// Plans only the jigsaw center piece and generation anchor so callers can
    /// verify biome eligibility before expanding child pieces.
    #[allow(clippy::too_many_arguments)]
    pub fn plan_jigsaw_anchor(
        &self,
        startup: &DatapackStartup,
        structure_id: &str,
        level_seed: i64,
        start_chunk_x: i32,
        start_chunk_z: i32,
        world_min_y: i32,
        world_height: i32,
        height_access: &dyn JigsawHeightAccess,
    ) -> Option<PlannedJigsawAnchor<'_>> {
        self.plan_jigsaw_anchor_with_templates(
            &startup.structures,
            structure_id,
            level_seed,
            start_chunk_x,
            start_chunk_z,
            world_min_y,
            world_height,
            height_access,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn plan_jigsaw_anchor_with_templates(
        &self,
        templates: &dyn JigsawTemplateAccess,
        structure_id: &str,
        level_seed: i64,
        start_chunk_x: i32,
        start_chunk_z: i32,
        world_min_y: i32,
        world_height: i32,
        height_access: &dyn JigsawHeightAccess,
    ) -> Option<PlannedJigsawAnchor<'_>> {
        let settings = self.structures().get(structure_id)?.jigsaw.as_ref()?;
        let world_max_y = world_min_y + world_height - 1;
        let mut random = large_feature_random(level_seed, start_chunk_x, start_chunk_z);
        let (generation_min_y, generation_height) = height_access
            .generation_bounds()
            .unwrap_or((world_min_y, world_height));
        let position = BlockPos {
            x: start_chunk_x * 16,
            y: sample_height_provider_with_bounds(
                &settings.start_height,
                generation_min_y,
                generation_height,
                &mut random,
            )
            .ok()?,
            z: start_chunk_z * 16,
        };
        let aliases = AliasLookup::create(&settings.pool_aliases, level_seed, position)?;
        let (generation_anchor, center_piece) = plan_center_piece(
            position,
            settings,
            self.pools(),
            templates,
            world_min_y,
            world_max_y,
            height_access,
            &aliases,
            &mut random,
        )?;
        Some(PlannedJigsawAnchor {
            structure_id: structure_id.to_string(),
            start_chunk: [start_chunk_x, start_chunk_z],
            generation_anchor,
            center_piece,
            settings,
            pools: self.pools(),
            world_min_y,
            world_max_y,
            aliases,
            random,
        })
    }

    /// Builds the ordered, bounded piece graph used by the later template-piece,
    /// StructureStart and Beardifier owners. No blocks or chunk status are written.
    #[allow(clippy::too_many_arguments)]
    pub fn jigsaw_piece_graph(
        &self,
        startup: &DatapackStartup,
        structure_id: &str,
        level_seed: i64,
        start_chunk_x: i32,
        start_chunk_z: i32,
        world_min_y: i32,
        world_height: i32,
        height_access: &dyn JigsawHeightAccess,
    ) -> Option<JigsawPieceGraph> {
        self.jigsaw_piece_graph_with_templates(
            &startup.structures,
            structure_id,
            level_seed,
            start_chunk_x,
            start_chunk_z,
            world_min_y,
            world_height,
            height_access,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn jigsaw_piece_graph_with_templates(
        &self,
        templates: &dyn JigsawTemplateAccess,
        structure_id: &str,
        level_seed: i64,
        start_chunk_x: i32,
        start_chunk_z: i32,
        world_min_y: i32,
        world_height: i32,
        height_access: &dyn JigsawHeightAccess,
    ) -> Option<JigsawPieceGraph> {
        self.plan_jigsaw_anchor_with_templates(
            templates,
            structure_id,
            level_seed,
            start_chunk_x,
            start_chunk_z,
            world_min_y,
            world_height,
            height_access,
        )?
        .expand_with_templates(templates, height_access)
    }
}

#[allow(clippy::too_many_arguments)]
fn build_graph(
    structure_id: &str,
    start_chunk: [i32; 2],
    position: BlockPos,
    settings: &JigsawStructureSettings,
    pools: &StructurePoolSet,
    templates: &dyn JigsawTemplateAccess,
    world_min_y: i32,
    world_max_y: i32,
    height_access: &dyn JigsawHeightAccess,
    aliases: AliasLookup,
    random: &mut dyn RandomSource,
) -> Option<JigsawPieceGraph> {
    let (generation_anchor, center_piece) = plan_center_piece(
        position,
        settings,
        pools,
        templates,
        world_min_y,
        world_max_y,
        height_access,
        &aliases,
        random,
    )?;
    expand_planned_anchor(
        structure_id.to_string(),
        start_chunk,
        generation_anchor,
        center_piece,
        settings,
        pools,
        templates,
        world_min_y,
        world_max_y,
        height_access,
        aliases,
        random,
    )
}

#[allow(clippy::too_many_arguments)]
fn plan_center_piece(
    position: BlockPos,
    settings: &JigsawStructureSettings,
    pools: &StructurePoolSet,
    templates: &dyn JigsawTemplateAccess,
    world_min_y: i32,
    world_max_y: i32,
    height_access: &dyn JigsawHeightAccess,
    aliases: &AliasLookup,
    random: &mut dyn RandomSource,
) -> Option<(BlockPos, JigsawPiece)> {
    // `JigsawPlacement.addPieces` chooses rotation before the weighted center
    // template (`JigsawPlacement.java:67-80`).
    let center_rotation = random_rotation(random);
    let original_pool = pools.pool(&settings.start_pool)?;
    let center_pool = pools
        .pool(aliases.lookup(&settings.start_pool))
        .unwrap_or(original_pool);
    let center_element = random_pool_element(center_pool, random)?;
    let center_ground_level_delta = element_ground_level_delta(&center_element);

    let adjusted_position = if let Some(start_name) = &settings.start_jigsaw_name {
        let connectors = shuffled_connectors(
            &center_element,
            templates,
            position,
            center_rotation,
            random,
        );
        let anchor = connectors
            .iter()
            .find(|connector| connector.name == *start_name)?
            .position;
        sub(position, sub(anchor, position))
    } else {
        position
    };

    let mut center_box = element_bounding_box(
        &center_element,
        templates,
        adjusted_position,
        center_rotation,
    )?;
    let center_x = midpoint(center_box.min.x, center_box.max.x);
    let center_z = midpoint(center_box.min.z, center_box.max.z);
    let bottom_y = match settings.project_start_to_heightmap {
        None => adjusted_position.y,
        Some(heightmap) => {
            position.y + height_access.first_free_height(center_x, center_z, heightmap)?
        }
    };
    let move_y = bottom_y - (center_box.min.y + center_ground_level_delta);
    let center_position = offset(adjusted_position, 0, move_y, 0);
    center_box = center_box.moved(0, move_y, 0);
    let padding = settings.dimension_padding;
    if (padding.bottom != 0 || padding.top != 0)
        && (center_box.min.y < world_min_y + padding.bottom
            || center_box.max.y > world_max_y - padding.top)
    {
        return None;
    }

    let center_y = bottom_y + position.y - adjusted_position.y;
    let generation_anchor = BlockPos {
        x: center_x,
        y: center_y,
        z: center_z,
    };
    let center_piece = JigsawPiece {
        element: center_element,
        position: center_position,
        ground_level_delta: center_ground_level_delta,
        rotation: center_rotation,
        bounding_box: center_box,
    };
    Some((generation_anchor, center_piece))
}

#[allow(clippy::too_many_arguments)]
fn expand_planned_anchor(
    structure_id: String,
    start_chunk: [i32; 2],
    generation_anchor: BlockPos,
    center_piece: JigsawPiece,
    settings: &JigsawStructureSettings,
    pools: &StructurePoolSet,
    templates: &dyn JigsawTemplateAccess,
    world_min_y: i32,
    world_max_y: i32,
    height_access: &dyn JigsawHeightAccess,
    aliases: AliasLookup,
    random: &mut dyn RandomSource,
) -> Option<JigsawPieceGraph> {
    let center_box = center_piece.bounding_box;
    let mut graph = JigsawPieceGraph {
        structure_id,
        start_chunk,
        generation_anchor,
        center_bounding_box: center_box,
        liquid_settings: settings.liquid_settings,
        pieces: vec![center_piece],
        junctions: Vec::new(),
    };
    if settings.size == 0 {
        // JigsawPlacement.java:125-153: zero depth adds no piece to the builder.
        graph.pieces.clear();
        return Some(graph);
    }

    let padding = settings.dimension_padding;
    let horizontal = settings.max_distance_from_center.horizontal;
    let vertical = settings.max_distance_from_center.vertical;
    let boundary = JigsawBoundingBox {
        min: BlockPos {
            x: generation_anchor.x - horizontal,
            y: (generation_anchor.y - vertical).max(world_min_y + padding.bottom),
            z: generation_anchor.z - horizontal,
        },
        max: BlockPos {
            x: generation_anchor.x + horizontal,
            y: (generation_anchor.y + vertical).min(world_max_y - padding.top),
            z: generation_anchor.z + horizontal,
        },
    };
    let mut contexts = vec![FreeContext {
        boundary,
        occupied: vec![center_box],
    }];
    let mut queue = PriorityQueue::default();
    let mut pool_max_heights = FastHashMap::default();
    try_placing_children(
        0,
        0,
        0,
        settings,
        pools,
        templates,
        height_access,
        &aliases,
        random,
        &mut pool_max_heights,
        &mut graph,
        &mut contexts,
        &mut queue,
    )?;
    while let Some(state) = queue.pop() {
        try_placing_children(
            state.piece_index,
            state.context_index,
            state.depth,
            settings,
            pools,
            templates,
            height_access,
            &aliases,
            random,
            &mut pool_max_heights,
            &mut graph,
            &mut contexts,
            &mut queue,
        )?;
    }
    Some(graph)
}

#[derive(Clone, Copy)]
struct PieceState {
    piece_index: usize,
    context_index: usize,
    depth: i32,
}

#[derive(Default)]
struct PriorityQueue {
    queues: BTreeMap<i32, VecDeque<PieceState>>,
}

impl PriorityQueue {
    fn push(&mut self, state: PieceState, priority: i32) {
        self.queues.entry(priority).or_default().push_back(state);
    }

    fn pop(&mut self) -> Option<PieceState> {
        // `SequencedPriorityIterator.java:20-67`: highest priority first,
        // preserving FIFO within one priority.
        let priority = *self.queues.last_key_value()?.0;
        let queue = self.queues.get_mut(&priority).unwrap();
        let state = queue.pop_front();
        if queue.is_empty() {
            self.queues.remove(&priority);
        }
        state
    }
}

struct FreeContext {
    boundary: JigsawBoundingBox,
    occupied: Vec<JigsawBoundingBox>,
}

impl FreeContext {
    fn accepts(&self, target: JigsawBoundingBox) -> bool {
        // `JigsawPlacement.java:409-415` builds these shapes solely from integer
        // bounding boxes; its 0.25-deflated target fits exactly when it stays in
        // the boundary and shares no integer cell with an occupied box.
        self.boundary.contains_box(target)
            && self
                .occupied
                .iter()
                .all(|occupied| !occupied.intersects(target))
    }
}

#[allow(clippy::too_many_arguments)]
fn try_placing_children<'p>(
    source_index: usize,
    context_index: usize,
    depth: i32,
    settings: &JigsawStructureSettings,
    pools: &'p StructurePoolSet,
    templates: &dyn JigsawTemplateAccess,
    height_access: &dyn JigsawHeightAccess,
    aliases: &AliasLookup,
    random: &mut dyn RandomSource,
    pool_max_heights: &mut FastHashMap<&'p str, i32>,
    graph: &mut JigsawPieceGraph,
    contexts: &mut Vec<FreeContext>,
    queue: &mut PriorityQueue,
) -> Option<()> {
    let source_piece = graph.pieces[source_index].clone();
    let source_projection = element_projection(&source_piece.element);
    let source_rigid = source_projection == StructurePoolProjection::Rigid;
    let source_connectors = shuffled_connectors(
        &source_piece.element,
        templates,
        source_piece.position,
        source_piece.rotation,
        random,
    );
    let mut internal_context = None;
    for source in source_connectors {
        let source_direction = source.front;
        let target_jigsaw_position = relative(source.position, source_direction);
        let source_local_y = source.position.y - source_piece.bounding_box.min.y;
        let mut source_base_height = None;
        let Some(target_pool) = pools.pool(aliases.lookup(&source.pool)) else {
            continue;
        };
        if target_pool_size(target_pool) == 0 && target_pool.id != "minecraft:empty" {
            continue;
        }
        let Some(alternate_pool) = pools.pool(&target_pool.alternate_pool) else {
            continue;
        };
        if target_pool_size(alternate_pool) == 0 && alternate_pool.id != "minecraft:empty" {
            continue;
        }
        let chosen_context = if source_piece.bounding_box.contains(target_jigsaw_position) {
            match internal_context {
                Some(index) => index,
                None => {
                    let index = contexts.len();
                    contexts.push(FreeContext {
                        boundary: source_piece.bounding_box,
                        occupied: Vec::new(),
                    });
                    internal_context = Some(index);
                    index
                }
            }
        } else {
            context_index
        };

        let target_offset = 0;
        let alternate_offset = target_pool.elements.len();
        let mut candidates = Vec::new();
        if depth != settings.size {
            candidates.extend(shuffled_pool_elements(target_pool, target_offset, random));
        }
        candidates.extend(shuffled_pool_elements(
            alternate_pool,
            alternate_offset,
            random,
        ));
        // Skip already-failed duplicate copies of weighted entries: every rotation
        // and connector is tested deterministically on the first attempt.
        let mut tried_entries =
            vec![false; target_pool.elements.len() + alternate_pool.elements.len()];
        let mut accepted = false;
        for (entry_slot, target_element) in candidates {
            if matches!(target_element, StructurePoolElement::Empty) {
                break;
            }
            if tried_entries[entry_slot] {
                continue;
            }
            tried_entries[entry_slot] = true;
            let mut rotations = ROTATIONS;
            util_shuffle(&mut rotations, random);
            for target_rotation in rotations {
                let target_connectors = shuffled_connectors(
                    target_element,
                    templates,
                    BlockPos { x: 0, y: 0, z: 0 },
                    target_rotation,
                    random,
                );
                let expand_to = if settings.use_expansion_hack {
                    let hack_box = element_bounding_box(
                        target_element,
                        templates,
                        BlockPos { x: 0, y: 0, z: 0 },
                        target_rotation,
                    )?;
                    if hack_box.y_span() <= 16 {
                        max_child_pool_height(
                            &target_connectors,
                            hack_box,
                            pools,
                            templates,
                            aliases,
                            pool_max_heights,
                        )?
                    } else {
                        0
                    }
                } else {
                    0
                };
                for target in target_connectors {
                    if !can_attach(&source, &target) {
                        continue;
                    }
                    let raw_target_position = sub(target_jigsaw_position, target.position);
                    let raw_target_box = element_bounding_box(
                        target_element,
                        templates,
                        raw_target_position,
                        target_rotation,
                    )?;
                    let target_projection = element_projection(target_element);
                    let target_rigid = target_projection == StructurePoolProjection::Rigid;
                    let target_local_y = target.position.y;
                    let delta_y =
                        source_local_y - target_local_y + direction_step_y(source_direction);
                    let target_box_y = if source_rigid && target_rigid {
                        source_piece.bounding_box.min.y + delta_y
                    } else {
                        cached_surface_height(
                            &mut source_base_height,
                            height_access,
                            source.position,
                        )? - target_local_y
                    };
                    let y_offset = target_box_y - raw_target_box.min.y;
                    let target_position = offset(raw_target_position, 0, y_offset, 0);
                    let mut target_box = raw_target_box.moved(0, y_offset, 0);
                    if expand_to > 0 {
                        let height_delta = target_box.max.y - target_box.min.y;
                        target_box.max.y = target_box.min.y + (expand_to + 1).max(height_delta);
                    }
                    if !contexts[chosen_context].accepts(target_box) {
                        continue;
                    }
                    contexts[chosen_context].occupied.push(target_box);
                    let target_ground_delta = if target_rigid {
                        source_piece.ground_level_delta - delta_y
                    } else {
                        element_ground_level_delta(target_element)
                    };
                    let junction_y = if source_rigid {
                        source_piece.bounding_box.min.y + source_local_y
                    } else if target_rigid {
                        target_box_y + target_local_y
                    } else {
                        cached_surface_height(
                            &mut source_base_height,
                            height_access,
                            source.position,
                        )? + delta_y / 2
                    };
                    graph.junctions.push(JigsawGraphJunction {
                        piece_index: source_index,
                        source_x: target_jigsaw_position.x,
                        source_ground_y: junction_y - source_local_y
                            + source_piece.ground_level_delta,
                        source_z: target_jigsaw_position.z,
                        delta_y,
                        destination_projection: target_projection,
                    });
                    let target_index = graph.pieces.len();
                    graph.pieces.push(JigsawPiece {
                        element: target_element.clone(),
                        position: target_position,
                        ground_level_delta: target_ground_delta,
                        rotation: target_rotation,
                        bounding_box: target_box,
                    });
                    graph.junctions.push(JigsawGraphJunction {
                        piece_index: target_index,
                        source_x: source.position.x,
                        source_ground_y: junction_y - target_local_y + target_ground_delta,
                        source_z: source.position.z,
                        delta_y: -delta_y,
                        destination_projection: source_projection,
                    });
                    if depth < settings.size {
                        queue.push(
                            PieceState {
                                piece_index: target_index,
                                context_index: chosen_context,
                                depth: depth + 1,
                            },
                            source.placement_priority,
                        );
                    }
                    accepted = true;
                    break;
                }
                if accepted {
                    break;
                }
            }
            if accepted {
                break;
            }
        }
    }
    Some(())
}

#[derive(Clone, Debug)]
struct RuntimeConnector {
    position: BlockPos,
    front: StructureTemplateDirection,
    top: StructureTemplateDirection,
    joint: StructureTemplateJigsawJoint,
    name: String,
    pool: String,
    target: String,
    placement_priority: i32,
    selection_priority: i32,
}

fn shuffled_connectors(
    element: &StructurePoolElement,
    templates: &dyn JigsawTemplateAccess,
    position: BlockPos,
    rotation: JigsawRotation,
    random: &mut dyn RandomSource,
) -> Vec<RuntimeConnector> {
    // `SinglePoolElement.java:106-122`: shuffle first, then stable-sort the
    // shuffled result by descending selection priority.
    let mut connectors = element_connectors(element, templates, position, rotation);
    shuffle_and_sort_connectors(&mut connectors, random);
    connectors
}

fn shuffle_and_sort_connectors(connectors: &mut [RuntimeConnector], random: &mut dyn RandomSource) {
    util_shuffle(connectors, random);
    connectors.sort_by_key(|connector| std::cmp::Reverse(connector.selection_priority));
}

fn element_connectors(
    element: &StructurePoolElement,
    templates: &dyn JigsawTemplateAccess,
    position: BlockPos,
    rotation: JigsawRotation,
) -> Vec<RuntimeConnector> {
    match element {
        StructurePoolElement::Empty => Vec::new(),
        StructurePoolElement::Feature { .. } => vec![RuntimeConnector {
            position,
            front: StructureTemplateDirection::Down,
            top: rotate_direction(StructureTemplateDirection::South, rotation),
            joint: StructureTemplateJigsawJoint::Rollable,
            name: "minecraft:bottom".to_string(),
            pool: "minecraft:empty".to_string(),
            target: "minecraft:empty".to_string(),
            placement_priority: 0,
            selection_priority: 0,
        }],
        StructurePoolElement::Single { location, .. } => {
            let Some(template) = templates.structure_template(location) else {
                // `StructureTemplateManager.getOrCreate` (`StructureTemplateManager.java:83-92`).
                return Vec::new();
            };
            let materialized = template.materialized();
            if materialized.palettes.is_empty() {
                return Vec::new();
            }
            let mut palette_random =
                WorldgenRandom::with_seed(position_seed(position.x, position.y, position.z));
            let palette_index =
                palette_random.next_int(materialized.palettes.len() as i32) as usize;
            materialized.palettes[palette_index]
                .jigsaws
                .iter()
                .map(|jigsaw| runtime_connector(jigsaw, position, rotation))
                .collect()
        }
        StructurePoolElement::List { elements, .. } => elements
            .first()
            .map(|first| element_connectors(first, templates, position, rotation))
            .unwrap_or_default(),
    }
}

fn runtime_connector(
    jigsaw: &StructureTemplateJigsaw,
    position: BlockPos,
    rotation: JigsawRotation,
) -> RuntimeConnector {
    RuntimeConnector {
        position: add(position, rotate_position(jigsaw.position, rotation)),
        front: rotate_direction(jigsaw.front, rotation),
        top: rotate_direction(jigsaw.top, rotation),
        joint: jigsaw.joint,
        name: jigsaw.name.clone(),
        pool: jigsaw.pool.clone(),
        target: jigsaw.target.clone(),
        placement_priority: jigsaw.placement_priority,
        selection_priority: jigsaw.selection_priority,
    }
}

fn can_attach(source: &RuntimeConnector, target: &RuntimeConnector) -> bool {
    source.front == opposite(target.front)
        && (source.joint == StructureTemplateJigsawJoint::Rollable || source.top == target.top)
        && source.target == target.name
}

fn cached_surface_height(
    cached: &mut Option<i32>,
    height_access: &dyn JigsawHeightAccess,
    position: BlockPos,
) -> Option<i32> {
    if cached.is_none() {
        *cached = height_access.first_free_height(
            position.x,
            position.z,
            StructureHeightmap::WorldSurfaceWg,
        );
    }
    *cached
}

pub fn element_projection(element: &StructurePoolElement) -> StructurePoolProjection {
    match element {
        StructurePoolElement::Empty => StructurePoolProjection::TerrainMatching,
        StructurePoolElement::Feature { projection, .. }
        | StructurePoolElement::Single { projection, .. }
        | StructurePoolElement::List { projection, .. } => *projection,
    }
}

fn element_ground_level_delta(element: &StructurePoolElement) -> i32 {
    // Every 26.1 `StructurePoolElement` family inherits the base value
    // (`StructurePoolElement.java:92-94`); keeping the dispatch here makes a
    // later family-specific override an owned graph change.
    match element {
        StructurePoolElement::Empty
        | StructurePoolElement::Feature { .. }
        | StructurePoolElement::Single { .. }
        | StructurePoolElement::List { .. } => 1,
    }
}

pub fn element_bounding_box(
    element: &StructurePoolElement,
    templates: &dyn JigsawTemplateAccess,
    position: BlockPos,
    rotation: JigsawRotation,
) -> Option<JigsawBoundingBox> {
    match element {
        StructurePoolElement::Empty => None,
        StructurePoolElement::Feature { .. } => Some(JigsawBoundingBox {
            min: position,
            max: position,
        }),
        StructurePoolElement::Single { location, .. } => {
            let [size_x, size_y, size_z] = templates
                .structure_template(location)
                .map(|template| template.materialized().size)
                .unwrap_or([0, 0, 0]);
            // Empty/`getOrCreate` size is ZERO; Vanilla still uses
            // `size.offset(-1,-1,-1)` (`StructureTemplate.java:624-628`).
            let far = BlockPos {
                x: size_x - 1,
                y: size_y - 1,
                z: size_z - 1,
            };
            let other = add(position, rotate_position(far, rotation));
            Some(JigsawBoundingBox::from_corners(position, other))
        }
        StructurePoolElement::List { elements, .. } => {
            let mut boxes = elements
                .iter()
                .filter(|element| !matches!(element, StructurePoolElement::Empty))
                .map(|element| element_bounding_box(element, templates, position, rotation));
            let mut result = boxes.next()??;
            for child in boxes {
                result.encapsulate(child?);
            }
            Some(result)
        }
    }
}

fn max_child_pool_height<'p>(
    connectors: &[RuntimeConnector],
    element_box: JigsawBoundingBox,
    pools: &'p StructurePoolSet,
    templates: &dyn JigsawTemplateAccess,
    aliases: &AliasLookup,
    pool_max_heights: &mut FastHashMap<&'p str, i32>,
) -> Option<i32> {
    let mut max_height = 0;
    for connector in connectors {
        if !element_box.contains(relative(connector.position, connector.front)) {
            continue;
        }
        let Some(pool) = pools.pool(aliases.lookup(&connector.pool)) else {
            continue;
        };
        let pool_height = pool_max_height(pool, templates, pool_max_heights)?;
        let alternate_height = match pools.pool(&pool.alternate_pool) {
            Some(pool) => pool_max_height(pool, templates, pool_max_heights)?,
            None => 0,
        };
        max_height = max_height.max(pool_height).max(alternate_height);
    }
    Some(max_height)
}

fn pool_max_height<'p>(
    pool: &'p StructurePoolDefinition,
    templates: &dyn JigsawTemplateAccess,
    pool_max_heights: &mut FastHashMap<&'p str, i32>,
) -> Option<i32> {
    if let Some(&height) = pool_max_heights.get(pool.id.as_str()) {
        return Some(height);
    }
    let height = pool
        .elements
        .iter()
        .filter(|entry| !matches!(entry.element, StructurePoolElement::Empty))
        .try_fold(0, |maximum, entry| {
            let bounds = element_bounding_box(
                &entry.element,
                templates,
                BlockPos { x: 0, y: 0, z: 0 },
                JigsawRotation::None,
            )?;
            Some(maximum.max(bounds.y_span()))
        })?;
    pool_max_heights.insert(pool.id.as_str(), height);
    Some(height)
}

fn target_pool_size(pool: &StructurePoolDefinition) -> i32 {
    pool.elements.iter().map(|entry| entry.weight).sum()
}

fn random_pool_element(
    pool: &StructurePoolDefinition,
    random: &mut dyn RandomSource,
) -> Option<StructurePoolElement> {
    let size = target_pool_size(pool);
    if size == 0 {
        return None;
    }
    let mut choice = random.next_int(size);
    let entry = pool.elements.iter().find(|entry| {
        choice -= entry.weight;
        choice < 0
    })?;
    (!matches!(entry.element, StructurePoolElement::Empty)).then(|| entry.element.clone())
}

fn shuffled_pool_elements<'p>(
    pool: &'p StructurePoolDefinition,
    slot_offset: usize,
    random: &mut dyn RandomSource,
) -> Vec<(usize, &'p StructurePoolElement)> {
    let mut elements = Vec::with_capacity(target_pool_size(pool) as usize);
    for (entry_index, entry) in pool.elements.iter().enumerate() {
        let slot = slot_offset + entry_index;
        for _ in 0..entry.weight {
            elements.push((slot, &entry.element));
        }
    }
    util_shuffle(&mut elements, random);
    elements
}

#[derive(Default)]
struct AliasLookup {
    aliases: BTreeMap<String, String>,
}

impl AliasLookup {
    fn create(bindings: &[PoolAliasBinding], seed: i64, position: BlockPos) -> Option<Self> {
        // One positional random resolves all bindings in declaration order
        // before graph expansion starts.
        let mut random = WorldgenRandom::at(seed, position.x, position.y, position.z);
        let mut lookup = Self::default();
        for binding in bindings {
            lookup.resolve_binding(binding, &mut random)?;
        }
        Some(lookup)
    }

    fn lookup<'a>(&'a self, id: &'a str) -> &'a str {
        self.aliases.get(id).map(String::as_str).unwrap_or(id)
    }

    fn resolve_binding(
        &mut self,
        binding: &PoolAliasBinding,
        random: &mut dyn RandomSource,
    ) -> Option<()> {
        match binding {
            PoolAliasBinding::Direct { alias, target } => self.insert(alias, target),
            PoolAliasBinding::Random { alias, targets } => {
                let target = choose_weighted_pool(targets, random)?;
                self.insert(alias, target)
            }
            PoolAliasBinding::RandomGroup { groups } => {
                let group = choose_weighted_group(groups, random)?;
                for binding in &group.bindings {
                    self.resolve_binding(binding, random)?;
                }
                Some(())
            }
        }
    }

    /// `ImmutableMap.Builder.build` rejects a duplicate alias.
    fn insert(&mut self, alias: &str, target: &str) -> Option<()> {
        self.aliases
            .insert(alias.to_string(), target.to_string())
            .is_none()
            .then_some(())
    }
}

fn choose_weighted_pool<'a>(
    entries: &'a [WeightedPool],
    random: &mut dyn RandomSource,
) -> Option<&'a str> {
    let total = entries.iter().map(|entry| entry.weight).sum::<i32>();
    if total <= 0 {
        return None;
    }
    let mut choice = random.next_int(total);
    entries.iter().find_map(|entry| {
        choice -= entry.weight;
        (choice < 0).then_some(entry.pool.as_str())
    })
}

fn choose_weighted_group<'a>(
    entries: &'a [WeightedAliasGroup],
    random: &mut dyn RandomSource,
) -> Option<&'a WeightedAliasGroup> {
    let total = entries.iter().map(|entry| entry.weight).sum::<i32>();
    if total <= 0 {
        return None;
    }
    let mut choice = random.next_int(total);
    entries.iter().find(|entry| {
        choice -= entry.weight;
        choice < 0
    })
}

impl JigsawBoundingBox {
    pub const fn from_array(bounds: [i32; 6]) -> Self {
        Self {
            min: BlockPos {
                x: bounds[0],
                y: bounds[1],
                z: bounds[2],
            },
            max: BlockPos {
                x: bounds[3],
                y: bounds[4],
                z: bounds[5],
            },
        }
    }

    pub const fn point(position: BlockPos) -> Self {
        Self {
            min: position,
            max: position,
        }
    }

    /// `Structure.adjustBoundingBox` (`Structure.java:79-81`): inflates every
    /// non-`None` terrain-adapting bounding box by 12 blocks in all directions.
    pub const fn adjusted_for_terrain(
        self,
        terrain_adjustment: StructureTerrainAdjustment,
    ) -> Self {
        if matches!(terrain_adjustment, StructureTerrainAdjustment::None) {
            self
        } else {
            Self {
                min: BlockPos {
                    x: self.min.x.saturating_sub(12),
                    y: self.min.y.saturating_sub(12),
                    z: self.min.z.saturating_sub(12),
                },
                max: BlockPos {
                    x: self.max.x.saturating_add(12),
                    y: self.max.y.saturating_add(12),
                    z: self.max.z.saturating_add(12),
                },
            }
        }
    }

    /// `StructureStart.getBoundingBox` (`StructureStart.java:69-77`) and
    /// `Structure.adjustBoundingBox` (`Structure.java:79-81`).
    pub fn adjusted_union(
        boxes: impl IntoIterator<Item = Self>,
        terrain_adjustment: StructureTerrainAdjustment,
    ) -> Option<Self> {
        let mut iter = boxes.into_iter();
        let mut bounds = iter.next()?;
        for other in iter {
            bounds.encapsulate(other);
        }
        Some(bounds.adjusted_for_terrain(terrain_adjustment))
    }

    pub fn intersects_chunk(self, chunk_x: i32, chunk_z: i32) -> bool {
        let min_x = i64::from(chunk_x) * 16;
        let min_z = i64::from(chunk_z) * 16;
        i64::from(self.min.x) <= min_x + 15
            && i64::from(self.max.x) >= min_x
            && i64::from(self.min.z) <= min_z + 15
            && i64::from(self.max.z) >= min_z
    }

    fn from_corners(first: BlockPos, second: BlockPos) -> Self {
        Self {
            min: BlockPos {
                x: first.x.min(second.x),
                y: first.y.min(second.y),
                z: first.z.min(second.z),
            },
            max: BlockPos {
                x: first.x.max(second.x),
                y: first.y.max(second.y),
                z: first.z.max(second.z),
            },
        }
    }

    fn moved(self, x: i32, y: i32, z: i32) -> Self {
        Self {
            min: offset(self.min, x, y, z),
            max: offset(self.max, x, y, z),
        }
    }

    pub fn contains(self, position: BlockPos) -> bool {
        position.x >= self.min.x
            && position.x <= self.max.x
            && position.y >= self.min.y
            && position.y <= self.max.y
            && position.z >= self.min.z
            && position.z <= self.max.z
    }

    fn contains_box(self, other: Self) -> bool {
        self.contains(other.min) && self.contains(other.max)
    }

    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    fn encapsulate(&mut self, other: Self) {
        *self = Self::from_corners(
            BlockPos {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
                z: self.min.z.min(other.min.z),
            },
            BlockPos {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
                z: self.max.z.max(other.max.z),
            },
        );
    }

    fn y_span(self) -> i32 {
        self.max.y - self.min.y + 1
    }
}

fn random_rotation(random: &mut dyn RandomSource) -> JigsawRotation {
    ROTATIONS[random.next_int(ROTATIONS.len() as i32) as usize]
}

fn util_shuffle<T>(values: &mut [T], random: &mut dyn RandomSource) {
    for index in (1..values.len()).rev() {
        let swap_to = random.next_int((index + 1) as i32) as usize;
        values.swap(index, swap_to);
    }
}

pub fn rotate_position(position: BlockPos, rotation: JigsawRotation) -> BlockPos {
    let (x, z) = match rotation {
        JigsawRotation::None => (position.x, position.z),
        JigsawRotation::Clockwise90 => (-position.z, position.x),
        JigsawRotation::Clockwise180 => (-position.x, -position.z),
        JigsawRotation::CounterClockwise90 => (position.z, -position.x),
    };
    BlockPos {
        x,
        y: position.y,
        z,
    }
}

fn rotate_direction(
    direction: StructureTemplateDirection,
    rotation: JigsawRotation,
) -> StructureTemplateDirection {
    use StructureTemplateDirection::{Down, East, North, South, Up, West};
    if matches!(direction, Down | Up) {
        return direction;
    }
    match (direction, rotation) {
        (direction, JigsawRotation::None) => direction,
        (North, JigsawRotation::Clockwise90) => East,
        (East, JigsawRotation::Clockwise90) => South,
        (South, JigsawRotation::Clockwise90) => West,
        (West, JigsawRotation::Clockwise90) => North,
        (North, JigsawRotation::Clockwise180) => South,
        (East, JigsawRotation::Clockwise180) => West,
        (South, JigsawRotation::Clockwise180) => North,
        (West, JigsawRotation::Clockwise180) => East,
        (North, JigsawRotation::CounterClockwise90) => West,
        (East, JigsawRotation::CounterClockwise90) => North,
        (South, JigsawRotation::CounterClockwise90) => East,
        (West, JigsawRotation::CounterClockwise90) => South,
        _ => direction,
    }
}

fn opposite(direction: StructureTemplateDirection) -> StructureTemplateDirection {
    use StructureTemplateDirection::{Down, East, North, South, Up, West};
    match direction {
        Down => Up,
        Up => Down,
        North => South,
        South => North,
        West => East,
        East => West,
    }
}

fn direction_step_y(direction: StructureTemplateDirection) -> i32 {
    match direction {
        StructureTemplateDirection::Down => -1,
        StructureTemplateDirection::Up => 1,
        _ => 0,
    }
}

fn relative(position: BlockPos, direction: StructureTemplateDirection) -> BlockPos {
    use StructureTemplateDirection::{Down, East, North, South, Up, West};
    let (x, y, z) = match direction {
        Down => (0, -1, 0),
        Up => (0, 1, 0),
        North => (0, 0, -1),
        South => (0, 0, 1),
        West => (-1, 0, 0),
        East => (1, 0, 0),
    };
    offset(position, x, y, z)
}

fn add(left: BlockPos, right: BlockPos) -> BlockPos {
    offset(left, right.x, right.y, right.z)
}

fn sub(left: BlockPos, right: BlockPos) -> BlockPos {
    offset(left, -right.x, -right.y, -right.z)
}

fn offset(position: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: position.x + x,
        y: position.y + y,
        z: position.z + z,
    }
}

fn midpoint(minimum: i32, maximum: i32) -> i32 {
    minimum.wrapping_add(maximum) / 2
}

#[cfg(test)]
mod tests {
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    struct FlatHeight(i32);

    impl JigsawHeightAccess for FlatHeight {
        fn generation_bounds(&self) -> Option<(i32, i32)> {
            Some((0, 384))
        }

        fn first_free_height(
            &self,
            _x: i32,
            _z: i32,
            _heightmap: StructureHeightmap,
        ) -> Option<i32> {
            Some(self.0)
        }
    }

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn village_graph(startup: &DatapackStartup, depth: i32, chunk: [i32; 2]) -> JigsawPieceGraph {
        village_graph_result(startup, depth, chunk, None).unwrap()
    }

    fn village_graph_result(
        startup: &DatapackStartup,
        depth: i32,
        chunk: [i32; 2],
        start_name: Option<&str>,
    ) -> Option<JigsawPieceGraph> {
        let plan = NoiseStructurePlan::from_startup(startup, &["minecraft:plains"]).unwrap();
        let structure_id = "minecraft:village_plains";
        let mut settings = plan.structures()[structure_id].jigsaw.clone().unwrap();
        settings.size = depth;
        if let Some(start_name) = start_name {
            settings.start_jigsaw_name = Some(start_name.to_string());
        }
        let mut random = large_feature_random(0, chunk[0], chunk[1]);
        let sampled_y =
            sample_height_provider_with_bounds(&settings.start_height, -64, 384, &mut random)
                .unwrap();
        let position = BlockPos {
            x: chunk[0] * 16,
            y: sampled_y,
            z: chunk[1] * 16,
        };
        let aliases = AliasLookup::create(&settings.pool_aliases, 0, position).unwrap();
        build_graph(
            structure_id,
            chunk,
            position,
            &settings,
            plan.pools(),
            &startup.structures,
            -64,
            319,
            &FlatHeight(64),
            aliases,
            &mut random,
        )
    }

    #[test]
    fn missing_single_template_matches_get_or_create_empty() {
        use bbb_server_datapack::structure_pool::ProcessorListHolder;

        let startup = startup();
        let location = "minecraft:ancient_city/walls/intact_horizontal_wall_stairs_5";
        assert!(startup.structures.template(location).is_none());
        let element = StructurePoolElement::Single {
            legacy: false,
            location: location.to_string(),
            processors: ProcessorListHolder::Named("minecraft:empty".to_string()),
            projection: StructurePoolProjection::Rigid,
            liquid_settings: None,
        };
        let origin = BlockPos {
            x: 10,
            y: 20,
            z: 30,
        };
        let bounds =
            element_bounding_box(&element, &startup.structures, origin, JigsawRotation::None)
                .unwrap();
        assert_eq!(
            bounds,
            JigsawBoundingBox {
                min: BlockPos { x: 9, y: 19, z: 29 },
                max: origin,
            }
        );
        assert!(
            element_connectors(&element, &startup.structures, origin, JigsawRotation::None,)
                .is_empty()
        );
    }

    #[test]
    fn builtin_village_graph_is_bounded_deterministic_and_depth_sensitive() {
        let startup = startup();
        let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
        let production_graph = plan
            .jigsaw_piece_graph(
                &startup,
                "minecraft:village_plains",
                0,
                0,
                0,
                -64,
                384,
                &FlatHeight(64),
            )
            .unwrap();
        let planned = plan
            .plan_jigsaw_anchor(
                &startup,
                "minecraft:village_plains",
                0,
                0,
                0,
                -64,
                384,
                &FlatHeight(64),
            )
            .unwrap();
        assert_eq!(
            planned.generation_anchor(),
            production_graph.generation_anchor
        );
        assert_eq!(
            planned.expand(&startup, &FlatHeight(64)).unwrap(),
            production_graph
        );
        assert!(!production_graph.pieces.is_empty());
        let mut relative = startup.clone();
        let definition = relative
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
            .unwrap()
            .elements
            .iter_mut()
            .find(|element| element.id == "minecraft:village_plains")
            .unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&definition.source_json).unwrap();
        json["start_height"] = serde_json::json!({
            "type": "minecraft:weighted_list",
            "distribution": [
                { "data": { "absolute": -32 }, "weight": 0 },
                {
                    "data": {
                        "type": "minecraft:weighted_list",
                        "distribution": [{ "data": { "above_bottom": 12 }, "weight": 1 }]
                    },
                    "weight": 1
                }
            ]
        });
        definition.source_json = serde_json::to_vec(&json).unwrap().into();
        definition.source =
            Some(bbb_registry_codec::decode_direct_registry_json(&definition.source_json).unwrap());
        definition.source_json =
            std::sync::Arc::from(serde_json::to_vec(definition.json().unwrap()).unwrap());
        let plan = NoiseStructurePlan::from_startup(&relative, &["minecraft:plains"]).unwrap();
        let graph = plan
            .jigsaw_piece_graph(
                &relative,
                "minecraft:village_plains",
                0,
                0,
                0,
                -64,
                384,
                &FlatHeight(64),
            )
            .unwrap();
        assert_eq!(graph.generation_anchor.y, 76);
        let depth_zero = village_graph(&startup, 0, [0, 0]);
        let depth_one = village_graph(&startup, 1, [0, 0]);
        let depth_two = village_graph(&startup, 2, [0, 0]);
        assert_eq!(depth_zero.structure_id, "minecraft:village_plains");
        assert_eq!(depth_zero.start_chunk, [0, 0]);
        assert!(depth_zero.pieces.is_empty());
        assert!(depth_zero.junctions.is_empty());
        assert!(!depth_one.pieces.is_empty());
        assert!(depth_two.pieces.len() > depth_one.pieces.len());
        assert!(depth_two.junctions.len() > depth_one.junctions.len());
        assert_eq!(depth_two, village_graph(&startup, 2, [0, 0]));

        let center = &depth_two.pieces[0];
        assert_eq!(depth_two.center_bounding_box, center.bounding_box);
        assert!(center.bounding_box.contains(depth_two.generation_anchor));
        assert_eq!(center.ground_level_delta, 1);
        assert!(!matches!(center.element, StructurePoolElement::Empty));
        assert!(matches!(
            center.rotation,
            JigsawRotation::None
                | JigsawRotation::Clockwise90
                | JigsawRotation::Clockwise180
                | JigsawRotation::CounterClockwise90
        ));
        for junction in &depth_two.junctions {
            assert!(junction.piece_index < depth_two.pieces.len());
            assert!(matches!(
                junction.destination_projection,
                StructurePoolProjection::Rigid | StructurePoolProjection::TerrainMatching
            ));
            let _ordered_payload = (
                junction.source_x,
                junction.source_ground_y,
                junction.source_z,
                junction.delta_y,
            );
        }
    }

    #[test]
    fn negative_chunk_graph_keeps_floor_aligned_origin_and_is_deterministic() {
        let startup = startup();
        let graph = village_graph(&startup, 1, [-3, -5]);
        assert_eq!(graph.start_chunk, [-3, -5]);
        assert_eq!(graph, village_graph(&startup, 1, [-3, -5]));
        assert!(graph.pieces.iter().all(|piece| {
            piece.position.x >= graph.generation_anchor.x - 128
                && piece.position.x <= graph.generation_anchor.x + 128
                && piece.position.z >= graph.generation_anchor.z - 128
                && piece.position.z <= graph.generation_anchor.z + 128
        }));
    }

    #[test]
    fn alias_lookup_resolves_once_in_declaration_order() {
        let bindings = vec![
            PoolAliasBinding::Direct {
                alias: "minecraft:direct_alias".to_string(),
                target: "minecraft:direct_target".to_string(),
            },
            PoolAliasBinding::Random {
                alias: "minecraft:random_alias".to_string(),
                targets: vec![
                    WeightedPool {
                        pool: "minecraft:first".to_string(),
                        weight: 1,
                    },
                    WeightedPool {
                        pool: "minecraft:second".to_string(),
                        weight: 2,
                    },
                ],
            },
        ];
        let position = BlockPos { x: -9, y: 71, z: 4 };
        let first = AliasLookup::create(&bindings, 1234, position).unwrap();
        let second = AliasLookup::create(&bindings, 1234, position).unwrap();
        assert_eq!(
            first.lookup("minecraft:direct_alias"),
            "minecraft:direct_target"
        );
        assert_eq!(
            first.lookup("minecraft:random_alias"),
            second.lookup("minecraft:random_alias")
        );
        assert!(matches!(
            first.lookup("minecraft:random_alias"),
            "minecraft:first" | "minecraft:second"
        ));
        assert_eq!(first.lookup("minecraft:unbound"), "minecraft:unbound");
        assert!(
            AliasLookup::create(&[bindings[0].clone(), bindings[0].clone()], 1234, position,)
                .is_none()
        );
    }

    #[test]
    fn priority_queue_is_highest_first_and_fifo_within_one_priority() {
        let mut queue = PriorityQueue::default();
        for (piece_index, priority) in [(0, 2), (1, 7), (2, 7), (3, -1)] {
            queue.push(
                PieceState {
                    piece_index,
                    context_index: 0,
                    depth: 0,
                },
                priority,
            );
        }
        assert_eq!(queue.pop().unwrap().piece_index, 1);
        assert_eq!(queue.pop().unwrap().piece_index, 2);
        assert_eq!(queue.pop().unwrap().piece_index, 0);
        assert_eq!(queue.pop().unwrap().piece_index, 3);
        assert!(queue.pop().is_none());
    }

    #[test]
    fn rotated_attachment_checks_facing_joint_and_target_name() {
        let connector = |front, top, joint, name: &str, target: &str| RuntimeConnector {
            position: BlockPos { x: 0, y: 0, z: 0 },
            front,
            top,
            joint,
            name: name.to_string(),
            pool: "minecraft:empty".to_string(),
            target: target.to_string(),
            placement_priority: 0,
            selection_priority: 0,
        };
        let source = connector(
            StructureTemplateDirection::East,
            StructureTemplateDirection::Up,
            StructureTemplateJigsawJoint::Aligned,
            "minecraft:source",
            "minecraft:door",
        );
        let target = connector(
            StructureTemplateDirection::West,
            StructureTemplateDirection::Up,
            StructureTemplateJigsawJoint::Rollable,
            "minecraft:door",
            "minecraft:empty",
        );
        assert!(can_attach(&source, &target));
        let wrong_top = connector(
            StructureTemplateDirection::West,
            StructureTemplateDirection::North,
            StructureTemplateJigsawJoint::Rollable,
            "minecraft:door",
            "minecraft:empty",
        );
        assert!(!can_attach(&source, &wrong_top));
    }

    #[test]
    fn connector_rotation_and_stable_priority_sort_preserve_vanilla_order() {
        let rotated = runtime_connector(
            &StructureTemplateJigsaw {
                position: BlockPos { x: 2, y: 1, z: 3 },
                front: StructureTemplateDirection::North,
                top: StructureTemplateDirection::Up,
                joint: StructureTemplateJigsawJoint::Aligned,
                name: "minecraft:name".to_string(),
                pool: "minecraft:pool".to_string(),
                target: "minecraft:target".to_string(),
                placement_priority: 4,
                selection_priority: 5,
            },
            BlockPos {
                x: 10,
                y: 20,
                z: 30,
            },
            JigsawRotation::Clockwise90,
        );
        assert_eq!(rotated.position, BlockPos { x: 7, y: 21, z: 32 });
        assert_eq!(rotated.front, StructureTemplateDirection::East);
        assert_eq!(rotated.top, StructureTemplateDirection::Up);

        let connector = |id: i32, priority: i32| RuntimeConnector {
            position: BlockPos { x: id, y: 0, z: 0 },
            front: StructureTemplateDirection::North,
            top: StructureTemplateDirection::Up,
            joint: StructureTemplateJigsawJoint::Aligned,
            name: "minecraft:name".to_string(),
            pool: "minecraft:pool".to_string(),
            target: "minecraft:target".to_string(),
            placement_priority: 0,
            selection_priority: priority,
        };
        let original = vec![
            connector(0, 2),
            connector(1, 7),
            connector(2, 7),
            connector(3, -1),
            connector(4, 2),
        ];
        let mut shuffled = original.clone();
        util_shuffle(&mut shuffled, &mut WorldgenRandom::with_seed(99));
        let expected_sevens = shuffled
            .iter()
            .filter(|connector| connector.selection_priority == 7)
            .map(|connector| connector.position.x)
            .collect::<Vec<_>>();
        let expected_twos = shuffled
            .iter()
            .filter(|connector| connector.selection_priority == 2)
            .map(|connector| connector.position.x)
            .collect::<Vec<_>>();
        let mut actual = original;
        shuffle_and_sort_connectors(&mut actual, &mut WorldgenRandom::with_seed(99));
        assert_eq!(
            actual
                .iter()
                .map(|connector| connector.selection_priority)
                .collect::<Vec<_>>(),
            vec![7, 7, 2, 2, -1]
        );
        assert_eq!(
            actual
                .iter()
                .filter(|connector| connector.selection_priority == 7)
                .map(|connector| connector.position.x)
                .collect::<Vec<_>>(),
            expected_sevens
        );
        assert_eq!(
            actual
                .iter()
                .filter(|connector| connector.selection_priority == 2)
                .map(|connector| connector.position.x)
                .collect::<Vec<_>>(),
            expected_twos
        );
    }

    #[test]
    fn free_context_allows_touching_faces_but_refuses_overlap_and_escape() {
        let boundary = JigsawBoundingBox {
            min: BlockPos { x: 0, y: 0, z: 0 },
            max: BlockPos { x: 9, y: 9, z: 9 },
        };
        let occupied = JigsawBoundingBox {
            min: BlockPos { x: 1, y: 1, z: 1 },
            max: BlockPos { x: 3, y: 3, z: 3 },
        };
        let context = FreeContext {
            boundary,
            occupied: vec![occupied],
        };
        assert!(context.accepts(JigsawBoundingBox {
            min: BlockPos { x: 4, y: 1, z: 1 },
            max: BlockPos { x: 5, y: 3, z: 3 },
        }));
        assert!(!context.accepts(JigsawBoundingBox {
            min: BlockPos { x: 3, y: 2, z: 2 },
            max: BlockPos { x: 4, y: 4, z: 4 },
        }));
        assert!(!context.accepts(JigsawBoundingBox {
            min: BlockPos { x: -1, y: 0, z: 0 },
            max: BlockPos { x: 0, y: 0, z: 0 },
        }));
    }
}
