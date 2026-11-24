//! Breaking a block: the destroy stages a survival session climbs, the gates the
//! game modes put in front of it, and the events the break publishes.

use super::*;

mod block_entity_loot;
mod creaking_heart;
mod hit_block;
mod infested;
mod player_overrides;

#[test]
fn block_gates_ask_the_world_for_its_bounds_rather_than_the_spawn_column() {
    // `Level.isInWorldBounds` (Level.java:167-169) is the build height plus the
    // horizontal block domain. A column a thousand chunks out is inside it and
    // is generated on demand; a y above the dimension and an x at the domain
    // edge are not, and `handleUseItemOn` (:1346-1375) answers both with the
    // build-limit overlay instead of a disconnect.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);

    // The positions here are thousands of chunks apart, which a fixture
    // writes directly rather than sweeping through the collision probe.
    let walk_to = |world: &mut ServerWorld, position: Vec3d| {
        world.set_player_position_for_test(ConnectionId(1), position);
        world.tick(receiver.drain());
    };
    let place_on_top_of = |world: &mut ServerWorld, pos: BlockPos| {
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 1.0,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 1,
            }),
        )
    };

    // Far from spawn and inside the world: the generator answers the column and
    // the placement lands on the floor it produced.
    walk_to(
        &mut world,
        Vec3d {
            x: 16_000.5,
            y: 65.0,
            z: -32_000.5,
        },
    );
    let placed = place_on_top_of(
        &mut world,
        BlockPos {
            x: 16_000,
            y: 63,
            z: -32_001,
        },
    );
    let target = BlockPos {
        x: 16_000,
        y: 64,
        z: -32_001,
    };
    assert!(
        placed
            .snapshot
            .changed_blocks
            .iter()
            .any(|block| block.pos == target),
        "a block a thousand chunks out is placed on generated ground"
    );

    // Above the dimension: inside reach, outside the build height, and beside
    // the top-layer cell so the builder does not obstruct it.
    walk_to(
        &mut world,
        Vec3d {
            x: 2.5,
            y: 318.0,
            z: 0.5,
        },
    );
    let above = place_on_top_of(&mut world, BlockPos { x: 0, y: 320, z: 0 });
    assert_build_limit_refusal(&above.events);
    // The top layer itself: clicking the air there places into it, and clicking
    // the placed block's top then fails with the same overlay.
    let top_layer = BlockPos { x: 0, y: 319, z: 0 };
    let filled = place_on_top_of(&mut world, top_layer);
    assert!(
        filled
            .snapshot
            .changed_blocks
            .iter()
            .any(|block| block.pos == top_layer),
        "{filled:?}"
    );
    let top = place_on_top_of(&mut world, top_layer);
    assert_build_limit_refusal(&top.events);
    assert!(
        !top.snapshot
            .changed_blocks
            .iter()
            .any(|block| block.pos.y == 320),
        "{top:?}"
    );

    // And the horizontal domain is half-open: `MAX_LEVEL_SIZE` itself is out.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    walk_to(
        &mut world,
        Vec3d {
            x: 29_999_999.5,
            y: crate::world::SPAWN_Y,
            z: 0.5,
        },
    );
    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BlockPos {
                    x: 30_000_000,
                    y: 63,
                    z: 0,
                },
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );
    assert_build_limit_refusal(&refused.events);
}

fn assert_build_limit_refusal(events: &[WorldEvent]) {
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "{events:?}"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SystemChat(chat),
                ..
            } if chat.overlay && format!("{chat:?}").contains("build.tooHigh")
        )),
        "{events:?}"
    );
}

/// `Level.removeBlock` (`Level.java:269-272`) writes
/// `fluidState.createLegacyBlock()`, not air. Breaking a waterlogged block
/// therefore leaves the water it held, and breaking water writes the same
/// water back over itself -- which `LevelChunk.setBlockState` answers null for
/// (`LevelChunk.java:280-281`) and `Level.setBlock` turns into false
/// (`Level.java:229-231`), so nothing moves and nothing is broadcast.
///
/// `destroyAndAck`'s correction packet (`ServerPlayerGameMode.java:253-260`)
/// does *not* fire for that: `destroyBlock` answers true whatever `removeBlock`
/// did (`:280`, `:301`). The only answer the breaker gets is the sequence
/// acknowledgement.
#[test]
fn breaking_water_writes_the_water_back_and_breaking_a_waterlogged_block_leaves_it() {
    let water = (
        [1_usize, 0_usize, 1_usize],
        "minecraft:water",
        &[("level", "0")][..],
    );
    let waterlogged_slab = (
        [2_usize, 0_usize, 1_usize],
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "true")][..],
    );
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(
                4,
                "minecraft:air",
                &[water, waterlogged_slab],
            )],
            Vec::new(),
            Vec::new(),
        )),
        Vec::new(),
    );
    let mut world = spawn_pinned_restore(creative_fixture_config(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "breaker", 1);

    let water_source =
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:water",
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .expect("built-in Java 26.1 block state");
    let broken_water = BlockPos { x: 1, y: 64, z: 1 };
    let broken_slab = BlockPos { x: 2, y: 64, z: 1 };
    for (pos, sequence) in [(broken_water, 3), (broken_slab, 4)] {
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::PlayerAction(PlayerAction {
                    action: PlayerActionKind::StartDestroyBlock,
                    pos,
                    direction: Direction::Up,
                    sequence,
                }),
            },
        )
        .unwrap();
    }
    let broken = world.tick(receiver.drain());

    // The water cell is untouched: same state, and no `BlockUpdate` for it at
    // all -- neither a broadcast of a change nor a correction to the breaker.
    assert_eq!(published_block(&world, broken_water), Some(water_source));
    assert!(
        !broken.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::BlockUpdate(update),
                ..
            } if update.pos == broken_water
        )),
        "breaking water changes nothing, so nothing goes on the wire: {:?}",
        broken.events
    );
    // The waterlogged slab is gone and the water it carried stayed behind.
    assert_eq!(published_block(&world, broken_slab), Some(water_source));
    assert!(broken.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::BlockUpdate(update),
            ..
        } if update.pos == broken_slab && update.block_state_id == water_source
    )));
}

/// `Item.canDestroyBlock` (`Item.java:169-172`) refuses every block when the
/// held stack carries a `DataComponents.TOOL` with
/// `canDestroyBlocksInCreative == false` and the user has `instabuild`, and in
/// 26.1 every sword is exactly that (`ToolMaterial.applySwordProperties` builds
/// `new Tool(.., 1.0F, 2, false)`, `ToolMaterial.java:82-91`).
/// `ServerPlayerGameMode.destroyAndAck` (`:253-260`) then answers the refusal
/// with a `ClientboundBlockUpdatePacket` carrying the block that is still
/// there. A creative client can put a sword in the main hand over
/// `SetCreativeModeSlot`, so this is the reachable half of the guard.
#[test]
fn a_creative_player_holding_a_sword_is_told_the_block_is_still_there() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let air = world.fixture().air_state_id();
    let solid = world.fixture().stone_state_id();

    let hold = |item_id: i32, receiver: &WorldCommandReceiver, world: &mut ServerWorld| {
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                    slot_num: 36,
                    item: ItemStackSummary {
                        item_id: Some(item_id),
                        count: 1,
                        component_patch: DataComponentPatchSummary::default().into(),
                    },
                }),
            },
        )
        .unwrap();
        world.tick(receiver.drain());
    };
    let swing =
        |pos: BlockPos, sequence: i32, receiver: &WorldCommandReceiver, world: &mut ServerWorld| {
            play(
                &sender,
                receiver,
                world,
                1,
                PlayServerbound::PlayerAction(PlayerAction {
                    action: PlayerActionKind::StartDestroyBlock,
                    pos,
                    direction: Direction::Up,
                    sequence,
                }),
            )
        };

    let refused_pos = BlockPos { x: 1, y: 63, z: 1 };
    hold(
        bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID,
        &receiver,
        &mut world,
    );
    let refused = swing(refused_pos, 3, &receiver, &mut world);
    assert_eq!(
        published_block(&world, refused_pos),
        None,
        "a sword destroys nothing in creative, so the column keeps what it \
         generated"
    );
    let corrections = refused
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::BlockUpdate(update),
            } => Some((*connection_id, update.pos, update.block_state_id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        corrections,
        vec![(ConnectionId(1), refused_pos, solid)],
        "the refusal is answered with the block that is still there: {refused:?}"
    );

    // The same swing with a pickaxe -- whose tool component leaves
    // `can_destroy_blocks_in_creative` at the codec default `true`
    // (`Tool.java:23`) -- breaks the block instead, so the refusal above is the
    // held item's doing rather than a swing this server refuses outright.
    let broken_pos = BlockPos { x: 2, y: 63, z: 1 };
    hold(
        bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_PICKAXE_ID,
        &receiver,
        &mut world,
    );
    swing(broken_pos, 4, &receiver, &mut world);
    assert_eq!(published_block(&world, broken_pos), Some(air));
}

#[test]
fn spawn_protection_refuses_a_non_operator_break_near_spawn() {
    let mut lists = crate::admin::StoredUserLists::default();
    let operator = crate::admin::NameAndId {
        id: Uuid::from_u128(99),
        name: "Operator".into(),
    };
    lists.operators.insert(
        operator.id,
        crate::admin::OperatorEntry {
            user: operator,
            level: crate::commands::CommandPermission::All,
            bypasses_player_limit: false,
        },
    );
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        admin_lists: lists,
        ..creative_fixture_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let solid = world.fixture().stone_state_id();
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary {
                    item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_PICKAXE_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            }),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    let pos = BlockPos { x: 1, y: 63, z: 1 };
    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos,
            direction: Direction::Up,
            sequence: 2,
        }),
    );
    assert_eq!(published_block(&world, pos), None, "the block stays");
    assert!(refused.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::BlockUpdate(update),
        } if update.pos == pos && update.block_state_id == solid
    )));
    assert!(refused.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::SystemChat(chat),
        } if chat.overlay
    )));
}

/// Every game-mode gate a block placement passes, and the price only one of
/// them pays.
///
/// `ServerPlayerGameMode.useItemOn` (`ServerPlayerGameMode.java:349-406`) turns
/// a spectator away before the item is consulted at all (`:358-365`);
/// `ItemStack.useOn` (`ItemStack.java:358-363`) turns away anyone whose
/// `Abilities.mayBuild` is clear, which `GameType.updatePlayerAbilities` makes
/// adventure and spectator (`GameType.java:79`, `:82-84`); and `BlockItem.place`
/// ends with `itemStack.consume(1, player)` (`BlockItem.java:89`), which
/// `ItemStack.consume` (`:1082-1086`) skips for a holder with infinite
/// materials. Before this gate existed every session placed the block and none
/// of them paid for it.
#[test]
fn a_placement_passes_the_game_mode_gates_and_only_survival_pays_for_it() {
    // The face of a floor block away from the spawn column, and the air cell
    // above it that a placement fills.
    let clicked = BlockPos { x: 2, y: 63, z: 1 };
    let target = BlockPos { x: 2, y: 64, z: 1 };
    for (game_type, places, held_after) in [
        (GameType::Creative, true, 64),
        (GameType::Survival, true, 63),
        (GameType::Adventure, false, 64),
        (GameType::Spectator, false, 64),
    ] {
        let mut world = spawn_pinned_world(ServerWorldConfig {
            game_type,
            ..creative_fixture_config()
        });
        let stone = world.fixture().stone_state_id();
        let (sender, receiver) = mailbox();
        join_loaded_stocked(&sender, &receiver, &mut world, 1, "builder", 1);
        let placed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: clicked,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 1.0,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 1,
            }),
        );
        assert_eq!(
            placed
                .snapshot
                .changed_blocks
                .iter()
                .any(|block| block.pos == target && block.block_state_id == stone),
            places,
            "{game_type:?} placed the block"
        );
        assert_eq!(
            placed.snapshot.players[0].inventory.slots[0].count, held_after,
            "{game_type:?} paid for the block"
        );
        // The consume is only visible to the client through the menu broadcast
        // the player tick runs (`ServerPlayer.tick`), so the packet is the
        // observable half of it.
        let republished = placed
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::ContainerSetSlot(update),
                        ..
                    } if update.item.count == held_after
                )
            })
            .count();
        assert_eq!(
            republished,
            usize::from(held_after != 64),
            "{game_type:?} told the client what it now holds"
        );
    }
}

/// Every `ClientboundBlockDestructionPacket` one tick sent, paired with the
/// session it reached.
fn block_destructions(tick: &TickResult) -> Vec<(ConnectionId, BlockDestruction)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::BlockDestruction(update),
            } => Some((*connection_id, *update)),
            _ => None,
        })
        .collect()
}

fn survival_config() -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Survival,
        ..creative_fixture_config()
    }
}

/// One survival break, from the swing that starts it to the release that lands
/// it, with a second session watching the crack.
///
/// The whole arithmetic is fixed by two generated facts and one vanilla
/// constant: the fixture floor is stone, whose `strength(1.5F, 6.0F)` and
/// `requiresCorrectToolForDrops()` sit on the same builder chain
/// (`Blocks.java:54-57`), and the hand holds `minecraft:stone`, which carries no
/// `DataComponents.TOOL`, so `Item.getDestroySpeed` answers `1.0F`
/// (`Item.java:183-186`). `BlockBehaviour.getDestroyProgress`
/// (`BlockBehaviour.java:355-363`) is then `1.0 / 1.5 / 100`, the 100 being the
/// divisor a hand that is not the correct tool takes (`:361`).
///
/// `STOP_DESTROY_BLOCK` accepts at `getDestroyProgress * (ticksSpentDestroying +
/// 1) >= 0.7F` (`ServerPlayerGameMode.java:218-226`), which needs 104 ticks
///    > between the two packets.
#[test]
fn a_survival_break_climbs_the_stages_and_lands_on_the_release_that_earned_it() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "watcher", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "distant", 3);
    stand_on_ground(&sender, &receiver, &mut world, 1);
    // 40 blocks away, which is outside the 1024 squared-block radius
    // `ServerLevel.destroyBlockProgress` sends inside (`ServerLevel.java:1015`).
    place_player(
        &sender,
        &receiver,
        &mut world,
        3,
        Vec3d {
            x: 40.5,
            y: crate::world::SPAWN_Y,
            z: 0.5,
        },
        None,
    );
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    let stone = world.fixture().stone_state_id();
    assert_eq!(published_block(&world, floor), None);

    let started = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: floor,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    // `:212` broadcasts the opening stage through
    // `ServerLevel.destroyBlockProgress` (`ServerLevel.java:1009-1020`), which
    // skips the breaker itself (`:1011`).
    let miner_entity = world
        .publication()
        .players
        .iter()
        .find(|player| player.name == "miner")
        .expect("the miner is online")
        .entity_id;
    assert_eq!(
        block_destructions(&started),
        vec![(
            ConnectionId(2),
            BlockDestruction {
                id: miner_entity,
                pos: floor,
                progress: 0,
            }
        )]
    );
    assert_eq!(
        published_block(&world, floor),
        None,
        "the swing removed nothing"
    );

    // The tick that carried the START already ran one
    // `ServerPlayerGameMode.tick` (`:107-130`) of its own, so 103 more make the
    // 104 `ticksSpentDestroying` the release below needs. The stage climbs one
    // step at a time over them and is broadcast only when it moves
    // (`:136-139`).
    let mut stages = Vec::new();
    for _ in 0..103 {
        let tick = world.tick(receiver.drain());
        for (connection_id, update) in block_destructions(&tick) {
            assert_eq!(connection_id, ConnectionId(2));
            assert_eq!(update.id, miner_entity);
            assert_eq!(update.pos, floor);
            stages.push(update.progress);
        }
    }
    assert_eq!(stages, vec![1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(published_block(&world, floor), None);

    let stopped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StopDestroyBlock,
            pos: floor,
            direction: Direction::Up,
            sequence: 2,
        }),
    );
    // `:224` clears the crack before `:225` destroys the block, and `-1` travels
    // as the byte `ClientboundBlockDestructionPacket.write` puts on the wire.
    assert_eq!(
        block_destructions(&stopped),
        vec![(
            ConnectionId(2),
            BlockDestruction {
                id: miner_entity,
                pos: floor,
                progress: 0xff,
            }
        )]
    );
    assert_eq!(
        published_block(&world, floor),
        Some(world.fixture().air_state_id()),
        "the release that earned 0.7 removed the block"
    );
    assert_ne!(stone, world.fixture().air_state_id());
}

/// A release one tick short of the threshold hands the break to the delayed arm
/// (`ServerPlayerGameMode.java:229-234`), which finishes it without another
/// packet from the client (`:109-119`).
#[test]
fn a_release_one_tick_early_is_finished_by_the_server_alone() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    stand_on_ground(&sender, &receiver, &mut world, 1);
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    let action = |action, sequence| {
        PlayServerbound::PlayerAction(PlayerAction {
            action,
            pos: floor,
            direction: Direction::Up,
            sequence,
        })
    };

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        action(PlayerActionKind::StartDestroyBlock, 1),
    );
    // One short of the 104 `ticksSpentDestroying` the release needs: 0.006666667
    // * 104 is 0.69333, and the threshold is 0.7.
    for _ in 0..102 {
        world.tick(receiver.drain());
    }
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        action(PlayerActionKind::StopDestroyBlock, 2),
    );
    assert_eq!(
        published_block(&world, floor),
        None,
        "0.69333 is short of the 0.7 the release needs"
    );

    // `delayedTickStart` is the same `destroyProgressStart` (`:233`), so the
    // delayed arm keeps counting from where the interrupted break was and
    // finishes when `getDestroyProgress * (ticksSpentDestroying + 1)` reaches
    // 1.0, which is 150 -- and the release's own tick already brought it to 105.
    for _ in 0..44 {
        world.tick(receiver.drain());
        assert_eq!(published_block(&world, floor), None);
    }
    world.tick(receiver.drain());
    assert_eq!(
        published_block(&world, floor),
        Some(world.fixture().air_state_id()),
        "the delayed arm finished the break with no client packet"
    );
}

/// `Player.blockActionRestricted` (`Player.java:188-203`) refuses adventure and
/// spectator, and `handleBlockBreakAction` answers the refusal by putting the
/// block back on the client (`ServerPlayerGameMode.java:177-181`).
#[test]
fn an_adventure_or_spectator_session_is_answered_with_the_block_it_may_not_break() {
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    for game_type in [GameType::Adventure, GameType::Spectator] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig {
            game_type,
            ..creative_fixture_config()
        });
        join_loaded(&sender, &receiver, &mut world, 1, "guest", 1);
        let swung = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: floor,
                direction: Direction::Up,
                sequence: 1,
            }),
        );
        let updates: Vec<_> = swung
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::BlockUpdate(update),
                } => Some((*connection_id, *update)),
                _ => None,
            })
            .collect();
        assert_eq!(
            updates,
            vec![(
                ConnectionId(1),
                BlockUpdate {
                    pos: floor,
                    block_state_id: world.fixture().stone_state_id(),
                }
            )],
            "{game_type:?} is told the block is still there"
        );
        assert!(
            block_destructions(&swung).is_empty(),
            "{game_type:?} started no break to crack"
        );
        assert_eq!(published_block(&world, floor), None, "{game_type:?}");
    }
}

/// `:201-202` insta-mines on the swing that starts the break whenever
/// `getDestroyProgress` already reads 1.0, and the tool the stack carries is
/// what decides that.
///
/// The stack is written through `SetCreativeModeSlot` and the session is then
/// moved to survival by `MinecraftServer.enforceGameTypeForPlayers`
/// (`MinecraftServer.java:1389-1403`), which is the one route this server has
/// for putting a chosen tool in a survival hand: `handleSetCreativeModeSlot`
/// needs `instabuild` and the enforcement keeps the inventory.
#[test]
fn a_tool_fast_enough_breaks_the_block_on_the_swing_that_starts_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        force_game_mode: true,
        datapacks: datapack_config().datapacks,
        ..operator_config(&[("miner", 1)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    let floor = BlockPos { x: 0, y: 63, z: 0 };

    // A `Tool` whose one rule names the floor block and mines it fast enough
    // that a single tick is already the whole block.
    let mut pick = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 1,
        ..ItemStackSummary::empty()
    };
    pick.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
    pick.component_patch.tool = Some(bbb_protocol::packets::ToolSummary {
        rules: vec![bbb_protocol::packets::ToolRuleSummary {
            block_registry_ids: vec![bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID],
            block_tag: None,
            speed_bits: 1_000.0_f32.to_bits().into(),
            correct_for_drops: Some(true),
        }],
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 1,
        can_destroy_blocks_in_creative: true,
    });
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: pick,
        }),
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );

    let swung = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: floor,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    assert!(
        block_destructions(&swung).is_empty(),
        "an insta-mine returns at `:202` before the crack is armed"
    );
    assert_eq!(
        published_block(&world, floor),
        Some(world.fixture().air_state_id()),
        "1000 / 1.5 / 30 is well past 1.0, so the swing is the whole break"
    );
}

/// `handleBlockBreakAction`'s ABORT branch (`ServerPlayerGameMode.java:239-249`)
/// clears the crack, and a mismatched position clears two (`:241-247`).
#[test]
fn aborting_a_break_clears_the_crack_on_both_positions_it_names() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "watcher", 2);
    let started_at = BlockPos { x: 0, y: 63, z: 0 };
    let named = BlockPos { x: 1, y: 63, z: 0 };
    let miner_entity = world
        .publication()
        .players
        .iter()
        .find(|player| player.name == "miner")
        .expect("the miner is online")
        .entity_id;

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: started_at,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    let aborted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::AbortDestroyBlock,
            pos: named,
            direction: Direction::Up,
            sequence: 2,
        }),
    );
    let clear = |pos| {
        (
            ConnectionId(2),
            BlockDestruction {
                id: miner_entity,
                pos,
                progress: 0xff,
            },
        )
    };
    assert_eq!(
        block_destructions(&aborted),
        vec![clear(started_at), clear(named)],
        "the mismatched position is cleared first (`:243`), the named one second (`:247`)"
    );

    // The break is over, so the next tick sends nothing more.
    let quiet = world.tick(receiver.drain());
    assert!(block_destructions(&quiet).is_empty());
    assert_eq!(published_block(&world, started_at), None);
}

/// A session outside creative reaches the destroy-progress machinery instead of
/// the creative half, so one `START_DESTROY_BLOCK` on a block whose progress is
/// below 1.0 removes nothing.
///
/// `handleBlockBreakAction`'s START branch splits on
/// `getAbilities().instabuild` (`ServerPlayerGameMode.java:172-175`): creative
/// goes straight to `destroyAndAck`, and everything else reaches the restricted
/// answer (`:177-181`) or the destroy-progress machinery (`:183-215`). That
/// second arm can still remove a block on the swing that starts it -- `:201-202`
/// insta-mines whenever `getDestroyProgress` already reads 1.0 -- so what this
/// pins is the arithmetic and not the absence of a mechanism: a bare hand on
/// stone earns `1.0 / 1.5 / 100 = 0.0066…` (`Blocks.java:54-57` for the hardness
/// and the `requiresCorrectToolForDrops()` beside it), which is nowhere near it.
#[test]
fn a_non_creative_session_does_not_break_a_block_on_the_first_swing() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-non-creative-break-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-non-creative-break");
    let candidate = storage.load_candidate().unwrap();

    let floor = BlockPos { x: 0, y: 63, z: 0 };
    let swing = PlayServerbound::PlayerAction(PlayerAction {
        action: PlayerActionKind::StartDestroyBlock,
        pos: floor,
        direction: Direction::Up,
        sequence: 1,
    });

    // Creative, which is what the created save declares: the swing removes it.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(operator_config(&[("op", 1)]), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let before = world.publication().changed_blocks.len();
    let broke = play(&sender, &receiver, &mut world, 1, swing.clone());
    let removed = broke
        .snapshot
        .changed_blocks
        .iter()
        .find(|block| block.pos == floor)
        .expect("the creative swing removed the floor block");
    assert_eq!(broke.snapshot.changed_blocks.len(), before + 1);
    assert_eq!(removed.block_state_id, world.fixture().air_state_id());

    // Survival, on the same save after `/defaultgamemode`: the same swing
    // changes nothing at all.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(operator_config(&[("op", 1)]), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    assert_eq!(
        login_game_type(&sender, &receiver, &mut world, 2, "miner", 2),
        GameType::Survival.id() as i8
    );
    let before = world.publication().changed_blocks.clone();
    let swung = play(&sender, &receiver, &mut world, 2, swing);
    assert_eq!(
        swung.snapshot.changed_blocks, before,
        "a survival swing may not insta-break"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `Block.spawnDestroyParticles` (`Block.java:514-516`) through
/// `Block.playerWillDestroy` (`:518-526`): `levelEvent(player, 2001, pos,
/// getId(state))`.
///
/// Three things are pinned at once, and all three are `PlayerList.broadcast`'s
/// (`PlayerList.java:597-617`), not this server's invention:
///
/// * the breaking session is excluded by identity (`:608`) — its own client
///   already drew the particles when it swung;
/// * a listener inside 64 blocks is sent the packet, and the distance is
///   measured from the raw block coordinates to the listener's feet
///   (`:609-611`);
/// * the comparison is a strict `<` on the squares (`:612-614`), so a listener
///   one block further out — at exactly 64 on the x axis, which is 4100 square
///   blocks away once the two-block drop to the floor is counted — hears
///   nothing.
#[test]
fn the_break_particle_event_skips_the_breaker_and_stops_at_sixty_four_blocks() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);

    let broken = BlockPos { x: 1, y: 63, z: 1 };
    let stone = world.fixture().stone_state_id();
    // `dx = 1 - 64`, `dy = 63 - 65`, `dz = 0`: 3973 < 4096.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            x: 64.0,
            y: 65.0,
            z: 1.0,
        },
    );
    // One block further: 4100, which is not less than 4096.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        3,
        Vec3d {
            x: 65.0,
            y: 65.0,
            z: 1.0,
        },
    );

    let destroyed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: broken,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, broken),
        Some(world.fixture().air_state_id()),
        "the break has to happen for the broadcast to mean anything"
    );

    let heard = level_events_sent(&destroyed);
    assert_eq!(heard.len(), 1, "{heard:?}");
    assert_eq!(
        heard[0].0,
        ConnectionId(2),
        "alice broke it and carol is out of range"
    );
    assert_eq!(
        heard[0].1,
        bbb_protocol::packets::LevelEvent {
            // `LevelEvent.PARTICLES_DESTROY_BLOCK` (`LevelEvent.java:53`).
            event_type: 2001,
            pos: broken,
            // `Block.getId(state)` (`Block.java:131-138`) is the state that
            // stood there, not the air that replaced it.
            data: stone,
            // `ServerLevel.levelEvent` builds a non-global packet
            // (`ServerLevel.java:1105`).
            global: false,
        }
    );
}

/// The break sound belongs to the level event alone: `Block.playerWillDestroy`
/// (`Block.java:518-526`) raises the destroy-particle level event and reaches
/// no sound channel, so a break may not also broadcast a sound beside it.
///
/// This is the arm that keeps the block sound group out of the break: the
/// group's `getBreakSound` is carried by the 2001 payload the client resolves
/// itself (`LevelEventHandler.java:280-289`), so a watcher that hears the event
/// must receive exactly that and nothing on the sound channel.
#[test]
fn a_break_publishes_its_level_event_and_no_sound_beside_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let broken = BlockPos { x: 1, y: 63, z: 1 };
    let destroyed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: broken,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, broken),
        Some(world.fixture().air_state_id()),
        "the break has to happen for the absence of a sound to mean anything"
    );

    let heard = level_events_sent(&destroyed);
    assert_eq!(heard.len(), 1, "{heard:?}");
    assert_eq!(
        heard[0].0,
        ConnectionId(2),
        "the watcher hears the particle event"
    );
    assert_eq!(heard[0].1.event_type, 2001);
    assert!(
        sounds_sent(&destroyed).is_empty(),
        "`Block.playerWillDestroy` raises the level event only; the break sound \
         has no second broadcast, so nothing may reach the sound channel"
    );
}

/// `Block.playerWillDestroy`'s third side effect, `gameEvent(GameEvent.BLOCK_DESTROY,
/// pos, Context.of(player, state))` (`Block.java:524`).
#[test]
fn a_player_break_records_the_block_destroy_game_event() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stone = world.fixture().stone_state_id();
    assert_eq!(
        world.game_events(Java26BuiltinDimension::Overworld),
        (Vec::new(), 0)
    );

    let broken = BlockPos { x: 1, y: 63, z: 1 };
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: broken,
            direction: Direction::Up,
            sequence: 1,
        }),
    );

    let (recorded, count) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(count, 1, "{recorded:?}");
    assert_eq!(
        recorded,
        vec![RecordedGameEvent {
            game_event: VanillaGameEvent::BLOCK_DESTROY,
            pos: broken,
            source_entity_id: Some(entity_id_of(&world, 1)),
            affected_state: Some(stone),
        }]
    );
}

#[test]
fn an_out_of_reach_break_is_acknowledged_without_a_disconnect() {
    // `handleBlockBreakAction` (:153-154) drops an out-of-reach break, and
    // `handlePlayerAction` (:1306) still acknowledges its sequence.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let pos = BlockPos { x: 40, y: 63, z: 0 };
    let far = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos,
            direction: Direction::Up,
            sequence: 4,
        }),
    );
    assert!(
        !far.events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "{far:?}"
    );
    assert!(far.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::BlockChangedAck(ack),
            ..
        } if ack.sequence == 4
    )));
    assert_eq!(published_block(&world, pos), None, "the block stays");
}
