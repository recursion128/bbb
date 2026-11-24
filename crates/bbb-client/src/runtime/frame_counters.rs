use super::*;

/// Project the renderer-owned counter block into the control/snapshot mirror.
///
/// The renderer owns the authoritative `bbb_client_render::RendererCounters`; the
/// control crate keeps a serialization-facing mirror so it never has to depend
/// on the renderer. This field-by-field copy is the boundary crossing, matching
/// the repo's established projection philosophy (as with `NetCounters`).
pub(crate) fn control_renderer_counters(
    counters: bbb_client_render::RendererCounters,
) -> RendererCounters {
    RendererCounters {
        recording: Default::default(),
        frame_index: counters.frame_index,
        width: counters.width,
        height: counters.height,
        gui_width: counters.gui_width,
        gui_height: counters.gui_height,
        surface_resizes: counters.surface_resizes,
        surface_zero_size_skips: counters.surface_zero_size_skips,
        surface_lost: counters.surface_lost,
        surface_outdated: counters.surface_outdated,
        surface_timeouts: counters.surface_timeouts,
        surface_reconfigurations: counters.surface_reconfigurations,
        surface_frames_skipped: counters.surface_frames_skipped,
        window_resize_requests: counters.window_resize_requests,
        window_minimize_requests: counters.window_minimize_requests,
        window_restore_requests: counters.window_restore_requests,
        draw_calls: counters.draw_calls,
        opaque_draw_calls: counters.opaque_draw_calls,
        cutout_draw_calls: counters.cutout_draw_calls,
        translucent_draw_calls: counters.translucent_draw_calls,
        block_destroy_overlay_draw_calls: counters.block_destroy_overlay_draw_calls,
        sky_draw_calls: counters.sky_draw_calls,
        particle_draw_calls: counters.particle_draw_calls,
        weather_draw_calls: counters.weather_draw_calls,
        item_entity_draw_calls: counters.item_entity_draw_calls,
        selection_draw_calls: counters.selection_draw_calls,
        entity_scene_draw_calls: counters.entity_scene_draw_calls,
        chunk_border_draw_calls: counters.chunk_border_draw_calls,
        chunk_border_boxes: counters.chunk_border_boxes,
        entity_scene_boxes: counters.entity_scene_boxes,
        item_entity_billboards: counters.item_entity_billboards,
        item_model_residency_hits: counters.item_model_residency_hits,
        item_model_residency_misses: counters.item_model_residency_misses,
        item_model_residency_builds: counters.item_model_residency_builds,
        item_model_residency_stale: counters.item_model_residency_stale,
        item_model_lookup_nanos: counters.item_model_lookup_nanos,
        item_model_lookup_peak_nanos: counters.item_model_lookup_peak_nanos,
        item_instance_extraction_nanos: counters.item_instance_extraction_nanos,
        item_instance_extraction_peak_nanos: counters.item_instance_extraction_peak_nanos,
        item_gpu_apply_nanos: counters.item_gpu_apply_nanos,
        item_gpu_apply_peak_nanos: counters.item_gpu_apply_peak_nanos,
        item_gpu_owners_created: counters.item_gpu_owners_created,
        item_gpu_owners_reused: counters.item_gpu_owners_reused,
        item_gpu_owners_replaced: counters.item_gpu_owners_replaced,
        item_model_vertex_upload_bytes: counters.item_model_vertex_upload_bytes,
        item_model_index_upload_bytes: counters.item_model_index_upload_bytes,
        item_instance_upload_bytes: counters.item_instance_upload_bytes,
        item_instance_upload_writes: counters.item_instance_upload_writes,
        item_instance_buffer_replacements: counters.item_instance_buffer_replacements,
        item_map_upload_bytes: counters.item_map_upload_bytes,
        item_full_residency_replacements: counters.item_full_residency_replacements,
        item_model_resident_bytes: counters.item_model_resident_bytes,
        item_model_peak_resident_bytes: counters.item_model_peak_resident_bytes,
        item_model_instances: counters.item_model_instances,
        item_model_fallbacks: counters.item_model_fallbacks,
        map_textures_created: counters.map_textures_created,
        map_textures_updated: counters.map_textures_updated,
        map_textures_reused: counters.map_textures_reused,
        map_textures_stale: counters.map_textures_stale,
        map_texture_fallbacks: counters.map_texture_fallbacks,
        map_full_residency_replacements: counters.map_full_residency_replacements,
        map_texture_resident_bytes: counters.map_texture_resident_bytes,
        map_texture_peak_resident_bytes: counters.map_texture_peak_resident_bytes,
        map_instance_upload_bytes: counters.map_instance_upload_bytes,
        map_surface_instances: counters.map_surface_instances,
        item_extraction_scratch_capacity_growths: counters.item_extraction_scratch_capacity_growths,
        panorama_resource_ready: counters.panorama_resource_ready,
        panorama_draw_calls: counters.panorama_draw_calls,
        panorama_resident_bytes: counters.panorama_resident_bytes,
        hud_draw_calls: counters.hud_draw_calls,
        screen_blur_draw_calls: counters.screen_blur_draw_calls,
        post_effect_draw_calls: counters.post_effect_draw_calls,
        pipeline_switches: counters.pipeline_switches,
        screenshots_written: counters.screenshots_written,
        queued_sections: counters.queued_sections,
        meshed_sections: counters.meshed_sections,
        uploaded_sections: counters.uploaded_sections,
        visible_sections: counters.visible_sections,
        upload_bytes: counters.upload_bytes,
        resident_bytes: counters.resident_bytes,
        terrain_changed_owners: counters.terrain_changed_owners,
        terrain_uploaded_owners: counters.terrain_uploaded_owners,
        terrain_removed_owners: counters.terrain_removed_owners,
        terrain_vertex_upload_bytes: counters.terrain_vertex_upload_bytes,
        terrain_index_upload_bytes: counters.terrain_index_upload_bytes,
        terrain_translucent_resort_owners: counters.terrain_translucent_resort_owners,
        terrain_translucent_resort_index_bytes: counters.terrain_translucent_resort_index_bytes,
        terrain_full_replacements: counters.terrain_full_replacements,
        terrain_peak_resident_bytes: counters.terrain_peak_resident_bytes,
        terrain_selected_chunks: counters.terrain_selected_chunks,
        terrain_resident_chunks: counters.terrain_resident_chunks,
        terrain_selected_sections: counters.terrain_selected_sections,
        terrain_resident_sections: counters.terrain_resident_sections,
        terrain_pending_jobs: counters.terrain_pending_jobs,
        terrain_cache_ready: counters.terrain_cache_ready,
        terrain_mesh_jobs_queued: counters.terrain_mesh_jobs_queued,
        terrain_mesh_jobs_completed: counters.terrain_mesh_jobs_completed,
        terrain_mesh_jobs_stale: counters.terrain_mesh_jobs_stale,
        terrain_mesh_sections_queued: counters.terrain_mesh_sections_queued,
        terrain_mesh_sections_completed: counters.terrain_mesh_sections_completed,
        terrain_mesh_sections_stale: counters.terrain_mesh_sections_stale,
        terrain_mesh_queue_full: counters.terrain_mesh_queue_full,
        terrain_mesh_queue_depth: counters.terrain_mesh_queue_depth,
        terrain_mesh_peak_queue_depth: counters.terrain_mesh_peak_queue_depth,
        terrain_mesh_recent_build_nanos: counters.terrain_mesh_recent_build_nanos,
        terrain_mesh_peak_build_nanos: counters.terrain_mesh_peak_build_nanos,
        terrain_mesh_recent_build_sections: counters.terrain_mesh_recent_build_sections,
        terrain_mesh_peak_build_sections: counters.terrain_mesh_peak_build_sections,
        atlas_pages: counters.atlas_pages,
        atlas_reallocations: counters.atlas_reallocations,
        atlas_width: counters.atlas_width,
        atlas_height: counters.atlas_height,
        hud_crosshair_width: counters.hud_crosshair_width,
        hud_crosshair_height: counters.hud_crosshair_height,
        terrain_vertices: counters.terrain_vertices,
        terrain_indices: counters.terrain_indices,
        opaque_faces: counters.opaque_faces,
        cutout_faces: counters.cutout_faces,
        translucent_faces: counters.translucent_faces,
        culled_faces: counters.culled_faces,
        pending_particle_spawns: counters.pending_particle_spawns,
        active_particle_instances: counters.active_particle_instances,
        entity_render_candidates: 0,
        entity_render_survivors: 0,
        entity_render_projected: 0,
        entity_candidate_nanos: 0,
        entity_candidate_peak_nanos: 0,
        entity_cull_nanos: 0,
        entity_cull_peak_nanos: 0,
        entity_full_projection_nanos: 0,
        entity_full_projection_peak_nanos: 0,
        entity_model_build_nanos: 0,
        entity_model_build_peak_nanos: 0,
        entity_extraction_capacity_growths: 0,
        non_entity_hud_nanos: 0,
        non_entity_hud_peak_nanos: 0,
        non_entity_item_nanos: 0,
        non_entity_item_peak_nanos: 0,
        non_entity_block_entity_nanos: 0,
        non_entity_block_entity_peak_nanos: 0,
        non_entity_world_debug_nanos: 0,
        non_entity_world_debug_peak_nanos: 0,
        non_entity_apply_nanos: 0,
        non_entity_apply_peak_nanos: 0,
        non_entity_extraction_capacity_growths: 0,
        non_entity_cache_hits: 0,
        non_entity_cache_misses: 0,
        non_entity_rebuilt_owners: 0,
        dynamic_texture_cache_entries: 0,
        profile_texture_set_entries: 0,
        profile_texture_asset_entries: 0,
        profile_texture_key_bytes: 0,
        dynamic_texture_cache_pending: 0,
        dynamic_texture_cache_ready: 0,
        dynamic_texture_cache_failed: 0,
        dynamic_texture_cache_stale_results: 0,
        dynamic_texture_cache_queue_full: 0,
        dynamic_texture_cache_max_resident_bytes: 0,
        dynamic_texture_cache_resident_bytes: 0,
        dynamic_texture_cache_peak_resident_bytes: 0,
        dynamic_texture_cache_resident_evictions: 0,
        dynamic_texture_cache_resident_rejections: 0,
        dynamic_texture_worker_result_bytes: 0,
        dynamic_texture_peak_worker_result_bytes: 0,
        dynamic_texture_recent_delivery_bytes: 0,
        dynamic_texture_peak_delivery_bytes: 0,
    }
}

pub(crate) fn project_dynamic_texture_counters(
    counters: &mut RendererCounters,
    items: Option<&NativeItemRuntime>,
) {
    let observation = items
        .map(NativeItemRuntime::dynamic_texture_cache_observation)
        .unwrap_or_default();
    counters.dynamic_texture_cache_entries = observation.entries;
    counters.profile_texture_set_entries = observation.profile_sets;
    counters.profile_texture_asset_entries = observation.profile_assets;
    counters.profile_texture_key_bytes =
        observation.profile_key_bytes.try_into().unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_pending = observation.pending;
    counters.dynamic_texture_cache_ready = observation.ready;
    counters.dynamic_texture_cache_failed = observation.failed;
    counters.dynamic_texture_cache_stale_results = observation.stale_results;
    counters.dynamic_texture_cache_queue_full = observation.queue_full;
    counters.dynamic_texture_cache_max_resident_bytes = observation
        .max_resident_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_resident_bytes =
        observation.resident_bytes.try_into().unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_peak_resident_bytes = observation
        .peak_resident_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_resident_evictions = observation.resident_evictions;
    counters.dynamic_texture_cache_resident_rejections = observation.resident_rejections;
    counters.dynamic_texture_worker_result_bytes = observation
        .worker_result_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_peak_worker_result_bytes = observation
        .peak_worker_result_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_recent_delivery_bytes = observation
        .recent_delivery_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_peak_delivery_bytes = observation
        .peak_delivery_bytes
        .try_into()
        .unwrap_or(u64::MAX);
}
