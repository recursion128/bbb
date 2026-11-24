use super::*;

pub(super) const CHUNK_BORDER_NEIGHBOR_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 0.5];
pub(super) const CHUNK_BORDER_CELL_COLOR: [f32; 4] = [0.0, 155.0 / 255.0, 155.0 / 255.0, 1.0];
pub(super) const CHUNK_BORDER_YELLOW_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
pub(super) const CHUNK_BORDER_MAJOR_COLOR: [f32; 4] = [0.25, 0.25, 1.0, 1.0];
pub(super) const CHUNK_BORDER_THICK_LINE_WIDTH: f32 = 4.0;
pub(super) const CHUNK_BORDER_THIN_LINE_WIDTH: f32 = 1.0;
const DEBUG_HEIGHTMAP_CHUNK_DISTANCE: i32 = 2;
pub(super) const DEBUG_HEIGHTMAP_BOX_HEIGHT: f32 = 3.0 / 32.0;
pub(super) const DEBUG_WATER_SCAN_RADIUS: i32 = 10;
pub(super) const DEBUG_WATER_BOX_INSET: f32 = 0.01;
pub(super) const DEBUG_WATER_BOX_COLOR: [u8; 4] = [0, 255, 0, 38];
pub(super) fn debug_entity_scene_outline(
    input: &ClientInputState,
    world: &WorldStore,
    entity_partial_tick: f32,
) -> Option<SelectionOutline> {
    if !input.debug_entity_hitboxes_visible_for_world(world) {
        return None;
    }
    if input.debug_show_local_server_entity_hit_boxes() {
        entity_scene_outline_from_world_at_partial_tick_with_server_details(
            world,
            entity_partial_tick,
            true,
        )
    } else {
        entity_scene_outline_from_world_at_partial_tick(world, entity_partial_tick)
    }
}

pub(super) fn debug_chunk_border_outline(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<SelectionOutline> {
    if !input.debug_chunk_borders_visible_for_world(world) {
        return None;
    }
    world.level_info()?;
    let camera_pose = camera_pose?;
    let dimension = world.dimension();
    let min_y = dimension.min_y;
    let max_y = min_y + dimension.height;
    if max_y <= min_y {
        return None;
    }
    let x_start = chunk_min_block(camera_pose.position[0]);
    let z_start = chunk_min_block(camera_pose.position[2]);
    let y_start = chunk_min_block(camera_pose.position[1]);
    let mut lines = Vec::new();

    for x in (-16..=32).step_by(16) {
        for z in (-16..=32).step_by(16) {
            push_chunk_border_line(
                &mut lines,
                [x_start + x, min_y, z_start + z],
                [x_start + x, max_y, z_start + z],
                CHUNK_BORDER_NEIGHBOR_COLOR,
                CHUNK_BORDER_THICK_LINE_WIDTH,
            );
        }
    }

    for x in (2..16).step_by(2) {
        let color = if x % 4 == 0 {
            CHUNK_BORDER_CELL_COLOR
        } else {
            CHUNK_BORDER_YELLOW_COLOR
        };
        push_chunk_border_line(
            &mut lines,
            [x_start + x, min_y, z_start],
            [x_start + x, max_y, z_start],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
        push_chunk_border_line(
            &mut lines,
            [x_start + x, min_y, z_start + 16],
            [x_start + x, max_y, z_start + 16],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
    }

    for z in (2..16).step_by(2) {
        let color = if z % 4 == 0 {
            CHUNK_BORDER_CELL_COLOR
        } else {
            CHUNK_BORDER_YELLOW_COLOR
        };
        push_chunk_border_line(
            &mut lines,
            [x_start, min_y, z_start + z],
            [x_start, max_y, z_start + z],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
        push_chunk_border_line(
            &mut lines,
            [x_start + 16, min_y, z_start + z],
            [x_start + 16, max_y, z_start + z],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
    }

    for y in (min_y..=max_y).step_by(2) {
        let color = if y % 8 == 0 {
            CHUNK_BORDER_CELL_COLOR
        } else {
            CHUNK_BORDER_YELLOW_COLOR
        };
        push_chunk_border_horizontal_ring(
            &mut lines,
            x_start,
            z_start,
            y,
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
    }

    for x in (0..=16).step_by(16) {
        for z in (0..=16).step_by(16) {
            push_chunk_border_line(
                &mut lines,
                [x_start + x, min_y, z_start + z],
                [x_start + x, max_y, z_start + z],
                CHUNK_BORDER_MAJOR_COLOR,
                CHUNK_BORDER_THICK_LINE_WIDTH,
            );
        }
    }

    for y in (min_y..=max_y).step_by(16) {
        push_chunk_border_horizontal_ring(
            &mut lines,
            x_start,
            z_start,
            y,
            CHUNK_BORDER_MAJOR_COLOR,
            CHUNK_BORDER_THICK_LINE_WIDTH,
        );
    }

    Some(SelectionOutline {
        boxes: Vec::new(),
        colored_boxes: vec![SelectionColoredBox {
            min: [x_start as f32, y_start as f32, z_start as f32],
            max: [
                (x_start + 16) as f32,
                (y_start + 16) as f32,
                (z_start + 16) as f32,
            ],
            color: CHUNK_BORDER_MAJOR_COLOR,
            line_width: CHUNK_BORDER_THIN_LINE_WIDTH,
            always_on_top: true,
        }],
        lines,
        points: Vec::new(),
        text_labels: Vec::new(),
    })
}

pub(super) fn debug_heightmap_boxes(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Vec<DebugFilledBox> {
    if !input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeHeightmap,
        input.show_only_reduced_info(world),
    ) || world.level_info().is_none()
    {
        return Vec::new();
    }
    let Some(camera_pose) = camera_pose else {
        return Vec::new();
    };
    if !camera_pose.position[0].is_finite() || !camera_pose.position[2].is_finite() {
        return Vec::new();
    }

    let center = ChunkPos {
        x: (camera_pose.position[0].floor() as i32).div_euclid(16),
        z: (camera_pose.position[2].floor() as i32).div_euclid(16),
    };
    let mut boxes = Vec::new();
    for chunk_offset_x in -DEBUG_HEIGHTMAP_CHUNK_DISTANCE..=DEBUG_HEIGHTMAP_CHUNK_DISTANCE {
        for chunk_offset_z in -DEBUG_HEIGHTMAP_CHUNK_DISTANCE..=DEBUG_HEIGHTMAP_CHUNK_DISTANCE {
            let chunk_pos = ChunkPos {
                x: center.x.saturating_add(chunk_offset_x),
                z: center.z.saturating_add(chunk_offset_z),
            };
            let Some(sampler) = world.chunk_heightmap_sampler(chunk_pos) else {
                continue;
            };
            let chunk_min_x = chunk_pos.x.saturating_mul(16);
            let chunk_min_z = chunk_pos.z.saturating_mul(16);
            for kind_id in sampler.effective_kind_ids() {
                let Some(color) = vanilla_heightmap_type(kind_id)
                    .map(|heightmap_type| heightmap_type.debug_color_rgba)
                else {
                    continue;
                };
                for local_x in 0_u8..16 {
                    for local_z in 0_u8..16 {
                        let Some(first_available) =
                            sampler.first_available(kind_id, local_x, local_z)
                        else {
                            continue;
                        };
                        let x = chunk_min_x.saturating_add(i32::from(local_x)) as f32;
                        let z = chunk_min_z.saturating_add(i32::from(local_z)) as f32;
                        let y =
                            first_available as f32 + kind_id as f32 * DEBUG_HEIGHTMAP_BOX_HEIGHT;
                        boxes.push(DebugFilledBox {
                            min: [x + 0.25, y, z + 0.25],
                            max: [x + 0.75, y + DEBUG_HEIGHTMAP_BOX_HEIGHT, z + 0.75],
                            color,
                        });
                    }
                }
            }
        }
    }
    boxes
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DebugWaterSample {
    pos: BlockPos,
    amount: u8,
    height: f64,
}

pub(super) fn debug_water_samples(
    input: &ClientInputState,
    world: &WorldStore,
) -> Vec<DebugWaterSample> {
    if !input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeWaterLevels,
        input.show_only_reduced_info(world),
    ) || world.level_info().is_none()
    {
        return Vec::new();
    }
    // Vanilla WaterDebugRenderer anchors the 21^3 scan on
    // `minecraft.player.blockPosition()`, not the active camera entity.
    let Some(player_pose) = world.local_player_pose() else {
        return Vec::new();
    };
    let player_position = [
        player_pose.position.x,
        player_pose.position.y,
        player_pose.position.z,
    ];
    if !player_position
        .iter()
        .all(|coordinate| coordinate.is_finite())
    {
        return Vec::new();
    }
    let center = BlockPos {
        x: player_position[0].floor() as i32,
        y: player_position[1].floor() as i32,
        z: player_position[2].floor() as i32,
    };
    let min = BlockPos {
        x: center.x.saturating_sub(DEBUG_WATER_SCAN_RADIUS),
        y: center.y.saturating_sub(DEBUG_WATER_SCAN_RADIUS),
        z: center.z.saturating_sub(DEBUG_WATER_SCAN_RADIUS),
    };
    let max = BlockPos {
        x: center.x.saturating_add(DEBUG_WATER_SCAN_RADIUS),
        y: center.y.saturating_add(DEBUG_WATER_SCAN_RADIUS),
        z: center.z.saturating_add(DEBUG_WATER_SCAN_RADIUS),
    };

    let mut samples = Vec::new();
    // `BlockPos.betweenClosed`: x advances fastest, then y, then z.
    for z in min.z..=max.z {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let pos = BlockPos { x, y, z };
                let Some((fluid, height)) = world.fluid_state_and_height_at(pos) else {
                    continue;
                };
                if fluid.kind != TerrainFluidKind::Water {
                    continue;
                }
                samples.push(DebugWaterSample {
                    pos,
                    amount: fluid.amount,
                    height,
                });
            }
        }
    }
    samples
}

pub(super) fn debug_water_boxes(samples: &[DebugWaterSample]) -> Vec<DebugFilledBox> {
    samples
        .iter()
        .map(|sample| {
            let BlockPos { x, y, z } = sample.pos;
            DebugFilledBox {
                min: [
                    x as f32 + DEBUG_WATER_BOX_INSET,
                    y as f32 + DEBUG_WATER_BOX_INSET,
                    z as f32 + DEBUG_WATER_BOX_INSET,
                ],
                max: [
                    x as f32 + (1.0 - DEBUG_WATER_BOX_INSET),
                    y as f32 + sample.height as f32,
                    z as f32 + (1.0 - DEBUG_WATER_BOX_INSET),
                ],
                color: DEBUG_WATER_BOX_COLOR,
            }
        })
        .collect()
}

pub(super) fn debug_water_amount_billboards(
    samples: &[DebugWaterSample],
) -> Vec<DebugTextBillboard> {
    samples
        .iter()
        .map(|sample| {
            let BlockPos { x, y, z } = sample.pos;
            DebugTextBillboard::centered(
                [
                    x as f32 + 0.5,
                    y as f32 + sample.height as f32,
                    z as f32 + 0.5,
                ],
                sample.amount.to_string(),
                [0, 0, 0, 255],
            )
        })
        .collect()
}

pub(super) const DEBUG_LIGHT_SCAN_RADIUS: i32 = 10;
pub(super) const DEBUG_LIGHT_SECTION_LABEL_SCALE: f32 = 4.8;
pub(super) const DEBUG_LIGHT_SECTION_LABEL_COLOR: [u8; 4] = [255, 0, 0, 255];
const DEBUG_LIGHT_SKY_FROM_COLOR: i32 = argb_color(255, 0, 0, 255);
const DEBUG_LIGHT_SKY_TO_COLOR: i32 = argb_color(255, 0, 255, 255);
const DEBUG_LIGHT_BLOCK_FROM_COLOR: i32 = argb_color(255, 170, 0, 0);
const DEBUG_LIGHT_BLOCK_TO_COLOR: i32 = argb_color(255, 255, 255, 0);

pub(super) fn debug_light_level_billboards(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Vec<DebugTextBillboard> {
    let reduced_debug_info = input.show_only_reduced_info(world);
    let show_block_light = input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeBlockLightLevels,
        reduced_debug_info,
    );
    let show_sky_light = input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeSkyLightLevels,
        reduced_debug_info,
    );
    if (!show_block_light && !show_sky_light) || world.level_info().is_none() {
        return Vec::new();
    }
    let Some(camera_eye) = camera_pose.map(camera_eye_position) else {
        return Vec::new();
    };
    if !camera_eye.iter().all(|coordinate| coordinate.is_finite()) {
        return Vec::new();
    }
    let center = BlockPos {
        x: camera_eye[0].floor() as i32,
        y: camera_eye[1].floor() as i32,
        z: camera_eye[2].floor() as i32,
    };
    let has_sky_light = world
        .level_info()
        .is_some_and(bbb_client_world::WorldLevelInfo::has_sky_light);
    let mut seen_sections = Vec::new();
    let mut billboards = Vec::new();
    // `BlockPos.betweenClosed`: x advances fastest, then y, then z.
    for z in center.z.saturating_sub(DEBUG_LIGHT_SCAN_RADIUS)
        ..=center.z.saturating_add(DEBUG_LIGHT_SCAN_RADIUS)
    {
        for y in center.y.saturating_sub(DEBUG_LIGHT_SCAN_RADIUS)
            ..=center.y.saturating_add(DEBUG_LIGHT_SCAN_RADIUS)
        {
            for x in center.x.saturating_sub(DEBUG_LIGHT_SCAN_RADIUS)
                ..=center.x.saturating_add(DEBUG_LIGHT_SCAN_RADIUS)
            {
                let pos = BlockPos { x, y, z };
                let section = [x.div_euclid(16), y.div_euclid(16), z.div_euclid(16)];
                if !seen_sections.contains(&section) {
                    seen_sections.push(section);
                    let mut billboard = DebugTextBillboard::centered(
                        section.map(|coordinate| {
                            coordinate.saturating_mul(16).saturating_add(8) as f32
                        }),
                        world.debug_sky_light_section_status(section),
                        DEBUG_LIGHT_SECTION_LABEL_COLOR,
                    );
                    billboard.scale = DEBUG_LIGHT_SECTION_LABEL_SCALE;
                    billboards.push(billboard);
                }

                let light = world
                    .sample_debug_light(pos)
                    .unwrap_or(TerrainLight::FULL_BRIGHT);
                let sky_brightness = if has_sky_light { light.sky } else { 0 };
                if show_sky_light && sky_brightness != 15 {
                    let color = argb_srgb_lerp(
                        f32::from(sky_brightness) / 15.0,
                        DEBUG_LIGHT_SKY_FROM_COLOR,
                        DEBUG_LIGHT_SKY_TO_COLOR,
                    );
                    billboards.push(DebugTextBillboard::centered(
                        [x as f32 + 0.5, y as f32 + 0.25, z as f32 + 0.5],
                        sky_brightness.to_string(),
                        argb_to_rgba8(color),
                    ));
                }
                if show_block_light && light.block != 0 {
                    let color = argb_srgb_lerp(
                        f32::from(light.block) / 15.0,
                        DEBUG_LIGHT_BLOCK_FROM_COLOR,
                        DEBUG_LIGHT_BLOCK_TO_COLOR,
                    );
                    billboards.push(DebugTextBillboard::centered(
                        [x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5],
                        light.block.to_string(),
                        argb_to_rgba8(color),
                    ));
                }
            }
        }
    }
    billboards
}

pub(super) fn debug_selection_text_billboards(
    outline: Option<&SelectionOutline>,
) -> Vec<DebugTextBillboard> {
    outline
        .into_iter()
        .flat_map(|outline| &outline.text_labels)
        .map(|label| {
            let mut billboard = DebugTextBillboard::centered(
                label.position,
                label.text.clone(),
                argb_to_rgba8(rgba01_to_argb(label.color)),
            );
            if !label.centered {
                billboard.adjust_left = Some(0.0);
            }
            billboard
        })
        .collect()
}

pub(super) const DEBUG_SOLID_FACE_SCAN_RADIUS: i32 = 6;
pub(super) const DEBUG_SOLID_FACE_INFLATE: f32 = 0.002;
pub(super) const DEBUG_SOLID_FACE_COLOR: [u8; 4] = [255, 0, 0, 128];
const DEBUG_SOLID_FACE_DIRECTIONS: [DebugCuboidFace; 6] = [
    DebugCuboidFace::West,
    DebugCuboidFace::South,
    DebugCuboidFace::East,
    DebugCuboidFace::North,
    DebugCuboidFace::Down,
    DebugCuboidFace::Up,
];

pub(super) fn debug_solid_face_rects(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Vec<DebugFilledRect> {
    if !input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeSolidFaces,
        input.show_only_reduced_info(world),
    ) || world.level_info().is_none()
    {
        return Vec::new();
    }
    let Some(camera_position) = camera_pose.map(camera_eye_position) else {
        return Vec::new();
    };
    if !camera_position
        .iter()
        .all(|coordinate| coordinate.is_finite())
    {
        return Vec::new();
    }
    let center = BlockPos {
        x: camera_position[0].floor() as i32,
        y: camera_position[1].floor() as i32,
        z: camera_position[2].floor() as i32,
    };
    let mut rects = Vec::new();
    // `BlockPos.betweenClosed`: x advances fastest, then y, then z.
    for z in center.z.saturating_sub(DEBUG_SOLID_FACE_SCAN_RADIUS)
        ..=center.z.saturating_add(DEBUG_SOLID_FACE_SCAN_RADIUS)
    {
        for y in center.y.saturating_sub(DEBUG_SOLID_FACE_SCAN_RADIUS)
            ..=center.y.saturating_add(DEBUG_SOLID_FACE_SCAN_RADIUS)
        {
            for x in center.x.saturating_sub(DEBUG_SOLID_FACE_SCAN_RADIUS)
                ..=center.x.saturating_add(DEBUG_SOLID_FACE_SCAN_RADIUS)
            {
                let pos = BlockPos { x, y, z };
                let Some(probe) = world.probe_block(pos) else {
                    continue;
                };
                // Vanilla skips only `Blocks.AIR`; the other air variants
                // naturally contribute no outline AABBs.
                if probe.block_name.as_deref() == Some("minecraft:air") {
                    continue;
                }
                let sturdy_faces = world.debug_block_sturdy_faces(&probe);
                if !sturdy_faces.contains(&true) {
                    continue;
                }
                let outline_boxes = if probe.block_name.as_deref() == Some("minecraft:scaffolding")
                {
                    world.debug_scaffolding_outline_shape_boxes(&probe)
                } else {
                    bbb_client_world::block_outline::block_probe_shape_boxes(&probe)
                };
                let Some(outline_boxes) = outline_boxes else {
                    continue;
                };
                for (outline_min, outline_max) in outline_boxes {
                    let min = [
                        x as f32 + outline_min[0] as f32 - DEBUG_SOLID_FACE_INFLATE,
                        y as f32 + outline_min[1] as f32 - DEBUG_SOLID_FACE_INFLATE,
                        z as f32 + outline_min[2] as f32 - DEBUG_SOLID_FACE_INFLATE,
                    ];
                    let max = [
                        x as f32 + outline_max[0] as f32 + DEBUG_SOLID_FACE_INFLATE,
                        y as f32 + outline_max[1] as f32 + DEBUG_SOLID_FACE_INFLATE,
                        z as f32 + outline_max[2] as f32 + DEBUG_SOLID_FACE_INFLATE,
                    ];
                    for (face, sturdy) in DEBUG_SOLID_FACE_DIRECTIONS.into_iter().zip(sturdy_faces)
                    {
                        if sturdy {
                            rects.push(DebugFilledRect::cuboid_face(
                                min,
                                max,
                                face,
                                DEBUG_SOLID_FACE_COLOR,
                            ));
                        }
                    }
                }
            }
        }
    }
    rects
}

pub(super) const DEBUG_COLLISION_QUERY_RADIUS: f64 = 6.0;
pub(super) const DEBUG_COLLISION_COLOR: [u8; 4] = [255; 4];
pub(super) const DEBUG_SUPPORTING_BLOCK_OUTLINE_ALPHA: u8 = 102;
const DEBUG_SUPPORTING_BLOCK_PLAYER_COLOR: [u8; 4] = [255, 0, 0, 255];
pub(super) const DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR: [u8; 4] = [0, 255, 0, 255];
const DEBUG_SUPPORTING_BLOCK_LEGACY_COLOR: [u8; 4] = [0, 255, 255, 255];
pub(super) const DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET: f64 = 0.02;
pub(super) const DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET: f64 = 0.04;

pub(super) fn debug_collision_entry_enabled(input: &ClientInputState, world: &WorldStore) -> bool {
    input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        input.show_only_reduced_info(world),
    )
}

pub(super) fn collect_debug_collision_lines(world: &WorldStore) -> Vec<DebugLine> {
    world
        .debug_camera_collision_boxes(DEBUG_COLLISION_QUERY_RADIUS)
        .into_iter()
        .flat_map(|collision_box| {
            DebugLine::cuboid(
                collision_box.min.map(|coordinate| coordinate as f32),
                collision_box.max.map(|coordinate| coordinate as f32),
                DEBUG_COLLISION_COLOR,
                DebugLine::DEFAULT_WIDTH,
            )
        })
        .collect()
}

pub(super) fn debug_supporting_block_entry_enabled(
    input: &ClientInputState,
    world: &WorldStore,
) -> bool {
    input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        input.show_only_reduced_info(world),
    )
}

pub(super) fn collect_debug_supporting_block_lines(
    world: &WorldStore,
    cached_entity_ids: &[i32],
) -> Vec<DebugLine> {
    let mut lines = Vec::new();
    if let Some(debug) = world.debug_local_player_supporting_blocks() {
        append_debug_supporting_block_highlight(
            &mut lines,
            &debug.supporting,
            DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET,
            DEBUG_SUPPORTING_BLOCK_PLAYER_COLOR,
        );
        if let Some(legacy_effect) = debug.legacy_effect.as_ref() {
            append_debug_supporting_block_highlight(
                &mut lines,
                legacy_effect,
                DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET,
                DEBUG_SUPPORTING_BLOCK_LEGACY_COLOR,
            );
        }
    }
    for entity_id in cached_entity_ids {
        let Some(debug) = world.debug_entity_supporting_blocks(*entity_id) else {
            continue;
        };
        let bias = debug_supporting_block_entity_bias(*entity_id);
        append_debug_supporting_block_highlight(
            &mut lines,
            &debug.supporting,
            DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET + bias,
            DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR,
        );
        if let Some(legacy_effect) = debug.legacy_effect.as_ref() {
            append_debug_supporting_block_highlight(
                &mut lines,
                legacy_effect,
                DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET + bias,
                DEBUG_SUPPORTING_BLOCK_LEGACY_COLOR,
            );
        }
    }
    lines
}

pub(super) fn debug_supporting_block_entity_bias(entity_id: i32) -> f64 {
    let hash_input = (f64::from(entity_id) + 0.132_453_657).to_string();
    0.02 * f64::from(java_string_hash_code(&hash_input) % 1000) / 1000.0
}

fn append_debug_supporting_block_highlight(
    lines: &mut Vec<DebugLine>,
    highlight: &bbb_client_world::DebugSupportingBlockHighlight,
    offset: f64,
    color: [u8; 4],
) {
    let BlockPos { x, y, z } = highlight.pos;
    let outline_offset = 2.0 * offset;
    let outline_color = [
        color[0],
        color[1],
        color[2],
        DEBUG_SUPPORTING_BLOCK_OUTLINE_ALPHA,
    ];
    lines.extend(DebugLine::cuboid(
        [
            (f64::from(x) - outline_offset) as f32,
            (f64::from(y) - outline_offset) as f32,
            (f64::from(z) - outline_offset) as f32,
        ],
        [
            (f64::from(x) + 1.0 + outline_offset) as f32,
            (f64::from(y) + 1.0 + outline_offset) as f32,
            (f64::from(z) + 1.0 + outline_offset) as f32,
        ],
        outline_color,
        DebugLine::DEFAULT_WIDTH,
    ));
    lines.extend(highlight.collision_boxes.iter().flat_map(|collision_box| {
        DebugLine::cuboid(
            collision_box.min.map(|coordinate| coordinate as f32),
            collision_box.max.map(|coordinate| coordinate as f32),
            color,
            DebugLine::DEFAULT_WIDTH,
        )
    }));
}

fn chunk_min_block(position: f32) -> i32 {
    (position.floor() as i32).div_euclid(16) * 16
}

fn push_chunk_border_horizontal_ring(
    lines: &mut Vec<SelectionLine>,
    x_start: i32,
    z_start: i32,
    y: i32,
    color: [f32; 4],
    width: f32,
) {
    push_chunk_border_line(
        lines,
        [x_start, y, z_start],
        [x_start, y, z_start + 16],
        color,
        width,
    );
    push_chunk_border_line(
        lines,
        [x_start, y, z_start + 16],
        [x_start + 16, y, z_start + 16],
        color,
        width,
    );
    push_chunk_border_line(
        lines,
        [x_start + 16, y, z_start + 16],
        [x_start + 16, y, z_start],
        color,
        width,
    );
    push_chunk_border_line(
        lines,
        [x_start + 16, y, z_start],
        [x_start, y, z_start],
        color,
        width,
    );
}

fn push_chunk_border_line(
    lines: &mut Vec<SelectionLine>,
    from: [i32; 3],
    to: [i32; 3],
    color: [f32; 4],
    width: f32,
) {
    lines.push(SelectionLine {
        from: [from[0] as f32, from[1] as f32, from[2] as f32],
        to: [to[0] as f32, to[1] as f32, to[2] as f32],
        color,
        width,
    });
}
