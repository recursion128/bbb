//! Fence gates, fence leash binds, cauldron bucket fill and empty, and the
//! dragon egg.

use super::*;

/// `FenceGateBlock.useWithoutItem` (`FenceGateBlock.java:147-167`) through the
/// live click path: `Player.getDirection` comes off the connection's own
/// rotation, and the default-spawn player faces south, so a closed gate whose
/// `FACING` is north — exactly away — turns to meet them as it opens
/// (`:152-155`).
#[test]
fn clicking_a_closed_gate_facing_away_turns_it_to_the_player_and_opens_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        fence_gate_state("north", false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = play(
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
    let turned = fence_gate_state("south", true);
    assert_eq!(block_update_sent(&opened, at), Some(turned));
    assert_eq!(live_block(&mut world, at), Some(turned));
    // FenceGateBlock.java:162-164: the clicking player predicted the sound.
    assert!(
        !sounds_sent(&opened).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.fence_gate.open")
                    .expect("a registered sound"),
            }
            && (sound.volume - 1.0).abs() < f32::EPSILON
            && (0.9..=1.0).contains(&sound.pitch)),
        "{opened:?}"
    );
}

/// A search that finds no dest still answers SUCCESS and leaves the egg.
#[test]
fn empty_hand_use_keeps_a_dragon_egg_when_no_dest_is_legal() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let egg = dragon_egg_state();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        egg,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let stone = block_state_id("minecraft:stone");
    for y in 65..=71 {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 0, y, z: 1 },
            stone,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    world.set_world_border_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_server_types::interaction::WorldBorder::from_settings(
            0.5, 1.5, 0.2, 5.0, 5, 300, 1.0, 0, 1.0, 0,
        ),
    );
    clear_hand(&sender, &receiver, &mut world);
    assert_eq!(
        dragon_egg_teleport_dest(
            &mut world,
            &crate::random::GameRandom::with_seed(0),
            at,
            bbb_server_types::interaction::WorldBorder::from_settings(
                0.5, 1.5, 0.2, 5.0, 5, 300, 1.0, 0, 1.0, 0,
            ),
        ),
        None
    );
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(egg));
}

/// `FenceBlock.useWithoutItem` (`FenceBlock.java:71-75`) over
/// `LeadItem.bindPlayerMobs` (`LeadItem.java:37-65`): empty hand binds held
/// boats. Bound is `SUCCESS_SERVER` (`:54-57`); there is no `ITEM_USED`.
#[test]
fn empty_hand_use_on_a_fence_binds_held_boats() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let boat = attach_boat_then_clear_hand(&sender, &receiver, &mut world);
    let player_position = world.publication().players[0].position;
    let fence = BlockPos {
        x: player_position.x.floor() as i32,
        y: player_position.y.floor() as i32,
        z: player_position.z.floor() as i32 + 1,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        fence,
        oak_fence(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let bound = click_use(&sender, &receiver, &mut world, fence, 0.5, 1);
    let knots = fence_live_knots(&world);
    assert_eq!(knots.len(), 1);
    assert_eq!(
        crate::entity::leash::leash_holder(fence_live_boat(&world, boat)),
        Some(knots[0])
    );
    assert!(
        fence_entity_links(&bound).contains(&(boat, knots[0])),
        "bind links: {:?}",
        fence_entity_links(&bound)
    );
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(game_events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::BLOCK_ATTACH
            && event.pos == fence
            && event.source_entity_id == Some(entity_id_of(&world, 1))
    }));
    assert!(fence_swung_main_hand(&bound));
    assert!(!fence_awarded_lead_used(&play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    )));
}

/// `:39-41` and `:60-62` through the without-item path: nothing held is PASS.
#[test]
fn empty_hand_use_on_a_fence_passes_when_nothing_is_leashed() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let fence = BlockPos {
        x: player_position.x.floor() as i32,
        y: player_position.y.floor() as i32,
        z: player_position.z.floor() as i32 + 1,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        fence,
        oak_fence(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);

    let empty = click_use(&sender, &receiver, &mut world, fence, 0.5, 1);
    assert!(fence_live_knots(&world).is_empty());
    assert!(!fence_swung_main_hand(&empty));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(!game_events
        .iter()
        .any(|event| { event.game_event == crate::level_event::VanillaGameEvent::BLOCK_ATTACH }));
}

/// `CauldronInteractions.fillWaterInteraction` (`CauldronInteractions.java:229-235`).
#[test]
fn water_bucket_use_fills_an_empty_cauldron() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        crate::block_behavior::cauldron::empty_cauldron_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        cauldron_item("minecraft:water_bucket", 1),
    );
    let filled = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(crate::block_behavior::cauldron::filled_water_cauldron_state())
    );
    assert!(
        cauldron_sound_sent(&filled, "minecraft:item.bucket.empty"),
        "{filled:?}"
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::FLUID_PLACE && event.pos == at
    }));
    let water =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:water_bucket")
            .expect("water bucket")
            .id;
    assert_eq!(held_cauldron_item(&world, 1), (Some(water), 1));
}

/// `CauldronInteractions` WATER + BUCKET (`CauldronInteractions.java:68-81`).
#[test]
fn empty_bucket_use_takes_a_full_water_cauldron() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        crate::block_behavior::cauldron::filled_water_cauldron_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, cauldron_item("minecraft:bucket", 1));
    let resolver = world.item_stack_resolver();
    let existing = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            nbt(
                "id",
                NbtSummaryValue::String("minecraft:water_bucket".into()),
            ),
            nbt("count", NbtSummaryValue::Int(1)),
            nbt(
                "components",
                NbtSummaryValue::Compound(vec![nbt(
                    "minecraft:rarity",
                    NbtSummaryValue::String("common".into()),
                )]),
            ),
        ]),
        &resolver,
    )
    .unwrap();
    *world
        .player_inventory_for_test(ConnectionId(1))
        .slot_mut(1)
        .unwrap() = existing;
    let emptied = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(crate::block_behavior::cauldron::empty_cauldron_state())
    );
    assert!(
        cauldron_sound_sent(&emptied, "minecraft:item.bucket.fill"),
        "{emptied:?}"
    );
    let water = bbb_protocol::item_types::VANILLA_ITEM_WATER_BUCKET_ID;
    assert_eq!(
        emptied.snapshot.players[0]
            .inventory
            .slots
            .iter()
            .filter(|stack| stack.item_id == Some(water))
            .map(|stack| stack.count)
            .sum::<i32>(),
        1,
        "creative filling must not duplicate an effectively equal bucket",
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::FLUID_PICKUP && event.pos == at
    }));
}

/// `CauldronInteractions.fillLavaInteraction` (`CauldronInteractions.java:237-243`).
#[test]
fn lava_bucket_use_consumes_when_the_cell_above_is_water() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let above = BlockPos { x: 0, y: 65, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        crate::block_behavior::cauldron::empty_cauldron_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        above,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:water")
                .expect("water")
                .id,
        )
        .expect("water default"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        cauldron_item("minecraft:lava_bucket", 1),
    );
    let consumed = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(crate::block_behavior::cauldron::empty_cauldron_state())
    );
    assert!(
        !cauldron_sound_sent(&consumed, "minecraft:item.bucket.empty_lava"),
        "{consumed:?}"
    );
    let lava = bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:lava_bucket")
        .expect("lava bucket")
        .id;
    assert_eq!(held_cauldron_item(&world, 1), (Some(lava), 1));
}

/// A non-bucket falls through to `CauldronInteraction.DEFAULT`
/// (`CauldronInteraction.java:18`).
#[test]
fn stick_use_on_a_cauldron_tries_the_empty_hand() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        crate::block_behavior::cauldron::empty_cauldron_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, cauldron_item("minecraft:stick", 1));
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(crate::block_behavior::cauldron::empty_cauldron_state())
    );
    let stick = bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:stick")
        .expect("stick")
        .id;
    assert_eq!(held_cauldron_item(&world, 1), (Some(stick), 1));
}

/// Survival `ItemUtils.createFilledResult` (`ItemUtils.java:27-36`) replaces
/// a single source bucket in hand.
#[test]
fn survival_water_bucket_use_leaves_an_empty_bucket_in_hand() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        force_game_mode: true,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        crate::block_behavior::cauldron::empty_cauldron_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        cauldron_item("minecraft:water_bucket", 1),
    );
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(crate::block_behavior::cauldron::filled_water_cauldron_state())
    );
    let bucket = bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:bucket")
        .expect("bucket")
        .id;
    assert_eq!(held_cauldron_item(&world, 1), (Some(bucket), 1));
}
