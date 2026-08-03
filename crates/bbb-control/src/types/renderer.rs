use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RendererCounters {
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub surface_resizes: u64,
    #[serde(default)]
    pub surface_zero_size_skips: u64,
    #[serde(default)]
    pub surface_lost: u64,
    #[serde(default)]
    pub surface_outdated: u64,
    #[serde(default)]
    pub surface_timeouts: u64,
    #[serde(default)]
    pub surface_reconfigurations: u64,
    #[serde(default)]
    pub surface_frames_skipped: u64,
    #[serde(default)]
    pub window_resize_requests: u64,
    #[serde(default)]
    pub window_minimize_requests: u64,
    #[serde(default)]
    pub window_restore_requests: u64,
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
    pub chunk_border_boxes: usize,
    #[serde(default)]
    pub entity_scene_boxes: usize,
    #[serde(default)]
    pub item_entity_billboards: usize,
    #[serde(default)]
    pub item_model_residency_hits: u64,
    #[serde(default)]
    pub item_model_residency_misses: u64,
    #[serde(default)]
    pub item_model_residency_builds: u64,
    #[serde(default)]
    pub item_model_residency_stale: u64,
    #[serde(default)]
    pub item_model_lookup_nanos: u64,
    #[serde(default)]
    pub item_model_lookup_peak_nanos: u64,
    #[serde(default)]
    pub item_instance_extraction_nanos: u64,
    #[serde(default)]
    pub item_instance_extraction_peak_nanos: u64,
    #[serde(default)]
    pub item_gpu_apply_nanos: u64,
    #[serde(default)]
    pub item_gpu_apply_peak_nanos: u64,
    #[serde(default)]
    pub item_gpu_owners_created: u64,
    #[serde(default)]
    pub item_gpu_owners_reused: u64,
    #[serde(default)]
    pub item_gpu_owners_replaced: u64,
    #[serde(default)]
    pub item_model_vertex_upload_bytes: u64,
    #[serde(default)]
    pub item_model_index_upload_bytes: u64,
    #[serde(default)]
    pub item_instance_upload_bytes: u64,
    #[serde(default)]
    pub item_instance_upload_writes: u64,
    #[serde(default)]
    pub item_instance_buffer_replacements: u64,
    #[serde(default)]
    pub item_map_upload_bytes: u64,
    #[serde(default)]
    pub item_transient_buffer_creations: u64,
    #[serde(default)]
    pub item_full_residency_replacements: u64,
    #[serde(default)]
    pub item_model_resident_bytes: u64,
    #[serde(default)]
    pub item_model_peak_resident_bytes: u64,
    #[serde(default)]
    pub item_model_instances: usize,
    #[serde(default)]
    pub item_model_fallbacks: u64,
    #[serde(default)]
    pub map_textures_created: u64,
    #[serde(default)]
    pub map_textures_updated: u64,
    #[serde(default)]
    pub map_textures_reused: u64,
    #[serde(default)]
    pub map_textures_stale: u64,
    #[serde(default)]
    pub map_texture_fallbacks: u64,
    #[serde(default)]
    pub map_full_residency_replacements: u64,
    #[serde(default)]
    pub map_texture_resident_bytes: u64,
    #[serde(default)]
    pub map_texture_peak_resident_bytes: u64,
    #[serde(default)]
    pub map_instance_upload_bytes: u64,
    #[serde(default)]
    pub map_surface_instances: usize,
    #[serde(default)]
    pub item_extraction_scratch_capacity_growths: u64,
    #[serde(default)]
    pub panorama_resource_ready: bool,
    #[serde(default)]
    pub panorama_draw_calls: u64,
    #[serde(default)]
    pub panorama_resident_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_cache_entries: usize,
    #[serde(default)]
    pub profile_texture_set_entries: usize,
    #[serde(default)]
    pub profile_texture_asset_entries: usize,
    #[serde(default)]
    pub profile_texture_key_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_cache_pending: usize,
    #[serde(default)]
    pub dynamic_texture_cache_ready: usize,
    #[serde(default)]
    pub dynamic_texture_cache_failed: usize,
    #[serde(default)]
    pub dynamic_texture_cache_stale_results: u64,
    #[serde(default)]
    pub dynamic_texture_cache_queue_full: u64,
    #[serde(default)]
    pub dynamic_texture_cache_max_resident_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_cache_resident_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_cache_peak_resident_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_cache_resident_evictions: u64,
    #[serde(default)]
    pub dynamic_texture_cache_resident_rejections: u64,
    #[serde(default)]
    pub dynamic_texture_worker_result_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_peak_worker_result_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_recent_delivery_bytes: u64,
    #[serde(default)]
    pub dynamic_texture_peak_delivery_bytes: u64,
    pub hud_draw_calls: u64,
    #[serde(default)]
    pub screen_blur_draw_calls: u64,
    #[serde(default)]
    pub post_effect_draw_calls: u64,
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
    pub terrain_selected_chunks: usize,
    #[serde(default)]
    pub terrain_resident_chunks: usize,
    #[serde(default)]
    pub terrain_selected_sections: usize,
    #[serde(default)]
    pub terrain_resident_sections: usize,
    #[serde(default)]
    pub terrain_pending_jobs: usize,
    #[serde(default)]
    pub terrain_cache_ready: bool,
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
    pub particle_unsupported_options: u64,
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
    #[serde(default)]
    pub non_entity_hud_nanos: u64,
    #[serde(default)]
    pub non_entity_hud_peak_nanos: u64,
    #[serde(default)]
    pub non_entity_item_nanos: u64,
    #[serde(default)]
    pub non_entity_item_peak_nanos: u64,
    #[serde(default)]
    pub non_entity_block_entity_nanos: u64,
    #[serde(default)]
    pub non_entity_block_entity_peak_nanos: u64,
    #[serde(default)]
    pub non_entity_world_debug_nanos: u64,
    #[serde(default)]
    pub non_entity_world_debug_peak_nanos: u64,
    #[serde(default)]
    pub non_entity_apply_nanos: u64,
    #[serde(default)]
    pub non_entity_apply_peak_nanos: u64,
    #[serde(default)]
    pub non_entity_extraction_capacity_growths: u64,
    #[serde(default)]
    pub non_entity_cache_hits: u64,
    #[serde(default)]
    pub non_entity_cache_misses: u64,
    #[serde(default)]
    pub non_entity_rebuilt_owners: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_renderer_observations_default_for_older_payloads() {
        let mut value = serde_json::to_value(RendererCounters::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        for field in [
            "item_instance_upload_writes",
            "item_instance_buffer_replacements",
            "dynamic_texture_cache_max_resident_bytes",
            "dynamic_texture_cache_resident_evictions",
            "dynamic_texture_cache_resident_rejections",
            "dynamic_texture_worker_result_bytes",
            "dynamic_texture_peak_worker_result_bytes",
            "dynamic_texture_recent_delivery_bytes",
            "dynamic_texture_peak_delivery_bytes",
        ] {
            object.remove(field);
        }

        let counters: RendererCounters = serde_json::from_value(value).unwrap();
        assert_eq!(counters.item_instance_upload_writes, 0);
        assert_eq!(counters.item_instance_buffer_replacements, 0);
        assert_eq!(counters.dynamic_texture_cache_max_resident_bytes, 0);
        assert_eq!(counters.dynamic_texture_cache_resident_evictions, 0);
        assert_eq!(counters.dynamic_texture_cache_resident_rejections, 0);
        assert_eq!(counters.dynamic_texture_worker_result_bytes, 0);
        assert_eq!(counters.dynamic_texture_peak_worker_result_bytes, 0);
        assert_eq!(counters.dynamic_texture_recent_delivery_bytes, 0);
        assert_eq!(counters.dynamic_texture_peak_delivery_bytes, 0);
    }
}
