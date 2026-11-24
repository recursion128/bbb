use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainChunkMeshOwner {
    pub level_generation: u64,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainSectionMeshOwner {
    pub chunk: TerrainChunkMeshOwner,
    pub section_y: i32,
}

/// Terrain residency keys are trusted packed coordinates, mirroring
/// Vanilla's primitive-long fastutil maps. Avoid SipHash on every visible
/// section while retaining collision-safe HashMap/HashSet equality semantics.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TerrainMeshOwnerBuildHasher;

#[derive(Debug, Clone, Copy)]
pub(super) struct TerrainMeshOwnerHasher(u64);

impl BuildHasher for TerrainMeshOwnerBuildHasher {
    type Hasher = TerrainMeshOwnerHasher;

    fn build_hasher(&self) -> Self::Hasher {
        TerrainMeshOwnerHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for TerrainMeshOwnerHasher {
    fn finish(&self) -> u64 {
        // MurmurHash3's finalizer preserves the cheap word-at-a-time input path below while
        // spreading adjacent chunk/section coordinates across both hashbrown's bucket bits and
        // its seven-bit control fingerprint.
        let mut hash = self.0;
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        hash ^ (hash >> 33)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 ^= value;
        self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
    }

    fn write_i32(&mut self, value: i32) {
        self.write_u64(u64::from(value as u32));
    }
}

pub(super) type TerrainSectionMap<V> =
    HashMap<TerrainSectionMeshOwner, V, TerrainMeshOwnerBuildHasher>;

pub(super) type TerrainSectionSet = HashSet<TerrainSectionMeshOwner, TerrainMeshOwnerBuildHasher>;

/// Immutable native section-lifecycle projection consumed by the terrain GPU
/// owner in near-to-far traversal order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainSectionRenderState {
    pub owner: TerrainSectionMeshOwner,
    pub visibility: f32,
    /// Vanilla `SectionOcclusionGraph.addSectionsInFrustum`'s `isClose`
    /// projection, consumed by nearby-first translucent resort scheduling.
    pub nearby: bool,
}

/// `GraphicsPreset.FABULOUS` defaults to anisotropic terrain filtering
/// uniformly across adapters unless an explicit texture-filtering option
/// overrides it.
pub(super) fn resolve_fabulous_terrain_texture_filtering(
    requested: TerrainTextureFiltering,
    fabulous_auto: bool,
) -> TerrainTextureFiltering {
    if fabulous_auto {
        TerrainTextureFiltering::Anisotropic
    } else {
        requested
    }
}

pub(super) fn camera_block_position(camera_position: [f32; 3]) -> [i32; 3] {
    camera_position.map(|coordinate| coordinate.floor() as i32)
}

pub(super) fn translucency_point_of_view(
    camera_position: [f32; 3],
    owner: TerrainSectionMeshOwner,
) -> [i8; 3] {
    let camera_section =
        camera_block_position(camera_position).map(|coordinate| coordinate.div_euclid(16));
    [
        (camera_section[0] - owner.chunk.chunk_x).clamp(-1, 1) as i8,
        (camera_section[1] - owner.section_y).clamp(-1, 1) as i8,
        (camera_section[2] - owner.chunk.chunk_z).clamp(-1, 1) as i8,
    ]
}

impl Renderer {
    pub fn set_terrain_mesh_worker_counters(&mut self, counters: TerrainMeshWorkerCounters) {
        self.counters.terrain_selected_chunks = counters.selected_chunks;
        self.counters.terrain_resident_chunks = counters.resident_chunks;
        self.counters.terrain_selected_sections = counters.selected_sections;
        self.counters.terrain_resident_sections = counters.resident_sections;
        self.counters.terrain_pending_jobs = counters.pending_jobs;
        self.counters.terrain_cache_ready = counters.cache_ready;
        self.counters.terrain_mesh_jobs_queued = counters.queued_jobs;
        self.counters.terrain_mesh_jobs_completed = counters.completed_jobs;
        self.counters.terrain_mesh_jobs_stale = counters.stale_jobs;
        self.counters.terrain_mesh_sections_queued = counters.queued_sections;
        self.counters.terrain_mesh_sections_completed = counters.completed_sections;
        self.counters.terrain_mesh_sections_stale = counters.stale_sections;
        self.counters.terrain_mesh_queue_full = counters.queue_full;
        self.counters.terrain_mesh_queue_depth = counters.queue_depth;
        self.counters.terrain_mesh_peak_queue_depth = counters.peak_queue_depth;
        self.counters.terrain_mesh_recent_build_nanos = counters.recent_build_nanos;
        self.counters.terrain_mesh_peak_build_nanos = counters.peak_build_nanos;
        self.counters.terrain_mesh_recent_build_sections = counters.recent_build_sections;
        self.counters.terrain_mesh_peak_build_sections = counters.peak_build_sections;
    }

    fn begin_terrain_upload(&mut self, source_sections: usize) {
        self.terrain.set_source_sections(source_sections);
        self.counters.queued_sections = source_sections;
        self.counters.meshed_sections = source_sections;
        self.counters.upload_bytes = 0;
        self.counters.terrain_changed_owners = 0;
        self.counters.terrain_uploaded_owners = 0;
        self.counters.terrain_removed_owners = 0;
        self.counters.terrain_vertex_upload_bytes = 0;
        self.counters.terrain_index_upload_bytes = 0;
    }

    /// Synchronizes renderer-owned section/layer GPU residency by stable
    /// chunk/section owner and revision. Unchanged sections keep their exact
    /// arena ranges even when a sibling section in the chunk changes.
    pub fn synchronize_terrain_section_mesh_layer_refs<'a>(
        &mut self,
        source_sections: usize,
        chunks: impl IntoIterator<
            Item = (
                TerrainChunkMeshOwner,
                &'a [(i32, u64)],
                &'a terrain::TerrainSectionMeshLayers,
                [f32; 3],
            ),
        >,
    ) {
        let chunks: Vec<_> = chunks.into_iter().collect();
        let mut desired = TerrainSectionSet::with_capacity_and_hasher(
            source_sections,
            TerrainMeshOwnerBuildHasher,
        );
        for (chunk, revisions, _, _) in &chunks {
            desired.extend(
                revisions
                    .iter()
                    .map(|&(section_y, _)| TerrainSectionMeshOwner {
                        chunk: *chunk,
                        section_y,
                    }),
            );
        }
        self.begin_terrain_upload(source_sections);

        // Vanilla keeps the current SectionOcclusionGraph while `schedulePropagationFrom`
        // incrementally admits newly compiled sections: owners leaving residency drop out of
        // the projection, a newly resident owner stays culled until native publishes membership
        // and fade together, and recompiling an existing owner keeps its projection.
        let removed: Vec<_> = self
            .terrain
            .resident_owners()
            .filter(|owner| !desired.contains(owner))
            .collect();
        for owner in removed {
            self.counters.terrain_removed_owners += self.terrain.remove_owner(owner).0;
        }
        self.terrain.finish_removals();

        for (chunk, revisions, layers, mesh_camera_position) in chunks {
            for &(section_y, revision) in revisions {
                let owner = TerrainSectionMeshOwner { chunk, section_y };
                if self.terrain.revision(&owner) == Some(revision) {
                    continue;
                }
                self.counters.terrain_changed_owners += 1;
                self.terrain.upsert(
                    &self.device,
                    &mut self.terrain_uploads,
                    &mut self.counters,
                    owner,
                    revision,
                    layers,
                    mesh_camera_position,
                );
            }
        }

        self.terrain_uploads.submit(&self.queue);
        self.resort_translucent_terrain_for_camera();
        self.update_camera();
        self.terrain.finish_upload(&mut self.counters);
    }

    /// Applies Vanilla-style recently-compiled section deltas without walking
    /// every resident section. Full synchronization remains the owner for
    /// level/view-area replacement; ordinary compile completion updates only
    /// the stable section owners named here.
    pub fn update_terrain_section_mesh_layer_refs<'a>(
        &mut self,
        source_sections: usize,
        changes: impl IntoIterator<
            Item = (
                TerrainSectionMeshOwner,
                u64,
                &'a terrain::TerrainSectionMeshLayers,
                [f32; 3],
            ),
        >,
        removed: impl IntoIterator<Item = TerrainSectionMeshOwner>,
    ) {
        self.begin_terrain_upload(source_sections);
        let mut translucent_changed = false;
        for owner in removed {
            let (meshes, translucent) = self.terrain.remove_owner(owner);
            self.counters.terrain_removed_owners += meshes;
            translucent_changed |= translucent;
        }
        self.terrain.finish_removals();

        for (owner, revision, layers, mesh_camera_position) in changes {
            if self.terrain.revision(&owner) == Some(revision) {
                continue;
            }
            self.counters.terrain_changed_owners += 1;
            translucent_changed |= self.terrain.upsert(
                &self.device,
                &mut self.terrain_uploads,
                &mut self.counters,
                owner,
                revision,
                layers,
                mesh_camera_position,
            );
        }

        self.terrain_uploads.submit(&self.queue);
        if translucent_changed {
            self.resort_translucent_terrain_for_camera();
        }
        self.terrain.finish_upload(&mut self.counters);
    }

    /// Atomically replaces the native section-lifecycle projection. Membership,
    /// near-to-far order and fade visibility travel in one value so the terrain
    /// shader cannot drift from entity/block-entity visibility decisions.
    pub fn set_terrain_section_states(
        &mut self,
        revision: u64,
        sections: &[TerrainSectionRenderState],
    ) {
        if self.terrain.set_section_states(revision, sections) {
            self.counters.visible_sections = self.terrain.visible_sections();
        }
    }

    /// Updates section fade progress in place when `layout_revision` (membership and draw order)
    /// is unchanged, avoiding draw-list rebuilds and `RenderBundle` invalidation during fade-in.
    pub fn set_terrain_section_states_with_fades(
        &mut self,
        layout_revision: u64,
        state_revision: u64,
        sections: &[TerrainSectionRenderState],
        fading_sections: &[(TerrainSectionMeshOwner, f32)],
    ) {
        if self.terrain.set_section_states_with_fades(
            layout_revision,
            state_revision,
            sections,
            fading_sections,
        ) {
            self.counters.visible_sections = self.terrain.visible_sections();
        }
    }

    /// Section owners whose upload failed and may fit now; the delta upload names them again.
    pub fn terrain_retry_owners(&self) -> Vec<TerrainSectionMeshOwner> {
        self.terrain.retry_owners().collect()
    }

    pub fn terrain_section_mesh_is_uploaded(
        &self,
        owner: &TerrainSectionMeshOwner,
        revision: u64,
    ) -> bool {
        self.terrain.revision(owner) == Some(revision)
    }

    pub(super) fn resort_translucent_terrain_for_camera(&mut self) {
        let camera_position = self.camera_sort_position();
        self.terrain.resort_translucent_for_camera(
            &self.device,
            &mut self.frame_uploads,
            camera_position,
            &mut self.counters,
        );
    }

    pub(crate) fn camera_sort_position(&self) -> Option<[f32; 3]> {
        self.camera_pose.map(|pose| {
            [
                pose.position[0],
                pose.position[1] + pose.eye_height,
                pose.position[2],
            ]
        })
    }

    /// Test fixture: one always-visible opaque section at the origin.
    #[cfg(test)]
    pub(crate) fn show_test_terrain_mesh(&mut self, mesh: terrain::TerrainMesh) {
        let chunk = TerrainChunkMeshOwner {
            level_generation: 0,
            chunk_x: 0,
            chunk_z: 0,
        };
        let layers = terrain::TerrainSectionMeshLayers {
            opaque: vec![terrain::TerrainSectionMesh { section_y: 0, mesh }],
            source_sections: 1,
            ..terrain::TerrainSectionMeshLayers::default()
        };
        self.synchronize_terrain_section_mesh_layer_refs(
            1,
            [(chunk, &[(0, 1)][..], &layers, [0.0; 3])],
        );
        self.set_terrain_section_states(
            u64::MAX,
            &[TerrainSectionRenderState {
                owner: TerrainSectionMeshOwner {
                    chunk,
                    section_y: 0,
                },
                visibility: 1.0,
                nearby: true,
            }],
        );
    }

    pub fn upload_terrain_texture_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.upload_terrain_texture_atlas_mips(width, height, &[rgba])
    }

    /// Vanilla `Options.maxAnisotropyBit` invalidates the chunk-layer sampler
    /// via `LevelRenderer::resetSampler`; bbb options are startup-only, so the
    /// bit must be set before the real atlas upload recreates the sampler.
    pub fn set_terrain_max_anisotropy_bit(&mut self, max_anisotropy_bit: u32) {
        self.terrain_max_anisotropy_bit = max_anisotropy_bit;
    }

    pub fn terrain_max_anisotropy(&self) -> u16 {
        effective_terrain_max_anisotropy(
            self.terrain_texture_filtering,
            self.terrain_max_anisotropy_bit,
        )
    }

    pub fn terrain_texture_filtering(&self) -> TerrainTextureFiltering {
        self.terrain_texture_filtering
    }

    pub fn update_terrain_texture_atlas(&mut self, rgba: &[u8]) -> Result<()> {
        write_terrain_atlas_gpu(&self.queue, &self.pack.terrain_atlas, rgba)
    }

    pub fn update_terrain_texture_atlas_mips(&mut self, mip_rgba: &[&[u8]]) -> Result<()> {
        write_terrain_atlas_mips_gpu(&self.queue, &self.pack.terrain_atlas, mip_rgba)
    }

    pub fn update_terrain_texture_atlas_mip_patches(
        &mut self,
        patches: &[bbb_render_types::AtlasMipPatch],
    ) -> Result<()> {
        write_terrain_atlas_mip_patches_gpu(&self.queue, &self.pack.terrain_atlas, patches)
    }

    pub fn upload_terrain_texture_animation(
        &mut self,
        plan: Option<bbb_render_types::AtlasGpuAnimationPlan>,
    ) -> Result<()> {
        let animation = plan
            .map(|plan| TerrainAtlasAnimationGpu::new(&self.device, &self.queue, plan))
            .transpose()?;
        if animation
            .as_ref()
            .is_some_and(|animation| !animation.matches_atlas(&self.pack.terrain_atlas))
        {
            anyhow::bail!("terrain texture animation plan does not match the resident atlas");
        }
        self.pack.terrain_atlas_animation = animation;
        Ok(())
    }

    pub fn advance_terrain_texture_animation(&mut self, previous_tick: u64, tick: u64) {
        if let Some(animation) = self.pack.terrain_atlas_animation.as_mut() {
            animation.advance(previous_tick, tick);
        }
    }
}
