use bbb_protocol::packets::BlockPos;
use bbb_server_types::save::{SaveError, SaveResult};
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
use bbb_server_worldgen::noise::structures::{
    CommonPlacement, NoiseStructurePlacement, NoiseStructurePlan,
};

use crate::chunk::noise_generator::NoiseFinalDensityProgram;
use crate::chunk::{ChunkSource, ChunkStore};

enum StructureLocatorProgram {
    Noise(std::sync::Arc<NoiseFinalDensityProgram>),
    Ordinary(std::sync::Arc<crate::chunk::structure_generation::OrdinaryStructureProgram>),
}

impl StructureLocatorProgram {
    fn plan(&self) -> Option<&NoiseStructurePlan> {
        match self {
            Self::Noise(program) => program.natural_spawn_structure_plan(),
            Self::Ordinary(program) => Some(program.plan()),
        }
    }

    fn world_seed(&self) -> i64 {
        match self {
            Self::Noise(program) => program.structure_world_seed(),
            Self::Ordinary(program) => program.world_seed(),
        }
    }

    fn ring_positions(&self, set: &str) -> SaveResult<Vec<(i32, i32)>> {
        match self {
            Self::Noise(program) => program.structure_ring_positions_component(set),
            Self::Ordinary(program) => program
                .ordered_ring_positions(set)
                .map(<[_]>::to_vec)
                .ok_or_else(|| unavailable("structure ring positions are absent")),
        }
    }

    fn compute_starts_ephemeral(
        &self,
        store: &mut ChunkStore,
        chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureChunkData> {
        match self {
            Self::Noise(program) => store
                .structure_status
                .compute_starts_chunk_ephemeral(program, chunk[0], chunk[1]),
            Self::Ordinary(program) => program.compute_starts_ephemeral(
                &store.source,
                chunk[0],
                chunk[1],
                store.structure_status.generate_structures(),
            ),
        }
    }
}

struct PlacementGroup<'a> {
    set_id: &'a str,
    placement: &'a NoiseStructurePlacement,
    wanted: Vec<&'a str>,
}

enum ProbedStart {
    Existing {
        id: String,
    },
    Stored {
        id: String,
    },
    Generated {
        id: String,
        data: Box<NoiseStructureChunkData>,
    },
}

struct CandidateHit {
    pos: BlockPos,
    chunk: [i32; 2],
    start: ProbedStart,
}

impl ChunkStore {
    pub fn find_nearest_map_structure(
        &mut self,
        origin: BlockPos,
        allowed_structure_ids: &[String],
        max_search_radius: i32,
        skip_known: bool,
    ) -> SaveResult<Option<(BlockPos, String)>> {
        let program = if let ChunkSource::NoiseCarvers(program) = &self.source {
            StructureLocatorProgram::Noise(std::sync::Arc::clone(program))
        } else if let Some(program) = &self.ordinary_structures {
            StructureLocatorProgram::Ordinary(std::sync::Arc::clone(program))
        } else {
            return Err(unavailable(
                "structure placement is unavailable for this generator",
            ));
        };
        let Some(plan) = program.plan() else {
            return Err(unavailable(
                "structure placement requires a frozen structure plan",
            ));
        };
        let mut groups: Vec<PlacementGroup<'_>> = Vec::new();
        for wanted in allowed_structure_ids {
            let Some(definition) = plan.structures().get(wanted) else {
                continue;
            };
            if definition.biomes.is_disjoint(plan.possible_biomes()) {
                continue;
            }
            for set in plan.sets() {
                if !set
                    .structures
                    .iter()
                    .any(|entry| entry.structure == *wanted)
                {
                    continue;
                }
                // Distinct sets keep their own identity even with equal placement values.
                if let Some(group) = groups.iter_mut().find(|group| group.set_id == set.id) {
                    if !group.wanted.contains(&wanted.as_str()) {
                        group.wanted.push(wanted);
                    }
                } else {
                    groups.push(PlacementGroup {
                        set_id: &set.id,
                        placement: &set.placement,
                        wanted: vec![wanted],
                    });
                }
            }
        }
        let mut nearest: Option<CandidateHit> = None;
        for group in &groups {
            let NoiseStructurePlacement::ConcentricRings { common, .. } = group.placement else {
                continue;
            };
            let mut ring_nearest: Option<CandidateHit> = None;
            let mut ring_distance = f64::INFINITY;
            // Membership caches sort chunks; locating must retain the generated ring order.
            for (x, z) in program.ring_positions(group.set_id)? {
                let center = BlockPos {
                    x: x.wrapping_mul(16).wrapping_add(8),
                    y: 32,
                    z: z.wrapping_mul(16).wrapping_add(8),
                };
                let distance = distance_squared(origin, center);
                if distance < ring_distance {
                    if let Some(start) =
                        probe_start(self, &program, [x, z], &group.wanted, skip_known)?
                    {
                        ring_nearest = Some(CandidateHit {
                            pos: locate_position(common, [x, z]),
                            chunk: [x, z],
                            start,
                        });
                        ring_distance = distance;
                    }
                }
            }
            if let Some(found) = ring_nearest {
                keep_nearest(origin, &mut nearest, found);
            }
        }
        let origin_chunk = [origin.x >> 4, origin.z >> 4];
        for radius in 0..=max_search_radius {
            let mut found_in_shell = false;
            for group in &groups {
                let NoiseStructurePlacement::RandomSpread {
                    common, spacing, ..
                } = group.placement
                else {
                    continue;
                };
                let mut group_found: Option<CandidateHit> = None;
                'shell: for x_offset in -radius..=radius {
                    for z_offset in -radius..=radius {
                        if x_offset != -radius
                            && x_offset != radius
                            && z_offset != -radius
                            && z_offset != radius
                        {
                            continue;
                        }
                        let x = origin_chunk[0].wrapping_add(spacing.wrapping_mul(x_offset));
                        let z = origin_chunk[1].wrapping_add(spacing.wrapping_mul(z_offset));
                        let (x, z) = group
                            .placement
                            .random_spread_candidate(program.world_seed(), x, z)
                            .expect("random spread group has a candidate");
                        if let Some(start) =
                            probe_start(self, &program, [x, z], &group.wanted, skip_known)?
                        {
                            group_found = Some(CandidateHit {
                                pos: locate_position(common, [x, z]),
                                chunk: [x, z],
                                start,
                            });
                            break 'shell;
                        }
                    }
                }
                if let Some(found) = group_found {
                    found_in_shell = true;
                    keep_nearest(origin, &mut nearest, found);
                }
            }
            if found_in_shell {
                break;
            }
        }
        commit_nearest_hit(self, nearest, skip_known)
    }
}

fn match_wanted(
    data: &NoiseStructureChunkData,
    wanted: &[&str],
    skip_known: bool,
) -> Option<String> {
    wanted.iter().find_map(|&id| {
        (data.has_valid_start(id) && (!skip_known || data.can_add_reference(id)))
            .then(|| id.to_string())
    })
}

fn probe_start(
    store: &mut ChunkStore,
    program: &StructureLocatorProgram,
    chunk: [i32; 2],
    wanted: &[&str],
    skip_known: bool,
) -> SaveResult<Option<ProbedStart>> {
    if let Some(data) = store.structure_status.component(chunk[0], chunk[1]) {
        if let Some(id) = match_wanted(data, wanted, skip_known) {
            return Ok(Some(ProbedStart::Existing { id }));
        }
        if store.structure_status.has_ready_starts(chunk) {
            return Ok(None);
        }
    }
    let key = (chunk[0], chunk[1]);
    if store.is_stored_readable(key) {
        if let Some((status, component)) =
            store.stored_structure_component_with_status(chunk[0], chunk[1])
        {
            if let Some(data) = component {
                if let Some(id) = match_wanted(&data, wanted, skip_known) {
                    return Ok(Some(ProbedStart::Stored { id }));
                }
            }
            if status >= bbb_server_persistence::Java26ChunkStatus::StructureStarts {
                return Ok(None);
            }
        }
        store.prepare_generation_admission(key)?;
        if let Some(data) = store.structure_status.component(chunk[0], chunk[1]) {
            if let Some(id) = match_wanted(data, wanted, skip_known) {
                return Ok(Some(ProbedStart::Existing { id }));
            }
            if store.structure_status.has_ready_starts(chunk) {
                return Ok(None);
            }
        }
    } else if store.cold_columns.contains_key(&key) {
        store.prepare_generation_admission(key)?;
        if let Some(data) = store.structure_status.component(chunk[0], chunk[1]) {
            if let Some(id) = match_wanted(data, wanted, skip_known) {
                return Ok(Some(ProbedStart::Existing { id }));
            }
            if store.structure_status.has_ready_starts(chunk) {
                return Ok(None);
            }
        }
    }
    let data = program.compute_starts_ephemeral(store, chunk)?;
    Ok(
        match_wanted(&data, wanted, skip_known).map(|id| ProbedStart::Generated {
            id,
            data: Box::new(data),
        }),
    )
}

fn commit_nearest_hit(
    store: &mut ChunkStore,
    nearest: Option<CandidateHit>,
    skip_known: bool,
) -> SaveResult<Option<(BlockPos, String)>> {
    let Some(hit) = nearest else {
        return Ok(None);
    };
    let id = match hit.start {
        ProbedStart::Existing { id } => {
            store.publish_structure_starts_column(hit.chunk)?;
            id
        }
        ProbedStart::Stored { id } => {
            store.prepare_generation_admission((hit.chunk[0], hit.chunk[1]))?;
            store.publish_structure_starts_column(hit.chunk)?;
            id
        }
        ProbedStart::Generated { id, data } => {
            store.prepare_generation_admission((hit.chunk[0], hit.chunk[1]))?;
            store.structure_status.publish_generated_starts(*data);
            store.publish_structure_starts_column(hit.chunk)?;
            id
        }
    };
    if skip_known && !store.add_map_structure_reference(hit.chunk, &id)? {
        return Ok(None);
    }
    Ok(Some((hit.pos, id)))
}

impl ChunkStore {
    // ChunkGenerator.java:278-311: each selected candidate consumes its own start reference.
    fn add_map_structure_reference(&mut self, chunk: [i32; 2], id: &str) -> SaveResult<bool> {
        let key = (chunk[0], chunk[1]);
        let durable = self.is_durable(key);
        if durable && self.next_whole_rewrite_epoch == u64::MAX {
            return Err(unavailable("structure-reference rewrite epoch overflows"));
        }
        if !self.structure_status.try_add_start_reference(chunk, id)? {
            return Ok(false);
        }
        if durable {
            self.mark_whole_rewrite_pending(key)?;
        } else {
            self.loaded_mut(key).pending_initial_persistence = true;
        }
        Ok(true)
    }
}

fn locate_position(common: &CommonPlacement, chunk: [i32; 2]) -> BlockPos {
    BlockPos {
        x: chunk[0]
            .wrapping_mul(16)
            .wrapping_add(common.locate_offset[0]),
        y: common.locate_offset[1],
        z: chunk[1]
            .wrapping_mul(16)
            .wrapping_add(common.locate_offset[2]),
    }
}

fn distance_squared(a: BlockPos, b: BlockPos) -> f64 {
    let x = f64::from(a.x) - f64::from(b.x);
    let y = f64::from(a.y) - f64::from(b.y);
    let z = f64::from(a.z) - f64::from(b.z);
    x * x + y * y + z * z
}

fn keep_nearest(origin: BlockPos, nearest: &mut Option<CandidateHit>, found: CandidateHit) {
    if nearest.as_ref().is_none_or(|current| {
        distance_squared(origin, found.pos) < distance_squared(origin, current.pos)
    }) {
        *nearest = Some(found);
    }
}

fn unavailable(detail: &str) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: detail.to_string(),
    }
}
