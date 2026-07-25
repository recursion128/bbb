use super::{
    LocalBlockUseContext, LocalInteractionParticleAction, LocalInteractionResult,
    LocalInteractionSuccess, LocalLeverAttachFace, LocalTargetUsePrediction,
};
use crate::WorldStore;
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
            // Comparator output refresh and wire connection recomputation share
            // the redstone-neighbour evaluator implemented by the second stage.
            VanillaBlockUseWithoutItemOverride::ComparatorBlock
            | VanillaBlockUseWithoutItemOverride::RedStoneWireBlock => unsupported(),
            _ => unsupported(),
        }
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
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: POS.x,
                y: POS.y,
                z: POS.z,
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
