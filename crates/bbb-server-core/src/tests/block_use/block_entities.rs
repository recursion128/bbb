//! Clicking a block that answers with a block entity: note blocks, bells,
//! copper golem statues, bee nests, skulls, copper bulbs, jukeboxes, moving
//! pistons, respawn anchors and the light block.

use super::*;

/// The whole block-event mechanism, end to end and without a client: a note
/// block is placed, right-clicked with the same `UseItemOn` the interaction
/// replay sends, and the `ClientboundBlockEventPacket` vanilla would broadcast
/// reaches the session inside the same tick.
///
/// The parameters are vanilla's, not this server's: `NoteBlock.playNote`
/// (`NoteBlock.java:103`) calls `level.blockEvent(pos, this, 0, 0)`, so both
/// bytes are zero and the block is `minecraft:note_block`, at the note block's
/// own position. `ServerLevel.runBlockEvents` (`ServerLevel.java:1247-1258`)
/// only sends it because `NoteBlock.triggerEvent` (`:148-175`) answered true,
/// and it lands in the tick the click arrived in because
/// `processPacketsAndTick` drains the packet queue (`MinecraftServer.java:1005`)
/// before `tickServer` (`:1007`) reaches `ServerLevel.tick`\'s block-event arm
/// (`ServerLevel.java:388-391`).
#[test]
fn a_right_clicked_note_block_broadcasts_its_block_event_to_the_session() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "tuner", 1);
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 5.5,
            y: 65.0,
            z: 6.5,
        },
        None,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_NOTE_BLOCK_ID),
                count: 1,
                ..ItemStackSummary::empty()
            },
        }),
    );

    // The flat fixture floor is stone at y=63, so the note block lands on top
    // of it and `NoteBlock.setInstrument` (`NoteBlock.java:60-62`) reads
    // `NoteBlockInstrument.BASEDRUM` off the block below.
    let floor = BlockPos { x: 5, y: 63, z: 6 };
    let placed_at = BlockPos { x: 5, y: 64, z: 6 };
    let hit = |pos| BlockHitResult {
        pos,
        direction: Direction::Up,
        cursor_x: 0.5,
        cursor_y: 1.0,
        cursor_z: 0.5,
        inside: false,
        world_border_hit: false,
    };

    let placed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: hit(floor),
            sequence: 1,
        }),
    );
    assert!(
        block_events_sent(&placed).is_empty(),
        "placing a note block raises no block event"
    );
    assert_eq!(
        block_update_sent(&placed, placed_at),
        Some(note_block_state("basedrum", 0)),
        "`NoteBlock.getStateForPlacement` (`NoteBlock.java:66-68`) takes the \
         instrument from the block below and `registerDefaultState` (`:51`) \
         starts the note at 0"
    );

    // The same packet again, now against the note block itself. Vanilla\'s
    // `ServerPlayerGameMode.useItemOn` (`ServerPlayerGameMode.java:377-383`)
    // reaches `useWithoutItem` because `NoteBlock.useItemOn` (`:118-120`) only
    // passes for a head on the top face, and a note-block item is not one.
    let tuned = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: hit(placed_at),
            sequence: 2,
        }),
    );

    assert_eq!(
        block_events_sent(&tuned),
        vec![bbb_protocol::packets::BlockEvent {
            pos: placed_at,
            b0: 0,
            b1: 0,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID,
        }]
    );
    // `NoteBlock.useWithoutItem` (`:126-128`) advances the note and writes it
    // before `playNote` sees the state, so what stands there is note 1 and
    // nothing was built on top of the note block.
    assert_eq!(
        block_update_sent(&tuned, placed_at),
        Some(note_block_state("basedrum", 1))
    );
    assert_eq!(
        world.publication().changed_blocks.len(),
        1,
        "the click tuned the note block instead of placing a second one"
    );

    // `NoteBlock.triggerEvent` plays the note (`:171-173`) before it answers
    // (`:174`), so `ServerLevel.playSeededSound`'s own broadcast
    // (`ServerLevel.java:1023-1045`) is on the wire ahead of the block event
    // `runBlockEvents` sends for that same event (`:1247-1258`).
    assert_eq!(block_event_packet_order(&tuned), ["sound", "block_event"]);
    let sounds = sounds_sent(&tuned);
    assert_eq!(sounds.len(), 1);
    let sound = &sounds[0];
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.note_block.basedrum")
                .expect("a registered sound"),
        },
        "`NoteBlockInstrument.BASEDRUM` names `SoundEvents.NOTE_BLOCK_BASEDRUM` \
         (`NoteBlockInstrument.java:10`)"
    );
    // `:172`: `SoundSource.RECORDS`, volume `3.0F`, the block's own centre.
    assert_eq!(sound.source, SoundSource::Records);
    assert_eq!(sound.volume, 3.0);
    assert_eq!(
        sound.position,
        Vec3d {
            x: 5.5,
            y: 64.5,
            z: 6.5,
        }
    );
    // The state the click wrote carries note 1, and `getPitchFromNote`
    // (`:143-145`) turns that into `2^((1 - 12) / 12)`.
    assert_eq!(sound.pitch, 2.0_f64.powf(-11.0 / 12.0) as f32);
}

/// `BellBlock.useWithoutItem` (`BellBlock.java:91-95`) rings from a proper hit.
#[test]
fn clicking_a_floor_bell_rings_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        bell_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let rung = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: at,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );
    let bell = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.bell())
        .expect("bell");
    assert!(bell.shaking());
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(game_events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
            && event.pos == at
            && event.source_entity_id == Some(entity_id_of(&world, 1))
            && event.affected_state.is_none()
    }));
    assert!(
        sounds_sent(&rung).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.bell.use")
                    .expect("a registered sound"),
            }
            && (sound.volume - 2.0).abs() < f32::EPSILON),
        "{rung:?}"
    );
    assert!(
        block_events_sent(&rung)
            .iter()
            .any(|event| event.b0 == 1 && event.b1 == 2),
        "{rung:?}"
    );
}

/// `CopperGolemStatueBlock.updatePose` (`CopperGolemStatueBlock.java:119-123`).
#[test]
fn clicking_a_copper_golem_statue_cycles_its_pose() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        copper_golem_statue_state("minecraft:waxed_copper_golem_statue", "standing"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .map(|block_entity| block_entity.type_id()),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID)
    );
    let clicked = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(
        copper_golem_pose(&mut world, at).as_deref(),
        Some("sitting")
    );
    assert!(
        sounds_sent(&clicked).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(
                    "minecraft:entity.copper_golem_become_statue"
                )
                .expect("a registered sound"),
            }),
        "{clicked:?}"
    );
    right_click(&sender, &receiver, &mut world, 1, at, 2);
    assert_eq!(
        copper_golem_pose(&mut world, at).as_deref(),
        Some("running")
    );
    right_click(&sender, &receiver, &mut world, 1, at, 3);
    assert_eq!(copper_golem_pose(&mut world, at).as_deref(), Some("star"));
    right_click(&sender, &receiver, &mut world, 1, at, 4);
    assert_eq!(
        copper_golem_pose(&mut world, at).as_deref(),
        Some("standing")
    );
}

#[test]
fn an_axe_restores_a_saved_named_copper_golem_statue_and_keeps_its_water() {
    let dimension = Java26BuiltinDimension::Overworld;
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let state = placement_state(
        "minecraft:copper_golem_statue",
        &[
            ("facing", "east"),
            ("copper_golem_pose", "sitting"),
            ("waterlogged", "true"),
        ],
    );
    let name = bbb_protocol::component::RichTextComponent::try_from_component_nbt(
        bbb_protocol::parse_snbt_compound(
            r#"{text:"Copper",color:"gold",bold:1b,extra:[{text:" friend",italic:1b}]}"#,
        )
        .unwrap(),
    )
    .unwrap();
    let mut world = spawn_pinned_world(creative_fixture_config());
    assert!(world.set_block(dimension, at, state, crate::fluid::SET_BLOCK_UPDATE_ALL));
    assert!(world
        .level_block_entity_for_test(dimension, at)
        .unwrap()
        .create_copper_golem_statue(Some(&name)));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .unwrap();
    let chunk = Java26ChunkData::parse(
        write.document.clone(),
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    let mut world = spawn_pinned_restore(
        creative_fixture_config(),
        &restore_candidate(0, Some(chunk), Vec::new()),
    );
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WOODEN_AXE_ID),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let clicked = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let entities: Vec<_> = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(dimension, id))
        .filter(|entity| {
            entity.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID
        })
        .collect();
    assert_eq!(
        entities.len(),
        1,
        "the statue creates a golem without a block drop"
    );
    let golem = entities[0];
    assert_eq!(
        golem.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
    );
    assert_eq!(golem.custom_name(), Some(&name));
    assert_eq!(
        golem.position(),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 1.5
        }
    );
    assert_eq!(golem.y_rot(), -90.0);
    assert_eq!(golem.living().unwrap().y_head_rot(), -90.0);
    assert_eq!(golem.living().unwrap().y_body_rot, -90.0);
    assert_eq!(
        live_block(&mut world, at),
        Some(placement_state("minecraft:water", &[("level", "0")]))
    );
    assert!(world.get_block_entity(dimension, at).is_none());
    assert_eq!(
        world.publication().players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(1)
    );
    assert!(sounds_sent(&clicked).iter().any(|sound| {
        sound.source == SoundSource::Neutral
            && sound.sound
                == SoundEventHolder::Reference {
                    registry_id: vanilla_sound_event_registry_id(
                        "minecraft:entity.copper_golem.spawn",
                    )
                    .unwrap(),
                }
            && sound.position
                == Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 1.5,
                }
    }));
}

/// Oxidized statues pass axes to scraping; honeycomb waxes without cycling pose.
#[test]
fn an_axe_or_honeycomb_does_not_cycle_an_oxidized_or_waxed_copper_golem_statue() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        copper_golem_statue_state("minecraft:exposed_copper_golem_statue", "standing"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WOODEN_AXE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(copper_golem_statue_state(
            "minecraft:copper_golem_statue",
            "standing"
        ))
    );
    assert_eq!(
        copper_golem_pose(&mut world, at).as_deref(),
        Some("standing")
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_HONEYCOMB_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    right_click(&sender, &receiver, &mut world, 1, at, 2);
    assert_eq!(
        copper_golem_pose(&mut world, at).as_deref(),
        Some("standing")
    );
    assert_eq!(
        live_block(&mut world, at),
        Some(copper_golem_statue_state(
            "minecraft:waxed_copper_golem_statue",
            "standing"
        ))
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WOODEN_AXE_ID),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    right_click(&sender, &receiver, &mut world, 1, at, 3);
    assert_eq!(
        live_block(&mut world, at),
        Some(copper_golem_statue_state(
            "minecraft:copper_golem_statue",
            "standing"
        ))
    );
}

/// `BeehiveBlock.useItemOn` (`BeehiveBlock.java:169-180`) fills a glass bottle.
#[test]
fn filling_a_bottle_from_a_full_bee_nest_resets_honey() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        bee_nest_state(5),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    let filled = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(honey_level(&mut world, at).as_deref(), Some("0"));
    let alice = filled
        .snapshot
        .players
        .iter()
        .find(|player| player.name == "alice")
        .expect("alice snapshot");
    assert_eq!(
        alice.inventory.slots[0].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_HONEY_BOTTLE_ID)
    );
    let bottle_fill = SoundEventHolder::Reference {
        registry_id: vanilla_sound_event_registry_id("minecraft:item.bottle.fill")
            .expect("a registered sound"),
    };
    // `BeehiveBlock.java:171` passes `player` to `playSound`, so nearby observers hear
    // `BOTTLE_FILL` while the interacting player predicts it client-side.
    assert!(
        filled.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(sound),
            } if *connection_id == ConnectionId(2) && sound.sound == bottle_fill
        )),
        "{filled:?}"
    );
    assert!(
        !filled.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(sound),
            } if *connection_id == ConnectionId(1) && sound.sound == bottle_fill
        )),
        "{filled:?}"
    );
}

/// `BeehiveBlock.dropHoneycomb` (`BeehiveBlock.java:137-147`) draws
/// `minecraft:harvest/beehive`.
#[test]
fn shearing_a_full_bee_nest_drops_three_honeycomb() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        bee_nest_state(5),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    let sheared = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(honey_level(&mut world, at).as_deref(), Some("0"));
    assert!(
        sounds_sent(&sheared).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.beehive.shear")
                    .expect("a registered sound"),
            }),
        "{sheared:?}"
    );
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    assert_eq!(entities.len(), 1, "{entities:?}");
    let NbtSummaryValue::Compound(stack) = field(&entities[0], "Item") else {
        panic!("Item is a compound");
    };
    assert_eq!(
        field(stack, "id"),
        &NbtSummaryValue::string("minecraft:honeycomb")
    );
    assert_eq!(field(stack, "count"), &NbtSummaryValue::Int(3));
}

/// `AbstractSkullBlock.neighborChanged` (`AbstractSkullBlock.java:73-82`).
#[test]
fn a_player_head_powers_from_a_neighbouring_redstone_block() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let side = BlockPos { x: 1, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        player_head_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .map(|block_entity| block_entity.type_id()),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID)
    );
    assert_eq!(skull_powered(&mut world, at).as_deref(), Some("false"));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        side,
        block_state_id("minecraft:redstone_block"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(skull_powered(&mut world, at).as_deref(), Some("true"));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        side,
        block_state_id("minecraft:air"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(skull_powered(&mut world, at).as_deref(), Some("false"));
}

/// `NoteBlock.triggerEvent` (`NoteBlock.java:160-164`, `:177-179`) plus
/// `Holder.direct(SoundEvent.createVariableRangeEvent)` (`:166`).
#[test]
fn a_custom_head_note_block_plays_the_skull_sound_above() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let note = BlockPos { x: 0, y: 64, z: 1 };
    let head = BlockPos { x: 0, y: 65, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        note,
        note_block_state("custom_head", 0),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        head,
        player_head_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let silent = right_click(&sender, &receiver, &mut world, 1, note, 1);
    assert!(sounds_sent(&silent).is_empty(), "{silent:?}");
    assert!(block_events_sent(&silent).is_empty(), "{silent:?}");
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, head)
        .and_then(BlockEntity::skull_mut)
        .expect("player head")
        .set_note_block_sound(Some("minecraft:block.note_block.harp".to_string()));
    let played = right_click(&sender, &receiver, &mut world, 1, note, 2);
    assert_eq!(
        block_events_sent(&played),
        vec![bbb_protocol::packets::BlockEvent {
            pos: note,
            b0: 0,
            b1: 0,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID,
        }]
    );
    let sounds = sounds_sent(&played);
    assert_eq!(sounds.len(), 1, "{played:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Direct {
            location: "minecraft:block.note_block.harp".to_string(),
            fixed_range: None,
        }
    );
    assert_eq!(sounds[0].source, SoundSource::Records);
    assert_eq!(sounds[0].volume, 3.0);
    assert_eq!(sounds[0].pitch, 1.0);
}

/// `CopperBulbBlock.checkAndFlip` (`CopperBulbBlock.java:49-60`).
#[test]
fn a_copper_bulb_flips_lit_on_a_rising_neighbour_signal() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let side = BlockPos { x: 1, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        copper_bulb_state("minecraft:waxed_copper_bulb", false, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        copper_bulb_prop(&mut world, at, "lit").as_deref(),
        Some("false")
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        side,
        block_state_id("minecraft:redstone_block"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        copper_bulb_prop(&mut world, at, "lit").as_deref(),
        Some("true")
    );
    assert_eq!(
        copper_bulb_prop(&mut world, at, "powered").as_deref(),
        Some("true")
    );
    let ticked = world.tick(Vec::new());
    assert!(
        sounds_sent(&ticked).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.copper_bulb.turn_on")
                    .expect("a registered sound"),
            }),
        "{ticked:?}"
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        side,
        block_state_id("minecraft:air"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        copper_bulb_prop(&mut world, at, "lit").as_deref(),
        Some("true")
    );
    assert_eq!(
        copper_bulb_prop(&mut world, at, "powered").as_deref(),
        Some("false")
    );
}

/// `CopperBulbBlock.onPlace` (`CopperBulbBlock.java:35-38`).
#[test]
fn placing_a_copper_bulb_beside_power_lights_it() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let side = BlockPos { x: 1, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        side,
        block_state_id("minecraft:redstone_block"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        copper_bulb_state("minecraft:copper_bulb", false, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        copper_bulb_prop(&mut world, at, "lit").as_deref(),
        Some("true")
    );
    assert_eq!(
        copper_bulb_prop(&mut world, at, "powered").as_deref(),
        Some("true")
    );
}

/// `RespawnAnchorBlock.useItemOn` (`RespawnAnchorBlock.java:87-90`).
#[test]
fn respawn_anchor_glowstone_charges_and_consumes() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        respawn_anchor_state(0),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, cauldron_item("minecraft:glowstone", 2));
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let charged = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(respawn_anchor_state(1)));
    assert_eq!(
        held_cauldron_item(&world, 1),
        (
            Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:glowstone")
                    .expect("glowstone")
                    .id
            ),
            1
        )
    );
    assert!(
        cauldron_sound_sent(&charged, "minecraft:block.respawn_anchor.charge"),
        "{charged:?}"
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE && event.pos == at
    }));
}

/// `RespawnAnchorBlock.useItemOn` (`RespawnAnchorBlock.java:91-94`): a full
/// cell cannot charge, so glowstone falls through to `TRY_WITH_EMPTY_HAND`.
/// `useWithoutItem` (`RespawnAnchorBlock.java:106-109`) explodes it in the Overworld.
#[test]
fn respawn_anchor_full_glowstone_try_with_empty_hand() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(crate::tests::datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        respawn_anchor_state(4),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, cauldron_item("minecraft:glowstone", 1));
    let exploded = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(held_cauldron_item(&world, 1), glowstone_held());
    assert!(!cauldron_sound_sent(
        &exploded,
        "minecraft:block.respawn_anchor.charge"
    ));
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        events.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::EXPLODE && event.pos == at
        }),
        "events={events:?}"
    );
}

/// `RespawnAnchorBlock.useItemOn` (`RespawnAnchorBlock.java:92-93`).
#[test]
fn respawn_anchor_main_hand_passes_when_off_hand_can_charge() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        respawn_anchor_state(0),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let off_hand = crate::inventory::menu_layout::player_inventory_slot(
        bbb_protocol::equipment_slots::EquipmentSlot::OffHand,
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        off_hand,
        cauldron_item("minecraft:glowstone", 1),
    );
    let passed = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(respawn_anchor_state(0)));
    assert_eq!(
        world.publication().players[0].inventory.slots[off_hand].count,
        1
    );
    assert!(!cauldron_sound_sent(
        &passed,
        "minecraft:block.respawn_anchor.charge"
    ));
}

/// `ItemStack.consume` keeps the source when the player has infinite
/// materials (`RespawnAnchorBlock.java:89`).
#[test]
fn respawn_anchor_creative_charge_does_not_consume() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        respawn_anchor_state(2),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, cauldron_item("minecraft:glowstone", 1));
    let mut abilities = world.player_abilities_for_test(ConnectionId(1));
    abilities.instabuild = true;
    world.set_player_mode_and_abilities_for_test(ConnectionId(1), GameType::Creative, abilities);
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(respawn_anchor_state(3)));
    assert_eq!(held_cauldron_item(&world, 1), glowstone_held());
}

/// `MovingPistonBlock.useWithoutItem` (`MovingPistonBlock.java:86-88`).
#[test]
fn moving_piston_without_block_entity_is_removed() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        moving_piston_cell(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.remove_block_entity_for_test(Java26BuiltinDimension::Overworld, at);
    assert!(world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .is_none());
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(bbb_server_types::fluid::air_block_state_id())
    );
}

/// `MovingPistonBlock.useWithoutItem` (`MovingPistonBlock.java:89-90`).
#[test]
fn moving_piston_with_block_entity_passes() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        moving_piston_cell(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .is_some());
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(moving_piston_cell()));
}

/// `JukeboxPlayable.tryInsertIntoJukebox` (`JukeboxPlayable.java:43-55`).
#[test]
fn jukebox_inserts_a_music_disc_and_consumes() {
    let (sender, receiver) = mailbox();
    let dir = std::env::temp_dir().join(format!("bbb-jukebox-song-{}", Uuid::new_v4()));
    let pack = dir.join("test");
    std::fs::create_dir_all(pack.join("data/test/jukebox_song")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"jukebox song","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(pack.join("data/test/jukebox_song/short.json"), r#"{"sound_event":"minecraft:music_disc.cat","description":"Short song","length_in_seconds":0.05,"comparator_output":9}"#).unwrap();
    let options = bbb_server_datapack::startup::DatapackStartupOptions {
        datapacks_dir: Some(dir.clone()),
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    };
    let startup = bbb_server_datapack::startup::DatapackStartup::load(&options).unwrap();
    std::fs::write(pack.join("data/test/jukebox_song/short.json"), r#"{"sound_event":"minecraft:music_disc.cat","description":"Longer song","length_in_seconds":1.0,"comparator_output":14}"#).unwrap();
    let next_startup = bbb_server_datapack::startup::DatapackStartup::load(&options).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        datapacks: Some(std::sync::Arc::new(startup)),
        ..creative_fixture_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        jukebox_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &bbb_protocol::parse_snbt_compound(r#"{id:"minecraft:music_disc_13",count:2,components:{"minecraft:jukebox_playable":"test:short"}}"#).unwrap(),
            &world.item_stack_resolver(),
        ).unwrap().into_summary(),
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let started = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    let song_holder = bbb_registry_codec::PersistentItemStackHolderResolver::holder_id(
        &world.item_stack_resolver(),
        "minecraft:jukebox_song",
        "test:short",
    )
    .unwrap();
    let record_event = |tick: &TickResult, pos: BlockPos, kind: i32, data: i32| {
        tick.events.iter().any(|event| {
            matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::LevelEvent(packet), .. }
            | WorldEvent::Broadcast { packet: ClientboundPlayPacket::LevelEvent(packet), .. }
            if packet.event_type == kind && packet.pos == pos && packet.data == data)
        })
    };
    assert!(record_event(&started, at, 1010, song_holder));
    assert_eq!(live_block(&mut world, at), Some(jukebox_state(true)));
    assert_eq!(
        held_cauldron_item(&world, 1),
        (
            Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:music_disc_13"
                )
                .expect("cat disc")
                .id
            ),
            1
        )
    );
    let record = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.jukebox().cloned())
        .expect("the jukebox keeps a record");
    assert_eq!(
        record.item().item_id(),
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:music_disc_13")
            .map(|item| item.id)
    );
    assert_eq!(record.song_player().song_id(), Some("test:short"));
    assert_eq!(
        record
            .comparator_output(&world.item_stack_resolver())
            .unwrap(),
        9
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE && event.pos == at
    }));
    let next_world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(std::sync::Arc::new(next_startup)),
        ..creative_fixture_config()
    })
    .unwrap();
    let next_resolver = next_world.item_stack_resolver();
    assert_eq!(
        bbb_registry_codec::PersistentItemStackHolderResolver::holder_id(
            &world.item_stack_resolver(),
            "minecraft:jukebox_song",
            "test:short"
        ),
        bbb_registry_codec::PersistentItemStackHolderResolver::holder_id(
            &next_resolver,
            "minecraft:jukebox_song",
            "test:short"
        ),
    );
    let mut next_record = record.clone();
    assert_eq!(next_record.comparator_output(&next_resolver).unwrap(), 14);
    let mut stopped = false;
    let mut noted = false;
    for _ in 0..22 {
        let tick = world.tick(Vec::new());
        stopped |= record_event(&tick, at, 1011, 0);
        // `JukeboxSongPlayer.spawnMusicParticles` (`JukeboxSongPlayer.java:82-88`).
        noted |= tick.events.iter().any(|event| {
            matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::LevelParticles(particles), .. }
                if particles.particle.particle_type_id
                    == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_NOTE_ID
                    && particles.count == 0
                    && particles.position.y == f64::from(at.y) + f64::from(1.2_f32))
        });
        next_record.tick(&next_resolver).unwrap();
    }
    assert!(stopped, "the player receives the song's natural stop");
    assert!(noted, "the playing beat sends a note particle");
    let record = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .unwrap()
        .jukebox()
        .unwrap();
    assert!(!record.song_player().is_playing());
    assert!(next_record.song_player().is_playing());
    assert_eq!(
        record.item().item_resource_id(),
        Some("minecraft:music_disc_13")
    );
    let removed_at = BlockPos { x: 1, ..at };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        removed_at,
        jukebox_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    let started = click_use(&sender, &receiver, &mut world, removed_at, 0.5, 2);
    assert!(record_event(&started, removed_at, 1010, song_holder));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        removed_at,
        block_state_id("minecraft:air"),
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    let removed = world.tick(receiver.drain());
    assert!(
        record_event(&removed, removed_at, 1011, 0),
        "removing a playing jukebox stops the client's record presentation"
    );
}

/// `JukeboxBlock.useItemOn` (`JukeboxBlock.java:75-76`) falls through to
/// `popOutTheItem` (`JukeboxBlockEntity.java:48-61`).
#[test]
fn jukebox_with_a_record_pops_it_as_an_item_entity() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        jukebox_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        cauldron_item("minecraft:music_disc_cat", 2),
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(jukebox_state(true)));
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 2);
    assert_eq!(live_block(&mut world, at), Some(jukebox_state(false)));
    assert_eq!(
        held_cauldron_item(&world, 1),
        (
            Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:music_disc_cat"
                )
                .expect("cat disc")
                .id
            ),
            1
        )
    );
    let record = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.jukebox().cloned())
        .expect("the jukebox object remains");
    assert!(record.item().is_empty());
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    assert_eq!(entities.len(), 1, "{entities:?}");
    let NbtSummaryValue::Compound(stack) = field(&entities[0], "Item") else {
        panic!("Item is a compound");
    };
    assert_eq!(
        field(stack, "id"),
        &NbtSummaryValue::string("minecraft:music_disc_cat")
    );
    assert_eq!(field(stack, "count"), &NbtSummaryValue::Int(1));
    let pos = doubles(field(&entities[0], "Pos"));
    assert_eq!(pos.len(), 3, "{pos:?}");
    assert!(
        (pos[0] - 0.5).abs() < 0.4 && (pos[1] - 65.17).abs() < 0.2 && (pos[2] - 1.5).abs() < 0.4,
        "{pos:?}"
    );
}

/// An empty cell has no record to pop (`JukeboxBlock.java:57-61`).
#[test]
fn jukebox_without_a_record_passes_the_empty_hand() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        jukebox_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(jukebox_state(false)));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert!(saved_entities(&request).is_empty());
}

/// A stick has no `minecraft:jukebox_playable` prototype, so the click is
/// `TRY_WITH_EMPTY_HAND` (`JukeboxPlayable.java:38-40`).
#[test]
fn jukebox_refuses_an_item_without_jukebox_playable() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        jukebox_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, cauldron_item("minecraft:stick", 1));
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(jukebox_state(false)));
    assert_eq!(
        held_cauldron_item(&world, 1),
        (
            Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:stick")
                    .expect("stick")
                    .id
            ),
            1
        )
    );
}

/// `LightBlock.useWithoutItem` (`LightBlock.java:57-59`).
#[test]
fn light_cycles_level_for_a_game_master() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        light_state(15, true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
    let cycled = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(light_state(0, true)));
    assert_eq!(block_update_sent(&cycled, at), Some(light_state(0, true)));
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 2);
    assert_eq!(live_block(&mut world, at), Some(light_state(1, true)));
}

/// `Player.canUseGameMasterBlocks` (`Player.java:1840-1842`) is false without
/// both instabuild and gamemaster permission, so the click is `CONSUME`.
#[test]
fn light_consumes_without_cycling_when_not_a_game_master() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        light_state(15, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(light_state(15, false)));

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        light_state(7, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(light_state(7, false)));
}
