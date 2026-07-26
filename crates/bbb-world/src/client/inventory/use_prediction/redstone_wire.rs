use std::collections::BTreeMap;

use bbb_protocol::packets::Direction;

use crate::{client::block_collision_shape_is_full_block, BlockPos, WorldStore};

impl WorldStore {
    pub(super) fn local_redstone_wire_connection_state(
        &self,
        pos: BlockPos,
        mut state: BTreeMap<String, String>,
        was_dot: bool,
    ) -> Option<BTreeMap<String, String>> {
        let can_connect_up =
            !self.local_block_is_redstone_conductor(relative(pos, Direction::Up), pos)?;
        for direction in horizontal_directions() {
            let property = direction_name(direction);
            if state.get(property).is_none_or(|value| value == "none") {
                state.insert(
                    property.to_string(),
                    self.local_redstone_wire_connecting_side(pos, direction, can_connect_up)?
                        .to_string(),
                );
            }
        }
        let connected = wire_connected_sides(&state)?;
        if was_dot && connected.iter().all(|connected| !*connected) {
            return Some(state);
        }

        let [north, east, south, west] = connected;
        let north_south_empty = !north && !south;
        let east_west_empty = !east && !west;
        if !west && north_south_empty {
            state.insert("west".to_string(), "side".to_string());
        }
        if !east && north_south_empty {
            state.insert("east".to_string(), "side".to_string());
        }
        if !north && east_west_empty {
            state.insert("north".to_string(), "side".to_string());
        }
        if !south && east_west_empty {
            state.insert("south".to_string(), "side".to_string());
        }
        Some(state)
    }

    fn local_redstone_wire_connecting_side(
        &self,
        pos: BlockPos,
        direction: Direction,
        can_connect_up: bool,
    ) -> Option<&'static str> {
        let relative_pos = relative(pos, direction);
        let relative_block = self.probe_block(relative_pos);
        if can_connect_up {
            let placeable_above = relative_block
                .as_ref()
                .and_then(|block| block.block_name.as_deref())
                .is_some_and(is_trapdoor_name)
                || relative_block.as_ref().is_some_and(|block| {
                    self.block_has_full_sturdy_face(block, Direction::Up)
                        || block.block_name.as_deref() == Some("minecraft:hopper")
                });
            if placeable_above
                && self.local_block_should_connect_redstone(
                    relative(relative_pos, Direction::Up),
                    None,
                )?
            {
                if relative_block.as_ref().is_some_and(|block| {
                    self.block_has_full_sturdy_face(block, opposite(direction))
                }) {
                    return Some("up");
                }
                return Some("side");
            }
        }

        let connects_relative =
            self.local_block_should_connect_redstone(relative_pos, Some(direction))?;
        let relative_conductor =
            self.local_block_is_redstone_conductor(relative_pos, relative_pos)?;
        let connects_below = self
            .local_block_should_connect_redstone(relative(relative_pos, Direction::Down), None)?;
        if !connects_relative && (relative_conductor || !connects_below) {
            Some("none")
        } else {
            Some("side")
        }
    }

    fn local_block_should_connect_redstone(
        &self,
        pos: BlockPos,
        direction: Option<Direction>,
    ) -> Option<bool> {
        let Some(block) = self.probe_block(pos) else {
            return Some(false);
        };
        let name = block.block_name.as_deref()?;
        if name == "minecraft:redstone_wire" {
            return Some(true);
        }
        if name == "minecraft:repeater" {
            let facing = horizontal_direction_property(&block.block_properties, "facing")?;
            return Some(direction == Some(facing) || direction == Some(opposite(facing)));
        }
        if name == "minecraft:observer" {
            let facing = direction_property(&block.block_properties, "facing")?;
            return Some(direction == Some(facing));
        }
        Some(direction.is_some() && block_name_is_redstone_signal_source(name))
    }

    fn local_block_is_redstone_conductor(
        &self,
        state_pos: BlockPos,
        query_pos: BlockPos,
    ) -> Option<bool> {
        let Some(block) = self.probe_block(state_pos) else {
            return Some(false);
        };
        let name = block.block_name.as_deref()?;
        if matches!(name, "minecraft:soul_sand" | "minecraft:mud") {
            return Some(true);
        }
        if block_name_has_never_redstone_conductor_override(name) {
            return Some(false);
        }
        Some(block_collision_shape_is_full_block(&block, query_pos))
    }
}

pub(super) fn wire_connected_sides(properties: &BTreeMap<String, String>) -> Option<[bool; 4]> {
    let side = |name| match properties.get(name)?.as_str() {
        "none" => Some(false),
        "side" | "up" => Some(true),
        _ => None,
    };
    Some([side("north")?, side("east")?, side("south")?, side("west")?])
}

fn horizontal_direction_property(
    properties: &BTreeMap<String, String>,
    property: &str,
) -> Option<Direction> {
    match properties.get(property)?.as_str() {
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

fn direction_property(properties: &BTreeMap<String, String>, property: &str) -> Option<Direction> {
    match properties.get(property)?.as_str() {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::North => "north",
        Direction::East => "east",
        Direction::South => "south",
        Direction::West => "west",
        Direction::Down | Direction::Up => unreachable!("horizontal direction"),
    }
}

fn horizontal_directions() -> [Direction; 4] {
    [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ]
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    match direction {
        Direction::Down => BlockPos {
            y: pos.y - 1,
            ..pos
        },
        Direction::Up => BlockPos {
            y: pos.y + 1,
            ..pos
        },
        Direction::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        Direction::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        Direction::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        Direction::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}

fn is_trapdoor_name(name: &str) -> bool {
    name.ends_with("_trapdoor")
}

fn block_name_is_redstone_signal_source(name: &str) -> bool {
    name.ends_with("_button")
        || name.ends_with("_pressure_plate")
        || name.ends_with("lightning_rod")
        || matches!(
            name,
            "minecraft:lever"
                | "minecraft:redstone_torch"
                | "minecraft:redstone_wall_torch"
                | "minecraft:jukebox"
                | "minecraft:comparator"
                | "minecraft:repeater"
                | "minecraft:redstone_block"
                | "minecraft:detector_rail"
                | "minecraft:daylight_detector"
                | "minecraft:target"
                | "minecraft:sculk_sensor"
                | "minecraft:calibrated_sculk_sensor"
                | "minecraft:lectern"
                | "minecraft:trapped_chest"
                | "minecraft:observer"
                | "minecraft:tripwire_hook"
        )
}

fn block_name_has_never_redstone_conductor_override(name: &str) -> bool {
    name.ends_with("_leaves")
        || name.ends_with("_stained_glass")
        || name.ends_with("_copper_grate")
        || name.ends_with("_copper_bulb")
        || matches!(
            name,
            "minecraft:glass"
                | "minecraft:tinted_glass"
                | "minecraft:moving_piston"
                | "minecraft:piston"
                | "minecraft:sticky_piston"
                | "minecraft:tnt"
                | "minecraft:ice"
                | "minecraft:frosted_ice"
                | "minecraft:glowstone"
                | "minecraft:beacon"
                | "minecraft:redstone_block"
                | "minecraft:sea_lantern"
                | "minecraft:chorus_flower"
                | "minecraft:observer"
                | "minecraft:bamboo"
                | "minecraft:scaffolding"
                | "minecraft:powder_snow"
                | "minecraft:copper_grate"
                | "minecraft:copper_bulb"
                | "minecraft:pointed_dripstone"
        )
}
