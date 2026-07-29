//! Startup-selected vanilla remote debug subscriptions and immutable gizmo extraction.
//!
//! The simulation owner derives one deterministic request set from this static
//! selection plus the live FPS-chart toggle. Render extraction reads the same
//! selection and the canonical values from one published `WorldStore`
//! generation. Its only mutable render-owned state is vanilla's remembered
//! brain target UUID; it never owns subscription values or mutable world data.

use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{
    debug_subscription_types::{
        VANILLA_DEBUG_SUBSCRIPTION_BEES_ID, VANILLA_DEBUG_SUBSCRIPTION_BEE_HIVES_ID,
        VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID, VANILLA_DEBUG_SUBSCRIPTION_BREEZES_ID,
        VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID,
        VANILLA_DEBUG_SUBSCRIPTION_ENTITY_BLOCK_INTERSECTIONS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID, VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID, VANILLA_DEBUG_SUBSCRIPTION_POIS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_REDSTONE_WIRE_ORIENTATIONS_ID,
        VANILLA_DEBUG_SUBSCRIPTION_STRUCTURES_ID, VANILLA_DEBUG_SUBSCRIPTION_VILLAGE_SECTIONS_ID,
    },
    packets::{
        DebugBrainDump, DebugEntityBlockIntersection, DebugPathInfo, DebugPathNode, DebugPathType,
    },
};
use bbb_renderer::{CameraPose, DebugFilledBox, DebugLine, DebugTextBillboard};
use bbb_world::{
    BlockPos, DebugBoundingBoxState, DebugSubscriptionValueState, DebugTimedValueState,
    EntityTransformState, WorldStore,
};
use clap::ValueEnum;
use uuid::Uuid;

const WHITE: [u8; 4] = argb(-1);
const GRAY: [u8; 4] = argb(-3_355_444);
const PINK: [u8; 4] = argb(-98_404);
const ORANGE: [u8; 4] = argb(-23_296);
const YELLOW: [u8; 4] = argb(-256);
const RED: [u8; 4] = argb(-65_536);
const GREEN: [u8; 4] = argb(-16_711_936);
const CYAN: [u8; 4] = argb(-16_711_681);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum RemoteDebugRenderer {
    Bees,
    Brain,
    Breeze,
    EntityBlockIntersections,
    Pathfinding,
    GameEventListeners,
    GoalSelector,
    NeighborUpdates,
    Poi,
    Raids,
    RedstoneWireOrientations,
    Structures,
    VillageSections,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RemoteDebugRendererSelection(u16);

impl RemoteDebugRendererSelection {
    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn contains(self, renderer: RemoteDebugRenderer) -> bool {
        self.0 & renderer.bit() != 0
    }

    fn needs_entity_transforms(self) -> bool {
        [
            RemoteDebugRenderer::Bees,
            RemoteDebugRenderer::Brain,
            RemoteDebugRenderer::Breeze,
            RemoteDebugRenderer::GameEventListeners,
            RemoteDebugRenderer::GoalSelector,
            RemoteDebugRenderer::Pathfinding,
            RemoteDebugRenderer::Poi,
        ]
        .into_iter()
        .any(|renderer| self.contains(renderer))
    }

    pub(crate) fn subscription_ids(self, fps_chart_visible: bool) -> Vec<i32> {
        let mut ids = BTreeSet::new();
        if fps_chart_visible {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID);
        }
        if self.contains(RemoteDebugRenderer::Bees) {
            ids.extend([
                VANILLA_DEBUG_SUBSCRIPTION_BEES_ID,
                VANILLA_DEBUG_SUBSCRIPTION_BEE_HIVES_ID,
                VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID,
            ]);
        }
        if self.contains(RemoteDebugRenderer::Brain) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID);
        }
        if self.contains(RemoteDebugRenderer::Breeze) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_BREEZES_ID);
        }
        if self.contains(RemoteDebugRenderer::EntityBlockIntersections) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_ENTITY_BLOCK_INTERSECTIONS_ID);
        }
        if self.contains(RemoteDebugRenderer::Pathfinding) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID);
        }
        if self.contains(RemoteDebugRenderer::GameEventListeners) {
            ids.extend([
                VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
                VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
            ]);
        }
        if self.contains(RemoteDebugRenderer::GoalSelector) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID);
        }
        if self.contains(RemoteDebugRenderer::NeighborUpdates) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID);
        }
        if self.contains(RemoteDebugRenderer::Poi) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_POIS_ID);
        }
        if self.contains(RemoteDebugRenderer::Raids) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID);
        }
        if self.contains(RemoteDebugRenderer::RedstoneWireOrientations) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_REDSTONE_WIRE_ORIENTATIONS_ID);
        }
        if self.contains(RemoteDebugRenderer::Structures) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_STRUCTURES_ID);
        }
        if self.contains(RemoteDebugRenderer::VillageSections) {
            ids.insert(VANILLA_DEBUG_SUBSCRIPTION_VILLAGE_SECTIONS_ID);
        }
        ids.into_iter().collect()
    }
}

impl RemoteDebugRenderer {
    const fn bit(self) -> u16 {
        1 << self as u16
    }
}

impl FromIterator<RemoteDebugRenderer> for RemoteDebugRendererSelection {
    fn from_iter<T: IntoIterator<Item = RemoteDebugRenderer>>(renderers: T) -> Self {
        Self(
            renderers
                .into_iter()
                .fold(0, |selection, renderer| selection | renderer.bit()),
        )
    }
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct RemoteDebugGizmos {
    pub(crate) filled_boxes: Vec<DebugFilledBox>,
    pub(crate) text_billboards: Vec<DebugTextBillboard>,
    pub(crate) lines: Vec<DebugLine>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RemoteDebugSelectionCache {
    level_generation: Option<u64>,
    last_looked_at_uuid: Option<Uuid>,
}

impl RemoteDebugSelectionCache {
    /// Returns the target selected during the previous render, resetting the
    /// renderer-local state when the canonical client level changes. Vanilla
    /// renders with `lastLookedAtUuid` and updates it after gizmo emission.
    pub(crate) fn selected_for_generation(&mut self, level_generation: u64) -> Option<Uuid> {
        if self.level_generation != Some(level_generation) {
            self.level_generation = Some(level_generation);
            self.last_looked_at_uuid = None;
        }
        self.last_looked_at_uuid
    }

    pub(crate) fn finish_frame(
        &mut self,
        selection: RemoteDebugRendererSelection,
        local_player_is_spectator: bool,
        targeted_entity_uuid: Option<Uuid>,
    ) {
        if selection.contains(RemoteDebugRenderer::Brain) && !local_player_is_spectator {
            if let Some(uuid) = targeted_entity_uuid {
                self.last_looked_at_uuid = Some(uuid);
            }
        }
    }
}

pub(crate) fn remote_debug_gizmos(
    world: &WorldStore,
    selection: RemoteDebugRendererSelection,
    camera: Option<CameraPose>,
    selected_entity_uuid: Option<Uuid>,
) -> RemoteDebugGizmos {
    let mut output = RemoteDebugGizmos::default();
    if selection.is_empty() {
        return output;
    }
    let transforms = selection
        .needs_entity_transforms()
        .then(|| world.entity_transforms())
        .unwrap_or_default();
    let by_uuid = transforms
        .iter()
        .map(|transform| (transform.uuid, *transform))
        .collect::<BTreeMap<_, _>>();
    let by_id = transforms
        .iter()
        .map(|transform| (transform.id, *transform))
        .collect::<BTreeMap<_, _>>();
    let local_player_position = world
        .local_player_id()
        .and_then(|id| by_id.get(&id).copied())
        .map(entity_position)
        .or_else(|| camera.map(|camera| camera.position));

    // Preserve `DebugRenderer.refreshRendererList`'s renderer order.
    if selection.contains(RemoteDebugRenderer::Pathfinding) {
        render_paths(world, camera, &by_uuid, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::NeighborUpdates) {
        render_neighbor_updates(world, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::RedstoneWireOrientations) {
        render_redstone_orientations(world, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::Structures) {
        render_structures(world, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::VillageSections) {
        render_village_sections(world, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::Brain) {
        render_brains(
            world,
            local_player_position,
            selected_entity_uuid,
            &by_uuid,
            &mut output,
        );
    }
    if selection.contains(RemoteDebugRenderer::Poi) {
        render_pois(world, selection, camera, &by_uuid, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::Bees) {
        render_bees(world, local_player_position, camera, &by_uuid, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::Raids) {
        render_raids(world, camera, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::GoalSelector) {
        render_goals(world, camera, &by_uuid, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::GameEventListeners) {
        render_game_events(world, &by_uuid, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::Breeze) {
        render_breezes(world, &by_uuid, &by_id, &mut output);
    }
    if selection.contains(RemoteDebugRenderer::EntityBlockIntersections) {
        render_entity_block_intersections(world, &mut output);
    }
    output
}

fn render_entity_block_intersections(world: &WorldStore, output: &mut RemoteDebugGizmos) {
    let Some(values) =
        world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_ENTITY_BLOCK_INTERSECTIONS_ID)
    else {
        return;
    };
    for (pos, timed) in &*values.block_values {
        let DebugSubscriptionValueState::EntityBlockIntersection(kind) = timed.value else {
            continue;
        };
        let color = match kind {
            DebugEntityBlockIntersection::InBlock => [0, 255, 0, 96],
            DebugEntityBlockIntersection::InFluid => [0, 0, 255, 96],
            DebugEntityBlockIntersection::InAir => [51, 51, 51, 96],
        };
        output.filled_boxes.push(block_box(*pos, 0.02, color));
    }
}

fn render_redstone_orientations(world: &WorldStore, output: &mut RemoteDebugGizmos) {
    let Some(values) =
        world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_REDSTONE_WIRE_ORIENTATIONS_ID)
    else {
        return;
    };
    for (pos, timed) in &*values.block_values {
        let DebugSubscriptionValueState::RedstoneOrientation(orientation) = timed.value else {
            continue;
        };
        let Some((front, up, side)) = redstone_orientation_vectors(orientation.index) else {
            continue;
        };
        let center = [pos.x as f32 + 0.5, pos.y as f32 - 0.1, pos.z as f32 + 0.5];
        push_vector(output, center, front, 0.5, argb(-16_776_961));
        push_vector(output, center, up, 0.4, RED);
        push_vector(output, center, side, 0.3, YELLOW);
    }
}

fn render_village_sections(world: &WorldStore, output: &mut RemoteDebugGizmos) {
    let Some(values) =
        world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_VILLAGE_SECTIONS_ID)
    else {
        return;
    };
    for (pos, timed) in &*values.block_values {
        if matches!(timed.value, DebugSubscriptionValueState::VillageSection) {
            let section_center = BlockPos {
                x: pos.x.div_euclid(16) * 16 + 8,
                y: pos.y.div_euclid(16) * 16 + 8,
                z: pos.z.div_euclid(16) * 16 + 8,
            };
            output
                .filled_boxes
                .push(block_box(section_center, 0.0, [51, 255, 51, 38]));
        }
    }
}

fn render_raids(world: &WorldStore, camera: Option<CameraPose>, output: &mut RemoteDebugGizmos) {
    let Some(values) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID) else {
        return;
    };
    for timed in values.chunk_values.values() {
        let DebugSubscriptionValueState::Raids(centers) = &timed.value else {
            continue;
        };
        for center in centers {
            if !within_block_distance(camera, *center, 160.0) {
                continue;
            }
            output
                .filled_boxes
                .push(block_box(*center, 0.0, [255, 0, 0, 38]));
            push_block_text(output, *center, 0, "Raid center", RED, 0.64);
        }
    }
}

fn render_structures(world: &WorldStore, output: &mut RemoteDebugGizmos) {
    let Some(values) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_STRUCTURES_ID)
    else {
        return;
    };
    for timed in values.chunk_values.values() {
        let DebugSubscriptionValueState::Structures(structures) = &timed.value else {
            continue;
        };
        for structure in structures {
            push_bounding_box_lines(output, structure.bounding_box, WHITE);
            for piece in &structure.pieces {
                push_bounding_box_lines(
                    output,
                    piece.bounding_box,
                    if piece.is_start {
                        [0, 255, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    },
                );
            }
        }
    }
}

fn render_neighbor_updates(world: &WorldStore, output: &mut RemoteDebugGizmos) {
    let Some(values) =
        world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID)
    else {
        return;
    };
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    let mut updates = BTreeMap::<BlockPos, (i64, usize)>::new();
    for timed in &values.events {
        let DebugSubscriptionValueState::NeighborUpdate(pos) = timed.value else {
            continue;
        };
        let remaining = timed
            .expires_after_game_time
            .unwrap_or(game_time)
            .saturating_sub(game_time);
        let age = 200_i64.saturating_sub(remaining).clamp(0, 200);
        match updates.entry(pos) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert((age, 1));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let (youngest_age, count) = entry.get_mut();
                if age < *youngest_age {
                    (*youngest_age, *count) = (age, 1);
                } else if age == *youngest_age {
                    *count += 1;
                }
            }
        }
    }
    for (pos, (age, count)) in updates {
        let shrink = age as f32 / 400.0;
        let min = [
            pos.x as f32 - 0.002 + shrink,
            pos.y as f32 - 0.002 + shrink,
            pos.z as f32 - 0.002 + shrink,
        ];
        let max = [
            pos.x as f32 + 1.002 - shrink,
            pos.y as f32 + 1.002 - shrink,
            pos.z as f32 + 1.002 - shrink,
        ];
        output
            .lines
            .extend(DebugLine::cuboid(min, max, WHITE, DebugLine::DEFAULT_WIDTH));
        output.text_billboards.push(DebugTextBillboard::centered(
            block_center(pos),
            count.to_string(),
            WHITE,
        ));
    }
}

fn render_game_events(
    world: &WorldStore,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    output: &mut RemoteDebugGizmos,
) {
    if let Some(values) =
        world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID)
    {
        for (pos, timed) in &*values.block_values {
            let DebugSubscriptionValueState::GameEventListener(listener) = timed.value else {
                continue;
            };
            render_game_event_listener(block_center(*pos), listener.listener_radius, output);
        }
        for (uuid, timed) in &*values.entity_values {
            let DebugSubscriptionValueState::GameEventListener(listener) = timed.value else {
                continue;
            };
            let Some(entity) = by_uuid.get(uuid) else {
                continue;
            };
            render_game_event_listener(entity_position(*entity), listener.listener_radius, output);
        }
    }
    if let Some(values) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID)
    {
        for timed in &values.events {
            let DebugSubscriptionValueState::GameEvent(event) = &timed.value else {
                continue;
            };
            let origin = event.pos.map(|value| value as f32);
            output.filled_boxes.push(DebugFilledBox {
                min: [origin[0] - 0.2, origin[1] + 0.05, origin[2] - 0.2],
                max: [origin[0] + 0.2, origin[1] + 0.95, origin[2] + 0.2],
                color: [255, 255, 255, 51],
            });
            let mut label = DebugTextBillboard::centered(
                [origin[0], origin[1] + 0.85, origin[2]],
                event.game_event_resource_id.clone(),
                argb(-7_564_911),
            );
            label.scale = 0.12;
            output.text_billboards.push(label);
        }
    }
}

fn render_game_event_listener(origin: [f32; 3], radius: i32, output: &mut RemoteDebugGizmos) {
    let radius = radius.max(0) as f32;
    output.filled_boxes.push(DebugFilledBox {
        min: [origin[0] - radius, origin[1] - radius, origin[2] - radius],
        max: [origin[0] + radius, origin[1] + radius, origin[2] + radius],
        color: [255, 255, 0, 89],
    });
    output.filled_boxes.push(DebugFilledBox {
        min: [origin[0] - 0.25, origin[1], origin[2] - 0.25],
        max: [origin[0] + 0.25, origin[1] + 1.0, origin[2] + 0.25],
        color: [255, 255, 0, 89],
    });
    let mut title = DebugTextBillboard::centered(
        [origin[0], origin[1] + 1.8, origin[2]],
        "Listener Origin",
        WHITE,
    );
    title.scale = 0.4;
    output.text_billboards.push(title);
    let block = BlockPos {
        x: origin[0].floor() as i32,
        y: origin[1].floor() as i32,
        z: origin[2].floor() as i32,
    };
    let mut pos = DebugTextBillboard::centered(
        [origin[0], origin[1] + 1.5, origin[2]],
        format!("BlockPos{{x={}, y={}, z={}}}", block.x, block.y, block.z),
        argb(-6_959_665),
    );
    pos.scale = 0.4;
    output.text_billboards.push(pos);
}

fn render_breezes(
    world: &WorldStore,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    by_id: &BTreeMap<i32, EntityTransformState>,
    output: &mut RemoteDebugGizmos,
) {
    let Some(values) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_BREEZES_ID)
    else {
        return;
    };
    for (uuid, timed) in &*values.entity_values {
        let DebugSubscriptionValueState::Breeze(info) = timed.value else {
            continue;
        };
        let Some(source) = by_uuid.get(uuid).copied() else {
            continue;
        };
        let source = entity_position(source);
        if let Some(target) = info.attack_target.and_then(|id| by_id.get(&id).copied()) {
            let target = entity_position(target);
            output.lines.push(DebugLine::new(
                source,
                target,
                [100, 255, 255, 255],
                DebugLine::DEFAULT_WIDTH,
            ));
            for (radius, color) in [
                (4.0, [0, 255, 0, 255]),
                (8.0, [255, 165, 0, 255]),
                (24.0, [255, 0, 0, 255]),
            ] {
                push_circle(
                    output,
                    [target[0], target[1] + 0.01, target[2]],
                    radius,
                    color,
                );
            }
        }
        if let Some(target) = info.jump_target {
            let target = world_block_pos(target);
            output.lines.push(DebugLine::new(
                source,
                block_center(target),
                [255, 100, 255, 255],
                DebugLine::DEFAULT_WIDTH,
            ));
            output
                .filled_boxes
                .push(block_box(target, 0.0, [255, 0, 0, 255]));
        }
    }
}

fn render_goals(
    world: &WorldStore,
    camera: Option<CameraPose>,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    output: &mut RemoteDebugGizmos,
) {
    let Some(values) =
        world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID)
    else {
        return;
    };
    for (uuid, timed) in &*values.entity_values {
        let DebugSubscriptionValueState::Goals(info) = &timed.value else {
            continue;
        };
        let Some(entity) = by_uuid.get(uuid).copied() else {
            continue;
        };
        let position = entity_position(entity);
        if !within_goal_range(camera, position, 160.0) {
            continue;
        }
        for (index, goal) in info.goals.iter().enumerate() {
            output.text_billboards.push(DebugTextBillboard::centered(
                [
                    position[0].floor() + 0.5,
                    position[1] + 2.0 + index as f32 * 0.25,
                    position[2].floor() + 0.5,
                ],
                goal.name.clone(),
                if goal.is_running { GREEN } else { GRAY },
            ));
        }
    }
}

fn render_paths(
    world: &WorldStore,
    camera: Option<CameraPose>,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    output: &mut RemoteDebugGizmos,
) {
    let Some(values) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID)
    else {
        return;
    };
    for (uuid, timed) in &*values.entity_values {
        if !by_uuid.contains_key(uuid) {
            continue;
        }
        let DebugSubscriptionValueState::Path(info) = &timed.value else {
            continue;
        };
        render_path(info, camera, output);
    }
}

fn render_path(info: &DebugPathInfo, camera: Option<CameraPose>, output: &mut RemoteDebugGizmos) {
    for (index, nodes) in info.path.nodes.windows(2).enumerate() {
        let to = nodes[1];
        if !within_manhattan(camera, node_corner(to), 80.0) {
            continue;
        }
        let hue = (index + 1) as f32 / info.path.nodes.len().max(1) as f32 * 0.33;
        output.lines.push(DebugLine::new(
            node_center(nodes[0]),
            node_center(to),
            vanilla_hsv_rgb(hue, 0.9, 0.9),
            DebugLine::DEFAULT_WIDTH,
        ));
    }
    if within_manhattan(
        camera,
        block_corner(world_block_pos(info.path.target)),
        80.0,
    ) {
        let target = info.path.target;
        output.filled_boxes.push(DebugFilledBox {
            min: [
                target.x as f32 + 0.25,
                target.y as f32 + 0.25,
                target.z as f32 + 0.25,
            ],
            max: [
                target.x as f32 + 0.75,
                target.y as f32 + 0.75,
                target.z as f32 + 0.75,
            ],
            color: [0, 255, 0, 128],
        });
        for (index, node) in info.path.nodes.iter().copied().enumerate() {
            if !within_manhattan(camera, node_corner(node), 80.0) {
                continue;
            }
            let next = i32::try_from(index).ok() == Some(info.path.next_node_index);
            let distance = info.max_node_distance;
            output.filled_boxes.push(DebugFilledBox {
                min: [
                    node.x as f32 + 0.5 - distance,
                    node.y as f32 + 0.01 * index as f32,
                    node.z as f32 + 0.5 - distance,
                ],
                max: [
                    node.x as f32 + 0.5 + distance,
                    node.y as f32 + 0.25 + 0.01 * index as f32,
                    node.z as f32 + 0.5 + distance,
                ],
                color: if next {
                    [255, 0, 0, 128]
                } else {
                    [0, 0, 255, 128]
                },
            });
        }
    }
    for (nodes, color) in [
        (&info.path.closed_set, [255, 204, 204, 128]),
        (&info.path.open_set, [204, 255, 255, 128]),
    ] {
        for node in nodes {
            if !within_manhattan(camera, node_corner(*node), 80.0) {
                continue;
            }
            let half = info.max_node_distance / 2.0;
            output.filled_boxes.push(DebugFilledBox {
                min: [
                    node.x as f32 + 0.5 - half,
                    node.y as f32 + 0.01,
                    node.z as f32 + 0.5 - half,
                ],
                max: [
                    node.x as f32 + 0.5 + half,
                    node.y as f32 + 0.1,
                    node.z as f32 + 0.5 + half,
                ],
                color,
            });
        }
    }
    for node in info.path.nodes.iter().copied() {
        if within_manhattan(camera, node_corner(node), 80.0) {
            push_path_node_labels(output, node);
        }
    }
}

fn push_path_node_labels(output: &mut RemoteDebugGizmos, node: DebugPathNode) {
    let mut type_label = DebugTextBillboard::centered(
        [
            node.x as f32 + 0.5,
            node.y as f32 + 0.75,
            node.z as f32 + 0.5,
        ],
        debug_path_type_name(node.path_type),
        WHITE,
    );
    type_label.scale = 0.32;
    output.text_billboards.push(type_label);
    let mut cost_label = DebugTextBillboard::centered(
        [
            node.x as f32 + 0.5,
            node.y as f32 + 0.25,
            node.z as f32 + 0.5,
        ],
        format!("{:.2}", node.cost_malus),
        WHITE,
    );
    cost_label.scale = 0.32;
    output.text_billboards.push(cost_label);
}

fn render_brains(
    world: &WorldStore,
    local_player_position: Option<[f32; 3]>,
    selected: Option<Uuid>,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    output: &mut RemoteDebugGizmos,
) {
    let Some(values) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID) else {
        return;
    };
    for (uuid, timed) in &*values.entity_values {
        let DebugSubscriptionValueState::Brain(brain) = &timed.value else {
            continue;
        };
        let Some(entity) = by_uuid.get(uuid).copied() else {
            continue;
        };
        if !within_position(local_player_position, entity_position(entity), 30.0) {
            continue;
        }
        render_brain_info(entity, brain, selected == Some(*uuid), output);
    }
}

fn render_brain_info(
    entity: EntityTransformState,
    brain: &DebugBrainDump,
    selected: bool,
    output: &mut RemoteDebugGizmos,
) {
    let mut row = 0;
    push_entity_text(output, entity, row, brain.name.clone(), WHITE, 0.48);
    row += 1;
    if selected {
        push_entity_text(
            output,
            entity,
            row,
            format!("{} {} xp", brain.profession, brain.xp),
            WHITE,
            0.32,
        );
        row += 1;
        push_entity_text(
            output,
            entity,
            row,
            format!("health: {:.1} / {:.1}", brain.health, brain.max_health),
            if brain.health < brain.max_health {
                ORANGE
            } else {
                WHITE
            },
            0.32,
        );
        row += 1;
        if !brain.inventory.is_empty() {
            push_entity_text(output, entity, row, brain.inventory.clone(), PINK, 0.32);
            row += 1;
        }
        for behavior in &brain.behaviors {
            push_entity_text(output, entity, row, behavior.clone(), CYAN, 0.32);
            row += 1;
        }
        for activity in &brain.activities {
            push_entity_text(output, entity, row, activity.clone(), GREEN, 0.32);
            row += 1;
        }
    }
    if brain.wants_golem {
        push_entity_text(output, entity, row, "Wants Golem", ORANGE, 0.32);
        row += 1;
    }
    if selected && brain.anger_level != -1 {
        push_entity_text(
            output,
            entity,
            row,
            format!("Anger Level: {}", brain.anger_level),
            PINK,
            0.32,
        );
        row += 1;
    }
    if selected {
        for gossip in &brain.gossips {
            push_entity_text(
                output,
                entity,
                row,
                gossip.clone(),
                if gossip.starts_with(&brain.name) {
                    WHITE
                } else {
                    ORANGE
                },
                0.32,
            );
            row += 1;
        }
        for memory in brain.memories.iter().rev() {
            push_entity_text(output, entity, row, memory.clone(), GRAY, 0.32);
            row += 1;
        }
    }
}

fn render_pois(
    world: &WorldStore,
    selection: RemoteDebugRendererSelection,
    camera: Option<CameraPose>,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    output: &mut RemoteDebugGizmos,
) {
    let Some(values) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_POIS_ID) else {
        return;
    };
    for (pos, timed) in &*values.block_values {
        let DebugSubscriptionValueState::Poi(poi) = &timed.value else {
            continue;
        };
        if !within_block_distance(camera, *pos, 30.0) {
            continue;
        }
        output
            .filled_boxes
            .push(block_box(*pos, 0.05, [51, 51, 255, 76]));
        let mut row = 0;
        if selection.contains(RemoteDebugRenderer::Brain) {
            let owners = brain_poi_names(world, by_uuid, poi.pos, false);
            let candidates = brain_poi_names(world, by_uuid, poi.pos, true);
            push_block_text(
                output,
                *pos,
                row,
                format_name_count("Owners", &owners),
                YELLOW,
                0.32,
            );
            row += 1;
            push_block_text(
                output,
                *pos,
                row,
                format_name_count("Candidates", &candidates),
                ORANGE,
                0.32,
            );
            row += 1;
        }
        push_block_text(
            output,
            *pos,
            row,
            format!("Free tickets: {}", poi.free_ticket_count),
            YELLOW,
            0.32,
        );
        push_block_text(
            output,
            *pos,
            row + 1,
            poi.poi_type_resource_id.clone(),
            WHITE,
            0.32,
        );
    }
    if !selection.contains(RemoteDebugRenderer::Brain) {
        return;
    }
    for (pos, names) in ghost_pois(world, by_uuid) {
        if values.block_values.contains_key(&pos) || !within_block_distance(camera, pos, 30.0) {
            continue;
        }
        output
            .filled_boxes
            .push(block_box(pos, 0.05, [51, 51, 255, 76]));
        push_block_text(
            output,
            pos,
            0,
            format_debug_names(names.iter()),
            YELLOW,
            0.32,
        );
        push_block_text(output, pos, 1, "Ghost POI", RED, 0.32);
    }
}

fn render_bees(
    world: &WorldStore,
    local_player_position: Option<[f32; 3]>,
    camera: Option<CameraPose>,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    output: &mut RemoteDebugGizmos,
) {
    let Some(bees) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_BEES_ID) else {
        return;
    };
    let goals = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID);
    for (uuid, timed) in &*bees.entity_values {
        let DebugSubscriptionValueState::Bee(bee) = &timed.value else {
            continue;
        };
        let Some(entity) = by_uuid.get(uuid).copied() else {
            continue;
        };
        if !within_position(local_player_position, entity_position(entity), 30.0) {
            continue;
        }
        let mut row = 0;
        push_entity_text(output, entity, row, format_debug_bee_info(bee), WHITE, 0.48);
        row += 1;
        push_entity_text(
            output,
            entity,
            row,
            bee.hive_pos
                .map(|pos| format!("Hive: {}", bee_pos_description(entity, pos)))
                .unwrap_or_else(|| "No hive".to_string()),
            if bee.hive_pos.is_some() { YELLOW } else { PINK },
            0.32,
        );
        row += 1;
        push_entity_text(
            output,
            entity,
            row,
            bee.flower_pos
                .map(|pos| format!("Flower: {}", bee_pos_description(entity, pos)))
                .unwrap_or_else(|| "No flower".to_string()),
            if bee.flower_pos.is_some() {
                YELLOW
            } else {
                PINK
            },
            0.32,
        );
        row += 1;
        if let Some(goal_values) = goals.and_then(|values| values.entity_values.get(uuid)) {
            if let DebugSubscriptionValueState::Goals(info) = &goal_values.value {
                for goal in info.goals.iter().filter(|goal| goal.is_running) {
                    push_entity_text(output, entity, row, goal.name.clone(), GREEN, 0.32);
                    row += 1;
                }
            }
        }
        if bee.travel_ticks > 0 {
            push_entity_text(
                output,
                entity,
                row,
                format!("Travelling: {} ticks", bee.travel_ticks),
                if bee.travel_ticks < 2_400 {
                    GRAY
                } else {
                    ORANGE
                },
                0.32,
            );
        }
    }
    render_bee_flowers(bees.entity_values.iter(), output);
    render_bee_hives(world, bees.entity_values.iter(), camera, output);
}

fn render_bee_flowers<'a>(
    bees: impl Iterator<Item = (&'a Uuid, &'a DebugTimedValueState)>,
    output: &mut RemoteDebugGizmos,
) {
    let mut flowers = BTreeMap::<BlockPos, BTreeSet<String>>::new();
    for (uuid, timed) in bees {
        if let DebugSubscriptionValueState::Bee(bee) = &timed.value {
            if let Some(pos) = bee.flower_pos {
                flowers
                    .entry(world_block_pos(pos))
                    .or_default()
                    .insert(debug_entity_name(*uuid));
            }
        }
    }
    for (pos, names) in flowers {
        output
            .filled_boxes
            .push(block_box(pos, 0.05, [204, 204, 0, 76]));
        push_block_text(
            output,
            pos,
            1,
            format_debug_names(names.iter()),
            YELLOW,
            0.32,
        );
        push_block_text(output, pos, 2, "Flower", WHITE, 0.32);
    }
}

fn render_bee_hives<'a>(
    world: &WorldStore,
    bees: impl Iterator<Item = (&'a Uuid, &'a DebugTimedValueState)> + Clone,
    camera: Option<CameraPose>,
    output: &mut RemoteDebugGizmos,
) {
    let hives = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_BEE_HIVES_ID);
    let mut hive_members = BTreeMap::<BlockPos, BTreeSet<Uuid>>::new();
    let mut hive_blacklists = BTreeMap::<BlockPos, BTreeSet<Uuid>>::new();
    let mut ghost_hives = BTreeMap::<BlockPos, Vec<String>>::new();
    for (uuid, timed) in bees.clone() {
        let DebugSubscriptionValueState::Bee(bee) = &timed.value else {
            continue;
        };
        if let Some(pos) = bee.hive_pos.map(world_block_pos) {
            hive_members.entry(pos).or_default().insert(*uuid);
            if hives.is_none_or(|values| !values.block_values.contains_key(&pos)) {
                ghost_hives
                    .entry(pos)
                    .or_default()
                    .push(debug_entity_name(*uuid));
            }
        }
        for pos in bee.blacklisted_hives.iter().copied().map(world_block_pos) {
            hive_blacklists.entry(pos).or_default().insert(*uuid);
        }
    }
    if let Some(hives) = hives {
        for (pos, timed) in &*hives.block_values {
            let DebugSubscriptionValueState::Hive(hive) = &timed.value else {
                continue;
            };
            if !within_block_distance(camera, *pos, 30.0) {
                continue;
            }
            output
                .filled_boxes
                .push(block_box(*pos, 0.05, [51, 51, 255, 76]));
            let mut row = 0;
            if let Some(blacklisted) = hive_blacklists.get(pos) {
                push_block_text(
                    output,
                    *pos,
                    row,
                    format!("Blacklisted by {}", format_bee_uuids(blacklisted)),
                    RED,
                    0.32,
                );
                row += 1;
            }
            push_block_text(
                output,
                *pos,
                row,
                format!(
                    "Out: {}",
                    hive_members
                        .get(pos)
                        .map_or_else(|| "-".to_string(), format_bee_uuids)
                ),
                GRAY,
                0.32,
            );
            row += 1;
            push_block_text(
                output,
                *pos,
                row,
                match hive.occupant_count {
                    0 => "In: -".to_string(),
                    1 => "In: 1 bee".to_string(),
                    count => format!("In: {count} bees"),
                },
                YELLOW,
                0.32,
            );
            row += 1;
            push_block_text(
                output,
                *pos,
                row,
                format!("Honey: {}", hive.honey_level),
                ORANGE,
                0.32,
            );
            push_block_text(
                output,
                *pos,
                row + 1,
                format!(
                    "{}{}",
                    hive.block_resource_id,
                    if hive.sedated { " (sedated)" } else { "" }
                ),
                WHITE,
                0.32,
            );
        }
    }
    for (pos, names) in ghost_hives {
        if !within_block_distance(camera, pos, 30.0) {
            continue;
        }
        output
            .filled_boxes
            .push(block_box(pos, 0.05, [51, 51, 255, 76]));
        push_block_text(
            output,
            pos,
            0,
            format_debug_names(names.iter()),
            YELLOW,
            0.32,
        );
        push_block_text(output, pos, 1, "Ghost Hive", RED, 0.32);
    }
}

fn brain_poi_names(
    world: &WorldStore,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
    pos: BlockPos,
    potential: bool,
) -> Vec<String> {
    let Some(brains) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID) else {
        return Vec::new();
    };
    brains
        .entity_values
        .iter()
        .filter_map(|(uuid, timed)| {
            by_uuid.contains_key(uuid).then_some(())?;
            let DebugSubscriptionValueState::Brain(brain) = &timed.value else {
                return None;
            };
            let positions = if potential {
                &brain.potential_pois
            } else {
                &brain.pois
            };
            positions
                .contains(&bbb_protocol::packets::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                })
                .then(|| debug_entity_name(*uuid))
        })
        .collect()
}

fn ghost_pois(
    world: &WorldStore,
    by_uuid: &BTreeMap<Uuid, EntityTransformState>,
) -> BTreeMap<BlockPos, Vec<String>> {
    let mut ghosts = BTreeMap::new();
    let Some(brains) = world.debug_subscription_values(VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID) else {
        return ghosts;
    };
    for (uuid, timed) in &*brains.entity_values {
        if !by_uuid.contains_key(uuid) {
            continue;
        }
        let DebugSubscriptionValueState::Brain(brain) = &timed.value else {
            continue;
        };
        for pos in brain.pois.iter().chain(&brain.potential_pois) {
            ghosts
                .entry(BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                })
                .or_insert_with(Vec::new)
                .push(brain.name.clone());
        }
    }
    ghosts
}

fn format_name_count(prefix: &str, names: &[String]) -> String {
    if names.len() < 4 {
        format!("{prefix}: {}", format_debug_names(names.iter()))
    } else {
        match prefix {
            "Owners" => format!("{} ticket holders", names.len()),
            "Candidates" => format!("{} potential owners", names.len()),
            _ => format!("{} {}", names.len(), prefix.to_ascii_lowercase()),
        }
    }
}

fn push_entity_text(
    output: &mut RemoteDebugGizmos,
    entity: EntityTransformState,
    row: usize,
    text: impl Into<String>,
    color: [u8; 4],
    scale: f32,
) {
    let position = entity_position(entity);
    let mut billboard = DebugTextBillboard::centered(
        [
            position[0].floor() + 0.5,
            position[1] + 2.4 + row as f32 * 0.25,
            position[2].floor() + 0.5,
        ],
        text,
        color,
    );
    billboard.scale = scale;
    billboard.adjust_left = Some(0.5);
    output.text_billboards.push(billboard);
}

fn push_block_text(
    output: &mut RemoteDebugGizmos,
    pos: BlockPos,
    row: usize,
    text: impl Into<String>,
    color: [u8; 4],
    scale: f32,
) {
    let mut billboard = DebugTextBillboard::centered(
        [
            pos.x as f32 + 0.5,
            pos.y as f32 + 1.3 + row as f32 * 0.2,
            pos.z as f32 + 0.5,
        ],
        text,
        color,
    );
    billboard.scale = scale;
    output.text_billboards.push(billboard);
}

fn push_bounding_box_lines(
    output: &mut RemoteDebugGizmos,
    bounds: DebugBoundingBoxState,
    color: [u8; 4],
) {
    output.lines.extend(DebugLine::cuboid(
        [
            bounds.min.x as f32,
            bounds.min.y as f32,
            bounds.min.z as f32,
        ],
        [
            bounds.max.x as f32 + 1.0,
            bounds.max.y as f32 + 1.0,
            bounds.max.z as f32 + 1.0,
        ],
        color,
        DebugLine::DEFAULT_WIDTH,
    ));
}

fn push_vector(
    output: &mut RemoteDebugGizmos,
    center: [f32; 3],
    vector: [i32; 3],
    scale: f32,
    color: [u8; 4],
) {
    output.lines.push(DebugLine::new(
        center,
        [
            center[0] + vector[0] as f32 * scale,
            center[1] + vector[1] as f32 * scale,
            center[2] + vector[2] as f32 * scale,
        ],
        color,
        DebugLine::DEFAULT_WIDTH,
    ));
}

fn push_circle(output: &mut RemoteDebugGizmos, center: [f32; 3], radius: f32, color: [u8; 4]) {
    const SEGMENTS: usize = 32;
    for index in 0..SEGMENTS {
        let angle_a = std::f32::consts::TAU * index as f32 / SEGMENTS as f32;
        let angle_b = std::f32::consts::TAU * (index + 1) as f32 / SEGMENTS as f32;
        output.lines.push(DebugLine::new(
            [
                center[0] + angle_a.cos() * radius,
                center[1],
                center[2] + angle_a.sin() * radius,
            ],
            [
                center[0] + angle_b.cos() * radius,
                center[1],
                center[2] + angle_b.sin() * radius,
            ],
            color,
            DebugLine::DEFAULT_WIDTH,
        ));
    }
}

fn block_box(pos: BlockPos, padding: f32, color: [u8; 4]) -> DebugFilledBox {
    DebugFilledBox {
        min: [
            pos.x as f32 - padding,
            pos.y as f32 - padding,
            pos.z as f32 - padding,
        ],
        max: [
            pos.x as f32 + 1.0 + padding,
            pos.y as f32 + 1.0 + padding,
            pos.z as f32 + 1.0 + padding,
        ],
        color,
    }
}

fn block_center(pos: BlockPos) -> [f32; 3] {
    [pos.x as f32 + 0.5, pos.y as f32 + 0.5, pos.z as f32 + 0.5]
}

fn block_corner(pos: BlockPos) -> [f32; 3] {
    [pos.x as f32, pos.y as f32, pos.z as f32]
}

fn world_block_pos(pos: bbb_protocol::packets::BlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn node_center(node: DebugPathNode) -> [f32; 3] {
    [
        node.x as f32 + 0.5,
        node.y as f32 + 0.5,
        node.z as f32 + 0.5,
    ]
}

fn node_corner(node: DebugPathNode) -> [f32; 3] {
    [node.x as f32, node.y as f32, node.z as f32]
}

fn entity_position(entity: EntityTransformState) -> [f32; 3] {
    [
        entity.position.x as f32,
        entity.position.y as f32,
        entity.position.z as f32,
    ]
}

fn bee_pos_description(
    entity: EntityTransformState,
    pos: bbb_protocol::packets::BlockPos,
) -> String {
    let entity = entity_position(entity).map(f64::from);
    let dx = f64::from(pos.x) + 0.5 - entity[0];
    let dy = f64::from(pos.y) + 0.5 - entity[1];
    let dz = f64::from(pos.z) + 0.5 - entity[2];
    let distance_squared = ((dx * dx + dy * dy + dz * dz) * 10.0).round() / 10.0;
    format!(
        "{}, {}, {} (dist {distance_squared:.1})",
        pos.x, pos.y, pos.z
    )
}

fn format_debug_bee_info(bee: &bbb_protocol::packets::DebugBeeInfo) -> String {
    let hive = bee
        .hive_pos
        .map(|pos| format!("Optional[{}]", java_block_pos(pos)))
        .unwrap_or_else(|| "Optional.empty".to_string());
    let flower = bee
        .flower_pos
        .map(|pos| format!("Optional[{}]", java_block_pos(pos)))
        .unwrap_or_else(|| "Optional.empty".to_string());
    let blacklisted = bee
        .blacklisted_hives
        .iter()
        .copied()
        .map(java_block_pos)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "DebugBeeInfo[hivePos={hive}, flowerPos={flower}, travelTicks={}, blacklistedHives=[{blacklisted}]]",
        bee.travel_ticks
    )
}

fn java_block_pos(pos: bbb_protocol::packets::BlockPos) -> String {
    format!("BlockPos{{x={}, y={}, z={}}}", pos.x, pos.y, pos.z)
}

fn format_bee_uuids(uuids: &BTreeSet<Uuid>) -> String {
    match uuids.len() {
        0 => "-".to_string(),
        1..=3 => format_debug_names(uuids.iter().copied().map(debug_entity_name)),
        count => format!("{count} bees"),
    }
}

fn format_debug_names<T: ToString>(names: impl IntoIterator<Item = T>) -> String {
    format!(
        "[{}]",
        names
            .into_iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn within_position(origin: Option<[f32; 3]>, position: [f32; 3], limit: f32) -> bool {
    origin.is_none_or(|origin| {
        position
            .iter()
            .zip(origin)
            .map(|(value, origin)| (value - origin).powi(2))
            .sum::<f32>()
            < limit * limit
    })
}

fn within_block_distance(camera: Option<CameraPose>, pos: BlockPos, limit: f32) -> bool {
    camera.is_none_or(|camera| {
        let dx = pos.x as f32 - camera.position[0].floor();
        let dy = pos.y as f32 - camera.position[1].floor();
        let dz = pos.z as f32 - camera.position[2].floor();
        dx * dx + dy * dy + dz * dz < limit * limit
    })
}

fn within_goal_range(camera: Option<CameraPose>, position: [f32; 3], limit: f32) -> bool {
    camera.is_none_or(|camera| {
        let dx = position[0].floor() - camera.position[0].floor();
        let dy = position[1].floor();
        let dz = position[2].floor() - camera.position[2].floor();
        dx * dx + dy * dy + dz * dz < limit * limit
    })
}

fn within_manhattan(camera: Option<CameraPose>, position: [f32; 3], limit: f32) -> bool {
    camera.is_none_or(|camera| {
        position
            .iter()
            .zip(camera.position)
            .map(|(value, camera)| (value - camera).abs())
            .sum::<f32>()
            <= limit
    })
}

fn debug_entity_name(uuid: Uuid) -> String {
    const FIRST: [&str; 33] = [
        "Slim", "Far", "River", "Silly", "Fat", "Thin", "Fish", "Bat", "Dark", "Oak", "Sly",
        "Bush", "Zen", "Bark", "Cry", "Slack", "Soup", "Grim", "Hook", "Dirt", "Mud", "Sad",
        "Hard", "Crook", "Sneak", "Stink", "Weird", "Fire", "Soot", "Soft", "Rough", "Cling",
        "Scar",
    ];
    const SECOND: [&str; 30] = [
        "Fox", "Tail", "Jaw", "Whisper", "Twig", "Root", "Finder", "Nose", "Brow", "Blade", "Fry",
        "Seek", "Wart", "Tooth", "Foot", "Leaf", "Stone", "Fall", "Face", "Tongue", "Voice", "Lip",
        "Mouth", "Snail", "Toe", "Ear", "Hair", "Beard", "Shirt", "Fist",
    ];
    let value = uuid.as_u128();
    let most = (value >> 64) as u64;
    let least = value as u64;
    let hilo = most ^ least;
    let hash = ((hilo >> 32) as u32 ^ hilo as u32) as i32;
    let mut random = JavaLegacyRandom::new(i64::from(hash >> 2));
    format!(
        "{}{}",
        FIRST[random.next_int(FIRST.len())],
        SECOND[random.next_int(SECOND.len())]
    )
}

struct JavaLegacyRandom {
    seed: u64,
}

impl JavaLegacyRandom {
    const MASK: u64 = (1_u64 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;

    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(11) & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    fn next_int(&mut self, bound: usize) -> usize {
        let bound = i32::try_from(bound).expect("debug name tables fit Java i32 bounds");
        if bound & (bound - 1) == 0 {
            return ((i64::from(bound) * i64::from(self.next(31))) >> 31) as usize;
        }
        loop {
            let sample = self.next(31);
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo as usize;
            }
        }
    }
}

fn debug_path_type_name(path_type: DebugPathType) -> &'static str {
    match path_type {
        DebugPathType::Blocked => "BLOCKED",
        DebugPathType::Open => "OPEN",
        DebugPathType::Walkable => "WALKABLE",
        DebugPathType::WalkableDoor => "WALKABLE_DOOR",
        DebugPathType::Trapdoor => "TRAPDOOR",
        DebugPathType::PowderSnow => "POWDER_SNOW",
        DebugPathType::OnTopOfPowderSnow => "ON_TOP_OF_POWDER_SNOW",
        DebugPathType::Fence => "FENCE",
        DebugPathType::Lava => "LAVA",
        DebugPathType::Water => "WATER",
        DebugPathType::WaterBorder => "WATER_BORDER",
        DebugPathType::Rail => "RAIL",
        DebugPathType::UnpassableRail => "UNPASSABLE_RAIL",
        DebugPathType::FireInNeighbor => "FIRE_IN_NEIGHBOR",
        DebugPathType::Fire => "FIRE",
        DebugPathType::DamagingInNeighbor => "DAMAGING_IN_NEIGHBOR",
        DebugPathType::Damaging => "DAMAGING",
        DebugPathType::DoorOpen => "DOOR_OPEN",
        DebugPathType::DoorWoodClosed => "DOOR_WOOD_CLOSED",
        DebugPathType::DoorIronClosed => "DOOR_IRON_CLOSED",
        DebugPathType::Breach => "BREACH",
        DebugPathType::Leaves => "LEAVES",
        DebugPathType::StickyHoney => "STICKY_HONEY",
        DebugPathType::Cocoa => "COCOA",
        DebugPathType::DamageCautious => "DAMAGE_CAUTIOUS",
        DebugPathType::OnTopOfTrapdoor => "ON_TOP_OF_TRAPDOOR",
        DebugPathType::BigMobsCloseToDanger => "BIG_MOBS_CLOSE_TO_DANGER",
    }
}

fn redstone_orientation_vectors(index: i32) -> Option<([i32; 3], [i32; 3], [i32; 3])> {
    const DIRECTIONS: [([i32; 3], i32, i32); 6] = [
        ([0, -1, 0], 1, 0),
        ([0, 1, 0], 1, 1),
        ([0, 0, -1], 2, 0),
        ([0, 0, 1], 2, 1),
        ([-1, 0, 0], 0, 0),
        ([1, 0, 0], 0, 1),
    ];
    for (up_ordinal, (up, up_axis, _)) in DIRECTIONS.iter().copied().enumerate() {
        for (front, front_axis, front_direction) in DIRECTIONS.iter().copied() {
            if up_axis == front_axis {
                continue;
            }
            let front_axis_key = if up_axis == 1 {
                i32::from(front_axis == 0)
            } else {
                i32::from(front_axis == 1)
            };
            let front_key = front_axis_key << 1 | front_direction;
            for side_bias in 0..=1 {
                let generated = (((up_ordinal as i32) << 2) + front_key) << 1 | side_bias;
                if generated != index {
                    continue;
                }
                let right = cross(front, up);
                let side = if side_bias == 0 {
                    [-right[0], -right[1], -right[2]]
                } else {
                    right
                };
                return Some((front, up, side));
            }
        }
    }
    None
}

const fn cross(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn vanilla_hsv_rgb(hue: f32, saturation: f32, value: f32) -> [u8; 4] {
    // Keep `Mth.hsvToRgb`'s Java ordering: the sector is reduced modulo six
    // before it is subtracted from the unwrapped hue. Values above one
    // therefore clamp rather than becoming purely periodic.
    let sector = (hue * 6.0) as i32 % 6;
    let fraction = hue * 6.0 - sector as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - fraction * saturation);
    let t = value * (1.0 - (1.0 - fraction) * saturation);
    let (r, g, b) = match sector.rem_euclid(6) {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    [
        ((r * 255.0) as i32).clamp(0, 255) as u8,
        ((g * 255.0) as i32).clamp(0, 255) as u8,
        ((b * 255.0) as i32).clamp(0, 255) as u8,
        255,
    ]
}

const fn argb(value: i32) -> [u8; 4] {
    let value = value as u32;
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
        ((value >> 24) & 0xff) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
        packets::{
            AddEntity, BlockPos as ProtocolBlockPos, ChunkPos as ProtocolChunkPos, DebugBeeInfo,
            DebugBlockValue, DebugBoundingBox, DebugBrainDump, DebugBreezeInfo, DebugChunkValue,
            DebugEntityValue, DebugEvent, DebugGameEventInfo, DebugGameEventListenerInfo,
            DebugGoal, DebugGoalInfo, DebugHiveInfo, DebugPath, DebugPoiInfo,
            DebugRedstoneOrientation, DebugStructureInfo, DebugStructurePiece,
            DebugSubscriptionEvent, DebugSubscriptionUpdate, DebugSubscriptionValue, Vec3d,
        },
    };

    #[test]
    fn renderer_groups_derive_exact_sorted_vanilla_subscription_set() {
        let selection = [
            RemoteDebugRenderer::Bees,
            RemoteDebugRenderer::GameEventListeners,
            RemoteDebugRenderer::Poi,
        ]
        .into_iter()
        .collect::<RemoteDebugRendererSelection>();

        assert_eq!(selection.subscription_ids(false), vec![1, 4, 7, 8, 13, 15]);
        assert_eq!(
            selection.subscription_ids(true),
            vec![0, 1, 4, 7, 8, 13, 15]
        );
    }

    #[test]
    fn only_entity_backed_renderers_request_entity_transform_projection() {
        let empty = RemoteDebugRendererSelection::default();
        assert!(empty.is_empty());
        assert!(!empty.needs_entity_transforms());
        assert_eq!(
            remote_debug_gizmos(&WorldStore::new(), empty, None, None),
            RemoteDebugGizmos::default()
        );

        for renderer in [
            RemoteDebugRenderer::EntityBlockIntersections,
            RemoteDebugRenderer::NeighborUpdates,
            RemoteDebugRenderer::Raids,
            RemoteDebugRenderer::RedstoneWireOrientations,
            RemoteDebugRenderer::Structures,
            RemoteDebugRenderer::VillageSections,
        ] {
            let selection = [renderer]
                .into_iter()
                .collect::<RemoteDebugRendererSelection>();
            assert!(!selection.needs_entity_transforms(), "{renderer:?}");
        }

        for renderer in [
            RemoteDebugRenderer::Bees,
            RemoteDebugRenderer::Brain,
            RemoteDebugRenderer::Breeze,
            RemoteDebugRenderer::GameEventListeners,
            RemoteDebugRenderer::GoalSelector,
            RemoteDebugRenderer::Pathfinding,
            RemoteDebugRenderer::Poi,
        ] {
            let selection = [renderer]
                .into_iter()
                .collect::<RemoteDebugRendererSelection>();
            assert!(selection.needs_entity_transforms(), "{renderer:?}");
        }
    }

    #[test]
    fn brain_selection_uses_previous_frame_and_resets_with_client_level() {
        let selection = [RemoteDebugRenderer::Brain]
            .into_iter()
            .collect::<RemoteDebugRendererSelection>();
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let mut cache = RemoteDebugSelectionCache::default();

        assert_eq!(cache.selected_for_generation(7), None);
        cache.finish_frame(selection, false, Some(first));
        assert_eq!(cache.selected_for_generation(7), Some(first));
        cache.finish_frame(selection, false, None);
        assert_eq!(cache.selected_for_generation(7), Some(first));
        cache.finish_frame(selection, true, Some(second));
        assert_eq!(cache.selected_for_generation(7), Some(first));
        assert_eq!(cache.selected_for_generation(8), None);
    }

    #[test]
    fn entity_debug_names_match_vanilla_java_uuid_random_vectors() {
        for (uuid, expected) in [
            ("00000000-0000-0000-0000-000000000000", "SlimShirt"),
            ("550e8400-e29b-41d4-a716-446655440000", "ThinHair"),
            ("de305d54-75b4-431b-adb2-eb6b9e546014", "HardHair"),
        ] {
            assert_eq!(debug_entity_name(uuid.parse().unwrap()), expected);
        }
    }

    #[test]
    fn all_redstone_orientation_indices_resolve_unique_orthogonal_vectors() {
        let orientations = (0..48)
            .map(|index| redstone_orientation_vectors(index).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(orientations.len(), 48);
        assert!(redstone_orientation_vectors(48).is_none());
        for (front, up, side) in orientations {
            assert_eq!(front.iter().zip(up).map(|(a, b)| a * b).sum::<i32>(), 0);
            assert_eq!(front.iter().zip(side).map(|(a, b)| a * b).sum::<i32>(), 0);
            assert_eq!(up.iter().zip(side).map(|(a, b)| a * b).sum::<i32>(), 0);
        }
    }

    #[test]
    fn every_remote_renderer_projects_from_one_canonical_world_generation() {
        let mut world = WorldStore::new();
        world
            .set_requested_debug_subscriptions(&(1..=15).collect::<Vec<_>>())
            .unwrap();
        let uuid = Uuid::from_u128(0x12345678123456781234567812345678);
        world.apply_add_entity(AddEntity {
            id: 42,
            uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_BEE_ID,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        for (subscription_id, value) in [
            (
                1,
                DebugSubscriptionValue::Bee(DebugBeeInfo {
                    hive_pos: Some(pos(2, 64, 2)),
                    flower_pos: Some(pos(3, 64, 3)),
                    travel_ticks: 5,
                    blacklisted_hives: vec![pos(2, 64, 2)],
                }),
            ),
            (
                2,
                DebugSubscriptionValue::Brain(DebugBrainDump {
                    name: "Brainy".to_string(),
                    profession: "farmer".to_string(),
                    xp: 3,
                    health: 10.0,
                    max_health: 20.0,
                    inventory: "wheat".to_string(),
                    wants_golem: true,
                    anger_level: 2,
                    activities: vec!["work".to_string()],
                    behaviors: vec!["harvest".to_string()],
                    memories: vec!["home".to_string()],
                    gossips: vec!["Brainy trusts".to_string()],
                    pois: vec![pos(4, 64, 4)],
                    potential_pois: Vec::new(),
                }),
            ),
            (
                3,
                DebugSubscriptionValue::Breeze(DebugBreezeInfo {
                    attack_target: Some(42),
                    jump_target: Some(pos(5, 64, 5)),
                }),
            ),
            (
                4,
                DebugSubscriptionValue::Goals(DebugGoalInfo {
                    goals: vec![DebugGoal {
                        priority: 0,
                        is_running: true,
                        name: "FloatGoal".to_string(),
                    }],
                }),
            ),
            (
                5,
                DebugSubscriptionValue::Path(DebugPathInfo {
                    path: DebugPath {
                        reached: false,
                        next_node_index: 1,
                        target: pos(7, 64, 7),
                        nodes: vec![path_node(6, 64, 6), path_node(7, 64, 7)],
                        target_nodes: Vec::new(),
                        open_set: vec![path_node(8, 64, 8)],
                        closed_set: vec![path_node(9, 64, 9)],
                    },
                    max_node_distance: 0.5,
                }),
            ),
            (
                13,
                DebugSubscriptionValue::GameEventListener(DebugGameEventListenerInfo {
                    listener_radius: 4,
                }),
            ),
        ] {
            world.apply_debug_entity_value(DebugEntityValue {
                entity_id: 42,
                update: DebugSubscriptionUpdate {
                    subscription_id,
                    value: Some(value),
                },
            });
        }
        for (position, subscription_id, value) in [
            (
                pos(1, 64, 1),
                6,
                DebugSubscriptionValue::EntityBlockIntersection(
                    DebugEntityBlockIntersection::InAir,
                ),
            ),
            (
                pos(2, 64, 2),
                7,
                DebugSubscriptionValue::Hive(DebugHiveInfo {
                    block_registry_id: 0,
                    occupant_count: 1,
                    honey_level: 3,
                    sedated: false,
                }),
            ),
            (
                pos(4, 64, 4),
                8,
                DebugSubscriptionValue::Poi(DebugPoiInfo {
                    pos: pos(4, 64, 4),
                    poi_type_holder_id: 13,
                    free_ticket_count: 1,
                }),
            ),
            (
                pos(10, 64, 10),
                9,
                DebugSubscriptionValue::RedstoneOrientation(DebugRedstoneOrientation { index: 0 }),
            ),
            (pos(16, 64, 16), 10, DebugSubscriptionValue::VillageSection),
        ] {
            world.apply_debug_block_value(DebugBlockValue {
                pos: position,
                update: DebugSubscriptionUpdate {
                    subscription_id,
                    value: Some(value),
                },
            });
        }
        world.apply_debug_chunk_value(DebugChunkValue {
            pos: ProtocolChunkPos { x: 0, z: 0 },
            update: DebugSubscriptionUpdate {
                subscription_id: 11,
                value: Some(DebugSubscriptionValue::Raids(vec![pos(12, 64, 12)])),
            },
        });
        world.apply_debug_chunk_value(DebugChunkValue {
            pos: ProtocolChunkPos { x: 0, z: 0 },
            update: DebugSubscriptionUpdate {
                subscription_id: 12,
                value: Some(DebugSubscriptionValue::Structures(vec![
                    DebugStructureInfo {
                        bounding_box: DebugBoundingBox {
                            min: pos(0, 60, 0),
                            max: pos(2, 62, 2),
                        },
                        pieces: vec![DebugStructurePiece {
                            bounding_box: DebugBoundingBox {
                                min: pos(0, 60, 0),
                                max: pos(1, 61, 1),
                            },
                            is_start: true,
                        }],
                    },
                ])),
            },
        });
        world.apply_debug_event(DebugEvent {
            event: DebugSubscriptionEvent {
                subscription_id: 14,
                value: DebugSubscriptionValue::NeighborUpdate(pos(11, 64, 11)),
            },
        });
        world.apply_debug_event(DebugEvent {
            event: DebugSubscriptionEvent {
                subscription_id: 15,
                value: DebugSubscriptionValue::GameEvent(DebugGameEventInfo {
                    game_event_holder_id: 0,
                    pos: Vec3d {
                        x: 13.0,
                        y: 64.0,
                        z: 13.0,
                    },
                }),
            },
        });
        let selection = [
            RemoteDebugRenderer::Bees,
            RemoteDebugRenderer::Brain,
            RemoteDebugRenderer::Breeze,
            RemoteDebugRenderer::EntityBlockIntersections,
            RemoteDebugRenderer::Pathfinding,
            RemoteDebugRenderer::GameEventListeners,
            RemoteDebugRenderer::GoalSelector,
            RemoteDebugRenderer::NeighborUpdates,
            RemoteDebugRenderer::Poi,
            RemoteDebugRenderer::Raids,
            RemoteDebugRenderer::RedstoneWireOrientations,
            RemoteDebugRenderer::Structures,
            RemoteDebugRenderer::VillageSections,
        ]
        .into_iter()
        .collect();

        let gizmos = remote_debug_gizmos(&world, selection, None, Some(uuid));

        assert!(gizmos.filled_boxes.len() >= 12);
        assert!(gizmos.lines.len() >= 40);
        for expected in [
            "Brainy",
            "FloatGoal",
            "Raid center",
            "minecraft:block_activate",
            "minecraft:home",
            "Listener Origin",
        ] {
            assert!(
                gizmos
                    .text_billboards
                    .iter()
                    .any(|billboard| billboard.text.contains(expected)),
                "missing remote debug text {expected}"
            );
        }
    }

    fn pos(x: i32, y: i32, z: i32) -> ProtocolBlockPos {
        ProtocolBlockPos { x, y, z }
    }

    fn path_node(x: i32, y: i32, z: i32) -> DebugPathNode {
        DebugPathNode {
            x,
            y,
            z,
            walked_distance: 0.0,
            cost_malus: 1.0,
            closed: false,
            path_type: DebugPathType::Walkable,
            f: 0.0,
        }
    }
}
