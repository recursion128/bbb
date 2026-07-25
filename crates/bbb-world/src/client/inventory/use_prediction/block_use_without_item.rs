use super::{
    LocalBlockUseContext, LocalInteractionParticleAction, LocalInteractionResult,
    LocalInteractionSuccess, LocalLeverAttachFace, LocalTargetUsePrediction,
};
use crate::{client::block_collision_shape_is_full_block, BlockPos, WorldStore};
use bbb_protocol::{
    block_types::{
        BlockTypeCatalogEntry, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_IRON_DOOR_ID,
        VANILLA_BLOCK_IRON_TRAPDOOR_ID,
    },
    packets::Direction as ProtocolDirection,
};
use std::collections::BTreeMap;

impl WorldStore {
    pub(super) fn predict_local_block_use_without_item(
        &mut self,
        owner: VanillaBlockUseWithoutItemOverride,
        block_type: &BlockTypeCatalogEntry,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        let Some(block) = self.probe_block(context.pos) else {
            return unsupported();
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return unsupported();
        };
        if block_name != block_type.resource_id {
            return unsupported();
        }

        match owner {
            VanillaBlockUseWithoutItemOverride::ButtonBlock => {
                let Some(powered) = boolean_property(&block.block_properties, "powered") else {
                    return unsupported();
                };
                if powered {
                    return success(LocalInteractionSuccess::consume(false));
                }
                self.predict_local_block_property(
                    context,
                    block_name,
                    block.block_properties,
                    "powered",
                    "true",
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::LeverBlock => {
                let Some(powered) = boolean_property(&block.block_properties, "powered") else {
                    return unsupported();
                };
                let particle_action = if powered {
                    None
                } else {
                    let Some(facing) =
                        horizontal_direction_property(&block.block_properties, "facing")
                    else {
                        return unsupported();
                    };
                    let Some(face) = lever_attach_face(&block.block_properties) else {
                        return unsupported();
                    };
                    Some(LocalInteractionParticleAction::LeverActivation {
                        pos: context.pos,
                        facing,
                        face,
                    })
                };
                success(LocalInteractionSuccess {
                    particle_action,
                    ..LocalInteractionSuccess::client_item_interaction()
                })
            }
            VanillaBlockUseWithoutItemOverride::DoorBlock => {
                if block_type.id == VANILLA_BLOCK_IRON_DOOR_ID {
                    return pass();
                }
                self.predict_local_toggled_boolean_property(
                    context,
                    block_name,
                    block.block_properties,
                    "open",
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::TrapDoorBlock => {
                if block_type.id == VANILLA_BLOCK_IRON_TRAPDOOR_ID {
                    return pass();
                }
                self.predict_local_toggled_boolean_property(
                    context,
                    block_name,
                    block.block_properties,
                    "open",
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::FenceGateBlock => {
                let Some(open) = boolean_property(&block.block_properties, "open") else {
                    return unsupported();
                };
                let mut properties = block.block_properties;
                if open {
                    properties.insert("open".to_string(), "false".to_string());
                } else {
                    let Some(player_direction) = self.local_player_horizontal_direction() else {
                        return unsupported();
                    };
                    let Some(facing) = horizontal_direction_property(&properties, "facing") else {
                        return unsupported();
                    };
                    if facing == opposite_horizontal(player_direction) {
                        properties.insert(
                            "facing".to_string(),
                            direction_name(player_direction).to_string(),
                        );
                    }
                    properties.insert("open".to_string(), "true".to_string());
                }
                self.predict_local_block_properties(context, block_name, properties, owner)
            }
            VanillaBlockUseWithoutItemOverride::RepeaterBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                let Some(delay) = block
                    .block_properties
                    .get("delay")
                    .and_then(|delay| delay.parse::<u8>().ok())
                    .filter(|delay| (1..=4).contains(delay))
                else {
                    return unsupported();
                };
                self.predict_local_block_property(
                    context,
                    block_name,
                    block.block_properties,
                    "delay",
                    if delay == 4 {
                        "1".to_string()
                    } else {
                        (delay + 1).to_string()
                    },
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::DaylightDetectorBlock => {
                if self.local_player_may_build() {
                    // `DaylightDetectorBlock.useWithoutItem` changes the state
                    // only on the logical server, but still returns client
                    // `SUCCESS` and therefore swings immediately.
                    success(LocalInteractionSuccess::client_item_interaction())
                } else {
                    pass()
                }
            }
            VanillaBlockUseWithoutItemOverride::ComparatorBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                let mode = match block.block_properties.get("mode").map(String::as_str) {
                    Some("compare") => "subtract",
                    Some("subtract") => "compare",
                    _ => return unsupported(),
                };
                // Vanilla immediately calls `refreshOutputState` after this
                // mode mutation. Its output derives from arbitrary analog
                // block entities and item frames, which remain synchronized
                // server state here; the mode itself is client canonical.
                self.predict_local_block_property(
                    context,
                    block_name,
                    block.block_properties,
                    "mode",
                    mode,
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::RedStoneWireBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                self.predict_local_redstone_wire_toggle(
                    context,
                    block_name,
                    block.block_properties,
                    owner,
                )
            }
            _ => unsupported(),
        }
    }

    fn predict_local_redstone_wire_toggle(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        properties: BTreeMap<String, String>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(power) = properties
            .get("power")
            .and_then(|power| power.parse::<u8>().ok())
            .filter(|power| *power <= 15)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        let Some(connected) = wire_connected_sides(&properties) else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        let is_cross = connected.iter().all(|connected| *connected);
        let is_dot = connected.iter().all(|connected| !*connected);
        if !is_cross && !is_dot {
            return pass();
        }

        let updated = BTreeMap::from([
            ("north".to_string(), "none".to_string()),
            ("east".to_string(), "none".to_string()),
            ("south".to_string(), "none".to_string()),
            ("west".to_string(), "none".to_string()),
            ("power".to_string(), power.to_string()),
        ]);
        // Vanilla computes `wasDot` from the replacement state: a cross input
        // starts from the default dot, while a dot input starts from the cross.
        let replacement_was_dot = is_cross;
        let Some(updated) =
            self.local_redstone_wire_connection_state(context.pos, updated, replacement_was_dot)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        if updated == properties {
            return pass();
        }
        // Preserve the exact resource rather than assuming only the built-in
        // redstone-wire name can own this generated override.
        self.predict_local_block_properties(context, block_name, updated, owner)
    }

    fn local_redstone_wire_connection_state(
        &self,
        pos: BlockPos,
        mut state: BTreeMap<String, String>,
        was_dot: bool,
    ) -> Option<BTreeMap<String, String>> {
        let can_connect_up = !self.local_block_is_redstone_conductor(above(pos), pos)?;
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
        direction: ProtocolDirection,
        can_connect_up: bool,
    ) -> Option<&'static str> {
        let relative_pos = relative(pos, direction);
        let relative = self.probe_block(relative_pos);
        if can_connect_up {
            let placeable_above = relative
                .as_ref()
                .and_then(|block| block.block_name.as_deref())
                .is_some_and(is_trapdoor_name)
                || relative.as_ref().is_some_and(|block| {
                    self.block_has_full_sturdy_face(block, ProtocolDirection::Up)
                        || block.block_name.as_deref() == Some("minecraft:hopper")
                });
            if placeable_above
                && self.local_block_should_connect_redstone(above(relative_pos), None)?
            {
                if relative.as_ref().is_some_and(|block| {
                    self.block_has_full_sturdy_face(block, opposite_direction(direction))
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
        let connects_below = self.local_block_should_connect_redstone(below(relative_pos), None)?;
        if !connects_relative && (relative_conductor || !connects_below) {
            Some("none")
        } else {
            Some("side")
        }
    }

    fn local_block_should_connect_redstone(
        &self,
        pos: BlockPos,
        direction: Option<ProtocolDirection>,
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
            return Some(
                direction == Some(facing) || direction == Some(opposite_direction(facing)),
            );
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

    fn predict_local_toggled_boolean_property(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        properties: BTreeMap<String, String>,
        property: &str,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(value) = boolean_property(&properties, property) else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        self.predict_local_block_property(
            context,
            block_name,
            properties,
            property,
            if value { "false" } else { "true" },
            owner,
        )
    }

    fn predict_local_block_property(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        mut properties: BTreeMap<String, String>,
        property: &str,
        value: impl Into<String>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        properties.insert(property.to_string(), value.into());
        self.predict_local_block_properties(context, block_name, properties, owner)
    }

    fn predict_local_block_properties(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        properties: BTreeMap<String, String>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(state_id) = self
            .registries
            .block_state_id_by_name_and_properties(block_name, &properties)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        }
        success(LocalInteractionSuccess::client_item_interaction())
    }

    fn local_player_may_build(&self) -> bool {
        !self.local_player_is_adventure() && !self.local_player_is_spectator()
    }

    fn local_player_horizontal_direction(&self) -> Option<ProtocolDirection> {
        let yaw = f64::from(self.local_player().pose?.y_rot);
        if !yaw.is_finite() {
            return None;
        }
        match ((yaw / 90.0 + 0.5).floor() as i64).rem_euclid(4) {
            0 => Some(ProtocolDirection::South),
            1 => Some(ProtocolDirection::West),
            2 => Some(ProtocolDirection::North),
            3 => Some(ProtocolDirection::East),
            _ => unreachable!("rem_euclid(4) is in 0..4"),
        }
    }
}

fn boolean_property(properties: &BTreeMap<String, String>, property: &str) -> Option<bool> {
    match properties.get(property)?.as_str() {
        "false" => Some(false),
        "true" => Some(true),
        _ => None,
    }
}

fn horizontal_direction_property(
    properties: &BTreeMap<String, String>,
    property: &str,
) -> Option<ProtocolDirection> {
    match properties.get(property)?.as_str() {
        "north" => Some(ProtocolDirection::North),
        "south" => Some(ProtocolDirection::South),
        "west" => Some(ProtocolDirection::West),
        "east" => Some(ProtocolDirection::East),
        _ => None,
    }
}

fn direction_property(
    properties: &BTreeMap<String, String>,
    property: &str,
) -> Option<ProtocolDirection> {
    match properties.get(property)?.as_str() {
        "down" => Some(ProtocolDirection::Down),
        "up" => Some(ProtocolDirection::Up),
        "north" => Some(ProtocolDirection::North),
        "south" => Some(ProtocolDirection::South),
        "west" => Some(ProtocolDirection::West),
        "east" => Some(ProtocolDirection::East),
        _ => None,
    }
}

fn lever_attach_face(properties: &BTreeMap<String, String>) -> Option<LocalLeverAttachFace> {
    match properties.get("face")?.as_str() {
        "floor" => Some(LocalLeverAttachFace::Floor),
        "wall" => Some(LocalLeverAttachFace::Wall),
        "ceiling" => Some(LocalLeverAttachFace::Ceiling),
        _ => None,
    }
}

fn opposite_horizontal(direction: ProtocolDirection) -> ProtocolDirection {
    match direction {
        ProtocolDirection::North => ProtocolDirection::South,
        ProtocolDirection::South => ProtocolDirection::North,
        ProtocolDirection::West => ProtocolDirection::East,
        ProtocolDirection::East => ProtocolDirection::West,
        ProtocolDirection::Down | ProtocolDirection::Up => {
            unreachable!("horizontal direction cannot be vertical")
        }
    }
}

fn opposite_direction(direction: ProtocolDirection) -> ProtocolDirection {
    match direction {
        ProtocolDirection::Down => ProtocolDirection::Up,
        ProtocolDirection::Up => ProtocolDirection::Down,
        ProtocolDirection::North => ProtocolDirection::South,
        ProtocolDirection::South => ProtocolDirection::North,
        ProtocolDirection::West => ProtocolDirection::East,
        ProtocolDirection::East => ProtocolDirection::West,
    }
}

fn direction_name(direction: ProtocolDirection) -> &'static str {
    match direction {
        ProtocolDirection::North => "north",
        ProtocolDirection::South => "south",
        ProtocolDirection::West => "west",
        ProtocolDirection::East => "east",
        ProtocolDirection::Down | ProtocolDirection::Up => {
            unreachable!("horizontal direction cannot be vertical")
        }
    }
}

fn horizontal_directions() -> [ProtocolDirection; 4] {
    [
        ProtocolDirection::North,
        ProtocolDirection::East,
        ProtocolDirection::South,
        ProtocolDirection::West,
    ]
}

fn relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
    match direction {
        ProtocolDirection::Down => below(pos),
        ProtocolDirection::Up => above(pos),
        ProtocolDirection::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        ProtocolDirection::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        ProtocolDirection::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        ProtocolDirection::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}

fn above(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y + 1,
        ..pos
    }
}

fn below(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y - 1,
        ..pos
    }
}

fn wire_connected_sides(properties: &BTreeMap<String, String>) -> Option<[bool; 4]> {
    let side = |name| match properties.get(name)?.as_str() {
        "none" => Some(false),
        "side" | "up" => Some(true),
        _ => None,
    };
    Some([side("north")?, side("east")?, side("south")?, side("west")?])
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

fn success(success: LocalInteractionSuccess) -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success))
}

fn pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::{
        block_types::vanilla_block_type_for_resource_id,
        game_types::{VANILLA_GAME_TYPE_ADVENTURE_ID, VANILLA_GAME_TYPE_SURVIVAL_ID},
        packets::{
            BlockPos as ProtocolBlockPos, BlockUpdate, CommonPlayerSpawnInfo, Direction,
            InteractionHand, PlayLogin, SetChunkCacheCenter,
        },
    };

    const POS: BlockPos = BlockPos { x: 0, y: 8, z: 0 };

    #[test]
    fn button_press_and_powered_consume_match_vanilla_results() {
        let mut world = block_use_world(
            "minecraft:stone_button",
            [("face", "wall"), ("facing", "north"), ("powered", "false")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut world, 11),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&world, "powered"), Some("true".to_string()));
        assert_eq!(world.local_block_predictions()[0].sequence, 11);

        assert_eq!(
            use_main_hand(&mut world, 12),
            success(LocalInteractionSuccess::consume(false))
        );
        assert_eq!(world.local_block_predictions().len(), 1);
    }

    #[test]
    fn lever_only_requests_client_particle_when_switching_toward_powered() {
        let mut world = block_use_world(
            "minecraft:lever",
            [
                ("face", "ceiling"),
                ("facing", "east"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut world, 1),
            success(LocalInteractionSuccess {
                particle_action: Some(LocalInteractionParticleAction::LeverActivation {
                    pos: POS,
                    facing: Direction::East,
                    face: LocalLeverAttachFace::Ceiling,
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(
            block_property(&world, "powered"),
            Some("false".to_string()),
            "the client-side branch does not pull the lever state"
        );
        set_test_block(
            &mut world,
            state_id(
                "minecraft:lever",
                [("face", "ceiling"), ("facing", "east"), ("powered", "true")],
            ),
        );
        assert_eq!(
            use_main_hand(&mut world, 2),
            success(LocalInteractionSuccess::client_item_interaction())
        );
    }

    #[test]
    fn wooden_and_copper_openables_toggle_while_iron_passes_to_item_use() {
        for (name, properties) in [
            (
                "minecraft:oak_door",
                vec![
                    ("facing", "north"),
                    ("half", "lower"),
                    ("hinge", "left"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            ),
            (
                "minecraft:copper_trapdoor",
                vec![
                    ("facing", "north"),
                    ("half", "bottom"),
                    ("open", "false"),
                    ("powered", "false"),
                    ("waterlogged", "false"),
                ],
            ),
        ] {
            let mut world =
                block_use_world_vec(name, properties, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            assert_eq!(
                use_main_hand(&mut world, 4),
                success(LocalInteractionSuccess::client_item_interaction())
            );
            assert_eq!(block_property(&world, "open"), Some("true".to_string()));
        }

        for (name, properties) in [
            (
                "minecraft:iron_door",
                vec![
                    ("facing", "north"),
                    ("half", "lower"),
                    ("hinge", "left"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            ),
            (
                "minecraft:iron_trapdoor",
                vec![
                    ("facing", "north"),
                    ("half", "bottom"),
                    ("open", "false"),
                    ("powered", "false"),
                    ("waterlogged", "false"),
                ],
            ),
        ] {
            let mut world =
                block_use_world_vec(name, properties, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            assert_eq!(use_main_hand(&mut world, 5), pass());
            assert!(world.local_block_predictions().is_empty());
        }
    }

    #[test]
    fn fence_gate_uses_vanilla_player_direction_only_when_opening() {
        let mut world = block_use_world(
            "minecraft:oak_fence_gate",
            [
                ("facing", "north"),
                ("in_wall", "false"),
                ("open", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        assert_eq!(
            use_main_hand(&mut world, 21),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&world, "open"), Some("true".to_string()));
        assert_eq!(
            block_property(&world, "facing"),
            Some("south".to_string()),
            "SOUTH player direction replaces opposite NORTH facing"
        );

        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: f32::NAN,
            ..LocalPlayerPoseState::default()
        });
        assert_eq!(
            use_main_hand(&mut world, 22),
            success(LocalInteractionSuccess::client_item_interaction()),
            "closing does not query player direction"
        );
        assert_eq!(block_property(&world, "open"), Some("false".to_string()));
    }

    #[test]
    fn repeater_cycles_delay_only_with_may_build_and_daylight_is_server_owned() {
        let mut survival = block_use_world(
            "minecraft:repeater",
            [
                ("delay", "4"),
                ("facing", "north"),
                ("locked", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut survival, 31),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&survival, "delay"), Some("1".to_string()));

        let mut adventure = block_use_world(
            "minecraft:repeater",
            [
                ("delay", "2"),
                ("facing", "north"),
                ("locked", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_ADVENTURE_ID as i8,
        );
        assert_eq!(use_main_hand(&mut adventure, 32), pass());
        assert!(adventure.local_block_predictions().is_empty());

        let mut detector = block_use_world(
            "minecraft:daylight_detector",
            [("inverted", "false"), ("power", "7")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut detector, 33),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            block_property(&detector, "inverted"),
            Some("false".to_string())
        );
        assert!(detector.local_block_predictions().is_empty());
    }

    #[test]
    fn comparator_cycles_mode_with_may_build_and_keeps_server_output_state() {
        let mut survival = block_use_world(
            "minecraft:comparator",
            [
                ("facing", "north"),
                ("mode", "compare"),
                ("powered", "true"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut survival, 41),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            block_property(&survival, "mode"),
            Some("subtract".to_string())
        );
        assert_eq!(
            block_property(&survival, "powered"),
            Some("true".to_string())
        );

        let mut adventure = block_use_world(
            "minecraft:comparator",
            [
                ("facing", "north"),
                ("mode", "subtract"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_ADVENTURE_ID as i8,
        );
        assert_eq!(use_main_hand(&mut adventure, 42), pass());
        assert_eq!(
            block_property(&adventure, "mode"),
            Some("subtract".to_string())
        );
    }

    #[test]
    fn redstone_wire_toggles_cross_and_dot_with_preserved_power() {
        let mut world = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "9"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut world, 51),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&world, ["none", "none", "none", "none"], "9");

        set_test_block(
            &mut world,
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "9"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut world, 52),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&world, ["side", "side", "side", "side"], "9");
    }

    #[test]
    fn redstone_wire_recomputes_real_side_and_up_connections() {
        let mut side = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "0"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(
            &mut side,
            relative(POS, Direction::East),
            state_id_vec(
                "minecraft:repeater",
                vec![
                    ("delay", "1"),
                    ("facing", "east"),
                    ("locked", "false"),
                    ("powered", "false"),
                ],
            ),
        );
        assert_eq!(
            use_main_hand(&mut side, 61),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&side, ["none", "side", "none", "side"], "0");

        let mut climb = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "4"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        let east = relative(POS, Direction::East);
        set_block_at(&mut climb, east, state_id("minecraft:stone", []));
        set_block_at(
            &mut climb,
            above(east),
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "4"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut climb, 62),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&climb, ["none", "up", "none", "side"], "4");

        let mut support_override = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "4"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(
            &mut support_override,
            east,
            state_id("minecraft:soul_sand", []),
        );
        set_block_at(
            &mut support_override,
            above(east),
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "4"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut support_override, 63),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&support_override, ["none", "up", "none", "side"], "4");

        let mut blocked_climb = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "4"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(
            &mut blocked_climb,
            above(POS),
            state_id("minecraft:stone", []),
        );
        set_block_at(&mut blocked_climb, east, state_id("minecraft:stone", []));
        set_block_at(
            &mut blocked_climb,
            above(east),
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "4"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut blocked_climb, 64),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&blocked_climb, ["none", "none", "none", "none"], "4");
    }

    #[test]
    fn redstone_wire_honors_always_and_never_conductor_overrides() {
        let east = relative(POS, Direction::East);
        let below_east = below(east);
        let lower_wire = state_id_vec(
            "minecraft:redstone_wire",
            wire_properties("none", "none", "none", "none", "0"),
        );

        let mut soul_sand = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "0"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(&mut soul_sand, east, state_id("minecraft:soul_sand", []));
        set_block_at(&mut soul_sand, below_east, lower_wire);
        assert_eq!(
            use_main_hand(&mut soul_sand, 71),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&soul_sand, ["none", "none", "none", "none"], "0");

        let mut glass = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "0"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(&mut glass, east, state_id("minecraft:glass", []));
        set_block_at(&mut glass, below_east, lower_wire);
        assert_eq!(
            use_main_hand(&mut glass, 72),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&glass, ["none", "side", "none", "side"], "0");
    }

    #[test]
    fn offhand_skips_empty_hand_block_owner_and_main_hand_pass_continues_dispatch() {
        let mut world = block_use_world(
            "minecraft:oak_door",
            [
                ("facing", "north"),
                ("half", "lower"),
                ("hinge", "left"),
                ("open", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            world.predict_local_block_use(POS, Direction::Up, InteractionHand::OffHand, false, 1,),
            pass()
        );
        assert_eq!(block_property(&world, "open"), Some("false".to_string()));
    }

    fn use_main_hand(world: &mut WorldStore, sequence: i32) -> LocalTargetUsePrediction {
        world.predict_local_block_use(
            POS,
            Direction::Up,
            InteractionHand::MainHand,
            false,
            sequence,
        )
    }

    fn block_property(world: &WorldStore, property: &str) -> Option<String> {
        world
            .probe_block(POS)
            .and_then(|block| block.block_properties.get(property).cloned())
    }

    fn assert_wire_sides(world: &WorldStore, sides: [&str; 4], power: &str) {
        for (property, expected) in ["north", "east", "south", "west"].into_iter().zip(sides) {
            assert_eq!(
                block_property(world, property).as_deref(),
                Some(expected),
                "{property}"
            );
        }
        assert_eq!(block_property(world, "power").as_deref(), Some(power));
    }

    fn wire_properties(
        north: &'static str,
        east: &'static str,
        south: &'static str,
        west: &'static str,
        power: &'static str,
    ) -> Vec<(&'static str, &'static str)> {
        vec![
            ("north", north),
            ("east", east),
            ("south", south),
            ("west", west),
            ("power", power),
        ]
    }

    fn block_use_world<const N: usize>(
        name: &str,
        properties: [(&str, &str); N],
        game_type: i8,
    ) -> WorldStore {
        block_use_world_vec(name, properties.into_iter().collect(), game_type)
    }

    fn block_use_world_vec(name: &str, properties: Vec<(&str, &str)>, game_type: i8) -> WorldStore {
        let air = state_id("minecraft:air", []);
        let single_value = |domain, entry_count, global_id| PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        };
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.apply_login(&PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.apply_set_chunk_cache_center(SetChunkCacheCenter {
            chunk_x: 0,
            chunk_z: 0,
        });
        let section_count =
            usize::try_from(world.dimension().height.div_euclid(16)).expect("section count");
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..section_count)
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value(PaletteDomain::BlockStates, 4096, air),
                    biomes: single_value(PaletteDomain::Biomes, 64, 0),
                })
                .collect(),
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        set_test_block(&mut world, state_id_vec(name, properties));
        world
    }

    fn set_test_block(world: &mut WorldStore, block_state_id: i32) {
        set_block_at(world, POS, block_state_id);
    }

    fn set_block_at(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
    }

    fn state_id<const N: usize>(name: &str, properties: [(&str, &str); N]) -> i32 {
        state_id_vec(name, properties.into_iter().collect())
    }

    fn state_id_vec(name: &str, properties: Vec<(&str, &str)>) -> i32 {
        crate::vanilla_block_state_id(
            name,
            &properties
                .into_iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        )
        .unwrap_or_else(|| {
            let owner = vanilla_block_type_for_resource_id(name)
                .map(|block| block.constant_name)
                .unwrap_or("unknown");
            panic!("missing {owner} state")
        })
    }
}
