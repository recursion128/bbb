//! Signs: the record a chunk packet carries, the editor a right click opens, and
//! the dyes, inks and honeycomb applied to a written face.

use super::*;

// ---------------------------------------------------------------------------
// Block entities
// ---------------------------------------------------------------------------

/// `minecraft:oak_sign` turned to one of its sixteen rotations and not
/// waterlogged.
fn oak_sign_state(rotation: &str) -> i32 {
    let properties = BTreeMap::from([
        ("rotation".to_string(), rotation.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_sign",
        &properties,
    )
    .expect("oak_sign is a vanilla 26.1 block")
}

/// One `block_entities` record as an official save writes it for a sign, with
/// the two front lines already filled in.
fn saved_sign_record(x: i32, y: i32, z: i32, lines: [&str; 4]) -> NbtSummaryValue {
    let messages = NbtSummaryValue::List(
        lines
            .iter()
            .map(|line| NbtSummaryValue::string(*line))
            .collect(),
    );
    let empty = NbtSummaryValue::List(
        (0..4)
            .map(|_| NbtSummaryValue::string(String::new()))
            .collect(),
    );
    let face = |messages: NbtSummaryValue| {
        NbtSummaryValue::Compound(vec![
            nbt("messages", messages),
            nbt("color", NbtSummaryValue::string("black")),
            nbt("has_glowing_text", NbtSummaryValue::Byte(0)),
        ])
    };
    NbtSummaryValue::Compound(vec![
        nbt("id", NbtSummaryValue::string("minecraft:sign")),
        nbt("x", NbtSummaryValue::Int(x)),
        nbt("y", NbtSummaryValue::Int(y)),
        nbt("z", NbtSummaryValue::Int(z)),
        nbt("keepPacked", NbtSummaryValue::Byte(0)),
        nbt("components", NbtSummaryValue::Compound(Vec::new())),
        nbt("front_text", face(messages)),
        nbt("back_text", face(empty)),
        nbt("is_waxed", NbtSummaryValue::Byte(0)),
    ])
}

/// The restore fixture chunk with one section holding an oak sign at
/// (1, -64, 2) and the matching block-entity record.
fn restore_sign_chunk(lines: [&str; 4]) -> Java26ChunkData {
    Java26ChunkData::parse(restore_sign_chunk_raw(lines), restore_chunk_context()).unwrap()
}

fn restore_sign_chunk_raw(lines: [&str; 4]) -> bbb_server_types::save::NamedCompound {
    let mut data = vec![0_i64; 256];
    // 4096 entries over 256 longs is four bits each, so `Strategy`'s
    // `(y << 4 | z) << 4 | x` index 33 -- the cell (1, 0, 2) -- is the second
    // nibble of the third long.
    data[2] = 1 << 4;
    let section = NbtSummaryValue::Compound(vec![
        nbt("Y", NbtSummaryValue::Byte(-4)),
        nbt(
            "block_states",
            NbtSummaryValue::Compound(vec![
                nbt(
                    "palette",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Compound(vec![nbt(
                            "Name",
                            NbtSummaryValue::string("minecraft:stone"),
                        )]),
                        NbtSummaryValue::Compound(vec![
                            nbt("Name", NbtSummaryValue::string("minecraft:oak_sign")),
                            nbt(
                                "Properties",
                                NbtSummaryValue::Compound(vec![
                                    nbt("rotation", NbtSummaryValue::string("0")),
                                    nbt("waterlogged", NbtSummaryValue::string("false")),
                                ]),
                            ),
                        ]),
                    ]),
                ),
                nbt("data", NbtSummaryValue::LongArray(data)),
            ]),
        ),
        nbt(
            "biomes",
            NbtSummaryValue::Compound(vec![nbt(
                "palette",
                NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "minecraft:plains".to_string(),
                )]),
            )]),
        ),
    ]);
    let mut raw = restore_fixture_chunk_raw(vec![section]);
    for entry in raw.entries_mut() {
        if entry.name == "block_entities" {
            entry.value = NbtSummaryValue::List(vec![saved_sign_record(1, -64, 2, lines)]);
        }
    }
    raw
}

/// `LevelChunk.setBlockState`'s creation arm reached through the one
/// `Level.setBlock` every writer in this crate goes through, and then
/// `ClientboundLevelChunkPacketData`'s constructor, which walks
/// `chunk.getBlockEntities()`.
#[test]
fn a_sign_written_into_the_world_rides_the_chunk_packet_that_carries_its_column() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let at = BlockPos { x: 2, y: 64, z: 3 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let block_entity = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .expect("the write created the sign's block entity");
    assert_eq!(
        block_entity.type_id(),
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID
    );

    // A join after the write receives the column with the record on board,
    // which is what stops the client having to be told twice.
    let (sender_two, receiver_two) = (sender, receiver);
    enqueue(
        &sender_two,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let joined = world.tick(receiver_two.drain());
    let column = joined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
                ..
            } if chunk.x == 0 && chunk.z == 0 => Some(chunk.clone()),
            _ => None,
        })
        .expect("the join releases the spawn column");
    assert_eq!(column.chunk_data.block_entities.len(), 1);
    let wire = &column.chunk_data.block_entities[0];
    assert_eq!(wire.packed_xz, (2 << 4) | 3);
    assert_eq!(wire.y, 64);
    assert_eq!(
        wire.block_entity_type_id,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID
    );
    // Every other column the join released carries none.
    for chunk in joined.events.iter().filter_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
            ..
        } if chunk.x != 0 || chunk.z != 0 => Some(chunk),
        _ => None,
    }) {
        assert!(chunk.chunk_data.block_entities.is_empty());
    }
}

/// `SignBlock.useWithoutItem` (`SignBlock.java:127-152`) into
/// `ServerPlayer.openTextEdit` (`ServerPlayer.java:1304-1307`), then
/// `handleSignUpdate` (:2028-2044) writing the lines back.
#[test]
fn a_right_click_opens_the_sign_editor_and_the_written_lines_come_back_off_the_block() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );

    let clicked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: at,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );
    // `openTextEdit` sends the block first and the editor second.
    let editor = clicked
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::OpenSignEditor(editor),
                ..
            } => Some(*editor),
            _ => None,
        })
        .expect("clicking an empty sign opens its editor");
    assert_eq!(editor.pos, at);
    assert_eq!(
        world
            .get_block_entity(Java26BuiltinDimension::Overworld, at)
            .and_then(BlockEntity::sign)
            .and_then(crate::block_entity::sign::SignBlockEntity::player_who_may_edit),
        Some(Uuid::from_u128(1)),
        "the click claims the editor for the player who made it"
    );

    let written = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SignUpdate(SignUpdate {
            pos: at,
            is_front_text: editor.is_front_text,
            lines: [
                "§chello".to_string(),
                "there".to_string(),
                String::new(),
                String::new(),
            ],
        }),
    );
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .expect("the sign is still there");
    assert_eq!(
        sign.text(editor.is_front_text).plain_lines(false),
        // `ChatFormatting.stripFormatting` removed the section code.
        ["hello", "there", "", ""].map(str::to_string)
    );
    assert_eq!(
        sign.player_who_may_edit(),
        None,
        "the write releases the claim"
    );
    // `SignBlockEntity.markUpdated`'s `sendBlockUpdated` reaches
    // `ChunkHolder.broadcastBlockEntityIfNeeded`, so the session is told the
    // new text on the next tick's broadcast.
    let broadcast = written
        .events
        .iter()
        .chain(world.tick(receiver.drain()).events.iter())
        .any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::BlockEntityData(update),
                    ..
                } if update.pos == at
            )
        });
    assert!(broadcast, "the edited sign is broadcast to the trackers");

    // A second click opens the editor again, which is what proves the text is
    // read off the block rather than remembered by the session.
    let clicked_again = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: at,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 2,
        }),
    );
    assert!(clicked_again.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenSignEditor(_),
            ..
        }
    )));
}

/// A sign nobody claimed refuses a `ServerboundSignUpdatePacket`
/// (`SignBlockEntity.java:131-137`), and so does a position holding no sign at
/// all (`:2038-2040`).
#[test]
fn an_unclaimed_sign_and_an_empty_position_both_refuse_a_sign_update() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    for pos in [at, BlockPos { x: 5, y: 64, z: 5 }] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::SignUpdate(SignUpdate {
                pos,
                is_front_text: true,
                lines: [
                    "nope".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
            }),
        );
    }
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .unwrap();
    assert_eq!(
        sign.text(true).plain_lines(false),
        ["", "", "", ""].map(str::to_string)
    );
}

fn sign_use_item_on(pos: BlockPos, sequence: i32) -> PlayServerbound {
    PlayServerbound::UseItemOn(UseItemOn {
        hand: InteractionHand::MainHand,
        hit: BlockHitResult {
            pos,
            direction: Direction::Up,
            cursor_x: 0.5,
            cursor_y: 0.5,
            cursor_z: 0.5,
            inside: false,
            world_border_hit: false,
        },
        sequence,
    })
}

fn hold_in_hotbar(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    item_id: i32,
    count: i32,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count,
                ..ItemStackSummary::empty()
            },
        }),
    );
}

fn write_sign_from_click(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    sequence: i32,
    lines: [&str; 4],
) -> bool {
    let clicked = play(sender, receiver, world, 1, sign_use_item_on(pos, sequence));
    let Some(is_front_text) = clicked.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenSignEditor(editor),
            ..
        } => Some(editor.is_front_text),
        _ => None,
    }) else {
        return false;
    };
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SignUpdate(SignUpdate {
            pos,
            is_front_text,
            lines: lines.map(str::to_string),
        }),
    );
    is_front_text
}

fn played_sound(events: &[WorldEvent], resource_id: &str) -> bool {
    events.iter().any(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } => match sound.sound {
            SoundEventHolder::Reference { registry_id } => {
                vanilla_sound_event_resource_id(registry_id) == Some(resource_id)
            }
            SoundEventHolder::Direct { .. } => false,
        },
        _ => false,
    })
}

/// `SignBlock.useItemOn` (`SignBlock.java:90-116`) through `DyeItem.tryApplyToSign`
/// (`DyeItem.java:38-46`): the faced side is coloured, the item is consumed
/// in survival, the dye sound and `BLOCK_CHANGE` game event fire, and the
/// writer-owned save carries the new colour.
#[test]
fn a_dye_on_a_written_sign_colours_the_faced_side_and_is_consumed() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        force_game_mode: true,
        ..operator_config(&[("alice", 1)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    let faced = write_sign_from_click(&sender, &receiver, &mut world, at, 1, ["hello", "", "", ""]);
    hold_in_hotbar(
        &sender,
        &receiver,
        &mut world,
        bbb_protocol::item_types::VANILLA_ITEM_RED_DYE_ID,
        4,
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    let applied = play(&sender, &receiver, &mut world, 1, sign_use_item_on(at, 2));
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .expect("the sign is still there");
    assert_eq!(sign.text(faced).color(), "red");
    assert_eq!(sign.text(!faced).color(), "black");
    assert_eq!(
        applied.snapshot.players[0].inventory.slots[0].count, 3,
        "ItemStack.consume(1, player) shrinks a survival stack"
    );
    assert!(played_sound(&applied.events, "minecraft:item.dye.use"));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        game_events.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
                && event.pos == at
                && event.source_entity_id == Some(applied.snapshot.players[0].entity_id)
        }),
        "BLOCK_CHANGE is raised with the clicking player as source: {game_events:?}"
    );
    assert!(applied.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::BlockEntityData(update),
            ..
        } if update.pos == at
    )));
    let stats = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    let used = stats.events.iter().any(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AwardStats(update),
            ..
        } => update.stats.iter().any(|stat| {
            let key = bbb_server_types::stats::StatKey::new(stat.stat_type_id, stat.value_id);
            key.resource_ids() == Some(("minecraft:used", "minecraft:red_dye")) && stat.amount >= 1
        }),
        _ => false,
    });
    assert!(used, "Stats.ITEM_USED counts the dye: {stats:?}");

    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the dyed column is written");
    let parsed = Java26ChunkData::parse(
        created.document.clone(),
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .expect("the runtime's own document is a valid Java 26.1 chunk");
    let loaded = BlockEntity::load_static(at, oak_sign_state("0"), &parsed.block_entities()[0])
        .expect("the dyed record reads back")
        .expect("the fixture block entity loads");
    assert_eq!(loaded.sign().unwrap().text(faced).color(), "red");
}

/// `GlowInkSacItem` / `InkSacItem.tryApplyToSign` (`GlowInkSacItem.java:15-22`,
/// `InkSacItem.java:15-22`).
#[test]
fn glow_ink_and_ink_sac_toggle_the_faced_side_glow() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    let faced = write_sign_from_click(&sender, &receiver, &mut world, at, 1, ["glow", "", "", ""]);
    hold_in_hotbar(
        &sender,
        &receiver,
        &mut world,
        bbb_protocol::item_types::VANILLA_ITEM_GLOW_INK_SAC_ID,
        1,
    );
    let glowed = play(&sender, &receiver, &mut world, 1, sign_use_item_on(at, 2));
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .unwrap();
    assert!(sign.text(faced).has_glowing_text());
    assert!(!sign.text(!faced).has_glowing_text());
    assert!(played_sound(
        &glowed.events,
        "minecraft:item.glow_ink_sac.use"
    ));
    hold_in_hotbar(
        &sender,
        &receiver,
        &mut world,
        bbb_protocol::item_types::VANILLA_ITEM_INK_SAC_ID,
        1,
    );
    let inked = play(&sender, &receiver, &mut world, 1, sign_use_item_on(at, 3));
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .unwrap();
    assert!(!sign.text(faced).has_glowing_text());
    assert!(played_sound(&inked.events, "minecraft:item.ink_sac.use"));
}

/// `HoneycombItem.tryApplyToSign` (`HoneycombItem.java:171-178`) then the
/// waxed-sign fallthrough of `useItemOn` (`SignBlock.java:103`, `:115-117`)
/// into `useWithoutItem` (`:134-136`).
#[test]
fn honeycomb_waxes_a_sign_and_further_applicators_fall_through() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    hold_in_hotbar(
        &sender,
        &receiver,
        &mut world,
        bbb_protocol::item_types::VANILLA_ITEM_HONEYCOMB_ID,
        1,
    );
    let waxed = play(&sender, &receiver, &mut world, 1, sign_use_item_on(at, 1));
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .unwrap();
    assert!(sign.is_waxed());
    assert!(waxed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::LevelEvent(event),
            ..
        } if event.event_type == crate::block_entity::sign::LEVEL_EVENT_WAX_ON
            && event.pos == at
            && event.data == 0
    )));
    hold_in_hotbar(
        &sender,
        &receiver,
        &mut world,
        bbb_protocol::item_types::VANILLA_ITEM_RED_DYE_ID,
        1,
    );
    let refused = play(&sender, &receiver, &mut world, 1, sign_use_item_on(at, 2));
    assert!(played_sound(
        &refused.events,
        "minecraft:block.sign.waxed_interact_fail"
    ));
    assert!(!refused.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenSignEditor(_),
            ..
        }
    )));
    assert_eq!(
        world
            .get_block_entity(Java26BuiltinDimension::Overworld, at)
            .and_then(BlockEntity::sign)
            .unwrap()
            .text(true)
            .color(),
        "black"
    );
}

/// `SignApplicator.canApplyToSign` (`SignApplicator.java:11-13`) refuses a
/// dye on a sign with no message, so `useItemOn` falls through to the editor.
#[test]
fn a_dye_on_an_empty_sign_opens_the_editor_instead() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    hold_in_hotbar(
        &sender,
        &receiver,
        &mut world,
        bbb_protocol::item_types::VANILLA_ITEM_RED_DYE_ID,
        1,
    );
    let clicked = play(&sender, &receiver, &mut world, 1, sign_use_item_on(at, 1));
    assert!(clicked.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenSignEditor(_),
            ..
        }
    )));
    assert_eq!(
        world
            .get_block_entity(Java26BuiltinDimension::Overworld, at)
            .and_then(BlockEntity::sign)
            .unwrap()
            .text(true)
            .color(),
        "black"
    );
}

/// The runtime's own record has to be readable by the schema that owns saved
/// records: the save request is written into a chunk document and parsed back
/// by [`Java26ChunkData`] with no second decoder anywhere.
#[test]
fn a_runtime_signs_record_round_trips_through_the_saved_chunk_schema() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 4, y: 64, z: 6 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    if let Some(sign) = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign_mut)
    {
        sign.set_allowed_player_editor(Some(Uuid::from_u128(1)));
        sign.update_sign_text(
            Uuid::from_u128(1),
            false,
            true,
            &[
                "saved".to_string(),
                "lines".to_string(),
                String::new(),
                String::new(),
            ],
        );
    }

    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    // The spawn column is not in the loaded save, so it is written entire.
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the edited column is written");
    let parsed = Java26ChunkData::parse(
        created.document.clone(),
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .expect("the runtime's own document is a valid Java 26.1 chunk");
    assert_eq!(parsed.block_entities().len(), 1);
    let record = &parsed.block_entities()[0];
    let loaded = BlockEntity::load_static(at, oak_sign_state("0"), record)
        .expect("the record reads back as the same block entity")
        .expect("the fixture block entity loads");
    assert_eq!(
        loaded.sign().unwrap().text(true).plain_lines(false),
        ["saved", "lines", "", ""].map(str::to_string)
    );
}

/// The other direction: a record an official save carried reaches the runtime,
/// and the runtime hands it back to a client without the save being consulted
/// again.
#[test]
fn a_sign_record_from_a_loaded_save_reaches_the_runtime_and_its_clients() {
    let candidate = restore_candidate(
        1_000,
        Some(restore_sign_chunk(["from", "the", "save", ""])),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let at = BlockPos { x: 1, y: -64, z: 2 };
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .expect("the saved record was promoted into a live sign");
    assert_eq!(
        sign.text(true).plain_lines(false),
        ["from", "the", "save", ""].map(str::to_string)
    );

    // A changed saved column writes the block entities it holds at their current
    // state (`SerializableChunkData.java:357-363`), so an untouched sign
    // reads back as the save had it.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 3, y: -64, z: 3 },
        block_state_id("minecraft:diamond_block"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the loaded column is written");
    let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context()).unwrap();
    assert_eq!(parsed.block_entities().len(), 1);
    assert_eq!(
        BlockEntity::load_static(at, oak_sign_state("0"), &parsed.block_entities()[0])
            .unwrap()
            .expect("the fixture block entity loads")
            .sign()
            .unwrap()
            .text(true)
            .plain_lines(false),
        ["from", "the", "save", ""].map(str::to_string)
    );
}

/// Breaking the block takes its record with it. Vanilla expresses that by
/// rebuilding the record list from the live chunk; a patching writer has to
/// say it, and the patch has to remove the record the document still holds.
#[test]
fn breaking_a_saved_sign_drops_its_record_from_the_patched_document() {
    let candidate = restore_candidate(
        1_000,
        Some(restore_sign_chunk(["gone", "", "", ""])),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let at = BlockPos { x: 1, y: -64, z: 2 };
    let air = world.fixture().air_state_id();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .is_none());

    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the broken column is written");
    let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context()).unwrap();
    assert!(parsed.block_entities().is_empty());
}

/// `SignBlockEntity.tick` (`SignBlockEntity.java:265-276`) through
/// `Level.tickBlockEntities` (`Level.java:529-549`): the claim a click made
/// survives while the player stands next to the sign and is dropped once they
/// walk out of `Player.isWithinBlockInteractionRange(pos, 4.0)`.
#[test]
fn the_sign_ticker_drops_an_editor_claim_once_the_player_walks_away() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_sign_state("0"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: at,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );
    let claim = |world: &ServerWorld| {
        world
            .get_block_entity(Java26BuiltinDimension::Overworld, at)
            .and_then(BlockEntity::sign)
            .and_then(crate::block_entity::sign::SignBlockEntity::player_who_may_edit)
    };
    assert_eq!(claim(&world), Some(Uuid::from_u128(1)));
    world.tick(receiver.drain());
    assert_eq!(
        claim(&world),
        Some(Uuid::from_u128(1)),
        "a tick beside the sign keeps the claim"
    );

    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 40.5,
            y: 65.0,
            z: 40.5,
        },
        None,
    );
    world.tick(receiver.drain());
    assert_eq!(
        claim(&world),
        None,
        "the ticker drops a claim whose player has walked out of reach"
    );
}

/// `SignBlock.useWithoutItem` (`SignBlock.java:131`) runs a waxed sign's
/// `run_command` click event as the clicking player with gamemaster
/// permission (`SignBlockEntity.java:189-222`).
#[test]
fn clicking_a_waxed_sign_runs_its_click_command() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let face = r#"{messages:[{text:"go",click_event:{action:"run_command",command:"/setblock 3 64 3 gold_block"}},"","",""]}"#;
    for (id, command) in [
        (
            10,
            format!("setblock 0 64 1 oak_sign{{is_waxed:1b,front_text:{face}}}"),
        ),
        (11, format!("data merge block 0 64 1 {{back_text:{face}}}")),
    ] {
        let tick =
            crate::tests::entities::console_command(&sender, &receiver, &mut world, id, &command);
        assert_eq!(
            crate::tests::entities::console_command_answer(&tick).0,
            Some(1),
            "{command}"
        );
    }
    let clicked = play(&sender, &receiver, &mut world, 1, sign_use_item_on(at, 1));
    assert_eq!(
        live_block(&mut world, BlockPos { x: 3, y: 64, z: 3 })
            .and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id),
        Some(bbb_protocol::block_types::VANILLA_BLOCK_GOLD_BLOCK_ID)
    );
    assert!(played_sound(
        &clicked.events,
        "minecraft:block.sign.waxed_interact_fail"
    ));
}

/// `SignBlockEntity.loadLines` (`SignBlockEntity.java:109-127`) resolves a
/// loaded line's selector against a command source at the sign.
#[test]
fn a_loaded_sign_resolves_its_selector_lines() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let tick = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        10,
        r#"setblock 0 64 1 oak_sign{front_text:{messages:[{selector:"@p"},"","",""]}}"#,
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&tick).0,
        Some(1)
    );
    world.tick(receiver.drain());
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sign)
        .expect("the sign is there");
    assert_eq!(sign.text(true).plain_lines(false)[0], "alice");
}
