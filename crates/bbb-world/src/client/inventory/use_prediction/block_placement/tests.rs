use super::*;
use crate::{
    ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState,
    PaletteDomain, PaletteKind, PalettedContainerData, WorldBlockSoundProfile, WorldDimension,
};
use bbb_protocol::{
    block_types::{
        VANILLA_BLOCK_COBBLESTONE_WALL_ID, VANILLA_BLOCK_NETHER_BRICK_FENCE_ID,
        VANILLA_BLOCK_OAK_FENCE_ID, VANILLA_BLOCK_STONE_ID,
    },
    data_component_types::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
    game_types::VANILLA_GAME_TYPE_SURVIVAL_ID,
    item_types::{
        VANILLA_ITEM_AMETHYST_CLUSTER_ID, VANILLA_ITEM_CANDLE_ID, VANILLA_ITEM_CHEST_ID,
        VANILLA_ITEM_COBBLESTONE_WALL_ID, VANILLA_ITEM_COMMAND_BLOCK_ID, VANILLA_ITEM_FURNACE_ID,
        VANILLA_ITEM_HEAVY_CORE_ID, VANILLA_ITEM_IRON_BARS_ID, VANILLA_ITEM_LADDER_ID,
        VANILLA_ITEM_LANTERN_ID, VANILLA_ITEM_OAK_DOOR_ID, VANILLA_ITEM_OAK_FENCE_ID,
        VANILLA_ITEM_OAK_HANGING_SIGN_ID, VANILLA_ITEM_OAK_LOG_ID, VANILLA_ITEM_OAK_SIGN_ID,
        VANILLA_ITEM_OAK_SLAB_ID, VANILLA_ITEM_OAK_STAIRS_ID, VANILLA_ITEM_OBSERVER_ID,
        VANILLA_ITEM_RESPAWN_ANCHOR_ID, VANILLA_ITEM_SMALL_DRIPLEAF_ID, VANILLA_ITEM_STONE_ID,
        VANILLA_ITEM_SUNFLOWER_ID, VANILLA_ITEM_WHITE_BED_ID,
    },
    packets::{
        BlockPos as ProtocolBlockPos, BlockUpdate, CommonPlayerSpawnInfo,
        DataComponentPatchSummary, ItemStackSummary, PlayLogin, RegistryTags, SetChunkCacheCenter,
        SetPlayerInventory, TagNetworkPayload, UpdateTags, Vec3d,
    },
};

mod connections;

const CLICKED: BlockPos = BlockPos { x: 0, y: 8, z: 0 };
const TARGET: BlockPos = BlockPos { x: 0, y: 9, z: 0 };

#[test]
fn plain_block_item_places_consumes_sounds_and_rolls_back_on_ack() {
    let mut world = placement_world();
    set_held(&mut world, VANILLA_ITEM_STONE_ID, 2);
    world.set_default_block_sound_profiles(BTreeMap::from([(
        "minecraft:stone".to_string(),
        WorldBlockSoundProfile {
            break_sound: "minecraft:block.stone.break".to_string(),
            place_sound: "minecraft:block.stone.place".to_string(),
            hit_sound: "minecraft:block.stone.hit".to_string(),
            volume: 1.0,
            pitch: 1.0,
        },
    )]));

    assert_eq!(
        use_block(&mut world, 17),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::block_place_from_profile(
                TARGET,
                bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
            )
        ))
    );
    assert_eq!(
        world.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:stone", [])
    );
    assert_eq!(held(&world).count, 1);
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 17);

    let mut random = crate::LevelEventSoundRandomState::default();
    let effect = world
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::BlockPlaceFromProfile {
                pos: TARGET,
                block_type_id: bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
            },
            &mut random,
        )
        .unwrap();
    let crate::LocalInteractionSoundEffect::Positioned(sound) = effect else {
        panic!("block placement sound must be positioned");
    };
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.stone.place")
    );
    assert_eq!((sound.volume, sound.pitch), (1.0, 0.8));

    world.apply_block_changed_ack(bbb_protocol::packets::BlockChangedAck { sequence: 17 });
    assert_eq!(
        world.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert!(world.local_block_predictions().is_empty());
}

#[test]
fn block_state_component_applies_known_values_and_ignores_unknown_properties() {
    let mut world = placement_world();
    let mut stack = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_RESPAWN_ANCHOR_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    stack.component_patch.added = 1;
    stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID);
    stack.component_patch.block_state_properties = BTreeMap::from([
        ("charges".to_string(), "2".to_string()),
        ("missing".to_string(), "ignored".to_string()),
    ]);
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: stack,
    });

    assert!(matches!(
        use_block(&mut world, 18),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:respawn_anchor", [("charges", "2")])
    );
    assert_eq!(held(&world).count, 0);
}

#[test]
fn plain_block_item_replaces_clicked_block_but_rejects_player_obstruction() {
    let mut replace = placement_world();
    set_block(&mut replace, CLICKED, state_id("minecraft:short_grass", []));
    set_held(&mut replace, VANILLA_ITEM_STONE_ID, 1);
    assert!(matches!(
        use_block(&mut replace, 1),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        replace.probe_block(CLICKED).unwrap().block_state_id,
        state_id("minecraft:stone", [])
    );

    let mut obstructed = placement_world();
    obstructed.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: 0.5,
            y: 9.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    set_held(&mut obstructed, VANILLA_ITEM_STONE_ID, 1);
    assert_eq!(
        use_block(&mut obstructed, 2),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        obstructed.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(held(&obstructed).count, 1);
    assert!(obstructed.local_block_predictions().is_empty());
}

#[test]
fn directional_and_axis_placement_owners_use_exact_player_and_hit_directions() {
    let mut log = placement_world();
    set_held(&mut log, VANILLA_ITEM_OAK_LOG_ID, 1);
    assert!(matches!(
        use_block(&mut log, 3),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        log.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:oak_log", [("axis", "y")])
    );

    let mut furnace = placement_world();
    set_held(&mut furnace, VANILLA_ITEM_FURNACE_ID, 1);
    assert!(matches!(
        use_block(&mut furnace, 4),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        furnace.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:furnace", [("facing", "north"), ("lit", "false")])
    );

    let mut observer = placement_world();
    set_held(&mut observer, VANILLA_ITEM_OBSERVER_ID, 1);
    assert!(matches!(
        use_block(&mut observer, 5),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        observer.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:observer",
            [("facing", "south"), ("powered", "false")]
        )
    );
}

#[test]
fn same_item_replace_context_stacks_slabs_and_candles_in_place() {
    let mut slab = placement_world();
    set_held(&mut slab, VANILLA_ITEM_OAK_SLAB_ID, 2);
    assert!(matches!(
        use_block(&mut slab, 9),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        slab.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:oak_slab",
            [("type", "bottom"), ("waterlogged", "false")]
        )
    );
    assert!(matches!(
        use_block(&mut slab, 10),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        slab.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:oak_slab",
            [("type", "double"), ("waterlogged", "false")]
        )
    );
    assert_eq!(held(&slab).count, 0);

    let mut candle = placement_world();
    set_held(&mut candle, VANILLA_ITEM_CANDLE_ID, 2);
    assert!(matches!(
        use_block(&mut candle, 11),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert!(matches!(
        use_block(&mut candle, 12),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        candle.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:candle",
            [("candles", "2"), ("lit", "false"), ("waterlogged", "false"),]
        )
    );
}

#[test]
fn ordered_direction_placement_selects_exact_ladder_lantern_and_stair_states() {
    let east_target = BlockPos { x: 1, y: 8, z: 0 };
    let mut ladder = placement_world();
    set_held(&mut ladder, VANILLA_ITEM_LADDER_ID, 1);
    assert!(matches!(
        ladder.predict_local_block_use_with_hit(
            CLICKED,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            13,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        ladder.probe_block(east_target).unwrap().block_state_id,
        state_id(
            "minecraft:ladder",
            [("facing", "east"), ("waterlogged", "false")]
        )
    );

    let mut lantern = placement_world();
    set_held(&mut lantern, VANILLA_ITEM_LANTERN_ID, 1);
    assert!(matches!(
        use_block(&mut lantern, 14),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        lantern.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:lantern",
            [("hanging", "false"), ("waterlogged", "false")]
        )
    );

    let stair_clicked = BlockPos { x: 8, y: 8, z: 8 };
    let stair_target = BlockPos {
        y: 9,
        ..stair_clicked
    };
    let mut stair = placement_world();
    set_block(&mut stair, stair_clicked, state_id("minecraft:stone", []));
    stair.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: 4.0,
            y: 8.0,
            z: 4.0,
        },
        ..LocalPlayerPoseState::default()
    });
    set_held(&mut stair, VANILLA_ITEM_OAK_STAIRS_ID, 1);
    assert!(matches!(
        stair.predict_local_block_use_with_hit(
            stair_clicked,
            ProtocolDirection::Up,
            [0.25, 1.0, 0.75],
            false,
            InteractionHand::MainHand,
            false,
            15,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        stair.probe_block(stair_target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_stairs",
            [
                ("facing", "south"),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ]
        )
    );
}

#[test]
fn bed_item_places_its_client_foot_and_gamemaster_item_checks_permission() {
    let mut bed = placement_world();
    set_held(&mut bed, VANILLA_ITEM_WHITE_BED_ID, 1);
    assert!(matches!(
        use_block(&mut bed, 16),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        bed.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:white_bed",
            [("facing", "south"), ("occupied", "false"), ("part", "foot"),]
        )
    );
    assert_eq!(
        bed.probe_block(BlockPos {
            z: TARGET.z + 1,
            ..TARGET
        })
        .unwrap()
        .block_state_id,
        state_id("minecraft:air", [])
    );

    let mut command = placement_world();
    set_held(&mut command, VANILLA_ITEM_COMMAND_BLOCK_ID, 1);
    assert_eq!(
        use_block(&mut command, 17),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(held(&command).count, 1);
    assert_eq!(
        command.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
}

#[test]
fn placement_survival_uses_the_shared_exact_face_support_evaluator() {
    let mut supported = placement_world();
    set_held(&mut supported, VANILLA_ITEM_AMETHYST_CLUSTER_ID, 1);
    assert!(matches!(
        use_block(&mut supported, 7),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:amethyst_cluster",
            [("facing", "up"), ("waterlogged", "false")]
        )
    );

    let mut unsupported = placement_world();
    set_block(
        &mut unsupported,
        CLICKED,
        state_id(
            "minecraft:glass_pane",
            [
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        ),
    );
    set_held(&mut unsupported, VANILLA_ITEM_AMETHYST_CLUSTER_ID, 1);
    assert_eq!(
        use_block(&mut unsupported, 8),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        unsupported.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(held(&unsupported).count, 1);
}

#[test]
fn waterlogged_placement_reads_the_replaced_source_fluid() {
    let mut world = placement_world();
    set_block(
        &mut world,
        CLICKED,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut world, VANILLA_ITEM_HEAVY_CORE_ID, 1);

    assert!(matches!(
        use_block(&mut world, 6),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(CLICKED).unwrap().block_state_id,
        state_id("minecraft:heavy_core", [("waterlogged", "true")])
    );
}

#[test]
fn standing_and_wall_item_selects_the_exact_attachment_form() {
    let mut standing = placement_world();
    set_held(&mut standing, VANILLA_ITEM_OAK_SIGN_ID, 1);
    assert!(matches!(
        use_block(&mut standing, 18),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        standing.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:oak_sign",
            [("rotation", "8"), ("waterlogged", "false")]
        )
    );

    let east_target = BlockPos { x: 1, y: 8, z: 0 };
    let mut wall = placement_world();
    set_held(&mut wall, VANILLA_ITEM_OAK_SIGN_ID, 1);
    assert!(matches!(
        wall.predict_local_block_use_with_hit(
            CLICKED,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            19,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        wall.probe_block(east_target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_wall_sign",
            [("facing", "east"), ("waterlogged", "false")]
        )
    );
}

#[test]
fn double_high_item_predicts_each_client_side_write_and_rolls_back_together() {
    let upper = relative(TARGET, ProtocolDirection::Up);
    let mut flower = placement_world();
    set_support_tag(&mut flower, "minecraft:supports_vegetation");
    set_held(&mut flower, VANILLA_ITEM_SUNFLOWER_ID, 1);
    assert!(matches!(
        use_block(&mut flower, 26),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        flower.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:sunflower", [("half", "lower")])
    );
    assert_eq!(
        flower.probe_block(upper).unwrap().block_state_id,
        state_id("minecraft:sunflower", [("half", "upper")])
    );
    assert_eq!(flower.local_block_predictions().len(), 2);
    assert!(flower
        .local_block_predictions()
        .iter()
        .all(|prediction| prediction.sequence == 26));
    flower.apply_block_changed_ack(bbb_protocol::packets::BlockChangedAck { sequence: 26 });
    assert_eq!(
        flower.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(
        flower.probe_block(upper).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );

    let mut dripleaf = placement_world();
    set_support_tag(&mut dripleaf, "minecraft:supports_small_dripleaf");
    set_block(&mut dripleaf, upper, state_id("minecraft:seagrass", []));
    set_held(&mut dripleaf, VANILLA_ITEM_SMALL_DRIPLEAF_ID, 1);
    assert!(matches!(
        use_block(&mut dripleaf, 27),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        dripleaf.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:small_dripleaf",
            [
                ("facing", "north"),
                ("half", "lower"),
                ("waterlogged", "false"),
            ],
        )
    );
    assert_eq!(
        dripleaf.probe_block(upper).unwrap().block_state_id,
        state_id("minecraft:water", [("level", "0")])
    );
    assert_eq!(dripleaf.local_block_predictions().len(), 2);
}

#[test]
fn door_item_predicts_hinge_and_both_halves_atomically() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let lower = relative(clicked, ProtocolDirection::Up);
    let upper = relative(lower, ProtocolDirection::Up);
    let left = relative(lower, ProtocolDirection::East);
    let left_upper = relative(upper, ProtocolDirection::East);
    let mut door = placement_world();
    set_block(&mut door, clicked, state_id("minecraft:stone", []));
    set_block(&mut door, left, state_id("minecraft:stone", []));
    set_block(&mut door, left_upper, state_id("minecraft:stone", []));
    set_held(&mut door, VANILLA_ITEM_OAK_DOOR_ID, 1);

    assert!(matches!(
        use_block_at(&mut door, clicked, false, 28),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        door.probe_block(lower).unwrap().block_state_id,
        state_id(
            "minecraft:oak_door",
            [
                ("facing", "south"),
                ("half", "lower"),
                ("hinge", "left"),
                ("open", "false"),
                ("powered", "false"),
            ],
        )
    );
    assert_eq!(
        door.probe_block(upper).unwrap().block_state_id,
        state_id(
            "minecraft:oak_door",
            [
                ("facing", "south"),
                ("half", "upper"),
                ("hinge", "left"),
                ("open", "false"),
                ("powered", "false"),
            ],
        )
    );
    assert_eq!(door.local_block_predictions().len(), 2);
    assert!(door
        .local_block_predictions()
        .iter()
        .all(|prediction| prediction.sequence == 28));

    let mut powered = placement_world();
    set_block(&mut powered, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut powered,
        relative(lower, ProtocolDirection::West),
        state_id("minecraft:redstone_block", []),
    );
    set_held(&mut powered, VANILLA_ITEM_OAK_DOOR_ID, 1);
    assert_eq!(
        use_block_at(&mut powered, clicked, false, 29),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
    );
    assert!(powered.local_block_predictions().is_empty());
    assert_eq!(held(&powered).count, 1);
}

#[test]
fn remaining_hanging_sign_subclass_stays_explicit() {
    let mut world = placement_world();
    set_held(&mut world, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    assert_eq!(
        world.predict_local_block_use_with_hit(
            CLICKED,
            ProtocolDirection::Up,
            [0.25, 1.0, 0.75],
            false,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
    );
    assert_eq!(
        world.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
}

fn use_block(world: &mut WorldStore, sequence: i32) -> LocalTargetUsePrediction {
    use_block_at(world, CLICKED, false, sequence)
}

fn use_block_at(
    world: &mut WorldStore,
    clicked: BlockPos,
    secondary_use_active: bool,
    sequence: i32,
) -> LocalTargetUsePrediction {
    world.predict_local_block_use_with_hit(
        clicked,
        ProtocolDirection::Up,
        [0.25, 1.0, 0.75],
        false,
        InteractionHand::MainHand,
        secondary_use_active,
        sequence,
    )
}

fn placement_world() -> WorldStore {
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
            game_type: VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
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
    set_block(&mut world, CLICKED, state_id("minecraft:stone", []));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: 8.0,
            y: 8.0,
            z: 8.0,
        },
        ..LocalPlayerPoseState::default()
    });
    world
}

fn set_held(world: &mut WorldStore, item_id: i32, count: i32) {
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary::default(),
        },
    });
}

fn set_support_tag(world: &mut WorldStore, tag: &str) {
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: tag.to_string(),
                entries: vec![VANILLA_BLOCK_STONE_ID],
            }],
        }],
    });
}

fn held(world: &WorldStore) -> &ItemStackSummary {
    &world
        .inventory()
        .player_slots
        .iter()
        .find(|slot| slot.slot == 0)
        .expect("selected slot")
        .item
}

fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
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
    crate::vanilla_block_state_id(
        name,
        &properties
            .into_iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
    )
    .expect("vanilla block state")
}
