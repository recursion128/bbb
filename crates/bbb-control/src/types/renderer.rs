use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RendererCounters {
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub draw_calls: u64,
    pub opaque_draw_calls: u64,
    pub cutout_draw_calls: u64,
    pub translucent_draw_calls: u64,
    #[serde(default)]
    pub block_destroy_overlay_draw_calls: u64,
    #[serde(default)]
    pub sky_draw_calls: u64,
    #[serde(default)]
    pub particle_draw_calls: u64,
    #[serde(default)]
    pub weather_draw_calls: u64,
    #[serde(default)]
    pub item_entity_draw_calls: u64,
    pub selection_draw_calls: u64,
    #[serde(default)]
    pub chunk_border_draw_calls: u64,
    #[serde(default)]
    pub entity_scene_draw_calls: u64,
    #[serde(default)]
    pub entity_target_draw_calls: u64,
    #[serde(default)]
    pub chunk_border_boxes: usize,
    #[serde(default)]
    pub entity_scene_boxes: usize,
    #[serde(default)]
    pub item_entity_billboards: usize,
    pub hud_draw_calls: u64,
    #[serde(default)]
    pub screen_blur_draw_calls: u64,
    pub pipeline_switches: u64,
    pub screenshots_written: u64,
    pub queued_sections: usize,
    pub meshed_sections: usize,
    pub uploaded_sections: usize,
    pub visible_sections: usize,
    pub upload_bytes: u64,
    pub resident_bytes: u64,
    #[serde(default)]
    pub terrain_changed_owners: usize,
    #[serde(default)]
    pub terrain_uploaded_owners: usize,
    #[serde(default)]
    pub terrain_removed_owners: usize,
    #[serde(default)]
    pub terrain_vertex_upload_bytes: u64,
    #[serde(default)]
    pub terrain_index_upload_bytes: u64,
    #[serde(default)]
    pub terrain_translucent_resort_owners: usize,
    #[serde(default)]
    pub terrain_translucent_resort_index_bytes: u64,
    #[serde(default)]
    pub terrain_full_replacements: u64,
    #[serde(default)]
    pub terrain_peak_resident_bytes: u64,
    #[serde(default)]
    pub terrain_mesh_jobs_queued: u64,
    #[serde(default)]
    pub terrain_mesh_jobs_completed: u64,
    #[serde(default)]
    pub terrain_mesh_jobs_stale: u64,
    #[serde(default)]
    pub terrain_mesh_sections_queued: u64,
    #[serde(default)]
    pub terrain_mesh_sections_completed: u64,
    #[serde(default)]
    pub terrain_mesh_sections_stale: u64,
    #[serde(default)]
    pub terrain_mesh_queue_full: u64,
    #[serde(default)]
    pub terrain_mesh_queue_depth: usize,
    #[serde(default)]
    pub terrain_mesh_peak_queue_depth: usize,
    #[serde(default)]
    pub terrain_mesh_recent_build_nanos: u64,
    #[serde(default)]
    pub terrain_mesh_peak_build_nanos: u64,
    #[serde(default)]
    pub terrain_mesh_recent_build_sections: usize,
    #[serde(default)]
    pub terrain_mesh_peak_build_sections: usize,
    pub atlas_pages: usize,
    pub atlas_reallocations: u64,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub hud_crosshair_width: u32,
    pub hud_crosshair_height: u32,
    pub terrain_vertices: usize,
    pub terrain_indices: usize,
    pub opaque_faces: usize,
    pub cutout_faces: usize,
    pub translucent_faces: usize,
    pub culled_faces: usize,
    #[serde(default)]
    pub particle_spawn_batches: u64,
    #[serde(default)]
    pub particle_spawn_commands: u64,
    #[serde(default)]
    pub particle_missing_definitions: u64,
    #[serde(default)]
    pub particle_missing_sprites: u64,
    #[serde(default)]
    pub particle_unknown_types: u64,
    #[serde(default)]
    pub last_particle_spawn_count: usize,
    #[serde(default)]
    pub pending_particle_spawns: usize,
    #[serde(default)]
    pub dropped_particle_spawns: u64,
    #[serde(default)]
    pub active_particle_instances: usize,
    #[serde(default)]
    pub last_particle_intake_count: usize,
    #[serde(default)]
    pub last_particle_tick_count: usize,
    #[serde(default)]
    pub last_particle_expired_count: usize,
    #[serde(default)]
    pub last_particle_active_drop_count: usize,
    #[serde(default)]
    pub last_particle_limited_drop_count: usize,
    #[serde(default)]
    pub particle_runtime_ticks: u64,
    #[serde(default)]
    pub particle_instances_created: u64,
    #[serde(default)]
    pub particle_instances_expired: u64,
    #[serde(default)]
    pub dropped_active_particle_instances: u64,
    #[serde(default)]
    pub dropped_limited_particle_instances: u64,
    #[serde(default)]
    pub entity_render_candidates: usize,
    #[serde(default)]
    pub entity_render_survivors: usize,
    #[serde(default)]
    pub entity_render_projected: usize,
    #[serde(default)]
    pub entity_candidate_nanos: u64,
    #[serde(default)]
    pub entity_candidate_peak_nanos: u64,
    #[serde(default)]
    pub entity_cull_nanos: u64,
    #[serde(default)]
    pub entity_cull_peak_nanos: u64,
    #[serde(default)]
    pub entity_full_projection_nanos: u64,
    #[serde(default)]
    pub entity_full_projection_peak_nanos: u64,
    #[serde(default)]
    pub entity_model_build_nanos: u64,
    #[serde(default)]
    pub entity_model_build_peak_nanos: u64,
    #[serde(default)]
    pub entity_extraction_capacity_growths: u64,
}
