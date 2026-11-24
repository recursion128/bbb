//! F3 debug overlay text (`DebugScreenOverlay.extractRenderState`): one line producer per
//! debug screen entry, in display order.

use super::*;
use bbb_client_render::{CloudShape, TerrainTextureFiltering};

/// Everything the F3 entry producers and charts read for one frame.
pub(super) struct HudDebugOverlayInputs<'a> {
    pub(super) input: &'a ClientInputState,
    pub(super) world: &'a WorldStore,
    pub(super) visible_entity_render_state_count: usize,
    pub(super) camera_pose: Option<CameraPose>,
    pub(super) interaction_camera_pose: Option<CameraPose>,
    pub(super) targeted_entity_id: Option<i32>,
    pub(super) entity_partial_tick: f32,
    pub(super) item_runtime: Option<&'a NativeItemRuntime>,
    pub(super) terrain_textures: &'a TerrainTextureState,
    pub(super) renderer_counters: &'a bbb_client_render::RendererCounters,
    pub(super) gpu_utilization: Option<f64>,
    pub(super) improved_transparency: bool,
    pub(super) cloud_shape: Option<CloudShape>,
    pub(super) texture_filtering: TerrainTextureFiltering,
    pub(super) max_anisotropy: u16,
    pub(super) render_distance_chunks: u32,
    pub(super) surface_size: winit::dpi::PhysicalSize<u32>,
    pub(super) physical_surface_size: winit::dpi::PhysicalSize<u32>,
    pub(super) fps_sampler: &'a HudDebugFpsSampler,
    pub(super) client_framerate_limit: u32,
    pub(super) client_vsync: bool,
    pub(super) network_sampler: &'a HudDebugNetworkSampler,
    pub(super) tps_sampler: &'a HudDebugTpsSampler,
    pub(super) net_counters: &'a NetCounters,
    pub(super) audio_counters: &'a AudioCounters,
    pub(super) current_post_effect_id: Option<&'a str>,
    pub(super) profiler_chart: Option<HudDebugProfilerChart>,
    pub(super) hud_info_enabled: bool,
}

#[derive(Clone, Copy)]
enum HudDebugColumn {
    Left,
    /// The position group, set off from the left lines by a blank line.
    Position,
    Right,
    /// A right-column group, set off from the lines above it by a blank line.
    RightGroup,
}

type HudDebugEntryLines = fn(&HudDebugOverlayInputs<'_>) -> Vec<String>;

/// Text entries in display order. `LocalDifficulty`, `EntitySpawnCounts` and
/// `ChunkGenerationStats` read the integrated server's level, which bbb never has, so they
/// have no producer.
const HUD_DEBUG_TEXT_ENTRIES: &[(DebugScreenEntryId, HudDebugColumn, HudDebugEntryLines)] = &[
    (
        DebugScreenEntryId::GameVersion,
        HudDebugColumn::Left,
        |_| vec![format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-client)")],
    ),
    (DebugScreenEntryId::Fps, HudDebugColumn::Left, |cx| {
        vec![hud_debug_fps_line(
            cx.fps_sampler.fps(),
            cx.client_framerate_limit,
            cx.client_vsync,
        )]
    }),
    (DebugScreenEntryId::Tps, HudDebugColumn::Left, |cx| {
        hud_debug_tps_line(cx.world).into_iter().collect()
    }),
    (DebugScreenEntryId::DayCount, HudDebugColumn::Left, |cx| {
        cx.world
            .world_time()
            .map(|time| {
                format!(
                    "Day #{}",
                    time.day_time / VANILLA_OVERWORLD_DAY_PERIOD_TICKS
                )
            })
            .into_iter()
            .collect()
    }),
    (
        DebugScreenEntryId::LightLevels,
        HudDebugColumn::Left,
        |cx| {
            cx.camera_pose
                .and_then(|pose| hud_debug_light_line(cx.world, pose))
                .into_iter()
                .collect()
        },
    ),
    (DebugScreenEntryId::Heightmap, HudDebugColumn::Left, |cx| {
        cx.camera_pose
            .and_then(|pose| hud_debug_heightmap_lines(cx.world, pose))
            .unwrap_or_default()
    }),
    (DebugScreenEntryId::Biome, HudDebugColumn::Left, |cx| {
        cx.camera_pose
            .and_then(|pose| hud_debug_biome_line(cx.world, pose))
            .into_iter()
            .collect()
    }),
    (
        DebugScreenEntryId::LookingAtBlockState,
        HudDebugColumn::Left,
        |cx| {
            hud_debug_looking_at_block_state_lines(cx.world, cx.interaction_camera_pose)
                .unwrap_or_default()
        },
    ),
    (
        DebugScreenEntryId::LookingAtBlockTags,
        HudDebugColumn::Left,
        |cx| {
            hud_debug_looking_at_block_tag_lines(cx.world, cx.interaction_camera_pose)
                .unwrap_or_default()
        },
    ),
    (
        DebugScreenEntryId::LookingAtFluidState,
        HudDebugColumn::Left,
        |cx| {
            hud_debug_looking_at_fluid_state_lines(cx.world, cx.interaction_camera_pose)
                .unwrap_or_default()
        },
    ),
    (
        DebugScreenEntryId::LookingAtFluidTags,
        HudDebugColumn::Left,
        |cx| {
            hud_debug_looking_at_fluid_tag_lines(cx.world, cx.interaction_camera_pose)
                .unwrap_or_default()
        },
    ),
    (
        DebugScreenEntryId::LookingAtEntity,
        HudDebugColumn::Left,
        |cx| {
            hud_debug_looking_at_entity(cx)
                .and_then(|entity| {
                    Some(vec![
                        "Targeted Entity".to_string(),
                        hud_debug_entity_type_name(cx.world, entity.entity_type_id)?,
                    ])
                })
                .unwrap_or_default()
        },
    ),
    (
        DebugScreenEntryId::LookingAtEntityTags,
        HudDebugColumn::Left,
        |cx| {
            hud_debug_looking_at_entity(cx)
                .and_then(|entity| {
                    Some(
                        cx.world
                            .registry_tag_names_containing_holder_id(
                                "minecraft:entity_type",
                                entity.entity_type_id,
                            )?
                            .map(|tag| format!("#{tag}"))
                            .collect(),
                    )
                })
                .unwrap_or_default()
        },
    ),
    (
        DebugScreenEntryId::ChunkRenderStats,
        HudDebugColumn::Left,
        |cx| {
            let counters = cx.renderer_counters;
            let smart_cull = if cx.input.debug_smart_cull_enabled() {
                "(s) "
            } else {
                ""
            };
            vec![format!(
                "C: {}/{} {smart_cull}D: {}, pC: {:03}, aB: {:02}",
                counters.visible_sections,
                counters.uploaded_sections,
                cx.render_distance_chunks,
                counters.queued_sections,
                0
            )]
        },
    ),
    (
        DebugScreenEntryId::EntityRenderStats,
        HudDebugColumn::Left,
        |cx| {
            vec![hud_debug_entity_render_stats_line(
                cx.world,
                cx.visible_entity_render_state_count,
            )]
        },
    ),
    (
        DebugScreenEntryId::ParticleRenderStats,
        HudDebugColumn::Left,
        |cx| {
            vec![format!(
                "P: {}",
                cx.renderer_counters.active_particle_instances
            )]
        },
    ),
    (
        DebugScreenEntryId::ChunkSourceStats,
        HudDebugColumn::Left,
        |cx| vec![cx.world.chunk_source_stats_line()],
    ),
    (DebugScreenEntryId::SoundCache, HudDebugColumn::Left, |cx| {
        vec![format!(
            "Sound cache: {} buffers, {} MiB",
            cx.audio_counters.sound_cache_buffers,
            hud_debug_sound_cache_bytes_to_mebibytes(cx.audio_counters.sound_cache_bytes)
        )]
    }),
    (DebugScreenEntryId::SoundMood, HudDebugColumn::Left, |cx| {
        let counters = cx.audio_counters;
        vec![format!(
            "Sounds: {}/{} + {}/{} (Mood {}%)",
            counters.sound_static_channels_used,
            counters.sound_static_channels_capacity,
            counters.sound_streaming_channels_used,
            counters.sound_streaming_channels_capacity,
            counters.sound_mood_percent.min(100)
        )]
    }),
    (DebugScreenEntryId::PostEffect, HudDebugColumn::Left, |cx| {
        hud_debug_post_effect_line(cx.current_post_effect_id)
            .into_iter()
            .collect()
    }),
    (
        DebugScreenEntryId::PlayerPosition,
        HudDebugColumn::Position,
        |cx| {
            cx.camera_pose
                .map(|pose| hud_debug_position_lines(cx.world, pose))
                .unwrap_or_default()
        },
    ),
    (
        DebugScreenEntryId::PlayerSectionPosition,
        HudDebugColumn::Position,
        |cx| {
            cx.camera_pose
                .map(hud_debug_section_position_line)
                .into_iter()
                .collect()
        },
    ),
    (DebugScreenEntryId::Memory, HudDebugColumn::Right, |_| {
        hud_debug_memory_lines(hud_debug_memory_snapshot())
    }),
    (
        DebugScreenEntryId::DetailedMemory,
        HudDebugColumn::Right,
        |_| hud_debug_detailed_memory_lines(hud_debug_detailed_memory_snapshot()),
    ),
    (
        DebugScreenEntryId::SystemSpecs,
        HudDebugColumn::RightGroup,
        |cx| hud_debug_system_lines(cx.physical_surface_size),
    ),
    (
        DebugScreenEntryId::GpuUtilization,
        HudDebugColumn::RightGroup,
        |cx| vec![hud_debug_gpu_utilization_line(cx.gpu_utilization)],
    ),
    (
        DebugScreenEntryId::SimplePerformanceImpactors,
        HudDebugColumn::RightGroup,
        simple_performance_impactors,
    ),
];

fn simple_performance_impactors(cx: &HudDebugOverlayInputs<'_>) -> Vec<String> {
    let transparency = if cx.improved_transparency {
        "improved-transparency "
    } else {
        ""
    };
    let clouds = match cx.cloud_shape {
        None => "",
        Some(CloudShape::Flat) => "fast-clouds ",
        Some(CloudShape::Fancy) => "fancy-clouds ",
    };
    let radius = cx.terrain_textures.biome_blend_radius();
    let (key, english) = match cx.texture_filtering {
        TerrainTextureFiltering::None => ("options.textureFiltering.none", "None"),
        TerrainTextureFiltering::Rgss => ("options.textureFiltering.rgss", "RGSS"),
        TerrainTextureFiltering::Anisotropic => {
            ("options.textureFiltering.anisotropic", "Anisotropic")
        }
    };
    let caption = localized_gui_text(cx.item_runtime, key, english);
    let filtering = if cx.texture_filtering == TerrainTextureFiltering::Anisotropic {
        format!("Filtering: {caption} {}x", cx.max_anisotropy)
    } else {
        format!("Filtering: {caption}")
    };
    vec![format!("{transparency}{clouds}B: {radius}"), filtering]
}

pub(super) fn hud_debug_overlay(cx: HudDebugOverlayInputs<'_>) -> Option<HudDebugOverlay> {
    let input = cx.input;
    let world = cx.world;
    let reduced_debug_info = input.show_only_reduced_info(world);
    let entry_enabled = |entry| input.debug_screen_entry_enabled(entry, reduced_debug_info);

    let mut left_lines = Vec::new();
    let mut position_lines = Vec::new();
    let mut right_lines = Vec::new();
    for &(entry, column, lines) in HUD_DEBUG_TEXT_ENTRIES {
        if !entry_enabled(entry) {
            continue;
        }
        let lines = lines(&cx);
        match column {
            HudDebugColumn::Left => left_lines.extend(lines),
            HudDebugColumn::Position => position_lines.extend(lines),
            HudDebugColumn::Right => right_lines.extend(lines),
            HudDebugColumn::RightGroup => {
                if !right_lines.is_empty() {
                    right_lines.push(String::new());
                }
                right_lines.extend(lines);
            }
        }
    }
    if !position_lines.is_empty() {
        if !left_lines.is_empty() {
            left_lines.push(String::new());
        }
        left_lines.extend(position_lines);
    }
    if input.debug_overlay_visible() {
        left_lines.extend(hud_debug_help_lines(input, world));
    }
    if cx.hud_info_enabled && !input.debug_overlay_visible() && left_lines.is_empty() {
        left_lines.extend(hud_info_status_lines(&cx));
    }
    let debug_crosshair = cx
        .camera_pose
        .filter(|_| entry_enabled(DebugScreenEntryId::ThreeDimensionalCrosshair))
        .map(|pose| HudDebugCrosshair {
            x_rot_degrees: pose.x_rot,
            y_rot_degrees: pose.y_rot,
            gui_scale: 1,
        });
    let game_mode_switcher = hud_debug_game_mode_switcher(
        input,
        world,
        cx.item_runtime,
        cx.terrain_textures,
        cx.surface_size,
        cx.entity_partial_tick,
    );
    if left_lines.is_empty()
        && right_lines.is_empty()
        && debug_crosshair.is_none()
        && game_mode_switcher.is_none()
    {
        return cx.profiler_chart.map(|chart| HudDebugOverlay {
            profiler_chart: Some(chart),
            ..HudDebugOverlay::default()
        });
    }

    let fps_charts = input.debug_fps_charts_visible();
    Some(HudDebugOverlay {
        left_lines,
        right_lines,
        debug_crosshair,
        game_mode_switcher,
        profiler_chart: cx.profiler_chart,
        fps_chart: fps_charts.then(|| HudDebugFrameTimeChart {
            frame_time_nanos: cx.fps_sampler.frame_time_nanos(),
            configured_framerate_limit: hud_debug_configured_framerate_limit(
                cx.client_framerate_limit,
            ),
        }),
        tps_chart: fps_charts.then(|| cx.tps_sampler.chart(world)).flatten(),
        network_charts: (input.debug_network_charts_visible() && cx.net_counters.connected).then(
            || HudDebugNetworkCharts {
                ping_millis: cx.network_sampler.ping_millis(),
                bandwidth_bytes_per_tick: cx.network_sampler.bandwidth_bytes_per_tick(),
                show_bandwidth: true,
            },
        ),
        show_lightmap_preview: input.debug_lightmap_texture_visible(),
    })
}

fn hud_debug_help_lines(input: &ClientInputState, world: &WorldStore) -> [String; 10] {
    let shown = |visible| if visible { "visible" } else { "hidden" };
    let enabled = |enabled| if enabled { "enabled" } else { "disabled" };
    [
        String::new(),
        format!(
            "Debug charts: [F3+1] Profiler {}; [F3+2] FPS {};",
            shown(input.debug_profiler_chart_visible()),
            shown(input.debug_fps_charts_visible())
        ),
        format!(
            "[F3+3] Network {}; [F3+4] Lightmap {}",
            shown(input.debug_network_charts_visible()),
            shown(input.debug_lightmap_texture_visible())
        ),
        format!(
            "Debug toggles: [F3+B] Hitboxes {}; [F3+G] Chunks {}",
            shown(input.debug_entity_hitboxes_visible_for_world(world)),
            shown(input.debug_chunk_borders_visible_for_world(world))
        ),
        format!(
            "Debug options: [F3+H] Tooltips {}; [F3+P] Focus pause {}",
            enabled(input.debug_advanced_item_tooltips()),
            enabled(input.debug_pause_on_lost_focus())
        ),
        "Debug actions: [F3+A] Reload chunks; [F3+C] Copy location".to_string(),
        "Debug actions: [F3+D] Clear chat; [F3+S] Dump textures".to_string(),
        "Debug actions: [F3+T] Reload packs; [F3+V] Version".to_string(),
        "Game mode: [F3+N] Spectator; [F3+F4] Switcher".to_string(),
        "To edit: press [F3+F6]".to_string(),
    ]
}

fn hud_debug_looking_at_entity(
    cx: &HudDebugOverlayInputs<'_>,
) -> Option<bbb_client_world::EntityState> {
    cx.world.probe_entity(cx.targeted_entity_id?)
}

pub(super) fn hud_debug_fps_line(
    fps: u32,
    client_framerate_limit: u32,
    client_vsync: bool,
) -> String {
    let limit = if client_framerate_limit == VANILLA_UNLIMITED_FRAMERATE_LIMIT {
        "inf".to_string()
    } else {
        client_framerate_limit.to_string()
    };
    let vsync = if client_vsync { " vsync" } else { "" };
    format!("{fps} fps T: {limit}{vsync}")
}

pub(super) fn hud_debug_configured_framerate_limit(client_framerate_limit: u32) -> Option<u32> {
    (1..=250)
        .contains(&client_framerate_limit)
        .then_some(client_framerate_limit)
}

pub(super) fn hud_debug_tps_line(world: &WorldStore) -> Option<String> {
    world.level_info()?;
    let ticking = world.ticking();
    let run_status = if ticking.frozen_ticks_to_run > 0 {
        " (frozen - stepping)"
    } else if ticking.frozen {
        " (frozen)"
    } else {
        ""
    };
    let brand = world.server_brand().unwrap_or("unknown");

    Some(format!("\"{brand}\" server{run_status}, 0 tx, 0 rx"))
}

fn hud_debug_light_line(world: &WorldStore, camera_pose: CameraPose) -> Option<String> {
    let feet_pos = camera_feet_block_position(camera_pose)?;
    let light = world.sample_block_light(feet_pos)?;
    let raw = light.sky.max(light.block);
    Some(format!(
        "Client Light: {raw} ({} sky, {} block)",
        light.sky, light.block
    ))
}

fn hud_debug_heightmap_lines(world: &WorldStore, camera_pose: CameraPose) -> Option<Vec<String>> {
    let feet_pos = camera_feet_block_position(camera_pose)?;
    world.probe_chunk(ChunkPos {
        x: feet_pos.x.div_euclid(16),
        z: feet_pos.z.div_euclid(16),
    })?;
    let client_height = |kind_id| hud_debug_heightmap_value(world, kind_id, feet_pos.x, feet_pos.z);
    Some(vec![
        format!(
            "CH S: {} M: {} ML: {}",
            client_height(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID),
            client_height(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID),
            client_height(VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID)
        ),
        // Server heights come from the integrated server's chunk, which bbb never has.
        "SH S: ?? O: ?? M: ?? ML: ??".to_string(),
    ])
}

fn hud_debug_heightmap_value(world: &WorldStore, kind_id: i32, x: i32, z: i32) -> String {
    world
        .sample_heightmap_first_available(kind_id, x, z)
        .map(|height| height.to_string())
        .unwrap_or_else(|| "??".to_string())
}

fn hud_debug_biome_line(world: &WorldStore, camera_pose: CameraPose) -> Option<String> {
    let feet_pos = camera_feet_block_position(camera_pose)?;
    let biome_id = world.probe_block(feet_pos)?.biome_id?;
    let biome = world
        .registry_resource_id_by_holder_id("minecraft:worldgen/biome", biome_id)
        .map(str::to_string)
        .unwrap_or_else(|| format!("[unregistered {biome_id}]"));
    Some(format!("Biome: {biome}"))
}

fn hud_debug_looking_at_block_state_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_block_hit_from_camera(world, camera_pose)?;
    let probe = world.probe_block(hit.pos)?;
    let block_name = probe.block_name?;
    let mut lines = vec![
        format!(
            "Targeted Block: {}, {}, {}",
            hit.pos.x, hit.pos.y, hit.pos.z
        ),
        block_name,
    ];
    lines.extend(
        probe
            .block_properties
            .into_iter()
            .map(|(name, value)| format!("{name}: {value}")),
    );
    Some(lines)
}

fn hud_debug_looking_at_block_tag_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_block_hit_from_camera(world, camera_pose)?;
    let block_name = world.probe_block(hit.pos)?.block_name?;
    let block_id = world.registry_holder_id_by_resource_id("minecraft:block", &block_name)?;
    Some(
        world
            .registry_tag_names_containing_holder_id("minecraft:block", block_id)?
            .map(|tag| format!("#{tag}"))
            .collect(),
    )
}

fn hud_debug_looking_at_fluid_state_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_fluid_hit_from_camera(world, camera_pose)?;
    let fluid = world.probe_block(hit.pos)?.fluid?;
    let mut lines = vec![
        format!(
            "Targeted Fluid: {}, {}, {}",
            hit.pos.x, hit.pos.y, hit.pos.z
        ),
        hud_debug_fluid_state_name(fluid).to_string(),
        format!("falling: {}", fluid.falling),
    ];
    if fluid.amount < 8 || fluid.falling {
        lines.push(format!("level: {}", fluid.amount));
    }
    Some(lines)
}

fn hud_debug_looking_at_fluid_tag_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_fluid_hit_from_camera(world, camera_pose)?;
    let fluid = world.probe_block(hit.pos)?.fluid?;
    let fluid_name = hud_debug_fluid_state_name(fluid);
    let fluid_id = world.registry_holder_id_by_resource_id("minecraft:fluid", fluid_name)?;
    Some(
        world
            .registry_tag_names_containing_holder_id("minecraft:fluid", fluid_id)?
            .map(|tag| format!("#{tag}"))
            .collect(),
    )
}

pub(super) fn hud_debug_entity_render_stats_line(
    world: &WorldStore,
    visible_entity_render_state_count: usize,
) -> String {
    let total_entity_count = world.entity_count();
    let simulation_distance = world.local_player().simulation_distance.unwrap_or(0);
    format!(
        "E: {visible_entity_render_state_count}/{total_entity_count}, SD: {simulation_distance}"
    )
}

pub(super) fn hud_debug_post_effect_line(current_post_effect_id: Option<&str>) -> Option<String> {
    Some(format!("Post: {}", current_post_effect_id?))
}

pub(super) fn hud_debug_sound_cache_bytes_to_mebibytes(bytes: u64) -> u64 {
    if bytes == 0 {
        0
    } else {
        bytes.saturating_add(1024 * 1024 - 1) / (1024 * 1024)
    }
}

pub(super) fn hud_debug_entity_type_name(
    world: &WorldStore,
    entity_type_id: i32,
) -> Option<String> {
    world
        .registry_resource_id_by_holder_id("minecraft:entity_type", entity_type_id)
        .map(str::to_string)
}

fn hud_debug_fluid_state_name(fluid: TerrainFluidState) -> &'static str {
    match (fluid.kind, fluid.amount >= 8 && !fluid.falling) {
        (TerrainFluidKind::Water, true) => "minecraft:water",
        (TerrainFluidKind::Water, false) => "minecraft:flowing_water",
        (TerrainFluidKind::Lava, true) => "minecraft:lava",
        (TerrainFluidKind::Lava, false) => "minecraft:flowing_lava",
    }
}

fn camera_feet_block_position(camera: CameraPose) -> Option<BlockPos> {
    let [x, y, z] = camera.position;
    if ![x, y, z].into_iter().all(f32::is_finite) {
        return None;
    }
    Some(BlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudDebugMemorySnapshot {
    pub(super) used_mib: u64,
    pub(super) max_mib: u64,
    pub(super) allocated_mib: u64,
    pub(super) allocation_rate_mib_per_s: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudDebugMemoryUsageSnapshot {
    pub(super) init_mib: u64,
    pub(super) used_mib: u64,
    pub(super) committed_mib: u64,
    pub(super) max_mib: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudDebugDetailedMemorySnapshot {
    pub(super) heap: HudDebugMemoryUsageSnapshot,
    pub(super) non_heap: HudDebugMemoryUsageSnapshot,
}

fn hud_debug_memory_snapshot() -> HudDebugMemorySnapshot {
    let used_kib = proc_kib_field("/proc/self/status", "VmRSS:").unwrap_or(0);
    let allocated_kib = proc_kib_field("/proc/self/status", "VmSize:").unwrap_or(used_kib);
    let max_kib =
        proc_kib_field("/proc/meminfo", "MemTotal:").unwrap_or_else(|| allocated_kib.max(used_kib));

    HudDebugMemorySnapshot {
        used_mib: kib_to_mib(used_kib),
        max_mib: kib_to_mib(max_kib),
        allocated_mib: kib_to_mib(allocated_kib),
        allocation_rate_mib_per_s: 0,
    }
}

pub(super) fn hud_debug_memory_lines(snapshot: HudDebugMemorySnapshot) -> Vec<String> {
    vec![
        format!(
            "Mem: {:2}% {:03}/{:03}MiB",
            debug_percent(snapshot.used_mib, snapshot.max_mib),
            snapshot.used_mib,
            snapshot.max_mib
        ),
        format!(
            "Allocation rate: {:03}MiB/s",
            snapshot.allocation_rate_mib_per_s
        ),
        format!(
            "Allocated: {:2}% {:03}MiB",
            debug_percent(snapshot.allocated_mib, snapshot.max_mib),
            snapshot.allocated_mib
        ),
    ]
}

fn hud_debug_detailed_memory_snapshot() -> HudDebugDetailedMemorySnapshot {
    let status_path = "/proc/self/status";
    let used_kib = proc_kib_field(status_path, "VmRSS:").unwrap_or(0);
    let committed_kib = proc_kib_field(status_path, "VmSize:").unwrap_or(used_kib);
    let max_kib =
        proc_kib_field("/proc/meminfo", "MemTotal:").unwrap_or_else(|| committed_kib.max(used_kib));
    let non_heap_used_kib = proc_kib_field(status_path, "VmExe:")
        .unwrap_or(0)
        .saturating_add(proc_kib_field(status_path, "VmLib:").unwrap_or(0))
        .saturating_add(proc_kib_field(status_path, "VmStk:").unwrap_or(0));

    HudDebugDetailedMemorySnapshot {
        heap: HudDebugMemoryUsageSnapshot {
            init_mib: 0,
            used_mib: kib_to_mib(used_kib),
            committed_mib: kib_to_mib(committed_kib),
            max_mib: kib_to_mib(max_kib),
        },
        non_heap: HudDebugMemoryUsageSnapshot {
            init_mib: 0,
            used_mib: kib_to_mib(non_heap_used_kib),
            committed_mib: kib_to_mib(non_heap_used_kib),
            max_mib: 0,
        },
    }
}

pub(super) fn hud_debug_detailed_memory_lines(
    snapshot: HudDebugDetailedMemorySnapshot,
) -> Vec<String> {
    vec![
        hud_debug_memory_usage_line("heap", snapshot.heap),
        hud_debug_memory_usage_line("non-heap", snapshot.non_heap),
    ]
}

fn hud_debug_memory_usage_line(kind: &str, usage: HudDebugMemoryUsageSnapshot) -> String {
    format!(
        "Memory ({kind}): i={:03}MiB u={:03}MiB c={:03}MiB m={:03}MiB",
        usage.init_mib, usage.used_mib, usage.committed_mib, usage.max_mib
    )
}

fn hud_debug_system_lines(surface_size: winit::dpi::PhysicalSize<u32>) -> Vec<String> {
    let cpu_threads = std::thread::available_parallelism()
        .map(|threads| threads.get())
        .unwrap_or(1);

    vec![
        format!("Java: n/a (bbb-client {})", env!("CARGO_PKG_VERSION")),
        format!(
            "CPU: {}x {}-{}",
            cpu_threads,
            std::env::consts::ARCH,
            std::env::consts::OS
        ),
        format!(
            "Display: {}x{} (wgpu)",
            surface_size.width, surface_size.height
        ),
        "bbb renderer".to_string(),
        "wgpu native".to_string(),
    ]
}

pub(super) fn hud_debug_gpu_utilization_line(gpu_utilization: Option<f64>) -> String {
    let Some(utilization) = gpu_utilization.filter(|value| value.is_finite()) else {
        return "GPU: N/A".to_string();
    };
    format!("GPU: {:.0}%", utilization.clamp(0.0, 100.0))
}

fn debug_percent(value: u64, total: u64) -> u64 {
    value.saturating_mul(100).checked_div(total).unwrap_or(0)
}

fn kib_to_mib(kib: u64) -> u64 {
    kib / 1024
}

fn proc_kib_field(path: &str, field: &str) -> Option<u64> {
    fs::read_to_string(path).ok()?.lines().find_map(|line| {
        let rest = line.strip_prefix(field)?;
        rest.split_whitespace().next()?.parse().ok()
    })
}

fn hud_debug_position_lines(world: &WorldStore, camera_pose: CameraPose) -> Vec<String> {
    let x = f64::from(camera_pose.position[0]);
    let y = f64::from(camera_pose.position[1]);
    let z = f64::from(camera_pose.position[2]);
    let block_x = x.floor() as i32;
    let block_y = y.floor() as i32;
    let block_z = z.floor() as i32;
    let chunk_x = block_x.div_euclid(16);
    let chunk_z = block_z.div_euclid(16);
    let section_y = block_y.div_euclid(16);
    let (direction, face_string) = hud_debug_facing(camera_pose.y_rot);
    let dimension = world
        .level_info()
        .map(|level| level.dimension.as_str())
        .unwrap_or("minecraft:overworld");

    vec![
        format!("XYZ: {x:.3} / {y:.5} / {z:.3}"),
        format!("Block: {block_x} {block_y} {block_z}"),
        format!(
            "Chunk: {chunk_x} {section_y} {chunk_z} [{} {} in r.{}.{}.mca]",
            chunk_x.rem_euclid(32),
            chunk_z.rem_euclid(32),
            chunk_x.div_euclid(32),
            chunk_z.div_euclid(32)
        ),
        format!(
            "Facing: {direction} ({face_string}) ({:.1} / {:.1})",
            hud_debug_wrap_degrees(camera_pose.y_rot),
            hud_debug_wrap_degrees(camera_pose.x_rot)
        ),
        format!("{dimension} FC: 0"),
    ]
}

fn hud_debug_section_position_line(camera_pose: CameraPose) -> String {
    let block_x = f64::from(camera_pose.position[0]).floor() as i32;
    let block_y = f64::from(camera_pose.position[1]).floor() as i32;
    let block_z = f64::from(camera_pose.position[2]).floor() as i32;

    format!(
        "Section-relative: {:02} {:02} {:02}",
        block_x & 15,
        block_y & 15,
        block_z & 15
    )
}

fn hud_debug_facing(y_rot: f32) -> (&'static str, &'static str) {
    let wrapped = hud_debug_wrap_degrees(y_rot);
    if (-45.0..45.0).contains(&wrapped) {
        ("south", "Towards positive Z")
    } else if (45.0..135.0).contains(&wrapped) {
        ("west", "Towards negative X")
    } else if (-135.0..-45.0).contains(&wrapped) {
        ("east", "Towards positive X")
    } else {
        ("north", "Towards negative Z")
    }
}

fn hud_debug_wrap_degrees(degrees: f32) -> f32 {
    let wrapped = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 {
        180.0
    } else {
        wrapped
    }
}

fn hud_info_status_lines(cx: &HudDebugOverlayInputs<'_>) -> [String; 2] {
    let fps = cx.fps_sampler.fps();
    let (pos, y_rot) = cx
        .camera_pose
        .map_or(([0.0_f32; 3], 0.0_f32), |pose| (pose.position, pose.y_rot));
    let facing = match hud_debug_facing(y_rot).0 {
        "north" => "N",
        "south" => "S",
        "east" => "E",
        _ => "W",
    };
    let biome = cx
        .camera_pose
        .and_then(camera_feet_block_position)
        .and_then(|feet| cx.world.probe_block(feet)?.biome_id)
        .and_then(|id| {
            cx.world
                .registry_resource_id_by_holder_id("minecraft:worldgen/biome", id)
        })
        .map(|id| id.strip_prefix("minecraft:").unwrap_or(id))
        .unwrap_or("unknown");
    let line1 = format!(
        "{fps} FPS | XYZ: {:.1} {:.1} {:.1} ({facing}) | {biome}",
        pos[0], pos[1], pos[2]
    );

    let day_time = cx.world.world_time().map_or(0_i64, |t| t.day_time);
    let day = day_time.div_euclid(VANILLA_OVERWORLD_DAY_PERIOD_TICKS) + 1;
    let day_ticks = day_time.rem_euclid(VANILLA_OVERWORLD_DAY_PERIOD_TICKS);
    let total_minutes = ((day_ticks + 6_000) % VANILLA_OVERWORLD_DAY_PERIOD_TICKS) * 60 / 1_000;
    let hh = total_minutes / 60;
    let mm = total_minutes % 60;
    let selected_slot = usize::from(cx.world.local_player().selected_hotbar_slot.min(8));
    let durability_suffix = cx
        .world
        .inventory()
        .hotbar_item_states()
        .get(selected_slot)
        .and_then(|slot| slot.as_ref())
        .and_then(|hotbar| {
            let effective = cx.world.effective_item_stack(hotbar.item);
            bbb_client_world::ItemProfiles::damage_state(
                Some(cx.world.item_profiles()),
                effective.item_id,
                &effective.component_patch,
            )
        })
        .map(|(damage, max_damage)| {
            format!(" | Mainhand: {}/{max_damage}", (max_damage - damage).max(0))
        })
        .unwrap_or_default();
    let line2 = format!("Day {day} {hh:02}:{mm:02}{durability_suffix}");
    [line1, line2]
}
