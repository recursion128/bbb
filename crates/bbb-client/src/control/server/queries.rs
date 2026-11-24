use bbb_client_world::{BlockPos, WorldStore};
use bbb_protocol::packets::Direction as ProtocolDirection;
use serde_json::{json, Value};

use crate::camera_pose::camera_pose_from_world;
use crate::control::types::ControlResponse;
use crate::crosshair::{crosshair_pick_from_camera, CrosshairBlockHit, CrosshairTarget};

use super::params::{bool_param, f64_param, i32_param};

pub(crate) const MAX_WORLD_PROBE_BLOCKS: usize = 256;
pub(crate) const MAX_WORLD_SURFACE_COLUMNS: usize = 256;
pub(crate) const MAX_NEARBY_ENTITIES: usize = 256;
const DEFAULT_NEARBY_ENTITIES_MAX_DISTANCE: f64 = 32.0;
const DEFAULT_NEARBY_ENTITIES_LIMIT: usize = 64;

pub(super) fn probe_blocks_response(world: &WorldStore, params: &Value) -> ControlResponse {
    let Some(positions) = params.get("positions").and_then(Value::as_array) else {
        return err("world.probe_blocks requires array param positions");
    };
    if positions.is_empty() || positions.len() > MAX_WORLD_PROBE_BLOCKS {
        return err(format!(
            "world.probe_blocks requires 1..={MAX_WORLD_PROBE_BLOCKS} positions"
        ));
    }
    let skip_air = match params.get("skip_air") {
        None | Some(Value::Null) => false,
        Some(_) => match bool_param(params, "skip_air") {
            Some(value) => value,
            None => return err("world.probe_blocks requires boolean skip_air"),
        },
    };

    let mut blocks = Vec::with_capacity(positions.len());
    for (index, entry) in positions.iter().enumerate() {
        let Some(x) = i32_param(entry, "x") else {
            return err(format!(
                "world.probe_blocks positions[{index}] requires integer param x"
            ));
        };
        let Some(y) = i32_param(entry, "y") else {
            return err(format!(
                "world.probe_blocks positions[{index}] requires integer param y"
            ));
        };
        let Some(z) = i32_param(entry, "z") else {
            return err(format!(
                "world.probe_blocks positions[{index}] requires integer param z"
            ));
        };
        let pos = BlockPos { x, y, z };
        let value = match world.block_view(pos) {
            Some(view) => {
                if skip_air
                    && matches!(
                        view.block_name,
                        Some("minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
                    )
                {
                    continue;
                }
                json!({
                    "x": pos.x,
                    "y": pos.y,
                    "z": pos.z,
                    "loaded": true,
                    "block_state_id": view.block_state_id,
                    "block_name": view.block_name,
                    "block_properties": view.block_properties,
                    "material": view.material,
                    "fluid": view.fluid,
                })
            }
            None => json!({
                "x": pos.x,
                "y": pos.y,
                "z": pos.z,
                "loaded": false,
                "block_name": Value::Null,
            }),
        };
        blocks.push(value);
    }

    ok(json!({ "blocks": blocks }))
}

pub(super) fn probe_surface_response(world: &WorldStore, params: &Value) -> ControlResponse {
    let Some(min_x) = i32_param(params, "min_x") else {
        return err("world.probe_surface requires integer param min_x");
    };
    let Some(min_z) = i32_param(params, "min_z") else {
        return err("world.probe_surface requires integer param min_z");
    };
    let Some(max_x) = i32_param(params, "max_x") else {
        return err("world.probe_surface requires integer param max_x");
    };
    let Some(max_z) = i32_param(params, "max_z") else {
        return err("world.probe_surface requires integer param max_z");
    };
    let max_y = match params.get("max_y") {
        None | Some(Value::Null) => None,
        Some(value) => match value.as_i64().and_then(|v| i32::try_from(v).ok()) {
            Some(v) => Some(v),
            None => return err("world.probe_surface param max_y must be an integer"),
        },
    };
    if min_x > max_x || min_z > max_z {
        return err("world.probe_surface requires min_x <= max_x and min_z <= max_z");
    }
    let width = i128::from(max_x) - i128::from(min_x) + 1;
    let depth = i128::from(max_z) - i128::from(min_z) + 1;
    let count = width.saturating_mul(depth);
    if count <= 0 || count > MAX_WORLD_SURFACE_COLUMNS as i128 {
        return err(format!(
            "world.probe_surface rectangle exceeds {MAX_WORLD_SURFACE_COLUMNS} columns"
        ));
    }

    let mut columns = Vec::with_capacity(count as usize);
    for z in min_z..=max_z {
        for x in min_x..=max_x {
            columns.push(world.probe_surface_column(x, z, max_y));
        }
    }

    ok(json!({
        "min_x": min_x,
        "min_z": min_z,
        "max_x": max_x,
        "max_z": max_z,
        "columns": columns,
    }))
}

pub(super) fn crosshair_target_response(world: &WorldStore) -> ControlResponse {
    let pose = camera_pose_from_world(world);
    let eye = pose.map(|pose| {
        [
            f64::from(pose.position[0]),
            f64::from(pose.position[1]) + f64::from(pose.eye_height),
            f64::from(pose.position[2]),
        ]
    });
    let pick = crosshair_pick_from_camera(world, pose);
    let block_hit = pick.block_hit.map(|hit| block_hit_json(world, eye, hit));

    let value = match pick.target {
        Some(CrosshairTarget::Block(hit)) => json!({
            "kind": "block",
            "block": block_hit_json(world, eye, hit),
            "entity": Value::Null,
        }),
        Some(CrosshairTarget::Entity(hit)) => {
            let distance = eye.map(|eye| {
                euclidean_distance(eye, [hit.location.x, hit.location.y, hit.location.z])
            });
            json!({
                "kind": "entity",
                "entity": {
                    "entity_id": hit.entity_id,
                    "entity_type": world.entity_type_name(hit.entity_id),
                    "location": hit.location,
                    "relative_location": hit.relative_location,
                    "distance": distance,
                },
                "block": block_hit,
            })
        }
        None => json!({
            "kind": "miss",
            "block": Value::Null,
            "entity": Value::Null,
        }),
    };
    ok(value)
}

pub(super) fn nearby_entities_response(world: &WorldStore, params: &Value) -> ControlResponse {
    let max_distance = match params.get("max_distance") {
        None | Some(Value::Null) => DEFAULT_NEARBY_ENTITIES_MAX_DISTANCE,
        Some(_) => match f64_param(params, "max_distance").filter(|d| *d >= 0.0) {
            Some(distance) => distance,
            None => return err("world.nearby_entities requires non-negative finite max_distance"),
        },
    };
    let limit = match params.get("limit") {
        None | Some(Value::Null) => DEFAULT_NEARBY_ENTITIES_LIMIT,
        Some(_) => match i32_param(params, "limit")
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| (1..=MAX_NEARBY_ENTITIES).contains(v))
        {
            Some(limit) => limit,
            None => {
                return err(format!(
                    "world.nearby_entities requires integer limit in 1..={MAX_NEARBY_ENTITIES}"
                ))
            }
        },
    };
    let include_local_player = match params.get("include_local_player") {
        None | Some(Value::Null) => false,
        Some(_) => match bool_param(params, "include_local_player") {
            Some(value) => value,
            None => return err("world.nearby_entities requires boolean include_local_player"),
        },
    };

    let entities = world.nearby_entities(max_distance, limit, include_local_player);
    let origin = world.local_player_pose().map(|pose| pose.position);
    ok(json!({
        "origin": origin,
        "max_distance": max_distance,
        "entities": entities,
    }))
}

/// Adds human-readable `item_name` and `menu_type` resource IDs alongside raw
/// protocol `item_id` / `menu_type_id` fields and strips default empty fields
/// inside `component_patch` objects so inventory/recipe readouts stay compact
/// without removing any active data component values.
pub(super) fn annotate_item_and_menu_names(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                annotate_item_and_menu_names(child);
                if key == "component_patch" {
                    compact_component_patch(child);
                } else if key == "component_predicate" {
                    if let Some(components) = child.get_mut("components") {
                        compact_component_patch(components);
                    }
                }
            }
            if !map.contains_key("item_name") {
                if let Some(item_id) = map
                    .get("item_id")
                    .and_then(Value::as_i64)
                    .and_then(|id| i32::try_from(id).ok())
                {
                    if let Some(entry) = bbb_protocol::item_types::vanilla_item_type(item_id) {
                        map.insert(
                            "item_name".to_string(),
                            Value::String(entry.resource_id.to_string()),
                        );
                    }
                }
            }
            if !map.contains_key("menu_type") {
                if let Some(menu_id) = map
                    .get("menu_type_id")
                    .and_then(Value::as_i64)
                    .and_then(|id| i32::try_from(id).ok())
                {
                    if let Some(entry) = bbb_protocol::menu_types::vanilla_menu_type(menu_id) {
                        map.insert(
                            "menu_type".to_string(),
                            Value::String(entry.resource_id.to_string()),
                        );
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                annotate_item_and_menu_names(item);
            }
        }
        _ => {}
    }
}

fn compact_component_patch(value: &mut Value) {
    let Some(map) = value.as_object_mut() else {
        return;
    };
    map.retain(|key, field| match field {
        Value::Null | Value::Bool(false) => false,
        Value::Array(items) => !items.is_empty(),
        Value::Object(entries) => !entries.is_empty(),
        Value::Number(num) if matches!(key.as_str(), "added" | "bees_count") => {
            num.as_u64() != Some(0)
        }
        _ => true,
    });
}

fn block_hit_json(world: &WorldStore, eye: Option<[f64; 3]>, hit: CrosshairBlockHit) -> Value {
    let (face, (dx, dy, dz)) = direction_name_and_offset(hit.face);
    let adjacent_pos = BlockPos {
        x: hit.pos.x + dx,
        y: hit.pos.y + dy,
        z: hit.pos.z + dz,
    };
    let view = world.block_view(hit.pos);
    let hit_world = [
        f64::from(hit.pos.x) + hit.cursor[0],
        f64::from(hit.pos.y) + hit.cursor[1],
        f64::from(hit.pos.z) + hit.cursor[2],
    ];
    let distance = eye.map(|eye| euclidean_distance(eye, hit_world));
    json!({
        "pos": hit.pos,
        "face": face,
        "adjacent_pos": adjacent_pos,
        "cursor": hit.cursor,
        "inside": hit.inside,
        "distance": distance,
        "block_state_id": view.map(|v| v.block_state_id),
        "block_name": view.and_then(|v| v.block_name),
        "block_properties": view.map(|v| v.block_properties),
    })
}

fn direction_name_and_offset(direction: ProtocolDirection) -> (&'static str, (i32, i32, i32)) {
    match direction {
        ProtocolDirection::Down => ("down", (0, -1, 0)),
        ProtocolDirection::Up => ("up", (0, 1, 0)),
        ProtocolDirection::North => ("north", (0, 0, -1)),
        ProtocolDirection::South => ("south", (0, 0, 1)),
        ProtocolDirection::West => ("west", (-1, 0, 0)),
        ProtocolDirection::East => ("east", (1, 0, 0)),
    }
}

fn euclidean_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn ok(value: Value) -> ControlResponse {
    ControlResponse {
        ok: true,
        result: Some(value),
        error: None,
    }
}

fn err(message: impl Into<String>) -> ControlResponse {
    ControlResponse {
        ok: false,
        result: None,
        error: Some(message.into()),
    }
}
