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
    pub selection_draw_calls: u64,
    pub hud_draw_calls: u64,
    pub pipeline_switches: u64,
    pub screenshots_written: u64,
    pub queued_sections: usize,
    pub meshed_sections: usize,
    pub uploaded_sections: usize,
    pub visible_sections: usize,
    pub upload_bytes: u64,
    pub resident_bytes: u64,
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
    pub particle_runtime_ticks: u64,
    #[serde(default)]
    pub particle_instances_created: u64,
    #[serde(default)]
    pub particle_instances_expired: u64,
    #[serde(default)]
    pub dropped_active_particle_instances: u64,
}
