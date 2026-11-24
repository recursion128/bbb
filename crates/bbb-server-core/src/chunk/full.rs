//! SPAWN → FULL promotion for storage-only noise columns.
//!
//! `ChunkStatus.FULL` (`ChunkStatus.java:32`) changes the chunk type to
//! `LEVELCHUNK`. `ChunkStatusTasks.full` (`:176-204`) builds a `LevelChunk`
//! from the ProtoChunk, runs `postLoadProtoChunk` (`:206-210`) to publish
//! ProtoChunk entities, then registers tick containers and block entities.
//! The chunk store keeps that publication in a disjoint handoff owner until
//! the live entity store consumes it. Tick containers already live on the
//! column. `LOADING_PYRAMID` (`ChunkPyramid.java:53`) still runs `full`.

use super::spawn::OriginalMobPopulation;
use super::{ChunkSource, ChunkStore};
use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::chunk_data::spawn::ChunkSpawnState;
use bbb_server_types::chunk_data::{ChunkGenerationState, ChunkLightState};
use bbb_server_types::save::{SaveError, SaveResult};
use std::collections::{BTreeMap, BTreeSet};

use bbb_server_types::chunk_data::full::ChunkNoiseFullState;

#[derive(Clone, Copy)]
enum BatchPostLightStage {
    Light,
    Spawn {
        disable_mob_generation: bool,
        original_mobs_refused: bool,
    },
    PreviouslyFull {
        disable_mob_generation: bool,
        original_mobs_refused: bool,
    },
}

impl ChunkStore {
    pub fn validate_served_full_column(&self, cx: i32, cz: i32) -> SaveResult<()> {
        if !self.is_ticking_full(cx, cz) {
            return Err(invalid_full(
                (cx, cz),
                "column still requires FULL scheduling",
            ));
        }
        if self.is_ordinary_source() {
            self.validate_ordinary_light((cx, cz))
        } else {
            self.validate_servable_noise_full_center((cx, cz))
        }
    }

    /// Ensures one built-in noise center reaches FULL while reusing proven
    /// overlapping status work. `ChunkGenerationTask.java:35-70`.
    /// Layer scheduling and pass-through live at `ChunkGenerationTask.java:84-149`.
    pub fn try_ensure_noise_full_column(
        &mut self,
        cx: i32,
        cz: i32,
    ) -> SaveResult<&super::ChunkColumn> {
        let center = (cx, cz);
        self.require_legacy_generation_work_completed(center)?;
        if !matches!(self.source, ChunkSource::NoiseCarvers(_)) {
            return Err(invalid_full(
                center,
                "noise FULL requires a noise CARVERS store",
            ));
        }
        if self.is_storage_backed_noise_full(center.0, center.1)
            && self
                .generation(center)
                .and_then(post_light_state)
                .is_none_or(ChunkLightState::is_light_correct)
        {
            self.validate_servable_noise_full_center(center)?;
            return self.column_at(center).ok_or_else(|| {
                invalid_full(
                    center,
                    "the storage-backed FULL status has no loaded column",
                )
            });
        }
        if self.has_whole_rewrite(center) {
            self.validate_pending_whole_rewrite_center(center)?;
            if matches!(
                self.generation(center),
                Some(ChunkGenerationState::NoiseFull(_))
            ) {
                return self.column_at(center).ok_or_else(|| {
                    invalid_full(center, "the pending FULL rewrite has no loaded column")
                });
            }
        }
        if let Some(light) = self.generation(center).and_then(post_light_state) {
            if light.is_light_correct() {
                if self.is_durable(center) {
                    self.validate_durable_post_light_center(center, light, "durable")?;
                } else {
                    self.validate_current_noise_light(center, light)?;
                }
            }
        }
        if matches!(
            self.generation(center),
            Some(ChunkGenerationState::NoiseFull(full)) if full.light().is_light_correct()
        ) {
            self.validate_fresh_noise_full_center(center)?;
            return self.column_at(center).ok_or_else(|| {
                invalid_full(center, "the completed FULL status has no loaded column")
            });
        }
        self.try_ensure_noise_full_columns(&[center])?;
        Ok(self
            .column_at(center)
            .expect("successful FULL scheduling publishes its center"))
    }

    /// Ensures a delivery batch reaches FULL, with its FEATURES
    /// write halo complete before any requested center crosses LIGHT.
    /// `ChunkPyramid.java:30-39`; `ChunkGenerationTask.java:115-133`.
    pub fn try_ensure_noise_full_columns(&mut self, centers: &[(i32, i32)]) -> SaveResult<()> {
        self.ensure_noise_full_columns_populated(centers, None)
    }

    pub(super) fn ensure_noise_full_columns_populated(
        &mut self,
        centers: &[(i32, i32)],
        mut population: Option<&mut OriginalMobPopulation<'_>>,
    ) -> SaveResult<()> {
        if centers.is_empty() {
            return Ok(());
        }
        if !matches!(self.source, ChunkSource::NoiseCarvers(_)) {
            return Err(invalid_full(
                centers[0],
                "noise FULL batch requires a noise CARVERS store",
            ));
        }

        let mut unique = Vec::with_capacity(centers.len());
        let mut seen = BTreeSet::new();
        for &center in centers {
            if seen.insert(center) {
                self.require_legacy_generation_work_completed(center)?;
                unique.push(center);
            }
        }

        let mut schedulers = Vec::new();
        let mut durable_schedulers = BTreeSet::new();
        let mut unlit_restored = BTreeSet::new();
        for &center in &unique {
            let needs_relight = self
                .generation(center)
                .and_then(post_light_state)
                .is_some_and(|light| !light.is_light_correct());
            if needs_relight && self.is_durable(center) {
                unlit_restored.insert(center);
                durable_schedulers.insert(center);
                schedulers.push(center);
            } else if self.is_storage_backed_noise_full(center.0, center.1) {
                self.validate_servable_noise_full_center(center)?;
            } else if self.has_whole_rewrite(center) {
                self.validate_pending_whole_rewrite_center(center)?;
                if matches!(
                    self.generation(center),
                    Some(ChunkGenerationState::NoiseFull(_))
                ) {
                    continue;
                }
                durable_schedulers.insert(center);
                schedulers.push(center);
            } else if matches!(
                self.generation(center),
                Some(ChunkGenerationState::NoiseFull(_))
            ) {
                self.validate_fresh_noise_full_center(center)?;
            } else {
                if self.is_storage_backed_generation(center) {
                    durable_schedulers.insert(center);
                }
                schedulers.push(center);
            }
        }
        if schedulers.is_empty() {
            return Ok(());
        }
        self.pregenerate(&schedulers, &[])?;

        let mut affected = unlit_restored;
        for &scheduler in &schedulers {
            affected.extend(self.feature_relight_centers(scheduler, None)?);
        }
        let mut final_stages = self.demote_post_light(&affected)?;
        self.ensure_initialize_light_windows_for_relight(&schedulers)?;
        for &center in &unique {
            if matches!(
                self.generation(center),
                Some(ChunkGenerationState::InitializeLight(_))
            ) {
                final_stages
                    .entry(center)
                    .or_insert(BatchPostLightStage::Light);
            }
        }
        self.finish_generation_relight(&final_stages)?;
        for &center in &unique {
            if matches!(
                self.generation(center),
                Some(ChunkGenerationState::Light(_))
            ) {
                match population.as_mut() {
                    Some(population) => {
                        self.spawn_chunk_with_population(center, Some(&mut **population))?
                    }
                    None => self.spawn_chunk(center)?,
                }
            }
            if matches!(
                self.generation(center),
                Some(ChunkGenerationState::Spawn(_))
            ) {
                self.full_noise_chunk(center)?;
            }
            if durable_schedulers.contains(&center) {
                self.mark_whole_rewrite_pending(center)?;
            }
            self.validate_servable_noise_full_center(center)?;
        }
        Ok(())
    }

    pub(super) fn ensure_loading_features_with_relight(
        &mut self,
        targets: &BTreeSet<(i32, i32)>,
    ) -> SaveResult<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let mut remaining = targets.clone();
        let mut affected = BTreeSet::new();
        while let Some(&center) = remaining.first() {
            let selected = remaining
                .iter()
                .copied()
                .filter(|chunk| chunk.0.abs_diff(center.0) <= 1 && chunk.1.abs_diff(center.1) <= 1)
                .collect::<BTreeSet<_>>();
            affected.extend(self.feature_relight_centers(center, Some(&selected))?);
            remaining.retain(|chunk| !selected.contains(chunk));
        }
        let stages = self.demote_post_light(&affected)?;
        self.generate_loading_features_batch(targets)?;
        self.finish_generation_relight(&stages)?;
        Ok(())
    }

    fn finish_generation_relight(
        &mut self,
        stages: &BTreeMap<(i32, i32), BatchPostLightStage>,
    ) -> SaveResult<()> {
        let changes = std::mem::take(&mut self.pending_light_changes);
        self.light_storage.take_dirty();
        self.initialize_light_windows(&stages.keys().copied().collect::<Vec<_>>())?;
        self.relight_changed_blocks(&changes)?;
        self.rebuild_batch_post_light(stages)?;
        let dirty = self.light_storage.take_dirty();
        for (&chunk, (sky, block)) in &dirty {
            if let Some(loaded) = self.chunks.get_mut(&chunk) {
                loaded.cached_packet.take();
            }
            if ((self.has_skylight && !sky.is_empty()) || !block.is_empty())
                && self.is_durable(chunk)
            {
                self.mark_whole_rewrite_pending(chunk)?;
            }
        }
        let visible = dirty
            .into_iter()
            .filter(|(chunk, _)| {
                matches!(
                    self.generation(*chunk),
                    Some(ChunkGenerationState::NoiseFull(_) | ChunkGenerationState::Full { .. })
                )
            })
            .collect();
        self.record_light_dirt(visible);
        Ok(())
    }

    fn demote_post_light(
        &mut self,
        affected_centers: &BTreeSet<(i32, i32)>,
    ) -> SaveResult<BTreeMap<(i32, i32), BatchPostLightStage>> {
        let affected = affected_centers
            .iter()
            .filter_map(|&center| {
                let stage = match self.generation(center)? {
                    ChunkGenerationState::Light(light)
                        if light.is_light_correct() || self.is_durable(center) =>
                    {
                        BatchPostLightStage::Light
                    }
                    ChunkGenerationState::Light(_) => {
                        return Some(Err(invalid_full(
                            center,
                            "fresh scheduling cannot extend beside incorrect LIGHT",
                        )));
                    }
                    ChunkGenerationState::Spawn(spawn) => BatchPostLightStage::Spawn {
                        disable_mob_generation: spawn.disable_mob_generation(),
                        original_mobs_refused: spawn.original_mobs_refused(),
                    },
                    ChunkGenerationState::NoiseFull(full) => BatchPostLightStage::PreviouslyFull {
                        disable_mob_generation: full.spawn().disable_mob_generation(),
                        original_mobs_refused: full.spawn().original_mobs_refused(),
                    },
                    _ => return None,
                };
                Some(Ok((center, stage)))
            })
            .collect::<SaveResult<BTreeMap<_, _>>>()?;

        for (&center, stage) in &affected {
            if matches!(stage, BatchPostLightStage::PreviouslyFull { .. })
                && !self.proto_entity_records(center.0, center.1).is_empty()
            {
                return Err(invalid_full(
                    center,
                    "existing FULL status retains pending ProtoChunk entities",
                ));
            }
            let light = post_light_state(
                self.generation(center)
                    .expect("the affected post-LIGHT owner remains present"),
            )
            .expect("the affected owner is post-LIGHT");
            if light.is_light_correct() {
                if self.is_durable(center) {
                    self.validate_durable_post_light_center(center, light, "durable")?;
                } else {
                    self.validate_current_noise_light(center, light)?;
                }
            }
        }
        for (&center, stage) in &affected {
            if matches!(stage, BatchPostLightStage::PreviouslyFull { .. }) {
                self.loaded_mut(center).relighting_full = true;
            }
            let initialized = post_light_state(
                self.generation(center)
                    .expect("the affected post-LIGHT owner remains present"),
            )
            .expect("the affected owner is post-LIGHT")
            .initialized()
            .clone();
            self.set_generation(center, ChunkGenerationState::InitializeLight(initialized));
        }
        Ok(affected)
    }

    fn rebuild_batch_post_light(
        &mut self,
        stages: &BTreeMap<(i32, i32), BatchPostLightStage>,
    ) -> SaveResult<()> {
        self.initialize_light_windows(&stages.keys().copied().collect::<Vec<_>>())?;
        for (&center, &stage) in stages {
            self.light_chunk(center)?;
            let ChunkGenerationState::Light(light) = self.state(center) else {
                unreachable!("LIGHT was published immediately above")
            };
            let light = light.clone();
            match stage {
                BatchPostLightStage::Light => {}
                BatchPostLightStage::Spawn {
                    disable_mob_generation,
                    original_mobs_refused,
                } => {
                    self.set_generation(
                        center,
                        ChunkGenerationState::Spawn(ChunkSpawnState::from_light(
                            light,
                            disable_mob_generation,
                            original_mobs_refused,
                        )),
                    );
                }
                BatchPostLightStage::PreviouslyFull {
                    disable_mob_generation,
                    original_mobs_refused,
                } => {
                    let spawn = ChunkSpawnState::from_light(
                        light,
                        disable_mob_generation,
                        original_mobs_refused,
                    );
                    self.set_generation(center, ChunkGenerationState::Spawn(spawn));
                    self.restore_demoted_full_noise_chunk(center)?;
                }
            }
            let completed = match self.generation(center) {
                Some(ChunkGenerationState::Light(_)) => Java26ChunkStatus::Light,
                Some(ChunkGenerationState::Spawn(_)) => Java26ChunkStatus::Spawn,
                Some(ChunkGenerationState::NoiseFull(_)) => Java26ChunkStatus::Full,
                _ => continue,
            };
            self.advance_pending_retrogen_status(center, completed);
        }
        Ok(())
    }

    pub(super) fn validate_servable_noise_full_center(&self, center: (i32, i32)) -> SaveResult<()> {
        if self.is_storage_backed_noise_full(center.0, center.1) {
            self.validate_storage_backed_noise_full_center(center)
        } else if self.has_whole_rewrite(center) {
            self.validate_pending_whole_rewrite_center(center)
        } else {
            self.validate_fresh_noise_full_center(center)
        }
    }

    pub(super) fn validate_fresh_noise_full_center(&self, center: (i32, i32)) -> SaveResult<()> {
        let Some(ChunkGenerationState::NoiseFull(full)) = self.generation(center) else {
            return Err(invalid_full(center, "the scheduler did not reach FULL"));
        };
        if !full.light().is_light_correct() {
            return Err(invalid_full(
                center,
                "the existing FULL status requires correct light",
            ));
        }
        self.validate_current_noise_light(center, full.light())?;
        if !self.proto_entity_records(center.0, center.1).is_empty() {
            return Err(invalid_full(
                center,
                "service refuses nonempty ProtoChunk entities",
            ));
        }
        if !self.chunks.contains_key(&center) {
            return Err(invalid_full(
                center,
                "the completed FULL status has no loaded column",
            ));
        }
        Ok(())
    }

    fn validate_storage_backed_noise_full_center(&self, center: (i32, i32)) -> SaveResult<()> {
        if !self.is_storage_backed_noise_full(center.0, center.1) {
            return Err(invalid_full(
                center,
                "the existing FULL status has no storage-backed provenance",
            ));
        }
        self.validate_durable_noise_full_center(center, "storage-backed")
    }

    pub(super) fn validate_pending_whole_rewrite_center(
        &self,
        center: (i32, i32),
    ) -> SaveResult<()> {
        if !self.has_whole_rewrite(center) {
            return Err(invalid_full(
                center,
                "the existing FULL status has no pending whole-rewrite provenance",
            ));
        }
        if self.pre_carvers(center).is_some() {
            if self.chunks.contains_key(&center) {
                return Ok(());
            }
            return Err(invalid_full(
                center,
                "whole-rewrite proto generation has no loaded column",
            ));
        }
        let state = self.generation(center).ok_or_else(|| {
            invalid_full(
                center,
                "whole-rewrite pending generation has no typed status",
            )
        })?;
        if matches!(state, ChunkGenerationState::NoiseFull(_)) {
            return self.validate_durable_noise_full_center(center, "whole-rewrite pending");
        }
        if matches!(state, ChunkGenerationState::Full { .. }) {
            return self.validate_ordinary_light(center);
        }
        if !self.chunks.contains_key(&center) {
            return Err(invalid_full(
                center,
                "whole-rewrite pending generation has no loaded column",
            ));
        }
        Ok(())
    }

    fn validate_durable_noise_full_center(
        &self,
        center: (i32, i32),
        provenance: &str,
    ) -> SaveResult<()> {
        let Some(ChunkGenerationState::NoiseFull(full)) = self.generation(center) else {
            return Err(invalid_full(
                center,
                format!("{provenance} service requires typed noise FULL"),
            ));
        };
        self.validate_durable_post_light_center(center, full.light(), provenance)?;
        if !self.proto_entity_records(center.0, center.1).is_empty() {
            return Err(invalid_full(
                center,
                format!("{provenance} FULL has ProtoChunk entities"),
            ));
        }
        Ok(())
    }

    fn validate_durable_post_light_center(
        &self,
        center: (i32, i32),
        light: &ChunkLightState,
        provenance: &str,
    ) -> SaveResult<()> {
        if !light.is_light_correct() {
            return Err(invalid_full(
                center,
                format!("{provenance} service requires correct light"),
            ));
        }
        if !self.chunks.contains_key(&center) {
            return Err(invalid_full(
                center,
                format!("{provenance} generation has no loaded column"),
            ));
        }
        if self.light_storage.top_section_y(center).is_none()
            || self.light_storage.bottom_section_y().is_none()
        {
            return Err(invalid_full(
                center,
                format!("{provenance} generation has no restored light payload"),
            ));
        }
        Ok(())
    }

    pub(super) fn validate_current_noise_light(
        &self,
        center: (i32, i32),
        light: &ChunkLightState,
    ) -> SaveResult<()> {
        if self.chunks.contains_key(&center) && light.is_light_correct() {
            return Ok(());
        }
        Err(invalid_full(center, "column requires current light"))
    }

    /// Generates one absent noise center through the complete owned status
    /// chain.
    /// `ChunkPyramid.java:10-39`; `ChunkStatus.java:21-32`.
    pub fn try_generate_fresh_noise_full_column(
        &mut self,
        cx: i32,
        cz: i32,
    ) -> SaveResult<&super::ChunkColumn> {
        let center = (cx, cz);
        if self.is_loaded(cx, cz) || self.generation_state(cx, cz).is_some() {
            return Err(invalid_full(
                center,
                "fresh FULL generation requires an absent target",
            ));
        }
        self.generate_fresh_initialize_light_window(center)?;
        self.light_chunk(center)?;
        self.spawn_chunk(center)?;
        self.full_noise_chunk(center)?;
        Ok(self
            .column(cx, cz)
            .expect("successful fresh FULL generation publishes its target"))
    }

    /// Advances one SPAWN center to noise FULL.
    /// `ChunkStatusTasks.java:176-204`; `ChunkPyramid.java:39,53`.
    pub fn full_noise_chunk(&mut self, center: (i32, i32)) -> SaveResult<()> {
        self.advance_full_noise_chunk(center)
    }

    pub fn advance_full_noise_chunk(&mut self, center: (i32, i32)) -> SaveResult<()> {
        let full = self.noise_full_state_from_spawn(center)?;
        self.publish_proto_entities_at_full(center)?;
        self.set_generation(center, ChunkGenerationState::NoiseFull(full));
        self.advance_pending_retrogen_status(center, Java26ChunkStatus::Full);
        Ok(())
    }

    pub(crate) fn publish_proto_entities_at_full(&mut self, center: (i32, i32)) -> SaveResult<()> {
        if !self
            .published_worldgen_entity_records(center.0, center.1)
            .is_empty()
        {
            return Err(invalid_full(
                center,
                "FULL worldgen-entity publication destination is not pristine",
            ));
        }
        self.update_chunk(center, |chunk| {
            chunk.published_worldgen_entities = std::mem::take(&mut chunk.proto_entities);
        });
        Ok(())
    }

    fn noise_full_state_from_spawn(&self, center: (i32, i32)) -> SaveResult<ChunkNoiseFullState> {
        let Some(state) = self.generation(center) else {
            return Err(invalid_full(center, "the center has no generation state"));
        };
        let ChunkGenerationState::Spawn(spawn) = state else {
            return Err(invalid_full(center, "FULL requires a SPAWN center"));
        };
        if !matches!(self.source, ChunkSource::NoiseCarvers(_)) {
            return Err(invalid_full(center, "noise FULL is not a flat FULL column"));
        }
        Ok(ChunkNoiseFullState::from_spawn(spawn.clone()))
    }

    fn restore_demoted_full_noise_chunk(&mut self, center: (i32, i32)) -> SaveResult<()> {
        let full = self.noise_full_state_from_spawn(center)?;
        // ChunkStatusTasks.java:185-197; ImposterProtoChunk.java:75-78.
        let loaded = self.loaded_mut(center);
        loaded.proto_entities.clear();
        loaded.generation = Some(ChunkGenerationState::NoiseFull(full));
        loaded.relighting_full = false;
        self.advance_pending_retrogen_status(center, Java26ChunkStatus::Full);
        Ok(())
    }
}

fn post_light_state(state: &ChunkGenerationState) -> Option<&ChunkLightState> {
    match state {
        ChunkGenerationState::Light(light) => Some(light),
        ChunkGenerationState::Spawn(spawn) => Some(spawn.light()),
        ChunkGenerationState::NoiseFull(full) => Some(full.light()),
        _ => None,
    }
}

pub fn invalid_full(center: (i32, i32), detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("FULL [{},{}]: {}", center.0, center.1, detail.into()),
    }
}
