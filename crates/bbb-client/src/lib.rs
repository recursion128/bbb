//! BBB external client library shared by the native desktop binary, the Web
//! WASM client, and the `bbb-web-host` reverse control/MCP gateway.
#![cfg_attr(
    all(target_arch = "wasm32", target_feature = "atomics"),
    feature(stdarch_wasm_atomic_wait)
)]

#[cfg(not(target_arch = "wasm32"))]
mod application_lifecycle;
mod audio_runtime;
mod banner_scene;
mod beacon_scene;
mod bed_scene;
mod bell_scene;
mod block_entity_visibility;
mod bounding_box_gizmos;
mod camera_fov;
mod camera_pose;
mod cartography_preview;
mod chest_scene;
mod code_of_conduct;
mod code_of_conduct_overlay;
mod component_text;
mod conduit_scene;
pub mod control;
mod copper_golem_statue_scene;
#[cfg(not(target_arch = "wasm32"))]
mod crash_report;
mod crosshair;
mod debug_entries;
mod debug_profiler;
mod decorated_pot_scene;
#[cfg(not(target_arch = "wasm32"))]
mod desktop;
mod dynamic_lights;
mod enchanting_table_book_scene;
mod end_portal_scene;
mod entity_assets;
mod entity_features;
mod entity_name_tags;
mod entity_scene;
mod entity_variant_textures;
mod environment_animate_runtime;
mod environment_audio_runtime;
mod event_loop_shared;
mod frame_diagnostics;
mod game_test_highlights;
mod hotbar_presets;
mod hud_assets;
mod input;
mod item_entities;
mod item_frames;
mod item_models;
#[cfg(test)]
mod item_registry_test_fixture;
mod item_texture_animation;
mod lectern_book_scene;
mod level_load_runtime;
#[cfg(feature = "lifecycle-soak")]
mod lifecycle_soak;
mod lod_terrain;
mod map_textures;
#[cfg(not(target_arch = "wasm32"))]
pub mod mcp;
#[cfg(not(target_arch = "wasm32"))]
mod mcp_runtime;
mod music_runtime;
mod narrator_runtime;
mod network_runtime;
mod occlusion_culling;
mod options;
mod particle_runtime;
mod platform;
#[cfg(not(target_arch = "wasm32"))]
mod platform_macos;
mod pointer_input;
mod qol_extensions;
mod recipe_book_ui;
mod recording;
mod remote_debug;
mod resource_pack_runtime;
mod resource_reload;
mod runtime;
mod screen_effects;
mod screenshot;
mod server_list;
mod session_shared;
mod shulker_box_scene;
mod sign_scene;
mod simulation;
mod skull_scene;
mod sky_assets;
mod sound_subtitle_runtime;
mod spawner_scene;
mod startup;
pub(crate) mod terrain_upload;
mod test_instance_gizmos;
mod weather_effect_runtime;
#[cfg(any(target_arch = "wasm32", test))]
pub mod web;
mod win_screen_resources;
#[cfg(not(target_arch = "wasm32"))]
mod window_icon;
mod window_surface;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::run_desktop_main;

#[cfg(target_arch = "wasm32")]
pub use web::{init_thread_pool, open_web_resource_pack_picker, start_web_client};
