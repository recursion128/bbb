//! The sound `BlockItem.place` plays for the block it placed
//! (`BlockItem.java:86-87`).

use super::*;

/// The `ClientboundSoundPacket`s one tick sent, paired with the session each
/// one went to and the resolved sound-event registry id.
///
/// Every one of them has to be a `SoundSource.BLOCKS` event: the placement
/// sound is played on that channel and no other.
fn placement_sounds(tick: &TickResult) -> Vec<(ConnectionId, i32, Vec3d, f32, f32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(sound),
            } => {
                let SoundEventHolder::Reference { registry_id } = sound.sound else {
                    return None;
                };
                assert_eq!(sound.source, SoundSource::Blocks);
                Some((
                    *connection_id,
                    registry_id,
                    sound.position,
                    sound.volume,
                    sound.pitch,
                ))
            }
            _ => None,
        })
        .collect()
}

fn place_sound(sound: &str) -> i32 {
    vanilla_sound_event_registry_id(sound).expect("a place sound of the built-in catalog")
}

fn block_name(world: &mut ServerWorld, pos: BlockPos) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(live_block(world, pos)?)
        .map(|identity| identity.name.to_string())
}

/// Two sessions around the fixtures' own column: the first places, the second
/// watches. The spawn ground is solid at `y = 63`, so a click on `(x, 63, z)`
/// upward places into `(x, 64, z)`.
fn placer_and_watcher() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "placer", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "watcher", 2);
    for (connection_id, x) in [(1, 6.5), (2, 8.5)] {
        place_player(
            &sender,
            &receiver,
            &mut world,
            connection_id,
            Vec3d { x, y: 65.0, z: 6.5 },
            Some(PlayerRotation {
                y_rot: 0.0,
                x_rot: 0.0,
            }),
        );
    }
    (sender, receiver, world)
}

/// A placement by one session reaches every *other* watcher with the placed
/// block's own group: stone and the anvil differ in both the event and the
/// volume `BlockItem.place` derives from that group
/// (`(getVolume() + 1.0F) / 2.0F`, `getPitch() * 0.8F`).
///
/// The placing player is the `except` of `level.playSound(player, pos, ...)`
/// (`Level.java:389-392` into `PlayerList.broadcast`, `PlayerList.java:608`),
/// because that client predicts its own placement sound. The exact vectors
/// below are what pins it: a broadcast that also sent the placer, or one that
/// sent nobody, would not match.
#[test]
fn placing_a_block_broadcasts_its_own_group_place_sound_to_the_other_watchers() {
    let (sender, receiver, mut world) = placer_and_watcher();

    select_placement_item(&sender, &receiver, &mut world, "minecraft:stone");
    let placed = place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 6, y: 63, z: 6 },
        Direction::Up,
        1.0,
        1,
    );
    let stone = BlockPos { x: 6, y: 64, z: 6 };
    assert_eq!(
        block_name(&mut world, stone).as_deref(),
        Some("minecraft:stone")
    );
    assert_eq!(
        placement_sounds(&placed),
        vec![(
            ConnectionId(2),
            place_sound("minecraft:block.stone.place"),
            Vec3d {
                x: 6.5,
                y: 64.5,
                z: 6.5,
            },
            1.0,
            0.8,
        )],
        "`SoundType.STONE` is (1.0F, 1.0F) (`SoundType.java:28-30`), so the \
         placement scales it to volume 1.0 and pitch 0.8"
    );

    // The anvil's group is (0.3F, 1.0F) (`SoundType.java:58-60`): a fact a
    // hardcoded id and a constant volume could not carry.
    select_placement_item(&sender, &receiver, &mut world, "minecraft:anvil");
    let placed = place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 7, y: 63, z: 6 },
        Direction::Up,
        1.0,
        2,
    );
    let anvil = BlockPos { x: 7, y: 64, z: 6 };
    assert_eq!(
        block_name(&mut world, anvil).as_deref(),
        Some("minecraft:anvil")
    );
    assert_eq!(
        placement_sounds(&placed),
        vec![(
            ConnectionId(2),
            place_sound("minecraft:block.anvil.place"),
            Vec3d {
                x: 7.5,
                y: 64.5,
                z: 6.5,
            },
            0.65,
            0.8,
        )],
        "`(0.3F + 1.0F) / 2.0F` and `1.0F * 0.8F` (`BlockItem.java:87`)"
    );
}

/// The sound is `BlockItem.place`'s second-to-last act, after `setPlacedBy`
/// (`BlockItem.java:80`) and before `itemStack.consume` (`:89`), so a
/// placement that writes a second cell plays exactly one sound, at the clicked
/// position, and plays it after that write.
#[test]
fn a_two_cell_placement_plays_one_sound_after_the_second_write() {
    let (sender, receiver, mut world) = placer_and_watcher();

    select_placement_item(&sender, &receiver, &mut world, "minecraft:oak_door");
    let placed = place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 4, y: 63, z: 6 },
        Direction::Up,
        1.0,
        1,
    );
    let lower = BlockPos { x: 4, y: 64, z: 6 };
    let upper = BlockPos { x: 4, y: 65, z: 6 };
    for pos in [lower, upper] {
        assert_eq!(
            block_name(&mut world, pos).as_deref(),
            Some("minecraft:oak_door"),
            "both cells of the door are written"
        );
    }
    assert_eq!(
        placement_sounds(&placed),
        vec![(
            ConnectionId(2),
            place_sound("minecraft:block.wood.place"),
            Vec3d {
                x: 4.5,
                y: 64.5,
                z: 6.5,
            },
            1.0,
            0.8,
        )],
        "one sound, at the clicked position, from the door's own group"
    );
    // The upper half reaches the clients in the same tick as the sound. The
    // packet order inside the tick is not the source order -- the level
    // collects block changes and flushes them later in the tick, while
    // `play_block_sound_as` pushes the sound where the placement body stands --
    // so the write's own presence here is what says `setPlacedBy` ran beside
    // the sound rather than after it.
    assert!(
        placed.events.iter().any(|event| match event.sent_packet() {
            Some(ClientboundPlayPacket::BlockUpdate(update)) => update.pos == upper,
            Some(ClientboundPlayPacket::SectionBlocksUpdate(section)) => {
                section.updates.iter().any(|update| update.pos == upper)
            }
            _ => false,
        }),
        "the upper half reached the clients on the placement tick"
    );
}

/// A placement the target refuses writes nothing and therefore plays nothing:
/// the sound sits inside the arm that wrote the block, not beside the click.
#[test]
fn a_refused_placement_plays_no_place_sound() {
    let (sender, receiver, mut world) = placer_and_watcher();
    let occupied = BlockPos { x: 6, y: 64, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        occupied,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));

    select_placement_item(&sender, &receiver, &mut world, "minecraft:gold_block");
    let refused = place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 6, y: 63, z: 6 },
        Direction::Up,
        1.0,
        1,
    );
    assert_eq!(
        block_name(&mut world, occupied).as_deref(),
        Some("minecraft:stone"),
        "the occupied cell is not replaced"
    );
    assert!(
        placement_sounds(&refused).is_empty(),
        "`canPlace`/`getPlacementState` refused the placement before any sound"
    );
}
